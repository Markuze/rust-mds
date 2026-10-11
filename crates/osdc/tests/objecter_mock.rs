//! Objecter behaviour with a mock transport and real maps (`testdata/privmon`, six OSDs).

mod common;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use common::*;
use denc::{EntityAddrVec, Message, RawMessage, features};
use osdc::messages::{CEPH_MSG_OSD_BACKOFF, CEPH_MSG_OSD_OP, CEPH_MSG_PING, MOsdBackoff, MOsdMap, MOsdOp, MOsdOpReply, RequestRedirect, backoff_op};
use osdc::objecter::{ConnSink, ObjecterConfig, ObjecterEvent, OpError, OpRequest, OsdConnection, OsdTransport};
use osdc::ops::{ObjectOperation, flags};
use osdc::osdmap::{map_flags, osd_state};
use osdc::pool::pool_flags;
use osdc::types::{EVersion, HObject, ObjectLocator, SpgId};
use osdc::{Objecter, OsdMap};
use tokio::sync::mpsc;

struct MockConn {
    osd: i32,
    sent: Mutex<Vec<RawMessage>>,
    features: AtomicU64,
    down: AtomicBool,
    sink: ConnSink,
}

impl OsdConnection for MockConn {
    fn send(&self, msg: RawMessage) -> bool {
        if self.down.load(Ordering::SeqCst) {
            return false;
        }
        self.sent.lock().unwrap().push(msg);
        true
    }
    fn features(&self) -> u64 {
        self.features.load(Ordering::SeqCst)
    }
    fn mark_down(&self) {
        self.down.store(true, Ordering::SeqCst);
    }
}

#[derive(Default)]
struct MockTransport {
    conns: Mutex<Vec<Arc<MockConn>>>,
    features: AtomicU64,
}

impl OsdTransport for MockTransport {
    fn connect(&self, osd: i32, _addrs: &EntityAddrVec, sink: ConnSink) -> Arc<dyn OsdConnection> {
        let c = Arc::new(MockConn {
            osd,
            sent: Mutex::new(Vec::new()),
            features: AtomicU64::new(self.features.load(Ordering::SeqCst)),
            down: AtomicBool::new(false),
            sink,
        });
        self.conns.lock().unwrap().push(c.clone());
        c
    }
}

impl MockTransport {
    fn new(features: u64) -> Arc<Self> {
        let t = MockTransport::default();
        t.features.store(features, Ordering::SeqCst);
        Arc::new(t)
    }

    fn live_conn(&self, osd: i32) -> Option<Arc<MockConn>> {
        self.conns.lock().unwrap().iter().rev().find(|c| c.osd == osd && !c.down.load(Ordering::SeqCst)).cloned()
    }

    fn conn_count(&self, osd: i32) -> usize {
        self.conns.lock().unwrap().iter().filter(|c| c.osd == osd).count()
    }

    /// All messages of `msg_type` sent so far, with their connection.
    fn sent(&self, msg_type: u16) -> Vec<(Arc<MockConn>, RawMessage)> {
        let mut v = Vec::new();
        for c in self.conns.lock().unwrap().iter() {
            for m in c.sent.lock().unwrap().iter() {
                if m.msg_type == msg_type {
                    v.push((c.clone(), m.clone()));
                }
            }
        }
        v
    }

    /// Wait until `n` MOSDOps have been sent in total; return the last.
    async fn op_n(&self, n: usize) -> (Arc<MockConn>, MOsdOp) {
        for _ in 0..200 {
            let ops = self.sent(CEPH_MSG_OSD_OP);
            if ops.len() >= n {
                let (c, m) = ops[n - 1].clone();
                return (c, MOsdOp::decode_raw(&m).unwrap());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("op #{n} not sent (sent {})", self.sent(CEPH_MSG_OSD_OP).len());
    }

    async fn settle() {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    fn op_count(&self) -> usize {
        self.sent(CEPH_MSG_OSD_OP).len()
    }
}

fn reply_to(conn: &MockConn, m: &MOsdOp, result: i32, out: Vec<Bytes>) {
    let mut ops = m.ops.clone();
    for (i, o) in ops.iter_mut().enumerate() {
        o.indata = Bytes::new();
        o.outdata = out.get(i).cloned().unwrap_or_default();
        o.rval = result.min(0);
    }
    let r = MOsdOpReply {
        tid: m.tid,
        oid: m.hobj.oid.clone(),
        pgid: m.pgid.pgid,
        flags: i64::from(m.flags & !(flags::ACK | flags::ONDISK | flags::ONNVRAM) | flags::ONDISK),
        result,
        bad_replay_version: EVersion { version: 7, epoch: m.osdmap_epoch },
        osdmap_epoch: m.osdmap_epoch,
        ops,
        retry_attempt: m.retry_attempt,
        replay_version: EVersion { version: 7, epoch: m.osdmap_epoch },
        user_version: 7,
        redirect: None,
    };
    conn.sink.message(r.to_raw());
}

/// The first privmon epoch where pools `rep` and `ec` exist and all six OSDs are up and in.
fn base_map() -> OsdMap {
    for e in 1..200 {
        let p = testdata().join(format!("privmon/full.{e}"));
        if !p.exists() {
            continue;
        }
        let m = load_map(&p);
        let pools = m.pool_id("rep").is_some() && m.pool_id("ec").is_some();
        let all_up = (0..6).all(|o| m.is_up(o) && m.is_in(o));
        if pools && all_up && m.pg_temp.is_empty() && m.osd_primary_affinity.is_none() {
            return m;
        }
    }
    unreachable!()
}

fn next_epoch(m: &OsdMap) -> OsdMap {
    let mut n = m.clone();
    n.epoch += 1;
    n
}

fn write_req(map: &OsdMap, name: &str) -> OpRequest {
    let pool = map.pool_id("rep").unwrap();
    let mut op = ObjectOperation::new();
    op.write_full(Bytes::from_static(b"data"));
    OpRequest::new(ObjectLocator::new(pool), name.to_string(), op)
}

fn read_req(map: &OsdMap, name: &str) -> OpRequest {
    let pool = map.pool_id("rep").unwrap();
    let mut op = ObjectOperation::new();
    op.read(0, 0);
    OpRequest::new(ObjectLocator::new(pool), name.to_string(), op)
}

fn primary_of(map: &OsdMap, name: &str) -> i32 {
    let pool = map.pool_id("rep").unwrap();
    map.map_object(pool, name.as_bytes(), b"").unwrap().sets.acting_primary
}

fn start(t: &Arc<MockTransport>, cfg: ObjecterConfig) -> (Objecter, mpsc::UnboundedReceiver<ObjecterEvent>) {
    Objecter::start(t.clone(), cfg)
}

async fn drain(ev: &mut mpsc::UnboundedReceiver<ObjecterEvent>) -> Vec<ObjecterEvent> {
    MockTransport::settle().await;
    let mut v = Vec::new();
    while let Ok(e) = ev.try_recv() {
        v.push(e);
    }
    v
}

#[tokio::test]
async fn op_waits_for_first_map_then_goes_to_the_primary() {
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    let evs = drain(&mut ev).await;
    assert!(evs.contains(&ObjecterEvent::RequestMap { start: 0, onetime: true }), "{evs:?}");
    let map = base_map();
    let fut = o.submit(write_req(&map, "foo"));
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 0, "no map yet");

    // the monitor's first answer: a full map
    let full = read(&testdata().join(format!("privmon/full.{}", map.epoch)));
    let mut m = MOsdMap { fsid: map.fsid, newest_map: map.epoch, ..Default::default() };
    m.maps.insert(map.epoch, full);
    o.handle_osd_map(m);
    let (conn, op) = t.op_n(1).await;
    assert_eq!(conn.osd, primary_of(&map, "foo"));
    for f in [flags::WRITE, flags::ONDISK, flags::KNOWN_REDIR, flags::SUPPORTSPOOLEIO, flags::FULL_FORCE] {
        assert!(op.flags & f != 0, "flag {f:#x} missing in {:#x}", op.flags);
    }
    assert_eq!(op.flags & flags::RETRY, 0);
    assert_eq!(op.retry_attempt, 0);
    assert_eq!(op.osdmap_epoch, map.epoch);
    let pl = map.map_object(map.pool_id("rep").unwrap(), b"foo", b"").unwrap();
    assert_eq!(op.pgid, SpgId::new(pl.pg));
    assert_eq!(op.hobj.hash, pl.raw_pg.seed);
    assert_eq!(&op.ops[0].indata[..], b"data");
    reply_to(&conn, &op, 0, vec![]);
    let r = fut.await.unwrap().unwrap();
    assert_eq!(r.result, 0);
    assert_eq!(r.user_version, 7);
    let evs = drain(&mut ev).await;
    assert!(evs.contains(&ObjecterEvent::MapApplied { epoch: map.epoch }), "{evs:?}");
    assert_eq!(o.epoch(), map.epoch);
}

#[tokio::test]
async fn primary_change_resends_with_retry_flag_and_ignores_stale_replies() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    let fut = o.submit(write_req(&map, "obj-a"));
    let (c0, op0) = t.op_n(1).await;
    let p0 = c0.osd;
    // the primary goes down
    let mut m2 = next_epoch(&map);
    m2.osd_state[p0 as usize] &= !osd_state::UP;
    let p1 = primary_of(&m2, "obj-a");
    assert_ne!(p0, p1);
    o.install_map(m2.clone());
    let (c1, op1) = t.op_n(2).await;
    assert_eq!(c1.osd, p1);
    assert!(c0.down.load(Ordering::SeqCst), "session to the down osd closed");
    assert_eq!(op1.tid, op0.tid, "same tid on resend");
    assert_eq!(op1.retry_attempt, 1);
    assert!(op1.flags & flags::RETRY != 0);
    assert_eq!(op1.osdmap_epoch, m2.epoch);
    // a late reply to attempt 0 (delivered through the new session) is ignored
    let mut stale = op1.clone();
    stale.retry_attempt = 0;
    reply_to(&c1, &stale, -5, vec![]);
    MockTransport::settle().await;
    reply_to(&c1, &op1, 0, vec![]);
    assert_eq!(fut.await.unwrap().unwrap().result, 0);
}

#[tokio::test]
async fn unchanged_mapping_is_not_resent() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    let _fut = o.submit(write_req(&map, "steady"));
    t.op_n(1).await;
    o.install_map(next_epoch(&map));
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 1);
}

#[tokio::test]
async fn pause_flags_hold_writes_and_reads_separately() {
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    let mut paused = map.clone();
    paused.flags |= map_flags::PAUSEWR;
    o.install_map(paused.clone());
    let w = o.submit(write_req(&map, "w1"));
    let r = o.submit(read_req(&map, "r1"));
    let (cr, opr) = t.op_n(1).await;
    assert!(opr.flags & flags::READ != 0, "only the read goes out");
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 1);
    let evs = drain(&mut ev).await;
    assert!(evs.iter().any(|e| matches!(e, ObjecterEvent::RequestMap { onetime: false, .. })), "continuous subscription while paused: {evs:?}");
    reply_to(&cr, &opr, 0, vec![Bytes::from_static(b"x")]);
    assert_eq!(r.await.unwrap().unwrap().out(0), Bytes::from_static(b"x"));
    // unpause
    let unpaused = next_epoch(&map);
    let mut unpaused = unpaused;
    unpaused.epoch = paused.epoch + 1;
    o.install_map(unpaused);
    let (cw, opw) = t.op_n(2).await;
    assert!(opw.flags & flags::WRITE != 0);
    reply_to(&cw, &opw, 0, vec![]);
    assert_eq!(w.await.unwrap().unwrap().result, 0);
}

#[tokio::test]
async fn full_flags_do_not_stop_mds_writes() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let mut map = base_map();
    map.flags |= map_flags::FULL;
    let pool = map.pool_id("rep").unwrap();
    map.pools.get_mut(&pool).unwrap().flags |= pool_flags::FULL;
    o.install_map(map.clone());
    let _w = o.submit(write_req(&map, "full"));
    let (_, op) = t.op_n(1).await;
    assert!(op.flags & flags::FULL_FORCE != 0);
}

#[tokio::test]
async fn epoch_barrier_pauses_until_the_map_arrives() {
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    MockTransport::settle().await;
    drain(&mut ev).await;
    o.set_epoch_barrier(map.epoch + 2);
    let w = o.submit(write_req(&map, "barrier"));
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 0);
    let evs = drain(&mut ev).await;
    assert!(evs.contains(&ObjecterEvent::RequestMap { start: map.epoch + 1, onetime: true }), "{evs:?}");
    let m1 = next_epoch(&map);
    o.install_map(m1.clone());
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 0, "still below the barrier");
    o.install_map(next_epoch(&m1));
    let (c, op) = t.op_n(1).await;
    assert_eq!(op.osdmap_epoch, map.epoch + 2);
    reply_to(&c, &op, 0, vec![]);
    w.await.unwrap().unwrap();
    // waiting for a map
    let waiter = o.wait_for_map(map.epoch + 3);
    let o2 = o.clone();
    let m3 = next_epoch(&next_epoch(&m1));
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        o2.install_map(m3);
    });
    assert_eq!(waiter.await, map.epoch + 3);
}

#[tokio::test]
async fn eagain_and_redirect_resubmit() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    let fut = o.submit(read_req(&map, "again"));
    let (c, op) = t.op_n(1).await;
    reply_to(&c, &op, -11, vec![]);
    let (c2, op2) = t.op_n(2).await;
    assert_ne!(op2.tid, op.tid, "-EAGAIN resubmits with a new tid");
    assert_eq!(op2.retry_attempt, 1);
    // now a redirect (cache tiering: "go to the base pool"); as in C++ the op is resubmitted
    // with IGNORE_OVERLAY, so it targets its base locator again, here pool `rep`
    let ec = map.pool_id("ec").unwrap();
    let rep = map.pool_id("rep").unwrap();
    let mut r = MOsdOpReply {
        tid: op2.tid,
        oid: op2.hobj.oid.clone(),
        pgid: op2.pgid.pgid,
        flags: 0,
        result: 0,
        bad_replay_version: EVersion::default(),
        osdmap_epoch: map.epoch,
        ops: vec![],
        retry_attempt: op2.retry_attempt,
        replay_version: EVersion::default(),
        user_version: 0,
        redirect: Some(RequestRedirect { locator: ObjectLocator::new(ec), object: Bytes::new() }),
    };
    c2.sink.message(r.to_raw());
    let (c3, op3) = t.op_n(3).await;
    assert_eq!(op3.hobj.pool, rep);
    assert_ne!(op3.tid, op2.tid);
    for f in [flags::REDIRECTED, flags::IGNORE_CACHE, flags::IGNORE_OVERLAY] {
        assert!(op3.flags & f != 0);
    }
    r.redirect = None;
    reply_to(&c3, &op3, 0, vec![Bytes::from_static(b"ok")]);
    assert_eq!(fut.await.unwrap().unwrap().out(0), Bytes::from_static(b"ok"));
}

#[tokio::test]
async fn blocklisted_reply_fences_the_daemon() {
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    let fut = o.submit(write_req(&map, "fenced"));
    let (c, op) = t.op_n(1).await;
    reply_to(&c, &op, -108, vec![]);
    assert_eq!(fut.await.unwrap(), Err(OpError::Blocklisted));
    assert!(drain(&mut ev).await.contains(&ObjecterEvent::Fenced));
}

#[tokio::test]
async fn osd_errors_are_returned_in_the_reply() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    let fut = o.submit(read_req(&map, "nothere"));
    let (c, op) = t.op_n(1).await;
    reply_to(&c, &op, -2, vec![]);
    assert_eq!(fut.await.unwrap().unwrap().result, -2);
    // the convenience helpers turn it into an error
    let o2 = o.clone();
    let pool = map.pool_id("rep").unwrap();
    let h = tokio::spawn(async move { o2.read(ObjectLocator::new(pool), "nothere2", 0, 0).await });
    let (c, op) = t.op_n(2).await;
    reply_to(&c, &op, -2, vec![]);
    assert_eq!(h.await.unwrap(), Err(OpError::Rados(-2)));
}

#[tokio::test]
async fn backoff_holds_ops_until_unblocked() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    let first = o.submit(write_req(&map, "bo"));
    let (c, op) = t.op_n(1).await;
    // the OSD blocks the whole pg
    let pool = map.pool_id("rep").unwrap();
    let begin = HObject::new(Bytes::new(), Bytes::new(), 0, 0, pool, Bytes::new());
    let end = HObject { max: true, ..begin.clone() };
    let block = MOsdBackoff { pgid: op.pgid, map_epoch: map.epoch, op: backoff_op::BLOCK, id: 42, begin: begin.clone(), end: end.clone() };
    c.sink.message(block.to_raw(features::ALL));
    MockTransport::settle().await;
    let acks = t.sent(CEPH_MSG_OSD_BACKOFF);
    assert_eq!(acks.len(), 1);
    let ack = MOsdBackoff::from_raw(acks[0].1.clone()).unwrap();
    assert_eq!((ack.op, ack.id, ack.map_epoch), (backoff_op::ACK_BLOCK, 42, map.epoch));
    assert_eq!(acks[0].1.priority, 63);
    // a new op to the same object is held
    let second = o.submit(write_req(&map, "bo"));
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 1);
    // unblock: every op of the session in the range goes out again, in tid order (the OSD
    // drops ops that hit a backoff, so the first one is resent too)
    let unblock = MOsdBackoff { op: backoff_op::UNBLOCK, ..block };
    c.sink.message(unblock.to_raw(features::ALL));
    let (_, again) = t.op_n(2).await;
    let (_, op2) = t.op_n(3).await;
    assert_eq!(again.tid, op.tid);
    assert_eq!(again.retry_attempt, 1);
    assert_ne!(op2.tid, op.tid);
    assert_eq!(op2.retry_attempt, 0);
    reply_to(&c, &again, 0, vec![]);
    reply_to(&c, &op2, 0, vec![]);
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();
}

#[tokio::test]
async fn connection_reset_resends_in_tid_order() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    // three objects with the same primary
    let p = primary_of(&map, "r0");
    let names: Vec<String> = (0..200).map(|i| format!("r{i}")).filter(|n| primary_of(&map, n) == p).take(3).collect();
    let futs: Vec<_> = names.iter().map(|n| o.submit(write_req(&map, n))).collect();
    t.op_n(3).await;
    let c = t.live_conn(p).unwrap();
    c.sink.reset();
    let (_, _) = t.op_n(6).await;
    assert_eq!(t.conn_count(p), 2, "a new connection");
    let c2 = t.live_conn(p).unwrap();
    let resent: Vec<MOsdOp> = c2.sent.lock().unwrap().iter().map(|m| MOsdOp::decode_raw(m).unwrap()).collect();
    let tids: Vec<u64> = resent.iter().map(|m| m.tid).collect();
    let mut sorted = tids.clone();
    sorted.sort();
    assert_eq!(tids, sorted);
    assert!(resent.iter().all(|m| m.retry_attempt == 1 && m.flags & flags::RETRY != 0));
    for m in &resent {
        reply_to(&c2, m, 0, vec![]);
    }
    for f in futs {
        f.await.unwrap().unwrap();
    }
}

#[tokio::test]
async fn missing_and_eio_pools() {
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    let mut map = base_map();
    // a pool that never existed, with the monitor's latest version known
    o.install_map(map.clone());
    o.set_latest_map_version(map.epoch);
    let mut op = ObjectOperation::new();
    op.stat();
    let r = o.operate(OpRequest::new(ObjectLocator::new(4242), "x", op.clone())).await;
    assert_eq!(r, Err(OpError::PoolDoesNotExist));
    // unknown latest version: ask, then conclude when told
    let (t2, (o2, mut ev2)) = {
        let t2 = MockTransport::new(features::ALL);
        let s = start(&t2, ObjecterConfig::default());
        (t2, s)
    };
    let _ = &t2;
    o2.install_map(map.clone());
    let fut = o2.submit(OpRequest::new(ObjectLocator::new(4242), "x", op.clone()));
    let evs = drain(&mut ev2).await;
    assert!(evs.contains(&ObjecterEvent::CheckLatestMap), "{evs:?}");
    o2.set_latest_map_version(map.epoch);
    assert_eq!(fut.await.unwrap(), Err(OpError::PoolDoesNotExist));
    // a pool deleted while an op waits for it (paused by PAUSEWR)
    map.flags |= map_flags::PAUSEWR;
    let rep = map.pool_id("rep").unwrap();
    o.install_map(map.clone());
    let w = o.submit(write_req(&map, "gone"));
    MockTransport::settle().await;
    let mut m2 = next_epoch(&map);
    m2.pools.remove(&rep);
    o.install_map(m2.clone());
    assert_eq!(w.await.unwrap(), Err(OpError::PoolDoesNotExist));
    // FLAG_EIO
    let mut m3 = next_epoch(&m2);
    m3.flags &= !map_flags::PAUSEWR;
    let ec = m3.pool_id("ec").unwrap();
    m3.pools.get_mut(&ec).unwrap().flags |= pool_flags::EIO;
    o.install_map(m3);
    let r = o.operate(OpRequest::new(ObjectLocator::new(ec), "y", op)).await;
    assert_eq!(r, Err(OpError::PoolEio));
    drain(&mut ev).await;
}

#[tokio::test]
async fn laggy_ops_ping_and_time_out() {
    let t = MockTransport::new(features::ALL);
    let cfg = ObjecterConfig {
        tick_interval: Duration::from_millis(30),
        laggy_timeout: Duration::from_millis(40),
        op_timeout: Some(Duration::from_millis(400)),
        ..Default::default()
    };
    let (o, _ev) = start(&t, cfg);
    let map = base_map();
    o.install_map(map.clone());
    let fut = o.submit(write_req(&map, "slow"));
    t.op_n(1).await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(!t.sent(CEPH_MSG_PING).is_empty(), "laggy op pings its osd");
    assert_eq!(fut.await.unwrap(), Err(OpError::TimedOut));
}

#[tokio::test]
async fn incrementals_and_blocklist_events_via_mosdmap() {
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    // privmon epochs 45..=60 include blocklist additions
    let first = 45;
    let full = read(&testdata().join(format!("privmon/full.{first}")));
    let fsid = load_map(&testdata().join(format!("privmon/full.{first}"))).fsid;
    let mut m = MOsdMap { fsid, newest_map: 60, ..Default::default() };
    m.maps.insert(first, full);
    o.handle_osd_map(m);
    let mut inc = MOsdMap { fsid, newest_map: 60, ..Default::default() };
    for e in first + 1..=60 {
        inc.incremental_maps.insert(e, read(&testdata().join(format!("privmon/inc.{e}"))));
    }
    // deliver the incrementals as an OSD would, over an OSD session
    let map = load_map(&testdata().join(format!("privmon/full.{first}")));
    let _w = o.submit(read_req(&map, "via-osd"));
    let (c, _) = t.op_n(1).await;
    c.sink.message(inc.to_raw(features::ALL));
    assert_eq!(o.wait_for_map(60).await, 60);
    let want = load_map(&testdata().join("privmon/full.60"));
    assert_eq!(*o.osdmap(), want);
    let evs = drain(&mut ev).await;
    let blocklisted: Vec<_> = evs.iter().filter_map(|e| if let ObjecterEvent::Blocklisted { addrs } = e { Some(addrs.clone()) } else { None }).flatten().collect();
    assert!(blocklisted.len() >= 3, "{evs:?}");
    assert!(evs.contains(&ObjecterEvent::MapApplied { epoch: 60 }));
}

#[tokio::test]
async fn encoding_follows_osd_features() {
    // an OSD without SERVER_SQUID gets MOSDOp v8
    let t = MockTransport::new(features::ALL & !features::SERVER_SQUID);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    let _w = o.submit(write_req(&map, "v8"));
    MockTransport::settle().await;
    let sent = t.sent(CEPH_MSG_OSD_OP);
    assert_eq!(sent[0].1.version, 8);
    // features unknown before the handshake: the OSD's features in the map decide (v9)
    let t9 = MockTransport::new(0);
    let (o9, _ev9) = start(&t9, ObjecterConfig::default());
    o9.install_map(map.clone());
    let _w = o9.submit(write_req(&map, "v9"));
    MockTransport::settle().await;
    assert_eq!(t9.sent(CEPH_MSG_OSD_OP)[0].1.version, 9);
}

#[tokio::test]
async fn snap_context_and_namespace_and_status() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig { client_inc: 17, ..Default::default() });
    let map = base_map();
    o.install_map(map.clone());
    let pool = map.pool_id("rep").unwrap();
    let mut op = ObjectOperation::new();
    op.remove();
    let req = OpRequest::new(ObjectLocator::with_namespace(pool, "ns1"), "snappy", op)
        .with_snapc(osdc::SnapContext { seq: 9, snaps: vec![9, 4] });
    let fut = o.submit(req);
    let (c, m) = t.op_n(1).await;
    assert_eq!(&m.hobj.nspace[..], b"ns1");
    assert_eq!((m.snap_seq, m.snaps.clone()), (9, vec![9, 4]));
    assert_eq!(m.client_inc, 17);
    assert_eq!((m.reqid.inc, m.reqid.tid, m.reqid.name.ty.0), (17, 0, 0), "reqid carries client_inc");
    let pl = map.map_object(pool, b"snappy", b"ns1").unwrap();
    assert_eq!(c.osd, pl.sets.acting_primary);
    let st = o.status().await;
    assert_eq!((st.inflight, st.sessions, st.epoch), (1, 1, map.epoch));
    reply_to(&c, &m, 0, vec![]);
    fut.await.unwrap().unwrap();
    let mut by_osd = BTreeMap::new();
    for i in 0..60 {
        *by_osd.entry(primary_of(&map, &format!("spread{i}"))).or_insert(0) += 1;
    }
    assert!(by_osd.len() > 1, "objects spread over primaries");
}

#[tokio::test]
async fn pg_split_and_forced_resend() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    let pool = map.pool_id("rep").unwrap();
    o.install_map(map.clone());
    // pick an object whose pg gets a child when pg_num doubles and that moves to the child
    let name = (0..500)
        .map(|i| format!("split{i}"))
        .find(|n| {
            let raw = map.map_object(pool, n.as_bytes(), b"").unwrap().raw_pg;
            osdc::hash::ceph_stable_mod(raw.seed, 64, 63) >= 32
        })
        .unwrap();
    let fut = o.submit(write_req(&map, &name));
    let (_, op1) = t.op_n(1).await;
    // pg_num 32 -> 64: the target pg splits, the op is resent to the child pg
    let mut m2 = next_epoch(&map);
    {
        let p = m2.pools.get_mut(&pool).unwrap();
        p.pg_num = 64;
        p.pg_num_target = 64;
        p.pg_num_pending = 64;
        p.calc_pg_masks();
    }
    o.install_map(m2.clone());
    let (c2, op2) = t.op_n(2).await;
    assert_eq!(op2.tid, op1.tid);
    assert_eq!(op2.retry_attempt, 1);
    assert_eq!(op2.pgid.pgid.seed, op1.pgid.pgid.seed + 32, "resent to the child pg");
    // the pool's last_force_op_resend equal to the map epoch forces another resend
    let mut m3 = next_epoch(&m2);
    m3.pools.get_mut(&pool).unwrap().last_force_op_resend = m3.epoch;
    o.install_map(m3);
    let (c3, op3) = t.op_n(3).await;
    assert_eq!((op3.tid, op3.retry_attempt), (op1.tid, 2));
    let _ = c2;
    reply_to(&c3, &op3, 0, vec![]);
    fut.await.unwrap().unwrap();
}
