# Verification lanes

## Offline

Run `bash scripts/ci-offline.sh`. Rust 1.99, rustfmt, clippy, and
`ceph-dencoder` are required. The lane runs unit tests and golden tests.
The test log exposes optional capture-key skips. Public CI does not have the
capture decryption keys. Never store keys in workflow artifacts.
The optional `kernel-v1-mdsv1` capture/corpus is absent at the G0 baseline.
Its test logs a skip. The other tracked golden fixtures still run.

## G0 disposable live baseline

Run `RMDS_DISPOSABLE_CLUSTER=yes bash scripts/ci-live.sh` on the dedicated
dev-cluster host. This lane deletes the existing filesystem and pools through
`newfs.sh`. It refuses existing mounts and MDS processes. It uses a host lock
and CI concurrency control. Missing credentials or live prerequisites fail the
lane. They do not produce a successful skip.

`scripts/smoke.sh` accepts `CARGO_TARGET_DIR` and `RMDS_BIN`. `RMDS_BIN` skips
the build. Wrap the script in `timeout 300` to bound client I/O. Cleanup removes
its mounts and stops its MDS unless `--keep` succeeds.
Mount guards resolve physical path aliases and read decoded findmnt JSON.
Cleanup checks the captured boot UUID and kernel unique mount ID, plus mount
metadata, before touching a mount. A replacement mount is left unchanged. The
required unique-ID interface is checked before mounting or starting smoke I/O.

## Baseline benchmark

With a mounted filesystem and an active C++ or Rust MDS:

```sh
sudo timeout 120 python3 scripts/metadata-bench.py /mnt/test --label cpp --count 100 > /tmp/opencode/cpp-bench.json
# Switch MDS implementations only on a fresh disposable filesystem.
sudo timeout 120 python3 scripts/metadata-bench.py /mnt/test --label rust --count 100 > /tmp/opencode/rust-bench.json
```

The runner uses one client and a unique directory. It removes only its own files.
It reports individual samples and p50/p99 latency in microseconds. It measures
create/close, create/file-fsync/directory-fsync, and 4 KiB write/file-fsync.
Record filesystem/pool IDs, peer versions, source diff, build profile, client
type, and MDS durability settings beside the output. Cached create/close can use
C++ early replies. Current Rust timings do not establish durable performance.
Run equivalent clients, configurations, and build profiles before comparing.

## G2 executable clean-restart gate: live qualification pending

Fresh Rust filesystem setup is a **separate, explicitly designated operation**.
Do not run this gate on the old C++ filesystem (fs43 at the time of assignment).
The controller never calls newfs/reset or changes pools. Wait for parent approval
of the integrated daemon binary. Start that binary and create the owned root
kernel mount before this command. Save the designated new fscid separately.

```sh
source devcluster/env.sh
sudo env RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh \
  g2 "$ACTIVE_PID" "$OWNED_KERNEL_MOUNT" "$NEW_ARTIFACT_DIR" \
  --owned-mount --rust-filesystem --expected-fscid "$DESIGNATED_FSCID" \
  --binary "$REVIEWED_BINARY" --keyring "$RMDS_KEYRING"
```

The artifact directory must not exist; its parent must exist on a local
filesystem outside CephFS. `--binary` defaults to the running active executable.
The controller records its checksum and the configuration checksum. It launches
the restart directly as an argv array with the active daemon ID, same config and
selected keyring. External shell restart commands are not accepted. Do not
rebuild/change the selected binary or config during a trial. `--stop kill`
selects MDS SIGKILL **after clean client close**; the default is TERM.

Read-only preflight requires explicit mount ownership, the designated fscid, a
Rust active rank-0 PID, and matching cluster/filesystem/pool identity. It retains
the accepted executable/config/name/listener/start-time/pidfd checks. Root with
CAP_SYS_ADMIN and CAP_KILL is required to inspect mount namespaces and own
cancellation. Every visible mount namespace must be readable. Other CephFS
mounts cause refusal, not cleanup. The host lock and repeated identity checks
remain mandatory. No workload/artifact/unmount/signal precedes preflight.
Preflight also requires `findmnt --kernel=listmount` to return exactly one valid
nonzero u64 `UNIQ-ID` for the mount and a valid host boot UUID. Unsupported,
unavailable or malformed unique-ID data returns prerequisite exit 2 before
workload admission. The same failure after workload admission is runtime exit 1.
There is no fallback to recyclable `/proc/*/mountinfo` IDs.

The external fsynced oracle records 17 mixed operations and all 10 allocated
inode numbers, including deleted and overwritten inodes. It tests nested/empty
directories, file data and fsync, binary/empty xattrs, cross-directory hard links,
rename-over-target, file deletion and directory deletion. It compares exact
names, types, inode numbers, mode/uid/gid, file size/data/mtime, link identity/count
and xattrs. Partial histories cannot pass.

G2 then requires one **normal unmount**, proves no old CephFS mounts remain in
any visible namespace, stops only the validated Rust MDS, launches the selected
binary, and observes a new active MDS in the monitor map. A stale active entry
for the killed gid is not recovery. The observer polls real map state; the
reviewed daemon must advance its own phases from actual completion events.
`--active-timeout` defaults to 120 seconds. A changed identity or an exited
daemon fails, rather than becoming a delay to ignore.

Cold remount uses `noshare` and must have a different kernel unique mount ID
within the recorded boot UUID. Legacy numeric mountinfo IDs may be recycled and
are retained only as metadata, not as the coldness proof. A changed boot UUID or
the same unique mount identity fails. The no-old-CephFS-mount audit is still
required: a new mount ID alone does not prove a cold superblock. The gate verifies the full
acknowledged namespace, then creates/fsyncs 16 new files. Their inode numbers
must differ from every recorded pre-restart inode (including deleted/overwritten
ones) and from one another. A second comparison checks old and new state.
Only completion of every step writes `result.json` with `result=PASS` and exits
**0**. This is the G2 clean-restart application gate, not the storage fault matrix
or a claim that it has already passed live.
All directory enumeration errors propagate from the shared comparison oracle.
An unreadable expected-empty directory is a runtime failure, not evidence that
it is empty. The oracle also requires positive traversal coverage for every
expected directory, including root. A name seen in its parent and a successful
later stat do not establish that its contents were enumerated. Missing traversal
coverage fails even when os.walk suppresses an entry-type lookup error. Neither
G2 nor G3 writes result metadata after a comparison error.

| Exit | Meaning |
|---|---|
| 0 | Complete G2 clean-restart workload, cold comparison and future-inode checks passed. |
| 1 | Runtime/consistency/timeout/cancellation failure, or any failure after workload admission. No pass. |
| 2 | Bad arguments or missing/unsupported prerequisite before workload admission. No pass. |
| 77 | Readiness disabled, or successful G3 foundation with intentionally incomplete qualification. No full G3 pass. |

Oracle commands live in `scripts/restart_history.py`. The existing sequential
uncertain-outcome oracle remains separate below. G2 never treats its 77 result
as success.

## G3 mounted-recovery foundation: executable, intentionally partial

Use the same preflight/ownership flags with `g3` and an existing empty local
directory supplied as `--cold-mount`. The primary kernel mount stays attached.
The controller sends KILL only to the identified active MDS. It either launches
the selected binary or uses `--takeover-pid PID` for one independently launched
cold standby. Preflight binds that standby to its monitor entry, owned listener,
process start time and designated binary checksum. Unselected/raced takeover
does not pass. The controller never signals an externally launched standby.

```sh
sudo env RMDS_RECOVERY_READY=yes RMDS_G3_READY=yes bash scripts/recovery-trial.sh \
  g3 "$ACTIVE_PID" "$OWNED_KERNEL_MOUNT" "$NEW_ARTIFACT_DIR" \
  --owned-mount --rust-filesystem --expected-fscid "$DESIGNATED_FSCID" \
  --binary "$REVIEWED_BINARY" --keyring "$RMDS_KEYRING" \
  --cold-mount "$EMPTY_COLD_TARGET" --inflight
```

G3 requires the separate `RMDS_G3_READY=yes` acknowledgment. G2 readiness alone
does not enable a mounted-client crash trial.

`--fuse-mount EMPTY_LOCAL_TARGET` creates an additional owned foreground
ceph-fuse client. Existing arbitrary FUSE mounts are still refused. Ownership
requires successful controlled creation, the new mount ID/type/root, a live
ceph-fuse PID with a /dev/fuse connection, unchanged cluster identity and the
captured process start time. The installed ceph-fuse manual defines `-f` as
foreground/no daemonization. The optional FUSE client verifies the same history
before and after the crash. NFS and IPv6-only MDS identity remain unsupported.

`--inflight` launches an owned sequential client workload and observes a logged
pending intent before stopping the MDS. It preserves the observed prefix and
requires that client to finish successfully after recovery. Verification on the
surviving and independent cold kernel clients checks its final data, overwrites,
deletes and hard-link count/identity. The observed intent does **not** prove that
its wire request reached the MDS or committed before the crash. Exact same-tid
retry/reply-loss injection remains unimplemented and is explicitly false in the
result metadata. A workload that finishes before a pending intent is observed,
or an incomplete/failed client run, fails. It cannot silently establish retry coverage.

`timings.json` records stop-to-observed-new-active and the first post-recovery
O_EXCL create/write/file-fsync/parent-fsync probe completion. The latter is a
defined server-visible progress probe, not an assertion that no other client
operation succeeded earlier. Cold verification prevents surviving-client cache
state alone from establishing success. Successful G3 foundation runs still exit
77 with `result=PARTIAL` because lock/dirty-cap/flush-ack/unlink-open/Ganesha and
precise committed-retry windows remain unqualified.

Successful runs leave the restarted MDS and owned client mounts up. Their IDs
are retained in `result.json` and `client-state.json`. On failure, controller
cancellation stops only its own workload processes, then attempts normal unmount
of captured matching mounts. Busy/replaced/unvalidated mounts stay for the
operator. Only controller-created daemon/client children are eligible for
termination, and only when no test target remains mounted. PID searches never
select timeout victims. All history/logs remain; `error.json` identifies owned
children and known mounts. OSDs and unrelated system Ganesha are never signaled.
Captured primary, cold and owned FUSE cleanup identities include boot UUID and
unique mount ID. Matching legacy ID/path/source alone never authorizes cleanup.
Old identity files without these fields fail closed.

### Same-filesystem retry after the initial mount-ID false-negative

Filesystem **44**, metadata pool **88**, data pool **89** already exist and must
be preserved. Do not run fresh setup or newfs for this retry. The earlier trial
exited 1 before comparison/future-allocation checks; it has no pass artifact.
Wait for independent acceptance of the unique-ID correction and explicit parent
retry authorization. Start the approved existing binary on this filesystem and
wait for clientless active. Then use the new active PID below.

The host propagates ordinary mounts into peer namespaces. Use an isolated
private mount namespace so the accepted no-other-CephFS guard remains meaningful:

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

Choose a new artifact directory, never overwrite the original evidence. On
success the new daemon and owned cold mount remain inside the private namespace;
the existing artifact/ownership rules still apply. No live retry has run with
this correction merely because the command is documented here.

## Sequential interrupted-operation oracle

`python3 scripts/crash-history.py gate-status` exits **77** until full recovery
integration and concurrent crash coverage are qualified. This status is not PASS.
The external oracle records an acknowledged sequential
history and verifies successful overwrites, deletes, hard links, data and xattrs:

```sh
sudo timeout 120 python3 scripts/crash-history.py record --mount /mnt/test --history /tmp/opencode/history.jsonl
# Perform the explicitly coordinated restart below. Never call newfs.sh here.
sudo timeout 120 python3 scripts/crash-history.py verify --mount /mnt/test --history /tmp/opencode/history.jsonl
```

Keep history on a local filesystem outside CephFS. The controller flushes each
intent and observed success before proceeding. Preserve the history on failure.
Physical parent resolution, device identity, and mount type checks keep history
off the tested filesystem, including symlink and bind aliases. Parents must
already exist. Log creation stays exclusive.

For an interrupted sequential request, the verifier accepts only the state before
the pending operation or its allowed completed state. Pending O_EXCL create also
permits an empty or partially written file with a matching data prefix. Rename,
link, xattr and unlink have explicit before/after states. Malformed ordering,
lost acknowledged operations and states outside these outcomes fail. A valid
incomplete history still returns 77, not PASS. This does not qualify concurrent
requests, transport retries, dirty caps, flush acknowledgments, locks or unlink-open.

Required controller protocol for the next gate:

1. Capture cluster fsid, fscid, metadata pool and data pool IDs before the trial.
2. G2: record, cleanly unmount, stop only the MDS, restart, remount and verify.
3. G3: keep the client mounted. Record acknowledgments outside the MDS. Check
   the target PID's executable and identity, then send SIGKILL to **that MDS
   only**. Do not kill OSDs. Restart with the same config and pools.
4. Bound every start, mount, workload and verification call with a timeout.
   Capture recovery time and time to first successful operation.
5. Reject a changed filesystem or pool identity. Never reset or recreate the
   filesystem during a trial. Verify allowed outcomes for unacknowledged calls
   and at-most-once retries before claiming a full G3 pass.
6. On failure, keep history and daemon logs. Unmount only owned mounts. Stop only
   owned MDS processes. Leave OSDs running. Report remaining mounts and daemons.

## Ganesha smoke and recovery modes

`devcluster/ganesha.sh start|stop|status [smoke|recovery]` keeps smoke mode as the
default. Both modes use 127.0.0.1:12049. They cannot run at the same time. The
system instance on 2049 is not touched. Stop the current helper mode explicitly
before switching modes. Never switch modes inside a recovery trial.

Recovery mode enables NFSv4 grace (90 seconds) and a 60-second lease. It uses the
`fs` recovery backend under `$DEVCLUSTER_DATA/ganesha-recovery/recov`. Keep this
directory across Ganesha restarts. The helper uses a stable node ID, cluster-scoped
server owner/scope, no delegations, and `client_oc=false`. Give fresh client work
more than 90 seconds when a grace period is active. Smoke mode remains graceless.

`devcluster/ganesha.sh print-config recovery` renders without starting a daemon.
For offline rendering only, `GANESHA_CONFIG_FSID` can supply a UUID. Actual starts
always read the real cluster fsid. Mode rendering and option names are checked
offline against installed Ganesha 6.5 documentation. Live recovery-mode startup
and reclaim are not qualified yet.

NFS grace is distinct from CephFS MDS reconnect. FSAL_CEPH recreates a cmount by
setting session timeout, performing `ceph_start_reclaim(uuid, CEPH_RECLAIM_RESET)`,
accepting ENOENT for a missing prior session, setting its stable UUID, and mounting.
The C++ behavior is documented in
`mds-rust-map/cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md`
section 3.16.2 and Ceph `Client::start_reclaim`/`set_uuid`. The Cephadm Ganesha
template also disables delegations. This single-node `fs` setup is not a
`rados_cluster` takeover configuration. No Rust G3 reclaim pass is claimed.
