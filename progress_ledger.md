# progress_ledger

Task: Continue the Rust MDS build per HANDOVER.md. 5.1 (engine wiring, gate
`scripts/smoke.sh`) and 5.2 (OSD client in the daemon) are done. This ledger
tracks agent progress.

Status:
- **5.1 COMPLETE** (2026-10-13 13:55): `scripts/smoke.sh` passes; three kernel-visible cap
  bugs fixed against the golden kernel-v1 captures (see decision log 8.1).
- **5.2 COMPLETE** (2026-10-13 15:30, second agent): Objecter wired into the daemon
  (one messenger identity, osdmap subscription, client_inc from the MDSMap incarnation,
  `Fenced` exits). Effects run RADOS work via `crates/mds/src/rados.rs`: PurgeInode
  (verified: data pool empty after a smoke run), TruncateData (TRIMTRUNC per object),
  RecoverFileSize (probe + log; engine callback pending), EvictClient (monitor blocklist
  through new `MMonCommand`/`MonClient::command`, verified in rados_live, then session kill).
- **5.1 tail DONE** (2026-10-13 17:05, first agent): pjdfstest on a kernel mount:
  8769 pass, 29 not-ok = 28 suite `# TODO`s (known Linux behaviors) + 1 Y2038 assertion
  (utimensat/09.t: the kernel client truncates tv_sec to u32 on the wire,
  `ceph_encode_timespec64`; fails identically against C++). One more engine bug fixed
  via rename/24.t: `HoldKind::RenameSrcWrlock` recalls `Fs Fx` on the rename source
  parent from every holder incl. the renamer, so the revoke/re-grant cycle refreshes
  the client's cached `nsubdirs` (commit 1e7c974).
- Gates re-run after 5.2 and the rename fix: workspace tests pass, fmt clean, smoke.sh
  PASS, `rados_live` PASS (now including blocklist add/rm).
- Committed: 9743767 (engine wiring + smoke gate + 5.2 RADOS work), 1e7c974 (rename
  source-dirstat refresh + pjdfstest result).

Context:
- Deviations logged in decision log 8.1 (2026-10-13 "Eviction ordering without
  persistence"): no OSDMap wait/epoch barrier before the kill; RecoverFileSize result
  not applied. Both land with 5.3.
- `MonClient::command` fails in-flight commands on session loss (caller retries); tids
  are per-MonClient-task.
- Build with `CARGO_TARGET_DIR=target-monitor` for agent work; smoke.sh builds into
  ./target itself.
- Dev cluster up (data kept). Between daemon runs use `devcluster/newfs.sh`.
- A stale hung ceph-fuse mount wedged scripts on Oct 13 morning (realpath on a dead
  FUSE mount blocks in D state); `devcluster/umount.sh` now resolves the mountpoint
  lexically. `mount-fuse.sh` still uses realpath (line 14, 20) - safe unless a stale
  mount is present; fix if it bites again.

Next (HANDOVER 5.3, persistence per design/store.md):
1. Format marker, shard map with ownership epochs, cmpxattr guards on WAL appends.
2. Per-shard WAL streams with group commit; inode table and directory objects.
3. Inode-number leases; replay. Engine already emits self-contained JournalRecords.
4. Then failover: replay, reconnect, rejoin (gate: kill -9 under load, workplan stage 3).

pjdfstest tooling: autotools are absent; /mnt/workspace/pjdfstest is built with a
hand-written config.h (define HAVE_STRUCT_STAT_ST_{ATIM,MTIM,CTIM} or ns assertions
report "unknown") and `cc -O2 -o pjdfstest pjdfstest.c`. Run as root from a directory
on the mount: `cd <mnt> && sudo prove -rv /mnt/workspace/pjdfstest/tests`.

Log:
- [2026-10-13 06:48] Ledger created; inherited WIP assessed (compiles, 328 tests pass).
- [2026-10-13 07:00-13:50] Live gate debugging: smoke.sh line-87 ENOENT after drop_caches.
  Wire-diffed against golden kernel-v1 captures; three bugs found and fixed (see above).
- [2026-10-13 13:55] scripts/smoke.sh PASS. Full workspace tests green. fmt clean.
  Not committed. Daemon stopped, mounts cleaned, cluster data kept.
- [2026-10-13 15:30] 5.2 done (second agent): osdc in the daemon, RADOS effects,
  mon command + blocklist. 343 tests, smoke PASS, rados_live PASS. Not committed.
  Umount.sh realpath fix from the morning wedge kept.
- [2026-10-13 16:45] First agent committed 9743767 (everything: 5.1 + 5.2 + fmt churn).
- [2026-10-13 17:05] 5.1 tail done: pjdfstest 8769 pass (28 TODO + 1 Y2038 wire limit).
  rename/24 fix (RenameSrcWrlock) committed as 1e7c974. utimensat/08 was a helper-build
  artifact (missing ns stat defines), not an MDS bug. C++ MDS bring-up is currently
  broken (msgr2 auth `bad_method` in mds.a; keys verified matching) - worth fixing for
  future reference diffs.
