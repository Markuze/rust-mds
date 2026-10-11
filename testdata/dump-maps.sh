#!/usr/bin/env bash
# Dump the cluster maps of the dev cluster into testdata/maps/ (JSON from the
# monitor, binaries from the monitor store and from the captures, ceph-dencoder
# dumps of the binaries), plus testdata/dencoder-types.txt.
#
#   testdata/dump-maps.sh
set -euo pipefail
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
source "$HERE/../devcluster/common.sh"
OUT=$HERE/maps
CAP=$HERE/captures
mkdir -p "$OUT"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

c fs dump > "$OUT/fs-dump.txt"
c fs dump --format json-pretty > "$OUT/fs-dump.json"
c fs get $FS_NAME --format json-pretty > "$OUT/fs-get-a.json"
c mds metadata --format json-pretty > "$OUT/mds-metadata.json"
c mon getmap -o "$OUT/monmap.bin" 2>/dev/null
monmaptool --print "$OUT/monmap.bin" > "$OUT/monmap.txt" 2>/dev/null
c mon dump --format json-pretty > "$OUT/mon-dump.json"
c osd pool ls detail --format json-pretty > "$OUT/osd-pool-ls-detail.json"
c auth get mds.$RUST_MDS_ID 2>/dev/null | grep -v 'key =' > "$OUT/auth-mds.rust-caps.txt"

# Binary FSMap as stored by the monitor (MDSMonitor), read from a copy of the
# live mon store (ceph-monstore-tool needs exclusive access).
cp -a "$DATA/mon/$MON_ID" "$TMP/mon"
rm -f "$TMP/mon/store.db/LOCK"
ceph-monstore-tool "$TMP/mon" get mdsmap -- --out "$OUT/fsmap.bin" > /dev/null 2>&1 ||
    echo "ceph-monstore-tool get mdsmap failed" >&2

# Embedded maps from captured msgr1 messages (front payload layouts:
# MMDSMap = fsid(16) epoch(u32) map(u32 len + bytes) [map_fs_name];
# MFSMapUser = epoch(u32) FSMapUser (rest); MMonMap = monmap(u32 len + bytes)).
python3 -I - "$CAP" "$OUT" <<'EOF'
import glob, os, struct, sys
cap, out = sys.argv[1], sys.argv[2]
def take(front, off):
    n = struct.unpack_from("<I", front, off)[0]
    return front[off + 4: off + 4 + n]
picks = [
    ("kernel-v1", "MMDSMap", "mdsmap-to-kernel", lambda f: take(f, 20)),
    ("kernel-v1", "MFSMapUser", "fsmapuser-to-kernel", lambda f: f[4:]),
    ("kernel-v1", "MMonMap", "monmap-to-kernel", lambda f: take(f, 0)),
    ("mds-v1-boot", "MMDSMap", "mdsmap-to-mds", lambda f: take(f, 20)),
]
for scn, typ, name, get in picks:
    files = sorted(glob.glob(os.path.join(cap, f"{scn}.msgs", f"*-s2c-{typ}.front.bin")))
    for i, fn in enumerate(files):
        blob = get(open(fn, "rb").read())
        dst = os.path.join(out, f"{name}.{i}.bin")
        open(dst, "wb").write(blob)
        print(dst, "from", os.path.basename(fn), len(blob), "bytes")
EOF

# ceph-dencoder JSON dumps of the binaries. Not for FSMap/MDSMap: in 20.2.0
# MDSMap::decode logs through g_ceph_context, which is NULL in ceph-dencoder, and
# segfaults (same crash as "ceph tell mds.*"); FSMapUser is not a dencoder type.
for f in "$OUT"/monmap*.bin; do
    ceph-dencoder type MonMap import "$f" decode dump_json > "${f%.bin}.json" 2>&1 ||
        mv "${f%.bin}.json" "${f%.bin}.json.err"
done
rm -f "$OUT"/fsmap*.json* "$OUT"/mdsmap-*.json* "$OUT"/fsmapuser-*.json*

# Key types of the keyring files (base64 CryptoKey: u16 type, utime created, u16 len, secret).
python3 -I - "$KEYRING_DIR" "$DATA" > "$OUT/keyring-key-types.txt" <<'EOF'
import base64, glob, os, struct, sys
kd, data = sys.argv[1], sys.argv[2]
names = {0: "CEPH_CRYPTO_NONE", 1: "CEPH_CRYPTO_AES", 2: "CEPH_CRYPTO_AES256KRB5"}
print("# file entity key_type secret_len (key = base64 of: u16 type, u32 sec, u32 nsec, u16 len, secret)")
for f in sorted(glob.glob(os.path.join(kd, "*.keyring"))) + sorted(glob.glob(os.path.join(data, "osd/*/keyring"))):
    ent = None
    for line in open(f):
        line = line.strip()
        if line.startswith("["):
            ent = line.strip("[]")
        elif line.startswith("key"):
            b = base64.b64decode(line.split("=", 1)[1].strip())
            t, sec, nsec, n = struct.unpack("<HIIH", b[:12])
            print(os.path.relpath(f, data), ent, f"{t}={names.get(t, '?')}", n)
EOF

ceph-dencoder list_types > "$HERE/dencoder-types.txt"
ls -la "$OUT" >&2
