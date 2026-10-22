# sol-verification-review
Task: Independent Sol review of completed QA scripts/CI/devcluster changes vs f54d298. Review scripts/**, devcluster/** and .github/**. No live operations or source edits.
Status: ACCEPT for the unique-mount-identity correction and same-filesystem G2 retry tooling. No remaining scoped blocker. Complete G2 is still NOT PASSED; nonempty G3 remains held. Prior oracle acceptance is unchanged.
Context: Sole writable file is current/sol-verification-review.md. QA owns all live operations. Reviewed tracked diff against f54d298 and new scripts, tests, workflow, and verification instructions. No source edits, builds, commits, live queries, signals, mounts, or cluster operations.
Next: Parent may separately authorize QA to retry on preserved fs44/meta88/data89 with the accepted binary and a new artifact directory. Do not run newfs/reset or infer live gate success from this review. This reviewer performs no live actions.
Log:
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
