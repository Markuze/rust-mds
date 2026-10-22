//! Objecter behaviour with a mock transport and real maps (`testdata/privmon`, six OSDs).

mod common;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use common::*;
use denc::{EntityAddrVec, Message, RawMessage, features};
use osdc::messages::{
    CEPH_MSG_OSD_BACKOFF, CEPH_MSG_OSD_OP, CEPH_MSG_PING, MOsdBackoff, MOsdMap, MOsdOp,
    MOsdOpReply, RequestRedirect, backoff_op,
};
use osdc::objecter::{
    ConnSink, ObjecterConfig, ObjecterEvent, OpError, OpRequest, OsdConnection, OsdTransport,
};
use osdc::ops::{ObjectOperation, flags};
use osdc::osdmap::{map_flags, osd_state};
use osdc::pool::pool_flags;
use osdc::types::{EVersion, HObject, ObjectLocator, SpgId};
use osdc::{NamespaceScope, PoolListCursor, PoolListError, PoolListPage, PoolObject};
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
        self.conns
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|c| c.osd == osd && !c.down.load(Ordering::SeqCst))
            .cloned()
    }

    fn conn_count(&self, osd: i32) -> usize {
        self.conns
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.osd == osd)
            .count()
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
        panic!(
            "op #{n} not sent (sent {})",
            self.sent(CEPH_MSG_OSD_OP).len()
        );
    }

    /// PG listing can visit different OSDs. `sent` groups by connection, not time.
    async fn list_op(&self, tid: u64) -> (Arc<MockConn>, MOsdOp) {
        for _ in 0..200 {
            if let Some((conn, msg)) = self
                .sent(CEPH_MSG_OSD_OP)
                .into_iter()
                .find(|(_, msg)| msg.tid == tid)
            {
                return (conn, MOsdOp::decode_raw(&msg).unwrap());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("listing tid {tid} not sent");
    }

    async fn settle() {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    fn op_count(&self) -> usize {
        self.sent(CEPH_MSG_OSD_OP).len()
    }
}

fn reply_to(conn: &MockConn, m: &MOsdOp, result: i32, out: Vec<Bytes>) {
    reply_with_results(conn, m, result, out, &[]);
}

fn reply_with_results(conn: &MockConn, m: &MOsdOp, result: i32, out: Vec<Bytes>, rvals: &[i32]) {
    let mut ops = m.ops.clone();
    for (i, o) in ops.iter_mut().enumerate() {
        o.indata = Bytes::new();
        o.outdata = out.get(i).cloned().unwrap_or_default();
        o.rval = rvals.get(i).copied().unwrap_or(result.min(0));
    }
    let r = MOsdOpReply {
        tid: m.tid,
        oid: m.hobj.oid.clone(),
        pgid: m.pgid.pgid,
        flags: i64::from(m.flags & !(flags::ACK | flags::ONDISK | flags::ONNVRAM) | flags::ONDISK),
        result,
        bad_replay_version: EVersion {
            version: 7,
            epoch: m.osdmap_epoch,
        },
        osdmap_epoch: m.osdmap_epoch,
        ops,
        retry_attempt: m.retry_attempt,
        replay_version: EVersion {
            version: 7,
            epoch: m.osdmap_epoch,
        },
        user_version: 7,
        redirect: None,
    };
    conn.sink.message(r.to_raw());
}

fn listing_map() -> OsdMap {
    let mut map = base_map();
    let id = map.pool_id("rep").unwrap();
    let pool = map.pools.get_mut(&id).unwrap();
    pool.pg_num = 4;
    pool.pgp_num = 4;
    pool.pg_num_pending = 4;
    pool.calc_pg_masks();
    map
}

// Independent oracle for a four-PG pool: PG ranges each span 2^30 reversed hashes.
fn listing_end(pool: i64, pg: u32) -> HObject {
    let end = u64::from(pg.reverse_bits()) + (1u64 << 30);
    HObject {
        max: end == 1u64 << 32,
        ..HObject::new(
            Bytes::new(),
            Bytes::new(),
            if end == 1u64 << 32 {
                0
            } else {
                osdc::types::CEPH_NOSNAP
            },
            if end == 1u64 << 32 {
                0
            } else {
                (end as u32).reverse_bits()
            },
            if end == 1u64 << 32 { 0 } else { pool },
            Bytes::new(),
        )
    }
}

fn list_response(handle: &HObject, objects: &[PoolObject]) -> Bytes {
    use denc::{BytesMut, Encode};
    let mut out = BytesMut::new();
    let frame = denc::encode_start(&mut out, 1, 1);
    handle.encode(&mut out, 0);
    (objects.len() as u32).encode(&mut out, 0);
    for object in objects {
        Bytes::copy_from_slice(&object.namespace).encode(&mut out, 0);
        Bytes::copy_from_slice(&object.name).encode(&mut out, 0);
        Bytes::copy_from_slice(&object.locator).encode(&mut out, 0);
    }
    denc::encode_finish(&mut out, frame);
    out.freeze()
}

fn list_object(map: &OsdMap, pg: u32, namespace: &[u8], ordinal: usize) -> PoolObject {
    let pool = &map.pools[&map.pool_id("rep").unwrap()];
    let mut found = 0;
    for n in 0..10000 {
        let name = format!("foreign-{n}").into_bytes();
        if pool.hash_key(&name, namespace) & 3 == pg {
            if found == ordinal {
                return PoolObject {
                    namespace: namespace.to_vec(),
                    name,
                    locator: vec![],
                };
            }
            found += 1;
        }
    }
    unreachable!()
}

fn object_handle(map: &OsdMap, object: &PoolObject) -> HObject {
    let pool = map.pool_id("rep").unwrap();
    HObject::new(
        object.name.clone(),
        object.locator.clone(),
        osdc::types::CEPH_NOSNAP,
        map.pools[&pool].hash_key(&object.name, &object.namespace),
        pool,
        object.namespace.clone(),
    )
}

fn list_page(
    o: &Objecter,
    pool: i64,
    cursor: Option<PoolListCursor>,
    count: u32,
    bytes: usize,
) -> tokio::task::JoinHandle<Result<PoolListPage, PoolListError>> {
    let o = o.clone();
    tokio::spawn(async move {
        o.list_pool_page(pool, NamespaceScope::All, cursor, count, bytes)
            .await
    })
}

#[tokio::test]
async fn pool_listing_finds_foreign_namespace_in_later_pg_and_exhausts_every_pg() {
    let map = listing_map();
    let pool = map.pool_id("rep").unwrap();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    o.install_map(map.clone());
    let _ = o.status().await;
    let foreign = list_object(&map, 3, b"other-filesystem", 0);
    let mut cursor = None;
    let mut seen = vec![];
    for pg in 0..4 {
        let pending = list_page(&o, pool, cursor, u32::MAX, 4096);
        let (conn, sent) = t.list_op(u64::from(pg) + 1).await;
        assert_eq!(sent.pgid.pgid.seed, pg);
        assert_eq!(sent.hobj.hash, pg);
        assert_eq!(sent.hobj.nspace.as_ref(), b"\x01");
        assert_eq!(sent.ops[0].op, 0x1505);
        assert_eq!(&sent.ops[0].args[..8], &1024u64.to_le_bytes());
        assert_eq!(
            sent.flags & (flags::PGOP | flags::IGNORE_OVERLAY),
            flags::PGOP | flags::IGNORE_OVERLAY
        );
        let objects = if pg == 3 {
            vec![foreign.clone()]
        } else {
            vec![]
        };
        reply_to(
            &conn,
            &sent,
            1,
            vec![list_response(&listing_end(pool, pg), &objects)],
        );
        let page = pending.await.unwrap().unwrap();
        assert_eq!(page.osdmap_epoch, map.epoch);
        seen.extend(page.objects);
        cursor = page.next;
        assert_eq!(cursor.is_none(), pg == 3);
    }
    assert_eq!(seen, vec![foreign]);
    o.shutdown();
}

#[tokio::test]
async fn pool_listing_continuation_is_inclusive_and_count_bounded() {
    use denc::Decode;
    let map = listing_map();
    let pool = map.pool_id("rep").unwrap();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    o.install_map(map.clone());
    let _ = o.status().await;
    let mut objects = [list_object(&map, 0, b"", 0), list_object(&map, 0, b"", 1)];
    objects.sort_by_key(|object| object_handle(&map, object));
    let next_handle = object_handle(&map, &objects[1]);
    let pending = list_page(&o, pool, None, 1, 4096);
    let (conn, sent) = t.op_n(1).await;
    assert_eq!(&sent.ops[0].args[..8], &1u64.to_le_bytes());
    assert_eq!(&sent.ops[0].args[8..12], &map.epoch.to_le_bytes());
    reply_to(
        &conn,
        &sent,
        0,
        vec![list_response(&next_handle, &objects[..1])],
    );
    let page = pending.await.unwrap().unwrap();
    assert_eq!(page.objects, objects[..1]);
    let cursor = page.next.unwrap();
    assert_eq!(cursor.pg, 0);
    let mut invalid = cursor.clone();
    invalid.handle = vec![0];
    assert_eq!(
        o.list_pool_page(pool, NamespaceScope::All, Some(invalid), 1, 4096)
            .await,
        Err(PoolListError::InvalidCursor)
    );
    assert_eq!(
        o.list_pool_page(
            map.pool_id("ec").unwrap(),
            NamespaceScope::All,
            Some(cursor.clone()),
            1,
            4096
        )
        .await,
        Err(PoolListError::InvalidCursor)
    );
    let pending = list_page(&o, pool, Some(cursor), 1, 4096);
    let (conn, sent) = t.op_n(2).await;
    assert_eq!(
        HObject::decode(&mut sent.ops[0].indata.clone()).unwrap(),
        next_handle
    );
    reply_to(
        &conn,
        &sent,
        1,
        vec![list_response(&listing_end(pool, 0), &objects[1..])],
    );
    let page = pending.await.unwrap().unwrap();
    assert_eq!(page.objects, objects[1..]);
    assert_eq!(page.next.unwrap().pg, 1);
    o.shutdown();
}

#[tokio::test]
async fn pool_listing_rejects_oversize_count_and_malformed_responses() {
    let map = listing_map();
    let pool = map.pool_id("rep").unwrap();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    o.install_map(map.clone());
    let _ = o.status().await;
    let object = list_object(&map, 0, b"foreign", 0);
    for (i, (count, bytes, data, expected)) in [
        (
            1,
            49,
            list_response(&listing_end(pool, 0), std::slice::from_ref(&object)),
            PoolListError::SizeLimit,
        ),
        (
            1,
            4096,
            list_response(&listing_end(pool, 0), &[object.clone(), object]),
            PoolListError::SizeLimit,
        ),
        (1, 4096, Bytes::from_static(b"bad"), PoolListError::Decode),
        (
            1,
            4096,
            list_response(
                &HObject::new(Bytes::new(), Bytes::new(), 0, 0, pool, Bytes::new()),
                &[],
            ),
            PoolListError::Decode,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let pending = list_page(&o, pool, None, count, bytes);
        let (conn, sent) = t.op_n(i + 1).await;
        reply_to(&conn, &sent, 0, vec![data]);
        assert_eq!(pending.await.unwrap(), Err(expected));
    }
    o.shutdown();
}

#[tokio::test]
async fn pool_listing_map_change_and_split_require_restart() {
    let map = listing_map();
    let pool = map.pool_id("rep").unwrap();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    o.install_map(map.clone());
    let _ = o.status().await;
    let pending = list_page(&o, pool, None, 1, 4096);
    let (conn, sent) = t.op_n(1).await;
    reply_to(
        &conn,
        &sent,
        1,
        vec![list_response(&listing_end(pool, 0), &[])],
    );
    let cursor = pending.await.unwrap().unwrap().next.unwrap();
    let pending = list_page(&o, pool, Some(cursor.clone()), 1, 4096);
    let (old_conn, old_op) = t.list_op(2).await;
    let mut split = next_epoch(&map);
    split.pools.get_mut(&pool).unwrap().pg_num = 8;
    split.pools.get_mut(&pool).unwrap().calc_pg_masks();
    o.install_map(split.clone());
    assert_eq!(pending.await.unwrap(), Err(PoolListError::MapChanged));
    assert_eq!(
        o.list_pool_page(pool, NamespaceScope::All, Some(cursor), 1, 4096)
            .await,
        Err(PoolListError::MapChanged)
    );
    reply_to(
        &old_conn,
        &old_op,
        1,
        vec![list_response(&listing_end(pool, 1), &[])],
    );
    assert_eq!(o.status().await.inflight, 0);
    // Any remap/epoch change invalidates an in-flight fresh scan too.
    let pending = list_page(&o, pool, None, 1, 4096);
    let _ = t.list_op(3).await;
    o.install_map(next_epoch(&split));
    assert_eq!(pending.await.unwrap(), Err(PoolListError::MapChanged));
    let pending = list_page(&o, pool, None, 1, 4096);
    let _ = t.list_op(4).await;
    let mut removed = o.osdmap().as_ref().clone();
    removed.epoch += 1;
    removed.pools.remove(&pool);
    o.install_map(removed);
    assert_eq!(pending.await.unwrap(), Err(PoolListError::MapChanged));
    assert_eq!(
        o.list_pool_page(pool, NamespaceScope::All, None, 1, 4096)
            .await,
        Err(PoolListError::PoolDoesNotExist)
    );
    o.shutdown();
}

#[tokio::test]
async fn pool_listing_failures_never_mean_empty_pool_and_cancellation_cleans_up() {
    let map = listing_map();
    let pool = map.pool_id("rep").unwrap();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    assert_eq!(
        o.list_pool_page(pool, NamespaceScope::All, None, 1, 4096)
            .await,
        Err(PoolListError::NoMap)
    );
    o.install_map(map.clone());
    let _ = o.status().await;
    assert_eq!(
        o.list_pool_page(999999, NamespaceScope::All, None, 1, 4096)
            .await,
        Err(PoolListError::PoolDoesNotExist)
    );
    assert_eq!(
        o.list_pool_page(pool, NamespaceScope::All, None, 0, 4096)
            .await,
        Err(PoolListError::InvalidLimits)
    );
    for (i, (result, rvals, expected)) in [
        (-5, vec![], PoolListError::Io(OpError::Rados(-5))),
        (-95, vec![], PoolListError::Unsupported),
        (0, vec![-5], PoolListError::Io(OpError::Rados(-5))),
    ]
    .into_iter()
    .enumerate()
    {
        let pending = list_page(&o, pool, None, 1, 4096);
        let (conn, sent) = t.op_n(i + 1).await;
        reply_with_results(&conn, &sent, result, vec![], &rvals);
        assert_eq!(pending.await.unwrap(), Err(expected));
    }
    let pending = list_page(&o, pool, None, 1, 4096);
    let _ = t.op_n(4).await;
    pending.abort();
    let _ = pending.await;
    assert_eq!(o.status().await.inflight, 0);
    let pending = list_page(&o, pool, None, 1, 4096);
    let _ = t.op_n(5).await;
    o.shutdown();
    assert_eq!(
        pending.await.unwrap(),
        Err(PoolListError::Io(OpError::Shutdown))
    );
}

#[tokio::test]
async fn pool_listing_has_its_own_deadline_and_prunes_the_timed_out_op() {
    // Ordinary MDS operations have no timeout. Native listing must still terminate.
    let map = listing_map();
    let pool = map.pool_id("rep").unwrap();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    o.install_map(map);
    let _ = o.status().await;
    let pending = list_page(&o, pool, None, 1, 4096);
    let _ = t.op_n(1).await;
    assert_eq!(
        pending.await.unwrap(),
        Err(PoolListError::Io(OpError::TimedOut))
    );
    assert_eq!(o.status().await.inflight, 0);
    o.shutdown();
}

#[tokio::test]
async fn pool_listing_fencing_and_unsupported_sort_order_are_errors() {
    let mut map = listing_map();
    let pool = map.pool_id("rep").unwrap();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    map.flags &= !map_flags::SORTBITWISE;
    o.install_map(map.clone());
    let _ = o.status().await;
    assert_eq!(
        o.list_pool_page(pool, NamespaceScope::All, None, 1, 4096)
            .await,
        Err(PoolListError::Unsupported)
    );
    map.flags |= map_flags::SORTBITWISE;
    map.epoch += 1;
    o.install_map(map);
    let _ = o.status().await;
    let pending = list_page(&o, pool, None, 1, 4096);
    let (conn, sent) = t.op_n(1).await;
    reply_to(&conn, &sent, -108, vec![]);
    assert_eq!(
        pending.await.unwrap(),
        Err(PoolListError::Io(OpError::Blocklisted))
    );
    assert_eq!(
        o.list_pool_page(pool, NamespaceScope::All, None, 1, 4096)
            .await,
        Err(PoolListError::Io(OpError::Blocklisted))
    );
    o.shutdown();
}

#[tokio::test]
async fn checked_compounds_require_exact_reply_cardinality() {
    let map = base_map();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    o.install_map(map.clone());
    for (i, (count, result)) in [(0, 0), (1, 0), (3, 0), (2, 0), (0, -125)]
        .into_iter()
        .enumerate()
    {
        let mut op = ObjectOperation::new();
        op.cmpxattr_str("owner", 1, b"token");
        op.write_full(Bytes::from_static(b"batch"));
        let req = OpRequest::new(
            ObjectLocator::new(map.pool_id("rep").unwrap()),
            "guarded",
            op,
        );
        let pending = {
            let o = o.clone();
            tokio::spawn(async move { o.operate_checked(req).await })
        };
        let (conn, mut sent) = t.op_n(i + 1).await;
        sent.ops.resize(count, sent.ops[0].clone());
        reply_to(&conn, &sent, result, vec![]);
        let got = pending.await.unwrap();
        if result < 0 {
            assert_eq!(got, Err(OpError::Rados(result)));
        } else if count == 2 {
            assert!(got.is_ok());
        } else {
            assert_eq!(
                got,
                Err(OpError::InvalidReply),
                "result count {count} must not advance a commit"
            );
        }
    }
    // The raw API still exposes a self-consistent, incomplete reply unchanged.
    let pending = o.submit(write_req(&map, "raw"));
    let (conn, mut sent) = t.op_n(6).await;
    sent.ops.clear();
    reply_to(&conn, &sent, 0, vec![]);
    assert!(pending.await.unwrap().unwrap().ops.is_empty());
    o.shutdown();
}

#[tokio::test]
async fn checked_compounds_reject_guard_and_per_op_failures() {
    let map = base_map();
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    o.install_map(map.clone());
    // A checked compound cannot report success when either guard fails. Also check
    // per-op negatives even if a peer returns a nonnegative top-level result.
    for (i, (result, rvals)) in [
        (-125, vec![-125, 0]), // CMPXATTR: ECANCELED
        (-34, vec![-34, 0]),   // ASSERT_VER: ERANGE
        (0, vec![0, -5]),      // mutation result: EIO
        (0, vec![-125, 0]),    // guard result: ECANCELED
        (0, vec![0, 0]),
    ]
    .into_iter()
    .enumerate()
    {
        let mut op = ObjectOperation::new();
        if i == 1 {
            op.assert_version(7);
        } else {
            op.cmpxattr_str("owner", 1, b"token");
        }
        op.write_full(Bytes::from_static(b"batch"));
        let req = OpRequest::new(
            ObjectLocator::new(map.pool_id("rep").unwrap()),
            "guarded",
            op,
        );
        let pending = {
            let o = o.clone();
            tokio::spawn(async move { o.operate_checked(req).await })
        };
        let (conn, sent) = t.op_n(i + 1).await;
        reply_with_results(&conn, &sent, result, vec![], &rvals);
        let got = pending.await.unwrap();
        let error = if result < 0 {
            Some(result)
        } else {
            rvals.into_iter().find(|v| *v < 0)
        };
        match error {
            Some(errno) => assert_eq!(got, Err(OpError::Rados(errno))),
            None => assert!(got.is_ok()),
        }
    }
    o.shutdown();
}

#[tokio::test]
async fn checked_map_waits_report_timeout_and_shutdown() {
    let t = MockTransport::new(features::ALL);
    let (o, _ev) = start(&t, ObjecterConfig::default());
    assert_eq!(
        o.wait_for_map_timeout(1, Duration::from_millis(10)).await,
        Err(OpError::TimedOut)
    );
    let map = base_map();
    let epoch = map.epoch;
    o.install_map(map);
    assert_eq!(
        o.wait_for_map_timeout(epoch, Duration::from_secs(1)).await,
        Ok(epoch)
    );
    o.set_latest_map_version(epoch + 5);
    assert_eq!(o.status().await.latest_map_version, epoch + 5);
    let pending = {
        let o = o.clone();
        tokio::spawn(async move { o.wait_for_map_result(epoch + 1).await })
    };
    MockTransport::settle().await;
    o.shutdown();
    assert_eq!(pending.await.unwrap(), Err(OpError::Shutdown));
    assert_eq!(o.wait_for_map_result(epoch).await, Err(OpError::Shutdown));
}

#[tokio::test]
async fn missing_pool_requires_fresh_latest_map_confirmation() {
    let map = base_map();
    let epoch = map.epoch;
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    o.install_map(map.clone());
    o.set_latest_map_version(epoch);
    let _ = o.status().await; // Ensure this cached answer predates the request.
    let mut op = ObjectOperation::new();
    op.stat();
    let mut pending = o.submit(OpRequest::new(ObjectLocator::new(999999), "missing", op));
    let events = drain(&mut ev).await;
    assert!(events.contains(&ObjecterEvent::CheckLatestMap));
    assert!(pending.try_recv().is_err());
    // Fresh confirmation is newer than our map. It must not finish before that map.
    o.set_latest_map_version(epoch + 1);
    MockTransport::settle().await;
    assert!(pending.try_recv().is_err());
    o.install_map(next_epoch(&map));
    assert_eq!(pending.await.unwrap(), Err(OpError::PoolDoesNotExist));
    o.shutdown();
}

#[tokio::test]
async fn latest_map_answer_does_not_cover_ops_submitted_after_query_start() {
    let map = base_map();
    let epoch = map.epoch;
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    o.install_map(map);
    let mut op = ObjectOperation::new();
    op.stat();
    let first = o.submit(OpRequest::new(
        ObjectLocator::new(999999),
        "first",
        op.clone(),
    ));
    assert!(
        drain(&mut ev)
            .await
            .contains(&ObjecterEvent::CheckLatestMap)
    );
    let mut later = o.submit(OpRequest::new(ObjectLocator::new(999999), "later", op));
    let _ = o.status().await;
    o.set_latest_map_version(epoch);
    assert_eq!(first.await.unwrap(), Err(OpError::PoolDoesNotExist));
    assert!(
        drain(&mut ev)
            .await
            .contains(&ObjecterEvent::CheckLatestMap)
    );
    assert!(later.try_recv().is_err());
    o.set_latest_map_version(epoch);
    assert_eq!(later.await.unwrap(), Err(OpError::PoolDoesNotExist));
    o.shutdown();
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
    map.map_object(pool, name.as_bytes(), b"")
        .unwrap()
        .sets
        .acting_primary
}

fn start(
    t: &Arc<MockTransport>,
    cfg: ObjecterConfig,
) -> (Objecter, mpsc::UnboundedReceiver<ObjecterEvent>) {
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
    assert!(
        evs.contains(&ObjecterEvent::RequestMap {
            start: 0,
            onetime: true
        }),
        "{evs:?}"
    );
    let map = base_map();
    let fut = o.submit(write_req(&map, "foo"));
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 0, "no map yet");

    // the monitor's first answer: a full map
    let full = read(&testdata().join(format!("privmon/full.{}", map.epoch)));
    let mut m = MOsdMap {
        fsid: map.fsid,
        newest_map: map.epoch,
        ..Default::default()
    };
    m.maps.insert(map.epoch, full);
    o.handle_osd_map(m);
    let (conn, op) = t.op_n(1).await;
    assert_eq!(conn.osd, primary_of(&map, "foo"));
    for f in [
        flags::WRITE,
        flags::ONDISK,
        flags::KNOWN_REDIR,
        flags::SUPPORTSPOOLEIO,
        flags::FULL_FORCE,
    ] {
        assert!(op.flags & f != 0, "flag {f:#x} missing in {:#x}", op.flags);
    }
    assert_eq!(op.flags & flags::RETRY, 0);
    assert_eq!(op.retry_attempt, 0);
    assert_eq!(op.osdmap_epoch, map.epoch);
    let pl = map
        .map_object(map.pool_id("rep").unwrap(), b"foo", b"")
        .unwrap();
    assert_eq!(op.pgid, SpgId::new(pl.pg));
    assert_eq!(op.hobj.hash, pl.raw_pg.seed);
    assert_eq!(&op.ops[0].indata[..], b"data");
    reply_to(&conn, &op, 0, vec![]);
    let r = fut.await.unwrap().unwrap();
    assert_eq!(r.result, 0);
    assert_eq!(r.user_version, 7);
    let evs = drain(&mut ev).await;
    assert!(
        evs.contains(&ObjecterEvent::MapApplied { epoch: map.epoch }),
        "{evs:?}"
    );
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
    assert!(
        c0.down.load(Ordering::SeqCst),
        "session to the down osd closed"
    );
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
    assert!(
        evs.iter()
            .any(|e| matches!(e, ObjecterEvent::RequestMap { onetime: false, .. })),
        "continuous subscription while paused: {evs:?}"
    );
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
    assert!(
        evs.contains(&ObjecterEvent::RequestMap {
            start: map.epoch + 1,
            onetime: true
        }),
        "{evs:?}"
    );
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
        redirect: Some(RequestRedirect {
            locator: ObjectLocator::new(ec),
            object: Bytes::new(),
        }),
    };
    c2.sink.message(r.to_raw());
    let (c3, op3) = t.op_n(3).await;
    assert_eq!(op3.hobj.pool, rep);
    assert_ne!(op3.tid, op2.tid);
    for f in [
        flags::REDIRECTED,
        flags::IGNORE_CACHE,
        flags::IGNORE_OVERLAY,
    ] {
        assert!(op3.flags & f != 0);
    }
    r.redirect = None;
    reply_to(&c3, &op3, 0, vec![Bytes::from_static(b"ok")]);
    assert_eq!(
        fut.await.unwrap().unwrap().out(0),
        Bytes::from_static(b"ok")
    );
}

#[tokio::test]
async fn blocklisted_reply_fences_the_daemon() {
    let t = MockTransport::new(features::ALL);
    let (o, mut ev) = start(&t, ObjecterConfig::default());
    let map = base_map();
    o.install_map(map.clone());
    let fut = o.submit(write_req(&map, "fenced"));
    let (c, op) = t.op_n(1).await;
    let other = o.submit(write_req(&map, "other"));
    let _ = t.op_n(2).await;
    let waiter = {
        let o = o.clone();
        tokio::spawn(async move { o.wait_for_map_result(u32::MAX).await })
    };
    reply_to(&c, &op, -108, vec![]);
    assert_eq!(fut.await.unwrap(), Err(OpError::Blocklisted));
    assert_eq!(other.await.unwrap(), Err(OpError::Blocklisted));
    assert_eq!(waiter.await.unwrap(), Err(OpError::Blocklisted));
    assert_eq!(
        o.operate(write_req(&map, "late")).await,
        Err(OpError::Blocklisted)
    );
    assert_eq!(
        o.wait_for_map_result(map.epoch).await,
        Err(OpError::Blocklisted)
    );
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
    let end = HObject {
        max: true,
        ..begin.clone()
    };
    let block = MOsdBackoff {
        pgid: op.pgid,
        map_epoch: map.epoch,
        op: backoff_op::BLOCK,
        id: 42,
        begin: begin.clone(),
        end: end.clone(),
    };
    c.sink.message(block.to_raw(features::ALL));
    MockTransport::settle().await;
    let acks = t.sent(CEPH_MSG_OSD_BACKOFF);
    assert_eq!(acks.len(), 1);
    let ack = MOsdBackoff::from_raw(acks[0].1.clone()).unwrap();
    assert_eq!(
        (ack.op, ack.id, ack.map_epoch),
        (backoff_op::ACK_BLOCK, 42, map.epoch)
    );
    assert_eq!(acks[0].1.priority, 63);
    // a new op to the same object is held
    let second = o.submit(write_req(&map, "bo"));
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 1);
    // unblock: every op of the session in the range goes out again, in tid order (the OSD
    // drops ops that hit a backoff, so the first one is resent too)
    let unblock = MOsdBackoff {
        op: backoff_op::UNBLOCK,
        ..block
    };
    // A fencing barrier raised during backoff must also hold the unblock resend.
    o.set_epoch_barrier(map.epoch + 1);
    let _ = o.status().await;
    c.sink.message(unblock.to_raw(features::ALL));
    MockTransport::settle().await;
    assert_eq!(t.op_count(), 1);
    o.install_map(next_epoch(&map));
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
    let names: Vec<String> = (0..200)
        .map(|i| format!("r{i}"))
        .filter(|n| primary_of(&map, n) == p)
        .take(3)
        .collect();
    let futs: Vec<_> = names.iter().map(|n| o.submit(write_req(&map, n))).collect();
    t.op_n(3).await;
    let c = t.live_conn(p).unwrap();
    c.sink.reset();
    let (_, _) = t.op_n(6).await;
    assert_eq!(t.conn_count(p), 2, "a new connection");
    let c2 = t.live_conn(p).unwrap();
    let resent: Vec<MOsdOp> = c2
        .sent
        .lock()
        .unwrap()
        .iter()
        .map(|m| MOsdOp::decode_raw(m).unwrap())
        .collect();
    let tids: Vec<u64> = resent.iter().map(|m| m.tid).collect();
    let mut sorted = tids.clone();
    sorted.sort();
    assert_eq!(tids, sorted);
    assert!(
        resent
            .iter()
            .all(|m| m.retry_attempt == 1 && m.flags & flags::RETRY != 0)
    );
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
    // A pool that never existed requires a confirmation after submission.
    o.install_map(map.clone());
    let mut op = ObjectOperation::new();
    op.stat();
    let pending = o.submit(OpRequest::new(ObjectLocator::new(4242), "x", op.clone()));
    drain(&mut ev).await;
    o.set_latest_map_version(map.epoch);
    let r = pending.await.unwrap();
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
    let r = o
        .operate(OpRequest::new(ObjectLocator::new(ec), "y", op))
        .await;
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
    let mut m = MOsdMap {
        fsid,
        newest_map: 60,
        ..Default::default()
    };
    m.maps.insert(first, full);
    o.handle_osd_map(m);
    let mut inc = MOsdMap {
        fsid,
        newest_map: 60,
        ..Default::default()
    };
    for e in first + 1..=60 {
        inc.incremental_maps
            .insert(e, read(&testdata().join(format!("privmon/inc.{e}"))));
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
    let blocklisted: Vec<_> = evs
        .iter()
        .filter_map(|e| {
            if let ObjecterEvent::Blocklisted { addrs } = e {
                Some(addrs.clone())
            } else {
                None
            }
        })
        .flatten()
        .collect();
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
    let (o, _ev) = start(
        &t,
        ObjecterConfig {
            client_inc: 17,
            ..Default::default()
        },
    );
    let map = base_map();
    o.install_map(map.clone());
    let pool = map.pool_id("rep").unwrap();
    let mut op = ObjectOperation::new();
    op.remove();
    let req = OpRequest::new(ObjectLocator::with_namespace(pool, "ns1"), "snappy", op).with_snapc(
        osdc::SnapContext {
            seq: 9,
            snaps: vec![9, 4],
        },
    );
    let fut = o.submit(req);
    let (c, m) = t.op_n(1).await;
    assert_eq!(&m.hobj.nspace[..], b"ns1");
    assert_eq!((m.snap_seq, m.snaps.clone()), (9, vec![9, 4]));
    assert_eq!(m.client_inc, 17);
    assert_eq!(
        (m.reqid.inc, m.reqid.tid, m.reqid.name.ty.0),
        (17, 0, 0),
        "reqid carries client_inc"
    );
    let pl = map.map_object(pool, b"snappy", b"ns1").unwrap();
    assert_eq!(c.osd, pl.sets.acting_primary);
    let st = o.status().await;
    assert_eq!((st.inflight, st.sessions, st.epoch), (1, 1, map.epoch));
    reply_to(&c, &m, 0, vec![]);
    fut.await.unwrap().unwrap();
    let mut by_osd = BTreeMap::new();
    for i in 0..60 {
        *by_osd
            .entry(primary_of(&map, &format!("spread{i}")))
            .or_insert(0) += 1;
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
    assert_eq!(
        op2.pgid.pgid.seed,
        op1.pgid.pgid.seed + 32,
        "resent to the child pg"
    );
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
