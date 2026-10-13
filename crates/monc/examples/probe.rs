//! Platform probe: become a Ceph MDS daemon over msgr1 without serving metadata.
//!
//! ```text
//! cargo run -p monc --example probe -- --id rust --conf $CEPH_CONF --keyring $RMDS_KEYRING \
//!     --bind 127.0.0.1:6810 [--fs a] [--dump DIR]
//! ```
//!
//! It binds the messenger, authenticates to the monitor as `mds.<id>`, fetches the rotating
//! `mds` secrets, subscribes to the mdsmap, beacons `up:boot`, follows the monitor to
//! `up:standby` and, when it is given a rank, walks the rank to `up:active` (creating/starting
//! go straight to active; replay goes through reconnect and rejoin), printing every MDSMap
//! transition. Meanwhile it accepts client connections, verifies their cephx authorizers and
//! prints the first message each client sends (a kernel mount shows `MClientSession`
//! `REQUEST_OPEN`, type 22), then holds the connection. Ctrl-C sends a `down:dne` beacon and
//! exits. `--dump DIR` writes each MDSMap and each first client message to `DIR`;
//! `--no-beacon` only authenticates and subscribes (the daemon does not join the FSMap).

use std::collections::{BTreeMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use auth::{AuthContext, AuthName, Keyring};
use denc::{EntityName, EntityType, RawMessage, Uuid};
use monc::{
    AddrsString, Beacon, BeaconConfig, CephConf, MdsMap, MdsState, MonClient, MonClientConfig,
    MonEvent,
};
use msgr::{Event, Messenger};

struct Args {
    id: String,
    conf: PathBuf,
    keyring: Option<PathBuf>,
    bind: SocketAddr,
    fs: Option<String>,
    dump: Option<PathBuf>,
    no_beacon: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut id = None;
    let mut conf = None;
    let mut keyring = None;
    let mut bind = None;
    let mut fs = None;
    let mut dump = None;
    let mut no_beacon = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--id" | "-i" => id = Some(val()?),
            "--conf" | "-c" => conf = Some(PathBuf::from(val()?)),
            "--keyring" | "-k" => keyring = Some(PathBuf::from(val()?)),
            "--bind" => {
                bind = Some(
                    val()?
                        .parse::<SocketAddr>()
                        .map_err(|e| format!("--bind: {e}"))?,
                )
            }
            "--fs" => fs = Some(val()?),
            "--dump" => dump = Some(PathBuf::from(val()?)),
            "--no-beacon" => no_beacon = true,
            "-h" | "--help" => {
                return Err("usage: probe --id ID --conf CEPH_CONF [--keyring K] --bind IP:PORT [--fs NAME] [--dump DIR] [--no-beacon]".into());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Args {
        id: id.ok_or("--id is required")?,
        conf: conf
            .or_else(|| std::env::var_os("CEPH_CONF").map(PathBuf::from))
            .ok_or("--conf is required")?,
        keyring,
        bind: bind.ok_or("--bind is required")?,
        fs,
        dump,
        no_beacon,
    })
}

fn sys_info(hostname: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let read = |p: &str| {
        std::fs::read_to_string(p)
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    m.insert("hostname".into(), hostname.to_string());
    m.insert("ceph_version".into(), "ceph version 20.2.0 (69f84cc2651aa259a15bc192ddaabd3baba07489) tentacle (stable - RelWithDebInfo)".into());
    m.insert("ceph_version_short".into(), "20.2.0".into());
    m.insert("ceph_release".into(), "tentacle".into());
    m.insert("os".into(), "Linux".into());
    m.insert("arch".into(), std::env::consts::ARCH.into());
    m.insert("kernel_version".into(), read("/proc/sys/kernel/osrelease"));
    m.insert(
        "kernel_description".into(),
        read("/proc/sys/kernel/version"),
    );
    m.insert("mds_impl".into(), "rust-mds probe".into());
    m
}

/// `uuid_d` in its canonical 8-4-4-4-12 form.
fn fmt_uuid(u: &Uuid) -> String {
    let h: String = u.0.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

fn session_op_name(op: u32) -> &'static str {
    match op {
        0 => "REQUEST_OPEN",
        1 => "OPEN",
        2 => "REQUEST_CLOSE",
        3 => "CLOSE",
        4 => "REQUEST_RENEWCAPS",
        5 => "RENEWCAPS",
        6 => "STALE",
        7 => "RECALL_STATE",
        8 => "FLUSHMSG",
        9 => "FLUSHMSG_ACK",
        10 => "FORCE_RO",
        11 => "REJECT",
        12 => "REQUEST_FLUSH_MDLOG",
        _ => "?",
    }
}

fn describe_first_message(m: &RawMessage) -> String {
    let mut s = format!(
        "type {} v{} compat {} prio {} tid {} front {} middle {} data {} from {}",
        m.msg_type,
        m.version,
        m.compat_version,
        m.priority,
        m.tid,
        m.front.len(),
        m.middle.len(),
        m.data.len(),
        m.src.map(|n| n.to_string()).unwrap_or_default()
    );
    if m.msg_type == 22 && m.front.len() >= 28 {
        // ceph_mds_session_head: le32 op, le64 seq, ceph_timespec stamp, le32 max_caps,
        // le32 max_leases.
        let op = u32::from_le_bytes(m.front[0..4].try_into().unwrap());
        let seq = u64::from_le_bytes(m.front[4..12].try_into().unwrap());
        s.push_str(&format!(
            " => MClientSession op {} ({}) seq {}",
            op,
            session_op_name(op),
            seq
        ));
    }
    s
}

/// What the probe does with the rank it is given.
fn next_request(state: MdsState) -> Option<MdsState> {
    match state {
        MdsState::CREATING | MdsState::STARTING => Some(MdsState::ACTIVE),
        MdsState::REPLAY => Some(MdsState::RECONNECT),
        MdsState::RESOLVE => Some(MdsState::RECONNECT),
        MdsState::RECONNECT => Some(MdsState::REJOIN),
        MdsState::REJOIN => Some(MdsState::ACTIVE),
        MdsState::CLIENTREPLAY => Some(MdsState::ACTIVE),
        MdsState::STOPPING => Some(MdsState::STOPPED),
        _ => None,
    }
}

fn summarize(m: &MdsMap) -> String {
    let infos: Vec<String> = m
        .mds_info
        .values()
        .map(|i| {
            format!(
                "{}:{}(gid {}, rank {}, {}{})",
                i.name,
                i.state,
                i.global_id,
                i.rank,
                i.addrs.legacy_addr(),
                if i.is_laggy() { ", laggy" } else { "" }
            )
        })
        .collect();
    format!(
        "fs '{}' max_mds {} in {:?} up {:?} failed {:?} stopped {:?} damaged {:?} metadata_pool {} data_pools {:?} [{}]",
        m.fs_name,
        m.max_mds,
        m.in_,
        m.up,
        m.failed,
        m.stopped,
        m.damaged,
        m.metadata_pool,
        m.data_pools,
        infos.join(", ")
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = run(args).await {
        eprintln!("probe: {e}");
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<(), String> {
    let conf = CephConf::load(&args.conf)
        .map_err(|e| format!("cannot read {}: {e}", args.conf.display()))?;
    let mon_host = conf
        .get("mds", &args.id, "mon_host")
        .ok_or("mon_host not set in ceph.conf")?;
    let mons = monc::parse_mon_host_v1(&mon_host);
    if mons.is_empty() {
        return Err(format!("no v1 monitor address in mon_host = {mon_host}"));
    }
    let keyring_path = args
        .keyring
        .clone()
        .or_else(|| conf.get("mds", &args.id, "keyring").map(PathBuf::from))
        .ok_or("no keyring")?;
    let keyring = Keyring::load(&keyring_path).map_err(|e| e.to_string())?;
    let name = AuthName::new(EntityType::MDS, args.id.clone());
    let key = keyring
        .key(&name)
        .ok_or_else(|| format!("no key for {name} in {}", keyring_path.display()))?
        .clone();
    let join_fs = args
        .fs
        .clone()
        .or_else(|| conf.get("mds", &args.id, "mds_join_fs"))
        .unwrap_or_default();
    let conf_fsid = conf
        .get("mds", &args.id, "fsid")
        .and_then(|s| Uuid::parse(&s));

    let auth = AuthContext::new(
        name.clone(),
        key,
        auth::ENTITY_MON | auth::ENTITY_OSD | auth::ENTITY_MDS | auth::ENTITY_MGR,
    );
    let (msgr, mut msgr_events) = Messenger::bind(
        EntityName::new(EntityType::MDS, -1),
        args.bind,
        auth.clone(),
    )
    .await
    .map_err(|e| format!("bind {}: {e}", args.bind))?;
    println!("probe: {name} bound at {}", msgr.my_addrs_string());
    // ms_handle_fast_authentication: a client needs a decodable mds cap (MDSAuthCaps parsing
    // is the engine's job; an empty cap blob is refused as in C++).
    msgr.set_caps_validator(|v| v.caps.allow_all || v.caps.cap_string().is_some());

    let (monc, mut mon_events) = MonClient::start(msgr.clone(), MonClientConfig::new(mons.clone()));
    let gid = monc
        .wait_authenticated(Duration::from_secs(60))
        .await
        .map_err(|e| format!("authentication: {e}"))?;
    println!("probe: authenticated to the monitor as {name}, global_id {gid}");
    monc.wait_rotating(Duration::from_secs(30))
        .await
        .map_err(|e| format!("rotating secrets: {e}"))?;
    let rot = auth.rotating();
    println!(
        "probe: got {} rotating mds secrets (ids {:?})",
        rot.secrets.len(),
        rot.secrets.keys().collect::<Vec<_>>()
    );
    monc.subscribe("mdsmap", 0, 0);

    let fsid = match monc.fsid().or(conf_fsid) {
        Some(f) => f,
        None => {
            tokio::time::sleep(Duration::from_secs(2)).await;
            monc.fsid().ok_or("fsid unknown")?
        }
    };
    let mut bcfg = BeaconConfig::new(args.id.clone(), fsid);
    bcfg.sys_info = sys_info(&monc::monclient::short_hostname());
    bcfg.fs = join_fs;
    if args.no_beacon {
        // Authentication and subscription only: do not join the FSMap.
        println!("probe: --no-beacon: not joining the FSMap; waiting 15 s for maps");
        let deadline = tokio::time::sleep(Duration::from_secs(15));
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                ev = mon_events.recv() => match ev {
                    Some(MonEvent::Message(raw)) if raw.msg_type == monc::messages::CEPH_MSG_MDS_MAP => {
                        match monc::decode_mdsmap(raw) {
                            Ok((_, map)) => println!("probe: mdsmap e{}: {}", map.epoch, summarize(&map)),
                            Err(e) => println!("probe: undecodable MDSMap: {e}"),
                        }
                    }
                    Some(e) => println!("probe: {e:?}"),
                    None => break,
                },
                _ = &mut deadline => break,
            }
        }
        let st = monc.status();
        println!(
            "probe: status: authenticated {} global_id {} rotating secrets {} fsid {}",
            st.authenticated,
            st.global_id,
            st.have_rotating,
            st.fsid.as_ref().map(fmt_uuid).unwrap_or_default()
        );
        monc.shutdown();
        msgr.shutdown();
        return Ok(());
    }
    let beacon = Beacon::start(monc.clone(), bcfg);
    println!(
        "probe: beaconing {} (compat incompat mask {:#x})",
        MdsState::BOOT,
        monc::CompatSet::rust_mds().incompat.mask()
    );

    let mut last_epoch: u32 = 0;
    let mut my_state = MdsState::NULL;
    let mut seen_in_map = false;
    let mut first_msg_seen: HashSet<u64> = HashSet::new();
    let mut nmsg = 0u64;
    if let Some(d) = &args.dump {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let mut status_tick = tokio::time::interval(Duration::from_secs(30));
    status_tick.tick().await;

    loop {
        tokio::select! {
            ev = mon_events.recv() => {
                let Some(ev) = ev else { return Err("monclient stopped".into()) };
                match ev {
                    MonEvent::Message(raw) if raw.msg_type == monc::messages::CEPH_MSG_MDS_MAP => {
                        let raw_front = raw.front.clone();
                        let (mm, map) = match monc::decode_mdsmap(raw) {
                            Ok(x) => x,
                            Err(e) => { println!("probe: undecodable MDSMap: {e}"); continue; }
                        };
                        if mm.fsid != fsid { println!("probe: MDSMap for another fsid {}", fmt_uuid(&mm.fsid)); continue; }
                        if map.epoch <= last_epoch && map.epoch != MdsMap::NULL_EPOCH { continue; }
                        if let Some(d) = &args.dump {
                            let _ = std::fs::write(d.join(format!("mdsmap_e{}.bin", map.epoch)), &mm.encoded);
                            let _ = std::fs::write(d.join(format!("mmdsmap_front_e{}.bin", map.epoch)), &raw_front);
                        }
                        let gid = monc.global_id();
                        let new_state = map.state_of(gid);
                        if map.epoch == MdsMap::NULL_EPOCH || (seen_in_map && new_state == MdsState::NULL) {
                            println!("probe: the monitor removed this daemon from the map (epoch {}); a real MDS would respawn now", map.epoch);
                            beacon.stop();
                            return Err("removed from the MDSMap".into());
                        }
                        last_epoch = map.epoch;
                        monc.sub_got("mdsmap", u64::from(map.epoch));
                        if !monc::CompatSet::rust_mds().writeable(&map.compat) {
                            println!("probe: WARNING: map compat {:#x} not writeable by our compat", map.compat.incompat.mask());
                        }
                        println!("probe: mdsmap e{}: {}", map.epoch, summarize(&map));
                        let info = map.info(gid).cloned();
                        if new_state != my_state {
                            println!(
                                "probe: e{}: my state {} -> {}{}",
                                map.epoch,
                                my_state,
                                new_state,
                                info.as_ref().map(|i| if i.rank >= 0 { format!(" (rank {}, inc {})", i.rank, i.inc) } else { String::new() }).unwrap_or_default()
                            );
                        }
                        if new_state != MdsState::NULL {
                            seen_in_map = true;
                        }
                        let prev = my_state;
                        my_state = new_state;
                        match new_state {
                            MdsState::NULL => beacon.notify_mdsmap(map.epoch),
                            MdsState::STANDBY => beacon.set_want_state(map.epoch, MdsState::STANDBY),
                            s => {
                                if let Some(i) = &info && i.rank >= 0 {
                                    msgr.set_my_name(EntityName::new(EntityType::MDS, i64::from(i.rank)));
                                }
                                // The C++ rank first records the state the monitor assigned,
                                // then requests the next one when the phase is done.
                                beacon.set_want_state(map.epoch, s);
                                if prev != s
                                    && let Some(next) = next_request(s)
                                {
                                    println!("probe: e{}: requesting {next}", map.epoch);
                                    beacon.set_want_state(map.epoch, next);
                                    beacon.send_now();
                                }
                                if s == MdsState::ACTIVE && prev != s {
                                    println!("probe: ACTIVE as rank {} of fs '{}' at {}", info.as_ref().map(|i| i.rank).unwrap_or(-1), map.fs_name, msgr.my_addrs_string());
                                }
                            }
                        }
                    }
                    MonEvent::Message(raw) => {
                        println!("probe: monitor message type {} ({} bytes) ignored", raw.msg_type, raw.front.len());
                    }
                    MonEvent::SessionEstablished { global_id, mon } => println!("probe: monitor session with {mon} (global_id {global_id})"),
                    MonEvent::SessionLost => println!("probe: monitor session lost; hunting"),
                    MonEvent::MonMap(m) => println!("probe: monmap e{} fsid {} mons {:?}", m.epoch, fmt_uuid(&m.fsid), m.mons.keys().collect::<Vec<_>>()),
                    MonEvent::Config(c) => println!("probe: config from monitor: {} keys (ignored)", c.len()),
                }
            }
            ev = msgr_events.recv() => {
                let Some(ev) = ev else { return Err("messenger stopped".into()) };
                match ev {
                    Event::Accepted { conn } => println!(
                        "probe: accepted {} connection {} from {} as {} (global_id {}, caps {:?}, features {:#x})",
                        conn.peer_type().name(),
                        conn.id(),
                        conn.peer_addr(),
                        conn.peer_auth_name().map(|n| n.to_string()).unwrap_or_default(),
                        conn.peer_global_id(),
                        conn.peer_caps().and_then(|c| c.cap_string()),
                        conn.features()
                    ),
                    Event::Message { conn, msg } => {
                        nmsg += 1;
                        if first_msg_seen.insert(conn.id()) {
                            println!("probe: first message on connection {} ({}): {}", conn.id(), conn.peer_auth_name().map(|n| n.to_string()).unwrap_or_default(), describe_first_message(&msg));
                            if let Some(d) = &args.dump {
                                let _ = std::fs::write(d.join(format!("client{}_first_type{}_front.bin", conn.id(), msg.msg_type)), &msg.front);
                            }
                        } else {
                            println!("probe: connection {}: message type {} (holding; not answered)", conn.id(), msg.msg_type);
                        }
                    }
                    Event::Reset { conn } => println!("probe: connection {} reset", conn.id()),
                    Event::Closed { conn } => println!("probe: connection {} closed", conn.id()),
                    Event::Connected { conn } => println!("probe: connection {} connected", conn.id()),
                }
            }
            _ = status_tick.tick() => {
                let s = beacon.stats();
                println!(
                    "probe: status: state {} epoch {} beacons sent {} acked {} (last acked seq {}) laggy {} client messages {}",
                    my_state, s.epoch, s.last_seq, s.acks, s.last_acked_seq, beacon.is_laggy(), nmsg
                );
            }
            _ = tokio::signal::ctrl_c() => {
                println!("probe: interrupted; sending down:dne beacon and exiting");
                beacon.set_want_state(last_epoch, MdsState::DNE);
                beacon.send_now();
                tokio::time::sleep(Duration::from_millis(1000)).await;
                beacon.stop();
                msgr.shutdown();
                return Ok(());
            }
        }
    }
}
