# Store integration boundary

This crate implements the G2 object protocol. It has offline fault tests. It has
not passed a live RADOS gate. The daemon must enforce assignment, fencing,
admission and recovery ordering from `design/persistence-contract.md`.
The isolated E1 format/upgrade candidate follows the accepted primary
`design/recovery-contract.md` sections 3.3–3.8. It awaits fresh independent review
and primary promotion. It supplies no live-upgrade authorization.

## Native backend

`RadosBackend::new(objecter, metadata_pool)` uses the daemon's Objecter. It does
not configure the actor or install a deadline. No shell command or librados is involved.
`Backend` exposes actual compound `ObjectOperation` requests. `PoolListing`
exposes native namespace-aware pages. Its associated opaque cursor is the native
`osdc::PoolListCursor` in production and a script position in fault tests.
`PoolPage` only wraps the page result; the native continuation passes unchanged.
The fault backend exists only in tests.

### Required daemon deadline policy

1. Start Objecter with `ObjecterConfig::op_timeout = Some(positive finite Duration)`.
   A caller-only timeout is insufficient. Dropping `Objecter::operate` drops its
   response receiver. The actor can retain and resend the operation until its
   own timeout or completion. `RadosBackend::new` does not enforce this setting.
2. Bound each logical operation's total time, retry count and outstanding native
   requests. Prefer keeping one worker awaiting completion while a caller parks.
   Do not accumulate retries by repeatedly dropping response receivers. Native
   pool-list pages have their own finite platform timeout; scan/recovery budgets
   must still bound the whole logical attempt.
3. If an outer deadline drops a mutation future after possible submission,
   classify it as `StoreError::UnknownOutcome`. Cancellation is not rejection,
   fencing proof, or proof that the mutation is absent. Native actor `TimedOut`
   and `Shutdown` already map to `UnknownOutcome` in this backend.
4. Keep the exact logical operation and close admission while its result is
   uncertain. Do not advance the durable prefix or release client success.
   Retain the store/repository instance and pending batch/action. Reconcile with
   guarded durable state; otherwise stop service and recover. Do not create an
   empty namespace or replace an uncertain operation with a new identity.

Retain the following across a caller deadline:

| Operation | Required reconciliation identity/state |
|---|---|
| Append | Exact owner, expected tail, batch id and bytes; use `reconcile_pending` |
| Shard/session/Preparing acquisition | Exact old inspection, requested owner/writer and predecessor proof |
| Bootstrap create | Exact persisted plan, bootstrap id and writer; inspect format before retry |
| Session lifecycle | Exact action, key, expected revision and Open id; preserve incarnation allocation |
| Allocator reservation | Same reservation id and count |
| Checkpoint write/publication | Same generation, bytes and descriptor; inspect the published pointer |
| Trim | Exact descriptor and bounded sequence set; reconcile the trim cursor |

Read-only deadline failure is not successful load or scan exhaustion. Retry it
only within a bounded attempt. No format write follows an incomplete pool scan.

## WAL and checkpoints

- Construct `StableStore::new(backend, identity, limits)`.
- `inspect_control()` is read-only. It returns writer addresses and predecessor
  evidence. It does not grant ownership or load the namespace.
- After fencing, call `open(expected, new_owner, writer, predecessor)`.
  Acquisition preserves tail and checkpoint. Retain the exact arguments on an
  unknown result. `OpenedShard` has a private frozen snapshot.
- Encode records with `record::encode_record_bounded(record, max_body_bytes)`.
  Reserve `framing::OVERHEAD` from `Limits::max_record_bytes` for each WAL value.
  The preflight size estimate is conservative. It can reject near-limit input.
- `append(owner, expected_tail, batch_id, bodies)` admits one exact batch.
  `reconcile_pending()` reconciles an unknown result. A different batch cannot
  replace it. Cancellation retains the pending batch too.
- `load(opened)` requires a checkpoint. It returns materialized
  `EngineCheckpoint` state and exactly the uncovered records. The daemon then
  uses checked engine restore/replay. Short OMAP pages continue from their last
  returned key. Missing required keys, nonprogress or continuation inconsistent
  with the frozen tail are errors. No empty-namespace fallback exists.
- `write_checkpoint(owner, generation, state)` writes immutable state and its
  manifest. It requires native pool listing to bound all retained checkpoint
  objects, including orphans from earlier owners.
- `publish_checkpoint(owner, expected_generation, descriptor)` guards the exact
  previous descriptor and a captured numeric tail. Coverage cannot move back.
- `trim(owner, descriptor, sequences)` removes a bounded contiguous prefix.
  The exact current descriptor guards removal. Retried removed keys are harmless.

WAL admission uses a conservative byte bound: retained keys times the maximum
record size. Publication alone does not release this budget. Trim must advance
the durable trim cursor. Old checkpoint generations remain; no garbage collector
exists. A checkpoint backlog `Capacity` result must stop ordinary admission
immediately. Do not keep admitting mutations until the separate WAL budget fills.

Production bounded encoding writes WAL v3/compat3, checkpoint v2/compat2,
checkpoint-session v2/compat2 and completion v2/compat2. It rejects Legacy2 input.
Completion v2 appends the optional original reply-target inode. The unbounded
encoding helpers remain fixture helpers; do not use them for production admission.
Decoding accepts only the exact old/new pairs. WAL v2, checkpoint v1 and their
nested old frames select Legacy2; a missing target stays unknown None. Schema 1
WAL remains unsupported. Unknown versions remain `UnsupportedVersion`.
The outer checksum/identity envelope stays version 1. The legacy segment codec
remains a fixture helper only.

Authority v1 selects Legacy2. Authority v2 includes required semantics 3 and its
CRC. Acquisitions and every revision preserve that selection. Append, checkpoint
publication and trim compare the complete encoded WAL authority, not only owner.
Normal load requires semantics 3 and rejects legacy uncovered WAL. Normal session
mutations reject Legacy2. Read-only inspection and the explicit upgrade loader
retain access to valid old state without guessing missing target identities.

## Rank sessions

`SessionRepository::new(backend, identity, rank, max_records)` owns a separate
authority. Use `inspect_sessions`, then `acquire_sessions` after fencing its
writer. `load_sessions` includes Closed tombstones. Each mutation takes acquired
ownership and the current revision:

- `open_session_record` returns `(SessionKey, revision)`. Retain its `open_id`
  across uncertainty. A live OPEN attaches or updates metadata without allocating
  another incarnation. Metadata keys and values preserve opaque bytes.
- `mark_session_closing` precedes shard cleanup.
- `mark_session_closed(key, cleanup_seq)` follows the engine's durable cleanup
  receipt. The daemon must validate that receipt and the shard/rank join.

Unknown mutations retain their exact operation. Other actions fail closed until
the outcome is known. Incarnation allocation and the Open row commit atomically.

## Bootstrap and allocator

`BootstrapPlan::new` accepts the initial materialized root, initial range, owner
and writer. It creates exact object names and bytes. The plan permits only one
empty root and its complete reserved range. It does not accept a historical log
as a root image. The caller must explicitly supply a Legacy2 seed. The original
plan, root checkpoint, initial Authority-v1 objects and allocator receipt use the
frozen legacy encoding. Their bytes do not change when the production codec does.

`BootstrapStore` provides `inspect_format`, `begin_bootstrap`,
`acquire_preparing`, `resume_bootstrap`, `publish_ready` and `validate_ready`.
Fresh bootstrap scans every application namespace with an empty allowlist.
Resume accepts only exact plan objects. Map changes restart the scan. Retry,
page, byte and time limits fail closed.

The format guard cannot acquire the shard or session guard. On takeover,
`resume_bootstrap` verifies/creates the original planned dependencies, then
returns `Fenced` if either independent guard differs. The daemon must inspect,
fence and acquire those objects with `open` and `acquire_sessions`, then retry
resume. It never substitutes a new
bootstrap identity, checkpoint name, root seed or allocator range.

`validate_ready` rejects missing dependencies. Later mutable repository positions
are not compared with their initial counters.

`InoAllocator::reserve_range(reservation_id, count)` atomically advances the global
high watermark and records the reservation. Retain the reservation id across
uncertainty and restart. Lost reservations waste numbers; they do not reuse them.
The daemon journals a reserved range before the engine can use it.

## E1 checked adoption

Additional APIs:

```text
BootstrapStore::acquire_format(expected, writer, predecessor)
BootstrapStore::upgrade_retry_targets(expected, shard, repository, opened,
    sessions, engine_config, upgrade_id, check_authority) -> RetryUpgradeReady
```

`acquire_format` generalizes Preparing acquisition without changing semantics or
intent. It guards the format object's complete authority, phase and present
intent. Format inspection reads plan data and all xattrs in one atomic operation.
Absence of an intent is not an empty-value comparison. The guarded revision
protects its inspected absence.

The seed-completing process must stop unready at `UpgradeRequired`, before Loaded
or any OPEN/service. It cannot self-upgrade, including after Preparing takeover.
The later monitor-assigned process must use a higher incarnation and different
GID, fence every actual predecessor and the persisted candidate producer, and
acquire all three objects independently. Retain the exact `acquire_format` return
handle. Each inspection has a private immutable snapshot of its complete
authority, phase, intent and canonical plan bytes, including filesystem identity.
All format consumers reject changed public projections before backend I/O or
acquisition reconciliation. Private, nonpersistent acquisition evidence binds
both the validated source snapshot and the exact returned snapshot. Only a real
Ready takeover, or exact-argument reconciliation of that takeover, supplies the
Ready-origin proof. Preparing publication and inspection alone do not supply it.
Cloning a handle cannot transfer its proof to changed public fields.
The daemon still supplies assignment and fencing proof;
store does not invent an owner or self-blocklist.

The coordinator refuses pending store/repository operations. It loads all guarded
rank rows before legacy replay, binds range-only historical keys by their cleanup
lifetimes, and lets actual terminal records retire them. It checks the all-Closed
receipt bijection at the real tail, obtains the engine's checked clean export,
checks allocator reservations and compares every logical field. Valid surviving
legacy Open/Closing state returns `UpgradeRequired`; no history is dropped.

It writes/read-verifies the candidate, installs the irreversible format barrier,
promotes sessions with their own CAS, then atomically promotes WAL authority and
the candidate pointer on the WAL object. It trims under new guards and rechecks
state before final Ready. `rmds.retry_upgrade` is bounded to 8192 bytes and remains
as origin metadata. The coordinator returns refreshed format/shard/session handles.
It uses the supplied `EngineConfig` snapshot for all source/candidate engine checks.

The live synchronous callback runs before and after each backend await, including
inner reads, pages and reconciliation. The independent acquisition worker must
also check live authority at its own backend-operation boundaries; an outer check
around a multi-I/O future is insufficient. Preserve the positive finite Objecter
actor timeout and all caller deadline rules above. A cancelled/unknown transition
retains the exact upgrade id and arguments. Reconcile it before any next stage.

After the barrier, only its persisted candidate can be adopted. A fenced producer's
original immutable name is allowed only by the private upgrade publication path.
Normal checkpoint publication keeps its current-owner naming rule. Once WAL is
promoted, recovery uses that candidate and does not need trimmed legacy records.
There is no rollback, checkpoint deletion, identity substitution or GC.

Ready3 ordinary restart uses the latest pointer and can have new Open rows and
histories. It does not repeat the all-Closed adoption rule or use the retained
origin intent as the current checkpoint. Checked load, rank join, both E2 gates
and confirmed monitor service remain daemon obligations after an upgrade result.

## Verification

```sh
CARGO_TARGET_DIR=target-store-e1 cargo test -p store --locked
CARGO_TARGET_DIR=target-store-e1 cargo clippy -p store --all-targets --no-deps --locked -- -D warnings
```

Live guard, listing and restart evidence remains QA-owned. Independent review is
required before daemon integration.
