# local-cluster

Task: Add a small script that starts a local Ceph cluster from a built tree, for Rust MDS tests. No C++ MDS.
Status: Native G3 application passed two mounted rounds on the rebuilt daemon. Result is APPLICATION_PASS. full_g3_qualified remains false. Evidence is /var/tmp/g3-native-r19. Do not newfs. Do not delete checkpoint objects.
Context: CEPH_LIB must point at the build lib directory or ceph-mon exits looking for /usr/local/lib/ceph plugins. osd stat JSON has no space after the colon, so the OSD wait uses Python. Health stays HEALTH_ERR until an MDS starts, and this Ceph build reports ten missing manager modules. The OSD is memstore. stop sends SIGTERM.
Next: Native mounted G3 is recorded. Remaining steps are in `current/g3-follow-up.md`. The next gate is G4.
Log:
- [2026-11-10 10:40] Ledger created before the script.
- [2026-11-10 10:45] Cluster is up. mon quorum a, osd.0 up, 16 PGs active+clean, filesystem a, mds.rust key, no MDS.
- [2026-11-10 17:59] Fresh root process exited Legacy2. The next process with --upgrade-retry-targets reached up:active. Kernel client read back hello, then unlink. Daemon stopped.
- [2026-11-10 21:30] Built /mnt/igor/github/salieri11/ceph/build/bin/ceph-fuse. The generated git version had to be the commit hash; the packed branch name is not a C identifier.
- [2026-11-11 11:20] Native G3 trial stopped in preflight. findmnt 2.37.2 has no listmount UNIQ-ID. Rank 0 stayed up:active. No SIGKILL.
- [2026-11-11 15:22] /var/tmp/g3-native-r18 exited 0. Two native rounds, kernel and fuse stayed mounted, both client assertion sets true, cold comparison true, no early lock grant. full_g3_qualified false.
- [2026-11-11 15:40] Lock-wait cap exemption is limited to the inode being locked. A full WAL checkpoints when only those waits remain, instead of latching admission. Offline tests passed. The r18 process was not replaced.
- [2026-11-11 15:41] /var/tmp/g3-native-r19 exited 0 on the rebuilt binary. Two native rounds passed. full_g3_qualified false.
