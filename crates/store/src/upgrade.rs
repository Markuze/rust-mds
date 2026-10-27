//! Checked, output-free Legacy2 -> RetainedTargets3 adoption. This coordinates
//! the existing three independent object authorities; it is not cross-object CAS.

use crate::{
    backend::{Backend, PoolListing, PoolPage, checked},
    bootstrap::{BootstrapStore, FormatInspection, FormatPhase, format_authority},
    sessions::{
        AcquiredSessions, RankSessionState, SessionLifecycle, SessionRepository, authority,
    },
    stable::{
        CheckpointDescriptor, FrozenShard, OpenedShard, StableStore, StoreError, StoreIdentity,
        WriterIdentity, guards,
    },
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use engine::{
    Engine, EngineConfig,
    durability::{EngineCheckpoint, RecoverySemantics, RestoreError},
    journal::MemJournal,
};
use osdc::{ObjectOperation, ops::cmp_op};
use std::collections::BTreeMap;

const FORMAT: &str = "rmds.format";
const WAL: &str = "rmds.wal.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryUpgradePlan {
    pub id: [u8; 16],
    pub from: RecoverySemantics,
    pub to: RecoverySemantics,
    pub frozen_tail: u64,
    pub prepared_by: WriterIdentity,
    pub source: CheckpointDescriptor,
    pub target: CheckpointDescriptor,
}

impl RetryUpgradePlan {
    pub(crate) fn validate(&self, _: &StoreIdentity) -> Result<(), StoreError> {
        if self.from != RecoverySemantics::Legacy2
            || self.to != RecoverySemantics::RetainedTargets3
            || self.source.generation.checked_add(1) != Some(self.target.generation)
            || self.source.covered_seq > self.frozen_tail
            || self.target.covered_seq != self.frozen_tail
            || self.prepared_by.rank_owner.rank != 0
        {
            return Err(StoreError::Corrupt);
        }
        Ok(())
    }
    pub(crate) fn encode(&self) -> Result<Bytes, StoreError> {
        if self.from != RecoverySemantics::Legacy2 || self.to != RecoverySemantics::RetainedTargets3
        {
            return Err(StoreError::UnsupportedVersion);
        }
        let mut b = BytesMut::new();
        b.put_u8(1);
        b.put_u16_le(2);
        b.put_u16_le(3);
        b.extend_from_slice(&self.id);
        b.put_u64_le(self.frozen_tail);
        crate::stable::put_writer(&mut b, &self.prepared_by)?;
        put_blob(&mut b, &self.source.encode()?);
        put_blob(&mut b, &self.target.encode()?);
        b.put_u32_le(crc32c::crc32c(&b));
        if b.len() > 8192 {
            return Err(StoreError::Capacity);
        }
        Ok(b.freeze())
    }
    pub(crate) fn decode(bytes: &Bytes) -> Result<Self, StoreError> {
        if bytes.len() > 8192 {
            return Err(StoreError::Capacity);
        }
        if bytes.len() < 33 {
            return Err(StoreError::Corrupt);
        }
        let end = bytes.len() - 4;
        let mut crc = bytes.slice(end..);
        if crc32c::crc32c(&bytes[..end]) != crc.get_u32_le() {
            return Err(StoreError::Corrupt);
        }
        let mut b = bytes.slice(..end);
        if b.get_u8() != 1 || b.get_u16_le() != 2 || b.get_u16_le() != 3 {
            return Err(StoreError::UnsupportedVersion);
        }
        let mut id = [0; 16];
        b.copy_to_slice(&mut id);
        let frozen_tail = b.get_u64_le();
        let prepared_by = crate::stable::get_writer(&mut b)?;
        let source = CheckpointDescriptor::decode(&blob(&mut b)?)?.ok_or(StoreError::Corrupt)?;
        let target = CheckpointDescriptor::decode(&blob(&mut b)?)?.ok_or(StoreError::Corrupt)?;
        if !b.is_empty() {
            return Err(StoreError::Corrupt);
        }
        Ok(Self {
            id,
            from: RecoverySemantics::Legacy2,
            to: RecoverySemantics::RetainedTargets3,
            frozen_tail,
            prepared_by,
            source,
            target,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RetryUpgradeReady {
    pub format: FormatInspection,
    pub opened: OpenedShard,
    pub sessions: AcquiredSessions,
}

/// This small adapter checks the live watch around every backend await, including
/// all nested reads, listing pages and reconciliation. It changes no OSD payload.
struct Watched<'a, B, F> {
    backend: &'a B,
    check: &'a F,
}
impl<B: Backend + Sync, F: Fn() -> Result<(), StoreError> + Send + Sync> Backend
    for Watched<'_, B, F>
{
    async fn operate(&self, name: &str, op: ObjectOperation) -> Result<osdc::OpReply, StoreError> {
        (self.check)()?;
        let result = self.backend.operate(name, op).await;
        (self.check)()?;
        result
    }
}
impl<B: PoolListing + Sync, F: Fn() -> Result<(), StoreError> + Send + Sync> PoolListing
    for Watched<'_, B, F>
{
    type Cursor = B::Cursor;
    async fn list_pool_page(
        &self,
        cursor: Option<Self::Cursor>,
        count: u32,
        bytes: usize,
    ) -> Result<PoolPage<Self::Cursor>, osdc::PoolListError> {
        (self.check)().map_err(|_| osdc::PoolListError::Io(osdc::OpError::Blocklisted))?;
        let result = self.backend.list_pool_page(cursor, count, bytes).await;
        (self.check)().map_err(|_| osdc::PoolListError::Io(osdc::OpError::Blocklisted))?;
        result
    }
}

impl<B: Backend + PoolListing + Sync> BootstrapStore<B> {
    /// No service result follows this method. The caller reloads the refreshed
    /// handles and performs the normal engine/recovery/monitor gates afterward.
    #[allow(clippy::too_many_arguments)]
    pub async fn upgrade_retry_targets(
        &self,
        expected: &FormatInspection,
        shard: &mut StableStore<B>,
        repository: &mut SessionRepository<B>,
        opened: &OpenedShard,
        sessions: &AcquiredSessions,
        engine_config: &EngineConfig,
        upgrade_id: [u8; 16],
        check_authority: &(impl Fn() -> Result<(), StoreError> + Send + Sync),
    ) -> Result<RetryUpgradeReady, StoreError> {
        check_authority()?;
        expected.validate_snapshot(&self.identity)?;
        shard.no_pending()?;
        repository.no_pending()?;
        if self.identity != shard.identity || self.identity != repository.identity {
            return Err(StoreError::WrongFormat);
        }
        let format = BootstrapStore::new(
            Watched {
                backend: &self.backend,
                check: check_authority,
            },
            self.identity.clone(),
        );
        let store = StableStore::new(
            Watched {
                backend: &shard.backend,
                check: check_authority,
            },
            self.identity.clone(),
            shard.limits.clone(),
        )?;
        let rank = SessionRepository::new(
            Watched {
                backend: &repository.backend,
                check: check_authority,
            },
            self.identity.clone(),
            repository.rank,
            repository.max_records,
        )?;
        let mut actual = format
            .inspect_format()
            .await?
            .ok_or(StoreError::MissingState)?;
        let mut control = store.inspect_frozen().await?;
        let mut rows = rank.load_sessions(sessions).await?;
        if opened.owner != opened.frozen.owner
            || opened.tail != opened.frozen.tail
            || opened.checkpoint != opened.frozen.checkpoint
        {
            return Err(StoreError::Corrupt);
        }
        if actual.plan != expected.plan
            || actual.writer != expected.writer
            || actual.writer != control.authority.writer
            || actual.writer != rows.control.writer
            || control.owner != opened.owner
            || opened.required_semantics != opened.frozen.authority.required_semantics
            || control.tail != opened.tail
            || rows.control.owner != sessions.control.owner
        {
            return Err(StoreError::Fenced);
        }
        // Same-attempt reconciliation may observe its persisted later phase.
        if actual != *expected && actual.retry_upgrade.as_ref().map(|p| p.id) != Some(upgrade_id) {
            return Err(StoreError::Fenced);
        }
        // Only this exact attempt's one-step semantic promotion may refresh a
        // supplied snapshot without a new acquisition. Never accept unrelated
        // session mutations or a different control generation under the same gid.
        if rows.control != sessions.control {
            let mut promoted = sessions.control.clone();
            promoted.required_semantics = RecoverySemantics::RetainedTargets3;
            promoted.revision = promoted
                .revision
                .checked_add(1)
                .ok_or(StoreError::Capacity)?;
            if actual.retry_upgrade.as_ref().map(|p| p.id) != Some(upgrade_id)
                || rows.control != promoted
            {
                return Err(StoreError::Fenced);
            }
        }
        if control != opened.frozen {
            let intent = actual
                .retry_upgrade
                .as_ref()
                .filter(|p| p.id == upgrade_id)
                .ok_or(StoreError::Fenced)?;
            let mut promoted = opened.frozen.clone();
            if promoted.authority.required_semantics == RecoverySemantics::Legacy2 {
                promoted.authority.required_semantics = RecoverySemantics::RetainedTargets3;
                promoted.authority.revision = promoted
                    .authority
                    .revision
                    .checked_add(1)
                    .ok_or(StoreError::Capacity)?;
            }
            promoted.checkpoint = Some(intent.target.clone());
            // Trim alone may advance between cancelled attempts.
            promoted.trimmed = control.trimmed;
            if control != promoted || control.trimmed < opened.frozen.trimmed {
                return Err(StoreError::Fenced);
            }
        }
        if actual.phase == FormatPhase::Preparing {
            return Err(StoreError::UpgradeRequired);
        }
        if actual.phase == FormatPhase::Ready
            && actual.required_semantics == RecoverySemantics::RetainedTargets3
        {
            if control.authority.required_semantics != RecoverySemantics::RetainedTargets3
                || rows.control.required_semantics != RecoverySemantics::RetainedTargets3
            {
                return Err(StoreError::Corrupt);
            }
            format.validate_ready(&actual).await?;
            check_authority()?;
            return Ok(RetryUpgradeReady {
                format: actual,
                opened: OpenedShard::from_frozen(control),
                sessions: AcquiredSessions {
                    control: rows.control,
                },
            });
        }
        if actual.retry_upgrade.is_none() {
            if !expected.acquired_from_ready() {
                return Err(StoreError::UpgradeRequired);
            }
            if control.authority.required_semantics != RecoverySemantics::Legacy2
                || rows.control.required_semantics != RecoverySemantics::Legacy2
            {
                return Err(StoreError::Corrupt);
            }
            // No same-start fresh exception. Every live authority needs durable
            // prior-writer evidence from a higher-incarnation, different-GID CAS.
            for a in [
                format_authority(&actual),
                control.authority.clone(),
                authority(&rows.control),
            ] {
                let prior = a.predecessor.as_ref().ok_or(StoreError::UpgradeRequired)?;
                if a.required_semantics != RecoverySemantics::Legacy2
                    || a.writer.rank_owner.incarnation <= prior.writer.rank_owner.incarnation
                    || a.writer.rank_owner.gid == prior.writer.rank_owner.gid
                {
                    return Err(StoreError::UpgradeRequired);
                }
            }
            if actual.phase != FormatPhase::Ready {
                return Err(StoreError::Corrupt);
            }
            format.validate_ready(&actual).await?;
            let candidate = clean_source(&store, &control, &rows, engine_config).await?;
            let source = control.checkpoint.clone().ok_or(StoreError::MissingState)?;
            let target = store
                .write_checkpoint(
                    control.owner,
                    source
                        .generation
                        .checked_add(1)
                        .ok_or(StoreError::Capacity)?,
                    &candidate,
                )
                .await?;
            let verified = read_candidate(&store, &target, engine_config).await?;
            if candidate != verified {
                return Err(StoreError::Corrupt);
            }
            let plan = RetryUpgradePlan {
                id: upgrade_id,
                from: RecoverySemantics::Legacy2,
                to: RecoverySemantics::RetainedTargets3,
                frozen_tail: control.tail,
                prepared_by: actual.writer.clone(),
                source,
                target,
            };
            plan.validate(&self.identity)?;
            // Recheck all frozen controls after candidate I/O, before barrier.
            if format.inspect_format().await?.as_ref() != Some(&actual)
                || store.inspect_frozen().await? != control
                || rank.inspect_sessions().await? != rows.control
            {
                return Err(StoreError::Fenced);
            }
            let mut desired = actual.clone();
            desired.required_semantics = RecoverySemantics::RetainedTargets3;
            desired.phase = FormatPhase::UpgradingRetryTargets;
            desired.retry_upgrade = Some(plan);
            desired.revision = desired
                .revision
                .checked_add(1)
                .ok_or(StoreError::Capacity)?;
            actual = format.transition(&actual, &desired).await?;
        }
        let plan = actual
            .retry_upgrade
            .clone()
            .ok_or(StoreError::MissingState)?;
        if plan.id != upgrade_id
            || actual.phase != FormatPhase::UpgradingRetryTargets
            || actual.required_semantics != RecoverySemantics::RetainedTargets3
            || control.tail != plan.frozen_tail
        {
            return Err(StoreError::Corrupt);
        }
        plan.validate(&self.identity)?;
        if actual.writer != plan.prepared_by
            && (actual.writer.rank_owner.incarnation <= plan.prepared_by.rank_owner.incarnation
                || actual.writer.rank_owner.gid == plan.prepared_by.rank_owner.gid)
        {
            return Err(StoreError::Fenced);
        }
        // Always verify the persisted producer's original names and bytes. Only
        // this private promotion path can publish a predecessor's candidate.
        let candidate = read_candidate(&store, &plan.target, engine_config).await?;
        allocator_provenance(&store, &candidate).await?;
        verify_producer(&store, &plan).await?;
        match control.authority.required_semantics {
            RecoverySemantics::Legacy2 => {
                if control.checkpoint.as_ref() != Some(&plan.source) {
                    return Err(StoreError::Corrupt);
                }
                let source_candidate = clean_source(&store, &control, &rows, engine_config).await?;
                if source_candidate != candidate {
                    return Err(StoreError::Corrupt);
                }
            }
            RecoverySemantics::RetainedTargets3 => {
                if rows.control.required_semantics != RecoverySemantics::RetainedTargets3
                    || control.checkpoint.as_ref() != Some(&plan.target)
                {
                    return Err(StoreError::Corrupt);
                }
                clean_join(&candidate, &rows)?;
            }
        }
        rows = rank
            .load_sessions(&AcquiredSessions {
                control: rows.control.clone(),
            })
            .await?;
        clean_join(&candidate, &rows)?;
        if rows.control.required_semantics == RecoverySemantics::Legacy2 {
            let mut next = rows.control.clone();
            next.required_semantics = RecoverySemantics::RetainedTargets3;
            next.revision = next.revision.checked_add(1).ok_or(StoreError::Capacity)?;
            let mut op = rank.guarded(&rows.control)?;
            op.setxattr("rmds.authority", authority(&next).encode()?);
            let result = checked(&rank.backend, "rmds.sessions.0", op).await;
            let now = rank.inspect_sessions().await?;
            if now != next {
                return Err(if now == rows.control {
                    result.err().unwrap_or(StoreError::UnknownOutcome)
                } else {
                    StoreError::Fenced
                });
            }
            rows.control = now;
        }
        if control.authority.required_semantics == RecoverySemantics::Legacy2 {
            verify_producer(&store, &plan).await?;
            let mut next = control.clone();
            next.authority.required_semantics = RecoverySemantics::RetainedTargets3;
            next.authority.revision = next
                .authority
                .revision
                .checked_add(1)
                .ok_or(StoreError::Capacity)?;
            next.checkpoint = Some(plan.target.clone());
            let mut op = wal_snapshot_guard(&self.identity, &control)?;
            op.setxattr("rmds.authority", next.authority.encode()?)
                .setxattr("rmds.checkpoint", plan.target.encode()?);
            let result = checked(&store.backend, WAL, op).await;
            let now = store.inspect_frozen().await?;
            if now != next {
                return Err(if now == control {
                    result.err().unwrap_or(StoreError::UnknownOutcome)
                } else {
                    StoreError::Fenced
                });
            }
            control = now;
        }
        while control.trimmed < plan.frozen_tail {
            let end = plan.frozen_tail.min(
                control
                    .trimmed
                    .saturating_add(shard.limits.page_keys as u64),
            );
            let keys: Vec<_> = (control.trimmed + 1..=end).collect();
            let result = store.trim(control.owner, &plan.target, &keys).await;
            let now = store.inspect_frozen().await?;
            if now.authority != control.authority
                || now.owner != control.owner
                || now.tail != control.tail
                || now.checkpoint != control.checkpoint
                || now.last_batch != control.last_batch
            {
                return Err(StoreError::Fenced);
            }
            if now.trimmed != end {
                return Err(result.err().unwrap_or(StoreError::Corrupt));
            }
            control = now;
        }
        let final_rows = rank
            .load_sessions(&AcquiredSessions {
                control: rows.control.clone(),
            })
            .await?;
        let final_state = read_candidate(&store, &plan.target, engine_config).await?;
        clean_join(&final_state, &final_rows)?;
        if candidate != final_state
            || final_rows.control != rows.control
            || store.inspect_frozen().await? != control
            || format.inspect_format().await?.as_ref() != Some(&actual)
        {
            return Err(StoreError::Fenced);
        }
        let mut ready = actual.clone();
        ready.phase = FormatPhase::Ready;
        ready.revision = ready.revision.checked_add(1).ok_or(StoreError::Capacity)?;
        let ready = format.transition(&actual, &ready).await?;
        check_authority()?;
        Ok(RetryUpgradeReady {
            format: ready,
            opened: OpenedShard::from_frozen(control),
            sessions: AcquiredSessions {
                control: rows.control,
            },
        })
    }

    async fn transition(
        &self,
        expected: &FormatInspection,
        desired: &FormatInspection,
    ) -> Result<FormatInspection, StoreError> {
        let mut op = self.guard(expected)?;
        op.setxattr("rmds.authority", format_authority(desired).encode()?)
            .setxattr(
                "rmds.phase",
                Bytes::from_static(match desired.phase {
                    FormatPhase::Ready => b"Ready",
                    FormatPhase::UpgradingRetryTargets => b"UpgradingRetryTargets",
                    FormatPhase::Preparing => b"Preparing",
                }),
            );
        if let Some(intent) = &desired.retry_upgrade {
            op.setxattr("rmds.retry_upgrade", intent.encode()?);
        }
        let result = checked(&self.backend, FORMAT, op).await;
        let now = self
            .inspect_format()
            .await?
            .ok_or(StoreError::MissingState)?;
        if now == *desired {
            Ok(now)
        } else if now == *expected {
            Err(result.err().unwrap_or(StoreError::UnknownOutcome))
        } else {
            Err(StoreError::Fenced)
        }
    }
}

async fn read_candidate<B: Backend>(
    store: &StableStore<B>,
    descriptor: &CheckpointDescriptor,
    config: &EngineConfig,
) -> Result<EngineCheckpoint, StoreError> {
    let bytes = store.load_checkpoint_bytes(descriptor).await?;
    let state =
        crate::record::decode_checkpoint(&bytes).map_err(crate::record::store_decode_error)?;
    if state.recovery_semantics != RecoverySemantics::RetainedTargets3 {
        return Err(StoreError::UnsupportedVersion);
    }
    if state.covered_seq != descriptor.covered_seq {
        return Err(StoreError::Corrupt);
    }
    Engine::restore(
        config.clone(),
        MemJournal::at_seq(state.covered_seq, false),
        state.clone(),
    )
    .map_err(engine_error)?;
    Ok(state)
}

async fn verify_producer<B: Backend>(
    store: &StableStore<B>,
    plan: &RetryUpgradePlan,
) -> Result<(), StoreError> {
    let bytes = store.read_object(&plan.target.manifest_name, 4096).await?;
    let frame = crate::framing::decode(
        &bytes,
        crate::framing::FrameKind::Manifest,
        &store.identity,
        plan.frozen_tail,
        4096,
    )?;
    if frame.owner.rank != plan.prepared_by.rank_owner.rank
        || frame.owner.incarnation != plan.prepared_by.rank_owner.incarnation
        || frame.owner.gid != plan.prepared_by.rank_owner.gid
        || frame.transaction != [0; 16]
    {
        return Err(StoreError::Corrupt);
    }
    Ok(())
}

async fn clean_source<B: Backend>(
    store: &StableStore<B>,
    control: &FrozenShard,
    rows: &RankSessionState,
    config: &EngineConfig,
) -> Result<EngineCheckpoint, StoreError> {
    let descriptor = control
        .checkpoint
        .as_ref()
        .ok_or(StoreError::MissingState)?;
    let bytes = store.load_checkpoint_bytes(descriptor).await?;
    let state =
        crate::record::decode_checkpoint(&bytes).map_err(crate::record::store_decode_error)?;
    if state.recovery_semantics != RecoverySemantics::Legacy2 {
        return Err(StoreError::UnsupportedVersion);
    }
    if state.covered_seq != descriptor.covered_seq {
        return Err(StoreError::Corrupt);
    }
    let mut engine = Engine::restore(
        config.clone(),
        MemJournal::at_seq(state.covered_seq, false),
        state,
    )
    .map_err(engine_error)?;
    for (seq, bytes) in store.load_tail(control).await? {
        let record =
            crate::record::decode_record(&bytes).map_err(crate::record::store_decode_error)?;
        if record.recovery_semantics != RecoverySemantics::Legacy2 {
            return Err(StoreError::UnsupportedVersion);
        }
        for inode in &record.inodes {
            for client in inode.client_ranges.keys() {
                if engine.session(*client).is_some() {
                    continue;
                }
                if let Some(row) = rows
                    .records
                    .iter()
                    .filter(|r| {
                        r.key.client == *client
                            && engine.session_cleanup_seq(r.key).is_none()
                            && r.cleanup_seq.is_none_or(|end| end >= seq)
                    })
                    .min_by_key(|r| r.cleanup_seq.unwrap_or(u64::MAX))
                {
                    engine.install_session_key(row.key).map_err(engine_error)?;
                }
            }
        }
        engine.replay_record(seq, &record).map_err(engine_error)?;
    }
    *engine.journal_mut() = MemJournal::at_seq(control.tail, false);
    let source = engine.checkpoint().map_err(|_| StoreError::Corrupt)?;
    clean_join(&source, rows)?;
    let candidate = engine
        .checkpoint_for_retry_upgrade()
        .map_err(engine_error)?;
    allocator_provenance(store, &candidate).await?;
    let mut expected = source;
    expected.recovery_semantics = RecoverySemantics::RetainedTargets3;
    if candidate != expected {
        return Err(StoreError::Corrupt);
    }
    Ok(candidate)
}

fn clean_join(state: &EngineCheckpoint, rows: &RankSessionState) -> Result<(), StoreError> {
    if state
        .sessions
        .iter()
        .any(|s| !rows.records.iter().any(|r| r.key == s.key))
    {
        return Err(StoreError::Corrupt);
    }
    if rows
        .records
        .iter()
        .any(|r| r.lifecycle != SessionLifecycle::Closed)
        || !state.sessions.is_empty()
    {
        return Err(StoreError::UpgradeRequired);
    }
    let receipts: BTreeMap<_, _> = state.cleanup_receipts.iter().copied().collect();
    if receipts.len() != state.cleanup_receipts.len() || receipts.len() != rows.records.len() {
        return Err(StoreError::Corrupt);
    }
    for row in &rows.records {
        if row
            .cleanup_seq
            .is_none_or(|seq| seq > state.covered_seq || receipts.get(&row.key) != Some(&seq))
        {
            return Err(StoreError::Corrupt);
        }
    }
    Ok(())
}

fn engine_error(error: RestoreError) -> StoreError {
    match error {
        RestoreError::UpgradeRequired(_) => StoreError::UpgradeRequired,
        RestoreError::Capacity => StoreError::Capacity,
        _ => StoreError::Corrupt,
    }
}

async fn allocator_provenance<B: Backend>(
    store: &StableStore<B>,
    state: &EngineCheckpoint,
) -> Result<(), StoreError> {
    let mut op = ObjectOperation::new();
    op.getxattr("rmds.identity").getxattr("rmds.high_watermark");
    let reply = checked(&store.backend, "rmds.inoalloc", op).await?;
    if reply.out(0) != store.identity.encode() {
        return Err(StoreError::WrongFormat);
    }
    let high_bytes = reply.out(1);
    let high = std::str::from_utf8(&high_bytes)
        .map_err(|_| StoreError::Corrupt)?
        .parse::<u64>()
        .map_err(|_| StoreError::Corrupt)?;
    if high.to_string().as_bytes() != high_bytes.as_ref()
        || high > 1 << 48
        || state.ino_alloc.floor > high
    {
        return Err(StoreError::Corrupt);
    }
    let mut after = Bytes::new();
    let mut ranges = Vec::new();
    loop {
        let mut op = ObjectOperation::new();
        op.cmpxattr_str("rmds.identity", cmp_op::EQ, &store.identity.encode());
        op.cmpxattr_str("rmds.high_watermark", cmp_op::EQ, &high_bytes);
        op.omap_get_vals(&after, b"a/", 16);
        let reply = checked(&store.backend, "rmds.inoalloc", op).await?;
        let mut bytes = reply.out(2);
        if bytes.len() < 5 || bytes.len() > 16 * 128 + 5 {
            return Err(StoreError::Corrupt);
        }
        let count = bytes.get_u32_le();
        if count > 16 {
            return Err(StoreError::Corrupt);
        }
        for _ in 0..count {
            let key = blob(&mut bytes)?;
            let value = blob(&mut bytes)?;
            if key <= after || key.len() != 34 || !key.starts_with(b"a/") {
                return Err(StoreError::Corrupt);
            }
            let mut id = [0; 16];
            for (i, pair) in key[2..].as_chunks::<2>().0.iter().enumerate() {
                if pair
                    .iter()
                    .any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(b))
                {
                    return Err(StoreError::Corrupt);
                }
                id[i] = u8::from_str_radix(
                    std::str::from_utf8(pair).map_err(|_| StoreError::Corrupt)?,
                    16,
                )
                .map_err(|_| StoreError::Corrupt)?;
            }
            let range = crate::allocator::decode_reservation(&value, &store.identity, id)?;
            if range.start + range.len > high {
                return Err(StoreError::Corrupt);
            }
            ranges.push(range);
            after = key;
            if ranges.len() > 4096 {
                return Err(StoreError::Capacity);
            }
        }
        if bytes.len() != 1 || bytes[0] > 1 || (count == 0 && bytes[0] != 0) {
            return Err(StoreError::Corrupt);
        }
        if bytes[0] == 0 {
            break;
        }
    }
    ranges.sort_by_key(|r| r.start);
    if ranges
        .windows(2)
        .any(|r| r[0].start + r[0].len > r[1].start)
    {
        return Err(StoreError::Corrupt);
    }
    for range in state
        .ino_alloc
        .free
        .iter()
        .chain(state.ino_alloc.delegated.iter().map(|(_, r)| r))
    {
        if !ranges.iter().any(|r| {
            r.start <= range.start
                && range
                    .start
                    .checked_add(range.len)
                    .is_some_and(|end| end <= r.start + r.len)
        }) {
            return Err(StoreError::Corrupt);
        }
    }
    for ino in state
        .inodes
        .iter()
        .map(|i| i.core.ino)
        .chain(state.jobs.iter().map(|j| j.ino))
        .filter(|ino| *ino != 1)
    {
        if !ranges
            .iter()
            .any(|r| r.start <= ino && ino < r.start + r.len)
        {
            return Err(StoreError::Corrupt);
        }
    }
    Ok(())
}
fn wal_snapshot_guard(
    identity: &StoreIdentity,
    c: &FrozenShard,
) -> Result<ObjectOperation, StoreError> {
    let mut op = ObjectOperation::new();
    guards(&mut op, identity, c.owner, &c.authority)?;
    op.cmpxattr_str("rmds.tail", cmp_op::EQ, c.tail.to_string().as_bytes());
    op.cmpxattr_str("rmds.trimmed", cmp_op::EQ, c.trimmed.to_string().as_bytes());
    op.cmpxattr_str("rmds.batch", cmp_op::EQ, &c.last_batch);
    op.cmpxattr_str(
        "rmds.checkpoint",
        cmp_op::EQ,
        &c.checkpoint
            .as_ref()
            .ok_or(StoreError::MissingState)?
            .encode()?,
    );
    Ok(op)
}
fn put_blob(b: &mut BytesMut, bytes: &[u8]) {
    b.put_u32_le(bytes.len() as u32);
    b.extend_from_slice(bytes);
}
fn blob(b: &mut Bytes) -> Result<Bytes, StoreError> {
    if b.len() < 4 {
        return Err(StoreError::Corrupt);
    }
    let n = b.get_u32_le() as usize;
    if n > 4096 || n > b.len() {
        return Err(StoreError::Corrupt);
    }
    Ok(b.split_to(n))
}
