# sol-engine-review
Task: Independent Sol review of the completed F1 engine implementation against f54d298. Review only the assigned engine files. Report ACCEPT or CHANGES REQUIRED with actionable findings.
Status: ACCEPT for the re-reviewed admission/failure/visibility package. Both prior findings are fixed. Original F1 ACCEPT remains unchanged. This does not accept the complete G1 gate or the parallel reviewer's recovery-state scope.
Context: Reviewed the assigned diff against f54d298. Read HANDOVER.md, the build brief, requirements, approved workplan, persistence contract, and author ledger. Traced adjacent cap-update, session cleanup, namespace purge, attribute, and read paths to check interactions. Only this ledger was edited. Builds used target-sol-engine-review. No live cluster actions, commits, subagents, or whole-workspace formatting ran.
Next: Parent can integrate the admission/failure/visibility fixes, subject to the parallel recovery-state review and consumer integration checks. No remaining blocker in this review scope.
Log:
- [2026-10-22] Fix re-review complete: ACCEPT in the assigned admission/failure/visibility scope. Recompiled both original independent probe scenarios against the revised library. Both pass. Added repeated permitless-resume and duplicate old-cleanup-event checks; all 3 independent probes pass. Existing durability and review_regressions suites pass all 37 tests. No unrelated full-suite or live rerun was needed.
- [2026-10-22] Started fix re-review. Keep both earlier verdicts as historical evidence until verification completes. No production edits permitted.
- [2026-10-22] Started the next-package independent admission/failure/visibility review. Only this ledger remains writable. Use target-sol-engine-review for verification.
- [2026-10-22] Read the final amended persistence contract and author integration notes. Traced step, submit, completion, direct cap hooks, close cursors, request holds, file-lock controls, and job output boundaries. No production/test source edits made.
- [2026-10-22] Independent existing tests: CARGO_TARGET_DIR=target-sol-engine-review cargo test -p engine --test durability passed all 20. CARGO_TARGET_DIR=target-sol-engine-review cargo test -p engine passed 153 tests plus 1 doctest, with no failures, ignored tests, or reported skips.
- [2026-10-22] Compiled additional Rust tests from stdin against the reviewed engine library. Only build artifacts were written under target-sol-engine-review. review_deferred_admission_ack_must_wake_parked failed: WaitingInput instead of NeedsAdmission after a pure acknowledgment resumed with a permit. review_cleanup_must_retire_eviction_marker failed: the second incarnation emitted no required EvictClient. Both probes exited 101. Reproduction details follow below.
- [2026-10-22] Created the review ledger before other work.
- [2026-10-22] Completed source review. PendingCommit retains the submitted inode images. process_commits installs each record's images before releasing that record's deferred effects. Later pending cap updates do not change those output fields. Purged images retire only at their purge commit. Replay initializes the committed image map.
- [2026-10-22] Checked the request barrier before resolution and READDIR processing. Checked deferred cap evaluation and its queue drain. Cap receipt confirmation and flush submission remain runnable. The revised run_parked exit condition prevents spinning on deferred cap evaluation while a record remains pending. The pending-subset-issued invariant matches Capability's issued-set construction.
- [2026-10-22] Independent verification: CARGO_TARGET_DIR=target-sol-engine-review cargo test -p engine passed 133 tests and 1 doctest. No failures, ignored tests, or reported skips. CARGO_TARGET_DIR=target-sol-engine-review ENGINE_STRESS_SEEDS=64 ENGINE_STRESS_STEPS=2000 cargo test -p engine --test stress passed all 3 scenarios.

## Review report

**ACCEPT — assigned F1 scope.** No ranked blocking findings.

The immutable images fix the 24 MiB versus 64 MiB partial-completion leak. Reply attributes and TRUNC/data-truncation fields now use the same committed image source. The topology barrier prevents speculative child, ancestor, and path-error output. Cap acknowledgments do not wait behind that barrier.

### Non-blocking limits and further regression coverage

- `crates/engine/src/engine/mod.rs:539` and `crates/engine/src/engine/caps_eval.rs:217`: continuous pending cap flushes can delay requests and cap policy indefinitely. This is the documented conservative scheduling limit, not a newly discovered partial-commit leak. The next admission package must provide a drain opportunity. Test a parked request under sustained cap flush input and require bounded progress after admission pauses.
- `crates/engine/src/engine/caps_eval.rs:493`: add a deterministic partial-commit TRUNC test. Queue a truncate and a later dirty cap image. Complete only the truncate. Assert its TRUNC fields, TruncateData fields, reply size, and max_size match its journal image. Repeat with a completion that covers both records.
- `crates/engine/src/engine/mod.rs:998`: add deterministic pending lifecycle coverage. Close or evict a session with a pending create/unlink/range update, then reopen the same client ID. Advance one record at a time. Check that old-session replies do not escape, committed images do not disappear early, and final purge retires the image without a later grant or panic.

Result-based submission, admission bounds, failed/quiescent state, checked checkpoint/replay, durable lifecycle jobs, and complete session fencing remain explicitly outside this F1 package. Their absence is not evidence that G1 is complete.

## Next-package review: admission, failure, and visibility

**Historical verdict: CHANGES REQUIRED.** The following findings were reproduced before the fixes. Both are resolved by the fix re-review below. Original F1 acceptance above remains valid.

### 1. P1 — Deferred admission loses the control-input wakeup

Locations: `crates/engine/src/engine/step.rs:106-111`, `:152-162`, `:208-214`.

`control` describes this call's incoming input, not the retained continuation that the call executes. An acknowledgment accepted through `step(Some(CapUpdate), None)` is retained. The later `step(None, Some(permit))` executes that acknowledgment with `control = false`. A pure acknowledgment can satisfy a parked request's hold without submitting a record. `run_parked` correctly does not run inside that step. However, the step then returns `WaitingInput`, not `NeedsAdmission`, because it does not mark the parked request ready. No commit event will arrive. A daemon that obeys the contract and does not poll WaitingInput leaves the request blocked indefinitely.

Independent reproduction:

1. Create and commit a C1 writable file. Install C2.
2. Submit C2 GETATTR with `CEPH_STAT_CAP_INODE_ALL` through step. Capture its C1 revocation. The state is WaitingInput.
3. Build a clean C1 UPDATE acknowledging the returned cap seq/bits. Keep the cap's existing wanted mask. This preserves its range and needs no journal record.
4. Accept that input with no permit. Resume it with `step(None, permit)`.
5. The journal tail stays unchanged, but the state is WaitingInput. It must request a fresh step to retry C2.

Action: derive the control/progress classification from the operation actually executed, including retained continuations. Preserve that classification across permit acquisition and release cursors. Do not execute the parked request in the acknowledgment step.

Required regressions: compare immediate-permit and deferred-permit pure acknowledgments; require the same wakeup and eventual reply. Cover an unlock and the last item of a resumed CapRelease cursor as well. Check one-record limits and unused permits throughout.

Probe: `target-sol-engine-review/review-probe --exact review_deferred_admission_ack_must_wake_parked --nocapture`.

### 2. P1 — Production cleanup leaves the old incarnation's eviction suppression marker

Locations: `crates/engine/src/engine/mod.rs:1191-1197`; `crates/engine/src/engine/step.rs:217-241`. Compare legacy cleanup at `crates/engine/src/engine/client_caps.rs:433-435`.

The eviction suppression set is keyed by ClientId. Production terminal cleanup removes the session and publishes its receipt, but it never removes that client's `evicting` entry. The legacy close helper does remove it. After the daemon installs a new incarnation for the same ClientId, a later stale-writer conflict calls `evicting.insert(client)`. It returns false because the retired incarnation left its entry behind. The engine emits no EvictClient, so the conflicting request has no eviction-driven progress path.

Independent reproduction:

1. C1 creates a directory and holds its exclusive file cap. C2 READDIR causes a revocation.
2. Mark C1 stale. Verify the engine requests EvictClient.
3. Finish C1 cleanup through production CloseSession steps and journal events. Verify the terminal receipt. Clean up C2 too, so no cap conflict remains.
4. Install new C1/C2 incarnations with the same ClientIds. Repeat directory creation, C2 READDIR, and C1 staleness.
5. The first incarnation emits EvictClient. The second emits no effect, although it reaches the same stale-writer revocation state.

Action: retire incarnation-local eviction suppression state when terminal cleanup commits, or scope that state by SessionKey. Preserve suppression while the original eviction remains in progress.

Required regression: evict, durably clean up through step, reopen the same ClientId with a new incarnation, and cause another eviction. Require one eviction request for each incarnation, with no duplicates within either incarnation.

Probe: `target-sol-engine-review/review-marker-probe --exact review_cleanup_must_retire_eviction_marker --nocapture`.

### Other checked boundaries

- Production submit consumes one permit and fails closed on a missing permit, an oversized size bound, submission error, or noncontiguous returned sequence. Actual encoded-size validation remains an explicit Journal adapter obligation. This engine-only review does not qualify a store adapter.
- Production commit callbacks materialize effects and schedule purge continuations without calling the journal submission path. Auto-commit still waits for the explicit completion boundary.
- Failed step results clear effects. Failure clears pending commits and continuations. Late committed events do not release discarded success or data work.
- The immutable committed-image path remains in place for cap fields, replies, and truncate notifications. Production stepping rejects later mutation inputs while a record remains pending.
- The terminal cleanup record follows the per-cap records and emits its receipt only on commit. Finding 2 concerns runtime state retirement at that boundary, not premature publication of the receipt.

No live checks, workspace checks, commits, code edits, subagents, or new G4 concurrency requirements were introduced. Deep recovery-state acceptance belongs to the parallel review.

## Fix re-review: ACCEPT

Both admission findings are resolved. No remaining blocker was found in this review scope.

### Verified fixes

1. `crates/engine/src/engine/step.rs:164-166` derives progress classification from the continuation that the step executes. Repeated `step(None, None)` calls retain the input without applying it. A later permitted pure acknowledgment returns NeedsAdmission and the unused permit. It does not execute the parked request in the same step. The next permitted step returns the parked reply. The same classification covers resumed unlocks and every CapRelease cursor item, including the last one.
2. `crates/engine/src/engine/mod.rs:1191-1198` retires eviction suppression only when terminal cleanup matches the current session incarnation. Reconnect bookkeeping retirement uses the exact SessionKey. A new incarnation can request its own eviction. Duplicate old cleanup completions neither remove the fresh session nor clear its eviction suppression.

### Cross-effects checked

- The durable writer-range scan in close_step handles one inode and then retains the close continuation. It does not continue into the terminal record in that step.
- The common range-cleanup path either submits a range/recovery record, submits a purge record, or performs no persistent mutation. Pending inode state prevents a second submission after a range record.
- New post-gather stray work enters the purge queue. Gather completion and journal completion do not submit that work. Each purge requires a fresh permitted step.
- The existing one-record permit consumption, failed-output suppression, and committed-image materialization paths remain intact. The fixes introduce no public API or persistent schema change in this review scope.

### Independent verification

Command: `CARGO_TARGET_DIR=target-sol-engine-review cargo test -p engine --test review_regressions --test durability`.

Result: 17 review regressions and 20 durability tests pass. No failures, ignored tests, or reported skips.

Recompiled stdin-only probe source against the revised engine library. Build artifact: `target-sol-engine-review/review-recheck-probe`.

Command: `target-sol-engine-review/review-recheck-probe review_ --nocapture`.

Result: all 3 independent probes pass:

- `review_deferred_admission_ack_must_wake_parked`: original failing pure-ack scenario now wakes and returns the parked reply on a fresh step.
- `review_repeated_permitless_resume_retains_ack_and_ready_request`: eight permitless resumes before applying the acknowledgment and eight after its wakeup preserve progress. No record is submitted; no parked reply escapes early; the unused permit returns.
- `review_cleanup_must_retire_eviction_marker`: original failing two-incarnation scenario now emits the required eviction for both incarnations. Extended checks deliver the old cleanup prefix three times during the fresh incarnation's eviction. They produce no effects/notices/submissions and do not allow duplicate fresh eviction output. Every cleanup step submits at most one record; every checked callback submits none.

Only this report and test build artifacts were written. Original F1 acceptance remains unchanged. Store/daemon integration, actual encoded-byte enforcement by the journal adapter, full G1 evidence, and the parallel reviewer's deep recovery-state verdict remain separate.
