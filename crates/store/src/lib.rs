//! Native guarded metadata persistence. [`stable`] implements the production
//! stable WAL boundary. [`record`] encodes engine records. [`wal`] is the legacy
//! segment fixture codec; its prefix recovery is not production recovery.

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
