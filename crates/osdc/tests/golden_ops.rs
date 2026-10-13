//! MOSDOp/MOSDOpReply golden tests: messages of the C++ Objecter (librados, driven by
//! `testdata/ops_driver.c`) and of the OSDs, captured on msgr1 by `testdata/gen_ops.sh`.
//! Every captured message is decoded, checked, and re-encoded byte for byte.

mod common;

use std::collections::BTreeMap;

use bytes::Bytes;
use common::*;
use denc::RawMessage;
use osdc::hash::{ceph_stable_mod, ceph_str_hash_rjenkins};
use osdc::messages::{MOsdOp, MOsdOpReply};
use osdc::ops::{self, flags, op};

/// Features both sides negotiated in the capture (`connections.tsv`).
const FEATURES: u64 = 0x3f07fffffffdffff;

struct Captured {
    n: u32,
    tid: u64,
    raw: RawMessage,
}

fn load() -> (Vec<Captured>, Vec<Captured>, i64) {
    let dir = testdata().join("ops");
    let pool: i64 = std::fs::read_to_string(dir.join("pool_id"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let index = std::fs::read_to_string(dir.join("index.tsv")).unwrap();
    let (mut reqs, mut replies) = (Vec::new(), Vec::new());
    for line in index.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        let n: u32 = f[0].parse().unwrap();
        let tid: u64 = f[8].parse().unwrap();
        let ty: u16 = f[9].parse().unwrap();
        let version: u16 = f[11].parse().unwrap();
        let compat: u16 = f[12].parse().unwrap();
        let file = f[17];
        let front = read(&dir.join(format!("{file}.front.bin")));
        let data = std::fs::read(dir.join(format!("{file}.data.bin")))
            .map(Bytes::from)
            .unwrap_or_default();
        let mut raw = RawMessage::new(ty, version, compat, front);
        raw.tid = tid;
        raw.data = data;
        let c = Captured { n, tid, raw };
        if ty == 42 {
            reqs.push(c)
        } else {
            replies.push(c)
        }
    }
    (reqs, replies, pool)
}

#[test]
fn mosdop_decodes_and_reencodes_identically() {
    let (reqs, _, pool) = load();
    assert!(reqs.len() >= 20);
    for c in &reqs {
        let m = MOsdOp::decode_raw(&c.raw).unwrap_or_else(|e| panic!("msg {}: {e}", c.n));
        assert_eq!(m.tid, c.tid);
        // the C++ Objecter's flags on every op
        for f in [flags::ONDISK, flags::KNOWN_REDIR, flags::SUPPORTSPOOLEIO] {
            assert!(m.flags & f != 0, "msg {} flags {:#x}", c.n, m.flags);
        }
        assert!(m.flags & (flags::READ | flags::WRITE) != 0);
        // placement: hash of (key or name) with the namespace, folded by pg_num 8
        assert_eq!(m.hobj.pool, pool);
        let key = if m.hobj.key.is_empty() {
            &m.hobj.oid
        } else {
            &m.hobj.key
        };
        let mut input = Vec::new();
        if !m.hobj.nspace.is_empty() {
            input.extend_from_slice(&m.hobj.nspace);
            input.push(0x1f);
        }
        input.extend_from_slice(key);
        assert_eq!(
            m.hobj.hash,
            ceph_str_hash_rjenkins(&input),
            "msg {} hash",
            c.n
        );
        assert_eq!(m.pgid.pgid.pool, pool as u64);
        assert_eq!(
            m.pgid.pgid.seed,
            ceph_stable_mod(m.hobj.hash, 8, 7),
            "msg {} pg",
            c.n
        );
        assert_eq!(m.retry_attempt, 0);
        assert_eq!(m.features, FEATURES);
        // re-encode: identical front and data segments
        let back = m.to_raw_for(FEATURES).unwrap();
        assert_eq!(back.version, c.raw.version);
        assert_eq!(back.front, c.raw.front, "msg {} front", c.n);
        assert_eq!(back.data, c.raw.data, "msg {} data", c.n);
    }
}

#[test]
fn mosdop_contents() {
    let (reqs, _, _) = load();
    let by_tid: BTreeMap<u64, MOsdOp> = reqs
        .iter()
        .map(|c| (c.tid, MOsdOp::decode_raw(&c.raw).unwrap()))
        .collect();
    // tid 1: write_full obj1
    let m = &by_tid[&1];
    assert_eq!(&m.hobj.oid[..], b"obj1");
    assert_eq!(m.ops.len(), 1);
    assert_eq!(m.ops[0].op, op::WRITEFULL);
    assert_eq!(&m.ops[0].indata[..], b"hello world");
    assert_eq!(
        (m.ops[0].extent_offset(), m.ops[0].extent_length()),
        (0, 11)
    );
    assert!(m.flags & flags::WRITE != 0);
    // tid 2: create excl + setxattr + omap_set
    let m = &by_tid[&2];
    let codes: Vec<u16> = m.ops.iter().map(|o| o.op).collect();
    assert_eq!(codes, vec![op::CREATE, op::SETXATTR, op::OMAPSETVALS]);
    assert_eq!(m.ops[0].flags, ops::op_flags::EXCL);
    assert_eq!(&m.ops[1].indata[..], b"parentabc");
    // tid 3: reads
    let codes: Vec<u16> = by_tid[&3].ops.iter().map(|o| o.op).collect();
    assert_eq!(
        codes,
        vec![
            op::STAT,
            op::READ,
            op::GETXATTRS,
            op::OMAPGETVALS,
            op::OMAPGETVALSBYKEYS
        ]
    );
    assert!(by_tid[&3].flags & flags::READ != 0);
    // tid 5: cmpxattr + write
    let m = &by_tid[&5];
    assert_eq!(m.ops[0].op, op::CMPXATTR);
    assert_eq!(&m.ops[0].indata[..], b"parentabc");
    assert_eq!(m.ops[0].args[8], ops::cmp_op::EQ);
    assert_eq!(m.ops[0].args[9], ops::cmp_mode::STRING);
    // tid 7: assert_version + truncate
    assert_eq!(by_tid[&7].ops[0].op, op::ASSERT_VER);
    assert_eq!(by_tid[&7].ops[1].op, op::TRUNCATE);
    // a write with a self-managed snap context
    let snapped: Vec<&MOsdOp> = by_tid.values().filter(|m| !m.snaps.is_empty()).collect();
    assert_eq!(snapped.len(), 2);
    assert!(
        snapped
            .iter()
            .all(|m| m.snap_seq == m.snaps[0] && m.snaps.len() == 2)
    );
    // namespace and locator key
    assert!(
        by_tid
            .values()
            .any(|m| &m.hobj.nspace[..] == b"ns1" && &m.hobj.oid[..] == b"obj3")
    );
    assert!(
        by_tid
            .values()
            .any(|m| &m.hobj.key[..] == b"lockey" && &m.hobj.oid[..] == b"obj4")
    );
}

#[test]
fn mosdopreply_decodes_and_reencodes_identically() {
    let (reqs, replies, _) = load();
    let req_by_tid: BTreeMap<u64, MOsdOp> = reqs
        .iter()
        .map(|c| (c.tid, MOsdOp::decode_raw(&c.raw).unwrap()))
        .collect();
    let mut results = BTreeMap::new();
    for c in &replies {
        let r = MOsdOpReply::decode_raw(&c.raw).unwrap_or_else(|e| panic!("msg {}: {e}", c.n));
        let req = &req_by_tid[&c.tid];
        assert_eq!(r.ops.len(), req.ops.len(), "msg {}", c.n);
        assert_eq!(r.oid, req.hobj.oid);
        assert_eq!(r.retry_attempt, 0);
        assert!(r.redirect.is_none());
        results.insert(c.tid, r.result);
        let back = r.to_raw();
        assert_eq!(back.front, c.raw.front, "msg {} front", c.n);
        assert_eq!(back.data, c.raw.data, "msg {} data", c.n);
    }
    // results the driver saw: ERANGE for the stale assert_version, EEXIST for the exclusive
    // create of an existing object, ENOENT for stat/remove of a missing object
    let errs: Vec<i32> = results.values().copied().filter(|r| *r < 0).collect();
    assert_eq!(errs, vec![-34, -17, -2, -2]);
}

#[test]
fn mosdopreply_outputs() {
    let (_, replies, _) = load();
    let r = replies
        .iter()
        .find(|c| c.tid == 3)
        .map(|c| MOsdOpReply::decode_raw(&c.raw).unwrap())
        .unwrap();
    assert_eq!(r.result, 0);
    let (size, _mtime) = ops::decode_stat(&r.ops[0].outdata).unwrap();
    assert_eq!(size, 11);
    assert_eq!(&r.ops[1].outdata[..], b"hello world");
    // obj1 has no omap: empty map, "more" false
    let (vals, more) = ops::decode_omap_vals(&r.ops[3].outdata, 10).unwrap();
    assert!(vals.is_empty() && !more);
    let r = replies
        .iter()
        .find(|c| c.tid == 4)
        .map(|c| MOsdOpReply::decode_raw(&c.raw).unwrap())
        .unwrap();
    let (vals, more) = ops::decode_omap_vals(&r.ops[2].outdata, 10).unwrap();
    assert_eq!(vals.len(), 1, "values after k1");
    assert_eq!(&vals[&Bytes::from_static(b"k2")][..], b"value2");
    assert!(!more);
    assert!(r.user_version > 0);
}
