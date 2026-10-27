//! Client cap messages, cap releases and session life cycle
//! (`Locker::handle_client_caps`, `_do_cap_update`, `_update_cap_fields`,
//! `handle_client_cap_release`, `process_request_cap_release`,
//! `revoke_stale_caps`, `resume_stale_caps`, `Server::_session_logged`).

use super::{Deferred, Engine, Txn, calc_new_max_size};
use crate::caps::Capability;
use crate::consts::*;
use crate::effects::*;
use crate::errno::Errno;
use crate::filelock::FileLock;
use crate::journal::{ClientFlush, Journal, JournalRecord, RecordKind};
use crate::request::{CapReleaseItem, CapUpdate, Request};
use crate::session::SessionState;
use crate::types::*;

#[cfg(test)]
#[path = "compat_close_r4_tests.rs"]
mod compat_close_r4_tests;

/// `ceph_seq_cmp`.
fn seq_cmp(a: u32, b: u32) -> i32 {
    a.wrapping_sub(b) as i32
}

impl<J: Journal> Engine<J> {
    /// Releases embedded in a request, processed before the request runs.
    /// Grants are re-evaluated only after the request has registered its
    /// holds, so a released bit is not granted straight back.
    pub(crate) fn process_request_releases(
        &mut self,
        client: ClientId,
        req: &Request,
        _out: &mut Vec<Effect>,
    ) {
        for r in &req.releases {
            let Some(cap) = self
                .inodes
                .get_mut(&r.ino)
                .and_then(|i| i.caps.get_mut(&client))
            else {
                continue;
            };
            if cap.cap_id != r.cap_id || r.mseq < cap.mseq {
                continue;
            }
            let caps = r.caps & cap.issued();
            cap.confirm_receipt(r.seq, caps);
            if r.issue_seq == cap.last_issue {
                cap.set_wanted(r.wanted);
            } else {
                cap.set_wanted(cap.wanted() | r.wanted);
            }
            self.eval_queue.insert(r.ino);
        }
    }

    /// Handle a client cap message (UPDATE, FLUSH, DROP, RELEASE, RENEW are
    /// processed alike; FLUSHSNAP is acknowledged, there are no snapshots).
    pub fn handle_cap_update(&mut self, client: ClientId, m: CapUpdate) -> Vec<Effect> {
        if self.is_failed() || self.recovery_semantics == crate::RecoverySemantics::Legacy2 {
            return Vec::new();
        }
        let mut out = Vec::new();
        self.cap_update(client, m, &mut out);
        self.run_parked(&mut out);
        out
    }

    fn cap_update(&mut self, client: ClientId, m: CapUpdate, out: &mut Vec<Effect>) {
        let Some(sess) = self.sessions.get_mut(&client) else {
            return;
        };
        let mut dirty = m.dirty;
        if m.flush_tid != 0 && sess.completed_flushes.contains(&m.flush_tid) {
            // Already applied: acknowledge again at once, process as a plain update.
            if let Some(cap) = self.inodes.get(&m.ino).and_then(|i| i.caps.get(&client)) {
                out.push(Effect::Caps(CapMessage {
                    client,
                    op: if m.op == CEPH_CAP_OP_FLUSHSNAP {
                        CEPH_CAP_OP_FLUSHSNAP_ACK
                    } else {
                        CEPH_CAP_OP_FLUSH_ACK
                    },
                    ino: m.ino,
                    realm: 0,
                    cap_id: cap.cap_id,
                    seq: m.seq,
                    issue_seq: 0,
                    caps: if self.grants_released() {
                        m.caps
                    } else {
                        m.caps & self.effective_caps(m.ino, client)
                    },
                    wanted: 0,
                    dirty: m.dirty,
                    mseq: 0,
                    flush_tid: m.flush_tid,
                    oldest_flush_tid: 0,
                    inode: None,
                }));
            }
            if m.op == CEPH_CAP_OP_FLUSHSNAP {
                return;
            }
            dirty = 0;
        }
        let sess = self.sessions.get_mut(&client).unwrap();
        sess.trim_completed_flushes(m.oldest_flush_tid);
        let ino = m.ino;
        let Some(cap) = self
            .inodes
            .get_mut(&ino)
            .and_then(|i| i.caps.get_mut(&client))
        else {
            return;
        };
        if m.mseq < cap.mseq {
            return;
        }
        if m.op == CEPH_CAP_OP_FLUSHSNAP {
            // No snapshots: nothing to apply. Acknowledge so the client frees
            // its capsnap (sent from a rank holding a cap, as libcephfs asserts).
            out.push(Effect::Caps(CapMessage {
                client,
                op: CEPH_CAP_OP_FLUSHSNAP_ACK,
                ino,
                realm: 0,
                cap_id: 0,
                seq: 0,
                issue_seq: 0,
                caps: 0,
                wanted: 0,
                dirty: m.dirty,
                mseq: 0,
                flush_tid: m.flush_tid,
                oldest_flush_tid: 0,
                inode: None,
            }));
            return;
        }
        if m.cap_id != cap.cap_id {
            return;
        }
        // The client's `implemented` set acknowledges revocations; never
        // confirm bits that were not issued.
        let caps = m.caps & cap.issued();
        cap.confirm_receipt(m.seq, caps);
        if m.issue_seq == cap.last_issue {
            cap.set_wanted(m.wanted);
        } else {
            cap.set_wanted(cap.wanted() | m.wanted);
        }
        self.apply_cap_update(client, &m, dirty, out);
        if self.is_failed() {
            out.clear();
            return;
        }
        if m.flags & CEPH_CLIENT_CAPS_SYNC != 0 {
            self.journal.flush();
        }
        self.eval_caps(ino, out);
    }

    /// Apply flushed metadata and `max_size` changes (`_do_cap_update`).
    fn apply_cap_update(
        &mut self,
        client: ClientId,
        m: &CapUpdate,
        dirty: u32,
        out: &mut Vec<Effect>,
    ) {
        let ino = m.ino;
        if self.stepping
            && dirty == 0
            && self
                .jobs
                .values()
                .any(|j| j.ino == ino && matches!(j.kind, crate::LifecycleKind::RecoverSize { .. }))
        {
            return;
        }
        let inc = self.config.writeable_range_max_inc_objs;
        let i = &self.inodes[&ino];
        let cap = &i.caps[&client];
        let old_max = i.core.client_ranges.get(&client).copied().unwrap_or(0);
        let mut new_max = old_max;
        let mut change_max = false;
        if i.is_file() {
            if (cap.issued() | cap.wanted()) & CEPH_CAP_ANY_FILE_WR != 0 {
                if m.max_size > old_max {
                    new_max = calc_new_max_size(&i.core.layout, m.max_size, inc);
                    change_max = true;
                } else {
                    let nm = calc_new_max_size(&i.core.layout, m.size, inc);
                    if nm > old_max {
                        new_max = nm;
                        change_max = true;
                    }
                }
            } else if old_max != 0 {
                new_max = 0;
                change_max = true;
            }
        }
        if dirty == 0 && !change_max {
            return;
        }
        let cap_id = cap.cap_id;
        let epoch = self.session_epoch(client);
        let mut txn = Txn::new(if dirty != 0 {
            RecordKind::CapUpdate
        } else {
            RecordKind::ClientRanges
        });
        self.touch(&mut txn, ino);
        self.update_cap_fields(ino, dirty, m);
        if change_max {
            let c = &mut self.inodes.get_mut(&ino).unwrap().core;
            if new_max != 0 {
                c.client_ranges.insert(client, new_max);
            } else {
                c.client_ranges.remove(&client);
            }
        }
        self.update_file_rstat(&mut txn, ino);
        if dirty != 0 {
            if m.flush_tid != 0 {
                txn.rec.client_flush = Some(ClientFlush {
                    client,
                    session_incarnation: epoch,
                    tid: m.flush_tid,
                    oldest_flush_tid: m.oldest_flush_tid,
                });
                txn.deferred.push(Deferred::CompleteFlush {
                    client,
                    epoch,
                    tid: m.flush_tid,
                });
            }
            txn.deferred.push(Deferred::FlushAck {
                client,
                epoch,
                ino,
                cap_id,
                seq: m.seq,
                caps: m.caps,
                dirty,
                flush_tid: m.flush_tid,
                oldest_flush_tid: m.oldest_flush_tid,
            });
        }
        if change_max && new_max > old_max {
            txn.deferred.push(Deferred::ShareMaxSize {
                ino,
                clients: vec![client],
            });
        }
        self.submit(txn, None, out);
    }

    /// `Locker::_update_cap_fields`.
    fn update_cap_fields(&mut self, ino: Ino, dirty: u32, m: &CapUpdate) {
        if dirty == 0 {
            return;
        }
        let max_xattr = self.config.max_xattr_size;
        let c = &mut self.inodes.get_mut(&ino).unwrap().core;
        if m.ctime > c.ctime {
            c.ctime = m.ctime;
        }
        if m.change_attr > c.change_attr {
            c.change_attr = m.change_attr;
        }
        if dirty & (CEPH_CAP_FILE_EXCL | CEPH_CAP_FILE_WR) != 0 {
            if (dirty & CEPH_CAP_FILE_WR != 0 && m.mtime > c.mtime)
                || (dirty & CEPH_CAP_FILE_EXCL != 0 && m.mtime != c.mtime)
            {
                c.mtime = m.mtime;
            }
            if c.is_file() && m.size > c.size {
                c.size = m.size;
            }
            if dirty & CEPH_CAP_FILE_EXCL != 0 && m.atime != c.atime {
                c.atime = m.atime;
            }
            if dirty & CEPH_CAP_FILE_EXCL != 0 && seq_cmp(c.time_warp_seq, m.time_warp_seq) < 0 {
                c.time_warp_seq = m.time_warp_seq;
            }
        }
        if dirty & CEPH_CAP_AUTH_EXCL != 0 {
            c.uid = m.uid;
            c.gid = m.gid;
            c.mode = (c.mode & S_IFMT) | (m.mode & !S_IFMT);
            if !m.btime.is_zero() {
                c.btime = m.btime;
            }
        }
        if dirty & CEPH_CAP_XATTR_EXCL != 0
            && let Some((ver, map)) = &m.xattrs
            && *ver > c.xattr_version
        {
            let bytes: u64 = map.iter().map(|(k, v)| (k.len() + v.len()) as u64).sum();
            // An oversized blob is ignored but the version still
            // advances (`Locker::decode_new_xattrs`).
            if bytes <= max_xattr {
                c.xattrs = map.clone();
            }
            c.xattr_version = *ver;
        }
    }

    /// Handle `MClientCapRelease`.
    pub fn handle_cap_release(
        &mut self,
        client: ClientId,
        items: &[CapReleaseItem],
    ) -> Vec<Effect> {
        let mut out = Vec::new();
        if self.is_failed() || self.recovery_semantics == crate::RecoverySemantics::Legacy2 {
            return out;
        }
        if !self.sessions.contains_key(&client) {
            return out;
        }
        for it in items {
            let Some(cap) = self
                .inodes
                .get_mut(&it.ino)
                .and_then(|i| i.caps.get_mut(&client))
            else {
                continue;
            };
            if cap.cap_id != it.cap_id || it.mseq < cap.mseq {
                continue;
            }
            if it.issue_seq < cap.last_issue {
                // A newer reply re-issued the cap after the client released it.
                cap.clean_revoke_from(it.issue_seq);
                self.eval_caps(it.ino, &mut out);
                continue;
            }
            self.remove_cap(it.ino, client, false, &mut out);
            if self.is_failed() {
                out.clear();
                return out;
            }
        }
        self.run_parked(&mut out);
        out
    }

    /// `Locker::remove_client_cap`: drop the cap, the client's file locks on
    /// the inode and its writeable range; purge an unlinked inode whose last
    /// cap went away.
    pub(crate) fn remove_cap(
        &mut self,
        ino: Ino,
        client: ClientId,
        kill: bool,
        out: &mut Vec<Effect>,
    ) {
        let Some(i) = self.inodes.get_mut(&ino) else {
            return;
        };
        if i.caps.remove(&client).is_none() {
            return;
        }
        if let Some(s) = self.sessions.get_mut(&client) {
            s.caps.remove(&ino);
        }
        self.drop_client_locks(ino, client);
        self.cleanup_client_range(ino, client, kill, out);
    }

    /// Retire a durable writer bound independently of volatile cap recovery.
    /// A missing/restored client has no cap, but still needs the same intent.
    pub(super) fn cleanup_client_range(
        &mut self,
        ino: Ino,
        client: ClientId,
        kill: bool,
        out: &mut Vec<Effect>,
    ) {
        let Some(i) = self.inodes.get(&ino) else {
            return;
        };
        if self
            .jobs
            .values()
            .any(|j| j.ino == ino && matches!(j.kind, crate::LifecycleKind::RecoverSize { .. }))
        {
            // The existing probe covers every durable writer range. Keep all
            // bounds until its completion transaction, including later drops.
            self.maybe_purge_stray(ino, out);
            self.eval_caps(ino, out);
            return;
        }
        let range = i.core.client_ranges.get(&client).copied();
        let recovery_bound = i.core.max_client_range();
        let doomed = self.grants_released()
            && !self.retains_retry_target(ino)
            && i.core.nlink == 0
            && i.caps.is_empty()
            && !self.jobs.values().any(|j| j.ino == ino);
        if let Some(max) = range
            && !doomed
        {
            let mut txn = Txn::new(RecordKind::ClientRanges);
            self.touch(&mut txn, ino);
            if !kill {
                self.inodes
                    .get_mut(&ino)
                    .unwrap()
                    .core
                    .client_ranges
                    .remove(&client);
            }
            if kill {
                let layout = self.inodes[&ino].core.layout.clone();
                txn.deferred.push(Deferred::RecoverFileSize {
                    ino,
                    layout,
                    max_size: recovery_bound.max(max),
                });
            }
            self.submit(txn, None, out);
            if self.is_failed() {
                out.clear();
                return;
            }
        }
        self.maybe_purge_stray(ino, out);
        self.eval_caps(ino, out);
    }

    /// The session closed (`kill == false`) or was evicted (`kill == true`):
    /// drop its waiting requests, file locks and caps (purging unlinked
    /// inodes it kept alive, recovering sizes of files it was writing when
    /// killed), and its idempotency tables.
    pub fn close_session(&mut self, client: ClientId, kill: bool) -> Vec<Effect> {
        if self.is_failed() || self.recovery_semantics == crate::RecoverySemantics::Legacy2 {
            return Vec::new();
        }
        let mut out = Vec::new();
        let Some(s) = self.sessions.get(&client) else {
            return out;
        };
        let epoch = s.epoch;
        let caps: Vec<Ino> = s.caps.iter().copied().collect();
        let locks: Vec<Ino> = s.lock_inos.iter().copied().collect();
        self.drop_requests_of(client, &mut out);
        if self.is_failed() {
            out.clear();
            return out;
        }
        for ino in locks {
            self.drop_client_locks(ino, client);
        }
        for ino in caps {
            self.remove_cap(ino, client, kill, &mut out);
            if self.is_failed() {
                out.clear();
                return out;
            }
        }
        for r in self.ino_alloc.revoke_delegations(client) {
            let _ = r;
        }
        let mut rec = JournalRecord::new(RecordKind::SessionClose);
        rec.closed_sessions.push(crate::SessionKey {
            client,
            incarnation: epoch,
        });
        let txn = Txn {
            rec,
            ..Txn::new(RecordKind::SessionClose)
        };
        self.submit(txn, None, &mut out);
        self.run_parked(&mut out);
        out
    }

    /// The session missed its renewals (`Locker::revoke_stale_caps`): its
    /// caps become invalid and are ignored by the policy; revocations in
    /// progress are forced, except that a client still owing write caps is
    /// to be evicted.
    pub fn session_stale(&mut self, client: ClientId) -> Vec<Effect> {
        if self.is_failed() || self.recovery_semantics == crate::RecoverySemantics::Legacy2 {
            return Vec::new();
        }
        let mut out = Vec::new();
        let Some(s) = self.sessions.get_mut(&client) else {
            return out;
        };
        s.state = SessionState::Stale;
        s.cap_gen = s.cap_gen.wrapping_add(1);
        let caps: Vec<Ino> = s.caps.iter().copied().collect();
        for ino in &caps {
            let Some(cap) = self
                .inodes
                .get_mut(ino)
                .and_then(|i| i.caps.get_mut(&client))
            else {
                continue;
            };
            let revoking = cap.revoking();
            if revoking == 0 {
                continue;
            }
            if revoking & CEPH_CAP_ANY_WR != 0 {
                if self.evicting.insert(client) {
                    out.push(Effect::EvictClient { client });
                }
            } else {
                cap.force_revoke();
            }
        }
        for ino in caps {
            self.eval_caps(ino, &mut out);
        }
        self.run_parked(&mut out);
        out
    }

    /// A stale session renewed (`Locker::resume_stale_caps`): re-issue its
    /// caps (the client reset its view to `p`).
    pub fn session_resume(&mut self, client: ClientId) -> Vec<Effect> {
        if self.is_failed() || self.recovery_semantics == crate::RecoverySemantics::Legacy2 {
            return Vec::new();
        }
        let mut out = Vec::new();
        let Some(s) = self.sessions.get_mut(&client) else {
            return out;
        };
        if s.state != SessionState::Stale {
            return out;
        }
        s.state = SessionState::Open;
        let caps: Vec<Ino> = s.caps.iter().copied().collect();
        for ino in caps {
            self.eval_caps(ino, &mut out);
        }
        self.run_parked(&mut out);
        out
    }

    /// Client reconnect after an MDS restart (one cap record of
    /// `MClientReconnect`, `CInode::reconnect_cap` + `recover_filelocks`):
    /// recreate the client's cap with its `cap_id` and the bits it holds
    /// (sequence numbers restart at 0, as the client zeroes them), and
    /// reinstate its file locks verbatim. ESTALE for an unknown inode: the
    /// daemon then tells the client to drop the cap. Production calls
    /// `reconnect_chunk`, `finish_reconnect`, then admits
    /// `EngineInput::RecoveryControlsDrained` after the control cut drains.
    #[allow(clippy::too_many_arguments)]
    pub fn reconnect_cap(
        &mut self,
        client: ClientId,
        ino: Ino,
        cap_id: u64,
        wanted: u32,
        issued: u32,
        fcntl_locks: &[FileLock],
        flock_locks: &[FileLock],
    ) -> Result<(), Errno> {
        if self.is_failed() || self.recovery_semantics == crate::RecoverySemantics::Legacy2 {
            return Err(Errno::EIO);
        }
        if cap_id == u64::MAX {
            return Err(Errno::EINVAL);
        }
        let Some(sess) = self.sessions.get(&client) else {
            return Err(Errno::ESTALE);
        };
        let cap_gen = sess.cap_gen;
        let Some(i) = self.inodes.get_mut(&ino) else {
            return Err(Errno::ESTALE);
        };
        let mut bits = issued | CEPH_CAP_PIN;
        if i.is_dir() {
            bits &= !(CEPH_CAP_ANY_DIR_OPS | CEPH_CAP_FILE_WR | CEPH_CAP_FILE_BUFFER);
        }
        let cap = i
            .caps
            .entry(client)
            .or_insert_with(|| Capability::new(cap_id, cap_gen));
        // Repeated reconnect chunks replace this client's reconstruction.
        *cap = Capability::new(cap_id, cap_gen);
        cap.issue_norevoke(bits);
        cap.set_wanted(wanted);
        cap.last_sent = 0;
        cap.last_issue = 0;
        let fix = |l: &FileLock| FileLock { client, ..*l };
        let f: Vec<FileLock> = fcntl_locks.iter().map(fix).collect();
        let k: Vec<FileLock> = flock_locks.iter().map(fix).collect();
        i.fcntl_locks.remove_all_from(client);
        i.flock_locks.remove_all_from(client);
        i.fcntl_locks.reinstate(&f);
        i.flock_locks.reinstate(&k);
        self.last_cap_id = self.last_cap_id.max(cap_id);
        self.sessions.get_mut(&client).unwrap().caps.insert(ino);
        if !self.grants_released() {
            let key = crate::SessionKey {
                client,
                incarnation: self.session_epoch(client),
            };
            self.max_size_queue.insert((ino, key));
        }
        self.note_lock_state(ino, client);
        Ok(())
    }

    /// Legacy evaluation helper (`MDCache::reissue_all_caps` at `active_start`).
    /// It does not open either checked-restore recovery gate. Production uses
    /// `finish_reconnect`, then the admitted `RecoveryControlsDrained` marker.
    pub fn reconnect_done(&mut self) -> Vec<Effect> {
        if self.is_failed() || self.recovery_semantics == crate::RecoverySemantics::Legacy2 {
            return Vec::new();
        }
        if self.grants_released() {
            self.queue_unclaimed_strays();
        }
        let mut out = Vec::new();
        let inos: Vec<Ino> = self
            .inodes
            .iter()
            .filter(|(_, i)| !i.caps.is_empty())
            .map(|(k, _)| *k)
            .collect();
        for ino in inos {
            self.eval_caps(ino, &mut out);
        }
        self.run_parked(&mut out);
        out
    }
}
