#!/usr/bin/env bash
# Kernel-client mount of file system "a" as client.admin (uses sudo).
#
#   mount-kernel.sh <mountpoint> [v1|v2] [extra,mount,options]
#
#   v1 (default): old device syntax 127.0.0.1:6789:/ and no ms_mode, so the kernel
#                 speaks msgr1 to the monitor and to the MDS (MDS v1 address).
#   v2:           new syntax admin@<fsid>.a=/ with mon_addr=127.0.0.1:3300 and
#                 ms_mode=prefer-crc (msgr2, crc mode).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

[ $# -ge 1 ] || die "usage: $0 <mountpoint> [v1|v2] [extra,options]"
MNT=$1 MODE=${2:-v1} EXTRA=${3:-}
cluster_exists || die "no cluster (run start.sh)"
FSID=$(cat "$DATA/fsid")
sudo modprobe ceph
sudo mkdir -p "$MNT"
case $MODE in
    v1) DEV="$MON_V1:/"
        OPTS="name=admin,secretfile=$ADMIN_SECRET,mds_namespace=$FS_NAME,conf=$CONF" ;;
    v2) DEV="admin@$FSID.$FS_NAME=/"
        OPTS="mon_addr=$MON_V2,ms_mode=prefer-crc,secretfile=$ADMIN_SECRET,conf=$CONF" ;;
    *) die "mode must be v1 or v2" ;;
esac
[ -z "$EXTRA" ] || OPTS="$OPTS,$EXTRA"
log "sudo mount -t ceph $DEV $MNT -o $OPTS"
sudo mount -t ceph "$DEV" "$MNT" -o "$OPTS"
grep " $MNT " /proc/mounts >&2
