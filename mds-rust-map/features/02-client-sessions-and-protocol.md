# Client sessions and protocol: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [wire-protocol.md](../cpp-map/02-client-protocol/wire-protocol.md) · [sessions.md](../cpp-map/02-client-protocol/sessions.md) · [kclient-expectations.md](../cpp-map/02-client-protocol/kclient-expectations.md) · [libcephfs-fuse-ganesha-expectations.md](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md) · [transport-auth-and-client-library-contract.md](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md)

## 1. Scope

This category covers everything a CephFS client sees of an MDS below the level of individual operations: the transport (msgr1, msgr2.1 crc and secure modes, compression negotiation, connection policy, replay of unacknowledged messages, peer-reset signalling, entity names and addresses), cephx on the service side (the parsing of MDS caps into the session is catalogued with the auth caps in 11), the encoding conventions, the client message catalog and the field-level encoding of every client message (requests, replies, traces, InodeStat, DirStat, LeaseStat, snap traces, extra_bl payloads, caps, cap releases, leases, snaps, quota, metrics, quarantine), the CephFS and connection feature bits with the first kernel and Ceph release of each bit and each protocol fallback, client metadata, the session life cycle (open, admission, renew, stale, autoclose, close, kill, eviction with blocklisting), the reconnect protocol after an MDS failover, the NFS-Ganesha reclaim protocol, request forwarding and retries, and the client-specific expectations of the Linux kernel client, libcephfs, ceph-fuse, NFS-Ganesha and Samba. It also records the obligations the embedded MonClient, MgrClient and Objecter place on an MDS process, because they share the MDS messenger and identity.

Sources are the fact-checked map pages wire-protocol, sessions, kclient-expectations, libcephfs-fuse-ganesha-expectations and transport-auth-and-client-library-contract, plus the staging notes of the items wire-protocol, sessions, kclient and libcephfs. Entries taken only from the transport page have empty staging_refs.

Excluded, and catalogued elsewhere: the semantics of capabilities, cap recall, dentry leases, async dirops and delegated inode numbers (03-caps-leases-and-locking); every request op handler, the completed-request table, request batching, file locks, xattrs, vxattrs, ACLs, fscrypt and inline-data semantics (04-namespace-and-posix-operations); snap realms, quotas and subvolume ids (08-snapshots-quotas-subvolumes); SessionMap persistence, ESession events and inode preallocation tables (06-journal-and-storage); metrics, the dmClock queue and quarantine (10-qos-metrics-quiesce-quarantine); MDS auth cap grammar, session admin commands, health warnings and perf counters (11-security-admin-and-observability); MDSMap, rank states and the recovery state machine (01-cluster-membership-and-ha). Where an entry here touches one of those mechanisms it describes only the wire or session part and links to the owning map page.

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

100 features: 95 used by current peers (38 of them with legacy parts) and 5 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**Transport: msgr2 (and legacy msgr1) as clients see them**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-002](#f-proto-002) | msgr2.1 crc mode for client connections *(legacy parts)* | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha, smb, mgr |
| [F-PROTO-003](#f-proto-003) | msgr2.1 secure mode (AES-128-GCM) *(legacy parts)* | operational | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-004](#f-proto-004) | msgr2 on-wire compression negotiation | internal | client-wire, config | libcephfs, ceph-fuse, ganesha |
| [F-PROTO-005](#f-proto-005) | Address vector publication and server identity checks *(legacy parts)* | supporting | client-wire, cluster-wire, config | kclient, libcephfs, ceph-fuse, ganesha, mon, osd |
| [F-PROTO-006](#f-proto-006) | Lossless stateful-server connection policy and replay of unacknowledged messages *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-007](#f-proto-007) | Peer-reset detection and session-reset signalling *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-008](#f-proto-008) | MDS entity name and rank identity on the wire *(legacy parts)* | supporting | client-wire, mds-wire | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-PROTO-009](#f-proto-009) | Keepalives and connection timeouts *(legacy parts)* | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-010](#f-proto-010) | Inbound flow control (messenger dispatch throttle) *(legacy parts)* | internal | config | internal |

**Authentication and connection identity**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-011](#f-proto-011) | cephx authorizer verification for service mds *(legacy parts)* | supporting | client-wire, cluster-wire, config | kclient, libcephfs, ceph-fuse, ganesha, mon |
| [F-PROTO-012](#f-proto-012) | Rotating mds service secrets | supporting | cluster-wire, config | mon, kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-015](#f-proto-015) | Connection-to-session binding (identity is the cephx global id) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Encoding conventions and message catalog**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-016](#f-proto-016) | Encoding primitives and versioned-struct rules | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-017](#f-proto-017) | Client message catalog and peer-type filtering | supporting | client-wire, mds-wire | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-PROTO-018](#f-proto-018) | Decode failures and client-triggerable aborts | internal | client-wire | internal |
| [F-PROTO-019](#f-proto-019) | Golden encodings and encoding test data | operational | internal | tools |

**Feature negotiation and client generations**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-020](#f-proto-020) | CephFS feature bitmap exchange at session open *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-021](#f-proto-021) | Per-bit gated behaviour and feature names *(legacy parts)* | supporting | client-wire, admin | kclient, libcephfs, ceph-fuse, ganesha, mon, admin |
| [F-PROTO-023](#f-proto-023) | required_client_features enforcement and min_compat_client *(legacy parts)* | operational | client-wire, cluster-wire, admin | mon, admin, kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-024](#f-proto-024) | Connection-level feature bits consulted by the client protocol *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-025](#f-proto-025) | Feature bit 22 numbering conflict (Ceph CHARMAP vs kernel SUBVOLUME_METRICS) *(legacy parts)* | extension | client-wire | kclient, libcephfs, ceph-fuse |
| [F-PROTO-026](#f-proto-026) | Client generations and candidate minimum client floors *(legacy parts)* | operational | client-wire, admin | kclient, libcephfs, ceph-fuse, ganesha, admin |

**Session establishment and session messages**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-027](#f-proto-027) | Session state machine | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-028](#f-proto-028) | Session open handshake (REQUEST_OPEN, journaled open, OPEN) *(legacy parts)* | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-029](#f-proto-029) | Session open admission checks and rejection reasons *(legacy parts)* | supporting | client-wire, cluster-wire | kclient, libcephfs, ceph-fuse, ganesha, mon, admin |
| [F-PROTO-030](#f-proto-030) | REJECT encoding and blocklist signalling to clients *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-031](#f-proto-031) | MClientSession encoding and the ops the MDS sends *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-032](#f-proto-032) | Session op dispatch and rank-state gating | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-033](#f-proto-033) | Unknown session op handling | internal | client-wire | internal |
| [F-PROTO-034](#f-proto-034) | Client metadata capture and use *(legacy parts)* | operational | client-wire, admin, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mgr, admin |
| [F-PROTO-037](#f-proto-037) | Duplicate REQUEST_OPEN answered with OPEN (NOTIFY_SESSION_STATE) *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-038](#f-proto-038) | FORCE_RO: read-only MDS notification | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-039](#f-proto-039) | FLUSHMSG / FLUSHMSG_ACK session barrier *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-PROTO-040](#f-proto-040) | REQUEST_FLUSH_MDLOG and the client sync paths | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-041](#f-proto-041) | Force-opened sessions (imports, rejoin, cross-MDS rename) | supporting | client-wire, mds-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |

**Session liveness, close and eviction**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-042](#f-proto-042) | Cap renewal (REQUEST_RENEWCAPS / RENEWCAPS) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-043](#f-proto-043) | Stale session detection and the STALE message | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-044](#f-proto-044) | Deferred stale marking for quiet sessions (mds_defer_session_stale) | operational | config | internal |
| [F-PROTO-045](#f-proto-045) | Session autoclose eviction | supporting | client-wire, cluster-wire, config | kclient, libcephfs, ceph-fuse, ganesha, mon, osd |
| [F-PROTO-046](#f-proto-046) | Per-session timeout metadata | operational | client-wire, admin | ganesha, libcephfs, admin |
| [F-PROTO-047](#f-proto-047) | Eviction deferral while OSDs are laggy | operational | config, metrics | osd, mon, admin |
| [F-PROTO-048](#f-proto-048) | Cap-revoke non-responder eviction | operational | config, metrics | kclient, libcephfs, mon, admin |
| [F-PROTO-049](#f-proto-049) | Client-initiated close with push-sequence handshake | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-050](#f-proto-050) | Counted messages and the session push sequence | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-051](#f-proto-051) | Client eviction: OSD blocklist, epoch barrier and session kill | supporting | client-wire, cluster-wire, admin | kclient, libcephfs, ceph-fuse, ganesha, mon, osd, admin, mgr |
| [F-PROTO-052](#f-proto-052) | OSDMap blocklist-driven session kill | supporting | client-wire, cluster-wire | osd, mon, admin, kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-053](#f-proto-053) | Eviction of sessions with oversized metadata | operational | config, metrics | kclient, libcephfs, mon, admin, ganesha |
| [F-PROTO-054](#f-proto-054) | Session termination at rank shutdown | operational | client-wire, cluster-wire | kclient, libcephfs, ceph-fuse, ganesha, mon, admin, osd |

**MDS failover reconnect and NFS-Ganesha reclaim**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-055](#f-proto-055) | Reconnect trigger and client-side ordering | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-056](#f-proto-056) | MClientReconnect encodings (v1 to v5, multi-message) *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-057](#f-proto-057) | Reconnect processing and outcomes *(legacy parts)* | supporting | client-wire, mds-wire | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-PROTO-061](#f-proto-061) | Snap realm reconnect | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-062](#f-proto-062) | Client reactions to MDSMap rank changes | supporting | cluster-wire | kclient, libcephfs, ceph-fuse, ganesha, mon |
| [F-PROTO-063](#f-proto-063) | Session reclaim (MClientReclaim with CEPH_RECLAIM_RESET) *(legacy parts)* | operational | client-wire | ganesha, libcephfs |

**Request encoding, routing and forwarding**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-065](#f-proto-065) | MClientRequest versioned head (head v1, v2, v3) *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-067](#f-proto-067) | Request message tail: stamp, gid_list, alternate_name, fscrypt fields *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-068](#f-proto-068) | Request flags REPLAY, WANT_DENTRY and ASYNC | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-069](#f-proto-069) | Request argument union per op | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-070](#f-proto-070) | Embedded cap and dentry-lease release records | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-071](#f-proto-071) | Request data segment per op | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-072](#f-proto-072) | Path encoding on the wire | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-073](#f-proto-073) | Op code numbering and the write bit | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-074](#f-proto-074) | Client-chosen inode number (head.ino) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-075](#f-proto-075) | Client-side rank selection and the hints the MDS provides | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-076](#f-proto-076) | Request forwarding (MClientRequestForward) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-077](#f-proto-077) | Retry counters and attempt limits (32BITS_RETRY_FWD) *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-078](#f-proto-078) | Request identity, tid ordering and requests without a session | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Reply encoding**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-079](#f-proto-079) | MClientReply head and error codes | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-080](#f-proto-080) | Reply trace layout | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-081](#f-proto-081) | InodeStat versioned encoding (v9, REPLY_ENCODING) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-083](#f-proto-083) | DirStat and LeaseStat records *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-084](#f-proto-084) | Caps, xattrs and inline data carried inside InodeStat *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-085](#f-proto-085) | Snap trace encoding and format selection | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-086](#f-proto-086) | Readdir and LSSNAP reply format and ordering contract *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-087](#f-proto-087) | CREATE reply: created inode and delegated inode ranges *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-088](#f-proto-088) | GETFILELOCK, GETVXATTR and FILE_BLOCKDIFF extra payloads | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-089](#f-proto-089) | Early (unsafe) and safe replies | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-090](#f-proto-090) | Reply content invariants enforced by clients | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Other client messages**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-091](#f-proto-091) | MClientCaps encoding (v14, xattrs in the middle segment) *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-092](#f-proto-092) | MClientCapRelease encoding and the OSD epoch barrier | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-093](#f-proto-093) | MClientLease encoding | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-094](#f-proto-094) | MClientSnap encoding | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-095](#f-proto-095) | MClientQuota encoding *(legacy parts)* | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-096](#f-proto-096) | MClientMetrics encoding and client send rules | operational | client-wire, metrics | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-PROTO-097](#f-proto-097) | MQuarantineDisable encoding | extension | client-wire | libcephfs, ceph-fuse, ganesha |

**Client-specific expectations (kclient, libcephfs, ceph-fuse, NFS-Ganesha, Samba)**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-098](#f-proto-098) | Mount flow requirements | supporting | client-wire, cluster-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-PROTO-099](#f-proto-099) | Inode-number addressed access (NFS file handles, #ino paths, LOOKUPINO) | supporting | client-wire | kclient, libcephfs, ganesha, ceph-fuse |
| [F-PROTO-100](#f-proto-100) | Inode number ranges clients accept | supporting | client-wire | kclient, ceph-fuse, libcephfs |
| [F-PROTO-101](#f-proto-101) | Shared sessions: NFS-Ganesha and Samba libcephfsd | supporting | client-wire | ganesha, smb, libcephfs |
| [F-PROTO-102](#f-proto-102) | NFS-Ganesha delegations built on caps | extension | client-wire | ganesha, libcephfs |

**Cluster client libraries sharing the MDS identity (MonClient, MgrClient, Objecter)**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-PROTO-103](#f-proto-103) | MonClient obligations *(legacy parts)* | operational | cluster-wire, config | mon |
| [F-PROTO-104](#f-proto-104) | MgrClient obligations | operational | cluster-wire, metrics | mgr |
| [F-PROTO-105](#f-proto-105) | Objecter obligations (OSD client protocol) *(legacy parts)* | supporting | cluster-wire | osd |
| [F-PROTO-106](#f-proto-106) | Fencing identity: one address vector for clients, monitors and OSDs *(legacy parts)* | supporting | cluster-wire | mon, osd |
| [F-PROTO-107](#f-proto-107) | Peer-MDS connections *(legacy parts)* | internal | mds-wire | mds-ranks |

**Legacy entries**

| ID | Feature | Legacy reasons | Relied on by |
|---|---|---|---|
| [F-PROTO-001](#f-proto-001) | msgr1 (legacy protocol) server side for client connections | msgr1 | kclient, libcephfs, ceph-fuse |
| [F-PROTO-013](#f-proto-013) | msgr1 message signatures | msgr1 | kclient |
| [F-PROTO-022](#f-proto-022) | Feature inference for clients that send no feature bits | old-clients | kclient, libcephfs, ceph-fuse |
| [F-PROTO-066](#f-proto-066) | Legacy 88-byte request head (message version below 4) | old-clients | kclient, libcephfs, ceph-fuse |
| [F-PROTO-082](#f-proto-082) | Legacy (connection-feature-conditional) reply encodings | old-clients | kclient, libcephfs, ceph-fuse |

## 3. Features

### Transport: msgr2 (and legacy msgr1) as clients see them

The messenger layer between clients and an MDS rank: protocol versions, framing, modes, the connection policy clients rely on, and how resets and identities appear on the wire. All of it lives in shared Ceph code, not in src/mds.

<a id="f-proto-002"></a>

#### F-PROTO-002 · msgr2.1 crc mode for client connections

*Frame-based messenger protocol revision 2.1 with crc32c integrity, used by C++ clients by default and by kernels mounted with ms_mode=crc or prefer-crc (mount.ceph passes prefer-crc for the new device syntax).*

Banner "ceph v2\n" + le16 16 + le64 supported + le64 required; C++ advertises REVISION_1|COMPRESSION and requires nothing, the kernel advertises and requires REVISION_1 only. Every frame starts with a 32-byte preamble (tag, segment count 1-4, four length/alignment pairs, flags, crc32c over 28 bytes); in 2.1-crc segment 1 is followed by its crc and a 13-byte epilogue with crcs for segments 2-4 is present only in multi-segment frames, with crc32c(-1, empty) = 0xFFFFFFFF for an empty segment followed by a non-empty one and 0 for absent segments. Control frames are HELLO, AUTH_REQUEST, AUTH_BAD_METHOD, AUTH_REPLY_MORE, AUTH_REQUEST_MORE, AUTH_DONE, AUTH_SIGNATURE, CLIENT_IDENT, SERVER_IDENT, IDENT_MISSING_FEATURES, SESSION_RECONNECT/RESET/RETRY/RETRY_GLOBAL/RECONNECT_OK, WAIT, KEEPALIVE2(_ACK), ACK; MESSAGE carries the 41-byte ceph_msg_header2 (seq, tid, type, priority, version, ack_seq, compat_version). The kernel always verifies crcs (it ignores nocrc on msgr2), rejects empty or aborted control frames, WAIT, KEEPALIVE2 from the server and SESSION_RESET with full=0, and requires the first MSGR2 address of SERVER_IDENT to equal the dialled MDSMap address byte for byte and MSG_AUTH in the server supported features.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, smb, mgr
- **Legacy parts** (old-clients): msgr2.0 (rev0) frame layout (17-byte epilogue), selected per connection for peers whose banner lacks REVISION_1. The 7.3 kernel advertises and requires REVISION_1, and C++ peers at this commit advertise it, so every baseline connection uses msgr2.1 framing. The C++ server requires no msgr2 feature and switches a connection to msgr2.0 (rev0) framing when the peer's banner lacks REVISION_1. REVISION_1 first appears in Ceph 14.2.11, 15.2.5 and 16.1.0, so rev0 peers are Ceph 14.2.0-14.2.10 and 15.2.0-15.2.4 clients. Evidence: `src/include/msgr.h:53 (CEPH_MSGR2_FEATURE_REVISION_1)`, `src/include/msgr.h:59-64 (CEPH_MSGR2_SUPPORTED_FEATURES, CEPH_MSGR2_REQUIRED_FEATURES)`, `src/msg/async/ProtocolV2.cc:1013-1016 (ProtocolV2::_handle_peer_banner_payload, set_is_rev1)`, `linux/include/linux/ceph/msgr.h:45-47 (CEPH_MSGR2_SUPPORTED_FEATURES, CEPH_MSGR2_REQUIRED_FEATURES)`, `v14.2.10:src/include/msgr.h:61 (CEPH_MSGR2_SUPPORTED_FEATURES 0)`, `v14.2.11:src/include/msgr.h:60 (REVISION_1)`, `v15.2.5:src/include/msgr.h:60 (REVISION_1)`.
- **Gates:** ms_bind_msgr2 (default true); ms_service_mode (default "crc secure"); kernel mount option ms_mode=crc|prefer-crc (kernels v5.11+; mount.ceph passes prefer-crc for the new device syntax)
- **Map:** [transport-auth-and-client-library-contract.md › 3.4 msgr2.0 and msgr2.1](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#34-msgr20-and-msgr21) · [transport-auth-and-client-library-contract.md › 7.2 Linux kernel client over msgr2](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#72-linux-kernel-client-over-msgr2) · [transport-auth-and-client-library-contract.md › 7.4 libcephfs, ceph-fuse and NFS-Ganesha (C++ messenger)](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#74-libcephfs-ceph-fuse-and-nfs-ganesha-c-messenger)
- **Code:** `src/msg/async/frames_v2.cc:147-175 (asm_crc_rev1)`, `src/msg/async/ProtocolV2.cc:2418-2518 (ProtocolV2::handle_client_ident)`, `src/msg/async/ProtocolV2.cc:2966-3037 (ProtocolV2::send_server_ident)`, `linux/net/ceph/messenger_v2.c:2494-2519 (process_server_ident)`, `linux/net/ceph/messenger_v2.c:2766-2825 (process_control)`
- **Notes:** msgr2.0 framing (17-byte epilogue) is used only with a peer lacking REVISION_1; kernels and C++ clients are 2.1. C++ clients offer ms_client_mode "crc secure" and the server picks the first mode in the client list it allows, so C++ clients end up in crc mode. An MDS with ms_crc_data=false writes zero segment crcs and cannot serve msgr2-crc kernel mounts (transport page 8, item 11). msgr2 has no protocol-number check.

<a id="f-proto-003"></a>

#### F-PROTO-003 · msgr2.1 secure mode (AES-128-GCM)

*Encrypted msgr2.1 frames for clients that select secure mode (kernel ms_mode=secure or prefer-secure, or a C++ client configured for it).*

After AUTH_DONE both sides derive AES-128-GCM handlers from the 64-byte connection secret carried in the CephXAuthorizeReply v2: bytes 0-15 are the key, 16-27 nonce A and 28-39 nonce B; the client receives with A and sends with B and the server does the opposite. A nonce is {le32 fixed, le64 counter} and msgr2.1 increments the 64-bit counter per encrypted unit. In 2.1-secure the first block is the preamble plus a 48-byte inline buffer with the start of segment 1 plus a 16-byte tag (96 bytes); the rest of segment 1 and segments 2-4 with a 16-byte epilogue follow as further encrypted units. Secure mode needs a cephx session key: with auth none only crc is offered and a client insisting on secure gets AUTH_BAD_METHOD. The kernel requires at least 40 and accepts at most 64 secret bytes.

- **Class:** POSIX `operational` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): msgr2.0 secure-mode nonce format and frame layout (new_nonce_format=false) for peers without REVISION_1. Every baseline peer negotiates msgr2.1, whose secure mode increments a 64-bit nonce counter. The msgr2.0 nonce format (counter in the 32-bit field, connection fault 'out of nonces' on wrap) is chosen only for peers without REVISION_1, which are Ceph 14.2.0-14.2.10 and 15.2.0-15.2.4. Evidence: `src/msg/async/ProtocolV2.cc:1908-1910 (ProtocolV2::handle_auth_done, new_nonce_format=is_rev1)`, `src/msg/async/ProtocolV2.cc:2352-2354 (ProtocolV2::finish_auth, new_nonce_format=is_rev1)`, `src/msg/async/crypto_onwire.cc:91-94 (AES128GCM_OnWireTxHandler::reset_tx_handler, out of nonces)`, `src/msg/async/crypto_onwire.cc:106-112 (AES128GCM_OnWireTxHandler::reset_tx_handler, msgr2.0 counter)`, `linux/include/linux/ceph/msgr.h:45-47 (CEPH_MSGR2_REQUIRED_FEATURES)`.
- **Gates:** ms_service_mode listing secure (default "crc secure"); kernel mount option ms_mode=secure|prefer-secure
- **Map:** [transport-auth-and-client-library-contract.md › 3.4 msgr2.0 and msgr2.1](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#34-msgr20-and-msgr21) · [transport-auth-and-client-library-contract.md › 3.6 cephx on the service side](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#36-cephx-on-the-service-side)
- **Code:** `src/msg/async/crypto_onwire.cc:280-318 (rxtx_t::create_handler_pair)`, `src/msg/async/frames_v2.cc:177-232`, `linux/net/ceph/messenger_v2.c:701-778 (setup_crypto)`
- **Notes:** msgr2.0 secure mode increments a 32-bit nonce field and faults the connection with "out of nonces" when it would wrap (transport page 8, item 13); msgr2.1 does not have that limit. Secure mode is selected only on explicit client configuration; no default client path uses it towards an MDS.

<a id="f-proto-004"></a>

#### F-PROTO-004 · msgr2 on-wire compression negotiation

*Optional COMPRESSION_REQUEST/COMPRESSION_DONE phase of msgr2; for MDS client connections it always negotiates no compression.*

C++ messengers advertise the COMPRESSION banner bit; the phase runs only if both banners carry it, so it never runs with the kernel. The server answers COMPRESSION_REQUEST with COMPRESSION_DONE whose mode comes from CompressorRegistry::get_mode, which returns COMP_NONE for every peer type except OSD (ms_osd_compress_mode, default none) and for secure mode unless ms_compress_secure is set. A compressed frame sets FRAME_EARLY_DATA_COMPRESSED in the preamble flags; such a frame on a connection without negotiated compression trips ceph_assert_always in the C++ receiver.

- **Class:** POSIX `internal` · exposure `client-wire`, `config` · relied on by libcephfs, ceph-fuse, ganesha
- **Gates:** ms_osd_compress_mode (default none); ms_compress_secure (default false)
- **Map:** [transport-auth-and-client-library-contract.md › 3.4 msgr2.0 and msgr2.1](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#34-msgr20-and-msgr21) · [transport-auth-and-client-library-contract.md › 7.4 libcephfs, ceph-fuse and NFS-Ganesha (C++ messenger)](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#74-libcephfs-ceph-fuse-and-nfs-ganesha-c-messenger)
- **Code:** `src/msg/compressor_registry.cc:111-128 (CompressorRegistry::get_mode)`, `src/msg/async/ProtocolV2.cc:3095-3135 (ProtocolV2::handle_compression_request)`, `src/msg/async/frames_v2.cc:323`
- **Notes:** C++ clients run the handshake only when the server advertises the COMPRESSION bit; a server that does not advertise it skips the phase with them (transport page 7.4).

<a id="f-proto-005"></a>

#### F-PROTO-005 · Address vector publication and server identity checks

*The MDS listens on a v2 and, by default, a v1 address per IP, publishes them through its beacon into the MDSMap, and every client checks the server identity against that map entry.*

With ms_bind_msgr1 and ms_bind_msgr2 (both default true) the address vector is [v2:ip:port/nonce, v1:ip:port/nonce] per IP, ports from ms_bind_port_min..max (6800-7568) and a random per-process nonce; clients learn it only from the MDSMap, never from the MDS. The kernel picks the first address of the type its ms_mode selects and fails the whole MDSMap decode (mount shut down) if an entry has addresses but none of that type. C++ clients connect to the first address they speak (the v2 one), require the SERVER_IDENT address vector to contain the dialled address, and match it for equality against the MDSMap address vector when handling a remote reset; the kernel requires the first MSGR2 address of SERVER_IDENT (or the msgr1 banner address, nonce included) to equal the dialled one. A restarted MDS has a new nonce, so clients see a new address in the MDSMap and open new connections. Only kernels mounted without ms_mode dial the v1 entry; with ms_bind_msgr1=false the vector holds only v2 entries.

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, osd
- **Legacy parts** (msgr1): Publication of a v1 (TYPE_LEGACY) address per IP (ms_bind_msgr1, default true) and the msgr1 banner address identity check. A 7.3 kernel mounted with ms_mode=prefer-crc takes only the first TYPE_MSGR2 address of each MDSMap entry and compares the first v2 address of SERVER_IDENT with the dialled one. C++ clients dial the first entry of the vector, which is the v2 address. The v1 entries and the kernel's msgr1 banner address check are used only by msgr1 peers: kernels mounted without ms_mode and Ceph clients 13.x and older. Evidence: `src/common/pick_address.cc:473-488 (pick_addresses)`, `src/common/options/global.yaml.in:1270-1276 (ms_bind_msgr1)`, `src/msg/async/ProtocolV1.cc:1852-1865 (ProtocolV1::send_server_banner)`, `linux/net/ceph/decode.c:96-145 (ceph_decode_entity_addrvec)`, `linux/net/ceph/messenger_v1.c:697-720 (process_banner)`, `linux/net/ceph/messenger_v2.c:2494-2519 (process_server_ident)`, `src/msg/async/AsyncMessenger.cc:847-858 (AsyncMessenger::create_connect)`.
- **Gates:** ms_bind_msgr1, ms_bind_msgr2 (default true, true); ms_bind_port_min/ms_bind_port_max (6800/7568)
- **Map:** [transport-auth-and-client-library-contract.md › 3.2 Identity, addresses and what is authenticated](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#32-identity-addresses-and-what-is-authenticated) · [transport-auth-and-client-library-contract.md › 3.1 One messenger, six peer classes, policies and throttles](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#31-one-messenger-six-peer-classes-policies-and-throttles) · [kclient-expectations.md › 3.1 Transport, identity and messenger feature bits](../cpp-map/02-client-protocol/kclient-expectations.md#31-transport-identity-and-messenger-feature-bits)
- **Code:** `src/common/pick_address.cc:473-488 (pick_addresses)`, `linux/net/ceph/decode.c:96-148 (ceph_decode_entity_addrvec)`, `src/msg/async/ProtocolV2.cc:2178-2182 (ProtocolV2::handle_server_ident)`, `src/client/Client.cc:18252 (Client::ms_handle_remote_reset)`
- **Notes:** The same address vector is what the monitor uses to find the MDS for its mdsmap subscription and what OSD blocklisting fences (F-PROTO-106).

<a id="f-proto-006"></a>

#### F-PROTO-006 · Lossless stateful-server connection policy and replay of unacknowledged messages

*Client connections never lose an acknowledged-pending message across a TCP fault: the MDS keeps sent messages until acked and replays them with their original sequence numbers when the client reconnects.*

ceph-mds installs Policy::stateful_server(0) for client peers: lossless, server (the MDS never initiates TCP to a client), standby, resetcheck, no required connection features. Each direction numbers messages at write time (++out_seq); non-lossy connections keep written messages until acknowledged (msgr1 TAG_ACK with le64 in_seq; msgr2 ack_seq piggybacked on every MESSAGE or an ACK frame). On a transport fault the server-side connection goes to STANDBY and raises no event to the MDS; when the client reconnects (msgr1 SEQ exchange, msgr2 SESSION_RECONNECT with msg_seq answered by SESSION_RECONNECT_OK(in_seq)) the unacknowledged messages are requeued at priority 255 and resent with their original sequence numbers. Output is FIFO only among equal priorities on one connection; all MClient* messages use the default priority 127 except MClientMetrics.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (msgr1): msgr1 acknowledgement (TAG_ACK with le64 in_seq) and the msgr1 SEQ exchange on reconnect. On baseline (msgr2) client connections acknowledgements travel as ack_seq in MESSAGE or ACK frames, and reconnects use SESSION_RECONNECT answered by SESSION_RECONNECT_OK. The msgr1 TAG_ACK and the msgr1 SEQ exchange are reached only by kernels mounted without ms_mode and Ceph clients 13.x and older. Evidence: `src/msg/async/ProtocolV1.cc:602-637 (ProtocolV1::handle_tag_ack)`, `src/msg/async/ProtocolV1.cc:2505 (ProtocolV1::open, CEPH_MSGR_TAG_SEQ)`, `src/msg/async/ProtocolV1.cc:2609-2623 (ProtocolV1::handle_seq)`, `src/msg/async/ProtocolV2.cc:2520-2678 (ProtocolV2::handle_reconnect)`.
- **Map:** [transport-auth-and-client-library-contract.md › 3.5 Delivery guarantees the MDS relies on](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#35-delivery-guarantees-the-mds-relies-on) · [transport-auth-and-client-library-contract.md › 3.1 One messenger, six peer classes, policies and throttles](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#31-one-messenger-six-peer-classes-policies-and-throttles) · [wire-protocol.md › 6. Concurrency and mds_lock](../cpp-map/02-client-protocol/wire-protocol.md#6-concurrency-and-mds_lock)
- **Code:** `src/ceph_mds.cc:184-191 (main)`, `src/msg/Policy.h:69-89 (Policy::stateful_server)`, `src/msg/async/ProtocolV2.cc:210-249 (ProtocolV2::requeue_sent, discard_requeued_up_to)`, `src/msg/async/ProtocolV2.cc:2520-2678 (ProtocolV2::handle_reconnect)`
- **Notes:** The kernel accepts no sequence gaps ("bad message sequence # for incoming message"); C++ receivers log gaps but drop a message with an old sequence number and, reading the code, then stop reading the connection (derived from code, unverified at runtime; transport page 8, item 3). Connection events are queued at priority 255 and can overtake messages still queued from the same connection (transport page 8, item 7). Session liveness is handled by the session layer, not by the messenger. See also F-CLU-109.

<a id="f-proto-007"></a>

#### F-PROTO-007 · Peer-reset detection and session-reset signalling

*How a client learns that the MDS side of its connection (or session) is gone, and how the MDS reacts when a client restarts its side.*

With resetcheck, a peer that reconnects after dropping its own side is detected (msgr1 connect_seq 0 against an existing value above 0; msgr2 client-cookie mismatch); the MDS messenger then discards both queues of that peer and raises ms_handle_remote_reset. When the MDS side has no record of the session it answers the reconnect with RESETSESSION (msgr1) or SESSION_RESET(full=1) (msgr2). MDSDaemon reacts to resets only for closed sessions (mark_down, clear Connection::priv); open sessions survive connection loss. Before sending CEPH_SESSION_CLOSE the MDS marks the connection disposable so a client that missed CLOSE gets a reset on reconnect; killed (evicted) sessions get mark_down without CLOSE. The kernel (mds_peer_reset) sends MClientReconnect if the rank is in up:reconnect, ignores the reset if the rank is below it or the mount is fenced, and otherwise tears the session down locally; libcephfs treats a closing session as closed, reopens an opening one, and marks an open one stale (or closed with client_reconnect_stale) after requesting a fresh OSDMap to detect blocklisting.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (msgr1): msgr1 remote-reset detection by connect_seq and the RESETSESSION reply. On msgr2 a client that restarted its side is detected by a client-cookie mismatch, and a lost server-side session is answered with SESSION_RESET(full=1). The msgr1 detection (connect_seq 0 against an existing value above 0) and the RESETSESSION reply tag are reached only by msgr1 peers. Evidence: `src/msg/async/ProtocolV1.cc:2208 (ProtocolV1::handle_connect_message_2, connect_seq 0)`, `src/msg/async/ProtocolV1.cc:2284-2307 (ProtocolV1::handle_connect_message_2, RESETSESSION)`, `src/msg/async/ProtocolV2.cc:2520-2678 (ProtocolV2::handle_reconnect)`.
- **Map:** [transport-auth-and-client-library-contract.md › 3.5 Delivery guarantees the MDS relies on](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#35-delivery-guarantees-the-mds-relies-on) · [sessions.md › 3.1 Session objects and connection binding](../cpp-map/02-client-protocol/sessions.md#31-session-objects-and-connection-binding) · [sessions.md › 7. Externally visible contract](../cpp-map/02-client-protocol/sessions.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSDaemon.cc:1175-1224 (MDSDaemon::ms_handle_reset, ms_handle_remote_reset)`, `src/mds/Server.cc:1012-1041 (Server::_session_logged)`, `linux/fs/ceph/mds_client.c:7106-7196 (mds_peer_reset)`, `src/client/Client.cc:18227-18287 (Client::ms_handle_remote_reset)`
- **Notes:** The kernel rejects SESSION_RESET(full=0) as a protocol error; the C++ server sends full=0 only for MDS peers and for its own half-established outgoing connections (transport page 8, item 15). A kernel whose session is torn down locally while the rank is active loses all caps of that session silently (kclient expectations 8).

<a id="f-proto-008"></a>

#### F-PROTO-008 · MDS entity name and rank identity on the wire

*Clients identify which rank a message came from by the MDS messenger name (mds.<rank>) or by the connection.*

The MDS messenger is named mds.-1 (MDS_RANK_NONE) before a role is held, mds.<rank> once a rank is held and mds.<gid> in standby-replay. msgr1 copies the name into every message header; msgr2 sends the name number once as the SERVER_IDENT gid and the receiver stamps every message of the connection with it. libcephfs reads the sending rank as m->get_source().num() in its session, forward, snap, cap and reply handlers and drops messages whose (rank, connection) pair matches no session; the kernel keys MDS messages by the connection and checks only the peer entity type (CEPH_ENTITY_TYPE_MDS, 0x02) in HELLO. The MDS accepts session, reconnect, reclaim, caps, cap-release and lease messages only from CEPH_ENTITY_TYPE_CLIENT (0x08) peers.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Legacy parts** (msgr1): msgr1 per-message header src (entity name copied into each ceph_msg_header on the wire). On msgr2 the name number is sent once as the IDENT gid and the receiver stamps it on every message of the connection. Carrying the entity name in every message header on the wire is msgr1 framing and is reached only by msgr1 peers. Evidence: `src/msg/async/AsyncConnection.cc:576 (AsyncConnection::send_msg, header.src)`, `src/msg/async/ProtocolV1.cc:1135 (ProtocolV1::write_message)`, `src/msg/async/ProtocolV2.cc:2187 (ProtocolV2::handle_server_ident, peer_name)`, `src/msg/async/ProtocolV2.cc:2457 (ProtocolV2::handle_client_ident, peer_name)`.
- **Map:** [wire-protocol.md › 3.1 Transport layer](../cpp-map/02-client-protocol/wire-protocol.md#31-transport-layer) · [transport-auth-and-client-library-contract.md › 3.2 Identity, addresses and what is authenticated](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#32-identity-addresses-and-what-is-authenticated)
- **Code:** `src/mds/MDSRank.cc:2343-2350 (MDSRankDispatcher::handle_mds_map)`, `src/mds/MDSDaemon.cc:738 (MDSDaemon::init)`, `src/msg/async/ProtocolV2.cc:2983-2990 (ProtocolV2::send_server_ident)`, `src/client/Client.cc:2559-2567 (Client::_get_mds_session)`
- **Notes:** Because msgr2 fixes the name number at handshake, a connection established while the messenger had another number keeps that number for its lifetime; the C++ MDS renames its messenger before it becomes reachable as a rank (transport page 8, item 32). The kernel overwrites peer_name.num with the number the server reports (wire-protocol 8). The messenger starts with a random nonce per process and is renamed on every state change. The kernel also checks that the server address and nonce reported in SERVER_IDENT equal the map entry (linux/net/ceph/messenger_v2.c:2512 (process_server_ident)). See also F-CLU-109 (messenger policies), F-PROTO-106 (fencing identity).

<a id="f-proto-009"></a>

#### F-PROTO-009 · Keepalives and connection timeouts

*Transport-level liveness between clients and the MDS: client keepalives, the server echo, and the messenger handshake and idle timeouts.*

KEEPALIVE2 (msgr1 tag 14, msgr2 frame 18) carries a timestamp and is answered with KEEPALIVE2_ACK echoing it; the MDS itself never sends keepalives, and the kernel treats KEEPALIVE/KEEPALIVE2 from a server as a protocol error. The kernel sends a keepalive or a REQUEST_RENEWCAPS to each MDS session every time its housekeeping work runs (at most 5 s apart). The C++ messenger faults a handshake that makes no progress within ms_connection_ready_timeout (10 s) and an established connection idle for ms_connection_idle_timeout (900 s).

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (msgr1): msgr1 KEEPALIVE2 tag handling. Baseline peers exchange KEEPALIVE2 and KEEPALIVE2_ACK as msgr2 frames. The msgr1 tag form (tag 14 and its ack) is reached only by msgr1 peers. Evidence: `src/msg/async/ProtocolV1.cc:540-584 (ProtocolV1::handle_keepalive2)`, `src/msg/async/ProtocolV2.cc:1692-1723 (ProtocolV2::handle_keepalive2)`.
- **Gates:** ms_connection_ready_timeout (10 s); ms_connection_idle_timeout (900 s)
- **Map:** [transport-auth-and-client-library-contract.md › 3.5 Delivery guarantees the MDS relies on](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#35-delivery-guarantees-the-mds-relies-on) · [transport-auth-and-client-library-contract.md › 7.2 Linux kernel client over msgr2](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#72-linux-kernel-client-over-msgr2)
- **Code:** `src/msg/async/ProtocolV2.cc:1692-1723 (ProtocolV2::handle_keepalive2)`, `src/msg/async/ProtocolV1.cc:517-584 (CEPH_MSGR_TAG_KEEPALIVE2)`, `src/msg/async/AsyncConnection.cc:856-883 (AsyncConnection::tick)`, `linux/fs/ceph/mds_client.c:6264-6316 (delayed_work)`

<a id="f-proto-010"></a>

#### F-PROTO-010 · Inbound flow control (messenger dispatch throttle)

*The only inbound back-pressure of the C++ MDS messenger: one byte budget shared by all connections.*

ceph_mds.cc installs no per-policy message or byte throttlers, so both protocols skip their per-connection throttles. Each message size (front + middle + data) is taken from the messenger-wide ms_dispatch_throttle_bytes budget (100 MiB) after the msgr1 header or msgr2 preamble is read; when it is exhausted the connection stops reading its socket and retries every ms_client_throttle_retry_time_interval (5000 us), so a slow dispatcher pushes back on every peer through TCP. The budget is returned after dispatch or when queued messages are discarded. Nothing bounds a connection output queue.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Legacy parts** (msgr1): ProtocolV1::throttle_dispatch_queue (throttle taken after the msgr1 header). For baseline peers the messenger-wide dispatch throttle is taken after the msgr2 preamble is read. The msgr1 throttle point (after the msgr1 message header) is reached only by msgr1 peers. Evidence: `src/msg/async/ProtocolV1.cc:741-768 (ProtocolV1::throttle_dispatch_queue)`, `src/msg/async/ProtocolV2.cc:1662-1690`.
- **Gates:** ms_dispatch_throttle_bytes (100 MiB); ms_client_throttle_retry_time_interval (5000 us)
- **Map:** [transport-auth-and-client-library-contract.md › 3.1 One messenger, six peer classes, policies and throttles](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#31-one-messenger-six-peer-classes-policies-and-throttles)
- **Code:** `src/msg/DispatchQueue.cc:41-64 (DispatchQueue::pre_dispatch, post_dispatch)`, `src/msg/async/ProtocolV1.cc:741-768 (ProtocolV1::throttle_dispatch_queue)`, `src/msg/async/ProtocolV2.cc:1662-1690`
- **Notes:** Reading the code, a dropped old-sequence message never returns its dispatch-throttle bytes (derived from code, unverified; transport page 3.3).

### Authentication and connection identity

cephx as the MDS verifies it for clients, the hand-off of MDS caps into the session, and how a connection is bound to a session.

<a id="f-proto-011"></a>

#### F-PROTO-011 · cephx authorizer verification for service mds

*Every client connection authenticates with a cephx authorizer for service type mds, verified by the MDS (through its MonClient auth-server hooks) against rotating service secrets.*

The authorizer is {u8 1, le64 global_id, le32 service_id = 2 (MDS), CephXTicketBlob {u8 1, le64 secret_id, ticket encrypted with a rotating secret}, CephXAuthorize encrypted with the ticket session key {magic, nonce, have_challenge, server_challenge + 1}}. Verification selects the keyring key for secret_id -1 or one of the held rotating secrets, decrypts the ticket, checks the global id, the default auid, allow_all false and a caps blob of exactly one string. A CEPHX_V2 peer first gets a challenge (msgr1 CHALLENGE_AUTHORIZER, msgr2 AUTH_REPLY_MORE with the raw ciphertext) and resends the authorizer with challenge + 1; the reply is CephXAuthorizeReply {nonce + 1, connection secret (64 random bytes in secure mode, empty in crc mode or msgr1)}. The service does not compare ticket expiry with its clock: a ticket is accepted while its secret_id is one of the at most three held secrets. Both key types occur: CEPH_CRYPTO_AES (1) and CEPH_CRYPTO_AES256KRB5 (2), depending on the cluster history; the kernel implements both. An empty authorizer fails (the MDS messenger requires one).

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon
- **Legacy parts** (msgr1): msgr1 authorizer exchange: CHALLENGE_AUTHORIZER/BADAUTHORIZER reply tags, the msgr1 CEPHX_V2 requirement and no-challenge path, the empty msgr1 connection secret, and the kernel BADAUTHORIZER retry. Over msgr2 the authorizer challenge travels in AUTH_REPLY_MORE and the reply carries a 64-byte connection secret in secure mode or none in crc mode. The msgr1 exchange (CHALLENGE_AUTHORIZER and BADAUTHORIZER tags, CEPHX_V2 added to the msgr1 required features by cephx_service_require_version, no challenge for peers without CEPHX_V2, the kernel's single BADAUTHORIZER retry) is reached only by kernels mounted without ms_mode and Ceph clients 13.x and older. Evidence: `src/msg/async/ProtocolV1.cc:2047-2053 (ProtocolV1::handle_connect_message_2, CEPHX_V2 required)`, `src/msg/async/ProtocolV1.cc:2070-2073 (ProtocolV1::handle_connect_message_2, skip_authorizer_challenge)`, `src/msg/async/ProtocolV1.cc:2096 (ProtocolV1::handle_connect_message_2, BADAUTHORIZER)`, `src/msg/async/ProtocolV1.cc:2107 (ProtocolV1::handle_connect_message_2, CHALLENGE_AUTHORIZER)`, `linux/net/ceph/messenger_v1.c:740-932 (process_connect)`, `linux/include/linux/ceph/ceph_features.h:220 (CEPHX_V2 advertised)`.
- **Gates:** auth_service_required (default cephx); cephx_service_require_version (default 2, adds CEPHX_V2 to msgr1 requirements)
- **Map:** [transport-auth-and-client-library-contract.md › 3.6 cephx on the service side](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#36-cephx-on-the-service-side) · [wire-protocol.md › 3.1 Transport layer](../cpp-map/02-client-protocol/wire-protocol.md#31-transport-layer) · [sessions.md › 3.1 Session objects and connection binding](../cpp-map/02-client-protocol/sessions.md#31-session-objects-and-connection-binding)
- **Code:** `src/auth/cephx/CephxProtocol.cc:451-638 (cephx_verify_authorizer)`, `src/mon/MonClient.cc:1815-1880 (MonClient::handle_auth_request)`, `src/auth/RotatingKeyRing.cc:65-90 (RotatingKeyRing::get_service_secret)`, `src/auth/Crypto.cc:554-1000 (CryptoAES256KRB5KeyHandler)`, `linux/net/ceph/auth_x.c:398-468 (ceph_x_build_authorizer)`
- **Notes:** msgr1 BADAUTHORIZER makes the kernel retry once with a fresh authorizer and fail at the second. Ticket expiry is not checked by the service (transport page 8, item 18). When the MonMap auth_epoch grows the MonClient wipes rotating secrets and client authorizers fail until new ones arrive (item 19). See also F-ADM-002 (caps loading at accept), F-PROTO-012 (rotating secrets).

<a id="f-proto-012"></a>

#### F-PROTO-012 · Rotating mds service secrets

*The MDS keeps the current set of rotating mds service secrets from the monitors; without them no client authorizer can be verified.*

Daemons of type MDS need rotating keys. The MonClient requests them with MAuth CEPHX_GET_ROTATING_KEY when fewer than three are held or the current one expired before now - min(30 s, auth_service_ticket_ttl/4), at most once per second; the monitor returns the whole RotatingSecrets map (previous, current, next) encrypted with the MDS key. The monitor seals service tickets with the current secret and ticket lifetimes are auth_service_ticket_ttl (1 h). At boot the MDS waits for rotating secrets (rotating_keys_bootstrap_timeout 30 s, max_rotating_auth_attempts 10). A MonMap whose auth_epoch increased wipes all rotating secrets and tickets.

- **Class:** POSIX `supporting` · exposure `cluster-wire`, `config` · relied on by mon, kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** auth_service_ticket_ttl (1 h); rotating_keys_bootstrap_timeout (30 s); max_rotating_auth_attempts (10)
- **Map:** [transport-auth-and-client-library-contract.md › 3.6 cephx on the service side](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#36-cephx-on-the-service-side) · [transport-auth-and-client-library-contract.md › 7.5 Monitors](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#75-monitors)
- **Code:** `src/mon/MonClient.cc:1173-1220 (MonClient::_check_auth_rotating)`, `src/auth/Auth.h:168-251 (RotatingSecrets)`, `src/auth/cephx/CephxKeyServer.cc:286-332 (KeyServer::_rotate_secret)`
- **Notes:** Boot-time waiting is described in auth-and-mds-caps 3.2 (owned by 11-security-admin-and-observability). See also F-ADM-001 (MDS self-authentication and keyring).

<a id="f-proto-015"></a>

#### F-PROTO-015 · Connection-to-session binding (identity is the cephx global id)

*Each client entity (client.<global_id>) has one session per rank; a connection is bound to it when it is accepted, and messages to a session without a connection are queued.*

ms_handle_accept (under mds_lock) looks up the session by entity_name_t(peer_type, peer_global_id); if none exists it creates a bootstrap Session in state closed that is not inserted in the SessionMap, stores it in Connection::priv, copies auth_name, socket address and name into the session, and flushes the session preopen_out_queue. Every later message finds its session through Connection::priv; for CEPH_MSG_CLIENT_SESSION MDSRank::get_session can re-point the connection to an imported session of the same entity. Sends to a session without a connection are appended to preopen_out_queue. A remount gets a new global id and therefore a new session.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.1 Transport layer](../cpp-map/02-client-protocol/wire-protocol.md#31-transport-layer) · [sessions.md › 3.1 Session objects and connection binding](../cpp-map/02-client-protocol/sessions.md#31-session-objects-and-connection-binding) · [transport-auth-and-client-library-contract.md › 3.2 Identity, addresses and what is authenticated](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#32-identity-addresses-and-what-is-authenticated)
- **Code:** `src/mds/MDSDaemon.cc:1266-1319 (MDSDaemon::ms_handle_accept)`, `src/mds/MDSRank.cc:1417-1451 (MDSRank::get_session)`, `src/mds/SessionMap.h:387-395 (Session::set_connection)`, `src/mds/MDSRank.cc:1529-1571 (MDSRank::send_message_client)`
- **Notes:** MDSRank::get_session flushes only the bootstrap session queue when it re-points a connection to an imported session; messages queued on the imported session are sent only by ms_handle_accept (code observation, race not tested; sessions 8).

### Encoding conventions and message catalog

The byte-level conventions every client message follows, the list of client message types, and what happens with malformed input.

<a id="f-proto-016"></a>

#### F-PROTO-016 · Encoding primitives and versioned-struct rules

*Little-endian Ceph encoding with ENCODE_START framing; client-facing versioned records always use struct_compat 1 and only append fields.*

Integers are little-endian; bool is one byte; string, bufferlist and vector<u8> are u32 length + bytes; containers are u32 count + elements; utime_t/ceph_timespec are u32 sec + u32 nsec; filepath is u8 1, u64 ino, string (ino 0 = relative); frag_t u32; fragtree map<frag_t, i32>; interval_set u32 count + (start, len) pairs; errors travel as errorcode32_t (Linux errno). ENCODE_START(v, compat) writes u8 struct_v, u8 struct_compat, u32 length; DECODE_START throws if compat exceeds what the decoder knows and DECODE_FINISH skips unread trailing bytes. Client-facing sub-encodings use struct_compat 1 (InodeStat 9/1, DirStat 1/1, LeaseStat 2/1, quota_info_t 1/1, openc_response_t 1/1, getvxattr 1/1, SnapRealmInfoNew 2/1, optmetadata 1/1, MDSCapAuth/MDSCapMatch 1/1, BlockDiff 1/1). Raw packed structs (ceph_mds_request_head_legacy, reply head, caps head, cap items, session head) are memcpy-encoded.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.2 Encoding primitives](../cpp-map/02-client-protocol/wire-protocol.md#32-encoding-primitives) · [transport-auth-and-client-library-contract.md › 3.10 Encoding framework and golden data](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#310-encoding-framework-and-golden-data) · [wire-protocol.md › 4. Data structures and invariants](../cpp-map/02-client-protocol/wire-protocol.md#4-data-structures-and-invariants)
- **Code:** `src/include/encoding.h:1506-1528 (ENCODE_START)`, `src/include/encoding.h:1569-1605 (DECODE_START)`, `src/include/filepath.cc:120 (filepath::encode)`, `src/include/ceph_fs_encoder.h:21-40 (WRITE_RAW_ENCODER)`
- **Notes:** The kernel requires struct_compat == 1 on InodeStat, DirStat, LeaseStat and quota_info_t and uses struct_len to skip unknown trailing fields, but parses MDSCapAuth/MDSCapMatch positionally without struct_len (F-ADM-021). The stated interoperability rule is n-2 releases between daemons and n-3 for clients (doc/dev/encoding.rst, transport page 3.10).

<a id="f-proto-017"></a>

#### F-PROTO-017 · Client message catalog and peer-type filtering

*The set of client message types, their versions, segments and directions, and which peer types the MDS accepts each from.*

Types: 22 MClientSession (both, v7/compat 1), 23 MClientReconnect (C to M, v5/compat 4, payload in the data segment), 24 MClientRequest (C to M, v6/compat 1, front + data), 25 MClientRequestForward (M to C, v1), 26 MClientReply (M to C, v1), 27 MClientReclaim and 28 MClientReclaimReply (v1), 29 MClientMetrics (C to M), 0x310 MClientCaps (both, v14, xattrs in the middle segment), 0x311 MClientLease (both), 0x312 MClientSnap, 0x313 MClientCapRelease (v2), 0x314 MClientQuota, 0x315 MQuarantineDisable. header.tid carries the request tid (requests, replies, forwards) and the cap-flush tid (MClientCaps). Session, reconnect, reclaim, caps, cap-release and lease messages are accepted only from client peers and metrics only from client peers; MClientRequest and MClientReply are not filtered because ranks also exchange them (stray reintegration, quota realm creation). The MDS sends MMDSMap only to MDS peers; the kernel nevertheless accepts MDS_MAP and FS_MAP_USER on an MDS connection.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [wire-protocol.md › 3.3 Message catalog](../cpp-map/02-client-protocol/wire-protocol.md#33-message-catalog) · [wire-protocol.md › 3.11 Other client messages](../cpp-map/02-client-protocol/wire-protocol.md#311-other-client-messages)
- **Code:** `src/include/ceph_fs.h:152-169`, `src/mds/MDSRank.cc:1191-1307 (MDSRank::is_valid_message, handle_message)`, `src/mds/MetricsHandler.cc:46-62 (MetricsHandler::ms_dispatch2)`, `src/mds/Server.cc:2730-2734 (Server::handle_client_reply)`
- **Notes:** Because CEPH_MSG_CLIENT_REPLY is not filtered by peer type, an unsafe MClientReply sent by a client reaches ceph_assert(reply->is_safe()) in Server::handle_client_reply and aborts the MDS (wire-protocol 8). MDS-originated MClientRequest/MClientReply between ranks are an internal use of the client message types.

<a id="f-proto-018"></a>

#### F-PROTO-018 · Decode failures and client-triggerable aborts

*What the C++ MDS does with malformed or unexpected client input; unmodified clients do not send any of it.*

Userspace peers decode in the messenger worker: an unknown type, a compat_version above the local HEAD_VERSION or a buffer::error makes decode_message return null and the connection faults (the kernel instead logs and drops unknown types). A ceph_assert inside decode_payload aborts the process: MClientCaps whose middle length differs from xattr_len, and MClientReply, MClientSnap or MClientQuota with trailing front bytes (the MDS decodes every known type before checking whether it accepts it). Under mds_lock: an MClientLease action other than RELEASE/RENEW/REVOKE_ACK on a held lease reaches ceph_abort(); handler-side decodes without try (initial-xattr map of new-inode ops, fscrypt last-block header of a truncating SETATTR, reconnect flock blob) throw out of the dispatcher; an owner uid/gid of -1 on a new-inode op trips ceph_assert in Server::prepare_new_inode. MKSNAP and SNAP_METADATA payloads are decoded inside try.

- **Class:** POSIX `internal` · exposure `client-wire` · relied on by internal
- **Map:** [wire-protocol.md › 3.1 Transport layer](../cpp-map/02-client-protocol/wire-protocol.md#31-transport-layer) · [wire-protocol.md › 8. Edge cases, failure handling and gotchas](../cpp-map/02-client-protocol/wire-protocol.md#8-edge-cases-failure-handling-and-gotchas) · [transport-auth-and-client-library-contract.md › 3.10 Encoding framework and golden data](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#310-encoding-framework-and-golden-data)
- **Code:** `src/msg/Message.cc:980-1031 (decode_message)`, `src/messages/MClientCaps.h:260 (MClientCaps::decode_payload)`, `src/mds/Locker.cc:4535-4537 (Locker::handle_client_lease)`, `src/mds/Server.cc:1945-1965 (Server::recover_filelocks)`, `src/mds/Server.cc:3721-3734 (Server::prepare_new_inode)`
- **Notes:** All of these are derived from code (wire-protocol 8). Sending a message type a C++ peer does not know breaks that connection (transport page 3.10).

<a id="f-proto-019"></a>

#### F-PROTO-019 · Golden encodings and encoding test data

*Tooling and reference data that pin the wire encodings: ceph-dencoder registrations, ceph-object-corpus, frame layout tables and crypto vectors.*

ceph-dencoder registers the MClient* and MMDS* messages, cephx structures (CephXAuthorize, CephXTicketBlob, RotatingSecrets, CryptoKey, ...), entity addresses and OSDMap/MonMap/MgrMap types and supports import/decode/dump_json, encode, select_test and is_deterministic. ceph-object-corpus (git submodule, not initialised in this workspace) holds archived encodings per release, checked by readable.sh and check-generated.sh. msgr2 frame layouts in all four modes are tabulated in src/test/msgr/test_frames_v2.cc, AES256KRB5 vectors from RFC 8009 in src/test/crypto.cc. Not registered: MOSDBackoff, the msgr1 raw structs and every msgr2 frame.

- **Class:** POSIX `operational` · exposure `internal` · relied on by tools
- **Map:** [transport-auth-and-client-library-contract.md › 3.10 Encoding framework and golden data](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#310-encoding-framework-and-golden-data)
- **Code:** `src/tools/ceph-dencoder/ceph_dencoder.cc:36-64 (usage)`, `src/test/msgr/test_frames_v2.cc:283-419 (round_trip_instances)`, `src/test/crypto.cc:391-470`
- **Notes:** Message::encode has a dump block guarded by ENCODE_DUMP, which nothing in the tree defines (transport page 3.10).

### Feature negotiation and client generations

CephFS feature bits and connection feature bits, how they are exchanged and enforced, and which client generations send or need each bit and each fallback.

<a id="f-proto-020"></a>

#### F-PROTO-020 · CephFS feature bitmap exchange at session open

*Client and MDS exchange variable-length CephFS feature bitmaps in MClientSession; every feature-gated MDS decision tests the client bitmap, and clients pick decoders from the MDS bitmap.*

feature_bitset_t is u32 byte length + little-endian u64 words (bit n in word n/64); a trailing partial word is tolerated on decode. The client sends its bits in REQUEST_OPEN (message v3+); the MDS stores them in session client_metadata.features and tests them with session->info.has_feature(bit). The MDS advertises CEPHFS_FEATURES_MDS_SUPPORTED = bits 0..24 (word 0x0000000001ffffff) and metric_spec bits 0..16 (0x1ffff) in OPEN, but only to clients that advertised MIMIC (bit 8); without it OPEN is sent as v1. Both clients overwrite their copy of the MDS bits on every OPEN: the kernel keeps only the first 64 bits (s_features) and needs a non-zero length of at least 8 bytes, libcephfs keeps the full bitset and the metric flags. The kernel 7.3 advertises 0-9, 11-18, 20-22; libcephfs advertises every bit 0..24 unless client_debug_inject_features overrides it.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): OPEN without supported_features and metric_spec (MClientSession v1) for sessions lacking CEPHFS_FEATURE_MIMIC (bit 8). The 7.3 kernel (bits 0-9, 11-18, 20-22) and libcephfs at this commit (bits 0-24) advertise bit 8, so every OPEN they receive is v7 with the MDS bitmap and metric spec. An OPEN without them (MClientSession v1) goes only to sessions without bit 8: kernels up to v4.18 and Ceph clients up to 13.2.1. Evidence: `src/mds/Server.cc:971-974 (Server::_session_logged)`, `src/messages/MClientSession.h:101-106 (MClientSession::encode_payload)`, `linux/fs/ceph/mds_client.h:45-60 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/client/Client.cc:448-452 (myfeatures)`, `linux-v4.18/fs/ceph/mds_client.c:953 (create_session_open_msg, version 2)`.
- **Gates:** CEPHFS_FEATURE_MIMIC (bit 8) on the client side for the MDS bitmap to be sent
- **Map:** [wire-protocol.md › 3.5 CephFS feature bits (CEPHFS_FEATURE_*)](../cpp-map/02-client-protocol/wire-protocol.md#35-cephfs-feature-bits-cephfs_feature_) · [sessions.md › 3.5 Feature negotiation](../cpp-map/02-client-protocol/sessions.md#35-feature-negotiation) · [kclient-expectations.md › 3.5 CephFS feature negotiation](../cpp-map/02-client-protocol/kclient-expectations.md#35-cephfs-feature-negotiation)
- **Code:** `src/mds/cephfs_features.h:31-105 (CEPHFS_FEATURE_*)`, `src/mds/mdstypes.cc:532-555 (feature_bitset_t::encode, decode)`, `src/mds/Server.cc:956-980 (Server::_session_logged)`, `linux/fs/ceph/mds_client.c:4391 (handle_session)`, `src/client/Client.cc:2688-2728 (Client::reinit_mds_features)`
- **Notes:** First releases per bit (wire-protocol 3.6.1): bits 5-8 Linux v4.19 / Ceph 13.2.2; 9 v5.1 / 14.2.0; 10 never in Linux / 14.2.0; 11 v5.0 / 13.2.7, 14.2.0; 12 v5.1 / 14.2.0; 13 v5.7 / 15.2.0; 14 v5.9 / 16.2.0; 15 v6.6 / 16.2.0; 16 v6.0 / 16.2.11, 17.2.6, 18.2.0; 17 v6.1 / 16.2.11, 17.2.6, 18.2.0; 18 v6.6 / 16.2.14, 17.2.7, 18.2.0; 19 never / 17.2.8, 18.2.0; 20 v6.7 / 17.2.8, 18.2.1, 19.2.0; 21 v6.10 / 18.2.4, 19.2.0; 22 v7.1 (kernel meaning SUBVOLUME_METRICS) / 19.2.3, 20.2.0; 23 never / 19.2.3, 20.2.0; 24 never / no tag yet (main only). Kernels before v6.0 ORed each bit into byte (list index)/8 instead of bit/8, correct only while advertised bits from 8 up stay below 16 (fixed in v6.0; kclient expectations 7.1.1). CentOS Stream 8 (4.18.0-448) advertises the v6.0 set, CentOS Stream 9 (5.14.0-757) the v6.10 set. See also F-QOS-011 (metrics negotiation).

<a id="f-proto-021"></a>

#### F-PROTO-021 · Per-bit gated behaviour and feature names

*Which CephFS bits change MDS behaviour, which bits only steer the client, and the bit names used by mon commands and dumps.*

MDS behaviour gated on the client bit: 8 MIMIC (session messages carry features, metric spec and error_string), 9 REPLY_ENCODING (versioned InodeStat/DirStat/LeaseStat), 11 LAZY_CAP_WANTED (only notable caps re-issued on stale-session resume), 13 DELEG_INO (openc_response_t create reply), 16 NOTIFY_SESSION_STATE (duplicate open answered with OPEN), 19 NEW_SNAPREALM_INFO (new snap-trace element), 21 MDS_AUTH_CAPS_CHECK (broken root_squash reporting), 22 CHARMAP (charmap gate), 24 QUARANTINE (quarantine checks and MQuarantineDisable). Bits 5-7 are release markers and bits 10, 12, 14, 15, 17, 18, 20 and 23 change nothing in the MDS. Clients test the MDS bits: the kernel 9, 12, 13, 14, 15, 17, 18, 20, 22; libcephfs 9, 10, 12, 13, 14, 17, 18, 19, 20, 23 plus metric-spec bit 16; a missing MDS bit degrades client behaviour (legacy encodings, no metrics, -EOPNOTSUPP for getvxattr, blockdiff and reclaim, 8-bit retry counters). cephfs_feature_name/cephfs_feature_from_name map bits to names (jewel .. quarantine, reserved for 0-4) for ceph fs required_client_features, ceph fs feature ls and session dumps.

- **Class:** POSIX `supporting` · exposure `client-wire`, `admin` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, admin
- **Legacy parts** (old-clients): MDS behaviour for sessions without bit 8 (v1 session messages, no error_string), 9 (legacy InodeStat/DirStat/LeaseStat), 11 (all caps re-issued on stale resume), 13 (raw created ino), 16 (duplicate REQUEST_OPEN dropped), 21 (broken root_squash reporting) and 22 (charmap -EPERM); client fallbacks towards an MDS without bits 9, 10, 12-15, 17-20, 23 or metric-spec bit 16. The 7.3 kernel and libcephfs both advertise bits 8, 9, 11, 13, 16, 21 and 22, so the MDS branches for sessions missing those bits are not taken by baseline peers. The branches for missing bits 19 (old snap-trace format) and 24 (request parking) stay current because the 7.3 kernel lacks both bits. The client fallbacks towards an MDS lacking a bit (legacy encodings, no metrics, -EOPNOTSUPP for getvxattr, blockdiff, reclaim and fscrypt policies, 8-bit retry counters, -EIO on idmapped mounts) are not taken because the baseline MDS advertises bits 0-24. Evidence: `src/mds/Server.cc:720-726 (send_reject_message, bit 8)`, `src/mds/Server.cc:971-974 (Server::_session_logged, bit 8)`, `src/mds/CInode.cc:4315 (CInode::encode_inodestat, bit 9)`, `src/mds/Locker.cc:2849-2855 (Locker::resume_stale_caps, bit 11)`, `src/mds/Server.cc:2595 (Server::set_reply_extra_bl, bit 13)`, `src/mds/Server.cc:668-686 (Server::handle_client_session, bit 16)`, `src/mds/Server.cc:770-774 (Server::handle_client_session, bit 21)`, `src/mds/Server.cc:4884-4888 (Server::can_handle_charmap, bit 22)`, `src/mds/Server.cc:12429-12437 (Server::get_snap_trace, bit 19)`, `src/mds/Server.cc:309 (Server::Server, supported_features)`, `linux/fs/ceph/mds_client.h:45-60 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/client/Client.cc:448-452 (myfeatures)`.
- **Map:** [wire-protocol.md › 3.5 CephFS feature bits (CEPHFS_FEATURE_*)](../cpp-map/02-client-protocol/wire-protocol.md#35-cephfs-feature-bits-cephfs_feature_) · [wire-protocol.md › 7. Externally visible contract](../cpp-map/02-client-protocol/wire-protocol.md#7-externally-visible-contract) · [kclient-expectations.md › 7.1.1 Feature bits 5-24: first releases](../cpp-map/02-client-protocol/kclient-expectations.md#711-feature-bits-5-24-first-releases)
- **Code:** `src/mds/cephfs_features.cc:13-60 (cephfs_feature_name, cephfs_feature_from_name)`, `src/mds/Server.cc:309-310 (Server::supported_features)`, `src/mds/Locker.cc:2849-2855 (Locker::resume_stale_caps)`, `linux/fs/ceph/ioctl.c:288-305 (vet_mds_for_fscrypt)`, `linux/fs/ceph/mds_client.c:3323-3344 (create_request_message)`
- **Notes:** cephfs_feature_name tests "if (id > size)" and so indexes one past the name array for id 25 (wire-protocol 8). CEPHFS_FEATURE_MAX is 24. See also F-QOS-070 (quarantine bit 24). Kernel uses of two MDS bits not named above: setting an fscrypt policy (FS_IOC_SET_ENCRYPTION_POLICY) fails with -EOPNOTSUPP unless the first existing session in the kernel's session array has MDS bit 15 ALTERNATE_NAME (vet_mds_for_fscrypt stops at the first non-NULL session); on an idmapped mount a request sent to an MDS session without bit 20 HAS_OWNER_UIDGID fails with -EIO unless the module parameter enable_unsafe_idmap is set (then the client sends the idmapped fsuid/fsgid as caller and warns once). See also F-NS-064 (fscrypt), F-NS-076 (alternate_name), F-PROTO-065 (request head versions).

<a id="f-proto-023"></a>

#### F-PROTO-023 · required_client_features enforcement and min_compat_client

*A per-file-system bitmap in the MDSMap lists CephFS bits required from every client of that file system; the MDS rejects, denies or evicts clients that miss one.*

ceph fs required_client_features <fs> add|rm <name|bit> edits the bitmap (bit 0..CEPHFS_FEATURE_MAX, name reserved refused, connected clients not checked); the deprecated ceph fs set <fs> min_compat_client <release> replaces the whole bitmap with one release-marker bit (13 octopus and later, 12 nautilus, 8 mimic, 7 luminous, 6 kraken, 5 jewel), so it cannot require anything newer than octopus. The MDS copies the bitmap when a new MDSMap changes it. REQUEST_OPEN missing a bit (after inference) gets REJECT plus a cluster-log warning (error_string "missing required features '<hex>'" only for bit-8 sessions); a reconnect missing a bit gets CLOSE (reconnect denied); on a bitmap change, once the rank is at least up:reconnect, every non-blocklisted client session missing a bit is evicted (blocklisted per mds_session_blocklist_on_evict). The test is per bit, applies to every session of the file system, and the MDS never consults the OSDMap require_min_compat_client.

- **Class:** POSIX `operational` · exposure `client-wire`, `cluster-wire`, `admin` · relied on by mon, admin, kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients, deprecated-admin): `ceph fs set <fs> min_compat_client` and MDSMap::set_min_compat_client; feature inference before the required-bit test at open and reconnect; REJECT without error_string for sessions without bit 8. The required_client_features bitmap, its mon command and the reject, deny and evict enforcement are current. The deprecated `ceph fs set <fs> min_compat_client` (its output calls itself deprecated and points to required_client_features) is used by no baseline peer: a search of src/pybind/mgr, src/cephadm, src/python-common and rook/pkg finds only uses of the OSDMap require_min_compat_client. Inference before the test and REJECT without error_string concern only sessions without a bitmap or without bit 8 (kernels up to v4.18, Ceph up to 13.2.1). Evidence: `src/mon/FSCommands.cc:779-791 (min_compat_client, deprecation warning)`, `src/mds/MDSMap.cc:1327-1353 (MDSMap::set_min_compat_client)`, `src/mds/Server.cc:747-748 (Server::handle_client_session, infer)`, `src/mds/Server.cc:1655-1656 (Server::handle_client_reconnect, infer)`, `src/mds/Server.cc:720-726 (send_reject_message)`, `src/pybind/mgr/balancer/module.py:405 (OSDMap require_min_compat_client)`.
- **Gates:** MDSMap required_client_features (default empty); mon command ceph fs required_client_features; mon command ceph fs set <fs> min_compat_client (deprecated); mds_session_blocklist_on_evict (default true)
- **Map:** [wire-protocol.md › 3.5 CephFS feature bits (CEPHFS_FEATURE_*)](../cpp-map/02-client-protocol/wire-protocol.md#35-cephfs-feature-bits-cephfs_feature_) · [kclient-expectations.md › 7.1.6 How older clients are refused today](../cpp-map/02-client-protocol/kclient-expectations.md#716-how-older-clients-are-refused-today) · [sessions.md › 3.5 Feature negotiation](../cpp-map/02-client-protocol/sessions.md#35-feature-negotiation)
- **Code:** `src/mds/Server.cc:1799-1826 (Server::update_required_client_features)`, `src/mds/MDSMap.cc:1327-1353 (MDSMap::set_min_compat_client)`, `src/mon/FSCommands.cc:966-1050 (RequiredClientFeaturesHandler::handle)`, `src/mds/Server.cc:1650-1661 (Server::handle_client_reconnect)`
- **Notes:** Derived from code: v6.0+ kernels (and CentOS Stream 8/9) skip required_client_features in the MDSMap as a u64 set while it is encoded as a byte-length bitmap, so once any bit is required they misparse the rest of the map extension and usually lose max_xattr_size, which sends every buffered xattr change through a synchronous SETXATTR (kclient expectations 7.1.6). Bits 10, 19, 23 and 24 cannot be required without refusing every kernel, and bit 22 means different things to kernels and Ceph clients (7.1.7). libcephfs reports a rejection as "mds.N rejected us" and -EPERM; the kernel marks the session REJECTED and requests fail with -EACCES.

<a id="f-proto-024"></a>

#### F-PROTO-024 · Connection-level feature bits consulted by the client protocol

*Messenger CEPH_FEATURE_* bits negotiated per connection that select client-message encodings and a few behaviours.*

The MDS messenger advertises CEPH_FEATURES_ALL. Bits consulted on client connections: FLOCK (3) - MClientCaps encoded as v1 without it; DIRLAYOUTHASH (7) - dir_layout in legacy InodeStat; CREATEPOOLID (26) - args.open.pool honoured; REPLY_CREATE_INODE (27) - raw u64 created ino; MDSENC (29) - client-chosen reconnect v3; EXPORT_PEER (37) - MClientCaps v3 import peer, and FLUSHMSG only sent to such clients; MDS_INLINE_DATA (40) - inline fields, opening an inline file without it is -EPERM, new inodes never inline; MDS_QUOTA (47) - quota in legacy InodeStat; FS_FILE_LAYOUT_V2 (58) - pool namespace, a directory with a namespaced layout gives -EIO without it; FS_BTIME = FS_CHANGE_ATTR = MSG_ADDR2 (59) - request head format and btime/change_attr; NEW_OSDOP_ENCODING (56) for feature inference. The kernel itself tests the MDS connection for FS_BTIME (legacy request head without it), MDSENC (reconnect v3) and SERVER_LUMINOUS (21; REQUEST_FLUSH_MDLOG only to such MDSs). Feature tests use masks that include incarnation bits.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients, deprecated-admin): MClientCaps v1/v2 (no FLOCK/EXPORT_PEER); connection-feature field presence in the legacy InodeStat (only without bit 9); raw u64 created ino via REPLY_CREATE_INODE (only without bit 13); FLUSHMSG skipped without EXPORT_PEER; -EPERM opening an inline file, non-inline new inodes and NOINLINE/NOPOOLNS/NOQUOTA cap states without MDS_INLINE_DATA/FS_FILE_LAYOUT_V2/MDS_QUOTA; -EIO for namespaced directory layouts without FS_FILE_LAYOUT_V2; NEW_OSDOP_ENCODING, FS_CHANGE_ATTR and FS_FILE_LAYOUT_V2 in feature inference; kernel legacy request head (no FS_BTIME), reconnect v2 (no MDSENC) and suppressed FLUSH_MDLOG (no SERVER_LUMINOUS); inline data content (deprecated inline_data setting). The 7.3 kernel advertises FLOCK, DIRLAYOUTHASH, REPLY_CREATE_INODE, MDSENC, EXPORT_PEER, MDS_INLINE_DATA, MDS_QUOTA, NEW_OSDOP_ENCODING, bit 58 and bit 59, and C++ clients advertise CEPH_FEATURES_ALL, so the MDS branches for connections lacking these bits and the client fallbacks towards an MDS lacking FS_BTIME, MDSENC or SERVER_LUMINOUS are not taken by baseline peers. CREATEPOOLID is current on both sides of its test, because libcephfs advertises it and the 7.3 kernel does not. Inline data beyond CEPH_INLINE_NONE exists only after the deprecated `ceph fs set <fs> inline_data true --yes-i-really-really-mean-it`, which no mgr module, cephadm, Rook or tool sets. Evidence: `linux/include/linux/ceph/ceph_features.h:179-220 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:263 (CEPH_FEATURES_SUPPORTED_DEFAULT = CEPH_FEATURES_ALL)`, `src/messages/MClientCaps.h:347-360 (MClientCaps::encode_payload)`, `src/mds/CInode.cc:4408-4425 (CInode::encode_inodestat, legacy branch)`, `src/mds/Server.cc:2609-2612 (Server::set_reply_extra_bl)`, `src/mds/Server.cc:897-902 (Server::flush_session)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mds/Server.cc:4704-4709 (Server::handle_client_open)`, `src/mds/SessionMap.cc:1156-1162 (Session::check_access)`, `src/mds/Capability.cc:175-180 (Capability::Capability)`, `src/mds/Server.cc:1778-1790 (Server::infer_supported_features)`, `src/mds/Server.cc:4954-4956 (Server::handle_client_openc, CREATEPOOLID)`, `linux/fs/ceph/mds_client.c:3202 (create_request_message)`, `linux/fs/ceph/mds_client.c:5241-5248 (send_mds_reconnect)`, `linux/fs/ceph/mds_client.c:6453-6454 (send_flush_mdlog)`, `src/mon/FSCommands.cc:471-478 (inline_data, deprecated)`.
- **Map:** [wire-protocol.md › 3.6 Connection-level features consulted by the client pro...](../cpp-map/02-client-protocol/wire-protocol.md#36-connection-level-features-consulted-by-the-client-protocol) · [kclient-expectations.md › 3.1 Transport, identity and messenger feature bits](../cpp-map/02-client-protocol/kclient-expectations.md#31-transport-identity-and-messenger-feature-bits) · [transport-auth-and-client-library-contract.md › 5.1 Raw wire structures](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#51-raw-wire-structures)
- **Code:** `src/include/ceph_features.h:194-261 (CEPH_FEATURES_ALL)`, `src/messages/MClientCaps.h:347-360 (MClientCaps::encode_payload)`, `src/mds/SessionMap.cc:1156-1162 (Session::check_access)`, `linux/include/linux/ceph/ceph_features.h:179-222 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `linux/fs/ceph/mds_client.c:6445-6468 (send_flush_mdlog)`
- **Notes:** The kernel advertises bit 59 under the name MSG_ADDR2 and bit 58 under CRUSH_TUNABLES5/NEW_OSDOPREPLY_ENCODING, so a modern kernel looks like it has FS_BTIME and FS_FILE_LAYOUT_V2 (wire-protocol 3.6). The kernel advertises all transport-relevant bits except UID and OSDREPLYMUX, which the MDS requires only from mons, MDS peers, the mgr and OSDs. It also does not advertise CREATEPOOLID (26) (linux/include/linux/ceph/ceph_features.h:115, absent from :179-220) and never sets args.open.pool, so the MDS ignores that field for kernel CREATE and OPEN requests (src/mds/Server.cc:4954-4956); libcephfs advertises the bit and sends -1 or a chosen pool (src/client/Client.cc:11151, :16198). FLOCK exists since kernel 2.6.36 and EXPORT_PEER since 3.14, so no current client exercises the v1/v2 MClientCaps fallback.

<a id="f-proto-025"></a>

#### F-PROTO-025 · Feature bit 22 numbering conflict (Ceph CHARMAP vs kernel SUBVOLUME_METRICS)

*The 7.1+ kernel and Ceph use CephFS bit 22 for different features, which changes MDS and kernel behaviour in both directions.*

In this tree bit 22 is CEPHFS_FEATURE_CHARMAP (first released in 19.2.3 and 20.2.0) and there is no subvolume-metrics feature bit (libcephfs uses metric-spec bit 16). The kernel numbers CEPHFS_FEATURE_SUBVOLUME_METRICS 22 by enum position and advertises it since v7.1. The only MDS test of the bit is Server::can_handle_charmap, which fails create (O_CREAT), mkdir, symlink, link and rename into a directory with a charmap with -EPERM unless the session has bit 22 (mknod is not gated); a v7.1+ kernel therefore passes the gate although it skips the charmap optmetadata and neither normalizes nor case-folds names. In the other direction the kernel enables subvolume metrics on every OPEN whose MDS bitmap has bit 22 (the last OPEN from any rank decides) and then sends type-16 metric items, which this tree decodes.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Legacy parts** (old-clients): The -EPERM branch of Server::can_handle_charmap for sessions without bit 22. Both baseline client families advertise bit 22 (the 7.3 kernel as SUBVOLUME_METRICS since v7.1, libcephfs as CHARMAP), so Server::can_handle_charmap never refuses them and the numbering conflict itself is current. The -EPERM refusal is reached only by sessions without bit 22: kernels before v7.1 and Ceph clients before 19.2.3 and 20.2.0. Evidence: `src/mds/Server.cc:4884-4888 (Server::can_handle_charmap)`, `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP)`, `linux/fs/ceph/mds_client.h:59 (CEPHFS_FEATURE_SUBVOLUME_METRICS in CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux-v7.1/fs/ceph/mds_client.h:59 (first kernel advertising bit 22)`, `v19.2.3:src/mds/cephfs_features.h:52 (CEPHFS_FEATURE_CHARMAP)`, `src/client/Client.cc:448-452 (myfeatures)`.
- **Map:** [wire-protocol.md › 8. Edge cases, failure handling and gotchas](../cpp-map/02-client-protocol/wire-protocol.md#8-edge-cases-failure-handling-and-gotchas) · [kclient-expectations.md › 7.1.4 The bit 22 conflict](../cpp-map/02-client-protocol/kclient-expectations.md#714-the-bit-22-conflict) · [sessions.md › 3.5 Feature negotiation](../cpp-map/02-client-protocol/sessions.md#35-feature-negotiation)
- **Code:** `src/mds/Server.cc:4876-4892 (Server::can_handle_charmap)`, `linux/fs/ceph/mds_client.h:25-60 (enum ceph_feature_type, CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.c:4580-4584 (handle_session)`, `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP)`
- **Notes:** Derived from code, not observed: a v7.1+ kernel creates entries under the user spelling with an empty alternate_name in a directory whose libcephfs-created entries are stored folded, so names differing only in case can coexist (kclient expectations 7.1.4). Per release: 18.2.x and older and 19.2.0-19.2.2 lack bit 22; 19.2.3 and 20.2.x advertise it but send InodeStat v8 (no subvolume_id), so no type-16 items flow; this tree and 21.x send InodeStat v9 and decode type 16. Requiring bit 22 admits only v7.1+ kernels and 19.2.3+/20.2.0+ Ceph clients, with different meanings. See also F-QOS-017 (metrics side), F-NS-075 (charmap).

<a id="f-proto-026"></a>

#### F-PROTO-026 · Client generations and candidate minimum client floors

*Fact table of what representative kernel and Ceph client generations send and expect, and which fallbacks each candidate minimum generation would leave unused.*

Representative generations (kclient expectations 7.1.5): Linux v4.18 (no bitmap, session v2, legacy request head, reconnect v3, legacy replies, raw created ino, caps v10, msgr1 only); v5.4 (bits 0-9, 11, 12, versioned replies); v5.10 (0-9, 11-14, openc_response_t); v5.14/v5.15 (request head v1, msgr2 opt-in); v6.6 (head v2, caps v12); v6.8 (session v7, head v3); v6.12 and 7.3 (cap_auths used); Ceph 13.2.2 (bits 0-8), 14.2.0 (0-12, reconnect v5, msgr2), 15.2.0 (0-13), 16.2.0/17.2.0 (0-15), 18.2.0 (0-19), 19.2.0 (0-21), 20.2.0 (0-23). Candidate floors and the bit that enforces them: F1 bitmap (bit 8, v4.19+, 13.2.2+), F2 Nautilus (bit 12 or 9, v5.1+, 14.2.0+), F3 Octopus (bit 13, v5.7+, 15.2.0+), F4 versioned head (bit 16 or 17, v6.0+/v6.1+), F5 32-bit retry/forward (bit 18, v6.6+), F6 owner uid/gid (bit 20, v6.7+), F7 client-side MDS cap checks (bit 21, v6.10+). Not removed by any floor: msgr1 for kernels mounted without ms_mode (no CephFS bit reveals msgr2 capability; mount.ceph passes ms_mode=prefer-crc for the new device syntax since 17.2.0), single-message reconnect v3 (every kernel v4.8-7.3, the current kernel's format), the old snap-trace format (no kernel sets bit 19, so it is the current kernel's format), session open v4 (CentOS Stream 9 with bit 21), and bits no kernel has (10, 19, 23, 24).

- **Class:** POSIX `operational` · exposure `client-wire`, `admin` · relied on by kclient, libcephfs, ceph-fuse, ganesha, admin
- **Legacy parts** (old-clients, msgr1): Rows for Linux v4.18-v6.12, CentOS Stream 8/9 and Ceph 13.2.2-20.2.0; candidate floors F1-F7 with the fallbacks each would leave unused; the 'not removed by any floor' items msgr1 and session open v4 (CentOS Stream 9). Of the generations in the table only the 7.3 kernel and Ceph clients at this commit are baseline peers. The facts that stay current are those of the 7.3 kernel: session open v7, head v3 in message v6, single-message reconnect v3, the old snap-trace format and the absence of bits 10, 19, 23 and 24. The older Linux, CentOS Stream 8/9 and Ceph 13.2.2-20.2.0 rows, the candidate floors F1-F7, session open v4 (CentOS Stream 9) and msgr1 concern clients or mount configurations outside the baseline. Evidence: `linux/fs/ceph/mds_client.h:45-60 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.c:1704 (create_session_full_msg, version 7)`, `linux/fs/ceph/mds_client.c:3367-3375 (create_request_message, message v6, head v3)`, `linux/fs/ceph/mds_client.c:5241-5248 (send_mds_reconnect)`, `src/mds/Server.cc:12429-12437 (Server::get_snap_trace)`, `cs9-kernel/fs/ceph/mds_client.c:1591 (session open version 4)`, `linux/net/ceph/ceph_common.c:329-330 (ceph_alloc_options)`, `src/mount/mount.ceph.c:306-309 (parse_new_dev)`.
- **Map:** [kclient-expectations.md › 7.1.5 Representative client generations](../cpp-map/02-client-protocol/kclient-expectations.md#715-representative-client-generations) · [kclient-expectations.md › 7.1.7 Decision input: candidate minimum client generations](../cpp-map/02-client-protocol/kclient-expectations.md#717-decision-input-candidate-minimum-client-generations) · [wire-protocol.md › 3.6.1 Client generations: first releases per bit and per ...](../cpp-map/02-client-protocol/wire-protocol.md#361-client-generations-first-releases-per-bit-and-per-fallback)
- **Code:** `src/mds/cephfs_features.h:104-105 (CEPHFS_FEATURES_MDS_SUPPORTED, CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.h:45-60 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`
- **Notes:** Enterprise kernels backport CephFS changes and do not follow their base version: CentOS Stream 8 4.18.0-448.el8 behaves like upstream v6.0 for feature bits, CentOS Stream 9 5.14.0-757.el9 like v6.10; no other distribution kernel was examined. A non-empty required_client_features also triggers the kernel MDSMap mis-skip described in F-PROTO-023.

### Session establishment and session messages

The per-rank client session: its states, the open handshake and admission checks, the MClientSession encoding, and the session-level barriers and notifications.

<a id="f-proto-027"></a>

#### F-PROTO-027 · Session state machine

*Every client holds one session per rank, in one of six MDS-side states, before requests, caps or leases are accepted.*

States: closed (0), opening (1, journaling open), open (2), closing (3, journaling close), stale (4), killing (5), plus an orthogonal importing_count for sessions force-opened on behalf of a client. Every state change increments state_seq and asynchronous completions become no-ops if it changed. Transitions: closed to opening on a valid REQUEST_OPEN or force open; opening to open when ESession(open) is safe; open to stale in idle detection; stale to open on REQUEST_RENEWCAPS; open/stale/opening to closing on REQUEST_CLOSE with matching seq; open/stale/opening (not importing) to killing on kill_session; closing/killing to closed and removal when the close is journaled; closing/killing to opening on a force open by import or rejoin. Sessions loaded from RADOS or created by journal replay start in open. Client-side states differ: the kernel has NEW, OPENING, OPEN, HUNG, CLOSING, CLOSED, RESTARTING, RECONNECTING, REJECTED; libcephfs OPENING, OPEN, CLOSING, CLOSED, STALE, REJECTED, and keeps sending requests on a STALE session.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [sessions.md › 3.2 State machine](../cpp-map/02-client-protocol/sessions.md#32-state-machine) · [kclient-expectations.md › 3.4.1 Client session states](../cpp-map/02-client-protocol/kclient-expectations.md#341-client-session-states) · [libcephfs-fuse-ganesha-expectations.md › 3.3 Session states and session messages](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#33-session-states-and-session-messages)
- **Code:** `src/mds/SessionMap.h:93-100 (Session::STATE_CLOSED .. STATE_KILLING)`, `src/mds/SessionMap.h:136-142 (Session::set_state)`, `src/mds/SessionMap.cc:568-589 (SessionMap::set_state)`
- **Notes:** REQUEST_CLOSE is accepted in opening; the later open completion still consumes its projected version but makes no state change and sends no OPEN (sessions 3.2).

<a id="f-proto-028"></a>

#### F-PROTO-028 · Session open handshake (REQUEST_OPEN, journaled open, OPEN)

*A client opens a session per rank with REQUEST_OPEN; the MDS validates it, journals the open and only then replies OPEN.*

The kernel sends REQUEST_OPEN as MClientSession v7 with exactly four metadata keys (hostname, kernel_version, entity_id, root), its feature bitmap, a metric spec for types 0-16, flags 0, an empty cap_auths vector and oldest_client_tid; libcephfs sends v7 with its metadata, all feature bits and CEPHFS_METRIC_FEATURES_ALL. On acceptance the MDS adds the session to the SessionMap, projects a new SessionMap version, sets opening, submits ESession(open, client_metadata, auth_name) and flushes the log; after the commit it sets open, registers the session with the dmClock scheduler and MetricsHandler, and sends OPEN with seq 0, supported_features and metric_spec (MIMIC clients only) and cap_auths, followed by FORCE_RO if the cache is read-only. REQUEST_OPEN is queued until the rank is at least up:clientreplay. The kernel queues requests for that rank until OPEN; libcephfs sends an immediate REQUEST_RENEWCAPS after OPEN and opens sessions to the rank's export targets.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Decoding REQUEST_OPEN messages that end at v2-v5 (no cap_auths, no oldest_client_tid; v2 also without a bitmap); OPEN encoded as v1 for sessions without bit 8. The 7.3 kernel sends REQUEST_OPEN as MClientSession v7 and libcephfs at this commit encodes it as v7. Versions v2-v5 come only from older clients: v2 from kernels up to v4.18 and Ceph up to 13.2.1, v3 from kernels v4.19-v5.8, v4 from kernels v5.9-v6.7 and CentOS Stream 8 and 9, and v3-v5 from Ceph releases before 18.2.4 and 19.2.0. OPEN without supported_features and metric_spec goes only to sessions without bit 8. Evidence: `src/messages/MClientSession.h:77-97 (MClientSession::decode_payload)`, `src/messages/MClientSession.h:99-115 (MClientSession::encode_payload)`, `linux/fs/ceph/mds_client.c:1704 (create_session_full_msg, version 7)`, `src/client/Client.cc:2656-2660 (Client::_open_mds_session)`, `linux-v4.18/fs/ceph/mds_client.c:953 (version 2)`, `linux-v4.19/fs/ceph/mds_client.c:979 (version 3)`, `linux-v5.9/fs/ceph/mds_client.c:1305 (version 4)`, `cs8-kernel/fs/ceph/mds_client.c:1403 (version 4)`, `cs9-kernel/fs/ceph/mds_client.c:1591 (version 4)`, `linux-v6.8/fs/ceph/mds_client.c:1605 (version 7)`, `src/mds/Server.cc:971-974 (Server::_session_logged)`.
- **Map:** [sessions.md › 3.4 Open handshake, metadata and rejection reasons](../cpp-map/02-client-protocol/sessions.md#34-open-handshake-metadata-and-rejection-reasons) · [wire-protocol.md › 3.4 Session open and feature exchange](../cpp-map/02-client-protocol/wire-protocol.md#34-session-open-and-feature-exchange) · [kclient-expectations.md › 3.4.2 REQUEST_OPEN / REQUEST_RENEWCAPS encoding](../cpp-map/02-client-protocol/kclient-expectations.md#342-request_open--request_renewcaps-encoding) · [libcephfs-fuse-ganesha-expectations.md › 3.2 Mount, map subscription and session open](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#32-mount-map-subscription-and-session-open)
- **Code:** `src/mds/Server.cc:645-827 (Server::handle_client_session)`, `src/mds/Server.cc:956-980 (Server::_session_logged)`, `linux/fs/ceph/mds_client.c:1636 (create_session_full_msg)`, `src/client/Client.cc:2646 (Client::_open_mds_session)`
- **Notes:** Open completes after one journal flush (sessions 7, timing summary). Session-open message versions received from clients: v2 (kernels up to v4.18, Ceph up to 13.2.1), v3 (v4.19-v5.8, 13.2.2-14.x), v4 (v5.9-v6.7, CentOS Stream 8 and 9, 15.2.0-15.2.15, 16.2.0-16.2.6), v5 (15.2.16+, 16.2.7+, 17.2.x, 18.2.0-18.2.2), v7 (v6.8+, 18.2.4, 19.2.0+) (kclient expectations 7.1.2). The kernel never writes stamp, max_caps or max_leases in its session head; the MDS does not read them. See also F-STORE-028 (ESession events).

<a id="f-proto-029"></a>

#### F-PROTO-029 · Session open admission checks and rejection reasons

*The ordered checks the MDS applies to REQUEST_OPEN, and the REJECT reasons clients can see.*

Before any op: no session on the connection gives REJECT "sessionless"; a session lacking MAY_READ caps for this rank's fs_name gives REJECT naming the file system (both for every session op, not only open). For REQUEST_OPEN, in order: MDSMap flag refuse_client_session (REJECT with an explanatory error_string); session closing (wait for the journal and retry); session opening/open/stale/killing or sessions being terminated (drop, or duplicate-open OPEN, F-PROTO-037); rank stopping (drop); store client_metadata; address in the OSD blocklist (REJECT with SESSION_BLOCKLISTED); infer features if none; missing required features (REJECT); root_squash caps without bit 21 (flagged, not rejected); metadata root empty or not starting with "/" (REJECT "invalue root", literal typo) or not allowed by the cap path (REJECT "non-allowable root"); metadata uuid already used by another session (REJECT "duplicated session uuid"); otherwise accept. Rejections of the last rows also emit cluster-log warnings and a debug log line with status REJECTED.

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, admin
- **Legacy parts** (old-clients): The infer_supported_features step; flagging root_squash sessions without bit 21 as broken. Feature inference runs only for REQUEST_OPEN without a bitmap, which kernels up to v4.18 and Ceph up to 13.2.1 send. The broken-root_squash flag, reported as MDS_HEALTH_CLIENTS_BROKEN_ROOTSQUASH, applies only to sessions without bit 21 (kernels before v6.10, Ceph before 18.2.4 and 19.2.0). The 7.3 kernel and libcephfs send a bitmap that contains bit 21. Evidence: `src/mds/Server.cc:747-748 (Server::handle_client_session)`, `src/mds/Server.cc:770-774 (Server::handle_client_session)`, `src/mds/Beacon.cc:489-510 (Beacon::notify_health)`, `linux/fs/ceph/mds_client.h:58 (CEPHFS_FEATURE_MDS_AUTH_CAPS_CHECK advertised)`, `linux-v6.10/fs/ceph/mds_client.h:56 (first kernel advertising bit 21)`, `src/client/Client.cc:448-452 (myfeatures)`.
- **Gates:** MDSMap flag refuse_client_session (ceph fs set <fs> refuse_client_session true); MDSMap required_client_features
- **Map:** [sessions.md › 3.3 Session message dispatch and state gating](../cpp-map/02-client-protocol/sessions.md#33-session-message-dispatch-and-state-gating) · [sessions.md › 3.4 Open handshake, metadata and rejection reasons](../cpp-map/02-client-protocol/sessions.md#34-open-handshake-metadata-and-rejection-reasons)
- **Code:** `src/mds/Server.cc:599-641 (Server::handle_client_session)`, `src/mds/Server.cc:645-822 (Server::handle_client_session)`, `src/mds/Server.cc:720-726 (Server::send_reject_message)`
- **Notes:** With refuse_client_session set even a duplicate REQUEST_OPEN from an open session gets REJECT, because the flag is checked before the state (sessions 8). libcephfs fails the mount with -EACCES before opening a session when the MDSMap has the flag. The monitor requires refuse_client_session before fs rename. Connection-level refusals (cephx failure, unparsable MDS caps) never reach this code. See also F-ADM-017 (mount root and reclaim auth checks).

<a id="f-proto-030"></a>

#### F-PROTO-030 · REJECT encoding and blocklist signalling to clients

*How a refused or blocklisted client learns about it from CEPH_SESSION_REJECT, and what each client does next.*

REJECT messages built by send_reject_message carry metadata error_string only for clients that advertised MIMIC; the sessionless, no-fs-caps and refuse_client_session REJECTs always carry it (so they are encoded as v7). For a blocklisted address the MDS sets flags = SESSION_BLOCKLISTED (1) and error_string "blocklisted (blacklisted)". A v1-encoded REJECT (pre-mimic client) has neither flags nor error_string. The kernel marks itself blocklisted from the flag (session message v5+) or from an error_string containing "blacklisted" (v3/v4 only), marks the session REJECTED, fails later requests to that rank with -EACCES, and with recover_session=clean force-reconnects with a new address nonce; it WARNs on a REJECT that arrives when the session is neither OPENING nor RECONNECTING. libcephfs logs error_string, closes the session with -EPERM and ignores flags; requests to a REJECTED rank fail with -EPERM.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): v1 REJECT for sessions without bit 8; the '(blacklisted)' substring in the blocklist error_string; the kernel's blocklist-marker scan of v3/v4 session metadata. Baseline clients advertise bit 8 and receive REJECT as v7 with flags and error_string; the 7.3 kernel takes blocklisting from the flag of v5+ messages and libcephfs logs error_string. A REJECT without flags or error_string goes only to sessions without bit 8, and in a v7 REJECT the 'blacklisted' substring of error_string is read only by kernels v5.4-v5.15, because flag decoding arrived with kernel commit e1c9788cb397 in v5.16. The kernel's error_string scan of v3/v4 session messages applies only to MDS releases that sent MClientSession v3 or v4. Evidence: `src/mds/Server.cc:720-726 (send_reject_message)`, `src/mds/Server.cc:733-745 (Server::handle_client_session, blocklisted REJECT)`, `src/messages/MClientSession.h:101-106 (MClientSession::encode_payload)`, `linux/fs/ceph/mds_client.c:4376-4380 (__decode_session_metadata)`, `linux/fs/ceph/mds_client.c:4417-4425 (handle_session, v3/v4 metadata)`, `linux/fs/ceph/mds_client.c:4435-4449 (handle_session, v5+ flags)`, `linux-v5.4/fs/ceph/mds_client.c:3050 (error_string 'blacklisted' scan)`.
- **Gates:** client bit CEPHFS_FEATURE_MIMIC (8) for error_string and flags; kernel mount option recover_session=clean
- **Map:** [wire-protocol.md › 3.11.1 MClientSession](../cpp-map/02-client-protocol/wire-protocol.md#3111-mclientsession) · [kclient-expectations.md › 3.4.3 Session messages from the MDS](../cpp-map/02-client-protocol/kclient-expectations.md#343-session-messages-from-the-mds) · [kclient-expectations.md › 3.4.7 Blocklist recovery and manual reset](../cpp-map/02-client-protocol/kclient-expectations.md#347-blocklist-recovery-and-manual-reset) · [libcephfs-fuse-ganesha-expectations.md › 3.3 Session states and session messages](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#33-session-states-and-session-messages)
- **Code:** `src/mds/Server.cc:720-743 (Server::handle_client_session)`, `src/messages/MClientSession.h:98-115 (MClientSession::encode_payload)`, `linux/fs/ceph/mds_client.c:4360-4449 (__decode_session_metadata, handle_session)`, `linux/fs/ceph/mds_client.c:6185-6200 (maybe_recover_session)`, `src/client/Client.cc:2791-2802 (Client::handle_client_session)`
- **Notes:** The C++ code comment states the kernel depends on the substring "blacklisted"; current kernels receive v7 REJECTs and use the flag only. A sessionless REJECT to a RENEWCAPS of an OPEN kernel session trips the kernel WARN_ON and marks the session REJECTED (kclient expectations 3.4.3). See also F-PROTO-052 (blocklist-driven kill).

<a id="f-proto-031"></a>

#### F-PROTO-031 · MClientSession encoding and the ops the MDS sends

*Field layout of MClientSession v1-v7 and which session ops the MDS sends to clients.*

Payload: raw ceph_mds_session_head (28 bytes: u32 op, u64 seq, ceph_timespec stamp, u32 max_caps, u32 max_leases); v2 map<string,string> metadata; v3 feature_bitset_t supported_features; v4 metric_spec_t; v5 u32 flags; v6 vector<MDSCapAuth> cap_auths; v7 u64 oldest_client_tid (default UINT64_MAX = do not trim). The encoder writes v1 (head only) when metadata and supported_features are both empty, otherwise v7. MDS-to-client ops: OPEN (1), CLOSE (3), RENEWCAPS (5), STALE (6), RECALL_STATE (7, head.max_caps = new cap limit; owned by cap recall), FLUSHMSG (8), FORCE_RO (10), REJECT (11); client-to-MDS ops: REQUEST_OPEN (0), REQUEST_CLOSE (2), REQUEST_RENEWCAPS (4), FLUSHMSG_ACK (9), REQUEST_FLUSH_MDLOG (12). MDS-originated messages are therefore v1 unless they carry error_string or are an OPEN to a MIMIC client. libcephfs ceph_abort()s on any received op other than the eight MDS ops; the kernel WARNs. Any session message resets the kernel s_ttl and moves a HUNG session back to OPEN.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Decoding client-sent MClientSession versions 2-5; the kernel's version-3/4 metadata scan for the blocklist marker. Baseline clients send session messages as v1-format head-only messages (kernel ceph_create_session_msg, libcephfs messages without metadata) or as v7, and the MDS sends v1 or v7, so these encodings are current. Client messages that end at v2-v5 come only from kernels up to v6.7, CentOS Stream 8 and 9 and Ceph releases before 18.2.4 and 19.2.0. The kernel's v3/v4 decode path applies only to MDS releases that sent those versions. Evidence: `src/messages/MClientSession.h:77-97 (MClientSession::decode_payload)`, `src/messages/MClientSession.h:99-115 (MClientSession::encode_payload)`, `linux/fs/ceph/mds_client.c:1540-1557 (ceph_create_session_msg)`, `linux/fs/ceph/mds_client.c:1704 (create_session_full_msg, version 7)`, `linux/fs/ceph/mds_client.c:4417-4425 (handle_session)`, `cs9-kernel/fs/ceph/mds_client.c:1591 (version 4)`.
- **Map:** [wire-protocol.md › 3.11.1 MClientSession](../cpp-map/02-client-protocol/wire-protocol.md#3111-mclientsession) · [sessions.md › 5. Interfaces](../cpp-map/02-client-protocol/sessions.md#5-interfaces) · [kclient-expectations.md › 3.4.3 Session messages from the MDS](../cpp-map/02-client-protocol/kclient-expectations.md#343-session-messages-from-the-mds)
- **Code:** `src/messages/MClientSession.h:77-115 (MClientSession::decode_payload, encode_payload)`, `src/include/ceph_fs.h:358-387 (CEPH_SESSION_*, ceph_mds_session_head)`, `src/client/Client.cc:2805-2806 (Client::handle_client_session)`, `linux/fs/ceph/mds_client.c:4391 (handle_session)`
- **Notes:** The head stamp is read by neither the MDS, libcephfs nor the kernel handle_session (search only). The kernel decodes by version: v3/v4 scan metadata for the blocklist marker, v5+ skip metadata and read flags, it reads only the first 64 feature bits, skips metric_spec, parses cap_auths only on OPEN and ignores oldest_client_tid.

<a id="f-proto-032"></a>

#### F-PROTO-032 · Session op dispatch and rank-state gating

*Which session ops the MDS handles in which rank state, and what each client op does.*

REQUEST_RENEWCAPS is handled in any rank state; REQUEST_CLOSE waits until the rank is up:active; REQUEST_OPEN, FLUSHMSG_ACK and REQUEST_FLUSH_MDLOG wait until up:clientreplay (CREATING and STARTING are numerically below REPLAY, so such comparisons also hold for them). Per op: REQUEST_OPEN validates and journals an open; REQUEST_RENEWCAPS renews (open or stale sessions only); REQUEST_CLOSE closes when seq equals push_seq; FLUSHMSG_ACK completes flush waiters with key <= seq; REQUEST_FLUSH_MDLOG flushes the log when active, without reply; any other value gets REJECT and eviction. Every client message of any type that reaches the MDS dispatcher updates Session::last_seen, which idle detection uses as liveness.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [sessions.md › 3.3 Session message dispatch and state gating](../cpp-map/02-client-protocol/sessions.md#33-session-message-dispatch-and-state-gating) · [kclient-expectations.md › 3.4.4 Session upkeep and timing](../cpp-map/02-client-protocol/kclient-expectations.md#344-session-upkeep-and-timing)
- **Code:** `src/mds/Server.cc:599-641 (Server::handle_client_session)`, `src/mds/Server.cc:829-895 (Server::handle_client_session)`, `src/mds/MDSRank.cc:1046-1050 (MDSRankDispatcher::ms_dispatch)`

<a id="f-proto-033"></a>

#### F-PROTO-033 · Unknown session op handling

*A session op the MDS does not handle (including MDS-to-client ops sent by a client) gets REJECT and the client is evicted and blocklisted.*

Server::handle_client_session sends a v1 REJECT (no error_string), logs at derr and calls evict_client with blocklist=true regardless of mds_session_blocklist_on_evict. Unmodified kernel and libcephfs clients only send the five client ops.

- **Class:** POSIX `internal` · exposure `client-wire` · relied on by internal
- **Map:** [wire-protocol.md › 3.11.1 MClientSession](../cpp-map/02-client-protocol/wire-protocol.md#3111-mclientsession) · [sessions.md › 3.3 Session message dispatch and state gating](../cpp-map/02-client-protocol/sessions.md#33-session-message-dispatch-and-state-gating)
- **Code:** `src/mds/Server.cc:887-893 (Server::handle_client_session)`
- **Notes:** Old kernels gate their newer ops on MDS features for this reason: REQUEST_FLUSH_MDLOG only to SERVER_LUMINOUS peers ("Pre-luminous MDS crashes when it sees an unknown session request"), GETVXATTR only with bit 17 (kclient expectations 8).

<a id="f-proto-034"></a>

#### F-PROTO-034 · Client metadata capture and use

*The key/value metadata a client sends at open, which keys the MDS interprets, and which consumers read it back.*

client_metadata_t = kv map + feature bitset + metric spec, stored on the session and journaled with it. Kernel keys: hostname, kernel_version, entity_id, root (server path or "/"). libcephfs keys: hostname, pid, entity_id, root (at mount; absent on sessions opened before mount by ceph_start_reclaim), ceph_version, ceph_sha1, client_metadata config pairs, uuid (ceph_set_uuid), timeout (ceph_set_session_timeout); ceph-fuse adds mount_point. The MDS uses root (open-time path check, dmClock volume id), uuid (duplicate check, reclaim lookup), timeout (idle and reconnect handling), ceph_version/kernel_version (feature inference) and hostname (session human name); other keys are only shown in session ls and matched by client_metadata.<key> filters. On force open the kv keys are merged without overwriting and features/metric spec replaced.

- **Class:** POSIX `operational` · exposure `client-wire`, `admin`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, admin
- **Legacy parts** (old-clients): Use of the ceph_version and kernel_version metadata keys for feature inference. The MDS reads ceph_version and kernel_version only in infer_supported_features, which runs only for sessions without a feature bitmap (kernels up to v4.18, Ceph up to 13.2.1). Baseline clients still send these keys and the dashboard shows them, but they always send a bitmap. Evidence: `src/mds/Server.cc:1773-1785 (Server::infer_supported_features)`, `src/mds/Server.cc:747-748 (Server::handle_client_session)`, `linux/fs/ceph/mds_client.c:1726 (create_session_full_msg, encode_supported_features)`, `src/client/Client.cc:2658 (Client::_open_mds_session)`.
- **Map:** [sessions.md › Client metadata keys and where the MDS uses them](../cpp-map/02-client-protocol/sessions.md#client-metadata-keys-and-where-the-mds-uses-them) · [wire-protocol.md › 3.4 Session open and feature exchange](../cpp-map/02-client-protocol/wire-protocol.md#34-session-open-and-feature-exchange)
- **Code:** `src/mds/mdstypes.h:314-356 (client_metadata_t)`, `src/mds/SessionMap.cc:1077-1098 (Session::_update_human_name)`, `linux/fs/ceph/mds_client.c:1651-1657 (create_session_full_msg)`, `src/client/Client.cc:2579-2625 (Client::populate_metadata)`
- **Notes:** mgr stats requires client_metadata.hostname and client_metadata.root in client ls output (KeyError otherwise) and parses metric_spec.metric_flags.feature_bits; volumes evicts by client_metadata.root and auth_name; the dashboard reads ceph_version, kernel_version, hostname and root (sessions 7).

<a id="f-proto-037"></a>

#### F-PROTO-037 · Duplicate REQUEST_OPEN answered with OPEN (NOTIFY_SESSION_STATE)

*A client that re-sends REQUEST_OPEN on a session the MDS already has open gets a fresh OPEN if it advertises bit 16; otherwise the request is dropped.*

If the duplicate REQUEST_OPEN advertises CEPHFS_FEATURE_NOTIFY_SESSION_STATE, the session is open and the rank is not stopping, the MDS re-sends OPEN with seq = current push_seq, supported_features (if the stored features have MIMIC) and cap_auths but no metric_spec, followed by FORCE_RO if read-only; otherwise it drops the message. Both clients copy a non-zero OPEN seq into their session seq when theirs is 0 (libcephfs only if its session was not already open), which resynchronizes the push-seq counter after a client-side session re-init; both overwrite their copy of the MDS feature bits on every OPEN.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Silent drop of a duplicate REQUEST_OPEN from sessions without bit 16; OPEN without supported_features for sessions without bit 8. The 7.3 kernel and libcephfs at this commit advertise bits 16 and 8, so a duplicate REQUEST_OPEN from them is answered with OPEN carrying supported_features. Dropping the duplicate applies only to clients without bit 16 (kernels before v6.0, Ceph before 16.2.11, 17.2.6 and 18.2.0), and omitting supported_features only to sessions without bit 8. Evidence: `src/mds/Server.cc:668-686 (Server::handle_client_session)`, `src/mds/Server.cc:674 (Server::handle_client_session, MIMIC test)`, `linux/fs/ceph/mds_client.h:54 (CEPHFS_FEATURE_NOTIFY_SESSION_STATE advertised)`, `linux-v6.0/fs/ceph/mds_client.h:46 (first kernel advertising bit 16)`, `src/client/Client.cc:448-452 (myfeatures)`.
- **Gates:** client bit CEPHFS_FEATURE_NOTIFY_SESSION_STATE (16): Linux v6.0+, Ceph 16.2.11, 17.2.6, 18.2.0+
- **Map:** [sessions.md › 3.4 Open handshake, metadata and rejection reasons](../cpp-map/02-client-protocol/sessions.md#34-open-handshake-metadata-and-rejection-reasons) · [wire-protocol.md › 3.4 Session open and feature exchange](../cpp-map/02-client-protocol/wire-protocol.md#34-session-open-and-feature-exchange)
- **Code:** `src/mds/Server.cc:663-686 (Server::handle_client_session)`, `linux/fs/ceph/mds_client.c:4601-4602 (handle_session)`, `src/client/Client.cc:2709-2726 (Client::handle_client_session)`
- **Notes:** Because this OPEN has no metric_spec and libcephfs copies both fields on every OPEN, it clears libcephfs's copy of the MDS metric spec, which stops spec-gated metric types including subvolume metrics (derived from code; wire-protocol 8). A client without bit 16 that lost the first OPEN waits.

<a id="f-proto-038"></a>

#### F-PROTO-038 · FORCE_RO: read-only MDS notification

*When the MDS metadata cache is (or turns) read-only it tells every client session, and clients then refuse new write caps locally.*

Server::force_clients_readonly sends CEPH_SESSION_FORCE_RO (v1) to every client session in open or stale when MDCache::force_readonly runs, and new and reconnected sessions get FORCE_RO right after OPEN. The kernel sets s_readonly and later cap waits needing Fw or Fx on that session's auth cap fail with -EROFS; libcephfs sets session->readonly and get_caps(Fw) on an inode whose auth cap is on that session returns -EROFS. A read-only rank also denies reconnects with CLOSE and rejects W-bit requests with -EROFS.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [sessions.md › 3.11 Session flush barrier, FLUSH_MDLOG, FORCE_RO](../cpp-map/02-client-protocol/sessions.md#311-session-flush-barrier-flush_mdlog-force_ro) · [kclient-expectations.md › 3.4.3 Session messages from the MDS](../cpp-map/02-client-protocol/kclient-expectations.md#343-session-messages-from-the-mds) · [libcephfs-fuse-ganesha-expectations.md › 3.3 Session states and session messages](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#33-session-states-and-session-messages)
- **Code:** `src/mds/Server.cc:2105-2119 (Server::force_clients_readonly)`, `src/mds/MDCache.cc:12871-12894 (MDCache::force_readonly)`, `linux/fs/ceph/caps.c:2968 (try_get_cap_refs)`, `src/client/Client.cc:4075 (Client::get_caps)`
- **Notes:** The read-only transition itself (write errors, mds_action_on_write_error) is owned by 06-journal-and-storage. See also F-STORE-100, F-ADM-042.

<a id="f-proto-039"></a>

#### F-PROTO-039 · FLUSHMSG / FLUSHMSG_ACK session barrier

*The MDS can make sure a client has processed every counted message sent so far, and flushed its pending cap releases, before it proceeds.*

Server::flush_session sends CEPH_SESSION_FLUSHMSG with seq = the session push_seq to open sessions whose connection has EXPORT_PEER and registers a waiter; the waiter completes on FLUSHMSG_ACK with seq >= that value, when the session goes stale, or when it is closed. Both clients first send their queued MClientCapRelease and then answer FLUSHMSG_ACK with the same seq. Users: subtree export (clients have seen every cap message before the export proceeds), the cross-MDS rename peer that exports the renamed inode, and the cache drop recall loop.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Legacy parts** (old-clients): flush_session returning early for connections without EXPORT_PEER. Both baseline client families negotiate EXPORT_PEER (bit 37), so every open baseline session receives FLUSHMSG. Skipping the barrier for connections without EXPORT_PEER applies only to clients older than Linux 3.14 and Ceph v0.75. Evidence: `src/mds/Server.cc:897-902 (Server::flush_session)`, `linux/include/linux/ceph/ceph_features.h:205 (EXPORT_PEER advertised)`, `src/include/ceph_features.h:141 (EXPORT_PEER, kernel 3.14)`, `src/include/ceph_features.h:220 (EXPORT_PEER in CEPH_FEATURES_ALL)`.
- **Gates:** connection feature EXPORT_PEER (37) on the client connection
- **Map:** [sessions.md › 3.11 Session flush barrier, FLUSH_MDLOG, FORCE_RO](../cpp-map/02-client-protocol/sessions.md#311-session-flush-barrier-flush_mdlog-force_ro) · [kclient-expectations.md › 3.4.3 Session messages from the MDS](../cpp-map/02-client-protocol/kclient-expectations.md#343-session-messages-from-the-mds)
- **Code:** `src/mds/Server.cc:897-923 (Server::flush_session, finish_flush_session)`, `linux/fs/ceph/mds_client.c:2147 (send_flushmsg_ack)`, `src/client/Client.cc:2779-2785 (Client::handle_client_session)`

<a id="f-proto-040"></a>

#### F-PROTO-040 · REQUEST_FLUSH_MDLOG and the client sync paths

*Clients ask the MDS to flush its journal so that unsafe requests become safe sooner; fsync, syncfs, unmount and every NFS commit or setattr through Ganesha depend on it.*

CEPH_SESSION_REQUEST_FLUSH_MDLOG (op 12, seq = the client session seq) makes the MDS call mdlog->flush() when the rank is active; there is no reply. The kernel sends it only to MDSs whose connection has SERVER_LUMINOUS: from fsync when the inode (or a directory's dir ops) has unsafe requests, to every session holding those plus the auth-cap session; from syncfs to each session with outstanding write requests; to all sessions at unmount and manual reset. fsync then waits for the safe replies of the last unsafe dir op and inode op and, if non-file metadata was dirty, for flush acks. libcephfs _fsync flushes data, sends a FLAG_SYNC cap flush, sends FLUSH_MDLOG to the ranks holding unsafe requests plus the auth rank, and waits for the newest unsafe request's safe reply and the FLUSH_ACK. FSAL_CEPH turns NFS COMMIT, stable WRITEs, ALLOCATE/DEALLOCATE and every SETATTR into ceph_ll_sync_inode/ceph_ll_fsync, so the MDS log-flush latency bounds those NFS operations.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** connection feature SERVER_LUMINOUS (21) on the MDS connection (kernel and libcephfs send the op only then)
- **Map:** [sessions.md › 3.11 Session flush barrier, FLUSH_MDLOG, FORCE_RO](../cpp-map/02-client-protocol/sessions.md#311-session-flush-barrier-flush_mdlog-force_ro) · [kclient-expectations.md › 3.4.6 FLUSH_MDLOG, sync, unmount](../cpp-map/02-client-protocol/kclient-expectations.md#346-flush_mdlog-sync-unmount) · [libcephfs-fuse-ganesha-expectations.md › 3.16.6 Writes, sync writes and fsync](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3166-writes-sync-writes-and-fsync)
- **Code:** `src/mds/Server.cc:882-886 (Server::handle_client_session)`, `linux/fs/ceph/mds_client.c:6445-6468 (send_flush_mdlog)`, `linux/fs/ceph/caps.c:2522 (ceph_fsync)`, `src/client/Client.cc:7231-7240 (Client::flush_mdlog)`, `src/client/Client.cc:13395 (Client::_fsync)`
- **Notes:** The cap-flush side of fsync (FLAG_SYNC, FLUSH_ACK) is owned by 03-caps-leases-and-locking. Derived from code: every NFS SETATTR through Ganesha waits for the MDS journal (libcephfs-fuse-ganesha 8). The op is ignored in every rank state other than up:active (including up:clientreplay). The kernel waits with wait_for_completion_timeout(r_timeout) and returns -EIO on expiry, but r_timeout is 0 (no timeout) for ordinary requests, so fsync waits until the safe reply (linux/fs/ceph/caps.c:2502-2512). See also F-NS-016 (fsync contract), F-STORE-008 (group commit).

<a id="f-proto-041"></a>

#### F-PROTO-041 · Force-opened sessions (imports, rejoin, cross-MDS rename)

*A rank can open a session on behalf of a client that never sent REQUEST_OPEN to it, when it imports caps for that client; the client receives an unsolicited OPEN.*

prepare_force_open_sessions drops blocklisted clients, gets or adds a session without connection, projects a SessionMap version, moves closed/closing/killing sessions to opening (kv metadata merged without overwriting, features and metric spec replaced) and increments importing_count; the caller journals the projected version in EImportStart, ESessions or the rename leader's EUpdate. After the journal commit, sessions still at the captured state_seq become open, are registered with metrics and dmClock, and receive OPEN (queued in preopen_out_queue until the client connects) and FORCE_RO if needed. An importing session is not closed by REQUEST_CLOSE, kill_session or idle eviction; on import failure the sessions the client never opened itself are closed again. Callers: subtree and cap import, rejoin, cross-rank rename.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [sessions.md › 3.16 Force-opened sessions (imports, rejoin, cross-MDS re...](../cpp-map/02-client-protocol/sessions.md#316-force-opened-sessions-imports-rejoin-cross-mds-rename)
- **Code:** `src/mds/Server.cc:1059-1104 (Server::prepare_force_open_sessions)`, `src/mds/Server.cc:1106-1154 (Server::finish_force_open_sessions)`, `src/mds/Server.cc:1156-1177 (Server::close_forced_opened_sessions)`
- **Notes:** The kernel opens sessions to a laggy rank's export targets and sends a reconnect to export targets in up:reconnect for which it has no session (kclient expectations 3.15). REQUEST_CLOSE on an importing session is dropped silently and the client resends it only when another counted message arrives (sessions 8). See also F-MULTI-017 (import), F-CLU-089 (rejoin), F-NS-057 (cross-rank rename).

### Session liveness, close and eviction

Renewal, staleness, autoclose and per-session timeouts, the close handshake with its push-sequence check, and every path that kills or evicts a client session.

<a id="f-proto-042"></a>

#### F-PROTO-042 · Cap renewal (REQUEST_RENEWCAPS / RENEWCAPS)

*Clients periodically renew their session; caps and dentry leases are trusted by the client only while renewals are answered.*

The client sends REQUEST_RENEWCAPS with seq = ++renew_seq. The MDS handles it in any rank state, but only for open or stale sessions: touch_session (last_cap_renew = now), a stale session goes back to open with Locker::resume_stale_caps, completed requests are trimmed by oldest_client_tid, and RENEWCAPS echoing the seq is sent immediately. The kernel renews every session_timeout/4 (15 s by default) and only to ranks in up:reconnect or later, treats caps as valid until renew_requested + session_timeout and accepts RENEWCAPS only with seq == s_renew_seq; its RENEWCAPS is a full v7 message carrying metadata and oldest_client_tid. libcephfs renews when more than session_timeout/3 elapsed and only to ranks in up:rejoin or later; its v1 RENEWCAPS carries no oldest_client_tid.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** MDSMap session_timeout (default 60 s, minimum 30; ceph fs set <fs> session_timeout)
- **Map:** [sessions.md › 3.6 Renewal, staleness, autoclose](../cpp-map/02-client-protocol/sessions.md#36-renewal-staleness-autoclose) · [kclient-expectations.md › 3.4.4 Session upkeep and timing](../cpp-map/02-client-protocol/kclient-expectations.md#344-session-upkeep-and-timing) · [libcephfs-fuse-ganesha-expectations.md › 3.3 Session states and session messages](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#33-session-states-and-session-messages)
- **Code:** `src/mds/Server.cc:829-843 (Server::handle_client_session)`, `linux/fs/ceph/mds_client.c:2116-2198 (send_renew_caps, renewed_caps)`, `src/client/Client.cc:7530-7538 (Client::renew_and_flush_cap_releases)`, `src/client/Client.cc:7766-7784 (Client::renew_caps)`
- **Notes:** Completed-request trimming for libcephfs therefore relies only on MClientRequest.head.oldest_client_tid; the table itself is owned by 04-namespace-and-posix-operations. A cap is valid in the kernel only while cap_gen equals s_cap_gen and jiffies < s_cap_ttl; libcephfs Inode::cap_is_valid applies the same rule.

<a id="f-proto-043"></a>

#### F-PROTO-043 · Stale session detection and the STALE message

*A session that has not renewed for session_timeout is marked stale, its caps are invalidated, and the client is told with CEPH_SESSION_STALE.*

Every mds_tick_interval (5 s) in clientreplay, active or stopping, find_idle_sessions uses cutoff = messenger dispatch-queue max age + session_timeout; it does nothing in a tick if the MDS itself cleared its laggy state less than cutoff ago. It walks open sessions in renewal LRU order; a later last_seen (any message) counts as liveness. Sessions past cutoff (and not handled by the autoclose, deferred-stale or timeout rules) become stale: Locker::revoke_stale_caps increments Session::cap_gen (invalidating every cap of the session) and forcibly completes in-flight revocations without write bits; if that succeeded the leases are removed, flush waiters released and STALE sent; if a revocation including write bits was pending the client is evicted instead. Clients answer STALE by bumping their cap generation, setting cap_ttl to now - 1 and sending REQUEST_RENEWCAPS; RENEWCAPS then resets caps of older generations to PIN and the MDS re-issues caps in resume_stale_caps.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** MDSMap session_timeout (60 s); mds_tick_interval (5 s)
- **Map:** [sessions.md › 3.6 Renewal, staleness, autoclose](../cpp-map/02-client-protocol/sessions.md#36-renewal-staleness-autoclose) · [kclient-expectations.md › 3.4.3 Session messages from the MDS](../cpp-map/02-client-protocol/kclient-expectations.md#343-session-messages-from-the-mds) · [libcephfs-fuse-ganesha-expectations.md › 3.3 Session states and session messages](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#33-session-states-and-session-messages)
- **Code:** `src/mds/Server.cc:1211-1359 (Server::find_idle_sessions)`, `src/mds/Locker.cc:2778-2843 (Locker::revoke_stale_caps)`, `src/mds/Locker.cc:2845-2870 (Locker::resume_stale_caps)`, `src/mds/MDSRank.cc:732-737 (MDSRankDispatcher::tick)`
- **Notes:** The LRU order is approximate because reconnect updates last_cap_renew without reordering (sessions 3.6). Stale caps are revoked without waiting for an ack when another client needs them; revoking write caps from a stale session evicts it (cap semantics owned by 03-caps-leases-and-locking). libcephfs keeps sending requests on a STALE session. See also F-CAPS-030 (cap side of stale sessions).

<a id="f-proto-044"></a>

#### F-PROTO-044 · Deferred stale marking for quiet sessions (mds_defer_session_stale)

*With the default configuration a silent session is marked stale only while the MDS is revoking caps from it or waiting for a flush from it.*

When mds_defer_session_stale (dev option, default true) is set, find_idle_sessions skips a session past the stale cutoff that has no flush waiters and from which the Locker is not revoking any caps. Such a session keeps its caps until it either becomes involved in a revocation (then it goes stale or, with timeout metadata, is evicted) or reaches session_autoclose.

- **Class:** POSIX `operational` · exposure `config` · relied on by internal
- **Gates:** mds_defer_session_stale (default true)
- **Map:** [sessions.md › 3.6 Renewal, staleness, autoclose](../cpp-map/02-client-protocol/sessions.md#36-renewal-staleness-autoclose) · [sessions.md › 7. Externally visible contract](../cpp-map/02-client-protocol/sessions.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:1256-1290 (Server::find_idle_sessions)`

<a id="f-proto-045"></a>

#### F-PROTO-045 · Session autoclose eviction

*A session silent for session_autoclose is evicted (blocklisted by default) without going through the stale state.*

In find_idle_sessions an open session whose silence span reached session_autoclose (default 300 s) is put on laggy_clients; this check runs before the custom timeout check, so it also bounds sessions with a larger or infinite timeout. Stale sessions are evicted once now - last_cap_renew >= dispatch-queue max age + session_autoclose. Each laggy client that is not importing gets a cluster-log warning "evicting unresponsive client <name>, after <span> seconds" and is evicted with blocklisting if mds_session_blocklist_on_timeout (default true), otherwise killed. The kernel marks its session HUNG (client-side bookkeeping only) when no session message arrived for session_autoclose.

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, osd
- **Gates:** MDSMap session_autoclose (default 300 s, minimum 30); mds_session_blocklist_on_timeout (default true)
- **Map:** [sessions.md › 3.6 Renewal, staleness, autoclose](../cpp-map/02-client-protocol/sessions.md#36-renewal-staleness-autoclose) · [sessions.md › 3.8 Eviction paths, blocklisting and the OSD epoch barrier](../cpp-map/02-client-protocol/sessions.md#38-eviction-paths-blocklisting-and-the-osd-epoch-barrier)
- **Code:** `src/mds/Server.cc:1297-1355 (Server::find_idle_sessions)`, `linux/fs/ceph/mds_client.c:6203-6213 (check_session_state)`

<a id="f-proto-046"></a>

#### F-PROTO-046 · Per-session timeout metadata

*A client can set session metadata "timeout" so that its session is never marked stale and is evicted only by its own timeout or by autoclose; NFS-Ganesha always sets it.*

libcephfs ceph_set_session_timeout(T) adds metadata timeout=T before the session opens (FSAL_CEPH always sends 300); the admin command session config <id> timeout [<value>] sets or removes it on any session. In find_idle_sessions it applies only after the autoclose check and, with mds_defer_session_stale, only when the MDS is revoking caps from or waiting for a flush from the session: such a session is never marked stale and is evicted directly once silent for dispatch-queue max age + T (T = 0: never by this rule). A timeout session that does not reconnect after a failover is kept in client_reclaim_gather instead of being evicted at the end of the reconnect window. The kernel never sends the key.

- **Class:** POSIX `operational` · exposure `client-wire`, `admin` · relied on by ganesha, libcephfs, admin
- **Map:** [libcephfs-fuse-ganesha-expectations.md › 3.16.3 Session timeout override](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3163-session-timeout-override) · [sessions.md › 3.6 Renewal, staleness, autoclose](../cpp-map/02-client-protocol/sessions.md#36-renewal-staleness-autoclose) · [sessions.md › 8. Edge cases, failure handling and gotchas](../cpp-map/02-client-protocol/sessions.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Server.cc:1272-1290 (Server::find_idle_sessions)`, `src/client/Client.cc:18850-18858 (Client::set_session_timeout)`, `src/mds/MDSRank.cc:3915-3949 (MDSRank::config_client)`, `nfs-ganesha/src/FSAL/FSAL_CEPH/main.c:806-815 (reclaim_reset)`
- **Notes:** Derived from code: with the default session_autoclose of 300 s the timeout=300 cutoff is never reached, so a silent Ganesha session from which the MDS revokes caps is never marked STALE and is evicted at autoclose, while an ordinary client in that situation goes STALE after session_timeout (libcephfs-fuse-ganesha 3.16.3). session config changes are not persisted until the session is written for another reason. See also F-ADM-039 (session config command).

<a id="f-proto-047"></a>

#### F-PROTO-047 · Eviction deferral while OSDs are laggy

*Optionally, the MDS skips evicting unresponsive clients while the OSDMap shows laggy OSDs, and reports them instead.*

If defer_client_eviction_on_laggy_osds (default false) is set, the OSDMap has any laggy OSD and laggy_clients is non-empty, find_idle_sessions returns without evicting; the set is kept for the MDS_CLIENTS_LAGGY health warning until the next tick clears it. The same deferral applies to cap-revoke non-responder eviction.

- **Class:** POSIX `operational` · exposure `config`, `metrics` · relied on by osd, mon, admin
- **Gates:** defer_client_eviction_on_laggy_osds (default false)
- **Map:** [sessions.md › 3.6 Renewal, staleness, autoclose](../cpp-map/02-client-protocol/sessions.md#36-renewal-staleness-autoclose)
- **Code:** `src/mds/Server.cc:1233 (Server::find_idle_sessions)`, `src/mds/Server.cc:1369 (Server::evict_cap_revoke_non_responders)`
- **Notes:** See also F-ADM-080 (MDS_CLIENTS_LAGGY).

<a id="f-proto-048"></a>

#### F-PROTO-048 · Cap-revoke non-responder eviction

*Optionally, a client that does not acknowledge a cap revocation within a timeout is evicted.*

With mds_cap_revoke_eviction_timeout set (default 0 = disabled), every tick the clients returned by Locker::get_late_revoking_clients for that timeout are evicted (blocklisted per mds_session_blocklist_on_evict) and perf counter cap_revoke_eviction is incremented; the laggy-OSD deferral applies. Not answering RECALL_STATE never evicts a client; only an unanswered revoke can, and only with this option.

- **Class:** POSIX `operational` · exposure `config`, `metrics` · relied on by kclient, libcephfs, mon, admin
- **Gates:** mds_cap_revoke_eviction_timeout (default 0, off); mds_session_blocklist_on_evict (default true)
- **Map:** [sessions.md › 3.6 Renewal, staleness, autoclose](../cpp-map/02-client-protocol/sessions.md#36-renewal-staleness-autoclose) · [libcephfs-fuse-ganesha-expectations.md › 7.1 Hard requirements an MDS must meet for unmodified lib...](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#71-hard-requirements-an-mds-must-meet-for-unmodified-libcephfs--ceph-fuse)
- **Code:** `src/mds/Server.cc:1361-1397 (Server::evict_cap_revoke_non_responders)`
- **Notes:** Ganesha delegations can hold a revoke un-acked up to the delegation timeout (F-PROTO-102). See also F-CAPS-031 (late revocation tracking).

<a id="f-proto-049"></a>

#### F-PROTO-049 · Client-initiated close with push-sequence handshake

*A client closes its session with REQUEST_CLOSE carrying its count of received counted messages; the MDS journals the close and answers CLOSE.*

REQUEST_CLOSE is handled once the rank is up:active. It is dropped if the session is closed, closing, killing or importing, dropped silently if seq < push_seq and dropped with a cluster-log warning "incorrect push seq" if seq > push_seq; with seq == push_seq journal_close_session sets closing, journals ESession(close) with the prealloc inodes to free and delegated inodes to purge, flushes the log, kills the session's in-flight requests and releases flush waiters. After the commit the MDS removes all caps and leases of the client, marks the connection disposable, sends CLOSE and removes the session. Both clients re-send REQUEST_CLOSE with the new count whenever a counted message arrives while closing. The kernel unmount waits up to mount_timeout for CLOSE, then tears sessions down locally; libcephfs waits client_shutdown_timeout (30 s) and then treats them as closed with -ETIMEDOUT. The kernel manual reset (debugfs) sends REQUEST_CLOSE, waits 100 ms and tears down locally.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [sessions.md › 3.7 Close and kill](../cpp-map/02-client-protocol/sessions.md#37-close-and-kill) · [kclient-expectations.md › 3.4.5 Push sequence (s_seq) and close](../cpp-map/02-client-protocol/kclient-expectations.md#345-push-sequence-s_seq-and-close) · [kclient-expectations.md › 3.4.7 Blocklist recovery and manual reset](../cpp-map/02-client-protocol/kclient-expectations.md#347-blocklist-recovery-and-manual-reset) · [libcephfs-fuse-ganesha-expectations.md › 3.14 Blocklisting, stale sessions and unmount](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#314-blocklisting-stale-sessions-and-unmount)
- **Code:** `src/mds/Server.cc:845-877 (Server::handle_client_session)`, `src/mds/Server.cc:1524-1561 (Server::journal_close_session)`, `src/mds/Server.cc:1012-1031 (Server::_session_logged)`, `linux/fs/ceph/mds_client.c:6786 (ceph_mdsc_close_sessions)`, `linux/fs/ceph/mds_client.c:5472 (ceph_mdsc_reset_workfn)`
- **Notes:** libcephfs ceph_set_uuid closes every session and can stall up to client_shutdown_timeout while a rank is held in clientreplay, because REQUEST_CLOSE waits for up:active (libcephfs-fuse-ganesha 8). The prealloc and purge bookkeeping of the close is owned by 06-journal-and-storage.

<a id="f-proto-050"></a>

#### F-PROTO-050 · Counted messages and the session push sequence

*Both sides count the cap, lease, snap and quota messages the MDS sends on a session; the count gates REQUEST_CLOSE and FLUSHMSG.*

The MDS sends MClientCaps, MClientLease, MClientSnap, MClientQuota and (only to clients with bit 24) MQuarantineDisable through send_message_client_counted, which increments Session::cap_push_seq. The kernel increments s_seq for every received CAPS, SNAP, LEASE and QUOTA message before decoding it (malformed or unmount-dropped ones are counted too); libcephfs Client::got_mds_push counts caps (all ops, including messages for unknown inodes), lease, snap and quota messages received on the session's current connection. Replies, forwards and session messages are not counted. The count is not persisted: both sides restart at 0 on reconnect (kernel s_seq = 0 in send_mds_reconnect, new or cleared Session on the MDS).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [sessions.md › 3.7 Close and kill](../cpp-map/02-client-protocol/sessions.md#37-close-and-kill) · [wire-protocol.md › 3.3 Message catalog](../cpp-map/02-client-protocol/wire-protocol.md#33-message-catalog) · [kclient-expectations.md › 3.4.5 Push sequence (s_seq) and close](../cpp-map/02-client-protocol/kclient-expectations.md#345-push-sequence-s_seq-and-close)
- **Code:** `src/mds/MDSRank.cc:1551-1571 (MDSRank::send_message_client_counted)`, `linux/fs/ceph/mds_client.c:6229-6246 (inc_session_sequence)`, `src/client/Client.cc:3648-3655 (Client::got_mds_push)`, `src/mds/MDCache.cc:14096-14107 (MDCache::dispatch_quiesce_inode)`
- **Notes:** Observed mismatch in this tree (derived from code, not verified by test): libcephfs advertises bit 24 but Client::handle_quarantine_disable does not call got_mds_push, so after one MQuarantineDisable its REQUEST_CLOSE carries a seq below push_seq and is dropped until client_shutdown_timeout (sessions 8, wire-protocol 8). The kernel does not advertise bit 24.

<a id="f-proto-051"></a>

#### F-PROTO-051 · Client eviction: OSD blocklist, epoch barrier and session kill

*Evicting a client blocklists its address through the monitor, waits for the resulting OSDMap, raises the OSD epoch barrier and kills the session without sending CLOSE.*

MDSRank::evict_client refuses while the rank replays ("MDS is replaying log"), logs "Evicting (and blocklisting) client session <id>" and, with blocklisting, sends the mon command osd blocklist add <addr> (no expiry argument, so mon_osd_blocklist_default_expire, 1 h); when the command completes (its result is ignored) it waits for the latest OSDMap, sets the MDS OSD epoch barrier to that epoch and calls kill_session. kill_session journals ESession(close) with state killing for opening/open/stale sessions that are not importing; after the commit the connection is marked down without a CLOSE message and caps are removed with the kill flag (an inode is flagged NEEDSRECOVER if the client had a writable range). The barrier goes out in later MClientCaps so other clients wait for that OSDMap before doing I/O. Triggers: autoclose and timeout eviction, reconnect timeout, stale write-cap revocation, cap-revoke timeout, missing required features, admin session evict/kill and client evict, reclaim RESET, oversized session metadata, unknown session op, last rank stopping. Clients learn of it through a peer reset, a REJECT with SESSION_BLOCKLISTED on reopen, or -EBLOCKLISTED from OSDs: libcephfs then aborts every request with -EBLOCKLISTED and force-closes all sessions; the kernel recovers only with recover_session=clean.

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire`, `admin` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, osd, admin, mgr
- **Gates:** mds_session_blocklist_on_timeout (default true); mds_session_blocklist_on_evict (default true); mon_osd_blocklist_default_expire (1 h, monitor)
- **Map:** [sessions.md › 3.8 Eviction paths, blocklisting and the OSD epoch barrier](../cpp-map/02-client-protocol/sessions.md#38-eviction-paths-blocklisting-and-the-osd-epoch-barrier) · [sessions.md › 3.7 Close and kill](../cpp-map/02-client-protocol/sessions.md#37-close-and-kill) · [libcephfs-fuse-ganesha-expectations.md › 3.14 Blocklisting, stale sessions and unmount](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#314-blocklisting-stale-sessions-and-unmount)
- **Code:** `src/mds/MDSRank.cc:3951-4068 (MDSRank::evict_client)`, `src/mds/Server.cc:1472-1495 (Server::kill_session)`, `src/mds/MDSRank.cc:1580-1584 (MDSRank::set_osd_epoch_barrier)`, `src/client/Client.cc:3154 (Client::handle_osd_map)`
- **Notes:** If the blocklist command failed the session is still killed after waiting for the latest OSDMap; if the session disappears meanwhile on_killed still completes (sessions 8). The OSD epoch barrier mechanism is owned by 06-journal-and-storage; libcephfs with client_reconnect_stale auto-recovers from blocklisting at most every 30 minutes with a new messenger address. The mon command is {"prefix":"osd blocklist","blocklistop":"add","addr":"<session addr>"}; it requires the osd blocklist mon cap that profile mds grants. Killing after the blocklist is in the MDS's OSDMap means size recovery of NEEDSRECOVER inodes starts only after the client is fenced. session kill is synchronous. mgr volumes and the dashboard rely on the blocklist side effect of session evict. See also F-ADM-038 (admin commands), F-STORE-098 (epoch barrier).

<a id="f-proto-052"></a>

#### F-PROTO-052 · OSDMap blocklist-driven session kill

*Client sessions whose address appears in the OSD blocklist (added by anyone) are killed when the MDS sees the new OSDMap.*

Outside replay, MDSRankDispatcher::handle_osd_map takes the newly blocklisted addresses from the Objecter and MDSRank::apply_blocklist runs Server::apply_blocklist, which kills every client session (MDS peers are skipped) whose address is blocklisted in the current OSDMap, and sets the OSD epoch barrier if anything was killed. MDSRank::reconnect_start applies the full current blocklist before the reconnect phase, and a blocklisted address gets REJECT at open.

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire` · relied on by osd, mon, admin, kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [sessions.md › 3.8 Eviction paths, blocklisting and the OSD epoch barrier](../cpp-map/02-client-protocol/sessions.md#38-eviction-paths-blocklisting-and-the-osd-epoch-barrier)
- **Code:** `src/mds/Server.cc:1497-1522 (Server::apply_blocklist)`, `src/mds/MDSRank.cc:1996-2004 (MDSRank::apply_blocklist)`, `src/mds/MDSRank.cc:3887-3913 (MDSRankDispatcher::handle_osd_map)`, `src/mds/Server.cc:728-744 (Server::handle_client_session)`
- **Notes:** Blocklisting by anyone therefore closes the client's sessions on every rank. Force-open (rejoin, import, cross-rank rename) skips blocklisted clients, and a session open from a blocklisted address gets REJECT with SESSION_BLOCKLISTED (F-PROTO-030).

<a id="f-proto-053"></a>

#### F-PROTO-053 · Eviction of sessions with oversized metadata

*A session whose encoded state grows beyond mds_session_metadata_threshold is evicted when the SessionMap is saved.*

At SessionMap save, sessions whose encoded session_info_t (mostly completed_requests) is at least mds_session_metadata_threshold (16 MiB) are skipped and evicted (blocklisted per mds_session_blocklist_on_evict); perf counter mdthresh_evicted counts them. The typical cause is a client that does not advance oldest_client_tid; the MDS warns earlier with "does not advance its oldest_client_tid" and health MDS_CLIENT_OLDEST_TID.

- **Class:** POSIX `operational` · exposure `config`, `metrics` · relied on by kclient, libcephfs, mon, admin, ganesha
- **Gates:** mds_session_metadata_threshold (16 MiB); mds_max_completed_requests (100000, warning threshold); mds_session_blocklist_on_evict (default true)
- **Map:** [sessions.md › 3.8 Eviction paths, blocklisting and the OSD epoch barrier](../cpp-map/02-client-protocol/sessions.md#38-eviction-paths-blocklisting-and-the-osd-epoch-barrier) · [libcephfs-fuse-ganesha-expectations.md › 8. Edge cases, failure handling and gotchas](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/SessionMap.cc:1262-1274 (SessionMap::apply_blocklist)`, `src/mds/SessionMap.cc:437-439 (SessionMap::validate_and_encode_session)`
- **Notes:** libcephfs never transmits oldest_client_tid in RENEWCAPS, so a long-outstanding non-SETFILELOCK request pins its table (libcephfs-fuse-ganesha 8). Counter: mds_sessions.mdthresh_evicted. See also F-STORE-075 (SessionMap object), F-NS-012.

<a id="f-proto-054"></a>

#### F-PROTO-054 · Session termination at rank shutdown

*A stopping rank closes every client session; when it is the last in rank it evicts them instead.*

Server::terminate_sessions, driven by MDCache::shutdown_pass while unclosed sessions remain, journals a close (closing) for every client session not already closing, killing or closed, so clients receive CLOSE. When the stopping rank is the only in rank, MDSRank::stopping_start first evicts every client session (blocklisting per mds_session_blocklist_on_evict), so the clients of a file system being shut down are blocklisted rather than closed.

- **Class:** POSIX `operational` · exposure `client-wire`, `cluster-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, admin, osd
- **Gates:** mds_session_blocklist_on_evict (default true)
- **Map:** [sessions.md › 3.7 Close and kill](../cpp-map/02-client-protocol/sessions.md#37-close-and-kill) · [sessions.md › 8. Edge cases, failure handling and gotchas](../cpp-map/02-client-protocol/sessions.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Server.cc:1187-1208 (Server::terminate_sessions)`, `src/mds/MDSRank.cc:2259-2281 (MDSRank::stopping_start)`
- **Notes:** The up:stopping state itself is owned by 01-cluster-membership-and-ha. The eviction trigger is "the only rank in" (get_num_in_mds() == 1, for example `fs set down true` or max_mds reduced to 0), not specifically rank 0; clients must remount after the file system is brought back. See also F-CLU-102 (stopping).

### MDS failover reconnect and NFS-Ganesha reclaim

How clients rebuild their state on a replacement MDS (reconnect) and how a restarted NFS-Ganesha removes the session of its previous incarnation (reclaim).

<a id="f-proto-055"></a>

#### F-PROTO-055 · Reconnect trigger and client-side ordering

*When a rank is taken over by a new daemon, clients replay unsafe requests, resend unanswered ones, re-send revoked cap flushes and then describe all their caps in MClientReconnect, in that order on the new connection.*

The MDS enters up:reconnect with MDSRank::reconnect_start: it applies the OSD blocklist, puts every open session (all sessions loaded or replayed are open) into client_reconnect_gather, marks them reconnecting and sets last_cap_renew = now. The kernel notices the rank's new address in the MDSMap, closes the connection, marks the session RESTARTING and, once the rank state is up:reconnect or later, resets s_seq to 0, bumps the cap generation, drops queued cap releases and delegated inodes, trims the dcache, re-sends unsafe requests with REPLAY, re-sends requests that were sent but not answered (as retries without releases), re-sends flushes of revoked flushing caps with seq = issue_seq = mseq = 0, and finally sends MClientReconnect. libcephfs reconnects when the rank state becomes exactly up:reconnect, in the same order. After the rank becomes active, clients kick waiting requests and in-flight cap flushes and reset requested max_size.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [sessions.md › 3.10 Reconnect protocol (MDS failover)](../cpp-map/02-client-protocol/sessions.md#310-reconnect-protocol-mds-failover) · [kclient-expectations.md › 3.15 Reconnect after an MDS failover](../cpp-map/02-client-protocol/kclient-expectations.md#315-reconnect-after-an-mds-failover) · [libcephfs-fuse-ganesha-expectations.md › 3.13 MDS failover, reconnect and replay](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#313-mds-failover-reconnect-and-replay)
- **Code:** `src/mds/MDSRank.cc:2007-2030 (MDSRank::reconnect_start)`, `src/mds/Server.cc:1563-1589 (Server::reconnect_clients)`, `linux/fs/ceph/mds_client.c:5145-5359 (send_mds_reconnect)`, `src/client/Client.cc:3438-3528 (Client::send_reconnect)`
- **Notes:** libcephfs closes the session and drops caps (dirty metadata lost) if it sees the rank jump from below up:reconnect to beyond it (libcephfs-fuse-ganesha 8). The kernel also sends a reconnect to export targets in up:reconnect for which it has no session. See also F-CLU-080 (reconnect window and timeout).

<a id="f-proto-056"></a>

#### F-PROTO-056 · MClientReconnect encodings (v1 to v5, multi-message)

*Layout of the reconnect message that carries every cap, realm and file lock a client holds from a rank.*

The whole payload is in the data segment. v4/v5 (compat 4): map<inodeno_t, cap_reconnect_t> caps, vector<snaprealm_reconnect_t> realms, v5 bool more (another reconnect message follows). cap_reconnect_t = ENCODE_START(2,1) {string path, raw 36-byte ceph_mds_cap_reconnect {u64 cap_id, u32 wanted, u32 issued, u64 snaprealm, u64 pathbase, u32 flock_len}, flock_len bytes of lock blob, v2 u64 snap_follows}; snaprealm_reconnect_t = ENCODE_START(1,1) {u64 ino, u64 seq, u64 parent}. v3: same caps map, raw 24-byte realm records until the end of the buffer; v2: u32 n + (ino, old cap record without header), realms raw; v1: map<ino, old_cap_reconnect_t> with the pre-FLOCK record. A session's reconnect completes only with a message without more. The kernel sends v3 (v2 when the previous connection lacked MDSENC and the MDS lacked bit 12) and switches to v5 multi-message only when the state exceeds INT_MAX - PAGE_SIZE; libcephfs sends v5 to an MDS with bit 12 and splits at about INT_MAX/2.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Decoding MClientReconnect v1 (old_cap_reconnect_t map) and v2 (count plus old cap records); client paths towards an MDS without bit 12 (kernel v2 selection and -ENOSPC above RECONNECT_MAX_SIZE, libcephfs single-message v1-v3 without realms). Baseline clients send v3 (the 7.3 kernel, one message unless the state exceeds INT_MAX - PAGE_SIZE, then v5 parts) and v5 (libcephfs towards an MDS with bit 12), so v3 and v5 decoding are current. v1 and v2 come only from kernels before v4.8 and from clients talking to an MDS that lacks bit 12 and MDSENC or FLOCK. The kernel's -ENOSPC limit and the libcephfs single-message encoder apply only towards an MDS without bit 12, which the baseline MDS advertises. Evidence: `src/messages/MClientReconnect.h:101-108 (MClientReconnect::encode_payload)`, `linux/fs/ceph/mds_client.c:5241-5248 (send_mds_reconnect)`, `linux/fs/ceph/mds_client.c:5272-5275 (send_mds_reconnect, -ENOSPC)`, `src/client/Client.cc:3460 (Client::send_reconnect, allow_multi)`, `src/client/Client.cc:3520-3521 (Client::send_reconnect, set_encoding_version(0))`, `linux-v4.5/fs/ceph/mds_client.c:3020-3021 (send_mds_reconnect, version 2)`, `linux-v4.8/fs/ceph/mds_client.c:2957-2962 (send_mds_reconnect, v1/v2/v3)`. Also: src/messages/MClientReconnect.h:151-163 (MClientReconnect::decode_payload, v2; v1).
- **Gates:** CEPHFS_FEATURE_MULTI_RECONNECT (12) in the MDS bitmap: Linux v5.1+, Ceph 14.2.0+; connection feature MDSENC (29) for v3
- **Map:** [wire-protocol.md › 3.11.7 MClientReconnect](../cpp-map/02-client-protocol/wire-protocol.md#3117-mclientreconnect) · [sessions.md › 5. Interfaces](../cpp-map/02-client-protocol/sessions.md#5-interfaces) · [kclient-expectations.md › 3.15 Reconnect after an MDS failover](../cpp-map/02-client-protocol/kclient-expectations.md#315-reconnect-after-an-mds-failover)
- **Code:** `src/messages/MClientReconnect.h:100-170 (MClientReconnect::encode_payload, decode_payload)`, `src/mds/mdstypes.cc:1184-1212 (cap_reconnect_t::encode, decode)`, `linux/fs/ceph/mds_client.c:4766-4840 (send_reconnect_partial)`, `src/client/Client.cc:3460-3475 (Client::send_reconnect)`
- **Notes:** Generations (kclient expectations 7.1.2): kernels before v4.8 send v2 (v1 towards an MDS without FLOCK); v4.8 and later send v3; no kernel sends v4; every kernel v4.8-7.3 normally sends one v3 message. The kernel chooses the version from the previous daemon's OPEN bits and the previous connection's features. Ceph 13.x and older send one v3 message; 14.2.0+ send v5. Derived from code: since 14.1.0 the libcephfs single-message encoder writes realm records only in the v1 branch, so its v2/v3 reconnects carry no realms. Without bit 12 a kernel reconnect larger than RECONNECT_MAX_SIZE fails locally with -ENOSPC.

<a id="f-proto-057"></a>

#### F-PROTO-057 · Reconnect processing and outcomes

*How the MDS answers each reconnect: OPEN on success, CLOSE when it denies, REJECT when there is no session.*

Server::handle_client_reconnect: no session gives REJECT "sessionless"; refuse_client_session gives CLOSE with error_string; a session that is not open gets CLOSE; a reconnect that arrives while the rank wants up:reconnect but is not yet in it (MDSMap delivery race) waits until the rank is in up:reconnect; mds_deny_all_reconnect, a rank no longer in (or wanting) reconnect, timeout evictions in progress (reconnect_evicting), a read-only MDS or missing required_client_features (after inferring features from version metadata for old clients) give CLOSE (denied) with the cluster-log line "denied reconnect attempt ...". On the last part (no `more` flag) the MDS registers the session with metrics and dmClock and sends OPEN (supported features and metric spec for Mimic+ clients, cap_auths). The per-cap and per-realm processing of accepted messages is F-CLU-082. A denied client that was open stays in the reconnect gather and is evicted (blocklisted by default) when the window times out. On OPEN the kernel calls it "reconnect success"; on CLOSE it drops all caps and dirty state (-EIO on mappings) and its unsafe requests, and restarts requests on a new session; libcephfs closes the session itself if it sees the rank past up:reconnect without having seen up:reconnect.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Legacy parts** (old-clients): infer_supported_features at reconnect; reconnect OPEN encoded as v1 for sessions without bit 8. Reconnect admission and outcomes are current for baseline sessions. Inferring features from ceph_version or kernel_version before the required-features test runs only for sessions stored without a bitmap (kernels up to v4.18, Ceph up to 13.2.1). The reconnect OPEN without supported_features and metric_spec goes only to sessions without bit 8. Evidence: `src/mds/Server.cc:1655-1656 (Server::handle_client_reconnect, infer)`, `src/mds/Server.cc:1694-1697 (Server::handle_client_reconnect, OPEN)`, `linux/fs/ceph/mds_client.c:1726 (create_session_full_msg, encode_supported_features)`, `src/client/Client.cc:2658 (Client::_open_mds_session)`.
- **Gates:** mds_deny_all_reconnect (default false); MDSMap flag refuse_client_session; required_client_features
- **Map:** [sessions.md › 3.10 Reconnect protocol (MDS failover)](../cpp-map/02-client-protocol/sessions.md#310-reconnect-protocol-mds-failover) · [kclient-expectations.md › 3.15 Reconnect after an MDS failover](../cpp-map/02-client-protocol/kclient-expectations.md#315-reconnect-after-an-mds-failover)
- **Code:** `src/mds/Server.cc:1591-1768 (Server::handle_client_reconnect)`, `src/mds/Server.cc:1770-1797 (Server::infer_supported_features)`
- **Notes:** Cap reconciliation in rejoin is owned by 01-cluster-membership-and-ha (resolve-and-rejoin page). The kernel clears dir-op cap bits from directory caps before reconnecting, so reconnect cannot seed Fc/Fr (kclient expectations 8). See also F-CLU-080 (reconnect window), F-CAPS-034 (cap side).

<a id="f-proto-061"></a>

#### F-PROTO-061 · Snap realm reconnect

*Clients report the snap realms they know in the reconnect message, and the MDS catches up clients whose realm sequence is old.*

Realm records carry {ino, seq, parent}. The kernel sends every snap realm it knows; libcephfs sends only the realms of inodes that hold a cap from this rank (and, derived from code, none in its single-message v2/v3 reconnects). The MDS skips realms whose inode is being purged and records the others with add_reconnected_snaprealm; a client whose realm seq is older gets an MClientSnap UPDATE in MDCache::finish_snaprealm_reconnect.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [sessions.md › 3.10 Reconnect protocol (MDS failover)](../cpp-map/02-client-protocol/sessions.md#310-reconnect-protocol-mds-failover) · [wire-protocol.md › 3.11.5 MClientSnap](../cpp-map/02-client-protocol/wire-protocol.md#3115-mclientsnap)
- **Code:** `src/mds/Server.cc:1591-1768 (Server::handle_client_reconnect)`, `src/mds/MDCache.cc:6151 (MDCache::finish_snaprealm_reconnect)`, `linux/fs/ceph/mds_client.c:5074-5132 (encode_snap_realms)`
- **Notes:** Snap realm semantics are owned by 08-snapshots-quotas-subvolumes. See also F-SNAP-032 (reconciliation at reconnect and rejoin).

<a id="f-proto-062"></a>

#### F-PROTO-062 · Client reactions to MDSMap rank changes

*What clients do when a rank's state, address or presence changes in the MDSMap, which defines when they talk to a rank.*

Kernel check_new_map: a rank beyond the new possible max is force-closed (unsafe requests dropped, caps removed, requests kicked); an address change closes the connection and sets RESTARTING; RESTARTING with state up:reconnect or later triggers the reconnect; a transition from below up:active to up:active or later kicks new requests and in-flight flushes and resets max_size requests; a laggy rank makes the kernel open sessions to its export targets. New requests go only to ranks in up:active or later (up:clientreplay is not enough), renewals from up:reconnect on, metrics only to up:active. MDSMap entries with state <= 0 (creating, starting, standby-replay) are invisible to the kernel. libcephfs handle_mds_map marks the connection down for a down rank, sends reconnect at exactly up:reconnect, closes the session on a missed reconnect, kicks at active or stopping, and sends requests to active or stopping ranks.

- **Class:** POSIX `supporting` · exposure `cluster-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon
- **Map:** [kclient-expectations.md › 3.16 Reactions to MDSMap changes (check_new_map)](../cpp-map/02-client-protocol/kclient-expectations.md#316-reactions-to-mdsmap-changes-check_new_map) · [kclient-expectations.md › 3.3 MDSMap fields the kclient consumes](../cpp-map/02-client-protocol/kclient-expectations.md#33-mdsmap-fields-the-kclient-consumes) · [libcephfs-fuse-ganesha-expectations.md › 3.13 MDS failover, reconnect and replay](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#313-mds-failover-reconnect-and-replay)
- **Code:** `linux/fs/ceph/mds_client.c:5843-6014 (check_new_map)`, `linux/fs/ceph/mds_client.c:3687-3699 (__do_request)`, `src/client/Client.cc:3360-3420 (Client::handle_mds_map)`
- **Notes:** The MDS influences these reactions only through what its beacons put in the MDSMap (owned by 01-cluster-membership-and-ha). The kernel MDSMap decoder and its quirks are in kclient expectations 3.3. See also F-CLU-006 (kernel MDSMap decode).

<a id="f-proto-063"></a>

#### F-PROTO-063 · Session reclaim (MClientReclaim with CEPH_RECLAIM_RESET)

*A restarted NFS-Ganesha (new libcephfs instance, new global id) destroys the MDS session of its previous incarnation, identified by a uuid, so the dead instance's caps and locks are released immediately.*

MClientReclaim = string uuid, u32 flags (CEPH_RECLAIM_RESET = 1, FLAG_FINISH = 1<<31); MClientReclaimReply = i32 result (raw host errno), u32 epoch (OSDMap epoch after the eviction), entity_addrvec_t addrs (old session address). The MDS ignores a reclaim without session, drops it without MAY_READ fs caps, waits for up:clientreplay, drops it if the requesting session is not open or stale, answers -EINVAL for an empty uuid or flags other than exactly RESET (FLAG_FINISH combined with another flag gives -EINVAL). A session whose metadata uuid matches is checked for the same cephx auth_name as the requester (-EPERM otherwise); it is killed (if already blocklisted or mds_session_blocklist_on_evict is false) or evicted with blocklisting, and the reply is sent when that completes. No match gives result 0, no address, epoch 0. libcephfs ceph_start_reclaim opens a session (metadata timeout only, no root) to every rank 0..num_in-1 in turn, requires RECLAIM_CLIENT in that rank's OPEN (-EOPNOTSUPP), sends one reclaim per rank and returns -ENOENT if no rank returned addresses; after a RESET reclaim ceph_finish_reclaim sends nothing; ceph_set_uuid then closes all sessions so that the mount reopens them with the uuid. A second session opening with a uuid still owned by a live session is rejected. auth_name is journaled in ESession so a session recreated by replay can still be reclaimed.

- **Class:** POSIX `operational` · exposure `client-wire` · relied on by ganesha, libcephfs
- **Legacy parts** (dead-code): Non-RESET reclaim handling: the ceph_assert(0) branch of Server::reclaim_session, the FLAG_FINISH path of Server::handle_client_reclaim and the reclaiming_from pairing it would complete. NFS-Ganesha V15.7 always calls ceph_start_reclaim with CEPH_RECLAIM_RESET, and that path is current. The MDS answers -EINVAL to any flags other than exactly RESET, so the non-RESET branch of Server::reclaim_session (ceph_assert(0)) cannot run. libcephfs sends FLAG_FINISH only after a successful non-RESET reclaim, so baseline clients never send it, and because the RESET path clears reclaiming_from at once the FLAG_FINISH handler would find no pending reclaim. Evidence: `src/mds/Server.cc:492-497 (Server::reclaim_session, -EINVAL)`, `src/mds/Server.cc:518-520 (Server::reclaim_session, ceph_assert(0))`, `src/mds/Server.cc:523-527 (Server::finish_reclaim_session)`, `src/mds/Server.cc:584-593 (Server::handle_client_reclaim, FLAG_FINISH)`, `src/mds/Server.cc:464-470 (Server::find_session_by_uuid, reclaiming_from pairs)`, `src/client/Client.cc:18936-18937 (Client::start_reclaim)`, `src/client/Client.cc:18961-18976 (Client::finish_reclaim)`, `nfs-ganesha/src/FSAL/FSAL_CEPH/main.c:834 (reclaim_reset)`, `nfs-ganesha/src/FSAL/FSAL_CEPH/main.c:882-883`.
- **Gates:** MDS bit CEPHFS_FEATURE_RECLAIM_CLIENT (10), Ceph 14.2.0+; no kernel advertises it; mds_session_blocklist_on_evict (default true)
- **Map:** [sessions.md › 3.9 Reclaim protocol (NFS-Ganesha via libcephfs)](../cpp-map/02-client-protocol/sessions.md#39-reclaim-protocol-nfs-ganesha-via-libcephfs) · [wire-protocol.md › 3.11.8 MClientReclaim / MClientReclaimReply](../cpp-map/02-client-protocol/wire-protocol.md#3118-mclientreclaim--mclientreclaimreply) · [libcephfs-fuse-ganesha-expectations.md › 3.16.2 Session reclaim (grace-period takeover)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3162-session-reclaim-grace-period-takeover) · [libcephfs-fuse-ganesha-expectations.md › 3.16.9 MDS paths that only Ganesha exercises](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3169-mds-paths-that-only-ganesha-exercises)
- **Code:** `src/mds/Server.cc:454-597 (Server::find_session_by_uuid, reclaim_session, finish_reclaim_session, handle_client_reclaim)`, `src/messages/MClientReclaim.h:26-49 (MClientReclaim)`, `src/messages/MClientReclaimReply.h:40-53 (MClientReclaimReply)`, `src/client/Client.cc:18861-19004 (Client::start_reclaim, finish_reclaim, handle_client_reclaim_reply)`
- **Notes:** FSAL_CEPH (nfs-ganesha V15.7) runs ceph_set_session_timeout(300), ceph_start_reclaim(uuid, RESET), ceph_finish_reclaim, ceph_set_uuid at every cmount creation (daemon start and every new export key) and reclaims on rados_cluster nodeid takeover; the uuid is ganesha-0x<CityHash64(nodeid, user, fs, cmount path)>. Code observation: libcephfs reclaim results are sticky per client, so a later reclaim for an unknown uuid returns 0 instead of -ENOENT once one matched (libcephfs-fuse-ganesha 8). The non-RESET path is unreachable because the MDS rejects other flags. See also F-CLU-096 (clientreplay held for pending reclaims).

### Request encoding, routing and forwarding

The MClientRequest wire format with all its versions, how clients pick a rank, and how a request is redirected, retried and identified.

<a id="f-proto-065"></a>

#### F-PROTO-065 · MClientRequest versioned head (head v1, v2, v3)

*The fixed request head every metadata request starts with, in the three versioned forms current clients send.*

MClientRequest is message v6/compat 1; for message version 4 and above the head is the versioned ceph_mds_request_head: u16 version, u64 oldest_client_tid, u32 mdsmap_epoch (ignored by the MDS), u32 flags, u8 num_retry, u8 num_fwd, u16 num_releases, u32 op, u32 caller_uid, u32 caller_gid, u64 ino, 56-byte args union (v1, 98 bytes); u32 ext_num_retry, u32 ext_num_fwd (v2, 106 bytes); u32 struct_len, u32 owner_uid, u32 owner_gid (v3, 118 bytes). The decoder skips unknown trailing head bytes by struct_len and throws if it consumed more; for v1/v2 heads it sets owner = caller. Clients send head v1 if the MDS lacks 32BITS_RETRY_FWD, v2 if it lacks HAS_OWNER_UIDGID, else v3; owner_uid/gid carry the (idmapped) fsuid/fsgid for CREATE, MKNOD, MKDIR and SYMLINK and -1 otherwise; caller_uid/gid are used for MDS cap checks. The kernel uses message version 4 for head v1 and 6 for head v2/v3.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Decoding head v1 and v2 (ext counters from the 8-bit fields, owner_uid/gid set to caller_uid/gid); the kernel's message v4 labelling for head v1. Towards an MDS with bits 18 and 20 the 7.3 kernel sends message v6 with head v3 and libcephfs at this commit sends head v3, so head v3 decoding is current. Head v1 comes from kernels v5.11-v6.5, CentOS Stream 8 and Ceph up to 16.2.13 and 17.2.6, and head v2 from Linux v6.6 and Ceph 16.2.14-16.2.15, 17.2.7 and 18.1.0-18.2.0. Current clients choose head v1 or v2 only towards an MDS without bit 18 or 20. Evidence: `src/common/ceph_fs.cc:74-77 (decode(ceph_mds_request_head&), v1 counters)`, `src/common/ceph_fs.cc:85-92 (decode(ceph_mds_request_head&), owner = caller)`, `linux/fs/ceph/mds_client.c:3160-3169 (mds_supported_head_version)`, `src/messages/MClientRequest.h:321-327 (MClientRequest::encode_payload)`, `linux-v6.6/fs/ceph/mds_client.h:51 (first kernel advertising bit 18)`, `cs8-kernel/fs/ceph/mds_client.c:2578-2589 (message v4 with head v1)`. Also: linux/fs/ceph/mds_client.c:3354-3366 (create_request_message, message v4; v6).
- **Gates:** MDS bit CEPHFS_FEATURE_32BITS_RETRY_FWD (18) selects head v2+; MDS bit CEPHFS_FEATURE_HAS_OWNER_UIDGID (20) selects head v3; kernel module parameter enable_unsafe_idmap
- **Map:** [wire-protocol.md › 3.7 MClientRequest encoding](../cpp-map/02-client-protocol/wire-protocol.md#37-mclientrequest-encoding) · [wire-protocol.md › 3.7.1 ceph_mds_request_head layout (ceph_fs.h:688-706, co...](../cpp-map/02-client-protocol/wire-protocol.md#371-ceph_mds_request_head-layout-ceph_fsh688-706-codec-ceph_fscc14-114) · [kclient-expectations.md › 3.6.3 MClientRequest encoding](../cpp-map/02-client-protocol/kclient-expectations.md#363-mclientrequest-encoding) · [libcephfs-fuse-ganesha-expectations.md › 3.4 Request lifecycle (client side)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#34-request-lifecycle-client-side)
- **Code:** `src/common/ceph_fs.cc:14-114 (ceph_mds_request_head encode and decode)`, `src/include/ceph_fs.h:688-706 (ceph_mds_request_head)`, `src/messages/MClientRequest.h:267-346 (MClientRequest::decode_payload, encode_payload)`, `linux/fs/ceph/mds_client.c:3160-3169 (mds_supported_head_version)`, `src/mds/Server.cc:3721-3734 (Server::prepare_new_inode)`
- **Notes:** Generations (kclient expectations 7.1.2): head v1 from kernels v5.11-v6.5 (and later kernels towards an MDS without bit 18), v2 from v6.6, v3 from v6.7; Ceph v1 up to 16.2.13 and 17.2.6, v2 in 16.2.14-16.2.15, 17.2.7, 18.1.0-18.2.0, v3 from 17.2.8, 18.2.1, 19.2.0. On an idmapped mount a kernel session without bit 20 fails CREATE/MKNOD/MKDIR/SYMLINK with -EIO unless enable_unsafe_idmap is set. For new-inode ops an owner uid (or gid when the parent is not setgid) of -1 trips ceph_assert in Server::prepare_new_inode, whatever the head version.

<a id="f-proto-067"></a>

#### F-PROTO-067 · Request message tail: stamp, gid_list, alternate_name, fscrypt fields

*Fields that follow the paths and release records of a request, added over message versions 2 to 6.*

After head, path, path2 and the release records come: v2 utime stamp (the client's operation time, which becomes ctime/mtime/atime/btime of the mutation; MDS time if zero); v4 vector<u64> gid_list (supplementary groups for MDS cap checks); v5 string alternate_name (fscrypt ciphertext of long names, or the original spelling of a charmap name; longer than mds_alternate_name_max, 8 KiB, gives -ENAMETOOLONG; returned in LeaseStat v2); v6 vector<u8> fscrypt_auth and vector<u8> fscrypt_file. Trailing bytes are ignored. The kernel writes the complete v6 tail even in messages it labels v3 or v4, and the MDS reads only what the header version covers, so with message version 4 alternate_name and the fscrypt fields are ignored and with version 3 also gid_list.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Decoding request messages v2-v5 (tail cut after stamp, gid_list or alternate_name), including v3/v4-labelled kernel messages whose extra tail fields the MDS ignores. The 7.3 kernel and libcephfs at this commit send MClientRequest v6 to an MDS with bits 18 and 20, so the full tail (stamp, gid_list, alternate_name, fscrypt_auth, fscrypt_file) is current. Message versions 2-5 come from kernels up to v6.5 (v2 up to v5.10, v3/v4 from v5.11, also CentOS Stream 8) and Ceph clients up to 17.x, or from current kernels towards an MDS without bit 18. Evidence: `src/messages/MClientRequest.h:300-309 (MClientRequest::decode_payload)`, `src/messages/MClientRequest.h:92 (HEAD_VERSION 6)`, `linux/fs/ceph/mds_client.c:3367-3375 (create_request_message, message v6)`, `linux-v5.10/fs/ceph/mds_client.c:2540 (message v2)`, `cs8-kernel/fs/ceph/mds_client.c:2578-2589 (message v3/v4)`.
- **Gates:** mds_alternate_name_max (8 KiB)
- **Map:** [wire-protocol.md › 3.7 MClientRequest encoding](../cpp-map/02-client-protocol/wire-protocol.md#37-mclientrequest-encoding) · [kclient-expectations.md › 3.6.3 MClientRequest encoding](../cpp-map/02-client-protocol/kclient-expectations.md#363-mclientrequest-encoding) · [kclient-expectations.md › 3.19 fscrypt and alternate names](../cpp-map/02-client-protocol/kclient-expectations.md#319-fscrypt-and-alternate-names)
- **Code:** `src/messages/MClientRequest.h:300-309 (MClientRequest::decode_payload)`, `linux/fs/ceph/mds_client.c:3124 (encode_mclientrequest_tail)`, `src/mds/MDCache.cc:9845 (MDCache::request_start)`
- **Notes:** Generations: kernels up to v5.10 send message v2 without gid_list, so they never send supplementary groups; v5.11-v6.5 send v3/v4; v6.6+ the full v6 tail. Ceph sends v3 (10.2.x), v4 (11.x-15.x), v5 (16.x-17.x), v6 (18.x+) (kclient expectations 7.1.2). fscrypt semantics (fscrypt_file size rules, last-block truncate) are owned by 04-namespace-and-posix-operations.

<a id="f-proto-068"></a>

#### F-PROTO-068 · Request flags REPLAY, WANT_DENTRY and ASYNC

*Three head flags that change how the MDS treats a request.*

CEPH_MDS_FLAG_REPLAY (1) is set on a resend of a request that already got an unsafe reply: no early reply, embedded releases ignored, a completed-request hit gives a traceless safe reply, queued for clientreplay in up:reconnect, and the stray-directory space check skipped. CEPH_MDS_FLAG_WANT_DENTRY (2) (kernel whenever r_parent is set; libcephfs when the request has a dentry and always for LOOKUP/LOOKUPSNAP) makes the reply include the dentry trace, including the null dentry on -ENOENT, and turns a completed CREATE retry into LOOKUP instead of GETATTR. CEPH_MDS_FLAG_ASYNC (4) marks kernel async create/unlink; in up:reconnect it is queued like a replay, otherwise processed normally.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.7.2 Request flags (ceph_fs.h:584-586)](../cpp-map/02-client-protocol/wire-protocol.md#372-request-flags-ceph_fsh584-586)
- **Code:** `src/include/ceph_fs.h:584-586 (CEPH_MDS_FLAG_*)`, `src/mds/Server.cc:2318-2328 (Server::early_reply)`, `src/mds/Server.cc:349-372 (Server::dispatch)`
- **Notes:** The kernel sets ASYNC only when the directory cap has Fx|Dc or Fx|Dr, which the C++ MDS at this commit never grants, so no async request reaches it (leases-lock-caches-and-async-dirops 8, G1). libcephfs never sets ASYNC.

<a id="f-proto-069"></a>

#### F-PROTO-069 · Request argument union per op

*The 56-byte args union in the request head and which ops use which member.*

Members (offset:size): getattr {mask} for GETATTR, LOOKUP, LOOKUPSNAP; setattr {mode, uid, gid, mtime, atime, size, old_size, mask (CEPH_SETATTR_* bits 0-14), btime} for SETATTR (uid/gid are also passed to every MDS cap check as the new owner); readdir {frag, max_entries (0 = no limit), max_bytes (0 = 512 KiB + max xattr size), flags, offset_hash} for READDIR and LSSNAP; mknod {mode, rdev}; mkdir {mode}; open {flags (CEPH_O_*), mode, stripe_unit, stripe_count, object_size, pool (only with CREATEPOOLID), mask, old_size} for OPEN and CREATE; setxattr {flags (CREATE/REPLACE/REMOVE/REMOVE2), osdmap_epoch}; setlayout {raw 28-byte ceph_file_layout}; filelock_change {rule, type, owner, pid, start, length, wait} for SETFILELOCK/GETFILELOCK; lookupino {mask, snapid, parent, hash}; snapdiff {frag, max_entries, max_bytes, flags, offset_hash, snap_other, mask}; blockdiff {scan_idx, max_objects}.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.7.3 Argument union per op (ceph_fs.h:605-680)](../cpp-map/02-client-protocol/wire-protocol.md#373-argument-union-per-op-ceph_fsh605-680)
- **Code:** `src/include/ceph_fs.h:605-680 (ceph_mds_request_args)`
- **Notes:** Open flags are translated to CEPH_O_* (RDONLY 0, WRONLY 1, RDWR 2, CREAT 0100, EXCL 0200, TRUNC 01000, DIRECTORY 0200000, NOFOLLOW 0400000); O_APPEND, O_SYNC, O_DSYNC, O_DIRECT never reach the MDS. Server::check_access reads args.setattr.uid/gid for every op, i.e. unrelated union bytes for non-setattr ops (wire-protocol 8). Op semantics are owned by 04-namespace-and-posix-operations.

<a id="f-proto-070"></a>

#### F-PROTO-070 · Embedded cap and dentry-lease release records

*Clients piggy-back cap and dentry-lease releases on a request so the MDS does not have to revoke them first.*

head.num_releases records follow the two paths: raw ceph_mds_request_release (ino, cap_id, caps, wanted, seq, issue_seq, mseq, dname_seq, dname_len; 44 bytes) followed by dname_len name bytes without a length prefix. A record with a name releases the dentry lease on (ino = parent dir, dname) and then the cap on ino. The MDS processes them with Locker::process_request_cap_release before dispatch, only for non-replay client requests; records with a stale mseq or another cap_id are ignored, and they are cleared for retried requests arriving in up:reconnect. The kernel drops the released bits locally before sending; libcephfs always emits a record for the parent directory's cap on a dentry release. Both clients strip the records on replays and on resends during reconnect.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.7.4 Release records (ceph_mds_request_release, ceph_fs....](../cpp-map/02-client-protocol/wire-protocol.md#374-release-records-ceph_mds_request_release-ceph_fsh712-718) · [kclient-expectations.md › 3.6.3 MClientRequest encoding](../cpp-map/02-client-protocol/kclient-expectations.md#363-mclientrequest-encoding) · [libcephfs-fuse-ganesha-expectations.md › 3.4 Request lifecycle (client side)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#34-request-lifecycle-client-side)
- **Code:** `src/include/ceph_fs.h:712-718 (ceph_mds_request_release)`, `src/mds/Locker.cc:3714-3766 (Locker::process_request_cap_release)`, `linux/fs/ceph/caps.c:4934 (ceph_encode_inode_release)`, `src/client/Client.cc:2446-2549 (Client::encode_cap_releases)`
- **Notes:** Cap and lease semantics are owned by 03-caps-leases-and-locking. See also F-CAPS-029 (MDS processing).

<a id="f-proto-071"></a>

#### F-PROTO-071 · Request data segment per op

*Ops that carry a blob in the message data segment, and its format.*

CREATE, MKNOD, MKDIR and SYMLINK: optional initial xattrs, u32 count + (string key, u32 len + value) (the kernel sends system.posix_acl_access/default from ceph_pre_init_acls and the LSM security label; libcephfs ACL blobs), decoded by Server::prepare_new_inode as the new inode's xattr map without a try block. SETXATTR: the xattr value (for ceph.* vxattrs the textual value). SETATTR with CEPH_SETATTR_SIZE on an fscrypt file: ENCODE_START(1) {u64 change_attr, u64 file_offset, u64 block_size} followed by the last block, at most mds_fscrypt_last_block_max_size (4 KiB) plus header (-EINVAL if larger); a change_attr that differs from the inode gives -EAGAIN. MKSNAP: SnapPayload ENCODE_START(1,1) {map<string,string> metadata}, undecodable payload ignored. SNAP_METADATA: string key, string value, u32 op_flag; undecodable gives -EBADMSG.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_fscrypt_last_block_max_size (4 KiB)
- **Map:** [wire-protocol.md › 3.7.5 Data segment (req->get_data()) per op](../cpp-map/02-client-protocol/wire-protocol.md#375-data-segment-req-get_data-per-op) · [kclient-expectations.md › 3.6.4 Ops the kclient issues](../cpp-map/02-client-protocol/kclient-expectations.md#364-ops-the-kclient-issues)
- **Code:** `src/mds/Server.cc:3748-3755 (Server::prepare_new_inode)`, `src/mds/Server.cc:5643-5687 (Server::handle_client_setattr)`, `linux/fs/ceph/acl.c:165 (ceph_pre_init_acls)`, `src/mds/Server.cc:12099-12110 (Server::handle_client_snap_md_op)`
- **Notes:** The kernel retries an fscrypt truncate up to 20 times on -EAGAIN; libcephfs does not retry and the error reaches the application (libcephfs-fuse-ganesha 8).

<a id="f-proto-072"></a>

#### F-PROTO-072 · Path encoding on the wire

*How clients name the object of a request: an inode number plus a relative path, with snapshot directories encoded as empty components.*

filepath = u8 1, u64 ino, string path (ino 0 = relative). The kernel sends (ino, "") for a head inode, (parent ino, name) for a dentry op with the parent locked, and otherwise a path relative to the nearest ancestor whose inode is a head inode, where each .snap directory is an empty component (a/.snap/s1/b is (ino(a), "/s1/b"), LSSNAP of a/.snap is (ino(a), "")); the snapshot id itself is never on the wire; mount uses (1, server path without leading "/"). libcephfs uses #ino or #parent_ino/name and builds snapped paths from the nearest non-snap ancestor. The MDS keeps empty components of wire-decoded paths, interprets an empty first component as the snapdir and rejects empty components elsewhere with -EINVAL. libcephfs sends requests relative to any inode number it has cached.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [kclient-expectations.md › 3.6.2 Paths on the wire](../cpp-map/02-client-protocol/kclient-expectations.md#362-paths-on-the-wire) · [libcephfs-fuse-ganesha-expectations.md › 3.4 Request lifecycle (client side)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#34-request-lifecycle-client-side) · [wire-protocol.md › 3.2 Encoding primitives](../cpp-map/02-client-protocol/wire-protocol.md#32-encoding-primitives)
- **Code:** `src/include/filepath.cc:120 (filepath::encode)`, `src/mds/MDCache.cc:8529 (MDCache::path_traverse)`, `linux/fs/ceph/mds_client.c:2899-3122 (ceph_mdsc_build_path, set_request_path_attr)`, `src/client/Inode.cc:146 (Inode::make_nosnap_relative_path)`
- **Notes:** If the parent was not locked and r_parent no longer matches the ino in the built path, the kernel switches r_parent to that inode with a one-time WARN. Path traversal is owned by 05-metadata-cache-and-dirfrags.

<a id="f-proto-073"></a>

#### F-PROTO-073 · Op code numbering and the write bit

*The CEPH_MDS_OP_* numbers, the 0x1000 write bit and its effects, and which ops each client sends.*

Client ops: LOOKUP 0x100, GETATTR 0x101, LOOKUPHASH 0x102, LOOKUPPARENT 0x103, LOOKUPINO 0x104, LOOKUPNAME 0x105, GETVXATTR 0x106, SETXATTR 0x1105, RMXATTR 0x1106, SETLAYOUT 0x1107, SETATTR 0x1108, SETFILELOCK 0x1109, GETFILELOCK 0x110, SETDIRLAYOUT 0x110a, MKNOD 0x1201, LINK 0x1202, UNLINK 0x1203, RENAME 0x1204, MKDIR 0x1220, RMDIR 0x1221, SYMLINK 0x1222, CREATE 0x1301, OPEN 0x302, READDIR 0x305, LOOKUPSNAP 0x400, MKSNAP 0x1400, RMSNAP 0x1401, LSSNAP 0x402, RENAMESNAP 0x1403, READDIR_SNAPDIFF 0x1404, FILE_BLOCKDIFF 0x1405, SNAP_METADATA 0x1406; 0x15xx are internal ops (from a client: -EOPNOTSUPP, or -EROFS first) and DUMMY 0x107 gives -EOPNOTSUPP. may_write() is the write bit or OPEN with O_CREAT|O_TRUNC; it decides -EROFS on a read-only cache and whether a success is recorded as a completed request (never for SETFILELOCK). IS_CEPH_MDS_OP_NEWINODE = CREATE, MKNOD, MKDIR, SYMLINK (the ops that consume head.ino and owner_uid/gid).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.8 Op code table (CEPH_MDS_OP_*, ceph_fs.h:396-451)](../cpp-map/02-client-protocol/wire-protocol.md#38-op-code-table-ceph_mds_op_-ceph_fsh396-451) · [kclient-expectations.md › 3.6.4 Ops the kclient issues](../cpp-map/02-client-protocol/kclient-expectations.md#364-ops-the-kclient-issues) · [libcephfs-fuse-ganesha-expectations.md › 3.5 Operation map: API to MDS op](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#35-operation-map-api-to-mds-op)
- **Code:** `src/include/ceph_fs.h:396-456 (CEPH_MDS_OP_*, IS_CEPH_MDS_OP_NEWINODE)`, `src/messages/MClientRequest.h:196-200 (MClientRequest::may_write)`, `src/mds/Server.cc:2841-2963 (Server::dispatch_client_request)`
- **Notes:** READDIR_SNAPDIFF and FILE_BLOCKDIFF are read-like ops with the write bit, so they fail with -EROFS on a read-only cache and are recorded as completed. may_write tests args.open.flags against host O_CREAT|O_TRUNC while handlers use CEPH_O_*; the values only match on hosts with the asm-generic Linux values (wire-protocol 8). The kernel never sends LOOKUPHASH, READDIR_SNAPDIFF, FILE_BLOCKDIFF or SNAP_METADATA; libcephfs never sends SETLAYOUT or SETDIRLAYOUT. Handlers are owned by 04-namespace-and-posix-operations. See also F-NS-001 (dispatch to handlers).

<a id="f-proto-074"></a>

#### F-PROTO-074 · Client-chosen inode number (head.ino)

*Create-type requests can name the inode number to use (head.ino).*

See F-NS-041. Wire and client side: head.ino is the ino field of the request head (F-PROTO-065); IS_CEPH_MDS_OP_NEWINODE ops (CREATE, MKNOD, MKDIR, SYMLINK) consume it. Clients set it to the target ino of an earlier unsafe reply when they replay a request, and the kernel sets it to a delegated ino for an async create and expects the MDS to create exactly that ino. In up:reconnect the MDS reserves a carried ino with add_replay_ino_alloc. A number that is neither in the session preallocation nor free in the rank InoTable is not replaced but trips an assert (strict interval_set erase); only a number found in replay_taken_inos is replaced, with the cluster-log error "specified ino X but mds allocated Y", which the client does not see.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.10 Forwarding, retry and replay](../cpp-map/02-client-protocol/wire-protocol.md#310-forwarding-retry-and-replay) · [wire-protocol.md › 3.7.1 ceph_mds_request_head layout (ceph_fs.h:688-706, co...](../cpp-map/02-client-protocol/wire-protocol.md#371-ceph_mds_request_head-layout-ceph_fsh688-706-codec-ceph_fscc14-114)
- **Code:** `src/mds/Server.cc:3637-3690 (Server::prepare_new_inode)`, `src/mds/MDCache.h:735 (MDCache::add_replay_ino_alloc)`
- **Notes:** Against the C++ MDS at this commit the kernel never issues async creates (Dc, the Fc bit on a directory cap, is never granted; F-CAPS-054), so head.ino is only used by replays. Delegated inode numbers are owned by 03-caps-leases-and-locking.

<a id="f-proto-075"></a>

#### F-PROTO-075 · Client-side rank selection and the hints the MDS provides

*Clients choose the rank for each request themselves, from caps, fragtree delegation hints in DirStat and random choice; any rank can receive any request.*

The kernel (__choose_mds) uses a forward target first, then the inode or the dentry (parent dir plus name hash with the directory's dl_dir_hash), a dirfrag delegation from the fragtree (a random replica from dist for read-only requests, else the frag auth), then the auth-cap rank (or any cap rank) and finally a random up rank; it sends only to ranks in up:active or later. libcephfs (choose_target_mds) does the same with auth_is_best true for write-bit ops, OPEN, READDIR and some GETATTR masks, and sends LOOKUPINO, LOOKUPPARENT, LOOKUPNAME and LOOKUPHASH to rand() % num_in. The MDS supplies DirStat.auth (-1 = same as the dir inode's auth) and DirStat.dist (replica ranks plus self when the dir is auth and replicated and mds_forward_all_requests_to_auth is false), the dirfragtree in InodeStat and dir_layout.dl_dir_hash (mds_default_dir_hash, default 2 = rjenkins, on new directories).

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_forward_all_requests_to_auth (default false); mds_default_dir_hash (default 2, rjenkins)
- **Map:** [kclient-expectations.md › 3.6.1 Choosing the target rank](../cpp-map/02-client-protocol/kclient-expectations.md#361-choosing-the-target-rank) · [libcephfs-fuse-ganesha-expectations.md › 3.4 Request lifecycle (client side)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#34-request-lifecycle-client-side) · [wire-protocol.md › 3.9.1 Trace layout (Server::set_trace_dist, Server.cc:249...](../cpp-map/02-client-protocol/wire-protocol.md#391-trace-layout-serverset_trace_dist-servercc2491-2561)
- **Code:** `linux/fs/ceph/mds_client.c:1381-1534 (__choose_mds)`, `linux/fs/ceph/inode.c:424 (ceph_fill_dirfrag)`, `src/client/Client.cc:1978 (Client::choose_target_mds)`, `src/mds/CDir.h:436 (CDir::get_dist_spec)`
- **Notes:** The kernel hashes names with the VFS d_name.hash for dl_dir_hash 0 or 1 and with rjenkins for 2 (other values yield 0xffffffff); readdir offsets always use ceph_str_hash(dl_dir_hash, name) (kclient expectations 3.6.1). Subtree authority is owned by 07-multi-mds. See also F-CACHE-079 (DirStat hints), F-MULTI-050 (hot-directory replication).

<a id="f-proto-076"></a>

#### F-PROTO-076 · Request forwarding (MClientRequestForward)

*A rank that is not authoritative tells the client to resend the request to another rank; it never relays the request itself.*

MDCache::request_forward calls MDSRank::forward_message_mds, which sends MClientRequestForward(tid, dest_mds, num_fwd = request num_fwd + 1, client_must_resend = true) and cleans up the request; payload i32 dest_mds, i32 num_fwd, bool client_must_resend. Every member of a batched getattr/lookup is forwarded; an internal op gets -EXDEV and a request from an MDS is dropped. The kernel reads next_mds and fwd_seq (ignoring client_must_resend): if fwd_seq <= its stored count or >= U32_MAX it aborts the request with -EMULTIHOP, otherwise it resets r_attempts, stores the count and resends to next_mds; libcephfs does the same. The resent request carries the count in ext_num_fwd/num_fwd. MDCache::path_traverse discovers (replicates) instead of forwarding again when the path depth is below the request's num_fwd, which bounds forward ping-pong between ranks.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.10 Forwarding, retry and replay](../cpp-map/02-client-protocol/wire-protocol.md#310-forwarding-retry-and-replay) · [kclient-expectations.md › 3.6.5 Retries, forwards, replay, abort](../cpp-map/02-client-protocol/kclient-expectations.md#365-retries-forwards-replay-abort) · [libcephfs-fuse-ganesha-expectations.md › 3.4 Request lifecycle (client side)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#34-request-lifecycle-client-side)
- **Code:** `src/mds/MDSRank.cc:1504-1527 (MDSRank::forward_message_mds)`, `src/messages/MClientRequestForward.h:54-67 (MClientRequestForward)`, `src/mds/MDCache.cc:8821-8829 (MDCache::path_traverse)`, `linux/fs/ceph/mds_client.c:4291-4355 (handle_forward)`, `src/client/Client.cc:2955 (Client::handle_client_request_forward)`
- **Notes:** A forward for a request that already applied an (unsafe) reply or recorded an error hits BUG_ON in the kernel handle_forward ("forward race not possible; mds would drop") (kclient expectations 8). Before v5.19 (and in CentOS Stream 8) kernels ignore a forward whose 8-bit count wrapped, so the request is not resent (derived from code). Forwarding policy is owned by 07-multi-mds. See also F-CACHE-044 (forward or discover), F-NS-053 (which rank serves each op), F-MULTI-018 (migration).

<a id="f-proto-077"></a>

#### F-PROTO-077 · Retry counters and attempt limits (32BITS_RETRY_FWD)

*Each resend of a request carries an attempt count; 8-bit counters limit old clients to 256 attempts or forwards.*

Every send increments the client attempt counter; the head carries attempts - 1 in ext_num_retry and its low 8 bits in num_retry. Both clients reset the counter after a forward, and the kernel also after a local session teardown. Without the MDS bit 18 clients use head v1 (8-bit counters only) and abort the request with -EMULTIHOP after 256 attempts; with it at U32_MAX. The MDS uses a non-zero retry count only to consult the completed-request table and, in up:reconnect, to queue a completed request for clientreplay and drop its embedded releases; forward counts are always widened to 32 bits on decode.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Widening of 8-bit num_retry/num_fwd from legacy and v1 heads; the clients' 256-attempt limit towards an MDS without bit 18. The baseline MDS advertises bit 18, so the 7.3 kernel and libcephfs send 32-bit counters (head v3) and abort only at U32_MAX. The 8-bit counters and the 256-attempt abort apply to clients without bit 18 (kernels before v6.6, CentOS Stream 8, Ceph before 16.2.14, 17.2.7 and 18.1.0) and to current clients only towards an MDS without bit 18. Evidence: `src/common/ceph_fs.cc:74-77 (decode(ceph_mds_request_head&))`, `src/messages/MClientRequest.h:280-281 (MClientRequest::decode_payload)`, `linux/fs/ceph/mds_client.c:3507-3524 (__prepare_send_request)`, `src/client/Client.cc:2890-2912 (Client::build_client_request)`, `linux/fs/ceph/mds_client.h:56 (CEPHFS_FEATURE_32BITS_RETRY_FWD advertised)`, `linux-v6.6/fs/ceph/mds_client.h:51 (first kernel advertising bit 18)`.
- **Gates:** MDS bit CEPHFS_FEATURE_32BITS_RETRY_FWD (18): Linux v6.6+, Ceph 16.2.14, 17.2.7, 18.2.0+
- **Map:** [wire-protocol.md › 3.10 Forwarding, retry and replay](../cpp-map/02-client-protocol/wire-protocol.md#310-forwarding-retry-and-replay) · [kclient-expectations.md › 3.6.5 Retries, forwards, replay, abort](../cpp-map/02-client-protocol/kclient-expectations.md#365-retries-forwards-replay-abort) · [kclient-expectations.md › 7.1.2 Protocol fallbacks and the clients that exercise them](../cpp-map/02-client-protocol/kclient-expectations.md#712-protocol-fallbacks-and-the-clients-that-exercise-them)
- **Code:** `src/common/ceph_fs.cc:71-77 (decode of ceph_mds_request_head)`, `linux/fs/ceph/mds_client.c:3507-3524 (__prepare_send_request)`, `src/client/Client.cc:2893-2911 (Client::build_client_request)`, `src/mds/Server.cc:2657-2694 (Server::handle_client_request)`
- **Notes:** Kernels v5.19-v6.5 abort at 256 retries or forwards; before v5.19 nothing stops the wrap (kclient expectations 7.1.2). Completed-request handling is owned by 04-namespace-and-posix-operations.

<a id="f-proto-078"></a>

#### F-PROTO-078 · Request identity, tid ordering and requests without a session

*Requests are identified by (client, tid); the MDS drops duplicates of an active request and requests from clients without an open session.*

tids increase per client incarnation, but libcephfs assigns the tid when a call enters make_request and the request may then wait for an MDSMap or a session open, or be forwarded, so requests of one session can reach a rank out of tid order. MDCache::request_start drops a request whose metareqid_t(client, tid) is already active. Requests from a client without a session, or with a closed, closing or killing session, are dropped without reply. oldest_client_tid is the lowest outstanding tid that is not SETFILELOCK, so it lags behind the slowest outstanding request.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.10 Forwarding, retry and replay](../cpp-map/02-client-protocol/wire-protocol.md#310-forwarding-retry-and-replay) · [libcephfs-fuse-ganesha-expectations.md › 3.16.1 Concurrency model seen by the MDS](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3161-concurrency-model-seen-by-the-mds) · [libcephfs-fuse-ganesha-expectations.md › 6. Concurrency and mds_lock](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/MDCache.cc:9817 (MDCache::request_start)`, `src/mds/Server.cc:2633-2647 (Server::handle_client_request)`, `src/client/Client.cc:2251 (Client::make_request)`

### Reply encoding

MClientReply: the head, the trace with InodeStat, DirStat and LeaseStat, the snap trace, the op-specific extra payloads, the early/safe split, and the invariants clients check on reply content.

<a id="f-proto-079"></a>

#### F-PROTO-079 · MClientReply head and error codes

*Every request is answered with MClientReply: a 15-byte head, the trace, an op-specific extra blob and a snap trace.*

Front payload: raw ceph_mds_reply_head {u32 op (copied from the request), u32 result (errorcode32_t: 0 or a negative Linux errno), u32 mdsmap_epoch (0 in early replies, set only on the final reply), u8 safe, u8 is_dentry, u8 is_target}, then bufferlist trace_bl, bufferlist extra_bl, bufferlist snapbl; message version 1; header.tid = request tid. The kernel dispatches the extra parser on the head op, requires the trace and the whole front to be consumed exactly, and matches replies by tid on the session the request was last sent to (others are dropped); libcephfs asserts that the front is consumed. Result codes produced: -EINVAL, -ENOENT, -EPERM, -EACCES, -ENOTDIR, -EISDIR, -EROFS, -ENOTEMPTY, -ESTALE, -ENAMETOOLONG, -ENOSPC, -EEXIST, -EAGAIN, -ENODATA, -EBUSY, -EXDEV, -EOPNOTSUPP, -EDOM, -ENXIO, -EMLINK, -EIO, -EINTR, -EINPROGRESS, -EDEADLK, -EBADMSG; the MDS never returns -EDQUOT (quota is enforced by clients).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.9 MClientReply encoding](../cpp-map/02-client-protocol/wire-protocol.md#39-mclientreply-encoding) · [wire-protocol.md › 3.9.7 Result codes](../cpp-map/02-client-protocol/wire-protocol.md#397-result-codes) · [kclient-expectations.md › 3.7 Reply parsing (parse_reply_info)](../cpp-map/02-client-protocol/kclient-expectations.md#37-reply-parsing-parse_reply_info)
- **Code:** `src/messages/MClientReply.h:403-434 (MClientReply::decode_payload, encode_payload)`, `src/include/ceph_fs.h:739-788 (ceph_mds_reply_head)`, `linux/fs/ceph/mds_client.c:843 (parse_reply_info)`, `linux/fs/ceph/mds_client.c:4094 (handle_reply)`
- **Notes:** -EMULTIHOP and -EJUKEBOX are generated client side only. POSIX permission bits are not checked by the MDS except through uid/gid-restricted MDS cap grants (wire-protocol 3.9.7).

<a id="f-proto-080"></a>

#### F-PROTO-080 · Reply trace layout

*The trace tells the client which directory, dentry and inode the request touched, so it can update its cache without another round trip.*

trace_bl = [InodeStat(dir inode), DirStat(dir frag), string dname, LeaseStat(dentry)] if is_dentry, then [InodeStat(target)] if is_target; nothing else. snapbl is the snap trace of the target's realm (or the dir's realm without target), empty for snapshot ids; clients apply it before the trace. Error replies can carry a trace (-ENOENT with WANT_DENTRY gives a null-dentry trace). No trace is built for a reply that follows an early reply or for a re-executed REPLAY request. The dir InodeStat is encoded without a realm and may issue caps on the parent. Traceless replies also occur for completed requests and with mds_inject_traceless_reply_probability (default 0, test only); clients then look the target up themselves (the kernel follow-up LOOKUP for creates, libcephfs verify_reply_trace).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_inject_traceless_reply_probability (default 0)
- **Map:** [wire-protocol.md › 3.9.1 Trace layout (Server::set_trace_dist, Server.cc:249...](../cpp-map/02-client-protocol/wire-protocol.md#391-trace-layout-serverset_trace_dist-servercc2491-2561) · [libcephfs-fuse-ganesha-expectations.md › 3.6 Reply decoding](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#36-reply-decoding) · [kclient-expectations.md › 3.8.3 Dentry trace (ceph_fill_trace, linux/fs/ceph/inode....](../cpp-map/02-client-protocol/kclient-expectations.md#383-dentry-trace-ceph_fill_trace-linuxfscephinodec1593-ceph_fill_trace)
- **Code:** `src/mds/Server.cc:2491-2561 (Server::set_trace_dist)`, `linux/fs/ceph/mds_client.c:441 (parse_reply_info_trace)`, `src/client/Client.cc:1771 (Client::insert_trace)`
- **Notes:** libcephfs asserts that the trace of a safe reply following an unsafe one is empty, and a traceless reply to a request whose dentry is no longer linked hits ceph_abort_msg in verify_reply_trace (libcephfs-fuse-ganesha 7.1).

<a id="f-proto-081"></a>

#### F-PROTO-081 · InodeStat versioned encoding (v9, REPLY_ENCODING)

*The per-inode record in traces and readdir listings, in the versioned form sent to clients that advertise bit 9.*

ENCODE_START(9, 1) { u64 ino, u64 snapid, u32 rdev, u64 version (2 x version + 1 if projected), u64 xattr_version, raw 33-byte reply cap {caps, wanted, cap_id, seq, mseq, realm, flags (CEPH_CAP_FLAG_AUTH)}, raw 28-byte legacy layout, ctime, mtime, atime, u32 time_warp_seq, size, max_size, truncate_size, u32 truncate_seq, mode, uid, gid, nlink, nfiles, nsubdirs, rbytes, rfiles, rsubdirs, rctime, dirfragtree, symlink, 8-byte dir_layout, xattr blob, inline_version, inline_data, quota ENCODE_START(1,1){max_bytes, max_files}, pool_ns, btime, change_attr; v2 dir_pin; v3 snap_btime; v4 rsnaps; v5 snap_metadata; v6 fscrypt flag; v7 fscrypt_auth, fscrypt_file; v8 optmetadata (kind 1 CHARMAP, kind 2 QUARANTINE); v9 subvolume_id }. The fixed prefix up to the fragtree count is byte-identical to the kernel's 221-byte struct ceph_mds_reply_inode. The kernel requires compat 1, skips snap_metadata, the fscrypt flag and optmetadata, and uses struct_len to skip later fields.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** client bit CEPHFS_FEATURE_REPLY_ENCODING (9): Linux v5.1+, Ceph 14.2.0+
- **Map:** [wire-protocol.md › 3.9.2 InodeStat (CInode::encode_inodestat, CInode.cc:3959...](../cpp-map/02-client-protocol/wire-protocol.md#392-inodestat-cinodeencode_inodestat-cinodecc3959-4429-decoder-mclientreplyh118-307) · [kclient-expectations.md › 3.7 Reply parsing (parse_reply_info)](../cpp-map/02-client-protocol/kclient-expectations.md#37-reply-parsing-parse_reply_info)
- **Code:** `src/mds/CInode.cc:4315-4371 (CInode::encode_inodestat)`, `src/messages/MClientReply.h:118-307 (InodeStat::decode)`, `linux/fs/ceph/mds_client.c:117 (parse_reply_info_in)`
- **Notes:** Code observations (wire-protocol 8): optmetadata_singleton::get_kind treats only kinds > 3 as unknown, so kind 3 indexes past the variant table; for a quarantined inode the encoder emits a default record (enabled, timestamp 0) instead of the stored one; for snapped inodes cap_id and wanted are left uninitialised. Ceph 19.2.3 and 20.2.x send v8; this tree and 21.x send v9. Field semantics are owned by 05-metadata-cache-and-dirfrags.

<a id="f-proto-083"></a>

#### F-PROTO-083 · DirStat and LeaseStat records

*The directory-fragment record and the dentry-lease record inside traces and readdir entries.*

DirStat = ENCODE_START(1,1) {u32 frag, i32 auth, set<i32> dist} from CDir::encode_dirstat. LeaseStat = ENCODE_START(2,1) {u16 mask, u32 duration_ms, u32 seq, v2 string alternate_name} from Locker::encode_lease. A real dentry lease (Locker::issue_client_lease) has mask CEPH_LEASE_VALID plus CEPH_LEASE_PRIMARY_LINK for a primary dentry, duration 30000 ms (client_lease_durations[1]) and seq = ++lease seq; when the client cannot get a lease (snapshot, dentry lock not leaseable, stray dir, dir filelock leaseable by the client, or the client holds Fs/Fx on the dir) a null lease is sent (mask 0, duration 0, seq 0) that still carries alternate_name.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Header-less DirStat and raw 10-byte ceph_mds_reply_lease for sessions without CEPHFS_FEATURE_REPLY_ENCODING. Baseline clients advertise bit 9 and receive DirStat v1 and LeaseStat v2 with alternate_name. The header-less DirStat and the raw 10-byte LeaseStat go only to sessions without bit 9 (kernels up to v5.0, Ceph up to 13.2.x, sessions with inferred features). Evidence: `src/mds/Locker.cc:4620-4625 (Locker::encode_lease)`, `src/mds/CDir.cc:3066-3070 (CDir::encode_dirstat)`, `linux/fs/ceph/mds_client.h:48 (CEPHFS_FEATURE_REPLY_ENCODING advertised)`, `src/client/Client.cc:448-452 (myfeatures)`.
- **Gates:** CEPHFS_FEATURE_REPLY_ENCODING (bit 9)
- **Map:** [wire-protocol.md › 3.9.3 DirStat and LeaseStat](../cpp-map/02-client-protocol/wire-protocol.md#393-dirstat-and-leasestat) · [wire-protocol.md › 3.9.1 Trace layout (Server::set_trace_dist, Server.cc:249...](../cpp-map/02-client-protocol/wire-protocol.md#391-trace-layout-serverset_trace_dist-servercc2491-2561)
- **Code:** `src/mds/CDir.cc:3058-3071 (CDir::encode_dirstat)`, `src/mds/Locker.cc:4542-4626 (Locker::issue_client_lease, encode_lease)`, `linux/fs/ceph/mds_client.c:325-380 (parse_reply_info_dir, parse_reply_info_lease)`
- **Notes:** Lease semantics (validity, revocation, PRIMARY_LINK and async unlink) are owned by 03-caps-leases-and-locking. Without CEPHFS_FEATURE_REPLY_ENCODING the lease record is the raw 10-byte ceph_mds_reply_lease {u16 mask, u32 duration_ms, u32 seq} (F-PROTO-082). In a trace the order is [dir InodeStat][DirStat][dname][lease][target InodeStat]; readdir entries are [dname][lease][InodeStat]. The kernel rejects struct_compat != 1, reads alternate_name only for struct_v >= 2 and skips to struct_len; both client families use alternate_name. See also F-CAPS-038 (lease issue), F-CACHE-079 (DirStat hints).

<a id="f-proto-084"></a>

#### F-PROTO-084 · Caps, xattrs and inline data carried inside InodeStat

*Wire view of the caps, xattr blob and inline data that an InodeStat can carry.*

See F-CAPS-009 (cap issued with a reply), F-NS-068 (xattr blob inclusion rules) and F-CAPS-015 (inline data and xattrs versioned per client). Wire-specific: inline_version is CEPH_INLINE_NONE when the inode is not inline and 0 when the data is not sent; xattr_version is 0 when the blob is dropped; max_size is the client's writeable range. During readdir an estimate is checked against max_bytes and -ENOSPC stops the listing.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (deprecated-admin): Inline data content in InodeStat (inline_version other than CEPH_INLINE_NONE, including 0 when the data is withheld). New inodes get inline_version CEPH_INLINE_NONE unless the MDSMap enables inline data, which only the deprecated `ceph fs set <fs> inline_data true --yes-i-really-really-mean-it` does. No mgr module, cephadm, Rook or tool sets it, so inline_version 0 (data withheld) and non-empty inline data occur only on file systems where that deprecated setting was used. Evidence: `src/mds/CInode.cc:4079-4090 (CInode::encode_inodestat)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mon/FSCommands.cc:471-478 (inline_data, deprecated)`.
- **Map:** [wire-protocol.md › 3.9.2 InodeStat (CInode::encode_inodestat, CInode.cc:3959...](../cpp-map/02-client-protocol/wire-protocol.md#392-inodestat-cinodeencode_inodestat-cinodecc3959-4429-decoder-mclientreplyh118-307)
- **Code:** `src/mds/CInode.cc:3959-4429 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4079-4090 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4143-4176 (CInode::encode_inodestat)`
- **Notes:** libcephfs asserts that inline_version becomes > 0 after a GETATTR asking for inline data (libcephfs-fuse-ganesha 4).

<a id="f-proto-085"></a>

#### F-PROTO-085 · Snap trace encoding and format selection

*The snap realm chain sent with replies, cap imports and MClientSnap, in an old raw form or a versioned form chosen by client bit 19.*

A snap trace is the concatenation of realm records from the target realm up to the root realm. Old element (SnapRealmInfo): raw 48-byte ceph_mds_snap_realm {ino, created, parent, parent_since, seq, u32 num_snaps, u32 num_prior_parent_snaps} followed by those u64 snap ids. New element (SnapRealmInfoNew): ENCODE_START(2,1) {old element, utime last_modified, u64 change_attr, v2 u32 flags (SNAPDIR_VISIBILITY = 4)}. Server::get_snap_trace picks the new form for sessions with NEW_SNAPREALM_INFO; libcephfs picks its decoder from the MDS's bit, so the two sides agree because the MDS always advertises it. A snap trace the kernel cannot parse makes it blocklist itself.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** client bit CEPHFS_FEATURE_NEW_SNAPREALM_INFO (19): never in Linux, Ceph 17.2.8 and 18.2.0+
- **Map:** [wire-protocol.md › 3.9.4 Snap trace](../cpp-map/02-client-protocol/wire-protocol.md#394-snap-trace) · [libcephfs-fuse-ganesha-expectations.md › 3.10 Snapshots and snap realms](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#310-snapshots-and-snap-realms)
- **Code:** `src/mds/Server.cc:12429-12442 (Server::get_snap_trace)`, `src/common/snap_types.cc:10 (SnapRealmInfo::encode)`, `src/common/snap_types.cc:63 (SnapRealmInfoNew::encode)`, `src/mds/SnapRealm.cc:450-508 (SnapRealm::build_snap_trace)`
- **Notes:** No kernel advertises bit 19, so every kernel needs the old format (kclient expectations 7.1.7). Snap realm semantics are owned by 08-snapshots-quotas-subvolumes. Element order is child first up to the root realm (parent 0) or the global realm; my_snaps and prior_parent_snaps are descending le64 lists; clients test only flag 4. Traces travel in MClientReply.snapbl (only for replies whose snapid is CEPH_NOSNAP; LSSNAP replies carry the directory realm's trace; replies about snapshots carry none), in MClientSnap and in MClientCaps IMPORT. See also F-SNAP-008 (contents and client interpretation), F-SNAP-006.

<a id="f-proto-086"></a>

#### F-PROTO-086 · Readdir and LSSNAP reply format and ordering contract

*The extra_bl of directory listings and the ordering and size rules clients enforce on it.*

READDIR and READDIR_SNAPDIFF extra_bl: DirStat, u32 num, u16 flags, then num x (string dname, LeaseStat, InodeStat). flags: FRAG_END (1) when the frag is exhausted, FRAG_COMPLETE (0x100) when it was also read from its start, HASH_ORDER (0x200) and OFFSET_HASH (0x400) when the request had CEPH_READDIR_REPLY_BITFLAGS. Entries come in dirfrag key order (24-bit name hash, name, snapid); resume is by path2 = last name, else by offset_hash; a non-existent frag is adjusted to the one containing offset_hash; the .ceph inode is skipped. LSSNAP: empty DirStat (frag 0, auth -1), entries named by snapshot (long name _<name>_<ino> for inherited snaps) with lease {VALID, 0xffffffff ms, seq 0} and the dir InodeStat at that snapid. The kernel sends max_entries from its preallocated buffer and fails a reply with more entries (-EIO), requires non-decreasing hash order from the continuation point (readdir -EIO otherwise), adopts the returned frag, and needs the extra on every successful READDIR/LSSNAP. libcephfs sends max_entries = max_bytes = 0, re-chooses the frag on -EAGAIN and parses the extra only when the trace has the target.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): READDIR handling without CEPH_READDIR_REPLY_BITFLAGS: frag adjustment that ignores offset_hash and clears the resume name, and replies without HASH_ORDER/OFFSET_HASH whose flags such clients read as the old u8 end, u8 complete pair. The 7.3 kernel and libcephfs set CEPH_READDIR_REPLY_BITFLAGS on every READDIR and LSSNAP, and libcephfs also on READDIR_SNAPDIFF, so replies carry HASH_ORDER and OFFSET_HASH and offset_hash selects the frag. Requests without the flag come only from kernels before v4.7. Evidence: `src/mds/Server.cc:5150-5160 (Server::handle_client_readdir)`, `src/mds/Server.cc:5325-5327 (Server::handle_client_readdir)`, `src/mds/Server.cc:12313 (Server::handle_client_readdir_snapdiff)`, `linux/fs/ceph/dir.c:450-451 (ceph_readdir)`, `src/client/Client.cc:10116 (Client::readdir_r_cb)`, `src/client/Client.cc:10622 (READDIR_SNAPDIFF flags)`. Also: linux-v4.7/fs/ceph/dir.c:409 (first fetched tag that sets the flag; absent in linux-v4.5).
- **Map:** [wire-protocol.md › 3.9.5 extra_bl per op](../cpp-map/02-client-protocol/wire-protocol.md#395-extra_bl-per-op) · [kclient-expectations.md › 3.8.4 Readdir semantics](../cpp-map/02-client-protocol/kclient-expectations.md#384-readdir-semantics) · [libcephfs-fuse-ganesha-expectations.md › 3.9 Readdir](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#39-readdir)
- **Code:** `src/mds/Server.cc:5050-5090 (Server::_finalize_readdir)`, `src/mds/Server.cc:11520-11572 (Server::handle_client_lssnap)`, `linux/fs/ceph/mds_client.c:455-588 (parse_reply_info_readdir)`, `linux/fs/ceph/dir.c:545 (ceph_readdir)`, `src/mds/mdstypes.h:410 (dentry_key_t operator<)`
- **Notes:** Derived from code: a successful READDIR without extra makes the kernel dereference a NULL DirStat, and a reply whose head op is READDIR/LSSNAP with entries for a request without a readdir buffer hits BUG_ON (kclient expectations 8). Kernels before v4.7 do not send REPLY_BITFLAGS; the flags field then reads as the old u8 end, u8 complete pair. Readdir semantics are owned by 04-namespace-and-posix-operations. See also F-NS-032, F-NS-033.

<a id="f-proto-087"></a>

#### F-PROTO-087 · CREATE reply: created inode and delegated inode ranges

*A CREATE that made a new inode reports its number in extra_bl, as openc_response_t (with delegated inode ranges) or as a raw u64.*

Server::set_reply_extra_bl writes openc_response_t = ENCODE_START(1,1) {u64 created_ino, interval_set<inodeno_t> delegated_inos} when the MDS member mds_allow_async_dirops is true and the client has DELEG_INO; else a raw u64 ino when the connection has REPLY_CREATE_INODE; else nothing (an empty extra means "opened an existing file"). Both clients choose the parser from the MDS's DELEG_INO bit, not from the bytes. Delegation: when mds_client_delegate_inos_pct is non-zero, the request is not a queued replay and the session holds fewer than 250 delegated inodes (defaults), the set is topped up to 500 from the free preallocated range and only the new ranges are returned. The kernel skips the 6-byte header unchecked, ignores ranges below 0x1000 and fails the reply with -EIO above 8192 delegated inodes; libcephfs logs and discards them. Replies from the completed-request table for requests that created an inode carry the same extra. libcephfs verify_reply_trace re-resolves a traceless created inode with LOOKUP or GETATTR and returns -EINTR on an ino mismatch.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Raw u64 created ino for sessions without bit 13; no created-ino extra for connections without REPLY_CREATE_INODE. Baseline clients advertise bit 13 and choose the openc_response_t parser from the MDS's bit 13, which is current with mds_allow_async_dirops true (default). The raw u64 created ino serves sessions without bit 13 (kernels up to v5.6, Ceph up to 14.x), and the empty extra serves connections without REPLY_CREATE_INODE (kernels before 3.9). With mds_allow_async_dirops false the raw u64 also reaches bit-13 clients, which then fail the reply. Evidence: `src/mds/Server.cc:2595 (Server::set_reply_extra_bl, bit 13)`, `src/mds/Server.cc:2609-2612 (Server::set_reply_extra_bl, raw u64)`, `linux/fs/ceph/mds_client.c:762-783 (parse_reply_info_create)`, `src/client/Client.cc:2159 (Client::verify_reply_trace)`, `linux/fs/ceph/mds_client.h:51 (CEPHFS_FEATURE_DELEG_INO advertised)`, `src/include/ceph_features.h:126 (REPLY_CREATE_INODE, kernel 3.9)`.
- **Gates:** client bit CEPHFS_FEATURE_DELEG_INO (13): Linux v5.7+, Ceph 15.2.0+; connection feature REPLY_CREATE_INODE (27); mds_allow_async_dirops (default true); mds_client_delegate_inos_pct (50); mds_client_prealloc_inos (1000)
- **Map:** [wire-protocol.md › 3.9.5 extra_bl per op](../cpp-map/02-client-protocol/wire-protocol.md#395-extra_bl-per-op) · [kclient-expectations.md › 3.7 Reply parsing (parse_reply_info)](../cpp-map/02-client-protocol/kclient-expectations.md#37-reply-parsing-parse_reply_info) · [libcephfs-fuse-ganesha-expectations.md › 3.4 Request lifecycle (client side)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#34-request-lifecycle-client-side)
- **Code:** `src/mds/Server.cc:2591-2614 (Server::set_reply_extra_bl)`, `src/messages/MClientReply.h:309 (openc_response_t)`, `linux/fs/ceph/mds_client.c:622-783 (ceph_parse_deleg_inos, parse_reply_info_create)`, `src/client/Client.cc:2146-2177 (Client::verify_reply_trace)`
- **Notes:** Derived from code: with mds_allow_async_dirops turned off at runtime a raw 8-byte ino reaches clients that chose the openc_response_t parser, so every kernel CREATE reply fails with -EIO and libcephfs throws in DECODE_START (wire-protocol 8, kclient expectations 8). The member is read only in handle_conf_change, so a value set before startup is not applied until it changes at runtime. A retried MKNOD/MKDIR/SYMLINK found in the completed-request table outside clientreplay gets a traceless reply with this extra, which the kernel rejects with -EIO for those ops (narrow window, derived from code). Raw u64 users: kernels up to v5.6, Ceph up to 14.x. The kernel accepts a non-empty extra_bl only for GETFILELOCK, READDIR, LSSNAP, CREATE and GETVXATTR and fails the reply with -EIO otherwise. A CREATE that finds an existing name without O_EXCL is served as OPEN and carries no created ino, which is how both clients learn that the file already existed. See also F-CAPS-054 (delegation), F-NS-035 (CREATE), F-NS-012 (completed-request replies).

<a id="f-proto-088"></a>

#### F-PROTO-088 · GETFILELOCK, GETVXATTR and FILE_BLOCKDIFF extra payloads

*The remaining op-specific extra_bl formats; the kernel rejects a non-empty extra on any other op.*

GETFILELOCK: raw 41-byte ceph_filelock {u64 start, length, client, owner, pid, u8 type} describing the conflicting lock, or type CEPH_LOCK_UNLOCK. GETVXATTR: ENCODE_START(1,1) {string value}, which the kernel requires to fill the section exactly. FILE_BLOCKDIFF: BlockDiff = ENCODE_START(1,1) {i32 rval, u64 scan_idx, interval_set<u64> blocks}. The kernel accepts extra_bl only for GETFILELOCK, READDIR, LSSNAP, CREATE and GETVXATTR (any other op with a non-empty extra is -EIO). Clients send GETVXATTR only to an MDS with bit 17 (otherwise -EOPNOTSUPP locally, mapped to -ENODATA by the kernel) and FILE_BLOCKDIFF only with bit 23.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** MDS bit CEPHFS_FEATURE_OP_GETVXATTR (17): Linux v6.1+ checks it, Ceph 16.2.11, 17.2.6, 18.2.0+; MDS bit CEPHFS_FEATURE_BLOCKDIFF (23): libcephfs only, Ceph 19.2.3, 20.2.0+
- **Map:** [wire-protocol.md › 3.9.5 extra_bl per op](../cpp-map/02-client-protocol/wire-protocol.md#395-extra_bl-per-op) · [kclient-expectations.md › 3.7 Reply parsing (parse_reply_info)](../cpp-map/02-client-protocol/kclient-expectations.md#37-reply-parsing-parse_reply_info)
- **Code:** `src/mds/Server.cc:5525-5576 (Server::handle_client_file_readlock)`, `src/mds/Server.cc:7509-7519 (Server::handle_client_getvxattr)`, `src/mds/Server.cc:12239-12252 (Server::handle_file_blockdiff_finish)`, `linux/fs/ceph/mds_client.c:795-841 (parse_reply_info_getvxattr, parse_reply_info_extra)`
- **Notes:** Kernels v5.18-v6.0 send GETVXATTR without checking any bit ("The old ceph will crash the MDSs when see unknown OPs"). The ops themselves are owned by 04-namespace-and-posix-operations and 08-snapshots-quotas-subvolumes.

<a id="f-proto-089"></a>

#### F-PROTO-089 · Early (unsafe) and safe replies

*A mutation is answered twice: an early unsafe reply as soon as it is applied in memory, and a safe reply once its journal event is committed.*

Server::journal_and_reply calls early_reply before submitting the event. An early reply is sent unless mds_early_reply is false, the request set no_early_reply, it has journaled peers (multi-rank two-phase commit), its inode number came from the InoTable rather than the session's preallocated range, the requester is an MDS, or it is a replay; xlocks are marked done first so the uncommitted state becomes visible. The early reply carries the trace and extra_bl with safe = 0 and mdsmap_epoch = 0; the final reply after it carries only the result. Read-only and non-journaling ops get one safe reply with the trace; a re-executed REPLAY gets one traceless safe reply. The kernel applies the unsafe reply, keeps the request on the session unsafe list, does not parse the later safe reply and ignores duplicates; libcephfs links unsafe requests into session and inode lists and completes fsync waiters on the safe reply. Clients replay requests that got only an unsafe reply after an MDS failure; if the session is closed or the reconnect denied they are lost and the kernel marks the affected mappings -EIO.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_early_reply (default true)
- **Map:** [wire-protocol.md › 3.9.6 Safe vs unsafe replies](../cpp-map/02-client-protocol/wire-protocol.md#396-safe-vs-unsafe-replies) · [kclient-expectations.md › 3.6.5 Retries, forwards, replay, abort](../cpp-map/02-client-protocol/kclient-expectations.md#365-retries-forwards-replay-abort) · [libcephfs-fuse-ganesha-expectations.md › 3.4 Request lifecycle (client side)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#34-request-lifecycle-client-side)
- **Code:** `src/mds/Server.cc:2124 (Server::journal_and_reply)`, `src/mds/Server.cc:2300-2328 (Server::early_reply)`, `linux/fs/ceph/mds_client.c:4102-4143 (handle_reply)`, `src/client/Client.cc:3022-3098 (Client::handle_client_reply)`
- **Notes:** Perf counters measure latency to the first reply, i.e. to the early reply (wire-protocol 3.9.6). At most one unsafe and one safe reply per tid, unsafe before safe, on the session's current connection (wire-protocol 7). See also F-NS-011 (conditions), F-NS-010.

<a id="f-proto-090"></a>

#### F-PROTO-090 · Reply content invariants enforced by clients

*Properties of trace content that clients check, some with a kernel BUG or an assert.*

Kernel: a dentry trace whose directory differs from the request's locked parent in ino or snapid hits BUG_ON, a LOOKUPNAME dentry trace without target hits BUG_ON, inode numbers in [0x100, 0x1000) are refused with -EREMOTEIO, a changed inode type or device rdev gives -ESTALE, directories report nlink 1 and size 0 (the kernel shows 2 + subdirs). Version fields: version (2 x version + projected) is applied only if newer with an AUTH cap; size only decreases with a newer truncate_seq; mtime/atime only go back with a newer time_warp_seq; xattr_version and change_attr are monotonic; max_size is taken only from an auth cap; subvolume_id is immutable once non-zero. libcephfs applies the same version, cap and AUTH rules in add_update_inode and marks a directory complete when the reply shows it empty with Fs and without Fx.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [kclient-expectations.md › 3.8 Applying replies](../cpp-map/02-client-protocol/kclient-expectations.md#38-applying-replies) · [kclient-expectations.md › 3.8.2 ceph_fill_inode rules (linux/fs/ceph/inode.c:1013 (...](../cpp-map/02-client-protocol/kclient-expectations.md#382-ceph_fill_inode-rules-linuxfscephinodec1013-ceph_fill_inode) · [kclient-expectations.md › 3.8.3 Dentry trace (ceph_fill_trace, linux/fs/ceph/inode....](../cpp-map/02-client-protocol/kclient-expectations.md#383-dentry-trace-ceph_fill_trace-linuxfscephinodec1593-ceph_fill_trace) · [libcephfs-fuse-ganesha-expectations.md › 3.6 Reply decoding](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#36-reply-decoding)
- **Code:** `linux/fs/ceph/inode.c:1013 (ceph_fill_inode)`, `linux/fs/ceph/inode.c:1593 (ceph_fill_trace)`, `linux/fs/ceph/inode.c:1762 (ceph_fill_trace)`, `src/client/Client.cc:1106 (Client::add_update_inode)`
- **Notes:** Kernel BUG paths reachable from MDS input are listed in kclient expectations 8. The C++ MDS uses nlink 1 for directories. Inode attribute semantics are owned by 05-metadata-cache-and-dirfrags.

### Other client messages

Wire layouts of the non-request client messages (caps, cap releases, leases, snaps, quota, metrics, quarantine); their semantics are owned by other categories and linked from each entry.

<a id="f-proto-091"></a>

#### F-PROTO-091 · MClientCaps encoding (v14, xattrs in the middle segment)

*Layout of the capability message exchanged in both directions; semantics belong to the capability model.*

Front: raw 92-byte ceph_mds_caps_head {u32 op, u64 ino, realm, cap_id, u32 seq, issue_seq, caps, wanted, dirty, migrate_seq, u64 snap_follows, u32 snap_trace_len, uid, gid, mode, nlink, xattr_len, u64 xattr_version}, the 84-byte non-export body {size, max_size, truncate_size, truncate_seq, mtime, atime, ctime, legacy layout, time_warp_seq} or for EXPORT a 21-byte ceph_mds_cap_peer padded to 84 bytes, snap_trace_len bytes of snap trace, then v2 flockbl, v3 cap_peer (IMPORT), v4 inline_version + inline_data, v5 osd_epoch_barrier, v6 oldest_flush_tid, v7 caller_uid/gid, v8 pool_ns, v9 btime + change_attr, v10 flags (SYNC 1, PENDING_CAPSNAP 4), v11 nfiles/nsubdirs, v12 fscrypt_auth/file, v13 subvolume_id, v14 oserrno. The xattr blob is in the middle segment (xattr_len bytes). header.tid carries the client flush tid and is echoed in FLUSH_ACK/FLUSHSNAP_ACK. Ops: GRANT 0, REVOKE 1, TRUNC 2, EXPORT 3, IMPORT 4 (M to C); UPDATE 5, DROP 6, FLUSH 7, FLUSHSNAP 9, RELEASE 11, RENEW 12 (C to M); FLUSH_ACK 8, FLUSHSNAP_ACK 10 (M to C). The MDS encoder stops at v1 without FLOCK and v2 without EXPORT_PEER. Counted in push_seq.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients, deprecated-admin): Encoder stopping at v1 (no FLOCK) or v2 (no EXPORT_PEER); decoding client caps messages of versions other than 12 and 14; empty inline_data for connections without MDS_INLINE_DATA; non-empty inline_version/inline_data (deprecated inline_data setting). The MDS sends v14 to every baseline client and receives v12 from the 7.3 kernel and v14 from libcephfs at this commit. The v1/v2 encodings (no FLOCK or EXPORT_PEER) serve no client since Linux 2.6.36/3.14 and Ceph v0.75, other decoded versions (v6, v8, v10, v11, v13) come from older kernels and Ceph releases, and the empty inline_data without MDS_INLINE_DATA serves only kernels before 3.19. Non-empty inline data exists only after the deprecated inline_data file-system setting. Evidence: `src/messages/MClientCaps.h:347-360 (MClientCaps::encode_payload)`, `src/messages/MClientCaps.h:265-313 (MClientCaps::decode_payload)`, `src/messages/MClientCaps.h:362-368 (MClientCaps::encode_payload, inline_data)`, `src/messages/MClientCaps.h:27 (HEAD_VERSION 14)`, `linux/fs/ceph/caps.c:1293 (encode_cap_msg, version 12)`, `src/mds/CInode.cc:4467-4473 (CInode::encode_cap_message)`, `src/include/ceph_features.h:89 (FLOCK, kernel 2.6.36)`, `src/include/ceph_features.h:141 (EXPORT_PEER, kernel 3.14)`, `src/include/ceph_features.h:144 (MDS_INLINE_DATA, kernel 3.19)`, `src/mon/FSCommands.cc:471-478 (inline_data, deprecated)`.
- **Gates:** CEPH_FEATURE_FLOCK (bit 3); CEPH_FEATURE_EXPORT_PEER (bit 37); CEPH_FEATURE_MDS_INLINE_DATA (bit 40)
- **Map:** [wire-protocol.md › 3.11.2 MClientCaps](../cpp-map/02-client-protocol/wire-protocol.md#3112-mclientcaps) · [kclient-expectations.md › 7.1.2 Protocol fallbacks and the clients that exercise them](../cpp-map/02-client-protocol/kclient-expectations.md#712-protocol-fallbacks-and-the-clients-that-exercise-them)
- **Code:** `src/messages/MClientCaps.h:235-386 (MClientCaps::decode_payload, encode_payload)`, `src/include/ceph_fs.h:947-965 (ceph_mds_caps_head)`, `src/mds/CInode.cc:4432-4495 (CInode::encode_cap_message)`, `linux/fs/ceph/caps.c:1275 (encode_cap_msg)`
- **Notes:** The kernel sends v12 (v6 up to v4.9, v10 in v4.10-v6.5) and parses up to v12, ignoring v13/v14; its snap trace starts right after a combined 176-byte head+body struct. Ceph clients send v8 (10.2.x), v10 (11.2-12.2.5), v11 (12.2.6-17.2.x), v12 (18.x-20.x), v13 (21.x tags), v14 (main); libcephfs sends only UPDATE and FLUSHSNAP. The decoder asserts middle length == xattr_len. Semantics owned by 03-caps-leases-and-locking. Message type CEPH_MSG_CLIENT_CAPS 0x310, HEAD_VERSION 14, COMPAT_VERSION 1. The EXPORT peer is ceph_mds_cap_peer {u64 cap_id, u32 issue_seq, u32 mseq, u32 mds, u8 flags}; flag NO_CAPSNAP (2) is unused. Without CEPH_FEATURE_MDS_INLINE_DATA (bit 40) the inline_data field is sent empty. _do_cap_update still applies a non-empty flockbl, but the kernel always sends a zero-length flock blob and in-tree libcephfs never sets it (other clients unverified). See also F-CAPS-011 (GRANT/REVOKE semantics), F-CAPS-013 (processing), F-STORE-098 (osd_epoch_barrier).

<a id="f-proto-092"></a>

#### F-PROTO-092 · MClientCapRelease encoding and the OSD epoch barrier

*Batched cap releases from clients, carrying the client's OSD epoch barrier.*

v2/compat 1: raw ceph_mds_cap_release {u32 num}, num x raw 24-byte ceph_mds_cap_item {u64 ino, u64 cap_id, u32 migrate_seq, u32 issue_seq}, then v2 u32 osd_epoch_barrier. Handled only in clientreplay, active or stopping (otherwise queued for replay); a non-zero barrier the Objecter does not have yet becomes an Objecter epoch barrier (RADOS ops paused until that OSDMap arrives) and raises the MDS barrier, which it then advertises in cap messages. The kernel batches up to 170 items per message; libcephfs sends them on each tick to ranks in clientreplay, active or stopping and on FLUSHMSG.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.11.3 MClientCapRelease](../cpp-map/02-client-protocol/wire-protocol.md#3113-mclientcaprelease) · [kclient-expectations.md › 3.9.5 CAPRELEASE](../cpp-map/02-client-protocol/kclient-expectations.md#395-caprelease)
- **Code:** `src/messages/MClientCapRelease.h:30-61 (MClientCapRelease)`, `src/mds/Locker.cc:4258-4289 (Locker::handle_client_cap_release)`
- **Notes:** Release semantics are owned by 03-caps-leases-and-locking; the OSD epoch barrier by 06-journal-and-storage. See also F-STORE-098 (OSD epoch barrier), F-CAPS-028 (batched cap release).

<a id="f-proto-093"></a>

#### F-PROTO-093 · MClientLease encoding

*Layout of the dentry-lease control message and how each client uses its actions.*

Raw 35-byte ceph_mds_lease {u8 action, u16 mask, u64 ino (parent dir), u64 first, u64 last, u32 seq, u32 duration_ms} + string dname. Actions: REVOKE 1 (M to C), RELEASE 2 (C to M), RENEW 3 (both), REVOKE_ACK 4 (C to M). The MDS revokes with mask 1 and last = CEPH_NOSNAP, answers RENEW with a fresh seq and duration if the lease is still grantable (counted), removes a lease on RELEASE/REVOKE_ACK only if seq matches, drops messages for unknown inodes, dentries or leases, and reaches ceph_abort() for any other action on a held lease. The kernel answers a revoke (and any lease message for an unknown inode or dentry) with REVOKE_ACK and sends RENEW; libcephfs asserts the action is REVOKE, answers RELEASE and never renews. Counted in push_seq.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.11.4 MClientLease](../cpp-map/02-client-protocol/wire-protocol.md#3114-mclientlease) · [kclient-expectations.md › 3.12 Dentry leases and directory "leases"](../cpp-map/02-client-protocol/kclient-expectations.md#312-dentry-leases-and-directory-leases) · [libcephfs-fuse-ganesha-expectations.md › 3.8 Dentry leases and directory caching](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#38-dentry-leases-and-directory-caching)
- **Code:** `src/messages/MClientLease.h:79-89 (MClientLease)`, `src/mds/Locker.cc:4472-4539 (Locker::handle_client_lease)`, `src/mds/Locker.cc:4592 (Locker::revoke_client_leases)`, `linux/fs/ceph/mds_client.c:6091-6127 (handle_lease)`, `src/client/Client.cc:3657 (Client::handle_lease)`
- **Notes:** The revoke mask is "1 | CEPH_LOCK_DN" = 1 in userspace numbering; userspace and kernel CEPH_LOCK_* numbering differ, so no lock-type numbers can be exchanged with the kernel (wire-protocol 8). Lease semantics are owned by 03-caps-leases-and-locking. See also F-CAPS-042 (renew), F-CAPS-043 (revocation), F-CAPS-044 (expiry).

<a id="f-proto-094"></a>

#### F-PROTO-094 · MClientSnap encoding

*Layout of snap realm update and split notifications, and what the MDS actually puts on the wire.*

Raw 24-byte ceph_mds_snap_head {u32 op, u64 split, u32 num_split_inos, u32 num_split_realms, u32 trace_len}, then the split inos and split realms (u64 each) and trace_len bytes of snap trace. Ops UPDATE 0, CREATE 1, DESTROY 2, SPLIT 3. Per-realm notifications (mksnap, rmsnap, renamesnap, realm creation, realm moves by link/unlink/rename) always use op SPLIT with split = the realm inode and that client's trace, with split lists filled only for a real split; realm split/merge also send SPLIT; a committed snap-table transaction sends every open or stale session UPDATE (DESTROY for rmsnap) for the global snaprealm inode 3; UPDATE is also the reconnect catch-up. CREATE is never sent. All sends are counted. libcephfs asserts that nothing follows the trace and that a SPLIT trace starts with head.split.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [wire-protocol.md › 3.11.5 MClientSnap](../cpp-map/02-client-protocol/wire-protocol.md#3115-mclientsnap) · [libcephfs-fuse-ganesha-expectations.md › 3.10 Snapshots and snap realms](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#310-snapshots-and-snap-realms)
- **Code:** `src/messages/MClientSnap.h:48-66 (MClientSnap)`, `src/mds/MDCache.cc:10225-10276 (MDCache::do_realm_invalidate_and_update_notify)`, `src/mds/MDCache.cc:10348-10363 (MDCache::notify_global_snaprealm_update)`, `src/client/Client.cc:5620 (Client::handle_snap)`
- **Notes:** Snapshot semantics are owned by 08-snapshots-quotas-subvolumes. See also F-SNAP-009 (per-realm notifications) and F-SNAP-010 (global realm broadcast).

<a id="f-proto-095"></a>

#### F-PROTO-095 · MClientQuota encoding

*Layout of the quota and recursive-statistics push to clients holding caps on a quota realm inode.*

Payload: u64 ino, utime rctime, i64 rbytes, i64 rfiles, i64 rsubdirs, quota_info_t ENCODE_START(1,1) {i64 max_bytes, i64 max_files}; rsnaps is not sent. MDCache::broadcast_quota_to_client sends it from the auth rank (active or stopping, inode not frozen, quota enabled or just changed) to cap holders not marked noquota: unconditionally on quota updates, otherwise when rstat changed and usage reached (bytes: within 1/8 of) the limit or moved by more than 1/16 of the remaining distance since the last notification. Counted in push_seq. libcephfs replaces quota and rstat wholesale (zeroing rsnaps until the next InodeStat); both clients enforce quotas locally.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Skipping cap holders marked noquota (Capability STATE_NOQUOTA, set for connections without MDS_QUOTA). Both baseline client families negotiate MDS_QUOTA (bit 47), so no baseline cap is marked noquota and every quota-realm cap holder receives MClientQuota. The noquota exclusion applies only to connections without MDS_QUOTA (kernels before 4.17). Evidence: `src/mds/MDCache.cc:2145 (MDCache::broadcast_quota_to_client)`, `src/mds/Capability.cc:179-180 (Capability::Capability, STATE_NOQUOTA)`, `linux/include/linux/ceph/ceph_features.h:212 (MDS_QUOTA advertised)`, `src/include/ceph_features.h:156 (MDS_QUOTA, kernel 4.17)`.
- **Map:** [wire-protocol.md › 3.11.6 MClientQuota](../cpp-map/02-client-protocol/wire-protocol.md#3116-mclientquota) · [kclient-expectations.md › 3.11 Quotas](../cpp-map/02-client-protocol/kclient-expectations.md#311-quotas) · [libcephfs-fuse-ganesha-expectations.md › 3.11 Quotas, statfs and pool permissions](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#311-quotas-statfs-and-pool-permissions)
- **Code:** `src/messages/MClientQuota.h:30-49 (MClientQuota)`, `src/mds/MDCache.cc:2103-2190 (MDCache::broadcast_quota_to_client)`, `linux/fs/ceph/quota.c:40 (ceph_handle_quota)`, `src/client/Client.cc:5693 (Client::handle_quota)`
- **Notes:** MClientQuota asserts on trailing bytes in the C++ decoder. Quota semantics are owned by 08-snapshots-quotas-subvolumes. See also F-SNAP-045 (push policy and thresholds).

<a id="f-proto-096"></a>

#### F-PROTO-096 · MClientMetrics encoding and client send rules

*Layout of the client metrics upload and when each client sends it.*

Payload: u32 count of ClientMetricMessage, each u32 metric_type + a payload wrapped in ENCODE_START; types 0 CAP_INFO, 1 READ_LATENCY, 2 WRITE_LATENCY, 3 METADATA_LATENCY, 4 DENTRY_LEASE, 5 OPENED_FILES, 6 PINNED_ICAPS, 7 OPENED_INODES, 8 READ_IO_SIZES, 9 WRITE_IO_SIZES, 10-15 average/stdev types (no MDS decoder), 16 SUBVOLUME_METRICS (ENCODE_START(1,1) {u64 count, count x AggregatedIOMetrics {subvolume_id, read/write counts, bytes, latency_us, time_stamp}}); unknown types are skipped by their length. The MDS processes it in MetricsHandler without mds_lock and drops it while the rank is not active. The kernel sends once per second to one session with METRIC_COLLECT whose rank is exactly up:active (with subvolume metrics, also bit 22), and its type 16 item is a bare u32 type followed by the payload; libcephfs sends every tick to rank 0 only (priority CEPH_MSG_PRIO_HIGH - 1), including only the types in the MDS metric spec.

- **Class:** POSIX `operational` · exposure `client-wire`, `metrics` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Gates:** MDS bit CEPHFS_FEATURE_METRIC_COLLECT (14): Linux v5.9+, Ceph 16.2.0+; MDS metric spec bit 16 (libcephfs subvolume metrics); kernel module parameter disable_send_metrics
- **Map:** [wire-protocol.md › 3.11.9 MClientMetrics](../cpp-map/02-client-protocol/wire-protocol.md#3119-mclientmetrics) · [kclient-expectations.md › 3.17 Metrics](../cpp-map/02-client-protocol/kclient-expectations.md#317-metrics) · [libcephfs-fuse-ganesha-expectations.md › 3.15 Metrics](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#315-metrics)
- **Code:** `src/messages/MClientMetrics.h:12-59 (MClientMetrics)`, `src/include/cephfs/metrics/Types.h:804-890 (ClientMetricMessage)`, `src/mds/MetricsHandler.cc:418-444 (MetricsHandler::handle_client_metrics)`, `linux/fs/ceph/metric.c:91 (ceph_mdsc_send_metrics)`, `src/client/Client.cc:7640-7660 (Client::collect_and_send_global_metrics)`
- **Notes:** Without METRIC_COLLECT the kernel never sends metrics ("MDS will close the session's socket"). libcephfs subtracts a CLOCK_REALTIME stamp from a CLOCK_MONOTONIC one for metadata latency, so that metric is not a real latency (libcephfs-fuse-ganesha 8). Metrics aggregation is owned by 10-qos-metrics-quiesce-quarantine. The kernel ignores failed ops except -ENOENT and -ETIMEDOUT in its latency statistics. Each rank that holds a session but receives no metrics for it forwards zeroed values to rank 0 (F-QOS-014). See also F-QOS-011 (negotiation), F-QOS-012 (ingestion).

<a id="f-proto-097"></a>

#### F-PROTO-097 · MQuarantineDisable encoding

*Layout of the notification that lifts quarantine on an inode for clients that understand quarantine.*

Payload: u64 ino, i32 oserrno (always 0, tells the client to clear its per-inode error). Sent counted, only to sessions with CEPHFS_FEATURE_QUARANTINE (24), when quarantine is lifted on an inode they hold caps on. libcephfs handles it in Client::handle_quarantine_disable without counting it and without checking which session it came from.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by libcephfs, ceph-fuse, ganesha
- **Gates:** client bit CEPHFS_FEATURE_QUARANTINE (24): no kernel, no Ceph release tag yet (main only)
- **Map:** [wire-protocol.md › 3.11.10 MQuarantineDisable](../cpp-map/02-client-protocol/wire-protocol.md#31110-mquarantinedisable) · [sessions.md › 8. Edge cases, failure handling and gotchas](../cpp-map/02-client-protocol/sessions.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/messages/MQuarantineDisable.h:29-39 (MQuarantineDisable)`, `src/mds/MDCache.cc:14096-14107 (MDCache::dispatch_quiesce_inode)`, `src/client/Client.cc:5718-5731 (Client::handle_quarantine_disable)`
- **Notes:** The counting mismatch makes a later libcephfs REQUEST_CLOSE carry a seq below push_seq, which the MDS drops (derived from code; F-PROTO-050). Quarantine is owned by 10-qos-metrics-quiesce-quarantine. Message type CEPH_MSG_CLIENT_QUARANTINE_DISABLE (0x315), head and compat version 1; the kernel never receives it (it would only log an unknown type). After the dropped REQUEST_CLOSE the C++ client gives up after client_shutdown_timeout (30 s) and the session lingers until it goes stale (code reading, not covered by tests). See also F-QOS-077 (when quarantine sends it).

### Client-specific expectations (kclient, libcephfs, ceph-fuse, NFS-Ganesha, Samba)

Behaviours of particular clients that shape what the MDS sees: mount, inode-number addressing, inode number ranges, how NFS-Ganesha and Samba share sessions, and Ganesha delegations.

<a id="f-proto-098"></a>

#### F-PROTO-098 · Mount flow requirements

*What a mount sends to the MDS before any application request: map subscription, root GETATTR by a root-relative path, and ancestor lookups.*

The kernel subscribes to the mdsmap (or fsmap.user and mdsmap.<fscid> with mds_namespace; a name mismatch shuts the mount down), keeps requests waiting while the map epoch is 0, fails them with -EHOSTUNREACH when no rank is active unless norequire_active_mds is set, and opens the root with GETATTR on path (1, server path without leading "/"), mask PIN, any rank, timeout mount_timeout (60 s); the target inode becomes the superblock root, so mounting depends on GETATTR resolving a root-relative multi-component path. libcephfs fails the mount with -EACCES if the MDSMap has refuse_client_session, then issues GETATTR (CEPH_STAT_CAP_INODE_ALL) on the mount path and on each ancestor up to "/" so quota realms above the mount are visible (-EACCES on an ancestor is tolerated). ceph-fuse first waits for the cluster to be available (fuse_require_active_mds). Sessions are opened lazily by the first request.

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** kernel mount options mds_namespace, require_active_mds, mount_timeout; fuse_require_active_mds (default true)
- **Map:** [kclient-expectations.md › 3.2 Mount flow](../cpp-map/02-client-protocol/kclient-expectations.md#32-mount-flow) · [libcephfs-fuse-ganesha-expectations.md › 3.2 Mount, map subscription and session open](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#32-mount-map-subscription-and-session-open)
- **Code:** `linux/fs/ceph/super.c:1065 (open_root_dentry)`, `linux/fs/ceph/mds_client.c:3664 (__do_request)`, `src/client/Client.cc:7042-7117 (Client::mount)`, `src/client/Client.cc:7002 (Client::subscribe_mdsmap)`
- **Notes:** Mount-time GETATTRs from libcephfs run with caller_uid/gid -1 unless client_mount_uid/gid are set (libcephfs-fuse-ganesha 8).

<a id="f-proto-099"></a>

#### F-PROTO-099 · Inode-number addressed access (NFS file handles, #ino paths, LOOKUPINO)

*NFS servers and libcephfs address objects by inode number; the MDS resolves any inode number a client presents.*

libcephfs builds almost every request relative to an inode number (#ino or #parent/name). Ganesha file handles carry {ino, snapid, fscid}; on a cache miss ceph_ll_lookup_vino sends LOOKUPINO to rank rand() % num_in with the mount's default credentials (uid/gid -1 unless set), with args.lookupino.snapid for snapped inodes, and LOOKUPPARENT resolves ".." of a handle-only directory. The kernel NFS export uses LOOKUPINO (fh_to_dentry, with snapid, parent and hash for snapped handles; a reply whose snapid differs is treated as unsupported), LOOKUPPARENT (get_parent) and LOOKUPNAME (get_name; reply needs dentry and target). The MDS opens uncached inodes for LOOKUPINO with open_ino and returns -ESTALE for purging or MDS-private inodes. Inodes known only by handle have no path to the mount root, so libcephfs cannot evaluate MDS auth caps locally: their setattr is never buffered and every open goes to the MDS.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ganesha, ceph-fuse
- **Map:** [libcephfs-fuse-ganesha-expectations.md › 3.16.5 Handle-based access and lookup by inode number](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3165-handle-based-access-and-lookup-by-inode-number) · [kclient-expectations.md › 3.20 NFS export operations](../cpp-map/02-client-protocol/kclient-expectations.md#320-nfs-export-operations)
- **Code:** `src/mds/Server.cc:4433-4463 (Server::handle_client_lookup_ino)`, `linux/fs/ceph/export.c:131-618 (__lookup_inode, __snapfh_to_dentry, __get_parent, ceph_get_name)`, `src/client/Client.cc:14545-14599 (Client::ll_lookup_vino)`
- **Notes:** Ganesha never causes LOOKUPHASH or LOOKUPNAME (libcephfs-fuse-ganesha 3.16.5). LOOKUPINO semantics are owned by 04-namespace-and-posix-operations.

<a id="f-proto-100"></a>

#### F-PROTO-100 · Inode number ranges clients accept

*Constraints on the inode numbers an MDS hands out, imposed by the kernel reserved range and by ceph-fuse inode numbering.*

The kernel refuses inode numbers in [0x100, 0x1000) (MDS-reserved: mdsdirs, logs, strays) with -EREMOTEIO in replies and -ESTALE in NFS paths, and ignores delegated ranges starting below 0x1000; libcephfs returns -ESTALE locally for reserved vinos. ceph-fuse exposes FUSE inode numbers (stag << 48) | ino, with stag 0 for head, 1 for snapdir and other values mapped per snapid, so MDS inode numbers at or above 2^48 alias in FUSE (unless client_use_faked_inos) and one inode can be exposed under at most 65534 snapids at a time (else ceph_abort_msg("run out of stag")).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, ceph-fuse, libcephfs
- **Gates:** client_use_faked_inos (ceph-fuse)
- **Map:** [kclient-expectations.md › 3.8.1 Target and parent identity](../cpp-map/02-client-protocol/kclient-expectations.md#381-target-and-parent-identity) · [libcephfs-fuse-ganesha-expectations.md › 3.17 ceph-fuse specifics](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#317-ceph-fuse-specifics)
- **Code:** `linux/fs/ceph/super.h:666 (ceph_vino_is_reserved)`, `src/client/fuse_ll.cc:59 (FINO_INO)`, `src/client/fuse_ll.cc:1889 (make_fake_ino)`
- **Notes:** See also F-STORE-046 (special inode numbers).

<a id="f-proto-101"></a>

#### F-PROTO-101 · Shared sessions: NFS-Ganesha and Samba libcephfsd

*One MDS session per rank can carry the requests of many NFS clients or SMB processes, distinguished only by credentials in each request.*

FSAL_CEPH (nfs-ganesha V15.7) creates one cmount per {filesystem, cmount_path, user_id, key} and shares it among exports; mgr/nfs gives all exports of one (cluster, fs, cmount_path) the same cephx user and defaults cmount_path to "/", so by default a Ganesha daemon holds one session per rank per file system. Many NFS worker threads have requests outstanding on that session at once, distinguished by caller_uid/gid and gid_list; requests can reach a rank out of tid order. ceph_ll_open always runs as uid 0/gid 0 (ceph_reopen_func), so every OPEN Ganesha issues through it, and the lock and read/write-path requests on such an Fh, are checked as root (a root_squash MDS cap grant refuses Ganesha write opens unless another grant matches); lock owners are Ganesha object addresses with pid 0 and Ganesha never sends a blocking lock. Samba vfs_ceph_new with proxy=yes goes through the in-tree libcephfsd daemon, which shares one libcephfs mount among SMB processes with identical configuration.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by ganesha, smb, libcephfs
- **Map:** [libcephfs-fuse-ganesha-expectations.md › 3.16.1 Concurrency model seen by the MDS](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3161-concurrency-model-seen-by-the-mds) · [libcephfs-fuse-ganesha-expectations.md › 3.16.8 FSAL_CEPH call mix (nfs-ganesha V15.7)](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3168-fsal_ceph-call-mix-nfs-ganesha-v157) · [libcephfs-fuse-ganesha-expectations.md › 1. Purpose and responsibilities](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#1-purpose-and-responsibilities)
- **Code:** `nfs-ganesha/src/FSAL/FSAL_CEPH/main.c:1188-1214 (create_export)`, `nfs-ganesha/src/FSAL/FSAL_CEPH/handle.c:990-1004 (ceph_reopen_func)`, `src/libcephfs_proxy/proxy_mount.c:81-118`, `src/client/Client.cc:2251 (Client::make_request)`
- **Notes:** Derived from code (libcephfs-fuse-ganesha 8): Ganesha opens as uid 0, and a write through such an Fh never clears setuid/setgid bits through libcephfs. FSAL_CEPH sets client_acl_type=posix_acl and client_oc (default false) on every cmount. The SMB server source is not in the repository.

<a id="f-proto-102"></a>

#### F-PROTO-102 · NFS-Ganesha delegations built on caps

*NFSv4 delegations in Ganesha are implemented inside libcephfs by holding cap references; the MDS sees only cap revokes that take longer to be acknowledged.*

ceph_set_deleg_timeout(t) (t < session_autoclose, else -EINVAL) enables delegations per mount; Inode::set_deleg grants a read delegation only with PIN, As, Ls, Xs, Fs, Fr, Fc already issued (write adds Fx, Fw, Fb) and holds cap references, so those caps are not released. A revoke of a delegated cap (or a conflicting local open, unlink, link or rename) invokes the application recall callback; the revoke is acknowledged only after the delegation is returned, and after deleg_timeout without return the whole client is force-unmounted. With delegations enabled a successful OPEN also waits until Fr/Fw are actually issued. FSAL_CEPH V15.7 enables them only with NFSv4 Delegations = true (timeout Lease_Lifetime * 2 + 5, 125 s by default) and returns a recalled delegation from a fridge thread right after the callback; the cephadm template disables delegations. The MDS has no delegation-specific code.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by ganesha, libcephfs
- **Gates:** ceph_set_deleg_timeout (libcephfs API); Ganesha NFSv4 Delegations (default false; cephadm template false)
- **Map:** [libcephfs-fuse-ganesha-expectations.md › 3.16.4 Delegations built on caps](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3164-delegations-built-on-caps) · [libcephfs-fuse-ganesha-expectations.md › 3.16.9 MDS paths that only Ganesha exercises](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#3169-mds-paths-that-only-ganesha-exercises)
- **Code:** `src/client/Delegation.cc:41 (ceph_deleg_caps_for_type)`, `src/client/Inode.cc:738 (Inode::set_deleg)`, `src/client/Client.cc:6243-6247 (Client::handle_cap_grant)`, `src/client/Client.cc:11169-11185 (Client::_open)`
- **Notes:** mds_cap_revoke_eviction_timeout (default 0) would evict a client holding a revoke longer than that. The client_deleg_timeout option named in the libcephfs documentation is not defined (libcephfs-fuse-ganesha 5.2). Cap revocation is owned by 03-caps-leases-and-locking. Delegations depend on ordinary GRANT/REVOKE and the loner/EXCL policy (F-CAPS-005, F-CAPS-011).

### Cluster client libraries sharing the MDS identity (MonClient, MgrClient, Objecter)

Obligations of the client libraries embedded in the MDS process, which share its messenger, address and cephx identity; detailed mon, mgr and OSD protocols are owned by other categories.

<a id="f-proto-103"></a>

#### F-PROTO-103 · MonClient obligations

*How the MDS process talks to the monitors as a client: bootstrap, hunting, authentication, tickets, subscriptions, commands and the cluster log.*

Before the daemon exists, global_init fetches the MonMap and MConfig on a temporary CLIENT-typed messenger with the MDS key. The MonClient then hunts mon_client_hunt_parallel (3) monitors every mon_client_hunt_interval (3 s, backoff x1.5 up to x10) and keeps the first that authenticates; it sends keepalives every 10 s and re-hunts after 30 s without KEEPALIVE2_ACK or on reset. It authenticates with msgr2 AUTH_MODE_MON (10) + EntityName + global_id (msgr1: MAuth with CEPH_AUTH_UNKNOWN), presenting the previous auth ticket so the global id (the FSMap gid) is kept, requests tickets for MON, OSD, MDS and MGR and renews them at three quarters of their validity. Subscriptions: monmap, config, mdsmap (from 0), mgrmap (from the first FSMap appearance), osdmap (Objecter); the monitor serves the mdsmap subscription by exact address-vector match against the FSMap. MConfig is applied, MLog entries carry sequence numbers and are dropped only on MLogAck, pending MMonCommands are resent on a new session, queued messages are dropped on re-hunt, and MMonGetVersion serves map waits. MDS code calls the MonClient while holding mds_lock (order mds_lock then monc_lock).

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config` · relied on by mon
- **Legacy parts** (msgr1): msgr1 monitor authentication (MonConnection::start sending MAuth with CEPH_AUTH_UNKNOWN). The MDS binds msgr2 and authenticates to monitors at this commit over msgr2 with AUTH_MODE_MON. The MAuth exchange with protocol CEPH_AUTH_UNKNOWN is used only on a msgr1 connection to a monitor. Evidence: `src/mon/MonClient.cc:1921-1945 (MonConnection::start)`, `src/mon/MonClient.cc:1950-1990 (MonConnection::get_auth_request)`.
- **Gates:** mon_client_hunt_parallel (3); mon_client_hunt_interval (3 s); mon_client_ping_interval (10 s); mon_client_ping_timeout (30 s); auth_mon_ticket_ttl (72 h)
- **Map:** [transport-auth-and-client-library-contract.md › 3.7 MonClient](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#37-monclient) · [transport-auth-and-client-library-contract.md › 7.5 Monitors](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#75-monitors) · [transport-auth-and-client-library-contract.md › 6. Concurrency and mds_lock](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#6-concurrency-and-mds_lock)
- **Code:** `src/mon/MonClient.cc:818-934 (MonClient::_reopen_session)`, `src/mon/MonClient.cc:1154-1171 (MonClient::_check_auth_tickets)`, `src/mon/MonClient.cc:1950-1990 (MonConnection::get_auth_request)`, `src/mon/MDSMonitor.cc:1901-1990 (MDSMonitor::check_sub)`, `src/common/LogClient.cc:215-354 (LogClient::_get_mon_log_message)`
- **Notes:** A new global id means a new gid, which the FSMap treats as a different daemon (transport page 8, item 21). MonClient subscriptions and beacons are owned by 01-cluster-membership-and-ha; the transport page lists the full duty table (3.7). See also F-CLU-110 (MonClient subscriptions).

<a id="f-proto-104"></a>

#### F-PROTO-104 · MgrClient obligations

*How the MDS process reports to the active manager: session open keyed by daemon name, configure, and periodic perf-counter reports.*

When the daemon first appears in the FSMap the MgrClient starts and subscribes to mgrmap; on each new active mgr it connects (lossy, secure mode by default, at most once per mgr_connect_retry_interval, 1 s) and sends MMgrOpen v3 (daemon_name = <id>, empty service name, running config and defaults). The mgr keys the session as ("mds", id) and disconnects until it has fetched mds metadata from the monitor. MMgrConfigure (stats_period from mgr_stats_period 5 s, stats_threshold 5, OSD and MDS perf-metric queries) starts the report timer; every period an MMgrReport v9 declares new counters (path, description, nick, type, priority, unit) and packs values in lexicographic order of the declared paths, with undeclare lists, config when changed, task_status (scrub status, every 2 s while non-empty) and the rank 0 metric report. The mgr requires SERVER_LUMINOUS from MDS peers and ignores daemon_health_metrics from an MDS; mgr modules never open connections to an MDS (they use libcephfs).

- **Class:** POSIX `operational` · exposure `cluster-wire`, `metrics` · relied on by mgr
- **Gates:** mgr_stats_period (5 s); mgr_stats_threshold (5); mgr_connect_retry_interval (1 s)
- **Map:** [transport-auth-and-client-library-contract.md › 3.8 MgrClient](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#38-mgrclient) · [transport-auth-and-client-library-contract.md › 7.6 Manager](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#76-manager)
- **Code:** `src/mgr/MgrClient.cc:253-271 (MgrClient::_send_open)`, `src/mgr/MgrClient.cc:341-453 (MgrClient::_send_report)`, `src/mgr/DaemonPerfCounters.cc:32-82 (DaemonPerfCounters::update)`, `src/mgr/DaemonServer.cc:601-636 (DaemonServer::handle_open)`
- **Notes:** MMgrReport.packed has no per-value keys, so a different packing order shifts every value on the mgr side (transport page 8, item 26). Perf counters and mgr modules are owned by 11-security-admin-and-observability, the metric report by 10-qos-metrics-quiesce-quarantine. See also F-ADM-067 (counter reporting).

<a id="f-proto-105"></a>

#### F-PROTO-105 · Objecter obligations (OSD client protocol)

*How the MDS process reads and writes RADOS: OSDMap following, placement, resend and pause rules, epoch barrier, backoffs and fencing.*

The Objecter applies OSDMap incrementals in epoch order (full maps when an incremental is missing, skipping trimmed epochs and then resending every op), re-targets every op on each epoch with placement identical to the C++ OSDMap/CRUSH code (object hash, stable_mod, crush rule, upmaps, primary affinity, pg_temp), resends on interval change, split/merge, forced resend or unpause, pauses reads on PAUSERD, writes on PAUSEWR and everything while the map epoch is below its epoch barrier (set_epoch_barrier only raises it). The MDS ignores pool full flags (FULL_FORCE on every op). MOSDOp is encoded by the OSD connection features (v9 with SERVER_SQUID, v8, v7); replies with a stale retry_attempt are ignored, redirects resubmitted, -EAGAIN resent to the primary. With RADOS_BACKOFF, BLOCK is acknowledged with ACK_BLOCK, ops in the blocked range are held and resent on UNBLOCK. OSD sessions are lossy: a reset reopens the session and resends all ops in tid order; laggy ops (older than objecter_timeout, 10 s) trigger an MPing. -EBLOCKLISTED (108) means the MDS has been fenced and it respawns.

- **Class:** POSIX `supporting` · exposure `cluster-wire` · relied on by osd
- **Legacy parts** (old-daemons): MOSDOp encodings v8 and older, chosen from OSD connection features. OSDs at this commit advertise SERVER_SQUID (in CEPH_FEATURES_ALL), so the Objecter encodes MOSDOp as v9. The v8 encoding (no SERVER_SQUID, OSDs older than 19.x), v7 (no RESEND_ON_SPLIT) and older encodings serve only OSDs of older releases in a mixed-version cluster. Evidence: `src/messages/MOSDOp.h:318-319 (MOSDOp::encode_payload, v6)`, `src/messages/MOSDOp.h:348-350 (MOSDOp::encode_payload, v7)`, `src/messages/MOSDOp.h:372-375 (MOSDOp::encode_payload, v8)`, `src/messages/MOSDOp.h:402-404 (MOSDOp::encode_payload, v9)`, `src/include/ceph_features.h:258 (SERVER_SQUID in CEPH_FEATURES_ALL)`.
- **Gates:** objecter_inflight_ops (1024); objecter_inflight_op_bytes (100 MiB); objecter_tick_interval (5 s); objecter_timeout (10 s); osd_client_op_priority (63)
- **Map:** [transport-auth-and-client-library-contract.md › 3.9 Objecter](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#39-objecter) · [transport-auth-and-client-library-contract.md › 7.7 OSDs](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#77-osds)
- **Code:** `src/osdc/Objecter.cc:1253-1485 (Objecter::handle_osd_map)`, `src/osdc/Objecter.cc:3001-3011 (Objecter::target_should_be_paused)`, `src/osdc/Objecter.cc:3126-3406 (Objecter::_calc_target)`, `src/osdc/Objecter.cc:4069-4166 (Objecter::handle_osd_backoff)`, `src/osdc/Objecter.cc:5563-5574 (Objecter::set_epoch_barrier)`
- **Notes:** The MDS-specific RADOS operation catalogue and full handling are owned by 06-journal-and-storage. librados is a separate identity (client.<global_id>, own nonce, client_inc 0), unlike the Objecter embedded in the MDS messenger (transport page 8, item 30). See also F-STORE-090 (per-rank Objecter).

<a id="f-proto-106"></a>

#### F-PROTO-106 · Fencing identity: one address vector for clients, monitors and OSDs

*Monitors find the MDS and OSDs fence it by address, so the address vector the MDS beacons is the one it presents on every connection.*

The monitor serves an MDS-named session's mdsmap subscription by searching the FSMap for the daemon whose mds_info_t::addrs equal the session's address vector (msgr2 CLIENT_IDENT addrvec or msgr1 banner address); a daemon not found gets an empty map or only its standby entry. On failover the monitor blocklists every address in the failed daemon's addrs (only for a daemon that held a rank and was not in standby-replay), and an OSD rejects ops with -EBLOCKLISTED when the connection's first peer address matches the blocklist exactly, by IP, or by range. The C++ MDS satisfies both with its single messenger: msgr2 CLIENT_IDENT carries messenger->get_myaddrs() and the msgr1 banner get_myaddr_legacy(); MDS OSD traffic also carries the entity mds.<rank> and client_inc = the MDSMap incarnation.

- **Class:** POSIX `supporting` · exposure `cluster-wire` · relied on by mon, osd
- **Legacy parts** (msgr1): Identity presented in the msgr1 banner (legacy address) on msgr1 connections to monitors and OSDs. The MDS's MonClient and Objecter connect over msgr2, where CLIENT_IDENT carries the full address vector that the monitor matches against the FSMap and OSDs fence. The msgr1 banner address (get_myaddr_legacy) is the presented identity only on msgr1 connections. Evidence: `src/msg/async/ProtocolV2.cc:1973-1981 (ProtocolV2::send_client_ident)`, `src/msg/async/ProtocolV1.cc:1483 (ProtocolV1::handle_server_banner_and_identify)`, `src/mon/MDSMonitor.cc:1909 (MDSMonitor::check_sub)`.
- **Map:** [transport-auth-and-client-library-contract.md › 3.2 Identity, addresses and what is authenticated](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#32-identity-addresses-and-what-is-authenticated) · [transport-auth-and-client-library-contract.md › 7.1 Every peer](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#71-every-peer) · [transport-auth-and-client-library-contract.md › 7.7 OSDs](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#77-osds)
- **Code:** `src/mon/MDSMonitor.cc:1377-1389 (MDSMonitor::fail_mds_gid)`, `src/mon/OSDMonitor.cc:4979-4991 (OSDMonitor::blocklist)`, `src/osd/OSDMap.cc:1478-1518 (OSDMap::is_blocklisted)`, `src/msg/async/ProtocolV2.cc:1973-1981 (ProtocolV2::send_client_ident)`
- **Notes:** An MDS whose mon-session address vector differs from its beaconed addrs receives no MDSMap for its file system (transport page 8, item 24). Failover fencing is owned by 01-cluster-membership-and-ha. See also F-CLU-036 (blocklist fencing).

<a id="f-proto-107"></a>

#### F-PROTO-107 · Peer-MDS connections

*Transport settings between MDS daemons of the same file system; the project constraint means only the new daemons speak this protocol to each other.*

C++ uses Policy::lossless_peer(CEPH_FEATURE_UID) for MDS peers (no resetcheck), protocol 36 (CEPH_MDS_PROTOCOL) on msgr1, the cluster auth method and ms_cluster_mode (crc by default), and connection-race resolution by address order (WAIT) in both protocols. MDSRank::send_message_mds first sends its MDSMap to a peer whose known epoch is older; peers whose address left the map or whose incarnation changed are marked down. MDS-to-MDS messages flow only between daemons of the same file system.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Legacy parts** (msgr1): msgr1 protocol number 36 (CEPH_MDS_PROTOCOL) check and msgr1 connection-race resolution by legacy address. C++ MDS daemons at this commit connect to the v2 address each peer publishes first, and msgr2 has no protocol-number check. The protocol number 36 check (BADPROTOVER) and the msgr1 race rule by legacy address order apply only to msgr1 connections between MDS daemons. Evidence: `src/mds/MDSDaemon.h:30 (CEPH_MDS_PROTOCOL)`, `src/msg/async/AsyncMessenger.cc:1099-1116 (AsyncMessenger::get_proto_version)`, `src/msg/async/ProtocolV1.cc:2004-2013 (ProtocolV1::handle_connect_message_2, BADPROTOVER)`, `src/msg/async/ProtocolV1.cc:2256 (ProtocolV1::handle_connect_message_2, legacy address order)`, `src/msg/async/AsyncMessenger.cc:847-858 (AsyncMessenger::create_connect)`.
- **Map:** [transport-auth-and-client-library-contract.md › 7.8 Other MDS daemons](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#78-other-mds-daemons) · [transport-auth-and-client-library-contract.md › 3.1 One messenger, six peer classes, policies and throttles](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#31-one-messenger-six-peer-classes-policies-and-throttles)
- **Code:** `src/ceph_mds.cc:188-189 (main)`, `src/mds/MDSRank.cc:1476-1502 (MDSRank::send_message_mds)`, `src/mds/MDSDaemon.h:30 (CEPH_MDS_PROTOCOL)`
- **Notes:** The new MDS never runs mixed with C++ MDS daemons in one file system, so no C++ daemon expects this protocol from it (transport page 7.8). Inter-rank messages are owned by 07-multi-mds. See also F-CLU-109, F-CLU-072.

### Legacy entries

These entries serve only clients older than the [baseline](../README.md#baseline-current-peers), msgr1 mounts or older C++ releases, or describe code that cannot run. Each keeps its ID and group.

<a id="f-proto-001"></a>

#### F-PROTO-001 · msgr1 (legacy protocol) server side for client connections

*Accepting side of the legacy messenger protocol, used only by a Linux kernel mounted without ms_mode (old mon:/path device syntax, mon_addr= without ms_mode, or ms_mode=legacy) and by Ceph clients 13.x and older.*

The server writes the banner "ceph v027", its legacy 136-byte address and the client socket address, then reads the client banner, the 33-byte ceph_msg_connect and the authorizer. A protocol_version other than 32 (CEPH_MDSC_PROTOCOL) from a client peer gets the reply tag BADPROTOVER; other reply tags are READY, SEQ, RETRY_SESSION, RETRY_GLOBAL, RESETSESSION, WAIT, BADAUTHORIZER, FEATURES and CHALLENGE_AUTHORIZER, and replacing an existing connection costs one extra RETRY_GLOBAL round trip. In the open state every MSG carries a 53-byte header (crc32c over the first 49 bytes), front/middle/data crcs and, when MSG_AUTH was negotiated, a 21-byte footer with a cephx signature (13-byte old footer otherwise); ACK carries le64 in_seq and KEEPALIVE2 is answered with KEEPALIVE2_ACK. The kernel requires MSG_AUTH in the READY/SEQ reply features (unless mounted with nocephx_sign_messages), accepts only MSG, ACK, KEEPALIVE2_ACK and CLOSE in the open state, rejects WAIT, accepts only in_seq + 1 as the next sequence number, and limits front and middle to 16 MiB and data to 64 MiB.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse
- **Group:** Transport: msgr2 (and legacy msgr1) as clients see them
- **Legacy** (msgr1): The C++ MDS speaks msgr1 only to peers that dial its v1 address: a kernel mounted without ms_mode (old mon:/path device syntax, mon_addr= without ms_mode, or ms_mode=legacy) and Ceph clients 13.x and older. mount.ceph passes ms_mode=prefer-crc for the new device syntax since commit 9663f8aaec9 (first released in 17.2.0), and libcephfs, ceph-fuse and NFS-Ganesha dial the first entry of the address vector, which is the v2 one. Rook v1.21.0 gives ceph-csi kernel mounts ms_mode=prefer-crc and the v2 monitor port only when requireMsgr2 or compression is set (ms_mode=secure with encryption; its example cluster.yaml sets requireMsgr2: true), and passes no ms_mode with monitor port 6789 under the CRD default requireMsgr2=false. Evidence: `linux/net/ceph/ceph_common.c:329-330 (ceph_alloc_options)`, `linux/include/linux/ceph/libceph.h:146-149 (ceph_msgr2)`, `src/mount/mount.ceph.c:43 (CEPH_DEFAULT_V2_MS_MODE)`, `src/mount/mount.ceph.c:306-309 (parse_new_dev)`, `src/mount/mount.ceph.c:597-615 (parse_options: ms_mode, mon_addr)`, `src/msg/async/AsyncMessenger.cc:847-858 (AsyncMessenger::create_connect)`, `src/common/pick_address.cc:473-488 (pick_addresses)`, `rook/pkg/operator/ceph/controller/cluster_info.go:221-232 (loadCsiSettings)`, `rook/pkg/operator/ceph/cluster/mon/mon.go:671-674 (newMonConfig)`, `rook/pkg/operator/ceph/csi/cluster_config.go:82-98 (MonEndpoints)`, `rook/Documentation/CRDs/Cluster/ceph-cluster-crd.md:240-242 (requireMsgr2, default false)`, `rook/deploy/examples/cluster.yaml:133 (requireMsgr2: true)`.
- **Gates:** ms_bind_msgr1 (default true); kernel mount without ms_mode or with ms_mode=legacy (the kernel's built-in default; mount.ceph passes ms_mode=prefer-crc for the new device syntax unless ms_mode or mon_addr is given)
- **Map:** [transport-auth-and-client-library-contract.md › 3.3 msgr1 (legacy protocol)](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#33-msgr1-legacy-protocol) · [transport-auth-and-client-library-contract.md › 7.3 Linux kernel client over msgr1 (legacy)](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#73-linux-kernel-client-over-msgr1-legacy) · [wire-protocol.md › 3.1 Transport layer](../cpp-map/02-client-protocol/wire-protocol.md#31-transport-layer)
- **Code:** `src/msg/async/ProtocolV1.cc:1982-2315 (ProtocolV1::handle_connect_message_2)`, `src/msg/async/ProtocolV1.cc:2492-2578 (ProtocolV1::open)`, `src/include/ceph_fs.h:48 (CEPH_MDSC_PROTOCOL)`, `linux/net/ceph/messenger_v1.c:740-932 (process_connect)`, `linux/net/ceph/messenger_v1.c:1181-1197 (read_partial_message)`
- **Notes:** Client generations: a kernel uses msgr1 only when mounted without ms_mode; mount.ceph has passed ms_mode=prefer-crc for the new device syntax since commit 9663f8aaec9 (first released in 17.2.0); kernels before v5.11 have no msgr2 code at all; Ceph clients 13.x and older speak only msgr1, 14.x and later connect to the first (v2) address of the vector (kclient expectations 7.1.2, msgr row). No CephFS feature bit reveals msgr2 capability. Disabling ms_bind_msgr1 removes the v1 addresses from the MDSMap and a kernel without an ms_mode option then fails the MDSMap decode and shuts the mount down. The C++ messenger never sends the CLOSE tag; the kernel would move the connection to CLOSED for good on it. Only msgr1 negotiates the protocol number; msgr2 has no protocol-number check.

<a id="f-proto-013"></a>

#### F-PROTO-013 · msgr1 message signatures

*Per-message cephx signatures on msgr1 connections, which a kernel mounted without ms_mode verifies by default.*

With cephx_sign_messages (default true) msgr1 signs every message and checks signatures when MSG_AUTH was negotiated; the 64-bit signature goes into the 21-byte footer. Without CEPHX_V2 on the peer it is the first 8 bytes of AES-encrypt({1, magic, 16, header_crc, front_crc, middle_crc, data_crc}); with CEPHX_V2 it is the XOR of the four le64 words of AES-encrypt (AES keys) or HMAC-SHA256 (AES256KRB5 keys) over {header_crc, front_crc, front_len, middle_crc, middle_len, data_crc, data_len, low 32 bits of seq}. On msgr1 connections (a kernel mounted without ms_mode) the kernel signs and verifies every message unless mounted with nocephx_sign_messages; over msgr2, which mount.ceph selects for the new device syntax, it signs nothing (signing exists only in linux/net/ceph/messenger_v1.c). msgr2 has no per-message signature: AUTH_SIGNATURE (HMAC-SHA256 with the session key over all bytes received during the pre-auth phase) binds the handshake, and secure mode authenticates frames by GCM.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient
- **Group:** Authentication and connection identity
- **Legacy** (msgr1): Per-message signing and checking exist only in the msgr1 implementations (C++ ProtocolV1, kernel messenger_v1), so only kernels mounted without ms_mode and Ceph clients 13.x and older use them. A search for sign_message and check_message_signature finds no call in src/msg/async/ProtocolV2.cc or linux/net/ceph/messenger_v2.c. Over msgr2 the AUTH_SIGNATURE frame (F-PROTO-002) binds the handshake and secure mode authenticates frames with AES-GCM. Evidence: `src/msg/async/ProtocolV1.cc:975 (ProtocolV1::handle_message_footer, check_message_signature)`, `src/msg/async/ProtocolV1.cc:1147-1156 (ProtocolV1::write_message, sign_message)`, `linux/net/ceph/messenger_v1.c:180-181 (sign_message)`, `linux/net/ceph/messenger_v1.c:1298-1299 (check_message_signature)`, `src/msg/async/ProtocolV2.cc:2347-2416 (ProtocolV2::finish_auth)`.
- **Gates:** cephx_sign_messages (default true); cephx_service_require_signatures (default false); kernel mount option nocephx_sign_messages
- **Map:** [transport-auth-and-client-library-contract.md › 3.6 cephx on the service side](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#36-cephx-on-the-service-side) · [transport-auth-and-client-library-contract.md › 3.4 msgr2.0 and msgr2.1](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md#34-msgr20-and-msgr21)
- **Code:** `src/auth/cephx/CephxSessionHandler.cc:40-219 (CephxSessionHandler::_calc_signature, check_message_signature)`, `linux/net/ceph/auth_x.c:1042-1121 (calc_signature)`, `src/msg/async/ProtocolV2.cc:2347-2416 (ProtocolV2::finish_auth)`

<a id="f-proto-022"></a>

#### F-PROTO-022 · Feature inference for clients that send no feature bits

*Clients whose REQUEST_OPEN carries an empty bitmap (session messages below v3) get a synthesized set of bits 0..N.*

Server::infer_supported_features applies the first matching rule: metadata ceph_version starting with "ceph version 12." gives bits 0-7 (LUMINOUS); another ceph_version with connection feature FS_CHANGE_ATTR (bit 59) gives 0-6 (KRAKEN); metadata kernel_version with NEW_OSDOP_ENCODING (bit 56) gives 0-7; otherwise connection FS_FILE_LAYOUT_V2 (bit 58) gives 0-5 (JEWEL); none gives an empty set. It runs at open and at reconnect when the stored set is empty, and the result is journaled with the session. Such sessions never have bit 8, 9 or 13, so they receive v1 session messages, legacy reply encodings and the raw created ino, and any required bit above the inferred range rejects them.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Group:** Feature negotiation and client generations
- **Legacy** (old-clients): Inference runs only when REQUEST_OPEN, or the stored session at reconnect, carries an empty CephFS bitmap. Such sessions come from kernels up to v4.18 and Ceph clients up to 13.2.1, which send session message v2. The 7.3 kernel sends a v7 REQUEST_OPEN with bits 0-9, 11-18, 20-22 and libcephfs at this commit sends bits 0-24. Evidence: `src/mds/Server.cc:1770-1797 (Server::infer_supported_features)`, `src/mds/Server.cc:747-748 (Server::handle_client_session)`, `src/mds/Server.cc:1655-1656 (Server::handle_client_reconnect)`, `linux/fs/ceph/mds_client.c:1704 (create_session_full_msg, version 7)`, `linux/fs/ceph/mds_client.c:1726 (create_session_full_msg, encode_supported_features)`, `src/client/Client.cc:2656-2660 (Client::_open_mds_session)`, `linux-v4.18/fs/ceph/mds_client.c:953 (create_session_open_msg, version 2)`.
- **Map:** [wire-protocol.md › 3.4 Session open and feature exchange](../cpp-map/02-client-protocol/wire-protocol.md#34-session-open-and-feature-exchange) · [kclient-expectations.md › 7.1.3 Clients that send no feature bits](../cpp-map/02-client-protocol/kclient-expectations.md#713-clients-that-send-no-feature-bits)
- **Code:** `src/mds/Server.cc:1770-1797 (Server::infer_supported_features)`, `src/mds/Server.cc:1655-1656 (Server::handle_client_reconnect)`
- **Notes:** Matching clients: kernels v4.13-v4.18 (luminous rule), v4.5-v4.12 (jewel rule), none before v4.5 (empty); luminous 12.x userspace; kraken 11.x and mimic 13.2.0-13.2.1 (kraken rule, they still send session v2); jewel 10.2.x (jewel rule) (kclient expectations 7.1.3). Requiring bit 8 (candidate floor F1) leaves this path unused.

<a id="f-proto-066"></a>

#### F-PROTO-066 · Legacy 88-byte request head (message version below 4)

*Fallback request head without a version field, used by old clients and by any client talking to an MDS without FS_BTIME.*

For MClientRequest message versions below 4 the head is the raw 88-byte ceph_mds_request_head_legacy (no version field, 48-byte args). The MDS decoder synthesizes version 0, widens the 8-bit num_retry/num_fwd into the 32-bit fields, sets owner_uid/gid = caller_uid/gid and strips CEPH_SETATTR_BTIME from a setattr mask. Current clients fall back to it only when the MDS connection lacks FS_BTIME (bit 59).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Group:** Request encoding, routing and forwarding
- **Legacy** (old-clients): Message versions below 4 come from kernels up to v5.10 (message v2) and Ceph 10.2.x and older (message v3). Later kernels and libcephfs use the legacy head only towards an MDS connection without FS_BTIME (bit 59), which the baseline MDS advertises. The 7.3 kernel and libcephfs at this commit send message v6 with head v3. Evidence: `src/messages/MClientRequest.h:271-296 (MClientRequest::decode_payload)`, `src/messages/MClientRequest.h:329-336 (MClientRequest::encode_payload)`, `linux/fs/ceph/mds_client.c:3202 (create_request_message, legacy)`, `linux/fs/ceph/mds_client.c:3351-3353 (create_request_message, message v3)`, `src/include/ceph_features.h:238 (FS_BTIME in CEPH_FEATURES_ALL)`, `linux-v5.10/fs/ceph/mds_client.c:2540 (create_request_message, message v2)`.
- **Gates:** connection feature FS_BTIME (59) on the MDS connection
- **Map:** [wire-protocol.md › 3.7 MClientRequest encoding](../cpp-map/02-client-protocol/wire-protocol.md#37-mclientrequest-encoding) · [kclient-expectations.md › 7.1.2 Protocol fallbacks and the clients that exercise them](../cpp-map/02-client-protocol/kclient-expectations.md#712-protocol-fallbacks-and-the-clients-that-exercise-them)
- **Code:** `src/messages/MClientRequest.h:271-296 (MClientRequest::decode_payload)`, `src/include/ceph_fs.h:529-599 (ceph_mds_request_head_legacy)`
- **Notes:** Users: every kernel up to v5.10 (message v2), Ceph 10.2.x and older (message v3); later clients only towards an MDS without FS_BTIME. Candidate floor F4 (bit 16 or 17) leaves it unused (kclient expectations 7.1.7).

<a id="f-proto-082"></a>

#### F-PROTO-082 · Legacy (connection-feature-conditional) reply encodings

*Fallback InodeStat, DirStat and LeaseStat encodings without ENCODE_START headers, for clients that do not advertise bit 9.*

Without REPLY_ENCODING the InodeStat has no header: the same fields up to symlink, then dir_layout only with DIRLAYOUTHASH, xattrs always, inline_version + inline_data with MDS_INLINE_DATA, quota with MDS_QUOTA, pool_ns with FS_FILE_LAYOUT_V2, btime + change_attr with FS_BTIME, and nothing from struct v2 on. DirStat is the same fields without header; LeaseStat is u16 mask, u32 duration_ms, u32 seq (10 bytes, no alternate_name). The kernel legacy decoder takes field presence from the MDS's advertised connection features while the MDS uses the negotiated intersection; the two agree only because each kernel advertises every feature whose field it decodes.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Group:** Reply encoding
- **Legacy** (old-clients): The header-less InodeStat, DirStat and LeaseStat go only to sessions without bit 9: kernels up to v5.0, Ceph clients up to 13.2.x and sessions whose bits were inferred. The 7.3 kernel and libcephfs at this commit advertise bit 9 and receive InodeStat v9, DirStat v1 and LeaseStat v2. Evidence: `src/mds/CInode.cc:4315 (CInode::encode_inodestat, REPLY_ENCODING test)`, `src/mds/CInode.cc:4373-4427 (CInode::encode_inodestat, legacy branch)`, `src/mds/CDir.cc:3066-3070 (CDir::encode_dirstat)`, `src/mds/Locker.cc:4620-4625 (Locker::encode_lease)`, `linux/fs/ceph/mds_client.h:48 (CEPHFS_FEATURE_REPLY_ENCODING advertised)`, `src/client/Client.cc:448-452 (myfeatures)`.
- **Map:** [wire-protocol.md › 3.9.2 InodeStat (CInode::encode_inodestat, CInode.cc:3959...](../cpp-map/02-client-protocol/wire-protocol.md#392-inodestat-cinodeencode_inodestat-cinodecc3959-4429-decoder-mclientreplyh118-307) · [kclient-expectations.md › 7.1.2 Protocol fallbacks and the clients that exercise them](../cpp-map/02-client-protocol/kclient-expectations.md#712-protocol-fallbacks-and-the-clients-that-exercise-them)
- **Code:** `src/mds/CInode.cc:4373-4427 (CInode::encode_inodestat)`, `src/mds/CDir.cc:3058-3071 (CDir::encode_dirstat)`, `src/mds/Locker.cc:4609-4626 (Locker::encode_lease)`, `linux/fs/ceph/mds_client.c:280-313 (parse_reply_info_in)`
- **Notes:** Users: kernels up to v5.0, Ceph up to 13.2.x, and every session with inferred features. Candidate floor F2 (bit 12 or 9) leaves it unused (kclient expectations 7.1.7).

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-proto-014"></a>F-PROTO-014 | [F-ADM-002](11-security-admin-and-observability.md#f-adm-002) · Peer authorizer verification and caps loading at connection accept |
| <a id="f-proto-035"></a>F-PROTO-035 | [F-ADM-021](11-security-admin-and-observability.md#f-adm-021) · cap_auths export in CEPH_SESSION_OPEN and client-side checks |
| <a id="f-proto-036"></a>F-PROTO-036 | [F-ADM-022](11-security-admin-and-observability.md#f-adm-022) · MDS_CLIENTS_BROKEN_ROOTSQUASH health check |
| <a id="f-proto-058"></a>F-PROTO-058 | [F-CLU-080](01-cluster-membership-and-ha.md#f-clu-080) · up:reconnect: the client reconnect window and its timeout |
| <a id="f-proto-059"></a>F-PROTO-059 | [F-CLU-084](01-cluster-membership-and-ha.md#f-clu-084) · Admission of replayed and retried requests during reconnect |
| <a id="f-proto-060"></a>F-PROTO-060 | [F-NS-084](04-namespace-and-posix-operations.md#f-ns-084) · Lock reinstatement after an MDS restart |
| <a id="f-proto-064"></a>F-PROTO-064 | [F-CLU-096](01-cluster-membership-and-ha.md#f-clu-096) · Reclaim gating of the clientreplay to active transition |

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
