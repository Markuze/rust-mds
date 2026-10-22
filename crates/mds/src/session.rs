//! Client sessions.
//!
//! A session is per client entity (its cephx global id) on this rank. It owns the client's
//! connection and is the **single funnel for every message sent to that client**
//! ([`Session::send`]): messages leave in the order they are queued, and the funnel counts
//! `push_seq` exactly as clients do (caps, snap, lease and quota messages), which
//! `REQUEST_CLOSE` is checked against (design 5.1, day-one hook). Later, when requests of one
//! session run in parallel, this funnel becomes the ordering point; callers do not change.
//!
//! Session messages follow the C++ `Server::handle_client_session` and `_session_logged`
//! behaviour that the kernel and libcephfs depend on (the map's sessions page, 3.2-3.7):
//! `OPEN` carries the MDS feature bitmap, the metric spec and the client's `cap_auths`;
//! `RENEWCAPS` echoes the client's seq; `REQUEST_CLOSE` is honoured only when its seq equals
//! `push_seq`.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Instant;

use denc::{EntityAddr, Message, RawMessage};
use proto::{FeatureBitset, MClientSession, MdsCapAuth, SessionOp, is_push_seq_counted};
use tracing::{debug, info, warn};

const MAX_QUEUED_OUTPUT_BYTES: usize = 16 << 20;
const MAX_SESSION_OUTPUTS: usize = 1024;
const MAX_TERMINAL_FUNNELS: usize = 1024;
pub(crate) const MAX_SESSIONS: usize = 1024;

enum PendingOutput {
    Raw(RawMessage),
    /// OPEN.seq belongs to the ordered transmission position, not enqueue time.
    Open(MClientSession),
}

impl PendingOutput {
    fn bytes(&self, features: u64) -> usize {
        match self {
            Self::Raw(raw) => message_bytes(raw),
            Self::Open(msg) => message_bytes(&msg.to_raw(features)),
        }
    }
}

/// Own a retired session's funnel until its final message reaches the transport.
/// A rejection never installs a runtime session and does not block a clean OPEN.
struct TerminalOutput {
    session: Session,
    blocks_reopen: bool,
}

/// Session states (`Session::STATE_*`), reduced to what this MDS uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Identity metadata is not durable yet. No new engine input is admitted.
    Opening,
    Open,
    Stale,
    /// Admission stops, but engine rights remain until fencing completes.
    Fencing,
    Closing,
    Closed,
}

/// Where a session's messages go. Implemented by `msgr::Connection`; a test double records them.
pub trait ClientLink: Send + Sync {
    /// Queue a message on the connection (in order). Returns false if the connection is gone.
    fn send_raw(&self, msg: RawMessage) -> bool;
    /// Connection features negotiated with the peer.
    fn features(&self) -> u64;
    /// The peer's address (the eviction blocklist needs it).
    fn peer_addr(&self) -> EntityAddr;
    fn authenticated_identity(&self) -> Option<(String, denc::Bytes)> {
        None
    }
}

impl ClientLink for msgr::Connection {
    fn send_raw(&self, msg: RawMessage) -> bool {
        self.send(msg)
    }
    fn features(&self) -> u64 {
        msgr::Connection::features(self)
    }
    fn peer_addr(&self) -> EntityAddr {
        msgr::Connection::peer_addr(self)
    }
    fn authenticated_identity(&self) -> Option<(String, denc::Bytes)> {
        use denc::Encode;
        let name = self.peer_auth_name()?.to_string();
        let caps = self.peer_caps()?;
        let mut bytes = denc::BytesMut::new();
        caps.encode(&mut bytes, 0);
        Some((name, bytes.freeze()))
    }
}

pub struct Session {
    /// Client id: the cephx global id (`entity_name_t` number of `client.<gid>`).
    pub client: u64,
    pub generation: u64,
    pub state: SessionState,
    pub link: Box<dyn ClientLink>,
    /// The peer address (the eviction blocklist names it).
    pub addr: EntityAddr,
    /// Client metadata from `REQUEST_OPEN` (`hostname`, `root`, `entity_id`, `uuid`, `timeout`, ...).
    pub metadata: BTreeMap<String, String>,
    /// CephFS features the client advertised.
    pub features: FeatureBitset,
    pub cap_auths: Vec<MdsCapAuth>,
    /// Counted messages sent (caps, snap, lease, quota).
    push_seq: u64,
    pub last_renew: Instant,
    paused: bool,
    queued: VecDeque<(PendingOutput, usize)>,
    queued_bytes: Arc<AtomicUsize>,
    output_failed: bool,
    rank_output_failed: Arc<AtomicBool>,
}

impl Session {
    /// Send a typed message to the client through the session's ordered funnel.
    pub fn send<M: Message>(&mut self, msg: &M) -> bool {
        let raw = msg.to_raw(self.link.features());
        self.send_raw(raw)
    }

    /// Send an already encoded message through the funnel.
    pub fn send_raw(&mut self, raw: RawMessage) -> bool {
        let output = if raw.msg_type == proto::msg_type::CLIENT_SESSION {
            match MClientSession::from_raw(raw.clone()) {
                Ok(msg) if msg.op == SessionOp::OPEN => PendingOutput::Open(msg),
                Ok(_) => PendingOutput::Raw(raw),
                Err(_) => {
                    self.fail_output();
                    return false;
                }
            }
        } else {
            PendingOutput::Raw(raw)
        };
        self.enqueue(output)
    }

    /// C++ Server::handle_client_session OPEN notification. Bind seq only when
    /// preceding messages have been accepted by the same ordered transport.
    fn send_open(&mut self) -> bool {
        self.send(&MClientSession::open_default(0, self.cap_auths.clone()))
    }

    fn fail_output(&mut self) {
        self.output_failed = true;
        // Independent sticky failure flag. Queue state remains on the owning task.
        self.rank_output_failed.store(true, Ordering::Relaxed);
    }

    fn enqueue(&mut self, output: PendingOutput) -> bool {
        if self.rank_output_failed.load(Ordering::Relaxed) {
            return false;
        }
        if self.paused || !self.queued.is_empty() {
            let bytes = output.bytes(self.link.features());
            // Relaxed is sufficient: this atomic is only an independent byte budget.
            // The owning daemon task orders all queue accesses and wire output.
            if self.queued.len() >= MAX_SESSION_OUTPUTS
                || self
                    .queued_bytes
                    .try_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                        used.checked_add(bytes)
                            .filter(|next| *next <= MAX_QUEUED_OUTPUT_BYTES)
                    })
                    .is_err()
            {
                self.fail_output();
                return false;
            }
            self.queued.push_back((output, bytes));
            return true;
        }
        self.transmit(output)
    }

    fn transmit(&mut self, output: PendingOutput) -> bool {
        if self.rank_output_failed.load(Ordering::Relaxed) {
            return false;
        }
        let raw = match output {
            PendingOutput::Raw(raw) => raw,
            PendingOutput::Open(mut msg) => {
                msg.seq = self.push_seq;
                msg.to_raw(self.link.features())
            }
        };
        let counted = is_push_seq_counted(raw.msg_type);
        if !self.link.send_raw(raw) {
            self.fail_output();
            return false;
        }
        if counted {
            self.push_seq += 1;
        }
        true
    }

    fn drain_output(&mut self, budget: usize) {
        for _ in 0..budget {
            if self.paused || self.output_failed {
                break;
            }
            let Some((output, bytes)) = self.queued.pop_front() else {
                break;
            };
            self.queued_bytes.fetch_sub(bytes, Ordering::Relaxed);
            if !self.transmit(output) {
                break;
            }
        }
    }

    pub fn push_seq(&self) -> u64 {
        self.push_seq
    }

    /// Validate CLOSE only after admitted work drains. Counted output that has
    /// not reached the transport cannot be covered by the client's current seq.
    pub fn close_ready(&self, generation: u64, seq: u64) -> bool {
        self.generation == generation
            && self.state == SessionState::Closing
            && seq == self.push_seq
            && !self.queued.iter().any(|(output, _)| {
                matches!(output,
                PendingOutput::Raw(raw) if is_push_seq_counted(raw.msg_type))
            })
    }

    pub fn is_open(&self) -> bool {
        matches!(self.state, SessionState::Open | SessionState::Stale)
    }
}

/// What the caller must do after a session message.
#[derive(Debug, PartialEq, Eq)]
pub enum SessionOutcome {
    /// Nothing beyond what the session layer already sent.
    Done,
    /// A new session was opened: the caller may now accept its requests.
    Opened,
    /// Runtime admission closed: the caller must drop caps, locks and requests.
    /// SessionMap retains queued terminal output until transport acceptance.
    Closed,
    /// The client renewed in time (or recovered a stale session): re-evaluate its caps.
    Renewed,
    /// Ask the journal to commit submitted metadata. There is no wire acknowledgment.
    FlushMdlog,
    /// Invalid client protocol. Stop service until fenced eviction is available.
    ProtocolError,
}

/// Runtime sessions plus bounded terminal funnels owned independently of admission.
#[derive(Default)]
pub struct SessionMap {
    sessions: HashMap<u64, Session>,
    next_generation: u64,
    paused: bool,
    queued_bytes: Arc<AtomicUsize>,
    rank_output_failed: Arc<AtomicBool>,
    terminal: VecDeque<TerminalOutput>,
}

impl SessionMap {
    pub fn terminal_close_pending(&self, client: u64) -> bool {
        self.terminal
            .iter()
            .any(|output| output.blocks_reopen && output.session.client == client)
    }

    /// Begin a durable CLOSE without sending its terminal response early.
    pub fn begin_close(&mut self, client: u64, seq: u64) -> Option<u64> {
        let s = self.sessions.get_mut(&client)?;
        if (!s.is_open() && s.state != SessionState::Closing) || seq != s.push_seq {
            return None;
        }
        s.state = SessionState::Closing;
        Some(s.generation)
    }

    pub fn complete_close(&mut self, client: u64, generation: u64, reply: bool) -> bool {
        if self
            .sessions
            .get(&client)
            .is_none_or(|s| s.generation != generation)
        {
            return false;
        }
        let Some(mut session) = self.sessions.remove(&client) else {
            return false;
        };
        if reply {
            session.send(&MClientSession::close());
            session.state = SessionState::Closed;
            self.retain_terminal(session, true);
        }
        // A fenced close explicitly discards this old connection's queued output.
        true
    }

    pub fn install_silent(
        &mut self,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: MClientSession,
        cap_auths: Vec<MdsCapAuth>,
    ) {
        if self.sessions.contains_key(&client) || self.terminal_close_pending(client) {
            return;
        }
        self.next_generation += 1;
        let mut session = self.new_funnel(client, self.next_generation, link);
        session.state = SessionState::Opening;
        session.metadata = msg.metadata;
        session.features = msg.supported_features;
        session.cap_auths = cap_auths;
        self.sessions.insert(client, session);
    }

    pub fn publish_open(
        &mut self,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: MClientSession,
        cap_auths: Vec<MdsCapAuth>,
    ) -> SessionOutcome {
        if let Some(session) = self.sessions.get_mut(&client)
            && session.state == SessionState::Opening
        {
            session.state = SessionState::Open;
        }
        self.handle(client, link, msg, cap_auths)
    }
    fn new_funnel(&self, client: u64, generation: u64, link: Box<dyn ClientLink>) -> Session {
        let addr = link.peer_addr();
        Session {
            client,
            generation,
            state: SessionState::Open,
            link,
            addr,
            metadata: BTreeMap::new(),
            features: FeatureBitset::default(),
            cap_auths: Vec::new(),
            push_seq: 0,
            last_renew: Instant::now(),
            paused: self.paused,
            queued: VecDeque::new(),
            queued_bytes: self.queued_bytes.clone(),
            output_failed: false,
            rank_output_failed: self.rank_output_failed.clone(),
        }
    }

    /// Reject an OPEN/reconnect without installing runtime or engine state. The
    /// map owns a bounded terminal funnel if the ordered send cannot run yet.
    pub fn reject_blocklisted(&mut self, client: u64, link: Box<dyn ClientLink>) {
        if self.output_failed() {
            return;
        }
        if self.terminal.len() >= MAX_TERMINAL_FUNNELS {
            self.rank_output_failed.store(true, Ordering::Relaxed);
            return;
        }
        // Non-admitted rejection funnels do not consume session generations.
        let mut session = self.new_funnel(client, 0, link);
        session.send(&MClientSession::reject_blocklisted());
        session.state = SessionState::Closed;
        self.retain_terminal(session, false);
    }

    pub fn pause_output(&mut self, paused: bool) {
        self.paused = paused;
        for s in self.sessions.values_mut() {
            s.paused = paused;
        }
        for output in &mut self.terminal {
            output.session.paused = paused;
        }
    }

    /// Bound actual transmission per scheduling turn across the rank.
    pub fn drain_output(&mut self, budget: usize) {
        if self.output_failed() {
            return;
        }
        let mut remaining = budget;
        // Terminal output has an owner independent of runtime session membership.
        for output in &mut self.terminal {
            let count = output.session.queued.len().min(remaining);
            output.session.drain_output(count);
            if output.session.output_failed {
                return;
            }
            remaining -= count;
            if remaining == 0 {
                break;
            }
        }
        self.terminal
            .retain(|output| !output.session.queued.is_empty() || output.session.output_failed);
        if remaining == 0 {
            return;
        }
        for s in self.sessions.values_mut() {
            let count = s.queued.len().min(remaining);
            s.drain_output(count);
            if s.output_failed {
                break;
            }
            remaining -= count;
            if remaining == 0 {
                break;
            }
        }
    }

    pub fn output_failed(&self) -> bool {
        self.rank_output_failed.load(Ordering::Relaxed)
    }
    pub fn queued_output_bytes(&self) -> usize {
        self.queued_bytes.load(Ordering::Relaxed)
    }
    pub fn get_mut(&mut self, client: u64) -> Option<&mut Session> {
        self.sessions.get_mut(&client)
    }

    pub fn get(&self, client: u64) -> Option<&Session> {
        self.sessions.get(&client)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Session> {
        self.sessions.values_mut()
    }

    /// The client ids of all sessions.
    pub fn clients(&self) -> Vec<u64> {
        self.sessions.keys().copied().collect()
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Handle an `MClientSession` from `client` arriving on `link`. `cap_auths` are the client's
    /// MDS auth caps, exported to it in `OPEN`.
    pub fn handle(
        &mut self,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: MClientSession,
        cap_auths: Vec<MdsCapAuth>,
    ) -> SessionOutcome {
        if self.output_failed() {
            return SessionOutcome::Done;
        }
        match msg.op {
            SessionOp::REQUEST_OPEN => {
                if self
                    .terminal
                    .iter()
                    .any(|output| output.blocks_reopen && output.session.client == client)
                {
                    return SessionOutcome::Done;
                }
                if let Some(s) = self.sessions.get_mut(&client) {
                    if !s.is_open() {
                        return SessionOutcome::Done;
                    }
                    // C++ Server::handle_client_session: notify only an open session,
                    // only when the duplicate request advertises NOTIFY_SESSION_STATE.
                    s.addr = link.peer_addr();
                    s.link = link;
                    if s.state == SessionState::Open
                        && msg
                            .supported_features
                            .test(proto::features::bit::NOTIFY_SESSION_STATE)
                    {
                        s.send_open();
                    }
                    return SessionOutcome::Done;
                }
                self.next_generation += 1;
                let mut s = self.new_funnel(client, self.next_generation, link);
                s.metadata = msg.metadata;
                s.features = msg.supported_features;
                s.cap_auths = cap_auths;
                s.send_open();
                info!(
                    client,
                    host = s
                        .metadata
                        .get("hostname")
                        .map(String::as_str)
                        .unwrap_or("?"),
                    root = s.metadata.get("root").map(String::as_str).unwrap_or("/"),
                    "session opened"
                );
                self.sessions.insert(client, s);
                SessionOutcome::Opened
            }
            SessionOp::REQUEST_RENEWCAPS => {
                let Some(s) = self.sessions.get_mut(&client) else {
                    // REJECT is valid only for OPEN/reconnect. Ignore late renewals.
                    return SessionOutcome::Done;
                };
                if !s.is_open() {
                    return SessionOutcome::Done;
                }
                s.last_renew = Instant::now();
                if s.state == SessionState::Stale {
                    s.state = SessionState::Open;
                }
                s.send(&MClientSession::renewcaps(msg.seq));
                SessionOutcome::Renewed
            }
            SessionOp::REQUEST_CLOSE => {
                let Some(s) = self.sessions.get_mut(&client) else {
                    return SessionOutcome::Done;
                };
                if !s.is_open() {
                    return SessionOutcome::Done;
                }
                if msg.seq < s.push_seq {
                    debug!(
                        client,
                        seq = msg.seq,
                        push_seq = s.push_seq,
                        "close with an old push seq: ignored"
                    );
                    return SessionOutcome::Done;
                }
                if msg.seq != s.push_seq {
                    warn!(
                        client,
                        seq = msg.seq,
                        push_seq = s.push_seq,
                        "incorrect push seq on close: dropped"
                    );
                    return SessionOutcome::Done;
                }
                s.state = SessionState::Closing;
                s.send(&MClientSession::close());
                s.state = SessionState::Closed;
                info!(client, "session closed");
                if let Some(s) = self.sessions.remove(&client) {
                    self.retain_terminal(s, true);
                }
                SessionOutcome::Closed
            }
            SessionOp::FLUSHMSG_ACK => SessionOutcome::Done,
            SessionOp::REQUEST_FLUSH_MDLOG => {
                if self.sessions.get(&client).is_some_and(Session::is_open) {
                    SessionOutcome::FlushMdlog
                } else {
                    SessionOutcome::Done
                }
            }
            other => {
                warn!(
                    client,
                    op = other.name(),
                    "unexpected session op from a client"
                );
                SessionOutcome::ProtocolError
            }
        }
    }

    /// Explicitly discard runtime output after verified fencing. Terminal CLOSE
    /// output has a separate owner and cannot be removed through this method.
    /// Rank-wide output failure is sticky even when a runtime funnel is discarded.
    pub fn remove(&mut self, client: u64) -> bool {
        self.sessions.remove(&client).is_some()
    }

    fn retain_terminal(&mut self, session: Session, blocks_reopen: bool) {
        if session.queued.is_empty() && !session.output_failed {
            return;
        }
        if self.terminal.len() >= MAX_TERMINAL_FUNNELS {
            // This is an explicit discard on terminal capacity failure. The sticky
            // rank failure prevents admission or successful service afterward.
            self.rank_output_failed.store(true, Ordering::Relaxed);
            return;
        }
        self.terminal.push_back(TerminalOutput {
            session,
            blocks_reopen,
        });
    }
}

fn message_bytes(raw: &RawMessage) -> usize {
    raw.front.len() + raw.middle.len() + raw.data.len() + 64
}

impl Drop for Session {
    fn drop(&mut self) {
        let bytes: usize = self.queued.iter().map(|(_, bytes)| bytes).sum();
        self.queued_bytes.fetch_sub(bytes, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<RawMessage>>>);
    impl ClientLink for Recorder {
        fn send_raw(&self, msg: RawMessage) -> bool {
            self.0.lock().unwrap().push(msg);
            true
        }
        fn features(&self) -> u64 {
            denc::features::ALL
        }
        fn peer_addr(&self) -> EntityAddr {
            EntityAddr::new(denc::AddrType::LEGACY, "127.0.0.1:6789".parse().unwrap(), 1)
        }
    }

    fn sent_ops(r: &Recorder) -> Vec<SessionOp> {
        r.0.lock()
            .unwrap()
            .iter()
            .filter(|m| m.msg_type == proto::msg_type::CLIENT_SESSION)
            .map(|m| MClientSession::from_raw(m.clone()).unwrap().op)
            .collect()
    }

    #[test]
    fn sessionless_renewal_has_no_reply() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        map.handle(
            42,
            Box::new(r.clone()),
            MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 7),
            vec![],
        );
        assert!(sent_ops(&r).is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn flush_request_has_no_ack_and_unknown_op_is_protocol_error() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        map.handle(
            42,
            Box::new(r.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        assert_eq!(
            map.handle(
                42,
                Box::new(r.clone()),
                MClientSession::new(SessionOp::REQUEST_FLUSH_MDLOG, 9),
                vec![]
            ),
            SessionOutcome::FlushMdlog
        );
        assert_eq!(sent_ops(&r), vec![SessionOp::OPEN]);
        // A server-only opcode is malformed client protocol. Never answer REJECT.
        assert_eq!(
            map.handle(
                42,
                Box::new(r.clone()),
                MClientSession::new(SessionOp::OPEN, 0),
                vec![]
            ),
            SessionOutcome::ProtocolError
        );
        assert_eq!(sent_ops(&r), vec![SessionOp::OPEN]);
        assert!(map.get(42).unwrap().is_open());
    }

    #[test]
    fn paused_funnel_preserves_order_and_counts_only_transmitted_pushes() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        map.handle(
            42,
            Box::new(r.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        map.pause_output(true);
        let caps = RawMessage::new(proto::msg_type::CLIENT_CAPS, 1, 1, denc::Bytes::new());
        assert!(map.get_mut(42).unwrap().send_raw(caps));
        assert!(
            map.get_mut(42)
                .unwrap()
                .send(&MClientSession::renewcaps(19))
        );
        assert_eq!(r.0.lock().unwrap().len(), 1);
        assert_eq!(map.get(42).unwrap().push_seq(), 0);
        map.pause_output(false);
        map.drain_output(1);
        assert_eq!(r.0.lock().unwrap().len(), 2);
        assert_eq!(map.get(42).unwrap().push_seq(), 1);
        // New messages cannot overtake the remaining queued renewal.
        map.get_mut(42)
            .unwrap()
            .send(&MClientSession::renewcaps(20));
        map.drain_output(1);
        let raw = r.0.lock().unwrap().last().unwrap().clone();
        assert_eq!(MClientSession::from_raw(raw).unwrap().seq, 19);
        map.drain_output(1);
        let raw = r.0.lock().unwrap().last().unwrap().clone();
        assert_eq!(MClientSession::from_raw(raw).unwrap().seq, 20);
        assert_eq!(map.get(42).unwrap().push_seq(), 1);
        assert_eq!(map.queued_bytes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn output_budget_failure_retains_push_sequence_and_releases_budget_on_remove() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        map.handle(
            42,
            Box::new(r.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        map.pause_output(true);
        map.get_mut(42).unwrap().send_raw(RawMessage::new(
            proto::msg_type::CLIENT_CAPS,
            1,
            1,
            denc::Bytes::new(),
        ));
        let big = RawMessage::new(
            proto::msg_type::CLIENT_CAPS,
            1,
            1,
            denc::Bytes::from(vec![0; MAX_QUEUED_OUTPUT_BYTES]),
        );
        assert!(!map.get_mut(42).unwrap().send_raw(big));
        assert!(map.output_failed());
        assert_eq!(map.get(42).unwrap().push_seq(), 0);
        map.remove(42);
        assert_eq!(map.queued_bytes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn duplicate_open_requires_notify_and_preserves_push_seq() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        let open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
        map.handle(42, Box::new(r.clone()), open.clone(), vec![]);
        map.get_mut(42).unwrap().send_raw(RawMessage::new(
            proto::msg_type::CLIENT_CAPS,
            1,
            1,
            denc::Bytes::new(),
        ));
        map.handle(42, Box::new(r.clone()), open.clone(), vec![]);
        assert_eq!(sent_ops(&r), vec![SessionOp::OPEN]);
        let mut notify = open;
        notify
            .supported_features
            .insert(proto::features::bit::NOTIFY_SESSION_STATE);
        map.handle(42, Box::new(r.clone()), notify.clone(), vec![]);
        let raw = r.0.lock().unwrap().last().unwrap().clone();
        assert_eq!(MClientSession::from_raw(raw).unwrap().seq, 1);
        map.get_mut(42).unwrap().state = SessionState::Stale;
        map.handle(42, Box::new(r.clone()), notify, vec![]);
        assert_eq!(sent_ops(&r), vec![SessionOp::OPEN, SessionOp::OPEN]);
    }

    #[test]
    fn close_retains_funnel_until_older_output_and_close_are_transmitted() {
        for paused_at_close in [true, false] {
            let mut map = SessionMap::default();
            let r = Recorder::default();
            map.handle(
                42,
                Box::new(r.clone()),
                MClientSession::new(SessionOp::REQUEST_OPEN, 0),
                vec![],
            );
            let generation = map.get(42).unwrap().generation;
            map.pause_output(true);
            map.get_mut(42)
                .unwrap()
                .send(&MClientSession::renewcaps(77));
            map.pause_output(paused_at_close);
            assert_eq!(
                map.handle(
                    42,
                    Box::new(r.clone()),
                    MClientSession::new(SessionOp::REQUEST_CLOSE, 0),
                    vec![]
                ),
                SessionOutcome::Closed
            );
            assert!(map.get(42).is_none());
            assert_eq!(map.terminal.front().unwrap().session.generation, generation);
            assert!(!map.terminal.front().unwrap().session.is_open());
            assert!(!map.remove(42), "terminal output must keep its owner");
            assert_eq!(
                map.handle(
                    42,
                    Box::new(r.clone()),
                    MClientSession::new(SessionOp::REQUEST_OPEN, 0),
                    vec![]
                ),
                SessionOutcome::Done
            );
            map.pause_output(false);
            map.drain_output(1);
            let last = r.0.lock().unwrap().last().unwrap().clone();
            assert_eq!(MClientSession::from_raw(last).unwrap().seq, 77);
            assert_eq!(map.terminal.len(), 1);
            map.drain_output(1);
            assert_eq!(
                sent_ops(&r),
                vec![SessionOp::OPEN, SessionOp::RENEWCAPS, SessionOp::CLOSE]
            );
            assert!(map.get(42).is_none());
            assert_eq!(map.queued_output_bytes(), 0);
            map.drain_output(9);
            assert_eq!(
                sent_ops(&r)
                    .iter()
                    .filter(|op| **op == SessionOp::CLOSE)
                    .count(),
                1
            );
            assert_eq!(
                map.handle(
                    42,
                    Box::new(r.clone()),
                    MClientSession::new(SessionOp::REQUEST_OPEN, 0),
                    vec![]
                ),
                SessionOutcome::Opened
            );
            assert!(map.get(42).unwrap().generation > generation);
        }
    }

    #[test]
    fn terminal_send_failure_survives_removal_and_blocks_new_admission() {
        #[derive(Clone)]
        struct FailClose(Recorder);
        impl ClientLink for FailClose {
            fn send_raw(&self, raw: RawMessage) -> bool {
                if raw.msg_type == proto::msg_type::CLIENT_SESSION
                    && MClientSession::from_raw(raw.clone()).unwrap().op == SessionOp::CLOSE
                {
                    return false;
                }
                self.0.send_raw(raw)
            }
            fn features(&self) -> u64 {
                self.0.features()
            }
            fn peer_addr(&self) -> EntityAddr {
                self.0.peer_addr()
            }
        }
        for queued in [false, true] {
            let mut map = SessionMap::default();
            let r = Recorder::default();
            let link = FailClose(r.clone());
            map.handle(
                42,
                Box::new(link.clone()),
                MClientSession::new(SessionOp::REQUEST_OPEN, 0),
                vec![],
            );
            map.pause_output(queued);
            map.handle(
                42,
                Box::new(link),
                MClientSession::new(SessionOp::REQUEST_CLOSE, 0),
                vec![],
            );
            map.pause_output(false);
            map.drain_output(1);
            assert!(map.output_failed());
            map.remove(42);
            assert!(map.output_failed());
            let count = r.0.lock().unwrap().len();
            assert_eq!(
                map.handle(
                    42,
                    Box::new(r.clone()),
                    MClientSession::new(SessionOp::REQUEST_OPEN, 0),
                    vec![]
                ),
                SessionOutcome::Done
            );
            assert_eq!(r.0.lock().unwrap().len(), count);
            assert!(map.get(42).is_none_or(|s| !s.is_open()));
        }
    }

    #[test]
    fn duplicate_open_sequence_binds_to_partial_drain_position() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        map.handle(
            42,
            Box::new(r.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        let push = || RawMessage::new(proto::msg_type::CLIENT_CAPS, 1, 1, denc::Bytes::new());
        map.get_mut(42).unwrap().send_raw(push()); // transmitted baseline
        map.pause_output(true);
        map.get_mut(42).unwrap().send_raw(push());
        map.get_mut(42).unwrap().send_raw(push());
        let mut open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
        open.supported_features
            .insert(proto::features::bit::NOTIFY_SESSION_STATE);
        map.handle(42, Box::new(r.clone()), open, vec![]);
        map.get_mut(42).unwrap().send_raw(push()); // must not count in OPEN.seq
        map.get_mut(42)
            .unwrap()
            .send(&MClientSession::renewcaps(91));
        map.pause_output(false);
        map.drain_output(1);
        assert_eq!(map.get(42).unwrap().push_seq(), 2);
        map.drain_output(2);
        let raw = r.0.lock().unwrap().last().unwrap().clone();
        let open = MClientSession::from_raw(raw).unwrap();
        assert_eq!((open.op, open.seq), (SessionOp::OPEN, 3));
        map.drain_output(2);
        assert_eq!(map.get(42).unwrap().push_seq(), 4);
        let raw = r.0.lock().unwrap().last().unwrap().clone();
        assert_eq!(MClientSession::from_raw(raw).unwrap().seq, 91);
    }

    #[test]
    fn rejection_has_bounded_terminal_owner_but_does_not_install_or_reserve_session() {
        let mut map = SessionMap::default();
        let rejected = Recorder::default();
        map.pause_output(true);
        map.reject_blocklisted(42, Box::new(rejected.clone()));
        assert_eq!(map.len(), 0);
        assert_eq!(map.next_generation, 0);
        assert_eq!(map.terminal.len(), 1);
        assert!(map.queued_output_bytes() > 0);
        assert!(sent_ops(&rejected).is_empty());
        let clean = Recorder::default();
        assert_eq!(
            map.handle(
                42,
                Box::new(clean.clone()),
                MClientSession::new(SessionOp::REQUEST_OPEN, 0),
                vec![]
            ),
            SessionOutcome::Opened
        );
        assert_eq!(map.get(42).unwrap().generation, 1);
        map.pause_output(false);
        map.drain_output(1);
        assert_eq!(sent_ops(&rejected), vec![SessionOp::REJECT]);
        assert!(sent_ops(&clean).is_empty());
        assert!(map.terminal.is_empty());
        map.drain_output(1);
        assert_eq!(sent_ops(&clean), vec![SessionOp::OPEN]);
        assert_eq!(map.queued_output_bytes(), 0);
    }

    #[test]
    fn terminal_capacity_discard_is_explicit_fail_stop_with_no_admission() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        map.pause_output(true);
        for _ in 0..MAX_TERMINAL_FUNNELS {
            map.reject_blocklisted(42, Box::new(r.clone()));
        }
        assert_eq!(map.terminal.len(), MAX_TERMINAL_FUNNELS);
        assert!(!map.output_failed());
        map.handle(
            43,
            Box::new(r.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        // Runtime retirement cannot grow an already full terminal-owner set.
        map.handle(
            43,
            Box::new(r.clone()),
            MClientSession::new(SessionOp::REQUEST_CLOSE, 0),
            vec![],
        );
        assert_eq!(map.terminal.len(), MAX_TERMINAL_FUNNELS);
        assert!(map.output_failed());
        assert!(map.get(43).is_none());
        map.handle(
            43,
            Box::new(r.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        assert!(map.get(43).is_none());
        map.pause_output(false);
        map.drain_output(MAX_TERMINAL_FUNNELS);
        assert!(sent_ops(&r).is_empty());
        let bytes = map.queued_bytes.clone();
        drop(map);
        assert_eq!(bytes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn open_renew_close() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        let open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
        assert_eq!(
            map.handle(4242, Box::new(r.clone()), open, vec![]),
            SessionOutcome::Opened
        );
        assert_eq!(
            map.handle(
                4242,
                Box::new(r.clone()),
                MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 7),
                vec![]
            ),
            SessionOutcome::Renewed
        );
        let msgs = r.0.lock().unwrap().clone();
        let renew = MClientSession::from_raw(msgs[1].clone()).unwrap();
        assert_eq!((renew.op, renew.seq), (SessionOp::RENEWCAPS, 7));

        // A counted message bumps push_seq; a close with the old seq is ignored.
        let s = map.get_mut(4242).unwrap();
        let mut caps = RawMessage::new(proto::msg_type::CLIENT_CAPS, 1, 1, denc::Bytes::new());
        caps.tid = 0;
        s.send_raw(caps);
        assert_eq!(map.get(4242).unwrap().push_seq(), 1);
        assert_eq!(
            map.handle(
                4242,
                Box::new(r.clone()),
                MClientSession::new(SessionOp::REQUEST_CLOSE, 0),
                vec![]
            ),
            SessionOutcome::Done
        );
        assert_eq!(
            map.handle(
                4242,
                Box::new(r.clone()),
                MClientSession::new(SessionOp::REQUEST_CLOSE, 1),
                vec![]
            ),
            SessionOutcome::Closed
        );
        assert_eq!(
            sent_ops(&r),
            vec![SessionOp::OPEN, SessionOp::RENEWCAPS, SessionOp::CLOSE]
        );
        assert_eq!(map.len(), 0);
    }
}
