# astra-g3-fault-controller-review
Task: NEW independent review of frozen G3 fault-v1 OFFLINE core. Own only this new primary report and external review artifacts under /tmp/opencode. Verify the frozen inputs, run the 68 focused tests normally and optimized, and drive targeted failure probes through the public controller. Return scoped offline-core acceptance or ranked findings. No source publication is authorized.
Status: COMPLETE — CHANGES REQUIRED for the frozen offline core. Five P1 findings and one P2 finding remain open. The 68 focused tests pass in both Python modes. Independent public-API probes reproduce final-action races, incomplete evidence joins, execution before BindAck, stale evidence acceptance and saved-descriptor guard gaps. All 218 initial input pins and three additional source-reference pins remain unchanged. No offline-core acceptance or native qualification is granted.
Context: Candidate root is /tmp/opencode/sol-g3-fault-controller-20261025.2_nqiu7r. The candidate has 11 frozen paths. The accepted interface is current/astra-g3-fault-interface.md. Published application tools and all prior reports are preserved. No code/test/design/Git writes, live queries, actual signals, mounts, process launches, capture or subagents are allowed. Test guards must be installed before import/discovery. Disposable local worker file I/O is allowed.
Next: Parent assigns corrections for G3-FC1 through G3-FC6. Preserve this frozen candidate and all failure artifacts. Re-review the corrected offline candidate with these exact public-API reproductions. Native providers, launcher and unfinished case assessment remain separate work.
Log:
- [2026-10-26] Completed review. All 218 initial pins and three additional source-reference pins match. All nine candidate Python files parse; four runtime modules have no assert; all 11 candidate files have no trailing whitespace. Prior application review remains SHA256 1f8a665769425004ae63cf8fd832f3933be32add256623cd804f8b9c1635aa2d, mode0664. Recorded five P1 and one P2 findings with public-path reproductions and minimal corrections. Only this new primary report and external reviewer artifacts changed.
- [2026-10-26] Refined two A1 probes to use RENAME destination/source roles from the frozen interface. In each mode, the correct control passes and both isolated rejection expectations fail, with zero harness errors. Initial probe scripts/logs remain intact. The refined results overlap the earlier two targets.
- [2026-10-26] Public-controller probes completed in both modes: 16 tests, 14 failed rejection expectations, two passing controls, zero harness errors. Supplemental probes completed in both modes: eight tests, seven failed rejection expectations, one passing parser matrix, zero harness errors. Failures are retained as failures. Complete false-positive series retain native_fault_case_pass=false and full_g3_qualified=false. The CAPS matrix accepted five versions with nonempty blobs and rejected all 1,402 truncated prefixes plus five trailing suffixes per mode. No candidate source changed.
- [2026-10-26] Verified all 11 candidate bytes/modes and exact archive, 14 author preservation pins, 31 author artifacts, old binary and 13 original plus 139 retry evidence files. The reviewer baseline contains 218 input pins. Both author full logs contain all 166 published baseline identities and the 68 new identities. The initial optimized timeout remains incomplete. The first reviewer provenance attempt rejected a log with stdout after its OK line; corrected only the reviewer parser, then verification passed.
- [2026-10-26] Independent focused runs: normal 68/0 failures/0 errors/0 skips; optimized 68/0/0/0. Signal and process-launch guards precede import/discovery. External logs are under /tmp/opencode/astra-g3-fault-controller-review-20261026. No full-suite rerun was needed.
- [2026-10-26] Created the sole writable primary ledger before review edits. The earlier application-tooling review remains unchanged.

## Verdict

**CHANGES REQUIRED for the frozen G3 fault-v1 offline core.** Findings G3-FC1 through G3-FC6 concern implemented APIs. The missing native providers and unfinished cases are separate scope limits.

The independent 68-test suite passes in both Python modes. Targeted rejection probes still produce `OFFLINE_VALIDATED` from malformed conversations. Those results retain `native_fault_case_pass=false` and `full_g3_qualified=false`. They are false offline successes, not native PASS claims.

| Finding | Priority | Required correction |
|---|---|---|
| G3-FC1 | P1 | Recheck terminal state and ownership after signal-intent persistence. Preserve actual EOF/action order. |
| G3-FC2 | P1 | Join the selected proof, retry input, original proof, output and final source by exact identity. |
| G3-FC3 | P1 | Reject execution of a held discovery input before its valid persisted BindAck. |
| G3-FC4 | P1 | Join original calls and competitor receipts to the actual peer operations and ordered outcomes. |
| G3-FC5 | P1 | Validate persisted transcript bytes and retain all evidence checks through final result creation. |
| G3-FC6 | P2 | Recheck saved descriptor/path guards after durable intent and before the raw mutation. |

## G3-FC1 — final action can follow an already observed failure

**Locations:** `scripts/g3_fault_trial.py:795-812`; `scripts/g3_fault_protocol.py:602-620`.

`seal_and_crash()` checks ownership and `BufferedChannel.check_terminal()` before writing the signal-intent artifact. That write performs blocking persistence. After it returns, the controller checks only the clock before calling the action. EOF, an event, a reader error or an ownership change observed during this interval does not prevent the action.

The probes wrap the real signal-intent writer. They let it write/fsync, introduce one fault, and wait until the actual reader records that fault. The action remains a producer double.

| Probe in `probes.py` | Actual result in both modes |
|---|---|
| `test_eof_during_intent_fsync_prevents_action` | The action sees `channel.ended=true`. `seal_and_crash()` sets `closed_expected=true`. A complete A0 series writes `OFFLINE_VALIDATED`. |
| `test_reader_error_during_intent_fsync_prevents_action` | The reader already holds the overlong-frame error. The action runs. Only the subsequent drain raises the error. |
| `test_queued_event_during_intent_fsync_prevents_action` | One post-seal event is already queued. The action runs. Only the subsequent drain rejects it. |
| `test_ownership_change_during_intent_fsync_prevents_action` | The owned-target double is invalid before the action. The action runs and the complete series passes. |

The EOF case violates interface section 4's explicit rule: EOF before the intended signal fails the case. A later injected `-SIGKILL` wait result cannot establish when EOF occurred.

**Minimal correction:** Complete signal-intent persistence first. Revalidate evidence and ownership, then terminal state and deadline at the action boundary. Synchronize reader terminal observation with that transition. Preserve pre-action versus post-action EOF explicitly. Any pre-action failure must prevent the action and latch failure. Keep the normal-stop path separate.

## G3-FC2 — identities and proof references do not form one evidence chain

**Locations:** `scripts/g3_fault_peer.py:39-53,301-336`; `scripts/g3_fault_trial.py:317-329,438-442,781-793,948-982`.

`Proof` retains a key and attachment but no selected tid, opcode/domain, input fingerprint or proof interval. `seal_and_crash()` therefore cannot verify that the proof describes the selected request. History decisions require only a nonnull `original_proof`. Recovery selects inputs and accepted outputs independently. It does not resolve each output's input and attachment against the selected resend.

Each following mutation reaches a complete A0 `OFFLINE_VALIDATED` result in both modes:

| Probe | Sole changed relationship |
|---|---|
| `probes.py::test_wrong_original_proof_rejected` | Replace the recovery Decision's original proof with 64 `f` characters. It differs from the retained Hit prefix and Hello history proof. |
| `probes.py::test_changed_daemon_resend_path_rejected` | Change only the daemon resend Input's raw-path fingerprint to `other-path`. Original and peer resend paths stay correct. |
| `probes.py::test_dead_attachment_output_rejected` | Put the dead process's attachment on the accepted retry output. The new Input uses the fresh attachment. |
| `probes.py::test_unknown_output_input_rejected` | Set accepted output `tag.input=88888`. No such Input exists. |
| `supplemental.py::test_unrelated_tid_proof_cannot_authorize_selected_cut` | Select tid 100, but authorize its crash with a valid persisted pending-call proof for tid 999 on the same key/attachment. Use tid-100 evidence during `OfflineSeries.add()`. The seal and final source digests differ, but the series passes. |

The last probe records selected tid 100, seal-proof tid 999, both source digests and the action invocation. It does not forge a native provider or bypass the public proof constructor.

**Minimal correction:** Give the internal proof a checked selected identity and evidence-prefix scope. Compare that scope at bind/crash use. Resolve `Decision.original_proof` against the exact retained history probe. Join the fresh Input, decision, accepted output, attachment and independent peer record. Compare both raw paths and the relevant flag namespaces. Resolve every nonnull reference. Require the final evidence to retain the exact sealed source prefix or an explicitly verified extension of it. A different whole-source hash alone is not a continuity proof.

These are consumer/internal-proof corrections. They do not require a change to the frozen daemon wire protocol. Until corrected, a future truthful provider cannot rely on the consumer to reject misjoined observations.

## G3-FC3 — discovery executes before BindAck

**Locations:** `scripts/g3_fault_trial.py:232-258,269-284,425-437`.

On SessionBound, the consumer immediately installs the key and attachment. Journal validation checks the input identity and journal stage order, but does not check whether that discovery input remains held.

`probes.py::test_discovery_executes_before_bind_ack_rejected` does this through Controller:

1. Start with an unresolved reserved slot.
2. Install an exact OwnedPath MKDIR discovery and receive SessionBound.
3. Before sending any BindAck, emit Encoded, WorkerEnqueued, AppendStarted, StoreConfirmed, EngineCommitted and DurableInode for that held input.
4. The consumer accepts the complete execution chain.
5. Bind normally, run a selected A0 crash/restart, then close normally.
6. `OfflineSeries.finish()` writes `OFFLINE_VALIDATED` in both modes.

**Minimal correction:** Track execution eligibility for each held discovery input. Reject its decision/journal/effect/output execution before the exact proof and persisted BindAck permit release. Keep other sessions runnable. Preserve the explicit UuidOpen exception: durable OPEN may execute before binding, but its typed output stays held. Apply the pre-execution rule to OwnedPath and Reset, as interface section 4.4 requires.

## G3-FC4 — original call and competitor evidence can contradict the case

**Locations:** `scripts/g3_fault_peer.py:301-365`; `scripts/g3_fault_trial.py:581-595,981-982`.

The pending-call check accepts any one Enter on the selected slot. It does not join that call's action/arguments to the selected peer request. Recovery checks that Enter and Exit agree with each other, but does not order the return after the corresponding response and processing. Competitor proof joins IDs and the worker's return, but omits the worker-argument-to-wire-path join.

**Public-path reproductions:**

- `probes.py::test_swapped_call_action_rejected`: label the original Enter/Exit as Unlink while the selected operation remains CREATE/Open. The complete A0 series passes.
- `probes.py::test_application_return_before_peer_processing_rejected`: move the original return to time 9,999,999. The resend is at 10,000,000, response at 10,000,040 and processing at 10,000,050. The return is after the crash, so the pre-kill test passes. The complete A0 series also passes despite the reversed causal order.
- `joins_refined.py::test_refined_worker_names_disagree_with_wire_rejected`: the real second worker renames `selected` to `moved`. Daemon and peer evidence instead describe `unrelated` to `elsewhere`, with the same IDs. The complete A1 series passes.
- `joins_refined.py::test_refined_successful_original_after_only_eintr_reply_rejected`: retain the capless SAFE `-4` response and processing result, but change only the original Open Exit from `-1/errno=4` to successful `0/errno=0`. The complete A1 series passes. There is no preceding selected reply or observed replacement call that explains this outcome.

The refined A1 control uses the contract's actual RENAME roles: Path is destination `moved`; Path2 is source `selected`. It passes with the matching raw worker receipt and original EINTR return. Both single-fault variants still pass the implementation and fail the reviewer's rejection expectation. The initial A1 probes used the opposite path roles; their logs remain intact. The refined probes provide the isolated evidence for these two findings.

The original syscall result and the wire retry result remain distinct facts. Do not synthesize a syscall return from a wire errno. The defect is acceptance without a checked call/profile relationship or causal chain. The local C++ `Client::handle_client_reply` and `Client::make_request` source confirms the first-reply/result distinction; it is source context, not installed-peer qualification.

**Minimal correction:** Join the selected original worker/trace identity, action and argument/FD anchors to the independently decoded request. Join competitor paths through an explicit checked representation mapping, including RENAME's two roles. Require entry/send/response/processing/return order consistent with the provider profile. Validate the original result against that complete observed call, including the no-userspace-retry rule. Missing joins must fail the implemented offline assessment instead of passing on matching numeric IDs.

## G3-FC5 — durable evidence can change while cached validation still passes

**Locations:** `scripts/g3_fault_protocol.py:489-524`; `scripts/g3_fault_trial.py:983-995`.

There are two related gaps:

1. `Transcript.persist()` checks the path's inode but does not recompute the framed prefix hash from the bytes it fsyncs. `records()` parses bytes without comparing them to the cached digest.
2. `OfflineSeries` retains consumer objects but not peer/source validation handles. `finish()` rechecks filesystem oracles and cached closure/failure flags. It does not revalidate earlier transcript identities/bytes or peer/worker source pins.

**Reproductions, both modes:**

| Probe | Observed result |
|---|---|
| `supplemental.py::test_same_inode_mutation_before_persist_rejected` | Change the earlier Input path hash in-place, preserving frame sizes and inode. Controller seals, persists and accepts the injected death. Its persisted digest is `fa93799391cf9a902f32f7eec8457693de4ea0c20ace00bfc92667c7610af04f`; the actual disk prefix hashes to `fa341a0ac75b0b1f85ce847b01dabc4c879e51e67f7cf6ea387915633ba0d3bf`. |
| `probes.py::test_changed_evidence_after_add_rejected` | Replace a validated peer source with `{}` after `add()`. `finish()` writes `OFFLINE_VALIDATED`. |
| `supplemental.py::test_truncated_old_transcript_before_finish_rejected` | Truncate the old closed process's transcript to zero bytes after `add()`. The series passes. |
| `supplemental.py::test_rotated_old_transcript_before_finish_rejected` | Rename the old transcript and recreate its pathname with identical bytes. The final result bypasses the existing inode-identity check and passes. |
| `supplemental.py::test_symlinked_old_source_before_finish_rejected` | Replace the old source pathname with a symlink to its retained bytes. The final result bypasses the existing no-symlink check and passes. |

**Minimal correction:** Verify exact framed length, sequence and cached prefix hash against the retained file before issuing a persistence receipt. Retain source/transcript/worker validation references for every round. Recheck them before later use and before final result creation. Preserve inode/no-symlink checks. Latch any validation failure and leave no successful result. This is consistency checking of already required pins; it adds no signature or speculative authenticity requirement.

## G3-FC6 — durable intent separates saved-state checks from mutation

**Location:** `scripts/g3_fault_client.py:114-145`.

Saved-descriptor and O_TRUNC pathname checks run before `save(Intent)`. After that blocking persistence, the worker rechecks only mount/root identity. It does not recheck saved data or the pathname's selected inode.

- `supplemental.py::test_saved_descriptor_change_during_intent_is_not_healed`: retain an unlinked FD containing `saved`. During the real intent-persistence wrapper, truncate that FD to zero bytes. The guarded `Worker.call('Write', ..., saved=...)` writes `saved` back and reports success. The corruption is hidden.
- `supplemental.py::test_truncate_path_change_during_intent_is_rejected`: validate an original FD/path, then replace the pathname during intent persistence. The real raw Open with O_TRUNC truncates the replacement inode and reports success. The original FD still refers to the old inode.

Both probes operate only on disposable local files. Both fail their rejection expectations in each Python mode. The O_TRUNC finding concerns the implemented Worker API; it does not claim A4 case assessment exists.

**Minimal correction:** Recheck saved descriptor data/identity and the O_TRUNC target after durable intent, immediately before the syscall. Fail without repair if they changed. Define the caller's exclusive mutation interval for the final pathname check/action pair. A pre-intent check is insufficient across blocking I/O. Keep the raw syscall and its actual return intact.

## Verification and artifacts

Reviewer artifacts: `/tmp/opencode/astra-g3-fault-controller-review-20261026/`.

| Check | Normal | Optimized |
|---|---|---|
| Frozen focused suite, `review.py tests` | 68 passed, 0 failures/errors/skips; 3.754 s | 68 passed, 0 failures/errors/skips; 3.768 s |
| Public-API rejection probes, `probes.py` | 16 tests; 14 failures, 0 errors; two controls pass | Same counts |
| Supplemental probes, `supplemental.py` | Eight tests; seven failures, 0 errors; parser matrix passes | Same counts |
| Isolated corrected-role A1 probes, `joins_refined.py` | Three tests; two failures, 0 errors; positive control passes | Same counts |

The last three rows are deliberately failing rejection expectations. They are not passing product tests. The refined row overlaps two initial probe targets; do not add these counts as independent coverage.

Exact commands executed from `/mnt/workspace/rust-mds`:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/review.py pin
PYTHONDONTWRITEBYTECODE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/review.py tests
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/review.py tests
PYTHONDONTWRITEBYTECODE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/probes.py
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/probes.py
PYTHONDONTWRITEBYTECODE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/supplemental.py
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/supplemental.py
PYTHONDONTWRITEBYTECODE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/joins_refined.py
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/joins_refined.py
```

Each test runner installs signal and process-launch guards before candidate import/discovery. They cover `os.kill`, `os.killpg`, `signal.pidfd_send_signal`, Popen signal methods, `subprocess.Popen`, `os.system`, `os.fork` and both posix_spawn variants. Tests use threads, injected producer actions, local sockets and disposable file/FD I/O. The author runner was read but not executed. No author artifact was used as a writable output.

Logs and JSON summaries use the names `normal`, `optimized`, `probes-normal`, `probes-optimized`, `supplemental-normal`, `supplemental-optimized`, `refined-normal` and `refined-optimized`. Per-probe directories retain transcripts, receipts, malformed sources and any false-positive `result.json`. The scripts create outputs exclusively. For a repeat, copy the four reviewer scripts to a new directory under `/tmp/opencode`; their output root follows `review.py`. Preserve the original artifacts.

## Checks that passed and their limits

- The independently encoded producer supplies real framed bytes without using the consumer encoder/schema as its event oracle. Focused tests cover strict numeric types, unknown/duplicate fields, UTF-8/JSON errors, length-first framing, sequence/nonce/correlation errors and persisted milestone receipts.
- Snapshot checks cover all 696 items, eight reservations, 64-address unions, typed absence, alias consistency, complete writer ranges and pre-resend versus later baseline purpose. Armed and startup snapshot persistence failures block the corresponding control commands. These checks do not close G3-FC3.
- B1 tests distinguish encoding from worker handoff, append start and checked durability. B2 tests reject accepted exact/greater covering ACKs even with changed cap ID/dirty bits. Greater-suppression counters require a complete matching Coverage record. Modeled B1 and B2 each complete two restarts in the focused suite.
- The independent reader continues during controller fsync. It bounds its queue, latches overflow/parser errors, and closes by a thread event. Deadline checks reject a delayed snapshot release. G3-FC1 is the remaining observed action-boundary gap.
- NormalStop tests require client close, ProcessExiting, persisted ExitAck and exit status 0. Crash tests require the final seal and injected SIGKILL status. No real signal ran.
- Cumulative filesystem oracles reject old acknowledged-byte corruption and changed retained roots. Saved unlinked FDs are checked before ordinary recovery mutation. The new probes expose the evidence-retention gap and the interval after intent persistence.
- The ownership adapter statically calls the existing target/start/config/listener, mount/worker, fixed filesystem, physical-root, pidfd/binary and capacity checks before the action. Its Ready check binds PID/start/boot/gid. Tests use an owned-target double; this review does not establish live adapter operation. G3-FC1 shows its check can become stale across intent persistence.
- Raw worker tests exercise disposable libc operations, binary names, no-follow roots, exact second-client inode checks, durable intents/returns and single-call EINTR without Python retry. Existing malformed receipt tests pass. G3-FC4 and G3-FC6 remain open.
- CAPS field offsets and version suffixes were checked against `crates/proto/src/caps.rs` and C++ `MClientCaps.h`. The additional matrix uses nonempty snap/flock/inline/pool/fscrypt fields. All five supported versions parse; all 1,402 truncated prefixes and five trailing suffixes reject in each mode. No parser defect was reproduced in this scoped matrix.
- Nested NFS handle, tracefs profile and Ganesha phrase parsers retain their partial scope. Their outputs do not establish complete peer collection or NFS joins.
- `offline=False`, live/prerequisites and native peer qualification remain refusing boundaries. Explicit prepare remains 77/PARTIAL. No tested successful result sets a native or full-G3 qualification flag.

## Frozen provenance

The candidate manifest is `/tmp/opencode/sol-g3-fault-controller-20261025.2_nqiu7r/candidate-files.sha256.json`.

- Manifest SHA256: `8c1d08c3cfe87f2be1ea5a1ba63c8fb6dd9ba630b795555d48507ec5f564c04f`.
- Archive SHA256: `6aa007a7cc7952dc09e1e54c53760d1f516c12a3d590caa631e025c2dbb9a6c8`.
- All 11 candidate files and archive members have the supplied bytes and mode `0664`.
- Interface SHA256: `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523`, mode `0664`.
- `inputs-before.json` contains 218 distinct source/evidence pins. It includes the candidate, interface, helper inputs, interface reports, prior application review, 31 author artifacts, preserved binary and retained G2 manifests/evidence.
- The author full logs each have 234 unique passing test identities. All 166 published baseline identities are included. Their remaining 68 identities equal the focused set in both modes. Reported full times are 54.781 s normal and 208.036 s optimized retry. The first optimized 120 s timeout has no completed summary and remains incomplete. No full-suite rerun was needed for this review.
- The startup pin check verified binary SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`, 13 original evidence files and 139 retry evidence files. These are file-preservation facts, not fresh cluster observations.

Final verification command:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 /tmp/opencode/astra-g3-fault-controller-review-20261026/final_checks.py
```

`final-checks.json` and `inputs-after.json` record **218 of 218 unchanged initial pins**, plus three unchanged C++/Rust reference inputs. All nine Python candidate files parse. All four runtime modules have no runtime assert. All 11 candidate files have no trailing whitespace. The prior application review remains SHA256 `1f8a665769425004ae63cf8fd832f3933be32add256623cd804f8b9c1635aa2d`, mode `0664`. Both interface reports and all pinned author evidence remain unchanged. Active engine/coordinator outputs were outside this freeze.

## Remaining implementation boundary

The proposed wire interface retains its prior scoped acceptance. This review does not change that verdict or the accepted application-tooling verdict.

Complete WireCeph1 parsing/continuity, installed processing/identity providers, syscall trace continuity, NFS compound/session/owner joins, native launcher and complete A4/B3/R0/N1–N4 assessment remain unfinished. Those limits are explicit and do not by themselves block a safe offline core. Correct G3-FC1 through G3-FC6 before independent offline-core re-review. Native capability and full G3 still require separate implementation and qualification.

Only this new primary report and reviewer-local artifacts are writable. No source publication is authorized. Preserve filesystem `a`/44, pools 88/89, both acknowledged namespaces, the old binary and all prior evidence/reports. This review ran no live query, mount, daemon launch, real signal, capture, Cargo command, Git write or subagent.
