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
//!   `Reconnect` is denied with `CLOSE` until persistence exists; `Reclaim` answers "unknown
//!   uuid" (result 0, no address), which FSAL_CEPH treats as ENOENT.
//! - Effects: `Reply` → `MClientReply` (engine `InodeStat`/`DirFragInfo`/`LeaseStat` → `proto`
//!   types; snap trace in `SnapFormat::for_client` of the session's features), `Caps` →
//!   `MClientCaps`, `RealmSplit` → `MClientSnap` (before the reply it belongs to), `Recall` →
//!   `RECALL_STATE`; `TruncateData`, `PurgeInode`, `RecoverFileSize` run as RADOS work through
//!   the Objecter ([`crate::rados`]) once the daemon provides it (`set_objecter`);
//!   `EvictClient` hands the client to the daemon for the monitor blocklist, then the kill.
//!   Every message goes through `Session::send`, in the order the engine returned the effects.
//! - `tick`: journal completions, engine retries, recall hints, and the session timers
//!   (`session_timeout` → STALE, `session_autoclose` after that) from the MDSMap.
//!
//! Invariant checking: before a reply is sent, `MClientReply::check_invariants` runs and
//! violations are logged (`check_invariants_for` needs the request, which deferred replies no
//! longer carry; the request-independent checks cover the crash triggers of design section 7).

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use denc::Bytes;
use engine::{
    CapReleaseItem, CapUpdate, Effect, Engine, EngineConfig, LayoutOverride, MemJournal, Op,
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
use tracing::{debug, info, warn};

use osdc::objecter::Objecter;

use crate::session::{SessionMap, SessionState};

/// How often `Engine::recall_hints` runs (C++ uses the mds tick, 1 s; every few is enough).
const RECALL_HINT_INTERVAL: Duration = Duration::from_secs(10);

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
/// request's data payload; a payload that does not decode is dropped with a warning (the C++
/// fails the request; the kernel only sends well-formed blobs).
fn request_from_proto(
    req: &proto::MClientRequest,
    now: engine::Timestamp,
    conn_features: u64,
) -> Request {
    let h = &req.head;
    let a = &h.args;
    let getattr_mask = a.getattr_mask();
    let lookupino = a.lookupino();
    let open = a.open();
    let xattrs = |req: &proto::MClientRequest| -> engine::XattrMap {
        match req.initial_xattrs() {
            Ok(x) => xattrs_from_wire(&x),
            Err(e) => {
                warn!(tid = req.tid, "undecodable initial xattrs ({e}); ignored");
                engine::XattrMap::new()
            }
        }
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
                xattrs: xattrs(req),
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
            xattrs: xattrs(req),
            ino: h.ino,
        },
        MdsOp::RMDIR => Op::Rmdir {
            path: file_path(&req.path),
        },
        MdsOp::SYMLINK => Op::Symlink {
            path: file_path(&req.path),
            target: req.path2.path.to_vec(),
            xattrs: xattrs(req),
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
            xattrs: xattrs(req),
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
    Request {
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
    }
}

/// `MClientCaps` from a client → `engine::CapUpdate`. An xattr blob that does not decode is
/// dropped with a warning (the client sends what we issued).
fn cap_update_from_proto(m: &MClientCaps) -> CapUpdate {
    let xattrs = (!m.xattrbl.is_empty()).then(|| {
        decode_xattr_blob(m.xattrbl.clone())
            .map(|x| (m.xattr_version, xattrs_from_wire(&x)))
            .unwrap_or_else(|e| {
                warn!(
                    ino = m.ino,
                    "undecodable xattr blob in cap message ({e}); ignored"
                );
                (m.xattr_version, engine::XattrMap::new())
            })
    });
    CapUpdate {
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
    }
}

fn cap_item(i: &CapItem) -> CapReleaseItem {
    CapReleaseItem {
        ino: i.ino,
        cap_id: i.cap_id,
        mseq: i.migrate_seq,
        issue_seq: i.issue_seq,
    }
}

pub struct Server {
    pub fs_name: String,
    pub data_pool: i64,
    pub mdsmap_epoch: u32,
    engine: Engine<MemJournal>,
    /// `MDSMap::session_timeout`: renewals older than this make the session stale.
    session_timeout: Duration,
    /// `MDSMap::session_autoclose`: a stale session this old is killed.
    session_autoclose: Duration,
    /// When a session became stale (engine client id → time).
    stale_since: HashMap<u64, Instant>,
    last_recall_hints: Instant,
    /// The OSD client: runs the RADOS work of `PurgeInode`, `TruncateData` and
    /// `RecoverFileSize` (set by the daemon once the Objecter exists).
    objecter: Option<Objecter>,
    /// Where `EvictClient` goes: the daemon blocklists the client through the
    /// monitor, then kills the session (the reply channel).
    evict_tx: Option<tokio::sync::mpsc::UnboundedSender<(u64, denc::EntityAddr)>>,
}

impl Server {
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
        Server {
            fs_name,
            data_pool,
            mdsmap_epoch,
            engine,
            session_timeout: Duration::from_secs(60),
            session_autoclose: Duration::from_secs(300),
            stale_since: HashMap::new(),
            last_recall_hints: Instant::now(),
            objecter: None,
            evict_tx: None,
        }
    }

    /// Provide the OSD client for the RADOS-work effects.
    pub fn set_objecter(&mut self, objecter: Objecter) {
        self.objecter = Some(objecter);
    }

    /// Provide the eviction channel (the daemon blocklists, then kills).
    pub fn set_evict_channel(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<(u64, denc::EntityAddr)>,
    ) {
        self.evict_tx = Some(tx);
    }

    pub fn set_mdsmap(&mut self, map: &MdsMap) {
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

    pub fn session_opened(&mut self, sessions: &mut SessionMap, client: u64) {
        self.stale_since.remove(&client);
        let effects = self.engine.open_session(client);
        self.send_effects(sessions, effects);
        info!(client, "session opened");
    }

    pub fn session_closed(&mut self, sessions: &mut SessionMap, client: u64) {
        self.stale_since.remove(&client);
        let effects = self.engine.close_session(client, false);
        self.send_effects(sessions, effects);
        info!(client, "session closed");
    }

    /// The client renewed (or recovered a stale session): re-issue its caps if it was stale.
    pub fn session_resumed(&mut self, sessions: &mut SessionMap, client: u64) {
        let effects = self.engine.session_resume(client);
        self.send_effects(sessions, effects);
    }

    pub fn handle(&mut self, sessions: &mut SessionMap, client: u64, msg: ClientMessage) {
        match msg {
            ClientMessage::Request(req) => {
                let tid = req.tid;
                let op = req.op();
                let conn_features = sessions.get(client).map(|s| s.link.features()).unwrap_or(0);
                let ereq = request_from_proto(&req, now_ts(), conn_features);
                let effects = self.engine.handle_request(client, ereq);
                debug!(client, tid, op = ?op, effects = effects.len(), "request handled");
                self.send_effects(sessions, effects);
            }
            ClientMessage::Caps(m) => {
                debug!(client, ino = m.ino, op = m.op.name(), "cap update");
                let effects = self
                    .engine
                    .handle_cap_update(client, cap_update_from_proto(&m));
                self.send_effects(sessions, effects);
            }
            ClientMessage::CapRelease(m) => {
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

    /// Send the effects of an engine call, in order, through the sessions' funnels.
    fn send_effects(&mut self, sessions: &mut SessionMap, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Reply(r) => {
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
                    let violations = reply.check_invariants();
                    if !violations.is_empty() {
                        warn!(
                            client = r.client,
                            tid = r.tid,
                            ?violations,
                            "reply invariant violations"
                        );
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
                        0,
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
                    let addr = sessions.get(client).map(|s| s.addr.clone());
                    if let (Some(tx), Some(addr)) = (&self.evict_tx, addr) {
                        // The daemon blocklists the address through the monitor,
                        // then kills the session (C++ `MDSRank::evict_client`).
                        let _ = tx.send((client, addr));
                    } else {
                        warn!(client, "no monitor session: killing without blocklist");
                        self.kill_session(sessions, client);
                    }
                }
                Effect::TruncateData(t) => match &self.objecter {
                    Some(o) => {
                        crate::rados::spawn_truncate(
                            o.clone(),
                            t.ino,
                            t.layout,
                            t.truncate_seq,
                            t.size,
                            t.from,
                        );
                    }
                    None => warn!(ino = t.ino, "TruncateData: no Objecter yet"),
                },
                Effect::PurgeInode(p) => {
                    if p.is_dir {
                        // Directory objects are metadata-pool work (5.3).
                        debug!(ino = p.ino, "PurgeInode: directory, nothing to do yet");
                    } else if let Some(o) = &self.objecter {
                        crate::rados::spawn_purge(o.clone(), p.ino, p.layout, p.size);
                    } else {
                        warn!(ino = p.ino, "PurgeInode: no Objecter yet");
                    }
                }
                Effect::RecoverFileSize {
                    ino,
                    layout,
                    max_size,
                } => {
                    if let Some(o) = &self.objecter {
                        crate::rados::spawn_recover(o.clone(), ino, layout.clone(), max_size);
                    } else {
                        warn!(ino, max_size, "RecoverFileSize: no Objecter yet");
                    }
                }
            }
        }
    }

    /// Kill a session without `CLOSE` (`Server::kill_session`: engine close
    /// with the kill flag, then the connection is dropped).
    pub fn kill_session(&mut self, sessions: &mut SessionMap, client: u64) {
        self.stale_since.remove(&client);
        let effects = self.engine.close_session(client, true);
        sessions.remove(client);
        self.send_effects(sessions, effects);
    }

    /// Once a second (the daemon tick): journal completions, retries, recall hints and the
    /// session staleness/autoclose timers.
    pub fn tick(&mut self, sessions: &mut SessionMap) {
        let committed = self.engine.journal_committed();
        self.send_effects(sessions, committed);
        let tick = self.engine.tick();
        self.send_effects(sessions, tick);
        if self.last_recall_hints.elapsed() >= RECALL_HINT_INTERVAL {
            self.last_recall_hints = Instant::now();
            let hints = self.engine.recall_hints();
            self.send_effects(sessions, hints);
        }

        let timeout = self.session_timeout;
        let autoclose = self.session_autoclose;
        let now = Instant::now();
        for client in sessions.clients() {
            let Some(s) = sessions.get(client) else {
                continue;
            };
            let renewed = s.last_renew.elapsed();
            let engine_stale = self.engine.session(client).is_some_and(|es| es.is_stale());
            if engine_stale {
                if let Some(t) = self.stale_since.get(&client)
                    && now.duration_since(*t) > autoclose
                {
                    warn!(client, "stale session past session_autoclose: killing");
                    self.stale_since.remove(&client);
                    let effects = self.engine.close_session(client, true);
                    sessions.remove(client);
                    self.send_effects(sessions, effects);
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
                if let Some(sm) = sessions.get_mut(client) {
                    sm.send(&MClientSession::new(SessionOp::STALE, 0));
                }
                self.send_effects(sessions, effects);
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

    fn engine_request(req: &MClientRequest) -> Request {
        request_from_proto(
            req,
            engine::Timestamp::new(1_700_000_000, 0),
            denc::features::CREATEPOOLID,
        )
    }

    fn engine_request_kernel(req: &MClientRequest) -> Request {
        request_from_proto(req, engine::Timestamp::new(1_700_000_000, 0), 0)
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
        let u = cap_update_from_proto(&m);
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
