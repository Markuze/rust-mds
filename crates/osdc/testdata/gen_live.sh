#!/usr/bin/env bash
# Snapshot the dev cluster's OSDMap history into testdata/live/: every epoch's full map and
# incremental from a copy of the monitor store (ceph-monstore-tool needs exclusive access, so
# the live store is copied first, as testdata/dump-maps.sh does), the CRUSH map
# (`ceph osd getcrushmap`), and `osdmaptool --test-map-pgs-dump-all` of each full map.
#
#   crates/osdc/testdata/gen_live.sh <workdir>
set -euo pipefail
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
WORK=${1:?usage: gen_live.sh <workdir>}
source "$HERE/../../../devcluster/env.sh"
OUT=$HERE/live
rm -rf "$WORK" && mkdir -p "$WORK" "$OUT"
cp -a "$DEVCLUSTER_DATA/mon/a" "$WORK/mon"
rm -f "$WORK/mon/store.db/LOCK"
LAST=$(ceph-monstore-tool "$WORK/mon" show-versions -- --map-type osdmap 2>/dev/null | awk '/last/ {print $NF}')
rm -f "$OUT"/full.* "$OUT"/inc.* "$OUT"/pgs.* "$OUT"/osdmap.*
for e in $(seq 1 "$LAST"); do
  ceph-monstore-tool "$WORK/mon" get osdmap -- --version "$e" --out "$OUT/full.$e" >/dev/null 2>&1
  ceph-monstore-tool "$WORK/mon" get-key osdmap "$e" -- --out "$OUT/inc.$e" >/dev/null 2>&1
  osdmaptool "$OUT/full.$e" --test-map-pgs-dump-all 2>/dev/null | grep -E '^[0-9]+\.[0-9a-f]+ raw' > "$OUT/pgs.$e" || true
done
timeout 30 ceph osd getcrushmap -o "$OUT/crush.bin" 2>/dev/null
# keep every incremental, full maps (and pg dumps) only for every 3rd epoch and the last 6
for e in $(seq 1 "$LAST"); do
  if [ $((e % 3)) -ne 1 ] && [ "$e" -lt $((LAST - 5)) ]; then
    rm -f "$OUT/full.$e" "$OUT/pgs.$e"
  fi
done
echo "epochs 1..$LAST"

# Object placement from the monitor (`ceph osd map <pool> <object> [<nspace>]`): raw hash pg,
# pg, up and acting sets, for MDS-style and other names, with and without a namespace.
python3 -I - "$OUT/objects.tsv" <<'PY'
import json, subprocess, sys
names = ["1.00000000", "100.00000000", "200.00000000", "200.00000001", "300.00000000", "400.00000000",
         "500.00000000", "600.00000000", "601.00000000", "609.00000000", "mds0_inotable", "mds0_sessionmap",
         "mds_snaptable", "mds0_openfiles.0", "1.00000000.inode", "100.00000000.inode",
         "10000000000.00000000", "10000000000.00000001", "1000000000a.00000003", "10000000123.0000abcd",
         "10000000001.00000000", "1099511627776.00000000", "", "a", "ab", "abc", "abcd", "abcdefghijk",
         "abcdefghijkl", "abcdefghijklm", "x" * 40, "rbd_header.1234", "ünïcödé", "dir/with/slashes",
         "trailing space ", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12"]
pools = [p for p in json.loads(subprocess.check_output(["ceph", "osd", "pool", "ls", "detail", "-f", "json"]))]
out = open(sys.argv[1], "w")
for p in pools:
    if not p["pool_name"].startswith("cephfs."):
        continue
    for ns in ["", "ns1"]:
        for n in names:
            if n == "":
                continue
            args = ["ceph", "osd", "map", p["pool_name"], n] + ([ns] if ns else []) + ["-f", "json"]
            j = json.loads(subprocess.check_output(args))
            out.write("\t".join([str(j["epoch"]), str(j["pool_id"]), ns, n, j["raw_pgid"], j["pgid"],
                                 ",".join(map(str, j["up"])), str(j["up_primary"]),
                                 ",".join(map(str, j["acting"])), str(j["acting_primary"])]) + "\n")
PY
echo "$(wc -l < "$OUT/objects.tsv") object mappings"
