# local-cluster

Task: Add a small script that starts a local Ceph cluster from a built tree, for Rust MDS tests. No C++ MDS.
Status: G0–G4 closed on fs `a` (2026-11-16). Serve with `target-g4/debug/ceph-rmds` on `rmds.wal.0.0`. G3 APPLICATION_PASS remains `/var/tmp/g3-native-r19`; `full_g3_qualified` false. Do not newfs. Do not delete checkpoint objects.
Context: CEPH_LIB must point at the build lib directory or ceph-mon exits looking for /usr/local/lib/ceph plugins. osd stat JSON has no space after the colon, so the OSD wait uses Python. Health stays HEALTH_ERR until an MDS starts, and this Ceph build reports ten missing manager modules. The OSD is memstore. stop sends SIGTERM.
Next: G0–G4 closed on fs `a` 2026-11-16 (`current/g0-g4-closeout.md`). Serve with `target-g4/debug/ceph-rmds`. G3 follow-ups remain in `current/g3-follow-up.md`.
Log:
- [2026-11-10 10:40] Ledger created before the script.
- [2026-11-10 10:45] Cluster is up. mon quorum a, osd.0 up, 16 PGs active+clean, filesystem a, mds.rust key, no MDS.
- [2026-11-10 17:59] Fresh root process exited Legacy2. The next process with --upgrade-retry-targets reached up:active. Kernel client read back hello, then unlink. Daemon stopped.
- [2026-11-10 21:30] Built /mnt/igor/github/salieri11/ceph/build/bin/ceph-fuse. The generated git version had to be the commit hash; the packed branch name is not a C identifier.
- [2026-11-11 11:20] Native G3 trial stopped in preflight. findmnt 2.37.2 has no listmount UNIQ-ID. Rank 0 stayed up:active. No SIGKILL.
- [2026-11-11 15:22] /var/tmp/g3-native-r18 exited 0. Two native rounds, kernel and fuse stayed mounted, both client assertion sets true, cold comparison true, no early lock grant. full_g3_qualified false.
- [2026-11-11 15:40] Lock-wait cap exemption is limited to the inode being locked. A full WAL checkpoints when only those waits remain, instead of latching admission. Offline tests passed. The r18 process was not replaced.
- [2026-11-11 15:41] /var/tmp/g3-native-r19 exited 0 on the rebuilt binary. Two native rounds passed. full_g3_qualified false.
- [2026-11-16 15:10] Documented G0–G4 closeout; default MDS binary target-g4. G2 trial and G4 live scenario recorded in `current/g0-g4-closeout.md`.
