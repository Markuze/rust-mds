//! Wire-free persistence and one-record execution contract.

use crate::{effects::Effect, ino::InoAllocState, inode::*, journal::*, request::*, types::*};

/// Required recovery behavior, selected by the store's semantic frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum RecoverySemantics {
    Legacy2 = 2,
    RetainedTargets3 = 3,
}

/// Startup-only accounting limits for durable retry histories and their targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryLimits {
    pub max_completions: usize,
    pub max_bytes: usize,
}
impl Default for RetryLimits {
    fn default() -> Self {
        Self {
            max_completions: 65_536,
            max_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionKey {
    pub client: ClientId,
    pub incarnation: u64,
}

pub type JobId = u64;
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleIntent {
    pub id: JobId,
    pub ino: Ino,
    pub kind: LifecycleKind,
}
#[derive(Debug, Clone, PartialEq)]
pub enum LifecycleKind {
    Purge {
        layouts: Vec<FileLayout>,
        max_size: u64,
    },
    Truncate {
        layout: FileLayout,
        from: u64,
        size: u64,
        truncate_seq: u32,
    },
    RecoverSize {
        layouts: Vec<FileLayout>,
        max_size: u64,
        inode_version: u64,
        truncate_seq: u32,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleResult {
    Done,
    Recovered { size: u64, mtime: Timestamp },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineCheckpoint {
    pub recovery_semantics: RecoverySemantics,
    pub covered_seq: JournalSeq,
    pub inodes: Vec<CheckpointInode>,
    pub ino_alloc: InoAllocState,
    pub sessions: Vec<CheckpointSession>,
    pub jobs: Vec<LifecycleIntent>,
    pub next_job_id: u64,
    pub last_cap_id: u64,
    pub cleanup_receipts: Vec<(SessionKey, JournalSeq)>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct CheckpointInode {
    pub core: InodeCore,
    pub entries: Vec<(DentryKey, Dentry)>,
    pub parent: Option<(Ino, Vec<u8>)>,
    pub remote_links: Vec<(Ino, Vec<u8>)>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointSession {
    pub key: SessionKey,
    pub completed_requests: Vec<CompletedRequest>,
    pub completed_flushes: Vec<u64>,
    pub oldest_client_tid: u64,
    pub oldest_flush_tid: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RestoreError {
    #[error("retry-target upgrade required: {0}")]
    UpgradeRequired(&'static str),
    #[error("retry retention capacity exceeded")]
    Capacity,
    #[error("invalid recovery state: {0}")]
    Invalid(&'static str),
    #[error("expected sequence {expected}, received {actual}")]
    Sequence { expected: u64, actual: u64 },
    #[error("engine failed")]
    Failed,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CheckpointError {
    #[error("engine is not drained")]
    Busy,
    #[error("engine failed")]
    Failed,
}

/// One reserved maximum-record slot. This token is deliberately not Clone.
#[derive(Debug, PartialEq, Eq)]
pub struct RecordPermit {
    pub max_record_bytes: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Idle,
    NeedsAdmission,
    WaitingCommit(JournalSeq),
    WaitingInput,
    Failed,
}
#[derive(Debug)]
pub struct EngineStep {
    pub effects: Vec<Effect>,
    pub notices: Vec<EngineNotice>,
    pub state: StepState,
    pub unused_permit: Option<RecordPermit>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineNotice {
    SessionCleanupCommitted { key: SessionKey, seq: JournalSeq },
}
#[derive(Debug, Clone)]
pub enum EngineInput {
    Request {
        key: SessionKey,
        request: Request,
    },
    CapUpdate {
        key: SessionKey,
        update: CapUpdate,
    },
    CapRelease {
        key: SessionKey,
        items: Vec<CapReleaseItem>,
    },
    CloseSession {
        key: SessionKey,
        kill: bool,
    },
    Tick,
    LifecycleCompleted {
        id: JobId,
        result: LifecycleResult,
    },
    /// Release recovery grants after the daemon's ordered control cut drains.
    /// Structural gather completion alone does not release this gate.
    RecoveryControlsDrained,
}
#[derive(Debug, thiserror::Error)]
pub enum StepError {
    #[error("retry-target upgrade required; input and permit returned")]
    UpgradeRequired {
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
    },
    #[error("retry retention capacity exceeded; input and permit returned")]
    RetryCapacity {
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
    },
    #[error("invalid retry identity; input and permit returned")]
    InvalidRetryIdentity {
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
    },
    #[error("terminal retry control capacity exceeded; input and permit returned")]
    ControlCapacity {
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
        required_completions: Option<usize>,
        required_bytes: Option<usize>,
        required_transient_bytes: Option<usize>,
    },
    #[error("engine busy; input and permit returned")]
    Busy {
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
    },
    #[error("session key is not live; input and permit returned")]
    InvalidSession {
        input: Option<EngineInput>,
        permit: Option<RecordPermit>,
    },
}

#[derive(Debug, Clone)]
pub struct ReconnectCap {
    pub ino: Ino,
    pub cap_id: u64,
    pub wanted: u32,
    pub issued: u32,
    pub realm: Ino,
    pub snap_follows: u64,
    pub fcntl_locks: Vec<crate::filelock::FileLock>,
    pub flock_locks: Vec<crate::filelock::FileLock>,
}
#[derive(Debug, Clone, Copy)]
pub struct ReconnectRealm {
    pub ino: Ino,
    pub seq: u64,
    pub parent: Ino,
}
#[derive(Debug, Clone)]
pub struct ReconnectProgress {
    pub complete: bool,
    /// Authoritative realm traces that differ from the client's records.
    /// The daemon encodes the appropriate SNAP UPDATE, not a synthetic split.
    pub realm_traces: Vec<Vec<crate::SnapRealmInfo>>,
}
