# g0-g4-closeout

Task: Close workplan gates G0 through G4 on filesystem `a` with the current tree and `target-g4` daemon.
Status: **Closed through G4.** G5 is not started. G3 follow-ups stay in `current/g3-follow-up.md` and do not reopen the application pass.
Context: Cluster data `/var/tmp/rust-mds-local`, fscid `1`, `rmds.wal.0.0` only. No newfs, no osd/mon/mgr stop.
Next: G5 only after an explicit assignment. Do not create `rmds.wal.1.0`.

## Evidence (2026-11-16)

| Gate | Result | Artifact / command |
|------|--------|---------------------|
| G0 offline | PASS | `cargo test --workspace` with Ceph build `LD_LIBRARY_PATH`/`CEPH_LIB`: 770 passed, 0 failed, 2 ignored (`test_g0_captures` corpus tools). Python scripts `test_g0_*` / wire / dencoder suites in that run. |
| G1 offline | PASS | Same workspace run: engine G1 journal/retry/recovery, daemon completion semantics, no invalid-reply regressions in scoped suites. |
| G2 application | PASS | `RMDS_RECOVERY_READY=yes` `scripts/recovery_trial.py g2` on `/mnt/rmds`, fscid 1, `target-g4/debug/ceph-rmds`; fuse unmounted for controller hygiene; result `PASS`, `coverage=clean-restart-application`, 16 future nonreuse checks; `/var/tmp/g0-g4-closeout/g2-trial/`. Active ~5.4s. |
| G3 application | PASS (prior) | `/var/tmp/g3-native-r19`, `APPLICATION_PASS`, 2026-11-11. `full_g3_qualified` stays false. Follow-ups: reply loss, FLUSH_ACK loss, cold standby, reconnect timing. |
| G4 offline + live | PASS | Engine/daemon G4 suites; live `g4.exercise` on `/mnt/rmds-fuse` as root with `target-g4` daemon (subvolume, EXDEV, ENOTEMPTY, client EDQUOT). |

## Daemon

- Binary: `target-g4/debug/ceph-rmds`
- Active log line: `object=rmds.wal.0.0`, `later_shard=false`
- Post-G2 pid: recovered daemon on same WAL object

## Exclusions (explicit)

- G3 `full_g3_qualified` and the four items in `current/g3-follow-up.md`
- G0 manual corpus regeneration (`--ignored` capture tools)
- G5 and M0b
- October historical fs44 G2 layered acceptance remains historical; this row supersedes for **filesystem `a`**

Log:
- [2026-11-16 14:19] Swapped MDS from target-g3 to target-g4; rank 0 active.
- [2026-11-16 14:20] Live G4 scenario OK on fuse (root). Temporary fuse umount for G2 controller; fuse remounted after trial.
- [2026-11-16 14:54] G2 recovery trial PASS on fs `a`, fscid 1, target-g4.