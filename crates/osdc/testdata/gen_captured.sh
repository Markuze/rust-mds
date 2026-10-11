#!/usr/bin/env bash
# Copy the MOSDMap messages of the env agent's msgr1 captures (testdata/captures, see its
# README) and dump their placement with osdmaptool: the monitor's MOSDMap to the C++ MDS
# (canonical encoding) and to the kernel client (re-encoded for the kernel's features: older
# OSDMap, pg_pool_t and CRUSH encodings).
#
#   crates/osdc/testdata/gen_captured.sh
set -euo pipefail
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CAP=$HERE/../../../testdata/captures
OUT=$HERE/captured
mkdir -p "$OUT"
cp "$CAP/mds-v1-boot.msgs/0024-c0-s2c-MOSDMap.front.bin" "$OUT/mosdmap-to-mds.front.bin"
cp "$CAP/kernel-v1.msgs/0010-c0-s2c-MOSDMap.front.bin" "$OUT/mosdmap-to-kernel.front.bin"
# the full map inside: fsid(16), u32 n incrementals (0 here), u32 n maps, u32 epoch, u32 len, map
python3 -I - "$OUT" <<'PY'
import struct, sys
out = sys.argv[1]
for name in ["mosdmap-to-mds", "mosdmap-to-kernel"]:
    b = open(f"{out}/{name}.front.bin", "rb").read()
    off = 16
    (ninc,) = struct.unpack_from("<I", b, off); off += 4
    assert ninc == 0, ninc
    (nmaps,) = struct.unpack_from("<I", b, off); off += 4
    epoch, ln = struct.unpack_from("<II", b, off); off += 8
    open(f"{out}/{name}.map.bin", "wb").write(b[off:off + ln])
    print(name, "epoch", epoch, "len", ln)
PY
for name in mosdmap-to-mds mosdmap-to-kernel; do
  osdmaptool "$OUT/$name.map.bin" --test-map-pgs-dump-all 2>/dev/null | grep -E '^[0-9]+\.[0-9a-f]+ raw' > "$OUT/$name.pgs"
done
