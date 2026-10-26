# astra-g2-fault-matrix

## Follow-up independent test review — ACCEPT

Task: Independently review the completed four tests for the two G2 fault families identified below. Inspect the frozen primary daemon and `/tmp/opencode/sol-daemon-d6-g2-fault-20261025.9LwREj/`. Run focused offline checks with a fresh `target-astra-g2-fault-review` target. Own only this report; no production edits, live actions, Git writes, commits, or subagents. The separate reviewer owns D6 timer/reclaim behavior.
Status: COMPLETE. ACCEPT the four tests and the audited G2 storage-fault matrix scope. Family A (interrupted Closing joins) and family B (lifecycle executor/retry/completion boundaries) are CLOSED. No production defect or additional mandatory deterministic cut was found in this scope. Independent execution passed 4 new fault tests, 11 prior G2 regressions, and all 147 daemon tests. The coordinator/design owner retains the canonical gate decision after current-source integration checks.
Context: E1 work uses a separate snapshot. The primary daemon is frozen for this review. Accepted old-version native application evidence remains separate from current E2/D1/D6 integration and future E1 upgrade qualification.
Next: Coordinator/design owner integrates this verdict with the separately assigned workspace checks and retained native/application evidence. E1 adoption and G3 qualification retain their separate assignments.
Log:
- [2026-10-25] Resumed from this ledger. Added follow-up scope before further edits. Test execution is authorized for this review only; the original audit's no-test restriction remains historical.
- [2026-10-25] Verified author before/after manifests are identical and all 129 current inputs match. Fresh target was absent. Independent evidence: `/tmp/opencode/astra-g2-fault-review-20261025.njen9i5j/`. Read the actual worker, checked join, lifecycle executor, Objecter transport and nested fault assertions.
- [2026-10-25] Independent `cargo test -p rmds --locked g2_fault_ -- --nocapture`: 4 passed. `g2_r -- --nocapture`: 11 passed. Full `-- --nocapture`: 147 passed. Each exit is 0, with no failure or ignored test. These counts overlap. All use `target-astra-g2-fault-review`; four existing dead-code warnings remain.
- [2026-10-25] Traced all six join negatives to rank/shard consistency checks. Verified three Closing cuts, three partial-I/O/retry cases and four completion cuts. Rechecked all 129 inputs after execution/review: unchanged and identical to both author manifests. Compared 14 storage/executor/Objecter inputs with a6ae05a: all identical; production persistence code is also unchanged. Issued scoped ACCEPT. Only this report was edited in the source tree; Cargo output and review artifacts use the assigned target and scratch directory. No live action, signal, mount, cluster query, Git write, commit or subagent occurred.

### Verdict and tested identity

**A CLOSED; B CLOSED.** The new evidence closes the two integration gaps in the preserved audit below. The result rests on injected boundaries, durable reloads, data-state assertions and production-path inspection, together with the earlier layered evidence.

- Author evidence: `/tmp/opencode/sol-daemon-d6-g2-fault-20261025.9LwREj/`.
- Independent evidence: `/tmp/opencode/astra-g2-fault-review-20261025.njen9i5j/`. Contains commands, complete logs, exit files, provenance, before/after manifests, source-stability results and comparison with the accepted revision.
- HEAD read after review: `85a37c8d615992b4600d7467fdedd514fecfac03`. The tested package includes the frozen, uncommitted primary-tree changes; HEAD alone does not identify it.
- All **129** Rust-source/manifest inputs match both author manifests and both independent manifests. SHA256 of the literal manifest is `12392c30bcfc0b31a13355556b636e76fa311b5c37d2bb78d06eaa0602071ceb`.
- The author records aggregate fingerprint `3fc3c84314b2727b5ffe6b642ed06b6e9add5000636cbd15c1472c5222e0c115`. Its calculation is not in the retained artifacts, and it is not the literal manifest's SHA256. Acceptance uses the independently checked 129 entries and manifest digest above.
- Rust/Cargo **1.99.0**. The target was absent before execution. Test executable SHA256: `d8a4c959b9f26dd3b8923df74d9ad364a1fc6cd170637dde618550546420054b`. The 48 fsmap/privmon fixture hashes in `fixtures-after.sha256` are post-run records only.

All commands used `CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-astra-g2-fault-review`:

| Independent command | Result | Log |
|---|---|---|
| `cargo test -p rmds --locked g2_fault_ -- --nocapture` | 4 passed; exit 0 | `g2-fault.log` |
| `cargo test -p rmds --locked g2_r -- --nocapture` | 11 passed; exit 0 | `g2-regressions.log` |
| `cargo test -p rmds --locked -- --nocapture` | 147 passed; exit 0 | `daemon-full.log` |

Every run has zero failed/ignored tests. Counts overlap. Warnings concern existing `Config.cluster` and the three `spawn_*` compatibility functions in `rados.rs`. The full-suite result includes D6 tests; independent D6 protocol/timer acceptance belongs to its assigned reviewer.

### A. Interrupted Closing and checked joins — CLOSED

`crates/mds/src/persistence/fault_tests.rs:10-127` runs three distinct cuts. Its one writable file creates one cleanup job decision before the terminal receipt. Each cut holds a real worker event, aborts/joins that worker task, and starts a new owner on the same backend. `Worker::load` calls production `persistence::worker -> open`; `Worker::server` calls checked `Server::from_loaded` (`test_runtime.rs:90-168`). No positive Loaded state or successful completion is fabricated.

| Cut | Durable state at restart | Verified consequence |
|---|---|---|
| 0: hold `Closing` | Rank Closing; cleanup has not started | Same SessionKey and next-incarnation counter; original completed-request history survives. Restart creates the retained-bound recovery intent and terminal receipt. |
| 1: apply Closing, hold first `Committed` | RecoverSize intent is durable; terminal receipt is absent | Reload finds one job with the original bound. Resumed cleanup retains its exact ID/payload and creates one terminal receipt. |
| 2: apply first commit, hold second `Committed` | Terminal receipt is durable; rank remains Closing | Checked load queues Closed from that receipt. The backend append count does not increase. Cleanup is not repeated. |

The step boundary follows `engine/src/engine/step.rs:242-280`: a retained range creates its own decision, then a separate `closed_sessions` record. Only its commit removes history and emits the receipt (`engine/mod.rs:1215-1224`). Holding the returned event prevents the next daemon transition.

Each takeover confirms persisted peer nonce 42 was fenced before constructing the runtime. Production `persistence.rs:421-439` performs the fresh map/entry check and barrier raise before Loaded. Closing stays out of the Open gather. Final assertions require one exact `(SessionKey, cleanup_seq)`, no live session, no old peer output, one retained job and its original maximum bound. A checkpoint/trim and third-owner load then require empty replay records, rank Closed with that exact receipt sequence, the same checkpoint receipt, and an empty gather.

The positive Closed callback also checks `engine.session_cleanup_seq(key) == Some(seq)` before removing the key/gather (`server/runtime.rs:1010-1033`). Thus the test's exact persisted Closed sequence joins the production retirement check.

#### Six negative cases: actual rejection paths

`g2_fault_recovery_join_rejects_mismatches` first obtains a successful real-worker Loaded result, then changes only its rank-row vector. The underlying bootstrap/checkpoint/WAL fixture remains valid. Source tracing gives these guards:

| Fixture change | Rejecting check |
|---|---|
| Open fixture: remove rank rows | CREATE replay restores its completed-request key; `runtime.rs:292-298` rejects shard history without a live rank row. |
| Open fixture: label its row Closed without a shard receipt | The same live-history join rejects Closed plus surviving history. This exercises the inconsistency, rather than the later Closed-only branch at `:353-358`. |
| Open fixture: add a second live incarnation | `install_session_key` rejects `live incarnation mismatch` while joining the second row (`runtime.rs:318-320`; `engine/durable.rs:14-17`). |
| Closed fixture: label its row Open | Replayed receipt plus Open fails the receipt/rank join (`runtime.rs:300-306`). |
| Closed fixture: change cleanup sequence to tail + 1 | Receipt/rank sequence comparison fails at the same join. |
| Closed fixture: change incarnation by +100 | Range-owner binding selects the changed row; CREATE's recorded completion still carries the original key. Checked replay rejects `live incarnation mismatch` (`runtime.rs:255-281`; `engine/ops_ns.rs:418-433`; `engine/durable.rs:577-590`). |

These are consistency failures inside the intended checked-load path, not setup failures. All occur before runtime construction or command scheduling at `runtime.rs:365-408`. The tests assert `is_err`, not exact error text; the table records source-traced return sites. The retained RECLAIM-output assertion is not the main proof of load rejection.

### B. Lifecycle execution and completion — CLOSED

`crates/mds/src/daemon/tests/faults.rs:55-270` creates each intent through ordinary dispatched OPEN, namespace SETXATTR, CREATE, dirty-cap/range updates, truncate/unlink and close/fence operations. It commits the inherited `fault-data` namespace and uses real nonzero bounds. `install` replaces the offline fixture's seed server/channel with the actual worker-backed server/channel. `settle_real` consumes the actual worker receiver. Readiness follows the production phase and E2 structural path.

`DataIo` sits below the real Objecter (`test_backend/data.rs:33-60,103-187`). It decodes actual MOSDOp requests and returns MOSDOpReply results. Its second request fails with **EIO before mutation**. The first request has already changed or read the retained object set. The executor is production `Daemon::drive_jobs -> rados::execute_lifecycle`, followed by tagged `ControlEvent::Job` dispatch (`daemon.rs:663-714,1110-1139`).

| Partial-I/O case | Actual oracle in `g2_fault_lifecycle_partial_io_retry` (`:274-480`) |
|---|---|
| Purge | First object removed; second REMOVE fails. Restart keeps the partial object set and exact intent. Retry receives ENOENT for the first object, removes the second, and leaves only the unrelated-namespace sentinel. |
| Truncate | First TRIMTRUNC retains the first half-object bytes; second fails and retains its original 123 bytes. Retry preserves the sequence, leaves the first bytes exact and makes the second object empty. |
| RecoverSize | Descending STAT reads object 1, then object 0 fails. The daemon receives an error, retains the job/range and leaves size zero. Retry recovers the independently expected `object_size + 123` and mtime `(42,0)`, then clears the range only through completion commit. |

After each error, the complete in-process checkpoint equals its pre-attempt image, the job remains, failure count is one, and no storage completion transaction is pending. Across worker/runtime recreation, every checkpoint field is compared except the explicitly volatile cap-ID cursor. Data bytes, mtime and truncate sequence are compared across that restart. Pool/namespace trace checks and the unrelated sentinel exclude default-namespace substitution. A pre-structural `drive_jobs` emits no data request. Successful I/O still leaves the intent present until the completion is pumped and committed. The existing reviewed striped reference vectors remain exercised by the full daemon suite; these integration cases use the default single-stripe geometry.

#### Four completion cuts and late events

`g2_fault_lifecycle_completion_cuts_and_late_events` (`faults.rs:483-615`) runs the actual probe first, then supplies its result through production control dispatch:

| Cut | Durable reload assertion |
|---|---|
| Before completion admission/pump | Original job/range and size zero; unchanged covered sequence. |
| Completion WAL write fails | Backend fails the actual WAL compound; tagged Failed event causes daemon fail-stop. Reload retains the original job/range, size and covered sequence. |
| Completion WAL durable; callback withheld | A real Committed event is received but not applied. Reload retires the job, installs size 1234 and clears the range. Covered sequence advances by exactly one. |
| Callback delivered | Same durable result. A later duplicate carrying size 9999 produces neither a state change nor another pending storage command. |

Both durable cuts run normal clientless E2 readiness after reload and require no ready job. Successful synthetic mismatches use `gid + 100` and `operation + 1` while the current attempt is active. Both leave the complete checkpoint and active attempt unchanged. These are unequal-token tests, not captured previous-attempt results; the production filters use exact equality, so direction is immaterial. Duplicate current results before admission produce only one completion record. This covers the shared owner/attempt dispatch filter without a separate case for every job kind.

These lifecycle fixtures are clientless after cleanup. They directly prove durable-state retention and failure gating. Existing `failed_real_append_discards_reply_and_stops_engine_service` and `real_worker_delays_open_reply_and_cleanup_close_until_durable_completions`, rerun in the 147-test suite, supply the live-funnel output assertions. The latter also injects committed-but-unknown append response, keeps output withheld, reconciles the identical batch and asserts exactly one backend append. Earlier store tests supply absent/cancelled/changed-owner unknown outcomes.

### Gate reconciliation

1. **Written storage-fault requirements:** `design/workplan.md:36-42,99` and `design/persistence-contract.md:70,147-149,286-295,309` require these durable cuts, joins, retries and stale-result rules. A/B now close the original matrix's two partial rows. No additional mandatory deterministic G2 cut was identified.
2. **Layered/native evidence remains applicable:** all ten store manifest/source inputs, `rados.rs`, and Objecter's `objecter.rs`, `ops.rs`, `messages.rs` are byte-identical to accepted `a6ae05a`. The production portion of `persistence.rs` is also unchanged; its differences are test-module/fixture changes. The earlier S/D/E/O/I/N evidence below retains its stated scope. New tests supply the missing worker/executor joins. They do not turn deterministic backends into native OSD fault trials.
3. **Application evidence stays accepted:** `current/astra-g2-evidence.md` still establishes the old-version same-fs44/meta88/data89 clean restart, 17 acknowledged operations, 10 saved identities, 16 future nonreuse checks and final comparison. This review makes no fresh cluster observation.
4. **Symlink and mounted-client scope:** APP did not exercise positive symlink persistence. `minimum-feature-set.md:221` requires SYMLINK in P0, and `persistence-contract.md:112,124` requires preservation of every inode field. Neither specifies an additional native symlink fault cut for this A/B package. The explicit G2 application gate is `workplan.md:40`; its accepted scope remains as recorded. Same-wire-tid loss and mounted-client SIGKILL requirements remain G3 work (`workplan.md:42,56`; the first-milestone list in `persistence-contract.md:309`). They are not inferred from these four tests.
5. **Canonical decision:** the release owner supplies current-source combined workspace/build/format/lint evidence under `workplan.md:101`. The coordinator/design owner then updates the gate. This is scoped tests/fault-matrix acceptance, with accepted old-version application evidence preserved. E1 format adoption, daemon D5 and nonempty G3 finalization require their own reviewed implementation and qualification.

## Original source audit — preserved

Task: Independent focused G2 fault-coverage audit. Map each storage-fault requirement to actual test symbols and independent evidence. Identify the smallest remaining qualification package. Use committed a6ae05a daemon source. Do not inspect the active daemon diff. Sole writable file: current/astra-g2-fault-matrix.md. No source changes, tests, builds, live actions, commits, or subagents.
Status: COMPLETE. Most store fault requirements already have sufficient layered evidence. Two daemon integration test families remain: interrupted Closing recovery joins, and lifecycle executor retry/completion boundaries. No production defect is asserted. No tests or live checks executed.
Context: Audit revision is a6ae05a10bfefdcea8dae75cd29597c84a3fadcd. All daemon source reads use that commit. Nineteen relevant committed files match the accepted QA source archive byte for byte. Stable engine/store/osdc paths had no diff from that commit at inspection. Accepted test results are historical evidence, not fresh passes.
Next: Parent assigns the two test families below to the daemon owner after the active package is stable. A separate reviewer verifies the results. Native mounted-client cases stay with the assigned G3 QA lane.
Log:
- [2026-10-25] Created the audit ledger. All findings are pending source and report review.
- [2026-10-25] Read AGENTS/HANDOVER, the G2 workplan and persistence contract, store documentation, assignments, and independent store/engine/daemon/platform/application reports.
- [2026-10-25] Inspected exact test bodies and retained unit.log entries. Confirmed memory-backend fault coverage, not just test names. All daemon reads used git show/git grep at a6ae05a; no active daemon diff was inspected.
- [2026-10-25] Compared 19 audit source files in accepted-source.tar with a6ae05a: all match. Read retained 82-daemon and 41+5+4-store result groups. Ran no tests, builds, live actions, commits, or subagents.
- [2026-10-25] Completed gate accounting. The smallest remaining storage-fault package is deterministic daemon integration coverage. A new native checkpoint fault-injection seam is not required by the written G2 gate.

## Decision

**Do not schedule a new full live storage-fault matrix.** The accepted evidence already covers most listed requirements. Two specific integration gaps remain at the audited revision:

1. Reloading a durable **Closing** session before, during, and after shard cleanup.
2. Executing partially completed lifecycle I/O, retaining it across restart/failure, and handling late daemon job results through completion commit.

These are missing test evidence, not demonstrated source defects. Both can use deterministic backends and the real daemon/engine/store/Objecter paths. No store algorithm, schema, or engine API change is indicated.

`design/workplan.md:34-42,95-99` requires deterministic durable-boundary faults, native RADOS compound-guard evidence, and an unmodified-client acknowledged history. It does **not** require every deterministic fault to run again against a live OSD. The native compound and application prerequisites now have accepted evidence. Older reports' lists of unexercised native scenarios describe their scope; they do not make every scenario an additional G2 prerequisite.

This is storage-fault accounting. Positive symlink persistence and mounted-client/same-wire-tid scenarios retain their existing G3 assignments. This report does not declare the entire G2 or G3 gate passed.

## Evidence and revision

All source references below use **`a6ae05a10bfefdcea8dae75cd29597c84a3fadcd`**. Daemon source came only from that commit. I compared 19 relevant source/test files with `$I/accepted-source.tar`: all matched. This includes the store fault backend, daemon persistence/runtime/job code, and cited engine/Objecter tests. I also found the cited test results in `$I/unit.log`. These are retained results, not a new execution.

| ID | Accepted evidence |
|---|---|
| **S** | `current/sol-store-wal-review.md`: independent **50** store tests, 2026-10-22 correction revision; 41 unit + 5 checkpoint/schema + 4 legacy fixtures. `current/sol-store-state-review.md` accepts authority/schema at the earlier 40-test revision. Later pagination regressions close its P3. |
| **D** | `current/sol-daemon-store-review.md`: persistence review with **71** tests. `current/sol-daemon-review.md`, final correction section: **82** tests, including 11 focused corrections. |
| **E** | `current/sol-engine-state-review.md`: **42** focused durability/review regressions, plus separately recorded restore/retry probes. Other engine tests cited below appear in accepted combined QA. |
| **O** | `current/sol-platform-review.md`: independent **70** osdc tests + doctest and 24 monc tests. Includes real Objecter actor tests over a mock transport. |
| **I** | `$I=/tmp/opencode/sol-integrated-g2-20261022-2014/`. Independent retained-evidence acceptance in the daemon-store report: **524 passed, 0 failed, 2 ignored**, build/fmt/default clippy passed. `unit.log:969,1016,1027,1037` records the 82-daemon and 41+5+4-store groups. Publication preserved the tested Cargo bytes. |
| **N** | `current/sol-platform-review.md`, retained native evidence: `/tmp/opencode/sol-platform-live-20261022.YcfShl/`. Successful/rejected CMPXATTR and ASSERT_VER compounds preserve exact data/tail/version on rejection. Native exclusive creation, checked results, and full all-namespace listing passed. This was pool87, not fs44 recovery. |
| **APP** | `current/astra-g2-evidence.md`: `/tmp/opencode/sol-g2-resume-20261025-1736/`. Native same-fs44/meta88/data89 clean restart; 17 acknowledged operations, 10 saved IDs, cold comparison, 16 nonreused allocations, final comparison. Binary SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`. Final Ready/tail165/checkpoint15/coverage165/trim165 and seven Closed rows. |

**Covered** below means sufficient layered evidence for the stated G2 property. It does not mean the named unit test used a real OSD. In particular, `native_store_shape_*`, `*_native_pages`, and `real_worker_*` use memory storage. The worker tests do call the production store and daemon paths.

Symbol prefixes:

- `S::` = `crates/store/src/stable.rs::tests::`.
- `D::` = `crates/mds/src/persistence.rs::tests::`.
- `R::` = `crates/mds/src/server/runtime/tests.rs::`.
- `E::` = `crates/engine/tests/durability.rs::`.
- `ER::` = `crates/engine/tests/review_regressions.rs::`.
- `O::` = `crates/osdc/tests/objecter_mock.rs::`.

## Compact requirement matrix

| G2 requirement | Actual test symbols and observations | Accounting |
|---|---|---|
| Durable append before callback/output | `D::real_worker_delays_open_reply_and_cleanup_close_until_durable_completions`; `journal::tests::reservation_recovered_sequence_and_completion_are_not_submission` in `crates/mds/src/journal.rs`; `D::failed_real_append_discards_reply_and_stops_engine_service`. The worker test holds the returned event: storage has committed, but the link is still empty. | **Covered: D/I + N/APP.** Native Objecter requests ONDISK; checked result/cardinality handling is independently accepted. This is not host-power-loss evidence. |
| Unknown append: committed, absent, cancellation, identical retry | `S::unknown_success_freezes_and_reconciles_identical_batch`; `S::unknown_uncommitted_append_retries_exact_bytes_and_owner_loss_stays_fenced`; `S::caller_cancellation_after_durable_append_retains_the_uncertain_batch`; `S::batch_identity_rejects_different_same_length_payload_after_completion`. The D worker test injects committed-but-lost reply and asserts exactly one durable append. | **Covered: S/D/I + N.** No replacement sequence/batch and no premature reply. Real network-induced uncertainty remains unexercised; it is not a missing pure-logic test. |
| Short pagination, gaps, corruption, bounds | `S::wal_recovery_accepts_count_short_native_pages`; `S::wal_recovery_accepts_byte_short_native_pages`; `S::wal_short_pages_reject_missing_required_keys_and_owner_change`; `S::wal_pages_reject_nonprogress_and_contradictory_continuation`; `S::wal_short_pages_preserve_total_key_byte_and_value_limits`; `S::missing_or_corrupt_required_records_never_shorten_replay`. | **Covered: S/I.** Exact continuation requests, missing middle/final keys, empty/repeated pages, false end, frozen-tail contradiction, owner change and byte/key/value limits are asserted. Native page semantics were checked against Ceph during independent review. No live OSD-limit change is required. |
| Stale append/publish/trim/storage completion | `S::stale_owner_cannot_append_publish_or_trim`; `S::ownership_change_between_inspection_and_write_is_rejected_atomically`; D worker test injects a stale-owner event; `R::g2_r3_open_reservation_survives_stale_owner_but_releases_on_failure_or_cancel`. `runtime.rs:733-739` checks owner/op before every event variant. | **Covered for metadata/storage events: S/D/I + N.** Existing stale-event tests use failed events; the shared pre-dispatch owner/op filter also protects success variants. Repeating every storage event variant is not a separate requirement. Lifecycle job events use a different path: see B below. |
| Interrupted bootstrap and uncertain format CAS | `S::bootstrap_crashes_resume_exact_plan_and_restore_without_old_wal` cuts six dependency-write boundaries; `S::preparing_takeover_requires_each_object_guard_and_rejects_old_preparer`; `S::uncertain_format_create_and_acquire_reconcile_exact_plan_and_writer`. `S::bootstrap_late_foreign_namespace_after_map_restart_never_writes_format`, `S::bootstrap_map_change_restarts_the_complete_successful_scan_before_writing`, and `S::bootstrap_repeated_listing_cursor_is_not_scan_completion` cover scan failure. | **Covered: S/I + N + accepted native fresh bootstrap in I.** Lost Ready response is also exercised in the takeover test. Preserve the original plan/root/range. No new filesystem or native bootstrap-crash series is needed for this logic. |
| Independent shard/session acquisition | `S::partial_shard_session_acquisition_preserves_each_independent_authority`; `S::reverse_partial_acquisition_fences_both_actual_writers_without_reset`; `S::acquisition_preserves_tail_and_reconciles_lost_response`; `D::partial_shard_acquisition_resumes_with_next_owner`; `D::bootstrap_restart_preserves_identity_and_fences_before_acquisition`. | **Covered: S/D/I + N/APP.** Both partial directions, preserved counters/rows, lost acquisition replies and barrier-before-first-write have evidence. Native destructive-worker fencing during takeover belongs to G3. |
| Interrupted Open/Closing/cleanup/Closed | `S::rank_open_close_unknown_outcomes_preserve_incarnations_and_tombstones`; D worker ordering test; `D::paused_normal_queue_does_not_spin_and_nonempty_restart_holds_gather`; `E::cleanup_wal_receipt_replays_without_resurrecting_old_incarnation`; `ER::checkpoint_only_cleanup_records_recovery_before_its_receipt`. | **Partial: A below.** Repository uncertainty, engine interruption, restored Open gather and native Closed recovery are covered. No existing test feeds interrupted **Closing** states through `open -> Server::from_loaded -> pump -> worker`. The two recovered Closing branches at `runtime.rs:287-312` lack that integration evidence. |
| Materialized checkpoint-only restore; old WAL removed | `S::native_store_shape_restores_materialized_engine_after_every_wal_key_is_removed`; `E::materialized_checkpoint_survives_all_history_removal`; `crates/store/tests/checkpoint.rs::materialized_checkpoint_roundtrip_preserves_all_fields`; D worker checkpoint/restart test. | **Covered: S/E/D/I + APP.** Typed state, histories/watermarks, delegation/linkage and exact SessionKey survive. Final native empty WAL corroborates trim; it does not alone prove the trial's exact replay count. |
| Partial immutable writes; unpublished orphans | `S::partial_checkpoint_write_and_uncertain_publication_trim_preserve_history`; `S::crash_before_publication_keeps_old_checkpoint_and_tail`. At `stable.rs:2327-2332`, the fault permits the state object write, then fails the manifest write. Reload still selects the old checkpoint and required WAL. | **Covered: S/I + N/APP.** Retry verifies existing identical state bytes. The real single-object EXCL-create + WRITEFULL compound is the atomic boundary. A byte-torn successful compound is not an additional G2 model; corruption checks cover damaged persisted bytes. |
| Publication and bounded trim | `S::checkpoint_publication_trim_and_checkpoint_only_load`; `S::partial_checkpoint_write_and_uncertain_publication_trim_preserve_history`; stale-owner/race tests above. Unknown publication and unknown trim occur **after** mutation, then reconcile. Separate one-key trims leave a safe redundant prefix. | **Covered: S/I + N/APP.** Exact descriptor/owner/trim-cursor guards share accepted native compound semantics. Old generations remain. A fresh native crash at each boundary adds fidelity, not a missing deterministic requirement. |
| Retained writer ranges and lifecycle work | `E::lost_writer_range_and_job_survive_crash_until_completion_commit`; `E::purge_intent_retains_all_pools_after_inode_retirement`; `E::stale_probe_result_cannot_retire_or_change_metadata`; `ER::gather_discovers_range_free_strays_and_replays_purge_before_callback`; `ER::restored_stray_waits_for_truncate_before_replacing_recovery_with_purge`; `R::g2_r4_busy_cap_does_not_hide_queued_lifecycle_completion`. | **State logic covered: E/D/I. Executor integration open: B below.** These tests do not execute partial data I/O through `rados::execute_lifecycle`, nor inject old-owner/old-attempt `ControlEvent::Job` results. |
| Missing/foreign/corrupt/unsupported identity | `D::missing_foreign_corrupt_and_failed_fence_never_create_namespace`; `S::framing_protects_every_byte_and_rejects_other_identity`; `S::descriptor_substitution_and_missing_checkpoint_fail`; `S::ready_marker_never_substitutes_for_missing_dependencies`; `S::rank_session_row_cannot_substitute_another_valid_identity`; `S::unsupported_or_unencodable_body_never_receives_append_success`. | **Covered: S/D/I.** Byte-by-byte protected-frame damage, wrong identity, missing dependency and unsupported schema fail explicitly. No deliberate corruption of preserved native pools is needed. Rank/shard join-specific negatives can share A's fixture. |
| Capacity and timeouts | `S::pages_and_admission_are_bounded`; `S::orphan_checkpoint_objects_apply_backlog_pressure_across_owner_epochs`; `D::retained_checkpoint_generation_capacity_pauses_without_retry_spin`; `R::g2_r3_interleaved_open_completions_keep_live_plus_reserved_within_limit`; `O::laggy_ops_ping_and_time_out`; `O::pool_listing_has_its_own_deadline_and_prunes_the_timed_out_op`; `O::checked_map_waits_report_timeout_and_shutdown`. | **Covered within stated budgets: S/D/O/I.** Accepted production review verifies positive 30 s actor timeout, serialized storage work and one initial attempt plus eight retries. A store attempt can contain multiple native operations; this is not a 270 s whole-command promise. Active messenger outbound memory remains an explicit separate limit. |
| Lost client reply / exactly-once mutation | `crates/engine/tests/posix.rs::completed_requests_answer_retries`; `crates/engine/tests/journal.rs::replayed_engine_answers_retries_from_the_table`; E checkpoint/history tests; the independent restored O_EXCL retry probe in `sol-engine-state-review.md:104`. | **Logical deduplication covered; wire crash/retry remains G3.** The journal test uses the legacy replay helper; do not describe it as native checked startup. Store unknown-batch tests are not same-client-tid tests. The accepted application trial explicitly does not prove same-wire-tid reply loss. |

The four tests in `crates/store/tests/wal.rs` use legacy segment framing. Their torn-tail/valid-prefix behavior is **not** evidence for production stable-WAL gap recovery. The stable tests above provide that evidence.

## Smallest actionable remaining package

Proposed symbols below are **new tests**, not existing passes. Parent assigns them to the **daemon owner**, after the active G3 package is stable or in an assigned isolated snapshot. A separate reviewer owns acceptance. Sole QA retains any later live operations.

### A. Interrupted Closing recovery join

Add `g2_fault_closing_restart_cuts` using the real persistence worker and atomic `MemoryBackend`. Start with a durable session that owns a retained writer range and request history. Cut at three distinct durable states:

1. Rank Closing is durable; no cleanup WAL record exists.
2. A cleanup range/job decision is durable; the terminal cleanup receipt is absent.
3. The terminal receipt is durable; the rank row is still Closing because Closed has not committed.

Drop the worker/runtime and reconstruct with the same backend, new owner, actual `open`, checked `Server::from_loaded`, and production pump. Do not fabricate Loaded state or successful store completions for the positive cases. A test-only commit-and-pause hook can select a named operation/sequence and withhold its return. The existing command/event channels can hold the next boundary.

Required observations:

- Fence the persisted Closing peer addresses before resumed cleanup. Keep its key out of the Open gather.
- Preserve SessionKey, incarnation counter, histories until their allowed cleanup, original range bounds and job IDs. Do not duplicate an already committed intent.
- Reach one terminal receipt and rank Closed with its exact sequence. Case 3 must not append another cleanup transaction.
- Restore again from Closed. Keep the tombstone and receipt; install no live session. Emit no old OPEN/CLOSE success while callbacks remain withheld.

Add a compact companion table, `g2_fault_recovery_join_rejects_mismatches`, against the same fixture: shard history without a rank row, Open with a cleanup receipt, Closed with a missing/wrong receipt, and two live incarnations for one client. Require checked-load failure before service/output/data work. These exercise the actual join rather than repeat codec corruption tests.

**Source change:** daemon test code and, if needed, a test-only named pause in `test_backend.rs`. No production change is established. Open-response loss and completed Closed restart already have layered coverage; a new native trial for every row is unnecessary.

### B. Lifecycle executor and completion boundaries

Add `g2_fault_lifecycle_partial_io_retry`. Use the real `rados::execute_lifecycle` and Objecter with a deterministic `OsdTransport`/`OsdConnection` implementation. The existing public transport boundary supports this; no new production executor abstraction is needed.

- For a multi-object purge, let one removal succeed and fail the next. Preserve partial data state across metadata-worker/runtime recreation. Retry the same durable intent. Assert that ENOENT from an already removed object permits purge progress; other errors retain the intent. Check the retained pool/namespace and final object set.
- For a multi-object truncate, let one object succeed and interrupt the next. Retry with the same truncate sequence and object-local sizes. Check final bytes/sizes. Reuse accepted striping vectors; do not invent a second mapping formula.
- For a RecoverSize probe, fail one STAT. Require no successful result/retirement from the partial scan. Retry to the independent expected size/mtime.

Add `g2_fault_lifecycle_completion_cuts_and_late_events` through `Daemon::on_control_event`, real engine steps and the persistence worker:

- Complete data I/O, then stop before its completion WAL is durable. Reload: retain the intent and writer bounds. A failed completion append must release no recovered attributes or success.
- Commit completion WAL but withhold its callback. Reload: apply the recovered metadata/retirement once; do not reschedule a retired job.
- While a current job attempt is pending, inject successful results with an old owner and an old attempt ID. Also deliver a duplicate current result. Require no extra completion record, metadata change or retirement. One representative job suffices for this shared dispatch filter.
- Confirm that service/recovery gating prevents data work before readiness. Successful retry must eventually retire the intent through a durable completion, not merely return an I/O result.

**Why this is a real gap:** at a6ae05a, only `drive_jobs` calls `execute_lifecycle`; no test calls it directly or injects `ControlEvent::Job`. The rados tests exercise mapping/error/preflight helpers. The engine tests exercise state retention. G2-R4 supplies a successful result directly to the runtime. None supplies a partial executor fault followed by daemon retry and durable retirement. `sol-rados-review.md:18` and `sol-daemon-store-review.md:97` state these limits.

**Source change:** daemon test support only unless these tests reveal a defect. The actual Objecter over a controlled transport supplies deterministic completion/error behavior. Native primitive and mapping evidence need not be repeated for every cut.

### Acceptance and stopping rule

The implementer and independent reviewer should run the four new `g2_fault_` tests on a pinned package, then the normal daemon suite. Record source revision, command, case names, injected boundary, result and artifact path. Tests must compare durable rows/WAL/jobs/data and emitted output, not only a final namespace listing.

When A and B pass independent review, the listed **G2 storage-fault logic** can be closed against the existing native primitive and application evidence. Do not require an extra native matrix solely because the earlier reports retained a broad “fault matrix open” sentence. If a new test finds a defect, qualify the corrected boundary and its affected regressions.

## Native-only claims and a possible seam

These stronger claims remain unobserved. They are not all additional G2 blockers:

| Claim | Execution needed to make that claim |
|---|---|
| Real OSD reply suppression leads to Objecter timeout and store reconciliation | A native peer plus reply suppression before Objecter consumes the selected reply. Existing actor timeout tests use a mock transport. A wrapper that returns UnknownOutcome **after** native success proves native reconciliation, but not the actor timeout path. |
| A real daemon process dies between immutable state, manifest, pointer publication and trim | Boundary markers and process termination at those cuts. Current memory tests prove the state machine; APP proves a clean native restart, not those exact crashes. This native series is not required to repeat the already covered pure logic. |
| Partially executed native data jobs resume after takeover; mounted retries retain one mutation | The separately assigned G3 daemon/client crash trials, with job/attempt and wire-tid observables. Reuse that evidence for overlapping G2 subclaims; do not run a duplicate mounted-client lane. |
| OSD/host power-loss durability | Disk-backed G7 qualification. Memstore MDS-only trials cannot establish it. |

**No native fault-injection seam is needed for A/B.** If the parent explicitly requests the stronger native checkpoint/append claim, the narrow seam is an opt-in wrapper around the existing `Backend::operate`, with native `RadosBackend` underneath. Select one exact object, operation kind and owner/sequence/generation. After checked native success, record the boundary and withhold the return, or return one UnknownOutcome. Forward listing and all unrelated operations unchanged. This can select append, state write, manifest write, pointer publication, or one trim batch. A separate test harness can use the existing generic worker; a real daemon-process trial needs opt-in daemon wiring.

Record identity, old/new owner, operation ID, tail/batch hash, descriptor/trim cursor and immutable-object hashes before restart. After restart, compare the same state and assert the expected old-checkpoint-plus-WAL or new-checkpoint-plus-uncovered-WAL selection. QA alone controls any signal and preserves fs44/meta88/data89 and both acknowledged namespaces.

For an actual native actor-timeout claim, the hook must instead withhold the selected `MOSDOpReply` at the `MsgrTransport` delivery to `ConnSink`, including duplicate replies for that request until timeout. This is a different, more invasive seam. Do not label post-return error injection as transport loss. The main instrumentation risk is selecting the wrong boundary or changing event ordering and then claiming a crash state that never existed. Keep hooks out of the default production build and independently review/hash the instrumented package before a live run.

## Audit limits

Only this report was written. No source changes, tests, builds, live actions, commits or subagents occurred. No active daemon source or unfinished daemon diff was read. New G3 code and its eventual test results need their own review; this matrix does not pre-accept them.
