//! CephFS client protocol messages (`MClient*`) and their encodings.
//!
//! Wire formats follow the C++ sources of Ceph `main` (`src/messages/MClient*.h`,
//! `src/include/ceph_fs.h`, `src/mds/CInode.cc`, `src/mds/Server.cc`) in their **current**
//! forms only: the versioned reply encodings (`CEPHFS_FEATURE_REPLY_ENCODING`), request message
//! v6 with request head v3, single-message reconnect v3 and multi-part v5. Legacy encodings
//! (88-byte request head, connection-feature-conditional `InodeStat`, raw created-ino extras,
//! reconnect v1/v2) are not implemented (design D1).
//!
//! Every message implements [`denc::Message`]; use `to_raw`/`from_raw` (some messages pick
//! their header version from their content, and requests, replies, forwards and cap messages
//! carry their tid in the header). Incoming client messages can be decoded in one step with
//! [`ClientMessage::from_raw`].
//!
//! | Message | Type | Direction | Module |
//! |---|---|---|---|
//! | `MClientSession` | 22 | both | [`session`] |
//! | `MClientReconnect` | 23 | client to MDS | [`reconnect`] |
//! | `MClientRequest` | 24 | client to MDS | [`request`] |
//! | `MClientRequestForward` | 25 | MDS to client | [`forward`] |
//! | `MClientReply` | 26 | MDS to client | [`reply`] |
//! | `MClientReclaim` / `MClientReclaimReply` | 27 / 28 | client to MDS / back | [`reclaim`] |
//! | `MClientMetrics` | 29 | client to MDS | [`metrics`] |
//! | `MClientCaps` | 0x310 | both | [`caps`] |
//! | `MClientLease` | 0x311 | both | [`lease`] |
//! | `MClientSnap` | 0x312 | MDS to client | [`snap`] |
//! | `MClientCapRelease` | 0x313 | client to MDS | [`cap_release`] |
//! | `MClientQuota` | 0x314 | MDS to client | [`quota`] |
//!
//! # Per-session choices the caller makes
//!
//! - The session `OPEN` advertises [`features::MDS_SUPPORTED_FEATURES`] (bits 0-14 and 16-21)
//!   and [`features::MDS_METRIC_SPEC`] ([`MClientSession::open_default`]); the encodings in this
//!   crate assume the client then uses the current paths those bits select.
//! - Snap traces (replies, `MClientSnap`, cap `IMPORT`) use [`SnapFormat::for_client`] of the
//!   client's features: the old format for the kernel, `SnapRealmInfoNew` for libcephfs.
//! - Readdir order: entries of a frag sorted by [`frag::dentry_key_cmp`] of
//!   [`hash::hash_dentry_name`] with the directory's `dir_layout.dir_hash`.
//! - Messages counted in `push_seq`: [`is_push_seq_counted`].
//! - Before sending a reply, [`MClientReply::check_invariants_for`] lists violations of the
//!   client invariants (design section 7) the reply itself can show.
//!
//! # Example: a LOOKUP reply
//!
//! ```
//! use denc::{Message, UTime};
//! use proto::*;
//!
//! // The request as decoded from the wire: LOOKUP "f" in the root directory.
//! let mut req = MClientRequest::new(7, MdsOp::LOOKUP);
//! req.path = FilePath::new(CEPH_INO_ROOT, "f");
//!
//! let root = InodeStat {
//!     ino: CEPH_INO_ROOT,
//!     mode: reply::mode::S_IFDIR | 0o755,
//!     nlink: 1,
//!     dir_layout: DirLayout { dir_hash: hash::CEPH_STR_HASH_RJENKINS },
//!     cap: ReplyCap { caps: cap::PIN | cap::ANY_SHARED, cap_id: 1, seq: 1, realm: CEPH_INO_ROOT,
//!                     flags: CEPH_CAP_FLAG_AUTH, ..Default::default() },
//!     ..Default::default()
//! };
//! let file = InodeStat {
//!     ino: 0x100_0000_0000,
//!     mode: reply::mode::S_IFREG | 0o644,
//!     nlink: 1,
//!     size: 12,
//!     layout: FileLayout::default_for_pool(2),
//!     ctime: UTime { sec: 1_760_000_000, nsec: 0 },
//!     ..Default::default()
//! };
//! let mut reply = MClientReply::for_request(&req, 0);
//! reply.mdsmap_epoch = 12;
//! reply.trace = Trace {
//!     dentry: Some(TraceDentry { dir: root, dirstat: DirStat::root(0), name: "f".into(), lease: LeaseStat::null() }),
//!     target: Some(file),
//! };
//! let client_features = FeatureBitset::from_word(features::KERNEL_7_0_FEATURES);
//! reply.set_snap_trace(&SnapTrace::new(vec![SnapRealmInfo::new(CEPH_INO_ROOT, 0, 1)]),
//!                      SnapFormat::for_client(&client_features));
//! assert!(reply.check_invariants_for(&req).is_empty());
//! let raw = reply.to_raw(0); // type 26, version 1, tid 7
//! assert_eq!(MClientReply::from_raw(raw).unwrap(), reply);
//! ```

mod util;

pub mod cap_release;
pub mod caps;
pub mod features;
pub mod forward;
pub mod frag;
pub mod hash;
pub mod lease;
pub mod metrics;
pub mod quota;
pub mod reclaim;
pub mod reconnect;
pub mod reply;
pub mod request;
pub mod session;
pub mod snap;
pub mod types;
pub mod wire;

pub use cap_release::{CapItem, MClientCapRelease};
pub use caps::{CapInodeFields, CapOp, CapPeer, MClientCaps};
pub use features::{FeatureBitset, MetricSpec};
pub use forward::MClientRequestForward;
pub use frag::{Frag, FragTree};
pub use lease::{LeaseAction, MClientLease};
pub use metrics::{ClientMetric, MClientMetrics};
pub use quota::MClientQuota;
pub use reclaim::{MClientReclaim, MClientReclaimReply};
pub use reconnect::{CapReconnect, FlockState, MClientReconnect, SnaprealmReconnect};
pub use reply::{
    DirStat, InodeStat, LeaseStat, MClientReply, OpencResponse, ReaddirEntry, ReaddirReply,
    ReplyCap, ReplyExtra, Trace, TraceDentry,
};
pub use request::{MClientRequest, MdsOp, RequestArgs, RequestHead, RequestRelease};
pub use session::{MClientSession, MdsCapAuth, MdsCapMatch, SessionOp};
pub use snap::{MClientSnap, SnapFormat, SnapOp, SnapRealmInfo, SnapTrace};
pub use types::*;

use denc::{Message, RawMessage};

/// Client message types (`CEPH_MSG_CLIENT_*`, `include/ceph_fs.h`).
pub mod msg_type {
    pub const CLIENT_SESSION: u16 = 22;
    pub const CLIENT_RECONNECT: u16 = 23;
    pub const CLIENT_REQUEST: u16 = 24;
    pub const CLIENT_REQUEST_FORWARD: u16 = 25;
    pub const CLIENT_REPLY: u16 = 26;
    pub const CLIENT_RECLAIM: u16 = 27;
    pub const CLIENT_RECLAIM_REPLY: u16 = 28;
    pub const CLIENT_METRICS: u16 = 29;
    pub const CLIENT_CAPS: u16 = 0x310;
    pub const CLIENT_LEASE: u16 = 0x311;
    pub const CLIENT_SNAP: u16 = 0x312;
    pub const CLIENT_CAPRELEASE: u16 = 0x313;
    pub const CLIENT_QUOTA: u16 = 0x314;
    pub const CLIENT_QUARANTINE_DISABLE: u16 = 0x315;
}

/// Whether an MDS-to-client message of this type counts towards the session's `push_seq`
/// (`MDSRank::send_message_client_counted`): caps, snap, lease and quota messages. Both clients
/// count exactly these, and `REQUEST_CLOSE` is accepted only when the counts agree.
pub const fn is_push_seq_counted(msg_type: u16) -> bool {
    matches!(
        msg_type,
        msg_type::CLIENT_CAPS
            | msg_type::CLIENT_SNAP
            | msg_type::CLIENT_LEASE
            | msg_type::CLIENT_QUOTA
    )
}

/// A message a client may send to the MDS (the C++ MDS accepts these from client peers).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientMessage {
    Session(MClientSession),
    Reconnect(MClientReconnect),
    Request(MClientRequest),
    Reclaim(MClientReclaim),
    Metrics(MClientMetrics),
    Caps(MClientCaps),
    Lease(MClientLease),
    CapRelease(MClientCapRelease),
}

impl ClientMessage {
    /// Decode a message received from a client. `Ok(None)` for a type clients do not send to
    /// an MDS (the caller drops it).
    pub fn from_raw(raw: RawMessage) -> denc::Result<Option<ClientMessage>> {
        Ok(Some(match raw.msg_type {
            msg_type::CLIENT_SESSION => ClientMessage::Session(MClientSession::from_raw(raw)?),
            msg_type::CLIENT_RECONNECT => {
                ClientMessage::Reconnect(MClientReconnect::from_raw(raw)?)
            }
            msg_type::CLIENT_REQUEST => ClientMessage::Request(MClientRequest::from_raw(raw)?),
            msg_type::CLIENT_RECLAIM => ClientMessage::Reclaim(MClientReclaim::from_raw(raw)?),
            msg_type::CLIENT_METRICS => ClientMessage::Metrics(MClientMetrics::from_raw(raw)?),
            msg_type::CLIENT_CAPS => ClientMessage::Caps(MClientCaps::from_raw(raw)?),
            msg_type::CLIENT_LEASE => ClientMessage::Lease(MClientLease::from_raw(raw)?),
            msg_type::CLIENT_CAPRELEASE => {
                ClientMessage::CapRelease(MClientCapRelease::from_raw(raw)?)
            }
            _ => return Ok(None),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch() {
        let raw = MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 4).to_raw(0);
        assert!(matches!(
            ClientMessage::from_raw(raw).unwrap(),
            Some(ClientMessage::Session(_))
        ));
        let raw = MClientCapRelease::default().to_raw(0);
        assert!(matches!(
            ClientMessage::from_raw(raw).unwrap(),
            Some(ClientMessage::CapRelease(_))
        ));
        let raw = MClientQuota::default().to_raw(0);
        assert_eq!(ClientMessage::from_raw(raw).unwrap(), None);
        let mut raw = MClientRequest::new(9, MdsOp::GETATTR).to_raw(0);
        raw.tid = 10;
        match ClientMessage::from_raw(raw).unwrap() {
            Some(ClientMessage::Request(r)) => assert_eq!(r.tid, 10),
            other => panic!("{other:?}"),
        }
        assert!(is_push_seq_counted(msg_type::CLIENT_CAPS));
        assert!(!is_push_seq_counted(msg_type::CLIENT_REPLY));
        assert!(!is_push_seq_counted(msg_type::CLIENT_SESSION));
    }
}
