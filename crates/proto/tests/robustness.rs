//! No client input may crash the daemon: every decoder must return an error, never panic, on
//! malformed input. Mutates every message of the golden corpus (bit flips, byte changes,
//! truncation, insertion, other header versions) and feeds the results to all decoders.

mod common;

use bytes::{Bytes, BytesMut};
use denc::{Message, RawMessage};
use proto::*;

/// xorshift64*: deterministic, no dependencies.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
}

fn mutate(rng: &mut Rng, b: &Bytes) -> Bytes {
    let mut v = b.to_vec();
    match rng.below(6) {
        0 if !v.is_empty() => {
            let i = rng.below(v.len());
            v[i] ^= 1 << rng.below(8);
        }
        1 if !v.is_empty() => {
            let i = rng.below(v.len());
            v[i] = [0x00, 0xff, 0x7f, 0x80, 0x01][rng.below(5)];
        }
        2 => v.truncate(rng.below(v.len() + 1)),
        3 => {
            let i = rng.below(v.len() + 1);
            let n = 1 + rng.below(8);
            for _ in 0..n {
                v.insert(i, rng.next() as u8);
            }
        }
        4 if v.len() >= 4 => {
            // Turn a 32-bit field into a huge length or count.
            let i = rng.below(v.len() - 3);
            v[i..i + 4].copy_from_slice(&[0xff, 0xff, 0xff, 0x7f]);
        }
        _ => {
            let n = rng.below(4);
            for _ in 0..n {
                if !v.is_empty() {
                    let i = rng.below(v.len());
                    v[i] = rng.next() as u8;
                }
            }
        }
    }
    Bytes::from(v)
}

/// Run every decoder that could see this message type (both directions).
fn decode_all(raw: RawMessage) {
    let _ = ClientMessage::from_raw(raw.clone());
    match raw.msg_type {
        msg_type::CLIENT_REPLY => {
            if let Ok(r) = MClientReply::from_raw(raw) {
                let _ = r.check_invariants();
                let _ = r.to_raw(0);
                if let ReplyExtra::Readdir(rd) = &r.extra {
                    let _ = rd.entries.len();
                }
                let _ = SnapTrace::decode(r.snapbl.clone(), SnapFormat::Old);
                let _ = SnapTrace::decode(r.snapbl, SnapFormat::New);
            }
        }
        msg_type::CLIENT_SESSION => {
            if let Ok(s) = MClientSession::from_raw(raw) {
                let _ = s.to_raw(0);
            }
        }
        msg_type::CLIENT_CAPS => {
            if let Ok(c) = MClientCaps::from_raw(raw) {
                let _ = c.to_raw(0);
                let _ = decode_xattr_blob(c.xattrbl);
                let _ = SnapTrace::decode(c.snapbl, SnapFormat::Old);
            }
        }
        msg_type::CLIENT_REQUEST => {
            if let Ok(r) = MClientRequest::from_raw(raw) {
                let _ = r.initial_xattrs();
                let _ = r.may_write();
                let a = r.head.args;
                let _ = (a.setattr(), a.readdir(), a.open(), a.filelock(), a.lookupino(), a.setlayout());
                let _ = r.path.components();
                let _ = r.to_raw(0);
            }
        }
        msg_type::CLIENT_RECONNECT => {
            if let Ok(r) = MClientReconnect::from_raw(raw) {
                for c in r.caps.values() {
                    let _ = c.locks();
                }
            }
        }
        msg_type::CLIENT_SNAP => {
            let _ = MClientSnap::from_raw(raw);
        }
        msg_type::CLIENT_QUOTA => {
            let _ = MClientQuota::from_raw(raw);
        }
        msg_type::CLIENT_REQUEST_FORWARD => {
            let _ = MClientRequestForward::from_raw(raw);
        }
        msg_type::CLIENT_RECLAIM_REPLY => {
            let _ = MClientReclaimReply::from_raw(raw);
        }
        _ => {}
    }
}

fn corpus_messages() -> Vec<RawMessage> {
    let mut out = Vec::new();
    for name in ["kernel-v1", "fuse-v1", "fuse-v2"] {
        let p = common::corpus_path(name);
        if !p.exists() {
            continue;
        }
        let conv = common::read_corpus(&p);
        out.extend(conv.from_client.into_iter().chain(conv.from_server).map(|m| m.raw));
    }
    // Messages the captures lack.
    let mut rec = MClientReconnect::default();
    rec.caps.insert(
        0x100_0000_0001,
        CapReconnect {
            path: Bytes::from_static(b"f"),
            cap_id: 1,
            flockbl: FlockState {
                fcntl: vec![FileLock { start: 0, length: 1, client: 0, owner: 1, pid: 2, lock_type: 2 }],
                flock: vec![],
            }
            .encode(),
            ..Default::default()
        },
    );
    rec.realms.push(SnaprealmReconnect { ino: 1, seq: 1, parent: 0 });
    out.push(rec.to_raw(0));
    rec.version = 3;
    out.push(rec.to_raw(0));
    out.push(MClientLease::revoke(1, 2, Bytes::from_static(b"x"), 3).to_raw(0));
    out.push(
        MClientSnap::split(
            1,
            vec![2],
            vec![3],
            SnapTrace::new(vec![SnapRealmInfo::new(1, 0, 1)]).encode(SnapFormat::Old),
        )
        .to_raw(0),
    );
    out.push(MClientQuota::default().to_raw(0));
    out.push(MClientReclaim { uuid: "u".into(), flags: 1 }.to_raw(0));
    out.push(MClientRequestForward::new(1, 0, 1).to_raw(0));
    out
}

#[test]
fn mutated_corpus_never_panics() {
    let msgs = corpus_messages();
    assert!(msgs.len() > 20, "corpus missing");
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let rounds: usize = std::env::var("PROTO_FUZZ_ROUNDS").ok().and_then(|v| v.parse().ok()).unwrap_or(1000);
    for raw in &msgs {
        for _ in 0..rounds {
            let mut m = raw.clone();
            match rng.below(4) {
                0 => m.front = mutate(&mut rng, &m.front),
                1 => m.middle = mutate(&mut rng, &m.middle),
                2 => m.data = mutate(&mut rng, &m.data),
                _ => {
                    m.version = rng.below(20) as u16;
                    m.front = mutate(&mut rng, &m.front);
                }
            }
            decode_all(m);
        }
    }
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = Rng(42);
    let types = [
        msg_type::CLIENT_SESSION,
        msg_type::CLIENT_RECONNECT,
        msg_type::CLIENT_REQUEST,
        msg_type::CLIENT_REPLY,
        msg_type::CLIENT_METRICS,
        msg_type::CLIENT_CAPS,
        msg_type::CLIENT_LEASE,
        msg_type::CLIENT_SNAP,
        msg_type::CLIENT_CAPRELEASE,
        msg_type::CLIENT_QUOTA,
        msg_type::CLIENT_RECLAIM,
        msg_type::CLIENT_RECLAIM_REPLY,
        msg_type::CLIENT_REQUEST_FORWARD,
    ];
    for _ in 0..20_000 {
        let t = types[rng.below(types.len())];
        let mut seg = || {
            let n = rng.below(300);
            let mut b = BytesMut::with_capacity(n);
            for _ in 0..n {
                b.extend_from_slice(&[rng.next() as u8]);
            }
            b.freeze()
        };
        let (front, middle, data) = (seg(), seg(), seg());
        let mut raw = RawMessage::new(t, rng.below(16) as u16, 0, front);
        raw.middle = middle;
        raw.data = data;
        decode_all(raw);
    }
}
