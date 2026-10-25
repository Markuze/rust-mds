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

## Phased G3 application gates (offline implementation; live review required)

`scripts/g3_trial.py` is a separate controller over the accepted ownership and
identity guards. It does not change the accepted G2 controller/oracle. Do not
run it against the old G2 binary. Parent authorization, accepted G3 runtime,
accepted tooling and a separately identified reviewed G3 binary are required.
No native G3 or recovery-mode Ganesha run is established by this implementation.

### Native application scope

```sh
source devcluster/env.sh
# Inside the owned private mount namespace, after the separate readiness review:
sudo env RMDS_RECOVERY_READY=yes RMDS_G3_READY=yes \
  bash scripts/g3-trial.sh native "$ACTIVE_PID" "$OWNED_KERNEL_MOUNT" "$NEW_OUT" \
  --owned-mount --rust-filesystem --expected-fscid 44 \
  --binary "$REVIEWED_G3_BINARY" --keyring "$RMDS_KEYRING" \
  --fuse-mount "$EMPTY_FUSE_TARGET" --cold-mount "$EMPTY_COLD_TARGET" --rounds 2
```

The default is two rounds; the maximum is eight. One round is useful preparation
but cannot establish repeated recovery. `--active-timeout` defaults to 120 s.
`--case-timeout` defaults to 120 s, with a maximum of 300 s. Every mount, command,
worker result, lock handoff and active-state wait has a finite deadline.
Required tools are checked before workload admission. Missing tools fail rather
than skip successfully. There is no newfs/reset/delete-metadata trial action.

Use `--takeover-pid PID` to select one existing cold standby for the first
promotion. Preflight binds its exact PID/start/config/name/listener/binary and
monitor entry. Supply that standby's existing `--keyring`, which must match its
argv. Later rounds explicitly restart the promoted daemon ID with that keyring.
This does not promise repeated cold-standby promotion. Unselected or changed
standbys fail. Externally launched standbys are not cleanup-owned children.

The kernel mount and the controller-created foreground FUSE mount stay attached
through every MDS SIGKILL. Each has a persistent worker. Each worker:

- Reuses the complete 17-operation/10-inode G2 acknowledged history and oracle.
- Creates an additional unique namespace. It checks O_EXCL, rename-over-target,
  hard-link identity/count, exact names/types/ownership/modes, binary/empty user
  xattrs and file data/mtime. It fsyncs files and affected directories.
- Creates valid relative, dangling and directory symlinks. It checks lstat inode
  identity and exact readlink text. It also checks resolution of the valid file
  link. Namespace traversal never follows symlinks. Read and type errors fail.
- Truncates and grows a file, checks preserved prefix/zero fill/new tail and
  fsyncs it before failure. The surviving and cold comparisons check that state.
- Retains a descriptor after unlink. It checks the same inode, zero link count,
  exact saved size and every saved byte **before any recovery mutation**. It
  fsyncs that observation to the external client history before a separate new
  descriptor write/fsync. Truncation, extension, corruption and read errors fail
  without repairing the old state. Pathname existence cannot satisfy this case.
- Writes a bounded 1 MiB dirty buffer before failure. Its external ACK explicitly
  says `write-return-only`. After failure it fsyncs the same descriptor and checks
  the known bytes/size/inode. This exercises client writeback/fsync, not a proven
  deterministic MClientCaps or FLUSH_ACK-loss window.
- Holds an exclusive whole-file fcntl lock and an exclusive flock across all
  rounds. Four separate contenders use the opposite native client. Each must
  first observe EAGAIN/EACCES, then wait. No contender may exit or log acquisition
  before the holder's explicit release. All must acquire after release and exit
  successfully. The holder reads its lock inode through its existing descriptor:
  opening/closing another descriptor in that process would release POSIX locks.

### Per-round protocol and result

1. Recheck owner, active PID/start/listener, binary/config, mounted identities and
   all visible CephFS namespaces. Recheck the planned checkpoint budget.
2. Arm both dirty writers. Run the existing sequential acknowledged-history
   workload on both clients. Pause each at its externally fsynced rename intent.
   `--pause-before 2` is a **client-side** barrier, not wire delivery/commit proof.
3. SIGKILL only the pidfd-bound active MDS. Wait for its exit. Do not unmount either
   surviving client. Release both sequential barriers and submit worker progress
   commands before starting the selected restart/takeover.
4. Observe a different active gid through real monitor state. Bind it to the
   selected process/binary, then use it as the next round's exact signal target.
5. Require both workers to acknowledge server-visible O_EXCL create/write/file-
   fsync/parent-fsync progress and all selected case assertions. Require both
   sequential histories to finish with exit 0. The reused uncertain-outcome
   verifier returns 77 for a valid incomplete prefix; that cannot pass this gate.
6. Compare every saved mixed/application/sequential history through both mounted
   clients. Create an independent cold kernel client with `noshare` and a new
   boot-scoped unique mount identity. Compare all histories there too. Normal-
   unmount that cold client before the next failure. Unlike G2, G3 intentionally
   retains the two surviving clients; only those captured mounts are allowed.
   Sequential history paths are cumulative across rounds. Replacing the active
   child list never removes an earlier acknowledged namespace from comparison.
7. After every round, reject any early contender grant. At the end, release both
   lock classes, require all four positive contender handoffs, and close every
   persistent worker successfully. Only then can result metadata be written.

Artifacts include `plan.json`, per-client fsynced intent/ACK logs, the unchanged
mixed histories, per-round sequential histories, stop intent, restart output,
recovered PID/gid/identity, cold unique identity, capacity report, checked case
assertions/timings, final mount ownership and `lock-proof.json`.

Persistent-client histories now use version2 with explicit profile/coverage and
pre-write descriptor observations. Each arm/recovery observation must match the
previous saved descriptor ACK. The independent oracle rejects a missing proof.

The version2 descriptor proof has exactly these fields. JSON booleans are not
integers. Every proof is checked before comparison, including earlier snapshots.

| Field | Required value |
|---|---|
| `inode` | Integer, `1 <= inode < 2^64`; the original prepare descriptor inode |
| `size` | Integer, `0 <= size < 2^63` |
| `sha256` | String of exactly 64 lowercase ASCII hexadecimal digits |
| `nlink` | Integer 0 for `native`, integer 1 for `nfs41` |

Prepare/recover ACKs must include `unlinked_observation` and matching typed flat
fields: `unlinked_inode`, `unlinked_size`, `unlinked_data_sha256`, `unlinked_nlink`.
Arm ACKs must include `unlinked_fsync_ack`. Each arm/recover needs exactly one
preceding observation row and a matching ACK `unlinked_before_write`. Both copies
must match the previous checked descriptor state. All copies retain the original
descriptor inode and profile link count. Size/digest must describe the actual
fixed workload: prepare `unlinked-open`; arm N `unlinked-open-round-N`; recover N
that arm value plus `/recovered-N`. Consistently equal but fabricated values do
not satisfy this contract. Required proof/flat fields cannot move to other ACKs.
Proof, manifest and round fields in non-producer record positions also fail.

The loader checks this action state, not only sequence-number equality:

```text
new --prepare--> ready --arm(N+1)--> armed --recover(N+1)--> ready
ready --release--> released --close--> closed
ready --close--> closed
```

Round numbers are nonboolean integers in `[1, 2^64)`. The first arm is round 1.
Each recover ACK matches its arm ACK; the next arm increments the round by 1.
There is one prepare, at most one release and one terminal close. Close without
release permits normal descriptor teardown; it is not a lock-handoff proof.
Release requires a nonboolean integer `released_ns` in `[0, 2^64)`. Close requires
`closed=true`, not integer 1. Pending actions, dirty rounds, repeated prepares,
skipped/repeated rounds and actions after release/close fail. Valid prepared,
recovered, released and closed prefixes remain readable. Mounted/cold comparison
normally occurs in `ready`, before final release/close.

Every prepare/recover ACK requires a manifest and the declared profile/exclusions.
Other ACKs cannot substitute a manifest. Every saved manifest must retain the
header root name/profile and original prepare root inode/type. Its root entry
must be a directory with typed inode/mode/uid/gid fields. A later correct manifest
cannot erase an earlier invalid root. Native manifests must not name the unlinked
descriptor; NFSv4.1 manifests must retain the linked file with the same proof
fields. Malformed evidence is a runtime failure, not a partial/success result.
This strengthens the reader only. It changes no worker I/O or version2 fields.

New sequential histories also use version2 and retain the acknowledged root
inode. Their producer pins the root directory across the client-intent pause.
The application gate requires that identity-bearing version for every round.
Older native synthetic histories retain their read-only uncertain-outcome model;
they do not satisfy the new application history contract.

Before preparation, round arming and recovery mutation, validate the physical
mount/root against retained no-follow directory descriptors. Use descriptor-
relative creation, link/rename, unlink and file I/O. A replaced root symlink or
different directory inode fails before the first write. Cold comparison and
lock contenders also open no-follow directory boundaries. Valid symlinks inside
the workload remain separate declared data, not paths used for mutation.

`client_reconnect_progress_seconds` records each client's first post-failure
server-visible create/write/fsync completion. `first_success_seconds` is the
minimum of those two observations. These are application progress bounds, not
protocol OPEN timestamps. `protocol_reconnect_seconds` is null until the tagged
transport event described below exists. Map ACTIVE timing stays separate.

| Invocation/result | Exit | Meaning |
|---|---:|---|
| `native`, at least two rounds, every selected assertion | 0 | `APPLICATION_PASS`, scope `mounted-recovery-application`; **not full G3** |
| `native --rounds 1`, all selected assertions | 77 | `PARTIAL`; repeated recovery not exercised |
| `ganesha-only` or `mds-ganesha`, application assertions succeed | 77 | `PARTIAL`; protocol/shared-session qualification remains missing |
| `reply-loss` or `flush-ack-loss` | 2 | Required deterministic fault seam is not implemented; no probes/signals/workload |
| Declared prerequisite missing before admission | 2 | No pass; missing tools, ownership, unique-ID support or run budget |
| Any I/O/assertion/timeout/cancellation failure after admission | 1 | No result artifact; exact phase and owned state retained in `error.json` |
| Readiness not enabled | 77 | Dependency disabled; no live action |

Every result has `full_g3_qualified=false`, `same_wire_tid_retry_proven=false`
and `flush_ack_loss_proven=false`. A scoped application exit 0 cannot close the
full R16/R17 matrix. Random SIGKILL and a client-intent pause do not identify a
committed-reply or FLUSH_ACK-loss window.

### Private namespace and retained ownership

Use a new local run directory and a nonexistent artifact directory. Preserve
fs44/meta88/data89 and both accepted acknowledged namespaces. Do not rebuild or
overwrite the preserved G2 binary. The following is a **future authorized run**,
not an instruction to start now. `ACTIVE_PID` must name an already reviewed G3
daemon. The local directory variables must be new, owner-selected paths.

```sh
source devcluster/env.sh
RUN=/mnt/workspace/devcluster-data/sol-g3-UNIQUE
OUT=/tmp/opencode/sol-g3-UNIQUE
mkdir -p "$RUN/mnt-kernel" "$RUN/mnt-fuse" "$RUN/mnt-cold"
sudo env RMDS_RECOVERY_READY=yes RMDS_G3_READY=yes RMDS_LOG=info \
  unshare --mount --propagation private bash -s -- \
  "$ACTIVE_PID" "$RUN" "$OUT" "$REVIEWED_G3_BINARY" "$RMDS_KEYRING" <<'SH'
set -euo pipefail
cd /mnt/workspace/rust-mds
PID=$1 RUN=$2 OUT=$3 BIN=$4 KEYRING=$5
sleep infinity </dev/null >/dev/null 2>&1 &
KEEPER=$!
python3 - "$KEEPER" "$OUT.namespace-owner.json" <<'PY'
import json, os, sys
from pathlib import Path
pid = int(sys.argv[1]); proc = Path(f"/proc/{pid}")
with open(sys.argv[2], "x") as log:
    json.dump({"pid": pid, "start": (proc / "stat").read_text().rsplit(")", 1)[1].split()[19],
               "namespace": os.readlink(proc / "ns/mnt"), "exe": str((proc / "exe").resolve())}, log)
PY
devcluster/mount-kernel.sh "$RUN/mnt-kernel" v1 noshare
exec bash scripts/g3-trial.sh native "$PID" "$RUN/mnt-kernel" "$OUT" \
  --owned-mount --rust-filesystem --expected-fscid 44 --binary "$BIN" --keyring "$KEYRING" \
  --fuse-mount "$RUN/mnt-fuse" --cold-mount "$RUN/mnt-cold" --rounds 2
SH
```

Success leaves the MDS and owned mounts up, but closes the application workers.
Keep the namespace holder until identity-checked **normal** unmount of all owned
clients and validated MDS/helper stop complete. Then release that holder only.
On failure, the controller cancels its own descriptor holders, sequential
workers and lock waiters before attempting normal unmount. Replaced, busy or
unvalidated mounts retain their service processes for the operator. No global
PID search selects a signal victim. OSDs and system Ganesha are never signaled.

### Checkpoint capacity is a prerequisite, not garbage collection

Read the selected daemon configuration and list native checkpoint objects.
The default retained-generation limit is 64. Configuration includes are not
silently flattened; the reader refuses unsupported syntax. Parsing matches the
daemon's `crates/mds/src/config.rs`: ASCII case-insensitive sections/keys; each
key space or dash becomes underscore; `#` and `;` comments; outer double quotes;
last normalized duplicate wins. Lookup is `[mds.ID]`, `[mds]`, then `[global]`.
Read UTF-8 bytes without Python newline conversion; do not invent separate
settings from bare CR or Python-only Unicode/control line separators.
The generation limit, checkpoint interval and record threshold are decimal u64
values. Leading plus/zeros are valid. Suffix units are not. Interval units are
seconds. Invalid/overflow/zero thresholds fail rather than silently using defaults.

With remaining rounds N, selected interval I and selected record threshold R,
the planned extra budget is
`N * (ceil(128/R) + ceil((active_timeout + case_timeout)/I)) + 4`.
The128-record per-round estimate preserves the old eight-checkpoint estimate at
the default R=16, while respecting actual threshold overrides.
Retained plus planned must be **less than** the configured limit. The default
two-round 120s/120s plan budgets 36 extra generations. The accepted G2 final count
was 15, not a new current observation. Recheck before each round. This estimate
is not a reservation or a guarantee against pressure. If capacity runs out,
stop and report. An insufficient preflight budget is exit2; pressure at a
per-round check after admission is runtime exit1, never a successful partial
result. Check the selected standby's settings too. Do not delete checkpoints/WAL
or automatically change budgets.

### Separate Ganesha preparation gates

The controller creates its own foreground recovery-mode helper on **12049**.
`devcluster/ganesha.sh run recovery` preserves the configured stable node,
server owner/scope, recovery directory and 90s grace. It refuses a different
existing recovery configuration. It uses the existing shim; it does not build
one. The system service on 2049 is not selected, stopped or reconfigured.

Inside a retained private namespace with an owned root kernel mount, existing
empty NFS/cold directories, and **separate runtime/tooling/live approval**:

```sh
sudo env RMDS_RECOVERY_READY=yes RMDS_G3_READY=yes RMDS_GANESHA_RECOVERY_READY=yes \
  bash scripts/g3-trial.sh ganesha-only "$ACTIVE_PID" "$OWNED_KERNEL_MOUNT" "$NEW_OUT" \
  --owned-mount --rust-filesystem --expected-fscid 44 \
  --binary "$REVIEWED_G3_BINARY" --keyring "$RMDS_KEYRING" \
  --nfs-mount "$EMPTY_NFS_TARGET" --cold-mount "$EMPTY_COLD_NFS_TARGET" \
  --rounds 1 --case-timeout 180
# A separate trial/new artifact directory, not a switch within the first trial:
sudo env RMDS_RECOVERY_READY=yes RMDS_G3_READY=yes RMDS_GANESHA_RECOVERY_READY=yes \
  bash scripts/g3-trial.sh mds-ganesha "$ACTIVE_PID" "$OWNED_KERNEL_MOUNT" "$OTHER_NEW_OUT" \
  --owned-mount --rust-filesystem --expected-fscid 44 \
  --binary "$REVIEWED_G3_BINARY" --keyring "$RMDS_KEYRING" \
  --nfs-mount "$EMPTY_NFS_TARGET" --cold-mount "$EMPTY_COLD_NFS_TARGET" \
  --rounds 1 --case-timeout 180
```

The first case SIGKILLs only its Ganesha child. The second sends both owned
signals before waiting for either exit and requires signal skew at most 1s.
It restarts the selected MDS and its Ganesha child on unchanged identities.
Two NFS workers share one export/helper and keep descriptors, dirty writes and
locks across failure. The selected and observed mount protocol must be NFSv4.1.
It uses an explicit `nfs41` coverage profile, not the native CephFS profile:

- User xattrs are **excluded** because NFSv4.1 has no user-xattr extension. The
  producer/oracles make no xattr calls and do not report an observed empty set.
  `selected_cases` excludes `xattrs`; `excluded_cases` states the protocol reason;
  the corresponding assertion remains false. Native CephFS xattrs stay mandatory.
- The initial native17-operation history is replaced by a declared NFSv4.1
  create/create/rename-overwrite/link/unlink acknowledged history. It reuses the
  sequential uncertain-outcome model with an explicit versioned profile. Native
  and NFS histories cannot be reinterpreted as the other profile.
- Exact names, inode/metadata/data, hard links, positive symlinks, truncate/grow,
  dirty write/fsync and both lock classes remain selected NFS application cases.

NFS keeps the test open descriptor linked: its client-side
silly-rename semantics are not the native zero-link FD test. The result does not
claim NFS unlink-open server-restart coverage. Both NFS application histories
must pass through the mounted view and an independent `nosharecache` cold view.
No final listing substitutes for their selected exact application assertions.
Excluded NFS xattrs or native unlink-open semantics never become passing cases.

Retain PID/start/owned-listener proofs, stable configuration hash, recovery-root
inode/tree hashes and bounded raw Ganesha log slices before/after each failure.
Log rotation, unknown files, evidence overflow or I/O errors fail explicitly.
These files aid review; configured grace is not observed NFS reclaim success.
Both gates still return 77. They do not yet prove RESET/FINISH, shared native
SessionKey/outstanding cap flushes, NFS reclaim RPCs or the native-CephFS-versus-
NFS lock gap after absent-session eviction. Do not promote them to full R17.

### Minimal daemon/observer seams required from the owners

These are requirements, **not implemented APIs or accepted schema changes**.
The current `reply-loss`/`flush-ack-loss` gates fail prerequisite exit 2. A future
adapter needs one-shot, explicitly armed, owner-only test control plus external
fsynced events. Do not add another journal or alter committed reply contents.

1. **Safe-reply loss:** match one durable session key, authenticated connection
   generation, original request op/tid and rank owner. After its WAL commit, but
   before that selected safe reply's transport acceptance, report the exact
   committed sequence and created inode/result. Hold/drop only that reply until
   the controller kills the MDS. Observe the same session incarnation/op/tid on
   retry, restored completion dedupe and the original mutation/inode/trace.
2. **FLUSH_ACK loss:** match session key, flush tid, cap ID/sequence, dirty mask
   and OSD barrier. Report actual commit before withholding the matching ACK at
   the session funnel. Observe the same flush tid before reconnect and after
   OPEN. Assert no duplicate metadata application. A separate pre-commit hold
   must prove that a new dirty flush cannot be acknowledged or overtaken by a
   conflicting grant. Keep beacons, reading and unrelated recovery controls live.
3. **Protocol reconnect timing:** emit monotonic events for the validated final
   reconnect chunk and final OPEN transport acceptance. Tag PID, rank owner,
   durable session key, connection/attachment generation and client global ID.
   Correlate the kernel and owned FUSE sessions. A requested/confirmed map phase
   or a cached read is not this observation.
4. **Ganesha R17:** expose UUID installation and exact RESET completion, target
   durable Closed receipt, old-address union and post-fence OSDMap epoch under
   requester/owner tags. Correlate both NFS workers with the shared native
   SessionKey and an outstanding flush at failure. Retain actual NFS grace and
   reclaim RPC evidence under the stable server owner/scope. Observe a native
   CephFS lock contender after absent-session eviction and before NFS reclaim;
   report that accepted cross-protocol lock gap rather than claiming continuous
   exclusion. The current opaque shared FSAL export alone does not prove it.

Independent review must accept each seam and its exact-window assertions before
any corresponding qualification is reported. Source fault injection, random
timing and successful ordinary fsync are not interchangeable evidence.

### Same-filesystem retry after the initial mount-ID false-negative

Filesystem **44**, metadata pool **88**, data pool **89** already exist and must
be preserved. Do not run fresh setup or newfs for this retry. The earlier trial
exited 1 before comparison/future-allocation checks; it has no pass artifact.
The authorized 2026-10-25 retry passed and its application evidence is independently
accepted in `current/astra-g2-evidence.md`. The command below records that retry
procedure; it does not authorize another run or qualify the storage fault matrix.

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
the existing artifact/ownership rules still apply. Keep the accepted original
and retry evidence unchanged.

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
