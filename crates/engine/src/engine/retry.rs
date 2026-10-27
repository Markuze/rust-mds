//! Retained current targets, canonical durable history, and retry reconstruction.
use super::*;

#[cfg(test)]
#[path = "retry_wake_tests.rs"]
mod wake_tests;

pub(super) fn target_operation(op: u32) -> bool {
    matches!(
        op,
        CEPH_MDS_OP_CREATE
            | CEPH_MDS_OP_OPEN
            | CEPH_MDS_OP_MKNOD
            | CEPH_MDS_OP_MKDIR
            | CEPH_MDS_OP_SYMLINK
    )
}

impl<J: Journal> Engine<J> {
    /// Instance-local release observation for external retry-capacity waiters.
    /// Zero on construction/checked restore. It advances when a nonzero ordinary
    /// reservation, charged captured context/pin or retained transaction use
    /// ends, including committed history/reference/metadata release. A transfer
    /// into a pending record is not a release. Read this baseline after load.
    /// A change means a waiter may fit, not guaranteed admission. This getter
    /// allocates nothing, schedules nothing and does not acknowledge/reset it.
    /// It is not persistent. Read-only floors, unchanged ticks, rejected
    /// preflights and duplicate callbacks give no credit. Exhaustion latches
    /// `JournalError::Capacity` before more scheduling or output.
    pub fn retry_capacity_epoch(&self) -> u64 {
        self.retry_capacity_epoch
    }

    pub(super) fn retry_capacity_released(&mut self) -> bool {
        if self.is_failed() {
            return false;
        }
        let Some(next) = self.retry_capacity_epoch.checked_add(1) else {
            self.fail(JournalError::Capacity);
            return false;
        };
        self.retry_capacity_epoch = next;
        self.pressure_ready = !self.pressure_wait.is_empty();
        true
    }

    pub(super) fn release_retry_reservation(&mut self, reservation: &mut (usize, usize)) -> bool {
        if *reservation != (0, 0) {
            if !self.retry_capacity_released() {
                return false;
            }
            *reservation = (0, 0);
        }
        true
    }

    /// Refuse a bounded drop/revocation batch before it can lose a prefix.
    pub(super) fn check_retry_capacity_releases(&mut self, count: usize) -> bool {
        if self.is_failed() {
            return false;
        }
        if u64::try_from(count)
            .ok()
            .and_then(|n| self.retry_capacity_epoch.checked_add(n))
            .is_none()
        {
            self.fail(JournalError::Capacity);
            return false;
        }
        true
    }

    pub(super) fn observe_request_floor(&mut self, client: ClientId, floor: u64) {
        if let Some(session) = self.sessions.get_mut(&client) {
            let old = session.oldest_client_tid;
            session.trim_completed_requests(floor);
            if session.oldest_client_tid > old {
                // Identity readiness is not capacity readiness. This bounded
                // scan neither projects nor wakes valid pressure waiters.
                self.invalid_retry_ready |=
                    self.parked.iter().chain(&self.pressure_wait).any(|p| {
                        p.client == client
                            && p.completed.is_none()
                            && !self.valid_pending_identity(p.client, p.epoch, &p.req, None)
                    });
            }
        }
    }

    pub(super) fn take_invalid_retry(&mut self) -> Option<Pending> {
        if !self.invalid_retry_ready {
            return None;
        }
        // Keep the existing parked-before-pressure priority and FIFO order
        // among invalid identities within each owned queue. One return/turn.
        let pending = if let Some(index) = self.parked.iter().position(|p| {
            !self.valid_pending_identity(p.client, p.epoch, &p.req, p.completed.as_ref())
        }) {
            self.parked.remove(index)
        } else if let Some(index) = self.pressure_wait.iter().position(|p| {
            !self.valid_pending_identity(p.client, p.epoch, &p.req, p.completed.as_ref())
        }) {
            self.pressure_wait.remove(index)
        } else {
            None
        };
        self.invalid_retry_ready =
            self.parked.iter().chain(&self.pressure_wait).any(|p| {
                !self.valid_pending_identity(p.client, p.epoch, &p.req, p.completed.as_ref())
            });
        pending
    }

    pub(super) fn valid_retry_identity(&self, client: ClientId, req: &Request) -> bool {
        let Some(session) = self.sessions.get(&client) else {
            return true;
        };
        if let Some(EngineInput::Request { key, request }) = &self.continuation
            && key.client == client
            && request.tid == req.tid
        {
            return request.op.code() == req.op.code();
        }
        if let Some(op) = self.inflight.get(&(client, req.tid)) {
            return *op == req.op.code();
        }
        if let Some(done) = session.completed_requests.get(&req.tid) {
            return done.op == req.op.code();
        }
        req.tid >= session.oldest_client_tid && req.oldest_client_tid <= req.tid
    }

    /// An owned new request has no exemption from a later accepted floor.
    /// Captured completions retain their original identity after retirement.
    pub(super) fn valid_pending_identity(
        &self,
        client: ClientId,
        epoch: u64,
        req: &Request,
        captured: Option<&CompletedRequest>,
    ) -> bool {
        let Some(session) = self.sessions.get(&client).filter(|s| s.epoch == epoch) else {
            return false;
        };
        if let Some(done) = captured {
            return done.client == client
                && done.session_incarnation == epoch
                && done.tid == req.tid
                && done.op == req.op.code();
        }
        req.tid
            >= session
                .oldest_client_tid
                .max(session.durable_oldest_client_tid)
            && req.oldest_client_tid <= req.tid
            && !session.completed_requests.contains_key(&req.tid)
    }

    pub(super) fn take_request_continuation(&mut self) -> Option<Pending> {
        match self.continuation.take() {
            Some(EngineInput::Request { key, request }) => Some(Pending {
                client: key.client,
                epoch: key.incarnation,
                req: request,
                holds: Vec::new(),
                flock_was_waiting: false,
                completed: self.continuation_completed.take(),
                retry_reservation: std::mem::take(&mut self.continuation_reservation),
                started: false,
            }),
            other => {
                self.continuation = other;
                None
            }
        }
    }

    /// Return ownership without evaluating caps or publishing any effects.
    pub(super) fn unown_pending_request(&mut self, mut p: Pending) -> EngineInput {
        if (p.retry_reservation != (0, 0) || p.completed.is_some())
            && !self.retry_capacity_released()
        {
            return EngineInput::Request {
                key: SessionKey {
                    client: p.client,
                    incarnation: p.epoch,
                },
                request: p.req,
            };
        }
        for (ino, id) in p.holds.drain(..) {
            if let Some(i) = self.inodes.get_mut(&ino) {
                i.holds.retain(|h| h.id != id);
            }
            self.eval_queue.insert(ino);
        }
        self.cancel_request_wait(&p);
        self.inflight.remove(&(p.client, p.req.tid));
        if p.retry_reservation != (0, 0) || p.completed.is_some() {
            self.pressure_ready = !self.pressure_wait.is_empty();
        }
        if let Some(ino) = p.completed.as_ref().and_then(|c| c.reply_target_ino)
            && self.grants_released()
        {
            self.purge_queue.insert(ino);
        }
        EngineInput::Request {
            key: SessionKey {
                client: p.client,
                incarnation: p.epoch,
            },
            request: p.req,
        }
    }

    pub(super) fn rebuild_retry_refs(&mut self) {
        self.retry_refs.clear();
        if self.recovery_semantics == RecoverySemantics::RetainedTargets3 {
            for s in self.sessions.values() {
                for done in s.completed_requests.values() {
                    if let Some(ino) = done.reply_target_ino {
                        *self.retry_refs.entry(ino).or_default() += 1;
                    }
                }
            }
        }
    }

    fn drop_retry_ref(&mut self, ino: Ino) {
        if let Some(count) = self.retry_refs.get_mut(&ino) {
            *count -= 1;
            if *count == 0 {
                self.retry_refs.remove(&ino);
                if self.grants_released() {
                    self.purge_queue.insert(ino);
                }
            }
        }
    }

    pub(super) fn commit_retry_completion(&mut self, done: CompletedRequest) {
        let client = done.client;
        let floor = self.sessions[&client]
            .durable_oldest_client_tid
            .max(done.oldest_client_tid);
        self.observe_request_floor(client, floor);
        while self.sessions[&client]
            .completed_requests
            .first_key_value()
            .is_some_and(|(tid, _)| *tid < floor)
        {
            let retired = self
                .sessions
                .get_mut(&client)
                .and_then(|s| s.completed_requests.pop_first());
            if let Some((_, c)) = retired
                && let Some(ino) = c.reply_target_ino
            {
                self.drop_retry_ref(ino);
            }
        }
        if let Some(retired) = self
            .sessions
            .get_mut(&client)
            .and_then(|s| s.completed_requests.remove(&done.tid))
            && let Some(ino) = retired.reply_target_ino
        {
            self.drop_retry_ref(ino);
        }
        if self.recovery_semantics == RecoverySemantics::RetainedTargets3
            && done.tid >= floor
            && let Some(ino) = done.reply_target_ino
        {
            *self.retry_refs.entry(ino).or_default() += 1;
        }
        if let Some(session) = self.sessions.get_mut(&client) {
            session.commit_request(done);
        }
    }

    pub(super) fn retire_retry_session(&mut self, key: SessionKey) {
        while let Some((_, done)) = self
            .sessions
            .get_mut(&key.client)
            .and_then(|s| s.completed_requests.pop_first())
        {
            if let Some(ino) = done.reply_target_ino {
                self.drop_retry_ref(ino);
            }
        }
    }

    pub(super) fn retains_retry_target(&self, ino: Ino) -> bool {
        self.retry_refs.contains_key(&ino) || self.reply_holds_target(ino)
    }
    pub(super) fn reply_holds_target(&self, ino: Ino) -> bool {
        self.parked
            .iter()
            .any(|p| p.completed.as_ref().and_then(|c| c.reply_target_ino) == Some(ino))
            || self
                .pressure_wait
                .iter()
                .any(|p| p.completed.as_ref().and_then(|c| c.reply_target_ino) == Some(ino))
            || self.commits.iter().any(|c| c.reply_hold == Some(ino))
            || self
                .continuation_completed
                .as_ref()
                .and_then(|c| c.reply_target_ino)
                == Some(ino)
    }

    pub(super) fn owned_completion_contexts(&self) -> Option<usize> {
        let slots = self.parked.iter().chain(&self.pressure_wait).try_fold(
            usize::from(self.continuation_completed.is_some()),
            |n, p| n.checked_add(usize::from(p.completed.is_some())),
        )?;
        self.commits.iter().try_fold(slots, |n, c| {
            n.checked_add(usize::from(
                c.completed_request.is_some() || c.reply_hold.is_some(),
            ))
        })
    }

    pub(super) fn owns_completed_context(&self, key: SessionKey, tid: u64) -> bool {
        self.continuation_completed.as_ref().is_some_and(|c| {
            c.client == key.client && c.session_incarnation == key.incarnation && c.tid == tid
        }) || self.parked.iter().chain(&self.pressure_wait).any(|p| {
            p.client == key.client
                && p.epoch == key.incarnation
                && p.req.tid == tid
                && p.completed.is_some()
        })
    }

    pub(super) fn park_continuation_for_pressure(&mut self) -> Option<EngineInput> {
        if let Some(mut p) = self.take_request_continuation() {
            if !self.release_retry_reservation(&mut p.retry_reservation) {
                return Some(self.unown_pending_request(p));
            }
            self.inflight.insert((p.client, p.req.tid), p.req.op.code());
            p.retry_reservation = (0, 0);
            self.pressure_wait.push_back(p);
        }
        None
    }
    pub(super) fn revoke_retry_reservations(&mut self) -> bool {
        let releases = usize::from(
            matches!(self.continuation, Some(EngineInput::Request { .. }))
                && self.continuation_reservation != (0, 0),
        ) + self
            .parked
            .iter()
            .filter(|p| p.retry_reservation != (0, 0))
            .count();
        if !self.check_retry_capacity_releases(releases) {
            return false;
        }
        if matches!(self.continuation, Some(EngineInput::Request { .. }))
            && self.park_continuation_for_pressure().is_some()
        {
            return false;
        }
        while let Some(mut pending) = self.parked.pop_front() {
            if !self.release_retry_reservation(&mut pending.retry_reservation) {
                self.parked.push_front(pending);
                return false;
            }
            // A revoked pre-mutation reservation must not retain namespace
            // locks that prevent fitting retirement work from passing it.
            for (ino, id) in pending.holds.drain(..) {
                if let Some(i) = self.inodes.get_mut(&ino) {
                    i.holds.retain(|h| h.id != id);
                }
                self.eval_queue.insert(ino);
            }
            pending.retry_reservation = (0, 0);
            self.pressure_wait.push_back(pending);
        }
        self.parked_ready = false;
        true
    }
    pub(super) fn resume_pressure_request(&mut self, index: usize, out: &mut Vec<Effect>) {
        let Some(mut p) = self.pressure_wait.remove(index) else {
            return;
        };
        // Continue the bounded wake pass. A full pass with no eligible fitting
        // candidate, not one nonfitting front request, closes this wake.
        self.pressure_ready = !self.pressure_wait.is_empty();
        p.retry_reservation = self
            .projected_request_charge(&p)
            .reservation(self.retry_charge());
        if !self.session_matches(p.client, p.epoch) {
            if !self.drop_request(&mut p, out) {
                self.pressure_wait.insert(index, p);
            }
            return;
        }
        if !p.started {
            if p.completed.is_none() {
                self.observe_request_floor(p.client, p.req.oldest_client_tid);
                self.process_request_releases(p.client, &p.req, out);
            }
            p.started = true;
        }
        match self.execute(&mut p, out) {
            Exec::Done => {
                if !self.finish_request(&mut p, out) {
                    self.pressure_wait.insert(index, p);
                }
            }
            Exec::Wait => self.parked.push_back(p),
        }
    }

    pub(super) fn retry_completed(&mut self, p: &mut Pending, out: &mut Vec<Effect>) -> Exec {
        let Some(done) = p.completed.as_ref() else {
            return Exec::Wait;
        };
        let Some(ino) = done.reply_target_ino else {
            return Exec::Wait;
        };
        let (op, result, created_ino) = (done.op, done.result, done.created_ino);
        let (path, flags, mask) = match &p.req.op {
            Op::Create { path, flags, .. } => (path.clone(), *flags, 0),
            Op::Open {
                path, flags, mask, ..
            } => (path.clone(), *flags, *mask),
            Op::Mknod { path, .. } | Op::Mkdir { path, .. } | Op::Symlink { path, .. } => {
                (path.clone(), 0, 0)
            }
            _ => return Exec::Wait,
        };
        let dentry = self
            .resolve_parent(&path)
            .ok()
            .filter(|(_, name)| !name.is_empty());
        let named = !path.names.is_empty();
        let unchanged = self.resolve(&path) == Ok(ino)
            && (dentry.is_some() || (!named && op == CEPH_MDS_OP_OPEN));
        if unchanged && matches!(op, CEPH_MDS_OP_CREATE | CEPH_MDS_OP_OPEN) {
            return self.op_open(p, path, flags & !CEPH_O_TRUNC, mask, 0, dentry, out);
        }
        let mut spec = self.spec(
            p,
            if unchanged {
                result
            } else {
                Errno::EINTR.as_neg()
            },
        );
        spec.target = Some(ino);
        spec.created_ino = created_ino;
        if op == CEPH_MDS_OP_CREATE
            && let Some(created) = created_ino
        {
            spec.extra = ReplyExtra::Create {
                ino: created,
                delegated: Vec::new(),
            };
        }
        spec.issue_caps = unchanged;
        if unchanged {
            let holds = self.read_holds(ino, p.client, CEPH_STAT_CAP_INODE_ALL);
            if !self.acquire_holds(p, &holds, out) {
                return Exec::Wait;
            }
            spec.dentry = dentry;
        }
        self.reply_now(spec, out)
    }
}
