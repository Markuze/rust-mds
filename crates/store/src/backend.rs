//! Single-object atomic operations. The native implementation uses Objecter,
//! not a shell or librados. See Ceph `osd/PrimaryLogPG.cc`, `do_osd_ops`.

use std::future::Future;

use bytes::Bytes;
use osdc::{ObjectLocator, ObjectOperation, Objecter, OpError, OpReply, OpRequest};

use crate::stable::StoreError;

/// The test boundary is the actual compound OSD operation, not a synthetic WAL.
/// A deadline after possible write submission is UnknownOutcome, not rejection.
/// Dropping the returned future does not prove cancellation of the backend op.
pub trait Backend {
    fn operate(
        &self,
        object: &str,
        operation: ObjectOperation,
    ) -> impl Future<Output = Result<OpReply, StoreError>> + Send;
}

/// Native namespace-aware pool listing is a separate capability. An object
/// backend without listing cannot bootstrap a filesystem.
pub trait PoolListing {
    /// The store only compares and passes this opaque continuation. Production
    /// uses the native cursor unchanged; fault tests use script positions.
    type Cursor: Clone + PartialEq + Send + Sync;
    fn list_pool_page(
        &self,
        cursor: Option<Self::Cursor>,
        max_entries: u32,
        max_bytes: usize,
    ) -> impl Future<Output = Result<PoolPage<Self::Cursor>, osdc::PoolListError>> + Send;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolPage<C> {
    pub objects: Vec<osdc::PoolObject>,
    pub next: Option<C>,
    pub osdmap_epoch: u32,
}

pub(crate) async fn checked<B: Backend>(
    backend: &B,
    object: &str,
    operation: ObjectOperation,
) -> Result<OpReply, StoreError> {
    let count = operation.ops.len();
    let reply = backend.operate(object, operation).await?;
    let negative = if reply.result < 0 {
        Some(reply.result)
    } else {
        reply.ops.iter().find(|o| o.rval < 0).map(|o| o.rval)
    };
    if let Some(errno) = negative {
        return Err(match errno {
            -125 => StoreError::GuardRejected,
            -17 => StoreError::AlreadyExists,
            -2 | -61 => StoreError::MissingState,
            _ => StoreError::Io,
        });
    }
    if reply.ops.len() != count {
        return Err(StoreError::Corrupt);
    }
    Ok(reply)
}

/// A native Objecter bound to one filesystem metadata pool.
///
/// The caller must start Objecter with a positive finite `op_timeout`. An outer
/// deadline alone is insufficient: Objecter retains/resends requests after their
/// response receiver is dropped. This backend does not configure the actor.
pub struct RadosBackend {
    objecter: Objecter,
    pool: i64,
}

impl RadosBackend {
    /// Bind an existing Objecter. This does not install an operation deadline.
    /// The daemon must bound actor timeout, total attempt time, retry count and
    /// retained requests. It must classify caller-cancelled writes as uncertain.
    pub fn new(objecter: Objecter, pool: i64) -> Self {
        Self { objecter, pool }
    }
}

impl Backend for RadosBackend {
    async fn operate(
        &self,
        object: &str,
        operation: ObjectOperation,
    ) -> Result<OpReply, StoreError> {
        let count = operation.ops.len();
        let reply = self
            .objecter
            .operate_checked(OpRequest::new(
                ObjectLocator::new(self.pool),
                Bytes::copy_from_slice(object.as_bytes()),
                operation,
            ))
            .await
            .map_err(|error| match error {
                OpError::Blocklisted => StoreError::Fenced,
                OpError::TimedOut | OpError::Shutdown => StoreError::UnknownOutcome,
                OpError::Rados(-125) => StoreError::GuardRejected,
                OpError::Rados(-17) => StoreError::AlreadyExists,
                OpError::Rados(-2) | OpError::Rados(-61) => StoreError::MissingState,
                _ => StoreError::Io,
            })?;
        if reply.ops.len() != count {
            return Err(StoreError::Corrupt);
        }
        Ok(reply)
    }
}

impl PoolListing for RadosBackend {
    type Cursor = osdc::PoolListCursor;
    async fn list_pool_page(
        &self,
        cursor: Option<osdc::PoolListCursor>,
        max_entries: u32,
        max_bytes: usize,
    ) -> Result<PoolPage<Self::Cursor>, osdc::PoolListError> {
        let page = self
            .objecter
            .list_pool_page(
                self.pool,
                osdc::NamespaceScope::All,
                cursor,
                max_entries,
                max_bytes,
            )
            .await?;
        Ok(PoolPage {
            objects: page.objects,
            next: page.next,
            osdmap_epoch: page.osdmap_epoch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        pin::pin,
        task::{Context, Poll, Waker},
    };
    struct Replies(OpReply);
    impl Backend for Replies {
        async fn operate(&self, _: &str, _: ObjectOperation) -> Result<OpReply, StoreError> {
            Ok(self.0.clone())
        }
    }
    fn run<T>(future: impl Future<Output = T>) -> T {
        let mut context = Context::from_waker(Waker::noop());
        match pin!(future).as_mut().poll(&mut context) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("unexpected pending reply"),
        }
    }
    #[test]
    fn compound_results_require_overall_per_op_and_complete_result_vector() {
        let mut op = ObjectOperation::new();
        op.getxattr("rmds.owner");
        let overall = Replies(OpReply {
            result: -5,
            ..Default::default()
        });
        assert_eq!(
            run(checked(&overall, "wal", op.clone())),
            Err(StoreError::Io)
        );
        let subop = Replies(OpReply {
            ops: vec![osdc::objecter::OpResult {
                rval: -125,
                outdata: Bytes::new(),
            }],
            ..Default::default()
        });
        assert_eq!(
            run(checked(&subop, "wal", op.clone())),
            Err(StoreError::GuardRejected)
        );
        let missing = Replies(OpReply::default());
        assert_eq!(run(checked(&missing, "wal", op)), Err(StoreError::Corrupt));
    }
}
