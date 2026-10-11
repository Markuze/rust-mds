#!/usr/bin/env bash
# Stop the cluster and delete all of its data. The next start.sh creates a new
# cluster (new fsid, new keys).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

"$DEVCLUSTER_DIR/stop.sh"
case $DATA in
    /mnt/workspace/devcluster-data) ;;
    *) die "refusing to delete unexpected data path $DATA" ;;
esac
# sudo: ceph-fuse (run as root) leaves root-owned sockets/logs behind.
sudo rm -rf --one-file-system "$DATA"
log "deleted $DATA"
