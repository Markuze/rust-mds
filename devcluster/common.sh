# shellcheck shell=bash
# Shared definitions for the devcluster scripts. Sourced by the other scripts;
# not meant to be run directly. Everything here is fixed on purpose: one
# cluster per machine, one data directory, fixed monitor ports.

DEVCLUSTER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA=/mnt/workspace/devcluster-data
CONF=$DATA/ceph.conf
KEYRING_DIR=$DATA/keyring
LOG_DIR=$DATA/log
RUN_DIR=$DATA/run

MON_ID=a
MGR_ID=x
OSD_IDS=(0 1 2)
CPP_MDS_ID=a
RUST_MDS_ID=rust
FS_NAME=a
META_POOL=cephfs.a.meta
DATA_POOL=cephfs.a.data
PG_NUM=8

MON_IP=127.0.0.1
MON_V2=$MON_IP:3300
MON_V1=$MON_IP:6789
# Fixed listening ports of the C++ MDS (outside the 6800-7300 range the other
# daemons pick random ports from). In --mds cpp-v1 mode it binds v1 only, on 7401.
CPP_MDS_PORT_V2=7400
CPP_MDS_PORT_V1=7401

ADMIN_KEYRING=$KEYRING_DIR/ceph.client.admin.keyring
ADMIN_SECRET=$KEYRING_DIR/client.admin.secret
RMDS_KEYRING=$KEYRING_DIR/ceph.mds.rust.keyring
CPP_MDS_KEYRING=$KEYRING_DIR/ceph.mds.a.keyring
MGR_KEYRING=$KEYRING_DIR/ceph.mgr.x.keyring
MON_KEYRING=$KEYRING_DIR/ceph.mon.keyring

# Caps cephadm/vstart give an MDS daemon.
MDS_CAPS=(mon 'allow profile mds' osd 'allow rw tag cephfs *=*' mds 'allow' mgr 'allow profile mds')

log() { echo "[devcluster $(date +%T)] $*" >&2; }
die() { log "ERROR: $*"; exit 1; }

# ceph CLI against this cluster, bounded so a dead monitor cannot hang a script.
c() { timeout 60 ceph -c "$CONF" "$@"; }

cluster_exists() { [ -f "$DATA/created" ]; }

# Daemon bookkeeping. Each daemon runs in the foreground (-f) of its own
# session (setsid + nohup), its pid is kept in $RUN_DIR/<type>.<id>.pid.
pidfile() { echo "$RUN_DIR/$1.pid"; }

# pid of a running daemon "<type>.<id>" or empty. Checks that the pid still
# belongs to a process using our ceph.conf (guards against pid reuse).
daemon_pid() {
    local f pid
    f=$(pidfile "$1")
    [ -f "$f" ] || return 0
    pid=$(cat "$f")
    if [ -n "$pid" ] && [ -r "/proc/$pid/cmdline" ] &&
        tr '\0' ' ' < "/proc/$pid/cmdline" | grep -q -- "$CONF"; then
        echo "$pid"
    fi
}

# start_daemon <type.id> <command...>
start_daemon() {
    local name=$1; shift
    local pid
    pid=$(daemon_pid "$name")
    if [ -n "$pid" ]; then
        log "$name already running (pid $pid)"
        return 0
    fi
    mkdir -p "$LOG_DIR" "$RUN_DIR"
    setsid nohup "$@" >>"$LOG_DIR/$name.stdout" 2>&1 </dev/null &
    echo $! > "$(pidfile "$name")"
    sleep 0.5
    pid=$(daemon_pid "$name")
    [ -n "$pid" ] || die "$name failed to start; see $LOG_DIR/$name.stdout and $LOG_DIR/ceph-$name.log"
    log "started $name (pid $pid): $*"
}

# stop_daemon <type.id> [timeout_s]: SIGTERM, wait, then SIGKILL.
stop_daemon() {
    local name=$1 t=${2:-30} pid i
    pid=$(daemon_pid "$name")
    if [ -z "$pid" ]; then
        rm -f "$(pidfile "$name")"
        return 0
    fi
    kill -TERM "$pid" 2>/dev/null || true
    for ((i = 0; i < t * 10; i++)); do
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.1
    done
    if kill -0 "$pid" 2>/dev/null; then
        log "$name (pid $pid) did not exit after ${t}s, sending SIGKILL"
        kill -KILL "$pid" 2>/dev/null || true
        sleep 0.5
    fi
    rm -f "$(pidfile "$name")"
    log "stopped $name"
}

# wait_until <timeout_s> <description> <command...>
wait_until() {
    local t=$1 what=$2 i; shift 2
    for ((i = 0; i < t; i++)); do
        if "$@" >/dev/null 2>&1; then return 0; fi
        sleep 1
    done
    die "timed out after ${t}s waiting for: $what"
}

# Mount points of file system a on this machine: kernel, ceph-fuse, and NFS
# mounts of our Ganesha (port 12049).
ceph_mounts() { awk '$3 == "ceph" || $3 == "fuse.ceph-fuse" || ($3 ~ /^nfs/ && $4 ~ /port=12049/) { print $2 }' /proc/mounts; }

# LD_PRELOAD shim working around the Ceph 20.2.0 MDSMap::decode crash in libcephfs
# users (see gctx-shim.c); built on demand.
GCTX_SHIM=$DATA/lib/gctx-shim.so
build_gctx_shim() {
    [ "$GCTX_SHIM" -nt "$DEVCLUSTER_DIR/gctx-shim.c" ] && return 0
    mkdir -p "$DATA/lib"
    gcc -shared -fPIC -O2 -o "$GCTX_SHIM" "$DEVCLUSTER_DIR/gctx-shim.c" -ldl
}
