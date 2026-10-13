//! `MonClient`: one authenticated session with a monitor over msgr1.
//!
//! Follows `src/mon/MonClient.cc`:
//!
//! - hunting (`_reopen_session`, `tick`): connect to one monitor at a time (lossy
//!   connection), rotating through the known v1 addresses; a session that is not established
//!   within `hunt_interval` (× a backoff multiplier, 1.5 per round up to 10) is dropped for
//!   the next monitor;
//! - authentication (`MonConnection::start`, `handle_auth`): keepalive, then `MAuth` with
//!   protocol `CEPH_AUTH_UNKNOWN` listing cephx, then the cephx exchange of
//!   [`auth::CephxClient`]; the global id is kept across sessions (the old auth ticket is
//!   presented for reclaim);
//! - after authentication (`_finish_auth`, `_check_auth_tickets`, `_check_auth_rotating`):
//!   queued messages are flushed, subscriptions re-sent, tickets renewed when due, and the
//!   rotating secrets of our service type fetched when fewer than three are held or the
//!   current one expires within `min(30 s, ttl/4)` (at most once per second);
//! - liveness: a keepalive every `ping_interval` (10 s); no `KEEPALIVE2_ACK` for
//!   `ping_timeout` (30 s), or a reset of the connection, re-hunts;
//! - subscriptions (`MonSub`): `want`, `got`, re-sent on every new session;
//! - consumed messages: `MAuthReply`, `MMonMap`, `MMonSubscribeAck`, `MConfig` (stored and
//!   reported, not applied); everything else from the monitor is passed on (`route` or the
//!   event channel).

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use auth::{AuthContext, Progress};
use bytes::Bytes;
use denc::{AddrType, EntityAddr, EntityType, Message, RawMessage, UTime, Uuid, features};
use msgr::{Connection, Event, Messenger};
use tokio::sync::{mpsc, oneshot, watch};
use tracing::{debug, info, warn};

use crate::messages::*;
use crate::monmap::MonMap;

/// MonClient settings (defaults from `src/common/options/global.yaml.in`).
#[derive(Debug, Clone)]
pub struct MonClientConfig {
    /// Initial monitor addresses (msgr1), e.g. from `mon_host`.
    pub mon_addrs: Vec<SocketAddr>,
    /// `mon_client_hunt_interval`.
    pub hunt_interval: Duration,
    /// `mon_client_ping_interval`.
    pub ping_interval: Duration,
    /// `mon_client_ping_timeout`.
    pub ping_timeout: Duration,
    /// `auth_service_ticket_ttl` in seconds.
    pub service_ticket_ttl: f64,
    /// Short hostname sent with subscriptions.
    pub hostname: String,
}

impl MonClientConfig {
    pub fn new(mon_addrs: Vec<SocketAddr>) -> Self {
        MonClientConfig {
            mon_addrs,
            hunt_interval: Duration::from_secs(3),
            ping_interval: Duration::from_secs(10),
            ping_timeout: Duration::from_secs(30),
            service_ticket_ttl: 3600.0,
            hostname: short_hostname(),
        }
    }
}

/// The machine's short host name (`ceph_get_short_hostname`).
pub fn short_hostname() -> String {
    let h =
        std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_else(|_| "localhost".into());
    h.trim()
        .split('.')
        .next()
        .unwrap_or("localhost")
        .to_string()
}

/// Something the application may care about.
#[derive(Debug)]
pub enum MonEvent {
    /// An authenticated monitor session was established.
    SessionEstablished { global_id: u64, mon: EntityAddr },
    /// The session was lost; the client is hunting.
    SessionLost,
    /// A new MonMap.
    MonMap(MonMap),
    /// Configuration from the monitor's config database (not applied).
    Config(BTreeMap<String, String>),
    /// A message from the monitor that the MonClient does not consume (`MMDSMap`, ...).
    Message(RawMessage),
}

/// Observable status.
#[derive(Debug, Clone, Default)]
pub struct MonStatus {
    pub authenticated: bool,
    pub global_id: u64,
    pub have_rotating: bool,
    pub sessions: u64,
    pub auth_error: Option<String>,
    pub fsid: Option<Uuid>,
    pub monmap_epoch: u32,
}

enum Cmd {
    Send(RawMessage),
    Want {
        what: String,
        start: u64,
        flags: u8,
    },
    Got {
        what: String,
        epoch: u64,
    },
    Route {
        msg_type: u16,
        tx: mpsc::Sender<RawMessage>,
    },
    /// Run a monitor command (`MonClient::start_command`): send `MMonCommand`
    /// with the next tid and complete the caller when the ack arrives.
    Command {
        cmd: Vec<String>,
        tx: oneshot::Sender<Result<(i32, String), String>>,
    },
    Shutdown,
}

/// Handle on the MonClient task.
#[derive(Clone)]
pub struct MonClient {
    cmd: mpsc::UnboundedSender<Cmd>,
    status: watch::Receiver<MonStatus>,
    shared: Arc<Shared>,
}

struct Shared {
    monmap: Mutex<MonMap>,
    config: Mutex<BTreeMap<String, String>>,
    auth: Arc<AuthContext>,
    msgr: Messenger,
}

impl std::fmt::Debug for MonClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MonClient({:?})", *self.status.borrow())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessPhase {
    Negotiating,
    Authenticating,
    HaveSession,
}

struct Session {
    conn: Connection,
    addr: EntityAddr,
    phase: SessPhase,
    started: Instant,
}

/// `MonSub`.
#[derive(Debug, Default)]
struct MonSub {
    new: BTreeMap<String, (u64, u8)>,
    sent: BTreeMap<String, (u64, u8)>,
    renew_after: Option<Instant>,
}

impl MonSub {
    fn want(&mut self, what: &str, start: u64, flags: u8) -> bool {
        if self.new.get(what) == Some(&(start, flags))
            || self.sent.get(what) == Some(&(start, flags))
        {
            return false;
        }
        self.new.insert(what.to_string(), (start, flags));
        true
    }

    fn got(&mut self, what: &str, have: u64) {
        for m in [&mut self.new, &mut self.sent] {
            if let Some((start, flags)) = m.get(what).copied() {
                if flags & SUBSCRIBE_ONETIME != 0 {
                    if have >= start {
                        m.remove(what);
                    }
                } else if start <= have {
                    m.insert(what.to_string(), (have + 1, flags));
                }
            }
        }
    }

    fn reload(&mut self) -> bool {
        for (k, v) in std::mem::take(&mut self.sent) {
            self.new.entry(k).or_insert(v);
        }
        !self.new.is_empty()
    }

    fn renewed(&mut self) -> BTreeMap<String, (u64, u8)> {
        let mut all = self.sent.clone();
        for (k, v) in std::mem::take(&mut self.new) {
            self.sent.insert(k.clone(), v);
            all.insert(k, v);
        }
        all
    }
}

impl MonClient {
    /// Start the MonClient task on `msgr` (whose [`AuthContext`] holds our key). Hunting
    /// starts immediately; also subscribes to `monmap` and `config` (`MonClient::authenticate`).
    pub fn start(msgr: Messenger, cfg: MonClientConfig) -> (MonClient, mpsc::Receiver<MonEvent>) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = watch::channel(MonStatus::default());
        let (ev_tx, ev_rx) = mpsc::channel(1024);
        let shared = Arc::new(Shared {
            monmap: Mutex::new(MonMap::default()),
            config: Mutex::new(BTreeMap::new()),
            auth: msgr.auth().clone(),
            msgr: msgr.clone(),
        });
        let mut subs = MonSub::default();
        subs.want("monmap", 0, 0);
        subs.want("config", 0, 0);
        let (conn_tx, conn_rx) = mpsc::channel(1024);
        let task = Task {
            cfg,
            shared: shared.clone(),
            status: status_tx,
            events: ev_tx,
            cmd_rx,
            conn_tx,
            conn_rx,
            session: None,
            had_session: false,
            hunt_multiplier: 1.0,
            next_mon: 0,
            waiting: VecDeque::new(),
            subs,
            routes: HashMap::new(),
            commands: HashMap::new(),
            next_command_tid: 0,
            last_rotating_sent: None,
            last_tickets_sent: None,
            last_keepalive: Instant::now(),
            stopping: false,
        };
        tokio::spawn(task.run());
        (
            MonClient {
                cmd: cmd_tx,
                status: status_rx,
                shared,
            },
            ev_rx,
        )
    }

    /// Current status.
    pub fn status(&self) -> MonStatus {
        self.status.borrow().clone()
    }

    /// Our global id (0 before the first authentication).
    pub fn global_id(&self) -> u64 {
        self.status.borrow().global_id
    }

    /// The cluster fsid (from the MonMap), once known.
    pub fn fsid(&self) -> Option<Uuid> {
        self.status.borrow().fsid
    }

    pub fn monmap(&self) -> MonMap {
        self.shared.monmap.lock().unwrap().clone()
    }

    /// Configuration received in `MConfig`.
    pub fn config(&self) -> BTreeMap<String, String> {
        self.shared.config.lock().unwrap().clone()
    }

    pub fn messenger(&self) -> &Messenger {
        &self.shared.msgr
    }

    pub fn auth(&self) -> &Arc<AuthContext> {
        &self.shared.auth
    }

    async fn wait_for(
        &self,
        timeout: Duration,
        mut pred: impl FnMut(&MonStatus) -> Option<Result<(), String>>,
    ) -> Result<MonStatus, String> {
        let mut rx = self.status.clone();
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            {
                let s = rx.borrow_and_update();
                if let Some(r) = pred(&s) {
                    return r.map(|_| s.clone());
                }
            }
            match tokio::time::timeout_at(deadline, rx.changed()).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => return Err("MonClient stopped".into()),
                Err(_) => return Err("timed out".into()),
            }
        }
    }

    /// Wait until authenticated (`MonClient::authenticate`); returns the global id.
    pub async fn wait_authenticated(&self, timeout: Duration) -> Result<u64, String> {
        // Authentication errors are retried with the next monitor (hunting), so only the
        // timeout ends the wait; the last error is reported with it.
        let s = self
            .wait_for(timeout, |s| s.authenticated.then_some(Ok(())))
            .await
            .map_err(|e| match self.status().auth_error {
                Some(a) => format!("{e} (last auth error: {a})"),
                None => e,
            })?;
        Ok(s.global_id)
    }

    /// Wait until rotating secrets are held (`wait_auth_rotating`).
    pub async fn wait_rotating(&self, timeout: Duration) -> Result<(), String> {
        self.wait_for(timeout, |s| s.have_rotating.then_some(Ok(())))
            .await
            .map(|_| ())
    }

    /// Send a message to the monitor (`send_mon_message`): queued while there is no session,
    /// dropped when a new hunt starts.
    pub fn send(&self, msg: RawMessage) {
        let _ = self.cmd.send(Cmd::Send(msg));
    }

    /// Subscribe (`sub_want` + `renew_subs`).
    pub fn subscribe(&self, what: &str, start: u64, flags: u8) {
        let _ = self.cmd.send(Cmd::Want {
            what: what.to_string(),
            start,
            flags,
        });
    }

    /// Note that epoch `epoch` of `what` was received (`sub_got`).
    pub fn sub_got(&self, what: &str, epoch: u64) {
        let _ = self.cmd.send(Cmd::Got {
            what: what.to_string(),
            epoch,
        });
    }

    /// Deliver monitor messages of `msg_type` to `tx` instead of the event channel.
    pub fn route(&self, msg_type: u16, tx: mpsc::Sender<RawMessage>) {
        let _ = self.cmd.send(Cmd::Route { msg_type, tx });
    }

    /// Run a monitor command (`MonClient::start_command`) and return `(r, rs)`.
    /// A lost monitor session fails the command; the caller may retry.
    pub async fn command(
        &self,
        cmd: Vec<String>,
        timeout: Duration,
    ) -> Result<(i32, String), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd
            .send(Cmd::Command { cmd, tx })
            .map_err(|_| "MonClient stopped".to_string())?;
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(r)) => r,
            Ok(Err(_)) => Err("command dropped".into()),
            Err(_) => Err("command timed out".into()),
        }
    }

    /// Blocklist `addr` (`MDSRank::evict_client`: `osd blocklist add`, no
    /// expiry, so `mon_osd_blocklist_default_expire` applies).
    pub async fn blocklist(&self, addr: &EntityAddr) -> Result<(i32, String), String> {
        let cmd = vec![format!(
            "{{\"prefix\":\"osd blocklist\", \"blocklistop\":\"add\",\"addr\":\"{addr}\"}}"
        )];
        self.command(cmd, Duration::from_secs(30)).await
    }

    pub fn shutdown(&self) {
        let _ = self.cmd.send(Cmd::Shutdown);
    }
}

struct Task {
    cfg: MonClientConfig,
    shared: Arc<Shared>,
    status: watch::Sender<MonStatus>,
    events: mpsc::Sender<MonEvent>,
    cmd_rx: mpsc::UnboundedReceiver<Cmd>,
    conn_tx: mpsc::Sender<Event>,
    conn_rx: mpsc::Receiver<Event>,
    session: Option<Session>,
    had_session: bool,
    hunt_multiplier: f64,
    next_mon: usize,
    waiting: VecDeque<RawMessage>,
    subs: MonSub,
    routes: HashMap<u16, mpsc::Sender<RawMessage>>,
    /// Monitor commands in flight (`MonClient::command`): tid -> caller.
    commands: HashMap<u64, oneshot::Sender<Result<(i32, String), String>>>,
    next_command_tid: u64,
    last_rotating_sent: Option<Instant>,
    last_tickets_sent: Option<Instant>,
    last_keepalive: Instant,
    stopping: bool,
}

impl Task {
    async fn run(mut self) {
        self.reopen_session();
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        while !self.stopping {
            tokio::select! {
                ev = self.conn_rx.recv() => match ev {
                    Some(ev) => self.handle_event(ev).await,
                    None => break,
                },
                cmd = self.cmd_rx.recv() => match cmd {
                    Some(c) => self.handle_cmd(c),
                    None => break,
                },
                _ = tick.tick() => self.tick().await,
            }
        }
        if let Some(s) = self.session.take() {
            s.conn.mark_down();
        }
        info!("monclient stopped");
    }

    fn update_status(&self, f: impl FnOnce(&mut MonStatus)) {
        self.status.send_modify(f);
    }

    fn mon_addrs(&self) -> Vec<EntityAddr> {
        let mm = self.shared.monmap.lock().unwrap();
        let v = mm.v1_addrs();
        if !v.is_empty() {
            return v;
        }
        self.cfg
            .mon_addrs
            .iter()
            .map(|a| EntityAddr::new(AddrType::LEGACY, *a, 0))
            .collect()
    }

    /// `_reopen_session`: drop the current connection, pick the next monitor, start the
    /// handshake, throw away queued messages.
    fn reopen_session(&mut self) {
        if let Some(s) = self.session.take() {
            s.conn.mark_down();
            if s.phase == SessPhase::HaveSession {
                let _ = self.events.try_send(MonEvent::SessionLost);
            }
            self.update_status(|st| st.authenticated = false);
        }
        if self.had_session {
            self.hunt_multiplier = (self.hunt_multiplier * 1.5).min(10.0);
        }
        let addrs = self.mon_addrs();
        if addrs.is_empty() {
            warn!("no monitor addresses known");
            return;
        }
        let addr = addrs[self.next_mon % addrs.len()];
        self.next_mon = self.next_mon.wrapping_add(1);
        self.waiting.clear();
        // A command that did not get its ack on this connection fails; the caller retries.
        for (tid, tx) in self.commands.drain() {
            let _ = tx.send(Err("monitor session lost".into()));
            debug!(tid, "mon command failed: session lost");
        }
        info!(mon = %addr, "hunting: connecting to monitor");
        let conn = self
            .shared
            .msgr
            .connect_with(addr, EntityType::MON, self.conn_tx.clone());
        // MonConnection::start: a keepalive first, then MAuth with CEPH_AUTH_UNKNOWN.
        conn.send_keepalive();
        let (name, global_id) = {
            let c = self.shared.auth.client();
            (c.name().clone(), c.global_id())
        };
        let payload = auth::client::initial_mauth_payload(&name, global_id, &[auth::AUTH_CEPHX]);
        let epoch = self.shared.monmap.lock().unwrap().epoch;
        conn.send(
            MAuth {
                protocol: auth::AUTH_UNKNOWN,
                auth_payload: payload,
                monmap_epoch: epoch,
            }
            .to_raw(features::ALL),
        );
        self.last_keepalive = Instant::now();
        self.session = Some(Session {
            conn,
            addr,
            phase: SessPhase::Negotiating,
            started: Instant::now(),
        });
    }

    fn send_now(&mut self, msg: RawMessage) {
        match &self.session {
            Some(s) if s.phase == SessPhase::HaveSession => {
                s.conn.send(msg);
            }
            _ => self.waiting.push_back(msg),
        }
    }

    fn send_auth(&self, protocol: u32, payload: Bytes) {
        if let Some(s) = &self.session {
            let epoch = self.shared.monmap.lock().unwrap().epoch;
            s.conn.send(
                MAuth {
                    protocol,
                    auth_payload: payload,
                    monmap_epoch: epoch,
                }
                .to_raw(features::ALL),
            );
        }
    }

    fn renew_subs(&mut self) {
        if self.subs.new.is_empty() {
            return;
        }
        if !matches!(&self.session, Some(s) if s.phase == SessPhase::HaveSession) {
            return;
        }
        let what = self.subs.renewed();
        debug!(?what, "renew subscriptions");
        let m = MMonSubscribe {
            what,
            hostname: self.cfg.hostname.clone(),
        };
        if let Some(s) = &self.session {
            s.conn.send(m.to_raw(features::ALL));
        }
    }

    fn handle_cmd(&mut self, c: Cmd) {
        match c {
            Cmd::Send(m) => self.send_now(m),
            Cmd::Want { what, start, flags } => {
                if self.subs.want(&what, start, flags) {
                    self.renew_subs();
                }
            }
            Cmd::Got { what, epoch } => self.subs.got(&what, epoch),
            Cmd::Route { msg_type, tx } => {
                self.routes.insert(msg_type, tx);
            }
            Cmd::Command { cmd, tx } => {
                self.next_command_tid += 1;
                let tid = self.next_command_tid;
                let fsid = self.status.borrow().fsid.unwrap_or_default();
                let mut m = MMonCommand { fsid, cmd }.to_raw(features::ALL);
                m.tid = tid;
                debug!(tid, "mon command");
                self.commands.insert(tid, tx);
                self.send_now(m);
            }
            Cmd::Shutdown => self.stopping = true,
        }
    }

    async fn handle_event(&mut self, ev: Event) {
        let is_current = self.session.as_ref().is_some_and(|s| &s.conn == ev.conn());
        match ev {
            Event::Connected { conn } if is_current => {
                debug!(conn = conn.id(), "monitor connection established");
            }
            Event::Reset { conn } | Event::Closed { conn } if is_current => {
                info!(conn = conn.id(), "monitor connection lost; re-hunting");
                self.reopen_session();
            }
            Event::Message { msg, .. } if is_current => self.handle_message(msg).await,
            Event::Message { msg, conn } => {
                debug!(
                    conn = conn.id(),
                    msg_type = msg.msg_type,
                    "discarding stray monitor message"
                );
            }
            _ => {}
        }
    }

    async fn handle_message(&mut self, msg: RawMessage) {
        match msg.msg_type {
            CEPH_MSG_AUTH_REPLY => match MAuthReply::from_raw(msg) {
                Ok(r) => self.handle_auth_reply(r),
                Err(e) => warn!("bad MAuthReply: {e}"),
            },
            CEPH_MSG_MON_MAP => match MMonMap::from_raw(msg) {
                Ok(m) => self.handle_monmap(m).await,
                Err(e) => warn!("bad MMonMap: {e}"),
            },
            CEPH_MSG_MON_SUBSCRIBE_ACK => {
                if let Ok(a) = MMonSubscribeAck::from_raw(msg) {
                    debug!(interval = a.interval, "subscribe ack");
                    let has_stateful = self.session.as_ref().is_some_and(|s| s.conn.features() & features::MON_STATEFUL_SUB != 0);
                    if !has_stateful {
                        self.subs.renew_after = Some(Instant::now() + Duration::from_secs(u64::from(a.interval / 2).max(1)));
                    }
                }
            }
            CEPH_MSG_MON_COMMAND_ACK => {
                let tid = msg.tid;
                match MMonCommandAck::from_raw(msg) {
                    Ok(a) => {
                        if let Some(tx) = self.commands.remove(&tid) {
                            debug!(tid, r = a.r, "mon command ack");
                            let _ = tx.send(Ok((a.r, a.rs)));
                        } else {
                            debug!(tid, "mon command ack without a waiter");
                        }
                    }
                    Err(e) => warn!(tid, "bad MMonCommandAck: {e}"),
                }
            }
            MSG_CONFIG => match MConfig::from_raw(msg) {
                Ok(c) => {
                    debug!(keys = c.config.len(), "config from monitor (not applied)");
                    *self.shared.config.lock().unwrap() = c.config.clone();
                    self.subs.got("config", 0);
                    let _ = self.events.send(MonEvent::Config(c.config)).await;
                }
                Err(e) => warn!("bad MConfig: {e}"),
            },
            2 /* CEPH_MSG_PING */ => {}
            t => {
                if let Some(tx) = self.routes.get(&t) {
                    if tx.send(msg).await.is_err() {
                        self.routes.remove(&t);
                    }
                } else {
                    let _ = self.events.send(MonEvent::Message(msg)).await;
                }
            }
        }
    }

    fn handle_auth_reply(&mut self, r: MAuthReply) {
        let Some(sess) = self.session.as_mut() else {
            return;
        };
        debug!(
            protocol = r.protocol,
            result = r.result,
            global_id = r.global_id,
            len = r.result_bl.len(),
            "MAuthReply"
        );
        if sess.phase == SessPhase::Negotiating {
            // _negotiate / _init_auth: the server picked a protocol.
            if r.protocol != auth::AUTH_CEPHX {
                let err = format!(
                    "monitor chose auth protocol {} (result {}): {}",
                    r.protocol, r.result, r.result_msg
                );
                warn!("{err}");
                self.update_status(|s| s.auth_error = Some(err));
                return;
            }
            self.shared.auth.client().reset();
            sess.phase = SessPhase::Authenticating;
        }
        let result = {
            let mut c = self.shared.auth.client();
            if r.global_id != c.global_id() {
                // A new session id from the monitor (MonConnection::authenticate).
                if c.global_id() != 0 {
                    warn!(
                        old = c.global_id(),
                        new = r.global_id,
                        "monitor assigned a different global_id"
                    );
                }
                c.set_global_id(r.global_id);
            }
            c.handle_response(r.result, r.result_bl.clone())
        };
        match result {
            Ok(Progress::Continue) => {
                let req = self.shared.auth.client().build_request();
                match req {
                    Ok(p) => self.send_auth(auth::AUTH_CEPHX, p),
                    Err(e) => warn!("cannot build auth request: {e}"),
                }
            }
            Ok(Progress::Done) => {
                if sess.phase != SessPhase::HaveSession {
                    self.finish_hunting(r.global_id);
                } else {
                    debug!("tickets renewed");
                }
                self.check_auth_tickets();
            }
            Ok(Progress::Rotating(secrets)) => {
                info!(
                    n = secrets.secrets.len(),
                    max_ver = secrets.max_ver,
                    "got rotating secrets"
                );
                self.shared.auth.set_rotating(secrets);
                let have = !self.shared.auth.need_rotating(self.rotating_cutoff());
                self.update_status(|s| s.have_rotating = have);
            }
            Err(e) => {
                let err = format!("authentication failed: {e} {}", r.result_msg);
                warn!("{err}");
                self.update_status(|s| s.auth_error = Some(err));
                // _finish_auth with an error: drop this monitor; the next tick re-hunts.
                if let Some(s) = self.session.as_mut()
                    && s.phase != SessPhase::HaveSession
                {
                    s.started =
                        Instant::now() - self.cfg.hunt_interval.mul_f64(self.hunt_multiplier);
                }
            }
        }
    }

    /// `_finish_hunting` + `_finish_auth(0)`.
    fn finish_hunting(&mut self, global_id: u64) {
        let Some(sess) = self.session.as_mut() else {
            return;
        };
        sess.phase = SessPhase::HaveSession;
        let addr = sess.addr;
        info!(mon = %addr, global_id, "authenticated with monitor");
        self.had_session = true;
        self.hunt_multiplier = (self.hunt_multiplier / 1.5).max(1.0);
        self.last_rotating_sent = None;
        for m in std::mem::take(&mut self.waiting) {
            if let Some(s) = &self.session {
                s.conn.send(m);
            }
        }
        self.subs.reload();
        self.renew_subs();
        self.update_status(|s| {
            s.authenticated = true;
            s.global_id = global_id;
            s.sessions += 1;
            s.auth_error = None;
        });
        let _ = self.events.try_send(MonEvent::SessionEstablished {
            global_id,
            mon: addr,
        });
    }

    fn rotating_cutoff(&self) -> UTime {
        auth::utime_add_secs(
            auth::utime_now(),
            -(30f64.min(self.cfg.service_ticket_ttl / 4.0)),
        )
    }

    /// `_check_auth_tickets` + `_check_auth_rotating`.
    fn check_auth_tickets(&mut self) {
        let Some(sess) = &self.session else { return };
        if sess.phase != SessPhase::HaveSession {
            return;
        }
        let now = Instant::now();
        let need = self.shared.auth.client().need_tickets();
        if need
            && self
                .last_tickets_sent
                .is_none_or(|t| now.duration_since(t) >= Duration::from_secs(1))
        {
            info!("renewing tickets");
            if let Ok(p) = self.shared.auth.client().build_request() {
                self.send_auth(auth::AUTH_CEPHX, p);
                self.last_tickets_sent = Some(now);
            }
        }
        // Only OSD, MDS and MGR daemons hold rotating secrets.
        let ty = self.shared.auth.name().ty;
        if !matches!(ty, EntityType::OSD | EntityType::MDS | EntityType::MGR) {
            return;
        }
        let cutoff = self.rotating_cutoff();
        if !self.shared.auth.need_rotating(cutoff) {
            return;
        }
        if self
            .last_rotating_sent
            .is_some_and(|t| now.duration_since(t) < Duration::from_secs(1))
        {
            return;
        }
        debug!("requesting rotating secrets");
        let p = self.shared.auth.client().build_rotating_request();
        self.send_auth(auth::AUTH_CEPHX, p);
        self.last_rotating_sent = Some(now);
    }

    async fn handle_monmap(&mut self, m: MMonMap) {
        let mut b = m.monmap.clone();
        let mm = match <MonMap as denc::Decode>::decode(&mut b) {
            Ok(mm) => mm,
            Err(e) => {
                warn!("cannot decode MonMap: {e}");
                return;
            }
        };
        info!(epoch = mm.epoch, mons = mm.mons.len(), fsid = ?mm.fsid, "got monmap");
        let old_auth_epoch = {
            let mut cur = self.shared.monmap.lock().unwrap();
            let old = cur.auth_epoch;
            *cur = mm.clone();
            old
        };
        if mm.auth_epoch > old_auth_epoch {
            info!("auth epoch changed: wiping rotating secrets and tickets");
            self.shared.auth.wipe_secrets_and_tickets();
            self.update_status(|s| s.have_rotating = false);
            self.check_auth_tickets();
        }
        self.subs.got("monmap", u64::from(mm.epoch));
        let (fsid, epoch) = (mm.fsid, mm.epoch);
        self.update_status(|s| {
            s.fsid = Some(fsid);
            s.monmap_epoch = epoch;
        });
        let _ = self.events.send(MonEvent::MonMap(mm)).await;
    }

    async fn tick(&mut self) {
        let now = Instant::now();
        let hunting = !matches!(&self.session, Some(s) if s.phase == SessPhase::HaveSession);
        if hunting {
            let limit = self.cfg.hunt_interval.mul_f64(self.hunt_multiplier);
            if self
                .session
                .as_ref()
                .is_none_or(|s| now.duration_since(s.started) >= limit)
            {
                info!("continuing hunt");
                self.reopen_session();
            }
            return;
        }
        self.check_auth_tickets();
        if self.subs.renew_after.is_some_and(|t| now >= t) {
            self.subs.renew_after = None;
            self.subs.reload();
            self.renew_subs();
        }
        if now.duration_since(self.last_keepalive) >= self.cfg.ping_interval {
            self.last_keepalive = now;
            let mut lost = false;
            if let Some(s) = &self.session {
                s.conn.send_keepalive();
                if s.conn.features() & features::MSGR_KEEPALIVE2 != 0 {
                    let last = s.conn.last_keepalive_ack();
                    let since = last
                        .map(|t| f64::from(auth::utime_now().sec.saturating_sub(t.sec)))
                        .unwrap_or(f64::MAX);
                    let session_age = now.duration_since(s.started).as_secs_f64();
                    if since > self.cfg.ping_timeout.as_secs_f64()
                        && session_age > self.cfg.ping_timeout.as_secs_f64()
                    {
                        warn!(since, "no keepalive ack from monitor; reconnecting");
                        lost = true;
                    }
                }
            }
            if lost {
                self.reopen_session();
            }
        }
        let have = !self.shared.auth.need_rotating(self.rotating_cutoff());
        if have != self.status.borrow().have_rotating {
            self.update_status(|s| s.have_rotating = have);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monsub_semantics() {
        let mut s = MonSub::default();
        assert!(s.want("mdsmap", 0, 0));
        assert!(!s.want("mdsmap", 0, 0));
        assert!(s.want("osdmap", 5, SUBSCRIBE_ONETIME));
        let sent = s.renewed();
        assert_eq!(sent.len(), 2);
        assert!(s.new.is_empty());
        s.got("mdsmap", 7);
        assert_eq!(s.sent["mdsmap"], (8, 0));
        s.got("osdmap", 6);
        assert!(!s.sent.contains_key("osdmap"));
        assert!(s.reload());
        assert_eq!(s.new["mdsmap"], (8, 0));
    }
}
