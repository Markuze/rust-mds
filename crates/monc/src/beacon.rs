//! The MDS beacon (`src/mds/Beacon.cc`).
//!
//! A task of its own sends an `MMDSBeacon` every `interval` (4 s, `mds_beacon_interval`;
//! a beacon is due once 0.9 × interval passed since the last one) through the MonClient, so
//! metadata work never delays it. Each beacon carries the daemon's wanted state together with
//! the epoch of the last MDSMap applied (`set_want_state` updates both at once, so the monitor
//! never sees a new epoch with a stale state), a strictly increasing `seq`, the constant compat
//! set, and `sys_info` while booting.
//!
//! Acks are `MMDSBeacon` messages from the monitor with the same `seq`; the MonClient routes
//! them straight to this helper (`Beacon::handle_mds_beacon`): the acked beacon's send time
//! becomes `last_acked_stamp`, entries up to that seq are forgotten, and a laggy daemon whose
//! round trip is below the grace stops being laggy. [`Beacon::is_laggy`] is true while no ack
//! arrived within `grace` (15 s, `mds_beacon_grace`), including before the first ack.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use denc::{EntityType, Message, RawMessage, Uuid, features};
use tokio::sync::{Notify, mpsc};
use tracing::{debug, info, warn};

use crate::mdsmap::{CompatSet, MdsState};
use crate::messages::{MMDSBeacon, MSG_MDS_BEACON, MdsHealth};
use crate::monclient::MonClient;

/// Beacon settings.
#[derive(Debug, Clone)]
pub struct BeaconConfig {
    /// Daemon id (`rust` for `mds.rust`).
    pub name: String,
    pub fsid: Uuid,
    pub compat: CompatSet,
    /// Metadata sent with `up:boot` (`collect_sys_info` + `addr`).
    pub sys_info: BTreeMap<String, String>,
    /// `mds_join_fs`.
    pub fs: String,
    /// `mds_beacon_interval`.
    pub interval: Duration,
    /// `mds_beacon_grace`.
    pub grace: Duration,
    /// `mds_features`.
    pub mds_features: u64,
}

impl BeaconConfig {
    pub fn new(name: impl Into<String>, fsid: Uuid) -> Self {
        BeaconConfig {
            name: name.into(),
            fsid,
            compat: CompatSet::rust_mds(),
            sys_info: BTreeMap::new(),
            fs: String::new(),
            interval: Duration::from_secs(4),
            grace: Duration::from_secs(15),
            mds_features: features::ALL,
        }
    }
}

#[derive(Debug)]
struct State {
    epoch: u32,
    want_state: MdsState,
    last_seq: u64,
    seq_stamp: BTreeMap<u64, Instant>,
    last_send: Option<Instant>,
    last_acked_stamp: Option<Instant>,
    last_acked_seq: u64,
    laggy: bool,
    health: MdsHealth,
    acks: u64,
}

struct Shared {
    cfg: BeaconConfig,
    monc: MonClient,
    st: Mutex<State>,
    wake: Notify,
    stopped: AtomicBool,
}

/// Handle on the beacon sender. Cheap to clone.
#[derive(Clone)]
pub struct Beacon(Arc<Shared>);

/// Snapshot for diagnostics.
#[derive(Debug, Clone)]
pub struct BeaconStats {
    pub epoch: u32,
    pub want_state: MdsState,
    pub last_seq: u64,
    pub last_acked_seq: u64,
    pub acks: u64,
    pub laggy: bool,
}

impl Beacon {
    /// Start sending with `want_state = up:boot` at epoch 0. Registers with the MonClient to
    /// receive beacon acks.
    pub fn start(monc: MonClient, cfg: BeaconConfig) -> Beacon {
        let shared = Arc::new(Shared {
            cfg,
            monc: monc.clone(),
            st: Mutex::new(State {
                epoch: 0,
                want_state: MdsState::BOOT,
                last_seq: 0,
                seq_stamp: BTreeMap::new(),
                last_send: None,
                last_acked_stamp: None,
                last_acked_seq: 0,
                laggy: false,
                health: MdsHealth::default(),
                acks: 0,
            }),
            wake: Notify::new(),
            stopped: AtomicBool::new(false),
        });
        let (tx, mut rx) = mpsc::channel::<RawMessage>(64);
        monc.route(MSG_MDS_BEACON, tx);
        let b = Beacon(shared);
        let acker = b.clone();
        tokio::spawn(async move {
            while let Some(m) = rx.recv().await {
                acker.handle_mds_beacon(m);
            }
        });
        let sender = b.clone();
        tokio::spawn(async move { sender.run().await });
        b
    }

    async fn run(self) {
        let interval = self.0.cfg.interval;
        loop {
            let wait = {
                let st = self.0.st.lock().unwrap();
                match st.last_send {
                    Some(t) => {
                        let since = t.elapsed();
                        if since >= interval.mul_f64(0.9) { Duration::ZERO } else { interval - since }
                    }
                    None => Duration::ZERO,
                }
            };
            if wait.is_zero() {
                self.send();
                let _ = tokio::time::timeout(interval, self.0.wake.notified()).await;
            } else {
                let _ = tokio::time::timeout(wait, self.0.wake.notified()).await;
            }
            if self.0.stopped.load(Ordering::Relaxed) {
                debug!("beacon stopped");
                return;
            }
        }
    }

    /// `Beacon::_send`.
    fn send(&self) {
        let global_id = self.0.monc.global_id();
        if global_id == 0 {
            debug!("not authenticated yet; beacon deferred");
            return;
        }
        let msg = {
            let mut st = self.0.st.lock().unwrap();
            st.last_seq += 1;
            let seq = st.last_seq;
            let now = Instant::now();
            st.seq_stamp.insert(seq, now);
            st.last_send = Some(now);
            assert_ne!(st.want_state, MdsState::NULL);
            let mut sys_info = self.0.cfg.sys_info.clone();
            if st.want_state == MdsState::BOOT {
                sys_info.insert("addr".into(), self.0.monc.messenger().my_addrs_string());
            }
            debug!(seq, state = %st.want_state, epoch = st.epoch, "sending beacon");
            MMDSBeacon {
                version: u64::from(st.epoch),
                fsid: self.0.cfg.fsid,
                global_id,
                state: st.want_state,
                seq,
                name: self.0.cfg.name.clone(),
                compat: self.0.cfg.compat.clone(),
                health: st.health.clone(),
                sys_info,
                mds_features: self.0.cfg.mds_features,
                fs: self.0.cfg.fs.clone(),
            }
        };
        self.0.monc.send(msg.to_raw(features::ALL));
    }

    /// `Beacon::handle_mds_beacon`: process an ack from the monitor.
    fn handle_mds_beacon(&self, m: RawMessage) {
        if m.src.is_some_and(|s| s.ty != EntityType::MON) {
            return;
        }
        let ack = match MMDSBeacon::from_raw(m) {
            Ok(a) => a,
            Err(e) => {
                warn!("bad beacon ack: {e}");
                return;
            }
        };
        let mut st = self.0.st.lock().unwrap();
        let Some(&sent) = st.seq_stamp.get(&ack.seq) else {
            debug!(seq = ack.seq, "ack for unknown beacon seq");
            return;
        };
        let rtt = sent.elapsed();
        st.last_acked_stamp = Some(sent);
        st.last_acked_seq = ack.seq;
        st.acks += 1;
        if st.laggy && rtt < self.0.cfg.grace {
            info!(?rtt, "MDS is no longer laggy");
            st.laggy = false;
        }
        st.seq_stamp.retain(|s, _| *s > ack.seq);
        debug!(seq = ack.seq, ?rtt, state = %ack.state, "beacon acked");
    }

    /// `set_want_state(mdsmap, state)`: epoch and state change together.
    pub fn set_want_state(&self, epoch: u32, state: MdsState) {
        let mut st = self.0.st.lock().unwrap();
        if st.want_state != state {
            info!(from = %st.want_state, to = %state, epoch, "want state");
        }
        st.epoch = st.epoch.max(epoch);
        st.want_state = state;
    }

    /// `notify_mdsmap`: record the epoch of an applied map.
    pub fn notify_mdsmap(&self, epoch: u32) {
        let mut st = self.0.st.lock().unwrap();
        st.epoch = st.epoch.max(epoch);
    }

    /// Send a beacon now (`MDSRank::request_state` does this after `set_want_state`).
    pub fn send_now(&self) {
        self.send();
        self.0.wake.notify_one();
    }

    /// Stop sending (the daemon is exiting).
    pub fn stop(&self) {
        self.0.stopped.store(true, Ordering::Relaxed);
        self.0.wake.notify_one();
    }

    pub fn want_state(&self) -> MdsState {
        self.0.st.lock().unwrap().want_state
    }

    pub fn set_health(&self, h: MdsHealth) {
        self.0.st.lock().unwrap().health = h;
    }

    /// `Beacon::is_laggy`: no ack for longer than the grace (or none yet).
    pub fn is_laggy(&self) -> bool {
        let mut st = self.0.st.lock().unwrap();
        let laggy = st.last_acked_stamp.is_none_or(|t| t.elapsed() > self.0.cfg.grace);
        if laggy && !st.laggy {
            warn!("MDS is laggy: no beacon ack within the grace");
            st.laggy = true;
        }
        laggy
    }

    pub fn stats(&self) -> BeaconStats {
        let st = self.0.st.lock().unwrap();
        BeaconStats {
            epoch: st.epoch,
            want_state: st.want_state,
            last_seq: st.last_seq,
            last_acked_seq: st.last_acked_seq,
            acks: st.acks,
            laggy: st.laggy,
        }
    }
}

/// `stringify(entity_addrvec_t)`: one address prints bare, several as `[a,b]`.
pub trait AddrsString {
    fn my_addrs_string(&self) -> String;
}

impl AddrsString for msgr::Messenger {
    fn my_addrs_string(&self) -> String {
        let v = self.my_addrs();
        match v.0.len() {
            0 => String::new(),
            1 => v.0[0].to_string(),
            _ => format!("[{}]", v.0.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(",")),
        }
    }
}
