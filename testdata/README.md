# testdata: reference data from the C++ MDS (Ceph 20.2.0)

Recorded on 2026-10-11 on the dev cluster (`devcluster/`, fsid
`3ceca2c6-851f-41cf-b538-1cfded4f1e82`), Ceph `20.2.0-0ubuntu2.1`, kernel `7.0.0-1012-ibm`,
ceph-fuse 20.2.0. `kernel-v1` and then `fuse-v2` ran on a file system freshly recreated
with `devcluster/newfs.sh` and created by the C++ MDS (`--mds cpp`); `mds-v1-boot`
recreated it again and records the msgr1-only C++ MDS creating it; `fuse-v1` ran right
after on that file system.

| Path | What |
|---|---|
| `captures/` | tcpdump captures on `lo` and everything derived from them (below) |
| `maps/` | FSMap/MDSMap/MonMap binaries and JSON dumps, MDS metadata, key types |
| `dencoder-types.txt` | `ceph-dencoder list_types` (675 types) |
| `capture.sh <scenario>` | re-records a capture (see its header); `capture.sh --post <scenario>` redoes the post-processing |
| `msgr1_split.py` | splits msgr1 TCP streams of a pcap into messages (stdlib Python, see below) |
| `dump-maps.sh` | regenerates `maps/` and `dencoder-types.txt` from the running cluster |

The cephx keys of `client.admin` and `mds.a` in force during the captures, which tests need
to decrypt the captured cephx exchanges, are not committed. They live in the git-ignored
`.env` at the workspace root (`RMDS_TEST_CLIENT_ADMIN_KEY`, `RMDS_TEST_MDS_A_KEY`; see
`.env.example`). After a new capture, put the new cluster's keys there
(`ceph auth get-key client.admin`, `ceph auth get-key mds.a`).

## Captures

Workload in every client capture (one step per second, `*.steps.txt` has the
timestamps): mount; `mkdir dir1`; `dd` 1 MiB into `dir1/file1` (`conv=fsync`);
`echo > dir1/file2`; `sync; echo 3 > /proc/sys/vm/drop_caches`; `md5sum file1`;
`cat file2`; `ls -l dir1`; `mv file2 file2.renamed`; `rm` both; `rmdir dir1`; umount.
File data goes to the OSDs and is **not** captured: the BPF filters keep only
monitor and MDS ports.

| Capture | Protocol | Filter | Content |
|---|---|---|---|
| `kernel-v1.pcap` (68 KB) | msgr1 | `tcp port 6789 or tcp port 7401` | kernel 7.0 client, mounted without `ms_mode`: full kernel<->mon session (auth, monmap, subscribe, fsmap.user, mdsmap, osdmap, statfs) and kernel<->MDS session (session open, 14 requests, caps, cap releases, metrics, session close). The C++ MDS was bound to v2 7400 + v1 7401; the kernel used v1 7401. |
| `fuse-v2.pcap` (55 KB) | msgr2 crc | `tcp port 7400` | ceph-fuse<->MDS only. msgr2 in crc mode: frames are plaintext but there is no decoder here (tshark and `msgr1_split.py` are msgr1-only). For the later msgr2 work. |
| `mds-v1-boot.pcap` (173 KB) | msgr1 | `tcp port 6789 or tcp port 6825` | the C++ MDS started with `ms_bind_msgr2=false` (v1 7401 only), from process start: MDS<->mon (3 cephx rounds, monmap, config, subscribe, MDS maps standby -> creating -> active, 7 beacons and acks, mgrmap, osdmap, 35 `MMonGetVersion`) and MDS<->mgr on the mgr's v1 port (MMgrOpen ~82 KB, MMgrConfigure, MMgrReport). This is the msgr1 MDS-to-monitor traffic a Rust MDS must produce. |
| `fuse-v1.pcap` (60 KB) | msgr1 | `tcp port 6789 or tcp port 7401` | ceph-fuse (`-m v1:127.0.0.1:6789`) <-> msgr1-only C++ MDS: full libcephfs<->MDS session. Only ceph-fuse's bootstrap mon connection (auth + monmap) is on msgr1: its real MonClient session then uses the v2 address from the monmap (clients that do not bind always pick the first address, see `AsyncMessenger::should_use_msgr2`), which this filter does not capture. |

Files per capture `<name>`:

- `<name>.pcap`: the capture (classic pcap, Ethernet link type, snaplen 0).
- `<name>.steps.txt`: workload steps with unix and UTC timestamps, filter, mount line.
- `<name>.summary.txt`, `<name>.tshark.txt`: `tshark -Y ceph` one line per packet and
  `tshark -Y ceph -O ceph` detail (msgr1 captures only).
- `<name>.mds-msgs.txt`: the C++ MDS's own log of every message it sent/received in the
  window (`debug_ms = 1`, one line each with the message's `print()` summary, e.g.
  `client_request(client.14202:2 mkdir #0x1/dir1 ...)`, `client_caps(grant ino ... caps=pAsxLsXsxFsxcrwb ...)`),
  without its OSD traffic. Easiest way to read what happened.
- `<name>.session.json`: `session ls` from the MDS admin socket right after the mount
  (client metadata, features, address type).
- `<name>.msgs/` (msgr1 captures): output of `msgr1_split.py`.

### `msgr1_split.py`

`python3 -I testdata/msgr1_split.py <pcap> <outdir>` reassembles each TCP connection
whose SYN is in the capture, parses the msgr1 handshake (banner, addresses,
`ceph_msg_connect`/`ceph_msg_connect_reply` rounds incl. `TAG_SEQ`) and every frame.
Output: `index.tsv` (every frame: time, connection, direction c2s/s2c, tag, seq, tid, type
number and name, header version/compat_version, front/middle/data sizes, source entity),
`connections.tsv` (features, entity type, rounds, flags), the authorizer bytes of each
connect round (`cN-c2s.authorizerK.bin`, `cN-s2c.authorizerK.bin`), and per message:

- `NNNN-cN-DIR-Type.front.bin` (+ `.middle.bin`, `.data.bin` when non-empty): the payloads
  exactly as on the wire; `front` is what `decode_payload()` reads.
- `NNNN-cN-DIR-Type.dencoder.bin`: the message in `encode_message()` layout (53-byte header,
  13-byte old footer, u32-length-prefixed front/middle/data):
  `ceph-dencoder type MClientRequest import F decode dump_json` decodes it.
- `NNNN-cN-DIR-Type.json`: that ceph-dencoder output. For messages it is only
  `{"summary": "<print() of the message>"}` (most message classes have no `dump()`), but a
  successful decode confirms the bytes are a valid message of that type and version.
  All captured message types decode except `MClientMetrics` (not a dencoder type).

Observations from the captures (useful for the Rust side):

- Features: kernel `0x2f018fb87aa4aafe`; C++ daemons and ceph-fuse `0x3f07fffffffdffff`
  (the kernel lacks e.g. `SERVER_REEF`/`SQUID`/`TENTACLE`, bits 31/49/50). It has
  `PGID64`, `MDSENC`, `MSG_ADDR2` and `SERVER_NAUTILUS`, so `MMDSMap::encode_payload` does
  not re-encode: kernel and MDS get the same MDSMap encoding (struct_v 5, compat 4).
- msgr1 handshakes: to the monitor 1 round (`authorizer_protocol` 0, cephx then runs in
  `MAuth`/`MAuthReply`); to the MDS/mgr 2 rounds: the first connect carries a cephx
  authorizer and is answered `TAG_CHALLENGE_AUTHORIZER`, the second carries the
  authorizer with the challenge response and is answered `TAG_SEQ` (then both sides
  exchange a u64 seq). Both rounds carry a 174-byte authorizer (`authorizer_protocol` 2 =
  cephx); the challenge reply is 32 bytes, the final reply 36. `connect.flags` is 0 from
  the kernel and ceph-fuse (to mon and MDS) and 1 (`CEPH_MSG_CONNECT_LOSSY`) from the MDS
  to mon and mgr; the monitor's `connect_reply.flags` is 1 (lossy) for everyone, the MDS's
  reply to clients is 0, the mgr's reply to the MDS is 1.
- Message footers: flags 0x05 (`COMPLETE | SIGNED`, cephx message signatures in `sig`) on
  every connection opened with a cephx authorizer (client<->MDS, MDS<->mgr), both
  directions; 0x01 (`COMPLETE`, unsigned) on all monitor connections.
- The MDS sends messages as `mds.-1` (`mds.18446744073709551615` in `index.tsv`) until it
  holds a rank, then as `mds.0` (the messenger name follows the rank).
- The C++ MDS answers `create` with an early `unsafe` reply followed by the `safe`
  reply (the Rust MDS only sends safe replies). The kernel also sends
  `client_session(request_flushmdlog)` after cap flushes, and `MClientMetrics` every second.

## maps/

From the same cluster after the captures (MDS `mds.a` active in `cpp-v1` mode, fs `a`
at FSMap epoch 25, fscid 3, pools 6/7 after `newfs.sh`):

| File | Source |
|---|---|
| `fs-dump.txt`, `fs-dump.json` | `ceph fs dump [--format json-pretty]` (FSMap, epoch 25) |
| `fs-get-a.json` | `ceph fs get a --format json-pretty` (MDSMap of fs a) |
| `fsmap.bin` | binary FSMap epoch 25 as stored by the monitor (`ceph-monstore-tool <copy of mon store> get mdsmap`); there is no `ceph mds getmap` in 20.2 |
| `mdsmap-to-mds.{0..3}.bin` | MDSMap blobs from the `MMDSMap` messages the monitor sent the msgr1-only MDS in `mds-v1-boot` (standby -> creating -> active) |
| `mdsmap-to-kernel.0.bin` | MDSMap blob the monitor sent the kernel in `kernel-v1` |
| `fsmapuser-to-kernel.0.bin` | FSMapUser from `MFSMapUser` sent to the kernel (`fsmap.user` subscription) |
| `monmap.bin`, `monmap.txt`, `monmap.json` | `ceph mon getmap -o`, `monmaptool --print`, ceph-dencoder `MonMap` dump |
| `monmap-to-kernel.{0,1}.bin/.json` | MonMap blobs from `MMonMap` sent to the kernel, with dencoder dumps |
| `mon-dump.json`, `osd-pool-ls-detail.json` | `ceph mon dump`, `ceph osd pool ls detail` |
| `mds-metadata.json` | `ceph mds metadata` |
| `auth-mds.rust-caps.txt` | caps of `mds.rust` (key removed) |
| `keyring-key-types.txt` | key type of every keyring: all `1 = CEPH_CRYPTO_AES`, 16-byte secrets |

**ceph-dencoder cannot decode `MDSMap` or `FSMap` in 20.2.0:** it segfaults in
`MDSMap::decode` (the release build logs through `g_ceph_context` after decoding the
`required_client_features` field, and that pointer is NULL in ceph-dencoder; the
`ceph tell mds.*` crash in the CLI is the same bug). Use the JSON dumps above as the
decoded reference for the binaries; `FSMapUser` is not a dencoder type.

## Limitations

- The Wireshark 4.6 Ceph dissector knows msgr1 only and is old: headers, tags, connect
  fields and old message fields decode; newer fields show as "unused bytes", some maps
  are "Malformed", and it names footer flag bit 0 "Lossy" (it is `COMPLETE`).
- tshark is confined by AppArmor and cannot read files under `/mnt/workspace`; copy a
  pcap to `/tmp` first (`capture.sh` does).
- No OSD traffic, no msgr2 decoding, and the captures hold no file data.
