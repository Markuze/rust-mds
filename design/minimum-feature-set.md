# Rust MDS: minimum feature set

> Draft 2, 2026-10-11: decisions 1-7 recorded (8.1). A top-down answer to "which features must the Rust MDS have". Derived from the [feature catalog](../mds-rust-map/features/README.md) (935 entries) under the [constraints of the reference set](../mds-rust-map/README.md#goals-and-constraints-of-the-rust-mds) and the [architecture proposal](../README.md). Every catalog entry is classified in [`minimum-feature-set.json`](minimum-feature-set.json). [`minimum-feature-set-catalog.md`](minimum-feature-set-catalog.md) shows the same data as per-area tables with the reason for each entry.

## 1. The rule

A feature is in the minimum set (tier **P0**) when leaving it out breaks one of these:

1. **An unmodified current peer in its default flow.** A mount fails, a client asserts, hits a `BUG_ON`, hangs or loses data, the monitor evicts the daemon, or a default flow of mgr/volumes, mgr/nfs, cephadm, Rook or ceph-exporter fails.
2. **POSIX behaviour an application can observe.**
3. **Durability and failover.** Acknowledged metadata survives a crash, a standby takes over a failed rank, and the failed daemon is fenced.
4. **Tenant isolation.** MDS auth caps and quotas hold, and one tenant's failure does not stop another tenant.
5. **The product thesis.** Every subvolume is an autonomous shard, and shards spread over ranks.

Everything else is one of:

- **P1**: not needed for the first release. The P0 format and protocols are still designed so that it can be added.
- **P2**: optional, built on demand.
- **OUT**: never built.

The set can be small because the MDS decides what clients may use. Clients pick their code paths from:

- the CephFS feature bits in the session OPEN;
- the caps and leases the MDS grants;
- the fragtree it reports;
- the errors it returns.

Not offering a capability is how a feature is left out safely:

- no `Dc`/`Dr` caps on directories: the kernel never creates or unlinks asynchronously;
- a one-frag fragtree: no fragmentation;
- MKSNAP answered with EPERM: no snapshots.

The reverse also holds. A few feature bits must be advertised even for features we do not build, because without them current clients fall back to legacy code paths (4.2).

## 2. Result at a glance

| Tier | Entries | Content |
|---|---:|---|
| P0 | 503 | 478 the MDS implements; 25 the unmodified monitor provides once beacons are right |
| P1 | 123 | snapshots, early replies, async dirops, dentry leases, warm standby-replay, scrub, client and subvolume metrics |
| P2 | 83 | quiesce, quarantine, fscrypt, charmap, LazyIO, NFS delegations, extra admin commands |
| OUT | 226 | 137 C++ internals; C++ on-disk formats and offline tools; legacy paths; balancer, Mantle, dmClock; cross-rank 2PC |

P0 is large in entries, but much of it is surface rather than mechanism. Of the 478 entries the MDS implements:

- 248 reproduce a wire, admin or POSIX contract as it is;
- 132 keep an outcome with a new mechanism;
- 87 keep a subset;
- 11 only accept and ignore input.

The capability tree, in five layers:

| Layer | Area | P0 | P1 | P2 | OUT |
|---|---|---:|---:|---:|---:|
| A. Be a Ceph daemon | [Platform](minimum-feature-set-catalog.md#platform): msgr2, msgr1 listener, cephx, MonClient, MgrClient, OSD client | 31 | 1 | 2 | 2 |
| | [Membership and failover](minimum-feature-set-catalog.md#membership) | 65 | 3 | 3 | 1 |
| B. Be a CephFS server | [Sessions](minimum-feature-set-catalog.md#sessions) | 38 | 1 | 3 | 2 |
| | [Requests and replies](minimum-feature-set-catalog.md#requests) | 37 | 9 | 0 | 2 |
| | [Namespace and POSIX operations](minimum-feature-set-catalog.md#namespace) | 67 | 0 | 7 | 2 |
| | [Caps and leases](minimum-feature-set-catalog.md#caps) | 42 | 14 | 3 | 1 |
| C. Keep metadata safe | [Metadata store](minimum-feature-set-catalog.md#metadata-store) | 28 | 3 | 0 | 37 |
| | [Strays, purge, truncation, damage](minimum-feature-set-catalog.md#lifecycle) | 31 | 14 | 1 | 5 |
| D. Tenants and scale-out | [Subvolumes as shards](minimum-feature-set-catalog.md#tenancy): quotas, statistics, layouts, auth caps | 46 | 1 | 4 | 1 |
| | [Multiple ranks](minimum-feature-set-catalog.md#scale-out): placement, migration | 47 | 2 | 2 | 13 |
| E. Be operable | [Operations](minimum-feature-set-catalog.md#operations): admin, counters, health, config, orchestrators | 54 | 12 | 14 | 7 |
| | [Metrics, QoS, quiesce, quarantine](minimum-feature-set-catalog.md#qos-metrics) | 4 | 19 | 43 | 9 |
| P1 block | [Snapshots](minimum-feature-set-catalog.md#snapshots) | 13 | 44 | 1 | 7 |
| Gone | [C++ internals](minimum-feature-set-catalog.md#cxx-internal) | 0 | 0 | 0 | 137 |
| | **Total** | **503** | **123** | **83** | **226** |

## 3. Working assumptions

The classification rests on these assumptions. Section 8 lists the ones the team must confirm.

| # | Assumption |
|---|---|
| D1 | Current peers only: kernel 7.3, libcephfs and ceph-fuse at `main`, Ganesha V15.7, Rook v1.21. Legacy paths are OUT, except a basic msgr1 listener for kernel clients (decision 3). |
| D2 | New on-disk format. C++ formats and the C++ offline tools are OUT; their purposes are redesigned where still needed. |
| D3 | Shard = subvolume. Every directory with `ceph.dir.subvolume` is a shard root, and the rest of the namespace is the root shard. A shard has one owner rank at a time and its own WAL, inode-number leases, strays, purge queue, cache budget and request scheduling. |
| D4 | link and rename across subvolumes stay EXDEV, as today (F-SNAP-052). There is no cross-rank 2PC for namespace operations; only shard-root create, rename and remove cross shards. |
| D5 | Clients see one frag per directory. There is no split or merge, no hot-directory replication and no balancer, and replicas exist only at shard boundaries. |
| D6 | Multiple active ranks are P0. Whole shards are placed on ranks, with pins as hints, and migrate live. |
| D7 | Replies are sent only after durable commit. Early replies, clientreplay and async dirops are P1. |
| D8 | Snapshots are P1. P0 keeps snap realms for quota and subvolume roots and tolerates `.snap` traffic. |
| D9 | Takeover by a cold standby is P0; warm standby-replay is P1. |
| D10 | Damage is contained: EIO, and the rest keeps serving. Scrub is P1. |
| D11 | Per-shard fair scheduling is an internal design property. Client metrics are accepted, dmClock is OUT and subvolume metrics are P1. |
| D12 | Quiesce, quarantine, fscrypt, charmap, LazyIO and NFS delegations are P2. Inline data and Mantle are OUT. |
| D13 | The admin surface in P0 is what the monitor, mgr modules, cephadm, Rook and ceph-exporter use by default, plus operator essentials. |
| D14 | Every POSIX behaviour the C++ MDS provides is P0. |
| D15 | Advertise exactly the feature bits that keep current clients on their current code paths. |
| D16 | The C++ concurrency machinery is internal; only its client-visible outcomes are kept. |
| D17 | Homogeneous clusters only: every MDS daemon in the cluster is a Rust MDS. Clusters that mix C++ and Rust MDS daemons are out of scope. |
| D18 | Incremental delivery. Each item starts with its simplest correct form; performance and robustness policies come once a basic Rust MDS runs. Only choices that would otherwise force a rewrite are made up front: the day-one hooks of 5.1. |

## 4. What each area must provide in P0

Each item names its main catalog entries. The [catalog tables](minimum-feature-set-catalog.md) give every entry with its reason.

### 4.1 Layer A: be a Ceph daemon

**Platform** (31 P0)

- **msgr2.1, as strict as the kernel requires**, server and client, crc and secure modes (F-PROTO-002, -003, -005, -006, -009):
  - the first v2 address in `SERVER_IDENT` equals the MDSMap entry, nonce included;
  - `MSG_AUTH` is among the server features;
  - the server never sends `WAIT` or `KEEPALIVE2`, and `SESSION_RESET` always has `full=1`;
  - sequence numbers are strict;
  - lossless stateful-server policy with replay of unacknowledged messages.

  No compression (F-PROTO-004 OUT).
- **A basic msgr1 listener for kernel clients** (decision 3; F-PROTO-001, -013). It serves kernels mounted without `ms_mode`, such as ceph-csi under Rook's default `requireMsgr2: false`, the old device syntax, or `ms_mode=legacy`. Its scope follows the [kernel-over-msgr1 checklist](../mds-rust-map/cpp-map/00-architecture/transport-auth-and-client-library-contract.md#73-linux-kernel-client-over-msgr1-legacy):
  - accept side only, for client peers; outbound connections to mons, mgr, OSDs and peer MDS daemons stay msgr2;
  - the address vector lists v2 first and v1 second with the same nonce, so libcephfs, ceph-fuse and Ganesha keep dialling msgr2;
  - banner, connect with protocol 32, a cephx authorizer with exactly one challenge round;
  - header, front and data crcs, and the signed 21-byte footer, because the kernel requires `MSG_AUTH`;
  - strict `in_seq + 1`, ACK, KEEPALIVE2_ACK, RESETSESSION for unknown sessions, never WAIT.
- **One messenger and one address vector for everything**: mon session, beacons, OSD traffic and the client listener. The monitor fences a failed MDS by blocklisting that address vector, so RADOS I/O sent from another identity, such as a separate librados instance, escapes fencing (F-PROTO-106, F-CLU-036, F-CLU-109).
- **cephx** (F-PROTO-011, -012, F-ADM-001, -002):
  - service side: verify authorizers with the rotating `mds` secrets, for both key types (Rook v1.21 selects aes256k where the release supports it);
  - the ticket's mds caps become the session's auth caps;
  - client side towards mons, mgr and OSDs.
- **MonClient and MgrClient** (F-PROTO-103, -104, F-ADM-061, -062):
  - MonClient: hunting, subscriptions (monmap, config, mdsmap, mgrmap, osdmap), MConfig, MLog;
  - MgrClient: MMgrOpen, MMgrConfigure, MMgrReport with the counter schema.
- **An OSD client** (F-PROTO-105, F-STORE-090..093):
  - CRUSH placement identical to C++, OSDMap following, the resend rules and backoff;
  - the epoch barrier and `-EBLOCKLISTED`;
  - `FULL_FORCE` on every op, unique reqids as an `mds` entity;
  - plain RADOS ops only: cephadm's MDS caps carry no `x`, so object classes are not available.
- **Implementation** (decision 4): native Rust for every layer; librados and `libceph-common` are not used. The C++ sources (LGPL) and the map are the code references. The kernel's `net/ceph` is a behavioural reference for the client side only: it is GPL-2.0, so its code is read to understand the protocol but never translated. The project's own license is still to be chosen (open question in 8, item 12).
- **Ceph encoding rules**: versioned structs, trailing bytes skipped. Keep a corpus of golden encodings for every message sent or accepted. No client input may crash the daemon (F-PROTO-016, -018, -019).

**Membership and failover** (65 P0, 20 of them provided by the monitor; [monitor minimum](../mds-rust-map/cpp-map/01-cluster-integration/beacon-and-monitor.md#317-minimum-behaviour-required-by-an-unmodified-monitor))

- **Beacons** (F-CLU-021..027, -054, -055, -023):
  - sent from a thread that metadata work cannot block;
  - the monitor's `want_state` discipline: only legal transitions, `version` equal to the last applied epoch;
  - health metrics with ids 1..17 only;
  - a DNE beacon on SIGTERM so the rank fails over at once;
  - the daemon stops serving itself when beacon acks stop.
- **Compat set**, constant for the process lifetime (decision 2, F-CLU-014..019, F-QOS-033):
  - the C++ default incompat bits 1-12, which include 7, plus a private Rust bit N;
  - reserve N upstream, and use a high id such as 63 until then;
  - N merges into every new file system at its first promotion, which then locks C++ daemons out;
  - each later incompatible Rust format change adds a new bit, rolled out through the existing cephadm and Rook upgrade sequence (`max_mds 1`, standby-replay off);
  - a promoted daemon reads the Rust format marker in the metadata pool before touching anything, and stops without writing if the pool holds a non-Rust file system.
- **Rank states** (F-CLU-035, -037, -063..067, -094):
  - `up:creating` and `up:starting`;
  - `up:replay`, after waiting for `last_failure_osd_epoch`;
  - `up:reconnect`, `up:rejoin` (the monitor never skips it), `up:active` and `up:stopping`;
  - `up:resolve` and `up:clientreplay` are skipped.
- **Standby-replay**: `up:standby-replay` must be accepted as a cold follower. Rook enables standby-replay for file systems with `activeStandby: true`, as in its examples, and a daemon cannot refuse the role. A follower never reports `down:damaged` (F-CLU-039, -069).
- **Identity and process** (F-CLU-104, -105, -108, -058, -025, F-ADM-116):
  - respawn as a new identity when removed from the map or blocklisted;
  - accept the cephadm and Rook command lines;
  - a rank holder's wire identity is `mds.<rank>`;
  - `ceph_version` is exactly the image's version string, because Rook reruns its upgrade sequence when they differ.

### 4.2 Layer B: be a CephFS server

**Sessions** (38 P0)

- **Session messages** (F-PROTO-027..050):
  - the session state machine;
  - OPEN carrying the supported-feature bitmap, `metric_spec` and `cap_auths` (the kernel parses `cap_auths` by position);
  - RENEWCAPS echoing `seq`, STALE, autoclose;
  - the FLUSHMSG barrier and FORCE_RO;
  - REJECT only in answer to an open or a reconnect;
  - an exact `push_seq`.
- **Eviction** follows a fixed order: blocklist, wait for that OSDMap, raise the epoch barrier, then kill the session. OSDMap blocklists also kill sessions (F-PROTO-051, -052).
- **NFS-Ganesha** (F-PROTO-063, -046, -101, F-STORE-028, -075):
  - RESET reclaim: FSAL_CEPH sends it on every cmount creation and fails the export on any error except ENOENT;
  - rejection of a second session with an existing uuid;
  - the session `timeout` metadata;
  - sessions that outlive their connection;
  - uuid, timeout and auth name all persist across failover;
  - at rank recovery, a session that misses reconnect is evicted like any other, whether or not it carries `timeout` (decision 6). While the MDS stays up, a silent `timeout` session is kept until its successor reclaims it or `session_autoclose` hits.
- **Reconnect after failover** (F-PROTO-055..057, F-CLU-080..084):
  - accept MClientReconnect v3 and v5, and take caps, file locks and realms from it;
  - queue retried requests and answer the already-committed ones first.

CephFS feature bits to advertise (F-PROTO-020, -025):

| Bits | Advertise | Reason |
|---|---|---|
| 0-8, 11, 16, 21 | yes | No current client changes behaviour on them; advertise them as the C++ MDS does. |
| 9 `reply_encoding` | **required** | Otherwise both clients decode replies the legacy way. Implies InodeStat v9, DirStat v1 and LeaseStat v2. |
| 10 `reclaim_client` | **required** | Without it `ceph_start_reclaim` returns `-EOPNOTSUPP` and every Ganesha export fails. |
| 12 `multi_reconnect` | **required** | Without it libcephfs reconnects carry no realms, and the kernel fails reconnects above 2 GiB. |
| 13 `deleg_ino` | **required** | Keeps the current create path: every CREATE extra is `openc_response_t` with an empty delegation. No `Dc`/`Dr` is ever granted in P0. |
| 14 `metric_collect` | yes | Clients then send MClientMetrics, which must be accepted. |
| 17 `op_getvxattr` | **required** | Without it MDS-served vxattrs read ENODATA, which breaks mgr/volumes. |
| 18 `32bits_retry_fwd`, 20 `has_owner_uidgid` | **required** | Without them clients send request heads v1/v2; idmapped kernel creates fail. |
| 19 `new_snaprealm_info` | yes | libcephfs gets the new snap-trace format; the kernel does not set bit 19 and gets the old one. |
| 15 `alternate_name` | no | fscrypt is P2. |
| 22 | no | The kernel reads it as subvolume metrics, libcephfs as charmap. Turn it on with P1 subvolume metrics; never read a client's bit 22 as charmap capability. |
| 23 `blockdiff`, 24 `quarantine` | no | Snapshots are P1, quarantine P2. |

**Requests and replies** (37 P0)

- **Decoding** (F-PROTO-065..073):
  - MClientRequest v6 with head v3 (both clients' current paths) and the v6 tail;
  - paths relative to `#ino`, and snapdir paths;
  - new inodes take the client's owner uid/gid and its timestamp.
- **Forwarding** with a strictly increasing `num_fwd`. Never forward a tid that already got a reply: that is a kernel `BUG_ON` (F-PROTO-076, F-NS-053).
- **Idempotency**: a completed-request table recorded in the same transaction as the mutation and moved with its shard. A committed retry is answered, not executed again; executing an UNLINK, RENAME or `O_EXCL` CREATE twice returns the wrong error (F-NS-012, F-STORE-029).
- **Durability point**: reply only after durable commit in the shard's WAL, with group commit. `REQUEST_FLUSH_MDLOG` and synchronous cap flushes commit immediately (F-NS-010, F-STORE-012, F-PROTO-040, F-CAPS-020).
- **Reply encodings** (F-PROTO-079..090):
  - InodeStat v9, DirStat v1 and LeaseStat v2, all compat 1;
  - traces whose directory is the request's locked parent;
  - an extra payload only for CREATE (always `openc_response_t`), READDIR, LSSNAP, GETFILELOCK and GETVXATTR.

**Namespace and POSIX operations** (67 P0)

| Ops | Tier | Note |
|---|---|---|
| LOOKUP, GETATTR | P0 | rstat getattrs answered by the inode's owner, flagged AUTH |
| LOOKUPINO, LOOKUPHASH, LOOKUPPARENT, LOOKUPNAME | P0 | from any rank, for uncached inodes (NFS handles); MDS-private inode numbers give ESTALE |
| OPEN, CREATE, MKNOD, MKDIR, SYMLINK | P0 | created-inode extra |
| LINK, UNLINK, RMDIR, RENAME | P0 | EXDEV across shards; unlink-while-open; atomic rename, including over a target |
| SETATTR, SETXATTR, RMXATTR, GETVXATTR | P0 | fscrypt bits P2 |
| READDIR | P0 | one frag, (rjenkins hash, name) order, resume by name or by offset hash |
| SETFILELOCK, GETFILELOCK | P0 | fcntl and flock: blocking waits, interruption, EDEADLK, reinstated at reconnect, moved with the shard |
| LOOKUPSNAP, LSSNAP, MKSNAP | P0 tolerate | ENOENT, empty listing with FRAG_END, EPERM |
| RMSNAP, RENAMESNAP, SNAP_METADATA, READDIR_SNAPDIFF, FILE_BLOCKDIFF | P1 | ENOENT or EOPNOTSUPP in P0 |
| SETLAYOUT, SETDIRLAYOUT | P2 | kernel ioctls only; EOPNOTSUPP |

POSIX details that are easy to get wrong (F-NS-085..088, F-CACHE-004, -010, -071, -077, F-NS-072):

- Directories report `nlink` 1 and size 0; libcephfs aborts on anything else.
- Times come from the client clock. `version`, `change_attr` and `time_warp_seq` stay monotonic across failover and migration.
- The root directory carries an explicit default layout: mgr/volumes walks up to `/` looking for `ceph.dir.layout.pool`.

Virtual and plain xattrs the MDS serves in P0:

- **Set (SETXATTR/RMXATTR), gated by the `p` cap flag:**
  - `ceph.dir.layout*` and `ceph.file.layout*` (`.pool`, `.pool_namespace`, `.pool_id`, `.pool_name`, `.json` and the stripe fields);
  - `ceph.quota`, `ceph.quota.max_bytes`, `ceph.quota.max_files`;
  - `ceph.dir.subvolume`;
  - `ceph.dir.pin` and `ceph.dir.pin.distributed`;
  - `ceph.dir.pin.random`, accepted and ignored.
- **GETVXATTR:** the names above. ENODATA for every other `ceph.*` name, including the charmap ones that mgr/volumes `subvolume info` reads; it only accepts ENODATA as "not set".
- **Computed by clients from InodeStat:** `ceph.dir.{entries,files,subdirs,rentries,rfiles,rsubdirs,rbytes,rctime}`, quotas, layouts and pins.
- **Plain xattrs, stored opaquely:** `user.*`, `security.*`, `trusted.*` and `system.posix_acl_*`. Zero-length values round-trip, because mgr/volumes always writes `user.ceph.subvolume.earmark`.

**Caps and leases** (42 P0)

- **The cap model and the client-visible policy** (F-CAPS-001..006, -067):
  - the caps allowed in each inode state;
  - an exclusive writer gets `Fx|Fb|Fc` so it can buffer;
  - a conflicting operation waits until revocations are acknowledged.
- **Cap messages** (F-CAPS-003, -011, -018, -019):
  - GRANT, REVOKE and flushes follow the `seq`, `issue_seq` and `cap_id` rules;
  - FLUSH_ACK is sent only after commit, in commit order;
  - flushes are exactly-once across failover and migration.
- **Size management** (F-CAPS-021, -022, F-STORE-066, F-CLU-099):
  - `max_size` and client writeable ranges, never beyond the durable range;
  - when a writer is lost, recover the file size and mtime by probing the data objects.
- **Other cap duties** (F-CAPS-016, F-BG-008, F-CAPS-030..032):
  - the truncate notification;
  - prompt cap release for unlinked inodes;
  - stale-session handling;
  - cache-pressure recall and the late-release health warning.
- **Dentry caching** (F-CAPS-046, -038, -047):
  - in P0 clients cache dentries only through the directory's `Fs` cap;
  - every namespace change first revokes `Fs` from the other clients;
  - directory caps never include `Fc|Fr`, which would turn on async dirops;
  - replies carry null leases; real dentry leases are P1.

### 4.3 Layer C: keep metadata safe

**Metadata store** (28 P0)

- **Per-shard WAL in RADOS** (F-STORE-007..009, -016, -018, -036, -037):
  - one or more streams per shard, adaptive to load, so one hot subvolume (strict-I/O NFS) commits in parallel; each stream is its own sequence of RADOS objects, so streams land in different PGs;
  - one atomic, self-contained record per transaction, written to exactly one stream, with group commit; no cross-stream two-phase commit;
  - records are routed by the primary inode's hash, so every cap flush of a file stays in one stream and FLUSH_ACKs keep the kernel's cumulative order (F-CAPS-018);
  - records carry per-object versions, and replay applies each object's records in version order, in parallel across objects and streams;
  - aggregates (directory size and link count, rstat, mtime as a maximum) are commutative deltas that need neither a lock nor a replay order;
  - a non-commutative object stays locked until its record is durable, so a record only ever depends on records that are already durable;
  - the completed-request record is part of the mutation's record;
  - each stream checkpoints and trims on its own; checkpoints store object versions, so a stale record never overwrites newer state;
  - records stay small (inode and changed fields); rstat propagation and quota pushes stay off the commit path;
  - a commit starts immediately when someone waits on it (sync flush, `REQUEST_FLUSH_MDLOG`), never on a timer;
  - durable within about 5 s at the latest, which the kernel assumes;
  - the torn tail found by sequence numbers and checksums;
  - replay, then checkpoints into the store and trimming.
- **Directory index ordered by (hash24, name)** and split internally across RADOS objects. The C++ limit of 100,000 entries per dirfrag disappears; the OSD warns about omap objects above 200,000 keys (F-STORE-047, F-CACHE-077, -023).
- **Inode records with the field semantics clients see**, and remote (hard-link) dentries (F-CACHE-001, F-STORE-049, -050).
- **Inode numbers** (F-STORE-046, -069, -070, -073, F-PROTO-100):
  - allocated from range leases;
  - never reused, because NFS handles carry no generation number;
  - never in [0x100, 0x1000), always below 2^48 (ceph-fuse).
- **A transactional inode-number index**: inode → shard, location, primary parent and name. It is updated in the same transaction as the namespace change and reachable from any rank. It serves LOOKUPINO, LOOKUPPARENT, LOOKUPNAME and reconnect (F-STORE-062, F-CACHE-047). Backtraces are not needed for this; no current peer reads them.
- **A persisted per-shard set of files open for write**, to recover sizes after a failover (F-STORE-084).
- **Data-object naming and striping**, exactly as clients compute them (F-STORE-064).
- **Metadata-pool hygiene**: never touch the snap_schedule, cephfs-mirror and ceph-csi objects that share the pool (F-STORE-102).

**Strays, purge, truncation, damage** (31 P0)

- **Unlinked-but-open inodes** (F-NS-048, F-BG-001, -002):
  - they stay in a per-shard stray area outside the shard's rstat and quota, so quota usage drops at unlink;
  - they keep their prior path, so path-restricted clients stay authorized on them.
- **Purge** (F-BG-006, -014, -015, -017, -019, -021):
  - record the deletion intent durably before the stray metadata goes;
  - delete data objects up to the largest size a client may have written, with `FULL_FORCE`;
  - at-least-once execution after crashes;
  - a budget shared fairly across shards;
  - never before the shard's reconnect phase ends, and never while any session holds caps on the inode.
- **Truncation**: TRIMTRUNC semantics, and interrupted truncates resumed after recovery (F-STORE-065, F-CLU-100).
- **Failure handling** (F-STORE-100, F-BG-052..061):
  - write errors put the MDS into read-only mode and send FORCE_RO to clients;
  - damage is recorded per shard;
  - requests that touch damage get EIO;
  - a damaged directory never lists as empty;
  - `damage ls` and `damage rm` are available.

### 4.4 Layer D: tenants and scale-out

**Subvolumes as shards** (46 P0)

- **Shard creation** (F-SNAP-050, -051):
  - setting `ceph.dir.subvolume` creates a shard, also from a non-empty directory: mgr/volumes v2 creates `<uuid>` before it flags the base directory;
  - setting it again is a cheap no-op, since mgr/volumes does so on every subvolume open;
  - EINVAL for non-directories and nested subvolumes.
- **Cross-shard rules**: link and rename across subvolumes return EXDEV (F-SNAP-052). Creating shard roots, renaming them into `/volumes/_deleting` (subvolume removal) and removing them need a small crash-safe cross-shard protocol (F-STORE-032).
- **Quotas** (F-SNAP-042..048):
  - `ceph.quota.*` vxattrs;
  - every quota directory is a realm root, and the exact SPLIT notification reaches clients before the setxattr reply;
  - quota and rstat are carried in authoritative InodeStat;
  - MClientQuota pushes at the C++ thresholds;
  - enforcement stays client-side, as today.
- **Statistics** (F-SNAP-033..039):
  - dirstat is exact on read;
  - rstat at the shard root is exact enough for `bytes_used` and quotas, with a bounded lag;
  - above shard roots (groups, `/volumes`, `/`) a delta channel makes it eventually consistent, as in C++.
- **Layouts**: file and directory layouts with inheritance and pool validation (F-CACHE-007, F-NS-042, -072).
- **MDS auth caps as the tenant boundary** (F-ADM-006..018, -021):
  - the full cap grammar, as the monitor stores it;
  - `path=` is checked against the inode's current absolute path on whichever rank serves it;
  - `fsname=`, `uid=`/`gids=`, `root_squash`, `f` and `network`;
  - the `p` flag, which keeps tenants from changing quotas, layouts, pins and the subvolume flag;
  - the mount root is checked at session open.
- **mgr/volumes and mgr/nfs default flows** run on plain POSIX plus the vxattrs above (F-ADM-088..096, -100, -112):
  - volume, group and subvolume lifecycle;
  - resize, pin, authorize, evict, info;
  - earmark and enctag;
  - `.meta` files replaced by rename.

**Multiple ranks** (47 P0)

- **Shard map**: a shard map with fenced ownership epochs replaces the subtree maps, with an ownership agreement after a rank failure (F-CACHE-035, -036, F-CLU-074, -075). Every WAL append carries an OSD-checked guard on the owner's epoch (`cmpxattr` or `assert_version`), so a deposed owner can never append after a handoff or a failover (decision 4).
- **Placement** (F-MULTI-038..048):
  - `ceph.dir.pin` on subvolumes and groups;
  - `ceph.dir.pin.distributed` spreads a group's shards by consistent hash (Rook sets it on its CSI group);
  - `ceph.dir.pin.random` is accepted and ignored;
  - pins to ranks at or above `max_mds` take effect when the rank exists.
- **Live shard migration with unmodified clients** (F-MULTI-001..023, F-CAPS-036):
  - export targets published with MMDSLoadTargets;
  - the FLUSHMSG barrier;
  - a session force-opened on the importer;
  - requests blocked, then forwarded;
  - cap EXPORT/IMPORT with matching `cap_id`/`seq`/`mseq`;
  - writeable ranges, file locks, completed requests and flush records all move with the shard.
- **`max_mds` grow and shrink**: a stopping rank hands off all its shards, with their strays and purge queues, then requests `down:stopped`. cephadm and Rook upgrades rely on this when they set `max_mds 1` (F-CLU-040, -041, -102, F-MULTI-049, F-BG-022, -023).
- **LOOKUPINO and ESTALE recovery from any rank**: libcephfs (and so Ganesha) sends LOOKUPINO to a random rank (F-PROTO-099, F-CACHE-047, -050, F-NS-022).
- **Boundary copies**: the parent shard answers lookup and getattr of a shard-root inode with current attributes (F-CACHE-055).

### 4.5 Layer E: be operable

**Operations** (54 P0)

- **Transports**: the admin socket (ceph-exporter, `ceph daemon`) and `tell` restricted to `allow *`, with self-description and the C++ output and return-code conventions (F-ADM-026..031).
- **Commands**:
  - `get_command_descriptions`, `help`, `0` (ping; ceph-exporter expects `2`), `version`, `git_version`;
  - `counter dump|schema`, `perf dump|schema`;
  - `config show|get|set|unset|diff|help`, `injectargs`;
  - `status`;
  - `session ls` and `client ls`, with `hostname`, `root` and the metric_spec feature bits;
  - `session evict` and `client evict`, with the filter grammar mgr/volumes and ceph-csi use;
  - `dump_ops_in_flight` and the blocked and historic op dumps;
  - `flush journal`;
  - `damage ls|rm`.
- **Perf counters peers read** (F-ADM-066, -067, -069..072):
  - fs status: `mds_mem.{ino,dn,dir,cap}`, `mds_sessions.session_count`, `mds_server.handle_client_request`, `mds_log.replayed`;
  - dashboard: `mds_log.ev`, `mds_cache.num_strays`, `mds.{inodes,caps,subtrees,exported_inodes,imported_inodes}`;
  - Grafana: `mds.forward`, `mds.reply_latency`, `mds_server.handle_peer_request`, `mds_log.{evadd,seg}`, `mds_mem.{heap,rss}`, `objecter.{op_r,op_w}`.
- **Health checks** the MDS reports (F-ADM-073..079): MDS_DAMAGE, MDS_READ_ONLY, MDS_TRIM, MDS_CACHE_OVERSIZED, MDS_SLOW_REQUEST, MDS_SLOW_METADATA_IO, MDS_CLIENT_LATE_RELEASE, MDS_CLIENT_RECALL and MDS_CLIENT_OLDEST_TID (with their `_MANY` variants). While any MDS reports any metric, the monitor refuses Rook's unconfirmed `fs set max_mds` (F-ADM-083).
- **Configuration** (F-ADM-060..064, F-CACHE-061):
  - options use the C++ names, because the monitor validates names against its compiled table;
  - `mds_cache_memory_limit` is the real memory bound (Rook sets it to half the pod limit);
  - runtime changes take effect.
- **Orchestrators**: cephadm deployment and its upgrade choreography, Rook pods, probes and upgrade gating (F-ADM-113..116).
- **Cluster log** lines for evictions, denials, slow requests, write errors and damage (F-ADM-084).

**Metrics, QoS, quiesce, quarantine** (4 P0)

- Accept MClientMetrics on any rank without ever faulting the connection; skip unknown or newer items (F-PROTO-096, F-QOS-013).
- Advertise bit 14 with a metric spec of types 0-15 (F-QOS-011).
- Decode the perf queries in MMgrConfigure. Answering them, for `fs perf stats` and cephfs-top, is P1: mgr `stats` is not an always-on module (F-QOS-028).
- Per-shard fair scheduling, the isolation goal, is a property of the design rather than a catalog entry.

### 4.6 NFS-Ganesha and strict I/O

Ganesha support is required, and Ganesha is the client that stresses per-subvolume autonomy most. Facts from the [libcephfs and Ganesha page](../mds-rust-map/cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md) (3.16) and the ceph.git source:

- **One CephFS client per Ganesha daemon.** mgr/nfs defaults `cmount_path` to `/`, and exports with the same `cmount_path` share one libcephfs client (`src/pybind/mgr/nfs/ganesha_conf.py:170-175`). A Ganesha daemon therefore holds one MDS session per rank for all its exports and tenants, with caps `allow rw path=/`; tenant isolation is Ganesha's export configuration.
- **The session multiplexes.** Many NFS clients and uids share it, with many concurrent requests that can arrive out of tid order. fcntl locks are told apart by owner, and Ganesha never sends blocking locks.
- **Everything is synchronous.** Every NFS SETATTR, including the one that follows a create, is followed by a sync; COMMIT and stable writes sync too. With sync mounts or stable writes (strict I/O), every NFS WRITE becomes a data write to the OSDs plus a cap flush with `FLAG_SYNC`, i.e. one MDS round trip and one WAL commit.
- **Handles, not paths.** NFS handles carry only {ino, snapid, fscid}. On a cache miss libcephfs sends LOOKUPINO to a random rank; after a Ganesha restart or node takeover every NFS client's handles are resolved this way. Inodes known only by handle have no path, so libcephfs cannot buffer setattr or skip opens for them.
- **Session identity survives restarts.** Ganesha sets `uuid` and `timeout=300`, sends a reset-reclaim to every rank at every cmount creation (and on node takeover), and fails the export on any error except ENOENT. Its reclaim waits until every rank is up.

What follows for the design:

- **WAL commit latency is a product KPI.** It is the NFS metadata latency and the strict-I/O write latency. Hence the WAL sharded below the subvolume (4.3) and the strict-I/O benchmark (section 9).
- **Parallelism inside one session.** Scaling synchronous NFS across cores happens mostly inside a single session. The design keeps a session's requests independent and enforces only the orderings clients check, at the send side (5.1).
- **Per-session state must not couple shards.** Completed-request and flush records live in the shard that ran the operation; the session itself (identity, feature bits, `uuid`, `timeout`) is small rank-level state with a durable open.
- **Inode lookup is a hot path.** Any rank maps ino to shard without I/O, then does one indexed read inside the shard.
- **Availability coupling.** A rank in recovery blocks new Ganesha exports, because reclaim waits for every rank, so fast failover is worth more.
- **Recognising Ganesha.** No message says "I am Ganesha", and the MDS does not try to detect it. Behaviour follows the session metadata keys the client declares: `timeout` (FSAL_CEPH always sends 300) and `uuid` (FSAL_CEPH uses `ganesha-0x<hash>`). Kernel clients send `hostname`, `kernel_version`, `entity_id` and `root`; libcephfs adds `pid`, `ceph_version` and `ceph_sha1`.
- **Decisions taken for Ganesha:** no recovery hold for absent sessions (decision 6), and the shared-session limits are deferred with their hooks in place (decision 7).

## 5. P1: design now, build next

| Feature | Why it is needed | What the P0 design must already contain |
|---|---|---|
| Snapshots: snapflush, ceph-csi VolumeSnapshot and clones, snap_schedule, cephfs-mirror snapdiff/blockdiff | CSI snapshots and clones, scheduled snapshots, mirroring | the full realm node; versioned dentry keys (`first`/`last`) with room for old inode versions; a plan for a file-system-wide snapid allocator; SnapContext passed to purge and truncate; strays that can keep snapshot history |
| Early (unsafe) replies and `up:clientreplay` | latency for single-client metadata streams such as untar or checkout, where every create otherwise waits for a RADOS commit | a recovery state machine with a clientreplay phase; inode-number reservations journaled so a replayed create gets the same number |
| Kernel async dirops (`Dc`/`Dr`, delegated inode ranges); not to be forgotten (decision 5) | the C++ MDS never grants them because of its lock-cache defect, so kernel clients would get asynchronous create and unlink for the first time; expect kernel-side fixes, since the path has not run against an MDS since squid | bit 13 and `openc_response_t` (already P0); an inode allocator API that can delegate durable sub-ranges to a session and purge orphaned ones after session loss; a cap model with a client-exclusive directory state (`Fx` with `Fc`/`Fr` on a directory) that other clients' access revokes, forcing the holder to flush; validation of a client-chosen `head.ino` against the session's delegations, never a crash; the per-shard clientreplay phase shared with early replies |
| Dentry leases | path walks through shared directories that other clients are modifying | null leases already in every reply; a lease-revocation hook in every namespace mutation |
| Warm standby-replay | faster failover | per-shard WALs that a follower can tail without racing trimming |
| Scrub and repair of the new format; a per-shard `repaired` command | integrity tooling | checksummed records; the damage registry |
| `fs perf stats`, subvolume metrics (bit 22), `mds_rank_perf` | observability and per-subvolume SLAs | `subvolume_id` filled correctly in InodeStat from day one: the shard root, and never changed |

### 5.1 Day-one design hooks

These choices are made from the start, because changing them later would mean a rewrite (D18). Everything they enable can then be added incrementally.

| Hook | Enables later | Source |
|---|---|---|
| WAL records are self-contained, one per transaction, with per-object versions | several streams per shard; parallel replay | WAL decision |
| The completed-request record is written in the same WAL record as its mutation | exactly-once retries (correctness, P0); bounded in-memory window with lookup on retry | decision 7 |
| Dentry keys and inode records can carry snapshot versions (`first`/`last`, old versions) | snapshots | decision 1 |
| The recovery state machine has a per-shard clientreplay phase, empty at first | early replies; kernel async dirops | decision 5 |
| The inode allocator hands out range leases per shard and can delegate durable sub-ranges to a session | kernel async dirops (delegated inodes); replayed creates | decision 5 |
| The cap model has a client-exclusive directory state | kernel async dirops (`Dc`/`Dr`) | decision 5 |
| Shard ownership epochs, checked by the OSDs on every WAL append | live shard migration; multiple ranks; fencing as a backstop | decision 4 |
| One messenger abstraction over msgr1 and msgr2 | msgr2 and secure mode after the msgr1-first M0 | decisions 3 and 4 |
| Reading a connection is separate from executing its requests (decode, queue per shard, execute) | back-pressure per shard; fair scheduling between tenants | decision 7 |
| Every outgoing client message goes through one per-session send queue that keeps per-inode order and counts `push_seq` at send | parallel execution inside one session (strict-I/O NFS) | decision 7 |
| Caps are tracked where they live, per shard and per session | cap budgets; readdir throttling per (session, shard) | decision 7 |

## 6. OUT: what the minimal set removes

- **The global lock and the C++ concurrency machinery**: lock state machines, scatterlocks, auth pins, freezing, projection chains and finisher threads (137 entries).
- **Cross-rank namespace 2PC**: peer requests, rename witnesses, rollback, and the resolve of 2PC outcomes. EXDEV across subvolumes makes them unnecessary, so the proposal's cross-shard transactions (Phase 4) shrink to the shard-root operations.
- **Recovery between ranks**: `up:resolve`, the weak and strong rejoin exchange, and cluster-wide barriers. clientreplay returns with P1.
- **Dynamic partitioning inside a subvolume**: the balancer, popularity counters, Mantle, dirfrag split and merge, hot-directory replication and replica caps.
- **The snap-table 2PC on rank 0.** P1 snapshots can use a simple file-system-wide snapid allocator.
- **The C++ on-disk formats and their tools**: Journaler, EMetaBlob, the dirfrag omap layout, the InoTable, SessionMap and OpenFileTable objects, backtraces and scrub tags; cephfs-journal-tool, cephfs-data-scan and cephfs-table-tool.
- **Rank-wide singletons, replaced by per-shard equivalents**: the `~mdsN` stray directories, the rank-wide PurgeQueue and the SessionMap version chain.
- **Other**: dmClock and its admin surface, compression, inline data, legacy encodings and client fallbacks.

## 7. Invariants that must never be violated

Each of these makes an unmodified peer crash, hang, evict or blocklist itself.

- **Kernel client**:
  - never forward a tid that already got a reply, and keep `num_fwd` strictly increasing (`BUG_ON`);
  - a dentry trace's directory is the request's locked parent, and a LOOKUPNAME trace includes the target (`BUG_ON`);
  - no inode numbers in [0x100, 0x1000);
  - every MDSMap entry has a v2 address;
  - READDIR stays within `max_entries`, in non-decreasing hash order, always with its extra payload; an empty LSSNAP sets FRAG_END;
  - snap traces are well formed (otherwise the kernel blocklists itself);
  - a non-zero `subvolume_id` never changes;
  - REJECT is sent only in answer to an open or a reconnect.
- **libcephfs**:
  - only the eight session ops it knows;
  - lease messages are REVOKE only;
  - no trailing bytes in reply, snap and quota messages;
  - directory `nlink` 1, and `inline_version` never 0;
  - no traceless reply for an unlinked dentry;
  - FLUSHSNAP_ACK only from a rank holding a cap;
  - `cap_id` unchanged while `seq` does not advance.
- **Both clients**:
  - `push_seq` counts exactly the CAPS, SNAP, LEASE and QUOTA messages;
  - cap messages per (inode, rank) arrive in `seq` order;
  - FLUSH_ACK only after commit;
  - `max_size` never beyond the durable range;
  - eviction order is blocklist, latest OSDMap, epoch barrier, kill;
  - inode numbers are never reused and stay below 2^48.
- **Monitor and mgr**:
  - a health metric with id 0 aborts the monitor;
  - the compat set never changes after boot;
  - a standby never requests a state, and a standby-replay follower never reports `down:damaged`;
  - an mgr metric report of a type other than 0 or 1 aborts the mgr.

## 8. Decisions for the team

Questions 1-7 were discussed on 2026-10-11; the decision log (8.1) holds their outcome. Questions 8-12 are open.

1. **Snapshots: when and where.** *Deferred until P0 works, see 8.1.* This draft puts them in P1. Two questions need answers before the metadata format is frozen:
   - Does the first release need ceph-csi VolumeSnapshot and clones, snap_schedule and mirroring?
   - Are snapshots above shard roots supported? C++ allows `snap_schedule` on `/`. Supporting it means cross-shard snapshots; refusing it means EPERM.
2. **Compat sets and the deployment model.** *Decided: option (a), see 8.1.* The monitor keeps C++ and Rust daemons apart only through compat sets, and `fs new` copies the C++ default compat into every new file system (`fs volume create` and Rook both call it). Choose one:
   - **(a) Advertise the C++ default bits plus a private bit N.** Zero-touch: N merges into a new file system on its first promotion, and C++ daemons can never join it after that. In a mixed cluster, though, a Rust standby can be promoted into an upgradeable C++ file system, and a C++ standby can take a new file system before N merges. `mds_join_fs`, which cephadm and Rook set, plus `refuse_standby_for_another_fs` mitigate this.
   - **(b) Advertise only {7, N}.** Safe in mixed clusters, but every new file system then needs `fs compat` edits while no rank is up, which breaks zero-touch creation.
   - **(c) A small monitor change** that creates a file system with a given compat set.

   Reserve N upstream whichever option is chosen.
3. **msgr1 under Rook's default.** *Decided: a basic msgr1 listener for kernel clients is P0, see 8.1.* With Rook v1.21's CRD default `requireMsgr2: false`, ceph-csi mounts kernel clients without `ms_mode` on port 6789, which means msgr1. Either make `requireMsgr2: true` a deployment requirement (Rook's own example cluster sets it), or implement a msgr1 listener for kernel clients.
4. **Platform stack.** *Decided: native Rust, msgr1 first, see 8.1.* Native Rust msgr2, cephx, MonClient, MgrClient and OSD client (with CRUSH) on one messenger, or reuse of C++ libraries? Fencing requires every RADOS operation to leave from the beaconed address vector.
5. **Early replies in the first release?** *Decided: option A, safe-only in P0, with kernel async dirops tracked explicitly in P1; see 8.1.* Without them every metadata mutation from kernel and ceph-fuse clients waits for a RADOS commit. NFS-Ganesha is not affected, because it syncs after every SETATTR anyway. If single-client latency is a release criterion, move early replies and clientreplay to P0.
6. **Recovery hold for Ganesha sessions that miss reconnect.** *Decided: option (d), no hold, see 8.1.* C++ holds the whole rank in `up:clientreplay`, serving no new requests, for up to about 300 s when a `timeout` session misses reconnect. Options were (a) that rank-wide hold, (b) a hold per shard, (c) a reservation of the absent session's locks per inode, and (d) no hold.
7. **Limits on the shared Ganesha session.** *Deferred until a basic Rust MDS runs, see 8.1.* Completed-request bounding, cap budgets and readdir throttling per (session, shard), conservative eviction, per-shard back-pressure, and parallel execution inside one session. Their structural hooks are in 5.1.
8. **Shard boundaries.**
   - Who owns a shard-root inode? (Recommended: the child shard.)
   - Who issues caps on it when a client reaches it through the parent rank?
   - Where does the epoch-fenced shard map live?
   - Does the root shard stay on rank 0? Rank 0 never stops, and libcephfs sends its metrics there.
9. **Shard creation rules.**
   - Adopting a non-empty subtree when the flag is set is required by mgr/volumes v2.
   - Refuse clearing the flag, and refuse marking a directory above an existing subvolume?
   - Pins on directories that are not subvolumes (`/home/*` style) cannot become implicit shards, because that would silently turn renames between them into EXDEV. Ignore them for placement?
10. **Per-shard failure scope.**
   - Report one tenant's damage as HEALTH_ERR, which stops Rook's reconcile, or as WARN?
   - Is a per-shard `repaired` command needed?
   - The Ganesha reclaim gate: *decided, no hold at all (decision 6).*
11. **Fix rather than copy?** Candidates:
   - RMXATTR and GETVXATTR skip the path check;
   - a full metadata pool returns EACCES instead of the documented ENOSPC;
   - a damaged directory reads as silently empty;
   - quota usage is broadcast before commit;
   - flushed dirty bits are not validated.
12. **Smaller items.**
    - Turn on bit 22 with the P1 subvolume metrics.
    - Is LOOKUPNAME for kernel NFS re-export needed in P0?
    - Keep quotas client-enforced, or add MDS-side enforcement for per-subvolume SLAs?
    - How should Rust-only options be named?
    - How is the reported version string packaged?
    - Disaster-recovery tooling and backtraces for the new format (P2).
    - The project license. LGPL-2.1-or-later, like Ceph, would allow porting C++ Ceph logic; translating kernel code would require GPL-2.0.

### 8.1 Decision log

| Date | Question | Status |
|---|---|---|
| 2026-10-11 | Ganesha support | Confirmed required. Strict-I/O NFS clients (sync mounts, stable writes) turn every write into an MDS sync flush and a WAL commit; this is a primary load to design for. |
| 2026-10-11 | 1. Snapshots | Deferred: snapshots stay P1, and their scope (A, B or C) is decided with Igor once P0 works. Constraint until then: WAL records and dentry keys stay extensible to snapshot versions. |
| 2026-10-11 | WAL below the shard | Decided (requested by Alex): each shard has one or more WAL streams, adaptive to load. A transaction is one self-contained record in one stream, routed by the primary inode's hash so a file's cap flushes stay ordered. Records carry per-object versions; aggregates (sizes, link counts, rstat, mtime as max) are commutative deltas. A conflicting object stays locked until its record is durable. |
| 2026-10-11 | 2. Compat sets | Decided: option (a), the C++ default bits plus a private bit N. Homogeneous Rust-MDS clusters only (D17), so mixed-cluster mitigations are not needed. A format-marker check guards against a Rust daemon promoted into a pool that holds C++ CephFS data. |
| 2026-10-11 | 3. msgr1 | Decided: a basic msgr1 listener for kernel clients is P0, so tests can use kernel mounts without `ms_mode` and Rook's defaults. In the P0 end state it is the accept side for kernel clients only, and everything else uses msgr2; M0 starts with msgr1 everywhere (decision 4). |
| 2026-10-11 | 4. Platform stack | Decided: native Rust for every layer (option A); per-shard ownership epochs enforced by OSD guards on every WAL append. Staging: msgr1 first in both directions in M0, then msgr2 (client, server, secure mode) in M0b before P0 is done. |
| 2026-10-11 | 5. Early replies | Decided: option A. P0 replies only after durable commit; early replies are P1 and come before kernel async dirops, which share their replay and inode-reservation machinery. Kernel async dirops are tracked explicitly in P1, with their P0 design hooks listed in section 5. A single-client create-rate benchmark (kernel, ceph-fuse) joins the acceptance bar. |
| 2026-10-11 | 6. Recovery hold for absent Ganesha sessions | Decided: option (d), no hold. At the end of reconnect, a session that did not reconnect is evicted whether or not it carries `timeout`; every shard goes active at once. Gap accepted: if the MDS and a Ganesha fail together, native CephFS clients can take locks that NFS clients will try to reclaim through the new Ganesha (NFS-vs-NFS contention stays protected by Ganesha's grace). Mitigation: keep MDS and Ganesha in different failure domains. libcephfs `reclaim.cc` (reclaim after an MDS respawn) expects C++ behaviour and fails; FSAL_CEPH ignores the resulting ENOENT. Option (c), per-inode lock reservation, stays possible later. |
| 2026-10-11 | 7. Limits on the shared Ganesha session | Deferred until a basic Rust MDS runs: they are performance and robustness policies (completed-request bounding, cap budgets and readdir throttling per (session, shard), conservative eviction, per-shard back-pressure, parallel execution inside one session). Their correctness cores and structural hooks are in P0 now (5.1). F-PROTO-053 and F-NS-034 stay P1. Working principle adopted: incremental delivery (D18). |

### 8.2 Snapshot scope: analysis for the deferred decision

Input for the discussion with Igor (decision 1).

- **Who needs snapshots:**
  - ceph-csi VolumeSnapshot and clones (mgr/volumes clones by copying data out of a snapshot);
  - `subvolume rm --retain-snapshots`;
  - snap_schedule;
  - cephfs-mirror, including snapdiff and blockdiff; Rook's mirroring setup also adds snapshot schedules;
  - the dashboard file browser;
  - NFS clients browsing `.snap`.
- **What C++ allows:** snapshots on any directory, except below a subvolume root; mgr/volumes refuses group snapshots.
- **Shard-local if taken at shard roots.** The realm is the subvolume root and every inode in it lives in the same shard. Hard links never leave the shard, so no global realm and no realm replication between ranks are needed.
- **Snapshot ids stay global.** They must be unique and increasing per data pool and stay above every pool's `snap_seq`, which the monitor raises on every removal. Pre-leased per-shard id ranges are therefore unsafe; one file-system-wide counter read at each MKSNAP (a compare-and-swap on a RADOS object, no object classes needed) is enough, because MKSNAP is rare.
- **Snapshots above shard roots are the expensive part.** A snapshot of a group, `/volumes` or `/` needs a consistent cut across shards at one snapshot id, realm updates to the clients of every subvolume below, and snapflush and trimming in every one of those shards.
- **Options:**
  - **A. Subvolume roots only**, plus directories with no subvolume below them (plain CephFS keeps directory snapshots). EPERM for `/` or a group once subvolumes exist below it. Covers CSI, mgr/volumes, snap_schedule with `--subvol` and mirroring of subvolumes; breaks schedules or mirroring of `/` on file systems that also contain subvolumes. Edge rule: a directory under a live snapshot cannot become a subvolume.
  - **B. A, plus whole-file-system snapshots at `/`**, through one coordinated barrier across all shards.
  - **C. Full C++ generality.**
- **Recommendation so far:** A, with the format designed for it (5.1). B can be added later without a format change if a realm record can name a parent realm in another shard.

## 9. Acceptance and build order

**P0 acceptance.** These tests reach the MDS only through shared interfaces, so they run against any MDS ([QA inventory](../mds-rust-map/cpp-map/14-testing/qa-and-conformance-suites.md)):

- **External suites**:
  - pjdfstest (upstream);
  - fstests `generic` with the ceph exclusions, plus `ceph/001`-`005` (`006` needs snapshots);
  - LTP `fs` and `syscalls` with `TMPDIR` on the mount;
  - fsstress and fsx;
  - pynfs against a Ganesha FSAL_CEPH export.
- **qa modules marked "runs"**: `test_volumes.py` (303 of 313 tests run against any MDS; its snapshot and clone tests wait for P1), `test_nfs.py`, `test_quota.py`, `test_failover.py` (28 of 37), `test_admin.py` (105 of 129), `test_multifs_auth.py`, `test_pool_perm.py`.
- **Benchmarks** (measured against the C++ MDS on the same cluster):
  - strict-I/O NFS: fio with sync writes from many NFS clients, on one export and on many exports;
  - single-client create rate on kernel and ceph-fuse mounts (untar), to size the gap that early replies close.
- **Peer flows end to end**:
  - Rook CephFilesystem with ceph-csi: PVC create, expand and delete;
  - `fs volume create` and the cephadm upgrade choreography;
  - an mgr/nfs export through a Ganesha restart and a node takeover (reclaim).

**Build order inside P0.** These milestones are external; they compose with the proposal's concurrency phases. Per-shard journals are needed from M1, and the cross-shard work is limited to M2 and M3.

| Milestone | Content | Gate |
|---|---|---|
| M0 Platform (msgr1 first) | msgr1 in both directions with a v1-only address, cephx, MonClient, MgrClient, OSDMap, CRUSH, OSD client | the daemon boots, beacons and stays a healthy standby; CRUSH placement matches `osdmaptool`; a kernel without `ms_mode` and Ganesha complete a session open; test clusters keep `ms_bind_msgr1` on |
| M0b msgr2 (in parallel with M1 and M2) | msgr2 client and server in crc mode, then secure mode; the address vector becomes v2 first, v1 second | kernels with `ms_mode=prefer-crc` and `secure` mount; clusters with `requireMsgr2: true` work; must land before P0 is done |
| M1 One rank, one shard | sessions, request path, namespace ops, caps, WAL, inode allocation and index, strays and purge, failover (replay, reconnect, rejoin) | pjdfstest and fstests on kernel and ceph-fuse mounts; `kill -9` under load |
| M2 Subvolumes as shards | shard creation, EXDEV, quota realms, rstat aggregation, auth caps, mgr/volumes and mgr/nfs flows, Ganesha reclaim | `test_volumes.py` without snapshots, ceph-csi flows, pynfs |
| M3 Multiple ranks | shard map, placement, live migration, `max_mds` grow and shrink, LOOKUPINO from any rank | multi-rank failover and `max_mds` changes under load |
| M4 Operability | counters, health checks, admin commands, damage containment | cephadm and Rook upgrades end to end |

P1 follows, in the order set by decisions 1 and 5.

## Appendix: method

- Seven reviewers classified the 935 catalog entries, one or two categories each, all with the same rubric and the assumptions of section 3. Each reviewer read the full entries and the map pages they cite.
- Every output was checked by script: each live id exactly once, allowed values only.
- Five conflicts between categories were reconciled, touching 12 entries. They concern dentry leases, `fs perf stats`, the rank-stop purge drain, the timing of `subvolume_id`, and MMgrConfigure perf queries.
- Decisions taken later are applied the same way: decision 3 (msgr1) changed 11 entries and decision 6 (recovery hold) 3. The JSON marks every changed entry with an `override` field that gives the reason.
- Statements that decide a tier were spot-checked against ceph.git at `5f7e383ae6d`:
  - `stats` is not an always-on mgr module;
  - mgr/volumes sets charmap attributes only on request but reads them in `subvolume info`;
  - mgr/volumes v2 creates `<uuid>` before flagging the subvolume.
