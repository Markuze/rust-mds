//! The messenger: one bound msgr1 address, a listener, the session registry and outgoing
//! connections. Follows `src/msg/async/AsyncMessenger.cc`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use auth::{AuthContext, VerifiedAuthorizer};
use denc::{AddrType, EntityAddr, EntityAddrVec, EntityName, EntityType};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::AbortHandle;
use tracing::{debug, info, warn};

use crate::conn::{ConnShared, Phase};
use crate::v1::peer_key;
use crate::{Connection, Event, MsgrConfig, MsgrError, Policy};

type CapsValidator = Arc<dyn Fn(&VerifiedAuthorizer) -> bool + Send + Sync>;
/// Registry key: the peer's legacy socket address and nonce.
type PeerKey = (Option<SocketAddr>, u32);

pub(crate) struct MsgrShared {
    pub cfg: MsgrConfig,
    pub my_name: RwLock<EntityName>,
    pub my_addr: EntityAddr,
    pub auth: Arc<AuthContext>,
    pub events: mpsc::Sender<Event>,
    conns: Mutex<HashMap<PeerKey, Arc<ConnShared>>>,
    all: Mutex<HashMap<u64, std::sync::Weak<ConnShared>>>,
    next_id: AtomicU64,
    global_seq: Mutex<u32>,
    caps_validator: RwLock<Option<CapsValidator>>,
    listener: Mutex<Option<AbortHandle>>,
}

impl MsgrShared {
    pub fn my_name(&self) -> EntityName {
        *self.my_name.read().unwrap()
    }

    /// `AsyncMessenger::get_global_seq(old)`: raise to `old` if larger, then increment.
    pub fn next_global_seq(&self, old: u32) -> u32 {
        let mut g = self.global_seq.lock().unwrap();
        if old > *g {
            *g = old;
        }
        *g += 1;
        *g
    }

    pub fn lookup(&self, peer: &EntityAddr) -> Option<Arc<ConnShared>> {
        self.conns.lock().unwrap().get(&peer_key(peer)).cloned()
    }

    pub fn register(&self, conn: &Arc<ConnShared>) {
        let key = peer_key(&conn.st.lock().unwrap().peer_addr);
        if let Some(old) = self.conns.lock().unwrap().insert(key, conn.clone())
            && old.id != conn.id
        {
            debug!(old = old.id, new = conn.id, "registry: replaced session entry");
        }
    }

    pub fn unregister(&self, conn: &Arc<ConnShared>) {
        let key = peer_key(&conn.st.lock().unwrap().peer_addr);
        let mut m = self.conns.lock().unwrap();
        if m.get(&key).is_some_and(|c| c.id == conn.id) {
            m.remove(&key);
        }
        self.all.lock().unwrap().remove(&conn.id);
    }

    pub fn new_conn(self: &Arc<Self>, policy: Policy, outgoing: bool, peer: EntityAddr, peer_type: EntityType, events: mpsc::Sender<Event>) -> Arc<ConnShared> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let c = ConnShared::new(id, Arc::downgrade(self), policy, outgoing, peer, peer_type, events);
        self.all.lock().unwrap().insert(id, Arc::downgrade(&c));
        c
    }

    pub fn validate_caps(&self, v: &VerifiedAuthorizer) -> bool {
        match &*self.caps_validator.read().unwrap() {
            Some(f) => f(v),
            None => true,
        }
    }

    /// Spawn the connect loop of an outgoing session (with backoff, `ProtocolV1::fault` while
    /// connecting: 0.2 s doubling to 15 s, the maximum after `WAIT`).
    pub fn spawn_connect(self: &Arc<Self>, conn: Arc<ConnShared>, sgen: u64) {
        let msgr = self.clone();
        let conn2 = conn.clone();
        let task = tokio::spawn(async move {
            let mut backoff = Duration::from_millis(200);
            loop {
                {
                    let st = conn.st.lock().unwrap();
                    if st.sgen != sgen || st.phase == Phase::Closed {
                        return;
                    }
                }
                match tokio::time::timeout(msgr.cfg.handshake_timeout, crate::v1::connect(msgr.clone(), conn.clone(), sgen)).await {
                    Ok(Ok(())) => return,
                    Ok(Err(MsgrError::Closed)) => return,
                    Ok(Err(MsgrError::Wait)) => {
                        info!(conn = conn.id, "connect: WAIT (connection race)");
                        backoff = Duration::from_secs(15);
                    }
                    Ok(Err(e)) => {
                        info!(conn = conn.id, "connect failed: {e}");
                    }
                    Err(_) => info!(conn = conn.id, "connect timed out"),
                }
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(15));
            }
        });
        let handle = task.abort_handle();
        conn2.st.lock().unwrap().tasks.push(handle);
    }
}

/// The messenger handle. Cheap to clone.
#[derive(Clone)]
pub struct Messenger(pub(crate) Arc<MsgrShared>);

impl std::fmt::Debug for Messenger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Messenger({} at {})", self.my_name(), self.my_addr())
    }
}

impl Messenger {
    /// Bind a msgr1 (`TYPE_LEGACY`) address with a random nonce and start accepting with the
    /// default configuration for `name`'s entity type. Events of accepted connections (and of
    /// outgoing ones opened with [`connect`](Self::connect)) arrive on the returned receiver.
    pub async fn bind(name: EntityName, addr: SocketAddr, auth: Arc<AuthContext>) -> Result<(Messenger, mpsc::Receiver<Event>), MsgrError> {
        Self::bind_with(MsgrConfig::for_entity(name.ty), name, addr, auth).await
    }

    /// [`bind`](Self::bind) with an explicit configuration.
    pub async fn bind_with(cfg: MsgrConfig, name: EntityName, addr: SocketAddr, auth: Arc<AuthContext>) -> Result<(Messenger, mpsc::Receiver<Event>), MsgrError> {
        let listener = TcpListener::bind(addr).await?;
        let local = listener.local_addr()?;
        if local.ip().is_unspecified() {
            warn!(%local, "bound to an unspecified IP; peers will see a blank address");
        }
        let nonce = cfg.nonce.unwrap_or_else(|| loop {
            let n: u32 = rand::random();
            if n != 0 {
                break n;
            }
        });
        let my_addr = EntityAddr::new(AddrType::LEGACY, local, nonce);
        let (tx, rx) = mpsc::channel(cfg.event_queue);
        let shared = Arc::new(MsgrShared {
            cfg,
            my_name: RwLock::new(name),
            my_addr,
            auth,
            events: tx,
            conns: Mutex::new(HashMap::new()),
            all: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            global_seq: Mutex::new(0),
            caps_validator: RwLock::new(None),
            listener: Mutex::new(None),
        });
        let s = shared.clone();
        let task = tokio::spawn(async move {
            loop {
                let (sock, peer) = match listener.accept().await {
                    Ok(x) => x,
                    Err(e) => {
                        warn!("accept failed: {e}");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };
                let m = s.clone();
                tokio::spawn(async move {
                    match tokio::time::timeout(m.cfg.handshake_timeout, crate::v1::accept(m.clone(), sock, peer)).await {
                        Ok(Ok(())) => {}
                        Ok(Err(e)) => info!(%peer, "accept handshake failed: {e}"),
                        Err(_) => info!(%peer, "accept handshake timed out"),
                    }
                });
            }
        });
        *shared.listener.lock().unwrap() = Some(task.abort_handle());
        info!(name = %name, addr = %my_addr, "messenger bound");
        Ok((Messenger(shared), rx))
    }

    /// Our entity name as stamped into every msgr1 message header.
    pub fn my_name(&self) -> EntityName {
        self.0.my_name()
    }

    /// Change our name (an MDS becomes `mds.<rank>` when it gets a rank).
    pub fn set_my_name(&self, name: EntityName) {
        *self.0.my_name.write().unwrap() = name;
    }

    /// Our legacy address (`v1:ip:port/nonce`).
    pub fn my_addr(&self) -> EntityAddr {
        self.0.my_addr
    }

    /// Our address vector (only the v1 entry for now).
    pub fn my_addrs(&self) -> EntityAddrVec {
        EntityAddrVec(vec![self.0.my_addr])
    }

    pub fn auth(&self) -> &Arc<AuthContext> {
        &self.0.auth
    }

    pub fn config(&self) -> &MsgrConfig {
        &self.0.cfg
    }

    /// Open a connection to `peer`; its events go to the messenger's event channel.
    pub fn connect(&self, peer: EntityAddr, peer_type: EntityType) -> Connection {
        self.connect_with(peer, peer_type, self.0.events.clone())
    }

    /// Open a connection whose events go to `events` (e.g. a MonClient's own channel).
    pub fn connect_with(&self, peer: EntityAddr, peer_type: EntityType, events: mpsc::Sender<Event>) -> Connection {
        let policy = self.0.cfg.policy_for(peer_type);
        let conn = self.0.new_conn(policy, true, peer, peer_type, events);
        if !policy.lossy {
            self.0.register(&conn);
        }
        let sgen = conn.st.lock().unwrap().sgen;
        self.0.spawn_connect(conn.clone(), sgen);
        conn.handle()
    }

    /// Hook run on every verified incoming authorizer before the connection is accepted
    /// (`ms_handle_fast_authentication`); returning false answers `BADAUTHORIZER`.
    pub fn set_caps_validator(&self, f: impl Fn(&VerifiedAuthorizer) -> bool + Send + Sync + 'static) {
        *self.0.caps_validator.write().unwrap() = Some(Arc::new(f));
    }

    /// All live connections.
    pub fn connections(&self) -> Vec<Connection> {
        self.0.all.lock().unwrap().values().filter_map(|w| w.upgrade()).map(|c| c.handle()).collect()
    }

    /// Stop listening and mark every connection down.
    pub fn shutdown(&self) {
        if let Some(l) = self.0.listener.lock().unwrap().take() {
            l.abort();
        }
        for c in self.connections() {
            c.0.mark_down(false);
        }
    }
}
