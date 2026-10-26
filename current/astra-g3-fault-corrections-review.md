# astra-g3-fault-corrections-review
Task: Independent FC1–FC6 correction re-review. Own only this new primary report and new exclusive external review artifacts. Verify the final frozen package, audit the four-module correction and fixture adaptations, run focused90 and the original16/8/3 public suites in both Python modes, and return scoped acceptance or exact remaining findings.
Status: COMPLETE — CHANGES REQUIRED. Two P1 correction findings remain: G3-FC1-C1 readiness/publication race and G3-FC45-C1 mutable decoded worker receipts. Independent focused90 and copied public16/supplemental8/refined3 pass in both Python modes. Six new public probes have three passing controls and three failed rejection expectations per mode, with zero errors/skips. All1521 artifact pins and784 unique reference pins remain stable. Combined256 remains failed pending separate QA verification.
Context: Final freeze F is /mnt/workspace/sol-g3-fault-corrections-freeze-20261026.z67rpxas. Prior evidence E is /tmp/opencode/sol-g3-fault-corrections-20261026.xwmwyhpo. E/candidate-files.sha256.json is an empty failed output; only F's final manifest is valid. Preserve all candidate paths, old reports/ledgers/protocol/plans and original failure evidence. No live activity, process/signal/network actions, Git writes or delegation. Test guards must precede import/discovery.
Next: Parent assigns the two corrections. Preserve F, E, the original review and this review's red evidence. Re-review the readiness boundary and decoded receipt-to-file binding after a new freeze. Native providers, launcher and full case qualification remain separate assignments.
Log:
- [2026-10-26] Created the sole writable primary ledger before review edits.
- [2026-10-26] Verified F/package and both archives before import. Saved1519+2 artifact and784 unique reference pins in /mnt/workspace/astra-g3-fault-corrections-review-s4o_ocyi/inputs-before.json. Read the four-module delta,22 correction tests and exact reviewer adaptations. Copied runners keep exact bytes. An external wrapper relocates only disposable tempfile directories to the exclusive review root and retains pre-import guards. One combined edit attempt failed on ledger context before changing files; the corrected edit succeeded.
- [2026-10-26] Ran focused90 once per mode: normal14.480s and optimized14.720s, zero failures/errors/skips and zero guard events. Copied public16/supplemental8/refined3 pass per mode. Refined A1 has a genuine positive control and intended result/path rejection messages.
- [2026-10-26] Added six probes for two concrete adjacent concerns. Each mode has three passing controls and three failed rejection expectations, zero errors/skips. Readable pre-action EOF and mutated decoded competitor receipts each produce a false OFFLINE_VALIDATED result. A readable malformed frame invokes the action before failing. All action callbacks remain bounded doubles.
- [2026-10-26] Final preservation check passed:1521/1521 artifact entries and784/784 reference pins. Exact F/E inventory membership and copied runner bytes remain stable. All10 candidate Python files parse; all four runtime modules have no assert; all12 candidate paths have no trailing whitespace. Recorded CHANGES REQUIRED without changing source or prior verdicts.

## Verdict and finding disposition

**CHANGES REQUIRED for the frozen offline core.** The original reproductions now reject. Two adjacent public-path reproductions still violate their correction requirements.

| Original finding | Correction assessment |
|---|---|
| FC1 — final action races | Published EOF/error/queue, partial frames, changed ownership/proof and stale deadlines now block the action and latch consumer failure. **Still open at G3-FC1-C1:** transport readiness can precede reader publication. |
| FC2 — identity/proof joins | Original selected-tid, history-prefix, resend-path, output-input and attachment defects close. The correction checks selected fingerprint/slot/interval, retained source prefix and verified append extension. Commit/cause/final-chunk references resolve against the transcript. Every selected resend requires a matching independent peer record. |
| FC3 — pre-BindAck execution | Original OwnedPath execution defect closes. Held OwnedPath/Reset controls cannot execute. UuidOpen allows journal execution and queued OPEN only. Typed null-input outputs remain held. Exact BindReleased releases the hold; unrelated-client progress passes. **Reset/UuidOpen provider binding remains incomplete.** |
| FC4 — call/competitor joins | Original action, causal return, RENAME-role and first-result defects close with genuine controls. **Still open at G3-FC45-C1:** mutable decoded worker receipts can replace the original argument facts. |
| FC5 — retained evidence | Original persisted-file corruption, truncation, rotation, symlink and stale-finalization defects close. Detected file/transcript corruption stays failed after restoration. **Still open at G3-FC45-C1:** decoded receipt contents are not bound to those retained bytes. |
| FC6 — saved-state guard | Closed in the assigned offline Worker scope. Saved bytes/inode/size/nlink and O_TRUNC pathname are rechecked after durable Intent. Both original healing/replacement probes reject without repair. The caller-owned final exclusive mutation interval remains explicit. |

These are correction assessments, not amendments to the immutable original review. The two remaining findings block offline-core acceptance.

## G3-FC1-C1 — readable pre-action EOF can become expected post-action EOF

**Priority: P1.** Locations: `scripts/g3_fault_protocol.py:628–645,685–699`; `scripts/g3_fault_trial.py:913–931`.

`BufferedChannel._read()` waits for transport readiness before it acquires the condition and sets `read_in_progress`. `action_transition()` checks only published reader state. It does not check unread transport readiness. The reader can therefore observe readiness, lose execution before publication, and let the controller invoke the action.

The new public-controller probe uses a real disposable AF_UNIX socket pair. It completes signal-intent persistence, shuts down the peer's write side and confirms EOF with nonblocking MSG_PEEK. A scheduling barrier pauses the reader after the real select reports readiness, before `_read()` publishes `read_in_progress`. The barrier holds no channel lock. Candidate source, `action_transition()` and consumer state are unchanged.

At the action boundary the channel reports `ended=false`, `error=None`, empty queue and `read_in_progress=false`. The bounded action double records one invocation and releases the reader. The reader then marks `eof_after_action=true`. `seal_and_crash()` returns -9, leaves `failed=false` and sets `closed_expected=true`. A complete A0 series writes `OFFLINE_VALIDATED` in both Python modes, although EOF was confirmed before the action.

The same scheduling boundary with an already readable overlong frame invokes the action before the parser rejects it. That consumer eventually fails, but the required no-action property already failed.

Relevant tests in external `adjacent-probes.py`:

- `test_pre_action_readable_eof_not_relabelled_after_action`: failed rejection expectation; false complete A0 result.
- `test_pre_action_readable_bad_frame_prevents_action`: failed no-action expectation; later parser rejection.
- `test_control_real_socket_eof_after_action`: passes with genuine post-action EOF and a complete A0 result.

**Required correction:** Include unread transport readiness in the final synchronized transition. Make readiness/read publication and action admission use one consistent boundary. A pre-action ready stream must fail closed before the action, including the reader's pre-publication interval. Preserve sticky failure, genuine post-action EOF and separate NormalStop. Keep persistence outside the reader lock and keep action callbacks bounded and nonblocking.

## G3-FC45-C1 — decoded worker receipts can contradict their pinned file

**Priority: P1.** Locations: `scripts/g3_fault_peer.py:286–319,464–483`; `scripts/g3_fault_trial.py:1023–1032,1061–1064,1157–1165`.

`attach_receipts()` stores mutable decoded dictionaries in `Evidence.worker_records`. It also pins the original receipt files. `check_persisted()` rehashes those files, but never compares the decoded worker records with their retained bytes. `check_source()` protects the decoded peer records only. Competitor validation then trusts the mutable worker dictionaries.

The new A1 probe performs a real disposable worker rename from `selected` to `moved` and fsyncs the directory. Its receipt file remains unchanged. Daemon/peer/CallTrace fixtures instead describe `unrelated` to `elsewhere`, using the actual parent inode and correct RENAME destination/source roles. With untouched decoded receipts, `prekill()` rejects the mismatch as expected.

The failing variant changes only the decoded Rename Intent/Return `names` fields after `attach_receipts()`. It does not change a file, hash, file pin, manifest, proof constructor or validation method. The altered dictionaries now agree with the false peer names. The public prekill/seal/add/finish chain accepts them and writes `OFFLINE_VALIDATED` in both modes. The retained raw receipt SHA256 still matches the original `selected`→`moved` call.

Relevant tests:

- `test_mutated_decoded_receipts_cannot_replace_raw_worker_names`: failed rejection expectation; false complete A1 result.
- `test_control_matching_raw_worker_receipts`: genuine matching A1 control passes.
- `test_control_unmodified_receipts_reject_false_wire_names`: rejects at the worker argument/process anchor check.

**Required correction:** Bind every decoded worker record to its exact retained source. Use an immutable representation or re-decode/hash the bounded canonical receipt content before proof use and retained-result validation. Do not rely on file hashing alone while consuming separate mutable dictionaries. A detected mismatch must latch failure and prevent action/result publication. Preserve the actual raw syscall result and the correct two-role RENAME join.

## Verification scope and fixture audit

The exact author runner and four copied reviewer scripts retain their frozen bytes. `run-local.py` changes only `tempfile.mkdtemp(dir="/tmp/opencode")` to a new reviewer-local scratch directory. Real local file, descriptor, socket and thread behavior remains in use. Signal/process-launch guards precede candidate imports in every run. No candidate test or source file changed.

| Independent run | Normal | Optimized |
|---|---|---|
| Explicit focused inventory:68 original+22 corrections |90 passed;0 failures/errors/skips |90 passed;0 failures/errors/skips |
| Copied original public probes |16 passed |16 passed |
| Copied original supplemental probes |8 passed |8 passed |
| Copied original refined A1 probes |3 passed |3 passed |
| New adjacent probes |6 tests:3 pass,3 failed rejection expectations;0 errors/skips |Same |

Counts overlap and must not be added as independent coverage. New failed expectations remain failed product checks. Their false successes retain `native_fault_case_pass=false` and `full_g3_qualified=false`.

The fixture audit confirms these specific adaptations:

- Internal `Call.anchor` is a required typed OfflinePosix1 profile. It carries action argument/FD/root/process/mount/salt facts. All eight daemon `TYPES`/`COMMANDS`/`EVENTS` assignment ASTs remain unchanged.
- Author fixtures put original call entry before send and add competitor processing. The original syscall return remains a separate trace fact. The validator checks its causal and first-result relationship rather than manufacturing it from wire errno.
- Copied reviewer changes only obtain the real disposable parent inode and retain that base in refined paths. Rejection predicates remain unchanged. The refined positive control uses Path=destination and Path2=source.
- The isolated return rejection is `original syscall result disagrees with observed first-result chain`. The isolated pathname rejection is `RENAME destination/source argument join failed`. No TypeError substitutes for closure. The earlier unadapted positive-control error stays in E.
- Focused positive cases complete A0, A1/Rename, A2/Replace, A3/Unlink and two restarts each for B1/B2. NormalStop still requires its own final-frame/ExitAck/status path. Real saved descriptors and cumulative histories remain checked.
- Proof key/attachment construction uses deep copies. Peer decoded records have a canonical-content check. The missing equivalent worker-content check is G3-FC45-C1.
- Reading remains size-limited:16KiB frames,64MiB transcript,4MiB peer/receipt sources,4096 decoded rows and8 worker sources. Framed verification streams records rather than reading an unbounded transcript blob. `validate_references()` still performs repeated full scans per reference; this result does not establish linear-time or maximum-capacity throughput.
- New probe socket/reader threads terminate through local events and joins. Worker/file cleanup runs through test cleanups. No live target, real signal, child launch, external network connection, provider test, Git write or delegation ran.

## Artifacts, exact commands and preservation

Reviewer root: `/mnt/workspace/astra-g3-fault-corrections-review-s4o_ocyi/` (`R` below).

```sh
PYTHONDONTWRITEBYTECODE=1 python3 "$R/verify.py"
CHECK_LABEL=independent-focused PYTHONDONTWRITEBYTECODE=1 python3 "$R/run-local.py" focused
CHECK_LABEL=independent-focused PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R/run-local.py" focused
PYTHONDONTWRITEBYTECODE=1 python3 "$R/run-local.py" probes
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R/run-local.py" probes
PYTHONDONTWRITEBYTECODE=1 python3 "$R/run-local.py" supplemental
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R/run-local.py" supplemental
PYTHONDONTWRITEBYTECODE=1 python3 "$R/run-local.py" joins_refined
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R/run-local.py" joins_refined
PYTHONDONTWRITEBYTECODE=1 python3 "$R/adjacent-probes.py"
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R/adjacent-probes.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$R/final-checks.py"
```

Public and adjacent command stdout is retained as `*-normal.stdout`/`*-optimized.stdout`. Focused logs/JSON are under `focused/`; copied probe logs/JSON are under `public/`; new red logs/JSON and complete false-positive series are under `adjacent-{normal,optimized}/`. For any rerun, use new output roots. The scripts refuse existing test directories/logs.

Before import, independent `verify.py` checked all eight PACKAGE entries, all1519 artifact entries with exact membership,12 source pins/modes and12 exact regular archive members. It also checked the original11-member archive,603 immutable artifacts,218 current reference pins,3 additional reference pins and17 tested inputs. Exactly seven assigned paths differ from original218;211 remain unchanged. `inputs-before.json` and `inputs-after.json` contain1521 artifact entries including the two manifest exclusions, plus784 unique reference pins. Every pin matches after testing.

Frozen correction identifiers:

- Manifest: `dca2d63bf08d627b987dbd33cfb49a261a5f34e1026efed3b05a79175ceec7e0`.
- Archive: `0568b6dd5172e35695b8662b26e722cc8caa17409a5f53210a2c25fd49c5e5b5`.
- Final author checks: `54bcbea5763daa36efec122b08abe358b46ca1712d60345066bed8d704fec72d`.
- Interface: `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523`.

Original manifest/archive remain `8c1d08c3cfe87f2be1ea5a1ba63c8fb6dd9ba630b795555d48507ec5f564c04f` / `6aa007a7cc7952dc09e1e54c53760d1f516c12a3d590caa631e025c2dbb9a6c8`. Original red16/14,8/7 and refined3/2 per mode remain failures. The original68/234 proofs, incomplete120s optimized attempt and208.036s retry retain their scopes. All packaging stops remain retained, including the intentional-symlink assembly rejection and quota failure with E's empty manifest. Only F's final manifest is valid.

The preserved daemon binary remains SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`. All13 original and139 retry native evidence files retain their pins. These checks are file-preservation facts. Filesystem44/pools88/89 and both acknowledged namespaces were not queried or modified.

## Qualification limits

**Combined256 remains FAILED in each mode:**219 passing IDs,37 blocked baseline IDs,6 failures,240 subtest errors and246 Popen denials. This is not a fresh166-baseline PASS. Its metadata and explicit identity inventory were independently verified; it was not rerun here. Separate QA baseline verification remains unread and supplies no result to this report.

Native construction stays disabled. Reset/UuidOpen provider binding, native providers/launcher, daemon hooks and full A4/B3/R0/N1–N4 assessment remain incomplete. Active provider/review outputs were not inspected. This report grants no source publication, promotion or live permission.
