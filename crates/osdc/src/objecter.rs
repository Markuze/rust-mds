//! The Objecter: an async RADOS client for the MDS (`src/osdc/Objecter.{h,cc}`).
//!
//! One task owns all state (OSDMap, ops, OSD sessions); the [`Objecter`] handle talks to it
//! through a channel. Behaviour follows the C++ Objecter as the MDS configures it:
//!
//! - **Targets** (`_calc_target`): object → raw pg → actual pg → up/acting sets → the acting
//!   primary; pool tiering (`read_tier`/`write_tier`) unless `IGNORE_OVERLAY`; ops to a pool
//!   with `FLAG_EIO` fail with `PoolEio`; ops to a deleted (or never created) pool fail with
//!   `PoolDoesNotExist` once the map is known to be current (`_check_op_pool_dne`).
//! - **Resends** after every map epoch (`_scan_requests`): the raw pg or the acting primary
//!   changed (`is_pg_changed`), a split/merge of the target pg, the pool's
//!   `last_force_op_resend`, an op becoming unpaused, a skipped map; also after a connection
//!   reset (`_kick_requests`, in tid order). Retries carry `retry_attempt` and the `RETRY`
//!   flag; stale replies (older attempts) are ignored.
//! - **Pausing** (`target_should_be_paused`): reads under `PAUSERD`, writes under `PAUSEWR`
//!   (full flags are ignored: the MDS does not honour them, so every op carries
//!   `FULL_FORCE`), and everything while the map epoch is below the epoch barrier.
//! - **Flags on every op**: `KNOWN_REDIR | SUPPORTSPOOLEIO | ONDISK (| FULL_FORCE)`, plus
//!   `READ`/`WRITE` from the operation (`_prepare_osd_op`).
//! - **Replies** (`handle_osd_op_reply`): redirects and `-EAGAIN` resubmit (with a new tid);
//!   `-EBLOCKLISTED` completes the op with [`OpError::Blocklisted`] and emits
//!   [`ObjecterEvent::Fenced`]: the daemon must respawn.
//! - **Backoffs** (`handle_osd_backoff`): `BLOCK` is acknowledged with `ACK_BLOCK` (same
//!   epoch, priority 63) and ops in the range are held; `UNBLOCK` resends them.
//! - **Liveness** (`tick`): ops older than `laggy_timeout` make the Objecter ping their OSD
//!   (`MPing`) so that a dead lossy connection surfaces; homeless ops request a map.
//! - **Maps** (`handle_osd_map`): incrementals in order, full maps when needed, skipping
//!   below `cluster_osdmap_trim_lower_bound`; [`ObjecterEvent::RequestMap`] asks the caller to
//!   subscribe (onetime unless `PAUSERD`/`PAUSEWR`, like `_maybe_request_map`).
//!
//! The transport is the [`OsdTransport`] trait: open a lossy session to an OSD, send raw
//! messages, report connection events through a [`ConnSink`]. [`crate::msgr_transport`]
//! binds it to the `msgr` crate; tests use a mock.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use denc::{Bytes, EntityAddr, EntityAddrVec, Message, RawMessage, UTime, features};
use tokio::sync::{mpsc, oneshot, watch};
use tracing::{debug, info, trace, warn};

use crate::hash::ceph_stable_mod;
use crate::messages::{
    CEPH_MSG_OSD_BACKOFF, CEPH_MSG_OSD_MAP, CEPH_MSG_OSD_OPREPLY, MOsdBackoff, MOsdMap, MOsdOp,
    MOsdOpReply, OSD_CLIENT_OP_PRIORITY, backoff_op, mping,
};
use crate::ops::{ObjectOperation, OsdOp, flags};
use crate::osdmap::{Incremental, OsdMap, map_flags};
use crate::pool::pool_flags;
use crate::types::{
    CEPH_NOSNAP, HObject, ObjectLocator, OsdReqId, PgId, Release, SnapContext, SpgId,
};

/// `EBLOCKLISTED`.
pub const EBLOCKLISTED: i32 = 108;
const EAGAIN: i32 = 11;

/// A connection to one OSD, as the transport provides it.
pub trait OsdConnection: Send + Sync {
    /// Queue a message (never blocks); false if the connection is closed.
    fn send(&self, msg: RawMessage) -> bool;
    /// Features negotiated with the peer, 0 until the handshake completed.
    fn features(&self) -> u64;
    /// Close the connection and drop its queues.
    fn mark_down(&self);
}

/// Opens connections to OSDs (lossy, authenticated, from the daemon's single messenger
/// identity: fencing by blocklist relies on that).
pub trait OsdTransport: Send + Sync + 'static {
    /// Connect to `osd` at `addrs`. Everything that happens on the connection must be
    /// reported to `sink` (connected, messages received, reset/closed).
    fn connect(&self, osd: i32, addrs: &EntityAddrVec, sink: ConnSink) -> Arc<dyn OsdConnection>;
}

/// Where a transport delivers the events of one connection.
#[derive(Clone)]
pub struct ConnSink {
    tx: mpsc::UnboundedSender<Cmd>,
    conn_id: u64,
}

impl ConnSink {
    /// The handshake completed.
    pub fn connected(&self) {
        let _ = self.tx.send(Cmd::Conn {
            conn_id: self.conn_id,
            ev: ConnEv::Connected,
        });
    }

    /// A message arrived (`MOSDOpReply`, `MOSDBackoff`, `MOSDMap`; others are ignored).
    pub fn message(&self, msg: RawMessage) {
        let _ = self.tx.send(Cmd::Conn {
            conn_id: self.conn_id,
            ev: ConnEv::Message(msg),
        });
    }

    /// The (lossy) session is gone: queued and in-flight messages are lost.
    pub fn reset(&self) {
        let _ = self.tx.send(Cmd::Conn {
            conn_id: self.conn_id,
            ev: ConnEv::Reset,
        });
    }
}

/// Objecter settings.
#[derive(Debug, Clone)]
pub struct ObjecterConfig {
    /// `client_inc`: the MDS uses its MDSMap incarnation, so that `(name, inc, tid)` reqids
    /// of a new instance of a rank never collide with the previous one's.
    pub client_inc: u32,
    /// Honour pool/cluster full flags (librados does; the MDS does not and sends
    /// `FULL_FORCE`).
    pub honor_full: bool,
    /// `osd_client_op_priority`.
    pub op_priority: u16,
    /// `objecter_tick_interval`.
    pub tick_interval: Duration,
    /// `objecter_timeout`: ops older than this are laggy and their OSD is pinged.
    pub laggy_timeout: Duration,
    /// `rados_osd_op_timeout` (`None` = never time out; the MDS default).
    pub op_timeout: Option<Duration>,
    /// Feature bits recorded in each `MOSDOp` (`CEPH_FEATURES_SUPPORTED_DEFAULT`).
    pub features: u64,
    /// The cluster fsid (`monc->get_fsid()`); maps of other clusters are ignored. `None`:
    /// learn it from the first map.
    pub fsid: Option<denc::Uuid>,
}

impl Default for ObjecterConfig {
    fn default() -> Self {
        ObjecterConfig {
            client_inc: 0,
            honor_full: false,
            op_priority: OSD_CLIENT_OP_PRIORITY,
            tick_interval: Duration::from_secs(5),
            laggy_timeout: Duration::from_secs(10),
            op_timeout: None,
            features: features::ALL,
            fsid: None,
        }
    }
}

/// Something the caller must act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjecterEvent {
    /// Subscribe to `osdmap` from `start` (0 = the latest full map), onetime or continuous,
    /// as `MonClient::sub_want("osdmap", start, flags)`.
    RequestMap { start: u32, onetime: bool },
    /// Ask the monitor for the latest OSDMap version (`MonClient::get_version("osdmap")`)
    /// and report it with [`Objecter::set_latest_map_version`]: an op targets a pool that
    /// is not in our map.
    CheckLatestMap,
    /// A new OSDMap epoch is in effect.
    MapApplied { epoch: u32 },
    /// Addresses newly blocklisted by the maps just applied (`consume_blocklist_events`).
    Blocklisted { addrs: Vec<EntityAddr> },
    /// An OSD rejected an op with `-EBLOCKLISTED`: this instance is fenced and must respawn.
    Fenced,
}

/// One op's result in a reply.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpResult {
    pub rval: i32,
    pub outdata: Bytes,
}

/// The OSD's answer to an operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpReply {
    /// 0 or a negative errno (e.g. `-ENOENT`, `-ECANCELED` for a failed `CMPXATTR`).
    pub result: i32,
    /// Per-op results, in op order.
    pub ops: Vec<OpResult>,
    /// The object's user version after the op (`ASSERT_VER` guards compare against it).
    pub user_version: u64,
    /// The OSD's map epoch.
    pub epoch: u32,
}

impl OpReply {
    /// The output of op `i` (empty if absent).
    pub fn out(&self, i: usize) -> Bytes {
        self.ops
            .get(i)
            .map(|o| o.outdata.clone())
            .unwrap_or_default()
    }
}

/// Client-side failures.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpError {
    /// `-EBLOCKLISTED` from an OSD: the daemon has been fenced.
    #[error("blocklisted by the OSDs (fenced)")]
    Blocklisted,
    /// The target pool does not exist (`-ENOENT` in C++).
    #[error("pool does not exist")]
    PoolDoesNotExist,
    /// The target pool has `FLAG_EIO` (`-EIO`).
    #[error("pool has the EIO flag")]
    PoolEio,
    /// `rados_osd_op_timeout` expired (`-ETIMEDOUT`).
    #[error("op timed out")]
    TimedOut,
    /// The Objecter was shut down.
    #[error("objecter shut down")]
    Shutdown,
    /// The OSD returned an error (convenience helpers only; `operate` returns it in
    /// [`OpReply::result`]).
    #[error("rados error {0}")]
    Rados(i32),
}

/// What to execute and where.
#[derive(Debug, Clone)]
pub struct OpRequest {
    pub oid: Bytes,
    pub oloc: ObjectLocator,
    pub op: ObjectOperation,
    /// Snapshot context for writes.
    pub snapc: SnapContext,
    /// Snapshot to read (`CEPH_NOSNAP` = head).
    pub snapid: u64,
    /// Modification time recorded by writes (0 = now).
    pub mtime: UTime,
    /// Extra `CEPH_OSD_FLAG_*` (e.g. `RWORDERED`); `READ`/`WRITE` are derived from the ops.
    pub flags: u32,
    /// Message priority (0 = `op_priority`).
    pub priority: u16,
}

impl OpRequest {
    pub fn new(oloc: ObjectLocator, oid: impl Into<Bytes>, op: ObjectOperation) -> Self {
        OpRequest {
            oid: oid.into(),
            oloc,
            op,
            snapc: SnapContext::default(),
            snapid: CEPH_NOSNAP,
            mtime: UTime::default(),
            flags: 0,
            priority: 0,
        }
    }

    pub fn with_snapc(mut self, snapc: SnapContext) -> Self {
        self.snapc = snapc;
        self
    }

    pub fn with_mtime(mut self, mtime: UTime) -> Self {
        self.mtime = mtime;
        self
    }

    pub fn with_flags(mut self, f: u32) -> Self {
        self.flags |= f;
        self
    }
}

/// Counters for health reporting.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjecterStatus {
    pub epoch: u32,
    pub epoch_barrier: u32,
    pub inflight: usize,
    pub homeless: usize,
    pub paused: usize,
    pub laggy: usize,
    pub sessions: usize,
    pub oldest_op_age: Option<Duration>,
}

// ---------------------------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) enum ConnEv {
    Connected,
    Message(RawMessage),
    Reset,
}

pub(crate) enum Cmd {
    Submit(Box<OpRequest>, oneshot::Sender<Result<OpReply, OpError>>),
    OsdMap(MOsdMap),
    InstallMap(Box<OsdMap>),
    Conn { conn_id: u64, ev: ConnEv },
    SetEpochBarrier(u32),
    LatestMapVersion(u32),
    WaitForMap(u32, oneshot::Sender<u32>),
    SetClientInc(u32),
    Reopen { osd: i32, generation: u64 },
    RequestMap,
    Status(oneshot::Sender<ObjecterStatus>),
    Shutdown,
}

/// `op_target_t`.
#[derive(Debug, Clone)]
struct OpTarget {
    flags: u32,
    base_oid: Bytes,
    base_oloc: ObjectLocator,
    target_oid: Bytes,
    target_oloc: ObjectLocator,
    pool_ever_existed: bool,
    /// Raw pg (full hash); `None` until computed or after `-EAGAIN`.
    pgid: Option<PgId>,
    actual_pgid: SpgId,
    pg_num: u32,
    pg_num_mask: u32,
    pg_num_pending: u32,
    up: Vec<i32>,
    acting: Vec<i32>,
    up_primary: i32,
    acting_primary: i32,
    size: i32,
    min_size: i32,
    sort_bitwise: bool,
    recovery_deletes: bool,
    peering_crush_bucket_count: u32,
    peering_crush_bucket_target: u32,
    peering_crush_bucket_barrier: u32,
    peering_crush_mandatory_member: i32,
    allows_ecoptimizations: bool,
    used_replica: bool,
    paused: bool,
    osd: i32,
    epoch: u32,
    last_force_resend: u32,
}

impl OpTarget {
    fn new(oid: Bytes, oloc: ObjectLocator, flags: u32) -> Self {
        OpTarget {
            flags,
            base_oid: oid.clone(),
            base_oloc: oloc.clone(),
            target_oid: oid,
            target_oloc: oloc,
            pool_ever_existed: false,
            pgid: None,
            actual_pgid: SpgId::default(),
            pg_num: 0,
            pg_num_mask: 0,
            pg_num_pending: 0,
            up: Vec::new(),
            acting: Vec::new(),
            up_primary: -1,
            acting_primary: -1,
            size: -1,
            min_size: -1,
            sort_bitwise: false,
            recovery_deletes: false,
            peering_crush_bucket_count: 0,
            peering_crush_bucket_target: 0,
            peering_crush_bucket_barrier: 0,
            peering_crush_mandatory_member: 0x7fff_ffff,
            allows_ecoptimizations: false,
            used_replica: false,
            paused: false,
            osd: -1,
            epoch: 0,
            last_force_resend: 0,
        }
    }

    /// `get_hobj()`: the target object with the full-precision hash.
    fn hobj(&self) -> HObject {
        let hash = if self.target_oloc.hash >= 0 {
            self.target_oloc.hash as u32
        } else {
            self.pgid.map(|p| p.seed).unwrap_or(0)
        };
        HObject::new(
            self.target_oid.clone(),
            self.target_oloc.key.clone(),
            CEPH_NOSNAP,
            hash,
            self.target_oloc.pool,
            self.target_oloc.nspace.clone(),
        )
    }

    /// `respects_full()`.
    fn respects_full(&self) -> bool {
        self.flags & (flags::WRITE | flags::RWORDERED) != 0
            && self.flags & (flags::FULL_TRY | flags::FULL_FORCE) == 0
    }

    /// `contained_by(begin, end)`.
    fn contained_by(&self, begin: &HObject, end: &HObject) -> bool {
        let h = self.hobj();
        match h.cmp(begin) {
            std::cmp::Ordering::Equal => true,
            std::cmp::Ordering::Greater => h < *end,
            std::cmp::Ordering::Less => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Recalc {
    NoAction,
    NeedResend,
    PoolDne,
    PoolEio,
}

struct Op {
    tid: u64,
    target: OpTarget,
    ops: Vec<OsdOp>,
    snapid: u64,
    snapc: SnapContext,
    mtime: UTime,
    attempts: i32,
    priority: u16,
    /// OSD whose session holds the op; `None` = homeless.
    session: Option<i32>,
    stamp: Instant,
    submitted: Instant,
    map_dne_bound: u32,
    done: Option<oneshot::Sender<Result<OpReply, OpError>>>,
}

#[derive(Debug, Clone)]
struct Backoff {
    id: u64,
    begin: HObject,
    end: HObject,
}

struct Session {
    osd: i32,
    conn_id: u64,
    conn: Arc<dyn OsdConnection>,
    addrs: EntityAddrVec,
    incarnation: u32,
    backoffs: BTreeMap<SpgId, Vec<Backoff>>,
    /// Reconnect throttling: when the last reopen happened and the current delay.
    last_reopen: Option<Instant>,
    reopen_delay: Duration,
    /// A delayed reopen is pending (generation of the request).
    reopen_pending: Option<u64>,
}

/// First delay before re-reconnecting a session that keeps resetting.
const REOPEN_MIN_DELAY: Duration = Duration::from_millis(200);
/// Upper bound (`ms_max_backoff`).
const REOPEN_MAX_DELAY: Duration = Duration::from_secs(15);

/// Handle on the Objecter task. Cheap to clone.
#[derive(Clone)]
pub struct Objecter {
    tx: mpsc::UnboundedSender<Cmd>,
    map: watch::Receiver<Arc<OsdMap>>,
}

impl Objecter {
    /// Start the Objecter task. It has no map until one is fed with
    /// [`handle_osd_map`](Self::handle_osd_map); it asks for one with
    /// [`ObjecterEvent::RequestMap`] right away (`Objecter::start` subscribing).
    pub fn start(
        transport: Arc<dyn OsdTransport>,
        cfg: ObjecterConfig,
    ) -> (Objecter, mpsc::UnboundedReceiver<ObjecterEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let (ev_tx, ev_rx) = mpsc::unbounded_channel();
        let (map_tx, map_rx) = watch::channel(Arc::new(OsdMap::empty()));
        let state = State {
            client_inc: cfg.client_inc,
            cfg,
            transport,
            cmd_tx: tx.clone(),
            events: ev_tx,
            map_tx,
            osdmap: Arc::new(OsdMap::empty()),
            last_tid: 0,
            ops: BTreeMap::new(),
            sessions: HashMap::new(),
            conn_to_osd: HashMap::new(),
            next_conn_id: 1,
            epoch_barrier: 0,
            latest_version: 0,
            waiting_for_map: BTreeMap::new(),
        };
        tokio::spawn(state.run(rx));
        (Objecter { tx, map: map_rx }, ev_rx)
    }

    fn cmd(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }

    /// Submit an operation; resolves when the OSD answers (or on a client-side failure).
    pub fn submit(&self, req: OpRequest) -> oneshot::Receiver<Result<OpReply, OpError>> {
        let (tx, rx) = oneshot::channel();
        if self.tx.send(Cmd::Submit(Box::new(req), tx)).is_err() {
            let (tx, rx) = oneshot::channel();
            let _ = tx.send(Err(OpError::Shutdown));
            return rx;
        }
        rx
    }

    /// Submit and wait.
    pub async fn operate(&self, req: OpRequest) -> Result<OpReply, OpError> {
        self.submit(req).await.unwrap_or(Err(OpError::Shutdown))
    }

    async fn operate_ok(&self, req: OpRequest) -> Result<OpReply, OpError> {
        let r = self.operate(req).await?;
        if r.result < 0 {
            Err(OpError::Rados(r.result))
        } else {
            Ok(r)
        }
    }

    /// `WRITEFULL` (replace the object's data).
    pub async fn write_full(
        &self,
        oloc: ObjectLocator,
        oid: impl Into<Bytes>,
        data: Bytes,
    ) -> Result<OpReply, OpError> {
        let mut op = ObjectOperation::new();
        op.write_full(data);
        self.operate_ok(OpRequest::new(oloc, oid, op).with_mtime(UTime::now()))
            .await
    }

    /// `READ` of `len` bytes (0 = all) at `off`.
    pub async fn read(
        &self,
        oloc: ObjectLocator,
        oid: impl Into<Bytes>,
        off: u64,
        len: u64,
    ) -> Result<Bytes, OpError> {
        let mut op = ObjectOperation::new();
        op.read(off, len);
        Ok(self.operate_ok(OpRequest::new(oloc, oid, op)).await?.out(0))
    }

    /// `STAT`: size and mtime.
    pub async fn stat(
        &self,
        oloc: ObjectLocator,
        oid: impl Into<Bytes>,
    ) -> Result<(u64, UTime), OpError> {
        let mut op = ObjectOperation::new();
        op.stat();
        let r = self.operate_ok(OpRequest::new(oloc, oid, op)).await?;
        crate::ops::decode_stat(&r.out(0)).map_err(|_| OpError::Rados(-5))
    }

    /// `DELETE` with a snap context.
    pub async fn remove(
        &self,
        oloc: ObjectLocator,
        oid: impl Into<Bytes>,
        snapc: SnapContext,
    ) -> Result<OpReply, OpError> {
        let mut op = ObjectOperation::new();
        op.remove();
        self.operate_ok(
            OpRequest::new(oloc, oid, op)
                .with_snapc(snapc)
                .with_mtime(UTime::now()),
        )
        .await
    }

    /// Feed an `MOSDMap` (from the monitor subscription or from an OSD).
    pub fn handle_osd_map(&self, m: MOsdMap) {
        self.cmd(Cmd::OsdMap(m));
    }

    /// Install a decoded map directly (tests and tools; the daemon feeds `MOSDMap`s).
    #[doc(hidden)]
    pub fn install_map(&self, m: OsdMap) {
        self.cmd(Cmd::InstallMap(Box::new(m)));
    }

    /// `set_epoch_barrier`: pause every op until the map reaches `epoch` (only raises it).
    pub fn set_epoch_barrier(&self, epoch: u32) {
        self.cmd(Cmd::SetEpochBarrier(epoch));
    }

    /// Report the monitor's latest OSDMap version (answer to
    /// [`ObjecterEvent::CheckLatestMap`]).
    pub fn set_latest_map_version(&self, v: u32) {
        self.cmd(Cmd::LatestMapVersion(v));
    }

    /// Wait until the map epoch is at least `epoch` (requests maps as needed); returns the
    /// epoch reached.
    pub async fn wait_for_map(&self, epoch: u32) -> u32 {
        let (tx, rx) = oneshot::channel();
        self.cmd(Cmd::WaitForMap(epoch, tx));
        rx.await.unwrap_or(0)
    }

    /// `maybe_request_map`: emit [`ObjecterEvent::RequestMap`] for the next epoch (the MDS
    /// does this after every map so that its subscription is effectively continuous).
    pub fn request_map(&self) {
        self.cmd(Cmd::RequestMap);
    }

    /// Set the client incarnation used for new messages (`set_client_incarnation`).
    pub fn set_client_incarnation(&self, inc: u32) {
        self.cmd(Cmd::SetClientInc(inc));
    }

    /// The current map.
    pub fn osdmap(&self) -> Arc<OsdMap> {
        self.map.borrow().clone()
    }

    /// The current map epoch (0 before the first map).
    pub fn epoch(&self) -> u32 {
        self.map.borrow().epoch
    }

    /// A watch on the current map.
    pub fn map_watch(&self) -> watch::Receiver<Arc<OsdMap>> {
        self.map.clone()
    }

    /// Op and session counters.
    pub async fn status(&self) -> ObjecterStatus {
        let (tx, rx) = oneshot::channel();
        self.cmd(Cmd::Status(tx));
        rx.await.unwrap_or_default()
    }

    /// Stop: pending ops fail with [`OpError::Shutdown`], sessions are closed.
    pub fn shutdown(&self) {
        self.cmd(Cmd::Shutdown);
    }
}

struct State {
    cfg: ObjecterConfig,
    client_inc: u32,
    transport: Arc<dyn OsdTransport>,
    cmd_tx: mpsc::UnboundedSender<Cmd>,
    events: mpsc::UnboundedSender<ObjecterEvent>,
    map_tx: watch::Sender<Arc<OsdMap>>,
    osdmap: Arc<OsdMap>,
    last_tid: u64,
    ops: BTreeMap<u64, Op>,
    sessions: HashMap<i32, Session>,
    conn_to_osd: HashMap<u64, i32>,
    next_conn_id: u64,
    epoch_barrier: u32,
    /// The monitor's latest epoch as last reported (`MOSDMap::newest_map`, get_version).
    latest_version: u32,
    waiting_for_map: BTreeMap<u32, Vec<oneshot::Sender<u32>>>,
}

impl State {
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<Cmd>) {
        // Objecter::start: no map yet, so subscribe for the latest one.
        self.maybe_request_map();
        let mut tick = tokio::time::interval(self.cfg.tick_interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        tick.tick().await;
        loop {
            tokio::select! {
                c = rx.recv() => {
                    let Some(c) = c else { break };
                    if matches!(c, Cmd::Shutdown) {
                        self.shutdown();
                        break;
                    }
                    self.handle(c);
                }
                _ = tick.tick() => self.tick(),
            }
        }
    }

    fn emit(&self, ev: ObjecterEvent) {
        let _ = self.events.send(ev);
    }

    fn handle(&mut self, c: Cmd) {
        match c {
            Cmd::Submit(req, done) => self.submit(*req, done),
            Cmd::OsdMap(m) => self.handle_osd_map(m),
            Cmd::InstallMap(m) => self.install_map(*m),
            Cmd::Conn { conn_id, ev } => self.handle_conn(conn_id, ev),
            Cmd::SetEpochBarrier(e) => {
                if e > self.epoch_barrier {
                    debug!(barrier = e, epoch = self.osdmap.epoch, "set_epoch_barrier");
                    self.epoch_barrier = e;
                    self.maybe_request_map();
                }
            }
            Cmd::LatestMapVersion(v) => {
                self.latest_version = self.latest_version.max(v);
                self.recheck_pool_dne();
            }
            Cmd::WaitForMap(e, tx) => {
                if self.osdmap.epoch >= e {
                    let _ = tx.send(self.osdmap.epoch);
                } else {
                    self.waiting_for_map.entry(e).or_default().push(tx);
                    self.maybe_request_map();
                }
            }
            Cmd::SetClientInc(inc) => self.client_inc = inc,
            Cmd::RequestMap => self.maybe_request_map(),
            Cmd::Reopen { osd, generation } => {
                if self
                    .sessions
                    .get(&osd)
                    .is_some_and(|s| s.reopen_pending == Some(generation))
                {
                    if self.osdmap.is_up(osd) {
                        self.reopen_session(osd);
                    } else {
                        self.close_session(osd);
                        let orphans: Vec<u64> = self
                            .ops
                            .values()
                            .filter(|o| o.session == Some(osd))
                            .map(|o| o.tid)
                            .collect();
                        for tid in orphans {
                            self.ops.get_mut(&tid).unwrap().session = None;
                            self.op_submit(tid);
                        }
                    }
                }
            }
            Cmd::Status(tx) => {
                let _ = tx.send(self.status());
            }
            Cmd::Shutdown => {}
        }
    }

    fn status(&self) -> ObjecterStatus {
        let now = Instant::now();
        ObjecterStatus {
            epoch: self.osdmap.epoch,
            epoch_barrier: self.epoch_barrier,
            inflight: self.ops.len(),
            homeless: self.ops.values().filter(|o| o.session.is_none()).count(),
            paused: self.ops.values().filter(|o| o.target.paused).count(),
            laggy: self
                .ops
                .values()
                .filter(|o| o.session.is_some() && now - o.stamp > self.cfg.laggy_timeout)
                .count(),
            sessions: self.sessions.len(),
            oldest_op_age: self.ops.values().map(|o| now - o.submitted).max(),
        }
    }

    fn shutdown(&mut self) {
        for (_, mut op) in std::mem::take(&mut self.ops) {
            if let Some(d) = op.done.take() {
                let _ = d.send(Err(OpError::Shutdown));
            }
        }
        for (_, s) in self.sessions.drain() {
            s.conn.mark_down();
        }
        self.conn_to_osd.clear();
        self.waiting_for_map.clear();
    }

    // ---- maps ----

    /// `_maybe_request_map`.
    fn maybe_request_map(&self) {
        let continuous = self.osdmap.test_flag(map_flags::PAUSERD)
            || self.osdmap.test_flag(map_flags::PAUSEWR)
            || self.osdmap_full_flag();
        let start = if self.osdmap.epoch != 0 {
            self.osdmap.epoch + 1
        } else {
            0
        };
        self.emit(ObjecterEvent::RequestMap {
            start,
            onetime: !continuous,
        });
    }

    /// `_osdmap_full_flag`: the cluster FULL flag, only when honoured.
    fn osdmap_full_flag(&self) -> bool {
        self.cfg.honor_full && self.osdmap.test_flag(map_flags::FULL)
    }

    fn publish_map(&self) {
        let _ = self.map_tx.send(self.osdmap.clone());
    }

    fn install_map(&mut self, m: OsdMap) {
        let old = std::mem::replace(&mut self.osdmap, Arc::new(m));
        let added = blocklist_diff(&old, &self.osdmap);
        let mut need_resend = BTreeSet::new();
        self.scan_requests(false, &mut need_resend);
        self.close_stale_sessions(&mut need_resend);
        self.finish_map_update(old.epoch, need_resend, added);
    }

    fn handle_osd_map(&mut self, m: MOsdMap) {
        let fsid = if self.osdmap.epoch != 0 {
            Some(self.osdmap.fsid)
        } else {
            self.cfg.fsid
        };
        if fsid.is_some_and(|f| f != m.fsid) {
            warn!(fsid = ?m.fsid, "ignoring osdmap for another cluster");
            return;
        }
        let prev_epoch = self.osdmap.epoch;
        if m.newest_map > self.latest_version {
            self.latest_version = m.newest_map;
        }
        let last = m.last();
        let mut need_resend = BTreeSet::new();
        let mut blocklisted = Vec::new();
        if last <= self.osdmap.epoch {
            debug!(
                first = m.first(),
                last,
                epoch = self.osdmap.epoch,
                "ignoring old osdmap epochs"
            );
        } else if self.osdmap.epoch != 0 {
            let mut skipped_map = false;
            let mut e = self.osdmap.epoch + 1;
            while e <= last {
                if self.osdmap.epoch == e - 1
                    && let Some(b) = m.incremental_maps.get(&e)
                {
                    match Incremental::decode(b) {
                        Ok(inc) => {
                            let map = Arc::make_mut(&mut self.osdmap);
                            if let Err(err) = map.apply_incremental(&inc) {
                                warn!(epoch = e, "cannot apply incremental osdmap: {err}");
                                self.maybe_request_map();
                                break;
                            }
                            blocklisted.extend(inc.new_blocklist_addrs().copied());
                        }
                        Err(err) => {
                            warn!(epoch = e, "undecodable incremental osdmap: {err}");
                            self.maybe_request_map();
                            break;
                        }
                    }
                } else if let Some(b) = m.maps.get(&e) {
                    match OsdMap::decode(b) {
                        Ok(full) => {
                            let full = Arc::new(full);
                            blocklisted.extend(blocklist_diff(&self.osdmap, &full));
                            self.osdmap = full;
                        }
                        Err(err) => {
                            warn!(epoch = e, "undecodable full osdmap: {err}");
                            self.maybe_request_map();
                            break;
                        }
                    }
                } else if e >= m.cluster_osdmap_trim_lower_bound {
                    debug!(missing = e, "requesting missing osdmap epoch");
                    self.maybe_request_map();
                    break;
                } else {
                    debug!(
                        missing = e,
                        jump = m.cluster_osdmap_trim_lower_bound,
                        "missing epoch below the trim bound, skipping"
                    );
                    e = m.cluster_osdmap_trim_lower_bound;
                    skipped_map = true;
                    continue;
                }
                // check all outstanding requests on every epoch
                let removed = self.osdmap.new_removed_snaps.clone();
                for tid in need_resend.iter() {
                    if let Some(op) = self.ops.get_mut(tid) {
                        prune_snapc(&removed, op);
                    }
                }
                self.scan_requests(skipped_map, &mut need_resend);
                self.close_stale_sessions(&mut need_resend);
                e += 1;
            }
        } else if let Some(b) = m.maps.get(&last) {
            // first map: we want the full thing
            match OsdMap::decode(b) {
                Ok(full) => {
                    self.osdmap = Arc::new(full);
                    self.scan_requests(false, &mut need_resend);
                }
                Err(err) => {
                    warn!(epoch = last, "undecodable full osdmap: {err}");
                    self.maybe_request_map();
                }
            }
        } else {
            debug!("no map yet and the message has no full map; requesting one");
            self.maybe_request_map();
        }
        self.finish_map_update(prev_epoch, need_resend, blocklisted);
    }

    /// Common tail of map handling: refresh targets of ops to resend, resend them, wake
    /// waiters, publish the map, emit events.
    fn finish_map_update(
        &mut self,
        prev_epoch: u32,
        need_resend: BTreeSet<u64>,
        blocklisted: Vec<EntityAddr>,
    ) {
        let epoch = self.osdmap.epoch;
        // make sure need_resend targets reflect the latest map
        let mut dne = Vec::new();
        for tid in &need_resend {
            if let Some(op) = self.ops.get_mut(tid)
                && op.target.epoch < epoch
            {
                let mut t = op.target.clone();
                let r = calc_target(&self.osdmap, &self.cfg, self.epoch_barrier, &mut t, false);
                self.ops.get_mut(tid).unwrap().target = t;
                if r == Recalc::PoolDne {
                    dne.push(*tid);
                }
            }
        }
        let mut need_resend = need_resend;
        for tid in dne {
            need_resend.remove(&tid);
            self.check_op_pool_dne(tid);
        }
        let paused = self.osdmap.test_flag(map_flags::PAUSERD)
            || self.osdmap.test_flag(map_flags::PAUSEWR)
            || self.osdmap_full_flag();
        if paused || epoch < self.epoch_barrier || !self.waiting_for_map.is_empty() {
            self.maybe_request_map();
        }
        // resend requests
        for tid in need_resend {
            let Some(op) = self.ops.get(&tid) else {
                continue;
            };
            let osd = op.target.osd;
            let paused = op.target.paused;
            let session = self.get_session(osd);
            if let Some(op) = self.ops.get_mut(&tid) {
                op.session = session;
            }
            if session.is_some() && !paused {
                self.send_op(tid);
            }
        }
        // ops that point at missing pools may now be concluded
        self.recheck_pool_dne();
        // finish any waiters
        let ready: Vec<u32> = self
            .waiting_for_map
            .range(..=epoch)
            .map(|(e, _)| *e)
            .collect();
        for e in ready {
            for tx in self.waiting_for_map.remove(&e).unwrap_or_default() {
                let _ = tx.send(epoch);
            }
        }
        if !self.waiting_for_map.is_empty() {
            self.maybe_request_map();
        }
        self.publish_map();
        if !blocklisted.is_empty() {
            self.emit(ObjecterEvent::Blocklisted { addrs: blocklisted });
        }
        if epoch != prev_epoch {
            self.emit(ObjecterEvent::MapApplied { epoch });
        }
    }

    /// `_scan_requests` over every op (homeless included).
    fn scan_requests(&mut self, skipped_map: bool, need_resend: &mut BTreeSet<u64>) {
        let tids: Vec<u64> = self
            .ops
            .keys()
            .copied()
            .filter(|t| !need_resend.contains(t))
            .collect();
        let removed = self.osdmap.new_removed_snaps.clone();
        let mut dne = Vec::new();
        let mut eio = Vec::new();
        for tid in tids {
            let op = self.ops.get_mut(&tid).unwrap();
            prune_snapc(&removed, op);
            let r = calc_target(
                &self.osdmap,
                &self.cfg,
                self.epoch_barrier,
                &mut op.target,
                false,
            );
            match r {
                Recalc::NoAction if !skipped_map => {}
                Recalc::NoAction | Recalc::NeedResend => {
                    op.session = None;
                    need_resend.insert(tid);
                }
                Recalc::PoolDne => dne.push(tid),
                Recalc::PoolEio => eio.push(tid),
            }
        }
        for tid in dne {
            self.check_op_pool_dne(tid);
        }
        for tid in eio {
            self.finish_op(tid, Err(OpError::PoolEio));
        }
    }

    /// Close sessions whose OSD went down or changed address; their ops are resent.
    fn close_stale_sessions(&mut self, need_resend: &mut BTreeSet<u64>) {
        let stale: Vec<i32> = self
            .sessions
            .values()
            .filter(|s| !self.osdmap.is_up(s.osd) || self.osdmap.osd_addrs(s.osd) != Some(&s.addrs))
            .map(|s| s.osd)
            .collect();
        for osd in stale {
            self.close_session(osd);
            for op in self.ops.values_mut() {
                if op.session == Some(osd) {
                    op.session = None;
                    need_resend.insert(op.tid);
                }
            }
        }
    }

    fn close_session(&mut self, osd: i32) {
        if let Some(s) = self.sessions.remove(&osd) {
            debug!(osd, "closing osd session");
            s.conn.mark_down();
            self.conn_to_osd.remove(&s.conn_id);
        }
    }

    /// `_get_session`: the session for `osd` (opened on demand), `None` = homeless.
    fn get_session(&mut self, osd: i32) -> Option<i32> {
        if osd < 0 {
            return None;
        }
        if self.sessions.contains_key(&osd) {
            return Some(osd);
        }
        let addrs = self.osdmap.osd_addrs(osd).cloned().unwrap_or_default();
        let conn_id = self.next_conn_id;
        self.next_conn_id += 1;
        debug!(osd, addrs = ?addrs, "opening osd session");
        let conn = self.transport.connect(
            osd,
            &addrs,
            ConnSink {
                tx: self.cmd_tx.clone(),
                conn_id,
            },
        );
        self.conn_to_osd.insert(conn_id, osd);
        self.sessions.insert(
            osd,
            Session {
                osd,
                conn_id,
                conn,
                addrs,
                incarnation: 0,
                backoffs: BTreeMap::new(),
                last_reopen: None,
                reopen_delay: Duration::ZERO,
                reopen_pending: None,
            },
        );
        Some(osd)
    }

    /// `_reopen_session` + `_kick_requests` after a connection reset.
    fn reopen_session(&mut self, osd: i32) {
        let addrs = self.osdmap.osd_addrs(osd).cloned().unwrap_or_default();
        let conn_id = self.next_conn_id;
        self.next_conn_id += 1;
        let conn = self.transport.connect(
            osd,
            &addrs,
            ConnSink {
                tx: self.cmd_tx.clone(),
                conn_id,
            },
        );
        if let Some(s) = self.sessions.get_mut(&osd) {
            s.conn.mark_down();
            self.conn_to_osd.remove(&s.conn_id);
            s.conn = conn;
            s.conn_id = conn_id;
            s.addrs = addrs;
            s.incarnation += 1;
            s.backoffs.clear();
            s.last_reopen = Some(Instant::now());
            s.reopen_pending = None;
        }
        self.conn_to_osd.insert(conn_id, osd);
        // resend in tid order
        let resend: Vec<u64> = self
            .ops
            .values()
            .filter(|o| o.session == Some(osd) && !o.target.paused)
            .map(|o| o.tid)
            .collect();
        for tid in resend {
            self.send_op(tid);
        }
    }

    // ---- ops ----

    fn submit(&mut self, mut req: OpRequest, done: oneshot::Sender<Result<OpReply, OpError>>) {
        let rw = if req.op.has_write() {
            flags::WRITE
        } else {
            flags::READ
        };
        if rw == flags::WRITE && req.mtime == UTime::default() {
            // the C++ callers always pass real_clock::now() for mutations
            req.mtime = UTime::now();
        }
        let target = OpTarget::new(req.oid, req.oloc, req.flags | rw);
        self.last_tid += 1;
        let tid = self.last_tid;
        let now = Instant::now();
        let op = Op {
            tid,
            target,
            ops: req.op.ops,
            snapid: req.snapid,
            snapc: req.snapc,
            mtime: req.mtime,
            attempts: 0,
            priority: if req.priority != 0 {
                req.priority
            } else {
                self.cfg.op_priority
            },
            session: None,
            stamp: now,
            submitted: now,
            map_dne_bound: 0,
            done: Some(done),
        };
        self.ops.insert(tid, op);
        self.op_submit(tid);
    }

    /// `_op_submit`.
    fn op_submit(&mut self, tid: u64) {
        let Some(op) = self.ops.get_mut(&tid) else {
            return;
        };
        let r = calc_target(
            &self.osdmap,
            &self.cfg,
            self.epoch_barrier,
            &mut op.target,
            false,
        );
        if r == Recalc::PoolEio {
            self.finish_op(tid, Err(OpError::PoolEio));
            return;
        }
        let osd = op.target.osd;
        let paused = op.target.paused;
        let session = if r == Recalc::PoolDne {
            None
        } else {
            self.get_session(osd)
        };
        let op = self.ops.get_mut(&tid).unwrap();
        op.session = session;
        trace!(tid, osd, paused, "op_submit");
        if r == Recalc::PoolDne {
            self.check_op_pool_dne(tid);
            return;
        }
        if paused || session.is_none() {
            self.maybe_request_map();
        } else {
            self.send_op(tid);
        }
    }

    /// `_check_op_pool_dne`: conclude that the pool does not exist once our map is at least
    /// the monitor's latest epoch (C++ asks `get_version`; here also `MOSDMap::newest_map`).
    fn check_op_pool_dne(&mut self, tid: u64) {
        let epoch = self.osdmap.epoch;
        let latest = self.latest_version;
        let Some(op) = self.ops.get_mut(&tid) else {
            return;
        };
        if op.target.pool_ever_existed {
            // the pool previously existed and now it does not: it was deleted
            op.map_dne_bound = epoch;
        } else if op.map_dne_bound == 0 && latest > 0 && epoch > 0 {
            op.map_dne_bound = latest;
        }
        if op.map_dne_bound > 0 && epoch >= op.map_dne_bound {
            debug!(
                tid,
                pool = op.target.base_oloc.pool,
                "concluding pool does not exist"
            );
            self.finish_op(tid, Err(OpError::PoolDoesNotExist));
        } else {
            op.session = None;
            self.emit(ObjecterEvent::CheckLatestMap);
            self.maybe_request_map();
        }
    }

    /// Re-run the pool-DNE check for ops waiting on it.
    fn recheck_pool_dne(&mut self) {
        let tids: Vec<u64> = self
            .ops
            .values()
            .filter(|o| o.session.is_none() && self.osdmap.pool(o.target.base_oloc.pool).is_none())
            .map(|o| o.tid)
            .collect();
        let epoch = self.osdmap.epoch;
        let latest = self.latest_version;
        for tid in tids {
            let op = self.ops.get_mut(&tid).unwrap();
            if op.map_dne_bound == 0 && latest > 0 && epoch > 0 {
                op.map_dne_bound = latest;
            }
            if op.map_dne_bound > 0 && epoch >= op.map_dne_bound {
                self.finish_op(tid, Err(OpError::PoolDoesNotExist));
            }
        }
    }

    /// `_send_op`: hold the op if a backoff covers it, else build and send the `MOSDOp`.
    fn send_op(&mut self, tid: u64) {
        let Some(op) = self.ops.get(&tid) else { return };
        let Some(osd) = op.session else { return };
        let Some(session) = self.sessions.get(&osd) else {
            return;
        };
        // backoff?
        if let Some(bs) = session.backoffs.get(&op.target.actual_pgid) {
            let h = op.target.hobj();
            if let Some(b) = bs
                .iter()
                .find(|b| h == b.begin || (h > b.begin && h < b.end))
            {
                debug!(tid, osd, backoff = b.id, "op held by backoff");
                return;
            }
        }
        // _prepare_osd_op
        let mut f = op.target.flags | flags::KNOWN_REDIR | flags::SUPPORTSPOOLEIO | flags::ONDISK;
        if !self.cfg.honor_full {
            f |= flags::FULL_FORCE;
        }
        let attempt = op.attempts;
        // set_retry_attempt
        if attempt > 0 {
            f |= flags::RETRY;
        } else {
            f &= !flags::RETRY;
        }
        let mut hobj = op.target.hobj();
        hobj.snap = op.snapid;
        let m = MOsdOp {
            tid,
            client_inc: self.client_inc,
            osdmap_epoch: self.osdmap.epoch,
            flags: f,
            mtime: op.mtime,
            retry_attempt: attempt,
            hobj,
            pgid: op.target.actual_pgid,
            ops: op.ops.clone(),
            snap_seq: op.snapc.seq,
            snaps: op.snapc.snaps.clone(),
            features: self.cfg.features,
            // MOSDOp's constructor puts client_inc into reqid.inc; with no explicit name/tid the
            // OSD identifies the request as (message source, inc, tid), which keeps the reqids of
            // successive instances of a rank (tids restart at 1) distinct
            reqid: OsdReqId {
                inc: self.client_inc as i32,
                ..OsdReqId::default()
            },
            priority: op.priority,
        };
        // Encoding by the connection's features; before the handshake, by the features the
        // OSD advertises in the map.
        let mut feat = session.conn.features();
        if feat == 0 {
            feat = self
                .osdmap
                .osd_xinfo
                .get(osd as usize)
                .map(|x| x.features)
                .unwrap_or(0);
            if feat == 0 {
                feat = self.cfg.features;
            }
            feat &= self.cfg.features;
        }
        let Some(raw) = m.to_raw_for(feat) else {
            warn!(
                osd,
                features = format!("{feat:#x}"),
                "osd too old for MOSDOp v8; op not sent"
            );
            return;
        };
        trace!(tid, osd, attempt, flags = format!("{f:#x}"), pg = %m.pgid, "send_op");
        session.conn.send(raw);
        let op = self.ops.get_mut(&tid).unwrap();
        op.attempts += 1;
        op.stamp = Instant::now();
    }

    fn finish_op(&mut self, tid: u64, r: Result<OpReply, OpError>) {
        if let Some(mut op) = self.ops.remove(&tid)
            && let Some(d) = op.done.take()
        {
            let _ = d.send(r);
        }
    }

    // ---- connections ----

    fn handle_conn(&mut self, conn_id: u64, ev: ConnEv) {
        let Some(&osd) = self.conn_to_osd.get(&conn_id) else {
            trace!(conn_id, "event for a closed session");
            return;
        };
        match ev {
            ConnEv::Connected => debug!(osd, "osd session connected"),
            ConnEv::Reset => {
                // ms_handle_reset: reopen unless the osd went down meanwhile
                if self.osdmap.is_up(osd) {
                    let now = Instant::now();
                    let s = self.sessions.get_mut(&osd).unwrap();
                    // a session that resets right after being reopened is retried with an
                    // exponential delay instead of spinning
                    let recent = s
                        .last_reopen
                        .is_some_and(|t| now - t < Duration::from_secs(1));
                    s.reopen_delay = if recent {
                        (s.reopen_delay * 2).clamp(REOPEN_MIN_DELAY, REOPEN_MAX_DELAY)
                    } else {
                        Duration::ZERO
                    };
                    if s.reopen_delay.is_zero() {
                        info!(osd, "osd session reset; reopening and resending");
                        self.reopen_session(osd);
                    } else {
                        let generation = conn_id;
                        s.reopen_pending = Some(generation);
                        let delay = s.reopen_delay;
                        debug!(osd, ?delay, "osd session reset again; delaying reconnect");
                        let tx = self.cmd_tx.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(delay).await;
                            let _ = tx.send(Cmd::Reopen { osd, generation });
                        });
                    }
                    self.maybe_request_map();
                } else {
                    // the map already says it is down: requeue its ops on the current map
                    self.close_session(osd);
                    let orphans: Vec<u64> = self
                        .ops
                        .values()
                        .filter(|o| o.session == Some(osd))
                        .map(|o| o.tid)
                        .collect();
                    for tid in orphans {
                        self.ops.get_mut(&tid).unwrap().session = None;
                        self.op_submit(tid);
                    }
                }
            }
            ConnEv::Message(m) => match m.msg_type {
                CEPH_MSG_OSD_OPREPLY => match MOsdOpReply::decode_raw(&m) {
                    Ok(r) => self.handle_op_reply(osd, r),
                    Err(e) => warn!(osd, "undecodable MOSDOpReply: {e}"),
                },
                CEPH_MSG_OSD_BACKOFF => match MOsdBackoff::from_raw(m) {
                    Ok(b) => self.handle_backoff(osd, b),
                    Err(e) => warn!(osd, "undecodable MOSDBackoff: {e}"),
                },
                CEPH_MSG_OSD_MAP => match MOsdMap::from_raw(m) {
                    Ok(map) => self.handle_osd_map(map),
                    Err(e) => warn!(osd, "undecodable MOSDMap: {e}"),
                },
                other => trace!(osd, msg_type = other, "ignoring message from osd"),
            },
        }
    }

    /// `handle_osd_op_reply`.
    fn handle_op_reply(&mut self, osd: i32, r: MOsdOpReply) {
        let tid = r.tid;
        let Some(op) = self.ops.get_mut(&tid) else {
            trace!(tid, osd, "stray op reply");
            return;
        };
        if op.session != Some(osd) {
            trace!(tid, osd, "reply from an osd that no longer has the op");
            return;
        }
        if r.retry_attempt >= 0 && r.retry_attempt != op.attempts - 1 {
            debug!(
                tid,
                got = r.retry_attempt,
                last = op.attempts - 1,
                "ignoring reply from an older attempt"
            );
            return;
        }
        if let Some(redir) = &r.redirect {
            debug!(tid, "redirect reply; resubmitting");
            let mut op = self.ops.remove(&tid).unwrap();
            redir.combine_with_locator(&mut op.target.target_oloc, &mut op.target.target_oid);
            op.target.flags |= flags::REDIRECTED | flags::IGNORE_CACHE | flags::IGNORE_OVERLAY;
            self.resubmit_with_new_tid(op);
            return;
        }
        if r.result == -EAGAIN && op.target.flags & flags::FAIL_ON_EAGAIN == 0 {
            debug!(tid, "-EAGAIN; resubmitting");
            let mut op = self.ops.remove(&tid).unwrap();
            op.target.flags &= !(flags::BALANCE_READS
                | flags::LOCALIZE_READS
                | flags::EC_DIRECT_READ
                | flags::FORCE_OSD);
            op.target.pgid = None;
            self.resubmit_with_new_tid(op);
            return;
        }
        if r.result == -EBLOCKLISTED {
            warn!(tid, osd, "osd reports that we are blocklisted");
            self.emit(ObjecterEvent::Fenced);
            self.finish_op(tid, Err(OpError::Blocklisted));
            return;
        }
        let reply = OpReply {
            result: r.result,
            ops: r
                .ops
                .into_iter()
                .map(|o| OpResult {
                    rval: o.rval,
                    outdata: o.outdata,
                })
                .collect(),
            user_version: r.user_version,
            epoch: r.osdmap_epoch,
        };
        self.finish_op(tid, Ok(reply));
    }

    /// Resubmit (`op->tid = 0; _op_submit`): a new tid.
    fn resubmit_with_new_tid(&mut self, mut op: Op) {
        self.last_tid += 1;
        op.tid = self.last_tid;
        op.session = None;
        let tid = op.tid;
        self.ops.insert(tid, op);
        self.op_submit(tid);
    }

    /// `handle_osd_backoff`.
    fn handle_backoff(&mut self, osd: i32, m: MOsdBackoff) {
        let Some(s) = self.sessions.get_mut(&osd) else {
            return;
        };
        match m.op {
            backoff_op::BLOCK => {
                debug!(osd, pg = %m.pgid, id = m.id, "backoff block");
                let list = s.backoffs.entry(m.pgid).or_default();
                list.retain(|b| b.begin != m.begin);
                list.push(Backoff {
                    id: m.id,
                    begin: m.begin.clone(),
                    end: m.end.clone(),
                });
                // ack with the original backoff's epoch so that the osd can discard this if
                // there was a pg split
                let ack = MOsdBackoff {
                    pgid: m.pgid,
                    map_epoch: m.map_epoch,
                    op: backoff_op::ACK_BLOCK,
                    id: m.id,
                    begin: m.begin,
                    end: m.end,
                };
                let f = s.conn.features();
                s.conn
                    .send(ack.to_raw(if f == 0 { self.cfg.features } else { f }));
            }
            backoff_op::UNBLOCK => {
                let mut found = None;
                for (pg, list) in s.backoffs.iter_mut() {
                    if let Some(i) = list.iter().position(|b| b.id == m.id) {
                        let b = list.remove(i);
                        if b.begin != m.begin && b.end != m.end {
                            warn!(
                                osd,
                                id = m.id,
                                "unblock range differs from the backoff's; unblocking anyway"
                            );
                        }
                        found = Some(*pg);
                        break;
                    }
                }
                let Some(pg) = found else {
                    warn!(osd, id = m.id, "unblock for an unknown backoff");
                    return;
                };
                if s.backoffs.get(&pg).is_some_and(|l| l.is_empty()) {
                    s.backoffs.remove(&pg);
                }
                debug!(osd, pg = %m.pgid, id = m.id, "backoff unblock");
                let resend: Vec<u64> = self
                    .ops
                    .values()
                    .filter(|o| {
                        o.session == Some(osd)
                            && o.target.actual_pgid == m.pgid
                            && o.target.contained_by(&m.begin, &m.end)
                    })
                    .map(|o| o.tid)
                    .collect();
                for tid in resend {
                    self.send_op(tid);
                }
            }
            other => debug!(osd, op = other, "unrecognized backoff op"),
        }
    }

    // ---- tick ----

    fn tick(&mut self) {
        let now = Instant::now();
        // op timeouts
        if let Some(to) = self.cfg.op_timeout {
            let expired: Vec<u64> = self
                .ops
                .values()
                .filter(|o| now - o.submitted > to)
                .map(|o| o.tid)
                .collect();
            for tid in expired {
                self.finish_op(tid, Err(OpError::TimedOut));
            }
        }
        // look for laggy requests and ping their osds
        let mut toping = BTreeSet::new();
        let mut homeless = 0;
        for op in self.ops.values() {
            match op.session {
                Some(osd) if now - op.stamp > self.cfg.laggy_timeout => {
                    debug!(tid = op.tid, osd, "op is laggy");
                    toping.insert(osd);
                }
                Some(_) => {}
                None => homeless += 1,
            }
        }
        if homeless > 0 || !toping.is_empty() {
            self.maybe_request_map();
        }
        for osd in toping {
            if let Some(s) = self.sessions.get(&osd) {
                s.conn.send(mping());
            }
        }
    }
}

/// `_prune_snapc`: drop snaps of the op's snap context that the map just removed.
fn prune_snapc(removed: &BTreeMap<i64, crate::osdmap::IntervalSet>, op: &mut Op) {
    let Some(set) = removed.get(&op.target.base_oloc.pool) else {
        return;
    };
    let contains = |s: u64| {
        set.range(..=s)
            .next_back()
            .is_some_and(|(&start, &len)| s < start + len)
    };
    if op.snapc.snaps.iter().any(|&s| contains(s)) {
        op.snapc.snaps.retain(|&s| !contains(s));
    }
}

/// Addresses (and ranges) in `new`'s blocklist that `old` did not have.
fn blocklist_diff(old: &OsdMap, new: &OsdMap) -> Vec<EntityAddr> {
    let mut v: Vec<EntityAddr> = new
        .blocklist
        .keys()
        .filter(|a| !old.blocklist.contains_key(a))
        .copied()
        .collect();
    v.extend(
        new.range_blocklist
            .keys()
            .filter(|a| !old.range_blocklist.contains_key(a))
            .copied(),
    );
    v
}

/// `target_should_be_paused`.
fn target_should_be_paused(map: &OsdMap, cfg: &ObjecterConfig, barrier: u32, t: &OpTarget) -> bool {
    let pauserd = map.test_flag(map_flags::PAUSERD);
    let full = cfg.honor_full
        && (map.test_flag(map_flags::FULL)
            || map
                .pool(t.base_oloc.pool)
                .is_some_and(|p| p.has_flag(pool_flags::FULL)));
    let pausewr = map.test_flag(map_flags::PAUSEWR) || (t.respects_full() && full);
    (t.flags & flags::READ != 0 && pauserd)
        || (t.flags & flags::WRITE != 0 && pausewr)
        || map.epoch < barrier
}

/// `calc_pg_role_broken`.
fn pg_role(osd: i32, acting: &[i32]) -> i32 {
    acting
        .iter()
        .position(|&a| a == osd)
        .map(|p| p as i32)
        .unwrap_or(-1)
}

/// `OSDMap::primary_changed_broken`.
fn primary_changed(oldprimary: i32, oldacting: &[i32], newprimary: i32, newacting: &[i32]) -> bool {
    if oldacting.is_empty() && newacting.is_empty() {
        return false;
    }
    if oldacting.is_empty() != newacting.is_empty() {
        return true;
    }
    if oldprimary != newprimary {
        return true;
    }
    pg_role(oldprimary, oldacting) != pg_role(newprimary, newacting)
}

/// `Objecter::is_pg_changed`.
fn is_pg_changed(
    oldprimary: i32,
    oldacting: &[i32],
    newprimary: i32,
    newacting: &[i32],
    any_change: bool,
) -> bool {
    primary_changed(oldprimary, oldacting, newprimary, newacting)
        || (any_change && oldacting != newacting)
}

/// `Objecter::_calc_target` (without balanced/localized replica reads, which the MDS does not
/// use).
fn calc_target(
    map: &OsdMap,
    cfg: &ObjecterConfig,
    barrier: u32,
    t: &mut OpTarget,
    any_change: bool,
) -> Recalc {
    let is_read = t.flags & flags::READ != 0;
    let is_write = t.flags & flags::WRITE != 0;
    t.epoch = map.epoch;
    let Some(mut pi) = map.pool(t.base_oloc.pool) else {
        t.osd = -1;
        return Recalc::PoolDne;
    };
    if pi.has_flag(pool_flags::EIO) {
        return Recalc::PoolEio;
    }
    let mut force_resend = false;
    if map.epoch == pi.last_force_op_resend {
        if t.last_force_resend < pi.last_force_op_resend {
            t.last_force_resend = pi.last_force_op_resend;
            force_resend = true;
        } else if t.last_force_resend == 0 {
            force_resend = true;
        }
    }
    // apply tiering
    t.target_oid = t.base_oid.clone();
    t.target_oloc = t.base_oloc.clone();
    if t.flags & flags::IGNORE_OVERLAY == 0 {
        if is_read && pi.has_read_tier() {
            t.target_oloc.pool = pi.read_tier;
        }
        if is_write && pi.has_write_tier() {
            t.target_oloc.pool = pi.write_tier;
        }
        match map.pool(t.target_oloc.pool) {
            Some(p) => pi = p,
            None => {
                t.osd = -1;
                return Recalc::PoolDne;
            }
        }
    }
    let Some(pgid) = map.object_locator_to_pg(&t.target_oid, &t.target_oloc) else {
        t.osd = -1;
        return Recalc::PoolDne;
    };
    t.pool_ever_existed = true;

    let size = i32::from(pi.size);
    let min_size = i32::from(pi.min_size);
    let pg_num = pi.pg_num;
    let pg_num_mask = pi.pg_num_mask;
    let pg_num_pending = pi.pg_num_pending;
    let actual_ps = ceph_stable_mod(pgid.seed, pg_num, pg_num_mask);
    let actual_pgid = PgId::new(pgid.pool, actual_ps);
    let ua = map.pg_to_up_acting_osds(actual_pgid);
    let sort_bitwise = map.test_flag(map_flags::SORTBITWISE);
    let recovery_deletes = map.test_flag(map_flags::RECOVERY_DELETES);
    let prev_seed = ceph_stable_mod(pgid.seed, t.pg_num, t.pg_num_mask);
    let prev_pgid = PgId::new(pgid.pool, prev_seed);
    if any_change
        && is_new_interval(
            t,
            &ua,
            size,
            min_size,
            pg_num,
            pg_num_pending,
            sort_bitwise,
            recovery_deletes,
            pi,
            prev_pgid,
        )
    {
        force_resend = true;
    }

    let should_be_paused = target_should_be_paused(map, cfg, barrier, t);
    let unpaused = t.paused && !should_be_paused;
    t.paused = should_be_paused;

    let legacy_change = t.pgid != Some(pgid)
        || is_pg_changed(
            t.acting_primary,
            &t.acting,
            ua.acting_primary,
            &ua.acting,
            t.used_replica || any_change,
        );
    let split_or_merge = t.pg_num != 0
        && (prev_pgid.is_split(t.pg_num, pg_num)
            || prev_pgid.is_merge_source(t.pg_num, pg_num)
            || prev_pgid.is_merge_target(t.pg_num, pg_num));

    if legacy_change || split_or_merge || force_resend {
        t.pgid = Some(pgid);
        t.acting = ua.acting.clone();
        t.acting_primary = ua.acting_primary;
        t.up_primary = ua.up_primary;
        t.up = ua.up.clone();
        t.size = size;
        t.min_size = min_size;
        t.pg_num = pg_num;
        t.pg_num_mask = pg_num_mask;
        t.pg_num_pending = pg_num_pending;
        let mut spgid = SpgId::new(actual_pgid);
        if pi.is_erasure() {
            // the shard is the primary's position (in primaryfirst order with pg_temp)
            for (i, &o) in t.acting.iter().enumerate() {
                if o == ua.acting_primary {
                    spgid.shard = i as i8;
                    break;
                }
            }
        }
        t.actual_pgid = spgid;
        t.sort_bitwise = sort_bitwise;
        t.recovery_deletes = recovery_deletes;
        t.peering_crush_bucket_count = pi.peering_crush_bucket_count;
        t.peering_crush_bucket_target = pi.peering_crush_bucket_target;
        t.peering_crush_bucket_barrier = pi.peering_crush_bucket_barrier;
        t.peering_crush_mandatory_member = pi.peering_crush_mandatory_member;
        t.allows_ecoptimizations = pi.allows_ecoptimizations();
        t.used_replica = false;
        t.osd = ua.acting_primary;
    }
    if legacy_change || unpaused || force_resend {
        return Recalc::NeedResend;
    }
    if split_or_merge
        && (map.require_osd_release.ge(Release::LUMINOUS)
            || map
                .osd_xinfo
                .get(ua.acting_primary as usize)
                .is_some_and(|x| x.features & features::RESEND_ON_SPLIT != 0))
    {
        return Recalc::NeedResend;
    }
    Recalc::NoAction
}

/// `PastIntervals::is_new_interval` (used only with `any_change`, i.e. not for plain ops).
#[allow(clippy::too_many_arguments)]
fn is_new_interval(
    t: &OpTarget,
    ua: &crate::osdmap::UpActing,
    size: i32,
    min_size: i32,
    pg_num: u32,
    pg_num_pending: u32,
    sort_bitwise: bool,
    recovery_deletes: bool,
    pi: &crate::pool::PgPool,
    pgid: PgId,
) -> bool {
    t.acting_primary != ua.acting_primary
        || ua.acting != t.acting
        || t.up_primary != ua.up_primary
        || ua.up != t.up
        || t.min_size != min_size
        || t.size != size
        || pgid.is_split(t.pg_num, pg_num)
        || pgid.is_merge_source(t.pg_num_pending, pg_num_pending)
        || pgid.is_merge_source(pg_num_pending, t.pg_num_pending)
        || pgid.is_merge_source(t.pg_num, pg_num)
        || pgid.is_merge_target(t.pg_num_pending, pg_num_pending)
        || pgid.is_merge_target(pg_num_pending, t.pg_num_pending)
        || pgid.is_merge_target(t.pg_num, pg_num)
        || t.sort_bitwise != sort_bitwise
        || t.recovery_deletes != recovery_deletes
        || t.peering_crush_bucket_count != pi.peering_crush_bucket_count
        || t.peering_crush_bucket_target != pi.peering_crush_bucket_target
        || t.peering_crush_bucket_barrier != pi.peering_crush_bucket_barrier
        || t.peering_crush_mandatory_member != pi.peering_crush_mandatory_member
        || t.allows_ecoptimizations != pi.allows_ecoptimizations()
}
