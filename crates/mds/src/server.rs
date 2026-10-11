//! The metadata server of one rank: translates client messages into engine calls and engine
//! effects into wire messages.
//!
//! **Status: stub.** The cluster side, sessions and the wire crates are done; the translation to
//! and from `engine` is the next step (see `HANDOVER.md`, "Next steps"). Until then every
//! client request is answered `-ENOSYS` (a kernel mount therefore fails at its first GETATTR),
//! and caps, cap releases, leases and metrics are accepted and ignored.
//!
//! What the full version does (the shape the daemon already calls):
//!
//! - `session_opened` / `session_closed`: `Engine::open_session` / `Engine::close_session`, then
//!   send the returned effects.
//! - `handle`: `ClientMessage::Request` → `engine::Request` → `Engine::handle_request`;
//!   `Caps` → `Engine::handle_cap_update`; `CapRelease` → `Engine::handle_cap_release`;
//!   `Lease` and `Metrics` are ignored (P0 issues null leases; metrics are tolerated);
//!   `Reconnect` is denied with `CLOSE` until persistence exists; `Reclaim` answers "unknown
//!   uuid" (result 0, no address), which FSAL_CEPH treats as ENOENT.
//! - Effects: `Reply` → `MClientReply` (engine `InodeStat`/`DirFragInfo`/`LeaseStat` → `proto`
//!   types; snap trace in `SnapFormat::for_client` of the session's features), `Caps` →
//!   `MClientCaps`, `RealmSplit` → `MClientSnap` (before the reply), `Recall` →
//!   `RECALL_STATE`; `TruncateData`, `PurgeInode`, `RecoverFileSize` need the OSD client
//!   (`osdc`), `EvictClient` needs an OSD blocklist through the monitor. Every message goes
//!   through `Session::send`, in the order the engine returned the effects.
//! - `cap_auths_for`: parse the connection's MDS auth caps (`Connection::peer_caps`) into
//!   `MdsCapAuth` entries for the session OPEN; an empty list means no client-side
//!   restriction.

use denc::Message;
use monc::MdsMap;
use msgr::Connection;
use proto::{ClientMessage, MClientReply, MdsCapAuth};
use tracing::debug;

use crate::session::SessionMap;

/// Linux `ENOSYS`.
const ENOSYS: i32 = 38;

pub struct Server {
    pub fs_name: String,
    pub data_pool: i64,
    pub mdsmap_epoch: u32,
}

impl Server {
    pub fn new(fs_name: String, data_pool: i64, mdsmap_epoch: u32) -> Self {
        Server { fs_name, data_pool, mdsmap_epoch }
    }

    pub fn set_mdsmap(&mut self, map: &MdsMap) {
        self.mdsmap_epoch = map.epoch;
        if let Some(p) = map.data_pools.first() {
            self.data_pool = *p;
        }
    }

    /// The client's MDS auth caps as exported in the session OPEN. Stub: no restriction.
    pub fn cap_auths_for(&self, _conn: &Connection) -> Vec<MdsCapAuth> {
        Vec::new()
    }

    pub fn session_opened(&mut self, _sessions: &mut SessionMap, client: u64) {
        debug!(client, "session opened (engine not wired yet)");
    }

    pub fn session_closed(&mut self, _sessions: &mut SessionMap, client: u64) {
        debug!(client, "session closed (engine not wired yet)");
    }

    pub fn handle(&mut self, sessions: &mut SessionMap, client: u64, msg: ClientMessage) {
        match msg {
            ClientMessage::Request(req) => {
                let mut reply = MClientReply::for_request(&req, -ENOSYS);
                reply.mdsmap_epoch = self.mdsmap_epoch;
                if let Some(s) = sessions.get_mut(client) {
                    s.send_raw(reply.to_raw(s.link.features()));
                }
                debug!(client, tid = req.tid, op = ?req.head.op, "request answered ENOSYS (engine not wired yet)");
            }
            other => debug!(client, msg = ?std::mem::discriminant(&other), "client message ignored (engine not wired yet)"),
        }
    }

    pub fn tick(&mut self, _sessions: &mut SessionMap) {}
}
