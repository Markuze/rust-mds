//! The daemon: cluster membership (monitor session, beacons, the rank state machine) and the
//! routing of client messages to the session layer and the metadata server.
//!
//! One task runs the loop below. Reading connections is separate (the messenger delivers
//! events on a bounded channel), and every message to a client goes through its session's
//! funnel ([`crate::session`]). The metadata server is created when the rank is assigned.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use auth::{AuthContext, AuthName, Keyring};
use denc::{EntityName, EntityType, Message, Uuid};
use monc::{
    AddrsString, Beacon, BeaconConfig, MdsMap, MdsState, MonClient, MonClientConfig, MonEvent,
};
use msgr::{Connection, Event, Messenger};
use osdc::messages::MOsdMap;
use osdc::msgr_transport::MsgrTransport;
use osdc::objecter::{Objecter, ObjecterConfig, ObjecterEvent};
use proto::ClientMessage;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::fencing::{self, ClusterFence, Eviction, Membership};
use crate::persistence::{self, Authority};
use crate::server::{RecoveryFence, Server, WireDispatch};
use crate::session::{ClientLink, MAX_SESSIONS, SessionMap};

/// The Ceph version string reported in `sys_info`. Rook compares it with the image's
/// `ceph --version` (F-ADM-116); it matches the dev cluster's packages for now.
const CEPH_VERSION: &str = "ceph version 20.2.0 (69f84cc2651aa259a15bc192ddaabd3baba07489) tentacle (stable - RelWithDebInfo)";
const CEPH_VERSION_SHORT: &str = "20.2.0";
const MAX_HELD_MESSAGES: usize = 1024;
const MAX_HELD_BYTES: usize = 16 << 20;
const MAX_FENCES: usize = MAX_SESSIONS;
const MAX_CONTROL_WORKERS: usize = 8;
const CLIENT_TURN_BUDGET: usize = 32;
const CONTROL_RETRY: Duration = Duration::from_secs(1);

enum ControlEvent {
    RecoveryFence {
        membership: Membership,
        operation: u64,
        target: RecoveryFence,
        result: Result<u32, String>,
    },
    Job {
        owner: store::OwnerToken,
        operation: u64,
        id: u64,
        result: Result<engine::LifecycleResult, String>,
    },
    Fence {
        membership: Membership,
        operation: u64,
        request: Eviction,
        result: Result<u32, String>,
    },
    Latest {
        membership: Option<Membership>,
        operation: u64,
        result: Result<u32, String>,
    },
}

struct PendingFence {
    request: Eviction,
    membership: Membership,
    operation: u64,
    active: bool,
    retry_at: Instant,
    proof: Option<u32>,
}
struct PendingRecoveryFence {
    target: RecoveryFence,
    operation: u64,
    active: bool,
    retry_at: Instant,
    proof: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Waiting,
    Opening,
    Loading,
    Reconnect,
    Rejoin,
    ClientReplay,
    Serving,
}
struct JobAttempt {
    operation: u64,
    active: bool,
    submitted: bool,
    retry_at: Instant,
    failures: u32,
}

fn sys_info(hostname: &str, addr: &str) -> BTreeMap<String, String> {
    let read = |p: &str| {
        std::fs::read_to_string(p)
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let mut m = BTreeMap::new();
    m.insert("addr".into(), addr.to_string());
    m.insert("hostname".into(), hostname.to_string());
    m.insert("ceph_version".into(), CEPH_VERSION.into());
    m.insert("ceph_version_short".into(), CEPH_VERSION_SHORT.into());
    m.insert("ceph_release".into(), "tentacle".into());
    m.insert("os".into(), "Linux".into());
    m.insert("arch".into(), std::env::consts::ARCH.into());
    m.insert("kernel_version".into(), read("/proc/sys/kernel/osrelease"));
    m.insert(
        "kernel_description".into(),
        read("/proc/sys/kernel/version"),
    );
    m.insert(
        "mds_impl".into(),
        format!("rust-mds {}", env!("CARGO_PKG_VERSION")),
    );
    m
}

/// Bind the messenger: the configured address, or the monitors' IP with the first free port of
/// `ms_bind_port_min..=ms_bind_port_max`.
async fn bind_messenger(
    cfg: &Config,
    mons: &[SocketAddr],
    auth: std::sync::Arc<AuthContext>,
) -> Result<(Messenger, tokio::sync::mpsc::Receiver<Event>)> {
    let ip = cfg
        .public_addr
        .map(|a| a.ip())
        .or_else(|| mons.first().map(|m| m.ip()))
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    let ports: Vec<u16> = match cfg.public_addr.map(|a| a.port()).filter(|p| *p != 0) {
        Some(p) => vec![p],
        None => (cfg.bind_port_min..=cfg.bind_port_max).collect(),
    };
    let mut last_err = None;
    for port in ports {
        let addr = SocketAddr::new(ip, port);
        match Messenger::bind(EntityName::new(EntityType::MDS, -1), addr, auth.clone()).await {
            Ok(m) => return Ok(m),
            Err(e) => last_err = Some(e),
        }
    }
    Err(anyhow!(
        "cannot bind on {ip} in ports {}..={}: {:?}",
        cfg.bind_port_min,
        cfg.bind_port_max,
        last_err
    ))
}

pub async fn run(cfg: Config) -> Result<()> {
    let mons = monc::parse_mon_host_v1(&cfg.mon_host);
    if mons.is_empty() {
        bail!(
            "no msgr1 (v1) monitor address in mon_host = {}",
            cfg.mon_host
        );
    }
    let keyring = Keyring::load(&cfg.keyring)
        .map_err(|e| anyhow!("keyring {}: {e}", cfg.keyring.display()))?;
    let name = AuthName::new(EntityType::MDS, cfg.id.clone());
    let key = keyring
        .key(&name)
        .with_context(|| format!("no key for {name} in {}", cfg.keyring.display()))?
        .clone();
    let auth = AuthContext::new(
        name.clone(),
        key,
        auth::ENTITY_MON | auth::ENTITY_OSD | auth::ENTITY_MDS | auth::ENTITY_MGR,
    );

    let (msgr, mut msgr_events) = bind_messenger(&cfg, &mons, auth.clone()).await?;
    info!(addr = %msgr.my_addrs_string(), "bound");
    // A client must present a decodable mds cap (an empty cap blob is refused, as in C++).
    msgr.set_caps_validator(|v| v.caps.allow_all || v.caps.cap_string().is_some());

    let (monc, mut mon_events) = MonClient::start(msgr.clone(), MonClientConfig::new(mons));
    let gid = monc
        .wait_authenticated(Duration::from_secs(60))
        .await
        .map_err(|e| anyhow!("authentication: {e}"))?;
    info!(%name, gid, "authenticated to the monitor");
    monc.wait_rotating(Duration::from_secs(30))
        .await
        .map_err(|e| anyhow!("rotating secrets: {e}"))?;
    monc.subscribe("mdsmap", 0, 0);
    monc.subscribe("fsmap", 0, 0);

    // The OSD client shares the daemon's messenger and identity (one address to fence);
    // its maps arrive through the monitor subscription (`MonEvent::Message`).
    let obj_cfg = ObjecterConfig {
        fsid: monc.fsid(),
        op_timeout: Some(cfg.persistence.op_timeout),
        ..ObjecterConfig::default()
    };
    let (objecter, mut obj_events) = Objecter::start(MsgrTransport::new(msgr.clone()), obj_cfg);

    let (evict_tx, mut evict_rx) = mpsc::channel(256);
    let (control_tx, mut control_rx) = mpsc::channel(64);
    let (storage_events, mut storage_rx) = mpsc::channel(64);
    let (authority, authority_rx) = tokio::sync::watch::channel(Authority {
        membership: None,
        state: MdsState::NULL,
        healthy: false,
    });

    let fsid = match monc
        .fsid()
        .or_else(|| cfg.conf.get(&cfg.id, "fsid").and_then(Uuid::parse))
    {
        Some(f) => f,
        None => {
            tokio::time::sleep(Duration::from_secs(2)).await;
            monc.fsid().context("fsid unknown")?
        }
    };
    let mut bcfg = BeaconConfig::new(cfg.id.clone(), fsid);
    bcfg.sys_info = sys_info(&monc::monclient::short_hostname(), &msgr.my_addrs_string());
    bcfg.fs = cfg.join_fs.clone().unwrap_or_default();
    let beacon = Beacon::start(monc.clone(), bcfg);
    info!("beaconing up:boot");

    let mut d = Daemon {
        msgr: msgr.clone(),
        monc: monc.clone(),
        beacon: beacon.clone(),
        fsid,
        last_epoch: 0,
        state: MdsState::NULL,
        seen_in_map: false,
        mdsmap: None,
        sessions: SessionMap::default(),
        server: None,
        held: VecDeque::new(),
        held_bytes: 0,
        objecter,
        evict_tx,
        control_tx,
        membership: None,
        fences: HashMap::new(),
        recovery_fences: BTreeMap::new(),
        next_operation: 0,
        latest_needed: false,
        latest_active: None,
        latest_retry_at: Instant::now(),
        fatal_error: None,
        phase: Phase::Waiting,
        filesystems: Vec::new(),
        persistence_config: cfg.persistence.clone(),
        storage_events,
        storage_tx: None,
        startup_operation: None,
        authority,
        authority_rx,
        jobs: HashMap::new(),
        reconnect_held: VecDeque::new(),
        reconnect_bytes: 0,
        reconnect_confirmed_at: None,
        #[cfg(test)]
        test_storage_rx: None,
        #[cfg(test)]
        beacon_lag_override: None,
        #[cfg(test)]
        test_global_id: None,
    };

    let mut tick = tokio::time::interval(Duration::from_secs(1));
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut checkpoint_signal =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())?;
    loop {
        let reconnect_deadline = d.server.as_ref().and_then(Server::reconnect_deadline);
        tokio::select! {
            _ = async { match reconnect_deadline { Some(deadline) => tokio::time::sleep_until(deadline.into()).await, None => std::future::pending::<()>().await } } => {
                if let Some(server) = &mut d.server { server.expire_reconnect(&mut d.sessions, Instant::now()); }
            }
            ev = mon_events.recv() => {
                let ev = ev.ok_or_else(|| anyhow!("monitor client stopped"))?;
                d.on_mon_event(ev)?;
            }
            ev = obj_events.recv() => {
                let ev = ev.ok_or_else(|| anyhow!("objecter stopped"))?;
                d.on_objecter_event(ev)?;
            }
            Some(request) = evict_rx.recv() => d.queue_eviction(request)?,
            Some(event) = control_rx.recv() => d.on_control_event(event),
            Some(event) = storage_rx.recv() => d.on_storage_event(event)?,
            ev = msgr_events.recv() => {
                let ev = ev.ok_or_else(|| anyhow!("messenger stopped"))?;
                d.on_msgr_event(ev);
            }
            _ = tick.tick() => d.on_tick(),
            _ = tokio::task::yield_now(), if d.runnable() => {}
            _ = checkpoint_signal.recv() => { if let Some(server) = &mut d.server { server.request_checkpoint(); } }
            _ = tokio::signal::ctrl_c() => { d.shutdown().await; return Ok(()); }
            _ = sigterm.recv() => { d.shutdown().await; return Ok(()); }
        }
        d.check_failure()?;
        d.refresh_authority();
        d.maybe_start_storage()?;
        if let Some(server) = &mut d.server {
            server.pump(&mut d.sessions, CLIENT_TURN_BUDGET);
        }
        d.drive_phase()?;
        d.drive_control();
        d.drive_jobs();
        d.check_failure()?;
        let serving = d.is_serving();
        let data = d.data_work_allowed();
        let execution = d.control_allowed();
        d.sessions.pause_output(!d.output_allowed());
        if let Some(server) = &mut d.server {
            server.set_runtime_permissions(serving, data, execution);
            server.resume_data_work(&mut d.sessions, CLIENT_TURN_BUDGET);
        }
        d.check_failure()?;
        d.sessions.drain_output(CLIENT_TURN_BUDGET);
        d.check_failure()?;
        d.drive_phase()?;
        d.release_held();
        d.check_failure()?;
    }
}

struct Daemon {
    msgr: Messenger,
    monc: MonClient,
    beacon: Beacon,
    fsid: Uuid,
    last_epoch: u32,
    state: MdsState,
    seen_in_map: bool,
    mdsmap: Option<MdsMap>,
    sessions: SessionMap,
    server: Option<Server>,
    /// Client messages that arrived before the rank could serve them.
    held: VecDeque<(Connection, denc::RawMessage, Instant)>,
    held_bytes: usize,
    /// The OSD client of this daemon (RADOS work, fencing detection).
    objecter: Objecter,
    /// Evictions in flight: the blocklist task answers on the daemon loop's channel.
    evict_tx: mpsc::Sender<Eviction>,
    control_tx: mpsc::Sender<ControlEvent>,
    membership: Option<Membership>,
    fences: HashMap<u64, PendingFence>,
    recovery_fences: BTreeMap<engine::SessionKey, PendingRecoveryFence>,
    next_operation: u64,
    latest_needed: bool,
    latest_active: Option<u64>,
    latest_retry_at: Instant,
    fatal_error: Option<String>,
    phase: Phase,
    filesystems: Vec<(u64, MdsMap)>,
    persistence_config: crate::config::PersistenceConfig,
    storage_events: mpsc::Sender<persistence::Event>,
    storage_tx: Option<mpsc::Sender<persistence::Command>>,
    startup_operation: Option<u64>,
    authority: tokio::sync::watch::Sender<Authority>,
    authority_rx: tokio::sync::watch::Receiver<Authority>,
    jobs: HashMap<u64, JobAttempt>,
    reconnect_held: VecDeque<(Connection, denc::RawMessage, Instant)>,
    reconnect_bytes: usize,
    reconnect_confirmed_at: Option<Instant>,
    #[cfg(test)]
    test_storage_rx: Option<mpsc::Receiver<persistence::Event>>,
    /// Fault injection for health transitions without a live monitor.
    #[cfg(test)]
    beacon_lag_override: Option<bool>,
    /// Offline tests replace only the monitor authentication result, not readiness.
    #[cfg(test)]
    test_global_id: Option<u64>,
}

impl Daemon {
    fn refresh_authority(&mut self) {
        let healthy = self.control_allowed();
        self.authority.send_replace(Authority {
            membership: self.membership,
            state: self.state,
            healthy,
        });
        let serving = self.is_serving();
        let data = self.data_work_allowed();
        self.sessions.pause_output(!self.output_allowed());
        if let Some(server) = &mut self.server {
            server.set_runtime_permissions(serving, data, healthy);
            server.allow_recovery_collection(
                healthy
                    && matches!(
                        self.state,
                        MdsState::RECONNECT | MdsState::REJOIN | MdsState::CLIENTREPLAY
                    ),
            );
        }
    }

    fn runnable(&self) -> bool {
        self.check_failure().is_ok()
            && (self.server.as_ref().is_some_and(Server::has_work)
                || (self.output_allowed() && self.sessions.queued_output_bytes() != 0)
                || (self.reconnect_input_allowed() && !self.reconnect_held.is_empty())
                || (self.recovery_input_allowed() && !self.held.is_empty())
                || (self.is_serving()
                    && (self.sessions.queued_output_bytes() != 0
                        || self.held.front().is_some_and(|(_, raw, _)| {
                            self.server.as_ref().is_some_and(|s| {
                                s.input_capacity(raw_bytes(raw), is_control_raw(raw))
                            })
                        }))))
    }

    fn maybe_start_storage(&mut self) -> Result<()> {
        if self.phase != Phase::Waiting
            || !self.has_membership()
            || self.beacon.is_laggy()
            || !matches!(
                self.state,
                MdsState::CREATING
                    | MdsState::STARTING
                    | MdsState::REPLAY
                    | MdsState::RESOLVE
                    | MdsState::RECONNECT
                    | MdsState::REJOIN
                    | MdsState::CLIENTREPLAY
                    | MdsState::ACTIVE
            )
        {
            return Ok(());
        }
        let membership = self.membership.context("storage without membership")?;
        let map = self.mdsmap.as_ref().context("storage without MDSMap")?;
        let matching: Vec<_> = self
            .filesystems
            .iter()
            .filter(|(_, fs)| {
                fs.fs_name == map.fs_name
                    && fs.metadata_pool == map.metadata_pool
                    && fs.up.get(&membership.rank) == Some(&membership.gid)
                    && fs
                        .info(membership.gid)
                        .is_some_and(|i| i.inc == membership.incarnation)
            })
            .collect();
        if matching.is_empty() {
            return Ok(());
        }
        if matching.len() != 1 {
            bail!("ambiguous native filesystem identity");
        }
        let identity = store::StoreIdentity {
            cluster_fsid: self.fsid.0,
            filesystem_id: matching[0].0,
            metadata_pool: map.metadata_pool,
        };
        let rank_owner = persistence::rank_owner(membership)?;
        let writer = store::WriterIdentity {
            rank_owner,
            addresses: self.msgr.my_addrs(),
        };
        let config = engine::EngineConfig {
            data_pools: map.data_pools.clone(),
            pool_names: BTreeMap::new(),
            mkfs_stamp: engine::Timestamp::new(denc::UTime::now().sec, denc::UTime::now().nsec),
            max_record_bytes: self.persistence_config.limits.max_record_bytes
                - store::framing::OVERHEAD,
            ..Default::default()
        };
        self.next_operation += 1;
        let operation = self.next_operation;
        self.startup_operation = Some(operation);
        let (tx, rx) = mpsc::channel(16);
        self.storage_tx = Some(tx);
        let backend = store::RadosBackend::new(self.objecter.clone(), map.metadata_pool);
        let start = persistence::Start {
            identity,
            writer,
            config,
            creating: self.state == MdsState::CREATING,
            failure_epoch: map.last_failure_osd_epoch,
            limits: self.persistence_config.limits.clone(),
            operation,
        };
        let gate = persistence::Gate {
            rx: self.authority_rx.clone(),
            expected: membership,
        };
        let fence = ClusterFence {
            monc: self.monc.clone(),
            objecter: self.objecter.clone(),
        };
        let events = self.storage_events.clone();
        self.phase = Phase::Opening;
        tokio::spawn(persistence::worker(backend, start, gate, fence, rx, events));
        Ok(())
    }

    fn on_storage_event(&mut self, event: persistence::Event) -> Result<()> {
        if self.membership != Some(event.membership) {
            return Ok(());
        }
        self.refresh_authority();
        if let persistence::EventKind::Loaded(loaded) = event.kind {
            if self.startup_operation != Some(event.operation) || self.server.is_some() {
                return Ok(());
            }
            let map = self.mdsmap.as_ref().context("loaded without map")?;
            if loaded.owner.gid != event.membership.gid
                || loaded.owner.incarnation != event.membership.incarnation as u64
                || loaded.identity.cluster_fsid != self.fsid.0
                || loaded.identity.metadata_pool != map.metadata_pool
            {
                bail!("loaded store authority/identity mismatch");
            }
            let config = engine::EngineConfig {
                data_pools: map.data_pools.clone(),
                pool_names: BTreeMap::new(),
                ..Default::default()
            };
            let tx = self
                .storage_tx
                .as_ref()
                .context("loaded without worker")?
                .clone();
            let mut server = Server::from_loaded(
                map.fs_name.clone(),
                map.data_pools.first().copied().unwrap_or(-1),
                map.epoch,
                *loaded,
                config,
                &self.persistence_config.limits,
                tx,
                event.operation,
                self.persistence_config.checkpoint_records,
                self.persistence_config.checkpoint_interval,
            )
            .map_err(|e| anyhow!("checked persistent restore: {e}"))?;
            server.set_objecter(self.objecter.clone());
            server.set_evict_channel(self.evict_tx.clone());
            server.set_mdsmap(map);
            server.raise_epoch_barrier(server.epoch_barrier());
            info!(
                restored_open_sessions = server.gather_len(),
                "checked persistent state loaded; nonempty recovery is collection-only"
            );
            self.server = Some(server);
            self.phase = Phase::Loading;
        } else if let Some(server) = &mut self.server {
            if event.owner == Some(server.owner()) {
                server.on_storage_event(
                    &mut self.sessions,
                    server.owner(),
                    event.operation,
                    event.kind,
                );
            }
        } else if self.startup_operation == Some(event.operation)
            && let persistence::EventKind::Failed(error) = event.kind
        {
            bail!("persistent startup failed closed: {error}");
        }
        Ok(())
    }

    fn request_state(&self, state: MdsState) -> Result<()> {
        if !MdsState::transition_valid(self.state, state) {
            bail!("illegal requested transition {} -> {}", self.state, state);
        }
        if self.beacon.want_state() != state {
            self.beacon.set_want_state(self.last_epoch, state);
            self.beacon.send_now();
        }
        Ok(())
    }

    fn drive_phase(&mut self) -> Result<()> {
        if !self.control_allowed() {
            return Ok(());
        }
        let Some(server) = &mut self.server else {
            return Ok(());
        };
        if !server.recovery_loaded() {
            return Ok(());
        }
        match self.state {
            MdsState::CREATING | MdsState::STARTING => {
                if server.try_finish_recovery(&mut self.sessions) {
                    self.request_state(MdsState::ACTIVE)?;
                } else {
                    self.phase = Phase::Reconnect;
                }
            }
            MdsState::REPLAY | MdsState::RESOLVE => {
                self.phase = Phase::Reconnect;
                self.request_state(MdsState::RECONNECT)?;
            }
            MdsState::RECONNECT => {
                self.phase = Phase::Reconnect;
                server.confirm_reconnect(
                    self.reconnect_confirmed_at.unwrap_or_else(Instant::now),
                    self.persistence_config.reconnect_timeout,
                );
                server.try_finish_recovery(&mut self.sessions);
                if server.recovery_structural_done() {
                    self.phase = Phase::Rejoin;
                    self.request_state(MdsState::REJOIN)?;
                }
            }
            MdsState::REJOIN => {
                server.try_finish_recovery(&mut self.sessions);
                if server.recovery_structural_done() {
                    self.phase = Phase::ClientReplay;
                    let next = if server.recovered()
                        && server.drained()
                        && self.sessions.queued_output_bytes() == 0
                    {
                        MdsState::ACTIVE
                    } else {
                        MdsState::CLIENTREPLAY
                    };
                    self.request_state(next)?;
                }
            }
            MdsState::CLIENTREPLAY => {
                self.phase = Phase::ClientReplay;
                if server.try_finish_recovery(&mut self.sessions)
                    && server.drained()
                    && self.sessions.queued_output_bytes() == 0
                {
                    self.request_state(MdsState::ACTIVE)?;
                }
            }
            MdsState::ACTIVE => {
                // This also permits a monitor-confirmed active map received before a
                // clientless load completed. Readiness still comes from checked load.
                if server.try_finish_recovery(&mut self.sessions) {
                    self.phase = Phase::Serving;
                }
            }
            MdsState::STOPPING => {
                if server.recovered() {
                    self.phase = Phase::Serving;
                }
                if server.drained() && self.sessions.len() == 0 && self.jobs.is_empty() {
                    self.request_state(MdsState::STOPPED)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn drive_jobs(&mut self) {
        if !self.data_work_allowed() {
            return;
        }
        let Some(server) = &self.server else {
            return;
        };
        let owner = server.owner();
        self.jobs.retain(|id, _| server.has_job(*id));
        let mut active = self.jobs.values().filter(|job| job.active).count();
        for intent in server.ready_jobs() {
            if self.jobs.len() >= 1024 && !self.jobs.contains_key(&intent.id) {
                break;
            }
            let attempt = self.jobs.entry(intent.id).or_insert(JobAttempt {
                operation: 0,
                active: false,
                submitted: false,
                retry_at: Instant::now(),
                failures: 0,
            });
            if attempt.active
                || attempt.submitted
                || attempt.retry_at > Instant::now()
                || active >= 4
            {
                continue;
            }
            self.next_operation += 1;
            attempt.operation = self.next_operation;
            attempt.active = true;
            active += 1;
            let (tx, objecter, operation, id) = (
                self.control_tx.clone(),
                self.objecter.clone(),
                attempt.operation,
                intent.id,
            );
            tokio::spawn(async move {
                let result = crate::rados::execute_lifecycle(&objecter, &intent)
                    .await
                    .map_err(|e| e.to_string());
                let _ = tx
                    .send(ControlEvent::Job {
                        owner,
                        operation,
                        id,
                        result,
                    })
                    .await;
            });
        }
    }

    fn check_failure(&self) -> Result<()> {
        // Do not close sessions or release speculative state. Runtime exit stops
        // the shared messenger. A replacement must establish fencing before service.
        if let Some(reason) = &self.fatal_error {
            bail!("metadata service fail-stop: {reason}");
        }
        if let Some(reason) = self.server.as_ref().and_then(Server::fatal_error) {
            bail!("metadata service fail-stop: {reason}");
        }
        if self.sessions.output_failed() {
            bail!("session output unavailable or over budget");
        }
        Ok(())
    }
    fn on_mon_event(&mut self, ev: MonEvent) -> Result<()> {
        match ev {
            MonEvent::Message(raw) if raw.msg_type == 45 => {
                self.filesystems = persistence::decode_filesystems(raw.front)
                    .map_err(|e| anyhow!("native FSMap: {e}"))?;
            }
            MonEvent::Message(raw) if raw.msg_type == monc::messages::CEPH_MSG_OSD_MAP => {
                match MOsdMap::from_raw(raw) {
                    Ok(m) => self.objecter.handle_osd_map(m),
                    Err(e) => warn!("undecodable MOSDMap: {e}"),
                }
            }
            MonEvent::Message(raw) if raw.msg_type == monc::messages::CEPH_MSG_MDS_MAP => {
                let (mm, map) = match monc::decode_mdsmap(raw) {
                    Ok(x) => x,
                    Err(e) => {
                        warn!("undecodable MDSMap: {e}");
                        return Ok(());
                    }
                };
                if mm.fsid != self.fsid {
                    return Ok(());
                }
                if map.epoch <= self.last_epoch && map.epoch != MdsMap::NULL_EPOCH {
                    return Ok(());
                }
                self.on_mdsmap(map)?;
            }
            MonEvent::Message(raw) => debug!(msg_type = raw.msg_type, "monitor message ignored"),
            MonEvent::SessionEstablished { global_id, mon } => {
                if self.membership.is_some_and(|m| m.gid != global_id) {
                    self.beacon.stop();
                    bail!("authenticated daemon identity changed");
                }
                info!(%mon, global_id, "monitor session established")
            }
            MonEvent::SessionLost => warn!("monitor session lost; hunting"),
            MonEvent::MonMap(m) => debug!(epoch = m.epoch, "monmap"),
            MonEvent::Config(c) => debug!(keys = c.len(), "config from the monitor (ignored)"),
        }
        Ok(())
    }

    fn on_mdsmap(&mut self, map: MdsMap) -> Result<()> {
        let gid = self.monc.global_id();
        let new_state = map.state_of(gid);
        if map.epoch == MdsMap::NULL_EPOCH || (self.seen_in_map && new_state == MdsState::NULL) {
            // Removed from the map: a real MDS respawns with a new identity (F-CLU-105).
            error!(epoch = map.epoch, "removed from the MDSMap; exiting");
            self.beacon.stop();
            bail!("removed from the MDSMap");
        }
        self.last_epoch = map.epoch;
        self.monc.sub_got("mdsmap", u64::from(map.epoch));
        if !monc::CompatSet::rust_mds().writeable(&map.compat) {
            warn!(
                mask = format!("{:#x}", map.compat.incompat.mask()),
                "file system compat not writeable by this daemon"
            );
        }
        if new_state != MdsState::NULL {
            self.seen_in_map = true;
        }
        let info = map.info(gid).cloned();
        let membership = info
            .as_ref()
            .filter(|i| i.rank >= 0 && map.up.get(&i.rank) == Some(&gid))
            .map(|i| Membership {
                gid,
                rank: i.rank,
                incarnation: i.inc,
            });
        if self.membership.is_some() && membership != self.membership {
            self.beacon.stop();
            bail!("rank assignment/incarnation changed");
        }
        self.membership = membership;
        let prev = self.state;
        self.state = new_state;
        if new_state == MdsState::RECONNECT && self.reconnect_confirmed_at.is_none() {
            self.reconnect_confirmed_at = Some(Instant::now());
        }
        if prev != new_state {
            info!(epoch = map.epoch, from = %prev, to = %new_state, rank = info.as_ref().map(|i| i.rank).unwrap_or(-1), "state change");
        }
        match new_state {
            MdsState::NULL => self.beacon.notify_mdsmap(map.epoch),
            MdsState::STANDBY => self.beacon.set_want_state(map.epoch, MdsState::STANDBY),
            s => {
                if let Some(i) = &info
                    && i.rank >= 0
                {
                    self.msgr
                        .set_my_name(EntityName::new(EntityType::MDS, i64::from(i.rank)));
                    // The rank's incarnation identifies this instance's OSD ops
                    // (`ObjecterConfig::client_inc`).
                    self.objecter.set_client_incarnation(i.inc.max(0) as u32);
                }
                if let Some(server) = &mut self.server {
                    server.set_mdsmap(&map);
                }
                self.beacon.set_want_state(map.epoch, s);
                if s == MdsState::ACTIVE && prev != s {
                    info!(rank = info.as_ref().map(|i| i.rank).unwrap_or(-1), fs = %map.fs_name, addr = %self.msgr.my_addrs_string(), "active");
                }
            }
        }
        self.mdsmap = Some(map);
        Ok(())
    }

    fn control_allowed(&self) -> bool {
        let laggy = self.beacon.is_laggy();
        #[cfg(test)]
        let laggy = self.beacon_lag_override.unwrap_or(laggy);
        matches!(
            self.state,
            MdsState::CREATING
                | MdsState::STARTING
                | MdsState::REPLAY
                | MdsState::RESOLVE
                | MdsState::RECONNECT
                | MdsState::REJOIN
                | MdsState::CLIENTREPLAY
                | MdsState::ACTIVE
                | MdsState::STOPPING
        ) && self.has_membership()
            && self.server.as_ref().is_none_or(|server| {
                let owner = server.owner();
                self.membership.is_some_and(|membership| {
                    owner.gid == membership.gid
                        && i32::try_from(owner.rank) == Ok(membership.rank)
                        && i32::try_from(owner.incarnation) == Ok(membership.incarnation)
                })
            })
            && !laggy
            && self.check_failure().is_ok()
            && !self
                .msgr
                .my_addrs()
                .0
                .iter()
                .any(|a| self.objecter.osdmap().is_blocklisted(a))
    }
    fn recovery_input_allowed(&self) -> bool {
        self.control_allowed()
            && self.server.as_ref().is_some_and(Server::recovery_pending)
            && (matches!(
                self.state,
                MdsState::RECONNECT | MdsState::REJOIN | MdsState::CLIENTREPLAY | MdsState::ACTIVE
            ) || (matches!(self.state, MdsState::REPLAY | MdsState::RESOLVE)
                && self.beacon.want_state() == MdsState::RECONNECT))
    }
    fn reconnect_input_allowed(&self) -> bool {
        self.control_allowed()
            && self.server.as_ref().is_some_and(|server| {
                server.recovery_loaded()
                    && (!server.recovery_pending() || self.recovery_input_allowed())
            })
    }
    fn output_allowed(&self) -> bool {
        self.control_allowed()
            && (self.is_serving()
                || matches!(
                    self.state,
                    MdsState::RECONNECT | MdsState::REJOIN | MdsState::CLIENTREPLAY
                )
                || (matches!(
                    self.state,
                    MdsState::CREATING | MdsState::STARTING | MdsState::ACTIVE
                ) && self
                    .server
                    .as_ref()
                    .is_some_and(Server::recovery_structural_done)))
    }
    fn data_work_allowed(&self) -> bool {
        self.control_allowed()
            && (self.is_serving()
                || (matches!(
                    self.state,
                    MdsState::CREATING
                        | MdsState::STARTING
                        | MdsState::CLIENTREPLAY
                        | MdsState::ACTIVE
                ) && self
                    .server
                    .as_ref()
                    .is_some_and(Server::recovery_structural_done)))
    }

    /// Ordinary requests require confirmed ACTIVE plus completed recovery.
    fn is_serving(&self) -> bool {
        matches!(self.state, MdsState::ACTIVE | MdsState::STOPPING)
            && self.server.as_ref().is_some_and(Server::recovered)
            && self.phase == Phase::Serving
            && self.control_allowed()
    }

    fn has_membership(&self) -> bool {
        let global_id = self.monc.global_id();
        #[cfg(test)]
        let global_id = self.test_global_id.unwrap_or(global_id);
        self.membership.is_some_and(|m| {
            global_id == m.gid
                && self.mdsmap.as_ref().is_some_and(|map| {
                    map.up.get(&m.rank) == Some(&m.gid)
                        && map.info(m.gid).is_some_and(|i| {
                            i.rank == m.rank && i.inc == m.incarnation && !i.is_laggy()
                        })
                })
        })
    }

    fn release_held(&mut self) {
        if self.reconnect_input_allowed() {
            for _ in 0..CLIENT_TURN_BUDGET {
                let Some((conn, raw, received)) = self.reconnect_held.pop_front() else {
                    break;
                };
                self.reconnect_bytes -= raw_bytes(&raw);
                self.dispatch_admitted(conn, raw, received);
            }
        }
        if self.recovery_input_allowed() {
            // Early requests held before checked load must not prevent reading
            // reconnect behind them. The runtime has separate protected lanes.
            for _ in 0..CLIENT_TURN_BUDGET {
                let Some((conn, raw, received)) = self.held.pop_front() else {
                    break;
                };
                self.held_bytes -= raw_bytes(&raw);
                self.dispatch_admitted(conn, raw, received);
            }
            return;
        }
        for _ in 0..CLIENT_TURN_BUDGET {
            if !self.is_serving() {
                break;
            }
            let eligible = self.held.iter().position(|(_, raw, _)| {
                let control = is_control_raw(raw);
                self.server
                    .as_ref()
                    .is_some_and(|s| s.input_capacity(raw_bytes(raw), control))
                    && (control
                        || self
                            .held
                            .front()
                            .is_some_and(|(_, front, _)| std::ptr::eq(front, raw)))
            });
            let Some(index) = eligible else {
                break;
            };
            let Some((conn, msg, received)) = self.held.remove(index) else {
                break;
            };
            self.held_bytes -= raw_bytes(&msg);
            self.dispatch_admitted(conn, msg, received);
        }
    }

    fn on_objecter_event(&mut self, ev: ObjecterEvent) -> Result<()> {
        match ev {
            ObjecterEvent::RequestMap { start, onetime } => {
                self.monc.subscribe(
                    "osdmap",
                    u64::from(start),
                    if onetime {
                        monc::messages::SUBSCRIBE_ONETIME
                    } else {
                        0
                    },
                );
            }
            ObjecterEvent::MapApplied { epoch } => {
                self.monc.sub_got("osdmap", u64::from(epoch));
                self.check_blocklists()?;
            }
            ObjecterEvent::CheckLatestMap => self.latest_needed = true,
            ObjecterEvent::Blocklisted { .. } => self.check_blocklists()?,
            ObjecterEvent::Fenced => {
                // A later instance of this rank blocklisted us; C++ respawns.
                error!("fenced: this instance is blocklisted; exiting");
                self.beacon.stop();
                self.objecter.shutdown();
                bail!("fenced");
            }
        }
        Ok(())
    }

    fn check_blocklists(&mut self) -> Result<()> {
        let map = self.objecter.osdmap();
        if self
            .msgr
            .my_addrs()
            .0
            .iter()
            .any(|addr| map.is_blocklisted(addr))
        {
            self.beacon.stop();
            self.objecter.shutdown();
            bail!("shared messenger address is blocklisted");
        }
        if let Some(server) = &mut self.server {
            for client in self.sessions.clients() {
                if self
                    .sessions
                    .get(client)
                    .is_some_and(|s| map.is_blocklisted(&s.addr))
                {
                    server.request_eviction(&mut self.sessions, client);
                }
            }
        }
        Ok(())
    }

    fn queue_eviction(&mut self, request: Eviction) -> Result<()> {
        let Some(membership) = self.membership else {
            bail!("client eviction without current assignment");
        };
        if !matches_eviction(&self.sessions, request) {
            return Ok(());
        }
        if let Some(pending) = self.fences.get(&request.client)
            && pending.request == request
        {
            return Ok(());
        }
        if self.fences.len() >= MAX_FENCES {
            bail!("fencing queue capacity exhausted");
        }
        self.next_operation += 1;
        self.fences.insert(
            request.client,
            PendingFence {
                request,
                membership,
                operation: self.next_operation,
                active: false,
                retry_at: Instant::now(),
                proof: None,
            },
        );
        Ok(())
    }

    fn on_control_event(&mut self, event: ControlEvent) {
        match event {
            ControlEvent::RecoveryFence {
                membership,
                operation,
                target,
                result,
            } => {
                if self.membership != Some(membership)
                    || self
                        .server
                        .as_ref()
                        .is_none_or(|s| !s.matches_recovery_fence(&self.sessions, &target))
                {
                    return;
                }
                let Some(p) = self.recovery_fences.get_mut(&target.key) else {
                    return;
                };
                if p.operation != operation || p.target != target || !p.active {
                    return;
                }
                p.active = false;
                match result {
                    Ok(epoch) => p.proof = Some(epoch),
                    Err(error) => {
                        warn!(key = ?target.key, %error, "recovery fence failed; gather/rights retained");
                        p.retry_at = Instant::now() + CONTROL_RETRY;
                    }
                }
            }
            ControlEvent::Job {
                owner,
                operation,
                id,
                result,
            } => {
                let Some(server) = &mut self.server else {
                    return;
                };
                if server.owner() != owner {
                    return;
                }
                let Some(attempt) = self.jobs.get_mut(&id) else {
                    return;
                };
                if !attempt.active || attempt.operation != operation {
                    return;
                }
                attempt.active = false;
                match result {
                    Ok(result) => {
                        attempt.submitted = true;
                        server.lifecycle_completed(id, result);
                    }
                    Err(error) => {
                        attempt.failures = attempt.failures.saturating_add(1).min(6);
                        attempt.retry_at =
                            Instant::now() + Duration::from_secs(1 << attempt.failures);
                        warn!(id, %error, "durable data job retained for retry");
                    }
                }
            }
            ControlEvent::Latest {
                membership,
                operation,
                result,
            } => {
                if self.latest_active != Some(operation) {
                    return;
                }
                self.latest_active = None;
                if membership != self.membership {
                    self.latest_needed = true;
                    return;
                }
                match result {
                    Ok(epoch) if epoch >= self.objecter.epoch() => {
                        self.objecter.set_latest_map_version(epoch);
                        self.latest_needed = false;
                    }
                    Ok(_) => {
                        self.latest_needed = true;
                        self.latest_retry_at = Instant::now() + CONTROL_RETRY;
                    }
                    Err(e) => {
                        warn!(error = %e, "latest OSDMap query failed; retrying");
                        self.latest_needed = true;
                        self.latest_retry_at = Instant::now() + CONTROL_RETRY;
                    }
                }
            }
            ControlEvent::Fence {
                membership,
                operation,
                request,
                result,
            } => {
                if self.membership != Some(membership) || !matches_eviction(&self.sessions, request)
                {
                    return;
                }
                let Some(pending) = self.fences.get_mut(&request.client) else {
                    return;
                };
                if pending.operation != operation
                    || pending.membership != membership
                    || pending.request != request
                {
                    return;
                }
                pending.active = false;
                match result {
                    Ok(epoch) => pending.proof = Some(epoch),
                    Err(e) => {
                        warn!(client = request.client, error = %e, "fencing failed; retaining rights and retrying");
                        pending.retry_at = Instant::now() + CONTROL_RETRY;
                    }
                }
            }
        }
    }

    fn drive_recovery_fences(&mut self) {
        if !self.control_allowed() {
            return;
        }
        let Some(membership) = self.membership else {
            return;
        };
        let Some(server) = &mut self.server else {
            return;
        };
        for target in server.recovery_fences(&mut self.sessions) {
            if self.recovery_fences.contains_key(&target.key) {
                continue;
            }
            if self.recovery_fences.len() + self.fences.len() >= MAX_FENCES
                || target.addresses.0.is_empty()
                || target.addresses.0.len() > 64
                || target.addresses.0.iter().any(|a| {
                    a.addr
                        .is_none_or(|s| s.port() == 0 || s.ip().is_unspecified())
                })
            {
                self.fatal_error = Some("invalid or over-budget recovery fencing union".into());
                return;
            }
            self.next_operation += 1;
            self.recovery_fences.insert(
                target.key,
                PendingRecoveryFence {
                    target,
                    operation: self.next_operation,
                    active: false,
                    retry_at: Instant::now(),
                    proof: None,
                },
            );
        }
        let mut active = self.fences.values().filter(|p| p.active).count()
            + self.recovery_fences.values().filter(|p| p.active).count();
        let mut done = Vec::new();
        for (key, pending) in &mut self.recovery_fences {
            if self
                .server
                .as_ref()
                .is_none_or(|s| !s.matches_recovery_fence(&self.sessions, &pending.target))
            {
                done.push(*key);
                continue;
            }
            if let Some(epoch) = pending.proof {
                if epoch != 0
                    && self.objecter.epoch() >= epoch
                    && pending
                        .target
                        .addresses
                        .0
                        .iter()
                        .all(|addr| self.objecter.osdmap().is_blocklisted(addr))
                {
                    self.objecter.set_epoch_barrier(epoch);
                    if let Some(server) = &mut self.server {
                        server.finish_recovery_eviction(&mut self.sessions, &pending.target, epoch);
                    }
                    done.push(*key);
                    continue;
                }
                pending.proof = None;
            }
            if pending.active || active >= MAX_CONTROL_WORKERS || Instant::now() < pending.retry_at
            {
                continue;
            }
            pending.active = true;
            active += 1;
            let (target, operation, tx) = (
                pending.target.clone(),
                pending.operation,
                self.control_tx.clone(),
            );
            let backend = ClusterFence {
                monc: self.monc.clone(),
                objecter: self.objecter.clone(),
            };
            let barrier = self.server.as_ref().map_or(0, Server::epoch_barrier);
            tokio::spawn(async move {
                let result = async {
                    let mut epoch = barrier;
                    for addr in &target.addresses.0 {
                        epoch = epoch.max(fencing::fence(&backend, addr, epoch).await?);
                    }
                    Ok(epoch)
                }
                .await;
                let _ = tx
                    .send(ControlEvent::RecoveryFence {
                        membership,
                        operation,
                        target,
                        result,
                    })
                    .await;
            });
        }
        for key in done {
            self.recovery_fences.remove(&key);
        }
    }

    fn drive_control(&mut self) {
        if self.check_failure().is_err() {
            return;
        }
        self.drive_recovery_fences();
        let now = Instant::now();
        if self.latest_needed && self.latest_active.is_none() && now >= self.latest_retry_at {
            self.next_operation += 1;
            let operation = self.next_operation;
            self.latest_active = Some(operation);
            let (tx, monc, objecter, membership) = (
                self.control_tx.clone(),
                self.monc.clone(),
                self.objecter.clone(),
                self.membership,
            );
            tokio::spawn(async move {
                let result = fencing::latest_epoch(&monc, &objecter).await;
                let _ = tx
                    .send(ControlEvent::Latest {
                        membership,
                        operation,
                        result,
                    })
                    .await;
            });
        }
        let serving = self.is_serving();
        let mut active = self.fences.values().filter(|p| p.active).count()
            + self.recovery_fences.values().filter(|p| p.active).count();
        let mut completed = Vec::new();
        for (&client, pending) in &mut self.fences {
            if Some(pending.membership) != self.membership
                || !matches_eviction(&self.sessions, pending.request)
            {
                completed.push(client);
                continue;
            }
            if let Some(epoch) = pending.proof {
                if !serving {
                    continue;
                }
                let addresses = self
                    .server
                    .as_ref()
                    .map(|s| s.fence_addresses(client))
                    .unwrap_or_default();
                if self.objecter.epoch() >= epoch
                    && self.objecter.osdmap().is_blocklisted(&pending.request.addr)
                    && addresses
                        .iter()
                        .all(|addr| self.objecter.osdmap().is_blocklisted(addr))
                {
                    // This is the sole cleanup boundary. Publish both barriers first.
                    self.objecter.set_epoch_barrier(epoch);
                    if let Some(server) = &mut self.server {
                        server.finish_eviction(&mut self.sessions, pending.request, epoch);
                    }
                    completed.push(client);
                    if self.server.as_ref().and_then(Server::fatal_error).is_some() {
                        break;
                    }
                    continue;
                }
                pending.proof = None;
            }
            if pending.active || active >= MAX_CONTROL_WORKERS || pending.retry_at > now {
                continue;
            }
            pending.active = true;
            active += 1;
            let (tx, membership, operation, request) = (
                self.control_tx.clone(),
                pending.membership,
                pending.operation,
                pending.request,
            );
            let backend = ClusterFence {
                monc: self.monc.clone(),
                objecter: self.objecter.clone(),
            };
            let barrier = self.server.as_ref().map(Server::epoch_barrier).unwrap_or(0);
            let mut addresses = self
                .server
                .as_ref()
                .map(|s| s.fence_addresses(client))
                .unwrap_or_default();
            if !addresses.contains(&request.addr) {
                addresses.push(request.addr);
            }
            tokio::spawn(async move {
                let result = async {
                    let mut epoch = barrier;
                    for addr in &addresses {
                        epoch = epoch.max(fencing::fence(&backend, addr, epoch).await?);
                    }
                    Ok(epoch)
                }
                .await;
                let _ = tx
                    .send(ControlEvent::Fence {
                        membership,
                        operation,
                        request,
                        result,
                    })
                    .await;
            });
        }
        for client in completed {
            self.fences.remove(&client);
        }
    }

    fn on_msgr_event(&mut self, ev: Event) {
        match ev {
            Event::Accepted { conn } => {
                debug!(
                    conn = conn.id(),
                    peer = %conn.peer_addr(),
                    who = %conn.peer_auth_name().map(|n| n.to_string()).unwrap_or_default(),
                    "accepted"
                );
                if conn.peer_type() == EntityType::CLIENT
                    && self.recovery_input_allowed()
                    && let Some(server) = &mut self.server
                {
                    let auths = server.cap_auths_for(&conn);
                    server.recovery_accepted(
                        &mut self.sessions,
                        conn.peer_global_id(),
                        Box::new(conn),
                        auths,
                    );
                }
            }
            Event::Message { conn, msg } => {
                if conn.peer_type() == EntityType::CLIENT {
                    self.on_client_raw(conn, msg);
                } else {
                    debug!(
                        peer = conn.peer_type().name(),
                        msg_type = msg.msg_type,
                        "non-client message ignored"
                    );
                }
            }
            Event::Reset { conn } | Event::Closed { conn } => {
                debug!(conn = conn.id(), "connection gone");
                if let Some(server) = &mut self.server {
                    server.recovery_connection_gone(
                        &mut self.sessions,
                        conn.peer_global_id(),
                        conn.id(),
                    );
                }
            }
            Event::Connected { .. } => {}
        }
    }

    fn on_client_raw(&mut self, conn: Connection, raw: denc::RawMessage) {
        let received = Instant::now();
        if self.check_failure().is_err() {
            return;
        }
        self.sessions.pause_output(!self.output_allowed());
        if raw_bytes(&raw) > MAX_HELD_BYTES {
            if raw.msg_type != proto::msg_type::CLIENT_METRICS {
                self.fatal_error = Some("client message over admission byte limit".into());
            }
            return;
        }
        if raw.msg_type == proto::msg_type::CLIENT_RECONNECT && self.reconnect_input_allowed() {
            self.dispatch_admitted(conn, raw, received);
            return;
        }
        if self.recovery_input_allowed() {
            self.dispatch_admitted(conn, raw, received);
            return;
        }
        if raw.msg_type == proto::msg_type::CLIENT_RECONNECT && !self.has_membership() {
            // A cold/unassigned follower has neither a gather nor terminal authority.
            return;
        }
        // Only startup/map/health waits hold reconnect. Checked late keys have
        // a terminal consumer independent of the nonempty collection path.
        if raw.msg_type == proto::msg_type::CLIENT_RECONNECT && self.has_membership() {
            let bytes = raw_bytes(&raw);
            if self.reconnect_held.len() >= MAX_HELD_MESSAGES
                || bytes > MAX_HELD_BYTES.saturating_sub(self.reconnect_bytes)
            {
                self.fatal_error = Some("reconnect gather input over budget".into());
                return;
            }
            self.reconnect_bytes += bytes;
            self.reconnect_held.push_back((conn, raw, received));
            return;
        }
        let control = is_control_raw(&raw);
        let capacity = self
            .server
            .as_ref()
            .is_some_and(|s| s.input_capacity(raw_bytes(&raw), control));
        if !self.is_serving() || !capacity || (!control && !self.held.is_empty()) {
            let bytes = raw_bytes(&raw);
            if self.held.len() >= MAX_HELD_MESSAGES
                || bytes > MAX_HELD_BYTES.saturating_sub(self.held_bytes)
            {
                self.fatal_error = Some("held client input capacity exhausted".into());
                return;
            }
            if let Some(server) = &self.server {
                server.note_held_activity(
                    &mut self.sessions,
                    conn.peer_global_id(),
                    &conn,
                    &raw,
                    received,
                );
            }
            self.held_bytes += bytes;
            self.held.push_back((conn, raw, received));
            return;
        }
        self.dispatch_admitted(conn, raw, received);
    }

    fn dispatch_admitted(&mut self, conn: Connection, raw: denc::RawMessage, received: Instant) {
        let client = conn.peer_global_id();
        let Some(server) = self.server.as_mut() else {
            return;
        };
        let cap_auths = server.cap_auths_for(&conn);
        dispatch_client_raw_at(
            server,
            &mut self.sessions,
            client,
            Box::new(conn),
            cap_auths,
            raw,
            received,
        );
    }

    fn on_tick(&mut self) {
        if self.check_failure().is_err() {
            return;
        }
        // Eligible queued traffic keeps its original receive time. Account for
        // it before ACTIVE expiry; rejected or old attachments cannot renew.
        self.release_held();
        if self.check_failure().is_err() {
            return;
        }
        let serving = self.is_serving();
        let data = self.data_work_allowed();
        let execution = self.control_allowed();
        debug!(state = %self.state, membership = ?self.membership, serving,
            held_inputs = self.held.len(), held_bytes = self.held_bytes,
            queued_output_bytes = self.sessions.queued_output_bytes(),
            fencing = self.fences.len(), latest_query = self.latest_active.is_some(), "daemon status");
        self.sessions.pause_output(!self.output_allowed());
        if let Some(server) = &mut self.server {
            server.set_runtime_permissions(serving, data, execution);
            if serving {
                server.tick_serving(&mut self.sessions, self.state == MdsState::ACTIVE);
            } else {
                server.pump(&mut self.sessions, CLIENT_TURN_BUDGET);
            }
            server.log_status();
            if let Some(r) = server.recovery_status() {
                debug!(
                    missing = r.missing,
                    surviving = r.surviving,
                    complete = r.complete,
                    caps = r.caps,
                    locks = r.locks,
                    realms = r.realms,
                    bytes = r.bytes,
                    early_inputs = r.early_inputs,
                    finalization_blocked = r.finalization_blocked,
                    "recovery collection status"
                );
            }
        }
    }

    async fn shutdown(&mut self) {
        info!("shutting down: down:dne beacon");
        self.beacon.set_want_state(self.last_epoch, MdsState::DNE);
        self.beacon.send_now();
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.beacon.stop();
        self.objecter.shutdown();
        self.monc.shutdown();
        self.msgr.shutdown();
    }
}

/// Tests without an earlier holding queue start the receive clock here.
#[cfg(test)]
pub(crate) fn dispatch_client_raw<J: engine::Journal>(
    server: &mut Server<J>,
    sessions: &mut SessionMap,
    client: u64,
    link: Box<dyn ClientLink>,
    cap_auths: Vec<proto::MdsCapAuth>,
    raw: denc::RawMessage,
) where
    Server<J>: WireDispatch,
{
    dispatch_client_raw_at(
        server,
        sessions,
        client,
        link,
        cap_auths,
        raw,
        Instant::now(),
    );
}

/// Shared decode/admission boundary. Holding queues retain the receive clock.
pub(crate) fn dispatch_client_raw_at<J: engine::Journal>(
    server: &mut Server<J>,
    sessions: &mut SessionMap,
    client: u64,
    link: Box<dyn ClientLink>,
    cap_auths: Vec<proto::MdsCapAuth>,
    raw: denc::RawMessage,
    received: Instant,
) where
    Server<J>: WireDispatch,
{
    if server.fatal_error().is_some() || sessions.output_failed() {
        return;
    }
    let msg_type = raw.msg_type;
    let bytes = raw_bytes(&raw);
    let msg = match ClientMessage::from_raw(raw) {
        Ok(Some(m)) => m,
        Ok(None) => {
            debug!(
                client,
                msg_type, "message type clients do not send; dropped"
            );
            return;
        }
        Err(e) => {
            warn!(client, msg_type, "undecodable client message: {e}");
            if matches!(
                msg_type,
                proto::msg_type::CLIENT_SESSION
                    | proto::msg_type::CLIENT_CAPS
                    | proto::msg_type::CLIENT_CAPRELEASE
            ) {
                server.protocol_error(client);
            }
            return;
        }
    };
    if server.collecting_recovery() {
        // Recovery owns denied/late CLOSE and its durable cleanup boundary.
        server.dispatch_recovery(sessions, client, link, msg, cap_auths, bytes, received);
        return;
    }
    if matches!(msg, ClientMessage::Reconnect(_)) {
        server.dispatch_late_reconnect(sessions, client, link);
        return;
    }
    let addr = link.peer_addr();
    if server.is_blocklisted(&addr) {
        // C++ Server::handle_client_session rejects blocklisted OPENs. Check at
        // dispatch, not only on map events: cleanup and held input can race.
        let may_reject = matches!(&msg, ClientMessage::Session(m) if m.op == proto::SessionOp::REQUEST_OPEN)
            || matches!(&msg, ClientMessage::Reconnect(_));
        if may_reject {
            sessions.reject_blocklisted(client, link);
        }
        if sessions.output_failed() {
            return;
        }
        if sessions.get(client).is_some_and(|s| s.addr == addr) {
            server.request_eviction(sessions, client);
        }
        return;
    }
    match msg {
        ClientMessage::Session(m) => {
            if m.op != proto::SessionOp::REQUEST_OPEN
                && !server.attached_link(sessions, client, link.as_ref())
            {
                return;
            }
            server.dispatch_session(sessions, client, link, m, cap_auths, received);
        }
        other => {
            if sessions.get(client).is_none_or(|s| !s.is_open())
                || !server.attached_link(sessions, client, link.as_ref())
            {
                debug!(client, "message without an open session; dropped");
                return;
            }
            server.dispatch_message(sessions, client, other, received);
        }
    }
}

fn raw_bytes(raw: &denc::RawMessage) -> usize {
    raw.front.len() + raw.middle.len() + raw.data.len() + 64
}

fn is_control_raw(raw: &denc::RawMessage) -> bool {
    if matches!(
        raw.msg_type,
        proto::msg_type::CLIENT_CAPS | proto::msg_type::CLIENT_CAPRELEASE
    ) {
        return true;
    }
    if raw.msg_type == proto::msg_type::CLIENT_SESSION {
        return proto::MClientSession::from_raw(raw.clone())
            .is_ok_and(|s| s.op != proto::SessionOp::REQUEST_OPEN);
    }
    if raw.msg_type == proto::msg_type::CLIENT_REQUEST {
        return proto::MClientRequest::from_raw(raw.clone()).is_ok_and(|r| {
            r.op() == proto::MdsOp::SETFILELOCK
                && r.head.args.filelock().lock_type == engine::consts::CEPH_LOCK_UNLOCK
        });
    }
    false
}

fn matches_eviction(sessions: &SessionMap, request: Eviction) -> bool {
    sessions.get(request.client).is_some_and(|s| {
        s.generation == request.session_generation
            && s.addr == request.addr
            && s.state == crate::session::SessionState::Fencing
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    mod faults;

    async fn offline_daemon() -> (
        Daemon,
        mpsc::Receiver<Eviction>,
        mpsc::Receiver<ControlEvent>,
    ) {
        offline_daemon_rows(Vec::new()).await
    }
    async fn offline_daemon_rows(
        records: Vec<store::SessionRecord>,
    ) -> (
        Daemon,
        mpsc::Receiver<Eviction>,
        mpsc::Receiver<ControlEvent>,
    ) {
        let (mut daemon, evictions, controls, _) = offline_daemon_loading(records).await;
        settle(&mut daemon).await;
        (daemon, evictions, controls)
    }
    async fn offline_daemon_loading(
        records: Vec<store::SessionRecord>,
    ) -> (
        Daemon,
        mpsc::Receiver<Eviction>,
        mpsc::Receiver<ControlEvent>,
        mpsc::Receiver<Event>,
    ) {
        let auth = AuthContext::new(
            AuthName::new(EntityType::MDS, "test"),
            auth::CryptoKey::generate_aes(denc::UTime::default()),
            auth::ENTITY_MON | auth::ENTITY_OSD,
        );
        let (msgr, msgr_events) = Messenger::bind(
            EntityName::new(EntityType::MDS, 0),
            "127.0.0.1:0".parse().unwrap(),
            auth,
        )
        .await
        .unwrap();
        // No monitor addresses. This fixture never contacts a Ceph cluster.
        let (monc, _) = MonClient::start(msgr.clone(), MonClientConfig::new(vec![]));
        let beacon = Beacon::start(monc.clone(), BeaconConfig::new("test", Uuid::default()));
        let (objecter, _) =
            Objecter::start(MsgrTransport::new(msgr.clone()), ObjecterConfig::default());
        let (evict_tx, evict_rx) = mpsc::channel(8);
        let (control_tx, control_rx) = mpsc::channel(16);
        let membership = Membership {
            gid: 0,
            rank: 0,
            incarnation: 2,
        };
        let mut map = MdsMap {
            epoch: 1,
            ..Default::default()
        };
        map.up.insert(0, 0);
        map.mds_info.insert(
            0,
            monc::mdsmap::MdsInfo {
                global_id: 0,
                name: "test".into(),
                rank: 0,
                inc: 2,
                state: MdsState::ACTIVE,
                state_seq: 1,
                addrs: msgr.my_addrs(),
                laggy_since: denc::UTime::default(),
                export_targets: Default::default(),
                mds_features: 0,
                join_fscid: 0,
                flags: 0,
                compat: monc::CompatSet::rust_mds(),
            },
        );
        let owner = store::OwnerToken {
            shard_epoch: 1,
            rank: 0,
            incarnation: 2,
            gid: 0,
        };
        let writer = store::WriterIdentity {
            rank_owner: store::RankOwner {
                rank: 0,
                incarnation: 2,
                gid: 0,
            },
            addresses: msgr.my_addrs(),
        };
        let config = engine::EngineConfig::default();
        let mut state = engine::Engine::new(config.clone(), crate::journal::SeedJournal::new())
            .checkpoint()
            .unwrap();
        // Closed-row routing fixtures supply a matching checked seed receipt.
        // They exercise receive policy, not the already-tested durability chain.
        for row in &records {
            if row.lifecycle == store::SessionLifecycle::Closed {
                state
                    .cleanup_receipts
                    .push((row.key, row.cleanup_seq.unwrap()));
            }
        }
        let descriptor = store::CheckpointDescriptor {
            generation: 1,
            covered_seq: state.covered_seq,
            manifest_name: "test".into(),
            manifest_checksum: 0,
        };
        let loaded = persistence::Loaded {
            identity: store::StoreIdentity {
                cluster_fsid: [0; 16],
                filesystem_id: 1,
                metadata_pool: 0,
            },
            owner,
            tail: state.covered_seq,
            checkpoint: descriptor.clone(),
            state,
            records: vec![],
            sessions: store::RankSessionState {
                control: store::SessionsInspection {
                    owner: writer.rank_owner,
                    writer,
                    predecessor: None,
                    revision: 1,
                    next_incarnation: 1,
                },
                records,
            },
            initial_range: config.ino_lease,
            barrier: 0,
        };
        let (storage_tx, mut commands) = mpsc::channel::<persistence::Command>(16);
        let (storage_events, storage_rx) = mpsc::channel(64);
        let events = storage_events.clone();
        // G1 wire/fencing fixture: deterministic asynchronous completion transport.
        // Native store integration has separate actual compound-operation tests.
        tokio::spawn(async move {
            let mut next_key = 1;
            let mut keys = HashMap::new();
            let mut tail = descriptor.covered_seq;
            while let Some(command) = commands.recv().await {
                use persistence::{CommandKind as C, EventKind as E};
                let kind = match command.kind {
                    C::TrimInitial => E::InitialTrimmed(descriptor.covered_seq),
                    C::Append { seq, body } => {
                        assert_eq!(seq, tail + 1);
                        store::record::decode_record(&body).unwrap();
                        tail = seq;
                        E::Committed(seq)
                    }
                    C::Open(identity) => {
                        let key = *keys.entry(identity.client).or_insert_with(|| {
                            let key = engine::SessionKey {
                                client: identity.client,
                                incarnation: next_key,
                            };
                            next_key += 1;
                            key
                        });
                        E::Opened(store::SessionRecord {
                            key,
                            identity,
                            lifecycle: store::SessionLifecycle::Open,
                            cleanup_seq: None,
                        })
                    }
                    C::Closing(key) => E::Closing(key),
                    C::Closed { key, cleanup_seq } => {
                        keys.remove(&key.client);
                        E::Closed { key, cleanup_seq }
                    }
                    C::Checkpoint(state) => E::Checkpoint(store::CheckpointDescriptor {
                        generation: 2,
                        covered_seq: state.covered_seq,
                        manifest_name: "test2".into(),
                        manifest_checksum: 0,
                    }),
                };
                if events
                    .send(persistence::Event {
                        membership,
                        owner: Some(owner),
                        operation: command.operation,
                        kind,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        let persistence_config = crate::config::PersistenceConfig::default();
        let mut server = Server::from_loaded(
            "a".into(),
            2,
            1,
            loaded,
            config,
            &persistence_config.limits,
            storage_tx.clone(),
            10000,
            16,
            Duration::from_secs(30),
        )
        .unwrap();
        server.set_objecter(objecter.clone());
        server.set_evict_channel(evict_tx.clone());
        let (authority, authority_rx) = tokio::sync::watch::channel(Authority {
            membership: Some(membership),
            state: MdsState::ACTIVE,
            healthy: true,
        });
        let daemon = Daemon {
            msgr,
            monc,
            beacon,
            objecter,
            fsid: Uuid::default(),
            last_epoch: 1,
            state: MdsState::ACTIVE,
            seen_in_map: true,
            mdsmap: Some(map),
            sessions: SessionMap::default(),
            server: Some(server),
            held: VecDeque::new(),
            held_bytes: 0,
            evict_tx,
            control_tx,
            membership: Some(membership),
            fences: HashMap::new(),
            recovery_fences: BTreeMap::new(),
            next_operation: 0,
            latest_needed: false,
            latest_active: None,
            latest_retry_at: Instant::now(),
            fatal_error: None,
            beacon_lag_override: Some(false),
            test_global_id: None,
            phase: Phase::Loading,
            filesystems: vec![],
            persistence_config,
            storage_events,
            storage_tx: Some(storage_tx),
            startup_operation: Some(10000),
            authority,
            authority_rx,
            jobs: HashMap::new(),
            reconnect_held: VecDeque::new(),
            reconnect_bytes: 0,
            reconnect_confirmed_at: None,
            test_storage_rx: Some(storage_rx),
        };
        (daemon, evict_rx, control_rx, msgr_events)
    }

    async fn settle(d: &mut Daemon) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                d.refresh_authority();
                if let Some(server) = &mut d.server {
                    server.pump(&mut d.sessions, 32);
                }
                let mut events = Vec::new();
                if let Some(rx) = &mut d.test_storage_rx {
                    while let Ok(event) = rx.try_recv() {
                        events.push(event);
                    }
                }
                for event in events {
                    d.on_storage_event(event).unwrap();
                }
                d.drive_phase().unwrap();
                d.refresh_authority();
                d.sessions.drain_output(32);
                d.drive_phase().unwrap();
                d.release_held();
                if d.server
                    .as_ref()
                    .is_some_and(|s| s.pending_storage() == 0 && !s.has_work())
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn recovery_phase_requests_wait_for_monitor_confirmation() {
        let (mut d, _, _, _) = offline_daemon_loading(vec![]).await;
        d.state = MdsState::REPLAY;
        settle(&mut d).await;
        assert_eq!(d.beacon.want_state(), MdsState::RECONNECT);
        assert_eq!(d.phase, Phase::Reconnect);
        d.drive_phase().unwrap();
        assert_eq!(d.beacon.want_state(), MdsState::RECONNECT);
        assert_ne!(d.phase, Phase::Serving);
        d.state = MdsState::RECONNECT;
        d.drive_phase().unwrap();
        assert_eq!(d.beacon.want_state(), MdsState::REJOIN);
        assert_eq!(d.phase, Phase::Rejoin);
        assert!(d.server.as_ref().unwrap().recovery_structural_done());
        assert!(!d.server.as_ref().unwrap().recovered());
        d.state = MdsState::REJOIN;
        d.drive_phase().unwrap();
        assert_eq!(d.beacon.want_state(), MdsState::CLIENTREPLAY);
        assert_eq!(d.phase, Phase::ClientReplay);
        d.state = MdsState::CLIENTREPLAY;
        settle(&mut d).await;
        assert!(d.server.as_ref().unwrap().recovered());
        assert_eq!(d.beacon.want_state(), MdsState::ACTIVE);
        d.state = MdsState::ACTIVE;
        d.drive_phase().unwrap();
        assert_eq!(d.phase, Phase::Serving);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn g3_e2_creating_starting_and_already_active_load_all_execute_the_marker() {
        for state in [MdsState::CREATING, MdsState::STARTING, MdsState::ACTIVE] {
            let (mut d, _, _, _) = offline_daemon_loading(vec![]).await;
            d.state = state;
            d.beacon.set_want_state(1, state);
            d.refresh_authority();
            d.server.as_mut().unwrap().pump(&mut d.sessions, 32);
            let event = tokio::time::timeout(
                Duration::from_secs(2),
                d.test_storage_rx.as_mut().unwrap().recv(),
            )
            .await
            .unwrap()
            .unwrap();
            d.on_storage_event(event).unwrap();
            assert!(!d.server.as_ref().unwrap().recovered());
            assert!(!d.server.as_ref().unwrap().recovery_structural_done());
            d.drive_phase().unwrap();
            assert!(d.server.as_ref().unwrap().recovery_structural_done());
            assert!(!d.server.as_ref().unwrap().recovered());
            assert!(!d.is_serving());
            assert!(d.server.as_ref().unwrap().has_work());
            d.refresh_authority();
            d.server.as_mut().unwrap().pump(&mut d.sessions, 1);
            assert!(d.server.as_ref().unwrap().recovered());
            assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
            d.drive_phase().unwrap();
            if state != MdsState::ACTIVE {
                assert_eq!(d.beacon.want_state(), MdsState::ACTIVE);
                assert!(!d.is_serving());
                d.state = MdsState::ACTIVE;
                d.drive_phase().unwrap();
            }
            assert!(d.is_serving());
            assert_eq!(
                d.server
                    .as_ref()
                    .unwrap()
                    .checkpoint_for_test()
                    .unwrap()
                    .covered_seq,
                1
            );
            stop_offline(&d);
        }
    }

    async fn next_loopback_event(rx: &mut mpsc::Receiver<Event>) -> Event {
        tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .unwrap()
            .unwrap()
    }

    async fn loopback_client(
        d: &mut Daemon,
        events: &mut mpsc::Receiver<Event>,
        issuer: &auth::service::MonTicketIssuer,
        client: u64,
    ) -> (Messenger, Connection, mpsc::Receiver<Event>, Connection) {
        let name = AuthName::new(EntityType::CLIENT, "test");
        let key = auth::CryptoKey::generate_aes(denc::UTime::now());
        let context = AuthContext::new(name.clone(), key.clone(), auth::ENTITY_MDS);
        *context.client() = issuer.client_for(name, key, client).unwrap();
        let (messenger, mut received) = Messenger::bind(
            EntityName::new(EntityType::CLIENT, client as i64),
            "127.0.0.1:0".parse().unwrap(),
            context,
        )
        .await
        .unwrap();
        let connection = messenger.connect(d.msgr.my_addr(), EntityType::MDS);
        assert!(
            connection.send(
                proto::MClientSession::new(proto::SessionOp::REQUEST_RENEWCAPS, 17)
                    .to_raw(denc::features::ALL)
            )
        );
        let accepted = next_loopback_event(events).await;
        let Event::Accepted { conn } = &accepted else {
            panic!("expected authenticated accept");
        };
        assert_eq!(conn.peer_global_id(), client);
        let server_link = conn.clone();
        d.on_msgr_event(accepted);
        d.on_msgr_event(next_loopback_event(events).await);
        assert!(matches!(
            next_loopback_event(&mut received).await,
            Event::Connected { .. }
        ));
        (messenger, connection, received, server_link)
    }

    #[tokio::test]
    async fn g3_c1_all_closed_and_unknown_late_reconnect_never_stays_in_raw_hold() {
        use proto::{MClientReconnect, MClientSession, SessionOp};
        for before_load in [false, true] {
            for version in [3, 5] {
                let mut row = missing_row();
                row.lifecycle = store::SessionLifecycle::Closed;
                row.cleanup_seq = Some(1);
                let (mut d, _, _, mut events) = offline_daemon_loading(vec![row]).await;
                let issuer = auth::service::MonTicketIssuer::new(
                    d.msgr.auth().name().clone(),
                    d.msgr.auth().key().clone(),
                    1,
                );
                d.msgr.auth().set_rotating(issuer.mds_keystore().rotating);
                let loading = if before_load {
                    d.server.take()
                } else {
                    settle(&mut d).await;
                    None
                };
                let (known, known_link, mut known_rx, server_link) =
                    loopback_client(&mut d, &mut events, &issuer, 42).await;
                let (unknown, unknown_link, mut unknown_rx, _) =
                    loopback_client(&mut d, &mut events, &issuer, 43).await;
                d.sessions.pause_output(true);
                d.sessions.recovery_terminal(
                    42,
                    Box::new(server_link),
                    MClientSession::renewcaps(99),
                );
                for link in [&known_link, &unknown_link] {
                    assert!(
                        link.send(
                            MClientReconnect {
                                version,
                                ..Default::default()
                            }
                            .to_raw(link.features())
                        )
                    );
                    d.on_msgr_event(next_loopback_event(&mut events).await);
                }
                if before_load {
                    assert_eq!(d.reconnect_held.len(), 2);
                    d.server = loading;
                }
                settle(&mut d).await;
                assert!(d.is_serving());
                assert!(
                    d.reconnect_held.is_empty(),
                    "late v{version} reconnect has no consumer after clientless/all-Closed load"
                );
                assert_eq!(d.reconnect_bytes, 0);
                assert!(d.held.is_empty());
                assert_eq!(d.sessions.len(), 0);
                let Event::Message { msg: older, .. } = next_loopback_event(&mut known_rx).await
                else {
                    panic!("expected ordered older output");
                };
                let older = MClientSession::from_raw(older).unwrap();
                assert_eq!((older.op, older.seq), (SessionOp::RENEWCAPS, 99));
                let Event::Message { msg: close, .. } = next_loopback_event(&mut known_rx).await
                else {
                    panic!("expected CLOSE");
                };
                assert_eq!(
                    MClientSession::from_raw(close).unwrap().op,
                    SessionOp::CLOSE
                );
                let Event::Message { msg: reject, .. } = next_loopback_event(&mut unknown_rx).await
                else {
                    panic!("expected sessionless REJECT");
                };
                assert_eq!(
                    MClientSession::from_raw(reject).unwrap().op,
                    SessionOp::REJECT
                );
                let server = d.server.as_ref().unwrap();
                let image = server.checkpoint_for_test().unwrap();
                assert!(image.sessions.is_empty());
                assert_eq!(image.covered_seq, 1);
                assert_eq!(server.pending_storage(), 0);
                known.shutdown();
                unknown.shutdown();
                stop_offline(&d);
            }
        }
    }

    #[tokio::test]
    async fn g3_c1_nonempty_early_map_race_waits_for_confirmed_collection() {
        for version in [3, 5] {
            let (mut d, _, _, mut events) = offline_daemon_loading(vec![missing_row()]).await;
            d.state = MdsState::REPLAY;
            settle(&mut d).await;
            d.beacon.set_want_state(1, MdsState::REPLAY);
            let issuer = auth::service::MonTicketIssuer::new(
                d.msgr.auth().name().clone(),
                d.msgr.auth().key().clone(),
                1,
            );
            d.msgr.auth().set_rotating(issuer.mds_keystore().rotating);
            let (client, link, _received, _) =
                loopback_client(&mut d, &mut events, &issuer, 42).await;
            assert!(
                link.send(
                    proto::MClientReconnect {
                        version,
                        ..Default::default()
                    }
                    .to_raw(link.features())
                )
            );
            d.on_msgr_event(next_loopback_event(&mut events).await);
            assert_eq!(d.reconnect_held.len(), 1);
            d.drive_phase().unwrap();
            assert_eq!(d.beacon.want_state(), MdsState::RECONNECT);
            settle(&mut d).await;
            assert!(d.reconnect_held.is_empty());
            assert_eq!(d.server.as_ref().unwrap().gather_len(), 1);
            assert_eq!(
                d.server
                    .as_ref()
                    .unwrap()
                    .recovery_status()
                    .unwrap()
                    .complete,
                0
            );
            d.state = MdsState::RECONNECT;
            d.drive_phase().unwrap();
            settle(&mut d).await;
            assert_eq!(d.server.as_ref().unwrap().gather_len(), 0);
            assert!(!d.server.as_ref().unwrap().recovered());
            assert_eq!(
                d.sessions.get(42).unwrap().state,
                crate::session::SessionState::Recovering
            );
            assert_ne!(d.beacon.want_state(), MdsState::ACTIVE);
            client.shutdown();
            stop_offline(&d);
        }
    }

    fn stop_offline(d: &Daemon) {
        d.beacon.stop();
        d.objecter.shutdown();
        d.monc.shutdown();
        d.msgr.shutdown();
    }

    fn missing_row() -> store::SessionRecord {
        store::SessionRecord {
            key: engine::SessionKey {
                client: 42,
                incarnation: 1,
            },
            identity: store::NewSessionRecord {
                client: 42,
                open_id: [1; 16],
                authenticated_entity: "client.test".into(),
                authorization_context: denc::Bytes::new(),
                peer_addresses: denc::EntityAddrVec(vec![
                    denc::EntityAddr::new(
                        denc::AddrType::LEGACY,
                        "127.0.0.1:4100".parse().unwrap(),
                        10,
                    ),
                    denc::EntityAddr::new(
                        denc::AddrType::LEGACY,
                        "127.0.0.1:4100".parse().unwrap(),
                        20,
                    ),
                ]),
                client_features: proto::features::CEPH_20_2_FEATURES,
                server_features: proto::features::MDS_SUPPORTED_FEATURES,
                metadata: [(
                    denc::Bytes::from_static(b"timeout"),
                    denc::Bytes::from_static(b"0"),
                )]
                .into(),
            },
            lifecycle: store::SessionLifecycle::Open,
            cleanup_seq: None,
        }
    }

    #[tokio::test]
    async fn g3_missing_key_fence_failure_and_partial_map_do_not_complete_gather() {
        let row = missing_row();
        let key = row.key;
        let (mut d, _, _) = offline_daemon_rows(vec![row.clone()]).await;
        d.state = MdsState::RECONNECT;
        d.drive_phase().unwrap();
        d.refresh_authority();
        let deadline = d.server.as_ref().unwrap().reconnect_deadline().unwrap();
        d.server
            .as_mut()
            .unwrap()
            .expire_reconnect(&mut d.sessions, deadline);
        d.drive_control();
        let target = d.recovery_fences[&key].target.clone();
        let operation = d.recovery_fences[&key].operation;
        assert!(target.attachment.is_none());
        assert!(d.sessions.get(42).is_none());
        d.on_control_event(ControlEvent::RecoveryFence {
            membership: d.membership.unwrap(),
            operation,
            target: target.clone(),
            result: Err("injected blocklist/map wait failure".into()),
        });
        assert_eq!(d.server.as_ref().unwrap().gather_len(), 1);
        assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
        let mut map = osdc::OsdMap::empty();
        map.epoch = 9;
        map.blocklist
            .insert(row.identity.peer_addresses.0[0], denc::UTime::default());
        d.objecter.install_map(map.clone());
        d.objecter
            .wait_for_map_timeout(9, Duration::from_secs(1))
            .await
            .unwrap();
        d.recovery_fences.get_mut(&key).unwrap().active = true;
        d.on_control_event(ControlEvent::RecoveryFence {
            membership: d.membership.unwrap(),
            operation,
            target: target.clone(),
            result: Ok(9),
        });
        d.drive_control();
        assert_eq!(d.server.as_ref().unwrap().gather_len(), 1);
        assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
        map.epoch = 10;
        map.blocklist
            .insert(row.identity.peer_addresses.0[1], denc::UTime::default());
        d.objecter.install_map(map);
        d.objecter
            .wait_for_map_timeout(10, Duration::from_secs(1))
            .await
            .unwrap();
        d.recovery_fences.get_mut(&key).unwrap().active = true;
        let mut stale = target.clone();
        stale.owner.gid += 1;
        d.on_control_event(ControlEvent::RecoveryFence {
            membership: d.membership.unwrap(),
            operation,
            target: stale,
            result: Ok(10),
        });
        assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
        d.on_control_event(ControlEvent::RecoveryFence {
            membership: d.membership.unwrap(),
            operation,
            target,
            result: Ok(10),
        });
        d.drive_control();
        assert_eq!(
            d.server.as_ref().unwrap().gather_len(),
            1,
            "fencing alone is not Closed"
        );
        settle(&mut d).await;
        assert_eq!(d.server.as_ref().unwrap().gather_len(), 0);
        assert_eq!(d.server.as_ref().unwrap().epoch_barrier(), 10);
        assert!(!d.is_serving());
        assert!(!d.server.as_ref().unwrap().recovered());
        assert!(d.sessions.get(42).is_none());
        stop_offline(&d);
    }

    #[tokio::test]
    async fn g3_requested_active_never_unblocks_nonempty_ordinary_admission() {
        let (mut d, _, _) = offline_daemon_rows(vec![missing_row()]).await;
        d.state = MdsState::RECONNECT;
        d.drive_phase().unwrap();
        d.beacon.set_want_state(1, MdsState::ACTIVE);
        assert!(!d.is_serving());
        assert!(d.control_allowed());
        assert!(d.output_allowed());
        d.state = MdsState::ACTIVE;
        d.drive_phase().unwrap();
        assert!(
            !d.is_serving(),
            "even confirmed ACTIVE cannot bypass nonempty E2 finalization"
        );
        d.beacon_lag_override = Some(true);
        d.refresh_authority();
        assert!(!d.control_allowed());
        assert!(!d.output_allowed());
        assert!(!d.data_work_allowed());
        stop_offline(&d);
    }

    #[tokio::test]
    async fn g3_rejoin_output_requires_confirmed_clientreplay_and_healthy_permissions() {
        let (mut d, _, _) = offline_daemon().await;
        let link = Link::default();
        d.sessions.install_silent(
            42,
            Box::new(link.clone()),
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        d.sessions.pause_output(true);
        d.sessions
            .get_mut(42)
            .unwrap()
            .send(&proto::MClientSession::renewcaps(71));
        d.state = MdsState::REJOIN;
        d.beacon.set_want_state(1, MdsState::REJOIN);
        d.beacon_lag_override = Some(true);
        d.drive_phase().unwrap();
        assert_eq!(
            d.beacon.want_state(),
            MdsState::REJOIN,
            "unhealthy recovery cannot advance phases"
        );
        d.beacon_lag_override = Some(false);
        d.drive_phase().unwrap();
        assert_eq!(d.beacon.want_state(), MdsState::CLIENTREPLAY);
        assert!(!d.is_serving());
        d.state = MdsState::CLIENTREPLAY;
        d.refresh_authority();
        assert!(d.output_allowed());
        assert!(d.data_work_allowed());
        d.sessions.drain_output(32);
        d.drive_phase().unwrap();
        assert_eq!(d.beacon.want_state(), MdsState::ACTIVE);
        assert!(!d.is_serving(), "requested ACTIVE is not confirmed ACTIVE");
        d.state = MdsState::ACTIVE;
        d.drive_phase().unwrap();
        assert!(d.is_serving());
        assert_eq!(link.0.lock().unwrap().len(), 1);
        let mut map = osdc::OsdMap::empty();
        map.epoch = 10;
        map.blocklist
            .insert(d.msgr.my_addrs().0[0], denc::UTime::default());
        d.objecter.install_map(map);
        d.objecter
            .wait_for_map_timeout(10, Duration::from_secs(1))
            .await
            .unwrap();
        assert!(
            !d.is_serving(),
            "self blocklist closes ordinary permission before its event callback"
        );
        stop_offline(&d);
    }

    #[tokio::test]
    async fn g3_permissions_reject_loaded_owner_from_another_rank_incarnation() {
        let (mut d, _, _) = offline_daemon().await;
        d.membership.as_mut().unwrap().incarnation += 1;
        d.mdsmap.as_mut().unwrap().mds_info.get_mut(&0).unwrap().inc += 1;
        assert!(d.has_membership());
        assert!(!d.control_allowed());
        assert!(!d.output_allowed());
        assert!(!d.data_work_allowed());
        assert!(!d.is_serving());
        stop_offline(&d);
    }

    #[tokio::test]
    async fn g3_cold_follower_never_acquires_or_reports_damage_and_promotion_uses_load() {
        let (mut d, _, _) = offline_daemon().await;
        d.server = None;
        d.phase = Phase::Waiting;
        d.membership = None;
        let operation = d.next_operation;
        for state in [MdsState::STANDBY, MdsState::STANDBY_REPLAY] {
            d.state = state;
            d.refresh_authority();
            d.maybe_start_storage().unwrap();
            d.drive_control();
            d.drive_jobs();
            d.drive_phase().unwrap();
            assert_eq!(d.phase, Phase::Waiting);
            assert_eq!(d.next_operation, operation);
            assert!(!d.control_allowed());
            assert!(!d.is_serving());
            assert!(d.server.is_none());
            assert_ne!(d.beacon.want_state(), MdsState::DAMAGED);
        }
        // No native backend starts without a matching authoritative FSMap. This
        // proves the promotion gate without contacting a cluster from this test.
        d.membership = Some(Membership {
            gid: 0,
            rank: 0,
            incarnation: 2,
        });
        d.state = MdsState::REPLAY;
        d.maybe_start_storage().unwrap();
        assert_eq!(d.phase, Phase::Waiting);
        assert!(d.server.is_none());
        stop_offline(&d);
    }

    #[tokio::test]
    async fn g2_r1_reattachment_requires_both_addresses_fenced_before_cleanup() {
        #[derive(Clone)]
        struct Peer {
            link: Link,
            addr: denc::EntityAddr,
        }
        impl ClientLink for Peer {
            fn send_raw(&self, raw: denc::RawMessage) -> bool {
                self.link.send_raw(raw)
            }
            fn features(&self) -> u64 {
                self.link.features()
            }
            fn peer_addr(&self) -> denc::EntityAddr {
                self.addr
            }
            fn authenticated_identity(&self) -> Option<(String, denc::Bytes)> {
                self.link.authenticated_identity()
            }
        }
        let (mut d, mut evictions, _) = offline_daemon().await;
        let a = Peer {
            link: Link::default(),
            addr: denc::EntityAddr::new(
                denc::AddrType::LEGACY,
                "127.0.0.1:4100".parse().unwrap(),
                10,
            ),
        };
        let b = Peer {
            link: Link::default(),
            addr: denc::EntityAddr {
                nonce: 20,
                ..a.addr
            },
        };
        for peer in [&a, &b] {
            dispatch_client_raw(
                d.server.as_mut().unwrap(),
                &mut d.sessions,
                42,
                Box::new(peer.clone()),
                vec![],
                proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
                    .to_raw(denc::features::ALL),
            );
            settle(&mut d).await;
        }
        assert_eq!(d.sessions.get(42).unwrap().addr, b.addr);
        assert_eq!(
            d.server.as_ref().unwrap().fence_addresses(42),
            vec![a.addr, b.addr]
        );
        d.server
            .as_mut()
            .unwrap()
            .request_eviction(&mut d.sessions, 42);
        let request = evictions.recv().await.unwrap();
        d.queue_eviction(request).unwrap();
        let mut map = osdc::OsdMap::empty();
        map.epoch = 11;
        map.blocklist.insert(b.addr, denc::UTime::default());
        d.objecter.install_map(map.clone());
        d.objecter
            .wait_for_map_timeout(11, Duration::from_secs(1))
            .await
            .unwrap();
        let pending = d.fences.get_mut(&42).unwrap();
        pending.proof = Some(11);
        pending.active = true;
        d.drive_control();
        assert!(d.fences.contains_key(&42));
        assert_eq!(
            d.sessions.get(42).unwrap().state,
            crate::session::SessionState::Fencing
        );
        assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
        map.epoch = 12;
        map.blocklist.insert(a.addr, denc::UTime::default());
        d.objecter.install_map(map);
        d.objecter
            .wait_for_map_timeout(12, Duration::from_secs(1))
            .await
            .unwrap();
        d.fences.get_mut(&42).unwrap().proof = Some(12);
        d.drive_control();
        settle(&mut d).await;
        assert!(!d.fences.contains_key(&42));
        assert!(d.sessions.get(42).is_none());
        assert_eq!(d.server.as_ref().unwrap().epoch_barrier(), 12);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn blocklisted_reopen_after_eviction_is_rejected_without_another_map_event() {
        #[derive(Clone)]
        struct AddressLink {
            inner: Link,
            addr: denc::EntityAddr,
        }
        impl ClientLink for AddressLink {
            fn authenticated_identity(&self) -> Option<(String, denc::Bytes)> {
                self.inner.authenticated_identity()
            }
            fn send_raw(&self, raw: denc::RawMessage) -> bool {
                self.inner.send_raw(raw)
            }
            fn features(&self) -> u64 {
                self.inner.features()
            }
            fn peer_addr(&self) -> denc::EntityAddr {
                self.addr
            }
        }
        let (mut d, mut evictions, _) = offline_daemon().await;
        d.beacon_lag_override = Some(false);
        let link = Link::default();
        let open = proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
            .to_raw(denc::features::ALL);
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(link.clone()),
            vec![],
            open.clone(),
        );
        settle(&mut d).await;
        let mut map = osdc::OsdMap::empty();
        map.epoch = 9;
        map.blocklist
            .insert(link.peer_addr(), denc::UTime::default());
        d.objecter.install_map(map);
        d.objecter
            .wait_for_map_timeout(9, Duration::from_secs(1))
            .await
            .unwrap();
        d.server
            .as_mut()
            .unwrap()
            .request_eviction(&mut d.sessions, 42);
        let request = evictions.try_recv().unwrap();
        d.queue_eviction(request).unwrap();
        d.on_control_event(ControlEvent::Fence {
            membership: d.membership.unwrap(),
            operation: d.fences[&42].operation,
            request,
            result: Ok(9),
        });
        d.drive_control();
        settle(&mut d).await;
        assert!(d.sessions.get(42).is_none());
        // No MapApplied event follows cleanup. Dispatch must check the existing map.
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(link.clone()),
            vec![],
            open.clone(),
        );
        assert!(d.sessions.get(42).is_none());
        let last = link.0.lock().unwrap().last().unwrap().clone();
        let reject = proto::MClientSession::from_raw(last).unwrap();
        assert_eq!(reject.op, proto::SessionOp::REJECT);
        assert!(reject.is_blocklisted());
        assert_eq!(reject.error_string(), Some("blocklisted (blacklisted)"));
        let count = link.0.lock().unwrap().len();
        let mut mkdir = proto::MClientRequest::new(1, proto::MdsOp::MKDIR);
        mkdir.path = proto::FilePath::new(1, "blocked");
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(link.clone()),
            vec![],
            mkdir.to_raw(denc::features::ALL),
        );
        assert_eq!(link.0.lock().unwrap().len(), count);
        assert!(d.sessions.get(42).is_none());
        let valid = AddressLink {
            inner: Link::default(),
            addr: denc::EntityAddr::new(
                denc::AddrType::LEGACY,
                "127.0.0.1:6789".parse().unwrap(),
                2,
            ),
        };
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(valid.clone()),
            vec![],
            open,
        );
        settle(&mut d).await;
        assert!(d.sessions.get(42).unwrap().is_open());
        let generation = d.sessions.get(42).unwrap().generation;
        let mut duplicate = proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0);
        duplicate
            .supported_features
            .insert(proto::features::bit::NOTIFY_SESSION_STATE);
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(link.clone()),
            vec![],
            duplicate.to_raw(denc::features::ALL),
        );
        assert_eq!(d.sessions.get(42).unwrap().addr, valid.addr);
        assert_eq!(d.sessions.get(42).unwrap().generation, generation);
        assert!(d.sessions.get(42).unwrap().is_open());
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(link.clone()),
            vec![],
            mkdir.to_raw(denc::features::ALL),
        );
        let mut lookup = proto::MClientRequest::new(2, proto::MdsOp::LOOKUP);
        lookup.path = proto::FilePath::new(1, "blocked");
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(valid.clone()),
            vec![],
            lookup.to_raw(denc::features::ALL),
        );
        settle(&mut d).await;
        let last = valid.inner.0.lock().unwrap().last().unwrap().clone();
        assert_eq!(proto::MClientReply::from_raw(last).unwrap().result, -2);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn held_open_rechecks_blocklist_when_released() {
        let (mut d, _, _) = offline_daemon().await;
        d.beacon_lag_override = Some(true);
        let conn = d.msgr.connect(denc::EntityAddr::none(), EntityType::CLIENT);
        let open = proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
            .to_raw(denc::features::ALL);
        d.on_client_raw(conn.clone(), open);
        assert_eq!(d.held.len(), 1);
        let mut map = osdc::OsdMap::empty();
        map.epoch = 9;
        let mut blocked = conn.peer_addr();
        blocked.ty = denc::AddrType::LEGACY;
        map.blocklist.insert(blocked, denc::UTime::default());
        assert!(map.is_blocklisted(&conn.peer_addr()));
        d.objecter.install_map(map);
        d.objecter
            .wait_for_map_timeout(9, Duration::from_secs(1))
            .await
            .unwrap();
        d.beacon_lag_override = Some(false);
        d.sessions.pause_output(false);
        d.release_held();
        assert!(d.held.is_empty());
        assert_eq!(d.sessions.len(), 0);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn terminal_output_failure_keeps_daemon_failed_after_runtime_removal() {
        struct FailClose(Link);
        impl ClientLink for FailClose {
            fn send_raw(&self, raw: denc::RawMessage) -> bool {
                if raw.msg_type == proto::msg_type::CLIENT_SESSION
                    && proto::MClientSession::from_raw(raw.clone()).unwrap().op
                        == proto::SessionOp::CLOSE
                {
                    return false;
                }
                self.0.send_raw(raw)
            }
            fn features(&self) -> u64 {
                self.0.features()
            }
            fn peer_addr(&self) -> denc::EntityAddr {
                self.0.peer_addr()
            }
        }
        let (mut d, _, _) = offline_daemon().await;
        d.beacon_lag_override = Some(false);
        let link = Link::default();
        d.sessions.handle(
            42,
            Box::new(FailClose(link.clone())),
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        assert!(d.is_serving());
        d.sessions.pause_output(true);
        d.sessions.handle(
            42,
            Box::new(link.clone()),
            proto::MClientSession::new(proto::SessionOp::REQUEST_CLOSE, 0),
            vec![],
        );
        assert!(d.sessions.get(42).is_none());
        d.sessions.pause_output(false);
        d.sessions.drain_output(1);
        d.sessions.remove(42);
        assert!(d.server.as_ref().unwrap().fatal_error().is_none());
        assert!(d.check_failure().is_err());
        assert!(!d.is_serving());
        d.on_tick();
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            43,
            Box::new(link.clone()),
            vec![],
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
                .to_raw(denc::features::ALL),
        );
        assert!(d.sessions.get(43).is_none());
        assert_eq!(link.0.lock().unwrap().len(), 1);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn laggy_admission_is_bounded_before_session_or_engine_mutation() {
        let (mut d, _, _) = offline_daemon().await;
        d.beacon_lag_override = Some(true);
        assert!(!d.is_serving());
        // No endpoint exists, so this test connection cannot contact a cluster.
        let conn = d.msgr.connect(denc::EntityAddr::none(), EntityType::CLIENT);
        let raw = proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
            .to_raw(denc::features::ALL);
        for _ in 0..MAX_HELD_MESSAGES {
            d.on_client_raw(conn.clone(), raw.clone());
        }
        assert_eq!(d.held.len(), MAX_HELD_MESSAGES);
        assert!(d.held_bytes <= MAX_HELD_BYTES);
        assert_eq!(d.sessions.len(), 0);
        d.on_client_raw(conn.clone(), raw.clone());
        assert!(d.check_failure().is_err());
        let before = d.held_bytes;
        d.on_tick();
        d.on_client_raw(conn, raw);
        assert_eq!(d.held_bytes, before);
        assert_eq!(d.sessions.len(), 0);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn laggy_tick_keeps_timers_paused_and_output_waits_for_health() {
        let (mut d, _, _) = offline_daemon().await;
        let link = Link::default();
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(link.clone()),
            vec![],
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
                .to_raw(denc::features::ALL),
        );
        settle(&mut d).await;
        d.beacon_lag_override = Some(true);
        d.sessions.get_mut(42).unwrap().last_renew = Instant::now() - Duration::from_secs(61);
        d.on_tick();
        assert_eq!(
            d.sessions.get(42).unwrap().state,
            crate::session::SessionState::Open
        );
        assert_eq!(link.0.lock().unwrap().len(), 1);
        // New engine execution also waits while ordinary service is paused.
        let mut request = proto::MClientRequest::new(1, proto::MdsOp::MKDIR);
        request.path = proto::FilePath::new(1, "d");
        d.server
            .as_mut()
            .unwrap()
            .handle(&mut d.sessions, 42, ClientMessage::Request(request));
        assert_eq!(d.sessions.queued_output_bytes(), 0);
        assert_eq!(link.0.lock().unwrap().len(), 1);
        d.beacon_lag_override = Some(false);
        assert!(d.is_serving());
        settle(&mut d).await;
        d.sessions.pause_output(false);
        d.sessions.drain_output(CLIENT_TURN_BUDGET);
        assert!(
            link.0
                .lock()
                .unwrap()
                .iter()
                .any(|m| m.msg_type == proto::msg_type::CLIENT_REPLY)
        );
        assert_eq!(d.sessions.queued_output_bytes(), 0);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn latest_map_failures_and_stale_completions_do_not_feed_cached_versions() {
        let (mut d, _, _) = offline_daemon().await;
        d.on_objecter_event(ObjecterEvent::CheckLatestMap).unwrap();
        d.latest_active = Some(7);
        d.on_control_event(ControlEvent::Latest {
            membership: d.membership,
            operation: 6,
            result: Ok(99),
        });
        assert_eq!(d.latest_active, Some(7));
        assert_eq!(d.objecter.status().await.latest_map_version, 0);
        d.on_control_event(ControlEvent::Latest {
            membership: d.membership,
            operation: 7,
            result: Err("timeout".into()),
        });
        assert!(d.latest_needed);
        assert!(d.latest_active.is_none());
        assert!(d.latest_retry_at > Instant::now());
        assert_eq!(d.objecter.status().await.latest_map_version, 0);
        d.latest_active = Some(8);
        d.on_control_event(ControlEvent::Latest {
            membership: d.membership,
            operation: 8,
            result: Ok(4),
        });
        assert!(!d.latest_needed);
        assert_eq!(d.objecter.status().await.latest_map_version, 4);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn failed_or_stale_fence_completion_retains_rights_and_laggy_gate_holds_cleanup() {
        let (mut d, mut evictions, _) = offline_daemon().await;
        let link = Link::default();
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(link.clone()),
            vec![],
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
                .to_raw(denc::features::ALL),
        );
        settle(&mut d).await;
        d.beacon_lag_override = Some(true);
        d.server
            .as_mut()
            .unwrap()
            .request_eviction(&mut d.sessions, 42);
        let request = evictions.try_recv().unwrap();
        d.queue_eviction(request).unwrap();
        let membership = d.membership.unwrap();
        let operation = d.fences[&42].operation;
        d.on_control_event(ControlEvent::Fence {
            membership,
            operation,
            request,
            result: Err("rejected".into()),
        });
        assert!(d.sessions.get(42).is_some());
        assert_eq!(
            d.sessions.get(42).unwrap().state,
            crate::session::SessionState::Fencing
        );
        assert!(d.fences[&42].proof.is_none());
        d.on_control_event(ControlEvent::Fence {
            membership: Membership {
                incarnation: 99,
                ..membership
            },
            operation,
            request,
            result: Ok(9),
        });
        assert!(d.fences[&42].proof.is_none());
        d.on_control_event(ControlEvent::Fence {
            membership,
            operation: operation + 1,
            request,
            result: Ok(9),
        });
        assert!(d.fences[&42].proof.is_none());
        d.on_control_event(ControlEvent::Fence {
            membership,
            operation,
            request,
            result: Ok(9),
        });
        assert_eq!(d.fences[&42].proof, Some(9));
        assert!(d.has_membership());
        assert!(!d.is_serving()); // No beacon acknowledgment exists in this fixture.
        d.drive_control();
        assert!(d.sessions.get(42).is_some());
        assert_eq!(d.server.as_ref().unwrap().epoch_barrier(), 0);
        d.mdsmap.as_mut().unwrap().up.insert(0, 2);
        assert!(!d.has_membership());
        d.beacon_lag_override = Some(false);
        d.drive_control();
        assert!(d.sessions.get(42).is_some());
        d.mdsmap.as_mut().unwrap().up.insert(0, 0);
        let mut map = osdc::OsdMap::empty();
        map.epoch = 9;
        map.blocklist.insert(request.addr, denc::UTime::default());
        d.objecter.install_map(map);
        d.objecter
            .wait_for_map_timeout(9, Duration::from_secs(1))
            .await
            .unwrap();
        assert!(d.is_serving());
        d.drive_control();
        settle(&mut d).await;
        assert!(d.sessions.get(42).is_none());
        assert!(!d.fences.contains_key(&42));
        assert_eq!(d.objecter.status().await.epoch_barrier, 9);
        assert_eq!(d.server.as_ref().unwrap().epoch_barrier(), 9);
        stop_offline(&d);
    }

    #[tokio::test]
    async fn applied_external_blocklist_queues_fencing_and_self_blocklist_stops_service() {
        let (mut d, mut evictions, _) = offline_daemon().await;
        let link = Link::default();
        dispatch_client_raw(
            d.server.as_mut().unwrap(),
            &mut d.sessions,
            42,
            Box::new(link.clone()),
            vec![],
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
                .to_raw(denc::features::ALL),
        );
        settle(&mut d).await;
        let mut map = osdc::OsdMap::empty();
        map.epoch = 1;
        let mut client_addr = link.peer_addr();
        client_addr.ty = denc::AddrType::LEGACY;
        map.blocklist.insert(client_addr, denc::UTime::default());
        d.objecter.install_map(map.clone());
        d.objecter
            .wait_for_map_timeout(1, Duration::from_secs(1))
            .await
            .unwrap();
        d.on_objecter_event(ObjecterEvent::MapApplied { epoch: 1 })
            .unwrap();
        assert_eq!(evictions.try_recv().unwrap().client, 42);
        assert_eq!(
            d.sessions.get(42).unwrap().state,
            crate::session::SessionState::Fencing
        );
        map.epoch = 2;
        map.blocklist
            .insert(d.msgr.my_addr(), denc::UTime::default());
        d.objecter.install_map(map);
        d.objecter
            .wait_for_map_timeout(2, Duration::from_secs(1))
            .await
            .unwrap();
        assert!(
            d.on_objecter_event(ObjecterEvent::MapApplied { epoch: 2 })
                .is_err()
        );
        assert!(d.sessions.get(42).is_some());
        stop_offline(&d);
    }
    #[derive(Clone, Default)]
    struct Link(std::sync::Arc<std::sync::Mutex<Vec<denc::RawMessage>>>);
    impl ClientLink for Link {
        fn authenticated_identity(&self) -> Option<(String, denc::Bytes)> {
            Some(("client.test".into(), denc::Bytes::new()))
        }
        fn send_raw(&self, raw: denc::RawMessage) -> bool {
            self.0.lock().unwrap().push(raw);
            true
        }
        fn features(&self) -> u64 {
            denc::features::ALL
        }
        fn peer_addr(&self) -> denc::EntityAddr {
            denc::EntityAddr::new(denc::AddrType::LEGACY, "127.0.0.1:6789".parse().unwrap(), 1)
        }
    }
    #[test]
    fn truncated_cap_packets_fail_stop_but_metrics_remain_tolerant() {
        for kind in [
            proto::msg_type::CLIENT_CAPS,
            proto::msg_type::CLIENT_CAPRELEASE,
            proto::msg_type::CLIENT_METRICS,
        ] {
            let mut server = Server::new("a".into(), 2, 7);
            let mut sessions = SessionMap::default();
            let link = Link::default();
            let open = proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0)
                .to_raw(denc::features::ALL);
            dispatch_client_raw(
                &mut server,
                &mut sessions,
                42,
                Box::new(link.clone()),
                vec![],
                open.clone(),
            );
            let count = link.0.lock().unwrap().len();
            dispatch_client_raw(
                &mut server,
                &mut sessions,
                42,
                Box::new(link.clone()),
                vec![],
                denc::RawMessage::new(kind, 2, 1, denc::Bytes::new()),
            );
            let fatal = kind != proto::msg_type::CLIENT_METRICS;
            assert_eq!(server.fatal_error().is_some(), fatal, "kind {kind}");
            dispatch_client_raw(
                &mut server,
                &mut sessions,
                43,
                Box::new(link.clone()),
                vec![],
                open,
            );
            assert_eq!(link.0.lock().unwrap().len(), count + usize::from(!fatal));
            assert_eq!(sessions.len(), 1 + usize::from(!fatal));
        }
    }
}
