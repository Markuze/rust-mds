# First durable milestone: interface contract

Status: G0 contract, 2026-10-22, based on source at `f54d298`. These are target interfaces, not claims about implemented APIs or passing gates. This document controls G1-G3 integration. `store.md` defines object roles. `workplan.md` defines gates.

G3 extensions are in [recovery-contract.md](recovery-contract.md). Independent review **ACCEPTS E2, D6 and matching wire REPLAY**; their assigned consumers may proceed. The ordinary E1 contract is independently accepted: sections 3.2–3.4 and engine/configuration joins in [current/astra-recovery-contract-review.md](../current/astra-recovery-contract-review.md), and sections 3.5–3.8 in [current/astra-retry-upgrade-review.md](../current/astra-retry-upgrade-review.md). The user selected the clean-session deployment boundary on 2026-10-25. Its original-target field, durable retention and required-semantics-3 transition extend this old-version contract only as specified there. No code or live upgrade is accepted by this decision.

## 1. Scope and owners

Use rank 0, shard 0, stream 0, full inode images, and ordered replay. `JournalSeq = u64` remains valid. Do not implement independent streams, shard adoption, indexed loading, migration, or aggregate deltas to satisfy this contract. The separately proposed E1 **format** adoption is not G4 shard adoption. It preserves the filesystem and pool identities.

- `engine` owns logical state, transaction construction, visibility, checkpoint state, and job state. It imports no `denc`, `proto`, `osdc`, RADOS, or async runtime types.
- `store` owns persistent encoding, storage guards, batches, checkpoint objects, and durable session records.
- `mds` owns membership authorization, rank sessions, bounded queues, storage workers, recovery phases, and wire output. It translates storage events to engine inputs.
- The stream-0 control object alone grants shard write ownership. The shard map is a routing projection. The rank session object alone owns session identity/lifecycle. Shard retry tables alone own completed operations.

Rust signatures below specify names and semantics for cross-crate integration. Supporting error details and private helpers can differ. New types live in their stated owning crate.

## 2. Journal admission, completion, and visibility (`engine`)

Keep `submit`, `committed_seq`, and `flush`. Change submission to return an explicit error:

```rust
enum JournalError { Unavailable, Capacity, RecordTooLarge, Fenced, Corrupt }
trait Journal {
    fn submit(&mut self, record: JournalRecord) -> Result<JournalSeq, JournalError>;
    fn committed_seq(&self) -> JournalSeq;
    fn flush(&mut self);
}
enum JournalEvent {
    Committed(JournalSeq),
    Failed(JournalError),
}
// Engine methods:
fn journal_event(&mut self, event: JournalEvent) -> EngineStep;
fn is_failed(&self) -> bool;
fn is_quiescent(&self) -> bool;
```

The daemon journal adapter owns an outbox. `submit` synchronously transfers a complete record into that outbox. It performs no I/O and never blocks. Sequence allocation starts at recovered tail + 1. `committed_seq` is a contiguous, monotonic durable prefix, never a submitted prefix. `flush` wakes the worker immediately; it does not imply completion. `MemJournal` implements these rules without I/O.

For this first implementation, pause engine execution globally after each state-changing transaction until its commit effects have been materialized. Do not run the next transaction from inside a completion loop before materializing the preceding transaction's output. Queue later reads as well as mutations. This conservative policy covers cap updates, ancestor statistics, path errors, READDIR children, session cleanup, ticks, revocations, and lifecycle callbacks. Pure control-plane handling stays runnable outside the engine. Acknowledgments required to obtain a hold can run before mutation; once a WAL record is pending, buffer engine inputs until completion.

Admission capacity is checked before calling a mutation-capable engine entry point. The adapter reserves one maximum-size record slot for the active transaction. Internal loops must stop at a transaction boundary and must not construct a second transaction without a new slot. Bound record bytes, queued input bytes/count, outbox bytes/count, and retained WAL bytes/keys. A full budget parks input outside the engine without applying it. Reserve control-event queue capacity independently. The engine and adapter must use the same configured maximum record size. Oversized or unencodable records must never receive success.

The enforceable production entry point is one-record stepping, not wrapping today's multi-record calls in one reservation:

```rust
// engine types; no channel or store type enters this boundary.
struct RecordPermit { max_record_bytes: usize } // one use, not Clone
enum StepState { Idle, NeedsAdmission, WaitingCommit(JournalSeq), WaitingInput, Failed }
struct EngineStep { effects: Vec<Effect>, notices: Vec<EngineNotice>, state: StepState,
                    unused_permit: Option<RecordPermit> }
enum EngineNotice { SessionCleanupCommitted { key: SessionKey, seq: JournalSeq } }
enum EngineInput {
    Request { key: SessionKey, request: Request },
    CapUpdate { key: SessionKey, update: CapUpdate },
    CapRelease { key: SessionKey, items: Vec<CapReleaseItem> },
    CloseSession { key: SessionKey, kill: bool },
    Tick,
    LifecycleCompleted { id: JobId, result: LifecycleResult },
}
fn step(&mut self, input: Option<EngineInput>, permit: Option<RecordPermit>)
    -> Result<EngineStep, StepError>;
```

Use the existing wire-free `CapReleaseItem { ino, cap_id, mseq, issue_seq }`. `StepError::Busy { input: Option<EngineInput>, permit: Option<RecordPermit> }` returns rejected input and the unused permit unchanged. The daemon admits at most one input at a time, apart from control acknowledgments needed to satisfy pre-mutation holds. Such acknowledgments use the same typed cap inputs and cannot start a second logical operation. `step(None, permit)` resumes an internal continuation. Without a permit, the engine can validate/park input or emit already-safe control effects, but cannot apply a persistent mutation. A permit authorizes at most one `submit`, including indirect range removal, purge, or completion transactions. Consume it at the transaction boundary. Retain remaining release items, cap-close cursor and parked-work cursor internally; stop before constructing the next transaction. Budget pure work per step too. `NeedsAdmission` requests another reserved slot; `WaitingCommit` requires that prefix's completion; `WaitingInput` requires a cap/lock acknowledgment rather than a WAL slot. Do not spin resume on either wait state.

`journal_event` releases/materializes committed effects only; it never submits a record or runs parked work. It returns `NeedsAdmission` when commit-triggered purge or other continuation remains. The daemon processes each returned effect vector before the next step, even with an auto-committing test journal. Existing direct entry points may remain test helpers, but production close, releases, tick, lifecycle completion and parked work must use the step path. There is never an unbounded per-close reservation.

Session cleanup ends with a dedicated record containing `closed_sessions: [key]`, after every preceding range-removal/job-intent transaction commits. Emit `SessionCleanupCommitted` only when this terminal record commits. A recovered terminal record provides the same durable proof through `session_cleanup_seq(key) -> Option<JournalSeq>`; preserve these receipts in checkpoint state. Absence of runtime caps or an empty continuation is not cleanup proof. The rank repository may mark Closed only after this receipt. No later step for that key may reintroduce histories or rights.

If `submit` nevertheless fails after an in-memory change, mark the engine failed before returning any effects. Discard all deferred client success effects and data jobs. Do not attempt to continue or retry the client operation in that engine. This includes a capacity contract violation. The daemon stops all reads and writes and reloads durable state through recovery. It may send FORCE_RO, but must not serve speculative reads. This fail-closed fallback avoids requiring rollback. Rejected input before mutation is not a journal failure.

The adapter updates its durable prefix before delivering `Committed`. Reject a prefix above submitted. Duplicate prefixes are harmless. After `Failed`, no completion can release effects. The daemon tags every worker event with its current ownership token and ignores events from another generation. Ownership loss immediately stops output and data-job scheduling, not only new writes. On ordinary storage failure, FORCE_RO is optional; exit/replay is the initial policy. `EngineConfig::read_only` alone is insufficient.

Global serialization is the initial visibility proof, not a permanent performance design. F1 regression tests with manually delayed journals must still cover two pending cap extensions. If the engine retains concurrent pending transactions for those tests or later use, it must serialize conflicts or retain immutable committed output images. A callback must never read a future image. Every returned effect must be safe when returned, not merely when its record was submitted.

## 3. Checkpoint and replay state (`engine`)

```rust
struct EngineCheckpoint {
    covered_seq: JournalSeq,
    inodes: Vec<CheckpointInode>,
    ino_alloc: InoAllocState,
    sessions: Vec<CheckpointSession>,
    jobs: Vec<LifecycleIntent>,
    next_job_id: u64,
    last_cap_id: u64,
}
struct CheckpointInode {
    core: InodeCore,
    entries: Vec<(DentryKey, Dentry)>,
    parent: Option<(Ino, Vec<u8>)>,
    remote_links: Vec<(Ino, Vec<u8>)>,
}
struct SessionKey { client: ClientId, incarnation: u64 }
struct CheckpointSession {
    key: SessionKey,
    completed_requests: Vec<CompletedRequest>,
    completed_flushes: Vec<u64>,
    oldest_client_tid: u64,
    oldest_flush_tid: u64,
}
// Engine methods:
fn checkpoint(&self) -> Result<EngineCheckpoint, CheckpointError>;
fn restore(config: EngineConfig, journal: J, state: EngineCheckpoint)
    -> Result<Engine<J>, RestoreError>;
fn replay_record(&mut self, seq: JournalSeq, record: &JournalRecord)
    -> Result<(), RestoreError>;
```

`checkpoint` requires paused admission, no staged transaction, no pending commits, and submitted = durable = covered_seq. Parked requests and runtime holds are not checkpoint state. Drain them or stop them at a pre-mutation boundary. The exported state is a materialized namespace, not a vector of historical WAL records. It must restore with all history <= covered_seq deleted. Encode parent/link state explicitly, including strays without a dentry. Preserve every `InodeCore` field, snap realms, stable subvolume identity, writable ranges, allocator leases/delegations, and prior authorization path. Do not infer lost-writer bounds only from runtime caps.

Checkpoint session history is scoped by durable `SessionKey`. Add incarnation to request/flush/closed-session WAL bookkeeping; `ClientId` alone is insufficient after CLOSE/OPEN. The daemon supplies this incarnation on durable session open. Preserve durable trimming watermarks, including a watermark with no surviving entry. Runtime session epoch counters are not a source of durable identity.

Accepted E1 contract override: recovery-contract sections 3.3–3.5 add `reply_target_ino` and explicit semantic versions. The canonical retry table retains target dependencies until a committed completion-watermark or terminal cleanup retires them. Checkpoint export/publication is not a new retirement event. Do not infer an existing CREATE/OPEN mutation's target from `created_ino=None`, unordered images or a post-replay path. These E1 changes have independent contract acceptance; that acceptance does not itself establish implementation acceptance, native proof, or live-upgrade authorization.

Concretely, add `session_incarnation: u64` to `CompletedRequest` and `ClientFlush`, and change `JournalRecord::closed_sessions` to `Vec<SessionKey>`. Production close uses `EngineInput::CloseSession`, not the old all-at-once close helper. Install/restoration uses `install_session_key(key: SessionKey) -> Result<(), RestoreError>`: identical key preserves replayed histories and watermarks; an absent key creates an empty session; a different live incarnation is an error. It submits no WAL record and resets no durable identity. Existing client-id helpers may remain for tests. Store encoding carries new fields in versioned nested frames, not by inserting fields into the middle of an old unframed vector element. Add `cleanup_receipts: Vec<(SessionKey, JournalSeq)>` to `EngineCheckpoint` for the terminal cleanup proofs.

Add persistent `InodeCore::stray_auth_path: Option<Vec<u8>>` and `InodeCore::stable_subvolume_id: Ino` (0 outside a subvolume). `CheckpointInode` reads these from `core`; remove its duplicate `stray_auth_path` field. Each full WAL inode image includes both fields in a new supported schema version. Set stable identity at inode creation from its owning subvolume context; root-shard ordinary inodes use 0. Once nonzero, never change it during rename, unlink, replay or recovery. Keep reply identity independent of walking live parents. G4 adoption must separately specify the transition from 0 before it is enabled.

Before last-link removal, compute and journal the former absolute authorization path in the same transaction as the dentry deletion. Use the pre-operation primary link (walk primary parents to root) even if the last removed link is remote; if primary promotion is necessary, retain the old primary path before promotion/deletion. Last unlink and rename-over-target both follow this rule. For a preexisting stray, preserve its already recorded path. A retained unlinked inode without this path is corruption. Replay applies the explicit inode image rather than guessing from already removed dentries. A purged inode may disappear only when its durable job already retains all required inputs; stable identity and path remain on any retained stray inode. Required test: checkpoint at L, last unlink or rename-over-target at L+1, crash before next checkpoint, restore and replay; assert former path and stable subvolume identity as well as linkage.

Restore creates no root and submits no records. It validates duplicate inodes, linkage, ranges, session keys, and job references before service. Replay requires exactly the next sequence. Missing references are errors, not silently skipped updates. Replay emits no client output or data work. Runtime caps, holds, file locks, connections, and parked requests are empty; reconnect reconstructs them. Seed runtime cap identifiers safely from checkpoint/replay state. A round trip must compare all recovery state, not only `namespace_eq`.

Keep the adapter closed to submission during load. After replay reaches the opened durable tail, initialize its submitted and committed positions to that tail; its next submission is tail + 1. The engine's replay cursor starts at checkpoint.covered_seq. A Ready filesystem always has an initial materialized checkpoint; an absent descriptor in Preparing is not loadable service state.

The existing `Engine::replay` may remain a test helper. The production loader uses checked restore/replay. Constructors that create root state remain exclusive to bootstrap.

## 4. Durable lifecycle (`engine`, executed by `mds`)

```rust
type JobId = u64; // unique and monotonic within the shard
struct LifecycleIntent { id: JobId, ino: Ino, kind: LifecycleKind }
enum LifecycleKind {
    Purge { layouts: Vec<FileLayout>, max_size: u64 },
    Truncate { layout: FileLayout, from: u64, size: u64, truncate_seq: u32 },
    RecoverSize { layouts: Vec<FileLayout>, max_size: u64, inode_version: u64,
                  truncate_seq: u32 },
}
enum LifecycleResult { Done, Recovered { size: u64, mtime: Timestamp } }
// Production completion enters step via EngineInput::LifecycleCompleted.
```

Extend `JournalRecord` with `lifecycle_intents: Vec<LifecycleIntent>` and `completed_jobs: Vec<JobId>`. Add the matching record kind. Version all new persistent nested types. Each layout includes pool, namespace, stripe unit/count and object size; retain all old pool/layout inputs needed to remove old objects. Preserve the empty snapshot-context extension point in the store encoding. The worker envelope carries the current owner token; job identity itself survives takeover.

Commit intent before emitting work. Retain full intent and lost-writer range until completion commits. Purge may remove the namespace inode in its intent transaction only if the retained intent is independently sufficient for retry. Keep stray authorization context until it is no longer needed. On recovery, expose pending jobs only after fencing and reconnect complete and after confirming no caps prevent purge. Execute at least once. Missing data objects are success for purge, not for arbitrary probe failures. Transient worker errors retain the intent; no completion is journaled.

Completion journals job retirement and resulting metadata atomically. A recovery result applies only to its recorded inode version/truncate sequence; otherwise retain or supersede the job through a journaled decision. Serialize conflicting inode mutations while a size-recovery job is active in the first version. Duplicate completion is harmless. Stale owner events do nothing. Do not publish recovered attributes or retire an intent on a failed completion commit. Truncate ordering uses `truncate_seq`; drain destructive I/O before starting conflicting recovery work. WAL ownership guards do not fence data objects: takeover requires old-owner address fencing first.

## 5. Stable WAL and bootstrap (`store`)

```rust
struct OwnerToken { shard_epoch: u64, rank: u64, incarnation: u64, gid: u64 }
struct RankOwner { rank: u64, incarnation: u64, gid: u64 }
// store types can use denc::EntityAddrVec; engine never imports this type.
struct WriterIdentity { rank_owner: RankOwner, addresses: EntityAddrVec }
struct FencedWriter { writer: WriterIdentity, verified_osd_epoch: u32 }
struct ControlInspection {
    revision: u64, owner: OwnerToken, writer: WriterIdentity,
    predecessor: Option<FencedWriter>, tail: JournalSeq,
    checkpoint: Option<CheckpointDescriptor>,
}
struct StoreIdentity { cluster_fsid: [u8; 16], filesystem_id: u64, metadata_pool: i64 }
struct OpenedShard { owner: OwnerToken, tail: JournalSeq,
                     checkpoint: Option<CheckpointDescriptor> }
enum StoreError { Io, UnknownOutcome, Fenced, Corrupt, WrongFormat,
                  UnsupportedVersion, MissingState, Capacity }
// async store operations (no engine async trait required):
inspect_control(identity: &StoreIdentity) -> Result<ControlInspection, StoreError>
open(identity: &StoreIdentity, expected: &ControlInspection,
     new_owner: OwnerToken, writer: WriterIdentity, predecessor: FencedWriter)
     -> Result<OpenedShard, StoreError>
append(owner, expected_tail, records) -> Result<JournalSeq, StoreError>
load(opened) -> Result<(EngineCheckpoint, Vec<(JournalSeq, JournalRecord)>), StoreError>
```

`inspect_control` is read-only and precedes guard acquisition and namespace replay. Read format and rank-session guard metadata read-only too. Each current writer stores the exact beaconed messenger address vector, including type/IP/port/nonce, before it can do any work. A replacement must not depend on a removed MdsInfo or client-session addresses to identify its predecessor. The daemon validates current MDSMap assignment, obtains or verifies a monitor blocklist for every predecessor writer found in these authorities, queries latest osdmap, waits successfully for at least that epoch and last_failure_osd_epoch, checks `OsdMap::is_blocklisted` for every usable predecessor address/nonce, and raises the Objecter barrier. Fail closed on missing identity/address or failed evidence. A zero failure epoch does not bypass this chain, including after standby-replay removal. Only a verified fresh bootstrap has no predecessor.

`open` compares old owner and control revision on `rmds.wal.0.0` and writes new owner, writer identity and predecessor evidence in that same operation. It preserves tail/checkpoint and increments shard epoch. Store checks that supplied predecessor matches `expected.writer`; daemon owns the proof that its addresses are fenced. Persist the evidence before acknowledging acquisition, including when no client work follows. Unknown acquisition outcome is reconciled by rereading control: exact requested token/writer/predecessor proves acquisition; unchanged expected revision permits identical CAS; any other owner requires reinspection and a new fencing proof. Never reset state. Retain the predecessor evidence across interrupted acquisition; verify its epoch on subsequent load. No shard/session mutations or replay-dependent data work occur until both authorities are acquired (section 7).

### Bootstrap: bounded native scan and per-object guards

Platform (`monc/osdc` owner) must implement native PG object listing; current Objecter object operations cannot enumerate a pool. Target API in `osdc`:

```rust
enum NamespaceScope { All }
struct PoolListCursor { osdmap_epoch: u32, pg: u32, handle: Vec<u8> }
struct PoolObject { namespace: Vec<u8>, name: Vec<u8>, locator: Vec<u8> }
struct PoolListPage { objects: Vec<PoolObject>, next: Option<PoolListCursor> }
async fn list_pool_page(&self, pool: i64, scope: NamespaceScope,
    cursor: Option<PoolListCursor>, max_entries: u32, max_bytes: usize)
    -> Result<PoolListPage, PoolListError>;
// PoolListError includes MapChanged (restart scan), Unsupported, Io and decode/size errors.
```

Use Ceph native PG listing with namespace-aware results and opaque continuation. A cursor pins a map epoch/PG layout. Return MapChanged rather than skipping entries on PG split/remap; store restarts the scan at None with bounded retry/deadline and no bootstrap writes. Validate page count/bytes and bound each turn. Unsupported listing is a bootstrap error, not permission to probe a few known names. Store consumes pages incrementally, not by accumulating the pool. Initial shared-object allowlist is explicitly empty: G2 bootstrap supports a fresh empty metadata pool only. Reject every foreign object in every namespace. Resume allows only the exact deterministic object names in the persisted bootstrap plan, with matching identities/contents; an `rmds.` prefix alone is not accepted. Later shared-service exceptions require exact documented names/prefixes and tests. Require exclusive assigned creating rank and no concurrent foreign filesystem initialization; pool listing is not a cross-object lock.

```rust
struct BootstrapPlan {
    bootstrap_id: [u8; 16], identity: StoreIdentity,
    root_state: EngineCheckpoint, initial_range: InoRange,
    initial_checkpoint: CheckpointDescriptor,
    // Exact deterministic object names/checksums, including allocator initial state.
    objects: Vec<BootstrapObject>,
}
struct FormatInspection {
    revision: u64, writer: WriterIdentity, predecessor: Option<FencedWriter>,
    phase: FormatPhase, plan: BootstrapPlan,
}
enum FormatPhase { Preparing, Ready }
inspect_format(identity: &StoreIdentity) -> Result<Option<FormatInspection>, StoreError>
begin_bootstrap(plan: BootstrapPlan, writer: WriterIdentity)
    -> Result<FormatInspection, StoreError>
acquire_preparing(expected: &FormatInspection, writer: WriterIdentity,
    predecessor: FencedWriter) -> Result<FormatInspection, StoreError>
resume_bootstrap(format: &FormatInspection, owner: OwnerToken)
    -> Result<OpenedShard, StoreError>
publish_ready(expected: &FormatInspection) -> Result<FormatInspection, StoreError>
```

`begin_bootstrap` exclusive-creates `rmds.format` as Preparing after the empty-pool scan. Persist the entire bounded deterministic plan, root seed, initial range and manifest identity before creating dependencies. A replacement reads this plan; it never invents a new bootstrap id/root/range or rewrites immutable names under its own epoch. `acquire_preparing` uses the format object's own writer/revision CAS after fencing its writer; preserve plan and record predecessor. Unknown results reconcile exact requested writer/revision/content before proceeding. Resume only the same supported identity/plan in monitor-confirmed creating. Starting/replay never completes Preparing by silently creating a new filesystem.

`resume_bootstrap` verifies or exclusive-creates each planned dependency. Allocator initial state reserves `initial_range` and advances the global high watermark before root state is publishable. A conflicting existing allocator or object is an error, not an overwrite. Acquire existing shard/session objects with their own guarded CAS protocols and preserve all positions/counters; no object guard grants write access to another object. Verify all planned objects and guards, then `publish_ready` compares Preparing, expected revision and current writer on `rmds.format` itself and sets Ready. Its guard is not a WAL guard. Assignment and prior-writer fencing prevent a deposed preparer from completing this operation; a CAS failure aborts/reinspects. A crash after dependencies but before Ready resumes verification. An uncertain Ready result is reread on the format object. Ready plus missing dependencies is damage. Concurrent initialization loses exclusive-create/CAS and must stop; no imagined cross-object transaction exists.

For the first version, control xattrs contain owner, tail, and checkpoint generation/descriptor. Encode numeric compared values canonically. Use one bounded compound write: compare owner, compare expected tail, set omap records at `r/<16 lower-case hex seq>`, update tail and last-batch identity. Protect framing length, format identity, seq, owner generation, transaction identity and body with checksum. A record occupies one omap value. Do not split a transaction across independently committed objects. The expected tail is the batch's first seq minus one. One batch is in flight per stream. Check transport result, overall OSD result, and all suboperation results.

Unknown completion freezes append admission. Retry the identical logical batch, identity and bytes. Read owner/tail/last-batch identity atomically: matching committed batch proves success; unchanged tail under the same owner permits identical resubmission; changed owner means fenced. Any unexplained tail/identity is corruption. Do not allocate replacement sequences. Suppress duplicate completions. Do not checkpoint/trim while an append outcome is unknown. Objecter resends must preserve the same request identity and payload.

Durable keys in `(checkpoint.covered_seq, tail]` must all exist, decode within byte/count bounds, and pass identity/sequence/checksum checks. Any gap or corruption is `Corrupt`, never ordinary end-of-log. Old segment framing remains a fixture codec, not the production recovery boundary.

## 6. Immutable checkpoints and trim (`store`)

```rust
struct CheckpointDescriptor { generation: u64, covered_seq: JournalSeq,
                              manifest_name: String, manifest_checksum: u32 }
write_checkpoint(owner, state) -> Result<CheckpointDescriptor, StoreError>
publish_checkpoint(owner, expected_generation, descriptor) -> Result<(), StoreError>
trim(owner, descriptor, bounded_keys) -> Result<(), StoreError>
```

Write immutable objects named `rmds.checkpoint.<shard>.<epoch>.<generation>.*`. Exclusive create, or verify identical existing bytes. The manifest includes identity, owner generation, covered seq, schema versions, object names, lengths and checksums. Finish all writes before publishing the manifest pointer in the stable control object. Publication compares owner, expected checkpoint generation, and a tail >= covered seq. Never move coverage backward or replace a published generation's contents. Failure before publication leaves only orphans.

Trim compares current owner and exact published descriptor in the same compound operation as bounded omap removal. Remove only seq <= covered seq. Crash after publication but before trim is safe. Loader restores that complete materialized checkpoint and replays only uncovered records through the durable tail. Missing checkpoint objects are damage even if old WAL remains. No mutable inode/dir checkpoint writes exist in G1-G3.

Do not implement checkpoint-object garbage collection in the first milestone. Retain old generations and enforce a configured backlog bound through admission pressure. Before enabling collection, add a durable irrevocable retirement set under the control guard. A collector may delete only that exact retired generation. A stale listing or pointer is not deletion authority. This conservative deferral does not defer WAL trim.

## 7. Rank session durability (`store` data, `mds` policy)

`rmds.sessions.0` is a stable omap object with rank incarnation guard and CAS revision. Persist a monotonically allocated session incarnation and records keyed by `SessionKey`. Persist client global id, authenticated entity name and authorization context, peer address/nonce for fencing, client/server feature bitmaps, metadata (including root, uuid, timeout), and lifecycle `Open`/`Closing`/`Closed`. Connections, clocks and `push_seq` are runtime state. Start a fresh transport/funnel generation on recovery; never accept a pre-recovery CLOSE seq as proof of current push delivery.

Expose async `load_sessions`, `open_session_record`, `mark_session_closing`, and `mark_session_closed`, all with rank-owner guard and expected revision. Duplicate OPEN for the same live key attaches a connection; it does not allocate a new incarnation. Persist changed identity/address metadata before admitting work that depends on it. Validate current cephx credentials on reconnect; persisted authorization is not permission to bypass authentication.

The rank repository acquisition boundary is explicit:

```rust
struct SessionsInspection {
    owner: RankOwner, writer: WriterIdentity, predecessor: Option<FencedWriter>,
    revision: u64, next_incarnation: u64,
}
struct AcquiredSessions { control: SessionsInspection }
inspect_sessions(rank: u64) -> Result<SessionsInspection, StoreError>
acquire_sessions(expected: &SessionsInspection, new_owner: RankOwner,
    writer: WriterIdentity, predecessor: FencedWriter)
    -> Result<AcquiredSessions, StoreError>
load_sessions(acquired: &AcquiredSessions) -> Result<RankSessionState, StoreError>
```

`RankSessionState` contains the acquired control (including next_incarnation) and all Open/Closing/Closed records. Acquire compares the expected owner and revision on `rmds.sessions.<rank>` itself; increment revision, replace owner/writer and retain predecessor evidence, all session records and next_incarnation. Allocate/increment next_incarnation atomically with a new rank Open record under this guard. Never derive it from the set of live sessions. Unknown acquire results are reconciled by exact new owner/writer/predecessor plus preserved counter, or retried identically only if the expected revision remains. Different state requires reinspection and fencing, never overwriting it.

All mutations below compare `acquired.control.owner` and the supplied current revision on the rank-session object and return its next revision. The row types carry the identity/features/metadata fields listed above:

```rust
open_session_record(acquired: &AcquiredSessions, expected_revision: u64,
    record: NewSessionRecord) -> Result<(SessionKey, u64), StoreError>
mark_session_closing(acquired: &AcquiredSessions, expected_revision: u64,
    key: SessionKey) -> Result<u64, StoreError>
mark_session_closed(acquired: &AcquiredSessions, expected_revision: u64,
    key: SessionKey, cleanup_seq: JournalSeq) -> Result<u64, StoreError>
```

Reconcile unknown lifecycle outcomes by inspecting the exact key/state/revision; never allocate another incarnation for an uncertain Open. Preserve the pending logical operation until outcome is known. Return unused step permits immediately to the adapter budget; a returned permit never authorizes a second submission in the step that returned it.

Recovery order: read-only inspect format, shard and sessions; fence the union of their current writers (and honor retained predecessor evidence); acquire Preparing format if applicable; acquire shard; acquire sessions; then load both and join. Until both shard and session guards belong to the new assigned rank incarnation, perform no shard/session mutations or service. Crash after either acquisition is safe: a replacement reads both actual writers, fences both as necessary, CAS-acquires each independently, and preserves state/counters. A shard acquired by B and sessions still held by A is a resumable partial state, not a reason to initialize sessions or reset incarnation allocation. If a preexisting reverse partial state is observed, use the same inspection/fencing rule. Cross-object acquisition is not atomic.

Join rank lifecycle with replayed shard histories before advertising reconnect:

- Open: `install_session_key` binds the identical key without clearing histories/watermarks or allocating an epoch. An empty history is valid. A cleanup receipt for an Open key is corruption.
- Closing: bind the identical key only for cleanup, restore its histories until the terminal cleanup record commits, and resume through `step`. If its durable cleanup receipt already exists, mark rank Closed without repeating cleanup. Do not include it in the Open reconnect gather or allocate a replacement incarnation yet.
- Closed: require its persisted cleanup receipt/sequence to agree with shard recovery, then retain the tombstone and never reinstall the session. If this requirement is violated, report corruption, not a new Open. Do not discard another incarnation's histories.
- Shard histories with no corresponding rank record, duplicate live keys for a client, or a key mismatch are corruption. No new incarnation is substituted. Closed tombstones and cleanup receipts are retained in G1-G3 so this check does not depend on lost history.

OPEN: commit rank Open record, install the same SessionKey in the engine, then send OPEN and admit requests. An empty shard history is valid. Reconnect uses the existing key. A rank Open record whose reply was lost is still an open session.

CLOSE: stop session admission, drain its admitted transactions, validate current push_seq (or complete eviction fencing), persist Closing, then submit `CloseSession` to the engine step protocol. Cleanup retains durable lost-writer jobs. `mark_session_closed` takes `(key: SessionKey, cleanup_seq: JournalSeq)` and persists that proof after `SessionCleanupCommitted` or its recovered receipt. Only then send CLOSE when required. Recovery of Closing resumes cleanup; Closed cannot resurrect retired histories. Do not delete history before Closing is durable. Do not reopen a Closing key. Retain Closed tombstones in G1-G3. A later OPEN allocates a new incarnation. Cross-object atomicity is not assumed. Rank records do not duplicate shard retry tables; shard records cannot resurrect a rank-closed session. Preserve histories for every Open key even if it has no connection. Reclaim/uuid changes follow the same ordered close/open protocol.

## 8. Daemon integration and acceptance

Internal phases: `WaitingForFence -> OpeningStore -> Loading -> Reconnect -> Rejoin -> ClientReplay -> Serving`; `Failed` stops service. Bootstrap has its own `Creating` path. ClientReplay exists but is empty. Monitor state transitions remain legal; internal phase completion does not fabricate a monitor acknowledgment.

Read-only control inspection is permitted in WaitingForFence to discover the predecessor. Do not load/replay namespace state or start data work before the fencing proof. Distinguish requested and monitor-confirmed phases. Single-rank recovery still requires monitor-visible `reconnect -> rejoin -> active` (or rejoin -> clientreplay -> active when needed); never skip rejoin. The internal clientreplay hook is empty only when no recovery replies/writes remain. Drain committed retries and their required valid trace reconstruction before ordinary mutations can race their parent/target; safe-only replies do not remove the lost-safe-reply case.

Build the reconnect gather from joined Open keys. Accept reconnect data while ordinary retry/flush traffic waits. v3 is complete in one message; v5 may mix caps and realms in every chunk, including an empty final chunk. `more=true` never completes the gather or sends OPEN. Validate all lock blobs before installing any part of a chunk. Bound gathered data. On connection reset before the final chunk, discard that connection's staged reconstruction and collect anew; repeated chunks replace per-inode cap/lock state rather than append duplicate locks. Apply authenticated client identity to locks. Finish cap ids/sequences, fcntl/flock locks, realm reconciliation and required features before final OPEN. Missing sessions finish only through verified fencing and durable cleanup, including timeout sessions; no extra Ganesha recovery hold is added. Keep existing codecs; no chunk ids or new wire messages are introduced. The independent acceptance checklist in `current/sol-recovery-review.md` remains required for G3.

Typed events carry owner token and operation id: store opened/loaded, journal committed/failed, session durable, checkpoint published, lifecycle completed, map applied, fencing verified, reconnect complete. Drive progress from events, not a one-second polling tick. Reject stale events. Maps, beacons, Objecter events, journal completions and reconnect chunks run while ordinary traffic waits. Bound held traffic. Beacon lag, rank removal, self blocklist, and owner mismatch close admission immediately. `CheckLatestMap` uses platform `get_version("osdmap")`; wait success is explicit. A zero failure epoch alone is not proof that prior destructive workers are fenced.

Retain request validation context until safe output; failed invariant validation must not send the reply. `REQUEST_FLUSH_MDLOG` captures submitted tail, calls flush, and waits for that durable prefix. Include monotonic OSD epoch barriers on client cap traffic. Eviction requires successful blocklist (or verified existing blocklist), latest-map wait, barrier raise, then cleanup. Failed blocklist never removes rights.

Required tests: partial commit output; submit failure after mutation; full admission without mutation; unknown append success/retry; stale owner append/publish/trim/completion; checkpoint-only restore with old WAL deleted; missing committed record; interrupted OPEN/CLOSE; lifecycle crash/retry and late recovery; mounted-client acknowledged history after SIGKILL. QA owns fresh baseline/live evidence. G0 is not passed by this document alone.
