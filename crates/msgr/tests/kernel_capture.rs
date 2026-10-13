//! Golden test with real traffic between the Linux 7.0 kernel client and the Ceph 20.2 C++
//! MDS over msgr1 (see `tests/data/kernel_v1/README.txt`): our cephx service side decrypts the
//! kernel's authorizer, the C++ challenge and reply have the layout we produce, the kernel's
//! challenge answer verifies, and every captured frame passes our crc and signature checks in
//! both directions. Needs the MDS's rotating secret at capture time from `.env`
//! (`RMDS_TEST_MDS_ROTATING_SECRET_2`, see `.env.example`); skipped without it.

use auth::service::{ServiceKeys, VerifyOutcome, verify_authorizer};
use auth::types::{CephXAuthorizeReply, CephXChallenge, ExpiringCryptoKey, RotatingSecrets};
use auth::{CryptoKey, SessionSigner, usage};
use bytes::Bytes;
use denc::{EntityName, EntityType, UTime};

const FEATURES: u64 = 0x2f01_8fb8_7aa4_aafe & 0x3f07_ffff_fffd_ffff;

fn data(name: &str) -> Vec<u8> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/kernel_v1")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// A dev-cluster secret from the environment or the workspace's git-ignored `.env` (see
/// `.env.example`). `None`, and the caller skips its test, when neither has it.
fn dev_key(var: &str) -> Option<CryptoKey> {
    let val = std::env::var(var).ok().or_else(|| {
        let env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.env");
        std::fs::read_to_string(env).ok()?.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == var).then(|| v.trim().to_string())
        })
    });
    match val.filter(|v| !v.is_empty()) {
        Some(v) => Some(CryptoKey::from_base64(&v).unwrap_or_else(|e| panic!("{var}: {e}"))),
        None => {
            eprintln!("{var} is not set (see .env.example): skipping");
            None
        }
    }
}

/// The MDS key store at capture time: the rotating "mds" secret with id 2.
fn keystore() -> Option<ServiceKeys> {
    let key = dev_key("RMDS_TEST_MDS_ROTATING_SECRET_2")?;
    let mut rotating = RotatingSecrets::default();
    rotating.secrets.insert(
        2,
        ExpiringCryptoKey {
            key,
            expiration: UTime::default(),
        },
    );
    rotating.max_ver = 2;
    Some(ServiceKeys {
        service_id: auth::ENTITY_MDS,
        rotating,
    })
}

#[test]
fn kernel_authorizer_and_cpp_challenge() {
    let Some(ks) = keystore() else { return };
    // Without a challenge round, the kernel's first authorizer yields its identity and key.
    let a0 = data("kernel_authorizer0.bin");
    let VerifyOutcome::Accepted(v0) =
        verify_authorizer(&ks, auth::ENTITY_MDS, &a0, None, 0).unwrap()
    else {
        panic!()
    };
    assert_eq!(v0.name.to_string(), "client.admin");
    assert_eq!(v0.global_id, 14202);
    assert_eq!(v0.caps.cap_string().as_deref(), Some("allow *"));
    // With a challenge round, our challenge has the C++ layout: 32 bytes of raw ciphertext.
    let mut ours = None;
    let VerifyOutcome::Challenge { reply } =
        verify_authorizer(&ks, auth::ENTITY_MDS, &a0, Some(&mut ours), 0).unwrap()
    else {
        panic!()
    };
    let cpp_challenge = data("cpp_challenge.bin");
    assert_eq!(reply.len(), cpp_challenge.len());
    // The C++ challenge decrypts with the session key, and the kernel's answer verifies.
    let ch: CephXChallenge = auth::crypto::decrypt_enc_bl_value(
        &v0.session_key,
        usage::AUTHORIZE_CHALLENGE,
        &cpp_challenge,
    )
    .unwrap();
    let a1 = data("kernel_authorizer1.bin");
    let mut st = Some(ch.server_challenge);
    let VerifyOutcome::Accepted(v1) =
        verify_authorizer(&ks, auth::ENTITY_MDS, &a1, Some(&mut st), 0).unwrap()
    else {
        panic!("kernel challenge answer rejected")
    };
    // A wrong challenge is refused.
    let mut wrong = Some(ch.server_challenge ^ 1);
    assert!(verify_authorizer(&ks, auth::ENTITY_MDS, &a1, Some(&mut wrong), 0).is_err());
    // The C++ reply and ours have the same layout and both carry the kernel's nonce + 1.
    let cpp_reply = data("cpp_authorizer_reply.bin");
    assert_eq!(v1.reply.len(), cpp_reply.len());
    let mut b = Bytes::from(cpp_reply);
    let theirs: CephXAuthorizeReply =
        auth::crypto::decode_decrypt_value(&mut b, &v1.session_key, usage::AUTHORIZE_REPLY)
            .unwrap();
    let mut b = v1.reply.clone();
    let mine: CephXAuthorizeReply =
        auth::crypto::decode_decrypt_value(&mut b, &v1.session_key, usage::AUTHORIZE_REPLY)
            .unwrap();
    assert_eq!(theirs, mine);
}

#[test]
fn kernel_and_cpp_signatures_verify() {
    let Some(ks) = keystore() else { return };
    let a1 = data("kernel_authorizer1.bin");
    let VerifyOutcome::Accepted(v) =
        verify_authorizer(&ks, auth::ENTITY_MDS, &a1, None, 0).unwrap()
    else {
        panic!()
    };
    let signer = SessionSigner::new(v.session_key.clone(), FEATURES).unwrap();
    let frames = [
        (
            "0013-c1-c2s-MClientSession.frame.bin",
            1,
            22,
            EntityType::CLIENT,
        ),
        (
            "0014-c1-s2c-MClientSession.frame.bin",
            1,
            22,
            EntityType::MDS,
        ),
        (
            "0015-c1-c2s-MClientRequest.frame.bin",
            2,
            24,
            EntityType::CLIENT,
        ),
        ("0016-c1-s2c-MClientReply.frame.bin", 2, 26, EntityType::MDS),
        (
            "0017-c1-c2s-MClientRequest.frame.bin",
            3,
            24,
            EntityType::CLIENT,
        ),
        ("0018-c1-s2c-MClientReply.frame.bin", 3, 26, EntityType::MDS),
    ];
    for (name, seq, ty, src) in frames {
        let f = data(name);
        let (s, m) = msgr::v1::decode_msg_frame(&f, true, Some(&signer))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((s, m.msg_type, m.src.unwrap().ty), (seq, ty, src), "{name}");
        // Re-encoding with our writer reproduces the frame byte for byte (header crc, footer
        // crcs and the signature), which checks our signing against both peers. The kernel
        // sends compat_version 0 on some messages, which our writer (like C++
        // Message::encode) replaces with the version, so those are skipped.
        if m.compat_version != 0 {
            let mut out = Vec::new();
            msgr::v1::encode_msg_frame(&mut out, s, &m, m.src.unwrap(), true, Some(&signer))
                .unwrap();
            assert_eq!(out, f, "{name}: re-encoding differs");
        }
        // A flipped payload byte fails the crc; a flipped signature fails the check.
        let mut bad = f.clone();
        let n = bad.len();
        bad[n - 2] ^= 1;
        assert!(msgr::v1::decode_msg_frame(&bad, true, Some(&signer)).is_err());
    }
    // The kernel's MClientSession is REQUEST_OPEN (op 0).
    let (_, m) = msgr::v1::decode_msg_frame(&data(frames[0].0), true, Some(&signer)).unwrap();
    assert_eq!(u32::from_le_bytes(m.front[0..4].try_into().unwrap()), 0);
    assert_eq!(m.src, Some(EntityName::new(EntityType::CLIENT, 14202)));
}
