#!/usr/bin/env bash
# Show daemons, cluster health, the FSMap and CephFS mounts.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

if ! cluster_exists; then echo "no cluster in $DATA (run start.sh)"; exit 1; fi
echo "== daemons (data $DATA, fsid $(cat "$DATA/fsid"))"
for n in mon.$MON_ID mgr.$MGR_ID $(printf 'osd.%s ' "${OSD_IDS[@]}") mds.$CPP_MDS_ID; do
    pid=$(daemon_pid "$n")
    if [ -n "$pid" ]; then st="running (pid $pid)"; else st=stopped; fi
    printf '  %-8s %s\n' "$n" "$st"
done
rmds=$(pgrep -x ceph-rmds | tr '\n' ' ')
printf '  %-8s %s\n' "ceph-rmds" "${rmds:-not running}"
echo "== ceph -s"
c -s
echo "== ceph fs dump"
c fs dump
echo "== CephFS mounts"
grep -E ' (ceph|fuse\.ceph-fuse) ' /proc/mounts || echo "  none"
