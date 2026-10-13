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
- Gates re-run after 5.2: workspace 343 tests pass, fmt clean, smoke.sh PASS,
  `rados_live` PASS (now including blocklist add/rm).
- Tree not committed (user has not asked for a commit).

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
Also pending from earlier stages: pjdfstest on a kernel mount (5.1 tail).

Log:
- [2026-10-13 06:48] Ledger created; inherited WIP assessed (compiles, 328 tests pass).
- [2026-10-13 07:00-13:50] Live gate debugging: smoke.sh line-87 ENOENT after drop_caches.
  Wire-diffed against golden kernel-v1 captures; three bugs found and fixed (see above).
- [2026-10-13 13:55] scripts/smoke.sh PASS. Full workspace tests green. fmt clean.
  Not committed. Daemon stopped, mounts cleaned, cluster data kept.
- [2026-10-13 15:30] 5.2 done (second agent): osdc in the daemon, RADOS effects,
  mon command + blocklist. 343 tests, smoke PASS, rados_live PASS. Not committed.
  Umount.sh realpath fix from the morning wedge kept.
