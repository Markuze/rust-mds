# Features by peer

> Part of the [features catalog](README.md). Generated from [features.json](features.json): for each peer, the features it relies on (facts about the C++ MDS). Peers are the current versions of the [baseline](../README.md#baseline-current-peers). Legacy entries are left out here and listed in [legacy.md](legacy.md).

## kclient

Linux kernel CephFS client (fs/ceph). 412 features (8 legacy entries left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-002](01-cluster-membership-and-ha.md#f-clu-002) | MDSMap: per-file-system map and its wire encoding *(legacy parts)* | supporting | cluster-wire, client-wire |
| [F-CLU-003](01-cluster-membership-and-ha.md#f-clu-003) | Epoch model: FSMap, MDSMap and standby epochs, null map | operational | cluster-wire, client-wire |
| [F-CLU-004](01-cluster-membership-and-ha.md#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire |
| [F-CLU-005](01-cluster-membership-and-ha.md#f-clu-005) | FSMapUser: file system name directory for clients | operational | cluster-wire, client-wire |
| [F-CLU-006](01-cluster-membership-and-ha.md#f-clu-006) | Kernel client partial MDSMap decode *(legacy parts)* | supporting | client-wire |
| [F-CLU-008](01-cluster-membership-and-ha.md#f-clu-008) | FSMap filtering for file-system-restricted sessions | operational | cluster-wire, admin |
| [F-CLU-010](01-cluster-membership-and-ha.md#f-clu-010) | Multiple file systems: enable_multiple and the default file system *(legacy parts)* | operational | cluster-wire, admin |
| [F-CLU-011](01-cluster-membership-and-ha.md#f-clu-011) | Data pool list of a file system | extension | cluster-wire, client-wire, admin |
| [F-CLU-012](01-cluster-membership-and-ha.md#f-clu-012) | max_file_size published in the MDSMap | supporting | client-wire, admin |
| [F-CLU-023](01-cluster-membership-and-ha.md#f-clu-023) | Beacon acks and MDS-side lagginess (self-fencing deferral) | operational | cluster-wire, config |
| [F-CLU-029](01-cluster-membership-and-ha.md#f-clu-029) | Monitor-side laggy detection and its protections | operational | cluster-wire, config |
| [F-CLU-033](01-cluster-membership-and-ha.md#f-clu-033) | Affinity-driven swaps of healthy daemons | operational | cluster-wire |
| [F-CLU-034](01-cluster-membership-and-ha.md#f-clu-034) | Failed-rank and laggy-holder replacement | operational | cluster-wire |
| [F-CLU-035](01-cluster-membership-and-ha.md#f-clu-035) | Promotion into a rank and the initial rank state | operational | cluster-wire |
| [F-CLU-040](01-cluster-membership-and-ha.md#f-clu-040) | Growing the number of active ranks (max_mds) | operational | cluster-wire, admin |
| [F-CLU-041](01-cluster-membership-and-ha.md#f-clu-041) | Shrinking: monitor-driven up:stopping and FSMap::stop | operational | cluster-wire, admin |
| [F-CLU-047](01-cluster-membership-and-ha.md#f-clu-047) | fs rename and fs swap | operational | admin, cluster-wire |
| [F-CLU-049](01-cluster-membership-and-ha.md#f-clu-049) | Damaged ranks and mds repaired | operational | admin, cluster-wire, metrics |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-056](01-cluster-membership-and-ha.md#f-clu-056) | Rank sets and derived predicates | operational | cluster-wire |
| [F-CLU-060](01-cluster-membership-and-ha.md#f-clu-060) | MDSMap epoch waits for client requests | supporting | client-wire |
| [F-CLU-069](01-cluster-membership-and-ha.md#f-clu-069) | Standby-replay takeover | operational | cluster-wire |
| [F-CLU-071](01-cluster-membership-and-ha.md#f-clu-071) | Survivor reactions to a peer failing and recovering | supporting | mds-wire |
| [F-CLU-075](01-cluster-membership-and-ha.md#f-clu-075) | Ambiguous subtree import disambiguation | supporting | mds-wire, on-disk |
| [F-CLU-077](01-cluster-membership-and-ha.md#f-clu-077) | Rename-specific resolve handling | supporting | mds-wire, client-wire |
| [F-CLU-080](01-cluster-membership-and-ha.md#f-clu-080) | up:reconnect: the client reconnect window and its timeout | supporting | client-wire, config |
| [F-CLU-082](01-cluster-membership-and-ha.md#f-clu-082) | Recording caps and snaprealms from reconnect messages | supporting | client-wire |
| [F-CLU-084](01-cluster-membership-and-ha.md#f-clu-084) | Admission of replayed and retried requests during reconnect | supporting | client-wire |
| [F-CLU-087](01-cluster-membership-and-ha.md#f-clu-087) | Matching reconnected caps to inodes: OpenFileTable prefetch and open by number | supporting | on-disk, client-wire |
| [F-CLU-088](01-cluster-membership-and-ha.md#f-clu-088) | Moving reconnected caps between ranks (IMPORT/EXPORT during rejoin) | supporting | client-wire, mds-wire |
| [F-CLU-089](01-cluster-membership-and-ha.md#f-clu-089) | Forced session open during rejoin *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-090](01-cluster-membership-and-ha.md#f-clu-090) | Choosing lock states from reconnected caps | supporting | internal |
| [F-CLU-092](01-cluster-membership-and-ha.md#f-clu-092) | Snaprealm reconciliation and snapflush rebuild at the end of rejoin | supporting | client-wire |
| [F-CLU-094](01-cluster-membership-and-ha.md#f-clu-094) | Rejoin on a single-rank file system | supporting | client-wire, on-disk |
| [F-CLU-095](01-cluster-membership-and-ha.md#f-clu-095) | up:clientreplay: ordered re-execution of replayed requests | supporting | client-wire |
| [F-CLU-097](01-cluster-membership-and-ha.md#f-clu-097) | Activation (recovery_done and active_start) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-098](01-cluster-membership-and-ha.md#f-clu-098) | Leftover reconnect caps and cap reissue at activation | supporting | client-wire, metrics |
| [F-CLU-099](01-cluster-membership-and-ha.md#f-clu-099) | File size and mtime recovery for writers that did not come back | supporting | client-wire, on-disk, metrics |
| [F-CLU-100](01-cluster-membership-and-ha.md#f-clu-100) | Re-executing interrupted truncates after recovery | supporting | on-disk |
| [F-CLU-101](01-cluster-membership-and-ha.md#f-clu-101) | Purging orphaned preallocated and delegated inodes | supporting | on-disk |
| [F-CLU-102](01-cluster-membership-and-ha.md#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk |
| [F-CLU-104](01-cluster-membership-and-ha.md#f-clu-104) | Reporting a rank as DAMAGED | operational | cluster-wire, metrics |
| [F-CLU-107](01-cluster-membership-and-ha.md#f-clu-107) | Write error policy | operational | config, metrics |
| [F-CLU-109](01-cluster-membership-and-ha.md#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config |
| [F-CLU-113](01-cluster-membership-and-ha.md#f-clu-113) | Progress thread and rank waiter queues | internal | internal |
| [F-CLU-114](01-cluster-membership-and-ha.md#f-clu-114) | Dispatcher chain, message priorities and ordering | internal | internal |
| [F-CLU-115](01-cluster-membership-and-ha.md#f-clu-115) | Rank message validation and routing *(legacy parts)* | internal | client-wire, mds-wire |
| [F-CLU-116](01-cluster-membership-and-ha.md#f-clu-116) | Outbound messaging helpers and local delivery | internal | client-wire, mds-wire |
| [F-CLU-117](01-cluster-membership-and-ha.md#f-clu-117) | Tick and the shared timer | internal | config |
| [F-PROTO-002](02-client-sessions-and-protocol.md#f-proto-002) | msgr2.1 crc mode for client connections *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-003](02-client-sessions-and-protocol.md#f-proto-003) | msgr2.1 secure mode (AES-128-GCM) *(legacy parts)* | operational | client-wire, config |
| [F-PROTO-005](02-client-sessions-and-protocol.md#f-proto-005) | Address vector publication and server identity checks *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-006](02-client-sessions-and-protocol.md#f-proto-006) | Lossless stateful-server connection policy and replay of unacknowledged messages *(legacy parts)* | supporting | client-wire |
| [F-PROTO-007](02-client-sessions-and-protocol.md#f-proto-007) | Peer-reset detection and session-reset signalling *(legacy parts)* | supporting | client-wire |
| [F-PROTO-008](02-client-sessions-and-protocol.md#f-proto-008) | MDS entity name and rank identity on the wire *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-009](02-client-sessions-and-protocol.md#f-proto-009) | Keepalives and connection timeouts *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-011](02-client-sessions-and-protocol.md#f-proto-011) | cephx authorizer verification for service mds *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-012](02-client-sessions-and-protocol.md#f-proto-012) | Rotating mds service secrets | supporting | cluster-wire, config |
| [F-PROTO-015](02-client-sessions-and-protocol.md#f-proto-015) | Connection-to-session binding (identity is the cephx global id) | supporting | client-wire |
| [F-PROTO-016](02-client-sessions-and-protocol.md#f-proto-016) | Encoding primitives and versioned-struct rules | supporting | client-wire |
| [F-PROTO-017](02-client-sessions-and-protocol.md#f-proto-017) | Client message catalog and peer-type filtering | supporting | client-wire, mds-wire |
| [F-PROTO-020](02-client-sessions-and-protocol.md#f-proto-020) | CephFS feature bitmap exchange at session open *(legacy parts)* | supporting | client-wire |
| [F-PROTO-021](02-client-sessions-and-protocol.md#f-proto-021) | Per-bit gated behaviour and feature names *(legacy parts)* | supporting | client-wire, admin |
| [F-PROTO-023](02-client-sessions-and-protocol.md#f-proto-023) | required_client_features enforcement and min_compat_client *(legacy parts)* | operational | client-wire, cluster-wire, admin |
| [F-PROTO-024](02-client-sessions-and-protocol.md#f-proto-024) | Connection-level feature bits consulted by the client protocol *(legacy parts)* | supporting | client-wire |
| [F-PROTO-025](02-client-sessions-and-protocol.md#f-proto-025) | Feature bit 22 numbering conflict (Ceph CHARMAP vs kernel SUBVOLUME_METRICS) *(legacy parts)* | extension | client-wire |
| [F-PROTO-026](02-client-sessions-and-protocol.md#f-proto-026) | Client generations and candidate minimum client floors *(legacy parts)* | operational | client-wire, admin |
| [F-PROTO-027](02-client-sessions-and-protocol.md#f-proto-027) | Session state machine | supporting | client-wire |
| [F-PROTO-028](02-client-sessions-and-protocol.md#f-proto-028) | Session open handshake (REQUEST_OPEN, journaled open, OPEN) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-PROTO-029](02-client-sessions-and-protocol.md#f-proto-029) | Session open admission checks and rejection reasons *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-PROTO-030](02-client-sessions-and-protocol.md#f-proto-030) | REJECT encoding and blocklist signalling to clients *(legacy parts)* | supporting | client-wire |
| [F-PROTO-031](02-client-sessions-and-protocol.md#f-proto-031) | MClientSession encoding and the ops the MDS sends *(legacy parts)* | supporting | client-wire |
| [F-PROTO-032](02-client-sessions-and-protocol.md#f-proto-032) | Session op dispatch and rank-state gating | supporting | client-wire |
| [F-PROTO-034](02-client-sessions-and-protocol.md#f-proto-034) | Client metadata capture and use *(legacy parts)* | operational | client-wire, admin, on-disk |
| [F-PROTO-037](02-client-sessions-and-protocol.md#f-proto-037) | Duplicate REQUEST_OPEN answered with OPEN (NOTIFY_SESSION_STATE) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-038](02-client-sessions-and-protocol.md#f-proto-038) | FORCE_RO: read-only MDS notification | supporting | client-wire |
| [F-PROTO-039](02-client-sessions-and-protocol.md#f-proto-039) | FLUSHMSG / FLUSHMSG_ACK session barrier *(legacy parts)* | supporting | client-wire |
| [F-PROTO-040](02-client-sessions-and-protocol.md#f-proto-040) | REQUEST_FLUSH_MDLOG and the client sync paths | supporting | client-wire |
| [F-PROTO-041](02-client-sessions-and-protocol.md#f-proto-041) | Force-opened sessions (imports, rejoin, cross-MDS rename) | supporting | client-wire, mds-wire, on-disk |
| [F-PROTO-042](02-client-sessions-and-protocol.md#f-proto-042) | Cap renewal (REQUEST_RENEWCAPS / RENEWCAPS) | supporting | client-wire |
| [F-PROTO-043](02-client-sessions-and-protocol.md#f-proto-043) | Stale session detection and the STALE message | supporting | client-wire |
| [F-PROTO-045](02-client-sessions-and-protocol.md#f-proto-045) | Session autoclose eviction | supporting | client-wire, cluster-wire, config |
| [F-PROTO-048](02-client-sessions-and-protocol.md#f-proto-048) | Cap-revoke non-responder eviction | operational | config, metrics |
| [F-PROTO-049](02-client-sessions-and-protocol.md#f-proto-049) | Client-initiated close with push-sequence handshake | supporting | client-wire, on-disk |
| [F-PROTO-050](02-client-sessions-and-protocol.md#f-proto-050) | Counted messages and the session push sequence | supporting | client-wire |
| [F-PROTO-051](02-client-sessions-and-protocol.md#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin |
| [F-PROTO-052](02-client-sessions-and-protocol.md#f-proto-052) | OSDMap blocklist-driven session kill | supporting | client-wire, cluster-wire |
| [F-PROTO-053](02-client-sessions-and-protocol.md#f-proto-053) | Eviction of sessions with oversized metadata | operational | config, metrics |
| [F-PROTO-054](02-client-sessions-and-protocol.md#f-proto-054) | Session termination at rank shutdown | operational | client-wire, cluster-wire |
| [F-PROTO-055](02-client-sessions-and-protocol.md#f-proto-055) | Reconnect trigger and client-side ordering | supporting | client-wire |
| [F-PROTO-056](02-client-sessions-and-protocol.md#f-proto-056) | MClientReconnect encodings (v1 to v5, multi-message) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-057](02-client-sessions-and-protocol.md#f-proto-057) | Reconnect processing and outcomes *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-061](02-client-sessions-and-protocol.md#f-proto-061) | Snap realm reconnect | extension | client-wire |
| [F-PROTO-062](02-client-sessions-and-protocol.md#f-proto-062) | Client reactions to MDSMap rank changes | supporting | cluster-wire |
| [F-PROTO-065](02-client-sessions-and-protocol.md#f-proto-065) | MClientRequest versioned head (head v1, v2, v3) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-067](02-client-sessions-and-protocol.md#f-proto-067) | Request message tail: stamp, gid_list, alternate_name, fscrypt fields *(legacy parts)* | supporting | client-wire |
| [F-PROTO-068](02-client-sessions-and-protocol.md#f-proto-068) | Request flags REPLAY, WANT_DENTRY and ASYNC | supporting | client-wire |
| [F-PROTO-069](02-client-sessions-and-protocol.md#f-proto-069) | Request argument union per op | supporting | client-wire |
| [F-PROTO-070](02-client-sessions-and-protocol.md#f-proto-070) | Embedded cap and dentry-lease release records | supporting | client-wire |
| [F-PROTO-071](02-client-sessions-and-protocol.md#f-proto-071) | Request data segment per op | supporting | client-wire |
| [F-PROTO-072](02-client-sessions-and-protocol.md#f-proto-072) | Path encoding on the wire | supporting | client-wire |
| [F-PROTO-073](02-client-sessions-and-protocol.md#f-proto-073) | Op code numbering and the write bit | supporting | client-wire |
| [F-PROTO-074](02-client-sessions-and-protocol.md#f-proto-074) | Client-chosen inode number (head.ino) | supporting | client-wire |
| [F-PROTO-075](02-client-sessions-and-protocol.md#f-proto-075) | Client-side rank selection and the hints the MDS provides | supporting | client-wire, config |
| [F-PROTO-076](02-client-sessions-and-protocol.md#f-proto-076) | Request forwarding (MClientRequestForward) | supporting | client-wire |
| [F-PROTO-077](02-client-sessions-and-protocol.md#f-proto-077) | Retry counters and attempt limits (32BITS_RETRY_FWD) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-078](02-client-sessions-and-protocol.md#f-proto-078) | Request identity, tid ordering and requests without a session | supporting | client-wire |
| [F-PROTO-079](02-client-sessions-and-protocol.md#f-proto-079) | MClientReply head and error codes | supporting | client-wire |
| [F-PROTO-080](02-client-sessions-and-protocol.md#f-proto-080) | Reply trace layout | supporting | client-wire |
| [F-PROTO-081](02-client-sessions-and-protocol.md#f-proto-081) | InodeStat versioned encoding (v9, REPLY_ENCODING) | supporting | client-wire |
| [F-PROTO-083](02-client-sessions-and-protocol.md#f-proto-083) | DirStat and LeaseStat records *(legacy parts)* | supporting | client-wire |
| [F-PROTO-084](02-client-sessions-and-protocol.md#f-proto-084) | Caps, xattrs and inline data carried inside InodeStat *(legacy parts)* | supporting | client-wire |
| [F-PROTO-085](02-client-sessions-and-protocol.md#f-proto-085) | Snap trace encoding and format selection | extension | client-wire |
| [F-PROTO-086](02-client-sessions-and-protocol.md#f-proto-086) | Readdir and LSSNAP reply format and ordering contract *(legacy parts)* | supporting | client-wire |
| [F-PROTO-087](02-client-sessions-and-protocol.md#f-proto-087) | CREATE reply: created inode and delegated inode ranges *(legacy parts)* | supporting | client-wire |
| [F-PROTO-088](02-client-sessions-and-protocol.md#f-proto-088) | GETFILELOCK, GETVXATTR and FILE_BLOCKDIFF extra payloads | supporting | client-wire |
| [F-PROTO-089](02-client-sessions-and-protocol.md#f-proto-089) | Early (unsafe) and safe replies | supporting | client-wire |
| [F-PROTO-090](02-client-sessions-and-protocol.md#f-proto-090) | Reply content invariants enforced by clients | supporting | client-wire |
| [F-PROTO-091](02-client-sessions-and-protocol.md#f-proto-091) | MClientCaps encoding (v14, xattrs in the middle segment) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-092](02-client-sessions-and-protocol.md#f-proto-092) | MClientCapRelease encoding and the OSD epoch barrier | supporting | client-wire |
| [F-PROTO-093](02-client-sessions-and-protocol.md#f-proto-093) | MClientLease encoding | supporting | client-wire |
| [F-PROTO-094](02-client-sessions-and-protocol.md#f-proto-094) | MClientSnap encoding | extension | client-wire |
| [F-PROTO-095](02-client-sessions-and-protocol.md#f-proto-095) | MClientQuota encoding *(legacy parts)* | extension | client-wire |
| [F-PROTO-096](02-client-sessions-and-protocol.md#f-proto-096) | MClientMetrics encoding and client send rules | operational | client-wire, metrics |
| [F-PROTO-098](02-client-sessions-and-protocol.md#f-proto-098) | Mount flow requirements | supporting | client-wire, cluster-wire |
| [F-PROTO-099](02-client-sessions-and-protocol.md#f-proto-099) | Inode-number addressed access (NFS file handles, #ino paths, LOOKUPINO) | supporting | client-wire |
| [F-PROTO-100](02-client-sessions-and-protocol.md#f-proto-100) | Inode number ranges clients accept | supporting | client-wire |
| [F-CAPS-001](03-caps-leases-and-locking.md#f-caps-001) | Cap bit model and composite masks | supporting | client-wire |
| [F-CAPS-002](03-caps-leases-and-locking.md#f-caps-002) | Open mode to wanted caps | supporting | client-wire |
| [F-CAPS-003](03-caps-leases-and-locking.md#f-caps-003) | Capability record, cap ids and sequence numbers | supporting | client-wire |
| [F-CAPS-004](03-caps-leases-and-locking.md#f-caps-004) | Lock-state-derived allowed caps *(legacy parts)* | supporting | client-wire |
| [F-CAPS-008](03-caps-leases-and-locking.md#f-caps-008) | Lazy I/O cap (Fl) | extension | client-wire |
| [F-CAPS-009](03-caps-leases-and-locking.md#f-caps-009) | Caps in request replies *(legacy parts)* | supporting | client-wire |
| [F-CAPS-010](03-caps-leases-and-locking.md#f-caps-010) | New caps on open, create, mknod and mkdir | supporting | client-wire |
| [F-CAPS-011](03-caps-leases-and-locking.md#f-caps-011) | Cap grant and revoke messages (GRANT/REVOKE) | supporting | client-wire |
| [F-CAPS-013](03-caps-leases-and-locking.md#f-caps-013) | Client cap message processing (handle_client_caps) | supporting | client-wire |
| [F-CAPS-014](03-caps-leases-and-locking.md#f-caps-014) | Wanted caps tracking (issue_seq rule) | supporting | client-wire |
| [F-CAPS-015](03-caps-leases-and-locking.md#f-caps-015) | Xattr blob and inline data carried on caps *(legacy parts)* | supporting | client-wire |
| [F-CAPS-016](03-caps-leases-and-locking.md#f-caps-016) | Truncate notification (CEPH_CAP_OP_TRUNC) | supporting | client-wire |
| [F-CAPS-018](03-caps-leases-and-locking.md#f-caps-018) | Dirty metadata flush and FLUSH_ACK *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CAPS-019](03-caps-leases-and-locking.md#f-caps-019) | Flush idempotency (completed_flushes, oldest_flush_tid) | supporting | client-wire, on-disk, metrics |
| [F-CAPS-020](03-caps-leases-and-locking.md#f-caps-020) | Synchronous flush and journal-flush heuristics | supporting | client-wire, config |
| [F-CAPS-021](03-caps-leases-and-locking.md#f-caps-021) | max_size and per-client writeable ranges (client_ranges) | supporting | client-wire, on-disk, config |
| [F-CAPS-022](03-caps-leases-and-locking.md#f-caps-022) | File size recovery after loss of a writer | supporting | cluster-wire, on-disk |
| [F-CAPS-023](03-caps-leases-and-locking.md#f-caps-023) | Snapshot COW of cap state and SNAP_SYNC gating | extension | on-disk |
| [F-CAPS-024](03-caps-leases-and-locking.md#f-caps-024) | FLUSHSNAP / FLUSHSNAP_ACK protocol | extension | client-wire, on-disk |
| [F-CAPS-025](03-caps-leases-and-locking.md#f-caps-025) | Null snapflush inference | extension | client-wire, on-disk |
| [F-CAPS-026](03-caps-leases-and-locking.md#f-caps-026) | Snapflush nudge | extension | config |
| [F-CAPS-027](03-caps-leases-and-locking.md#f-caps-027) | XLOCKSNAP: truncate with pending snapflush | extension | internal |
| [F-CAPS-028](03-caps-leases-and-locking.md#f-caps-028) | Batched cap release (MClientCapRelease) | supporting | client-wire |
| [F-CAPS-029](03-caps-leases-and-locking.md#f-caps-029) | Cap and dentry-lease releases embedded in requests | supporting | client-wire |
| [F-CAPS-030](03-caps-leases-and-locking.md#f-caps-030) | Stale-session cap handling *(legacy parts)* | supporting | client-wire, config |
| [F-CAPS-032](03-caps-leases-and-locking.md#f-caps-032) | Cache-pressure cap recall (CEPH_SESSION_RECALL_STATE) | operational | client-wire, metrics, config, admin |
| [F-CAPS-034](03-caps-leases-and-locking.md#f-caps-034) | Cap re-establishment after MDS failover (cap side) | supporting | client-wire |
| [F-CAPS-035](03-caps-leases-and-locking.md#f-caps-035) | Replica caps and MInodeFileCaps | internal | mds-wire |
| [F-CAPS-036](03-caps-leases-and-locking.md#f-caps-036) | Cap migration to clients (EXPORT / IMPORT) | supporting | client-wire, mds-wire |
| [F-CAPS-038](03-caps-leases-and-locking.md#f-caps-038) | Dentry lease issue | supporting | client-wire |
| [F-CAPS-039](03-caps-leases-and-locking.md#f-caps-039) | Negative dentry leases | supporting | client-wire |
| [F-CAPS-040](03-caps-leases-and-locking.md#f-caps-040) | CEPH_LEASE_PRIMARY_LINK hint | supporting | client-wire |
| [F-CAPS-042](03-caps-leases-and-locking.md#f-caps-042) | Lease renewal (CEPH_MDS_LEASE_RENEW) | supporting | client-wire |
| [F-CAPS-043](03-caps-leases-and-locking.md#f-caps-043) | Lease revocation on dentry change | supporting | client-wire |
| [F-CAPS-044](03-caps-leases-and-locking.md#f-caps-044) | Lease expiry and cleanup | supporting | client-wire, config |
| [F-CAPS-045](03-caps-leases-and-locking.md#f-caps-045) | Infinite leases on snapshot listings | extension | client-wire |
| [F-CAPS-046](03-caps-leases-and-locking.md#f-caps-046) | Directory-wide dentry validity via Fs | supporting | client-wire |
| [F-CAPS-047](03-caps-leases-and-locking.md#f-caps-047) | Directory-operation cap bits Dc and Dr | supporting | client-wire |
| [F-CAPS-048](03-caps-leases-and-locking.md#f-caps-048) | Lock cache creation | internal | config |
| [F-CAPS-049](03-caps-leases-and-locking.md#f-caps-049) | Lock cache attachment | internal | internal |
| [F-CAPS-050](03-caps-leases-and-locking.md#f-caps-050) | Lock cache invalidation and lifecycle | internal | internal |
| [F-CAPS-051](03-caps-leases-and-locking.md#f-caps-051) | Forced auth pins for lock-cache requests | internal | internal |
| [F-CAPS-052](03-caps-leases-and-locking.md#f-caps-052) | Forcing async-operation flush for readers | supporting | internal |
| [F-CAPS-053](03-caps-leases-and-locking.md#f-caps-053) | Per-session inode-number preallocation | internal | on-disk, config |
| [F-CAPS-054](03-caps-leases-and-locking.md#f-caps-054) | Inode-number delegation to clients | supporting | client-wire, config |
| [F-CAPS-057](03-caps-leases-and-locking.md#f-caps-057) | Asynchronous create (kclient) | supporting | client-wire |
| [F-CAPS-058](03-caps-leases-and-locking.md#f-caps-058) | Asynchronous unlink (kclient) | supporting | client-wire |
| [F-CAPS-059](03-caps-leases-and-locking.md#f-caps-059) | Replay of unsafe and async creates after failover | supporting | client-wire, config |
| [F-CAPS-067](03-caps-leases-and-locking.md#f-caps-067) | Gather engine (eval_gather) and cap revocation gather | supporting | internal |
| [F-CAPS-081](03-caps-leases-and-locking.md#f-caps-081) | Implicit quiescelock wrlock | extension | internal |
| [F-CAPS-092](03-caps-leases-and-locking.md#f-caps-092) | XLOCKDONE: read-your-own-unsafe-writes | supporting | client-wire, config |
| [F-NS-001](04-namespace-and-posix-operations.md#f-ns-001) | Client op set and dispatch switch | required | client-wire |
| [F-NS-002](04-namespace-and-posix-operations.md#f-ns-002) | Request intake gating by rank state and session | supporting | client-wire |
| [F-NS-003](04-namespace-and-posix-operations.md#f-ns-003) | Request identity and in-flight duplicate detection | supporting | client-wire |
| [F-NS-010](04-namespace-and-posix-operations.md#f-ns-010) | Journaled mutation commit and safe reply | supporting | client-wire, on-disk |
| [F-NS-011](04-namespace-and-posix-operations.md#f-ns-011) | Early (unsafe) replies | supporting | client-wire, config |
| [F-NS-012](04-namespace-and-posix-operations.md#f-ns-012) | Completed-request table and idempotent retries/replays | supporting | client-wire, on-disk, metrics |
| [F-NS-013](04-namespace-and-posix-operations.md#f-ns-013) | Replay of unsafe requests after an MDS restart | supporting | client-wire |
| [F-NS-014](04-namespace-and-posix-operations.md#f-ns-014) | Per-op reply trace contents | supporting | client-wire |
| [F-NS-016](04-namespace-and-posix-operations.md#f-ns-016) | On-demand journal flush for fsync, syncfs and sync writes | required | client-wire |
| [F-NS-017](04-namespace-and-posix-operations.md#f-ns-017) | Read-only and snapshot EROFS gates | required | client-wire |
| [F-NS-018](04-namespace-and-posix-operations.md#f-ns-018) | Full metadata pool gate | required | client-wire, cluster-wire |
| [F-NS-019](04-namespace-and-posix-operations.md#f-ns-019) | Create/remove name gates and their error order | required | client-wire |
| [F-NS-020](04-namespace-and-posix-operations.md#f-ns-020) | Directory entry limits (ENOSPC) | operational | client-wire, config |
| [F-NS-021](04-namespace-and-posix-operations.md#f-ns-021) | Permission checking split: clients enforce POSIX, the MDS enforces auth caps *(legacy parts)* | required | client-wire |
| [F-NS-022](04-namespace-and-posix-operations.md#f-ns-022) | ESTALE recovery of unknown inode numbers | supporting | client-wire, mds-wire, on-disk |
| [F-NS-023](04-namespace-and-posix-operations.md#f-ns-023) | GETATTR (stat by inode) | required | client-wire |
| [F-NS-024](04-namespace-and-posix-operations.md#f-ns-024) | LOOKUP of one name in a directory | required | client-wire |
| [F-NS-025](04-namespace-and-posix-operations.md#f-ns-025) | Negative lookup replies (null-dentry traces) | supporting | client-wire |
| [F-NS-026](04-namespace-and-posix-operations.md#f-ns-026) | LOOKUPINO and LOOKUPHASH (resolve an inode number) | supporting | client-wire, mds-wire, on-disk |
| [F-NS-027](04-namespace-and-posix-operations.md#f-ns-027) | LOOKUPPARENT (parent directory of an inode) | supporting | client-wire |
| [F-NS-028](04-namespace-and-posix-operations.md#f-ns-028) | LOOKUPNAME (name of an inode under its parent) | supporting | client-wire |
| [F-NS-029](04-namespace-and-posix-operations.md#f-ns-029) | Reads at a snapshot id (LOOKUPSNAP, snapshot LOOKUPINO, open and readdir in snapshots) | extension | client-wire |
| [F-NS-030](04-namespace-and-posix-operations.md#f-ns-030) | OPEN of an existing file or directory *(legacy parts)* | required | client-wire |
| [F-NS-031](04-namespace-and-posix-operations.md#f-ns-031) | O_TRUNC on open | required | client-wire, on-disk |
| [F-NS-032](04-namespace-and-posix-operations.md#f-ns-032) | READDIR paging of one dirfrag in hash order *(legacy parts)* | required | client-wire |
| [F-NS-033](04-namespace-and-posix-operations.md#f-ns-033) | Readdir across dirfrags: frag selection and adjustment *(legacy parts)* | required | client-wire |
| [F-NS-034](04-namespace-and-posix-operations.md#f-ns-034) | Readdir cap-acquisition throttle | operational | config, metrics |
| [F-NS-035](04-namespace-and-posix-operations.md#f-ns-035) | CREATE (open with O_CREAT, optionally O_EXCL) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-036](04-namespace-and-posix-operations.md#f-ns-036) | MKNOD (regular file, FIFO, device, socket) | required | client-wire, on-disk |
| [F-NS-037](04-namespace-and-posix-operations.md#f-ns-037) | MKDIR *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-038](04-namespace-and-posix-operations.md#f-ns-038) | SYMLINK (and readlink without an MDS op) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-039](04-namespace-and-posix-operations.md#f-ns-039) | New inode initialization *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-040](04-namespace-and-posix-operations.md#f-ns-040) | Inode number allocation (session preallocation and InoTable) | supporting | on-disk, config |
| [F-NS-041](04-namespace-and-posix-operations.md#f-ns-041) | Client-specified inode number (head.ino) on create | supporting | client-wire |
| [F-NS-042](04-namespace-and-posix-operations.md#f-ns-042) | File layout selection and validation at create | extension | client-wire |
| [F-NS-043](04-namespace-and-posix-operations.md#f-ns-043) | Parent directory updates on link and unlink | required | client-wire, on-disk |
| [F-NS-044](04-namespace-and-posix-operations.md#f-ns-044) | LINK (hard link) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-045](04-namespace-and-posix-operations.md#f-ns-045) | UNLINK of a primary link | required | client-wire, on-disk |
| [F-NS-046](04-namespace-and-posix-operations.md#f-ns-046) | UNLINK of a remote (hard-link) dentry | required | client-wire, on-disk |
| [F-NS-047](04-namespace-and-posix-operations.md#f-ns-047) | RMDIR with emptiness checks | required | client-wire, on-disk |
| [F-NS-048](04-namespace-and-posix-operations.md#f-ns-048) | Unlinked-but-open inodes stay usable until the last cap is released | required | on-disk |
| [F-NS-049](04-namespace-and-posix-operations.md#f-ns-049) | RENAME of a file or directory (move, same-inode no-op) | required | client-wire, on-disk |
| [F-NS-050](04-namespace-and-posix-operations.md#f-ns-050) | RENAME over an existing target | required | client-wire, on-disk |
| [F-NS-051](04-namespace-and-posix-operations.md#f-ns-051) | RENAME validation and error order *(legacy parts)* | required | client-wire |
| [F-NS-053](04-namespace-and-posix-operations.md#f-ns-053) | Which rank serves each op (auth requirement, forward or discover) | supporting | client-wire, mds-wire, config |
| [F-NS-057](04-namespace-and-posix-operations.md#f-ns-057) | Inode authority and cap migration inside a cross-rank rename *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-NS-058](04-namespace-and-posix-operations.md#f-ns-058) | SETATTR request: mask, locks, error order and common tail *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-059](04-namespace-and-posix-operations.md#f-ns-059) | chmod and chown, with set-user/group-ID clearing | required | client-wire |
| [F-NS-060](04-namespace-and-posix-operations.md#f-ns-060) | Explicit times (utimes) and btime, with time_warp_seq *(legacy parts)* | required | client-wire |
| [F-NS-061](04-namespace-and-posix-operations.md#f-ns-061) | Size growth through SETATTR | required | client-wire |
| [F-NS-062](04-namespace-and-posix-operations.md#f-ns-062) | Truncate (shrink) with asynchronous OSD truncation | required | client-wire, cluster-wire, on-disk |
| [F-NS-063](04-namespace-and-posix-operations.md#f-ns-063) | Timestamps from the client clock and change_attr | required | client-wire, on-disk |
| [F-NS-064](04-namespace-and-posix-operations.md#f-ns-064) | fscrypt inode fields and last-block truncate *(legacy parts)* | extension | client-wire, on-disk, config |
| [F-NS-065](04-namespace-and-posix-operations.md#f-ns-065) | Layout ops SETLAYOUT and SETDIRLAYOUT (kernel layout ioctls) | extension | client-wire |
| [F-NS-066](04-namespace-and-posix-operations.md#f-ns-066) | SETXATTR and RMXATTR of stored xattrs | supporting | client-wire, on-disk |
| [F-NS-067](04-namespace-and-posix-operations.md#f-ns-067) | Total xattr size limit (max_xattr_size) | supporting | client-wire, cluster-wire, admin |
| [F-NS-068](04-namespace-and-posix-operations.md#f-ns-068) | xattr blob delivery to clients | supporting | client-wire |
| [F-NS-069](04-namespace-and-posix-operations.md#f-ns-069) | POSIX ACLs stored as opaque xattrs | supporting | client-wire, on-disk |
| [F-NS-071](04-namespace-and-posix-operations.md#f-ns-071) | Virtual xattr set/remove path (common rules, p flag) | extension | client-wire, admin |
| [F-NS-072](04-namespace-and-posix-operations.md#f-ns-072) | Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*) *(legacy parts)* | extension | client-wire, cluster-wire |
| [F-NS-073](04-namespace-and-posix-operations.md#f-ns-073) | GETVXATTR op | extension | client-wire |
| [F-NS-074](04-namespace-and-posix-operations.md#f-ns-074) | Client-synthesized ceph.* vxattrs (InodeStat contract) | extension | client-wire |
| [F-NS-075](04-namespace-and-posix-operations.md#f-ns-075) | Case-insensitive and normalized directories (charmap) *(legacy parts)* | extension | client-wire, on-disk |
| [F-NS-076](04-namespace-and-posix-operations.md#f-ns-076) | alternate_name on dentries | extension | client-wire, on-disk |
| [F-NS-077](04-namespace-and-posix-operations.md#f-ns-077) | SETFILELOCK: POSIX fcntl byte-range locks | required | client-wire |
| [F-NS-078](04-namespace-and-posix-operations.md#f-ns-078) | BSD flock whole-file locks | extension | client-wire |
| [F-NS-079](04-namespace-and-posix-operations.md#f-ns-079) | Blocking lock waits and wake-ups | required | client-wire |
| [F-NS-080](04-namespace-and-posix-operations.md#f-ns-080) | Lock wait interruption (FCNTL_INTR / FLOCK_INTR) | required | client-wire |
| [F-NS-081](04-namespace-and-posix-operations.md#f-ns-081) | fcntl deadlock detection (EDEADLK) | required | client-wire |
| [F-NS-082](04-namespace-and-posix-operations.md#f-ns-082) | GETFILELOCK (F_GETLK) | required | client-wire |
| [F-NS-083](04-namespace-and-posix-operations.md#f-ns-083) | Lock cleanup on cap removal and session end | required | client-wire |
| [F-NS-084](04-namespace-and-posix-operations.md#f-ns-084) | Lock reinstatement after an MDS restart *(legacy parts)* | required | client-wire |
| [F-NS-085](04-namespace-and-posix-operations.md#f-ns-085) | Directory link count and size presentation | required | client-wire |
| [F-NS-086](04-namespace-and-posix-operations.md#f-ns-086) | Access time is not maintained by the MDS | required | client-wire |
| [F-NS-087](04-namespace-and-posix-operations.md#f-ns-087) | Timestamp range limited to unsigned 32-bit seconds | required | client-wire, on-disk |
| [F-NS-088](04-namespace-and-posix-operations.md#f-ns-088) | Limits enforced only by clients, and no hard-link limit | required | client-wire |
| [F-NS-089](04-namespace-and-posix-operations.md#f-ns-089) | Unsupported Linux extensions: renameat2 flags and O_TMPFILE | required | client-wire |
| [F-NS-090](04-namespace-and-posix-operations.md#f-ns-090) | Data-path coherency departures outside the MDS | required | client-wire |
| [F-NS-091](04-namespace-and-posix-operations.md#f-ns-091) | statfs is answered by the monitors | required | cluster-wire |
| [F-CACHE-001](05-metadata-cache-and-dirfrags.md#f-cache-001) | Inode record field set (inode_t) | required | on-disk, client-wire |
| [F-CACHE-003](05-metadata-cache-and-dirfrags.md#f-cache-003) | Refusal of client operations on private and system inodes | supporting | client-wire |
| [F-CACHE-004](05-metadata-cache-and-dirfrags.md#f-cache-004) | Timestamps, time_warp_seq and change_attr *(legacy parts)* | required | client-wire, on-disk |
| [F-CACHE-006](05-metadata-cache-and-dirfrags.md#f-cache-006) | Truncation state fields | required | client-wire, on-disk |
| [F-CACHE-007](05-metadata-cache-and-dirfrags.md#f-cache-007) | File layout field and layout inheritance *(legacy parts)* | extension | client-wire, on-disk |
| [F-CACHE-008](05-metadata-cache-and-dirfrags.md#f-cache-008) | Inline data *(legacy parts)* | extension | client-wire, on-disk |
| [F-CACHE-009](05-metadata-cache-and-dirfrags.md#f-cache-009) | fscrypt metadata fields | extension | client-wire, on-disk |
| [F-CACHE-010](05-metadata-cache-and-dirfrags.md#f-cache-010) | Inode version and client-visible version | supporting | client-wire |
| [F-CACHE-011](05-metadata-cache-and-dirfrags.md#f-cache-011) | Snapshot-range identity of inodes and dentries | extension | on-disk |
| [F-CACHE-014](05-metadata-cache-and-dirfrags.md#f-cache-014) | Projected dentry linkage (primary, remote, null) | supporting | on-disk |
| [F-CACHE-023](05-metadata-cache-and-dirfrags.md#f-cache-023) | Directory completeness and authoritative negative lookups | supporting | internal |
| [F-CACHE-041](05-metadata-cache-and-dirfrags.md#f-cache-041) | Path traversal (path_traverse) | required | client-wire |
| [F-CACHE-044](05-metadata-cache-and-dirfrags.md#f-cache-044) | Forward-or-discover for non-auth objects | supporting | client-wire, config |
| [F-CACHE-045](05-metadata-cache-and-dirfrags.md#f-cache-045) | Hard-link target resolution | required | client-wire |
| [F-CACHE-047](05-metadata-cache-and-dirfrags.md#f-cache-047) | Lookup by inode number (open_ino) | supporting | client-wire, on-disk, mds-wire |
| [F-CACHE-050](05-metadata-cache-and-dirfrags.md#f-cache-050) | ESTALE recovery via find_ino_peers | supporting | client-wire, mds-wire |
| [F-CACHE-060](05-metadata-cache-and-dirfrags.md#f-cache-060) | Pin reasons that block eviction | operational | internal |
| [F-CACHE-071](05-metadata-cache-and-dirfrags.md#f-cache-071) | Root and system inode initial attributes and default layouts | required | client-wire, on-disk, config |
| [F-CACHE-072](05-metadata-cache-and-dirfrags.md#f-cache-072) | Global snaprealm dummy inode | supporting | client-wire |
| [F-CACHE-074](05-metadata-cache-and-dirfrags.md#f-cache-074) | Fragment identifier and fragment tree (frag_t, fragtree_t) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CACHE-075](05-metadata-cache-and-dirfrags.md#f-cache-075) | Dentry name hash per directory (dl_dir_hash) *(legacy parts)* | supporting | client-wire, on-disk, config |
| [F-CACHE-076](05-metadata-cache-and-dirfrags.md#f-cache-076) | Fragtree and dir_layout in InodeStat *(legacy parts)* | supporting | client-wire |
| [F-CACHE-077](05-metadata-cache-and-dirfrags.md#f-cache-077) | Hash-ordered dentry storage and readdir order *(legacy parts)* | required | client-wire |
| [F-CACHE-078](05-metadata-cache-and-dirfrags.md#f-cache-078) | Readdir fragment mapping and resume across refragmentation | required | client-wire |
| [F-CACHE-079](05-metadata-cache-and-dirfrags.md#f-cache-079) | DirStat delegation hints *(legacy parts)* | supporting | client-wire, config |
| [F-CACHE-080](05-metadata-cache-and-dirfrags.md#f-cache-080) | Per-dirfrag size limit (ENOSPC) | operational | client-wire, config |
| [F-CACHE-081](05-metadata-cache-and-dirfrags.md#f-cache-081) | Per-directory entry limit | operational | client-wire, config |
| [F-CACHE-097](05-metadata-cache-and-dirfrags.md#f-cache-097) | Effects of a fragment freeze on clients | supporting | client-wire |
| [F-STORE-007](06-journal-and-storage.md#f-store-007) | Per-rank write-ahead metadata journal with one submission order | supporting | on-disk |
| [F-STORE-008](06-journal-and-storage.md#f-store-008) | Group commit: flush and wait_for_safe | supporting | internal |
| [F-STORE-009](06-journal-and-storage.md#f-store-009) | Periodic journal flush on tick | supporting | config |
| [F-STORE-011](06-journal-and-storage.md#f-store-011) | Ordered completions and the safe position | supporting | internal |
| [F-STORE-012](06-journal-and-storage.md#f-store-012) | Safe reply only after the journal commit (early unsafe reply) | supporting | client-wire |
| [F-STORE-024](06-journal-and-storage.md#f-store-024) | Snapshot COW images in the journal | extension | on-disk |
| [F-STORE-026](06-journal-and-storage.md#f-store-026) | EUpdate: generic mutation record | supporting | on-disk |
| [F-STORE-027](06-journal-and-storage.md#f-store-027) | EOpen: open-file journaling | supporting | on-disk |
| [F-STORE-028](06-journal-and-storage.md#f-store-028) | Session events: ESession and ESessions *(legacy parts)* | supporting | on-disk |
| [F-STORE-029](06-journal-and-storage.md#f-store-029) | Completed client request ids in journal and SessionMap | supporting | on-disk |
| [F-STORE-030](06-journal-and-storage.md#f-store-030) | Completed cap flush ids in journal and SessionMap | supporting | on-disk |
| [F-STORE-046](06-journal-and-storage.md#f-store-046) | Special inode numbers and per-rank ranges *(legacy parts)* | supporting | client-wire, on-disk |
| [F-STORE-058](06-journal-and-storage.md#f-store-058) | Backtrace parent xattr on object 0 | supporting | on-disk |
| [F-STORE-062](06-journal-and-storage.md#f-store-062) | Lookup by inode number through backtraces (open_ino storage path) | supporting | client-wire, on-disk |
| [F-STORE-064](06-journal-and-storage.md#f-store-064) | Data object naming and striping shared with clients | supporting | client-wire, on-disk |
| [F-STORE-065](06-journal-and-storage.md#f-store-065) | Data-object truncation (truncate start/finish and TRIMTRUNC) | required | on-disk, cluster-wire |
| [F-STORE-066](06-journal-and-storage.md#f-store-066) | File size and mtime recovery by probing data objects | supporting | cluster-wire |
| [F-STORE-070](06-journal-and-storage.md#f-store-070) | Inode number allocation in create paths | supporting | on-disk, client-wire |
| [F-STORE-072](06-journal-and-storage.md#f-store-072) | Release of preallocated and delegated numbers at session close (EPurged) | internal | on-disk, cluster-wire |
| [F-STORE-073](06-journal-and-storage.md#f-store-073) | Inode numbers are never reused after deletion | required | client-wire, on-disk |
| [F-STORE-075](06-journal-and-storage.md#f-store-075) | SessionMap object | supporting | on-disk, config |
| [F-STORE-079](06-journal-and-storage.md#f-store-079) | Table server placement on rank 0 | extension | cluster-wire |
| [F-STORE-089](06-journal-and-storage.md#f-store-089) | OpenFileTable prefetch during rejoin | operational | internal |
| [F-STORE-098](06-journal-and-storage.md#f-store-098) | OSD epoch barrier in cap messages | supporting | client-wire, admin |
| [F-STORE-099](06-journal-and-storage.md#f-store-099) | Metadata-pool-full ENOSPC policy | required | client-wire |
| [F-STORE-100](06-journal-and-storage.md#f-store-100) | Metadata write-error policy and read-only mode | operational | client-wire, config, metrics |
| [F-MULTI-012](07-multi-mds.md#f-multi-012) | Cap EXPORT message to clients (CEPH_CAP_OP_EXPORT) | supporting | client-wire |
| [F-MULTI-013](07-multi-mds.md#f-multi-013) | Cap IMPORT message to clients and auth-cap switch (CEPH_CAP_OP_IMPORT) *(legacy parts)* | supporting | client-wire |
| [F-MULTI-014](07-multi-mds.md#f-multi-014) | Migrate sequence (mseq) and stale cap-message handling | supporting | client-wire |
| [F-MULTI-015](07-multi-mds.md#f-multi-015) | Writeable-range reconciliation on cap import | supporting | internal |
| [F-MULTI-016](07-multi-mds.md#f-multi-016) | Client session flush barrier before shipping (FLUSHMSG) *(legacy parts)* | supporting | client-wire |
| [F-MULTI-017](07-multi-mds.md#f-multi-017) | Forced client session open on the importer | supporting | client-wire, on-disk |
| [F-MULTI-018](07-multi-mds.md#f-multi-018) | Request blocking and forwarding around migration | supporting | client-wire |
| [F-MULTI-019](07-multi-mds.md#f-multi-019) | Export-target publication (MMDSLoadTargets, MDSMap export_targets) | supporting | cluster-wire, client-wire |
| [F-MULTI-020](07-multi-mds.md#f-multi-020) | Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps) | supporting | mds-wire, client-wire, on-disk |
| [F-MULTI-038](07-multi-mds.md#f-multi-038) | Set and remove ceph.dir.pin | extension | client-wire, on-disk |
| [F-MULTI-039](07-multi-mds.md#f-multi-039) | Pin read-back (InodeStat dir_pin, GETVXATTR) *(legacy parts)* | extension | client-wire |
| [F-MULTI-040](07-multi-mds.md#f-multi-040) | Pin inheritance and precedence resolution | extension | internal |
| [F-MULTI-041](07-multi-mds.md#f-multi-041) | Pin enforcement and aux subtrees (handle_export_pins) | extension | internal |
| [F-MULTI-045](07-multi-mds.md#f-multi-045) | Distributed ephemeral pins (ceph.dir.pin.distributed) | extension | client-wire, on-disk |
| [F-MULTI-046](07-multi-mds.md#f-multi-046) | Random ephemeral pins (ceph.dir.pin.random) | extension | client-wire, on-disk |
| [F-MULTI-050](07-multi-mds.md#f-multi-050) | Hot-directory replication (REP_ALL, MDirUpdate) | operational | mds-wire, client-wire, config |
| [F-SNAP-001](08-snapshots-quotas-subvolumes.md#f-snap-001) | Snapshot id and realm seq allocation from one monotonic counter | extension | client-wire, cluster-wire, on-disk |
| [F-SNAP-006](08-snapshots-quotas-subvolumes.md#f-snap-006) | Snap realms and effective snapshot sets *(legacy parts)* | extension | client-wire, on-disk |
| [F-SNAP-007](08-snapshots-quotas-subvolumes.md#f-snap-007) | Realm membership of caps and realm split on creation | extension | client-wire |
| [F-SNAP-008](08-snapshots-quotas-subvolumes.md#f-snap-008) | Snap trace contents and client interpretation | extension | client-wire |
| [F-SNAP-009](08-snapshots-quotas-subvolumes.md#f-snap-009) | Per-realm MClientSnap notification (always op SPLIT) | extension | client-wire |
| [F-SNAP-010](08-snapshots-quotas-subvolumes.md#f-snap-010) | Global realm broadcast on every snap-table operation | extension | client-wire |
| [F-SNAP-011](08-snapshots-quotas-subvolumes.md#f-snap-011) | Hard-link global realm (PARENT_GLOBAL) | extension | client-wire, on-disk |
| [F-SNAP-012](08-snapshots-quotas-subvolumes.md#f-snap-012) | Past-parent snapshots on rename and unlink | extension | client-wire, on-disk |
| [F-SNAP-013](08-snapshots-quotas-subvolumes.md#f-snap-013) | Create a snapshot (MKSNAP, mkdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-014](08-snapshots-quotas-subvolumes.md#f-snap-014) | Remove a snapshot (RMSNAP, rmdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-015](08-snapshots-quotas-subvolumes.md#f-snap-015) | Rename a snapshot (RENAMESNAP) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-019](08-snapshots-quotas-subvolumes.md#f-snap-019) | Per-directory snapshot limit (mds_max_snaps_per_dir) | extension | config, client-wire |
| [F-SNAP-021](08-snapshots-quotas-subvolumes.md#f-snap-021) | Snapshot path encoding, traversal and LOOKUPSNAP | extension | client-wire |
| [F-SNAP-022](08-snapshots-quotas-subvolumes.md#f-snap-022) | List snapshots (LSSNAP, readdir of .snap) | extension | client-wire |
| [F-SNAP-023](08-snapshots-quotas-subvolumes.md#f-snap-023) | Read-only snapshot semantics and snapshot caps | extension | client-wire |
| [F-SNAP-024](08-snapshots-quotas-subvolumes.md#f-snap-024) | Snapshot attributes in InodeStat (snap_btime, snap_metadata) | extension | client-wire |
| [F-SNAP-025](08-snapshots-quotas-subvolumes.md#f-snap-025) | Snapshot inode lookup by ino (LOOKUPINO with snapid) | extension | client-wire |
| [F-SNAP-026](08-snapshots-quotas-subvolumes.md#f-snap-026) | Metadata copy-on-write (first/last ranges, snapped dentries, old_inodes) | extension | on-disk |
| [F-SNAP-032](08-snapshots-quotas-subvolumes.md#f-snap-032) | Snaprealm reconciliation at client reconnect and rejoin | extension | client-wire |
| [F-SNAP-033](08-snapshots-quotas-subvolumes.md#f-snap-033) | Directory entry statistics (dirstat / fragstat) | required | client-wire, on-disk |
| [F-SNAP-034](08-snapshots-quotas-subvolumes.md#f-snap-034) | Recursive subtree statistics (rstat) | extension | client-wire, on-disk |
| [F-SNAP-039](08-snapshots-quotas-subvolumes.md#f-snap-039) | rstat reads routed to the auth rank (GETATTR CEPH_STAT_RSTAT) | extension | client-wire |
| [F-SNAP-040](08-snapshots-quotas-subvolumes.md#f-snap-040) | Snapshot rstats (mds_snap_rstat) | extension | config, on-disk |
| [F-SNAP-042](08-snapshots-quotas-subvolumes.md#f-snap-042) | Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files) | extension | client-wire, on-disk |
| [F-SNAP-043](08-snapshots-quotas-subvolumes.md#f-snap-043) | Quota directories become snaprealm roots | extension | client-wire, on-disk |
| [F-SNAP-044](08-snapshots-quotas-subvolumes.md#f-snap-044) | Quota and usage in InodeStat *(legacy parts)* | extension | client-wire |
| [F-SNAP-045](08-snapshots-quotas-subvolumes.md#f-snap-045) | MClientQuota usage pushes with thresholds *(legacy parts)* | extension | client-wire |
| [F-SNAP-047](08-snapshots-quotas-subvolumes.md#f-snap-047) | Client-side quota enforcement (EDQUOT, cross-quota EXDEV) | extension | client-wire |
| [F-SNAP-048](08-snapshots-quotas-subvolumes.md#f-snap-048) | Quota-based statfs | extension | client-wire |
| [F-SNAP-050](08-snapshots-quotas-subvolumes.md#f-snap-050) | Subvolume flag (ceph.dir.subvolume) | extension | client-wire, on-disk |
| [F-SNAP-052](08-snapshots-quotas-subvolumes.md#f-snap-052) | Cross-subvolume link and rename rejected with EXDEV | extension | client-wire |
| [F-SNAP-053](08-snapshots-quotas-subvolumes.md#f-snap-053) | Snapshots only at the subvolume root | extension | client-wire |
| [F-SNAP-054](08-snapshots-quotas-subvolumes.md#f-snap-054) | Subvolume id on the wire (InodeStat, MClientCaps) | extension | client-wire |
| [F-BG-002](09-background-services.md#f-bg-002) | Deferred deletion: unlink, rmdir and rename-over move the inode to a stray | required | client-wire, on-disk |
| [F-BG-004](09-background-services.md#f-bg-004) | ENOSPC on unlink when the current stray dirfrag is full | operational | client-wire, config |
| [F-BG-006](09-background-services.md#f-bg-006) | Purge eligibility decision (eval_stray) | supporting | internal |
| [F-BG-008](09-background-services.md#f-bg-008) | Prompt cap release for unlinked inodes (nlink 0 grant with Ls) | supporting | client-wire |
| [F-BG-015](09-background-services.md#f-bg-015) | File data and backtrace object deletion (PURGE_FILE) | supporting | cluster-wire, on-disk |
| [F-BG-025](09-background-services.md#f-bg-025) | Purging inodes are invisible to clients (ENOENT / ESTALE) | supporting | client-wire |
| [F-BG-026](09-background-services.md#f-bg-026) | Namespace rules for stray and unlinked directories | required | client-wire |
| [F-BG-056](09-background-services.md#f-bg-056) | Duplicate primary inode detection on dirfrag load | supporting | metrics |
| [F-BG-060](09-background-services.md#f-bg-060) | EIO for requests that touch known damage | required | client-wire |
| [F-BG-061](09-background-services.md#f-bg-061) | readdir behaviour on damaged directories | required | client-wire |
| [F-QOS-001](10-qos-metrics-quiesce-quarantine.md#f-qos-001) | QoS admission gate for client metadata requests | operational | config, admin |
| [F-QOS-002](10-qos-metrics-quiesce-quarantine.md#f-qos-002) | QoS bucket classification from the session mount root | operational | client-wire, admin |
| [F-QOS-004](10-qos-metrics-quiesce-quarantine.md#f-qos-004) | dmClock tag-based scheduling per bucket | operational | internal |
| [F-QOS-011](10-qos-metrics-quiesce-quarantine.md#f-qos-011) | Client metrics negotiation (feature bit 14 and metric_spec) *(legacy parts)* | operational | client-wire |
| [F-QOS-012](10-qos-metrics-quiesce-quarantine.md#f-qos-012) | MClientMetrics ingestion of per-client metrics (types 0..9) *(legacy parts)* | operational | client-wire |
| [F-QOS-013](10-qos-metrics-quiesce-quarantine.md#f-qos-013) | Tolerant decoding of unknown metric types | supporting | client-wire |
| [F-QOS-014](10-qos-metrics-quiesce-quarantine.md#f-qos-014) | Metrics accepted on any active rank (client send rules) | operational | client-wire |
| [F-QOS-015](10-qos-metrics-quiesce-quarantine.md#f-qos-015) | Subvolume id in InodeStat and cap flush acks | extension | client-wire |
| [F-QOS-016](10-qos-metrics-quiesce-quarantine.md#f-qos-016) | Subvolume I/O metrics (type 16) ingestion and path resolution | operational | client-wire, metrics |
| [F-QOS-017](10-qos-metrics-quiesce-quarantine.md#f-qos-017) | Feature bit 22 enables kernel subvolume metrics | operational | client-wire |
| [F-QOS-044](10-qos-metrics-quiesce-quarantine.md#f-qos-044) | Per-inode quiesce request (quiesce_inode) | extension | client-wire |
| [F-QOS-045](10-qos-metrics-quiesce-quarantine.md#f-qos-045) | quiescelock and parking of mutations | extension | internal |
| [F-QOS-046](10-qos-metrics-quiesce-quarantine.md#f-qos-046) | Cap masking while quiesced | extension | client-wire |
| [F-QOS-052](10-qos-metrics-quiesce-quarantine.md#f-qos-052) | ceph.quiesce.block vxattr | extension | client-wire, on-disk |
| [F-QOS-064](10-qos-metrics-quiesce-quarantine.md#f-qos-064) | Quarantine cap-revocation walk | extension | client-wire |
| [F-QOS-067](10-qos-metrics-quiesce-quarantine.md#f-qos-067) | Quarantine re-enforcement after restart | extension | client-wire |
| [F-QOS-070](10-qos-metrics-quiesce-quarantine.md#f-qos-070) | Quarantine client feature bit 24 | extension | client-wire |
| [F-QOS-072](10-qos-metrics-quiesce-quarantine.md#f-qos-072) | Request parking for quarantine-unaware sessions | extension | client-wire |
| [F-QOS-073](10-qos-metrics-quiesce-quarantine.md#f-qos-073) | Snapshot operation parking under quarantine | extension | client-wire |
| [F-QOS-074](10-qos-metrics-quiesce-quarantine.md#f-qos-074) | Cap restriction to PIN under quarantine | extension | client-wire |
| [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) | Peer authorizer verification and caps loading at connection accept *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-ADM-003](11-security-admin-and-observability.md#f-adm-003) | Authentication method none (and Kerberos) means allow-all | operational | config |
| [F-ADM-007](11-security-admin-and-observability.md#f-adm-007) | Grant evaluation: union of grants in a fixed step order | extension | client-wire, admin |
| [F-ADM-008](11-security-admin-and-observability.md#f-adm-008) | path= restriction and the inode path used for checks | extension | client-wire, admin |
| [F-ADM-009](11-security-admin-and-observability.md#f-adm-009) | fsname= restriction and file-system gating of session messages | extension | client-wire, admin |
| [F-ADM-010](11-security-admin-and-observability.md#f-adm-010) | uid=/gids= restriction with simplified mode-bit check | extension | client-wire, admin |
| [F-ADM-011](11-security-admin-and-observability.md#f-adm-011) | root_squash grants | extension | client-wire, admin |
| [F-ADM-012](11-security-admin-and-observability.md#f-adm-012) | p flag: permission to set ceph.* vxattrs and non-default layouts | extension | client-wire, admin |
| [F-ADM-013](11-security-admin-and-observability.md#f-adm-013) | s flag and snapshot uid range | extension | client-wire, config |
| [F-ADM-014](11-security-admin-and-observability.md#f-adm-014) | f flag: metadata-pool-full bypass | extension | client-wire, admin |
| [F-ADM-015](11-security-admin-and-observability.md#f-adm-015) | q/Q flags: access to quarantined subtrees | extension | client-wire, admin |
| [F-ADM-016](11-security-admin-and-observability.md#f-adm-016) | network restriction | extension | admin |
| [F-ADM-017](11-security-admin-and-observability.md#f-adm-017) | Mount root check at session open and auth_name check at reclaim *(legacy parts)* | supporting | client-wire, on-disk |
| [F-ADM-018](11-security-admin-and-observability.md#f-adm-018) | Per-request access-check matrix *(legacy parts)* | extension | client-wire |
| [F-ADM-020](11-security-admin-and-observability.md#f-adm-020) | fscrypt attribute changes require allow * | extension | client-wire |
| [F-ADM-021](11-security-admin-and-observability.md#f-adm-021) | cap_auths export in CEPH_SESSION_OPEN and client-side checks *(legacy parts)* | extension | client-wire |
| [F-ADM-042](11-security-admin-and-observability.md#f-adm-042) | force_readonly | operational | admin, client-wire |
| [F-ADM-044](11-security-admin-and-observability.md#f-adm-044) | cache drop | operational | admin, client-wire |
| [F-ADM-093](11-security-admin-and-observability.md#f-adm-093) | mgr/volumes: subvolume resize via quota | operational | client-wire |
| [F-ADM-095](11-security-admin-and-observability.md#f-adm-095) | mgr/volumes: charmap (case sensitivity, normalization, encoding) | operational | client-wire |
| [F-ADM-098](11-security-admin-and-observability.md#f-adm-098) | mgr/volumes: snapshot visibility | operational | client-wire |
| [F-ADM-106](11-security-admin-and-observability.md#f-adm-106) | mgr/stats: fs perf stats (and cephfs-top) | operational | cluster-wire, admin |

## libcephfs

Userspace client library src/client (libcephfs API users). 423 features (7 legacy entries left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-001](01-cluster-membership-and-ha.md#f-clu-001) | FSMap: monitor-owned map of all file systems | operational | cluster-wire |
| [F-CLU-002](01-cluster-membership-and-ha.md#f-clu-002) | MDSMap: per-file-system map and its wire encoding *(legacy parts)* | supporting | cluster-wire, client-wire |
| [F-CLU-003](01-cluster-membership-and-ha.md#f-clu-003) | Epoch model: FSMap, MDSMap and standby epochs, null map | operational | cluster-wire, client-wire |
| [F-CLU-004](01-cluster-membership-and-ha.md#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire |
| [F-CLU-005](01-cluster-membership-and-ha.md#f-clu-005) | FSMapUser: file system name directory for clients | operational | cluster-wire, client-wire |
| [F-CLU-007](01-cluster-membership-and-ha.md#f-clu-007) | libcephfs map use and mount availability check | supporting | client-wire |
| [F-CLU-008](01-cluster-membership-and-ha.md#f-clu-008) | FSMap filtering for file-system-restricted sessions | operational | cluster-wire, admin |
| [F-CLU-010](01-cluster-membership-and-ha.md#f-clu-010) | Multiple file systems: enable_multiple and the default file system *(legacy parts)* | operational | cluster-wire, admin |
| [F-CLU-011](01-cluster-membership-and-ha.md#f-clu-011) | Data pool list of a file system | extension | cluster-wire, client-wire, admin |
| [F-CLU-012](01-cluster-membership-and-ha.md#f-clu-012) | max_file_size published in the MDSMap | supporting | client-wire, admin |
| [F-CLU-023](01-cluster-membership-and-ha.md#f-clu-023) | Beacon acks and MDS-side lagginess (self-fencing deferral) | operational | cluster-wire, config |
| [F-CLU-029](01-cluster-membership-and-ha.md#f-clu-029) | Monitor-side laggy detection and its protections | operational | cluster-wire, config |
| [F-CLU-033](01-cluster-membership-and-ha.md#f-clu-033) | Affinity-driven swaps of healthy daemons | operational | cluster-wire |
| [F-CLU-034](01-cluster-membership-and-ha.md#f-clu-034) | Failed-rank and laggy-holder replacement | operational | cluster-wire |
| [F-CLU-035](01-cluster-membership-and-ha.md#f-clu-035) | Promotion into a rank and the initial rank state | operational | cluster-wire |
| [F-CLU-038](01-cluster-membership-and-ha.md#f-clu-038) | Rank incarnation | supporting | cluster-wire, client-wire |
| [F-CLU-040](01-cluster-membership-and-ha.md#f-clu-040) | Growing the number of active ranks (max_mds) | operational | cluster-wire, admin |
| [F-CLU-041](01-cluster-membership-and-ha.md#f-clu-041) | Shrinking: monitor-driven up:stopping and FSMap::stop | operational | cluster-wire, admin |
| [F-CLU-049](01-cluster-membership-and-ha.md#f-clu-049) | Damaged ranks and mds repaired | operational | admin, cluster-wire, metrics |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-056](01-cluster-membership-and-ha.md#f-clu-056) | Rank sets and derived predicates | operational | cluster-wire |
| [F-CLU-060](01-cluster-membership-and-ha.md#f-clu-060) | MDSMap epoch waits for client requests | supporting | client-wire |
| [F-CLU-069](01-cluster-membership-and-ha.md#f-clu-069) | Standby-replay takeover | operational | cluster-wire |
| [F-CLU-071](01-cluster-membership-and-ha.md#f-clu-071) | Survivor reactions to a peer failing and recovering | supporting | mds-wire |
| [F-CLU-077](01-cluster-membership-and-ha.md#f-clu-077) | Rename-specific resolve handling | supporting | mds-wire, client-wire |
| [F-CLU-080](01-cluster-membership-and-ha.md#f-clu-080) | up:reconnect: the client reconnect window and its timeout | supporting | client-wire, config |
| [F-CLU-082](01-cluster-membership-and-ha.md#f-clu-082) | Recording caps and snaprealms from reconnect messages | supporting | client-wire |
| [F-CLU-084](01-cluster-membership-and-ha.md#f-clu-084) | Admission of replayed and retried requests during reconnect | supporting | client-wire |
| [F-CLU-087](01-cluster-membership-and-ha.md#f-clu-087) | Matching reconnected caps to inodes: OpenFileTable prefetch and open by number | supporting | on-disk, client-wire |
| [F-CLU-088](01-cluster-membership-and-ha.md#f-clu-088) | Moving reconnected caps between ranks (IMPORT/EXPORT during rejoin) | supporting | client-wire, mds-wire |
| [F-CLU-089](01-cluster-membership-and-ha.md#f-clu-089) | Forced session open during rejoin *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-090](01-cluster-membership-and-ha.md#f-clu-090) | Choosing lock states from reconnected caps | supporting | internal |
| [F-CLU-092](01-cluster-membership-and-ha.md#f-clu-092) | Snaprealm reconciliation and snapflush rebuild at the end of rejoin | supporting | client-wire |
| [F-CLU-094](01-cluster-membership-and-ha.md#f-clu-094) | Rejoin on a single-rank file system | supporting | client-wire, on-disk |
| [F-CLU-095](01-cluster-membership-and-ha.md#f-clu-095) | up:clientreplay: ordered re-execution of replayed requests | supporting | client-wire |
| [F-CLU-096](01-cluster-membership-and-ha.md#f-clu-096) | Reclaim gating of the clientreplay to active transition | supporting | client-wire |
| [F-CLU-097](01-cluster-membership-and-ha.md#f-clu-097) | Activation (recovery_done and active_start) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-098](01-cluster-membership-and-ha.md#f-clu-098) | Leftover reconnect caps and cap reissue at activation | supporting | client-wire, metrics |
| [F-CLU-099](01-cluster-membership-and-ha.md#f-clu-099) | File size and mtime recovery for writers that did not come back | supporting | client-wire, on-disk, metrics |
| [F-CLU-100](01-cluster-membership-and-ha.md#f-clu-100) | Re-executing interrupted truncates after recovery | supporting | on-disk |
| [F-CLU-102](01-cluster-membership-and-ha.md#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk |
| [F-CLU-104](01-cluster-membership-and-ha.md#f-clu-104) | Reporting a rank as DAMAGED | operational | cluster-wire, metrics |
| [F-CLU-107](01-cluster-membership-and-ha.md#f-clu-107) | Write error policy | operational | config, metrics |
| [F-CLU-109](01-cluster-membership-and-ha.md#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config |
| [F-CLU-113](01-cluster-membership-and-ha.md#f-clu-113) | Progress thread and rank waiter queues | internal | internal |
| [F-CLU-114](01-cluster-membership-and-ha.md#f-clu-114) | Dispatcher chain, message priorities and ordering | internal | internal |
| [F-CLU-115](01-cluster-membership-and-ha.md#f-clu-115) | Rank message validation and routing *(legacy parts)* | internal | client-wire, mds-wire |
| [F-CLU-116](01-cluster-membership-and-ha.md#f-clu-116) | Outbound messaging helpers and local delivery | internal | client-wire, mds-wire |
| [F-CLU-117](01-cluster-membership-and-ha.md#f-clu-117) | Tick and the shared timer | internal | config |
| [F-PROTO-002](02-client-sessions-and-protocol.md#f-proto-002) | msgr2.1 crc mode for client connections *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-003](02-client-sessions-and-protocol.md#f-proto-003) | msgr2.1 secure mode (AES-128-GCM) *(legacy parts)* | operational | client-wire, config |
| [F-PROTO-004](02-client-sessions-and-protocol.md#f-proto-004) | msgr2 on-wire compression negotiation | internal | client-wire, config |
| [F-PROTO-005](02-client-sessions-and-protocol.md#f-proto-005) | Address vector publication and server identity checks *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-006](02-client-sessions-and-protocol.md#f-proto-006) | Lossless stateful-server connection policy and replay of unacknowledged messages *(legacy parts)* | supporting | client-wire |
| [F-PROTO-007](02-client-sessions-and-protocol.md#f-proto-007) | Peer-reset detection and session-reset signalling *(legacy parts)* | supporting | client-wire |
| [F-PROTO-008](02-client-sessions-and-protocol.md#f-proto-008) | MDS entity name and rank identity on the wire *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-009](02-client-sessions-and-protocol.md#f-proto-009) | Keepalives and connection timeouts *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-011](02-client-sessions-and-protocol.md#f-proto-011) | cephx authorizer verification for service mds *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-012](02-client-sessions-and-protocol.md#f-proto-012) | Rotating mds service secrets | supporting | cluster-wire, config |
| [F-PROTO-015](02-client-sessions-and-protocol.md#f-proto-015) | Connection-to-session binding (identity is the cephx global id) | supporting | client-wire |
| [F-PROTO-016](02-client-sessions-and-protocol.md#f-proto-016) | Encoding primitives and versioned-struct rules | supporting | client-wire |
| [F-PROTO-017](02-client-sessions-and-protocol.md#f-proto-017) | Client message catalog and peer-type filtering | supporting | client-wire, mds-wire |
| [F-PROTO-020](02-client-sessions-and-protocol.md#f-proto-020) | CephFS feature bitmap exchange at session open *(legacy parts)* | supporting | client-wire |
| [F-PROTO-021](02-client-sessions-and-protocol.md#f-proto-021) | Per-bit gated behaviour and feature names *(legacy parts)* | supporting | client-wire, admin |
| [F-PROTO-023](02-client-sessions-and-protocol.md#f-proto-023) | required_client_features enforcement and min_compat_client *(legacy parts)* | operational | client-wire, cluster-wire, admin |
| [F-PROTO-024](02-client-sessions-and-protocol.md#f-proto-024) | Connection-level feature bits consulted by the client protocol *(legacy parts)* | supporting | client-wire |
| [F-PROTO-025](02-client-sessions-and-protocol.md#f-proto-025) | Feature bit 22 numbering conflict (Ceph CHARMAP vs kernel SUBVOLUME_METRICS) *(legacy parts)* | extension | client-wire |
| [F-PROTO-026](02-client-sessions-and-protocol.md#f-proto-026) | Client generations and candidate minimum client floors *(legacy parts)* | operational | client-wire, admin |
| [F-PROTO-027](02-client-sessions-and-protocol.md#f-proto-027) | Session state machine | supporting | client-wire |
| [F-PROTO-028](02-client-sessions-and-protocol.md#f-proto-028) | Session open handshake (REQUEST_OPEN, journaled open, OPEN) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-PROTO-029](02-client-sessions-and-protocol.md#f-proto-029) | Session open admission checks and rejection reasons *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-PROTO-030](02-client-sessions-and-protocol.md#f-proto-030) | REJECT encoding and blocklist signalling to clients *(legacy parts)* | supporting | client-wire |
| [F-PROTO-031](02-client-sessions-and-protocol.md#f-proto-031) | MClientSession encoding and the ops the MDS sends *(legacy parts)* | supporting | client-wire |
| [F-PROTO-032](02-client-sessions-and-protocol.md#f-proto-032) | Session op dispatch and rank-state gating | supporting | client-wire |
| [F-PROTO-034](02-client-sessions-and-protocol.md#f-proto-034) | Client metadata capture and use *(legacy parts)* | operational | client-wire, admin, on-disk |
| [F-PROTO-037](02-client-sessions-and-protocol.md#f-proto-037) | Duplicate REQUEST_OPEN answered with OPEN (NOTIFY_SESSION_STATE) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-038](02-client-sessions-and-protocol.md#f-proto-038) | FORCE_RO: read-only MDS notification | supporting | client-wire |
| [F-PROTO-039](02-client-sessions-and-protocol.md#f-proto-039) | FLUSHMSG / FLUSHMSG_ACK session barrier *(legacy parts)* | supporting | client-wire |
| [F-PROTO-040](02-client-sessions-and-protocol.md#f-proto-040) | REQUEST_FLUSH_MDLOG and the client sync paths | supporting | client-wire |
| [F-PROTO-041](02-client-sessions-and-protocol.md#f-proto-041) | Force-opened sessions (imports, rejoin, cross-MDS rename) | supporting | client-wire, mds-wire, on-disk |
| [F-PROTO-042](02-client-sessions-and-protocol.md#f-proto-042) | Cap renewal (REQUEST_RENEWCAPS / RENEWCAPS) | supporting | client-wire |
| [F-PROTO-043](02-client-sessions-and-protocol.md#f-proto-043) | Stale session detection and the STALE message | supporting | client-wire |
| [F-PROTO-045](02-client-sessions-and-protocol.md#f-proto-045) | Session autoclose eviction | supporting | client-wire, cluster-wire, config |
| [F-PROTO-046](02-client-sessions-and-protocol.md#f-proto-046) | Per-session timeout metadata | operational | client-wire, admin |
| [F-PROTO-048](02-client-sessions-and-protocol.md#f-proto-048) | Cap-revoke non-responder eviction | operational | config, metrics |
| [F-PROTO-049](02-client-sessions-and-protocol.md#f-proto-049) | Client-initiated close with push-sequence handshake | supporting | client-wire, on-disk |
| [F-PROTO-050](02-client-sessions-and-protocol.md#f-proto-050) | Counted messages and the session push sequence | supporting | client-wire |
| [F-PROTO-051](02-client-sessions-and-protocol.md#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin |
| [F-PROTO-052](02-client-sessions-and-protocol.md#f-proto-052) | OSDMap blocklist-driven session kill | supporting | client-wire, cluster-wire |
| [F-PROTO-053](02-client-sessions-and-protocol.md#f-proto-053) | Eviction of sessions with oversized metadata | operational | config, metrics |
| [F-PROTO-054](02-client-sessions-and-protocol.md#f-proto-054) | Session termination at rank shutdown | operational | client-wire, cluster-wire |
| [F-PROTO-055](02-client-sessions-and-protocol.md#f-proto-055) | Reconnect trigger and client-side ordering | supporting | client-wire |
| [F-PROTO-056](02-client-sessions-and-protocol.md#f-proto-056) | MClientReconnect encodings (v1 to v5, multi-message) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-057](02-client-sessions-and-protocol.md#f-proto-057) | Reconnect processing and outcomes *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-061](02-client-sessions-and-protocol.md#f-proto-061) | Snap realm reconnect | extension | client-wire |
| [F-PROTO-062](02-client-sessions-and-protocol.md#f-proto-062) | Client reactions to MDSMap rank changes | supporting | cluster-wire |
| [F-PROTO-063](02-client-sessions-and-protocol.md#f-proto-063) | Session reclaim (MClientReclaim with CEPH_RECLAIM_RESET) *(legacy parts)* | operational | client-wire |
| [F-PROTO-065](02-client-sessions-and-protocol.md#f-proto-065) | MClientRequest versioned head (head v1, v2, v3) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-067](02-client-sessions-and-protocol.md#f-proto-067) | Request message tail: stamp, gid_list, alternate_name, fscrypt fields *(legacy parts)* | supporting | client-wire |
| [F-PROTO-068](02-client-sessions-and-protocol.md#f-proto-068) | Request flags REPLAY, WANT_DENTRY and ASYNC | supporting | client-wire |
| [F-PROTO-069](02-client-sessions-and-protocol.md#f-proto-069) | Request argument union per op | supporting | client-wire |
| [F-PROTO-070](02-client-sessions-and-protocol.md#f-proto-070) | Embedded cap and dentry-lease release records | supporting | client-wire |
| [F-PROTO-071](02-client-sessions-and-protocol.md#f-proto-071) | Request data segment per op | supporting | client-wire |
| [F-PROTO-072](02-client-sessions-and-protocol.md#f-proto-072) | Path encoding on the wire | supporting | client-wire |
| [F-PROTO-073](02-client-sessions-and-protocol.md#f-proto-073) | Op code numbering and the write bit | supporting | client-wire |
| [F-PROTO-074](02-client-sessions-and-protocol.md#f-proto-074) | Client-chosen inode number (head.ino) | supporting | client-wire |
| [F-PROTO-075](02-client-sessions-and-protocol.md#f-proto-075) | Client-side rank selection and the hints the MDS provides | supporting | client-wire, config |
| [F-PROTO-076](02-client-sessions-and-protocol.md#f-proto-076) | Request forwarding (MClientRequestForward) | supporting | client-wire |
| [F-PROTO-077](02-client-sessions-and-protocol.md#f-proto-077) | Retry counters and attempt limits (32BITS_RETRY_FWD) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-078](02-client-sessions-and-protocol.md#f-proto-078) | Request identity, tid ordering and requests without a session | supporting | client-wire |
| [F-PROTO-079](02-client-sessions-and-protocol.md#f-proto-079) | MClientReply head and error codes | supporting | client-wire |
| [F-PROTO-080](02-client-sessions-and-protocol.md#f-proto-080) | Reply trace layout | supporting | client-wire |
| [F-PROTO-081](02-client-sessions-and-protocol.md#f-proto-081) | InodeStat versioned encoding (v9, REPLY_ENCODING) | supporting | client-wire |
| [F-PROTO-083](02-client-sessions-and-protocol.md#f-proto-083) | DirStat and LeaseStat records *(legacy parts)* | supporting | client-wire |
| [F-PROTO-084](02-client-sessions-and-protocol.md#f-proto-084) | Caps, xattrs and inline data carried inside InodeStat *(legacy parts)* | supporting | client-wire |
| [F-PROTO-085](02-client-sessions-and-protocol.md#f-proto-085) | Snap trace encoding and format selection | extension | client-wire |
| [F-PROTO-086](02-client-sessions-and-protocol.md#f-proto-086) | Readdir and LSSNAP reply format and ordering contract *(legacy parts)* | supporting | client-wire |
| [F-PROTO-087](02-client-sessions-and-protocol.md#f-proto-087) | CREATE reply: created inode and delegated inode ranges *(legacy parts)* | supporting | client-wire |
| [F-PROTO-088](02-client-sessions-and-protocol.md#f-proto-088) | GETFILELOCK, GETVXATTR and FILE_BLOCKDIFF extra payloads | supporting | client-wire |
| [F-PROTO-089](02-client-sessions-and-protocol.md#f-proto-089) | Early (unsafe) and safe replies | supporting | client-wire |
| [F-PROTO-090](02-client-sessions-and-protocol.md#f-proto-090) | Reply content invariants enforced by clients | supporting | client-wire |
| [F-PROTO-091](02-client-sessions-and-protocol.md#f-proto-091) | MClientCaps encoding (v14, xattrs in the middle segment) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-092](02-client-sessions-and-protocol.md#f-proto-092) | MClientCapRelease encoding and the OSD epoch barrier | supporting | client-wire |
| [F-PROTO-093](02-client-sessions-and-protocol.md#f-proto-093) | MClientLease encoding | supporting | client-wire |
| [F-PROTO-094](02-client-sessions-and-protocol.md#f-proto-094) | MClientSnap encoding | extension | client-wire |
| [F-PROTO-095](02-client-sessions-and-protocol.md#f-proto-095) | MClientQuota encoding *(legacy parts)* | extension | client-wire |
| [F-PROTO-096](02-client-sessions-and-protocol.md#f-proto-096) | MClientMetrics encoding and client send rules | operational | client-wire, metrics |
| [F-PROTO-097](02-client-sessions-and-protocol.md#f-proto-097) | MQuarantineDisable encoding | extension | client-wire |
| [F-PROTO-098](02-client-sessions-and-protocol.md#f-proto-098) | Mount flow requirements | supporting | client-wire, cluster-wire |
| [F-PROTO-099](02-client-sessions-and-protocol.md#f-proto-099) | Inode-number addressed access (NFS file handles, #ino paths, LOOKUPINO) | supporting | client-wire |
| [F-PROTO-100](02-client-sessions-and-protocol.md#f-proto-100) | Inode number ranges clients accept | supporting | client-wire |
| [F-PROTO-101](02-client-sessions-and-protocol.md#f-proto-101) | Shared sessions: NFS-Ganesha and Samba libcephfsd | supporting | client-wire |
| [F-PROTO-102](02-client-sessions-and-protocol.md#f-proto-102) | NFS-Ganesha delegations built on caps | extension | client-wire |
| [F-CAPS-001](03-caps-leases-and-locking.md#f-caps-001) | Cap bit model and composite masks | supporting | client-wire |
| [F-CAPS-002](03-caps-leases-and-locking.md#f-caps-002) | Open mode to wanted caps | supporting | client-wire |
| [F-CAPS-003](03-caps-leases-and-locking.md#f-caps-003) | Capability record, cap ids and sequence numbers | supporting | client-wire |
| [F-CAPS-004](03-caps-leases-and-locking.md#f-caps-004) | Lock-state-derived allowed caps *(legacy parts)* | supporting | client-wire |
| [F-CAPS-008](03-caps-leases-and-locking.md#f-caps-008) | Lazy I/O cap (Fl) | extension | client-wire |
| [F-CAPS-009](03-caps-leases-and-locking.md#f-caps-009) | Caps in request replies *(legacy parts)* | supporting | client-wire |
| [F-CAPS-010](03-caps-leases-and-locking.md#f-caps-010) | New caps on open, create, mknod and mkdir | supporting | client-wire |
| [F-CAPS-011](03-caps-leases-and-locking.md#f-caps-011) | Cap grant and revoke messages (GRANT/REVOKE) | supporting | client-wire |
| [F-CAPS-013](03-caps-leases-and-locking.md#f-caps-013) | Client cap message processing (handle_client_caps) | supporting | client-wire |
| [F-CAPS-014](03-caps-leases-and-locking.md#f-caps-014) | Wanted caps tracking (issue_seq rule) | supporting | client-wire |
| [F-CAPS-015](03-caps-leases-and-locking.md#f-caps-015) | Xattr blob and inline data carried on caps *(legacy parts)* | supporting | client-wire |
| [F-CAPS-016](03-caps-leases-and-locking.md#f-caps-016) | Truncate notification (CEPH_CAP_OP_TRUNC) | supporting | client-wire |
| [F-CAPS-018](03-caps-leases-and-locking.md#f-caps-018) | Dirty metadata flush and FLUSH_ACK *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CAPS-019](03-caps-leases-and-locking.md#f-caps-019) | Flush idempotency (completed_flushes, oldest_flush_tid) | supporting | client-wire, on-disk, metrics |
| [F-CAPS-020](03-caps-leases-and-locking.md#f-caps-020) | Synchronous flush and journal-flush heuristics | supporting | client-wire, config |
| [F-CAPS-021](03-caps-leases-and-locking.md#f-caps-021) | max_size and per-client writeable ranges (client_ranges) | supporting | client-wire, on-disk, config |
| [F-CAPS-022](03-caps-leases-and-locking.md#f-caps-022) | File size recovery after loss of a writer | supporting | cluster-wire, on-disk |
| [F-CAPS-023](03-caps-leases-and-locking.md#f-caps-023) | Snapshot COW of cap state and SNAP_SYNC gating | extension | on-disk |
| [F-CAPS-024](03-caps-leases-and-locking.md#f-caps-024) | FLUSHSNAP / FLUSHSNAP_ACK protocol | extension | client-wire, on-disk |
| [F-CAPS-025](03-caps-leases-and-locking.md#f-caps-025) | Null snapflush inference | extension | client-wire, on-disk |
| [F-CAPS-026](03-caps-leases-and-locking.md#f-caps-026) | Snapflush nudge | extension | config |
| [F-CAPS-027](03-caps-leases-and-locking.md#f-caps-027) | XLOCKSNAP: truncate with pending snapflush | extension | internal |
| [F-CAPS-028](03-caps-leases-and-locking.md#f-caps-028) | Batched cap release (MClientCapRelease) | supporting | client-wire |
| [F-CAPS-029](03-caps-leases-and-locking.md#f-caps-029) | Cap and dentry-lease releases embedded in requests | supporting | client-wire |
| [F-CAPS-030](03-caps-leases-and-locking.md#f-caps-030) | Stale-session cap handling *(legacy parts)* | supporting | client-wire, config |
| [F-CAPS-032](03-caps-leases-and-locking.md#f-caps-032) | Cache-pressure cap recall (CEPH_SESSION_RECALL_STATE) | operational | client-wire, metrics, config, admin |
| [F-CAPS-034](03-caps-leases-and-locking.md#f-caps-034) | Cap re-establishment after MDS failover (cap side) | supporting | client-wire |
| [F-CAPS-035](03-caps-leases-and-locking.md#f-caps-035) | Replica caps and MInodeFileCaps | internal | mds-wire |
| [F-CAPS-036](03-caps-leases-and-locking.md#f-caps-036) | Cap migration to clients (EXPORT / IMPORT) | supporting | client-wire, mds-wire |
| [F-CAPS-038](03-caps-leases-and-locking.md#f-caps-038) | Dentry lease issue | supporting | client-wire |
| [F-CAPS-039](03-caps-leases-and-locking.md#f-caps-039) | Negative dentry leases | supporting | client-wire |
| [F-CAPS-040](03-caps-leases-and-locking.md#f-caps-040) | CEPH_LEASE_PRIMARY_LINK hint | supporting | client-wire |
| [F-CAPS-043](03-caps-leases-and-locking.md#f-caps-043) | Lease revocation on dentry change | supporting | client-wire |
| [F-CAPS-044](03-caps-leases-and-locking.md#f-caps-044) | Lease expiry and cleanup | supporting | client-wire, config |
| [F-CAPS-045](03-caps-leases-and-locking.md#f-caps-045) | Infinite leases on snapshot listings | extension | client-wire |
| [F-CAPS-046](03-caps-leases-and-locking.md#f-caps-046) | Directory-wide dentry validity via Fs | supporting | client-wire |
| [F-CAPS-053](03-caps-leases-and-locking.md#f-caps-053) | Per-session inode-number preallocation | internal | on-disk, config |
| [F-CAPS-059](03-caps-leases-and-locking.md#f-caps-059) | Replay of unsafe and async creates after failover | supporting | client-wire, config |
| [F-CAPS-067](03-caps-leases-and-locking.md#f-caps-067) | Gather engine (eval_gather) and cap revocation gather | supporting | internal |
| [F-CAPS-081](03-caps-leases-and-locking.md#f-caps-081) | Implicit quiescelock wrlock | extension | internal |
| [F-CAPS-092](03-caps-leases-and-locking.md#f-caps-092) | XLOCKDONE: read-your-own-unsafe-writes | supporting | client-wire, config |
| [F-NS-001](04-namespace-and-posix-operations.md#f-ns-001) | Client op set and dispatch switch | required | client-wire |
| [F-NS-002](04-namespace-and-posix-operations.md#f-ns-002) | Request intake gating by rank state and session | supporting | client-wire |
| [F-NS-003](04-namespace-and-posix-operations.md#f-ns-003) | Request identity and in-flight duplicate detection | supporting | client-wire |
| [F-NS-010](04-namespace-and-posix-operations.md#f-ns-010) | Journaled mutation commit and safe reply | supporting | client-wire, on-disk |
| [F-NS-011](04-namespace-and-posix-operations.md#f-ns-011) | Early (unsafe) replies | supporting | client-wire, config |
| [F-NS-012](04-namespace-and-posix-operations.md#f-ns-012) | Completed-request table and idempotent retries/replays | supporting | client-wire, on-disk, metrics |
| [F-NS-013](04-namespace-and-posix-operations.md#f-ns-013) | Replay of unsafe requests after an MDS restart | supporting | client-wire |
| [F-NS-014](04-namespace-and-posix-operations.md#f-ns-014) | Per-op reply trace contents | supporting | client-wire |
| [F-NS-016](04-namespace-and-posix-operations.md#f-ns-016) | On-demand journal flush for fsync, syncfs and sync writes | required | client-wire |
| [F-NS-017](04-namespace-and-posix-operations.md#f-ns-017) | Read-only and snapshot EROFS gates | required | client-wire |
| [F-NS-018](04-namespace-and-posix-operations.md#f-ns-018) | Full metadata pool gate | required | client-wire, cluster-wire |
| [F-NS-019](04-namespace-and-posix-operations.md#f-ns-019) | Create/remove name gates and their error order | required | client-wire |
| [F-NS-020](04-namespace-and-posix-operations.md#f-ns-020) | Directory entry limits (ENOSPC) | operational | client-wire, config |
| [F-NS-021](04-namespace-and-posix-operations.md#f-ns-021) | Permission checking split: clients enforce POSIX, the MDS enforces auth caps *(legacy parts)* | required | client-wire |
| [F-NS-022](04-namespace-and-posix-operations.md#f-ns-022) | ESTALE recovery of unknown inode numbers | supporting | client-wire, mds-wire, on-disk |
| [F-NS-023](04-namespace-and-posix-operations.md#f-ns-023) | GETATTR (stat by inode) | required | client-wire |
| [F-NS-024](04-namespace-and-posix-operations.md#f-ns-024) | LOOKUP of one name in a directory | required | client-wire |
| [F-NS-025](04-namespace-and-posix-operations.md#f-ns-025) | Negative lookup replies (null-dentry traces) | supporting | client-wire |
| [F-NS-026](04-namespace-and-posix-operations.md#f-ns-026) | LOOKUPINO and LOOKUPHASH (resolve an inode number) | supporting | client-wire, mds-wire, on-disk |
| [F-NS-027](04-namespace-and-posix-operations.md#f-ns-027) | LOOKUPPARENT (parent directory of an inode) | supporting | client-wire |
| [F-NS-028](04-namespace-and-posix-operations.md#f-ns-028) | LOOKUPNAME (name of an inode under its parent) | supporting | client-wire |
| [F-NS-029](04-namespace-and-posix-operations.md#f-ns-029) | Reads at a snapshot id (LOOKUPSNAP, snapshot LOOKUPINO, open and readdir in snapshots) | extension | client-wire |
| [F-NS-030](04-namespace-and-posix-operations.md#f-ns-030) | OPEN of an existing file or directory *(legacy parts)* | required | client-wire |
| [F-NS-031](04-namespace-and-posix-operations.md#f-ns-031) | O_TRUNC on open | required | client-wire, on-disk |
| [F-NS-032](04-namespace-and-posix-operations.md#f-ns-032) | READDIR paging of one dirfrag in hash order *(legacy parts)* | required | client-wire |
| [F-NS-033](04-namespace-and-posix-operations.md#f-ns-033) | Readdir across dirfrags: frag selection and adjustment *(legacy parts)* | required | client-wire |
| [F-NS-034](04-namespace-and-posix-operations.md#f-ns-034) | Readdir cap-acquisition throttle | operational | config, metrics |
| [F-NS-035](04-namespace-and-posix-operations.md#f-ns-035) | CREATE (open with O_CREAT, optionally O_EXCL) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-036](04-namespace-and-posix-operations.md#f-ns-036) | MKNOD (regular file, FIFO, device, socket) | required | client-wire, on-disk |
| [F-NS-037](04-namespace-and-posix-operations.md#f-ns-037) | MKDIR *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-038](04-namespace-and-posix-operations.md#f-ns-038) | SYMLINK (and readlink without an MDS op) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-039](04-namespace-and-posix-operations.md#f-ns-039) | New inode initialization *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-041](04-namespace-and-posix-operations.md#f-ns-041) | Client-specified inode number (head.ino) on create | supporting | client-wire |
| [F-NS-042](04-namespace-and-posix-operations.md#f-ns-042) | File layout selection and validation at create | extension | client-wire |
| [F-NS-043](04-namespace-and-posix-operations.md#f-ns-043) | Parent directory updates on link and unlink | required | client-wire, on-disk |
| [F-NS-044](04-namespace-and-posix-operations.md#f-ns-044) | LINK (hard link) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-045](04-namespace-and-posix-operations.md#f-ns-045) | UNLINK of a primary link | required | client-wire, on-disk |
| [F-NS-046](04-namespace-and-posix-operations.md#f-ns-046) | UNLINK of a remote (hard-link) dentry | required | client-wire, on-disk |
| [F-NS-047](04-namespace-and-posix-operations.md#f-ns-047) | RMDIR with emptiness checks | required | client-wire, on-disk |
| [F-NS-048](04-namespace-and-posix-operations.md#f-ns-048) | Unlinked-but-open inodes stay usable until the last cap is released | required | on-disk |
| [F-NS-049](04-namespace-and-posix-operations.md#f-ns-049) | RENAME of a file or directory (move, same-inode no-op) | required | client-wire, on-disk |
| [F-NS-050](04-namespace-and-posix-operations.md#f-ns-050) | RENAME over an existing target | required | client-wire, on-disk |
| [F-NS-051](04-namespace-and-posix-operations.md#f-ns-051) | RENAME validation and error order *(legacy parts)* | required | client-wire |
| [F-NS-053](04-namespace-and-posix-operations.md#f-ns-053) | Which rank serves each op (auth requirement, forward or discover) | supporting | client-wire, mds-wire, config |
| [F-NS-057](04-namespace-and-posix-operations.md#f-ns-057) | Inode authority and cap migration inside a cross-rank rename *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-NS-058](04-namespace-and-posix-operations.md#f-ns-058) | SETATTR request: mask, locks, error order and common tail *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-059](04-namespace-and-posix-operations.md#f-ns-059) | chmod and chown, with set-user/group-ID clearing | required | client-wire |
| [F-NS-060](04-namespace-and-posix-operations.md#f-ns-060) | Explicit times (utimes) and btime, with time_warp_seq *(legacy parts)* | required | client-wire |
| [F-NS-061](04-namespace-and-posix-operations.md#f-ns-061) | Size growth through SETATTR | required | client-wire |
| [F-NS-062](04-namespace-and-posix-operations.md#f-ns-062) | Truncate (shrink) with asynchronous OSD truncation | required | client-wire, cluster-wire, on-disk |
| [F-NS-063](04-namespace-and-posix-operations.md#f-ns-063) | Timestamps from the client clock and change_attr | required | client-wire, on-disk |
| [F-NS-064](04-namespace-and-posix-operations.md#f-ns-064) | fscrypt inode fields and last-block truncate *(legacy parts)* | extension | client-wire, on-disk, config |
| [F-NS-066](04-namespace-and-posix-operations.md#f-ns-066) | SETXATTR and RMXATTR of stored xattrs | supporting | client-wire, on-disk |
| [F-NS-067](04-namespace-and-posix-operations.md#f-ns-067) | Total xattr size limit (max_xattr_size) | supporting | client-wire, cluster-wire, admin |
| [F-NS-068](04-namespace-and-posix-operations.md#f-ns-068) | xattr blob delivery to clients | supporting | client-wire |
| [F-NS-069](04-namespace-and-posix-operations.md#f-ns-069) | POSIX ACLs stored as opaque xattrs | supporting | client-wire, on-disk |
| [F-NS-070](04-namespace-and-posix-operations.md#f-ns-070) | ceph.mirror.* xattrs for snapshot mirroring | extension | client-wire, on-disk |
| [F-NS-071](04-namespace-and-posix-operations.md#f-ns-071) | Virtual xattr set/remove path (common rules, p flag) | extension | client-wire, admin |
| [F-NS-072](04-namespace-and-posix-operations.md#f-ns-072) | Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*) *(legacy parts)* | extension | client-wire, cluster-wire |
| [F-NS-073](04-namespace-and-posix-operations.md#f-ns-073) | GETVXATTR op | extension | client-wire |
| [F-NS-074](04-namespace-and-posix-operations.md#f-ns-074) | Client-synthesized ceph.* vxattrs (InodeStat contract) | extension | client-wire |
| [F-NS-075](04-namespace-and-posix-operations.md#f-ns-075) | Case-insensitive and normalized directories (charmap) *(legacy parts)* | extension | client-wire, on-disk |
| [F-NS-076](04-namespace-and-posix-operations.md#f-ns-076) | alternate_name on dentries | extension | client-wire, on-disk |
| [F-NS-077](04-namespace-and-posix-operations.md#f-ns-077) | SETFILELOCK: POSIX fcntl byte-range locks | required | client-wire |
| [F-NS-078](04-namespace-and-posix-operations.md#f-ns-078) | BSD flock whole-file locks | extension | client-wire |
| [F-NS-079](04-namespace-and-posix-operations.md#f-ns-079) | Blocking lock waits and wake-ups | required | client-wire |
| [F-NS-080](04-namespace-and-posix-operations.md#f-ns-080) | Lock wait interruption (FCNTL_INTR / FLOCK_INTR) | required | client-wire |
| [F-NS-081](04-namespace-and-posix-operations.md#f-ns-081) | fcntl deadlock detection (EDEADLK) | required | client-wire |
| [F-NS-082](04-namespace-and-posix-operations.md#f-ns-082) | GETFILELOCK (F_GETLK) | required | client-wire |
| [F-NS-083](04-namespace-and-posix-operations.md#f-ns-083) | Lock cleanup on cap removal and session end | required | client-wire |
| [F-NS-084](04-namespace-and-posix-operations.md#f-ns-084) | Lock reinstatement after an MDS restart *(legacy parts)* | required | client-wire |
| [F-NS-085](04-namespace-and-posix-operations.md#f-ns-085) | Directory link count and size presentation | required | client-wire |
| [F-NS-086](04-namespace-and-posix-operations.md#f-ns-086) | Access time is not maintained by the MDS | required | client-wire |
| [F-NS-087](04-namespace-and-posix-operations.md#f-ns-087) | Timestamp range limited to unsigned 32-bit seconds | required | client-wire, on-disk |
| [F-NS-088](04-namespace-and-posix-operations.md#f-ns-088) | Limits enforced only by clients, and no hard-link limit | required | client-wire |
| [F-NS-089](04-namespace-and-posix-operations.md#f-ns-089) | Unsupported Linux extensions: renameat2 flags and O_TMPFILE | required | client-wire |
| [F-NS-090](04-namespace-and-posix-operations.md#f-ns-090) | Data-path coherency departures outside the MDS | required | client-wire |
| [F-NS-091](04-namespace-and-posix-operations.md#f-ns-091) | statfs is answered by the monitors | required | cluster-wire |
| [F-CACHE-001](05-metadata-cache-and-dirfrags.md#f-cache-001) | Inode record field set (inode_t) | required | on-disk, client-wire |
| [F-CACHE-003](05-metadata-cache-and-dirfrags.md#f-cache-003) | Refusal of client operations on private and system inodes | supporting | client-wire |
| [F-CACHE-004](05-metadata-cache-and-dirfrags.md#f-cache-004) | Timestamps, time_warp_seq and change_attr *(legacy parts)* | required | client-wire, on-disk |
| [F-CACHE-006](05-metadata-cache-and-dirfrags.md#f-cache-006) | Truncation state fields | required | client-wire, on-disk |
| [F-CACHE-007](05-metadata-cache-and-dirfrags.md#f-cache-007) | File layout field and layout inheritance *(legacy parts)* | extension | client-wire, on-disk |
| [F-CACHE-008](05-metadata-cache-and-dirfrags.md#f-cache-008) | Inline data *(legacy parts)* | extension | client-wire, on-disk |
| [F-CACHE-010](05-metadata-cache-and-dirfrags.md#f-cache-010) | Inode version and client-visible version | supporting | client-wire |
| [F-CACHE-011](05-metadata-cache-and-dirfrags.md#f-cache-011) | Snapshot-range identity of inodes and dentries | extension | on-disk |
| [F-CACHE-014](05-metadata-cache-and-dirfrags.md#f-cache-014) | Projected dentry linkage (primary, remote, null) | supporting | on-disk |
| [F-CACHE-023](05-metadata-cache-and-dirfrags.md#f-cache-023) | Directory completeness and authoritative negative lookups | supporting | internal |
| [F-CACHE-041](05-metadata-cache-and-dirfrags.md#f-cache-041) | Path traversal (path_traverse) | required | client-wire |
| [F-CACHE-044](05-metadata-cache-and-dirfrags.md#f-cache-044) | Forward-or-discover for non-auth objects | supporting | client-wire, config |
| [F-CACHE-045](05-metadata-cache-and-dirfrags.md#f-cache-045) | Hard-link target resolution | required | client-wire |
| [F-CACHE-047](05-metadata-cache-and-dirfrags.md#f-cache-047) | Lookup by inode number (open_ino) | supporting | client-wire, on-disk, mds-wire |
| [F-CACHE-050](05-metadata-cache-and-dirfrags.md#f-cache-050) | ESTALE recovery via find_ino_peers | supporting | client-wire, mds-wire |
| [F-CACHE-060](05-metadata-cache-and-dirfrags.md#f-cache-060) | Pin reasons that block eviction | operational | internal |
| [F-CACHE-071](05-metadata-cache-and-dirfrags.md#f-cache-071) | Root and system inode initial attributes and default layouts | required | client-wire, on-disk, config |
| [F-CACHE-072](05-metadata-cache-and-dirfrags.md#f-cache-072) | Global snaprealm dummy inode | supporting | client-wire |
| [F-CACHE-074](05-metadata-cache-and-dirfrags.md#f-cache-074) | Fragment identifier and fragment tree (frag_t, fragtree_t) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CACHE-075](05-metadata-cache-and-dirfrags.md#f-cache-075) | Dentry name hash per directory (dl_dir_hash) *(legacy parts)* | supporting | client-wire, on-disk, config |
| [F-CACHE-076](05-metadata-cache-and-dirfrags.md#f-cache-076) | Fragtree and dir_layout in InodeStat *(legacy parts)* | supporting | client-wire |
| [F-CACHE-077](05-metadata-cache-and-dirfrags.md#f-cache-077) | Hash-ordered dentry storage and readdir order *(legacy parts)* | required | client-wire |
| [F-CACHE-078](05-metadata-cache-and-dirfrags.md#f-cache-078) | Readdir fragment mapping and resume across refragmentation | required | client-wire |
| [F-CACHE-079](05-metadata-cache-and-dirfrags.md#f-cache-079) | DirStat delegation hints *(legacy parts)* | supporting | client-wire, config |
| [F-CACHE-080](05-metadata-cache-and-dirfrags.md#f-cache-080) | Per-dirfrag size limit (ENOSPC) | operational | client-wire, config |
| [F-CACHE-081](05-metadata-cache-and-dirfrags.md#f-cache-081) | Per-directory entry limit | operational | client-wire, config |
| [F-CACHE-097](05-metadata-cache-and-dirfrags.md#f-cache-097) | Effects of a fragment freeze on clients | supporting | client-wire |
| [F-STORE-007](06-journal-and-storage.md#f-store-007) | Per-rank write-ahead metadata journal with one submission order | supporting | on-disk |
| [F-STORE-008](06-journal-and-storage.md#f-store-008) | Group commit: flush and wait_for_safe | supporting | internal |
| [F-STORE-009](06-journal-and-storage.md#f-store-009) | Periodic journal flush on tick | supporting | config |
| [F-STORE-011](06-journal-and-storage.md#f-store-011) | Ordered completions and the safe position | supporting | internal |
| [F-STORE-012](06-journal-and-storage.md#f-store-012) | Safe reply only after the journal commit (early unsafe reply) | supporting | client-wire |
| [F-STORE-024](06-journal-and-storage.md#f-store-024) | Snapshot COW images in the journal | extension | on-disk |
| [F-STORE-026](06-journal-and-storage.md#f-store-026) | EUpdate: generic mutation record | supporting | on-disk |
| [F-STORE-027](06-journal-and-storage.md#f-store-027) | EOpen: open-file journaling | supporting | on-disk |
| [F-STORE-028](06-journal-and-storage.md#f-store-028) | Session events: ESession and ESessions *(legacy parts)* | supporting | on-disk |
| [F-STORE-029](06-journal-and-storage.md#f-store-029) | Completed client request ids in journal and SessionMap | supporting | on-disk |
| [F-STORE-030](06-journal-and-storage.md#f-store-030) | Completed cap flush ids in journal and SessionMap | supporting | on-disk |
| [F-STORE-046](06-journal-and-storage.md#f-store-046) | Special inode numbers and per-rank ranges *(legacy parts)* | supporting | client-wire, on-disk |
| [F-STORE-058](06-journal-and-storage.md#f-store-058) | Backtrace parent xattr on object 0 | supporting | on-disk |
| [F-STORE-062](06-journal-and-storage.md#f-store-062) | Lookup by inode number through backtraces (open_ino storage path) | supporting | client-wire, on-disk |
| [F-STORE-064](06-journal-and-storage.md#f-store-064) | Data object naming and striping shared with clients | supporting | client-wire, on-disk |
| [F-STORE-065](06-journal-and-storage.md#f-store-065) | Data-object truncation (truncate start/finish and TRIMTRUNC) | required | on-disk, cluster-wire |
| [F-STORE-066](06-journal-and-storage.md#f-store-066) | File size and mtime recovery by probing data objects | supporting | cluster-wire |
| [F-STORE-070](06-journal-and-storage.md#f-store-070) | Inode number allocation in create paths | supporting | on-disk, client-wire |
| [F-STORE-073](06-journal-and-storage.md#f-store-073) | Inode numbers are never reused after deletion | required | client-wire, on-disk |
| [F-STORE-075](06-journal-and-storage.md#f-store-075) | SessionMap object | supporting | on-disk, config |
| [F-STORE-089](06-journal-and-storage.md#f-store-089) | OpenFileTable prefetch during rejoin | operational | internal |
| [F-STORE-098](06-journal-and-storage.md#f-store-098) | OSD epoch barrier in cap messages | supporting | client-wire, admin |
| [F-STORE-099](06-journal-and-storage.md#f-store-099) | Metadata-pool-full ENOSPC policy | required | client-wire |
| [F-STORE-100](06-journal-and-storage.md#f-store-100) | Metadata write-error policy and read-only mode | operational | client-wire, config, metrics |
| [F-MULTI-012](07-multi-mds.md#f-multi-012) | Cap EXPORT message to clients (CEPH_CAP_OP_EXPORT) | supporting | client-wire |
| [F-MULTI-013](07-multi-mds.md#f-multi-013) | Cap IMPORT message to clients and auth-cap switch (CEPH_CAP_OP_IMPORT) *(legacy parts)* | supporting | client-wire |
| [F-MULTI-014](07-multi-mds.md#f-multi-014) | Migrate sequence (mseq) and stale cap-message handling | supporting | client-wire |
| [F-MULTI-015](07-multi-mds.md#f-multi-015) | Writeable-range reconciliation on cap import | supporting | internal |
| [F-MULTI-016](07-multi-mds.md#f-multi-016) | Client session flush barrier before shipping (FLUSHMSG) *(legacy parts)* | supporting | client-wire |
| [F-MULTI-017](07-multi-mds.md#f-multi-017) | Forced client session open on the importer | supporting | client-wire, on-disk |
| [F-MULTI-018](07-multi-mds.md#f-multi-018) | Request blocking and forwarding around migration | supporting | client-wire |
| [F-MULTI-019](07-multi-mds.md#f-multi-019) | Export-target publication (MMDSLoadTargets, MDSMap export_targets) | supporting | cluster-wire, client-wire |
| [F-MULTI-020](07-multi-mds.md#f-multi-020) | Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps) | supporting | mds-wire, client-wire, on-disk |
| [F-MULTI-038](07-multi-mds.md#f-multi-038) | Set and remove ceph.dir.pin | extension | client-wire, on-disk |
| [F-MULTI-039](07-multi-mds.md#f-multi-039) | Pin read-back (InodeStat dir_pin, GETVXATTR) *(legacy parts)* | extension | client-wire |
| [F-MULTI-040](07-multi-mds.md#f-multi-040) | Pin inheritance and precedence resolution | extension | internal |
| [F-MULTI-041](07-multi-mds.md#f-multi-041) | Pin enforcement and aux subtrees (handle_export_pins) | extension | internal |
| [F-MULTI-045](07-multi-mds.md#f-multi-045) | Distributed ephemeral pins (ceph.dir.pin.distributed) | extension | client-wire, on-disk |
| [F-MULTI-046](07-multi-mds.md#f-multi-046) | Random ephemeral pins (ceph.dir.pin.random) | extension | client-wire, on-disk |
| [F-SNAP-001](08-snapshots-quotas-subvolumes.md#f-snap-001) | Snapshot id and realm seq allocation from one monotonic counter | extension | client-wire, cluster-wire, on-disk |
| [F-SNAP-006](08-snapshots-quotas-subvolumes.md#f-snap-006) | Snap realms and effective snapshot sets *(legacy parts)* | extension | client-wire, on-disk |
| [F-SNAP-007](08-snapshots-quotas-subvolumes.md#f-snap-007) | Realm membership of caps and realm split on creation | extension | client-wire |
| [F-SNAP-008](08-snapshots-quotas-subvolumes.md#f-snap-008) | Snap trace contents and client interpretation | extension | client-wire |
| [F-SNAP-009](08-snapshots-quotas-subvolumes.md#f-snap-009) | Per-realm MClientSnap notification (always op SPLIT) | extension | client-wire |
| [F-SNAP-010](08-snapshots-quotas-subvolumes.md#f-snap-010) | Global realm broadcast on every snap-table operation | extension | client-wire |
| [F-SNAP-011](08-snapshots-quotas-subvolumes.md#f-snap-011) | Hard-link global realm (PARENT_GLOBAL) | extension | client-wire, on-disk |
| [F-SNAP-012](08-snapshots-quotas-subvolumes.md#f-snap-012) | Past-parent snapshots on rename and unlink | extension | client-wire, on-disk |
| [F-SNAP-013](08-snapshots-quotas-subvolumes.md#f-snap-013) | Create a snapshot (MKSNAP, mkdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-014](08-snapshots-quotas-subvolumes.md#f-snap-014) | Remove a snapshot (RMSNAP, rmdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-015](08-snapshots-quotas-subvolumes.md#f-snap-015) | Rename a snapshot (RENAMESNAP) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-016](08-snapshots-quotas-subvolumes.md#f-snap-016) | Snapshot metadata at creation (SnapPayload) | extension | client-wire, on-disk |
| [F-SNAP-017](08-snapshots-quotas-subvolumes.md#f-snap-017) | Modify snapshot metadata (SNAP_METADATA op) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-019](08-snapshots-quotas-subvolumes.md#f-snap-019) | Per-directory snapshot limit (mds_max_snaps_per_dir) | extension | config, client-wire |
| [F-SNAP-021](08-snapshots-quotas-subvolumes.md#f-snap-021) | Snapshot path encoding, traversal and LOOKUPSNAP | extension | client-wire |
| [F-SNAP-022](08-snapshots-quotas-subvolumes.md#f-snap-022) | List snapshots (LSSNAP, readdir of .snap) | extension | client-wire |
| [F-SNAP-023](08-snapshots-quotas-subvolumes.md#f-snap-023) | Read-only snapshot semantics and snapshot caps | extension | client-wire |
| [F-SNAP-024](08-snapshots-quotas-subvolumes.md#f-snap-024) | Snapshot attributes in InodeStat (snap_btime, snap_metadata) | extension | client-wire |
| [F-SNAP-026](08-snapshots-quotas-subvolumes.md#f-snap-026) | Metadata copy-on-write (first/last ranges, snapped dentries, old_inodes) | extension | on-disk |
| [F-SNAP-029](08-snapshots-quotas-subvolumes.md#f-snap-029) | Directory snapshot diff (READDIR_SNAPDIFF) | extension | client-wire |
| [F-SNAP-030](08-snapshots-quotas-subvolumes.md#f-snap-030) | File block diff between snapshots (FILE_BLOCKDIFF) | extension | client-wire, cluster-wire |
| [F-SNAP-032](08-snapshots-quotas-subvolumes.md#f-snap-032) | Snaprealm reconciliation at client reconnect and rejoin | extension | client-wire |
| [F-SNAP-033](08-snapshots-quotas-subvolumes.md#f-snap-033) | Directory entry statistics (dirstat / fragstat) | required | client-wire, on-disk |
| [F-SNAP-034](08-snapshots-quotas-subvolumes.md#f-snap-034) | Recursive subtree statistics (rstat) | extension | client-wire, on-disk |
| [F-SNAP-039](08-snapshots-quotas-subvolumes.md#f-snap-039) | rstat reads routed to the auth rank (GETATTR CEPH_STAT_RSTAT) | extension | client-wire |
| [F-SNAP-040](08-snapshots-quotas-subvolumes.md#f-snap-040) | Snapshot rstats (mds_snap_rstat) | extension | config, on-disk |
| [F-SNAP-042](08-snapshots-quotas-subvolumes.md#f-snap-042) | Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files) | extension | client-wire, on-disk |
| [F-SNAP-043](08-snapshots-quotas-subvolumes.md#f-snap-043) | Quota directories become snaprealm roots | extension | client-wire, on-disk |
| [F-SNAP-044](08-snapshots-quotas-subvolumes.md#f-snap-044) | Quota and usage in InodeStat *(legacy parts)* | extension | client-wire |
| [F-SNAP-045](08-snapshots-quotas-subvolumes.md#f-snap-045) | MClientQuota usage pushes with thresholds *(legacy parts)* | extension | client-wire |
| [F-SNAP-047](08-snapshots-quotas-subvolumes.md#f-snap-047) | Client-side quota enforcement (EDQUOT, cross-quota EXDEV) | extension | client-wire |
| [F-SNAP-048](08-snapshots-quotas-subvolumes.md#f-snap-048) | Quota-based statfs | extension | client-wire |
| [F-SNAP-050](08-snapshots-quotas-subvolumes.md#f-snap-050) | Subvolume flag (ceph.dir.subvolume) | extension | client-wire, on-disk |
| [F-SNAP-052](08-snapshots-quotas-subvolumes.md#f-snap-052) | Cross-subvolume link and rename rejected with EXDEV | extension | client-wire |
| [F-SNAP-053](08-snapshots-quotas-subvolumes.md#f-snap-053) | Snapshots only at the subvolume root | extension | client-wire |
| [F-SNAP-054](08-snapshots-quotas-subvolumes.md#f-snap-054) | Subvolume id on the wire (InodeStat, MClientCaps) | extension | client-wire |
| [F-SNAP-055](08-snapshots-quotas-subvolumes.md#f-snap-055) | Snapshot visibility flag (ceph.dir.subvolume.snaps.visible) | extension | client-wire, on-disk |
| [F-BG-002](09-background-services.md#f-bg-002) | Deferred deletion: unlink, rmdir and rename-over move the inode to a stray | required | client-wire, on-disk |
| [F-BG-004](09-background-services.md#f-bg-004) | ENOSPC on unlink when the current stray dirfrag is full | operational | client-wire, config |
| [F-BG-006](09-background-services.md#f-bg-006) | Purge eligibility decision (eval_stray) | supporting | internal |
| [F-BG-008](09-background-services.md#f-bg-008) | Prompt cap release for unlinked inodes (nlink 0 grant with Ls) | supporting | client-wire |
| [F-BG-015](09-background-services.md#f-bg-015) | File data and backtrace object deletion (PURGE_FILE) | supporting | cluster-wire, on-disk |
| [F-BG-025](09-background-services.md#f-bg-025) | Purging inodes are invisible to clients (ENOENT / ESTALE) | supporting | client-wire |
| [F-BG-026](09-background-services.md#f-bg-026) | Namespace rules for stray and unlinked directories | required | client-wire |
| [F-BG-056](09-background-services.md#f-bg-056) | Duplicate primary inode detection on dirfrag load | supporting | metrics |
| [F-BG-060](09-background-services.md#f-bg-060) | EIO for requests that touch known damage | required | client-wire |
| [F-BG-061](09-background-services.md#f-bg-061) | readdir behaviour on damaged directories | required | client-wire |
| [F-QOS-001](10-qos-metrics-quiesce-quarantine.md#f-qos-001) | QoS admission gate for client metadata requests | operational | config, admin |
| [F-QOS-002](10-qos-metrics-quiesce-quarantine.md#f-qos-002) | QoS bucket classification from the session mount root | operational | client-wire, admin |
| [F-QOS-004](10-qos-metrics-quiesce-quarantine.md#f-qos-004) | dmClock tag-based scheduling per bucket | operational | internal |
| [F-QOS-011](10-qos-metrics-quiesce-quarantine.md#f-qos-011) | Client metrics negotiation (feature bit 14 and metric_spec) *(legacy parts)* | operational | client-wire |
| [F-QOS-012](10-qos-metrics-quiesce-quarantine.md#f-qos-012) | MClientMetrics ingestion of per-client metrics (types 0..9) *(legacy parts)* | operational | client-wire |
| [F-QOS-013](10-qos-metrics-quiesce-quarantine.md#f-qos-013) | Tolerant decoding of unknown metric types | supporting | client-wire |
| [F-QOS-014](10-qos-metrics-quiesce-quarantine.md#f-qos-014) | Metrics accepted on any active rank (client send rules) | operational | client-wire |
| [F-QOS-015](10-qos-metrics-quiesce-quarantine.md#f-qos-015) | Subvolume id in InodeStat and cap flush acks | extension | client-wire |
| [F-QOS-016](10-qos-metrics-quiesce-quarantine.md#f-qos-016) | Subvolume I/O metrics (type 16) ingestion and path resolution | operational | client-wire, metrics |
| [F-QOS-044](10-qos-metrics-quiesce-quarantine.md#f-qos-044) | Per-inode quiesce request (quiesce_inode) | extension | client-wire |
| [F-QOS-045](10-qos-metrics-quiesce-quarantine.md#f-qos-045) | quiescelock and parking of mutations | extension | internal |
| [F-QOS-046](10-qos-metrics-quiesce-quarantine.md#f-qos-046) | Cap masking while quiesced | extension | client-wire |
| [F-QOS-052](10-qos-metrics-quiesce-quarantine.md#f-qos-052) | ceph.quiesce.block vxattr | extension | client-wire, on-disk |
| [F-QOS-064](10-qos-metrics-quiesce-quarantine.md#f-qos-064) | Quarantine cap-revocation walk | extension | client-wire |
| [F-QOS-067](10-qos-metrics-quiesce-quarantine.md#f-qos-067) | Quarantine re-enforcement after restart | extension | client-wire |
| [F-QOS-069](10-qos-metrics-quiesce-quarantine.md#f-qos-069) | MDS auth cap flags q and Q (quarantine access) | extension | cluster-wire, admin |
| [F-QOS-070](10-qos-metrics-quiesce-quarantine.md#f-qos-070) | Quarantine client feature bit 24 | extension | client-wire |
| [F-QOS-071](10-qos-metrics-quiesce-quarantine.md#f-qos-071) | EACCES for quarantine-aware sessions | extension | client-wire |
| [F-QOS-072](10-qos-metrics-quiesce-quarantine.md#f-qos-072) | Request parking for quarantine-unaware sessions | extension | client-wire |
| [F-QOS-074](10-qos-metrics-quiesce-quarantine.md#f-qos-074) | Cap restriction to PIN under quarantine | extension | client-wire |
| [F-QOS-075](10-qos-metrics-quiesce-quarantine.md#f-qos-075) | EQUARANTINED errno in MClientCaps | extension | client-wire |
| [F-QOS-076](10-qos-metrics-quiesce-quarantine.md#f-qos-076) | QUARANTINE marker in InodeStat | extension | client-wire |
| [F-QOS-077](10-qos-metrics-quiesce-quarantine.md#f-qos-077) | MQuarantineDisable notification | extension | client-wire |
| [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) | Peer authorizer verification and caps loading at connection accept *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-ADM-003](11-security-admin-and-observability.md#f-adm-003) | Authentication method none (and Kerberos) means allow-all | operational | config |
| [F-ADM-007](11-security-admin-and-observability.md#f-adm-007) | Grant evaluation: union of grants in a fixed step order | extension | client-wire, admin |
| [F-ADM-008](11-security-admin-and-observability.md#f-adm-008) | path= restriction and the inode path used for checks | extension | client-wire, admin |
| [F-ADM-009](11-security-admin-and-observability.md#f-adm-009) | fsname= restriction and file-system gating of session messages | extension | client-wire, admin |
| [F-ADM-010](11-security-admin-and-observability.md#f-adm-010) | uid=/gids= restriction with simplified mode-bit check | extension | client-wire, admin |
| [F-ADM-011](11-security-admin-and-observability.md#f-adm-011) | root_squash grants | extension | client-wire, admin |
| [F-ADM-012](11-security-admin-and-observability.md#f-adm-012) | p flag: permission to set ceph.* vxattrs and non-default layouts | extension | client-wire, admin |
| [F-ADM-013](11-security-admin-and-observability.md#f-adm-013) | s flag and snapshot uid range | extension | client-wire, config |
| [F-ADM-014](11-security-admin-and-observability.md#f-adm-014) | f flag: metadata-pool-full bypass | extension | client-wire, admin |
| [F-ADM-015](11-security-admin-and-observability.md#f-adm-015) | q/Q flags: access to quarantined subtrees | extension | client-wire, admin |
| [F-ADM-016](11-security-admin-and-observability.md#f-adm-016) | network restriction | extension | admin |
| [F-ADM-017](11-security-admin-and-observability.md#f-adm-017) | Mount root check at session open and auth_name check at reclaim *(legacy parts)* | supporting | client-wire, on-disk |
| [F-ADM-018](11-security-admin-and-observability.md#f-adm-018) | Per-request access-check matrix *(legacy parts)* | extension | client-wire |
| [F-ADM-020](11-security-admin-and-observability.md#f-adm-020) | fscrypt attribute changes require allow * | extension | client-wire |
| [F-ADM-021](11-security-admin-and-observability.md#f-adm-021) | cap_auths export in CEPH_SESSION_OPEN and client-side checks *(legacy parts)* | extension | client-wire |
| [F-ADM-025](11-security-admin-and-observability.md#f-adm-025) | mgr/nfs export user caps | operational | cluster-wire, admin |
| [F-ADM-027](11-security-admin-and-observability.md#f-adm-027) | Remote tell transport (MCommand / MCommandReply) *(legacy parts)* | operational | admin, client-wire |
| [F-ADM-031](11-security-admin-and-observability.md#f-adm-031) | Client-side target resolution for tell (libcephfs mds_command) | operational | admin, cluster-wire |
| [F-ADM-042](11-security-admin-and-observability.md#f-adm-042) | force_readonly | operational | admin, client-wire |
| [F-ADM-044](11-security-admin-and-observability.md#f-adm-044) | cache drop | operational | admin, client-wire |
| [F-ADM-064](11-security-admin-and-observability.md#f-adm-064) | MDS option names read or written by peers | operational | config |
| [F-ADM-090](11-security-admin-and-observability.md#f-adm-090) | mgr/volumes: subvolume groups | operational | client-wire |
| [F-ADM-091](11-security-admin-and-observability.md#f-adm-091) | mgr/volumes: subvolume create | operational | client-wire |
| [F-ADM-093](11-security-admin-and-observability.md#f-adm-093) | mgr/volumes: subvolume resize via quota | operational | client-wire |
| [F-ADM-095](11-security-admin-and-observability.md#f-adm-095) | mgr/volumes: charmap (case sensitivity, normalization, encoding) | operational | client-wire |
| [F-ADM-097](11-security-admin-and-observability.md#f-adm-097) | mgr/volumes: subvolume snapshots | operational | client-wire |
| [F-ADM-098](11-security-admin-and-observability.md#f-adm-098) | mgr/volumes: snapshot visibility | operational | client-wire |
| [F-ADM-099](11-security-admin-and-observability.md#f-adm-099) | mgr/volumes: snapshot clone | operational | client-wire |
| [F-ADM-105](11-security-admin-and-observability.md#f-adm-105) | mgr/mirroring: checkpoints through snapshot metadata | operational | client-wire |
| [F-ADM-106](11-security-admin-and-observability.md#f-adm-106) | mgr/stats: fs perf stats (and cephfs-top) | operational | cluster-wire, admin |
| [F-ADM-108](11-security-admin-and-observability.md#f-adm-108) | mgr/dashboard: CephFS pages | operational | admin, client-wire, metrics |
| [F-ADM-112](11-security-admin-and-observability.md#f-adm-112) | mgr/nfs: CephFS exports | operational | client-wire, cluster-wire |
| [F-ADM-117](11-security-admin-and-observability.md#f-adm-117) | cephfs-mirror daemon | operational | client-wire |
| [F-ADM-121](11-security-admin-and-observability.md#f-adm-121) | Client-side tools: cephfs-tool bench and cephfs-shell | operational | client-wire |

## ceph-fuse

ceph-fuse (built on the same src/client code). 341 features (5 legacy entries left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-002](01-cluster-membership-and-ha.md#f-clu-002) | MDSMap: per-file-system map and its wire encoding *(legacy parts)* | supporting | cluster-wire, client-wire |
| [F-CLU-003](01-cluster-membership-and-ha.md#f-clu-003) | Epoch model: FSMap, MDSMap and standby epochs, null map | operational | cluster-wire, client-wire |
| [F-CLU-004](01-cluster-membership-and-ha.md#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire |
| [F-CLU-005](01-cluster-membership-and-ha.md#f-clu-005) | FSMapUser: file system name directory for clients | operational | cluster-wire, client-wire |
| [F-CLU-007](01-cluster-membership-and-ha.md#f-clu-007) | libcephfs map use and mount availability check | supporting | client-wire |
| [F-CLU-011](01-cluster-membership-and-ha.md#f-clu-011) | Data pool list of a file system | extension | cluster-wire, client-wire, admin |
| [F-CLU-012](01-cluster-membership-and-ha.md#f-clu-012) | max_file_size published in the MDSMap | supporting | client-wire, admin |
| [F-CLU-029](01-cluster-membership-and-ha.md#f-clu-029) | Monitor-side laggy detection and its protections | operational | cluster-wire, config |
| [F-CLU-034](01-cluster-membership-and-ha.md#f-clu-034) | Failed-rank and laggy-holder replacement | operational | cluster-wire |
| [F-CLU-038](01-cluster-membership-and-ha.md#f-clu-038) | Rank incarnation | supporting | cluster-wire, client-wire |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-060](01-cluster-membership-and-ha.md#f-clu-060) | MDSMap epoch waits for client requests | supporting | client-wire |
| [F-CLU-077](01-cluster-membership-and-ha.md#f-clu-077) | Rename-specific resolve handling | supporting | mds-wire, client-wire |
| [F-CLU-080](01-cluster-membership-and-ha.md#f-clu-080) | up:reconnect: the client reconnect window and its timeout | supporting | client-wire, config |
| [F-CLU-082](01-cluster-membership-and-ha.md#f-clu-082) | Recording caps and snaprealms from reconnect messages | supporting | client-wire |
| [F-CLU-084](01-cluster-membership-and-ha.md#f-clu-084) | Admission of replayed and retried requests during reconnect | supporting | client-wire |
| [F-CLU-087](01-cluster-membership-and-ha.md#f-clu-087) | Matching reconnected caps to inodes: OpenFileTable prefetch and open by number | supporting | on-disk, client-wire |
| [F-CLU-088](01-cluster-membership-and-ha.md#f-clu-088) | Moving reconnected caps between ranks (IMPORT/EXPORT during rejoin) | supporting | client-wire, mds-wire |
| [F-CLU-089](01-cluster-membership-and-ha.md#f-clu-089) | Forced session open during rejoin *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-092](01-cluster-membership-and-ha.md#f-clu-092) | Snaprealm reconciliation and snapflush rebuild at the end of rejoin | supporting | client-wire |
| [F-CLU-094](01-cluster-membership-and-ha.md#f-clu-094) | Rejoin on a single-rank file system | supporting | client-wire, on-disk |
| [F-CLU-095](01-cluster-membership-and-ha.md#f-clu-095) | up:clientreplay: ordered re-execution of replayed requests | supporting | client-wire |
| [F-CLU-097](01-cluster-membership-and-ha.md#f-clu-097) | Activation (recovery_done and active_start) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-098](01-cluster-membership-and-ha.md#f-clu-098) | Leftover reconnect caps and cap reissue at activation | supporting | client-wire, metrics |
| [F-CLU-099](01-cluster-membership-and-ha.md#f-clu-099) | File size and mtime recovery for writers that did not come back | supporting | client-wire, on-disk, metrics |
| [F-CLU-102](01-cluster-membership-and-ha.md#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk |
| [F-CLU-109](01-cluster-membership-and-ha.md#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config |
| [F-CLU-116](01-cluster-membership-and-ha.md#f-clu-116) | Outbound messaging helpers and local delivery | internal | client-wire, mds-wire |
| [F-PROTO-002](02-client-sessions-and-protocol.md#f-proto-002) | msgr2.1 crc mode for client connections *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-003](02-client-sessions-and-protocol.md#f-proto-003) | msgr2.1 secure mode (AES-128-GCM) *(legacy parts)* | operational | client-wire, config |
| [F-PROTO-004](02-client-sessions-and-protocol.md#f-proto-004) | msgr2 on-wire compression negotiation | internal | client-wire, config |
| [F-PROTO-005](02-client-sessions-and-protocol.md#f-proto-005) | Address vector publication and server identity checks *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-006](02-client-sessions-and-protocol.md#f-proto-006) | Lossless stateful-server connection policy and replay of unacknowledged messages *(legacy parts)* | supporting | client-wire |
| [F-PROTO-007](02-client-sessions-and-protocol.md#f-proto-007) | Peer-reset detection and session-reset signalling *(legacy parts)* | supporting | client-wire |
| [F-PROTO-008](02-client-sessions-and-protocol.md#f-proto-008) | MDS entity name and rank identity on the wire *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-009](02-client-sessions-and-protocol.md#f-proto-009) | Keepalives and connection timeouts *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-011](02-client-sessions-and-protocol.md#f-proto-011) | cephx authorizer verification for service mds *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-012](02-client-sessions-and-protocol.md#f-proto-012) | Rotating mds service secrets | supporting | cluster-wire, config |
| [F-PROTO-015](02-client-sessions-and-protocol.md#f-proto-015) | Connection-to-session binding (identity is the cephx global id) | supporting | client-wire |
| [F-PROTO-016](02-client-sessions-and-protocol.md#f-proto-016) | Encoding primitives and versioned-struct rules | supporting | client-wire |
| [F-PROTO-017](02-client-sessions-and-protocol.md#f-proto-017) | Client message catalog and peer-type filtering | supporting | client-wire, mds-wire |
| [F-PROTO-020](02-client-sessions-and-protocol.md#f-proto-020) | CephFS feature bitmap exchange at session open *(legacy parts)* | supporting | client-wire |
| [F-PROTO-021](02-client-sessions-and-protocol.md#f-proto-021) | Per-bit gated behaviour and feature names *(legacy parts)* | supporting | client-wire, admin |
| [F-PROTO-023](02-client-sessions-and-protocol.md#f-proto-023) | required_client_features enforcement and min_compat_client *(legacy parts)* | operational | client-wire, cluster-wire, admin |
| [F-PROTO-024](02-client-sessions-and-protocol.md#f-proto-024) | Connection-level feature bits consulted by the client protocol *(legacy parts)* | supporting | client-wire |
| [F-PROTO-025](02-client-sessions-and-protocol.md#f-proto-025) | Feature bit 22 numbering conflict (Ceph CHARMAP vs kernel SUBVOLUME_METRICS) *(legacy parts)* | extension | client-wire |
| [F-PROTO-026](02-client-sessions-and-protocol.md#f-proto-026) | Client generations and candidate minimum client floors *(legacy parts)* | operational | client-wire, admin |
| [F-PROTO-027](02-client-sessions-and-protocol.md#f-proto-027) | Session state machine | supporting | client-wire |
| [F-PROTO-028](02-client-sessions-and-protocol.md#f-proto-028) | Session open handshake (REQUEST_OPEN, journaled open, OPEN) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-PROTO-029](02-client-sessions-and-protocol.md#f-proto-029) | Session open admission checks and rejection reasons *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-PROTO-030](02-client-sessions-and-protocol.md#f-proto-030) | REJECT encoding and blocklist signalling to clients *(legacy parts)* | supporting | client-wire |
| [F-PROTO-031](02-client-sessions-and-protocol.md#f-proto-031) | MClientSession encoding and the ops the MDS sends *(legacy parts)* | supporting | client-wire |
| [F-PROTO-032](02-client-sessions-and-protocol.md#f-proto-032) | Session op dispatch and rank-state gating | supporting | client-wire |
| [F-PROTO-034](02-client-sessions-and-protocol.md#f-proto-034) | Client metadata capture and use *(legacy parts)* | operational | client-wire, admin, on-disk |
| [F-PROTO-037](02-client-sessions-and-protocol.md#f-proto-037) | Duplicate REQUEST_OPEN answered with OPEN (NOTIFY_SESSION_STATE) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-038](02-client-sessions-and-protocol.md#f-proto-038) | FORCE_RO: read-only MDS notification | supporting | client-wire |
| [F-PROTO-039](02-client-sessions-and-protocol.md#f-proto-039) | FLUSHMSG / FLUSHMSG_ACK session barrier *(legacy parts)* | supporting | client-wire |
| [F-PROTO-040](02-client-sessions-and-protocol.md#f-proto-040) | REQUEST_FLUSH_MDLOG and the client sync paths | supporting | client-wire |
| [F-PROTO-041](02-client-sessions-and-protocol.md#f-proto-041) | Force-opened sessions (imports, rejoin, cross-MDS rename) | supporting | client-wire, mds-wire, on-disk |
| [F-PROTO-042](02-client-sessions-and-protocol.md#f-proto-042) | Cap renewal (REQUEST_RENEWCAPS / RENEWCAPS) | supporting | client-wire |
| [F-PROTO-043](02-client-sessions-and-protocol.md#f-proto-043) | Stale session detection and the STALE message | supporting | client-wire |
| [F-PROTO-045](02-client-sessions-and-protocol.md#f-proto-045) | Session autoclose eviction | supporting | client-wire, cluster-wire, config |
| [F-PROTO-049](02-client-sessions-and-protocol.md#f-proto-049) | Client-initiated close with push-sequence handshake | supporting | client-wire, on-disk |
| [F-PROTO-050](02-client-sessions-and-protocol.md#f-proto-050) | Counted messages and the session push sequence | supporting | client-wire |
| [F-PROTO-051](02-client-sessions-and-protocol.md#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin |
| [F-PROTO-052](02-client-sessions-and-protocol.md#f-proto-052) | OSDMap blocklist-driven session kill | supporting | client-wire, cluster-wire |
| [F-PROTO-054](02-client-sessions-and-protocol.md#f-proto-054) | Session termination at rank shutdown | operational | client-wire, cluster-wire |
| [F-PROTO-055](02-client-sessions-and-protocol.md#f-proto-055) | Reconnect trigger and client-side ordering | supporting | client-wire |
| [F-PROTO-056](02-client-sessions-and-protocol.md#f-proto-056) | MClientReconnect encodings (v1 to v5, multi-message) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-057](02-client-sessions-and-protocol.md#f-proto-057) | Reconnect processing and outcomes *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-061](02-client-sessions-and-protocol.md#f-proto-061) | Snap realm reconnect | extension | client-wire |
| [F-PROTO-062](02-client-sessions-and-protocol.md#f-proto-062) | Client reactions to MDSMap rank changes | supporting | cluster-wire |
| [F-PROTO-065](02-client-sessions-and-protocol.md#f-proto-065) | MClientRequest versioned head (head v1, v2, v3) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-067](02-client-sessions-and-protocol.md#f-proto-067) | Request message tail: stamp, gid_list, alternate_name, fscrypt fields *(legacy parts)* | supporting | client-wire |
| [F-PROTO-068](02-client-sessions-and-protocol.md#f-proto-068) | Request flags REPLAY, WANT_DENTRY and ASYNC | supporting | client-wire |
| [F-PROTO-069](02-client-sessions-and-protocol.md#f-proto-069) | Request argument union per op | supporting | client-wire |
| [F-PROTO-070](02-client-sessions-and-protocol.md#f-proto-070) | Embedded cap and dentry-lease release records | supporting | client-wire |
| [F-PROTO-071](02-client-sessions-and-protocol.md#f-proto-071) | Request data segment per op | supporting | client-wire |
| [F-PROTO-072](02-client-sessions-and-protocol.md#f-proto-072) | Path encoding on the wire | supporting | client-wire |
| [F-PROTO-073](02-client-sessions-and-protocol.md#f-proto-073) | Op code numbering and the write bit | supporting | client-wire |
| [F-PROTO-074](02-client-sessions-and-protocol.md#f-proto-074) | Client-chosen inode number (head.ino) | supporting | client-wire |
| [F-PROTO-075](02-client-sessions-and-protocol.md#f-proto-075) | Client-side rank selection and the hints the MDS provides | supporting | client-wire, config |
| [F-PROTO-076](02-client-sessions-and-protocol.md#f-proto-076) | Request forwarding (MClientRequestForward) | supporting | client-wire |
| [F-PROTO-077](02-client-sessions-and-protocol.md#f-proto-077) | Retry counters and attempt limits (32BITS_RETRY_FWD) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-078](02-client-sessions-and-protocol.md#f-proto-078) | Request identity, tid ordering and requests without a session | supporting | client-wire |
| [F-PROTO-079](02-client-sessions-and-protocol.md#f-proto-079) | MClientReply head and error codes | supporting | client-wire |
| [F-PROTO-080](02-client-sessions-and-protocol.md#f-proto-080) | Reply trace layout | supporting | client-wire |
| [F-PROTO-081](02-client-sessions-and-protocol.md#f-proto-081) | InodeStat versioned encoding (v9, REPLY_ENCODING) | supporting | client-wire |
| [F-PROTO-083](02-client-sessions-and-protocol.md#f-proto-083) | DirStat and LeaseStat records *(legacy parts)* | supporting | client-wire |
| [F-PROTO-084](02-client-sessions-and-protocol.md#f-proto-084) | Caps, xattrs and inline data carried inside InodeStat *(legacy parts)* | supporting | client-wire |
| [F-PROTO-085](02-client-sessions-and-protocol.md#f-proto-085) | Snap trace encoding and format selection | extension | client-wire |
| [F-PROTO-086](02-client-sessions-and-protocol.md#f-proto-086) | Readdir and LSSNAP reply format and ordering contract *(legacy parts)* | supporting | client-wire |
| [F-PROTO-087](02-client-sessions-and-protocol.md#f-proto-087) | CREATE reply: created inode and delegated inode ranges *(legacy parts)* | supporting | client-wire |
| [F-PROTO-088](02-client-sessions-and-protocol.md#f-proto-088) | GETFILELOCK, GETVXATTR and FILE_BLOCKDIFF extra payloads | supporting | client-wire |
| [F-PROTO-089](02-client-sessions-and-protocol.md#f-proto-089) | Early (unsafe) and safe replies | supporting | client-wire |
| [F-PROTO-090](02-client-sessions-and-protocol.md#f-proto-090) | Reply content invariants enforced by clients | supporting | client-wire |
| [F-PROTO-091](02-client-sessions-and-protocol.md#f-proto-091) | MClientCaps encoding (v14, xattrs in the middle segment) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-092](02-client-sessions-and-protocol.md#f-proto-092) | MClientCapRelease encoding and the OSD epoch barrier | supporting | client-wire |
| [F-PROTO-093](02-client-sessions-and-protocol.md#f-proto-093) | MClientLease encoding | supporting | client-wire |
| [F-PROTO-094](02-client-sessions-and-protocol.md#f-proto-094) | MClientSnap encoding | extension | client-wire |
| [F-PROTO-095](02-client-sessions-and-protocol.md#f-proto-095) | MClientQuota encoding *(legacy parts)* | extension | client-wire |
| [F-PROTO-096](02-client-sessions-and-protocol.md#f-proto-096) | MClientMetrics encoding and client send rules | operational | client-wire, metrics |
| [F-PROTO-097](02-client-sessions-and-protocol.md#f-proto-097) | MQuarantineDisable encoding | extension | client-wire |
| [F-PROTO-098](02-client-sessions-and-protocol.md#f-proto-098) | Mount flow requirements | supporting | client-wire, cluster-wire |
| [F-PROTO-099](02-client-sessions-and-protocol.md#f-proto-099) | Inode-number addressed access (NFS file handles, #ino paths, LOOKUPINO) | supporting | client-wire |
| [F-PROTO-100](02-client-sessions-and-protocol.md#f-proto-100) | Inode number ranges clients accept | supporting | client-wire |
| [F-CAPS-001](03-caps-leases-and-locking.md#f-caps-001) | Cap bit model and composite masks | supporting | client-wire |
| [F-CAPS-002](03-caps-leases-and-locking.md#f-caps-002) | Open mode to wanted caps | supporting | client-wire |
| [F-CAPS-003](03-caps-leases-and-locking.md#f-caps-003) | Capability record, cap ids and sequence numbers | supporting | client-wire |
| [F-CAPS-004](03-caps-leases-and-locking.md#f-caps-004) | Lock-state-derived allowed caps *(legacy parts)* | supporting | client-wire |
| [F-CAPS-009](03-caps-leases-and-locking.md#f-caps-009) | Caps in request replies *(legacy parts)* | supporting | client-wire |
| [F-CAPS-010](03-caps-leases-and-locking.md#f-caps-010) | New caps on open, create, mknod and mkdir | supporting | client-wire |
| [F-CAPS-011](03-caps-leases-and-locking.md#f-caps-011) | Cap grant and revoke messages (GRANT/REVOKE) | supporting | client-wire |
| [F-CAPS-013](03-caps-leases-and-locking.md#f-caps-013) | Client cap message processing (handle_client_caps) | supporting | client-wire |
| [F-CAPS-014](03-caps-leases-and-locking.md#f-caps-014) | Wanted caps tracking (issue_seq rule) | supporting | client-wire |
| [F-CAPS-015](03-caps-leases-and-locking.md#f-caps-015) | Xattr blob and inline data carried on caps *(legacy parts)* | supporting | client-wire |
| [F-CAPS-016](03-caps-leases-and-locking.md#f-caps-016) | Truncate notification (CEPH_CAP_OP_TRUNC) | supporting | client-wire |
| [F-CAPS-018](03-caps-leases-and-locking.md#f-caps-018) | Dirty metadata flush and FLUSH_ACK *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CAPS-019](03-caps-leases-and-locking.md#f-caps-019) | Flush idempotency (completed_flushes, oldest_flush_tid) | supporting | client-wire, on-disk, metrics |
| [F-CAPS-020](03-caps-leases-and-locking.md#f-caps-020) | Synchronous flush and journal-flush heuristics | supporting | client-wire, config |
| [F-CAPS-021](03-caps-leases-and-locking.md#f-caps-021) | max_size and per-client writeable ranges (client_ranges) | supporting | client-wire, on-disk, config |
| [F-CAPS-022](03-caps-leases-and-locking.md#f-caps-022) | File size recovery after loss of a writer | supporting | cluster-wire, on-disk |
| [F-CAPS-023](03-caps-leases-and-locking.md#f-caps-023) | Snapshot COW of cap state and SNAP_SYNC gating | extension | on-disk |
| [F-CAPS-024](03-caps-leases-and-locking.md#f-caps-024) | FLUSHSNAP / FLUSHSNAP_ACK protocol | extension | client-wire, on-disk |
| [F-CAPS-025](03-caps-leases-and-locking.md#f-caps-025) | Null snapflush inference | extension | client-wire, on-disk |
| [F-CAPS-026](03-caps-leases-and-locking.md#f-caps-026) | Snapflush nudge | extension | config |
| [F-CAPS-028](03-caps-leases-and-locking.md#f-caps-028) | Batched cap release (MClientCapRelease) | supporting | client-wire |
| [F-CAPS-029](03-caps-leases-and-locking.md#f-caps-029) | Cap and dentry-lease releases embedded in requests | supporting | client-wire |
| [F-CAPS-030](03-caps-leases-and-locking.md#f-caps-030) | Stale-session cap handling *(legacy parts)* | supporting | client-wire, config |
| [F-CAPS-032](03-caps-leases-and-locking.md#f-caps-032) | Cache-pressure cap recall (CEPH_SESSION_RECALL_STATE) | operational | client-wire, metrics, config, admin |
| [F-CAPS-034](03-caps-leases-and-locking.md#f-caps-034) | Cap re-establishment after MDS failover (cap side) | supporting | client-wire |
| [F-CAPS-036](03-caps-leases-and-locking.md#f-caps-036) | Cap migration to clients (EXPORT / IMPORT) | supporting | client-wire, mds-wire |
| [F-CAPS-038](03-caps-leases-and-locking.md#f-caps-038) | Dentry lease issue | supporting | client-wire |
| [F-CAPS-039](03-caps-leases-and-locking.md#f-caps-039) | Negative dentry leases | supporting | client-wire |
| [F-CAPS-043](03-caps-leases-and-locking.md#f-caps-043) | Lease revocation on dentry change | supporting | client-wire |
| [F-CAPS-044](03-caps-leases-and-locking.md#f-caps-044) | Lease expiry and cleanup | supporting | client-wire, config |
| [F-CAPS-046](03-caps-leases-and-locking.md#f-caps-046) | Directory-wide dentry validity via Fs | supporting | client-wire |
| [F-CAPS-059](03-caps-leases-and-locking.md#f-caps-059) | Replay of unsafe and async creates after failover | supporting | client-wire, config |
| [F-CAPS-067](03-caps-leases-and-locking.md#f-caps-067) | Gather engine (eval_gather) and cap revocation gather | supporting | internal |
| [F-CAPS-081](03-caps-leases-and-locking.md#f-caps-081) | Implicit quiescelock wrlock | extension | internal |
| [F-CAPS-092](03-caps-leases-and-locking.md#f-caps-092) | XLOCKDONE: read-your-own-unsafe-writes | supporting | client-wire, config |
| [F-NS-001](04-namespace-and-posix-operations.md#f-ns-001) | Client op set and dispatch switch | required | client-wire |
| [F-NS-002](04-namespace-and-posix-operations.md#f-ns-002) | Request intake gating by rank state and session | supporting | client-wire |
| [F-NS-003](04-namespace-and-posix-operations.md#f-ns-003) | Request identity and in-flight duplicate detection | supporting | client-wire |
| [F-NS-010](04-namespace-and-posix-operations.md#f-ns-010) | Journaled mutation commit and safe reply | supporting | client-wire, on-disk |
| [F-NS-011](04-namespace-and-posix-operations.md#f-ns-011) | Early (unsafe) replies | supporting | client-wire, config |
| [F-NS-012](04-namespace-and-posix-operations.md#f-ns-012) | Completed-request table and idempotent retries/replays | supporting | client-wire, on-disk, metrics |
| [F-NS-013](04-namespace-and-posix-operations.md#f-ns-013) | Replay of unsafe requests after an MDS restart | supporting | client-wire |
| [F-NS-014](04-namespace-and-posix-operations.md#f-ns-014) | Per-op reply trace contents | supporting | client-wire |
| [F-NS-016](04-namespace-and-posix-operations.md#f-ns-016) | On-demand journal flush for fsync, syncfs and sync writes | required | client-wire |
| [F-NS-017](04-namespace-and-posix-operations.md#f-ns-017) | Read-only and snapshot EROFS gates | required | client-wire |
| [F-NS-018](04-namespace-and-posix-operations.md#f-ns-018) | Full metadata pool gate | required | client-wire, cluster-wire |
| [F-NS-019](04-namespace-and-posix-operations.md#f-ns-019) | Create/remove name gates and their error order | required | client-wire |
| [F-NS-020](04-namespace-and-posix-operations.md#f-ns-020) | Directory entry limits (ENOSPC) | operational | client-wire, config |
| [F-NS-021](04-namespace-and-posix-operations.md#f-ns-021) | Permission checking split: clients enforce POSIX, the MDS enforces auth caps *(legacy parts)* | required | client-wire |
| [F-NS-022](04-namespace-and-posix-operations.md#f-ns-022) | ESTALE recovery of unknown inode numbers | supporting | client-wire, mds-wire, on-disk |
| [F-NS-023](04-namespace-and-posix-operations.md#f-ns-023) | GETATTR (stat by inode) | required | client-wire |
| [F-NS-024](04-namespace-and-posix-operations.md#f-ns-024) | LOOKUP of one name in a directory | required | client-wire |
| [F-NS-025](04-namespace-and-posix-operations.md#f-ns-025) | Negative lookup replies (null-dentry traces) | supporting | client-wire |
| [F-NS-029](04-namespace-and-posix-operations.md#f-ns-029) | Reads at a snapshot id (LOOKUPSNAP, snapshot LOOKUPINO, open and readdir in snapshots) | extension | client-wire |
| [F-NS-030](04-namespace-and-posix-operations.md#f-ns-030) | OPEN of an existing file or directory *(legacy parts)* | required | client-wire |
| [F-NS-031](04-namespace-and-posix-operations.md#f-ns-031) | O_TRUNC on open | required | client-wire, on-disk |
| [F-NS-032](04-namespace-and-posix-operations.md#f-ns-032) | READDIR paging of one dirfrag in hash order *(legacy parts)* | required | client-wire |
| [F-NS-033](04-namespace-and-posix-operations.md#f-ns-033) | Readdir across dirfrags: frag selection and adjustment *(legacy parts)* | required | client-wire |
| [F-NS-034](04-namespace-and-posix-operations.md#f-ns-034) | Readdir cap-acquisition throttle | operational | config, metrics |
| [F-NS-035](04-namespace-and-posix-operations.md#f-ns-035) | CREATE (open with O_CREAT, optionally O_EXCL) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-036](04-namespace-and-posix-operations.md#f-ns-036) | MKNOD (regular file, FIFO, device, socket) | required | client-wire, on-disk |
| [F-NS-037](04-namespace-and-posix-operations.md#f-ns-037) | MKDIR *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-038](04-namespace-and-posix-operations.md#f-ns-038) | SYMLINK (and readlink without an MDS op) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-039](04-namespace-and-posix-operations.md#f-ns-039) | New inode initialization *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-041](04-namespace-and-posix-operations.md#f-ns-041) | Client-specified inode number (head.ino) on create | supporting | client-wire |
| [F-NS-042](04-namespace-and-posix-operations.md#f-ns-042) | File layout selection and validation at create | extension | client-wire |
| [F-NS-043](04-namespace-and-posix-operations.md#f-ns-043) | Parent directory updates on link and unlink | required | client-wire, on-disk |
| [F-NS-044](04-namespace-and-posix-operations.md#f-ns-044) | LINK (hard link) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-045](04-namespace-and-posix-operations.md#f-ns-045) | UNLINK of a primary link | required | client-wire, on-disk |
| [F-NS-046](04-namespace-and-posix-operations.md#f-ns-046) | UNLINK of a remote (hard-link) dentry | required | client-wire, on-disk |
| [F-NS-047](04-namespace-and-posix-operations.md#f-ns-047) | RMDIR with emptiness checks | required | client-wire, on-disk |
| [F-NS-048](04-namespace-and-posix-operations.md#f-ns-048) | Unlinked-but-open inodes stay usable until the last cap is released | required | on-disk |
| [F-NS-049](04-namespace-and-posix-operations.md#f-ns-049) | RENAME of a file or directory (move, same-inode no-op) | required | client-wire, on-disk |
| [F-NS-050](04-namespace-and-posix-operations.md#f-ns-050) | RENAME over an existing target | required | client-wire, on-disk |
| [F-NS-051](04-namespace-and-posix-operations.md#f-ns-051) | RENAME validation and error order *(legacy parts)* | required | client-wire |
| [F-NS-053](04-namespace-and-posix-operations.md#f-ns-053) | Which rank serves each op (auth requirement, forward or discover) | supporting | client-wire, mds-wire, config |
| [F-NS-057](04-namespace-and-posix-operations.md#f-ns-057) | Inode authority and cap migration inside a cross-rank rename *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-NS-058](04-namespace-and-posix-operations.md#f-ns-058) | SETATTR request: mask, locks, error order and common tail *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-059](04-namespace-and-posix-operations.md#f-ns-059) | chmod and chown, with set-user/group-ID clearing | required | client-wire |
| [F-NS-060](04-namespace-and-posix-operations.md#f-ns-060) | Explicit times (utimes) and btime, with time_warp_seq *(legacy parts)* | required | client-wire |
| [F-NS-061](04-namespace-and-posix-operations.md#f-ns-061) | Size growth through SETATTR | required | client-wire |
| [F-NS-062](04-namespace-and-posix-operations.md#f-ns-062) | Truncate (shrink) with asynchronous OSD truncation | required | client-wire, cluster-wire, on-disk |
| [F-NS-063](04-namespace-and-posix-operations.md#f-ns-063) | Timestamps from the client clock and change_attr | required | client-wire, on-disk |
| [F-NS-066](04-namespace-and-posix-operations.md#f-ns-066) | SETXATTR and RMXATTR of stored xattrs | supporting | client-wire, on-disk |
| [F-NS-067](04-namespace-and-posix-operations.md#f-ns-067) | Total xattr size limit (max_xattr_size) | supporting | client-wire, cluster-wire, admin |
| [F-NS-068](04-namespace-and-posix-operations.md#f-ns-068) | xattr blob delivery to clients | supporting | client-wire |
| [F-NS-069](04-namespace-and-posix-operations.md#f-ns-069) | POSIX ACLs stored as opaque xattrs | supporting | client-wire, on-disk |
| [F-NS-071](04-namespace-and-posix-operations.md#f-ns-071) | Virtual xattr set/remove path (common rules, p flag) | extension | client-wire, admin |
| [F-NS-072](04-namespace-and-posix-operations.md#f-ns-072) | Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*) *(legacy parts)* | extension | client-wire, cluster-wire |
| [F-NS-073](04-namespace-and-posix-operations.md#f-ns-073) | GETVXATTR op | extension | client-wire |
| [F-NS-074](04-namespace-and-posix-operations.md#f-ns-074) | Client-synthesized ceph.* vxattrs (InodeStat contract) | extension | client-wire |
| [F-NS-075](04-namespace-and-posix-operations.md#f-ns-075) | Case-insensitive and normalized directories (charmap) *(legacy parts)* | extension | client-wire, on-disk |
| [F-NS-077](04-namespace-and-posix-operations.md#f-ns-077) | SETFILELOCK: POSIX fcntl byte-range locks | required | client-wire |
| [F-NS-078](04-namespace-and-posix-operations.md#f-ns-078) | BSD flock whole-file locks | extension | client-wire |
| [F-NS-079](04-namespace-and-posix-operations.md#f-ns-079) | Blocking lock waits and wake-ups | required | client-wire |
| [F-NS-080](04-namespace-and-posix-operations.md#f-ns-080) | Lock wait interruption (FCNTL_INTR / FLOCK_INTR) | required | client-wire |
| [F-NS-081](04-namespace-and-posix-operations.md#f-ns-081) | fcntl deadlock detection (EDEADLK) | required | client-wire |
| [F-NS-082](04-namespace-and-posix-operations.md#f-ns-082) | GETFILELOCK (F_GETLK) | required | client-wire |
| [F-NS-083](04-namespace-and-posix-operations.md#f-ns-083) | Lock cleanup on cap removal and session end | required | client-wire |
| [F-NS-084](04-namespace-and-posix-operations.md#f-ns-084) | Lock reinstatement after an MDS restart *(legacy parts)* | required | client-wire |
| [F-NS-085](04-namespace-and-posix-operations.md#f-ns-085) | Directory link count and size presentation | required | client-wire |
| [F-NS-086](04-namespace-and-posix-operations.md#f-ns-086) | Access time is not maintained by the MDS | required | client-wire |
| [F-NS-087](04-namespace-and-posix-operations.md#f-ns-087) | Timestamp range limited to unsigned 32-bit seconds | required | client-wire, on-disk |
| [F-NS-088](04-namespace-and-posix-operations.md#f-ns-088) | Limits enforced only by clients, and no hard-link limit | required | client-wire |
| [F-NS-089](04-namespace-and-posix-operations.md#f-ns-089) | Unsupported Linux extensions: renameat2 flags and O_TMPFILE | required | client-wire |
| [F-NS-090](04-namespace-and-posix-operations.md#f-ns-090) | Data-path coherency departures outside the MDS | required | client-wire |
| [F-NS-091](04-namespace-and-posix-operations.md#f-ns-091) | statfs is answered by the monitors | required | cluster-wire |
| [F-CACHE-001](05-metadata-cache-and-dirfrags.md#f-cache-001) | Inode record field set (inode_t) | required | on-disk, client-wire |
| [F-CACHE-003](05-metadata-cache-and-dirfrags.md#f-cache-003) | Refusal of client operations on private and system inodes | supporting | client-wire |
| [F-CACHE-004](05-metadata-cache-and-dirfrags.md#f-cache-004) | Timestamps, time_warp_seq and change_attr *(legacy parts)* | required | client-wire, on-disk |
| [F-CACHE-006](05-metadata-cache-and-dirfrags.md#f-cache-006) | Truncation state fields | required | client-wire, on-disk |
| [F-CACHE-007](05-metadata-cache-and-dirfrags.md#f-cache-007) | File layout field and layout inheritance *(legacy parts)* | extension | client-wire, on-disk |
| [F-CACHE-010](05-metadata-cache-and-dirfrags.md#f-cache-010) | Inode version and client-visible version | supporting | client-wire |
| [F-CACHE-014](05-metadata-cache-and-dirfrags.md#f-cache-014) | Projected dentry linkage (primary, remote, null) | supporting | on-disk |
| [F-CACHE-041](05-metadata-cache-and-dirfrags.md#f-cache-041) | Path traversal (path_traverse) | required | client-wire |
| [F-CACHE-044](05-metadata-cache-and-dirfrags.md#f-cache-044) | Forward-or-discover for non-auth objects | supporting | client-wire, config |
| [F-CACHE-045](05-metadata-cache-and-dirfrags.md#f-cache-045) | Hard-link target resolution | required | client-wire |
| [F-CACHE-047](05-metadata-cache-and-dirfrags.md#f-cache-047) | Lookup by inode number (open_ino) | supporting | client-wire, on-disk, mds-wire |
| [F-CACHE-050](05-metadata-cache-and-dirfrags.md#f-cache-050) | ESTALE recovery via find_ino_peers | supporting | client-wire, mds-wire |
| [F-CACHE-060](05-metadata-cache-and-dirfrags.md#f-cache-060) | Pin reasons that block eviction | operational | internal |
| [F-CACHE-071](05-metadata-cache-and-dirfrags.md#f-cache-071) | Root and system inode initial attributes and default layouts | required | client-wire, on-disk, config |
| [F-CACHE-072](05-metadata-cache-and-dirfrags.md#f-cache-072) | Global snaprealm dummy inode | supporting | client-wire |
| [F-CACHE-074](05-metadata-cache-and-dirfrags.md#f-cache-074) | Fragment identifier and fragment tree (frag_t, fragtree_t) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CACHE-075](05-metadata-cache-and-dirfrags.md#f-cache-075) | Dentry name hash per directory (dl_dir_hash) *(legacy parts)* | supporting | client-wire, on-disk, config |
| [F-CACHE-076](05-metadata-cache-and-dirfrags.md#f-cache-076) | Fragtree and dir_layout in InodeStat *(legacy parts)* | supporting | client-wire |
| [F-CACHE-077](05-metadata-cache-and-dirfrags.md#f-cache-077) | Hash-ordered dentry storage and readdir order *(legacy parts)* | required | client-wire |
| [F-CACHE-078](05-metadata-cache-and-dirfrags.md#f-cache-078) | Readdir fragment mapping and resume across refragmentation | required | client-wire |
| [F-CACHE-079](05-metadata-cache-and-dirfrags.md#f-cache-079) | DirStat delegation hints *(legacy parts)* | supporting | client-wire, config |
| [F-CACHE-080](05-metadata-cache-and-dirfrags.md#f-cache-080) | Per-dirfrag size limit (ENOSPC) | operational | client-wire, config |
| [F-CACHE-081](05-metadata-cache-and-dirfrags.md#f-cache-081) | Per-directory entry limit | operational | client-wire, config |
| [F-CACHE-097](05-metadata-cache-and-dirfrags.md#f-cache-097) | Effects of a fragment freeze on clients | supporting | client-wire |
| [F-STORE-007](06-journal-and-storage.md#f-store-007) | Per-rank write-ahead metadata journal with one submission order | supporting | on-disk |
| [F-STORE-012](06-journal-and-storage.md#f-store-012) | Safe reply only after the journal commit (early unsafe reply) | supporting | client-wire |
| [F-STORE-026](06-journal-and-storage.md#f-store-026) | EUpdate: generic mutation record | supporting | on-disk |
| [F-STORE-028](06-journal-and-storage.md#f-store-028) | Session events: ESession and ESessions *(legacy parts)* | supporting | on-disk |
| [F-STORE-029](06-journal-and-storage.md#f-store-029) | Completed client request ids in journal and SessionMap | supporting | on-disk |
| [F-STORE-046](06-journal-and-storage.md#f-store-046) | Special inode numbers and per-rank ranges *(legacy parts)* | supporting | client-wire, on-disk |
| [F-STORE-062](06-journal-and-storage.md#f-store-062) | Lookup by inode number through backtraces (open_ino storage path) | supporting | client-wire, on-disk |
| [F-STORE-064](06-journal-and-storage.md#f-store-064) | Data object naming and striping shared with clients | supporting | client-wire, on-disk |
| [F-STORE-070](06-journal-and-storage.md#f-store-070) | Inode number allocation in create paths | supporting | on-disk, client-wire |
| [F-STORE-073](06-journal-and-storage.md#f-store-073) | Inode numbers are never reused after deletion | required | client-wire, on-disk |
| [F-STORE-098](06-journal-and-storage.md#f-store-098) | OSD epoch barrier in cap messages | supporting | client-wire, admin |
| [F-STORE-099](06-journal-and-storage.md#f-store-099) | Metadata-pool-full ENOSPC policy | required | client-wire |
| [F-STORE-100](06-journal-and-storage.md#f-store-100) | Metadata write-error policy and read-only mode | operational | client-wire, config, metrics |
| [F-MULTI-012](07-multi-mds.md#f-multi-012) | Cap EXPORT message to clients (CEPH_CAP_OP_EXPORT) | supporting | client-wire |
| [F-MULTI-013](07-multi-mds.md#f-multi-013) | Cap IMPORT message to clients and auth-cap switch (CEPH_CAP_OP_IMPORT) *(legacy parts)* | supporting | client-wire |
| [F-MULTI-014](07-multi-mds.md#f-multi-014) | Migrate sequence (mseq) and stale cap-message handling | supporting | client-wire |
| [F-MULTI-015](07-multi-mds.md#f-multi-015) | Writeable-range reconciliation on cap import | supporting | internal |
| [F-MULTI-016](07-multi-mds.md#f-multi-016) | Client session flush barrier before shipping (FLUSHMSG) *(legacy parts)* | supporting | client-wire |
| [F-MULTI-017](07-multi-mds.md#f-multi-017) | Forced client session open on the importer | supporting | client-wire, on-disk |
| [F-MULTI-018](07-multi-mds.md#f-multi-018) | Request blocking and forwarding around migration | supporting | client-wire |
| [F-MULTI-019](07-multi-mds.md#f-multi-019) | Export-target publication (MMDSLoadTargets, MDSMap export_targets) | supporting | cluster-wire, client-wire |
| [F-MULTI-020](07-multi-mds.md#f-multi-020) | Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps) | supporting | mds-wire, client-wire, on-disk |
| [F-MULTI-038](07-multi-mds.md#f-multi-038) | Set and remove ceph.dir.pin | extension | client-wire, on-disk |
| [F-MULTI-039](07-multi-mds.md#f-multi-039) | Pin read-back (InodeStat dir_pin, GETVXATTR) *(legacy parts)* | extension | client-wire |
| [F-MULTI-040](07-multi-mds.md#f-multi-040) | Pin inheritance and precedence resolution | extension | internal |
| [F-MULTI-041](07-multi-mds.md#f-multi-041) | Pin enforcement and aux subtrees (handle_export_pins) | extension | internal |
| [F-MULTI-045](07-multi-mds.md#f-multi-045) | Distributed ephemeral pins (ceph.dir.pin.distributed) | extension | client-wire, on-disk |
| [F-MULTI-046](07-multi-mds.md#f-multi-046) | Random ephemeral pins (ceph.dir.pin.random) | extension | client-wire, on-disk |
| [F-SNAP-001](08-snapshots-quotas-subvolumes.md#f-snap-001) | Snapshot id and realm seq allocation from one monotonic counter | extension | client-wire, cluster-wire, on-disk |
| [F-SNAP-006](08-snapshots-quotas-subvolumes.md#f-snap-006) | Snap realms and effective snapshot sets *(legacy parts)* | extension | client-wire, on-disk |
| [F-SNAP-007](08-snapshots-quotas-subvolumes.md#f-snap-007) | Realm membership of caps and realm split on creation | extension | client-wire |
| [F-SNAP-008](08-snapshots-quotas-subvolumes.md#f-snap-008) | Snap trace contents and client interpretation | extension | client-wire |
| [F-SNAP-009](08-snapshots-quotas-subvolumes.md#f-snap-009) | Per-realm MClientSnap notification (always op SPLIT) | extension | client-wire |
| [F-SNAP-010](08-snapshots-quotas-subvolumes.md#f-snap-010) | Global realm broadcast on every snap-table operation | extension | client-wire |
| [F-SNAP-011](08-snapshots-quotas-subvolumes.md#f-snap-011) | Hard-link global realm (PARENT_GLOBAL) | extension | client-wire, on-disk |
| [F-SNAP-012](08-snapshots-quotas-subvolumes.md#f-snap-012) | Past-parent snapshots on rename and unlink | extension | client-wire, on-disk |
| [F-SNAP-013](08-snapshots-quotas-subvolumes.md#f-snap-013) | Create a snapshot (MKSNAP, mkdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-014](08-snapshots-quotas-subvolumes.md#f-snap-014) | Remove a snapshot (RMSNAP, rmdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-015](08-snapshots-quotas-subvolumes.md#f-snap-015) | Rename a snapshot (RENAMESNAP) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-019](08-snapshots-quotas-subvolumes.md#f-snap-019) | Per-directory snapshot limit (mds_max_snaps_per_dir) | extension | config, client-wire |
| [F-SNAP-021](08-snapshots-quotas-subvolumes.md#f-snap-021) | Snapshot path encoding, traversal and LOOKUPSNAP | extension | client-wire |
| [F-SNAP-022](08-snapshots-quotas-subvolumes.md#f-snap-022) | List snapshots (LSSNAP, readdir of .snap) | extension | client-wire |
| [F-SNAP-023](08-snapshots-quotas-subvolumes.md#f-snap-023) | Read-only snapshot semantics and snapshot caps | extension | client-wire |
| [F-SNAP-024](08-snapshots-quotas-subvolumes.md#f-snap-024) | Snapshot attributes in InodeStat (snap_btime, snap_metadata) | extension | client-wire |
| [F-SNAP-026](08-snapshots-quotas-subvolumes.md#f-snap-026) | Metadata copy-on-write (first/last ranges, snapped dentries, old_inodes) | extension | on-disk |
| [F-SNAP-032](08-snapshots-quotas-subvolumes.md#f-snap-032) | Snaprealm reconciliation at client reconnect and rejoin | extension | client-wire |
| [F-SNAP-033](08-snapshots-quotas-subvolumes.md#f-snap-033) | Directory entry statistics (dirstat / fragstat) | required | client-wire, on-disk |
| [F-SNAP-034](08-snapshots-quotas-subvolumes.md#f-snap-034) | Recursive subtree statistics (rstat) | extension | client-wire, on-disk |
| [F-SNAP-039](08-snapshots-quotas-subvolumes.md#f-snap-039) | rstat reads routed to the auth rank (GETATTR CEPH_STAT_RSTAT) | extension | client-wire |
| [F-SNAP-040](08-snapshots-quotas-subvolumes.md#f-snap-040) | Snapshot rstats (mds_snap_rstat) | extension | config, on-disk |
| [F-SNAP-042](08-snapshots-quotas-subvolumes.md#f-snap-042) | Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files) | extension | client-wire, on-disk |
| [F-SNAP-043](08-snapshots-quotas-subvolumes.md#f-snap-043) | Quota directories become snaprealm roots | extension | client-wire, on-disk |
| [F-SNAP-044](08-snapshots-quotas-subvolumes.md#f-snap-044) | Quota and usage in InodeStat *(legacy parts)* | extension | client-wire |
| [F-SNAP-045](08-snapshots-quotas-subvolumes.md#f-snap-045) | MClientQuota usage pushes with thresholds *(legacy parts)* | extension | client-wire |
| [F-SNAP-047](08-snapshots-quotas-subvolumes.md#f-snap-047) | Client-side quota enforcement (EDQUOT, cross-quota EXDEV) | extension | client-wire |
| [F-SNAP-048](08-snapshots-quotas-subvolumes.md#f-snap-048) | Quota-based statfs | extension | client-wire |
| [F-SNAP-052](08-snapshots-quotas-subvolumes.md#f-snap-052) | Cross-subvolume link and rename rejected with EXDEV | extension | client-wire |
| [F-SNAP-053](08-snapshots-quotas-subvolumes.md#f-snap-053) | Snapshots only at the subvolume root | extension | client-wire |
| [F-SNAP-054](08-snapshots-quotas-subvolumes.md#f-snap-054) | Subvolume id on the wire (InodeStat, MClientCaps) | extension | client-wire |
| [F-SNAP-055](08-snapshots-quotas-subvolumes.md#f-snap-055) | Snapshot visibility flag (ceph.dir.subvolume.snaps.visible) | extension | client-wire, on-disk |
| [F-BG-002](09-background-services.md#f-bg-002) | Deferred deletion: unlink, rmdir and rename-over move the inode to a stray | required | client-wire, on-disk |
| [F-BG-004](09-background-services.md#f-bg-004) | ENOSPC on unlink when the current stray dirfrag is full | operational | client-wire, config |
| [F-BG-006](09-background-services.md#f-bg-006) | Purge eligibility decision (eval_stray) | supporting | internal |
| [F-BG-008](09-background-services.md#f-bg-008) | Prompt cap release for unlinked inodes (nlink 0 grant with Ls) | supporting | client-wire |
| [F-BG-025](09-background-services.md#f-bg-025) | Purging inodes are invisible to clients (ENOENT / ESTALE) | supporting | client-wire |
| [F-BG-026](09-background-services.md#f-bg-026) | Namespace rules for stray and unlinked directories | required | client-wire |
| [F-BG-060](09-background-services.md#f-bg-060) | EIO for requests that touch known damage | required | client-wire |
| [F-BG-061](09-background-services.md#f-bg-061) | readdir behaviour on damaged directories | required | client-wire |
| [F-QOS-001](10-qos-metrics-quiesce-quarantine.md#f-qos-001) | QoS admission gate for client metadata requests | operational | config, admin |
| [F-QOS-002](10-qos-metrics-quiesce-quarantine.md#f-qos-002) | QoS bucket classification from the session mount root | operational | client-wire, admin |
| [F-QOS-004](10-qos-metrics-quiesce-quarantine.md#f-qos-004) | dmClock tag-based scheduling per bucket | operational | internal |
| [F-QOS-011](10-qos-metrics-quiesce-quarantine.md#f-qos-011) | Client metrics negotiation (feature bit 14 and metric_spec) *(legacy parts)* | operational | client-wire |
| [F-QOS-012](10-qos-metrics-quiesce-quarantine.md#f-qos-012) | MClientMetrics ingestion of per-client metrics (types 0..9) *(legacy parts)* | operational | client-wire |
| [F-QOS-014](10-qos-metrics-quiesce-quarantine.md#f-qos-014) | Metrics accepted on any active rank (client send rules) | operational | client-wire |
| [F-QOS-015](10-qos-metrics-quiesce-quarantine.md#f-qos-015) | Subvolume id in InodeStat and cap flush acks | extension | client-wire |
| [F-QOS-016](10-qos-metrics-quiesce-quarantine.md#f-qos-016) | Subvolume I/O metrics (type 16) ingestion and path resolution | operational | client-wire, metrics |
| [F-QOS-044](10-qos-metrics-quiesce-quarantine.md#f-qos-044) | Per-inode quiesce request (quiesce_inode) | extension | client-wire |
| [F-QOS-045](10-qos-metrics-quiesce-quarantine.md#f-qos-045) | quiescelock and parking of mutations | extension | internal |
| [F-QOS-046](10-qos-metrics-quiesce-quarantine.md#f-qos-046) | Cap masking while quiesced | extension | client-wire |
| [F-QOS-052](10-qos-metrics-quiesce-quarantine.md#f-qos-052) | ceph.quiesce.block vxattr | extension | client-wire, on-disk |
| [F-QOS-064](10-qos-metrics-quiesce-quarantine.md#f-qos-064) | Quarantine cap-revocation walk | extension | client-wire |
| [F-QOS-067](10-qos-metrics-quiesce-quarantine.md#f-qos-067) | Quarantine re-enforcement after restart | extension | client-wire |
| [F-QOS-069](10-qos-metrics-quiesce-quarantine.md#f-qos-069) | MDS auth cap flags q and Q (quarantine access) | extension | cluster-wire, admin |
| [F-QOS-070](10-qos-metrics-quiesce-quarantine.md#f-qos-070) | Quarantine client feature bit 24 | extension | client-wire |
| [F-QOS-071](10-qos-metrics-quiesce-quarantine.md#f-qos-071) | EACCES for quarantine-aware sessions | extension | client-wire |
| [F-QOS-072](10-qos-metrics-quiesce-quarantine.md#f-qos-072) | Request parking for quarantine-unaware sessions | extension | client-wire |
| [F-QOS-074](10-qos-metrics-quiesce-quarantine.md#f-qos-074) | Cap restriction to PIN under quarantine | extension | client-wire |
| [F-QOS-075](10-qos-metrics-quiesce-quarantine.md#f-qos-075) | EQUARANTINED errno in MClientCaps | extension | client-wire |
| [F-QOS-076](10-qos-metrics-quiesce-quarantine.md#f-qos-076) | QUARANTINE marker in InodeStat | extension | client-wire |
| [F-QOS-077](10-qos-metrics-quiesce-quarantine.md#f-qos-077) | MQuarantineDisable notification | extension | client-wire |
| [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) | Peer authorizer verification and caps loading at connection accept *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-ADM-007](11-security-admin-and-observability.md#f-adm-007) | Grant evaluation: union of grants in a fixed step order | extension | client-wire, admin |
| [F-ADM-008](11-security-admin-and-observability.md#f-adm-008) | path= restriction and the inode path used for checks | extension | client-wire, admin |
| [F-ADM-009](11-security-admin-and-observability.md#f-adm-009) | fsname= restriction and file-system gating of session messages | extension | client-wire, admin |
| [F-ADM-010](11-security-admin-and-observability.md#f-adm-010) | uid=/gids= restriction with simplified mode-bit check | extension | client-wire, admin |
| [F-ADM-011](11-security-admin-and-observability.md#f-adm-011) | root_squash grants | extension | client-wire, admin |
| [F-ADM-012](11-security-admin-and-observability.md#f-adm-012) | p flag: permission to set ceph.* vxattrs and non-default layouts | extension | client-wire, admin |
| [F-ADM-013](11-security-admin-and-observability.md#f-adm-013) | s flag and snapshot uid range | extension | client-wire, config |
| [F-ADM-015](11-security-admin-and-observability.md#f-adm-015) | q/Q flags: access to quarantined subtrees | extension | client-wire, admin |
| [F-ADM-017](11-security-admin-and-observability.md#f-adm-017) | Mount root check at session open and auth_name check at reclaim *(legacy parts)* | supporting | client-wire, on-disk |
| [F-ADM-018](11-security-admin-and-observability.md#f-adm-018) | Per-request access-check matrix *(legacy parts)* | extension | client-wire |
| [F-ADM-021](11-security-admin-and-observability.md#f-adm-021) | cap_auths export in CEPH_SESSION_OPEN and client-side checks *(legacy parts)* | extension | client-wire |
| [F-ADM-042](11-security-admin-and-observability.md#f-adm-042) | force_readonly | operational | admin, client-wire |
| [F-ADM-064](11-security-admin-and-observability.md#f-adm-064) | MDS option names read or written by peers | operational | config |

## ganesha

NFS-Ganesha FSAL_CEPH (via libcephfs). 324 features.

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-002](01-cluster-membership-and-ha.md#f-clu-002) | MDSMap: per-file-system map and its wire encoding *(legacy parts)* | supporting | cluster-wire, client-wire |
| [F-CLU-003](01-cluster-membership-and-ha.md#f-clu-003) | Epoch model: FSMap, MDSMap and standby epochs, null map | operational | cluster-wire, client-wire |
| [F-CLU-004](01-cluster-membership-and-ha.md#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire |
| [F-CLU-005](01-cluster-membership-and-ha.md#f-clu-005) | FSMapUser: file system name directory for clients | operational | cluster-wire, client-wire |
| [F-CLU-007](01-cluster-membership-and-ha.md#f-clu-007) | libcephfs map use and mount availability check | supporting | client-wire |
| [F-CLU-029](01-cluster-membership-and-ha.md#f-clu-029) | Monitor-side laggy detection and its protections | operational | cluster-wire, config |
| [F-CLU-034](01-cluster-membership-and-ha.md#f-clu-034) | Failed-rank and laggy-holder replacement | operational | cluster-wire |
| [F-CLU-038](01-cluster-membership-and-ha.md#f-clu-038) | Rank incarnation | supporting | cluster-wire, client-wire |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-077](01-cluster-membership-and-ha.md#f-clu-077) | Rename-specific resolve handling | supporting | mds-wire, client-wire |
| [F-CLU-080](01-cluster-membership-and-ha.md#f-clu-080) | up:reconnect: the client reconnect window and its timeout | supporting | client-wire, config |
| [F-CLU-082](01-cluster-membership-and-ha.md#f-clu-082) | Recording caps and snaprealms from reconnect messages | supporting | client-wire |
| [F-CLU-084](01-cluster-membership-and-ha.md#f-clu-084) | Admission of replayed and retried requests during reconnect | supporting | client-wire |
| [F-CLU-087](01-cluster-membership-and-ha.md#f-clu-087) | Matching reconnected caps to inodes: OpenFileTable prefetch and open by number | supporting | on-disk, client-wire |
| [F-CLU-088](01-cluster-membership-and-ha.md#f-clu-088) | Moving reconnected caps between ranks (IMPORT/EXPORT during rejoin) | supporting | client-wire, mds-wire |
| [F-CLU-089](01-cluster-membership-and-ha.md#f-clu-089) | Forced session open during rejoin *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-092](01-cluster-membership-and-ha.md#f-clu-092) | Snaprealm reconciliation and snapflush rebuild at the end of rejoin | supporting | client-wire |
| [F-CLU-094](01-cluster-membership-and-ha.md#f-clu-094) | Rejoin on a single-rank file system | supporting | client-wire, on-disk |
| [F-CLU-095](01-cluster-membership-and-ha.md#f-clu-095) | up:clientreplay: ordered re-execution of replayed requests | supporting | client-wire |
| [F-CLU-096](01-cluster-membership-and-ha.md#f-clu-096) | Reclaim gating of the clientreplay to active transition | supporting | client-wire |
| [F-CLU-097](01-cluster-membership-and-ha.md#f-clu-097) | Activation (recovery_done and active_start) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-098](01-cluster-membership-and-ha.md#f-clu-098) | Leftover reconnect caps and cap reissue at activation | supporting | client-wire, metrics |
| [F-CLU-102](01-cluster-membership-and-ha.md#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk |
| [F-CLU-109](01-cluster-membership-and-ha.md#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config |
| [F-CLU-116](01-cluster-membership-and-ha.md#f-clu-116) | Outbound messaging helpers and local delivery | internal | client-wire, mds-wire |
| [F-PROTO-002](02-client-sessions-and-protocol.md#f-proto-002) | msgr2.1 crc mode for client connections *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-003](02-client-sessions-and-protocol.md#f-proto-003) | msgr2.1 secure mode (AES-128-GCM) *(legacy parts)* | operational | client-wire, config |
| [F-PROTO-004](02-client-sessions-and-protocol.md#f-proto-004) | msgr2 on-wire compression negotiation | internal | client-wire, config |
| [F-PROTO-005](02-client-sessions-and-protocol.md#f-proto-005) | Address vector publication and server identity checks *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-006](02-client-sessions-and-protocol.md#f-proto-006) | Lossless stateful-server connection policy and replay of unacknowledged messages *(legacy parts)* | supporting | client-wire |
| [F-PROTO-007](02-client-sessions-and-protocol.md#f-proto-007) | Peer-reset detection and session-reset signalling *(legacy parts)* | supporting | client-wire |
| [F-PROTO-008](02-client-sessions-and-protocol.md#f-proto-008) | MDS entity name and rank identity on the wire *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-009](02-client-sessions-and-protocol.md#f-proto-009) | Keepalives and connection timeouts *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-011](02-client-sessions-and-protocol.md#f-proto-011) | cephx authorizer verification for service mds *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-012](02-client-sessions-and-protocol.md#f-proto-012) | Rotating mds service secrets | supporting | cluster-wire, config |
| [F-PROTO-015](02-client-sessions-and-protocol.md#f-proto-015) | Connection-to-session binding (identity is the cephx global id) | supporting | client-wire |
| [F-PROTO-016](02-client-sessions-and-protocol.md#f-proto-016) | Encoding primitives and versioned-struct rules | supporting | client-wire |
| [F-PROTO-017](02-client-sessions-and-protocol.md#f-proto-017) | Client message catalog and peer-type filtering | supporting | client-wire, mds-wire |
| [F-PROTO-020](02-client-sessions-and-protocol.md#f-proto-020) | CephFS feature bitmap exchange at session open *(legacy parts)* | supporting | client-wire |
| [F-PROTO-021](02-client-sessions-and-protocol.md#f-proto-021) | Per-bit gated behaviour and feature names *(legacy parts)* | supporting | client-wire, admin |
| [F-PROTO-023](02-client-sessions-and-protocol.md#f-proto-023) | required_client_features enforcement and min_compat_client *(legacy parts)* | operational | client-wire, cluster-wire, admin |
| [F-PROTO-024](02-client-sessions-and-protocol.md#f-proto-024) | Connection-level feature bits consulted by the client protocol *(legacy parts)* | supporting | client-wire |
| [F-PROTO-026](02-client-sessions-and-protocol.md#f-proto-026) | Client generations and candidate minimum client floors *(legacy parts)* | operational | client-wire, admin |
| [F-PROTO-027](02-client-sessions-and-protocol.md#f-proto-027) | Session state machine | supporting | client-wire |
| [F-PROTO-028](02-client-sessions-and-protocol.md#f-proto-028) | Session open handshake (REQUEST_OPEN, journaled open, OPEN) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-PROTO-029](02-client-sessions-and-protocol.md#f-proto-029) | Session open admission checks and rejection reasons *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-PROTO-030](02-client-sessions-and-protocol.md#f-proto-030) | REJECT encoding and blocklist signalling to clients *(legacy parts)* | supporting | client-wire |
| [F-PROTO-031](02-client-sessions-and-protocol.md#f-proto-031) | MClientSession encoding and the ops the MDS sends *(legacy parts)* | supporting | client-wire |
| [F-PROTO-032](02-client-sessions-and-protocol.md#f-proto-032) | Session op dispatch and rank-state gating | supporting | client-wire |
| [F-PROTO-034](02-client-sessions-and-protocol.md#f-proto-034) | Client metadata capture and use *(legacy parts)* | operational | client-wire, admin, on-disk |
| [F-PROTO-037](02-client-sessions-and-protocol.md#f-proto-037) | Duplicate REQUEST_OPEN answered with OPEN (NOTIFY_SESSION_STATE) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-038](02-client-sessions-and-protocol.md#f-proto-038) | FORCE_RO: read-only MDS notification | supporting | client-wire |
| [F-PROTO-039](02-client-sessions-and-protocol.md#f-proto-039) | FLUSHMSG / FLUSHMSG_ACK session barrier *(legacy parts)* | supporting | client-wire |
| [F-PROTO-040](02-client-sessions-and-protocol.md#f-proto-040) | REQUEST_FLUSH_MDLOG and the client sync paths | supporting | client-wire |
| [F-PROTO-041](02-client-sessions-and-protocol.md#f-proto-041) | Force-opened sessions (imports, rejoin, cross-MDS rename) | supporting | client-wire, mds-wire, on-disk |
| [F-PROTO-042](02-client-sessions-and-protocol.md#f-proto-042) | Cap renewal (REQUEST_RENEWCAPS / RENEWCAPS) | supporting | client-wire |
| [F-PROTO-043](02-client-sessions-and-protocol.md#f-proto-043) | Stale session detection and the STALE message | supporting | client-wire |
| [F-PROTO-045](02-client-sessions-and-protocol.md#f-proto-045) | Session autoclose eviction | supporting | client-wire, cluster-wire, config |
| [F-PROTO-046](02-client-sessions-and-protocol.md#f-proto-046) | Per-session timeout metadata | operational | client-wire, admin |
| [F-PROTO-049](02-client-sessions-and-protocol.md#f-proto-049) | Client-initiated close with push-sequence handshake | supporting | client-wire, on-disk |
| [F-PROTO-050](02-client-sessions-and-protocol.md#f-proto-050) | Counted messages and the session push sequence | supporting | client-wire |
| [F-PROTO-051](02-client-sessions-and-protocol.md#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin |
| [F-PROTO-052](02-client-sessions-and-protocol.md#f-proto-052) | OSDMap blocklist-driven session kill | supporting | client-wire, cluster-wire |
| [F-PROTO-053](02-client-sessions-and-protocol.md#f-proto-053) | Eviction of sessions with oversized metadata | operational | config, metrics |
| [F-PROTO-054](02-client-sessions-and-protocol.md#f-proto-054) | Session termination at rank shutdown | operational | client-wire, cluster-wire |
| [F-PROTO-055](02-client-sessions-and-protocol.md#f-proto-055) | Reconnect trigger and client-side ordering | supporting | client-wire |
| [F-PROTO-056](02-client-sessions-and-protocol.md#f-proto-056) | MClientReconnect encodings (v1 to v5, multi-message) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-057](02-client-sessions-and-protocol.md#f-proto-057) | Reconnect processing and outcomes *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-061](02-client-sessions-and-protocol.md#f-proto-061) | Snap realm reconnect | extension | client-wire |
| [F-PROTO-062](02-client-sessions-and-protocol.md#f-proto-062) | Client reactions to MDSMap rank changes | supporting | cluster-wire |
| [F-PROTO-063](02-client-sessions-and-protocol.md#f-proto-063) | Session reclaim (MClientReclaim with CEPH_RECLAIM_RESET) *(legacy parts)* | operational | client-wire |
| [F-PROTO-065](02-client-sessions-and-protocol.md#f-proto-065) | MClientRequest versioned head (head v1, v2, v3) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-067](02-client-sessions-and-protocol.md#f-proto-067) | Request message tail: stamp, gid_list, alternate_name, fscrypt fields *(legacy parts)* | supporting | client-wire |
| [F-PROTO-068](02-client-sessions-and-protocol.md#f-proto-068) | Request flags REPLAY, WANT_DENTRY and ASYNC | supporting | client-wire |
| [F-PROTO-069](02-client-sessions-and-protocol.md#f-proto-069) | Request argument union per op | supporting | client-wire |
| [F-PROTO-070](02-client-sessions-and-protocol.md#f-proto-070) | Embedded cap and dentry-lease release records | supporting | client-wire |
| [F-PROTO-071](02-client-sessions-and-protocol.md#f-proto-071) | Request data segment per op | supporting | client-wire |
| [F-PROTO-072](02-client-sessions-and-protocol.md#f-proto-072) | Path encoding on the wire | supporting | client-wire |
| [F-PROTO-073](02-client-sessions-and-protocol.md#f-proto-073) | Op code numbering and the write bit | supporting | client-wire |
| [F-PROTO-074](02-client-sessions-and-protocol.md#f-proto-074) | Client-chosen inode number (head.ino) | supporting | client-wire |
| [F-PROTO-075](02-client-sessions-and-protocol.md#f-proto-075) | Client-side rank selection and the hints the MDS provides | supporting | client-wire, config |
| [F-PROTO-076](02-client-sessions-and-protocol.md#f-proto-076) | Request forwarding (MClientRequestForward) | supporting | client-wire |
| [F-PROTO-077](02-client-sessions-and-protocol.md#f-proto-077) | Retry counters and attempt limits (32BITS_RETRY_FWD) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-078](02-client-sessions-and-protocol.md#f-proto-078) | Request identity, tid ordering and requests without a session | supporting | client-wire |
| [F-PROTO-079](02-client-sessions-and-protocol.md#f-proto-079) | MClientReply head and error codes | supporting | client-wire |
| [F-PROTO-080](02-client-sessions-and-protocol.md#f-proto-080) | Reply trace layout | supporting | client-wire |
| [F-PROTO-081](02-client-sessions-and-protocol.md#f-proto-081) | InodeStat versioned encoding (v9, REPLY_ENCODING) | supporting | client-wire |
| [F-PROTO-083](02-client-sessions-and-protocol.md#f-proto-083) | DirStat and LeaseStat records *(legacy parts)* | supporting | client-wire |
| [F-PROTO-084](02-client-sessions-and-protocol.md#f-proto-084) | Caps, xattrs and inline data carried inside InodeStat *(legacy parts)* | supporting | client-wire |
| [F-PROTO-085](02-client-sessions-and-protocol.md#f-proto-085) | Snap trace encoding and format selection | extension | client-wire |
| [F-PROTO-086](02-client-sessions-and-protocol.md#f-proto-086) | Readdir and LSSNAP reply format and ordering contract *(legacy parts)* | supporting | client-wire |
| [F-PROTO-087](02-client-sessions-and-protocol.md#f-proto-087) | CREATE reply: created inode and delegated inode ranges *(legacy parts)* | supporting | client-wire |
| [F-PROTO-088](02-client-sessions-and-protocol.md#f-proto-088) | GETFILELOCK, GETVXATTR and FILE_BLOCKDIFF extra payloads | supporting | client-wire |
| [F-PROTO-089](02-client-sessions-and-protocol.md#f-proto-089) | Early (unsafe) and safe replies | supporting | client-wire |
| [F-PROTO-090](02-client-sessions-and-protocol.md#f-proto-090) | Reply content invariants enforced by clients | supporting | client-wire |
| [F-PROTO-091](02-client-sessions-and-protocol.md#f-proto-091) | MClientCaps encoding (v14, xattrs in the middle segment) *(legacy parts)* | supporting | client-wire |
| [F-PROTO-092](02-client-sessions-and-protocol.md#f-proto-092) | MClientCapRelease encoding and the OSD epoch barrier | supporting | client-wire |
| [F-PROTO-093](02-client-sessions-and-protocol.md#f-proto-093) | MClientLease encoding | supporting | client-wire |
| [F-PROTO-094](02-client-sessions-and-protocol.md#f-proto-094) | MClientSnap encoding | extension | client-wire |
| [F-PROTO-095](02-client-sessions-and-protocol.md#f-proto-095) | MClientQuota encoding *(legacy parts)* | extension | client-wire |
| [F-PROTO-096](02-client-sessions-and-protocol.md#f-proto-096) | MClientMetrics encoding and client send rules | operational | client-wire, metrics |
| [F-PROTO-097](02-client-sessions-and-protocol.md#f-proto-097) | MQuarantineDisable encoding | extension | client-wire |
| [F-PROTO-098](02-client-sessions-and-protocol.md#f-proto-098) | Mount flow requirements | supporting | client-wire, cluster-wire |
| [F-PROTO-099](02-client-sessions-and-protocol.md#f-proto-099) | Inode-number addressed access (NFS file handles, #ino paths, LOOKUPINO) | supporting | client-wire |
| [F-PROTO-101](02-client-sessions-and-protocol.md#f-proto-101) | Shared sessions: NFS-Ganesha and Samba libcephfsd | supporting | client-wire |
| [F-PROTO-102](02-client-sessions-and-protocol.md#f-proto-102) | NFS-Ganesha delegations built on caps | extension | client-wire |
| [F-CAPS-001](03-caps-leases-and-locking.md#f-caps-001) | Cap bit model and composite masks | supporting | client-wire |
| [F-CAPS-002](03-caps-leases-and-locking.md#f-caps-002) | Open mode to wanted caps | supporting | client-wire |
| [F-CAPS-003](03-caps-leases-and-locking.md#f-caps-003) | Capability record, cap ids and sequence numbers | supporting | client-wire |
| [F-CAPS-004](03-caps-leases-and-locking.md#f-caps-004) | Lock-state-derived allowed caps *(legacy parts)* | supporting | client-wire |
| [F-CAPS-009](03-caps-leases-and-locking.md#f-caps-009) | Caps in request replies *(legacy parts)* | supporting | client-wire |
| [F-CAPS-010](03-caps-leases-and-locking.md#f-caps-010) | New caps on open, create, mknod and mkdir | supporting | client-wire |
| [F-CAPS-011](03-caps-leases-and-locking.md#f-caps-011) | Cap grant and revoke messages (GRANT/REVOKE) | supporting | client-wire |
| [F-CAPS-013](03-caps-leases-and-locking.md#f-caps-013) | Client cap message processing (handle_client_caps) | supporting | client-wire |
| [F-CAPS-014](03-caps-leases-and-locking.md#f-caps-014) | Wanted caps tracking (issue_seq rule) | supporting | client-wire |
| [F-CAPS-015](03-caps-leases-and-locking.md#f-caps-015) | Xattr blob and inline data carried on caps *(legacy parts)* | supporting | client-wire |
| [F-CAPS-016](03-caps-leases-and-locking.md#f-caps-016) | Truncate notification (CEPH_CAP_OP_TRUNC) | supporting | client-wire |
| [F-CAPS-018](03-caps-leases-and-locking.md#f-caps-018) | Dirty metadata flush and FLUSH_ACK *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CAPS-019](03-caps-leases-and-locking.md#f-caps-019) | Flush idempotency (completed_flushes, oldest_flush_tid) | supporting | client-wire, on-disk, metrics |
| [F-CAPS-020](03-caps-leases-and-locking.md#f-caps-020) | Synchronous flush and journal-flush heuristics | supporting | client-wire, config |
| [F-CAPS-021](03-caps-leases-and-locking.md#f-caps-021) | max_size and per-client writeable ranges (client_ranges) | supporting | client-wire, on-disk, config |
| [F-CAPS-028](03-caps-leases-and-locking.md#f-caps-028) | Batched cap release (MClientCapRelease) | supporting | client-wire |
| [F-CAPS-029](03-caps-leases-and-locking.md#f-caps-029) | Cap and dentry-lease releases embedded in requests | supporting | client-wire |
| [F-CAPS-030](03-caps-leases-and-locking.md#f-caps-030) | Stale-session cap handling *(legacy parts)* | supporting | client-wire, config |
| [F-CAPS-032](03-caps-leases-and-locking.md#f-caps-032) | Cache-pressure cap recall (CEPH_SESSION_RECALL_STATE) | operational | client-wire, metrics, config, admin |
| [F-CAPS-034](03-caps-leases-and-locking.md#f-caps-034) | Cap re-establishment after MDS failover (cap side) | supporting | client-wire |
| [F-CAPS-036](03-caps-leases-and-locking.md#f-caps-036) | Cap migration to clients (EXPORT / IMPORT) | supporting | client-wire, mds-wire |
| [F-CAPS-038](03-caps-leases-and-locking.md#f-caps-038) | Dentry lease issue | supporting | client-wire |
| [F-CAPS-039](03-caps-leases-and-locking.md#f-caps-039) | Negative dentry leases | supporting | client-wire |
| [F-CAPS-043](03-caps-leases-and-locking.md#f-caps-043) | Lease revocation on dentry change | supporting | client-wire |
| [F-CAPS-044](03-caps-leases-and-locking.md#f-caps-044) | Lease expiry and cleanup | supporting | client-wire, config |
| [F-CAPS-046](03-caps-leases-and-locking.md#f-caps-046) | Directory-wide dentry validity via Fs | supporting | client-wire |
| [F-CAPS-059](03-caps-leases-and-locking.md#f-caps-059) | Replay of unsafe and async creates after failover | supporting | client-wire, config |
| [F-CAPS-067](03-caps-leases-and-locking.md#f-caps-067) | Gather engine (eval_gather) and cap revocation gather | supporting | internal |
| [F-CAPS-081](03-caps-leases-and-locking.md#f-caps-081) | Implicit quiescelock wrlock | extension | internal |
| [F-CAPS-092](03-caps-leases-and-locking.md#f-caps-092) | XLOCKDONE: read-your-own-unsafe-writes | supporting | client-wire, config |
| [F-NS-001](04-namespace-and-posix-operations.md#f-ns-001) | Client op set and dispatch switch | required | client-wire |
| [F-NS-002](04-namespace-and-posix-operations.md#f-ns-002) | Request intake gating by rank state and session | supporting | client-wire |
| [F-NS-003](04-namespace-and-posix-operations.md#f-ns-003) | Request identity and in-flight duplicate detection | supporting | client-wire |
| [F-NS-010](04-namespace-and-posix-operations.md#f-ns-010) | Journaled mutation commit and safe reply | supporting | client-wire, on-disk |
| [F-NS-011](04-namespace-and-posix-operations.md#f-ns-011) | Early (unsafe) replies | supporting | client-wire, config |
| [F-NS-012](04-namespace-and-posix-operations.md#f-ns-012) | Completed-request table and idempotent retries/replays | supporting | client-wire, on-disk, metrics |
| [F-NS-013](04-namespace-and-posix-operations.md#f-ns-013) | Replay of unsafe requests after an MDS restart | supporting | client-wire |
| [F-NS-014](04-namespace-and-posix-operations.md#f-ns-014) | Per-op reply trace contents | supporting | client-wire |
| [F-NS-016](04-namespace-and-posix-operations.md#f-ns-016) | On-demand journal flush for fsync, syncfs and sync writes | required | client-wire |
| [F-NS-017](04-namespace-and-posix-operations.md#f-ns-017) | Read-only and snapshot EROFS gates | required | client-wire |
| [F-NS-018](04-namespace-and-posix-operations.md#f-ns-018) | Full metadata pool gate | required | client-wire, cluster-wire |
| [F-NS-019](04-namespace-and-posix-operations.md#f-ns-019) | Create/remove name gates and their error order | required | client-wire |
| [F-NS-020](04-namespace-and-posix-operations.md#f-ns-020) | Directory entry limits (ENOSPC) | operational | client-wire, config |
| [F-NS-021](04-namespace-and-posix-operations.md#f-ns-021) | Permission checking split: clients enforce POSIX, the MDS enforces auth caps *(legacy parts)* | required | client-wire |
| [F-NS-022](04-namespace-and-posix-operations.md#f-ns-022) | ESTALE recovery of unknown inode numbers | supporting | client-wire, mds-wire, on-disk |
| [F-NS-023](04-namespace-and-posix-operations.md#f-ns-023) | GETATTR (stat by inode) | required | client-wire |
| [F-NS-024](04-namespace-and-posix-operations.md#f-ns-024) | LOOKUP of one name in a directory | required | client-wire |
| [F-NS-025](04-namespace-and-posix-operations.md#f-ns-025) | Negative lookup replies (null-dentry traces) | supporting | client-wire |
| [F-NS-026](04-namespace-and-posix-operations.md#f-ns-026) | LOOKUPINO and LOOKUPHASH (resolve an inode number) | supporting | client-wire, mds-wire, on-disk |
| [F-NS-027](04-namespace-and-posix-operations.md#f-ns-027) | LOOKUPPARENT (parent directory of an inode) | supporting | client-wire |
| [F-NS-029](04-namespace-and-posix-operations.md#f-ns-029) | Reads at a snapshot id (LOOKUPSNAP, snapshot LOOKUPINO, open and readdir in snapshots) | extension | client-wire |
| [F-NS-030](04-namespace-and-posix-operations.md#f-ns-030) | OPEN of an existing file or directory *(legacy parts)* | required | client-wire |
| [F-NS-031](04-namespace-and-posix-operations.md#f-ns-031) | O_TRUNC on open | required | client-wire, on-disk |
| [F-NS-032](04-namespace-and-posix-operations.md#f-ns-032) | READDIR paging of one dirfrag in hash order *(legacy parts)* | required | client-wire |
| [F-NS-033](04-namespace-and-posix-operations.md#f-ns-033) | Readdir across dirfrags: frag selection and adjustment *(legacy parts)* | required | client-wire |
| [F-NS-034](04-namespace-and-posix-operations.md#f-ns-034) | Readdir cap-acquisition throttle | operational | config, metrics |
| [F-NS-035](04-namespace-and-posix-operations.md#f-ns-035) | CREATE (open with O_CREAT, optionally O_EXCL) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-036](04-namespace-and-posix-operations.md#f-ns-036) | MKNOD (regular file, FIFO, device, socket) | required | client-wire, on-disk |
| [F-NS-037](04-namespace-and-posix-operations.md#f-ns-037) | MKDIR *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-038](04-namespace-and-posix-operations.md#f-ns-038) | SYMLINK (and readlink without an MDS op) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-039](04-namespace-and-posix-operations.md#f-ns-039) | New inode initialization *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-041](04-namespace-and-posix-operations.md#f-ns-041) | Client-specified inode number (head.ino) on create | supporting | client-wire |
| [F-NS-042](04-namespace-and-posix-operations.md#f-ns-042) | File layout selection and validation at create | extension | client-wire |
| [F-NS-043](04-namespace-and-posix-operations.md#f-ns-043) | Parent directory updates on link and unlink | required | client-wire, on-disk |
| [F-NS-044](04-namespace-and-posix-operations.md#f-ns-044) | LINK (hard link) *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-045](04-namespace-and-posix-operations.md#f-ns-045) | UNLINK of a primary link | required | client-wire, on-disk |
| [F-NS-046](04-namespace-and-posix-operations.md#f-ns-046) | UNLINK of a remote (hard-link) dentry | required | client-wire, on-disk |
| [F-NS-047](04-namespace-and-posix-operations.md#f-ns-047) | RMDIR with emptiness checks | required | client-wire, on-disk |
| [F-NS-048](04-namespace-and-posix-operations.md#f-ns-048) | Unlinked-but-open inodes stay usable until the last cap is released | required | on-disk |
| [F-NS-049](04-namespace-and-posix-operations.md#f-ns-049) | RENAME of a file or directory (move, same-inode no-op) | required | client-wire, on-disk |
| [F-NS-050](04-namespace-and-posix-operations.md#f-ns-050) | RENAME over an existing target | required | client-wire, on-disk |
| [F-NS-051](04-namespace-and-posix-operations.md#f-ns-051) | RENAME validation and error order *(legacy parts)* | required | client-wire |
| [F-NS-053](04-namespace-and-posix-operations.md#f-ns-053) | Which rank serves each op (auth requirement, forward or discover) | supporting | client-wire, mds-wire, config |
| [F-NS-057](04-namespace-and-posix-operations.md#f-ns-057) | Inode authority and cap migration inside a cross-rank rename *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-NS-058](04-namespace-and-posix-operations.md#f-ns-058) | SETATTR request: mask, locks, error order and common tail *(legacy parts)* | required | client-wire, on-disk |
| [F-NS-059](04-namespace-and-posix-operations.md#f-ns-059) | chmod and chown, with set-user/group-ID clearing | required | client-wire |
| [F-NS-060](04-namespace-and-posix-operations.md#f-ns-060) | Explicit times (utimes) and btime, with time_warp_seq *(legacy parts)* | required | client-wire |
| [F-NS-061](04-namespace-and-posix-operations.md#f-ns-061) | Size growth through SETATTR | required | client-wire |
| [F-NS-062](04-namespace-and-posix-operations.md#f-ns-062) | Truncate (shrink) with asynchronous OSD truncation | required | client-wire, cluster-wire, on-disk |
| [F-NS-063](04-namespace-and-posix-operations.md#f-ns-063) | Timestamps from the client clock and change_attr | required | client-wire, on-disk |
| [F-NS-066](04-namespace-and-posix-operations.md#f-ns-066) | SETXATTR and RMXATTR of stored xattrs | supporting | client-wire, on-disk |
| [F-NS-067](04-namespace-and-posix-operations.md#f-ns-067) | Total xattr size limit (max_xattr_size) | supporting | client-wire, cluster-wire, admin |
| [F-NS-068](04-namespace-and-posix-operations.md#f-ns-068) | xattr blob delivery to clients | supporting | client-wire |
| [F-NS-069](04-namespace-and-posix-operations.md#f-ns-069) | POSIX ACLs stored as opaque xattrs | supporting | client-wire, on-disk |
| [F-NS-071](04-namespace-and-posix-operations.md#f-ns-071) | Virtual xattr set/remove path (common rules, p flag) | extension | client-wire, admin |
| [F-NS-072](04-namespace-and-posix-operations.md#f-ns-072) | Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*) *(legacy parts)* | extension | client-wire, cluster-wire |
| [F-NS-073](04-namespace-and-posix-operations.md#f-ns-073) | GETVXATTR op | extension | client-wire |
| [F-NS-074](04-namespace-and-posix-operations.md#f-ns-074) | Client-synthesized ceph.* vxattrs (InodeStat contract) | extension | client-wire |
| [F-NS-075](04-namespace-and-posix-operations.md#f-ns-075) | Case-insensitive and normalized directories (charmap) *(legacy parts)* | extension | client-wire, on-disk |
| [F-NS-077](04-namespace-and-posix-operations.md#f-ns-077) | SETFILELOCK: POSIX fcntl byte-range locks | required | client-wire |
| [F-NS-079](04-namespace-and-posix-operations.md#f-ns-079) | Blocking lock waits and wake-ups | required | client-wire |
| [F-NS-082](04-namespace-and-posix-operations.md#f-ns-082) | GETFILELOCK (F_GETLK) | required | client-wire |
| [F-NS-083](04-namespace-and-posix-operations.md#f-ns-083) | Lock cleanup on cap removal and session end | required | client-wire |
| [F-NS-084](04-namespace-and-posix-operations.md#f-ns-084) | Lock reinstatement after an MDS restart *(legacy parts)* | required | client-wire |
| [F-NS-085](04-namespace-and-posix-operations.md#f-ns-085) | Directory link count and size presentation | required | client-wire |
| [F-NS-086](04-namespace-and-posix-operations.md#f-ns-086) | Access time is not maintained by the MDS | required | client-wire |
| [F-NS-087](04-namespace-and-posix-operations.md#f-ns-087) | Timestamp range limited to unsigned 32-bit seconds | required | client-wire, on-disk |
| [F-NS-088](04-namespace-and-posix-operations.md#f-ns-088) | Limits enforced only by clients, and no hard-link limit | required | client-wire |
| [F-NS-090](04-namespace-and-posix-operations.md#f-ns-090) | Data-path coherency departures outside the MDS | required | client-wire |
| [F-NS-091](04-namespace-and-posix-operations.md#f-ns-091) | statfs is answered by the monitors | required | cluster-wire |
| [F-CACHE-001](05-metadata-cache-and-dirfrags.md#f-cache-001) | Inode record field set (inode_t) | required | on-disk, client-wire |
| [F-CACHE-003](05-metadata-cache-and-dirfrags.md#f-cache-003) | Refusal of client operations on private and system inodes | supporting | client-wire |
| [F-CACHE-004](05-metadata-cache-and-dirfrags.md#f-cache-004) | Timestamps, time_warp_seq and change_attr *(legacy parts)* | required | client-wire, on-disk |
| [F-CACHE-006](05-metadata-cache-and-dirfrags.md#f-cache-006) | Truncation state fields | required | client-wire, on-disk |
| [F-CACHE-007](05-metadata-cache-and-dirfrags.md#f-cache-007) | File layout field and layout inheritance *(legacy parts)* | extension | client-wire, on-disk |
| [F-CACHE-010](05-metadata-cache-and-dirfrags.md#f-cache-010) | Inode version and client-visible version | supporting | client-wire |
| [F-CACHE-014](05-metadata-cache-and-dirfrags.md#f-cache-014) | Projected dentry linkage (primary, remote, null) | supporting | on-disk |
| [F-CACHE-023](05-metadata-cache-and-dirfrags.md#f-cache-023) | Directory completeness and authoritative negative lookups | supporting | internal |
| [F-CACHE-041](05-metadata-cache-and-dirfrags.md#f-cache-041) | Path traversal (path_traverse) | required | client-wire |
| [F-CACHE-044](05-metadata-cache-and-dirfrags.md#f-cache-044) | Forward-or-discover for non-auth objects | supporting | client-wire, config |
| [F-CACHE-045](05-metadata-cache-and-dirfrags.md#f-cache-045) | Hard-link target resolution | required | client-wire |
| [F-CACHE-047](05-metadata-cache-and-dirfrags.md#f-cache-047) | Lookup by inode number (open_ino) | supporting | client-wire, on-disk, mds-wire |
| [F-CACHE-050](05-metadata-cache-and-dirfrags.md#f-cache-050) | ESTALE recovery via find_ino_peers | supporting | client-wire, mds-wire |
| [F-CACHE-060](05-metadata-cache-and-dirfrags.md#f-cache-060) | Pin reasons that block eviction | operational | internal |
| [F-CACHE-071](05-metadata-cache-and-dirfrags.md#f-cache-071) | Root and system inode initial attributes and default layouts | required | client-wire, on-disk, config |
| [F-CACHE-072](05-metadata-cache-and-dirfrags.md#f-cache-072) | Global snaprealm dummy inode | supporting | client-wire |
| [F-CACHE-074](05-metadata-cache-and-dirfrags.md#f-cache-074) | Fragment identifier and fragment tree (frag_t, fragtree_t) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CACHE-075](05-metadata-cache-and-dirfrags.md#f-cache-075) | Dentry name hash per directory (dl_dir_hash) *(legacy parts)* | supporting | client-wire, on-disk, config |
| [F-CACHE-076](05-metadata-cache-and-dirfrags.md#f-cache-076) | Fragtree and dir_layout in InodeStat *(legacy parts)* | supporting | client-wire |
| [F-CACHE-077](05-metadata-cache-and-dirfrags.md#f-cache-077) | Hash-ordered dentry storage and readdir order *(legacy parts)* | required | client-wire |
| [F-CACHE-078](05-metadata-cache-and-dirfrags.md#f-cache-078) | Readdir fragment mapping and resume across refragmentation | required | client-wire |
| [F-CACHE-079](05-metadata-cache-and-dirfrags.md#f-cache-079) | DirStat delegation hints *(legacy parts)* | supporting | client-wire, config |
| [F-CACHE-080](05-metadata-cache-and-dirfrags.md#f-cache-080) | Per-dirfrag size limit (ENOSPC) | operational | client-wire, config |
| [F-CACHE-081](05-metadata-cache-and-dirfrags.md#f-cache-081) | Per-directory entry limit | operational | client-wire, config |
| [F-CACHE-097](05-metadata-cache-and-dirfrags.md#f-cache-097) | Effects of a fragment freeze on clients | supporting | client-wire |
| [F-STORE-007](06-journal-and-storage.md#f-store-007) | Per-rank write-ahead metadata journal with one submission order | supporting | on-disk |
| [F-STORE-008](06-journal-and-storage.md#f-store-008) | Group commit: flush and wait_for_safe | supporting | internal |
| [F-STORE-009](06-journal-and-storage.md#f-store-009) | Periodic journal flush on tick | supporting | config |
| [F-STORE-012](06-journal-and-storage.md#f-store-012) | Safe reply only after the journal commit (early unsafe reply) | supporting | client-wire |
| [F-STORE-026](06-journal-and-storage.md#f-store-026) | EUpdate: generic mutation record | supporting | on-disk |
| [F-STORE-028](06-journal-and-storage.md#f-store-028) | Session events: ESession and ESessions *(legacy parts)* | supporting | on-disk |
| [F-STORE-029](06-journal-and-storage.md#f-store-029) | Completed client request ids in journal and SessionMap | supporting | on-disk |
| [F-STORE-046](06-journal-and-storage.md#f-store-046) | Special inode numbers and per-rank ranges *(legacy parts)* | supporting | client-wire, on-disk |
| [F-STORE-058](06-journal-and-storage.md#f-store-058) | Backtrace parent xattr on object 0 | supporting | on-disk |
| [F-STORE-062](06-journal-and-storage.md#f-store-062) | Lookup by inode number through backtraces (open_ino storage path) | supporting | client-wire, on-disk |
| [F-STORE-064](06-journal-and-storage.md#f-store-064) | Data object naming and striping shared with clients | supporting | client-wire, on-disk |
| [F-STORE-070](06-journal-and-storage.md#f-store-070) | Inode number allocation in create paths | supporting | on-disk, client-wire |
| [F-STORE-073](06-journal-and-storage.md#f-store-073) | Inode numbers are never reused after deletion | required | client-wire, on-disk |
| [F-STORE-075](06-journal-and-storage.md#f-store-075) | SessionMap object | supporting | on-disk, config |
| [F-STORE-098](06-journal-and-storage.md#f-store-098) | OSD epoch barrier in cap messages | supporting | client-wire, admin |
| [F-STORE-099](06-journal-and-storage.md#f-store-099) | Metadata-pool-full ENOSPC policy | required | client-wire |
| [F-STORE-100](06-journal-and-storage.md#f-store-100) | Metadata write-error policy and read-only mode | operational | client-wire, config, metrics |
| [F-MULTI-012](07-multi-mds.md#f-multi-012) | Cap EXPORT message to clients (CEPH_CAP_OP_EXPORT) | supporting | client-wire |
| [F-MULTI-013](07-multi-mds.md#f-multi-013) | Cap IMPORT message to clients and auth-cap switch (CEPH_CAP_OP_IMPORT) *(legacy parts)* | supporting | client-wire |
| [F-MULTI-014](07-multi-mds.md#f-multi-014) | Migrate sequence (mseq) and stale cap-message handling | supporting | client-wire |
| [F-MULTI-015](07-multi-mds.md#f-multi-015) | Writeable-range reconciliation on cap import | supporting | internal |
| [F-MULTI-016](07-multi-mds.md#f-multi-016) | Client session flush barrier before shipping (FLUSHMSG) *(legacy parts)* | supporting | client-wire |
| [F-MULTI-017](07-multi-mds.md#f-multi-017) | Forced client session open on the importer | supporting | client-wire, on-disk |
| [F-MULTI-018](07-multi-mds.md#f-multi-018) | Request blocking and forwarding around migration | supporting | client-wire |
| [F-MULTI-019](07-multi-mds.md#f-multi-019) | Export-target publication (MMDSLoadTargets, MDSMap export_targets) | supporting | cluster-wire, client-wire |
| [F-MULTI-020](07-multi-mds.md#f-multi-020) | Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps) | supporting | mds-wire, client-wire, on-disk |
| [F-SNAP-001](08-snapshots-quotas-subvolumes.md#f-snap-001) | Snapshot id and realm seq allocation from one monotonic counter | extension | client-wire, cluster-wire, on-disk |
| [F-SNAP-006](08-snapshots-quotas-subvolumes.md#f-snap-006) | Snap realms and effective snapshot sets *(legacy parts)* | extension | client-wire, on-disk |
| [F-SNAP-007](08-snapshots-quotas-subvolumes.md#f-snap-007) | Realm membership of caps and realm split on creation | extension | client-wire |
| [F-SNAP-008](08-snapshots-quotas-subvolumes.md#f-snap-008) | Snap trace contents and client interpretation | extension | client-wire |
| [F-SNAP-009](08-snapshots-quotas-subvolumes.md#f-snap-009) | Per-realm MClientSnap notification (always op SPLIT) | extension | client-wire |
| [F-SNAP-010](08-snapshots-quotas-subvolumes.md#f-snap-010) | Global realm broadcast on every snap-table operation | extension | client-wire |
| [F-SNAP-011](08-snapshots-quotas-subvolumes.md#f-snap-011) | Hard-link global realm (PARENT_GLOBAL) | extension | client-wire, on-disk |
| [F-SNAP-012](08-snapshots-quotas-subvolumes.md#f-snap-012) | Past-parent snapshots on rename and unlink | extension | client-wire, on-disk |
| [F-SNAP-013](08-snapshots-quotas-subvolumes.md#f-snap-013) | Create a snapshot (MKSNAP, mkdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-014](08-snapshots-quotas-subvolumes.md#f-snap-014) | Remove a snapshot (RMSNAP, rmdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-019](08-snapshots-quotas-subvolumes.md#f-snap-019) | Per-directory snapshot limit (mds_max_snaps_per_dir) | extension | config, client-wire |
| [F-SNAP-021](08-snapshots-quotas-subvolumes.md#f-snap-021) | Snapshot path encoding, traversal and LOOKUPSNAP | extension | client-wire |
| [F-SNAP-022](08-snapshots-quotas-subvolumes.md#f-snap-022) | List snapshots (LSSNAP, readdir of .snap) | extension | client-wire |
| [F-SNAP-023](08-snapshots-quotas-subvolumes.md#f-snap-023) | Read-only snapshot semantics and snapshot caps | extension | client-wire |
| [F-SNAP-024](08-snapshots-quotas-subvolumes.md#f-snap-024) | Snapshot attributes in InodeStat (snap_btime, snap_metadata) | extension | client-wire |
| [F-SNAP-026](08-snapshots-quotas-subvolumes.md#f-snap-026) | Metadata copy-on-write (first/last ranges, snapped dentries, old_inodes) | extension | on-disk |
| [F-SNAP-032](08-snapshots-quotas-subvolumes.md#f-snap-032) | Snaprealm reconciliation at client reconnect and rejoin | extension | client-wire |
| [F-SNAP-033](08-snapshots-quotas-subvolumes.md#f-snap-033) | Directory entry statistics (dirstat / fragstat) | required | client-wire, on-disk |
| [F-SNAP-034](08-snapshots-quotas-subvolumes.md#f-snap-034) | Recursive subtree statistics (rstat) | extension | client-wire, on-disk |
| [F-SNAP-039](08-snapshots-quotas-subvolumes.md#f-snap-039) | rstat reads routed to the auth rank (GETATTR CEPH_STAT_RSTAT) | extension | client-wire |
| [F-SNAP-042](08-snapshots-quotas-subvolumes.md#f-snap-042) | Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files) | extension | client-wire, on-disk |
| [F-SNAP-043](08-snapshots-quotas-subvolumes.md#f-snap-043) | Quota directories become snaprealm roots | extension | client-wire, on-disk |
| [F-SNAP-044](08-snapshots-quotas-subvolumes.md#f-snap-044) | Quota and usage in InodeStat *(legacy parts)* | extension | client-wire |
| [F-SNAP-045](08-snapshots-quotas-subvolumes.md#f-snap-045) | MClientQuota usage pushes with thresholds *(legacy parts)* | extension | client-wire |
| [F-SNAP-047](08-snapshots-quotas-subvolumes.md#f-snap-047) | Client-side quota enforcement (EDQUOT, cross-quota EXDEV) | extension | client-wire |
| [F-SNAP-048](08-snapshots-quotas-subvolumes.md#f-snap-048) | Quota-based statfs | extension | client-wire |
| [F-SNAP-052](08-snapshots-quotas-subvolumes.md#f-snap-052) | Cross-subvolume link and rename rejected with EXDEV | extension | client-wire |
| [F-SNAP-053](08-snapshots-quotas-subvolumes.md#f-snap-053) | Snapshots only at the subvolume root | extension | client-wire |
| [F-BG-002](09-background-services.md#f-bg-002) | Deferred deletion: unlink, rmdir and rename-over move the inode to a stray | required | client-wire, on-disk |
| [F-BG-004](09-background-services.md#f-bg-004) | ENOSPC on unlink when the current stray dirfrag is full | operational | client-wire, config |
| [F-BG-006](09-background-services.md#f-bg-006) | Purge eligibility decision (eval_stray) | supporting | internal |
| [F-BG-008](09-background-services.md#f-bg-008) | Prompt cap release for unlinked inodes (nlink 0 grant with Ls) | supporting | client-wire |
| [F-BG-025](09-background-services.md#f-bg-025) | Purging inodes are invisible to clients (ENOENT / ESTALE) | supporting | client-wire |
| [F-BG-026](09-background-services.md#f-bg-026) | Namespace rules for stray and unlinked directories | required | client-wire |
| [F-BG-060](09-background-services.md#f-bg-060) | EIO for requests that touch known damage | required | client-wire |
| [F-BG-061](09-background-services.md#f-bg-061) | readdir behaviour on damaged directories | required | client-wire |
| [F-QOS-001](10-qos-metrics-quiesce-quarantine.md#f-qos-001) | QoS admission gate for client metadata requests | operational | config, admin |
| [F-QOS-002](10-qos-metrics-quiesce-quarantine.md#f-qos-002) | QoS bucket classification from the session mount root | operational | client-wire, admin |
| [F-QOS-004](10-qos-metrics-quiesce-quarantine.md#f-qos-004) | dmClock tag-based scheduling per bucket | operational | internal |
| [F-QOS-011](10-qos-metrics-quiesce-quarantine.md#f-qos-011) | Client metrics negotiation (feature bit 14 and metric_spec) *(legacy parts)* | operational | client-wire |
| [F-QOS-012](10-qos-metrics-quiesce-quarantine.md#f-qos-012) | MClientMetrics ingestion of per-client metrics (types 0..9) *(legacy parts)* | operational | client-wire |
| [F-QOS-014](10-qos-metrics-quiesce-quarantine.md#f-qos-014) | Metrics accepted on any active rank (client send rules) | operational | client-wire |
| [F-QOS-015](10-qos-metrics-quiesce-quarantine.md#f-qos-015) | Subvolume id in InodeStat and cap flush acks | extension | client-wire |
| [F-QOS-016](10-qos-metrics-quiesce-quarantine.md#f-qos-016) | Subvolume I/O metrics (type 16) ingestion and path resolution | operational | client-wire, metrics |
| [F-QOS-044](10-qos-metrics-quiesce-quarantine.md#f-qos-044) | Per-inode quiesce request (quiesce_inode) | extension | client-wire |
| [F-QOS-045](10-qos-metrics-quiesce-quarantine.md#f-qos-045) | quiescelock and parking of mutations | extension | internal |
| [F-QOS-046](10-qos-metrics-quiesce-quarantine.md#f-qos-046) | Cap masking while quiesced | extension | client-wire |
| [F-QOS-064](10-qos-metrics-quiesce-quarantine.md#f-qos-064) | Quarantine cap-revocation walk | extension | client-wire |
| [F-QOS-067](10-qos-metrics-quiesce-quarantine.md#f-qos-067) | Quarantine re-enforcement after restart | extension | client-wire |
| [F-QOS-069](10-qos-metrics-quiesce-quarantine.md#f-qos-069) | MDS auth cap flags q and Q (quarantine access) | extension | cluster-wire, admin |
| [F-QOS-070](10-qos-metrics-quiesce-quarantine.md#f-qos-070) | Quarantine client feature bit 24 | extension | client-wire |
| [F-QOS-071](10-qos-metrics-quiesce-quarantine.md#f-qos-071) | EACCES for quarantine-aware sessions | extension | client-wire |
| [F-QOS-072](10-qos-metrics-quiesce-quarantine.md#f-qos-072) | Request parking for quarantine-unaware sessions | extension | client-wire |
| [F-QOS-074](10-qos-metrics-quiesce-quarantine.md#f-qos-074) | Cap restriction to PIN under quarantine | extension | client-wire |
| [F-QOS-075](10-qos-metrics-quiesce-quarantine.md#f-qos-075) | EQUARANTINED errno in MClientCaps | extension | client-wire |
| [F-QOS-077](10-qos-metrics-quiesce-quarantine.md#f-qos-077) | MQuarantineDisable notification | extension | client-wire |
| [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) | Peer authorizer verification and caps loading at connection accept *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-ADM-007](11-security-admin-and-observability.md#f-adm-007) | Grant evaluation: union of grants in a fixed step order | extension | client-wire, admin |
| [F-ADM-008](11-security-admin-and-observability.md#f-adm-008) | path= restriction and the inode path used for checks | extension | client-wire, admin |
| [F-ADM-009](11-security-admin-and-observability.md#f-adm-009) | fsname= restriction and file-system gating of session messages | extension | client-wire, admin |
| [F-ADM-010](11-security-admin-and-observability.md#f-adm-010) | uid=/gids= restriction with simplified mode-bit check | extension | client-wire, admin |
| [F-ADM-015](11-security-admin-and-observability.md#f-adm-015) | q/Q flags: access to quarantined subtrees | extension | client-wire, admin |
| [F-ADM-017](11-security-admin-and-observability.md#f-adm-017) | Mount root check at session open and auth_name check at reclaim *(legacy parts)* | supporting | client-wire, on-disk |
| [F-ADM-018](11-security-admin-and-observability.md#f-adm-018) | Per-request access-check matrix *(legacy parts)* | extension | client-wire |
| [F-ADM-021](11-security-admin-and-observability.md#f-adm-021) | cap_auths export in CEPH_SESSION_OPEN and client-side checks *(legacy parts)* | extension | client-wire |
| [F-ADM-025](11-security-admin-and-observability.md#f-adm-025) | mgr/nfs export user caps | operational | cluster-wire, admin |
| [F-ADM-112](11-security-admin-and-observability.md#f-adm-112) | mgr/nfs: CephFS exports | operational | client-wire, cluster-wire |

## smb

Samba vfs_ceph (via libcephfs). 2 features.

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-PROTO-002](02-client-sessions-and-protocol.md#f-proto-002) | msgr2.1 crc mode for client connections *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-101](02-client-sessions-and-protocol.md#f-proto-101) | Shared sessions: NFS-Ganesha and Samba libcephfsd | supporting | client-wire |

## cephfs-mirror

cephfs-mirror daemon (via libcephfs). 21 features.

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-001](01-cluster-membership-and-ha.md#f-clu-001) | FSMap: monitor-owned map of all file systems | operational | cluster-wire |
| [F-CLU-004](01-cluster-membership-and-ha.md#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire |
| [F-CLU-013](01-cluster-membership-and-ha.md#f-clu-013) | Mirroring metadata (MirrorInfo) in the FSMap | operational | cluster-wire, admin |
| [F-NS-070](04-namespace-and-posix-operations.md#f-ns-070) | ceph.mirror.* xattrs for snapshot mirroring | extension | client-wire, on-disk |
| [F-STORE-102](06-journal-and-storage.md#f-store-102) | Metadata pool shared with mgr-owned objects *(legacy parts)* | operational | on-disk |
| [F-SNAP-013](08-snapshots-quotas-subvolumes.md#f-snap-013) | Create a snapshot (MKSNAP, mkdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-014](08-snapshots-quotas-subvolumes.md#f-snap-014) | Remove a snapshot (RMSNAP, rmdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-016](08-snapshots-quotas-subvolumes.md#f-snap-016) | Snapshot metadata at creation (SnapPayload) | extension | client-wire, on-disk |
| [F-SNAP-017](08-snapshots-quotas-subvolumes.md#f-snap-017) | Modify snapshot metadata (SNAP_METADATA op) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-022](08-snapshots-quotas-subvolumes.md#f-snap-022) | List snapshots (LSSNAP, readdir of .snap) | extension | client-wire |
| [F-SNAP-024](08-snapshots-quotas-subvolumes.md#f-snap-024) | Snapshot attributes in InodeStat (snap_btime, snap_metadata) | extension | client-wire |
| [F-SNAP-026](08-snapshots-quotas-subvolumes.md#f-snap-026) | Metadata copy-on-write (first/last ranges, snapped dentries, old_inodes) | extension | on-disk |
| [F-SNAP-029](08-snapshots-quotas-subvolumes.md#f-snap-029) | Directory snapshot diff (READDIR_SNAPDIFF) | extension | client-wire |
| [F-SNAP-030](08-snapshots-quotas-subvolumes.md#f-snap-030) | File block diff between snapshots (FILE_BLOCKDIFF) | extension | client-wire, cluster-wire |
| [F-QOS-002](10-qos-metrics-quiesce-quarantine.md#f-qos-002) | QoS bucket classification from the session mount root | operational | client-wire, admin |
| [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) | Peer authorizer verification and caps loading at connection accept *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-ADM-013](11-security-admin-and-observability.md#f-adm-013) | s flag and snapshot uid range | extension | client-wire, config |
| [F-ADM-018](11-security-admin-and-observability.md#f-adm-018) | Per-request access-check matrix *(legacy parts)* | extension | client-wire |
| [F-ADM-104](11-security-admin-and-observability.md#f-adm-104) | mgr/mirroring: enable, peers, bootstrap and directory distribution | operational | client-wire, on-disk, cluster-wire |
| [F-ADM-105](11-security-admin-and-observability.md#f-adm-105) | mgr/mirroring: checkpoints through snapshot metadata | operational | client-wire |
| [F-ADM-117](11-security-admin-and-observability.md#f-adm-117) | cephfs-mirror daemon | operational | client-wire |

## mon

Ceph monitors (MDSMonitor, FSMap, auth). 150 features (2 legacy entries left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-001](01-cluster-membership-and-ha.md#f-clu-001) | FSMap: monitor-owned map of all file systems | operational | cluster-wire |
| [F-CLU-002](01-cluster-membership-and-ha.md#f-clu-002) | MDSMap: per-file-system map and its wire encoding *(legacy parts)* | supporting | cluster-wire, client-wire |
| [F-CLU-003](01-cluster-membership-and-ha.md#f-clu-003) | Epoch model: FSMap, MDSMap and standby epochs, null map | operational | cluster-wire, client-wire |
| [F-CLU-004](01-cluster-membership-and-ha.md#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire |
| [F-CLU-009](01-cluster-membership-and-ha.md#f-clu-009) | FSMap sanity invariants enforced by the monitor | operational | cluster-wire |
| [F-CLU-014](01-cluster-membership-and-ha.md#f-clu-014) | CompatSet encoding and the MDS incompat feature ids | operational | cluster-wire |
| [F-CLU-015](01-cluster-membership-and-ha.md#f-clu-015) | Standby eligibility by compat set | operational | cluster-wire |
| [F-CLU-016](01-cluster-membership-and-ha.md#f-clu-016) | Forced INLINE incompat bit and compat bootstrap *(legacy parts)* | operational | cluster-wire |
| [F-CLU-017](01-cluster-membership-and-ha.md#f-clu-017) | Compat set must not change after boot | operational | cluster-wire |
| [F-CLU-018](01-cluster-membership-and-ha.md#f-clu-018) | Upgradeable file systems and compat merge on promotion *(legacy parts)* | operational | cluster-wire |
| [F-CLU-021](01-cluster-membership-and-ha.md#f-clu-021) | MMDSBeacon message and the beacon sender thread *(legacy parts)* | operational | cluster-wire, config |
| [F-CLU-022](01-cluster-membership-and-ha.md#f-clu-022) | Monitor beacon admission rules | operational | cluster-wire |
| [F-CLU-023](01-cluster-membership-and-ha.md#f-clu-023) | Beacon acks and MDS-side lagginess (self-fencing deferral) | operational | cluster-wire, config |
| [F-CLU-024](01-cluster-membership-and-ha.md#f-clu-024) | Heartbeat-gated beacons (internal watchdog) | operational | cluster-wire, config, metrics |
| [F-CLU-025](01-cluster-membership-and-ha.md#f-clu-025) | Boot registration as a standby with daemon metadata | operational | cluster-wire, admin |
| [F-CLU-026](01-cluster-membership-and-ha.md#f-clu-026) | Unique daemon names | operational | cluster-wire, config |
| [F-CLU-027](01-cluster-membership-and-ha.md#f-clu-027) | DNE beacon on clean shutdown | operational | cluster-wire |
| [F-CLU-028](01-cluster-membership-and-ha.md#f-clu-028) | Health metrics carried in beacons | operational | cluster-wire, metrics |
| [F-CLU-029](01-cluster-membership-and-ha.md#f-clu-029) | Monitor-side laggy detection and its protections | operational | cluster-wire, config |
| [F-CLU-030](01-cluster-membership-and-ha.md#f-clu-030) | Standby selection scoring | operational | cluster-wire |
| [F-CLU-031](01-cluster-membership-and-ha.md#f-clu-031) | mds_join_fs file system affinity | operational | cluster-wire, config |
| [F-CLU-032](01-cluster-membership-and-ha.md#f-clu-032) | Per-file-system selection flags: refuse_standby_for_another_fs and host anti-affinity | operational | cluster-wire, admin |
| [F-CLU-033](01-cluster-membership-and-ha.md#f-clu-033) | Affinity-driven swaps of healthy daemons | operational | cluster-wire |
| [F-CLU-034](01-cluster-membership-and-ha.md#f-clu-034) | Failed-rank and laggy-holder replacement | operational | cluster-wire |
| [F-CLU-035](01-cluster-membership-and-ha.md#f-clu-035) | Promotion into a rank and the initial rank state | operational | cluster-wire |
| [F-CLU-036](01-cluster-membership-and-ha.md#f-clu-036) | Fencing failed rank holders with OSD blocklists | supporting | cluster-wire |
| [F-CLU-037](01-cluster-membership-and-ha.md#f-clu-037) | Wait for last_failure_osd_epoch before touching rank objects | supporting | cluster-wire |
| [F-CLU-038](01-cluster-membership-and-ha.md#f-clu-038) | Rank incarnation | supporting | cluster-wire, client-wire |
| [F-CLU-039](01-cluster-membership-and-ha.md#f-clu-039) | Standby-replay assignment | operational | cluster-wire, admin |
| [F-CLU-040](01-cluster-membership-and-ha.md#f-clu-040) | Growing the number of active ranks (max_mds) | operational | cluster-wire, admin |
| [F-CLU-041](01-cluster-membership-and-ha.md#f-clu-041) | Shrinking: monitor-driven up:stopping and FSMap::stop | operational | cluster-wire, admin |
| [F-CLU-042](01-cluster-membership-and-ha.md#f-clu-042) | fs new | operational | admin, cluster-wire |
| [F-CLU-049](01-cluster-membership-and-ha.md#f-clu-049) | Damaged ranks and mds repaired | operational | admin, cluster-wire, metrics |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-054](01-cluster-membership-and-ha.md#f-clu-054) | State transition validation on both sides | operational | cluster-wire |
| [F-CLU-055](01-cluster-membership-and-ha.md#f-clu-055) | Requesting the next state through beacons | operational | cluster-wire |
| [F-CLU-056](01-cluster-membership-and-ha.md#f-clu-056) | Rank sets and derived predicates | operational | cluster-wire |
| [F-CLU-057](01-cluster-membership-and-ha.md#f-clu-057) | Daemon-level map handling | operational | cluster-wire |
| [F-CLU-058](01-cluster-membership-and-ha.md#f-clu-058) | Rank-level map handling and peer change detection | operational | cluster-wire |
| [F-CLU-062](01-cluster-membership-and-ha.md#f-clu-062) | File-system health checks derived from the map | operational | metrics, admin |
| [F-CLU-064](01-cluster-membership-and-ha.md#f-clu-064) | Creating a new rank (up:creating) | supporting | on-disk, cluster-wire |
| [F-CLU-065](01-cluster-membership-and-ha.md#f-clu-065) | Restarting a stopped rank (up:starting) | supporting | on-disk, cluster-wire |
| [F-CLU-067](01-cluster-membership-and-ha.md#f-clu-067) | End of replay: journal reformat, writeable journal and the single-rank shortcut *(legacy parts)* | supporting | on-disk, cluster-wire |
| [F-CLU-068](01-cluster-membership-and-ha.md#f-clu-068) | Standby-replay follower loop | operational | cluster-wire, config |
| [F-CLU-069](01-cluster-membership-and-ha.md#f-clu-069) | Standby-replay takeover | operational | cluster-wire |
| [F-CLU-070](01-cluster-membership-and-ha.md#f-clu-070) | Recovery set and cluster-wide resolve/rejoin barriers | supporting | mds-wire, cluster-wire |
| [F-CLU-073](01-cluster-membership-and-ha.md#f-clu-073) | up:resolve entry and exit | supporting | mds-wire, cluster-wire, on-disk |
| [F-CLU-096](01-cluster-membership-and-ha.md#f-clu-096) | Reclaim gating of the clientreplay to active transition | supporting | client-wire |
| [F-CLU-102](01-cluster-membership-and-ha.md#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk |
| [F-CLU-104](01-cluster-membership-and-ha.md#f-clu-104) | Reporting a rank as DAMAGED | operational | cluster-wire, metrics |
| [F-CLU-105](01-cluster-membership-and-ha.md#f-clu-105) | Respawn: re-exec as a fresh standby | operational | cluster-wire |
| [F-CLU-106](01-cluster-membership-and-ha.md#f-clu-106) | Suicide and orderly shutdown | operational | cluster-wire, admin |
| [F-CLU-109](01-cluster-membership-and-ha.md#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config |
| [F-CLU-110](01-cluster-membership-and-ha.md#f-clu-110) | MonClient session, keys and subscriptions *(legacy parts)* | operational | cluster-wire, config |
| [F-CLU-114](01-cluster-membership-and-ha.md#f-clu-114) | Dispatcher chain, message priorities and ordering | internal | internal |
| [F-CLU-115](01-cluster-membership-and-ha.md#f-clu-115) | Rank message validation and routing *(legacy parts)* | internal | client-wire, mds-wire |
| [F-CLU-117](01-cluster-membership-and-ha.md#f-clu-117) | Tick and the shared timer | internal | config |
| [F-PROTO-005](02-client-sessions-and-protocol.md#f-proto-005) | Address vector publication and server identity checks *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-011](02-client-sessions-and-protocol.md#f-proto-011) | cephx authorizer verification for service mds *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-012](02-client-sessions-and-protocol.md#f-proto-012) | Rotating mds service secrets | supporting | cluster-wire, config |
| [F-PROTO-021](02-client-sessions-and-protocol.md#f-proto-021) | Per-bit gated behaviour and feature names *(legacy parts)* | supporting | client-wire, admin |
| [F-PROTO-023](02-client-sessions-and-protocol.md#f-proto-023) | required_client_features enforcement and min_compat_client *(legacy parts)* | operational | client-wire, cluster-wire, admin |
| [F-PROTO-029](02-client-sessions-and-protocol.md#f-proto-029) | Session open admission checks and rejection reasons *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-PROTO-045](02-client-sessions-and-protocol.md#f-proto-045) | Session autoclose eviction | supporting | client-wire, cluster-wire, config |
| [F-PROTO-047](02-client-sessions-and-protocol.md#f-proto-047) | Eviction deferral while OSDs are laggy | operational | config, metrics |
| [F-PROTO-048](02-client-sessions-and-protocol.md#f-proto-048) | Cap-revoke non-responder eviction | operational | config, metrics |
| [F-PROTO-051](02-client-sessions-and-protocol.md#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin |
| [F-PROTO-052](02-client-sessions-and-protocol.md#f-proto-052) | OSDMap blocklist-driven session kill | supporting | client-wire, cluster-wire |
| [F-PROTO-053](02-client-sessions-and-protocol.md#f-proto-053) | Eviction of sessions with oversized metadata | operational | config, metrics |
| [F-PROTO-054](02-client-sessions-and-protocol.md#f-proto-054) | Session termination at rank shutdown | operational | client-wire, cluster-wire |
| [F-PROTO-062](02-client-sessions-and-protocol.md#f-proto-062) | Client reactions to MDSMap rank changes | supporting | cluster-wire |
| [F-PROTO-103](02-client-sessions-and-protocol.md#f-proto-103) | MonClient obligations *(legacy parts)* | operational | cluster-wire, config |
| [F-PROTO-106](02-client-sessions-and-protocol.md#f-proto-106) | Fencing identity: one address vector for clients, monitors and OSDs *(legacy parts)* | supporting | cluster-wire |
| [F-CAPS-019](03-caps-leases-and-locking.md#f-caps-019) | Flush idempotency (completed_flushes, oldest_flush_tid) | supporting | client-wire, on-disk, metrics |
| [F-CAPS-031](03-caps-leases-and-locking.md#f-caps-031) | Late revocation warnings, health and eviction | operational | metrics, config |
| [F-CAPS-032](03-caps-leases-and-locking.md#f-caps-032) | Cache-pressure cap recall (CEPH_SESSION_RECALL_STATE) | operational | client-wire, metrics, config, admin |
| [F-NS-067](04-namespace-and-posix-operations.md#f-ns-067) | Total xattr size limit (max_xattr_size) | supporting | client-wire, cluster-wire, admin |
| [F-NS-091](04-namespace-and-posix-operations.md#f-ns-091) | statfs is answered by the monitors | required | cluster-wire |
| [F-CACHE-018](05-metadata-cache-and-dirfrags.md#f-cache-018) | Commit error handling and read-only mode | operational | metrics, config |
| [F-CACHE-061](05-metadata-cache-and-dirfrags.md#f-cache-061) | Memory-based soft cache limit | operational | config, metrics |
| [F-CACHE-070](05-metadata-cache-and-dirfrags.md#f-cache-070) | mkfs base hierarchy creation | operational | on-disk |
| [F-CACHE-073](05-metadata-cache-and-dirfrags.md#f-cache-073) | Boot-time opening of the base hierarchy and the open gate *(legacy parts)* | operational | on-disk, metrics |
| [F-STORE-015](06-journal-and-storage.md#f-store-015) | Journal write-error policy (damaged or respawn) | operational | cluster-wire, metrics |
| [F-STORE-021](06-journal-and-storage.md#f-store-021) | Journal capping when a rank stops | operational | on-disk |
| [F-STORE-037](06-journal-and-storage.md#f-store-037) | Journal replay pipeline | supporting | on-disk |
| [F-STORE-040](06-journal-and-storage.md#f-store-040) | Replay error policy and the damaged rank | operational | cluster-wire, metrics |
| [F-STORE-041](06-journal-and-storage.md#f-store-041) | Post-replay session and InoTable consistency check | internal | internal |
| [F-STORE-042](06-journal-and-storage.md#f-store-042) | Single-writer fencing before reading rank state | supporting | cluster-wire |
| [F-STORE-043](06-journal-and-storage.md#f-store-043) | Standby-replay journal tailing (journal side) *(legacy parts)* | operational | on-disk |
| [F-STORE-044](06-journal-and-storage.md#f-store-044) | Journal creation and append-only open per rank state *(legacy parts)* | operational | on-disk |
| [F-STORE-055](06-journal-and-storage.md#f-store-055) | Object creation for a new rank (mkfs) | operational | on-disk |
| [F-STORE-079](06-journal-and-storage.md#f-store-079) | Table server placement on rank 0 | extension | cluster-wire |
| [F-STORE-082](06-journal-and-storage.md#f-store-082) | Table 2PC recovery and message delivery rules | internal | mds-wire |
| [F-STORE-091](06-journal-and-storage.md#f-store-091) | RADOS identity and request ids | internal | cluster-wire |
| [F-STORE-093](06-journal-and-storage.md#f-store-093) | Continuous OSDMap following | operational | cluster-wire |
| [F-STORE-094](06-journal-and-storage.md#f-store-094) | wait_for_map and wait_for_latest_osdmap | internal | cluster-wire |
| [F-STORE-095](06-journal-and-storage.md#f-store-095) | I/O completion path and self-respawn when fenced | operational | internal |
| [F-STORE-100](06-journal-and-storage.md#f-store-100) | Metadata write-error policy and read-only mode | operational | client-wire, config, metrics |
| [F-STORE-101](06-journal-and-storage.md#f-store-101) | Pool requirements for CephFS pools | operational | cluster-wire |
| [F-MULTI-019](07-multi-mds.md#f-multi-019) | Export-target publication (MMDSLoadTargets, MDSMap export_targets) | supporting | cluster-wire, client-wire |
| [F-MULTI-030](07-multi-mds.md#f-multi-030) | balance_automate file-system flag | operational | cluster-wire, admin |
| [F-MULTI-035](07-multi-mds.md#f-multi-035) | bal_rank_mask (balancer target ranks) | operational | cluster-wire, admin |
| [F-MULTI-036](07-multi-mds.md#f-multi-036) | Mantle Lua balancer policy | operational | cluster-wire, admin, config |
| [F-MULTI-048](07-multi-mds.md#f-multi-048) | Ephemeral pin redistribution on max_mds and config changes | extension | config |
| [F-MULTI-049](07-multi-mds.md#f-multi-049) | Subtree hand-off by a stopping rank (max_mds decrease) | operational | mds-wire, admin |
| [F-SNAP-001](08-snapshots-quotas-subvolumes.md#f-snap-001) | Snapshot id and realm seq allocation from one monotonic counter | extension | client-wire, cluster-wire, on-disk |
| [F-SNAP-003](08-snapshots-quotas-subvolumes.md#f-snap-003) | Deleted snapshots reported to the monitor (MRemoveSnaps) *(legacy parts)* | extension | cluster-wire, on-disk |
| [F-SNAP-018](08-snapshots-quotas-subvolumes.md#f-snap-018) | allow_new_snaps filesystem flag | operational | cluster-wire, admin |
| [F-BG-022](09-background-services.md#f-bg-022) | Rank stop: export all strays to rank 0 | operational | mds-wire |
| [F-BG-023](09-background-services.md#f-bg-023) | Rank stop: drain the PurgeQueue | operational | metrics |
| [F-BG-052](09-background-services.md#f-bg-052) | DamageTable registry *(legacy parts)* | operational | config |
| [F-BG-053](09-background-services.md#f-bg-053) | Dirfrag load damage detection | supporting | metrics |
| [F-BG-057](09-background-services.md#f-bg-057) | Fatal damage takes the rank down:damaged | operational | cluster-wire |
| [F-BG-059](09-background-services.md#f-bg-059) | damage ls and damage rm commands *(legacy parts)* | operational | admin |
| [F-QOS-006](10-qos-metrics-quiesce-quarantine.md#f-qos-006) | Default QoS parameters (mds_dmclock_* options) | operational | config |
| [F-QOS-032](10-qos-metrics-quiesce-quarantine.md#f-qos-032) | Quiesce DB leader and membership from the MDSMap | operational | cluster-wire |
| [F-QOS-033](10-qos-metrics-quiesce-quarantine.md#f-qos-033) | MDS compat incompat feature 12 (quiesce subvolumes) | operational | cluster-wire |
| [F-QOS-069](10-qos-metrics-quiesce-quarantine.md#f-qos-069) | MDS auth cap flags q and Q (quarantine access) | extension | cluster-wire, admin |
| [F-QOS-070](10-qos-metrics-quiesce-quarantine.md#f-qos-070) | Quarantine client feature bit 24 | extension | client-wire |
| [F-ADM-001](11-security-admin-and-observability.md#f-adm-001) | MDS self-authentication, keyring and rotating service secrets | operational | cluster-wire, config |
| [F-ADM-004](11-security-admin-and-observability.md#f-adm-004) | Peer-type message filtering and rank address check | internal | client-wire, mds-wire, cluster-wire |
| [F-ADM-006](11-security-admin-and-observability.md#f-adm-006) | MDS cap string grammar, parser and canonical forms | operational | admin, cluster-wire |
| [F-ADM-009](11-security-admin-and-observability.md#f-adm-009) | fsname= restriction and file-system gating of session messages | extension | client-wire, admin |
| [F-ADM-011](11-security-admin-and-observability.md#f-adm-011) | root_squash grants | extension | client-wire, admin |
| [F-ADM-015](11-security-admin-and-observability.md#f-adm-015) | q/Q flags: access to quarantined subtrees | extension | client-wire, admin |
| [F-ADM-023](11-security-admin-and-observability.md#f-adm-023) | ceph fs authorize: cap generation and merge on the monitor | operational | admin, cluster-wire |
| [F-ADM-038](11-security-admin-and-observability.md#f-adm-038) | session evict / client evict / session kill and the session filter grammar | operational | admin, cluster-wire |
| [F-ADM-050](11-security-admin-and-observability.md#f-adm-050) | damage ls and damage rm | operational | admin |
| [F-ADM-060](11-security-admin-and-observability.md#f-adm-060) | MDS option catalogue and option metadata | operational | config |
| [F-ADM-061](11-security-admin-and-observability.md#f-adm-061) | Runtime config observer and per-subsystem fan-out | operational | config |
| [F-ADM-062](11-security-admin-and-observability.md#f-adm-062) | Config delivery from the monitor config DB and reporting to the mgr *(legacy parts)* | operational | config, cluster-wire |
| [F-ADM-073](11-security-admin-and-observability.md#f-adm-073) | Health metric pipeline (MDSHealth in beacons to monitor checks) | operational | cluster-wire, metrics |
| [F-ADM-074](11-security-admin-and-observability.md#f-adm-074) | MDS_TRIM: behind on trimming | operational | metrics |
| [F-ADM-075](11-security-admin-and-observability.md#f-adm-075) | Client-behaviour warnings: late cap release, cache-pressure recall, oldest tid | operational | metrics |
| [F-ADM-076](11-security-admin-and-observability.md#f-adm-076) | MDS_SLOW_REQUEST and slow-request detection | operational | metrics, config |
| [F-ADM-077](11-security-admin-and-observability.md#f-adm-077) | MDS_SLOW_METADATA_IO and slow RADOS IO detection | operational | metrics |
| [F-ADM-078](11-security-admin-and-observability.md#f-adm-078) | MDS_DAMAGE and MDS_READ_ONLY state warnings | operational | metrics |
| [F-ADM-079](11-security-admin-and-observability.md#f-adm-079) | MDS_CACHE_OVERSIZED | operational | metrics |
| [F-ADM-080](11-security-admin-and-observability.md#f-adm-080) | MDS_CLIENTS_LAGGY: evictions deferred because of laggy OSDs | operational | metrics, config |
| [F-ADM-081](11-security-admin-and-observability.md#f-adm-081) | MDS_ESTIMATED_REPLAY_TIME | operational | metrics, admin |
| [F-ADM-082](11-security-admin-and-observability.md#f-adm-082) | Monitor-derived file-system health checks *(legacy parts)* | operational | metrics |
| [F-ADM-083](11-security-admin-and-observability.md#f-adm-083) | Health-gated monitor commands | operational | metrics, admin |
| [F-ADM-084](11-security-admin-and-observability.md#f-adm-084) | Cluster log (clog) messages | operational | metrics, cluster-wire |
| [F-ADM-085](11-security-admin-and-observability.md#f-adm-085) | Scrub task status in the mgr service map | operational | metrics, cluster-wire |
| [F-ADM-088](11-security-admin-and-observability.md#f-adm-088) | mgr/volumes: volume lifecycle (fs volume create, rm, rename, ls, info) | operational | client-wire, cluster-wire |
| [F-ADM-112](11-security-admin-and-observability.md#f-adm-112) | mgr/nfs: CephFS exports | operational | client-wire, cluster-wire |
| [F-ADM-113](11-security-admin-and-observability.md#f-adm-113) | cephadm MDS deployment | operational | config, cluster-wire |
| [F-ADM-114](11-security-admin-and-observability.md#f-adm-114) | cephadm MDS upgrade choreography | operational | cluster-wire |
| [F-ADM-115](11-security-admin-and-observability.md#f-adm-115) | Rook operator: MDS pods, command line, keyring, config and probes | operational | config, cluster-wire |
| [F-ADM-116](11-security-admin-and-observability.md#f-adm-116) | Rook operator: upgrade order, ok-to-stop gate and health gating | operational | cluster-wire, metrics |

## mgr

ceph-mgr and its modules (volumes, nfs, status, dashboard, prometheus, stats, mds_autoscaler, snap_schedule, ...). 156 features (1 legacy entry left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-001](01-cluster-membership-and-ha.md#f-clu-001) | FSMap: monitor-owned map of all file systems | operational | cluster-wire |
| [F-CLU-002](01-cluster-membership-and-ha.md#f-clu-002) | MDSMap: per-file-system map and its wire encoding *(legacy parts)* | supporting | cluster-wire, client-wire |
| [F-CLU-004](01-cluster-membership-and-ha.md#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire |
| [F-CLU-008](01-cluster-membership-and-ha.md#f-clu-008) | FSMap filtering for file-system-restricted sessions | operational | cluster-wire, admin |
| [F-CLU-013](01-cluster-membership-and-ha.md#f-clu-013) | Mirroring metadata (MirrorInfo) in the FSMap | operational | cluster-wire, admin |
| [F-CLU-025](01-cluster-membership-and-ha.md#f-clu-025) | Boot registration as a standby with daemon metadata | operational | cluster-wire, admin |
| [F-CLU-028](01-cluster-membership-and-ha.md#f-clu-028) | Health metrics carried in beacons | operational | cluster-wire, metrics |
| [F-CLU-040](01-cluster-membership-and-ha.md#f-clu-040) | Growing the number of active ranks (max_mds) | operational | cluster-wire, admin |
| [F-CLU-041](01-cluster-membership-and-ha.md#f-clu-041) | Shrinking: monitor-driven up:stopping and FSMap::stop | operational | cluster-wire, admin |
| [F-CLU-042](01-cluster-membership-and-ha.md#f-clu-042) | fs new | operational | admin, cluster-wire |
| [F-CLU-043](01-cluster-membership-and-ha.md#f-clu-043) | fs rm | operational | admin, cluster-wire |
| [F-CLU-044](01-cluster-membership-and-ha.md#f-clu-044) | fs fail and the joinable flag *(legacy parts)* | operational | admin, cluster-wire |
| [F-CLU-047](01-cluster-membership-and-ha.md#f-clu-047) | fs rename and fs swap | operational | admin, cluster-wire |
| [F-CLU-048](01-cluster-membership-and-ha.md#f-clu-048) | mds fail | operational | admin, cluster-wire |
| [F-CLU-051](01-cluster-membership-and-ha.md#f-clu-051) | Read-only fs and mds commands and the fs dump layout *(legacy parts)* | operational | admin |
| [F-CLU-052](01-cluster-membership-and-ha.md#f-clu-052) | Orchestration helpers: mds ok-to-stop, last-seen, metadata, versions | operational | admin |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-062](01-cluster-membership-and-ha.md#f-clu-062) | File-system health checks derived from the map | operational | metrics, admin |
| [F-CLU-097](01-cluster-membership-and-ha.md#f-clu-097) | Activation (recovery_done and active_start) *(legacy parts)* | supporting | client-wire, on-disk |
| [F-CLU-102](01-cluster-membership-and-ha.md#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk |
| [F-CLU-109](01-cluster-membership-and-ha.md#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config |
| [F-CLU-110](01-cluster-membership-and-ha.md#f-clu-110) | MonClient session, keys and subscriptions *(legacy parts)* | operational | cluster-wire, config |
| [F-CLU-114](01-cluster-membership-and-ha.md#f-clu-114) | Dispatcher chain, message priorities and ordering | internal | internal |
| [F-PROTO-002](02-client-sessions-and-protocol.md#f-proto-002) | msgr2.1 crc mode for client connections *(legacy parts)* | supporting | client-wire, config |
| [F-PROTO-034](02-client-sessions-and-protocol.md#f-proto-034) | Client metadata capture and use *(legacy parts)* | operational | client-wire, admin, on-disk |
| [F-PROTO-051](02-client-sessions-and-protocol.md#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin |
| [F-PROTO-096](02-client-sessions-and-protocol.md#f-proto-096) | MClientMetrics encoding and client send rules | operational | client-wire, metrics |
| [F-PROTO-104](02-client-sessions-and-protocol.md#f-proto-104) | MgrClient obligations | operational | cluster-wire, metrics |
| [F-CAPS-031](03-caps-leases-and-locking.md#f-caps-031) | Late revocation warnings, health and eviction | operational | metrics, config |
| [F-CAPS-032](03-caps-leases-and-locking.md#f-caps-032) | Cache-pressure cap recall (CEPH_SESSION_RECALL_STATE) | operational | client-wire, metrics, config, admin |
| [F-CAPS-081](03-caps-leases-and-locking.md#f-caps-081) | Implicit quiescelock wrlock | extension | internal |
| [F-CAPS-095](03-caps-leases-and-locking.md#f-caps-095) | Cap and lease counters and dumps | operational | metrics, admin |
| [F-NS-049](04-namespace-and-posix-operations.md#f-ns-049) | RENAME of a file or directory (move, same-inode no-op) | required | client-wire, on-disk |
| [F-NS-050](04-namespace-and-posix-operations.md#f-ns-050) | RENAME over an existing target | required | client-wire, on-disk |
| [F-NS-070](04-namespace-and-posix-operations.md#f-ns-070) | ceph.mirror.* xattrs for snapshot mirroring | extension | client-wire, on-disk |
| [F-NS-071](04-namespace-and-posix-operations.md#f-ns-071) | Virtual xattr set/remove path (common rules, p flag) | extension | client-wire, admin |
| [F-NS-072](04-namespace-and-posix-operations.md#f-ns-072) | Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*) *(legacy parts)* | extension | client-wire, cluster-wire |
| [F-NS-073](04-namespace-and-posix-operations.md#f-ns-073) | GETVXATTR op | extension | client-wire |
| [F-NS-074](04-namespace-and-posix-operations.md#f-ns-074) | Client-synthesized ceph.* vxattrs (InodeStat contract) | extension | client-wire |
| [F-NS-075](04-namespace-and-posix-operations.md#f-ns-075) | Case-insensitive and normalized directories (charmap) *(legacy parts)* | extension | client-wire, on-disk |
| [F-CACHE-007](05-metadata-cache-and-dirfrags.md#f-cache-007) | File layout field and layout inheritance *(legacy parts)* | extension | client-wire, on-disk |
| [F-CACHE-061](05-metadata-cache-and-dirfrags.md#f-cache-061) | Memory-based soft cache limit | operational | config, metrics |
| [F-STORE-043](06-journal-and-storage.md#f-store-043) | Standby-replay journal tailing (journal side) *(legacy parts)* | operational | on-disk |
| [F-STORE-101](06-journal-and-storage.md#f-store-101) | Pool requirements for CephFS pools | operational | cluster-wire |
| [F-STORE-102](06-journal-and-storage.md#f-store-102) | Metadata pool shared with mgr-owned objects *(legacy parts)* | operational | on-disk |
| [F-STORE-105](06-journal-and-storage.md#f-store-105) | Journal and OpenFileTable perf counters | operational | metrics |
| [F-MULTI-025](07-multi-mds.md#f-multi-025) | Quiesce and quarantine guards on migration | internal | internal |
| [F-MULTI-038](07-multi-mds.md#f-multi-038) | Set and remove ceph.dir.pin | extension | client-wire, on-disk |
| [F-MULTI-040](07-multi-mds.md#f-multi-040) | Pin inheritance and precedence resolution | extension | internal |
| [F-MULTI-041](07-multi-mds.md#f-multi-041) | Pin enforcement and aux subtrees (handle_export_pins) | extension | internal |
| [F-MULTI-045](07-multi-mds.md#f-multi-045) | Distributed ephemeral pins (ceph.dir.pin.distributed) | extension | client-wire, on-disk |
| [F-MULTI-046](07-multi-mds.md#f-multi-046) | Random ephemeral pins (ceph.dir.pin.random) | extension | client-wire, on-disk |
| [F-SNAP-013](08-snapshots-quotas-subvolumes.md#f-snap-013) | Create a snapshot (MKSNAP, mkdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-014](08-snapshots-quotas-subvolumes.md#f-snap-014) | Remove a snapshot (RMSNAP, rmdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-019](08-snapshots-quotas-subvolumes.md#f-snap-019) | Per-directory snapshot limit (mds_max_snaps_per_dir) | extension | config, client-wire |
| [F-SNAP-022](08-snapshots-quotas-subvolumes.md#f-snap-022) | List snapshots (LSSNAP, readdir of .snap) | extension | client-wire |
| [F-SNAP-024](08-snapshots-quotas-subvolumes.md#f-snap-024) | Snapshot attributes in InodeStat (snap_btime, snap_metadata) | extension | client-wire |
| [F-SNAP-034](08-snapshots-quotas-subvolumes.md#f-snap-034) | Recursive subtree statistics (rstat) | extension | client-wire, on-disk |
| [F-SNAP-042](08-snapshots-quotas-subvolumes.md#f-snap-042) | Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files) | extension | client-wire, on-disk |
| [F-SNAP-047](08-snapshots-quotas-subvolumes.md#f-snap-047) | Client-side quota enforcement (EDQUOT, cross-quota EXDEV) | extension | client-wire |
| [F-SNAP-048](08-snapshots-quotas-subvolumes.md#f-snap-048) | Quota-based statfs | extension | client-wire |
| [F-SNAP-050](08-snapshots-quotas-subvolumes.md#f-snap-050) | Subvolume flag (ceph.dir.subvolume) | extension | client-wire, on-disk |
| [F-SNAP-051](08-snapshots-quotas-subvolumes.md#f-snap-051) | No nested subvolumes; outermost subvolume wins | extension | client-wire |
| [F-SNAP-052](08-snapshots-quotas-subvolumes.md#f-snap-052) | Cross-subvolume link and rename rejected with EXDEV | extension | client-wire |
| [F-SNAP-053](08-snapshots-quotas-subvolumes.md#f-snap-053) | Snapshots only at the subvolume root | extension | client-wire |
| [F-SNAP-055](08-snapshots-quotas-subvolumes.md#f-snap-055) | Snapshot visibility flag (ceph.dir.subvolume.snaps.visible) | extension | client-wire, on-disk |
| [F-BG-037](09-background-services.md#f-bg-037) | Scrub status, summary and mgr task status *(legacy parts)* | operational | admin, metrics, cluster-wire |
| [F-BG-038](09-background-services.md#f-bg-038) | Scrub counters and scrub purge_status *(legacy parts)* | operational | admin, metrics, config |
| [F-QOS-002](10-qos-metrics-quiesce-quarantine.md#f-qos-002) | QoS bucket classification from the session mount root | operational | client-wire, admin |
| [F-QOS-016](10-qos-metrics-quiesce-quarantine.md#f-qos-016) | Subvolume I/O metrics (type 16) ingestion and path resolution | operational | client-wire, metrics |
| [F-QOS-019](10-qos-metrics-quiesce-quarantine.md#f-qos-019) | Rank-0 ping sequence and delayed-rank detection | internal | mds-wire, config |
| [F-QOS-021](10-qos-metrics-quiesce-quarantine.md#f-qos-021) | Metrics state culling on mdsmap changes | internal | metrics |
| [F-QOS-023](10-qos-metrics-quiesce-quarantine.md#f-qos-023) | Subvolume quota_bytes and used_bytes reporting | operational | metrics |
| [F-QOS-025](10-qos-metrics-quiesce-quarantine.md#f-qos-025) | Subvolume sliding-window metrics (mds_subvolume_metrics) | operational | metrics, mds-wire, config |
| [F-QOS-027](10-qos-metrics-quiesce-quarantine.md#f-qos-027) | Labeled metric counter naming and export | operational | metrics, admin |
| [F-QOS-028](10-qos-metrics-quiesce-quarantine.md#f-qos-028) | MDS perf query configuration from ceph-mgr | operational | cluster-wire |
| [F-QOS-029](10-qos-metrics-quiesce-quarantine.md#f-qos-029) | MDS perf query key construction and counter pairs | operational | cluster-wire |
| [F-QOS-030](10-qos-metrics-quiesce-quarantine.md#f-qos-030) | MDS perf report to ceph-mgr (MMgrReport) | operational | cluster-wire |
| [F-QOS-032](10-qos-metrics-quiesce-quarantine.md#f-qos-032) | Quiesce DB leader and membership from the MDSMap | operational | cluster-wire |
| [F-QOS-034](10-qos-metrics-quiesce-quarantine.md#f-qos-034) | Quiesce sets and request operations | extension | admin |
| [F-QOS-035](10-qos-metrics-quiesce-quarantine.md#f-qos-035) | Quiesce set and member state machine | extension | admin |
| [F-QOS-036](10-qos-metrics-quiesce-quarantine.md#f-qos-036) | Quiesce timeout and expiration | extension | admin |
| [F-QOS-037](10-qos-metrics-quiesce-quarantine.md#f-qos-037) | Quiesce await semantics | extension | admin |
| [F-QOS-038](10-qos-metrics-quiesce-quarantine.md#f-qos-038) | Quiesce if_version optimistic concurrency | extension | admin |
| [F-QOS-039](10-qos-metrics-quiesce-quarantine.md#f-qos-039) | Quiesce root URI sanitization | extension | admin |
| [F-QOS-042](10-qos-metrics-quiesce-quarantine.md#f-qos-042) | Quiesce on degraded file systems and non-active ranks | operational | admin |
| [F-QOS-053](10-qos-metrics-quiesce-quarantine.md#f-qos-053) | quiesce db command | extension | admin |
| [F-QOS-058](10-qos-metrics-quiesce-quarantine.md#f-qos-058) | quarantine enable and disable commands | extension | admin |
| [F-QOS-071](10-qos-metrics-quiesce-quarantine.md#f-qos-071) | EACCES for quarantine-aware sessions | extension | client-wire |
| [F-ADM-001](11-security-admin-and-observability.md#f-adm-001) | MDS self-authentication, keyring and rotating service secrets | operational | cluster-wire, config |
| [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) | Peer authorizer verification and caps loading at connection accept *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-ADM-006](11-security-admin-and-observability.md#f-adm-006) | MDS cap string grammar, parser and canonical forms | operational | admin, cluster-wire |
| [F-ADM-007](11-security-admin-and-observability.md#f-adm-007) | Grant evaluation: union of grants in a fixed step order | extension | client-wire, admin |
| [F-ADM-008](11-security-admin-and-observability.md#f-adm-008) | path= restriction and the inode path used for checks | extension | client-wire, admin |
| [F-ADM-011](11-security-admin-and-observability.md#f-adm-011) | root_squash grants | extension | client-wire, admin |
| [F-ADM-012](11-security-admin-and-observability.md#f-adm-012) | p flag: permission to set ceph.* vxattrs and non-default layouts | extension | client-wire, admin |
| [F-ADM-013](11-security-admin-and-observability.md#f-adm-013) | s flag and snapshot uid range | extension | client-wire, config |
| [F-ADM-014](11-security-admin-and-observability.md#f-adm-014) | f flag: metadata-pool-full bypass | extension | client-wire, admin |
| [F-ADM-015](11-security-admin-and-observability.md#f-adm-015) | q/Q flags: access to quarantined subtrees | extension | client-wire, admin |
| [F-ADM-018](11-security-admin-and-observability.md#f-adm-018) | Per-request access-check matrix *(legacy parts)* | extension | client-wire |
| [F-ADM-020](11-security-admin-and-observability.md#f-adm-020) | fscrypt attribute changes require allow * | extension | client-wire |
| [F-ADM-023](11-security-admin-and-observability.md#f-adm-023) | ceph fs authorize: cap generation and merge on the monitor | operational | admin, cluster-wire |
| [F-ADM-024](11-security-admin-and-observability.md#f-adm-024) | mgr/volumes subvolume authorize, deauthorize and authorized_list | operational | client-wire, cluster-wire, admin |
| [F-ADM-025](11-security-admin-and-observability.md#f-adm-025) | mgr/nfs export user caps | operational | cluster-wire, admin |
| [F-ADM-027](11-security-admin-and-observability.md#f-adm-027) | Remote tell transport (MCommand / MCommandReply) *(legacy parts)* | operational | admin, client-wire |
| [F-ADM-028](11-security-admin-and-observability.md#f-adm-028) | Tell authorization: allow * required | operational | admin |
| [F-ADM-030](11-security-admin-and-observability.md#f-adm-030) | Argument validation, output formats, output-file and return-code conventions | operational | admin |
| [F-ADM-031](11-security-admin-and-observability.md#f-adm-031) | Client-side target resolution for tell (libcephfs mds_command) | operational | admin, cluster-wire |
| [F-ADM-034](11-security-admin-and-observability.md#f-adm-034) | heap and cpu_profiler commands | operational | admin |
| [F-ADM-037](11-security-admin-and-observability.md#f-adm-037) | session ls / client ls | operational | admin |
| [F-ADM-038](11-security-admin-and-observability.md#f-adm-038) | session evict / client evict / session kill and the session filter grammar | operational | admin, cluster-wire |
| [F-ADM-054](11-security-admin-and-observability.md#f-adm-054) | quiesce db command | extension | admin |
| [F-ADM-055](11-security-admin-and-observability.md#f-adm-055) | quarantine enable / disable command | extension | admin |
| [F-ADM-057](11-security-admin-and-observability.md#f-adm-057) | Generic process commands: perf, counter, config, log, injectargs *(legacy parts)* | operational | admin, metrics, config |
| [F-ADM-058](11-security-admin-and-observability.md#f-adm-058) | dump_mempools | operational | admin |
| [F-ADM-060](11-security-admin-and-observability.md#f-adm-060) | MDS option catalogue and option metadata | operational | config |
| [F-ADM-062](11-security-admin-and-observability.md#f-adm-062) | Config delivery from the monitor config DB and reporting to the mgr *(legacy parts)* | operational | config, cluster-wire |
| [F-ADM-064](11-security-admin-and-observability.md#f-adm-064) | MDS option names read or written by peers | operational | config |
| [F-ADM-066](11-security-admin-and-observability.md#f-adm-066) | Unlabeled MDS perf counter loggers | operational | metrics |
| [F-ADM-067](11-security-admin-and-observability.md#f-adm-067) | Counter reporting to the mgr and the names mgr modules read | operational | metrics, cluster-wire |
| [F-ADM-068](11-security-admin-and-observability.md#f-adm-068) | Labeled perf counter families (client, subvolume and rank metrics) | operational | metrics |
| [F-ADM-071](11-security-admin-and-observability.md#f-adm-071) | Memory gauges and gauge refresh cadence | operational | metrics |
| [F-ADM-073](11-security-admin-and-observability.md#f-adm-073) | Health metric pipeline (MDSHealth in beacons to monitor checks) | operational | cluster-wire, metrics |
| [F-ADM-074](11-security-admin-and-observability.md#f-adm-074) | MDS_TRIM: behind on trimming | operational | metrics |
| [F-ADM-078](11-security-admin-and-observability.md#f-adm-078) | MDS_DAMAGE and MDS_READ_ONLY state warnings | operational | metrics |
| [F-ADM-081](11-security-admin-and-observability.md#f-adm-081) | MDS_ESTIMATED_REPLAY_TIME | operational | metrics, admin |
| [F-ADM-082](11-security-admin-and-observability.md#f-adm-082) | Monitor-derived file-system health checks *(legacy parts)* | operational | metrics |
| [F-ADM-085](11-security-admin-and-observability.md#f-adm-085) | Scrub task status in the mgr service map | operational | metrics, cluster-wire |
| [F-ADM-087](11-security-admin-and-observability.md#f-adm-087) | mgr libcephfs connection pool and mount identity | operational | client-wire |
| [F-ADM-088](11-security-admin-and-observability.md#f-adm-088) | mgr/volumes: volume lifecycle (fs volume create, rm, rename, ls, info) | operational | client-wire, cluster-wire |
| [F-ADM-089](11-security-admin-and-observability.md#f-adm-089) | mgr/volumes: in-filesystem layout, .meta files, versions, user metadata, open side effects *(legacy parts)* | operational | client-wire |
| [F-ADM-090](11-security-admin-and-observability.md#f-adm-090) | mgr/volumes: subvolume groups | operational | client-wire |
| [F-ADM-091](11-security-admin-and-observability.md#f-adm-091) | mgr/volumes: subvolume create | operational | client-wire |
| [F-ADM-092](11-security-admin-and-observability.md#f-adm-092) | mgr/volumes: removal, trash, snapshot retention and asynchronous purge | operational | client-wire |
| [F-ADM-093](11-security-admin-and-observability.md#f-adm-093) | mgr/volumes: subvolume resize via quota | operational | client-wire |
| [F-ADM-094](11-security-admin-and-observability.md#f-adm-094) | mgr/volumes: subvolume and group pinning | operational | client-wire, admin |
| [F-ADM-095](11-security-admin-and-observability.md#f-adm-095) | mgr/volumes: charmap (case sensitivity, normalization, encoding) | operational | client-wire |
| [F-ADM-096](11-security-admin-and-observability.md#f-adm-096) | mgr/volumes and mgr/nfs: earmark and enctag xattrs | operational | client-wire |
| [F-ADM-097](11-security-admin-and-observability.md#f-adm-097) | mgr/volumes: subvolume snapshots | operational | client-wire |
| [F-ADM-098](11-security-admin-and-observability.md#f-adm-098) | mgr/volumes: snapshot visibility | operational | client-wire |
| [F-ADM-099](11-security-admin-and-observability.md#f-adm-099) | mgr/volumes: snapshot clone | operational | client-wire |
| [F-ADM-100](11-security-admin-and-observability.md#f-adm-100) | mgr/volumes: fs subvolume evict | operational | admin |
| [F-ADM-101](11-security-admin-and-observability.md#f-adm-101) | mgr/volumes: fs quiesce | operational | admin |
| [F-ADM-102](11-security-admin-and-observability.md#f-adm-102) | mgr/volumes: subvolume quarantine | operational | admin |
| [F-ADM-103](11-security-admin-and-observability.md#f-adm-103) | mgr/snap_schedule: scheduled snapshots and retention *(legacy parts)* | operational | client-wire, on-disk, config |
| [F-ADM-104](11-security-admin-and-observability.md#f-adm-104) | mgr/mirroring: enable, peers, bootstrap and directory distribution | operational | client-wire, on-disk, cluster-wire |
| [F-ADM-105](11-security-admin-and-observability.md#f-adm-105) | mgr/mirroring: checkpoints through snapshot metadata | operational | client-wire |
| [F-ADM-106](11-security-admin-and-observability.md#f-adm-106) | mgr/stats: fs perf stats (and cephfs-top) | operational | cluster-wire, admin |
| [F-ADM-107](11-security-admin-and-observability.md#f-adm-107) | mgr/status: fs status | operational | metrics |
| [F-ADM-108](11-security-admin-and-observability.md#f-adm-108) | mgr/dashboard: CephFS pages | operational | admin, client-wire, metrics |
| [F-ADM-109](11-security-admin-and-observability.md#f-adm-109) | mgr/prometheus: MDS and file-system metadata metrics | operational | metrics |
| [F-ADM-110](11-security-admin-and-observability.md#f-adm-110) | mgr/telemetry: MDS data collection | operational | metrics, admin |
| [F-ADM-111](11-security-admin-and-observability.md#f-adm-111) | mgr/mds_autoscaler | operational | cluster-wire |
| [F-ADM-112](11-security-admin-and-observability.md#f-adm-112) | mgr/nfs: CephFS exports | operational | client-wire, cluster-wire |
| [F-ADM-113](11-security-admin-and-observability.md#f-adm-113) | cephadm MDS deployment | operational | config, cluster-wire |
| [F-ADM-117](11-security-admin-and-observability.md#f-adm-117) | cephfs-mirror daemon | operational | client-wire |

## cephadm

cephadm orchestrator. 46 features.

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-014](01-cluster-membership-and-ha.md#f-clu-014) | CompatSet encoding and the MDS incompat feature ids | operational | cluster-wire |
| [F-CLU-015](01-cluster-membership-and-ha.md#f-clu-015) | Standby eligibility by compat set | operational | cluster-wire |
| [F-CLU-018](01-cluster-membership-and-ha.md#f-clu-018) | Upgradeable file systems and compat merge on promotion *(legacy parts)* | operational | cluster-wire |
| [F-CLU-026](01-cluster-membership-and-ha.md#f-clu-026) | Unique daemon names | operational | cluster-wire, config |
| [F-CLU-027](01-cluster-membership-and-ha.md#f-clu-027) | DNE beacon on clean shutdown | operational | cluster-wire |
| [F-CLU-030](01-cluster-membership-and-ha.md#f-clu-030) | Standby selection scoring | operational | cluster-wire |
| [F-CLU-031](01-cluster-membership-and-ha.md#f-clu-031) | mds_join_fs file system affinity | operational | cluster-wire, config |
| [F-CLU-033](01-cluster-membership-and-ha.md#f-clu-033) | Affinity-driven swaps of healthy daemons | operational | cluster-wire |
| [F-CLU-035](01-cluster-membership-and-ha.md#f-clu-035) | Promotion into a rank and the initial rank state | operational | cluster-wire |
| [F-CLU-039](01-cluster-membership-and-ha.md#f-clu-039) | Standby-replay assignment | operational | cluster-wire, admin |
| [F-CLU-040](01-cluster-membership-and-ha.md#f-clu-040) | Growing the number of active ranks (max_mds) | operational | cluster-wire, admin |
| [F-CLU-041](01-cluster-membership-and-ha.md#f-clu-041) | Shrinking: monitor-driven up:stopping and FSMap::stop | operational | cluster-wire, admin |
| [F-CLU-042](01-cluster-membership-and-ha.md#f-clu-042) | fs new | operational | admin, cluster-wire |
| [F-CLU-043](01-cluster-membership-and-ha.md#f-clu-043) | fs rm | operational | admin, cluster-wire |
| [F-CLU-044](01-cluster-membership-and-ha.md#f-clu-044) | fs fail and the joinable flag *(legacy parts)* | operational | admin, cluster-wire |
| [F-CLU-046](01-cluster-membership-and-ha.md#f-clu-046) | fs set down | operational | admin, cluster-wire |
| [F-CLU-048](01-cluster-membership-and-ha.md#f-clu-048) | mds fail | operational | admin, cluster-wire |
| [F-CLU-051](01-cluster-membership-and-ha.md#f-clu-051) | Read-only fs and mds commands and the fs dump layout *(legacy parts)* | operational | admin |
| [F-CLU-052](01-cluster-membership-and-ha.md#f-clu-052) | Orchestration helpers: mds ok-to-stop, last-seen, metadata, versions | operational | admin |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-062](01-cluster-membership-and-ha.md#f-clu-062) | File-system health checks derived from the map | operational | metrics, admin |
| [F-CLU-102](01-cluster-membership-and-ha.md#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk |
| [F-CLU-105](01-cluster-membership-and-ha.md#f-clu-105) | Respawn: re-exec as a fresh standby | operational | cluster-wire |
| [F-CLU-106](01-cluster-membership-and-ha.md#f-clu-106) | Suicide and orderly shutdown | operational | cluster-wire, admin |
| [F-CLU-108](01-cluster-membership-and-ha.md#f-clu-108) | Process bootstrap (ceph-mds -i <id>) *(legacy parts)* | operational | config, cluster-wire |
| [F-CLU-110](01-cluster-membership-and-ha.md#f-clu-110) | MonClient session, keys and subscriptions *(legacy parts)* | operational | cluster-wire, config |
| [F-CACHE-061](05-metadata-cache-and-dirfrags.md#f-cache-061) | Memory-based soft cache limit | operational | config, metrics |
| [F-STORE-090](06-journal-and-storage.md#f-store-090) | Per-rank Objecter RADOS client | internal | cluster-wire |
| [F-STORE-101](06-journal-and-storage.md#f-store-101) | Pool requirements for CephFS pools | operational | cluster-wire |
| [F-BG-022](09-background-services.md#f-bg-022) | Rank stop: export all strays to rank 0 | operational | mds-wire |
| [F-BG-023](09-background-services.md#f-bg-023) | Rank stop: drain the PurgeQueue | operational | metrics |
| [F-ADM-001](11-security-admin-and-observability.md#f-adm-001) | MDS self-authentication, keyring and rotating service secrets | operational | cluster-wire, config |
| [F-ADM-005](11-security-admin-and-observability.md#f-adm-005) | Daemon key rotation commands (rotate-key, wipe-rotating-secrets) | operational | admin |
| [F-ADM-026](11-security-admin-and-observability.md#f-adm-026) | Local admin socket transport *(legacy parts)* | operational | admin, config |
| [F-ADM-028](11-security-admin-and-observability.md#f-adm-028) | Tell authorization: allow * required | operational | admin |
| [F-ADM-060](11-security-admin-and-observability.md#f-adm-060) | MDS option catalogue and option metadata | operational | config |
| [F-ADM-062](11-security-admin-and-observability.md#f-adm-062) | Config delivery from the monitor config DB and reporting to the mgr *(legacy parts)* | operational | config, cluster-wire |
| [F-ADM-064](11-security-admin-and-observability.md#f-adm-064) | MDS option names read or written by peers | operational | config |
| [F-ADM-070](11-security-admin-and-observability.md#f-adm-070) | Prometheus export through ceph-exporter | operational | admin, metrics |
| [F-ADM-074](11-security-admin-and-observability.md#f-adm-074) | MDS_TRIM: behind on trimming | operational | metrics |
| [F-ADM-079](11-security-admin-and-observability.md#f-adm-079) | MDS_CACHE_OVERSIZED | operational | metrics |
| [F-ADM-082](11-security-admin-and-observability.md#f-adm-082) | Monitor-derived file-system health checks *(legacy parts)* | operational | metrics |
| [F-ADM-083](11-security-admin-and-observability.md#f-adm-083) | Health-gated monitor commands | operational | metrics, admin |
| [F-ADM-111](11-security-admin-and-observability.md#f-adm-111) | mgr/mds_autoscaler | operational | cluster-wire |
| [F-ADM-113](11-security-admin-and-observability.md#f-adm-113) | cephadm MDS deployment | operational | config, cluster-wire |
| [F-ADM-114](11-security-admin-and-observability.md#f-adm-114) | cephadm MDS upgrade choreography | operational | cluster-wire |

## rook

Rook operator. 54 features (1 legacy entry left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-026](01-cluster-membership-and-ha.md#f-clu-026) | Unique daemon names | operational | cluster-wire, config |
| [F-CLU-027](01-cluster-membership-and-ha.md#f-clu-027) | DNE beacon on clean shutdown | operational | cluster-wire |
| [F-CLU-030](01-cluster-membership-and-ha.md#f-clu-030) | Standby selection scoring | operational | cluster-wire |
| [F-CLU-031](01-cluster-membership-and-ha.md#f-clu-031) | mds_join_fs file system affinity | operational | cluster-wire, config |
| [F-CLU-042](01-cluster-membership-and-ha.md#f-clu-042) | fs new | operational | admin, cluster-wire |
| [F-CLU-043](01-cluster-membership-and-ha.md#f-clu-043) | fs rm | operational | admin, cluster-wire |
| [F-CLU-044](01-cluster-membership-and-ha.md#f-clu-044) | fs fail and the joinable flag *(legacy parts)* | operational | admin, cluster-wire |
| [F-CLU-046](01-cluster-membership-and-ha.md#f-clu-046) | fs set down | operational | admin, cluster-wire |
| [F-CLU-048](01-cluster-membership-and-ha.md#f-clu-048) | mds fail | operational | admin, cluster-wire |
| [F-CLU-051](01-cluster-membership-and-ha.md#f-clu-051) | Read-only fs and mds commands and the fs dump layout *(legacy parts)* | operational | admin |
| [F-CLU-052](01-cluster-membership-and-ha.md#f-clu-052) | Orchestration helpers: mds ok-to-stop, last-seen, metadata, versions | operational | admin |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-062](01-cluster-membership-and-ha.md#f-clu-062) | File-system health checks derived from the map | operational | metrics, admin |
| [F-CLU-105](01-cluster-membership-and-ha.md#f-clu-105) | Respawn: re-exec as a fresh standby | operational | cluster-wire |
| [F-CLU-106](01-cluster-membership-and-ha.md#f-clu-106) | Suicide and orderly shutdown | operational | cluster-wire, admin |
| [F-CLU-108](01-cluster-membership-and-ha.md#f-clu-108) | Process bootstrap (ceph-mds -i <id>) *(legacy parts)* | operational | config, cluster-wire |
| [F-CLU-110](01-cluster-membership-and-ha.md#f-clu-110) | MonClient session, keys and subscriptions *(legacy parts)* | operational | cluster-wire, config |
| [F-STORE-101](06-journal-and-storage.md#f-store-101) | Pool requirements for CephFS pools | operational | cluster-wire |
| [F-STORE-102](06-journal-and-storage.md#f-store-102) | Metadata pool shared with mgr-owned objects *(legacy parts)* | operational | on-disk |
| [F-MULTI-038](07-multi-mds.md#f-multi-038) | Set and remove ceph.dir.pin | extension | client-wire, on-disk |
| [F-MULTI-040](07-multi-mds.md#f-multi-040) | Pin inheritance and precedence resolution | extension | internal |
| [F-MULTI-041](07-multi-mds.md#f-multi-041) | Pin enforcement and aux subtrees (handle_export_pins) | extension | internal |
| [F-MULTI-045](07-multi-mds.md#f-multi-045) | Distributed ephemeral pins (ceph.dir.pin.distributed) | extension | client-wire, on-disk |
| [F-BG-022](09-background-services.md#f-bg-022) | Rank stop: export all strays to rank 0 | operational | mds-wire |
| [F-BG-023](09-background-services.md#f-bg-023) | Rank stop: drain the PurgeQueue | operational | metrics |
| [F-BG-059](09-background-services.md#f-bg-059) | damage ls and damage rm commands *(legacy parts)* | operational | admin |
| [F-ADM-001](11-security-admin-and-observability.md#f-adm-001) | MDS self-authentication, keyring and rotating service secrets | operational | cluster-wire, config |
| [F-ADM-005](11-security-admin-and-observability.md#f-adm-005) | Daemon key rotation commands (rotate-key, wipe-rotating-secrets) | operational | admin |
| [F-ADM-026](11-security-admin-and-observability.md#f-adm-026) | Local admin socket transport *(legacy parts)* | operational | admin, config |
| [F-ADM-028](11-security-admin-and-observability.md#f-adm-028) | Tell authorization: allow * required | operational | admin |
| [F-ADM-038](11-security-admin-and-observability.md#f-adm-038) | session evict / client evict / session kill and the session filter grammar | operational | admin, cluster-wire |
| [F-ADM-050](11-security-admin-and-observability.md#f-adm-050) | damage ls and damage rm | operational | admin |
| [F-ADM-060](11-security-admin-and-observability.md#f-adm-060) | MDS option catalogue and option metadata | operational | config |
| [F-ADM-062](11-security-admin-and-observability.md#f-adm-062) | Config delivery from the monitor config DB and reporting to the mgr *(legacy parts)* | operational | config, cluster-wire |
| [F-ADM-064](11-security-admin-and-observability.md#f-adm-064) | MDS option names read or written by peers | operational | config |
| [F-ADM-070](11-security-admin-and-observability.md#f-adm-070) | Prometheus export through ceph-exporter | operational | admin, metrics |
| [F-ADM-073](11-security-admin-and-observability.md#f-adm-073) | Health metric pipeline (MDSHealth in beacons to monitor checks) | operational | cluster-wire, metrics |
| [F-ADM-074](11-security-admin-and-observability.md#f-adm-074) | MDS_TRIM: behind on trimming | operational | metrics |
| [F-ADM-078](11-security-admin-and-observability.md#f-adm-078) | MDS_DAMAGE and MDS_READ_ONLY state warnings | operational | metrics |
| [F-ADM-079](11-security-admin-and-observability.md#f-adm-079) | MDS_CACHE_OVERSIZED | operational | metrics |
| [F-ADM-082](11-security-admin-and-observability.md#f-adm-082) | Monitor-derived file-system health checks *(legacy parts)* | operational | metrics |
| [F-ADM-083](11-security-admin-and-observability.md#f-adm-083) | Health-gated monitor commands | operational | metrics, admin |
| [F-ADM-088](11-security-admin-and-observability.md#f-adm-088) | mgr/volumes: volume lifecycle (fs volume create, rm, rename, ls, info) | operational | client-wire, cluster-wire |
| [F-ADM-090](11-security-admin-and-observability.md#f-adm-090) | mgr/volumes: subvolume groups | operational | client-wire |
| [F-ADM-091](11-security-admin-and-observability.md#f-adm-091) | mgr/volumes: subvolume create | operational | client-wire |
| [F-ADM-092](11-security-admin-and-observability.md#f-adm-092) | mgr/volumes: removal, trash, snapshot retention and asynchronous purge | operational | client-wire |
| [F-ADM-093](11-security-admin-and-observability.md#f-adm-093) | mgr/volumes: subvolume resize via quota | operational | client-wire |
| [F-ADM-094](11-security-admin-and-observability.md#f-adm-094) | mgr/volumes: subvolume and group pinning | operational | client-wire, admin |
| [F-ADM-097](11-security-admin-and-observability.md#f-adm-097) | mgr/volumes: subvolume snapshots | operational | client-wire |
| [F-ADM-103](11-security-admin-and-observability.md#f-adm-103) | mgr/snap_schedule: scheduled snapshots and retention *(legacy parts)* | operational | client-wire, on-disk, config |
| [F-ADM-104](11-security-admin-and-observability.md#f-adm-104) | mgr/mirroring: enable, peers, bootstrap and directory distribution | operational | client-wire, on-disk, cluster-wire |
| [F-ADM-115](11-security-admin-and-observability.md#f-adm-115) | Rook operator: MDS pods, command line, keyring, config and probes | operational | config, cluster-wire |
| [F-ADM-116](11-security-admin-and-observability.md#f-adm-116) | Rook operator: upgrade order, ok-to-stop gate and health gating | operational | cluster-wire, metrics |
| [F-ADM-117](11-security-admin-and-observability.md#f-adm-117) | cephfs-mirror daemon | operational | client-wire |

## osd

OSDs (RADOS operations, blocklists, snap contexts). 88 features (2 legacy entries left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-011](01-cluster-membership-and-ha.md#f-clu-011) | Data pool list of a file system | extension | cluster-wire, client-wire, admin |
| [F-CLU-036](01-cluster-membership-and-ha.md#f-clu-036) | Fencing failed rank holders with OSD blocklists | supporting | cluster-wire |
| [F-CLU-037](01-cluster-membership-and-ha.md#f-clu-037) | Wait for last_failure_osd_epoch before touching rank objects | supporting | cluster-wire |
| [F-CLU-038](01-cluster-membership-and-ha.md#f-clu-038) | Rank incarnation | supporting | cluster-wire, client-wire |
| [F-CLU-042](01-cluster-membership-and-ha.md#f-clu-042) | fs new | operational | admin, cluster-wire |
| [F-CLU-044](01-cluster-membership-and-ha.md#f-clu-044) | fs fail and the joinable flag *(legacy parts)* | operational | admin, cluster-wire |
| [F-CLU-048](01-cluster-membership-and-ha.md#f-clu-048) | mds fail | operational | admin, cluster-wire |
| [F-CLU-058](01-cluster-membership-and-ha.md#f-clu-058) | Rank-level map handling and peer change detection | operational | cluster-wire |
| [F-CLU-063](01-cluster-membership-and-ha.md#f-clu-063) | Boot stages for up:starting, up:replay and up:standby-replay | supporting | on-disk |
| [F-CLU-064](01-cluster-membership-and-ha.md#f-clu-064) | Creating a new rank (up:creating) | supporting | on-disk, cluster-wire |
| [F-CLU-066](01-cluster-membership-and-ha.md#f-clu-066) | up:replay: journal replay of a failed rank | supporting | on-disk, metrics |
| [F-CLU-068](01-cluster-membership-and-ha.md#f-clu-068) | Standby-replay follower loop | operational | cluster-wire, config |
| [F-CLU-080](01-cluster-membership-and-ha.md#f-clu-080) | up:reconnect: the client reconnect window and its timeout | supporting | client-wire, config |
| [F-CLU-087](01-cluster-membership-and-ha.md#f-clu-087) | Matching reconnected caps to inodes: OpenFileTable prefetch and open by number | supporting | on-disk, client-wire |
| [F-CLU-093](01-cluster-membership-and-ha.md#f-clu-093) | Rejoin cache repair: invented objects, stale fragmentation, missed link/unlink | internal | mds-wire, on-disk |
| [F-CLU-099](01-cluster-membership-and-ha.md#f-clu-099) | File size and mtime recovery for writers that did not come back | supporting | client-wire, on-disk, metrics |
| [F-CLU-100](01-cluster-membership-and-ha.md#f-clu-100) | Re-executing interrupted truncates after recovery | supporting | on-disk |
| [F-CLU-101](01-cluster-membership-and-ha.md#f-clu-101) | Purging orphaned preallocated and delegated inodes | supporting | on-disk |
| [F-CLU-105](01-cluster-membership-and-ha.md#f-clu-105) | Respawn: re-exec as a fresh standby | operational | cluster-wire |
| [F-CLU-107](01-cluster-membership-and-ha.md#f-clu-107) | Write error policy | operational | config, metrics |
| [F-CLU-109](01-cluster-membership-and-ha.md#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config |
| [F-CLU-112](01-cluster-membership-and-ha.md#f-clu-112) | Completion contexts and the rank finisher | internal | internal |
| [F-CLU-114](01-cluster-membership-and-ha.md#f-clu-114) | Dispatcher chain, message priorities and ordering | internal | internal |
| [F-PROTO-005](02-client-sessions-and-protocol.md#f-proto-005) | Address vector publication and server identity checks *(legacy parts)* | supporting | client-wire, cluster-wire, config |
| [F-PROTO-045](02-client-sessions-and-protocol.md#f-proto-045) | Session autoclose eviction | supporting | client-wire, cluster-wire, config |
| [F-PROTO-047](02-client-sessions-and-protocol.md#f-proto-047) | Eviction deferral while OSDs are laggy | operational | config, metrics |
| [F-PROTO-051](02-client-sessions-and-protocol.md#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin |
| [F-PROTO-052](02-client-sessions-and-protocol.md#f-proto-052) | OSDMap blocklist-driven session kill | supporting | client-wire, cluster-wire |
| [F-PROTO-054](02-client-sessions-and-protocol.md#f-proto-054) | Session termination at rank shutdown | operational | client-wire, cluster-wire |
| [F-PROTO-105](02-client-sessions-and-protocol.md#f-proto-105) | Objecter obligations (OSD client protocol) *(legacy parts)* | supporting | cluster-wire |
| [F-PROTO-106](02-client-sessions-and-protocol.md#f-proto-106) | Fencing identity: one address vector for clients, monitors and OSDs *(legacy parts)* | supporting | cluster-wire |
| [F-CAPS-022](03-caps-leases-and-locking.md#f-caps-022) | File size recovery after loss of a writer | supporting | cluster-wire, on-disk |
| [F-CAPS-030](03-caps-leases-and-locking.md#f-caps-030) | Stale-session cap handling *(legacy parts)* | supporting | client-wire, config |
| [F-NS-018](04-namespace-and-posix-operations.md#f-ns-018) | Full metadata pool gate | required | client-wire, cluster-wire |
| [F-NS-042](04-namespace-and-posix-operations.md#f-ns-042) | File layout selection and validation at create | extension | client-wire |
| [F-NS-048](04-namespace-and-posix-operations.md#f-ns-048) | Unlinked-but-open inodes stay usable until the last cap is released | required | on-disk |
| [F-NS-062](04-namespace-and-posix-operations.md#f-ns-062) | Truncate (shrink) with asynchronous OSD truncation | required | client-wire, cluster-wire, on-disk |
| [F-NS-064](04-namespace-and-posix-operations.md#f-ns-064) | fscrypt inode fields and last-block truncate *(legacy parts)* | extension | client-wire, on-disk, config |
| [F-NS-072](04-namespace-and-posix-operations.md#f-ns-072) | Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*) *(legacy parts)* | extension | client-wire, cluster-wire |
| [F-CACHE-006](05-metadata-cache-and-dirfrags.md#f-cache-006) | Truncation state fields | required | client-wire, on-disk |
| [F-CACHE-047](05-metadata-cache-and-dirfrags.md#f-cache-047) | Lookup by inode number (open_ino) | supporting | client-wire, on-disk, mds-wire |
| [F-CACHE-049](05-metadata-cache-and-dirfrags.md#f-cache-049) | Batched dentry-key fetches for open_ino | internal | internal |
| [F-CACHE-087](05-metadata-cache-and-dirfrags.md#f-cache-087) | Fragment operation (FRAGMENTDIR) | internal | on-disk |
| [F-CACHE-089](05-metadata-cache-and-dirfrags.md#f-cache-089) | New dirfrag object write and old object purge | internal | on-disk |
| [F-CACHE-094](05-metadata-cache-and-dirfrags.md#f-cache-094) | Recovery of in-flight fragment operations | internal | on-disk |
| [F-STORE-003](06-journal-and-storage.md#f-store-003) | Striped journal byte stream and its RADOS objects | internal | on-disk |
| [F-STORE-013](06-journal-and-storage.md#f-store-013) | Journal prezeroing ahead of the write position | internal | on-disk |
| [F-STORE-016](06-journal-and-storage.md#f-store-016) | Log segments as units of lazy writeback (segment expiry) | supporting | on-disk |
| [F-STORE-019](06-journal-and-storage.md#f-store-019) | Major-group trimming and journal object deletion | internal | on-disk |
| [F-STORE-034](06-journal-and-storage.md#f-store-034) | EFragment journal protocol and old dirfrag object cleanup | internal | on-disk |
| [F-STORE-035](06-journal-and-storage.md#f-store-035) | Journal open and recovery thread *(legacy parts)* | internal | on-disk |
| [F-STORE-036](06-journal-and-storage.md#f-store-036) | End-of-journal probe and torn-tail handling | supporting | on-disk |
| [F-STORE-042](06-journal-and-storage.md#f-store-042) | Single-writer fencing before reading rank state | supporting | cluster-wire |
| [F-STORE-051](06-journal-and-storage.md#f-store-051) | Dirfrag fetch from RADOS | supporting | on-disk, config |
| [F-STORE-052](06-journal-and-storage.md#f-store-052) | Dirfrag commit to RADOS | supporting | on-disk, config |
| [F-STORE-058](06-journal-and-storage.md#f-store-058) | Backtrace parent xattr on object 0 | supporting | on-disk |
| [F-STORE-060](06-journal-and-storage.md#f-store-060) | Backtrace copies in the default data pool and old pools | operational | on-disk |
| [F-STORE-064](06-journal-and-storage.md#f-store-064) | Data object naming and striping shared with clients | supporting | client-wire, on-disk |
| [F-STORE-065](06-journal-and-storage.md#f-store-065) | Data-object truncation (truncate start/finish and TRIMTRUNC) | required | on-disk, cluster-wire |
| [F-STORE-066](06-journal-and-storage.md#f-store-066) | File size and mtime recovery by probing data objects | supporting | cluster-wire |
| [F-STORE-068](06-journal-and-storage.md#f-store-068) | MDSTable: versioned single-object table persistence *(legacy parts)* | internal | on-disk |
| [F-STORE-072](06-journal-and-storage.md#f-store-072) | Release of preallocated and delegated numbers at session close (EPurged) | internal | on-disk, cluster-wire |
| [F-STORE-085](06-journal-and-storage.md#f-store-085) | OpenFileTable objects and commit with redo journal | internal | on-disk |
| [F-STORE-090](06-journal-and-storage.md#f-store-090) | Per-rank Objecter RADOS client | internal | cluster-wire |
| [F-STORE-091](06-journal-and-storage.md#f-store-091) | RADOS identity and request ids | internal | cluster-wire |
| [F-STORE-092](06-journal-and-storage.md#f-store-092) | FULL_FORCE: MDS I/O ignores full flags | operational | cluster-wire |
| [F-STORE-093](06-journal-and-storage.md#f-store-093) | Continuous OSDMap following | operational | cluster-wire |
| [F-STORE-095](06-journal-and-storage.md#f-store-095) | I/O completion path and self-respawn when fenced | operational | internal |
| [F-MULTI-036](07-multi-mds.md#f-multi-036) | Mantle Lua balancer policy | operational | cluster-wire, admin, config |
| [F-SNAP-001](08-snapshots-quotas-subvolumes.md#f-snap-001) | Snapshot id and realm seq allocation from one monotonic counter | extension | client-wire, cluster-wire, on-disk |
| [F-SNAP-003](08-snapshots-quotas-subvolumes.md#f-snap-003) | Deleted snapshots reported to the monitor (MRemoveSnaps) *(legacy parts)* | extension | cluster-wire, on-disk |
| [F-SNAP-004](08-snapshots-quotas-subvolumes.md#f-snap-004) | MDS data writes carry the realm SnapContext *(legacy parts)* | extension | cluster-wire |
| [F-SNAP-030](08-snapshots-quotas-subvolumes.md#f-snap-030) | File block diff between snapshots (FILE_BLOCKDIFF) | extension | client-wire, cluster-wire |
| [F-BG-009](09-background-services.md#f-bg-009) | Snapshot-aware purge decision (past-parent snaps, realm SnapContext) | extension | cluster-wire |
| [F-BG-010](09-background-services.md#f-bg-010) | Truncate-only purge for multiversion files (TRUNCATE_FILE) | extension | on-disk, cluster-wire |
| [F-BG-015](09-background-services.md#f-bg-015) | File data and backtrace object deletion (PURGE_FILE) | supporting | cluster-wire, on-disk |
| [F-BG-016](09-background-services.md#f-bg-016) | Directory object deletion (PURGE_DIR) | supporting | cluster-wire, on-disk |
| [F-BG-017](09-background-services.md#f-bg-017) | Purge throttling and pacing | operational | config |
| [F-BG-029](09-background-services.md#f-bg-029) | Scrub tags and the scrub_tag object xattr | operational | on-disk, admin |
| [F-BG-030](09-background-services.md#f-bg-030) | tag path command | operational | admin |
| [F-BG-039](09-background-services.md#f-bg-039) | Backtrace validation | operational | on-disk, admin |
| [F-BG-040](09-background-services.md#f-bg-040) | Backtrace repair | operational | on-disk |
| [F-ADM-001](11-security-admin-and-observability.md#f-adm-001) | MDS self-authentication, keyring and rotating service secrets | operational | cluster-wire, config |
| [F-ADM-004](11-security-admin-and-observability.md#f-adm-004) | Peer-type message filtering and rank address check | internal | client-wire, mds-wire, cluster-wire |
| [F-ADM-038](11-security-admin-and-observability.md#f-adm-038) | session evict / client evict / session kill and the session filter grammar | operational | admin, cluster-wire |
| [F-ADM-049](11-security-admin-and-observability.md#f-adm-049) | tag path and the scrub_tag xattr | operational | admin, on-disk |
| [F-ADM-119](11-security-admin-and-observability.md#f-adm-119) | Offline journal and table tools (cephfs-journal-tool, cephfs-table-tool) | operational | on-disk |
| [F-ADM-120](11-security-admin-and-observability.md#f-adm-120) | Offline data and metadata repair tools (cephfs-data-scan, cephfs-meta-injection, first-damage.py) | operational | on-disk |

## tools

cephfs-journal-tool, cephfs-data-scan, cephfs-table-tool, cephfs-top, cephfs-shell and similar tools. 136 features (1 legacy entry left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-001](01-cluster-membership-and-ha.md#f-clu-001) | FSMap: monitor-owned map of all file systems | operational | cluster-wire |
| [F-CLU-004](01-cluster-membership-and-ha.md#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire |
| [F-CLU-025](01-cluster-membership-and-ha.md#f-clu-025) | Boot registration as a standby with daemon metadata | operational | cluster-wire, admin |
| [F-CLU-045](01-cluster-membership-and-ha.md#f-clu-045) | fs reset | operational | admin, cluster-wire |
| [F-CLU-049](01-cluster-membership-and-ha.md#f-clu-049) | Damaged ranks and mds repaired | operational | admin, cluster-wire, metrics |
| [F-CLU-050](01-cluster-membership-and-ha.md#f-clu-050) | Hidden and debug rank commands: mds rm, rmfailed, freeze, set_state | operational | admin, cluster-wire |
| [F-CLU-051](01-cluster-membership-and-ha.md#f-clu-051) | Read-only fs and mds commands and the fs dump layout *(legacy parts)* | operational | admin |
| [F-CLU-052](01-cluster-membership-and-ha.md#f-clu-052) | Orchestration helpers: mds ok-to-stop, last-seen, metadata, versions | operational | admin |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-064](01-cluster-membership-and-ha.md#f-clu-064) | Creating a new rank (up:creating) | supporting | on-disk, cluster-wire |
| [F-CLU-104](01-cluster-membership-and-ha.md#f-clu-104) | Reporting a rank as DAMAGED | operational | cluster-wire, metrics |
| [F-CLU-108](01-cluster-membership-and-ha.md#f-clu-108) | Process bootstrap (ceph-mds -i <id>) *(legacy parts)* | operational | config, cluster-wire |
| [F-PROTO-019](02-client-sessions-and-protocol.md#f-proto-019) | Golden encodings and encoding test data | operational | internal |
| [F-CAPS-095](03-caps-leases-and-locking.md#f-caps-095) | Cap and lease counters and dumps | operational | metrics, admin |
| [F-CAPS-096](03-caps-leases-and-locking.md#f-caps-096) | Lock state dumps and lock-related op events | operational | admin |
| [F-NS-009](04-namespace-and-posix-operations.md#f-ns-009) | Request-path fault injection and test kill points | internal | config |
| [F-NS-026](04-namespace-and-posix-operations.md#f-ns-026) | LOOKUPINO and LOOKUPHASH (resolve an inode number) | supporting | client-wire, mds-wire, on-disk |
| [F-NS-040](04-namespace-and-posix-operations.md#f-ns-040) | Inode number allocation (session preallocation and InoTable) | supporting | on-disk, config |
| [F-NS-073](04-namespace-and-posix-operations.md#f-ns-073) | GETVXATTR op | extension | client-wire |
| [F-NS-092](04-namespace-and-posix-operations.md#f-ns-092) | Conformance suites in the tree (pjd-fstest, xfstests) | operational | internal |
| [F-CACHE-001](05-metadata-cache-and-dirfrags.md#f-cache-001) | Inode record field set (inode_t) | required | on-disk, client-wire |
| [F-CACHE-070](05-metadata-cache-and-dirfrags.md#f-cache-070) | mkfs base hierarchy creation | operational | on-disk |
| [F-CACHE-089](05-metadata-cache-and-dirfrags.md#f-cache-089) | New dirfrag object write and old object purge | internal | on-disk |
| [F-STORE-001](06-journal-and-storage.md#f-store-001) | Journal pointer object (front/back journal inode) *(legacy parts)* | internal | on-disk |
| [F-STORE-002](06-journal-and-storage.md#f-store-002) | Journaler header object | internal | on-disk |
| [F-STORE-003](06-journal-and-storage.md#f-store-003) | Striped journal byte stream and its RADOS objects | internal | on-disk |
| [F-STORE-004](06-journal-and-storage.md#f-store-004) | RESILIENT journal entry framing *(legacy parts)* | internal | on-disk |
| [F-STORE-005](06-journal-and-storage.md#f-store-005) | LogEvent envelope and event type ids *(legacy parts)* | internal | on-disk |
| [F-STORE-013](06-journal-and-storage.md#f-store-013) | Journal prezeroing ahead of the write position | internal | on-disk |
| [F-STORE-014](06-journal-and-storage.md#f-store-014) | Journal header refresh | internal | on-disk |
| [F-STORE-020](06-journal-and-storage.md#f-store-020) | flush journal admin command | operational | admin |
| [F-STORE-022](06-journal-and-storage.md#f-store-022) | EMetaBlob after-image container *(legacy parts)* | internal | on-disk |
| [F-STORE-025](06-journal-and-storage.md#f-store-025) | ESubtreeMap: major boundary with the subtree map | internal | on-disk |
| [F-STORE-045](06-journal-and-storage.md#f-store-045) | Metadata object naming and placement *(legacy parts)* | internal | on-disk |
| [F-STORE-046](06-journal-and-storage.md#f-store-046) | Special inode numbers and per-rank ranges *(legacy parts)* | supporting | client-wire, on-disk |
| [F-STORE-047](06-journal-and-storage.md#f-store-047) | Dirfrag omap object and fnode header | supporting | on-disk |
| [F-STORE-048](06-journal-and-storage.md#f-store-048) | Dentry omap keys, including snapshotted dentries | supporting | on-disk |
| [F-STORE-049](06-journal-and-storage.md#f-store-049) | Primary dentry value with embedded inode (InodeStore) | supporting | on-disk |
| [F-STORE-050](06-journal-and-storage.md#f-store-050) | Remote (hard link) dentry value *(legacy parts)* | supporting | on-disk |
| [F-STORE-053](06-journal-and-storage.md#f-store-053) | Base inode objects (.inode) *(legacy parts)* | supporting | on-disk |
| [F-STORE-054](06-journal-and-storage.md#f-store-054) | Per-rank mdsdir and stray directories *(legacy parts)* | internal | on-disk |
| [F-STORE-057](06-journal-and-storage.md#f-store-057) | Legacy on-disk formats still decoded *(legacy parts)* | internal | on-disk |
| [F-STORE-058](06-journal-and-storage.md#f-store-058) | Backtrace parent xattr on object 0 | supporting | on-disk |
| [F-STORE-059](06-journal-and-storage.md#f-store-059) | Layout and symlink xattrs on object 0 | operational | on-disk |
| [F-STORE-060](06-journal-and-storage.md#f-store-060) | Backtrace copies in the default data pool and old pools | operational | on-disk |
| [F-STORE-063](06-journal-and-storage.md#f-store-063) | Scrub tag xattr on object 0 *(legacy parts)* | operational | on-disk, admin |
| [F-STORE-064](06-journal-and-storage.md#f-store-064) | Data object naming and striping shared with clients | supporting | client-wire, on-disk |
| [F-STORE-068](06-journal-and-storage.md#f-store-068) | MDSTable: versioned single-object table persistence *(legacy parts)* | internal | on-disk |
| [F-STORE-069](06-journal-and-storage.md#f-store-069) | InoTable object and per-rank inode ranges | supporting | on-disk |
| [F-STORE-075](06-journal-and-storage.md#f-store-075) | SessionMap object | supporting | on-disk, config |
| [F-STORE-077](06-journal-and-storage.md#f-store-077) | SnapTable object (mds_snaptable) *(legacy parts)* | extension | on-disk |
| [F-STORE-102](06-journal-and-storage.md#f-store-102) | Metadata pool shared with mgr-owned objects *(legacy parts)* | operational | on-disk |
| [F-STORE-108](06-journal-and-storage.md#f-store-108) | Offline journal reset and EResetJournal | operational | on-disk, admin |
| [F-STORE-109](06-journal-and-storage.md#f-store-109) | cephfs-journal-tool inspection, export/import, header and event commands | operational | on-disk, admin |
| [F-STORE-110](06-journal-and-storage.md#f-store-110) | Other offline tools over the on-disk format | operational | on-disk |
| [F-STORE-112](06-journal-and-storage.md#f-store-112) | Developer and fault-injection options *(legacy parts)* | internal | config |
| [F-MULTI-021](07-multi-mds.md#f-multi-021) | Migration journaling (EImportStart, EExport, EImportFinish) | supporting | on-disk |
| [F-MULTI-037](07-multi-mds.md#f-multi-037) | Mantle cluster log messages | operational | admin |
| [F-SNAP-002](08-snapshots-quotas-subvolumes.md#f-snap-002) | Rank-0 snap table service (SnapServer) and per-rank SnapClient cache *(legacy parts)* | internal | mds-wire, on-disk |
| [F-SNAP-006](08-snapshots-quotas-subvolumes.md#f-snap-006) | Snap realms and effective snapshot sets *(legacy parts)* | extension | client-wire, on-disk |
| [F-SNAP-026](08-snapshots-quotas-subvolumes.md#f-snap-026) | Metadata copy-on-write (first/last ranges, snapped dentries, old_inodes) | extension | on-disk |
| [F-SNAP-034](08-snapshots-quotas-subvolumes.md#f-snap-034) | Recursive subtree statistics (rstat) | extension | client-wire, on-disk |
| [F-SNAP-042](08-snapshots-quotas-subvolumes.md#f-snap-042) | Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files) | extension | client-wire, on-disk |
| [F-SNAP-050](08-snapshots-quotas-subvolumes.md#f-snap-050) | Subvolume flag (ceph.dir.subvolume) | extension | client-wire, on-disk |
| [F-BG-001](09-background-services.md#f-bg-001) | Per-rank stray directories (~mdsN/stray0..stray9) *(legacy parts)* | internal | on-disk |
| [F-BG-013](09-background-services.md#f-bg-013) | PurgeQueue journal of PurgeItems *(legacy parts)* | internal | on-disk |
| [F-BG-027](09-background-services.md#f-bg-027) | dump stray admin command | operational | admin |
| [F-BG-029](09-background-services.md#f-bg-029) | Scrub tags and the scrub_tag object xattr | operational | on-disk, admin |
| [F-BG-030](09-background-services.md#f-bg-030) | tag path command | operational | admin |
| [F-BG-046](09-background-services.md#f-bg-046) | Dentry snapshot-bound check during scrub | operational | admin |
| [F-BG-054](09-background-services.md#f-bg-054) | Dentry snapshot-bound corruption check at load and commit | operational | config, metrics |
| [F-BG-059](09-background-services.md#f-bg-059) | damage ls and damage rm commands *(legacy parts)* | operational | admin |
| [F-QOS-020](10-qos-metrics-quiesce-quarantine.md#f-qos-020) | Client session lifecycle in metrics (REFRESH and REMOVE) | operational | mds-wire, metrics |
| [F-QOS-022](10-qos-metrics-quiesce-quarantine.md#f-qos-022) | Rank telemetry: CPU usage and open requests (mds_rank_perf) | operational | metrics, mds-wire |
| [F-QOS-023](10-qos-metrics-quiesce-quarantine.md#f-qos-023) | Subvolume quota_bytes and used_bytes reporting | operational | metrics |
| [F-QOS-024](10-qos-metrics-quiesce-quarantine.md#f-qos-024) | Per-client labeled metric counters (mds_client_metrics) | operational | metrics |
| [F-QOS-025](10-qos-metrics-quiesce-quarantine.md#f-qos-025) | Subvolume sliding-window metrics (mds_subvolume_metrics) | operational | metrics, mds-wire, config |
| [F-QOS-026](10-qos-metrics-quiesce-quarantine.md#f-qos-026) | Eviction of idle subvolume metrics | operational | metrics |
| [F-QOS-027](10-qos-metrics-quiesce-quarantine.md#f-qos-027) | Labeled metric counter naming and export | operational | metrics, admin |
| [F-QOS-053](10-qos-metrics-quiesce-quarantine.md#f-qos-053) | quiesce db command | extension | admin |
| [F-QOS-057](10-qos-metrics-quiesce-quarantine.md#f-qos-057) | Quiesce observability: counters and op events | operational | metrics, admin |
| [F-QOS-058](10-qos-metrics-quiesce-quarantine.md#f-qos-058) | quarantine enable and disable commands | extension | admin |
| [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) | Peer authorizer verification and caps loading at connection accept *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-ADM-006](11-security-admin-and-observability.md#f-adm-006) | MDS cap string grammar, parser and canonical forms | operational | admin, cluster-wire |
| [F-ADM-026](11-security-admin-and-observability.md#f-adm-026) | Local admin socket transport *(legacy parts)* | operational | admin, config |
| [F-ADM-027](11-security-admin-and-observability.md#f-adm-027) | Remote tell transport (MCommand / MCommandReply) *(legacy parts)* | operational | admin, client-wire |
| [F-ADM-028](11-security-admin-and-observability.md#f-adm-028) | Tell authorization: allow * required | operational | admin |
| [F-ADM-029](11-security-admin-and-observability.md#f-adm-029) | Command self-description (get_command_descriptions, help) | operational | admin |
| [F-ADM-030](11-security-admin-and-observability.md#f-adm-030) | Argument validation, output formats, output-file and return-code conventions | operational | admin |
| [F-ADM-031](11-security-admin-and-observability.md#f-adm-031) | Client-side target resolution for tell (libcephfs mds_command) | operational | admin, cluster-wire |
| [F-ADM-032](11-security-admin-and-observability.md#f-adm-032) | status command | operational | admin |
| [F-ADM-033](11-security-admin-and-observability.md#f-adm-033) | Process control commands: exit, respawn, lockup | operational | admin |
| [F-ADM-035](11-security-admin-and-observability.md#f-adm-035) | Op tracker dumps (ops, dump_ops_in_flight, blocked and historic ops) | operational | admin, config |
| [F-ADM-036](11-security-admin-and-observability.md#f-adm-036) | op get and op kill | operational | admin |
| [F-ADM-037](11-security-admin-and-observability.md#f-adm-037) | session ls / client ls | operational | admin |
| [F-ADM-038](11-security-admin-and-observability.md#f-adm-038) | session evict / client evict / session kill and the session filter grammar | operational | admin, cluster-wire |
| [F-ADM-039](11-security-admin-and-observability.md#f-adm-039) | session config / client config timeout | operational | admin |
| [F-ADM-040](11-security-admin-and-observability.md#f-adm-040) | flush journal | operational | admin, on-disk |
| [F-ADM-041](11-security-admin-and-observability.md#f-adm-041) | flush_path | operational | admin, on-disk |
| [F-ADM-043](11-security-admin-and-observability.md#f-adm-043) | osdmap barrier | operational | admin |
| [F-ADM-044](11-security-admin-and-observability.md#f-adm-044) | cache drop | operational | admin, client-wire |
| [F-ADM-045](11-security-admin-and-observability.md#f-adm-045) | cache status and dump cache | operational | admin, config |
| [F-ADM-046](11-security-admin-and-observability.md#f-adm-046) | Cache and table dumps: dump tree, dump inode, dump dir, openfiles ls, dump snaps | operational | admin |
| [F-ADM-047](11-security-admin-and-observability.md#f-adm-047) | dump stray | operational | admin |
| [F-ADM-048](11-security-admin-and-observability.md#f-adm-048) | Scrub control commands (scrub start, abort, pause, resume, status, purge_status) *(legacy parts)* | operational | admin, mds-wire |
| [F-ADM-049](11-security-admin-and-observability.md#f-adm-049) | tag path and the scrub_tag xattr | operational | admin, on-disk |
| [F-ADM-050](11-security-admin-and-observability.md#f-adm-050) | damage ls and damage rm | operational | admin |
| [F-ADM-051](11-security-admin-and-observability.md#f-adm-051) | Multi-MDS inspection and control: get subtrees, dump loads, export dir, dump_export_states | operational | admin |
| [F-ADM-052](11-security-admin-and-observability.md#f-adm-052) | dirfrag split, merge and ls | operational | admin |
| [F-ADM-053](11-security-admin-and-observability.md#f-adm-053) | lock path and quiesce path test hooks | operational | admin |
| [F-ADM-054](11-security-admin-and-observability.md#f-adm-054) | quiesce db command | extension | admin |
| [F-ADM-055](11-security-admin-and-observability.md#f-adm-055) | quarantine enable / disable command | extension | admin |
| [F-ADM-057](11-security-admin-and-observability.md#f-adm-057) | Generic process commands: perf, counter, config, log, injectargs *(legacy parts)* | operational | admin, metrics, config |
| [F-ADM-058](11-security-admin-and-observability.md#f-adm-058) | dump_mempools | operational | admin |
| [F-ADM-059](11-security-admin-and-observability.md#f-adm-059) | Version and ping built-ins (0, version, git_version) | operational | admin |
| [F-ADM-060](11-security-admin-and-observability.md#f-adm-060) | MDS option catalogue and option metadata | operational | config |
| [F-ADM-065](11-security-admin-and-observability.md#f-adm-065) | Fault injection, killpoints and debug options | internal | config |
| [F-ADM-066](11-security-admin-and-observability.md#f-adm-066) | Unlabeled MDS perf counter loggers | operational | metrics |
| [F-ADM-067](11-security-admin-and-observability.md#f-adm-067) | Counter reporting to the mgr and the names mgr modules read | operational | metrics, cluster-wire |
| [F-ADM-068](11-security-admin-and-observability.md#f-adm-068) | Labeled perf counter families (client, subvolume and rank metrics) | operational | metrics |
| [F-ADM-069](11-security-admin-and-observability.md#f-adm-069) | Admin-socket counter dump and schema JSON format | operational | admin, metrics |
| [F-ADM-070](11-security-admin-and-observability.md#f-adm-070) | Prometheus export through ceph-exporter | operational | admin, metrics |
| [F-ADM-071](11-security-admin-and-observability.md#f-adm-071) | Memory gauges and gauge refresh cadence | operational | metrics |
| [F-ADM-072](11-security-admin-and-observability.md#f-adm-072) | Grafana dashboards and Prometheus alerts shipped with Ceph | operational | metrics |
| [F-ADM-073](11-security-admin-and-observability.md#f-adm-073) | Health metric pipeline (MDSHealth in beacons to monitor checks) | operational | cluster-wire, metrics |
| [F-ADM-075](11-security-admin-and-observability.md#f-adm-075) | Client-behaviour warnings: late cap release, cache-pressure recall, oldest tid | operational | metrics |
| [F-ADM-076](11-security-admin-and-observability.md#f-adm-076) | MDS_SLOW_REQUEST and slow-request detection | operational | metrics, config |
| [F-ADM-078](11-security-admin-and-observability.md#f-adm-078) | MDS_DAMAGE and MDS_READ_ONLY state warnings | operational | metrics |
| [F-ADM-082](11-security-admin-and-observability.md#f-adm-082) | Monitor-derived file-system health checks *(legacy parts)* | operational | metrics |
| [F-ADM-084](11-security-admin-and-observability.md#f-adm-084) | Cluster log (clog) messages | operational | metrics, cluster-wire |
| [F-ADM-085](11-security-admin-and-observability.md#f-adm-085) | Scrub task status in the mgr service map | operational | metrics, cluster-wire |
| [F-ADM-106](11-security-admin-and-observability.md#f-adm-106) | mgr/stats: fs perf stats (and cephfs-top) | operational | cluster-wire, admin |
| [F-ADM-109](11-security-admin-and-observability.md#f-adm-109) | mgr/prometheus: MDS and file-system metadata metrics | operational | metrics |
| [F-ADM-119](11-security-admin-and-observability.md#f-adm-119) | Offline journal and table tools (cephfs-journal-tool, cephfs-table-tool) | operational | on-disk |
| [F-ADM-120](11-security-admin-and-observability.md#f-adm-120) | Offline data and metadata repair tools (cephfs-data-scan, cephfs-meta-injection, first-damage.py) | operational | on-disk |
| [F-ADM-121](11-security-admin-and-observability.md#f-adm-121) | Client-side tools: cephfs-tool bench and cephfs-shell | operational | client-wire |

## admin

Operators and scripts using admin commands, logs and health output. 246 features (2 legacy entries left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-008](01-cluster-membership-and-ha.md#f-clu-008) | FSMap filtering for file-system-restricted sessions | operational | cluster-wire, admin |
| [F-CLU-010](01-cluster-membership-and-ha.md#f-clu-010) | Multiple file systems: enable_multiple and the default file system *(legacy parts)* | operational | cluster-wire, admin |
| [F-CLU-011](01-cluster-membership-and-ha.md#f-clu-011) | Data pool list of a file system | extension | cluster-wire, client-wire, admin |
| [F-CLU-012](01-cluster-membership-and-ha.md#f-clu-012) | max_file_size published in the MDSMap | supporting | client-wire, admin |
| [F-CLU-013](01-cluster-membership-and-ha.md#f-clu-013) | Mirroring metadata (MirrorInfo) in the FSMap | operational | cluster-wire, admin |
| [F-CLU-014](01-cluster-membership-and-ha.md#f-clu-014) | CompatSet encoding and the MDS incompat feature ids | operational | cluster-wire |
| [F-CLU-015](01-cluster-membership-and-ha.md#f-clu-015) | Standby eligibility by compat set | operational | cluster-wire |
| [F-CLU-018](01-cluster-membership-and-ha.md#f-clu-018) | Upgradeable file systems and compat merge on promotion *(legacy parts)* | operational | cluster-wire |
| [F-CLU-019](01-cluster-membership-and-ha.md#f-clu-019) | fs compat and mds compat commands *(legacy parts)* | operational | admin |
| [F-CLU-025](01-cluster-membership-and-ha.md#f-clu-025) | Boot registration as a standby with daemon metadata | operational | cluster-wire, admin |
| [F-CLU-026](01-cluster-membership-and-ha.md#f-clu-026) | Unique daemon names | operational | cluster-wire, config |
| [F-CLU-027](01-cluster-membership-and-ha.md#f-clu-027) | DNE beacon on clean shutdown | operational | cluster-wire |
| [F-CLU-028](01-cluster-membership-and-ha.md#f-clu-028) | Health metrics carried in beacons | operational | cluster-wire, metrics |
| [F-CLU-030](01-cluster-membership-and-ha.md#f-clu-030) | Standby selection scoring | operational | cluster-wire |
| [F-CLU-031](01-cluster-membership-and-ha.md#f-clu-031) | mds_join_fs file system affinity | operational | cluster-wire, config |
| [F-CLU-032](01-cluster-membership-and-ha.md#f-clu-032) | Per-file-system selection flags: refuse_standby_for_another_fs and host anti-affinity | operational | cluster-wire, admin |
| [F-CLU-033](01-cluster-membership-and-ha.md#f-clu-033) | Affinity-driven swaps of healthy daemons | operational | cluster-wire |
| [F-CLU-034](01-cluster-membership-and-ha.md#f-clu-034) | Failed-rank and laggy-holder replacement | operational | cluster-wire |
| [F-CLU-035](01-cluster-membership-and-ha.md#f-clu-035) | Promotion into a rank and the initial rank state | operational | cluster-wire |
| [F-CLU-039](01-cluster-membership-and-ha.md#f-clu-039) | Standby-replay assignment | operational | cluster-wire, admin |
| [F-CLU-040](01-cluster-membership-and-ha.md#f-clu-040) | Growing the number of active ranks (max_mds) | operational | cluster-wire, admin |
| [F-CLU-041](01-cluster-membership-and-ha.md#f-clu-041) | Shrinking: monitor-driven up:stopping and FSMap::stop | operational | cluster-wire, admin |
| [F-CLU-042](01-cluster-membership-and-ha.md#f-clu-042) | fs new | operational | admin, cluster-wire |
| [F-CLU-043](01-cluster-membership-and-ha.md#f-clu-043) | fs rm | operational | admin, cluster-wire |
| [F-CLU-044](01-cluster-membership-and-ha.md#f-clu-044) | fs fail and the joinable flag *(legacy parts)* | operational | admin, cluster-wire |
| [F-CLU-045](01-cluster-membership-and-ha.md#f-clu-045) | fs reset | operational | admin, cluster-wire |
| [F-CLU-046](01-cluster-membership-and-ha.md#f-clu-046) | fs set down | operational | admin, cluster-wire |
| [F-CLU-047](01-cluster-membership-and-ha.md#f-clu-047) | fs rename and fs swap | operational | admin, cluster-wire |
| [F-CLU-048](01-cluster-membership-and-ha.md#f-clu-048) | mds fail | operational | admin, cluster-wire |
| [F-CLU-049](01-cluster-membership-and-ha.md#f-clu-049) | Damaged ranks and mds repaired | operational | admin, cluster-wire, metrics |
| [F-CLU-050](01-cluster-membership-and-ha.md#f-clu-050) | Hidden and debug rank commands: mds rm, rmfailed, freeze, set_state | operational | admin, cluster-wire |
| [F-CLU-051](01-cluster-membership-and-ha.md#f-clu-051) | Read-only fs and mds commands and the fs dump layout *(legacy parts)* | operational | admin |
| [F-CLU-052](01-cluster-membership-and-ha.md#f-clu-052) | Orchestration helpers: mds ok-to-stop, last-seen, metadata, versions | operational | admin |
| [F-CLU-053](01-cluster-membership-and-ha.md#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin |
| [F-CLU-062](01-cluster-membership-and-ha.md#f-clu-062) | File-system health checks derived from the map | operational | metrics, admin |
| [F-CLU-064](01-cluster-membership-and-ha.md#f-clu-064) | Creating a new rank (up:creating) | supporting | on-disk, cluster-wire |
| [F-CLU-065](01-cluster-membership-and-ha.md#f-clu-065) | Restarting a stopped rank (up:starting) | supporting | on-disk, cluster-wire |
| [F-CLU-066](01-cluster-membership-and-ha.md#f-clu-066) | up:replay: journal replay of a failed rank | supporting | on-disk, metrics |
| [F-CLU-068](01-cluster-membership-and-ha.md#f-clu-068) | Standby-replay follower loop | operational | cluster-wire, config |
| [F-CLU-069](01-cluster-membership-and-ha.md#f-clu-069) | Standby-replay takeover | operational | cluster-wire |
| [F-CLU-080](01-cluster-membership-and-ha.md#f-clu-080) | up:reconnect: the client reconnect window and its timeout | supporting | client-wire, config |
| [F-CLU-098](01-cluster-membership-and-ha.md#f-clu-098) | Leftover reconnect caps and cap reissue at activation | supporting | client-wire, metrics |
| [F-CLU-102](01-cluster-membership-and-ha.md#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk |
| [F-CLU-104](01-cluster-membership-and-ha.md#f-clu-104) | Reporting a rank as DAMAGED | operational | cluster-wire, metrics |
| [F-CLU-105](01-cluster-membership-and-ha.md#f-clu-105) | Respawn: re-exec as a fresh standby | operational | cluster-wire |
| [F-CLU-106](01-cluster-membership-and-ha.md#f-clu-106) | Suicide and orderly shutdown | operational | cluster-wire, admin |
| [F-CLU-107](01-cluster-membership-and-ha.md#f-clu-107) | Write error policy | operational | config, metrics |
| [F-CLU-108](01-cluster-membership-and-ha.md#f-clu-108) | Process bootstrap (ceph-mds -i <id>) *(legacy parts)* | operational | config, cluster-wire |
| [F-PROTO-021](02-client-sessions-and-protocol.md#f-proto-021) | Per-bit gated behaviour and feature names *(legacy parts)* | supporting | client-wire, admin |
| [F-PROTO-023](02-client-sessions-and-protocol.md#f-proto-023) | required_client_features enforcement and min_compat_client *(legacy parts)* | operational | client-wire, cluster-wire, admin |
| [F-PROTO-026](02-client-sessions-and-protocol.md#f-proto-026) | Client generations and candidate minimum client floors *(legacy parts)* | operational | client-wire, admin |
| [F-PROTO-029](02-client-sessions-and-protocol.md#f-proto-029) | Session open admission checks and rejection reasons *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-PROTO-034](02-client-sessions-and-protocol.md#f-proto-034) | Client metadata capture and use *(legacy parts)* | operational | client-wire, admin, on-disk |
| [F-PROTO-046](02-client-sessions-and-protocol.md#f-proto-046) | Per-session timeout metadata | operational | client-wire, admin |
| [F-PROTO-047](02-client-sessions-and-protocol.md#f-proto-047) | Eviction deferral while OSDs are laggy | operational | config, metrics |
| [F-PROTO-048](02-client-sessions-and-protocol.md#f-proto-048) | Cap-revoke non-responder eviction | operational | config, metrics |
| [F-PROTO-051](02-client-sessions-and-protocol.md#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin |
| [F-PROTO-052](02-client-sessions-and-protocol.md#f-proto-052) | OSDMap blocklist-driven session kill | supporting | client-wire, cluster-wire |
| [F-PROTO-053](02-client-sessions-and-protocol.md#f-proto-053) | Eviction of sessions with oversized metadata | operational | config, metrics |
| [F-PROTO-054](02-client-sessions-and-protocol.md#f-proto-054) | Session termination at rank shutdown | operational | client-wire, cluster-wire |
| [F-CAPS-031](03-caps-leases-and-locking.md#f-caps-031) | Late revocation warnings, health and eviction | operational | metrics, config |
| [F-CAPS-081](03-caps-leases-and-locking.md#f-caps-081) | Implicit quiescelock wrlock | extension | internal |
| [F-CAPS-095](03-caps-leases-and-locking.md#f-caps-095) | Cap and lease counters and dumps | operational | metrics, admin |
| [F-CAPS-096](03-caps-leases-and-locking.md#f-caps-096) | Lock state dumps and lock-related op events | operational | admin |
| [F-NS-007](04-namespace-and-posix-operations.md#f-ns-007) | Request kill and abort | operational | admin |
| [F-NS-020](04-namespace-and-posix-operations.md#f-ns-020) | Directory entry limits (ENOSPC) | operational | client-wire, config |
| [F-NS-034](04-namespace-and-posix-operations.md#f-ns-034) | Readdir cap-acquisition throttle | operational | config, metrics |
| [F-NS-071](04-namespace-and-posix-operations.md#f-ns-071) | Virtual xattr set/remove path (common rules, p flag) | extension | client-wire, admin |
| [F-NS-072](04-namespace-and-posix-operations.md#f-ns-072) | Layout virtual xattrs (ceph.dir.layout*, ceph.file.layout*) *(legacy parts)* | extension | client-wire, cluster-wire |
| [F-CACHE-016](05-metadata-cache-and-dirfrags.md#f-cache-016) | Write-back only at journal segment expiry | supporting | on-disk, config, admin |
| [F-CACHE-018](05-metadata-cache-and-dirfrags.md#f-cache-018) | Commit error handling and read-only mode | operational | metrics, config |
| [F-CACHE-040](05-metadata-cache-and-dirfrags.md#f-cache-040) | Aux subtrees | internal | internal |
| [F-CACHE-061](05-metadata-cache-and-dirfrags.md#f-cache-061) | Memory-based soft cache limit | operational | config, metrics |
| [F-CACHE-062](05-metadata-cache-and-dirfrags.md#f-cache-062) | Trim throttle | operational | config, metrics |
| [F-CACHE-080](05-metadata-cache-and-dirfrags.md#f-cache-080) | Per-dirfrag size limit (ENOSPC) | operational | client-wire, config |
| [F-CACHE-081](05-metadata-cache-and-dirfrags.md#f-cache-081) | Per-directory entry limit | operational | client-wire, config |
| [F-CACHE-086](05-metadata-cache-and-dirfrags.md#f-cache-086) | Fragmentation preconditions and exclusions *(legacy parts)* | internal | internal |
| [F-STORE-015](06-journal-and-storage.md#f-store-015) | Journal write-error policy (damaged or respawn) | operational | cluster-wire, metrics |
| [F-STORE-018](06-journal-and-storage.md#f-store-018) | Background journal trimming with throttle | operational | config |
| [F-STORE-020](06-journal-and-storage.md#f-store-020) | flush journal admin command | operational | admin |
| [F-STORE-040](06-journal-and-storage.md#f-store-040) | Replay error policy and the damaged rank | operational | cluster-wire, metrics |
| [F-STORE-063](06-journal-and-storage.md#f-store-063) | Scrub tag xattr on object 0 *(legacy parts)* | operational | on-disk, admin |
| [F-STORE-098](06-journal-and-storage.md#f-store-098) | OSD epoch barrier in cap messages | supporting | client-wire, admin |
| [F-STORE-100](06-journal-and-storage.md#f-store-100) | Metadata write-error policy and read-only mode | operational | client-wire, config, metrics |
| [F-STORE-101](06-journal-and-storage.md#f-store-101) | Pool requirements for CephFS pools | operational | cluster-wire |
| [F-STORE-105](06-journal-and-storage.md#f-store-105) | Journal and OpenFileTable perf counters | operational | metrics |
| [F-STORE-107](06-journal-and-storage.md#f-store-107) | Op tracker journal events | operational | admin |
| [F-STORE-108](06-journal-and-storage.md#f-store-108) | Offline journal reset and EResetJournal | operational | on-disk, admin |
| [F-STORE-109](06-journal-and-storage.md#f-store-109) | cephfs-journal-tool inspection, export/import, header and event commands | operational | on-disk, admin |
| [F-STORE-110](06-journal-and-storage.md#f-store-110) | Other offline tools over the on-disk format | operational | on-disk |
| [F-MULTI-002](07-multi-mds.md#f-multi-002) | Export preconditions and silent refusals | internal | internal |
| [F-MULTI-019](07-multi-mds.md#f-multi-019) | Export-target publication (MMDSLoadTargets, MDSMap export_targets) | supporting | cluster-wire, client-wire |
| [F-MULTI-030](07-multi-mds.md#f-multi-030) | balance_automate file-system flag | operational | cluster-wire, admin |
| [F-MULTI-034](07-multi-mds.md#f-multi-034) | Idle import return (mds_bal_idle_threshold) | operational | config |
| [F-MULTI-035](07-multi-mds.md#f-multi-035) | bal_rank_mask (balancer target ranks) | operational | cluster-wire, admin |
| [F-MULTI-036](07-multi-mds.md#f-multi-036) | Mantle Lua balancer policy | operational | cluster-wire, admin, config |
| [F-MULTI-037](07-multi-mds.md#f-multi-037) | Mantle cluster log messages | operational | admin |
| [F-MULTI-040](07-multi-mds.md#f-multi-040) | Pin inheritance and precedence resolution | extension | internal |
| [F-MULTI-041](07-multi-mds.md#f-multi-041) | Pin enforcement and aux subtrees (handle_export_pins) | extension | internal |
| [F-MULTI-042](07-multi-mds.md#f-multi-042) | Delayed pins (target rank >= max_mds) | extension | internal |
| [F-MULTI-043](07-multi-mds.md#f-multi-043) | Pins veto balancer exports | operational | internal |
| [F-MULTI-048](07-multi-mds.md#f-multi-048) | Ephemeral pin redistribution on max_mds and config changes | extension | config |
| [F-MULTI-049](07-multi-mds.md#f-multi-049) | Subtree hand-off by a stopping rank (max_mds decrease) | operational | mds-wire, admin |
| [F-SNAP-018](08-snapshots-quotas-subvolumes.md#f-snap-018) | allow_new_snaps filesystem flag | operational | cluster-wire, admin |
| [F-SNAP-019](08-snapshots-quotas-subvolumes.md#f-snap-019) | Per-directory snapshot limit (mds_max_snaps_per_dir) | extension | config, client-wire |
| [F-SNAP-020](08-snapshots-quotas-subvolumes.md#f-snap-020) | Snapshot uid range (mds_snap_min_uid, mds_snap_max_uid) | operational | config |
| [F-SNAP-041](08-snapshots-quotas-subvolumes.md#f-snap-041) | Stat mismatch detection and in-place repair | internal | metrics, config |
| [F-SNAP-042](08-snapshots-quotas-subvolumes.md#f-snap-042) | Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files) | extension | client-wire, on-disk |
| [F-BG-017](09-background-services.md#f-bg-017) | Purge throttling and pacing | operational | config |
| [F-BG-022](09-background-services.md#f-bg-022) | Rank stop: export all strays to rank 0 | operational | mds-wire |
| [F-BG-023](09-background-services.md#f-bg-023) | Rank stop: drain the PurgeQueue | operational | metrics |
| [F-BG-027](09-background-services.md#f-bg-027) | dump stray admin command | operational | admin |
| [F-BG-028](09-background-services.md#f-bg-028) | scrub start command *(legacy parts)* | operational | admin |
| [F-BG-029](09-background-services.md#f-bg-029) | Scrub tags and the scrub_tag object xattr | operational | on-disk, admin |
| [F-BG-030](09-background-services.md#f-bg-030) | tag path command | operational | admin |
| [F-BG-032](09-background-services.md#f-bg-032) | Scrub concurrency throttle | operational | config |
| [F-BG-033](09-background-services.md#f-bg-033) | Incremental scrub and the force option | operational | admin, on-disk |
| [F-BG-035](09-background-services.md#f-bg-035) | Scrubbing ~mdsdir / ~mdsN and scrub_mdsdir (stray loading) *(legacy parts)* | operational | admin |
| [F-BG-036](09-background-services.md#f-bg-036) | Scrub pause, resume and abort | operational | admin, mds-wire |
| [F-BG-037](09-background-services.md#f-bg-037) | Scrub status, summary and mgr task status *(legacy parts)* | operational | admin, metrics, cluster-wire |
| [F-BG-038](09-background-services.md#f-bg-038) | Scrub counters and scrub purge_status *(legacy parts)* | operational | admin, metrics, config |
| [F-BG-039](09-background-services.md#f-bg-039) | Backtrace validation | operational | on-disk, admin |
| [F-BG-040](09-background-services.md#f-bg-040) | Backtrace repair | operational | on-disk |
| [F-BG-046](09-background-services.md#f-bg-046) | Dentry snapshot-bound check during scrub | operational | admin |
| [F-BG-047](09-background-services.md#f-bg-047) | Hard-link resolvability check | operational | admin |
| [F-BG-051](09-background-services.md#f-bg-051) | Scrub behaviour on rank failure and restart | operational | internal |
| [F-BG-052](09-background-services.md#f-bg-052) | DamageTable registry *(legacy parts)* | operational | config |
| [F-BG-053](09-background-services.md#f-bg-053) | Dirfrag load damage detection | supporting | metrics |
| [F-BG-054](09-background-services.md#f-bg-054) | Dentry snapshot-bound corruption check at load and commit | operational | config, metrics |
| [F-BG-056](09-background-services.md#f-bg-056) | Duplicate primary inode detection on dirfrag load | supporting | metrics |
| [F-BG-057](09-background-services.md#f-bg-057) | Fatal damage takes the rank down:damaged | operational | cluster-wire |
| [F-BG-059](09-background-services.md#f-bg-059) | damage ls and damage rm commands *(legacy parts)* | operational | admin |
| [F-QOS-001](10-qos-metrics-quiesce-quarantine.md#f-qos-001) | QoS admission gate for client metadata requests | operational | config, admin |
| [F-QOS-003](10-qos-metrics-quiesce-quarantine.md#f-qos-003) | QoS per-op cost table | operational | config |
| [F-QOS-005](10-qos-metrics-quiesce-quarantine.md#f-qos-005) | QoS scheduler threads and hand-off to request processing | operational | metrics |
| [F-QOS-006](10-qos-metrics-quiesce-quarantine.md#f-qos-006) | Default QoS parameters (mds_dmclock_* options) | operational | config |
| [F-QOS-007](10-qos-metrics-quiesce-quarantine.md#f-qos-007) | Per-bucket QoS overrides (qos set, qos rm) | operational | admin |
| [F-QOS-008](10-qos-metrics-quiesce-quarantine.md#f-qos-008) | QoS inspection commands and per-bucket statistics | operational | admin |
| [F-QOS-009](10-qos-metrics-quiesce-quarantine.md#f-qos-009) | QoS bucket state lifecycle tied to sessions | internal | internal |
| [F-QOS-010](10-qos-metrics-quiesce-quarantine.md#f-qos-010) | QoS enable, disable and drain on rank state and option changes | operational | config |
| [F-QOS-016](10-qos-metrics-quiesce-quarantine.md#f-qos-016) | Subvolume I/O metrics (type 16) ingestion and path resolution | operational | client-wire, metrics |
| [F-QOS-020](10-qos-metrics-quiesce-quarantine.md#f-qos-020) | Client session lifecycle in metrics (REFRESH and REMOVE) | operational | mds-wire, metrics |
| [F-QOS-022](10-qos-metrics-quiesce-quarantine.md#f-qos-022) | Rank telemetry: CPU usage and open requests (mds_rank_perf) | operational | metrics, mds-wire |
| [F-QOS-023](10-qos-metrics-quiesce-quarantine.md#f-qos-023) | Subvolume quota_bytes and used_bytes reporting | operational | metrics |
| [F-QOS-024](10-qos-metrics-quiesce-quarantine.md#f-qos-024) | Per-client labeled metric counters (mds_client_metrics) | operational | metrics |
| [F-QOS-025](10-qos-metrics-quiesce-quarantine.md#f-qos-025) | Subvolume sliding-window metrics (mds_subvolume_metrics) | operational | metrics, mds-wire, config |
| [F-QOS-026](10-qos-metrics-quiesce-quarantine.md#f-qos-026) | Eviction of idle subvolume metrics | operational | metrics |
| [F-QOS-027](10-qos-metrics-quiesce-quarantine.md#f-qos-027) | Labeled metric counter naming and export | operational | metrics, admin |
| [F-QOS-034](10-qos-metrics-quiesce-quarantine.md#f-qos-034) | Quiesce sets and request operations | extension | admin |
| [F-QOS-035](10-qos-metrics-quiesce-quarantine.md#f-qos-035) | Quiesce set and member state machine | extension | admin |
| [F-QOS-036](10-qos-metrics-quiesce-quarantine.md#f-qos-036) | Quiesce timeout and expiration | extension | admin |
| [F-QOS-037](10-qos-metrics-quiesce-quarantine.md#f-qos-037) | Quiesce await semantics | extension | admin |
| [F-QOS-038](10-qos-metrics-quiesce-quarantine.md#f-qos-038) | Quiesce if_version optimistic concurrency | extension | admin |
| [F-QOS-039](10-qos-metrics-quiesce-quarantine.md#f-qos-039) | Quiesce root URI sanitization | extension | admin |
| [F-QOS-042](10-qos-metrics-quiesce-quarantine.md#f-qos-042) | Quiesce on degraded file systems and non-active ranks | operational | admin |
| [F-QOS-051](10-qos-metrics-quiesce-quarantine.md#f-qos-051) | Quiesce dispatch throttle and heartbeat resets | operational | config |
| [F-QOS-052](10-qos-metrics-quiesce-quarantine.md#f-qos-052) | ceph.quiesce.block vxattr | extension | client-wire, on-disk |
| [F-QOS-053](10-qos-metrics-quiesce-quarantine.md#f-qos-053) | quiesce db command | extension | admin |
| [F-QOS-057](10-qos-metrics-quiesce-quarantine.md#f-qos-057) | Quiesce observability: counters and op events | operational | metrics, admin |
| [F-QOS-058](10-qos-metrics-quiesce-quarantine.md#f-qos-058) | quarantine enable and disable commands | extension | admin |
| [F-QOS-062](10-qos-metrics-quiesce-quarantine.md#f-qos-062) | Per-subvolume quarantine operation registry | internal | admin |
| [F-QOS-066](10-qos-metrics-quiesce-quarantine.md#f-qos-066) | Subtree export refused inside a quarantined subvolume | internal | internal |
| [F-QOS-069](10-qos-metrics-quiesce-quarantine.md#f-qos-069) | MDS auth cap flags q and Q (quarantine access) | extension | cluster-wire, admin |
| [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) | Peer authorizer verification and caps loading at connection accept *(legacy parts)* | supporting | client-wire, cluster-wire |
| [F-ADM-003](11-security-admin-and-observability.md#f-adm-003) | Authentication method none (and Kerberos) means allow-all | operational | config |
| [F-ADM-005](11-security-admin-and-observability.md#f-adm-005) | Daemon key rotation commands (rotate-key, wipe-rotating-secrets) | operational | admin |
| [F-ADM-006](11-security-admin-and-observability.md#f-adm-006) | MDS cap string grammar, parser and canonical forms | operational | admin, cluster-wire |
| [F-ADM-011](11-security-admin-and-observability.md#f-adm-011) | root_squash grants | extension | client-wire, admin |
| [F-ADM-012](11-security-admin-and-observability.md#f-adm-012) | p flag: permission to set ceph.* vxattrs and non-default layouts | extension | client-wire, admin |
| [F-ADM-015](11-security-admin-and-observability.md#f-adm-015) | q/Q flags: access to quarantined subtrees | extension | client-wire, admin |
| [F-ADM-016](11-security-admin-and-observability.md#f-adm-016) | network restriction | extension | admin |
| [F-ADM-023](11-security-admin-and-observability.md#f-adm-023) | ceph fs authorize: cap generation and merge on the monitor | operational | admin, cluster-wire |
| [F-ADM-024](11-security-admin-and-observability.md#f-adm-024) | mgr/volumes subvolume authorize, deauthorize and authorized_list | operational | client-wire, cluster-wire, admin |
| [F-ADM-026](11-security-admin-and-observability.md#f-adm-026) | Local admin socket transport *(legacy parts)* | operational | admin, config |
| [F-ADM-027](11-security-admin-and-observability.md#f-adm-027) | Remote tell transport (MCommand / MCommandReply) *(legacy parts)* | operational | admin, client-wire |
| [F-ADM-028](11-security-admin-and-observability.md#f-adm-028) | Tell authorization: allow * required | operational | admin |
| [F-ADM-029](11-security-admin-and-observability.md#f-adm-029) | Command self-description (get_command_descriptions, help) | operational | admin |
| [F-ADM-030](11-security-admin-and-observability.md#f-adm-030) | Argument validation, output formats, output-file and return-code conventions | operational | admin |
| [F-ADM-031](11-security-admin-and-observability.md#f-adm-031) | Client-side target resolution for tell (libcephfs mds_command) | operational | admin, cluster-wire |
| [F-ADM-032](11-security-admin-and-observability.md#f-adm-032) | status command | operational | admin |
| [F-ADM-033](11-security-admin-and-observability.md#f-adm-033) | Process control commands: exit, respawn, lockup | operational | admin |
| [F-ADM-034](11-security-admin-and-observability.md#f-adm-034) | heap and cpu_profiler commands | operational | admin |
| [F-ADM-035](11-security-admin-and-observability.md#f-adm-035) | Op tracker dumps (ops, dump_ops_in_flight, blocked and historic ops) | operational | admin, config |
| [F-ADM-037](11-security-admin-and-observability.md#f-adm-037) | session ls / client ls | operational | admin |
| [F-ADM-038](11-security-admin-and-observability.md#f-adm-038) | session evict / client evict / session kill and the session filter grammar | operational | admin, cluster-wire |
| [F-ADM-039](11-security-admin-and-observability.md#f-adm-039) | session config / client config timeout | operational | admin |
| [F-ADM-040](11-security-admin-and-observability.md#f-adm-040) | flush journal | operational | admin, on-disk |
| [F-ADM-042](11-security-admin-and-observability.md#f-adm-042) | force_readonly | operational | admin, client-wire |
| [F-ADM-044](11-security-admin-and-observability.md#f-adm-044) | cache drop | operational | admin, client-wire |
| [F-ADM-045](11-security-admin-and-observability.md#f-adm-045) | cache status and dump cache | operational | admin, config |
| [F-ADM-047](11-security-admin-and-observability.md#f-adm-047) | dump stray | operational | admin |
| [F-ADM-048](11-security-admin-and-observability.md#f-adm-048) | Scrub control commands (scrub start, abort, pause, resume, status, purge_status) *(legacy parts)* | operational | admin, mds-wire |
| [F-ADM-050](11-security-admin-and-observability.md#f-adm-050) | damage ls and damage rm | operational | admin |
| [F-ADM-053](11-security-admin-and-observability.md#f-adm-053) | lock path and quiesce path test hooks | operational | admin |
| [F-ADM-054](11-security-admin-and-observability.md#f-adm-054) | quiesce db command | extension | admin |
| [F-ADM-055](11-security-admin-and-observability.md#f-adm-055) | quarantine enable / disable command | extension | admin |
| [F-ADM-056](11-security-admin-and-observability.md#f-adm-056) | QoS commands (dump qos, qos set, qos rm, qos get) | operational | admin |
| [F-ADM-057](11-security-admin-and-observability.md#f-adm-057) | Generic process commands: perf, counter, config, log, injectargs *(legacy parts)* | operational | admin, metrics, config |
| [F-ADM-060](11-security-admin-and-observability.md#f-adm-060) | MDS option catalogue and option metadata | operational | config |
| [F-ADM-061](11-security-admin-and-observability.md#f-adm-061) | Runtime config observer and per-subsystem fan-out | operational | config |
| [F-ADM-062](11-security-admin-and-observability.md#f-adm-062) | Config delivery from the monitor config DB and reporting to the mgr *(legacy parts)* | operational | config, cluster-wire |
| [F-ADM-063](11-security-admin-and-observability.md#f-adm-063) | Option apply-class anomalies *(legacy parts)* | internal | config |
| [F-ADM-066](11-security-admin-and-observability.md#f-adm-066) | Unlabeled MDS perf counter loggers | operational | metrics |
| [F-ADM-067](11-security-admin-and-observability.md#f-adm-067) | Counter reporting to the mgr and the names mgr modules read | operational | metrics, cluster-wire |
| [F-ADM-069](11-security-admin-and-observability.md#f-adm-069) | Admin-socket counter dump and schema JSON format | operational | admin, metrics |
| [F-ADM-070](11-security-admin-and-observability.md#f-adm-070) | Prometheus export through ceph-exporter | operational | admin, metrics |
| [F-ADM-072](11-security-admin-and-observability.md#f-adm-072) | Grafana dashboards and Prometheus alerts shipped with Ceph | operational | metrics |
| [F-ADM-073](11-security-admin-and-observability.md#f-adm-073) | Health metric pipeline (MDSHealth in beacons to monitor checks) | operational | cluster-wire, metrics |
| [F-ADM-074](11-security-admin-and-observability.md#f-adm-074) | MDS_TRIM: behind on trimming | operational | metrics |
| [F-ADM-075](11-security-admin-and-observability.md#f-adm-075) | Client-behaviour warnings: late cap release, cache-pressure recall, oldest tid | operational | metrics |
| [F-ADM-076](11-security-admin-and-observability.md#f-adm-076) | MDS_SLOW_REQUEST and slow-request detection | operational | metrics, config |
| [F-ADM-077](11-security-admin-and-observability.md#f-adm-077) | MDS_SLOW_METADATA_IO and slow RADOS IO detection | operational | metrics |
| [F-ADM-078](11-security-admin-and-observability.md#f-adm-078) | MDS_DAMAGE and MDS_READ_ONLY state warnings | operational | metrics |
| [F-ADM-079](11-security-admin-and-observability.md#f-adm-079) | MDS_CACHE_OVERSIZED | operational | metrics |
| [F-ADM-080](11-security-admin-and-observability.md#f-adm-080) | MDS_CLIENTS_LAGGY: evictions deferred because of laggy OSDs | operational | metrics, config |
| [F-ADM-081](11-security-admin-and-observability.md#f-adm-081) | MDS_ESTIMATED_REPLAY_TIME | operational | metrics, admin |
| [F-ADM-082](11-security-admin-and-observability.md#f-adm-082) | Monitor-derived file-system health checks *(legacy parts)* | operational | metrics |
| [F-ADM-083](11-security-admin-and-observability.md#f-adm-083) | Health-gated monitor commands | operational | metrics, admin |
| [F-ADM-084](11-security-admin-and-observability.md#f-adm-084) | Cluster log (clog) messages | operational | metrics, cluster-wire |
| [F-ADM-085](11-security-admin-and-observability.md#f-adm-085) | Scrub task status in the mgr service map | operational | metrics, cluster-wire |
| [F-ADM-086](11-security-admin-and-observability.md#f-adm-086) | In-memory log dump on extraordinary events | operational | config |
| [F-ADM-088](11-security-admin-and-observability.md#f-adm-088) | mgr/volumes: volume lifecycle (fs volume create, rm, rename, ls, info) | operational | client-wire, cluster-wire |
| [F-ADM-089](11-security-admin-and-observability.md#f-adm-089) | mgr/volumes: in-filesystem layout, .meta files, versions, user metadata, open side effects *(legacy parts)* | operational | client-wire |
| [F-ADM-090](11-security-admin-and-observability.md#f-adm-090) | mgr/volumes: subvolume groups | operational | client-wire |
| [F-ADM-091](11-security-admin-and-observability.md#f-adm-091) | mgr/volumes: subvolume create | operational | client-wire |
| [F-ADM-092](11-security-admin-and-observability.md#f-adm-092) | mgr/volumes: removal, trash, snapshot retention and asynchronous purge | operational | client-wire |
| [F-ADM-093](11-security-admin-and-observability.md#f-adm-093) | mgr/volumes: subvolume resize via quota | operational | client-wire |
| [F-ADM-094](11-security-admin-and-observability.md#f-adm-094) | mgr/volumes: subvolume and group pinning | operational | client-wire, admin |
| [F-ADM-095](11-security-admin-and-observability.md#f-adm-095) | mgr/volumes: charmap (case sensitivity, normalization, encoding) | operational | client-wire |
| [F-ADM-096](11-security-admin-and-observability.md#f-adm-096) | mgr/volumes and mgr/nfs: earmark and enctag xattrs | operational | client-wire |
| [F-ADM-097](11-security-admin-and-observability.md#f-adm-097) | mgr/volumes: subvolume snapshots | operational | client-wire |
| [F-ADM-098](11-security-admin-and-observability.md#f-adm-098) | mgr/volumes: snapshot visibility | operational | client-wire |
| [F-ADM-099](11-security-admin-and-observability.md#f-adm-099) | mgr/volumes: snapshot clone | operational | client-wire |
| [F-ADM-100](11-security-admin-and-observability.md#f-adm-100) | mgr/volumes: fs subvolume evict | operational | admin |
| [F-ADM-101](11-security-admin-and-observability.md#f-adm-101) | mgr/volumes: fs quiesce | operational | admin |
| [F-ADM-102](11-security-admin-and-observability.md#f-adm-102) | mgr/volumes: subvolume quarantine | operational | admin |
| [F-ADM-103](11-security-admin-and-observability.md#f-adm-103) | mgr/snap_schedule: scheduled snapshots and retention *(legacy parts)* | operational | client-wire, on-disk, config |
| [F-ADM-104](11-security-admin-and-observability.md#f-adm-104) | mgr/mirroring: enable, peers, bootstrap and directory distribution | operational | client-wire, on-disk, cluster-wire |
| [F-ADM-106](11-security-admin-and-observability.md#f-adm-106) | mgr/stats: fs perf stats (and cephfs-top) | operational | cluster-wire, admin |
| [F-ADM-107](11-security-admin-and-observability.md#f-adm-107) | mgr/status: fs status | operational | metrics |
| [F-ADM-108](11-security-admin-and-observability.md#f-adm-108) | mgr/dashboard: CephFS pages | operational | admin, client-wire, metrics |
| [F-ADM-113](11-security-admin-and-observability.md#f-adm-113) | cephadm MDS deployment | operational | config, cluster-wire |
| [F-ADM-114](11-security-admin-and-observability.md#f-adm-114) | cephadm MDS upgrade choreography | operational | cluster-wire |
| [F-ADM-115](11-security-admin-and-observability.md#f-adm-115) | Rook operator: MDS pods, command line, keyring, config and probes | operational | config, cluster-wire |
| [F-ADM-119](11-security-admin-and-observability.md#f-adm-119) | Offline journal and table tools (cephfs-journal-tool, cephfs-table-tool) | operational | on-disk |
| [F-ADM-120](11-security-admin-and-observability.md#f-adm-120) | Offline data and metadata repair tools (cephfs-data-scan, cephfs-meta-injection, first-damage.py) | operational | on-disk |

## mds-ranks

Other MDS ranks of the same filesystem. 139 features.

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-002](01-cluster-membership-and-ha.md#f-clu-002) | MDSMap: per-file-system map and its wire encoding *(legacy parts)* | supporting | cluster-wire, client-wire |
| [F-CLU-003](01-cluster-membership-and-ha.md#f-clu-003) | Epoch model: FSMap, MDSMap and standby epochs, null map | operational | cluster-wire, client-wire |
| [F-CLU-020](01-cluster-membership-and-ha.md#f-clu-020) | mds_features in the boot beacon and get_up_features *(legacy parts)* | internal | cluster-wire, on-disk |
| [F-CLU-038](01-cluster-membership-and-ha.md#f-clu-038) | Rank incarnation | supporting | cluster-wire, client-wire |
| [F-CLU-056](01-cluster-membership-and-ha.md#f-clu-056) | Rank sets and derived predicates | operational | cluster-wire |
| [F-CLU-058](01-cluster-membership-and-ha.md#f-clu-058) | Rank-level map handling and peer change detection | operational | cluster-wire |
| [F-CLU-070](01-cluster-membership-and-ha.md#f-clu-070) | Recovery set and cluster-wide resolve/rejoin barriers | supporting | mds-wire, cluster-wire |
| [F-CLU-071](01-cluster-membership-and-ha.md#f-clu-071) | Survivor reactions to a peer failing and recovering | supporting | mds-wire |
| [F-CLU-072](01-cluster-membership-and-ha.md#f-clu-072) | Deferring messages to bootstrapping peers and dropping stale peer messages | internal | mds-wire |
| [F-CLU-073](01-cluster-membership-and-ha.md#f-clu-073) | up:resolve entry and exit | supporting | mds-wire, cluster-wire, on-disk |
| [F-CLU-074](01-cluster-membership-and-ha.md#f-clu-074) | Subtree authority resolution (subtree resolves) | supporting | mds-wire |
| [F-CLU-075](01-cluster-membership-and-ha.md#f-clu-075) | Ambiguous subtree import disambiguation | supporting | mds-wire, on-disk |
| [F-CLU-076](01-cluster-membership-and-ha.md#f-clu-076) | Cross-rank two-phase update outcomes (peer resolves and ECommitted) | supporting | mds-wire, on-disk |
| [F-CLU-077](01-cluster-membership-and-ha.md#f-clu-077) | Rename-specific resolve handling | supporting | mds-wire, client-wire |
| [F-CLU-078](01-cluster-membership-and-ha.md#f-clu-078) | Snap table recovery and the SERVER_READY gate | supporting | mds-wire |
| [F-CLU-085](01-cluster-membership-and-ha.md#f-clu-085) | Rejoin message exchange: weak rejoin, strong rejoin and acks | supporting | mds-wire |
| [F-CLU-086](01-cluster-membership-and-ha.md#f-clu-086) | Recreating survivors' in-flight lock state on the recovering rank | supporting | mds-wire |
| [F-CLU-088](01-cluster-membership-and-ha.md#f-clu-088) | Moving reconnected caps between ranks (IMPORT/EXPORT during rejoin) | supporting | client-wire, mds-wire |
| [F-CLU-091](01-cluster-membership-and-ha.md#f-clu-091) | Scatterlock recovery and NEED_RECOVER | supporting | mds-wire |
| [F-CLU-093](01-cluster-membership-and-ha.md#f-clu-093) | Rejoin cache repair: invented objects, stale fragmentation, missed link/unlink | internal | mds-wire, on-disk |
| [F-CLU-109](01-cluster-membership-and-ha.md#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config |
| [F-CLU-115](01-cluster-membership-and-ha.md#f-clu-115) | Rank message validation and routing *(legacy parts)* | internal | client-wire, mds-wire |
| [F-CLU-116](01-cluster-membership-and-ha.md#f-clu-116) | Outbound messaging helpers and local delivery | internal | client-wire, mds-wire |
| [F-CLU-119](01-cluster-membership-and-ha.md#f-clu-119) | MDS-originated client requests (MDSMetaRequest) | internal | mds-wire |
| [F-PROTO-008](02-client-sessions-and-protocol.md#f-proto-008) | MDS entity name and rank identity on the wire *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-017](02-client-sessions-and-protocol.md#f-proto-017) | Client message catalog and peer-type filtering | supporting | client-wire, mds-wire |
| [F-PROTO-039](02-client-sessions-and-protocol.md#f-proto-039) | FLUSHMSG / FLUSHMSG_ACK session barrier *(legacy parts)* | supporting | client-wire |
| [F-PROTO-041](02-client-sessions-and-protocol.md#f-proto-041) | Force-opened sessions (imports, rejoin, cross-MDS rename) | supporting | client-wire, mds-wire, on-disk |
| [F-PROTO-057](02-client-sessions-and-protocol.md#f-proto-057) | Reconnect processing and outcomes *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-PROTO-107](02-client-sessions-and-protocol.md#f-proto-107) | Peer-MDS connections *(legacy parts)* | internal | mds-wire |
| [F-CAPS-035](03-caps-leases-and-locking.md#f-caps-035) | Replica caps and MInodeFileCaps | internal | mds-wire |
| [F-CAPS-036](03-caps-leases-and-locking.md#f-caps-036) | Cap migration to clients (EXPORT / IMPORT) | supporting | client-wire, mds-wire |
| [F-CAPS-061](03-caps-leases-and-locking.md#f-caps-061) | Per-object lock catalogue *(legacy parts)* | internal | mds-wire |
| [F-CAPS-062](03-caps-leases-and-locking.md#f-caps-062) | SimpleLock state machine (sm_simplelock) *(legacy parts)* | internal | internal |
| [F-CAPS-063](03-caps-leases-and-locking.md#f-caps-063) | ScatterLock state machine (nestlock, dirfragtreelock) *(legacy parts)* | internal | internal |
| [F-CAPS-064](03-caps-leases-and-locking.md#f-caps-064) | FileLock state machine (sm_filelock) | internal | internal |
| [F-CAPS-070](03-caps-leases-and-locking.md#f-caps-070) | Scatter data exchange and writebehind | internal | mds-wire, on-disk |
| [F-CAPS-071](03-caps-leases-and-locking.md#f-caps-071) | Scatter nudging | internal | config |
| [F-CAPS-072](03-caps-leases-and-locking.md#f-caps-072) | Inter-MDS lock protocol (MLock) *(legacy parts)* | internal | mds-wire |
| [F-CAPS-073](03-caps-leases-and-locking.md#f-caps-073) | Replica-initiated lock state requests | internal | mds-wire |
| [F-CAPS-074](03-caps-leases-and-locking.md#f-caps-074) | Lock state across replication, migration and rejoin | internal | mds-wire |
| [F-CAPS-080](03-caps-leases-and-locking.md#f-caps-080) | Implicit versionlock (pipelining and peer rollback) | internal | internal |
| [F-CAPS-083](03-caps-leases-and-locking.md#f-caps-083) | Remote auth pins and the rename frozen auth pin | internal | mds-wire |
| [F-CAPS-084](03-caps-leases-and-locking.md#f-caps-084) | Remote xlocks and remote wrlocks | internal | mds-wire |
| [F-CAPS-085](03-caps-leases-and-locking.md#f-caps-085) | Remote lock teardown and peer failure | internal | mds-wire |
| [F-CAPS-089](03-caps-leases-and-locking.md#f-caps-089) | Freeze versus auth-pin deadlock breaker | internal | config |
| [F-CAPS-090](03-caps-leases-and-locking.md#f-caps-090) | Lock waits on a degraded cluster | internal | internal |
| [F-NS-022](04-namespace-and-posix-operations.md#f-ns-022) | ESTALE recovery of unknown inode numbers | supporting | client-wire, mds-wire, on-disk |
| [F-NS-026](04-namespace-and-posix-operations.md#f-ns-026) | LOOKUPINO and LOOKUPHASH (resolve an inode number) | supporting | client-wire, mds-wire, on-disk |
| [F-NS-052](04-namespace-and-posix-operations.md#f-ns-052) | MDS-originated RENAME (stray reintegration and migration) | internal | mds-wire |
| [F-NS-053](04-namespace-and-posix-operations.md#f-ns-053) | Which rank serves each op (auth requirement, forward or discover) | supporting | client-wire, mds-wire, config |
| [F-NS-054](04-namespace-and-posix-operations.md#f-ns-054) | Peer requests and the LINK/UNLINK two-phase commit | internal | mds-wire, on-disk |
| [F-NS-055](04-namespace-and-posix-operations.md#f-ns-055) | RMDIR witness protocol | internal | mds-wire, on-disk |
| [F-NS-056](04-namespace-and-posix-operations.md#f-ns-056) | RENAME cross-rank two-phase commit | internal | mds-wire, on-disk |
| [F-NS-057](04-namespace-and-posix-operations.md#f-ns-057) | Inode authority and cap migration inside a cross-rank rename *(legacy parts)* | supporting | client-wire, mds-wire |
| [F-CACHE-030](05-metadata-cache-and-dirfrags.md#f-cache-030) | Subtree freeze (freeze_tree) | internal | internal |
| [F-CACHE-032](05-metadata-cache-and-dirfrags.md#f-cache-032) | Single-inode freeze, ambiguous inode auth and frozen-inode tracking | internal | internal |
| [F-CACHE-035](05-metadata-cache-and-dirfrags.md#f-cache-035) | Authority of cache objects and the auth bit | internal | internal |
| [F-CACHE-036](05-metadata-cache-and-dirfrags.md#f-cache-036) | Subtree map | internal | internal |
| [F-CACHE-037](05-metadata-cache-and-dirfrags.md#f-cache-037) | Subtree create, merge and bounded-adjust primitives | internal | internal |
| [F-CACHE-038](05-metadata-cache-and-dirfrags.md#f-cache-038) | Ambiguous authority and WAIT_SINGLEAUTH | internal | internal |
| [F-CACHE-039](05-metadata-cache-and-dirfrags.md#f-cache-039) | Subtree map maintenance on directory rename | internal | internal |
| [F-CACHE-040](05-metadata-cache-and-dirfrags.md#f-cache-040) | Aux subtrees | internal | internal |
| [F-CACHE-044](05-metadata-cache-and-dirfrags.md#f-cache-044) | Forward-or-discover for non-auth objects | supporting | client-wire, config |
| [F-CACHE-047](05-metadata-cache-and-dirfrags.md#f-cache-047) | Lookup by inode number (open_ino) | supporting | client-wire, on-disk, mds-wire |
| [F-CACHE-048](05-metadata-cache-and-dirfrags.md#f-cache-048) | open_ino peer protocol | internal | mds-wire |
| [F-CACHE-050](05-metadata-cache-and-dirfrags.md#f-cache-050) | ESTALE recovery via find_ino_peers | supporting | client-wire, mds-wire |
| [F-CACHE-051](05-metadata-cache-and-dirfrags.md#f-cache-051) | Foreign mdsdir and stray base resolution | internal | mds-wire |
| [F-CACHE-052](05-metadata-cache-and-dirfrags.md#f-cache-052) | Replica map and nonces | internal | mds-wire |
| [F-CACHE-053](05-metadata-cache-and-dirfrags.md#f-cache-053) | Discover protocol (pull replication) | internal | mds-wire |
| [F-CACHE-054](05-metadata-cache-and-dirfrags.md#f-cache-054) | Replica encodings | internal | mds-wire |
| [F-CACHE-055](05-metadata-cache-and-dirfrags.md#f-cache-055) | Replica subtrees at delegation boundaries | internal | internal |
| [F-CACHE-056](05-metadata-cache-and-dirfrags.md#f-cache-056) | Push link/unlink to replicas *(legacy parts)* | internal | mds-wire |
| [F-CACHE-057](05-metadata-cache-and-dirfrags.md#f-cache-057) | Replica refresh through lock-state payloads | internal | mds-wire |
| [F-CACHE-058](05-metadata-cache-and-dirfrags.md#f-cache-058) | Replica expiry (MCacheExpire) | internal | mds-wire |
| [F-CACHE-066](05-metadata-cache-and-dirfrags.md#f-cache-066) | Empty-subtree trimming and empty-import return | internal | internal |
| [F-CACHE-067](05-metadata-cache-and-dirfrags.md#f-cache-067) | Expiry of a stopping peer's mdsdir replicas | internal | internal |
| [F-CACHE-068](05-metadata-cache-and-dirfrags.md#f-cache-068) | Recovery-time non-auth trimming | internal | internal |
| [F-CACHE-090](05-metadata-cache-and-dirfrags.md#f-cache-090) | Replica notification of refragmentation | internal | mds-wire |
| [F-CACHE-091](05-metadata-cache-and-dirfrags.md#f-cache-091) | Fragtree propagation through dirfragtreelock | internal | mds-wire, on-disk |
| [F-CACHE-092](05-metadata-cache-and-dirfrags.md#f-cache-092) | Subtree map maintenance on split and merge | internal | internal |
| [F-CACHE-093](05-metadata-cache-and-dirfrags.md#f-cache-093) | Abort, requeue and cancellation of fragment operations | internal | internal |
| [F-CACHE-095](05-metadata-cache-and-dirfrags.md#f-cache-095) | Forced refragmentation during replay, resolve and rejoin | internal | internal |
| [F-CACHE-096](05-metadata-cache-and-dirfrags.md#f-cache-096) | Migration and fragmentation mutual exclusion | internal | internal |
| [F-STORE-032](06-journal-and-storage.md#f-store-032) | Cross-rank two-phase-commit records | internal | on-disk |
| [F-STORE-033](06-journal-and-storage.md#f-store-033) | Subtree migration records and rename subtree hints | internal | on-disk |
| [F-STORE-062](06-journal-and-storage.md#f-store-062) | Lookup by inode number through backtraces (open_ino storage path) | supporting | client-wire, on-disk |
| [F-STORE-078](06-journal-and-storage.md#f-store-078) | Table server/client two-phase commit and its journal events *(legacy parts)* | extension | mds-wire, on-disk |
| [F-STORE-079](06-journal-and-storage.md#f-store-079) | Table server placement on rank 0 | extension | cluster-wire |
| [F-STORE-080](06-journal-and-storage.md#f-store-080) | NOTIFY_PREP fan-out barrier | extension | mds-wire |
| [F-STORE-081](06-journal-and-storage.md#f-store-081) | Server-local table updates and table queries | extension | mds-wire, on-disk |
| [F-STORE-082](06-journal-and-storage.md#f-store-082) | Table 2PC recovery and message delivery rules | internal | mds-wire |
| [F-STORE-083](06-journal-and-storage.md#f-store-083) | Resolve integration of table commits | extension | mds-wire |
| [F-MULTI-001](07-multi-mds.md#f-multi-001) | Subtree authority hand-off protocol (export/import state machines) *(legacy parts)* | internal | mds-wire |
| [F-MULTI-004](07-multi-mds.md#f-multi-004) | Lock set held during migration | internal | internal |
| [F-MULTI-005](07-multi-mds.md#f-multi-005) | Prep phase: spanning-tree replication and bound opening | internal | mds-wire |
| [F-MULTI-006](07-multi-mds.md#f-multi-006) | Metadata and cap transfer encoding (MExportDir) | internal | mds-wire |
| [F-MULTI-007](07-multi-mds.md#f-multi-007) | Bystander warning and notify | internal | mds-wire |
| [F-MULTI-008](07-multi-mds.md#f-multi-008) | Cache-expire routing during migration | internal | mds-wire |
| [F-MULTI-019](07-multi-mds.md#f-multi-019) | Export-target publication (MMDSLoadTargets, MDSMap export_targets) | supporting | cluster-wire, client-wire |
| [F-MULTI-020](07-multi-mds.md#f-multi-020) | Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps) | supporting | mds-wire, client-wire, on-disk |
| [F-MULTI-022](07-multi-mds.md#f-multi-022) | Export cancellation and reversal | internal | mds-wire, on-disk |
| [F-MULTI-023](07-multi-mds.md#f-multi-023) | Rank failure during migration and resolve interplay | internal | mds-wire, on-disk |
| [F-MULTI-024](07-multi-mds.md#f-multi-024) | Freeze deadlock breaker (mds_freeze_tree_timeout) | internal | config |
| [F-MULTI-028](07-multi-mds.md#f-multi-028) | Popularity transfer on export, import, subtree change, rename and split | internal | mds-wire |
| [F-MULTI-031](07-multi-mds.md#f-multi-031) | Heartbeat epoch exchange (MHeartbeat) | operational | mds-wire, config |
| [F-MULTI-049](07-multi-mds.md#f-multi-049) | Subtree hand-off by a stopping rank (max_mds decrease) | operational | mds-wire, admin |
| [F-MULTI-050](07-multi-mds.md#f-multi-050) | Hot-directory replication (REP_ALL, MDirUpdate) | operational | mds-wire, client-wire, config |
| [F-SNAP-002](08-snapshots-quotas-subvolumes.md#f-snap-002) | Rank-0 snap table service (SnapServer) and per-rank SnapClient cache *(legacy parts)* | internal | mds-wire, on-disk |
| [F-SNAP-013](08-snapshots-quotas-subvolumes.md#f-snap-013) | Create a snapshot (MKSNAP, mkdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-014](08-snapshots-quotas-subvolumes.md#f-snap-014) | Remove a snapshot (RMSNAP, rmdir in .snap) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-015](08-snapshots-quotas-subvolumes.md#f-snap-015) | Rename a snapshot (RENAMESNAP) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-017](08-snapshots-quotas-subvolumes.md#f-snap-017) | Modify snapshot metadata (SNAP_METADATA op) | extension | client-wire, mds-wire, on-disk |
| [F-SNAP-031](08-snapshots-quotas-subvolumes.md#f-snap-031) | Realm state replication between ranks (MMDSSnapUpdate, snaplock payload) | internal | mds-wire |
| [F-SNAP-036](08-snapshots-quotas-subvolumes.md#f-snap-036) | Deferred stat propagation (dirty rstat, updated scatterlocks, tick and nudge) | internal | mds-wire, config |
| [F-SNAP-037](08-snapshots-quotas-subvolumes.md#f-snap-037) | Scatter-gather of dirstat and rstat across dirfrags and ranks | internal | mds-wire, on-disk |
| [F-BG-005](09-background-services.md#f-bg-005) | Stray dentry replication to other ranks | internal | mds-wire |
| [F-BG-011](09-background-services.md#f-bg-011) | Hard-link reintegration (silent link-merge rename) | internal | mds-wire |
| [F-BG-012](09-background-services.md#f-bg-012) | Stray migration between ranks | internal | mds-wire |
| [F-BG-022](09-background-services.md#f-bg-022) | Rank stop: export all strays to rank 0 | operational | mds-wire |
| [F-BG-036](09-background-services.md#f-bg-036) | Scrub pause, resume and abort | operational | admin, mds-wire |
| [F-BG-049](09-background-services.md#f-bg-049) | Scrub work forwarding to authoritative ranks (MMDSScrub) | internal | mds-wire |
| [F-BG-050](09-background-services.md#f-bg-050) | Scrub completion protocol (MMDSScrubStats epochs) *(legacy parts)* | internal | mds-wire |
| [F-BG-051](09-background-services.md#f-bg-051) | Scrub behaviour on rank failure and restart | operational | internal |
| [F-QOS-018](10-qos-metrics-quiesce-quarantine.md#f-qos-018) | Per-rank metrics forwarding to rank 0 (MMDSMetrics) | internal | mds-wire, config |
| [F-QOS-019](10-qos-metrics-quiesce-quarantine.md#f-qos-019) | Rank-0 ping sequence and delayed-rank detection | internal | mds-wire, config |
| [F-QOS-020](10-qos-metrics-quiesce-quarantine.md#f-qos-020) | Client session lifecycle in metrics (REFRESH and REMOVE) | operational | mds-wire, metrics |
| [F-QOS-022](10-qos-metrics-quiesce-quarantine.md#f-qos-022) | Rank telemetry: CPU usage and open requests (mds_rank_perf) | operational | metrics, mds-wire |
| [F-QOS-025](10-qos-metrics-quiesce-quarantine.md#f-qos-025) | Subvolume sliding-window metrics (mds_subvolume_metrics) | operational | metrics, mds-wire, config |
| [F-QOS-032](10-qos-metrics-quiesce-quarantine.md#f-qos-032) | Quiesce DB leader and membership from the MDSMap | operational | cluster-wire |
| [F-QOS-040](10-qos-metrics-quiesce-quarantine.md#f-qos-040) | Quiesce DB replication between ranks | internal | mds-wire |
| [F-QOS-041](10-qos-metrics-quiesce-quarantine.md#f-qos-041) | Per-rank quiesce agent | internal | internal |
| [F-QOS-049](10-qos-metrics-quiesce-quarantine.md#f-qos-049) | Quiesce and subtree migration | internal | internal |
| [F-QOS-060](10-qos-metrics-quiesce-quarantine.md#f-qos-060) | Persistent quarantine flag in inode optmetadata | extension | on-disk |
| [F-QOS-065](10-qos-metrics-quiesce-quarantine.md#f-qos-065) | Quarantine propagation to replica ranks | internal | mds-wire |
| [F-QOS-066](10-qos-metrics-quiesce-quarantine.md#f-qos-066) | Subtree export refused inside a quarantined subvolume | internal | internal |
| [F-ADM-001](11-security-admin-and-observability.md#f-adm-001) | MDS self-authentication, keyring and rotating service secrets | operational | cluster-wire, config |
| [F-ADM-004](11-security-admin-and-observability.md#f-adm-004) | Peer-type message filtering and rank address check | internal | client-wire, mds-wire, cluster-wire |
| [F-ADM-048](11-security-admin-and-observability.md#f-adm-048) | Scrub control commands (scrub start, abort, pause, resume, status, purge_status) *(legacy parts)* | operational | admin, mds-wire |

## internal

Only the MDS itself. 234 features (2 legacy entries left out).

| ID | Feature | POSIX | Exposure |
|---|---|---|---|
| [F-CLU-020](01-cluster-membership-and-ha.md#f-clu-020) | mds_features in the boot beacon and get_up_features *(legacy parts)* | internal | cluster-wire, on-disk |
| [F-CLU-023](01-cluster-membership-and-ha.md#f-clu-023) | Beacon acks and MDS-side lagginess (self-fencing deferral) | operational | cluster-wire, config |
| [F-CLU-057](01-cluster-membership-and-ha.md#f-clu-057) | Daemon-level map handling | operational | cluster-wire |
| [F-CLU-058](01-cluster-membership-and-ha.md#f-clu-058) | Rank-level map handling and peer change detection | operational | cluster-wire |
| [F-CLU-063](01-cluster-membership-and-ha.md#f-clu-063) | Boot stages for up:starting, up:replay and up:standby-replay | supporting | on-disk |
| [F-CLU-066](01-cluster-membership-and-ha.md#f-clu-066) | up:replay: journal replay of a failed rank | supporting | on-disk, metrics |
| [F-CLU-067](01-cluster-membership-and-ha.md#f-clu-067) | End of replay: journal reformat, writeable journal and the single-rank shortcut *(legacy parts)* | supporting | on-disk, cluster-wire |
| [F-CLU-076](01-cluster-membership-and-ha.md#f-clu-076) | Cross-rank two-phase update outcomes (peer resolves and ECommitted) | supporting | mds-wire, on-disk |
| [F-CLU-078](01-cluster-membership-and-ha.md#f-clu-078) | Snap table recovery and the SERVER_READY gate | supporting | mds-wire |
| [F-CLU-079](01-cluster-membership-and-ha.md#f-clu-079) | Post-resolve cache cleanup and authority bits | internal | internal |
| [F-CLU-087](01-cluster-membership-and-ha.md#f-clu-087) | Matching reconnected caps to inodes: OpenFileTable prefetch and open by number | supporting | on-disk, client-wire |
| [F-CLU-090](01-cluster-membership-and-ha.md#f-clu-090) | Choosing lock states from reconnected caps | supporting | internal |
| [F-CLU-091](01-cluster-membership-and-ha.md#f-clu-091) | Scatterlock recovery and NEED_RECOVER | supporting | mds-wire |
| [F-CLU-093](01-cluster-membership-and-ha.md#f-clu-093) | Rejoin cache repair: invented objects, stale fragmentation, missed link/unlink | internal | mds-wire, on-disk |
| [F-CLU-094](01-cluster-membership-and-ha.md#f-clu-094) | Rejoin on a single-rank file system | supporting | client-wire, on-disk |
| [F-CLU-111](01-cluster-membership-and-ha.md#f-clu-111) | Global mds_lock (FIFO fair mutex) | internal | internal |
| [F-CLU-112](01-cluster-membership-and-ha.md#f-clu-112) | Completion contexts and the rank finisher | internal | internal |
| [F-CLU-113](01-cluster-membership-and-ha.md#f-clu-113) | Progress thread and rank waiter queues | internal | internal |
| [F-CLU-114](01-cluster-membership-and-ha.md#f-clu-114) | Dispatcher chain, message priorities and ordering | internal | internal |
| [F-CLU-115](01-cluster-membership-and-ha.md#f-clu-115) | Rank message validation and routing *(legacy parts)* | internal | client-wire, mds-wire |
| [F-CLU-116](01-cluster-membership-and-ha.md#f-clu-116) | Outbound messaging helpers and local delivery | internal | client-wire, mds-wire |
| [F-CLU-117](01-cluster-membership-and-ha.md#f-clu-117) | Tick and the shared timer | internal | config |
| [F-CLU-118](01-cluster-membership-and-ha.md#f-clu-118) | Thread inventory of the daemon and its subsystems | internal | internal |
| [F-CLU-119](01-cluster-membership-and-ha.md#f-clu-119) | MDS-originated client requests (MDSMetaRequest) | internal | mds-wire |
| [F-PROTO-010](02-client-sessions-and-protocol.md#f-proto-010) | Inbound flow control (messenger dispatch throttle) *(legacy parts)* | internal | config |
| [F-PROTO-018](02-client-sessions-and-protocol.md#f-proto-018) | Decode failures and client-triggerable aborts | internal | client-wire |
| [F-PROTO-033](02-client-sessions-and-protocol.md#f-proto-033) | Unknown session op handling | internal | client-wire |
| [F-PROTO-044](02-client-sessions-and-protocol.md#f-proto-044) | Deferred stale marking for quiet sessions (mds_defer_session_stale) | operational | config |
| [F-CAPS-005](03-caps-leases-and-locking.md#f-caps-005) | Loner selection (exclusive client) | internal | internal |
| [F-CAPS-006](03-caps-leases-and-locking.md#f-caps-006) | Wanted-cap-driven stable-state policy | internal | internal |
| [F-CAPS-022](03-caps-leases-and-locking.md#f-caps-022) | File size recovery after loss of a writer | supporting | cluster-wire, on-disk |
| [F-CAPS-048](03-caps-leases-and-locking.md#f-caps-048) | Lock cache creation | internal | config |
| [F-CAPS-049](03-caps-leases-and-locking.md#f-caps-049) | Lock cache attachment | internal | internal |
| [F-CAPS-050](03-caps-leases-and-locking.md#f-caps-050) | Lock cache invalidation and lifecycle | internal | internal |
| [F-CAPS-051](03-caps-leases-and-locking.md#f-caps-051) | Forced auth pins for lock-cache requests | internal | internal |
| [F-CAPS-052](03-caps-leases-and-locking.md#f-caps-052) | Forcing async-operation flush for readers | supporting | internal |
| [F-CAPS-053](03-caps-leases-and-locking.md#f-caps-053) | Per-session inode-number preallocation | internal | on-disk, config |
| [F-CAPS-061](03-caps-leases-and-locking.md#f-caps-061) | Per-object lock catalogue *(legacy parts)* | internal | mds-wire |
| [F-CAPS-062](03-caps-leases-and-locking.md#f-caps-062) | SimpleLock state machine (sm_simplelock) *(legacy parts)* | internal | internal |
| [F-CAPS-063](03-caps-leases-and-locking.md#f-caps-063) | ScatterLock state machine (nestlock, dirfragtreelock) *(legacy parts)* | internal | internal |
| [F-CAPS-064](03-caps-leases-and-locking.md#f-caps-064) | FileLock state machine (sm_filelock) | internal | internal |
| [F-CAPS-065](03-caps-leases-and-locking.md#f-caps-065) | XSYN: rdlock while a loner keeps buffered data | internal | internal |
| [F-CAPS-066](03-caps-leases-and-locking.md#f-caps-066) | Local locks (versionlock, quiescelock) | internal | internal |
| [F-CAPS-067](03-caps-leases-and-locking.md#f-caps-067) | Gather engine (eval_gather) and cap revocation gather | supporting | internal |
| [F-CAPS-068](03-caps-leases-and-locking.md#f-caps-068) | Lock-mode grant rules (rdlock / wrlock / xlock compatibility) | supporting | internal |
| [F-CAPS-069](03-caps-leases-and-locking.md#f-caps-069) | Lock-request-driven state changes and journal nudging | internal | internal |
| [F-CAPS-070](03-caps-leases-and-locking.md#f-caps-070) | Scatter data exchange and writebehind | internal | mds-wire, on-disk |
| [F-CAPS-071](03-caps-leases-and-locking.md#f-caps-071) | Scatter nudging | internal | config |
| [F-CAPS-075](03-caps-leases-and-locking.md#f-caps-075) | acquire_locks and lock vectors | supporting | internal |
| [F-CAPS-076](03-caps-leases-and-locking.md#f-caps-076) | Per-object lock order | internal | internal |
| [F-CAPS-077](03-caps-leases-and-locking.md#f-caps-077) | Cross-object ordering rules and no upgrades | internal | internal |
| [F-CAPS-078](03-caps-leases-and-locking.md#f-caps-078) | Ancestor snaplock/policylock try-rdlocks and layout inheritance | supporting | internal |
| [F-CAPS-079](03-caps-leases-and-locking.md#f-caps-079) | Path traversal locking and path helpers | internal | internal |
| [F-CAPS-080](03-caps-leases-and-locking.md#f-caps-080) | Implicit versionlock (pipelining and peer rollback) | internal | internal |
| [F-CAPS-082](03-caps-leases-and-locking.md#f-caps-082) | Local auth pins on locked objects | internal | internal |
| [F-CAPS-087](03-caps-leases-and-locking.md#f-caps-087) | FIFO lock waiters with synchronous wake-up | internal | internal |
| [F-CAPS-088](03-caps-leases-and-locking.md#f-caps-088) | Non-blocking lock helpers | internal | internal |
| [F-CAPS-090](03-caps-leases-and-locking.md#f-caps-090) | Lock waits on a degraded cluster | internal | internal |
| [F-CAPS-091](03-caps-leases-and-locking.md#f-caps-091) | xlock lifecycle | supporting | internal |
| [F-CAPS-093](03-caps-leases-and-locking.md#f-caps-093) | Early-reply rdlock release and journal flush policy | internal | internal |
| [F-CAPS-094](03-caps-leases-and-locking.md#f-caps-094) | Lock release at safe reply and cleanup | internal | internal |
| [F-NS-004](04-namespace-and-posix-operations.md#f-ns-004) | Re-entrant handler execution (restart from the top) | internal | internal |
| [F-NS-005](04-namespace-and-posix-operations.md#f-ns-005) | Per-op path resolution and locking helpers | internal | internal |
| [F-NS-006](04-namespace-and-posix-operations.md#f-ns-006) | GETATTR/LOOKUP batching (BatchOp) | internal | config |
| [F-NS-007](04-namespace-and-posix-operations.md#f-ns-007) | Request kill and abort | operational | admin |
| [F-NS-008](04-namespace-and-posix-operations.md#f-ns-008) | Internal (MDS-originated) requests | internal | internal |
| [F-NS-040](04-namespace-and-posix-operations.md#f-ns-040) | Inode number allocation (session preallocation and InoTable) | supporting | on-disk, config |
| [F-NS-052](04-namespace-and-posix-operations.md#f-ns-052) | MDS-originated RENAME (stray reintegration and migration) | internal | mds-wire |
| [F-CACHE-012](05-metadata-cache-and-dirfrags.md#f-cache-012) | Inode projection (read-copy-update) *(legacy parts)* | internal | internal |
| [F-CACHE-013](05-metadata-cache-and-dirfrags.md#f-cache-013) | Dirfrag version sequence and fnode projection | internal | on-disk |
| [F-CACHE-015](05-metadata-cache-and-dirfrags.md#f-cache-015) | Dirty tracking on journal segments | internal | internal |
| [F-CACHE-016](05-metadata-cache-and-dirfrags.md#f-cache-016) | Write-back only at journal segment expiry | supporting | on-disk, config, admin |
| [F-CACHE-020](05-metadata-cache-and-dirfrags.md#f-cache-020) | Underwater item reconciliation after replay | internal | internal |
| [F-CACHE-024](05-metadata-cache-and-dirfrags.md#f-cache-024) | Bloom filter of trimmed names | internal | internal |
| [F-CACHE-025](05-metadata-cache-and-dirfrags.md#f-cache-025) | Null dentries | internal | internal |
| [F-CACHE-026](05-metadata-cache-and-dirfrags.md#f-cache-026) | Reference counts and upward pinning | internal | internal |
| [F-CACHE-027](05-metadata-cache-and-dirfrags.md#f-cache-027) | Inode maps and object registry | internal | internal |
| [F-CACHE-028](05-metadata-cache-and-dirfrags.md#f-cache-028) | Auth pins with one-level nested accounting | internal | internal |
| [F-CACHE-029](05-metadata-cache-and-dirfrags.md#f-cache-029) | Dirfrag freeze (freeze_dir) | internal | internal |
| [F-CACHE-030](05-metadata-cache-and-dirfrags.md#f-cache-030) | Subtree freeze (freeze_tree) | internal | internal |
| [F-CACHE-032](05-metadata-cache-and-dirfrags.md#f-cache-032) | Single-inode freeze, ambiguous inode auth and frozen-inode tracking | internal | internal |
| [F-CACHE-033](05-metadata-cache-and-dirfrags.md#f-cache-033) | Waiter queues on cache objects | internal | internal |
| [F-CACHE-034](05-metadata-cache-and-dirfrags.md#f-cache-034) | Waiter completion semantics | internal | internal |
| [F-CACHE-035](05-metadata-cache-and-dirfrags.md#f-cache-035) | Authority of cache objects and the auth bit | internal | internal |
| [F-CACHE-036](05-metadata-cache-and-dirfrags.md#f-cache-036) | Subtree map | internal | internal |
| [F-CACHE-037](05-metadata-cache-and-dirfrags.md#f-cache-037) | Subtree create, merge and bounded-adjust primitives | internal | internal |
| [F-CACHE-038](05-metadata-cache-and-dirfrags.md#f-cache-038) | Ambiguous authority and WAIT_SINGLEAUTH | internal | internal |
| [F-CACHE-039](05-metadata-cache-and-dirfrags.md#f-cache-039) | Subtree map maintenance on directory rename | internal | internal |
| [F-CACHE-049](05-metadata-cache-and-dirfrags.md#f-cache-049) | Batched dentry-key fetches for open_ino | internal | internal |
| [F-CACHE-059](05-metadata-cache-and-dirfrags.md#f-cache-059) | Dentry LRUs (lru and bottom_lru) | internal | config |
| [F-CACHE-063](05-metadata-cache-and-dirfrags.md#f-cache-063) | Trim pass and per-object removal | internal | internal |
| [F-CACHE-064](05-metadata-cache-and-dirfrags.md#f-cache-064) | Upkeep thread | internal | config |
| [F-CACHE-065](05-metadata-cache-and-dirfrags.md#f-cache-065) | Heap release to the OS | operational | config |
| [F-CACHE-069](05-metadata-cache-and-dirfrags.md#f-cache-069) | Standby-replay cache maintenance | operational | internal |
| [F-CACHE-073](05-metadata-cache-and-dirfrags.md#f-cache-073) | Boot-time opening of the base hierarchy and the open gate *(legacy parts)* | operational | on-disk, metrics |
| [F-CACHE-082](05-metadata-cache-and-dirfrags.md#f-cache-082) | Size-triggered split and split checks | internal | config |
| [F-CACHE-083](05-metadata-cache-and-dirfrags.md#f-cache-083) | Fast split | internal | config |
| [F-CACHE-084](05-metadata-cache-and-dirfrags.md#f-cache-084) | Temperature-triggered split | internal | config |
| [F-CACHE-085](05-metadata-cache-and-dirfrags.md#f-cache-085) | Size-triggered merge | internal | config |
| [F-CACHE-086](05-metadata-cache-and-dirfrags.md#f-cache-086) | Fragmentation preconditions and exclusions *(legacy parts)* | internal | internal |
| [F-CACHE-087](05-metadata-cache-and-dirfrags.md#f-cache-087) | Fragment operation (FRAGMENTDIR) | internal | on-disk |
| [F-CACHE-088](05-metadata-cache-and-dirfrags.md#f-cache-088) | Dentry repartitioning and stat-delta preservation | internal | internal |
| [F-CACHE-093](05-metadata-cache-and-dirfrags.md#f-cache-093) | Abort, requeue and cancellation of fragment operations | internal | internal |
| [F-CACHE-094](05-metadata-cache-and-dirfrags.md#f-cache-094) | Recovery of in-flight fragment operations | internal | on-disk |
| [F-CACHE-095](05-metadata-cache-and-dirfrags.md#f-cache-095) | Forced refragmentation during replay, resolve and rejoin | internal | internal |
| [F-STORE-001](06-journal-and-storage.md#f-store-001) | Journal pointer object (front/back journal inode) *(legacy parts)* | internal | on-disk |
| [F-STORE-002](06-journal-and-storage.md#f-store-002) | Journaler header object | internal | on-disk |
| [F-STORE-003](06-journal-and-storage.md#f-store-003) | Striped journal byte stream and its RADOS objects | internal | on-disk |
| [F-STORE-004](06-journal-and-storage.md#f-store-004) | RESILIENT journal entry framing *(legacy parts)* | internal | on-disk |
| [F-STORE-005](06-journal-and-storage.md#f-store-005) | LogEvent envelope and event type ids *(legacy parts)* | internal | on-disk |
| [F-STORE-007](06-journal-and-storage.md#f-store-007) | Per-rank write-ahead metadata journal with one submission order | supporting | on-disk |
| [F-STORE-008](06-journal-and-storage.md#f-store-008) | Group commit: flush and wait_for_safe | supporting | internal |
| [F-STORE-011](06-journal-and-storage.md#f-store-011) | Ordered completions and the safe position | supporting | internal |
| [F-STORE-013](06-journal-and-storage.md#f-store-013) | Journal prezeroing ahead of the write position | internal | on-disk |
| [F-STORE-014](06-journal-and-storage.md#f-store-014) | Journal header refresh | internal | on-disk |
| [F-STORE-016](06-journal-and-storage.md#f-store-016) | Log segments as units of lazy writeback (segment expiry) | supporting | on-disk |
| [F-STORE-017](06-journal-and-storage.md#f-store-017) | Segment boundaries and rotation policy *(legacy parts)* | internal | on-disk, config |
| [F-STORE-018](06-journal-and-storage.md#f-store-018) | Background journal trimming with throttle | operational | config |
| [F-STORE-019](06-journal-and-storage.md#f-store-019) | Major-group trimming and journal object deletion | internal | on-disk |
| [F-STORE-021](06-journal-and-storage.md#f-store-021) | Journal capping when a rank stops | operational | on-disk |
| [F-STORE-022](06-journal-and-storage.md#f-store-022) | EMetaBlob after-image container *(legacy parts)* | internal | on-disk |
| [F-STORE-023](06-journal-and-storage.md#f-store-023) | EMetaBlob path context and ancestor images | internal | internal |
| [F-STORE-024](06-journal-and-storage.md#f-store-024) | Snapshot COW images in the journal | extension | on-disk |
| [F-STORE-025](06-journal-and-storage.md#f-store-025) | ESubtreeMap: major boundary with the subtree map | internal | on-disk |
| [F-STORE-026](06-journal-and-storage.md#f-store-026) | EUpdate: generic mutation record | supporting | on-disk |
| [F-STORE-027](06-journal-and-storage.md#f-store-027) | EOpen: open-file journaling | supporting | on-disk |
| [F-STORE-031](06-journal-and-storage.md#f-store-031) | Destroyed inode records | internal | on-disk |
| [F-STORE-032](06-journal-and-storage.md#f-store-032) | Cross-rank two-phase-commit records | internal | on-disk |
| [F-STORE-033](06-journal-and-storage.md#f-store-033) | Subtree migration records and rename subtree hints | internal | on-disk |
| [F-STORE-034](06-journal-and-storage.md#f-store-034) | EFragment journal protocol and old dirfrag object cleanup | internal | on-disk |
| [F-STORE-035](06-journal-and-storage.md#f-store-035) | Journal open and recovery thread *(legacy parts)* | internal | on-disk |
| [F-STORE-036](06-journal-and-storage.md#f-store-036) | End-of-journal probe and torn-tail handling | supporting | on-disk |
| [F-STORE-037](06-journal-and-storage.md#f-store-037) | Journal replay pipeline | supporting | on-disk |
| [F-STORE-038](06-journal-and-storage.md#f-store-038) | Cache reconstruction from after-images | supporting | internal |
| [F-STORE-039](06-journal-and-storage.md#f-store-039) | Version-gated redo of table operations | internal | on-disk |
| [F-STORE-041](06-journal-and-storage.md#f-store-041) | Post-replay session and InoTable consistency check | internal | internal |
| [F-STORE-044](06-journal-and-storage.md#f-store-044) | Journal creation and append-only open per rank state *(legacy parts)* | operational | on-disk |
| [F-STORE-045](06-journal-and-storage.md#f-store-045) | Metadata object naming and placement *(legacy parts)* | internal | on-disk |
| [F-STORE-047](06-journal-and-storage.md#f-store-047) | Dirfrag omap object and fnode header | supporting | on-disk |
| [F-STORE-048](06-journal-and-storage.md#f-store-048) | Dentry omap keys, including snapshotted dentries | supporting | on-disk |
| [F-STORE-049](06-journal-and-storage.md#f-store-049) | Primary dentry value with embedded inode (InodeStore) | supporting | on-disk |
| [F-STORE-050](06-journal-and-storage.md#f-store-050) | Remote (hard link) dentry value *(legacy parts)* | supporting | on-disk |
| [F-STORE-051](06-journal-and-storage.md#f-store-051) | Dirfrag fetch from RADOS | supporting | on-disk, config |
| [F-STORE-052](06-journal-and-storage.md#f-store-052) | Dirfrag commit to RADOS | supporting | on-disk, config |
| [F-STORE-053](06-journal-and-storage.md#f-store-053) | Base inode objects (.inode) *(legacy parts)* | supporting | on-disk |
| [F-STORE-054](06-journal-and-storage.md#f-store-054) | Per-rank mdsdir and stray directories *(legacy parts)* | internal | on-disk |
| [F-STORE-055](06-journal-and-storage.md#f-store-055) | Object creation for a new rank (mkfs) | operational | on-disk |
| [F-STORE-056](06-journal-and-storage.md#f-store-056) | Boot-time object loading | operational | on-disk |
| [F-STORE-057](06-journal-and-storage.md#f-store-057) | Legacy on-disk formats still decoded *(legacy parts)* | internal | on-disk |
| [F-STORE-061](06-journal-and-storage.md#f-store-061) | Directory backtrace verification on fetch | internal | config, on-disk, metrics |
| [F-STORE-068](06-journal-and-storage.md#f-store-068) | MDSTable: versioned single-object table persistence *(legacy parts)* | internal | on-disk |
| [F-STORE-069](06-journal-and-storage.md#f-store-069) | InoTable object and per-rank inode ranges | supporting | on-disk |
| [F-STORE-071](06-journal-and-storage.md#f-store-071) | Session inode preallocation and refill | internal | on-disk, config |
| [F-STORE-072](06-journal-and-storage.md#f-store-072) | Release of preallocated and delegated numbers at session close (EPurged) | internal | on-disk, cluster-wire |
| [F-STORE-074](06-journal-and-storage.md#f-store-074) | InoTable replay, gap repair and the taken-number guard | internal | internal |
| [F-STORE-077](06-journal-and-storage.md#f-store-077) | SnapTable object (mds_snaptable) *(legacy parts)* | extension | on-disk |
| [F-STORE-078](06-journal-and-storage.md#f-store-078) | Table server/client two-phase commit and its journal events *(legacy parts)* | extension | mds-wire, on-disk |
| [F-STORE-081](06-journal-and-storage.md#f-store-081) | Server-local table updates and table queries | extension | mds-wire, on-disk |
| [F-STORE-084](06-journal-and-storage.md#f-store-084) | OpenFileTable tracking and Anchor records | internal | internal |
| [F-STORE-085](06-journal-and-storage.md#f-store-085) | OpenFileTable objects and commit with redo journal | internal | on-disk |
| [F-STORE-086](06-journal-and-storage.md#f-store-086) | OpenFileTable load and redo-journal recovery | internal | on-disk |
| [F-STORE-087](06-journal-and-storage.md#f-store-087) | OpenFileTable-gated segment expiry | internal | internal |
| [F-STORE-088](06-journal-and-storage.md#f-store-088) | EOpen suppression (should_log_open) | internal | internal |
| [F-STORE-089](06-journal-and-storage.md#f-store-089) | OpenFileTable prefetch during rejoin | operational | internal |
| [F-MULTI-001](07-multi-mds.md#f-multi-001) | Subtree authority hand-off protocol (export/import state machines) *(legacy parts)* | internal | mds-wire |
| [F-MULTI-002](07-multi-mds.md#f-multi-002) | Export preconditions and silent refusals | internal | internal |
| [F-MULTI-003](07-multi-mds.md#f-multi-003) | Subtree freeze and auth-pin drain | internal | internal |
| [F-MULTI-004](07-multi-mds.md#f-multi-004) | Lock set held during migration | internal | internal |
| [F-MULTI-005](07-multi-mds.md#f-multi-005) | Prep phase: spanning-tree replication and bound opening | internal | mds-wire |
| [F-MULTI-006](07-multi-mds.md#f-multi-006) | Metadata and cap transfer encoding (MExportDir) | internal | mds-wire |
| [F-MULTI-007](07-multi-mds.md#f-multi-007) | Bystander warning and notify | internal | mds-wire |
| [F-MULTI-008](07-multi-mds.md#f-multi-008) | Cache-expire routing during migration | internal | mds-wire |
| [F-MULTI-009](07-multi-mds.md#f-multi-009) | Empty import return | internal | internal |
| [F-MULTI-010](07-multi-mds.md#f-multi-010) | Export queue and in-flight size throttle | internal | config |
| [F-MULTI-011](07-multi-mds.md#f-multi-011) | Export splitting of large subtrees | internal | config |
| [F-MULTI-021](07-multi-mds.md#f-multi-021) | Migration journaling (EImportStart, EExport, EImportFinish) | supporting | on-disk |
| [F-MULTI-022](07-multi-mds.md#f-multi-022) | Export cancellation and reversal | internal | mds-wire, on-disk |
| [F-MULTI-023](07-multi-mds.md#f-multi-023) | Rank failure during migration and resolve interplay | internal | mds-wire, on-disk |
| [F-MULTI-024](07-multi-mds.md#f-multi-024) | Freeze deadlock breaker (mds_freeze_tree_timeout) | internal | config |
| [F-MULTI-025](07-multi-mds.md#f-multi-025) | Quiesce and quarantine guards on migration | internal | internal |
| [F-MULTI-026](07-multi-mds.md#f-multi-026) | Decayed popularity counters (DecayCounter, meta_load) | internal | config |
| [F-MULTI-027](07-multi-mds.md#f-multi-027) | Popularity hits on metadata operations | internal | internal |
| [F-MULTI-028](07-multi-mds.md#f-multi-028) | Popularity transfer on export, import, subtree change, rename and split | internal | mds-wire |
| [F-MULTI-029](07-multi-mds.md#f-multi-029) | Per-rank load (mds_load_t) and mds_bal_mode | internal | config, metrics |
| [F-MULTI-031](07-multi-mds.md#f-multi-031) | Heartbeat epoch exchange (MHeartbeat) | operational | mds-wire, config |
| [F-MULTI-032](07-multi-mds.md#f-multi-032) | Default rebalance target computation (prep_rebalance) | operational | config |
| [F-MULTI-033](07-multi-mds.md#f-multi-033) | Export selection (try_rebalance, find_exports) | operational | config |
| [F-MULTI-047](07-multi-mds.md#f-multi-047) | Consistent hash into a rank bucket (hash_into_rank_bucket) | internal | internal |
| [F-SNAP-011](08-snapshots-quotas-subvolumes.md#f-snap-011) | Hard-link global realm (PARENT_GLOBAL) | extension | client-wire, on-disk |
| [F-SNAP-012](08-snapshots-quotas-subvolumes.md#f-snap-012) | Past-parent snapshots on rename and unlink | extension | client-wire, on-disk |
| [F-SNAP-027](08-snapshots-quotas-subvolumes.md#f-snap-027) | Ancestor pre-COW and the COW sequence (global or subvolume-local) | internal | config |
| [F-SNAP-028](08-snapshots-quotas-subvolumes.md#f-snap-028) | Lazy trimming of deleted-snapshot metadata | internal | on-disk |
| [F-SNAP-035](08-snapshots-quotas-subvolumes.md#f-snap-035) | Eager upward stat propagation (predirty_journal_parents) | supporting | on-disk, config |
| [F-SNAP-036](08-snapshots-quotas-subvolumes.md#f-snap-036) | Deferred stat propagation (dirty rstat, updated scatterlocks, tick and nudge) | internal | mds-wire, config |
| [F-SNAP-037](08-snapshots-quotas-subvolumes.md#f-snap-037) | Scatter-gather of dirstat and rstat across dirfrags and ranks | internal | mds-wire, on-disk |
| [F-SNAP-038](08-snapshots-quotas-subvolumes.md#f-snap-038) | Journal segment expiry waits for stat folding | internal | on-disk |
| [F-SNAP-041](08-snapshots-quotas-subvolumes.md#f-snap-041) | Stat mismatch detection and in-place repair | internal | metrics, config |
| [F-BG-001](09-background-services.md#f-bg-001) | Per-rank stray directories (~mdsN/stray0..stray9) *(legacy parts)* | internal | on-disk |
| [F-BG-003](09-background-services.md#f-bg-003) | Stray directory rotation per log segment and pre-fragmentation | internal | config |
| [F-BG-007](09-background-services.md#f-bg-007) | Stray re-evaluation triggers (NOTIFYREF hooks and delayed list) | internal | config |
| [F-BG-009](09-background-services.md#f-bg-009) | Snapshot-aware purge decision (past-parent snaps, realm SnapContext) | extension | cluster-wire |
| [F-BG-010](09-background-services.md#f-bg-010) | Truncate-only purge for multiversion files (TRUNCATE_FILE) | extension | on-disk, cluster-wire |
| [F-BG-011](09-background-services.md#f-bg-011) | Hard-link reintegration (silent link-merge rename) | internal | mds-wire |
| [F-BG-013](09-background-services.md#f-bg-013) | PurgeQueue journal of PurgeItems *(legacy parts)* | internal | on-disk |
| [F-BG-014](09-background-services.md#f-bg-014) | Durable deletion intent before stray metadata removal | internal | on-disk |
| [F-BG-016](09-background-services.md#f-bg-016) | Directory object deletion (PURGE_DIR) | supporting | cluster-wire, on-disk |
| [F-BG-018](09-background-services.md#f-bg-018) | Parallel execution with contiguous expire_pos progress | internal | config |
| [F-BG-019](09-background-services.md#f-bg-019) | Purge recovery and at-least-once execution | supporting | on-disk |
| [F-BG-020](09-background-services.md#f-bg-020) | PurgeQueue error handling (read-only queue) | internal | config |
| [F-BG-021](09-background-services.md#f-bg-021) | Activation stray scan | internal | internal |
| [F-BG-031](09-background-services.md#f-bg-031) | Scrub traversal order and scheduling (ScrubStack) *(legacy parts)* | internal | internal |
| [F-BG-034](09-background-services.md#f-bg-034) | Scrub stamps on inodes and dirfrags *(legacy parts)* | internal | on-disk |
| [F-BG-041](09-background-services.md#f-bg-041) | Journal trim after repairs | internal | internal |
| [F-BG-042](09-background-services.md#f-bg-042) | InoTable "inode wrongly marked free" check and repair | operational | on-disk |
| [F-BG-043](09-background-services.md#f-bg-043) | Base inode on-disk validation (root, ~mdsN) *(legacy parts)* | internal | on-disk |
| [F-BG-044](09-background-services.md#f-bg-044) | Directory dirstat/rstat validation and repair | operational | internal |
| [F-BG-045](09-background-services.md#f-bg-045) | Dirfrag fragstat/rstat validation and repair | operational | on-disk |
| [F-BG-052](09-background-services.md#f-bg-052) | DamageTable registry *(legacy parts)* | operational | config |
| [F-BG-053](09-background-services.md#f-bg-053) | Dirfrag load damage detection | supporting | metrics |
| [F-QOS-005](10-qos-metrics-quiesce-quarantine.md#f-qos-005) | QoS scheduler threads and hand-off to request processing | operational | metrics |
| [F-QOS-009](10-qos-metrics-quiesce-quarantine.md#f-qos-009) | QoS bucket state lifecycle tied to sessions | internal | internal |
| [F-QOS-010](10-qos-metrics-quiesce-quarantine.md#f-qos-010) | QoS enable, disable and drain on rank state and option changes | operational | config |
| [F-QOS-021](10-qos-metrics-quiesce-quarantine.md#f-qos-021) | Metrics state culling on mdsmap changes | internal | metrics |
| [F-QOS-043](10-qos-metrics-quiesce-quarantine.md#f-qos-043) | Tree quiesce request (quiesce_path) | internal | internal |
| [F-QOS-047](10-qos-metrics-quiesce-quarantine.md#f-qos-047) | Quiescing inodes that appear during a quiesce | internal | internal |
| [F-QOS-048](10-qos-metrics-quiesce-quarantine.md#f-qos-048) | Quiesce and directory fragmentation | internal | internal |
| [F-QOS-050](10-qos-metrics-quiesce-quarantine.md#f-qos-050) | Quiesce release and cancel cascade | internal | internal |
| [F-QOS-060](10-qos-metrics-quiesce-quarantine.md#f-qos-060) | Persistent quarantine flag in inode optmetadata | extension | on-disk |
| [F-QOS-061](10-qos-metrics-quiesce-quarantine.md#f-qos-061) | In-progress quarantine state and operation ordering | internal | internal |
| [F-QOS-062](10-qos-metrics-quiesce-quarantine.md#f-qos-062) | Per-subvolume quarantine operation registry | internal | admin |
| [F-QOS-063](10-qos-metrics-quiesce-quarantine.md#f-qos-063) | Quarantine undo on failure | internal | internal |
| [F-QOS-068](10-qos-metrics-quiesce-quarantine.md#f-qos-068) | Quarantine enforcement predicate | internal | internal |
| [F-ADM-004](11-security-admin-and-observability.md#f-adm-004) | Peer-type message filtering and rank address check | internal | client-wire, mds-wire, cluster-wire |
| [F-ADM-065](11-security-admin-and-observability.md#f-adm-065) | Fault injection, killpoints and debug options | internal | config |
