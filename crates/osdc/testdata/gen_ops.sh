#!/usr/bin/env bash
# Record golden MOSDOp/MOSDOpReply messages: a throwaway msgr1-only private cluster (one
# monitor, three memstore OSDs, auth none, ports 47300-47399, under <workdir>) and the C++
# Objecter in librados driven by ops_driver.c, captured with tcpdump on lo and split with
# testdata/msgr1_split.py. Output: testdata/ops/ (the split messages of the client<->OSD
# connections, index.tsv, connections.tsv, the driver log and the pool id).
#
#   crates/osdc/testdata/gen_ops.sh <workdir>
set -euo pipefail
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
SPLIT=$HERE/../../../testdata/msgr1_split.py
WORK=${1:?usage: gen_ops.sh <workdir>}
OUT=$HERE/ops
rm -rf "$WORK" && mkdir -p "$WORK/mon" "$WORK/run"
gcc -O1 -I/home/ubuntu/ceph/src/include -o "$WORK/ops_driver" "$HERE/ops_driver.c" /usr/lib/x86_64-linux-gnu/librados.so.2
FSID=$(cat /proc/sys/kernel/random/uuid)
CONF=$WORK/ceph.conf
cat > "$CONF" <<EOC
[global]
fsid = $FSID
mon_host = v1:127.0.0.1:47301
ms_bind_msgr2 = false
auth_cluster_required = none
auth_service_required = none
auth_client_required = none
mon_allow_pool_delete = true
osd_pool_default_size = 3
osd_pool_default_min_size = 2
osd_pool_default_pg_autoscale_mode = off
osd_crush_chooseleaf_type = 0
mon_max_pg_per_osd = 2000
ms_bind_port_min = 47310
ms_bind_port_max = 47399
osd_objectstore = memstore
memstore_device_bytes = 268435456
osd_data = $WORK/\$name
mon_data = $WORK/mon
run_dir = $WORK/run
log_file = $WORK/\$name.log
admin_socket =
pid_file = $WORK/run/\$name.pid
mon_cluster_log_file = $WORK/cluster.log
EOC
monmaptool --create --add a v1:127.0.0.1:47301 --fsid "$FSID" "$WORK/monmap" >/dev/null
ceph-mon --mkfs -i a --monmap "$WORK/monmap" -c "$CONF" 2>>"$WORK/mkfs.log"
PIDS=()
TCPDUMP=
cleanup() {
  [ -n "$TCPDUMP" ] && sudo kill -INT "$TCPDUMP" 2>/dev/null || true
  for p in "${PIDS[@]}"; do kill "$p" 2>/dev/null || true; done; wait 2>/dev/null || true
}
trap cleanup EXIT
ceph-mon -i a -c "$CONF" -f >>"$WORK/mon.stdout" 2>&1 &
PIDS+=($!)
c() { timeout 60 ceph -c "$CONF" "$@"; }
for _ in $(seq 60); do c -s >/dev/null 2>&1 && break; sleep 0.5; done
for i in 0 1 2; do
  uuid=$(cat /proc/sys/kernel/random/uuid)
  c osd new "$uuid" >/dev/null
  mkdir -p "$WORK/osd.$i"
  ceph-osd -c "$CONF" -i $i --mkfs --osd-uuid "$uuid" >>"$WORK/mkfs.log" 2>&1
  ceph-osd -c "$CONF" -i $i -f >>"$WORK/osd.$i.stdout" 2>&1 &
  PIDS+=($!)
done
for _ in $(seq 120); do
  n=$(c osd dump -f json | python3 -I -c 'import json,sys; print(sum(1 for o in json.load(sys.stdin)["osds"] if o["up"]))')
  [ "$n" -ge 3 ] && break; sleep 0.5
done
c osd pool create osdc 8 8 >/dev/null
POOL=$(c osd pool ls detail -f json | python3 -I -c "import json,sys; print([p['pool_id'] for p in json.load(sys.stdin) if p['pool_name']=='osdc'][0])")
sleep 8   # let the pgs peer (no mgr, so no pg stat)

sudo bash -c "echo \$\$ > '$WORK/tcpdump.pid'; exec tcpdump -i lo -s 0 -U -Z '$(id -un)' -w '$WORK/ops.pcap' 'tcp portrange 47310-47399'" 2>"$WORK/tcpdump.log" &
for _ in $(seq 50); do grep -q 'listening on' "$WORK/tcpdump.log" 2>/dev/null && break; sleep 0.1; done
TCPDUMP=$(cat "$WORK/tcpdump.pid")
"$WORK/ops_driver" "$CONF" osdc > "$WORK/driver.log" 2>&1
sleep 1
sudo kill -INT "$TCPDUMP"; TCPDUMP=
sleep 1
cleanup
trap - EXIT

rm -rf "$OUT" && mkdir -p "$OUT"
python3 -I "$SPLIT" "$WORK/ops.pcap" "$WORK/split" >/dev/null
cp "$WORK/driver.log" "$OUT/driver.log"
echo "$POOL" > "$OUT/pool_id"
cp "$WORK/split/index.tsv" "$WORK/split/connections.tsv" "$OUT/"
# keep the client<->OSD op traffic only
# (the .dencoder.bin copies are not kept: they are front/middle/data in encode_message() layout)
for f in "$WORK"/split/*-MOSDOp.* "$WORK"/split/*-MOSDOpReply.* "$WORK"/split/*-MOSDBackoff.*; do
  case "$f" in *.dencoder.bin) continue;; esac
  [ -e "$f" ] && cp "$f" "$OUT/"
done
ls "$OUT" | grep -c 'front.bin' | sed 's/^/messages: /'
