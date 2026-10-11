#!/usr/bin/env bash
# ceph-fuse mount of file system "a" as client.admin (runs ceph-fuse with sudo).
#
#   mount-fuse.sh <mountpoint> [extra ceph-fuse args...]
#
# e.g. mount-fuse.sh /mnt/f --ms_bind_msgr2=false -m v1:127.0.0.1:6789
# Log: $DATA/log/ceph-fuse.<mountpoint>.log, admin socket: $DATA/run/ceph-fuse.<mountpoint>.asok
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

[ $# -ge 1 ] || die "usage: $0 <mountpoint> [extra ceph-fuse args...]"
MNT=$1; shift
cluster_exists || die "no cluster (run start.sh)"
tag=$(realpath -m "$MNT" | tr '/' '_' | sed 's/^_//')
sudo mkdir -p "$MNT"
log "sudo ceph-fuse -c $CONF -n client.admin --client_fs=$FS_NAME $* $MNT"
sudo ceph-fuse -c "$CONF" -n client.admin --client_fs=$FS_NAME \
    --log_file="$LOG_DIR/ceph-fuse.$tag.log" --admin_socket="$RUN_DIR/ceph-fuse.$tag.asok" \
    "$@" "$MNT"
grep " $(realpath -m "$MNT") " /proc/mounts >&2
