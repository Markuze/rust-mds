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
            && self.pressure_wait.is_empty()
            && (!self.grants_released() || self.max_size_queue.is_empty())
    }
    pub(super) fn fail(&mut self, error: JournalError) {
        self.failed = Some(error);
        if error == JournalError::Capacity {
            // Keep admitted inputs, cursors and uncertain commits owned for
            // terminal handoff. Other failure classes keep their old cleanup.
            if self.capacity_failure.is_none() {
                self.capacity_failure = Some(self.retry_charge());
            }
            self.notices.clear();
            return;
        }
        self.commits.clear();
        self.parked.clear();
        self.continuation = None;
        self.continuation_completed = None;
        self.continuation_reservation = (0, 0);
        self.pressure_wait.clear();
        self.pressure_ready = false;
        self.invalid_retry_ready = false;
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
            || (self.grants_released() && !self.pressure_wait.is_empty() && self.pressure_ready)
            || self.invalid_retry_ready
        {
            StepState::NeedsAdmission
        } else if !self.parked.is_empty() || !self.pressure_wait.is_empty() {
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
            JournalEvent::Failed(error) if !self.is_failed() => self.fail(error),
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
    fn control_capacity_error(
        &self,
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
        charge: retry_charge::Charge,
    ) -> StepError {
        StepError::ControlCapacity {
            input,
            permit,
            required_completions: charge.completions,
            required_bytes: charge.bytes,
            required_transient_bytes: charge.transient,
        }
    }
    fn take_capacity_continuation(&mut self) -> Option<EngineInput> {
        let mut input = self.continuation.take()?;
        if let EngineInput::CapRelease { items, .. } = &mut input {
            items.extend(self.release_cursor.drain(..));
        }
        Some(input)
    }
    // Busy must return the exact unboxed input and linear permit to admission.
    #[allow(clippy::result_large_err)]
    pub fn step(
        &mut self,
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
    ) -> Result<EngineStep, StepError> {
        if self.is_failed() {
            if let Some(charge) = self.capacity_failure {
                // A supplied input does not own an older control's cursor.
                let returned = input.or_else(|| self.take_capacity_continuation());
                if returned.is_some() {
                    return Err(self.control_capacity_error(returned, permit, charge));
                }
            }
            return Ok(self.result_step(Vec::new(), permit));
        }
        if self.recovery_semantics == RecoverySemantics::Legacy2 {
            return Err(StepError::UpgradeRequired { input, permit });
        }
        if let Some(EngineInput::Request { key, request }) = input.as_ref()
            && !self.valid_retry_identity(key.client, request)
        {
            return Err(StepError::InvalidRetryIdentity { input, permit });
        }
        let control = input.as_ref().is_some_and(Self::is_control);
        let protected = matches!(
            input,
            Some(
                EngineInput::CapUpdate { .. }
                    | EngineInput::CapRelease { .. }
                    | EngineInput::CloseSession { .. }
                    | EngineInput::LifecycleCompleted { .. }
            )
        );
        let preempt = protected
            && self.grants_released()
            && matches!(self.continuation, Some(EngineInput::Request { .. }));
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
                && ((self.continuation.is_some() && !preempt)
                    || (!self.parked.is_empty() && !control)))
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
        if let Some(EngineInput::Request { key, request }) = &input
            && self.inflight.contains_key(&(key.client, request.tid))
        {
            // The original bounded context still owns the result. Do not
            // execute releases or reserve a second transaction for its resend.
            return Ok(self.result_step(Vec::new(), permit));
        }
        if input.is_none()
            && let Some(EngineInput::Request { key, request }) = &self.continuation
            && !self.valid_pending_identity(
                key.client,
                key.incarnation,
                request,
                self.continuation_completed.as_ref(),
            )
            && let Some(p) = self.take_request_continuation()
        {
            let input = Some(self.unown_pending_request(p));
            return Err(StepError::InvalidRetryIdentity { input, permit });
        }
        if input.is_none()
            && self.continuation.is_none()
            && !self.parked_ready
            && !self.pressure_ready
            && let Some(p) = self.take_invalid_retry()
        {
            let input = Some(self.unown_pending_request(p));
            return Err(StepError::InvalidRetryIdentity { input, permit });
        }
        let effective = input.as_ref().or(self.continuation.as_ref());
        let captured = if input.is_some() {
            match input.as_ref() {
                Some(EngineInput::Request { key, request }) => self
                    .sessions
                    .get(&key.client)
                    .and_then(|s| s.completed_requests.get(&request.tid)),
                _ => None,
            }
        } else {
            self.continuation_completed.as_ref()
        };
        let ordinary = matches!(effective, Some(EngineInput::Request { .. }));
        let pressure_scan = effective.is_none()
            && self.purge_queue.is_empty()
            && !self.parked_ready
            && self.grants_released()
            && self.pressure_ready;
        let pressure_index = if pressure_scan {
            // At most the current owned-input count is inspected on this wake.
            // Preserve FIFO order among fitting candidates. Invalid owned
            // identities also make progress by returning their original input.
            self.pressure_wait.iter().position(|p| {
                !self.valid_pending_identity(p.client, p.epoch, &p.req, p.completed.as_ref())
                    || self
                        .with_retry_reservations(
                            self.projected_request_charge(p),
                            Some((p.client, p.req.tid)),
                        )
                        .fits(self.config.retry_limits)
            })
        } else {
            None
        };
        if pressure_scan && pressure_index.is_none() {
            self.pressure_ready = false;
        }
        let pending = if effective.is_none() && self.purge_queue.is_empty() {
            if self.parked_ready {
                self.parked.front()
            } else if let Some(index) = pressure_index {
                self.pressure_wait.get(index)
            } else {
                None
            }
        } else {
            None
        };
        if pending.is_some_and(|p| {
            !self.valid_pending_identity(p.client, p.epoch, &p.req, p.completed.as_ref())
        }) {
            let p = if self.parked_ready {
                self.parked.pop_front()
            } else {
                pressure_index.and_then(|index| self.pressure_wait.remove(index))
            };
            self.parked_ready = !self.parked.is_empty();
            let input = p.map(|p| self.unown_pending_request(p));
            return Err(StepError::InvalidRetryIdentity { input, permit });
        }
        let raw_charge = effective
            .map(|i| self.projected_retry_charge(i, captured))
            .or_else(|| pending.map(|p| self.projected_request_charge(p)))
            .unwrap_or_else(|| self.retry_charge());
        let exclude = match effective {
            Some(EngineInput::Request { key, request }) => Some((key.client, request.tid)),
            _ => pending.map(|p| (p.client, p.req.tid)),
        };
        let charge = if ordinary || pending.is_some() {
            self.with_retry_reservations(raw_charge, exclude)
        } else {
            raw_charge
        };
        if !charge.fits(self.config.retry_limits) {
            if ordinary || pending.is_some() {
                if input.is_some() {
                    return Err(StepError::RetryCapacity { input, permit });
                }
                if self.continuation.is_some() {
                    if let Some(returned) = self.park_continuation_for_pressure() {
                        self.capacity_failure = Some(charge);
                        return Err(self.control_capacity_error(Some(returned), permit, charge));
                    }
                } else if self.parked_ready
                    && let Some(mut p) = self.parked.pop_front()
                {
                    if !self.release_retry_reservation(&mut p.retry_reservation) {
                        let returned = self.unown_pending_request(p);
                        self.capacity_failure = Some(charge);
                        return Err(self.control_capacity_error(Some(returned), permit, charge));
                    }
                    p.retry_reservation = (0, 0);
                    self.pressure_wait.push_back(p);
                    self.parked_ready = false;
                }
                self.pressure_ready = false;
                return Ok(self.result_step(Vec::new(), permit));
            }
            let mut returned = input.or_else(|| self.continuation.take());
            if let Some(EngineInput::CapRelease { items, .. }) = &mut returned {
                items.extend(self.release_cursor.drain(..));
            }
            self.capacity_failure = Some(charge);
            self.fail(JournalError::Capacity);
            return Err(self.control_capacity_error(returned, permit, charge));
        }
        let reservation = raw_charge.reservation(self.retry_charge());
        if protected
            && (preempt
                || !self
                    .with_retry_reservations(charge, None)
                    .fits(self.config.retry_limits))
        {
            if !self.revoke_retry_reservations() {
                self.capacity_failure = Some(charge);
                return Err(self.control_capacity_error(input, permit, charge));
            }
            self.pressure_ready = !self.pressure_wait.is_empty();
        }
        if input.is_some() {
            self.continuation_reservation = if ordinary { reservation } else { (0, 0) };
            self.continuation_completed = match input.as_ref() {
                Some(EngineInput::Request { key, request }) => self
                    .sessions
                    .get(&key.client)
                    .and_then(|s| s.completed_requests.get(&request.tid))
                    .cloned(),
                _ => None,
            };
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
                let captured = self.continuation_completed.take();
                self.dispatch_captured(key.client, request, captured, &mut out)
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
                        if !self.drop_request(&mut p, &mut out) {
                            self.parked.push_front(p);
                        }
                    } else {
                        match self.execute(&mut p, &mut out) {
                            Exec::Done => {
                                if !self.finish_request(&mut p, &mut out) {
                                    self.parked.push_front(p);
                                }
                            }
                            Exec::Wait => self.parked.push_back(p),
                        }
                    }
                } else if self.grants_released()
                    && self.pressure_ready
                    && !self.pressure_wait.is_empty()
                {
                    if let Some(index) = pressure_index {
                        self.resume_pressure_request(index, &mut out);
                    }
                } else {
                    self.drain_evals(&mut out);
                }
            }
        }
        let unused = self.step_permit.take();
        self.stepping = false;
        if self.is_failed()
            && let Some(charge) = self.capacity_failure
            && let Some(returned) = self.take_capacity_continuation()
        {
            return Err(self.control_capacity_error(Some(returned), unused, charge));
        }
        let mut result = self.result_step(out, unused);
        if retry_parked && !self.is_failed() && self.commits.is_empty() && !self.parked.is_empty() {
            self.parked_ready = true;
            result.state = StepState::NeedsAdmission;
        }
        Ok(result)
    }
    fn close_step(&mut self, key: SessionKey, kill: bool, out: &mut Vec<Effect>) {
        // Retain the exact scalar control before dropping a context can fail.
        self.continuation = Some(EngineInput::CloseSession { key, kill });
        self.drop_requests_of(key.client, out);
        if self.is_failed() {
            out.clear();
            return;
        }
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
        if !self.is_failed() {
            self.continuation = None;
        }
    }
}
