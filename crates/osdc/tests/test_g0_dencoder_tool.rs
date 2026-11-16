//! Our `MOSDOp` (v9 and v8) and `MOSDOpReply` encodings decoded by `ceph-dencoder` (skipped
//! when the tool is not installed). ceph-dencoder reads messages in `encode_message()` layout:
//! 53-byte `ceph_msg_header`, 13-byte old footer, then u32-length-prefixed front, middle, data.

use bytes::{BufMut, Bytes, BytesMut};
use denc::{RawMessage, features};
use osdc::messages::{MOsdOp, MOsdOpReply};
use osdc::ops::{ObjectOperation, cmp_op, flags};
use osdc::types::{CEPH_NOSNAP, EVersion, HObject, OsdReqId, PgId, SpgId};

fn encode_message(m: &RawMessage) -> Bytes {
    let mut b = BytesMut::new();
    b.put_u64_le(1); // seq
    b.put_u64_le(m.tid);
    b.put_u16_le(m.msg_type);
    b.put_u16_le(m.priority);
    b.put_u16_le(m.version);
    b.put_u32_le(m.front.len() as u32);
    b.put_u32_le(m.middle.len() as u32);
    b.put_u32_le(m.data.len() as u32);
    b.put_u16_le(m.data_off);
    b.put_u8(0x02); // src: mds.0
    b.put_i64_le(0);
    b.put_u16_le(m.compat_version);
    b.put_u16_le(0);
    b.put_u32_le(0); // header crc (not checked by ceph-dencoder)
    b.put_bytes(0, 12); // footer crcs
    b.put_u8(1); // COMPLETE
    for seg in [&m.front, &m.middle, &m.data] {
        b.put_u32_le(seg.len() as u32);
        b.put_slice(seg);
    }
    b.freeze()
}

fn dencoder(ty: &str, blob: &Bytes, name: &str) -> Option<String> {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let path = dir.join(format!("osdc-{name}.bin"));
    std::fs::write(&path, blob).unwrap();
    let out = std::process::Command::new("ceph-dencoder")
        .args(["type", ty, "import"])
        .arg(&path)
        .args(["decode", "dump_json"])
        .output()
        .ok()?;
    assert!(
        out.status.success(),
        "ceph-dencoder failed on {name}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn sample_op() -> MOsdOp {
    let mut op = ObjectOperation::new();
    op.cmpxattr_u64("inline_version", cmp_op::GT, 3)
        .write_trunc(0, Bytes::from_static(b"data"), 7, 2)
        .setxattr(
            "inline_version",
            Bytes::from_static(b"18446744073709551615"),
        )
        .trimtrunc(3, 4096)
        .omap_set_header(Bytes::from_static(b"hdr"))
        .assert_version(9);
    MOsdOp {
        tid: 77,
        client_inc: 5,
        osdmap_epoch: 56,
        flags: flags::WRITE
            | flags::ONDISK
            | flags::KNOWN_REDIR
            | flags::SUPPORTSPOOLEIO
            | flags::FULL_FORCE,
        mtime: denc::UTime {
            sec: 1_700_000_000,
            nsec: 5,
        },
        retry_attempt: 0,
        hobj: HObject::new(
            &b"10000000000.00000000"[..],
            Bytes::new(),
            CEPH_NOSNAP,
            0xf0d2_ad31,
            12,
            Bytes::new(),
        ),
        pgid: SpgId::new(PgId::new(12, 1)),
        ops: op.ops,
        snap_seq: 4,
        snaps: vec![4, 2],
        features: features::ALL,
        reqid: OsdReqId {
            inc: 5,
            ..OsdReqId::default()
        },
        priority: 63,
    }
}

#[test]
fn ceph_dencoder_accepts_our_mosdop() {
    let m = sample_op();
    for (v, feat) in [
        (9u16, features::ALL),
        (8, features::ALL & !features::SERVER_SQUID),
    ] {
        let raw = m.to_raw_for(feat).unwrap();
        assert_eq!(raw.version, v);
        let Some(json) = dencoder("MOSDOp", &encode_message(&raw), &format!("mosdop-v{v}")) else {
            eprintln!("ceph-dencoder not installed; skipping");
            return;
        };
        // osd_op(mds.0.5:77 12.1 12.f0d2ad31 (undecoded) ondisk+write+known_if_redirected+full_force+supports_pool_eio e56)
        assert!(
            json.contains("osd_op(mds.0.5:77 12.1 12.f0d2ad31"),
            "v{v}: {json}"
        );
        assert!(
            json.contains("full_force") && json.contains("e56"),
            "v{v}: {json}"
        );
        // and our decoder agrees (re-encoding the decoded message gives the same bytes)
        let back = MOsdOp::decode_raw(&raw).unwrap();
        assert_eq!(
            (back.tid, back.client_inc, back.reqid, back.flags),
            (m.tid, m.client_inc, m.reqid, m.flags)
        );
        let again = back.to_raw_for(feat).unwrap();
        assert_eq!(
            (again.front, again.data),
            (raw.front.clone(), raw.data.clone()),
            "v{v} round trip"
        );
    }
}

#[test]
fn ceph_dencoder_accepts_our_mosdopreply() {
    let m = sample_op();
    let mut ops = m.ops.clone();
    for o in ops.iter_mut() {
        o.indata = Bytes::new();
    }
    ops[0].rval = -125;
    let r = MOsdOpReply {
        tid: 77,
        oid: m.hobj.oid.clone(),
        pgid: m.pgid.pgid,
        flags: i64::from(flags::ONDISK | flags::WRITE),
        result: -125,
        bad_replay_version: EVersion {
            version: 3,
            epoch: 56,
        },
        osdmap_epoch: 56,
        ops,
        retry_attempt: 0,
        replay_version: EVersion {
            version: 3,
            epoch: 56,
        },
        user_version: 3,
        redirect: None,
    };
    let raw = r.to_raw();
    let Some(json) = dencoder("MOSDOpReply", &encode_message(&raw), "mosdopreply") else {
        return;
    };
    assert!(
        json.contains("osd_op_reply(77 10000000000.00000000"),
        "{json}"
    );
    assert!(json.contains("= -125"), "{json}");
    assert_eq!(MOsdOpReply::decode_raw(&raw).unwrap(), r);
}
