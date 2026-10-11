//! The metadata engine of one shard: namespace, caps, file locks, sessions.
//!
//! See the crate documentation for the calling contract. Internally a request
//! goes through these steps, re-run from the top whenever it had to wait
//! (like the re-entrant C++ handlers):
//!
//! 1. resolve and validate (errors reply at once);
//! 2. wait while an inode it touches has an uncommitted journal record
//!    (other clients must not observe uncommitted state);
//! 3. register *holds* (temporary cap restrictions, the outcome of C++
//!    rd/wr/xlocks) and wait until no client's issued caps violate them,
//!    sending REVOKEs as needed;
//! 4. for a mutation: change the in-memory state, build one journal record,
//!    submit it, and defer the reply (and any FLUSH_ACK, max_size GRANT, TRUNC,
//!    snap SPLIT) until the record is committed; holds and pending marks are
//!    released at commit, after which caps are re-evaluated (GRANTs).
//!    For a read: reply at once.

mod caps_eval;
mod client_caps;
mod ops_attr;
mod ops_lock;
mod ops_ns;
mod ops_read;
mod ops_xattr;
mod paths;
mod replay;
mod vxattr;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::consts::*;
use crate::effects::*;
use crate::errno::Errno;
use crate::ino::InoAllocator;
use crate::inode::{Hold, HoldKind, Inode, InodeCore, SnapRealmNode};
use crate::journal::{CompletedRequest, Journal, JournalRecord, JournalSeq, RecordKind};
use crate::request::{Op, Request};
use crate::session::Session;
use crate::types::*;

/// Static configuration of an engine (mostly from the MDSMap and config).
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Data pools of the file system; the first is the default
    /// (`MDSMap::get_first_data_pool`).
    pub data_pools: Vec<i64>,
    /// Pool id to name, for layout vxattrs (from the OSDMap).
    pub pool_names: BTreeMap<i64, String>,
    /// `MDSMap::max_xattr_size`.
    pub max_xattr_size: u64,
    pub root_uid: u32,
    pub root_gid: u32,
    /// Permission bits of `/` (`S_IFDIR` is added).
    pub root_mode: u32,
    /// Time stamp of the root inode on a new file system.
    pub mkfs_stamp: Timestamp,
    /// Inode-number lease of this shard.
    pub ino_lease: InoRange,
    /// `mds_client_writeable_range_max_inc_objs`.
    pub writeable_range_max_inc_objs: u64,
    /// `mds_export_ephemeral_random_max`.
    pub export_ephemeral_random_max: f64,
    /// Sessions above this many caps get a recall hint (`mds_max_caps_per_client`).
    pub max_caps_per_client: u64,
    /// Refuse every write with EROFS.
    pub read_only: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let mut pool_names = BTreeMap::new();
        pool_names.insert(1, "cephfs.a.meta".to_string());
        pool_names.insert(2, "cephfs.a.data".to_string());
        EngineConfig {
            data_pools: vec![2],
            pool_names,
            max_xattr_size: MDS_MAX_XATTR_SIZE,
            root_uid: 0,
            root_gid: 0,
            root_mode: 0o755,
            mkfs_stamp: Timestamp::ZERO,
            ino_lease: InoRange::new(RANK0_INO_START, RANK0_INO_LEN),
            writeable_range_max_inc_objs: 1024,
            export_ephemeral_random_max: 0.01,
            max_caps_per_client: 1 << 20,
            read_only: false,
        }
    }
}

impl EngineConfig {
    pub fn default_pool(&self) -> i64 {
        self.data_pools.first().copied().unwrap_or(-1)
    }
    pub fn default_layout(&self) -> FileLayout {
        FileLayout::default_for_pool(self.default_pool())
    }
    pub fn pool_by_name(&self, name: &str) -> Option<i64> {
        self.pool_names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    }
    pub fn is_data_pool(&self, pool: i64) -> bool {
        self.data_pools.contains(&pool)
    }
}

/// Outcome of one attempt to execute a request.
pub(crate) enum Exec {
    Done,
    Wait,
}

/// A request being executed or waiting.
#[derive(Debug, Clone)]
pub(crate) struct Pending {
    pub client: ClientId,
    /// Session epoch the request belongs to.
    pub epoch: u64,
    pub req: Request,
    /// Holds registered by this request: (inode, hold id).
    pub holds: Vec<(Ino, u64)>,
    /// The request already queued a blocking file lock.
    pub flock_was_waiting: bool,
    /// A retried CREATE/OPEN whose first execution completed: open only.
    pub has_completed: bool,
    pub completed_ino: Option<Ino>,
}

/// Work deferred until a record commits, in order.
#[derive(Debug, Clone)]
pub(crate) enum Deferred {
    Reply(ReplySpec),
    FlushAck {
        client: ClientId,
        epoch: u64,
        ino: Ino,
        cap_id: u64,
        seq: u32,
        caps: u32,
        dirty: u32,
        flush_tid: u64,
        oldest_flush_tid: u64,
    },
    CompleteFlush {
        client: ClientId,
        epoch: u64,
        tid: u64,
    },
    CompleteRequest(CompletedRequest, u64),
    /// Send GRANTs carrying the new `max_size` to these clients.
    ShareMaxSize {
        ino: Ino,
        clients: Vec<ClientId>,
    },
    /// TRUNC to every cap holder and a data truncation for the daemon.
    Truncate {
        ino: Ino,
    },
    Purge(PurgeInode),
    /// MClientSnap SPLIT to every client with caps below the new realm.
    RealmSplit {
        realm: Ino,
    },
    RecoverFileSize {
        ino: Ino,
        max_size: u64,
    },
}

/// What a reply contains; materialized (InodeStats built, caps issued) when
/// it is released, so it reflects committed state.
#[derive(Debug, Clone)]
pub(crate) struct ReplySpec {
    pub client: ClientId,
    pub epoch: u64,
    pub tid: u64,
    pub op: u32,
    pub result: i32,
    pub dentry: Option<(Ino, Vec<u8>)>,
    pub target: Option<Ino>,
    pub mask: u32,
    pub extra: ReplyExtra,
    pub created_ino: Option<Ino>,
}

/// A transaction under construction.
#[derive(Debug)]
pub(crate) struct Txn {
    pub rec: JournalRecord,
    /// Inodes changed directly: other requests wait for the commit.
    pub touched: BTreeSet<Ino>,
    /// Inodes whose image goes into the record.
    pub imaged: BTreeSet<Ino>,
    pub deferred: Vec<Deferred>,
}

impl Txn {
    pub fn new(kind: RecordKind) -> Txn {
        Txn {
            rec: JournalRecord::new(kind),
            touched: BTreeSet::new(),
            imaged: BTreeSet::new(),
            deferred: Vec::new(),
        }
    }
}

/// A submitted record waiting for commit.
#[derive(Debug)]
struct PendingCommit {
    seq: JournalSeq,
    touched: Vec<Ino>,
    holds: Vec<(Ino, u64)>,
    deferred: Vec<Deferred>,
    /// (client, tid) of the request, removed from the in-flight set at commit.
    request: Option<(ClientId, u64)>,
}

/// The engine of one shard.
pub struct Engine<J: Journal> {
    pub(crate) config: EngineConfig,
    pub(crate) journal: J,
    pub(crate) inodes: BTreeMap<Ino, Inode>,
    pub(crate) sessions: BTreeMap<ClientId, Session>,
    pub(crate) ino_alloc: InoAllocator,
    pub(crate) last_cap_id: u64,
    next_hold_id: u64,
    parked: VecDeque<Pending>,
    commits: VecDeque<PendingCommit>,
    /// (client, tid) of requests parked or waiting for commit.
    inflight: BTreeSet<(ClientId, u64)>,
    /// Inodes that have file-lock state (for deadlock detection).
    pub(crate) lock_inodes: BTreeSet<Ino>,
    /// Inodes whose caps must be re-evaluated (a new cap was added).
    pub(crate) eval_queue: BTreeSet<Ino>,
    /// Clients an eviction was requested for (sent once).
    pub(crate) evicting: BTreeSet<ClientId>,
    next_session_epoch: u64,
}

impl<J: Journal> Engine<J> {
    /// Create a new file system: the root directory (`/`, inode 1) with an
    /// explicit default layout and its own snap realm (`MDCache::create_root_inode`),
    /// journaled as the first record.
    pub fn new(config: EngineConfig, journal: J) -> Engine<J> {
        let mut e = Engine::empty(config, journal);
        let mut core = InodeCore::new(
            CEPH_INO_ROOT,
            S_IFDIR | (e.config.root_mode & 0o7777),
            e.config.root_uid,
            e.config.root_gid,
            e.config.mkfs_stamp,
        );
        core.truncate_seq = 0;
        core.layout = e.config.default_layout();
        core.rstat.rsubdirs = 1;
        core.rstat.rctime = e.config.mkfs_stamp;
        core.snaprealm = Some(SnapRealmNode {
            seq: 1,
            created: 1,
            ..Default::default()
        });
        e.inodes.insert(CEPH_INO_ROOT, Inode::new(core));
        let mut rec = JournalRecord::new(RecordKind::Mkfs);
        rec.inodes.push(e.inodes[&CEPH_INO_ROOT].core.clone());
        rec.ino_alloc = Some(e.ino_alloc.state());
        let seq = e.journal.submit(rec);
        let _ = seq;
        e
    }

    fn empty(config: EngineConfig, journal: J) -> Engine<J> {
        let mut ino_alloc = InoAllocator::new();
        ino_alloc
            .add_lease(config.ino_lease)
            .expect("invalid inode lease in EngineConfig");
        Engine {
            config,
            journal,
            inodes: BTreeMap::new(),
            sessions: BTreeMap::new(),
            ino_alloc,
            last_cap_id: 0,
            next_hold_id: 1,
            parked: VecDeque::new(),
            commits: VecDeque::new(),
            inflight: BTreeSet::new(),
            lock_inodes: BTreeSet::new(),
            eval_queue: BTreeSet::new(),
            evicting: BTreeSet::new(),
            next_session_epoch: 0,
        }
    }

    // ------------------------------------------------------------------
    // Accessors

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Update pools from a new MDSMap/OSDMap.
    pub fn set_pools(&mut self, data_pools: Vec<i64>, pool_names: BTreeMap<i64, String>) {
        self.config.data_pools = data_pools;
        self.config.pool_names = pool_names;
    }

    pub fn journal(&self) -> &J {
        &self.journal
    }

    /// Mutable access to the journal (tests commit a manual journal through it;
    /// call [`Engine::journal_committed`] afterwards).
    pub fn journal_mut(&mut self) -> &mut J {
        &mut self.journal
    }

    /// The persistent attributes of an inode.
    pub fn inode(&self, ino: Ino) -> Option<&InodeCore> {
        self.inodes.get(&ino).map(|i| &i.core)
    }

    /// All inode numbers currently known (linked and stray).
    pub fn inode_numbers(&self) -> Vec<Ino> {
        self.inodes.keys().copied().collect()
    }

    /// Inodes unlinked but still alive because clients hold caps on them.
    pub fn strays(&self) -> Vec<Ino> {
        self.inodes
            .values()
            .filter(|i| i.core.nlink == 0)
            .map(|i| i.ino())
            .collect()
    }

    /// Look a name up in a directory (no caps, no waiting; for the daemon and tests).
    pub fn lookup_name(&self, dir: Ino, name: &[u8]) -> Option<Ino> {
        self.inodes.get(&dir)?.lookup(name).map(|d| d.ino)
    }

    /// Resolve a path to an inode (no caps, no waiting).
    pub fn resolve_path(&self, path: &FilePath) -> Result<Ino, Errno> {
        self.resolve(path)
    }

    /// Directory entries in readdir order: (name, ino, primary link?).
    pub fn list_dir(&self, dir: Ino) -> Vec<(Vec<u8>, Ino, bool)> {
        self.inodes
            .get(&dir)
            .map(|d| {
                d.entries
                    .iter()
                    .map(|(k, v)| (k.name.clone(), v.ino, v.primary))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Primary dentry of an inode.
    pub fn parent_of(&self, ino: Ino) -> Option<(Ino, Vec<u8>)> {
        self.inodes.get(&ino)?.parent.clone()
    }

    /// The capability `client` holds on `ino`.
    pub fn cap(&self, ino: Ino, client: ClientId) -> Option<&crate::caps::Capability> {
        self.inodes.get(&ino)?.caps.get(&client)
    }

    /// Caps on an inode by client.
    pub fn caps_of(&self, ino: Ino) -> Vec<(ClientId, crate::caps::Capability)> {
        self.inodes
            .get(&ino)
            .map(|i| i.caps.iter().map(|(c, k)| (*c, k.clone())).collect())
            .unwrap_or_default()
    }

    pub fn session(&self, client: ClientId) -> Option<&Session> {
        self.sessions.get(&client)
    }

    pub fn ino_allocator(&self) -> &InoAllocator {
        &self.ino_alloc
    }

    /// Number of requests waiting (for revocations, commits or file locks).
    pub fn waiting_requests(&self) -> usize {
        self.parked.len()
    }

    /// `(client, tid)` of every request in progress: waiting, or
    /// executed and waiting for its record to commit (for
    /// `dump_ops_in_flight`).
    pub fn requests_in_flight(&self) -> Vec<(ClientId, u64)> {
        self.inflight.iter().copied().collect()
    }

    /// Number of submitted records not yet committed.
    pub fn uncommitted_records(&self) -> usize {
        self.commits.len()
    }

    /// Held fcntl (`flock == false`) or flock locks on an inode.
    pub fn file_locks(&self, ino: Ino, flock: bool) -> Vec<crate::filelock::FileLock> {
        self.inodes
            .get(&ino)
            .map(|i| {
                if flock {
                    i.flock_locks.held().to_vec()
                } else {
                    i.fcntl_locks.held().to_vec()
                }
            })
            .unwrap_or_default()
    }

    // ------------------------------------------------------------------
    // Sessions

    /// A client session opened (or was reclaimed).
    pub fn open_session(&mut self, client: ClientId) -> Vec<Effect> {
        if !self.sessions.contains_key(&client) {
            self.next_session_epoch += 1;
            self.sessions
                .insert(client, Session::with_epoch(client, self.next_session_epoch));
        }
        Vec::new()
    }

    /// Is `client`'s current session the incarnation `epoch`?
    pub(crate) fn session_matches(&self, client: ClientId, epoch: u64) -> bool {
        self.sessions.get(&client).is_some_and(|s| s.epoch == epoch)
    }

    pub(crate) fn session_epoch(&self, client: ClientId) -> u64 {
        self.sessions.get(&client).map(|s| s.epoch).unwrap_or(0)
    }

    // ------------------------------------------------------------------
    // Requests

    /// Handle a client request. Returns the effects, among them the reply
    /// once the request is done (possibly together with replies of other
    /// requests it unblocked). A request that has to wait produces its reply
    /// from a later call.
    pub fn handle_request(&mut self, client: ClientId, req: Request) -> Vec<Effect> {
        let mut out = Vec::new();
        self.dispatch_new(client, req, &mut out);
        self.run_parked(&mut out);
        out
    }

    fn dispatch_new(&mut self, client: ClientId, req: Request, out: &mut Vec<Effect>) {
        let Some(session) = self.sessions.get_mut(&client) else {
            // No session: the C++ drops such requests.
            return;
        };
        if self.inflight.contains(&(client, req.tid)) {
            // A resend of a request still in progress: its reply will come.
            return;
        }
        let mut has_completed = false;
        let mut completed_ino = None;
        if (req.is_replay() || req.retry_attempt > 0)
            && let Some(done) = session.completed_requests.get(&req.tid).cloned()
        {
            let op = req.op.code();
            let is_open = op == CEPH_MDS_OP_OPEN || op == CEPH_MDS_OP_CREATE;
            if req.is_replay() || !is_open {
                out.push(Effect::Reply(Reply {
                    client,
                    tid: req.tid,
                    op,
                    result: 0,
                    dentry: None,
                    target: None,
                    snap_realm: None,
                    snap_trace: Vec::new(),
                    extra: match (op, done.created_ino) {
                        (CEPH_MDS_OP_CREATE, Some(ino)) => ReplyExtra::Create {
                            ino,
                            delegated: Vec::new(),
                        },
                        _ => ReplyExtra::None,
                    },
                    created_ino: done.created_ino,
                }));
                return;
            }
            has_completed = true;
            completed_ino = done.created_ino;
        }
        session.trim_completed_requests(req.oldest_client_tid);
        if !req.is_replay() {
            self.process_request_releases(client, &req, out);
        }
        let epoch = self.session_epoch(client);
        let mut p = Pending {
            client,
            epoch,
            req,
            holds: Vec::new(),
            flock_was_waiting: false,
            has_completed,
            completed_ino,
        };
        if self.config.read_only && p.req.op.may_write() {
            self.reply_error(&mut p, Errno::EROFS, out);
            self.finish_request(&mut p, out);
            return;
        }
        self.inflight.insert((client, p.req.tid));
        match self.execute(&mut p, out) {
            Exec::Done => self.finish_request(&mut p, out),
            Exec::Wait => self.parked.push_back(p),
        }
    }

    /// Run one attempt of a request.
    fn execute(&mut self, p: &mut Pending, out: &mut Vec<Effect>) -> Exec {
        match p.req.op.clone() {
            Op::Lookup { path, mask } => self.op_lookup(p, path, mask, out),
            Op::Getattr { path, mask } => self.op_getattr(p, path, mask, out),
            Op::LookupIno { ino, mask } => self.op_lookupino(p, ino, mask, out),
            Op::LookupParent { ino } => self.op_lookupparent(p, ino, out),
            Op::LookupName { ino, parent } => self.op_lookupname(p, ino, parent, out),
            Op::Setattr { path, args } => self.op_setattr(p, path, args, out),
            Op::Open {
                path,
                flags,
                mode: _,
                mask,
                old_size,
            } => self.op_open(p, path, flags, mask, old_size, None, out),
            Op::Create {
                path,
                flags,
                mode,
                layout,
                xattrs,
                ino: _,
            } => self.op_create(p, path, flags, mode, layout, xattrs, out),
            Op::Mknod {
                path,
                mode,
                rdev,
                xattrs,
                ino: _,
            } => self.op_mknod(p, path, mode, rdev, xattrs, out),
            Op::Mkdir {
                path,
                mode,
                xattrs,
                ino: _,
            } => self.op_mkdir(p, path, mode, xattrs, out),
            Op::Symlink {
                path,
                target,
                xattrs,
                ino: _,
            } => self.op_symlink(p, path, target, xattrs, out),
            Op::Link { path, target } => self.op_link(p, path, target, out),
            Op::Unlink { path } => self.op_unlink(p, path, false, out),
            Op::Rmdir { path } => self.op_unlink(p, path, true, out),
            Op::Rename { dst, src } => self.op_rename(p, dst, src, out),
            Op::Readdir {
                path,
                frag,
                max_entries,
                max_bytes,
                last_name,
                offset_hash,
                flags,
            } => self.op_readdir(
                p,
                path,
                frag,
                max_entries,
                max_bytes,
                last_name,
                offset_hash,
                flags,
                out,
            ),
            Op::Setxattr {
                path,
                name,
                value,
                flags,
            } => self.op_setxattr(p, path, name, value, flags, out),
            Op::Rmxattr { path, name } => self.op_rmxattr(p, path, name, out),
            Op::Getvxattr { path, name } => self.op_getvxattr(p, path, name, out),
            Op::SetFileLock { path, lock } => self.op_setfilelock(p, path, lock, out),
            Op::GetFileLock { path, lock } => self.op_getfilelock(p, path, lock, out),
            Op::LookupSnap { .. } => self.reply_error(p, Errno::ENOENT, out),
            Op::Lssnap { path } => self.op_lssnap(p, path, out),
            Op::Mksnap { .. } => self.reply_error(p, Errno::EPERM, out),
            Op::Other { op } => {
                let e = match op {
                    CEPH_MDS_OP_RMSNAP | CEPH_MDS_OP_RENAMESNAP => Errno::ENOENT,
                    _ => Errno::EOPNOTSUPP,
                };
                self.reply_error(p, e, out)
            }
        }
    }

    /// Clean up after a request is done (replied or handed to a record).
    fn finish_request(&mut self, p: &mut Pending, out: &mut Vec<Effect>) {
        let holds = std::mem::take(&mut p.holds);
        let mut inos: BTreeSet<Ino> = BTreeSet::new();
        for (ino, id) in holds {
            if let Some(i) = self.inodes.get_mut(&ino) {
                i.holds.retain(|h| h.id != id);
                inos.insert(ino);
            }
        }
        // Mutations stay in flight until their record commits (see submit).
        if !self
            .commits
            .iter()
            .any(|c| c.request == Some((p.client, p.req.tid)))
        {
            self.inflight.remove(&(p.client, p.req.tid));
        }
        for ino in inos {
            self.eval_caps(ino, out);
        }
    }

    /// Retry waiting requests until none makes progress.
    fn run_parked(&mut self, out: &mut Vec<Effect>) {
        loop {
            self.drain_evals(out);
            let mut progressed = false;
            let n = self.parked.len();
            for _ in 0..n {
                let Some(mut p) = self.parked.pop_front() else {
                    break;
                };
                if !self.session_matches(p.client, p.epoch) {
                    self.drop_request(&mut p, out);
                    progressed = true;
                    continue;
                }
                match self.execute(&mut p, out) {
                    Exec::Done => {
                        self.finish_request(&mut p, out);
                        progressed = true;
                    }
                    Exec::Wait => self.parked.push_back(p),
                }
            }
            if !progressed && self.eval_queue.is_empty() {
                break;
            }
        }
    }

    /// Forget a request without replying (its session is gone).
    fn drop_request(&mut self, p: &mut Pending, out: &mut Vec<Effect>) {
        // Remove a queued lock wait of this request.
        if let Op::SetFileLock { path, lock } = &p.req.op
            && let Ok(ino) = self.resolve(path)
            && let Some(i) = self.inodes.get_mut(&ino)
        {
            let fl = ops_lock::to_filelock(p.client, lock);
            i.fcntl_locks.remove_waiting(&fl);
            i.flock_locks.remove_waiting(&fl);
        }
        self.finish_request(p, out);
    }

    /// Drop waiting requests of a client.
    fn drop_requests_of(&mut self, client: ClientId, out: &mut Vec<Effect>) {
        let mut keep = VecDeque::new();
        while let Some(mut p) = self.parked.pop_front() {
            if p.client == client {
                self.drop_request(&mut p, out);
            } else {
                keep.push_back(p);
            }
        }
        self.parked = keep;
    }

    // ------------------------------------------------------------------
    // Replies

    /// Reply with an error and no trace.
    pub(crate) fn reply_error(&mut self, p: &mut Pending, e: Errno, out: &mut Vec<Effect>) -> Exec {
        out.push(Effect::Reply(Reply {
            client: p.client,
            tid: p.req.tid,
            op: p.req.op.code(),
            result: e.as_neg(),
            dentry: None,
            target: None,
            snap_realm: None,
            snap_trace: Vec::new(),
            extra: ReplyExtra::None,
            created_ino: None,
        }));
        Exec::Done
    }

    /// Build and send a reply now.
    pub(crate) fn reply_now(&mut self, spec: ReplySpec, out: &mut Vec<Effect>) -> Exec {
        if self.session_matches(spec.client, spec.epoch) {
            let r = self.materialize_reply(spec);
            out.push(Effect::Reply(r));
        }
        Exec::Done
    }

    pub(crate) fn spec(&self, p: &Pending, result: i32) -> ReplySpec {
        ReplySpec {
            client: p.client,
            epoch: p.epoch,
            tid: p.req.tid,
            op: p.req.op.code(),
            result,
            dentry: None,
            target: None,
            mask: 0,
            extra: ReplyExtra::None,
            created_ino: None,
        }
    }

    pub(crate) fn materialize_reply(&mut self, s: ReplySpec) -> Reply {
        let dentry = match &s.dentry {
            Some((dir, name)) if self.inodes.contains_key(dir) => Some(DentryTrace {
                dir: self.inode_stat(*dir, s.client, 0, true),
                dirfrag: self.dirfrag_info(*dir),
                name: name.clone(),
                lease: LeaseStat::default(),
            }),
            _ => None,
        };
        let target = match s.target {
            Some(t) if self.inodes.contains_key(&t) => {
                Some(self.inode_stat(t, s.client, s.mask, true))
            }
            _ => None,
        };
        let snap_realm = match (&target, &dentry) {
            (Some(t), _) => Some(t.cap.realm),
            (None, Some(d)) => Some(d.dir.cap.realm),
            _ => None,
        };
        let snap_trace = snap_realm.map(|r| self.snap_trace(r)).unwrap_or_default();
        Reply {
            client: s.client,
            tid: s.tid,
            op: s.op,
            result: s.result,
            dentry,
            target,
            snap_realm,
            snap_trace,
            extra: s.extra,
            created_ino: s.created_ino,
        }
    }

    pub(crate) fn dirfrag_info(&self, dir: Ino) -> DirFragInfo {
        DirFragInfo {
            frag: 0,
            auth: if dir == CEPH_INO_ROOT { 0 } else { -1 },
            dist: Vec::new(),
        }
    }

    // ------------------------------------------------------------------
    // Waiting on uncommitted records

    /// True if none of `inos` has an uncommitted record; otherwise asks the
    /// journal to commit soon.
    pub(crate) fn none_pending(&mut self, inos: &[Ino]) -> bool {
        let busy = inos
            .iter()
            .any(|i| self.inodes.get(i).is_some_and(|n| n.pending > 0));
        if busy {
            self.journal.flush();
        }
        !busy
    }

    // ------------------------------------------------------------------
    // Holds

    /// Register holds (once per request) and report whether all are
    /// satisfied by the clients' issued caps. Sends REVOKEs as needed.
    pub(crate) fn acquire_holds(
        &mut self,
        p: &mut Pending,
        wanted: &[(Ino, HoldKind)],
        out: &mut Vec<Effect>,
    ) -> bool {
        let mut new_inos = BTreeSet::new();
        for (ino, kind) in wanted {
            let have = p.holds.iter().any(|(hi, id)| {
                *hi == *ino
                    && self
                        .inodes
                        .get(hi)
                        .is_some_and(|i| i.holds.iter().any(|h| h.id == *id && h.kind == *kind))
            });
            if have {
                continue;
            }
            let id = self.next_hold_id;
            self.next_hold_id += 1;
            if let Some(i) = self.inodes.get_mut(ino) {
                i.holds.push(Hold { id, kind: *kind });
                p.holds.push((*ino, id));
                new_inos.insert(*ino);
            }
        }
        for ino in new_inos {
            self.eval_caps(ino, out);
        }
        wanted
            .iter()
            .all(|(ino, kind)| self.hold_satisfied(*ino, *kind))
    }

    /// No client's issued caps violate the hold.
    pub(crate) fn hold_satisfied(&self, ino: Ino, kind: HoldKind) -> bool {
        let Some(i) = self.inodes.get(&ino) else {
            return true;
        };
        let h = Hold { id: 0, kind };
        i.caps
            .iter()
            .all(|(c, cap)| cap.issued() & h.excluded_for(*c, cap.issued()) == 0)
    }

    // ------------------------------------------------------------------
    // Transactions

    /// Mark an inode changed by the transaction: bump its version once,
    /// image it, and make other requests wait for the commit.
    pub(crate) fn touch(&mut self, txn: &mut Txn, ino: Ino) {
        if txn.touched.insert(ino)
            && !txn.imaged.contains(&ino)
            && let Some(i) = self.inodes.get_mut(&ino)
        {
            i.core.version += 1;
        }
        txn.imaged.insert(ino);
    }

    /// Image an inode whose aggregate stats changed (no waiting for others).
    pub(crate) fn image(&mut self, txn: &mut Txn, ino: Ino) {
        if !txn.imaged.contains(&ino)
            && !txn.touched.contains(&ino)
            && let Some(i) = self.inodes.get_mut(&ino)
        {
            i.core.version += 1;
        }
        txn.imaged.insert(ino);
    }

    /// Submit a transaction for the request `p` (or for a cap message when
    /// `p` is `None`), deferring its effects until commit.
    pub(crate) fn submit(
        &mut self,
        txn: Txn,
        p: Option<&mut Pending>,
        out: &mut Vec<Effect>,
    ) -> JournalSeq {
        let Txn {
            mut rec,
            touched,
            imaged,
            deferred,
        } = txn;
        for ino in &imaged {
            if let Some(i) = self.inodes.get(ino) {
                rec.inodes.push(i.core.clone());
            }
        }
        let seq = self.journal.submit(rec);
        let mut touched_live = Vec::new();
        for ino in touched {
            if let Some(i) = self.inodes.get_mut(&ino) {
                i.pending += 1;
                touched_live.push(ino);
            }
        }
        let (holds, request) = match p {
            Some(p) => (std::mem::take(&mut p.holds), Some((p.client, p.req.tid))),
            None => (Vec::new(), None),
        };
        self.commits.push_back(PendingCommit {
            seq,
            touched: touched_live,
            holds,
            deferred,
            request,
        });
        self.process_commits(out);
        seq
    }

    /// Release the effects of every committed record, in order.
    fn process_commits(&mut self, out: &mut Vec<Effect>) {
        let committed = self.journal.committed_seq();
        while self.commits.front().is_some_and(|c| c.seq <= committed) {
            let c = self.commits.pop_front().unwrap();
            let mut eval: BTreeSet<Ino> = BTreeSet::new();
            for ino in &c.touched {
                if let Some(i) = self.inodes.get_mut(ino) {
                    i.pending = i.pending.saturating_sub(1);
                    eval.insert(*ino);
                }
            }
            for (ino, id) in &c.holds {
                if let Some(i) = self.inodes.get_mut(ino) {
                    i.holds.retain(|h| h.id != *id);
                    eval.insert(*ino);
                }
            }
            if let Some(key) = c.request {
                self.inflight.remove(&key);
            }
            for d in c.deferred {
                self.release_deferred(d, out);
            }
            for ino in eval {
                self.eval_caps(ino, out);
                self.maybe_purge_stray(ino, out);
            }
        }
    }

    fn release_deferred(&mut self, d: Deferred, out: &mut Vec<Effect>) {
        match d {
            Deferred::Reply(spec) => {
                if self.session_matches(spec.client, spec.epoch) {
                    let r = self.materialize_reply(spec);
                    out.push(Effect::Reply(r));
                }
            }
            Deferred::FlushAck {
                client,
                epoch,
                ino,
                cap_id,
                seq,
                caps,
                dirty,
                flush_tid,
                oldest_flush_tid,
            } => {
                if !self.session_matches(client, epoch) {
                    return;
                }
                let issue_seq = self
                    .inodes
                    .get(&ino)
                    .and_then(|i| i.caps.get(&client))
                    .map(|c| c.last_issue)
                    .unwrap_or(0);
                out.push(Effect::Caps(CapMessage {
                    client,
                    op: CEPH_CAP_OP_FLUSH_ACK,
                    ino,
                    realm: 0,
                    cap_id,
                    seq,
                    issue_seq,
                    caps,
                    wanted: 0,
                    dirty,
                    mseq: 0,
                    flush_tid,
                    oldest_flush_tid,
                    inode: None,
                }));
            }
            Deferred::CompleteFlush { client, epoch, tid } => {
                if self.session_matches(client, epoch) {
                    self.sessions
                        .get_mut(&client)
                        .unwrap()
                        .completed_flushes
                        .insert(tid);
                }
            }
            Deferred::CompleteRequest(cr, epoch) => {
                if self.session_matches(cr.client, epoch) {
                    self.sessions
                        .get_mut(&cr.client)
                        .unwrap()
                        .completed_requests
                        .insert(cr.tid, cr);
                }
            }
            Deferred::ShareMaxSize { ino, clients } => {
                for c in clients {
                    self.send_max_size_grant(ino, c, out);
                }
            }
            Deferred::Truncate { ino } => self.send_truncate(ino, out),
            Deferred::Purge(pi) => out.push(Effect::PurgeInode(pi)),
            Deferred::RealmSplit { realm } => self.send_realm_split(realm, out),
            Deferred::RecoverFileSize { ino, max_size } => {
                out.push(Effect::RecoverFileSize { ino, max_size })
            }
        }
    }

    /// The journal committed more records: release their effects. Call after
    /// an asynchronous journal reports progress (with [`MemJournal`] in
    /// manual mode, after `commit_upto`).
    ///
    /// [`MemJournal`]: crate::journal::MemJournal
    pub fn journal_committed(&mut self) -> Vec<Effect> {
        let mut out = Vec::new();
        self.process_commits(&mut out);
        self.run_parked(&mut out);
        out
    }

    /// Periodic work: ask the journal to commit what is pending (bounded
    /// durability lag) and retry waiting requests.
    pub fn tick(&mut self) -> Vec<Effect> {
        if !self.commits.is_empty() {
            self.journal.flush();
        }
        let mut out = Vec::new();
        self.process_commits(&mut out);
        self.run_parked(&mut out);
        out
    }

    /// Cache-pressure recall hints for sessions holding more caps than
    /// `max_caps_per_client` (`Server::recall_client_state`, reduced).
    pub fn recall_hints(&self) -> Vec<Effect> {
        let limit = self.config.max_caps_per_client;
        self.sessions
            .values()
            .filter(|s| s.num_caps() as u64 > limit)
            .map(|s| Effect::Recall {
                client: s.client,
                max_caps: limit,
            })
            .collect()
    }

    /// Allocate an inode number.
    pub(crate) fn alloc_ino(&mut self, txn: &mut Txn) -> Result<Ino, Errno> {
        let ino = self.ino_alloc.alloc().ok_or(Errno::ENOSPC)?;
        txn.rec.allocated.push(ino);
        txn.rec.ino_alloc = Some(self.ino_alloc.state());
        Ok(ino)
    }

    /// Record a successful mutation in the completed-request table (in the
    /// same record) and defer adding it to the session until commit.
    pub(crate) fn complete_request(&self, p: &Pending, txn: &mut Txn, created_ino: Option<Ino>) {
        let cr = CompletedRequest {
            client: p.client,
            tid: p.req.tid,
            op: p.req.op.code(),
            result: 0,
            created_ino,
            oldest_client_tid: p.req.oldest_client_tid,
        };
        txn.rec.completed_request = Some(cr.clone());
        txn.deferred.push(Deferred::CompleteRequest(cr, p.epoch));
    }
}

/// `Locker::calc_new_max_size`: the next writeable range for a file of
/// `size` bytes.
pub(crate) fn calc_new_max_size(layout: &FileLayout, size: u64, max_inc_objs: u64) -> u64 {
    let mut new_max = (size + 1) << 1;
    if max_inc_objs > 0 {
        let inc = max_inc_objs.saturating_mul(u64::from(layout.object_size));
        new_max = new_max.min(size.saturating_add(inc));
    }
    let period = layout.period();
    if period == 0 {
        return new_max;
    }
    new_max.div_ceil(period) * period
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_size_rounding() {
        let l = FileLayout::default_for_pool(1);
        assert_eq!(calc_new_max_size(&l, 0, 1024), 4 << 20);
        assert_eq!(calc_new_max_size(&l, 3 << 20, 1024), 8 << 20);
        // Growth is capped at 1024 objects beyond the size.
        let big = 100u64 << 30;
        assert_eq!(calc_new_max_size(&l, big, 1024), big + (4u64 << 30));
    }
}
