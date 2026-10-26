//! The metadata server of one rank: translates client messages into engine calls and engine
//! effects into wire messages.
//!
//! The engine ([`engine::Engine`]) is created with the rank (`up:creating` on a fresh file
//! system, `up:replay` with an empty namespace until persistence exists). Every client message
//! of an open session is translated and answered here:
//!
//! - `session_opened` / `session_closed` / `session_resumed`: `Engine::open_session` /
//!   `close_session` / `session_resume`, then the returned effects are sent.
//! - `handle`: `ClientMessage::Request` → `engine::Request` → `Engine::handle_request`;
//!   `Caps` → `Engine::handle_cap_update`; `CapRelease` → `Engine::handle_cap_release`;
//!   `Lease` and `Metrics` are ignored (P0 issues null leases; metrics are tolerated);
//!   the legacy Reconnect/Reclaim handlers below are test-only. Production uses
//!   checked collection and durable RESET reclaim in `runtime`.
//! - Effects: `Reply` → `MClientReply` (engine `InodeStat`/`DirFragInfo`/`LeaseStat` → `proto`
//!   types; snap trace in `SnapFormat::for_client` of the session's features), `Caps` →
//!   `MClientCaps`, `RealmSplit` → `MClientSnap` (before the reply it belongs to), `Recall` →
//!   `RECALL_STATE`; `TruncateData`, `PurgeInode`, `RecoverFileSize` run as RADOS work through
//!   the Objecter ([`crate::rados`]) once the daemon provides it (`set_objecter`);
//!   `EvictClient` hands the client to the daemon for the monitor blocklist, then the kill.
//!   Every message goes through `Session::send`, in the order the engine returned the effects.
//! - Production ACTIVE timers use `runtime::d6`. Autoclose measures last valid
//!   activity, not STALE entry. The legacy timer helper below is test-only.
//!
//! Reply validation retains bounded request context until completion. A violation stops
//! service before any invalid reply leaves the session funnel.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use denc::Bytes;
#[cfg(test)]
use engine::MemJournal;
use engine::{
    CapReleaseItem, CapUpdate, Effect, Engine, EngineConfig, Journal, LayoutOverride, Op,
    ReplyExtra, Request, RequestRelease,
};
use monc::MdsMap;
use msgr::Connection;
use proto::{
    CapInodeFields, CapItem, CapOp, ClientMessage, DirStat, FeatureBitset,
    FileLayout as WireLayout, FilePath as WireFilePath, InodeStat as WireInodeStat,
    LeaseStat as WireLeaseStat, MClientCaps, MClientReclaimReply, MClientReply, MClientSession,
    MdsCapAuth, MdsOp, OpencResponse, ReaddirEntry, ReaddirReply as WireReaddirReply,
    ReplyCap as WireReplyCap, ReplyExtra as WireReplyExtra, SessionOp, SnapFormat,
    SnapRealmInfo as WireSnapRealmInfo, SnapTrace, Trace, TraceDentry, XattrMap as WireXattrMap,
    decode_xattr_blob, xattrs_to_blob,
};
#[cfg(test)]
use tracing::info;
use tracing::{debug, warn};

use osdc::objecter::Objecter;

use crate::fencing::Eviction;
use crate::journal::DaemonJournal;
use crate::session::{SessionMap, SessionState};
mod runtime;
pub(crate) use runtime::RecoveryFence;

pub(crate) trait WireDispatch {
    fn dispatch_late_reconnect(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn crate::session::ClientLink>,
    ) {
        sessions.recovery_terminal(client, link, MClientSession::close());
    }
    fn collecting_recovery(&self) -> bool {
        false
    }
    #[allow(clippy::too_many_arguments)] // Preserve the authenticated input and its original receive clock.
    fn dispatch_recovery(
        &mut self,
        _sessions: &mut SessionMap,
        _client: u64,
        _link: Box<dyn crate::session::ClientLink>,
        _msg: ClientMessage,
        _auths: Vec<MdsCapAuth>,
        _bytes: usize,
        _received: Instant,
    ) {
    }
    fn attached_link(
        &self,
        sessions: &SessionMap,
        client: u64,
        link: &dyn crate::session::ClientLink,
    ) -> bool;
    fn dispatch_session(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn crate::session::ClientLink>,
        msg: MClientSession,
        auths: Vec<MdsCapAuth>,
        received: Instant,
    );
    fn dispatch_message(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        msg: ClientMessage,
        received: Instant,
    );
}
impl WireDispatch for Server<DaemonJournal> {
    fn dispatch_late_reconnect(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn crate::session::ClientLink>,
    ) {
        self.late_reconnect(sessions, client, link);
    }
    fn collecting_recovery(&self) -> bool {
        self.recovery_pending()
    }
    fn dispatch_recovery(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn crate::session::ClientLink>,
        msg: ClientMessage,
        auths: Vec<MdsCapAuth>,
        bytes: usize,
        received: Instant,
    ) {
        self.recovery_input(sessions, client, link, msg, auths, bytes, received);
    }
    fn attached_link(
        &self,
        sessions: &SessionMap,
        client: u64,
        link: &dyn crate::session::ClientLink,
    ) -> bool {
        self.is_attached(sessions, client, link)
    }
    fn dispatch_session(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn crate::session::ClientLink>,
        msg: MClientSession,
        auths: Vec<MdsCapAuth>,
        received: Instant,
    ) {
        self.handle_session_at(sessions, client, link, msg, auths, received);
    }
    fn dispatch_message(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        msg: ClientMessage,
        received: Instant,
    ) {
        self.handle_at(sessions, client, msg, received);
    }
}
#[cfg(test)]
impl WireDispatch for Server<MemJournal> {
    fn attached_link(
        &self,
        sessions: &SessionMap,
        client: u64,
        link: &dyn crate::session::ClientLink,
    ) -> bool {
        sessions
            .get(client)
            .is_some_and(|s| s.addr == link.peer_addr())
    }
    fn dispatch_session(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn crate::session::ClientLink>,
        msg: MClientSession,
        auths: Vec<MdsCapAuth>,
        _received: Instant,
    ) {
        use crate::session::SessionOutcome;
        match sessions.handle(client, link, msg, auths) {
            SessionOutcome::Opened => self.session_opened(sessions, client),
            SessionOutcome::Closed => self.session_closed(sessions, client),
            SessionOutcome::Renewed => self.session_resumed(sessions, client),
            SessionOutcome::FlushMdlog => self.flush_mdlog(sessions),
            SessionOutcome::ProtocolError => self.protocol_error(client),
            SessionOutcome::Done => {}
        }
    }
    fn dispatch_message(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        msg: ClientMessage,
        _received: Instant,
    ) {
        self.handle(sessions, client, msg);
    }
}

/// How often `Engine::recall_hints` runs (C++ uses the mds tick, 1 s; every few is enough).
#[cfg(test)]
const RECALL_HINT_INTERVAL: Duration = Duration::from_secs(10);

/// Bound retained validation contexts, including requests parked in the engine.
const MAX_PENDING_REQUESTS: usize = 4096;

/// Wall-clock now, as the engine wants time stamps (the engine has no clock of its own).
fn now_ts() -> engine::Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| engine::Timestamp::new(d.as_secs() as u32, d.subsec_nanos()))
        .unwrap_or(engine::Timestamp::ZERO)
}

fn ts(t: engine::Timestamp) -> denc::UTime {
    denc::UTime {
        sec: t.sec,
        nsec: t.nsec,
    }
}

fn utime(t: denc::UTime) -> engine::Timestamp {
    engine::Timestamp::new(t.sec, t.nsec)
}

fn file_path(p: &WireFilePath) -> engine::FilePath {
    engine::FilePath {
        ino: p.ino,
        names: p.components().iter().map(|c| c.to_vec()).collect(),
    }
}

fn layout(l: &engine::FileLayout) -> WireLayout {
    WireLayout {
        stripe_unit: l.stripe_unit,
        stripe_count: l.stripe_count,
        object_size: l.object_size,
        pool_id: l.pool_id,
        pool_ns: l.pool_ns.clone(),
    }
}

fn xattrs(x: &engine::XattrMap) -> WireXattrMap {
    x.iter()
        .map(|(k, v)| (Bytes::from(k.clone()), Bytes::from(v.clone())))
        .collect()
}

fn xattrs_from_wire(x: &WireXattrMap) -> engine::XattrMap {
    x.iter().map(|(k, v)| (k.to_vec(), v.to_vec())).collect()
}

fn lease(l: &engine::LeaseStat) -> WireLeaseStat {
    WireLeaseStat {
        mask: l.mask,
        duration_ms: l.duration_ms,
        seq: l.seq,
        alternate_name: Bytes::from(l.alternate_name.clone()),
    }
}

fn snap_realm(r: &engine::SnapRealmInfo) -> WireSnapRealmInfo {
    WireSnapRealmInfo {
        ino: r.ino,
        created: r.created,
        parent: r.parent,
        parent_since: r.parent_since,
        seq: r.seq,
        my_snaps: Vec::new(),
        prior_parent_snaps: Vec::new(),
        last_modified: ts(r.last_modified),
        change_attr: r.change_attr,
        flags: if r.subvolume {
            proto::snap::SNAPREALM_SNAPDIR_VISIBILITY
        } else {
            0
        },
    }
}

fn dirstat(d: &engine::DirFragInfo) -> DirStat {
    DirStat {
        frag: proto::Frag(d.frag),
        auth: d.auth,
        dist: d.dist.iter().copied().collect(),
    }
}

fn inode_stat(s: &engine::InodeStat) -> WireInodeStat {
    WireInodeStat {
        ino: s.ino,
        snapid: s.snapid,
        rdev: s.rdev,
        version: s.version,
        xattr_version: s.xattr_version,
        cap: WireReplyCap {
            caps: s.cap.caps,
            wanted: s.cap.wanted,
            cap_id: s.cap.cap_id,
            seq: s.cap.seq,
            mseq: s.cap.mseq,
            realm: s.cap.realm,
            flags: s.cap.flags,
        },
        layout: layout(&s.layout),
        ctime: ts(s.ctime),
        mtime: ts(s.mtime),
        atime: ts(s.atime),
        time_warp_seq: s.time_warp_seq,
        size: s.size,
        max_size: s.max_size,
        truncate_size: s.truncate_size,
        truncate_seq: s.truncate_seq,
        mode: s.mode,
        uid: s.uid,
        gid: s.gid,
        nlink: s.nlink,
        nfiles: s.dirstat.nfiles,
        nsubdirs: s.dirstat.nsubdirs,
        rbytes: s.rstat.rbytes,
        rfiles: s.rstat.rfiles,
        rsubdirs: s.rstat.rsubdirs,
        rctime: ts(s.rstat.rctime),
        fragtree: Default::default(),
        symlink: Bytes::from(s.symlink.clone()),
        dir_layout: proto::DirLayout {
            dir_hash: s.dir_hash,
        },
        xattrs: s.xattrs.as_ref().map(xattrs),
        inline_version: s.inline_version,
        inline_data: Bytes::new(),
        quota: proto::QuotaInfo {
            max_bytes: s.quota.max_bytes,
            max_files: s.quota.max_files,
        },
        btime: ts(s.btime),
        change_attr: s.change_attr,
        dir_pin: s.export_pin,
        rsnaps: s.rstat.rsnaps,
        fscrypt_auth: Bytes::from(s.fscrypt_auth.clone()),
        fscrypt_file: Bytes::from(s.fscrypt_file.clone()),
        subvolume_id: s.subvolume_id,
        ..Default::default()
    }
}

fn readdir_extra(r: &engine::ReaddirReply) -> WireReplyExtra {
    WireReplyExtra::Readdir(WireReaddirReply {
        dirstat: dirstat(&r.dirfrag),
        flags: r.flags,
        entries: r
            .entries
            .iter()
            .map(|e| ReaddirEntry {
                name: Bytes::from(e.name.clone()),
                lease: lease(&e.lease),
                inode: inode_stat(&e.inode),
            })
            .collect(),
    })
}

fn reply_extra(e: &ReplyExtra) -> WireReplyExtra {
    match e {
        ReplyExtra::None => WireReplyExtra::None,
        ReplyExtra::Create { ino, delegated } => WireReplyExtra::Create(OpencResponse {
            created_ino: *ino,
            delegated_inos: delegated.iter().map(|r| (r.start, r.len)).collect(),
        }),
        ReplyExtra::Readdir(r) => readdir_extra(r),
        ReplyExtra::FileLock(l) => WireReplyExtra::FileLock(proto::FileLock {
            start: l.start,
            length: l.length,
            client: l.client,
            owner: l.owner,
            pid: l.pid,
            lock_type: l.ltype,
        }),
        ReplyExtra::Vxattr(v) => WireReplyExtra::Vxattr(Bytes::from(v.clone())),
    }
}

/// The snap-trace encoding format of a session's client.
fn snap_format(features: &FeatureBitset) -> SnapFormat {
    SnapFormat::for_client(features)
}

fn file_lock_args(a: &proto::request::FilelockArgs) -> engine::FileLockArgs {
    engine::FileLockArgs {
        rule: a.rule,
        ltype: a.lock_type,
        owner: a.owner,
        pid: a.pid,
        start: a.start,
        length: a.length,
        wait: a.wait != 0,
    }
}

/// `MClientRequest` → `engine::Request`. `now` fills a zero client stamp (the engine has no
/// clock). `conn_features` are the connection's msgr features: a CREATE's `args.open.pool` is
/// honoured only for peers with `CREATEPOOLID` (`Server::prepare_new_inode`); the kernel never
/// advertises the bit and leaves the field zero. Initial xattrs of create ops come from the
/// request's data payload. Reject invalid payloads before calling the engine.
fn request_from_proto(
    req: &proto::MClientRequest,
    now: engine::Timestamp,
    conn_features: u64,
) -> denc::Result<Request> {
    let h = &req.head;
    let a = &h.args;
    let getattr_mask = a.getattr_mask();
    let lookupino = a.lookupino();
    let open = a.open();
    let xattrs = |req: &proto::MClientRequest| -> denc::Result<engine::XattrMap> {
        req.initial_xattrs().map(|x| xattrs_from_wire(&x))
    };
    let last_name = || (!req.path2.path.is_empty()).then(|| req.path2.path.to_vec());
    let op = match h.op {
        MdsOp::LOOKUP => Op::Lookup {
            path: file_path(&req.path),
            mask: getattr_mask,
        },
        MdsOp::GETATTR => Op::Getattr {
            path: file_path(&req.path),
            mask: getattr_mask,
        },
        MdsOp::LOOKUPHASH | MdsOp::LOOKUPINO => Op::LookupIno {
            ino: req.path.ino,
            mask: lookupino.mask,
        },
        MdsOp::LOOKUPPARENT => Op::LookupParent { ino: req.path.ino },
        MdsOp::LOOKUPNAME => Op::LookupName {
            ino: req.path.ino,
            parent: Some(req.path2.ino).filter(|p| *p != 0),
        },
        MdsOp::GETVXATTR => Op::Getvxattr {
            path: file_path(&req.path),
            name: req.path2.path.to_vec(),
        },
        MdsOp::SETXATTR => {
            let sx = a.setxattr();
            Op::Setxattr {
                path: file_path(&req.path),
                name: req.path2.path.to_vec(),
                value: req.data.to_vec(),
                flags: sx.flags,
            }
        }
        MdsOp::RMXATTR => Op::Rmxattr {
            path: file_path(&req.path),
            name: req.path2.path.to_vec(),
        },
        MdsOp::SETATTR => {
            let sa = a.setattr();
            Op::Setattr {
                path: file_path(&req.path),
                args: engine::SetattrArgs {
                    mask: sa.mask,
                    mode: sa.mode,
                    uid: sa.uid,
                    gid: sa.gid,
                    mtime: utime(sa.mtime),
                    atime: utime(sa.atime),
                    btime: utime(sa.btime),
                    size: sa.size,
                    old_size: sa.old_size,
                },
            }
        }
        MdsOp::SETFILELOCK => Op::SetFileLock {
            path: file_path(&req.path),
            lock: file_lock_args(&a.filelock()),
        },
        MdsOp::GETFILELOCK => Op::GetFileLock {
            path: file_path(&req.path),
            lock: file_lock_args(&a.filelock()),
        },
        MdsOp::MKNOD => {
            let m = a.mknod();
            Op::Mknod {
                path: file_path(&req.path),
                mode: m.mode,
                rdev: m.rdev,
                xattrs: xattrs(req)?,
                ino: h.ino,
            }
        }
        MdsOp::LINK => Op::Link {
            path: file_path(&req.path),
            target: file_path(&req.path2),
        },
        MdsOp::UNLINK => Op::Unlink {
            path: file_path(&req.path),
        },
        MdsOp::RENAME => Op::Rename {
            dst: file_path(&req.path),
            src: file_path(&req.path2),
        },
        MdsOp::MKDIR => Op::Mkdir {
            path: file_path(&req.path),
            mode: a.mkdir_mode(),
            xattrs: xattrs(req)?,
            ino: h.ino,
        },
        MdsOp::RMDIR => Op::Rmdir {
            path: file_path(&req.path),
        },
        MdsOp::SYMLINK => Op::Symlink {
            path: file_path(&req.path),
            target: req.path2.path.to_vec(),
            xattrs: xattrs(req)?,
            ino: h.ino,
        },
        MdsOp::CREATE => Op::Create {
            path: file_path(&req.path),
            flags: open.flags,
            mode: open.mode,
            layout: LayoutOverride {
                stripe_unit: open.stripe_unit,
                stripe_count: open.stripe_count,
                object_size: open.object_size,
                pool: if conn_features & denc::features::CREATEPOOLID != 0 {
                    open.pool as i64
                } else {
                    -1
                },
            },
            xattrs: xattrs(req)?,
            ino: h.ino,
        },
        MdsOp::OPEN => Op::Open {
            path: file_path(&req.path),
            flags: open.flags,
            mode: open.mode,
            mask: open.mask,
            old_size: open.old_size,
        },
        MdsOp::READDIR => {
            let rd = a.readdir();
            Op::Readdir {
                path: file_path(&req.path),
                frag: rd.frag,
                max_entries: rd.max_entries,
                max_bytes: rd.max_bytes,
                last_name: last_name(),
                offset_hash: rd.offset_hash,
                flags: u32::from(rd.flags),
            }
        }
        MdsOp::LOOKUPSNAP => Op::LookupSnap {
            path: file_path(&req.path),
        },
        MdsOp::LSSNAP => Op::Lssnap {
            path: file_path(&req.path),
        },
        MdsOp::MKSNAP => Op::Mksnap {
            path: file_path(&req.path),
        },
        other => Op::Other { op: other.0 },
    };
    Ok(Request {
        tid: req.tid,
        op,
        caller_uid: h.caller_uid,
        caller_gid: h.caller_gid,
        owner_uid: if h.owner_uid == u32::MAX {
            h.caller_uid
        } else {
            h.owner_uid
        },
        owner_gid: if h.owner_gid == u32::MAX {
            h.caller_gid
        } else {
            h.owner_gid
        },
        stamp: if req.stamp == denc::UTime::default() {
            now
        } else {
            utime(req.stamp)
        },
        flags: h.flags,
        retry_attempt: h.num_retry,
        oldest_client_tid: h.oldest_client_tid,
        releases: req
            .releases
            .iter()
            .map(|r| RequestRelease {
                ino: r.ino,
                cap_id: r.cap_id,
                caps: r.caps,
                wanted: r.wanted,
                seq: r.seq,
                issue_seq: r.issue_seq,
                mseq: r.mseq,
                dname: (!r.dname.is_empty()).then(|| r.dname.to_vec()),
            })
            .collect(),
    })
}

/// `MClientCaps` → `engine::CapUpdate`. Decode all xattrs before applying any field.
fn cap_update_from_proto(m: &MClientCaps) -> denc::Result<CapUpdate> {
    let xattrs = if m.xattrbl.is_empty() {
        None
    } else {
        Some((
            m.xattr_version,
            xattrs_from_wire(&decode_xattr_blob(m.xattrbl.clone())?),
        ))
    };
    Ok(CapUpdate {
        op: m.op.0,
        ino: m.ino,
        cap_id: m.cap_id,
        seq: m.seq,
        issue_seq: m.issue_seq,
        mseq: m.migrate_seq,
        caps: m.caps,
        wanted: m.wanted,
        dirty: m.dirty,
        flush_tid: m.tid,
        oldest_flush_tid: m.oldest_flush_tid,
        flags: m.flags,
        size: m.size,
        max_size: m.max_size,
        mtime: utime(m.mtime),
        atime: utime(m.atime),
        ctime: utime(m.ctime),
        btime: utime(m.btime),
        time_warp_seq: m.time_warp_seq,
        change_attr: m.change_attr,
        uid: m.uid,
        gid: m.gid,
        mode: m.mode,
        xattrs,
    })
}

fn cap_item(i: &CapItem) -> CapReleaseItem {
    CapReleaseItem {
        ino: i.ino,
        cap_id: i.cap_id,
        mseq: i.migrate_seq,
        issue_seq: i.issue_seq,
    }
}

pub struct Server<J: Journal = DaemonJournal> {
    required_client_features: Bytes,
    pub fs_name: String,
    pub data_pool: i64,
    pub mdsmap_epoch: u32,
    engine: Engine<J>,
    runtime: Option<runtime::Runtime>,
    /// Only fields used by check_invariants_for; no request data or release payloads.
    pending_requests: HashMap<(u64, u64), proto::MClientRequest>,
    fatal_error: Option<String>,
    osd_epoch_barrier: u32,
    mdlog_flush_target: Option<u64>,
    work_paused: bool,
    held_work: VecDeque<(Effect, usize)>,
    held_work_bytes: usize,
    #[cfg(test)]
    timer_fault_effects: Vec<Effect>,
    /// `MDSMap::session_timeout`: renewals older than this make the session stale.
    session_timeout: Duration,
    /// `MDSMap::session_autoclose`: a stale session this old is killed.
    session_autoclose: Duration,
    /// When a session became stale (engine client id → time).
    #[cfg(test)]
    stale_since: HashMap<u64, Instant>,
    #[cfg(test)]
    last_recall_hints: Instant,
    /// The OSD client: runs the RADOS work of `PurgeInode`, `TruncateData` and
    /// `RecoverFileSize` (set by the daemon once the Objecter exists).
    objecter: Option<Objecter>,
    /// Where `EvictClient` goes: the daemon blocklists the client through the
    /// monitor, then kills the session (the reply channel).
    evict_tx: Option<tokio::sync::mpsc::Sender<Eviction>>,
}

#[cfg(test)]
impl Server<MemJournal> {
    pub fn new(fs_name: String, data_pool: i64, mdsmap_epoch: u32) -> Self {
        let config = EngineConfig {
            data_pools: if data_pool >= 0 {
                vec![data_pool]
            } else {
                Vec::new()
            },
            pool_names: BTreeMap::new(),
            mkfs_stamp: now_ts(),
            ..EngineConfig::default()
        };
        let engine = Engine::new(config, MemJournal::new());
        Self::with_engine(fs_name, data_pool, mdsmap_epoch, engine)
    }
}

impl<J: Journal> Server<J> {
    fn with_engine(fs_name: String, data_pool: i64, mdsmap_epoch: u32, engine: Engine<J>) -> Self {
        Server {
            required_client_features: Bytes::new(),
            fs_name,
            data_pool,
            mdsmap_epoch,
            engine,
            runtime: None,
            pending_requests: HashMap::new(),
            fatal_error: None,
            osd_epoch_barrier: 0,
            mdlog_flush_target: None,
            work_paused: false,
            held_work: VecDeque::new(),
            held_work_bytes: 0,
            #[cfg(test)]
            timer_fault_effects: Vec::new(),
            session_timeout: Duration::from_secs(60),
            session_autoclose: Duration::from_secs(300),
            #[cfg(test)]
            stale_since: HashMap::new(),
            #[cfg(test)]
            last_recall_hints: Instant::now(),
            objecter: None,
            evict_tx: None,
        }
    }

    /// A fail-stop reason. The daemon must leave service before processing another event.
    pub fn fatal_error(&self) -> Option<&str> {
        self.fatal_error
            .as_deref()
            .or_else(|| self.engine.is_failed().then_some("metadata engine failed"))
    }

    pub fn protocol_error(&mut self, client: u64) {
        self.fatal_error = Some(format!("invalid client protocol from client {client}"));
        if let Some(runtime) = &mut self.runtime {
            runtime.cancel_pending();
        }
    }
}

#[cfg(test)]
impl Server<MemJournal> {
    /// C++ Server::handle_client_session REQUEST_FLUSH_MDLOG calls MDLog::flush,
    /// with no acknowledgment. The current MemJournal records the request but provides
    /// no persistence. A storage journal must start its commit here and report completion.
    pub fn flush_mdlog(&mut self, sessions: &mut SessionMap) {
        if self.fatal_error.is_some() {
            return;
        }
        self.engine.journal_mut().flush();
        self.mdlog_flush_target = Some(
            self.mdlog_flush_target
                .unwrap_or(0)
                .max(self.engine.journal().last_seq()),
        );
        self.poll_commits(sessions);
    }
}

impl<J: Journal> Server<J> {
    /// Provide the OSD client for the RADOS-work effects.
    pub fn set_objecter(&mut self, objecter: Objecter) {
        self.objecter = Some(objecter);
    }

    /// Provide the eviction channel (the daemon blocklists, then kills).
    pub fn set_evict_channel(&mut self, tx: tokio::sync::mpsc::Sender<Eviction>) {
        self.evict_tx = Some(tx);
    }

    /// Monotonic client/Objecter barrier, including FLUSH_ACK traffic.
    pub fn raise_epoch_barrier(&mut self, epoch: u32) {
        self.osd_epoch_barrier = self.osd_epoch_barrier.max(epoch);
        if let Some(objecter) = &self.objecter {
            objecter.set_epoch_barrier(self.osd_epoch_barrier);
        }
    }

    pub fn epoch_barrier(&self) -> u32 {
        self.osd_epoch_barrier
    }

    /// Admission uses the applied map at dispatch time, including held OPENs.
    pub fn is_blocklisted(&self, addr: &denc::EntityAddr) -> bool {
        self.objecter
            .as_ref()
            .is_some_and(|objecter| objecter.osdmap().is_blocklisted(addr))
    }

    /// Compatibility test helper. Production sets separate runtime permissions.
    #[cfg(test)]
    pub fn pause_data_work(&mut self, paused: bool) {
        self.work_paused = paused;
        if let Some(runtime) = &mut self.runtime {
            runtime.pause_ordinary(paused);
        }
    }

    pub fn log_status(&self) {
        debug!(fs = %self.fs_name, journal_backend = "guarded WAL",
            submitted = self.engine.submitted_seq(), committed = self.engine.journal().committed_seq(),
            flush_target = ?self.mdlog_flush_target, request_contexts = self.pending_requests.len(),
            held_work = self.held_work.len(), held_work_bytes = self.held_work_bytes,
            epoch_barrier = self.osd_epoch_barrier, failed = self.fatal_error().is_some(), "metadata status");
    }

    pub fn resume_data_work(&mut self, sessions: &mut SessionMap, budget: usize) {
        if self.work_paused || self.fatal_error().is_some() {
            return;
        }
        for _ in 0..budget {
            let Some((effect, bytes)) = self.held_work.pop_front() else {
                break;
            };
            self.held_work_bytes -= bytes;
            self.send_effects(sessions, vec![effect]);
            if self.fatal_error().is_some() {
                break;
            }
        }
    }
}

#[cfg(test)]
impl Server<MemJournal> {
    /// Apply a verified fence only to the transport generation that requested it.
    /// The daemon separately validates membership, operation id and current map evidence.
    pub fn finish_eviction(
        &mut self,
        sessions: &mut SessionMap,
        request: Eviction,
        epoch: u32,
    ) -> bool {
        if self.fatal_error.is_some()
            || sessions.get(request.client).is_none_or(|s| {
                s.generation != request.session_generation
                    || s.addr != request.addr
                    || s.state != SessionState::Fencing
            })
        {
            return false;
        }
        self.raise_epoch_barrier(epoch);
        self.kill_session(sessions, request.client);
        true
    }
}

impl<J: Journal> Server<J> {
    /// Stop this session's admission without releasing its caps, locks or ranges.
    pub fn request_eviction(&mut self, sessions: &mut SessionMap, client: u64) {
        if self.fatal_error.is_some() {
            return;
        }
        if self
            .runtime
            .as_mut()
            .is_some_and(|r| r.freeze_recovery_client(client))
        {
            if let Some(s) = sessions.get_mut(client) {
                s.state = SessionState::Fencing;
            }
            return;
        }
        let Some(s) = sessions.get_mut(client) else {
            return;
        };
        if s.state == SessionState::Fencing {
            return;
        }
        s.state = SessionState::Fencing;
        let request = Eviction {
            client,
            session_generation: s.generation,
            addr: s.addr,
        };
        if self
            .evict_tx
            .as_ref()
            .is_none_or(|tx| tx.try_send(request).is_err())
        {
            // Never fall back to releasing rights without a fence.
            self.fatal_error = Some(format!("cannot queue client {client} fencing"));
        }
    }

    pub fn set_mdsmap(&mut self, map: &MdsMap) {
        self.required_client_features = map.required_client_features.clone();
        self.mdsmap_epoch = map.epoch;
        if let Some(p) = map.data_pools.first() {
            self.data_pool = *p;
        }
        self.session_timeout = Duration::from_secs(u64::from(map.session_timeout.max(1)));
        self.session_autoclose = Duration::from_secs(u64::from(map.session_autoclose.max(1)));
        self.engine
            .set_pools(map.data_pools.clone(), BTreeMap::new());
    }

    /// The client's MDS auth caps as exported in the session OPEN. P0 applies no client-side
    /// restriction: an empty list (the C++ default for `allow` caps); `client.admin`'s
    /// `allow_all` blob and per-path cap strings are both reported as unrestricted, and a
    /// non-trivial cap string is logged.
    pub fn cap_auths_for(&self, conn: &Connection) -> Vec<MdsCapAuth> {
        if let Some(caps) = conn.peer_caps()
            && !caps.allow_all
            && let Some(s) = caps.cap_string()
            && !s.is_empty()
        {
            debug!(caps = %s, "mds auth caps not parsed (no restriction applied)");
        }
        Vec::new()
    }
}

#[cfg(test)]
impl Server<MemJournal> {
    pub fn session_opened(&mut self, sessions: &mut SessionMap, client: u64) {
        if self.fatal_error().is_some() {
            return;
        }
        self.stale_since.remove(&client);
        let effects = self.engine.open_session(client);
        self.send_effects(sessions, effects);
        info!(client, "session opened");
    }

    pub fn session_closed(&mut self, sessions: &mut SessionMap, client: u64) {
        if self.fatal_error().is_some() {
            return;
        }
        self.pending_requests.retain(|(c, _), _| *c != client);
        self.stale_since.remove(&client);
        let effects = self.engine.close_session(client, false);
        self.send_effects(sessions, effects);
        info!(client, "session closed");
    }

    /// The client renewed (or recovered a stale session): re-issue its caps if it was stale.
    pub fn session_resumed(&mut self, sessions: &mut SessionMap, client: u64) {
        if self.fatal_error().is_some() {
            return;
        }
        let effects = self.engine.session_resume(client);
        self.send_effects(sessions, effects);
    }

    pub fn handle(&mut self, sessions: &mut SessionMap, client: u64, msg: ClientMessage) {
        if self.fatal_error().is_some() {
            return;
        }
        match msg {
            ClientMessage::Request(req) => {
                let tid = req.tid;
                let op = req.op();
                let conn_features = sessions.get(client).map(|s| s.link.features()).unwrap_or(0);
                // Ignore in-flight retransmissions. Keep the first validation context.
                // A completed retry enters normally after its previous context is removed.
                if self.pending_requests.contains_key(&(client, tid)) {
                    return;
                }
                let ereq = match request_from_proto(&req, now_ts(), conn_features) {
                    Ok(r) => r,
                    Err(e) => {
                        warn!(client, tid, "invalid initial xattrs: {e}");
                        let mut reply = MClientReply::new(tid, op, -22);
                        reply.mdsmap_epoch = self.mdsmap_epoch;
                        if reply.check_invariants_for(&req).is_empty() {
                            if let Some(s) = sessions.get_mut(client) {
                                s.send(&reply);
                            }
                        } else {
                            self.protocol_error(client);
                        }
                        return;
                    }
                };
                if self.pending_requests.len() >= MAX_PENDING_REQUESTS {
                    self.fatal_error = Some("request validation capacity exhausted".into());
                    return;
                }
                let mut context = proto::MClientRequest::new(tid, op);
                context.head = req.head.clone();
                // Only component count and parent inode are used by the validator.
                context.path = WireFilePath::new(
                    req.path.ino,
                    match req.path.components().as_slice() {
                        [] => "",
                        [[]] => "/",
                        [_] => "x",
                        _ => "x/y",
                    },
                );
                self.pending_requests.insert((client, tid), context);
                let effects = self.engine.handle_request(client, ereq);
                debug!(client, tid, op = ?op, effects = effects.len(), "request handled");
                self.send_effects(sessions, effects);
            }
            ClientMessage::Caps(m) => {
                // C++ Locker::handle_client_caps accepts only client-originated ops.
                // Do not let an unknown or MDS-only opcode reach dirty metadata handling.
                if !matches!(
                    m.op,
                    CapOp::UPDATE
                        | CapOp::FLUSH
                        | CapOp::DROP
                        | CapOp::RELEASE
                        | CapOp::RENEW
                        | CapOp::FLUSHSNAP
                ) {
                    self.protocol_error(client);
                    return;
                }
                debug!(client, ino = m.ino, op = m.op.name(), "cap update");
                let update = match cap_update_from_proto(&m) {
                    Ok(update) => update,
                    Err(e) => {
                        warn!(client, ino = m.ino, "invalid cap xattrs: {e}");
                        self.protocol_error(client);
                        return;
                    }
                };
                self.raise_epoch_barrier(m.osd_epoch_barrier);
                let effects = self.engine.handle_cap_update(client, update);
                self.send_effects(sessions, effects);
            }
            ClientMessage::CapRelease(m) => {
                self.raise_epoch_barrier(m.osd_epoch_barrier);
                let items: Vec<CapReleaseItem> = m.caps.iter().map(cap_item).collect();
                let effects = self.engine.handle_cap_release(client, &items);
                self.send_effects(sessions, effects);
            }
            ClientMessage::Lease(_) | ClientMessage::Metrics(_) => {
                // P0 issues null leases; metrics are tolerated without an answer.
            }
            ClientMessage::Reconnect(m) => {
                // No persistence: there is no state to reconnect. CLOSE tells the client its
                // caps (and the mount) are gone (workplan stage 3 adds the real path).
                warn!(
                    client,
                    caps = m.caps.len(),
                    "reconnect denied (no persistence): CLOSE"
                );
                if let Some(s) = sessions.get_mut(client) {
                    s.send(&MClientSession::close());
                }
            }
            ClientMessage::Reclaim(m) => {
                // Unknown uuid: result 0, no address (FSAL_CEPH reads that as ENOENT).
                debug!(client, uuid = %m.uuid, "reclaim: unknown uuid");
                if let Some(s) = sessions.get_mut(client) {
                    s.send(&MClientReclaimReply::default());
                }
            }
            ClientMessage::Session(_) => {
                // Routed through SessionMap::handle before it reaches the server.
            }
        }
    }
}

impl<J: Journal> Server<J> {
    /// Send the effects of an engine call, in order, through the sessions' funnels.
    fn send_effects(&mut self, sessions: &mut SessionMap, effects: Vec<Effect>) {
        if self.engine.is_failed() {
            self.fatal_error = Some("metadata engine failed".into());
            return;
        }
        for effect in effects {
            if sessions.output_failed() {
                self.fatal_error = Some("session output unavailable or over budget".into());
                break;
            }
            if self.fatal_error.is_some() {
                break;
            }
            let work_bytes = match &effect {
                Effect::TruncateData(t) => Some(t.layout.pool_ns.len() + 128),
                Effect::PurgeInode(p) => Some(p.layout.pool_ns.len() + 128),
                Effect::RecoverFileSize { layout, .. } => Some(layout.pool_ns.len() + 128),
                _ => None,
            };
            if self.work_paused
                && let Some(bytes) = work_bytes
            {
                if self.held_work.len() >= MAX_PENDING_REQUESTS
                    || bytes > (16usize << 20).saturating_sub(self.held_work_bytes)
                {
                    self.fatal_error = Some("paused data work capacity exhausted".into());
                    break;
                }
                self.held_work_bytes += bytes;
                self.held_work.push_back((effect, bytes));
                continue;
            }
            match effect {
                Effect::Lifecycle(intent) => {
                    if self.runtime.is_none() {
                        self.fatal_error = Some(format!(
                            "lifecycle job {} without durable runtime",
                            intent.id
                        ));
                        break;
                    }
                }
                Effect::Reply(r) => {
                    let context = self.pending_requests.remove(&(r.client, r.tid));
                    let Some(s) = sessions.get_mut(r.client) else {
                        continue;
                    };
                    let mut reply = MClientReply::new(r.tid, MdsOp(r.op), r.result);
                    reply.mdsmap_epoch = self.mdsmap_epoch;
                    reply.trace = Trace {
                        dentry: r.dentry.as_ref().map(|d| TraceDentry {
                            dir: inode_stat(&d.dir),
                            dirstat: dirstat(&d.dirfrag),
                            name: Bytes::from(d.name.clone()),
                            lease: lease(&d.lease),
                        }),
                        target: r.target.as_ref().map(inode_stat),
                    };
                    if r.snap_realm.is_some() {
                        let format = snap_format(&s.features);
                        reply.set_snap_trace(
                            &SnapTrace::new(r.snap_trace.iter().map(snap_realm).collect()),
                            format,
                        );
                    }
                    reply.extra = reply_extra(&r.extra);
                    if r.op == MdsOp::CREATE.0 && r.created_ino.is_some() {
                        // OpencResponse also reaches retries answered from the completed table.
                        if let WireReplyExtra::None = reply.extra {
                            reply.extra = WireReplyExtra::Create(OpencResponse::default());
                        }
                        if let WireReplyExtra::Create(c) = &mut reply.extra {
                            c.created_ino = r.created_ino.unwrap_or(0);
                        }
                    }
                    let Some(context) = context else {
                        self.fatal_error = Some(format!(
                            "reply without request context: client {} tid {}",
                            r.client, r.tid
                        ));
                        break;
                    };
                    let violations = reply.check_invariants_for(&context);
                    if !violations.is_empty() {
                        warn!(
                            client = r.client,
                            tid = r.tid,
                            ?violations,
                            "reply invariant violations"
                        );
                        self.fatal_error = Some(format!(
                            "invalid reply: client {} tid {}: {violations:?}",
                            r.client, r.tid
                        ));
                        break;
                    }
                    s.send(&reply);
                }
                Effect::Caps(c) => {
                    let Some(s) = sessions.get_mut(c.client) else {
                        continue;
                    };
                    let mut m = MClientCaps::new(
                        CapOp(c.op),
                        c.ino,
                        c.realm,
                        c.cap_id,
                        c.seq,
                        c.caps,
                        c.wanted,
                        c.dirty,
                        c.mseq,
                        c.issue_seq,
                        self.osd_epoch_barrier,
                    );
                    m.tid = c.flush_tid;
                    m.oldest_flush_tid = c.oldest_flush_tid;
                    if let Some(f) = &c.inode {
                        m.set_inode(&CapInodeFields {
                            uid: f.uid,
                            gid: f.gid,
                            mode: f.mode,
                            nlink: f.nlink,
                            xattrs: f
                                .xattrs
                                .as_ref()
                                .map(|(v, x)| (*v, xattrs_to_blob(&xattrs(x)))),
                            size: f.size,
                            max_size: f.max_size,
                            truncate_size: f.truncate_size,
                            truncate_seq: f.truncate_seq,
                            mtime: ts(f.mtime),
                            atime: ts(f.atime),
                            ctime: ts(f.ctime),
                            btime: ts(f.btime),
                            layout: layout(&f.layout),
                            time_warp_seq: f.time_warp_seq,
                            change_attr: f.change_attr,
                            nfiles: f.nfiles,
                            nsubdirs: f.nsubdirs,
                            inline_version: f.inline_version,
                            fscrypt_auth: Bytes::new(),
                            fscrypt_file: Bytes::new(),
                        });
                    }
                    s.send(&m);
                }
                Effect::RealmSplit(split) => {
                    let Some(s) = sessions.get_mut(split.client) else {
                        continue;
                    };
                    let format = snap_format(&s.features);
                    let trace = SnapTrace::new(split.snap_trace.iter().map(snap_realm).collect())
                        .encode(format);
                    let msg = proto::MClientSnap::split(
                        split.realm,
                        split.split_inos,
                        split.split_realms,
                        trace,
                    );
                    s.send(&msg);
                }
                Effect::Recall { client, max_caps } => {
                    if let Some(s) = sessions.get_mut(client) {
                        s.send(&MClientSession::recall_state(
                            max_caps.min(u32::MAX as u64) as u32
                        ));
                    }
                }
                Effect::EvictClient { client } => {
                    warn!(client, "evicting client (and blocklisting)");
                    self.request_eviction(sessions, client);
                }
                Effect::TruncateData(_)
                | Effect::PurgeInode(_)
                | Effect::RecoverFileSize { .. } => {
                    self.fatal_error =
                        Some("compatibility data effect outside durable lifecycle protocol".into());
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
impl Server<MemJournal> {
    /// Kill a session without `CLOSE` (`Server::kill_session`: engine close
    /// with the kill flag, then the connection is dropped).
    fn kill_session(&mut self, sessions: &mut SessionMap, client: u64) {
        self.pending_requests.retain(|(c, _), _| *c != client);
        self.stale_since.remove(&client);
        let effects = self.engine.close_session(client, true);
        sessions.remove(client);
        self.send_effects(sessions, effects);
    }

    /// Once a second (the daemon tick): journal completions, retries, recall hints and the
    /// session staleness/autoclose timers.
    pub fn poll_commits(&mut self, sessions: &mut SessionMap) {
        if self.fatal_error().is_some() {
            return;
        }
        let committed = self.engine.journal_committed();
        self.send_effects(sessions, committed);
        if self
            .mdlog_flush_target
            .is_some_and(|seq| self.engine.journal().committed_seq() >= seq)
        {
            self.mdlog_flush_target = None;
        }
    }

    pub fn tick(&mut self, sessions: &mut SessionMap) {
        if self.fatal_error().is_some() {
            return;
        }
        self.poll_commits(sessions);
        if self.fatal_error.is_some() {
            return;
        }
        let tick = self.engine.tick();
        self.send_effects(sessions, tick);
        if self.fatal_error.is_some() {
            return;
        }
        if self.last_recall_hints.elapsed() >= RECALL_HINT_INTERVAL {
            self.last_recall_hints = Instant::now();
            let hints = self.engine.recall_hints();
            self.send_effects(sessions, hints);
            if self.fatal_error.is_some() {
                return;
            }
        }

        let timeout = self.session_timeout;
        let autoclose = self.session_autoclose;
        let now = Instant::now();
        let mut clients = sessions.clients();
        clients.sort_unstable();
        for client in clients {
            let Some(s) = sessions.get(client) else {
                continue;
            };
            if s.state == SessionState::Fencing {
                continue;
            }
            let renewed = s.last_renew.elapsed();
            let engine_stale = self.engine.session(client).is_some_and(|es| es.is_stale());
            if engine_stale {
                if let Some(t) = self.stale_since.get(&client)
                    && now.duration_since(*t) > autoclose
                {
                    warn!(client, "stale session past session_autoclose: fencing");
                    self.request_eviction(sessions, client);
                    if self.fatal_error.is_some() {
                        return;
                    }
                }
                continue;
            }
            if renewed > timeout {
                info!(
                    client,
                    secs = renewed.as_secs(),
                    "session stale (no renewals)"
                );
                self.stale_since.insert(client, now);
                if let Some(sm) = sessions.get_mut(client) {
                    sm.state = SessionState::Stale;
                }
                let effects = self.engine.session_stale(client);
                #[cfg(test)]
                let effects = {
                    let mut effects = effects;
                    effects.append(&mut self.timer_fault_effects);
                    effects
                };
                if let Some(sm) = sessions.get_mut(client) {
                    sm.send(&MClientSession::new(SessionOp::STALE, 0));
                }
                if sessions.output_failed() {
                    self.fatal_error = Some("session output unavailable or over budget".into());
                    return;
                }
                self.send_effects(sessions, effects);
                if self.fatal_error.is_some() {
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::Message;
    use engine::{Engine, EngineConfig, MemJournal};
    use proto::request::{MknodArgs, OpenArgs, ReaddirArgs, SetattrArgs};
    use proto::{MClientRequest, MdsOp};

    #[derive(Clone, Default)]
    struct TestLink(std::sync::Arc<std::sync::Mutex<Vec<denc::RawMessage>>>);

    impl crate::session::ClientLink for TestLink {
        fn send_raw(&self, msg: denc::RawMessage) -> bool {
            self.0.lock().unwrap().push(msg);
            true
        }
        fn features(&self) -> u64 {
            denc::features::ALL
        }
        fn peer_addr(&self) -> denc::EntityAddr {
            denc::EntityAddr::new(denc::AddrType::LEGACY, "127.0.0.1:6789".parse().unwrap(), 1)
        }
    }

    fn test_server() -> (Server<MemJournal>, SessionMap, TestLink) {
        let mut server = Server::new("a".into(), 2, 7);
        let mut sessions = SessionMap::default();
        let link = TestLink::default();
        sessions.handle(
            4100,
            Box::new(link.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        server.session_opened(&mut sessions, 4100);
        (server, sessions, link)
    }

    fn replies(link: &TestLink) -> Vec<MClientReply> {
        link.0
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|m| MClientReply::from_raw(m.clone()).unwrap())
            .collect()
    }

    fn mkdir_request(tid: u64, name: &str) -> MClientRequest {
        let mut m = MClientRequest::new(tid, MdsOp::MKDIR);
        m.path = WireFilePath::new(1, name.to_owned());
        m.head.flags = proto::request::request_flags::WANT_DENTRY;
        m
    }

    #[test]
    fn malformed_create_xattrs_do_not_mutate_namespace_or_journal() {
        let (mut server, mut sessions, link) = test_server();
        let before = server.engine.journal().last_seq();
        let mut m = mkdir_request(1, "bad");
        m.data = Bytes::from_static(b"\x01");
        server.handle(&mut sessions, 4100, ClientMessage::Request(m));
        assert_eq!(replies(&link)[0].result, -22);
        assert_eq!(server.engine.journal().last_seq(), before);
        let mut lookup = MClientRequest::new(2, MdsOp::LOOKUP);
        lookup.path = WireFilePath::new(1, "bad");
        server.handle(&mut sessions, 4100, ClientMessage::Request(lookup));
        assert_eq!(replies(&link)[1].result, -2);
        assert!(server.pending_requests.is_empty());
        assert!(server.fatal_error().is_none());
    }

    #[test]
    fn malformed_cap_xattrs_do_not_mutate_or_ack_flush() {
        let (mut server, mut sessions, link) = test_server();
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(1, "d")),
        );
        let inode = replies(&link)[0].trace.target.clone().unwrap();
        let before = server.engine.journal().last_seq();
        let messages = link.0.lock().unwrap().len();
        let m = MClientCaps {
            op: CapOp::UPDATE,
            ino: inode.ino,
            cap_id: inode.cap.cap_id,
            seq: inode.cap.seq,
            issue_seq: inode.cap.seq,
            dirty: proto::types::cap::XATTR_EXCL,
            tid: 99,
            xattr_version: 7,
            xattrbl: Bytes::from_static(b"\x01"),
            mode: 0o777,
            size: 1234,
            ..Default::default()
        };
        server.handle(&mut sessions, 4100, ClientMessage::Caps(m));
        assert!(server.fatal_error().is_some());
        assert_eq!(server.engine.journal().last_seq(), before);
        assert_eq!(link.0.lock().unwrap().len(), messages);
        assert_eq!(server.engine.inode(inode.ino).unwrap().mode, inode.mode);
    }

    #[test]
    fn invalid_cap_opcodes_cannot_apply_dirty_payload() {
        for op in [CapOp::GRANT, CapOp(999)] {
            let (mut server, mut sessions, link) = test_server();
            server.handle(
                &mut sessions,
                4100,
                ClientMessage::Request(mkdir_request(1, "d")),
            );
            let inode = replies(&link)[0].trace.target.clone().unwrap();
            let before = server.engine.inode(inode.ino).unwrap().clone();
            let cap = server.engine.cap(inode.ino, 4100).unwrap().clone();
            let seq = server.engine.journal().last_seq();
            let count = link.0.lock().unwrap().len();
            server.handle(
                &mut sessions,
                4100,
                ClientMessage::Caps(MClientCaps {
                    op,
                    ino: inode.ino,
                    cap_id: cap.cap_id,
                    seq: cap.last_sent,
                    issue_seq: cap.last_issue,
                    caps: cap.issued(),
                    wanted: 0,
                    dirty: proto::types::cap::AUTH_EXCL | proto::types::cap::XATTR_EXCL,
                    tid: 99,
                    mode: 0o40777,
                    uid: 55,
                    gid: 66,
                    xattr_version: 7,
                    xattrbl: xattrs_to_blob(
                        &[(Bytes::from_static(b"user.test"), Bytes::from_static(b"bad"))]
                            .into_iter()
                            .collect(),
                    ),
                    ..Default::default()
                }),
            );
            assert!(server.fatal_error().is_some(), "{op:?}");
            assert_eq!(server.engine.inode(inode.ino).unwrap(), &before);
            assert_eq!(
                format!("{:?}", server.engine.cap(inode.ino, 4100).unwrap()),
                format!("{cap:?}")
            );
            assert_eq!(server.engine.journal().last_seq(), seq);
            assert_eq!(link.0.lock().unwrap().len(), count);
            assert!(
                !server
                    .engine
                    .session(4100)
                    .unwrap()
                    .completed_flushes
                    .contains(&99)
            );
        }
    }

    #[test]
    fn legal_client_cap_opcodes_remain_admitted() {
        for op in [
            CapOp::UPDATE,
            CapOp::FLUSH,
            CapOp::DROP,
            CapOp::RELEASE,
            CapOp::RENEW,
            CapOp::FLUSHSNAP,
        ] {
            let (mut server, mut sessions, link) = test_server();
            server.handle(
                &mut sessions,
                4100,
                ClientMessage::Request(mkdir_request(1, "d")),
            );
            let inode = replies(&link)[0].trace.target.clone().unwrap();
            server.handle(
                &mut sessions,
                4100,
                ClientMessage::Caps(MClientCaps {
                    op,
                    ino: inode.ino,
                    cap_id: inode.cap.cap_id,
                    seq: inode.cap.seq,
                    issue_seq: inode.cap.seq,
                    caps: inode.cap.caps,
                    wanted: inode.cap.wanted,
                    tid: 99,
                    ..Default::default()
                }),
            );
            assert!(server.fatal_error().is_none(), "{op:?}");
        }
    }

    #[test]
    fn autoclose_queues_fence_and_retains_session_rights() {
        let (mut server, mut sessions, link) = test_server();
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        server.set_evict_channel(tx);
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(1, "d")),
        );
        let ino = replies(&link)[0].trace.target.as_ref().unwrap().ino;
        sessions.get_mut(4100).unwrap().last_renew = Instant::now() - Duration::from_secs(61);
        server.tick(&mut sessions);
        assert_eq!(sessions.get(4100).unwrap().state, SessionState::Stale);
        server
            .stale_since
            .insert(4100, Instant::now() - Duration::from_secs(301));
        let seq = server.engine.journal().last_seq();
        server.tick(&mut sessions);
        let request = rx.try_recv().unwrap();
        assert_eq!(request.client, 4100);
        assert_eq!(sessions.get(4100).unwrap().state, SessionState::Fencing);
        assert!(server.engine.session(4100).is_some());
        assert!(server.engine.cap(ino, 4100).is_some());
        assert_eq!(server.engine.journal().last_seq(), seq);
        server.tick(&mut sessions);
        assert!(rx.try_recv().is_err());
        assert!(server.engine.cap(ino, 4100).is_some());
        assert!(server.finish_eviction(&mut sessions, request, 11));
        assert_eq!(server.epoch_barrier(), 11);
        assert!(sessions.get(4100).is_none());
        assert!(server.engine.session(4100).is_none());
    }

    #[test]
    fn delayed_fence_cannot_remove_reopened_session() {
        let (mut server, mut sessions, _) = test_server();
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        server.set_evict_channel(tx);
        server.request_eviction(&mut sessions, 4100);
        let old = rx.try_recv().unwrap();
        // Simulate a completed old lifecycle and subsequent OPEN before a duplicate
        // worker callback arrives. Global ids and peer addresses can be identical.
        server.kill_session(&mut sessions, 4100);
        sessions.handle(
            4100,
            Box::new(TestLink::default()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        server.session_opened(&mut sessions, 4100);
        server.request_eviction(&mut sessions, 4100);
        let new = rx.try_recv().unwrap();
        assert_ne!(old.session_generation, new.session_generation);
        let seq = server.engine.journal().last_seq();
        assert!(!server.finish_eviction(&mut sessions, old, 99));
        assert_eq!(server.epoch_barrier(), 0);
        assert_eq!(server.engine.journal().last_seq(), seq);
        assert!(server.engine.session(4100).is_some());
        assert!(server.finish_eviction(&mut sessions, new, 7));
        server.raise_epoch_barrier(2);
        assert_eq!(server.epoch_barrier(), 7);
    }

    #[test]
    fn incoming_barriers_reach_flush_acks_and_later_grants() {
        let (mut server, mut sessions, link) = test_server();
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(1, "d")),
        );
        let inode = replies(&link)[0].trace.target.clone().unwrap();
        let cap = server.engine.cap(inode.ino, 4100).unwrap().clone();
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Caps(MClientCaps {
                op: CapOp::FLUSH,
                ino: inode.ino,
                cap_id: cap.cap_id,
                seq: cap.last_sent,
                issue_seq: cap.last_issue,
                caps: cap.issued(),
                wanted: cap.wanted(),
                dirty: proto::types::cap::AUTH_EXCL,
                tid: 99,
                mode: inode.mode,
                uid: inode.uid,
                gid: inode.gid,
                osd_epoch_barrier: 17,
                ..Default::default()
            }),
        );
        let caps: Vec<_> = link
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.msg_type == proto::msg_type::CLIENT_CAPS)
            .map(|m| MClientCaps::from_raw(m.clone()).unwrap())
            .collect();
        assert!(
            caps.iter()
                .any(|m| m.op == CapOp::FLUSH_ACK && m.tid == 99 && m.osd_epoch_barrier == 17)
        );
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::CapRelease(proto::MClientCapRelease {
                caps: vec![],
                osd_epoch_barrier: 23,
            }),
        );
        server.raise_epoch_barrier(3);
        assert_eq!(server.epoch_barrier(), 23);
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Caps(MClientCaps {
                op: CapOp::FLUSHSNAP,
                ino: inode.ino,
                tid: 100,
                osd_epoch_barrier: 4,
                ..Default::default()
            }),
        );
        let last = link.0.lock().unwrap().last().unwrap().clone();
        let ack = MClientCaps::from_raw(last).unwrap();
        assert_eq!((ack.op, ack.osd_epoch_barrier), (CapOp::FLUSHSNAP_ACK, 23));
    }

    #[test]
    fn engine_failure_stops_replies_and_further_admission() {
        let (mut server, mut sessions, link) = test_server();
        server.engine.journal_mut().set_auto_commit(false);
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(1, "pending")),
        );
        let count = server.pending_requests.len();
        let submitted = server.engine.journal().last_seq();
        let step = server.engine.journal_event(engine::JournalEvent::Failed(
            engine::JournalError::Unavailable,
        ));
        assert!(step.effects.is_empty());
        assert!(server.fatal_error().is_some());
        server.engine.journal_mut().commit_all();
        server.tick(&mut sessions);
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(2, "after")),
        );
        assert!(replies(&link).is_empty());
        assert_eq!(server.pending_requests.len(), count);
        assert_eq!(server.engine.journal().last_seq(), submitted);
    }

    #[test]
    fn timer_stops_before_second_expired_session_after_invalid_reply() {
        let (mut server, mut sessions, link) = test_server();
        sessions.handle(
            4101,
            Box::new(link.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        server.session_opened(&mut sessions, 4101);
        server.engine.journal_mut().set_auto_commit(false);
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(1, "d")),
        );
        server.engine.journal_mut().commit_all();
        let mut effects = server.engine.journal_committed();
        for e in &mut effects {
            if let Effect::Reply(r) = e {
                r.dentry.as_mut().unwrap().dir.ino = 2;
            }
        }
        server.timer_fault_effects = effects;
        for s in sessions.iter_mut() {
            s.last_renew = Instant::now() - Duration::from_secs(61);
        }
        let before = link.0.lock().unwrap().len();
        server.tick(&mut sessions);
        assert!(server.fatal_error().is_some());
        assert_eq!(sessions.get(4101).unwrap().state, SessionState::Open);
        assert!(!server.engine.session(4101).unwrap().is_stale());
        let after = link.0.lock().unwrap();
        assert_eq!(
            after[before..]
                .iter()
                .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_SESSION)
                .count(),
            1
        );
    }

    #[test]
    fn deferred_reply_keeps_context_and_retries_do_not_leak() {
        let (mut server, mut sessions, link) = test_server();
        server.engine.journal_mut().set_auto_commit(false);
        let request = mkdir_request(1, "d");
        server.handle(&mut sessions, 4100, ClientMessage::Request(request.clone()));
        let seq = server.engine.journal().last_seq();
        let mut retry = request.clone();
        retry.head.num_retry = 1;
        server.handle(&mut sessions, 4100, ClientMessage::Request(retry.clone()));
        assert_eq!(server.pending_requests.len(), 1);
        assert_eq!(server.engine.journal().last_seq(), seq);
        assert!(replies(&link).is_empty());
        server.flush_mdlog(&mut sessions);
        assert!(replies(&link).is_empty());
        assert_eq!(server.mdlog_flush_target, Some(seq));
        assert!(server.engine.journal().flush_count() > 0);
        server.engine.journal_mut().commit_all();
        server.tick(&mut sessions);
        assert_eq!(server.mdlog_flush_target, None);
        assert_eq!(replies(&link).len(), 1);
        assert!(replies(&link)[0].check_invariants_for(&request).is_empty());
        assert!(server.pending_requests.is_empty());
        server.handle(&mut sessions, 4100, ClientMessage::Request(retry));
        assert_eq!(replies(&link).len(), 2);
        assert_eq!(server.engine.journal().last_seq(), seq);
        assert!(server.pending_requests.is_empty());
        assert!(server.fatal_error().is_none());
    }

    #[test]
    fn deferred_reply_wrong_parent_stops_before_send() {
        let (mut server, mut sessions, link) = test_server();
        server.engine.journal_mut().set_auto_commit(false);
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(1, "d")),
        );
        server.engine.journal_mut().commit_all();
        let mut effects = server.engine.journal_committed();
        let mut injected = false;
        for effect in &mut effects {
            if let Effect::Reply(r) = effect {
                // Inject a valid inode number but the wrong locked parent at the
                // engine/wire boundary. Request-independent checks cannot detect it.
                r.dentry.as_mut().unwrap().dir.ino = 2;
                injected = true;
            }
        }
        assert!(injected);
        server.send_effects(&mut sessions, effects);
        assert!(server.fatal_error().unwrap().contains("request's parent"));
        assert!(replies(&link).is_empty());
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(2, "later")),
        );
        server.tick(&mut sessions);
        assert!(replies(&link).is_empty());
    }

    #[test]
    fn parked_request_contexts_are_bounded_and_removed_on_close() {
        let (mut server, mut sessions, link) = test_server();
        server.engine.journal_mut().set_auto_commit(false);
        for tid in 1..=MAX_PENDING_REQUESTS as u64 {
            server.handle(
                &mut sessions,
                4100,
                ClientMessage::Request(mkdir_request(tid, "d")),
            );
        }
        assert_eq!(server.pending_requests.len(), MAX_PENDING_REQUESTS);
        assert!(replies(&link).is_empty());
        let submitted = server.engine.journal().last_seq();
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(MAX_PENDING_REQUESTS as u64 + 1, "overflow")),
        );
        assert_eq!(server.pending_requests.len(), MAX_PENDING_REQUESTS);
        assert_eq!(server.engine.journal().last_seq(), submitted);
        assert!(server.fatal_error().is_some());

        let (mut server, mut sessions, _) = test_server();
        server.engine.journal_mut().set_auto_commit(false);
        server.handle(
            &mut sessions,
            4100,
            ClientMessage::Request(mkdir_request(1, "d")),
        );
        assert_eq!(server.pending_requests.len(), 1);
        assert_eq!(
            sessions.handle(
                4100,
                Box::new(TestLink::default()),
                MClientSession::new(
                    SessionOp::REQUEST_CLOSE,
                    sessions.get(4100).unwrap().push_seq()
                ),
                vec![]
            ),
            crate::session::SessionOutcome::Closed
        );
        server.session_closed(&mut sessions, 4100);
        assert!(server.pending_requests.is_empty());
        server.engine.journal_mut().commit_all();
        server.tick(&mut sessions);
        assert!(server.fatal_error().is_none());
    }

    fn engine_request(req: &MClientRequest) -> Request {
        request_from_proto(
            req,
            engine::Timestamp::new(1_700_000_000, 0),
            denc::features::CREATEPOOLID,
        )
        .unwrap()
    }

    fn engine_request_kernel(req: &MClientRequest) -> Request {
        request_from_proto(req, engine::Timestamp::new(1_700_000_000, 0), 0).unwrap()
    }

    fn engine_with_root_cap() -> Engine<MemJournal> {
        Engine::new(EngineConfig::default(), MemJournal::new())
    }

    #[test]
    fn create_request_translation() {
        let mut m = MClientRequest::new(7, MdsOp::CREATE);
        m.head.caller_uid = 1000;
        m.head.caller_gid = 1001;
        m.head.owner_uid = 1000;
        m.head.owner_gid = 1001;
        m.head.flags = proto::request::request_flags::WANT_DENTRY;
        m.head.oldest_client_tid = 5;
        m.head.args = OpenArgs {
            flags: proto::open_flags::CREAT | proto::open_flags::RDWR,
            mode: 0o644,
            stripe_unit: 4 << 20,
            pool: 3,
            ..Default::default()
        }
        .to_args();
        m.path = WireFilePath::new(1, "dir/f");
        m.stamp = denc::UTime { sec: 42, nsec: 7 };
        m.releases.push(proto::RequestRelease {
            ino: 1,
            cap_id: 9,
            caps: 0x11,
            wanted: 0x22,
            seq: 3,
            issue_seq: 2,
            mseq: 1,
            dname_seq: 0,
            dname: Bytes::from_static(b"f"),
        });
        let r = engine_request(&m);
        let Op::Create {
            path,
            flags,
            mode,
            layout,
            xattrs,
            ino,
        } = r.op.clone()
        else {
            panic!()
        };
        assert_eq!(
            path,
            engine::FilePath {
                ino: 1,
                names: vec![b"dir".to_vec(), b"f".to_vec()]
            }
        );
        assert_eq!(
            (flags, mode, ino),
            (proto::open_flags::CREAT | proto::open_flags::RDWR, 0o644, 0)
        );
        assert_eq!(layout.pool, 3);
        assert!(xattrs.is_empty());
        assert_eq!(
            (
                r.tid,
                r.caller_uid,
                r.owner_gid,
                r.flags,
                r.retry_attempt,
                r.oldest_client_tid
            ),
            (
                7,
                1000,
                1001,
                proto::request::request_flags::WANT_DENTRY,
                0,
                5
            )
        );
        assert_eq!(r.stamp, engine::Timestamp::new(42, 7));
        assert_eq!(r.releases.len(), 1);
        assert_eq!(r.releases[0].dname.as_deref(), Some(&b"f"[..]));
    }

    /// The kernel (no `CREATEPOOLID` on the connection) leaves `args.open.pool` zero; the
    /// field must be ignored, not treated as an override to pool 0 (which fails the engine's
    /// data-pool check with EINVAL). Exactly what the captured kernel-v1 CREATE sends.
    #[test]
    fn kernel_create_pool_zero_is_inherit() {
        let mut m = MClientRequest::new(3, MdsOp::CREATE);
        m.head.args = OpenArgs {
            flags: 0o101,
            mode: 0o100644,
            pool: 0,
            ..Default::default()
        }
        .to_args();
        m.path = WireFilePath::new(0x1000_0000_0000, "file1");
        let Op::Create { layout, .. } = engine_request_kernel(&m).op else {
            panic!()
        };
        assert_eq!(layout.pool, -1);
        // The same request from a libcephfs peer (feature present) keeps pool 0 as an override.
        m.head.args = OpenArgs {
            flags: 0o101,
            mode: 0o100644,
            pool: 7,
            ..Default::default()
        }
        .to_args();
        let Op::Create { layout, .. } = engine_request(&m).op else {
            panic!()
        };
        assert_eq!(layout.pool, 7);
    }

    #[test]
    fn zero_stamp_is_filled_owner_falls_back_to_caller() {
        let mut m = MClientRequest::new(1, MdsOp::MKDIR);
        m.head.args = MknodArgs {
            mode: 0o755,
            rdev: 0,
        }
        .to_args();
        m.path = WireFilePath::new(1, "d");
        m.head.caller_uid = 1234;
        m.head.owner_uid = u32::MAX; // a non-new-inode op: u32::MAX garbage
        let r = engine_request(&m);
        assert_eq!(r.owner_uid, 1234);
        assert_eq!(r.stamp, engine::Timestamp::new(1_700_000_000, 0));
    }

    #[test]
    fn readdir_and_rename_translation() {
        let mut m = MClientRequest::new(2, MdsOp::READDIR);
        m.path = WireFilePath::new(0x1000, "");
        m.path2 = WireFilePath::new(0, "last");
        m.head.args = ReaddirArgs {
            frag: 0,
            max_entries: 64,
            max_bytes: 4096,
            flags: proto::readdir_flags::REPLY_BITFLAGS,
            offset_hash: 8,
        }
        .to_args();
        let Op::Readdir {
            path,
            max_entries,
            last_name,
            offset_hash,
            flags,
            ..
        } = engine_request(&m).op
        else {
            panic!()
        };
        assert_eq!(path, engine::FilePath::ino(0x1000));
        assert_eq!(max_entries, 64);
        assert_eq!(last_name.as_deref(), Some(&b"last"[..]));
        assert_eq!(offset_hash, 8);
        assert_eq!(flags, u32::from(proto::readdir_flags::REPLY_BITFLAGS));

        let mut m = MClientRequest::new(3, MdsOp::RENAME);
        m.path = WireFilePath::new(1, "a");
        m.path2 = WireFilePath::new(1, "b");
        let Op::Rename { dst, src } = engine_request(&m).op else {
            panic!()
        };
        assert_eq!(dst, engine::FilePath::child(1, "a"));
        assert_eq!(src, engine::FilePath::child(1, "b"));
    }

    #[test]
    fn setattr_translation() {
        let mut m = MClientRequest::new(4, MdsOp::SETATTR);
        m.path = WireFilePath::new(0x2000, "");
        m.head.args = SetattrArgs {
            mask: proto::types::setattr::MODE,
            mode: 0o600,
            ..Default::default()
        }
        .to_args();
        let Op::Setattr { args, .. } = engine_request(&m).op else {
            panic!()
        };
        assert_eq!(args.mask, proto::types::setattr::MODE);
        assert_eq!(args.mode, 0o600);
    }

    #[test]
    fn unknown_op_maps_to_other() {
        let m = MClientRequest::new(5, MdsOp::SETLAYOUT);
        let Op::Other { op } = engine_request(&m).op else {
            panic!()
        };
        assert_eq!(op, MdsOp::SETLAYOUT.0);
    }

    #[test]
    fn cap_update_translation() {
        let mut blob = denc::BytesMut::new();
        proto::encode_xattrs(
            &[(Bytes::from_static(b"user.x"), Bytes::from_static(b"v"))]
                .into_iter()
                .collect(),
            &mut blob,
        );
        let m = MClientCaps {
            op: CapOp::GRANT,
            ino: 0x1000,
            cap_id: 77,
            seq: 3,
            issue_seq: 2,
            migrate_seq: 5,
            caps: 0x155,
            wanted: 0x33,
            dirty: 0x22,
            tid: 42,
            oldest_flush_tid: 40,
            size: 12,
            max_size: 4 << 20,
            change_attr: 9,
            xattr_version: 3,
            xattrbl: blob.freeze(),
            ..MClientCaps::default()
        };
        let u = cap_update_from_proto(&m).unwrap();
        assert_eq!(
            (
                u.op,
                u.ino,
                u.cap_id,
                u.seq,
                u.issue_seq,
                u.mseq,
                u.caps,
                u.wanted,
                u.dirty,
                u.flush_tid
            ),
            (CapOp::GRANT.0, 0x1000, 77, 3, 2, 5, 0x155, 0x33, 0x22, 42)
        );
        assert_eq!(u.oldest_flush_tid, 40);
        assert_eq!(u.size, 12);
        assert_eq!(u.change_attr, 9);
        let (v, x) = u.xattrs.expect("xattrs decoded");
        assert_eq!(v, 3);
        assert_eq!(
            x.get(b"user.x".as_slice()).map(|v| v.as_slice()),
            Some("v".as_bytes())
        );
    }

    #[test]
    fn inode_stat_translation_invariants() {
        let mut e = engine_with_root_cap();
        e.open_session(4100);
        let req = Request::new(
            1,
            Op::Create {
                path: engine::FilePath::child(1, "hello"),
                flags: proto::open_flags::CREAT | proto::open_flags::RDWR,
                mode: 0o644,
                layout: LayoutOverride::default(),
                xattrs: engine::XattrMap::new(),
                ino: 0,
            },
        )
        .with_owner(1000, 1000)
        .with_stamp(engine::Timestamp::new(1_700_000_000, 0));
        let effects = e.handle_request(4100, req);
        let Effect::Reply(reply) = &effects[0] else {
            panic!()
        };
        assert_eq!(reply.result, 0);
        let engine_st = reply.target.as_ref().unwrap();
        assert_eq!(engine_st.max_size, 4 << 20);
        let st = inode_stat(engine_st);
        assert_eq!(st.ino, engine_st.ino);
        assert!(st.ino >= 0x1000);
        assert_ne!(st.cap.caps, 0);
        assert_eq!(st.max_size, 4 << 20);
        assert_ne!(st.inline_version, 0);
        assert!(
            st.check_invariants().is_empty(),
            "{:?}",
            st.check_invariants()
        );
    }

    #[test]
    fn reply_effect_becomes_wire_message() {
        use crate::session::{ClientLink, SessionMap};
        use proto::MClientSession;
        use std::sync::{Arc, Mutex};

        #[derive(Clone, Default)]
        struct Recorder(Arc<Mutex<Vec<denc::RawMessage>>>);
        impl ClientLink for Recorder {
            fn send_raw(&self, msg: denc::RawMessage) -> bool {
                self.0.lock().unwrap().push(msg);
                true
            }
            fn features(&self) -> u64 {
                denc::features::ALL
            }
            fn peer_addr(&self) -> denc::EntityAddr {
                denc::EntityAddr::new(denc::AddrType::LEGACY, "127.0.0.1:6789".parse().unwrap(), 1)
            }
        }

        let mut server = Server::new("a".into(), 2, 7);
        let mut sessions = SessionMap::default();
        let rec = Recorder::default();
        sessions.handle(
            4100,
            Box::new(rec.clone()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            Vec::new(),
        );
        server.session_opened(&mut sessions, 4100);

        let mut m = MClientRequest::new(1, MdsOp::MKDIR);
        m.path = WireFilePath::new(1, "d");
        server.handle(&mut sessions, 4100, ClientMessage::Request(m));

        let msgs = rec.0.lock().unwrap().clone();
        let replies: Vec<MClientReply> = msgs
            .iter()
            .filter(|r| r.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|r| MClientReply::from_raw(r.clone()).unwrap())
            .collect();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].result, 0);
        assert_eq!(replies[0].mdsmap_epoch, 7);
        assert!(replies[0].trace.dentry.is_some());
        assert!(replies[0].trace.target.is_some());
        assert!(!replies[0].snapbl.is_empty());
        assert!(
            replies[0].check_invariants().is_empty(),
            "{:?}",
            replies[0].check_invariants()
        );
    }

    /// `kill_session` (the tail of `EvictClient`): the engine session closes with
    /// the kill flag and the daemon-side session is gone.
    #[test]
    fn kill_session_removes_the_session() {
        #[derive(Clone, Default)]
        struct Rec(std::sync::Arc<std::sync::Mutex<Vec<denc::RawMessage>>>);
        impl crate::session::ClientLink for Rec {
            fn send_raw(&self, msg: denc::RawMessage) -> bool {
                self.0.lock().unwrap().push(msg);
                true
            }
            fn features(&self) -> u64 {
                denc::features::ALL
            }
            fn peer_addr(&self) -> denc::EntityAddr {
                denc::EntityAddr::new(denc::AddrType::LEGACY, "127.0.0.1:6789".parse().unwrap(), 1)
            }
        }
        let mut server = Server::new("a".into(), 2, 7);
        let mut sessions = SessionMap::default();
        sessions.handle(
            4100,
            Box::new(Rec::default()),
            MClientSession::new(SessionOp::REQUEST_OPEN, 0),
            Vec::new(),
        );
        server.session_opened(&mut sessions, 4100);
        assert!(sessions.get(4100).is_some());
        server.kill_session(&mut sessions, 4100);
        assert!(sessions.get(4100).is_none());
        // Killing again is a no-op.
        server.kill_session(&mut sessions, 4100);
    }
}
