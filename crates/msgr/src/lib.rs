//! Ceph messenger: msgr1 (and later msgr2) client and server connections on tokio.
//!
//! - [`Messenger::bind`] binds one msgr1 address (`TYPE_LEGACY`, random nonce) and accepts
//!   connections; [`Messenger::connect`] opens outgoing ones from the same identity.
//! - A [`Connection`] is one messenger *session*: lossless sessions (clients of an MDS use the
//!   `stateful_server` policy) survive socket faults, keep unacknowledged messages and replay
//!   them when the peer reconnects; lossy ones (monitors, OSDs) die with their socket.
//! - Everything the application needs to know arrives as an [`Event`] on a bounded channel;
//!   socket reading never waits for request execution beyond that channel's back-pressure.
//! - cephx comes from [`auth::AuthContext`]: authorizers for outgoing connections, and
//!   verification (with the one CEPHX_V2 challenge round) of incoming ones; messages are signed
//!   and checked when `MSG_AUTH` is negotiated.
//!
//! The wire protocol sits behind the internal `proto` traits so msgr2 can be added later.

mod conn;
mod messenger;
mod proto;
pub mod v1;
pub mod wire;

use std::sync::Arc;
use std::time::Duration;

use auth::{AuthCapsInfo, AuthName};
use denc::{EntityAddr, EntityName, EntityType, RawMessage, UTime, features};

pub use messenger::Messenger;

/// Messenger errors.
#[derive(Debug, thiserror::Error)]
pub enum MsgrError {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("bad {0} crc")]
    BadCrc(&'static str),
    #[error("bad message signature (seq {seq})")]
    BadSignature { seq: u64 },
    #[error("connection race lost (WAIT)")]
    Wait,
    #[error("connection closed")]
    Closed,
}

/// Connection policy (`src/msg/Policy.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Messages may be dropped on a fault; the session dies with the socket.
    pub lossy: bool,
    /// Never initiate a reconnect; wait in standby for the peer.
    pub server: bool,
    /// Keep a lossless session without a socket when idle.
    pub standby: bool,
    /// Detect peer session resets (`connect_seq` 0 against an existing session).
    pub resetcheck: bool,
    /// Feature bits the peer must have.
    pub features_required: u64,
}

impl Policy {
    pub const fn stateful_server(req: u64) -> Self {
        Policy {
            lossy: false,
            server: true,
            standby: true,
            resetcheck: true,
            features_required: req,
        }
    }
    pub const fn stateless_server(req: u64) -> Self {
        Policy {
            lossy: true,
            server: true,
            standby: false,
            resetcheck: false,
            features_required: req,
        }
    }
    pub const fn lossless_peer(req: u64) -> Self {
        Policy {
            lossy: false,
            server: false,
            standby: true,
            resetcheck: false,
            features_required: req,
        }
    }
    pub const fn lossless_client(req: u64) -> Self {
        Policy {
            lossy: false,
            server: false,
            standby: false,
            resetcheck: true,
            features_required: req,
        }
    }
    pub const fn lossy_client(req: u64) -> Self {
        Policy {
            lossy: true,
            server: false,
            standby: false,
            resetcheck: false,
            features_required: req,
        }
    }
}

/// Messenger configuration.
#[derive(Debug, Clone)]
pub struct MsgrConfig {
    /// Features advertised (`CEPH_FEATURES_SUPPORTED_DEFAULT`).
    pub features: u64,
    /// Policy per peer type.
    pub policies: Vec<(EntityType, Policy)>,
    pub default_policy: Policy,
    /// Capacity of the event channel(s).
    pub event_queue: usize,
    /// Upper bound for a handshake.
    pub handshake_timeout: Duration,
    /// Require `MSG_AUTH` from cephx peers (`cephx_require_signatures`, default false).
    pub require_signatures: bool,
    /// Refuse incoming connections without an authorizer (`Messenger::require_authorizer`,
    /// default true). Monitors accept msgr1 connections without one and authenticate with
    /// `MAuth` instead; tests that play a monitor turn this off.
    pub require_authorizer: bool,
    /// Address nonce; `None` picks a random one per process (daemons). Monitors use 0.
    pub nonce: Option<u32>,
}

impl MsgrConfig {
    /// The policies `ceph-mds` installs (`src/ceph_mds.cc`): mons `lossy_client(UID|PGID64)`,
    /// MDS peers `lossless_peer(UID)`, clients `stateful_server(0)`, anything else
    /// `lossy_client(OSDREPLYMUX)`. A client-type messenger (tests, tools) uses
    /// `lossless_client` towards MDSs as libcephfs does.
    pub fn for_entity(ty: EntityType) -> Self {
        let mut policies = vec![(
            EntityType::MON,
            Policy::lossy_client(features::UID | features::PGID64),
        )];
        let default_policy = if ty == EntityType::CLIENT {
            policies.push((EntityType::MDS, Policy::lossless_client(0)));
            Policy::lossy_client(0)
        } else {
            policies.push((EntityType::MDS, Policy::lossless_peer(features::UID)));
            policies.push((EntityType::CLIENT, Policy::stateful_server(0)));
            Policy::lossy_client(features::OSDREPLYMUX)
        };
        MsgrConfig {
            features: features::ALL,
            policies,
            default_policy,
            event_queue: 4096,
            handshake_timeout: Duration::from_secs(30),
            require_signatures: false,
            require_authorizer: true,
            nonce: None,
        }
    }

    pub fn policy_for(&self, ty: EntityType) -> Policy {
        self.policies
            .iter()
            .find(|(t, _)| *t == ty)
            .map(|(_, p)| *p)
            .unwrap_or(self.default_policy)
    }
}

/// Something that happened on a connection.
#[derive(Debug)]
pub enum Event {
    /// A message arrived (`msg.src` is the sender's name from the header).
    Message { conn: Connection, msg: RawMessage },
    /// An incoming connection completed its handshake with a verified authorizer. Raised again
    /// for the same connection when the peer reconnects to its session (`ms_handle_accept`).
    Accepted { conn: Connection },
    /// An outgoing connection completed its handshake (`ms_handle_connect`).
    Connected { conn: Connection },
    /// The session is gone: a lossy connection faulted, or the peer reset its session. Queued
    /// and unacknowledged messages are lost; the handle is dead (`ms_handle_reset`,
    /// `ms_handle_remote_reset`).
    Reset { conn: Connection },
    /// The connection was closed for good (peer sent `CLOSE`, or it was marked down).
    Closed { conn: Connection },
}

impl Event {
    pub fn conn(&self) -> &Connection {
        match self {
            Event::Message { conn, .. }
            | Event::Accepted { conn }
            | Event::Connected { conn }
            | Event::Reset { conn }
            | Event::Closed { conn } => conn,
        }
    }
}

/// Handle on one messenger session. Cheap to clone; equality is by session id.
#[derive(Clone)]
pub struct Connection(pub(crate) Arc<conn::ConnShared>);

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}
impl Eq for Connection {}
impl std::hash::Hash for Connection {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

impl Connection {
    /// Unique id of the session within this process.
    pub fn id(&self) -> u64 {
        self.0.id
    }

    /// Queue a message; the messenger assigns the sequence number at write time. Returns false
    /// if the connection is closed (the message is dropped). Never blocks.
    pub fn send(&self, msg: RawMessage) -> bool {
        self.0.send(msg)
    }

    /// Send a keepalive (`KEEPALIVE2` with the current time when the peer supports it).
    /// Never use it towards kernel clients, which reject keepalives from servers.
    pub fn send_keepalive(&self) {
        self.0.send_keepalive()
    }

    /// Close the connection for good and discard its queues (`mark_down`).
    pub fn mark_down(&self) {
        self.0.mark_down(false)
    }

    /// Peer entity type (claimed in the connect message, or the type we connected to).
    pub fn peer_type(&self) -> EntityType {
        self.0.st.lock().unwrap().peer_type
    }

    /// Peer legacy address (with nonce).
    pub fn peer_addr(&self) -> EntityAddr {
        self.0.st.lock().unwrap().peer_addr
    }

    /// Source name of the last message received (`ceph_msg_header.src`; unauthenticated).
    pub fn peer_name(&self) -> Option<EntityName> {
        self.0.st.lock().unwrap().peer_name
    }

    /// Authenticated principal (`peer_name` of the cephx ticket) of an incoming connection.
    pub fn peer_auth_name(&self) -> Option<AuthName> {
        self.0
            .st
            .lock()
            .unwrap()
            .peer_auth
            .as_ref()
            .map(|a| a.name.clone())
    }

    /// Authenticated global id (0 if none).
    pub fn peer_global_id(&self) -> u64 {
        self.0
            .st
            .lock()
            .unwrap()
            .peer_auth
            .as_ref()
            .map(|a| a.global_id)
            .unwrap_or(0)
    }

    /// The caps blob of the peer's ticket for our service type.
    pub fn peer_caps(&self) -> Option<AuthCapsInfo> {
        self.0
            .st
            .lock()
            .unwrap()
            .peer_auth
            .as_ref()
            .map(|a| a.caps.clone())
    }

    /// Negotiated features of the current (or last) socket.
    pub fn features(&self) -> u64 {
        self.0.st.lock().unwrap().features
    }

    pub fn is_open(&self) -> bool {
        self.0.st.lock().unwrap().phase == conn::Phase::Open
    }

    pub fn is_closed(&self) -> bool {
        self.0.st.lock().unwrap().phase == conn::Phase::Closed
    }

    /// Whether this side opened the connection.
    pub fn is_outgoing(&self) -> bool {
        self.0.outgoing
    }

    /// Timestamp echoed by the peer's last `KEEPALIVE2_ACK`.
    pub fn last_keepalive_ack(&self) -> Option<UTime> {
        self.0.st.lock().unwrap().last_keepalive_ack
    }

    /// Fail the current socket as if the network broke (tests and fault injection): lossy
    /// sessions reset, lossless servers wait in standby, lossless clients reconnect.
    #[doc(hidden)]
    pub fn inject_socket_failure(&self) {
        let sgen = self.0.st.lock().unwrap().sgen;
        self.0.fault(sgen, "injected socket failure");
    }

    /// (out_seq, in_seq, unacknowledged messages, queued messages), for diagnostics.
    pub fn seq_state(&self) -> (u64, u64, usize, usize) {
        let st = self.0.st.lock().unwrap();
        (st.out_seq, st.in_seq, st.sent.len(), st.out_q.len())
    }
}

#[cfg(test)]
mod tests;
