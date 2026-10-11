# devcluster: single-node Ceph 20.2 cluster for rust-mds

A throwaway Ceph cluster on this machine (1 mon, 1 mgr, 3 OSDs, optional C++ MDS)
to run the Rust MDS against. Daemons run as the normal user (no systemd, no
containers), each in the foreground of its own `setsid nohup` session.

```sh
source /mnt/workspace/rust-mds/devcluster/env.sh   # CEPH_CONF, keyrings, MON_V1, FSID, ...
/mnt/workspace/rust-mds/devcluster/status.sh       # daemons, ceph -s, ceph fs dump, mounts
ceph -s                                            # any ceph/rados command works once env.sh is sourced
```

**State left for the team (2026-10-11 19:21):** the cluster runs with `--mds none` (mon,
mgr, 3 OSDs; no MDS, no Ganesha, nothing mounted) on a file system `a` freshly
recreated by `newfs.sh` that has never had an MDS: `in` and `failed` are empty, so the
first MDS that beacons (the Rust MDS as `mds.rust`) is told **up:creating**. `ceph -s`
shows `HEALTH_ERR 1 filesystem is offline` until an MDS is active; that is expected.
Reference captures and maps are in `../testdata/` (see its README).

Versions: Ceph `20.2.0-0ubuntu2.1` (tentacle) from Ubuntu 26.04, kernel `7.0.0-1012-ibm`.

## Scripts

| Script | What it does |
|---|---|
| `start.sh [--mds none\|cpp\|cpp-v1]` | Creates the cluster if `/mnt/workspace/devcluster-data` holds none, then starts mon, mgr, 3 OSDs (idempotent). `--mds none` (default): no MDS; stops the C++ MDS if it runs. `--mds cpp`: also start the C++ MDS `mds.a` (msgr2 + msgr1) and wait until it is `up:active`. `--mds cpp-v1`: the C++ MDS bound to msgr1 only (`ms_bind_msgr2=false`), so it also talks msgr1 to the monitor. |
| `stop.sh` | Unmounts CephFS mounts on this machine, stops MDS, OSDs, mgr, mon. Keeps data. A running `ceph-rmds` is reported, not stopped. |
| `reset.sh` | `stop.sh`, then deletes `/mnt/workspace/devcluster-data`. The next `start.sh` makes a new cluster (new fsid, new keys). |
| `status.sh` | Daemon pids, `ceph -s`, `ceph fs dump`, CephFS mounts. |
| `env.sh` | Source it. Exports `CEPH_CONF`, `CEPH_KEYRING` (client.admin), `CEPH_ADMIN_SECRET` (bare key), `RMDS_KEYRING` + `RMDS_NAME=mds.rust`, `MON_V1=127.0.0.1:6789`, `MON_V2=127.0.0.1:3300`, `FSID`, `CEPH_FS_NAME=a`, `CEPH_LOG_DIR`, `DEVCLUSTER`, `DEVCLUSTER_DATA`, `GCTX_SHIM` (see limitations). |
| `newfs.sh` | Deletes and recreates file system `a` with new empty pools, so the next MDS gets `up:creating` again (see "MDS ranks" below). Fails any MDS holding the rank. Refuses while CephFS is mounted. |
| `mount-kernel.sh <mnt> [v1\|v2] [extra,opts]` | Kernel mount as client.admin (sudo; runs `sudo modprobe ceph` first). `v1` (default): `mount -t ceph 127.0.0.1:6789:/ <mnt> -o name=admin,secretfile=...,mds_namespace=a,conf=...`; no `ms_mode`, so the kernel uses msgr1 to the mon and to the MDS's v1 address. `v2`: `mount -t ceph admin@<fsid>.a=/ <mnt> -o mon_addr=127.0.0.1:3300,ms_mode=prefer-crc,...`. |
| `mount-fuse.sh <mnt> [ceph-fuse args]` | `sudo ceph-fuse -n client.admin --client_fs=a <mnt>`; log `log/ceph-fuse.<mnt>.log`, admin socket `run/ceph-fuse.<mnt>.asok`. To keep ceph-fuse on msgr1 to the monitor too: `mount-fuse.sh <mnt> -m v1:127.0.0.1:6789`. |
| `umount.sh <mnt>` | Unmounts a kernel, FUSE or NFS mount; escalates to `umount -f`, then `umount -l`, if the server does not answer. |
| `smoke-test.sh <mnt> [MiB]` | Runs, as root: mkdir, create+write with `dd` (4 MiB default), read back with `md5sum`/`cat`, checks the first data object is in `cephfs.a.data`, `ls -l`, `stat`, `mv` (rename across dirs and in place), `rm`, `rmdir`. Prints PASS/FAIL per step; exit 0 if all pass. Works on kernel, ceph-fuse and NFSv4 mounts. |
| `ganesha.sh start\|stop\|status` | NFS-Ganesha with an FSAL_CEPH export of `a` (see below). |
| `common.sh` | shared definitions (sourced by the scripts); `gctx-shim.c`: see limitations |

Typical loops:

```sh
D=/mnt/workspace/rust-mds/devcluster
$D/start.sh --mds cpp && $D/mount-kernel.sh /mnt/k v1 && $D/smoke-test.sh /mnt/k && $D/umount.sh /mnt/k
$D/start.sh --mds none && $D/newfs.sh      # back to "no MDS, fresh fs" for the Rust MDS
```

## Paths

Everything lives in `/mnt/workspace/devcluster-data` (outside the repo):

| Path | Content |
|---|---|
| `ceph.conf` | the cluster config (`$CEPH_CONF`); written once at creation |
| `fsid` | cluster fsid (also `$FSID`) |
| `keyring/ceph.client.admin.keyring` | client.admin, caps `allow *` everywhere |
| `keyring/client.admin.secret` | client.admin key only, for `mount -o secretfile=` |
| `keyring/ceph.mds.rust.keyring` | **mds.rust**: `mon 'allow profile mds' osd 'allow rw tag cephfs *=*' mds 'allow' mgr 'allow profile mds'` |
| `keyring/ceph.mds.a.keyring` | mds.a (C++ MDS), same caps |
| `keyring/ceph.mgr.x.keyring`, `osd/N/keyring`, `mon/a/keyring` | daemon keys |
| `log/ceph-<type>.<id>.log` | daemon logs; `log/<type>.<id>.stdout` stdout/stderr; `log/ceph.log` cluster log |
| `run/ceph-<type>.<id>.asok` | admin sockets; `run/<type>.<id>.pid` pids |
| `mon/a`, `osd/N`, `mgr/x`, `mds/*` | daemon data |

All keys are cephx AES keys (`CEPH_CRYPTO_AES` = type 1, 16-byte secret; Ceph
20.2 has no AES256KRB5 keys). The fscid and pool ids of `a` change with every `newfs.sh`
(now: fscid 4, `cephfs.a.meta` id 8, `cephfs.a.data` id 9; `.mgr` is id 3); read them
from `ceph fs dump`.

## Ports and protocols

| Who | Address |
|---|---|
| mon.a | `v2:127.0.0.1:3300`, `v1:127.0.0.1:6789` (`mon_host = [v2:127.0.0.1:3300,v1:127.0.0.1:6789]`) |
| C++ MDS mds.a | `v2:127.0.0.1:7400`, `v1:127.0.0.1:7401` (`--mds cpp-v1`: only `v1:127.0.0.1:7401`) |
| OSDs, mgr | random ports in 6800-7300 (`ms_bind_port_min/max`), both v2 and v1 |
| free for the Rust MDS | 7402-7499 (e.g. `v1:127.0.0.1:7410`), or any free port outside 6800-7300 |
| NFS-Ganesha (`ganesha.sh`) | NFSv4.1/4.2 on `127.0.0.1:12049` (the packaged system `nfs-ganesha.service`, started by the package install, keeps 2049 and exports nothing of ours) |

`ms_bind_msgr1 = true` and `ms_bind_msgr2 = true` on every daemon. Clients
(ceph-fuse, CLI) try the first address of a peer's vector, i.e. v2; a daemon that
bound only v1 (the Rust MDS, `cpp-v1`) connects with v1 (see
`AsyncMessenger::should_use_msgr2`). The kernel uses v1 unless mounted with `ms_mode=`.

## Configuration choices

- cephx required for cluster, service and client. Insecure global_id reclaim
  stays allowed (default); its health warnings are muted.
- OSDs use **memstore** (`osd_objectstore = memstore`, 4 GiB "device" each, data in RAM;
  `osd_fast_shutdown = false` so a clean stop writes it to `osd/N/`).
- Pools: size 3, min_size 2, CRUSH failure domain `osd` (`osd_crush_chooseleaf_type = 0`,
  one host), pg_num 8 for the CephFS pools, autoscaler off. Pool deletion allowed.
- Debug levels: mon `debug_mon 10, debug_auth 10, debug_ms 1` (shows why a beacon or
  an authentication is accepted or rejected); C++ MDS `debug_mds 10, debug_ms 1`.
  Change at runtime with `ceph tell mon.a config set debug_mon 20` (the conf file
  wins over `ceph config set`, `ceph tell` wins over the file).

## MDS ranks: up:creating vs up:replay

- On a file system that never had an MDS (`in` empty), the monitor assigns rank 0
  to the first standby with state **up:creating**; the MDS then asks for `up:active`.
- Once any MDS has held rank 0 and stops (or misses beacons for `mds_beacon_grace`
  = 15 s), rank 0 is **failed**, and the next MDS gets **up:replay** (it must replay
  the previous MDS's journal). A C++ MDS cannot replay a Rust MDS's (absent) journal
  and vice versa, so run `newfs.sh` when switching between them.
- `start.sh` prints which case applies (`rank 0: none failed` means replay).
- The C++ MDS and a Rust MDS can run together; one is active, the other standby.

## Running the Rust MDS against it

Facts the daemon needs: entity `mds.rust`, keyring `$RMDS_KEYRING`, monitor
`$MON_V1` (msgr1) / `$MON_V2`, fsid `$FSID`, file system `a` (fscid from `ceph fs dump`).
Check that the key works: `ceph -n mds.rust -k $RMDS_KEYRING mds stat`. Watch
`ceph fs dump` / `ceph -w` and `$CEPH_LOG_DIR/ceph-mon.a.log` (grep `beacon`, `mds.rust`,
`cephx`) while it boots. Then `mount-kernel.sh /mnt/x v1` + `smoke-test.sh /mnt/x`
(or `mount-fuse.sh`) exercise it like the C++ MDS below. Between runs, `newfs.sh` gives
the next run `up:creating` again.

## Validation with the C++ MDS (2026-10-11)

`start.sh --mds cpp` → `HEALTH_OK`, `mds.a` `up:active` (creating → active in ~2 s). Then
`mount-*.sh` + `smoke-test.sh` + `umount.sh`, all steps PASS:

| Client | Protocol to MDS | mkdir / dd write 4 MiB / md5sum read / ls -l / mv / rm / rmdir / umount |
|---|---|---|
| kernel 7.0, `v1` | msgr1 (session inst `v1:127.0.0.1:0/…`) | all pass |
| kernel 7.0, `v2` (`ms_mode=prefer-crc`) | msgr2 crc | all pass |
| ceph-fuse 20.2 | msgr2 (first address) | all pass |
| ceph-fuse 20.2 with `--mds cpp-v1` | msgr1 | mount, mkdir, write, read, ls, mv, rm, rmdir, umount work (the `fuse-v1` capture workload, see `testdata/`) |
| NFS-Ganesha 6.5 FSAL_CEPH, kernel NFSv4.1 client | (Ganesha uses libcephfs, msgr2) | all pass (needs the `gctx-shim`, see below) |

## NFS-Ganesha

`ganesha.sh start` runs `ganesha.nfsd` (package `nfs-ganesha` 6.5 + `nfs-ganesha-ceph`)
as the normal user in the foreground of a `setsid nohup` session, config
`$DEVCLUSTER_DATA/ganesha/ganesha.conf`: NFSv4 only (minor versions 1, 2), port 12049
on 127.0.0.1, no NLM/RQUOTA, graceless, `fs` recovery backend under
`$DEVCLUSTER_DATA/ganesha/recov`, one export `Pseudo = /a` (`Path = /`, RW,
no root squash) with `FSAL { Name = CEPH; User_Id = admin; Filesystem = a; }`, log
`$CEPH_LOG_DIR/ganesha.log`. It needs an active MDS (`start.sh --mds cpp`).

```sh
devcluster/ganesha.sh start
sudo mkdir -p /mnt/nfs && sudo mount -t nfs -o vers=4.1,port=12049 127.0.0.1:/a /mnt/nfs
devcluster/smoke-test.sh /mnt/nfs && devcluster/umount.sh /mnt/nfs
devcluster/ganesha.sh stop
```

Two workarounds are built in: `allow_set_io_flusher_fail = true` (a non-root Ganesha
may not set `PR_SET_IO_FLUSHER`), and `LD_PRELOAD=$GCTX_SHIM` (without it Ganesha
segfaults in `MDSMap::decode` as soon as its libcephfs client receives the MDS map; see
below).

## Known limitations and surprises

- **Ceph 20.2.0 bug: `MDSMap::decode` segfaults outside daemons.** It checks a log level
  through the global `ceph::global::g_ceph_context`, which is NULL in processes that
  never call `global_init()`. Affected: `ceph tell mds.<x> ...` (Python CLI via
  libcephfs), NFS-Ganesha FSAL_CEPH, `ceph-dencoder type MDSMap|FSMap ...`. Not affected:
  the daemons, ceph-fuse, the kernel, `ceph fs dump`. Workaround for libcephfs users:
  `LD_PRELOAD=$GCTX_SHIM` (`devcluster/gctx-shim.c`, built by `start.sh` into
  `$DEVCLUSTER_DATA/lib/`), e.g. `LD_PRELOAD=$GCTX_SHIM ceph tell mds.a session ls`, or
  use the admin socket: `ceph --admin-daemon $DEVCLUSTER_DATA/run/ceph-mds.a.asok session ls`.
  No workaround for ceph-dencoder's MDSMap/FSMap types.
- **sudo noise:** sudo-rs prints "wildcards are not allowed in command arguments" for
  `/etc/sudoers.d/ceph-smartctl` (installed by `ceph-base` for the system `ceph` user)
  on every sudo call. Harmless; the file is not needed here and could be removed by the
  machine owner.
- **memstore:** OSD data lives in RAM and is written to `osd/N/` only when the OSD
  stops cleanly (`stop.sh` sends SIGTERM and waits). If an OSD is killed or crashes,
  its data is gone and the cluster is unusable: run `reset.sh` and `start.sh` again.
- The CephFS root is owned by root, mode 0755: write as root (sudo) or `chown` a
  directory first. Mounts need sudo.
- A kernel mount whose MDS died hangs on I/O; `umount.sh` falls back to `umount -f`/`-l`.
- The mgr logs Python errors for optional modules it cannot load (Python 3.14,
  e.g. `alerts`, `test_orchestrator`); the always-on modules work and health is OK.
- One cluster per machine: fixed data path and ports.
