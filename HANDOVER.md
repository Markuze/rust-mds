# Handover: Rust CephFS MDS

**PAUSED at the user's request.** All current agent assignments finished. Read the portable checkpoint at [`current/sol-handover.md`](current/sol-handover.md) first. Do not start implementation, tests, G2 retry, G3 or live operations until explicit user resume. Only handover preparation/independent review and the previously requested commit/push checkpoint remain authorized. Commands and historical procedures below are reference material, not authorization to execute them.

> Updated on 2026-10-22 with accepted modified-tree QA/native evidence and the first platform publication. Historical isolated `f54d298` evidence stays separate. The documentation owner ran no tests, live actions or commits. Read [`design/persistence-contract.md`](design/persistence-contract.md), [`design/workplan.md`](design/workplan.md), [`design/store.md`](design/store.md), then the requirements in [`design/minimum-feature-set.md`](design/minimum-feature-set.md). The build brief's original ownership table is historical.

## 1. Where things stand

**Accepted modified-tree QA:** 524 tests passed, 0 failed, 2 ignored including doctests; build, fmt check and default workspace clippy passed. Strict clippy exited 101 on the recorded generated-denc and engine baseline diagnostics. Independent retained-evidence review verified source/binary hashes and recounted the results; it did not rerun tests/live operations. **G0 remains partial; no full G1-G3 completion is declared. Complete G2 application comparison is blocked, not passed.**

Historical isolated prototype baseline: unmodified `f54d298` had 347 passed, 0 failed, 2 ignored and passing build/fmt/native RADOS/kernel+FUSE smoke. Its counts and MemJournal behavior are not the new integrated results.

The amended persistence contract remains independently ACCEPTED and unchanged. Engine admission/failure/visibility and recovery-state packages are independently ACCEPTED. Final state review ran 42 focused tests; the author separately reports 175 engine tests plus one doctest. Store WAL/checkpoint core (including corrected OMAP pagination) and authority/recovery-state/schema packages are ACCEPTED; core re-review independently passed 50 tests and store-only strict clippy. These are package-revision counts; combined QA is recorded separately above.

Daemon G2 persistence integration is independently ACCEPTED (71 tests and default daemon-only clippy at that review revision). Runtime corrections G2-R1 through G2-R4 are independently ACCEPTED (11 focused tests, 82 full daemon tests and production build). The author separately reports current default crate clippy and owned formatting checks pass with existing dead-code warnings. QA's complete-G2 application harness and partial-G3 foundation are ACCEPTED after both namespace-oracle fixes; final independent review ran 11 selected tests in each Python mode, while the author ran the full 67-test tooling suite. These are scoped offline checks, not native durability qualification.

**Current production path:** `Server<DaemonJournal>` owns `Engine<DaemonJournal>`. It submits one admitted record through `EngineInput` / `RecordPermit` / `EngineStep`. A native store worker writes guarded stable WAL records, delivers explicit tagged completions, and publishes immutable materialized checkpoints before bounded trim. Durable rank Open precedes OPEN; Closing precedes terminal engine cleanup; Closed follows its committed receipt. Startup discovers filesystem identity through FSMap, fences persisted predecessors, acquires separate shard/session authorities, and uses checked restore/replay. Daemon `MemJournal` compatibility entry points are test-only. Missing/foreign/corrupt recovery state fails closed.

QA completed one separately authorized fresh Rust setup: **filesystem44 / metadata88 / data89**. Native startup reached Ready and active. The trial recorded 17 acknowledged operations and 10 inode identities, cleanly closed/unmounted, and restarted through replay -> reconnect -> rejoin -> active with zero restored Open sessions. The cold mount succeeded, but Linux recycled legacy mountinfo ID323. The controller exited 1 before any post-restart state comparison or 16-future-inode nonreuse checks. No G2 PASS/result exists. The boot UUID + kernel 64-bit UNIQ-ID correction is now independently **ACCEPTED**: 83 author tests in each Python mode; 21 independent tests normally/optimized. No corrected live retry ran. Wait for explicit user resume and a sole QA assignment before retry on the **same fs44/pools88/89**, never another fresh setup.

Separate durable kernel/FUSE smoke then passed on that same filesystem without reset/newfs. Native final positions were tail83, checkpoint generation7/coverage82, trim82. This establishes scoped bootstrap/clientless startup/checkpoint/trim/normal-I/O observations, not the missing acknowledged-history comparison, storage fault matrix or G3 takeover. Final QA audit found no MDS/test mounts, original OSDs alive and 17 PGs active+clean. Nonempty Open-session gathers remain held for G3.

Integrated evidence: `/tmp/opencode/sol-integrated-g2-20261022-2014/`. Reviewed binary SHA256: `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`. All 640 Cargo inputs matched QA/release snapshots; source/manifests/scripts stayed unchanged during execution, apart from status documents. The following unique-mount tooling correction is separate. Contracts/Cargo code remain unchanged in this documentation update.

The release owner normally committed/pushed **`a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`**, `platform: add checked fencing and native pool enumeration` (13 paths), **`ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93`**, `mds: persist root-shard state with guarded WAL and checkpoints` (53 paths), and **`805a8b0f1c8388548f0c8adf869aa28c0e70b926`**, `qa: add guarded restart verification and CI lanes` (23 paths). Actual remote main matched local/tracking QA HEAD, ahead/behind 0/0. Baseline f54d298 remains intact/published and all 640 committed/working Cargo inputs retain accepted hashes. The final 27 status/handovers/task-review ledger paths form the documentation checkpoint at branch HEAD, subject `docs: record paused durability checkpoint and handover`. This text freezes before that commit; the final release report records its actual SHA, normal-push result, remote equality and clean-tree confirmation. No unobserved final publication is claimed here. See `current/sol-handover.md` section 7 and `current/sol-release.md`. This remains a paused checkpoint with incomplete G2/G3; publication does not resume implementation/live work.

QA's native platform probe passed on existing data pool 87; independent review accepted retained evidence at `/tmp/opencode/sol-platform-live-20261022.YcfShl/`. It verified get_version/checked map wait at epoch 402, successful and rejected CMPXATTR/ASSERT_VER compounds (-125/-34) with unchanged data/tail/version after rejection, and a complete all-namespace scan: 8 pages, 6 empty continuations, all eight PGs across three primaries, both sentinels found and independently confirmed absent after cleanup. Native sources/manifests stayed unchanged; only an unrelated store session file changed. Filesystem 43/pools 86/87 stayed unchanged, with 17 PGs active+clean. This completes scoped platform native validation, not bootstrap, format hygiene, durability or any G1-G3 gate. Live map-change and nonempty-locator coverage remain untested.

QA restored packaged C++ MDS startup with an explicit MDS keyring. C++ kernel and Ganesha/NFSv4.1 smoke passed. The graceless Ganesha helper does not qualify recovery/reclaim. The canonical evidence table in `progress_ledger.md` records exact commands, artifacts, peer versions and limits. Authoritative Rust baseline logs are `/tmp/opencode/sol-g0/f54d298-*`; old `immutable-*` and shared-target evidence are invalid. Independent review checked archived source identity and counts, but did not rerun live checks.

- `ceph-rmds -i rust` authenticates to the Ceph 20.2 monitor over msgr1 with cephx and gets the rotating `mds` secrets. It beacons and goes standby → `up:creating` → `up:active` as rank 0 of file system `a` in about a second.
- A kernel 7.0 client mounting without `ms_mode` and ceph-fuse both do full I/O: 8 MiB write/read-back after `drop_caches`, rename, hard link, readdir, unlink, rmdir, cross-client visibility.
- Getting there needed three kernel-visible correctness fixes (decision 2026-10-13 in `design/minimum-feature-set.md` 8.1): mutation replies must drop `Fs Fx` from the parent directory's cap (the creator keeps them), traced replies must not re-grant a cap's liked set once issued, and OPEN replies must trace the final dentry. Without them the kernel answers lookups from stale negative dentries after cache eviction.
- Progress is tracked in [`progress_ledger.md`](progress_ledger.md); keep it current.

Historical fresh-filesystem prototype procedure (QA only; not a recovery test):

```sh
source devcluster/env.sh
cargo build -p rmds
./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring $RMDS_KEYRING &
ceph fs dump | grep mds.rust            # mds.rust{0:...} state up:active ... addr v1:127.0.0.1:...
devcluster/mount-kernel.sh /tmp/m v1    # historical prototype supports metadata I/O
kill %1; devcluster/newfs.sh            # stop; recreate fs "a" so the next MDS gets up:creating
```

**Historical base f54d298 only:** the daemon used `Engine<MemJournal>`, store provided codecs only, and replay created an empty namespace. The historical ledger also records 8,769 pjdfstest passes, 28 TODO failures and one Y2038 client-wire case. These statements do not describe the new production persistence implementation or establish its qualification.

## 2. Repository map

| Path | What |
|---|---|
| `README.md` | The architecture proposal (Igor Golikov), plus the repository contents table |
| `design/minimum-feature-set.md` | Minimum feature set: tiers P0/P1/P2/OUT, working assumptions D1-D18, per-area requirements, invariants, the **decision log (8.1)** and the day-one design hooks (5.1) |
| `design/minimum-feature-set.json`, `design/minimum-feature-set-catalog.md` | All 935 catalog entries classified, with reasons (generated; regenerate from the classification JSON, do not edit by hand) |
| `design/workplan.md` | Current G0-G7 gates; first cycle ends at G3 |
| `design/store.md` | Selected stable guarded WAL and immutable checkpoint design |
| `design/persistence-contract.md` | First-milestone engine/store/daemon interfaces and failure rules |
| `design/build-brief.md` | Conventions and decisions the build followed |
| `mds-rust-map/` | Reference set, facts only: map of the C++ MDS and its peers, feature catalog. Start at `mds-rust-map/README.md` |
| `crates/` | The Rust workspace (section 3) |
| `devcluster/` | Scripts for the single-node Ceph 20.2 dev cluster (section 4) |
| `testdata/` | Real traffic between the C++ MDS and kernel/ceph-fuse clients and the monitor, maps, ceph-dencoder type list (`testdata/README.md`) |
| `scripts/smoke.sh` | Historical passing prototype gate; QA must rerun: rank 0 active, kernel/FUSE mounts and I/O |
| `tools/gen_features.py` | Generates `crates/denc/src/features.rs` from Ceph's `ceph_features.h` |
| `.env.example` | Variables of the git-ignored `.env`: the dev cluster's cephx keys at capture time, for the golden tests (section 4) |

## 3. Crates

Build and test everything: `cargo test --workspace`. Historical counts below are marked; current package evidence belongs in `progress_ledger.md`, not a combined-tree pass claim. Some golden tests need `/usr/bin/ceph-dencoder`, three need capture keys in `.env`, and live examples need the dev cluster. Use a separate `CARGO_TARGET_DIR` per agent. QA records actual commands, skips and results.

| Crate | Status | Tests | Notes |
|---|---|---|---|
| `denc` | done | 8 | Encoding traits, framing, `EntityAddr` (legacy and ADDR2), `EntityName`, `UTime`, `Uuid`, feature constants, message envelope (`RawMessage`, `Message`) |
| `auth` | done | part of 54 | cephx: AES (type 1) and AES256KRB5 (type 2) keys, keyring, monitor handshake, service tickets, authorizers (build and verify, with the CEPHX_V2 challenge), rotating secrets, msgr1 message signatures |
| `msgr` | done for msgr1 | part of 54 | tokio messenger: msgr1 client and server, lossless server and lossy client policies, signatures, keepalive, reconnect. msgr2 not implemented; the protocol sits behind internal frame traits for it |
| `monc` | done | part of 54 | MonClient (hunting, auth, ticket renewal, subscriptions), MDSMap decoding, `MMDSBeacon`, `Beacon` task, compat set, `CephConf`; `examples/probe.rs` is the minimal daemon |
| `proto` | done | 80 | Every CephFS client message (sessions, requests, replies with InodeStat v9, caps v12/v14, reconnect v3/v5, snap, quota, lease, reclaim, metrics). Byte-identical decode and re-encode of all captured traffic; ceph-dencoder cross-checks; fuzzed |
| `engine` | admission/visibility and recovery state ACCEPTED | 42 focused independent; 175 + doctest author | Wire-free state, one-record step protocol, committed output, checked restore/replay, materialized checkpoint, lifecycle and session receipts. |
| `osdc` | platform ACCEPTED; scoped native probe passed | 70 + doctest independent | OSDMap/CRUSH/Objecter, checked compounds, bounded native all-namespace listing and checked map waits. |
| `store` | core and authority/schema ACCEPTED | 50 independent | Guarded stable WAL, immutable checkpoints/trim, Preparing/Ready bootstrap, allocator reservations, durable rank sessions and schema 2. Legacy segment codec is fixture-only. |
| `mds` (`ceph-rmds`) | persistence/runtime packages and scoped native evidence ACCEPTED; complete G2 blocked | 82 independent; earlier persistence review 71 | Production `Engine<DaemonJournal>`, native Ready/clientless restart and durable kernel/FUSE smoke observed. Complete saved-state/nonreuse comparison awaits explicit user resume and a sole-QA retry using the accepted unique-mount correction on preserved fs44/meta88/data89. Nonempty G3 gathers remain held. |

### 3.1 Engine and OSD client

**engine:** accepted offline packages. Author and independent test scopes differ; see the canonical evidence table. Default engine clippy has baseline warnings; no workspace strict-lint pass is claimed.

An I/O-free metadata engine for one shard. It has no dependency on wire crates. The daemon translates its inputs and effects. Its API is documented at `crates/engine/src/lib.rs`.

- **Production entry points:** `Engine::step` consumes `EngineInput` and a one-record `RecordPermit`; `journal_event` materializes committed output before another admitted step. Checked checkpoint/restore/replay and session-key installation preserve durable state. Direct legacy request/cap/close helpers are not the daemon production path.
- **Effects:** client replies/caps/realm notifications and recall/eviction progress stay ordered through the session funnel. Durable lifecycle work uses committed job state and returns completion through a new admitted engine transaction.
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
  - current persistence uses one root shard/stream, full ancestor images and ordered replay; multiple streams/deltas are later gates;
  - production admits one record at a time; it prioritizes a simple committed-visibility proof over concurrency;
  - creator caps differ slightly from C++ (no `max_size` in an MKNOD reply, no `Ax`/`Xx` on read-only creates or new directories), which costs an extra round trip;
  - the readdir byte budget is an estimate;
  - the engine has no clock: the daemon must fill a zero request stamp.

**osdc:** accepted platform package and separately accepted retained native probe evidence. The historical base `rados_live` workload authenticated as `mds.rust` over msgr1 and covered:
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
  - `CheckLatestMap` now uses bounded native monitor version queries and explicit map-wait completion; live MapChanged listing and nonempty locators remain unqualified;
  - OSD connections use only the OSD's msgr1 address;
  - watch/notify, balanced reads and EC direct reads are not implemented;
  - helpers implemented locally that belong in `denc`: varints, `f32`, raw crc32c, the legacy-compat frame check, and `Ord` for `EntityAddr`.


### 3.2 Open problems reported per crate

- **auth/msgr/monc:**
  - msgr2 is missing;
  - ticket renewal and rotating-secret refresh are implemented but not yet observed live (needs a run of more than 45 minutes);
  - the MonClient hunts one monitor at a time;
  - MMonCommand/MonClient::command exist; MLog and MgrClient remain work;
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
   - filesystem/mount state is controlled by QA; old snapshots are not current observations. Fresh Rust setup is separately authorized before, not inside, an identity-preserving restart trial;
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
# Fresh filesystem setup requires separate QA authorization. Never run newfs inside a restart trial.
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

The historical prototype used `devcluster/newfs.sh` between fresh-filesystem runs because replay served an empty namespace. Do not use this procedure for G2/G3: preserve filesystem identity and pools. Only QA controls the shared cluster. OSDs use memstore; keep them alive during MDS-crash tests and stop them cleanly with `stop.sh`.

## 5. Next steps, in order

### 5.1 Historical prototype wiring — DONE 2026-10-13

`scripts/smoke.sh` passes: rank 0 active, kernel (msgr1) and ceph-fuse mounts, full I/O cycle.
The ledger records the pjdfstest tail as done on 2026-10-13: 8,769 passes, 28 TODO failures and one Y2038 client-wire case. Steps below describe historical prototype APIs/policy, not current production instructions. Current step/session/recovery behavior is in section 1 and `crates/mds/README.md`.

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

### 5.2 OSD client and current durable data-work boundary

The Objecter uses the same messenger identity as monitor/client traffic (`MsgrTransport::new(msgr)`). It follows subscribed OSDMaps and takes `client_inc` from the MDSMap rank incarnation. Self-blocklist/fencing is fatal. Reviewed `rados.rs` Result primitives use Ceph striping and check OSD errors; current durable workers, not log-only compatibility wrappers, call them:

- `PurgeInode` deletes objects `<ino hex>.<objno>` 0..N in the layout pool (`-ENOENT` ignored); verified end-to-end: the data pool is empty after a smoke run's unlinks.
- `TruncateData` sends `TRIMTRUNC` (object-local size, `truncate_seq`) to every object overlapping `[size, from)`.
- Size recovery retains durable ranges/layouts and applies size/mtime only through a checked completion transaction. Failed attempts retain intents.
- Eviction fences the durable peer-address union, confirms exact applied blocklist entries, raises the epoch barrier, then performs durable cleanup. Failed fencing never releases rights.
- Four lifecycle attempts may run at once. Each attempt preflights at most 4096 candidate object operations and 64 layouts. Oversized/failed work remains durable with capped retry delays. No data work runs before recovery/service readiness.

### 5.3 Paused build cycle: G0-G3

Follow `design/persistence-contract.md`. The user selected one stable guarded WAL object per stream and immutable checkpoints. Start with root shard 0, stream 0, full images and ordered replay. The control object is ownership authority; the shard map is only a routing projection.

1. G0 partial: contract, corrected partial QA tooling and scoped native platform validation ACCEPTED; isolated baseline and repaired C++ reference evidence recorded. Strict lint remains unresolved; GitHub CI execution is unverified. The canonical evidence table is in `progress_ledger.md`.
2. G1: safety packages accepted and combined offline QA passed at the recorded source snapshot; no separate full-gate completion declaration is made here.
3. G2: native Ready/bootstrap/clientless restart and same-filesystem smoke have accepted scoped evidence. Complete application trial exited 1 before comparison/nonreuse checks because a legacy mount ID was recycled. The unique-mount correction is ACCEPTED, but retry is PAUSED. After explicit user resume, assign one QA runner, check state read-only, then retry on preserved fs44/meta88/data89. Broader fault/uncertainty qualification remains separate.
4. G3: nonempty reconnect reconstruction, gather timeout/eviction, client replay and mounted-client qualification remain. Rejoin and the internal clientreplay hook exist; nonempty Open gathers are deliberately held.

The native daemon uses the failure-aware journal and step path. Source/tooling work and tests are PAUSED. Only handover/release checkpoint work remains authorized. Never serve speculative reads after failure or replace missing recovery state with an empty namespace. Future QA retry requires explicit user resume and a sole-owner assignment. Preserve the original failed-trial evidence, acknowledged namespace and filesystem identity.

Current defaults (`crates/mds/README.md`): Objecter actor timeout 30 s; checkpoint interval 30 s or 16 committed records; maximum framed record 1 MiB; retained WAL budget 64 MiB; retained checkpoint-generation limit 64. SIGUSR1 requests a quiescent immutable checkpoint. There is no checkpoint garbage collector. Capacity pauses ordinary admission; recovery is an explicit budget increase and same-identity restart, never metadata deletion. Initial inode-range allocation has no dynamic refill worker. Active messenger outbound backlog has no capacity feedback, so no end-to-end outbound memory bound is claimed.

Independent re-review ACCEPTED all five contract amendments. They cover persisted predecessor addresses/fencing, native bounded pool listing and format-object guards, rank-session acquisition/recovery join, one-record engine step/resume with durable cleanup completion, and WAL persistence of stray path/stable subvolume identity. Keep these normative interfaces stable. Native fresh bootstrap now has scoped accepted evidence; concurrent initialization, broader failure injection and complete G2/G3 qualification are not implied.

### 5.4 After that

Follow `design/workplan.md`:
- G4: autonomous subvolumes, authorization and bounded indexed state;
- G5: measured concurrency and dependency-safe multiple streams;
- M0b: msgr2 after G1 platform interfaces settle;
- G6: multiple ranks and migration;
- G7: operability and release qualification. Build earlier dependencies when needed.

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
