//! Advisory byte-range file locks (fcntl and flock), one table per kind per
//! inode.
//!
//! Semantics follow `src/mds/flock.{h,cc}` (`ceph_lock_state_t`) as seen by
//! clients: a lock is owned by `(client, owner)` when bit 63 of `owner` is set
//! (all current clients), else by `(client, owner, pid)`
//! (`ceph_filelock_owner_equal`); `length == 0` means "to end of file"; a lock
//! conflicts with another owner's overlapping lock when either is exclusive;
//! one owner's locks never overlap and same-type adjacent ranges are merged
//! (POSIX "replace" semantics, including atomic upgrade and downgrade);
//! GETLK reports the first conflicting lock in ascending start order or the
//! request with type UNLOCK. flock locks use the same tables with whole-file
//! ranges. Nothing here is journaled: lock state lives in MDS memory only and
//! is reinstated from client reconnect (C++ behaviour).

use crate::consts::{CEPH_LOCK_EXCL, CEPH_LOCK_SHARED, CEPH_LOCK_UNLOCK};
use crate::types::ClientId;

/// One lock record (`struct ceph_filelock`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileLock {
    pub start: u64,
    /// Number of bytes; 0 means to end of file.
    pub length: u64,
    pub client: ClientId,
    pub owner: u64,
    pub pid: u64,
    /// `CEPH_LOCK_SHARED`, `CEPH_LOCK_EXCL` or `CEPH_LOCK_UNLOCK`.
    pub ltype: u8,
}

/// The identity a lock is held by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LockOwner {
    pub client: ClientId,
    pub owner: u64,
    /// The pid, only for old clients that do not set bit 63 of `owner`.
    pub pid: Option<u64>,
}

impl FileLock {
    /// Inclusive last byte of the range.
    pub fn end(&self) -> u64 {
        if self.length == 0 {
            u64::MAX
        } else {
            self.start.saturating_add(self.length - 1)
        }
    }

    fn with_range(mut self, start: u64, end: u64) -> FileLock {
        self.start = start;
        self.length = if end == u64::MAX { 0 } else { end - start + 1 };
        self
    }

    pub fn owner_key(&self) -> LockOwner {
        LockOwner {
            client: self.client,
            owner: self.owner,
            pid: if self.owner & (1u64 << 63) != 0 {
                None
            } else {
                Some(self.pid)
            },
        }
    }

    /// `ceph_filelock_owner_equal`.
    pub fn same_owner(&self, other: &FileLock) -> bool {
        self.owner_key() == other.owner_key()
    }

    pub fn overlaps(&self, other: &FileLock) -> bool {
        self.start <= other.end() && other.start <= self.end()
    }

    /// Overlapping or touching (one byte apart).
    fn touches(&self, other: &FileLock) -> bool {
        self.start <= other.end().saturating_add(1) && other.start <= self.end().saturating_add(1)
    }

    /// Whether `self` (a request) is blocked by `held` of another owner.
    pub fn conflicts_with(&self, held: &FileLock) -> bool {
        !self.same_owner(held)
            && self.overlaps(held)
            && (self.ltype == CEPH_LOCK_EXCL || held.ltype == CEPH_LOCK_EXCL)
    }
}

/// Result of a lock attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockAttempt {
    Granted,
    /// Blocked by these locks of other owners.
    Blocked(Vec<FileLock>),
}

/// The held and waiting locks of one kind on one inode.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LockTable {
    /// Held locks, sorted by start.
    held: Vec<FileLock>,
    /// Queued blocking requests, in arrival order.
    waiting: Vec<FileLock>,
}

impl LockTable {
    pub fn new() -> LockTable {
        LockTable::default()
    }

    pub fn held(&self) -> &[FileLock] {
        &self.held
    }

    pub fn waiting(&self) -> &[FileLock] {
        &self.waiting
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty() && self.waiting.is_empty()
    }

    /// Other owners' held locks that block `req`, in ascending start order.
    pub fn blockers(&self, req: &FileLock) -> Vec<FileLock> {
        self.held
            .iter()
            .filter(|h| req.conflicts_with(h))
            .copied()
            .collect()
    }

    /// Try to take `req` (SHARED or EXCL). On success any queued wait of the
    /// same request is removed and the owner's locks are adjusted.
    pub fn try_lock(&mut self, req: &FileLock) -> LockAttempt {
        let blockers = self.blockers(req);
        if !blockers.is_empty() {
            return LockAttempt::Blocked(blockers);
        }
        self.remove_waiting(req);
        self.replace_range(req, Some(req.ltype));
        LockAttempt::Granted
    }

    /// Release the owner's locks in the range of `req` (type ignored).
    /// Returns true if anything changed.
    pub fn unlock(&mut self, req: &FileLock) -> bool {
        self.replace_range(req, None)
    }

    /// Set `[req.start, req.end()]` of `req`'s owner to `new_type` (or clear
    /// it), keeping the owner's parts outside the range and merging adjacent
    /// same-type ranges.
    fn replace_range(&mut self, req: &FileLock, new_type: Option<u8>) -> bool {
        let (s, e) = (req.start, req.end());
        let mut changed = false;
        let mut kept: Vec<FileLock> = Vec::with_capacity(self.held.len() + 2);
        for l in self.held.drain(..) {
            if !(l.same_owner(req) && l.overlaps(req)) {
                kept.push(l);
                continue;
            }
            changed = true;
            if l.start < s {
                kept.push(l.with_range(l.start, s - 1));
            }
            if l.end() > e {
                kept.push(l.with_range(e + 1, l.end()));
            }
        }
        self.held = kept;
        if let Some(t) = new_type {
            let mut new = *req;
            new.ltype = t;
            // Merge same-owner same-type locks that touch the new range.
            let mut i = 0;
            while i < self.held.len() {
                let l = self.held[i];
                if l.same_owner(&new) && l.ltype == t && l.touches(&new) {
                    let ns = new.start.min(l.start);
                    let ne = new.end().max(l.end());
                    new = new.with_range(ns, ne);
                    self.held.remove(i);
                } else {
                    i += 1;
                }
            }
            self.held.push(new);
            changed = true;
        }
        self.held
            .sort_by_key(|l| (l.start, l.end(), l.client, l.owner, l.pid));
        changed
    }

    /// GETLK: the first conflicting lock of another owner, or `req` with type
    /// UNLOCK (`ceph_lock_state_t::look_for_lock`).
    pub fn test(&self, req: &FileLock) -> FileLock {
        let overlapping: Vec<&FileLock> = self
            .held
            .iter()
            .filter(|h| !h.same_owner(req) && h.overlaps(req))
            .collect();
        if req.ltype == CEPH_LOCK_EXCL {
            if let Some(first) = overlapping.first() {
                return **first;
            }
        } else if let Some(x) = overlapping.iter().find(|h| h.ltype == CEPH_LOCK_EXCL) {
            return **x;
        }
        let mut r = *req;
        r.ltype = CEPH_LOCK_UNLOCK;
        r
    }

    /// Queue a blocking request.
    pub fn add_waiting(&mut self, req: &FileLock) {
        if !self.is_waiting(req) {
            self.waiting.push(*req);
        }
    }

    /// Whether a request with the same start, length and owner is queued
    /// (`ceph_lock_state_t::is_waiting`).
    pub fn is_waiting(&self, req: &FileLock) -> bool {
        self.waiting
            .iter()
            .any(|w| w.start == req.start && w.length == req.length && w.same_owner(req))
    }

    /// Drop a queued request with the same start, length and owner.
    pub fn remove_waiting(&mut self, req: &FileLock) -> bool {
        if let Some(i) = self
            .waiting
            .iter()
            .position(|w| w.start == req.start && w.length == req.length && w.same_owner(req))
        {
            self.waiting.remove(i);
            true
        } else {
            false
        }
    }

    /// Drop everything of a client (`remove_all_from`). Returns true if held
    /// locks were removed (only then are waiters woken, as in C++).
    pub fn remove_all_from(&mut self, client: ClientId) -> bool {
        let before = self.held.len();
        self.held.retain(|l| l.client != client);
        self.waiting.retain(|l| l.client != client);
        self.held.len() != before
    }

    /// Insert held locks verbatim (reconnect reinstatement, no conflict check).
    pub fn reinstate(&mut self, locks: &[FileLock]) {
        self.held.extend_from_slice(locks);
        self.held
            .sort_by_key(|l| (l.start, l.end(), l.client, l.owner, l.pid));
    }
}

/// Is `t` a valid lock type for a lock attempt?
pub fn is_lock_type(t: u8) -> bool {
    t == CEPH_LOCK_SHARED || t == CEPH_LOCK_EXCL
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: u64 = 1 << 63;

    fn lk(client: u64, owner: u64, start: u64, length: u64, t: u8) -> FileLock {
        FileLock {
            start,
            length,
            client,
            owner: owner | O,
            pid: 0,
            ltype: t,
        }
    }

    #[test]
    fn shared_locks_coexist_exclusive_conflicts() {
        let mut t = LockTable::new();
        assert_eq!(
            t.try_lock(&lk(1, 1, 0, 10, CEPH_LOCK_SHARED)),
            LockAttempt::Granted
        );
        assert_eq!(
            t.try_lock(&lk(2, 1, 5, 10, CEPH_LOCK_SHARED)),
            LockAttempt::Granted
        );
        match t.try_lock(&lk(3, 1, 9, 1, CEPH_LOCK_EXCL)) {
            LockAttempt::Blocked(b) => assert_eq!(b.len(), 2),
            other => panic!("{other:?}"),
        }
        // Non-overlapping exclusive is fine.
        assert_eq!(
            t.try_lock(&lk(3, 1, 15, 5, CEPH_LOCK_EXCL)),
            LockAttempt::Granted
        );
    }

    #[test]
    fn same_owner_replace_split_and_merge() {
        let mut t = LockTable::new();
        t.try_lock(&lk(1, 1, 0, 100, CEPH_LOCK_SHARED));
        // Upgrade the middle: splits into S[0,39] X[40,59] S[60,99].
        t.try_lock(&lk(1, 1, 40, 20, CEPH_LOCK_EXCL));
        let h = t.held();
        assert_eq!(h.len(), 3);
        assert_eq!(
            (h[0].start, h[0].end(), h[0].ltype),
            (0, 39, CEPH_LOCK_SHARED)
        );
        assert_eq!(
            (h[1].start, h[1].end(), h[1].ltype),
            (40, 59, CEPH_LOCK_EXCL)
        );
        assert_eq!(
            (h[2].start, h[2].end(), h[2].ltype),
            (60, 99, CEPH_LOCK_SHARED)
        );
        // Downgrade back merges everything.
        t.try_lock(&lk(1, 1, 40, 20, CEPH_LOCK_SHARED));
        assert_eq!(t.held().len(), 1);
        assert_eq!((t.held()[0].start, t.held()[0].end()), (0, 99));
        // Adjacent same-type ranges merge.
        t.try_lock(&lk(1, 1, 100, 0, CEPH_LOCK_SHARED));
        assert_eq!(t.held().len(), 1);
        assert_eq!(t.held()[0].length, 0, "now to end of file");
    }

    #[test]
    fn unlock_punches_holes() {
        let mut t = LockTable::new();
        t.try_lock(&lk(1, 1, 0, 0, CEPH_LOCK_EXCL));
        assert!(t.unlock(&lk(1, 1, 10, 10, CEPH_LOCK_UNLOCK)));
        let h = t.held();
        assert_eq!(h.len(), 2);
        assert_eq!((h[0].start, h[0].end()), (0, 9));
        assert_eq!((h[1].start, h[1].length), (20, 0));
        // Other owners' unlock changes nothing.
        assert!(!t.unlock(&lk(2, 1, 0, 0, CEPH_LOCK_UNLOCK)));
        // Unlock to EOF.
        assert!(t.unlock(&lk(1, 1, 5, 0, CEPH_LOCK_UNLOCK)));
        assert_eq!(t.held().len(), 1);
        assert_eq!(t.held()[0].end(), 4);
    }

    #[test]
    fn getlk_reports_first_conflict() {
        let mut t = LockTable::new();
        t.try_lock(&lk(1, 1, 50, 10, CEPH_LOCK_SHARED));
        t.try_lock(&lk(2, 1, 10, 10, CEPH_LOCK_SHARED));
        let r = t.test(&lk(3, 1, 0, 0, CEPH_LOCK_EXCL));
        assert_eq!((r.client, r.start), (2, 10));
        let r = t.test(&lk(3, 1, 0, 0, CEPH_LOCK_SHARED));
        assert_eq!(r.ltype, CEPH_LOCK_UNLOCK);
        assert_eq!(r.client, 3);
        // Own locks never conflict.
        let r = t.test(&lk(1, 1, 50, 1, CEPH_LOCK_EXCL));
        assert_eq!(r.ltype, CEPH_LOCK_UNLOCK);
    }

    #[test]
    fn pid_matters_only_for_old_owners() {
        let a = FileLock {
            start: 0,
            length: 1,
            client: 1,
            owner: 5,
            pid: 10,
            ltype: CEPH_LOCK_EXCL,
        };
        let b = FileLock { pid: 11, ..a };
        assert!(!a.same_owner(&b));
        let a2 = FileLock { owner: 5 | O, ..a };
        let b2 = FileLock { owner: 5 | O, ..b };
        assert!(a2.same_owner(&b2));
    }

    #[test]
    fn waiting_and_cleanup() {
        let mut t = LockTable::new();
        t.try_lock(&lk(1, 1, 0, 0, CEPH_LOCK_EXCL));
        let w = lk(2, 1, 0, 0, CEPH_LOCK_EXCL);
        t.add_waiting(&w);
        t.add_waiting(&w);
        assert_eq!(t.waiting().len(), 1);
        assert!(t.is_waiting(&w));
        assert!(t.remove_all_from(1));
        assert_eq!(t.try_lock(&w), LockAttempt::Granted);
        assert!(!t.is_waiting(&w), "granted request leaves the queue");
        assert!(!t.remove_all_from(9));
    }
}
