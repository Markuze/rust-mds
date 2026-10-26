# sol-engine-review
Task: Independently review the frozen engine E2 package against the accepted recovery contract. Report ACCEPT or ranked concrete findings. Preserve the separate earlier F1 and admission/failure/visibility verdicts.
Status: E2 ACCEPT. No blocking finding in the assigned engine-only scope. The earlier F1 and admission/failure/visibility ACCEPT verdicts remain unchanged. This does not qualify daemon integration or mounted-client G3.
Context: Reviewed E2 against accepted checkpoint a6ae05a and design/recovery-contract.md section 2. Traced every cap-output source, recovery gates, marker ownership, control/job progress, survivor sealing, failure handling, and the two lint-only changes. Earlier reviews below retain their original scope and evidence. Only this report was edited. Builds and stdin probes used target-sol-engine-review.
Next: Parent can integrate E2 with the separately reviewed daemon caller migration. Both recovery boundaries and required output/work drain must precede daemon readiness. E1 and mounted-client G3 remain separate assignments.
Log:
- [2026-10-25] Completed independent E2 source review and verification: ACCEPT. cargo test -p engine passed 185 unit/integration tests plus 1 doctest, including all 10 recovery_controls tests. Strict engine --no-deps clippy and engine formatting checks passed. Three additional stdin-only production-step probes passed. No source edits, live actions, commits, subagents, scaled stress, or daemon checks ran.
- [2026-10-25] Started the independent E2 review with Astra Ultrafast. Only this report is writable outside target-sol-engine-review build artifacts. No source edits, live actions, commits, or subagents.
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

## E2 independent review — 2026-10-25: ACCEPT

**ACCEPT for the frozen engine E2 package.** No ranked blocking findings. This verdict is separate from the earlier F1 and admission/failure/visibility acceptances.

### Scope and baseline

Reviewed the engine diff against accepted checkpoint `a6ae05a`, including the new `crates/engine/tests/recovery_controls.rs`. Read the final author ledger, accepted E2 section of `design/recovery-contract.md`, scoped contract review, and E2/realm requirements in `current/astra-g3-integration.md`. The unfinished daemon diff was outside this review.

The only public input addition is `EngineInput::RecoveryControlsDrained`. The phase, accepted key set, deferred evaluation identities, deferred range identities, and advertised range bound are runtime state. The diff adds no persistent schema or ordinary E1 behavior.

Source locations below are relative to `crates/engine/src/`.

### Boundaries and ownership

- `engine/durable.rs:106-147` initializes every checked restore in Reconstruction. The direct bootstrap constructor retains its documented serving exception. Checked load cannot inherit that exception.
- `engine/reconnect.rs:127-173` checks the duplicate-free full session-key set, final chunks, empty commit/continuation state, and equal submitted/durable/journal positions before the first seal. It stores that set separately from live sessions. Same-set repeats return the current state with empty output after later cleanup or while a later continuation/commit waits. A different set is invalid. Post-seal chunks and reset fail before mutation (`:26-36,92-100`).
- Structural finish opens control/job work, queues cap identities for bounded revoke evaluation, and exposes eligible retained jobs. It submits no record and schedules no new stray purge. Legacy `reconnect_done` cannot open either checked-load gate.
- `engine/step.rs:114-173` rejects ordinary requests while grants remain closed. This check is independent of whether the marker is Busy or retained. Reconstruction, an earlier continuation, or a pending commit returns the marker and permit unchanged through Busy.
- A permitless marker becomes the normal continuation. Repeated permitless turns preserve it. Its admitted turn changes only the phase and queues fresh evaluation/stray identities (`:207-210`). It emits no WAL, grant payload or notice and returns the permit unused. Deferred work runs on later admitted turns. A released duplicate is empty and returns its permit; commit/continuation exclusion still applies.
- Failure clears deferred work and owned continuations. Step, finish, cap helpers, and job enumeration keep the terminal failed behavior. Late commits and repeated markers cannot release discarded output or restart work.

### Cap-output audit

| Output source | Checked behavior |
|---|---|
| Cap evaluation, including cleanup and commit callbacks | `engine/caps_eval.rs:233-333` retains inode identities before any grant mutation. Reconstruction does no policy issue. Control drain can only reduce a valid cap's pending rights. Stale-generation reissue stays deferred. |
| Reply target, dentry and READDIR caps | Request dispatch and execution remain closed (`engine/mod.rs:534-537,604-607`). Checked restore carries no pending request reply. The shared `inode_stat` builder also suppresses cap issue and advertises max_size zero before release (`engine/caps_eval.rs:460-515`). After release it reads committed images. |
| Range-only sharing | `send_max_size_grant` retains `(inode, SessionKey)` before sequence mutation. Reconnect also retains these identities. The queue is not runnable until release. Later evaluation uses the current cap and committed range, including an unchanged cap mask (`engine/caps_eval.rs:336-361,560-591`). |
| REVOKE and retained TRUNC | The common message builder uses effective rights during drain. Non-GRANT messages clamp max_size to the last advertised bound and committed range. Reconnect starts that bound at zero. Retained truncate effects keep their committed metadata and do not advance a grant sequence (`engine/caps_eval.rs:364-440`; `engine/durable.rs:681-703`). |
| New and duplicate FLUSH_ACK | New ACKs remain deferred until commit. Both ACK paths mask echoed rights to effective rights before release and carry no inode/range payload. Durable history duplicates do not reapply dirty metadata (`engine/client_caps.rs:66-103`; `engine/mod.rs:1264-1312`). |
| Session stale/resume | Resume reaches the same evaluator. It cannot restore effective rights, change the cap generation, or advance a grant sequence while the gate is closed. The existing stale-reader regression covers this path. |
| Reconnect realm correction | Chunk validation builds authoritative realm traces before installation. These traces carry no cap/range grant. E2 adds phase rejection without changing realm reconstruction. Actual SNAP UPDATE/SPLIT encoding and funnel ordering remain the daemon consumer's responsibility. |

Commit callbacks install each record's committed image before releasing its effects. Missing-key cleanup and callbacks during Reconstruction cannot grant. Callback evaluation during Control drain can revoke but cannot grant. Deferred grant storage contains identities, not old attribute payloads. All grant construction after release reads the committed image source.

### Control and lifecycle progress

`engine/step.rs:118-135` leaves a new dirty control externally owned when RecoverSize blocks it and its cap has no issued writer rights. Busy returns before cap, history, journal or permit mutation. A history-proven duplicate bypasses this metadata conflict and follows the dedupe ACK path. An issued writer's dirty acknowledgment remains admitted.

`engine/mod.rs:1095-1119` atomically replaces affected RecoverSize guards in that writer's flush record. Replacement retains prior layouts and at least the prior bound, uses the new inode version/truncate sequence, and assigns a fresh job ID. Old results cannot retire the replacement.

`pending_jobs`, `emit_job` and `complete_job` use structural completion, not grant release or ACTIVE (`engine/durable.rs:641-776`). Retained truncate work can complete first. Writer acknowledgments remove the issued-right dependency. The guarded probe then runs and commits before retry of the externally retained dirty input. Invalid result kinds and stale IDs do not retire a job. No grant is needed for this valid recovery sequence. The daemon must schedule those dependencies ahead of the Busy control, as required by the accepted contract.

The ten recovery regressions exercise this sequence through manual journals and production steps. They also cover zero survivors with retained work: required recovery runs during drain; new stray purge waits for release. Existing one-record submit checks and submission-free callbacks remain intact. The `parked_ready` correction lets queued evaluation drain when a parked request still waits for input.

### Lint-only changes

- `engine/caps_eval.rs:145`: iterating `i.caps.keys()` preserves the same keys and order as the old unused-value map iteration.
- `engine/mod.rs:868-880`: the let-chain preserves the original `excluded != 0` short circuit and optional-cap condition. Its revocation and reply behavior is unchanged.

### Independent verification

All Cargo compilation/test/lint commands used `CARGO_TARGET_DIR=target-sol-engine-review`.

- `cargo test -p engine`: **185 unit/integration tests plus 1 doctest passed**. Zero failures, ignored tests, or reported skips. This includes 10 recovery_controls, 20 durability, 22 review_regressions and 26 cap tests. Only the three ordinary default stress scenarios ran with this crate check.
- `cargo clippy -p engine --all-targets --no-deps -- -D warnings`: exit 0, no warnings.
- `cargo fmt -p engine -- --check`: exit 0.
- `git diff --check -- crates/engine current/sol-engine-review.md`: exit 0.

Compiled three additional tests from stdin against the freshly built engine library. No source file was created or changed. Artifact: `target-sol-engine-review/review-e2-probe`.

Command: `target-sol-engine-review/review-e2-probe review_e2_ --nocapture`.

Result: **3 passed**, zero failures:

1. `review_e2_owned_marker_survives_repeated_permitless_turns`: eight permitless turns, duplicate seal calls, duplicate committed-prefix events, repeated markers and attempted ordinary requests preserve the owned marker. Busy returns the exact request/permit. No cap or journal state changes before execution. Execution returns the unused permit and emits no payload. Later turns grant normally without a WAL append.
2. `review_e2_two_dirty_commits_and_poisoned_duplicate_use_latest_committed_fields`: two ordered dirty FW/AX/XX controls have no corresponding reconnect-issued rights. Each survives four permitless turns and a partial/old-prefix completion without early ACK or competitor grant. Each admitted step submits one record. Each callback submits none. A durable duplicate with poisoned size/uid/xattrs and all cap bits emits a bounded ACK without a record or checkpoint change. The released grant contains the second committed image.
3. `review_e2_nonempty_failure_discards_owned_marker_and_late_flush_output`: retained truncate work and reconstructed caps exist at failure. Cover Reconstruction, Control drain with a pending dirty record, an owned permitless marker, and a released gate with a pending dirty record. Repeated late commits, resume calls and markers produce no effects/notices, no new record, and no runnable job after failure.

This accepts engine E2 only. Required daemon caller migration, ordinary E1 policy, storage-fault qualification and mounted-client G3 evidence retain their separate review and QA boundaries.
