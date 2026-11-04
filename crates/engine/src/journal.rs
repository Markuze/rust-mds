//! Journal records and the journal interface.
//!
//! Every mutation produces exactly one self-contained record (design 5.1,
//! "WAL records are self-contained, one per transaction, with per-object
//! versions"): full images of every inode it changed (each carrying its
//! `version`), the dentry changes in the order they were made, inode
//! allocation, purges, and the client bookkeeping that must be atomic with
//! the change (the completed request with its created inode, the completed
//! cap flush). Replaying the records in sequence order on an empty engine
//! (`Engine::replay`) rebuilds the namespace.
//!
//! The engine hands records to a [`Journal`] and sends replies, FLUSH_ACKs and
//! `max_size` grants only after the journal reports the record committed
//! (safe replies only, decision 5). [`MemJournal`] keeps records in memory and
//! can commit at once (the default) or on demand, which tests use to check
//! that nothing leaks before commit.

use crate::durability::{JobId, LifecycleIntent, RecoverySemantics, SessionKey};
use crate::ino::InoAllocState;
use crate::inode::InodeCore;
use crate::types::{ClientId, Ino};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum JournalError {
    #[error("journal unavailable")]
    Unavailable,
    #[error("journal capacity contract violated")]
    Capacity,
    #[error("record exceeds reserved byte bound")]
    RecordTooLarge,
    #[error("journal ownership lost")]
    Fenced,
    #[error("invalid journal state")]
    Corrupt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalEvent {
    Committed(JournalSeq),
    Failed(JournalError),
}

/// Journal sequence number; 0 means "nothing".
pub type JournalSeq = u64;

/// What kind of transaction a record describes (for debugging and replay
/// checks; replay itself is driven by the record's contents).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    Mkfs,
    Create,
    Mknod,
    Mkdir,
    Symlink,
    Link,
    Unlink,
    Rmdir,
    Rename,
    Setattr,
    Open,
    Setxattr,
    Rmxattr,
    SetVxattr,
    CapUpdate,
    ClientRanges,
    Purge,
    SessionClose,
    LifecycleComplete,
}

/// A successful mutating request, remembered so that a retry is answered
/// instead of executed again (`Session::add_completed_request`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedRequest {
    pub client: ClientId,
    pub session_incarnation: u64,
    pub tid: u64,
    pub op: u32,
    pub result: i32,
    pub created_ino: Option<Ino>,
    /// The client's oldest outstanding tid when it sent the request; entries
    /// below it can be trimmed.
    pub oldest_client_tid: u64,
    /// Original reply target. This is not evidence that CREATE created it.
    pub reply_target_ino: Option<Ino>,
}

/// A cap flush, remembered so that a resent flush is acknowledged without
/// being applied again (`session_info_t::completed_flushes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientFlush {
    pub client: ClientId,
    pub session_incarnation: u64,
    pub tid: u64,
    pub oldest_flush_tid: u64,
}

/// A dentry change: `link` is the new linkage, `None` removes the name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DentryUpdate {
    pub dir: Ino,
    pub name: Vec<u8>,
    pub link: Option<DentryLink>,
}

/// Target of a linked dentry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DentryLink {
    pub ino: Ino,
    pub primary: bool,
}

/// One transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalRecord {
    pub recovery_semantics: RecoverySemantics,
    pub kind: RecordKind,
    pub completed_request: Option<CompletedRequest>,
    pub client_flush: Option<ClientFlush>,
    /// Inode numbers allocated by this transaction.
    pub allocated: Vec<Ino>,
    /// Allocator state after the transaction, when it changed.
    pub ino_alloc: Option<InoAllocState>,
    /// Images of every inode changed by this transaction, after the change.
    pub inodes: Vec<InodeCore>,
    /// Dentry changes, in the order they were made.
    pub dentries: Vec<DentryUpdate>,
    /// Inodes removed for good (their data objects are purged after commit).
    pub purged: Vec<Ino>,
    /// Sessions whose per-session state (completed requests and flushes) is
    /// dropped by this transaction.
    pub closed_sessions: Vec<SessionKey>,
    pub lifecycle_intents: Vec<LifecycleIntent>,
    pub completed_jobs: Vec<JobId>,
}

impl JournalRecord {
    /// Conservative admission bound for the versioned persistent encoding.
    /// The adapter must also enforce its encoded-byte limit before I/O.
    pub fn size_bound(&self) -> usize {
        let mut n = 4096usize;
        for i in &self.inodes {
            n = n.saturating_add(
                4096 + i.symlink.len()
                    + i.fscrypt_auth.len()
                    + i.fscrypt_file.len()
                    + i.stray_auth_path.as_ref().map_or(0, Vec::len)
                    + i.layout.pool_ns.len()
                    + i.xattrs
                        .iter()
                        .map(|(k, v)| k.len() + v.len() + 32)
                        .sum::<usize>()
                    + i.client_ranges.len() * 32
                    + i.old_pools.len() * 16,
            );
            n = n.saturating_add(
                i.old_layouts
                    .iter()
                    .map(|l| 128 + l.pool_ns.len())
                    .sum::<usize>(),
            );
        }
        for d in &self.dentries {
            n = n.saturating_add(128 + d.name.len());
        }
        if let Some(a) = &self.ino_alloc {
            n = n.saturating_add(a.free.len() * 32 + a.delegated.len() * 48 + 64);
        }
        n = n.saturating_add(
            self.allocated.len() * 16
                + self.purged.len() * 16
                + self.closed_sessions.len() * 32
                + self.completed_jobs.len() * 16,
        );
        for j in &self.lifecycle_intents {
            let layouts: &[crate::FileLayout] = match &j.kind {
                crate::LifecycleKind::Purge { layouts, .. }
                | crate::LifecycleKind::RecoverSize { layouts, .. } => layouts,
                crate::LifecycleKind::Truncate { layout, .. } => std::slice::from_ref(layout),
            };
            n = n
                .saturating_add(256 + layouts.iter().map(|l| 128 + l.pool_ns.len()).sum::<usize>());
        }
        n
    }

    pub fn new(kind: RecordKind) -> JournalRecord {
        JournalRecord {
            recovery_semantics: RecoverySemantics::RetainedTargets3,
            kind,
            completed_request: None,
            client_flush: None,
            allocated: Vec::new(),
            ino_alloc: None,
            inodes: Vec::new(),
            dentries: Vec::new(),
            purged: Vec::new(),
            closed_sessions: Vec::new(),
            lifecycle_intents: Vec::new(),
            completed_jobs: Vec::new(),
        }
    }
}

/// Compatibility interface for the single in-memory journal.
///
/// The engine calls `submit` once per transaction and then checks
/// `committed_seq`; records up to that sequence are durable. When something
/// waits for a record (another request, a sync cap flush) the engine calls
/// `flush` to ask for an immediate commit instead of waiting for group commit.
/// A journal that commits asynchronously tells the daemon, which then calls
/// `Engine::journal_committed`.
///
/// G1–G3 keep this scalar sequence on stream 0. Do not grow it into multiple
/// streams. Those need the per-stream commit tokens in `design/transactions.md`.
pub trait Journal {
    /// Append a record; returns its sequence number (strictly increasing,
    /// starting at 1).
    fn submit(&mut self, record: JournalRecord) -> Result<JournalSeq, JournalError>;
    /// Highest sequence number known durable (every lower one is durable too).
    fn committed_seq(&self) -> JournalSeq;
    /// Start committing everything submitted so far.
    fn flush(&mut self);
}

/// In-memory journal.
#[derive(Debug, Clone)]
pub struct MemJournal {
    records: Vec<(JournalSeq, JournalRecord)>,
    committed: JournalSeq,
    auto_commit: bool,
    flushes: u64,
    base_seq: JournalSeq,
}

impl Default for MemJournal {
    fn default() -> Self {
        MemJournal::new()
    }
}

impl MemJournal {
    /// A journal that commits every record as soon as it is submitted.
    pub fn new() -> MemJournal {
        MemJournal {
            records: Vec::new(),
            committed: 0,
            auto_commit: true,
            flushes: 0,
            base_seq: 0,
        }
    }

    /// A journal that commits only when `commit_upto`/`commit_all` is called
    /// (or on `flush` if `commit_on_flush` is set later).
    pub fn manual() -> MemJournal {
        MemJournal {
            auto_commit: false,
            ..MemJournal::new()
        }
    }

    /// Initialize an empty journal adapter after checked restore/replay.
    pub fn at_seq(seq: JournalSeq, auto_commit: bool) -> Self {
        Self {
            base_seq: seq,
            committed: seq,
            auto_commit,
            ..Self::new()
        }
    }

    pub fn set_auto_commit(&mut self, on: bool) {
        self.auto_commit = on;
        if on {
            self.commit_all();
        }
    }

    pub fn commit_upto(&mut self, seq: JournalSeq) {
        let last = self.last_seq();
        self.committed = self.committed.max(seq.min(last));
    }

    pub fn commit_all(&mut self) {
        self.committed = self.last_seq();
    }

    pub fn last_seq(&self) -> JournalSeq {
        self.records
            .last()
            .map(|(s, _)| *s)
            .unwrap_or(self.base_seq)
    }

    pub fn records(&self) -> &[(JournalSeq, JournalRecord)] {
        &self.records
    }

    /// Number of `flush` calls (a waiter asked for an early commit).
    pub fn flush_count(&self) -> u64 {
        self.flushes
    }

    /// Records not yet committed.
    pub fn uncommitted(&self) -> usize {
        self.records
            .iter()
            .filter(|(s, _)| *s > self.committed)
            .count()
    }
}

impl Journal for MemJournal {
    fn submit(&mut self, record: JournalRecord) -> Result<JournalSeq, JournalError> {
        let seq = self
            .last_seq()
            .checked_add(1)
            .ok_or(JournalError::Corrupt)?;
        self.records.push((seq, record));
        if self.auto_commit {
            self.committed = seq;
        }
        Ok(seq)
    }

    fn committed_seq(&self) -> JournalSeq {
        self.committed
    }

    fn flush(&mut self) {
        self.flushes += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_and_manual_commit() {
        let mut j = MemJournal::new();
        assert_eq!(j.submit(JournalRecord::new(RecordKind::Mkfs)), Ok(1));
        assert_eq!(j.committed_seq(), 1);
        let mut m = MemJournal::manual();
        m.submit(JournalRecord::new(RecordKind::Create)).unwrap();
        m.submit(JournalRecord::new(RecordKind::Create)).unwrap();
        assert_eq!(m.committed_seq(), 0);
        assert_eq!(m.uncommitted(), 2);
        m.commit_upto(1);
        assert_eq!(m.committed_seq(), 1);
        m.commit_upto(99);
        assert_eq!(m.committed_seq(), 2);
        m.flush();
        assert_eq!(m.flush_count(), 1);
    }
}
