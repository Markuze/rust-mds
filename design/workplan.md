# Rust MDS: builder workplan

> Approved direction, 2026-10-22, base `f54d298`. This replaces the 2026-10-11 stage order. The user selected stable guarded WAL objects and immutable checkpoints. The first build cycle ends at G3. [persistence-contract.md](persistence-contract.md) fixes first-milestone interfaces; [store.md](store.md) fixes storage roles. No gate is passed by a design document alone.
>
> 2026-10-28 identity revision: a shard is not a subvolume. `SubvolumeId`, `ShardId`, `InodeId`, `OwnerEpoch` and `JournalStreamId` stay separate. G1–G3 keep the selected single-shard store. They do not wait for the concurrency proof. G4 applies the tenant boundary. G5 proves [transactions.md](transactions.md) before multi-stream and cross-shard layout changes. Inode-number leases allocate numbers. They do not own the inodes.

## Principles and evidence

- Preserve the wire-free, I/O-free engine and per-session send funnel.
- Start with one root shard, one stream, full images and ordered replay. Prefer a short execution pause over speculative visibility.
- Keep connection reads, execution, storage workers and sending separate. Bound queues and work per turn. Keep maps, beacons and completions runnable during admission pressure.
- Use plain RADOS operations through the beaconed messenger identity. No librados or object classes.
- Record gate evidence only in the canonical table in `progress_ledger.md`: commit/worktree, environment, command, result, skips and artifact path. Preserve historical evidence as historical.
- One designated QA runner owns live cluster/mount actions. Keep OSDs alive during memstore MDS-crash tests. Do not recreate the filesystem in restart/failover tests.

## Gates

### G0 — Baseline and contracts

Reconcile handover/guide/ledger. Publish the stable-WAL, engine, checkpoint, lifecycle, session and recovery contracts. Record the decision. Label historical ownership rules. Reproduce offline workspace/build/lint checks, smoke and live Objecter checks. Restore C++ reference bring-up and capture pinned peer versions and baseline workloads. Add offline CI and a serialized live lane with visible skips.

Gate: testable interface invariants, fresh baseline evidence, and a usable C++ comparison environment. Offline implementation may proceed while QA repairs the baseline. Report missing evidence; do not claim G0 complete prematurely.

Contract-review amendments: persist predecessor messenger identity for pre-acquisition fencing; platform owns bounded native pool listing; format Preparing/Ready and rank-session acquisition each use their own object's CAS. Engine production work uses one-record step/resume and durable terminal cleanup receipts. WAL inode images preserve stray path and stable subvolume identity before checkpoint. Independent re-review and fresh evidence remain required.

### G1 — Asynchronous commit and client safety

Fix F1 with two outstanding max_size extensions and partial commit tests. Audit READDIR, traversal/errors, ancestor state, replies, revocations, ranges and quota output. Add journal failure/admission and materialized checkpoint/job state. A full queue must not lose an applied transaction. Failure must never serve speculative reads.

Fix striped purge/truncate/probe mapping against C++ Striper semantics. Check negative overall and suboperation OSD results. Complete blocklist -> latest-map confirmation -> epoch barrier -> session cleanup. Route autoclose and external blocklists through this path. Add monitor `get_version("osdmap")` and explicit map-wait failure. Gate service on membership and beacon health.

Retain request validation context through deferred completion. Never send an invalid reply. Reject malformed metadata before mutation. Fix sessionless renewal and REQUEST_FLUSH_MDLOG. Expose minimal recovery phase, WAL positions, pending jobs, queue budgets and fencing reason.

Gate: delayed, failed, duplicate and partial completions release no invalid output or data job. Smoke and conformance regressions remain passing.

### G2 — Durable root shard and restart

Implement guarded Preparing/Ready bootstrap, format identity validation, allocation reservation, stable WAL batches and direct completion events. Implement checked restore/replay, immutable checkpoint creation/publication, bounded trim and backlog pressure. Preserve all state in the persistence contract; a log-history-only checkpoint does not qualify. Add deterministic fault points at each durable boundary. Keep old checkpoint objects until retirement/collection has its own proof.

Platform supplies `Objecter::list_pool_page` (native PG listing, all namespaces, bounded pages and explicit map-change restart). Initial bootstrap supports only an empty pool; the shared-object allowlist is empty. Resume uses the exact persisted Preparing plan. Test partial shard/session acquisition and uncertain format/session CAS results. No control-object comparison authorizes a different object's write.

Gate: clean unmount/restart/remount on the same filesystem retains namespace, hard links, xattrs, allocator and file data. Fault tests cover success-before-callback, unknown append result, partial checkpoint writes, publication and trim. Lost replies recover committed mutations exactly once. Missing/wrong-format/corrupt/stale-owner state fails explicitly. Never substitute a new namespace.

G2 proves storage restart, not mounted-client failover.

### G3 — Mounted-client single-rank failover

Drive replay/reconnect/rejoin/empty clientreplay/active from completions and legal monitor transitions. Verify prior-owner fencing and wait for last_failure_osd_epoch; zero alone proves nothing. Load durable sessions before reconnect. Accept reconnect v3/v5 chunks, caps, locks and realms while ordinary traffic waits. Answer committed retries once. Evict absent sessions through complete fencing, with no extra Ganesha recovery hold.

Read predecessor addresses/nonces from durable control metadata before acquisition; confirm their blocklist entries and raise the barrier. Rejoin remains mandatory. v5 chunks can mix caps and realms; only final `more=false` completes that session's gather. Validate locks before chunk installation. Preserve restored histories on identical session-key installation and finish Closing cleanup durably. Use `current/sol-recovery-review.md` as the detailed protocol acceptance checklist.

Resume durable lifecycle work after reconnect; journal recovered size/mtime before output. Support cold standby and standby-replay as a cold follower. Complete uuid/timeout and RESET reclaim. Use a Ganesha grace configuration suitable for takeover tests, not the existing graceless helper configuration.

Gate: repeated active-MDS SIGKILL and cold takeover preserve kernel/FUSE mounts and externally recorded acknowledged histories. Cover O_EXCL, rename-over-target, hard links, unlink-open, truncate/grow, xattrs, dirty caps, locks and flush acknowledgments. Test Ganesha reclaim/restart with shared-session outstanding flushes. Record reconnect and first-success latency. In-flight operations may be present or absent; retries must not repeat mutations.

### G4 — Subvolume tenancy and bounded resources

A subvolume is a tenant, not a shard. Setting `ceph.dir.subvolume` records `SubvolumeId` and a snap realm on the directory's current shard. It does not create a shard and does not copy the subtree. `EXDEV` stays at a subvolume boundary. Link and rename across shards inside one subvolume are atomic and do not change inode numbers. Use the protocol in [store.md](store.md) section 7.

Add multiple shard actors, independent budgets, indexed loading and cache accounting, boundary copies, and inode routing by home hint plus one forward. Resolve cross-boundary links before a split. Refuse clearing the subvolume flag while the subtree is non-empty. Enforce complete authenticated MDS caps, including handle lookup and strays. Complete quota pushes, bounded-lag statistics, layouts, pool validation and mgr/volumes and mgr/nfs lifecycle dependencies.

Gate: non-snapshot volumes/NFS/quota/auth suites pass with exclusions. A stalled or damaged shard does not stop unrelated serving shards. Demonstrate this inside one shared Ganesha session. Report rank recovery/reclaim coupling separately.

### G5 — Prove concurrency

This is the concurrency proof. Measure the single stream first. Introduce `MetadataTxn`: read set, write set, lock plan, mutations and dependency tokens ([transactions.md](transactions.md)). Hierarchical locks let two creates of different names in one directory overlap. Parent statistics are commutative deltas, applied once per transaction id. Capability revocation finishes before publish.

Then add per-stream commit tokens, deletion semantics and exact-stat read barriers. Replace scalar positions with per-stream receipts and checkpoint coverage vectors only after that proof. Add fixed streams, then durable routing transitions that preserve flush order. The metadata mutations themselves overlap. Overlapping journal waits around one critical section do not pass. Measure owning-task CPU limits before adding parallel preparation and state ownership.

Gate: one hot directory, concurrent `create` of distinct names, 1 to 16 workers. Replay rebuilds unique names, unique inode numbers and a matching `rstat`, with no lost delta and no double-applied delta. Independent same-directory operations and strict-I/O flushes overlap while true dependencies wait. Stall one stream. Independent work still completes. Randomized replay and checkpoint schedules match an independent oracle. Record operations per second, scaling efficiency, core use, commit latency and p99. Async overlap is not proof of multi-core execution.

### M0b — msgr2 parallel track

Start after G1 platform interfaces settle. Add client/server CRC then secure mode. Publish v2 then v1 with matching identity/nonce. Preserve one messenger identity.

Gate: kernel legacy/prefer-crc/secure, FUSE/libcephfs/Ganesha v2, reconnect/signatures and renewal/rotating-secret observation across actual renewal thresholds. Qualify deployed key types separately.

### G6 — Multiple ranks and migration

Implement placement/pins, registered-stream fencing, resumable handoff, FLUSHMSG, forced sessions, cap EXPORT/IMPORT, increasing forwarding, any-rank LOOKUPINO and max_mds grow/shrink.

Gate: crash at every handoff state. No dual authority, stale checkpoint publication, destructive old-owner I/O or forward after reply. Qualify multi-rank failure, cephadm/Rook shrink and upgrades. Stopping transfers jobs and drains obligations before STOPPED.

### G7 — Release qualification

Complete admin/tell/config/health/counters/MgrClient/packaging, bringing dependencies forward when earlier gates need them. Run selected fstests, fsstress/fsx, LTP, pynfs, Ceph QA, Rook/CSI and cephadm against pinned versions. Compare C++ on identical topology/durability; report safe-only create latency separately. Qualify OSD/host failure on disk-backed BlueStore, not memstore. Set numeric recovery, memory, queue and checkpoint limits from measured baselines. Resolve license and incompat-bit allocation before distribution.

## Coordinated run ownership and order

The parent orchestrates. Sol implementers own assigned files; separate Sol reviewers inspect contracts and changes. Do not infer ownership from the historical build brief. Current task assignments override the earlier review plan's proposed owner table. No commits, branch changes or live actions without explicit assignment.

1. Design owner publishes G0 contract. Engine starts F1 against existing APIs while this settles.
2. Engine implements visibility/failure/checkpoint/job types. Platform supplies latest-map support and checked compound results. Store implements schema and mocked guarded operations against the contract.
3. Daemon/store owners integrate real WAL, sessions and checkpoint/restart. QA supplies independent crash histories and baseline evidence.
4. Daemon integrates reconnect/jobs. Review G3 evidence before later gates.

Each owner maintains `current/<task>.md`, uses a distinct target directory, and reports exact commands/skips. Only the assigned owner edits each file. Cross-crate signature changes require agreement before consumers change. Do not require independent G4-G7 mechanisms to ship the conservative G1-G3 version.

## Verification matrix

Required deterministic cases: partial range commit; speculative READDIR/path output; capacity before mutation; failure after mutation; unknown append success; committed gap/corruption; bounded/versioned decode; checkpoint partial write/publish/trim; stale append/publish/trim/event; lifecycle crash/late result; failed blocklist/map wait; interrupted OPEN/CLOSE. Require real RADOS compound-guard evidence and unmodified-client acknowledged histories in addition to unit tests. A final namespace listing alone is not a durability oracle.

Integration commands: `cargo test --workspace`, `cargo build -p rmds`, formatting checks and `cargo clippy --workspace --all-targets`. Record existing failures separately. QA alone runs live gates. After P0, preserve the decision-log P1 order: early replies then async dirops, dentry leases, snapshots after scope decision, warm standby-replay, scrub, and metrics.
