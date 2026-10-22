# Store integration boundary

This crate implements the G2 object protocol. It has offline fault tests. It has
not passed a live RADOS gate. The daemon must enforce assignment, fencing,
admission and recovery ordering from `design/persistence-contract.md`.

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

Record schema 2 rejects schema 1. Schema 1 cannot represent session incarnations,
lifecycle intent, stray authorization paths, stable subvolume identity or old
layouts. The legacy segment codec remains a fixture helper only.

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
as a root image.

`BootstrapStore` provides `inspect_format`, `begin_bootstrap`,
`acquire_preparing`, `resume_bootstrap`, `publish_ready` and `validate_ready`.
Fresh bootstrap scans every application namespace with an empty allowlist.
Resume accepts only exact plan objects. Map changes restart the scan. Retry,
page, byte and time limits fail closed.

The format guard cannot acquire the shard or session guard. On takeover,
`resume_bootstrap` verifies/creates the original planned dependencies, then
returns `Fenced` if either independent guard differs. The daemon must inspect,
fence and acquire those objects with `open` and `acquire_sessions`, then retry
resume. This boundary needs orchestration review. It never substitutes a new
bootstrap identity, checkpoint name, root seed or allocator range.

`validate_ready` rejects missing dependencies. Later mutable repository positions
are not compared with their initial counters.

`InoAllocator::reserve_range(reservation_id, count)` atomically advances the global
high watermark and records the reservation. Retain the reservation id across
uncertainty and restart. Lost reservations waste numbers; they do not reuse them.
The daemon journals a reserved range before the engine can use it.

## Verification

```sh
CARGO_TARGET_DIR=target-sol-store cargo test -p store --locked
CARGO_TARGET_DIR=target-sol-store cargo clippy -p store --all-targets --no-deps --locked -- -D warnings
```

Live guard, listing and restart evidence remains QA-owned. Independent review is
required before daemon integration.
