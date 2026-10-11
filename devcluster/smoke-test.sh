#!/usr/bin/env bash
# Simple I/O check on a mounted CephFS (kernel or ceph-fuse):
#   mkdir, create + write (dd), read back (cat, md5sum), ls -l, rename (mv),
#   unlink (rm), rmdir; also checks that the file data reached the data pool.
#
#   smoke-test.sh <mountpoint> [size_MiB (default 4)]
#
# Runs as root (re-executes itself with sudo): the CephFS root directory is
# owned by root. Exit status 0 = every step passed.
set -euo pipefail
[ $# -ge 1 ] || { echo "usage: $0 <mountpoint> [size_MiB]" >&2; exit 2; }
if [ "$(id -u)" != 0 ]; then
    exec sudo "$0" "$@"
fi
MNT=$1 MB=${2:-4}
CONF=/mnt/workspace/devcluster-data/ceph.conf
grep -qE " $(realpath -m "$MNT") (ceph|fuse\.ceph-fuse|nfs4) " /proc/mounts || { echo "$MNT is not a CephFS (or NFSv4) mount" >&2; exit 2; }

D=$MNT/smoke.$$.$RANDOM
SRC=$(mktemp)
trap 'rm -f "$SRC"' EXIT
dd if=/dev/urandom of="$SRC" bs=1M count="$MB" status=none
want=$(md5sum < "$SRC" | cut -d' ' -f1)
fail=0
step() {   # step <description> <command...>
    local what=$1; shift
    if "$@"; then echo "PASS  $what"; else echo "FAIL  $what"; fail=1; fi
}
same_md5() { [ "$(md5sum < "$1" | cut -d' ' -f1)" = "$want" ]; }
in_rados() {   # first object of the file is in the data pool
    local obj
    obj=$(printf '%x.00000000' "$(stat -c %i "$1")")
    timeout 30 rados -c "$CONF" -p cephfs.a.data stat "$obj" >/dev/null
}

step "mkdir $D"                         mkdir "$D"
step "create+write f1 ($MB MiB, dd)"    dd if="$SRC" of="$D/f1" bs=1M conv=fsync status=none
step "create+write f2 (small, echo)"    sh -c "echo hello-cephfs > '$D/f2'"
step "read back f1 (md5sum)"            same_md5 "$D/f1"
step "read back f2 (cat)"               sh -c "[ \"\$(cat '$D/f2')\" = hello-cephfs ]"
step "f1 data object in data pool"      in_rados "$D/f1"
step "ls -l"                            sh -c "ls -l '$D' | sed 's/^/      /'"
step "stat size of f1"                  sh -c "[ \$(stat -c %s '$D/f1') = $((MB * 1048576)) ]"
step "mkdir sub, rename f1 into it"     sh -c "mkdir '$D/sub' && mv '$D/f1' '$D/sub/f1.renamed'"
step "rename f2 in place (mv)"          mv "$D/f2" "$D/f2.renamed"
step "old names gone, new names there"  sh -c "[ ! -e '$D/f1' ] && [ ! -e '$D/f2' ] && [ -f '$D/sub/f1.renamed' ] && [ -f '$D/f2.renamed' ]"
step "read renamed f1 (md5sum)"         same_md5 "$D/sub/f1.renamed"
step "unlink (rm) files"                rm "$D/sub/f1.renamed" "$D/f2.renamed"
step "rmdir sub and test dir"           sh -c "rmdir '$D/sub' && rmdir '$D'"
step "test dir gone"                    sh -c "[ ! -e '$D' ]"
if [ $fail = 0 ]; then echo "smoke test PASSED on $MNT"; else echo "smoke test FAILED on $MNT"; fi
exit $fail
