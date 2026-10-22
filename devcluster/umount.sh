#!/usr/bin/env bash
# Unmount a CephFS mount (kernel or ceph-fuse). Escalates to umount -f and then
# a lazy unmount if the MDS/cluster does not answer.
#
#   umount.sh <mountpoint>
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

[ $# -eq 1 ] || die "usage: $0 <mountpoint>"
# Resolve aliases before selecting a mount. Refuse resolution failures.
MNT=$(timeout --kill-after=2 10 python3 "$DEVCLUSTER_DIR/mount_guard.py" canonical "$1")
findmnt --mountpoint "$MNT" >/dev/null || { log "$MNT is not mounted"; exit 0; }
timeout 30 sudo umount "$MNT" && exit 0
log "umount $MNT failed or timed out; trying umount -f"
timeout 30 sudo umount -f "$MNT" && exit 0
log "trying lazy umount -l"
sudo umount -l "$MNT"
