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

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use denc::{Message, RawMessage};
use proto::{FeatureBitset, MClientSession, MdsCapAuth, SessionOp, is_push_seq_counted};
use tracing::{debug, info, warn};

/// Session states (`Session::STATE_*`), reduced to what this MDS uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Open,
    Stale,
    Closing,
    Closed,
}

/// Where a session's messages go. Implemented by `msgr::Connection`; a test double records them.
pub trait ClientLink: Send + Sync {
    /// Queue a message on the connection (in order). Returns false if the connection is gone.
    fn send_raw(&self, msg: RawMessage) -> bool;
    /// Connection features negotiated with the peer.
    fn features(&self) -> u64;
}

impl ClientLink for msgr::Connection {
    fn send_raw(&self, msg: RawMessage) -> bool {
        self.send(msg)
    }
    fn features(&self) -> u64 {
        msgr::Connection::features(self)
    }
}

pub struct Session {
    /// Client id: the cephx global id (`entity_name_t` number of `client.<gid>`).
    pub client: u64,
    pub state: SessionState,
    pub link: Box<dyn ClientLink>,
    /// Client metadata from `REQUEST_OPEN` (`hostname`, `root`, `entity_id`, `uuid`, `timeout`, ...).
    pub metadata: BTreeMap<String, String>,
    /// CephFS features the client advertised.
    pub features: FeatureBitset,
    pub cap_auths: Vec<MdsCapAuth>,
    /// Counted messages sent (caps, snap, lease, quota).
    push_seq: u64,
    pub last_renew: Instant,
}

impl Session {
    /// Send a typed message to the client through the session's ordered funnel.
    pub fn send<M: Message>(&mut self, msg: &M) -> bool {
        let raw = msg.to_raw(self.link.features());
        self.send_raw(raw)
    }

    /// Send an already encoded message through the funnel.
    pub fn send_raw(&mut self, raw: RawMessage) -> bool {
        if is_push_seq_counted(raw.msg_type) {
            self.push_seq += 1;
        }
        self.link.send_raw(raw)
    }

    pub fn push_seq(&self) -> u64 {
        self.push_seq
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
    /// The session closed: the caller must drop the client's caps, locks and requests.
    Closed,
}

/// All sessions of this rank, by client id.
#[derive(Default)]
pub struct SessionMap {
    sessions: HashMap<u64, Session>,
}

impl SessionMap {
    pub fn get_mut(&mut self, client: u64) -> Option<&mut Session> {
        self.sessions.get_mut(&client)
    }

    pub fn get(&self, client: u64) -> Option<&Session> {
        self.sessions.get(&client)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Session> {
        self.sessions.values_mut()
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
        match msg.op {
            SessionOp::REQUEST_OPEN => {
                if let Some(s) = self.sessions.get_mut(&client)
                    && s.is_open()
                {
                    // Duplicate open (NOTIFY_SESSION_STATE): answer OPEN again on the new link.
                    s.link = link;
                    let reply = MClientSession::open_default(0, s.cap_auths.clone());
                    s.send(&reply);
                    debug!(client, "duplicate REQUEST_OPEN answered with OPEN");
                    return SessionOutcome::Done;
                }
                let mut s = Session {
                    client,
                    state: SessionState::Open,
                    link,
                    metadata: msg.metadata,
                    features: msg.supported_features,
                    cap_auths,
                    push_seq: 0,
                    last_renew: Instant::now(),
                };
                let reply = MClientSession::open_default(0, s.cap_auths.clone());
                s.send(&reply);
                info!(
                    client,
                    host = s.metadata.get("hostname").map(String::as_str).unwrap_or("?"),
                    root = s.metadata.get("root").map(String::as_str).unwrap_or("/"),
                    "session opened"
                );
                self.sessions.insert(client, s);
                SessionOutcome::Opened
            }
            SessionOp::REQUEST_RENEWCAPS => {
                let Some(s) = self.sessions.get_mut(&client) else {
                    // A renewal without a session: the client has lost its session.
                    let _ = link.send_raw(MClientSession::reject("sessionless", false).to_raw(link.features()));
                    return SessionOutcome::Done;
                };
                s.last_renew = Instant::now();
                if s.state == SessionState::Stale {
                    s.state = SessionState::Open;
                }
                s.send(&MClientSession::renewcaps(msg.seq));
                SessionOutcome::Done
            }
            SessionOp::REQUEST_CLOSE => {
                let Some(s) = self.sessions.get_mut(&client) else {
                    let _ = link.send_raw(MClientSession::close().to_raw(link.features()));
                    return SessionOutcome::Done;
                };
                if msg.seq < s.push_seq {
                    debug!(client, seq = msg.seq, push_seq = s.push_seq, "close with an old push seq: ignored");
                    return SessionOutcome::Done;
                }
                if msg.seq != s.push_seq {
                    warn!(client, seq = msg.seq, push_seq = s.push_seq, "incorrect push seq on close: dropped");
                    return SessionOutcome::Done;
                }
                s.state = SessionState::Closed;
                s.send(&MClientSession::close());
                info!(client, "session closed");
                self.sessions.remove(&client);
                SessionOutcome::Closed
            }
            SessionOp::FLUSHMSG_ACK | SessionOp::REQUEST_FLUSH_MDLOG => {
                // Commits are immediate (safe replies only), so there is nothing to flush.
                SessionOutcome::Done
            }
            other => {
                warn!(client, op = other.name(), "unexpected session op from a client");
                SessionOutcome::Done
            }
        }
    }

    /// Forget a client whose connection died for good (no reconnect path yet).
    pub fn remove(&mut self, client: u64) -> bool {
        self.sessions.remove(&client).is_some()
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
    fn open_renew_close() {
        let mut map = SessionMap::default();
        let r = Recorder::default();
        let open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
        assert_eq!(map.handle(4242, Box::new(r.clone()), open, vec![]), SessionOutcome::Opened);
        assert_eq!(map.handle(4242, Box::new(r.clone()), MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 7), vec![]), SessionOutcome::Done);
        let msgs = r.0.lock().unwrap().clone();
        let renew = MClientSession::from_raw(msgs[1].clone()).unwrap();
        assert_eq!((renew.op, renew.seq), (SessionOp::RENEWCAPS, 7));

        // A counted message bumps push_seq; a close with the old seq is ignored.
        let s = map.get_mut(4242).unwrap();
        let mut caps = RawMessage::new(proto::msg_type::CLIENT_CAPS, 1, 1, denc::Bytes::new());
        caps.tid = 0;
        s.send_raw(caps);
        assert_eq!(map.get(4242).unwrap().push_seq(), 1);
        assert_eq!(map.handle(4242, Box::new(r.clone()), MClientSession::new(SessionOp::REQUEST_CLOSE, 0), vec![]), SessionOutcome::Done);
        assert_eq!(map.handle(4242, Box::new(r.clone()), MClientSession::new(SessionOp::REQUEST_CLOSE, 1), vec![]), SessionOutcome::Closed);
        assert_eq!(sent_ops(&r), vec![SessionOp::OPEN, SessionOp::RENEWCAPS, SessionOp::CLOSE]);
        assert_eq!(map.len(), 0);
    }
}
