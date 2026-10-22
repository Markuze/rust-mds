//! Validated, replaceable reconnect chunks. The daemon owns the gather budget.

use super::*;

impl<J: Journal> Engine<J> {
    pub(super) fn queue_unclaimed_strays(&mut self) {
        self.purge_queue.extend(
            self.inodes
                .iter()
                .filter_map(|(ino, i)| (i.core.nlink == 0 && i.caps.is_empty()).then_some(*ino)),
        );
    }
    pub fn reconnect_chunk(
        &mut self,
        key: SessionKey,
        caps: &[ReconnectCap],
        realms: &[ReconnectRealm],
        more: bool,
    ) -> Result<ReconnectProgress, RestoreError> {
        if self.is_failed() {
            return Err(RestoreError::Failed);
        }
        if self.recovery_ready {
            return Err(RestoreError::Invalid("reconnect after recovery"));
        }
        if !self.session_matches(key.client, key.incarnation)
            || self.cleanup_receipts.contains_key(&key)
            || !self.commits.is_empty()
        {
            return Err(RestoreError::Invalid("reconnect session/state"));
        }
        let mut seen = BTreeSet::new();
        let mut traces = Vec::new();
        for c in caps {
            if !seen.insert(c.ino) || c.cap_id == u64::MAX || !self.inodes.contains_key(&c.ino) {
                return Err(RestoreError::Invalid("reconnect inode/realm"));
            }
            if c.realm != self.realm_of(c.ino) {
                traces.push(self.snap_trace(c.ino));
            }
            for lock in c.fcntl_locks.iter().chain(&c.flock_locks) {
                if !matches!(lock.ltype, CEPH_LOCK_SHARED | CEPH_LOCK_EXCL)
                    || (lock.length != 0 && lock.start.checked_add(lock.length - 1).is_none())
                {
                    return Err(RestoreError::Invalid("reconnect lock"));
                }
            }
        }
        for r in realms {
            let info = self
                .snap_realm_info(r.ino)
                .ok_or(RestoreError::Invalid("reconnect realm"))?;
            if info.seq != r.seq || info.parent != r.parent {
                traces.push(self.snap_trace(r.ino));
            }
        }
        for c in caps {
            self.reconnect_cap(
                key.client,
                c.ino,
                c.cap_id,
                c.wanted,
                c.issued,
                &c.fcntl_locks,
                &c.flock_locks,
            )
            .map_err(|_| RestoreError::Invalid("reconnect cap"))?;
            self.reconnected
                .insert((key, c.ino), (c.realm, c.snap_follows));
        }
        if more {
            self.reconnect_complete.remove(&key);
        } else {
            self.reconnect_complete.insert(key);
        }
        Ok(ReconnectProgress {
            complete: !more,
            realm_traces: traces,
        })
    }
    pub fn reconnect_cap_context(&self, key: SessionKey, ino: Ino) -> Option<(Ino, u64)> {
        self.reconnected.get(&(key, ino)).copied()
    }
    /// Discard a connection's incomplete reconstruction, without closing history.
    pub fn reset_reconnect(&mut self, key: SessionKey) -> Result<(), RestoreError> {
        if self.is_failed() {
            return Err(RestoreError::Failed);
        }
        if !self.session_matches(key.client, key.incarnation) {
            return Err(RestoreError::Invalid("reconnect session"));
        }
        let inos: Vec<_> = self
            .reconnected
            .keys()
            .filter(|(k, _)| *k == key)
            .map(|(_, i)| *i)
            .collect();
        for ino in inos {
            if let Some(i) = self.inodes.get_mut(&ino) {
                i.caps.remove(&key.client);
                i.fcntl_locks.remove_all_from(key.client);
                i.flock_locks.remove_all_from(key.client);
            }
            if let Some(s) = self.sessions.get_mut(&key.client) {
                s.caps.remove(&ino);
                s.lock_inos.remove(&ino);
            }
            self.reconnected.remove(&(key, ino));
        }
        self.reconnect_complete.remove(&key);
        Ok(())
    }

    /// Complete the daemon's joined Open-session gather. Missing members must
    /// first finish fenced, durable cleanup. This method submits no records.
    pub fn finish_reconnect(&mut self, keys: &[SessionKey]) -> Result<EngineStep, RestoreError> {
        if self.is_failed() {
            return Err(RestoreError::Failed);
        }
        let gather: BTreeSet<_> = keys.iter().copied().collect();
        let sessions: BTreeSet<_> = self
            .sessions
            .values()
            .map(|s| SessionKey {
                client: s.client,
                incarnation: s.epoch,
            })
            .collect();
        if gather.len() != keys.len()
            || gather != sessions
            || !gather.is_subset(&self.reconnect_complete)
            || !self.commits.is_empty()
            || self.continuation.is_some()
            || self.journal.committed_seq() != self.durable_seq
        {
            return Err(RestoreError::Invalid("incomplete reconnect gather"));
        }
        self.recovery_ready = true;
        self.queue_unclaimed_strays();
        let mut effects = Vec::new();
        self.stepping = true;
        for ino in self.inodes.keys().copied().collect::<Vec<_>>() {
            self.eval_caps(ino, &mut effects);
        }
        self.stepping = false;
        for job in self.pending_jobs() {
            effects.push(Effect::Lifecycle(job));
        }
        Ok(EngineStep {
            effects,
            notices: Vec::new(),
            state: if self.is_quiescent() {
                StepState::Idle
            } else {
                StepState::NeedsAdmission
            },
            unused_permit: None,
        })
    }
}
