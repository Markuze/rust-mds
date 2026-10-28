//! Native guarded metadata persistence. [`stable`] implements the production
//! stable WAL boundary. [`record`] encodes engine records. [`wal`] is the legacy
//! segment fixture codec; its prefix recovery is not production recovery.
//!
//! A shard is not a subvolume, and an inode number is not ownership
//! (`design/store.md` sections 6 and 7). G1–G3 keep one scalar sequence on
//! stream 0. Do not grow that sequence into multiple streams. G5 uses the
//! per-stream commit tokens in `design/transactions.md`.

pub mod allocator;
pub mod backend;
pub mod bootstrap;
pub mod framing;
pub mod record;
pub mod sessions;
pub mod stable;
pub mod upgrade;
pub mod wal;
pub use upgrade::{RetryUpgradePlan, RetryUpgradeReady};

pub use allocator::InoAllocator;
pub use backend::{Backend, PoolListing, PoolPage, RadosBackend};
pub use bootstrap::{
    BootstrapObject, BootstrapPlan, BootstrapStore, FormatInspection, FormatPhase,
};
pub use sessions::{
    AcquiredSessions, NewSessionRecord, RankSessionState, SessionLifecycle, SessionRecord,
    SessionRepository, SessionsInspection,
};
pub use stable::{
    CheckpointDescriptor, ControlInspection, FencedWriter, Limits, OpenedShard, OwnerToken,
    RankOwner, StableStore, StoreError, StoreIdentity, WriterIdentity,
};
