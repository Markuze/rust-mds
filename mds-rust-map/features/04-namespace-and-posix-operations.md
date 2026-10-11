# Namespace and POSIX operations: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [request-lifecycle.md](../cpp-map/03-request-path/request-lifecycle.md) · [namespace-mutations.md](../cpp-map/03-request-path/namespace-mutations.md) · [rename.md](../cpp-map/03-request-path/rename.md) · [attributes-and-xattrs.md](../cpp-map/03-request-path/attributes-and-xattrs.md) · [lookup-readdir-open-and-file-locks.md](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md) · [posix-semantics-contract.md](../cpp-map/03-request-path/posix-semantics-contract.md)

## 1. Scope

This category covers every client metadata request the C++ MDS serves and its handler, from intake to the safe reply: the op set and dispatch, request identity, the re-entrant execution model, early and safe replies, the completed-request table that makes retries and replays idempotent, reply traces, and the per-op validation gates and error order. It then catalogues each POSIX-facing operation (lookup and getattr including the NFS inode-number lookups, open, readdir, create, mknod, mkdir, symlink, link, unlink, rmdir, rename, setattr and truncate, xattrs, ACL storage, the ceph.* virtual xattr path with layouts and charmap, GETVXATTR, fcntl and flock locks), the cross-rank protocols those operations use, and the POSIX departures recorded on the POSIX semantics contract page. Sources: the six fact-checked pages of cpp-map/03-request-path and the staging notes req-lifecycle, ns-mutations, rename, attrs-xattrs and reads-flocks.

Excluded and catalogued elsewhere: session messages, feature bits, reply encodings and MClientRequestForward (02); caps, dentry leases, lock caches, async dirops, delegated inode numbers, embedded releases, lock types and lock acquisition (03); path traversal internals, replica notifications and the subtree map (05); journal events, backtraces, InoTable/SessionMap objects and EOpen (06); export pins, balancer hooks and lock-table migration (07); snapshots (MKSNAP/RMSNAP/LSSNAP/RENAMESNAP/SNAP_METADATA/snapdiff), rstat propagation, quotas and their vxattrs, subvolumes and the EXDEV subvolume rule (08); strays and the purge queue (09); dmClock, quiesce and quarantine (10); MDS auth caps, perf counters, op-tracker admin commands and slow-request health (11). Where an entry here touches one of those mechanisms it describes only the request-side part and names the owning category in its notes.

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

91 features: 91 used by current peers (17 of them with legacy parts) and 0 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**Request intake, dispatch and execution model**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-001](#f-ns-001) | Client op set and dispatch switch | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-002](#f-ns-002) | Request intake gating by rank state and session | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-003](#f-ns-003) | Request identity and in-flight duplicate detection | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-004](#f-ns-004) | Re-entrant handler execution (restart from the top) | internal | internal | internal |
| [F-NS-005](#f-ns-005) | Per-op path resolution and locking helpers | internal | internal | internal |
| [F-NS-006](#f-ns-006) | GETATTR/LOOKUP batching (BatchOp) | internal | config | internal |
| [F-NS-007](#f-ns-007) | Request kill and abort | operational | admin | admin, internal |
| [F-NS-008](#f-ns-008) | Internal (MDS-originated) requests | internal | internal | internal |
| [F-NS-009](#f-ns-009) | Request-path fault injection and test kill points | internal | config | tools |

**Replies, idempotency and durability**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-010](#f-ns-010) | Journaled mutation commit and safe reply | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-011](#f-ns-011) | Early (unsafe) replies | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-012](#f-ns-012) | Completed-request table and idempotent retries/replays | supporting | client-wire, on-disk, metrics | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-013](#f-ns-013) | Replay of unsafe requests after an MDS restart | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-014](#f-ns-014) | Per-op reply trace contents | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-016](#f-ns-016) | On-demand journal flush for fsync, syncfs and sync writes | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Validation gates and POSIX error ordering**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-017](#f-ns-017) | Read-only and snapshot EROFS gates | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-018](#f-ns-018) | Full metadata pool gate | required | client-wire, cluster-wire | kclient, libcephfs, ceph-fuse, ganesha, osd |
| [F-NS-019](#f-ns-019) | Create/remove name gates and their error order | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-020](#f-ns-020) | Directory entry limits (ENOSPC) | operational | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha, admin |
| [F-NS-021](#f-ns-021) | Permission checking split: clients enforce POSIX, the MDS enforces auth caps *(legacy parts)* | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-022](#f-ns-022) | ESTALE recovery of unknown inode numbers | supporting | client-wire, mds-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |

**Lookup, getattr and inode-number lookups**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-023](#f-ns-023) | GETATTR (stat by inode) | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-024](#f-ns-024) | LOOKUP of one name in a directory | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-025](#f-ns-025) | Negative lookup replies (null-dentry traces) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-026](#f-ns-026) | LOOKUPINO and LOOKUPHASH (resolve an inode number) | supporting | client-wire, mds-wire, on-disk | kclient, libcephfs, ganesha, tools, mds-ranks |
| [F-NS-027](#f-ns-027) | LOOKUPPARENT (parent directory of an inode) | supporting | client-wire | kclient, libcephfs, ganesha |
| [F-NS-028](#f-ns-028) | LOOKUPNAME (name of an inode under its parent) | supporting | client-wire | kclient, libcephfs |
| [F-NS-029](#f-ns-029) | Reads at a snapshot id (LOOKUPSNAP, snapshot LOOKUPINO, open and readdir in snapshots) | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Open and readdir**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-030](#f-ns-030) | OPEN of an existing file or directory *(legacy parts)* | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-031](#f-ns-031) | O_TRUNC on open | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-032](#f-ns-032) | READDIR paging of one dirfrag in hash order *(legacy parts)* | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-033](#f-ns-033) | Readdir across dirfrags: frag selection and adjustment *(legacy parts)* | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-034](#f-ns-034) | Readdir cap-acquisition throttle | operational | config, metrics | kclient, libcephfs, admin, ceph-fuse, ganesha |

**Creating names: create, mknod, mkdir, symlink**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-035](#f-ns-035) | CREATE (open with O_CREAT, optionally O_EXCL) *(legacy parts)* | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-036](#f-ns-036) | MKNOD (regular file, FIFO, device, socket) | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-037](#f-ns-037) | MKDIR *(legacy parts)* | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-038](#f-ns-038) | SYMLINK (and readlink without an MDS op) *(legacy parts)* | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-039](#f-ns-039) | New inode initialization *(legacy parts)* | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-040](#f-ns-040) | Inode number allocation (session preallocation and InoTable) | supporting | on-disk, config | internal, kclient, tools |
| [F-NS-041](#f-ns-041) | Client-specified inode number (head.ino) on create | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-042](#f-ns-042) | File layout selection and validation at create | extension | client-wire | kclient, libcephfs, osd, ceph-fuse, ganesha |
| [F-NS-043](#f-ns-043) | Parent directory updates on link and unlink | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |

**Removing and relinking names: link, unlink, rmdir, rename**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-044](#f-ns-044) | LINK (hard link) *(legacy parts)* | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-045](#f-ns-045) | UNLINK of a primary link | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-046](#f-ns-046) | UNLINK of a remote (hard-link) dentry | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-047](#f-ns-047) | RMDIR with emptiness checks | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-048](#f-ns-048) | Unlinked-but-open inodes stay usable until the last cap is released | required | on-disk | kclient, libcephfs, ceph-fuse, ganesha, osd |
| [F-NS-049](#f-ns-049) | RENAME of a file or directory (move, same-inode no-op) | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-NS-050](#f-ns-050) | RENAME over an existing target | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-NS-051](#f-ns-051) | RENAME validation and error order *(legacy parts)* | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-052](#f-ns-052) | MDS-originated RENAME (stray reintegration and migration) | internal | mds-wire | internal, mds-ranks |

**Cross-rank execution of namespace operations**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-053](#f-ns-053) | Which rank serves each op (auth requirement, forward or discover) | supporting | client-wire, mds-wire, config | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-NS-054](#f-ns-054) | Peer requests and the LINK/UNLINK two-phase commit | internal | mds-wire, on-disk | mds-ranks |
| [F-NS-055](#f-ns-055) | RMDIR witness protocol | internal | mds-wire, on-disk | mds-ranks |
| [F-NS-056](#f-ns-056) | RENAME cross-rank two-phase commit | internal | mds-wire, on-disk | mds-ranks |
| [F-NS-057](#f-ns-057) | Inode authority and cap migration inside a cross-rank rename *(legacy parts)* | supporting | client-wire, mds-wire | kclient, libcephfs, mds-ranks, ceph-fuse, ganesha |

**Attributes: setattr, truncate and timestamps**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-058](#f-ns-058) | SETATTR request: mask, locks, error order and common tail *(legacy parts)* | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-059](#f-ns-059) | chmod and chown, with set-user/group-ID clearing | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-060](#f-ns-060) | Explicit times (utimes) and btime, with time_warp_seq *(legacy parts)* | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-061](#f-ns-061) | Size growth through SETATTR | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-062](#f-ns-062) | Truncate (shrink) with asynchronous OSD truncation | required | client-wire, cluster-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, osd |
| [F-NS-063](#f-ns-063) | Timestamps from the client clock and change_attr | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-064](#f-ns-064) | fscrypt inode fields and last-block truncate *(legacy parts)* | extension | client-wire, on-disk, config | kclient, libcephfs, osd |
| [F-NS-065](#f-ns-065) | Layout ops SETLAYOUT and SETDIRLAYOUT (kernel layout ioctls) | extension | client-wire | kclient |

**Extended attributes, ACLs and virtual xattrs**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-066](#f-ns-066) | SETXATTR and RMXATTR of stored xattrs | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-067](#f-ns-067) | Total xattr size limit (max_xattr_size) | supporting | client-wire, cluster-wire, admin | kclient, libcephfs, mon, ceph-fuse, ganesha |
| [F-NS-068](#f-ns-068) | xattr blob delivery to clients | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-069](#f-ns-069) | POSIX ACLs stored as opaque xattrs | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-070](#f-ns-070) | ceph.mirror.* xattrs for snapshot mirroring | extension | client-wire, on-disk | mgr, cephfs-mirror, libcephfs |
| [F-NS-071](#f-ns-071) | Virtual xattr set/remove path (common rules, p flag) | extension | client-wire, admin | kclient, libcephfs, ceph-fuse, ganesha, mgr, admin |
| [F-NS-072](#f-ns-072) | Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*) *(legacy parts)* | extension | client-wire, cluster-wire | kclient, libcephfs, ceph-fuse, ganesha, mgr, admin, osd |
| [F-NS-073](#f-ns-073) | GETVXATTR op | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, mgr, tools |
| [F-NS-074](#f-ns-074) | Client-synthesized ceph.* vxattrs (InodeStat contract) | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-NS-075](#f-ns-075) | Case-insensitive and normalized directories (charmap) *(legacy parts)* | extension | client-wire, on-disk | libcephfs, mgr, kclient, ceph-fuse, ganesha |
| [F-NS-076](#f-ns-076) | alternate_name on dentries | extension | client-wire, on-disk | kclient, libcephfs |

**Advisory file locks**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-077](#f-ns-077) | SETFILELOCK: POSIX fcntl byte-range locks | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-078](#f-ns-078) | BSD flock whole-file locks | extension | client-wire | kclient, libcephfs, ceph-fuse |
| [F-NS-079](#f-ns-079) | Blocking lock waits and wake-ups | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-080](#f-ns-080) | Lock wait interruption (FCNTL_INTR / FLOCK_INTR) | required | client-wire | kclient, libcephfs, ceph-fuse |
| [F-NS-081](#f-ns-081) | fcntl deadlock detection (EDEADLK) | required | client-wire | kclient, libcephfs, ceph-fuse |
| [F-NS-082](#f-ns-082) | GETFILELOCK (F_GETLK) | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-083](#f-ns-083) | Lock cleanup on cap removal and session end | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-084](#f-ns-084) | Lock reinstatement after an MDS restart *(legacy parts)* | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**POSIX contract and documented deviations**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-NS-085](#f-ns-085) | Directory link count and size presentation | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-086](#f-ns-086) | Access time is not maintained by the MDS | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-087](#f-ns-087) | Timestamp range limited to unsigned 32-bit seconds | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-088](#f-ns-088) | Limits enforced only by clients, and no hard-link limit | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-089](#f-ns-089) | Unsupported Linux extensions: renameat2 flags and O_TMPFILE | required | client-wire | kclient, ceph-fuse, libcephfs |
| [F-NS-090](#f-ns-090) | Data-path coherency departures outside the MDS | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-NS-091](#f-ns-091) | statfs is answered by the monitors | required | cluster-wire | kclient, libcephfs, ceph-fuse, ganesha, mon |
| [F-NS-092](#f-ns-092) | Conformance suites in the tree (pjd-fstest, xfstests) | operational | internal | tools |

## 3. Features

### Request intake, dispatch and execution model

How a client request becomes a registered MDS request, how it is routed to its handler, and how handlers wait and restart. Covers the op set itself, so every CEPH_MDS_OP_* is traceable from F-NS-001.

<a id="f-ns-001"></a>

#### F-NS-001 · Client op set and dispatch switch

*Maps every CEPH_MDS_OP_* a client can send to its handler; the op set is the client-visible metadata API.*

Ops are defined in src/include/ceph_fs.h (bit 0x1000 CEPH_MDS_OP_WRITE marks a write op) and routed by Server::dispatch_client_request: LOOKUP 0x100, GETATTR 0x101 and LOOKUPSNAP 0x400 -> handle_client_getattr (F-NS-023, F-NS-024, F-NS-029); LOOKUPHASH 0x102, LOOKUPINO 0x104, LOOKUPPARENT 0x103, LOOKUPNAME 0x105 -> handle_client_lookup_ino (F-NS-026..028); GETVXATTR 0x106 (F-NS-073); SETATTR 0x1108 (F-NS-058); SETLAYOUT 0x1107, SETDIRLAYOUT 0x110a (F-NS-065); SETXATTR 0x1105, RMXATTR 0x1106 (F-NS-066, F-NS-071); READDIR 0x305 (F-NS-032); SETFILELOCK 0x1109, GETFILELOCK 0x110 (F-NS-077, F-NS-082); CREATE 0x1301 (F-NS-035), OPEN 0x302 (F-NS-030); MKNOD 0x1201, MKDIR 0x1220, SYMLINK 0x1222 (F-NS-036..038); LINK 0x1202 (F-NS-044), UNLINK 0x1203 and RMDIR 0x1221 -> handle_client_unlink (F-NS-045..047); RENAME 0x1204 (F-NS-049). The snapshot ops LSSNAP 0x402, MKSNAP 0x1400, RMSNAP 0x1401, RENAMESNAP 0x1403, SNAP_METADATA 0x1406, READDIR_SNAPDIFF 0x1404 and FILE_BLOCKDIFF 0x1405 are dispatched by the same switch and are catalogued by the snapshots category (08). DUMMY 0x107 and the internal op codes 0x1500-0x150f sent by a client get -EOPNOTSUPP; internal codes are used only for MDS-originated requests (F-NS-008). MClientRequest::may_write() is true for ops with the write bit and for OPEN with O_CREAT or O_TRUNC; it drives the read-only gate (F-NS-017) and completed-request recording (F-NS-012).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [request-lifecycle.md › 3.5 dispatch_client_request](../cpp-map/03-request-path/request-lifecycle.md#35-dispatch_client_request) · [posix-semantics-contract.md › 5.1 What each POSIX call puts on the wire](../cpp-map/03-request-path/posix-semantics-contract.md#51-what-each-posix-call-puts-on-the-wire) · [lookup-readdir-open-and-file-locks.md › 3.1 Shared plumbing used by every handler on this page](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#31-shared-plumbing-used-by-every-handler-on-this-page)
- **Code:** `src/mds/Server.cc:2765 (Server::dispatch_client_request)`, `src/include/ceph_fs.h:396-451 (CEPH_MDS_OP_*)`, `src/messages/MClientRequest.h:196 (MClientRequest::may_write)`
- **Notes:** READDIR_SNAPDIFF (0x1404) and FILE_BLOCKDIFF (0x1405) carry the write bit although they only read, so they get -EROFS on a read-only MDS and are recorded in completed_requests (request-lifecycle 8, code reading). There is no readlink, access or statfs op: those are served by clients or monitors (F-NS-038, F-NS-021, F-NS-091). See also F-PROTO-073 (op code numbering and which client sends which op).

<a id="f-ns-002"></a>

#### F-NS-002 · Request intake gating by rank state and session

*Decides, before any handler runs, whether a client request is processed now, parked until the rank is active, queued for clientreplay, or dropped silently.*

Server::dispatch answers requests only in up:active, up:stopping, and (for requests queued for replay) up:clientreplay; in other states requests wait on waiting_for_active and are re-dispatched in FIFO order by MDSRank::active_start. In up:reconnect, requests flagged CEPH_MDS_FLAG_REPLAY or CEPH_MDS_FLAG_ASYNC, and retries of already-completed tids, are queued on the replay queue; retries without these flags lose their embedded releases and wait for active. Requests from a connection without a session, or whose session is closed, closing or killing, are dropped without a reply (a closed session is admitted in clientreplay only with mds_replay_unsafe_with_closed_session). Requests also wait while the cache is not open. CEPH_MSG_CLIENT_REQUEST is accepted from clients and from MDS daemons (MDS-originated stray renames).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_replay_unsafe_with_closed_session (default false, read at startup)
- **Map:** [request-lifecycle.md › 3.2 Intake and gating](../cpp-map/03-request-path/request-lifecycle.md#32-intake-and-gating) · [request-lifecycle.md › 3.3 handle_client_request: session checks, idempotency, r...](../cpp-map/03-request-path/request-lifecycle.md#33-handle_client_request-session-checks-idempotency-registration)
- **Code:** `src/mds/Server.cc:319 (Server::dispatch)`, `src/mds/Server.cc:2616 (Server::handle_client_request)`, `src/mds/MDSRank.cc:2129 (MDSRank::active_start)`
- **Notes:** Laggy deferral (all rank messages wait while the beacon is laggy), stale-MDS message filtering and dmClock QoS interception happen before this step and are catalogued by categories 01 and 10. Embedded cap/lease releases carried in the request are processed after registration and before dispatch (owned by category 03).

<a id="f-ns-003"></a>

#### F-NS-003 · Request identity and in-flight duplicate detection

*Identifies each request by (client entity name, client tid) so a resent request that is still executing is not run twice.*

The request id is metareqid_t(orig source entity, header tid) (MClientRequest::get_reqid); the MDS assumes one incarnation per client entity name. MDCache::request_start registers the request in active_requests; a second message with the same reqid is dropped, except that when the existing entry is a peer request for the same reqid (a forward race) the client message is parked until the peer request finishes. The client-supplied request stamp becomes the op stamp of the mutation (F-NS-063). Duplicate detection of in-flight requests uses only active_requests; completed ones are handled by F-NS-012.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [request-lifecycle.md › 3.3 handle_client_request: session checks, idempotency, r...](../cpp-map/03-request-path/request-lifecycle.md#33-handle_client_request-session-checks-idempotency-registration) · [request-lifecycle.md › 3.4 Idempotency: reqid and completed requests](../cpp-map/03-request-path/request-lifecycle.md#34-idempotency-reqid-and-completed-requests) · [request-lifecycle.md › 4.4 Invariants](../cpp-map/03-request-path/request-lifecycle.md#44-invariants)
- **Code:** `src/messages/MClientRequest.h:188 (MClientRequest::get_reqid)`, `src/mds/MDCache.cc:9817 (MDCache::request_start)`

<a id="f-ns-004"></a>

#### F-NS-004 · Re-entrant handler execution (restart from the top)

*Every wait re-dispatches the request from the top of its handler, with progress memoized on the request.*

Handlers never block a thread: a wait registers a C_MDS_RetryRequest (or factory-built context) on the object being waited for, and MDCache::dispatch_request later calls the handler again. Progress markers on the request (locking_state bits SNAP_LOCKED, SNAP2_LOCKED, PATH_LOCKED, ALL_LOCKED, already-held locks, witnessed peers, prepared stray dentry and snaprealm nodes) let the handler skip completed steps; Locker::drop_locks resets locking_state so the traversal is redone. Waits inside path_traverse drop all locks and local auth pins; waits for a lock state inside acquire_locks keep the locks already taken. The dead flag set at cleanup turns late contexts into no-ops.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [request-lifecycle.md › 3.6 Execution model: restart from the top](../cpp-map/03-request-path/request-lifecycle.md#36-execution-model-restart-from-the-top) · [namespace-mutations.md › 3.1 The common skeleton](../cpp-map/03-request-path/namespace-mutations.md#31-the-common-skeleton)
- **Code:** `src/mds/MDCache.cc:13271 (C_MDS_RetryRequest::finish)`, `src/mds/MDCache.cc:9995 (MDCache::dispatch_request)`, `src/mds/Locker.cc:878-891 (Locker::drop_locks)`, `src/mds/Mutation.h:281-287 (MutationImpl)`
- **Notes:** Re-entry has known consequences: a request woken from a quarantine wait skips the checks of the path helpers because PATH_LOCKED is already set (request-lifecycle 8 item 6, code reading), and dispatch_client_request re-runs the read-only and full-pool gates on every retry. Waits use C_MDS_RetryRequest or CF_MDS_RetryRequestFactory (which first drops locks); MDCache::dispatch_request re-runs dispatch_client_request, dispatch_peer_request or the internal-op dispatcher with mdr->retry incremented. All lock state is protected only by the rank-wide mds_lock; the restart cost is proportional to the path work.

<a id="f-ns-005"></a>

#### F-NS-005 · Per-op path resolution and locking helpers

*Shared helpers that resolve the request path(s), take path locks and return the target inode or dentry for each kind of op.*

rdlock_path_pin_ref (getattr, setattr, open, readdir, xattr ops) traverses with RDLOCK_PATH|RDLOCK_SNAP (+WANT_AUTH when asked), replies -ENOENT with a null-dentry trace when the client set CEPH_MDS_FLAG_WANT_DENTRY, starts ESTALE recovery, applies the quarantine block and auth-pins the target. rdlock_path_xlock_dentry (create, mknod, mkdir, symlink, link destination, unlink, rmdir) also xlocks the tail dentry and wrlocks the parent filelock/nestlock (rdlocks the parent authlock for creates). rdlock_two_paths_xlock_destdn (rename, path-addressed link) requires both paths to have depth 1 (-EINVAL), takes ancestor snaplock rdlocks for both parents and builds one lock vector in a canonical order (fs root before mdsdir, shallower first, then inode number, dentries by name). try_open_auth_dirfrag forwards to or opens a dirfrag; try_get_auth_inode returns -ESTALE for an uncached or purging inode and forwards when not auth.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [request-lifecycle.md › 3.7 Path traversal and locking helpers](../cpp-map/03-request-path/request-lifecycle.md#37-path-traversal-and-locking-helpers) · [namespace-mutations.md › 3.3 Path locking for a new or doomed dentry](../cpp-map/03-request-path/namespace-mutations.md#33-path-locking-for-a-new-or-doomed-dentry) · [rename.md › 3.6 Lock set and ordering](../cpp-map/03-request-path/rename.md#36-lock-set-and-ordering)
- **Code:** `src/mds/Server.cc:3879 (Server::rdlock_path_pin_ref)`, `src/mds/Server.cc:3960 (Server::rdlock_path_xlock_dentry)`, `src/mds/Server.cc:4073 (Server::rdlock_two_paths_xlock_destdn)`, `src/mds/Server.cc:4231 (Server::try_open_auth_dirfrag)`, `src/mds/Server.cc:5956 (Server::try_get_auth_inode)`
- **Notes:** Path traversal itself (MDCache::path_traverse) and lock acquisition/ordering rules belong to categories 05 and 03; this entry is the request-side use of them. Clients always send a parent inode plus one name (kernel) or resolve paths themselves (libcephfs), so multi-component paths are not needed for unmodified clients (posix-semantics-contract 3.1). See also F-CAPS-079 (locking flags), F-CACHE-041.

<a id="f-ns-006"></a>

#### F-NS-006 · GETATTR/LOOKUP batching (BatchOp)

*Coalesces identical concurrent GETATTR and LOOKUP requests so that only one of them takes locks.*

When mds_allow_batched_ops is true and the request holds no locks or pins, a GETATTR with depth-0 path or a LOOKUP with depth-1 non-snapshot path runs a lock-free traversal and looks up a batch keyed by (CInode or CDentry, mask). The first request becomes the head and executes normally; later identical requests join as followers holding only a pin. When the head replies, each live follower gets a reply with the head result and trace encoded with its own session (its own caps and leases); a forwarded head forwards all followers; a killed head hands the batch to the most recently added follower. A client that xlocks a lock named by the mask is never batched.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_allow_batched_ops (default true, runtime)
- **Map:** [request-lifecycle.md › 3.15 BatchOp: coalescing getattr and lookup](../cpp-map/03-request-path/request-lifecycle.md#315-batchop-coalescing-getattr-and-lookup) · [lookup-readdir-open-and-file-locks.md › 3.3 Batching (Batch_Getattr_Lookup)](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#33-batching-batch_getattr_lookup)
- **Code:** `src/mds/Server.cc:108-164 (Batch_Getattr_Lookup)`, `src/mds/Server.cc:4289-4337 (Server::handle_client_getattr)`, `src/mds/Mutation.cc:438 (MDRequestImpl::can_batch)`
- **Notes:** Followers never run check_access or check_quarantine_block and inherit the head result, so with path- or uid-restricted MDS caps a follower can receive attributes the head was allowed to read, or the head's -EACCES (lookup page 8, code reading). Followers' getattr_caps is 0, so their replies may omit xattrs/inline data they asked for.

<a id="f-ns-007"></a>

#### F-NS-007 · Request kill and abort

*Terminates an in-flight request on session close, admin "op kill", quiesce, migration or quarantine cleanup.*

MDCache::request_kill ignores killed or dead requests. A leader with witnesses or outstanding peer replies is only detached from its session (and marked aborted if not ALL_LOCKED), so it finishes without a client reply. Otherwise the request is marked killed: if it was already committing, only the session link is removed and the journal completion still sends the safe reply; else the request is cleaned up with no reply. The admin socket command "op kill <reqid>" calls this path, so a killed client request that was not committing leaves the client waiting until it resends on reconnect.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin, internal
- **Map:** [request-lifecycle.md › 3.14 Request kill and abort](../cpp-map/03-request-path/request-lifecycle.md#314-request-kill-and-abort) · [request-lifecycle.md › 5.6 Admin commands (asok / ceph tell mds.X)](../cpp-map/03-request-path/request-lifecycle.md#56-admin-commands-asok--ceph-tell-mdsx)
- **Code:** `src/mds/MDCache.cc:10155 (MDCache::request_kill)`, `src/mds/Locker.cc:700 (Locker::request_drop_remote_locks)`
- **Notes:** See also F-ADM-036 (op kill command).

<a id="f-ns-008"></a>

#### F-NS-008 · Internal (MDS-originated) requests

*MDS-internal work (fragmentation, export, scrub, flush, repair, uninline, quiesce, lock-path, quarantine) reuses the request machinery with internal op codes.*

MDCache::request_start_internal creates a request with reqid (mds.<rank>, tid) and an internal op code in 0x1500-0x150f (FRAGMENTDIR, EXPORTDIR, FLUSH, ENQUEUE_SCRUB, REPAIR_FRAGSTATS, REPAIR_INODESTATS, RDLOCK_FRAGSSTATS, QUIESCE_PATH, QUIESCE_INODE, LOCK_PATH, UNINLINE_DATA, QUARANTINEDIR_AUTH, QUARANTINE_INODE, QUARANTINEDIR_WORK). They skip check_access (no session), complete through internal_op_finish, and must set no_early_reply when they use journal_and_reply; quiesce, lock-path and quarantine ops are marked continuous and excluded from slow-request warnings. A forwarded internal op completes with -EXDEV and a killed one with -ECANCELED. Client requests carrying these op codes get -EOPNOTSUPP.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [request-lifecycle.md › 3.16 Internal requests](../cpp-map/03-request-path/request-lifecycle.md#316-internal-requests)
- **Code:** `src/mds/MDCache.cc:9870 (MDCache::request_start_internal)`, `src/mds/MDCache.cc:9995 (MDCache::dispatch_request)`

<a id="f-ns-009"></a>

#### F-NS-009 · Request-path fault injection and test kill points

*Debug options that delay, abort or alter request processing for QA.*

mds_server_dispatch_client_request_delay sleeps in dispatch while holding mds_lock; mds_server_dispatch_killpoint_random aborts the daemon at random; mds_inject_traceless_reply_probability drops reply traces of journaled requests (never for O_TRUNC), which clients tolerate (F-NS-014). Namespace ops have kill points mds_kill_openc_at, mds_kill_link_at, mds_kill_rename_at (1, 3-7), mds_kill_after_journal_logs_flushed, plus mds_inject_skip_replaying_inotable and mds_inject_rename_corrupt_dentry_first. All default to 0/false and are used only by qa suites.

- **Class:** POSIX `internal` · exposure `config` · relied on by tools
- **Gates:** mds_server_dispatch_client_request_delay; mds_server_dispatch_killpoint_random; mds_inject_traceless_reply_probability; mds_kill_openc_at; mds_kill_link_at; mds_kill_rename_at; mds_kill_after_journal_logs_flushed; mds_inject_skip_replaying_inotable; mds_inject_rename_corrupt_dentry_first
- **Map:** [request-lifecycle.md › 5.4 Configuration options](../cpp-map/03-request-path/request-lifecycle.md#54-configuration-options) · [namespace-mutations.md › 5.5 Configuration options](../cpp-map/03-request-path/namespace-mutations.md#55-configuration-options) · [rename.md › 5.5 Config options](../cpp-map/03-request-path/rename.md#55-config-options)
- **Code:** `src/mds/Server.cc:2765 (Server::dispatch_client_request)`, `src/mds/Server.cc:2491 (Server::set_trace_dist)`
- **Notes:** See also F-ADM-065 (fault injection options), F-STORE-112.

### Replies, idempotency and durability

The unsafe/safe reply pair, the completed-request table that makes retries and replays idempotent, reply traces, the created-ino payload of CREATE, and the journal flushes clients ask for in fsync.

<a id="f-ns-010"></a>

#### F-NS-010 · Journaled mutation commit and safe reply

*Every mutation is journaled before its effect is final; the safe reply is the metadata durability point clients wait for.*

Mutating handlers project the new state, build one EUpdate (with add_client_req for idempotency) and call Server::journal_and_reply, which optionally sends an early reply (F-NS-011), submits the event to the rank journal and flushes it when no early reply was sent. When the journal write is safe, the op-specific finisher applies the projections (MutationImpl::apply) and respond_to_request sends MClientReply with safe=1; xlocks, wrlocks and auth pins are held until then and dropped before the reply is sent so leases can be issued. Other clients therefore observe a mutation only after it is committed; requests needing those locks wait and nudge the journal (Locker::nudge_log). If the MDS fails before the commit the client replays the request (F-NS-013); if both client and MDS fail the change is lost.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [request-lifecycle.md › 3.10 Mutation commit pipeline: journal_and_reply](../cpp-map/03-request-path/request-lifecycle.md#310-mutation-commit-pipeline-journal_and_reply) · [request-lifecycle.md › 3.11 Replies](../cpp-map/03-request-path/request-lifecycle.md#311-replies) · [namespace-mutations.md › 3.10 Apply, commit and durability path](../cpp-map/03-request-path/namespace-mutations.md#310-apply-commit-and-durability-path) · [posix-semantics-contract.md › 3.6 Visibility and durability of metadata changes](../cpp-map/03-request-path/posix-semantics-contract.md#36-visibility-and-durability-of-metadata-changes)
- **Code:** `src/mds/Server.cc:2124 (Server::journal_and_reply)`, `src/mds/Server.cc:2379 (Server::reply_client_request)`, `src/mds/Server.cc:166-186 (ServerLogContext)`, `src/mds/Mutation.cc:254 (MutationImpl::apply)`
- **Notes:** Projections are popped FIFO, so correctness relies on journal completions being processed in submission order; CDir::mark_dirty tolerates out-of-order commits after tracker 79136 (namespace-mutations 3.6). Journal formats and the MDLog are catalogued by category 06.

<a id="f-ns-011"></a>

#### F-NS-011 · Early (unsafe) replies

*Answers a mutation to the requesting client before its journal entry is durable, followed later by a traceless safe reply.*

Server::early_reply sends MClientReply safe=0, result 0, with trace and extra_bl when mds_early_reply is true, no_early_reply is not set (SETDIRLAYOUT, dir layout/quota/subvolume/snaps.visible/quiesce.block vxattrs, internal ops), there are no journaled peers, no inode number was allocated from the InoTable, the requester is not an MDS and the request is not a replay. Before sending it moves held xlocks to LOCK_XLOCKDONE (rename keeps its dentry xlocks in LOCK_XLOCK), where only the xlocking client may read, rdlock or lease the object, so the requester can pipeline further ops while other clients wait for the commit; rdlocks other than snaplock/policylock rdlocks are dropped. The later safe reply carries no trace and no extra_bl. The system call returns on the unsafe reply; fsync/syncfs wait for the safe one (F-NS-016).

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_early_reply (default true)
- **Map:** [request-lifecycle.md › 3.11 Replies](../cpp-map/03-request-path/request-lifecycle.md#311-replies) · [rename.md › 3.6 Lock set and ordering](../cpp-map/03-request-path/rename.md#36-lock-set-and-ordering) · [posix-semantics-contract.md › 3.6 Visibility and durability of metadata changes](../cpp-map/03-request-path/posix-semantics-contract.md#36-visibility-and-durability-of-metadata-changes)
- **Code:** `src/mds/Server.cc:2300 (Server::early_reply)`, `src/mds/Locker.cc:781 (Locker::set_xlocks_done)`, `src/mds/Locker.cc:906 (Locker::handle_locks_for_early_reply)`
- **Notes:** libcephfs asserts that a safe reply following an unsafe one has an empty trace (Client.cc:1781-1785), so a trace there aborts the client process; the kernel ignores the content of such a safe reply. Per-op latency counters measure time to the first (possibly unsafe) reply. Early-replied ops are not flushed by their handler, so the safe reply can lag up to mds_tick_interval (5 s) unless something else flushes the journal. See also F-PROTO-089 (client handling), F-CAPS-092 (XLOCKDONE), F-CAPS-093 (rdlock release).

<a id="f-ns-012"></a>

#### F-NS-012 · Completed-request table and idempotent retries/replays

*Remembers successful write requests per session so that a retried or replayed request is answered instead of re-executed.*

In reply_client_request a successful may_write() request other than SETFILELOCK is recorded with Session::add_completed_request(tid, created_ino); journaled events carry the reqid and the client's oldest_client_tid (EMetaBlob::add_client_req), and the map is persisted in the rank SessionMap object. Only requests flagged CEPH_MDS_FLAG_REPLAY or with ext_num_retry > 0 are checked: a completed replay, or a completed retry of an op other than OPEN/CREATE, gets a traceless safe reply with result 0 and the created ino in extra_bl; in clientreplay a completed MKNOD/MKDIR/SYMLINK retry that created an inode is rewritten into LOOKUP (with WANT_DENTRY) or GETATTR so the client gets a trace; a retried (not replayed) CREATE is executed as OPEN of the created file without re-truncating. trim_completed_request_list erases tids below the request's oldest_client_tid; when it cannot trim and the map holds at least mds_max_completed_requests entries the MDS logs "client.N does not advance its oldest_client_tid" and raises MDS_CLIENT_OLDEST_TID(_MANY). Failed writes are not recorded, and completed entries are per rank and per session.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk`, `metrics` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_max_completed_requests (default 100000)
- **Map:** [request-lifecycle.md › 3.4 Idempotency: reqid and completed requests](../cpp-map/03-request-path/request-lifecycle.md#34-idempotency-reqid-and-completed-requests) · [namespace-mutations.md › 3.2 Entry, routing and idempotency](../cpp-map/03-request-path/namespace-mutations.md#32-entry-routing-and-idempotency) · [lookup-readdir-open-and-file-locks.md › 3.1 Shared plumbing used by every handler on this page](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#31-shared-plumbing-used-by-every-handler-on-this-page)
- **Code:** `src/mds/Server.cc:2659-2694 (Server::handle_client_request)`, `src/mds/Server.cc:2398-2405 (Server::reply_client_request)`, `src/mds/Server.cc:2563 (Server::trim_completed_request_list)`, `src/mds/SessionMap.h:307-321 (Session::trim_completed_requests)`
- **Notes:** A retried MKNOD/MKDIR/SYMLINK outside clientreplay gets a traceless reply whose extra_bl carries the created ino; the kernel rejects a non-empty extra_bl for these ops with -EIO, so a successful op is reported as failed (request-lifecycle 8 item 26, code reading). The kernel computes oldest_client_tid excluding SETFILELOCK requests (mds_client.c:1289). A completed request found while the session is not open returns without reply and without queue_one_replay (request-lifecycle 8 item 4). See also F-STORE-029 (journal side).

<a id="f-ns-013"></a>

#### F-NS-013 · Replay of unsafe requests after an MDS restart

*Handler-side rules for requests replayed after an MDS restart.*

See F-CLU-095 (re-execution in up:clientreplay) and F-CLU-084 (admission during up:reconnect). Handler-side facts: replays get no early reply and no embedded cap releases, reuse head.ino for creates (F-NS-041), get a traceless reply (try_reconnect_cap re-attaches the reconnected cap instead of set_trace_dist), and skip the cluster-degraded lock waits; a failed replay is logged to the cluster log as "failed to replay <reqid> error <r>". Replays that already completed are answered from the completed-request table (F-NS-012).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_replay_unsafe_with_closed_session (default false)
- **Map:** [request-lifecycle.md › 3.18 Clientreplay of client requests](../cpp-map/03-request-path/request-lifecycle.md#318-clientreplay-of-client-requests) · [request-lifecycle.md › 7.1 Linux kernel client (linux/fs/ceph)](../cpp-map/03-request-path/request-lifecycle.md#71-linux-kernel-client-linuxfsceph) · [namespace-mutations.md › 7. Externally visible contract](../cpp-map/03-request-path/namespace-mutations.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:349-381 (Server::dispatch)`, `src/mds/MDSRank.cc:2083 (MDSRank::clientreplay_start)`, `linux/fs/ceph/mds_client.c:4730 (replay_unsafe_requests)`
- **Notes:** An unsafe create from a client whose session failed to reconnect is lost although the client saw success (namespace-mutations 8).

<a id="f-ns-014"></a>

#### F-NS-014 · Per-op reply trace contents

*Which parent directory, dentry and target inode each reply describes; traces are how clients learn results and receive caps and leases.*

Server::set_trace_dist encodes the snap trace of the target realm, then (is_dentry) the parent directory InodeStat, its DirStat (frag, auth, replica set), the dentry name and a LeaseStat, then (is_target) the target InodeStat; encode_inodestat may create or extend a capability for the client on every inode it encodes. Per op: LOOKUP traces parent+dentry+target, a negative LOOKUP parent+null dentry; GETATTR, LOOKUPINO/HASH and SETATTR-style ops the target only; CREATE/MKNOD/MKDIR/SYMLINK/LINK the new dentry and inode; UNLINK/RMDIR the dentry with null linkage; RENAME the destination dir, destination name and the renamed inode (kclient BUG_ONs if the traced directory is not the request's destination parent); READDIR and LSSNAP the directory; file-lock ops carry no trace. Encoding versions (InodeStat v9, DirStat v1, LeaseStat v2 with CEPHFS_FEATURE_REPLY_ENCODING) are catalogued by category 02.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [request-lifecycle.md › 3.11 Replies](../cpp-map/03-request-path/request-lifecycle.md#311-replies) · [lookup-readdir-open-and-file-locks.md › 3.4 What a trace carries, and which caps are issued (CIno...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#34-what-a-trace-carries-and-which-caps-are-issued-cinodeencode_inodestat) · [lookup-readdir-open-and-file-locks.md › 7.1 Reply layouts](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#71-reply-layouts) · [rename.md › 7.1 Linux kernel client](../cpp-map/03-request-path/rename.md#71-linux-kernel-client)
- **Code:** `src/mds/Server.cc:2491 (Server::set_trace_dist)`, `src/mds/CInode.cc:3959 (CInode::encode_inodestat)`, `linux/fs/ceph/inode.c:1593 (ceph_fill_trace)`
- **Notes:** Traceless replies are tolerated: libcephfs invalidates directory completeness and issues its own lookup when it needs the inode (Client.cc:1787-1810, 2182-2224); the kernel does d_move itself for a traceless rename and an extra lookup for a traceless create. Readdir entries whose snaprealm differs from the directory get no caps.

<a id="f-ns-016"></a>

#### F-NS-016 · On-demand journal flush for fsync, syncfs and sync writes

*Lets clients make their unsafe metadata changes durable promptly, which fsync/syncfs and NFS commits wait for.*

A CEPH_SESSION_REQUEST_FLUSH_MDLOG session message makes an active MDS flush its journal; a cap message with the sync flag (CEPH_CLIENT_CAPS_SYNC) also flushes it. Kernel fsync sends dirty caps with the sync flag, sends FLUSH_MDLOG to every rank holding unsafe requests of the inode (or of the directory's operations) and waits without time limit for their safe replies; syncfs flushes all caps and journals and waits for every unsafe request. libcephfs _fsync does the same, unless syncdataonly. Otherwise early-replied mutations become durable only at the next journal flush (5 s tick, lock nudge, another op's flush, object boundary).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [posix-semantics-contract.md › 3.3.10 fsync, fdatasync, syncfs and synchronous writes](../cpp-map/03-request-path/posix-semantics-contract.md#3310-fsync-fdatasync-syncfs-and-synchronous-writes) · [namespace-mutations.md › 7. Externally visible contract](../cpp-map/03-request-path/namespace-mutations.md#7-externally-visible-contract) · [request-lifecycle.md › 5.1 Messages](../cpp-map/03-request-path/request-lifecycle.md#51-messages)
- **Code:** `src/mds/Server.cc:883-886 (Server::handle_client_session)`, `src/mds/Locker.cc:3656 (Locker::handle_client_caps)`, `linux/fs/ceph/caps.c:2405 (flush_mdlog_and_wait_inode_unsafe_requests)`, `src/client/Client.cc:13395 (Client::_fsync)`
- **Notes:** Kernel fsync does not wait for the size/mtime cap flush ("the mds can recover size and mtime"), so it relies on the MDS probing objects up to max_size when a writer's cap is lost by eviction (RecoveryQueue; posix-semantics-contract 3.3.10). fdatasync sends nothing to the MDS. The session message encoding is catalogued by category 02 and the journal by category 06. See also F-PROTO-040 (FLUSH_MDLOG message and client sync paths), F-CAPS-020 (sync cap flush).

### Validation gates and POSIX error ordering

Checks shared by many ops and the order in which they produce errors: read-only and full-pool gates, create/remove name gates, directory size limits, the split of permission checking between clients and the MDS, and ESTALE recovery.

<a id="f-ns-017"></a>

#### F-NS-017 · Read-only and snapshot EROFS gates

*Refuses writes when the MDS cache is read-only, inside snapshots and in MDS system directories.*

dispatch_client_request returns -EROFS for any may_write() request while MDCache::is_readonly() (set by MDCache::force_readonly on an unhandled journal/metadata write error when mds_action_on_write_error is 1, or by the force_readonly admin command). Path helpers return -EROFS for a create/remove name inside a snapshot (an empty path component) and for client requests whose parent is an MDS system directory other than the root; OPEN in write mode on a snapshot, SETATTR, SETXATTR and RMXATTR on a snapshot inode also return -EROFS. GETFILELOCK, GETATTR, LOOKUP*, READDIR and OPEN for read are not gated; SETFILELOCK is (it has the write bit).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_action_on_write_error (1 = go read-only); admin command force_readonly
- **Map:** [posix-semantics-contract.md › 3.1 Layers and the order of checks](../cpp-map/03-request-path/posix-semantics-contract.md#31-layers-and-the-order-of-checks) · [lookup-readdir-open-and-file-locks.md › 3.1 Shared plumbing used by every handler on this page](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#31-shared-plumbing-used-by-every-handler-on-this-page) · [request-lifecycle.md › 3.5 dispatch_client_request](../cpp-map/03-request-path/request-lifecycle.md#35-dispatch_client_request)
- **Code:** `src/mds/Server.cc:2794-2797 (Server::dispatch_client_request)`, `src/mds/Server.cc:3960 (Server::rdlock_path_xlock_dentry)`, `src/mds/Server.cc:4712-4717 (Server::handle_client_open)`, `src/mds/MDSRank.cc:981-986 (MDSRank::handle_write_error)`
- **Notes:** The vxattr set path (F-NS-071) has no snapshot check; clients refuse setxattr on snapshot inodes themselves. Clients also return -EROFS for snapshots before sending (kclient ceph_mknod/ceph_mkdir/ceph_setattr, libcephfs _create/_setxattr). The read-only state itself is catalogued by categories 01/06 (MDS_READ_ONLY health). See also F-SNAP-023 (snapshot read-only semantics).

<a id="f-ns-018"></a>

#### F-NS-018 · Full metadata pool gate

*Position of the metadata-pool-full gate in request dispatch.*

See F-STORE-099. Request-path specifics: the gate runs in Server::dispatch_client_request before the handler, on every (re-)dispatch; it fetches the request's base inode with try_get_auth_inode (-ESTALE if not cached) and requires MAY_FULL there for SETLAYOUT, SETDIRLAYOUT, RMXATTR, SETXATTR, CREATE, SYMLINK, MKSNAP, and LINK/RENAME before any witness acked. In SETATTR the size test runs even when SIZE is not in the mask.

- **Class:** POSIX `required` · exposure `client-wire`, `cluster-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, osd
- **Gates:** metadata pool FLAG_FULL (OSDMap); MDS auth cap flag f (MAY_FULL)
- **Map:** [posix-semantics-contract.md › 3.1 Layers and the order of checks](../cpp-map/03-request-path/posix-semantics-contract.md#31-layers-and-the-order-of-checks) · [request-lifecycle.md › 3.5 dispatch_client_request](../cpp-map/03-request-path/request-lifecycle.md#35-dispatch_client_request) · [attributes-and-xattrs.md › 3.1 Op codes, routing and pre-dispatch gates](../cpp-map/03-request-path/attributes-and-xattrs.md#31-op-codes-routing-and-pre-dispatch-gates)
- **Code:** `src/mds/Server.cc:2805-2836 (Server::dispatch_client_request)`, `src/mds/Server.cc:2752 (Server::handle_osd_map)`, `src/mds/Server.cc:5635-5640 (Server::handle_client_setattr)`
- **Notes:** Double reply on the same tid is found by code reading (request-lifecycle 8 item 5); not tested. Data-pool fullness is handled by clients and OSDs, not here. See also F-ADM-014 (f flag).

<a id="f-ns-019"></a>

#### F-NS-019 · Create/remove name gates and their error order

*Validates the name being created or removed and its parent before any op-specific check, fixing the order of EINVAL/EROFS/EEXIST/ENOTEMPTY/ENOENT.*

Server::rdlock_path_xlock_dentry checks, in order: path depth 0 -> -EINVAL; name inside .snap (empty component) -> -EROFS; name "." or ".." -> -EEXIST for creates, -ENOTEMPTY for UNLINK/RMDIR and the LINK destination; traversal errors (-ENOTDIR, -ENOENT, -ESTALE after recovery, -EIO for damaged metadata); quarantine wait for clients lacking CEPHFS_FEATURE_QUARANTINE; parent is an MDS system dir other than root -> -EROFS (unlink in lost+found allowed); parent already unlinked into a stray dir -> -ENOENT; absent name for remove -> -ENOENT; existing name for create -> -EEXIST (CREATE without O_EXCL opens it instead). Concurrent creators of one name serialize on the tail dentry xlock, so exactly one gets the name and the others get -EEXIST (O_EXCL) or open it. The MDS does not follow symlinks (-ENOTDIR for a non-directory component) and enforces no NAME_MAX.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [posix-semantics-contract.md › 3.1 Layers and the order of checks](../cpp-map/03-request-path/posix-semantics-contract.md#31-layers-and-the-order-of-checks) · [namespace-mutations.md › 3.3 Path locking for a new or doomed dentry](../cpp-map/03-request-path/namespace-mutations.md#33-path-locking-for-a-new-or-doomed-dentry) · [posix-semantics-contract.md › 6. Concurrency and mds_lock](../cpp-map/03-request-path/posix-semantics-contract.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/Server.cc:3960-4057 (Server::rdlock_path_xlock_dentry)`, `src/mds/MDCache.cc:8443 (MDCache::path_traverse)`, `src/mds/Server.cc:4913-4920 (Server::handle_client_openc)`
- **Notes:** The per-call order that follows these gates is in table 3.2 of the POSIX contract page and in each op entry below. Errors from client layers (VFS, kclient, libcephfs) come before any MDS error except for the kernel atomic_open, which sends CREATE/OPEN before the VFS may_open check (posix-semantics-contract 3.1).

<a id="f-ns-020"></a>

#### F-NS-020 · Directory entry limits (ENOSPC)

*The two ENOSPC entry limits as gates in the request path.*

See F-CACHE-080 (per-dirfrag limit, mds_bal_fragment_size_max) and F-CACHE-081 (per-directory limit, mds_dir_max_entries). Request-path specifics: check_fragment_space is called by CREATE, MKNOD, MKDIR, SYMLINK, LINK, RENAME (destination, unless link-merge) and by prepare_stray_dentry for the stray dirfrag (unless replay); check_dir_max_entries by the same callers except the stray dir. Because the stray dirfrag is checked, UNLINK, RMDIR and RENAME over a file can fail with -ENOSPC when the chosen stray dirfrag is full (F-BG-004).

- **Class:** POSIX `operational` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, admin
- **Gates:** mds_bal_fragment_size_max (default 100000); mds_dir_max_entries (default 0)
- **Map:** [request-lifecycle.md › 3.9 Validation helpers](../cpp-map/03-request-path/request-lifecycle.md#39-validation-helpers) · [posix-semantics-contract.md › 7.3 Further departures found in the code](../cpp-map/03-request-path/posix-semantics-contract.md#73-further-departures-found-in-the-code)
- **Code:** `src/mds/Server.cc:3563 (Server::check_fragment_space)`, `src/mds/Server.cc:3582 (Server::check_dir_max_entries)`, `src/mds/Server.cc:3595-3631 (Server::prepare_stray_dentry)`
- **Notes:** Same-directory renames are subject to both checks on the destination although the count does not grow (request-lifecycle 8 item 19).

<a id="f-ns-021"></a>

#### F-NS-021 · Permission checking split: clients enforce POSIX, the MDS enforces auth caps

*The MDS never evaluates POSIX mode bits, ACLs or the sticky bit for ordinary grants; it only evaluates MDS auth caps at fixed points per op.*

Mode bits, POSIX ACLs, the sticky bit and chmod/chown/utimes ownership rules are checked by the kernel VFS (generic_permission, may_delete_dentry, setattr_prepare) or by libcephfs (inode_permission, may_*, only when should_check_perms(): client_permissions=true for libcephfs, fuse_default_permissions=false for ceph-fuse). The MDS calls Server::check_access -> Session::check_access -> MDSAuthCaps::is_capable with the caller uid/gids the client wrote into the request: fs name, path prefix, network, r/w, root_squash, flags p/s/f, and a plain owner/group/other bit test only for grants restricted with uid=/gids=. MAY_EXECUTE is never requested by an MDS call site and src/mds contains no POSIX ACL or S_ISVTX logic. The per-op targets and masks, and the ops that make no check, are in F-ADM-018. check_access returns -EACCES, or -EIO for a directory whose layout has a pool namespace when the client lacks CEPH_FEATURE_FS_FILE_LAYOUT_V2.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The -EIO branch of Session::check_access for connections without CEPH_FEATURE_FS_FILE_LAYOUT_V2 (bit 58). Session::check_access returns -EIO for a directory whose layout has a pool namespace when the connection lacks CEPH_FEATURE_FS_FILE_LAYOUT_V2 (bit 58). The 7.3 kernel advertises bit 58 under its alias CRUSH_TUNABLES5 and current libcephfs advertises every feature, so no baseline client reaches this branch. Kernels before v4.5 and Ceph clients before 10.2.0 lack the bit. Evidence: `src/mds/SessionMap.cc:1156-1162 (Session::check_access)`, `linux/include/linux/ceph/ceph_features.h:217 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:236 (CEPH_FEATURES_ALL)`. Also: linux/include/linux/ceph/ceph_features.h:163,165 (CRUSH_TUNABLES5; FS_FILE_LAYOUT_V2 share bit 58); linux-v4.5/include/linux/ceph/ceph_features.h:75-78,125 (bit 58 first advertised; absent from linux-v4.4).
- **Gates:** client_permissions (lib, default true); fuse_default_permissions (default false); client_acl_type (lib, default empty)
- **Map:** [posix-semantics-contract.md › 1. Purpose and responsibilities](../cpp-map/03-request-path/posix-semantics-contract.md#1-purpose-and-responsibilities) · [attributes-and-xattrs.md › 3.2 Permission model: who checks what](../cpp-map/03-request-path/attributes-and-xattrs.md#32-permission-model-who-checks-what) · [request-lifecycle.md › 3.9 Validation helpers](../cpp-map/03-request-path/request-lifecycle.md#39-validation-helpers) · [auth-and-mds-caps.md › 3.7 Where checks happen: per request](../cpp-map/01-cluster-integration/auth-and-mds-caps.md#37-where-checks-happen-per-request)
- **Code:** `src/mds/Server.cc:3500 (Server::check_access)`, `src/mds/SessionMap.cc:1109 (Session::check_access)`, `src/mds/MDSAuthCaps.cc:266 (MDSAuthCaps::is_capable)`, `src/client/Client.h:2289-2291 (Client::should_check_perms)`
- **Notes:** The MDS auth-cap grammar and evaluation are catalogued by category 11. Consequences on the POSIX contract page 7.3: caller identity is asserted by the client; a client that skips its checks is limited only by its MDS caps; under uid= grants stat needs MAY_READ on the target; client-side checks are not atomic with other clients' mode/ACL changes. For changes flushed in cap messages the MDS checks neither the dirty bits nor values against the auth caps (capabilities 8). The kernel and libcephfs also evaluate MDS caps locally before using cached caps (ceph_mds_check_access, mds_check_access). libcephfs evaluates ACLs only with client_acl_type = posix_acl. File data access goes directly to OSDs and is governed by the client's OSD caps, not MDS caps.

<a id="f-ns-022"></a>

#### F-NS-022 · ESTALE recovery of unknown inode numbers

*Request-path use of the ESTALE recovery for unknown inode numbers.*

See F-CACHE-050. Request-path specifics: when path_traverse cannot find the base inode the path helpers run find_ino_peers and, on failure, C_MDS_TryFindInode falls back to MDCache::open_ino; Server::_try_open_ino then re-dispatches locally, forwards to the rank that has the inode, or maps -ENOENT/-ENODATA to -ESTALE. A purging inode gets -ESTALE immediately. Vxattr set/remove uses try_get_auth_inode and returns -ESTALE without this recovery (attributes 8 item 14).

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [request-lifecycle.md › 3.7 Path traversal and locking helpers](../cpp-map/03-request-path/request-lifecycle.md#37-path-traversal-and-locking-helpers) · [namespace-mutations.md › 3.2 Entry, routing and idempotency](../cpp-map/03-request-path/namespace-mutations.md#32-entry-routing-and-idempotency) · [lookup-readdir-open-and-file-locks.md › 3.1 Shared plumbing used by every handler on this page](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#31-shared-plumbing-used-by-every-handler-on-this-page)
- **Code:** `src/mds/Server.cc:3814-3867 (Server::_try_open_ino)`, `src/mds/MDCache.cc:9672 (MDCache::_do_find_ino_peer)`, `src/mds/MDCache.cc:9591 (MDCache::open_ino)`
- **Notes:** Kernel and libcephfs requests carry inode numbers (parent ino + name), so any inode a client still holds may be evicted from the MDS cache and must be re-found; NFS handles depend on the same machinery (F-NS-026).

### Lookup, getattr and inode-number lookups

Read-only resolution of names and inode numbers: GETATTR, LOOKUP (positive and negative), the NFS file-handle ops LOOKUPINO/LOOKUPHASH/LOOKUPPARENT/LOOKUPNAME, and reads at a snapshot id.

<a id="f-ns-023"></a>

#### F-NS-023 · GETATTR (stat by inode)

*Returns current attributes of an inode, rdlocking the field groups the client asks for unless it holds the matching exclusive cap.*

args.getattr.mask selects field groups: Ls -> rdlock linklock, As -> authlock, Xs -> xattrlock, Fs -> filelock, each skipped when the client holds the matching EXCL cap (its own view is then authoritative); CEPH_STAT_RSTAT forces the auth rank. If filelock is unstable, has no wrlocks and is readable by the client, it is not rdlocked and the current values are returned (the reply may then lack Fs); a change of that decision between retries makes the request drop its locks and requeue. Then check_access(MAY_READ); getattr_caps = mask so the reply includes the requested xattrs/inline data; tracei = inode. Rdlocking filelock revokes Fw/Fx/Fb from writers so their size and mtime are flushed first.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.2 GETATTR, LOOKUP and LOOKUPSNAP (Server::handle_client...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#32-getattr-lookup-and-lookupsnap-serverhandle_client_getattr) · [posix-semantics-contract.md › 3.3.6 stat, statx, access](../cpp-map/03-request-path/posix-semantics-contract.md#336-stat-statx-access) · [lookup-readdir-open-and-file-locks.md › 7.2 GETATTR and LOOKUP](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#72-getattr-and-lookup)
- **Code:** `src/mds/Server.cc:4272 (Server::handle_client_getattr)`, `src/mds/Server.cc:4343-4397 (Server::handle_client_getattr)`, `linux/fs/ceph/inode.c:2995-3015 (__ceph_do_getattr)`
- **Notes:** Clients send GETATTR only when cached caps do not cover the requested fields (kclient statx_to_caps; libcephfs). Any rank may answer reads; the kernel picks the auth when it holds an EXCL cap and asks for a SHARED bit, or for RSTAT/Xs. Under heavy write traffic the unstable-filelock shortcut can make a getattr loop several times (lookup page 8).

<a id="f-ns-024"></a>

#### F-NS-024 · LOOKUP of one name in a directory

*Resolves (parent ino, name) to an inode and returns parent, dentry, lease and target in one reply.*

LOOKUP is handled by handle_client_getattr with is_lookup: path depth 0 gives -EINVAL; otherwise the same mask-driven rdlocks as GETATTR apply to the target, with the traversed dentry rdlocked, and the reply traces the parent InodeStat, DirStat, name, a LeaseStat and the target. Hard-link (remote) dentries are resolved to their inode, opening it if not cached. Lookups of names whose dentry is xlocked by another client's uncommitted mutation wait for the commit and nudge the journal. The kernel sends mask CEPH_STAT_CAP_INODE | As (+ Xs with security xattrs) with CEPH_MDS_FLAG_WANT_DENTRY to any rank (USE_ANY_MDS).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.2 GETATTR, LOOKUP and LOOKUPSNAP (Server::handle_client...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#32-getattr-lookup-and-lookupsnap-serverhandle_client_getattr) · [lookup-readdir-open-and-file-locks.md › 7.2 GETATTR and LOOKUP](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#72-getattr-and-lookup) · [posix-semantics-contract.md › 3.5 Data coherency between clients](../cpp-map/03-request-path/posix-semantics-contract.md#35-data-coherency-between-clients)
- **Code:** `src/mds/Server.cc:4272 (Server::handle_client_getattr)`, `linux/fs/ceph/dir.c:806 (ceph_lookup)`, `src/mds/MDCache.cc:8443 (MDCache::path_traverse)`
- **Notes:** Dentry leases issued with the reply are catalogued by category 03 (valid only when the directory filelock is not in LOCK_SYNC and the client lacks Fs/Fx on it; 30 s).

<a id="f-ns-025"></a>

#### F-NS-025 · Negative lookup replies (null-dentry traces)

*Answers a lookup of a missing name with -ENOENT plus a trace of the parent and the null dentry, so clients can cache the negative result.*

When the name is absent from a complete auth dirfrag, or the dirfrag bloom filter excludes it, path_traverse returns -ENOENT without RADOS I/O and creates a rdlocked null CDentry; otherwise the incomplete dirfrag is fetched (whole dirfrag by default, or one omap key when mds_dir_prefetch is false) and the request retried. rdlock_path_pin_ref then replies -ENOENT with tracedn set when the client sent CEPH_MDS_FLAG_WANT_DENTRY (kclient lookups and libcephfs _do_lookup do), carrying the parent InodeStat, DirStat, name and a lease on the null dentry. Negative lookups are not batched and lookups inside snapshots never create null dentries.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_dir_prefetch (default true); mds_dir_prefetch_backend (default false)
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.6 Negative lookups](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#36-negative-lookups) · [request-lifecycle.md › 7.1 Linux kernel client (linux/fs/ceph)](../cpp-map/03-request-path/request-lifecycle.md#71-linux-kernel-client-linuxfsceph)
- **Code:** `src/mds/Server.cc:3906-3910 (Server::rdlock_path_pin_ref)`, `src/mds/MDCache.cc:8744-8801 (MDCache::path_traverse)`
- **Notes:** A lookup of the client's snapdir name (default .snap) has to fail with -ENOENT for the kernel to splice its virtual snapdir; the MDS has no notion of ".snap" and will create a real dentry of that name for a client using another snapdirname, which then shadows the snapdir for kernel clients (posix-semantics-contract 7.2 row 6, derived from code). A -ENOENT without trace is also accepted by both clients. A frozen dirfrag at the null-dentry step waits for UNFREEZE; with WANT_DENTRY traversal (creates) the caller receives the null dentry to fill. See also F-CACHE-023 (completeness), F-CAPS-039 (negative dentry leases).

<a id="f-ns-026"></a>

#### F-NS-026 · LOOKUPINO and LOOKUPHASH (resolve an inode number)

*Resolves a bare inode number to an inode, including inodes in no cache, for NFS file handles and libcephfs handle APIs.*

Inode numbers in [0x100, 0x1000) (mdsdirs, strays, logs) and purging inodes get -ESTALE. An uncached inode is opened with MDCache::open_ino: every other active rank is asked first (MMDSOpenIno), then the backtrace "parent" xattr of <ino>.00000000 is read from the default data pool (following the pool redirection, falling back to the metadata pool), ancestors are opened recursively; the request is then re-dispatched locally or forwarded to the auth; -ENOENT/-ENODATA become -ESTALE and a corrupt backtrace gives -EIO. Cached inodes are answered by any rank that has them. check_access(in, 0) applies only the MDS-cap path restriction; a non-zero mask rdlocks authlock/xattrlock (unless the client holds the EXCL cap) and then checks MAY_READ on the parent. LOOKUPHASH is handled identically; the (dirino, hash) path2 libcephfs sends is ignored.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire`, `on-disk` · relied on by kclient, libcephfs, ganesha, tools, mds-ranks
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.7 Lookup by inode number: LOOKUPINO, LOOKUPHASH, LOOKUP...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#37-lookup-by-inode-number-lookupino-lookuphash-lookupparent-lookupname-snapshot-inodes) · [lookup-readdir-open-and-file-locks.md › 7.3 NFS handle lookups](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#73-nfs-handle-lookups)
- **Code:** `src/mds/Server.cc:4433 (Server::handle_client_lookup_ino)`, `src/mds/Server.cc:4617 (Server::_lookup_ino_2)`, `src/mds/MDCache.cc:9591 (MDCache::open_ino)`, `linux/fs/ceph/export.c:131 (__lookup_inode)`
- **Notes:** Ganesha resolves handles with ceph_ll_lookup_inode/ceph_ll_lookup_vino, which send LOOKUPINO with mask 0. Resolution of uncached inodes stalls while any rank is down or recovering because the peer phase waits for all ranks to be active (lookup page 8). The kernel requires Ls via GETATTR afterwards to check nlink. See also F-CACHE-047 (open_ino), F-SNAP-025 (snapid branch).

<a id="f-ns-027"></a>

#### F-NS-027 · LOOKUPPARENT (parent directory of an inode)

*Returns the directory holding an inode's primary dentry, used by NFS to reconnect file handles.*

Same inode resolution as LOOKUPINO; then the primary parent dentry is rdlocked and the reply traces the parent directory inode. The root inode gives -EINVAL; an inode without a parent or whose parent is a stray directory gives -ESTALE.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ganesha
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.7 Lookup by inode number: LOOKUPINO, LOOKUPHASH, LOOKUP...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#37-lookup-by-inode-number-lookupino-lookuphash-lookupparent-lookupname-snapshot-inodes) · [lookup-readdir-open-and-file-locks.md › 7.3 NFS handle lookups](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#73-nfs-handle-lookups)
- **Code:** `src/mds/Server.cc:4433 (Server::handle_client_lookup_ino)`, `linux/fs/ceph/export.c:420 (ceph_fh_to_parent)`
- **Notes:** The kernel falls back to the parent ino stored in the handle when it gets -ENOENT.

<a id="f-ns-028"></a>

#### F-NS-028 · LOOKUPNAME (name of an inode under its parent)

*Returns the primary dentry name (and alternate_name) of an inode, used by NFS get_name.*

Same inode resolution as LOOKUPINO; the primary parent dentry (get_projected_parent_dn) is rdlocked and traced together with the inode; for hard-linked inodes the primary link is named. If the inode has no parent, or filepath2 names a parent inode different from the primary parent, the reply is -ENOENT.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.7 Lookup by inode number: LOOKUPINO, LOOKUPHASH, LOOKUP...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#37-lookup-by-inode-number-lookupino-lookuphash-lookupparent-lookupname-snapshot-inodes) · [lookup-readdir-open-and-file-locks.md › 7.3 NFS handle lookups](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#73-nfs-handle-lookups)
- **Code:** `src/mds/Server.cc:4433 (Server::handle_client_lookup_ino)`, `linux/fs/ceph/export.c:557 (ceph_get_name)`

<a id="f-ns-029"></a>

#### F-NS-029 · Reads at a snapshot id (LOOKUPSNAP, snapshot LOOKUPINO, open and readdir in snapshots)

*Handler-side facts about requests at a snapshot id.*

See F-SNAP-021 (LOOKUPSNAP and .snap paths) and F-SNAP-025 (LOOKUPINO with snapid). Handler-side facts: LOOKUPSNAP is never batched and carries no dentry trace; snapshot opens of files rdlock filelock so writers' snap flushes complete; readdir at a snapid returns the dentry versions alive at that snapid; replies inside snapshots carry no dentry leases and caps limited by lock type. Write attempts in snapshots get -EROFS (F-NS-017).

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.2 GETATTR, LOOKUP and LOOKUPSNAP (Server::handle_client...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#32-getattr-lookup-and-lookupsnap-serverhandle_client_getattr) · [lookup-readdir-open-and-file-locks.md › 3.7 Lookup by inode number: LOOKUPINO, LOOKUPHASH, LOOKUP...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#37-lookup-by-inode-number-lookupino-lookuphash-lookupparent-lookupname-snapshot-inodes) · [lookup-readdir-open-and-file-locks.md › 3.9 READDIR (Server::handle_client_readdir, Server::_fina...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#39-readdir-serverhandle_client_readdir-server_finalize_readdir) · [snapshots.md › 1. Purpose and responsibilities](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#1-purpose-and-responsibilities)
- **Code:** `src/mds/Server.cc:4539 (Server::_lookup_snap_ino)`, `src/mds/MDCache.cc:8538-8555 (MDCache::path_traverse)`, `src/mds/CInode.cc:4181-4206 (CInode::encode_inodestat)`
- **Notes:** The kernel returns -EOPNOTSUPP ("mds does not support lookup snapped inode") if the reply inode lacks the requested snapid. On the snapshot path of encode_inodestat cap_id and wanted are left uninitialized (lookup page 8).

### Open and readdir

OPEN of an existing inode (including O_TRUNC) and READDIR paging, frag handling and its cap-acquisition throttle.

<a id="f-ns-030"></a>

#### F-NS-030 · OPEN of an existing file or directory

*Registers the caps a client wants for an open mode and validates the open flags; no timestamp changes.*

Wire flags are CEPH_O_* (RDONLY 0, WRONLY 1, RDWR 2, CREAT 0100, EXCL 0200, TRUNC 01000, LAZY 020000, DIRECTORY 0200000, NOFOLLOW 0400000); ceph_flags_to_mode maps them to PIN/RD/WR/RDWR(+LAZY) and ceph_caps_for_mode to wanted caps (RD: Fs Fr Fc; WR: Fx Fw Fb As Ax Xs Xx; LAZY: Fl). Error order: bad mode -EINVAL -> write mode on a read-only cache -EROFS -> path errors -> O_DIRECTORY on a non-directory -EINVAL -> O_TRUNC on a directory -EISDIR / other non-regular -EINVAL -> inline-data inode and client lacking CEPH_FEATURE_MDS_INLINE_DATA -EPERM -> write in a snapshot -EROFS -> check_access(MAY_READ[|MAY_WRITE]) -EACCES. A read-only open of a regular file can be served by a replica; write, O_TRUNC and O_DIRECTORY opens need the auth. Locker::issue_new_caps records the wanted caps, which may revoke conflicting caps of other clients; writers get max_size later; EOpen is journaled for open-file tracking. The reply is always safe (no early reply unless O_TRUNC).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients, deprecated-admin): The -EPERM step ('old client cannot open inline data file') for an inode with inline data opened by a session whose connection lacks CEPH_FEATURE_MDS_INLINE_DATA (bit 40). The 7.3 kernel and current libcephfs advertise CEPH_FEATURE_MDS_INLINE_DATA, which the C++ feature table dates to kernel 3.19. Inline-data inodes are created only while the inline_data file-system flag is set; the monitor marks that flag deprecated and raises FS_INLINE_DATA_DEPRECATED, and no mgr module, cephadm or Rook sets it. Evidence: `src/mds/Server.cc:4704-4709 (Server::handle_client_open)`, `linux/include/linux/ceph/ceph_features.h:207 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:144 (MDS_INLINE_DATA // 3.19)`, `src/include/ceph_features.h:223 (CEPH_FEATURES_ALL)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mon/FSCommands.cc:471-478 (inline_data: --yes-i-really-really-mean-it, 'deprecated')`, `src/mds/MDSMap.cc:616-622 (FS_INLINE_DATA_DEPRECATED)`, `doc/cephfs/experimental-features.rst:24-28`. Also: searched src/pybind/mgr, src/cephadm and rook (*.go) for inline_data/uninline: the only Rook hits are RGW zone placement.
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.8 OPEN (Server::handle_client_open, Server::do_open_tru...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#38-open-serverhandle_client_open-serverdo_open_truncate) · [posix-semantics-contract.md › 3.3.1 open, creat and the open flags](../cpp-map/03-request-path/posix-semantics-contract.md#331-open-creat-and-the-open-flags) · [lookup-readdir-open-and-file-locks.md › 7.4 OPEN](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#74-open)
- **Code:** `src/mds/Server.cc:4639 (Server::handle_client_open)`, `src/common/ceph_fs.cc:116 (ceph_flags_to_mode)`, `src/common/ceph_fs.cc:143 (ceph_caps_for_mode)`, `linux/fs/ceph/file.c:379 (ceph_open)`
- **Notes:** The kernel skips OPEN when it already holds real caps (an auth cap for write modes) and only updates wanted caps through MClientCaps, so wanted-cap changes also arrive through cap messages. POSIX ENOTDIR for O_DIRECTORY on a non-directory comes from the VFS; the MDS answers -EINVAL (posix-semantics-contract 7.3). Open flags that reach the MDS are only the access mode, O_CREAT, O_EXCL, O_TRUNC, O_DIRECTORY, O_NOFOLLOW and CEPH_O_LAZY. Cap issue, max_size and EOpen journaling are catalogued by categories 03 and 06. See also F-CAPS-002 (wanted caps of an open mode).

<a id="f-ns-031"></a>

#### F-NS-031 · O_TRUNC on open

*Truncates an existing regular file to zero as part of OPEN and sets mtime/ctime, journaled and flushed at once.*

With CEPH_O_TRUNC (and not a retry of a completed request) the MDS xlocks filelock, checks MAY_WRITE, waits on WAIT_TRUNC while an earlier truncation is pending, then do_open_truncate issues the open caps, sets mtime = ctime = op stamp and change_attr += 1 even when the size is already 0, truncates to 0 when the old size (max of MDS size and args.open.old_size) is non-zero (truncate_seq++, truncate_start journaled), gives writers client_ranges [0, layout period], and journals EUpdate "open_truncate". An early reply is allowed, followed by an unconditional journal flush; the parent directory mtime is not touched. On commit clients get CEPH_CAP_OP_TRUNC and the OSD objects are truncated (F-NS-062). O_TRUNC is ignored for a followed symlink.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [attributes-and-xattrs.md › 3.4 O_TRUNC on open (src/mds/Server.cc:5780 (Server::do_o...](../cpp-map/03-request-path/attributes-and-xattrs.md#34-o_trunc-on-open-srcmdsservercc5780-serverdo_open_truncate) · [lookup-readdir-open-and-file-locks.md › 3.8 OPEN (Server::handle_client_open, Server::do_open_tru...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#38-open-serverhandle_client_open-serverdo_open_truncate) · [posix-semantics-contract.md › 3.2 Main table](../cpp-map/03-request-path/posix-semantics-contract.md#32-main-table)
- **Code:** `src/mds/Server.cc:5780 (Server::do_open_truncate)`, `src/mds/Server.cc:4737-4760 (Server::handle_client_open)`
- **Notes:** The kernel strips O_TRUNC in ceph_atomic_open and truncates through SETATTR after the open; libcephfs sends OPEN with O_TRUNC whenever O_TRUNC is given for an existing file. When ceph_open does send OPEN with O_TRUNC the VFS truncates again with another setattr (posix-semantics-contract 8, derived from code).

<a id="f-ns-032"></a>

#### F-NS-032 · READDIR paging of one dirfrag in hash order

*Lists one directory fragment per request, resuming after the last name or at a hash offset, with per-entry InodeStat, caps and lease.*

The directory filelock and dirfragtreelock are rdlocked and MAY_READ checked; -ENOTDIR for a non-directory. The resume point is path2 (last name returned) or args.readdir.offset_hash; entries come in dentry_key_t order (24-bit frag value of the dentry hash, then name, then snapid) from a single dirfrag, skipping purging dentries, null dentries, inode 2 (/.ceph) and remote dentries flagged BADREMOTEINO; hard-link entries open their target inode if needed (the reply may be cut short there). Budgets: max_entries (0 = all) and max_bytes (0 = 512 KiB + max_xattr_size); an entry that does not fit stops the reply, possibly with 0 entries. The extra blob is DirStat, u32 count, u16 flags (FRAG_END 1, FRAG_COMPLETE 0x100, HASH_ORDER 0x200 and OFFSET_HASH 0x400 when the client sent REPLY_BITFLAGS), then (name, LeaseStat, InodeStat) per entry; tracei is the directory. Clients emit "." and ".." themselves and derive offsets (0xff << 52) | (hash << 28) | n, so non-decreasing hash order and a stable order within a hash are what keep telldir/seekdir offsets valid; the kernel fails with -EIO on more entries than max_entries or an offset below its position. No directory atime is updated.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Reply flags without CEPH_READDIR_HASH_ORDER and CEPH_READDIR_OFFSET_HASH, produced when the READDIR request lacks CEPH_READDIR_REPLY_BITFLAGS. The 7.3 kernel (ceph_readdir, __get_snap_name) and current libcephfs (Client::readdir_r_cb) always set CEPH_READDIR_REPLY_BITFLAGS. Requests without it come from kernels before v4.7 and Ceph clients before 10.2.2. Evidence: `src/mds/Server.cc:5323-5327 (Server::handle_client_readdir)`, `linux/fs/ceph/dir.c:450-451 (ceph_readdir)`, `linux/fs/ceph/export.c:502-503 (__get_snap_name)`, `src/client/Client.cc:10116 (Client::readdir_r_cb)`. Also: linux-v4.7/fs/ceph/dir.c:409 (flag sent; linux-v4.5/fs/ceph/dir.c has no CEPH_READDIR_REPLY_BITFLAGS); v10.2.2:src/client/Client.cc:7012 (flag sent; absent at v10.2.1).
- **Gates:** mds_default_dir_hash (default 2, rjenkins); MDSMap max_xattr_size (default max_bytes)
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.9 READDIR (Server::handle_client_readdir, Server::_fina...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#39-readdir-serverhandle_client_readdir-server_finalize_readdir) · [lookup-readdir-open-and-file-locks.md › 7.5 READDIR](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#75-readdir) · [posix-semantics-contract.md › 3.3.9 readdir, telldir, seekdir](../cpp-map/03-request-path/posix-semantics-contract.md#339-readdir-telldir-seekdir)
- **Code:** `src/mds/Server.cc:5092 (Server::handle_client_readdir)`, `src/mds/Server.cc:5050 (Server::_finalize_readdir)`, `src/mds/mdstypes.h:388-422 (dentry_key_t)`, `linux/fs/ceph/dir.c:60-76 (ceph_make_fpos)`
- **Notes:** The dentry hash function must be the one reported in InodeStat dir_layout. Readdir shows a client its own early-replied mutations (use_projected) but other clients only committed linkage. Locks are held across the dirfrag fetch. Entries in a different snaprealm (subvolume roots, snapshot-bearing dirs) get no caps. See also F-NS-033 (across dirfrags), F-PROTO-086 (reply format), F-CACHE-077 (hash order).

<a id="f-ns-033"></a>

#### F-NS-033 · Readdir across dirfrags: frag selection and adjustment

*Lets a listing continue correctly when the directory was split or merged between two READDIR requests.*

If the requested frag is no longer a leaf of the fragtree: with CEPH_READDIR_REPLY_BITFLAGS the MDS continues at the leaf containing offset_hash when the old frag contains it, else at the leaf containing fg.value(); without the flag it restarts at the leaf containing fg.value(), keeping an offset_hash already computed from the sent name. try_open_auth_dirfrag forwards to the dirfrag auth when the frag is not local. The reply DirStat.frag may differ from the requested frag and clients adopt it; FRAG_END drives the client to the next frag.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Frag selection for a READDIR without CEPH_READDIR_REPLY_BITFLAGS whose frag is no longer a leaf: the resume name is cleared and the scan restarts at the leaf containing fg.value() (the case the notes call legacy, where the last returned name can repeat). The 7.3 kernel (ceph_readdir, __get_snap_name) and current libcephfs (Client::readdir_r_cb) always set CEPH_READDIR_REPLY_BITFLAGS. Requests without it come from kernels before v4.7 and Ceph clients before 10.2.2. Evidence: `src/mds/Server.cc:5149-5162 (Server::handle_client_readdir)`, `linux/fs/ceph/dir.c:450-451 (ceph_readdir)`, `linux/fs/ceph/export.c:502-503 (__get_snap_name)`, `src/client/Client.cc:10116 (Client::readdir_r_cb)`. Also: linux-v4.7/fs/ceph/dir.c:409 (flag sent; linux-v4.5/fs/ceph/dir.c has no CEPH_READDIR_REPLY_BITFLAGS); v10.2.2:src/client/Client.cc:7012 (flag sent; absent at v10.2.1).
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.9 READDIR (Server::handle_client_readdir, Server::_fina...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#39-readdir-serverhandle_client_readdir-server_finalize_readdir) · [lookup-readdir-open-and-file-locks.md › 7.5 READDIR](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#75-readdir) · [dirfrags-and-fragmentation.md › 1. Purpose and responsibilities](../cpp-map/04-cache/dirfrags-and-fragmentation.md#1-purpose-and-responsibilities)
- **Code:** `src/mds/Server.cc:5134-5160 (Server::handle_client_readdir)`, `src/mds/Server.cc:4231 (Server::try_open_auth_dirfrag)`, `src/mds/Server.cc:5063-5069 (Server::_finalize_readdir)`, `linux/fs/ceph/dir.c:317 (ceph_readdir)`, `src/client/Client.cc:9901 (Client::_readdir_next_frag)`
- **Notes:** Dirfrag split/merge itself is catalogued by category 05. The kernel sends READDIR with r_direct_hash = ceph_frag_value(frag) to its idea of the dirfrag auth; libcephfs leaves both budgets 0. A reply covers one leaf frag and reports the served frag in DirStat; flags FRAG_END when the frag is exhausted plus FRAG_COMPLETE if it also started at the frag start, and clients then move to frag.next() until the rightmost frag. Readdir positions (libcephfs telldir/seekdir offsets, which Ganesha uses as cookies) stay valid across splits and merges; the MDS accepts any frag value and maps it to a current leaf. Replies are bounded by max_entries and max_bytes (default 512 KiB + max_xattr_size). Without REPLY_BITFLAGS (legacy), when the requested frag is no longer a leaf the scan starts at the first key whose hash is >= the hash of the resume name, so the last returned name can repeat. See also F-NS-032 (paging), F-CACHE-077 (hash order), F-CACHE-078.

<a id="f-ns-034"></a>

#### F-NS-034 · Readdir cap-acquisition throttle

*Delays READDIR for a session that holds very many caps and acquired many recently.*

If session caps > mds_max_caps_per_client * mds_session_max_caps_throttle_ratio and the session's decaying cap_acquisition counter (half-life mds_session_cap_acquisition_decay_rate, fed by every readdir reply) is >= mds_session_cap_acquisition_throttle, the request is re-run from the timer after mds_cap_acquisition_throttle_retry_request_timeout; the perf counter cap_acquisition_throttle is incremented. The check runs after path rdlocks are taken and before the directory locks, and the same throttle applies to READDIR_SNAPDIFF. It never returns an error.

- **Class:** POSIX `operational` · exposure `config`, `metrics` · relied on by kclient, libcephfs, admin, ceph-fuse, ganesha
- **Gates:** mds_max_caps_per_client (1M); mds_session_max_caps_throttle_ratio (1.1); mds_session_cap_acquisition_throttle (100000); mds_session_cap_acquisition_decay_rate (30 s); mds_cap_acquisition_throttle_retry_request_timeout (0.5 s)
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.9 READDIR (Server::handle_client_readdir, Server::_fina...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#39-readdir-serverhandle_client_readdir-server_finalize_readdir) · [request-lifecycle.md › 3.20 Throttles and admission control](../cpp-map/03-request-path/request-lifecycle.md#320-throttles-and-admission-control)
- **Code:** `src/mds/Server.cc:5109-5121 (Server::handle_client_readdir)`
- **Notes:** Path rdlocks (and directory rdlocks on a retry after a fetch) stay held during the 0.5 s delay, so creates/unlinks/renames needing them wait (lookup page 8). The request records the op event "cap_acquisition_throttle". See also F-CAPS-032 (recall).

### Creating names: create, mknod, mkdir, symlink

Ops that add a new inode and its primary dentry, the initialization of the new inode, inode-number allocation, layout choice at create and the parent-directory updates shared with removal ops.

<a id="f-ns-035"></a>

#### F-NS-035 · CREATE (open with O_CREAT, optionally O_EXCL)

*Atomically creates and opens a regular file, or opens the existing file when O_EXCL is not set.*

Error order after the read-only and full gates: bad flags -EINVAL -> create gates (F-NS-019; -EEXIST only with O_EXCL) -> charmap directory and session without CEPHFS_FEATURE_CHARMAP -EPERM -> alternate_name over mds_alternate_name_max -ENAMETOOLONG -> invalid layout -EINVAL -> check_access(MAY_WRITE[|MAY_SET_VXATTR]) on the parent -EACCES -> dirfrag full -ENOSPC -> mds_dir_max_entries -ENOSPC. An existing name without O_EXCL is served by handle_client_open (no created ino in the reply). The new inode gets mode | S_IFREG and the inherited or requested layout (F-NS-042); the creator receives caps for its open mode, becomes loner, and for write opens gets client_ranges [0, stripe_unit]. One EUpdate "openc" journals the client request, the ino allocation, the parent updates (+1 file), the primary dentry and the opened ino; the reply carries the created ino (F-PROTO-087) and early reply is allowed unless the ino came from the InoTable.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The -EPERM step for CREATE into a charmap directory by a session without CEPHFS_FEATURE_CHARMAP (bit 22). Server::can_handle_charmap refuses only sessions without CephFS bit 22. The 7.3 kernel sets bit 22 under its own name CEPHFS_FEATURE_SUBVOLUME_METRICS and current libcephfs sets CEPHFS_FEATURE_CHARMAP, so no baseline client gets this -EPERM. Kernels before v7.1, both CentOS Stream samples and Ceph clients before 19.2.3/20.2.0 lack the bit. Evidence: `src/mds/Server.cc:4924 (Server::handle_client_openc)`, `src/mds/Server.cc:4876-4892 (Server::can_handle_charmap)`, `linux/fs/ceph/mds_client.h:40 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.h:59 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:79 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs8-kernel/fs/ceph/mds_client.h:38-47`, `cs9-kernel/fs/ceph/mds_client.h:43-57 (no bit 22)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:859 (bit 22: first kernel v7.1, first Ceph 19.2.3/20.2.0)`.
- **Map:** [namespace-mutations.md › 3.7.1 CREATE (src/mds/Server.cc:4895 (Server::handle_clie...](../cpp-map/03-request-path/namespace-mutations.md#371-create-srcmdsservercc4895-serverhandle_client_openc) · [posix-semantics-contract.md › 3.2 Main table](../cpp-map/03-request-path/posix-semantics-contract.md#32-main-table) · [posix-semantics-contract.md › 3.3.1 open, creat and the open flags](../cpp-map/03-request-path/posix-semantics-contract.md#331-open-creat-and-the-open-flags)
- **Code:** `src/mds/Server.cc:4895 (Server::handle_client_openc)`, `src/mds/Server.cc:4829 (C_MDS_openc_finish)`, `linux/fs/ceph/file.c:795 (ceph_atomic_open)`, `src/client/Client.cc:16137 (Client::_create)`
- **Notes:** The kernel sends CREATE only from ->atomic_open; vfs_create callers (knfsd, mknod(2) of a regular file) send MKNOD. A kclient async create (CEPH_MDS_FLAG_ASYNC, CEPH_O_EXCL, head.ino = delegated ino) needs Dc on the directory, which the C++ MDS at this commit never grants, so every create is synchronous (namespace-mutations 8; async dirops are catalogued by category 03). MDBalancer::maybe_fragment is called after each create.

<a id="f-ns-036"></a>

#### F-NS-036 · MKNOD (regular file, FIFO, device, socket)

*Creates a non-directory node of any type with rdev, without opening it.*

Mode gets S_IFREG when the type bits are 0; no other type validation is done, so any S_IFMT value is stored (the VFS filters types for the kernel; libcephfs _mknod does not). Error order: read-only gate (no full gate) -> create gates -> -EACCES -> -ENOSPC (dirfrag) -> -ENOSPC (dir entries) -> -ENAMETOOLONG (alternate_name) -> -EINVAL (layout, validated for all types); there is no charmap check. rdev = args.mknod.rdev; a regular file gets RDWR caps with wanted 0, filelock/authlock/xattrlock forced to LOCK_EXCL and a writable range [0, stripe_unit]; other types get the cap created by the reply. EUpdate "mknod" journals client request, ino allocation, parent update and primary dentry.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [namespace-mutations.md › 3.7.2 MKNOD (src/mds/Server.cc:7582 (Server::handle_clien...](../cpp-map/03-request-path/namespace-mutations.md#372-mknod-srcmdsservercc7582-serverhandle_client_mknod) · [posix-semantics-contract.md › 3.3.2 mknod, mkdir, symlink, readlink](../cpp-map/03-request-path/posix-semantics-contract.md#332-mknod-mkdir-symlink-readlink)
- **Code:** `src/mds/Server.cc:7582 (Server::handle_client_mknod)`, `src/mds/Server.cc:7532 (C_MDS_mknod_finish)`, `linux/fs/ceph/dir.c:924 (ceph_mknod)`
- **Notes:** MKNOD of S_IFDIR through libcephfs creates a directory-type inode without a dirfrag (posix-semantics-contract 8, derived from code, not tested). Missing charmap check is a known gap (namespace-mutations 8).

<a id="f-ns-037"></a>

#### F-NS-037 · MKDIR

*Creates a directory inode with its first, empty and complete dirfrag.*

Error order: read-only gate (no full gate) -> create gates -> -EACCES -> -ENOSPC -> -ENOSPC -> -EPERM (charmap) -> -ENAMETOOLONG. Mode is (mode & ~S_IFMT) | S_IFDIR; group and S_ISGID are inherited from a setgid parent (F-NS-039); the new directory inherits the parent charmap and gets dir hash mds_default_dir_hash. The first dirfrag frag_t() is created in memory, marked complete and new (written at segment expiry); stored nlink stays 1 and the parent's nlink is never changed (F-NS-085). The creator gets exclusive caps on the new directory as its only cap holder. Inside .snap clients send MKSNAP instead (category 08).

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The -EPERM (charmap) step for MKDIR by a session without CEPHFS_FEATURE_CHARMAP (bit 22). Server::can_handle_charmap refuses only sessions without CephFS bit 22. The 7.3 kernel sets bit 22 under its own name CEPHFS_FEATURE_SUBVOLUME_METRICS and current libcephfs sets CEPHFS_FEATURE_CHARMAP, so no baseline client gets this -EPERM. Kernels before v7.1, both CentOS Stream samples and Ceph clients before 19.2.3/20.2.0 lack the bit. Evidence: `src/mds/Server.cc:7711 (Server::handle_client_mkdir)`, `src/mds/Server.cc:4876-4892 (Server::can_handle_charmap)`, `linux/fs/ceph/mds_client.h:40 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.h:59 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:79 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs8-kernel/fs/ceph/mds_client.h:38-47`, `cs9-kernel/fs/ceph/mds_client.h:43-57 (no bit 22)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:859 (bit 22: first kernel v7.1, first Ceph 19.2.3/20.2.0)`.
- **Map:** [namespace-mutations.md › 3.7.3 MKDIR (src/mds/Server.cc:7688 (Server::handle_clien...](../cpp-map/03-request-path/namespace-mutations.md#373-mkdir-srcmdsservercc7688-serverhandle_client_mkdir) · [posix-semantics-contract.md › 3.3.2 mknod, mkdir, symlink, readlink](../cpp-map/03-request-path/posix-semantics-contract.md#332-mknod-mkdir-symlink-readlink)
- **Code:** `src/mds/Server.cc:7688 (Server::handle_client_mkdir)`, `src/mds/Server.cc:7532 (C_MDS_mknod_finish)`, `linux/fs/ceph/dir.c:1109 (ceph_mkdir)`
- **Notes:** The commit also evaluates ephemeral random pinning for the new directory (category 07).

<a id="f-ns-038"></a>

#### F-NS-038 · SYMLINK (and readlink without an MDS op)

*Creates a symbolic link whose target is stored in the inode; reading it never contacts the MDS.*

Error order: read-only and full gates -> create gates -> -EACCES -> -ENOSPC -> -ENOSPC -> -EPERM (charmap) -> -ENAMETOOLONG (alternate_name). Mode is S_IFLNK|0777, symlink = request path2 (client-encrypted base64 text for fscrypt dirs), size = rbytes = target length; the MDS imposes no target length limit (the VFS copies it with a PATH_MAX limit). No explicit cap issue; the reply's InodeStat creates the cap. EUpdate "symlink". There is no readlink op: the target travels in every InodeStat and clients serve readlink from cache; the target never changes after creation.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The -EPERM (charmap) step for SYMLINK by a session without CEPHFS_FEATURE_CHARMAP (bit 22). Server::can_handle_charmap refuses only sessions without CephFS bit 22. The 7.3 kernel sets bit 22 under its own name CEPHFS_FEATURE_SUBVOLUME_METRICS and current libcephfs sets CEPHFS_FEATURE_CHARMAP, so no baseline client gets this -EPERM. Kernels before v7.1, both CentOS Stream samples and Ceph clients before 19.2.3/20.2.0 lack the bit. Evidence: `src/mds/Server.cc:7813 (Server::handle_client_symlink)`, `src/mds/Server.cc:4876-4892 (Server::can_handle_charmap)`, `linux/fs/ceph/mds_client.h:40 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.h:59 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:79 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs8-kernel/fs/ceph/mds_client.h:38-47`, `cs9-kernel/fs/ceph/mds_client.h:43-57 (no bit 22)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:859 (bit 22: first kernel v7.1, first Ceph 19.2.3/20.2.0)`.
- **Map:** [namespace-mutations.md › 3.7.4 SYMLINK (src/mds/Server.cc:7792 (Server::handle_cli...](../cpp-map/03-request-path/namespace-mutations.md#374-symlink-srcmdsservercc7792-serverhandle_client_symlink) · [posix-semantics-contract.md › 3.3.2 mknod, mkdir, symlink, readlink](../cpp-map/03-request-path/posix-semantics-contract.md#332-mknod-mkdir-symlink-readlink)
- **Code:** `src/mds/Server.cc:7792 (Server::handle_client_symlink)`, `linux/fs/ceph/inode.c:1316-1319 (ceph_fill_inode)`, `src/client/Client.cc:8377 (Client::_readlink)`
- **Notes:** Clients compute no POSIX ACLs for symlinks. Symlinks are resolved by clients; the MDS never follows them during traversal.

<a id="f-ns-039"></a>

#### F-NS-039 · New inode initialization

*Fills in owner, group, mode, timestamps and initial xattrs of every inode created by CREATE, MKNOD, MKDIR or SYMLINK.*

Server::prepare_new_inode sets nlink = 1, uid = head.owner_uid, gid = the parent's gid when the parent has S_ISGID (and S_ISGID on a new subdirectory) else head.owner_gid (request heads older than v3 use the caller uid/gid), atime = mtime = ctime = btime = op stamp (client clock), change_attr = 0, truncate_seq 1 / truncate_size -1, layout for non-directories, dl_dir_hash for directories, fscrypt_auth/fscrypt_file from the request, and inline data off unless enabled in the MDSMap and supported by the client. If the request has a data payload it is decoded as the initial xattr map (POSIX ACLs and security labels computed by the client) and stored verbatim, without name or size validation. The MDS applies no umask and does not strip S_ISGID from a new file whose creator is not in the group; the kernel VFS and ACL code do that before sending.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients, deprecated-admin): (1) Owner uid/gid taken from the caller uid/gid for request heads older than v3 and for the legacy 88-byte head; the fallback sits in the head decoders, not in prepare_new_inode. (2) New inodes left with inline data enabled when the MDSMap inline_data flag is set and the client has CEPH_FEATURE_MDS_INLINE_DATA. The 7.3 kernel sends MClientRequest v6 with head v3 carrying owner_uid/owner_gid towards an MDS with FS_BTIME and CephFS bits 18 and 20, and current libcephfs sends head v3 with the owner set for every new-inode op. Heads below v3 come from kernels before v6.7, the CentOS Stream 8 sample and Ceph clients before 17.2.8/18.2.1/19.2.0. The inline_data flag that keeps inline data on new inodes is deprecated by the monitor and set by no mgr module, cephadm or Rook. Evidence: `src/common/ceph_fs.cc:85-92 (decode(ceph_mds_request_head&))`, `src/messages/MClientRequest.h:271-284 (MClientRequest::decode_payload)`, `linux/fs/ceph/mds_client.c:3160-3169 (mds_supported_head_version)`, `linux/fs/ceph/mds_client.c:3367-3373 (create_request_message)`, `src/messages/MClientRequest.h:321-327 (MClientRequest::encode_payload)`, `src/client/Client.cc:16041 (Client::_mknod, _create, _mkdir, _symlink: set_inode_owner_uid_gid)`, `src/client/Client.cc:16175 (Client::_mknod, _create, _mkdir, _symlink: set_inode_owner_uid_gid)`, `src/client/Client.cc:16276 (Client::_mknod, _create, _mkdir, _symlink: set_inode_owner_uid_gid)`, `src/client/Client.cc:16454 (Client::_mknod, _create, _mkdir, _symlink: set_inode_owner_uid_gid)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:877 (head version per client release)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mon/FSCommands.cc:471-478 (inline_data: --yes-i-really-really-mean-it, 'deprecated')`, `src/mds/MDSMap.cc:616-622 (FS_INLINE_DATA_DEPRECATED)`, `doc/cephfs/experimental-features.rst:24-28`. Also: searched src/pybind/mgr, src/cephadm and rook (*.go) for inline_data/uninline: the only Rook hits are RGW zone placement.
- **Map:** [namespace-mutations.md › 3.5 New-inode initialization (prepare_new_inode)](../cpp-map/03-request-path/namespace-mutations.md#35-new-inode-initialization-prepare_new_inode) · [posix-semantics-contract.md › 3.3.1 open, creat and the open flags](../cpp-map/03-request-path/posix-semantics-contract.md#331-open-creat-and-the-open-flags) · [posix-semantics-contract.md › 4. Data structures and invariants](../cpp-map/03-request-path/posix-semantics-contract.md#4-data-structures-and-invariants)
- **Code:** `src/mds/Server.cc:3637-3765 (Server::prepare_new_inode)`, `src/mds/Server.cc:3722-3735 (Server::prepare_new_inode)`, `linux/fs/ceph/acl.c:165 (ceph_pre_init_acls)`
- **Notes:** No equivalent S_ISGID stripping was found in libcephfs create paths (code search, posix-semantics-contract 3.3.1). Initial xattrs are not counted against max_xattr_size (attributes 3.6.4).

<a id="f-ns-040"></a>

#### F-NS-040 · Inode number allocation (session preallocation and InoTable)

*Create-type ops take their inode number from the session preallocation or the rank InoTable.*

See F-STORE-070 (allocation in create paths) and F-STORE-071 (session preallocation and refill). Request-side consequences: CREATE, MKNOD, MKDIR and SYMLINK call prepare_new_inode; an inode number taken from the InoTable (not from the session preallocation) forbids an early reply because the number is durable only with this event, while preallocated numbers were journaled earlier and may be exposed in an unsafe reply.

- **Class:** POSIX `supporting` · exposure `on-disk`, `config` · relied on by internal, kclient, tools
- **Gates:** mds_client_prealloc_inos (default 1000)
- **Map:** [namespace-mutations.md › 3.4 Inode-number allocation](../cpp-map/03-request-path/namespace-mutations.md#34-inode-number-allocation) · [mds-tables.md › 1. Purpose and responsibilities](../cpp-map/07-storage-formats/mds-tables.md#1-purpose-and-responsibilities)
- **Code:** `src/mds/Server.cc:3637 (Server::prepare_new_inode)`, `src/mds/Server.cc:3767-3802 (Server::journal_allocated_inos)`, `src/mds/SessionMap.h:179 (Session::take_ino)`, `src/mds/InoTable.cc:46 (InoTable::project_alloc_id)`
- **Notes:** Clients see inode numbers only as opaque 64-bit values (also in NFS handles and in kernel delegated ranges above CEPH_INO_SYSTEM_BASE). See also F-NS-041 (client-chosen head.ino).

<a id="f-ns-041"></a>

#### F-NS-041 · Client-specified inode number (head.ino) on create

*Recreates an object with exactly the inode number the client already received, on replay and for async creates.*

Replayed unsafe creates carry head.ino = the ino returned in the unsafe reply; kernel async creates carry a delegated ino. CREATE/MKNOD/MKDIR/SYMLINK pass head.ino to prepare_new_inode, which takes it from the session's prealloc set when present, otherwise hands it to InoTable::project_alloc_id; only when the ino is in replay_taken_inos does the MDS pick another one and log "client.N specified ino X but mds.R allocated Y" at error level. During up:reconnect the carried ino is recorded with add_replay_ino_alloc so cache rejoin does not try to open it. The kernel reports "inode number mismatch" when it gets a different ino for an async create.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [request-lifecycle.md › 3.18 Clientreplay of client requests](../cpp-map/03-request-path/request-lifecycle.md#318-clientreplay-of-client-requests) · [namespace-mutations.md › 3.4 Inode-number allocation](../cpp-map/03-request-path/namespace-mutations.md#34-inode-number-allocation) · [namespace-mutations.md › 7. Externally visible contract](../cpp-map/03-request-path/namespace-mutations.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:3637-3686 (Server::prepare_new_inode)`, `src/mds/SessionMap.h:179-188 (Session::take_ino)`, `linux/fs/ceph/mds_client.c:3540-3557 (__prepare_send_request)`, `src/mds/InoTable.cc:46 (InoTable::project_alloc_id)`
- **Notes:** The ino is not validated: one neither preallocated to the session nor free in the InoTable reaches a strict interval_set erase that asserts, and take_ino asserts when two in-flight requests carry the same prealloc ino; both are reachable from client input (request-lifecycle 8 item 27, namespace-mutations 8; derived from code, not tested). A kernel async create delegated by another rank and forwarded here, or a reused delegated ino, would also reach them (leases page G4), but unmodified clients send neither at this commit: the C++ MDS never grants the caps the kernel needs for async create (F-CAPS-048, leases page G1) and libcephfs discards delegated inos. A crafted head.ino still reaches the asserts. libcephfs sets head.ino on replays of requests that got an unsafe reply (src/client/Client.cc:2853-2855). See also F-PROTO-074 (wire field), F-CAPS-054 (delegation), F-STORE-070 (allocation).

<a id="f-ns-042"></a>

#### F-NS-042 · File layout selection and validation at create

*Chooses the striping layout and data pool of a new file from the nearest ancestor layout, the default, or the request.*

With want_layout the traversal rdlocks policylock from the parent upward until an inode with a layout and stores it in dir_layout; CREATE uses it or MDCache::default_file_layout (4 MiB stripe unit and object size, stripe count 1, first data pool), and stripe_unit, stripe_count, object_size and (with CEPH_FEATURE_CREATEPOOLID and pool >= 0) pool from args.open override it. When the client names a pool the MDS first waits until its MDSMap epoch reaches the client's; a layout different from the inherited one adds MAY_SET_VXATTR (MDS-cap p flag) to the access check; is_valid_layout requires non-zero multiples of 64 KiB, object_size % stripe_unit == 0, stripe_count > 0 and a data pool of this file system, else -EINVAL. MKNOD validates the inherited layout for every node type. A file in a pool other than the default data pool also gets the default pool recorded as an old pool so its backtrace is written there too.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, osd, ceph-fuse, ganesha
- **Gates:** CEPH_FEATURE_CREATEPOOLID; MDS auth cap flag p (MAY_SET_VXATTR)
- **Map:** [namespace-mutations.md › 3.7.1 CREATE (src/mds/Server.cc:4895 (Server::handle_clie...](../cpp-map/03-request-path/namespace-mutations.md#371-create-srcmdsservercc4895-serverhandle_client_openc) · [attributes-and-xattrs.md › 3.8.3 Layout vxattrs](../cpp-map/03-request-path/attributes-and-xattrs.md#383-layout-vxattrs)
- **Code:** `src/mds/Server.cc:4942-4974 (Server::handle_client_openc)`, `src/mds/Server.cc:4862 (Server::is_valid_layout)`, `src/mds/Locker.cc:157 (Locker::try_rdlock_snap_layout)`
- **Notes:** See also F-CACHE-007 (layout field and inheritance).

<a id="f-ns-043"></a>

#### F-NS-043 · Parent directory updates on link and unlink

*Updates the parent directory's mtime, ctime, change_attr and entry counts whenever a name is added or removed.*

MDCache::predirty_journal_parents with PREDIRTY_DIR on the parent dirfrag assigns fragstat.mtime = op stamp, increments fragstat.change_attr and adds +-1 to nfiles or nsubdirs; the directory inode then receives dirstat.add_delta, which adds the count difference but keeps the maximum mtime and change_attr, so the inode mtime = ctime and change_attr move only forward. The inode part is applied in the same mutation only when the directory inode is auth here and its nestlock and versionlock can be wrlocked; otherwise it is deferred to the next scatter-gather of the directory filelock. CREATE, MKNOD, MKDIR, SYMLINK, LINK, UNLINK, RMDIR and RENAME (both parents unless the source is a stray dir) do this; SETATTR, xattr ops and O_TRUNC dirty the parent dirfrag but do not touch its mtime.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [namespace-mutations.md › 3.6 Parent and ancestor updates: predirty_journal_parents](../cpp-map/03-request-path/namespace-mutations.md#36-parent-and-ancestor-updates-predirty_journal_parents) · [posix-semantics-contract.md › 3.4 Timestamps and change_attr](../cpp-map/03-request-path/posix-semantics-contract.md#34-timestamps-and-change_attr) · [rename.md › 7.2 libcephfs / ceph-fuse / NFS-Ganesha](../cpp-map/03-request-path/rename.md#72-libcephfs--ceph-fuse--nfs-ganesha)
- **Code:** `src/mds/MDCache.cc:2224 (MDCache::predirty_journal_parents)`, `src/mds/MDCache.cc:2286-2306 (MDCache::predirty_journal_parents)`, `src/include/cephfs/types.h:91-104 (frag_info_t::add_delta)`
- **Notes:** Consequences (code reading): a create from a client whose clock is behind leaves the parent mtime/ctime unchanged; in a directory with several dirfrags a create or unlink in a dirfrag whose counter is behind does not bump the directory change_attr, which clients expose as the NFS change attribute (namespace-mutations 8). Recursive stats (rbytes, rfiles, rctime) propagation up the tree is catalogued by category 08; dirstat nsubdirs is what clients turn into directory link counts (F-NS-085). See also F-SNAP-033 (dirstat).

### Removing and relinking names: link, unlink, rmdir, rename

Hard links, unlink of primary and remote dentries, rmdir, rename and the guarantee that unlinked open files stay usable; the single-rank behaviour of each op.

<a id="f-ns-044"></a>

#### F-NS-044 · LINK (hard link)

*Adds a remote dentry for an existing non-directory inode and increments its link count.*

The source is a bare inode number (filepath2 depth 0: libcephfs ll_link and kclient for disconnected dentries; target must be cached with a parent dentry, else -EINVAL) or a second depth-1 path (two-path locking). Error order: read-only and full gates -> -EINVAL or path errors for the target -> create gates for the new name (-EEXIST) -> -EPERM (charmap) -> -ENAMETOOLONG -> -EINVAL (target is a directory) -> -ENOENT (target nlink 0) -> -EACCES (target, then directory) -> -ENOSPC -> -ENOSPC -> -EXDEV (target and new parent in different subvolumes). With a local target, _link_local xlocks the target snaplock and linklock, projects nlink += 1, ctime = op stamp, change_attr += 1, journals EUpdate "link_local" with the remote dentry, parent update (+1 file) and the target inode; early reply allowed. A target on another rank uses a two-phase commit (F-NS-054).

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The -EPERM (charmap) step for LINK into a charmap directory by a session without CEPHFS_FEATURE_CHARMAP (bit 22). Server::can_handle_charmap refuses only sessions without CephFS bit 22. The 7.3 kernel sets bit 22 under its own name CEPHFS_FEATURE_SUBVOLUME_METRICS and current libcephfs sets CEPHFS_FEATURE_CHARMAP, so no baseline client gets this -EPERM. Kernels before v7.1, both CentOS Stream samples and Ceph clients before 19.2.3/20.2.0 lack the bit. Evidence: `src/mds/Server.cc:7919 (Server::handle_client_link)`, `src/mds/Server.cc:4876-4892 (Server::can_handle_charmap)`, `linux/fs/ceph/mds_client.h:40 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.h:59 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:79 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs8-kernel/fs/ceph/mds_client.h:38-47`, `cs9-kernel/fs/ceph/mds_client.h:43-57 (no bit 22)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:859 (bit 22: first kernel v7.1, first Ceph 19.2.3/20.2.0)`.
- **Map:** [namespace-mutations.md › 3.7.5 LINK, local target (src/mds/Server.cc:7865 (Server:...](../cpp-map/03-request-path/namespace-mutations.md#375-link-local-target-srcmdsservercc7865-serverhandle_client_link-srcmdsservercc8015-server_link_local) · [posix-semantics-contract.md › 3.3.3 link, unlink, rmdir](../cpp-map/03-request-path/posix-semantics-contract.md#333-link-unlink-rmdir)
- **Code:** `src/mds/Server.cc:7865 (Server::handle_client_link)`, `src/mds/Server.cc:8015 (Server::_link_local)`, `linux/fs/ceph/dir.c:1200 (ceph_link)`, `src/client/Client.cc:16836 (Client::_link)`
- **Notes:** There is no EMLINK check (F-NS-088). A directory target gets EPERM from clients but -EINVAL from the MDS (visible with client_permissions=false). The kernel does not check quotas or quota realms on link. Hard-linked inodes outside subvolumes get a "global" snaprealm on first link (category 08); the EXDEV subvolume rule is also catalogued there.

<a id="f-ns-045"></a>

#### F-NS-045 · UNLINK of a primary link

*Removes a name whose dentry holds the inode, decrementing nlink and moving the inode to a stray directory.*

Error order: read-only gate -> remove gates (-ENOTEMPTY for "."/".."; -ENOENT absent) -> -EISDIR for a directory -> check_access(MAY_WRITE) on the parent only -> -ENOSPC when the stray dirfrag is full. prepare_stray_dentry picks ~mds<rank>/stray<idx>/<ino hex> on the local rank (index rotating per log segment); _unlink_local projects the inode into the stray dentry with ctime = op stamp, change_attr += 1, nlink -= 1 (STATE_ORPHAN at 0) and stray_prior_path = old path (used for later MDS-cap checks), updates the parent (-1) and the stray dir (+1), and journals EUpdate "unlink_local" with the null dentry; the reply traces only the dentry. After commit the stray is evaluated for purge (F-NS-048).

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [namespace-mutations.md › 3.7.7 UNLINK of a primary link (src/mds/Server.cc:8572 (S...](../cpp-map/03-request-path/namespace-mutations.md#377-unlink-of-a-primary-link-srcmdsservercc8572-serverhandle_client_unlink-srcmdsservercc8738-server_unlink_local) · [posix-semantics-contract.md › 3.3.3 link, unlink, rmdir](../cpp-map/03-request-path/posix-semantics-contract.md#333-link-unlink-rmdir)
- **Code:** `src/mds/Server.cc:8572 (Server::handle_client_unlink)`, `src/mds/Server.cc:8738 (Server::_unlink_local)`, `src/mds/Server.cc:3595 (Server::prepare_stray_dentry)`, `linux/fs/ceph/dir.c:1348 (ceph_unlink)`
- **Notes:** Kernel async unlink (needs Dr on the directory) is never possible against the C++ MDS at this commit (namespace-mutations 8, category 03). The MDS does not check the sticky bit; clients do. Snapshot copy-on-write of the removed dentry and past-parent snaprealm records are catalogued by category 08. See also F-BG-002 (stray side).

<a id="f-ns-046"></a>

#### F-NS-046 · UNLINK of a remote (hard-link) dentry

*Removes a hard link that is not the inode's primary dentry.*

No stray dentry is used. With the target inode auth locally, _unlink_local journals the parent update (-1, fragstat only), the target with nlink - 1, ctime, change_attr and stray_prior_path, and a null dentry. A target on another rank uses the link two-phase commit with inc=false (F-NS-054). When the last remaining link of an inode whose primary already sits in a stray directory disappears, the stray is re-evaluated when the inode's extra references drop.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [namespace-mutations.md › 3.7.8 UNLINK of a remote dentry](../cpp-map/03-request-path/namespace-mutations.md#378-unlink-of-a-remote-dentry)
- **Code:** `src/mds/Server.cc:8738 (Server::_unlink_local)`, `src/mds/Server.cc:8106 (Server::_link_remote)`

<a id="f-ns-047"></a>

#### F-NS-047 · RMDIR with emptiness checks

*Removes an empty directory; refuses directories with entries or with snapshots of their own.*

RMDIR is handle_client_unlink in rmdir mode. Error order: read-only gate -> remove gates -> -ENOTEMPTY from the unlocked check (_dir_is_nonempty_unlocked: own snaprealm has snapshots, or an auth dirfrag has projected entries) or -ENOTDIR -> -EACCES (parent) -> -ENOSPC (stray) -> -ENOTEMPTY from the locked check (_dir_is_nonempty: rdlock of the directory filelock gathers every dirfrag's fragstat; non-empty if any cached frag has entries or the accounted sum differs from dirstat). Snapshots of ancestors do not block rmdir; the directory then moves to a stray dir like a primary unlink (-1 subdirectory on the parent). Ranks holding subtree-root dirfrags of the directory act as witnesses (F-NS-055).

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [namespace-mutations.md › 3.7.9 RMDIR: emptiness, snapshots and witnesses](../cpp-map/03-request-path/namespace-mutations.md#379-rmdir-emptiness-snapshots-and-witnesses) · [posix-semantics-contract.md › 3.3.3 link, unlink, rmdir](../cpp-map/03-request-path/posix-semantics-contract.md#333-link-unlink-rmdir)
- **Code:** `src/mds/Server.cc:8572 (Server::handle_client_unlink)`, `src/mds/Server.cc:9225 (Server::_dir_is_nonempty_unlocked)`, `src/mds/Server.cc:9262 (Server::_dir_is_nonempty)`
- **Notes:** The own-snapshot test exists only in the unlocked pre-check, which returns "maybe empty" first while the directory filelock is held by a lock cache, so an empty directory with its own snapshots would not be refused in that state (posix-semantics-contract 7.3, code reading). At this commit the state does not occur with unmodified clients, because no lock cache is ever created (F-CAPS-048, leases page G1), so the own-snapshot test always runs. libcephfs-based clients leave EISDIR/ENOTDIR/ENOTEMPTY to the MDS.

<a id="f-ns-048"></a>

#### F-NS-048 · Unlinked-but-open inodes stay usable until the last cap is released

*Keeps an inode whose last name was removed (by unlink, rmdir or rename-over) alive while any client holds caps on it.*

A removed primary link moves the inode into a stray dentry of the local rank instead of deleting it; a primary target replaced by RENAME is moved the same way (to the leader's stray dir). After commit MDCache::notify_stray -> StrayManager::eval_stray waits while the dentry is replicated, leased, the inode has caps, needs file recovery or holds references; only then is the inode handed to the purge queue, which deletes its data objects. Openers can keep reading and writing the inode meanwhile; if other remote links remain, the stray is relinked at one of them (F-NS-052).

- **Class:** POSIX `required` · exposure `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, osd
- **Map:** [namespace-mutations.md › 3.7.10 After unlink: stray evaluation, purge hand-off and...](../cpp-map/03-request-path/namespace-mutations.md#3710-after-unlink-stray-evaluation-purge-hand-off-and-hard-link-relink) · [rename.md › 3.5 Overwriting a target; the stray dentry](../cpp-map/03-request-path/rename.md#35-overwriting-a-target-the-stray-dentry) · [strays-and-purge.md › 1. Purpose and responsibilities](../cpp-map/11-background-services/strays-and-purge.md#1-purpose-and-responsibilities)
- **Code:** `src/mds/Server.cc:3595 (Server::prepare_stray_dentry)`, `src/mds/StrayManager.cc:447 (StrayManager::_eval_stray)`, `src/mds/Server.cc:9807-9811 (Server::_rename_finish)`
- **Notes:** Stray directories (10 per rank), their rotation and the purge queue are catalogued by category 09. Files with past-parent snapshots are only truncated, and directories with past-parent snapshots are not purged until those snapshots are deleted. See also F-BG-006 (purge eligibility), F-BG-002 (move to stray).

<a id="f-ns-049"></a>

#### F-NS-049 · RENAME of a file or directory (move, same-inode no-op)

*Atomically re-points a name from srcdir/srcname to destdir/destname within one file system.*

The request carries filepath = destination (dir ino + name), filepath2 = source, optional alternate_name and releases; it executes on the rank that is auth for the destination dentry. Both dentries are xlocked for the whole operation; srci's linklock and snaplock are xlocked. destdn == srcdn (also two spellings folding to one dentry in a charmap dir) and two names linking the same inode return 0 with no change (no ctime update). Effects: renamed inode ctime = op stamp and change_attr += 1 (rctime raised); both parents get the P-touch of F-NS-043 unless the source is a stray dir; a directory move updates nsubdirs and the subtree map. The reply traces the destination dir, destination name and the renamed inode; early reply is allowed when no peer journaled.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Map:** [rename.md › 1. Purpose and responsibilities](../cpp-map/03-request-path/rename.md#1-purpose-and-responsibilities) · [rename.md › 3.2 Intake, forwarding and the first lock batch](../cpp-map/03-request-path/rename.md#32-intake-forwarding-and-the-first-lock-batch) · [posix-semantics-contract.md › 3.3.4 rename](../cpp-map/03-request-path/posix-semantics-contract.md#334-rename)
- **Code:** `src/mds/Server.cc:9321 (Server::handle_client_rename)`, `src/mds/Server.cc:9932 (Server::_rename_prepare)`, `src/mds/Server.cc:10290 (Server::_rename_apply)`, `linux/fs/ceph/dir.c:1481 (ceph_rename)`
- **Notes:** mgr/volumes relies on atomic overwrite (.meta.tmp -> .meta), renames inside one subvolume, and moving a subvolume root between non-subvolume parents (rename 7.3). There is no flag field in RENAME; renameat2 flags are refused by clients (F-NS-089). The MDS does not check quota realms; clients return EXDEV across them.

<a id="f-ns-050"></a>

#### F-NS-050 · RENAME over an existing target

*Replaces an existing destination (file over file, empty directory over directory) in the same atomic step.*

The existing target oldin is resolved (opening a remote target if needed); its linklock and snaplock are xlocked and, for a directory, its filelock rdlocked to verify emptiness. oldin is projected with ctime = op stamp, change_attr += 1, stray_prior_path = destination path, nlink -= 1 (STATE_ORPHAN at 0); a primary target is re-homed into a stray dentry of the leader and evaluated for purge after the reply (F-NS-048); a remote target is decremented on its auth. An existing target dentry with a different alternate_name gives -EINVAL.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Map:** [rename.md › 3.5 Overwriting a target; the stray dentry](../cpp-map/03-request-path/rename.md#35-overwriting-a-target-the-stray-dentry) · [posix-semantics-contract.md › 3.2 Main table](../cpp-map/03-request-path/posix-semantics-contract.md#32-main-table)
- **Code:** `src/mds/Server.cc:9384-9413 (Server::handle_client_rename)`, `src/mds/Server.cc:10068-10094 (Server::_rename_prepare)`

<a id="f-ns-051"></a>

#### F-NS-051 · RENAME validation and error order

*Rejects invalid renames (directory loops, type mismatches, non-empty targets, bad inputs) in a fixed order.*

Order: read-only and full gates -> last component "." or ".." -EBUSY -> alternate_name too long -ENAMETOOLONG -> two-path checks (depth != 1 -EINVAL, snapshot path -EROFS, system parent -EROFS, destination parent unlinked -ENOENT, source missing -ENOENT) -> charmap -EPERM -> same dentry 0 -> directory moved below itself (CInode::is_projected_ancestor_of) -EINVAL -> MDS-originated request not stray->stray/linkmerge -EINVAL -> unlocked emptiness -ENOTEMPTY -> -EISDIR (dir target, non-dir source) -> -ENOTDIR -> same inode 0 -> alternate_name mismatch -EINVAL -> stray dirfrag full -ENOSPC -> MAY_WRITE on source dir then destination dir -EACCES -> destination -ENOSPC (dirfrag, entries) -> MAY_WRITE on the source inode -EACCES -> locked emptiness -ENOTEMPTY -> different subvolumes -EXDEV. Access and space checks are skipped once any witness acked. Renaming out of an unlinked source directory is not blocked.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The 'charmap -EPERM' step for RENAME into a charmap directory by a session without CEPHFS_FEATURE_CHARMAP (bit 22). Server::can_handle_charmap refuses only sessions without CephFS bit 22. The 7.3 kernel sets bit 22 under its own name CEPHFS_FEATURE_SUBVOLUME_METRICS and current libcephfs sets CEPHFS_FEATURE_CHARMAP, so no baseline client gets this -EPERM. Kernels before v7.1, both CentOS Stream samples and Ceph clients before 19.2.3/20.2.0 lack the bit. Evidence: `src/mds/Server.cc:9354 (Server::handle_client_rename)`, `src/mds/Server.cc:4876-4892 (Server::can_handle_charmap)`, `linux/fs/ceph/mds_client.h:40 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.h:59 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:79 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs8-kernel/fs/ceph/mds_client.h:38-47`, `cs9-kernel/fs/ceph/mds_client.h:43-57 (no bit 22)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:859 (bit 22: first kernel v7.1, first Ceph 19.2.3/20.2.0)`.
- **Map:** [rename.md › 3.3 Validation checks (evaluation order)](../cpp-map/03-request-path/rename.md#33-validation-checks-evaluation-order) · [posix-semantics-contract.md › 3.3.4 rename](../cpp-map/03-request-path/posix-semantics-contract.md#334-rename)
- **Code:** `src/mds/Server.cc:9321-9565 (Server::handle_client_rename)`, `src/mds/Server.cc:4073 (Server::rdlock_two_paths_xlock_destdn)`, `src/mds/CInode.cc:1078 (CInode::is_projected_ancestor_of)`
- **Notes:** The EXDEV subvolume rule is catalogued by category 08. Code reading suggests a possible self-conflict on snaplock for rename("a/b/c", "a") with c a directory sent by a client that does not pre-check ancestry (the VFS rejects it with ENOTEMPTY first); unverified (rename 8). The own-snapshot emptiness gap of RMDIR (F-NS-047) would apply to rename targets too; like that gap it needs a lock cache, which the C++ MDS never creates at this commit. See also F-BG-026 (stray and unlinked directory rules).

<a id="f-ns-052"></a>

#### F-NS-052 · MDS-originated RENAME (stray reintegration and migration)

*Accepts rename requests sent by MDS ranks to relink a stray inode at a remaining hard link or move it to another rank's stray dir.*

StrayManager::reintegrate_stray and migrate_stray (and shutdown_export_strays while a rank stops, targeting rank 0) send MClientRequest(CEPH_MDS_OP_RENAME) with filepath = destination and filepath2 = stray source to the destination auth, tracked in internal_client_requests. The receiving rank processes it without a session (no access checks, no early reply); only stray->stray (migration) or linkmerge (stray primary onto its own remote dentry) is accepted, else -EINVAL; the operation is silent (no nlink, ctime or directory mtime changes). Linkmerge turns the remote dentry into the primary link. The safe reply comes back through Server::handle_client_reply, which only erases the tracking entry; perf counters strays_reintegrated and strays_migrated count them.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by internal, mds-ranks
- **Map:** [rename.md › 3.16 MDS-internal renames: stray reintegration and migration](../cpp-map/03-request-path/rename.md#316-mds-internal-renames-stray-reintegration-and-migration) · [request-lifecycle.md › 3.16 Internal requests](../cpp-map/03-request-path/request-lifecycle.md#316-internal-requests) · [namespace-mutations.md › 3.7.10 After unlink: stray evaluation, purge hand-off and...](../cpp-map/03-request-path/namespace-mutations.md#3710-after-unlink-stray-evaluation-purge-hand-off-and-hard-link-relink)
- **Code:** `src/mds/StrayManager.cc:625-738 (StrayManager::_eval_stray_remote)`, `src/mds/Server.cc:2730 (Server::handle_client_reply)`, `src/mds/Server.cc:9374-9381 (Server::handle_client_rename)`
- **Notes:** The stray lifecycle that triggers these requests is catalogued by category 09. MDS-originated requests that would need forwarding are dropped, leaving the originating entry in internal_client_requests (request-lifecycle 8 item 15).

### Cross-rank execution of namespace operations

Which rank executes each op, and the MDS-to-MDS protocols (peer requests, two-phase commits, witnesses, inode migration) used when the objects of one op live on several ranks. All of it is MDS-to-MDS and not seen by clients except as forwards, cap import/export and latency.

<a id="f-ns-053"></a>

#### F-NS-053 · Which rank serves each op (auth requirement, forward or discover)

*Routes each request to a rank that may execute it: namespace mutations on the dentry auth, inode mutations and locks on the inode auth, reads on any rank holding the data.*

Mutations of a name traverse with MDS_TRAVERSE_WANT_AUTH and execute on the auth of the dirfrag owning the tail dentry (CREATE without O_EXCL of an existing file moves to the target inode auth); RENAME runs on the destination dentry auth and discovers the source as a replica; SETATTR, xattr ops, O_TRUNC/write opens, SETFILELOCK/GETFILELOCK and GETATTR with CEPH_STAT_RSTAT need the inode auth; READDIR is served by the dirfrag auth; GETATTR, LOOKUP, LOOKUPINO and read-only opens may be answered by replicas. On a cache miss in a non-auth dirfrag path_traverse forwards unless depth < the request's num_fwd, in which case it discovers the metadata instead; a not-open dirfrag of a non-auth inode is always discovered. Forwarding is always "client resends" (MClientRequestForward with num_fwd + 1); clients pick ranks from earlier DirStat auth/dist hints and abort with -EMULTIHOP if num_fwd does not increase.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Gates:** mds_forward_all_requests_to_auth (default false)
- **Map:** [request-lifecycle.md › 3.7 Path traversal and locking helpers](../cpp-map/03-request-path/request-lifecycle.md#37-path-traversal-and-locking-helpers) · [request-lifecycle.md › 3.13 Forwarding](../cpp-map/03-request-path/request-lifecycle.md#313-forwarding) · [namespace-mutations.md › 3.2 Entry, routing and idempotency](../cpp-map/03-request-path/namespace-mutations.md#32-entry-routing-and-idempotency) · [lookup-readdir-open-and-file-locks.md › 5.1 Client operations](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#51-client-operations)
- **Code:** `src/mds/MDCache.cc:8443 (MDCache::path_traverse)`, `src/mds/MDCache.cc:9970 (MDCache::request_forward)`, `linux/fs/ceph/mds_client.c:1381 (__choose_mds)`
- **Notes:** The MClientRequestForward message is catalogued by category 02; subtree authority and discover by categories 05 and 07. Completed-request entries are per rank, which is the code's stated reason for client resend instead of MDS relay (request-lifecycle 8 item 1). See also F-CACHE-044, F-PROTO-076, F-PROTO-075 (client-side rank selection).

<a id="f-ns-054"></a>

#### F-NS-054 · Peer requests and the LINK/UNLINK two-phase commit

*Lets the rank owning a dentry change the link count of an inode owned by another rank atomically, using MMDSPeerRequest messages.*

MMDSPeerRequest (v1) carries reqid, attempt, op, flags, lock type, object info, auth pins, src/dest paths, witnesses, op stamp, inode export blobs, stray and snaprealm blobs and alternate_name; ops are OP_AUTHPIN, OP_XLOCK/OP_WRLOCK and their un-ops, OP_LINKPREP/OP_UNLINKPREP, OP_RMDIRPREP, OP_RENAMEPREP, OP_RENAMENOTIFY, OP_DROPLOCKS, OP_FINISH (commit or abort) and OP_COMMITTED; a peer request is a separate MDRequestImpl on the peer, one op in flight at a time; replies for uncommitted leaders are parked while the peer is not active. For a remote link target the leader remote-auth-pins and remote-xlocks its snaplock and linklock, sends OP_LINKPREP/OP_UNLINKPREP; the peer projects nlink +-1 and ctime, journals EPeerUpdate "peer_link_prep" (with a link_rollback record) and acks; the leader journals EUpdate "link_remote"/"unlink_remote" with had_peers (no early reply), sends OP_FINISH before the client reply, the peer journals the commit and answers OP_COMMITTED, and the leader journals ECommitted. An abort makes the peer restore the old nlink, ctime and parent mtime from the rollback record.

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks
- **Map:** [request-lifecycle.md › 3.19 Peer-request framework](../cpp-map/03-request-path/request-lifecycle.md#319-peer-request-framework) · [namespace-mutations.md › 3.7.6 LINK and UNLINK with a remote target: two-phase commit](../cpp-map/03-request-path/namespace-mutations.md#376-link-and-unlink-with-a-remote-target-two-phase-commit) · [namespace-mutations.md › 5.2 MDS-to-MDS messages](../cpp-map/03-request-path/namespace-mutations.md#52-mds-to-mds-messages)
- **Code:** `src/messages/MMDSPeerRequest.h:23-234 (MMDSPeerRequest)`, `src/mds/Server.cc:2971 (Server::handle_peer_request)`, `src/mds/Server.cc:8106 (Server::_link_remote)`, `src/mds/Server.cc:8251 (Server::handle_peer_link_prep)`, `src/mds/Server.cc:8440 (Server::do_link_rollback)`
- **Notes:** MDS-to-MDS only: no client, monitor or OSD sees these messages or the EPeerUpdate/ECommitted events, so their format is free. Remote auth pins and remote locks are catalogued by category 03; recovery of in-doubt transactions through resolve by category 01. A peer applies its prepared state before the leader commits; the leader's xlocks hide it from clients.

<a id="f-ns-055"></a>

#### F-NS-055 · RMDIR witness protocol

*Makes every rank that holds subtree-root dirfrags of a removed directory agree to the removal.*

If any dirfrag of the directory is a subtree root, every rank in its replica list receives OP_RMDIRPREP with the trace path, the stray dentry replica blob (straybl), the prepared snaprealm and the op stamp. A witness traverses without locks, projects the dentry to null and the stray dentry to the directory, and journals EPeerUpdate "peer_rmdir" only if it is auth for a subtree-root dirfrag of the directory (otherwise it acks with NOTJOURNALED); the leader journals with had_peers and may early-reply only if no witness journaled. On commit a journaling witness drops the directory's remaining dirty state (clear_dirty_bits_for_stray); abort relinks the dentry.

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks
- **Map:** [namespace-mutations.md › 3.7.9 RMDIR: emptiness, snapshots and witnesses](../cpp-map/03-request-path/namespace-mutations.md#379-rmdir-emptiness-snapshots-and-witnesses) · [namespace-mutations.md › 4. Data structures and invariants](../cpp-map/03-request-path/namespace-mutations.md#4-data-structures-and-invariants)
- **Code:** `src/mds/Server.cc:8890 (Server::_rmdir_prepare_witness)`, `src/mds/Server.cc:8936 (Server::handle_peer_rmdir_prep)`, `src/mds/Server.cc:9124 (Server::do_rmdir_rollback)`
- **Notes:** A non-journaling witness still registers peer_commit, so after a leader failure it is resolved like a prepared one; a witness crash loses that in-memory state (namespace-mutations 8).

<a id="f-ns-056"></a>

#### F-NS-056 · RENAME cross-rank two-phase commit

*Makes a rename atomic when its dentries, inodes or replicas span several ranks.*

Witnesses are every rank replicating srcdn or destdn, the srcdn auth (always last) and the auths of remote-linked source/target inodes; ranks owning subtrees nested under a renamed directory journal the new location even if not auth (forced journaling). If witnesses exist and any dentry on the traces is projected, the leader first waits for the whole log to be safe (need_flush). Each witness receives OP_RENAMEPREP (paths from the common ancestor, stray blob, snaprealm blobs, op stamp, alternate_name), journals EPeerUpdate "peer_rename_prep" with an encoded rename_rollback, applies and acks; the leader journals EUpdate "rename" with had_peers (no early reply), sends OP_FINISH, peers journal the commit and send OP_COMMITTED, the leader journals ECommitted. Aborts run do_rename_rollback on every prepared peer; in-doubt transactions after a failure are settled in resolve.

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks
- **Map:** [rename.md › 3.8 Witness selection and the two-phase commit (leader side)](../cpp-map/03-request-path/rename.md#38-witness-selection-and-the-two-phase-commit-leader-side) · [rename.md › 3.9 Peer side: handle_peer_rename_prep](../cpp-map/03-request-path/rename.md#39-peer-side-handle_peer_rename_prep) · [rename.md › 3.11 _rename_prepare: projection and journaling per role](../cpp-map/03-request-path/rename.md#311-_rename_prepare-projection-and-journaling-per-role) · [rename.md › 3.14 Rollback](../cpp-map/03-request-path/rename.md#314-rollback) · [rename.md › 5.3 Journal events](../cpp-map/03-request-path/rename.md#53-journal-events)
- **Code:** `src/mds/Server.cc:9818 (Server::_rename_prepare_witness)`, `src/mds/Server.cc:10522 (Server::handle_peer_rename_prep)`, `src/mds/Server.cc:10838 (Server::_commit_peer_rename)`, `src/mds/Server.cc:10992 (Server::do_rename_rollback)`, `src/mds/Server.cc:9893 (Server::_need_force_journal)`
- **Notes:** Rollback never restores ctime/mtime because the rollback record's ctime fields are never set (rename 8, confirmed by grep). Witness traversal errors other than retry/ESTALE hit ceph_assert. Latency with journaled peers includes two sequential journal commits plus round trips; remote xlocks/wrlocks cost one round trip each. Recovery through resolve is catalogued by category 01.

<a id="f-ns-057"></a>

#### F-NS-057 · Inode authority and cap migration inside a cross-rank rename

*Moves a renamed inode's authority and all client caps to the destination rank when the source primary dentry lives elsewhere.*

When srcdn is primary and not auth on the leader, srci is frozen on the srcdn auth and marked ambiguous-auth on every participant (requests wait WAIT_SINGLEAUTH). Before its prepare the srcdn auth sends OP_RENAMENOTIFY to srcdn replicas and CEPH_SESSION_FLUSHMSG to open client sessions with caps on srci (only if the connection has CEPH_FEATURE_EXPORT_PEER), waiting for all acks. It exports srci with its caps in the prepare ack; the leader imports it, force-opens the client sessions (blocklisted clients skipped) and sends CEPH_CAP_OP_IMPORT; on commit the exporter sends CEPH_CAP_OP_EXPORT. Clients handle EXPORT and IMPORT in either order. A directory moved across ranks moves only its inode; its dirfrags stay on the old rank as new subtree roots.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, mds-ranks, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Server::flush_session returning without CEPH_SESSION_FLUSHMSG for a session whose connection lacks CEPH_FEATURE_EXPORT_PEER (bit 37). The 7.3 kernel and current libcephfs advertise CEPH_FEATURE_EXPORT_PEER, so every baseline session with caps on the renamed inode receives CEPH_SESSION_FLUSHMSG. The C++ feature table dates the bit to kernel 3.14, and Ceph clients have it since v0.75. Evidence: `src/mds/Server.cc:897-902 (Server::flush_session)`, `linux/include/linux/ceph/ceph_features.h:205 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:141 (EXPORT_PEER // 3.14)`, `src/include/ceph_features.h:220 (CEPH_FEATURES_ALL)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:882 (EXPORT_PEER since v0.75)`.
- **Map:** [rename.md › 3.10 Inode migration inside a rename](../cpp-map/03-request-path/rename.md#310-inode-migration-inside-a-rename) · [rename.md › 5.1 Client-facing messages](../cpp-map/03-request-path/rename.md#51-client-facing-messages) · [rename.md › 6. Concurrency and mds_lock](../cpp-map/03-request-path/rename.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/Server.cc:9863-9891 (Server::_rename_prepare_import)`, `src/mds/Server.cc:10760 (Server::_logged_peer_rename)`, `src/mds/Server.cc:897 (Server::flush_session)`, `linux/fs/ceph/caps.c:4154 (handle_cap_export)`
- **Notes:** Cap import/export message formats belong to category 03 and subtree migration to category 07; this entry is only the rename-specific use. Importing an inode can resurrect closed client sessions (OPENING -> OPEN) for clients holding caps on it (rename 8).

### Attributes: setattr, truncate and timestamps

SETATTR field by field, size growth and shrink with asynchronous OSD truncation, the client-clock timestamp rule, fscrypt inode fields and the legacy layout ops.

<a id="f-ns-058"></a>

#### F-NS-058 · SETATTR request: mask, locks, error order and common tail

*The single op that changes inode attributes when the client cannot apply the change under an exclusive cap.*

args.setattr carries mode, uid, gid, mtime, atime, size, old_size, mask, btime; MClientRequest v6 adds fscrypt_auth/fscrypt_file and a data payload. Mask bits: MODE 1, UID 2, GID 4, MTIME 8, ATIME 0x10, SIZE 0x20, CTIME 0x40, MTIME_NOW 0x80, ATIME_NOW 0x100 (both ignored by the MDS), BTIME 0x200, KILL_SGUID 0x400, FSCRYPT_AUTH 0x800, FSCRYPT_FILE 0x1000, KILL_SUID 0x2000, KILL_SGID 0x4000. Error order: read-only gate -> path errors -> snapshot -EROFS -> MDS-private inode (ino < 0x1000, not root/mdsdir) -EPERM -> fscrypt fields from an old header -EPERM/-EINVAL -> locks (authlock xlock for MODE/UID/GID/BTIME/KILL_*/FSCRYPT_AUTH, filelock xlock for MTIME/ATIME/SIZE/FSCRYPT_FILE, versionlock wrlock for CTIME) -> check_access(MAY_WRITE[|MAY_CHOWN|MAY_CHGRP]) -> growth while full -ENOSPC -> size-specific checks. Common tail for every SETATTR, including an empty mask ("touch ctime"): ctime = op stamp, rctime = max, change_attr += 1, EUpdate "setattr" with PREDIRTY_PRIMARY (parent mtime untouched); early reply allowed.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The SETATTR branch for MClientRequest messages below v6: -EPERM on an inode with fscrypt_auth and -EINVAL for the FSCRYPT_AUTH/FSCRYPT_FILE mask bits ('fscrypt fields from an old header'). The branch tests the MClientRequest message version. The 7.3 kernel sends message v6 towards an MDS with FS_BTIME and CephFS bits 18 and 20, and current libcephfs sends HEAD_VERSION 6. Messages below v6 come from kernels before v6.6, the CentOS Stream 8 sample (v4) and Ceph clients before 18.x. Evidence: `src/mds/Server.cc:5596-5607 (Server::handle_client_setattr)`, `linux/fs/ceph/mds_client.c:3367-3373 (create_request_message)`, `linux/fs/ceph/mds_client.c:3160-3169 (mds_supported_head_version)`, `src/messages/MClientRequest.h:92 (HEAD_VERSION = 6)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:878 (message version per client release)`.
- **Map:** [attributes-and-xattrs.md › 3.3.1 Request](../cpp-map/03-request-path/attributes-and-xattrs.md#331-request) · [attributes-and-xattrs.md › 3.3.2 Preconditions and error order (src/mds/Server.cc:55...](../cpp-map/03-request-path/attributes-and-xattrs.md#332-preconditions-and-error-order-srcmdsservercc5578-serverhandle_client_setattr) · [attributes-and-xattrs.md › 3.3.4 Locks taken by SETATTR](../cpp-map/03-request-path/attributes-and-xattrs.md#334-locks-taken-by-setattr) · [attributes-and-xattrs.md › 3.3.7 Which attribute changes never reach SETATTR](../cpp-map/03-request-path/attributes-and-xattrs.md#337-which-attribute-changes-never-reach-setattr)
- **Code:** `src/mds/Server.cc:5578 (Server::handle_client_setattr)`, `src/include/ceph_fs.h:461-476 (CEPH_SETATTR_*)`, `src/mds/Server.cc:5759-5777 (Server::handle_client_setattr)`
- **Notes:** Clients holding Ax/Fx/Fw apply many changes locally and flush them in cap messages instead (category 03); the kernel sends CTIME alone when only ctime must change and no exclusive cap is held. An xlock on filelock revokes Fs/Fr/Fw/Fx/Fl (not Fc/Fb) from every client; an atime-only SETATTR therefore revokes file caps from all other clients (attributes 8 item 25).

<a id="f-ns-059"></a>

#### F-NS-059 · chmod and chown, with set-user/group-ID clearing

*Changes mode bits, owner and group; clears setuid/setgid when asked or when ownership changes on an executable file.*

MODE sets mode = (mode & ~07777) | (args.mode & 07777) keeping the file type; UID/GID set uid/gid. Without MODE, for a regular file: if UID, GID or KILL_SGUID is present and any execute bit is set, both S_ISUID and S_ISGID are cleared (also when uid/gid are unchanged); otherwise KILL_SUID clears S_ISUID and KILL_SGID clears S_ISGID. Ownership rules (owner-only chmod, privileged chown, chgrp to own groups, S_ISGID clearing for non-members) are enforced by the VFS setattr_prepare or libcephfs may_setattr; the MDS checks only MDS caps, where MAY_CHOWN/MAY_CHGRP matter for uid= grants. authlock xlock revokes As/Ax first; the kernel rewrites the access ACL after a mode change (posix_acl_chmod).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [attributes-and-xattrs.md › 3.3.3 Per-field semantics](../cpp-map/03-request-path/attributes-and-xattrs.md#333-per-field-semantics) · [posix-semantics-contract.md › 3.3.5 chmod, chown, utimes, truncate](../cpp-map/03-request-path/posix-semantics-contract.md#335-chmod-chown-utimes-truncate)
- **Code:** `src/mds/Server.cc:5699-5719 (Server::handle_client_setattr)`, `linux/fs/attr.c:161-235 (setattr_prepare)`, `src/client/Client.cc:17615-17650 (Client::clear_suid_sgid)`
- **Notes:** The kernel never sends KILL_* bits (the VFS converts privilege removal into a mode change); libcephfs sends KILL_SUID/KILL_SGID on writes. Under uid= grants an owner chmod of a 0444 file fails because SETATTR always asks MAY_WRITE (attributes 3.2).

<a id="f-ns-060"></a>

#### F-NS-060 · Explicit times (utimes) and btime, with time_warp_seq

*Sets atime, mtime and birth time to client-supplied values.*

MTIME, ATIME and BTIME copy the request values into the inode and increment time_warp_seq, which tells cap holders that times may have moved backwards; ctime = op stamp as for every SETATTR. MTIME and ATIME take the filelock xlock, BTIME the authlock xlock; BTIME is cleared from the mask for request headers older than v4. MTIME_NOW/ATIME_NOW are ignored (libcephfs strips them after its permission check; the VFS resolves "now" itself).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Clearing CEPH_SETATTR_BTIME and zeroing btime for requests decoded from the legacy 88-byte head (MClientRequest message below v4, the 'request headers older than v4'). The 7.3 kernel and current libcephfs encode the legacy head only towards an MDS without FS_BTIME (bit 59), which the baseline MDS advertises. The legacy head comes from kernels up to v5.10 and Ceph 10.2.x and older. The 7.3 kernel defines CEPH_SETATTR_BTIME but never sets it. Evidence: `src/messages/MClientRequest.h:271-293 (MClientRequest::decode_payload)`, `linux/fs/ceph/mds_client.c:3202 (create_request_message: legacy = !FS_BTIME)`, `linux/fs/ceph/mds_client.c:3351-3352 (create_request_message)`, `src/messages/MClientRequest.h:329-336 (MClientRequest::encode_payload)`, `linux/include/linux/ceph/ceph_fs.h:383 (CEPH_SETATTR_BTIME, no user in linux/fs/ceph)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:876`.
- **Map:** [attributes-and-xattrs.md › 3.3.3 Per-field semantics](../cpp-map/03-request-path/attributes-and-xattrs.md#333-per-field-semantics) · [posix-semantics-contract.md › 3.4 Timestamps and change_attr](../cpp-map/03-request-path/posix-semantics-contract.md#34-timestamps-and-change_attr)
- **Code:** `src/mds/Server.cc:5722-5729 (Server::handle_client_setattr)`, `src/messages/MClientRequest.h:290 (MClientRequest::decode_payload)`
- **Notes:** btime is exposed through statx (kernel when non-zero; libcephfs) and settable through libcephfs/Ganesha; the kernel does not send BTIME. Explicit times need ownership, UTIME_NOW needs ownership or write permission: enforced by the VFS (may_setattr) or libcephfs.

<a id="f-ns-061"></a>

#### F-NS-061 · Size growth through SETATTR

*Extends a file; the new range reads as zeros because no objects exist there.*

SIZE not shrinking sets size = rbytes = args.size and mtime = op stamp (overriding an explicit MTIME in the same request; also when the size does not change), and recomputes client_ranges for every client with Fw/Fb issued or wanted. The MDS checks neither the file type, nor max_file_size, nor quotas for size changes: clients return -EFBIG and -EDQUOT themselves; growth is refused with -ENOSPC only when the metadata pool is full.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [attributes-and-xattrs.md › 3.3.3 Per-field semantics](../cpp-map/03-request-path/attributes-and-xattrs.md#333-per-field-semantics) · [posix-semantics-contract.md › 3.3.5 chmod, chown, utimes, truncate](../cpp-map/03-request-path/posix-semantics-contract.md#335-chmod-chown-utimes-truncate)
- **Code:** `src/mds/Server.cc:5730-5738 (Server::handle_client_setattr)`, `linux/fs/ceph/inode.c:2944-2951 (ceph_setattr)`
- **Notes:** The kernel grows files locally under Fx and flushes the size with caps. A SIZE applied to a directory would be accepted by the MDS; clients never send it (attributes 8 item 6).

<a id="f-ns-062"></a>

#### F-NS-062 · Truncate (shrink) with asynchronous OSD truncation

*Shrinks a file: the metadata change is journaled first, then clients are told and the RADOS objects are truncated in the background.*

A shrinking SIZE (or size equal with fscrypt data) waits while a previous truncate is pending (WAIT_TRUNC), then inode_t::truncate sets size = rbytes = truncate_size = new size, truncate_from = old size, truncate_seq++, truncate_pending++ and journals truncate_start with the EUpdate. After commit every cap holder gets CEPH_CAP_OP_TRUNC (size, truncate_seq, truncate_size) and MDCache::truncate_inode runs Filer truncate with the realm SnapContext and truncate_seq (waiting for a needed snapflush first); its completion journals truncate_finish and wakes WAIT_TRUNC. A log segment with a pending truncate cannot expire; replay restarts unfinished truncates from truncate_start/truncate_finish records. The OSDs use truncate_seq/truncate_size carried by client writes to discard stale data.

- **Class:** POSIX `required` · exposure `client-wire`, `cluster-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, osd
- **Map:** [attributes-and-xattrs.md › 3.3.5 Truncate (shrink) state machine](../cpp-map/03-request-path/attributes-and-xattrs.md#335-truncate-shrink-state-machine) · [rados-and-osd-interactions.md › 1. Purpose and responsibilities](../cpp-map/07-storage-formats/rados-and-osd-interactions.md#1-purpose-and-responsibilities)
- **Code:** `src/mds/Server.cc:5643-5686 (Server::handle_client_setattr)`, `src/mds/Locker.cc:2717 (Locker::issue_truncate)`, `src/mds/MDCache.cc:6586 (MDCache::truncate_inode)`, `src/mds/MDCache.cc:6847 (MDCache::start_recovered_truncates)`, `linux/fs/ceph/inode.c:808 (ceph_fill_file_size)`
- **Notes:** The kernel applies a TRUNC only if truncate_seq advances and warns if Fr/Fl were not revoked first. A non-ENOENT OSD error asserts in C_IO_MDC_TruncateFinish, and a stuck truncate keeps its journal segment unexpirable (attributes 8 item 16). Only SETATTR SIZE and OPEN O_TRUNC shrink files; cap flushes and recovery only grow size. See also F-STORE-065 (OSD truncation), F-CACHE-006, F-CAPS-016.

<a id="f-ns-063"></a>

#### F-NS-063 · Timestamps from the client clock and change_attr

*Every ctime and mtime the MDS writes is the client-supplied request stamp; change_attr counts metadata changes for NFS.*

MDCache::request_start copies the request stamp into the op stamp (the MDS clock is used only when the stamp is zero); the kernel stamps requests with coarse real time (SETATTR with ia_ctime), libcephfs with ceph_clock_now(). Targets of SETATTR, SETXATTR, RMXATTR, LINK, UNLINK/RMDIR, RENAME and O_TRUNC get ctime = op stamp, rctime = max(rctime, op stamp), change_attr += 1; new inodes get all four times = op stamp and change_attr 0; parent directories follow F-NS-043. Cap flushes apply ctime and change_attr only if newer. change_attr is exposed as STATX_CHANGE_COOKIE (kernel i_version) and stx_version (libcephfs), the NFS change attribute used by Ganesha.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [posix-semantics-contract.md › 3.4 Timestamps and change_attr](../cpp-map/03-request-path/posix-semantics-contract.md#34-timestamps-and-change_attr) · [request-lifecycle.md › 3.3 handle_client_request: session checks, idempotency, r...](../cpp-map/03-request-path/request-lifecycle.md#33-handle_client_request-session-checks-idempotency-registration) · [attributes-and-xattrs.md › 3.3.1 Request](../cpp-map/03-request-path/attributes-and-xattrs.md#331-request)
- **Code:** `src/mds/MDCache.cc:9845 (MDCache::request_start)`, `src/mds/Mutation.h:206-209 (MutationImpl::get_op_stamp)`, `linux/fs/ceph/mds_client.c:2792 (ceph_mdsc_create_request)`, `src/client/Client.cc:9336 (Client::fill_statx)`
- **Notes:** Clock skew: a client with a slow clock cannot move a directory mtime backwards and gives its own new files past timestamps; a fast clock pushes rctime ahead permanently. SETDIRLAYOUT is the one attribute op that updates neither ctime nor change_attr (attributes 8 item 7). libcephfs reports st_ctime as max(ctime, mtime). See also F-CACHE-004 (time fields).

<a id="f-ns-064"></a>

#### F-NS-064 · fscrypt inode fields and last-block truncate

*Stores the opaque fscrypt context and real size for encrypted files and performs the read-modify-write truncate of their last block.*

fscrypt_auth and fscrypt_file are opaque byte vectors copied from CREATE/MKNOD/MKDIR/SYMLINK requests and from SETATTR FSCRYPT_AUTH/FSCRYPT_FILE (MClientRequest v6); SETATTR with these bits requires a grant with allow * (MDSAuthCaps::allow_all) else -EACCES, and old request headers get -EPERM on an fscrypt inode or -EINVAL for the bits. For an encrypted file truncated to a non-block-aligned size the client sends rounded sizes, the real size in fscrypt_file and a payload = versioned header (change_attr, file_offset, block_size) + re-encrypted last block; payloads over sizeof(header) + mds_fscrypt_last_block_max_size give -EINVAL, a change_attr different from the projected one flushes the journal and returns -EAGAIN (the kernel retries up to 20 times), otherwise the block is stored in fscrypt_last_block, written to the OSD before truncation and cleared at truncate_finish. Symlink targets and dentry names of encrypted directories are stored as given by the client (base64 ciphertext, alternate_name).

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk`, `config` · relied on by kclient, libcephfs, osd
- **Legacy parts** (old-clients): The SETATTR branch for MClientRequest messages below v6 ('old request headers'): -EPERM on an fscrypt inode and -EINVAL for the FSCRYPT_AUTH/FSCRYPT_FILE bits. The branch tests the MClientRequest message version. The 7.3 kernel sends message v6 towards an MDS with FS_BTIME and CephFS bits 18 and 20, and current libcephfs sends HEAD_VERSION 6. Messages below v6 come from kernels before v6.6, the CentOS Stream 8 sample (v4) and Ceph clients before 18.x. Evidence: `src/mds/Server.cc:5596-5607 (Server::handle_client_setattr)`, `linux/fs/ceph/mds_client.c:3367-3373 (create_request_message)`, `linux/fs/ceph/mds_client.c:3160-3169 (mds_supported_head_version)`, `src/messages/MClientRequest.h:92 (HEAD_VERSION = 6)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:878 (message version per client release)`.
- **Gates:** mds_fscrypt_last_block_max_size (default 4096; startup value only); MDS auth cap allow * (allow_all)
- **Map:** [attributes-and-xattrs.md › 3.3.6 fscrypt truncate (last-block read-modify-write)](../cpp-map/03-request-path/attributes-and-xattrs.md#336-fscrypt-truncate-last-block-read-modify-write) · [attributes-and-xattrs.md › 3.2 Permission model: who checks what](../cpp-map/03-request-path/attributes-and-xattrs.md#32-permission-model-who-checks-what)
- **Code:** `src/mds/Server.cc:5596-5607 (Server::handle_client_setattr)`, `src/mds/Server.cc:5662-5686 (Server::handle_client_setattr)`, `src/mds/fscrypt.h:19 (ceph_fscrypt_last_block_header)`, `linux/fs/ceph/inode.c:2438 (fill_fscrypt_truncate)`
- **Notes:** Bug by code reading: the allow_all() check runs after project_inode() and leaves the rejected projection at the front of the inode's projection list, so later journal entries carry the rejected fields and committed state lags one projection (attributes 8 item 3; not observed on a cluster). Cap flushes set fscrypt_auth/fscrypt_file without the allow_all() check. The in-memory header size is ABI-dependent (32 bytes on x86_64) while the kernel sends a packed 26-byte header. See also F-CACHE-009 (fscrypt fields), F-ADM-020 (allow * gate).

<a id="f-ns-065"></a>

#### F-NS-065 · Layout ops SETLAYOUT and SETDIRLAYOUT (kernel layout ioctls)

*Ioctl-driven layout changes of an empty file or of a directory's layout policy, predating the layout vxattrs.*

SETLAYOUT (kclient CEPH_IOC_SET_LAYOUT): regular files only (-EINVAL), snapshot -EROFS, -ENOTEMPTY if size != 0 or truncate_seq > 1; each non-zero field overrides the current value (pool 0 cannot be chosen); a named pool newer than the MDS map waits for it; a changed layout adds MAY_SET_VXATTR; invalid -EINVAL; filelock xlock; projects layout, ctime, change_attr; EUpdate "setlayout", early reply allowed. SETDIRLAYOUT (CEPH_IOC_SET_LAYOUT_POLICY): -ENOTDIR for non-directories; starts from the own, inherited or default layout; projects only layout and version (no ctime/change_attr), no early reply. Both are gated by the full-pool check.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient
- **Map:** [attributes-and-xattrs.md › 3.5 Legacy layout ops](../cpp-map/03-request-path/attributes-and-xattrs.md#35-legacy-layout-ops)
- **Code:** `src/mds/Server.cc:5848 (Server::handle_client_setlayout)`, `src/mds/Server.cc:5971 (Server::handle_client_setdirlayout)`, `linux/fs/ceph/ioctl.c:65 (ceph_ioctl_set_layout)`
- **Notes:** Bug by code reading: SETLAYOUT records the old pool with a backtrace version that makes is_backtrace_updated() false, so no backtrace is written to the new pool until a later op dirties it (attributes 8 item 8). The same changes are available through layout vxattrs (F-NS-072).

### Extended attributes, ACLs and virtual xattrs

Stored xattrs and their flags and limits, how xattrs reach clients, POSIX ACLs as opaque xattrs, ceph.mirror.* names, ceph.* virtual xattrs that this category owns (layouts, charmap) and the common vxattr path, GETVXATTR, and alternate_name.

<a id="f-ns-066"></a>

#### F-NS-066 · SETXATTR and RMXATTR of stored xattrs

*Sets or removes extended attributes stored as an opaque name/value map per inode, with CREATE/REPLACE semantics.*

Names are routed first: ceph.* vxattr names go to the vxattr path (F-NS-071); other ceph.* names except ceph.mirror.info and ceph.mirror.dirty_snap_id get -EINVAL; any non-ceph name is accepted (namespace rules user./trusted./security./system. are enforced by clients). SETXATTR: snapshot -EROFS -> xattrlock xlock -> check_access(MAY_WRITE) -> size limit -ENOSPC (F-NS-067) -> flags: CEPH_XATTR_CREATE (1) -EEXIST if present, CEPH_XATTR_REPLACE (2) -ENODATA if absent, CEPH_XATTR_REMOVE2 (1<<30) -ENODATA if absent then remove, CEPH_XATTR_REMOVE (1<<31) remove even if absent, 0 create-or-replace -> ctime = op stamp, change_attr += 1, xattr_version += 1, EUpdate "setxattr", early reply allowed. RMXATTR: same routing and snapshot check, -ENODATA if absent, same version bumps, no check_access. The kernel sends RMXATTR for a NULL value with XATTR_REPLACE and SETXATTR|REMOVE otherwise; libcephfs uses REMOVE|REMOVE2 for NULL values and RMXATTR for removexattr.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [attributes-and-xattrs.md › 3.6.1 SETXATTR (src/mds/Server.cc:7151 (Server::handle_cl...](../cpp-map/03-request-path/attributes-and-xattrs.md#361-setxattr-srcmdsservercc7151-serverhandle_client_setxattr) · [attributes-and-xattrs.md › 3.6.2 RMXATTR (src/mds/Server.cc:7248 (Server::handle_cli...](../cpp-map/03-request-path/attributes-and-xattrs.md#362-rmxattr-srcmdsservercc7248-serverhandle_client_removexattr) · [attributes-and-xattrs.md › 3.6.3 Flag semantics (src/mds/Server.cc:7004 (Server::xat...](../cpp-map/03-request-path/attributes-and-xattrs.md#363-flag-semantics-srcmdsservercc7004-serverxattr_validate-flags-srcincludeceph_fsh510-513-ceph_xattr_) · [posix-semantics-contract.md › 3.3.7 xattrs and POSIX ACLs](../cpp-map/03-request-path/posix-semantics-contract.md#337-xattrs-and-posix-acls)
- **Code:** `src/mds/Server.cc:7151 (Server::handle_client_setxattr)`, `src/mds/Server.cc:7248 (Server::handle_client_removexattr)`, `src/mds/Server.cc:7004 (Server::xattr_validate)`, `linux/fs/ceph/xattr.c:1107-1135 (ceph_sync_setxattr)`
- **Notes:** RMXATTR of regular names has no MDS auth-cap check at all, so path, read-only and uid restrictions are not enforced for removal (attributes 8 item 12, code reading). The kernel applies xattr changes locally under Xx and flushes the whole map with caps (category 03). The kernel lists stored ceph.mirror.* names; libcephfs hides every ceph.* name in listxattr.

<a id="f-ns-067"></a>

#### F-NS-067 · Total xattr size limit (max_xattr_size)

*Limits the combined size of an inode's xattr names and values to a per-file-system value from the MDSMap.*

SETXATTR sums len(name) + len(value) over the projected map (excluding the entry being replaced only when CEPH_XATTR_REPLACE is set) and returns -ENOSPC if adding the new pair exceeds MDSMap::max_xattr_size (default 64 KiB, set with "ceph fs set <fs> max_xattr_size <n>", encoded from MDSMap ext version 18). The kernel reads the same value and falls back to a synchronous SETXATTR when its own framed estimate exceeds it; libcephfs does no client-side check. The limit is not applied to initial xattrs at create, and an oversize blob flushed with Xx caps is silently ignored while xattr_version is still bumped.

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire`, `admin` · relied on by kclient, libcephfs, mon, ceph-fuse, ganesha
- **Gates:** MDSMap max_xattr_size (ceph fs set <fs> max_xattr_size)
- **Map:** [attributes-and-xattrs.md › 3.6.4 Size limit](../cpp-map/03-request-path/attributes-and-xattrs.md#364-size-limit) · [attributes-and-xattrs.md › 7.5 Monitor](../cpp-map/03-request-path/attributes-and-xattrs.md#75-monitor)
- **Code:** `src/mds/Server.cc:7191-7212 (Server::handle_client_setxattr)`, `src/mds/MDSMap.h:64 (MDS_MAX_XATTR_SIZE)`, `src/mon/FSCommands.cc:541 (FileSystemCommandHandler::set_val)`, `linux/fs/ceph/xattr.c:1235-1271 (__ceph_setxattr)`
- **Notes:** Without REPLACE, replacing an existing xattr counts both old and new values, and a REMOVE through SETXATTR can fail with -ENOSPC near the limit; the VFS limits single values to 64 KiB (E2BIG) independently.

<a id="f-ns-068"></a>

#### F-NS-068 · xattr blob delivery to clients

*Sends the whole xattr map of an inode to clients inside InodeStat and cap messages, versioned by xattr_version.*

CInode::encode_inodestat includes the blob (u32 blob_len, u32 count, then per entry u32 name_len, name, u32 val_len, val; or u32 0) when the client has no cap yet, its cap's client_xattr_version is older, or the getattr mask asks for Xs, and keeps it only if Xs is pending or requested; xattr_version is a separate field. The xlocking client or the loner sees projected xattrs. Clients cache the blob under Xs and serve getxattr/listxattr locally; libcephfs forces a GETATTR with Xs the first time ACLs are evaluated.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [attributes-and-xattrs.md › 3.6.6 How xattrs reach clients](../cpp-map/03-request-path/attributes-and-xattrs.md#366-how-xattrs-reach-clients) · [posix-semantics-contract.md › 3.3.7 xattrs and POSIX ACLs](../cpp-map/03-request-path/posix-semantics-contract.md#337-xattrs-and-posix-acls)
- **Code:** `src/mds/CInode.cc:4104-4118 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4269-4278 (CInode::encode_inodestat)`
- **Notes:** libcephfs with client_debug_getattr_caps aborts if a reply lacks xattrs when Xs was requested (lookup 7.2); batch followers can hit this (F-NS-006).

<a id="f-ns-069"></a>

#### F-NS-069 · POSIX ACLs stored as opaque xattrs

*Persists system.posix_acl_access and system.posix_acl_default; all ACL evaluation, inheritance and chmod interaction happen in clients.*

The MDS has no ACL code: the two names are ordinary xattrs holding the Linux xattr ACL encoding, set through SETXATTR/RMXATTR and stored verbatim when sent as initial xattrs of CREATE/MKNOD/MKDIR. The kernel (on unless mounted noacl) evaluates them in generic_permission, computes default-ACL inheritance in ceph_pre_init_acls and rewrites the access ACL after chmod; removing an ACL is sent as SETXATTR with CEPH_XATTR_REMOVE. libcephfs handles ACLs only with client_acl_type=posix_acl (empty by default; NFS-Ganesha FSAL_CEPH sets it on every mount).

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** client_acl_type (lib, default empty); kernel mount option acl/noacl
- **Map:** [attributes-and-xattrs.md › 3.7 POSIX ACLs](../cpp-map/03-request-path/attributes-and-xattrs.md#37-posix-acls) · [posix-semantics-contract.md › 3.3.7 xattrs and POSIX ACLs](../cpp-map/03-request-path/posix-semantics-contract.md#337-xattrs-and-posix-acls)
- **Code:** `src/mds/Server.cc:3748-3755 (Server::prepare_new_inode)`, `linux/fs/ceph/acl.c:90 (ceph_set_acl)`, `src/client/Client.cc:18527 (Client::_posix_acl_permission)`
- **Notes:** The MDS's own mode-bit test for uid= grants ignores ACLs. pjd-fstest xacl tests run against ceph-fuse with client_acl_type=posix_acl in qa.

<a id="f-ns-070"></a>

#### F-NS-070 · ceph.mirror.* xattrs for snapshot mirroring

*Stores the mirroring peer identity on the root and a dirty snapshot id used by cephfs-mirror.*

ceph.mirror.info is accepted only on the root (-EINVAL elsewhere); its value must match "cluster_id=<uuid> fs_id=<int>" (-EINVAL otherwise) and is stored as two real xattrs ceph.mirror.info.cluster_id and ceph.mirror.info.fs_id, which removal deletes together; a SETXATTR with REMOVE flags on this name fails with -EINVAL. ceph.mirror.dirty_snap_id is stored with the default handler on any inode. Both take the regular xattr path (xattrlock xlock, early reply allowed).

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by mgr, cephfs-mirror, libcephfs
- **Map:** [attributes-and-xattrs.md › 3.6.5 Handler table and ceph.mirror.*](../cpp-map/03-request-path/attributes-and-xattrs.md#365-handler-table-and-cephmirror) · [attributes-and-xattrs.md › 7.4 mgr modules and tools](../cpp-map/03-request-path/attributes-and-xattrs.md#74-mgr-modules-and-tools)
- **Code:** `src/mds/Server.cc:7074-7130 (Server::mirror_info_xattr_validate)`, `src/mds/Server.cc:6968 (Server::xattr_handlers)`, `src/tools/cephfs_mirror/PeerReplayer.cc:3051 (PeerReplayer::do_synchronize)`
- **Notes:** Written by the mgr mirroring module on the peer file system and by cephfs-mirror. libcephfs synthesizes ceph.mirror.info from the two stored xattrs and reads dirty_snap_id after a GETVXATTR miss; the kernel has no table entry and gets -ENODATA from GETVXATTR. See also F-ADM-104 (mgr/mirroring).

<a id="f-ns-071"></a>

#### F-NS-071 · Virtual xattr set/remove path (common rules, p flag)

*Common handling of SETXATTR/RMXATTR on ceph.* virtual xattr names, which change inode policy instead of the xattr map.*

is_ceph_vxattr matches the prefixes ceph.dir.layout, ceph.file.layout, ceph.quota and the names ceph.quiesce.block, ceph.dir.subvolume, ceph.dir.pin, ceph.dir.pin.random, ceph.dir.pin.distributed, ceph.dir.charmap, ceph.dir.normalization, ceph.dir.encoding, ceph.dir.casesensitive, ceph.dir.subvolume.snaps.visible. handle_client_setvxattr uses only the base ino of the filepath (try_get_auth_inode: -ESTALE if uncached, forward if not auth), checks MAY_SET_VXATTR (MDS-cap p flag, not combined with MAY_WRITE, so root_squash does not apply and "allow rp" suffices), ignores CREATE/REPLACE flags, has no snapshot -EROFS check and skips the quarantine block; each name branch validates and locks (mostly xlock_policylock). Common tail: ctime = op stamp, change_attr += 1, backtrace update for regular files, EUpdate "set vxattr layout"; realm-changing names notify replicas and clients after commit. Policy-changing names get only a safe reply.

- **Class:** POSIX `extension` · exposure `client-wire`, `admin` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, admin
- **Gates:** MDS auth cap flag p (MAY_SET_VXATTR)
- **Map:** [attributes-and-xattrs.md › 3.8.1 Common structure](../cpp-map/03-request-path/attributes-and-xattrs.md#381-common-structure) · [attributes-and-xattrs.md › 3.8.2 Catalog of every ceph.* virtual xattr](../cpp-map/03-request-path/attributes-and-xattrs.md#382-catalog-of-every-ceph-virtual-xattr) · [attributes-and-xattrs.md › 3.2 Permission model: who checks what](../cpp-map/03-request-path/attributes-and-xattrs.md#32-permission-model-who-checks-what)
- **Code:** `src/mds/Server.cc:6342 (Server::handle_client_setvxattr)`, `src/mds/Server.h:500-557 (Server::is_ceph_vxattr)`, `src/mds/Server.cc:5930 (Server::xlock_policylock)`
- **Notes:** The individual policies behind quota, pin, subvolume, snaps.visible and quiesce.block vxattrs are catalogued by categories 08, 07 and 10; layouts and charmap here (F-NS-072, F-NS-075). xlocks on policylock/snaplock wait for the whole cluster to be recovered when any replica rank is not active (attributes 8 item 23).

<a id="f-ns-072"></a>

#### F-NS-072 · Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*)

*Reads and sets striping layouts and data pools of directories (policy for new files) and of empty files.*

Set: composite "key=value[ key=value]*" (keys stripe_unit, stripe_count, object_size, pool, pool_namespace), single fields, pool_id, pool_name, or .json (object with mandatory object_size, stripe_unit, stripe_count); validated by is_valid_layout; an unknown pool name in .pool/pool= waits for the client's osdmap epoch (args.setxattr.osdmap_epoch) or once for the latest map before -EINVAL, while pool_name and JSON fail at once. Directory layouts (xlock_policylock, no early reply) affect only future creates; removing resets the root to the default and makes other dirs inherit again; encrypted dirs -EINVAL. File layouts need size 0 and truncate_seq <= 1 (-ENOTEMPTY), take filelock, record the old pool and rewrite the backtrace; early reply allowed. GETVXATTR serves pool_name, pool_id and .json (with "inheritance": "@default", "@set" or "@inherited"); clients compute the composite and single-field names from InodeStat.

- **Class:** POSIX `extension` · exposure `client-wire`, `cluster-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, admin, osd
- **Legacy parts** (old-clients): The note's -EIO on access checks for a directory whose layout has a pool namespace when the client lacks CEPH_FEATURE_FS_FILE_LAYOUT_V2 (bit 58). The note restates the Session::check_access branch of F-NS-021. The 7.3 kernel advertises bit 58 under its alias CRUSH_TUNABLES5 and current libcephfs advertises every feature. Kernels before v4.5 and Ceph clients before 10.2.0 lack the bit. Evidence: `src/mds/SessionMap.cc:1156-1162 (Session::check_access)`, `linux/include/linux/ceph/ceph_features.h:217 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:236 (CEPH_FEATURES_ALL)`. Also: linux/include/linux/ceph/ceph_features.h:163,165 (CRUSH_TUNABLES5; FS_FILE_LAYOUT_V2 share bit 58); linux-v4.5/include/linux/ceph/ceph_features.h:75-78,125 (bit 58 first advertised; absent from linux-v4.4).
- **Map:** [attributes-and-xattrs.md › 3.8.3 Layout vxattrs](../cpp-map/03-request-path/attributes-and-xattrs.md#383-layout-vxattrs) · [attributes-and-xattrs.md › 3.8.2 Catalog of every ceph.* virtual xattr](../cpp-map/03-request-path/attributes-and-xattrs.md#382-catalog-of-every-ceph-virtual-xattr)
- **Code:** `src/mds/Server.cc:6122 (Server::parse_layout_vxattr_string)`, `src/mds/Server.cc:6046 (Server::parse_layout_vxattr_json)`, `src/mds/Server.cc:6281 (Server::check_layout_vxattr)`, `src/mds/Server.cc:7349-7442 (Server::handle_client_getvxattr)`
- **Notes:** ceph.dir.layout.pool_namespace with an empty value succeeds and, on a dir without its own layout, materializes the inherited layout as its own (code reading). JSON without pool keys resolves to pool 0. GETVXATTR aborts the MDS (ceph_abort) if it reaches the root without a layout. Clients without CEPH_FEATURE_FS_FILE_LAYOUT_V2 get -EIO on access checks for a directory whose layout has a namespace. See also F-CACHE-007 (layout field).

<a id="f-ns-073"></a>

#### F-NS-073 · GETVXATTR op

*Returns server-computed values of ceph.* virtual xattrs that clients cannot compute from InodeStat.*

A name that is not a vxattr gets -ENODATA before traversal; otherwise rdlock_path_pin_ref(want_auth) without check_access and without policylock/authlock rdlocks; dir-only names on non-dirs and file-layout names on dirs get -ENODATA. Served names: layout pool_name/pool_id/json, ceph.dir.pin, ceph.dir.pin.random, ceph.dir.pin.distributed, ceph.dir.subvolume, ceph.dir.subvolume.snaps.visible, charmap names, ceph.quiesce.block; values come from the projected inode and other vxattr names give -ENODATA. Reply extra_bl = ENCODE_START(1,1) + encoded std::string, no trace; the kernel requires value_len to equal the remaining bytes and only sends GETVXATTR when the session advertises CEPHFS_FEATURE_OP_GETVXATTR (mapping -EOPNOTSUPP to -ENODATA).

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, tools
- **Gates:** CEPHFS_FEATURE_OP_GETVXATTR
- **Map:** [attributes-and-xattrs.md › 3.9 GETVXATTR (src/mds/Server.cc:7317 (Server::handle_cli...](../cpp-map/03-request-path/attributes-and-xattrs.md#39-getvxattr-srcmdsservercc7317-serverhandle_client_getvxattr)
- **Code:** `src/mds/Server.cc:7317 (Server::handle_client_getvxattr)`, `linux/fs/ceph/mds_client.c:795 (parse_reply_info_getvxattr)`, `src/client/Client.cc:8443 (Client::_getvxattr)`
- **Notes:** No access check and projected values: a path-restricted client that knows an ino can read layout/pin/quiesce/charmap values outside its path (attributes 8 item 15, code reading). Clients send GETVXATTR for any ceph.* name missing from their tables; libcephfs then falls back to its cached xattr map.

<a id="f-ns-074"></a>

#### F-NS-074 · Client-synthesized ceph.* vxattrs (InodeStat contract)

*Most ceph.* names users read (rstats, dirstats, quota, layout, snap.btime, caps, fsid) are computed by clients from InodeStat fields the MDS sends.*

Clients build ceph.dir.entries/files/subdirs from dirstat (getattr with Fs, which rdlocks filelock), ceph.dir.rentries/rfiles/rsubdirs/rbytes/rsnaps/rctime from rstat (getattr with CEPH_STAT_RSTAT, which only forces the auth rank; no nestlock is taken so values may lag), ceph.quota* from the quota field, layouts from layout/dir_layout, ceph.snap.btime from snap_btime, and ceph.caps, ceph.cluster_fsid, ceph.client_id, ceph.auth_mds locally. InodeStat v9 carries ctime, mtime, atime, time_warp_seq, size, max_size, truncate_size/seq, mode, uid, gid, nlink, dirstat nfiles/nsubdirs, rstat rbytes/rfiles/rsubdirs/rctime, dirfragtree, symlink, dir_layout, xattrs, inline data, quota, pool_ns, btime, change_attr, export_pin, snap_btime, rsnaps, snap_metadata, fscrypt fields, optmetadata (charmap, quarantine) and subvolume_id. A set of a read-only name is refused by libcephfs (-EOPNOTSUPP); the kernel marks ceph.dir.rctime and ceph.caps writable, sends them, and the MDS answers -EINVAL.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Map:** [attributes-and-xattrs.md › 3.10 Client-synthesized vxattrs and the InodeStat contract](../cpp-map/03-request-path/attributes-and-xattrs.md#310-client-synthesized-vxattrs-and-the-inodestat-contract) · [attributes-and-xattrs.md › 3.8.2 Catalog of every ceph.* virtual xattr](../cpp-map/03-request-path/attributes-and-xattrs.md#382-catalog-of-every-ceph-virtual-xattr)
- **Code:** `src/mds/CInode.cc:4316-4371 (CInode::encode_inodestat)`, `linux/fs/ceph/xattr.c:411 (ceph_dir_vxattrs)`, `src/client/Client.cc:15826 (_dir_vxattrs)`
- **Notes:** The InodeStat encoding versions are catalogued by category 02; rstat semantics by category 08.

<a id="f-ns-075"></a>

#### F-NS-075 · Case-insensitive and normalized directories (charmap)

*Directories whose names are case-folded and Unicode-normalized by clients; the MDS stores the setting and refuses CREATE, MKDIR, SYMLINK, LINK and RENAME into such a directory from sessions without CephFS bit 22. The 7.3 kernel sets bit 22 with another meaning and does not fold names, so it is admitted.*

charmap_md_t {casesensitive=true, normalization="nfd", encoding="utf8"} lives in inode_t optmetadata, is set through ceph.dir.charmap removal and ceph.dir.casesensitive/normalization/encoding (only on an empty directory not covered by snapshots, else -ENOTEMPTY; strings not validated), is inherited by new subdirectories at MKDIR, replicated with policylock and sent to clients as InodeStat v8 optmetadata. The MDS never folds names: libcephfs with CEPHFS_FEATURE_CHARMAP (bit 22) sends the folded name as the dentry name and the original spelling as alternate_name; the MDS hashes and orders the folded name and returns alternate_name in LeaseStat v2. CREATE, MKDIR, SYMLINK, LINK and RENAME into a charmap directory return -EPERM for sessions without the feature; MKNOD, lookups and readdir are not checked.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by libcephfs, mgr, kclient, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The -EPERM that Server::can_handle_charmap returns to sessions without CEPHFS_FEATURE_CHARMAP (bit 22) for CREATE, MKDIR, SYMLINK, LINK and RENAME into a charmap directory. The 7.3 kernel sets bit 22 under its own name CEPHFS_FEATURE_SUBVOLUME_METRICS and current libcephfs sets CEPHFS_FEATURE_CHARMAP, so the gate refuses no baseline client. Kernels before v7.1, both CentOS Stream samples and Ceph clients before 19.2.3/20.2.0 lack the bit. The charmap metadata, its vxattrs and the libcephfs name folding are current. Evidence: `src/mds/Server.cc:4924 (callers of Server::can_handle_charmap)`, `src/mds/Server.cc:7711 (callers of Server::can_handle_charmap)`, `src/mds/Server.cc:7813 (callers of Server::can_handle_charmap)`, `src/mds/Server.cc:7919 (callers of Server::can_handle_charmap)`, `src/mds/Server.cc:9354 (callers of Server::can_handle_charmap)`, `src/mds/Server.cc:4876-4892 (Server::can_handle_charmap)`, `linux/fs/ceph/mds_client.h:40 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.h:59 (CEPHFS_FEATURE_SUBVOLUME_METRICS = bit 22 in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:79 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURE_CHARMAP in CEPHFS_FEATURES_ALL = CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs8-kernel/fs/ceph/mds_client.h:38-47`, `cs9-kernel/fs/ceph/mds_client.h:43-57 (no bit 22)`, `mds-rust-map/cpp-map/02-client-protocol/kclient-expectations.md:859 (bit 22: first kernel v7.1, first Ceph 19.2.3/20.2.0)`.
- **Gates:** CEPHFS_FEATURE_CHARMAP (bit 22)
- **Map:** [attributes-and-xattrs.md › 3.8.7 Charmap (case-insensitive / normalized directories)](../cpp-map/03-request-path/attributes-and-xattrs.md#387-charmap-case-insensitive--normalized-directories) · [lookup-readdir-open-and-file-locks.md › 3.9 READDIR (Server::handle_client_readdir, Server::_fina...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#39-readdir-serverhandle_client_readdir-server_finalize_readdir) · [rename.md › 3.3 Validation checks (evaluation order)](../cpp-map/03-request-path/rename.md#33-validation-checks-evaluation-order)
- **Code:** `src/mds/Server.cc:4876 (Server::can_handle_charmap)`, `src/include/cephfs/types.h:244 (charmap_md_t)`, `src/mds/CInode.cc:4120-4141 (CInode::encode_inodestat)`, `src/client/Client.cc:1344 (Client::_wrap_name)`
- **Notes:** Feature bit 22 collision (observed): the scanned Linux tree defines bit 22 as CEPHFS_FEATURE_SUBVOLUME_METRICS and advertises it, so such a kernel passes can_handle_charmap although it does not fold names; the kernel skips InodeStat v8 optmetadata (lookup page 8). The charmap vxattrs lack type and root checks, and an unusable encoding/normalization makes libcephfs fail every new name with -EACCES (attributes 8 items 9 and 27). mgr/volumes depends on the charmap JSON format. See also F-ADM-095 (mgr/volumes charmap).

<a id="f-ns-076"></a>

#### F-NS-076 · alternate_name on dentries

*Stores a second, client-defined name per dentry (fscrypt ciphertext name or the original spelling in a charmap directory).*

CREATE, MKNOD, MKDIR, SYMLINK, LINK and RENAME accept an alternate_name in the request, refuse values longer than mds_alternate_name_max (default 8 KiB) with -ENAMETOOLONG, and store it on the new dentry; RENAME onto an existing dentry with a different alternate_name returns -EINVAL. alternate_name is returned in every LeaseStat v2 (dentry traces, readdir entries, LOOKUPNAME) and travels in rename peer requests.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs
- **Gates:** mds_alternate_name_max (default 8 KiB); CEPHFS_FEATURE_REPLY_ENCODING (LeaseStat v2)
- **Map:** [rename.md › 3.3 Validation checks (evaluation order)](../cpp-map/03-request-path/rename.md#33-validation-checks-evaluation-order) · [namespace-mutations.md › 3.7.1 CREATE (src/mds/Server.cc:4895 (Server::handle_clie...](../cpp-map/03-request-path/namespace-mutations.md#371-create-srcmdsservercc4895-serverhandle_client_openc) · [lookup-readdir-open-and-file-locks.md › 3.5 Dentry leases (Locker::issue_client_lease)](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#35-dentry-leases-lockerissue_client_lease)
- **Code:** `src/mds/Server.cc:4928-4932 (Server::handle_client_openc)`, `src/mds/Server.cc:9409-9413 (Server::handle_client_rename)`, `src/mds/Locker.cc:4609 (Locker::encode_lease)`
- **Notes:** The MDS limits only alternate_name and snapshot names; NAME_MAX for ordinary names is enforced by clients (F-NS-088).

### Advisory file locks

Cluster-wide fcntl and flock locks kept in MDS memory: set/test, blocking waits, interruption, deadlock detection, cleanup on cap loss and reinstatement after an MDS restart.

<a id="f-ns-077"></a>

#### F-NS-077 · SETFILELOCK: POSIX fcntl byte-range locks

*Arbitrates cluster-wide POSIX advisory record locks (F_SETLK/F_SETLKW, also open-file-description locks) in MDS memory.*

Request args filelock_change {u8 rule, u8 type, u64 owner, u64 pid, u64 start, u64 length (0 = to EOF), u8 wait}; rule CEPH_LOCK_FCNTL 1; type SHARED 1, EXCL 2, UNLOCK 4. The op runs on the inode auth, xlocks the internal flocklock (no cap bits correspond to it) and edits the per-inode fcntl table; owner identity is (session client id forced by the MDS, owner) plus pid only when bit 63 of owner is clear. Granting merges/splits the owner's own ranges (one owner's locks never overlap; adjacent same-type ranges coalesce, giving atomic upgrade/downgrade when unopposed); conflict without wait returns -EAGAIN; UNLOCK is idempotent and returns 0. Unknown rules return -EOPNOTSUPP. No check_access is made, nothing is journaled, and SETFILELOCK is never recorded in completed_requests, so a resent request is executed again.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.11.1 Data model](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3111-data-model) · [lookup-readdir-open-and-file-locks.md › 3.11.2 Algorithms (src/mds/flock.cc)](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3112-algorithms-srcmdsflockcc) · [lookup-readdir-open-and-file-locks.md › 3.11.3 SETFILELOCK (Server::handle_client_file_setlock, s...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3113-setfilelock-serverhandle_client_file_setlock-srcmdsservercc5418) · [lookup-readdir-open-and-file-locks.md › 7.6 File locks](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#76-file-locks)
- **Code:** `src/mds/Server.cc:5418 (Server::handle_client_file_setlock)`, `src/mds/flock.cc:210 (ceph_lock_state_t::add_lock)`, `src/mds/flock.cc:392 (ceph_lock_state_t::adjust_locks)`, `src/include/ceph_fs.h:799 (ceph_filelock)`, `linux/fs/ceph/locks.c:248 (ceph_lock)`
- **Notes:** No permission check: any session that can name the inode can lock or test it; only the read-only gate and quarantine block apply (lookup page 8). No validation of type, start or length; a same-owner merge computes the end in an int, so ranges ending at or beyond 2^31 merge to a wrong length (flock.cc:452). Kernel OFD locks arrive as FCNTL locks owned by the open file.

<a id="f-ns-078"></a>

#### F-NS-078 · BSD flock whole-file locks

*Arbitrates flock(2) locks in a table separate from the fcntl locks.*

Rule CEPH_LOCK_FLOCK 2 (CEPH_LOCK_FLOCK_INTR 4 for cancellation) selects the per-inode flock table; clients send start 0, length 0 and the struct file as owner. Semantics, return codes and lifetime are those of the fcntl table except that there is no deadlock detection.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.11.1 Data model](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3111-data-model) · [posix-semantics-contract.md › 3.3.8 fcntl and flock locks](../cpp-map/03-request-path/posix-semantics-contract.md#338-fcntl-and-flock-locks)
- **Code:** `src/mds/Server.cc:5418 (Server::handle_client_file_setlock)`, `linux/fs/ceph/locks.c:318 (ceph_flock)`

<a id="f-ns-079"></a>

#### F-NS-079 · Blocking lock waits and wake-ups

*Parks F_SETLKW-style requests in the MDS until the lock can be granted.*

With wait = 1 and a conflict, the lock is added to the table's waiting list, the request drops all locks and auth pins, is marked nowarn (never a slow request) and waits on the inode's WAIT_FLOCK; there is no MDS-side timeout. Any UNLOCK that changed state, removal of a client's held locks on cap removal, and loss of auth (export) wake all WAIT_FLOCK waiters of the inode for both tables, and each retries under the flocklock xlock. Granting a lock or converting EXCL to SHARED wakes nobody.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.11.3 SETFILELOCK (Server::handle_client_file_setlock, s...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3113-setfilelock-serverhandle_client_file_setlock-srcmdsservercc5418)
- **Code:** `src/mds/Server.cc:5418 (Server::handle_client_file_setlock)`, `src/mds/flock.cc:201 (ceph_lock_state_t::add_waiting)`
- **Notes:** Compatible SHARED waiters stay parked after an EXCL->SHARED downgrade until some unlock happens on that inode; a cap release by a client that only had a queued wait wakes nobody (lookup page 8).

<a id="f-ns-080"></a>

#### F-NS-080 · Lock wait interruption (FCNTL_INTR / FLOCK_INTR)

*Cancels a parked lock request when the waiting application is interrupted by a signal.*

A client sends SETFILELOCK with rule CEPH_LOCK_FCNTL_INTR (3) or CEPH_LOCK_FLOCK_INTR (4), type UNLOCK and the same owner, pid, start and length; the MDS removes the matching waiting entry (without removing held locks) and wakes waiters; the original request then finds its entry gone and returns -EINTR. A plain UNLOCK of the same (start, length, owner) also cancels a pending wait. If the lock was already granted the INTR removes nothing and the original request returns 0.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.11.3 SETFILELOCK (Server::handle_client_file_setlock, s...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3113-setfilelock-serverhandle_client_file_setlock-srcmdsservercc5418) · [lookup-readdir-open-and-file-locks.md › 7.6 File locks](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#76-file-locks)
- **Code:** `src/mds/Server.cc:5418 (Server::handle_client_file_setlock)`, `linux/fs/ceph/locks.c:157 (ceph_lock_wait_for_completion)`, `src/client/Client.cc:13856 (Client::_interrupt_filelock)`
- **Notes:** In the grant-before-INTR race the kernel reports success, while libcephfs returns -EINTR to the caller although the MDS holds the lock until a later unlock or cap removal (lookup page 8).

<a id="f-ns-081"></a>

#### F-NS-081 · fcntl deadlock detection (EDEADLK)

*Refuses a blocking fcntl lock that would complete a wait-for cycle.*

Before parking a blocking FCNTL request, is_deadlock walks the conflicting holders and, through a process-global map of every waiting fcntl lock on any inode, the locks those holders wait for, up to MAX_DEADLK_DEPTH = 5 hops; reaching the requester returns -EDEADLK. flock locks are not checked.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.11.2 Algorithms (src/mds/flock.cc)](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3112-algorithms-srcmdsflockcc)
- **Code:** `src/mds/flock.cc:137 (ceph_lock_state_t::is_deadlock)`, `src/mds/flock.cc:19 (global_waiting_locks)`

<a id="f-ns-082"></a>

#### F-NS-082 · GETFILELOCK (F_GETLK)

*Tests whether a lock could be placed and returns the first conflicting lock.*

Runs on the inode auth with a flocklock rdlock; rule FCNTL or FLOCK selects the table (other rules -EINVAL); look_for_lock returns the first conflicting lock of another owner in ascending start order, or echoes the request with type CEPH_LOCK_UNLOCK. The reply has no trace and carries the 41-byte raw ceph_filelock (start, length, client, owner, pid, type) as extra_bl; the kernel negates the pid and converts start + length to fl_end. No check_access.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.11.4 GETFILELOCK (Server::handle_client_file_readlock, ...](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3114-getfilelock-serverhandle_client_file_readlock-srcmdsservercc5525) · [lookup-readdir-open-and-file-locks.md › 7.6 File locks](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#76-file-locks)
- **Code:** `src/mds/Server.cc:5525 (Server::handle_client_file_readlock)`, `src/mds/flock.cc:267 (ceph_lock_state_t::look_for_lock)`, `linux/fs/ceph/locks.c:131-147 (ceph_lock_message)`

<a id="f-ns-083"></a>

#### F-NS-083 · Lock cleanup on cap removal and session end

*Releases a client's file locks when its capability on the inode goes away, including on eviction.*

CInode::remove_client_cap calls remove_all_from(client) on both tables and wakes waiters if held locks were removed; this covers cap release, session close, eviction and blocklisting (every cap is removed), and cap teardown on export. Clients then mark their local locks lost (kernel CEPH_I_ERROR_FILELOCK, libcephfs I_ERROR_FILELOCK) and fail later lock calls and I/O with -EIO. A stale session only has its caps revoked, so its locks stay until eviction. Locks are not persistent: they live only in CInode memory and also vanish when the inode leaves the cache or the MDS restarts.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.11.5 Lifetime, migration and recovery](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3115-lifetime-migration-and-recovery) · [lookup-readdir-open-and-file-locks.md › 7.6 File locks](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#76-file-locks) · [posix-semantics-contract.md › 7.3 Further departures found in the code](../cpp-map/03-request-path/posix-semantics-contract.md#73-further-departures-found-in-the-code)
- **Code:** `src/mds/CInode.cc:3625 (CInode::remove_client_cap)`, `src/mds/flock.cc:360 (ceph_lock_state_t::remove_all_from)`, `linux/fs/ceph/caps.c:5099 (ceph_purge_inode_cap)`
- **Notes:** A lock whose owner holds no cap on the inode survives the owner's session end, and because locks do not pin the inode a lock can vanish silently when the inode is trimmed; clients avoid both by keeping a cap (kernel trim_caps_cb never trims the auth cap while i_filelock_ref > 0). Session reclaim (MClientReclaim, intended for Ganesha HA) kills the old session and thereby drops its locks. Export of lock tables with the inode belongs to category 07.

<a id="f-ns-084"></a>

#### F-NS-084 · Lock reinstatement after an MDS restart

*Rebuilds held fcntl and flock locks from the clients' reconnect messages.*

Each cap record in MClientReconnect (v2+) carries flockbl = u32 n_fcntl, ceph_filelock[n], u32 n_flock, ceph_filelock[n]; Server::recover_filelocks inserts them into held_locks verbatim (no conflict check, no merge), forcing client to the session id, but only for inodes that are cached and auth when the reconnect is processed. Waiting locks are not in reconnect: clients resend pending SETFILELOCK requests, which the MDS executes afresh once active. The kernel sends both counts 0 when it already flagged lock errors.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Locker::_do_cap_update inserting fcntl and flock locks from a non-empty flockbl in MClientCaps (the 'legacy flockbl' of the notes). The 7.3 kernel writes a flock length of 0 in every MClientCaps, and current libcephfs fills flockbl only in MClientReconnect cap records. No examined client sends a non-empty MClientCaps flockbl (kernels v4.4, v4.12, the CentOS Stream 8 sample and 7.3; libcephfs 10.2.0, 12.2.0 and main). The import comes from a 2010 design in which clients sent lock state in cap messages on reconnect (commit fa058c414c1). Evidence: `src/mds/Locker.cc:4161-4180 (Locker::_do_cap_update)`, `linux/fs/ceph/caps.c:1335-1336 (encode_cap_msg)`, `src/client/Client.cc:3486-3510 (Client::send_reconnect)`, `src/client/Client.cc:4119 (Client::send_cap, no flockbl)`, `linux-v4.4/fs/ceph/caps.c:1052-1053 (send_cap_msg)`, `cs8-kernel/fs/ceph/caps.c:1277-1278 (encode_cap_msg)`. Also: git fa058c414c1 (2010-05-12, mds: handle file locking in MClientCaps on a reconnect).
- **Map:** [lookup-readdir-open-and-file-locks.md › 3.11.5 Lifetime, migration and recovery](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#3115-lifetime-migration-and-recovery) · [lookup-readdir-open-and-file-locks.md › 7.6 File locks](../cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#76-file-locks)
- **Code:** `src/mds/Server.cc:1945 (Server::recover_filelocks)`, `src/mds/Server.cc:1591 (Server::handle_client_reconnect)`, `linux/fs/ceph/mds_client.c:4946-4948 (reconnect_caps_cb)`, `linux/fs/ceph/locks.c:498 (ceph_locks_to_pagelist)`
- **Notes:** Recovery gaps observed in code: inodes loaded later in rejoin (process_imported_caps, open-file-table prefetch) and, with several ranks, inodes cached as replicas get their caps back but not their locks; kernel whole-file locks arrive with finite lengths. A legacy flockbl in MClientCaps is also imported by Locker::_do_cap_update (current clients send it empty). The reconnect message format is catalogued by category 02. Each ceph_filelock record is a raw 41-byte struct; the blob is decoded under mds_lock without a try block. Because open files are tracked by the OpenFileTable and EOpen events are not re-journaled after segment expiry, whether a lock survives a failover depends on whether the inode happened to be cached from the replayed journal when the reconnect was processed (resolve-and-rejoin section 8, derived from code). After losing a session while holding locks the kernel fails further lock calls with -EIO until all local locks are released; libcephfs sends an empty blob for inodes that never had lock state and flags I_ERROR_FILELOCK after session loss. Ganesha lock owners are process addresses with pid 0, so locks of a previous Ganesha incarnation go away with the old session when the successor reclaims it (libcephfs-fuse-ganesha 8). See also F-PROTO-056 (reconnect encoding), F-CLU-082 (cap recording).

### POSIX contract and documented deviations

Departures from POSIX recorded on the POSIX semantics contract page that have no other home in this catalog: presentation of directory link counts and sizes, atime, timestamp range, limits enforced only by clients, unsupported Linux extensions, data-path coherency departures, statfs, and the conformance suites in the tree.

<a id="f-ns-085"></a>

#### F-NS-085 · Directory link count and size presentation

*Clients present POSIX-style directory link counts and sizes computed from MDS counters; the MDS stores directory nlink as 1.*

prepare_new_inode stores nlink = 1 for directories (marked FIXME) and MKDIR never changes the parent's nlink; unlink sets it to 0. The kernel reports 2 + i_subdirs for a directory whose nlink is 1; libcephfs reports 2 + dirstat.nsubdirs for 1, 0 for 0, and calls ceph_abort() for any other value. Directory st_size is the entry count (kernel) or rbytes (libcephfs default client_dirsize_rbytes=true, kernel mount option rbytes); st_blocks is ceil(size/512) for files and 0 (kernel) or 1 (libcephfs) for directories; rbytes of a file equals its logical size including holes.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** client_dirsize_rbytes (lib, default true); kernel mount option rbytes
- **Map:** [posix-semantics-contract.md › 3.3.2 mknod, mkdir, symlink, readlink](../cpp-map/03-request-path/posix-semantics-contract.md#332-mknod-mkdir-symlink-readlink) · [posix-semantics-contract.md › 4. Data structures and invariants](../cpp-map/03-request-path/posix-semantics-contract.md#4-data-structures-and-invariants) · [posix-semantics-contract.md › 7.3 Further departures found in the code](../cpp-map/03-request-path/posix-semantics-contract.md#73-further-departures-found-in-the-code)
- **Code:** `src/mds/Server.cc:3700 (Server::prepare_new_inode)`, `linux/fs/ceph/inode.c:3217-3224 (ceph_getattr)`, `src/client/Client.cc:9192-9204 (Client::fill_stat)`, `src/mds/Server.cc:8779 (Server::_unlink_local)`, `src/mds/Server.cc:8027 (Server::_link_local)`
- **Notes:** An MDS reporting a directory nlink other than 0 or 1 crashes libcephfs clients (Client::fill_stat). Accurate dirstat nsubdirs (F-NS-043) is what the presented link count depends on. Files count hard links: link increments nlink, unlink decrements it and sets STATE_ORPHAN at 0.

<a id="f-ns-086"></a>

#### F-NS-086 · Access time is not maintained by the MDS

*atime is stored and settable but never updated on reads.*

Kernel mounts set SB_NOATIME | SB_NODIRATIME; libcephfs never assigns atime in its read path; the MDS sets atime only at creation, on SETATTR ATIME and on Fx cap flushes; a forward atime change flushed under Fw alone is dropped; READDIR updates no directory atime. fstests marks atime tests "not run" for ceph and ceph-fuse ("atime not maintained").

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [posix-semantics-contract.md › 3.4 Timestamps and change_attr](../cpp-map/03-request-path/posix-semantics-contract.md#34-timestamps-and-change_attr) · [posix-semantics-contract.md › 7.2 Departures documented in doc/cephfs/posix.rst](../cpp-map/03-request-path/posix-semantics-contract.md#72-departures-documented-in-doccephfsposixrst)
- **Code:** `linux/fs/ceph/super.c:1235 (ceph_set_super)`, `src/mds/Locker.cc:4033-4036 (Locker::_update_cap_fields)`, `src/mds/Server.cc:5725 (Server::handle_client_setattr)`
- **Notes:** Documented in doc/cephfs/posix.rst lines 43-47.

<a id="f-ns-087"></a>

#### F-NS-087 · Timestamp range limited to unsigned 32-bit seconds

*Times before 1970 or after 2106 cannot be represented.*

Seconds are unsigned 32-bit in utime_t and in the wire ceph_timespec; the kernel sets s_time_min = 0, s_time_max = U32_MAX and 1 ns granularity. fstests _require_negative_timestamps marks such tests "not run" for ceph.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [posix-semantics-contract.md › 7.3 Further departures found in the code](../cpp-map/03-request-path/posix-semantics-contract.md#73-further-departures-found-in-the-code) · [posix-semantics-contract.md › 7.4 pjd-fstest and xfstests in the tree](../cpp-map/03-request-path/posix-semantics-contract.md#74-pjd-fstest-and-xfstests-in-the-tree)
- **Code:** `src/include/utime.h:47-51 (utime_t)`, `src/include/rados.h:42-45 (ceph_timespec)`, `linux/fs/ceph/super.c:1232-1234 (ceph_set_super)`

<a id="f-ns-088"></a>

#### F-NS-088 · Limits enforced only by clients, and no hard-link limit

*NAME_MAX, maximum file size and quotas are checked by clients; nobody enforces EMLINK.*

The MDS has no name-length limit for ordinary names (the kernel returns -ENAMETOOLONG above 255 bytes in atomic_open and lookup, libcephfs in _lookup); it limits only alternate_name and snapshot names. max_file_size (MDSMap, default 1 TiB) is enforced as -EFBIG by clients only. Quotas (EDQUOT, EXDEV for rename across quota realms) are enforced by clients only and are approximate. nlink is a signed 32-bit field with no EMLINK check in the MDS or the kernel client (no s_max_links).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** MDSMap max_file_size (default 1 TiB)
- **Map:** [posix-semantics-contract.md › 7.3 Further departures found in the code](../cpp-map/03-request-path/posix-semantics-contract.md#73-further-departures-found-in-the-code) · [posix-semantics-contract.md › 8. Edge cases, failure handling and gotchas](../cpp-map/03-request-path/posix-semantics-contract.md#8-edge-cases-failure-handling-and-gotchas) · [posix-semantics-contract.md › 5.2 Options and mount options that change POSIX behaviour](../cpp-map/03-request-path/posix-semantics-contract.md#52-options-and-mount-options-that-change-posix-behaviour)
- **Code:** `linux/fs/ceph/dir.c:820-821 (ceph_lookup)`, `src/mds/MDSMap.h:607 (MDSMap::max_file_size)`, `src/include/cephfs/types.h:966 (inode_t::nlink)`, `src/mds/Server.cc:8027 (Server::_link_local)`
- **Notes:** Quota enforcement and the realm rules are catalogued by category 08.

<a id="f-ns-089"></a>

#### F-NS-089 · Unsupported Linux extensions: renameat2 flags and O_TMPFILE

*RENAME_NOREPLACE, RENAME_EXCHANGE, RENAME_WHITEOUT and O_TMPFILE are not available; no MDS op exists for them.*

The kernel ceph_rename returns -EINVAL for any renameat2 flag (the VFS returns -EEXIST first for RENAME_NOREPLACE with an existing target); ceph-fuse returns -EINVAL ("cephfs does not support renameat2 flavors"); the libcephfs API has no flags and the RENAME request has no flag field. ceph_dir_iops has no .tmpfile, so O_TMPFILE gets -EOPNOTSUPP from the VFS; xfstests generic/531 (many unlinked O_TMPFILE files) is in the qa exclude list.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, ceph-fuse, libcephfs
- **Map:** [posix-semantics-contract.md › 3.3.4 rename](../cpp-map/03-request-path/posix-semantics-contract.md#334-rename) · [posix-semantics-contract.md › 7.3 Further departures found in the code](../cpp-map/03-request-path/posix-semantics-contract.md#73-further-departures-found-in-the-code)
- **Code:** `linux/fs/ceph/dir.c:1491-1492 (ceph_rename)`, `src/client/fuse_ll.cc:769-775 (fuse_ll_rename)`, `linux/fs/ceph/dir.c:2255-2271 (ceph_dir_iops)`

<a id="f-ns-090"></a>

#### F-NS-090 · Data-path coherency departures outside the MDS

*File-data behaviours that differ from POSIX and are decided by clients and OSDs, with the MDS only choosing lock states.*

O_APPEND is not atomic between clients writing concurrently (both compute end of file before obtaining caps; with several writers filelock is LOCK_MIX, granting Fr Fw Fl but neither Fs nor Fx). Writes spanning object boundaries are split into separate OSD operations and are not atomic for concurrent readers/writers. Shared writable mmap is not coherent across hosts (page faults fill the page cache without Fc/Fl). LazyIO (CEPH_O_LAZY, ceph_lazyio, CEPH_IOC_LAZYIO) wants Fl and relaxes coherency on request. O_SYNC/O_DSYNC are handled by clients; libcephfs O_SYNC flushes only the data range. Coherency otherwise comes from cap revocation driven by MDS lock states (no close-to-open semantics).

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [posix-semantics-contract.md › 3.5 Data coherency between clients](../cpp-map/03-request-path/posix-semantics-contract.md#35-data-coherency-between-clients) · [posix-semantics-contract.md › 7.2 Departures documented in doc/cephfs/posix.rst](../cpp-map/03-request-path/posix-semantics-contract.md#72-departures-documented-in-doccephfsposixrst) · [posix-semantics-contract.md › 3.3.1 open, creat and the open flags](../cpp-map/03-request-path/posix-semantics-contract.md#331-open-creat-and-the-open-flags)
- **Code:** `src/mds/locks.c:85-125 (filelock)`, `linux/fs/ceph/file.c:2431-2525 (ceph_write_iter)`, `src/client/Client.cc:12826-12838 (Client::_write)`, `linux/fs/ceph/addr.c:2022-2026 (ceph_filemap_fault)`
- **Notes:** O_APPEND and mmap findings are derived from code, not tested (posix-semantics-contract 3.3.1, 7.2 row 5). The cap model and lock states are catalogued by category 03.

<a id="f-ns-091"></a>

#### F-NS-091 · statfs is answered by the monitors

*Capacity reporting never involves the MDS except for a quota-root GETATTR from libcephfs.*

Clients send MStatfs to the monitors (MgrStatMonitor::preprocess_statfs) for the single data pool or the cluster; they report 4 MiB blocks, f_ffree = -1, f_namelen 255, f_files = RADOS object count (kernel) or rfiles + rsubdirs of the quota root (libcephfs), and substitute max_bytes quota and rbytes for mounts below a byte quota (kernel unless noquotadf, switching to 4 KiB blocks below 4 MiB; libcephfs with client_quota_df, sending a GETATTR of the quota root on each call).

- **Class:** POSIX `required` · exposure `cluster-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon
- **Gates:** client_quota_df (lib, default true); kernel mount option noquotadf
- **Map:** [posix-semantics-contract.md › 3.3.11 statfs](../cpp-map/03-request-path/posix-semantics-contract.md#3311-statfs)
- **Code:** `src/mon/MgrStatMonitor.cc:484 (MgrStatMonitor::preprocess_statfs)`, `linux/fs/ceph/super.c:59-123 (ceph_statfs)`, `src/client/Client.cc:13637 (Client::_statfs)`
- **Notes:** See also F-SNAP-048 (quota-based statfs).

<a id="f-ns-092"></a>

#### F-NS-092 · Conformance suites in the tree (pjd-fstest, xfstests)

*The POSIX acceptance tests the qa suites run against CephFS and the cases they exclude.*

qa/workunits/suites/pjd.sh downloads pjd-fstest 20090130-RC-aclfixes and runs prove -r as root; it is used by fs/permission (ceph-fuse with client acl type posix_acl and fuse set user groups), fs/workload (fuse default permissions false), fs/thrash, fs/32bits and fs/fscrypt suites; its test directories map to the calls in the POSIX contract table (chflags tests exit early on Linux). xfstests runs with FSTYP=ceph or ceph-fuse; xfstests_dev.py writes a ceph.exclude listing ceph/001, generic/003 (atime options), generic/075 (fsx), generic/538 (unaligned direct AIO) and generic/531 (O_TMPFILE), but only the fscrypt quick-group test passes it to ./check -E.

- **Class:** POSIX `operational` · exposure `internal` · relied on by tools
- **Map:** [posix-semantics-contract.md › 7.1 Use of this page as the acceptance checklist](../cpp-map/03-request-path/posix-semantics-contract.md#71-use-of-this-page-as-the-acceptance-checklist) · [posix-semantics-contract.md › 7.4 pjd-fstest and xfstests in the tree](../cpp-map/03-request-path/posix-semantics-contract.md#74-pjd-fstest-and-xfstests-in-the-tree) · [qa-and-conformance-suites.md › 7.4 Workunits](../cpp-map/14-testing/qa-and-conformance-suites.md#74-workunits)
- **Code:** `qa/workunits/suites/pjd.sh:5-14`, `qa/tasks/cephfs/xfstests_dev.py:328-336`, `qa/tasks/cephfs/test_fscrypt.py:591`

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-ns-015"></a>F-NS-015 | [F-PROTO-087](02-client-sessions-and-protocol.md#f-proto-087) · CREATE reply: created inode and delegated inode ranges |

## 4. Legend

**POSIX class**

- `required`: Implements a POSIX-specified operation, attribute, error code or guarantee that applications see directly.
- `supporting`: Not POSIX itself, but POSIX behaviour through unmodified clients depends on it (sessions, caps, leases, journaling for durability).
- `extension`: CephFS-specific feature visible to applications or administrators beyond POSIX (snapshots, quotas, layouts, vxattrs, subvolumes, fscrypt, charmap).
- `operational`: Cluster integration, high availability, administration, observability or tuning.
- `internal`: Implementation mechanism of the C++ MDS with no external contract of its own.

**Exposure**

- `client-wire`: Client protocol: MClient* messages, their fields, feature bits, reply encodings.
- `cluster-wire`: Protocols with monitors, mgr and OSDs (beacons, FSMap, MMgrReport, MOSDOp, ...).
- `mds-wire`: MDS-to-MDS messages between ranks (only Rust daemons will speak these, so they are free to redesign).
- `on-disk`: Objects, omap and xattrs the MDS stores in RADOS.
- `admin`: Admin socket, ceph tell, ceph fs / mds CLI commands and their output.
- `config`: Configuration options.
- `metrics`: Perf counters, health warnings, mgr metrics and cluster log.
- `internal`: No external surface.

**Relied on by**

- `kclient`: Linux kernel CephFS client (fs/ceph).
- `libcephfs`: Userspace client library src/client (libcephfs API users).
- `ceph-fuse`: ceph-fuse (built on the same src/client code).
- `ganesha`: NFS-Ganesha FSAL_CEPH (via libcephfs).
- `smb`: Samba vfs_ceph (via libcephfs).
- `cephfs-mirror`: cephfs-mirror daemon (via libcephfs).
- `mon`: Ceph monitors (MDSMonitor, FSMap, auth).
- `mgr`: ceph-mgr and its modules (volumes, nfs, status, dashboard, prometheus, stats, mds_autoscaler, snap_schedule, ...).
- `cephadm`: cephadm orchestrator.
- `rook`: Rook operator.
- `osd`: OSDs (RADOS operations, blocklists, snap contexts).
- `tools`: cephfs-journal-tool, cephfs-data-scan, cephfs-table-tool, cephfs-top, cephfs-shell and similar tools.
- `admin`: Operators and scripts using admin commands, logs and health output.
- `mds-ranks`: Other MDS ranks of the same filesystem.
- `internal`: Only the MDS itself.

**Legacy reasons**

- `old-clients`: Used only by clients older than the baseline (including the CentOS Stream kernel samples), or by client paths taken only towards an MDS that lacks a feature the C++ MDS advertises.
- `msgr1`: The msgr1 protocol or one of its features, such as message signatures. Current mounts use msgr2.
- `old-data`: Reads or upgrades on-disk or journal formats written by older Ceph releases (the C++ MDS or older mgr modules).
- `old-daemons`: Interoperation with older MDS, monitor or OSD daemons (mixed-version clusters, upgrades).
- `deprecated-admin`: Commands, options, health checks or metrics marked deprecated or obsolete that no current peer uses.
- `dead-code`: C++ MDS code that cannot run at ceph.git commit 5f7e383ae6d.

An entry marked **Legacy** exists only for the cases its reasons name. **Legacy parts** marks an entry that current peers use but that also contains such a path. Both state who uses a path, with evidence from the current peer code.

Code references are `path:line` against ceph.git commit 5f7e383ae6d; `linux/...` paths refer to the Linux 7.3.0-rc6 snapshot (master commit af32da41b03). Feature IDs are stable: they are never renumbered or reused.
