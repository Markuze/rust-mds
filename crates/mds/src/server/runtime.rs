//! Production one-record execution and durable session lifecycle. Compatibility
//! helpers in the parent module are compiled only for MemJournal unit tests.

#[cfg(test)]
mod tests;

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
}
struct Opening {
    link: Box<dyn ClientLink>,
    msg: MClientSession,
    cap_auths: Vec<MdsCapAuth>,
    bytes: usize,
    generation: Option<u64>,
    new_session: bool,
}
#[derive(Clone, Copy)]
enum CloseStage {
    Drain,
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

pub(super) struct Runtime {
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
}

impl Runtime {
    fn session_slots(&self) -> usize {
        self.keys.len()
            + self
                .opening
                .values()
                .filter(|open| open.new_session)
                .count()
    }
    pub(super) fn cancel_pending(&mut self) {
        self.commands.clear();
        self.pending.clear();
        self.opening.clear();
        self.opening_bytes = 0;
        self.normal.clear();
        self.control.clear();
        self.normal_bytes = 0;
        self.control_bytes = 0;
        self.busy_controls = 0;
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
    fn cancel_requests(&mut self, key: SessionKey) {
        self.normal.retain(|q| {
            if matches!(&q.input, EngineInput::Request { key: k, .. } if *k == key) {
                self.normal_bytes -= q.bytes;
                false
            } else {
                true
            }
        });
    }
    fn command(&mut self, kind: CommandKind, pending: Pending) {
        self.next_operation += 1;
        self.pending.insert(self.next_operation, pending);
        self.commands.push_back(Command {
            owner: self.owner,
            operation: self.next_operation,
            kind,
        });
    }
    fn flush_commands(&mut self) -> Result<(), ()> {
        while let Some(command) = self.commands.pop_front() {
            match self.tx.try_send(command) {
                Ok(()) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Full(command)) => {
                    self.commands.push_front(command);
                    break;
                }
                Err(_) => return Err(()),
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
        if queue.len() >= max_count || bytes > max_bytes.saturating_sub(*used) {
            return Err(());
        }
        *used += bytes;
        queue.push_back(Input { input, bytes });
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
        if !r.initialized || r.checkpoint_busy || self.engine.journal().busy() {
            return false;
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
            || (r.recovered
                && !self.work_paused
                && !r.pressure
                && !r.checkpoint_wanted
                && matches!(r.state, StepState::Idle)
                && !r.normal.is_empty())
            || r.closing.values().any(|c| {
                matches!(c.stage, CloseStage::Drain)
                    && !r.queued_for(c.key)
                    && !matches!(r.state, StepState::NeedsAdmission)
            })
    }

    fn fail_runtime(&mut self, error: JournalError, reason: impl Into<String>) {
        self.engine.journal_mut().fail(error);
        let _ = self.engine.journal_event(JournalEvent::Failed(error));
        self.fatal_error = Some(reason.into());
        if let Some(r) = &mut self.runtime {
            r.cancel_pending();
        }
    }

    fn consume_step(&mut self, sessions: &mut SessionMap, r: &mut Runtime, step: EngineStep) {
        self.engine.journal_mut().release(step.unused_permit);
        r.state = step.state;
        r.blocked = false;
        r.busy_controls = 0;
        if step.state == StepState::Failed {
            self.fail_runtime(JournalError::Unavailable, "engine journal failure");
            return;
        }
        for effect in step.effects {
            // Engine job state owns the committed intent. ready_jobs reads it;
            // do not retain a second, ever-growing history of completed jobs.
            if let Effect::Lifecycle(_) = effect {
            } else {
                self.send_effects(sessions, vec![effect]);
            }
            if self.fatal_error().is_some() || sessions.output_failed() {
                return;
            }
        }
        for notice in step.notices {
            match notice {
                EngineNotice::SessionCleanupCommitted { key, seq } => {
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
    }

    pub fn pump(&mut self, sessions: &mut SessionMap, budget: usize) {
        if self.fatal_error().is_some() || sessions.output_failed() {
            if let Some(r) = &mut self.runtime {
                r.cancel_pending();
            }
            return;
        }
        let Some(mut r) = self.runtime.take() else {
            return;
        };
        for _ in 0..budget {
            if r.flush_commands().is_err() {
                self.fail_runtime(JournalError::Unavailable, "store worker stopped");
                break;
            }
            if let Some(Write { seq, body }) = self.engine.journal_mut().take_write() {
                r.command(CommandKind::Append { seq, body }, Pending::Append(seq));
                continue;
            }
            if !r.initialized
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
            let normal = r.recovered
                && !self.work_paused
                && !r.pressure
                && !r.checkpoint_wanted
                && matches!(r.state, StepState::Idle);
            if !resume && r.control.is_empty() && (!normal || r.normal.is_empty()) {
                break;
            }
            let Some(permit) = self.engine.journal_mut().reserve() else {
                r.checkpoint_wanted = true;
                if !r.pressure && !self.engine.journal().busy() {
                    r.pressure = true;
                    warn!(
                        "WAL capacity reached before a quiescent checkpoint; admission paused; increase rmds_max_wal_bytes before restart"
                    );
                }
                break;
            };
            let control_input = !resume && !r.control.is_empty();
            let (input, bytes) = if resume {
                (None, 0)
            } else if let Some(input) = r.control.pop_front() {
                r.control_bytes -= input.bytes;
                (Some(input.input), input.bytes)
            } else if let Some(input) = r.normal.pop_front() {
                r.normal_bytes -= input.bytes;
                (Some(input.input), input.bytes)
            } else {
                self.engine.journal_mut().release(Some(permit));
                break;
            };
            match self.engine.step(input, Some(permit)) {
                Ok(step) => self.consume_step(sessions, &mut r, step),
                Err(StepError::Busy { input, permit }) => {
                    self.engine.journal_mut().release(permit);
                    if control_input && let Some(input) = input {
                        // Only this input is blocked. Try every other queued
                        // control once, across bounded scheduler turns if needed.
                        r.control.push_back(Input { input, bytes });
                        r.control_bytes += bytes;
                        r.busy_controls += 1;
                        if r.busy_controls < r.control.len() {
                            continue;
                        }
                    } else if let Some(input) = input
                        && r.enqueue(input, bytes).is_err()
                    {
                        self.fail_runtime(JournalError::Capacity, "returned input over budget");
                    }
                    r.blocked = true;
                    break;
                }
                Err(StepError::InvalidSession { permit, .. }) => {
                    self.engine.journal_mut().release(permit);
                    r.busy_controls = 0;
                }
            }
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
        self.runtime = Some(r);
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
                        r.keys.insert(client, row.key);
                        r.rows.insert(row.key, row);
                        if self.is_blocklisted(&open.link.peer_addr()) {
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
                                    s.send(&MClientSession::reject_blocklisted());
                                }
                                self.request_eviction(sessions, client);
                            } else {
                                sessions.reject_blocklisted(client, open.link);
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
                            if let Some(s) = sessions.get(client) {
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
                    r.attached.remove(&key.client);
                    r.gather.remove(&key);
                    if let Some(close) = r.closing.remove(&key) {
                        sessions.complete_close(key.client, close.generation, close.reply);
                    }
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
        let Some(mut r) = self.runtime.take() else {
            return false;
        };
        if !r.recovered
            && r.initialized
            && r.gather.is_empty()
            && r.closing.is_empty()
            && !self.engine.journal().busy()
            && let Ok(step) = self.engine.finish_reconnect(&[])
        {
            r.recovered = true;
            self.consume_step(sessions, &mut r, step);
        }
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

    pub fn handle_session(
        &mut self,
        sessions: &mut SessionMap,
        client: u64,
        link: Box<dyn ClientLink>,
        msg: MClientSession,
        cap_auths: Vec<MdsCapAuth>,
    ) {
        if self.fatal_error().is_some() || sessions.output_failed() {
            return;
        }
        match msg.op {
            SessionOp::REQUEST_OPEN => {
                let Some(r) = &mut self.runtime else {
                    return;
                };
                if sessions.terminal_close_pending(client)
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
            SessionOp::REQUEST_FLUSH_MDLOG => self.flush_mdlog(sessions),
            _ => {
                let outcome = sessions.handle(client, link, msg, cap_auths);
                if outcome == SessionOutcome::ProtocolError {
                    self.protocol_error(client);
                }
                // Renewal changes only wire-session time. Engine rights remain until
                // an explicit acknowledged release or verified durable eviction.
            }
        }
        self.pump(sessions, 32);
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
                    close.kill = true;
                    close.reply = false;
                    close.request_seq = None;
                    if matches!(close.stage, CloseStage::WaitSeq) {
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
    pub fn ready_jobs(&self) -> Vec<LifecycleIntent> {
        if !self.recovered() || self.work_paused {
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
        if let Some(r) = &mut self.runtime
            && r.enqueue(EngineInput::LifecycleCompleted { id, result }, 64)
                .is_err()
        {
            self.fail_runtime(JournalError::Capacity, "lifecycle completion capacity");
        }
    }

    pub fn handle(&mut self, sessions: &mut SessionMap, client: u64, msg: ClientMessage) {
        if self.fatal_error().is_some() {
            return;
        }
        let key = self
            .runtime
            .as_ref()
            .and_then(|r| r.keys.get(&client))
            .copied();
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
                let translated = match request_from_proto(&req, now_ts(), features) {
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
                (
                    EngineInput::Request {
                        key,
                        request: translated,
                    },
                    bytes,
                )
            }
            ClientMessage::Caps(msg) => {
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
            ClientMessage::Reclaim(_) => {
                if let Some(s) = sessions.get_mut(client) {
                    s.send(&MClientReclaimReply::default());
                }
                return;
            }
            ClientMessage::Reconnect(_)
            | ClientMessage::Lease(_)
            | ClientMessage::Metrics(_)
            | ClientMessage::Session(_) => return,
        };
        if let Some(r) = &mut self.runtime
            && r.enqueue(input, bytes).is_err()
        {
            self.fail_runtime(
                JournalError::Capacity,
                "input queue capacity before engine mutation",
            );
            return;
        }
        self.pump(sessions, 32);
    }

    pub fn tick(&mut self, sessions: &mut SessionMap) {
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
        // Do not invalidate engine cap rights through the compatibility stale API.
        // Silent sessions retain rights until the full fencing/cleanup protocol.
        for client in sessions.clients() {
            if sessions.get(client).is_some_and(|s| {
                s.is_open()
                    && s.last_renew.elapsed() > self.session_timeout + self.session_autoclose
            }) {
                self.request_eviction(sessions, client);
            }
            if self.fatal_error().is_some() {
                return;
            }
        }
        self.pump(sessions, 32);
    }
}
