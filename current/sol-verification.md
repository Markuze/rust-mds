# sol-verification
Task: Establish G0 baseline, implement verification CI and benchmark scripts, and prepare external crash-history gates. Own scripts/**, devcluster/**, .github/** and this ledger only. Operate the live cluster exclusively until ownership transfers.
Status: G3-R6 correction complete and ready for independent re-review. All 21 R6 tests and all 166 Python tooling tests pass normally and optimized under signal guards. Both later-cold complete-controller injections now classify as runtime 1 with no result.json. Typed descriptor/ACK/byte/round checks, original root/descriptor lineage and action states reject the captured false passes. R1–R5 remain independently closed. Worker I/O, controller, shared sequential code, 8 accepted G2 sources, reviewed binary, 13+139 retained evidence files and 20 prior correction artifacts remain unchanged. No live action, crate edit, build, commit or subagent.
Context: Requested base f54d298. Other agents modify the tree concurrently. No commits, checkouts, resets, or child agents permitted. Keep secrets out of artifacts.
Next: Parent independently re-reviews the four-path R6 correction and sealed evidence in /tmp/opencode/sol-g3-r6-20261025/. The fresh candidate hash manifest includes three unchanged context modules. Tooling is not accepted yet. Keep G3 live, upgrade and fault-interface work held until separate authorization. Preserve fs44/meta88/data89, both acknowledged histories, old binary and all evidence. Sole cluster ownership remains here until transfer.
Log:
- [2026-10-25] Final R6 verification: 21 focused tests and 166 full Python tooling tests pass normally and optimized. Both archived green two-round injections fail at round2:comparison with oracle/runtime exit 1 and no result.json. Python AST, shell syntax, whitespace and scoped diff checks pass. Worker/comparison/contender/main bodies match the red-before source; controller/shared sequential/directory helpers remain unchanged. Rehashed 8 accepted G2 sources against HEAD, old binary 330cf3ee, 13 original+139 retry evidence files and all 20 prior correction artifacts: no change. Fresh source/evidence manifests and archives accompany logs. No native cluster observation or live action occurred. R6 candidate awaits independent re-review.
- [2026-10-25] Focused20 and full165 tests passed normally/optimized on the first R6 correction. Added one further record-position regression before its guard:10misplaced proof/manifest/round cases each falsely passed both loader and real cold CLI (20failed checks per mode). Sealed that red source separately. The reader now rejects non-producer header/record fields, not only wrong fields inside ACK results. Final21focused/166full reruns are next. Worker/controller/shared sequential code remains unchanged.
- [2026-10-25] Implemented strict version2 external-history validation only in g3_client.py. The20focused R6 tests pass normally. Required proof dictionaries have exact typed/ranged fields, canonical SHA256, profile link counts, original descriptor identity and known workload size/digest. Matching flat ACK fields and ordered arm/recover rounds are mandatory. Every prepare/recover manifest binds the original root; action states reject repeated prepare and impossible release/close sequences. Valid native/NFS prepared/recovered/released/closed prefixes pass through the real cold CLI. The baseline's one synthetic empty-root/missing-flat fixture failed the stronger schema; replaced it with actual worker output while retaining its dirty/pending refusal assertions. No worker mutation or controller source changed.
- [2026-10-25] Sealed unchanged validator plus new tests before producer edits at /tmp/opencode/sol-g3-r6-20261025/red-source.tar.gz. Focused normal/optimized:20tests,400failed checks each,0errors. Failures cover real loader/cold CLI matrices and both two-round later-cold injections. Retained both false success result.json files and injected worker histories: each empty-proof history has13replacements; prepare-root drift changes only the original root inode. Valid native/NFS prefixes and incomplete/dirty refusals pass. Baseline145tests pass normally under real-signal guards. Next implement R6 only; worker mutation behavior stays unchanged.
- [2026-10-25] Read independent R6 report at current/sol-verification-review.md:300–320. User assigns only residual external-proof validation in primary scripts and this ledger. Worker read-before-write behavior is already correct; do not change it or implement the later fault-interface proposal. Plan red-before mutations of real worker histories, real cold verification, and later-cold-boundary full controllers before validator edits. No live action is authorized.
- [2026-10-25] Final correction verification:21review regressions green normally/optimized; full145test Python suite green normally/optimized. An additional red/green parser regression covers Python-only control/Unicode line splitting and CR newline translation; those must not invent daemon settings. Fail-fast guards cover os.kill/os.killpg/pidfd_send_signal and Popen.send_signal/terminate/kill. AST and shell syntax pass; owned diff check passes. Rehashed8accepted G2 sources against HEAD, preserved binary330cf3ee and13original+139retry evidence files: all unchanged. No native query/mount/signal/daemon operation, Cargo command, crate edit, Git write or subagent. Candidate awaits independent re-review; evidence root /tmp/opencode/sol-g3-review-fixes-20261025/.
- [2026-10-25] Implemented producer corrections. All21red regressions now pass. Preliminary full suite144/144 passes normally. Complete local controllers now reject the exact corrupted/deleted round-one history and lost surviving-FD bytes; a later-cold-only fault is also detected. Added positive full NFS4.1 local execution with all xattr syscalls forced EOPNOTSUPP, native mandatory-xattr refusal, daemon alias/u64/threshold controls, pre-write external-log ordering, and real-root inode replacement checks. New external histories use version2; accepted G2 controller/oracle/guard remain untouched.
- [2026-10-25] Red-before evidence captured before any producer edit:21new regressions all fail normally and optimized (20 assertion failures,1 EOPNOTSUPP error). Complete real-local two-round controller tests falsely pass round-one corruption/deletion and lost unlink-open bytes. Other regressions expose healing before read, wrong normalized limits/thresholds, NFS4.1 xattr calls, root-symlink writes and paused sequential root escape. Source/test snapshot and both red logs: /tmp/opencode/sol-g3-review-fixes-20261025/. No live boundary or real signal ran.
- [2026-10-25] Read all five independent G3 findings and the actual daemon config parser/limit lookups. User released owned files for offline correction. Chosen permitted R4 option: an explicit NFSv4.1 profile that excludes unsupported user xattrs, not an unverified NFSv4.2 capability claim. Ganesha remains PARTIAL/77. R5 correction will cover the paused sequential history root as well as the persistent worker; accepted G2 files stay unchanged.
- [2026-10-25] Completed offline R16/R17 harness candidate. Baseline83 tests passed before implementation; final112 tests passed normally and optimized. Real local end-to-end controller tests run persistent workers, four contenders, reused acknowledged/uncertain oracles and independent comparison subprocesses, with explicit mount/daemon/capacity boundaries and fail-fast real-signal guards. Found/fixed a POSIX-lock hazard: holder-side comparison opened/closed its fcntl inode, which released the lock. Comparison now uses retained descriptors; a separate real contender regression proves exclusion. Shell syntax/Python AST/owned diff checks passed. Accepted G2 controller/guard/oracle and old binary330cf3ee remain unchanged. No live cluster action or broad Rust tests/builds ran. Evidence: /tmp/opencode/sol-g3-tooling-20261025/.
- [2026-10-25] User assigned offline G3 harness completion after independent G2 acceptance. Read current/astra-g3-integration.md R16/R17, current/astra-g2-evidence.md, existing partial G3 semantics and isolated Ganesha recovery mode. Preserve the original binary, fs44/meta88/data89 and both acknowledged histories. Sole live ownership remains here, but no live G3 trial is authorized.
- [2026-10-25 17:47 UTC] Final evidence audit passed. Normal cold-client unmount preceded pidfd TERM of verified MDS2663998/gid16766 at17:44:47UTC; it stopped at17:44:48UTC. Released namespace holder only after both cleanup steps. Final read-only control: Ready, owner epoch5/incarnation354/gid16766, tail165, checkpoint15/covered165/trim165,15retained generations of64,7sessions Closed. Both namespaces are present in the materialized checkpoint;20distinct acknowledged data objects match manifests. Original mon/mgr/3OSDs remain alive,17PGs active+clean; no MDS/standby/FUSE/CephFS mounts. Rehashed640Cargo inputs,7accepted tools,binary/config and13original evidence files: unchanged. Only this ledger changed during the native run. No tests/build/newfs/reset/G3/crate edits/commits/subagents. Scoped application result awaits independent acceptance.
- [2026-10-25 17:43 UTC] Accepted private-namespace controller exited0 and wrote g2-trial/result.json PASS. Record->verify->allocate16->verify(future) all passed. Normal initial unmount and TERM/restart completed; recovered PID2663998/gid16766 loaded0Open sessions and reached replay->reconnect->rejoin->active. Active5.472680123s; first defined post-recovery create/write/fsync probe5.868492369s; total6.073532077s. Original history also verified through the initial mount before workload. Initial/cold legacyID430 is the same, boot UUIDcf0e3d48-7435-41f6-93ef-a8c06fd6d749 unchanged, UNIQ-ID2147565794->2147565795. Owned namespace holder2663721/start138858231 retains mnt:[4026533527] for guarded cleanup. No G3/fault-matrix claim; independent result review remains required.
- [2026-10-25 17:42 UTC] Started unchanged reviewed binary at17:41:26UTC as owned PID2662760/start138850848/gid16691. Checked active at17:41:32UTC on fs44/meta88/data89; log records replay->reconnect->rejoin->active and0restored Open sessions. Read-only pretrial control: Ready,owner epoch4/incarnation348/gid16691,tail83,checkpoint8/covered83/trim83. No CephFS mounts in visible namespaces. Artifacts: startup-owner.json,startup-active.json,startup.log,store-pretrial.json under the resume directory.
- [2026-10-25 17:36 UTC] Fresh read-only preflight passed. Evidence: /tmp/opencode/sol-g2-resume-20261025-1736/. Cluster fsid3ceca2c6-851f-41cf-b538-1cfded4f1e82, fs a/fscid44/meta88/data89 match. Original mon2312786/mgr2312859/OSDs2312891,2312954,2313017 remain alive.17PGs active+clean. No MDS/standby/FUSE/CephFS mounts in visible namespaces. Root capabilities and boot UUID/kernel UNIQ-ID capture work. Ready/tail83/cpgen7/covered82/trim82 and7retained generations of64; all5sessions Closed with nonzero receipts. Checkpoint framing/CRC passed; original namespace and both acknowledged data-object hashes match. Published HEAD,640Cargo inputs,7accepted-tool files,binary330cf3ee and13original evidence files match. No live mutation or test/build ran in preflight.
- [2026-10-22] Created ledger before other edits.
- [2026-10-22 17:26 UTC] Inspected cluster: mon/mgr/three OSDs up; 17 PGs active+clean; no mounts or MDS. Found orphan fuse PID 2974929 from Oct 12, no mount in its namespace; sent TERM. Recorded original fscid 37, metadata pool 74, data pool 75. Initial source diff was empty; concurrent test edits appeared during compilation. Logs and before/after diffs: /tmp/opencode/sol-g0/. Initial tests passed 347/0/2. Initial fmt passed.
- [2026-10-22 17:30 UTC] Diagnosed C++ bad_method: CEPH_KEYRING exported by env.sh selects client.admin keyring for mds.a. --show-config-value keyring reproduces this. Unsetting the variable or explicit --keyring selects mds.a. Startup now passes explicit MDS keyring. No keys copied into artifacts.
- [2026-10-22 17:33 UTC] Strict clippy failed: generated denc features trigger 46 identity_op diagnostics; engine triggers for_kv_map and collapsible_if. Native rados_live passed, including blocklist add/rm. First smoke reached kernel I/O success, then failed with shell EOF because I edited the running script. This is invalid smoke evidence, not a daemon failure. Do not edit scripts during their execution. Initial orphan TERM lacked permission; retried through sudo. Archived base f54d298 under /tmp/opencode/sol-g0/base-source for immutable baseline checks. The first archive test command accidentally used the working-tree manifest (base-unit.log); immutable-unit.log uses the explicit archived manifest.
- [2026-10-22 17:32 UTC] C++ startup with explicit keyring reached active in ~4 seconds. G0 C++ filesystem: fscid 39, metadata pool 78, data pool 79. Kernel smoke passed. Metadata benchmark ran 100 operations per class. Sequential acknowledged-history record/verify passed without a restart (oracle self-check only). Ganesha helper and NFSv4.1 mount/smoke passed; helper stopped and mount removed. System Ganesha on 2049 was not changed. G3 reclaim remains unqualified because helper uses graceless mode.
- [2026-10-22 17:33 UTC] Four offline history-oracle tests passed. Shell syntax checks passed. Recovery controller dependency guard returned 77 without signaling a process. Live orchestration itself remains untested and returns 77 even after successful partial verification; it is not full G2/G3 evidence. Public CI is strict and currently blocked by baseline clippy diagnostics. No baseline warning allowances were added.
- [2026-10-22 17:35 UTC] Invalidated the first archived-source evidence: the mistaken working-tree test command shared target-sol-verification-baseline with the archive. Cargo reused binaries compiled from the working tree because archive mtimes were older. The archive's objecter test log included new test names absent from its source. Stopped only the owned hung test process. All immutable-*.log, base-build.log, and Rust benchmark results from that target must be treated as contaminated-source evidence. Started a clean, never-shared target-sol-verification-f54d298 against the archived manifest. Initial unit.log and C++ reference evidence are unaffected. Will rerun Rust smoke/benchmark with the isolated target.
- [2026-10-22 17:37 UTC] Clean isolated f54d298 workspace tests passed: 347 passed including doctests, 0 failed, 2 ignored (dump, extract_corpus). The first isolated run lacked optional capture keys; a second run supplied only the three allowlisted key variables in subprocess memory and PROTO_CAPTURES pointing at the local captures. No key values entered output/artifacts. Second run passed; only optional kernel-v1-mdsv1 capture/corpus remains missing. f54d298-unit-with-local-fixtures.log is the authoritative offline result. Isolated baseline build passed. Isolated smoke/native/benchmark are in progress.
- [2026-10-22 17:40 UTC] Authoritative f54d298 original smoke passed kernel/FUSE I/O and cross-client visibility, on fscid 42 / metadata pool 84 / data pool 85. Both isolated native RADOS runs passed. Rust benchmark passed. Global sync took several minutes waiting for host writeback; modified smoke now uses sync -f on the tested filesystem. Modified script awaits its final run with the isolated binary. Baseline default clippy passed with 46 denc warnings, 2 engine warnings, 1 monc warning, and daemon warnings; strict clippy failed with exit 101. Offline CI script exercised against pinned source: exit 1, correctly preserving strict lint failure while still running unit/golden tests.
- [2026-10-22 17:42 UTC] Final modified smoke passed with RMDS_BIN pointing at the isolated f54d298 binary. Its kernel/FUSE/cross-client checks all passed. Cleanup removed both owned mounts and stopped the owned MDS. Final state: cluster fsid unchanged; fscid 43, metadata pool 86, data pool 87; mon/mgr/3 OSDs up, 17 PGs active+clean. No MDS or CephFS/NFS test mounts. Expected filesystem-offline/failed-rank health errors. Root-owned orphan ceph-fuse PID 2974929 from Oct 12 remains unmounted; user and sudo TERM attempts did not stop it. Left it rather than escalate to KILL. System Ganesha PID 991808 on 2049 remains unchanged. All temporary Ganesha helper processes and owned MDS processes stopped. No live operations in flight.

## Authoritative G0 evidence

Base: `f54d2988ebaeeb8311314fe6894ee030a80e955d`.
Archived source: `/tmp/opencode/sol-g0/base-source`.
Exclusive target: `/mnt/workspace/rust-mds/target-sol-verification-f54d298`.
Artifacts: `/tmp/opencode/sol-g0/`. Use only `f54d298-*` for authoritative Rust baseline results. The earlier `immutable-*` files are explicitly invalidated above.

Set `B=/tmp/opencode/sol-g0/base-source` and `T=/mnt/workspace/rust-mds/target-sol-verification-f54d298`.

| Command | Result | Artifact |
|---|---|---|
| `CARGO_TARGET_DIR=$T cargo test --manifest-path $B/Cargo.toml --workspace --locked -- --nocapture` | 347 passed, 0 failed, 2 ignored. Local optional keys were passed only in process memory. Optional kernel-v1-mdsv1 fixture missing. | `f54d298-unit-with-local-fixtures.log` |
| `CARGO_TARGET_DIR=$T cargo build --manifest-path $B/Cargo.toml -p rmds` | exit 0 | `f54d298-build.log` |
| `cargo fmt --manifest-path $B/Cargo.toml --all -- --check` | exit 0 | `f54d298-fmt.log` |
| `CARGO_TARGET_DIR=$T cargo clippy --manifest-path $B/Cargo.toml --workspace --all-targets` | exit 0, warnings | `f54d298-clippy.log` |
| Same clippy command plus `-- -D warnings` | exit 101 | `f54d298-clippy-strict.log` |
| `CARGO_TARGET_DIR=$T RMDS_RUN_DIR=/mnt/workspace/devcluster-data/sol-f54d298 timeout 360 bash $B/scripts/smoke.sh --keep` | exit 0, kernel/FUSE/cross-client PASS | `f54d298-smoke.log` |
| `CARGO_TARGET_DIR=$T timeout 180 cargo run --manifest-path $B/Cargo.toml -p osdc --example rados_live` | exit 0, PASS | `f54d298-rados.log` |
| `RMDS_BIN=$T/debug/ceph-rmds RMDS_RUN_DIR=/mnt/workspace/devcluster-data/sol-final timeout 300 scripts/smoke.sh` | exit 0, PASS; modified script with isolated base binary, cleanup complete | `final-script-smoke.log` |
| `source devcluster/env.sh; timeout 160 devcluster/start.sh --mds cpp` | exit 0, active with msgr2 enabled | `cpp-start.log` |
| `timeout 120 devcluster/smoke-test.sh /mnt/workspace/devcluster-data/sol-kernel` | exit 0 | `cpp-smoke.log` |
| `devcluster/ganesha.sh start`; NFSv4.1 mount on port 12049; `timeout 120 devcluster/smoke-test.sh /mnt/workspace/devcluster-data/sol-nfs` | start/mount/smoke all exit 0; helper then stopped | `ganesha-start.log`, `ganesha-smoke.log` |
| `sudo timeout 120 python3 scripts/metadata-bench.py MOUNT --label cpp\|rust --count 100` | both exit 0 | `cpp-bench.json`, `f54d298-rust-bench.json` |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests` | 4 pass; also passed with PYTHONOPTIMIZE=1 | tool output |
| `bash scripts/recovery-trial.sh g3 0 /not-mounted /tmp/opencode/sol-g0/probe -- true` | exit 77, no signal sent | tool output |

Peers: Ceph 20.2.0 commit `69f84cc2651aa259a15bc192ddaabd3baba07489`; rustc 1.99.0 `b940084d7`; kernel `7.0.0-1012-ibm`; C++ reference source checkout `8db74a411f998b5e6a697cf541113b0f98c1bf9f` (not built). C++ used packaged optimized binary. Rust used dev profile (opt-level 1). These timings do not compare equivalent durability: Rust uses MemJournal. No performance-ratio claim is valid.

| Single-client workload, 100 samples | C++ p50/p99 us | Rust f54d298 p50/p99 us |
|---|---:|---:|
| create/close | 764.0 / 58376.5 | 569.4 / 865.0 |
| create + file fsync + directory fsync | 2079.0 / 25227.5 | 554.0 / 725.6 |
| 4 KiB write + file fsync | 3035.4 / 1682955.3 | 1495.8 / 2190.1 |

## Remaining limits

- G0 is not fully green: strict lint fails. CI workflow execution on GitHub was not performed. The dedicated live wrapper was not run against the concurrent modified source tree; its constituent G0 actions were exercised against isolated baseline code.
- G2/G3 remain unqualified. The controller's crash orchestration was not exercised live. The sequential oracle passed no-restart self-checks on C++ and contaminated-tree Rust only. The revised oracle has offline pending-operation outcome tests and returns 77 for incomplete histories. Concurrent/under-load crash injection, locks, dirty caps, flush acknowledgments, unlink-open, allocator state and exactly-once retries remain unqualified.
- Ganesha is installed and historical baseline NFS I/O works. A recovery/grace mode now exists, but only offline rendering has been tested. No Rust G3 reclaim pass is claimed. The system Ganesha on port 2049 was not changed.
- No commits, checkouts, resets, OSD stops, or OSD signals were performed. Source edits stay within assigned ownership.

## Delivered APIs

- `bash scripts/ci-offline.sh`: all offline checks, strict lint, visible optional-fixture skips; returns failure if any check fails.
- `RMDS_DISPOSABLE_CLUSTER=yes bash scripts/ci-live.sh`: serialized disposable G0 lane; refuses existing mounts/MDS; missing live prerequisites fail. Never use for G2/G3.
- `CARGO_TARGET_DIR=... scripts/smoke.sh [--keep] [--release]`; `RMDS_BIN=...` skips build. Added existing-mount/MDS refusal, bounded monitor queries, and filesystem-scoped sync.
- `sudo timeout 120 python3 scripts/metadata-bench.py MOUNT --label cpp|rust --count N`: JSON samples and percentiles.
- `sudo timeout 120 python3 scripts/crash-history.py record|verify --mount MOUNT --history LOCAL_JSONL`: sequential external acknowledged history; incomplete history returns 77.
- `sudo env RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh g2 PID MOUNT NEW_ARTIFACT_DIR --owned-mount --rust-filesystem --expected-fscid ID --binary PATH --keyring PATH`: executable clean-restart application gate. Exit 0 requires every workload/cold-restart/nonreuse check. G3 additionally requires RMDS_G3_READY=yes and --cold-mount; successful G3 foundation remains 77. Arbitrary restart commands are no longer supported. See the extension coverage table below.
- `.github/workflows/verification.yml`: Ubuntu 26.04/Ceph-compatible offline container; manually dispatched, protected, serialized self-hosted disposable-live job.
- `scripts/VERIFICATION.md`: procedures and explicit coverage limits. `verification-scripts.tar.gz`, `owned-files.sha256`, and `final-owned.patch` retain the implementation outside tracked secrets.

## Review corrections, 2026-10-22

Requested follow-up: Fix all four findings in current/sol-verification-review.md. Add focused offline regressions. Add usable Ganesha grace/recovery configuration without claiming Rust G3 reclaim. Preserve exclusive cluster ownership, authoritative baseline evidence, system Ganesha and OSDs. Do not run a new full Rust smoke before parent readiness.

| Finding | Fix | Offline evidence |
|---|---|---|
| P1 recovery preflight/cleanup | Replaced unsafe shell sequence with scripts/recovery_trial.py. Explicit --owned-mount required. Read-only preflight binds root kernel CephFS mount fsid/name/fscid to monitor state. PID must match active rank 0 through executable, config, daemon name, process start time and an owned listening socket. pidfd prevents reuse. No artifact/workload/unmount/signal before preflight. Cleanup runs under host lock, after workload starts, and only with matching captured mount ID. | Monitor failure, missing ownership, wrong filesystem/fscid, actual non-MDS PID, same-config standby, replacement mount, and real process-owned socket tests. Invalid preflight creates no trial directory and invokes no workload, unmount, signal or restart. |
| P2 mount path aliases | Shared devcluster/mount_guard.py resolves physical aliases and uses decoded findmnt JSON/IDs. Smoke and mount helpers reject existing mounts. Smoke cleanup checks captured ID. umount helper now resolves aliases and uses findmnt rather than escaped /proc/mounts text. | Trailing/repeated slashes, relative paths, parent symlinks, spaces, stacked mounts; real root-mount guard. Actual smoke script refuses a symlink to root through four RMDS_RUN_DIR spellings before binary/build/log creation. No fake executable commands or actual mount operations used. |
| P2 physical history isolation | Resolve physical parents before opening; reject tested-tree paths, same-device aliases, network filesystem parents, and final symlinks. Keep exclusive log creation. | Original parent-symlink escape and final-symlink exclusive-create regressions; real CLI root rejection without log creation. |
| P2 G2 clean unmount | Recovery uses bounded normal sudo umount only. Failure clears cleanup eligibility and stops before MDS signal or restart. Neither G2 nor its failure cleanup uses force/lazy detach. Partial success no longer enters error cleanup. | Busy/timeout unmount tests check the exact normal command, one attempt, no signal/restart and no cleanup escalation. |

The partial controller currently accepts root kernel CephFS mounts and IPv4 MDS listeners only. FUSE/NFS and IPv6-only target identity probes are not implemented and fail closed. It records a sequential history before stopping the MDS. Successful partial verification leaves the mount and restarted MDS running and returns 77. Its restart/verification duration is not a separate reconnect or first-success metric. Surviving-client caches can hide missing server state; full G3 needs server-visible work and an independent cold client.

The oracle models one unacknowledged sequential operation: before/after rename-over-target, hard link, xattr and unlink; pending create also permits empty/partial matching data. It validates history order and acknowledged prefixes. Complete histories check exact directory names, data, link identity/count and xattrs. Invalid outcomes fail. Valid incomplete outcomes still return 77. These are offline model tests, not live crash evidence. No concurrent history linearizer or retry-ID oracle exists.

Ganesha: `devcluster/ganesha.sh start|stop|status|print-config [smoke|recovery]`. Smoke stays the default. Recovery uses a separate persistent ganesha-recovery/recov directory, fs backend, 90s grace, 60s lease, stable node ID and cluster-scoped server owner/scope. Delegations and client object cache are disabled. Modes share helper port 12049 and refuse simultaneous startup. System port 2049 is unchanged. Render-only GANESHA_CONFIG_FSID supports offline checks without cluster files; actual starts always use the real fsid. Installed Ganesha 6.5 man pages and Ceph Client::start_reclaim/set_uuid plus the protocol reference informed this mode. NFS grace is not CephFS reconnect or rados_cluster takeover.

Verification commands, 2026-10-22 18:07 UTC:
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v`: 29 passed.
- Same tests with `PYTHONOPTIMIZE=1`: 29 passed.
- `bash -n` on recovery-trial.sh, smoke.sh, ganesha.sh, mount-kernel.sh, mount-fuse.sh, umount.sh, ci-offline.sh and ci-live.sh: passed.
- Python AST checks on revised Python modules and tests: passed.
- `git diff --check -- scripts devcluster .github current/sol-verification.md`: passed.
- Disabled recovery guard: exit 77, no probes or signals.
- No Cargo workspace tests, native platform checks, live Rust smoke, mount operations, daemon starts/stops, resets or signals during this correction pass. Smoke guard tests intentionally refuse an actual root-mount alias before any build/start/mount. Process/mount metadata inspection and Ganesha help/config rendering required no cluster changes. No further signal was sent to orphan FUSE PID 2974929; ownership was not newly established.

New correction evidence: `/tmp/opencode/sol-verification-fixes/`. Do not overwrite or reinterpret `/tmp/opencode/sol-g0/f54d298-*`. The baseline remains 347 passed, 0 failed, 2 ignored as independently validated by the reviewer.

Final read-only process/mount snapshot: original mon PID 2312786, mgr PID 2312859 and OSD PIDs 2312891/2312954/2313017 remain running. No MDS and no CephFS/NFS test mounts. System Ganesha PID 991808 and unmounted orphan FUSE PID 2974929 remain unchanged. Cluster health/fs identity was not re-queried in this correction pass. No live operations are in flight. Correction artifacts include tests, source hashes, tooling.tar.gz, owned.patch and preserved-baseline-log hashes.

## Approved native platform probe

- [2026-10-22 18:13 UTC] Parent accepted tooling re-review and platform probe review. Read current/sol-platform-review.md. Read-only inspection confirms cephfs.a.data exists as pool 87; filesystem a is fscid 43, metadata pool 86. No MDS process, no active/standby MDS entries, no CephFS/NFS mounts. Mon/mgr and all three original OSD processes remain up; 17 PGs active+clean. Expected offline/failed-rank health errors. System Ganesha and orphan FUSE unchanged. Run only --platform-probe with explicit data pool; no legacy blocklist workloads, no reset/newfs. C++ MDS is absent, so optional Ganesha startup will be deferred rather than switch filesystem formats.
- [2026-10-22 18:14:40-18:14:57 UTC] Approved probe passed with exit 0. Exact source/environment/result artifacts: `/tmp/opencode/sol-platform-live-20261022.YcfShl/`. Build used target-sol-verification, not reviewer target. Build took 16.35 seconds. Native Mon get_version returned newest=402, oldest=1; checked wait reached 402. Successful CMPXATTR compound returned [1,0,0]; wrong-owner rejection returned overall -125, three results [-125,0,0]. Successful ASSERT_VER compound returned [0,0,0]; stale-version rejection returned overall -34, three results [-34,0,0]. Raw and checked guard failures preserved exact content/tail/nonzero user_version according to the probe's checked reads.
- [2026-10-22 18:15 UTC] Full NamespaceScope::All listing completed at epoch 402: 8 pages, 0 restarts, 6 empty continuation pages, default and non-default namespace sentinels both found. Read-only PG metadata confirms pool 87 has 8 PGs with acting primaries 0,1,2. Cleanup used owner-guarded DELETE on each exact verified candidate and returned [1,0] for both. Independent admin rados stat checks reported errno 2 (ENOENT) for both. The rados CLI uses exit 1, not exit 2, for this error; an initial QA post-check used the wrong CLI-exit expectation. Corrected only the post-check interpretation against retained stderr. No extra delete or mutation was needed, and no unknown candidate remains.

Approved command (one shell execution, no prerequisite-skip flag):
```sh
source devcluster/env.sh
CARGO_TARGET_DIR=target-sol-verification timeout --kill-after=15 360 cargo run -p osdc --example rados_live -- --platform-probe --pool cephfs.a.data --conf "$CEPH_CONF" --keyring "$RMDS_KEYRING" --id rust --inc 1
```

Environment: Ceph 20.2.0 `69f84cc2651aa259a15bc192ddaabd3baba07489`, rustc 1.99.0 `b940084d7`, cargo 1.99.0 `5f94df478`, Linux `7.0.0-1012-ibm`. HEAD remains `f54d2988ebaeeb8311314fe6894ee030a80e955d`, with the concurrent modified working tree. Probe messenger bound `v1:127.0.0.1:41463/2163076927`, authenticated as mds.rust, global_id 16405, and exited after the probe. No MDS actor was launched.

Tree evidence: before/after tracked diffs, git status, complete tracked/non-ignored-file SHA256 manifests, and native source tar are retained. Native crates denc/auth/msgr/monc/osdc and Cargo.toml/Cargo.lock did not change during the run. Only unrelated `crates/store/src/sessions.rs` changed; both hashes are recorded in changed-during-run.json. This is modified-tree platform evidence, not a rerun of the immutable f54d298 baseline.

Exact own candidates, both independently absent:
- Pool 87, default namespace: `osdc-platform-probe.1290818.1792692897111588293.default.10`.
- Pool 87, namespace `osdc-platform-probe.1290818.1792692897111588293`: `osdc-platform-probe.1290818.1792692897111588293.namespaced.2`.

Important artifacts: `probe.log`, `result.txt`, `environment.txt`, `command.txt`, `native-source.tar`, `tree-before.sha256.json`, `tree-after.sha256.json`, `changed-during-run.json`, `cleanup-independent-stat.json`, `cleanup-independent-verdict.txt`, `data-pool-pg-summary.json`, filesystem before/after JSON and final cluster/process/mount state. No capture keys or keyring contents were copied.

Final cluster identity is unchanged: fsid `3ceca2c6-851f-41cf-b538-1cfded4f1e82`, fscid 43, metadata pool 86, data pool 87. Mon/mgr/three original OSDs remain running; 17 PGs active+clean. No MDS, test mounts or probe process remains. Expected offline/failed-rank health errors remain. System Ganesha PID 991808 and unmounted orphan FUSE PID 2974929 remain unchanged. No OSD signals, resets, newfs, pool changes or broad cleanup occurred.

No mandatory probe prerequisites were skipped. This qualifies the reviewed native primitives on the allowed data pool, not metadata bootstrap, format hygiene, concurrent initialization, checkpoints, durability or G1/G2/G3. The scan had a stable epoch; live MapChanged recovery was not exercised. Sentinel locators were empty, so nonempty-locator coverage remains separate. Optional Ganesha recovery-mode startup was not attempted because the existing filesystem is Rust-format and no C++ MDS is active; switching formats would require an unapproved destructive reset. Rust reclaim remains unqualified.

## Executable G2 gate and G3 foundation extension

Task: Extend accepted partial recovery tooling into the complete executable G2 clean-restart application gate and safe G3 mounted-recovery preparation. Keep reviewed protections. Do not run live Rust G2 until parent approves an integrated binary. Work only in assigned tooling paths and this ledger. Parent now reports fs43 contains C++ metadata; do not use it as a designated Rust filesystem.

The following supersedes the earlier partial controller's API and always-77 G2 behavior. Historical acceptance applies to the retained safety core; these extensions need independent review.

### Commands and preconditions

The operator must separately create/bootstrap an explicitly designated fresh Rust filesystem, start the reviewed binary, and create the owned root kernel mount. No setup/reset/newfs/pool operation exists inside the trial. Record the resulting fscid as DESIGNATED_FSCID. Do not infer it from the old fs43 environment.

```sh
source devcluster/env.sh
sudo env RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh \
  g2 "$ACTIVE_PID" "$OWNED_KERNEL_MOUNT" "$NEW_ARTIFACT_DIR" \
  --owned-mount --rust-filesystem --expected-fscid "$DESIGNATED_FSCID" \
  --binary "$REVIEWED_BINARY" --keyring "$RMDS_KEYRING"
```

- NEW_ARTIFACT_DIR must not exist; its physical parent must exist on a local external controller filesystem.
- Run as root with CAP_SYS_ADMIN and CAP_KILL. Every visible mount namespace must be readable. Any other CephFS mount prevents a G2 cold-client proof and causes preflight refusal, not an unmount.
- PID must be the active Rust rank-0 holder. The accepted config/name/listener/start-time/pidfd and root kernel mount fsid/fscid checks remain. A different designated fscid is refused before workload admission.
- --binary selects an executable named ceph-rmds and defaults to the active executable. Its hash and config hash are recorded and checked. Restart is a direct argv launch using the original active daemon ID, same config and selected --keyring. No shell restart adapter is accepted.
- --stop term is the G2 default. --stop kill crashes only the identified Rust MDS after successful normal unmount and the no-old-mount audit. No escalation follows a failed unmount.
- --active-timeout defaults to 120 seconds (1..600). Map observation rejects identity changes, stale old-gid active entries and an exited or different restart process. The daemon must itself advance recovery from actual completion events; the observer does not manufacture phase completion.

G3 command adds a cold verification target and a separate readiness acknowledgment:

```sh
sudo env RMDS_RECOVERY_READY=yes RMDS_G3_READY=yes bash scripts/recovery-trial.sh \
  g3 "$ACTIVE_PID" "$OWNED_KERNEL_MOUNT" "$NEW_ARTIFACT_DIR" \
  --owned-mount --rust-filesystem --expected-fscid "$DESIGNATED_FSCID" \
  --binary "$REVIEWED_BINARY" --keyring "$RMDS_KEYRING" \
  --cold-mount "$EMPTY_LOCAL_COLD_TARGET" --inflight
```

Optional G3 flags:
- --takeover-pid PID: use one independently launched cold Rust standby instead of launching a restart. Preflight binds its monitor entry, process/start, listener and binary checksum. A different takeover fails. The controller never signals that externally launched standby.
- --fuse-mount EMPTY_LOCAL_TARGET: create an owned foreground ceph-fuse child with the explicit admin keyring/config/filesystem. Existing arbitrary FUSE mounts remain unsupported. Adoption requires a new mount ID/type/root, owned ceph-fuse PID/start, a /dev/fuse connection and unchanged filesystem identity. Both clients verify the oracle. Installed ceph-fuse documentation confirms -f means foreground/no PID-file daemonization. No live FUSE recovery qualification ran.
- --inflight: launch an owned sequential workload, observe a fully logged pending intent and retain that prefix before stopping the MDS. Require the client to complete after recovery, then verify exact outcomes on surviving and cold clients. An observed intent is not proof that its wire request reached the MDS or committed before the crash. Same-wire-tid retry proof is explicitly false in result metadata.

### Coverage table

| Capability | Executable implementation | Offline evidence | Live status |
|---|---|---|---|
| Caller-designated Rust fs and active PID | Explicit --rust-filesystem/--expected-fscid/--owned-mount, mount fsid/fscid and active PID checks, root/capability prerequisites | Prior 29 safety regressions retained; wrong mount/PID/ownership failure order remains tested | New gate not run |
| External acknowledged workload | scripts/restart_history.py: 17 ordered intent/ack steps, fsynced external log and strict completion marker | Real local-filesystem oracle record/verify tests; incomplete/failed-fsync histories fail | Not run on Rust |
| Namespace | Nested/empty dirs, cross-directory hard link, rename overwrite, file/rmdir deletion, exact entry set | Resurrected deletes, wrong target bytes and copy-instead-of-link rejected | Pending |
| Data/metadata | File and directory fsync, deterministic multi-block data, mode/uid/gid, file size/mtime, binary and empty user xattrs, inode/link identity/count | Local byte corruption, missing xattr and link tests | Pending |
| Allocator history | All 10 allocated pre-restart inodes retained, including 4 absent/overwritten inode identities; completion cross-checks creation ACKs | Deleted/overwritten inode nonreuse predicate and lost-allocation-manifest tests | Pending |
| Clean client close | One bounded normal unmount; busy/timeout stops before signal/restart; no force/lazy path | Existing exact-command/order regressions retained | Pending |
| Cold-client proof | Audit all visible mount namespaces; no old CephFS mounts after normal unmount; remount with noshare and new mount ID | Foreign namespace and reused mount-ID failures; mountinfo space decoding | Pending |
| Restart | Direct selected Rust executable, unchanged binary/config and identity; TERM or post-unmount KILL; selected process must become new active | Stale active entry, changed identity, timeout and phase-order models | Pending reviewed integrated binary |
| G2 pass path | Cold full comparison, 16 create/write/file-fsync/parent-fsync operations, unique future IDs against all pre-restart IDs, final old+new comparison | Ordered G2 state-machine unit returns 0 only through all required boundaries; incomplete future history/reused IDs fail | No live PASS claim |
| G3 active clients | Keep primary attached, optionally create owned FUSE, MDS-only KILL, explicit restart or selected cold standby | G3 state-machine unit preserves primary and verifies a separate cold target; retained process safety tests | Preparation only |
| G3 uncertain outcomes/retries | Accepted sequential before/after/partial-create model; optional observed-intent workload must finish without client errors; exact names/link state detects duplicate visible effects | Existing sequential outcome regressions retained | No precise request-committed/reply-lost or same-tid retry window proven |
| G3 metrics | Stop-to-new-active observation and post-recovery O_EXCL write+file/parent-fsync probe completion; cold comparison | Deterministic observer ordering/metric unit model | Not measured live; first-success metric is scoped to that progress probe |
| Ganesha | Previously accepted grace/recovery mode retained unchanged | Existing render tests retained | No Rust reclaim pass |
| Fault/feature matrix beyond this gate | No checkpoint publication/trim/crash injection, owner-fence racing, range-exhaustion, locks, dirty caps, explicit cap-flush ACK, unlink-open, security/ACL xattrs, directory timestamps, concurrent history linearization | Other lanes own engine/store tests; not claimed by this tooling | Required for broader G2/G3 qualification |

### Results, timeouts and artifacts

- Exit 0: complete **G2 clean-restart application** workload, cold remount and future-nonreuse comparison passed. result.json carries coverage=clean-restart-application.
- Exit 1: runtime/consistency/timeout/cancellation failure or any failure after workload admission. Partial histories never pass or become a skipped prerequisite.
- Exit 2: bad arguments or missing/unsupported prerequisite before workload admission. No pass.
- Exit 77: readiness disabled, or successful G3 foundation with coverage=mounted-recovery-foundation and result=PARTIAL. G2 no longer returns 77 after successful completion. G3 requires separate RMDS_G3_READY.
- Cancellation/timeouts signal only newly created command/workload process sessions or controller-created daemon/client children. The old active MDS is signaled only through its validated pidfd at the explicit stop step. No PID/name victim search exists. Externally launched standby, OSDs and system Ganesha are never timeout victims.
- Normal-only cleanup stays under the host lock and acts on captured matching IDs. It first cancels an owned in-flight workload, then attempts owned normal unmounts. Busy/replaced/unvalidated mounts and corresponding children are retained for the operator. Original caller-owned MDS is not implicitly stopped on workload failure.
- Success leaves the new MDS and owned clients up. client-before.json/client-state.json, daemon.json, identity-before/after.json, stop-intent/sent.json, recovered-active.json, oracle history/output, optional in-flight/FUSE logs, future-history.jsonl, timings.json and result.json identify observed state. Error artifacts retain phase, original mount ID/path and owned PIDs. No keyring contents are copied.

### Focused offline evidence

Evidence directory: `/tmp/opencode/sol-g2-tooling-20261022/`. These are **tooling tests only**, not native/Rust restart evidence. The README states this explicitly.

- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v`: 57 passed (2026-10-22 19:29 UTC).
- Same suite with `PYTHONOPTIMIZE=1`: 57 passed.
- Python AST checks on revised tools and tests: passed.
- bash -n on recovery/smoke/CI/mount/unmount/Ganesha scripts: passed.
- `git diff --check -- scripts devcluster .github current/sol-verification.md`: passed.
- Local-filesystem tests execute the real mixed oracle. Controller phase tests inject explicit kernel/daemon boundaries; they do not launch a pretend ceph-rmds or shadow executable commands on PATH. Positive model result output is captured and is not live PASS evidence.
- The timeout isolation test starts and terminates only its own Python test children, and proves another test-owned session remains alive. No MDS/OSD or existing FUSE/Ganesha process was signaled.
- No Cargo build/test, native probe, cluster query/mutation, Rust daemon start, actual mount/unmount, newfs or reset ran in this extension pass. Root-alias guard tests intentionally refuse before any such action. Existing f54d298 and accepted native-platform evidence remain untouched.

Remaining live dependencies: independent tooling review; a parent-approved integrated binary; separate authorized fresh Rust bootstrap (not fs43 C++ metadata); a recorded designated identity; one owned root kernel mount and readable mount namespaces; later explicit G3 readiness plus optional owned FUSE/selected standby. A native real-store/bootstrap probe may be assigned separately. Do not run any of these live steps without parent readiness. Sole cluster ownership remains with this agent.

## Oracle READDIR-error correction

- [2026-10-22 19:41 UTC] Read the independent executable G2/G3 review. Confirmed shared P1: os.walk without onerror suppressed scandir errors. Added an onerror callback that re-raises every directory-enumeration error. followlinks=False and explicit symlink rejection remain. Valid empty directories still pass the real mixed oracle.
- Added the reviewer's real local-filesystem reproduction with a single quote and spaces in the synthetic mount path. A hidden `empty/unexpected-resurrected-file` first fails exact namespace comparison. Mocking only scandir at the expected empty directory now raises EIO or EACCES, instead of hiding the child. Real traversal/stat/data checks remain active elsewhere.
- Added verification entry-point regression: only local synthetic mount/isolation boundaries are injected. The actual CLI/shared oracle handles the scandir EIO and exits 1; it prints no verified success. This is not prerequisite exit 2 or partial exit 77.
- Added controller regressions for both G2's cold comparison and G3's independent cold-client comparison. Each selected comparison invokes the real shared oracle on local data, with only the targeted scandir fault. Kernel/daemon phase boundaries are explicit mocks. Both paths classify the error as runtime exit 1 and produce no result.json, timings.json, client-state.json, PASS or PARTIAL output. No result/pass artifact exists on failure.
- Replaced `test_timeout_cancels_only_its_owned_command_session` real subprocess/signal behavior with Popen/communicate/killpg mocks. It asserts start_new_session, the exact owned child group selected, timeout/reap order and no pidfd signal. The focused suite no longer needs real signals for this test.

Verification, 2026-10-22 19:41 UTC:
- `PYTHONDONTWRITEBYTECODE=1 python3 -` with unittest discovery for scripts/tests and fail-fast outer mocks for os.kill, os.killpg and signal.pidfd_send_signal: **62 passed**, exit 0.
- Same command with `PYTHONOPTIMIZE=1`: **62 passed**, exit 0.
- The runner body uses `unittest.defaultTestLoader.discover('scripts/tests')`, enters the three fail-fast signal patches, runs `unittest.TextTestRunner(verbosity=2)`, and exits 0 only on wasSuccessful(). Exact logs: `/tmp/opencode/sol-g2-oracle-correction-20261022/unit.log` and `unit-optimized.log`.
- Python AST checks for restart_history.py and changed test modules: passed.
- `git diff --check -- scripts current/sol-verification.md`: passed.
- No cluster query/mutation, actual signal, mount/unmount, daemon action, build, fresh bootstrap, newfs, reset, commit or subagent. No non-tooling source edits. Existing fs43, OSDs, system Ganesha and orphan FUSE remain untouched. Authoritative f54d298 and accepted native platform evidence remain unchanged.

Ready for re-review. Exact remaining live dependencies are unchanged: independent tooling acceptance; both daemon reviews and parent readiness; parent-designated fresh Rust setup separate from the trial (fs43 must not be used); recorded new fsid/fscid/pool identity; reviewed executable and owned root kernel client; readable mount namespaces. Full G3 additionally needs explicit RMDS_G3_READY and its still-unqualified locks/dirty-cap/flush-ack/unlink-open/precise-retry/Ganesha coverage.

## Residual directory-type / enumeration-coverage correction

- [2026-10-22 19:52 UTC] Read the reviewer's residual P1 and exact probe. Root cause: os.walk independently suppresses DirEntry.is_dir OSError and treats the entry as a file. Its onerror callback does not see this error. The old oracle checked names and later stat types, so a known directory could pass without reading its contents.
- Added five regressions **before changing production code**: hidden-child DirEntry.is_dir EIO, omitted known-empty/root visits, real CLI failure/no verified output, G2 no-PASS artifact and G3 no-PARTIAL artifact. Ran those five under fail-fast signal guards. All five failed on the prior source, reproducing the defect. Log: `/tmp/opencode/sol-g2-enumeration-coverage-20261022/regressions-before.log`.
- Minimal fix in scripts/restart_history.py: track every directory yielded by the error-propagating traversal. Require `expected_directories <= successfully_enumerated_directories` before any exact namespace or metadata comparison can succeed. Every expected directory, including root and empty directories, must be positively enumerated. Seeing its name in a parent and later statting it is insufficient. Unexpected names still fail the existing exact namespace comparison. This is a general completeness invariant, not an exception for the injected type error.
- Keep onerror re-raising scandir/open/iteration errors, followlinks=False and explicit symlink rejection. The existing valid mixed workload proves empty directories still pass. The symlink regression proves no traversal of an external target.
- New scripts/tests/oracle_faults.py wraps real scandir entries and injects EIO only from the selected entry's is_dir method; all real scanning/stat/data checks remain. This repeats the reviewer's exact class of fault without faking the entire traversal. The separate missing-visit test removes an expected root/empty-directory yield and proves coverage failure even with otherwise successful metadata checks.
- The CLI regression uses the actual entry point and real shared oracle with only synthetic local mount/isolation boundaries. Type-lookup uncertainty exits **1**, prints no verified success, and is not converted to exit 2 or 77. G2 cold and G3 independent-cold controller regressions invoke that same oracle. Both classify the error as runtime 1 and leave result.json, timings.json, client-state.json, PASS and PARTIAL output absent.

Verification, 2026-10-22 19:53 UTC:
- Before fix: five selected new regressions, **5 failed as expected**, exit 1. No actual signal/mount/daemon activity.
- After fix: `PYTHONDONTWRITEBYTECODE=1 python3 -` with unittest discovery of scripts/tests and fail-fast patches for os.kill, os.killpg, signal.pidfd_send_signal and Popen.send_signal: **67 passed**, exit 0.
- Same discovery/guards with `PYTHONOPTIMIZE=1`: **67 passed**, exit 0.
- Python AST parsing of restart_history.py, oracle_faults.py and changed test modules: passed.
- `git diff --check -- scripts current/sol-verification.md`: passed.
- Logs and source snapshot: `/tmp/opencode/sol-g2-enumeration-coverage-20261022/`. Prior baseline/native/tooling artifacts remain unchanged. Earlier 62-test pass is historical and did not prove this missing invariant.

No code outside scripts and this permitted ledger changed in this correction. No real signals, cluster calls, mount/unmount, daemon start/stop, build, bootstrap, newfs, reset, commit or subagent. Fs43 remains untouched. Tooling still requires independent re-review; the daemon runtime review needs its separate corrections and acceptance. A reviewed integrated binary, explicit parent readiness and separately authorized fresh Rust setup remain mandatory before any G2 trial. No live gate is claimed. Sole live-cluster ownership remains with this agent.

## Authorized integrated verification, 2026-10-22 20:14–20:24 UTC

Parent authorized final frozen-tree offline checks, one separate fresh Rust setup, native bootstrap, accepted complete G2 application gate, and current durable kernel/FUSE smoke. No full G3, crate edits, commits or subagents. Read crates/mds/README.md, scripts/VERIFICATION.md and accepted daemon storage/runtime/tooling review reports before execution.

Artifact root: `/tmp/opencode/sol-integrated-g2-20261022-2014/`.
HEAD remains `f54d2988ebaeeb8311314fe6894ee030a80e955d` plus the accepted modified tree. This is final accepted-tree evidence, not the immutable base baseline.

### Offline results and frozen source

All Cargo commands used `CARGO_TARGET_DIR=target-sol-verification` and --locked where applicable. Complete stdout/stderr and exact timeout-wrapped commands are retained.

| Command | Exit/result | Artifact |
|---|---|---|
| `cargo test --workspace --locked -- --nocapture` | 0; **524 passed, 0 failed, 2 ignored**, including doctests | unit.log |
| `cargo build -p rmds --locked` | 0 | build.log |
| `cargo fmt --all --check` | 0; no formatter writes | fmt.log |
| `cargo clippy --workspace --all-targets --locked` | 0, documented warnings | clippy.log |
| Same clippy plus `-- -D warnings` | 101; known baseline denc/engine diagnostics | clippy-strict.log |
| 67 focused tooling tests with fail-fast real-signal guards | 0; 67 passed | tooling-unit.log |

Only optional `kernel-v1-mdsv1.pcap` / extracted corpus remains absent and logs a skip. The two ignored tooling tests remain dump/extract_corpus. No unit/source failure was reclassified as a baseline warning. Strict failures are 46 generated denc features.rs identity_op diagnostics plus engine caps_eval.rs:130 for_kv_map and mod.rs:845 collapsible_if (the same baseline issues, at current line numbers). Default clippy also reports one monc warning and daemon dead-code warnings. No warnings were hidden or changed. No crate formatting/edit was performed.

Approved binary:
`/mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds`
SHA256: `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`.
The hash was checked before bootstrap and smoke and retained after live validation.

`accepted-source.tar`, full before/after SHA256 manifests, status and binary diff files preserve the exact tree. Crates, Cargo.toml/Cargo.lock, scripts/devcluster/CI did not change during combined offline/live execution. Only status documentation changed (AGENTS.md, HANDOVER.md, progress_ledger.md and current ledgers). `changed-during-offline.json` and `changed-during-integrated-run.json` name those paths. Environment: rustc 1.99.0 b940084d7, cargo 1.99.0 5f94df478, Ceph 20.2.0 69f84cc2651aa259a15bc192ddaabd3baba07489, kernel 7.0.0-1012-ibm. No keyring contents or capture keys were copied into artifacts.

### Separate fresh Rust setup and native bootstrap — PASS

Read-only checks found no MDS process/map actor or CephFS/NFS mounts in any visible namespace. The old disposable fscid43 used meta86/data87. OSDs remained the original PIDs with 17 active+clean PGs. The old metadata pool listed empty. Root controller capabilities CAP_SYS_ADMIN/CAP_KILL were confirmed.

Confirmed stale FUSE PID2974929 as the repository's old smoke client: exact config/id/mount/log/admin-socket arguments, cwd rust-mds, executable ceph-fuse, and no CephFS mounts in any namespace. Retained inspection/start-tick proof and used its pidfd only. TERM did not exit within 5 seconds; KILL exited. This cleaned the known old test client before setup. No unrelated process or OSD received a signal. System Ganesha PID991808 was not changed.

`devcluster/newfs.sh` ran **once**, only as the separately authorized setup operation. Result:
- Cluster fsid unchanged: `3ceca2c6-851f-41cf-b538-1cfded4f1e82`.
- **New designated Rust fscid44; metadata pool88; data pool89.**
- Native admin all-namespace listings of both pools were empty before daemon startup. Initial in/failed/info were empty.

Exact daemon startup:
```sh
source devcluster/env.sh
RMDS_LOG=info target-sol-verification/debug/ceph-rmds \
  -i rust -c "$CEPH_CONF" --keyring "$RMDS_KEYRING" --mds-join-fs a
```
It ran as owned PID1608362, gid16461. Native startup followed standby -> creating -> active (fsmap epochs331–333). Store xattr `rmds.format/rmds.phase` was `Ready`, identity fscid44/meta88, initial checkpoint generation1/covered_seq1 at `rmds.checkpoint.0.1.1.manifest`. The log confirmed checked persistent load with zero restored Open sessions. This is real native bootstrap evidence. There was no fallback empty replay. See fresh-setup.log, filesystem-fresh.json, metadata/data-fresh-objects.json, bootstrap-daemon.log, bootstrap-map-observations.json and store-bootstrap.json/raw xattrs.

### G2 application trial — BLOCKED by tooling, not passed

First invocation (g2-controller.log) exited **2 before workload admission**. The host's shared mount propagation cloned the owned initial kernel mount into eight peer namespaces. The accepted guard refused those extra namespace entries. No trial directory/history was created. A normal unmount of the owned root removed every propagated copy; read-only all-namespace audit then returned []. This prerequisite failure was not counted as a pass.

Retried without any filesystem/pool change, using a private mount namespace:
```sh
sudo env RMDS_LOG=info unshare --mount --propagation private bash -s -- \
  1608362 /mnt/workspace/devcluster-data/sol-g2-20261022-2014/mnt-kernel \
  /tmp/opencode/sol-integrated-g2-20261022-2014/g2-trial-isolated \
  /mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds "$RMDS_KEYRING"
# Inside the private namespace: create the owned root kernel mount, then:
RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh g2 "$PID" "$MNT" "$OUT" \
  --owned-mount --rust-filesystem --expected-fscid 44 --binary "$BIN" --keyring "$KEYRING"
```

Observed native progress:
- Private namespace mnt:[4026533648]. Initial kernel mount ID323, device0:60.
- Real external oracle completed **17 acknowledged mixed operations and 10 distinct allocated inode numbers**. Root: `restart-history-1629333-1792700434474727536`. External history is complete and preserved. This is pre-stop state evidence, not post-restart equality evidence.
- Normal unmount and no-old-CephFS audit succeeded. The validated Rust MDS received TERM through its pidfd.
- Direct restart PID1629466, gid16519. Native checked persistent load reported zero restored Open sessions. Monitor transitions replay336 -> reconnect337 -> rejoin338 -> active339 succeeded. No native storage/phase failure appeared.
- Cold kernel mount with noshare succeeded. Linux **reused numeric mountinfo ID323** after the old mount was destroyed. The accepted guard requires a different numeric ID and therefore raised `cold remount retained the old mount ID` before oracle comparison or future allocation.
- Trial exit **1** (g2-isolated-result.txt), phase cold_remount. There is **no result.json/PASS, no post-restart comparison and no 16-future-inode nonreuse result**. Do not claim a complete G2 pass.
- Accepted cleanup normal-unmounted the owned cold client and stopped only its own restarted daemon. Every visible namespace then had no CephFS mounts. No reset, newfs or pool change occurred in or after the trial.

This is a harness false-negative, not evidence of a failed native load. Mountinfo IDs can be recycled; the coldness proof cannot assume numeric inequality after clean destruction. I did not relax the accepted guard during live verification. Proposed minimal follow-up: use the kernel's non-recycled 64-bit unique mount identity and test same numeric ID/different unique ID versus unchanged unique identity. Read-only `findmnt --kernel=listmount --json --mountpoint / -o TARGET,UNIQ-ID` succeeds here (root unique ID2147483802), so the environment supports that path. It needs a scoped correction/re-review before another G2 trial. Preserve fscid44/meta88/data89.

Native metadata evidence after this attempt (store-after-trial.json and raw xattrs): Ready, shard owner epoch2/incarnation336/gid16519; tail36; published checkpoint generation4/covered_seq34, `rmds.checkpoint.0.1.4.manifest`; trim cursor34. Real WAL keys are retained. This shows native checkpoint publication/trim and predecessor checkpoint preservation across restart. No unsupported exact replay-record-count or post-restart equality claim is inferred from these fields.

### Current durable kernel/FUSE smoke on same filesystem — PASS

After archiving the failed G2 application evidence and confirming no MDS/mount remained, ran the current smoke separately in a private mount namespace on **the same fscid44/meta88/data89**:
```sh
sudo env RMDS_LOG=info \
  RMDS_BIN=/mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds \
  RMDS_RUN_DIR=/mnt/workspace/devcluster-data/sol-durable-smoke-20261022-2014 \
  unshare --mount --propagation private bash /mnt/workspace/rust-mds/scripts/smoke.sh
```
Exit **0**. Kernel and FUSE 8MiB I/O/read-back, metadata operations and cross-client visibility passed on the durable daemon. No newfs was called. Existing G2 evidence namespace/data was not deleted. A separate pidfd deadline watch targeted only the exact owned smoke controller if it remained at 360 seconds; the controller had already exited, so it sent no signal. Smoke cleanup removed both mounts and stopped its MDS. Logs: durable-smoke.log, durable-smoke-daemon.log, durable-smoke-result.txt and smoke-deadline-watch.json. This is normal-operation smoke, not G3 mounted takeover.

Final native control after smoke (store-final.json): owner epoch3/incarnation342/gid16577; tail83; checkpoint generation7/covered_seq82, manifest `rmds.checkpoint.0.3.7.manifest`; trimmed82. Native WAL object/remaining keys and immutable checkpoint objects remain. Ready identity is unchanged. No metadata objects were manually removed.

### Final live state and blockers

- Preserved cluster fsid above; **fscid44/meta88/data89 remain designated and must not be recreated.**
- No MDS or CephFS/NFS test mount remains in any visible namespace.
- Original mon PID2312786, mgr PID2312859 and OSD PIDs2312891/2312954/2313017 remain running. No OSD signals/restarts. Expected offline/failed-rank health errors while no MDS runs; PGs active+clean.
- System Ganesha PID991808 remains unchanged. Old FUSE PID2974929 is gone; no new FUSE process remains.
- No crate edits, commits, checkouts, resets or subagents. Release agent retains eventual commit/push ownership.
- Immediate blocker: accepted G2 harness assumes mountinfo ID nonreuse. Fix/re-review this tooling check, then resume a new trial on **the existing fs44**, not fresh setup. Native bootstrap, legal clientless replay and durable smoke have passing evidence, but the complete application G2 comparison/nonreuse gate is not yet passed.
- Strict baseline lint remains nonzero; default lint/fmt/build/tests passed. Parent/release agent must state that policy explicitly.
- No full G3 qualification: nonempty recovery gathers intentionally remain held; precise retry windows, locks, dirty caps, flush acknowledgments, unlink-open and Rust Ganesha reclaim remain unqualified.

## Boot-scoped unique mount identity correction

Task: Correct the false-negative from recycled mountinfo ID323. Preserve filesystem44/meta88/data89 and original G2/native/oracle evidence. Do not retry live until independent review accepts and parent separately authorizes same-filesystem retry. HEAD advanced through release-owned platform commit a3dcd6f; no Git write by this agent.

Exact changed paths for this correction:
- `devcluster/mount_guard.py`
- `scripts/recovery_trial.py`
- `scripts/smoke.sh`
- `scripts/tests/test_recovery_gate.py`
- `scripts/tests/test_mount_identity.py` (new)
- `scripts/VERIFICATION.md`
- `current/sol-verification.md`

The required identity is `(host boot UUID, kernel unique mount ID)`. The helper uses the host-supported read-only `findmnt --kernel=listmount --json --mountpoint PHYSICAL_PATH -o TARGET,UNIQ-ID`. It requires exactly one matching decoded physical target and a nonzero u64 JSON integer. It records a non-nil canonical boot UUID from `/proc/sys/kernel/random/boot_id`, then checks that UUID and legacy mount metadata again after capture. Legacy ID, target, source, filesystem type/root and device remain useful context, but legacy ID inequality no longer establishes coldness.

Preflight captures this required unique identity before workload, artifact creation, cleanup arming, pidfd signaling or any unmount. Unsupported/missing/timed-out/malformed interface responses fail closed as a prerequisite (exit2). There is no fallback to legacy IDs. The same uncertainty after workload admission is runtime exit1, with no result artifact. A different boot UUID is not accepted as proof of a new mount.

The complete G2 path retains normal unmount, the all-visible-namespace no-old-CephFS audit and explicit noshare remount. It now requires a different kernel unique ID within the recorded boot UUID. A reused legacy ID with a different unique ID is permitted only through every other G2 boundary, including full state comparison, 16 future allocations and final comparison. The same unique identity fails even if the legacy ID changes. A unique mount alone is not a superblock proof; noshare and no-old-mount conditions remain unchanged.

Primary/cold and controller-created FUSE captures and cleanup compare the boot-scoped unique identity plus metadata. A recycled legacy ID/path/source cannot authorize cleanup of a replacement mount. Smoke's shared capture/same CLI uses the same required identity, with a capability check on root before creating mounts or starting smoke I/O. Old legacy-only identity files fail closed. Matching owned identities still permit normal-only cleanup; no force/lazy path was added to G2.

Offline regressions:
- Actual `Trial.run()` (including read-only preflight, host-lock boundary, real unique JSON parser, stop/restart/remount phase sequence and final result creation) accepts recycled legacy323 only when the new unique ID differs. It reaches record -> verify -> allocate -> verify(future=True), writes all model result metadata and asserts noshare plus both no-old-mount audit calls.
- Complete controller path rejects unchanged unique identity with changed legacy ID and writes no PASS artifact. Separate phase coverage rejects a changed boot UUID.
- Malformed and unavailable unique interface responses before admission produce exit2 with no artifact directory, workload, unmount, pidfd, signal or restart. Malformed/unavailable responses at cold remount produce runtime1 and no result/timing/client-state artifacts.
- Parser tests cover missing/null/zero/negative/bool/string/float/oversized IDs; wrong targets; absent/duplicate rows; invalid JSON; missing findmnt; unsupported command; timeout; invalid/changed boot UUID; and mount replacement during capture.
- Physical alias, quote/space paths, primary and extra-client owned cleanup remain covered. Same legacy metadata with a different unique ID or boot UUID refuses cleanup. Matching identities select exactly the normal owned umount command.
- Explicit kernel/monitor/process boundaries are mocked, not fake executables shadowing PATH. The synthetic executable is only read/hash-checked and never launched. The complete-path test uses a private temporary lock file instead of touching the root-owned live lane lock. All mutations/signals are mocks; the real oracle and prior traversal regressions remain unchanged.

Verification, 2026-10-22 20:43 UTC:
- `PYTHONDONTWRITEBYTECODE=1 python3 -` with unittest discovery of scripts/tests and fail-fast guards for os.kill/os.killpg/pidfd_send_signal/Popen.send_signal: **83 passed**, exit0.
- Same command with `PYTHONOPTIMIZE=1`: **83 passed**, exit0.
- bash -n on smoke.sh and recovery-trial.sh: passed.
- Python AST checks on changed Python modules/tests: passed.
- `git diff --check -- scripts devcluster current/sol-verification.md`: passed.
- Rehashed all **640 accepted Cargo input files** against the original integrated-QA snapshot: zero changed.
- Accepted restart oracle SHA256 remains `d391fe4f1267a5765a33ab7b0bb27e1f8cb8e40024ebcb4a57815263e1f851f9`.
- Daemon SHA256 remains `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`.
- Current HEAD read-only: `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`. No build, Cargo test or Git write was needed for this tooling correction.

New correction evidence: `/tmp/opencode/sol-unique-mount-correction-20261022/`. Original integrated evidence stays at `/tmp/opencode/sol-integrated-g2-20261022-2014/` without reinterpretation. Its isolated G2 exit1 remains a tooling false-negative before comparison/future-allocation checks; no pass artifact exists. No current live gate is claimed.

Retry command **for after independent review and explicit parent authorization only**. First start the existing approved binary on preserved fs44 and wait for active; supply that PID. Do not run fresh setup/newfs/reset. Use a new artifact directory:
```sh
source devcluster/env.sh
sudo env RMDS_LOG=info unshare --mount --propagation private bash -s -- \
  "$ACTIVE_PID" /mnt/workspace/devcluster-data/sol-g2-retry/mnt-kernel \
  /tmp/opencode/sol-g2-retry-NEW \
  /mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds "$RMDS_KEYRING" <<'SH'
set -euo pipefail
cd /mnt/workspace/rust-mds
PID=$1 MNT=$2 OUT=$3 BIN=$4 KEYRING=$5
devcluster/mount-kernel.sh "$MNT" v1
exec env RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh \
  g2 "$PID" "$MNT" "$OUT" --owned-mount --rust-filesystem \
  --expected-fscid 44 --binary "$BIN" --keyring "$KEYRING"
SH
```

No live/cluster action, real signal, mount/unmount, daemon start/stop, crate edit, commit or subagent occurred in this correction. Filesystem44 and pools88/89 remain untouched. Sole live ownership remains with this agent. Wait for review; do not retry merely because a command is documented.

## Authorized same-filesystem G2 retry — 2026-10-25

The user resumed work and assigned this agent sole live-cluster ownership. The user authorized the accepted G2 application retry after fresh read-only checks. This section records the new result. It does not change the earlier exit-1 result.

### Outcome and retained evidence

**PASS: accepted G2 clean-restart application controller, exit 0.** Independent result acceptance is pending. This is not full storage fault-matrix qualification or G3.

Artifact root: `/tmp/opencode/sol-g2-resume-20261025-1736/`.
Controller result: `g2-trial/result.json`.
Controller exit: `controller-exit.txt`.
Detailed evidence summary: `application-evidence-summary.json`.

New workload namespace: `restart-history-2663893-1792950162527640133`.
Preserved original namespace: `restart-history-1629333-1792700434474727536`.
The retry added only the new namespace. It did not remove either namespace or any earlier evidence.

### Read-only preflight and provenance

- Published HEAD matched `a6ae05a10bfefdcea8dae75cd29597c84a3fadcd`.
- All 640 Cargo input hashes matched the accepted snapshot. Seven accepted-tool file hashes matched. The wrapper, mount helper and other source bytes also stayed unchanged in the full tree snapshots.
- Existing binary: `/mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds`. SHA256 stayed `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`. No rebuild ran.
- All 13 original evidence files matched before and after. The earlier failed trial still has no result.json. Its exit 1 remains a tooling false-negative before comparison/nonreuse.
- Cluster fsid stayed `3ceca2c6-851f-41cf-b538-1cfded4f1e82`. Filesystem `a` stayed fscid 44, metadata pool 88, data pool 89.
- Original mon/mgr/three memstore OSD processes remained alive. No MDS, standby, FUSE client or CephFS mount existed before startup. All 17 PGs were active+clean.
- Native preflight found Ready, tail 83, checkpoint generation 7/coverage 82, trim 82 and seven retained generations. All five durable sessions were Closed with nonzero cleanup receipts. Checkpoint framing/CRC and the original namespace/data-object hashes passed.
- Root capabilities and the boot UUID/kernel UNIQ-ID interface passed read-only checks before workload admission.

### Native trial

At 17:41:26 UTC, the unchanged reviewed binary started as PID 2662760, start tick 138850848, gid 16691. It loaded persistent state with zero Open sessions. Monitor phases advanced replay -> reconnect -> rejoin -> active. Pretrial native control showed owner epoch 4/incarnation 348/gid 16691, tail 83, checkpoint 8/coverage 83/trim 83.

The trial used `unshare --mount --propagation private`. Owned holder PID 2663721/start tick 138858231 retained namespace `mnt:[4026533527]` through cleanup. The initial root kernel mount used the accepted mount helper. The original saved history passed full verification before the new workload.

Exact accepted controller invocation inside that namespace:
```sh
RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh \
  g2 2662760 /mnt/workspace/devcluster-data/sol-g2-retry/mnt-kernel \
  /tmp/opencode/sol-g2-resume-20261025-1736/g2-trial \
  --owned-mount --rust-filesystem --expected-fscid 44 \
  --binary /mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds \
  --keyring "$RMDS_KEYRING"
```

- The external history completed 17 acknowledged mixed operations and recorded 10 distinct allocated inode identities, including deleted/overwritten identities.
- The controller normal-unmounted the initial client, audited all visible namespaces for old CephFS mounts, and sent TERM only to its validated MDS through a pidfd.
- It restarted the same binary as PID 2663998/start tick 138858664/gid 16766. Native load restored zero Open sessions. Monitor phases advanced replay -> reconnect -> rejoin -> active. No empty-state fallback or phase bypass ran.
- The controller audited for old mounts again, then cold-mounted with `noshare`.
- The boot UUID stayed `cf0e3d48-7435-41f6-93ef-a8c06fd6d749`. Legacy mountinfo ID **430 recycled**. Kernel UNIQ-ID changed from **2147565794** to **2147565795**. This run exercised the accepted non-recycled mount proof.
- Full saved-state comparison passed. All 16 future create/write/file-fsync/parent-fsync operations were acknowledged. Their inode numbers differ from all 10 saved/deleted identities and each other. The final old+new comparison passed. Retained histories also show that the new and future inode identities differ from the original trial's 10 identities.
- At 17:42:49 UTC, the controller wrote result=PASS and exited 0. No error.json exists. The oracle phase log is record -> verify -> allocate -> verify.
- Stop-to-observed-new-active: 5.472680123 seconds. Stop-to-first defined post-recovery create/write/fsync probe: 5.868492369 seconds. Total post-stop trial duration: 6.073532077 seconds. These are the controller's defined observations, not separate reconnect timing or proof of a same-wire-tid retry.

### Guarded cleanup and final native state

The original acknowledged history passed full verification again through the cold client. Cleanup compared the captured boot-scoped unique identity, then normal-unmounted only that client. The all-visible-namespace mount audit returned empty before MDS stop.

The first postgate cleanup wrapper failed with `ModuleNotFoundError: No module named mount_guard` because it used a relative import after nsenter. It failed before any mount probe, original-after verification, unmount or signal. A wrapper with absolute existing module/oracle paths completed guarded cleanup. No repository/controller change or gate rerun occurred. `cleanup-notes.json` records this operator-wrapper error.

At 17:44:47 UTC, cleanup revalidated the restarted MDS binary, config/name/listener, start tick, active gid, filesystem identity and namespace. It sent TERM through that PID's pidfd. The MDS stopped at 17:44:48 UTC. Cleanup released the known namespace holder only after the client and MDS stopped. No force/lazy unmount or SIGKILL ran in this resume.

Final read-only native observation, 17:45:40 UTC:
- Ready; owner epoch 5/rank 0/incarnation 354/gid 16766.
- Tail **165**, checkpoint generation **15**, covered sequence **165**, trim **165**. Manifest: `rmds.checkpoint.0.5.15.manifest`. WAL key list is empty after trim.
- Fifteen retained checkpoint generations of the default 64. No metadata/checkpoint deletion or budget change ran.
- Seven durable sessions are Closed. The two new cleanup receipts are sequence 112 and 165. The five original session-record hashes remain unchanged.
- Final manifest/state framing and CRC passed. Both namespace names occur in the materialized checkpoint. All 20 distinct surviving acknowledged data objects from the original, retry and future manifests match size and SHA256.
- Cluster/fs/pool identities remain unchanged. Original mon PID 2312786, mgr PID 2312859 and OSD PIDs 2312891/2312954/2313017 remain alive. All 17 PGs are active+clean. No MDS, standby, FUSE client or CephFS mount remains. The offline/failed-rank health errors are expected with no MDS. System Ganesha PID 2055644 started on Oct 24, before this resume. Its PID/start/arguments match the fresh preflight and final snapshots. The historical Oct 22 Ganesha PID is not today's identity.

Evidence includes `startup.log`, `startup-active.json`, `controller.log`, both full histories, oracle outputs, `restart.log`, mount identities, cleanup records, preflight/pretrial/postgate/final native control, raw final objects/xattrs/omap/checkpoints, pool listings, process/mount/cluster snapshots and before/after hashes. `provenance-after.json` confirms unchanged source/tools/binary/config and original evidence. Only this owned ledger changed during the native run.

No tests, build, crate edits, newfs, reset, pool recreation, OSD signal, cluster stop, commit or subagent ran during this resume. Strict baseline lint is historical and was not rerun. Storage failure/uncertainty matrices, host/OSD power loss, nonempty recovery gathers, mounted takeover, locks, dirty caps, FLUSH_ACK, unlink-open, precise reply-loss/same-tid retries and Ganesha reclaim remain unqualified. G3 stays held pending independent acceptance and a new assignment.

## G3 phased application tooling candidate — 2026-10-25

Task: "Turn existing partialG3 into useful explicit phased workload/gates with truthfulcoverage and boundedrealisticclienthistory." The user assigned R16/R17 tooling only. No live G3, crate edits, commits, broad Rust checks or subagents are authorized.

### Exact changed paths

- `scripts/g3_trial.py` (new): separate phased application controller over accepted Trial ownership/identity/cancellation checks.
- `scripts/g3_client.py` (new): persistent descriptors, dirty writer, both lock classes, independent contenders and exact positive-symlink oracle.
- `scripts/g3-trial.sh` (new): environment wrapper; readiness remains checked in the Python entry point.
- `scripts/crash-history.py`: optional client-side pause after a saved intent, before the syscall. Default behavior and uncertain-outcome model remain unchanged.
- `scripts/tests/test_g3_application.py` (new): 29 focused regressions, including real local complete-controller execution.
- `devcluster/ganesha.sh`: controller-owned foreground recovery mode. It keeps stable identity/recovery files, refuses unknown helpers/config aliases and never uses the system service stop path.
- `scripts/VERIFICATION.md`: exact commands, phases, capacity budget, exit/coverage table, Ganesha limits and required owner seams. Historical G2 retry wording now points to independent acceptance.
- `current/sol-verification.md`: this ledger.

The accepted `scripts/recovery_trial.py`, `devcluster/mount_guard.py`, `scripts/restart_history.py` and reviewed G2 binary stay byte-identical. The old partial G3 foundation still returns 77. Its semantics were not silently promoted.

### Implemented native gate

The new `native` mode keeps the same captured kernel mount and controller-owned FUSE process/mount attached through every failure. Two persistent worker processes retain unlinked descriptors, dirty buffers and exclusive whole-file fcntl/flock locks. Four separate contenders use the opposite native client. They must prove initial exclusion, remain blocked until explicit holder release, then acquire and exit successfully. Holder comparison must not open/close another descriptor for its fcntl inode.

Each worker reuses the full G2 mixed history/oracle. A separate exact namespace adds relative/dangling/directory symlinks with lstat/readlink checks, file-link resolution, O_EXCL, rename overwrite, hard links, binary/empty xattrs, data/fsync, truncate/grow with zero-fill checks, and post-recovery write/fsync through a zero-link open descriptor. Dirty write ACKs explicitly mean write-return-only. Their fsync/data/identity checks follow recovery; no FLUSH_ACK-loss claim follows from them.

Each round pauses both existing sequential histories at their saved rename intent, kills only the validated active MDS through its pidfd, waits for exit, releases the client-side barriers, and sends server-visible progress commands before restart. A different active gid/process is required. The selected new active becomes the next exact signal target. `--takeover-pid` selects one pre-existing cold standby for the first promotion; later rounds explicitly restart its ID/keyring. Other/changing standbys fail.

All selected worker assertions, complete sequential history exits, both continuously mounted comparisons, the noshare independent cold comparison and both lock classes are mandatory. Cold mounts require boot UUID plus a fresh kernel UNIQ-ID and normal unmount between rounds. There is no legacy-ID fallback. Histories remain under new namespaces/artifact directories. No reset/newfs or metadata deletion command exists in the controller.

Default two rounds, maximum eight. Native success after at least two rounds returns **0 / APPLICATION_PASS / mounted-recovery-application**, with **full_g3_qualified=false**. One round returns **77 / PARTIAL** even if its application cases succeed. Case I/O/assertion/timeout/cancellation failure after admission returns **1**, with no result artifact. Missing declared prerequisites return **2**. Disabled readiness returns **77** before live actions.

Per-client reconnect-progress timing is the first post-failure O_EXCL create/write/file-fsync/parent-fsync completion. First success is the minimum of those observations. ACTIVE timing remains separate. Protocol OPEN timing is explicitly null, not inferred from a cached read or ACTIVE map.

### Ganesha preparation, not full R17

`ganesha-only` restarts only the controller-created recovery helper on12049. `mds-ganesha` sends both selected signals before waiting for either process and requires at most1s signal skew. Both require separate Ganesha readiness. The helper preserves its stable node, server owner/scope, recovery directory and90s grace. It requires an existing shim, refuses different existing configuration and never controls system Ganesha on2049.

Two NFS workers exercise exact application histories, dirty writes, descriptors and lock handoffs through one helper/export. Their open descriptor stays linked: NFS silly rename is not the native zero-link FD case. A nosharecache cold NFS view checks all saved state. Bounded raw logs, configuration/recovery-root identity and recovery-tree hashes are retained. Log truncation/rotation, unknown state files and evidence overflow fail.

Both Ganesha modes always return **77 / PARTIAL** after successful selected application assertions. Shared native SessionKey/outstanding FLUSH, RESET/FINISH, actual NFS grace/reclaim RPCs, NFS unlink-open server restart and the native-CephFS-versus-NFS lock gap remain visibly unqualified. No Ganesha helper started during this assignment.

### Required owner seams and capacity

The named `reply-loss` and `flush-ack-loss` modes return prerequisite **2** before probes/signals/workload. Minimal required hooks are specified in scripts/VERIFICATION.md:
- One-shot owner/session/connection-generation/op/tid-tagged safe-reply hold/drop after real commit and before transport acceptance, with exact original completion/inode and matching retry observations.
- Matching flush-tid/cap-ID/sequence/dirty/barrier commit-before-ACK hold/drop, plus a distinct pre-commit hold for dirty-flush/grant ordering. Random kill/client intent is not the exact window.
- Tagged validated-final-chunk and final-OPEN transport-acceptance timestamps for the kernel/FUSE durable session keys.
- Tagged UUID/RESET/Closed receipt/old-address/post-fence evidence, shared native session/outstanding flush correlation, real NFS reclaim observations and the cross-protocol lock-gap observation for R17.

These are requirements for owners, not implemented/accepted daemon APIs or schema changes. This harness does not consume the pending E1/E2/D6 addendum as an accepted interface.

Preflight and every round read native retained checkpoint objects and the selected daemon configuration. Includes are refused rather than guessed. Planned extra generations are `remaining_rounds * (8 + ceil((active_timeout + case_timeout)/30)) + 4`; retained plus planned must be below the actual configured limit. The default two-round120s/120s plan reserves no storage but reports36 planned extra generations. Selected cold-standby configuration is checked too. Default limit64, last accepted count15; neither is a fresh cluster query here. Pressure stops the run. No checkpoint/WAL deletion or automatic budget change occurs.

### Offline evidence and next gate

Evidence root: `/tmp/opencode/sol-g3-tooling-20261025/`.
- Before implementation:83 focused tooling tests passed.
- Final unittest discovery of scripts/tests under fail-fast guards for os.kill/os.killpg/pidfd_send_signal/Popen.send_signal: **112 passed**, normal Python.
- Same discovery with PYTHONOPTIMIZE=1: **112 passed**. No production assertion relies on Python assert.
- Complete-controller tests run real local persistent workers, four real lock contenders, acknowledged/uncertain oracles and comparison subprocesses. Explicit injected boundaries replace native mounts, daemon signals/process identity and native capacity only. These are offline tests, not cold-client/live recovery evidence.
- Regressions cover abnormal exits, closed result channels, response sequence errors, bounded timeouts, cancellation cleanup under the host lock, failed selected assertions/no result, missing tools, same/replaced unique cold identity, busy/unvalidated mount retention, Ganesha stable identity/system-port refusal, combined signal ordering, evidence truncation, positive/invalid symlinks, read errors, retained inode substitution and permitted-but-incomplete uncertain histories.
- Shell syntax checks passed for the new wrapper, ganesha.sh and accepted recovery wrapper. Python AST checks passed for changed modules/tests. Owned git diff --check passed.
- Rehashed accepted G2 controller/guard/oracle and old binary330cf3ee: unchanged. Rehashed all139retry and13original evidence files again: unchanged. No Cargo command, native Ceph query, mount/unmount, real signal, daemon start/stop, reset/newfs, crate edit, commit or subagent ran.
- `tooling.tar.gz`, `candidate-files.sha256.json`, `preserved-g2-verification.json`, `static-checks.json` and the normal/optimized unit logs retain this exact candidate outside the working tree. They contain no keyring contents or build output.

The private-namespace setup, exact native/standby/Ganesha commands, ownership retention and result/exit contract are in scripts/VERIFICATION.md. Parent must independently review the candidate and obtain accepted G3 runtime/binary provenance before assigning live work. Keep the existing fs44/meta88/data89 and both accepted namespace histories. Sole cluster ownership remains with this agent; no new live permission follows from this report.

## All five G3 tooling review corrections — 2026-10-25

The user released the files after CHANGES REQUIRED in current/sol-verification-review.md. The earlier112-test candidate is historical and not accepted. No live operation or crate change is part of this correction.

### Exact changed paths for this correction

- `scripts/g3_client.py`
- `scripts/g3_trial.py`
- `scripts/crash-history.py`
- `scripts/recovery_directory.py` (new)
- `scripts/tests/test_g3_application.py`
- `scripts/VERIFICATION.md`
- `current/sol-verification.md`

The accepted G2 controller, oracle, mount guard, wrapper and mount/smoke helpers remain unchanged. The new directory helper has only open/validate functions. It is shared by the persistent G3 worker, independent comparison/contender and paused sequential producer. The sequential producer is shared with the old partial G3 foundation, so its default producer and legacy-read behavior need re-review too. No accepted G2 source or native metadata schema changed.

### R1: saved unlink-open bytes are proved before mutation

After failure, the worker first validates its named/pinned root. It then checks the retained file inode, expected link count, exact old size and every saved byte. A read error is a failure. It fsyncs that pre-write observation to the external log before any recovery mutation. Only then does it append a separate suffix and fsync. It does not rewrite/truncate the old saved byte range. The final check validates the new state.

External persistent-client histories now use version2. Arm/recovery observations must match the previous saved descriptor ACK. Independent comparison rejects a missing/mismatched proof. All three corruption cases (truncate, overwrite, extension) and surviving-FD read EIO fail before any repair write. A real complete-controller regression loses the kernel worker's retained unlinked bytes and requires runtime1/no result; its stderr must name the descriptor failure.

### R2: all admitted sequential histories stay cumulative

The controller separates current child bookkeeping from its retained history-path list. Every mounted comparison and each later cold comparison checks the full retained list, not just the newest round. Per-round output lists the exact cumulative history paths. A completed round-one namespace cannot be forgotten during round two.

Real local two-round controller regressions corrupt round-one kernel data or delete the round-one FUSE namespace at the second failure. Both now fail on the exact earlier history command with the oracle's acknowledged-state violation and no result artifact. Another control corrupts that earlier state only after the mounted comparisons; the later cold comparison still rejects it. Valid two-round and incomplete-prefix/77 controls remain.

### R3: capacity uses actual daemon parsing and thresholds

The reader follows crates/mds/src/config.rs CephConf::parse/get and the numeric persistence settings: ASCII lowercase sections/keys, each ASCII key space/dash mapped to underscore, both comment characters, outer double quotes, normalized duplicate overwrite and exact daemon/global precedence. Decimal u64 values allow leading plus/zeros, not suffix units. Overflow/invalid/zero values fail explicitly. Very long valid zero prefixes do not hit Python's integer-string limit.

Read UTF-8 bytes without Python newline conversion. Split only the daemon's LF/CRLF lines, not Python's additional Unicode line separators. Unsupported control separators fail rather than becoming an invented higher override. `config-separators-red.log` records the extra failing three-separator probe; its green counterpart and final full suites pass.

The actual selected generation limit, interval in seconds and record threshold drive the plan. Extra generations are `N * (ceil(128/R) + ceil((active_timeout + case_timeout)/I)) + 4`. The default remains36 for two120s/120s rounds with R16/I30. This is a conservative estimate, not a reservation. Selected standby settings are checked before admission; each round rechecks current pressure. Preflight failure is exit2; pressure after admission is runtime1/no result. No deletion or automatic budget change exists.

The three independent spelling reproductions now report limit20 and reject15retained+36planned. Tests also cover quotes/comments/duplicates/section precedence, all supported separator/case families, selected standby limits, runtime classification, actual interval/record overrides, u64 bounds, valid plus/zero forms and invalid suffix units.

### R4: explicit NFSv4.1 profile, not impossible mandatory xattrs

The chosen permitted correction keeps NFSv4.1. The mounted protocol is checked, not inferred from argv alone. A declared `nfs41` profile excludes user xattrs, native17-operation mixed history and native zero-link FD semantics. Its initial history is the explicit create/create/rename-overwrite/link/unlink profile. Exact data/link/metadata/symlink, truncate/grow, dirty write/fsync and lock cases remain selected. The NFS open descriptor remains linked as before.

Producer/oracle code makes no NFSv4.1 user-xattr calls and records no fictitious observed empty-xattr set. Native CephFS still requires all xattr operations/checks. Both modes serialize selected/excluded cases and profile; wrong-profile histories fail. A full real-local Ganesha application pipeline runs with set/get/listxattr all forced EOPNOTSUPP and remains PARTIAL/77 with xattrs explicitly false/excluded. Native EOPNOTSUPP still fails.

Ganesha RESET/shared-session/grace/reclaim and exact wire-loss windows remain unqualified. The protocol profile is not installed-peer recovery evidence. No Ganesha process started.

### R5: no-follow physical directory boundaries precede writes

Workers retain physical mount/root directory FDs. They reject a named symlink or different directory inode before preparation, each arm and recovery. Progress create and all root mutations use the pinned directory FD; files use O_NOFOLLOW. Cold traversal opens each directory boundary without following links. Intentional internal positive symlinks remain checked data. Contenders also bind the root inode before opening the lock file.

The paused sequential producer now pins its mount/root FDs and validates them after the intent pause, before rename or any later mutation. New sequential histories use version2 and save the acknowledged root inode. Comparison rejects a replaced real directory even if its bytes resemble the saved tree. The application gate requires that identity-bearing version; older native synthetic histories remain readable only under their prior standalone model.

Regressions replace roots with empty-target and lookalike symlinks, replace a real root inode, replace the mount path before preparation, and replace the sequential root during its pause. External victim sentinel/data and entry sets stay unchanged. Pre-write spies prove no descriptor truncate/write precedes failed validation. Valid quote/space paths, legitimate symlinks and POSIX lock retention still pass.

### Red/green evidence and scoped semantics

Artifact root: `/tmp/opencode/sol-g3-review-fixes-20261025/`.

- `red-source.tar.gz` and `red-source.sha256.json` preserve the reviewed producers plus the initial failing regressions before any producer edit.
- `regressions-red.log` and `regressions-red-optimized.log`:21tests,20assertion failures+1expected EOPNOTSUPP setup error in each mode. All five findings reproduce; none failed merely because a proposed API was absent.
- `regressions-green.log` and `regressions-green-optimized.log`: the same21finding regressions pass in each mode.
- `unit.log` and `unit-optimized.log`: **145 passed** in each mode, including valid real local complete native/NFS pipelines and33new regressions/controls beyond the prior112.
- All runs use fail-fast real-signal guards on os.kill/os.killpg/pidfd_send_signal and Popen.send_signal/terminate/kill. Native mount, daemon, monitor and capacity boundaries remain injected. Disposable local file/FD/lock I/O is real.
- Shell syntax, changed Python AST and owned diff checks pass. No Cargo command ran.
- `preserved-g2-verification.json`:8accepted G2 inputs match HEAD; binary SHA256 remains `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`; all13original+139retry evidence files match. No new cluster observation is claimed.

Scoped exits remain: native at least2complete rounds -> **0/APPLICATION_PASS**, never full G3; one native round or any successful Ganesha preparation -> **77/PARTIAL**; missing prerequisite before admission -> **2**; runtime/capacity/I/O/assertion/cancellation failure after admission -> **1**, no result. Deterministic reply-loss/FLUSH_ACK-loss still require owner seams and fail prerequisite2. Protocol reconnect timing remains null; application progress timing stays explicit.

The corrected source archive/hash manifest and this ledger copy accompany the logs. Parent re-review is mandatory before live assignment. Preserve fs44/meta88/data89, both acknowledged namespaces, old binary and all retained evidence. No live calls, actual signals, mounts, daemon actions, crate edits, builds, commits or subagents occurred.

## G3-R6 external-proof validation correction — 2026-10-25

The independent five-fix re-review closed R1–R5. Its new P1 R6 remains the only assigned tooling correction. The worker's actual read-before-write behavior was already correct. This package changes the external-history reader, not that behavior. No fault-interface proposal is implemented.

### Exact R6 changed paths

- `scripts/g3_client.py`: strict descriptor, root and action validation in `load_snapshot()` and three small reader helpers; one `re` import.
- `scripts/tests/test_g3_application.py`: 21 new R6 tests, real worker/cold-CLI fixtures, later-cold controller injections and one repaired old synthetic fixture.
- `scripts/VERIFICATION.md`: exact proof schema, ACK/round links, root lineage, action states and prefix contract.
- `current/sol-verification.md`: this ledger.

`scripts/g3_trial.py`, `scripts/crash-history.py` and `scripts/recovery_directory.py` retain their reviewed bytes. They appear as unchanged context in the new source manifest. AST source-segment comparison proves all pre-existing functions except `load_snapshot()` and the entire `Client` class remain byte-identical. This includes `compare()`, descriptor reads/writes, contenders and CLI behavior. No accepted G2 source changed.

### Exact accepted version2 proof schema

Each descriptor proof must be a JSON object with exactly four fields:

| Field | Type and range |
|---|---|
| `inode` | Nonboolean integer, `1 <= inode < 2^64` |
| `size` | Nonboolean integer, `0 <= size < 2^63` |
| `sha256` | String, exactly 64 lowercase ASCII hexadecimal digits |
| `nlink` | Nonboolean integer: native 0, declared NFSv4.1 1 |

Reject missing/extra proof fields, empty objects, lists, booleans, numbers, strings, nulls, floats, invalid ranges and noncanonical digests. Check types before equality. No runtime validation uses Python `assert`.

The prepare observation establishes the original descriptor inode. Every later observation and ACK retains it. Prepare/recover ACKs require `unlinked_observation` plus typed matching `unlinked_inode`, `unlinked_size`, `unlinked_data_sha256` and `unlinked_nlink`. Arm ACKs require `unlinked_fsync_ack`. Each arm/recover has exactly one prior external observation and a matching ACK `unlinked_before_write`; both equal the previous checked descriptor state. Reject proof fields in other action ACKs or in non-producer record positions.

Size/digest must also match the fixed acknowledged worker bytes: prepare `unlinked-open`; arm N `unlinked-open-round-N`; recover N appends `/recovered-N`. A consistent arbitrary digest/size cannot replace this evidence. Round ACKs require nonboolean integers in `[1, 2^64)`, starting at 1, incrementing on arm and matching on recover. Sequence numbers also require nonboolean integers in `[0, 2^64)`. Version must be integer 2, not float 2.0. Header and record fields must follow the actual version2 producer format.

Every prepare/recover ACK requires a manifest. Other actions cannot replace it. Every saved root must have the header's safe root name/profile, a directory root type and a nonboolean inode in `[1, 2^64)`. Root mode is an integer in `[0, 0o10000)`; uid/gid are integers in `[0, 2^32)`. Every later manifest retains the original prepare root identity. Later valid records cannot erase an earlier malformed proof or root change. Native manifests cannot name the unlinked descriptor. NFSv4.1 manifests must contain the linked regular file and match its inode/size/digest/link proof. ACK profile/exclusions must match the declared profile too.

### Action states and valid comparison prefixes

```text
new --prepare--> ready
ready --arm(N+1)--> armed --recover(N+1)--> ready
ready --release--> released --close--> closed
ready --close--> closed
```

Permit one prepare, ordered matching arm/recover rounds, at most one release and one terminal close. Reject repeated prepare, duplicate arm/recover, skipped/repeated rounds, release before prepare/during a dirty round, arm after release, repeated release and actions after close. Release requires a nonboolean u64 `released_ns`. Close requires `closed=true`, not integer 1. Close without release represents normal FD teardown; only the separate controller contender checks prove lock handoff.

Prepared and recovered `ready` prefixes remain valid for mounted/cold comparison before release/close. Released and closed prefixes remain readable too. Pending actions, a header without prepare and armed/dirty prefixes cannot satisfy the oracle. This does not promote incomplete histories to application success.

### Genuine red-before and corrected regressions

Artifacts: `/tmp/opencode/sol-g3-r6-20261025/`.

- `baseline.log`: all 145 pre-existing tests pass before validator edits under signal guards.
- `red-source.tar.gz` and `red-source.sha256.json`: reviewed validator plus the first 20 R6 tests, sealed before the first producer edit.
- `r6-red.log` and `r6-red-optimized.log`: 20 tests, 400 failed checks per mode, no errors. The failures include loader/cold false passes and both complete-controller false successes. Valid native/NFS prefix and incomplete-history refusal controls pass. These are failed checks/subtests, not 400 separate test methods.
- `red-controller-empty-proofs/` and `red-controller-prepare-root-drift/`: actual injected worker histories and false `result.json` files. At the round-two cold boundary, empty-proof injection changes 13 proofs per kernel/FUSE history. The other injection changes only prepare's root inode by 1000000. Both unchanged controllers returned 0/APPLICATION_PASS.
- `record-fields-red-source.tar.gz`, its hash manifest and both `record-fields-red*.log`: one later added regression, captured before its record-position guard. Ten misplaced proof/manifest/round cases falsely passed loader and cold CLI: 20 failed checks per mode. This includes fields outside the producer's ACK result/observation positions.
- `r6-green.log` and `r6-green-optimized.log`: all 21 final R6 tests pass in both modes.
- `unit.log` and `unit-optimized.log`: all 166 full tooling tests pass in both modes. Prior R1–R5 regressions, valid complete two-round native APPLICATION_PASS controls and declared NFSv4.1 PARTIAL controls remain green.
- `green-controller-*/failure.json` and `green-controller-summary.json`: both actual corrected controllers fail at `round2:comparison`; oracle exit 1, runtime classification 1, `result.json` absent. Both histories still contain the injected faults. Reader rejection does not rely on a fake comparison.
- `validator.diff` and `static-checks.json`: reader-only source diff, unchanged worker/controller/context proofs, Python AST, shell syntax and scoped whitespace/diff checks.

The matrices cover missing proof fields; every wrong proof type; boolean/numeric/range violations; empty, short, long, non-hex, uppercase and non-string digests; profile link counts; later descriptor adoption; contradictory/missing flat ACK fields and rounds; required/earlier manifest drift; impossible action states; and malformed release/close results. All negative cases exercise both the loader and the real independent cold CLI. Valid native/NFS prepared, one/two-round recovered, released, closed and direct-close prefixes pass. The NFS producer control forces all xattr calls to EOPNOTSUPP. No native-xattr requirement was relaxed.

One prior test built an intentionally incomplete synthetic prepare ACK with an empty root and missing flat fields. The stronger schema correctly rejected it before its intended pending/dirty check. That fixture now uses real prepare/arm/recover output. It still requires both pending and dirty prefixes to fail with the original incomplete/unflushed error. `baseline-progress.log` retains the intermediate failure rather than hiding it.

All test runs block real `os.kill`, `os.killpg`, `signal.pidfd_send_signal` and `Popen.send_signal/terminate/kill`. Native mount, daemon, monitor and capacity boundaries remain mocked. Disposable local files, surviving FDs, lock workers, client histories and comparison subprocesses are real. These are offline tests, not native cold-client recovery evidence.

Reproduce the sealed focused/full runs from the primary tree with `PYTHONDONTWRITEBYTECODE=1 python3 /tmp/opencode/sol-g3-r6-20261025/run-tests.py r6` or `full`. Add `PYTHONOPTIMIZE=1` for the optimized mode. The artifact runner installs the signal guards before tests execute.

### Preserved state, provenance and result scope

`candidate-files.sha256.json` freezes all seven correction/context paths. `corrected-source.tar.gz` contains those exact bytes. `evidence-files.sha256.json` seals the artifact set. The archive also retains a copy of this final ledger and test summaries. These are author evidence for independent re-review, not tooling acceptance.

`preserved-g2-verification.json` records 8 accepted G2 inputs equal to HEAD; unchanged binary SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`; all 13 original and 139 retry evidence hashes equal; and all 20 prior five-fix correction artifacts unchanged. This is a retained-file check, not a new cluster query. Preserve fsid `3ceca2c6-851f-41cf-b538-1cfded4f1e82`, filesystem `a`/fscid44/meta88/data89 and namespaces `restart-history-1629333-1792700434474727536` and `restart-history-2663893-1792950162527640133`.

Result scopes do not change: native at least two complete rounds -> 0/APPLICATION_PASS; one native round or any successful Ganesha preparation -> 77/PARTIAL; missing prerequisite -> 2; runtime/evidence/capacity/I/O/cancellation failure -> 1 with no success result. `full_g3_qualified`, `same_wire_tid_retry_proven` and `flush_ack_loss_proven` stay false; protocol reconnect timing stays null. Storage-fault qualification, exact wire-loss windows and NFS reclaim remain open.

No crate edit, Cargo command, live query, native mount/unmount, actual process signal, daemon action, reset/newfs/deletion, Git write or subagent occurred. Parent re-review must precede live or fault-interface assignment. Sole cluster ownership stays here until transfer.
