//! Stable stream-0 persistence. Same-object CMPXATTR guards follow Ceph
//! `osd/PrimaryLogPG.cc`, `do_osd_ops`. All numeric equality guards use canonical
//! bytes; no lexicographic inequality implements a numeric comparison.

use std::collections::{BTreeMap, BTreeSet};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use denc::{Decode, Encode, EntityAddrVec};
use osdc::{ObjectOperation, OpReply, ops::cmp_op};

use crate::{
    backend::Backend,
    framing::{self, FrameKind},
};

const OBJECT: &str = "rmds.wal.0.0";
const IDENTITY: &str = "rmds.identity";
const OWNER: &str = "rmds.owner";
const TAIL: &str = "rmds.tail";
const BATCH: &str = "rmds.batch";
const CHECKPOINT: &str = "rmds.checkpoint";
const TRIMMED: &str = "rmds.trimmed";
const AUTHORITY: &str = "rmds.authority";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreError {
    Io,
    UnknownOutcome,
    Fenced,
    Corrupt,
    WrongFormat,
    UnsupportedVersion,
    MissingState,
    Capacity,
    GuardRejected,
    AlreadyExists,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StoreError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreIdentity {
    pub cluster_fsid: [u8; 16],
    pub filesystem_id: u64,
    pub metadata_pool: i64,
}

impl StoreIdentity {
    pub(crate) fn encode(&self) -> Bytes {
        let mut b = BytesMut::with_capacity(32);
        b.extend_from_slice(&self.cluster_fsid);
        b.put_u64_le(self.filesystem_id);
        b.put_i64_le(self.metadata_pool);
        b.freeze()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerToken {
    pub shard_epoch: u64,
    pub rank: u64,
    pub incarnation: u64,
    pub gid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankOwner {
    pub rank: u64,
    pub incarnation: u64,
    pub gid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriterIdentity {
    pub rank_owner: RankOwner,
    pub addresses: EntityAddrVec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FencedWriter {
    pub writer: WriterIdentity,
    pub verified_osd_epoch: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Authority {
    pub(crate) revision: u64,
    pub(crate) writer: WriterIdentity,
    pub(crate) predecessor: Option<FencedWriter>,
}

#[derive(Debug, Clone)]
pub struct ControlInspection {
    pub revision: u64,
    pub owner: OwnerToken,
    pub writer: WriterIdentity,
    pub predecessor: Option<FencedWriter>,
    pub tail: u64,
    pub checkpoint: Option<CheckpointDescriptor>,
}

#[derive(Debug, Clone)]
pub struct OpenedShard {
    pub owner: OwnerToken,
    pub tail: u64,
    pub checkpoint: Option<CheckpointDescriptor>,
    frozen: FrozenShard,
}

impl OpenedShard {
    pub(crate) fn from_frozen(frozen: FrozenShard) -> Self {
        Self {
            owner: frozen.owner,
            tail: frozen.tail,
            checkpoint: frozen.checkpoint.clone(),
            frozen,
        }
    }
}

impl OwnerToken {
    pub(crate) fn encode(self) -> Bytes {
        let mut b = BytesMut::with_capacity(32);
        for n in [self.shard_epoch, self.rank, self.incarnation, self.gid] {
            b.put_u64_le(n);
        }
        b.freeze()
    }

    pub(crate) fn decode(bytes: &Bytes) -> Result<Self, StoreError> {
        if bytes.len() != 32 {
            return Err(StoreError::Corrupt);
        }
        let mut b = bytes.clone();
        let token = Self {
            shard_epoch: b.get_u64_le(),
            rank: b.get_u64_le(),
            incarnation: b.get_u64_le(),
            gid: b.get_u64_le(),
        };
        if token.shard_epoch == 0 || token.rank != 0 || token.incarnation == 0 || token.gid == 0 {
            return Err(StoreError::Corrupt);
        }
        Ok(token)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointDescriptor {
    pub generation: u64,
    pub covered_seq: u64,
    pub manifest_name: String,
    pub manifest_checksum: u32,
}

impl CheckpointDescriptor {
    pub(crate) fn encode(&self) -> Result<Bytes, StoreError> {
        if self.manifest_name.len() > 256 {
            return Err(StoreError::Capacity);
        }
        let mut b = BytesMut::new();
        b.put_u8(1);
        b.put_u64_le(self.generation);
        b.put_u64_le(self.covered_seq);
        b.put_u32_le(self.manifest_checksum);
        b.put_u32_le(self.manifest_name.len() as u32);
        b.extend_from_slice(self.manifest_name.as_bytes());
        Ok(b.freeze())
    }

    pub(crate) fn decode(bytes: &Bytes) -> Result<Option<Self>, StoreError> {
        if bytes.is_empty() {
            return Ok(None);
        }
        if bytes.len() < 25 {
            return Err(StoreError::Corrupt);
        }
        let mut b = bytes.clone();
        if b.get_u8() != 1 {
            return Err(StoreError::UnsupportedVersion);
        }
        let generation = b.get_u64_le();
        let covered_seq = b.get_u64_le();
        let manifest_checksum = b.get_u32_le();
        let n = b.get_u32_le() as usize;
        if n > 256 || b.len() != n {
            return Err(StoreError::Corrupt);
        }
        let manifest_name = std::str::from_utf8(&b)
            .map_err(|_| StoreError::Corrupt)?
            .to_owned();
        Ok(Some(Self {
            generation,
            covered_seq,
            manifest_name,
            manifest_checksum,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub max_record_bytes: usize,
    pub max_batch_records: usize,
    pub max_batch_bytes: usize,
    pub max_wal_keys: u64,
    pub max_wal_bytes: usize,
    pub page_keys: usize,
    pub max_checkpoint_bytes: usize,
    pub max_checkpoint_generations: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_record_bytes: 1 << 20,
            max_batch_records: 64,
            max_batch_bytes: 4 << 20,
            max_wal_keys: 4096,
            max_wal_bytes: 64 << 20,
            page_keys: 16,
            max_checkpoint_bytes: 64 << 20,
            max_checkpoint_generations: 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BatchIdentity {
    id: [u8; 16],
    before: u64,
    tail: u64,
    checksum: u32,
}

impl BatchIdentity {
    fn encode(&self) -> Bytes {
        let mut b = BytesMut::with_capacity(36);
        b.extend_from_slice(&self.id);
        b.put_u64_le(self.before);
        b.put_u64_le(self.tail);
        b.put_u32_le(self.checksum);
        b.freeze()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FrozenShard {
    pub owner: OwnerToken,
    pub tail: u64,
    pub checkpoint: Option<CheckpointDescriptor>,
    last_batch: Bytes,
    trimmed: u64,
    authority: Authority,
}

#[derive(Clone, PartialEq, Eq)]
struct PendingBatch {
    owner: OwnerToken,
    identity: BatchIdentity,
    values: BTreeMap<Bytes, Bytes>,
}

/// One owner task holds this facade. `&mut self` and a retained pending batch
/// prevent a second append, publication or trim after an uncertain completion.
pub struct StableStore<B> {
    backend: B,
    identity: StoreIdentity,
    limits: Limits,
    pending: Option<PendingBatch>,
}

impl<B: Backend> StableStore<B> {
    pub fn new(backend: B, identity: StoreIdentity, limits: Limits) -> Result<Self, StoreError> {
        if limits.page_keys == 0
            || limits.max_batch_records == 0
            || limits.max_record_bytes < framing::OVERHEAD
            || limits.max_batch_bytes < limits.max_record_bytes
            || limits.max_wal_bytes < limits.max_record_bytes
        {
            return Err(StoreError::Capacity);
        }
        Ok(Self {
            backend,
            identity,
            limits,
            pending: None,
        })
    }

    async fn operate(&self, name: &str, op: ObjectOperation) -> Result<OpReply, StoreError> {
        crate::backend::checked(&self.backend, name, op).await
    }

    /// Read all mutable control values in one atomic read. Acquisition metadata
    /// is a separate integration layer; this function never grants ownership.
    pub(crate) async fn inspect_frozen(&self) -> Result<FrozenShard, StoreError> {
        let mut op = ObjectOperation::new();
        for name in [IDENTITY, OWNER, TAIL, BATCH, CHECKPOINT, TRIMMED, AUTHORITY] {
            op.getxattr(name);
        }
        let r = self.operate(OBJECT, op).await?;
        if r.out(0) != self.identity.encode() {
            return Err(StoreError::WrongFormat);
        }
        let owner = OwnerToken::decode(&r.out(1))?;
        let authority = Authority::decode(&r.out(6))?;
        if authority.writer.rank_owner != rank_owner(owner) {
            return Err(StoreError::Corrupt);
        }
        let tail = number(&r.out(2))?;
        let checkpoint = CheckpointDescriptor::decode(&r.out(4))?;
        let trimmed = number(&r.out(5))?;
        if trimmed > checkpoint.as_ref().map_or(0, |d| d.covered_seq) {
            return Err(StoreError::Corrupt);
        }
        if checkpoint.as_ref().is_some_and(|d| d.covered_seq > tail) {
            return Err(StoreError::Corrupt);
        }
        let last_batch = r.out(3);
        if (tail == 0 && !last_batch.is_empty()) || (tail != 0 && last_batch.len() != 36) {
            return Err(StoreError::Corrupt);
        }
        Ok(FrozenShard {
            owner,
            tail,
            checkpoint,
            last_batch,
            trimmed,
            authority,
        })
    }

    pub async fn inspect_control(&self) -> Result<ControlInspection, StoreError> {
        let c = self.inspect_frozen().await?;
        Ok(ControlInspection {
            revision: c.authority.revision,
            owner: c.owner,
            writer: c.authority.writer,
            predecessor: c.authority.predecessor,
            tail: c.tail,
            checkpoint: c.checkpoint,
        })
    }

    /// Acquire only this object's authority. The daemon must fence predecessors
    /// and acquire rank sessions separately before loading or admitting work.
    pub async fn open(
        &self,
        expected: &ControlInspection,
        new_owner: OwnerToken,
        writer: WriterIdentity,
        predecessor: FencedWriter,
    ) -> Result<OpenedShard, StoreError> {
        self.no_pending()?;
        if expected.owner.shard_epoch.checked_add(1) != Some(new_owner.shard_epoch)
            || new_owner.rank != 0
            || new_owner.gid == 0
            || new_owner.incarnation <= expected.owner.incarnation
            || new_owner.gid == expected.owner.gid
            || rank_owner(new_owner) != writer.rank_owner
            || writer.addresses.0.is_empty()
            || predecessor.writer != expected.writer
            || predecessor.verified_osd_epoch == 0
        {
            return Err(StoreError::Fenced);
        }
        let old = Authority {
            revision: expected.revision,
            writer: expected.writer.clone(),
            predecessor: expected.predecessor.clone(),
        };
        let new = Authority {
            revision: expected
                .revision
                .checked_add(1)
                .ok_or(StoreError::Capacity)?,
            writer,
            predecessor: Some(predecessor),
        };
        let actual = self.inspect_frozen().await?;
        if actual.owner == new_owner && actual.authority == new {
            return Ok(OpenedShard::from_frozen(actual));
        }
        if actual.owner != expected.owner || actual.authority != old {
            return Err(StoreError::Fenced);
        }
        let mut op = ObjectOperation::new();
        guards(&mut op, &self.identity, expected.owner);
        op.cmpxattr_str(AUTHORITY, cmp_op::EQ, &old.encode()?);
        op.setxattr(OWNER, new_owner.encode());
        op.setxattr(AUTHORITY, new.encode()?);
        match self.operate(OBJECT, op).await {
            Ok(_) | Err(StoreError::UnknownOutcome | StoreError::GuardRejected) => {
                let now = self.inspect_frozen().await?;
                if now.owner == new_owner && now.authority == new {
                    Ok(OpenedShard::from_frozen(now))
                } else if now.owner == expected.owner && now.authority == old {
                    Err(StoreError::UnknownOutcome)
                } else {
                    Err(StoreError::Fenced)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Production loader: a missing descriptor is missing required state. The
    /// engine's checked restore/replay boundary validates semantic references.
    pub async fn load(
        &self,
        opened: &OpenedShard,
    ) -> Result<
        (
            engine::durability::EngineCheckpoint,
            Vec<(u64, engine::JournalRecord)>,
        ),
        StoreError,
    > {
        self.no_pending()?;
        if opened.owner != opened.frozen.owner
            || opened.tail != opened.frozen.tail
            || opened.checkpoint != opened.frozen.checkpoint
        {
            return Err(StoreError::Corrupt);
        }
        let descriptor = opened.checkpoint.as_ref().ok_or(StoreError::MissingState)?;
        let state = self.load_checkpoint_bytes(descriptor).await?;
        let state =
            crate::record::decode_checkpoint(&state).map_err(crate::record::store_decode_error)?;
        if state.covered_seq != descriptor.covered_seq {
            return Err(StoreError::Corrupt);
        }
        let mut records = Vec::new();
        for (seq, bytes) in self.load_tail(&opened.frozen).await? {
            records.push((
                seq,
                crate::record::decode_record(&bytes).map_err(crate::record::store_decode_error)?,
            ));
        }
        Ok((state, records))
    }

    pub async fn write_checkpoint(
        &self,
        owner: OwnerToken,
        generation: u64,
        state: &engine::durability::EngineCheckpoint,
    ) -> Result<CheckpointDescriptor, StoreError>
    where
        B: crate::backend::PoolListing,
    {
        let bytes = crate::record::encode_checkpoint_bounded(
            state,
            self.limits
                .max_checkpoint_bytes
                .checked_sub(framing::OVERHEAD)
                .ok_or(StoreError::Capacity)?,
        )?;
        self.write_checkpoint_bytes(owner, generation, state.covered_seq, &bytes)
            .await
    }

    /// Append encoded transaction bodies. Validate each supported body schema
    /// before admission. The adapter uses the bounded record encoder.
    pub async fn append(
        &mut self,
        owner: OwnerToken,
        expected_tail: u64,
        batch_id: [u8; 16],
        records: &[Bytes],
    ) -> Result<u64, StoreError> {
        if records.is_empty() || records.len() > self.limits.max_batch_records {
            return Err(StoreError::Capacity);
        }
        let tail = expected_tail
            .checked_add(records.len() as u64)
            .ok_or(StoreError::Capacity)?;
        let mut values = BTreeMap::new();
        let mut size = 0usize;
        let mut checksum = 0;
        for (i, body) in records.iter().enumerate() {
            if body.len() > self.limits.max_record_bytes - framing::OVERHEAD {
                return Err(StoreError::Capacity);
            }
            crate::record::decode_record(body).map_err(crate::record::store_decode_error)?;
            let seq = expected_tail + i as u64 + 1;
            let frame = framing::encode(
                FrameKind::Record,
                &self.identity,
                owner,
                seq,
                batch_id,
                body,
                self.limits.max_record_bytes,
            )?;
            size = size.checked_add(frame.len()).ok_or(StoreError::Capacity)?;
            if size > self.limits.max_batch_bytes {
                return Err(StoreError::Capacity);
            }
            checksum = crc32c::crc32c_append(checksum, &frame[..frame.len() - 4]);
            values.insert(key(seq), frame);
        }
        let batch = PendingBatch {
            owner,
            identity: BatchIdentity {
                id: batch_id,
                before: expected_tail,
                tail,
                checksum,
            },
            values,
        };
        if self.pending.as_ref().is_some_and(|p| p != &batch) {
            return Err(StoreError::UnknownOutcome);
        }
        // Persist in memory before the first await. Cancellation also freezes admission.
        self.pending = Some(batch);
        self.reconcile_pending().await
    }

    pub async fn reconcile_pending(&mut self) -> Result<u64, StoreError> {
        let batch = self.pending.clone().ok_or(StoreError::MissingState)?;
        let control = self.inspect_frozen().await?;
        if control.owner != batch.owner {
            return Err(StoreError::Fenced);
        }
        if control.tail == batch.identity.tail && control.last_batch == batch.identity.encode() {
            self.pending = None;
            return Ok(control.tail);
        }
        if control.tail != batch.identity.before {
            return Err(StoreError::Corrupt);
        }
        let retained = batch
            .identity
            .tail
            .checked_sub(control.trimmed)
            .ok_or(StoreError::Corrupt)?;
        // Conservative bound: each retained value may occupy the maximum size.
        if retained > self.limits.max_wal_keys
            || retained > (self.limits.max_wal_bytes / self.limits.max_record_bytes) as u64
        {
            self.pending = None;
            return Err(StoreError::Capacity);
        }
        let mut op = ObjectOperation::new();
        guards(&mut op, &self.identity, batch.owner);
        op.cmpxattr_str(TAIL, cmp_op::EQ, &decimal(batch.identity.before));
        op.omap_set_vals(&batch.values);
        op.setxattr(TAIL, decimal(batch.identity.tail));
        op.setxattr(BATCH, batch.identity.encode());
        match self.operate(OBJECT, op).await {
            Ok(_) => {
                self.pending = None;
                Ok(batch.identity.tail)
            }
            Err(StoreError::GuardRejected) => {
                // A failed compare is not a commit. Inspect without recursive retry.
                let now = self.inspect_frozen().await?;
                if now.owner != batch.owner {
                    return Err(StoreError::Fenced);
                }
                if now.tail == batch.identity.tail && now.last_batch == batch.identity.encode() {
                    self.pending = None;
                    Ok(now.tail)
                } else if now.tail == batch.identity.before {
                    Err(StoreError::GuardRejected)
                } else {
                    Err(StoreError::Corrupt)
                }
            }
            Err(error) => Err(error),
        }
    }

    /// Read only the required uncovered keys. Every page must be contiguous.
    /// A missing final key is damage, not a successful shortened replay.
    pub(crate) async fn load_tail(
        &self,
        opened: &FrozenShard,
    ) -> Result<Vec<(u64, Bytes)>, StoreError> {
        let current = self.inspect_frozen().await?;
        if current.owner != opened.owner {
            return Err(StoreError::Fenced);
        }
        if current.tail != opened.tail || current.checkpoint != opened.checkpoint {
            return Err(StoreError::GuardRejected);
        }
        let mut next = opened.checkpoint.as_ref().map_or(0, |d| d.covered_seq);
        let count = opened.tail.checked_sub(next).ok_or(StoreError::Corrupt)?;
        if count > self.limits.max_wal_keys {
            return Err(StoreError::Capacity);
        }
        let mut result = Vec::new();
        let mut total = 0usize;
        while next < opened.tail {
            let n = (opened.tail - next).min(self.limits.page_keys as u64);
            let mut op = ObjectOperation::new();
            guards(&mut op, &self.identity, opened.owner);
            op.cmpxattr_str(TAIL, cmp_op::EQ, &decimal(opened.tail));
            op.cmpxattr_str(
                CHECKPOINT,
                cmp_op::EQ,
                &checkpoint_bytes(opened.checkpoint.as_ref())?,
            );
            op.omap_get_vals(&key(next), b"r/", n);
            let r = self.operate(OBJECT, op).await?;
            let (values, more) = decode_page(&r.out(4), n as usize, self.limits.max_record_bytes)?;
            if values.is_empty() {
                return Err(StoreError::Corrupt);
            }
            for (k, v) in values {
                next += 1;
                if k != key(next) {
                    return Err(StoreError::Corrupt);
                }
                total = total.checked_add(v.len()).ok_or(StoreError::Capacity)?;
                if total > self.limits.max_wal_bytes {
                    return Err(StoreError::Capacity);
                }
                let frame = framing::decode(
                    &v,
                    FrameKind::Record,
                    &self.identity,
                    next,
                    self.limits.max_record_bytes,
                )?;
                if frame.owner.shard_epoch > opened.owner.shard_epoch
                    || frame.owner.rank != opened.owner.rank
                {
                    return Err(StoreError::Corrupt);
                }
                result.push((next, frame.body));
            }
            // The OSD count is an upper bound. Byte/count caps may shorten a
            // page, but continuation must agree with this stream's frozen tail.
            if more != (next < opened.tail) {
                return Err(StoreError::Corrupt);
            }
        }
        Ok(result)
    }

    /// Write a materialized checkpoint payload, not WAL history. Engine schema
    /// encoding is supplied by the checkpoint codec at the integration boundary.
    pub(crate) async fn write_checkpoint_bytes(
        &self,
        owner: OwnerToken,
        generation: u64,
        covered_seq: u64,
        state: &Bytes,
    ) -> Result<CheckpointDescriptor, StoreError>
    where
        B: crate::backend::PoolListing,
    {
        self.no_pending()?;
        let current = self.inspect_frozen().await?;
        if current.owner != owner {
            return Err(StoreError::Fenced);
        }
        if covered_seq > current.tail {
            return Err(StoreError::GuardRejected);
        }
        if generation == 0 || generation > self.limits.max_checkpoint_generations {
            return Err(StoreError::Capacity);
        }
        let name = format!(
            "rmds.checkpoint.0.{}.{}.state",
            owner.shard_epoch, generation
        );
        let manifest_name = format!(
            "rmds.checkpoint.0.{}.{}.manifest",
            owner.shard_epoch, generation
        );
        self.check_checkpoint_backlog([&name, &manifest_name])
            .await?;
        let transaction = [0; 16];
        let frame = framing::encode(
            FrameKind::Checkpoint,
            &self.identity,
            owner,
            covered_seq,
            transaction,
            state,
            self.limits.max_checkpoint_bytes,
        )?;
        self.write_immutable(&name, frame.clone()).await?;
        let mut manifest = BytesMut::new();
        manifest.put_u64_le(generation);
        manifest.put_u32_le(name.len() as u32);
        manifest.extend_from_slice(name.as_bytes());
        manifest.put_u64_le(frame.len() as u64);
        manifest.put_u32_le(framing::content_checksum(&frame)?);
        let manifest = framing::encode(
            FrameKind::Manifest,
            &self.identity,
            owner,
            covered_seq,
            transaction,
            &manifest,
            4096,
        )?;
        self.write_immutable(&manifest_name, manifest.clone())
            .await?;
        Ok(CheckpointDescriptor {
            generation,
            covered_seq,
            manifest_name,
            manifest_checksum: framing::content_checksum(&manifest)?,
        })
    }

    async fn check_checkpoint_backlog(&self, planned: [&str; 2]) -> Result<(), StoreError>
    where
        B: crate::backend::PoolListing,
    {
        use std::time::{Duration, Instant};
        let mut cursor = None;
        let mut count = 0u64;
        let mut existing = BTreeSet::new();
        let mut restarts = 0;
        let started = Instant::now();
        for _ in 0..4096 {
            if started.elapsed() > Duration::from_secs(60) {
                return Err(StoreError::Capacity);
            }
            let page = match self.backend.list_pool_page(cursor.clone(), 64, 65536).await {
                Ok(page) => page,
                Err(osdc::PoolListError::MapChanged) if restarts < 4 => {
                    restarts += 1;
                    cursor = None;
                    count = 0;
                    existing.clear();
                    continue;
                }
                Err(osdc::PoolListError::Unsupported) => {
                    return Err(StoreError::UnsupportedVersion);
                }
                Err(_) => return Err(StoreError::Io),
            };
            if page.objects.len() > 64
                || page
                    .objects
                    .iter()
                    .map(|o| o.name.len() + o.namespace.len() + o.locator.len())
                    .sum::<usize>()
                    > 65536
            {
                return Err(StoreError::Capacity);
            }
            for object in page.objects {
                if object.namespace.is_empty() && object.name.starts_with(b"rmds.checkpoint.0.") {
                    count = count.checked_add(1).ok_or(StoreError::Capacity)?;
                    if planned.iter().any(|name| name.as_bytes() == object.name) {
                        existing.insert(object.name);
                    }
                }
            }
            if count > self.limits.max_checkpoint_generations.saturating_mul(2) {
                return Err(StoreError::Capacity);
            }
            if page.next.is_some() && page.next == cursor {
                return Err(StoreError::Corrupt);
            }
            cursor = page.next;
            if cursor.is_none() {
                if count.saturating_add(2 - existing.len() as u64)
                    > self.limits.max_checkpoint_generations.saturating_mul(2)
                {
                    return Err(StoreError::Capacity);
                }
                return Ok(());
            }
        }
        Err(StoreError::Capacity)
    }

    async fn write_immutable(&self, name: &str, bytes: Bytes) -> Result<(), StoreError> {
        let mut op = ObjectOperation::new();
        op.create(true).write_full(bytes.clone());
        match self.operate(name, op).await {
            Ok(_) => Ok(()),
            Err(StoreError::AlreadyExists | StoreError::UnknownOutcome) => {
                let existing = self.read_object(name, bytes.len()).await?;
                if existing == bytes {
                    Ok(())
                } else {
                    Err(StoreError::Corrupt)
                }
            }
            Err(e) => Err(e),
        }
    }

    async fn read_object(&self, name: &str, max: usize) -> Result<Bytes, StoreError> {
        let mut op = ObjectOperation::new();
        op.read(0, max.checked_add(1).ok_or(StoreError::Capacity)? as u64);
        let r = self.operate(name, op).await?;
        let b = r.out(0);
        if b.len() > max {
            return Err(StoreError::Capacity);
        }
        Ok(b)
    }

    pub async fn load_checkpoint_bytes(
        &self,
        descriptor: &CheckpointDescriptor,
    ) -> Result<Bytes, StoreError> {
        let manifest = self.read_object(&descriptor.manifest_name, 4096).await?;
        if framing::content_checksum(&manifest)? != descriptor.manifest_checksum {
            return Err(StoreError::Corrupt);
        }
        let decoded = framing::decode(
            &manifest,
            FrameKind::Manifest,
            &self.identity,
            descriptor.covered_seq,
            4096,
        )?;
        let expected_manifest = format!(
            "rmds.checkpoint.0.{}.{}.manifest",
            decoded.owner.shard_epoch, descriptor.generation
        );
        if descriptor.manifest_name != expected_manifest {
            return Err(StoreError::Corrupt);
        }
        let owner = decoded.owner;
        let mut b = decoded.body;
        if b.len() < 12 || b.get_u64_le() != descriptor.generation {
            return Err(StoreError::Corrupt);
        }
        let n = b.get_u32_le() as usize;
        if n > 256 || b.len() != n + 12 {
            return Err(StoreError::Corrupt);
        }
        let name = std::str::from_utf8(&b[..n])
            .map_err(|_| StoreError::Corrupt)?
            .to_owned();
        let expected_name = format!(
            "rmds.checkpoint.0.{}.{}.state",
            owner.shard_epoch, descriptor.generation
        );
        if name != expected_name {
            return Err(StoreError::Corrupt);
        }
        b.advance(n);
        let length = usize::try_from(b.get_u64_le()).map_err(|_| StoreError::Capacity)?;
        let checksum = b.get_u32_le();
        if length > self.limits.max_checkpoint_bytes {
            return Err(StoreError::Capacity);
        }
        let state = self.read_object(&name, length).await?;
        if state.len() != length || framing::content_checksum(&state)? != checksum {
            return Err(StoreError::Corrupt);
        }
        let state = framing::decode(
            &state,
            FrameKind::Checkpoint,
            &self.identity,
            descriptor.covered_seq,
            self.limits.max_checkpoint_bytes,
        )?;
        if state.owner != owner {
            return Err(StoreError::Corrupt);
        }
        Ok(state.body)
    }

    pub async fn publish_checkpoint(
        &self,
        owner: OwnerToken,
        expected_generation: u64,
        descriptor: &CheckpointDescriptor,
    ) -> Result<(), StoreError> {
        self.no_pending()?;
        let c = self.inspect_frozen().await?;
        if c.owner != owner {
            return Err(StoreError::Fenced);
        }
        if c.checkpoint.as_ref() == Some(descriptor) {
            self.load_checkpoint_bytes(descriptor).await?;
            return Ok(());
        }
        if descriptor.manifest_name
            != format!(
                "rmds.checkpoint.0.{}.{}.manifest",
                owner.shard_epoch, descriptor.generation
            )
        {
            return Err(StoreError::Fenced);
        }
        let old_generation = c.checkpoint.as_ref().map_or(0, |d| d.generation);
        let old_coverage = c.checkpoint.as_ref().map_or(0, |d| d.covered_seq);
        if old_generation != expected_generation
            || descriptor.generation
                != expected_generation
                    .checked_add(1)
                    .ok_or(StoreError::Capacity)?
            || descriptor.covered_seq < old_coverage
            || descriptor.covered_seq > c.tail
        {
            return Err(StoreError::GuardRejected);
        }
        // Verify the whole immutable dependency chain before publication.
        self.load_checkpoint_bytes(descriptor).await?;
        let mut op = ObjectOperation::new();
        guards(&mut op, &self.identity, owner);
        op.cmpxattr_str(
            CHECKPOINT,
            cmp_op::EQ,
            &checkpoint_bytes(c.checkpoint.as_ref())?,
        );
        op.cmpxattr_str(TAIL, cmp_op::EQ, &decimal(c.tail));
        op.setxattr(CHECKPOINT, descriptor.encode()?);
        self.operate(OBJECT, op).await.map(|_| ())
    }

    pub async fn trim(
        &self,
        owner: OwnerToken,
        descriptor: &CheckpointDescriptor,
        sequences: &[u64],
    ) -> Result<(), StoreError> {
        self.no_pending()?;
        if sequences.len() > self.limits.page_keys
            || sequences
                .iter()
                .any(|s| *s == 0 || *s > descriptor.covered_seq)
        {
            return Err(StoreError::Capacity);
        }
        let c = self.inspect_frozen().await?;
        if c.owner != owner {
            return Err(StoreError::Fenced);
        }
        let remaining: Vec<u64> = sequences
            .iter()
            .copied()
            .filter(|s| *s > c.trimmed)
            .collect();
        let mut through = c.trimmed;
        for seq in &remaining {
            through = through.checked_add(1).ok_or(StoreError::Capacity)?;
            if *seq != through {
                return Err(StoreError::GuardRejected);
            }
        }
        let mut op = ObjectOperation::new();
        guards(&mut op, &self.identity, owner);
        op.cmpxattr_str(CHECKPOINT, cmp_op::EQ, &descriptor.encode()?);
        op.cmpxattr_str(TRIMMED, cmp_op::EQ, &decimal(c.trimmed));
        op.omap_rm_keys(&remaining.iter().map(|s| key(*s)).collect::<BTreeSet<_>>());
        op.setxattr(TRIMMED, decimal(through));
        self.operate(OBJECT, op).await.map(|_| ())
    }

    fn no_pending(&self) -> Result<(), StoreError> {
        if self.pending.is_some() {
            Err(StoreError::UnknownOutcome)
        } else {
            Ok(())
        }
    }
}

fn guards(op: &mut ObjectOperation, identity: &StoreIdentity, owner: OwnerToken) {
    op.cmpxattr_str(IDENTITY, cmp_op::EQ, &identity.encode());
    op.cmpxattr_str(OWNER, cmp_op::EQ, &owner.encode());
}

fn checkpoint_bytes(d: Option<&CheckpointDescriptor>) -> Result<Bytes, StoreError> {
    d.map_or(Ok(Bytes::new()), CheckpointDescriptor::encode)
}

fn key(seq: u64) -> Bytes {
    Bytes::from(format!("r/{seq:016x}"))
}
fn decimal(n: u64) -> Bytes {
    Bytes::from(n.to_string())
}
fn number(b: &Bytes) -> Result<u64, StoreError> {
    let text = std::str::from_utf8(b).map_err(|_| StoreError::Corrupt)?;
    let n = text.parse::<u64>().map_err(|_| StoreError::Corrupt)?;
    if decimal(n) != *b {
        return Err(StoreError::Corrupt);
    }
    Ok(n)
}

fn rank_owner(owner: OwnerToken) -> RankOwner {
    RankOwner {
        rank: owner.rank,
        incarnation: owner.incarnation,
        gid: owner.gid,
    }
}

impl Authority {
    pub(crate) fn encode(&self) -> Result<Bytes, StoreError> {
        let mut b = BytesMut::new();
        b.put_u8(1);
        b.put_u64_le(self.revision);
        put_writer(&mut b, &self.writer)?;
        b.put_u8(u8::from(self.predecessor.is_some()));
        if let Some(p) = &self.predecessor {
            put_writer(&mut b, &p.writer)?;
            b.put_u32_le(p.verified_osd_epoch);
        }
        b.put_u32_le(crc32c::crc32c(&b));
        Ok(b.freeze())
    }

    pub(crate) fn decode(bytes: &Bytes) -> Result<Self, StoreError> {
        if bytes.len() < 14 || bytes.len() > 8192 {
            return Err(StoreError::Corrupt);
        }
        let end = bytes.len() - 4;
        let mut crc = bytes.slice(end..);
        if crc32c::crc32c(&bytes[..end]) != crc.get_u32_le() {
            return Err(StoreError::Corrupt);
        }
        let mut b = bytes.slice(..end);
        if b.get_u8() != 1 {
            return Err(StoreError::UnsupportedVersion);
        }
        let revision = b.get_u64_le();
        let writer = get_writer(&mut b)?;
        if b.is_empty() {
            return Err(StoreError::Corrupt);
        }
        let predecessor = match b.get_u8() {
            0 => None,
            1 => {
                let writer = get_writer(&mut b)?;
                if b.len() < 4 {
                    return Err(StoreError::Corrupt);
                }
                let verified_osd_epoch = b.get_u32_le();
                if verified_osd_epoch == 0 {
                    return Err(StoreError::Corrupt);
                }
                Some(FencedWriter {
                    writer,
                    verified_osd_epoch,
                })
            }
            _ => return Err(StoreError::Corrupt),
        };
        if !b.is_empty() {
            return Err(StoreError::Corrupt);
        }
        Ok(Self {
            revision,
            writer,
            predecessor,
        })
    }
}

pub(crate) fn put_writer(b: &mut BytesMut, writer: &WriterIdentity) -> Result<(), StoreError> {
    if writer.addresses.0.is_empty() || writer.addresses.0.len() > 16 {
        return Err(StoreError::Capacity);
    }
    if !usable_writer(writer) {
        return Err(StoreError::Fenced);
    }
    let mut value = BytesMut::new();
    value.put_u8(1);
    value.put_u64_le(writer.rank_owner.rank);
    value.put_u64_le(writer.rank_owner.incarnation);
    value.put_u64_le(writer.rank_owner.gid);
    writer
        .addresses
        .encode(&mut value, denc::features::MSG_ADDR2);
    if value.len() > 4096 {
        return Err(StoreError::Capacity);
    }
    b.put_u32_le(value.len() as u32);
    b.extend_from_slice(&value);
    Ok(())
}

pub(crate) fn get_writer(b: &mut Bytes) -> Result<WriterIdentity, StoreError> {
    let mut value = take_string(b, 4096)?;
    if value.len() < 30 {
        return Err(StoreError::Corrupt);
    }
    if value.get_u8() != 1 {
        return Err(StoreError::UnsupportedVersion);
    }
    let rank_owner = RankOwner {
        rank: value.get_u64_le(),
        incarnation: value.get_u64_le(),
        gid: value.get_u64_le(),
    };
    if value[0] != 2 {
        return Err(StoreError::Corrupt);
    }
    let mut count = value.slice(1..5);
    let count = count.get_u32_le();
    if count == 0 || count > 16 {
        return Err(StoreError::Corrupt);
    }
    let addresses = EntityAddrVec::decode(&mut value).map_err(|_| StoreError::Corrupt)?;
    if !value.is_empty() {
        return Err(StoreError::Corrupt);
    }
    let writer = WriterIdentity {
        rank_owner,
        addresses,
    };
    if !usable_writer(&writer) {
        return Err(StoreError::Corrupt);
    }
    Ok(writer)
}

fn usable_writer(writer: &WriterIdentity) -> bool {
    writer.rank_owner.incarnation != 0
        && writer.rank_owner.gid != 0
        && writer.addresses.0.iter().any(|address| {
            address.ty != denc::AddrType::NONE
                && address
                    .addr
                    .is_some_and(|socket| socket.port() != 0 && !socket.ip().is_unspecified())
        })
}

/// Decode OMAPGETVALS without unbounded container allocation. The OSD count and
/// each key/value length are checked before allocation. Reject trailing bytes.
fn decode_page(
    bytes: &Bytes,
    max_keys: usize,
    max_value: usize,
) -> Result<(BTreeMap<Bytes, Bytes>, bool), StoreError> {
    let max_page_bytes = max_value
        .checked_add(26)
        .and_then(|per_key| per_key.checked_mul(max_keys))
        .and_then(|n| n.checked_add(5))
        .ok_or(StoreError::Capacity)?;
    if bytes.len() > max_page_bytes {
        return Err(StoreError::Capacity);
    }
    let mut b = bytes.clone();
    if b.len() < 4 {
        return Err(StoreError::Corrupt);
    }
    let n = b.get_u32_le() as usize;
    if n > max_keys {
        return Err(StoreError::Capacity);
    }
    let mut values = BTreeMap::new();
    let mut previous = None;
    for _ in 0..n {
        let k = take_string(&mut b, 18)?;
        if k.len() != 18 || previous.as_ref().is_some_and(|p| p >= &k) {
            return Err(StoreError::Corrupt);
        }
        let value = take_string(&mut b, max_value)?;
        previous = Some(k.clone());
        values.insert(k, value);
    }
    if b.len() != 1 || b[0] > 1 {
        return Err(StoreError::Corrupt);
    }
    Ok((values, b[0] != 0))
}

fn take_string(b: &mut Bytes, max: usize) -> Result<Bytes, StoreError> {
    if b.len() < 4 {
        return Err(StoreError::Corrupt);
    }
    let n = b.get_u32_le() as usize;
    if n > max {
        return Err(StoreError::Capacity);
    }
    if b.len() < n {
        return Err(StoreError::Corrupt);
    }
    Ok(b.split_to(n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use osdc::{
        objecter::OpResult,
        ops::{self, op},
    };
    use std::{
        future::Future,
        pin::pin,
        sync::{Arc, Mutex},
        task::{Context, Poll, Waker},
    };

    #[derive(Clone, Default)]
    struct Object {
        attrs: BTreeMap<String, Bytes>,
        values: BTreeMap<Bytes, Bytes>,
        data: Bytes,
    }
    #[derive(Default)]
    struct Memory {
        objects: BTreeMap<String, Object>,
        lose_reply: bool,
        reject: bool,
        writes: usize,
        fail_at_write: Option<usize>,
        foreign_objects: Vec<osdc::PoolObject>,
        map_changes: usize,
        unknown_before_write: bool,
        depose_on_write: Option<OwnerToken>,
        omap_entry_cap: Option<usize>,
        omap_byte_cap: Option<usize>,
        omap_pages: Vec<(Bytes, Vec<Bytes>, bool)>,
        omap_more: Option<bool>,
        omap_repeat_first: bool,
        depose_after_omap: Option<OwnerToken>,
        listing_script: Option<std::collections::VecDeque<ListStep>>,
        listing_calls: Vec<(Option<usize>, usize)>,
    }
    struct ListStep {
        expected: Option<usize>,
        result: Result<crate::backend::PoolPage<usize>, osdc::PoolListError>,
    }
    #[derive(Clone, Default)]
    struct TestBackend(Arc<Mutex<Memory>>);

    impl Backend for TestBackend {
        async fn operate(
            &self,
            name: &str,
            operation: ObjectOperation,
        ) -> Result<OpReply, StoreError> {
            let mut memory = self.0.lock().unwrap();
            if memory.reject && operation.has_write() {
                memory.reject = false;
                return Err(StoreError::GuardRejected);
            }
            if operation.has_write() && memory.fail_at_write == Some(memory.writes) {
                memory.fail_at_write = None;
                return Err(StoreError::Io);
            }
            if operation.has_write() && memory.unknown_before_write {
                memory.unknown_before_write = false;
                return Err(StoreError::UnknownOutcome);
            }
            if operation.has_write()
                && let Some(next) = memory.depose_on_write.take()
            {
                let object = memory.objects.get_mut(OBJECT).unwrap();
                let old = Authority::decode(&object.attrs[AUTHORITY]).unwrap();
                let authority = Authority {
                    revision: old.revision + 1,
                    writer: writer(next),
                    predecessor: Some(FencedWriter {
                        writer: old.writer,
                        verified_osd_epoch: 55,
                    }),
                };
                object.attrs.insert(OWNER.to_owned(), next.encode());
                object
                    .attrs
                    .insert(AUTHORITY.to_owned(), authority.encode().unwrap());
            }
            let exists = memory.objects.contains_key(name);
            let mut object = memory.objects.get(name).cloned().unwrap_or_default();
            let mut reply = OpReply::default();
            for o in &operation.ops {
                let mut out = Bytes::new();
                match o.op {
                    op::CREATE => {
                        if exists && o.flags & ops::op_flags::EXCL != 0 {
                            return Err(StoreError::AlreadyExists);
                        }
                    }
                    op::CMPXATTR | op::GETXATTR | op::SETXATTR => {
                        let n = o.xattr_name_len() as usize;
                        let attr = std::str::from_utf8(&o.indata[..n]).unwrap();
                        let value = o.indata.slice(n..);
                        match o.op {
                            op::GETXATTR => {
                                out = object
                                    .attrs
                                    .get(attr)
                                    .cloned()
                                    .ok_or(StoreError::MissingState)?;
                            }
                            op::CMPXATTR => {
                                assert_eq!(o.args[8], cmp_op::EQ);
                                assert_eq!(o.args[9], ops::cmp_mode::STRING);
                                if object.attrs.get(attr) != Some(&value) {
                                    return Err(StoreError::GuardRejected);
                                }
                            }
                            _ => {
                                object.attrs.insert(attr.to_owned(), value);
                            }
                        }
                    }
                    op::OMAPSETVALS => {
                        use denc::Decode;
                        let values =
                            BTreeMap::<Bytes, Bytes>::decode(&mut o.indata.clone()).unwrap();
                        object.values.extend(values);
                    }
                    op::OMAPRMKEYS => {
                        use denc::Decode;
                        for k in BTreeSet::<Bytes>::decode(&mut o.indata.clone()).unwrap() {
                            object.values.remove(&k);
                        }
                    }
                    op::OMAPGETVALS => {
                        let mut input = o.indata.clone();
                        let requested_after = take_string(&mut input, 1024)?;
                        let after = if memory.omap_repeat_first {
                            Bytes::new()
                        } else {
                            requested_after.clone()
                        };
                        let max = input.get_u64_le() as usize;
                        let prefix = take_string(&mut input, 1024)?;
                        let matching: Vec<_> = object
                            .values
                            .iter()
                            .filter(|(k, _)| **k > after && k.starts_with(&prefix))
                            .collect();
                        let mut values = BTreeMap::new();
                        let mut encoded_bytes = 0;
                        for (k, v) in matching
                            .iter()
                            .take(max.min(memory.omap_entry_cap.unwrap_or(max)))
                        {
                            let entry_bytes = 8 + k.len() + v.len();
                            if memory.omap_byte_cap.is_some_and(|cap| encoded_bytes >= cap) {
                                break;
                            }
                            encoded_bytes += entry_bytes;
                            values.insert((*k).clone(), (*v).clone());
                        }
                        let more = memory.omap_more.unwrap_or(matching.len() > values.len());
                        memory.omap_pages.push((
                            requested_after,
                            values.keys().cloned().collect(),
                            more,
                        ));
                        let mut b = BytesMut::from(ops::encode_omap(&values).as_ref());
                        b.put_u8(u8::from(more));
                        out = b.freeze();
                    }
                    op::OMAPGETVALSBYKEYS => {
                        use denc::Decode;
                        let keys = BTreeSet::<Bytes>::decode(&mut o.indata.clone()).unwrap();
                        let values: BTreeMap<_, _> = keys
                            .iter()
                            .filter_map(|k| object.values.get(k).map(|v| (k.clone(), v.clone())))
                            .collect();
                        out = ops::encode_omap(&values);
                    }
                    op::WRITEFULL => {
                        object.data = o.indata.clone();
                    }
                    op::READ => {
                        if !exists {
                            return Err(StoreError::MissingState);
                        }
                        let n = (o.extent_length() as usize).min(object.data.len());
                        out = object.data.slice(..n);
                    }
                    _ => panic!("unexpected operation {}", o.op),
                }
                reply.ops.push(OpResult {
                    rval: 0,
                    outdata: out,
                });
            }
            if name == OBJECT
                && operation.ops.iter().any(|o| o.op == op::OMAPGETVALS)
                && let Some(next) = memory.depose_after_omap.take()
            {
                let current = memory.objects.get_mut(OBJECT).unwrap();
                let old = Authority::decode(&current.attrs[AUTHORITY]).unwrap();
                current.attrs.insert(OWNER.to_owned(), next.encode());
                current.attrs.insert(
                    AUTHORITY.to_owned(),
                    Authority {
                        revision: old.revision + 1,
                        writer: writer(next),
                        predecessor: Some(FencedWriter {
                            writer: old.writer,
                            verified_osd_epoch: 55,
                        }),
                    }
                    .encode()
                    .unwrap(),
                );
            }
            if operation.has_write() {
                // Commit the private image only after all guards succeed.
                memory.objects.insert(name.to_owned(), object);
                memory.writes += 1;
                if memory.lose_reply {
                    memory.lose_reply = false;
                    return Err(StoreError::UnknownOutcome);
                }
            }
            Ok(reply)
        }
    }

    impl crate::backend::PoolListing for TestBackend {
        type Cursor = usize;
        async fn list_pool_page(
            &self,
            cursor: Option<Self::Cursor>,
            max_entries: u32,
            max_bytes: usize,
        ) -> Result<crate::backend::PoolPage<Self::Cursor>, osdc::PoolListError> {
            let mut memory = self.0.lock().unwrap();
            let writes = memory.writes;
            memory.listing_calls.push((cursor, writes));
            if let Some(script) = &mut memory.listing_script {
                let step = script
                    .pop_front()
                    .expect("unexpected listing call after script exhaustion");
                assert_eq!(cursor, step.expected);
                if let Ok(page) = &step.result {
                    assert!(page.objects.len() <= max_entries as usize);
                    assert!(
                        page.objects
                            .iter()
                            .map(|o| o.name.len() + o.namespace.len() + o.locator.len())
                            .sum::<usize>()
                            <= max_bytes
                    );
                }
                return step.result;
            }
            if memory.map_changes > 0 {
                memory.map_changes -= 1;
                return Err(osdc::PoolListError::MapChanged);
            }
            let mut objects: Vec<_> = memory
                .objects
                .keys()
                .map(|name| osdc::PoolObject {
                    namespace: vec![],
                    name: name.as_bytes().to_vec(),
                    locator: vec![],
                })
                .collect();
            objects.extend(memory.foreign_objects.clone());
            assert!(objects.len() <= max_entries as usize);
            assert!(
                objects
                    .iter()
                    .map(|o| o.name.len() + o.namespace.len() + o.locator.len())
                    .sum::<usize>()
                    <= max_bytes
            );
            Ok(crate::backend::PoolPage {
                objects,
                next: None,
                osdmap_epoch: 1,
            })
        }
    }

    fn run<T>(future: impl Future<Output = T>) -> T {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        match pin!(future).as_mut().poll(&mut context) {
            Poll::Ready(v) => v,
            Poll::Pending => panic!("test backend unexpectedly yielded"),
        }
    }

    fn identity() -> StoreIdentity {
        StoreIdentity {
            cluster_fsid: [7; 16],
            filesystem_id: 42,
            metadata_pool: 3,
        }
    }
    fn owner() -> OwnerToken {
        OwnerToken {
            shard_epoch: 1,
            rank: 0,
            incarnation: 2,
            gid: 3,
        }
    }
    fn writer(owner: OwnerToken) -> WriterIdentity {
        WriterIdentity {
            rank_owner: rank_owner(owner),
            addresses: EntityAddrVec(vec![denc::EntityAddr::new(
                denc::AddrType::LEGACY,
                "127.0.0.1:7402".parse().unwrap(),
                owner.gid as u32,
            )]),
        }
    }

    fn fixture() -> (TestBackend, StableStore<TestBackend>) {
        let backend = TestBackend::default();
        let mut object = Object::default();
        for (name, value) in [
            (IDENTITY, identity().encode()),
            (OWNER, owner().encode()),
            (TAIL, decimal(0)),
            (BATCH, Bytes::new()),
            (CHECKPOINT, Bytes::new()),
            (TRIMMED, decimal(0)),
        ] {
            object.attrs.insert(name.to_owned(), value);
        }
        object.attrs.insert(
            AUTHORITY.to_owned(),
            Authority {
                revision: 1,
                writer: writer(owner()),
                predecessor: None,
            }
            .encode()
            .unwrap(),
        );
        backend
            .0
            .lock()
            .unwrap()
            .objects
            .insert(OBJECT.to_owned(), object);
        let store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
        (backend, store)
    }

    fn records() -> Vec<Bytes> {
        vec![
            crate::record::encode_record(&engine::JournalRecord::new(
                engine::journal::RecordKind::Create,
            )),
            crate::record::encode_record(&engine::JournalRecord::new(
                engine::journal::RecordKind::Rename,
            )),
        ]
    }

    #[test]
    fn unknown_success_freezes_and_reconciles_identical_batch() {
        let (backend, mut store) = fixture();
        backend.0.lock().unwrap().lose_reply = true;
        assert_eq!(
            run(store.append(owner(), 0, [4; 16], &records())),
            Err(StoreError::UnknownOutcome)
        );
        assert_eq!(
            run(store.append(owner(), 2, [5; 16], &records())),
            Err(StoreError::UnknownOutcome)
        );
        assert_eq!(
            run(store.trim(
                owner(),
                &CheckpointDescriptor {
                    generation: 1,
                    covered_seq: 0,
                    manifest_name: String::new(),
                    manifest_checksum: 0
                },
                &[]
            )),
            Err(StoreError::UnknownOutcome)
        );
        assert_eq!(run(store.append(owner(), 0, [4; 16], &records())), Ok(2));
        assert_eq!(run(store.append(owner(), 0, [4; 16], &records())), Ok(2));
        assert_eq!(backend.0.lock().unwrap().writes, 1);
        let opened = run(store.inspect_frozen()).unwrap();
        assert_eq!(
            run(store.load_tail(&opened)).unwrap(),
            vec![(1, records()[0].clone()), (2, records()[1].clone())]
        );
    }

    #[test]
    fn negative_guard_never_reports_commit_and_identical_retry_is_safe() {
        let (backend, mut store) = fixture();
        backend.0.lock().unwrap().reject = true;
        assert_eq!(
            run(store.append(owner(), 0, [4; 16], &records())),
            Err(StoreError::GuardRejected)
        );
        assert_eq!(run(store.inspect_frozen()).unwrap().tail, 0);
        assert_eq!(run(store.reconcile_pending()), Ok(2));
    }

    #[test]
    fn missing_or_corrupt_required_records_never_shorten_replay() {
        for corrupt in [false, true] {
            let (backend, mut store) = fixture();
            run(store.append(owner(), 0, [4; 16], &records())).unwrap();
            let opened = run(store.inspect_frozen()).unwrap();
            let mut memory = backend.0.lock().unwrap();
            let object = memory.objects.get_mut(OBJECT).unwrap();
            if corrupt {
                let mut frame = object.values[&key(2)].to_vec();
                frame[50] ^= 1;
                object.values.insert(key(2), Bytes::from(frame));
            } else {
                object.values.remove(&key(2));
            }
            drop(memory);
            assert_eq!(run(store.load_tail(&opened)), Err(StoreError::Corrupt));
        }
    }

    #[test]
    fn checkpoint_publication_trim_and_checkpoint_only_load() {
        let (_, mut store) = fixture();
        run(store.append(owner(), 0, [4; 16], &records())).unwrap();
        let state = Bytes::from_static(b"materialized namespace, allocator, histories and jobs");
        let descriptor = run(store.write_checkpoint_bytes(owner(), 1, 2, &state)).unwrap();
        assert!(run(store.inspect_frozen()).unwrap().checkpoint.is_none());
        run(store.publish_checkpoint(owner(), 0, &descriptor)).unwrap();
        run(store.trim(owner(), &descriptor, &[1])).unwrap();
        run(store.trim(owner(), &descriptor, &[2])).unwrap();
        let opened = run(store.inspect_frozen()).unwrap();
        assert_eq!(run(store.load_tail(&opened)), Ok(vec![]));
        assert_eq!(
            run(store.load_checkpoint_bytes(opened.checkpoint.as_ref().unwrap())),
            Ok(state)
        );
        run(store.publish_checkpoint(owner(), 0, &descriptor)).unwrap();
        run(store.trim(owner(), &descriptor, &[1, 2])).unwrap();
    }

    #[test]
    fn crash_before_publication_keeps_old_checkpoint_and_tail() {
        let (backend, mut store) = fixture();
        run(store.append(owner(), 0, [4; 16], &records())).unwrap();
        let old =
            run(store.write_checkpoint_bytes(owner(), 1, 0, &Bytes::from_static(b"old"))).unwrap();
        run(store.publish_checkpoint(owner(), 0, &old)).unwrap();
        let orphan =
            run(store.write_checkpoint_bytes(owner(), 2, 2, &Bytes::from_static(b"new"))).unwrap();
        let reloaded = StableStore::new(backend, identity(), Limits::default()).unwrap();
        let opened = run(reloaded.inspect_frozen()).unwrap();
        assert_eq!(opened.checkpoint, Some(old.clone()));
        assert_eq!(
            run(reloaded.load_checkpoint_bytes(&old)),
            Ok(Bytes::from_static(b"old"))
        );
        assert_eq!(run(reloaded.load_tail(&opened)).unwrap().len(), 2);
        assert_ne!(orphan, old);
    }

    #[test]
    fn stale_owner_cannot_append_publish_or_trim() {
        let (backend, mut store) = fixture();
        run(store.append(owner(), 0, [4; 16], &records())).unwrap();
        let descriptor =
            run(store.write_checkpoint_bytes(owner(), 1, 2, &Bytes::from_static(b"state")))
                .unwrap();
        run(store.publish_checkpoint(owner(), 0, &descriptor)).unwrap();
        let mut replacement = owner();
        replacement.shard_epoch += 1;
        backend
            .0
            .lock()
            .unwrap()
            .objects
            .get_mut(OBJECT)
            .unwrap()
            .attrs
            .insert(OWNER.to_owned(), replacement.encode());
        assert_eq!(
            run(store.append(owner(), 2, [5; 16], &records())),
            Err(StoreError::Fenced)
        );
        // Separate facade has no uncertain append. The on-object guard still rejects it.
        let stale = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
        assert_eq!(
            run(stale.publish_checkpoint(owner(), 0, &descriptor)),
            Err(StoreError::Fenced)
        );
        assert_eq!(
            run(stale.trim(owner(), &descriptor, &[1, 2])),
            Err(StoreError::Fenced)
        );
        assert_eq!(backend.0.lock().unwrap().objects[OBJECT].values.len(), 2);
    }

    #[test]
    fn descriptor_substitution_and_missing_checkpoint_fail() {
        let (backend, store) = fixture();
        let descriptor =
            run(store.write_checkpoint_bytes(owner(), 1, 0, &Bytes::from_static(b"state")))
                .unwrap();
        let mut wrong = descriptor.clone();
        wrong.covered_seq = 1;
        assert_eq!(
            run(store.load_checkpoint_bytes(&wrong)),
            Err(StoreError::Corrupt)
        );
        backend
            .0
            .lock()
            .unwrap()
            .objects
            .remove(&descriptor.manifest_name);
        assert_eq!(
            run(store.load_checkpoint_bytes(&descriptor)),
            Err(StoreError::MissingState)
        );
    }

    #[test]
    fn framing_protects_every_byte_and_rejects_other_identity() {
        let frame = framing::encode(
            FrameKind::Record,
            &identity(),
            owner(),
            7,
            [9; 16],
            b"body",
            1024,
        )
        .unwrap();
        for byte in 0..frame.len() {
            let mut damaged = frame.to_vec();
            damaged[byte] ^= 1;
            assert!(
                framing::decode(
                    &Bytes::from(damaged),
                    FrameKind::Record,
                    &identity(),
                    7,
                    1024
                )
                .is_err(),
                "byte {byte}"
            );
        }
        let mut wrong = identity();
        wrong.filesystem_id += 1;
        assert!(matches!(
            framing::decode(&frame, FrameKind::Record, &wrong, 7, 1024),
            Err(StoreError::WrongFormat)
        ));
        assert!(framing::decode(&frame, FrameKind::Record, &identity(), 8, 1024).is_err());
        assert!(framing::decode(&frame, FrameKind::Checkpoint, &identity(), 7, 1024).is_err());
    }

    #[test]
    fn pages_and_admission_are_bounded() {
        let (backend, _) = fixture();
        let limits = Limits {
            max_wal_keys: 1,
            page_keys: 1,
            ..Limits::default()
        };
        let mut store = StableStore::new(backend.clone(), identity(), limits).unwrap();
        assert_eq!(
            run(store.append(owner(), 0, [4; 16], &records())),
            Err(StoreError::Capacity)
        );
        assert_eq!(backend.0.lock().unwrap().writes, 0);
        let mut b = BytesMut::new();
        b.put_u32_le(u32::MAX);
        assert_eq!(decode_page(&b.freeze(), 1, 1024), Err(StoreError::Capacity));
    }

    #[test]
    fn acquisition_preserves_tail_and_reconciles_lost_response() {
        let (backend, mut store) = fixture();
        run(store.append(owner(), 0, [4; 16], &records())).unwrap();
        let expected = run(store.inspect_control()).unwrap();
        let mut next = owner();
        next.shard_epoch += 1;
        next.incarnation += 1;
        next.gid += 1;
        let evidence = FencedWriter {
            writer: expected.writer.clone(),
            verified_osd_epoch: 45,
        };
        backend.0.lock().unwrap().lose_reply = true;
        let opened = run(store.open(&expected, next, writer(next), evidence.clone())).unwrap();
        assert_eq!(opened.tail, 2);
        assert_eq!(opened.frozen.authority.predecessor, Some(evidence.clone()));
        assert_eq!(
            run(store.open(&expected, next, writer(next), evidence))
                .unwrap()
                .tail,
            2
        );
        assert_eq!(run(store.load_tail(&opened.frozen)).unwrap().len(), 2);
        assert_eq!(
            run(store.append(owner(), 2, [5; 16], &records())),
            Err(StoreError::Fenced)
        );
    }

    #[test]
    fn acquisition_rejects_identity_substitution_and_missing_fence_evidence() {
        let (_, store) = fixture();
        let expected = run(store.inspect_control()).unwrap();
        let mut next = owner();
        next.shard_epoch += 1;
        let evidence = FencedWriter {
            writer: writer(next),
            verified_osd_epoch: 0,
        };
        assert!(matches!(
            run(store.open(&expected, next, writer(next), evidence)),
            Err(StoreError::Fenced)
        ));
        let mut substituted = expected.writer.clone();
        substituted.rank_owner.gid += 1;
        let evidence = FencedWriter {
            writer: substituted,
            verified_osd_epoch: 45,
        };
        assert!(matches!(
            run(store.open(&expected, next, writer(next), evidence)),
            Err(StoreError::Fenced)
        ));
    }

    fn session_fixture() -> (
        TestBackend,
        crate::sessions::SessionRepository<TestBackend>,
        crate::sessions::AcquiredSessions,
    ) {
        let (backend, _) = fixture();
        let mut object = Object::default();
        object
            .attrs
            .insert(IDENTITY.to_owned(), identity().encode());
        object.attrs.insert(
            AUTHORITY.to_owned(),
            Authority {
                revision: 1,
                writer: writer(owner()),
                predecessor: None,
            }
            .encode()
            .unwrap(),
        );
        object
            .attrs
            .insert("rmds.next_incarnation".to_owned(), decimal(1));
        backend
            .0
            .lock()
            .unwrap()
            .objects
            .insert("rmds.sessions.0".to_owned(), object);
        let repository =
            crate::sessions::SessionRepository::new(backend.clone(), identity(), 0, 128).unwrap();
        let acquired = crate::sessions::AcquiredSessions {
            control: run(repository.inspect_sessions()).unwrap(),
        };
        (backend, repository, acquired)
    }

    fn new_session(client: u64) -> crate::sessions::NewSessionRecord {
        crate::sessions::NewSessionRecord {
            client,
            open_id: [client as u8; 16],
            authenticated_entity: "client.test".to_owned(),
            authorization_context: Bytes::from_static(b"allow rw path=/test"),
            peer_addresses: writer(owner()).addresses,
            client_features: 123,
            server_features: 456,
            metadata: [
                (Bytes::from_static(b"root"), Bytes::from_static(b"/test")),
                (
                    Bytes::from_static(b"uuid"),
                    Bytes::from_static(b"test-uuid"),
                ),
                (Bytes::from_static(b"timeout"), Bytes::from_static(b"60")),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn rank_open_close_unknown_outcomes_preserve_incarnations_and_tombstones() {
        use crate::sessions::SessionLifecycle;
        let (backend, mut repository, acquired) = session_fixture();
        let mut record = new_session(10);
        record.metadata.insert(
            Bytes::from_static(b"root"),
            Bytes::from_static(b"/test/\xff"),
        );
        backend.0.lock().unwrap().lose_reply = true;
        assert_eq!(
            run(repository.open_session_record(&acquired, 1, record.clone())),
            Err(StoreError::UnknownOutcome)
        );
        assert_eq!(
            run(repository.open_session_record(&acquired, 1, new_session(11))),
            Err(StoreError::UnknownOutcome)
        );
        let (key, revision) =
            run(repository.open_session_record(&acquired, 1, record.clone())).unwrap();
        assert_eq!(key.incarnation, 1);
        assert_eq!(
            run(repository.open_session_record(&acquired, 1, record.clone())),
            Ok((key, revision))
        );
        backend.0.lock().unwrap().lose_reply = true;
        assert_eq!(
            run(repository.mark_session_closing(&acquired, revision, key)),
            Err(StoreError::UnknownOutcome)
        );
        let revision = run(repository.mark_session_closing(&acquired, revision, key)).unwrap();
        assert_eq!(
            run(repository.open_session_record(&acquired, revision, record.clone())),
            Err(StoreError::GuardRejected)
        );
        assert_eq!(
            run(repository.mark_session_closed(&acquired, revision, key, 0)),
            Err(StoreError::Corrupt)
        );
        backend.0.lock().unwrap().lose_reply = true;
        assert_eq!(
            run(repository.mark_session_closed(&acquired, revision, key, 99)),
            Err(StoreError::UnknownOutcome)
        );
        let revision = run(repository.mark_session_closed(&acquired, revision, key, 99)).unwrap();
        let (new_key, _) =
            run(repository.open_session_record(&acquired, revision, record)).unwrap();
        assert_eq!(new_key.incarnation, 2);
        let state = run(repository.load_sessions(&acquired)).unwrap();
        assert_eq!(state.control.next_incarnation, 3);
        assert_eq!(state.records.len(), 2);
        assert_eq!(state.records[0].lifecycle, SessionLifecycle::Closed);
        assert_eq!(state.records[0].cleanup_seq, Some(99));
        assert_eq!(
            state.records[0]
                .identity
                .metadata
                .get(&Bytes::from_static(b"root")),
            Some(&Bytes::from_static(b"/test/\xff"))
        );
        assert_eq!(state.records[1].lifecycle, SessionLifecycle::Open);
    }

    #[test]
    fn partial_shard_session_acquisition_preserves_each_independent_authority() {
        let (backend, mut repository, acquired) = session_fixture();
        let (key, revision) =
            run(repository.open_session_record(&acquired, 1, new_session(10))).unwrap();
        let store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
        let old_shard = run(store.inspect_control()).unwrap();
        let old_sessions = run(repository.inspect_sessions()).unwrap();
        let mut next = owner();
        next.shard_epoch += 1;
        next.incarnation += 1;
        next.gid += 1;
        run(store.open(
            &old_shard,
            next,
            writer(next),
            FencedWriter {
                writer: old_shard.writer.clone(),
                verified_osd_epoch: 45,
            },
        ))
        .unwrap();
        assert_eq!(run(repository.inspect_sessions()).unwrap(), old_sessions);
        assert_eq!(old_sessions.revision, revision);
        backend.0.lock().unwrap().lose_reply = true;
        let next_acquired = run(repository.acquire_sessions(
            &old_sessions,
            rank_owner(next),
            writer(next),
            FencedWriter {
                writer: old_sessions.writer.clone(),
                verified_osd_epoch: 45,
            },
        ))
        .unwrap();
        let state = run(repository.load_sessions(&next_acquired)).unwrap();
        assert_eq!(state.control.next_incarnation, 2);
        assert_eq!(state.records[0].key, key);
        assert_eq!(
            run(repository.mark_session_closing(&acquired, revision, key)),
            Err(StoreError::Fenced)
        );
        assert_eq!(state.records[0].identity, new_session(10));
    }

    #[test]
    fn session_pages_include_all_rows_and_reject_corruption() {
        let (backend, mut repository, acquired) = session_fixture();
        let mut revision = 1;
        for client in 1..=20 {
            revision =
                run(repository.open_session_record(&acquired, revision, new_session(client)))
                    .unwrap()
                    .1;
        }
        assert_eq!(
            run(repository.load_sessions(&acquired))
                .unwrap()
                .records
                .len(),
            20
        );
        let mut memory = backend.0.lock().unwrap();
        let object = memory.objects.get_mut("rmds.sessions.0").unwrap();
        let k = Bytes::from("s/0000000000000001/0000000000000001");
        let mut damaged = object.values[&k].to_vec();
        damaged[10] ^= 1;
        object.values.insert(k, Bytes::from(damaged));
        drop(memory);
        assert!(matches!(
            run(repository.load_sessions(&acquired)),
            Err(StoreError::Corrupt)
        ));
    }

    fn bootstrap_plan() -> crate::bootstrap::BootstrapPlan {
        let config = engine::EngineConfig::default();
        let initial_range = config.ino_lease;
        let e = engine::Engine::new(config, engine::journal::MemJournal::new());
        crate::bootstrap::BootstrapPlan::new(
            [5; 16],
            identity(),
            e.checkpoint().unwrap(),
            initial_range,
            owner(),
            writer(owner()),
        )
        .unwrap()
    }

    #[test]
    fn bootstrap_crashes_resume_exact_plan_and_restore_without_old_wal() {
        use crate::bootstrap::{BootstrapStore, FormatPhase};
        for boundary in 1..=6 {
            let backend = TestBackend::default();
            let bootstrap = BootstrapStore::new(backend.clone(), identity());
            let plan = bootstrap_plan();
            let preparing = run(bootstrap.begin_bootstrap(plan.clone(), writer(owner()))).unwrap();
            backend.0.lock().unwrap().fail_at_write = Some(boundary);
            assert!(run(bootstrap.resume_bootstrap(&preparing, owner())).is_err());
            let reloaded = BootstrapStore::new(backend.clone(), identity());
            let preparing = run(reloaded.inspect_format()).unwrap().unwrap();
            assert_eq!(preparing.phase, FormatPhase::Preparing);
            assert_eq!(preparing.plan, plan);
            let opened = run(reloaded.resume_bootstrap(&preparing, owner())).unwrap();
            let ready = run(reloaded.publish_ready(&preparing)).unwrap();
            assert_eq!(ready.phase, FormatPhase::Ready);
            let store = StableStore::new(backend, identity(), Limits::default()).unwrap();
            let (checkpoint, tail) = run(store.load(&opened)).unwrap();
            assert!(tail.is_empty());
            assert_eq!(checkpoint, plan.root_state);
            let restored = engine::Engine::restore(
                engine::EngineConfig::default(),
                engine::journal::MemJournal::at_seq(checkpoint.covered_seq, true),
                checkpoint.clone(),
            )
            .unwrap();
            assert_eq!(restored.checkpoint().unwrap(), checkpoint);
        }
    }

    #[test]
    fn preparing_takeover_requires_each_object_guard_and_rejects_old_preparer() {
        use crate::bootstrap::BootstrapStore;
        let backend = TestBackend::default();
        let bootstrap = BootstrapStore::new(backend.clone(), identity());
        let preparing = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
        backend.0.lock().unwrap().fail_at_write = Some(3);
        assert!(run(bootstrap.resume_bootstrap(&preparing, owner())).is_err());
        let mut next = owner();
        next.shard_epoch += 1;
        next.incarnation += 1;
        next.gid += 1;
        let evidence = FencedWriter {
            writer: writer(owner()),
            verified_osd_epoch: 55,
        };
        let takeover =
            run(bootstrap.acquire_preparing(&preparing, writer(next), evidence.clone())).unwrap();
        assert!(matches!(
            run(bootstrap.resume_bootstrap(&takeover, next)),
            Err(StoreError::Fenced)
        ));
        let store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
        let control = run(store.inspect_control()).unwrap();
        run(store.open(&control, next, writer(next), evidence.clone())).unwrap();
        let sessions =
            crate::sessions::SessionRepository::new(backend.clone(), identity(), 0, 128).unwrap();
        let control = run(sessions.inspect_sessions()).unwrap();
        run(sessions.acquire_sessions(&control, rank_owner(next), writer(next), evidence)).unwrap();
        let opened = run(bootstrap.resume_bootstrap(&takeover, next)).unwrap();
        assert_eq!(opened.owner, next);
        assert!(matches!(
            run(bootstrap.publish_ready(&preparing)),
            Err(StoreError::Fenced)
        ));
        backend.0.lock().unwrap().lose_reply = true;
        run(bootstrap.publish_ready(&takeover)).unwrap();
    }

    #[test]
    fn bootstrap_pool_scan_rejects_foreign_namespace_and_prefix_lookalikes() {
        use crate::bootstrap::BootstrapStore;
        for (namespace, name) in [
            (vec![], b"rmds.not-ours".to_vec()),
            (b"foreign".to_vec(), b"rmds.format".to_vec()),
        ] {
            let backend = TestBackend::default();
            backend
                .0
                .lock()
                .unwrap()
                .foreign_objects
                .push(osdc::PoolObject {
                    namespace,
                    name,
                    locator: vec![],
                });
            let bootstrap = BootstrapStore::new(backend.clone(), identity());
            assert!(matches!(
                run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))),
                Err(StoreError::WrongFormat)
            ));
            assert_eq!(backend.0.lock().unwrap().writes, 0);
        }
        let backend = TestBackend::default();
        backend.0.lock().unwrap().map_changes = 5;
        let bootstrap = BootstrapStore::new(backend.clone(), identity());
        assert!(run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).is_err());
        assert_eq!(backend.0.lock().unwrap().writes, 0);
    }

    #[test]
    fn allocator_unknown_reservation_never_reuses_numbers() {
        let backend = TestBackend::default();
        let bootstrap = crate::bootstrap::BootstrapStore::new(backend.clone(), identity());
        let plan = bootstrap_plan();
        let first_free = plan.initial_range.start + plan.initial_range.len;
        let preparing = run(bootstrap.begin_bootstrap(plan, writer(owner()))).unwrap();
        run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
        run(bootstrap.publish_ready(&preparing)).unwrap();
        let mut allocator = crate::allocator::InoAllocator::new(backend.clone(), identity());
        backend.0.lock().unwrap().lose_reply = true;
        assert_eq!(
            run(allocator.reserve_range([6; 16], 10)),
            Err(StoreError::UnknownOutcome)
        );
        assert_eq!(
            run(allocator.reserve_range([7; 16], 10)),
            Err(StoreError::UnknownOutcome)
        );
        let range = run(allocator.reserve_range([6; 16], 10)).unwrap();
        assert_eq!(range.start, first_free);
        let mut reloaded = crate::allocator::InoAllocator::new(backend, identity());
        assert_eq!(run(reloaded.reserve_range([6; 16], 10)), Ok(range));
        let second = run(reloaded.reserve_range([7; 16], 10)).unwrap();
        assert_eq!(second.start, range.start + range.len);
        assert_eq!(
            run(reloaded.reserve_range([6; 16], 11)),
            Err(StoreError::Corrupt)
        );
    }

    #[test]
    fn native_store_shape_restores_materialized_engine_after_every_wal_key_is_removed() {
        use engine::{
            Engine, EngineConfig, Op, Request,
            journal::MemJournal,
            types::{FilePath, XattrMap},
        };
        let backend = TestBackend::default();
        let bootstrap = crate::bootstrap::BootstrapStore::new(backend.clone(), identity());
        let plan = bootstrap_plan();
        let preparing = run(bootstrap.begin_bootstrap(plan.clone(), writer(owner()))).unwrap();
        run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
        run(bootstrap.publish_ready(&preparing)).unwrap();
        let mut repository =
            crate::sessions::SessionRepository::new(backend.clone(), identity(), 0, 128).unwrap();
        let acquired = crate::sessions::AcquiredSessions {
            control: run(repository.inspect_sessions()).unwrap(),
        };
        let (key, _) = run(repository.open_session_record(
            &acquired,
            acquired.control.revision,
            new_session(10),
        ))
        .unwrap();
        let mut engine = Engine::restore(
            EngineConfig::default(),
            MemJournal::at_seq(plan.root_state.covered_seq, true),
            plan.root_state,
        )
        .unwrap();
        engine.install_session_key(key).unwrap();
        let effects = engine.handle_request(
            key.client,
            Request::new(
                1,
                Op::Create {
                    path: FilePath::child(1, "retained"),
                    flags: engine::consts::CEPH_O_CREAT | engine::consts::CEPH_O_RDWR,
                    mode: 0o640,
                    layout: Default::default(),
                    xattrs: XattrMap::from([(b"user.persist".to_vec(), b"yes".to_vec())]),
                    ino: 0,
                },
            ),
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, engine::Effect::Reply(reply) if reply.result == 0))
        );
        let checkpoint = engine.checkpoint().unwrap();
        let bodies: Vec<_> = engine
            .journal()
            .records()
            .iter()
            .map(|(_, record)| crate::record::encode_record(record))
            .collect();
        let mut store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
        run(store.append(
            owner(),
            preparing.plan.root_state.covered_seq,
            [9; 16],
            &bodies,
        ))
        .unwrap();
        let descriptor = run(store.write_checkpoint(owner(), 2, &checkpoint)).unwrap();
        run(store.publish_checkpoint(owner(), 1, &descriptor)).unwrap();
        run(store.trim(owner(), &descriptor, &[checkpoint.covered_seq])).unwrap();
        assert!(backend.0.lock().unwrap().objects[OBJECT].values.is_empty());
        let expected = run(store.inspect_control()).unwrap();
        let mut next = owner();
        next.shard_epoch += 1;
        next.incarnation += 1;
        next.gid += 1;
        let evidence = FencedWriter {
            writer: expected.writer.clone(),
            verified_osd_epoch: 55,
        };
        let opened = run(store.open(&expected, next, writer(next), evidence.clone())).unwrap();
        let old_sessions = run(repository.inspect_sessions()).unwrap();
        let acquired = run(repository.acquire_sessions(
            &old_sessions,
            rank_owner(next),
            writer(next),
            evidence,
        ))
        .unwrap();
        let rank_state = run(repository.load_sessions(&acquired)).unwrap();
        let (recovered, records) = run(store.load(&opened)).unwrap();
        assert!(records.is_empty());
        assert_eq!(recovered, checkpoint);
        let mut restored = Engine::restore(
            EngineConfig::default(),
            MemJournal::at_seq(recovered.covered_seq, false),
            recovered.clone(),
        )
        .unwrap();
        restored
            .install_session_key(rank_state.records[0].key)
            .unwrap();
        assert_eq!(restored.checkpoint().unwrap(), recovered);
        assert!(restored.lookup_name(1, b"retained").is_some());
        assert!(restored.journal().records().is_empty());
    }

    #[test]
    fn unknown_uncommitted_append_retries_exact_bytes_and_owner_loss_stays_fenced() {
        let (backend, mut store) = fixture();
        backend.0.lock().unwrap().unknown_before_write = true;
        assert_eq!(
            run(store.append(owner(), 0, [4; 16], &records())),
            Err(StoreError::UnknownOutcome)
        );
        assert_eq!(run(store.inspect_frozen()).unwrap().tail, 0);
        assert_eq!(run(store.reconcile_pending()), Ok(2));
        backend.0.lock().unwrap().lose_reply = true;
        assert_eq!(
            run(store.append(owner(), 2, [5; 16], &records())),
            Err(StoreError::UnknownOutcome)
        );
        let new_store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
        let expected = run(new_store.inspect_control()).unwrap();
        let mut next = owner();
        next.shard_epoch += 1;
        next.incarnation += 1;
        next.gid += 1;
        let opened = run(new_store.open(
            &expected,
            next,
            writer(next),
            FencedWriter {
                writer: expected.writer.clone(),
                verified_osd_epoch: 55,
            },
        ))
        .unwrap();
        assert_eq!(run(store.reconcile_pending()), Err(StoreError::Fenced));
        assert_eq!(run(new_store.load_tail(&opened.frozen)).unwrap().len(), 4);
    }

    #[test]
    fn partial_checkpoint_write_and_uncertain_publication_trim_preserve_history() {
        let (backend, mut store) = fixture();
        run(store.append(owner(), 0, [4; 16], &records())).unwrap();
        let old =
            run(store.write_checkpoint_bytes(owner(), 1, 0, &Bytes::from_static(b"old"))).unwrap();
        run(store.publish_checkpoint(owner(), 0, &old)).unwrap();
        let writes = backend.0.lock().unwrap().writes;
        backend.0.lock().unwrap().fail_at_write = Some(writes + 1);
        assert_eq!(
            run(store.write_checkpoint_bytes(owner(), 2, 2, &Bytes::from_static(b"new"))),
            Err(StoreError::Io)
        );
        let control = run(store.inspect_frozen()).unwrap();
        assert_eq!(control.checkpoint, Some(old.clone()));
        assert_eq!(
            run(store.load_checkpoint_bytes(&old)),
            Ok(Bytes::from_static(b"old"))
        );
        assert_eq!(run(store.load_tail(&control)).unwrap().len(), 2);
        let new =
            run(store.write_checkpoint_bytes(owner(), 2, 2, &Bytes::from_static(b"new"))).unwrap();
        backend.0.lock().unwrap().lose_reply = true;
        assert_eq!(
            run(store.publish_checkpoint(owner(), 1, &new)),
            Err(StoreError::UnknownOutcome)
        );
        run(store.publish_checkpoint(owner(), 1, &new)).unwrap();
        backend.0.lock().unwrap().lose_reply = true;
        assert_eq!(
            run(store.trim(owner(), &new, &[1, 2])),
            Err(StoreError::UnknownOutcome)
        );
        run(store.trim(owner(), &new, &[1, 2])).unwrap();
        assert!(backend.0.lock().unwrap().objects[OBJECT].values.is_empty());
        assert_eq!(
            run(store.load_checkpoint_bytes(&new)),
            Ok(Bytes::from_static(b"new"))
        );
    }

    #[test]
    fn orphan_checkpoint_objects_apply_backlog_pressure_across_owner_epochs() {
        let (backend, _) = fixture();
        let limits = Limits {
            max_checkpoint_generations: 1,
            ..Limits::default()
        };
        let store = StableStore::new(backend.clone(), identity(), limits).unwrap();
        run(store.write_checkpoint_bytes(owner(), 1, 0, &Bytes::from_static(b"orphan"))).unwrap();
        let expected = run(store.inspect_control()).unwrap();
        let mut next = owner();
        next.shard_epoch += 1;
        next.incarnation += 1;
        next.gid += 1;
        run(store.open(
            &expected,
            next,
            writer(next),
            FencedWriter {
                writer: expected.writer.clone(),
                verified_osd_epoch: 55,
            },
        ))
        .unwrap();
        let writes = backend.0.lock().unwrap().writes;
        assert_eq!(
            run(store.write_checkpoint_bytes(next, 1, 0, &Bytes::from_static(b"replacement"))),
            Err(StoreError::Capacity)
        );
        assert_eq!(backend.0.lock().unwrap().writes, writes);
    }

    #[test]
    fn ready_marker_never_substitutes_for_missing_dependencies() {
        let backend = TestBackend::default();
        let bootstrap = crate::bootstrap::BootstrapStore::new(backend.clone(), identity());
        let preparing = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
        run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
        let ready = run(bootstrap.publish_ready(&preparing)).unwrap();
        run(bootstrap.validate_ready(&ready)).unwrap();
        backend.0.lock().unwrap().objects.remove("rmds.inoalloc");
        assert_eq!(
            run(bootstrap.validate_ready(&ready)),
            Err(StoreError::MissingState)
        );
        assert_eq!(
            run(bootstrap.inspect_format()).unwrap().unwrap().phase,
            crate::bootstrap::FormatPhase::Ready
        );
    }

    #[test]
    fn unsupported_or_unencodable_body_never_receives_append_success() {
        let (backend, mut store) = fixture();
        let mut body = records()[0].to_vec();
        body[0] = 1;
        assert_eq!(
            run(store.append(owner(), 0, [4; 16], &[Bytes::from(body)])),
            Err(StoreError::UnsupportedVersion)
        );
        assert_eq!(
            run(store.append(
                owner(),
                0,
                [4; 16],
                &[Bytes::from_static(b"\x02\x02\0\0\0\0")]
            )),
            Err(StoreError::Corrupt)
        );
        assert_eq!(backend.0.lock().unwrap().writes, 0);
    }

    #[test]
    fn ownership_change_between_inspection_and_write_is_rejected_atomically() {
        let mut next = owner();
        next.shard_epoch += 1;
        next.incarnation += 1;
        next.gid += 1;
        let (backend, mut store) = fixture();
        backend.0.lock().unwrap().depose_on_write = Some(next);
        assert_eq!(
            run(store.append(owner(), 0, [4; 16], &records())),
            Err(StoreError::Fenced)
        );
        assert!(backend.0.lock().unwrap().objects[OBJECT].values.is_empty());
        for during_trim in [false, true] {
            let (backend, mut store) = fixture();
            run(store.append(owner(), 0, [4; 16], &records())).unwrap();
            let descriptor =
                run(store.write_checkpoint_bytes(owner(), 1, 2, &Bytes::from_static(b"state")))
                    .unwrap();
            if during_trim {
                run(store.publish_checkpoint(owner(), 0, &descriptor)).unwrap();
            }
            backend.0.lock().unwrap().depose_on_write = Some(next);
            let result = if during_trim {
                run(store.trim(owner(), &descriptor, &[1, 2]))
            } else {
                run(store.publish_checkpoint(owner(), 0, &descriptor))
            };
            assert_eq!(result, Err(StoreError::GuardRejected));
            let control = run(store.inspect_frozen()).unwrap();
            assert_eq!(control.checkpoint.is_some(), during_trim);
            assert_eq!(backend.0.lock().unwrap().objects[OBJECT].values.len(), 2);
        }
    }

    #[test]
    fn rank_session_row_cannot_substitute_another_valid_identity() {
        let (backend, mut repository, acquired) = session_fixture();
        let (first, revision) =
            run(repository.open_session_record(&acquired, 1, new_session(10))).unwrap();
        let (second, revision) =
            run(repository.open_session_record(&acquired, revision, new_session(11))).unwrap();
        let first_key = Bytes::from(format!(
            "s/{:016x}/{:016x}",
            first.client, first.incarnation
        ));
        let second_key = Bytes::from(format!(
            "s/{:016x}/{:016x}",
            second.client, second.incarnation
        ));
        let mut memory = backend.0.lock().unwrap();
        let object = memory.objects.get_mut("rmds.sessions.0").unwrap();
        let substituted = object.values[&second_key].clone();
        object.values.insert(first_key, substituted);
        let writes = memory.writes;
        drop(memory);
        assert_eq!(
            run(repository.mark_session_closing(&acquired, revision, first)),
            Err(StoreError::Corrupt)
        );
        assert_eq!(backend.0.lock().unwrap().writes, writes);
    }

    #[test]
    fn batch_identity_rejects_different_same_length_payload_after_completion() {
        let (_, mut store) = fixture();
        let mut bodies = records();
        assert_eq!(bodies[0].len(), bodies[1].len());
        run(store.append(owner(), 0, [4; 16], &bodies)).unwrap();
        bodies.reverse();
        assert_eq!(
            run(store.append(owner(), 0, [4; 16], &bodies)),
            Err(StoreError::Corrupt)
        );
    }

    #[test]
    fn descriptor_checksum_binds_content_instead_of_crc_residue() {
        let first = framing::encode(
            FrameKind::Checkpoint,
            &identity(),
            owner(),
            1,
            [0; 16],
            b"one",
            1024,
        )
        .unwrap();
        let second = framing::encode(
            FrameKind::Checkpoint,
            &identity(),
            owner(),
            1,
            [0; 16],
            b"two",
            1024,
        )
        .unwrap();
        assert_eq!(first.len(), second.len());
        assert_ne!(
            framing::content_checksum(&first).unwrap(),
            framing::content_checksum(&second).unwrap()
        );
    }

    #[test]
    fn reverse_partial_acquisition_fences_both_actual_writers_without_reset() {
        let (backend, mut repository, acquired) = session_fixture();
        let (key, _) = run(repository.open_session_record(&acquired, 1, new_session(10))).unwrap();
        let mut middle = owner();
        middle.incarnation += 1;
        middle.gid += 1;
        let expected_sessions = run(repository.inspect_sessions()).unwrap();
        run(repository.acquire_sessions(
            &expected_sessions,
            rank_owner(middle),
            writer(middle),
            FencedWriter {
                writer: expected_sessions.writer.clone(),
                verified_osd_epoch: 55,
            },
        ))
        .unwrap();
        let store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
        let expected_shard = run(store.inspect_control()).unwrap();
        assert_eq!(expected_shard.owner, owner());
        let expected_sessions = run(repository.inspect_sessions()).unwrap();
        assert_eq!(expected_sessions.owner, rank_owner(middle));
        let mut replacement = middle;
        replacement.shard_epoch += 1;
        replacement.incarnation += 1;
        replacement.gid += 1;
        run(store.open(
            &expected_shard,
            replacement,
            writer(replacement),
            FencedWriter {
                writer: expected_shard.writer.clone(),
                verified_osd_epoch: 60,
            },
        ))
        .unwrap();
        let acquired = run(repository.acquire_sessions(
            &expected_sessions,
            rank_owner(replacement),
            writer(replacement),
            FencedWriter {
                writer: expected_sessions.writer.clone(),
                verified_osd_epoch: 61,
            },
        ))
        .unwrap();
        let state = run(repository.load_sessions(&acquired)).unwrap();
        assert_eq!(state.records[0].key, key);
        assert_eq!(state.control.next_incarnation, 2);
    }

    #[test]
    fn uncertain_format_create_and_acquire_reconcile_exact_plan_and_writer() {
        let backend = TestBackend::default();
        let bootstrap = crate::bootstrap::BootstrapStore::new(backend.clone(), identity());
        let plan = bootstrap_plan();
        backend.0.lock().unwrap().lose_reply = true;
        let preparing = run(bootstrap.begin_bootstrap(plan.clone(), writer(owner()))).unwrap();
        assert_eq!(
            run(bootstrap.begin_bootstrap(plan, writer(owner()))).unwrap(),
            preparing
        );
        let mut next = owner();
        next.incarnation += 1;
        next.gid += 1;
        let evidence = FencedWriter {
            writer: preparing.writer.clone(),
            verified_osd_epoch: 55,
        };
        backend.0.lock().unwrap().lose_reply = true;
        let acquired =
            run(bootstrap.acquire_preparing(&preparing, writer(next), evidence.clone())).unwrap();
        assert_eq!(acquired.plan, preparing.plan);
        assert_eq!(acquired.predecessor, Some(evidence.clone()));
        assert_eq!(
            run(bootstrap.acquire_preparing(&preparing, writer(next), evidence)).unwrap(),
            acquired
        );
    }

    fn recover_short_native_pages(byte_limited: bool) {
        let (backend, mut store) = fixture();
        let bodies = records();
        run(store.append(owner(), 0, [4; 16], &bodies)).unwrap();
        let opened = run(store.inspect_frozen()).unwrap();
        let mut memory = backend.0.lock().unwrap();
        if byte_limited {
            let one_value_bytes = memory.objects[OBJECT].values[&key(1)].len();
            memory.omap_byte_cap = Some(8 + key(1).len() + one_value_bytes);
        } else {
            memory.omap_entry_cap = Some(1);
        }
        drop(memory);
        assert_eq!(
            run(store.load_tail(&opened)),
            Ok(vec![(1, bodies[0].clone()), (2, bodies[1].clone())]),
            "byte_limited={byte_limited}"
        );
        assert_eq!(
            backend.0.lock().unwrap().omap_pages,
            vec![(key(0), vec![key(1)], true), (key(1), vec![key(2)], false)]
        );
    }

    #[test]
    fn wal_recovery_accepts_count_short_native_pages() {
        recover_short_native_pages(false);
    }

    #[test]
    fn wal_recovery_accepts_byte_short_native_pages() {
        recover_short_native_pages(true);
    }

    #[test]
    fn wal_short_pages_reject_missing_required_keys_and_owner_change() {
        for missing_middle in [false, true] {
            let (backend, mut store) = fixture();
            let mut bodies = records();
            if missing_middle {
                bodies.push(bodies[0].clone());
            }
            run(store.append(owner(), 0, [4; 16], &bodies)).unwrap();
            let opened = run(store.inspect_frozen()).unwrap();
            let mut memory = backend.0.lock().unwrap();
            memory.omap_entry_cap = Some(1);
            memory
                .objects
                .get_mut(OBJECT)
                .unwrap()
                .values
                .remove(&key(2));
            drop(memory);
            assert_eq!(run(store.load_tail(&opened)), Err(StoreError::Corrupt));
        }
        let (backend, mut store) = fixture();
        run(store.append(owner(), 0, [4; 16], &records())).unwrap();
        let opened = run(store.inspect_frozen()).unwrap();
        let mut next = owner();
        next.shard_epoch += 1;
        next.incarnation += 1;
        next.gid += 1;
        let mut memory = backend.0.lock().unwrap();
        memory.omap_entry_cap = Some(1);
        memory.depose_after_omap = Some(next);
        drop(memory);
        assert_eq!(
            run(store.load_tail(&opened)),
            Err(StoreError::GuardRejected)
        );
        assert_eq!(backend.0.lock().unwrap().omap_pages.len(), 1);
    }

    #[test]
    fn wal_pages_reject_nonprogress_and_contradictory_continuation() {
        for fault in 0..4 {
            let (backend, mut store) = fixture();
            run(store.append(owner(), 0, [4; 16], &records())).unwrap();
            let opened = run(store.inspect_frozen()).unwrap();
            let mut memory = backend.0.lock().unwrap();
            match fault {
                0 => memory.omap_entry_cap = Some(0),
                1 => {
                    memory.omap_entry_cap = Some(1);
                    memory.omap_more = Some(false);
                }
                2 => memory.omap_more = Some(true),
                3 => {
                    memory.omap_entry_cap = Some(1);
                    memory.omap_repeat_first = true;
                }
                _ => unreachable!(),
            }
            drop(memory);
            assert_eq!(
                run(store.load_tail(&opened)),
                Err(StoreError::Corrupt),
                "fault={fault}"
            );
            assert!(backend.0.lock().unwrap().omap_pages.len() <= 2);
        }
    }

    #[test]
    fn wal_short_pages_preserve_total_key_byte_and_value_limits() {
        let (backend, mut store) = fixture();
        run(store.append(owner(), 0, [4; 16], &records())).unwrap();
        let opened = run(store.inspect_frozen()).unwrap();
        let value_bytes = backend.0.lock().unwrap().objects[OBJECT].values[&key(1)].len();
        backend.0.lock().unwrap().omap_entry_cap = Some(1);
        for limits in [
            Limits {
                max_wal_keys: 1,
                ..Limits::default()
            },
            Limits {
                max_record_bytes: value_bytes,
                max_wal_bytes: value_bytes,
                ..Limits::default()
            },
            Limits {
                max_record_bytes: value_bytes - 1,
                ..Limits::default()
            },
        ] {
            let bounded = StableStore::new(backend.clone(), identity(), limits).unwrap();
            assert_eq!(run(bounded.load_tail(&opened)), Err(StoreError::Capacity));
        }
    }

    fn list_page(
        expected: Option<usize>,
        next: Option<usize>,
        epoch: u32,
        objects: Vec<osdc::PoolObject>,
    ) -> ListStep {
        ListStep {
            expected,
            result: Ok(crate::backend::PoolPage {
                objects,
                next,
                osdmap_epoch: epoch,
            }),
        }
    }

    fn scripted_listing(steps: Vec<ListStep>) -> TestBackend {
        let backend = TestBackend::default();
        backend.0.lock().unwrap().listing_script = Some(steps.into());
        backend
    }

    #[test]
    fn bootstrap_completes_paginated_empty_pgs_before_format_creation() {
        let backend = scripted_listing(vec![
            list_page(None, Some(1), 1, vec![]),
            list_page(Some(1), Some(2), 1, vec![]),
            list_page(Some(2), None, 1, vec![]),
        ]);
        let bootstrap = crate::bootstrap::BootstrapStore::new(backend.clone(), identity());
        run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
        let memory = backend.0.lock().unwrap();
        assert_eq!(
            memory.listing_calls,
            vec![(None, 0), (Some(1), 0), (Some(2), 0)]
        );
        assert!(memory.listing_script.as_ref().unwrap().is_empty());
        assert_eq!(memory.writes, 1);
        assert_eq!(
            memory
                .objects
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["rmds.format"]
        );
    }

    #[test]
    fn bootstrap_late_foreign_namespace_after_map_restart_never_writes_format() {
        for restart in [false, true] {
            let foreign = osdc::PoolObject {
                namespace: b"other-filesystem".to_vec(),
                name: b"rmds.format".to_vec(),
                locator: vec![],
            };
            let mut steps = vec![list_page(None, Some(1), 1, vec![])];
            let mut expected_calls = vec![(None, 0)];
            if restart {
                steps.push(ListStep {
                    expected: Some(1),
                    result: Err(osdc::PoolListError::MapChanged),
                });
                steps.push(list_page(None, Some(7), 2, vec![]));
                steps.push(list_page(Some(7), Some(8), 2, vec![]));
                steps.push(list_page(Some(8), None, 2, vec![foreign]));
                expected_calls.extend([(Some(1), 0), (None, 0), (Some(7), 0), (Some(8), 0)]);
            } else {
                steps.push(list_page(Some(1), Some(2), 1, vec![]));
                steps.push(list_page(Some(2), None, 1, vec![foreign]));
                expected_calls.extend([(Some(1), 0), (Some(2), 0)]);
            }
            let backend = scripted_listing(steps);
            let bootstrap = crate::bootstrap::BootstrapStore::new(backend.clone(), identity());
            assert!(matches!(
                run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))),
                Err(StoreError::WrongFormat)
            ));
            let memory = backend.0.lock().unwrap();
            assert_eq!(memory.listing_calls, expected_calls);
            assert!(memory.listing_script.as_ref().unwrap().is_empty());
            assert_eq!(memory.writes, 0);
            assert!(memory.objects.is_empty());
        }
    }

    #[test]
    fn bootstrap_repeated_listing_cursor_is_not_scan_completion() {
        let backend = scripted_listing(vec![
            list_page(None, Some(1), 1, vec![]),
            list_page(Some(1), Some(1), 1, vec![]),
        ]);
        let bootstrap = crate::bootstrap::BootstrapStore::new(backend.clone(), identity());
        assert!(matches!(
            run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))),
            Err(StoreError::Corrupt)
        ));
        let memory = backend.0.lock().unwrap();
        assert_eq!(memory.listing_calls, vec![(None, 0), (Some(1), 0)]);
        assert_eq!(memory.writes, 0);
        assert!(memory.objects.is_empty());
    }

    #[test]
    fn bootstrap_map_change_restarts_the_complete_successful_scan_before_writing() {
        let backend = scripted_listing(vec![
            list_page(None, Some(1), 1, vec![]),
            ListStep {
                expected: Some(1),
                result: Err(osdc::PoolListError::MapChanged),
            },
            list_page(None, Some(7), 2, vec![]),
            list_page(Some(7), Some(8), 2, vec![]),
            list_page(Some(8), None, 2, vec![]),
        ]);
        let bootstrap = crate::bootstrap::BootstrapStore::new(backend.clone(), identity());
        run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
        let memory = backend.0.lock().unwrap();
        assert_eq!(
            memory.listing_calls,
            vec![
                (None, 0),
                (Some(1), 0),
                (None, 0),
                (Some(7), 0),
                (Some(8), 0)
            ]
        );
        assert!(memory.listing_script.as_ref().unwrap().is_empty());
        assert_eq!(memory.writes, 1);
        assert_eq!(
            memory
                .objects
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["rmds.format"]
        );
    }

    #[test]
    fn caller_cancellation_after_durable_append_retains_the_uncertain_batch() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct PauseAfterWrite {
            backend: TestBackend,
            pause: AtomicBool,
        }
        impl Backend for PauseAfterWrite {
            async fn operate(
                &self,
                name: &str,
                operation: ObjectOperation,
            ) -> Result<OpReply, StoreError> {
                let writing = operation.has_write();
                let reply = self.backend.operate(name, operation).await?;
                // This flag publishes no data. Objects remain protected by the
                // backend mutex; relaxed ordering only selects the fault point.
                if writing && self.pause.swap(false, Ordering::Relaxed) {
                    std::future::pending::<()>().await;
                }
                Ok(reply)
            }
        }
        let (backend, _) = fixture();
        let paused = PauseAfterWrite {
            backend: backend.clone(),
            pause: AtomicBool::new(true),
        };
        let mut store = StableStore::new(paused, identity(), Limits::default()).unwrap();
        let bodies = records();
        {
            let append = store.append(owner(), 0, [4; 16], &bodies);
            let mut append = pin!(append);
            let mut context = Context::from_waker(Waker::noop());
            assert!(matches!(append.as_mut().poll(&mut context), Poll::Pending));
            // A caller deadline drops this future. No completion was delivered.
        }
        assert_eq!(
            run(store.append(owner(), 2, [5; 16], &bodies)),
            Err(StoreError::UnknownOutcome)
        );
        assert_eq!(run(store.reconcile_pending()), Ok(2));
        assert_eq!(backend.0.lock().unwrap().writes, 1);
    }
}
