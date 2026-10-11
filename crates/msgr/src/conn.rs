//! Connection (session) state and the per-socket reader and writer tasks.
//!
//! A [`ConnShared`] is one messenger session with a peer. It outlives sockets when the policy
//! is lossless (`stateful_server` for clients): unacknowledged messages stay in `sent` and are
//! replayed with their original sequence numbers on the next socket, after the peer told us
//! which ones it already has (`ProtocolV1::requeue_sent`, `discard_requeued_up_to`).
//!
//! Each socket of a session is a *generation*: a reader task and a writer task tagged with
//! `sgen`. A fault or a replacement bumps `sgen`; tasks of an older generation stop.
//! Reading is decoupled from the application: the reader decodes and pushes
//! [`Event::Message`](crate::Event) into the session's event channel (bounded, so a slow
//! application pushes back on the socket), and never executes requests itself.

use std::cmp::Reverse;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, Weak};

use auth::{AuthCapsInfo, AuthName, SessionSigner};
use denc::{EntityAddr, EntityName, EntityType, RawMessage, UTime, features};
use tokio::sync::{Notify, mpsc};
use tokio::task::AbortHandle;
use tracing::{debug, info, trace, warn};

use crate::messenger::MsgrShared;
use crate::proto::{FrameReader, FrameWriter, InFrame, OutFrame};
use crate::{Connection, Event, MsgrError, Policy};

/// Life-cycle phase of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Outgoing: handshake in progress (or backing off). Incoming: being (re)opened.
    Connecting,
    /// A socket is attached and the handshake is complete.
    Open,
    /// Lossless server session without a socket, waiting for the peer to reconnect.
    Standby,
    /// Dead for good.
    Closed,
}

/// Authenticated identity of the peer (from its cephx ticket).
#[derive(Debug, Clone)]
pub(crate) struct PeerAuth {
    pub name: AuthName,
    pub global_id: u64,
    pub caps: AuthCapsInfo,
}

/// Priority output queue: highest priority first, FIFO within a priority
/// (`ProtocolV1::out_q`).
#[derive(Debug, Default)]
pub(crate) struct OutQueue {
    q: BTreeMap<Reverse<u16>, VecDeque<RawMessage>>,
}

impl OutQueue {
    pub fn push(&mut self, m: RawMessage) {
        self.q.entry(Reverse(m.priority)).or_default().push_back(m);
    }
    pub fn pop(&mut self) -> Option<RawMessage> {
        let mut e = self.q.first_entry()?;
        let m = e.get_mut().pop_front();
        if e.get().is_empty() {
            e.remove();
        }
        m
    }
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.q.is_empty()
    }
    pub fn clear(&mut self) {
        self.q.clear();
    }
    pub fn len(&self) -> usize {
        self.q.values().map(|v| v.len()).sum()
    }
}

/// Mutable session state, under `ConnShared::st`.
#[derive(Debug)]
pub(crate) struct ConnState {
    pub phase: Phase,
    /// Socket generation; tasks of other generations stop.
    pub sgen: u64,
    pub tasks: Vec<AbortHandle>,
    pub peer_addr: EntityAddr,
    pub peer_type: EntityType,
    /// Source name of the last message received (`ceph_msg_header.src`).
    pub peer_name: Option<EntityName>,
    pub peer_auth: Option<PeerAuth>,
    /// Negotiated features of the current socket.
    pub features: u64,
    pub signer: Option<SessionSigner>,
    /// Effective lossiness (an outgoing connection takes the server's `LOSSY` flag).
    pub lossy: bool,
    pub out_seq: u64,
    pub in_seq: u64,
    pub acked_in_seq: u64,
    pub out_q: OutQueue,
    /// Written but not acknowledged messages with their sequence numbers (lossless only).
    pub sent: VecDeque<(u64, RawMessage)>,
    /// Index into `sent` of the next message to (re)transmit on the current socket.
    pub resend_from: usize,
    pub keepalive: bool,
    pub keepalive_acks: Vec<UTime>,
    pub last_keepalive_ack: Option<UTime>,
    pub last_keepalive: Option<UTime>,
    pub connect_seq: u32,
    pub peer_global_seq: u32,
    /// An `Accepted`/`Connected` event was emitted.
    pub announced: bool,
}

/// One session.
pub(crate) struct ConnShared {
    pub id: u64,
    pub msgr: Weak<MsgrShared>,
    pub policy: Policy,
    pub outgoing: bool,
    pub events: mpsc::Sender<Event>,
    pub st: Mutex<ConnState>,
    pub wake: Notify,
}

impl std::fmt::Debug for ConnShared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let st = self.st.lock().unwrap();
        write!(f, "conn#{}({} {} {:?})", self.id, st.peer_type.name(), st.peer_addr, st.phase)
    }
}

/// Upper bound for one message segment; bigger messages fault the connection.
pub(crate) const MAX_SEGMENT: u32 = 256 << 20;

impl ConnShared {
    pub fn new(id: u64, msgr: Weak<MsgrShared>, policy: Policy, outgoing: bool, peer_addr: EntityAddr, peer_type: EntityType, events: mpsc::Sender<Event>) -> Arc<Self> {
        Arc::new(ConnShared {
            id,
            msgr,
            policy,
            outgoing,
            events,
            st: Mutex::new(ConnState {
                phase: Phase::Connecting,
                sgen: 0,
                tasks: Vec::new(),
                peer_addr,
                peer_type,
                peer_name: None,
                peer_auth: None,
                features: 0,
                signer: None,
                lossy: policy.lossy,
                out_seq: 0,
                in_seq: 0,
                acked_in_seq: 0,
                out_q: OutQueue::default(),
                sent: VecDeque::new(),
                resend_from: 0,
                keepalive: false,
                keepalive_acks: Vec::new(),
                last_keepalive_ack: None,
                last_keepalive: None,
                connect_seq: 0,
                peer_global_seq: 0,
                announced: false,
            }),
            wake: Notify::new(),
        })
    }

    pub fn handle(self: &Arc<Self>) -> Connection {
        Connection(self.clone())
    }

    /// Queue a message (`Connection::send_message`).
    pub fn send(&self, msg: RawMessage) -> bool {
        let mut st = self.st.lock().unwrap();
        if st.phase == Phase::Closed {
            debug!(conn = self.id, msg_type = msg.msg_type, "send on closed connection, dropped");
            return false;
        }
        st.out_q.push(msg);
        let open = st.phase == Phase::Open;
        drop(st);
        if open {
            self.wake.notify_one();
        }
        true
    }

    pub fn send_keepalive(&self) {
        let mut st = self.st.lock().unwrap();
        if st.phase == Phase::Closed {
            return;
        }
        st.keepalive = true;
        let open = st.phase == Phase::Open;
        drop(st);
        if open {
            self.wake.notify_one();
        }
    }

    /// Emit an event without awaiting (used from synchronous paths).
    pub fn emit_detached(self: &Arc<Self>, ev: Event) {
        let tx = self.events.clone();
        match tx.try_send(ev) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(ev)) => {
                tokio::spawn(async move {
                    let _ = tx.send(ev).await;
                });
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }
    }

    /// Stop the tasks of the current generation and start a new one. Returns the new `sgen`.
    pub fn bump_gen(st: &mut ConnState) -> u64 {
        st.sgen += 1;
        for t in st.tasks.drain(..) {
            t.abort();
        }
        st.sgen
    }

    /// `mark_down`: close for good, discard queues, no reconnect.
    pub fn mark_down(self: &Arc<Self>, emit: bool) {
        let mut st = self.st.lock().unwrap();
        if st.phase == Phase::Closed {
            return;
        }
        Self::bump_gen(&mut st);
        st.phase = Phase::Closed;
        st.out_q.clear();
        st.sent.clear();
        let announced = st.announced;
        drop(st);
        self.wake.notify_waiters();
        if let Some(m) = self.msgr.upgrade() {
            m.unregister(self);
        }
        if emit && announced {
            self.emit_detached(Event::Closed { conn: self.handle() });
        }
    }

    /// Close for good because the session is lost (lossy fault, or peer reset); emits `Reset`.
    pub fn reset(self: &Arc<Self>, why: &str) {
        let mut st = self.st.lock().unwrap();
        if st.phase == Phase::Closed {
            return;
        }
        Self::bump_gen(&mut st);
        st.phase = Phase::Closed;
        st.out_q.clear();
        st.sent.clear();
        let announced = st.announced;
        drop(st);
        self.wake.notify_waiters();
        info!(conn = self.id, "connection reset: {why}");
        if let Some(m) = self.msgr.upgrade() {
            m.unregister(self);
        }
        if announced {
            self.emit_detached(Event::Reset { conn: self.handle() });
        }
    }

    /// A socket of generation `sgen` failed (`ProtocolV1::fault`).
    pub fn fault(self: &Arc<Self>, sgen: u64, why: &str) {
        let mut st = self.st.lock().unwrap();
        if st.sgen != sgen || st.phase == Phase::Closed {
            return;
        }
        if st.lossy {
            drop(st);
            self.reset(why);
            return;
        }
        let new_sgen = Self::bump_gen(&mut st);
        // requeue_sent: everything unacknowledged is retransmitted on the next socket.
        st.resend_from = 0;
        if self.policy.server {
            st.phase = Phase::Standby;
            drop(st);
            self.wake.notify_waiters();
            info!(conn = self.id, "fault ({why}); server session in standby");
            return;
        }
        // Lossless outgoing: reconnect with connect_seq + 1.
        st.phase = Phase::Connecting;
        st.connect_seq += 1;
        drop(st);
        self.wake.notify_waiters();
        info!(conn = self.id, "fault ({why}); reconnecting");
        if let Some(m) = self.msgr.upgrade() {
            m.spawn_connect(self.clone(), new_sgen);
        }
    }

    /// Drop `sent` entries acknowledged by the peer (`handle_tag_ack`,
    /// `discard_requeued_up_to`).
    pub fn trim_sent(st: &mut ConnState, acked: u64) {
        while let Some((seq, _)) = st.sent.front() {
            if *seq > acked {
                break;
            }
            st.sent.pop_front();
            st.resend_from = st.resend_from.saturating_sub(1);
        }
        if st.out_seq < acked && st.sent.is_empty() {
            // The peer acknowledged numbers we no longer hold (session continued elsewhere).
            st.out_seq = acked;
        }
    }

    /// Start the reader and writer of generation `sgen` on an attached socket.
    pub fn start_io<R: FrameReader, W: FrameWriter>(self: &Arc<Self>, sgen: u64, reader: R, writer: W) {
        let mut st = self.st.lock().unwrap();
        if st.sgen != sgen || st.phase == Phase::Closed {
            return;
        }
        st.phase = Phase::Open;
        let r = tokio::spawn(read_loop(self.clone(), sgen, reader));
        let w = tokio::spawn(write_loop(self.clone(), sgen, writer));
        st.tasks.push(r.abort_handle());
        st.tasks.push(w.abort_handle());
        drop(st);
        self.wake.notify_one();
    }
}

async fn read_loop<R: FrameReader>(conn: Arc<ConnShared>, sgen: u64, mut reader: R) {
    loop {
        let frame = match reader.read_frame().await {
            Ok(f) => f,
            Err(e) => {
                conn.fault(sgen, &format!("read: {e}"));
                return;
            }
        };
        match frame {
            InFrame::Message { seq, msg } => {
                let deliver = {
                    let mut st = conn.st.lock().unwrap();
                    if st.sgen != sgen {
                        return;
                    }
                    if seq <= st.in_seq {
                        // ProtocolV1::handle_message_footer: old message, discard.
                        warn!(conn = conn.id, seq, in_seq = st.in_seq, "got old message, discarding");
                        false
                    } else {
                        if seq > st.in_seq + 1 {
                            warn!(conn = conn.id, seq, in_seq = st.in_seq, "missed message? skipped sequence numbers");
                        }
                        st.in_seq = seq;
                        st.peer_name = msg.src;
                        true
                    }
                };
                if !deliver {
                    continue;
                }
                trace!(conn = conn.id, seq, msg_type = msg.msg_type, "rx");
                if !conn.policy.lossy {
                    conn.wake.notify_one(); // send an ACK
                }
                if conn.events.send(Event::Message { conn: conn.handle(), msg }).await.is_err() {
                    debug!(conn = conn.id, "event receiver gone; dropping message");
                }
            }
            InFrame::Ack(seq) => {
                let mut st = conn.st.lock().unwrap();
                if st.sgen != sgen {
                    return;
                }
                ConnShared::trim_sent(&mut st, seq);
            }
            InFrame::Keepalive => {
                let mut st = conn.st.lock().unwrap();
                st.last_keepalive = Some(UTime::now());
            }
            InFrame::Keepalive2(t) => {
                {
                    let mut st = conn.st.lock().unwrap();
                    if st.sgen != sgen {
                        return;
                    }
                    st.keepalive_acks.push(t);
                    st.last_keepalive = Some(UTime::now());
                }
                conn.wake.notify_one();
            }
            InFrame::Keepalive2Ack(t) => {
                let mut st = conn.st.lock().unwrap();
                st.last_keepalive_ack = Some(t);
            }
            InFrame::Close => {
                info!(conn = conn.id, "peer sent CLOSE");
                conn.mark_down(true);
                return;
            }
        }
    }
}

async fn write_loop<W: FrameWriter>(conn: Arc<ConnShared>, sgen: u64, mut writer: W) {
    let Some(msgr) = conn.msgr.upgrade() else { return };
    loop {
        let (frames, src) = {
            let mut st = conn.st.lock().unwrap();
            if st.sgen != sgen || st.phase != Phase::Open {
                return;
            }
            let mut frames = Vec::new();
            for t in st.keepalive_acks.drain(..) {
                frames.push(OutFrame::Keepalive2Ack(t));
            }
            if st.keepalive {
                st.keepalive = false;
                if st.features & features::MSGR_KEEPALIVE2 != 0 {
                    frames.push(OutFrame::Keepalive2(UTime::now()));
                } else {
                    frames.push(OutFrame::Keepalive);
                }
            }
            // Retransmit unacknowledged messages first, with their original numbers.
            while st.resend_from < st.sent.len() && frames.len() < 64 {
                let (seq, m) = st.sent[st.resend_from].clone();
                st.resend_from += 1;
                frames.push(OutFrame::Message { seq, msg: m });
            }
            while frames.len() < 64 {
                let Some(m) = st.out_q.pop() else { break };
                st.out_seq += 1;
                let seq = st.out_seq;
                if !st.lossy {
                    st.sent.push_back((seq, m.clone()));
                    st.resend_from = st.sent.len();
                }
                frames.push(OutFrame::Message { seq, msg: m });
            }
            if !st.lossy && st.in_seq > st.acked_in_seq {
                st.acked_in_seq = st.in_seq;
                frames.push(OutFrame::Ack(st.in_seq));
            }
            (frames, msgr.my_name())
        };
        if frames.is_empty() {
            conn.wake.notified().await;
            continue;
        }
        if let Err(e) = writer.write_frames(&frames, src).await {
            conn.fault(sgen, &format!("write: {e}"));
            return;
        }
    }
}

/// Error helper for handshakes.
pub(crate) fn proto_err(s: impl Into<String>) -> MsgrError {
    MsgrError::Protocol(s.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_queue_priority_order() {
        let mut q = OutQueue::default();
        let mut a = RawMessage::new(1, 1, 1, bytes::Bytes::new());
        a.priority = 127;
        let mut b = RawMessage::new(2, 1, 1, bytes::Bytes::new());
        b.priority = 196;
        let mut c = RawMessage::new(3, 1, 1, bytes::Bytes::new());
        c.priority = 127;
        q.push(a);
        q.push(b);
        q.push(c);
        assert_eq!(q.len(), 3);
        assert_eq!(q.pop().unwrap().msg_type, 2);
        assert_eq!(q.pop().unwrap().msg_type, 1);
        assert_eq!(q.pop().unwrap().msg_type, 3);
        assert!(q.pop().is_none());
        assert!(q.is_empty());
    }
}
