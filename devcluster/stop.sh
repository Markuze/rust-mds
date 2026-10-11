#!/usr/bin/env bash
# Stop every daemon start.sh started (MDS, OSDs, mgr, mon). Keeps all data.
# CephFS mounts on this machine are unmounted first (they would hang otherwise).
# A Rust MDS (ceph-rmds) is not stopped, only reported.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

for m in $(ceph_mounts); do
    log "unmounting $m"
    "$DEVCLUSTER_DIR/umount.sh" "$m" || log "could not unmount $m"
done
"$DEVCLUSTER_DIR/ganesha.sh" stop >/dev/null 2>&1 || true

stop_daemon mds.$CPP_MDS_ID 30
for id in "${OSD_IDS[@]}"; do
    stop_daemon osd.$id 60 &
done
wait
stop_daemon mgr.$MGR_ID 30
stop_daemon mon.$MON_ID 30

# Anything left that uses our ceph.conf (e.g. a daemon whose pid file was lost).
left=$(pgrep -f -- "-c $CONF" || true)
if [ -n "$left" ]; then
    log "killing leftover processes using $CONF: $left"
    kill -TERM $left 2>/dev/null || true
    sleep 2
    kill -KILL $left 2>/dev/null || true
fi
rmds=$(pgrep -x ceph-rmds || true)
[ -z "$rmds" ] || log "note: Rust MDS process(es) still running: $rmds"
log "cluster stopped (data kept in $DATA)"
