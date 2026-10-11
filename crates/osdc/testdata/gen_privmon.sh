#!/usr/bin/env bash
# Generate OSDMap fixtures (full maps and incrementals) with a throwaway private cluster that
# runs only while this script runs: one monitor and six memstore OSDs on three hosts, auth
# none, ports 47200-47299, everything under <workdir>.
#
#   testdata/gen_privmon.sh <workdir>
#
# The script drives a series of OSDMap changes: replicated, EC and device-class pools, pool
# snapshots (removed/purged snaps), pg_temp, primary_temp, pg_upmap, pg_upmap_items,
# pg_upmap_primary, primary affinity, reweights, out/in, OSDs going down and away, blocklist
# and range blocklist entries, PAUSE/noout flags, compat and per-pool weight sets
# (choose_args), pg_num changes, pool deletion. Then it stops the daemons and extracts every
# epoch's full map and incremental from the monitor store with ceph-monstore-tool into
# testdata/privmon/{full,inc}.<epoch>, plus `osdmaptool --test-map-pgs-dump-all` of each full
# map into privmon/pgs.<epoch> (full maps are kept for a subset of the epochs, see the end).
set -euo pipefail
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WORK=${1:?usage: gen_privmon.sh <workdir>}
OUT=$HERE/privmon
rm -rf "$WORK" && mkdir -p "$WORK/mon" "$WORK/run" "$OUT"
FSID=$(cat /proc/sys/kernel/random/uuid)
CONF=$WORK/ceph.conf
cat > "$CONF" <<EOF
[global]
fsid = $FSID
mon_host = [v2:127.0.0.1:47200,v1:127.0.0.1:47201]
auth_cluster_required = none
auth_service_required = none
auth_client_required = none
mon_allow_pool_delete = true
mon_osd_allow_primary_temp = true
osd_pool_default_size = 3
osd_pool_default_min_size = 2
osd_pool_default_pg_autoscale_mode = off
osd_crush_chooseleaf_type = 1
osd_crush_update_on_start = false
mon_max_pg_per_osd = 2000
ms_bind_port_min = 47210
ms_bind_port_max = 47299
osd_objectstore = memstore
memstore_device_bytes = 268435456
osd_data = $WORK/\$name
mon_data = $WORK/mon
run_dir = $WORK/run
log_file = $WORK/\$name.log
admin_socket =
pid_file = $WORK/run/\$name.pid
mon_cluster_log_file = $WORK/cluster.log
EOF
monmaptool --create --addv a "[v2:127.0.0.1:47200,v1:127.0.0.1:47201]" --fsid "$FSID" "$WORK/monmap" >/dev/null
ceph-mon --mkfs -i a --monmap "$WORK/monmap" -c "$CONF" 2>>"$WORK/mkfs.log"
PIDS=()
cleanup() { for p in "${PIDS[@]}"; do kill "$p" 2>/dev/null || true; done; wait 2>/dev/null || true; }
trap cleanup EXIT
ceph-mon -i a -c "$CONF" -f >>"$WORK/mon.stdout" 2>&1 &
PIDS+=($!)
c() { timeout 60 ceph -c "$CONF" "$@"; }
for _ in $(seq 60); do c -s >/dev/null 2>&1 && break; sleep 0.5; done

declare -A OSDPID
start_osd() { ceph-osd -c "$CONF" -i "$1" -f >>"$WORK/osd.$1.stdout" 2>&1 & OSDPID[$1]=$!; PIDS+=($!); }
for h in 0 1 2; do c osd crush add-bucket h$h host >/dev/null; c osd crush move h$h root=default >/dev/null; done
for i in 0 1 2 3 4 5; do
  uuid=$(cat /proc/sys/kernel/random/uuid)
  c osd new "$uuid" >/dev/null
  mkdir -p "$WORK/osd.$i"
  ceph-osd -c "$CONF" -i $i --mkfs --osd-uuid "$uuid" >>"$WORK/mkfs.log" 2>&1
  c osd crush add osd.$i $(( (i % 3) + 1 )).0 host=h$(( i / 2 )) >/dev/null
  start_osd $i
done
wait_up() { # wait_up <n>
  for _ in $(seq 120); do
    n=$(c osd dump -f json | python3 -I -c 'import json,sys; print(sum(1 for o in json.load(sys.stdin)["osds"] if o["up"]))')
    [ "$n" -ge "$1" ] && return 0; sleep 0.5
  done
  echo "timeout waiting for $1 osds up" >&2; return 1
}
wait_up 6
pool_id() { c osd pool ls detail -f json | python3 -I -c "import json,sys; print([p['pool_id'] for p in json.load(sys.stdin) if p['pool_name']=='$1'][0])"; }
acting() { c pg map "$1" -f json | python3 -I -c 'import json,sys; print(" ".join(map(str, json.load(sys.stdin)["acting"])))'; }

c osd set-require-min-compat-client luminous >/dev/null
c osd pool create rep 32 32 replicated >/dev/null
c osd erasure-code-profile set ec21 k=2 m=1 crush-failure-domain=host >/dev/null
c osd pool create ec 16 16 erasure ec21 >/dev/null
c osd pool create snappy 8 8 >/dev/null
REP=$(pool_id rep); EC=$(pool_id ec)
sleep 2
c osd pool mksnap snappy s1 >/dev/null
c osd pool mksnap snappy s2 >/dev/null
c osd pool rmsnap snappy s1 >/dev/null
# pg_temp / primary_temp (the primaries may undo them a few epochs later)
read -r a b cc <<<"$(acting $REP.0)"; c osd pg-temp $REP.0 $cc $a $b >/dev/null
read -r a b cc <<<"$(acting $REP.1)"; c osd pg-temp $REP.1 $b $a >/dev/null
read -r a b cc <<<"$(acting $EC.2)"; c osd pg-temp $EC.2 $b $cc $a >/dev/null
read -r a b cc <<<"$(acting $REP.2)"; c osd primary-temp $REP.2 $cc >/dev/null
# upmaps
c osd pg-upmap $REP.3 1 3 5 >/dev/null
read -r a b cc <<<"$(acting $REP.4)"
for o in 0 1 2 3 4 5; do case " $a $b $cc " in *" $o "*) ;; *) c osd pg-upmap-items $REP.4 $a $o >/dev/null; break;; esac; done
read -r a b cc <<<"$(acting $EC.3)"
for o in 0 1 2 3 4 5; do case " $a $b $cc " in *" $o "*) ;; *) c osd pg-upmap-items $EC.3 $b $o >/dev/null; break;; esac; done
c osd set-require-min-compat-client reef --yes-i-really-mean-it >/dev/null
read -r a b cc <<<"$(acting $REP.6)"; c osd pg-upmap-primary $REP.6 $b >/dev/null
read -r a b cc <<<"$(acting $REP.7)"; c osd pg-upmap-primary $REP.7 $cc >/dev/null
sleep 2
# primary affinity, reweight, out/in
c osd primary-affinity osd.1 0.3 >/dev/null
c osd primary-affinity osd.4 0 >/dev/null
c osd reweight 5 0.6 >/dev/null
c osd out 3 >/dev/null
sleep 2
c osd in 3 >/dev/null
# blocklists and flags
c osd blocklist add 192.168.1.5:0/123 >/dev/null
c osd blocklist add 10.0.0.1:6789/77 >/dev/null
c osd blocklist add 10.0.0.9 >/dev/null
c osd blocklist range add 10.1.0.0/16 >/dev/null
c osd blocklist rm 192.168.1.5:0/123 >/dev/null
c osd set noout >/dev/null
c osd set pause >/dev/null
c osd unset pause >/dev/null
# device classes and a class rule
c osd crush rm-device-class osd.4 osd.5 >/dev/null 2>&1 || true
c osd crush set-device-class ssd osd.4 osd.5 >/dev/null
c osd crush rule create-replicated ssdrule default osd ssd >/dev/null
c osd pool create ssdpool 8 8 replicated ssdrule >/dev/null
# weight sets (choose_args): compat (-1) and per-pool positional
c osd crush weight-set create-compat >/dev/null
c osd crush weight-set reweight-compat osd.0 0.5 >/dev/null
c osd crush weight-set create rep positional >/dev/null
c osd crush weight-set reweight rep osd.1 0.8 0.4 0.2 >/dev/null
# pg_num changes (split), min_size change
c osd pool set rep pg_num_actual 40 >/dev/null 2>&1 || true
c osd pool set rep pgp_num_actual 40 >/dev/null 2>&1 || true
c osd pool set ec min_size 2 >/dev/null
sleep 2
# an OSD goes down (daemon stopped) and is marked out; another is marked down and reboots
kill "${OSDPID[5]}"; c osd down 5 >/dev/null; c osd out 5 >/dev/null
c osd down 2 >/dev/null
sleep 3
c osd unset noout >/dev/null
c osd pool rm snappy snappy --yes-i-really-really-mean-it >/dev/null
c osd rm-pg-upmap-items $REP.4 >/dev/null 2>&1 || true
sleep 2
LAST=$(c osd dump -f json | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["epoch"])')
cleanup
trap - EXIT

rm -f "$OUT"/full.* "$OUT"/inc.* "$OUT"/pgs.* "$OUT"/up.*
for e in $(seq 1 "$LAST"); do
  ceph-monstore-tool "$WORK/mon" get osdmap -- --version "$e" --out "$OUT/full.$e" >/dev/null 2>&1
  ceph-monstore-tool "$WORK/mon" get-key osdmap "$e" -- --out "$OUT/inc.$e" >/dev/null 2>&1
  osdmaptool "$OUT/full.$e" --test-map-pgs-dump-all 2>/dev/null | grep -E '^[0-9]+\.[0-9a-f]+ raw' > "$OUT/pgs.$e" || true
done
# keep every incremental, but full maps (and their pg dumps) only for every 5th epoch, the
# feature-rich middle of the history and the last epochs, to keep the fixtures small
for e in $(seq 1 "$LAST"); do
  if [ $((e % 5)) -ne 0 ] && [ "$e" -ne 1 ] && { [ "$e" -lt 28 ] || [ "$e" -gt 62 ]; } && [ "$e" -lt $((LAST - 3)) ]; then
    rm -f "$OUT/full.$e" "$OUT/pgs.$e"
  fi
done
echo "epochs 1..$LAST, $(cat "$OUT"/pgs.* | wc -l) pg mappings"
