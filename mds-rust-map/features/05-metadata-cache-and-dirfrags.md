# Metadata cache and directory fragments: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [cinode-and-inode-types.md](../cpp-map/04-cache/cinode-and-inode-types.md) · [cdir-cdentry-and-cache-objects.md](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md) · [mdcache-subtrees-traversal-replication.md](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md) · [mdcache-lifecycle-trim-and-memory.md](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md) · [dirfrags-and-fragmentation.md](../cpp-map/04-cache/dirfrags-and-fragmentation.md)

## 1. Scope

This category covers the in-memory metadata cache of one MDS rank as the C++ MDS implements it: the inode, dirfrag and dentry objects and their fields (inode_t, fnode, linkage, snapshot ranges, versions and projection), how dirty objects are tracked and when they are written back (the dirfrag fetch and commit operations and damage detection on load are catalogued in 06-journal-and-storage and 09-background-services), completeness and negative lookups, reference counts, auth pins, freezing and waiters, authority and the subtree map, path traversal and lookup by inode number, replication of cache objects between ranks, residency (LRUs, trimming, the memory limit), the base hierarchy created at mkfs and opened at boot, and directory fragmentation (fragment ids, name hashing, readdir ordering, split/merge triggers, protocol and recovery). Sources are the five pages under cpp-map/04-cache.

Excluded and catalogued elsewhere: lock types, caps, leases, lock caches, cap recall and the auth-pin discipline of acquire_locks (03-caps-leases-and-locking); request handlers, readdir/lookup handlers, xattrs and request batching (04-namespace-and-posix-operations); InodeStat and other reply encodings (02-client-sessions-and-protocol); RADOS byte formats of dirfrags, base inodes and backtraces, ESubtreeMap/EFragment event encodings and the OpenFileTable (06-journal-and-storage); subtree export/import, export and ephemeral pins, popularity and hot-directory replication (07-multi-mds); snapshot and rstat semantics, quotas, subvolumes (08-snapshots-quotas-subvolumes); strays, purge, scrub and the DamageTable (09-background-services); quiesce and quarantine (10-qos-metrics-quiesce-quarantine); admin commands, perf counters and health warnings (11-security-admin-and-observability); rank stopping and process threading (01-cluster-membership-and-ha). Where an entry depends on one of these it describes only the cache part and links to the owning map page.

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

87 features: 87 used by current peers (12 of them with legacy parts) and 0 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**Inode record and attribute semantics**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-001](#f-cache-001) | Inode record field set (inode_t) | required | on-disk, client-wire | kclient, libcephfs, ceph-fuse, ganesha, tools |
| [F-CACHE-003](#f-cache-003) | Refusal of client operations on private and system inodes | supporting | client-wire | kclient, libcephfs, ganesha, ceph-fuse |
| [F-CACHE-004](#f-cache-004) | Timestamps, time_warp_seq and change_attr *(legacy parts)* | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-006](#f-cache-006) | Truncation state fields | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, osd |
| [F-CACHE-007](#f-cache-007) | File layout field and layout inheritance *(legacy parts)* | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-CACHE-008](#f-cache-008) | Inline data *(legacy parts)* | extension | client-wire, on-disk | kclient, libcephfs |
| [F-CACHE-009](#f-cache-009) | fscrypt metadata fields | extension | client-wire, on-disk | kclient |
| [F-CACHE-010](#f-cache-010) | Inode version and client-visible version | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-011](#f-cache-011) | Snapshot-range identity of inodes and dentries | extension | on-disk | kclient, libcephfs |

**Projection and dirty state**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-012](#f-cache-012) | Inode projection (read-copy-update) *(legacy parts)* | internal | internal | internal |
| [F-CACHE-013](#f-cache-013) | Dirfrag version sequence and fnode projection | internal | on-disk | internal |
| [F-CACHE-014](#f-cache-014) | Projected dentry linkage (primary, remote, null) | supporting | on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-015](#f-cache-015) | Dirty tracking on journal segments | internal | internal | internal |

**Dirfrag load and write-back**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-016](#f-cache-016) | Write-back only at journal segment expiry | supporting | on-disk, config, admin | internal, admin |
| [F-CACHE-018](#f-cache-018) | Commit error handling and read-only mode | operational | metrics, config | mon, admin |
| [F-CACHE-020](#f-cache-020) | Underwater item reconciliation after replay | internal | internal | internal |

**Directory contents and negative lookups**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-023](#f-cache-023) | Directory completeness and authoritative negative lookups | supporting | internal | kclient, libcephfs, ganesha |
| [F-CACHE-024](#f-cache-024) | Bloom filter of trimmed names | internal | internal | internal |
| [F-CACHE-025](#f-cache-025) | Null dentries | internal | internal | internal |

**Cache object model: references, auth pins, freezing, waiters**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-026](#f-cache-026) | Reference counts and upward pinning | internal | internal | internal |
| [F-CACHE-027](#f-cache-027) | Inode maps and object registry | internal | internal | internal |
| [F-CACHE-028](#f-cache-028) | Auth pins with one-level nested accounting | internal | internal | internal |
| [F-CACHE-029](#f-cache-029) | Dirfrag freeze (freeze_dir) | internal | internal | internal |
| [F-CACHE-030](#f-cache-030) | Subtree freeze (freeze_tree) | internal | internal | mds-ranks, internal |
| [F-CACHE-032](#f-cache-032) | Single-inode freeze, ambiguous inode auth and frozen-inode tracking | internal | internal | mds-ranks, internal |
| [F-CACHE-033](#f-cache-033) | Waiter queues on cache objects | internal | internal | internal |
| [F-CACHE-034](#f-cache-034) | Waiter completion semantics | internal | internal | internal |

**Authority and the subtree map**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-035](#f-cache-035) | Authority of cache objects and the auth bit | internal | internal | mds-ranks, internal |
| [F-CACHE-036](#f-cache-036) | Subtree map | internal | internal | mds-ranks, internal |
| [F-CACHE-037](#f-cache-037) | Subtree create, merge and bounded-adjust primitives | internal | internal | mds-ranks, internal |
| [F-CACHE-038](#f-cache-038) | Ambiguous authority and WAIT_SINGLEAUTH | internal | internal | mds-ranks, internal |
| [F-CACHE-039](#f-cache-039) | Subtree map maintenance on directory rename | internal | internal | mds-ranks, internal |
| [F-CACHE-040](#f-cache-040) | Aux subtrees | internal | internal | mds-ranks, admin |

**Path traversal and lookup by inode number**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-041](#f-cache-041) | Path traversal (path_traverse) | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-044](#f-cache-044) | Forward-or-discover for non-auth objects | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-CACHE-045](#f-cache-045) | Hard-link target resolution | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-047](#f-cache-047) | Lookup by inode number (open_ino) | supporting | client-wire, on-disk, mds-wire | kclient, libcephfs, ceph-fuse, ganesha, osd, mds-ranks |
| [F-CACHE-048](#f-cache-048) | open_ino peer protocol | internal | mds-wire | mds-ranks |
| [F-CACHE-049](#f-cache-049) | Batched dentry-key fetches for open_ino | internal | internal | internal, osd |
| [F-CACHE-050](#f-cache-050) | ESTALE recovery via find_ino_peers | supporting | client-wire, mds-wire | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-CACHE-051](#f-cache-051) | Foreign mdsdir and stray base resolution | internal | mds-wire | mds-ranks |

**Cache replication between ranks**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-052](#f-cache-052) | Replica map and nonces | internal | mds-wire | mds-ranks |
| [F-CACHE-053](#f-cache-053) | Discover protocol (pull replication) | internal | mds-wire | mds-ranks |
| [F-CACHE-054](#f-cache-054) | Replica encodings | internal | mds-wire | mds-ranks |
| [F-CACHE-055](#f-cache-055) | Replica subtrees at delegation boundaries | internal | internal | mds-ranks |
| [F-CACHE-056](#f-cache-056) | Push link/unlink to replicas *(legacy parts)* | internal | mds-wire | mds-ranks |
| [F-CACHE-057](#f-cache-057) | Replica refresh through lock-state payloads | internal | mds-wire | mds-ranks |
| [F-CACHE-058](#f-cache-058) | Replica expiry (MCacheExpire) | internal | mds-wire | mds-ranks |

**Residency, trimming and memory limit**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-059](#f-cache-059) | Dentry LRUs (lru and bottom_lru) | internal | config | internal |
| [F-CACHE-060](#f-cache-060) | Pin reasons that block eviction | operational | internal | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-061](#f-cache-061) | Memory-based soft cache limit | operational | config, metrics | admin, mgr, mon, cephadm |
| [F-CACHE-062](#f-cache-062) | Trim throttle | operational | config, metrics | admin |
| [F-CACHE-063](#f-cache-063) | Trim pass and per-object removal | internal | internal | internal |
| [F-CACHE-064](#f-cache-064) | Upkeep thread | internal | config | internal |
| [F-CACHE-065](#f-cache-065) | Heap release to the OS | operational | config | internal |
| [F-CACHE-066](#f-cache-066) | Empty-subtree trimming and empty-import return | internal | internal | mds-ranks |
| [F-CACHE-067](#f-cache-067) | Expiry of a stopping peer's mdsdir replicas | internal | internal | mds-ranks |
| [F-CACHE-068](#f-cache-068) | Recovery-time non-auth trimming | internal | internal | mds-ranks |
| [F-CACHE-069](#f-cache-069) | Standby-replay cache maintenance | operational | internal | internal |

**Base hierarchy**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-070](#f-cache-070) | mkfs base hierarchy creation | operational | on-disk | mon, tools |
| [F-CACHE-071](#f-cache-071) | Root and system inode initial attributes and default layouts | required | client-wire, on-disk, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-072](#f-cache-072) | Global snaprealm dummy inode | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-073](#f-cache-073) | Boot-time opening of the base hierarchy and the open gate *(legacy parts)* | operational | on-disk, metrics | mon, internal |

**Fragment identity, name hashing and client exposure**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-074](#f-cache-074) | Fragment identifier and fragment tree (frag_t, fragtree_t) *(legacy parts)* | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-075](#f-cache-075) | Dentry name hash per directory (dl_dir_hash) *(legacy parts)* | supporting | client-wire, on-disk, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-076](#f-cache-076) | Fragtree and dir_layout in InodeStat *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-077](#f-cache-077) | Hash-ordered dentry storage and readdir order *(legacy parts)* | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-078](#f-cache-078) | Readdir fragment mapping and resume across refragmentation | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-079](#f-cache-079) | DirStat delegation hints *(legacy parts)* | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CACHE-080](#f-cache-080) | Per-dirfrag size limit (ENOSPC) | operational | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha, admin |
| [F-CACHE-081](#f-cache-081) | Per-directory entry limit | operational | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha, admin |

**Directory split and merge**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CACHE-082](#f-cache-082) | Size-triggered split and split checks | internal | config | internal |
| [F-CACHE-083](#f-cache-083) | Fast split | internal | config | internal |
| [F-CACHE-084](#f-cache-084) | Temperature-triggered split | internal | config | internal |
| [F-CACHE-085](#f-cache-085) | Size-triggered merge | internal | config | internal |
| [F-CACHE-086](#f-cache-086) | Fragmentation preconditions and exclusions *(legacy parts)* | internal | internal | internal, admin |
| [F-CACHE-087](#f-cache-087) | Fragment operation (FRAGMENTDIR) | internal | on-disk | internal, osd |
| [F-CACHE-088](#f-cache-088) | Dentry repartitioning and stat-delta preservation | internal | internal | internal |
| [F-CACHE-089](#f-cache-089) | New dirfrag object write and old object purge | internal | on-disk | osd, tools |
| [F-CACHE-090](#f-cache-090) | Replica notification of refragmentation | internal | mds-wire | mds-ranks |
| [F-CACHE-091](#f-cache-091) | Fragtree propagation through dirfragtreelock | internal | mds-wire, on-disk | mds-ranks |
| [F-CACHE-092](#f-cache-092) | Subtree map maintenance on split and merge | internal | internal | mds-ranks |
| [F-CACHE-093](#f-cache-093) | Abort, requeue and cancellation of fragment operations | internal | internal | internal, mds-ranks |
| [F-CACHE-094](#f-cache-094) | Recovery of in-flight fragment operations | internal | on-disk | internal, osd |
| [F-CACHE-095](#f-cache-095) | Forced refragmentation during replay, resolve and rejoin | internal | internal | mds-ranks, internal |
| [F-CACHE-096](#f-cache-096) | Migration and fragmentation mutual exclusion | internal | internal | mds-ranks |
| [F-CACHE-097](#f-cache-097) | Effects of a fragment freeze on clients | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

## 3. Features

### Inode record and attribute semantics

The per-inode record (inode_t) held by each CInode, the fixed inode numbers, and the attribute fields whose semantics clients observe directly. Lock ownership of each field group is in the locking category.

<a id="f-cache-001"></a>

#### F-CACHE-001 · Inode record field set (inode_t)

*The persistent per-inode record from which every client-visible attribute of a file or directory is derived.*

inode_t holds ino, rdev, ctime/btime/mtime/atime, mode/uid/gid, nlink, dir_layout, file layout, old_pools, size, max_size_ever, truncate_seq/size/from/pending, time_warp_seq, inline_data, change_attr, client_ranges, dirstat, rstat, accounted_rstat, quota, export_pin, export_ephemeral_random_pin, flags (F_EPHEMERAL_DISTRIBUTED_PIN = 1, F_QUIESCE_BLOCK = 2), version, xattr_version, backtrace_version, last_scrub_version/stamp, stray_prior_path, fscrypt_auth/fscrypt_file/fscrypt_last_block and optmetadata (kind CHARMAP = 1, QUARANTINE = 2; unknown kinds are re-encoded verbatim). Each field group is changed by the auth MDS under one inode lock (authlock: mode/uid/gid/fscrypt_auth; linklock: nlink; filelock: size, times, layout, truncation, client_ranges, inline data; xattrlock; policylock: dir layout, quota, pins, flags, optmetadata; nestlock: rstat). The C++ encoding is ENCODE_START(20, 6) and is what the dirfrag omap entries, base .inode objects and journal fullbits store; clients never see it, only the subsets carried in InodeStat and MClientCaps. file_data_version and inode_t::oldest_snap are declared but unused.

- **Class:** POSIX `required` · exposure `on-disk`, `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, tools
- **Map:** [cinode-and-inode-types.md › 3.3 inode_t field reference](../cpp-map/04-cache/cinode-and-inode-types.md#33-inode_t-field-reference) · [cinode-and-inode-types.md › 3.4 inode_t binary encoding (struct_v 20, compat 6)](../cpp-map/04-cache/cinode-and-inode-types.md#34-inode_t-binary-encoding-struct_v-20-compat-6) · [cinode-and-inode-types.md › 3.5 Xattrs, optmetadata and flags](../cpp-map/04-cache/cinode-and-inode-types.md#35-xattrs-optmetadata-and-flags)
- **Code:** `src/include/cephfs/types.h:771 (inode_t)`, `src/include/cephfs/types.h:1053 (inode_t::encode)`, `src/include/cephfs/types.h:1121 (inode_t::decode)`, `src/include/cephfs/types.h:532 (optmetadata_server_t)`
- **Notes:** The decoder declares struct_v 19 although the encoder writes 20 (works only because compat is 6); same pattern for InodeStoreBase (6 vs 5) and fnode_t (4 vs 3). optmetadata singleton equality uses memcmp over the in-memory variant (tracker 73382), so scrub's inode_t::compare can report spurious divergence; get_kind() lets kind == 3 index past the table (code reading). cephfs-data-scan, cephfs-meta-injector and ceph-dencoder decode this C++ encoding (tools contract only for the C++ format).

<a id="f-cache-003"></a>

#### F-CACHE-003 · Refusal of client operations on private and system inodes

*Client requests that name private or system inodes get fixed error codes.*

LOOKUPINO (and LOOKUPHASH, LOOKUPPARENT, LOOKUPNAME, which share the handler) on an MDS_IS_PRIVATE_INO number (~mdsN, strays, log inodes) returns -ESTALE, as does a lookup of an inode being purged. setattr on a system inode other than a base inode returns -EPERM; namespace changes inside system directories other than the root return -EROFS (unlink in lost+found excepted); mksnap in them returns -EPERM. The checks live in the Server handlers (namespace category); this layer supplies is_system(), is_base() and MDS_IS_PRIVATE_INO.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ganesha, ceph-fuse
- **Map:** [cinode-and-inode-types.md › 3.2 Identity, snapshot ranges and special inode numbers](../cpp-map/04-cache/cinode-and-inode-types.md#32-identity-snapshot-ranges-and-special-inode-numbers) · [mdcache-subtrees-traversal-replication.md › 3.9.3 Backtraces](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#393-backtraces)
- **Code:** `src/mds/Server.cc:4450 (Server::handle_client_lookup_ino)`, `src/mds/Server.cc:5589 (Server::handle_client_setattr)`, `src/mds/Server.cc:4026 (Server::rdlock_path_xlock_dentry)`, `src/mds/Server.cc:11616 (Server::handle_client_mksnap)`

<a id="f-cache-004"></a>

#### F-CACHE-004 · Timestamps, time_warp_seq and change_attr

*Rules by which ctime, mtime, atime, btime, time_warp_seq and the change attribute evolve and reach clients.*

ctime is set by most mutations (setattr, xattrs, vxattrs, link/unlink, rename, quarantine); a cap flush applies it only if newer; directory mtime and ctime follow dirstat.mtime during stat propagation. btime is set at creation and changed by setattr CEPH_SETATTR_BTIME or a cap flush holding Ax with FS_BTIME. mtime/atime change by setattr and by cap flushes (Fw: mtime only if newer; Fx: any value). time_warp_seq starts at 0 and is incremented on an explicit setattr of ATIME, MTIME or BTIME; a cap flush with Fx applies the client value when newer (ceph_seq_cmp), so clients discard older cached times. change_attr starts at 0, is incremented by setattr, setlayout, xattr and vxattr changes, link/unlink, rename and quarantine, and for directories when dirstat.change_attr advances; a cap flush applies a larger client value (FS_CHANGE_ATTR). btime and change_attr are sent in InodeStat (legacy encoding: only with FS_BTIME) and MClientCaps.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): (1) The legacy InodeStat encoding for sessions without CephFS bit 9 (reply_encoding), in which btime and change_attr are sent only when the connection has FS_BTIME. (2) The branch of the cap-flush gates for connections without bit 59: btime is applied only with FS_BTIME and change_attr only with FS_CHANGE_ATTR, so such clients' values are ignored. The legacy InodeStat goes only to sessions without CephFS bit 9 (kernels up to v5.0, Ceph 13.2.x and older), and only kernels before v5.3 lack connection bit 59. The 7.3 kernel advertises bit 9 and bit 59 (as MSG_ADDR2), and current libcephfs advertises every bit, so both receive the versioned InodeStat, which always carries btime and change_attr, and have cap-flush btime and change_attr applied. Evidence: `src/mds/CInode.cc:4315 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4373-4427 (CInode::encode_inodestat)`, `src/mds/Locker.cc:3985-3999 (Locker::_update_cap_fields)`, `src/mds/Locker.cc:4067 (Locker::_update_cap_fields)`, `linux/fs/ceph/mds_client.h:45-60 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/include/linux/ceph/ceph_features.h:166-168 (FS_BTIME, FS_CHANGE_ATTR, MSG_ADDR2)`, `linux/include/linux/ceph/ceph_features.h:219 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/include/ceph_features.h:180 (FS_BTIME, kernel 5.3)`.
- **Map:** [cinode-and-inode-types.md › 3.3 inode_t field reference](../cpp-map/04-cache/cinode-and-inode-types.md#33-inode_t-field-reference) · [cinode-and-inode-types.md › 7. Externally visible contract](../cpp-map/04-cache/cinode-and-inode-types.md#7-externally-visible-contract) · [cinode-and-inode-types.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/cinode-and-inode-types.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Locker.cc:3990 (Locker::_update_cap_fields)`, `src/mds/Server.cc:5729 (Server::handle_client_setattr)`, `src/mds/MDCache.cc:2413 (MDCache::predirty_journal_parents)`, `linux/fs/ceph/inode.c:1110 (ceph_fill_inode)`
- **Notes:** The kernel exposes change_attr as i_version with STATX_ATTR_CHANGE_MONOTONIC (NFS change attribute), so it must never go backwards. btime and change_attr are not in any replica lock payload, so a replica that answers stat or issues caps can report older values until it re-receives the whole inode (code reading). See also F-NS-063 (timestamps from client clock).

<a id="f-cache-006"></a>

#### F-CACHE-006 · Truncation state fields

*The truncate sequence, pending-truncation counters and size bounds that order file truncation between MDS, clients and OSDs.*

truncate_seq starts at 1 ("0 is kept for no-truncation"), truncate_size at -1 (not truncated), truncate_from and truncate_pending at 0. inode_t::truncate(old, new) sets truncate_from = old, truncate_size = new, increments truncate_seq and truncate_pending and raises max_size_ever; the MDS then truncates the RADOS objects and truncate_inode_finish decrements truncate_pending and clears truncate_from (and fscrypt_last_block). Clients receive truncate_seq/truncate_size in InodeStat and MClientCaps (TRUNC op) and pass them on OSD data ops. A shrinking setattr waits (WAIT_TRUNC) while an earlier truncation is pending; truncate() asserts new <= old. max_size_ever bounds the purge range of a deleted file.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, osd
- **Map:** [cinode-and-inode-types.md › 3.3 inode_t field reference](../cpp-map/04-cache/cinode-and-inode-types.md#33-inode_t-field-reference) · [cinode-and-inode-types.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/cinode-and-inode-types.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/include/cephfs/types.h:796 (inode_t::truncate)`, `src/mds/MDCache.cc:6781 (MDCache::truncate_inode_finish)`, `src/mds/MDCache.cc:6595 (MDCache::truncate_inode)`, `src/mds/Server.cc:5651 (Server::handle_client_setattr)`
- **Notes:** Execution and journaling of object truncation (and its restart after failover) belong to the storage category; cap TRUNC messages to the caps category. See also F-NS-062, F-STORE-065, F-CAPS-016.

<a id="f-cache-007"></a>

#### F-CACHE-007 · File layout field and layout inheritance

*Where a new file's striping layout and data pool come from, and when a layout may change.*

Files carry file_layout_t {stripe_unit, stripe_count, object_size, pool_id, pool_ns}. A new file inherits the layout of the nearest ancestor directory that has one, found by try_rdlock_snap_layout (rdlocks policylock up the chain), else default_file_layout = file_layout_t::get_default() on the MDSMap's first data pool (also the root's layout). A file layout may change only while size is 0 and truncate_seq <= 1 (SETLAYOUT or ceph.file.layout, under filelock); directory layouts change via ceph.dir.layout under policylock. A pool change adds the old pool to old_pools, which only grows. Encoding: legacy 28-byte ceph_file_layout when the peer lacks FS_FILE_LAYOUT_V2, else versioned v2 with pool_ns; InodeStat carries the legacy layout plus pool_ns, and Fr/Fw are withheld from clients without FS_FILE_LAYOUT_V2 when the layout has a namespace.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Legacy parts** (old-clients, old-daemons): (1) file_layout_t::encode writing the 28-byte ceph_file_layout when the feature mask lacks FS_FILE_LAYOUT_V2 (bit 58); for on-disk, journal and MDS-wire encodings the mask is the MDSMap up-features, so this needs an up MDS daemon older than 10.2 (old-cpp-daemons). (2) Withholding Fr/Fw from clients whose connection lacks FS_FILE_LAYOUT_V2 when the layout has a pool namespace (Capability STATE_NOPOOLNS) (old-clients). Clients without bit 58 are kernels before v4.5 and Ceph clients before 10.2, while the 7.3 kernel advertises bit 58 (as CRUSH_TUNABLES5) and current Ceph clients and daemons carry FS_FILE_LAYOUT_V2 in CEPH_FEATURES_ALL. On-disk, journal and MDS-to-MDS layouts are encoded with the MDSMap up-features, so the 28-byte form is written there only while an up MDS lacks bit 58. The 28-byte ceph_file_layout plus a separate pool_ns in InodeStat is current (the 7.3 kernel decodes it), and the 'legacy SETLAYOUT' op in the notes is current because the 7.3 kernel sends CEPH_MDS_OP_SETLAYOUT from its layout ioctl. Evidence: `src/common/fs_types.cc:89-98 (file_layout_t::encode)`, `src/mds/CDir.cc:2742 (CDir::_parse_dentry)`, `src/mds/CInode.cc:1284 (CInode::store)`, `src/mds/CInode.cc:1497 (CInode::_store_backtrace)`, `src/mds/Capability.cc:173-178 (Capability::Capability)`, `src/mds/CInode.cc:3826-3838 (CInode::get_caps_allowed_for_client)`, `src/mds/CInode.cc:4326-4328 (CInode::encode_inodestat)`, `linux/include/linux/ceph/ceph_features.h:163-165 (CRUSH_TUNABLES5, FS_FILE_LAYOUT_V2)`, `linux/include/linux/ceph/ceph_features.h:217 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:236 (CEPH_FEATURES_ALL)`, `linux/fs/ceph/ioctl.c:111 (ceph_ioctl_set_layout)`.
- **Gates:** feature FS_FILE_LAYOUT_V2
- **Map:** [cinode-and-inode-types.md › 3.3 inode_t field reference](../cpp-map/04-cache/cinode-and-inode-types.md#33-inode_t-field-reference) · [cinode-and-inode-types.md › 3.15 Client capability tracking on the inode](../cpp-map/04-cache/cinode-and-inode-types.md#315-client-capability-tracking-on-the-inode)
- **Code:** `src/mds/Locker.cc:157 (Locker::try_rdlock_snap_layout)`, `src/mds/Server.cc:5862 (Server::handle_client_setlayout)`, `src/mds/MDCache.cc:417 (MDCache::gen_default_file_layout)`, `src/common/fs_types.cc:89 (file_layout_t::encode)`
- **Notes:** Layout vxattrs are in the namespace category ([attributes and xattrs](../cpp-map/03-request-path/attributes-and-xattrs.md)). Legacy SETLAYOUT calls add_old_pool before pre_dirty, so the op itself does not dirty the backtrace; if the creation backtrace was already stored, the new pool gets none until a later backtrace update (code reading, not tested). See also F-NS-042 (layout at create), F-NS-072 (layout vxattrs).

<a id="f-cache-008"></a>

#### F-CACHE-008 · Inline data

*Small-file data stored inside the inode and handed to clients with caps.*

inline_data_t {u64 version, data}; version CEPH_INLINE_NONE ((u64)-1) means no inline data. New inodes get version 1 when inline data is enabled and the client has MDS_INLINE_DATA, else NONE. Clients change it through cap flushes holding Fw that carry a newer inline_version. InodeStat and MClientCaps include the blob only when the client's recorded inline version is older (or getattr asks for Fr) and the issued cap holds Fs. Fr/Fw are withheld from clients that cannot handle inline data (cap marked noinline or no MDS_INLINE_DATA feature). Scrub can uninline (background category).

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs
- **Legacy parts** (deprecated-admin, old-clients): Everything that creates or handles non-empty inline data (version 1 for new inodes when the MDSMap flag is set, cap-flush updates of inline data, sending the blob, scrub uninlining) is reachable only through the deprecated inline_data flag (deprecated-admin). The checks for clients without MDS_INLINE_DATA (Capability STATE_NOINLINE withholding Fr/Fw, EPERM on open of an inline file) apply only to clients older than kernel 3.19 (old-clients). New inodes get inline data only while the MDSMap inline_data flag is set, which the monitor accepts only with --yes-i-really-really-mean-it as a deprecated feature (health FS_INLINE_DATA_DEPRECATED) and which no mgr module, cephadm or Rook sets. With the flag off every inode has version CEPH_INLINE_NONE, and that value stays in the inode_t, InodeStat and MClientCaps encodings (current). The 7.3 kernel (reads, uninlines on open for write) and current libcephfs (reads, writes small data inline, uninlines) still handle inline data when an inode carries it, and only clients older than kernel 3.19 lack MDS_INLINE_DATA. Evidence: `src/mon/FSCommands.cc:464-485 (inline_data handler)`, `src/mds/MDSMap.cc:616-621 (FS_INLINE_DATA_DEPRECATED)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mds/Server.cc:4704-4709 (Server::handle_client_open)`, `src/mds/Capability.cc:175-176 (Capability::Capability)`, `src/mds/CInode.cc:3826-3838 (CInode::get_caps_allowed_for_client)`, `doc/cephfs/experimental-features.rst:24-28`, `linux/include/linux/ceph/ceph_features.h:207 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `linux/fs/ceph/file.c:261-262 (ceph_init_file_info)`, `src/client/Client.cc:12916-12940 (Client::_write)`, `src/include/ceph_features.h:144 (MDS_INLINE_DATA, kernel 3.19)`. Also: rook/pkg/daemon/ceph/client/filesystem.go:124 (fs set allow_standby_replay; :134 standby_count_wanted, :194 max_mds are the only fs set calls).
- **Gates:** MDSMap inline data flag; feature MDS_INLINE_DATA
- **Map:** [cinode-and-inode-types.md › 3.3 inode_t field reference](../cpp-map/04-cache/cinode-and-inode-types.md#33-inode_t-field-reference) · [cinode-and-inode-types.md › 3.16 Encoding inode state to clients](../cpp-map/04-cache/cinode-and-inode-types.md#316-encoding-inode-state-to-clients)
- **Code:** `src/mds/Server.cc:3758 (Server::prepare_new_inode)`, `src/mds/Locker.cc:4027 (Locker::_update_cap_fields)`, `src/mds/CInode.cc:3798 (CInode::get_caps_allowed_for_client)`, `src/mds/mdstypes.cc:293 (inline_data_t::encode)`
- **Notes:** See also F-STORE-067 (uninlining).

<a id="f-cache-009"></a>

#### F-CACHE-009 · fscrypt metadata fields

*Opaque per-inode encryption context and per-file data that fscrypt clients store on the MDS.*

fscrypt_auth (opaque context; non-empty means encrypted; authlock), fscrypt_file (opaque per-file data such as the real size; filelock) and fscrypt_last_block (raw truncation blob, never sent to clients). They are set at create from the request, by setattr FSCRYPT_AUTH (requires allow_all MDS auth caps, else -EACCES) or FSCRYPT_FILE, or by cap flushes. A shrinking setattr carries ceph_fscrypt_truncate_size_header {ver, compat, data_len, change_attr, file_offset, block_size} plus the last block: -EAGAIN if change_attr differs from the projected inode, -EINVAL if the payload exceeds the header size plus mds_fscrypt_last_block_max_size (4 KiB). Sent as InodeStat v6 flag and v7 blobs and in MClientCaps.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient
- **Gates:** mds_fscrypt_last_block_max_size (4 KiB)
- **Map:** [cinode-and-inode-types.md › 3.3 inode_t field reference](../cpp-map/04-cache/cinode-and-inode-types.md#33-inode_t-field-reference) · [cinode-and-inode-types.md › 7. Externally visible contract](../cpp-map/04-cache/cinode-and-inode-types.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:5643-5684 (Server::handle_client_setattr)`, `src/mds/fscrypt.h:19 (ceph_fscrypt_last_block_header)`, `linux/fs/ceph/crypto.h:35 (ceph_fscrypt_truncate_size_header)`, `src/mds/Server.cc:3745 (Server::prepare_new_inode)`
- **Notes:** The MDS header struct is unpacked (32 bytes on LP64) while the kernel wire header is packed (26 bytes), so the size bound admits 6 extra bytes. In InodeStat fscrypt_auth follows the filelock projection flag (pfile), not the authlock one (code reading). See also F-NS-064 (setattr and last-block truncate), F-ADM-020.

<a id="f-cache-010"></a>

#### F-CACHE-010 · Inode version and client-visible version

*The per-inode version number and the 2v+projected value clients use to discard stale attributes.*

inode.version is not a private counter: CInode::pre_dirty asks the projected parent dentry, which sets the parent dirfrag's projected_version = max(projected_version, inode's projected version) + 1, so all children of a dirfrag share one sequence and an inode moved by rename stays monotonic; base inodes increment their own. Replies carry 2 x stable version + 1 if a projection is pending. The kernel and libcephfs apply attributes only when their cached version is 0 or the reply comes from the auth (CEPH_CAP_FLAG_AUTH) with version > (cached & ~1). xattr_version and inline_data.version are separate per-blob versions that decide whether a blob is resent.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [cinode-and-inode-types.md › 3.8 Versions](../cpp-map/04-cache/cinode-and-inode-types.md#38-versions) · [cinode-and-inode-types.md › 7. Externally visible contract](../cpp-map/04-cache/cinode-and-inode-types.md#7-externally-visible-contract)
- **Code:** `src/mds/CInode.cc:1177 (CInode::pre_dirty)`, `src/mds/CDir.cc:1439 (CDir::pre_dirty)`, `src/mds/CInode.cc:4043 (CInode::encode_inodestat)`, `linux/fs/ceph/inode.c:1104 (ceph_fill_inode)`, `src/client/Client.cc:1150 (Client::add_update_inode)`
- **Notes:** Because versions come from the parent dirfrag counter, an inode's version jumps by the number of sibling updates in between; clients rely only on monotonicity per inode.

<a id="f-cache-011"></a>

#### F-CACHE-011 · Snapshot-range identity of inodes and dentries

*How cached inodes and dentries carry the snapshot id range they are valid for.*

A CInode covers snapids [first, last]; head inodes have last = CEPH_NOSNAP and live in inode_map, snapped CInodes in snap_inode_map keyed by (ino, last). Directories, inodes with their own snaprealm, inodes with nlink > 1 and already-multiversion inodes keep snapshot-era copies inside the head (old_inodes: map last -> {first, inode_t, xattrs}); plain files are copied-on-write into a separate CInode [first, follows]. Dentries are valid for [first, last]; the dirfrag key includes last, so several generations of a name coexist; CDir::lookup(name, snap) takes the smallest last >= snap and accepts it only if first <= snap; (name, last) is unique per dirfrag. Snapshot semantics (when to COW, realms, snap ids) are in the snapshots category ([snapshots](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md)).

- **Class:** POSIX `extension` · exposure `on-disk` · relied on by kclient, libcephfs
- **Map:** [cinode-and-inode-types.md › 3.2 Identity, snapshot ranges and special inode numbers](../cpp-map/04-cache/cinode-and-inode-types.md#32-identity-snapshot-ranges-and-special-inode-numbers) · [cinode-and-inode-types.md › 3.6 Old inodes (snapshot-era copies)](../cpp-map/04-cache/cinode-and-inode-types.md#36-old-inodes-snapshot-era-copies) · [cdir-cdentry-and-cache-objects.md › 3.13 Snapshot ranges on dentries](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#313-snapshot-ranges-on-dentries)
- **Code:** `src/mds/CInode.h:487 (CInode::is_multiversion)`, `src/mds/MDCache.cc:1573 (MDCache::cow_inode)`, `src/mds/CDir.cc:328 (CDir::lookup)`, `src/mds/old_inode.h:15 (old_inode_t)`
- **Notes:** See also F-SNAP-026.

### Projection and dirty state

Read-copy-update projection of inodes, dirfrag headers and dentry linkage between journaling and apply, and the per-segment dirty lists that decide what must be written back.

<a id="f-cache-012"></a>

#### F-CACHE-012 · Inode projection (read-copy-update)

*In-flight mutations edit a private copy of the inode that becomes current only after the journal entry is safe.*

project_inode copies the newest inode_t (and optionally the xattr map and snaprealm node) onto a per-inode FIFO projected list; a mutation projects an inode at most once. The mutation edits the copy and journals it (the journal fullbit holds the same immutable shared pointer); when the entry is safe MutationImpl::apply pops the front node, swaps it in and marks the inode dirty (and dirty-parent if its backtrace changed). InodeStat returns projected values only to the client that holds the relevant xlock, is the loner, or last wrlocked the versionlock (early reply); other clients see stable values. Journal encoding and dirfrag commit read these immutable snapshots without mds_lock.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Legacy parts** (dead-code): CInode::pop_and_apply_projected_inode, which has no caller. CInode::pop_and_apply_projected_inode is declared and defined, and a grep over src finds no caller. Projected inodes are applied through MutationImpl::apply and pop_and_dirty_projected_inode. Evidence: `src/mds/CInode.h:533 (CInode::pop_and_apply_projected_inode)`, `src/mds/CInode.cc:514 (CInode::pop_and_apply_projected_inode)`.
- **Map:** [cinode-and-inode-types.md › 3.7 Projection mechanics (read-copy-update)](../cpp-map/04-cache/cinode-and-inode-types.md#37-projection-mechanics-read-copy-update) · [cinode-and-inode-types.md › 3.16 Encoding inode state to clients](../cpp-map/04-cache/cinode-and-inode-types.md#316-encoding-inode-state-to-clients)
- **Code:** `src/mds/CInode.cc:474 (CInode::project_inode)`, `src/mds/CInode.cc:522 (CInode::pop_and_dirty_projected_inode)`, `src/mds/Mutation.cc:254 (MutationImpl::apply)`
- **Notes:** pop always takes the front node, so correctness relies on per-inode journal completion in projection order. pop_and_apply_projected_inode has no caller. If the MDS fails before commit, the acting client has seen uncommitted state and replays the unsafe request ([request lifecycle](../cpp-map/03-request-path/request-lifecycle.md)).

<a id="f-cache-013"></a>

#### F-CACHE-013 · Dirfrag version sequence and fnode projection

*Per-dirfrag version counters and the projected copies of the dirfrag header (fnode) used for commit tracking.*

Each dirfrag has an fnode (version, snap_purged_thru, fragstat, accounted_fragstat, rstat, accounted_rstat, damage_flags, scrub stamps; ENCODE_START(4, 3), stored as the omap header) plus projected_version, committing_version and committed_version, with committed <= committing <= version <= projected. CDir::pre_dirty hands out ++projected_version for every dirtying change to any dentry or primary inode in the frag and for each fnode projection. project_fnode copies the latest fnode (shared immutable pointer, also referenced by the journal dirlump) onto a FIFO; apply installs the front projection. Journal commits of mutations on one dirfrag can complete out of order: CDir::mark_dirty returns early for a stale version, and the applied fnode converges to the newest projection once all outstanding mutations apply.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.8 Versions, projection and dirty tracking](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#38-versions-projection-and-dirty-tracking) · [cdir-cdentry-and-cache-objects.md › 4. Data structures and invariants](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#4-data-structures-and-invariants)
- **Code:** `src/mds/CDir.cc:1400 (CDir::project_fnode)`, `src/mds/CDir.cc:1425 (CDir::pop_and_dirty_projected_fnode)`, `src/mds/CDir.cc:1448 (CDir::mark_dirty)`, `src/mds/mdstypes.cc:337 (fnode_t::encode)`
- **Notes:** The early return replaced an assert in commit fc0f61ca6ba (tracker 79136); which code paths complete out of order is not identified (unverified). _omap_fetched writes snap_purged_thru through a const_cast on the shared fnode, bypassing the RCU rule.

<a id="f-cache-014"></a>

#### F-CACHE-014 · Projected dentry linkage (primary, remote, null)

*The three kinds of dentry linkage and how a pending link change becomes visible.*

linkage_t is primary (the dentry owns the inode, which is stored in this dentry's omap value), remote (hard link: remote_ino and d_type; the target inode pointer is set only when cached and the dentry is then registered in remote_parents) or null (negative entry, placeholder for a create, or deletion awaiting commit). push_projected_linkage appends a projection (a primary projection also pushes the inode's projected parent); pop_projected_linkage applies the front projection via CDir::link_primary_inode / link_remote_inode, and a null projection requires the caller to have unlinked first. Readers choose the projected plane when the dentry lock allows can_read_projected for that client or the lock is xlocked by the same mutation. The opaque per-dentry alternate_name (fscrypt and charmap clients, limited by mds_alternate_name_max) is stored with the name.

- **Class:** POSIX `supporting` · exposure `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.12 CDentry linkage and projection](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#312-cdentry-linkage-and-projection) · [cdir-cdentry-and-cache-objects.md › 3.1 Object graph, identity and keys](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#31-object-graph-identity-and-keys)
- **Code:** `src/mds/CDentry.h:81 (CDentry::linkage_t)`, `src/mds/CDentry.cc:392 (CDentry::push_projected_linkage)`, `src/mds/CDentry.cc:412 (CDentry::pop_projected_linkage)`, `src/mds/CDir.cc:595 (CDir::link_primary_inode)`

<a id="f-cache-015"></a>

#### F-CACHE-015 · Dirty tracking on journal segments

*Which journal segment is responsible for writing back each dirty cache object.*

Dirty objects link intrusive items onto the current LogSegment lists: dirty_dirfrags, new_dirfrags (never-committed dirfrags; kept on the oldest segment that mentions them until the first successful commit), dirty_dentries, dirty_inodes, dirty_parent_inodes (backtraces) and dirty_dirfrag_dir/nest/dirfragtree (inodes with scatter-dirty locks). Re-dirtying moves an item to the newest segment, which is safe because journal entries carry full object state. STATE_DIRTY holds PIN_DIRTY; only auth objects become dirty. STATE_DIRTYRSTAT puts an inode on its parent dirfrag's dirty_rstat_inodes. LogSegment::try_to_expire commits dirfrags, stores base inodes, writes backtraces and nudges scatterlocks before a segment may be trimmed.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cinode-and-inode-types.md › 3.9 Dirty state and log-segment membership](../cpp-map/04-cache/cinode-and-inode-types.md#39-dirty-state-and-log-segment-membership) · [cdir-cdentry-and-cache-objects.md › 3.9 LogSegment dirty lists and what forces a commit](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#39-logsegment-dirty-lists-and-what-forces-a-commit)
- **Code:** `src/mds/LogSegment.h:45 (LogSegment)`, `src/mds/journal.cc:127 (LogSegment::try_to_expire)`, `src/mds/CInode.cc:1197 (CInode::_mark_dirty)`, `src/mds/CDir.cc:1474 (CDir::_mark_dirty)`
- **Notes:** Segment expiry and journal trimming themselves are in the journal category ([MDLog and Journaler](../cpp-map/06-journal/mdlog-and-journaler.md)). See also F-STORE-016.

### Dirfrag load and write-back

How dirfrags are read from and written to their RADOS objects as seen from the cache: when write-back happens, commit and fetch control flow, error handling and damage detection. Byte layouts are in the storage category.

<a id="f-cache-016"></a>

#### F-CACHE-016 · Write-back only at journal segment expiry

*Cache view of write-back: dirty dirfrags reach RADOS only at segment expiry or on explicit flushes.*

See F-STORE-016 (segment expiry) and F-STORE-018 (trim scheduling). Cache view: there is no per-dirfrag dirty-count or age threshold. Dirty dirfrags are committed when MDLog::trim expires their segment; on `flush journal`; on MDLog::trim_all at rank shutdown and after a scrub that repaired something; on `flush_path <path>` (backtrace plus the one dirfrag holding the dentry, -EROFS on a read-only rank); for fragmentation (a never-committed source frag and the new frags); and at mkfs. Slow or blocked commits delay segment expiry and surface as MDS_TRIM.

- **Class:** POSIX `supporting` · exposure `on-disk`, `config`, `admin` · relied on by internal, admin
- **Gates:** mds_log_max_segments (128); mds_log_max_events (-1); mds_log_trim_threshold (128); mds_log_trim_upkeep_interval (1000 ms)
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.9 LogSegment dirty lists and what forces a commit](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#39-logsegment-dirty-lists-and-what-forces-a-commit)
- **Code:** `src/mds/MDLog.cc:723 (MDLog::trim)`, `src/mds/journal.cc:127 (LogSegment::try_to_expire)`, `src/mds/MDCache.cc:13870 (MDCache::flush_dentry_work)`, `src/mds/CInode.cc:1320 (CInode::flush)`

<a id="f-cache-018"></a>

#### F-CACHE-018 · Commit error handling and read-only mode

*What happens when a dirfrag write fails, including the switch to read-only.*

-ENOENT on a never-committed dirfrag whose parent is a stray directory counts as success (purged during failover; STATE_MISSINGOBJS on snapshotted dirs). Any other error logs `"failed to commit dir <df> object, errno <r>"` to the cluster log and calls MDSRank::handle_write_error: respawn on -EBLOCKLISTED, else mds_action_on_write_error (0 ignore, 1 force read-only (default), 2 respawn). _committed then returns early, leaving STATE_COMMITTING, the waiters and the commit auth pin in place. Once read-only, MDLog::trim and _maybe_expired return immediately, log_mark_dirty is skipped and flush_path returns EROFS, so no dirfrag is written back again in that rank life; MDS_READ_ONLY is reported.

- **Class:** POSIX `operational` · exposure `metrics`, `config` · relied on by mon, admin
- **Gates:** mds_action_on_write_error (1)
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.10 Commit (write-back) flow](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#310-commit-write-back-flow) · [cdir-cdentry-and-cache-objects.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/CDir.cc:2808 (CDir::_committed)`, `src/mds/MDSRank.cc:973-991 (MDSRank::handle_write_error)`, `src/mds/MDLog.cc:727-730 (MDLog::trim)`
- **Notes:** See also F-STORE-100 (policy).

<a id="f-cache-020"></a>

#### F-CACHE-020 · Underwater item reconciliation after replay

*Clearing dirty state for replayed items that the previous MDS had already written back.*

When a dirfrag that has never been committed in this cache life (committed_version == 0) is fetched, cached dirty dentries and inodes that exactly match the on-disk copy (same remote target, or same ino and inode version) are marked clean, because they were flushed before the previous MDS died.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.11 Fetch (read) flow](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#311-fetch-read-flow) · [cinode-and-inode-types.md › 3.9 Dirty state and log-segment membership](../cpp-map/04-cache/cinode-and-inode-types.md#39-dirty-state-and-log-segment-membership)
- **Code:** `src/mds/CDir.cc:1967 (CDir::_load_dentry)`

### Directory contents and negative lookups

What a cached dirfrag knows about the names it does not hold: completeness, the bloom filter of trimmed names, and null dentries.

<a id="f-cache-023"></a>

#### F-CACHE-023 · Directory completeness and authoritative negative lookups

*A dirfrag flagged complete answers misses with ENOENT without reading RADOS.*

STATE_COMPLETE means every dentry of the fragment that exists in RADOS or in the journal is cached. Set by a full fetch, mkdir of a new directory, system dirs at mkfs and populate_mydir, fragment creation on the auth, replay of a dirlump flagged complete, go_bad, and the stray-parent fetch shortcut; carried by migration and split. Cleared by trimming an auth dentry that is not a clean null, by trim_non_auth, by rejoin of non-auth dirs and by replay of an importing dirlump. Readdir requires a complete dirfrag and fetches otherwise; path_traverse, open_ino and discover answer ENOENT (or create a null dentry) on a miss in a complete dirfrag. Directory emptiness for rmdir/rename is decided from the projected fragstat, not from completeness.

- **Class:** POSIX `supporting` · exposure `internal` · relied on by kclient, libcephfs, ganesha
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.14 Directory contents, counters and completeness](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#314-directory-contents-counters-and-completeness) · [cdir-cdentry-and-cache-objects.md › 7. Externally visible contract](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#7-externally-visible-contract)
- **Code:** `src/mds/MDCache.cc:8749 (MDCache::path_traverse)`, `src/mds/Server.cc:5172-5183 (Server::handle_client_readdir)`, `src/mds/MDCache.cc:7203-7246 (MDCache::trim_dentry)`, `src/mds/Server.cc:9225 (Server::_dir_is_nonempty_unlocked)`
- **Notes:** A wrong negative answer is a POSIX violation visible to every client.

<a id="f-cache-024"></a>

#### F-CACHE-024 · Bloom filter of trimmed names

*A per-dirfrag filter that keeps negative lookups cache-only after names were trimmed from a complete dirfrag.*

When trim_dentry removes a head dentry that is not a clean null from an auth dirfrag, it inserts the name into a per-dirfrag bloom filter and then clears STATE_COMPLETE. The filter is created on the first such trim from a complete dirfrag (capacity max(100, head + snap items), false-positive rate 1/capacity; never for an incomplete dirfrag or in standby-replay) and keeps accumulating afterwards. A head-snapid miss in an auth dirfrag whose filter does not contain the name is answered ENOENT without a fetch (path_traverse, open_ino_traverse_dir, handle_discover). mark_complete, export and abort_import drop the filter; it is not serialized and not counted in the mds_co memory pool.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.14 Directory contents, counters and completeness](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#314-directory-contents-counters-and-completeness) · [mdcache-lifecycle-trim-and-memory.md › 3.5.2 trim_dentry, trim_inode, trim_dirfrag](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#352-trim_dentry-trim_inode-trim_dirfrag)
- **Code:** `src/mds/CDir.cc:749 (CDir::add_to_bloom)`, `src/mds/MDCache.cc:8745-8752 (MDCache::path_traverse)`, `src/mds/MDCache.cc:7243-7245 (MDCache::trim_dentry)`
- **Notes:** Correctness requires every trim of such a name to insert before completeness is cleared; only false positives (an extra fetch) are allowed.

<a id="f-cache-025"></a>

#### F-CACHE-025 · Null dentries

*Cached negative entries and pending deletions, and when they are dropped.*

Null dentries cache non-existence (keyed-fetch misses get first = global snaprealm newest_seq + 1; discover replicates nulls; negative traversal tails) or represent deletions awaiting commit (a dirty null becomes an omap key removal). Clean nulls are trimmable without clearing completeness; readdir deletes unreferenced, unprojected nulls it walks over and close_dirfrag drops them. try_remove_unlinked_dn drops a STATE_NEW (never committed) dentry that became null with no refs other than the dirty pin; only new stray dentries carry STATE_NEW. try_remove_dentries_for_stray cleans dirfrags of directories that sit in a stray dir.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.14 Directory contents, counters and completeness](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#314-directory-contents-counters-and-completeness)
- **Code:** `src/mds/CDir.cc:687 (CDir::try_remove_unlinked_dn)`, `src/mds/CDir.cc:782 (CDir::remove_null_dentries)`, `src/mds/CDir.cc:805 (CDir::try_remove_dentries_for_stray)`, `src/mds/Server.cc:5240-5244 (Server::handle_client_readdir)`

### Cache object model: references, auth pins, freezing, waiters

The generic machinery shared by CInode, CDir and CDentry: reference counts that keep the cache a connected tree, object indexes, auth pins and the freeze protocols built on them, and waiter queues.

<a id="f-cache-026"></a>

#### F-CACHE-026 · Reference counts and upward pinning

*Named reference counts that keep every referenced object's ancestors resident.*

Every cache object has a total ref count plus a per-pin-type ref_map (MDS_REF_SET is always defined); get() calls first_get on 0 -> 1, put() asserts on underflow per pin type and calls last_put on 1 -> 0. Upward hooks: a non-empty CDir holds PIN_CHILD, a referenced CDir pins its inode (PIN_DIRFRAG), a referenced inode pins its primary dentry (PIN_INODEPIN), and a referenced dentry is pinned in the LRU. The cache is therefore a connected tree from base inodes and only unreferenced leaf dentries can be expired. Objects with STATE_NOTIFYREF (CDentry::STATE_STRAY, CInode::STATE_ORPHAN) get _put() on every put; once only dirty pins remain it queues a stray evaluation, which is how an unlinked but open file is purged after its last cap or request goes ([strays and purge](../cpp-map/11-background-services/strays-and-purge.md)). remove_inode and close_dirfrag assert ref == 0; CDir::remove_dentry does not check refs.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.2 Reference counting and pins](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#32-reference-counting-and-pins) · [mdcache-lifecycle-trim-and-memory.md › 3.1 Object graph and residency ("what is pinned and why")](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#31-object-graph-and-residency-what-is-pinned-and-why) · [cinode-and-inode-types.md › 3.20 Pins (reference counts)](../cpp-map/04-cache/cinode-and-inode-types.md#320-pins-reference-counts)
- **Code:** `src/mds/MDSCacheObject.h:138 (MDSCacheObject::put)`, `src/mds/MDSCacheObject.h:164 (MDSCacheObject::get)`, `src/mds/CInode.cc:977 (CInode::first_get)`, `src/mds/CDir.cc:1545 (CDir::first_get)`, `src/mds/CInode.cc:991 (CInode::_put)`
- **Notes:** Several pin ids are defined but never taken (PIN_OPENINGDIR, PIN_BATCHOPENJOURNAL, PIN_SCATTERED, PIN_PASTSNAPPARENT, CDir::PIN_INOWAITER, CDentry::PIN_SCRUBPARENT).

<a id="f-cache-027"></a>

#### F-CACHE-027 · Inode maps and object registry

*Indexes from inode numbers and object identifiers to cached objects.*

inode_map (unordered_map ino -> head CInode, auth and replica) and snap_inode_map (map vinodeno_t -> snapped CInode, searched with lower_bound), plus root, myin, strays[] and base_inodes pointers, maintained only by add_inode / remove_inode (duplicate (ino, last) asserts). remove_inode tears down dirty, dirty-parent, scatter-dirty and client-writeable state, the open-file list entry, export-pin queues, ephemeral pins and replay_taken_inos. get_object resolves MDSCacheObjectInfo (ino, snap) / (dirfrag, dname, snap) / dirfrag for peer auth-pin requests. Each upkeep period asserts CInode::count() == inode_map + snap_inode_map + shadow inodes.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 4. Data structures and invariants](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#4-data-structures-and-invariants) · [mdcache-lifecycle-trim-and-memory.md › 3.10.4 Opening at boot](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#3104-opening-at-boot) · [mdcache-subtrees-traversal-replication.md › 4. Data structures and invariants](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#4-data-structures-and-invariants)
- **Code:** `src/mds/MDCache.cc:331 (MDCache::add_inode)`, `src/mds/MDCache.cc:359 (MDCache::remove_inode)`, `src/mds/MDCache.cc:925 (MDCache::get_object)`, `src/mds/MDCache.cc:7972 (MDCache::check_memory_usage)`

<a id="f-cache-028"></a>

#### F-CACHE-028 · Auth pins with one-level nested accounting

*Counters that mark objects in use by operations on the authority and block authority changes or restructuring.*

An auth pin means an operation uses the object as its authority. Pins on a dentry or on a dentry's primary inode also increment the dirfrag's dir_auth_pins (exactly one level); CDir pins do not propagate upward; while a tree freeze is in progress every member dirfrag also adds to a shared freeze_tree_state counter. can_auth_pin fails with ERR_NOT_AUTH, ERR_FRAGMENTING_DIR (freezing/frozen dir), ERR_EXPORTING_TREE (freezing/frozen tree) or ERR_EXPORTING_INODE (freezing/frozen inode); a CInode otherwise defers to its parent dentry's dirfrag. Takers: requests, fetch, commit, the freezers, fragmentation, lock caches, predirty_journal_parents, scrub, purge, the Migrator and the recovery queue; readdir and fragmentation may force a fetch or commit with ignore_authpinnability. The drop-everything-and-retry rule of acquire_locks is in the locking category ([acquiring locks](../cpp-map/05-locking/acquiring-locks.md)).

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.5 Auth pins](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#35-auth-pins) · [cinode-and-inode-types.md › 3.18 Hierarchy, authority, auth pins, freezing and waiters](../cpp-map/04-cache/cinode-and-inode-types.md#318-hierarchy-authority-auth-pins-freezing-and-waiters)
- **Code:** `src/mds/CDir.cc:3210 (CDir::auth_pin)`, `src/mds/CDentry.cc:465 (CDentry::auth_pin)`, `src/mds/CInode.cc:3094 (CInode::can_auth_pin)`, `src/mds/CDir.cc:3528 (CDir::can_auth_pin)`
- **Notes:** See also F-CAPS-082.

<a id="f-cache-029"></a>

#### F-CACHE-029 · Dirfrag freeze (freeze_dir)

*Draining operations from one dirfrag and its direct children so it can be restructured.*

freeze_dir auth-pins the dir and freezes immediately if no other pin is on the dir, dir_auth_pins == 0 and (for a non-subtree-root) its inode is not frozen; otherwise it sets STATE_FREEZINGDIR and invalidates lock caches that auth-pin the dir. maybe_finish_freeze completes when auth_pins == 1 and dir_auth_pins == 0; pins deeper in the tree do not count. _freeze_dir sets STATE_FROZENDIR and PIN_FROZEN and, on a non-subtree-root auth frag, auth-pins the directory inode, which keeps the enclosing dirfrag from freezing above it. unfreeze_dir finishes WAIT_UNFREEZE; cancelling a freezing dir completes WAIT_FROZEN synchronously with -1. Used by fragmentation.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.6 Freezing](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#36-freezing)
- **Code:** `src/mds/CDir.cc:3610 (CDir::freeze_dir)`, `src/mds/CDir.cc:3565 (CDir::maybe_finish_freeze)`, `src/mds/CDir.cc:3629 (CDir::_freeze_dir)`, `src/mds/CDir.cc:3645 (CDir::unfreeze_dir)`

<a id="f-cache-030"></a>

#### F-CACHE-030 · Subtree freeze (freeze_tree)

*Draining operations from a whole subtree so its authority can move to another rank.*

freeze_tree auth-pins the root, creates freeze_tree_state {auth_pins = root pins + dir_auth_pins}, walks all nested non-subtree-root dirfrags breadth-first attaching the shared state and adding their pins, invalidates lock caches, and completes when the shared counter equals 1 (the freezer's pin). _freeze_tree turns dirfrags opened after the freeze started into separate subtree roots (bounds), makes the root a subtree root with dir_auth (me, me) and sets STATE_FROZENTREE. unfreeze_tree restores (me, -2) and queues WAIT_UNFREEZE; cancelling completes WAIT_FROZEN with -1. adjust_freeze_after_rename removes a directory renamed out of a freezing tree from the freeze set. Process-wide counters num_frozen_trees / num_freezing_trees make the predicates cheap. Used by subtree migration ([subtree migration](../cpp-map/08-multi-mds/subtree-migration.md)).

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.6 Freezing](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#36-freezing) · [mdcache-subtrees-traversal-replication.md › 3.1.4 Ambiguous authority and WAIT_SINGLEAUTH](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#314-ambiguous-authority-and-wait_singleauth)
- **Code:** `src/mds/CDir.cc:3345 (CDir::freeze_tree)`, `src/mds/CDir.cc:3385 (CDir::_freeze_tree)`, `src/mds/CDir.cc:3446 (CDir::unfreeze_tree)`, `src/mds/CDir.cc:3497 (CDir::adjust_freeze_after_rename)`
- **Notes:** See also F-MULTI-003 (use in migration).

<a id="f-cache-032"></a>

#### F-CACHE-032 · Single-inode freeze, ambiguous inode auth and frozen-inode tracking

*Freezing one inode for cross-rank rename and the per-dirfrag bookkeeping that interacts with lock caches.*

freeze_inode(allowance) and freeze_auth_pin are used only by cross-rank rename: peer rename prep freezes the source inode with allowance 3, a peer auth-pin request carrying auth_pin_freeze freezes the inode's auth pins to avoid an ABBA deadlock, and rejoin restores frozen auth pins. Each dirfrag tracks freezing_inodes and num_frozen_inodes; lock caches (async dirops) that auth-pin a dirfrag increment frozen_inode_suppressed, and while it is non-zero inodes there cannot finish freezing; freezing invalidates such lock caches. STATE_AMBIGUOUSAUTH marks an inode whose authority moves in a cross-rank rename; clear_ambiguous_auth wakes WAIT_SINGLEAUTH waiters. Cap export uses STATE_EXPORTINGCAPS, not a freeze. Rename protocol: [rename](../cpp-map/03-request-path/rename.md).

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, internal
- **Map:** [cinode-and-inode-types.md › 3.18 Hierarchy, authority, auth pins, freezing and waiters](../cpp-map/04-cache/cinode-and-inode-types.md#318-hierarchy-authority-auth-pins-freezing-and-waiters) · [cdir-cdentry-and-cache-objects.md › 3.6 Freezing](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#36-freezing)
- **Code:** `src/mds/Server.cc:10604 (Server::handle_peer_rename_prep)`, `src/mds/Server.cc:3373 (Server::handle_peer_auth_pin)`, `src/mds/CDir.cc:3670 (CDir::enable_frozen_inode)`, `src/mds/CInode.cc:3079 (CInode::clear_ambiguous_auth)`

<a id="f-cache-033"></a>

#### F-CACHE-033 · Waiter queues on cache objects

*Per-object queues of continuations waiting for a state change, with masks, ordering and routing rules.*

Each object has a lazily allocated multimap sequence -> {128-bit mask, MDSContext}. Bits 0-3 are CDir waits (DENTRY, COMPLETE, FROZEN, CREATED), bits 0-4 CInode waits (DIR, FROZEN, TRUNC, FLOCK, QUARANTINE), bit 59 WAIT_UNFREEZE, 60 WAIT_SINGLEAUTH, 61 WAIT_ORDERED (flag), and bits 64 + 4k .. 64 + 4k + 3 the RD/WR/XLOCK/REMOTEXLOCK waits of the embedded lock with wait shift k. Lock waiters always get a global sequence (WAIT_ORDERED), so unordered waiters come first, then lock waiters FIFO. Routing: CDentry forwards UNFREEZE/SINGLEAUTH to its dir; CDir forwards SINGLEAUTH up to the subtree root; CInode forwards SINGLEAUTH (unless itself ambiguous) and UNFREEZE (unless itself freezing/frozen) to its parent dir. CDir::waiting_on_dentry maps (name, snapid) to contexts for keyed fetches and discovers (PIN_DNWAITER); split and merge re-home them by name hash. PIN_WAITER while non-empty.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.7 Waiters](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#37-waiters) · [cinode-and-inode-types.md › 3.18 Hierarchy, authority, auth pins, freezing and waiters](../cpp-map/04-cache/cinode-and-inode-types.md#318-hierarchy-authority-auth-pins-freezing-and-waiters)
- **Code:** `src/mds/MDSCacheObject.h:267 (MDSCacheObject::add_waiter)`, `src/mds/CDir.cc:1344 (CDir::add_waiter)`, `src/mds/CInode.cc:2929 (CInode::add_waiter)`, `src/mds/CDir.cc:1321 (CDir::take_dentry_waiting)`
- **Notes:** count_waiters() counts entries whose sequence key equals the mask, not matching waiters, so the unfreeze_tree_waiters field of the export status dump is effectively always 0. See also F-CAPS-087 (FIFO lock waiters).

<a id="f-cache-034"></a>

#### F-CACHE-034 · Waiter completion semantics

*Whether woken continuations run inline or are queued, and which result code they see.*

MDSCacheObject::finish_waiting completes contexts synchronously in the caller's stack; CDir::finish_waiting does so only for r < 0 and otherwise appends them to MDSRank::finished_queue, drained under mds_lock by the mds-rank-progr thread with r = 0. C_MDS_RetryRequest ignores r and re-dispatches the request, so errors must be rediscovered by the retried request (for example through the DamageTable). Cancelled freezes complete WAIT_FROZEN waiters synchronously with -1.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.7 Waiters](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#37-waiters) · [cdir-cdentry-and-cache-objects.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDSCacheObject.cc:26 (MDSCacheObject::finish_waiting)`, `src/mds/CDir.cc:1384 (CDir::finish_waiting)`, `src/mds/MDSRank.cc:1320 (MDSRank::_advance_queues)`, `src/mds/MDCache.cc:13271 (C_MDS_RetryRequest)`

### Authority and the subtree map

Which rank is authoritative for each cached object, the subtree map that partitions the namespace among ranks, and the primitives that change it. Migration and balancing policies are in the multi-MDS category.

<a id="f-cache-035"></a>

#### F-CACHE-035 · Authority of cache objects and the auth bit

*Rules that decide which rank owns an inode, dirfrag or dentry, and the cached bit that records it.*

authority() returns a pair of ranks: a CDentry has its dirfrag's authority; a CDir has dir_auth if it is a subtree root (dir_auth != (-1,-2)), else its inode's; a CInode has inode_auth if set (only base inodes: the root (mdsmap root, always 0) and ~mdsN = N), else the authority of the dirfrag holding its primary dentry, else that of its first projected parent, else UNDEF. Different dirfrags of one directory can have different authorities; a hard-link target belongs to the rank owning its primary dentry. Decisions use the cached STATE_AUTH bit, equal to authority().first == whoami except during migration, cross-rank rename and replay/resolve, after which recalc_auth_bits recomputes it. Only auth objects are dirtied, fetched or committed. A directory inode with subtree-root dirfrags on other ranks keeps filelock/nestlock scattered (LOCK_MIX) so each rank updates its frags' stats.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, internal
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.1.1 mds_authority_t](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#311-mds_authority_t) · [mdcache-subtrees-traversal-replication.md › 3.1.2 Authority of each object type](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#312-authority-of-each-object-type) · [mdcache-subtrees-traversal-replication.md › 3.1.3 The STATE_AUTH bit versus authority()](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#313-the-state_auth-bit-versus-authority) · [cdir-cdentry-and-cache-objects.md › 3.4 Authority, replicas and nonces](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#34-authority-replicas-and-nonces)
- **Code:** `src/mds/CDir.cc:3081 (CDir::authority)`, `src/mds/CInode.cc:3157 (CInode::authority)`, `src/mds/CDentry.cc:191 (CDentry::authority)`, `src/mds/MDCache.cc:3902 (MDCache::recalc_auth_bits)`

<a id="f-cache-036"></a>

#### F-CACHE-036 · Subtree map

*The per-rank map of delegation points that partitions the namespace among active ranks.*

subtrees is `map<CDir*, set<CDir*>>`: keys are all dirfrags with dir_auth != DEFAULT known to the rank (its auth subtrees, including the root dirfrag on rank 0 and its own ~mdsN; imported and aux subtrees; every bound of those; non-auth subtree roots created when replicas are decoded at delegation boundaries; UNDEF subtrees during replay); values are the direct bounds. Every key holds PIN_SUBTREE and every non-base key is a bound of exactly one other key. Everything below a root down to its bounds inherits the root's authority. The rank's own part is journaled as ESubtreeMap at major segment boundaries ([events and replay](../cpp-map/06-journal/events-and-replay.md)) and exchanged in resolve ([resolve and rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md)). Clients learn authority only through DirStat hints and forwards.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, internal
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.2 The subtree map](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#32-the-subtree-map) · [mdcache-subtrees-traversal-replication.md › 3.2.1 Structure](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#321-structure) · [mdcache-subtrees-traversal-replication.md › 3.1.5 Example](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#315-example)
- **Code:** `src/mds/MDCache.h:1402 (MDCache::subtrees)`, `src/mds/MDCache.cc:1342 (MDCache::get_subtree_root)`, `src/mds/MDCache.cc:2624 (MDCache::create_subtree_map)`
- **Notes:** show_subtrees asserts subtree-map consistency only when the debug level gathers its output, so raising debug_mds can turn a latent inconsistency into a crash. See also F-STORE-025 (ESubtreeMap).

<a id="f-cache-037"></a>

#### F-CACHE-037 · Subtree create, merge and bounded-adjust primitives

*Operations that create, change, merge and reshape subtrees.*

adjust_subtree_auth(dir, auth) makes dir a subtree root (or changes its auth), moving bounds of the enclosing root that now lie under dir and adjusting popularity; it never merges. try_subtree_merge_at merges a subtree into its parent subtree when both have identical, unambiguous dir_auth and the child is not EXPORTBOUND or AUXSUBTREE, then eval_subtree_root re-evaluates the directory's scattered filelock/nestlock. adjust_bounded_subtree_auth(dir, bounds, auth) imposes exactly [dir, bounds) as one subtree (splitting local dirfrags to match finer bounds, never merging) and aborts the daemon via verify_subtree_bounds on mismatch; callers are journal replay of ESubtreeMap/EExport/`EImport*`, resolve, migration prep and bystander notify. remove_subtree drops a key without resetting dir_auth.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, internal
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.2.2 adjust_subtree_auth(dir, auth, adjust_pop=true) (sr...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#322-adjust_subtree_authdir-auth-adjust_poptrue-srcmdsmdcachecc978-mdcacheadjust_subtree_auth) · [mdcache-subtrees-traversal-replication.md › 3.2.4 adjust_bounded_subtree_auth(dir, bounds, auth) (src...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#324-adjust_bounded_subtree_authdir-bounds-auth-srcmdsmdcachecc1125-mdcacheadjust_bounded_subtree_auth) · [mdcache-subtrees-traversal-replication.md › 3.2.5 Other mutations](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#325-other-mutations)
- **Code:** `src/mds/MDCache.cc:978 (MDCache::adjust_subtree_auth)`, `src/mds/MDCache.cc:1065 (MDCache::try_subtree_merge_at)`, `src/mds/MDCache.cc:1125 (MDCache::adjust_bounded_subtree_auth)`, `src/mds/MDCache.cc:1412 (MDCache::verify_subtree_bounds)`

<a id="f-cache-038"></a>

#### F-CACHE-038 · Ambiguous authority and WAIT_SINGLEAUTH

*Marking objects whose authority is in transit and parking requests until it settles.*

dir_auth pairs (a, b) with b != -2 mark authority moving between ranks (export/import in flight, ambiguous imports until resolve); (a, a) marks a frozen export tree or an import being acked; CInode STATE_AMBIGUOUSAUTH marks an inode moving in a cross-rank rename. A request needing the auth of an ambiguous object waits WAIT_SINGLEAUTH; these waiters are hoisted to the subtree root dirfrag and woken together by set_dir_auth when the second rank becomes UNKNOWN. maybe_request_forward_to_auth waits on ambiguity, forwards when not auth, else proceeds.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, internal
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.1.4 Ambiguous authority and WAIT_SINGLEAUTH](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#314-ambiguous-authority-and-wait_singleauth)
- **Code:** `src/mds/MDSCacheObject.h:110 (MDSCacheObject::is_ambiguous_auth)`, `src/mds/CDir.cc:3143 (CDir::set_dir_auth)`, `src/mds/MDCache.cc:8893 (MDCache::maybe_request_forward_to_auth)`
- **Notes:** dir_auth transitions per migration step are listed in the multi-MDS category ([subtree migration](../cpp-map/08-multi-mds/subtree-migration.md)).

<a id="f-cache-039"></a>

#### F-CACHE-039 · Subtree map maintenance on directory rename

*Keeping the subtree map correct when a directory moves between parents.*

Every rename or rmdir of a directory registers project_subtree_rename before journaling (used only so that an ESubtreeMap written in between already reflects it) and runs adjust_subtree_after_rename after applying: subtree-root dirfrags move between parent bound sets and may merge; a mid-subtree dirfrag whose old and new parent subtrees have different authority becomes a subtree root that keeps the old authority; popularity moves with it. Renaming a directory across an authority boundary therefore creates new delegation points without balancer activity. Replicas run the same adjustment when MDentryUnlink moves a directory to a stray, and replay does too.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, internal
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.2.6 Directory renames (src/mds/MDCache.cc:1450 (MDCache...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#326-directory-renames-srcmdsmdcachecc1450-mdcacheproject_subtree_rename-srcmdsmdcachecc1457-mdcacheadjust_subtree_after_rename)
- **Code:** `src/mds/MDCache.cc:1450 (MDCache::project_subtree_rename)`, `src/mds/MDCache.cc:1457 (MDCache::adjust_subtree_after_rename)`

<a id="f-cache-040"></a>

#### F-CACHE-040 · Aux subtrees

*Subtree roots that are never merged into their parent subtree.*

STATE_AUXSUBTREE keeps a dirfrag as its own subtree root that try_subtree_merge never merges. It is set for dirfrags export-pinned to this rank (MDBalancer::handle_export_pins with adjust_subtree_auth(dir, me)) and for dirfrags being fragmented when only some sources are subtree roots (sources must be all subtree roots or none); it survives split and merge. trim() drops empty aux subtrees whose inode is also auth. Export pin policy: [balancer and pinning](../cpp-map/08-multi-mds/balancer-and-pinning.md).

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, admin
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.2.5 Other mutations](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#325-other-mutations) · [mdcache-lifecycle-trim-and-memory.md › 3.5.3 Subtree and base-inode trimming inside trim() (src/...](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#353-subtree-and-base-inode-trimming-inside-trim-srcmdsmdcachecc7042-7147-mdcachetrim)
- **Code:** `src/mds/CDir.h:145 (CDir::STATE_AUXSUBTREE)`, `src/mds/MDBalancer.cc:206-232 (MDBalancer::handle_export_pins)`, `src/mds/MDCache.cc:12025 (MDCache::fragment_freeze_dirs)`
- **Notes:** Under mds_thrash_exports (testing) Migrator::export_dir sets AUXSUBTREE on the export root rather than on the randomly chosen nested dirfrag (code reading). See also F-MULTI-041 (pin enforcement).

### Path traversal and lookup by inode number

Resolving client paths and inode numbers to cache objects on one rank: per-component locking, fetching, forwarding or discovering, hard links, ESTALE recovery and open_ino. Request handlers are in the namespace category.

<a id="f-cache-041"></a>

#### F-CACHE-041 · Path traversal (path_traverse)

*Resolving a request's (base inode, relative path) to dentries and inodes, taking per-component locks.*

Every client request names a base inode and a relative path. path_traverse returns 0 (done), 1 (waiter registered), 2 (forwarded) or -errno. Per component: -ENOTDIR if not a directory; pick the dirfrag by name hash; open it locally if the inode is auth (wait UNFREEZE on a frozen inode) or discover it; -EIO for damaged dirfrags or dentries; -ENOENT for purging dentries; with RDLOCK_PATH rdlock each dentry (XLOCK_DENTRY xlocks the tail plus parent filelock/nestlock wrlocks; RDLOCK_SNAP rdlocks snaplock of every ancestor; WANT_DIRLAYOUT also policylock up to the nearest layout), else wait WAIT_RD on unreadable dentries; a miss in an auth dirfrag is final if complete or absent from the bloom filter, otherwise fetch that one key; a non-auth dirfrag is discovered or forwarded. An uncached base returns -ESTALE (callers then run find_ino_peers), except ~mdsN (discovered) and stray inodes (rewritten as `stray<i>` under ~mdsN). 13 `MDS_TRAVERSE_*` flags select behaviour per caller.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.5 Path traversal: path_traverse (src/mds/MDCache.cc:844...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#35-path-traversal-path_traverse-srcmdsmdcachecc8443-mdcachepath_traverse) · [mdcache-subtrees-traversal-replication.md › 3.5.1 Flags (src/mds/MDCache.h:136-148 (MDS_TRAVERSE_*))](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#351-flags-srcmdsmdcacheh136-148-mds_traverse_) · [mdcache-subtrees-traversal-replication.md › 3.5.3 Per-component loop (src/mds/MDCache.cc:8520-8859 (M...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#353-per-component-loop-srcmdsmdcachecc8520-8859-mdcachepath_traverse)
- **Code:** `src/mds/MDCache.cc:8443 (MDCache::path_traverse)`, `src/mds/MDCache.h:136-148 (MDS_TRAVERSE_DISCOVER)`, `src/mds/Server.cc:3879 (Server::rdlock_path_pin_ref)`, `src/mds/Server.cc:3960 (Server::rdlock_path_xlock_dentry)`
- **Notes:** The traversal itself does not wait on frozen dirfrags (that check is commented out); waits come from lock acquisition and auth pins. Lock-cache attachment (CHECK_LOCKCACHE) belongs to the caps category ([leases, lock caches and async dirops](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md)). See also F-CAPS-079 (locking flags), F-NS-005 (helpers), F-SNAP-021 (.snap paths), F-BG-060 (damaged objects).

<a id="f-cache-044"></a>

#### F-CACHE-044 · Forward-or-discover for non-auth objects

*Sending a client to the authoritative rank, or replicating instead to stop forwarding loops.*

When traversal reaches a dirfrag or target this rank is not auth for, it forwards by default: request_forward makes the client resend through MClientRequestForward(tid, dest, num_fwd + 1, client_must_resend = true); the MDS never relays client requests. The kernel aborts the request with -EMULTIHOP if the new forward count is not larger than its own. For a client request at a path depth < num_fwd the rank discovers (replicates) from the auth instead of forwarding, so a request forwarded n times replicates the first n path levels on intermediate ranks. Batched getattr/lookup heads forward their whole batch; internal ops complete with -EXDEV; requests from MDSs are dropped. Traversals with the DISCOVER flag (peers, find_ino, dir updates, imports) always discover. mds_forward_all_requests_to_auth makes rdlock_path_pin_ref always want the auth.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Gates:** mds_forward_all_requests_to_auth (false)
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.5.3 Per-component loop (src/mds/MDCache.cc:8520-8859 (M...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#353-per-component-loop-srcmdsmdcachecc8520-8859-mdcachepath_traverse) · [mdcache-subtrees-traversal-replication.md › 3.5.4 End of traversal](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#354-end-of-traversal) · [mdcache-subtrees-traversal-replication.md › 7.1 Linux kernel client](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#71-linux-kernel-client)
- **Code:** `src/mds/MDCache.cc:8820-8829 (MDCache::path_traverse)`, `src/mds/MDCache.cc:9970 (MDCache::request_forward)`, `src/mds/MDSRank.cc:1504 (MDSRank::forward_message_mds)`, `linux/fs/ceph/mds_client.c:4291 (handle_forward)`
- **Notes:** Forward message encoding and client resend rules are in the protocol category ([request lifecycle](../cpp-map/03-request-path/request-lifecycle.md)). See also F-PROTO-076 (message and client resend rules), F-NS-053.

<a id="f-cache-045"></a>

#### F-CACHE-045 · Hard-link target resolution

*Following a remote dentry to the inode it names, wherever that inode lives.*

A remote dentry (remote_ino, d_type) is linked to the cached target (CDentry::link_remote, remote_parents, PIN_REMOTEPARENT) or resolved with open_remote_dentry -> open_ino(want_replica, pool = metadata pool for DT_DIR targets, else the default data pool); discover never follows remote links. On failure, if the dentry still names the same ino, it gets STATE_BADREMOTEINO and the DamageTable records remote damage: traversal then returns -EIO and readdir silently omits the entry; more than mds_damage_table_max_entries (10000) damage entries mark the rank damaged.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_damage_table_max_entries (10000)
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.5.5 Related helpers](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#355-related-helpers) · [mdcache-subtrees-traversal-replication.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDCache.cc:9016 (MDCache::open_remote_dentry)`, `src/mds/MDCache.cc:9026 (MDCache::_open_remote_dentry_finish)`, `src/mds/MDCache.cc:8975 (MDCache::get_dentry_inode)`, `src/mds/Server.cc:5268 (Server::handle_client_readdir)`
- **Notes:** See also F-BG-047 (scrub hard-link check).

<a id="f-cache-047"></a>

#### F-CACHE-047 · Lookup by inode number (open_ino)

*Locating any inode by number, cluster-wide, for NFS file handles, hard links and recovery.*

open_ino(ino, pool, ...) completes with the authoritative rank or -errno; concurrent opens of one ino coalesce. Without ancestor hints it asks peer caches first (see the peer protocol entry), then reads the backtrace: getxattr "parent" of `<ino hex>.00000000` in the pool the caller passes, or the default (first) data pool for -1 as LOOKUPINO passes, following the backtrace's pool field and retrying in the metadata pool on ENOENT; then it walks the ancestors from the immediate parent upward, fetching single keys in auth dirfrags or discovering in non-auth ones, recursively opening the parent when needed. Errors: decode failure -EIO, empty ancestors -ESTALE, the same parent twice -EINVAL; the Server maps -ENOENT/-ENODATA to -ESTALE for clients. Callers: LOOKUPINO, LOOKUPHASH, LOOKUPPARENT, LOOKUPNAME (NFS handles via the kernel and Ganesha, kernel quota realm inodes, snapdir stat, libcephfs lookup_ino), hard-link targets, ESTALE fallback, rejoin cap imports and OpenFileTable prefetch.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk`, `mds-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, osd, mds-ranks
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.9 open_ino: finding an inode by number](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#39-open_ino-finding-an-inode-by-number) · [mdcache-subtrees-traversal-replication.md › 3.9.1 Callers](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#391-callers) · [mdcache-subtrees-traversal-replication.md › 3.9.2 State machine (do_open_ino, src/mds/MDCache.cc:9372...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#392-state-machine-do_open_ino-srcmdsmdcachecc9372-mdcachedo_open_ino) · [mdcache-subtrees-traversal-replication.md › 3.9.3 Backtraces](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#393-backtraces)
- **Code:** `src/mds/MDCache.cc:9591 (MDCache::open_ino)`, `src/mds/MDCache.cc:9372 (MDCache::do_open_ino)`, `src/mds/MDCache.cc:10457 (MDCache::fetch_backtrace)`, `src/mds/MDCache.cc:9117 (MDCache::_open_ino_backtrace_fetched)`, `src/mds/Server.cc:4617 (Server::_lookup_ino_2)`
- **Notes:** Any inode number a client holds (file handles, caps) must stay resolvable by number. Backtrace format and its lazy writing at segment expiry are in the storage category ([RADOS object catalog](../cpp-map/07-storage-formats/rados-object-catalog.md)). Because the backtrace stage runs only after every peer was checked, an uncached LOOKUPINO waits while any in-rank is not active. See also F-STORE-062 (backtrace reads), F-NS-026 (LOOKUPINO handler), F-PROTO-099 (client expectations).

<a id="f-cache-048"></a>

#### F-CACHE-048 · open_ino peer protocol

*Asking other ranks whether they cache an inode before reading backtraces.*

MMDSOpenIno(tid, ino, ancestors) is sent to one active peer at a time (CLIENTREPLAY..STOPPING, or REJOIN.. when the requester itself is in REJOIN); an auth hint names the rank to ask, and if that rank is not active nobody else is asked and the search waits. A peer that caches the inode as auth replies MMDSOpenInoReply with the full ancestor chain (dirino, dname, version); with a replica it replies an auth hint; otherwise it runs its own non-discovering ancestor traversal (it may fetch dirfrags it is auth for). Peers below REJOIN drop the query silently. With want_replica the requester adopts returned ancestors and discovers. There are no timeouts: queries are re-sent when peers fail or recover (kick_open_ino_peers).

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.9.2 State machine (do_open_ino, src/mds/MDCache.cc:9372...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#392-state-machine-do_open_ino-srcmdsmdcachecc9372-mdcachedo_open_ino) · [mdcache-subtrees-traversal-replication.md › 5.1 Messages (all MMDSOp, routed by MDCache::dispatch bec...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#51-messages-all-mmdsop-routed-by-mdcachedispatch-because-type--0xff00--mds_port_cache-only-accepted-from-ceph_entity_type_mds-peers-srcmdsmdsrankcc1236-mdsrankhandle_message)
- **Code:** `src/mds/MDCache.cc:9414 (MDCache::do_open_ino_peer)`, `src/mds/MDCache.cc:9460 (MDCache::handle_open_ino)`, `src/mds/MDCache.cc:9502 (MDCache::handle_open_ino_reply)`
- **Notes:** A stale auth hint naming a rank that is not active can block the search until every other in-rank is checked; whether it hangs for good for a permanently stopped rank is unverified at runtime.

<a id="f-cache-049"></a>

#### F-CACHE-049 · Batched dentry-key fetches for open_ino

*Grouping many single-name dirfrag reads into one omap read during recovery prefetch.*

While open_ino_batch is set (rejoin cap import, OpenFileTable prefetch), single-key fetches issued by open_ino ancestor traversals are grouped per dirfrag into one fetch_keys omap read; their waiters are queued at the front.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, osd
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.9.2 State machine (do_open_ino, src/mds/MDCache.cc:9372...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#392-state-machine-do_open_ino-srcmdsmdcachecc9372-mdcachedo_open_ino)
- **Code:** `src/mds/MDCache.cc:9562-9589 (MDCache::open_ino_batch_start)`

<a id="f-cache-050"></a>

#### F-CACHE-050 · ESTALE recovery via find_ino_peers

*Recovering when a request names a base inode this rank does not cache.*

When a traversal's base inode is not cached, the rank asks a hint rank (once) and then each active peer (CLIENTREPLAY..STOPPING) in turn with MMDSFindIno(tid, ino); a peer that caches it replies MMDSFindInoReply with its path (an auth inode not yet linked yields filepath(ino) and the peer flushes its journal); the requester discovers that path and re-runs the request. If every in-rank was checked, it fails with -ESTALE and C_MDS_TryFindInode falls back to open_ino, forwarding the request to the returned rank or re-dispatching it; -ENOENT/-ENODATA reach the client as -ESTALE. Searches wait while ranks are not active and restart on rank failure or recovery. Callers: the `rdlock_path_*` helpers, link target lookup and peer rmdir/rename prep.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.10 find_ino_peers: ESTALE recovery (src/mds/MDCache.cc:...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#310-find_ino_peers-estale-recovery-srcmdsmdcachecc9651-9802-mdcachefind_ino_peers--kick_find_ino_peers)
- **Code:** `src/mds/MDCache.cc:9651-9802 (MDCache::find_ino_peers)`, `src/mds/MDCache.cc:9710 (MDCache::handle_find_ino)`, `src/mds/MDCache.cc:9740 (MDCache::handle_find_ino_reply)`, `src/mds/Server.cc:3834 (C_MDS_TryFindInode)`
- **Notes:** A peer that has the inode but cannot provide a traversable path causes "failed ... retrying" loops until the auth's journal flush links the inode (code reading). See also F-NS-022 (request path), F-CACHE-047 (open_ino).

<a id="f-cache-051"></a>

#### F-CACHE-051 · Foreign mdsdir and stray base resolution

*Traversing paths rooted at another rank's private directory or stray directories.*

A traversal whose base is an uncached ~mdsN inode discovers it from rank ino & (MAX_MDS - 1) (open_foreign_mdsdir); a base stray inode MDS_INO_STRAY(r, i) is rewritten to the path `stray<i>` under ~mdsr and traversed with DISCOVER. open_ino_traverse_dir does the same for ~mdsN ancestors. Used when ranks operate on each other's strays (stray migration, rename/unlink witnesses).

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.5.2 Base inode](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#352-base-inode) · [mdcache-lifecycle-trim-and-memory.md › 3.10.4 Opening at boot](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#3104-opening-at-boot)
- **Code:** `src/mds/MDCache.cc:907 (MDCache::open_foreign_mdsdir)`, `src/mds/MDCache.cc:8470-8486 (MDCache::path_traverse)`

### Cache replication between ranks

Read copies (replicas) of inodes, dirfrags and dentries on non-authoritative ranks: nonces, discover, replica encodings, pushed link/unlink updates, lock-state refresh and expiry. All messages here are MDS-to-MDS.

<a id="f-cache-052"></a>

#### F-CACHE-052 · Replica map and nonces

*Tracking which ranks hold a replica of an object and detecting stale expiries.*

On the authority, replica_map maps rank -> nonce (PIN_REPLICATED while non-empty); add_replica returns nonce 1 for a new holder and increments the stored nonce for an existing one; a replica stores the nonce it received. An expiry carries the replica's nonce and is applied only if it equals the current one, otherwise it crossed a newer replication and is dropped. After migration EXPORT_NONCE = 1: the exporter becomes a replica with nonce 1 and the importer registers the old auth with nonce 1. Replica objects are never dirty, fetched or committed.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.4 Authority, replicas and nonces](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#34-authority-replicas-and-nonces) · [mdcache-subtrees-traversal-replication.md › 3.7.1 Nonces and the replica map](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#371-nonces-and-the-replica-map)
- **Code:** `src/mds/MDSCacheObject.h:215-241 (MDSCacheObject::add_replica)`, `src/mds/MDCache.cc:7685 (MDCache::handle_cache_expire)`, `src/mds/CDir.h:183 (CDir::EXPORT_NONCE)`
- **Notes:** get_replica_nonce(from) asserts the sender is in the replica map.

<a id="f-cache-053"></a>

#### F-CACHE-053 · Discover protocol (pull replication)

*Pulling replicas of inodes, dirfrags and dentries along a path from the authoritative rank.*

MDiscover (0x203; base_ino, base_dir_frag, snapid, wanted path, want_base_dir, path_locked; header tid) goes to the auth when traversal, open_ino or base/dirfrag opening needs a replica; requests are tracked by tid and deduplicated by the presence of a dir or dentry waiter. The auth walks the path adding replicas, including null dentries for missing names (after fetching the key), stops at remote links, and replies MDiscoverReply (0x204) with a trace of [dir] dentry inode replica blobs, error flags (flag_error_dn, flag_error_dir, error_dentry) and a dir_auth_hint when it is not auth. It parks the message while it must fetch or while objects are frozen or xlocked (xlocked allowed with path_locked), sends partial replies instead of waiting once it has content, and defers discovers from survivors while it is in REJOIN or earlier. No timeouts: kick_discovers re-sends tracked discovers to a rank that reaches REJOIN or restarts; messages to down ranks are dropped but stay tracked.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.6 Discover: pull replication](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#36-discover-pull-replication) · [mdcache-subtrees-traversal-replication.md › 3.6.1 Requester side](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#361-requester-side) · [mdcache-subtrees-traversal-replication.md › 3.6.2 Auth side: handle_discover (src/mds/MDCache.cc:1064...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#362-auth-side-handle_discover-srcmdsmdcachecc10649-mdcachehandle_discover) · [mdcache-subtrees-traversal-replication.md › 3.6.3 Requester side: handle_discover_reply (src/mds/MDCa...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#363-requester-side-handle_discover_reply-srcmdsmdcachecc10951-mdcachehandle_discover_reply)
- **Code:** `src/mds/MDCache.cc:10482 (MDCache::_send_discover)`, `src/mds/MDCache.cc:10649 (MDCache::handle_discover)`, `src/mds/MDCache.cc:10951 (MDCache::handle_discover_reply)`, `src/mds/MDCache.cc:10637 (MDCache::kick_discovers)`
- **Notes:** Code-reading findings: C_MDC_RetryDiscoverPath::finish binds a rank to the bool path_locked parameter; `discover_path(CInode*)` targeting self wakes existing waiters without registering its own (reachability unverified). Null dentries replicated by discover live in both caches until trimmed.

<a id="f-cache-054"></a>

#### F-CACHE-054 · Replica encodings

*The blobs that create or refresh replicas of dirfrags, dentries, inodes and stray chains.*

encode_replica_dir {dirfrag, nonce, first, fnode, dir_rep, dir_rep_by}; encode_replica_dentry {name, last, nonce, first, remote_ino, d_type, dentry lock replica state, need_recover, alternate_name}; encode_replica_inode {ino, last, nonce, base (first, inode_t, symlink, dirfragtree, xattrs, old_inodes, damage flags, snap blob), nine lock replica states + need_recover, state masked to STATE_RANDEPHEMERALPIN}; encode_replica_stray (~mdsN inode and dirfrag, strayK dentry, stray dir inode and dirfrag, stray dentry and its inode). Decoding a new object applies lock states; re-decoding an existing replica refreshes nonce and base but not lock states or dentry linkage. Carried by MDiscoverReply, MDentryLink/MDentryUnlink, rename/rmdir witness requests, export prep, MMDSFragmentNotify and rejoin. Export/import encodings are in the multi-MDS category ([subtree migration](../cpp-map/08-multi-mds/subtree-migration.md)).

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.7.2 Blobs (src/mds/MDCache.cc:11104-11345 (MDCache::enc...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#372-blobs-srcmdsmdcachecc11104-11345-mdcacheencode_replica_dir--decode_replica_stray) · [mdcache-subtrees-traversal-replication.md › 3.7.3 Every path that creates or refreshes replicas](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#373-every-path-that-creates-or-refreshes-replicas) · [cinode-and-inode-types.md › 3.17 Replication, export/import and rejoin encodings](../cpp-map/04-cache/cinode-and-inode-types.md#317-replication-exportimport-and-rejoin-encodings)
- **Code:** `src/mds/MDCache.cc:11104-11345 (MDCache::encode_replica_dir)`, `src/mds/MDCache.cc:11253 (MDCache::decode_replica_inode)`, `src/mds/CInode.cc:4505 (CInode::_encode_base)`, `src/mds/MDCache.cc:11304 (MDCache::encode_replica_stray)`
- **Notes:** The trailing stray-inode replica (v2) in rename/rmdir witness requests is encoded but skipped by the decoder. A discover reply whose linkage differs from the local dentry is only logged.

<a id="f-cache-055"></a>

#### F-CACHE-055 · Replica subtrees at delegation boundaries

*Recording on a replica rank where authority changes inside the replicated namespace.*

decode_replica_dir makes a newly replicated dirfrag a non-auth subtree root with dir_auth = sender when the sender differs from the inode's authority, the inode is ambiguous, or the inode is a base inode. Unused non-auth subtree roots are trimmed by trim().

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.7.2 Blobs (src/mds/MDCache.cc:11104-11345 (MDCache::enc...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#372-blobs-srcmdsmdcachecc11104-11345-mdcacheencode_replica_dir--decode_replica_stray) · [mdcache-subtrees-traversal-replication.md › 3.2.1 Structure](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#321-structure)
- **Code:** `src/mds/MDCache.cc:11154 (MDCache::decode_replica_dir)`

<a id="f-cache-056"></a>

#### F-CACHE-056 · Push link/unlink to replicas

*Telling ranks that hold a dentry replica about a new link or an unlink.*

After applying a new link (openc, mknod/mkdir/symlink, link, rename) the auth sends MDentryLink (0x20c; subtree, dirfrag, name, is_primary, inode replica or remote link blob) to every dentry replica holder except witnesses and ranks below REJOIN (or in REJOIN still in rejoin_gather). Unlink, rmdir and remote unlink send MDentryUnlink (0x208; dirfrag, name, stray replica chain) to replicas of the dentry and of the stray dentry; receivers relink the inode under the stray, adjust subtrees for directories and export any client caps on the replica to the auth. Correctness relies on per-peer FIFO: the message is sent before respond_to_request drops the xlock (so the LOCK_AC_SYNC follows it), and replicas never trim unreadable dentries outside stray dirs; handle_dentry_link aborts if the dirfrag or dentry is missing.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Legacy parts** (dead-code): The snaprealm branch of MDCache::handle_dentry_unlink that decodes m->snapbl. send_dentry_unlink fills MDentryUnlink::snapbl, but encode_payload and decode_payload do not carry the field, so every received message has an empty snapbl. The field was added without wire encoding by commit 853e00a64ed (2017, first tag v13.1.0). Evidence: `src/messages/MDentryUnlink.h:54-66 (MDentryUnlink::decode_payload, MDentryUnlink::encode_payload)`, `src/mds/MDCache.cc:11549-11574 (MDCache::send_dentry_unlink)`, `src/mds/MDCache.cc:11615-11617 (MDCache::handle_dentry_unlink)`.
- **Map:** [mdcache-subtrees-traversal-replication.md › 3.8 Pushing changes to replicas](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#38-pushing-changes-to-replicas) · [mdcache-subtrees-traversal-replication.md › 3.8.1 MDentryLink (src/mds/MDCache.cc:11469 (MDCache::sen...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#381-mdentrylink-srcmdsmdcachecc11469-mdcachesend_dentry_link-srcmdsmdcachecc11500-mdcachehandle_dentry_link) · [mdcache-subtrees-traversal-replication.md › 3.8.2 MDentryUnlink (src/mds/MDCache.cc:11543 (MDCache::s...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#382-mdentryunlink-srcmdsmdcachecc11543-mdcachesend_dentry_unlink-srcmdsmdcachecc11580-mdcachehandle_dentry_unlink)
- **Code:** `src/mds/MDCache.cc:11469 (MDCache::send_dentry_link)`, `src/mds/MDCache.cc:11500 (MDCache::handle_dentry_link)`, `src/mds/MDCache.cc:11543 (MDCache::send_dentry_unlink)`, `src/mds/MDCache.cc:11580 (MDCache::handle_dentry_unlink)`
- **Notes:** MDentryUnlink::encode_payload does not encode snapbl, so the receiver's snaprealm branch is dead code (field added without encoding by commit 853e00a64ed, 2017). See also F-BG-005 (stray dentry replication).

<a id="f-cache-057"></a>

#### F-CACHE-057 · Replica refresh through lock-state payloads

*Which inode and dentry fields replicas receive with lock state changes after creation.*

After creation replicas get lock state and lock-protected fields only through MLock messages, sent to every replica_map holder (skipping holders below REJOIN while degraded). encode_lock_state ships per-lock subsets: IAUTH {version, ctime, mode, uid, gid, fscrypt_auth}; ILINK {version, ctime, nlink}; IDFT {version, dirfragtree, sender's auth frags}; IFILE {version, ctime, mtime, atime, time_warp_seq, [size, layout, `truncate_*`, client_ranges, inline_data], dirstat, per-frag fragstat, fscrypt_file}; INEST {version, rstat, per-frag rstat}; IXATTR {version, ctime, xattrs, xattr_version}; ISNAP {version, ctime, snap blob, oldest_snap}; IFLOCK; IPOLICY {layout, quota, pins, flags, optmetadata}; scatter locks also carry replica dirfrag stats back to the auth. CDentry lock state carries first and the linkage kind. Not carried by any payload: btime, change_attr, rdev, dir_layout, old_pools, max_size_ever, truncate_from/pending, backtrace_version, stray_prior_path, scrub stamps, fscrypt_last_block. The lock state machines are in the caps category ([lock types and state machines](../cpp-map/05-locking/lock-types-and-state-machines.md)).

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [cinode-and-inode-types.md › 3.13 Lock-state payloads (what replicas receive)](../cpp-map/04-cache/cinode-and-inode-types.md#313-lock-state-payloads-what-replicas-receive) · [mdcache-subtrees-traversal-replication.md › 3.7.2 Blobs (src/mds/MDCache.cc:11104-11345 (MDCache::enc...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#372-blobs-srcmdsmdcachecc11104-11345-mdcacheencode_replica_dir--decode_replica_stray)
- **Code:** `src/mds/CInode.cc:2315 (CInode::encode_lock_state)`, `src/mds/CInode.cc:2367 (CInode::decode_lock_state)`, `src/mds/Locker.cc:145 (Locker::send_lock_message)`, `src/mds/CDentry.cc:526-588 (CDentry::encode_lock_state)`

<a id="f-cache-058"></a>

#### F-CACHE-058 · Replica expiry (MCacheExpire)

*Telling the authority that a replica was dropped so it can release its replication pin.*

When a replica trims a dentry, inode or dirfrag it adds (container subtree-root dirfrag, object id, replica nonce) to an MCacheExpire (0x207) for the authority, and for both auth.first and auth.second while ambiguous; one message per peer is sent at the end of the trim pass. While degraded, expiries to ranks before REJOIN (or in REJOIN before we sent our rejoin) are dropped. The auth ignores the whole message before REJOIN, delays realms whose container is being exported or not auth (replayed or discarded at the export/import outcome), and removes the replica only on nonce match, dropping lock replica state and the peer's wanted caps and re-evaluating lock gathers and strays.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.6 Replica expiry protocol (MCacheExpire)](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#36-replica-expiry-protocol-mcacheexpire) · [mdcache-lifecycle-trim-and-memory.md › 5.1 Messages](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#51-messages)
- **Code:** `src/mds/MDCache.cc:7155 (MDCache::send_expire_messages)`, `src/mds/MDCache.cc:7685 (MDCache::handle_cache_expire)`, `src/messages/MCacheExpire.h:32-63 (MCacheExpire::realm)`, `src/mds/MDCache.cc:7895 (MDCache::inode_remove_replica)`
- **Notes:** An expiry for an inode or dentry the auth lacks or is not auth for, or from a rank not in the replica map, aborts the auth MDS (asserts). See also F-MULTI-008 (cache-expire routing in migration).

### Residency, trimming and memory limit

What stays in memory and for how long: the dentry LRUs, the reasons objects cannot be evicted, the memory target, the trim pass and its throttle, the upkeep thread, and the special trimming done for multi-rank, recovery and standby-replay. Cap recall and lease expiry are in the caps category.

<a id="f-cache-059"></a>

#### F-CACHE-059 · Dentry LRUs (lru and bottom_lru)

*The two least-recently-used lists from which unreferenced dentries are evicted.*

Only CDentry objects are LRU members; inodes and dirfrags leave through their dentries. lru (midpoint mds_cache_mid = 0.7) holds auth and remote dentries: new dentries enter at the midpoint, auth dentries touched by traversal or readdir move to the top, replica dentries are mid-touched. bottom_lru (midpoint 0) holds replica null dentries and replica primary dentries in stray dirs and is drained on every trim regardless of memory pressure (up to the throttle). Each LRU has three intrusive lists (top, bottom, pintail); adjust keeps top = midpoint x (size - pinned); expiry scans the bottom tail then the top tail and moves pinned items to pintail. Placement is re-evaluated on link/unlink and auth changes (STATE_BOTTOMLRU); new stray dentries and exported dentries are moved next to expire.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_cache_mid (0.7)
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.2 LRU mechanics (src/include/lru.h)](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#32-lru-mechanics-srcincludelruh) · [cdir-cdentry-and-cache-objects.md › 3.15 LRU membership](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#315-lru-membership)
- **Code:** `src/include/lru.h:158 (LRU::lru_get_next_expire)`, `src/mds/MDCache.h:919 (MDCache::touch_dentry)`, `src/mds/CDir.cc:352 (CDir::adjust_dentry_lru)`, `src/mds/MDCache.cc:206 (MDCache::MDCache)`

<a id="f-cache-060"></a>

#### F-CACHE-060 · Pin reasons that block eviction

*Why cached metadata cannot be trimmed, with client caps as the dominant reason.*

A dentry or inode is not trimmable while a client holds a cap on the inode (PIN_CAPS), while it is dirty (PIN_DIRTY until written back, which journal segment expiry forces), while a client dentry lease is outstanding (PIN_CLIENTLEASE), while replicated to another rank (PIN_REPLICATED on the auth), while locked, referenced by a request, waited on, auth-pinned, queued for scrub, purging, truncating or awaiting snapflush, or while any descendant is cached. The MDS therefore shrinks only as clients release caps after CEPH_SESSION_RECALL_STATE ([sessions](../cpp-map/02-client-protocol/sessions.md)): the kernel prunes dentry aliases so the VFS evicts inodes, libcephfs trims its cache, and NFS-Ganesha drops handles through ino_release_cb upcalls, so Ganesha's handle cache limits how far the MDS can trim.

- **Class:** POSIX `operational` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.1 Object graph and residency ("what is pinned and why")](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#31-object-graph-and-residency-what-is-pinned-and-why) · [mdcache-lifecycle-trim-and-memory.md › 3.8 Cache pressure and client recall](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#38-cache-pressure-and-client-recall) · [mdcache-lifecycle-trim-and-memory.md › 7. Externally visible contract](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSCacheObject.h:58-69 (MDSCacheObject::PIN_REPLICATED)`, `src/mds/CInode.cc:3595 (CInode::add_client_cap)`, `src/client/Client.cc:5136 (Client::_schedule_ino_release_callback)`, `linux/fs/ceph/mds_client.c:2263 (trim_caps_cb)`

<a id="f-cache-061"></a>

#### F-CACHE-061 · Memory-based soft cache limit

*The cache size target, measured on the MDS cache memory pool, that drives trimming and the oversized-cache warning.*

cache_size() is the bytes allocated in mempool mds_co (CInode, CDir, CDentry, Capability, ClientLease and scrub info objects plus their mds_co containers: inode_t copies, xattr maps, dentry maps, ref maps, waiter maps); it is read without locks. cache_toofull when size > L x (1 - R) (L = mds_cache_memory_limit, 4 GiB; R = mds_cache_reservation, 0.05): LRU trimming continues past bottom_lru and cap recall targets every session. cache_overfull when size > L x mds_health_cache_threshold (1.5) raises MDS_CACHE_OVERSIZED ("MDS cache is too large (size/limit); N inodes in use by clients, M stray files"). Not counted: xattr value buffers, bloom filters, the inode_map and subtree containers, sessions, requests, messages, journal buffers and lock side allocations, so RSS can exceed the limit by a large factor.

- **Class:** POSIX `operational` · exposure `config`, `metrics` · relied on by admin, mgr, mon, cephadm
- **Gates:** mds_cache_memory_limit (4 GiB); mds_cache_reservation (0.05); mds_health_cache_threshold (1.5)
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.3 Memory accounting](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#33-memory-accounting) · [cdir-cdentry-and-cache-objects.md › 3.17 Memory accounting](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#317-memory-accounting) · [mdcache-lifecycle-trim-and-memory.md › 5.6 Health](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#56-health)
- **Code:** `src/mds/MDCache.h:236-251 (MDCache::cache_toofull_ratio)`, `src/common/mempool.cc:124 (mempool::pool_t::allocated_bytes)`, `src/mds/Beacon.cc:513 (Beacon::notify_health)`, `src/pybind/mgr/cephadm/autotune.py:39-41 (MemoryAutotuner.tune)`
- **Notes:** cephadm's memory autotuner subtracts each MDS's mds_cache_memory_limit config value from host memory. The mon refuses `mds fail` / `fs fail` with EPERM without --yes-i-really-mean-it while an MDS reports MDS_CACHE_OVERSIZED (health and perf counters are catalogued in the admin category, [config, perf counters and health](../cpp-map/13-admin-and-config/config-perf-counters-and-health.md)). See also F-ADM-079 (MDS_CACHE_OVERSIZED).

<a id="f-cache-062"></a>

#### F-CACHE-062 · Trim throttle

*A decay-counter limit on how many dentries the LRU trim may remove per unit time.*

trim_lru stops once trim_counter (a DecayCounter whose argument mds_cache_trim_decay_rate = 1 is a half-life in seconds) plus the dentries trimmed in the current pass reaches mds_cache_trim_threshold (262144); each throttled pass increments mds_cache.cache_trim_throttle. Only LRU expirations count. With 1 s passes the sustained rate is about 131072 dentries/s (the documentation's continuous-time figure is about 181,700/s), with a burst of up to 256Ki from a cold counter. Changing the decay rate at runtime recreates the counter (resets it to 0).

- **Class:** POSIX `operational` · exposure `config`, `metrics` · relied on by admin
- **Gates:** mds_cache_trim_threshold (262144); mds_cache_trim_decay_rate (1)
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.5.1 trim_lru and the trim throttle (src/mds/MDCache.cc:...](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#351-trim_lru-and-the-trim-throttle-srcmdsmdcachecc6933-mdcachetrim_lru) · [mdcache-lifecycle-trim-and-memory.md › 3.13 Configuration changes and MDSMap updates](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#313-configuration-changes-and-mdsmap-updates)
- **Code:** `src/mds/MDCache.cc:6933 (MDCache::trim_lru)`, `src/common/DecayCounter.cc:71 (DecayCounter::decay)`, `src/mds/MDCache.cc:224 (MDCache::handle_conf_change)`

<a id="f-cache-063"></a>

#### F-CACHE-063 · Trim pass and per-object removal

*The periodic pass that evicts unreferenced dentries, inodes and dirfrags.*

trim(count) advances delayed stray evaluations, runs trim_lru (bottom_lru fully; lru while too full or until count dentries are trimmed), scans the subtree map (empty-subtree handling), runs the stopping-peer cleanups and sends the accumulated cache expiries. trim_dentry keeps (reinserts at the midpoint) a non-auth dentry whose lock is unreadable outside stray dirs (an MDentryLink/Unlink is probably in flight), clears completeness and feeds the bloom filter when removing a non-clean-null auth dentry, queues expiries for replicas and counts mds.inodes_expired. trim_inode keeps a replica directory inode whose dirfragtreelock is not readable (a fragment notify is pending) and an auth inode that maybe_eval_stray started purging; otherwise it trims the dirfrags and removes the inode. trim_dirfrag closes a dirfrag (a subtree root only if non-auth, or auth, unreplicated and base). Removal outside the LRU: readdir deletes unreferenced null dentries, close_dirfrag drops nulls, recovery code removes objects directly. Callers: upkeep thread (count 0), rank stopping and cache drop (all), export finish and import reverse, standby-replay segment trim, empty-journal replay.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.5 trim(count) — the trimming pass (src/mds/MDCache.cc:7...](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#35-trimcount--the-trimming-pass-srcmdsmdcachecc7025-mdcachetrim) · [mdcache-lifecycle-trim-and-memory.md › 3.5.2 trim_dentry, trim_inode, trim_dirfrag](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#352-trim_dentry-trim_inode-trim_dirfrag) · [mdcache-lifecycle-trim-and-memory.md › 3.5.4 Callers of trim](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#354-callers-of-trim)
- **Code:** `src/mds/MDCache.cc:7025 (MDCache::trim)`, `src/mds/MDCache.cc:7172 (MDCache::trim_dentry)`, `src/mds/MDCache.cc:7307 (MDCache::trim_inode)`, `src/mds/MDCache.cc:7256 (MDCache::trim_dirfrag)`
- **Notes:** A pass is one mds_lock hold of up to mds_cache_trim_threshold removals plus a full subtree-map scan, once per second.

<a id="f-cache-064"></a>

#### F-CACHE-064 · Upkeep thread

*The background thread that samples memory, expires leases, trims and drives cap recall every second.*

Thread mds-cache-trim, started by the MDCache constructor. Every 0.9 x mds_cache_trim_interval (1 s) it takes mds_lock and runs check_memory_usage (samples /proc for mds_mem.rss in kB and mds_mem.heap in bytes, refreshes object counts, asserts the inode-count invariant) in any state; when the rank is in standby-replay, clientreplay, active or stopping it also expires client dentry leases (ranks with clients), calls trim() when the base hierarchy is open and rejoin is done (or in standby-replay), and calls Server::recall_client_state(ENFORCE_MAX | ENFORCE_LIVENESS, plus TRIM when the cache is too full). There is no wake-up on allocation pressure, so reaction latency is up to one interval; intervals are re-read on every iteration. It keeps running while the beacon is laggy (the tick does not).

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_cache_trim_interval (1 s)
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.4 The upkeep thread mds-cache-trim](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#34-the-upkeep-thread-mds-cache-trim) · [mdcache-lifecycle-trim-and-memory.md › 6. Concurrency and mds_lock](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/MDCache.cc:14696 (MDCache::upkeep_main)`, `src/mds/MDCache.cc:7959 (MDCache::check_memory_usage)`, `src/mds/MDCache.cc:14689 (MDCache::is_ready_to_trim_cache)`

<a id="f-cache-065"></a>

#### F-CACHE-065 · Heap release to the OS

*Periodically returning freed allocator memory to the operating system.*

Every mds_cache_release_free_interval (10 s, dev option) the upkeep thread calls ceph_heap_release_free_memory (tcmalloc ReleaseFreeMemory; a no-op without tcmalloc) without holding mds_lock.

- **Class:** POSIX `operational` · exposure `config` · relied on by internal
- **Gates:** mds_cache_release_free_interval (10 s)
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.4 The upkeep thread mds-cache-trim](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#34-the-upkeep-thread-mds-cache-trim)
- **Code:** `src/mds/MDCache.cc:14721-14768 (MDCache::upkeep_main)`

<a id="f-cache-066"></a>

#### F-CACHE-066 · Empty-subtree trimming and empty-import return

*Dropping or handing back subtrees that no longer hold anything.*

During trim() the subtree scan removes an auth, non-base, empty (no head items) AUXSUBTREE dirfrag whose inode is auth and that only the subtree pin holds ("trimming empty pinned subtree"); hands an empty auth dirfrag whose inode is non-auth back to the inode's auth with Migrator::export_empty_import (rank active or stopping, not freezing or exporting); and trims an unused non-auth subtree root whose inode is non-auth unless its auth is still in rejoin_ack_gather.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.5.3 Subtree and base-inode trimming inside trim() (src/...](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#353-subtree-and-base-inode-trimming-inside-trim-srcmdsmdcachecc7042-7147-mdcachetrim)
- **Code:** `src/mds/MDCache.cc:7042-7147 (MDCache::trim)`, `src/mds/Migrator.cc:231 (Migrator::export_empty_import)`
- **Notes:** See also F-MULTI-009 (empty import return).

<a id="f-cache-067"></a>

#### F-CACHE-067 · Expiry of a stopping peer's mdsdir replicas

*Releasing replicas of another rank's private directory so that rank can finish stopping.*

For every other rank in up:stopping whose ~mdsN inode is cached, trim() runs expire_recursive: it trims all replica dentries below it, aborting on a nested subtree root, a non-stray inode with nlink > 0, or a pinned dentry; if nothing aborted it trims the mdsdir dirfrags and inode, so the stopping rank's PIN_REPLICATED pins drop and its cache can empty. A rank that is itself stopping also trims other ranks' unreferenced mdsdir base inodes.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.5.3 Subtree and base-inode trimming inside trim() (src/...](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#353-subtree-and-base-inode-trimming-inside-trim-srcmdsmdcachecc7042-7147-mdcachetrim)
- **Code:** `src/mds/MDCache.cc:3829 (MDCache::expire_recursive)`, `src/mds/MDCache.cc:7098-7130 (MDCache::trim)`

<a id="f-cache-068"></a>

#### F-CACHE-068 · Recovery-time non-auth trimming

*Discarding non-authoritative cache content whose accuracy cannot be guaranteed after a failure.*

trim_non_auth (during rejoin, at the end of process_imported_caps, before the rank sends its own rejoin) removes all non-auth dentries and inodes except the linkage to subtree roots, marks their dirfrags incomplete, and removes leftover non-auth base inodes when both LRUs are empty (non-auth linkage may be inaccurate). trim_non_auth_subtree and try_trim_non_auth_subtree remove subtrees found not to be ours during replay, resolve and rename, keeping null dentries of ambiguous imports or uncommitted peer-rename sources for rollback. trim_unlinked_inodes removes non-base inodes without a parent dentry when resolve finishes. Recovery protocol: [resolve and rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md).

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.9 Non-auth trimming for recovery and standby-replay](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#39-non-auth-trimming-for-recovery-and-standby-replay)
- **Code:** `src/mds/MDCache.cc:7401 (MDCache::trim_non_auth)`, `src/mds/MDCache.cc:7515 (MDCache::trim_non_auth_subtree)`, `src/mds/MDCache.cc:7582 (MDCache::try_trim_non_auth_subtree)`, `src/mds/MDCache.cc:3875 (MDCache::trim_unlinked_inodes)`

<a id="f-cache-069"></a>

#### F-CACHE-069 · Standby-replay cache maintenance

*Keeping a standby-replay daemon's cache bounded while it follows the active rank's journal.*

For each segment the active rank has expired, standby_trim_segment marks every dirty dirfrag, inode, dentry, dirty-parent and scatter-dirty object of that segment clean and drops PIN_TRUNCATING, so trim() (called from MDLog::standby_trim_segments and the upkeep thread) can evict them; while in standby-replay trim_lru keeps dentries whose inode is still on a segment's open-file list. The bloom filter is never created in standby-replay. Standby-replay itself is in the cluster category ([rank lifecycle and recovery](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md)).

- **Class:** POSIX `operational` · exposure `internal` · relied on by internal
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.9 Non-auth trimming for recovery and standby-replay](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#39-non-auth-trimming-for-recovery-and-standby-replay)
- **Code:** `src/mds/MDCache.cc:7644 (MDCache::standby_trim_segment)`, `src/mds/MDLog.cc:1615-1650 (MDLog::standby_trim_segments)`, `src/mds/MDCache.cc:6997-7000 (MDCache::trim_lru)`
- **Notes:** See also F-CLU-068.

### Base hierarchy

Creation at mkfs and boot-time opening of the root, the per-rank private directory and the global snaprealm inode. Stray directory policy is in the background category; rank stopping in the cluster category.

<a id="f-cache-070"></a>

#### F-CACHE-070 · mkfs base hierarchy creation

*Cache view of the base hierarchy created in up:creating.*

See F-CLU-064. Cache-side facts: create_empty_hierarchy runs only on the MDSMap root rank (always 0) and builds the root inode and an empty, complete root dirfrag that is a subtree root with dir_rep REP_ALL; every rank runs create_mydir_hierarchy for ~mdsR and its ten stray directories. The objects are written dirty in the first journal segment and committed before up:active is requested.

- **Class:** POSIX `operational` · exposure `on-disk` · relied on by mon, tools
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.10.3 mkfs (up:creating)](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#3103-mkfs-upcreating) · [mdcache-lifecycle-trim-and-memory.md › 5.3 RADOS objects touched here (metadata pool unless noted)](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#53-rados-objects-touched-here-metadata-pool-unless-noted)
- **Code:** `src/mds/MDSRank.cc:2189 (MDSRank::boot_create)`, `src/mds/MDCache.cc:498 (MDCache::create_empty_hierarchy)`, `src/mds/MDCache.cc:525 (MDCache::create_mydir_hierarchy)`

<a id="f-cache-071"></a>

#### F-CACHE-071 · Root and system inode initial attributes and default layouts

*The attributes the root directory and system inodes start with.*

create_unlinked_system_inode sets version 1, xattr_version 1, mode 0500 | type, size 0, ctime = mtime = btime = now, nlink 1, truncate_size -1, change_attr 0, export_pin none; directories get dl_dir_hash = mds_default_dir_hash and rsubdirs 1; base inodes get inode_auth and an own empty snaprealm (seq 1). The root is S_IFDIR | 0755 with uid/gid mds_root_ino_uid / mds_root_ino_gid (0/0) and layout = default file layout (file_layout_t::get_default() on the MDSMap's first data pool); mdsdir and stray dirs are S_IFDIR | 0500. The default log layout uses the metadata pool and, when mds_log_segment_size > 0, that object size and stripe unit.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_root_ino_uid (0); mds_root_ino_gid (0); mds_default_dir_hash (2)
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.10.2 In-memory construction](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#3102-in-memory-construction) · [mdcache-lifecycle-trim-and-memory.md › 7. Externally visible contract](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#7-externally-visible-contract)
- **Code:** `src/mds/MDCache.cc:441 (MDCache::create_unlinked_system_inode)`, `src/mds/MDCache.cc:487 (MDCache::create_root_inode)`, `src/mds/MDCache.cc:417-439 (MDCache::gen_default_file_layout)`

<a id="f-cache-072"></a>

#### F-CACHE-072 · Global snaprealm dummy inode

*An in-memory inode number 3 that carries the file system's global snaprealm.*

Every boot creates an unlinked, never-stored base inode CEPH_INO_GLOBAL_SNAPREALM (3) whose snaprealm is the global snaprealm; it is removed at the end of shutdown_pass. Being is_base() it gets the meaningless inode_auth rank 3 - 0x100, which no reviewed code path consults. Clients see ino 3 as a snaprealm ino: libcephfs keeps that realm pinned until unmount and the kernel keeps it for the mount lifetime. Snapshot semantics: [snapshots](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.10.2 In-memory construction](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#3102-in-memory-construction) · [cinode-and-inode-types.md › 7. Externally visible contract](../cpp-map/04-cache/cinode-and-inode-types.md#7-externally-visible-contract)
- **Code:** `src/mds/MDCache.cc:10217 (MDCache::create_global_snaprealm)`, `src/client/Client.cc:5439 (Client::get_snap_realm)`, `linux/fs/ceph/snap.c:126 (ceph_create_snap_realm)`

<a id="f-cache-073"></a>

#### F-CACHE-073 · Boot-time opening of the base hierarchy and the open gate

*Loading the root and private directories at rank start and holding requests until they are usable.*

boot_start step OPEN_ROOT fetches ~mdsR (and on the root rank the root inode; a replaying non-root rank creates a placeholder root) and creates the global snaprealm inode. open_root (after creating/starting) makes the root dirfrag available (rank 0: complete subtree root, fetched if needed; others: discover ino 1 from rank 0 and open the root dirfrag replica) and the mdsdir; recovery_done (after replay) or import of the own mdsdir calls populate_mydir: fetch mydir; recreate a BADFRAG version-0 mydir (cluster log warning); accept legacy "stray"; recreate a missing `stray<i>` through _create_system_file (journaled EUpdate "create system file" with forced filelock/nestlock wrlocks) and retry; pin each stray once (PIN_STRAY, sticky dirfrags); read unloaded stray frag headers; then set open = true, release waiting_for_open (client requests, export discover and balancer heartbeats wait on it), start the StrayManager and scan_stray_dir. Errors inside open_root/populate_mydir mark the rank damaged; boot_start classifies gather errors (-EINVAL/-ENOENT damaged, -EROFS read-only, others suicide).

- **Class:** POSIX `operational` · exposure `on-disk`, `metrics` · relied on by mon, internal
- **Legacy parts** (old-data): Accepting a dentry named "stray" in place of "stray0" for stray index 0 in populate_mydir. The fallback serves file systems created before stray directories were numbered (commit fc212548aea, 2010, first tag v0.24). create_mydir_hierarchy at this commit always creates stray0..stray9. The BADFRAG mydir recreation and the recreation of missing stray<i> dentries are current repair paths. Evidence: `src/mds/MDCache.cc:853-855 (MDCache::populate_mydir)`, `src/mds/MDCache.cc:538-543 (MDCache::create_mydir_hierarchy)`.
- **Map:** [mdcache-lifecycle-trim-and-memory.md › 3.10.4 Opening at boot](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#3104-opening-at-boot) · [mdcache-lifecycle-trim-and-memory.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDSRank.cc:1684-1705 (MDSRank::boot_start)`, `src/mds/MDCache.cc:750 (MDCache::open_root)`, `src/mds/MDCache.cc:821 (MDCache::populate_mydir)`, `src/mds/MDCache.cc:588-664 (MDCache::_create_system_file)`, `src/mds/MDCache.cc:668-683 (C_MDS_RetryOpenRoot)`

### Fragment identity, name hashing and client exposure

How directories are partitioned into fragments by name hash, the wire formats clients share with the MDS, the order and resumption of readdir across fragments, delegation hints and fragment size limits.

<a id="f-cache-074"></a>

#### F-CACHE-074 · Fragment identifier and fragment tree (frag_t, fragtree_t)

*The 32-bit fragment id and the split-point tree that partition a directory's 24-bit hash space.*

frag_t is a u32: upper 8 bits = depth b (at most 24), lower 24 bits = value using the most significant b bits of the 24-bit space (ceph_frag_make(b, v) = (b << 24) | (v & (0xffffff << (24 - b)) & 0xffffff)); a hash h is in f iff (h & mask(f)) == value(f), so only the low 24 bits of a 32-bit name hash matter; frag_t() is the whole space, printed `"*"`, others as their value bits in binary plus `"*"`; next() walks left to right. fragtree_t stores only split points (frag -> bits; leaves implicit); operator[] maps a hash to its leaf; split, merge, force_to_leaf and try_assimilate_children keep a canonical form. Wire/disk encoding: u32 count + count x (u32 frag, i32 bits), decoded by the kernel as ceph_frag_tree_head. The tree lives unprojected in CInode::dirfragtree, is stored and journaled with the directory inode, and open dirfrags are always its leaves. Each dirfrag is the RADOS object `<ino hex>.<frag 8 hex>` in the metadata pool.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-data): The byte-swap correction in frag_t::decode for invalid frag_t values whose byte-reversed value is valid (tracker 73792). Byte-swapped frag_t values were written by Ceph C++ code on big-endian hosts before commit 9e3837c837b (first in v21.0.0, backported to 19.2.7 and 20.2.1; v20.2.0 still used encode_raw). frag_t::encode at this commit writes little-endian on every host, and the kernel sends frags as __le32. Evidence: `src/common/frag.cc:39 (frag_t::encode)`, `src/common/frag.cc:42-61 (frag_t::decode)`, `v20.2.0:src/include/frag.h:158 (frag_t::encode)`.
- **Map:** [dirfrags-and-fragmentation.md › 3.1 Fragment identifiers (frag_t)](../cpp-map/04-cache/dirfrags-and-fragmentation.md#31-fragment-identifiers-frag_t) · [dirfrags-and-fragmentation.md › 3.2 Fragment trees (fragtree_t) and fragset_t](../cpp-map/04-cache/dirfrags-and-fragmentation.md#32-fragment-trees-fragtree_t-and-fragset_t) · [dirfrags-and-fragmentation.md › 3.4 Where the fragtree lives and how it stays consistent](../cpp-map/04-cache/dirfrags-and-fragmentation.md#34-where-the-fragtree-lives-and-how-it-stays-consistent) · [cdir-cdentry-and-cache-objects.md › 3.16 Fragments: frag_t and fragtree_t basics](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#316-fragments-frag_t-and-fragtree_t-basics)
- **Code:** `src/include/ceph_frag.h:23 (ceph_frag_make)`, `src/include/frag.h:186 (fragtree_t)`, `src/common/frag.cc:88 (fragtree_t::force_to_leaf)`, `linux/include/linux/ceph/ceph_fs.h:554-562 (ceph_frag_tree_head)`
- **Notes:** frag_t::decode silently byte-swaps an invalid encoding when the swapped value is valid (tracker 73792); client request heads (args.readdir.frag) are raw __le32 and bypass that validation. ceph_frag_is_right_child is only true for 24-bit frags; the frag.h comment about bit order contradicts the actual MSB-first encoding.

<a id="f-cache-075"></a>

#### F-CACHE-075 · Dentry name hash per directory (dl_dir_hash)

*The per-directory hash function that maps names to fragments, shared by MDS and clients.*

Each directory inode stores dir_layout.dl_dir_hash, set from mds_default_dir_hash (default 2 = CEPH_STR_HASH_RJENKINS) at creation and never changed; inodes decoded from inode_t encodings older than v4 get 0. hash_dentry_name = ceph_str_hash(dl_dir_hash, or CEPH_STR_HASH_LINUX (1) if 0, name) and asserts the type is 1 or 2; pick_dirfrag(name) returns frag_t() without hashing when the tree is empty, else dirfragtree[hash]. Every name-based lookup uses it (traversal, discover, stray placement, client lease messages, split repartitioning). Clients hash with the same function (from InodeStat dir_layout) to choose a fragment and rank and to compute readdir positions; for rjenkins the kernel computes the identical value.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-data): inode_t::decode zeroing dir_layout (dl_dir_hash 0) for encodings older than struct_v 4. Only inode_t encodings older than struct_v 4, written by C++ releases before dir_layout was added (commit e146767e5df, 2010, first tag v0.24), decode to dl_dir_hash 0. The encoder at this commit writes struct_v 20 including dir_layout. The mapping of 0 to CEPH_STR_HASH_LINUX stays reachable on current file systems because new directories copy mds_default_dir_hash, which has no lower bound. Evidence: `src/include/cephfs/types.h:1139-1144 (inode_t::decode)`, `src/include/cephfs/types.h:1055 (inode_t::encode)`, `src/mds/CInode.cc:720-726 (InodeStoreBase::hash_dentry_name)`, `src/mds/Server.cc:3707 (Server::prepare_new_inode)`, `src/common/options/mds.yaml.in:566-575 (mds_default_dir_hash)`.
- **Gates:** mds_default_dir_hash (2)
- **Map:** [dirfrags-and-fragmentation.md › 3.3 Mapping dentry names to fragments](../cpp-map/04-cache/dirfrags-and-fragmentation.md#33-mapping-dentry-names-to-fragments) · [cinode-and-inode-types.md › 7. Externally visible contract](../cpp-map/04-cache/cinode-and-inode-types.md#7-externally-visible-contract)
- **Code:** `src/mds/CInode.cc:720 (InodeStoreBase::hash_dentry_name)`, `src/mds/CInode.cc:729 (InodeStoreBase::pick_dirfrag)`, `src/common/ceph_hash.cc:23 (ceph_str_hash_rjenkins)`, `linux/fs/ceph/dir.c:2215 (ceph_dentry_hash)`
- **Notes:** For dl_dir_hash 0 or 1 the kernel routes with the VFS dcache hash, a different function from ceph_str_hash_linux, so its frag choice is only a hint the MDS forwards; the same hash goes into snapshot NFS handles, so LOOKUPINO may probe the wrong frag and return ESTALE for such directories (code reading). Rejoin placeholder inodes take the current option value and opened_undef_inode asserts the loaded value equals it, so changing mds_default_dir_hash while directories with another hash exist can abort a recovering rank.

<a id="f-cache-076"></a>

#### F-CACHE-076 · Fragtree and dir_layout in InodeStat

*How clients learn a directory's current fragmentation and hash function.*

Every directory InodeStat carries the current in-memory dirfragtree and dir_layout (the legacy encoding sends dir_layout only with CEPH_FEATURE_DIRLAYOUTHASH). No message pushes fragtree changes to clients: libcephfs replaces its tree whenever the InodeStat differs; the kernel refreshes it in ceph_fill_fragtree, rejecting splits with by == 0 or bits + by > 24 and detecting changes by sampling one random split when the count is unchanged. Because the tree is not projected, a reply can carry a tree whose fragment PREPARE is not yet durable; clients treat it as a routing hint.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The legacy (no CephFS bit 9) InodeStat encoding, in which dir_layout is sent only to connections with CEPH_FEATURE_DIRLAYOUTHASH. The legacy InodeStat goes only to sessions without CephFS bit 9 (kernels up to v5.0, Ceph 13.2.x and older). The 7.3 kernel advertises bit 9 and DIRLAYOUTHASH, and current libcephfs advertises every bit, so both receive the versioned InodeStat, which always carries dirfragtree and dir_layout. Evidence: `src/mds/CInode.cc:4315 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4352 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4407-4410 (CInode::encode_inodestat)`, `linux/fs/ceph/mds_client.h:48 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/include/linux/ceph/ceph_features.h:186 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`.
- **Gates:** feature CEPH_FEATURE_DIRLAYOUTHASH (legacy encoding)
- **Map:** [dirfrags-and-fragmentation.md › 3.13 Client-visible behavior](../cpp-map/04-cache/dirfrags-and-fragmentation.md#313-client-visible-behavior) · [cinode-and-inode-types.md › 3.16 Encoding inode state to clients](../cpp-map/04-cache/cinode-and-inode-types.md#316-encoding-inode-state-to-clients)
- **Code:** `src/mds/CInode.cc:4350-4352 (CInode::encode_inodestat)`, `linux/fs/ceph/inode.c:509 (ceph_fill_fragtree)`, `src/client/Client.cc:1213-1216 (Client::add_update_inode)`

<a id="f-cache-077"></a>

#### F-CACHE-077 · Hash-ordered dentry storage and readdir order

*Dentries inside a dirfrag are ordered by name hash, and readdir returns them in that order.*

CDir::items is keyed by dentry_key_t ordered by ceph_frag_value(hash) (24 bits), then name (byte compare), then snapid; readdir emits entries in this order from lower_bound of the resume key, skipping keys <= (offset_hash, offset name). With CEPH_READDIR_REPLY_BITFLAGS the MDS sets CEPH_READDIR_HASH_ORDER | CEPH_READDIR_OFFSET_HASH; the kernel recomputes each entry's hash to build f_pos (HASH_ORDER flag | hash24 << 28 | collision index), numbering same-hash entries in arrival order, so entries must arrive in non-decreasing 24-bit hash order (an entry below the current position makes ceph_readdir return EIO) and the name order within one hash keeps path2 resumption exact. On-disk omap keys (name_head, `name_<snap hex>`) are lexicographic, independent of this order. Names are compared byte-wise; the MDS performs no case folding or normalization.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Readdir requests without CEPH_READDIR_REPLY_BITFLAGS: the MDS picks the frag without offset_hash, clears offset_str and sets neither CEPH_READDIR_HASH_ORDER nor CEPH_READDIR_OFFSET_HASH in the reply. Only kernels before v4.7 (kernel commit 956d39d631db) send readdir requests without CEPH_READDIR_REPLY_BITFLAGS. The 7.3 kernel sets the flag on every readdir and snapdir-listing request, and current libcephfs sets it on every readdir. Evidence: `src/mds/Server.cc:5150-5160 (Server::handle_client_readdir)`, `src/mds/Server.cc:5325-5327 (Server::handle_client_readdir)`, `linux/fs/ceph/dir.c:450-451 (ceph_readdir)`, `linux/fs/ceph/export.c:502-503 (__get_snap_name)`, `src/client/Client.cc:10116 (Client::readdir_r_cb)`.
- **Map:** [dirfrags-and-fragmentation.md › 3.3 Mapping dentry names to fragments](../cpp-map/04-cache/dirfrags-and-fragmentation.md#33-mapping-dentry-names-to-fragments) · [dirfrags-and-fragmentation.md › 3.13 Client-visible behavior](../cpp-map/04-cache/dirfrags-and-fragmentation.md#313-client-visible-behavior) · [cdir-cdentry-and-cache-objects.md › 7. Externally visible contract](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#7-externally-visible-contract)
- **Code:** `src/mds/mdstypes.h:410 (operator<)`, `src/mds/Server.cc:5225-5228 (Server::handle_client_readdir)`, `linux/fs/ceph/dir.c:545-551 (ceph_readdir)`, `linux/fs/ceph/inode.c:2001 (ceph_readdir_prepopulate)`
- **Notes:** See also F-NS-032, F-NS-033.

<a id="f-cache-078"></a>

#### F-CACHE-078 · Readdir fragment mapping and resume across refragmentation

*Fragment-tree side of readdir resume across splits and merges.*

See F-NS-033. Fragment-side facts: readdir rdlocks the directory's filelock and dirfragtreelock; a frag that is no longer a leaf is mapped through the current dirfragtree (dirfragtree[offset_hash] or dirfragtree[frag value]), and the reply names the leaf it served, so positions keyed by name hash survive refragmentation.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [dirfrags-and-fragmentation.md › 3.13 Client-visible behavior](../cpp-map/04-cache/dirfrags-and-fragmentation.md#313-client-visible-behavior) · [dirfrags-and-fragmentation.md › 7. Externally visible contract](../cpp-map/04-cache/dirfrags-and-fragmentation.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:5092 (Server::handle_client_readdir)`, `src/mds/Server.cc:5147-5163 (Server::handle_client_readdir)`, `src/mds/Server.cc:5063-5069 (Server::_finalize_readdir)`, `linux/fs/ceph/dir.c:317 (ceph_readdir)`, `src/client/Client.cc:9901 (Client::_readdir_next_frag)`
- **Notes:** Ganesha's use of the offsets as cookies was not inspected (unverified).

<a id="f-cache-079"></a>

#### F-CACHE-079 · DirStat delegation hints

*Per-fragment authority and replica hints in replies that clients use to route requests.*

Every trace dentry and every readdir reply carries DirStat {frag, auth = dir_auth.first (-1 = same as the directory inode's auth), dist = replica ranks plus self}; dist is filled only when this rank is auth, the dirfrag has replicas and mds_forward_all_requests_to_auth is false, and reflects actual replica holders. Encoded ENCODE_START(1, 1) for sessions with CEPHFS_FEATURE_REPLY_ENCODING, else as raw ceph_mds_reply_dirfrag {frag, auth, ndist, dist[]}. The kernel stores mds and up to 4 dist ranks per frag (dropping the record when ndist == 0 and auth equals the inode's auth) and sends USE_ANY_MDS requests to a random dist rank; libcephfs records fragmap[frag] = auth (negative erases), accepts dist only from the auth rank's replies and forces the frag to a leaf. Any rank can therefore receive any request and must serve it from replicas or forward. Hot-directory replication (dir_rep REP_ALL, MDirUpdate) is decided by the balancer ([balancer and pinning](../cpp-map/08-multi-mds/balancer-and-pinning.md)).

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The raw ceph_mds_reply_dirfrag DirStat form (frag, auth, ndist, dist[] without an envelope) for sessions without CephFS bit 9. Only sessions without CephFS bit 9 (kernels up to v5.0, Ceph 13.2.x and older) get the raw form. The 7.3 kernel and current libcephfs advertise bit 9 and get the ENCODE_START(1, 1) form. Evidence: `src/mds/CDir.cc:3058-3071 (CDir::encode_dirstat)`, `linux/fs/ceph/mds_client.h:48 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux/fs/ceph/mds_client.c:4153-4157 (handle_reply)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`.
- **Gates:** mds_forward_all_requests_to_auth (false); CEPHFS_FEATURE_REPLY_ENCODING (bit 9)
- **Map:** [cdir-cdentry-and-cache-objects.md › 5.1 Message payloads produced or consumed by these classes](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#51-message-payloads-produced-or-consumed-by-these-classes) · [dirfrags-and-fragmentation.md › 3.13 Client-visible behavior](../cpp-map/04-cache/dirfrags-and-fragmentation.md#313-client-visible-behavior) · [mdcache-subtrees-traversal-replication.md › 3.8.3 MDirUpdate and the dir_rep hint (src/mds/MDCache.cc...](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#383-mdirupdate-and-the-dir_rep-hint-srcmdsmdcachecc11348-mdcachesend_dir_updates-srcmdsmdcachecc11396-mdcachehandle_dir_update)
- **Code:** `src/mds/CDir.cc:3058 (CDir::encode_dirstat)`, `src/mds/CDir.h:436 (CDir::get_dist_spec)`, `linux/fs/ceph/inode.c:424 (ceph_fill_dirfrag)`, `src/client/Client.cc:1525 (Client::update_dir_dist)`, `linux/fs/ceph/mds_client.c:1381 (__choose_mds)`
- **Notes:** See also F-PROTO-083 (DirStat encoding), F-PROTO-075 (client rank selection), F-MULTI-050.

<a id="f-cache-080"></a>

#### F-CACHE-080 · Per-dirfrag size limit (ENOSPC)

*Creates fail with ENOSPC when the target dirfrag holds too many entries.*

Server::check_fragment_space returns -ENOSPC when the target dirfrag's projected fragstat.size() (nfiles + nsubdirs, head entries only) is >= mds_bal_fragment_size_max (100000) for openc, mknod, mkdir, symlink, link and every rename that is not a link merge (including same-directory renames and renames over an existing name), and for the current stray dirfrag in prepare_stray_dentry, so unlink, rmdir and rename-over fail with ENOSPC when that stray frag is full (replayed requests excepted). The root directory is never fragmented, so / is limited to that many entries. CDir keeps num_head_items, num_head_null, num_snap_items and num_snap_null (asserted to sum to the item count) and num_dirty.

- **Class:** POSIX `operational` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, admin
- **Gates:** mds_bal_fragment_size_max (100000)
- **Map:** [cdir-cdentry-and-cache-objects.md › 3.14 Directory contents, counters and completeness](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#314-directory-contents-counters-and-completeness) · [dirfrags-and-fragmentation.md › 3.13 Client-visible behavior](../cpp-map/04-cache/dirfrags-and-fragmentation.md#313-client-visible-behavior) · [dirfrags-and-fragmentation.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/dirfrags-and-fragmentation.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Server.cc:3563 (Server::check_fragment_space)`, `src/mds/Server.cc:3607-3609 (Server::prepare_stray_dentry)`, `src/mds/CDir.h:377 (CDir::get_frag_size)`
- **Notes:** Result frags of a split cannot be split again until the fragment op finishes, so very fast creates can push one toward the limit in that window (qa test_fragment). Exact per frag. See also F-NS-020 (gates in the request path), F-BG-004 (stray dirfrag), F-CACHE-082 (split thresholds).

<a id="f-cache-081"></a>

#### F-CACHE-081 · Per-directory entry limit

*An optional cap on the total entries of one directory.*

With mds_dir_max_entries > 0 (default 0 = off, runtime option), Server::check_dir_max_entries returns ENOSPC for openc, mknod, mkdir, symlink, link and rename when the target directory's projected dirstat.nfiles + nsubdirs (remote dentries included) is at or above the limit, independently of fragmentation; the stray directory is not checked.

- **Class:** POSIX `operational` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, admin
- **Gates:** mds_dir_max_entries (0)
- **Map:** [dirfrags-and-fragmentation.md › 3.13 Client-visible behavior](../cpp-map/04-cache/dirfrags-and-fragmentation.md#313-client-visible-behavior) · [dirfrags-and-fragmentation.md › 5.4 Configuration options](../cpp-map/04-cache/dirfrags-and-fragmentation.md#54-configuration-options)
- **Code:** `src/mds/Server.cc:3582 (Server::check_dir_max_entries)`, `src/common/options/mds.yaml.in:1683-1690`
- **Notes:** Approximate across ranks: when the directory's dirfrags are on ranks other than the inode's auth, the inode dirstat is only as fresh as the last filelock gather, so creates on those ranks can exceed the limit (derived). For kclient async creates the ENOSPC would arrive after the client already reported success (leases-lock-caches-and-async-dirops). See also F-NS-020.

### Directory split and merge

Automatic and manual split/merge of dirfrags: triggers, preconditions, the FRAGMENTDIR operation and its storage, replica and journal effects, cancellation, crash recovery and visible side effects.

<a id="f-cache-082"></a>

#### F-CACHE-082 · Size-triggered split and split checks

*Splitting a dirfrag that grows past a size threshold, re-checked after creates and on every dirfrag hit.*

MDBalancer::maybe_fragment runs right after openc, mknod, mkdir, symlink, link and rename journal_and_reply, from every hit_dir (lookups, getattr, open, unlink/link/rename finish, dirfrag fetch and commit, readdir), at the end of a directory scrub and at the end of each fragment op. If should_split (split size > 0 and projected fragstat.size + num_snap_items > mds_bal_split_size, 10000), queue_split(dir, false) arms a timer of mds_bal_fragment_interval (5 s) once per frag (split_pending); the split is by mds_bal_split_bits (3, i.e. 8 children). maybe_fragment acts only if mds_bal_fragment_dirs is true and the interval > 0, the dirfrag is auth, and the inode is neither a base inode nor a stray directory; the callback drops the request if the rank is stopping or the frag is no longer cached or auth.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_bal_fragment_dirs (true); mds_bal_split_size (10000); mds_bal_split_bits (3); mds_bal_fragment_interval (5 s)
- **Map:** [dirfrags-and-fragmentation.md › 3.5 When splits and merges are triggered](../cpp-map/04-cache/dirfrags-and-fragmentation.md#35-when-splits-and-merges-are-triggered)
- **Code:** `src/mds/MDBalancer.cc:1299-1308 (MDBalancer::maybe_fragment)`, `src/mds/MDBalancer.cc:664-670 (MDBalancer::queue_split)`, `src/mds/CDir.cc:1036 (CDir::should_split)`

<a id="f-cache-083"></a>

#### F-CACHE-083 · Fast split

*Splitting immediately when a dirfrag far exceeds the split size while a split is already pending.*

If a split is already pending and should_split_fast holds (limit = mds_bal_split_size x mds_bal_fragment_fast_factor (1.5): projected fragstat size > limit, or, when size + head null dentries exceed the limit, the count of cached dentries whose projected linkage is non-null exceeds it), queue_split(dir, true) queues an immediate waiter that runs after the current dispatch instead of waiting for the timer.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_bal_fragment_fast_factor (1.5)
- **Map:** [dirfrags-and-fragmentation.md › 3.5 When splits and merges are triggered](../cpp-map/04-cache/dirfrags-and-fragmentation.md#35-when-splits-and-merges-are-triggered)
- **Code:** `src/mds/CDir.cc:3871 (CDir::should_split_fast)`, `src/mds/MDBalancer.cc:1309-1311 (MDBalancer::maybe_fragment)`

<a id="f-cache-084"></a>

#### F-CACHE-084 · Temperature-triggered split

*Splitting a busy dirfrag regardless of its size.*

hit_dir increments decaying per-dirfrag popularity counters (half-life mds_decay_halflife, 5 s); an IRD count (inode reads: lookup, getattr, open) above mds_bal_split_rd (25000) or an IWR count above mds_bal_split_wr (10000) queues a split regardless of size. READDIR hits do not count toward mds_bal_split_rd (the documentation says they do).

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_bal_split_rd (25000); mds_bal_split_wr (10000); mds_decay_halflife (5 s)
- **Map:** [dirfrags-and-fragmentation.md › 3.5 When splits and merges are triggered](../cpp-map/04-cache/dirfrags-and-fragmentation.md#35-when-splits-and-merges-are-triggered) · [dirfrags-and-fragmentation.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/dirfrags-and-fragmentation.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDBalancer.cc:1325-1338 (MDBalancer::hit_dir)`
- **Notes:** There is no size floor for temperature splits and merges look only at size, so a hot small directory can be split and later merged again (inference from the trigger code).

<a id="f-cache-085"></a>

#### F-CACHE-085 · Size-triggered merge

*Merging small sibling dirfrags back together.*

should_merge: not the root frag, fragstat.size + num_snap_items < mds_bal_merge_size (50), and not shallower than the ephemeral distributed-pin minimum. queue_merge arms a timer once per frag (merge_pending); the callback climbs toward the root while all dirfrags under the sibling are cached, auth and should_merge, then merges to the highest qualifying ancestor. merge_dir requires every frag under the target to be cached and the target not to be a leaf; its bits value is computed from the first source only (sign significant, magnitude informational).

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_bal_merge_size (50)
- **Map:** [dirfrags-and-fragmentation.md › 3.5 When splits and merges are triggered](../cpp-map/04-cache/dirfrags-and-fragmentation.md#35-when-splits-and-merges-are-triggered) · [dirfrags-and-fragmentation.md › 3.6 Preconditions](../cpp-map/04-cache/dirfrags-and-fragmentation.md#36-preconditions)
- **Code:** `src/mds/CDir.cc:3906 (CDir::should_merge)`, `src/mds/MDBalancer.cc:716-745 (MDBalancer::queue_merge)`, `src/mds/MDCache.cc:11964 (MDCache::merge_dir)`

<a id="f-cache-086"></a>

#### F-CACHE-086 · Fragmentation preconditions and exclusions

*Directories and situations in which no split or merge happens.*

can_fragment silently drops the operation when the file system is read-only, the cluster is degraded (any rank failed, damaged or in replay..clientreplay), the directory's parent is a stray directory, the inode is ~mdsN or /.ceph, the inode is quiesced, the inode is the root directory (added by commit f4bcf5bfd53, 2025), or any source dirfrag is being scrubbed, already FRAGMENTING, not auth, BADFRAG, or freezing/frozen (dir or tree). split_dir drops splits that would exceed 24 bits. maybe_fragment also skips base inodes and stray directories (strays are pre-fragmented by stray rotation, [strays and purge](../cpp-map/11-background-services/strays-and-purge.md)).

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, admin
- **Legacy parts** (old-data): The /.ceph (ino 2, CEPH_INO_CEPH) exclusion in can_fragment. Ino 2 exists only on file systems created by C++ releases before v0.79, because commit 81bcf43080a (2014) stopped creating /.ceph and left it on existing file systems. No component at this commit creates ino 2. Evidence: `src/mds/MDCache.cc:11889 (MDCache::can_fragment)`, `src/include/ceph_fs.h:58 (CEPH_INO_CEPH)`.
- **Map:** [dirfrags-and-fragmentation.md › 3.6 Preconditions](../cpp-map/04-cache/dirfrags-and-fragmentation.md#36-preconditions) · [dirfrags-and-fragmentation.md › 8. Edge cases, failure handling and gotchas](../cpp-map/04-cache/dirfrags-and-fragmentation.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDCache.cc:11874 (MDCache::can_fragment)`, `src/mds/MDCache.cc:11944 (MDCache::split_dir)`, `src/mds/MDBalancer.cc:1299-1302 (MDBalancer::maybe_fragment)`
- **Notes:** See also F-ADM-052 (dirfrag split/merge commands).

<a id="f-cache-087"></a>

#### F-CACHE-087 · Fragment operation (FRAGMENTDIR)

*The internal request that freezes source dirfrags, refragments the cache and makes the change durable.*

split_dir / merge_dir create an internal MDRequest CEPH_MDS_OP_FRAGMENTDIR (0x01500) and a fragments[base] entry (at most one per base dirfrag). Steps: freeze each source (auth pin, STATE_FRAGMENTING, freeze_dir; a mixed set of subtree roots is made homogeneous with aux subtrees); mark and complete (full fetch of incomplete sources, commit a never-committed root frag first, wait CREATED for a root frag still being created, pin every dentry with PIN_FRAGMENTING, set STATE_DNPINNEDFRAG, register WAIT_FROZEN and flush the journal so in-flight requests' auth pins become journal-safe and drop); when all are frozen, take the dirfragtreelock wrlock plus filelock/nestlock scatter-gather pins with nonblocking auth pins (a quiescelock wrlock is added); journal EFragment PREPARE (original frags with rollback fnodes, new dirlumps, inode fullbit if inode auth else DIRTYDFT) and refragment the cache; when safe, write the new objects; then send replica notifies, journal COMMIT, unfreeze the new frags and drop locks (keeping the dirfragtreelock while notify acks are pending); after COMMIT is safe purge old objects and journal FINISH; finally clear FRAGMENTING and re-evaluate the result frags. Result frags keep FRAGMENTING and an auth pin until FINISH and all acks, so they cannot be refragmented or frozen for export meanwhile.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal, osd
- **Map:** [dirfrags-and-fragmentation.md › 3.7 The fragment operation on the dirfrag auth](../cpp-map/04-cache/dirfrags-and-fragmentation.md#37-the-fragment-operation-on-the-dirfrag-auth) · [dirfrags-and-fragmentation.md › 5.2 Journal events](../cpp-map/04-cache/dirfrags-and-fragmentation.md#52-journal-events) · [dirfrags-and-fragmentation.md › 4. Data structures and invariants](../cpp-map/04-cache/dirfrags-and-fragmentation.md#4-data-structures-and-invariants)
- **Code:** `src/mds/MDCache.cc:11930 (MDCache::split_dir)`, `src/mds/MDCache.cc:12040 (MDCache::fragment_mark_and_complete)`, `src/mds/MDCache.cc:12282 (MDCache::dispatch_fragment_dir)`, `src/mds/MDCache.cc:12445 (MDCache::_fragment_stored)`, `src/mds/MDCache.cc:12608 (MDCache::fragment_maybe_finish)`
- **Notes:** COMMIT and FINISH are submitted without an explicit journal flush. Code-reading findings: the quiesce-overdrive abort can call request_finish twice (visible as a double count of mds_cache.ireq_fragmentdir); a RADOS error storing a new frag goes to handle_write_error and leaves the op stuck with its new frags frozen. Dev option mds_kill_dirfrag_at provides kill points 1..10 for tests. EFragment encoding: [events and replay](../cpp-map/06-journal/events-and-replay.md).

<a id="f-cache-088"></a>

#### F-CACHE-088 · Dentry repartitioning and stat-delta preservation

*Moving dentries between dirfrag objects in memory while keeping directory statistics exact.*

CDir::split creates 2^bits children that copy the replica map, dir_auth, freeze_tree_state, MASK_STATE_FRAGMENT_KEPT | COMPLETE state and popularity scaled by 1/2^bits; every dentry moves to the child chosen by the updated tree, and dentry waiters are re-homed by name hash. CDir::merge takes dir_auth and freeze state from the first sub (all must match), the maximum nonce per replica, the union of kept state and the highest fnode version; popularity of the merged frag starts at zero. steal_dentry moves the CDentry object itself (state, locks, client leases, replicas, pins) and recomputes counters, fragstat from linkage, rstat from children's accounted_rstat, num_inodes_with_caps (moving the open-file-table registration), dirty lists and nested auth pins. Pending deltas (fragstat - accounted_fragstat, rstat - accounted_rstat) go to the first child on split and are summed on merge, so the next scatter-gather still propagates them. Dentry leases survive because objects are moved, not recreated.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [dirfrags-and-fragmentation.md › 3.8 In-memory refragmentation](../cpp-map/04-cache/dirfrags-and-fragmentation.md#38-in-memory-refragmentation) · [cdir-cdentry-and-cache-objects.md › 3.16 Fragments: frag_t and fragtree_t basics](../cpp-map/04-cache/cdir-cdentry-and-cache-objects.md#316-fragments-frag_t-and-fragtree_t-basics)
- **Code:** `src/mds/CDir.cc:1042 (CDir::split)`, `src/mds/CDir.cc:1145 (CDir::merge)`, `src/mds/CDir.cc:883 (CDir::steal_dentry)`, `src/mds/MDCache.cc:11734 (MDCache::adjust_dir_fragments)`

<a id="f-cache-089"></a>

#### F-CACHE-089 · New dirfrag object write and old object purge

*Cache side of writing new dirfrag objects and purging old ones in a split or merge.*

See F-STORE-034 for the object operations and their order. Cache-side facts: after PREPARE is safe each result frag is marked dirty and new and committed with ignore_authpinnability; because it is FRAGMENTING and new, _omap_commit writes every non-null dentry without the stat guard, chunked by mds_dir_max_commit_size with the header last (encoding on the finisher thread without mds_lock). Between the store and the purge, dentries exist in both old and new objects; cephfs-data-scan scan_links tolerates such duplicates.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by osd, tools
- **Map:** [dirfrags-and-fragmentation.md › 3.7 The fragment operation on the dirfrag auth](../cpp-map/04-cache/dirfrags-and-fragmentation.md#37-the-fragment-operation-on-the-dirfrag-auth) · [dirfrags-and-fragmentation.md › 5.3 RADOS objects (metadata pool)](../cpp-map/04-cache/dirfrags-and-fragmentation.md#53-rados-objects-metadata-pool)
- **Code:** `src/mds/MDCache.cc:12413 (MDCache::_fragment_logged)`, `src/mds/MDCache.cc:12529 (MDCache::_fragment_committed)`, `src/mds/CDir.cc:2624 (CDir::_omap_commit)`, `src/tools/cephfs/DataScan.cc:1539 (DataScan::scan_links)`

<a id="f-cache-090"></a>

#### F-CACHE-090 · Replica notification of refragmentation

*Telling ranks that replicate a directory's dirfrags about a split or merge.*

After the new objects are stored, MMDSFragmentNotify (0x209; base dirfrag, int8 bits, replica-dir blobs of the result frags, ack_wanted (v2); header tid = request tid) goes to each replica rank of the first result frag that is in REJOIN or later and not still in rejoin_gather. Replicas (the message is dropped before REJOIN; a missing directory inode aborts the daemon, which trim_inode prevents by keeping replica directory inodes whose dirfragtreelock is unreadable) refragment without freezing, decode the replica dirs and reply MMDSFragmentNotifyAck (0x212) if asked. ack_wanted is set only when the first result frag is a subtree root, this rank is not the inode auth and the replica is not the inode auth; the dirfragtreelock wrlock is held until all acks arrive or the ranks fail. Cache expiries naming old frags are dropped, dentry expiries are re-routed by hash, and discover replies correct the base frag.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [dirfrags-and-fragmentation.md › 3.9 Replicas: MMDSFragmentNotify and MMDSFragmentNotifyAck](../cpp-map/04-cache/dirfrags-and-fragmentation.md#39-replicas-mmdsfragmentnotify-and-mmdsfragmentnotifyack) · [dirfrags-and-fragmentation.md › 5.1 Messages](../cpp-map/04-cache/dirfrags-and-fragmentation.md#51-messages)
- **Code:** `src/mds/MDCache.cc:12655 (MDCache::handle_fragment_notify)`, `src/mds/MDCache.cc:12631 (MDCache::handle_fragment_notify_ack)`, `src/mds/MDCache.cc:12460-12498 (MDCache::_fragment_stored)`
- **Notes:** Ranks before REJOIN miss notifies and learn the fragmentation from rejoin. Acks are ignored while the auth is not active.

<a id="f-cache-091"></a>

#### F-CACHE-091 · Fragtree propagation through dirfragtreelock

*Letting a rank that owns some dirfrags, but not the directory inode, change the fragment tree.*

A rank that is auth for a dirfrag but not for the directory inode fragments it while holding the IDFT scatterlock wrlock in LOCK_MIX (requested from the inode auth with REQSCATTER); it journals DIRTYDFT dirlumps and marks the lock dirty. Its lock payload carries a dirty flag, the tree and the frags it is auth for; the inode auth forces those frags to leaves in its tree and marks the lock dirty, so scatter writebehind journals and stores the updated tree; replicas accept the auth's tree while forcing their open frags to stay leaves and clear STATE_DIRTYDFT. A log segment with dirty dirfragtree state cannot expire until scatter_nudge flushes it; rejoin messages carry the dft lock state. Lock state machine: [lock types and state machines](../cpp-map/05-locking/lock-types-and-state-machines.md).

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks
- **Map:** [dirfrags-and-fragmentation.md › 3.10 Multi-MDS fragtree propagation (dirfragtreelock)](../cpp-map/04-cache/dirfrags-and-fragmentation.md#310-multi-mds-fragtree-propagation-dirfragtreelock)
- **Code:** `src/mds/CInode.cc:1856 (CInode::encode_lock_idft)`, `src/mds/CInode.cc:1884 (CInode::decode_lock_idft)`, `src/mds/Locker.cc:1997-2033 (Locker::wrlock_start)`, `src/mds/journal.cc:206-210 (LogSegment::try_to_expire)`

<a id="f-cache-092"></a>

#### F-CACHE-092 · Subtree map maintenance on split and merge

*Keeping delegation points correct when subtree-root dirfrags are split or merged.*

adjust_dir_fragments edits the subtree map directly: splitting a subtree root makes every child a subtree root that replaces the source in its parent's bound set, and the source's bounds go to the child that now contains each; merging requires the sources to be all subtree roots or none (asserted), unions their bounds and replaces them with the merged frag. AUXSUBTREE, EXPORTBOUND and IMPORTBOUND survive split and merge.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks
- **Map:** [dirfrags-and-fragmentation.md › 3.8 In-memory refragmentation](../cpp-map/04-cache/dirfrags-and-fragmentation.md#38-in-memory-refragmentation) · [mdcache-subtrees-traversal-replication.md › 3.2.5 Other mutations](../cpp-map/04-cache/mdcache-subtrees-traversal-replication.md#325-other-mutations)
- **Code:** `src/mds/MDCache.cc:11764-11800 (MDCache::adjust_dir_fragments)`, `src/mds/MDCache.cc:11807-11856 (MDCache::adjust_dir_fragments)`

<a id="f-cache-093"></a>

#### F-CACHE-093 · Abort, requeue and cancellation of fragment operations

*How fragment operations back off to avoid deadlocks and what happens to them on failures.*

Lock acquisition aborts the op and re-queues it (delayed split or merge) when a nonblocking auth pin would block (for example the parent dirfrag is freezing for export or fragmentation), a remote auth-pin reply carries an error, or the directory's quiescelock cannot be wrlocked (would deadlock with quiesce_inode ops on children). A quiesce_inode or quiesce_path op that meets a FRAGMENTING dir aborts the op asynchronously through the same path unless all its dirs are already frozen. Stale-freeze cancellation and cancellation of ops before PREPARE on another rank's failure unfreeze without re-queue; ops past PREPARE just drop the failed rank from the ack wait. A later maybe_fragment re-arms dropped ops.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, mds-ranks
- **Map:** [dirfrags-and-fragmentation.md › 3.11 Cancellation, abort and deadlock avoidance](../cpp-map/04-cache/dirfrags-and-fragmentation.md#311-cancellation-abort-and-deadlock-avoidance)
- **Code:** `src/mds/MDCache.cc:12338-12350 (MDCache::dispatch_fragment_dir)`, `src/mds/MDCache.cc:14019 (MDCache::quiesce_overdrive_fragmenting_async)`, `src/mds/MDCache.cc:3153-3174 (MDCache::handle_mds_failure)`
- **Notes:** See also F-QOS-048 (quiesce and fragmentation).

<a id="f-cache-094"></a>

#### F-CACHE-094 · Recovery of in-flight fragment operations

*Rolling interrupted splits and merges forward or back after a crash.*

EFragment (event type 6; op PREPARE 1, COMMIT 2, ROLLBACK 3, FINISH 4; ino, basefrag, signed bits, metablob, orig_frags, rollback fnodes; ENCODE_START(5, 4)) replay maintains uncommitted_fragments: PREPARE adds an entry (and refragments the cache if the inode is cached), COMMIT marks it committed, FINISH erases it. When the rank leaves replay (resolve_start, or reconnect_start for single-rank recovery), rollback_uncommitted_fragments deletes the old objects and journals FINISH for committed ops; PREPARE-only ops journal ROLLBACK (original frags restored from the rollback fnodes and marked dirty), then delete the new objects and journal FINISH, without waiting for ROLLBACK to be safe. A segment containing a PREPARE cannot expire before FINISH, and rejoin_done waits for all uncommitted fragments. Safety rests on: new objects complete before COMMIT is journaled, old objects removed only after COMMIT is durable, and COMMIT journaled before any mutation of the new frags.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal, osd
- **Map:** [dirfrags-and-fragmentation.md › 3.12 Journal replay and recovery of in-flight operations](../cpp-map/04-cache/dirfrags-and-fragmentation.md#312-journal-replay-and-recovery-of-in-flight-operations) · [dirfrags-and-fragmentation.md › 5.2 Journal events](../cpp-map/04-cache/dirfrags-and-fragmentation.md#52-journal-events)
- **Code:** `src/mds/journal.cc:2978 (EFragment::replay)`, `src/mds/MDCache.cc:12778 (MDCache::rollback_uncommitted_fragments)`, `src/mds/MDSRank.cc:2054-2058 (MDSRank::rejoin_done)`, `src/mds/journal.cc:192-198 (LogSegment::try_to_expire)`
- **Notes:** mds.dir_split / mds.dir_merge are incremented when the purge completes, including for rolled-back ops. See also F-STORE-034 (EFragment record and object cleanup).

<a id="f-cache-095"></a>

#### F-CACHE-095 · Forced refragmentation during replay, resolve and rejoin

*Making the cache's dirfrags match what the journal or peers describe.*

Every replayed EMetaBlob dirlump forces the cache to hold exactly that dirfrag (get_force_dirfrag -> force_dir_fragment: return it if open, else split the nearest open ancestor, else merge what is open under it); replayed inode fullbits replace the fragtree and force_dirfrags refragments open dirfrags to match; ESubtreeMap replay forces bound frags. Resolve forces local dirfrags to match peers' subtree claims (get_force_dirfrag_bound_set only splits, never merges); rejoin acks carry strong dirfrags that refragment replicas; a strong rejoin naming non-leaf frags is mapped onto local leaves and its dentries re-picked by name hash.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, internal
- **Map:** [dirfrags-and-fragmentation.md › 3.8 In-memory refragmentation](../cpp-map/04-cache/dirfrags-and-fragmentation.md#38-in-memory-refragmentation) · [dirfrags-and-fragmentation.md › 3.10 Multi-MDS fragtree propagation (dirfragtreelock)](../cpp-map/04-cache/dirfrags-and-fragmentation.md#310-multi-mds-fragtree-propagation-dirfragtreelock) · [dirfrags-and-fragmentation.md › 3.4 Where the fragtree lives and how it stays consistent](../cpp-map/04-cache/dirfrags-and-fragmentation.md#34-where-the-fragtree-lives-and-how-it-stays-consistent)
- **Code:** `src/mds/MDCache.cc:11681 (MDCache::force_dir_fragment)`, `src/mds/CInode.cc:796 (CInode::force_dirfrags)`, `src/mds/MDCache.cc:1252 (MDCache::get_force_dirfrag_bound_set)`, `src/mds/journal.cc:646-650 (EMetaBlob::fullbit::update_inode)`

<a id="f-cache-096"></a>

#### F-CACHE-096 · Migration and fragmentation mutual exclusion

*Keeping subtree export and dirfrag split/merge from interleaving on the same directories.*

An export rdlocks the dirfragtreelock of the exported dirfrag's inode and of every bound inode (scatterlock state shipped for bounds would be useless if a refragment raced); can_fragment refuses freezing or frozen trees; a fragment op's auth pins block an export's tree freeze; maybe_split_export skips dirfrags freezing or frozen as dirs; the importer forces each imported dirfrag to be a leaf of its tree. Export protocol: [subtree migration](../cpp-map/08-multi-mds/subtree-migration.md).

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks
- **Map:** [dirfrags-and-fragmentation.md › 3.10 Multi-MDS fragtree propagation (dirfragtreelock)](../cpp-map/04-cache/dirfrags-and-fragmentation.md#310-multi-mds-fragtree-propagation-dirfragtreelock)
- **Code:** `src/mds/Migrator.cc:1235-1262 (Migrator::dispatch_export_dir)`, `src/mds/Migrator.cc:877-895 (Migrator::export_try_grab_locks)`, `src/mds/Migrator.cc:2848-2849 (Migrator::handle_export_dir)`

<a id="f-cache-097"></a>

#### F-CACHE-097 · Effects of a fragment freeze on clients

*Stalls and cap changes clients observe while a dirfrag is being split or merged.*

From freeze until the new objects are stored: operations needing auth pins in the dirfrag wait, including those on child inodes (CInode::can_auth_pin defers to the parent dirfrag) and, on the dirfrag auth, lookups and getattrs that rdlock a dentry there (acquire_locks auth-pins every auth object it rdlocks); the traversal itself does not wait. Cap messages for child inodes are deferred while the parent dirfrag is frozen, or freezing and the inode has no auth pins (unless a lock cache pins the dirfrag); max_size increases and cap reissue wait for unfreeze. freeze_dir invalidates lock caches that pin the dirfrag, revoking DIR_CREATE / DIR_UNLINK so clients fall back to synchronous creates and unlinks. Readdir and snapshot lookup-by-hash on any frag of the directory wait while the dirfragtreelock wrlock is held. Replica ranks never freeze their copies. Lock caches: [leases, lock caches and async dirops](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [dirfrags-and-fragmentation.md › 3.13 Client-visible behavior](../cpp-map/04-cache/dirfrags-and-fragmentation.md#313-client-visible-behavior) · [dirfrags-and-fragmentation.md › 6. Concurrency and mds_lock](../cpp-map/04-cache/dirfrags-and-fragmentation.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/Locker.cc:3363 (Locker::should_defer_client_cap_frozen)`, `src/mds/Locker.cc:3105 (Locker::check_inode_max_size)`, `src/mds/CDir.cc:3622-3623 (CDir::freeze_dir)`, `src/mds/Locker.cc:388-392 (Locker::acquire_locks)`

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-cache-002"></a>F-CACHE-002 | [F-STORE-046](06-journal-and-storage.md#f-store-046) · Special inode numbers and per-rank ranges |
| <a id="f-cache-005"></a>F-CACHE-005 | [F-NS-085](04-namespace-and-posix-operations.md#f-ns-085) · Directory link count and size presentation |
| <a id="f-cache-017"></a>F-CACHE-017 | [F-STORE-052](06-journal-and-storage.md#f-store-052) · Dirfrag commit to RADOS |
| <a id="f-cache-019"></a>F-CACHE-019 | [F-STORE-051](06-journal-and-storage.md#f-store-051) · Dirfrag fetch from RADOS |
| <a id="f-cache-021"></a>F-CACHE-021 | [F-BG-053](09-background-services.md#f-bg-053) · Dirfrag load damage detection |
| <a id="f-cache-022"></a>F-CACHE-022 | [F-BG-054](09-background-services.md#f-bg-054) · Dentry snapshot-bound corruption check at load and commit |
| <a id="f-cache-031"></a>F-CACHE-031 | [F-CAPS-089](03-caps-leases-and-locking.md#f-caps-089) · Freeze versus auth-pin deadlock breaker |
| <a id="f-cache-042"></a>F-CACHE-042 | [F-SNAP-021](08-snapshots-quotas-subvolumes.md#f-snap-021) · Snapshot path encoding, traversal and LOOKUPSNAP |
| <a id="f-cache-043"></a>F-CACHE-043 | [F-NS-025](04-namespace-and-posix-operations.md#f-ns-025) · Negative lookup replies (null-dentry traces) |
| <a id="f-cache-046"></a>F-CACHE-046 | [F-BG-060](09-background-services.md#f-bg-060) · EIO for requests that touch known damage |

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
