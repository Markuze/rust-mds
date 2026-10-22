# sol-verification
Task: Establish G0 baseline, implement verification CI and benchmark scripts, and prepare external crash-history gates. Own scripts/**, devcluster/**, .github/** and this ledger only. Operate the live cluster exclusively until ownership transfers.
Status: Unique-mount identity correction implemented and ready for independent review. Capture boot UUID + kernel non-recycled 64-bit UNIQ-ID for cold proof and cleanup; unsupported interface fails preflight. 83 focused tests pass normally and optimized under fail-fast signal guards. All640Cargo inputs, accepted oracle and binary330cf3ee remain unchanged. No live retry; fscid44/meta88/data89 preserved. Complete G2 remains not passed.
Context: Requested base f54d298. Other agents modify the tree concurrently. No commits, checkouts, resets, or child agents permitted. Keep secrets out of artifacts.
Next: Parent obtains independent acceptance of this correction, then explicitly authorizes same-fs44 restart/retrial. Wait until both steps occur. Do not use newfs/reset or overwrite old trial evidence. Use private mount namespace, approved binary and a new artifact directory. No crate edits, commits, subagents or full G3. Retain exclusive cluster ownership.
Log:
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
