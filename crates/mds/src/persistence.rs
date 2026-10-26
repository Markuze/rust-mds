//! Native root-shard bootstrap, separate authority acquisition and serialized store worker.
//! Storage awaits run here, never in the daemon event dispatcher.

use denc::{Bytes, Decode};
use engine::{Engine, EngineCheckpoint, EngineConfig, InoRange, JournalRecord, SessionKey};
use monc::{MdsMap, MdsState};
use std::{sync::Arc, time::Duration};
use store::{
    AcquiredSessions, Backend, BootstrapPlan, BootstrapStore, CheckpointDescriptor, FencedWriter,
    FormatPhase, Limits, NewSessionRecord, OwnerToken, PoolListing, RankOwner, RankSessionState,
    SessionLifecycle, SessionRecord, SessionRepository, StableStore, StoreError, StoreIdentity,
    WriterIdentity,
};
use tokio::sync::{mpsc, watch};

use crate::{
    fencing::{self, FenceBackend, Membership},
    journal::SeedJournal,
};

pub const OP_TIMEOUT: Duration = Duration::from_secs(30);
pub const SESSION_LIMIT: usize = 4096;
const RETRIES: usize = 8;

pub struct SharedBackend<B>(Arc<B>);
impl<B> Clone for SharedBackend<B> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<B> SharedBackend<B> {
    pub fn new(backend: B) -> Self {
        Self(Arc::new(backend))
    }
}
impl<B: Backend + Sync + Send> Backend for SharedBackend<B> {
    async fn operate(
        &self,
        object: &str,
        op: osdc::ObjectOperation,
    ) -> Result<osdc::OpReply, StoreError> {
        self.0.operate(object, op).await
    }
}
impl<B: PoolListing + Sync + Send> PoolListing for SharedBackend<B> {
    type Cursor = B::Cursor;
    async fn list_pool_page(
        &self,
        cursor: Option<Self::Cursor>,
        count: u32,
        bytes: usize,
    ) -> Result<store::PoolPage<Self::Cursor>, osdc::PoolListError> {
        self.0.list_pool_page(cursor, count, bytes).await
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Authority {
    pub membership: Option<Membership>,
    pub state: MdsState,
    pub healthy: bool,
}
pub struct Gate {
    pub rx: watch::Receiver<Authority>,
    pub expected: Membership,
}
impl Gate {
    pub async fn wait(&mut self, creating: bool) -> Result<(), StoreError> {
        loop {
            let now = *self.rx.borrow();
            if now.membership != Some(self.expected)
                || (creating && now.state != MdsState::CREATING)
            {
                return Err(StoreError::Fenced);
            }
            if now.healthy {
                return Ok(());
            }
            tokio::time::timeout(Duration::from_secs(300), self.rx.changed())
                .await
                .map_err(|_| StoreError::Io)?
                .map_err(|_| StoreError::Fenced)?;
        }
    }
}

macro_rules! retry_unknown {
    ($gate:expr, $creating:expr, $operation:expr) => {{
        let mut attempts = 0;
        loop {
            $gate.wait($creating).await?;
            match $operation.await {
                Err(StoreError::UnknownOutcome) if attempts < RETRIES => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
                result => break result,
            }
        }
    }};
}

pub struct Loaded {
    pub identity: StoreIdentity,
    pub owner: OwnerToken,
    pub tail: u64,
    pub checkpoint: CheckpointDescriptor,
    pub state: EngineCheckpoint,
    pub records: Vec<(u64, JournalRecord)>,
    pub sessions: RankSessionState,
    pub initial_range: InoRange,
    pub barrier: u32,
}

pub enum CommandKind {
    Append { seq: u64, body: Bytes },
    TrimInitial,
    Open(NewSessionRecord),
    Closing(SessionKey),
    Closed { key: SessionKey, cleanup_seq: u64 },
    Checkpoint(EngineCheckpoint),
}
pub struct Command {
    pub owner: OwnerToken,
    pub operation: u64,
    pub kind: CommandKind,
}
pub enum EventKind {
    Loaded(Box<Loaded>),
    Committed(u64),
    InitialTrimmed(u64),
    Opened(SessionRecord),
    Closing(SessionKey),
    Closed { key: SessionKey, cleanup_seq: u64 },
    Checkpoint(CheckpointDescriptor),
    Capacity(&'static str),
    Failed(StoreError),
}
pub struct Event {
    pub membership: Membership,
    pub owner: Option<OwnerToken>,
    pub operation: u64,
    pub kind: EventKind,
}

pub struct Start {
    pub identity: StoreIdentity,
    pub writer: WriterIdentity,
    pub config: EngineConfig,
    pub creating: bool,
    pub failure_epoch: u32,
    pub limits: Limits,
    pub operation: u64,
}

struct Opened<B> {
    store: StableStore<SharedBackend<B>>,
    repository: SessionRepository<SharedBackend<B>>,
    acquired: AcquiredSessions,
    loaded: Loaded,
}

pub fn rank_owner(membership: Membership) -> Result<RankOwner, StoreError> {
    if membership.rank != 0 || membership.gid == 0 || membership.incarnation <= 0 {
        return Err(StoreError::Fenced);
    }
    Ok(RankOwner {
        rank: 0,
        incarnation: membership.incarnation as u64,
        gid: membership.gid,
    })
}

pub fn operation_id(gid: u64, operation: u64) -> [u8; 16] {
    let mut id = [0; 16];
    id[..8].copy_from_slice(&gid.to_le_bytes());
    id[8..].copy_from_slice(&operation.to_le_bytes());
    id
}

async fn prove_writers<F: FenceBackend>(
    fence: &F,
    writers: Vec<(WriterIdentity, u32)>,
    current: &WriterIdentity,
    failure_epoch: u32,
) -> Result<Vec<FencedWriter>, StoreError> {
    let mut proofs: Vec<FencedWriter> = Vec::new();
    for (writer, previous_epoch) in writers {
        if writer == *current {
            continue;
        }
        if writer.rank_owner == current.rank_owner {
            return Err(StoreError::Fenced);
        }
        if writer.addresses.0.is_empty() {
            return Err(StoreError::Corrupt);
        }
        let mut epoch = failure_epoch.max(previous_epoch);
        for addr in &writer.addresses.0 {
            if addr
                .addr
                .is_none_or(|socket| socket.port() == 0 || socket.ip().is_unspecified())
                || current.addresses.0.contains(addr)
            {
                return Err(StoreError::Corrupt);
            }
            epoch = epoch.max(
                fencing::fence(fence, addr, epoch)
                    .await
                    .map_err(|_| StoreError::Fenced)?,
            );
        }
        if epoch == 0 {
            return Err(StoreError::Fenced);
        }
        if let Some(old) = proofs.iter_mut().find(|p| p.writer == writer) {
            old.verified_osd_epoch = old.verified_osd_epoch.max(epoch);
        } else {
            proofs.push(FencedWriter {
                writer,
                verified_osd_epoch: epoch,
            });
        }
    }
    fence.raise_barrier(
        proofs
            .iter()
            .map(|p| p.verified_osd_epoch)
            .max()
            .unwrap_or(failure_epoch),
    );
    Ok(proofs)
}

fn proof(proofs: &[FencedWriter], writer: &WriterIdentity) -> Result<FencedWriter, StoreError> {
    proofs
        .iter()
        .find(|p| &p.writer == writer)
        .cloned()
        .ok_or(StoreError::Fenced)
}

async fn open<B: Backend + PoolListing + Sync + Send, F: FenceBackend>(
    backend: SharedBackend<B>,
    start: &Start,
    gate: &mut Gate,
    fence: &F,
) -> Result<Opened<B>, StoreError> {
    gate.wait(start.creating).await?;
    let bootstrap = BootstrapStore::new(backend.clone(), start.identity.clone());
    let store = StableStore::new(
        backend.clone(),
        start.identity.clone(),
        start.limits.clone(),
    )?;
    let repository = SessionRepository::new(backend, start.identity.clone(), 0, SESSION_LIMIT)?;
    let mut format = match retry_unknown!(gate, start.creating, bootstrap.inspect_format())? {
        Some(format) => format,
        None if start.creating => {
            // Constructors that create a root are exclusive to fresh bootstrap.
            let seed = Engine::new(start.config.clone(), SeedJournal::new())
                .checkpoint()
                .map_err(|_| StoreError::Corrupt)?;
            let rank = start.writer.rank_owner;
            let owner = OwnerToken {
                shard_epoch: 1,
                rank: rank.rank,
                incarnation: rank.incarnation,
                gid: rank.gid,
            };
            let plan = BootstrapPlan::new(
                operation_id(rank.gid, start.operation),
                start.identity.clone(),
                seed,
                start.config.ino_lease,
                owner,
                start.writer.clone(),
            )?;
            retry_unknown!(
                gate,
                true,
                bootstrap.begin_bootstrap(plan.clone(), start.writer.clone())
            )?
        }
        None => return Err(StoreError::MissingState),
    };
    if format.phase == FormatPhase::Preparing && !start.creating {
        return Err(StoreError::MissingState);
    }
    let mut control = match retry_unknown!(gate, start.creating, store.inspect_control()) {
        Ok(c) => Some(c),
        Err(StoreError::MissingState) if format.phase == FormatPhase::Preparing => None,
        Err(e) => return Err(e),
    };
    let mut sessions = match retry_unknown!(gate, start.creating, repository.inspect_sessions()) {
        Ok(c) => Some(c),
        Err(StoreError::MissingState) if format.phase == FormatPhase::Preparing => None,
        Err(e) => return Err(e),
    };
    let writers = |format: &store::FormatInspection,
                   control: &Option<store::ControlInspection>,
                   sessions: &Option<store::SessionsInspection>| {
        let mut writers = vec![(format.writer.clone(), 0)];
        if let Some(p) = &format.predecessor {
            writers.push((p.writer.clone(), p.verified_osd_epoch));
        }
        if let Some(c) = control {
            writers.push((c.writer.clone(), 0));
            if let Some(p) = &c.predecessor {
                writers.push((p.writer.clone(), p.verified_osd_epoch));
            }
        }
        if let Some(c) = sessions {
            writers.push((c.writer.clone(), 0));
            if let Some(p) = &c.predecessor {
                writers.push((p.writer.clone(), p.verified_osd_epoch));
            }
        }
        writers
    };
    let mut proofs = prove_writers(
        fence,
        writers(&format, &control, &sessions),
        &start.writer,
        start.failure_epoch,
    )
    .await?;
    if format.phase == FormatPhase::Preparing && format.writer != start.writer {
        let predecessor = proof(&proofs, &format.writer)?;
        format = retry_unknown!(
            gate,
            true,
            bootstrap.acquire_preparing(&format, start.writer.clone(), predecessor.clone())
        )?;
    }
    let mut prepared_open = None;
    if format.phase == FormatPhase::Preparing {
        let candidate = control
            .as_ref()
            .map(|c| c.owner)
            .unwrap_or(format.plan.initial_owner);
        let rank = start.writer.rank_owner;
        let candidate = OwnerToken {
            shard_epoch: candidate.shard_epoch,
            rank: rank.rank,
            incarnation: rank.incarnation,
            gid: rank.gid,
        };
        match retry_unknown!(gate, true, bootstrap.resume_bootstrap(&format, candidate)) {
            Ok(opened) => prepared_open = Some(opened),
            // The reviewed bootstrap protocol requires separate guard acquisition.
            Err(StoreError::Fenced) => {}
            Err(e) => return Err(e),
        }
        control = Some(retry_unknown!(gate, true, store.inspect_control())?);
        sessions = Some(retry_unknown!(gate, true, repository.inspect_sessions())?);
        proofs.extend(
            prove_writers(
                fence,
                writers(&format, &control, &sessions),
                &start.writer,
                start.failure_epoch,
            )
            .await?,
        );
    }
    let control = control.ok_or(StoreError::MissingState)?;
    let sessions = sessions.ok_or(StoreError::MissingState)?;
    let rank = start.writer.rank_owner;
    let mut opened = if control.writer == start.writer {
        prepared_open
    } else {
        let owner = OwnerToken {
            shard_epoch: control
                .owner
                .shard_epoch
                .checked_add(1)
                .ok_or(StoreError::Capacity)?,
            rank: rank.rank,
            incarnation: rank.incarnation,
            gid: rank.gid,
        };
        let predecessor = proof(&proofs, &control.writer)?;
        Some(retry_unknown!(
            gate,
            start.creating,
            store.open(&control, owner, start.writer.clone(), predecessor.clone())
        )?)
    };
    let acquired = if sessions.writer == start.writer {
        AcquiredSessions { control: sessions }
    } else {
        let predecessor = proof(&proofs, &sessions.writer)?;
        retry_unknown!(
            gate,
            start.creating,
            repository.acquire_sessions(&sessions, rank, start.writer.clone(), predecessor.clone())
        )?
    };
    if format.phase == FormatPhase::Preparing {
        let owner = match &opened {
            Some(o) => o.owner,
            None => control.owner,
        };
        opened = Some(retry_unknown!(
            gate,
            true,
            bootstrap.resume_bootstrap(&format, owner)
        )?);
        format = retry_unknown!(gate, true, bootstrap.publish_ready(&format))?;
    }
    let opened = opened.ok_or(StoreError::Fenced)?;
    retry_unknown!(gate, start.creating, bootstrap.validate_ready(&format))?;
    let (state, records) = retry_unknown!(gate, start.creating, store.load(&opened))?;
    let rank_sessions = retry_unknown!(gate, start.creating, repository.load_sessions(&acquired))?;
    let mut barrier = proofs
        .iter()
        .map(|p| p.verified_osd_epoch)
        .max()
        .unwrap_or(start.failure_epoch);
    // A persisted Closing row can still own dirty writers. Restart must prove
    // their exclusion before resuming its engine cleanup.
    for row in &rank_sessions.records {
        if row.lifecycle != SessionLifecycle::Closing {
            continue;
        }
        if row.identity.peer_addresses.0.is_empty() {
            return Err(StoreError::Corrupt);
        }
        for addr in &row.identity.peer_addresses.0 {
            gate.wait(start.creating).await?;
            barrier = barrier.max(
                fencing::fence(fence, addr, barrier)
                    .await
                    .map_err(|_| StoreError::Fenced)?,
            );
        }
    }
    fence.raise_barrier(barrier);
    let loaded = Loaded {
        identity: start.identity.clone(),
        owner: opened.owner,
        tail: opened.tail,
        checkpoint: opened.checkpoint.ok_or(StoreError::MissingState)?,
        state,
        records,
        sessions: rank_sessions,
        initial_range: format.plan.initial_range,
        barrier,
    };
    Ok(Opened {
        store,
        repository,
        acquired,
        loaded,
    })
}

async fn trim<B: Backend + Sync + Send>(
    store: &StableStore<SharedBackend<B>>,
    owner: OwnerToken,
    descriptor: &CheckpointDescriptor,
    from: u64,
    page: usize,
    gate: &mut Gate,
) -> Result<(), StoreError> {
    let mut cursor = from;
    while cursor < descriptor.covered_seq {
        let end = cursor
            .saturating_add(page as u64)
            .min(descriptor.covered_seq);
        let sequences: Vec<_> = (cursor + 1..=end).collect();
        retry_unknown!(gate, false, store.trim(owner, descriptor, &sequences))?;
        cursor = end;
    }
    Ok(())
}

pub async fn worker<
    B: Backend + PoolListing + Sync + Send + 'static,
    F: FenceBackend + Send + 'static,
>(
    backend: B,
    start: Start,
    mut gate: Gate,
    fence: F,
    mut commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<Event>,
) {
    let membership = gate.expected;
    let startup_operation = start.operation;
    let result = open(SharedBackend::new(backend), &start, &mut gate, &fence).await;
    let mut opened = match result {
        Ok(opened) => opened,
        Err(error) => {
            let _ = events
                .send(Event {
                    membership,
                    owner: None,
                    operation: startup_operation,
                    kind: EventKind::Failed(error),
                })
                .await;
            return;
        }
    };
    let owner = opened.loaded.owner;
    let mut descriptor = opened.loaded.checkpoint.clone();
    let mut trimmed = 0;
    let mut tail = opened.loaded.tail;
    let _ = events
        .send(Event {
            membership,
            owner: Some(owner),
            operation: startup_operation,
            kind: EventKind::Loaded(Box::new(opened.loaded)),
        })
        .await;
    while let Some(command) = commands.recv().await {
        if command.owner != owner {
            continue;
        }
        let result: Result<EventKind, StoreError> = async {
            gate.wait(false).await?;
            match command.kind {
                CommandKind::Append { seq, body } => {
                    if seq != tail + 1 {
                        return Err(StoreError::Corrupt);
                    }
                    let id = operation_id(owner.gid, seq);
                    // Repeated append arguments retain the same pending batch and identity.
                    let records = [body];
                    let committed = retry_unknown!(
                        &mut gate,
                        false,
                        opened.store.append(owner, tail, id, &records)
                    )?;
                    if committed != seq {
                        return Err(StoreError::Corrupt);
                    }
                    tail = committed;
                    Ok(EventKind::Committed(committed))
                }
                CommandKind::TrimInitial => {
                    trim(
                        &opened.store,
                        owner,
                        &descriptor,
                        0,
                        start.limits.page_keys,
                        &mut gate,
                    )
                    .await?;
                    trimmed = descriptor.covered_seq;
                    Ok(EventKind::InitialTrimmed(trimmed))
                }
                CommandKind::Open(record) => {
                    let before = opened.acquired.control.revision;
                    let (key, revision) = retry_unknown!(
                        &mut gate,
                        false,
                        opened.repository.open_session_record(
                            &opened.acquired,
                            before,
                            record.clone()
                        )
                    )?;
                    opened.acquired.control.revision = revision;
                    Ok(EventKind::Opened(SessionRecord {
                        key,
                        identity: record,
                        lifecycle: SessionLifecycle::Open,
                        cleanup_seq: None,
                    }))
                }
                CommandKind::Closing(key) => {
                    let before = opened.acquired.control.revision;
                    let revision = retry_unknown!(
                        &mut gate,
                        false,
                        opened
                            .repository
                            .mark_session_closing(&opened.acquired, before, key)
                    )?;
                    opened.acquired.control.revision = revision;
                    Ok(EventKind::Closing(key))
                }
                CommandKind::Closed { key, cleanup_seq } => {
                    let before = opened.acquired.control.revision;
                    let revision = retry_unknown!(
                        &mut gate,
                        false,
                        opened.repository.mark_session_closed(
                            &opened.acquired,
                            before,
                            key,
                            cleanup_seq
                        )
                    )?;
                    opened.acquired.control.revision = revision;
                    Ok(EventKind::Closed { key, cleanup_seq })
                }
                CommandKind::Checkpoint(state) => {
                    if state.covered_seq != tail {
                        return Err(StoreError::Corrupt);
                    }
                    let generation = descriptor
                        .generation
                        .checked_add(1)
                        .ok_or(StoreError::Capacity)?;
                    let next = retry_unknown!(
                        &mut gate,
                        false,
                        opened.store.write_checkpoint(owner, generation, &state)
                    )?;
                    retry_unknown!(
                        &mut gate,
                        false,
                        opened
                            .store
                            .publish_checkpoint(owner, descriptor.generation, &next)
                    )?;
                    trim(
                        &opened.store,
                        owner,
                        &next,
                        trimmed,
                        start.limits.page_keys,
                        &mut gate,
                    )
                    .await?;
                    trimmed = next.covered_seq;
                    descriptor = next.clone();
                    Ok(EventKind::Checkpoint(next))
                }
            }
        }
        .await;
        let kind = match result {
            Ok(event) => event,
            Err(StoreError::Capacity) => EventKind::Capacity("store admission/checkpoint backlog"),
            Err(error) => EventKind::Failed(error),
        };
        let failed = matches!(kind, EventKind::Failed(_));
        if events
            .send(Event {
                membership,
                owner: Some(owner),
                operation: command.operation,
                kind,
            })
            .await
            .is_err()
            || failed
        {
            return;
        }
    }
}

/// Prefix needed for authoritative filesystem id discovery. Follows C++
/// MFSMap::encode_payload, FSMap::encode and Filesystem::encode (FSMap.cc).
pub fn decode_filesystems(mut front: Bytes) -> denc::Result<Vec<(u64, MdsMap)>> {
    let _epoch = u32::decode(&mut front)?;
    let mut frame = denc::decode_start(&mut front, 8, "FSMap")?;
    let b = &mut frame.body;
    let _epoch = u32::decode(b)?;
    let _next_id = u64::decode(b)?;
    let _legacy = i32::decode(b)?;
    let _compat = monc::CompatSet::decode(b)?;
    let _multiple = bool::decode(b)?;
    let count = u32::decode(b)?;
    let mut filesystems = Vec::new();
    // Never allocate from an unchecked wire count.
    for _ in 0..count.min(1024) {
        let mut fs = denc::decode_start(b, 2, "Filesystem")?;
        let id = i32::decode(&mut fs.body)?;
        let mut encoded = Bytes::decode(&mut fs.body)?;
        let map = MdsMap::decode(&mut encoded)?;
        if id < 0 || filesystems.iter().any(|(old, _)| *old == id as u64) {
            return Err(denc::DecodeError::Invalid {
                what: "Filesystem",
                detail: "invalid or duplicate id".into(),
            });
        }
        filesystems.push((id as u64, map));
    }
    if count > 1024 {
        return Err(denc::DecodeError::Invalid {
            what: "FSMap",
            detail: "filesystem count exceeds budget".into(),
        });
    }
    Ok(filesystems)
}

#[cfg(test)]
mod fault_tests;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_backend::{Fence, MemoryBackend};

    fn start(incarnation: u64, creating: bool) -> (Start, Gate, watch::Sender<Authority>) {
        let membership = Membership {
            rank: 0,
            incarnation: incarnation as i32,
            gid: incarnation,
        };
        let (tx, rx) = watch::channel(Authority {
            membership: Some(membership),
            state: if creating {
                MdsState::CREATING
            } else {
                MdsState::REPLAY
            },
            healthy: true,
        });
        let writer = WriterIdentity {
            rank_owner: rank_owner(membership).unwrap(),
            addresses: denc::EntityAddrVec(vec![denc::EntityAddr::new(
                denc::AddrType::LEGACY,
                "127.0.0.1:7402".parse().unwrap(),
                incarnation as u32,
            )]),
        };
        (
            Start {
                identity: StoreIdentity {
                    cluster_fsid: [7; 16],
                    filesystem_id: 3,
                    metadata_pool: 6,
                },
                writer,
                config: EngineConfig::default(),
                creating,
                failure_epoch: 0,
                limits: Limits::default(),
                operation: 1,
            },
            Gate {
                rx,
                expected: membership,
            },
            tx,
        )
    }

    #[tokio::test]
    async fn bootstrap_restart_preserves_identity_and_fences_before_acquisition() {
        let backend = MemoryBackend::default();
        let (first, mut gate, _authority) = start(1, true);
        let opened = open(
            SharedBackend::new(backend.clone()),
            &first,
            &mut gate,
            &Fence::new(backend.clone()),
        )
        .await
        .unwrap();
        let original = opened.loaded.state.clone();
        backend.memory.lock().unwrap().trace.clear();
        let (second, mut gate, _authority) = start(2, false);
        let restored = open(
            SharedBackend::new(backend.clone()),
            &second,
            &mut gate,
            &Fence::new(backend.clone()),
        )
        .await
        .unwrap();
        assert_eq!(restored.loaded.identity, first.identity);
        assert_eq!(restored.loaded.state, original);
        assert!(restored.loaded.records.is_empty());
        assert_eq!(restored.loaded.owner.shard_epoch, 2);
        let memory = backend.memory.lock().unwrap();
        let barrier = memory.trace.iter().position(|s| s == "barrier:9").unwrap();
        let first_write = memory
            .trace
            .iter()
            .position(|s| s.starts_with("write:"))
            .unwrap();
        assert!(barrier < first_write);
    }

    #[tokio::test]
    async fn missing_foreign_corrupt_and_failed_fence_never_create_namespace() {
        let backend = MemoryBackend::default();
        let (replay, mut gate, _authority) = start(2, false);
        assert!(matches!(
            open(
                SharedBackend::new(backend.clone()),
                &replay,
                &mut gate,
                &Fence::new(backend.clone())
            )
            .await,
            Err(StoreError::MissingState)
        ));
        assert_eq!(backend.memory.lock().unwrap().writes, 0);
        let (fresh, mut gate, _authority) = start(1, true);
        open(
            SharedBackend::new(backend.clone()),
            &fresh,
            &mut gate,
            &Fence::new(backend.clone()),
        )
        .await
        .unwrap();
        let before = backend.memory.lock().unwrap().writes;
        let (mut replay, mut gate, _authority) = start(2, false);
        let mut fence = Fence::new(backend.clone());
        fence.fail = true;
        assert!(matches!(
            open(
                SharedBackend::new(backend.clone()),
                &replay,
                &mut gate,
                &fence
            )
            .await,
            Err(StoreError::Fenced)
        ));
        assert_eq!(backend.memory.lock().unwrap().writes, before);
        replay.identity.filesystem_id += 1;
        assert!(
            open(
                SharedBackend::new(backend.clone()),
                &replay,
                &mut gate,
                &Fence::new(backend.clone())
            )
            .await
            .is_err()
        );
        assert_eq!(backend.memory.lock().unwrap().writes, before);
        backend
            .memory
            .lock()
            .unwrap()
            .objects
            .get_mut("rmds.format")
            .unwrap()
            .data = Bytes::from_static(b"corrupt");
        replay.identity = fresh.identity.clone();
        assert!(
            open(
                SharedBackend::new(backend.clone()),
                &replay,
                &mut gate,
                &Fence::new(backend.clone())
            )
            .await
            .is_err()
        );
        assert_eq!(backend.memory.lock().unwrap().writes, before);
    }

    #[tokio::test]
    async fn partial_shard_acquisition_resumes_with_next_owner() {
        let backend = MemoryBackend::default();
        let (fresh, mut gate, _authority) = start(1, true);
        open(
            SharedBackend::new(backend.clone()),
            &fresh,
            &mut gate,
            &Fence::new(backend.clone()),
        )
        .await
        .unwrap();
        backend.memory.lock().unwrap().fail_write = Some("rmds.sessions.0".into());
        let (second, mut gate, _authority) = start(2, false);
        assert!(
            open(
                SharedBackend::new(backend.clone()),
                &second,
                &mut gate,
                &Fence::new(backend.clone())
            )
            .await
            .is_err()
        );
        let (third, mut gate, _authority) = start(3, false);
        let loaded = open(
            SharedBackend::new(backend.clone()),
            &third,
            &mut gate,
            &Fence::new(backend.clone()),
        )
        .await
        .unwrap()
        .loaded;
        assert_eq!(loaded.owner.gid, 3);
        assert_eq!(loaded.sessions.control.owner.gid, 3);
    }

    #[derive(Clone, Default)]
    struct Link(std::sync::Arc<std::sync::Mutex<Vec<denc::RawMessage>>>);
    impl crate::session::ClientLink for Link {
        fn send_raw(&self, raw: denc::RawMessage) -> bool {
            self.0.lock().unwrap().push(raw);
            true
        }
        fn features(&self) -> u64 {
            denc::features::ALL
        }
        fn peer_addr(&self) -> denc::EntityAddr {
            denc::EntityAddr::new(
                denc::AddrType::LEGACY,
                "127.0.0.1:4000".parse().unwrap(),
                42,
            )
        }
        fn authenticated_identity(&self) -> Option<(String, Bytes)> {
            Some(("client.42".into(), Bytes::new()))
        }
    }
    async fn event(rx: &mut mpsc::Receiver<Event>) -> Event {
        tokio::time::timeout(Duration::from_secs(3), rx.recv())
            .await
            .unwrap()
            .unwrap()
    }
    fn apply(
        server: &mut crate::server::Server,
        sessions: &mut crate::session::SessionMap,
        event: Event,
    ) {
        server.on_storage_event(sessions, event.owner.unwrap(), event.operation, event.kind);
        assert!(server.fatal_error().is_none(), "{:?}", server.fatal_error());
    }
    async fn settle(
        server: &mut crate::server::Server,
        sessions: &mut crate::session::SessionMap,
        rx: &mut mpsc::Receiver<Event>,
    ) {
        for _ in 0..4096 {
            server.pump(sessions, 32);
            if server.pending_storage() > 0 {
                apply(server, sessions, event(rx).await);
            } else if !server.has_work() {
                return;
            }
        }
        panic!("production scheduler did not settle");
    }
    async fn running(
        backend: MemoryBackend,
    ) -> (
        crate::server::Server,
        crate::session::SessionMap,
        mpsc::Receiver<Event>,
        tokio::task::JoinHandle<()>,
        watch::Sender<Authority>,
    ) {
        let (start, gate, authority) = start(1, true);
        let limits = start.limits.clone();
        let (tx, commands) = mpsc::channel(16);
        let (events, mut rx) = mpsc::channel(64);
        let task = tokio::spawn(worker(
            backend.clone(),
            start,
            gate,
            Fence::new(backend),
            commands,
            events,
        ));
        let loaded = match event(&mut rx).await.kind {
            EventKind::Loaded(loaded) => *loaded,
            _ => panic!("bootstrap failed"),
        };
        let server = crate::server::Server::from_loaded(
            "a".into(),
            1,
            1,
            loaded,
            EngineConfig::default(),
            &limits,
            tx,
            1,
            16,
            Duration::from_secs(30),
        )
        .unwrap();
        let mut sessions = crate::session::SessionMap::default();
        let mut server = server;
        settle(&mut server, &mut sessions, &mut rx).await;
        assert!(!server.try_finish_recovery(&mut sessions));
        assert!(server.recovery_structural_done());
        settle(&mut server, &mut sessions, &mut rx).await;
        assert!(server.try_finish_recovery(&mut sessions));
        server.pause_data_work(false);
        (server, sessions, rx, task, authority)
    }

    #[tokio::test]
    async fn real_worker_delays_open_reply_and_cleanup_close_until_durable_completions() {
        use denc::Message;
        let backend = MemoryBackend::default();
        let (mut server, mut sessions, mut rx, task, _authority) = running(backend.clone()).await;
        let link = Link::default();
        server.handle_session(
            &mut sessions,
            42,
            Box::new(link.clone()),
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        assert!(link.0.lock().unwrap().is_empty());
        assert!(sessions.get(42).is_none());
        let opened = event(&mut rx).await;
        let mut stale_owner = server.owner();
        stale_owner.gid += 10;
        server.on_storage_event(
            &mut sessions,
            stale_owner,
            opened.operation,
            EventKind::Failed(StoreError::Io),
        );
        assert!(server.fatal_error().is_none());
        assert!(link.0.lock().unwrap().is_empty());
        apply(&mut server, &mut sessions, opened);
        assert_eq!(
            proto::MClientSession::from_raw(link.0.lock().unwrap()[0].clone())
                .unwrap()
                .op,
            proto::SessionOp::OPEN
        );
        link.0.lock().unwrap().clear();
        let mut request = proto::MClientRequest::new(1, proto::MdsOp::MKDIR);
        request.path = proto::FilePath::new(1, "durable");
        request.head.flags = proto::request::request_flags::WANT_DENTRY;
        backend.memory.lock().unwrap().lose_append_reply = true;
        server.handle(&mut sessions, 42, proto::ClientMessage::Request(request));
        assert!(link.0.lock().unwrap().is_empty());
        let committed = event(&mut rx).await;
        assert!(matches!(committed.kind, EventKind::Committed(_)));
        assert!(link.0.lock().unwrap().is_empty());
        apply(&mut server, &mut sessions, committed);
        assert!(
            link.0
                .lock()
                .unwrap()
                .iter()
                .any(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
        );
        assert_eq!(
            backend.memory.lock().unwrap().appends,
            1,
            "unknown append must reconcile the original batch"
        );
        link.0.lock().unwrap().clear();
        let seq = sessions.get(42).unwrap().push_seq();
        server.handle_session(
            &mut sessions,
            42,
            Box::new(link.clone()),
            proto::MClientSession::new(proto::SessionOp::REQUEST_CLOSE, seq),
            vec![],
        );
        assert!(link.0.lock().unwrap().is_empty());
        let closing = event(&mut rx).await;
        assert!(matches!(closing.kind, EventKind::Closing(_)));
        apply(&mut server, &mut sessions, closing);
        assert!(link.0.lock().unwrap().is_empty());
        let cleanup = event(&mut rx).await;
        assert!(matches!(cleanup.kind, EventKind::Committed(_)));
        apply(&mut server, &mut sessions, cleanup);
        assert!(link.0.lock().unwrap().is_empty());
        let closed = event(&mut rx).await;
        assert!(matches!(closed.kind, EventKind::Closed { .. }));
        apply(&mut server, &mut sessions, closed);
        assert!(sessions.get(42).is_none());
        assert!(
            link.0
                .lock()
                .unwrap()
                .iter()
                .any(|raw| raw.msg_type == proto::msg_type::CLIENT_SESSION
                    && proto::MClientSession::from_raw(raw.clone()).unwrap().op
                        == proto::SessionOp::CLOSE)
        );
        server.request_checkpoint();
        settle(&mut server, &mut sessions, &mut rx).await;
        task.abort();
        let (restart, mut gate, _authority) = start(2, false);
        let loaded = open(
            SharedBackend::new(backend.clone()),
            &restart,
            &mut gate,
            &Fence::new(backend.clone()),
        )
        .await
        .unwrap()
        .loaded;
        assert!(
            loaded.records.is_empty(),
            "restart must load the published checkpoint without WAL"
        );
        assert!(loaded.state.inodes.len() > 1);
        assert!(
            loaded
                .sessions
                .records
                .iter()
                .all(|r| r.lifecycle == SessionLifecycle::Closed)
        );
    }

    #[tokio::test]
    async fn failed_real_append_discards_reply_and_stops_engine_service() {
        let backend = MemoryBackend::default();
        let (mut server, mut sessions, mut rx, task, _authority) = running(backend.clone()).await;
        let link = Link::default();
        server.handle_session(
            &mut sessions,
            42,
            Box::new(link.clone()),
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        settle(&mut server, &mut sessions, &mut rx).await;
        link.0.lock().unwrap().clear();
        backend.memory.lock().unwrap().fail_write = Some("rmds.wal.0.0".into());
        let mut request = proto::MClientRequest::new(1, proto::MdsOp::MKDIR);
        request.path = proto::FilePath::new(1, "failed");
        server.handle(&mut sessions, 42, proto::ClientMessage::Request(request));
        let failed = event(&mut rx).await;
        assert!(matches!(failed.kind, EventKind::Failed(StoreError::Io)));
        server.on_storage_event(
            &mut sessions,
            failed.owner.unwrap(),
            failed.operation,
            failed.kind,
        );
        assert!(server.fatal_error().is_some());
        assert!(link.0.lock().unwrap().is_empty());
        server.pump(&mut sessions, 32);
        assert!(!server.has_work());
        assert!(link.0.lock().unwrap().is_empty());
        task.abort();
    }

    #[tokio::test]
    async fn paused_normal_queue_does_not_spin_and_nonempty_restart_holds_gather() {
        let backend = MemoryBackend::default();
        let (mut server, mut sessions, mut rx, task, _authority) = running(backend.clone()).await;
        let link = Link::default();
        server.handle_session(
            &mut sessions,
            42,
            Box::new(link),
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        settle(&mut server, &mut sessions, &mut rx).await;
        server.pause_data_work(true);
        let mut request = proto::MClientRequest::new(1, proto::MdsOp::MKDIR);
        request.path = proto::FilePath::new(1, "paused");
        server.handle(&mut sessions, 42, proto::ClientMessage::Request(request));
        assert!(!server.has_work());
        server.pause_data_work(false);
        assert!(server.has_work());
        settle(&mut server, &mut sessions, &mut rx).await;
        task.abort();
        let (restart, mut gate, _authority) = start(2, false);
        let loaded = open(
            SharedBackend::new(backend.clone()),
            &restart,
            &mut gate,
            &Fence::new(backend.clone()),
        )
        .await
        .unwrap()
        .loaded;
        let (tx, mut commands) = mpsc::channel(16);
        let mut restored = crate::server::Server::from_loaded(
            "a".into(),
            1,
            1,
            loaded,
            EngineConfig::default(),
            &Limits::default(),
            tx,
            1,
            16,
            Duration::from_secs(30),
        )
        .unwrap();
        let mut sessions = crate::session::SessionMap::default();
        restored.pump(&mut sessions, 32);
        let command = commands.recv().await.unwrap();
        restored.on_storage_event(
            &mut sessions,
            command.owner,
            command.operation,
            EventKind::InitialTrimmed(1),
        );
        assert_eq!(restored.gather_len(), 1);
        assert!(!restored.try_finish_recovery(&mut sessions));
        assert!(!restored.recovered());
    }

    #[tokio::test]
    async fn commit_completion_wakes_queued_work_without_new_client_input() {
        let backend = MemoryBackend::default();
        let (mut server, mut sessions, mut rx, task, _authority) = running(backend).await;
        let link = Link::default();
        server.handle_session(
            &mut sessions,
            42,
            Box::new(link.clone()),
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        settle(&mut server, &mut sessions, &mut rx).await;
        link.0.lock().unwrap().clear();
        for tid in 1..=2 {
            let mut request = proto::MClientRequest::new(tid, proto::MdsOp::MKDIR);
            request.path = proto::FilePath::new(1, format!("directory{tid}"));
            request.head.flags = proto::request::request_flags::WANT_DENTRY;
            server.handle(&mut sessions, 42, proto::ClientMessage::Request(request));
        }
        assert!(link.0.lock().unwrap().is_empty());
        apply(&mut server, &mut sessions, event(&mut rx).await);
        assert!(
            server.pending_storage() > 0,
            "completion must schedule the next admitted record"
        );
        settle(&mut server, &mut sessions, &mut rx).await;
        assert_eq!(
            link.0
                .lock()
                .unwrap()
                .iter()
                .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
                .count(),
            2
        );
        task.abort();
    }

    #[tokio::test]
    async fn g3_real_worker_persists_attachment_union_before_production_reconnect_dispatch() {
        use denc::Message;
        #[derive(Clone)]
        struct Reattached(Link);
        impl crate::session::ClientLink for Reattached {
            fn connection_id(&self) -> u64 {
                9001
            }
            fn send_raw(&self, raw: denc::RawMessage) -> bool {
                self.0.send_raw(raw)
            }
            fn features(&self) -> u64 {
                self.0.features()
            }
            fn peer_addr(&self) -> denc::EntityAddr {
                denc::EntityAddr {
                    nonce: 43,
                    ..self.0.peer_addr()
                }
            }
            fn authenticated_identity(&self) -> Option<(String, Bytes)> {
                Some((
                    "client.42".into(),
                    Bytes::from_static(b"fresh verified context"),
                ))
            }
        }
        use crate::session::ClientLink;
        let backend = MemoryBackend::default();
        let (mut old, mut old_sessions, mut old_rx, old_task, _old_authority) =
            running(backend.clone()).await;
        let old_link = Link::default();
        old.handle_session(
            &mut old_sessions,
            42,
            Box::new(old_link.clone()),
            proto::MClientSession::new(proto::SessionOp::REQUEST_OPEN, 0),
            vec![],
        );
        settle(&mut old, &mut old_sessions, &mut old_rx).await;
        let mut request = proto::MClientRequest::new(1, proto::MdsOp::MKDIR);
        request.path = proto::FilePath::new(1, "kept");
        old.handle(
            &mut old_sessions,
            42,
            proto::ClientMessage::Request(request),
        );
        settle(&mut old, &mut old_sessions, &mut old_rx).await;
        {
            let replies = old_link.0.lock().unwrap();
            assert!(
                replies
                    .iter()
                    .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
                    .any(|raw| {
                        let reply = proto::MClientReply::from_raw(raw.clone()).unwrap();
                        reply.tid == 1 && reply.result == 0
                    }),
                "setup must execute the production E2 release marker and commit mkdir"
            );
        }
        assert!(
            old.checkpoint_for_test()
                .unwrap()
                .sessions
                .iter()
                .any(|session| session
                    .completed_requests
                    .iter()
                    .any(|request| request.tid == 1 && request.result == 0))
        );
        old_task.abort();
        let (start, gate, _authority) = start(2, false);
        let identity = start.identity.clone();
        let limits = start.limits.clone();
        let (tx, commands) = mpsc::channel(16);
        let (events, mut rx) = mpsc::channel(64);
        let task = tokio::spawn(worker(
            backend.clone(),
            start,
            gate,
            Fence::new(backend.clone()),
            commands,
            events,
        ));
        let loaded = match event(&mut rx).await.kind {
            EventKind::Loaded(loaded) => *loaded,
            _ => panic!("restart failed"),
        };
        let key = loaded.sessions.records[0].key;
        let mut server = crate::server::Server::from_loaded(
            "a".into(),
            2,
            1,
            loaded,
            EngineConfig::default(),
            &limits,
            tx,
            1,
            16,
            Duration::from_secs(30),
        )
        .unwrap();
        let mut sessions = crate::session::SessionMap::default();
        settle(&mut server, &mut sessions, &mut rx).await;
        server.confirm_reconnect(std::time::Instant::now(), Duration::from_secs(45));
        let link = Reattached(Link::default());
        let msg = proto::MClientReconnect::default();
        crate::daemon::dispatch_client_raw(
            &mut server,
            &mut sessions,
            42,
            Box::new(link.clone()),
            vec![],
            msg.to_raw(denc::features::ALL),
        );
        assert!(sessions.get(42).is_none());
        assert!(link.0.0.lock().unwrap().is_empty());
        let opened = event(&mut rx).await;
        assert!(matches!(&opened.kind, EventKind::Opened(row) if row.key == key));
        let repository = SessionRepository::new(backend, identity, 0, SESSION_LIMIT).unwrap();
        let control = repository.inspect_sessions().await.unwrap();
        let durable = repository
            .load_sessions(&AcquiredSessions { control })
            .await
            .unwrap();
        let row = durable.records.iter().find(|r| r.key == key).unwrap();
        assert_eq!(row.identity.peer_addresses.0.len(), 2);
        assert_eq!(
            row.identity.authorization_context,
            Bytes::from_static(b"fresh verified context")
        );
        assert_eq!(
            server.gather_len(),
            1,
            "durable write alone is not an applied completion"
        );
        apply(&mut server, &mut sessions, opened);
        assert_eq!(server.gather_len(), 0);
        assert_eq!(sessions.get(42).unwrap().addr, link.peer_addr());
        assert!(
            link.0.0.lock().unwrap().is_empty(),
            "collection must not publish OPEN or grants"
        );
        assert!(!server.try_finish_recovery(&mut sessions));
        task.abort();
    }

    #[tokio::test]
    async fn retained_checkpoint_generation_capacity_pauses_without_retry_spin() {
        let backend = MemoryBackend::default();
        let (mut server, mut sessions, mut rx, task, _authority) = running(backend.clone()).await;
        for _ in 0..Limits::default().max_checkpoint_generations {
            server.request_checkpoint();
            settle(&mut server, &mut sessions, &mut rx).await;
        }
        let writes = backend.memory.lock().unwrap().writes;
        server.pump(&mut sessions, 100);
        assert!(!server.has_work());
        assert!(server.fatal_error().is_none());
        assert_eq!(backend.memory.lock().unwrap().writes, writes);
        task.abort();
    }
    #[test]
    fn native_fsmap_identity_matches_real_capture() {
        let bytes = Bytes::from_static(include_bytes!("../../../testdata/maps/fsmap.bin"));
        // fsmap.bin is the encoded FSMap without the MFSMap epoch prefix.
        let mut front = denc::BytesMut::new();
        front.extend_from_slice(&0u32.to_le_bytes());
        front.extend_from_slice(&bytes);
        let filesystems = decode_filesystems(front.freeze()).unwrap();
        assert!(
            filesystems
                .iter()
                .any(|(id, map)| *id == 3 && map.fs_name == "a" && map.metadata_pool == 6)
        );
    }
}
