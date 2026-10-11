# Handover: Rust CephFS MDS

> State on 2026-10-11, end of the first build session. Written for whoever continues the work. Read this first, then [`design/minimum-feature-set.md`](design/minimum-feature-set.md) (what we build and the decisions so far), [`design/workplan.md`](design/workplan.md) (how), [`design/store.md`](design/store.md) (the persistence design to implement next), and [`design/build-brief.md`](design/build-brief.md) (working conventions).

## 1. Where things stand

**Working end to end on the dev cluster:**

- `ceph-rmds -i rust` authenticates to the Ceph 20.2 monitor over msgr1 with cephx and gets the rotating `mds` secrets. It beacons and goes standby → `up:creating` → `up:active` as rank 0 of file system `a` in about a second. Beacons are acked, and on SIGTERM it sends a DNE beacon and exits.
- Its compat bit 63 (`rust mds format v0`) is merged into the file system's compat set (decision 2).
- A kernel 7.0 client mounting without `ms_mode` connects over msgr1. Its authorizer is verified (including the challenge round) and its messages are signed and checked. The session opens with our feature bitmap and closes cleanly with the `push_seq` check.
- The first request (GETATTR of `/`) is answered `-ENOSYS`, because **the metadata engine is not wired into the daemon yet**: `mount` fails with "error 38 = Function not implemented". That is the next step (section 5.1).

Reproduce (about 1 minute):

```sh
source devcluster/env.sh
cargo build -p rmds
./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring $RMDS_KEYRING &
ceph fs dump | grep mds.rust            # mds.rust{0:...} state up:active ... addr v1:127.0.0.1:...
devcluster/mount-kernel.sh /tmp/m v1    # mount error 38 (expected until the engine is wired)
kill %1; devcluster/newfs.sh            # stop; recreate fs "a" so the next MDS gets up:creating
```

**Built and tested, not yet connected to the daemon:** the metadata engine (`crates/engine`) and the OSD client (`crates/osdc`, which already does live RADOS I/O as `mds.rust`); see section 3.

## 2. Repository map

| Path | What |
|---|---|
| `README.md` | The architecture proposal (Igor Golikov), plus the repository contents table |
| `design/minimum-feature-set.md` | Minimum feature set: tiers P0/P1/P2/OUT, working assumptions D1-D18, per-area requirements, invariants, the **decision log (8.1)** and the day-one design hooks (5.1) |
| `design/minimum-feature-set.json`, `design/minimum-feature-set-catalog.md` | All 935 catalog entries classified, with reasons (generated; regenerate from the classification JSON, do not edit by hand) |
| `design/workplan.md` | Stages 0-7 with gates |
| `design/store.md` | Shard model and RADOS metadata store design (draft for review) |
| `design/build-brief.md` | Conventions and decisions the build followed |
| `mds-rust-map/` | Reference set, facts only: map of the C++ MDS and its peers, feature catalog. Start at `mds-rust-map/README.md` |
| `crates/` | The Rust workspace (section 3) |
| `devcluster/` | Scripts for the single-node Ceph 20.2 dev cluster (section 4) |
| `testdata/` | Real traffic between the C++ MDS and kernel/ceph-fuse clients and the monitor, maps, ceph-dencoder type list (`testdata/README.md`) |
| `scripts/smoke.sh` | End-to-end test the daemon must pass next: rank 0 active, kernel (msgr1) and ceph-fuse mounts, 8 MiB write/read-back after dropping caches, rename, hard link, readdir, unlink, rmdir, cross-client visibility |
| `tools/gen_features.py` | Generates `crates/denc/src/features.rs` from Ceph's `ceph_features.h` |
| `.env.example` | Variables of the git-ignored `.env`: the dev cluster's cephx keys at capture time, for the golden tests (section 4) |

## 3. Crates

Build and test everything: `cargo test --workspace` (328 tests pass, 2 ignored, on 2026-10-11; some golden tests need `/usr/bin/ceph-dencoder`, three need the capture keys in `.env` and skip without them, and the live examples need the dev cluster). With several people or agents working at once, give each its own `CARGO_TARGET_DIR` to avoid the build-directory lock.

| Crate | Status | Tests | Notes |
|---|---|---|---|
| `denc` | done | 8 | Encoding traits, framing, `EntityAddr` (legacy and ADDR2), `EntityName`, `UTime`, `Uuid`, feature constants, message envelope (`RawMessage`, `Message`) |
| `auth` | done | part of 54 | cephx: AES (type 1) and AES256KRB5 (type 2) keys, keyring, monitor handshake, service tickets, authorizers (build and verify, with the CEPHX_V2 challenge), rotating secrets, msgr1 message signatures |
| `msgr` | done for msgr1 | part of 54 | tokio messenger: msgr1 client and server, lossless server and lossy client policies, signatures, keepalive, reconnect. msgr2 not implemented; the protocol sits behind internal frame traits for it |
| `monc` | done | part of 54 | MonClient (hunting, auth, ticket renewal, subscriptions), MDSMap decoding, `MMDSBeacon`, `Beacon` task, compat set, `CephConf`; `examples/probe.rs` is the minimal daemon |
| `proto` | done | 80 | Every CephFS client message (sessions, requests, replies with InodeStat v9, caps v12/v14, reconnect v3/v5, snap, quota, lease, reclaim, metrics). Byte-identical decode and re-encode of all captured traffic; ceph-dencoder cross-checks; fuzzed |
| `engine` | done | 126 | In-memory namespace, POSIX semantics, caps model, file locks, sessions, journal records, replay; API at the crate root (3.1) |
| `osdc` | done | 56 | OSDMap, CRUSH, placement, MOSDOp/MOSDOpReply, Objecter on the daemon's messenger; live RADOS example (3.1) |
| `mds` (`ceph-rmds`) | cluster side, sessions; metadata stubbed | 4 | `config.rs` (ceph-mds compatible flags, ceph.conf), `daemon.rs` (membership, state machine, routing), `session.rs` (sessions, the per-session send funnel), `server.rs` (**stub**) |

### 3.1 Engine and OSD client

**engine** (done; 126 tests, clippy and rustfmt clean): `cargo test -p engine`.

An in-memory metadata engine for one shard, about 8k lines plus 3.3k lines of tests. It has no dependency on the wire crates. The daemon translates to and from its types; its API is documented at the crate root (`crates/engine/src/lib.rs`).

- **Entry points:**
  - `Engine::new(EngineConfig, J: Journal)` and `Engine::replay`;
  - sessions: `open_session`, `close_session(c, kill)`, `session_stale`, `session_resume`;
  - `handle_request(c, Request) -> Vec<Effect>`;
  - `handle_cap_update`, `handle_cap_release`;
  - `reconnect_cap` and `reconnect_done`;
  - `journal_committed`, `tick`, `recall_hints`, `set_pools`.
- **Effects:** `Reply`, `Caps`, `RealmSplit`, `TruncateData`, `PurgeInode`, `Recall`, `EvictClient`, `RecoverFileSize`. Send them in the returned order.
- **Covered:**
  - all P0 namespace operations with the C++ error order and their effects on times, change_attr, nlink, dirstat and exact rstat;
  - hard links, with promotion of a remaining link to primary;
  - unlink-while-open through strays;
  - inode-number range leases, with an API to delegate ranges to a session;
  - readdir in rjenkins hash order, checked against Ceph's own hashes, with paging;
  - xattrs and the `ceph.*` vxattrs (quota opens a realm and emits SPLIT; EXDEV across subvolumes);
  - fcntl and flock locks;
  - the caps model (two-step revoke and ack, flushes with FLUSH_ACK after commit, `max_size` journaled before it is granted, stale sessions and eviction);
  - one self-contained journal record per mutation, with replies only after commit;
  - retries answered from the completed-request table;
  - replay;
  - a randomized multi-client stress test that recomputes statistics and checks replay.
- **Open problems:**
  - dentry leases (always null), MClientQuota pushes, MDS auth-cap checks, snapshots, early replies and async dirops are not implemented (the hooks are in place);
  - journal records are Rust structs without an encoding yet;
  - ancestor rstat changes are journaled as full ancestor images, not the commutative deltas of `design/store.md`;
  - waiting for a commit is tracked per inode, so with an asynchronous journal, creates in one directory will serialize on commits;
  - creator caps differ slightly from C++ (no `max_size` in an MKNOD reply, no `Ax`/`Xx` on read-only creates or new directories), which costs an extra round trip;
  - the readdir byte budget is an estimate;
  - the engine has no clock: the daemon must fill a zero request stamp.

**osdc** (done; 56 tests, clippy clean): `cargo test -p osdc`. Live check: `source devcluster/env.sh; cargo run -p osdc --example rados_live` prints PASS. It authenticates as `mds.rust` over msgr1, follows the OSDMap, and runs these operations on `cephfs.a.meta`:
- write_full, read, stat;
- exclusive create (`-EEXIST`);
- xattrs, omap (paging, header, by keys, rm, clear);
- CMPXATTR (`-ECANCELED`) and ASSERT_VER (`-ERANGE`), the ownership-guard primitives of `design/store.md`;
- trimtrunc, append, zero, remove.

What it covers:
- **API:**
  - `OsdMap` (`decode`, `apply_incremental`, `pg_to_up_acting_osds`, `map_object`, `is_blocklisted`) and `CrushMap::do_rule`;
  - `MOsdMap`, `MOsdOp`, `MOsdOpReply`, `MOsdBackoff`, and the `ObjectOperation` builder;
  - `Objecter::start(transport, ObjecterConfig { client_inc, .. })`, with `operate`/`submit`, `write_full`/`read`/`stat`/`remove`, `handle_osd_map`, `set_epoch_barrier`, `wait_for_map`;
  - events: `RequestMap`, `CheckLatestMap`, `MapApplied`, `Blocklisted`, `Fenced`;
  - `MsgrTransport::new(messenger)` binds it to the daemon's single messenger, and `file_object_name`.
- **Golden checks** (fixtures in `crates/osdc/testdata`, each with its regeneration script):
  - CRUSH matches `crushtool --test` for every bucket algorithm, firstn/indep, MSR rules, five tunable profiles, device classes and choose_args;
  - 130 OSDMap epochs from two histories: applying incrementals reproduces every full map exactly;
  - placement matches `osdmaptool --test-map-pgs-dump-all` and `ceph osd map`;
  - 46 MOSDOp/MOSDOpReply messages captured from librados re-encode byte-identically, and ceph-dencoder accepts our v8 and v9 encodings.
- **Mock tests** (17) for the Objecter:
  - resend on primary change, PAUSERD/PAUSEWR, FULL ignored (FULL_FORCE on every op);
  - epoch barrier, pg split, `-EAGAIN`/redirect, `-EBLOCKLISTED`, backoff;
  - resend in tid order after a reset, laggy pings.
- **Bug found and fixed:** `reqid.inc` must carry the MDSMap incarnation (`client_inc`). Otherwise a restarted MDS could receive the stale results of the previous instance's requests.
- **Open problems:**
  - answering `CheckLatestMap` needs a monitor `get_version("osdmap")` call. Until it exists, ops to a pool that never existed wait for the next MOSDMap;
  - OSD connections use only the OSD's msgr1 address;
  - watch/notify, balanced reads and EC direct reads are not implemented;
  - helpers implemented locally that belong in `denc`: varints, `f32`, raw crc32c, the legacy-compat frame check, and `Ord` for `EntityAddr`.


### 3.2 Open problems reported per crate

- **auth/msgr/monc:**
  - msgr2 is missing;
  - ticket renewal and rotating-secret refresh are implemented but not yet observed live (needs a run of more than 45 minutes);
  - the MonClient hunts one monitor at a time;
  - no MMonCommand, MLog or MgrClient yet;
  - AES256KRB5 is untested against a live cluster that uses it.
- **proto:**
  - reconnect, lease, snap, quota and reclaim messages are checked only through ceph-dencoder, since no real traffic was captured;
  - we send InodeStat v9, caps v14 and SnapRealmInfoNew v2, where Ceph 20.2 sends v8, v12 and v1. Both 20.2 and `main` clients accept ours.

## 4. Dev environment

- **Machine:** Ubuntu 26.04, 32 cores, 125 GB RAM, Rust 1.99, passwordless sudo.
- **Ceph:** the 20.2.0 distro packages (`ceph-mon`, `ceph-mgr`, `ceph-osd`, `ceph-mds`, `ceph-fuse`, `ceph-common` with `ceph-dencoder`, 675 types), plus nfs-ganesha 6.5 and tcpdump/tshark.
- **Ceph `main` source:** `/home/ubuntu/ceph`, at the commit the map was written against. It is not built.
- **Cluster** (`devcluster/README.md` has the details):
  - 1 mon (v2 3300, v1 6789), 1 mgr, 3 memstore OSDs, pool size 3, cephx everywhere, data in `/mnt/workspace/devcluster-data`;
  - file system `a`, recreated with `devcluster/newfs.sh` and left with **no MDS**;
  - `source devcluster/env.sh` exports `CEPH_CONF`, `CEPH_KEYRING`, `RMDS_KEYRING` (entity `mds.rust`, with the caps cephadm gives an MDS) and the monitor addresses;
  - scripts: `start.sh [--mds none|cpp|cpp-v1]`, `stop.sh`, `reset.sh`, `status.sh`, `newfs.sh`, `mount-kernel.sh <dir> v1|v2`, `mount-fuse.sh <dir>`, `umount.sh <dir>`, `smoke-test.sh <mnt>` (I/O checks on an existing mount), `ganesha.sh start|stop` (NFSv4.1 on port 12049).
- **Quirks:**
  - Ceph 20.2.0's `MDSMap::decode` crashes in programs that never call `global_init`. That breaks `ceph tell mds.*`, Ganesha's FSAL_CEPH and ceph-dencoder's MDSMap/FSMap types. `devcluster/gctx-shim.c` (`$GCTX_SHIM`, `LD_PRELOAD`) works around it for the first two.
  - memstore OSDs need `osd_fast_shutdown = false` (already set) to keep data across a restart.
  - `HEALTH_ERR` with no MDS running is expected.
  - ceph-fuse always talks msgr2 to the monitor, but msgr1 to an MDS that only publishes v1.
  - tshark cannot read under `/mnt/workspace` (AppArmor).
  - `/etc/sudoers.d/ceph-smartctl` (from `ceph-base`) makes every sudo call print a harmless warning.
  - The distro's `nfs-ganesha.service` (port 2049), started when the package was installed, is still running. `devcluster/ganesha.sh` uses its own instance on port 12049.
- **Keys:** the throwaway cephx secrets of the dev cluster at capture time (client.admin, mds.a and the rotating mds secret) are in `.env` at the workspace root, which git ignores; `.env.example` lists the variables. Three golden tests use them to decrypt the captured cephx exchanges and skip, with a note, when `.env` is missing. Never commit keys; the cluster's own keyrings stay in `/mnt/workspace/devcluster-data/keyring/`, outside the repository.

### 4.1 Cluster operations

All scripts are in `devcluster/`; `devcluster/README.md` has every option, path and port. Commands below run from the repository root as the normal user (scripts call sudo themselves where needed).

**First-time setup on a new machine** (Ubuntu 26.04, Ceph 20.2 from the distro):

```sh
sudo apt-get install -y ceph-mon ceph-mgr ceph-osd ceph-mds ceph-fuse ceph-common ceph-base gcc
# optional: captures and NFS
echo 'wireshark-common wireshark-common/install-setuid boolean false' | sudo debconf-set-selections
sudo apt-get install -y tcpdump tshark nfs-ganesha nfs-ganesha-ceph
```

`gcc` builds the `gctx-shim` that works around the Ceph 20.2 `MDSMap::decode` crash (section 4); `start.sh` warns if it cannot.

**Bring-up:**

```sh
devcluster/start.sh --mds none   # creates the cluster on first use (new fsid and keys), then starts
                                  # 1 mon, 1 mgr, 3 OSDs; idempotent; no MDS so mds.rust can take rank 0
source devcluster/env.sh          # CEPH_CONF, CEPH_KEYRING, RMDS_KEYRING, MON_V1/MON_V2, FSID, ...
devcluster/newfs.sh               # only if an MDS held rank 0 before: fresh fs "a", next MDS gets up:creating
```

`start.sh --mds cpp` brings up the C++ reference MDS `mds.a` instead (msgr2 and msgr1). `--mds cpp-v1` binds it to msgr1 only.

**Run the Rust MDS:**

```sh
cargo build -p rmds
RMDS_LOG=info ./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring $RMDS_KEYRING \
    > /mnt/workspace/devcluster-data/log/ceph-rmds.log 2>&1 &
ceph fs dump | grep mds.rust      # expect: mds.rust{0:<gid>} state up:active ... addr v1:127.0.0.1:<port>
devcluster/mount-kernel.sh /mnt/k v1   # kernel over msgr1 (v2 = ms_mode=prefer-crc, needs msgr2 in the MDS)
devcluster/mount-fuse.sh /mnt/f
```

**Status queries:**

| Question | Command |
|---|---|
| Everything at once (daemons, health, fs, mounts) | `devcluster/status.sh` |
| Cluster health | `ceph -s`; `ceph health detail` (`HEALTH_ERR 1 filesystem is offline` is expected while no MDS runs) |
| Who holds rank 0, in which state, at which address; fs compat set | `ceph fs dump` (look for `mds.rust{0:...} state up:active` and `63=rust mds format v0`) |
| Is our key valid | `ceph -n mds.rust -k $RMDS_KEYRING mds stat` |
| Why a beacon or an authentication was refused | `grep -E 'beacon|mds.rust|cephx' $CEPH_LOG_DIR/ceph-mon.a.log` (the mon logs at `debug_mon 10`, `debug_auth 10`) |
| What the Rust MDS is doing | its log (above); `RMDS_LOG=debug` for more; it has no admin socket yet |
| Clients and mounts | `mount | grep -E 'ceph|nfs'`; `sudo dmesg | grep -E 'ceph|libceph'` for kernel-client errors |
| OSDs and pools | `ceph osd tree`, `ceph osd lspools`, `rados -p cephfs.a.data ls` |

**Teardown:**

```sh
devcluster/umount.sh /mnt/k; devcluster/umount.sh /mnt/f   # escalates to umount -f / -l if the MDS is gone
kill %1   # or: pkill -TERM -x ceph-rmds; SIGTERM sends a down:dne beacon, so the rank fails over at once
devcluster/stop.sh    # stops MDS, OSDs, mgr, mon; keeps all data; start.sh resumes it
devcluster/reset.sh   # stop and delete /mnt/workspace/devcluster-data: the next start.sh makes a new
                      # cluster with a new fsid and NEW KEYS (the capture keys in .env stay valid for testdata/)
```

Between runs of the Rust MDS, use `devcluster/newfs.sh`. After any MDS has held rank 0, the next one gets `up:replay`, and `ceph-rmds` (no persistence yet) then serves an empty namespace. OSDs use memstore: a killed or crashed OSD loses its data, so stop the cluster only with `stop.sh`, or else run `reset.sh` and `start.sh`.

## 5. Next steps, in order

### 5.1 Wire the engine into the daemon (gate: `scripts/smoke.sh` passes)

Replace the stub `crates/mds/src/server.rs`. Its module comment describes the full version, and the daemon already calls its interface.

1. Hold an `engine::Engine` (created on `up:creating`, with the data pool from the MDSMap; `Engine::set_pools` on map changes).
2. Translate `proto::MClientRequest` into `engine::Request`:
   - op and arguments;
   - `FilePath`s;
   - caller and owner ids, and `stamp`;
   - flags, retry attempt and `oldest_client_tid`;
   - the embedded releases;
   - initial xattrs from the data payload.

   Likewise translate `MClientCaps` into `engine::CapUpdate`, and `MClientCapRelease` into release items.
3. Translate `engine::Effect` back into wire messages, in the returned order, through `Session::send`:
   - `Reply` → `proto::MClientReply` (engine `InodeStat`, `DirFragInfo` and `LeaseStat` → proto types; snap trace in `SnapFormat::for_client(&session.features)`; readdir, create, filelock and getvxattr extras);
   - `Caps` → `MClientCaps`;
   - `RealmSplit` → `MClientSnap`;
   - `Recall` → `RECALL_STATE`.
4. Before sending a reply, check `MClientReply::check_invariants_for(&req)` and log violations (design section 7 lists the client crash triggers).
5. Sessions: `Engine::open_session` / `close_session` from `session_opened` / `session_closed`. Add renewal staleness and autoclose timers (`Engine::session_stale`/`session_resume`; `session_timeout` 60 s and `session_autoclose` 300 s from the MDSMap).
6. `cap_auths_for`: parse `Connection::peer_caps()` (the MDS cap string) into `MdsCapAuth`. An empty list is acceptable while testing as `client.admin`.
7. Deny `MClientReconnect` with `CLOSE` until persistence exists. Answer `MClientReclaim` with result 0 and no address (Ganesha treats that as ENOENT).
8. Run `scripts/smoke.sh`, then pjdfstest on a kernel mount.

### 5.2 OSD client in the daemon

Bind `osdc` to the daemon's messenger with `MsgrTransport::new(messenger)`, so there is one identity for fencing, and feed it the OSDMap (`monc.subscribe("osdmap")`). `crates/osdc/examples/rados_live.rs` shows the wiring. Set `ObjecterConfig::client_inc` from the rank's incarnation in the MDSMap. Then implement the engine effects that need RADOS:
- `PurgeInode` deletes data objects up to the largest size a client may have written;
- `TruncateData`;
- `RecoverFileSize` probes data objects;
- `EvictClient` blocklists the client through the monitor.

### 5.3 Persistence

Implement `design/store.md`:
- format marker;
- shard map with ownership epochs, and `cmpxattr` guards on every WAL append;
- per-shard WAL streams with group commit;
- inode table and directory objects;
- inode-number leases;
- replay.

The engine already emits one self-contained `JournalRecord` per transaction through its `Journal` trait, and has `Engine::replay`. Then add failover: replay, reconnect and rejoin. Gate: `kill -9` under load with no acknowledged metadata lost (workplan stage 3).

### 5.4 After that

Follow `design/workplan.md`:
- stage 4: Ganesha (reclaim, `timeout` sessions, LOOKUPINO) and subvolumes as shards;
- stage 5: msgr2;
- stage 6: multiple ranks;
- stage 7: operability.

Open decisions are in `design/minimum-feature-set.md` section 8, items 8-12, including the project license.

## 6. Rules that matter

- **License:** the kernel's `net/ceph` and `fs/ceph` are GPL-2.0. Read them to understand client behaviour, but never translate their code. The C++ Ceph sources (LGPL) and `mds-rust-map/` are the code references.
- **Decisions are logged:** `design/minimum-feature-set.md` 8.1. Add new ones there.
- **Day-one hooks** (design 5.1) must stay intact; they are what keeps later features from requiring a rewrite:
  - connection reading is separate from execution;
  - every client message goes through the per-session funnel;
  - WAL records are self-contained and versioned;
  - the clientreplay phase exists;
  - the allocator can delegate inode ranges;
  - shard ownership epochs are checked on every WAL append.
- **Client invariants** (design section 7): a violation crashes or hangs unmodified clients. Examples: a forward for a tid that already got a reply, a wrong dentry-trace parent, directory `nlink` other than 1, trailing bytes in replies, a health metric with id 0.
- **Golden data first:** check encodings against `testdata/` captures and `ceph-dencoder` before trying them on a live client.
