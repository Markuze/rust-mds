//! SETFILELOCK and GETFILELOCK (`Server::handle_client_file_setlock`,
//! `handle_client_file_readlock`).
//!
//! Lock state is not journaled and SETFILELOCK is not recorded in the
//! completed-request table (a resent lock request is executed again, which
//! is harmless: re-locking merges, unlocking is idempotent). A blocking
//! request waits in the inode's queue and is retried whenever something
//! changes; it fails with EINTR when its queued wait was cancelled (an
//! `*_INTR` unlock, an unlock of the same range by the same owner, or the
//! client's cleanup). fcntl requests that would close a waits-for cycle
//! (up to 5 hops, `MAX_DEADLK_DEPTH`) fail with EDEADLK.

use std::collections::BTreeSet;

use super::{Engine, Exec, Pending};
use crate::consts::*;
use crate::effects::*;
use crate::errno::Errno;
use crate::filelock::{FileLock, LockAttempt, LockOwner};
use crate::journal::Journal;
use crate::request::FileLockArgs;
use crate::types::*;

const MAX_DEADLK_DEPTH: usize = 5;

/// The lock record of a request; the client id always comes from the session.
pub(crate) fn to_filelock(client: ClientId, a: &FileLockArgs) -> FileLock {
    FileLock {
        start: a.start,
        length: a.length,
        client,
        owner: a.owner,
        pid: a.pid,
        ltype: a.ltype,
    }
}

impl<J: Journal> Engine<J> {
    fn lock_table(&mut self, ino: Ino, flock: bool) -> &mut crate::filelock::LockTable {
        let i = self.inodes.get_mut(&ino).unwrap();
        if flock {
            &mut i.flock_locks
        } else {
            &mut i.fcntl_locks
        }
    }

    /// Keep the per-session and per-engine lock indexes in sync.
    pub(crate) fn note_lock_state(&mut self, ino: Ino, client: ClientId) {
        let (has_any, client_has) = match self.inodes.get(&ino) {
            Some(i) => {
                let mine = |t: &crate::filelock::LockTable| {
                    t.held().iter().any(|l| l.client == client)
                        || t.waiting().iter().any(|l| l.client == client)
                };
                (
                    i.has_file_locks(),
                    mine(&i.fcntl_locks) || mine(&i.flock_locks),
                )
            }
            None => (false, false),
        };
        if has_any {
            self.lock_inodes.insert(ino);
        } else {
            self.lock_inodes.remove(&ino);
        }
        if let Some(s) = self.sessions.get_mut(&client) {
            if client_has {
                s.lock_inos.insert(ino);
            } else {
                s.lock_inos.remove(&ino);
            }
        }
    }

    /// Would `req`, blocked by `blockers`, close a waits-for cycle of fcntl
    /// owners (`ceph_lock_state_t::is_deadlock`)?
    fn fcntl_deadlock(&self, req: &FileLock, blockers: &[FileLock]) -> bool {
        let me = req.owner_key();
        let mut frontier: BTreeSet<LockOwner> = blockers
            .iter()
            .filter(|b| !(req.ltype == CEPH_LOCK_SHARED && b.ltype == CEPH_LOCK_SHARED))
            .map(|b| b.owner_key())
            .collect();
        let mut seen = BTreeSet::new();
        for _ in 0..=MAX_DEADLK_DEPTH {
            if frontier.contains(&me) {
                return true;
            }
            let mut next = BTreeSet::new();
            for o in &frontier {
                if !seen.insert(*o) {
                    continue;
                }
                for ino in &self.lock_inodes {
                    let Some(i) = self.inodes.get(ino) else {
                        continue;
                    };
                    let t = &i.fcntl_locks;
                    for w in t.waiting().iter().filter(|w| w.owner_key() == *o) {
                        for b in t.blockers(w) {
                            if !(w.ltype == CEPH_LOCK_SHARED && b.ltype == CEPH_LOCK_SHARED) {
                                next.insert(b.owner_key());
                            }
                        }
                    }
                }
            }
            if next.is_empty() {
                return false;
            }
            frontier = next;
        }
        false
    }

    pub(crate) fn op_setfilelock(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        a: FileLockArgs,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let t = match self.resolve(&path) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        let (flock, interrupt) = match a.rule {
            CEPH_LOCK_FCNTL => (false, false),
            CEPH_LOCK_FLOCK => (true, false),
            CEPH_LOCK_FCNTL_INTR => (false, true),
            CEPH_LOCK_FLOCK_INTR => (true, true),
            _ => return self.reply_error(p, Errno::EOPNOTSUPP, out),
        };
        let fl = to_filelock(p.client, &a);
        match a.ltype {
            CEPH_LOCK_UNLOCK => {
                let table = self.lock_table(t, flock);
                table.remove_waiting(&fl);
                if !interrupt {
                    table.unlock(&fl);
                }
                self.note_lock_state(t, p.client);
                let s = self.spec(p, 0);
                self.reply_now(s, out)
            }
            CEPH_LOCK_SHARED | CEPH_LOCK_EXCL => {
                if p.flock_was_waiting && !self.lock_table(t, flock).is_waiting(&fl) {
                    return self.reply_error(p, Errno::EINTR, out);
                }
                match self.lock_table(t, flock).try_lock(&fl) {
                    LockAttempt::Granted => {
                        self.note_lock_state(t, p.client);
                        let s = self.spec(p, 0);
                        self.reply_now(s, out)
                    }
                    LockAttempt::Blocked(blockers) => {
                        if p.flock_was_waiting {
                            return Exec::Wait;
                        }
                        if !a.wait {
                            return self.reply_error(p, Errno::EAGAIN, out);
                        }
                        if !flock && self.fcntl_deadlock(&fl, &blockers) {
                            return self.reply_error(p, Errno::EDEADLK, out);
                        }
                        self.lock_table(t, flock).add_waiting(&fl);
                        p.flock_was_waiting = true;
                        self.note_lock_state(t, p.client);
                        Exec::Wait
                    }
                }
            }
            _ => self.reply_error(p, Errno::EINVAL, out),
        }
    }

    pub(crate) fn op_getfilelock(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        a: FileLockArgs,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let t = match self.resolve(&path) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        let flock = match a.rule {
            CEPH_LOCK_FCNTL => false,
            CEPH_LOCK_FLOCK => true,
            _ => return self.reply_error(p, Errno::EINVAL, out),
        };
        let fl = to_filelock(p.client, &a);
        let r = self.lock_table(t, flock).test(&fl);
        let mut s = self.spec(p, 0);
        s.extra = ReplyExtra::FileLock(r);
        self.reply_now(s, out)
    }

    /// Drop every file lock (held and waiting) of a client on an inode.
    /// Returns true if held locks were removed.
    pub(crate) fn drop_client_locks(&mut self, ino: Ino, client: ClientId) -> bool {
        let Some(i) = self.inodes.get_mut(&ino) else {
            return false;
        };
        let a = i.fcntl_locks.remove_all_from(client);
        let b = i.flock_locks.remove_all_from(client);
        self.note_lock_state(ino, client);
        a || b
    }
}
