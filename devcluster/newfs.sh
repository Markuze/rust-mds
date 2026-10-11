#!/usr/bin/env bash
# Recreate file system "a" with new, empty pools, so that the next MDS that
# beacons is told up:creating (as on a brand-new cluster) instead of up:replay.
# Run it when switching between the C++ and the Rust MDS. All CephFS data is lost.
# Any MDS holding a rank is failed by "ceph fs fail" (and blocklisted).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
cluster_exists || die "no cluster (run start.sh)"
[ -z "$(ceph_mounts)" ] || die "unmount CephFS first: $(ceph_mounts | tr '\n' ' ')"

if c fs get $FS_NAME >/dev/null 2>&1; then
    c fs fail $FS_NAME >/dev/null
    c fs rm $FS_NAME --yes-i-really-mean-it
fi
for p in $META_POOL $DATA_POOL; do
    c osd pool rm $p $p --yes-i-really-really-mean-it >/dev/null 2>&1 || true
done
c osd pool create $META_POOL $PG_NUM $PG_NUM replicated >/dev/null
c osd pool create $DATA_POOL $PG_NUM $PG_NUM replicated >/dev/null
c fs new $FS_NAME $META_POOL $DATA_POOL >/dev/null
for i in $(seq 120); do
    if c pg stat | grep -qE '^[0-9]+ pgs: [0-9]+ active\+clean;'; then break; fi
    sleep 1
done
log "file system $FS_NAME recreated: $(c fs get $FS_NAME --format json | python3 -c 'import json,sys; m=json.load(sys.stdin)["mdsmap"]; print("epoch", m["epoch"], "in", m["in"], "failed", m["failed"])')"
