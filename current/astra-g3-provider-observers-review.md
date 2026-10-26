# astra-g3-provider-observers-review

Task: NEW independent P3 offline observer/continuity/clock implementation review. Review the frozen seven-path candidate, public PeerObserver/CallTrace/ProviderIO seams, emitted upstream formats, identity joins, continuity and clock proofs. Verify pins and 127 focused tests in normal and optimized Python. Add independent targeted public-API probes. Source/tests/fixtures/contracts/prior reports are read-only. No installed probes, native collection, process/network/cluster control, Git writes or delegation.
Status: FINAL — CHANGES REQUIRED. Seven concrete findings: four P1 and three P2. The independent focused suite passes 127/127 in each Python mode. The independent public-API regression suite has eight passes and eight failures in each mode, with zero errors/skips/guard hits. Scoped static checks pass. All 76 pinned existing files, seven copied candidate paths and 21 references stay stable. Offline P3 is not accepted.
Context: Sole writable repository path is this report. Author handoff: /tmp/opencode/sol-g3-p3-observers-20261026.ibhcayes/HANDOFF.md. Expected seven-path manifest SHA256: 5ef0d9f42b1e7632da4cf892b2f43da845ba1f0299a0f514c6d3f4242b8ac5ad. Expected archive SHA256: 6f557c427ca1ab4977d0346842b6061342405ab1582b2e7aaf075a860538b24e. Expected 21-reference map SHA256: 7de4ed59c02753f5bbf070c54ec0d6054eb4c3ee79223e4c2ba87c7b18c6d028. Test only an exclusive external copy with pre-import process/signal/network guards and injected I/O.
Next: Parent assigns corrections for P3-OBS-1 through P3-OBS-7. Preserve this review and all frozen/red evidence. Freeze corrected inputs before independent re-review. P4 adaptation must retain the source/stream, final inventory and clock edges identified below.
Log:
- [2026-10-26] Created the sole new review report. Prior evidence remains read-only.
- [2026-10-26 03:19 UTC] Created exclusive evidence root /tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe. pin.py before verifies all requested hashes, seven primary/snapshot/archive paths and 21 references. It pins 76 existing author/reference/primary files. Copied the frozen candidate, references and two runners. Only copied runner ROOT paths changed.
- [2026-10-26] The copied guarded runner passes all 127 focused tests in each Python mode with zero failures/errors/skips. Independent copied static checks pass. The seven copied paths stay unchanged. Source review compares the reducers with pinned upstream emitted formats and final persistence transitions.
- [2026-10-26] Independent probes use new raw payload literals and public APIs. No author test helper or fixture is imported. Both Python modes record 16 tests: eight pass and eight fail. Failures cover borrowed kernel clean state, unrelated capture coverage, incomplete final call coverage, uncovered wall-clock intervals, future close/revalidation timestamps, an invalidated call still joining and contradictory RESET evidence. Real sink files and all failed artifacts remain retained.
- [2026-10-26 03:33 UTC] Final pin verifies zero errors and before_after_identical=true. All 76 existing author/reference/primary files and seven copied candidate inputs match startup. Completed the source audit and ranked seven findings. No installed/native action occurred.

## 1. Verdict and provenance

**CHANGES REQUIRED.** P3 can produce positive processing or completeness evidence from incomplete, unrelated or contradictory observations. These are offline reducer defects. Missing installed symbols or access are separate qualification limits.

| ID | Severity | Finding |
|---|---|---|
| P3-OBS-1 | P1 | A kernel ACK borrows a later handler's clean-state record. |
| P3-OBS-2 | P1 | An unrelated empty capture certifies absence on the selected stream. |
| P3-OBS-3 | P1 | Final pending-call proof accepts coverage that ends before the original exit. |
| P3-OBS-4 | P1 | Wall-clock coverage extends beyond all recorded clock brackets. |
| P3-OBS-5 | P2 | Close and post-close snapshots bypass the acquisition-clock bound. |
| P3-OBS-6 | P2 | `join_call` accepts a call whose failure latch is already set. |
| P3-OBS-7 | P2 | RESET success ignores a contradictory error in the owned invocation. |

Frozen author root:
`/tmp/opencode/sol-g3-p3-observers-20261026.ibhcayes`

Independent review root:
`/tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe`

Reviewed copy:
`/tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe/candidate`

| Input | SHA256 |
|---|---|
| Seven-path `MANIFEST.json` | `5ef0d9f42b1e7632da4cf892b2f43da845ba1f0299a0f514c6d3f4242b8ac5ad` |
| `candidate-seven-paths.tar` | `6f557c427ca1ab4977d0346842b6061342405ab1582b2e7aaf075a860538b24e` |
| 21-reference `references/SHA256.json` | `7de4ed59c02753f5bbf070c54ec0d6054eb4c3ee79223e4c2ba87c7b18c6d028` |
| Immutable fault-v1 interface | `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523` |
| `scripts/g3_peer_observe.py` | `720d7fdd7db1fe5001d8d336e702ec92a069abca38cfc0774e930198fbe0009f` |
| `scripts/g3_provider_io.py` | `a78428860bd49a7550008951288cc377dbd955f1680bedf316e413004b48a7c2` |
| Independent `probes.py` | `fb06e9d21afa1ac16db56cbabdf49dd2d3bb9d6c83cf1581296bcbb71d21ad6b` |

Only this new repository report and exclusive external review artifacts were written. Candidate source, tests, fixtures, contracts, prior reports and author evidence remain unchanged. No broad suite, installed probe, native collection, process/network/cluster control, Git write or delegation occurred. The review supplies no full-G3, native or publication acceptance.

## 2. Ranked findings and minimal reproductions

Implementation paths below are relative to the reviewed copy. Each also identifies the identical frozen primary file. Probe paths refer to the independent review root. All failing probes use public APIs. Their expected rejection is independent of the implementation.

### P3-OBS-1 — P1: kernel clean state crosses handler boundaries

**Source:** `scripts/g3_peer_observe.py:625–652`, `PeerObserver._flush`, especially the unbounded later-`clean` selection at 636–637.

**Probe:** `probes.py:244`, `test_kernel_flush_cannot_borrow_next_ack_clean_state`.

Minimal ordered observations use one client, inode, task and source:

```text
t20 ACK tid100 seq9
t30 ceph_handle_caps ... seq=9
t40 handle_cap_flush_ack ... seq 9 ... cleaned -, flushing Fw -> Fw
t60 ACK tid200 seq10
t70 ceph_handle_caps ... seq=10
t80 handle_cap_flush_ack ... seq 10 ... cleaned Fw, flushing Fw -> -
t90 handle_cap_flush_ack ... now clean
```

There is no detach/clean proof for tid100. `join((ack100, ack200), identity)` nevertheless returns `FlushProcessed` for both ACKs. Both use the final clean record. The first ACK can be an old ACK that removed no flush entry.

Pinned `linux-7.0-caps.c:3856–3863` emits the first cleanup line even when `to_remove` is empty and `cleaned` is zero. That path exits before the clean-state lines at 3885–3890. The t90 line therefore belongs to the second handler. Different sequence values evade the existing same-field candidate check.

**Required correction:** Bound positive clean/detach evidence to one identified ACK-handler interval. An intervening matching handler must end the earlier interval. Do not reuse one processing chain for multiple frames. Preserve valid zero-cleaned detach and false-wake return behavior. The independent single-handler positive control passes.

### P3-OBS-2 — P1: empty coverage has no selected-stream binding

**Source:** `scripts/g3_peer_observe.py:729–751`, `no_reply_before`; `Coverage:812–840`.

**Probe:** `probes.py:380`, `test_empty_negative_inventory_requires_owned_stream_source`.

Create valid identity anchors for owned wire source A. Finish a separate empty `ProviderIO` source B with the same boot but a different source identity. Convert B's real persisted receipt with `Coverage.from_receipt`. Then call:

```python
observer.no_reply_before((), identity_A, cut_50_51, coverage_B, 41)
```

Observed: an `Absence` record with `OFFLINE_VALIDATED` level. There is no edge from B's acquisition scope to A's stream. The check against `coverage.source` runs only inside the wire-record loop. An empty inventory bypasses it. The receipt proves that B drained; it does not prove that B observed the selected connection.

**Required correction:** Require an independently validated acquisition-source-to-stream scope edge even when the relevant frame inventory is empty. Preserve the full source/config/boot and stream identity in that edge. An explicit source-scope anchor can support distinct capture and identity sources. A daemon key or common boot/time interval cannot replace it.

### P3-OBS-3 — P1: final call coverage omits the eventual exit

**Source:** `scripts/g3_peer_observe.py:1034–1051`, `CallTrace.pending_at`; `Coverage.from_receipt:835–840`.

**Probe:** `probes.py:347`, `test_final_call_coverage_must_include_original_exit`.

1. Observe the original syscall entry at t20.
2. Persist and close its trace source at t60. Its final watermark is t55. The artifact contains only the entry.
3. Feed an exit at t90 and matching Return at t100 to the same call reducer.
4. Request `pending_at([40,41], old_coverage, final=True)`.

Observed: `PendingCut(final=True)`. The old receipt does not include the exit or the intervening trace interval. Missing exit/re-entry events after t55 can therefore break the claimed original-call association.

The method checks coverage only through the cut. Its final branch checks that some Return exists. It does not bind the selected exit to the final persisted source inventory or require coverage through that exit.

**Required correction:** Bind final proof to the final trace inventory and its original spans. Require continuous required-CPU coverage through the selected exit. Validate the worker Return through its own final receipt boundary. Reject an earlier receipt that excludes later call evidence. The independent fully covered negative-errno call passes.

### P3-OBS-4 — P1: wall-clock proof extends past the last bracket

**Source:** `scripts/g3_provider_io.py:849–855`, `ProviderIO.finish`; `scripts/g3_peer_observe.py:835–840` and `737–739`.

**Probe:** `probes.py:329`, `test_wall_coverage_requires_brackets_through_final_interval`.

Use a wall-bracketed packet source with valid samples at monotonic brackets `[10,12]`, `[30,32]`, `[50,52]`. Set `max_gap_ns=100`. Finish acquisition at t5000 with drainage watermark t4990 and no decoded rows. Query absence at `[4000,4001]`.

Observed: the provider returns a persisted receipt, and `no_reply_before` returns `Absence`. The last clock bracket ends at t52. There is no intermediate/end clock evidence for the queried interval or for the 4,948 ns tail gap.

`finish` checks that the last sample is not in the future. It rechecks mappings only for rows that already exist. An empty or quiet source can thus certify an interval far outside its clock observations. `Coverage.from_receipt` exports the entire source watermark without a clock-supported range.

**Required correction:** Require real start/intermediate/end brackets across the certified interval, including quiet periods. Enforce the maximum gap through final drainage. Carry the clock-supported interval into coverage and recheck the query against it. Do not extrapolate a completeness interval from old samples.

### P3-OBS-5 — P2: close snapshots bypass acquisition-clock validation

**Source:** `scripts/g3_provider_io.py:728–734,843–848,865–870`.

**Probes:** `probes.py:311`, `test_close_cannot_certify_time_beyond_acquisition_clock`; `probes.py:320`, `test_postclose_snapshot_cannot_jump_beyond_acquisition_clock`.

The normal `_state` path requires `state.at <= hooks.now()`. The source-close and final revalidation paths call `Continuity.state` directly and omit that bound.

With `now()` fixed at t100, the injected close returns `at=10100`, `watermark=10099`, zero descriptors and otherwise valid terminal metadata. Later snapshots return that latched state. `finish` accepts it and produces a receipt that certifies future time. A separate probe changes only `at` during sink-close revalidation; it also returns a receipt. Revalidation deliberately excludes `at` from field equality and does not check it against acquisition time.

**Required correction:** Apply the same acquisition-clock/domain/deadline checks to close results and every post-close snapshot. Preserve latched-source revalidation and the no-retry rule for an uncertain close. Neither invalid transcript may yield a receipt.

### P3-OBS-6 — P2: invalidated CallTrace still produces a joined result

**Source:** `scripts/g3_peer_observe.py:712–727`, `PeerObserver.join_call`; compare `CallTrace._run:875–884`.

**Probe:** `probes.py:392`, `test_join_call_rechecks_latched_failed_trace`.

Build a valid metadata processing chain and an exact Open call with matched paths. Record entry t20, exit t90 and Return t100. Admit the call reducer. Feed a delayed contradictory exit at t40. `CallTrace.feed` correctly raises exit1 and sets `call.failed=True`.

Observed next: `observer.join_call(processing, call)` still returns the old processing/entry/exit/Return tuple. It reads retained call fields but does not check the call's failure latch. The observer's own latch is independent.

**Required correction:** Reject an invalidated call at this public join seam. Bind any retained joined result to the validated final call state/inventory. Preserve the caller obligation to stop on exit1; the join method must not contradict that state with a positive result.

### P3-OBS-7 — P2: RESET success ignores contradictory owned error

**Source:** `scripts/g3_peer_observe.py:669–710`, especially success classification at 699–700.

**Probe:** `probes.py:292`, `test_reset_success_rejects_contradictory_owned_error`.

Use exact RESET flags/UUID, one nonempty successful wire reply, matching `ceph_start_reclaim` entry/return0, UUID continuation and FSAL return0. Insert this real-format line between the library return and UUID continuation, in the same PID/start/thread/Client instance and FSAL invocation:

```text
reclaim_reset :FSAL :EVENT : start_reclaim failed: Input/output error
```

Observed: `ResetSucceeded`. The success branch never checks owned `reset_error` facts. Pinned `ganesha-6.5-main.c:384–389` emits this line only when the actual libcephfs return is nonzero. The inputs are contradictory and cannot prove one successful invocation.

**Required correction:** Reconcile all outcome observations within the selected invocation. A success chain with an owned nonzero-return error must fail or remain unresolved. Preserve separate ENOENT continuation classification and the valid RESET path that sends no FINISH. The independent positive RESET control passes.

## 3. Source audit and retained valid behavior

The review compared the reducers with emitted formats and control-flow positions in the pinned references. It did not rely only on the author handoff.

| Source / location | Audited fact |
|---|---|
| Linux v7.0 `mds_client.c:3346,3884–3936,4031–4048`; `ceph-events.h:156,182` | The prefixed send supplies client/fsid. Send/completion tracepoints omit client. The result log follows wrong-session/duplicate checks. Completion `err` remains distinct from the wire result. |
| Ceph v20.2 `Client.cc:2901–2908,2937–2945` | SAFE receipt precedes stale-rank validation. Caller signaling follows validation. Receipt alone is insufficient. |
| Linux v7.0 `caps.c:3829–3863,3885–3890`; Ceph `Client.cc:5720–5765` | Zero cleaned bits do not prove no detach. The kernel helper's return is a wake boolean. Library signaling follows actual flush-entry removal. Positive evidence must stay inside the selected handler. |
| Ceph `Client.cc:17401–17409,17433–17449`; Ganesha V6.5 `main.c:354–392` | Empty-target RESET returns ENOENT. RESET success returns zero. FSAL continues after errors. RESET does not require FINISH. |
| Ganesha V6.5 `recovery.c:228,343,390,524` | Grace entry/reload/count/exit are separate observed records. Configuration duration is not observed elapsed grace. |
| Linux v7.0 `syscalls.h:18–37,44–61` and `syscall_64.tbl` | Raw entry emits all six registers. Raw exit is signed. The selected x86_64 syscall numbers and restart_syscall boundary are explicit. |

The 127 focused tests cover 75 peer/call methods and 52 I/O/clock methods. Their passing evidence includes client/instance mismatches, duplicate frames, receipt-fsync FD separation, signed libc results, restart context, CPU migration/reuse/death, delayed records, source loss, rotation/copytruncate, kmsg sequence, per-CPU barriers, bounds and persistence failures. The eight new passing probes independently confirm selected positive chains, receipt-only rejection, signed errno handling, restart_syscall context, real sink hashing and terminal failure cleanup. These results do not close the seven findings.

The implementation has no default collector or native constructor. Source origins remain `Fixture` or `Unqualified`. Output levels remain `OFFLINE_VALIDATED`. Imports stay on the finite allowlist. Sanitized facts preserve original references and finite scalar/hash fields. Raw headers, payloads, UUID text and pointer text do not enter the tested artifact paths.

Bounds exist for records, raw/artifact bytes, line/partial size, queue, descriptors, duration, drain reads, source checkpoints and join work. Batch checks precede decoder/queue copies. The injected reader must enforce its advertised limits before allocation. The review does not treat a local row count as upstream continuity.

### P4 adaptation boundary

P3's `Wire` is an explicit projection. P4 must retain complete P1 fronts, frame/stream identities and address unions outside it. The reference-bearing P3 types preserve source, cursor, byte span, boot, uncertainty, CPU and process/task/instance fields. P4 must keep those original records and the source/clock receipts through final assessment.

`join_call` covers metadata operations only. Flush-to-worker and NFS joins remain explicit P4 work. No extra edge is inferred here. Findings P3-OBS-2 through P3-OBS-6 identify missing validation at existing P3 seams; a future adapter must not turn those incomplete results into accepted evidence by adding a native flag or zero clock uncertainty.

## 4. Exact independent checks

Workdir for test/static/probe commands:
`/tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe/candidate`

Both copied author runners differ only in their `ROOT` assignment. Their output directory is the new review `checks/` directory. They import only the two frozen owned test modules. Those modules install process/signal/network guards before the first provider import.

| Actual command | Result |
|---|---|
| `python3 -B /tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe/checks/run_owned.py independent-normal` | 127 pass; 0 failures/errors/skips; exit0 |
| `python3 -B -O /tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe/checks/run_owned.py independent-optimized` | 127 pass; 0 failures/errors/skips; exit0 |
| `python3 -B /tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe/checks/verify_owned.py independent-static` | AST, compile optimize0/1/2, import/guard, whitespace, hashes and reference checks pass; exit0 |
| `python3 -B /tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe/probes.py` | 16 tests: 8 pass, 8 fail; 0 errors/skips/guard hits; exit1 |
| `python3 -B -O /tmp/opencode/astra-g3-provider-observers-review-20261026.h0cy8dpe/probes.py` | 16 tests: 8 pass, 8 fail; 0 errors/skips/guard hits; exit1 |

Normal/optimized counts overlap. They are not summed as unique tests. The focused 127 ran once per mode. No full suite was repeated after the independent failures.

New logs and metadata:

- `checks/independent-{normal,optimized}.{log,json}`.
- `checks/independent-static.json`.
- `probes-{normal,optimized}.{log,json}`. These retain all eight failed rejection assertions per mode.
- `{normal,optimized}-*.jsonl`. These are retained sanitized files from the independent injected-I/O probes. Failed artifacts are preserved.
- `before.json`, `after.json`, `pin.py` and `review-artifacts.json`.

The probe source contains its own raw row literals and scalar expectations. It imports neither author tests nor author fixtures. Its process/signal/network guards run before importing the candidate. Sink writes, file fsync, close and directory fsync use only new review-owned disposable files. Acquisition uses explicit injected callbacks. Guard hit counts are zero.

## 5. Stability and installed qualification limits

`pin.py before` at 03:19:10 UTC and `pin.py after` at 03:33:05 UTC verify all requested manifest/archive/reference hashes. The archive contains exactly seven files with the expected bytes and modes. All seven primary paths, frozen snapshot paths and independent copy paths match. All 21 reference copies match; declared local reference paths also match.

The before/after inventory includes **76 existing files** across author evidence, references and the seven primary paths. It includes historical failed/intermediate evidence. Final `before_after_identical=true`; zero errors. The copied candidate's seven input records also stay identical. No mismatch exclusion was needed. Coordinator files outside this explicit set are not part of the stability claim.

The author baseline79/three-failed-subtest result and intermediate runs remain historical. No new result relabels those failures. The accepted provider plan and fault-v1 interface remain unchanged.

Installed qualification remains open for kernel formats/symbols/layouts/access and probe misses; mount-to-client identity; exact library/process/instance-to-stream attribution; all-CPU source acquisition and barriers; packet/logger completeness; real clock monitoring; callback deadlines; and installed package/ABI equivalence. Local7.3 constants do not establish installed7.0 equivalence. These limits do not excuse the offline acceptance gaps above. P4/P5 integration, installed probes, daemon hooks and live scheduling retain their existing ownership and authorization requirements.
