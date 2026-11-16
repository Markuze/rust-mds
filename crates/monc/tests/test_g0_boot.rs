//! Golden data from the C++ MDS (Ceph 20.2.0) booting over msgr1 against the monitor
//! (`testdata/captures/mds-v1-boot.pcap`, split with `testdata/msgr1_split.py`; front payloads
//! in `tests/data/mds_v1_boot`). Expected values come from `ceph fs dump <epoch> -f json` on the
//! same cluster (ceph-dencoder 20.2.0 crashes in `MDSMap::decode` on these maps).

use bytes::Bytes;
use denc::{Message, RawMessage};
use monc::messages::*;
use monc::{MdsMap, MdsState, MonMap};

fn front(name: &str) -> Bytes {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/mds_v1_boot")
        .join(name);
    Bytes::from(std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
}

fn raw(ty: u16, version: u16, compat: u16, f: Bytes) -> RawMessage {
    RawMessage::new(ty, version, compat, f)
}

fn mdsmap(name: &str) -> (MMDSMap, MdsMap) {
    monc::decode_mdsmap(raw(CEPH_MSG_MDS_MAP, 2, 1, front(name))).unwrap()
}

#[test]
fn mdsmap_for_unknown_daemon() {
    // e22: the MDS is not in the FSMap yet: an empty map at the FSMap epoch.
    let (m, map) = mdsmap("0014-c0-s2c-MMDSMap.front.bin");
    assert_eq!(m.epoch, 22);
    assert_eq!(map.epoch, 22);
    assert!(map.mds_info.is_empty());
    assert!(map.in_.is_empty());
    assert_eq!(map.state_of(14266), MdsState::NULL);
}

#[test]
fn mdsmap_standby_view() {
    // e23: the standby's synthetic map holds only its own entry.
    let (_, map) = mdsmap("0016-c0-s2c-MMDSMap.front.bin");
    assert_eq!(map.epoch, 23);
    assert_eq!(map.mds_info.len(), 1);
    let i = map.info(14266).unwrap();
    assert_eq!(i.name, "a");
    assert_eq!(i.rank, -1);
    assert_eq!(i.state, MdsState::STANDBY);
    assert_eq!(
        i.addrs.legacy_addr().to_string(),
        "v1:127.0.0.1:7401/2549687507"
    );
}

#[test]
fn mdsmap_rank_assigned() {
    // e24: fs 'a' with rank 0 up:creating (`ceph fs dump 24`).
    let (m, map) = mdsmap("0020-c0-s2c-MMDSMap.front.bin");
    assert_eq!(m.epoch, 24);
    assert_eq!(map.epoch, 24);
    assert_eq!(map.fs_name, "a");
    assert!(map.enabled);
    assert_eq!(map.flags, 18);
    assert_eq!(map.max_mds, 1);
    assert_eq!(map.in_.iter().copied().collect::<Vec<_>>(), vec![0]);
    assert_eq!(map.up.get(&0), Some(&14266));
    assert!(map.failed.is_empty() && map.stopped.is_empty() && map.damaged.is_empty());
    assert_eq!(map.data_pools, vec![7]);
    assert_eq!(map.metadata_pool, 6);
    assert_eq!((map.session_timeout, map.session_autoclose), (60, 300));
    assert_eq!(map.max_file_size, 1 << 40);
    assert_eq!(map.max_xattr_size, 65536);
    assert_eq!(map.ev, 19, "Ceph 20.2 encodes MDSMap ev 19");
    assert_eq!(map.compat, monc::CompatSet::mds_all());
    let i = map.info(14266).unwrap();
    assert_eq!(
        (i.name.as_str(), i.rank, i.inc, i.state, i.state_seq),
        ("a", 0, 24, MdsState::CREATING, 1)
    );
    assert_eq!(
        i.addrs.legacy_addr().to_string(),
        "v1:127.0.0.1:7401/2549687507"
    );
    assert_eq!(i.join_fscid, -1);
    assert_eq!(i.mds_features, 0x3f07_ffff_fffd_ffff);
    assert_eq!(i.compat, monc::CompatSet::mds_all());
}

#[test]
fn cpp_boot_beacon_round_trips_byte_for_byte() {
    let f = front("0015-c0-c2s-MMDSBeacon.front.bin");
    let b = MMDSBeacon::from_raw(raw(MSG_MDS_BEACON, 8, 6, f.clone())).unwrap();
    assert_eq!(b.state, MdsState::BOOT);
    assert_eq!(
        (b.global_id, b.seq, b.name.as_str(), b.version),
        (14266, 1, "a", 0)
    );
    assert_eq!(b.compat, monc::CompatSet::mds_all());
    assert_eq!(b.sys_info["addr"], "v1:127.0.0.1:7401/2549687507");
    assert_eq!(b.mds_features, 0x3f07_ffff_fffd_ffff);
    // Our encoder produces the C++ bytes.
    assert_eq!(b.encode_payload(0).front.freeze(), f);
    // The monitor's ack echoes seq and state (empty compat and sys_info).
    let ack = MMDSBeacon::from_raw(raw(
        MSG_MDS_BEACON,
        8,
        6,
        front("0018-c0-s2c-MMDSBeacon.front.bin"),
    ))
    .unwrap();
    assert_eq!(
        (ack.seq, ack.state, ack.global_id),
        (1, MdsState::BOOT, 14266)
    );
    assert!(ack.sys_info.is_empty());
}

#[test]
fn cpp_auth_and_subscribe_round_trip() {
    // The first MAuth (CEPH_AUTH_UNKNOWN, methods [cephx], mds.a, global id 0).
    let f = front("0001-c0-c2s-MAuth.front.bin");
    let a = MAuth::from_raw(raw(CEPH_MSG_AUTH, 1, 1, f.clone())).unwrap();
    assert_eq!(a.protocol, auth::AUTH_UNKNOWN);
    let ours = auth::client::initial_mauth_payload(
        &auth::AuthName::new(denc::EntityType::MDS, "a"),
        0,
        &[auth::AUTH_CEPHX],
    );
    assert_eq!(a.auth_payload, ours);
    assert_eq!(a.encode_payload(0).front.freeze(), f);
    // The first MMonSubscribe {config 0+, monmap 0+} with the host name.
    let f = front("0008-c0-c2s-MMonSubscribe.front.bin");
    let s = MMonSubscribe::decode_payload(3, f.clone(), Bytes::new(), Bytes::new()).unwrap();
    assert_eq!(s.what.get("config"), Some(&(0, 0)));
    assert_eq!(s.what.get("monmap"), Some(&(0, 0)));
    assert_eq!(s.encode_payload(0).front.freeze(), f);
}

#[test]
fn monmap_and_config() {
    let m = MMonMap::from_raw(raw(
        CEPH_MSG_MON_MAP,
        1,
        1,
        front("0005-c0-s2c-MMonMap.front.bin"),
    ))
    .unwrap();
    let mm: MonMap = denc::decode_exact(m.monmap, "monmap").unwrap();
    assert_eq!(mm.epoch, 1);
    assert_eq!(mm.ranks, vec!["a"]);
    assert_eq!(
        mm.v1_addrs()
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>(),
        vec!["v1:127.0.0.1:6789/0"]
    );
    let c = MConfig::from_raw(raw(
        MSG_CONFIG,
        1,
        1,
        front("0010-c0-s2c-MConfig.front.bin"),
    ))
    .unwrap();
    assert!(c.config.is_empty());
}

/// A dev-cluster secret from the environment or the workspace's git-ignored `.env` (see
/// `.env.example`). `None`, and the caller skips its test, when neither has it.
fn dev_key(var: &str) -> Option<auth::CryptoKey> {
    let val = std::env::var(var).ok().or_else(|| {
        let env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
        std::fs::read_to_string(env).ok()?.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == var).then(|| v.trim().to_string())
        })
    });
    match val.filter(|v| !v.is_empty()) {
        Some(v) => Some(auth::CryptoKey::from_base64(&v).unwrap_or_else(|e| panic!("{var}: {e}"))),
        None => {
            eprintln!("{var} is not set (see .env.example): skipping");
            None
        }
    }
}

/// Replay the C++ MDS's cephx exchange with the real monitor through our `CephxClient`, using
/// mds.a's key in force at capture time (`RMDS_TEST_MDS_A_KEY` in `.env`; skipped without it).
#[test]
fn cpp_cephx_exchange_replays_through_our_client() {
    use auth::types::{CephXAuthenticate, CephXChallenge};
    use auth::{AuthName, CephxClient, Progress};
    use denc::Decode;

    let Some(key) = dev_key("RMDS_TEST_MDS_A_KEY") else {
        return;
    };
    let want = auth::ENTITY_MON | auth::ENTITY_OSD | auth::ENTITY_MDS | auth::ENTITY_MGR;
    let mut c = CephxClient::new(AuthName::new(denc::EntityType::MDS, "a"), key.clone(), want);
    let reply = |n: &str| MAuthReply::from_raw(raw(CEPH_MSG_AUTH_REPLY, 1, 1, front(n))).unwrap();
    let request = |n: &str| MAuth::from_raw(raw(CEPH_MSG_AUTH, 1, 1, front(n))).unwrap();

    // 1. Server challenge.
    let r1 = reply("0002-c0-s2c-MAuthReply.front.bin");
    assert_eq!((r1.protocol, r1.result), (auth::AUTH_CEPHX, 0));
    let server_challenge = CephXChallenge::decode(&mut r1.result_bl.clone())
        .unwrap()
        .server_challenge;
    c.set_global_id(r1.global_id);
    assert!(matches!(
        c.handle_response(0, r1.result_bl).unwrap(),
        Progress::Continue
    ));

    // 2. The C++ client's challenge proof equals ours for the same challenges.
    let a2 = request("0003-c0-c2s-MAuth.front.bin");
    let mut p = a2.auth_payload.clone();
    assert_eq!(
        u16::decode(&mut p).unwrap(),
        auth::client::CEPHX_GET_AUTH_SESSION_KEY
    );
    let auth_req = CephXAuthenticate::decode(&mut p).unwrap();
    assert_eq!(
        auth::client::calc_client_server_challenge(
            &key,
            server_challenge,
            auth_req.client_challenge
        )
        .unwrap(),
        auth_req.key
    );
    assert_eq!(auth_req.other_keys, want | auth::ENTITY_AUTH);
    assert!(auth_req.old_ticket.blob.is_empty());
    // Our request has the same shape.
    let ours = c.build_request().unwrap();
    assert_eq!(ours.len(), a2.auth_payload.len());

    // 3. The auth ticket decrypts with mds.a's key; service tickets are still needed.
    assert!(matches!(
        c.handle_response(0, reply("0004-c0-s2c-MAuthReply.front.bin").result_bl)
            .unwrap(),
        Progress::Continue
    ));
    assert!(c.ticket(auth::ENTITY_AUTH).is_some());
    // The C++ principal request: header 0x200, authorizer for the auth service, keys.
    let a3 = request("0006-c0-c2s-MAuth.front.bin");
    assert_eq!(
        &a3.auth_payload[..2],
        &auth::client::CEPHX_GET_PRINCIPAL_SESSION_KEY.to_le_bytes()
    );
    assert_eq!(c.build_request().unwrap().len(), a3.auth_payload.len());

    // 4. Service tickets, sealed with the auth session key.
    assert!(matches!(
        c.handle_response(0, reply("0007-c0-s2c-MAuthReply.front.bin").result_bl)
            .unwrap(),
        Progress::Done
    ));
    assert_eq!(c.have() & want, want);
    for svc in [
        auth::ENTITY_MON,
        auth::ENTITY_OSD,
        auth::ENTITY_MDS,
        auth::ENTITY_MGR,
    ] {
        assert!(
            c.build_authorizer(svc).is_some(),
            "no authorizer for service {svc:#x}"
        );
    }

    // 5. The rotating-secrets request is byte-identical; the reply holds 3 mds secrets.
    assert_eq!(
        request("0009-c0-c2s-MAuth.front.bin").auth_payload,
        c.build_rotating_request()
    );
    match c
        .handle_response(0, reply("0012-c0-s2c-MAuthReply.front.bin").result_bl)
        .unwrap()
    {
        Progress::Rotating(r) => {
            assert_eq!(r.secrets.len(), 3);
            assert!(
                r.secrets
                    .values()
                    .all(|s| s.key.key_type() == auth::crypto::CRYPTO_AES)
            );
        }
        p => panic!("unexpected {p:?}"),
    }
}
