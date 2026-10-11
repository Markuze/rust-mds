# Snapshots, quotas and subvolumes: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [snapshots.md](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md) · [rstats-quotas-and-subvolumes.md](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md)

## 1. Scope

This catalog lists the snapshot, recursive-statistics, quota and subvolume features of the C++ MDS, from the fact-checked map pages [snapshots](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md) and [rstats-quotas-and-subvolumes](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md). It covers snapshot id allocation (SnapServer/SnapClient), snap realms and snap traces, MClientSnap notifications, the `.snap` request ops (MKSNAP, RMSNAP, RENAMESNAP, LSSNAP, LOOKUPSNAP, SNAP_METADATA, the snapid branch of LOOKUPINO), metadata copy-on-write and its lazy cleanup, the cephfs-mirror helpers READDIR_SNAPDIFF and FILE_BLOCKDIFF, dirstat/rstat maintenance and propagation, quotas, and the subvolume flag and its rules. The two directory entry limits (mds_bal_fragment_size_max and mds_dir_max_entries) are catalogued in 05-metadata-cache-and-dirfrags.

Excluded and owned elsewhere: the cap-level snapflush protocol (03-caps-leases-and-locking), the generic table two-phase commit and journal mechanics (06-journal-and-storage), scatterlock state machines and lock acquisition (03-caps-leases-and-locking), the recovery state machine and reconnect message format (01-cluster-membership-and-ha, 02-client-sessions-and-protocol), stray handling and scrub-driven stat repair (09-background-services), subvolume metrics, quarantine and the QoS volume definition (10-qos-metrics-quiesce-quarantine), the server-side vxattr whitelist (04-namespace-and-posix-operations), and admin commands, perf counters and mgr/volumes itself (11-security-admin-and-observability). Where an entry here touches those mechanisms it describes only the snapshot, statistics, quota or subvolume part and links to the owning map page.

Cross-cutting facts: the C++ MDS never enforces quotas (clients do), its realm notifications always use op SPLIT on the wire, the kernel client only understands the old snap-trace format, and clients find quota roots only through snap realms.

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

54 features: 52 used by current peers (6 of them with legacy parts) and 2 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**Snapshot ids and the snap table**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-001](#f-snap-001) | Snapshot id and realm seq allocation from one monotonic counter | extension | client-wire, cluster-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, osd, mon |
| [F-SNAP-002](#f-snap-002) | Rank-0 snap table service (SnapServer) and per-rank SnapClient cache *(legacy parts)* | internal | mds-wire, on-disk | mds-ranks, tools |
| [F-SNAP-003](#f-snap-003) | Deleted snapshots reported to the monitor (MRemoveSnaps) *(legacy parts)* | extension | cluster-wire, on-disk | mon, osd |
| [F-SNAP-004](#f-snap-004) | MDS data writes carry the realm SnapContext *(legacy parts)* | extension | cluster-wire | osd |

**Snap realms and client notification**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-006](#f-snap-006) | Snap realms and effective snapshot sets *(legacy parts)* | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, tools |
| [F-SNAP-007](#f-snap-007) | Realm membership of caps and realm split on creation | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-008](#f-snap-008) | Snap trace contents and client interpretation | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-009](#f-snap-009) | Per-realm MClientSnap notification (always op SPLIT) | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-010](#f-snap-010) | Global realm broadcast on every snap-table operation | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-011](#f-snap-011) | Hard-link global realm (PARENT_GLOBAL) | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, internal |
| [F-SNAP-012](#f-snap-012) | Past-parent snapshots on rename and unlink | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, internal |

**Snapshot operations**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-013](#f-snap-013) | Create a snapshot (MKSNAP, mkdir in .snap) | extension | client-wire, mds-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, mgr, mds-ranks |
| [F-SNAP-014](#f-snap-014) | Remove a snapshot (RMSNAP, rmdir in .snap) | extension | client-wire, mds-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, mgr, mds-ranks |
| [F-SNAP-015](#f-snap-015) | Rename a snapshot (RENAMESNAP) | extension | client-wire, mds-wire, on-disk | kclient, libcephfs, ceph-fuse, mds-ranks |
| [F-SNAP-016](#f-snap-016) | Snapshot metadata at creation (SnapPayload) | extension | client-wire, on-disk | libcephfs, cephfs-mirror |
| [F-SNAP-017](#f-snap-017) | Modify snapshot metadata (SNAP_METADATA op) | extension | client-wire, mds-wire, on-disk | libcephfs, cephfs-mirror, mds-ranks |
| [F-SNAP-018](#f-snap-018) | allow_new_snaps filesystem flag | operational | cluster-wire, admin | admin, mon |
| [F-SNAP-019](#f-snap-019) | Per-directory snapshot limit (mds_max_snaps_per_dir) | extension | config, client-wire | kclient, libcephfs, ceph-fuse, ganesha, mgr, admin |
| [F-SNAP-020](#f-snap-020) | Snapshot uid range (mds_snap_min_uid, mds_snap_max_uid) | operational | config | admin |

**Reading snapshots**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-021](#f-snap-021) | Snapshot path encoding, traversal and LOOKUPSNAP | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-022](#f-snap-022) | List snapshots (LSSNAP, readdir of .snap) | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, mgr |
| [F-SNAP-023](#f-snap-023) | Read-only snapshot semantics and snapshot caps | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-024](#f-snap-024) | Snapshot attributes in InodeStat (snap_btime, snap_metadata) | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, mgr |
| [F-SNAP-025](#f-snap-025) | Snapshot inode lookup by ino (LOOKUPINO with snapid) | extension | client-wire | kclient |

**Metadata copy-on-write and cleanup**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-026](#f-snap-026) | Metadata copy-on-write (first/last ranges, snapped dentries, old_inodes) | extension | on-disk | kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, tools |
| [F-SNAP-027](#f-snap-027) | Ancestor pre-COW and the COW sequence (global or subvolume-local) | internal | config | internal |
| [F-SNAP-028](#f-snap-028) | Lazy trimming of deleted-snapshot metadata | internal | on-disk | internal |

**Snapshot diff for cephfs-mirror**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-029](#f-snap-029) | Directory snapshot diff (READDIR_SNAPDIFF) | extension | client-wire | libcephfs, cephfs-mirror |
| [F-SNAP-030](#f-snap-030) | File block diff between snapshots (FILE_BLOCKDIFF) | extension | client-wire, cluster-wire | libcephfs, cephfs-mirror, osd |

**Multi-rank snapshot coordination and recovery**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-031](#f-snap-031) | Realm state replication between ranks (MMDSSnapUpdate, snaplock payload) | internal | mds-wire | mds-ranks |
| [F-SNAP-032](#f-snap-032) | Snaprealm reconciliation at client reconnect and rejoin | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Directory statistics (dirstat and rstat)**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-033](#f-snap-033) | Directory entry statistics (dirstat / fragstat) | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-034](#f-snap-034) | Recursive subtree statistics (rstat) | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mgr, tools |
| [F-SNAP-035](#f-snap-035) | Eager upward stat propagation (predirty_journal_parents) | supporting | on-disk, config | internal |
| [F-SNAP-036](#f-snap-036) | Deferred stat propagation (dirty rstat, updated scatterlocks, tick and nudge) | internal | mds-wire, config | mds-ranks, internal |
| [F-SNAP-037](#f-snap-037) | Scatter-gather of dirstat and rstat across dirfrags and ranks | internal | mds-wire, on-disk | mds-ranks, internal |
| [F-SNAP-038](#f-snap-038) | Journal segment expiry waits for stat folding | internal | on-disk | internal |
| [F-SNAP-039](#f-snap-039) | rstat reads routed to the auth rank (GETATTR CEPH_STAT_RSTAT) | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-040](#f-snap-040) | Snapshot rstats (mds_snap_rstat) | extension | config, on-disk | kclient, libcephfs, ceph-fuse |
| [F-SNAP-041](#f-snap-041) | Stat mismatch detection and in-place repair | internal | metrics, config | admin, internal |

**Quotas**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-042](#f-snap-042) | Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files) | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mgr, tools, admin |
| [F-SNAP-043](#f-snap-043) | Quota directories become snaprealm roots | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-044](#f-snap-044) | Quota and usage in InodeStat *(legacy parts)* | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-045](#f-snap-045) | MClientQuota usage pushes with thresholds *(legacy parts)* | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-SNAP-047](#f-snap-047) | Client-side quota enforcement (EDQUOT, cross-quota EXDEV) | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-SNAP-048](#f-snap-048) | Quota-based statfs | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, mgr |

**Subvolumes**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-SNAP-050](#f-snap-050) | Subvolume flag (ceph.dir.subvolume) | extension | client-wire, on-disk | mgr, kclient, libcephfs, tools |
| [F-SNAP-051](#f-snap-051) | No nested subvolumes; outermost subvolume wins | extension | client-wire | mgr |
| [F-SNAP-052](#f-snap-052) | Cross-subvolume link and rename rejected with EXDEV | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-SNAP-053](#f-snap-053) | Snapshots only at the subvolume root | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-SNAP-054](#f-snap-054) | Subvolume id on the wire (InodeStat, MClientCaps) | extension | client-wire | kclient, libcephfs, ceph-fuse |
| [F-SNAP-055](#f-snap-055) | Snapshot visibility flag (ceph.dir.subvolume.snaps.visible) | extension | client-wire, on-disk | libcephfs, ceph-fuse, mgr |

**Legacy entries**

| ID | Feature | Legacy reasons | Relied on by |
|---|---|---|---|
| [F-SNAP-005](#f-snap-005) | allow_multimds_snaps flag and MULTIMDS_WITH_OLDSNAPS health check | old-data | mon, admin |
| [F-SNAP-049](#f-snap-049) | Legacy pre-Mimic quota realm creation (create_quota_realm) | old-data | internal |

## 3. Features

### Snapshot ids and the snap table

How snapshot ids are allocated filesystem-wide, how the allocation is shared between ranks, and how deleted snapshots reach the monitor and OSDs. The generic table two-phase commit is on [mds-tables](../cpp-map/07-storage-formats/mds-tables.md#33-table-server--client-the-generic-two-phase-commit).

<a id="f-snap-001"></a>

#### F-SNAP-001 · Snapshot id and realm seq allocation from one monotonic counter

*Gives every snapshot a filesystem-wide unique, increasing `snapid_t` that clients and OSDs use as a self-managed RADOS snapshot id.*

`snapid_t` is a u64; `CEPH_NOSNAP = -2` is head, `CEPH_SNAPDIR = -1` the `.snap` pseudo-directory, `CEPH_MAXSNAP = -3` the largest valid id, and id 1 is reserved. The table counter `last_snap` starts at `max(1, max snap_seq over the data pools)` (`SnapServer::reset_state`), so new ids never collide with ids the pools already used. Every snapshot create consumes one value (the snapid) and every destroy consumes one value (the new realm `seq`); realm seqs and snapids therefore come from the same sequence, and clients compare seqs to decide whether a trace is newer. Values handed out by a prepare are consumed even if the transaction rolls back, so ids have gaps. The "global newest seq" `max(last_created, last_destroyed)` is used as `follows` for copy-on-write and, plus one, as `first` of every new dentry.

- **Class:** POSIX `extension` · exposure `client-wire`, `cluster-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, osd, mon
- **Map:** [snapshots.md › 3.1 Snapshot ids and the global snap table](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#31-snapshot-ids-and-the-global-snap-table) · [snapshots.md › 4.3 Invariants](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#43-invariants) · [snapshots.md › 7.4 Monitor and OSDs](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#74-monitor-and-osds)
- **Code:** `src/include/rados.h:38-40`, `src/mds/SnapServer.cc:45-78 (SnapServer::reset_state)`, `src/mds/SnapServer.cc:127-286 (SnapServer::_prepare)`, `src/mds/SnapRealm.cc:128-130 (SnapRealm::check_cache)`
- **Notes:** Filesystems that share data pools collide on snap ids (documented in `doc/dev/cephfs-snapshots.rst`). The monitor raises a pool `snap_seq` to any removed snap (F-SNAP-003), which keeps the "ids above pool snap_seq" invariant. See also F-STORE-077 (SnapTable object).

<a id="f-snap-002"></a>

#### F-SNAP-002 · Rank-0 snap table service (SnapServer) and per-rank SnapClient cache

*Serialises snapshot create/destroy/update for the whole filesystem on rank 0 and keeps a synced copy of the table on every rank.*

Rank 0 (MDSMap `tableserver`) runs `SnapServer`; every rank runs a `SnapClient`. Table ops (first `__u32` of the payload): `CREATE {ino, name, stamp}` returns the new snapid, `DESTROY {ino, snapid}` returns the new seq, `UPDATE {ino, snapid, name, stamp}` is used by renamesnap and the snapshot metadata op. They run through the generic table 2PC: the server journals `ETableServer PREPARE` before computing the value, then sends `NOTIFY_PREP` carrying the *whole* table to every rank in state `>= resolve` (its own SnapClient included) and waits for every `NOTIFY_ACK` before `AGREE`; there is no incremental form. A pending entry becomes visible on a rank only after `notify_commit` (on the requester at commit, on other ranks via `MMDSSnapUpdate` or resolve). The set accessors assert a synced cache: `sync()` runs at `starting_done`, `creating_done`, `resolve_done` and the single-rank replay shortcut, and a rank does not leave reconnect until its cache is synced. The table is the `mds_snaptable` object in the metadata pool (`SnapServer` state `ENCODE_START(5, 3)`), rewritten whole by rank 0. While rank 0 is down no snapshot can be created, removed or renamed.

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks, tools
- **Legacy parts** (dead-code): TABLE_OP_CREATE carrying only an ino (SnapClient::prepare_create_realm) and the SnapServer::_prepare branch that records such a request in pending_noop. SnapClient::prepare_create_realm has no caller in the tree, and pending_noop is filled only by the matching server branch. Luminous 12.2.13 still called prepare_create_realm from MDCache; no MDS from Mimic 13.2.0 on calls it. The CREATE, DESTROY and UPDATE ops with full payloads are current. Evidence: `src/mds/SnapClient.cc:160 (SnapClient::prepare_create_realm)`, `src/mds/SnapClient.h:45 (SnapClient::prepare_create_realm)`, `src/mds/SnapServer.cc:146-149 (SnapServer::_prepare)`, `v12.2.13:src/mds/MDCache.cc:9532`. Also: search: grep -rn prepare_create_realm over src finds only SnapClient.cc:160 and SnapClient.h:45; pending_noop is inserted only at SnapServer.cc:147.
- **Map:** [snapshots.md › 3.1 Snapshot ids and the global snap table](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#31-snapshot-ids-and-the-global-snap-table) · [snapshots.md › 5.4 RADOS objects](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#54-rados-objects) · [mds-tables.md › 3.3.9 Hook contract and the snap-table instance](../cpp-map/07-storage-formats/mds-tables.md#339-hook-contract-and-the-snap-table-instance)
- **Code:** `src/mds/SnapServer.cc:308-327 (SnapServer::_notify_prep)`, `src/mds/MDSTableServer.cc:111-149 (MDSTableServer::handle_prepare)`, `src/mds/SnapClient.cc:221-361 (SnapClient::get_snaps)`, `src/mds/SnapServer.cc:80-123 (SnapServer::encode_server_state)`, `src/mds/Server.cc:1828-1841 (Server::reconnect_gather_finish)`
- **Notes:** Prepared snap-table transactions are only ever committed: no rollback call exists on the client side, so a request that fails or is killed after AGREE leaves its tid pending on rank 0 (and shipped in every NOTIFY_PREP) until the requesting rank fails and resolve rolls it back (inferred from code). `TABLE_OP_CREATE` with only an ino (`prepare_create_realm`) has no caller. Offline tools `cephfs-table-tool <rank> reset/show snap` and `cephfs-data-scan` (via `force_update`) read and rewrite this object in the C++ encoding. The generic 2PC, its journal events and recovery are owned by 06-journal-and-storage ([mds-tables](../cpp-map/07-storage-formats/mds-tables.md)). See also F-STORE-078 (table 2PC protocol), F-STORE-080 (NOTIFY_PREP), F-STORE-077 (table object).

<a id="f-snap-003"></a>

#### F-SNAP-003 · Deleted snapshots reported to the monitor (MRemoveSnaps)

*Lets OSDs trim clones of deleted snapshots by having rank 0 ask the monitor to mark the snap ids removed in the OSDMap.*

When a `DESTROY` commits, both the snapid and the destroy seq are added to `need_to_purge` for every current data pool. `SnapServer::check_osd_map` runs on the active rank 0 on every OSDMap update (forced) and on `tick` when the table version changed; for each pool it treats "pool gone" or `pg_pool_t::is_removed_snap` as purged and sends the rest in `MRemoveSnaps` (`MSG_REMOVE_SNAPS = 90`, `map<int32 pool, vector<snapid>>`). The monitor needs the sender to be capable of `osd pool rmsnap` (granted by `profile mds`; otherwise the message is dropped with no reply), adds the ids to `new_removed_snaps`, sets `FLAG_SELFMANAGED_SNAPS`, raises the pool `snap_seq` if lower, and echoes the message after commit to octopus+ peers. On the echo `SnapServer::handle_remove_snaps` journals `ETableServer SERVER_UPDATE` and drops the entries. The MDS never tracks clone trimming itself.

- **Class:** POSIX `extension` · exposure `cluster-wire`, `on-disk` · relied on by mon, osd
- **Legacy parts** (old-daemons): SnapServer::check_osd_map treating a snapid as purged because pg_pool_t::is_removed_snap reports it (code comment: 'only needed for support during upgrades from pre-octopus'), together with the monitor behaviours it relied on: adding removed snaps to pg_pool_t::removed_snaps while require_osd_release is below octopus, and sending no echo to peers without SERVER_OCTOPUS. Monitors at this commit add removed snaps to pg_pool_t::removed_snaps only while require_osd_release is below octopus, and an OSD at this commit cannot boot unless require_osd_release is at least squid. For self-managed snapshots is_removed_snap tests only removed_snaps, so in a current cluster need_to_purge is cleared by the monitor's echo, which every MDS with SERVER_OCTOPUS receives. The pool-gone check and the re-sending of MRemoveSnaps on OSDMap changes are current. Evidence: `src/mds/SnapServer.cc:376-378 (SnapServer::check_osd_map)`, `src/mds/SnapServer.cc:392-397 (SnapServer::check_osd_map)`, `src/osd/osd_types.cc:1757-1763 (pg_pool_t::is_removed_snap)`, `src/mon/OSDMonitor.cc:4457-4462 (OSDMonitor::prepare_remove_snaps)`, `src/mon/OSDMonitor.cc:4476-4480 (OSDMonitor::prepare_remove_snaps)`, `src/mon/OSDMonitor.cc:4426-4430 (OSDMonitor::preprocess_remove_snaps)`, `src/mon/OSDMonitor.cc:3573-3578 (OSDMonitor::preprocess_boot)`, `src/mds/SnapServer.cc:423-451 (SnapServer::handle_remove_snaps)`.
- **Gates:** mon cap `osd pool rmsnap` (in `profile mds`, `src/mon/MonCap.cc:206`)
- **Map:** [snapshots.md › 3.7 rmsnap and removal of snapshot data](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#37-rmsnap-and-removal-of-snapshot-data) · [snapshots.md › 7.4 Monitor and OSDs](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#74-monitor-and-osds)
- **Code:** `src/mds/SnapServer.cc:365-417 (SnapServer::check_osd_map)`, `src/mds/SnapServer.cc:423-451 (SnapServer::handle_remove_snaps)`, `src/mon/OSDMonitor.cc:4386-4480 (OSDMonitor::preprocess_remove_snaps)`, `src/messages/MRemoveSnaps.h:40-50`, `src/mds/MDSRank.cc:3887-3892 (MDSRankDispatcher::handle_osd_map)`
- **Notes:** Pending purges are rank-0 state persisted in `mds_snaptable`; requests are re-sent on every OSDMap change until echoed. A data pool removed from the filesystem keeps its snapshots unless the pool itself is deleted.

<a id="f-snap-004"></a>

#### F-SNAP-004 · MDS data writes carry the realm SnapContext

*Makes OSDs preserve snapshot clones when the MDS itself writes or deletes file data.*

`SnapRealm::get_snap_context()` returns `{seq = cached_seq, snaps = effective snapshot set in descending order}`. The MDS attaches it to its own data-pool operations: stray file purge, truncation (`_truncate_inode` and the truncate write finish) and data uninlining. OSDs clone an object for a snapshot only when a write arrives with a SnapContext that includes that snapshot, so an MDS write with an older context would put post-snapshot data into the snapshot.

- **Class:** POSIX `extension` · exposure `cluster-wire` · relied on by osd
- **Legacy parts** (deprecated-admin): Data uninlining: MDCache::uninline_data_work, dispatched by scrub for each file inode, writes the inode's first data object with the realm SnapContext only when the inode still carries inline data. uninline_data_work responds before any RADOS write when the inode has no inline data. Inline data exists only on inodes created while the deprecated inline_data file system flag is enabled (default off, FS_INLINE_DATA_DEPRECATED health warning when on), and no baseline peer sets that flag. Stray purge and truncation, the other two users of the realm SnapContext, are current. Evidence: `src/mds/MDCache.cc:13503-13508 (MDCache::uninline_data_work)`, `src/mds/MDCache.cc:13527-13532 (MDCache::uninline_data_work)`, `src/mds/ScrubStack.cc:294 (ScrubStack::kick_off_scrubs)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mds/MDSMap.h:647 (MDSMap::inline_data_enabled)`, `src/mon/FSCommands.cc:471-479 (inline_data)`, `src/mds/MDSMap.cc:616-622 (MDSMap::get_health_checks)`.
- **Map:** [snapshots.md › 3.2 Snap realms](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#32-snap-realms) · [snapshots.md › 4.3 Invariants](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#43-invariants)
- **Code:** `src/mds/SnapRealm.cc:182-197 (SnapRealm::get_snap_context)`, `src/mds/StrayManager.cc:123-131 (StrayManager::purge)`, `src/mds/MDCache.cc:6657-6662 (MDCache::_truncate_inode)`, `src/mds/MDCache.cc:13530-13532`

### Snap realms and client notification

Snap realms define which snapshots cover which inodes. Clients rebuild the same sets from snap traces and use them as RADOS SnapContexts for data writes, so the realm model is part of the client contract.

<a id="f-snap-006"></a>

#### F-SNAP-006 · Snap realms and effective snapshot sets

*Groups inodes into subtrees that share one set of snapshots, which is what clients and the MDS turn into SnapContexts.*

A `SnapRealm` is attached to an inode with a persistent `sr_t` (`seq, created, last_created, last_destroyed, current_parent_since, snaps, past_parent_snaps, flags, last_modified, change_attr`); every other inode belongs to the realm of its nearest ancestor that has one. Realms exist for root and `~mdsN` (seq 1), a dummy in-memory global realm (ino 3), a directory receiving its first snapshot, a directory getting a quota, a subvolume root, a hard-linked inode outside subvolumes, and an inode that leaves its realm with snapshot history. Effective set = own snaps + `SnapClient.filter(past_parent_snaps)` (drops deleted ones) + parent snaps with id `>= current_parent_since`; the global realm's set is the whole table. Caches are rebuilt lazily when `seq` grows, the global `last_destroyed` changes, or `last_modified`/`change_attr` change, so any rmsnap anywhere invalidates every realm. Deleting the last snapshot, removing a quota or clearing the subvolume flag never removes the realm.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, tools
- **Legacy parts** (old-data, old-daemons): (1) The legacy past_parents map: sr_t still encodes it (empty) and decodes it, and only cephfs-data-scan reads it. (2) The sr_t decode branches for struct_v below 8: flags default to SNAPDIR_VISIBILITY when struct_v < 6, the bit is forced on for struct_v 6-7, and fields added in v5 and v7 get defaults. No code in src/mds fills or reads past_parents at this commit; Luminous 12.2.13 filled it, and Mimic 13.2.0 replaced it with past_parent_snaps. sr_t v8 was introduced by commit f40f4cee940 (first tag v21.0.0) and C++ MDS 20.2.0 still encodes v7, so encodings below v8 reach the decoder only from data or ranks of earlier C++ MDS releases. The realm model, effective snapshot sets and cache rebuild rules are current. Evidence: `src/mds/snap.cc:224 (sr_t::encode)`, `src/mds/snap.cc:245 (sr_t::decode)`, `src/mds/snap.cc:246-266 (sr_t::decode)`, `src/mds/snap.h:113 (sr_t::past_parents)`, `src/tools/cephfs/DataScan.cc:1370-1372`, `v12.2.13:src/mds/CInode.cc:462-463`, `v20.2.0:src/mds/snap.cc:143 (sr_t::encode)`. Also: search: grep -rn past_parents over src/mds finds only snap.cc, snap.h and a comment at CInode.cc:629.
- **Map:** [snapshots.md › 3.2 Snap realms](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#32-snap-realms) · [snapshots.md › 4.1 Encodings](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#41-encodings) · [snapshots.md › 4.3 Invariants](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#43-invariants)
- **Code:** `src/mds/SnapRealm.cc:86-119 (SnapRealm::build_snap_set)`, `src/mds/SnapRealm.cc:121-168 (SnapRealm::check_cache)`, `src/mds/snap.h:88-127 (sr_t)`, `src/mds/CInode.cc:3352 (CInode::find_snaprealm)`, `src/mds/MDCache.cc:10217-10223 (MDCache::create_global_snaprealm)`
- **Notes:** `sr_t` is `ENCODE_START(8, 4)` and lives in the inode snap blob in the parent dirfrag. The legacy `past_parents` map (v1) is decoded and re-encoded but not consulted by current MDS code; only `cephfs-data-scan` reads it. Decoding forces `SNAPDIR_VISIBILITY` on for `struct_v < 8`. `doc/dev/cephfs-snapshots.rst` does not mention the quota and subvolume realm cases.

<a id="f-snap-007"></a>

#### F-SNAP-007 · Realm membership of caps and realm split on creation

*Tracks which inodes and clients hold caps in each realm so that a new realm can tell clients exactly which inodes moved into it.*

Each realm keeps `inodes_with_caps` and `client_caps`; an inode joins its containing realm when it gets its first cap and leaves when the last cap goes; a new cap starts with `client_follows = inode.first - 1`. Opening a new realm calls `parent->split_at(new)`: for a directory it moves every open child realm under the new inode and walks *all* of the parent's `inodes_with_caps`, moving those that descend from the new inode (cost linear in cap-holding inodes of the parent realm, under `mds_lock`); for a non-directory, an mdsdir parent or an unlinked directory only the inode's own caps move. The moved inodes and child realms become the `split_inos`/`split_realms` lists of the `MClientSnap` SPLIT message. Cap grants in replies name the containing realm ino; readdir and lssnap give no caps on entries whose realm differs from the directory's realm, because only the directory realm's trace is sent.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [snapshots.md › 3.2 Snap realms](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#32-snap-realms) · [snapshots.md › 3.3 Snap traces seen by clients](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#33-snap-traces-seen-by-clients) · [snapshots.md › 6. Concurrency and mds_lock](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/SnapRealm.cc:338-411 (SnapRealm::split_at)`, `src/mds/CInode.cc:3595-3657 (CInode::add_client_cap)`, `src/mds/CInode.cc:3321-3336 (CInode::open_snaprealm)`, `src/mds/CInode.cc:4027-4031 (CInode::encode_inodestat)`
- **Notes:** Invariant: an inode with caps is in exactly one realm's `inodes_with_caps`, equal to `find_snaprealm()` after every realm change. The kclient WARNs if a cap grant names a realm it has not received in a trace (`linux/fs/ceph/caps.c:721-735`).

<a id="f-snap-008"></a>

#### F-SNAP-008 · Snap trace contents and client interpretation

*What the realm elements of a snap trace contain and how clients compute snapshot sets from them.*

See F-PROTO-085 for the wire format and its selection by bit 19. Contents: one element per realm from the target realm up to the root realm or the global realm; the global realm is the single element {ino 3, created 0, parent 0, parent_since 0, seq = global newest seq, my_snaps = every snapshot}. Clients build seq = max(realm seq, parent seq) and snaps = parent snaps >= parent_since + my_snaps + prior_parent_snaps; the kclient replaces realm fields only when seq grows, libcephfs also when change_attr grows. Malformed traces make the kclient fence itself (self-blocklist).

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** CEPHFS_FEATURE_NEW_SNAPREALM_INFO (bit 19) selects the new format
- **Map:** [snapshots.md › 3.3 Snap traces seen by clients](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#33-snap-traces-seen-by-clients) · [snapshots.md › 7.1 Linux kernel client](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#71-linux-kernel-client) · [snapshots.md › 7.2 libcephfs, ceph-fuse, NFS-Ganesha](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#72-libcephfs-ceph-fuse-nfs-ganesha)
- **Code:** `src/mds/SnapRealm.cc:450-509 (SnapRealm::build_snap_trace)`, `src/common/snap_types.cc:10-85 (SnapRealmInfo::encode)`, `src/mds/Server.cc:12429-12442 (Server::get_snap_trace)`, `src/mds/Server.cc:2512-2521 (Server::set_trace_dist)`, `linux/fs/ceph/snap.c:779 (ceph_update_snap_trace)`
- **Notes:** libcephfs chooses its decoder from the MDS's advertised features (session OPEN) while the MDS chooses the encoder from the client's, so both must agree. See also F-SNAP-006 (effective snapshot sets), F-SNAP-009 (MClientSnap).

<a id="f-snap-009"></a>

#### F-SNAP-009 · Per-realm MClientSnap notification (always op SPLIT)

*Tells every client holding caps in a realm subtree that the realm changed, and which inodes and child realms moved into a new realm.*

`do_realm_invalidate_and_update_notify` walks the realm and its open child realms, invalidating caches, and sends each client with caps in any of them one `MClientSnap` (`CEPH_MSG_CLIENT_SNAP = 0x312`) with `op = CEPH_SNAP_OP_SPLIT` (3), `split = realm ino` and the realm's trace. `split_inos` (inodes with caps in the realm) and `split_realms` (open child realms) are filled only when the requested op is SPLIT (a newly created realm, or a realm made parent-global by a link); for create, destroy, rename, metadata, subvolume, visibility and existing-quota changes the lists are empty. Wire layout: packed `ceph_mds_snap_head {le32 op, le64 split, le32 num_split_inos, le32 num_split_realms, le32 trace_len}` (24 bytes), then the inos, the realm inos and `trace_len` bytes of trace, with no length prefixes. Op `CREATE` (1) is never sent. Messages are sent counted on the session connection, so the creating client receives its notification before the mksnap reply; other clients are notified asynchronously.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [snapshots.md › 3.6 mksnap](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#36-mksnap) · [snapshots.md › 5.2 Messages](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#52-messages) · [snapshots.md › 8. Edge cases, failure handling and gotchas](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDCache.cc:10225-10279 (MDCache::do_realm_invalidate_and_update_notify)`, `src/messages/MClientSnap.h:48-66`, `src/include/ceph_fs.h:1042-1047`, `linux/fs/ceph/snap.c:1016 (ceph_handle_snap)`, `src/client/Client.cc:5636-5640 (Client::handle_snap)`
- **Notes:** Kernel: for SPLIT, inodes in `split_inos` move to realm `split` unless their current realm's `created` is greater than the new realm's (race guard against splits from another MDS), child realms are re-parented, then the trace is applied and cap snaps are queued. libcephfs asserts `head.split != 0` and that the first trace element's ino equals `head.split`. A rollback realm merge is sent as SPLIT to the parent realm. See also F-PROTO-094 (encoding).

<a id="f-snap-010"></a>

#### F-SNAP-010 · Global realm broadcast on every snap-table operation

*Informs every client session of every snapshot create, destroy or update anywhere in the filesystem, so hard-linked inodes (parented to the global realm) see new snapshots.*

After a snap-table op commits, `send_snap_update` with a table tid runs `notify_global_snaprealm_update(op)` on the requesting rank, and every other rank runs it on receipt of `MMDSSnapUpdate`. For every open or stale client session it sends `MClientSnap{op = DESTROY for rmsnap else UPDATE, split = 3, empty lists, trace = global realm}`, whose `my_snaps` lists every snapshot in the filesystem. Realm-only changes (link, setvxattr, rename) do not trigger it.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [snapshots.md › 3.6 mksnap](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#36-mksnap) · [snapshots.md › 5.2 Messages](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#52-messages) · [snapshots.md › 6. Concurrency and mds_lock](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/MDCache.cc:10348-10362 (MDCache::notify_global_snaprealm_update)`, `src/mds/MDCache.cc:10281-10307 (MDCache::send_snap_update)`
- **Notes:** Cost: one message per session on every active rank per snapshot op, each carrying the full snapshot list (page section 6, serialization point 7). libcephfs applies the trace with `flush = (op != DESTROY)`. See also F-PROTO-094 (encoding).

<a id="f-snap-011"></a>

#### F-SNAP-011 · Hard-link global realm (PARENT_GLOBAL)

*Makes every later snapshot in the filesystem cover a hard-linked inode, because such an inode has several parent directories.*

On the first link of an inode outside subvolumes (local `_link_local` or peer `handle_peer_link_prep`), the target gets a realm flagged `PARENT_GLOBAL` (1) whose parent is the global realm: `current_parent_since = global seq + 1`, the primary parent realm's snapshots since the old `current_parent_since` are copied into `past_parent_snaps`, and `last_destroyed` is reused to hold the primary parent's "since" (`mark_snaprealm_global`, `record_snaprealm_parent_dentry`). Unlinking a remote dentry adds that dentry's directory-realm snapshots since the dentry's `first` to `past_parent_snaps`; unlinking back to one primary link clears the flag (`clear_snaprealm_global`, `seq` = global newest seq). After commit the realm is sent to replicas (`send_snap_update` with tid 0) and clients get a SPLIT notification. Inside a subvolume (target realm has a subvolume ino) no global realm is used, because links cannot cross the subvolume boundary.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, internal
- **Map:** [snapshots.md › 3.10 Realm changes caused by namespace operations](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#310-realm-changes-caused-by-namespace-operations) · [rstats-quotas-and-subvolumes.md › 3.13 What the subvolume flag changes inside the MDS](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#313-what-the-subvolume-flag-changes-inside-the-mds)
- **Code:** `src/mds/Server.cc:8034-8040 (Server::_link_local)`, `src/mds/Server.cc:8286-8295 (Server::handle_peer_link_prep)`, `src/mds/CInode.cc:596-612 (CInode::mark_snaprealm_global)`, `src/mds/CInode.cc:656-676 (CInode::record_snaprealm_parent_dentry)`
- **Notes:** Over-inclusive but correct for multiple parents: the realm covers every filesystem snapshot taken after the link. Detailed link/unlink mechanics are on [namespace-mutations](../cpp-map/03-request-path/namespace-mutations.md) (owned by 04-namespace-and-posix-operations).

<a id="f-snap-012"></a>

#### F-SNAP-012 · Past-parent snapshots on rename and unlink

*Keeps an inode covered by the snapshots of the directory it left when it is renamed to another realm or unlinked into a stray directory.*

On a primary rename across realms or an unlink of a primary link into a stray directory, if the inode already has a realm or the old realm's newest seq is at least the inode's `oldest_snap`, `record_snaprealm_past_parent` gives it a new srnode (`seq = created = old realm newest seq` when it had none), adds the old parent's snapshots since `current_parent_since` to `past_parent_snaps`, and sets `current_parent_since = global seq + 1`. The snapids are filtered against the snap table at use time, so deleted ones drop out. They reach clients as `prior_parent_snaps` in the trace. Rename xlocks the `snaplock` of the source inode and of an overwritten target; unlink/rmdir xlock the unlinked inode's `snaplock`.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, internal
- **Map:** [snapshots.md › 3.10 Realm changes caused by namespace operations](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#310-realm-changes-caused-by-namespace-operations) · [snapshots.md › 3.2 Snap realms](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#32-snap-realms)
- **Code:** `src/mds/CInode.cc:631-654 (CInode::record_snaprealm_past_parent)`, `src/mds/Server.cc:8679-8687`, `src/mds/Server.cc:9615-9636`
- **Notes:** `project_rstat_inode_to_frag` notes that rstats of old parents after a directory rename are propagated incompletely (FIXME). Stray directories with past-parent snapshots are not purged until those snapshots are deleted (F-SNAP-028).

### Snapshot operations

Creating, removing and renaming snapshots through the virtual `.snap` directory, snapshot metadata, and the policies that limit snapshot creation.

<a id="f-snap-013"></a>

#### F-SNAP-013 · Create a snapshot (MKSNAP, mkdir in .snap)

*Lets clients and mgr modules take a read-only point-in-time snapshot of a directory subtree.*

`CEPH_MDS_OP_MKSNAP` (0x01400): `filepath{ino = dir, path = "/<name>"}`, data = `SnapPayload`. Checks in order: MDSMap epoch (wait), `allow_snaps` flag (`EPERM`), inode cached and auth (`ESTALE` / forward), quarantine parking, `ENOTDIR`, system dir other than root (`EPERM`), uid range (`EPERM`), xlock dir `snaplock` + rdlock ancestor snaplocks, `check_access(MAY_WRITE + MAY_SNAPSHOT)` (`EACCES`), inside a subvolume but not its root (`EPERM`), `mds_max_snaps_per_dir` (`EMLINK`), own name exists (`EEXIST`), name empty, longer than 240 bytes (`NAME_MAX - 15`) or starting with `_` (`EINVAL`); before that, a read-only MDS gives `EROFS` and a full metadata pool requires the `f` auth cap. Then table `CREATE`; project the dir inode (`ctime = request stamp`, `rctime`, `rstat.rsnaps++`) and srnode (`created = seq = last_created = snapid`, insert `snaps[snapid]`, `last_modified`, `change_attr++`); journal `EUpdate("mksnap")` with the table tid; no early reply. On commit: apply (opens and splits a new realm), table commit, `MMDSSnapUpdate` to every rank, global UPDATE to all sessions, per-realm SPLIT notifications, then the reply with target = dir inode at the new snapid and no trace.

- **Class:** POSIX `extension` · exposure `client-wire`, `mds-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, mgr, mds-ranks
- **Gates:** MDSMap flag `allow_snaps` (F-SNAP-018); mds_max_snaps_per_dir (F-SNAP-019); mds_snap_min_uid / mds_snap_max_uid (F-SNAP-020); MDS auth cap flag `s` (MAY_SNAPSHOT), see [auth-and-mds-caps](../cpp-map/01-cluster-integration/auth-and-mds-caps.md#34-the-mds-cap-language)
- **Map:** [snapshots.md › 3.6 mksnap](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#36-mksnap) · [snapshots.md › 5.1 Client request ops](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#51-client-request-ops) · [snapshots.md › 7.3 cephfs-mirror and mgr modules](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#73-cephfs-mirror-and-mgr-modules)
- **Code:** `src/mds/Server.cc:11588-11676 (Server::handle_client_mksnap)`, `src/mds/Server.cc:11717-11733 (Server::handle_client_mksnap)`, `src/mds/Server.cc:11750-11773 (Server::_mksnap_finish)`, `src/mds/SnapClient.cc:149-187 (SnapClient::prepare_create)`, `linux/fs/ceph/dir.c:1124-1135 (ceph_mkdir)`
- **Notes:** mksnap is not a data barrier: other clients learn of the snapshot asynchronously, and data they dirtied before the notification lands in the snapshot; consistent multi-client snapshots need [quiesce](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md). Two stamps exist: the table copy uses the MDS stamp, the realm copy the client request stamp. With a full pool and no `f` cap the request gets a double response (`EACCES` then `ENOSPC`, inferred from code). A directory not in cache gets `ESTALE` instead of being loaded. Clients turn `mkdir` in the snapdir into MKSNAP (kernel and libcephfs); mgr/volumes and snap_schedule use libcephfs mkdir in `.snap`. See also F-ADM-097 (mgr/volumes snapshots).

<a id="f-snap-014"></a>

#### F-SNAP-014 · Remove a snapshot (RMSNAP, rmdir in .snap)

*Deletes one of a directory's own snapshots and starts reclaiming its data.*

`CEPH_MDS_OP_RMSNAP` (0x01401). Checks: auth inode (`ESTALE`/forward), quarantine parking, `ENOTDIR`, uid range (`EPERM`), name empty or starting with `_` (`EINVAL`: inherited snapshots cannot be removed from a descendant), no own snapshot of that name (`ENOENT`), xlock `snaplock` + ancestor snaplocks, `MAY_WRITE + MAY_SNAPSHOT` (`EACCES`). Table `DESTROY` returns a new seq; the srnode drops the snapshot and sets `seq = last_destroyed = new seq`, `last_modified`, `change_attr++`; the inode gets `ctime`, `rctime`, `rstat.rsnaps--`; journal `EUpdate("rmsnap")`. On commit: apply, table commit, `MMDSSnapUpdate` (DESTROY), global DESTROY notification, per-realm SPLIT notification, reply (`tracei = dir`, `snapid = removed id`), then `purge_stale_snap_data` on the directory. No subvolume, `mds_max_snaps_per_dir`, `allow_new_snaps`, system-directory or MDSMap-epoch checks apply, so snapshots can be removed after `allow_new_snaps` is turned off.

- **Class:** POSIX `extension` · exposure `client-wire`, `mds-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, mgr, mds-ranks
- **Gates:** mds_snap_min_uid / mds_snap_max_uid; MDS auth cap flag `s`
- **Map:** [snapshots.md › 3.7 rmsnap and removal of snapshot data](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#37-rmsnap-and-removal-of-snapshot-data) · [snapshots.md › 5.1 Client request ops](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#51-client-request-ops)
- **Code:** `src/mds/Server.cc:11788-11883 (Server::handle_client_rmsnap)`, `src/mds/Server.cc:11885-11909 (Server::_rmsnap_finish)`, `src/mds/SnapServer.cc:215-258 (SnapServer::_commit)`
- **Notes:** Data removal continues through F-SNAP-003; metadata of the deleted snapshot is cleaned lazily (F-SNAP-028). mgr/snap_schedule prunes scheduled snapshots with rmdir.

<a id="f-snap-015"></a>

#### F-SNAP-015 · Rename a snapshot (RENAMESNAP)

*Renames one of a directory's own snapshots in place.*

`CEPH_MDS_OP_RENAMESNAP` (0x01403): `filepath` = destination name, `filepath2` = source name; both must name the same directory ino (`EINVAL`). Checks: auth inode, quarantine, `ENOTDIR`, uid range, source empty or `_`-prefixed (`EINVAL`), source not an own snapshot (`ENOENT`), destination empty or `_`-prefixed (`EINVAL`), destination exists (`EEXIST`), xlock snaplock + ancestors, `MAY_WRITE + MAY_SNAPSHOT`. Table `UPDATE`; the srnode renames `snaps[snapid]` and bumps `last_modified` and `change_attr` but not `seq`; inode `ctime`/`rctime`; journal `EUpdate("renamesnap")`. On commit: `MMDSSnapUpdate` (UPDATE), global UPDATE, per-realm SPLIT notifications, reply with `tracei = dir` at that snapid.

- **Class:** POSIX `extension` · exposure `client-wire`, `mds-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, mds-ranks
- **Map:** [snapshots.md › 3.8 renamesnap](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#38-renamesnap) · [snapshots.md › 5.1 Client request ops](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#51-client-request-ops)
- **Code:** `src/mds/Server.cc:11921-12030 (Server::handle_client_renamesnap)`, `src/mds/Server.cc:12034-12054 (Server::_snap_mutate_generic_finish)`, `linux/fs/ceph/dir.c:1496-1501 (ceph_rename)`
- **Notes:** The destination length is not checked against the 240-byte limit. Because `seq` does not change, the kclient never notices the rename through traces and re-reads names with LSSNAP. The kernel sends RENAMESNAP only when both dentries are in the same snapdir (`EXDEV` locally for different snapids).

<a id="f-snap-016"></a>

#### F-SNAP-016 · Snapshot metadata at creation (SnapPayload)

*Stores a key/value map supplied by the creator on the snapshot, read back by cephfs-mirror and libcephfs users.*

The MKSNAP request data is decoded as `SnapPayload{map<string,string> metadata}` (`ENCODE_START(1, 1)`) and stored in the realm's `SnapInfo.metadata`; a decode failure is ignored (old clients send an xattr buffer there; the kernel sends ACL/security xattrs). The map is returned as `InodeStat.snap_metadata` for any inode viewed at that snapid. libcephfs exposes it as `ceph_mksnap(path, name, mode, snap_metadata*, nr)` and `ceph_get_snap_info`. The snap table copy of `SnapInfo` never carries metadata (table ops do not transport it).

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by libcephfs, cephfs-mirror
- **Map:** [snapshots.md › 3.9 Snapshot metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#39-snapshot-metadata) · [snapshots.md › 4.1 Encodings](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#41-encodings)
- **Code:** `src/mds/Server.cc:11696-11706 (Server::handle_client_mksnap)`, `src/messages/MClientRequest.h:57-70 (SnapPayload)`, `src/mds/snap.cc:33-59 (SnapInfo::encode)`
- **Notes:** cephfs-mirror stores `primary_snap_id` on the remote snapshots it creates (`src/tools/cephfs_mirror/PeerReplayer.cc:3144-3145`).

<a id="f-snap-017"></a>

#### F-SNAP-017 · Modify snapshot metadata (SNAP_METADATA op)

*Lets libcephfs callers add, replace or remove one metadata key of an existing snapshot; cephfs-mirror uses it for sync checkpoints.*

`CEPH_MDS_OP_SNAP_METADATA` (0x1406): path is the snapshot path (`#dirino` + `/<snapname>`), resolved with `rdlock_path_pin_ref` (auth pin only, no locks); data `{string key, string value, u32 op_flag}` with no versioning (decode error `EBADMSG`). Checks: `ENOTDIR`, uid range (`EPERM`), name empty or `_`-prefixed (`EINVAL`), not an own snapshot (`ENOENT`). `op_flag`: `CREATE` (1) inserts or updates, `CREATE | EXCL` (3) fails if the key exists, `REMOVE` (4) fails if absent, anything else fails; failure is `EINVAL` before projection. Then table `UPDATE` (same name), inode `ctime`/`rctime`, srnode `snaps[snapid].stamp = op stamp`, the metadata change, `last_modified`, `change_attr++`; journal `EUpdate("snap_metadata_op")`; finish as renamesnap.

- **Class:** POSIX `extension` · exposure `client-wire`, `mds-wire`, `on-disk` · relied on by libcephfs, cephfs-mirror, mds-ranks
- **Gates:** mds_snap_min_uid / mds_snap_max_uid
- **Map:** [snapshots.md › 3.9 Snapshot metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#39-snapshot-metadata) · [snapshots.md › 5.1 Client request ops](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#51-client-request-ops) · [snapshots.md › 8. Edge cases, failure handling and gotchas](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Server.cc:12056-12170 (Server::handle_client_snap_md_op)`, `src/mds/snap.cc:80-118 (SnapInfo::will_md_op_succeed)`, `src/tools/cephfs_mirror/Checkpoint.cc:123-166`
- **Notes:** Findings (inferred from code, not reproduced): there is no `check_access` call, so MDSAuthCaps (path restriction, `s` flag) are not consulted; no `snaplock` is taken; a request without data acts as `CREATE | EXCL` of an empty key; every op rewrites the snapshot's stamp, so `ceph.snap.btime` changes; an `ENOENT`/`EINVAL` after AGREE, or the `ENOENT` after `project_inode` (`src/mds/Server.cc:12146-12149`), leaves the prepared table tid (and in the latter case the projection) in place. See also F-ADM-105 (mirroring checkpoints).

<a id="f-snap-018"></a>

#### F-SNAP-018 · allow_new_snaps filesystem flag

*Lets the administrator disable creation of new snapshots for a filesystem.*

`ceph fs set <fs> allow_new_snaps <bool>` sets or clears the MDSMap flag `CEPH_MDSMAP_ALLOW_SNAPS`. MKSNAP first waits for the client's MDSMap epoch if newer, then returns `EPERM` when the flag is clear. RMSNAP, RENAMESNAP and SNAP_METADATA ignore the flag. New filesystems get it set (`CEPH_MDSMAP_DEFAULTS`).

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by admin, mon
- **Gates:** mon command `fs set <fs> allow_new_snaps <bool>`
- **Map:** [snapshots.md › 3.6 mksnap](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#36-mksnap) · [snapshots.md › 5.7 Admin and monitor interfaces](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#57-admin-and-monitor-interfaces)
- **Code:** `src/mon/FSCommands.cc:551-570`, `src/mds/Server.cc:11588-11600 (Server::handle_client_mksnap)`, `src/include/ceph_fs.h:298-299`

<a id="f-snap-019"></a>

#### F-SNAP-019 · Per-directory snapshot limit (mds_max_snaps_per_dir)

*Caps the number of snapshots visible in a directory's realm when a new one is created.*

MKSNAP returns `EMLINK` when the directory already has a realm and `realm->get_snaps().size() >= mds_max_snaps_per_dir` (default 100, min 0, max 4096, runtime). The count includes inherited snapshots (parent and past-parent), and the check is skipped for a directory without a realm. mgr/snap_schedule reads this MDS option with `get_foreign_ceph_option('mds', ...)` and keeps at most `mds_max_snaps_per_dir - 1` scheduled snapshots per path.

- **Class:** POSIX `extension` · exposure `config`, `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, admin
- **Gates:** mds_max_snaps_per_dir (default 100)
- **Map:** [snapshots.md › 5.5 Configuration options](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#55-configuration-options) · [snapshots.md › 8. Edge cases, failure handling and gotchas](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#8-edge-cases-failure-handling-and-gotchas) · [snapshots.md › 3.11 Subvolumes and snapshot visibility](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#311-subvolumes-and-snapshot-visibility)
- **Code:** `src/mds/Server.cc:11659-11663 (Server::handle_client_mksnap)`, `src/mds/Server.h:622-624`, `src/common/options/mds.yaml.in:1607-1618`, `src/pybind/mgr/snap_schedule/fs/schedule_client.py:352-355`
- **Notes:** Quirk: under an ancestor with 100 or more snapshots, a directory that already has a realm (quota, subvolume flag, earlier snapshot) gets `EMLINK` for its first own snapshot, while a directory without a realm is allowed.

<a id="f-snap-020"></a>

#### F-SNAP-020 · Snapshot uid range (mds_snap_min_uid, mds_snap_max_uid)

*Restricts which caller uids may create, remove, rename or annotate snapshots.*

MKSNAP, RMSNAP, RENAMESNAP and SNAP_METADATA return `EPERM` when the request caller uid is outside `[mds_snap_min_uid, mds_snap_max_uid]` (defaults 0 and 4294967294). It is independent of the MDS auth cap `s` flag (`MAY_SNAPSHOT`), which MKSNAP/RMSNAP/RENAMESNAP check separately through `check_access`.

- **Class:** POSIX `operational` · exposure `config` · relied on by admin
- **Gates:** mds_snap_min_uid (default 0); mds_snap_max_uid (default 4294967294)
- **Map:** [snapshots.md › 3.6 mksnap](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#36-mksnap) · [snapshots.md › 5.5 Configuration options](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#55-configuration-options)
- **Code:** `src/mds/Server.cc:11588-11676 (Server::handle_client_mksnap)`, `src/common/options/mds.yaml.in:1376-1391`
- **Notes:** See also F-ADM-013 (s flag).

### Reading snapshots

Path encoding, listing and lookup of snapshots, and what a client sees when it reads inodes at a snapshot id.

<a id="f-snap-021"></a>

#### F-SNAP-021 · Snapshot path encoding, traversal and LOOKUPSNAP

*Resolves `.snap/<name>/...` paths sent by clients to inodes and dentries as of that snapshot.*

`.snap` is never a dentry on the MDS; clients synthesise the snapdir inode with vino `(dir ino, CEPH_SNAPDIR)` and its name is client configuration (kernel `snapdirname`, libcephfs `client_snapdir`, default `.snap`). On the wire a snapshot path is `filepath{ino = real directory, path = "/<snapname>[/rest]"}`: an empty first component means "enter the snapdir" (empty components elsewhere are `EINVAL`). `path_traverse` resolves the name with `realm->resolve_snapname(name, dir ino)` (plain name for the directory's own snapshots, `_<name>_<owner ino decimal>` for inherited ones, last `_` as separator; `ENOENT` if unknown), sets the request snapid, and continues with `CDir::lookup(name, snap)` (dentry with `first <= snap <= last`), never creating null dentries and taking no path locks. `CEPH_MDS_OP_LOOKUPSNAP` (0x00400) is handled as a getattr; the reply target is the directory inode at the snapshot's snapid. The kernel sends an ordinary LOOKUP for the snapdir name first and splices its local snapdir only on `ENOENT`, then uses LOOKUPSNAP inside it; libcephfs never asks the MDS about the snapdir name.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [snapshots.md › 3.5 The .snap directory and snapshot paths](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#35-the-snap-directory-and-snapshot-paths) · [snapshots.md › 7.1 Linux kernel client](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#71-linux-kernel-client)
- **Code:** `src/mds/MDCache.cc:8528-8555 (MDCache::path_traverse)`, `src/mds/SnapRealm.cc:254-306 (SnapRealm::resolve_snapname)`, `src/mds/Server.cc:2858-2862 (Server::dispatch_client_request)`, `src/mds/CDir.cc:328-342 (CDir::lookup)`, `linux/fs/ceph/dir.c:727 (ceph_handle_snapdir)`
- **Notes:** The MDS does not reserve the snapdir name: a real dentry with that name is returned by LOOKUP and hides the snapdir for the kernel (inferred from `ceph_handle_snapdir`), while libcephfs hides the real dentry instead. `rdlock_path_pin_ref` forces `want_auth` for snapshot paths unless the caller passes `no_want_auth`. The path is relative to the nearest non-snapshot ancestor; path_traverse accepts an empty component only as the first one and only for a request. Paths through .snap are traversed without RDLOCK_PATH/RDLOCK_SNAP but with WANT_AUTH (unless the caller passes no_want_auth: readdir, readdir_snapdiff, file_blockdiff), and mutating helpers return -EROFS before traversing. See also F-PROTO-072 (path encoding), F-NS-029.

<a id="f-snap-022"></a>

#### F-SNAP-022 · List snapshots (LSSNAP, readdir of .snap)

*Returns the snapshots visible in a directory, with names that distinguish own from inherited snapshots.*

`CEPH_MDS_OP_LSSNAP` (0x00402): auth inode (`ESTALE`/forward), quarantine parking, `ENOTDIR`, rdlock of the snaplock of the directory and all ancestors, `check_access(MAY_READ)`. Entries are every snapshot of the directory's realm with id `>= oldest_snap` of the directory, sorted by snapid; names are plain for the directory's own snapshots and `_<name>_<ino>` otherwise. Pagination: `path2` is the last returned name, resolved to a snapid; limits `max_entries` (0 = all) and `max_bytes` (0 = 512 KiB + `max_xattr_size`). Each entry carries the name, `LeaseStat{mask CEPH_LEASE_VALID, duration_ms 0xffffffff, seq 0, alternate_name}` and the directory's `InodeStat` at that snapid; the dirstat header is empty; flags are `FRAG_END` when exhausted plus `FRAG_COMPLETE` if no offset was given. The reply carries the directory realm's trace.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, mgr
- **Map:** [snapshots.md › 3.5 The .snap directory and snapshot paths](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#35-the-snap-directory-and-snapshot-paths) · [snapshots.md › 5.1 Client request ops](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#51-client-request-ops) · [snapshots.md › 7.3 cephfs-mirror and mgr modules](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#73-cephfs-mirror-and-mgr-modules)
- **Code:** `src/mds/Server.cc:11475-11573 (Server::handle_client_lssnap)`, `src/mds/SnapRealm.cc:199-252 (SnapRealm::get_snap_info)`, `src/mds/snap.cc:158-168 (SnapInfo::get_long_name)`, `linux/fs/ceph/export.c:455 (__get_snap_name)`
- **Notes:** cephfs-mirror ignores names starting with `_`. The kernel NFS export pages LSSNAP and matches `InodeStat.snapid` to map a snapid to a name. Own snapshot names cannot start with `_`, which is why long names never clash with own names.

<a id="f-snap-023"></a>

#### F-SNAP-023 · Read-only snapshot semantics and snapshot caps

*Makes everything below `.snap` immutable and gives clients read-only caps on snapshot inodes.*

Namespace mutations and attribute changes on snapshot paths return `EROFS` (`rdlock_path_xlock_dentry`, `rdlock_two_paths_xlock_destdn`, setattr, setlayout, setxattr, removexattr). `open` of a snapshot file for writing returns `EROFS`; otherwise it rdlocks the snapped inode's `filelock` (which waits while a snapflush is pending) and issues `ceph_caps_for_mode(mode)` without a `Capability` object. Inode stats at a snapid get read-only caps with `seq = mseq = realm = 0`: every cap allowed for the inode type, masked by the current lock state only for a head multiversion inode or one that still has caps. Only head inodes have `Capability` objects.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [snapshots.md › 3.4 Copy-on-write of metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#34-copy-on-write-of-metadata) · [snapshots.md › 3.5 The .snap directory and snapshot paths](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#35-the-snap-directory-and-snapshot-paths) · [snapshots.md › 4.3 Invariants](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#43-invariants)
- **Code:** `src/mds/Server.cc:4712-4797 (Server::handle_client_open)`, `src/mds/CInode.cc:4181-4206 (CInode::encode_inodestat)`, `src/mds/Server.cc:3977 (Server::rdlock_path_xlock_dentry)`, `linux/fs/ceph/inode.c:1331-1362`
- **Notes:** The `cap_id` and `wanted` fields of the snapshot reply cap are left uninitialized; the kernel only ORs the caps into `i_snap_caps` for snapped inodes. Clients also refuse mutations in a snapshot locally with `EROFS` (`EXDEV` for a rename between different snapids). Reading a snapshot version of a file that was being written waits until writers flush their cap snaps (snapflush, owned by [capabilities](../cpp-map/02-client-protocol/capabilities.md#311-snapshots-snapflush)). See also F-NS-017 (EROFS gates).

<a id="f-snap-024"></a>

#### F-SNAP-024 · Snapshot attributes in InodeStat (snap_btime, snap_metadata)

*Reports, for an inode viewed at a snapid, the inode version as of that snapshot plus the snapshot's creation time and metadata.*

`encode_inodestat(..., snapid)` picks the `old_inodes` entry covering `snapid` when one exists; on a non-auth rank the stat is marked not valid, limiting issued caps to `CEPH_STAT_CAP_INODE`, because `old_inodes` is only valid on the auth. It adds `snap_btime` (`SnapInfo.stamp`) and `snap_metadata` from `get_snap_info(snapid)`. Clients expose `snap_btime` as `ceph.snap.btime` (kernel and libcephfs), and libcephfs `ceph_get_snap_info` returns the snapid and metadata.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, mgr
- **Map:** [snapshots.md › 3.4 Copy-on-write of metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#34-copy-on-write-of-metadata) · [snapshots.md › 7.1 Linux kernel client](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#71-linux-kernel-client) · [snapshots.md › 5.7 Admin and monitor interfaces](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#57-admin-and-monitor-interfaces)
- **Code:** `src/mds/CInode.cc:3976-4023 (CInode::encode_inodestat)`, `src/client/Client.cc:14056-14075 (Client::get_snap_info)`, `linux/fs/ceph/xattr.c:237-309`
- **Notes:** Two stamps: the realm copy (client request stamp, rewritten by every SNAP_METADATA op) is shown for the directory realm's own snapshots; the table copy (MDS stamp at lock time) for snapshots reached only through `past_parent_snaps`. mgr/volumes reads `ceph.snap.btime` for snapshot info.

<a id="f-snap-025"></a>

#### F-SNAP-025 · Snapshot inode lookup by ino (LOOKUPINO with snapid)

*Resolves NFS file handles of snapshot inodes exported by the kernel NFS server.*

`CEPH_MDS_OP_LOOKUPINO` (0x00104) with `args.lookupino.snapid` set goes to `_lookup_snap_ino`: a snapped `CInode` with `first <= snapid` is found with `lookup_snap_inode`, else the cached head inode if not purging and `has_snap_data(snapid)`; a found inode is answered after `check_access(MAY_READ)` at that snapid. A cached head without that snapshot data gets `ESTALE` if it is a directory or no `parent` was given. With `parent` and `hash`, the MDS opens the parent (`open_ino` if not cached), rdlocks its `dirfragtreelock`, fetches the dirfrag for the hash and retries, finally `ESTALE`; without `parent` it runs `open_ino(ino)` and retries (`ENOENT`/`ENODATA` become `ESTALE`).

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient
- **Map:** [snapshots.md › 3.5 The .snap directory and snapshot paths](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#35-the-snap-directory-and-snapshot-paths) · [snapshots.md › 4.2 Request argument unions used by snapshot ops](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#42-request-argument-unions-used-by-snapshot-ops)
- **Code:** `src/mds/Server.cc:4539-4615 (Server::_lookup_snap_ino)`, `linux/fs/ceph/export.c:211-292 (__snapfh_to_dentry)`
- **Notes:** The generic LOOKUPINO op is owned by 04-namespace-and-posix-operations; this entry covers only the snapid branch. libcephfs/Ganesha reach snapshots through `.snap` path lookups instead.

### Metadata copy-on-write and cleanup

How the MDS keeps older versions of dentries and inodes for snapshots, and how it discards them after snapshots are deleted. Cap-level snapflush is owned by [capabilities](../cpp-map/02-client-protocol/capabilities.md#311-snapshots-snapflush).

<a id="f-snap-026"></a>

#### F-SNAP-026 · Metadata copy-on-write (first/last ranges, snapped dentries, old_inodes)

*Preserves dentries and inode attributes as they were at each snapshot, so snapshot views show the namespace and attributes of snapshot time.*

Every dentry and inode has a `[first, last]` snapid range (`last = CEPH_NOSNAP` for head). Snapped dentries live in the same dirfrag object as head dentries under omap key `<name>_<last in lowercase hex>` (head: `<name>_head`); there are no per-snapshot directory objects. New dentries get `first = global newest seq + 1` and new inodes inherit it. Before journaling a change to a dentry or its primary inode, `journal_cow_dentry(follows)` checks whether a realm snapshot lies in `[first, follows]`: single-version inodes (plain files) are copied into a separate snapped `CInode` keyed by `(ino, last)` with a snapped primary dentry; multiversion inodes (directories, inodes with a realm, `nlink > 1`, or existing `old_inodes`) keep history in `old_inodes: map<last, {first, inode, xattrs}>`; remote dentries get snapped remote dentries. COW of a written file transfers snapflush obligations to the snapped inode. Copies are journaled in ordinary `EMetaBlob`s (no dedicated event type).

- **Class:** POSIX `extension` · exposure `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, cephfs-mirror, tools
- **Gates:** mds_go_bad_corrupt_dentry, mds_abort_on_newly_corrupt_dentry (reaction to `first > last` or `first > global seq + 1`)
- **Map:** [snapshots.md › 3.4 Copy-on-write of metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#34-copy-on-write-of-metadata) · [snapshots.md › 4.1 Encodings](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#41-encodings) · [snapshots.md › 4.3 Invariants](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#43-invariants)
- **Code:** `src/mds/MDCache.cc:1679-1810 (MDCache::journal_cow_dentry)`, `src/mds/MDCache.cc:1573-1677 (MDCache::cow_inode)`, `src/mds/CInode.h:487-492 (CInode::is_multiversion)`, `src/mds/mdstypes.cc:772-783 (dentry_key_t::encode)`, `src/mds/CDentry.cc:805-827 (CDentry::check_corruption)`
- **Notes:** Invariants: `first <= last`, `first <= global newest seq + 1`, and for one name the dentries with different `last` partition the snapid line. Dentries violating the first two are treated as corrupt on load, commit and scrub. The snapflush protocol that follows COW is owned by 03-caps-leases-and-locking ([capabilities](../cpp-map/02-client-protocol/capabilities.md#311-snapshots-snapflush)). See also F-CACHE-011 (cache identity), F-STORE-024 (journal images), F-CAPS-023 (cap state).

<a id="f-snap-027"></a>

#### F-SNAP-027 · Ancestor pre-COW and the COW sequence (global or subvolume-local)

*Preserves directory stats as of each snapshot when a descendant change updates ancestors, and decides which seq counts as "already snapshotted".*

`predirty_journal_parents` (and scatter write-behind) call `pre_cow_old_inode()` on every ancestor whose dirstat/rstat it updates; it copies the head into `old_inodes` when `first <= follows`. `follows` is the global newest seq when `mds_use_global_snaprealm_seq_for_subvol = true` (default, runtime). When false, the realm's own newest seq is used for realms inside a subvolume or whose newest seq is `<= 1` (never affected by a snapshot), else the global seq. Independently, every new dentry anywhere takes `first = global newest seq + 1`, so any snapshot anywhere raises it.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_use_global_snaprealm_seq_for_subvol (default true)
- **Map:** [snapshots.md › 3.4 Copy-on-write of metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#34-copy-on-write-of-metadata) · [rstats-quotas-and-subvolumes.md › 3.13 What the subvolume flag changes inside the MDS](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#313-what-the-subvolume-flag-changes-inside-the-mds) · [snapshots.md › 6. Concurrency and mds_lock](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/CInode.cc:3222-3258 (CInode::pre_cow_old_inode)`, `src/mds/MDCache.cc:2399 (MDCache::predirty_journal_parents)`, `src/common/options/mds.yaml.in:1619-1630`

<a id="f-snap-028"></a>

#### F-SNAP-028 · Lazy trimming of deleted-snapshot metadata

*Removes snapped dentries, snapped inodes and old_inodes that no live snapshot covers, without walking the tree at rmsnap time.*

Each dirfrag `fnode` has `snap_purged_thru`. When a dirfrag is fetched with `snap_purged_thru < realm.last_destroyed`, snapped dentries covering no live snapshot are skipped while loading, their omap keys are recorded in `stale_items` and the dirfrag is marked dirty so the next commit deletes them; `snap_purged_thru` advances when no snapped dentries of the dirfrag were cached. On commit, `try_trim_snap_dentry` removes dirty stale snapped dentries (and snapped inodes) with no other references. `CInode::purge_stale_snap_data` drops `old_inodes` entries covering no live snapshot (after rmsnap on the directory itself, for strays, and for realm-less inodes loaded from a lagging dirfrag). `prune_past_parent_snaps` drops deleted ids from strays' `past_parent_snaps`; a stray directory with past-parent snapshots is not purged, and a stray file with past-parent snaps and old inodes is truncated but its metadata kept.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal
- **Map:** [snapshots.md › 3.4 Copy-on-write of metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#34-copy-on-write-of-metadata) · [snapshots.md › 8. Edge cases, failure handling and gotchas](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/CDir.cc:2109-2121 (CDir::_omap_fetched)`, `src/mds/CDir.cc:851-877 (CDir::try_trim_snap_dentry)`, `src/mds/CDir.cc:2611-2652 (CDir::_omap_commit)`, `src/mds/CInode.cc:3278-3303 (CInode::purge_stale_snap_data)`, `src/mds/StrayManager.cc:478-545 (StrayManager::_eval_stray)`
- **Notes:** Dirfrags never fetched again keep stale snapped dentries. rmdir of a directory with own snapshots is refused (`ENOTEMPTY`) only by an unlocked pre-check that is skipped when the directory's `filelock` is held by a lock cache, and `_eval_stray` keeps a stray directory only for past-parent (not own) snapshots, so such a directory could be purged with its snapshots orphaned (inferred from code). Stray handling is owned by 09-background-services ([strays-and-purge](../cpp-map/11-background-services/strays-and-purge.md)).

### Snapshot diff for cephfs-mirror

Two request ops that exist for cephfs-mirror incremental sync: a directory diff between snapshots and a file block diff computed from RADOS clone information.

<a id="f-snap-029"></a>

#### F-SNAP-029 · Directory snapshot diff (READDIR_SNAPDIFF)

*Lists the entries of one dirfrag that differ between two snapshots, for cephfs-mirror incremental sync.*

`CEPH_MDS_OP_READDIR_SNAPDIFF` (0x01404): path = the directory as of snapshot A, `args.snapdiff = {frag, max_entries, max_bytes, flags, offset_hash, snap_other = B, mask}`, `path2` = last name. `EINVAL` if the two snapids are equal or either is `CEPH_NOSNAP`. Throttled like readdir (cap acquisition throttle), rdlocks `filelock` and `dirfragtreelock`, `check_access(MAY_READ)`, forwarded to the dirfrag auth. Dentries are walked in hash order: a directory dentry ending between the snapshots is reported deleted (at the older snapid), every other directory is reported as existing without comparison; a file is skipped when unchanged under `mask` (`CEPH_SNAPDIFF_{MODE,UID,GID,SIZE,NLINK,MTIME,ATIME,CTIME,BTIME}`, 0 = MTIME only), otherwise reported, waiting for pending snapflush first; a re-created name is reported as old deleted plus new existing. Each entry has name, lease and `InodeStat` at the newer snapid (existing) or older snapid (deleted); libcephfs returns that snapid in `stx_dev`. Entries for one name are never split across replies; the reply's directory inode is encoded at `snap_other`; flags add `HASH_ORDER`/`OFFSET_HASH` when requested.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by libcephfs, cephfs-mirror
- **Map:** [snapshots.md › 3.14 READDIR_SNAPDIFF (used by cephfs-mirror)](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#314-readdir_snapdiff-used-by-cephfs-mirror) · [snapshots.md › 4.2 Request argument unions used by snapshot ops](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#42-request-argument-unions-used-by-snapshot-ops) · [snapshots.md › 7.3 cephfs-mirror and mgr modules](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#73-cephfs-mirror-and-mgr-modules)
- **Code:** `src/mds/Server.cc:12255-12410 (Server::handle_client_readdir_snapdiff)`, `src/mds/Server.cc:12570-12935 (Server::build_snap_diff)`, `src/client/Client.cc:10612-10629`
- **Notes:** The op has the `0x1000` write bit, so it is refused with `EROFS` on a read-only MDS and recorded in completed requests. libcephfs leaves `max_entries`/`max_bytes` at 0 (MDS defaults: whole dirfrag, 512 KiB + `max_xattr_size`). APIs `ceph_open_snapdiff`/`ceph_readdir_snapdiff` (mask 0) and `..._snapdiff2` (explicit mask).

<a id="f-snap-030"></a>

#### F-SNAP-030 · File block diff between snapshots (FILE_BLOCKDIFF)

*Returns the file extents that changed between two snapshots of a file, computed from RADOS clone overlap data, for cephfs-mirror.*

`CEPH_MDS_OP_FILE_BLOCKDIFF` (0x01405): `path` = file in snapshot A, `path2` = same file in snapshot B, `args.blockdiff = {scan_idx, max_objects}`. Both paths are resolved without locks or auth pins; a non-regular file is `EINVAL`; the same `CInode` for both returns success with no blocks. Different layouts return one extent `[0, size of B)`. Otherwise the MDS issues `min(objects left, mds_file_blockdiff_max_concurrent_object_scans (16), max_objects)` concurrent `list_snaps` reads on `<ino>.<objno>` at `CEPH_SNAPDIR` in B's pool and accumulates the non-overlapping extents of every clone hop between the clones covering A's and B's `last`. Reply `reply_extra_bl` = `BlockDiff{int rval (1 = call again, 0 = done), u64 scan_idx, interval_set<u64> blocks}` (`ENCODE_START(1, 1)`); RADOS errors other than `ENOENT` are returned. The client loops while `rval == 1`.

- **Class:** POSIX `extension` · exposure `client-wire`, `cluster-wire` · relied on by libcephfs, cephfs-mirror, osd
- **Gates:** CEPHFS_FEATURE_BLOCKDIFF (23): libcephfs returns `EOPNOTSUPP` without it; mds_file_blockdiff_max_concurrent_object_scans (default 16); client_file_blockdiff_max_concurrent_object_scans (default 16)
- **Map:** [snapshots.md › 3.15 FILE_BLOCKDIFF (used by cephfs-mirror)](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#315-file_blockdiff-used-by-cephfs-mirror) · [snapshots.md › 4.1 Encodings](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#41-encodings)
- **Code:** `src/mds/Server.cc:12194-12253 (Server::handle_client_file_blockdiff)`, `src/mds/MDCache.cc:14801-14972 (MDCache::file_blockdiff)`, `src/mds/mdstypes.cc:1284-1300 (BlockDiff::encode)`, `src/client/Client.cc:10526-10587`
- **Notes:** Findings (inferred from code): `file_blockdiff` asserts `in1->last <= in2->last` on client-controlled input, and libcephfs does not order the snapshot names, so swapped arguments can crash the MDS; there is no `check_access`; the extent arithmetic assumes `stripe_count = 1`; `max_objects = 0` reports completion with no extents; hard-linked files resolve to the same head `CInode` and report no diffs. cephfs-mirror passes the previous snapshot as `path` and falls back to a full copy for new or small files.

### Multi-rank snapshot coordination and recovery

How snapshot state reaches other ranks and how client realm state is reconciled after an MDS failover. The recovery state machine itself is owned by 01-cluster-membership-and-ha.

<a id="f-snap-031"></a>

#### F-SNAP-031 · Realm state replication between ranks (MMDSSnapUpdate, snaplock payload)

*Keeps replica copies of realms on other ranks current and orders snapshot operations against in-flight requests across ranks.*

`MMDSSnapUpdate` (`MSG_MDS_SNAPUPDATE = 0x211`: `ino u64, snap_op s16, snap_blob` with the table tid in the header) goes, for snap-table ops, to every rank in state `>= resolve` except self, and, for realm-only changes (link, setvxattr, rename), only to the inode's replicas. Receivers call `notify_commit(tid)`, decode the srnode into their replica (from rejoin on), and notify their own clients (global UPDATE and per-realm SPLIT). The `snaplock` (ISNAP) lock payload also carries `version`, `ctime` and `encode_snap()` (srnode blob + `oldest_snap`); replica and export encodings carry the snap blob and `old_inodes`. An xlock of ISNAP (and IPOLICY) by a leader request waits, dropping all locks, while a replica rank is below `up:active` (`wait_for_cluster_recovered`), and requests keep their ISNAP/IPOLICY rdlocks past the early reply until safe, so a later mksnap waits for them to be journaled.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [snapshots.md › 3.12 Multi-MDS handling](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#312-multi-mds-handling) · [snapshots.md › 5.2 Messages](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#52-messages) · [snapshots.md › 6. Concurrency and mds_lock](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/MDCache.cc:10281-10307 (MDCache::send_snap_update)`, `src/mds/MDCache.cc:10309-10346 (MDCache::handle_snap_update)`, `src/mds/CInode.cc:2208-2229 (CInode::encode_lock_isnap)`, `src/mds/Locker.cc:274-306 (Locker::acquire_locks)`, `src/mds/Locker.cc:926-930 (Locker::handle_locks_for_early_reply)`
- **Notes:** Snapshot ops run on the directory's auth rank (`try_get_auth_inode` forwards). In resolve and reconnect a receiver only records the commit; in rejoin it queues the realm in `rejoin_pending_snaprealms` without client notification until `open_snaprealms`. Lock acquisition rules in general are owned by 03-caps-leases-and-locking.

<a id="f-snap-032"></a>

#### F-SNAP-032 · Snaprealm reconciliation at client reconnect and rejoin

*Brings clients' realm state up to date after an MDS failover and rebuilds pending snapflush expectations.*

Client reconnect messages report realms `{ino, seq, parent}` (`ceph_mds_snaprealm_reconnect`) and, per cap, `snaprealm` and `snap_follows`; the rank does not leave reconnect until its SnapClient is synced. During rejoin realms are queued in `rejoin_pending_snaprealms`; `open_snaprealms` then, per realm: sends `MClientSnap UPDATE` (split 0) with the realm trace to clients whose reconnected seq is older, rebuilds snapflush expectations from `snap_follows`, and for caps reconnected under a different realm adds the inode to a SPLIT message for that client; finally it broadcasts a global realm UPDATE to all sessions. Replay restores srnodes, `old_inodes`, `oldest_snap` and `need_snapflush` state from `EMetaBlob` full bits.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [snapshots.md › 3.13 Recovery](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#313-recovery) · [resolve-and-rejoin.md › 3.15 Snaprealm reconnect and snapflush rebuild (open_snap...](../cpp-map/09-recovery/resolve-and-rejoin.md#315-snaprealm-reconnect-and-snapflush-rebuild-open_snaprealms)
- **Code:** `src/mds/Server.cc:1705-1722 (Server::handle_client_reconnect)`, `src/mds/MDCache.cc:5982-6056 (MDCache::open_snaprealms)`, `src/mds/MDCache.cc:6151-6165 (MDCache::finish_snaprealm_reconnect)`, `src/mds/MDCache.cc:5730-5748 (MDCache::prepare_realm_split)`, `src/mds/MDCache.cc:5651-5694 (MDCache::rebuild_need_snapflush)`
- **Notes:** Possible `SPLIT` with `split = 0` (inferred, not observed): if one client has both a stale realm seq and a cap reconnected under a different realm, `prepare_realm_split` reuses the UPDATE message (split 0), changes its op to SPLIT and leaves `split = 0` and `split_realms` empty; libcephfs asserts `head.split != 0`, the kernel would create a realm with ino 0. The reconnect message format and the recovery state machine are owned by 02-client-sessions-and-protocol and 01-cluster-membership-and-ha. See also F-CLU-092 (rejoin step), F-PROTO-061 (realm records in the reconnect message).

### Directory statistics (dirstat and rstat)

Per-directory entry statistics (dirstat/fragstat) and recursive subtree statistics (rstat): what they count, how changes travel up the tree eagerly and lazily, and how exact they are when read. Scatterlock state machines are owned by [lock-types-and-state-machines](../cpp-map/05-locking/lock-types-and-state-machines.md#312-scattergather-of-dirstat-rstat-and-dirfragtree).

<a id="f-snap-033"></a>

#### F-SNAP-033 · Directory entry statistics (dirstat / fragstat)

*Counts the direct entries of each directory and tracks its mtime/change_attr, which clients present as directory `st_nlink`, `st_size`, mtime/ctime and the NFSv4 change attribute.*

`frag_info_t {version, mtime, change_attr, nfiles, nsubdirs}` is kept per dirfrag (`fnode.fragstat`, `accounted_fragstat`) and per directory inode (`inode.dirstat`, protected by the directory's `filelock`). `nfiles` counts head non-directory primary dentries plus every head remote (hard-link) dentry; `nsubdirs` counts head primary directory dentries. Every entry add/remove updates the dirfrag fragstat in the same journal event (`mtime = op stamp`, `change_attr++`, `nfiles`/`nsubdirs += linkunlink`), and level 1 of `predirty_journal_parents` folds it into the directory inode when the inode is auth and pinnable (`mtime = ctime = dirstat.mtime`, `change_attr++` when the fold raises them); otherwise it is folded by filelock scatter-gather. Wire encoding `frag_info_t` v3: `version, mtime, nfiles, nsubdirs, change_attr`. A `GETATTR` with `Fs` rdlocks the filelock, which makes the value exact.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [rstats-quotas-and-subvolumes.md › 3.1 The two statistic families and where they live](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#31-the-two-statistic-families-and-where-they-live) · [rstats-quotas-and-subvolumes.md › 3.2 What each field counts](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#32-what-each-field-counts) · [rstats-quotas-and-subvolumes.md › 3.7 Consistency guarantees (what is exact, and when)](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#37-consistency-guarantees-what-is-exact-and-when)
- **Code:** `src/include/cephfs/types.h:83 (frag_info_t)`, `src/mds/MDCache.cc:2286-2305 (MDCache::predirty_journal_parents)`, `src/mds/MDCache.cc:2406-2415 (MDCache::predirty_journal_parents)`, `src/mds/mdstypes.cc:46 (frag_info_t::encode)`, `linux/fs/ceph/inode.c:3189-3224 (ceph_getattr)`
- **Notes:** Clients compute directory `st_nlink = 2 + nsubdirs` (kclient when the MDS reports nlink 1; libcephfs aborts if a directory reports nlink other than 0 or 1). Directory `change_attr` is bumped once per fold, not once per entry change: on a single rank every change folds at once, while changes in frags on other ranks collapse into one bump per gather (relevant to NFS-Ganesha). Fragment split/merge (`steal_dentry`) counts a remote dentry with `d_type DT_DIR` in `nsubdirs`. See also F-NS-043 (parent updates on link and unlink).

<a id="f-snap-034"></a>

#### F-SNAP-034 · Recursive subtree statistics (rstat)

*Maintains recursive bytes, file, directory and snapshot counts plus newest ctime for every directory, exposed as `ceph.dir.r*` vxattrs, directory `st_size` and quota usage.*

`nest_info_t {version, rctime, rbytes, rfiles, rsubdirs, rsnaps}` is kept per inode (`rstat`, `accounted_rstat` = part already added to the parent dirfrag; files included) and per dirfrag (`fnode.rstat`, `accounted_rstat`), protected by the directory's `nestlock`. Files: `rbytes` = logical size (symlinks: target length), `rfiles` = 1. Directories: `rsubdirs` counts the directory itself (empty dir = 1), `rsnaps` = own snapshot count + children (mksnap `++`, rmsnap `--`), `rctime` = max over subtree and never decreases. A hard link counts once, at the primary dentry; data kept only by snapshots is not counted; unlinked (stray) files leave the user tree immediately. `rbytes` grows from setattr size, truncate, cap flushes (growth only) and max_size updates. Wire encoding `nest_info_t` v3: `version, rbytes, rfiles, rsubdirs, ranchors(=0), rsnaps, rctime`.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, tools
- **Map:** [rstats-quotas-and-subvolumes.md › 3.1 The two statistic families and where they live](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#31-the-two-statistic-families-and-where-they-live) · [rstats-quotas-and-subvolumes.md › 3.2 What each field counts](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#32-what-each-field-counts) · [rstats-quotas-and-subvolumes.md › 5.7 Virtual xattrs (who serves them)](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#57-virtual-xattrs-who-serves-them)
- **Code:** `src/include/cephfs/types.h:144 (nest_info_t)`, `src/mds/MDCache.cc:1909 (MDCache::_project_rstat_inode_to_frag)`, `src/mds/Server.cc:7734 (Server::handle_client_mkdir)`, `src/mds/Locker.cc:4017-4022 (Locker::_update_cap_fields)`, `src/mds/mdstypes.cc:119 (nest_info_t::encode)`
- **Notes:** Clients synthesise `ceph.dir.rentries/rfiles/rsubdirs/rsnaps/rbytes/rctime` from cached `InodeStat` values; the MDS GETVXATTR serves none of them (`ENODATA`), and `setxattr ceph.dir.rctime` (forwarded by the kclient) gets `EINVAL`. Directory `st_size` = `rbytes` with the kclient `rbytes` mount option or libcephfs `client_dirsize_rbytes` (default true); mgr/volumes relies on this for `bytes_used` and `resize --no_shrink`, and reads `ceph.dir.rbytes`/`rentries` for clone progress. `cephfs-shell` du/quota commands use the same vxattrs.

<a id="f-snap-035"></a>

#### F-SNAP-035 · Eager upward stat propagation (predirty_journal_parents)

*Pushes a mutation's dirstat and rstat changes up the ancestor chain inside the same journal event where locks allow.*

Every stat-changing mutation calls `predirty_journal_parents(mut, blob, in, parent, flags, linkunlink)` (`PREDIRTY_PRIMARY`, `PREDIRTY_DIR`; `PREDIRTY_SHALLOW` is accepted but ignored). For each level it projects the parent dirfrag fnode, applies the fragstat change (first level only) and the child's rstat delta (`rstat - accounted_rstat`, `-accounted_rstat` on unlink, `+rstat` on link), then projects the directory inode and folds the frag delta in. It stops (marking the scatterlocks updated and queueing the inode on the segment's `dirty_dirfrag_*` lists) at a non-auth or unpinnable inode, when `nestlock`/`versionlock` cannot be wrlocked, or, above level 1, when the ancestor was propagated less than `mds_dirstat_min_interval` (1 s) ago. If a child's delta cannot be pushed it is marked dirty-rstat on the parent dirfrag. All projected fnodes and ancestor inodes go into the same `EMetaBlob`; each projected ancestor gets `pre_cow_old_inode` and `broadcast_quota_to_client`.

- **Class:** POSIX `supporting` · exposure `on-disk`, `config` · relied on by internal
- **Gates:** mds_dirstat_min_interval (dev, default 1 s)
- **Map:** [rstats-quotas-and-subvolumes.md › 3.3 Eager propagation: MDCache::predirty_journal_parents](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#33-eager-propagation-mdcachepredirty_journal_parents) · [rstats-quotas-and-subvolumes.md › 3.4 Moving deltas: inode to dirfrag, dirfrag to inode, an...](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#34-moving-deltas-inode-to-dirfrag-dirfrag-to-inode-and-snapshots)
- **Code:** `src/mds/MDCache.cc:2224 (MDCache::predirty_journal_parents)`, `src/mds/MDCache.cc:2347-2387 (MDCache::predirty_journal_parents)`, `src/mds/MDCache.cc:1836 (MDCache::project_rstat_inode_to_frag)`, `src/mds/Locker.cc:1947-1979 (Locker::wrlock_try)`
- **Notes:** Result: the directory holding the change is exact at once on a single rank; each higher ancestor may lag by `mds_dirstat_min_interval` plus the nudge path. `PREDIRTY_SHALLOW` users (rename source, peer side of a remote link) still propagate to the root.

<a id="f-snap-036"></a>

#### F-SNAP-036 · Deferred stat propagation (dirty rstat, updated scatterlocks, tick and nudge)

*Completes stat propagation that the eager path could not finish, within a bounded delay.*

Pending state is kept as: a child with `STATE_DIRTYRSTAT` on its parent `CDir::dirty_rstat_inodes`; a dirty `nestlock`/`filelock` queued on `Locker::updated_scatterlocks` with an update stamp; and the inode on `LogSegment::dirty_dirfrag_{dir,nest,dirfragtree}`. `Locker::scatter_tick` runs from the rank tick every `mds_tick_interval` (5 s) in clientreplay/active/stopping (skipped while the beacon is laggy) and nudges locks dirty for at least `mds_scatter_nudge_interval` (5 s), then flushes the log. On the auth, `scatter_nudge` cycles the lock (MIX or LOCK, then SYNC) so the gather runs `scatter_writebehind`; a replica sends `MLock(LOCK_AC_NUDGE)` to the auth, which force-nudges and flushes its log. Dirty-rstat children in frags on a rank that is not the inode's auth are assimilated only when a later scatter delivers a new version (or the child is mutated again), so they need an extra cycle.

- **Class:** POSIX `internal` · exposure `mds-wire`, `config` · relied on by mds-ranks, internal
- **Gates:** mds_scatter_nudge_interval (default 5 s); mds_tick_interval (default 5 s)
- **Map:** [rstats-quotas-and-subvolumes.md › 3.5 Lazy propagation: dirty rstat, updated scatterlocks, ...](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#35-lazy-propagation-dirty-rstat-updated-scatterlocks-tick-nudge-journal-expiry) · [rstats-quotas-and-subvolumes.md › 3.7 Consistency guarantees (what is exact, and when)](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#37-consistency-guarantees-what-is-exact-and-when)
- **Code:** `src/mds/CInode.cc:446-463 (CInode::mark_dirty_rstat)`, `src/mds/Locker.cc:5381 (Locker::mark_updated_scatterlock)`, `src/mds/Locker.cc:5404 (Locker::scatter_nudge)`, `src/mds/Locker.cc:5517-5540 (Locker::scatter_tick)`, `src/mds/CDir.cc:1259-1302 (CDir::assimilate_dirty_rstat_inodes)`
- **Notes:** `scatter_nudge` requeues on frozen, freezing or ambiguous-auth parents ("starvation prone"); export and fragmentation keep frags stale across several gathers. See also F-CAPS-071 (scatter nudging), F-CAPS-070 (writebehind).

<a id="f-snap-037"></a>

#### F-SNAP-037 · Scatter-gather of dirstat and rstat across dirfrags and ranks

*Folds per-dirfrag stat deltas into the directory inode when its dirfrags are spread over several ranks or the eager path stopped.*

IFILE (dirstat) and INEST (rstat) scatterlocks carry version epochs: a frag is in sync when `accounted_*.version == inode version`. On gather the inode auth (`finish_scatter_gather_update` from `scatter_writebehind`) bumps the version, adds `current - accounted` of every frag whose accounted version is `version - 1` (others are skipped as stale), assimilates dirty-rstat children of updatable frags, resyncs updatable frags' accounted values, calls `broadcast_quota_to_client`, and journals `EUpdate("scatter_writebehind")` with upward propagation. On scatter (entering MIX, or on replicas when a lock message arrives) each frag auth journals `EUpdate("lock ifile|inest accounted scatter stat update")` setting its accounted values to the new version. Replica frag stats travel to the auth in `MLock` LOCKACK/SYNCACK payloads (`encode_lock_inest` v1: inode version or dirty flag, inode rstat, per frag `{frag, first, rstat, accounted_rstat, dirty_old_rstat}`; `encode_lock_ifile` v2 analogous plus inode times/size fields).

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks, internal
- **Map:** [rstats-quotas-and-subvolumes.md › 3.6 Scatter-gather of dirstat and rstat across dirfrags a...](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#36-scatter-gather-of-dirstat-and-rstat-across-dirfrags-and-ranks) · [lock-types-and-state-machines.md › 3.12 Scatter/gather of dirstat, rstat and dirfragtree](../cpp-map/05-locking/lock-types-and-state-machines.md#312-scattergather-of-dirstat-rstat-and-dirfragtree)
- **Code:** `src/mds/CInode.cc:2624 (CInode::finish_scatter_gather_update)`, `src/mds/CInode.cc:2521 (CInode::finish_scatter_update)`, `src/mds/CInode.cc:2073 (CInode::encode_lock_inest)`, `src/mds/Locker.cc:5260 (Locker::scatter_writebehind)`, `src/mds/Locker.cc:5352-5363 (Locker::scatter_eval)`
- **Notes:** `scatter_eval` keeps INEST in MIX when replicated and in LOCK otherwise ("keep INEST writable at all times"). Stat versions only advance by gather, never by individual writes. The lock state machines are owned by 03-caps-leases-and-locking. See also F-CAPS-070 (lock-side protocol).

<a id="f-snap-038"></a>

#### F-SNAP-038 · Journal segment expiry waits for stat folding

*Ensures that dirfrag stat deltas recorded in a journal segment are folded into their directory inodes before that segment is trimmed.*

`LogSegment::try_to_expire` calls `scatter_nudge(lock, gather)` for every inode on the segment's `dirty_dirfrag_dir`, `dirty_dirfrag_nest` and `dirty_dirfrag_dirfragtree` lists; an inode leaves those lists only when its lock flush finishes (`ScatterLock::finish_flush` then `CInode::clear_dirty_scattered`), i.e. after the write-behind that folded the stats committed (on a replica, after `LOCK_AC_LOCKFLUSHED`). Replay of a dirty dir lump with `rstat != accounted_rstat` (or fragstat mismatch), subtree import, replica scatter updates and repair re-mark the lock updated and re-queue the inode. A later re-dirtying moves the inode to the newer segment.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal
- **Map:** [rstats-quotas-and-subvolumes.md › 3.5 Lazy propagation: dirty rstat, updated scatterlocks, ...](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#35-lazy-propagation-dirty-rstat-updated-scatterlocks-tick-nudge-journal-expiry) · [rstats-quotas-and-subvolumes.md › 4. Data structures and invariants](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#4-data-structures-and-invariants) · [rstats-quotas-and-subvolumes.md › 5.2 Journal events](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#52-journal-events)
- **Code:** `src/mds/journal.cc:200-216 (LogSegment::try_to_expire)`, `src/mds/journal.cc:1325-1340 (EMetaBlob::replay)`, `src/mds/CDir.cc:3020-3028 (CDir::decode_import)`, `src/mds/CInode.cc:2447-2461 (CInode::clear_dirty_scattered)`
- **Notes:** This couples unrelated subtrees through the rank journal (page section 6, serialization point 7). Segment expiry in general is owned by 06-journal-and-storage. See also F-CAPS-071, F-STORE-016.

<a id="f-snap-039"></a>

#### F-SNAP-039 · rstat reads routed to the auth rank (GETATTR CEPH_STAT_RSTAT)

*Lets clients read the most recently folded recursive statistics of a directory by asking its authoritative rank.*

A `GETATTR` whose mask contains `CEPH_STAT_RSTAT` (the same bit as `CEPH_CAP_FILE_WREXTEND`) sets `want_auth`, so the request is forwarded to the inode's auth rank. No `nestlock` rdlock is taken, so the reply carries whatever the auth has folded so far; no client request rdlocks `nestlock`. Clients send these (and `Xs` getattrs) to the auth themselves (`ceph_try_to_choose_auth_mds`) and apply directory rstat only from replies carrying `CEPH_CAP_FLAG_AUTH` or a newer version.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [rstats-quotas-and-subvolumes.md › 3.7 Consistency guarantees (what is exact, and when)](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#37-consistency-guarantees-what-is-exact-and-when) · [rstats-quotas-and-subvolumes.md › 5.1 Messages](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#51-messages) · [rstats-quotas-and-subvolumes.md › 7. Externally visible contract](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:4272-4290 (Server::handle_client_getattr)`, `linux/fs/ceph/inode.c:2984-2987 (ceph_try_to_choose_auth_mds)`, `linux/fs/ceph/inode.c:1215-1226 (ceph_fill_inode)`
- **Notes:** Inode rstat is guaranteed exact only after a nestlock rdlock gather, which only scrub/repair takes; otherwise it is eventually consistent (each level above the first may lag by `mds_dirstat_min_interval` plus at least `mds_scatter_nudge_interval` per deferred level, more across ranks).

<a id="f-snap-040"></a>

#### F-SNAP-040 · Snapshot rstats (mds_snap_rstat)

*Optionally keeps recursive statistics of snapshotted directory versions up to date.*

With `mds_snap_rstat = true`, rstat deltas are also propagated for older snapshot intervals: sparse `CDir::dirty_old_rstat` entries keyed by `last`, `CInode::dirty_old_rstats`, and `old_inodes` entries split as needed by `project_rstat_frag_to_inode`. With the default `false`, `_project_rstat_inode_to_frag` stops after the head interval ("drop snapshots' rstats"), so a snapshot view shows rstats as they were COWed into `old_inodes`.

- **Class:** POSIX `extension` · exposure `config`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse
- **Gates:** mds_snap_rstat (default false)
- **Map:** [rstats-quotas-and-subvolumes.md › 3.4 Moving deltas: inode to dirfrag, dirfrag to inode, an...](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#34-moving-deltas-inode-to-dirfrag-dirfrag-to-inode-and-snapshots) · [snapshots.md › 3.4 Copy-on-write of metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#34-copy-on-write-of-metadata) · [rstats-quotas-and-subvolumes.md › 3.7 Consistency guarantees (what is exact, and when)](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#37-consistency-guarantees-what-is-exact-and-when)
- **Code:** `src/mds/MDCache.cc:1894-1905 (MDCache::project_rstat_inode_to_frag)`, `src/mds/MDCache.cc:1957-1959 (MDCache::_project_rstat_inode_to_frag)`, `src/mds/MDCache.cc:2029 (MDCache::project_rstat_frag_to_inode)`, `src/common/options/mds.yaml.in:1392-1398`

<a id="f-snap-041"></a>

#### F-SNAP-041 · Stat mismatch detection and in-place repair

*Detects inconsistent dirstat/rstat during normal propagation and repairs them silently, with optional assertions for debugging.*

During a gather, negative frag counts are clamped to 0 with a `clog` error; when all leaf frags are loaded and the sum differs, a `clog` error ("unmatched fragstat", "inconsistent rstat on inode"; silent under `STATE_REPAIRSTATS`) is written and the inode is overwritten with the frag sums. For a directory with a single dirfrag, `predirty_journal_parents` compares inode and frag after folding and on a size or `rbytes` mismatch logs a `clog` error and overwrites the inode dirstat/rstat with the frag's; the rstat copy drops the directory's own `rsubdirs = 1` and own snapshot count until the next gather (derived from code). These checks assert instead only with `mds_verify_scatter`; `CDir::check_rstats` (recompute from dentries) runs only with `mds_debug_scatterstat` or during scrub and asserts on mismatch outside scrub.

- **Class:** POSIX `internal` · exposure `metrics`, `config` · relied on by admin, internal
- **Gates:** mds_verify_scatter (dev, default false); mds_debug_scatterstat (dev, default false)
- **Map:** [rstats-quotas-and-subvolumes.md › 3.8 Verification and repair](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#38-verification-and-repair) · [rstats-quotas-and-subvolumes.md › 8. Edge cases, failure handling and gotchas](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#8-edge-cases-failure-handling-and-gotchas) · [scrub-and-damage.md › 3.10 Repair mode](../cpp-map/11-background-services/scrub-and-damage.md#310-repair-mode)
- **Code:** `src/mds/MDCache.cc:2418-2431 (MDCache::predirty_journal_parents)`, `src/mds/MDCache.cc:2451-2462 (MDCache::predirty_journal_parents)`, `src/mds/CInode.cc:2632-2746 (CInode::finish_scatter_gather_update)`, `src/mds/CDir.cc:238-314 (CDir::check_rstats)`
- **Notes:** Scrub-driven verification (`validate_disk_state`) and the repair ops `CEPH_MDS_OP_REPAIR_INODESTATS` (0x1505) and `CEPH_MDS_OP_REPAIR_FRAGSTATS` (0x1504), triggered by `scrub start ... repair`, are owned by 09-background-services ([scrub-and-damage](../cpp-map/11-background-services/scrub-and-damage.md)). `cephfs-data-scan` rebuilds dirstat/fragstat offline. See also F-BG-044 (scrub directory stat repair), F-BG-045 (scrub dirfrag stat repair).

### Quotas

Directory quotas on bytes and files. The MDS stores quotas, turns quota directories into realm roots and pushes usage; only clients enforce them.

<a id="f-snap-042"></a>

#### F-SNAP-042 · Quota vxattrs (ceph.quota, ceph.quota.max_bytes, ceph.quota.max_files)

*Lets administrators and mgr/volumes set byte and file limits on a directory.*

`SETXATTR`/`RMXATTR` with a name starting `ceph.quota` on a directory goes to `handle_client_setvxattr` on the auth (`ESTALE` if the inode is not cached), requiring the MDS cap `p` flag (`MAY_SET_VXATTR`). Non-directories, parse errors and invalid values give `EINVAL`. `max_bytes` takes a base-10 integer with an optional IEC suffix `K, M, G, T, P, E` (optionally `i`, optionally a trailing `B`, or a bare `B`), all powers of 1024; `max_files` a non-negative integer; `ceph.quota` takes `key=value` pairs separated by single spaces, where an empty value means "unchanged". 0 means no limit. RMXATTR of an unset quota succeeds; otherwise the value becomes `"0"`. Locks: xlock `policylock` (plus xlock `snaplock` when a realm is created, rdlock otherwise) and rdlock all ancestor snaplocks. An unchanged quota returns 0 without journaling. The quota is stored in `inode_t::quota` (`quota_info_t {i64 max_bytes, i64 max_files}`, versioned v1), journaled as `EUpdate("set vxattr layout")` with `ctime`, `change_attr++`, `rctime`, and replicated through the `policylock` payload; there is no early reply.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, tools, admin
- **Gates:** MDS auth cap flag `p`
- **Map:** [rstats-quotas-and-subvolumes.md › 3.9 Quotas: data model and how a quota is set](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#39-quotas-data-model-and-how-a-quota-is-set) · [rstats-quotas-and-subvolumes.md › 5.7 Virtual xattrs (who serves them)](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#57-virtual-xattrs-who-serves-them) · [attributes-and-xattrs.md › 3.8.4 Quotas](../cpp-map/03-request-path/attributes-and-xattrs.md#384-quotas)
- **Code:** `src/mds/Server.cc:6207-6273 (Server::parse_quota_vxattr)`, `src/mds/Server.cc:6456-6499 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:6948-6964 (Server::handle_client_setvxattr)`, `src/include/cephfs/types.h:326-375 (quota_info_t)`
- **Notes:** `removexattr ceph.quota` (combined name) with an active quota returns `EINVAL`: the value is rewritten to `"0"`, which the `key=value` grammar rejects; removing `max_bytes`/`max_files` individually works (derived from code). The MDS GETVXATTR serves no `ceph.quota*` names (`ENODATA`); clients answer reads from cached `InodeStat`. Other clients receive the new quota (F-SNAP-045) before the update is journaled, so a crash before commit leaves them with a quota that is not on disk. mgr/volumes copies both limits to clones only after the data copy.

<a id="f-snap-043"></a>

#### F-SNAP-043 · Quota directories become snaprealm roots

*Makes every directory with an enabled quota the root of its own snap realm, which is how clients find quota roots.*

When a quota is enabled on a directory without a (projected) srnode, the setxattr projects a new srnode with `created = seq = containing realm newest seq` under a `snaplock` xlock. On commit `C_MDS_inode_update_finish` applies it (opening the realm and splitting the parent realm's cap-holding inodes), sends `MMDSSnapUpdate` to replicas and an `MClientSnap` SPLIT with `split_inos`/`split_realms` to every client with caps in the realm, then replies. Removing a quota never closes the realm.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [rstats-quotas-and-subvolumes.md › 3.9 Quotas: data model and how a quota is set](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#39-quotas-data-model-and-how-a-quota-is-set) · [rstats-quotas-and-subvolumes.md › 3.10 How clients learn quotas and usage](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#310-how-clients-learn-quotas-and-usage) · [snapshots.md › 3.2 Snap realms](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#32-snap-realms)
- **Code:** `src/mds/Server.cc:6478-6493 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:5340-5376 (C_MDS_inode_update_finish)`, `src/mds/MDCache.cc:10225-10279 (MDCache::do_realm_invalidate_and_update_notify)`, `linux/fs/ceph/quota.c:214 (get_quota_realm)`
- **Notes:** Hard client requirement: if the directory is not its own realm root after a successful setxattr, the kclient and libcephfs return `EOPNOTSUPP`, the quota xattrs read as absent and the quota is never enforced (`linux/fs/ceph/xattr.c:1327-1335`, `src/client/Client.cc:15413-15417`).

<a id="f-snap-044"></a>

#### F-SNAP-044 · Quota and usage in InodeStat

*Delivers each directory's quota and current recursive usage to clients in every reply that includes the inode.*

`encode_inodestat` puts `quota` (from the projected inode when the client xlocks `policylock` or is loner), `dirstat.nfiles/nsubdirs`, `rbytes`, `rfiles`, `rsubdirs`, `rctime`, `rsnaps` (InodeStat v4) and `subvolume_id` (v9) into every reply trace and readdir entry, and records `cap->last_rbytes`/`last_rsize` for this client. The legacy encoding (no `REPLY_ENCODING` feature) sends the quota only with `CEPH_FEATURE_MDS_QUOTA`. The quota sub-struct must be `struct_compat == 1` or the kclient fails the reply with `-EIO`.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The legacy InodeStat encoding for sessions without bit 9: no version header, the quota only with CEPH_FEATURE_MDS_QUOTA (dir_layout, inline data, pool_ns, btime and change_attr likewise only with their connection bits), and no rsnaps, snap_btime, snap_metadata, fscrypt fields or subvolume_id. The 7.3 kernel, the CentOS Stream 8 and 9 samples and libcephfs advertise CEPHFS_FEATURE_REPLY_ENCODING (bit 9) and receive the versioned InodeStat v9. Upstream kernels advertise bit 9 from v5.1 and Ceph clients from 14.2.0. Kernels up to v5.0 and Ceph clients up to 13.2.x get the legacy form. Evidence: `src/mds/CInode.cc:4315 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4373-4427 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4416-4419 (CInode::encode_inodestat)`, `linux/fs/ceph/mds_client.h:48 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs8-kernel/fs/ceph/mds_client.h:41 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs9-kernel/fs/ceph/mds_client.h:46 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:64 (CEPHFS_FEATURES_ALL)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux-v5.0/fs/ceph/mds_client.h:25-29 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux-v5.1/fs/ceph/mds_client.h:25-31 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `v14.2.0:src/mds/cephfs_features.h:26 (CEPHFS_FEATURE_REPLY_ENCODING)`.
- **Map:** [rstats-quotas-and-subvolumes.md › 3.10 How clients learn quotas and usage](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#310-how-clients-learn-quotas-and-usage) · [rstats-quotas-and-subvolumes.md › 5.1 Messages](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#51-messages)
- **Code:** `src/mds/CInode.cc:4315-4369 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4092-4096 (CInode::encode_inodestat)`, `linux/fs/ceph/mds_client.c:94 (parse_reply_info_quota)`, `src/client/Client.cc:1203-1211 (Client::add_update_inode)`
- **Notes:** The kclient applies the quota from every reply but directory rstat only from auth (or newer-version) replies; libcephfs applies both only from auth or newer replies. `MClientCaps` v11 also carries directory `nfiles/nsubdirs`.

<a id="f-snap-045"></a>

#### F-SNAP-045 · MClientQuota usage pushes with thresholds

*Pushes quota and usage updates to clients holding caps on a quota directory when usage approaches the limit or the quota changes.*

`broadcast_quota_to_client(in)` runs on the inode's auth (active or stopping, not frozen) for every ancestor projected by `predirty_journal_parents`, at the end of an INEST gather, and on a quota change. For each client cap on the inode, skipping caps with `STATE_NOQUOTA` (no `CEPH_FEATURE_MDS_QUOTA`), it sends `MClientQuota` (`CEPH_MSG_CLIENT_QUOTA` 0x314: `u64 ino, utime rctime, i64 rbytes, rfiles, rsubdirs, quota_info_t`; no `rsnaps`, no version) when: the quota changed and the cap belongs to another client; or the last-sent values differ and `rsize >= max_files`, `rbytes > 7/8 max_bytes`, or usage moved by more than 1/16 of the headroom at the last update. It updates `last_rbytes`/`last_rsize` on send. Messages are sent counted (they increment the session push sequence). There is no timer.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Skipping caps marked STATE_NOQUOTA, i.e. caps of sessions whose connection lacks CEPH_FEATURE_MDS_QUOTA. The 7.3 kernel, the CentOS Stream 8 and 9 samples and libcephfs advertise CEPH_FEATURE_MDS_QUOTA (bit 47), so no baseline cap is NOQUOTA and every cap holder gets the threshold-driven MClientQuota pushes. Upstream kernels advertise bit 47 from v4.17. Evidence: `src/mds/MDCache.cc:2145-2146 (MDCache::broadcast_quota_to_client)`, `src/mds/Capability.cc:179-180 (Capability::Capability)`, `linux/include/linux/ceph/ceph_features.h:212 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `cs8-kernel/include/linux/ceph/ceph_features.h:212 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `cs9-kernel/include/linux/ceph/ceph_features.h:212 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:231 (CEPH_FEATURES_ALL)`, `linux-v4.13/include/linux/ceph/ceph_features.h:177-211 (CEPH_FEATURES_SUPPORTED_DEFAULT, no MDS_QUOTA)`, `linux-v4.17/include/linux/ceph/ceph_features.h:207 (CEPH_FEATURES_SUPPORTED_DEFAULT)`.
- **Map:** [rstats-quotas-and-subvolumes.md › 3.10 How clients learn quotas and usage](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#310-how-clients-learn-quotas-and-usage) · [rstats-quotas-and-subvolumes.md › 5.1 Messages](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#51-messages) · [rstats-quotas-and-subvolumes.md › 7. Externally visible contract](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#7-externally-visible-contract)
- **Code:** `src/mds/MDCache.cc:2103-2190 (MDCache::broadcast_quota_to_client)`, `src/messages/MClientQuota.h:30-49`, `linux/fs/ceph/quota.c:40 (ceph_handle_quota)`, `src/client/Client.cc:5693-5716 (Client::handle_quota)`
- **Notes:** The kclient parses the packed struct at fixed offsets (extra bytes ignored, embedded v/compat not checked, `rctime` ignored) and applies it only to cached inodes; libcephfs asserts no trailing bytes and replaces `in->quota` and the whole `in->rstat`, so `rsnaps` and `version` become 0. Usage pushes happen only where the quota inode's rstat is updated on its auth, so cross-rank propagation lag (F-SNAP-036) delays them. Upstream docs: writers are stopped "within tens of seconds". See also F-PROTO-095 (encoding).

<a id="f-snap-047"></a>

#### F-SNAP-047 · Client-side quota enforcement (EDQUOT, cross-quota EXDEV)

*Describes where quotas are actually enforced today: in the clients, from MDS-supplied quota and usage.*

The C++ MDS never returns `EDQUOT` and has no cross-quota-realm rename check. Clients find the quota root by walking snap realms and enforce: `max_files` on mknod/create/mkdir/symlink (kclient: `rfiles + rsubdirs + 1 > max_files`; libcephfs also on link: `rsize >= max_files`) with `-EDQUOT`; `max_bytes` on write, `setattr(size)`, copy_file_range (kclient) and `_write`, `_do_setattr`, `_fallocate` (libcephfs) when `rbytes + growth > max_bytes`; `-EXDEV` on rename between different quota realms/roots. Both flush caps early when growth since the last reported size exceeds 1/16 of the remaining headroom. The kclient fetches uncached realm root inodes with `LOOKUPINO` (failures negatively cached 60 s); libcephfs fetches every ancestor of the mount root with `GETATTR` at mount ("quotas may not work" on `EACCES`).

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Gates:** libcephfs `client_quota` (default true)
- **Map:** [rstats-quotas-and-subvolumes.md › 3.11 Enforcement (clients only)](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#311-enforcement-clients-only) · [rstats-quotas-and-subvolumes.md › 7. Externally visible contract](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#7-externally-visible-contract)
- **Code:** `linux/fs/ceph/quota.c:333 (check_quota_exceeded)`, `linux/fs/ceph/dir.c:1503-1505 (ceph_rename)`, `src/client/Client.cc:18350 (Client::check_quota_condition)`, `src/client/Client.cc:16749-16758 (Client::_rename)`, `src/client/Client.cc:7097-7117 (Client::mount)`
- **Notes:** Quota counts `rsize()`, so the quota directory itself counts against its own `max_files`. The kclient checks no quota on link or fallocate; libcephfs `check_quota_condition` tests the starting directory itself before walking realms. A libcephfs client with `client_quota=false` or a hand-crafted request can move a subtree between quota realms. Multiple writers can overshoot; mgr/volumes sets clone quotas after copying for that reason.

<a id="f-snap-048"></a>

#### F-SNAP-048 · Quota-based statfs

*Makes `df` on a mount inside a quota directory report the quota as the filesystem size.*

Kernel: if the mount root's `max_bytes` realm exists, `statfs` reports `f_blocks`/`f_bfree` from `max_bytes` and `rbytes` (4 MiB blocks, 4 KiB below 4 MiB) unless mounted with `noquotadf`. libcephfs (`client_quota_df`, default true): total = `max_bytes`, used = `rbytes`, `f_files = rfiles + rsubdirs` of the files quota root; before reporting it refreshes the byte quota root with a forced `GETATTR` (mask 0) unless a session is stale, so every `df` sends one request to the MDS.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Gates:** kclient mount option `noquotadf`; libcephfs `client_quota_df` (default true)
- **Map:** [rstats-quotas-and-subvolumes.md › 3.11 Enforcement (clients only)](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#311-enforcement-clients-only) · [rstats-quotas-and-subvolumes.md › 7. Externally visible contract](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#7-externally-visible-contract)
- **Code:** `linux/fs/ceph/quota.c:504 (ceph_quota_update_statfs)`, `src/client/Client.cc:13664-13730 (Client::_statfs)`
- **Notes:** The MDS side is only the quota and rstat values in `InodeStat` (F-SNAP-044) and the GETATTR. See also F-NS-091 (statfs).

### Subvolumes

The `SUBVOLUME` realm flag and the rules it switches on: no nesting, EXDEV at boundaries, snapshots only at the root, a per-inode subvolume id, and the advisory snapshot visibility flag.

<a id="f-snap-050"></a>

#### F-SNAP-050 · Subvolume flag (ceph.dir.subvolume)

*Marks a directory as a subvolume root, switching on the subvolume rules; mgr/volumes sets it on every subvolume it creates.*

`SETXATTR ceph.dir.subvolume` on a directory (`EINVAL` otherwise; value parsed with `strict_strtob`, `EINVAL` on error; RMXATTR of an unset flag returns 0, otherwise means "0") sets or clears `sr_t::SUBVOLUME` (2) in the directory's srnode, creating the realm if needed (`created = seq = realm newest seq`). A cheap pre-check under a `snaplock` rdlock returns 0 when the state already matches; otherwise `policylock` and `snaplock` are xlocked, ancestor snaplocks rdlocked, and the checks repeated. Journaled as `EUpdate("set vxattr layout")`; on commit `MMDSSnapUpdate` to replicas and `MClientSnap` SPLIT (lists filled only for a new realm), with realm caches invalidated so subvolume ids refresh. `GETVXATTR ceph.dir.subvolume` returns `"1"`/`"0"` from the projected srnode. The flag is persisted in the inode snap blob.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by mgr, kclient, libcephfs, tools
- **Gates:** MDS auth cap flag `p` (vxattr set)
- **Map:** [rstats-quotas-and-subvolumes.md › 3.12 Subvolumes: the SUBVOLUME realm flag](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#312-subvolumes-the-subvolume-realm-flag) · [snapshots.md › 3.11 Subvolumes and snapshot visibility](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#311-subvolumes-and-snapshot-visibility) · [rstats-quotas-and-subvolumes.md › 3.15 mgr/volumes as consumer](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#315-mgrvolumes-as-consumer)
- **Code:** `src/mds/Server.cc:6546-6632 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:7492-7494 (Server::handle_client_getvxattr)`, `src/mds/snap.h:88-128 (sr_t)`, `src/pybind/mgr/volumes/fs/operations/versions/subvolume_v2.py:95-104`
- **Notes:** Clearing keeps the realm and is not blocked by existing snapshots; marking `/` is not refused. Unlike the visibility flag it does not bump `last_modified`/`change_attr`. The kclient reads it through GETVXATTR, which needs `CEPHFS_FEATURE_OP_GETVXATTR`. mgr/volumes marks `/volumes/<group>/<subvol>` (v2) or the `<uuid>` directory (v1); a repeated set must be a no-op.

<a id="f-snap-051"></a>

#### F-SNAP-051 · No nested subvolumes; outermost subvolume wins

*Refuses to create a subvolume inside another subvolume and defines which subvolume an inode belongs to.*

Setting the flag when `cur->find_snaprealm()->get_subvolume_ino()` is non-zero and differs from the directory returns `EINVAL` ("can't create subvolume inside another subvolume"). `get_subvolume_ino()` of a realm is the parent's subvolume ino if non-zero, else this realm's inode if it has `SUBVOLUME`, else 0, i.e. the outermost subvolume ancestor. Marking an ancestor of an existing subvolume is not checked; it then changes the subvolume id of every inode below and moves the EXDEV boundaries.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by mgr
- **Map:** [rstats-quotas-and-subvolumes.md › 3.12 Subvolumes: the SUBVOLUME realm flag](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#312-subvolumes-the-subvolume-realm-flag) · [rstats-quotas-and-subvolumes.md › 4. Data structures and invariants](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#4-data-structures-and-invariants) · [rstats-quotas-and-subvolumes.md › 8. Edge cases, failure handling and gotchas](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Server.cc:6599-6611 (Server::handle_client_setvxattr)`, `src/mds/SnapRealm.cc:150-153 (SnapRealm::check_cache)`, `src/mds/CInode.cc:5597-5600 (CInode::get_subvolume_id)`

<a id="f-snap-052"></a>

#### F-SNAP-052 · Cross-subvolume link and rename rejected with EXDEV

*Keeps every inode inside one subvolume, so hard links and renames never cross a subvolume boundary.*

link: `EXDEV` when the target's primary parent directory (if it is not the destination directory and not a stray directory) has a realm whose `get_subvolume_ino()` differs from the destination directory's. rename: `EXDEV` when the realms of the source and destination *directories* (not of the moved inode) differ and their subvolume inos differ (skipped for link-merge), so a subvolume root can still be renamed between two non-subvolume directories (the mgr/volumes `_deleting` trash move) but not into another subvolume. Both checks run after the `ENOSPC` entry limits.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Map:** [rstats-quotas-and-subvolumes.md › 3.13 What the subvolume flag changes inside the MDS](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#313-what-the-subvolume-flag-changes-inside-the-mds) · [snapshots.md › 3.11 Subvolumes and snapshot visibility](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#311-subvolumes-and-snapshot-visibility)
- **Code:** `src/mds/Server.cc:7971-7983 (Server::handle_client_link)`, `src/mds/Server.cc:9548-9565 (Server::handle_client_rename)`
- **Notes:** POSIX allows `EXDEV` for link/rename across filesystems. The kclient's sticky `subvolume_id` (F-SNAP-054) and the absence of global realms for links inside subvolumes (F-SNAP-011) both rely on these rules. EXDEV between quota realms comes only from clients (F-SNAP-047).

<a id="f-snap-053"></a>

#### F-SNAP-053 · Snapshots only at the subvolume root

*Refuses snapshot creation on directories inside a subvolume, so a subvolume's snapshots are all taken at its root.*

MKSNAP returns `EPERM` when `find_snaprealm()->get_subvolume_ino()` is non-zero and is not the directory itself. RMSNAP and RENAMESNAP have no subvolume check. mgr/volumes v2 takes subvolume snapshots at `/volumes/<group>/<subvol>/.snap/<name>` with libcephfs `mkdir`.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Map:** [rstats-quotas-and-subvolumes.md › 3.13 What the subvolume flag changes inside the MDS](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#313-what-the-subvolume-flag-changes-inside-the-mds) · [snapshots.md › 3.6 mksnap](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#36-mksnap) · [snapshots.md › 3.11 Subvolumes and snapshot visibility](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#311-subvolumes-and-snapshot-visibility)
- **Code:** `src/mds/Server.cc:11650-11655 (Server::handle_client_mksnap)`

<a id="f-snap-054"></a>

#### F-SNAP-054 · Subvolume id on the wire (InodeStat, MClientCaps)

*Subvolume-side facts about the subvolume_id that InodeStat and cap flush acks carry.*

See F-QOS-015. Subvolume-side facts: the id is the outermost enclosing subvolume root (ceph.dir.subvolume, F-SNAP-050), cached per realm and refreshed when realm caches are invalidated. It stays stable for an inode only through the EXDEV rules (F-SNAP-052) and the refusal of nested subvolumes (F-SNAP-051); setting or clearing the flag on an ancestor changes it.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Map:** [rstats-quotas-and-subvolumes.md › 3.13 What the subvolume flag changes inside the MDS](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#313-what-the-subvolume-flag-changes-inside-the-mds) · [rstats-quotas-and-subvolumes.md › 7. Externally visible contract](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#7-externally-visible-contract)
- **Code:** `src/mds/CInode.cc:4369 (CInode::encode_inodestat)`, `src/mds/Locker.cc:2501-2506 (Locker::maybe_set_subvolume_id)`, `src/mds/CInode.cc:5597-5600 (CInode::get_subvolume_id)`, `linux/fs/ceph/inode.c:891-912 (ceph_inode_set_subvolume)`
- **Notes:** The kclient ignores 0 and treats a non-zero id as immutable (a different non-zero value only triggers WARN_ON_ONCE); libcephfs uses both InodeStat and flush acks.

<a id="f-snap-055"></a>

#### F-SNAP-055 · Snapshot visibility flag (ceph.dir.subvolume.snaps.visible)

*Stores a per-realm flag that libcephfs can use to hide `.snap` of a subvolume; the MDS itself does not enforce it.*

`SETXATTR ceph.dir.subvolume.snaps.visible <bool>` sets or clears `sr_t::SNAPDIR_VISIBILITY` (4, default set) on a directory that already has an srnode (returns 0 as a no-op otherwise), with the same rdlock-precheck-then-xlock pattern, and bumps `last_modified` and `change_attr` but not `seq`. `GETVXATTR` returns `1`/`0`, or an empty value without an srnode; non-directories get `ENODATA`. The flag reaches clients only in `SnapRealmInfoNew` traces. LSSNAP and LOOKUPSNAP ignore it. libcephfs returns `EPERM` on `.snap` lookups and its `mksnap`/`rmsnap` calls when `client_respect_subvolume_snapshot_visibility = true` (default false) and the flag is clear in the realm containing the directory.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by libcephfs, ceph-fuse, mgr
- **Gates:** MDS auth cap flag `p` (vxattr set); CEPHFS_FEATURE_NEW_SNAPREALM_INFO (19) needed to receive it; libcephfs `client_respect_subvolume_snapshot_visibility` (default false)
- **Map:** [rstats-quotas-and-subvolumes.md › 3.12 Subvolumes: the SUBVOLUME realm flag](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#312-subvolumes-the-subvolume-realm-flag) · [snapshots.md › 3.11 Subvolumes and snapshot visibility](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#311-subvolumes-and-snapshot-visibility)
- **Code:** `src/mds/Server.cc:6633-6722 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:7495-7507 (Server::handle_client_getvxattr)`, `src/client/Client.cc:7884-7891 (Client::_lookup)`, `src/client/Client.cc:14340-14366`
- **Notes:** The kclient does not advertise `NEW_SNAPREALM_INFO`, never receives the flag and ignores it. Child realms do not inherit it (each keeps its own default true), so mgr/volumes sets it on the subvolume root and, for sized subvolumes, also on the `<uuid>` quota directory (`subvolume_base.py:738-787`). See also F-ADM-098 (mgr/volumes snapshot visibility).

### Legacy entries

These entries serve only clients older than the [baseline](../README.md#baseline-current-peers), msgr1 mounts or older C++ releases, or describe code that cannot run. Each keeps its ID and group.

<a id="f-snap-005"></a>

#### F-SNAP-005 · allow_multimds_snaps flag and MULTIMDS_WITH_OLDSNAPS health check

*Blocks multiple active ranks on filesystems that still hold pre-Mimic ("v1") snapshots, and lifts the block automatically once they are gone.*

The table records `snaprealm_v2_since`, the first snapid created with the v2 realm format (`upgrade_format()` sets it on rank 0 at `replay_done` for old tables). `can_allow_multimds_snaps()` is true when no snapshot older than it remains; rank 0's `tick` then sends the mon command `fs set <fs> allow_multimds_snaps true --yes-i-am-really-a-mds` once per process. The monitor refuses raising `max_mds` above 1 while `was_snaps_ever_allowed() && !allows_multimds_snaps()` and raises the HEALTH_ERR check `MULTIMDS_WITH_OLDSNAPS`. New filesystems get `CEPH_MDSMAP_DEFAULTS = ALLOW_SNAPS + ALLOW_MULTIMDS_SNAPS`, so the flag is set from creation.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin`, `metrics` · relied on by mon, admin
- **Group:** Snapshot ids and the snap table
- **Legacy** (old-data): SnapServer::reset_state sets snaprealm_v2_since for every new snap table, and only a table decoded with struct_v below 5 has it unset until upgrade_format sets it at replay_done. New file systems start with CEPH_MDSMAP_ALLOW_MULTIMDS_SNAPS set (CEPH_MDSMAP_DEFAULTS), so the tick branch that sends fs set allow_multimds_snaps never runs and the monitor's max_mds refusal and MULTIMDS_WITH_OLDSNAPS check never trigger. The snapshots this mechanism guards were created by MDS releases before Mimic (Luminous 12.2.x and older). Evidence: `src/mds/SnapServer.cc:75 (SnapServer::reset_state)`, `src/mds/SnapServer.cc:117-120 (SnapServer::decode_server_state)`, `src/mds/SnapServer.h:42-57 (SnapServer::upgrade_format)`, `src/mds/MDSRank.cc:1935-1939 (MDSRank::replay_done)`, `src/mds/MDSRank.cc:754-760 (MDSRankDispatcher::tick)`, `src/mds/MDSMap.h:593 (MDSMap::flags)`, `src/include/ceph_fs.h:298-299 (CEPH_MDSMAP_DEFAULTS)`, `src/mon/FSCommands.cc:446-451 (max_mds)`, `src/mon/FSCommands.cc:585-590 (allow_multimds_snaps)`, `src/mds/MDSMap.cc:606-612 (MDSMap::get_health_checks)`.
- **Gates:** MDSMap flag `CEPH_MDSMAP_ALLOW_MULTIMDS_SNAPS`; mon command `fs set <fs> allow_multimds_snaps <bool> --yes-i-am-really-a-mds`
- **Map:** [snapshots.md › 3.1 Snapshot ids and the global snap table](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#31-snapshot-ids-and-the-global-snap-table) · [snapshots.md › 3.12 Multi-MDS handling](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#312-multi-mds-handling) · [snapshots.md › 5.7 Admin and monitor interfaces](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#57-admin-and-monitor-interfaces)
- **Code:** `src/mds/MDSRank.cc:690-708 (MDSRank::set_mdsmap_multimds_snaps_allowed)`, `src/mds/SnapServer.cc:419-421 (SnapServer::can_allow_multimds_snaps)`, `src/mon/FSCommands.cc:445-450`, `src/mds/MDSMap.cc:606-611 (MDSMap::get_health_checks)`, `src/include/ceph_fs.h:298-299`
- **Notes:** Only relevant to filesystems that had snapshots under a pre-Mimic MDS. The FSMap flag and health check are owned by 01-cluster-membership-and-ha; this entry covers the snapshot condition behind them.

<a id="f-snap-049"></a>

#### F-SNAP-049 · Legacy pre-Mimic quota realm creation (create_quota_realm)

*Was meant to give quota directories created before Mimic a snap realm on the fly.*

When `broadcast_quota_to_client` finds an enabled (or just changed) quota on an inode without a projected srnode, it calls `create_quota_realm(in)`, which sends an internal `MClientRequest SETXATTR "ceph.quota"` with an empty value to the inode's auth (itself). The request has no client session, so `check_access` passes. The empty value parses as "unchanged" and the handler returns 0 before projecting the srnode, so the realm is never created and the request is re-sent on every later broadcast through that inode.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Group:** Quotas
- **Legacy** (old-data): broadcast_quota_to_client calls create_quota_realm only for an inode that has a quota but no projected srnode. Since Mimic 13.2.0 a quota set on a directory without an srnode projects one in the same update, so only quota directories written by Luminous 12.2.x or older MDS releases lack a realm. The internal SETXATTR it sends has been a no-op since commit 8d531680cdf (first tag v16.1.0), which returns 0 for an unchanged quota before any projection. Evidence: `src/mds/MDCache.cc:2139-2141 (MDCache::broadcast_quota_to_client)`, `src/mds/Server.cc:6478-6479 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:6484-6487 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:6489-6493 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:6262-6273 (Server::create_quota_realm)`, `v13.2.0:src/mds/MDCache.cc:2016 (MDCache::broadcast_quota_to_client)`. Also: search: git show v12.2.13:src/mds/Server.cc contains neither adjust_realm nor create_quota_realm.
- **Map:** [rstats-quotas-and-subvolumes.md › 3.9 Quotas: data model and how a quota is set](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#39-quotas-data-model-and-how-a-quota-is-set) · [rstats-quotas-and-subvolumes.md › 8. Edge cases, failure handling and gotchas](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Server.cc:6262-6273 (Server::create_quota_realm)`, `src/mds/MDCache.cc:2139-2141 (MDCache::broadcast_quota_to_client)`, `src/mds/Server.cc:6484-6487 (Server::handle_client_setvxattr)`
- **Notes:** Ineffective since commit 8d531680cdf ("mds: add vxattr that marks/clears subvolume flag") added the unchanged-quota early return (derived from code reading; reachable only for pre-Mimic quota inodes).

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-snap-046"></a>F-SNAP-046 | [F-MULTI-020](07-multi-mds.md#f-multi-020) · Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps) |
| <a id="f-snap-056"></a>F-SNAP-056 | [F-CACHE-081](05-metadata-cache-and-dirfrags.md#f-cache-081) · Per-directory entry limit |
| <a id="f-snap-057"></a>F-SNAP-057 | [F-CACHE-080](05-metadata-cache-and-dirfrags.md#f-cache-080) · Per-dirfrag size limit (ENOSPC) |

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
