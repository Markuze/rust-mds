# g3-send-fault

Task: Finish the G3 commit/send fault seam. A committed reply or FLUSH_ACK can be dropped once for an exact owner gid, session, and tid. The mutation stays committed. A retry must not apply it again.
Status: The committed-send seam is in the daemon. Three focused tests passed. The native mounted SIGKILL trial passed later (`/var/tmp/g3-native-r19`, `APPLICATION_PASS`) without arming this socket. `full_g3_qualified` stays false.
Context: `crates/mds/src/send_fault.rs`, `crates/mds/src/server.rs`, `crates/mds/src/daemon.rs`, `scripts/g3_trial.py`. The socket is off unless `RMDS_SEND_FAULT_SOCK` is set.
Next: Same-tid reply loss and `FLUSH_ACK` loss are follow-ups in `current/g3-follow-up.md`. Arm one observed tid. Do not treat the mounted SIGKILL trial as that proof.
Log:
- [2026-11-04 15:40] Ledger created. Live G3 cannot run here. The missing committed-send seam is the code gap named by `scripts/g3_trial.py`.
- [2026-11-04 15:50] `cargo test -p rmds send_fault` passed 3 tests: command parse, one dropped mkdir reply with a same-tid retry and a later EEXIST, and one dropped FLUSH_ACK beside a grant that was still sent.
- [2026-11-12] Native mounted failover passed without this socket. Live reply loss and `FLUSH_ACK` loss stay in `current/g3-follow-up.md`.
