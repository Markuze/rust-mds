#!/usr/bin/env bash
# End-to-end smoke test of the Rust MDS against the dev cluster.
#
# Starts ceph-rmds as mds.rust, waits until it holds rank 0 of fs "a" in up:active,
# mounts with the kernel client over msgr1 and with ceph-fuse, does simple I/O and checks it.
#
# usage: scripts/smoke.sh [--keep] [--release]
#   --keep     leave ceph-rmds running and the mounts in place on success
#   --release  build and run the release binary
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
KEEP=0
PROFILE=debug
for a in "$@"; do
  case $a in
    --keep) KEEP=1 ;;
    --release) PROFILE=release ;;
    *) echo "unknown argument $a" >&2; exit 2 ;;
  esac
done

# shellcheck source=/dev/null
source "$ROOT/devcluster/env.sh"

RUN=${RMDS_RUN_DIR:-/mnt/workspace/devcluster-data/rmds}
mkdir -p "$RUN"
LOG="$RUN/ceph-rmds.log"
KMNT="$RUN/mnt-kernel"
FMNT="$RUN/mnt-fuse"
mkdir -p "$KMNT" "$FMNT"

log() { printf '[smoke] %s\n' "$*"; }
fail() { printf '[smoke] FAIL: %s\n' "$*" >&2; exit 1; }

cleanup() {
  local rc=$?
  if [[ $rc -ne 0 || $KEEP -eq 0 ]]; then
    "$ROOT/devcluster/umount.sh" "$KMNT" >/dev/null 2>&1 || true
    "$ROOT/devcluster/umount.sh" "$FMNT" >/dev/null 2>&1 || true
    [[ -n "${RMDS_PID:-}" ]] && kill "$RMDS_PID" 2>/dev/null || true
  fi
  [[ $rc -ne 0 ]] && { log "last 40 lines of $LOG:"; tail -n 40 "$LOG" >&2 || true; }
  exit $rc
}
trap cleanup EXIT

log "building ceph-rmds ($PROFILE)"
if [[ $PROFILE == release ]]; then
  (cd "$ROOT" && cargo build --release -p rmds)
else
  (cd "$ROOT" && cargo build -p rmds)
fi
BIN="$ROOT/target/$PROFILE/ceph-rmds"

if ceph fs dump 2>/dev/null | grep -qE "mds\.a\b.*up:"; then
  fail "the C++ MDS holds a rank; restart the cluster with devcluster/start.sh --mds none"
fi

log "starting ceph-rmds"
"$BIN" -i rust -c "$CEPH_CONF" --keyring "$RMDS_KEYRING" >"$LOG" 2>&1 &
RMDS_PID=$!

log "waiting for rank 0 up:active"
for _ in $(seq 1 60); do
  if ceph fs dump 2>/dev/null | grep -qE "\[mds\.rust\{0:[0-9]+\} state up:active"; then
    break
  fi
  kill -0 "$RMDS_PID" 2>/dev/null || fail "ceph-rmds exited"
  sleep 1
done
ceph fs dump 2>/dev/null | grep -qE "\[mds\.rust\{0:[0-9]+\} state up:active" || fail "mds.rust did not reach up:active"
log "mds.rust is up:active"

io_check() {
  local mnt=$1 tag=$2
  local d="$mnt/smoke-$tag-$$"
  mkdir "$d" || fail "$tag: mkdir"
  dd if=/dev/urandom of="$d/f1" bs=1M count=8 status=none || fail "$tag: write"
  local sum1 sum2
  sum1=$(md5sum <"$d/f1" | cut -d' ' -f1)
  sync
  echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null
  sum2=$(md5sum <"$d/f1" | cut -d' ' -f1)
  [[ $sum1 == "$sum2" ]] || fail "$tag: read-back mismatch"
  [[ $(stat -c %s "$d/f1") -eq $((8 * 1024 * 1024)) ]] || fail "$tag: size"
  echo hello >"$d/small" || fail "$tag: small write"
  [[ $(cat "$d/small") == hello ]] || fail "$tag: small read"
  mv "$d/small" "$d/renamed" || fail "$tag: rename"
  ln "$d/renamed" "$d/hardlink" || fail "$tag: link"
  [[ $(stat -c %h "$d/renamed") -eq 2 ]] || fail "$tag: nlink"
  ls -l "$d" >/dev/null || fail "$tag: readdir"
  rm "$d/renamed" "$d/hardlink" "$d/f1" || fail "$tag: unlink"
  rmdir "$d" || fail "$tag: rmdir"
  log "$tag: I/O ok"
}

log "kernel mount over msgr1"
"$ROOT/devcluster/mount-kernel.sh" "$KMNT" v1 || fail "kernel mount"
io_check "$KMNT" kernel

log "ceph-fuse mount"
"$ROOT/devcluster/mount-fuse.sh" "$FMNT" || fail "ceph-fuse mount"
io_check "$FMNT" fuse

log "cross-client visibility"
echo cross >"$KMNT/cross-$$"
sleep 1
[[ $(cat "$FMNT/cross-$$") == cross ]] || fail "fuse does not see the kernel's file"
rm "$FMNT/cross-$$"
[[ ! -e "$KMNT/cross-$$" ]] || fail "kernel still sees the file removed through fuse"

log "PASS"
