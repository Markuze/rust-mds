//! One-record production execution. Completion only releases committed output.

use super::*;

impl<J: Journal> Engine<J> {
    pub fn is_failed(&self) -> bool {
        self.failed.is_some()
    }
    pub fn failure(&self) -> Option<JournalError> {
        self.failed
    }
    pub fn is_quiescent(&self) -> bool {
        !self.is_failed()
            && self.commits.is_empty()
            && self.continuation.is_none()
            && self.parked.is_empty()
            && self.purge_queue.is_empty()
            && self.inflight.is_empty()
            && self.eval_queue.is_empty()
            && (!self.grants_released() || self.max_size_queue.is_empty())
    }
    pub(super) fn fail(&mut self, error: JournalError) {
        self.failed = Some(error);
        self.commits.clear();
        self.parked.clear();
        self.continuation = None;
        self.eval_queue.clear();
        self.deferred_evals.clear();
        self.max_size_queue.clear();
        self.purge_queue.clear();
        self.notices.clear();
        self.release_cursor.clear();
        self.parked_ready = false;
    }
    fn step_state(&self) -> StepState {
        if self.is_failed() {
            StepState::Failed
        } else if let Some(c) = self.commits.back() {
            StepState::WaitingCommit(c.seq)
        } else if self.continuation.is_some()
            || !self.purge_queue.is_empty()
            || !self.eval_queue.is_empty()
            || (self.grants_released() && !self.max_size_queue.is_empty())
            || (!self.parked.is_empty() && self.parked_ready)
        {
            StepState::NeedsAdmission
        } else if !self.parked.is_empty() {
            StepState::WaitingInput
        } else {
            StepState::Idle
        }
    }
    pub(super) fn result_step(
        &mut self,
        mut effects: Vec<Effect>,
        permit: Option<RecordPermit>,
    ) -> EngineStep {
        if self.is_failed() {
            effects.clear();
        }
        EngineStep {
            effects,
            notices: std::mem::take(&mut self.notices),
            state: self.step_state(),
            unused_permit: permit,
        }
    }
    pub fn journal_event(&mut self, event: JournalEvent) -> EngineStep {
        let mut out = Vec::new();
        match event {
            JournalEvent::Failed(error) => self.fail(error),
            JournalEvent::Committed(seq) if !self.is_failed() => {
                if seq > self.submitted_seq || seq > self.journal.committed_seq() {
                    self.fail(JournalError::Corrupt);
                } else if seq >= self.durable_seq {
                    self.stepping = true;
                    self.process_commits(&mut out);
                    self.stepping = false;
                }
            }
            _ => {}
        }
        let mut result = self.result_step(out, None);
        // A parked operation may now proceed, but only in a fresh step.
        if !self.is_failed() && self.commits.is_empty() && !self.parked.is_empty() {
            self.parked_ready = true;
            result.state = StepState::NeedsAdmission;
        }
        result
    }
    fn input_key(input: &EngineInput) -> Option<SessionKey> {
        match input {
            EngineInput::Request { key, .. }
            | EngineInput::CapUpdate { key, .. }
            | EngineInput::CapRelease { key, .. }
            | EngineInput::CloseSession { key, .. } => Some(*key),
            _ => None,
        }
    }
    fn is_control(input: &EngineInput) -> bool {
        match input {
            EngineInput::Request { request, .. } => matches!(&request.op,
                Op::SetFileLock { lock, .. } if lock.ltype == CEPH_LOCK_UNLOCK),
            _ => true,
        }
    }
    // Busy must return the exact unboxed input and linear permit to admission.
    #[allow(clippy::result_large_err)]
    pub fn step(
        &mut self,
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
    ) -> Result<EngineStep, StepError> {
        if self.is_failed() {
            return Ok(self.result_step(Vec::new(), permit));
        }
        let control = input.as_ref().is_some_and(Self::is_control);
        if let Some(EngineInput::CapUpdate { key, update }) = input.as_ref()
            && update.dirty != 0
            && !self.sessions.get(&key.client).is_some_and(|s| {
                s.epoch == key.incarnation
                    && update.flush_tid != 0
                    && s.completed_flushes.contains(&update.flush_tid)
            })
            && self
                .jobs
                .values()
                .any(|j| j.ino == update.ino && matches!(j.kind, LifecycleKind::RecoverSize { .. }))
            && self
                .inodes
                .get(&update.ino)
                .and_then(|i| i.caps.get(&key.client))
                .is_none_or(|c| c.issued() & CEPH_CAP_ANY_WR == 0)
        {
            return Err(StepError::Busy { input, permit });
        }
        if !self.commits.is_empty()
            || (input.is_some()
                && (self.continuation.is_some() || (!self.parked.is_empty() && !control)))
        {
            return Err(StepError::Busy { input, permit });
        }
        if input.as_ref().and_then(Self::input_key).is_some_and(|k| {
            !self.session_matches(k.client, k.incarnation) || self.cleanup_receipts.contains_key(&k)
        }) {
            return Err(StepError::InvalidSession { input, permit });
        }
        if matches!(input, Some(EngineInput::Request { .. })) && !self.grants_released()
            || !self.recovery_controls_runnable()
                && matches!(
                    input,
                    Some(
                        EngineInput::CapUpdate { .. }
                            | EngineInput::CapRelease { .. }
                            | EngineInput::LifecycleCompleted { .. }
                            | EngineInput::RecoveryControlsDrained
                    )
                )
        {
            return Err(StepError::Busy { input, permit });
        }
        if matches!(input, Some(EngineInput::RecoveryControlsDrained)) && self.grants_released() {
            return Ok(self.result_step(Vec::new(), permit));
        }
        if input.is_some() {
            self.continuation = input;
        }
        if permit.is_none() {
            let mut r = self.result_step(Vec::new(), None);
            if self.continuation.is_some() || (!self.parked.is_empty() && self.parked_ready) {
                r.state = StepState::NeedsAdmission;
            }
            return Ok(r);
        }
        self.stepping = true;
        self.step_permit = permit;
        let mut out = Vec::new();
        // Admission may have retained this operation in an earlier call.
        // Its progress classification must follow the executed continuation.
        let mut retry_parked = self.continuation.as_ref().is_some_and(Self::is_control);
        match self.continuation.take() {
            Some(EngineInput::Request { key, request }) => {
                self.dispatch_new(key.client, request, &mut out)
            }
            Some(EngineInput::CapUpdate { key, update }) => {
                out = self.handle_cap_update(key.client, update);
            }
            Some(EngineInput::CapRelease { key, items }) => {
                if !items.is_empty() {
                    self.release_cursor = items.into();
                }
                if let Some(item) = self.release_cursor.pop_front() {
                    out = self.handle_cap_release(key.client, &[item]);
                }
                if !self.release_cursor.is_empty() {
                    self.continuation = Some(EngineInput::CapRelease {
                        key,
                        items: Vec::new(),
                    });
                }
            }
            Some(EngineInput::CloseSession { key, kill }) => self.close_step(key, kill, &mut out),
            Some(EngineInput::Tick) => self.journal.flush(),
            Some(EngineInput::LifecycleCompleted { id, result }) => {
                self.complete_job(id, result, &mut out)
            }
            Some(EngineInput::RecoveryControlsDrained) => {
                self.recovery_phase = RecoveryPhase::GrantsReleased;
                self.eval_queue.append(&mut self.deferred_evals);
                self.queue_unclaimed_strays();
            }
            None => {
                if let Some(ino) = self.purge_queue.pop_first() {
                    self.maybe_purge_stray(ino, &mut out);
                    retry_parked = true;
                } else if self.parked_ready
                    && let Some(mut p) = self.parked.pop_front()
                {
                    self.parked_ready = false;
                    if !self.session_matches(p.client, p.epoch) {
                        self.drop_request(&mut p, &mut out);
                    } else {
                        match self.execute(&mut p, &mut out) {
                            Exec::Done => self.finish_request(&mut p, &mut out),
                            Exec::Wait => self.parked.push_back(p),
                        }
                    }
                } else {
                    self.drain_evals(&mut out);
                }
            }
        }
        let unused = self.step_permit.take();
        self.stepping = false;
        let mut result = self.result_step(out, unused);
        if retry_parked && !self.is_failed() && self.commits.is_empty() && !self.parked.is_empty() {
            self.parked_ready = true;
            result.state = StepState::NeedsAdmission;
        }
        Ok(result)
    }
    fn close_step(&mut self, key: SessionKey, kill: bool, out: &mut Vec<Effect>) {
        self.drop_requests_of(key.client, out);
        let cap = self
            .sessions
            .get(&key.client)
            .and_then(|s| s.caps.first().copied());
        if let Some(ino) = cap {
            self.remove_cap(ino, key.client, kill, out);
            self.continuation = Some(EngineInput::CloseSession { key, kill });
            return;
        }
        if let Some(ino) = self
            .sessions
            .get(&key.client)
            .and_then(|s| s.lock_inos.first().copied())
        {
            self.drop_client_locks(ino, key.client);
            self.continuation = Some(EngineInput::CloseSession { key, kill });
            return;
        }
        // Restore deliberately has no runtime caps. Do not mistake that for
        // proof that the client's durable writer bounds have been retired.
        let range = self.inodes.iter().find_map(|(ino, i)| {
            (i.core.client_ranges.contains_key(&key.client)
                && !self.writer_range_is_covered(*ino, key.client))
            .then_some(*ino)
        });
        if let Some(ino) = range {
            // A restored Closing session cannot prove a graceful cap flush.
            // Keep its bound and journal recovery, even if kill was false.
            self.cleanup_client_range(ino, key.client, true, out);
            self.continuation = Some(EngineInput::CloseSession { key, kill });
            return;
        }
        self.ino_alloc.revoke_delegations(key.client);
        let mut txn = Txn::new(RecordKind::SessionClose);
        txn.rec.ino_alloc = Some(self.ino_alloc.state());
        txn.rec.closed_sessions.push(key);
        self.submit(txn, None, out);
    }
}
