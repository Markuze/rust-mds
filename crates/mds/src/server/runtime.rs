//! Production one-record execution and durable session lifecycle. Compatibility
//! helpers in the parent module are compiled only for MemJournal unit tests.

mod d6;
mod ownership;
mod recovery;
#[cfg(test)]
mod tests;
pub(crate) use recovery::RecoveryFence;

use super::*;
use crate::session::{ClientLink, SessionOutcome};
use crate::{
    journal::{DaemonJournal, Write},
    persistence::{self, Command, CommandKind, EventKind, Loaded},
};
use denc::Message;
use engine::{
    EngineInput, EngineNotice, EngineStep, JournalError, JournalEvent, LifecycleIntent, SessionKey,
    StepError, StepState,
};
use store::{OwnerToken, SessionLifecycle, SessionRecord};

const INPUTS: usize = 1024;
const INPUT_BYTES: usize = 16 << 20;
const CONTROL_INPUTS: usize = 128;
const CONTROL_BYTES: usize = 4 << 20;

struct Input {
    input: EngineInput,
    bytes: usize,
    origin: ownership::Origin,
}
struct Opening {
    link: Box<dyn ClientLink>,
    msg: MClientSession,
    cap_auths: Vec<MdsCapAuth>,
    bytes: usize,
    generation: Option<u64>,
    new_session: bool,
    purpose: recovery::OpenPurpose,
}
#[derive(Clone, Copy)]
enum CloseStage {
    Drain,
    WaitFence,
    WaitSeq,
    Marking,
    ReadyClean,
    Cleaning,
    Publishing,
}
struct Closing {
    key: SessionKey,
    generation: u64,
    kill: bool,
    reply: bool,
    request_seq: Option<u64>,
    stage: CloseStage,
}
enum Pending {
    Append(u64),
    Open(u64),
    Closing(SessionKey),
    Closed(SessionKey, u64),
    Checkpoint,
    InitialTrim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryDrain {
    Reconstruction,
    Controls,
    GrantsReleased,
}

pub(super) struct Runtime {
    next_input: u64,
    receive_ordinal: Option<u64>,
    owned: BTreeMap<u64, ownership::Origin>,
    output_owned: std::collections::BTreeSet<u64>,
    retry_wait: VecDeque<ownership::Pressure>,
    terminal_hold: Vec<Input>,
    terminal_unidentified: Option<(EngineInput, usize)>,
    terminal_opening: HashMap<u64, Opening>,
    terminal_opening_bytes: usize,
    terminal_write: Option<Write>,
    submitted_commands: BTreeMap<u64, Command>,
    capacity_epoch: u64,
    identity_probe: bool,
    marker_owned: bool,
    raw_controls_pending: bool,
    recovery_requests_allowed: bool,
    recovery_marker_allowed: bool,
    retry_keys: std::collections::BTreeSet<(SessionKey, u64)>,
    recovery_jobs: BTreeMap<u64, u64>,
    required_jobs: std::collections::BTreeSet<u64>,
    d6: d6::D6,
    recovery: recovery::Recovery,
    execution_paused: bool,
    ordinary_paused: bool,
    pub owner: OwnerToken,
    tx: tokio::sync::mpsc::Sender<Command>,
    commands: VecDeque<Command>,
    pending: BTreeMap<u64, Pending>,
    next_operation: u64,
    rows: BTreeMap<SessionKey, SessionRecord>,
    keys: HashMap<u64, SessionKey>,
    attached: HashMap<u64, u64>,
    opening: HashMap<u64, Opening>,
    opening_bytes: usize,
    closing: BTreeMap<SessionKey, Closing>,
    gather: std::collections::BTreeSet<SessionKey>,
    normal: VecDeque<Input>,
    control: VecDeque<Input>,
    normal_bytes: usize,
    control_bytes: usize,
    state: StepState,
    initialized: bool,
    recovery_drain: RecoveryDrain,
    recovered: bool,
    checkpoint_wanted: bool,
    checkpoint_busy: bool,
    checkpoint_seq: u64,
    checkpoint_records: u64,
    checkpoint_interval: Duration,
    last_checkpoint: Instant,
    pressure: bool,
    blocked: bool,
    busy_controls: usize,
    busy_keys: std::collections::BTreeSet<SessionKey>,
}

impl Runtime {
    pub(super) fn pause_ordinary(&mut self, paused: bool) {
        self.ordinary_paused = paused;
    }
    fn session_slots(&self) -> usize {
        self.keys.len()
            + self
                .opening
                .values()
                .filter(|open| open.new_session)
                .count()
    }
    pub(super) fn cancel_pending(&mut self) {
        // Stop, do not acknowledge or discard ownership. The daemon retains this
        // runtime through shutdown, including exact unknown command arguments.
        self.execution_paused = true;
        self.ordinary_paused = true;
        self.recovered = false;
        self.terminal_opening
            .extend(std::mem::take(&mut self.opening));
        self.terminal_opening_bytes += std::mem::take(&mut self.opening_bytes);
        self.busy_controls = 0;
        self.busy_keys.clear();
        self.blocked = false;
    }
    fn queued_for(&self, key: SessionKey) -> bool {
        self.normal
            .iter()
            .chain(self.control.iter())
            .any(|q| match &q.input {
                EngineInput::Request { key: k, .. }
                | EngineInput::CapUpdate { key: k, .. }
                | EngineInput::CapRelease { key: k, .. } => *k == key,
                _ => false,
            })
    }
    fn recovery_cut_drained(&self) -> bool {
        self.initialized
            && self.opening.is_empty()
            && self.closing.is_empty()
            && self.pending.is_empty()
            && self.commands.is_empty()
            && self.control.is_empty()
            && !self.raw_controls_pending
            && !self.recovery.controls_pending()
            && !self.owned.values().any(|o| o.control)
    }
    fn cancel_requests(&mut self, key: SessionKey) {
        self.normal.retain(|q| {
            if matches!(&q.input, EngineInput::Request { key: k, .. } if *k == key) {
                self.normal_bytes -= q.bytes;
                false
            } else {
                true
            }
        });
        self.retry_wait.retain(|p| {
            if matches!(&p.input.input, EngineInput::Request { key: k, .. } if *k == key) {
                self.normal_bytes -= p.input.bytes;
                false
            } else {
                true
            }
        });
    }
    fn command(&mut self, kind: CommandKind, pending: Pending) {
        self.next_operation += 1;
        self.pending.insert(self.next_operation, pending);
        let command = Command {
            owner: self.owner,
            operation: self.next_operation,
            kind,
        };
        self.submitted_commands
            .insert(self.next_operation, command.clone());
        self.commands.push_back(command);
    }
    fn flush_commands(&mut self) -> Result<(), ()> {
        while let Some(command) = self.commands.pop_front() {
            match self.tx.try_send(command) {
                Ok(()) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Full(command)) => {
                    self.commands.push_front(command);
                    break;
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(command)) => {
                    self.commands.push_front(command);
                    return Err(());
                }
            }
        }
        Ok(())
    }
    fn enqueue(&mut self, input: EngineInput, bytes: usize) -> Result<(), ()> {
        let control = match &input {
            EngineInput::Request { request, .. } => {
                matches!(&request.op, Op::SetFileLock { lock, .. } if lock.ltype == engine::consts::CEPH_LOCK_UNLOCK)
            }
            _ => true,
        };
        let origin = match self.new_origin(&input, bytes, control) {
            Ok(origin) => origin,
            Err(()) => {
                self.terminal_unidentified = Some((input, bytes));
                return Err(());
            }
        };
        let owned_count = self.owned.values().filter(|o| o.control == control).count()
            + if control { 0 } else { self.retry_wait.len() };
        let (queue, used, max_count, max_bytes) = if control {
            (
                &mut self.control,
                &mut self.control_bytes,
                CONTROL_INPUTS,
                CONTROL_BYTES,
            )
        } else {
            (
                &mut self.normal,
                &mut self.normal_bytes,
                INPUTS,
                INPUT_BYTES,
            )
        };
        if queue.len() + owned_count >= max_count || bytes > max_bytes.saturating_sub(*used) {
            self.terminal_hold.push(Input {
                input,
                bytes,
                origin,
            });
            return Err(());
        }
        *used += bytes;
        queue.push_back(Input {
            input,
            bytes,
            origin,
        });
        self.blocked = false;
        self.busy_controls = 0;
        Ok(())
    }
}

impl Server<DaemonJournal> {
    #[allow(clippy::too_many_arguments)] // Startup joins wire identity, checked state and bounded worker settings.
    pub fn from_loaded(
        fs_name: String,
        data_pool: i64,
        epoch: u32,
        loaded: Loaded,
        mut config: EngineConfig,
        limits: &store::Limits,
        tx: tokio::sync::mpsc::Sender<Command>,
        startup_operation: u64,
        checkpoint_records: u64,
        checkpoint_interval: Duration,
    ) -> Result<Self, String> {
        if loaded.state.recovery_semantics == engine::RecoverySemantics::Legacy2
            && loaded.sessions.control.required_semantics == engine::RecoverySemantics::Legacy2
        {
            return Err("UpgradeRequired: legacy state cannot enter the serving runtime".into());
        }
        if loaded.state.recovery_semantics != engine::RecoverySemantics::RetainedTargets3
            || loaded.sessions.control.required_semantics
                != engine::RecoverySemantics::RetainedTargets3
        {
            return Err("loaded required-semantics guards disagree".into());
        }
        config.ino_lease = loaded.initial_range;
        config.max_record_bytes = limits.max_record_bytes - store::framing::OVERHEAD;
        let journal = DaemonJournal::closed(loaded.state.covered_seq, limits);
        let mut engine =
            Engine::restore(config, journal, loaded.state).map_err(|e| e.to_string())?;
        let rows: BTreeMap<_, _> = loaded
            .sessions
            .records
            .into_iter()
            .map(|r| (r.key, r))
            .collect();
        let mut uuids = std::collections::BTreeSet::new();
        for row in rows
            .values()
            .filter(|row| row.lifecycle != SessionLifecycle::Closed)
        {
            if let Some(uuid) = row.identity.metadata.get(b"uuid".as_slice())
                && !uuids.insert(uuid.clone())
            {
                return Err("duplicate live rank UUID".into());
            }
        }
        for (seq, record) in &loaded.records {
            // Range-only records do not carry retry-table session bookkeeping.
            // Bind their missing owner from the separately guarded rank history.
            // A Closed row's cleanup sequence bounds its historical lifetime.
            for inode in &record.inodes {
                for client in inode.client_ranges.keys() {
                    if engine.session(*client).is_some() {
                        continue;
                    }
                    let row = rows
                        .values()
                        .filter(|r| {
                            r.key.client == *client
                                && engine.session_cleanup_seq(r.key).is_none()
                                && r.cleanup_seq.is_none_or(|closed| closed >= *seq)
                        })
                        .min_by_key(|r| r.cleanup_seq.unwrap_or(u64::MAX));
                    if let Some(row) = row {
                        engine
                            .install_session_key(row.key)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            engine
                .replay_record(*seq, record)
                .map_err(|e| e.to_string())?;
        }
        if engine.durable_seq() != loaded.tail {
            return Err("replay did not reach frozen tail".into());
        }
        // The adapter remains submission-closed during all checked replay.
        engine
            .journal_mut()
            .enable(loaded.tail, 0)
            .map_err(|e| format!("journal load: {e:?}"))?;
        let image = engine.checkpoint().map_err(|e| e.to_string())?;
        for history in &image.sessions {
            if rows
                .get(&history.key)
                .is_none_or(|r| r.lifecycle == SessionLifecycle::Closed)
            {
                return Err("shard history has no live rank row".into());
            }
        }
        for (key, seq) in &image.cleanup_receipts {
            if rows.get(key).is_none_or(|r| {
                r.lifecycle == SessionLifecycle::Open
                    || (r.lifecycle == SessionLifecycle::Closed && r.cleanup_seq != Some(*seq))
            }) {
                return Err("cleanup receipt/rank row mismatch".into());
            }
        }
        let mut keys = HashMap::new();
        let mut gather = std::collections::BTreeSet::new();
        let mut closing = BTreeMap::new();
        let mut receipts = Vec::new();
        for row in rows.values() {
            match row.lifecycle {
                SessionLifecycle::Open => {
                    if engine.session_cleanup_seq(row.key).is_some() {
                        return Err("Open row has cleanup receipt".into());
                    }
                    engine
                        .install_session_key(row.key)
                        .map_err(|e| e.to_string())?;
                    if keys.insert(row.key.client, row.key).is_some() {
                        return Err("duplicate live rank client".into());
                    }
                    gather.insert(row.key);
                }
                SessionLifecycle::Closing => {
                    if keys.insert(row.key.client, row.key).is_some() {
                        return Err("duplicate closing rank client".into());
                    }
                    if let Some(seq) = engine.session_cleanup_seq(row.key) {
                        receipts.push((row.key, seq));
                    } else {
                        engine
                            .install_session_key(row.key)
                            .map_err(|e| e.to_string())?;
                    }
                    closing.insert(
                        row.key,
                        Closing {
                            key: row.key,
                            generation: 0,
                            kill: true,
                            reply: false,
                            request_seq: None,
                            stage: if engine.session_cleanup_seq(row.key).is_some() {
                                CloseStage::Publishing
                            } else {
                                CloseStage::ReadyClean
                            },
                        },
                    );
                }
                SessionLifecycle::Closed => {
                    if row.cleanup_seq.is_none()
                        || row.cleanup_seq != engine.session_cleanup_seq(row.key)
                    {
                        return Err("Closed row lacks matching durable cleanup receipt".into());
                    }
                }
            }
        }
        if keys.len() > crate::session::MAX_SESSIONS {
            return Err("restored live sessions exceed the rank budget".into());
        }
        let mut runtime = Runtime {
            next_input: 0,
            receive_ordinal: None,
            owned: BTreeMap::new(),
            output_owned: Default::default(),
            retry_wait: VecDeque::new(),
            terminal_hold: Vec::new(),
            terminal_unidentified: None,
            terminal_opening: HashMap::new(),
            terminal_opening_bytes: 0,
            terminal_write: None,
            submitted_commands: BTreeMap::new(),
            capacity_epoch: engine.retry_capacity_epoch(),
            identity_probe: false,
            marker_owned: false,
            raw_controls_pending: false,
            recovery_requests_allowed: false,
            recovery_marker_allowed: gather.is_empty(),
            retry_keys: Default::default(),
            recovery_jobs: image.jobs.iter().map(|job| (job.id, job.ino)).collect(),
            required_jobs: image
                .jobs
                .iter()
                .filter(|job| {
                    matches!(job.kind, engine::LifecycleKind::RecoverSize { .. })
                        && image.inodes.iter().any(|inode| {
                            inode.core.ino == job.ino && !inode.core.client_ranges.is_empty()
                        })
                })
                .map(|job| job.id)
                .collect(),
            d6: d6::D6::default(),
            recovery: recovery::Recovery::new(&gather),
            execution_paused: false,
            ordinary_paused: true,
            owner: loaded.owner,
            tx,
            commands: VecDeque::new(),
            pending: BTreeMap::new(),
            next_operation: startup_operation,
            rows,
            keys,
            attached: HashMap::new(),
            opening: HashMap::new(),
            opening_bytes: 0,
            closing,
            gather,
            normal: VecDeque::new(),
            control: VecDeque::new(),
            normal_bytes: 0,
            control_bytes: 0,
            state: StepState::Idle,
            initialized: false,
            recovery_drain: RecoveryDrain::Reconstruction,
            recovered: false,
            checkpoint_wanted: false,
            checkpoint_busy: false,
            checkpoint_seq: loaded.checkpoint.covered_seq,
            checkpoint_records: checkpoint_records.max(1),
            checkpoint_interval,
            last_checkpoint: Instant::now(),
            pressure: false,
            blocked: false,
            busy_controls: 0,
            busy_keys: Default::default(),
        };
        runtime.command(CommandKind::TrimInitial, Pending::InitialTrim);
        for (key, seq) in receipts {
            runtime.command(
                CommandKind::Closed {
                    key,
                    cleanup_seq: seq,
                },
                Pending::Closed(key, seq),
            );
        }
        let mut server = Self::with_engine(fs_name, data_pool, epoch, engine);
        server.osd_epoch_barrier = loaded.barrier;
        server.work_paused = true;
        server.runtime = Some(runtime);
        server.durable_output = true;
        Ok(server)
    }

    pub fn owner(&self) -> OwnerToken {
        self.runtime
            .as_ref()
            .map(|r| r.owner)
            .unwrap_or(OwnerToken {
                shard_epoch: 0,
                rank: 0,
                incarnation: 0,
                gid: 0,
            })
    }
    pub fn recovered(&self) -> bool {
        self.runtime.as_ref().is_some_and(|r| r.recovered)
    }
    /// Structural completion permits REJOIN and eligible jobs, not service.
    pub fn recovery_structural_done(&self) -> bool {
        self.runtime
            .as_ref()
            .is_some_and(|r| r.recovery_drain != RecoveryDrain::Reconstruction)
    }
    fn recovery_marker_ready(&self, r: &Runtime) -> bool {
        r.recovery_drain == RecoveryDrain::Controls
            && r.recovery_marker_allowed
            && !r.marker_owned
            && r.recovery_cut_drained()
            && self.engine.is_quiescent()
            && !r
                .required_jobs
                .iter()
                .any(|id| self.engine.lifecycle_intent(*id).is_some())
    }
    fn refresh_recovery_readiness(&self, sessions: &SessionMap, r: &mut Runtime) {
        if !r.recovered
            && r.recovery_drain == RecoveryDrain::GrantsReleased
            && r.recovery_cut_drained()
            && r.retry_keys.is_empty()
            && !r.recovery.retry_pending()
            && !r.recovery.committed_retries(&self.engine)
            && !r
                .required_jobs
                .iter()
                .any(|id| self.engine.lifecycle_intent(*id).is_some())
            && self.output_wait.is_empty()
            && self.engine.is_quiescent()
            && !self.engine.journal().busy()
            && sessions.queued_output_bytes() == 0
            && !sessions.output_failed()
            && self.fatal_error().is_none()
        {
            r.recovered = true;
        }
    }
    pub fn gather_len(&self) -> usize {
        self.runtime.as_ref().map_or(0, |r| r.gather.len())
    }
    pub fn recovery_loaded(&self) -> bool {
        self.runtime
            .as_ref()
            .is_some_and(|r| r.initialized && r.closing.is_empty())
    }
    pub fn drained(&self) -> bool {
        self.engine.is_quiescent()
            && self.runtime.as_ref().is_some_and(|r| {
                r.pending.is_empty()
                    && r.commands.is_empty()
                    && r.normal.is_empty()
                    && r.control.is_empty()
            })
    }
    #[cfg(test)]
    pub fn pending_storage(&self) -> usize {
        self.runtime.as_ref().map_or(0, |r| r.pending.len())
    }
    #[cfg(test)]
    pub fn checkpoint_for_test(&self) -> Result<engine::EngineCheckpoint, engine::CheckpointError> {
        self.engine.checkpoint()
    }
    #[cfg(test)]
    pub fn retry_limits_for_test(&self) -> engine::RetryLimits {
        self.engine.config().retry_limits
    }
    pub fn input_capacity(&self, bytes: usize, control: bool) -> bool {
        self.runtime.as_ref().is_some_and(|r| {
            if control {
                r.control.len() < CONTROL_INPUTS
                    && bytes <= CONTROL_BYTES.saturating_sub(r.control_bytes)
            } else {
                r.normal.len() < INPUTS && bytes <= INPUT_BYTES.saturating_sub(r.normal_bytes)
            }
        })
    }
    /// Address, credentials and runtime generation must match the attachment
    /// whose Open row completed. A same-address transport reset needs no new
    /// durable address; a changed address or credentials must complete OPEN.
    pub fn is_attached(&self, sessions: &SessionMap, client: u64, link: &dyn ClientLink) -> bool {
        let Some(r) = &self.runtime else {
            return false;
        };
        let Some(s) = sessions.get(client) else {
            return false;
        };
        let Some(row) = r.keys.get(&client).and_then(|key| r.rows.get(key)) else {
            return false;
        };
        !r.opening.contains_key(&client)
            && s.link.connection_id() == link.connection_id()
            && self.features_allowed(row.identity.client_features)
            && matches!(
                s.state,
                SessionState::Open | SessionState::Stale | SessionState::Closing
            )
            && r.attached.get(&client) == Some(&s.generation)
            && s.addr == link.peer_addr()
            && row.identity.peer_addresses.0.contains(&s.addr)
            && link
                .authenticated_identity()
                .is_some_and(|(entity, context)| {
                    entity == row.identity.authenticated_entity
                        && context == row.identity.authorization_context
                })
    }
    pub fn has_work(&self) -> bool {
        let Some(r) = &self.runtime else {
            return false;
        };
        if self.fatal_error().is_some() {
            return false;
        }
        if !r.commands.is_empty() && r.tx.capacity() > 0 {
            return true;
        }
        if r.blocked {
            return false;
        }
        if !r.initialized
            || r.checkpoint_busy
            || (self.engine.journal().busy()
                && !(r.state == StepState::NeedsAdmission && r.identity_probe))
        {
            return false;
        }
        if r.execution_paused {
            return false;
        }
        if r.recovery.runnable(&r.opening)
            || r.recovery.drain_runnable(
                r.recovery_drain != RecoveryDrain::Reconstruction && !r.marker_owned,
                false,
                r.recovered && !r.ordinary_paused,
            )
            || r.recovery_requests_allowed
                && r.recovery_drain == RecoveryDrain::GrantsReleased
                && r.recovery.committed_retries(&self.engine)
        {
            return true;
        }
        if self.recovery_marker_ready(r) {
            return true;
        }
        if r.closing
            .values()
            .any(|c| matches!(c.stage, CloseStage::Marking))
        {
            return false;
        }
        if r.checkpoint_wanted && self.engine.is_quiescent() && !r.pressure {
            return true;
        }
        if self.engine.journal().retained() >= self.engine.journal().max_keys() {
            return false;
        }
        matches!(r.state, StepState::NeedsAdmission)
            || !r.control.is_empty()
            || r.closing
                .values()
                .any(|c| matches!(c.stage, CloseStage::ReadyClean))
            || ((r.recovered && !r.ordinary_paused
                || r.recovery_requests_allowed
                    && r.recovery_drain == RecoveryDrain::GrantsReleased)
                && !r.pressure
                && !r.checkpoint_wanted
                && matches!(r.state, StepState::Idle | StepState::WaitingInput)
                && (!r.normal.is_empty() || r.pressure_ready()))
            || r.closing.values().any(|c| {
                matches!(c.stage, CloseStage::Drain)
                    && !r.queued_for(c.key)
                    && !matches!(r.state, StepState::NeedsAdmission)
            })
    }

    fn fail_runtime(&mut self, error: JournalError, reason: impl Into<String>) {
        let write = self.engine.journal_mut().take_write();
        if let Some(r) = &mut self.runtime {
            r.terminal_write = write;
        }
        self.engine.journal_mut().fail(error);
        let _ = self.engine.journal_event(JournalEvent::Failed(error));
        self.fatal_error = Some(reason.into());
        if let Some(r) = &mut self.runtime {
            r.cancel_pending();
        }
    }

    fn check_output_failure(&mut self, sessions: &SessionMap) {
        if sessions.output_failed() && self.fatal_error().is_none() {
            self.fail_runtime(
                JournalError::Unavailable,
                "session output failed; runtime admission stopped",
            );
        }
    }

    fn consume_step(&mut self, sessions: &mut SessionMap, r: &mut Runtime, step: EngineStep) {
        self.engine.journal_mut().release(step.unused_permit);
        r.state = step.state;
        r.blocked = false;
        r.busy_controls = 0;
        r.busy_keys.clear();
        if step.state == StepState::Failed || self.engine.failure().is_some() {
            let error = self.engine.failure().unwrap_or(JournalError::Unavailable);
            r.terminal_write = self.engine.journal_mut().take_write();
            self.fail_runtime(error, format!("engine failure: {error:?}"));
            r.cancel_pending();
            return;
        }
        for effect in step.effects {
            if r.recovery_drain != RecoveryDrain::GrantsReleased
                && matches!(&effect, Effect::Caps(c) if c.op == engine::consts::CEPH_CAP_OP_GRANT)
            {
                self.fail_runtime(
                    JournalError::Unavailable,
                    "nonempty recovery drain: grant before admitted recovery marker",
                );
                return;
            }
            // Engine job state owns the committed intent. ready_jobs reads it;
            // do not retain a second, ever-growing history of completed jobs.
            match effect {
                Effect::Lifecycle(intent) => {
                    r.recovery_jobs.insert(intent.id, intent.ino);
                    if !r.recovered
                        && r.owned.values().any(|origin| origin.control)
                        && !matches!(intent.kind, engine::LifecycleKind::Purge { .. })
                    {
                        r.required_jobs.insert(intent.id);
                    }
                }
                Effect::EvictClient { client } if r.freeze_recovery_client(client) => {
                    if let Some(s) = sessions.get_mut(client) {
                        s.state = SessionState::Fencing;
                    }
                }
                effect => {
                    if let Effect::Reply(reply) = &effect
                        && let Some(key) = r.keys.get(&reply.client)
                    {
                        r.retry_keys.remove(&(*key, reply.tid));
                    }
                    self.effect_owner = r.control_owner();
                    self.effect_origin = r.control_serial();
                    self.send_effects(sessions, vec![effect]);
                    if let Some(serial) = self.effect_origin
                        && self.output_wait.iter().any(|o| o.origin == Some(serial))
                    {
                        r.output_owned.insert(serial);
                    }
                    self.effect_owner = None;
                    self.effect_origin = None;
                }
            }
            if self.fatal_error().is_some() || sessions.output_failed() {
                return;
            }
        }
        for notice in step.notices {
            match notice {
                EngineNotice::SessionCleanupCommitted { key, seq } => {
                    r.closed_origins(key);
                    if let Some(close) = r.closing.get_mut(&key) {
                        close.stage = CloseStage::Publishing;
                    } else {
                        self.fail_runtime(
                            JournalError::Corrupt,
                            "cleanup receipt without Closing lifecycle",
                        );
                        return;
                    }
                    self.pending_requests
                        .retain(|(client, _), _| *client != key.client);
                    r.command(
                        CommandKind::Closed {
                            key,
                            cleanup_seq: seq,
                        },
                        Pending::Closed(key, seq),
                    );
                }
            }
        }
        r.finish_controls();
        r.required_jobs
            .retain(|id| self.engine.lifecycle_intent(*id).is_some());
        r.recovery_jobs
            .retain(|id, _| self.engine.lifecycle_intent(*id).is_some());
        r.observe_epoch(self.engine.retry_capacity_epoch());
        r.identity_probe = r.state == StepState::NeedsAdmission;
    }

    pub fn pump(&mut self, sessions: &mut SessionMap, budget: usize) {
        self.check_output_failure(sessions);
        if self.fatal_error().is_some() || sessions.output_failed() {
            if let Some(r) = &mut self.runtime {
                r.cancel_pending();
            }
            return;
        }
        self.drive_recovery(sessions, budget);
        self.check_output_failure(sessions);
        if self.fatal_error().is_some() || sessions.output_failed() {
            if let Some(r) = &mut self.runtime {
                r.cancel_pending();
            }
            return;
        }
        for _ in 0..budget {
            let Some(held) = self.take_fresh_recovery_input() else {
                break;
            };
            let Some((link, auths)) = held.origin else {
                break;
            };
            match held.msg {
                ClientMessage::Session(msg) if msg.op == SessionOp::REQUEST_OPEN => {
                    self.queue_session_at(sessions, held.client, link, msg, auths, held.received);
                }
                msg if self.is_attached(sessions, held.client, link.as_ref()) => {
                    if let Some(mut r) = self.runtime.take() {
                        self.queue_message_at(
                            sessions,
                            &mut r,
                            held.client,
                            msg,
                            held.received,
                            Some((held.ordinal, held.bytes)),
                        );
                        self.runtime = Some(r);
                    }
                }
                _ => {}
            }
        }
        let Some(mut r) = self.runtime.take() else {
            return;
        };
        self.reap_recovery_output(sessions, &mut r);
        for _ in 0..budget {
            if self.fatal_error().is_some() {
                break;
            }
            if r.flush_commands().is_err() {
                self.fail_runtime(JournalError::Unavailable, "store worker stopped");
                break;
            }
            if let Some(Write { seq, body }) = self.engine.journal_mut().take_write() {
                r.command(CommandKind::Append { seq, body }, Pending::Append(seq));
                continue;
            }
            if !r.initialized
                || r.execution_paused
                || r.checkpoint_busy
                || matches!(r.state, StepState::WaitingCommit(_))
                || r.blocked
            {
                break;
            }
            let cleanups: Vec<_> = r
                .closing
                .values()
                .filter(|c| matches!(c.stage, CloseStage::ReadyClean))
                .map(|c| (c.key, c.kill))
                .take(CONTROL_INPUTS.saturating_sub(r.control.len()))
                .collect();
            for (key, kill) in cleanups {
                if r.enqueue(EngineInput::CloseSession { key, kill }, 64)
                    .is_err()
                {
                    break;
                }
                if let Some(close) = r.closing.get_mut(&key) {
                    close.stage = CloseStage::Cleaning;
                }
            }
            let drains: Vec<_> = r
                .closing
                .values()
                .filter(|c| matches!(c.stage, CloseStage::Drain))
                .map(|c| c.key)
                .collect();
            for key in drains {
                let queued = r.queued_for(key);
                if !queued && !matches!(r.state, StepState::NeedsAdmission) {
                    if let Some(c) = r.closing.get_mut(&key) {
                        if let Some(seq) = c.request_seq
                            && !sessions
                                .get(key.client)
                                .is_some_and(|s| s.close_ready(c.generation, seq))
                        {
                            c.stage = CloseStage::WaitSeq;
                            debug!(
                                client = key.client,
                                seq,
                                "CLOSE invalidated during drain; waiting for a current sequence or fencing"
                            );
                            continue;
                        }
                        c.stage = CloseStage::Marking;
                    }
                    r.command(CommandKind::Closing(key), Pending::Closing(key));
                }
            }
            // No new engine output may invalidate a validated CLOSE while its
            // rank Closing transition is still pending in the storage worker.
            if r.closing
                .values()
                .any(|c| matches!(c.stage, CloseStage::Marking))
            {
                break;
            }
            let threshold = r
                .checkpoint_records
                .min(self.engine.journal().max_keys().saturating_sub(4).max(1));
            if self.engine.durable_seq().saturating_sub(r.checkpoint_seq) >= threshold {
                r.checkpoint_wanted = true;
            }
            if r.checkpoint_wanted && self.engine.is_quiescent() && !r.pressure {
                match self.engine.checkpoint() {
                    Ok(state) => {
                        r.checkpoint_busy = true;
                        r.command(CommandKind::Checkpoint(state), Pending::Checkpoint);
                        continue;
                    }
                    Err(_) => break,
                }
            }
            let resume = matches!(r.state, StepState::NeedsAdmission);
            let release_marker = self.recovery_marker_ready(&r);
            let normal = (r.recovered && !r.ordinary_paused
                || r.recovery_requests_allowed
                    && r.recovery_drain == RecoveryDrain::GrantsReleased)
                && !r.pressure
                && !r.checkpoint_wanted
                && matches!(r.state, StepState::Idle | StepState::WaitingInput);
            if !resume
                && !release_marker
                && r.control.is_empty()
                && (!normal || r.normal.is_empty() && !r.pressure_ready())
            {
                break;
            }
            let permit = self.engine.journal_mut().reserve();
            if permit.is_none() && !(resume && r.identity_probe) {
                if self.engine.journal().busy() {
                    // A held permit is not WAL pressure. Keep the marker
                    // obligation until the adapter can admit a fresh step.
                    break;
                }
                r.checkpoint_wanted = true;
                if !r.pressure {
                    r.pressure = true;
                    warn!(
                        "WAL capacity reached before a quiescent checkpoint; admission paused; increase rmds_max_wal_bytes before restart"
                    );
                }
                break;
            }
            let identity_only = permit.is_none();
            if identity_only {
                r.identity_probe = false;
            }
            let control_input = !resume && !r.control.is_empty();
            if control_input && r.control.front().is_some_and(|q| {
                matches!(&q.input, EngineInput::CapUpdate { key, update } if update.dirty != 0 && r.busy_keys.contains(key))
            }) {
                if let Some(input) = r.control.pop_front() { r.control.push_back(input); }
                self.engine.journal_mut().release(permit);
                r.busy_controls += 1;
                if r.busy_controls >= r.control.len() { r.blocked = true; break; }
                continue;
            }
            let selected = if resume {
                None
            } else if release_marker {
                let input = EngineInput::RecoveryControlsDrained;
                match r.new_origin(&input, 0, true) {
                    Ok(origin) => Some(Input {
                        input,
                        bytes: 0,
                        origin,
                    }),
                    Err(()) => {
                        self.engine.journal_mut().release(permit);
                        self.fail_runtime(JournalError::Capacity, "receive ordinal exhausted");
                        break;
                    }
                }
            } else if let Some(input) = r.control.pop_front() {
                Some(input)
            } else if normal && r.pressure_ready() {
                let index = r
                    .retry_wait
                    .iter()
                    .position(|p| p.epoch != r.capacity_epoch);
                index.and_then(|i| r.retry_wait.remove(i)).map(|p| p.input)
            } else if normal {
                r.normal.pop_front()
            } else {
                self.engine.journal_mut().release(permit);
                break;
            };
            let origin = selected.as_ref().map(|i| i.origin.clone());
            let input = selected.map(|i| i.input);
            if let Some(origin) = &origin {
                r.owned.insert(origin.serial, origin.clone());
            }
            let executing_marker = (release_marker || r.marker_owned) && permit.is_some();
            match self.engine.step(input, permit) {
                Ok(step) => {
                    // The admitted marker executes before any legal grant effect
                    // is consumed. NeedsAdmission alone without a permit is not execution.
                    if executing_marker
                        && step.state != StepState::Failed
                        && self.engine.failure().is_none()
                    {
                        r.marker_owned = false;
                        r.recovery_drain = RecoveryDrain::GrantsReleased;
                        self.queue_final_opens(sessions, &mut r);
                    } else if release_marker {
                        r.marker_owned = true;
                    }
                    self.consume_step(sessions, &mut r, step);
                    if resume || identity_only {
                        r.identity_probe = false;
                    }
                }
                Err(StepError::Busy { input, permit }) => {
                    self.engine.journal_mut().release(permit);
                    let returned = input.and_then(|input| {
                        r.returned_origin(&input).map(|origin| Input {
                            bytes: origin.bytes,
                            input,
                            origin,
                        })
                    });
                    if release_marker {
                        // The engine returned ownership. Keep the obligation in
                        // Controls and wait for its earlier work, not a new marker.
                        if let Some(input) = returned {
                            r.release_origin(input.origin);
                        }
                    } else if control_input && let Some(input) = returned {
                        // Only this input is blocked. Try every other queued
                        // control once, across bounded scheduler turns if needed.
                        if let EngineInput::CapUpdate { key, .. } = &input.input {
                            r.busy_keys.insert(*key);
                        }
                        r.control.push_back(input);
                        r.busy_controls += 1;
                        if r.busy_controls < r.control.len() {
                            continue;
                        }
                    } else if let Some(input) = returned {
                        r.normal.push_front(input);
                    }
                    r.blocked = true;
                    break;
                }
                Err(StepError::RetryCapacity { input, permit }) => {
                    self.engine.journal_mut().release(permit);
                    if let Some(input) = input {
                        if let Some(origin) = r.returned_origin(&input) {
                            r.retry_wait.push_back(ownership::Pressure {
                                input: Input {
                                    bytes: origin.bytes,
                                    input,
                                    origin,
                                },
                                epoch: self.engine.retry_capacity_epoch(),
                            });
                        } else {
                            self.fail_runtime(
                                JournalError::Corrupt,
                                "capacity return without input origin",
                            );
                            break;
                        }
                    }
                    r.observe_epoch(self.engine.retry_capacity_epoch());
                }
                Err(StepError::InvalidSession { input, permit }) => {
                    self.engine.journal_mut().release(permit);
                    if let Some(input) = input {
                        let obsolete = match &input {
                            EngineInput::Request { key, .. }
                            | EngineInput::CapUpdate { key, .. }
                            | EngineInput::CapRelease { key, .. }
                            | EngineInput::CloseSession { key, .. } => {
                                r.rows.get(key).is_some_and(|row| {
                                    row.lifecycle == SessionLifecycle::Closed
                                        && row.cleanup_seq == self.engine.session_cleanup_seq(*key)
                                })
                            }
                            _ => false,
                        };
                        if let Some(origin) = r.returned_origin(&input) {
                            if obsolete {
                                r.release_origin(origin);
                            } else {
                                r.terminal_hold.push(Input {
                                    bytes: origin.bytes,
                                    input,
                                    origin,
                                });
                                self.fail_runtime(
                                    JournalError::Corrupt,
                                    "current input returned InvalidSession",
                                );
                                break;
                            }
                        } else {
                            self.fail_runtime(
                                JournalError::Corrupt,
                                "session return without input origin",
                            );
                            break;
                        }
                    }
                }
                Err(StepError::ControlCapacity {
                    input,
                    permit,
                    required_completions,
                    required_bytes,
                    required_transient_bytes,
                }) => {
                    self.hold_terminal(&mut r, input);
                    self.engine.journal_mut().release(permit);
                    let limits = self.engine.config().retry_limits;
                    warn!(max_completions = limits.max_completions, max_bytes = limits.max_bytes,
                        active_completions = ?limits.max_completions.checked_add(1), active_bytes = ?limits.max_bytes.checked_add(8 << 20),
                        ?required_completions, ?required_bytes, ?required_transient_bytes,
                        "terminal retry capacity; None denotes overflow; same-identity reviewed larger-limit restart required");
                    r.terminal_write = self.engine.journal_mut().take_write();
                    self.fail_runtime(
                        JournalError::Capacity,
                        "terminal protected-control retry capacity",
                    );
                    r.cancel_pending();
                    break;
                }
                Err(StepError::InvalidRetryIdentity { input, permit }) => {
                    self.hold_terminal(&mut r, input);
                    self.engine.journal_mut().release(permit);
                    r.terminal_write = self.engine.journal_mut().take_write();
                    let error = self.engine.failure().unwrap_or(JournalError::Corrupt);
                    self.fail_runtime(
                        error,
                        "invalid retry identity; exact returned input retained",
                    );
                    r.cancel_pending();
                    break;
                }
                Err(StepError::UpgradeRequired { input, permit }) => {
                    self.hold_terminal(&mut r, input);
                    self.engine.journal_mut().release(permit);
                    r.terminal_write = self.engine.journal_mut().take_write();
                    self.fail_runtime(
                        JournalError::Unavailable,
                        "UpgradeRequired at runtime; service remains closed",
                    );
                    r.cancel_pending();
                    break;
                }
            }
            self.check_output_failure(sessions);
            if self.fatal_error().is_some() {
                break;
            }
        }
        if self.fatal_error().is_none() && !sessions.output_failed() {
            // A step on the final budget slot can submit a record. Transfer it
            // now so it cannot wait for an unrelated event to wake the outbox.
            if let Some(Write { seq, body }) = self.engine.journal_mut().take_write() {
                r.command(CommandKind::Append { seq, body }, Pending::Append(seq));
            }
            if r.flush_commands().is_err() {
                self.fail_runtime(JournalError::Unavailable, "store worker stopped");
                r.cancel_pending();
            }
        } else {
            r.cancel_pending();
        }
        self.reap_recovery_output(sessions, &mut r);
        self.refresh_recovery_readiness(sessions, &mut r);
        self.runtime = Some(r);
    }

    fn hold_terminal(&mut self, r: &mut Runtime, input: Option<EngineInput>) {
        if let Some(input) = input {
            if let Some(origin) = r.returned_origin(&input) {
                r.terminal_hold.push(Input {
                    bytes: origin.bytes,
                    input,
                    origin,
                });
            } else {
                // Internal origins have no external payload/charge. An external
                // return without its registry entry is an integration failure.
                self.fatal_error = Some("terminal return without original input ownership".into());
            }
        }
    }

    pub fn on_storage_event(
        &mut self,
        sessions: &mut SessionMap,
        owner: OwnerToken,
        operation: u64,
        event: EventKind,
    ) {
        if self.fatal_error().is_some() {
            return;
        }
        let Some(mut r) = self.runtime.take() else {
            return;
        };
        if r.owner != owner {
            self.runtime = Some(r);
            return;
        }
        let Some(pending) = r.pending.remove(&operation) else {
            self.runtime = Some(r);
            return;
        };
        let failed = matches!(event, EventKind::Failed(_));
        if !failed {
            r.submitted_commands.remove(&operation);
        }
        r.blocked = false;
        r.busy_controls = 0;
        match (pending, event) {
            (_, EventKind::Failed(error)) => self.fail_runtime(
                if error == store::StoreError::Fenced {
                    JournalError::Fenced
                } else {
                    JournalError::Unavailable
                },
                format!("store failure: {error}"),
            ),
            (Pending::Checkpoint, EventKind::Capacity(reason)) => {
                r.checkpoint_busy = false;
                r.pressure = true;
                warn!(
                    reason,
                    "checkpoint capacity: admission paused; increase retained-generation budget before restart"
                );
            }
            (_, EventKind::Capacity(reason)) => self.fail_runtime(JournalError::Capacity, reason),
            (Pending::InitialTrim, EventKind::InitialTrimmed(seq)) => {
                if self.engine.journal_mut().trimmed(seq).is_err() {
                    self.fail_runtime(JournalError::Corrupt, "invalid initial trim position");
                } else {
                    r.initialized = true;
                }
            }
            (Pending::Append(expected), EventKind::Committed(seq)) if expected == seq => {
                if self.engine.journal_mut().complete(seq).is_err() {
                    self.fail_runtime(JournalError::Corrupt, "invalid committed prefix");
                } else {
                    let step = self.engine.journal_event(JournalEvent::Committed(seq));
                    self.consume_step(sessions, &mut r, step);
                }
            }
            (Pending::Open(client), EventKind::Opened(row)) if client == row.key.client => {
                if let Some(open) = r.opening.remove(&client) {
                    r.opening_bytes -= open.bytes;
                    if matches!(open.purpose, recovery::OpenPurpose::Recovery { .. }) {
                        self.recovery_opened(sessions, &mut r, open, row);
                        self.runtime = Some(r);
                        self.pump(sessions, 32);
                        return;
                    }
                    let same_generation =
                        sessions.get(client).map(|s| s.generation) == open.generation;
                    if !same_generation {
                        self.fail_runtime(
                            JournalError::Corrupt,
                            "stale runtime generation in durable Open completion",
                        );
                    } else if self.engine.install_session_key(row.key).is_err() {
                        self.fail_runtime(JournalError::Corrupt, "durable Open key mismatch");
                    } else {
                        let features_denied = !self.features_allowed(row.identity.client_features);
                        let reclaiming = r.reclaiming(client);
                        let durable_open = (row.key, row.identity.open_id);
                        if let Some(s) = sessions.get_mut(client) {
                            // Even a refused new attachment changes the durable
                            // OPEN decision. Earlier queued reclaim output is obsolete.
                            s.durable_open = Some(durable_open);
                        }
                        r.keys.insert(client, row.key);
                        r.rows.insert(row.key, row);
                        let blocklisted = self.is_blocklisted(&open.link.peer_addr());
                        if reclaiming {
                            if let Some(s) = sessions.get_mut(client) {
                                s.state = SessionState::Fencing;
                            }
                        } else if blocklisted || features_denied {
                            let same = sessions
                                .get(client)
                                .is_none_or(|s| s.addr == open.link.peer_addr());
                            if same {
                                sessions.install_silent(
                                    client,
                                    open.link,
                                    open.msg,
                                    open.cap_auths,
                                );
                                if let Some(s) = sessions.get_mut(client) {
                                    s.send(&if blocklisted {
                                        MClientSession::reject_blocklisted()
                                    } else {
                                        MClientSession::reject(
                                            "missing required CephFS features",
                                            false,
                                        )
                                    });
                                }
                                self.request_eviction(sessions, client);
                            } else {
                                if blocklisted {
                                    sessions.reject_blocklisted(client, open.link);
                                } else {
                                    sessions.recovery_terminal(
                                        client,
                                        open.link,
                                        MClientSession::reject(
                                            "missing required CephFS features",
                                            false,
                                        ),
                                    );
                                }
                                if let Some(s) = sessions.get_mut(client)
                                    && s.state == SessionState::Opening
                                {
                                    s.state = SessionState::Open;
                                }
                            }
                        } else if sessions
                            .get(client)
                            .is_none_or(|s| s.state == SessionState::Opening)
                        {
                            sessions.publish_open(client, open.link, open.msg, open.cap_auths);
                            if let Some(s) = sessions.get_mut(client) {
                                s.durable_open = Some(durable_open);
                                r.attached.insert(client, s.generation);
                            }
                        }
                    }
                }
            }
            (Pending::Closing(key), EventKind::Closing(actual)) if key == actual => {
                if let Some(row) = r.rows.get_mut(&key) {
                    row.lifecycle = SessionLifecycle::Closing;
                }
                if let Some(close) = r.closing.get_mut(&key) {
                    close.stage = CloseStage::ReadyClean;
                }
            }
            (
                Pending::Closed(key, seq),
                EventKind::Closed {
                    key: actual,
                    cleanup_seq,
                },
            ) if key == actual && seq == cleanup_seq => {
                if self.engine.session_cleanup_seq(key) != Some(seq) {
                    self.fail_runtime(
                        JournalError::Corrupt,
                        "rank Closed lacks shard cleanup receipt",
                    );
                } else {
                    if let Some(row) = r.rows.get_mut(&key) {
                        row.lifecycle = SessionLifecycle::Closed;
                        row.cleanup_seq = Some(seq);
                    }
                    r.keys.remove(&key.client);
                    r.recovery.closed(key);
                    r.attached.remove(&key.client);
                    r.gather.remove(&key);
                    if let Some(close) = r.closing.remove(&key) {
                        sessions.complete_close(key.client, close.generation, close.reply);
                    }
                    r.finish_reclaims(
                        sessions,
                        &self.required_client_features,
                        self.objecter.as_ref(),
                    );
                }
            }
            (Pending::Checkpoint, EventKind::Checkpoint(descriptor)) => {
                if self
                    .engine
                    .journal_mut()
                    .trimmed(descriptor.covered_seq)
                    .is_err()
                {
                    self.fail_runtime(JournalError::Corrupt, "checkpoint coverage mismatch");
                } else {
                    r.checkpoint_seq = descriptor.covered_seq;
                    r.checkpoint_busy = false;
                    r.checkpoint_wanted = false;
                    r.last_checkpoint = Instant::now();
                }
            }
            _ => self.fail_runtime(JournalError::Corrupt, "storage completion kind mismatch"),
        }
        if self
            .mdlog_flush_target
            .is_some_and(|seq| self.engine.durable_seq() >= seq)
        {
            self.mdlog_flush_target = None;
        }
        if self.fatal_error().is_some() || sessions.output_failed() {
            r.cancel_pending();
        }
        self.runtime = Some(r);
        // Complete, materialize output, then resume only in a new admitted step.
        self.pump(sessions, 32);
    }

    pub fn try_finish_recovery(&mut self, sessions: &mut SessionMap) -> bool {
        if self.fatal_error().is_some() || sessions.output_failed() {
            return false;
        }
        let Some(mut r) = self.runtime.take() else {
            return false;
        };
        if r.recovery_drain == RecoveryDrain::Reconstruction
            && r.initialized
            && r.gather.is_empty()
            && r.closing.is_empty()
            && r.recovery.structural_ready(&r.rows, &r.opening)
            && !self.engine.journal().busy()
        {
            let keys: Vec<_> = r
                .rows
                .values()
                .filter(|row| row.lifecycle == SessionLifecycle::Open)
                .map(|row| row.key)
                .collect();
            if let Ok(step) = self.engine.finish_reconnect(&keys) {
                r.recovery_drain = RecoveryDrain::Controls;
                self.consume_step(sessions, &mut r, step);
            }
        }
        self.reap_recovery_output(sessions, &mut r);
        self.refresh_recovery_readiness(sessions, &mut r);
        let ready = r.recovered && self.fatal_error().is_none();
        self.runtime = Some(r);
        ready
    }
    pub fn request_checkpoint(&mut self) {
        if let Some(r) = &mut self.runtime {
            r.checkpoint_wanted = true;
        }
    }
    pub fn flush_mdlog(&mut self, _: &mut SessionMap) {
        self.mdlog_flush_target = Some(
            self.mdlog_flush_target
                .unwrap_or(0)
                .max(self.engine.submitted_seq()),
        );
        self.engine.journal_mut().flush();
    }

    #[cfg(test)]
    pub fn handle_session(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: MClientSession,
        cap_auths: Vec<MdsCapAuth>,
    ) {
        self.handle_session_at(sessions, client, link, msg, cap_auths, Instant::now());
    }
    pub(super) fn handle_session_at(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: MClientSession,
        cap_auths: Vec<MdsCapAuth>,
        received: Instant,
    ) {
        self.queue_session_at(sessions, client, link, msg, cap_auths, received);
        self.pump(sessions, 32);
    }
    fn queue_session_at(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: MClientSession,
        cap_auths: Vec<MdsCapAuth>,
        received: Instant,
    ) {
        if self.fatal_error().is_some() || sessions.output_failed() {
            return;
        }
        match msg.op {
            SessionOp::REQUEST_OPEN => {
                if !self.features_allowed(msg.supported_features.first_word()) {
                    sessions.recovery_terminal(
                        client,
                        link,
                        MClientSession::reject("missing required CephFS features", false),
                    );
                    return;
                }
                let Some(r) = &mut self.runtime else {
                    return;
                };
                if sessions.terminal_close_pending(client)
                    || r.reclaiming(client)
                    || r.opening.contains_key(&client)
                    || r.keys
                        .get(&client)
                        .is_some_and(|key| r.closing.contains_key(key))
                    || sessions.get(client).is_some_and(|s| {
                        matches!(s.state, SessionState::Fencing | SessionState::Closing)
                    })
                {
                    return;
                }
                if msg
                    .metadata
                    .get("uuid")
                    .is_some_and(|uuid| r.uuid_conflict(client, uuid))
                {
                    sessions.recovery_terminal(
                        client,
                        link,
                        MClientSession::reject("duplicated session uuid", false),
                    );
                    self.check_output_failure(sessions);
                    return;
                }
                let new_session = !r.keys.contains_key(&client);
                if new_session && r.session_slots() >= crate::session::MAX_SESSIONS {
                    debug!(
                        client,
                        "rank session slots full; OPEN dropped before repository mutation"
                    );
                    return;
                }
                let Some((authenticated_entity, authorization_context)) =
                    link.authenticated_identity()
                else {
                    self.protocol_error(client);
                    return;
                };
                let bytes =
                    msg.to_raw(link.features()).front.len() + authorization_context.len() + 256;
                if r.opening.len() >= INPUTS || bytes > INPUT_BYTES.saturating_sub(r.opening_bytes)
                {
                    self.protocol_error(client);
                    return;
                }
                let mut addresses = r
                    .keys
                    .get(&client)
                    .and_then(|key| r.rows.get(key))
                    .map(|row| row.identity.peer_addresses.clone())
                    .unwrap_or_default();
                if !addresses.0.contains(&link.peer_addr()) {
                    addresses.0.push(link.peer_addr());
                }
                r.next_operation += 1;
                let record = store::NewSessionRecord {
                    client,
                    open_id: persistence::operation_id(r.owner.gid, r.next_operation),
                    authenticated_entity,
                    authorization_context,
                    peer_addresses: addresses,
                    client_features: msg.supported_features.first_word(),
                    server_features: proto::features::MDS_SUPPORTED_FEATURES,
                    metadata: msg
                        .metadata
                        .iter()
                        .map(|(k, v)| {
                            (
                                Bytes::copy_from_slice(k.as_bytes()),
                                Bytes::copy_from_slice(v.as_bytes()),
                            )
                        })
                        .collect(),
                };
                r.opening_bytes += bytes;
                r.opening.insert(
                    client,
                    Opening {
                        link,
                        msg,
                        cap_auths,
                        bytes,
                        generation: sessions.get(client).map(|s| s.generation),
                        new_session,
                        purpose: recovery::OpenPurpose::Ordinary,
                    },
                );
                if let Some(s) = sessions.get_mut(client) {
                    s.state = SessionState::Opening;
                }
                r.command(CommandKind::Open(record), Pending::Open(client));
            }
            SessionOp::REQUEST_CLOSE => {
                let Some(generation) = sessions.begin_close(client, msg.seq) else {
                    return;
                };
                if let Some(r) = &mut self.runtime
                    && let Some(key) = r.keys.get(&client).copied()
                {
                    // Requests still outside the engine have not mutated state. A
                    // closing client abandons them; dirty cap inputs must drain.
                    r.cancel_requests(key);
                    r.blocked = false;
                    r.busy_controls = 0;
                    r.closing
                        .entry(key)
                        .and_modify(|close| {
                            if !close.kill
                                && matches!(close.stage, CloseStage::Drain | CloseStage::WaitSeq)
                            {
                                close.request_seq = Some(msg.seq);
                                close.stage = CloseStage::Drain;
                            }
                        })
                        .or_insert(Closing {
                            key,
                            generation,
                            kill: false,
                            reply: true,
                            request_seq: Some(msg.seq),
                            stage: CloseStage::Drain,
                        });
                }
            }
            SessionOp::REQUEST_FLUSH_MDLOG => {
                if let Some(s) = sessions.get_mut(client).filter(|s| s.is_open()) {
                    s.last_renew = s.last_renew.max(received);
                }
                self.flush_mdlog(sessions);
            }
            SessionOp::REQUEST_RENEWCAPS => {
                sessions.renew_at(client, msg.seq, received);
            }
            SessionOp::FLUSHMSG_ACK => {
                if let Some(s) = sessions.get_mut(client).filter(|s| s.is_open()) {
                    s.last_renew = s.last_renew.max(received);
                }
            }
            _ => {
                let outcome = sessions.handle(client, link, msg, cap_auths);
                if outcome == SessionOutcome::ProtocolError {
                    self.protocol_error(client);
                }
                // Renewal changes only wire-session time. Engine rights remain until
                // an explicit acknowledged release or verified durable eviction.
            }
        }
    }

    pub fn finish_eviction(
        &mut self,
        sessions: &mut SessionMap,
        request: Eviction,
        epoch: u32,
    ) -> bool {
        if sessions.get(request.client).is_none_or(|s| {
            s.generation != request.session_generation
                || s.addr != request.addr
                || s.state != SessionState::Fencing
        }) {
            return false;
        }
        self.raise_epoch_barrier(epoch);
        if let Some(r) = &mut self.runtime
            && let Some(key) = r.keys.get(&request.client).copied()
        {
            r.cancel_requests(key);
            r.blocked = false;
            r.busy_controls = 0;
            r.closing
                .entry(key)
                .and_modify(|close| {
                    let reply = matches!(close.stage, CloseStage::WaitFence);
                    close.kill = true;
                    close.reply = reply;
                    close.request_seq = None;
                    if matches!(close.stage, CloseStage::WaitSeq | CloseStage::WaitFence) {
                        close.stage = CloseStage::Drain;
                    }
                })
                .or_insert(Closing {
                    key,
                    generation: request.session_generation,
                    kill: true,
                    reply: false,
                    request_seq: None,
                    stage: CloseStage::Drain,
                });
        }
        self.pump(sessions, 32);
        true
    }

    pub fn fence_addresses(&self, client: u64) -> Vec<denc::EntityAddr> {
        self.runtime
            .as_ref()
            .and_then(|r| r.keys.get(&client).and_then(|key| r.rows.get(key)))
            .map(|row| row.identity.peer_addresses.0.clone())
            .unwrap_or_default()
    }
    /// C++ Server::handle_client_reconnect: checked late/Closed keys get CLOSE;
    /// unknown keys get the sessionless REJECT. Current rights still need fencing.
    pub fn late_reconnect(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
    ) {
        let Some(r) = &mut self.runtime else {
            return;
        };
        let known = r.rows.values().any(|row| row.key.client == client);
        let current = link.connection_id() != 0
            && sessions
                .get(client)
                .is_some_and(|s| s.link.connection_id() == link.connection_id());
        if known
            && current
            && let Some(key) = r.keys.get(&client).copied()
        {
            let fence = if let Some(close) = r.closing.get_mut(&key) {
                close.reply = true;
                if matches!(close.stage, CloseStage::Drain | CloseStage::WaitSeq) {
                    close.stage = CloseStage::WaitFence;
                    true
                } else {
                    false
                }
            } else {
                r.closing.insert(
                    key,
                    Closing {
                        key,
                        generation: sessions.get(client).map_or(0, |s| s.generation),
                        kill: true,
                        reply: true,
                        request_seq: None,
                        stage: CloseStage::WaitFence,
                    },
                );
                true
            };
            r.cancel_requests(key);
            if fence {
                self.request_eviction(sessions, client);
            }
            return;
        }
        let reply = if known {
            MClientSession::close()
        } else if self.is_blocklisted(&link.peer_addr()) {
            MClientSession::reject_blocklisted()
        } else {
            MClientSession::reject("unknown recovery session", false)
        };
        sessions.recovery_terminal(client, link, reply);
    }
    pub fn ready_jobs(&self) -> Vec<LifecycleIntent> {
        if self.fatal_error().is_some() || !self.recovery_structural_done() || self.work_paused {
            return Vec::new();
        }
        self.engine
            .pending_jobs()
            .into_iter()
            .take(INPUTS)
            .collect()
    }
    pub fn has_job(&self, id: u64) -> bool {
        self.engine.lifecycle_intent(id).is_some()
    }
    pub fn lifecycle_completed(&mut self, id: u64, result: engine::LifecycleResult) {
        if self.fatal_error().is_some() {
            return;
        }
        if let Some(r) = &mut self.runtime
            && r.enqueue(EngineInput::LifecycleCompleted { id, result }, 64)
                .is_err()
        {
            self.fail_runtime(JournalError::Capacity, "lifecycle completion capacity");
        }
    }

    #[cfg(test)]
    pub fn handle(&mut self, sessions: &mut SessionMap, client: u64, msg: ClientMessage) {
        self.handle_at(sessions, client, msg, Instant::now());
    }
    pub(super) fn handle_at(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        msg: ClientMessage,
        received: Instant,
    ) {
        if let ClientMessage::Reclaim(msg) = msg {
            if let Some(key) = self
                .runtime
                .as_ref()
                .and_then(|r| r.keys.get(&client))
                .copied()
            {
                self.reclaim(sessions, key, msg, received);
            }
            return;
        }
        let Some(mut r) = self.runtime.take() else {
            return;
        };
        self.queue_message_at(sessions, &mut r, client, msg, received, None);
        self.runtime = Some(r);
        self.pump(sessions, 32);
    }
    fn queue_message_at(
        &mut self,
        sessions: &mut SessionMap,
        r: &mut Runtime,
        client: u64,
        msg: ClientMessage,
        received: Instant,
        retained: Option<(u64, usize)>,
    ) {
        if self.fatal_error().is_some() {
            return;
        }
        let key = r.keys.get(&client).copied();
        let Some(key) = key else {
            return;
        };
        let (input, bytes) = match msg {
            ClientMessage::Request(req) => {
                if self.pending_requests.contains_key(&(client, req.tid)) {
                    return;
                }
                if self.pending_requests.len() >= MAX_PENDING_REQUESTS {
                    self.fail_runtime(JournalError::Capacity, "request context capacity");
                    return;
                }
                let features = sessions.get(client).map(|s| s.link.features()).unwrap_or(0);
                let mut translated = match request_from_proto(&req, now_ts(), features) {
                    Ok(request) => request,
                    Err(_) => {
                        if let Some(s) = sessions.get_mut(client) {
                            let mut reply = MClientReply::new(req.tid, req.op(), -22);
                            reply.mdsmap_epoch = self.mdsmap_epoch;
                            if reply.check_invariants_for(&req).is_empty() {
                                s.send(&reply);
                            } else {
                                self.protocol_error(client);
                            }
                        }
                        return;
                    }
                };
                if self.engine.session(client).is_some_and(|s| {
                    s.epoch == key.incarnation
                        && s.completed_requests
                            .get(&req.tid)
                            .is_some_and(|c| c.op == req.op().0)
                }) {
                    translated.releases.clear();
                }
                let bytes = req.to_raw(features).front.len() + req.data.len() + 256;
                let mut context = proto::MClientRequest::new(req.tid, req.op());
                context.head = req.head.clone();
                context.path = WireFilePath::new(
                    req.path.ino,
                    match req.path.components().as_slice() {
                        [] => "",
                        [[]] => "/",
                        [_] => "x",
                        _ => "x/y",
                    },
                );
                self.pending_requests.insert((client, req.tid), context);
                if let Some(s) = sessions.get(client) {
                    self.request_owners
                        .insert((client, req.tid), crate::session::OutputOwner::current(s));
                }
                (
                    EngineInput::Request {
                        key,
                        request: translated,
                    },
                    bytes,
                )
            }
            ClientMessage::Caps(msg) => {
                if r.recovery_drain == RecoveryDrain::Controls {
                    r.required_jobs.extend(
                        r.recovery_jobs
                            .iter()
                            .filter_map(|(id, ino)| (*ino == msg.ino).then_some(*id)),
                    );
                }
                if !matches!(
                    msg.op,
                    CapOp::UPDATE
                        | CapOp::FLUSH
                        | CapOp::DROP
                        | CapOp::RELEASE
                        | CapOp::RENEW
                        | CapOp::FLUSHSNAP
                ) {
                    self.protocol_error(client);
                    return;
                }
                let update = match cap_update_from_proto(&msg) {
                    Ok(update) => update,
                    Err(_) => {
                        self.protocol_error(client);
                        return;
                    }
                };
                self.raise_epoch_barrier(msg.osd_epoch_barrier);
                (
                    EngineInput::CapUpdate { key, update },
                    msg.xattrbl.len() + 512,
                )
            }
            ClientMessage::CapRelease(msg) => {
                self.raise_epoch_barrier(msg.osd_epoch_barrier);
                let bytes = msg.caps.len() * 32 + 64;
                (
                    EngineInput::CapRelease {
                        key,
                        items: msg.caps.iter().map(cap_item).collect(),
                    },
                    bytes,
                )
            }
            ClientMessage::Reclaim(_) => return,
            ClientMessage::Metrics(_) => {
                if let Some(s) = sessions.get_mut(client) {
                    s.last_renew = s.last_renew.max(received);
                }
                return;
            }
            ClientMessage::Reconnect(_) | ClientMessage::Lease(_) | ClientMessage::Session(_) => {
                return;
            }
        };
        if !matches!(&input, EngineInput::Request { request, .. } if matches!(request.op, Op::Other { .. }))
            && let Some(s) = sessions.get_mut(client)
        {
            s.last_renew = s.last_renew.max(received);
        }
        {
            let bytes = retained.map_or(bytes, |(_, bytes)| bytes);
            if r.enqueue(input, bytes).is_err() {
                if let Some(q) = r.terminal_hold.last_mut() {
                    q.origin.received = received;
                    if let Some((ordinal, _)) = retained {
                        q.origin.ordinal = ordinal;
                    }
                    q.origin.owner = sessions
                        .get(client)
                        .map(crate::session::OutputOwner::current);
                }
                self.fail_runtime(
                    JournalError::Capacity,
                    "input queue capacity before engine mutation",
                );
                return;
            }
            let queued = r
                .control
                .back_mut()
                .filter(|q| q.origin.serial == r.next_input)
                .or_else(|| {
                    r.normal
                        .back_mut()
                        .filter(|q| q.origin.serial == r.next_input)
                });
            if let Some(q) = queued {
                if let Some((ordinal, _)) = retained {
                    q.origin.ordinal = ordinal;
                }
                q.origin.received = received;
                q.origin.owner = sessions
                    .get(client)
                    .map(crate::session::OutputOwner::current);
            }
        }
    }

    #[cfg(test)]
    pub fn tick(&mut self, sessions: &mut SessionMap) {
        self.tick_serving(sessions, true);
    }
    pub fn tick_serving(&mut self, sessions: &mut SessionMap, active: bool) {
        if self.fatal_error().is_some() {
            return;
        }
        if let Some(r) = &mut self.runtime {
            if r.last_checkpoint.elapsed() >= r.checkpoint_interval
                && self.engine.durable_seq() > r.checkpoint_seq
            {
                r.checkpoint_wanted = true;
            }
            if !r
                .control
                .iter()
                .any(|q| matches!(q.input, EngineInput::Tick))
            {
                let _ = r.enqueue(EngineInput::Tick, 64);
            }
        }
        if active {
            self.active_timers(sessions, Instant::now());
        }
        self.pump(sessions, 32);
    }
}
