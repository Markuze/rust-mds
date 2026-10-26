//! Deterministic data OSD transport. Requests still traverse the real Objecter.
use denc::{BufMut, Bytes, BytesMut, Encode, EntityAddrVec, RawMessage, UTime};
use osdc::{
    Objecter, OsdMap,
    messages::{CEPH_MSG_OSD_OP, MOsdOp, MOsdOpReply},
    objecter::{ConnSink, ObjecterConfig, OsdConnection, OsdTransport},
    ops::{flags, op},
    types::EVersion,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

pub type ObjectKey = (i64, Bytes, String);
#[derive(Clone)]
pub struct DataObject {
    pub bytes: Vec<u8>,
    pub mtime: UTime,
    pub truncate_seq: u32,
}
#[derive(Default)]
pub struct DataState {
    pub objects: BTreeMap<ObjectKey, DataObject>,
    pub trace: Vec<(ObjectKey, u16, u32, u64)>,
    /// One absolute request number, counted from one. Fail before its mutation.
    pub fail_at: Option<usize>,
}
#[derive(Clone, Default)]
pub struct DataIo(pub Arc<Mutex<DataState>>);
impl DataIo {
    pub async fn objecter(&self) -> (Objecter, i64) {
        let map = (1..200)
            .find_map(|epoch| {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("../osdc/testdata/privmon/full.{epoch}"));
                let bytes = std::fs::read(path).ok()?;
                let map = OsdMap::decode(&Bytes::from(bytes)).unwrap();
                (map.pool_id("rep").is_some()
                    && (0..6).all(|osd| map.is_up(osd) && map.is_in(osd))
                    && map.pg_temp.is_empty()
                    && map.osd_primary_affinity.is_none())
                .then_some(map)
            })
            .expect("checked osdc privmon placement fixture");
        let pool = map.pool_id("rep").unwrap();
        let epoch = map.epoch;
        let (objecter, _) = Objecter::start(
            Arc::new(self.clone()),
            ObjecterConfig {
                op_timeout: Some(Duration::from_secs(1)),
                ..Default::default()
            },
        );
        objecter.install_map(map);
        objecter
            .wait_for_map_timeout(epoch, Duration::from_secs(1))
            .await
            .unwrap();
        (objecter, pool)
    }
    pub fn put(
        &self,
        pool: i64,
        namespace: &str,
        ino: u64,
        object: u64,
        bytes: Vec<u8>,
        mtime: UTime,
    ) {
        self.0.lock().unwrap().objects.insert(
            (
                pool,
                Bytes::copy_from_slice(namespace.as_bytes()),
                crate::rados::object_name(ino, object),
            ),
            DataObject {
                bytes,
                mtime,
                truncate_seq: 0,
            },
        );
    }
    pub fn fail_after(&self, successes: usize) {
        let mut state = self.0.lock().unwrap();
        state.fail_at = Some(state.trace.len() + successes + 1);
    }
}
struct Connection {
    data: DataIo,
    sink: ConnSink,
}
impl OsdTransport for DataIo {
    fn connect(&self, _: i32, _: &EntityAddrVec, sink: ConnSink) -> Arc<dyn OsdConnection> {
        sink.connected();
        Arc::new(Connection {
            data: self.clone(),
            sink,
        })
    }
}
impl OsdConnection for Connection {
    fn features(&self) -> u64 {
        denc::features::ALL
    }
    fn mark_down(&self) {}
    fn send(&self, raw: RawMessage) -> bool {
        if raw.msg_type != CEPH_MSG_OSD_OP {
            return true;
        }
        let request = MOsdOp::decode_raw(&raw).unwrap();
        assert_eq!(request.ops.len(), 1);
        let mut ops = request.ops.clone();
        let item = &mut ops[0];
        let key = (
            request.hobj.pool,
            request.hobj.nspace.clone(),
            String::from_utf8(request.hobj.oid.to_vec()).unwrap(),
        );
        let mut state = self.data.0.lock().unwrap();
        state.trace.push((
            key.clone(),
            item.op,
            item.truncate_seq(),
            item.truncate_size(),
        ));
        let result = if state.fail_at == Some(state.trace.len()) {
            state.fail_at = None;
            -5
        } else {
            match item.op {
                op::DELETE => {
                    if state.objects.remove(&key).is_some() {
                        0
                    } else {
                        -2
                    }
                }
                op::STAT => {
                    if let Some(object) = state.objects.get(&key) {
                        let mut out = BytesMut::new();
                        out.put_u64_le(object.bytes.len() as u64);
                        object.mtime.encode(&mut out, 0);
                        item.outdata = out.freeze();
                        0
                    } else {
                        -2
                    }
                }
                op::TRIMTRUNC => {
                    if let Some(object) = state.objects.get_mut(&key) {
                        if item.truncate_seq() >= object.truncate_seq {
                            object.truncate_seq = item.truncate_seq();
                            object.bytes.truncate(item.truncate_size() as usize);
                        }
                        0
                    } else {
                        -2
                    }
                }
                other => panic!("unexpected lifecycle OSD op {other}"),
            }
        };
        item.indata = Bytes::new();
        item.rval = result;
        drop(state);
        self.sink.message(
            MOsdOpReply {
                tid: request.tid,
                oid: request.hobj.oid.clone(),
                pgid: request.pgid.pgid,
                flags: i64::from(
                    request.flags & !(flags::ACK | flags::ONDISK | flags::ONNVRAM) | flags::ONDISK,
                ),
                result,
                bad_replay_version: EVersion::default(),
                osdmap_epoch: request.osdmap_epoch,
                ops,
                retry_attempt: request.retry_attempt,
                replay_version: EVersion::default(),
                user_version: 1,
                redirect: None,
            }
            .to_raw(),
        );
        true
    }
}
