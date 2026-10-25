# sol-verification-review
Task: Independent Sol review of completed QA scripts/CI/devcluster changes vs f54d298. Review scripts/**, devcluster/** and .github/**. No live operations or source edits.
Status: ACCEPT corrected G3 application tooling at the frozen R6 candidate. G3-R6 is closed; G3-R1 through G3-R5 remain closed. All 29 selected tests passed normally and optimized. Exact R6 reproductions now fail closed. Prior G2 acceptance remains valid. Full G3 remains open.
Context: Sole writable file is current/sol-verification-review.md. R6 release inputs are frozen in /tmp/opencode/sol-g3-r6-20261025/candidate-files.sha256.json. Earlier G3 review used base a6ae05a10bfefdcea8dae75cd29597c84a3fadcd. QA owns all live operations. Focused offline checks use local temporary data, mocked native boundaries and fail-fast signal guards.
Next: Designated release owner can use the accepted frozen manifest and archive hashes below for release provenance. Live G3 still requires accepted runtime/tooling/binary provenance and explicit parent assignment to sole QA. Exact wire-loss, protocol OPEN timing and reclaim qualification await implemented and reviewed seams.
Log:
- [2026-10-25] Completed R6 re-review: ACCEPT corrected scoped G3 application tooling. Original malformed-proof/root-drift/repeated-prepare reproductions now reject. Both complete-controller later-cold regressions return runtime1 with no result. Recorded the seven-path release manifest and archive hashes. Final candidate/source hashes, changed Python AST and scoped diff checks passed. Only this primary-tree report changed.
- [2026-10-25] Completed the optimized rerun with a longer tool limit: all 29 selected tests passed. Independently re-ran the original empty/wrong-type proof, prepare-inode drift and repeated-prepare probes in both modes; all now fail at the intended validator. A separate sweep rejects an empty object at each of the 13 produced proof positions, with all later records correct.
- [2026-10-25] Verified all seven frozen R6 candidate/context hashes and the user-supplied corrected archive hash. Only four paths changed from the prior candidate. AST source-segment comparison confirms that the worker, comparison, contender, CLI and controller/context bytes are unchanged. All 29 selected normal-mode tests passed. The first optimized attempt hit the tool's 120-second limit before completion; it supplies no complete optimized result. No matching local oracle/worker children remained at the follow-up read-only process check.
- [2026-10-25] Started independent re-review of the frozen G3-R6 correction at /tmp/opencode/sol-g3-r6-20261025. Only this primary-tree report is writable. Source/test edits, live calls, actual signals, Git writes, commits and subagents remain prohibited.
- [2026-10-25] Completed five-fix re-review: CHANGES REQUIRED for residual P1 G3-R6. Added optimized malformed-history controls and nested-directory replacement probes. Verified all seven frozen candidate inputs, 20 correction evidence files, eight accepted G2 inputs, the preserved binary, and all 13 original plus 139 retry evidence files. Scoped AST, shell syntax and diff checks passed. Only this report was edited.
- [2026-10-25] Independently passed all 62 G3 tests and all 10 sequential-history tests normally and optimized. Re-ran the original descriptor, earlier-history, capacity and root-escape reproductions; their corrections hold. Post-validation root-replacement probes remain FD-anchored. Reproduced empty descriptor proofs and changed prepare root identity passing the cold oracle. Empty proofs in both histories also permit 0/APPLICATION_PASS in the complete two-round controller.
- [2026-10-25] Started independent re-review of G3-R1 through G3-R5. Only this report is writable. No production edits, live calls, actual process signals, commits or subagents.
- [2026-10-25] Completed G3 candidate review. Independently passed 31 focused tests normally and optimized. Reproduced descriptor-data repair hiding loss, a complete two-round false pass after earlier-history corruption, ignored checkpoint limits, and a recovery write through a replaced root symlink. Modeled the NFS user-xattr incompatibility. Only this report was edited.
- [2026-10-25] Resumed as Astra for independent G3 tooling review. Only this report is writable. No source edits, native signals, live calls, builds, commits or subagents. Earlier gate-status statements below are historical.
- [2026-10-22] Started focused correction review after legacy mount ID 323 recycling stopped G2 before comparison/nonreuse. Production acceptance and 524/0/2 QA status come from updated repository instructions, not a new run here. Sole report/no-live/no-signal restrictions remain.
- [2026-10-22] Resumed for focused enumeration-coverage correction. Sole-report and no-live/no-signal restrictions remain. Runtime fixes are still underway.
- [2026-10-22] Started focused shared-oracle correction re-review. Runtime daemon review remains pending. All sole-report, no-live, no-signal and no-subagent restrictions remain in force.
- [2026-10-22] Resumed for independent review of executable G2 and partial G3. Sole writable report and all no-live restrictions remain in force. Daemon G2 implementation is still active; no live gate is authorized.
- [2026-10-22] Created review ledger before review work.
- [2026-10-22] Read supplied AGENTS instructions, HANDOVER, build brief, minimum feature set, approved G0-G3 workplan, and QA ledger.
- [2026-10-22] Compared every archived tracked file with its f54d298 Git blob: 1,722 checked; zero mismatches. Summed authoritative f54d298-unit-with-local-fixtures.log: 347 passed, 0 failed, 2 ignored. Missing optional capture is separately disclosed. QA explicitly invalidates contaminated immutable-* artifacts and uses an exclusive replacement target. Native RADOS and smoke results remain historical evidence inspected here, not independently rerun live tests.
- [2026-10-22] Ran bash -n on modified/new shell scripts: passed. Parsed all scripts/*.py and scripts/tests/*.py with ast.parse: passed. Ran PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v: 4 passed. No build needed.
- [2026-10-22] Reproduced external-history path escape entirely offline. Created a temporary directory and a symlink to its synthetic tested tree. Mocked only os.path.ismount. crash-history.py record accepted history through the symlink, returned 0, and stored history inside the tested tree. Temporary files stayed under /tmp/opencode and were removed by TemporaryDirectory.

## Original ranked findings (resolved by correction re-review below)

1. **P1: Validate mount and active target before any mutation or cleanup.** `scripts/recovery-trial.sh:35-57` installs unconditional failure-unmount cleanup before identity capture and records on the supplied mount before validating PID. `crash-history.py:86` accepts any mount point. No check binds that mount to cluster fsid, filesystem a/fscid, or the selected MDS rank. A local/NFS/other-cluster mount can receive the workload and then be unmounted. A failed first identity query can unmount the caller's existing mount without a workload or valid MDS. A same-config standby also passes the process check. Add read-only preflight for mount type/source/identity and active MDS mapping. Require explicit ownership of the supplied mount. Arm cleanup only after validation. Compare mount identity again before cleanup/remount/verification.
2. **P2: Smoke existing-mount protection compares path spelling, not mount identity.** `scripts/smoke.sh:29-38` constructs paths from unnormalized RMDS_RUN_DIR and compares them literally with escaped `/proc/mounts` fields. A trailing slash produces `...//mnt-kernel`; relative paths, symlink parents, and spaces also bypass the check. The helpers can then mount over an existing mount. Cleanup can miss those paths. Normalize paths and use mount-aware checks with decoded paths. Add offline guard tests for trailing slash, relative path, symlink, and spaces.
3. **P2: External-history location guard follows no parent symlinks.** `scripts/crash-history.py:88-91` uses absolute(), not resolve(), for history. A symlinked parent outside the tested path can point inside it. The independent controller log then shares the tested filesystem and can disappear with the state being tested. This was reproduced offline. Resolve the history parent and reject the tested filesystem, including aliases where feasible. Keep exclusive file creation. Add a regression test.
4. **P2: G2 permits forced/lazy detach instead of clean unmount.** `scripts/recovery-trial.sh:45-47` calls `devcluster/umount.sh`, which escalates normal unmount failure to force and lazy unmount. The trial continues after lazy detach without proving client flush/session shutdown. This cannot establish the advertised clean G2 restart path. Use non-forced bounded unmount for the trial. Reserve forced/lazy detach for explicitly owned failure cleanup and report it separately.

## Accepted behavior and limits

- Explicit C++ MDS keyring fix is correct. Smoke binary selection honors CARGO_TARGET_DIR and RMDS_BIN. Filesystem-scoped sync avoids unrelated host writeback.
- Offline CI preserves strict clippy failure while running later tests. Baseline strict-clippy failure is disclosed, not hidden by warning allowances. Workflow separates public offline checks from manually dispatched protected and serialized disposable live checks. GitHub execution itself remains unverified.
- ci-live.sh requires disposable authorization and does not silently skip missing native prerequisites. It calls newfs only in the explicitly disposable G0 lane. No new OSD kill/reset path was found.
- The sequential oracle checks overwritten data, deleted names, surviving link count, and xattrs. It is not an existence-only oracle. Incomplete histories return 77 instead of guessing uncertain outcomes. Full in-flight enumeration is explicitly absent.
- The partial controller checks cluster/filesystem/pool identity before and after restart and uses pidfd signaling restricted to MDS executables/config. It never returns a full gate PASS. It does not constrain the supplied restart command beyond a post-hoc identity comparison. It does not measure recovery/first-success latency or exercise under-load crashes, retries, locks, dirty caps, flush acknowledgments, or unlink-open. Success returns 77 and thus currently also enters failure-unmount cleanup. These limits do not justify demanding full G2/G3 implementation in this assignment.
- Canonical progress_ledger.md G0 rows still say pending. Parent must transfer the QA evidence and review result there without treating G0 or G2/G3 as green.

## Correction re-review, 2026-10-22

**ACCEPT. No remaining scoped blockers.** The original findings above describe the first reviewed version, not the current implementation.

Read the updated QA ledger, recovery shell wrapper and Python controller, mount guard, history oracle/tests, smoke script, mount/unmount helpers, Ganesha helper, verification instructions, and CI scripts/workflow.

### Verified corrections

- Recovery requires `--owned-mount` before probes. Preflight reads monitor identity, root kernel CephFS mount identity/fscid, and active rank-0 target identity before creating trial artifacts or running a workload. Executable, config, daemon ID, process start time and process-owned IPv4 listening sockets bind the PID to the active rank. The pidfd plus before/after process checks prevent ordinary PID-reuse signaling.
- The controller takes the host lock and repeats mount/target checks before execution. It checks again after recording and before signaling. Cleanup requires workload admission and the captured mount ID/key. A replaced mount fails closed. Successful partial trials return 77 without failure cleanup. G2 clears cleanup eligibility before its single normal-unmount attempt. Failed or timed-out unmount prevents signal, restart, and escalation.
- Physical mount alias resolution and decoded findmnt rows replace raw `/proc/mounts` comparisons. Existing and stacked mounts are refused. Smoke records mount IDs and compares them before cleanup. Actual smoke root-alias refusal is tested before binary startup, build, or log creation.
- History resolves physical parents, rejects a final symlink, rejects the tested device and CephFS/NFS controller-log parents, and retains exclusive creation. The original static symlink escape no longer works.
- The history verifier validates operation ordering and acknowledged prefixes. It checks exact names, data, inode/link relationships and xattrs. Interrupted sequential histories permit before/after outcomes and matching partial create data. Valid incomplete histories still return 77. No full-gate success is inferred.

### Independent offline evidence

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v`: 29 passed.
- Same command with `PYTHONOPTIMIZE=1`: 29 passed.
- `bash -n` on recovery-trial.sh, smoke.sh, ganesha.sh, mount-kernel.sh, mount-fuse.sh, umount.sh, ci-offline.sh, ci-live.sh: passed.
- Python AST parsing of scripts/*.py, scripts/tests/*.py, and devcluster/mount_guard.py: passed.
- `git diff --check -- scripts devcluster .github`: passed.
- Read-only reference check confirms that the kernel exposes fscid in the upper statfs fsid word used by the bounded statvfs probe. No kernel implementation was translated.
- No real signal, mount/unmount, daemon start/stop, cluster query, build, source edit, or subagent was used. Focused tests mock mutation paths. They use temporary files, read-only process/mount metadata and a local test socket.

### Ganesha recovery-mode review

Installed Ganesha 6.5 core/CEPH man pages document `RecoveryBackend=fs`, `RecoveryRoot`, grace/lease limits, `Server_Owner`, `Server_Scope`, and `client_oc`. The rendered recovery configuration uses valid documented values: 90s grace, 60s lease, persistent ganesha-recovery/recov, stable node identity, cluster-scoped owner/scope, no delegations, and disabled client object cache. Smoke stays graceless. Both helper modes use loopback port 12049 and refuse a recorded running opposite mode. Stop selects the requested helper's config/PID file, not the system instance. No new startup/stop code deletes the recovery directory or changes the system service on 2049. Live startup, NFS grace/reclaim, and Rust session recovery remain untested. The helper is single-node filesystem-backed recovery, not clustered RADOS-backed takeover.

### Scope and operational conditions

- These guards are not an atomic exclusion mechanism against an unrelated privileged actor replacing mounts, changing symlinks, or replacing processes between checks. The scripts require the documented exclusive QA owner and stable operator-owned paths. Rechecks, captured mount IDs and pidfd signaling are sufficient for that scope. No extra platform synchronization requirement is imposed.
- Smoke still contains pre-existing `sudo sh -c` interpolations. A run-directory name containing a single quote is not supported safely by those I/O commands. This predates the reviewed changes. Use ordinary operator-owned run paths. It is not a blocker for the scoped correction or the documented space/alias guards.
- The partial controller supports root kernel CephFS mounts and IPv4 MDS listener identity only. FUSE/NFS and IPv6-only targets fail closed. The restart command remains trusted operator input; changed filesystem/pool identity is rejected after restart, not prevented inside an arbitrary command.
- The controller records the whole sequential history before stopping the MDS. It does not inject an in-flight crash. Surviving-client caches can hide missing server state. Full G3 needs server-visible work and an independent cold client. Separate reconnect/first-success latency, concurrent history linearization, transport retry IDs, locks, dirty caps, flush acknowledgments, unlink-open, allocator state, and checkpoint/trim fault cases remain unqualified.
- CI still preserves strict lint failure. Adding util-linux supplies findmnt for the focused offline tests. No new skip-to-pass conversion, nondisposable newfs path, OSD signal, or reset path was introduced. Actual GitHub workflow execution and current-source native recovery integration remain separate evidence.
- Authoritative isolated f54d298 baseline evidence and its invalidation history remain unchanged. Acceptance here applies to tooling, not to G0 lint completion or G2/G3 gates.

## Executable G2 / G3 foundation review, 2026-10-22

### Verdicts

- **G2 tooling: CHANGES REQUIRED.** The executable application gate has a shared oracle false-pass. Fix it before an integration trial can establish evidence.
- **G3 foundation: CHANGES REQUIRED.** It uses the same oracle for surviving/cold-client comparison. Its explicit PARTIAL/exit-77 behavior is correct and never claims full G3.
- No live G2/G3 qualification exists. Daemon implementation/review and separately authorized fresh Rust setup remain dependencies. Do not use fs43, which the parent identifies as C++ metadata. The controller requires an operator-designated fscid and Rust-filesystem acknowledgment. It does not inspect the on-disk format itself.

### Ranked blocker

**P1 — Directory read errors can produce a successful durability comparison.** `scripts/restart_history.py:186-195` traverses with `os.walk(..., followlinks=False)` and no error handler. Python ignores scandir errors by default. The mixed workload contains an acknowledged empty directory. If its READDIR fails, traversal can omit its children without failing. Stat of the directory can still succeed. Verification then accepts an unreadable or damaged directory and can hide unexpected namespace entries. Both G2 post-restart comparisons and G3 independent cold comparison use this function. Consequently, an exit-0 G2 application result does not currently establish the advertised exact namespace comparison.

Independent offline reproduction:
1. Run the real mixed oracle on a temporary local synthetic mount path containing a single quote and spaces. Load its manifest.
2. Add `empty/unexpected-resurrected-file`. Unmodified verification correctly raises `namespace differs`.
3. Mock only `os.scandir` for that acknowledged empty directory to raise `OSError(EIO)`. Leave real traversal/stat/data checks active everywhere else.
4. `verify_manifest` returns successfully. No signal, mount, process launch, or Ceph operation is involved.

Required correction: use traversal that propagates directory-enumeration errors. A directory must not count as verified unless enumeration succeeds. Preserve type/symlink validation. Add an oracle regression for READDIR EIO or permission failure under an expected empty directory, including an unexpected child hidden behind that error. The verification command must fail and the controller must not write PASS/PARTIAL result metadata. Do not convert this runtime error into exit 2 or 77.

### Reviewed pass-path and safety behavior

- Explicit readiness, ownership, Rust-filesystem acknowledgment, designated fscid, root/capability checks, target process/config/listener validation and pidfd checks precede workload admission. Read-only preflight rejects other visible CephFS mounts. Exclusive artifacts and the host lock prevent ordinary old-artifact reuse or concurrent lane execution. No reset/newfs/pool/OSD-signal path exists in the controller.
- The real mixed oracle records 17 intent/ack operations after successful calls and flushes an external log. It retains 10 distinct pre-restart inode identities, including 4 deleted/overwritten identities. Data, xattrs, file metadata, hard links and deleted names have substantive checks. Partial history and failed fsync do not create successful completion markers. The blocker above is an ignored traversal error, not a demand for a larger workload.
- G2 uses one bounded normal unmount. Failure prevents signal/restart/escalation. The no-old-mount audit runs after close and before the cold remount. Only the validated old MDS pidfd receives the chosen TERM/KILL. Restart uses a direct argv launch of the selected binary with explicit keyring and original daemon ID/config. Binary/config checksums, selected child identity, a different active gid and observed active map state prevent stale-old-active success. Cold mount uses `noshare` and rejects the old mount ID.
- Post-restart verification precedes 16 future create/write/file-fsync/parent-fsync operations. Future IDs must be unique and absent from every saved allocation, including removed inodes. A second old+new comparison follows. G2 result creation occurs only after these boundaries. Fixing traversal is required to make those comparisons trustworthy.
- G3 retains the primary kernel mount. Optional FUSE is a controller-created foreground child, not an adopted arbitrary mount. Selected cold standby identity is bound before takeover; explicit restart uses the reviewed launch path. A separate cold kernel mount uses `noshare`. Pending-intent observation is accurately distinguished from proof of a request reaching/committing at the MDS. The owned workload must finish without errors. Same-wire-tid proof is explicitly false.
- G3 metrics name the observed-new-active interval and a defined post-recovery O_EXCL/write/file+parent-fsync progress probe. They do not claim the earliest success of every client. G3 has no exit-0 path. Full locks, dirty caps, cap-flush acknowledgments, unlink-open, Ganesha reclaim, precise committed-reply-lost retries and concurrent histories remain unqualified.
- New gate/launch/oracle paths use argv arrays or quoted helper arguments. The real local oracle worked with a quote and spaces. The old smoke.sh shell interpolation warning remains separate: the executable G2/G3 controller does not invoke smoke.sh. Static physical history-parent and same-filesystem containment checks remain in place. Stable operator-owned paths and exclusive QA ownership remain required; this review does not demand atomic defense against a separate privileged actor.
- Error cleanup selects captured matching mount IDs, uses normal unmount only, and retains busy/replaced/unvalidated mounts. It does not implicitly stop the original caller-owned MDS on workload failure. Newly created daemon/client children are retained while a test target remains mounted. The external standby and unrelated OSD/Ganesha processes are not selected as cancellation victims. Kernel-blocked commands may remain with pending termination; their PID is reported. No silent successful skip follows such a timeout.
- Ganesha recovery mode and CI policy remain as previously reviewed. Strict lint failure is not hidden. No new live/reclaim/whole-G2 storage-fault-matrix claim is made.

### Independent offline verification

- Ran **56 of 57** focused tests, normally and with `PYTHONOPTIMIZE=1`: all passed. Excluded `test_timeout_cancels_only_its_owned_command_session` because it sends real signals to test-owned children, which this review forbids. Added fail-fast signal mocks around the selected suite. No test issued a real signal or mount operation.
- Separately modeled `command` timeout and `stop_owned_child` TERM/KILL handling with mocked Popen, process-group signals, pidfd operations and waits. Both checks passed without launching or signaling a process. This confirms selected cancellation targets/order, not native cancellation behavior.
- `bash -n` on recovery/smoke/CI/mount/unmount/Ganesha scripts: passed. Python AST checks on all tooling/tests: passed. `git diff --check -- scripts devcluster .github`: passed.
- The new READDIR-EIO false-pass reproduction uses only local temporary files and a scandir fault mock. Temporary data stayed under /tmp/opencode and was removed. No source/test file was changed.
- Reviewed hashes: recovery_trial.py `5f7f0335833aa64bfef4f77cd5a456f0bed58e2db5a6e9020c367786644ea9d0`; restart_history.py `fc5b990771aad6945effcb2aa06db9d3278cf6fddb9770d7f878c7414823e5d0`; mount_guard.py `c6bd2fbcaec8b15ff565752a77c73ffc294dddcc49d9550144e7cbd2f2107001`.
- No live cluster calls, daemon actions, actual mount/unmount, real signals, builds, commits, source edits or subagents. Only this report was edited. Existing isolated baseline and native-platform evidence were not changed or reinterpreted.

## Focused scandir correction re-review, 2026-10-22

### Verdicts

- **G2 complete application harness: CHANGES REQUIRED.** Original scandir EIO/EACCES handling is fixed. The related residual false-pass below still permits an incomplete namespace comparison.
- **G3 partial foundation: CHANGES REQUIRED.** It uses the same shared oracle. PARTIAL/exit-77 behavior remains correct; there is no full-G3 pass path.
- These verdicts concern tooling only. The parent reports persistence review accepted and runtime daemon review pending. No live trial, fresh bootstrap, or use of fs43 is authorized here.

### Correction confirmed

- `restart_history.py:190-193` re-raises scandir/open/iteration errors through os.walk's onerror callback. Repeated the exact original local hidden-child reproduction under a path with a single quote and spaces. EIO and EACCES now propagate instead of returning success.
- The CLI regression returns runtime exit 1 with no verified output. Both G2 cold comparison and G3 independent cold comparison regressions propagate the real shared-oracle exception. `failure_code` returns 1 after workload admission. Neither writes result.json, timings.json, client-state.json, PASS or PARTIAL output. This is not exit 2 or 77.
- Valid empty directories and symlink/no-follow rejection remain covered. The timeout isolation test now uses mocks only; it no longer sends real signals.

### Remaining ranked blocker

**P1 — os.walk can still skip an expected directory after a type-lookup error.** `scripts/restart_history.py:193-207` checks names and later stat types, but does not require successful enumeration of every expected directory. Python's os.walk separately catches `DirEntry.is_dir()` OSError and treats that entry as a non-directory. It does not call onerror for that exception. A later Path.stat can identify the same entry as a directory. The verifier then continues without proving its contents were read.

Independent focused reproduction on the corrected source:
1. Record the real mixed oracle on local temporary data. Add `empty/unexpected-resurrected-file`.
2. Keep real scandir/stat/data checks. Wrap only the DirEntry for the acknowledged `empty` directory so its `is_dir()` raises EIO. This models failed directory-type lookup during enumeration.
3. os.walk suppresses that exception, classifies `empty` as a file, and never visits it. The later real stat says directory and takes the existing continue branch.
4. `verify_manifest` succeeds despite the unexpected child. Repeating through the real CLI entry point, with only synthetic mount/isolation boundaries injected, prints `{"verified": true}` and no error. No kernel/daemon/process operation is involved.

Required minimal correction: ensure each expected directory was successfully enumerated, not only seen by name and statted. For example, track the directories actually yielded by error-propagating traversal and compare them with the expected directory set. Alternatively, use explicit no-follow traversal that propagates type and enumeration errors. Preserve the existing symlink rejection. Add the `DirEntry.is_dir()` EIO regression with the hidden child; verification must fail as a runtime error and must not emit success artifacts. This is the same exact-namespace invariant as the original P1, not a request for more platform or workload coverage.

### Focused independent checks

- Ran nine selected correction/adjacent tests under fail-fast outer guards for os.kill, os.killpg, signal.pidfd_send_signal and Popen.send_signal. **9 passed normally; 9 passed with PYTHONOPTIMIZE=1.** Selection: hidden-child scandir EIO/EACCES, CLI exit 1, symlink/no-follow, valid mixed workload, G2 runtime/no-result, G3 runtime/no-result, mocked timeout cancellation, and retained G2/G3 positive phase models.
- Did not rerun the full 62-test suite or unrelated native/platform work. Source hashes confirm recovery_trial.py and mount_guard.py are unchanged from the prior full tooling review. The relevant corrected oracle hash is `c90c69b7daddd44ef8002a51d1f02a5816d6d51372a7aea5a9597ff22ae29f85`.
- Focused Python AST checks and `git diff --check -- scripts current/sol-verification.md`: passed.
- Original and residual reproductions used temporary local data under /tmp/opencode. All temporary data was removed. No real signal, live query, mount/unmount, daemon action, source edit, build, commit or subagent. Only this report was edited.

## Expected-directory coverage correction re-review, 2026-10-22

### Final tooling verdicts

- **G2 complete clean-restart application harness: ACCEPT.** No remaining scoped blocker from this correction or the prior executable tooling review.
- **G3 mounted-recovery foundation: ACCEPT, partial only.** Its PARTIAL/exit-77 contract and previously documented coverage limits remain in force. This is not full G3 acceptance.
- Runtime fixes remain underway. Neither verdict is live gate evidence or authorization. Parent approval, accepted runtime review, reviewed integrated executable, designated fresh Rust setup separate from the trial, recorded new identity and exclusive QA ownership remain required. Fs43 remains prohibited by the assigned protocol.

### Verified invariant and regressions

- `restart_history.py:190-205` now records only directory roots yielded by the error-propagating os.walk traversal. It requires every expected directory to appear in that set. Root is represented by the empty string on both sides. The initial `actual={""}` namespace seed cannot satisfy enumeration coverage. Merely seeing an entry name or later statting a directory is no longer sufficient.
- Retested the exact residual reproduction independently: real local mixed workload, hidden child under `empty`, and only its DirEntry.is_dir lookup raising EIO. Verification now raises `ValueError: directory enumeration incomplete`. It no longer returns success.
- Independently omitted the root and empty-directory yields, with all other real traversal/stat/data checks retained. Both omissions fail under direct paths, a symlink-parent alias, and a dot-dot spelling. Complete traversal passes under all three spellings. The relative-root normalization and set comparison do not hide an omitted directory.
- Every expected directory must now be enumerated. Unexpected entries still fail exact name-set comparison. Unexpected directory entries cannot disappear merely because their type lookup fails: their names remain extra. Scandir errors still propagate through onerror. No-follow behavior and explicit symlink rejection remain unchanged. Valid empty directories still pass.
- Focused real CLI regression confirms type-lookup uncertainty returns exit 1, with no verified output. G2 cold and G3 independent-cold caller regressions confirm runtime classification 1, no result.json/timings.json/client-state.json, and no PASS/PARTIAL output. There is no prerequisite-skip conversion.

### Focused independent evidence

- Ran **11 selected tests normally and with PYTHONOPTIMIZE=1: 11 passed in each run.** They cover the five new regressions, valid mixed/empty-directory traversal, symlink/no-follow behavior, original scandir EIO/EACCES, CLI exit 1 and both callers' original error/no-artifact paths. Outer fail-fast guards cover os.kill, os.killpg, signal.pidfd_send_signal and Popen.send_signal.
- Ran the independent original type-error reproduction and the six omitted-root/empty checks across physical/alias/dot-dot path spellings. All expected failures occurred; valid traversal controls passed. All synthetic files stayed under /tmp/opencode and were removed.
- Focused Python AST checks for the oracle, fault helper and changed test modules passed. `git diff --check -- scripts current/sol-verification.md` passed.
- Did not repeat the full 67-test suite or reopen unrelated platform/daemon work. recovery_trial.py and mount_guard.py hashes remain unchanged from the prior full review. Accepted oracle hash: `d391fe4f1267a5765a33ab7b0bb27e1f8cb8e40024ebcb4a57815263e1f851f9`.
- No live query, actual signal, mount/unmount, daemon action, build, source/test edit, commit or subagent. Only this report changed. Baseline/native evidence remains untouched.

## Boot-scoped unique mount identity review, 2026-10-22

**ACCEPT for the correction and G2 same-filesystem retry tooling.** No remaining scoped blocker was found. This supersedes the earlier cold-remount assumption that legacy mountinfo IDs cannot be reused. It does not reinterpret the prior failed trial as a pass. Complete G2 remains NOT PASSED until the full comparison/nonreuse path runs successfully. Nonempty G3 remains held.

### Query and parsing

- `capture_mount_identity` requires an exact physical mount in the flat mount table. It then invokes `findmnt --kernel=listmount --json --mountpoint PHYSICAL_PATH -o TARGET,UNIQ-ID`. The installed findmnt manual distinguishes exact `--mountpoint` from covering-parent `--target` and documents the listmount/statmount backend and 64-bit node IDs. No alternate legacy-ID query is used on failure.
- The parser requires exactly one row, an exact decoded target match and a nonzero JSON integer below 2^64. Python retains full integer precision. Bool, string, float, nil, zero, negative, oversized and missing IDs fail closed. Root `/` is a valid exact mount; a covering parent is not a substitute for the requested target.
- A canonical non-nil host boot UUID scopes saved unique IDs. Capture rechecks the boot UUID and mount metadata. Cleanup compares the entire saved identity, including unique ID and boot UUID. Legacy-only saved identities cannot authorize cleanup. A changed boot is an error, not evidence of coldness.

### Actual controller boundaries

- Preflight captures the required unique identity before artifact/workload admission, cleanup arming, pidfd signaling and unmount. Unsupported/malformed unique interface responses fail preflight with exit 2. The modeled corresponding cold-remount failures after admission return runtime exit 1 and create no result/timing/client-state success metadata.
- G2 still requires bounded normal unmount, the no-old-CephFS audit after close, exact old-MDS stop, selected new-active observation, another no-old-CephFS audit and an explicit `noshare` cold mount. A new unique mount ID is not used as a substitute for those conditions.
- A recycled legacy ID323 with a different unique ID in the same boot reaches the existing full verify -> 16 future allocations -> verify(future=True) path in the complete controller model. Retaining the original unique ID fails even when legacy ID changes. A changed boot UUID also fails. The completed gate still needs all prior oracle checks before result creation.
- Primary, cold and owned FUSE captures use the same required identity. Error/cancellation cleanup retains a replacement or uncertain mount rather than authorizing unmount from recycled legacy metadata. Matching owned identities select only normal unmount. Known child signaling remains restricted to the prior pidfd/owned-child paths. No OSD, reset, newfs or pool operation was added.
- Captures and comparisons occur in the caller's mount namespace. Helper children inherit it. Namespace clones represent separate mount instances; matching legacy IDs across namespaces do not establish identity. The all-visible-namespace audit remains independent and does not treat another namespace's same legacy ID as owned. Within the documented exclusive QA environment, these repeated checks are sufficient. They are not an atomic defense against an unrelated privileged actor replacing a mount between a check and unmount.

### Private namespace and smoke

- The retry command first creates a new mount namespace and makes propagation private there. It does not change host mount propagation. It creates the owned primary mount inside that namespace and execs the controller with quoted argv, expected fscid44 and the accepted binary/keyring. This prevents the observed propagation clones without weakening the no-other-CephFS guard.
- On success, the controller-created daemon retains the private namespace and its owned cold mount. On failure, normal owned cleanup and the existing retained-child rules apply. If the final namespace holder exits, kernel namespace teardown affects that private mount tree, not peer host namespaces. The protocol must remain exclusive to the assigned QA owner and its test mounts.
- New smoke capture/same calls are quoted and share the strict identity parser. Its root capability probe runs before daemon startup, test mounting and I/O. Existing physical alias, stacked-mount and root-alias refusal checks remain before that probe. No covering-parent match is introduced. The previously noted single-quote shell interpolation in unrelated smoke I/O is unchanged; the G2 controller does not invoke smoke.sh. The new identity path handles quotes/spaces without shell interpolation.

### Independent focused verification

- **21 selected tests passed normally; 21 passed with PYTHONOPTIMIZE=1.** Selection includes all unique-mount parser/controller/cleanup tests, changed-boot and same/different-unique phase tests, retained G2 full-boundary and G3 partial models, and physical-alias/stacked-mount refusal. All kernel/monitor/process mutation boundaries are mocked. Outer fail-fast guards cover os.kill, os.killpg, pidfd_send_signal and Popen.send_signal.
- Additional independent mocked probes accepted exact integer values 1, 2^32+17, 2^63+19 and 2^64-1 without precision loss. They rejected a root/covering-parent response for a non-root target, a non-mount child with only a covering root row, and a legacy-only cleanup identity. An exact root response was accepted.
- Focused Python AST checks, bash -n for smoke/recovery wrappers, and bash -n of the documented retry heredoc passed. `git diff --check -- scripts devcluster current/sol-verification.md` passed.
- Reviewed hashes: mount_guard.py `9117ce280a216534f5cce9e4abf55567edce9123a5cf6ffad066aba0ec74926f`; recovery_trial.py `c3123e78dcabaf599f4bd901eacd2c227c293dfdfaaf4bbbe75c0fc80d6d61f8`; smoke.sh `56a3e17ec4eed8be672397a628d3881163a6b560707dc5ce829311fe9925b362`. Accepted oracle hash remains `d391fe4f1267a5765a33ab7b0bb27e1f8cb8e40024ebcb4a57815263e1f851f9`.
- Did not repeat oracle review, the full 83-test suite, Cargo work or native mount queries. QA reports the 640 accepted Cargo inputs and binary SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374` unchanged; this focused review did not rebuild or alter them.
- No live cluster call, mount/unmount, actual signal, daemon action, production/test source edit, Git write or subagent. Only this report was edited. Preserve fs44/meta88/data89 and the original unsuccessful G2 artifacts for the separately authorized retry.

## Phased G3 application candidate review, 2026-10-25

**CHANGES REQUIRED.** The application-only result classification is correct. The selected application proof has two reproducible false passes. Three additional preparation/path defects require correction. These findings concern the new candidate, not the accepted G2 controller or its retained native evidence.

### G3-R1 — P1: unlink-open recovery repairs lost data before checking it

- Location: `scripts/g3_client.py:226-245`, especially 235-238; descriptor checks at 250-255.
- `arm()` writes and fsyncs the unlinked descriptor. After reconstruction, `recover()` first replaces the whole saved byte range with `self.unlinked_data + suffix` and truncates to the expected length. Only then does `check()` read the descriptor. Loss of the acknowledged pre-failure bytes or size is repaired before observation. The same inode and zero link count do not establish data preservation.
- Independent local reproduction: run real `prepare()` and `arm(1)`; alter the existing zero-link descriptor; call real `recover(1)` and `require_assertions(result, CASES)`. All selected assertions pass for each of three faults: truncate to zero, overwrite all bytes, and extend size. No pathname reopen, subprocess, mount or signal is involved.
- Required correction: check the saved inode, zero link count, old size and old bytes through the retained descriptor after recovery and before its first write/truncate. Retain that pre-write observation in the external history. Then perform and check the new descriptor write/fsync.
- Regression: each of the three corruptions and a descriptor-read EIO must fail before a repair write. An unchanged descriptor must pass. The controller must return runtime 1 and create no success result after any failed descriptor assertion.

### G3-R2 — P1: later rounds stop checking earlier acknowledged sequential histories

- Location: `scripts/g3_trial.py:253-262` and `433-445`.
- `start_sequential()` resets `self.sequential` each round. `verify_at()` checks only that current list. The mixed and persistent-client histories survive across rounds, but the sequential histories from earlier rounds drop out of all mounted/cold comparisons. This contradicts the promised repeated-crash preservation of every saved history.
- Independent complete-controller reproduction: use the real local two-round controller test, with its existing mocked mount/daemon/capacity boundaries and real workers, four contenders and comparison subprocesses. At the second simulated MDS failure, corrupt `alias` in the completed round-one kernel sequential namespace. The controller still returns **0 / APPLICATION_PASS**. The real sequential oracle separately rejects that same retained round-one history with `namespace/data/link/xattr state violates acknowledged or allowed pending history`.
- Required correction: retain every admitted history path across rounds. Keep active-child bookkeeping separate if needed. Verify every completed earlier history on both surviving clients and each later cold client before reporting the round or final result.
- Regression: corrupt or remove a round-one acknowledged namespace at round two. Require runtime 1 with no result artifact. Preserve a valid two-round positive case and the incomplete-prefix exit-77 rejection.

### G3-R3 — P2: capacity parsing silently ignores valid daemon limit settings

- Location: `scripts/g3_trial.py:165-183`; actual daemon parsing is `crates/mds/src/config.rs:19-74` and limit lookup at 280.
- The daemon lowercases keys/sections and normalizes key dashes. The tooling only replaces spaces in keys. A valid setting can disappear from its budget calculation, which then uses the default 64.
- Independent mocked reproduction used 15 retained generations and the default two-round plan of 36 additional generations. Each valid configuration below sets the daemon limit to 20. Each instead produces `configured_limit=64` and passes `capacity_check(2)`:
  - `[global]` with `rmds-max-checkpoint-generations = 20`.
  - `[global]` with `RMDS_MAX_CHECKPOINT_GENERATIONS = 20`.
  - `[MDS.RUST]` with `rmds_max_checkpoint_generations = 20`.
- Required correction: match the selected daemon's supported configuration normalization and section precedence, or explicitly reject a setting that cannot be interpreted. Never silently assume 64 for these valid overrides. Keep the budget labeled an estimate and preserve the no-delete/no-automatic-budget-change rules.
- Regression: all three spellings must report limit 20 and reject this plan before admission with prerequisite 2. Cover the selected standby's limit and the per-round recheck too. All native object/config command boundaries can remain mocked.

### G3-R4 — P2: selected NFSv4.1 mode cannot perform its required user-xattr workload

- Location: `scripts/g3_trial.py:324-327`; `scripts/g3_client.py:148-152,167-172`; reused `scripts/restart_history.py:102-106`.
- Both Ganesha modes force `vers=4.1`. Both call the complete mixed workload, which requires `user.recovery` and `user.empty`, before any recovery failure. Linux NFS user xattrs use the NFSv4.2 extension. NFSv4.1 cannot satisfy these mandatory cases. The current tests use a local filesystem or mock the Ganesha workers, so they miss this incompatibility.
- Read-only protocol reference: `/mnt/workspace/ceph-client/fs/nfs/nfs4proc.c:3940-3955` masks minor-version-1 capabilities; `:7966-8000` rejects absent `NFS_CAP_XATTR` and uses `nfs42_proc_setxattr`. No kernel implementation was copied or translated.
- Independent offline probe captured the actual modeled mount argv with `vers=4.1`. Running real `Client.prepare()` with the unsupported user-xattr syscall modeled as EOPNOTSUPP fails in the first mixed history. This is a protocol/configuration finding, not a native NFS test result.
- Required correction: select and validate a protocol/export capability combination that supports all mandatory NFS application assertions. Treat absent support as a prerequisite. Do not silently skip xattrs or turn this setup failure into partial recovery evidence.
- Regression: check the selected mount protocol/capability contract and unsupported-feature refusal. Retain the Ganesha PARTIAL/77 ceiling; actual RESET/shared-session/reclaim qualification remains separate.

### G3-R5 — P2: post-recovery progress writes can escape through a replaced root symlink

- Location: `scripts/g3_client.py:226-234`, calling `new_file()` at 138-142; root validation occurs later through `check()`/`compare()`.
- `recover()` creates/fsyncs `root/progress-NNN` before validating the saved root directory. If the tested filesystem returns a symlink in place of that root, normal pathname resolution follows it. The later no-follow comparison cannot undo this write outside the owned namespace.
- Independent local reproduction: prepare and arm the real worker, rename its root aside, and replace the original name with a symlink to a separate temporary directory. `recover(1)` creates/fsyncs `progress-001` in that outside directory before it fails on a later lookup. This is a static substitution; it requires no race or privileged concurrent actor.
- Required correction: validate the saved root's no-follow directory identity before any recovery mutation. Use a validated no-follow directory boundary for the progress create. Apply the same rule before later round mutations. Preserve the intentional positive-symlink cases inside the namespace.
- Regression: replaced root symlinks, including a target that resembles the saved tree, must fail without creating or modifying any target entry. No result artifact may follow. Keep the valid quote/space path and positive-symlink cases.

### Confirmed behavior and scope

- `native` can return APPLICATION_PASS only after at least two rounds, all selected case assertions, mounted/cold comparisons, completed sequential workers and four lock handoffs. One round and both Ganesha modes stay PARTIAL/77. Named reply-loss/flush-ack-loss modes return prerequisite 2 before probes/workload when readiness is enabled. Disabled readiness returns 77.
- All result variants retain `full_g3_qualified=false`, `same_wire_tid_retry_proven=false` and `flush_ack_loss_proven=false`. Documentation explicitly requires owner/session/connection/op/tid-tagged commit-before-send safe-reply and FLUSH_ACK seams. Random kill and the client-side rename barrier do not qualify those windows. Required runtime hooks remain unimplemented in this tooling.
- Timing fields measure application O_EXCL/create/write/file-fsync/parent-fsync progress. `protocol_reconnect_seconds` is null. Observed monitor ACTIVE remains a separate measurement.
- Lock contenders are separate processes. They perform actual nonblocking conflicting requests, then blocking requests. They must acquire and exit after explicit release. The holder's comparison uses its retained fcntl descriptor, so it does not release POSIX locks by closing an extra descriptor. The local complete-controller tests exercise all four contenders; they are not native reconstruction evidence.
- The MDS stop path retains active gid, config/name/listener, process start and pidfd checks. The selected first standby must match the saved identity. Later rounds adopt the recovered active process and restart its ID/keyring. No global PID search selects a signal victim. Ganesha selection is restricted to its owned child and test listener12049; the system2049 instance is not a controller target.
- Native cold mounts still invoke the accepted `noshare` helper and require a fresh boot-scoped unique ID. Surviving kernel/FUSE identities and visible CephFS namespaces are rechecked. Normal cold unmount precedes the next round. Unique IDs do not replace these other guards. Replacement/uncertain mounts remain retained during failure cleanup.
- Cleanup cancels known workers/contenders before normal mount cleanup and retains service children with remaining/unvalidated test mounts. Timeouts fail the run; they do not produce a successful skipped case. The new Ganesha foreground helper preserves its recovery directory/config identity and refuses a conflicting configuration. No new reset/newfs, metadata deletion, force/lazy unmount or OSD signal path was found.
- External artifact/history checks and argv-based launches remain in use. G3-R5 identifies the additional mutation-path no-follow check needed. These checks assume the documented sole QA owner and stable local operator paths; this review adds no requirement to defend against an unrelated privileged actor.

### Independent verification and provenance

- **31 focused tests passed normally and with PYTHONOPTIMIZE=1.** Selection: all29 new G3 tests plus the two changed sequential interface controls for complete history and incomplete-not-pass. Did not rerun the unrelated full112-test suite or Rust checks.
- Outer fail-fast guards covered `os.kill`, `os.killpg`, `signal.pidfd_send_signal`, and `Popen.send_signal/terminate/kill`. Native mount, daemon, monitor and capacity boundaries were mocked. Real local descriptor, lock, worker and oracle I/O stayed under disposable `/tmp/opencode` directories. Test cleanup used normal worker commands, not signals.
- Additional independent probes established G3-R1/R2/R3/R5 as described above. G3-R4 combines read-only protocol reference with an injected EOPNOTSUPP boundary. All temporary data was removed. No stored original/retry evidence or preserved binary was changed.
- Confirmed byte-for-byte equality with HEAD for eight accepted inputs: recovery_trial.py, mount_guard.py, restart_history.py, recovery-trial.sh, smoke.sh, mount-kernel.sh, mount-fuse.sh and umount.sh. G2's controller, oracle, coldness proof and cleanup implementation remain unchanged.
- Changed Python AST checks, bash syntax checks for g3-trial.sh/ganesha.sh and scoped `git diff --check HEAD -- scripts devcluster current/sol-verification-review.md` passed. No shell script was executed for live behavior.
- Reviewed SHA256 values:
  - g3_trial.py: `433f0aae573ef9ee038a52cc4619fe93d74e870dd356b8c4cfbb6aac53deba16`.
  - g3_client.py: `c92e34053ed63975d2413feeeb250d874d6f49d7ac77635cf0a7dfd73ebfa4ab`.
  - crash-history.py: `4045a7ee02d62ebc6c5f5d1dc5a0215edfa53fea15c108bbcb3b98a6b6e00c4c`.
  - test_g3_application.py: `51c5f86313adbabc8f03f529bb919e5d14b9eb70c28b4c6c376f667828b35250`.
  - g3-trial.sh: `30b79f7f1818d1fe57087bf89d2d45003d2bf0ba479c0183d36915adc835b235`.
  - ganesha.sh: `04a1ff17baf3db7fa7db149783774b942e76c5dc119f783828e1ffb6c7120f13`.
  - VERIFICATION.md: `eda575c1fed4578ae80f406e2925ab683b741641493de6985360fb594e8ee9b0`.
- Only this report was edited. No source/test edits, builds, formatting, real signals, native mounts, daemon actions, live cluster calls, commits or subagents. This review establishes no new live state. Preserve fs44/meta88/data89, both acknowledged namespaces and all original/retry evidence. No live G3 follows from this review.

## Five-fix G3 tooling re-review, 2026-10-25

### Verdict

**CHANGES REQUIRED.** The five original behavioral defects are corrected. One residual P1 blocks scoped tooling acceptance: required version-2 evidence can be malformed and still produce `0/APPLICATION_PASS`. This verdict concerns tooling only. Prior G2 tooling and the separate accepted G2 application evidence remain valid. Full G3 remains open.

Reviewed all seven frozen correction paths in `/tmp/opencode/sol-g3-review-fixes-20261025/candidate-files.sha256.json`. This includes the changed shared `scripts/crash-history.py` and new `scripts/recovery_directory.py`. Read the author's retained red/green evidence and updated coverage contract.

### Original finding closure

| Finding | Re-review result |
|---|---|
| G3-R1 | Original repair-before-check defect is closed. `recover()` checks and logs the retained descriptor before its first mutation. It then appends at the saved length. Independent truncate-to-zero, overwrite, extension and read-EIO probes fail before any recovery write, truncate or progress create. The valid descriptor and runtime1/no-result controls pass. External-history validation has the separate residual below. |
| G3-R2 | Closed. Every admitted sequential history remains in later mounted and cold comparisons. The original complete-controller reproduction now rejects round-one corruption at the second stop, through the exact earlier-history oracle, with runtime1 and no `result.json`. Focused tests also cover earlier namespace deletion and corruption introduced only at the later cold boundary. |
| G3-R3 | Closed. Parsing follows daemon key/section normalization, precedence, comments, numeric bounds and checkpoint thresholds. Each original spelling reports limit20 and rejects 15 retained plus 36 planned generations with prerequisite2. Selected-standby, interval/record override and post-admission capacity regressions pass. |
| G3-R4 | Closed under the user's permitted explicit NFSv4.1 profile. Selected/excluded cases remain explicit. NFSv4.1 excludes user xattrs, the native17-operation history and native zero-link semantics. Native xattrs remain mandatory. The real local NFS pipeline passes with xattr calls forced to EOPNOTSUPP and returns PARTIAL/77. Wrong-profile histories fail. |
| G3-R5 | Closed for the reviewed directory-boundary contract. Mutations use retained no-follow mount/root file descriptors (FDs). Root/mount replacement checks precede mutation. The original symlink escape now leaves the outside directory unchanged. Root, contender and paused sequential replacement regressions pass. Additional post-validation and nested-component probes are recorded below. |

### G3-R6 — P1: malformed required history proofs still yield APPLICATION_PASS

**Locations:** `scripts/g3_client.py:154-190`, especially 169-189. The controller consumes this oracle at `scripts/g3_trial.py:474-478` and writes its final result at 643-661.

`load_snapshot()` checks equality between descriptor-proof values and requires an observation other than `None`. It does not validate the proof object's fields or types. Equal empty objects therefore satisfy the pre-write proof requirement. The loader also replaces each earlier manifest with the latest manifest. It checks only the final root name against the header. It does not preserve the prepare manifest's root inode or enforce the worker's action transitions.

Independent reproductions:

1. Run the real local worker through prepare, arm1, recover1, release and close. In its external history, replace every `unlinked_before_write`, `unlinked_fsync_ack` and `unlinked_observation` value with `{}`. Include both observation rows and ACK results. Keep all other fields and application data unchanged. Both `load_snapshot()` and the real cold `compare()` succeed.
2. Use the real two-round local complete-controller fixture with mocked native boundaries. At the round-two cold boundary, perform the same replacement in `kernel.jsonl` and `fuse.jsonl`. Each history contains 13 replaced proof objects. The real verifier children accept both histories. The controller returns **0 / APPLICATION_PASS** and creates its success result.
3. Change only `prepare` ACK's `manifest.entries[""].ino` by adding 1000000. Leave the final manifest and filesystem correct. The cold oracle succeeds. It never enforces the original root identity across snapshots.
4. Under `PYTHONOPTIMIZE=1`, empty objects, empty lists, `false`, a string, and an object with boolean inode, negative size, malformed digest and wrong link count all pass when substituted consistently. The changed prepare inode also passes. A repeated prepare intent/ACK pair with valid sequence numbering passes despite being invalid for `Client.prepare()`.

These reproductions establish an evidence-validation false pass. The corrected worker's descriptor-byte checks still reject the original data-loss injections. The residual does not require a lost-data fault to be repaired by that worker.

Required correction:

- Validate every required descriptor-proof object before equality checks. Require the complete inode, size, SHA256 and link-count fields with valid types and ranges. Reject booleans as integers. Enforce the selected profile's link count and stable original descriptor inode. Cross-check the matching ACK fields and permitted round transitions.
- Bind all manifests to the original prepare root name and inode. Validate each manifest's root type and profile before retaining it. A later correct snapshot must not erase an earlier identity change or malformed required proof.
- Enforce the producer's action state: one prepare, matching ordered arm/recover rounds, and valid release/close transitions. Continue to permit valid prepared/recovered prefixes at mounted comparison boundaries. Those comparisons occur before final release/close.
- Add focused loader/cold-oracle regressions for missing fields, empty/wrong-type proofs, invalid numeric/digest/link values, original-root drift and impossible action order. Run them normally and optimized. Add complete-controller malformed-proof and earlier-root-identity injections at a later cold boundary. Each must return runtime1 and leave `result.json` absent. Preserve valid native and declared NFSv4.1 controls.

### Directory boundaries and shared-producer compatibility

- Replaced the persistent worker root immediately after its final recovery validation. The progress create stayed in the retained old root FD. The outside target stayed unchanged. A later identity check rejected the replacement. This exercises the check-to-use boundary directly.
- Replaced the shared sequential root immediately after validation. Its mutation stayed FD-relative. Outside `source` and `target` bytes stayed unchanged. The next named-root check rejected the replacement. Real child startup/reload and the intent-pause continuation are also exercised by the focused controller tests.
- Replaced the contender root immediately after validation. Its open, modeled conflicting/acquired lock operations and unlock all used the original inode. The outside lock victim stayed unchanged. The positive complete-controller tests use real local independent lock processes.
- Added disposable two-level nested-directory manifests. Symlink replacement at either level before open failed before file reads. Replacement immediately after that level's validation retained the old directory FD and read only the original leaf. No outside leaf was read or modified. Retained-FD comparison can finish after such a replacement; this review does not require a global atomic namespace freeze.
- Persistent history version1, wrong profiles and an absent observation row fail. Shared sequential version2 rejects missing, boolean, zero, negative, oversized, string, float and object root IDs. It rejects version1. Legacy version0 native histories retain their standalone read-only model; the application controller rejects them before a sequential verifier command. These controls also ran optimized.

### Independent verification and provenance

- **72 focused tests passed normally; 72 passed with `PYTHONOPTIMIZE=1`.** This includes all62 G3 tests and all10 shared sequential-history tests. It is independent of the author's reported145-test runs. The retained author logs show21 red-before regressions and their green results; this reviewer did not rerun the broad145-test suite.
- Outer fail-fast guards covered `os.kill`, `os.killpg`, `signal.pidfd_send_signal` and `Popen.send_signal/terminate/kill`. Native mount, daemon, monitor and capacity boundaries stayed mocked. Real local worker, FD, lock and oracle I/O used disposable `/tmp/opencode` directories. Child cleanup used the normal client protocol. Direct probes also blocked child launch when it was unnecessary.
- The original descriptor, root-escape, three capacity and complete-controller earlier-history reproductions now reject their injected faults. The residual complete-controller reproduction above still succeeds. Additional optimized malformed-history probes and four nested-directory boundary probes completed as described.
- AST parsing passed for the five changed Python modules/tests and `scripts/tests/test_history.py`. `bash -n` passed for `scripts/g3-trial.sh`, `devcluster/ganesha.sh` and `scripts/recovery-trial.sh`. Scoped `git diff --check HEAD -- scripts devcluster current/sol-verification-review.md` passed.
- Rehashed all seven frozen candidate inputs and all20 files in the correction evidence manifest: no mismatch. Rehashed all eight accepted G2 inputs and compared them with HEAD: no mismatch. The preserved `target-sol-verification/debug/ceph-rmds` remains SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`.
- Rehashed all13 original and all139 retry evidence files: no mismatch. This is a retained-file check, not a fresh cluster observation.
- Reviewed correction SHA256 values:
  - `scripts/g3_client.py`: `3f7979ce8f71e33d0f06f5f2270c9fc5e64ab0db6db0c7ca15e885292b15b7f9`.
  - `scripts/g3_trial.py`: `794858a5b0f23b2e717cc7dfab437305b3de418566483521dccab8ba3376210f`.
  - `scripts/crash-history.py`: `463546e82babced933b90cb6d6d5c78b521c00c86b8a2447d1f6bad4163cb7c1`.
  - `scripts/recovery_directory.py`: `c2193361609e69ecf7444ce405300116d71fbfeb407c900a4e2160aa375d624d`.
  - `scripts/tests/test_g3_application.py`: `74528d520b25779b27988971614775119070658a0df7e00f6f0d5c9164211c52`.
  - `scripts/VERIFICATION.md`: `ecae8a60e46def2364dfe23e1997a4f2ed5c66c7e62571001c946f16cdd9583c`.
  - `current/sol-verification.md`: `7285336bfec78a860ca3dd34d1991d3bfe37072ea76cf91f252302417efc3854`.

### Result scope

The result contract remains correctly scoped: native at least two complete rounds can report APPLICATION_PASS/0; a single round or Ganesha stays PARTIAL/77; prerequisites return2; post-admission failures return1. All selected assertions precede final success. G3-R6 must prevent malformed evidence from satisfying those comparisons.

Exact same-wire-tid safe-reply loss, FLUSH_ACK loss, protocol OPEN timing and Ganesha reclaim qualification still require unimplemented owner/observer seams. The application result retains explicit false/null qualification fields. The proposed interface is not implemented evidence.

Only this report was edited. No production/test edit, actual process signal, live call, mount, daemon action, build, commit or subagent occurred. Preserve fs44/meta88/data89, both acknowledged namespaces, the old binary and all retained evidence. Live G3 still needs accepted runtime/tooling/binary provenance and explicit sole-QA authorization.

## G3-R6 correction re-review, 2026-10-25

### Verdict

**ACCEPT corrected G3 application tooling at the frozen candidate below. G3-R6 is closed.** No remaining scoped blocker was found. G3-R1 through G3-R5 remain closed. Prior G2 tooling and application-evidence acceptance remain valid. This is tooling acceptance; full G3 remains open.

### Verified correction

- `scripts/g3_client.py:155-171` validates exact descriptor-proof fields before equality. Inode is a nonboolean integer in `[1, 2^64)`. Size is a nonboolean integer in `[0, 2^63)`. SHA256 is exactly 64 lowercase ASCII hexadecimal digits. Link count is integer 0 for native or integer 1 for NFSv4.1. Missing/extra fields and wrong types fail.
- `load_snapshot()` retains the prepare descriptor inode. Every later proof uses that inode. The external pre-write observation precedes its matching ACK and equals the last validated state. Prepare/recover flat fields must agree with the typed proof. The size and digest must match the fixed workload bytes for the acknowledged round.
- Every prepare/recover ACK requires a manifest. Each root retains the header name/profile and original prepare inode/type. Root attributes have explicit integer bounds. A later valid manifest cannot erase an earlier invalid root. Native manifests exclude the unlinked pathname. NFSv4.1 manifests retain the linked file with a matching descriptor proof.
- The reader enforces one prepare, ordered matching arm/recover rounds, optional release and one terminal close. Direct close from ready is valid. Prepared/recovered comparison prefixes remain valid before release/close. Pending and dirty prefixes fail. Sequence/round values reject boolean and floating-point lookalikes. Release and close ACKs have checked values.
- Header/record field-position checks reject proofs, manifests and rounds outside their producer-defined positions. Action ACKs cannot hide extra proof fields or substitute a manifest. Required profile/exclusion checks remain explicit. Native xattrs remain mandatory; the declared NFSv4.1 profile performs no xattr calls.
- AST source-segment comparison against the prior frozen archive found only `load_snapshot()` changed among pre-existing definitions. The three reader helpers and `re` import are new. The entire worker class, `compare()`, contender and CLI retain identical source segments. Controller, shared sequential producer and directory helper retain identical file hashes. The sole modified pre-existing test replaces an incomplete synthetic fixture with real worker output and retains both pending/dirty refusal assertions.

### Original reproductions and independent evidence

1. **Empty and wrong-type proofs:** Re-ran the original real-worker history transformation in both Python modes. Replacing all proof copies with `{}`, `[]`, `false`, a string, or the original malformed field object now raises a descriptor-proof error. Additional null, true, integer and float whole-proof values also fail. The application data and other records remain intact.
2. **Prepare root drift:** Added 1000000 only to the prepare manifest's root inode, with the later manifest and filesystem correct. Both modes now reject original-root identity drift.
3. **Repeated prepare:** Inserted a second prepare intent/ACK pair and renumbered independent row copies correctly. Both modes reject `ready -> prepare`.
4. **Earlier proof positions:** Produced a valid two-round history. Replaced each of its 13 proof objects individually with `{}`, leaving all later records correct. All 13 positions fail in each mode. A later correct snapshot cannot erase the malformed proof.
5. **Complete-controller impact:** Independently ran both later-cold regressions normally and optimized. At round 2, the empty-proof case changes 13 objects in each kernel/FUSE history. The root-drift case changes only each prepare root inode by 1000000. Both real local controller pipelines fail at `round2:comparison`, through the real independent verifier, with runtime classification 1 and no `result.json`. These close the original complete-controller false-pass path.

**29 focused tests passed normally; 29 passed with `PYTHONOPTIMIZE=1`.** Selection: all 21 R6 tests plus eight adjacent controls. Those controls cover the repaired pending/dirty fixture, fsynced observation-before-write ordering, mandatory native xattrs, profile rejection, real complete two-round native success, single-round partial, NFSv4.1 partial and unimplemented wire-loss prerequisites.

The focused matrices exercise typed fields, numeric/digest bounds, stable descriptor identity, flat ACK agreement, fixed workload bytes, rounds, earlier root manifests, record positions and action states. Valid native/NFS prepared, recovered, released, closed and direct-close prefixes pass. The independent cold CLI runs in real local child processes with only mount/isolation boundaries modeled.

The first optimized attempt reached the tool's 120-second limit before completion. It is not counted as a completed run. A read-only check found no matching local oracle/worker children afterward. The full optimized rerun completed all 29 tests in 180.810 s. The normal run completed all 29 in 45.677 s. Outer fail-fast guards covered `os.kill`, `os.killpg`, `signal.pidfd_send_signal` and `Popen.send_signal/terminate/kill`. Native boundaries remained mocked. Normal client commands handled local worker cleanup.

The retained author logs record 400 failed checks in each initial red mode, then 20 failed record-position checks in each mode. Their corrected logs report 21 R6 and 166 full-tooling tests passing per mode. These are author results, separate from this reviewer's 29-test runs. The broad 166-test suite and prior archive sweeps were not repeated.

### Static checks and retained provenance

- Changed Python AST checks and scoped `git diff --check HEAD -- scripts/g3_client.py scripts/tests/test_g3_application.py scripts/VERIFICATION.md current/sol-verification.md current/sol-verification-review.md` passed.
- All seven frozen candidate/context paths matched before and after testing. Only four differ from the prior candidate: `g3_client.py`, `test_g3_application.py`, `VERIFICATION.md` and the QA ledger. Every frozen path also matches its member in the corrected archive.
- Rechecked the eight accepted G2 source hashes: unchanged. Binary and original/retry evidence preservation reuse this report's prior independent hashes, consistent with the R6 `preserved-g2-verification.json`. This pass makes no fresh binary, retained-evidence or cluster-state observation.
- Only this primary-tree report was edited. No source/test edit, build, live call, mount/daemon operation, Git write, commit or subagent was used. No test requested a real process signal through the guarded APIs.

### Frozen release inputs

Use `/tmp/opencode/sol-g3-r6-20261025/candidate-files.sha256.json` as the accepted seven-path manifest.

- Manifest SHA256: `630d3b1ce1405dfa70367357212b001c078e602321149e0243bb2f262e37356a`.
- Archive: `/tmp/opencode/sol-g3-r6-20261025/corrected-source.tar.gz`.
- Archive SHA256: `0c97bde5a4dfd2e538afa584e39c7fe201b4d4c9b1ef52c69412cf355758156d`, equal to the user-supplied hash.

| Frozen path | SHA256 |
|---|---|
| `scripts/g3_client.py` | `98fa2083497ef2da9a15f62cbf96571e91537ac916e11f669f92828bd2c7c1d4` |
| `scripts/tests/test_g3_application.py` | `aa290bc361213fb9f23f9a5b89b71fada776c2bf1417e038f9a129d1bbfffd35` |
| `scripts/VERIFICATION.md` | `f6940a96daf87e8b8a1d36153941b9ebdd58b5e37e899174a1eaa3f070a7d8d5` |
| `current/sol-verification.md` | `ba869ef607876d6dbd4f690808899fc72b7c54cfc1db7971902f7354beed857a` |
| `scripts/g3_trial.py` | `794858a5b0f23b2e717cc7dfab437305b3de418566483521dccab8ba3376210f` |
| `scripts/crash-history.py` | `463546e82babced933b90cb6d6d5c78b521c00c86b8a2447d1f6bad4163cb7c1` |
| `scripts/recovery_directory.py` | `c2193361609e69ecf7444ce405300116d71fbfeb407c900a4e2160aa375d624d` |

### Scope retained

Native at least two complete rounds can return 0/APPLICATION_PASS. A single native round or Ganesha returns 77/PARTIAL. Prerequisites return 2; post-admission evidence/runtime failures return 1 without a success result. Exact same-wire-tid safe-reply loss, FLUSH_ACK loss, protocol OPEN timing and reclaim qualification still require implemented and independently reviewed seams. No new fault interface exists in this candidate.

Preserve fs44/meta88/data89, both acknowledged namespaces, the old binary and all retained evidence. Live G3 remains subject to accepted runtime/binary provenance and explicit sole-QA assignment.
