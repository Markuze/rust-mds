# g3-follow-up

Task: Record the G3 steps that the native mounted trial did not prove.
Status: Native mounted failover passed on 2026-11-11. These items remain open. They do not reopen that pass.
Context: Evidence is `/var/tmp/g3-native-r19`. Result `APPLICATION_PASS`. `full_g3_qualified` stays false. The trial modes `reply-loss` and `flush-ack-loss` still exit 2 before any workload.
Next: Arm one observed tid through `RMDS_SEND_FAULT_SOCK`.

The native gate is closed. `/var/tmp/g3-native-r19` exited 0 on the rebuilt `target-g3/debug/ceph-rmds`. Two active-MDS SIGKILL rounds kept `/mnt/rmds` and `/mnt/rmds-fuse` mounted. Both clients passed O_EXCL, rename-over-target, hard links, positive symlinks, xattrs, truncate/grow, unlink-open, dirty write then fsync, and the exact namespace comparison. Locks stayed with the holder. Cold comparison matched. First success was about 4.5 seconds. The replacement reached active in about 5 seconds.

## Still open

1. **Same-tid reply loss.** After the write-ahead log commit, drop one safe `MClientReply` before the client accepts it. The same session, operation, and tid must retry once and receive the original result. The mutation must not run again. The daemon drop is `reply <gid> <client> <incarnation> <tid>` on `RMDS_SEND_FAULT_SOCK`. The trial does not yet observe that tid.
2. **FLUSH_ACK loss.** Drop one committed `FLUSH_ACK` for an exact flush tid. The resend must be acknowledged without applying the size or times again. An uncommitted flush must not be acknowledged, and a later grant must not pass it. The daemon drop is `flush_ack <gid> <client> <incarnation> <tid>`.
3. **Cold standby takeover.** The native trial restarted the same daemon. It did not fail over to a cold standby or standby-replay follower.
4. **Protocol reconnect timing.** The trial records application first-success time. It does not record the final reconnect chunk or the final OPEN acceptance.

`cargo test -p rmds send_fault` covers one dropped mkdir reply and one dropped `FLUSH_ACK` offline. That is not a mounted client retry.

Log:
- [2026-11-12] Follow-up created from the `/var/tmp/g3-native-r19` APPLICATION_PASS.
- [2026-11-15] Ganesha reclaim left this follow-up. The remaining items are reply loss, `FLUSH_ACK` loss, cold standby takeover, and protocol reconnect timing.
