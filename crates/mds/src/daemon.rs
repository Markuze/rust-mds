//! The daemon: cluster membership (monitor session, beacons, the rank state machine) and the
//! routing of client messages to the session layer and the metadata server.
//!
//! One task runs the loop below. Reading connections is separate (the messenger delivers
//! events on a bounded channel), and every message to a client goes through its session's
//! funnel ([`crate::session`]). The metadata server is created when the rank is assigned.

use std::collections::{BTreeMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use auth::{AuthContext, AuthName, Keyring};
use denc::{EntityName, EntityType, Uuid};
use monc::{AddrsString, Beacon, BeaconConfig, MdsMap, MdsState, MonClient, MonClientConfig, MonEvent};
use msgr::{Connection, Event, Messenger};
use proto::ClientMessage;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::server::Server;
use crate::session::{SessionMap, SessionOutcome};

/// The Ceph version string reported in `sys_info`. Rook compares it with the image's
/// `ceph --version` (F-ADM-116); it matches the dev cluster's packages for now.
const CEPH_VERSION: &str = "ceph version 20.2.0 (69f84cc2651aa259a15bc192ddaabd3baba07489) tentacle (stable - RelWithDebInfo)";
const CEPH_VERSION_SHORT: &str = "20.2.0";

fn sys_info(hostname: &str, addr: &str) -> BTreeMap<String, String> {
    let read = |p: &str| std::fs::read_to_string(p).map(|s| s.trim().to_string()).unwrap_or_default();
    let mut m = BTreeMap::new();
    m.insert("addr".into(), addr.to_string());
    m.insert("hostname".into(), hostname.to_string());
    m.insert("ceph_version".into(), CEPH_VERSION.into());
    m.insert("ceph_version_short".into(), CEPH_VERSION_SHORT.into());
    m.insert("ceph_release".into(), "tentacle".into());
    m.insert("os".into(), "Linux".into());
    m.insert("arch".into(), std::env::consts::ARCH.into());
    m.insert("kernel_version".into(), read("/proc/sys/kernel/osrelease"));
    m.insert("kernel_description".into(), read("/proc/sys/kernel/version"));
    m.insert("mds_impl".into(), format!("rust-mds {}", env!("CARGO_PKG_VERSION")));
    m
}

/// The state to request once the phase of `state` is done. Without persistence every phase is
/// immediate: creating/starting go to active, and a replaying rank walks reconnect and rejoin
/// with a fresh namespace (logged loudly: nothing was persisted).
fn next_request(state: MdsState) -> Option<MdsState> {
    match state {
        MdsState::CREATING | MdsState::STARTING => Some(MdsState::ACTIVE),
        MdsState::REPLAY | MdsState::RESOLVE => Some(MdsState::RECONNECT),
        MdsState::RECONNECT => Some(MdsState::REJOIN),
        MdsState::REJOIN | MdsState::CLIENTREPLAY => Some(MdsState::ACTIVE),
        MdsState::STOPPING => Some(MdsState::STOPPED),
        _ => None,
    }
}

/// Bind the messenger: the configured address, or the monitors' IP with the first free port of
/// `ms_bind_port_min..=ms_bind_port_max`.
async fn bind_messenger(cfg: &Config, mons: &[SocketAddr], auth: std::sync::Arc<AuthContext>) -> Result<(Messenger, tokio::sync::mpsc::Receiver<Event>)> {
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
    Err(anyhow!("cannot bind on {ip} in ports {}..={}: {:?}", cfg.bind_port_min, cfg.bind_port_max, last_err))
}

pub async fn run(cfg: Config) -> Result<()> {
    let mons = monc::parse_mon_host_v1(&cfg.mon_host);
    if mons.is_empty() {
        bail!("no msgr1 (v1) monitor address in mon_host = {}", cfg.mon_host);
    }
    let keyring = Keyring::load(&cfg.keyring).map_err(|e| anyhow!("keyring {}: {e}", cfg.keyring.display()))?;
    let name = AuthName::new(EntityType::MDS, cfg.id.clone());
    let key = keyring.key(&name).with_context(|| format!("no key for {name} in {}", cfg.keyring.display()))?.clone();
    let auth = AuthContext::new(name.clone(), key, auth::ENTITY_MON | auth::ENTITY_OSD | auth::ENTITY_MDS | auth::ENTITY_MGR);

    let (msgr, mut msgr_events) = bind_messenger(&cfg, &mons, auth.clone()).await?;
    info!(addr = %msgr.my_addrs_string(), "bound");
    // A client must present a decodable mds cap (an empty cap blob is refused, as in C++).
    msgr.set_caps_validator(|v| v.caps.allow_all || v.caps.cap_string().is_some());

    let (monc, mut mon_events) = MonClient::start(msgr.clone(), MonClientConfig::new(mons));
    let gid = monc.wait_authenticated(Duration::from_secs(60)).await.map_err(|e| anyhow!("authentication: {e}"))?;
    info!(%name, gid, "authenticated to the monitor");
    monc.wait_rotating(Duration::from_secs(30)).await.map_err(|e| anyhow!("rotating secrets: {e}"))?;
    monc.subscribe("mdsmap", 0, 0);

    let fsid = match monc.fsid().or_else(|| cfg.conf.get(&cfg.id, "fsid").and_then(Uuid::parse)) {
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
    };

    let mut tick = tokio::time::interval(Duration::from_secs(1));
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
            ev = mon_events.recv() => {
                let ev = ev.ok_or_else(|| anyhow!("monitor client stopped"))?;
                d.on_mon_event(ev)?;
            }
            ev = msgr_events.recv() => {
                let ev = ev.ok_or_else(|| anyhow!("messenger stopped"))?;
                d.on_msgr_event(ev);
            }
            _ = tick.tick() => d.on_tick(),
            _ = tokio::signal::ctrl_c() => { d.shutdown().await; return Ok(()); }
            _ = sigterm.recv() => { d.shutdown().await; return Ok(()); }
        }
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
    held: VecDeque<(Connection, denc::RawMessage)>,
}

impl Daemon {
    fn on_mon_event(&mut self, ev: MonEvent) -> Result<()> {
        match ev {
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
            MonEvent::SessionEstablished { global_id, mon } => info!(%mon, global_id, "monitor session established"),
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
            warn!(mask = format!("{:#x}", map.compat.incompat.mask()), "file system compat not writeable by this daemon");
        }
        if new_state != MdsState::NULL {
            self.seen_in_map = true;
        }
        let info = map.info(gid).cloned();
        let prev = self.state;
        self.state = new_state;
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
                    self.msgr.set_my_name(EntityName::new(EntityType::MDS, i64::from(i.rank)));
                }
                if self.server.is_none() && s != MdsState::STANDBY_REPLAY {
                    if s != MdsState::CREATING {
                        warn!(state = %s, "no persistence yet: serving a fresh, empty namespace");
                    }
                    let data_pool = map.data_pools.first().copied().unwrap_or(-1);
                    self.server = Some(Server::new(map.fs_name.clone(), data_pool, map.epoch));
                }
                if let Some(server) = &mut self.server {
                    server.set_mdsmap(&map);
                }
                self.beacon.set_want_state(map.epoch, s);
                if prev != s
                    && let Some(next) = next_request(s)
                {
                    info!(epoch = map.epoch, request = %next, "phase done");
                    self.beacon.set_want_state(map.epoch, next);
                    self.beacon.send_now();
                }
                if s == MdsState::ACTIVE && prev != s {
                    info!(rank = info.as_ref().map(|i| i.rank).unwrap_or(-1), fs = %map.fs_name, addr = %self.msgr.my_addrs_string(), "active");
                }
            }
        }
        self.mdsmap = Some(map);
        if self.is_serving() {
            self.release_held();
        }
        Ok(())
    }

    /// Requests are served in `up:active` (and `up:clientreplay`, `up:stopping`); session
    /// messages from `up:reconnect` on.
    fn is_serving(&self) -> bool {
        matches!(self.state, MdsState::ACTIVE | MdsState::CLIENTREPLAY | MdsState::STOPPING) && self.server.is_some()
    }

    fn release_held(&mut self) {
        let held: Vec<_> = self.held.drain(..).collect();
        for (conn, msg) in held {
            self.on_client_raw(conn, msg);
        }
    }

    fn on_msgr_event(&mut self, ev: Event) {
        match ev {
            Event::Accepted { conn } => debug!(
                conn = conn.id(),
                peer = %conn.peer_addr(),
                who = %conn.peer_auth_name().map(|n| n.to_string()).unwrap_or_default(),
                "accepted"
            ),
            Event::Message { conn, msg } => {
                if conn.peer_type() == EntityType::CLIENT {
                    self.on_client_raw(conn, msg);
                } else {
                    debug!(peer = conn.peer_type().name(), msg_type = msg.msg_type, "non-client message ignored");
                }
            }
            Event::Reset { conn } | Event::Closed { conn } => debug!(conn = conn.id(), "connection gone"),
            Event::Connected { .. } => {}
        }
    }

    fn on_client_raw(&mut self, conn: Connection, raw: denc::RawMessage) {
        if !self.is_serving() {
            // Clients only talk to a rank the map shows active; hold anything early.
            self.held.push_back((conn, raw));
            return;
        }
        let client = conn.peer_global_id();
        let msg_type = raw.msg_type;
        let msg = match ClientMessage::from_raw(raw) {
            Ok(Some(m)) => m,
            Ok(None) => {
                debug!(client, msg_type, "message type clients do not send; dropped");
                return;
            }
            Err(e) => {
                warn!(client, msg_type, "undecodable client message: {e}");
                return;
            }
        };
        let server = self.server.as_mut().expect("serving implies a server");
        match msg {
            ClientMessage::Session(m) => {
                let cap_auths = server.cap_auths_for(&conn);
                match self.sessions.handle(client, Box::new(conn.clone()), m, cap_auths) {
                    SessionOutcome::Opened => server.session_opened(&mut self.sessions, client),
                    SessionOutcome::Closed => server.session_closed(&mut self.sessions, client),
                    SessionOutcome::Done => {}
                }
            }
            other => {
                if self.sessions.get(client).is_none_or(|s| !s.is_open()) {
                    debug!(client, "message without an open session; dropped");
                    return;
                }
                server.handle(&mut self.sessions, client, other);
            }
        }
    }

    fn on_tick(&mut self) {
        if let Some(server) = &mut self.server {
            server.tick(&mut self.sessions);
        }
    }

    async fn shutdown(&mut self) {
        info!("shutting down: down:dne beacon");
        self.beacon.set_want_state(self.last_epoch, MdsState::DNE);
        self.beacon.send_now();
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.beacon.stop();
        self.monc.shutdown();
        self.msgr.shutdown();
    }
}
