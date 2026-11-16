//! MonClient and Beacon against a fake monitor built from the messenger and the test ticket
//! issuer of the `auth` crate: msgr1 session without authorizer, `MAuth` negotiation, the cephx
//! exchange, rotating secrets, subscriptions, an `MMDSMap`, beacon acks, and re-hunting after
//! the monitor drops the connection.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use auth::service::MonTicketIssuer;
use auth::{AuthContext, AuthName, CryptoKey};
use bytes::Bytes;
use denc::{EntityName, EntityType, Message, UTime};
use monc::messages::*;
use monc::{Beacon, BeaconConfig, MdsState, MonClient, MonClientConfig, MonEvent};
use msgr::{Event, Messenger, MsgrConfig, Policy};

const GID: u64 = 4242;

async fn fake_mon(
    issuer: Arc<Mutex<MonTicketIssuer>>,
    allow_auth: Arc<AtomicBool>,
) -> (Messenger, tokio::task::JoinHandle<()>, Arc<Mutex<Vec<u16>>>) {
    let mut cfg = MsgrConfig::for_entity(EntityType::MON);
    cfg.policies.clear();
    cfg.default_policy = Policy::stateless_server(0);
    cfg.require_authorizer = false;
    cfg.nonce = Some(0);
    let ctx = AuthContext::new(
        AuthName::new(EntityType::MON, "a"),
        CryptoKey::generate_aes(UTime::now()),
        0,
    );
    let (mon, mut ev) = Messenger::bind_with(
        cfg,
        EntityName::new(EntityType::MON, 0),
        "127.0.0.1:0".parse().unwrap(),
        ctx,
    )
    .await
    .unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen2 = seen.clone();
    let task = tokio::spawn(async move {
        while let Some(e) = ev.recv().await {
            let Event::Message { conn, msg } = e else {
                continue;
            };
            seen2.lock().unwrap().push(msg.msg_type);
            match msg.msg_type {
                CEPH_MSG_MON_GET_VERSION => {
                    let q = MMonGetVersion::from_raw(msg).unwrap();
                    match q.what.as_str() {
                        "drop" => conn.mark_down(),
                        "silent" => {}
                        _ => {
                            // Deliberately unrelated header tid: only payload handle matches.
                            let mut reply = MMonGetVersionReply {
                                handle: q.handle,
                                version: if q.what == "osdmap" { 123 } else { 456 },
                                oldest_version: 7,
                            }
                            .to_raw(0);
                            reply.tid = 999;
                            conn.send(reply);
                        }
                    }
                }
                CEPH_MSG_AUTH => {
                    while !allow_auth.load(Ordering::Relaxed) {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    let a = MAuth::from_raw(msg).unwrap();
                    let mut iss = issuer.lock().unwrap();
                    let (protocol, bl) = if a.protocol == auth::AUTH_UNKNOWN {
                        (auth::AUTH_CEPHX, iss.server_challenge())
                    } else {
                        (
                            auth::AUTH_CEPHX,
                            iss.handle_request(a.auth_payload).unwrap(),
                        )
                    };
                    let r = MAuthReply {
                        protocol,
                        result: 0,
                        global_id: GID,
                        result_bl: bl,
                        result_msg: String::new(),
                    };
                    conn.send(r.to_raw(0));
                }
                CEPH_MSG_MON_SUBSCRIBE => {
                    let s = MMonSubscribe::decode_payload(
                        msg.version,
                        msg.front,
                        Bytes::new(),
                        Bytes::new(),
                    )
                    .unwrap();
                    if s.what.contains_key("monmap") {
                        let mm = Bytes::from_static(include_bytes!("data/monmap_test3.bin"));
                        conn.send(MMonMap { monmap: mm }.to_raw(0));
                    }
                    if s.what.contains_key("config") {
                        conn.send(MConfig::default().to_raw(0));
                    }
                    if s.what.contains_key("mdsmap") {
                        let map = Bytes::from_static(include_bytes!("data/mdsmap_test1.bin"));
                        conn.send(
                            MMDSMap {
                                fsid: Default::default(),
                                epoch: 0,
                                encoded: map,
                                map_fs_name: String::new(),
                            }
                            .to_raw(0),
                        );
                    }
                }
                MSG_MDS_BEACON => {
                    let b = MMDSBeacon::from_raw(msg).unwrap();
                    // The ack echoes seq and state with empty health/compat.
                    let ack = MMDSBeacon {
                        compat: Default::default(),
                        sys_info: Default::default(),
                        ..b
                    };
                    conn.send(ack.to_raw(0));
                }
                _ => {}
            }
        }
    });
    (mon, task, seen)
}

#[tokio::test]
async fn monclient_and_beacon_against_fake_monitor() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("info")
        .try_init();
    let name = AuthName::new(EntityType::MDS, "rust");
    let key = CryptoKey::generate_aes(UTime::now());
    let issuer = Arc::new(Mutex::new(MonTicketIssuer::new(
        name.clone(),
        key.clone(),
        GID,
    )));
    let allow_auth = Arc::new(AtomicBool::new(false));
    let (mon, _task, seen) = fake_mon(issuer.clone(), allow_auth.clone()).await;

    let ctx = AuthContext::new(
        name,
        key,
        auth::ENTITY_MON | auth::ENTITY_OSD | auth::ENTITY_MDS | auth::ENTITY_MGR,
    );
    let (msgr, _mds_events) = Messenger::bind(
        EntityName::new(EntityType::MDS, -1),
        "127.0.0.1:0".parse().unwrap(),
        ctx.clone(),
    )
    .await
    .unwrap();
    let mut cfg = MonClientConfig::new(vec![mon.my_addr().addr.unwrap()]);
    cfg.hunt_interval = Duration::from_millis(500);
    let (monc, mut mev) = MonClient::start(msgr.clone(), cfg);

    // Authentication is held. Expired/cancelled calls must never reach the monitor
    // when authentication later completes.
    for _ in 0..32 {
        assert!(
            monc.get_version("expired", Duration::from_millis(20))
                .await
                .is_err()
        );
        let m = monc.clone();
        let cancelled =
            tokio::spawn(
                async move { m.get_version("cancelled", Duration::from_millis(20)).await },
            );
        tokio::task::yield_now().await;
        cancelled.abort();
    }
    allow_auth.store(true, Ordering::Relaxed);

    assert_eq!(
        monc.wait_authenticated(Duration::from_secs(10))
            .await
            .unwrap(),
        GID
    );
    monc.wait_rotating(Duration::from_secs(10)).await.unwrap();
    assert_eq!(
        ctx.rotating(),
        issuer.lock().unwrap().mds_keystore().rotating
    );
    assert_eq!(ctx.global_id(), GID);
    assert!(!seen.lock().unwrap().contains(&CEPH_MSG_MON_GET_VERSION));

    let (a, b) = tokio::join!(
        monc.get_version("osdmap", Duration::from_secs(2)),
        monc.get_version("mdsmap", Duration::from_secs(2)),
    );
    assert_eq!(a.unwrap(), (123, 7));
    assert_eq!(b.unwrap(), (456, 7));
    assert!(
        monc.get_version("silent", Duration::from_millis(20))
            .await
            .unwrap_err()
            .contains("timed out")
    );
    assert!(
        monc.get_version("drop", Duration::from_secs(2))
            .await
            .unwrap_err()
            .contains("session lost")
    );
    monc.wait_authenticated(Duration::from_secs(10))
        .await
        .unwrap();

    monc.subscribe("mdsmap", 0, 0);
    let map = loop {
        match tokio::time::timeout(Duration::from_secs(10), mev.recv())
            .await
            .unwrap()
            .unwrap()
        {
            MonEvent::Message(raw) if raw.msg_type == CEPH_MSG_MDS_MAP => {
                break monc::decode_mdsmap(raw).unwrap().1;
            }
            _ => {}
        }
    };
    assert_eq!(map.fs_name, "cephfs");

    let beacon = Beacon::start(monc.clone(), BeaconConfig::new("rust", Default::default()));
    beacon.set_want_state(0, MdsState::BOOT);
    beacon.send_now();
    tokio::time::sleep(Duration::from_millis(500)).await;
    let s = beacon.stats();
    assert!(s.acks >= 1, "{s:?}");
    assert!(!beacon.is_laggy());

    // The monitor drops the session: the client re-hunts and keeps its global id.
    let sessions_before = monc.status().sessions;
    for c in mon.connections() {
        c.mark_down();
    }
    for _ in 0..50 {
        if monc.status().sessions > sessions_before && monc.status().authenticated {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let st = monc.status();
    assert!(st.sessions > sessions_before, "{st:?}");
    assert_eq!(st.global_id, GID);
    assert!(seen.lock().unwrap().contains(&MSG_MDS_BEACON));

    let pending = {
        let monc = monc.clone();
        tokio::spawn(async move { monc.get_version("silent", Duration::from_secs(10)).await })
    };
    tokio::time::sleep(Duration::from_millis(20)).await;
    monc.shutdown();
    assert!(
        tokio::time::timeout(Duration::from_secs(1), pending)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err()
            .contains("stopped")
    );
    assert!(
        monc.get_version("osdmap", Duration::from_secs(1))
            .await
            .is_err()
    );
}
