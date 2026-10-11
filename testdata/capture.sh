#!/usr/bin/env bash
# Record reference captures of the C++ MDS's traffic on the dev cluster.
#
#   testdata/capture.sh <scenario>
#
#   kernel-v1     kernel client, msgr1 (no ms_mode) <-> C++ MDS (v2+v1 bound) and mon.
#                 Needs: devcluster/start.sh --mds cpp
#   fuse-v2       ceph-fuse <-> C++ MDS over msgr2 (crc mode; port 7400 only).
#                 Needs: devcluster/start.sh --mds cpp
#   mds-v1-boot   C++ MDS bound to msgr1 only, from process start to up:active and a
#                 few beacons, on a fresh fs: MDS <-> mon (6789) and MDS <-> mgr (v1).
#                 Runs start.sh --mds none, newfs.sh, start.sh --mds cpp-v1 itself.
#   fuse-v1       ceph-fuse (-m v1:127.0.0.1:6789) <-> msgr1-only C++ MDS and mon.
#                 Needs: devcluster/start.sh --mds cpp-v1
#   kernel-v1-mdsv1  kernel v1 <-> msgr1-only C++ MDS. Needs: --mds cpp-v1
#
# Output in testdata/captures/<scenario>.*: .pcap, .steps.txt (workload steps with
# timestamps), .summary.txt and .tshark.txt (tshark Ceph dissector), .mds-msgs.txt
# (the C++ MDS's debug_ms=1 log lines for the window), .session.json (MDS view of
# the client session).
set -euo pipefail
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DEV=$HERE/../devcluster
source "$DEV/common.sh"
OUT=$HERE/captures
SCN=${1:-}
[ -n "$SCN" ] || { sed -n '2,22p' "$0"; exit 2; }
mkdir -p "$OUT"
PCAP=$OUT/$SCN.pcap
STEPS=$OUT/$SCN.steps.txt
TMP=$(mktemp -d)
MNT=/mnt/capture-$SCN
MDS_ASOK=$RUN_DIR/ceph-mds.$CPP_MDS_ID.asok

mds_mode() { tr '\0' ' ' < "/proc/$(daemon_pid mds.$CPP_MDS_ID)/cmdline" 2>/dev/null | grep -q 'ms_bind_msgr2=false' && echo cpp-v1 || echo cpp; }
need_mds() {
    [ -n "$(daemon_pid mds.$CPP_MDS_ID)" ] || die "C++ MDS not running; run devcluster/start.sh --mds $1"
    [ "$(mds_mode)" = "$1" ] || die "C++ MDS runs in mode $(mds_mode); run devcluster/start.sh --mds $1"
}
mgr_v1_port() { c mgr dump --format json | python3 -c 'import json,sys; print([a["addr"].split(":")[1] for a in json.load(sys.stdin)["active_addrs"]["addrvec"] if a["type"]=="v1"][0])'; }

step() { echo "$(date +%s.%N) $(date -u +%FT%T.%N | cut -c1-23) $*" >> "$STEPS"; }

start_capture() {   # start_capture <bpf filter>
    rm -f "$PCAP"
    : > "$STEPS"
    echo "# scenario $SCN, filter: $1" >> "$STEPS"
    echo "# columns: unix_time utc_time step" >> "$STEPS"
    sudo bash -c "echo \$\$ > '$TMP/tcpdump.pid'; exec tcpdump -i lo -s 0 -U -Z '$(id -un)' -w '$PCAP' '$1'" 2>"$TMP/tcpdump.log" &
    for _ in $(seq 50); do grep -q 'listening on' "$TMP/tcpdump.log" 2>/dev/null && break; sleep 0.1; done
    grep -q 'listening on' "$TMP/tcpdump.log" || die "tcpdump did not start: $(cat "$TMP/tcpdump.log")"
    step "capture started"
}

stop_capture() {
    sleep 2
    step "capture stopped"
    sudo kill -INT "$(cat "$TMP/tcpdump.pid")"
    wait || true
    grep -E 'packets (captured|received)' "$TMP/tcpdump.log" | sed 's/^/# tcpdump: /' >> "$STEPS"
}

# The file-system workload, run as root in one sudo call.
workload() {   # workload <mountpoint>
    sudo env M="$1" STEPS="$STEPS" bash -s <<'EOF'
set -e
step() { echo "$(date +%s.%N) $(date -u +%FT%T.%N | cut -c1-23) $*" >> "$STEPS"; }
s() { step "$*"; eval "$*" > /dev/null; sleep 1; }
s "mkdir $M/dir1"
s "dd if=/dev/urandom of=$M/dir1/file1 bs=64k count=16 conv=fsync status=none"
s "echo hello-cephfs > $M/dir1/file2"
s "sync; echo 3 > /proc/sys/vm/drop_caches"
s "md5sum $M/dir1/file1"
s "cat $M/dir1/file2"
s "ls -l $M/dir1"
s "mv $M/dir1/file2 $M/dir1/file2.renamed"
s "rm $M/dir1/file1 $M/dir1/file2.renamed"
s "rmdir $M/dir1"
EOF
}

session_json() { timeout 20 ceph --admin-daemon "$MDS_ASOK" session ls > "$OUT/$SCN.session.json" 2>/dev/null || true; }

postprocess() {
    # tshark's AppArmor profile does not allow reading under /mnt/workspace: copy to /tmp.
    # Its Ceph dissector only understands msgr1 ("ceph v027" banner).
    rm -f "$OUT/$SCN.summary.txt" "$OUT/$SCN.tshark.txt"
    if grep -aq 'ceph v027' "$PCAP"; then
        cp "$PCAP" "$TMP/c.pcap"
        tshark -r "$TMP/c.pcap" -Y ceph > "$OUT/$SCN.summary.txt" 2>/dev/null || true
        tshark -r "$TMP/c.pcap" -Y ceph -O ceph > "$OUT/$SCN.tshark.txt" 2>/dev/null || true
    fi
    # The C++ MDS's debug_ms=1 lines for the capture window, without its RADOS
    # (OSD) traffic and messages to itself.
    WIN_START=$(awk '!/^#/ {print $2; exit}' "$STEPS")
    WIN_END=$(awk '!/^#/ {t=$2} END {print t}' "$STEPS")
    awk -v s="$WIN_START" -v e="$WIN_END" 'substr($1,1,23) >= s && substr($1,1,23) <= e && / (<==|-->) /' \
        "$LOG_DIR/ceph-mds.$CPP_MDS_ID.log" |
        grep -v -E 'osd_op|osd_map|<== osd\.|<== mds\.0 |mdsping|mds_metrics from rank' \
        > "$OUT/$SCN.mds-msgs.txt" || true
    rm -rf "$OUT/$SCN.msgs"
    python3 -I "$HERE/msgr1_split.py" "$PCAP" "$OUT/$SCN.msgs" >&2 || true
    [ -s "$OUT/$SCN.msgs/index.tsv" ] && [ "$(wc -l < "$OUT/$SCN.msgs/index.tsv")" -gt 1 ] || rm -rf "$OUT/$SCN.msgs"
    ls -lad "$OUT/$SCN".* >&2
}

trap 'rm -rf "$TMP"' EXIT
if [ "$SCN" = --post ]; then    # redo the post-processing of an existing capture
    SCN=${2:?scenario}; PCAP=$OUT/$SCN.pcap; STEPS=$OUT/$SCN.steps.txt
    postprocess
    exit 0
fi
case $SCN in
    kernel-v1|kernel-v1-mdsv1)
        if [ "$SCN" = kernel-v1 ]; then need_mds cpp; else need_mds cpp-v1; fi
        start_capture "tcp port 6789 or tcp port $CPP_MDS_PORT_V1"
        step "mount (devcluster/mount-kernel.sh $MNT v1)"
        "$DEV/mount-kernel.sh" "$MNT" v1
        grep " $MNT " /proc/mounts | sed 's/^/# mount: /' >> "$STEPS"
        sleep 1
        session_json
        workload "$MNT"
        step "umount"
        "$DEV/umount.sh" "$MNT"
        stop_capture ;;
    fuse-v2)
        need_mds cpp
        start_capture "tcp port $CPP_MDS_PORT_V2"
        step "mount (devcluster/mount-fuse.sh $MNT)"
        "$DEV/mount-fuse.sh" "$MNT"
        sleep 1
        session_json
        workload "$MNT"
        step "umount"
        "$DEV/umount.sh" "$MNT"
        stop_capture ;;
    fuse-v1)
        need_mds cpp-v1
        start_capture "tcp port 6789 or tcp port $CPP_MDS_PORT_V1"
        step "mount (devcluster/mount-fuse.sh $MNT -m v1:$MON_V1)"
        "$DEV/mount-fuse.sh" "$MNT" -m "v1:$MON_V1"
        sleep 1
        session_json
        workload "$MNT"
        step "umount"
        "$DEV/umount.sh" "$MNT"
        stop_capture ;;
    mds-v1-boot)
        "$DEV/start.sh" --mds none
        "$DEV/newfs.sh"
        mport=$(mgr_v1_port)
        start_capture "tcp port 6789 or tcp port $mport"
        step "start C++ MDS msgr1-only (devcluster/start.sh --mds cpp-v1); mgr v1 port $mport"
        "$DEV/start.sh" --mds cpp-v1
        step "mds.a is up:active"
        sleep 20
        stop_capture ;;
    *) die "unknown scenario $SCN" ;;
esac
postprocess
