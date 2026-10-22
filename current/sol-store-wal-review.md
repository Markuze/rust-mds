# sol-store-wal-review
Task: Independent Sol review of completed G2 stable-store core WAL/checkpoint and backend correctness. No source edits, commits, subagents, or live actions.
Status: ACCEPT after correction re-review. Original P1 is closed. No remaining core correctness blockers found. Independent 50-test run and store-only strict clippy passed. G2 live remains unqualified.
Context: Base f54d298. Reviewed current backend.rs, framing.rs, stable.rs, their tests, integration tests, and relevant codec interfaces. Compared native compound operations and pagination with osdc and C++ Ceph. Scope excludes active engine work and the other reviewer's bootstrap/allocator/rank-session/schema review. Only this report was edited. Tests used target-sol-store-wal-review.
Next: Parent can proceed with integration. Enforce the documented deadline, uncertainty and admission preconditions. QA retains live qualification.
Log:
- [2026-10-22] Created review ledger before inspection.
- [2026-10-22] Read AGENTS, HANDOVER, build brief, requirements/hooks/decisions, approved workplan, storage design, frozen persistence contract, author ledger and store README.
- [2026-10-22] Independent cargo test passed: 31 unit + 5 checkpoint/schema + 4 legacy fixture tests; 0 failed, 0 ignored. No live actions.
- [2026-10-22] Independent store-only strict clippy passed. Completed core review. Found valid native short pages rejected as committed-history damage.
- [2026-10-22] Started requested correction re-review. Author reports 50 tests and strict store clippy passed. Prior verdict remains pending independent verification.
- [2026-10-22] Completed correction re-review. Independent 50-test run and strict store-only clippy passed. Closed original P1. Verified short-page limits, frozen-tail boundary, cancellation retention, opaque native cursor forwarding and real bootstrap scan-loop coverage. Verdict ACCEPT; no live qualification.

## Current verdict: ACCEPT

No remaining concrete correctness blockers were found in the correction scope. The original P1 is closed. The original finding below is historical evidence, not an open request.

### Correction verification

- `crates/store/src/stable.rs:602-645` now accepts nonempty short pages and advances from the last validated sequence. `decode_page` returns the continuation flag. Missing middle/final records, empty pages, repeated keys and early end fail explicitly. Every page still compares identity, owner, frozen tail and exact checkpoint descriptor.
- There is no sequence off-by-one. The request count is at most `frozen_tail - next`. The decoder rejects more values than requested. Thus `next += 1` cannot cross the frozen tail or overflow, including tail `u64::MAX`. A page cannot legitimately include records after that tail. If it reaches the tail with `more=true`, the implementation reports corruption rather than reading extra keys. This is safe for this backend: record insertion and tail advance are one atomic operation, and the same compound read compares the frozen tail. A subsequent append cannot interleave within that read. Extra `r/` keys after an unchanged tail contradict the stable-WAL invariant; they are not a valid short-page case. Native Ceph sets `more` when it encounters another matching key after its limit, not merely when the last returned value fills the limit (`PrimaryLogPG.cc:7984-8003`).
- Short pages cannot cause an unbounded loop. Each accepted page advances at least one required sequence. Required count is checked before the loop. Per-value and encoded-page bytes remain bounded; encoded overhead is exactly 26 bytes per 18-byte key plus 5 bytes per response. Aggregate frame bytes remain bounded by `max_wal_bytes`. The added tests exercise count, aggregate bytes and individual value limits.
- Count-capped and byte-capped regressions check both returned records and exact continuation requests (`stable.rs:2619-2650`). The byte mock matches Ceph's accumulated-byte check before the next entry. Missing middle/final keys and an owner change after page one fail (`:2654-2689`). Nonprogress and contradictory continuation fail within at most two calls (`:2693-2719`). The author records both positive regressions as red before the production correction. I independently verified their corrected pass, not the historical red run.
- The associated cursor is a testability boundary, not a native protocol change. `RadosBackend` still uses `osdc::PoolListCursor` and forwards namespace scope, request bounds, cursor, objects, continuation and map epoch without transformation (`crates/store/src/backend.rs:118-140`). No persistent schema changed.
- The requested bootstrap multipage tests execute the actual `BootstrapStore::begin_bootstrap -> scan` loop. Scripted pages only supply backend results. Tests assert the full cursor-call sequence, script exhaustion, write counts at every listing call and final object state (`stable.rs:2772-2886`). They cover empty PG continuations, later-page foreign namespace, map change after partial progress, restart from `None`, successful complete rescan and repeated-cursor rejection. Failed scans leave zero writes. Successful scans create only `rmds.format` after the final page. These are loop regressions, not mock-only assertions.
- Caller cancellation after a durable append now has direct regression coverage (`stable.rs:2890-2931`). The dropped future leaves the exact pending batch. A different batch is rejected until reconciliation proves the original commit. The positive finite actor timeout, bounded logical attempt/retries/outstanding operations and closed admission during uncertainty are explicit integration preconditions in `backend.rs:69-82` and `README.md:17-48`. I accept these as documented caller obligations. The constructor does not enforce them. A caller-only deadline still does not cancel the actor operation.
- Checkpoint backlog pressure is explicit in `README.md:80-84`: `Capacity` stops ordinary admission immediately. It is not permission to admit until the independent WAL budget fills.

## Original verdict: CHANGES REQUIRED — resolved

### 1. P1 — Recovery rejects valid truncated OMAP pages — CLOSED

**Location:** `crates/store/src/stable.rs:612-616`, `:1157-1160`. The test backend at `:1296-1316` hides this behavior.

`load_tail` requests up to `n` records. It reports `Corrupt` unless the response contains exactly `n` records. `decode_page` validates the final `more` byte but discards it. However, `OMAPGETVALS` specifies a maximum count, not an exact count. Ceph can return a smaller nonempty page with `more=true`. It clamps the requested count to `osd_max_omap_entries_per_request`. It also stops when the accumulated response reaches `osd_max_omap_bytes_per_request`.

**Native evidence:** `/home/ubuntu/ceph/src/osd/PrimaryLogPG.cc:7962-7964` clamps the count. Lines `7982-8003` implement the byte limit and encode the continuation flag. `crates/osdc/src/ops.rs:495-508` exposes this native protocol without an exact-count guarantee.

**Failure:** Commit records 1 and 2. Leave checkpoint coverage at 0. Crash and reopen. With `page_keys=16`, an OSD byte limit that permits only the first value returns record 1 and `more=true`. Both required records exist and pass their checksums. Recovery nevertheless returns `Corrupt` before requesting record 2. This also occurs with a supported store page size above the OSD entry cap. It prevents recovery of acknowledged metadata on a healthy store.

**Minimal fix:** Return the continuation flag from `decode_page`. Accept nonempty short pages. Validate each returned sequence against the next required sequence, then continue from the last returned key until the frozen tail. Reject an empty/no-progress page or end-of-log before the required tail. Preserve the owner/tail/descriptor guards and total count/byte limits. Do not treat a short page alone as a missing record.

**Concrete regression:** Make the fault backend cap each OMAP response at one value, even when the request asks for two. Return `more=true` on the first page and `false` on the second. Check that recovery returns both records exactly once. Repeat with byte-based truncation. Then remove record 2 and verify `Corrupt`, not a shortened successful replay. Check that owner change between pages still fails. Existing tests cannot catch this: the mock always fills the requested count when enough matching keys exist.

## Verified boundaries and integration conditions

- Append stores the exact pending batch before its first await. An uncertain response retains it. Owner/tail/batch inspection can prove commit or permit identical resubmission. Same-object guards reject stale append writes. The body checksum includes protected header fields and excludes the trailing CRC from external checksum composition.
- The mock commits a private object image only after all compound operations succeed. It does not supply cross-object CAS. Its important mismatch in this scope is pagination, as finding 1 explains.
- Checkpoint state and manifest use exclusive create or identical-byte verification. Publication verifies both immutable objects before guarded pointer update. Failed pre-publication writes leave orphans. Trim compares the exact descriptor and owner, removes only a contiguous covered prefix, and advances the trim cursor in that compound operation. Missing published objects fail explicitly; old WAL does not trigger fallback.
- Retained WAL admission uses `tail - trimmed`, not checkpoint coverage or current memory use. Checkpoint listing counts retained objects and orphans across epochs. It returns `Capacity` before adding objects beyond its configured backlog. The daemon must turn this result into admission pressure. It must not continue normal admission until the WAL budget fills. This review does not qualify an unfinished daemon adapter.
- A **finite Objecter operation timeout** is an integration precondition, not proof supplied by `RadosBackend::new`. The README and author ledger state it. An outer caller timeout alone is insufficient: dropping `Objecter::operate` drops its receiver, but the actor retains the operation and can resend it (`crates/osdc/src/objecter.rs:543-556`, `:1503-1508`, `:1738-1748`). Repeated caller-only timeouts can accumulate retained operations. Keep the pending batch and use a finite actor timeout; do not interpret cancellation as a rejected write. Native `TimedOut` and `Shutdown` correctly map to `UnknownOutcome` at `crates/store/src/backend.rs:84-90`.
- Native requests set `ONDISK` in `crates/osdc/src/objecter.rs:1438`. `operate_checked` checks overall result, result-vector length and all suboperation results. C++ Ceph also completes on its single requested ONDISK reply (`/home/ubuntu/ceph/src/osdc/Objecter.cc:4030-4034`). No additional blocker was found here.

## Independent verification

Correction re-review:

```text
CARGO_TARGET_DIR=target-sol-store-wal-review cargo test -p store --locked
PASS: 41 unit + 5 checkpoint/schema + 4 fixture tests = 50; 0 failed; 0 ignored; 0 doc tests.

CARGO_TARGET_DIR=target-sol-store-wal-review cargo clippy -p store --all-targets --no-deps --locked -- -D warnings
PASS.
```

Initial review, retained as historical evidence:

```text
CARGO_TARGET_DIR=target-sol-store-wal-review cargo test -p store --locked
PASS: 40 tests; 0 failed; 0 ignored; 0 doc tests.

CARGO_TARGET_DIR=target-sol-store-wal-review cargo clippy -p store --all-targets --no-deps --locked -- -D warnings
PASS.
```

No source edits, commits, subagents, live actions, or inspection of unfinished engine diffs occurred. Finding 1 uses direct source analysis against the native Ceph implementation. No new regression was added because the sole writable file is this report. Offline passes do not qualify G2 native checkpoint/WAL restart or G3 mounted-client failover.
