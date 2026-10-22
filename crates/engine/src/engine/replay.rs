//! Rebuilding an engine from its journal records.
//!
//! Records are self-contained: inode images carry every persistent field
//! (and their `version`), dentry updates are applied in order, purged inodes
//! are dropped, and the completed-request and completed-flush tables are
//! restored from the records that created them. Caps, holds and file locks
//! are runtime state and come back through client reconnect.

use super::Engine;
use crate::ino::InoAllocator;
use crate::inode::{Dentry, DentryKey, Inode};
use crate::journal::{Journal, JournalRecord};
use crate::session::Session;
use crate::types::*;

impl<J: Journal> Engine<J> {
    /// Build an engine by replaying `records` in order (the output of a
    /// previous engine's journal). The new engine writes to `journal`.
    pub fn replay<'a>(
        config: super::EngineConfig,
        journal: J,
        records: impl IntoIterator<Item = &'a JournalRecord>,
    ) -> Engine<J> {
        let mut e = Engine::empty(config, journal);
        for r in records {
            e.apply_record(r);
        }
        e
    }

    fn unlink_target_state(&mut self, dir: Ino, name: &[u8], old: Dentry) {
        if let Some(t) = self.inodes.get_mut(&old.ino) {
            if old.primary {
                if t.parent
                    .as_ref()
                    .is_some_and(|(p, n)| *p == dir && n == name)
                {
                    t.parent = None;
                }
            } else {
                t.remote_links.remove(&(dir, name.to_vec()));
            }
        }
    }

    /// Apply one record to the in-memory state.
    pub fn apply_record(&mut self, r: &JournalRecord) {
        if self.is_failed() {
            return;
        }
        for job in &r.lifecycle_intents {
            self.jobs.insert(job.id, job.clone());
            self.next_job_id = self.next_job_id.max(job.id.saturating_add(1));
        }
        for id in &r.completed_jobs {
            self.jobs.remove(id);
        }
        if let Some(st) = &r.ino_alloc {
            self.ino_alloc = InoAllocator::restore(st);
        }
        for core in &r.inodes {
            self.committed_inodes.insert(core.ino, core.clone());
            match self.inodes.get_mut(&core.ino) {
                Some(i) => i.core = core.clone(),
                None => {
                    self.inodes.insert(core.ino, Inode::new(core.clone()));
                }
            }
        }
        for d in &r.dentries {
            let Some(dir) = self.inodes.get_mut(&d.dir) else {
                continue;
            };
            let key = DentryKey::new(dir.core.dir_hash, &d.name);
            let old = match &d.link {
                Some(l) => dir.entries.insert(
                    key,
                    Dentry {
                        ino: l.ino,
                        primary: l.primary,
                    },
                ),
                None => dir.entries.remove(&key),
            };
            if let Some(old) = old {
                self.unlink_target_state(d.dir, &d.name, old);
            }
            if let Some(l) = &d.link
                && let Some(t) = self.inodes.get_mut(&l.ino)
            {
                if l.primary {
                    t.parent = Some((d.dir, d.name.clone()));
                    t.remote_links.remove(&(d.dir, d.name.clone()));
                } else {
                    t.remote_links.insert((d.dir, d.name.clone()));
                }
            }
        }
        for ino in &r.purged {
            self.inodes.remove(ino);
            self.committed_inodes.remove(ino);
        }
        if let Some(cr) = &r.completed_request {
            let s = self
                .sessions
                .entry(cr.client)
                .or_insert_with(|| Session::with_epoch(cr.client, cr.session_incarnation));
            s.trim_completed_requests(cr.oldest_client_tid);
            if cr.tid >= s.oldest_client_tid {
                s.completed_requests.insert(cr.tid, cr.clone());
            }
        }
        if let Some(cf) = &r.client_flush {
            let s = self
                .sessions
                .entry(cf.client)
                .or_insert_with(|| Session::with_epoch(cf.client, cf.session_incarnation));
            s.trim_completed_flushes(cf.oldest_flush_tid);
            if cf.tid >= s.oldest_flush_tid {
                s.completed_flushes.insert(cf.tid);
            }
        }
        for c in &r.closed_sessions {
            if self.session_matches(c.client, c.incarnation) {
                self.sessions.remove(&c.client);
            }
        }
    }

    /// Compare the persistent namespace with another engine's (for tests of
    /// replay): inode images and directory entries.
    pub fn namespace_eq<K: Journal>(&self, other: &Engine<K>) -> Result<(), String> {
        let a: Vec<Ino> = self.inodes.keys().copied().collect();
        let b: Vec<Ino> = other.inodes.keys().copied().collect();
        if a != b {
            return Err(format!("inode sets differ: {a:x?} vs {b:x?}"));
        }
        for (ino, i) in &self.inodes {
            let o = &other.inodes[ino];
            if i.core != o.core {
                return Err(format!(
                    "inode {ino:#x} differs:\n{:?}\n{:?}",
                    i.core, o.core
                ));
            }
            if i.entries != o.entries {
                return Err(format!("entries of {ino:#x} differ"));
            }
            if i.parent != o.parent || i.remote_links != o.remote_links {
                return Err(format!("linkage of {ino:#x} differs"));
            }
        }
        Ok(())
    }
}
