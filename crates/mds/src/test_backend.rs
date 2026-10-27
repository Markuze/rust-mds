//! Offline compound-operation oracle. Each write commits one private object image
//! after every comparison succeeds. It supplies no cross-object transaction.

pub mod data;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode};
use osdc::{
    ObjectOperation, OpReply,
    objecter::OpResult,
    ops::{self, op},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use store::{Backend, PoolListing, StoreError};

#[derive(Clone, Default)]
pub struct Object {
    pub data: Bytes,
    pub attrs: BTreeMap<String, Bytes>,
    pub values: BTreeMap<Bytes, Bytes>,
}
#[derive(Default)]
pub struct Memory {
    pub objects: BTreeMap<String, Object>,
    pub writes: usize,
    pub appends: usize,
    pub lose_append_reply: bool,
    pub fail_write: Option<String>,
    pub trace: Vec<String>,
}
#[derive(Clone, Default)]
pub struct MemoryBackend {
    pub memory: Arc<Mutex<Memory>>,
    pub stall_append: Arc<AtomicBool>,
    pub append_started: Arc<tokio::sync::Notify>,
    pub release_append: Arc<tokio::sync::Notify>,
    /// (object, mutation, after-operation). One selected native await pauses.
    pub pause_operation: Arc<Mutex<Option<(String, bool, bool)>>>,
    pub operation_paused: Arc<tokio::sync::Notify>,
    pub release_operation: Arc<tokio::sync::Notify>,
}

impl Backend for MemoryBackend {
    async fn operate(&self, name: &str, operation: ObjectOperation) -> Result<OpReply, StoreError> {
        let pause = {
            let mut selected = self.pause_operation.lock().unwrap();
            if selected
                .as_ref()
                .is_some_and(|(object, write, _)| object == name && *write == operation.has_write())
            {
                selected.take().map(|(_, _, after)| after)
            } else {
                None
            }
        };
        if pause == Some(false) {
            self.operation_paused.notify_one();
            self.release_operation.notified().await;
        }
        let append = name == "rmds.wal.0.0"
            && operation.ops.iter().any(|o| o.op == op::OMAPSETVALS)
            && !operation.ops.iter().any(|o| o.op == op::CREATE);
        if append && self.stall_append.load(Ordering::Relaxed) {
            self.append_started.notify_one();
            self.release_append.notified().await;
        }
        let result = self.operate_atomic(name, operation, append);
        if pause == Some(true) {
            self.operation_paused.notify_one();
            self.release_operation.notified().await;
        }
        result
    }
}
impl MemoryBackend {
    fn operate_atomic(
        &self,
        name: &str,
        operation: ObjectOperation,
        append: bool,
    ) -> Result<OpReply, StoreError> {
        let mut memory = self.memory.lock().unwrap();
        memory.trace.push(format!(
            "{}:{name}",
            if operation.has_write() {
                "write"
            } else {
                "read"
            }
        ));
        if operation.has_write() && memory.fail_write.as_deref() == Some(name) {
            memory.fail_write = None;
            return Err(StoreError::Io);
        }
        let writes = operation.has_write();
        let exists = memory.objects.contains_key(name);
        let mut object = memory.objects.get(name).cloned().unwrap_or_default();
        let mut reply = OpReply::default();
        for item in operation.ops {
            let mut out = Bytes::new();
            match item.op {
                op::CREATE => {
                    if exists && item.flags & ops::op_flags::EXCL != 0 {
                        return Err(StoreError::AlreadyExists);
                    }
                }
                op::GETXATTR | op::SETXATTR | op::CMPXATTR => {
                    let n = item.xattr_name_len() as usize;
                    let key = std::str::from_utf8(&item.indata[..n]).unwrap();
                    let value = item.indata.slice(n..);
                    match item.op {
                        op::GETXATTR => {
                            out = object
                                .attrs
                                .get(key)
                                .cloned()
                                .ok_or(StoreError::MissingState)?
                        }
                        op::CMPXATTR => {
                            assert_eq!(item.args[8], ops::cmp_op::EQ);
                            if object.attrs.get(key) != Some(&value) {
                                return Err(StoreError::GuardRejected);
                            }
                        }
                        _ => {
                            object.attrs.insert(key.into(), value);
                        }
                    }
                }
                op::GETXATTRS => {
                    if !exists {
                        return Err(StoreError::MissingState);
                    }
                    let mut data = BytesMut::new();
                    object.attrs.encode(&mut data, 0);
                    out = data.freeze();
                }
                op::WRITEFULL => object.data = item.indata,
                op::READ => {
                    if !exists {
                        return Err(StoreError::MissingState);
                    }
                    out = object
                        .data
                        .slice(..object.data.len().min(item.extent_length() as usize));
                }
                op::OMAPSETVALS => object
                    .values
                    .extend(BTreeMap::<Bytes, Bytes>::decode(&mut item.indata.clone()).unwrap()),
                op::OMAPRMKEYS => {
                    for key in BTreeSet::<Bytes>::decode(&mut item.indata.clone()).unwrap() {
                        object.values.remove(&key);
                    }
                }
                op::OMAPGETVALSBYKEYS => {
                    let keys = BTreeSet::<Bytes>::decode(&mut item.indata.clone()).unwrap();
                    let values = keys
                        .into_iter()
                        .filter_map(|key| {
                            object.values.get(&key).cloned().map(|value| (key, value))
                        })
                        .collect();
                    out = ops::encode_omap(&values);
                }
                op::OMAPGETVALS => {
                    let mut data = item.indata;
                    let after = Bytes::decode(&mut data).unwrap();
                    let max = data.get_u64_le() as usize;
                    let prefix = Bytes::decode(&mut data).unwrap();
                    let matching: Vec<_> = object
                        .values
                        .iter()
                        .filter(|(key, _)| **key > after && key.starts_with(&prefix))
                        .collect();
                    // Deliberately return native short pages to exercise integration.
                    let values: BTreeMap<_, _> = matching
                        .iter()
                        .take(max.min(2))
                        .map(|(key, value)| ((*key).clone(), (*value).clone()))
                        .collect();
                    let mut bytes = BytesMut::from(ops::encode_omap(&values).as_ref());
                    bytes.put_u8(u8::from(values.len() < matching.len()));
                    out = bytes.freeze();
                }
                other => panic!("unsupported compound op {other}"),
            }
            reply.ops.push(OpResult {
                rval: 0,
                outdata: out,
            });
        }
        if writes {
            memory.objects.insert(name.into(), object);
            memory.writes += 1;
            if append {
                memory.appends += 1;
                if memory.lose_append_reply {
                    memory.lose_append_reply = false;
                    return Err(StoreError::UnknownOutcome);
                }
            }
        }
        Ok(reply)
    }
}
impl PoolListing for MemoryBackend {
    type Cursor = usize;
    async fn list_pool_page(
        &self,
        cursor: Option<usize>,
        max_entries: u32,
        _: usize,
    ) -> Result<store::PoolPage<usize>, osdc::PoolListError> {
        let memory = self.memory.lock().unwrap();
        let names: Vec<_> = memory.objects.keys().collect();
        let start = cursor.unwrap_or(0);
        let end = (start + (max_entries as usize).min(2)).min(names.len());
        let objects = names[start..end]
            .iter()
            .map(|name| osdc::PoolObject {
                namespace: vec![],
                name: name.as_bytes().to_vec(),
                locator: vec![],
            })
            .collect();
        Ok(store::PoolPage {
            objects,
            next: (end < names.len()).then_some(end),
            osdmap_epoch: 1,
        })
    }
}

#[derive(Clone)]
pub struct Fence {
    pub backend: MemoryBackend,
    pub fail: bool,
    blocked: Arc<Mutex<Vec<denc::EntityAddr>>>,
}
impl Fence {
    pub fn new(backend: MemoryBackend) -> Self {
        Self {
            backend,
            fail: false,
            blocked: Default::default(),
        }
    }
}
impl crate::fencing::FenceBackend for Fence {
    fn raise_barrier(&self, epoch: u32) {
        self.backend
            .memory
            .lock()
            .unwrap()
            .trace
            .push(format!("barrier:{epoch}"));
    }
    fn is_blocklisted(&self, addr: &denc::EntityAddr) -> bool {
        self.blocked.lock().unwrap().contains(addr)
    }
    async fn blocklist(&self, addr: &denc::EntityAddr) -> Result<(), String> {
        self.backend
            .memory
            .lock()
            .unwrap()
            .trace
            .push(format!("blocklist:{}", addr.nonce));
        if self.fail {
            return Err("injected blocklist rejection".into());
        }
        self.blocked.lock().unwrap().push(*addr);
        Ok(())
    }
    async fn latest_epoch(&self) -> Result<u32, String> {
        self.backend
            .memory
            .lock()
            .unwrap()
            .trace
            .push("latest".into());
        Ok(9)
    }
    async fn wait_epoch(&self, epoch: u32) -> Result<u32, String> {
        self.backend
            .memory
            .lock()
            .unwrap()
            .trace
            .push(format!("map:{epoch}"));
        Ok(epoch)
    }
}
