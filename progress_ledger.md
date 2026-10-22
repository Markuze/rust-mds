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

Next (HANDOVER 5.3, persistence per design/store.md) - IN PROGRESS:
1. DONE: WAL record encoding. New crate `crates/store` (engine stays wire-free):
   versioned denc encoding of JournalRecord v1 (`record.rs`, plain functions,
   orphan rule) and the segment framing of design 3 (`wal.rs`:
   `u32 len | u32 crc32c | u64 seq | u64 txid | body`; `read_segment` stops at
   the first torn/corrupt/out-of-sequence record). Tests: workload round trip,
   replay rebuilds an identical engine, torn tail, crc corruption, unknown
   same-version tail fields skipped. Commit 66718c1.
2. Next: the RADOS objects - `rmds.format` marker, `rmds.shardmap` (owner
   epochs), the Objecter-based WAL appender (group commit, `cmpxattr(rmds.owner)`
   guard on every append), then checkpointing into inode/dir omaps and replay
   from RADOS. Gate: kill -9 under load, no acknowledged metadata lost.
   Entry points for step 2: `design/store.md` sections 1-3; the Objecter API in
   `crates/osdc/src/objecter.rs` and `examples/rados_live.rs` (wiring); the
   engine's `Journal` trait (`crates/engine/src/journal.rs`) is the interface
   the RADOS journal must implement; the daemon holds it in
   `crates/mds/src/server.rs` (`Server::new` creates `Engine::new(cfg,
   MemJournal::new())` today - swap in the RADOS journal behind an async
   commit); `crates/mds/src/rados.rs` already has the Objecter handle plumbing
   from 5.2.
3. Then failover: replay, reconnect, rejoin.

## Handover notes for a new agent (any harness)

Read order: AGENTS.md (repo guide: commands, conventions, gotchas) ->
HANDOVER.md (status) -> this file -> design/store.md (what you build next).
Everything is committed; the tree is clean at 66718c1.

Environment facts:
- Dev cluster is UP (1 mon, 1 mgr, 3 memstore OSDs; data kept in
  /mnt/workspace/devcluster-data). Between daemon runs: `devcluster/newfs.sh`.
  Never kill OSDs; stop only with `devcluster/stop.sh` (memstore loses data).
- No MDS is running. `./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring
  $RMDS_KEYRING` after `source devcluster/env.sh`.
- The C++ reference MDS (`start.sh --mds cpp`) currently FAILS to start (msgr2
  cephx "bad_method"; keys verified matching). Unresolved; fix if reference
  diffs are needed.
- Use `CARGO_TARGET_DIR=target-monitor` (or any target-*) for agent builds;
  `scripts/smoke.sh` builds into ./target itself.
- pjdfstest lives at /mnt/workspace/pjdfstest, prebuilt (no autotools here;
  config.h is hand-written - see "pjdfstest tooling" below).
- If a fuse mount hangs scripts, `devcluster/umount.sh <mnt>` handles it.
- One agent may be working in parallel (zellij sessions mds_peer/mds_charm);
  check `find crates -newermt <last-commit-time> -type f` and the ledger mtime
  before starting, and re-read files before editing if it was active.

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
- [2026-10-13 17:35] 5.3 started: `crates/store` with the WAL record encoding (v1)
  and segment framing; round-trip/replay/torn-tail tests green; workspace green.
