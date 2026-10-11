# Cluster membership and high availability: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [fsmap-and-mdsmap.md](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md) · [beacon-and-monitor.md](../cpp-map/01-cluster-integration/beacon-and-monitor.md) · [rank-lifecycle-and-recovery.md](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md) · [resolve-and-rejoin.md](../cpp-map/09-recovery/resolve-and-rejoin.md) · [process-and-threading.md](../cpp-map/00-architecture/process-and-threading.md)

## 1. Scope

This category covers how an MDS daemon lives inside an unmodified Ceph cluster and how a rank survives failures: the monitor-owned FSMap and per-file-system MDSMap (fields, epochs, encodings, subscriptions, client decoding), compat sets and upgrade gating, the MMDSBeacon protocol and liveness, standby selection, standby-replay, failover and OSD blocklist fencing, the monitor `fs`/`mds` commands that change or show membership, the rank state machine, and the recovery pipeline replay, resolve, reconnect, rejoin, clientreplay, active and stopping, plus the daemon process anatomy (messenger, MonClient subscriptions, threads, timers, `mds_lock`, dispatch). Sources are the fact-checked map pages fsmap-and-mdsmap, beacon-and-monitor, rank-lifecycle-and-recovery, resolve-and-rejoin and process-and-threading. Much of the behaviour in the first five groups is implemented by the unmodified MDSMonitor; those entries describe the contract an MDS daemon must live inside, not code the MDS runs.

Excluded and owned elsewhere: reconnect message encoding, session states, eviction and blocklisting of clients, `required_client_features`, `refuse_client_session` and session timeouts (02-client-sessions-and-protocol); completed-request bookkeeping and request forwarding semantics (04-namespace-and-posix-operations); cap and lock semantics referenced by rejoin (03-caps-leases-and-locking); journal event formats and OSD epoch barriers and blocklist checks (06-journal-and-storage); subtree migration and the balancer, including balancer settings and export targets in the MDSMap (07-multi-mds); snapshot flags, `MRemoveSnaps` and `allow_multimds_snaps` (08-snapshots-quotas-subvolumes); quiesce membership and metrics pings (10-qos-metrics-quiesce-quarantine); admin socket commands, the `status` output, perf counters, the meaning of individual health metrics, the health checks the monitor derives from the map, and configuration plumbing (11-security-admin-and-observability); file-lock recovery from reconnect messages (04-namespace-and-posix-operations). Background on what `mds_lock` protects is in [mds_lock-analysis](../cpp-map/00-architecture/mds_lock-analysis.md).

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

114 features: 114 used by current peers (18 of them with legacy parts) and 0 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**Cluster maps and subscriptions**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-001](#f-clu-001) | FSMap: monitor-owned map of all file systems | operational | cluster-wire | mon, mgr, tools, libcephfs, cephfs-mirror |
| [F-CLU-002](#f-clu-002) | MDSMap: per-file-system map and its wire encoding *(legacy parts)* | supporting | cluster-wire, client-wire | kclient, libcephfs, ceph-fuse, ganesha, mon, mgr, mds-ranks |
| [F-CLU-003](#f-clu-003) | Epoch model: FSMap, MDSMap and standby epochs, null map | operational | cluster-wire, client-wire | mon, kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-CLU-004](#f-clu-004) | Map subscriptions and the MDS view of mdsmap (null maps) | operational | cluster-wire, client-wire | mon, kclient, libcephfs, ceph-fuse, ganesha, mgr, tools, cephfs-mirror |
| [F-CLU-005](#f-clu-005) | FSMapUser: file system name directory for clients | operational | cluster-wire, client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CLU-006](#f-clu-006) | Kernel client partial MDSMap decode *(legacy parts)* | supporting | client-wire | kclient |
| [F-CLU-007](#f-clu-007) | libcephfs map use and mount availability check | supporting | client-wire | libcephfs, ceph-fuse, ganesha |
| [F-CLU-008](#f-clu-008) | FSMap filtering for file-system-restricted sessions | operational | cluster-wire, admin | kclient, libcephfs, mgr, admin |
| [F-CLU-009](#f-clu-009) | FSMap sanity invariants enforced by the monitor | operational | cluster-wire | mon |
| [F-CLU-010](#f-clu-010) | Multiple file systems: enable_multiple and the default file system *(legacy parts)* | operational | cluster-wire, admin | kclient, libcephfs, admin |
| [F-CLU-011](#f-clu-011) | Data pool list of a file system | extension | cluster-wire, client-wire, admin | kclient, libcephfs, admin, osd, ceph-fuse |
| [F-CLU-012](#f-clu-012) | max_file_size published in the MDSMap | supporting | client-wire, admin | kclient, libcephfs, ceph-fuse, admin |
| [F-CLU-013](#f-clu-013) | Mirroring metadata (MirrorInfo) in the FSMap | operational | cluster-wire, admin | cephfs-mirror, mgr, admin |

**Compat sets and upgrade gating**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-014](#f-clu-014) | CompatSet encoding and the MDS incompat feature ids | operational | cluster-wire | mon, cephadm, admin |
| [F-CLU-015](#f-clu-015) | Standby eligibility by compat set | operational | cluster-wire | mon, cephadm, admin |
| [F-CLU-016](#f-clu-016) | Forced INLINE incompat bit and compat bootstrap *(legacy parts)* | operational | cluster-wire | mon |
| [F-CLU-017](#f-clu-017) | Compat set must not change after boot | operational | cluster-wire | mon |
| [F-CLU-018](#f-clu-018) | Upgradeable file systems and compat merge on promotion *(legacy parts)* | operational | cluster-wire | mon, cephadm, admin |
| [F-CLU-019](#f-clu-019) | fs compat and mds compat commands *(legacy parts)* | operational | admin | admin |
| [F-CLU-020](#f-clu-020) | mds_features in the boot beacon and get_up_features *(legacy parts)* | internal | cluster-wire, on-disk | mds-ranks, internal |

**Beacons and liveness**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-021](#f-clu-021) | MMDSBeacon message and the beacon sender thread *(legacy parts)* | operational | cluster-wire, config | mon |
| [F-CLU-022](#f-clu-022) | Monitor beacon admission rules | operational | cluster-wire | mon |
| [F-CLU-023](#f-clu-023) | Beacon acks and MDS-side lagginess (self-fencing deferral) | operational | cluster-wire, config | mon, kclient, libcephfs, internal |
| [F-CLU-024](#f-clu-024) | Heartbeat-gated beacons (internal watchdog) | operational | cluster-wire, config, metrics | mon |
| [F-CLU-025](#f-clu-025) | Boot registration as a standby with daemon metadata | operational | cluster-wire, admin | mon, mgr, tools, admin |
| [F-CLU-026](#f-clu-026) | Unique daemon names | operational | cluster-wire, config | mon, admin, cephadm, rook |
| [F-CLU-027](#f-clu-027) | DNE beacon on clean shutdown | operational | cluster-wire | mon, cephadm, rook, admin |
| [F-CLU-028](#f-clu-028) | Health metrics carried in beacons | operational | cluster-wire, metrics | mon, mgr, admin |
| [F-CLU-029](#f-clu-029) | Monitor-side laggy detection and its protections | operational | cluster-wire, config | mon, kclient, libcephfs, ceph-fuse, ganesha |

**Standby selection, failover and fencing**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-030](#f-clu-030) | Standby selection scoring | operational | cluster-wire | mon, cephadm, rook, admin |
| [F-CLU-031](#f-clu-031) | mds_join_fs file system affinity | operational | cluster-wire, config | mon, cephadm, rook, admin |
| [F-CLU-032](#f-clu-032) | Per-file-system selection flags: refuse_standby_for_another_fs and host anti-affinity | operational | cluster-wire, admin | mon, admin |
| [F-CLU-033](#f-clu-033) | Affinity-driven swaps of healthy daemons | operational | cluster-wire | mon, cephadm, admin, kclient, libcephfs |
| [F-CLU-034](#f-clu-034) | Failed-rank and laggy-holder replacement | operational | cluster-wire | mon, kclient, libcephfs, ceph-fuse, ganesha, admin |
| [F-CLU-035](#f-clu-035) | Promotion into a rank and the initial rank state | operational | cluster-wire | mon, kclient, libcephfs, cephadm, admin |
| [F-CLU-036](#f-clu-036) | Fencing failed rank holders with OSD blocklists | supporting | cluster-wire | mon, osd |
| [F-CLU-037](#f-clu-037) | Wait for last_failure_osd_epoch before touching rank objects | supporting | cluster-wire | mon, osd |
| [F-CLU-038](#f-clu-038) | Rank incarnation | supporting | cluster-wire, client-wire | mon, osd, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-CLU-039](#f-clu-039) | Standby-replay assignment | operational | cluster-wire, admin | mon, admin, cephadm |
| [F-CLU-040](#f-clu-040) | Growing the number of active ranks (max_mds) | operational | cluster-wire, admin | mon, admin, cephadm, mgr, kclient, libcephfs |
| [F-CLU-041](#f-clu-041) | Shrinking: monitor-driven up:stopping and FSMap::stop | operational | cluster-wire, admin | mon, admin, cephadm, mgr, kclient, libcephfs |

**Monitor fs and mds commands**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-042](#f-clu-042) | fs new | operational | admin, cluster-wire | admin, mgr, cephadm, rook, mon, osd |
| [F-CLU-043](#f-clu-043) | fs rm | operational | admin, cluster-wire | admin, mgr, cephadm, rook |
| [F-CLU-044](#f-clu-044) | fs fail and the joinable flag *(legacy parts)* | operational | admin, cluster-wire | admin, mgr, cephadm, rook, osd |
| [F-CLU-045](#f-clu-045) | fs reset | operational | admin, cluster-wire | admin, tools |
| [F-CLU-046](#f-clu-046) | fs set down | operational | admin, cluster-wire | admin, cephadm, rook |
| [F-CLU-047](#f-clu-047) | fs rename and fs swap | operational | admin, cluster-wire | admin, kclient, mgr |
| [F-CLU-048](#f-clu-048) | mds fail | operational | admin, cluster-wire | admin, cephadm, rook, mgr, osd |
| [F-CLU-049](#f-clu-049) | Damaged ranks and mds repaired | operational | admin, cluster-wire, metrics | admin, mon, tools, kclient, libcephfs |
| [F-CLU-050](#f-clu-050) | Hidden and debug rank commands: mds rm, rmfailed, freeze, set_state | operational | admin, cluster-wire | admin, tools |
| [F-CLU-051](#f-clu-051) | Read-only fs and mds commands and the fs dump layout *(legacy parts)* | operational | admin | admin, mgr, cephadm, rook, tools |
| [F-CLU-052](#f-clu-052) | Orchestration helpers: mds ok-to-stop, last-seen, metadata, versions | operational | admin | cephadm, rook, mgr, admin, tools |

**Rank state machine and map handling**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-053](#f-clu-053) | Daemon and rank states: numeric values and names *(legacy parts)* | operational | cluster-wire, client-wire, admin | mon, kclient, libcephfs, ceph-fuse, ganesha, mgr, cephadm, rook, tools, admin |
| [F-CLU-054](#f-clu-054) | State transition validation on both sides | operational | cluster-wire | mon |
| [F-CLU-055](#f-clu-055) | Requesting the next state through beacons | operational | cluster-wire | mon |
| [F-CLU-056](#f-clu-056) | Rank sets and derived predicates | operational | cluster-wire | mon, mds-ranks, libcephfs, kclient |
| [F-CLU-057](#f-clu-057) | Daemon-level map handling | operational | cluster-wire | mon, internal |
| [F-CLU-058](#f-clu-058) | Rank-level map handling and peer change detection | operational | cluster-wire | mon, osd, mds-ranks, internal |
| [F-CLU-060](#f-clu-060) | MDSMap epoch waits for client requests | supporting | client-wire | kclient, libcephfs, ceph-fuse |
| [F-CLU-062](#f-clu-062) | File-system health checks derived from the map | operational | metrics, admin | mon, mgr, admin, cephadm, rook |

**Boot, creation, replay and standby-replay**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-063](#f-clu-063) | Boot stages for up:starting, up:replay and up:standby-replay | supporting | on-disk | osd, internal |
| [F-CLU-064](#f-clu-064) | Creating a new rank (up:creating) | supporting | on-disk, cluster-wire | mon, osd, admin, tools |
| [F-CLU-065](#f-clu-065) | Restarting a stopped rank (up:starting) | supporting | on-disk, cluster-wire | mon, admin |
| [F-CLU-066](#f-clu-066) | up:replay: journal replay of a failed rank | supporting | on-disk, metrics | osd, admin, internal |
| [F-CLU-067](#f-clu-067) | End of replay: journal reformat, writeable journal and the single-rank shortcut *(legacy parts)* | supporting | on-disk, cluster-wire | mon, internal |
| [F-CLU-068](#f-clu-068) | Standby-replay follower loop | operational | cluster-wire, config | mon, admin, osd |
| [F-CLU-069](#f-clu-069) | Standby-replay takeover | operational | cluster-wire | mon, admin, kclient, libcephfs |

**Multi-rank recovery and resolve**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-070](#f-clu-070) | Recovery set and cluster-wide resolve/rejoin barriers | supporting | mds-wire, cluster-wire | mds-ranks, mon |
| [F-CLU-071](#f-clu-071) | Survivor reactions to a peer failing and recovering | supporting | mds-wire | mds-ranks, kclient, libcephfs |
| [F-CLU-072](#f-clu-072) | Deferring messages to bootstrapping peers and dropping stale peer messages | internal | mds-wire | mds-ranks |
| [F-CLU-073](#f-clu-073) | up:resolve entry and exit | supporting | mds-wire, cluster-wire, on-disk | mds-ranks, mon |
| [F-CLU-074](#f-clu-074) | Subtree authority resolution (subtree resolves) | supporting | mds-wire | mds-ranks |
| [F-CLU-075](#f-clu-075) | Ambiguous subtree import disambiguation | supporting | mds-wire, on-disk | mds-ranks, kclient |
| [F-CLU-076](#f-clu-076) | Cross-rank two-phase update outcomes (peer resolves and ECommitted) | supporting | mds-wire, on-disk | mds-ranks, internal |
| [F-CLU-077](#f-clu-077) | Rename-specific resolve handling | supporting | mds-wire, client-wire | mds-ranks, kclient, libcephfs, ceph-fuse, ganesha |
| [F-CLU-078](#f-clu-078) | Snap table recovery and the SERVER_READY gate | supporting | mds-wire | mds-ranks, internal |
| [F-CLU-079](#f-clu-079) | Post-resolve cache cleanup and authority bits | internal | internal | internal |

**Client reconnect, rejoin and clientreplay**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-080](#f-clu-080) | up:reconnect: the client reconnect window and its timeout | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha, admin, osd |
| [F-CLU-082](#f-clu-082) | Recording caps and snaprealms from reconnect messages | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CLU-084](#f-clu-084) | Admission of replayed and retried requests during reconnect | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CLU-085](#f-clu-085) | Rejoin message exchange: weak rejoin, strong rejoin and acks | supporting | mds-wire | mds-ranks |
| [F-CLU-086](#f-clu-086) | Recreating survivors' in-flight lock state on the recovering rank | supporting | mds-wire | mds-ranks |
| [F-CLU-087](#f-clu-087) | Matching reconnected caps to inodes: OpenFileTable prefetch and open by number | supporting | on-disk, client-wire | kclient, libcephfs, osd, internal, ceph-fuse, ganesha |
| [F-CLU-088](#f-clu-088) | Moving reconnected caps between ranks (IMPORT/EXPORT during rejoin) | supporting | client-wire, mds-wire | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |
| [F-CLU-089](#f-clu-089) | Forced session open during rejoin *(legacy parts)* | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CLU-090](#f-clu-090) | Choosing lock states from reconnected caps | supporting | internal | kclient, libcephfs, internal |
| [F-CLU-091](#f-clu-091) | Scatterlock recovery and NEED_RECOVER | supporting | mds-wire | mds-ranks, internal |
| [F-CLU-092](#f-clu-092) | Snaprealm reconciliation and snapflush rebuild at the end of rejoin | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CLU-093](#f-clu-093) | Rejoin cache repair: invented objects, stale fragmentation, missed link/unlink | internal | mds-wire, on-disk | mds-ranks, osd, internal |
| [F-CLU-094](#f-clu-094) | Rejoin on a single-rank file system | supporting | client-wire, on-disk | kclient, libcephfs, internal, ceph-fuse, ganesha |
| [F-CLU-095](#f-clu-095) | up:clientreplay: ordered re-execution of replayed requests | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CLU-096](#f-clu-096) | Reclaim gating of the clientreplay to active transition | supporting | client-wire | ganesha, libcephfs, mon |

**Activation, post-recovery work and stopping**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-097](#f-clu-097) | Activation (recovery_done and active_start) *(legacy parts)* | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mgr |
| [F-CLU-098](#f-clu-098) | Leftover reconnect caps and cap reissue at activation | supporting | client-wire, metrics | kclient, libcephfs, admin, ceph-fuse, ganesha |
| [F-CLU-099](#f-clu-099) | File size and mtime recovery for writers that did not come back | supporting | client-wire, on-disk, metrics | kclient, libcephfs, ceph-fuse, osd |
| [F-CLU-100](#f-clu-100) | Re-executing interrupted truncates after recovery | supporting | on-disk | osd, kclient, libcephfs |
| [F-CLU-101](#f-clu-101) | Purging orphaned preallocated and delegated inodes | supporting | on-disk | osd, kclient |
| [F-CLU-102](#f-clu-102) | Stopping a rank (up:stopping to down:stopped) | operational | cluster-wire, client-wire, on-disk | mon, mgr, admin, cephadm, kclient, libcephfs, ceph-fuse, ganesha |

**Failure exits**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-104](#f-clu-104) | Reporting a rank as DAMAGED | operational | cluster-wire, metrics | mon, admin, tools, kclient, libcephfs |
| [F-CLU-105](#f-clu-105) | Respawn: re-exec as a fresh standby | operational | cluster-wire | mon, osd, admin, cephadm, rook |
| [F-CLU-106](#f-clu-106) | Suicide and orderly shutdown | operational | cluster-wire, admin | mon, cephadm, rook, admin |
| [F-CLU-107](#f-clu-107) | Write error policy | operational | config, metrics | admin, osd, kclient, libcephfs |

**Daemon process, threads and dispatch**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CLU-108](#f-clu-108) | Process bootstrap (ceph-mds -i <id>) *(legacy parts)* | operational | config, cluster-wire | cephadm, rook, tools, admin |
| [F-CLU-109](#f-clu-109) | Single public messenger with per-peer-type policies *(legacy parts)* | supporting | client-wire, cluster-wire, mds-wire, config | kclient, libcephfs, ceph-fuse, ganesha, mon, mgr, osd, mds-ranks |
| [F-CLU-110](#f-clu-110) | MonClient session, keys and subscriptions *(legacy parts)* | operational | cluster-wire, config | mon, mgr, cephadm, rook |
| [F-CLU-111](#f-clu-111) | Global mds_lock (FIFO fair mutex) | internal | internal | internal |
| [F-CLU-112](#f-clu-112) | Completion contexts and the rank finisher | internal | internal | osd, internal |
| [F-CLU-113](#f-clu-113) | Progress thread and rank waiter queues | internal | internal | kclient, libcephfs, internal |
| [F-CLU-114](#f-clu-114) | Dispatcher chain, message priorities and ordering | internal | internal | kclient, libcephfs, mon, osd, mgr, internal |
| [F-CLU-115](#f-clu-115) | Rank message validation and routing *(legacy parts)* | internal | client-wire, mds-wire | kclient, libcephfs, mon, mds-ranks, internal |
| [F-CLU-116](#f-clu-116) | Outbound messaging helpers and local delivery | internal | client-wire, mds-wire | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks, internal |
| [F-CLU-117](#f-clu-117) | Tick and the shared timer | internal | config | mon, kclient, libcephfs, internal |
| [F-CLU-118](#f-clu-118) | Thread inventory of the daemon and its subsystems | internal | internal | internal |
| [F-CLU-119](#f-clu-119) | MDS-originated client requests (MDSMetaRequest) | internal | mds-wire | mds-ranks, internal |

## 3. Features

### Cluster maps and subscriptions

The monitor-owned FSMap and per-file-system MDSMap, their epochs and encodings, who subscribes to what, and how the kernel client and libcephfs decode and use them.

<a id="f-clu-001"></a>

#### F-CLU-001 · FSMap: monitor-owned map of all file systems

*The Paxos-versioned cluster map that lists every file system, every known MDS daemon and the shared standby pool; read by mgr, tools, cephfs-mirror and libcephfs.*

The FSMap holds every `Filesystem` (fscid, MDSMap, MirrorInfo), the shared standby pool (`standby_daemons`, `standby_epochs`), `mds_roles` (gid to fscid or NONE), `default_compat`, `legacy_client_fscid`, `enable_multiple`, `next_filesystem_id` and `btime`. Only the MDSMonitor on the leader monitor writes it: commands, beacons and the 5 s monitor tick mutate one pending map, which is committed as one FSMap epoch per Paxos proposal (delayed to `paxos_propose_interval` 1 s after the previous commit, or `paxos_min_wait` 0.05 s; immediate when a daemon is blocklisted). Encoding is struct v8, compat 6 (the decoder requires v7 or later); it is sent whole as `MFSMap` (type 45) to `fsmap` subscribers. The monitor store keeps up to `mon_max_mdsmap_epochs` (500) versions. MDS daemons influence the map only through `MMDSBeacon`, `MMDSLoadTargets` and two mon commands.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, mgr, tools, libcephfs, cephfs-mirror
- **Gates:** paxos_propose_interval (1 s); paxos_min_wait (0.05 s); mon_max_mdsmap_epochs (500)
- **Map:** [fsmap-and-mdsmap.md › 3.1 Ownership and propagation](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#31-ownership-and-propagation) · [fsmap-and-mdsmap.md › 4.3 FSMap, Filesystem, MirrorInfo, FSMapUser](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#43-fsmap-filesystem-mirrorinfo-fsmapuser) · [fsmap-and-mdsmap.md › 4.4 Wire encodings](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#44-wire-encodings)
- **Code:** `src/mds/FSMap.h:275 (FSMap)`, `src/mds/FSMap.cc:648-707 (FSMap::encode)`, `src/mon/PaxosFSMap.h:39 (PaxosFSMap::create_pending)`, `src/mon/MDSMonitor.cc:236-344 (MDSMonitor::encode_pending)`
- **Notes:** `FSMap::operator=` does not copy `ever_enabled_multiple` or `struct_version`, and `PaxosFSMap::create_pending` uses it, so the pending map always carries the default `ever_enabled_multiple = true` (fsmap-and-mdsmap section 8, code reading). See also F-CLU-013 (mirroring metadata).

<a id="f-clu-002"></a>

#### F-CLU-002 · MDSMap: per-file-system map and its wire encoding

*The per-file-system projection of the FSMap that MDS daemons and every client decode to find ranks, their states and addresses, and the file system settings.*

Fields: rank sets `in`, `up` (rank to gid), `failed`, `stopped`, `damaged`; `mds_info` (every holder and standby-replay follower as `mds_info_t` v10: global_id, name, rank, inc, state, state_seq, addrs, laggy_since, export_targets, mds_features, join_fscid, flags FROZEN=1, compat); flags; data and metadata pools; `session_timeout` (60), `session_autoclose` (300); `max_file_size`; `max_xattr_size`; `required_client_features`; balancer settings; standby settings; `qdb_cluster_leader/members`; `compat`; `fs_name`; `enabled`; `last_failure_osd_epoch`; `tableserver` and `root` (always 0). Native encoding is struct_v 5, compat 4, extended version 20, carried in `MMDSMap` (type 21, header v2: fsid, u32 epoch, length-prefixed map, always-empty `map_fs_name`); the map is re-encoded per connection for peers lacking PGID64, MDSENC, MSG_ADDR2 or SERVER_NAUTILUS (the kernel advertises all four). Flag bits: NOT_JOINABLE 1<<0, ALLOW_SNAPS 1<<1, ALLOW_MULTIMDS_SNAPS 1<<4, ALLOW_STANDBY_REPLAY 1<<5, REFUSE_CLIENT_SESSION 1<<6, REFUSE_STANDBY_FOR_ANOTHER_FS 1<<7, BALANCE_AUTOMATE 1<<8; default 0x12.

- **Class:** POSIX `supporting` · exposure `cluster-wire`, `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, mgr, mds-ranks
- **Legacy parts** (old-clients): Per-connection re-encoding in MMDSMap::encode_payload: MDSMap struct v2 for peers without PGID64; struct v3 with extended version 5 and the unversioned mds_info_t for peers without MDSENC; mds_info_t v7 carrying only the legacy address for peers without SERVER_NAUTILUS; the legacy entity_addr_t encoding for peers without MSG_ADDR2. Client subscribers receive the MDSMap from the monitor in MMDSMap, whose encode_payload re-encodes the map for a connection that lacks PGID64, MDSENC, MSG_ADDR2 or SERVER_NAUTILUS. The 7.3 kernel advertises all four and current libcephfs advertises CEPH_FEATURES_ALL, so both receive struct_v 5 / extended version 20 with mds_info_t v10. The newest peers that get a re-encoded map are upstream kernels up to v5.10 (SERVER_NAUTILUS was added in 5.11) and Ceph clients before 14.x; the CentOS Stream 8/9 samples advertise SERVER_NAUTILUS and MSG_ADDR2. Evidence: `src/messages/MMDSMap.h:73-82 (MMDSMap::encode_payload)`, `src/mds/MDSMap.cc:739-799 (MDSMap::encode)`, `src/mds/MDSMap.h:145-148 (MDSMap::mds_info_t::encode)`, `src/mds/MDSMap.cc:626-658 (MDSMap::mds_info_t::encode_versioned)`, `src/mds/MDSMap.cc:660-676 (MDSMap::mds_info_t::encode_unversioned)`, `src/msg/msg_types.h:472-492 (entity_addr_t::encode)`, `src/mon/MDSMonitor.cc:1982 (MDSMonitor::check_sub)`, `linux/include/linux/ceph/ceph_features.h:179-220 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:88 (SERVER_NAUTILUS, kernel 5.11)`, `src/include/ceph_features.h:263 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `linux-v5.10/include/linux/ceph/ceph_features.h:178 (CEPH_FEATURES_SUPPORTED_DEFAULT, no SERVER_NAUTILUS)`.
- **Map:** [fsmap-and-mdsmap.md › 4.1 MDSMap fields](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#41-mdsmap-fields) · [fsmap-and-mdsmap.md › 4.2 mds_info_t](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#42-mds_info_t) · [fsmap-and-mdsmap.md › 4.4 Wire encodings](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#44-wire-encodings)
- **Code:** `src/mds/MDSMap.h:68 (MDSMap)`, `src/mds/MDSMap.cc:729-850 (MDSMap::encode)`, `src/mds/MDSMap.cc:869-1030 (MDSMap::decode)`, `src/mds/MDSMap.cc:626-658 (mds_info_t::encode_versioned)`, `src/messages/MMDSMap.h:24-91 (MMDSMap)`
- **Notes:** `ever_allowed_features` is a `uint8_t` while BALANCE_AUTOMATE is bit 8, so that bit is silently dropped; `last_failure` and `cas_pool` are never written (fsmap-and-mdsmap sections 4.1 and 8). Settings owned elsewhere: session timeouts and `refuse_client_session`/`required_client_features` ([sessions](../cpp-map/02-client-protocol/sessions.md)), `max_xattr_size` ([attributes-and-xattrs](../cpp-map/03-request-path/attributes-and-xattrs.md)), snapshot flags ([snapshots](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md)), balancer settings ([balancer-and-pinning](../cpp-map/08-multi-mds/balancer-and-pinning.md)), quiesce membership ([quiesce](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md)).

<a id="f-clu-003"></a>

#### F-CLU-003 · Epoch model: FSMap, MDSMap and standby epochs, null map

*The epoch rules that decide which map is newer, which epoch a beacon must carry and when a map is pushed to every client of a file system.*

Each committed proposal creates FSMap epoch `last_committed + 1`; a file system's MDSMap epoch is set to the FSMap epoch only when that file system or one of its daemons is modified (`modify_filesystem`, `modify_daemon`, `promote`, `erase`, `stop`, ...), so it is sparse but monotonic; a standby's map epoch is its `standby_epochs[gid]` entry; the null map has epoch `UINT32_MAX`. The kernel client, libcephfs and the C++ MDS drop maps whose epoch is not newer than the current one. Every beacon that reaches `prepare_beacon` (state change, health change, laggy clear, `mds_join_fs` change) calls `modify_daemon`, bumping the MDSMap epoch and pushing a new map to every MDS and mounted client of that file system.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `client-wire` · relied on by mon, kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [fsmap-and-mdsmap.md › 3.2 Epochs](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#32-epochs) · [beacon-and-monitor.md › 4. Data structures and invariants](../cpp-map/01-cluster-integration/beacon-and-monitor.md#4-data-structures-and-invariants) · [beacon-and-monitor.md › 3.5 Mon-side processing pipeline](../cpp-map/01-cluster-integration/beacon-and-monitor.md#35-mon-side-processing-pipeline)
- **Code:** `src/mon/PaxosFSMap.h:39-42 (PaxosFSMap::create_pending)`, `src/mds/FSMap.h:496-538 (FSMap::modify_daemon)`, `src/mds/MDSMap.cc:58-63 (MDSMap::create_null_mdsmap)`, `linux/fs/ceph/mds_client.c:7044 (ceph_mdsc_handle_mdsmap)`, `src/mds/MDSDaemon.cc:838 (MDSDaemon::handle_mds_map)`
- **Notes:** `fs reset` leaves the new MDSMap at epoch 0, so subscribers see the reset map only after the next modification (fsmap-and-mdsmap section 8, inferred from code). A health message whose text changes every tick (for example "Behind on trimming (130/128)") produces a new MDSMap epoch roughly every beacon interval (beacon-and-monitor section 8).

<a id="f-clu-004"></a>

#### F-CLU-004 · Map subscriptions and the MDS view of mdsmap (null maps)

*How the monitor decides which map each subscriber gets: clients by fs name or id, MDS daemons by matching their session address vector against the map.*

Subscriptions: `fsmap` (MFSMap: mgr, cephfs-mirror, offline tools, libcephfs one-time for tell), `fsmap.user` (MFSMapUser), `mdsmap` (client: the MDSMap of `legacy_client_fscid`), `mdsmap.<fscid>`; all need mon cap `mds r`; `check_sub` filters the FSMap by the session's allowed fs names and sends nothing while `sub->next > epoch`. An MDS-type session whose address vector equals some `mds_info_t::addrs` receives its file system's MDSMap if it is attached (holder or standby-replay), a null map containing only its own `mds_info` at `standby_epochs[gid]` if it is a standby, or an empty map at the FSMap epoch if unknown; a null map is a default-constructed MDSMap (empty compat that decodes as incompat 7, `enabled` false, `fs_name` "cephfs", no ranks). Beacons from gids not in the committed map (not booting), evicted daemons and a committed STOPPED get a null `MMDSMap` with epoch `UINT32_MAX`.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `client-wire` · relied on by mon, kclient, libcephfs, ceph-fuse, ganesha, mgr, tools, cephfs-mirror
- **Map:** [fsmap-and-mdsmap.md › 3.11 Subscriptions and what each subscriber receives](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#311-subscriptions-and-what-each-subscriber-receives) · [beacon-and-monitor.md › 3.6 Becoming a standby](../cpp-map/01-cluster-integration/beacon-and-monitor.md#36-becoming-a-standby)
- **Code:** `src/mon/MDSMonitor.cc:1902-1913 (MDSMonitor::check_sub)`, `src/mon/MDSMonitor.cc:1953-1968 (MDSMonitor::check_sub)`, `src/mon/Monitor.cc:5560-5566 (Monitor::handle_subscribe)`, `src/mon/MDSMonitor.cc:461 (MDSMonitor::preprocess_beacon)`, `src/mds/MDSDaemon.cc:720-721 (MDSDaemon::init)`
- **Notes:** Because the MDS view is keyed by address equality, the daemon must subscribe and beacon from the same messenger identity whose addresses the monitor recorded at boot (beacon-and-monitor section 3.17). A daemon that applied the `UINT32_MAX` null map without restarting would stall all later beacons in `PaxosService::dispatch` (beacon-and-monitor section 8).

<a id="f-clu-005"></a>

#### F-CLU-005 · FSMapUser: file system name directory for clients

*A small map of (fscid, name) pairs that clients use to mount a file system by name before subscribing to its MDSMap.*

FSMapUser (v1) contains `epoch`, `legacy_client_fscid` and a list of `fs_info_t{s32 cid, string name}`; it is sent as `MFSMapUser` (type 103) to `fsmap.user` subscribers. The kernel client subscribes to it when mounted with `mds_namespace=`, libcephfs when `client_fs` or an fs name is given (one-time); both then subscribe to `mdsmap.<fscid>`. The MDS does not use it.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [fsmap-and-mdsmap.md › 4.3 FSMap, Filesystem, MirrorInfo, FSMapUser](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#43-fsmap-filesystem-mirrorinfo-fsmapuser) · [fsmap-and-mdsmap.md › 3.11 Subscriptions and what each subscriber receives](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#311-subscriptions-and-what-each-subscriber-receives)
- **Code:** `src/mds/FSMapUser.h:29 (FSMapUser)`, `src/mds/FSMapUser.cc:4-42 (FSMapUser::encode)`, `linux/fs/ceph/mds_client.c:6949 (ceph_mdsc_handle_fsmap)`, `linux/fs/ceph/super.c:843 (create_fs_client)`

<a id="f-clu-006"></a>

#### F-CLU-006 · Kernel client partial MDSMap decode

*What the Linux kernel client reads from the MDSMap and how it reacts to rank changes; it fixes which map facts reach kernel mounts.*

The kernel reads epoch, flags (stored in an unused field `m_client_epoch`), root, session_timeout, session_autoclose, max_file_size, max_mds, and per `mds_info` the gid, rank, inc, state, address (msgr2 or legacy), laggy_since and export_targets; entries with a rank out of range or **state <= 0 are ignored**, so standby-replay followers and CREATING/STARTING holders are invisible and such a rank reads as state 0. It then reads data pools, `in` (which sets `possible_max_rank = size(in)`), `enabled`, `fs_name` (the decode fails and the mount is shut down if it differs from `mds_namespace`), the damaged count and (ev >= 18) `max_xattr_size`; reads past the end after the pools still return the partial map as success. `check_new_map` force-closes sessions of ranks `>= possible_max_rank`, restarts a session on an address change, sends a reconnect when a restarting rank reaches `>= RECONNECT`, kicks requests and cap flushes at `>= ACTIVE`, and uses export targets of laggy or reconnecting ranks.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient
- **Legacy parts** (msgr1): Selection of the LEGACY (msgr1) address from each mds_info address vector, the 'legacy' alternative of 'address (msgr2 or legacy)'. For each mds_info entry with info_v >= 8 the kernel takes the first address of the type its messenger speaks: MSGR2 when an ms_mode option is set, LEGACY when none is. mount.ceph with the new device syntax adds ms_mode=prefer-crc, so the baseline kernel selects the msgr2 address. LEGACY selection happens only with ms_mode=legacy or the old mon:/path device syntax, where the kernel's built-in default is msgr1. Evidence: `linux/fs/ceph/mdsmap.c:214-217 (ceph_mdsmap_decode)`, `linux/net/ceph/decode.c:96-100 (ceph_decode_entity_addrvec)`, `linux/fs/ceph/mds_client.c:7050 (ceph_mdsc_handle_mdsmap)`, `linux/include/linux/ceph/libceph.h:146-149 (ceph_msgr2)`, `linux/net/ceph/ceph_common.c:329-330 (ceph_alloc_options)`, `src/mount/mount.ceph.c:43 (CEPH_DEFAULT_V2_MS_MODE)`, `src/mount/mount.ceph.c:306-309 (parse_new_dev)`.
- **Map:** [fsmap-and-mdsmap.md › 3.12 How clients decode and use the MDSMap](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#312-how-clients-decode-and-use-the-mdsmap) · [fsmap-and-mdsmap.md › 7. Externally visible contract](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#7-externally-visible-contract)
- **Code:** `linux/fs/ceph/mdsmap.c:119-441 (ceph_mdsmap_decode)`, `linux/fs/ceph/mdsmap.c:248-257 (ceph_mdsmap_decode)`, `linux/fs/ceph/mds_client.c:5843 (check_new_map)`, `linux/fs/ceph/mds_client.c:7078-7082 (ceph_mdsc_handle_mdsmap)`
- **Notes:** Kernel `required_client_features` skip mismatch: the kernel skips the field as "u32 n then n x u64" while it is encoded as "u32 byte length then bytes"; with a non-empty set the skip over-runs, the later `max_xattr_size` read fails (map still accepted, `m_max_xattr_size` 0, all setxattr synchronous) or reads garbage (fsmap-and-mdsmap section 8, inferred from code, not runtime-verified). Ranks are array indices bounded by `size(in)`, so ranks must stay contiguous. See also F-PROTO-062 (client reactions to rank changes).

<a id="f-clu-007"></a>

#### F-CLU-007 · libcephfs map use and mount availability check

*How libcephfs, ceph-fuse and NFS-Ganesha decode and use the MDSMap, including the mount-time availability test.*

libcephfs decodes the whole map with the C++ `MDSMap::decode` (unknown trailing fields are skipped via the struct length). `Client::mount` fails with EACCES before contacting any MDS when REFUSE_CLIENT_SESSION is set, and checks `is_cluster_available()` only when `require_mds` is set: ceph-fuse passes `fuse_require_active_mds` (default true), `ceph_mount` passes false; STUCK (any damaged rank, empty `in`, any laggy up rank, or no ACTIVE rank) fails the mount with `CEPH_FUSE_NO_MDS_UP`, TRANSIENT (epoch 0) waits. It also uses `max_mds` to recognise stopped ranks, `export_targets` to pre-open sessions, `session_timeout/3` for cap renewal, `session_autoclose` as the upper bound of `ceph_set_deleg_timeout` (EINVAL otherwise, used by Ganesha delegations), `max_file_size` for writes and truncates, and the data pools for statfs; `mds_command` skips laggy daemons.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by libcephfs, ceph-fuse, ganesha
- **Gates:** fuse_require_active_mds (default true); REFUSE_CLIENT_SESSION flag
- **Map:** [fsmap-and-mdsmap.md › 3.12 How clients decode and use the MDSMap](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#312-how-clients-decode-and-use-the-mdsmap) · [beacon-and-monitor.md › 7. Externally visible contract](../cpp-map/01-cluster-integration/beacon-and-monitor.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSMap.cc:1032-1076 (MDSMap::is_cluster_available)`, `src/client/Client.cc:7066-7093 (Client::mount)`, `src/client/Client.cc:3360-3436 (Client::handle_mds_map)`, `src/ceph_fuse.cc:318 (main)`, `src/libcephfs.cc:196 (ceph_mount_info::mount)`

<a id="f-clu-008"></a>

#### F-CLU-008 · FSMap filtering for file-system-restricted sessions

*The monitor hides file systems a session is not allowed to see from its subscriptions and read-only commands.*

Before serving a subscription or a read-only `fs`/`mds` command, the monitor filters the FSMap by the session's allowed fs names (`FSMap::filter`); mutating `fs` commands check `fsname` write capability per file system (EPERM) and return ENOENT for unknown file systems except `fs rm`, `fs rename` and `fs swap`. `fs dump <epoch>` decodes a stored historical FSMap and is not filtered.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by kclient, libcephfs, mgr, admin
- **Map:** [fsmap-and-mdsmap.md › 3.11 Subscriptions and what each subscriber receives](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#311-subscriptions-and-what-each-subscriber-receives) · [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states)
- **Code:** `src/mds/FSMap.h:318-331 (FSMap::filter)`, `src/mon/FSCommands.cc:2031 (is_op_allowed)`, `src/mon/MDSMonitor.cc:1029-1031 (MDSMonitor::preprocess_command)`
- **Notes:** `FSMap::filter` removes the `mds_roles` entries of standbys but keeps `standby_daemons`, so filtered maps are internally inconsistent for standbys (fsmap-and-mdsmap section 8).

<a id="f-clu-009"></a>

#### F-CLU-009 · FSMap sanity invariants enforced by the monitor

*Assertions the monitor applies to every committed or decoded FSMap; they bound the states the monitor can ever publish.*

`FSMap::sanity` checks, unless `mon_mds_skip_sanity`: `legacy_client_fscid` is NONE or an existing fs; every daemon in a fs has a rank, maps to that fs in `mds_roles` and is not a standby; a non-standby-replay daemon is the `up` holder of its rank and the rank is neither failed nor damaged; standby-replay daemons exist only with `allow_standby_replay` (pending maps); every daemon compat is writeable for the fs compat; the quiesce leader is a member and members are in `mds_info`; `up` is a subset of `in`; standbys are STANDBY, rank NONE, with a `standby_epochs` entry. A violation asserts on the leader monitor. Not checked: `up + failed + damaged == in`, `in` and `stopped` disjoint, epochs.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon
- **Gates:** mon_mds_skip_sanity (default false)
- **Map:** [fsmap-and-mdsmap.md › 4.5 Invariants (FSMap::sanity, src/mds/FSMap.cc:943)](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#45-invariants-fsmapsanity-srcmdsfsmapcc943) · [beacon-and-monitor.md › 4. Data structures and invariants](../cpp-map/01-cluster-integration/beacon-and-monitor.md#4-data-structures-and-invariants)
- **Code:** `src/mds/FSMap.cc:943-1004 (FSMap::sanity)`
- **Notes:** Several command sequences reach monitor asserts or `std::terminate` by code reading: `fs compat add_incompat` while standby-replay daemons remain, `fs reset` with a standby-replay still attached, `mds set_state` on a standby gid, `mds rmfailed` followed by a shrink, `fs compat add_*` with id >= 64 (fsmap-and-mdsmap section 8, inferred from code).

<a id="f-clu-010"></a>

#### F-CLU-010 · Multiple file systems: enable_multiple and the default file system

*Whether more than one file system may exist, and which one unqualified clients and rank-only roles resolve to.*

`fs flag set enable_multiple <bool>` (default true) controls creation of a second file system. `legacy_client_fscid` (dumped as `default_fscid`) is set to the first file system committed or by `fs set-default` (alias `fs set_default`); unqualified `mdsmap` subscriptions receive that file system's map and a bare `<rank>` role parses only when it exists. `fs rm` of the default fs clears it without choosing another, after which unqualified subscribers receive nothing.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by kclient, libcephfs, admin
- **Legacy parts** (deprecated-admin): The deprecated command alias `fs set_default`. `fs set_default` is an AliasHandler of `fs set-default` registered with FLAG(DEPRECATED); a grep of src/pybind/mgr, src/cephadm, src/tools and Rook's Go sources finds no use of it. `fs set-default`, enable_multiple and the default file system are current: libcephfs mounted without client_fs or an fs name subscribes to the unqualified `mdsmap`, which the monitor serves from legacy_client_fscid. Evidence: `src/mon/MonCommands.h:471-474 (fs set_default, FLAG(DEPRECATED))`, `src/mon/FSCommands.cc:1910-1912 (AliasHandler<SetDefaultHandler>)`, `src/client/Client.cc:7011-7035 (Client::subscribe_mdsmap)`.
- **Map:** [fsmap-and-mdsmap.md › 4.3 FSMap, Filesystem, MirrorInfo, FSMapUser](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#43-fsmap-filesystem-mirrorinfo-fsmapuser) · [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states)
- **Code:** `src/mds/FSMap.cc:497-538 (FSMap::commit_filesystem)`, `src/mon/FSCommands.cc:43 (FlagSetHandler)`, `src/mds/FSMap.cc:1225-1289 (FSMap::parse_role)`

<a id="f-clu-011"></a>

#### F-CLU-011 · Data pool list of a file system

*The `data_pools` list in the MDSMap that clients and the MDS use to validate file layouts and to report statfs.*

`data_pools` starts with the data pool given to `fs new` (the default pool, which cannot be removed); `fs add_data_pool` runs the same pool checks as `fs new` and tags the pool with application `cephfs` (adding an existing pool succeeds), `fs rm_data_pool` refuses the first pool (EINVAL), silently succeeds for an absent pool and does not check whether layouts still use the pool. `sanitize` drops pools no longer in the OSDMap. The kernel validates layout ioctls against the list and reports the single pool in statfs when exactly one exists; libcephfs uses it for statfs; the C++ MDS validates layouts after waiting for the client's MDSMap epoch.

- **Class:** POSIX `extension` · exposure `cluster-wire`, `client-wire`, `admin` · relied on by kclient, libcephfs, admin, osd, ceph-fuse
- **Map:** [fsmap-and-mdsmap.md › 4.1 MDSMap fields](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#41-mdsmap-fields) · [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states)
- **Code:** `src/mon/FSCommands.cc:1055 (AddDataPoolHandler)`, `src/mon/FSCommands.cc:1588 (RemoveDataPoolHandler)`, `src/mds/MDSMap.cc:852 (MDSMap::sanitize)`, `linux/fs/ceph/ioctl.c:53 (__validate_layout)`, `linux/fs/ceph/super.c:68 (ceph_statfs)`

<a id="f-clu-012"></a>

#### F-CLU-012 · max_file_size published in the MDSMap

*The per-file-system maximum file size that clients enforce; the MDS only encodes and dumps it.*

`max_file_size` defaults to 1 TiB and is set by `fs set <fs> max_file_size` (values below 65536 give ERANGE). The kernel sets `fsc->max_file_size = min(max_file_size, MAX_LFS_FILESIZE)` on every map; libcephfs checks it in `_do_setattr` (truncate) and `_write`. The C++ MDS performs no size check against it.

- **Class:** POSIX `supporting` · exposure `client-wire`, `admin` · relied on by kclient, libcephfs, ceph-fuse, admin
- **Map:** [fsmap-and-mdsmap.md › 4.1 MDSMap fields](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#41-mdsmap-fields) · [fsmap-and-mdsmap.md › 3.12 How clients decode and use the MDSMap](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#312-how-clients-decode-and-use-the-mdsmap)
- **Code:** `linux/fs/ceph/mds_client.c:7023-7072 (ceph_mdsc_handle_mdsmap)`, `src/client/Client.cc:8711 (Client::_do_setattr)`, `src/client/Client.cc:12808 (Client::_write)`, `src/mon/FSCommands.cc:418-859 (FileSystemCommandHandler::set_val)`

<a id="f-clu-013"></a>

#### F-CLU-013 · Mirroring metadata (MirrorInfo) in the FSMap

*Per-file-system mirroring state and peer list stored in the FSMap for cephfs-mirror; the MDS does not read it.*

`fs mirror enable|disable <fs>`, `fs mirror peer_add <fs> <uuid> <client>@<cluster> <remote_fs>` and `peer_remove` edit `MirrorInfo{mirrored, peers}` where a peer is a uuid plus `{client_name, cluster_name, fs_name}`; peer operations need mirroring enabled and the spec must contain `@`; disable clears the peers. cephfs-mirror receives it through the `fsmap` subscription. `fs rename` and `fs swap` require mirroring disabled.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by cephfs-mirror, mgr, admin
- **Map:** [fsmap-and-mdsmap.md › 4.3 FSMap, Filesystem, MirrorInfo, FSMapUser](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#43-fsmap-filesystem-mirrorinfo-fsmapuser) · [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states)
- **Code:** `src/mds/FSMap.h:128-181 (MirrorInfo)`, `src/mon/FSCommands.cc:1686-1890 (MirrorHandlerEnable)`, `src/tools/cephfs_mirror/ClusterWatcher.cc:49 (ClusterWatcher::init)`
- **Notes:** See also F-CLU-001 (FSMap).

### Compat sets and upgrade gating

Feature compat sets that decide which daemons may serve which file system, and the upgrade path built on them.

<a id="f-clu-014"></a>

#### F-CLU-014 · CompatSet encoding and the MDS incompat feature ids

*The three-mask feature set the monitor uses to decide which daemons may serve which file system.*

A `CompatSet` is three `FeatureSet`s (compat, ro_compat, incompat), each a u64 mask plus `map<u64 id, string name>`; ids must be 1..63; bit 0 is set in memory and cleared on the wire; there is no version header. `A.readable(B)`: B.incompat is a subset of A.incompat; `A.writeable(B)`: readable and B.ro_compat a subset of A.ro_compat (plain `compat` ignored); `compare` returns 0 only if all masks are equal; `merge` copies missing features with their names. MDS incompat ids: 1 base v0.20, 2 client writeable ranges, 3 default file layouts on dirs, 4 dir inode in separate object, 5 versioned encoding, 6 dirfrag in omap, 7 inline data, 8 no anchor table, 9 file layout v2, 10 snaprealm v2, 11 minor log segments, 12 quiesce subvolumes. C++ daemons advertise all 12 (`get_compat_set_all`); new file systems get `default_compat` (all but 7 initially); no compat or ro_compat MDS features exist.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, cephadm, admin
- **Map:** [fsmap-and-mdsmap.md › 3.5 CompatSet mechanics](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#35-compatset-mechanics) · [fsmap-and-mdsmap.md › 4.4 Wire encodings](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#44-wire-encodings)
- **Code:** `src/include/CompatSet.h:88-134 (CompatSet)`, `src/common/CompatSet.cc:23-83 (FeatureSet::encode)`, `src/mds/MDSMap.h:45-56 (MDS_FEATURE_INCOMPAT_BASE)`, `src/mds/MDSMap.cc:66-131 (MDSMap::get_compat_set_all)`
- **Notes:** Every mask bit needs a name: `merge` asserts on a missing name (fsmap-and-mdsmap section 7 item 1). Because `default_compat` is set once when the FSMap is created and only `mds compat rm_*` can change it, long-lived clusters keep the default of the release that created the FSMap.

<a id="f-clu-015"></a>

#### F-CLU-015 · Standby eligibility by compat set

*Compat filters that decide whether a standby may be promoted into (or follow a rank of) a file system.*

A standby S is skipped for file system F when `!S.compat.writeable(F.compat)` (S lacks an incompat or ro_compat bit of F), or when F is not upgradeable and `!F.compat.writeable(S.compat)` (S has incompat/ro_compat bits F lacks). Plain compat bits never matter and only masks are compared, never names. A file system given a private incompat bit is never joined by daemons lacking it, and other file systems are unaffected (qa test_standby_incompat_reject, test_standby_incompat_reject_multifs).

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, cephadm, admin
- **Map:** [fsmap-and-mdsmap.md › 3.9 What prevents a standby from being assigned to a file...](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#39-what-prevents-a-standby-from-being-assigned-to-a-file-system) · [fsmap-and-mdsmap.md › 3.5 CompatSet mechanics](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#35-compatset-mechanics)
- **Code:** `src/mds/FSMap.cc:804-873 (FSMap::get_available_standby)`, `qa/tasks/cephfs/test_admin.py:1377-1440 (test_standby_compat)`

<a id="f-clu-016"></a>

#### F-CLU-016 · Forced INLINE incompat bit and compat bootstrap

*The monitor inserts incompat 7 into every stored daemon and file system compat, which every daemon must then echo forever.*

`FSMap::insert` (boot) replaces an empty boot compat by the v16.2.4 set and then always inserts incompat 7 ("mds uses inline data"); `MDSMap::decode` inserts 7 into the fs compat and into each `mds_info` compat (an empty one first inherits the fs compat), with a "TODO: remove after R is released" comment. `mds_info_t` before v10 decodes with the v16.2.4 set and MDSMap ev < 3 with the base set. Consequence: a daemon whose beacons omit bit 7 differs from its stored compat and is failed at its first beacon that reaches `prepare_beacon`.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon
- **Legacy parts** (old-daemons): FSMap::insert replacing an empty boot compat with get_compat_set_v16_2_4(); mds_info_t::decode defaulting to the v16.2.4 set for struct_v < 10; MDSMap::decode defaulting to the base set for extended version < 3 and copying the fs compat into an empty mds_info compat. Commit d341c5b7734 (2022) added the INLINE insertion for upgrades from MDS daemons of v16.2.4 and older, but the current monitor applies it to every boot beacon and every decoded map, so a daemon at this commit whose beacons omit bit 7 is failed; the C++ MDS sends get_compat_set_all(), which contains bit 7. The empty-compat bootstrap and the decode defaults serve only boot beacons from MDS v16.2.4 and older and maps encoded by monitors that predate mds_info_t v10 (first in v16.2.6 and v17.1.0). The current MDS always sends a non-empty compat. Evidence: `src/mds/FSMap.cc:1169-1173 (FSMap::insert)`, `src/mds/FSMap.cc:1174-1177 (FSMap::insert)`, `src/mds/MDSMap.cc:713-717 (MDSMap::mds_info_t::decode)`, `src/mds/MDSMap.cc:904-907 (MDSMap::decode)`, `src/mds/MDSMap.cc:1013-1026 (MDSMap::decode)`, `src/mds/MDSMap.cc:115-130 (MDSMap::get_compat_set_v16_2_4)`, `src/mds/Beacon.cc:57 (Beacon::Beacon)`, `src/mds/Beacon.cc:227 (Beacon::_send)`, `v16.2.6:src/mds/MDSMap.cc:529 (MDSMap::mds_info_t::encode_versioned)`.
- **Map:** [fsmap-and-mdsmap.md › 3.5 CompatSet mechanics](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#35-compatset-mechanics) · [beacon-and-monitor.md › 3.17 Minimum behaviour required by an unmodified monitor](../cpp-map/01-cluster-integration/beacon-and-monitor.md#317-minimum-behaviour-required-by-an-unmodified-monitor)
- **Code:** `src/mds/FSMap.cc:1160-1179 (FSMap::insert)`, `src/mds/MDSMap.cc:1013-1027 (MDSMap::decode)`, `src/mds/MDSMap.cc:713-717 (mds_info_t::decode)`

<a id="f-clu-017"></a>

#### F-CLU-017 · Compat set must not change after boot

*Every beacon must carry exactly the compat stored at boot, otherwise the monitor fails the daemon.*

In `prepare_beacon`, if `beacon.compat.compare(info.compat) != 0` the monitor logs a clog WARN "compat changed unexpectedly" and calls `fail_mds_gid` (blocklisting a rank holder); the next beacon then gets a null map. The check is not in `preprocess_beacon`, so unchanged STANDBY beacons are acked without it; it fires on the first beacon that needs a proposal (a requested state change, changed health, a changed `mds_join_fs` resolution or clearing `laggy`), which can be long after boot.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon
- **Map:** [fsmap-and-mdsmap.md › 3.5 CompatSet mechanics](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#35-compatset-mechanics) · [beacon-and-monitor.md › 3.5 Mon-side processing pipeline](../cpp-map/01-cluster-integration/beacon-and-monitor.md#35-mon-side-processing-pipeline)
- **Code:** `src/mon/MDSMonitor.cc:754-764 (MDSMonitor::prepare_beacon)`, `src/include/CompatSet.h:123-134 (CompatSet::compare)`

<a id="f-clu-018"></a>

#### F-CLU-018 · Upgradeable file systems and compat merge on promotion

*The rule that lets a daemon with new incompat bits join a file system and permanently upgrades that file system's compat; cephadm upgrades depend on it.*

`Filesystem::is_upgradeable()` is true when `in` is empty, or standby-replay is disallowed and either exactly one rank is in (even while it is up and healthy) or no rank is up. When `promote` moves a daemon whose compat the fs compat cannot write, it asserts upgradeability and merges the daemon compat into the fs compat permanently; afterwards daemons lacking those bits fail the eligibility filter for that file system. Standby-replay assignment never merges. cephadm's MDS upgrade disables `allow_standby_replay` and reduces `max_mds` to 1 (or runs `fs fail`) before restarting daemons, then waits for `up:active` in `fs dump`.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, cephadm, admin
- **Legacy parts** (old-daemons): Admitting and merging new incompat bits into a file system that already has ranks: is_upgradeable() through the standby-replay-disallowed clauses and the get_available_standby exception it controls, which exist for MDS upgrades across releases with different compat sets. At this commit `fs new` gives the new file system default_compat, which lacks INLINE (bit 7), and promotes standbys whose stored compat has bit 7 in the same pending map, so the promote-time merge already runs on `fs new` while `in` is empty (is_upgradeable() true). The other is_upgradeable() cases (standby-replay disallowed with exactly one rank in, or no rank up) matter only when a daemon carries incompat bits that an existing file system lacks, which happens when daemons of a release with additional bits replace older ones. cephadm and Rook run their MDS scale-down (allow_standby_replay off, max_mds 1 or fs fail) whenever they move MDS daemons to a new Ceph image or version. Evidence: `src/mds/FSMap.cc:480 (FSMap::create_filesystem)`, `src/mds/MDSMap.cc:86-102 (MDSMap::get_compat_set_default)`, `src/mon/FSCommands.cc:342-353 (FsNewHandler::handle)`, `src/mds/FSMap.cc:1031-1034 (FSMap::promote)`, `src/mds/FSMap.h:207-218 (Filesystem::is_upgradeable)`, `src/mds/FSMap.cc:808-823 (FSMap::get_available_standby)`, `src/pybind/mgr/cephadm/upgrade.py:1431-1538 (CephadmUpgrade._prepare_for_mds_upgrade)`, `src/pybind/mgr/cephadm/upgrade.py:2403-2408 (CephadmUpgrade._do_upgrade)`, `rook/pkg/operator/ceph/file/mds/mds.go:111-121 (Cluster.Start)`, `rook/pkg/operator/ceph/file/mds/mds.go:266-323 (Cluster.upgradeMDS)`.
- **Map:** [fsmap-and-mdsmap.md › 3.9 What prevents a standby from being assigned to a file...](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#39-what-prevents-a-standby-from-being-assigned-to-a-file-system) · [beacon-and-monitor.md › 3.13 Admin and orchestrator commands that change membership](../cpp-map/01-cluster-integration/beacon-and-monitor.md#313-admin-and-orchestrator-commands-that-change-membership)
- **Code:** `src/mds/FSMap.h:207-218 (Filesystem::is_upgradeable)`, `src/mds/FSMap.cc:1031-1034 (FSMap::promote)`, `src/pybind/mgr/cephadm/upgrade.py:1430-1530 (CephadmUpgrade._prepare_for_mds_upgrade)`
- **Notes:** Only `fs compat <fs> rm_*` (with no rank up) undoes a merge. `promote` hits `ceph_assert(fs.is_upgradeable())` if a standby-replay daemon left attached after `fs compat rm_incompat` is promoted while `allow_standby_replay` is set (fsmap-and-mdsmap section 8, inferred from code). See also F-ADM-114 (cephadm upgrade choreography).

<a id="f-clu-019"></a>

#### F-CLU-019 · fs compat and mds compat commands

*Operator commands that edit a file system's compat set or the default compat for new file systems.*

`fs compat <fs> add_compat|add_incompat|rm_compat|rm_incompat <id> [<name>]` edits one fs compat only while no rank is up (EBUSY otherwise); id <= 0 is EINVAL, add needs a name (EINVAL), adding an existing id with a different name is EEXIST; there is no ro_compat sub-op. Deprecated `mds compat rm_compat|rm_incompat <id>` edits `default_compat`, used by later `fs new`/`fs reset`. `fs compat show` and `mds compat show` display them.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin
- **Legacy parts** (deprecated-admin): The deprecated `mds compat show`, `mds compat rm_compat` and `mds compat rm_incompat` commands that operate on default_compat. `mds compat show`, `mds compat rm_compat` and `mds compat rm_incompat` are registered with FLAG(DEPRECATED) and are the only commands that show or edit default_compat; a grep of src/pybind/mgr, src/cephadm, src/tools and Rook's Go sources finds no use. `fs compat <fs> ...` and `fs compat show` are not deprecated. Evidence: `src/mon/MonCommands.h:366-377 (mds compat show/rm_compat/rm_incompat, FLAG(DEPRECATED))`, `src/mon/MDSMonitor.cc:1739-1768 (MDSMonitor::filesystem_command)`, `src/mon/MDSMonitor.cc:1250-1259 (MDSMonitor::preprocess_command)`.
- **Map:** [fsmap-and-mdsmap.md › 3.5 CompatSet mechanics](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#35-compatset-mechanics) · [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states)
- **Code:** `src/mon/FSCommands.cc:861-964 (CompatSetHandler)`, `src/mon/MDSMonitor.cc:1739-1768 (MDSMonitor::filesystem_command)`
- **Notes:** `fs compat add_*` with id >= 64 reaches `ceph_assert(f.id < 64)` in `FeatureSet::insert` and aborts the monitor (inferred from code).

<a id="f-clu-020"></a>

#### F-CLU-020 · mds_features in the boot beacon and get_up_features

*The messenger feature bits each daemon advertises at boot; MDS daemons AND them across all rank holders to choose encodings of stored and inter-rank data.*

The beacon's u64 `mds_features` (the C++ MDS sends `CEPH_FEATURES_SUPPORTED_DEFAULT`) is stored in `mds_info_t::mds_features` at boot only and is not interpreted by the monitor. `MDSMap::get_up_features()` returns the AND over all rank holders; the C++ MDS uses it to pick encodings of journal events, inode stores, session maps and migration payloads.

- **Class:** POSIX `internal` · exposure `cluster-wire`, `on-disk` · relied on by mds-ranks, internal
- **Legacy parts** (old-daemons): Selecting older encodings of journal events, inode stores, session maps, dirfrag commits and inter-rank payloads when the AND of the up ranks' mds_features lacks a bit (mixed-version ranks). Every C++ MDS at this commit reports CEPH_FEATURES_SUPPORTED_DEFAULT (= CEPH_FEATURES_ALL) as mds_features, so the AND over rank holders is the full set and the feature-dependent encoders take their current branches. The older branches reached through get_up_features() (file_layout_t without FS_FILE_LAYOUT_V2, entity_addr_t without MSG_ADDR2 or SERVER_NAUTILUS) are taken only when an older C++ MDS holds a rank of the same file system. The beacon field itself is still decoded, stored in mds_info_t and dumped by the current monitor. Evidence: `src/mds/Beacon.cc:219-225 (Beacon::_send)`, `src/include/ceph_features.h:263 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/mds/MDSMap.cc:1220-1234 (MDSMap::get_up_features)`, `src/common/fs_types.cc:89-98 (file_layout_t::encode)`, `src/msg/msg_types.h:472-492 (entity_addr_t::encode)`, `src/mds/MDLog.cc:430 (MDLog::_submit_entry)`, `src/mds/CInode.cc:1284 (CInode::store)`, `src/mds/SessionMap.cc:438 (SessionMap::validate_and_encode_session)`, `src/mds/CDir.cc:2742 (CDir::_parse_dentry)`.
- **Map:** [beacon-and-monitor.md › 3.4 MMDSBeacon wire format](../cpp-map/01-cluster-integration/beacon-and-monitor.md#34-mmdsbeacon-wire-format) · [fsmap-and-mdsmap.md › 3.4 Rank sets and derived predicates](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#34-rank-sets-and-derived-predicates)
- **Code:** `src/mds/MDSMap.cc:1220-1234 (MDSMap::get_up_features)`, `src/mon/MDSMonitor.cc:716 (MDSMonitor::prepare_beacon)`, `src/mds/MDLog.cc:430 (MDLog::_submit_entry)`, `src/mds/CInode.cc:1284 (CInode::store)`

### Beacons and liveness

The beacon protocol between each MDS and the monitors, liveness on both sides, and health metric transport.

<a id="f-clu-021"></a>

#### F-CLU-021 · MMDSBeacon message and the beacon sender thread

*The periodic MDS-to-monitor message that carries liveness, the wanted state, the last map epoch seen, compat, health and affinity.*

`MSG_MDS_BEACON` = 100, HEAD_VERSION 8, COMPAT_VERSION 6, priority 196. Payload order: PaxosServiceMessage `version` (last MDSMap epoch seen), deprecated session mon (s16 -1) and tid (u64 0), fsid, `global_id` (MonClient global id), `state` (wanted state), `seq` (strictly increasing per process), `name`, `standby_for_rank`/`standby_for_name` (ignored), `compat`, `health` (`MDSHealth` v1 of `MDSHealthMetric` v1 {u16 type, u8 sev, message, metadata}), `sys_info` (only when state is `up:boot`), `mds_features`, `standby_for_fscid` (ignored), `standby_replay` (v>=7, ignored), `fs` (v>=8, `mds_join_fs`). A dedicated `mds-beacon` thread sends one whenever `0.9 * mds_beacon_interval` (4 s) has elapsed, using its own mutex and copies of epoch, wanted state, compat and health so it never needs `mds_lock`; the MonClient queues beacons while it has no mon session. `mds_beacon_interval` is read only at construction.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config` · relied on by mon
- **Legacy parts** (old-daemons): MMDSBeacon::decode_payload handling of header versions 6 and 7: mapping a requested up:standby-replay to up:standby for v < 7 and omitting the fs (mds_join_fs) field for v < 8. The C++ MDS sends v8 beacons with the ignored standby fields filled with constants, and the current monitor decodes the whole v8 layout, so those vestigial fields are part of the current wire format. The decoder also accepts v6 beacons, last sent by 10.x MDS daemons, and v7 beacons, sent by 11.x to 14.x daemons; v8 first shipped in 15.2.0 (15.1.0 development tag). Evidence: `src/messages/MMDSBeacon.h:249-250 (MMDSBeacon::HEAD_VERSION, COMPAT_VERSION)`, `src/messages/MMDSBeacon.h:314-332 (MMDSBeacon::encode_payload)`, `src/messages/MMDSBeacon.h:363-375 (MMDSBeacon::decode_payload)`, `v10.2.0:src/messages/MMDSBeacon.h:125 (HEAD_VERSION = 6)`, `v14.2.0:src/messages/MMDSBeacon.h:187 (HEAD_VERSION = 7)`, `v15.2.0:src/messages/MMDSBeacon.h:185 (HEAD_VERSION = 8)`.
- **Gates:** mds_beacon_interval (4 s, read once)
- **Map:** [beacon-and-monitor.md › 3.3 The MDS-side beacon engine](../cpp-map/01-cluster-integration/beacon-and-monitor.md#33-the-mds-side-beacon-engine) · [beacon-and-monitor.md › 3.4 MMDSBeacon wire format](../cpp-map/01-cluster-integration/beacon-and-monitor.md#34-mmdsbeacon-wire-format) · [process-and-threading.md › 3.12 Tick, timers, heartbeat and lagginess](../cpp-map/00-architecture/process-and-threading.md#312-tick-timers-heartbeat-and-lagginess)
- **Code:** `src/messages/MMDSBeacon.h:246-382 (MMDSBeacon)`, `src/mds/Beacon.cc:198-239 (Beacon::_send)`, `src/mds/Beacon.cc:84-112 (Beacon::init)`, `src/mds/Beacon.cc:51-59 (Beacon::Beacon)`
- **Notes:** `MDSHealthMetric` type 0 (`MDS_HEALTH_NULL`) hits `ceph_assert` in the monitor's decoder, so a beacon carrying it aborts the monitor that decodes it; decoding a v<7 beacon maps a requested `up:standby-replay` to `up:standby` (beacon-and-monitor section 3.4).

<a id="f-clu-022"></a>

#### F-CLU-022 · Monitor beacon admission rules

*The checks the monitor applies to every beacon before it acts on it; a beacon failing them is ignored, acked only, or answered with a null map.*

`PaxosService::dispatch` drops forwarded messages from older elections and disconnected sessions and **waits until the committed FSMap epoch is >= the beacon `version`**. `preprocess_beacon` then, in order: ignores sessions without mon cap `mds x`, a wrong fsid, or a source port 0; peons forward to the leader; an unknown gid not booting gets a null map; a duplicate boot is ignored; DNE goes to prepare; `state_seq > seq` is ignored ("old seq"); a `version` different from the daemon's effective epoch (its fs MDSMap epoch or `standby_epochs[gid]`) is **acked only, without processing the requested state or health**; laggy daemons, a changed `mds_join_fs` resolution, a changed state or changed health (type, severity, message) go to `prepare_beacon`; otherwise ack. The ack is an `MMDSBeacon` echoing the seq and state with empty health/compat; the MDS uses only the seq. Liveness (`last_beacon`) is noted for acked and prepared beacons but not for old-seq beacons or repeated `up:boot`.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon
- **Map:** [beacon-and-monitor.md › 3.5 Mon-side processing pipeline](../cpp-map/01-cluster-integration/beacon-and-monitor.md#35-mon-side-processing-pipeline) · [beacon-and-monitor.md › 7. Externally visible contract](../cpp-map/01-cluster-integration/beacon-and-monitor.md#7-externally-visible-contract)
- **Code:** `src/mon/PaxosService.cc:48-152 (PaxosService::dispatch)`, `src/mon/MDSMonitor.cc:408-564 (MDSMonitor::preprocess_beacon)`, `src/mon/MDSMonitor.cc:633-934 (MDSMonitor::prepare_beacon)`, `src/mon/MDSMonitor.cc:961-981 (MDSMonitor::_updated)`
- **Notes:** A beacon whose `version` is larger than the committed epoch stalls in `PaxosService::dispatch`; a stale epoch silently prevents state changes and health updates (beacon-and-monitor section 8). Clearing `laggy_since` also needs the new epoch, because marking a daemon laggy bumps its map epoch.

<a id="f-clu-023"></a>

#### F-CLU-023 · Beacon acks and MDS-side lagginess (self-fencing deferral)

*The MDS measures beacon acks and, when none arrived for `mds_beacon_grace`, stops processing all rank messages and periodic work until the monitor answers again.*

Only `MSG_MDS_BEACON` from a monitor is accepted as an ack; it is matched by `seq`, `last_acked_stamp` becomes the send time of that beacon and laggy is cleared if the round trip is below `mds_beacon_grace`. `Beacon::is_laggy()` is true when `now - last_acked_stamp > mds_beacon_grace` (15 s, re-read at runtime) and is true until the first ack. While laggy: `MDSRankDispatcher::tick` returns before any upkeep (no session staleness, reconnect timeout, balancer, stopping passes, health refresh); every message reaching `MDSRank::_dispatch` (client and peer alike, except quiesce-DB peer messages) is appended to `waiting_for_nolaggy` and replayed in order by the progress thread afterwards; clients are not marked stale until `queue_max_age + session_timeout` after lagginess clears.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config` · relied on by mon, kclient, libcephfs, internal
- **Gates:** mds_beacon_grace (15 s)
- **Map:** [beacon-and-monitor.md › 3.3 The MDS-side beacon engine](../cpp-map/01-cluster-integration/beacon-and-monitor.md#33-the-mds-side-beacon-engine) · [process-and-threading.md › 3.12 Tick, timers, heartbeat and lagginess](../cpp-map/00-architecture/process-and-threading.md#312-tick-timers-heartbeat-and-lagginess) · [process-and-threading.md › 3.8 Queues and waiters owned by MDSRank](../cpp-map/00-architecture/process-and-threading.md#38-queues-and-waiters-owned-by-mdsrank)
- **Code:** `src/mds/Beacon.cc:132-163 (Beacon::handle_mds_beacon)`, `src/mds/Beacon.cc:261-276 (Beacon::is_laggy)`, `src/mds/MDSRank.cc:1060-1081 (MDSRank::_dispatch)`, `src/mds/MDSRank.cc:714-717 (MDSRankDispatcher::tick)`, `src/mds/Server.cc:1211-1227 (Server::find_idle_sessions)`
- **Notes:** Acks are dispatched on the same `ms_dispatch` thread that blocks on `mds_lock`, so a long `mds_lock` hold can make the daemon consider itself laggy although the monitor received its beacons. The progress thread is not woken when lagginess clears, so deferred messages wait for the next tick or message (process-and-threading section 8).

<a id="f-clu-024"></a>

#### F-CLU-024 · Heartbeat-gated beacons (internal watchdog)

*A daemon that is alive but wedged stops sending beacons so the monitor fails it over.*

`Beacon::_send` skips the beacon (retrying every 0.5 s) while `HeartbeatMap::is_healthy()` is false. The only heartbeat worker is the "MDSRank" handle, re-armed by `heartbeat_reset()` to now + `mds_heartbeat_grace` (15 s) with **no suicide grace** ("the mon takes care of killing us"); it is reset at tick start, after each dispatched message, on every `MDSContext::complete` and every `mds_heartbeat_reset_grace` (1000) iterations of long loops. A rankless standby has no worker and is always healthy. The `lockup <ms>` admin command and `heartbeat_inject_failure` test this path.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config`, `metrics` · relied on by mon
- **Gates:** mds_heartbeat_grace (15 s); mds_heartbeat_reset_grace (1000 iterations)
- **Map:** [beacon-and-monitor.md › 3.3 The MDS-side beacon engine](../cpp-map/01-cluster-integration/beacon-and-monitor.md#33-the-mds-side-beacon-engine) · [process-and-threading.md › 3.12 Tick, timers, heartbeat and lagginess](../cpp-map/00-architecture/process-and-threading.md#312-tick-timers-heartbeat-and-lagginess)
- **Code:** `src/mds/Beacon.cc:203-210 (Beacon::_send)`, `src/mds/MDSRank.cc:1359-1375 (MDSRank::heartbeat_reset)`, `src/mds/MDSRank.cc:505 (MDSRank::MDSRank)`, `src/common/HeartbeatMap.cc:119-137 (HeartbeatMap::is_healthy)`
- **Notes:** `mds_heartbeat_reset_grace` is documented in seconds but used as an iteration count. Large-cache rejoin handlers without heartbeat resets (`recalc_auth_bits`, `trim_non_auth`, `rejoin_send_acks`, ...) can exceed the grace and cause a failover that restarts recovery (resolve-and-rejoin section 3.20).

<a id="f-clu-025"></a>

#### F-CLU-025 · Boot registration as a standby with daemon metadata

*An `up:boot` beacon from an unknown gid registers the daemon in the shared standby pool and stores its system metadata.*

`prepare_beacon` inserts `mds_info_t{gid, name, addrs = message source addrs, mds_features, state = up:standby, state_seq = seq, compat, join_fscid}` with `FSMap::insert`, initialises `last_beacon[gid]` and stores `sys_info` as daemon metadata under `mds_metadata/last_metadata` (keys from `collect_sys_info`: ceph_version, ceph_version_short, ceph_release, os, kernel_*, hostname, arch, mem_*, cpu, distro*, container keys, plus `addr`). The daemon keeps `want_state = up:boot` until a map lists it as `up:standby` ("Monitors have assigned me to become a standby"); a daemon that keeps sending `up:boot` after its standby epoch is ignored without liveness and is eventually removed as laggy. The MgrClient is started and `mgrmap` subscribed the first time the daemon appears in a map.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by mon, mgr, tools, admin
- **Map:** [beacon-and-monitor.md › 3.6 Becoming a standby](../cpp-map/01-cluster-integration/beacon-and-monitor.md#36-becoming-a-standby) · [beacon-and-monitor.md › 3.4 MMDSBeacon wire format](../cpp-map/01-cluster-integration/beacon-and-monitor.md#34-mmdsbeacon-wire-format)
- **Code:** `src/mon/MDSMonitor.cc:690-736 (MDSMonitor::prepare_beacon)`, `src/mds/FSMap.cc:1160-1179 (FSMap::insert)`, `src/mds/MDSDaemon.cc:890-920 (MDSDaemon::handle_mds_map)`, `src/common/util.cc:233-384 (collect_sys_info)`
- **Notes:** mgr re-reads `mds metadata` when the metadata `addr` differs from `stringify(info.addrs)` (`src/mgr/Mgr.cc:753-822`); `mds metadata`, `ceph versions` and `mds count-metadata` read the stored `sys_info`.

<a id="f-clu-026"></a>

#### F-CLU-026 · Unique daemon names

*A booting daemon replaces (fails) any existing daemon with the same name.*

With `mds_enforce_unique_name` (default true) a boot beacon makes the monitor `fail_mds_gid` every existing daemon with the same name ("<name> restarted"), blocklisting it if it held a rank; this needs a writeable OSDMap (retried otherwise). `ceph-mds` refuses an id that is empty or starts with a digit, because `gid_from_arg` reads a numeric argument as a rank or gid, never as a name.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config` · relied on by mon, admin, cephadm, rook
- **Gates:** mds_enforce_unique_name (default true)
- **Map:** [beacon-and-monitor.md › 3.5 Mon-side processing pipeline](../cpp-map/01-cluster-integration/beacon-and-monitor.md#35-mon-side-processing-pipeline) · [beacon-and-monitor.md › 3.2 Daemon start until the first beacon](../cpp-map/01-cluster-integration/beacon-and-monitor.md#32-daemon-start-until-the-first-beacon)
- **Code:** `src/mon/MDSMonitor.cc:692-708 (MDSMonitor::prepare_beacon)`, `src/ceph_mds.cc:146-150 (main)`, `src/mon/MDSMonitor.cc:1407-1451 (MDSMonitor::gid_from_arg)`
- **Notes:** Two hosts sharing an MDS id flap each other; a respawned process (new gid, same name) fails its own previous gid through this rule (beacon-and-monitor sections 3.11 and 8).

<a id="f-clu-027"></a>

#### F-CLU-027 · DNE beacon on clean shutdown

*On orderly exit the MDS tells the monitor it is going away, so its rank fails over immediately instead of after the beacon grace.*

On SIGINT/SIGTERM or asok `exit`, `MDSDaemon::suicide` sets `want_state = down:dne`, releases `mds_lock` and, if the gid is still in its map, sends a beacon and waits up to 0.95 s for its ack (`send_and_wait(1)`). The monitor acks immediately (so the wait returns) and then evicts the daemon with `fail_mds_gid`, which blocklists a rank holder for `mon_mds_blocklist_interval`; the replacement is promoted at the next monitor tick.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, cephadm, rook, admin
- **Map:** [beacon-and-monitor.md › 3.11 Voluntary exits and special states](../cpp-map/01-cluster-integration/beacon-and-monitor.md#311-voluntary-exits-and-special-states) · [rank-lifecycle-and-recovery.md › 3.20 Failure exits: damaged, respawn, suicide](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#320-failure-exits-damaged-respawn-suicide)
- **Code:** `src/mds/MDSDaemon.cc:972-1019 (MDSDaemon::suicide)`, `src/mon/MDSMonitor.cc:766-777 (MDSMonitor::prepare_beacon)`, `src/mds/Beacon.cc:173-192 (Beacon::send_and_wait)`
- **Notes:** A clean shutdown of an active still blocklists its addresses for a day; harmless because every new process has a new nonce, but the blocklist grows with restarts (beacon-and-monitor section 8).

<a id="f-clu-028"></a>

#### F-CLU-028 · Health metrics carried in beacons

*Beacon side of the health metric pipeline.*

See F-ADM-073 for the metric encoding and how the monitor turns metrics into checks. Beacon-side facts: Beacon::notify_health recomputes the list at the end of every non-laggy rank tick (and in damaged()) and every MMDSBeacon carries the latest snapshot; standbys report none. The monitor stores the latest health per gid (pending_daemon_health, persisted as mds_health/<gid>) and uses it to gate commands (F-ADM-083).

- **Class:** POSIX `operational` · exposure `cluster-wire`, `metrics` · relied on by mon, mgr, admin
- **Map:** [beacon-and-monitor.md › 3.12 Health metrics → health checks](../cpp-map/01-cluster-integration/beacon-and-monitor.md#312-health-metrics--health-checks) · [process-and-threading.md › 5.6 Health metrics computed at tick (Beacon::notify_healt...](../cpp-map/00-architecture/process-and-threading.md#56-health-metrics-computed-at-tick-beaconnotify_health-srcmdsbeaconcc303)
- **Code:** `src/mds/Beacon.cc:303-573 (Beacon::notify_health)`, `src/messages/MMDSBeacon.h:38-243 (MDSHealthMetric)`, `src/mon/MDSMonitor.cc:276-343 (MDSMonitor::encode_pending)`, `src/mon/MDSMonitor.cc:1596-1601 (MDSMonitor::has_any_health_warning)`
- **Notes:** Health equality includes the message text, so a changing text sends every beacon through `prepare_beacon` and bumps the MDSMap epoch (map churn to all clients). A new leader does not reload `pending_daemon_health`, so the command health gates see no metrics until each daemon beacons again (beacon-and-monitor section 8).

<a id="f-clu-029"></a>

#### F-CLU-029 · Monitor-side laggy detection and its protections

*The leader marks a daemon laggy when no beacon was noted for `mds_beacon_grace`, with guards against monitor-side outages.*

Every tick (`mon_tick_interval` 5 s) the leader checks `last_beacon`: a daemon silent for `mds_beacon_grace` (15 s, the monitor's own config) gets `laggy_since = now` (a proposal, so clients see the rank laggy), and if the OSDMap is writeable and some MDS beaconed within `max(mds_beacon_interval, mds_beacon_grace/2)` it is queued for removal or replacement. Protections: while a monitor is down or the quorum is younger than `mds_beacon_mon_down_grace` (60 s), nothing is done until the silence exceeds that grace; if the tick itself was delayed by more than `grace - interval` (11 s) all beacon stamps are reset to now; a new leader starts with every daemon stamped now. Frozen daemons are never dropped. Laggy is cleared in `prepare_beacon` by the next beacon carrying the current epoch.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config` · relied on by mon, kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_beacon_grace (15 s, monitor value); mds_beacon_mon_down_grace (60 s); mon_tick_interval (5 s)
- **Map:** [beacon-and-monitor.md › 3.9 Laggy detection and replacement policy](../cpp-map/01-cluster-integration/beacon-and-monitor.md#39-laggy-detection-and-replacement-policy) · [fsmap-and-mdsmap.md › 3.8 The monitor policy loop that drives the FSMap](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#38-the-monitor-policy-loop-that-drives-the-fsmap)
- **Code:** `src/mon/MDSMonitor.cc:2254-2417 (MDSMonitor::check_health)`, `src/mon/MDSMonitor.cc:2266-2297 (MDSMonitor::check_health)`, `src/mon/MDSMonitor.cc:798-805 (MDSMonitor::prepare_beacon)`
- **Notes:** `mds_beacon_grace` and `mds_beacon_interval` are declared `services: mds` but the monitor uses its own values; inconsistent settings make monitor and MDS disagree on lagginess. A laggy holder without a replacement stays in `up` with `laggy_since` indefinitely; ceph-fuse then refuses new mounts (beacon-and-monitor section 8).

### Standby selection, failover and fencing

Monitor policy that chooses standbys, promotes them into ranks, fences failed holders and grows or shrinks the rank count.

<a id="f-clu-030"></a>

#### F-CLU-030 · Standby selection scoring

*How the monitor picks a standby from the shared pool for a rank of a file system.*

`get_available_standby(fs, avoid_addrs)` scans `standby_daemons` in ascending gid order, skips laggy, frozen and compat-ineligible daemons and scores the rest: join_fscid equal to the fs 6 (3 on the same host), NONE 5 (2), another fs 4 (1, skipped entirely if the fs has REFUSE_STANDBY_FOR_ANOTHER_FS); the highest score wins, ties go to the lowest gid, and the scan stops at a 6. `find_replacement_for(rank)` first returns the rank's standby-replay follower (or nothing at all if that follower is frozen), otherwise calls `get_available_standby` with the current holder's addresses as `avoid_addrs` when the rank is still up.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, cephadm, rook, admin
- **Map:** [fsmap-and-mdsmap.md › 3.6 Standby selection](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#36-standby-selection) · [beacon-and-monitor.md › 3.8 Rank assignment in MDSMonitor::tick](../cpp-map/01-cluster-integration/beacon-and-monitor.md#38-rank-assignment-in-mdsmonitortick)
- **Code:** `src/mds/FSMap.cc:804-873 (FSMap::get_available_standby)`, `src/mds/FSMap.cc:907-941 (FSMap::find_replacement_for)`, `src/mds/FSMap.h:696-708 (FSMap::StandbyScore)`
- **Notes:** A lower-scored eligible standby is used when no better one exists; ranking is not prevention (fsmap-and-mdsmap section 3.9).

<a id="f-clu-031"></a>

#### F-CLU-031 · mds_join_fs file system affinity

*Each daemon names a preferred file system in its beacons; the monitor uses it for standby scoring and affinity swaps.*

The beacon `fs` field carries `mds_join_fs` (read on every send, so runtime changes apply at once); the monitor resolves the **name** to an fscid at boot and on every later beacon that reaches `prepare_beacon`, storing NONE if no fs has that name, and a changed resolution forces a proposal. Removing a file system resets every `join_fscid` that pointed to it. cephadm sets `mds_join_fs = <service_id>` for every MDS service it deploys.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config` · relied on by mon, cephadm, rook, admin
- **Gates:** mds_join_fs (default "")
- **Map:** [fsmap-and-mdsmap.md › 3.6 Standby selection](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#36-standby-selection) · [beacon-and-monitor.md › 3.13 Admin and orchestrator commands that change membership](../cpp-map/01-cluster-integration/beacon-and-monitor.md#313-admin-and-orchestrator-commands-that-change-membership)
- **Code:** `src/mds/Beacon.cc:228 (Beacon::_send)`, `src/mon/MDSMonitor.cc:720-727 (MDSMonitor::prepare_beacon)`, `src/mon/MDSMonitor.cc:812-821 (MDSMonitor::prepare_beacon)`, `src/mds/FSMap.cc:1302 (FSMap::erase_filesystem)`, `src/pybind/mgr/cephadm/services/cephadmservice.py:1505-1528 (MdsService.config)`
- **Notes:** Affinity follows the name: after `fs rename` it silently drops to NONE, and after `fs swap --swap-fscids=no` it follows the name to the other fscid (fsmap-and-mdsmap section 8, inferred from code). See also F-ADM-113 (cephadm deployment).

<a id="f-clu-032"></a>

#### F-CLU-032 · Per-file-system selection flags: refuse_standby_for_another_fs and host anti-affinity

*Two file system settings that restrict or bias which standbys the monitor selects.*

`fs set <fs> refuse_standby_for_another_fs true` (flag 1<<7) excludes standbys whose `join_fscid` names another file system; vanilla (NONE) standbys remain eligible. `standby_enable_host_anti_affinity` (default true, ev >= 20) makes any different-host candidate beat every same-host candidate, including an exact affinity match, when an `avoid_addrs` set is supplied: replacing an up (laggy) holder, choosing a standby-replay follower, or an affinity swap; failed and new ranks get no anti-affinity. Host equality compares only the IP of the standby's first address with every address of the holder.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by mon, admin
- **Gates:** REFUSE_STANDBY_FOR_ANOTHER_FS flag; standby_enable_host_anti_affinity (default true)
- **Map:** [fsmap-and-mdsmap.md › 3.6 Standby selection](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#36-standby-selection) · [fsmap-and-mdsmap.md › 3.9 What prevents a standby from being assigned to a file...](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#39-what-prevents-a-standby-from-being-assigned-to-a-file-system)
- **Code:** `src/mds/FSMap.cc:829-861 (FSMap::get_available_standby)`, `src/msg/msg_types.h:409 (entity_addr_t::is_same_host)`, `src/mon/FSCommands.cc:823-852 (FileSystemCommandHandler::set_val)`

<a id="f-clu-033"></a>

#### F-CLU-033 · Affinity-driven swaps of healthy daemons

*The monitor replaces a healthy active or standby-replay daemon whose affinity is worse than an available standby's.*

Per joinable and resizeable file system with a writeable OSDMap, at most once per tick, for each daemon whose `join_fscid` is not the fs: compute a candidate (standby-replay: `get_available_standby` avoiding the followed host; active: `find_replacement_for`); stop scanning at the first daemon without a candidate; if the candidate has strictly better affinity (exact match, or vanilla when the daemon prefers another fs), a standby-replay is failed (re-assigned later) and an active is failed over with `drop_mds` (blocklist plus promotion).

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, cephadm, admin, kclient, libcephfs
- **Map:** [beacon-and-monitor.md › 3.9 Laggy detection and replacement policy](../cpp-map/01-cluster-integration/beacon-and-monitor.md#39-laggy-detection-and-replacement-policy) · [fsmap-and-mdsmap.md › 3.8 The monitor policy loop that drives the FSMap](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#38-the-monitor-policy-loop-that-drives-the-fsmap)
- **Code:** `src/mon/MDSMonitor.cc:2365-2415 (MDSMonitor::check_health)`
- **Notes:** Any daemon state other than ACTIVE or STANDBY_REPLAY in a resizeable file system aborts the monitor here (`ceph_abort_msg("invalid state in MDSMap")`); valid beacons cannot produce such a state, only `mds set_state` can (fsmap-and-mdsmap section 8, inferred from code).

<a id="f-clu-034"></a>

#### F-CLU-034 · Failed-rank and laggy-holder replacement

*Ranks without a live holder get a standby promoted into `up:replay`; a laggy holder is replaced only when a replacement exists.*

`maybe_promote_standby` (every tick, unless NOT_JOINABLE) gives each rank in `failed` the result of `find_replacement_for` and promotes it to `up:replay` (clog "Standby ... assigned to filesystem ... as rank"). For a laggy daemon, `drop_mds` removes standbys and standby-replay followers without blocklisting; a laggy rank holder is failed (blocklisted) and its replacement promoted in the same tick **only if** a replacement exists and the file system is joinable; otherwise it stays in `up` with `laggy_since` until it beacons again or a standby appears. Typical detection-to-promotion time with defaults is 15-20 s plus the Paxos commit.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, kclient, libcephfs, ceph-fuse, ganesha, admin
- **Gates:** NOT_JOINABLE flag
- **Map:** [beacon-and-monitor.md › 3.9 Laggy detection and replacement policy](../cpp-map/01-cluster-integration/beacon-and-monitor.md#39-laggy-detection-and-replacement-policy) · [fsmap-and-mdsmap.md › 3.8 The monitor policy loop that drives the FSMap](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#38-the-monitor-policy-loop-that-drives-the-fsmap) · [rank-lifecycle-and-recovery.md › 3.23 Recovery timeline and timing knobs](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#323-recovery-timeline-and-timing-knobs)
- **Code:** `src/mon/MDSMonitor.cc:2207-2252 (MDSMonitor::drop_mds)`, `src/mon/MDSMonitor.cc:2419-2475 (MDSMonitor::maybe_promote_standby)`, `src/mds/FSMap.cc:907-941 (FSMap::find_replacement_for)`

<a id="f-clu-035"></a>

#### F-CLU-035 · Promotion into a rank and the initial rank state

*The monitor, never the MDS, decides which rank a daemon holds and whether it starts by creating, starting or replaying it.*

`FSMap::promote(gid, fscid, rank)` moves a standby (or the rank's standby-replay follower, in place) into the file system: state `up:starting` if the rank was in `stopped` (removed from it), `up:creating` if the rank is not `is_in` (never existed or forgotten), else `up:replay` (removed from `failed`); sets `rank` and `inc` = pending FSMap epoch, adds the rank to `in` and `up`, may merge compat, and bumps the MDSMap epoch. Callers: `maybe_resize_cluster` (growth), `maybe_promote_standby` (failed ranks), `drop_mds` (laggy holders, affinity swaps) and `fs new` (ranks 0..max_mds-1 in `up:creating`). The MDS must implement every entry state the monitor can assign and cannot refuse an assignment.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, kclient, libcephfs, cephadm, admin
- **Map:** [beacon-and-monitor.md › 3.8 Rank assignment in MDSMonitor::tick](../cpp-map/01-cluster-integration/beacon-and-monitor.md#38-rank-assignment-in-mdsmonitortick) · [fsmap-and-mdsmap.md › 3.7 FSMap mutators (all daemon/rank state changes)](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#37-fsmap-mutators-all-daemonrank-state-changes) · [rank-lifecycle-and-recovery.md › 3.3 Requesting a state and how long a transition takes](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#33-requesting-a-state-and-how-long-a-transition-takes)
- **Code:** `src/mds/FSMap.cc:1006-1064 (FSMap::promote)`, `src/mon/MDSMonitor.cc:2477-2582 (MDSMonitor::tick)`

<a id="f-clu-036"></a>

#### F-CLU-036 · Fencing failed rank holders with OSD blocklists

*When the monitor removes a rank holder it blocklists every address of that daemon at the OSDs in the same Paxos transaction.*

`fail_mds_gid` (used by laggy replacement, beacon evictions, DNE, unique-name enforcement, affinity swaps, `mds fail`, `fs fail`) blocklists, only if the daemon holds a rank and is not standby-replay, every address of its address vector with `TYPE_ANY` until now + `mon_mds_blocklist_interval` (1 day), forces an immediate proposal, then `FSMap::erase(gid, blocklist_epoch)`. FSMap and OSDMap changes made under `paxos.plug()` commit together. Entries are exact `ip:port/nonce` instances, so a restarted process with a new nonce is unaffected. The fenced daemon sees `-EBLOCKLISTED` on writes. Standbys, standby-replay followers and daemons that report `down:stopped` are not blocklisted; `mds rm` bypasses blocklisting.

- **Class:** POSIX `supporting` · exposure `cluster-wire` · relied on by mon, osd
- **Gates:** mon_mds_blocklist_interval (1 day, min 1 h)
- **Map:** [beacon-and-monitor.md › 3.10 Failover and fencing](../cpp-map/01-cluster-integration/beacon-and-monitor.md#310-failover-and-fencing) · [fsmap-and-mdsmap.md › 3.7 FSMap mutators (all daemon/rank state changes)](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#37-fsmap-mutators-all-daemonrank-state-changes)
- **Code:** `src/mon/MDSMonitor.cc:1377-1401 (MDSMonitor::fail_mds_gid)`, `src/mon/OSDMonitor.cc:4979-4991 (OSDMonitor::blocklist)`, `src/mds/FSMap.cc:1092-1127 (FSMap::erase)`
- **Notes:** See also F-PROTO-106 (fencing identity).

<a id="f-clu-037"></a>

#### F-CLU-037 · Wait for last_failure_osd_epoch before touching rank objects

*A daemon taking over a failed rank waits until its OSDMap includes the blocklist of the previous holder before reading the journal.*

`FSMap::erase` records the caller's blocklist epoch in the MDSMap `last_failure_osd_epoch`. `replay_start` (for `up:replay` and `up:standby-replay`) waits until the Objecter has an OSDMap with epoch >= that value before `boot_start`; a promoted standby-replay follower waits again before its final takeover pass. `up:creating` and `up:starting` do not wait.

- **Class:** POSIX `supporting` · exposure `cluster-wire` · relied on by mon, osd
- **Map:** [fsmap-and-mdsmap.md › 4.1 MDSMap fields](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#41-mdsmap-fields) · [beacon-and-monitor.md › 3.10 Failover and fencing](../cpp-map/01-cluster-integration/beacon-and-monitor.md#310-failover-and-fencing) · [rank-lifecycle-and-recovery.md › 3.9 Replay and standby-replay](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#39-replay-and-standby-replay)
- **Code:** `src/mds/MDSRank.cc:1792-1819 (MDSRank::replay_start)`, `src/mds/MDSRank.cc:1872-1893 (MDSRank::standby_replay_restart)`, `src/mds/FSMap.cc:1121 (FSMap::erase)`
- **Notes:** `last_failure_osd_epoch` can go backwards to 0: `FSMap::erase` overwrites it for every erased daemon of the fs, with 0 when nothing was blocklisted (standby-replay removal, `FSMap::stop`, `mds rm`). Concrete paths by code reading: one `check_health` tick dropping a laggy holder then a laggy follower of another rank with a higher gid, and `fs fail` (daemons failed in gid order). The replacement then does not wait for the blocklist epoch; impact not tested (beacon-and-monitor section 8, rank-lifecycle gotcha 20). See also F-STORE-042 (OSD-side guarantee), F-CLU-036.

<a id="f-clu-038"></a>

#### F-CLU-038 · Rank incarnation

*The FSMap epoch at which the current holder was promoted; distinguishes successive holders of the same rank.*

`promote` sets `mds_info_t::inc` to the pending FSMap epoch; `assign_standby_replay` does not, so a follower carries `inc = 0` until promoted. The C++ MDS passes it to the Objecter as the OSD client incarnation, so OSD request ids `(mds.<rank>, incarnation, tid)` of two holders of the same rank never collide in OSD duplicate detection. libcephfs treats an incarnation change of a rank as a restart (old state counted as NULL); peers in `up:resolve` or later use an incarnation change to detect a restarted rank.

- **Class:** POSIX `supporting` · exposure `cluster-wire`, `client-wire` · relied on by mon, osd, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [fsmap-and-mdsmap.md › 3.2 Epochs](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#32-epochs) · [rank-lifecycle-and-recovery.md › 3.5 Rank map handling (MDSRankDispatcher::handle_mds_map)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#35-rank-map-handling-mdsrankdispatcherhandle_mds_map)
- **Code:** `src/mds/FSMap.cc:1048 (FSMap::promote)`, `src/mds/MDSRank.cc:2354 (MDSRankDispatcher::handle_mds_map)`, `src/client/Client.cc:3390-3396 (Client::handle_mds_map)`, `src/messages/MOSDOp.h:112-122 (MOSDOp)`
- **Notes:** See also F-STORE-091 (RADOS identity).

<a id="f-clu-039"></a>

#### F-CLU-039 · Standby-replay assignment

*With `allow_standby_replay`, the monitor attaches standbys as hot followers of active ranks; a follower is the preferred replacement for its rank.*

When the file system is joinable and resizeable and has ALLOW_STANDBY_REPLAY (1<<5), `maybe_promote_standby` repeatedly assigns an available standby (anti-affinity against the followed holder) to each followable rank (up, holder not in replay..clientreplay, no follower yet) via `assign_standby_replay` (state `up:standby-replay`, rank = followed rank, not in `up`, no compat merge). `fs set <fs> allow_standby_replay false` fails every follower (no blocklist). A follower may report only its own state or `down:damaged`; any other requested state evicts it. The kernel client ignores follower entries (state <= 0).

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by mon, admin, cephadm
- **Gates:** ALLOW_STANDBY_REPLAY flag (fs set allow_standby_replay)
- **Map:** [beacon-and-monitor.md › 3.8 Rank assignment in MDSMonitor::tick](../cpp-map/01-cluster-integration/beacon-and-monitor.md#38-rank-assignment-in-mdsmonitortick) · [fsmap-and-mdsmap.md › 3.7 FSMap mutators (all daemon/rank state changes)](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#37-fsmap-mutators-all-daemonrank-state-changes)
- **Code:** `src/mon/MDSMonitor.cc:2445-2467 (MDSMonitor::maybe_promote_standby)`, `src/mds/FSMap.cc:1066-1090 (FSMap::assign_standby_replay)`, `src/mds/MDSMap.cc:1270 (MDSMap::is_followable)`, `src/mon/FSCommands.cc:728-763 (FileSystemCommandHandler::set_val)`
- **Notes:** A frozen follower blocks replacement of its rank completely. A DAMAGED report from a follower blocklists the healthy active and marks the rank damaged; if the followed rank is not up at that moment, `MDSMap::get_gid` throws inside a `noexcept` function and terminates the leader monitor (beacon-and-monitor section 8, inferred from code).

<a id="f-clu-040"></a>

#### F-CLU-040 · Growing the number of active ranks (max_mds)

*The monitor adds ranks one at a time towards `max_mds`, each new rank starting in `up:creating` or `up:starting`.*

`fs set <fs> max_mds N` (1..256) sets `max_mds` and clears NOT_JOINABLE; it requires `--yes-i-really-mean-it` when any MDS in the cluster reports any health metric, and is EINVAL above 1 when snapshots were ever allowed without `allow_multimds_snaps`. Each tick, if both the committed and pending maps are resizeable and the fs is joinable with `size(in) < max_mds`, the lowest rank not `is_in` gets `find_replacement_for` + `promote` (`up:creating` or `up:starting`); because those states make the map non-resizeable, ranks are added strictly one after another.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by mon, admin, cephadm, mgr, kclient, libcephfs
- **Gates:** max_mds; NOT_JOINABLE flag
- **Map:** [beacon-and-monitor.md › 3.8 Rank assignment in MDSMonitor::tick](../cpp-map/01-cluster-integration/beacon-and-monitor.md#38-rank-assignment-in-mdsmonitortick) · [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states)
- **Code:** `src/mon/MDSMonitor.cc:2144-2183 (MDSMonitor::maybe_resize_cluster)`, `src/mon/FSCommands.cc:434-463 (FileSystemCommandHandler::set_val)`, `src/mds/MDSMap.h:460-465 (MDSMap::is_resizeable)`

<a id="f-clu-041"></a>

#### F-CLU-041 · Shrinking: monitor-driven up:stopping and FSMap::stop

*When `in > max_mds` the monitor asks the highest rank to stop and, once it reports `down:stopped`, removes the rank.*

Each tick, if both maps are resizeable and `size(in) > max_mds`, rank `size(in)-1` is set to `up:stopping` if it is `up:active` (otherwise nothing that tick); `up:stopping` keeps the map non-resizeable, so ranks stop one at a time. On a `down:stopped` beacon the monitor logs "finished stopping rank", `FSMap::stop` removes the rank from `up` and `in`, adds it to `stopped`, erases the daemon and every standby-replay follower of that rank, and replies with a null MDSMap so the daemon respawns as a new standby.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by mon, admin, cephadm, mgr, kclient, libcephfs
- **Gates:** max_mds
- **Map:** [beacon-and-monitor.md › 3.8 Rank assignment in MDSMonitor::tick](../cpp-map/01-cluster-integration/beacon-and-monitor.md#38-rank-assignment-in-mdsmonitortick) · [fsmap-and-mdsmap.md › 3.7 FSMap mutators (all daemon/rank state changes)](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#37-fsmap-mutators-all-daemonrank-state-changes) · [rank-lifecycle-and-recovery.md › 3.19 Stopping (up:stopping → down:stopped)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#319-stopping-upstopping--downstopped)
- **Code:** `src/mon/MDSMonitor.cc:2184-2198 (MDSMonitor::maybe_resize_cluster)`, `src/mds/FSMap.cc:1181-1215 (FSMap::stop)`, `src/mon/MDSMonitor.cc:823-841 (MDSMonitor::prepare_beacon)`, `src/mon/MDSMonitor.cc:971-974 (MDSMonitor::_updated)`

### Monitor fs and mds commands

Monitor commands that create, take down, reset and inspect file systems and MDS daemons.

<a id="f-clu-042"></a>

#### F-CLU-042 · fs new

*Creates a file system from existing pools and immediately assigns standbys to its ranks.*

`fs new <fs> <meta> <data> [--force] [--allow-dangerous-metadata-overlay] [--fscid N] [--recover] [--set k v ...]` (fs rw) checks: pools exist (ENOENT), data pool id 0 refused, same name with same pools succeeds as a no-op (else EINVAL), metadata pool empty unless `--force`, a second fs needs `enable_multiple`, pools used by another fs need the overlay flag, `--fscid` needs `--force` and a free id, and `_check_pool` (no mon-managed snaps, EC metadata refused, EC default data needs `--force`, EC data needs overwrites, no cache tier, no other application). It creates the `Filesystem` (compat = `default_compat`, max_mds 1), applies `--set` values, tags both pools with application `cephfs`, sets metadata pool `recovery_priority 5`, `pg_num_min 16`, `pg_autoscale_bias 4`, and promotes standbys to `up:creating` for ranks 0..max_mds-1 in the same pending map. With `--recover`, rank 0 is put in `in` and `failed` with NOT_JOINABLE and nothing is assigned, so the first holder replays existing metadata.

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire` · relied on by admin, mgr, cephadm, rook, mon, osd
- **Map:** [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states) · [rank-lifecycle-and-recovery.md › 3.7 Creating a rank (up:creating, MDSRank::boot_create)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#37-creating-a-rank-upcreating-mdsrankboot_create)
- **Code:** `src/mon/FSCommands.cc:150-361 (FsNewHandler::handle)`, `src/mds/FSMap.cc:470-494 (FSMap::create_filesystem)`, `src/mon/FSCommands.cc:342-353 (FsNewHandler::handle)`
- **Notes:** `fs new` ignores `joinable false` given via `--set`; only `--set down true` or `--recover` create a file system without immediate assignment (fsmap-and-mdsmap section 8, inferred from code). See also F-STORE-101 (pool requirements).

<a id="f-clu-043"></a>

#### F-CLU-043 · fs rm

*Deletes an offline file system from the FSMap; RADOS pools are untouched.*

`fs rm <fs> --yes-i-really-mean-it` (fs rw) requires that no rank is up (EINVAL) and the confirmation (EPERM); a missing fs succeeds. It clears `legacy_client_fscid` if it named this fs (no new default is chosen), fails remaining standby-replay daemons, erases the `Filesystem` and resets `join_fscid` of daemons pointing at it; subscribers of `mdsmap.<fscid>` receive nothing more.

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire` · relied on by admin, mgr, cephadm, rook
- **Map:** [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states) · [beacon-and-monitor.md › 3.13 Admin and orchestrator commands that change membership](../cpp-map/01-cluster-integration/beacon-and-monitor.md#313-admin-and-orchestrator-commands-that-change-membership)
- **Code:** `src/mon/FSCommands.cc:1160-1237 (RemoveFilesystemHandler::handle)`, `src/mds/FSMap.cc:1302 (FSMap::erase_filesystem)`

<a id="f-clu-044"></a>

#### F-CLU-044 · fs fail and the joinable flag

*Takes a whole file system offline (all daemons removed, no new assignments) and brings it back.*

`fs fail <fs> [--yes-i-really-mean-it]` sets NOT_JOINABLE and `fail_mds_gid`s every daemon of the fs in ascending gid order: holders are blocklisted, ranks go to `failed` (a CREATING rank is forgotten, a STARTING rank returns to `stopped`), followers are removed; it is EPERM without confirmation when a daemon reports TRIM or CACHE_OVERSIZED. `fs set <fs> joinable false|true` (deprecated inverse alias `cluster_down`) sets/clears NOT_JOINABLE: while set there is no failed-rank promotion, no standby-replay assignment, no growth (shrink still proceeds), no laggy-holder replacement and no affinity swap; laggy standbys and followers are still removed. `fs set max_mds` also clears the flag.

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire` · relied on by admin, mgr, cephadm, rook, osd
- **Legacy parts** (deprecated-admin): The deprecated `fs set <fs> cluster_down` alias of `joinable`. `fs set <fs> cluster_down` is handled in the joinable branch with the value inverted, and the monitor appends 'WARNING: cluster_down flag is deprecated and will be removed in a future version. Please use "joinable".' to its output. A grep of src/pybind/mgr, src/cephadm, src/tools and Rook's Go sources finds no use; `fs fail` and `joinable` are current (cephadm's MDS upgrade issues `fs fail`). Evidence: `src/mon/FSCommands.cc:643-674 (FileSystemCommandHandler::set_val)`, `src/mon/MonCommands.h:420 (fs set var cluster_down)`, `src/pybind/mgr/cephadm/upgrade.py:1472-1476 (CephadmUpgrade._prepare_for_mds_upgrade)`.
- **Gates:** NOT_JOINABLE flag
- **Map:** [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states) · [beacon-and-monitor.md › 3.13 Admin and orchestrator commands that change membership](../cpp-map/01-cluster-integration/beacon-and-monitor.md#313-admin-and-orchestrator-commands-that-change-membership) · [fsmap-and-mdsmap.md › 3.9 What prevents a standby from being assigned to a file...](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#39-what-prevents-a-standby-from-being-assigned-to-a-file-system)
- **Code:** `src/mon/FSCommands.cc:84-148 (FailHandler::handle)`, `src/mon/FSCommands.cc:643-674 (FileSystemCommandHandler::set_val)`, `src/mon/MDSMonitor.cc:2422 (MDSMonitor::maybe_promote_standby)`
- **Notes:** Because `fs fail` erases daemons in gid order, a standby-replay follower with a higher gid than a holder resets `last_failure_osd_epoch` to 0 after the holder was blocklisted (fsmap-and-mdsmap section 8).

<a id="f-clu-045"></a>

#### F-CLU-045 · fs reset

*Disaster-recovery command that replaces a file system's MDSMap by a fresh one whose rank 0 must replay.*

`fs reset <fs> --yes-i-really-mean-it` requires an existing fs (ENOENT), no rank up (EINVAL) and the confirmation (EPERM). `reset_filesystem` keeps only fscid, name, pools, `cas_pool`, `inline_data_enabled` and `standby_count_wanted`; compat becomes `default_compat`, `in = {0}`, `failed = {0}`, every other previously used rank goes to `stopped`, and flags (including NOT_JOINABLE), `max_mds` (1), timeouts, sizes, required features, balancer and mirror info return to defaults. On the next tick rank 0 is replaced and its holder enters `up:replay`.

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire` · relied on by admin, tools
- **Map:** [fsmap-and-mdsmap.md › 3.7 FSMap mutators (all daemon/rank state changes)](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#37-fsmap-mutators-all-daemonrank-state-changes) · [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states)
- **Code:** `src/mon/FSCommands.cc:1239-1282 (ResetFilesystemHandler::handle)`, `src/mds/FSMap.cc:540-571 (FSMap::reset_filesystem)`
- **Notes:** The reset MDSMap keeps epoch 0 until modified again; standby-replay daemons are not failed, which leaves an `mds_roles` entry without `mds_info` that `sanity()` rejects (fsmap-and-mdsmap section 8, inferred from code).

<a id="f-clu-046"></a>

#### F-CLU-046 · fs set down

*Stops every rank of a file system cleanly and later restores the previous rank count.*

`fs set <fs> down true` saves `old_max_mds = max_mds` and sets `max_mds = 0` without touching flags, so the monitor stops ranks one by one (`up:stopping` to `down:stopped`) until `in` is empty; `down false` restores `max_mds = old_max_mds` (or 1) and ranks regrow as `up:starting`; `down false` while `max_mds > 0` is a no-op ("already online").

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire` · relied on by admin, cephadm, rook
- **Map:** [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states) · [rank-lifecycle-and-recovery.md › 3.8 Starting a stopped rank (up:starting)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#38-starting-a-stopped-rank-upstarting)
- **Code:** `src/mon/FSCommands.cc:610-642 (FileSystemCommandHandler::set_val)`
- **Notes:** The last rank to stop evicts and (by default) blocklists every mounted client, so clients must remount after `down false` (rank-lifecycle gotcha 25).

<a id="f-clu-047"></a>

#### F-CLU-047 · fs rename and fs swap

*Renames an offline file system, or swaps the names (and optionally fscids) of two offline file systems.*

`fs rename <fs> <new> --yes-i-really-mean-it` requires a free target name (EINVAL), mirroring disabled (EPERM), the confirmation and the fs offline with `refuse_client_session` set (EPERM); it rewrites the pool application tags and `fs_name`. `fs swap <fs1> <id1> <fs2> <id2> --swap-fscids=yes|no --yes-i-really-mean-it` requires both to exist with matching ids, mirroring disabled, both offline and both `refuse_client_session`; `swap_fscids` exchanges the two `Filesystem` objects without updating `mds_roles`, `join_fscid` or `legacy_client_fscid`.

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire` · relied on by admin, kclient, mgr
- **Map:** [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states) · [fsmap-and-mdsmap.md › 8. Edge cases, failure handling and gotchas](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mon/FSCommands.cc:1284 (RenameFilesystemHandler::handle)`, `src/mon/FSCommands.cc:1389 (SwapFilesystemHandler::handle)`, `src/mds/FSMap.cc:1323 (FSMap::swap_fscids)`
- **Notes:** Kernel clients mounted with the old `mds_namespace` fail the next map decode and are unmounted, which is why the fs must be offline with `refuse_client_session`.

<a id="f-clu-048"></a>

#### F-CLU-048 · mds fail

*Fails one daemon by role, gid or name; a rank holder is blocklisted and replaced at the next tick.*

`mds fail <role|gid|name> [--yes-i-really-mean-it]` (mds rw) resolves the argument as a role if that rank is up, else as an integer gid, else as a daemon name, and calls `fail_mds_gid`: a rank holder (not standby-replay) is blocklisted, its rank goes to `failed` and is replaced next tick (its follower first); a standby or follower is just removed and respawns as a new standby. It returns 0 with a message when the target is unknown, and EPERM without confirmation when the target reports TRIM or CACHE_OVERSIZED.

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire` · relied on by admin, cephadm, rook, mgr, osd
- **Map:** [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states) · [beacon-and-monitor.md › 3.13 Admin and orchestrator commands that change membership](../cpp-map/01-cluster-integration/beacon-and-monitor.md#313-admin-and-orchestrator-commands-that-change-membership)
- **Code:** `src/mon/MDSMonitor.cc:1637-1677 (MDSMonitor::filesystem_command)`, `src/mon/MDSMonitor.cc:1453-1475 (MDSMonitor::fail_mds)`, `src/mon/MDSMonitor.cc:1407-1451 (MDSMonitor::gid_from_arg)`

<a id="f-clu-049"></a>

#### F-CLU-049 · Damaged ranks and mds repaired

*A rank reported damaged is quarantined (never reassigned) until an operator runs `mds repaired`.*

On a `down:damaged` beacon from the holder or its standby-replay follower the monitor (with a writeable OSDMap) blocklists the **holder's** addresses, fails the follower, runs `FSMap::damaged` (erase, then move the rank from `failed` to `damaged`; it stays in `in`) and proposes immediately. A damaged rank blocks `is_resolving`/`is_rejoining`, keeps the map degraded and raises `MDS_DAMAGE` (HEALTH_ERR). `mds repaired <role>` (`FSMap::undamaged`) moves it back to `failed`, so it is replaced at the next tick.

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire`, `metrics` · relied on by admin, mon, tools, kclient, libcephfs
- **Map:** [fsmap-and-mdsmap.md › 3.7 FSMap mutators (all daemon/rank state changes)](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#37-fsmap-mutators-all-daemonrank-state-changes) · [beacon-and-monitor.md › 3.11 Voluntary exits and special states](../cpp-map/01-cluster-integration/beacon-and-monitor.md#311-voluntary-exits-and-special-states) · [rank-lifecycle-and-recovery.md › 3.20 Failure exits: damaged, respawn, suicide](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#320-failure-exits-damaged-respawn-suicide)
- **Code:** `src/mon/MDSMonitor.cc:842-875 (MDSMonitor::prepare_beacon)`, `src/mds/FSMap.cc:1129-1158 (FSMap::damaged)`, `src/mon/MDSMonitor.cc:1769-1791 (MDSMonitor::filesystem_command)`
- **Notes:** A DAMAGED report from a CREATING holder leaves the rank in `damaged` but not in `in` (inferred from code). A follower reporting DAMAGED while its rank is not up terminates the leader monitor via `up.at()` in a `noexcept` function (beacon-and-monitor section 8, not tested).

<a id="f-clu-050"></a>

#### F-CLU-050 · Hidden and debug rank commands: mds rm, rmfailed, freeze, set_state

*Low-level monitor commands that edit daemon and rank records without the normal checks.*

`mds rm <gid>` erases a daemon in a state <= 0 (EBUSY otherwise; unknown gid succeeds) without blocklisting, which includes CREATING and STARTING holders. `mds rmfailed <role> --yes-i-really-mean-it` removes a rank from `failed` only: it stays in `in`, is never replaced and is handed out as CREATING only by growth. `mds freeze <role|gid> <bool>` sets `mds_info_t::flags` FROZEN: frozen daemons are never selected or dropped, and a frozen follower blocks replacement of its rank. `mds set_state <gid> <n>` writes the state without validation.

- **Class:** POSIX `operational` · exposure `admin`, `cluster-wire` · relied on by admin, tools
- **Map:** [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states) · [beacon-and-monitor.md › 3.13 Admin and orchestrator commands that change membership](../cpp-map/01-cluster-integration/beacon-and-monitor.md#313-admin-and-orchestrator-commands-that-change-membership)
- **Code:** `src/mon/MDSMonitor.cc:1678-1704 (MDSMonitor::filesystem_command)`, `src/mon/MDSMonitor.cc:1705-1738 (MDSMonitor::filesystem_command)`, `src/mon/MDSMonitor.cc:1792-1825 (MDSMonitor::filesystem_command)`, `src/mon/MDSMonitor.cc:1616-1636 (MDSMonitor::filesystem_command)`
- **Notes:** `mds set_state` on a standby gid trips `ceph_assert(info.state == STATE_STANDBY)` in `FSMap::modify_daemon` and aborts the leader; a shrink whose target is a `rmfailed` rank terminates the monitor via `up.at()` (code reading, not tested). `mds rm` leaves the gid's health entry behind, still counted by `has_any_health_warning`.

<a id="f-clu-051"></a>

#### F-CLU-051 · Read-only fs and mds commands and the fs dump layout

*Inspection commands whose JSON output orchestrators, mgr modules and tools parse.*

Commands: `fs dump [epoch]` (any stored epoch, unfiltered when historical), `fs get <fs>`, `fs ls`, `fs lsflags <fs>`, `fs feature ls`, `fs compat show <fs>`, `mds compat show`, `mds stat`, `mds metadata [who]`, `mds versions`, `mds count-metadata <field>`, `mds last-seen <name>`; `fs ls/get/lsflags` need `fs r`, the rest `mds r`. `fs dump` JSON: top-level `epoch, btime, default_fscid, compat, feature_flags, standbys[], filesystems[]{mdsmap, id, mirror_info}`; `mdsmap` keys include `flags_state`, `in`, `up{mds_<rank>: gid}`, `failed`, `damaged`, `stopped`, `info{gid_<gid>: {gid, name, rank, incarnation, state, state_seq, addr, addrs, join_fscid, laggy_since, export_targets, features, flags, compat}}` with state strings such as `up:active` that cephadm waits on. `mds last-seen` uses an in-memory history pruned after `mon_fsmap_prune_threshold` (300 s).

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin, mgr, cephadm, rook, tools
- **Legacy parts** (deprecated-admin): The deprecated `mds compat show` command. `mds compat show` is registered with FLAG(DEPRECATED) and no mgr module, cephadm, Rook or tool issues it (grep). The other listed commands and the fs dump layout are current: cephadm waits on the up:active state string and mgr/prometheus reads the per-daemon addr field. Evidence: `src/mon/MonCommands.h:366-367 (mds compat show, FLAG(DEPRECATED))`, `src/mon/MDSMonitor.cc:1250-1259 (MDSMonitor::preprocess_command)`, `src/pybind/mgr/prometheus/module.py:1348 (standby['addr'])`, `src/pybind/mgr/cephadm/upgrade.py:1527 (CephadmUpgrade._prepare_for_mds_upgrade)`.
- **Gates:** mon_fsmap_prune_threshold (300 s); mon_max_mdsmap_epochs (500)
- **Map:** [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states) · [fsmap-and-mdsmap.md › 7. Externally visible contract](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#7-externally-visible-contract)
- **Code:** `src/mon/MDSMonitor.cc:1002-1375 (MDSMonitor::preprocess_command)`, `src/mds/FSMap.cc:148 (FSMap::dump)`, `src/mds/MDSMap.cc:133-194 (MDSMap::mds_info_t::dump)`

<a id="f-clu-052"></a>

#### F-CLU-052 · Orchestration helpers: mds ok-to-stop, last-seen, metadata, versions

*Commands orchestrators use to decide whether MDS daemons can be stopped or upgraded.*

`mds ok-to-stop <ids...>` returns EBUSY if any file system is degraded, or if the standbys left after stopping the listed ones are fewer than the listed rank holders that have no standby-replay follower outside the list; unknown ids are skipped. `mds last-seen <name>` gives seconds since the daemon was last in an FSMap (0 if in the latest). `mds metadata`, `mds versions` and `mds count-metadata` read the boot `sys_info`.

- **Class:** POSIX `operational` · exposure `admin` · relied on by cephadm, rook, mgr, admin, tools
- **Map:** [beacon-and-monitor.md › 3.13 Admin and orchestrator commands that change membership](../cpp-map/01-cluster-integration/beacon-and-monitor.md#313-admin-and-orchestrator-commands-that-change-membership) · [fsmap-and-mdsmap.md › 3.10 Commands and their effect on states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#310-commands-and-their-effect-on-states)
- **Code:** `src/mon/MDSMonitor.cc:1090-1140 (MDSMonitor::preprocess_command)`, `src/mon/MDSMonitor.cc:1044-1089 (MDSMonitor::preprocess_command)`, `src/mon/MDSMonitor.cc:1176-1230 (MDSMonitor::preprocess_command)`
- **Notes:** See also F-ADM-116 (Rook ok-to-stop gate).

### Rank state machine and map handling

The rank state values and legal transitions, how the MDS requests states and reacts to each new map, and map-derived health.

<a id="f-clu-053"></a>

#### F-CLU-053 · Daemon and rank states: numeric values and names

*The wire values of every daemon and rank state and the strings tools display for them.*

Values (`ceph_fs.h`): dne 0, stopped -1, boot -4, standby -5, creating -6, starting -7, standby-replay -8, replayonce -9 (unused), null -10, replay 8, resolve 9, reconnect 10, rejoin 11, clientreplay 12, active 13, stopping 14, damaged 15; names `up:active`, `down:stopped`, ... from `ceph_mds_state_name`. Values > 0 mean the rank is in. BOOT, DNE, STOPPED and DAMAGED are beacon values never stored as a daemon state; failed/stopped/damaged/dne are rank conditions expressed by set membership. CREATING/STARTING/REPLAY/STANDBY_REPLAY/STOPPING are imposed by the monitor; RESOLVE through ACTIVE, STOPPED, DAMAGED and DNE are requested by the MDS. `mds_info_t::is_degraded()` is REPLAY <= state <= CLIENTREPLAY.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `client-wire`, `admin` · relied on by mon, kclient, libcephfs, ceph-fuse, ganesha, mgr, cephadm, rook, tools, admin
- **Legacy parts** (dead-code): State value -9 (replayonce, 'up:oneshot-replay'). CEPH_MDS_STATE_REPLAYONCE (-9) is commented 'Legacy, unused', is not a member of MDSMap::DaemonState, and no MDS, monitor or kernel code assigns it; only the name tables map it to 'up:oneshot-replay'. Evidence: `src/include/ceph_fs.h:314 (CEPH_MDS_STATE_REPLAYONCE)`, `src/mds/MDSMap.h:75-113 (MDSMap::DaemonState)`, `src/common/ceph_strings.cc:253 (ceph_mds_state_name)`, `linux/fs/ceph/strings.c:19 (ceph_mds_state_name)`.
- **Map:** [fsmap-and-mdsmap.md › 3.3 MDS daemon and rank states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#33-mds-daemon-and-rank-states) · [rank-lifecycle-and-recovery.md › 3.1 States, values and who sets them](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#31-states-values-and-who-sets-them)
- **Code:** `src/include/ceph_fs.h:306-325 (CEPH_MDS_STATE_DNE)`, `src/common/ceph_strings.cc:242-268 (ceph_mds_state_name)`, `src/mds/MDSMap.h:75-113 (MDSMap::DaemonState)`
- **Notes:** `doc/cephfs/mds-states.rst` spells `up:standby_replay`; the code string is `up:standby-replay` (rank-lifecycle gotcha 30).

<a id="f-clu-054"></a>

#### F-CLU-054 · State transition validation on both sides

*The legal state graph, enforced by the monitor on requested states and by the C++ MDS on assigned states.*

`MDSMap::state_transition_valid(prev, next)`: same state and DAMAGED always valid; BOOT to STANDBY; STANDBY to STANDBY_REPLAY, REPLAY, CREATING, STARTING; CREATING/STARTING to ACTIVE; STANDBY_REPLAY to REPLAY; REPLAY to RESOLVE or RECONNECT; RESOLVE, RECONNECT, CLIENTREPLAY to prev+1; REJOIN to CLIENTREPLAY, ACTIVE or STOPPED; ACTIVE to STOPPING; STOPPING to STOPPED; anything else invalid. The monitor evicts (with blocklisting for rank holders) a holder that reports an invalid transition, a standby that requests any other state, and a follower that requests anything but its state or DAMAGED. The C++ MDS respawns when a received map shows an invalid transition for itself. Removal is not a transition. The monitor does not check cluster-wide preconditions and has no per-state timeout.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon
- **Map:** [fsmap-and-mdsmap.md › 3.3 MDS daemon and rank states](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#33-mds-daemon-and-rank-states) · [beacon-and-monitor.md › 3.7 The daemon/rank state machine](../cpp-map/01-cluster-integration/beacon-and-monitor.md#37-the-daemonrank-state-machine) · [rank-lifecycle-and-recovery.md › 3.2 The state machine](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#32-the-state-machine)
- **Code:** `src/mds/MDSMap.cc:1104-1144 (MDSMap::state_transition_valid)`, `src/mon/MDSMonitor.cc:780-797 (MDSMonitor::prepare_beacon)`, `src/mds/MDSRank.cc:2328-2332 (MDSRankDispatcher::handle_mds_map)`

<a id="f-clu-055"></a>

#### F-CLU-055 · Requesting the next state through beacons

*The MDS moves a rank forward only by asking the monitor and acting when the new map shows the state.*

`request_state(s)` calls `Beacon::set_want_state(mdsmap, s)`, which updates the wanted state and the last-seen epoch under one lock so a beacon never carries a new epoch with a stale state, and sends a beacon at once; periodic beacons repeat it. The monitor applies it only when the beacon epoch equals the daemon's current effective epoch, records `state` and `state_seq`, logs "is now active" on the first `up:active`, and pushes the new map. The MDS acts in `handle_mds_map`, not at request time. Each MDS-driven step therefore costs a beacon round trip and a Paxos commit; a single-rank failover from replay to active needs 3 or 4 committed epochs. To close races the MDS compares against `get_want_state()`: replayed requests, dirty cap flushes and reconnect messages that arrive before it sees its own `up:reconnect` map are admitted or parked.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon
- **Map:** [rank-lifecycle-and-recovery.md › 3.3 Requesting a state and how long a transition takes](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#33-requesting-a-state-and-how-long-a-transition-takes) · [beacon-and-monitor.md › 3.7 The daemon/rank state machine](../cpp-map/01-cluster-integration/beacon-and-monitor.md#37-the-daemonrank-state-machine)
- **Code:** `src/mds/MDSRank.cc:1603-1608 (MDSRank::request_state)`, `src/mds/Beacon.cc:278-295 (Beacon::set_want_state)`, `src/mon/MDSMonitor.cc:488-501 (MDSMonitor::preprocess_beacon)`, `src/mon/MDSMonitor.cc:876-891 (MDSMonitor::prepare_beacon)`
- **Notes:** A beacon carrying the latest epoch with the pre-assignment state is an invalid transition and evicts the daemon with a blocklist, which is why epoch and state are updated atomically (beacon-and-monitor section 8).

<a id="f-clu-056"></a>

#### F-CLU-056 · Rank sets and derived predicates

*The sets `in/up/failed/stopped/damaged` and the predicates computed from them that gate resizing, standby-replay, recovery barriers and client availability.*

Effective rule: `up` keys + `failed` + `damaged` = `in` (the header comment `up + failed = in` predates `damaged`); `is_in(r)` is up or failed (not damaged). Predicates: `is_degraded()` = failed or damaged non-empty or any daemon in REPLAY..CLIENTREPLAY; `is_resizeable()` = not degraded and no CREATING/STARTING/STOPPING; `is_followable(r)`; `is_resolving()` = some RESOLVE, no REPLAY, no failed/damaged; `is_rejoining()` = some REJOIN, none in REPLAY/RESOLVE/RECONNECT, no failed/damaged; `get_recovery_mds_set` = failed + damaged + ranks in REPLAY..STOPPING; `get_mds_set_lower_bound`; `get_down_mds_set` = failed + damaged; `is_cluster_available()`. Implicit rank state without holder: failed, stopped (`down:stopped`), damaged (`down:damaged`), else dne.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, mds-ranks, libcephfs, kclient
- **Map:** [fsmap-and-mdsmap.md › 3.4 Rank sets and derived predicates](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#34-rank-sets-and-derived-predicates) · [rank-lifecycle-and-recovery.md › 3.10 Multiple ranks: recovery set, barriers and survivors](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#310-multiple-ranks-recovery-set-barriers-and-survivors)
- **Code:** `src/mds/MDSMap.h:617-642 (MDSMap)`, `src/mds/MDSMap.h:460-544 (MDSMap::is_resizeable)`, `src/mds/MDSMap.cc:1236-1326 (MDSMap::get_recovery_mds_set)`, `src/mds/MDSMap.cc:1287 (MDSMap::is_degraded)`
- **Notes:** Edge cases break the set rule: a DAMAGED report from a CREATING holder leaves the rank damaged but not in `in`; `mds rmfailed` leaves a rank in `in` only (fsmap-and-mdsmap section 3.4).

<a id="f-clu-057"></a>

#### F-CLU-057 · Daemon-level map handling

*How the daemon reacts to each new MDSMap before any rank logic: compat check, removal, standby, rank assignment.*

`MDSDaemon::handle_mds_map` (under `mds_lock`; maps accepted from monitors and peer MDSs): drop epochs <= current; decode; `sub_got`; **suicide if `get_compat_set_all()` cannot write the map compat**; compute the rank (for a follower, the followed rank); present in the old map and absent from the new one: respawn ("Map removed me"); first appearance: start the MgrClient and subscribe `mgrmap`; mark down connections to gids that left; rankless: NULL to STANDBY sets `want_state = up:standby`, NULL stays booting, any other rankless state aborts; with a rank: respawn if it differs from the existing rank object, else create `MDSRankDispatcher` once and call its map handler; finally record the epoch for later beacons. A rank object is never replaced in-process: a rank change, removal or invalid transition re-execs the process.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, internal
- **Map:** [rank-lifecycle-and-recovery.md › 3.4 Daemon map handling (MDSDaemon::handle_mds_map)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#34-daemon-map-handling-mdsdaemonhandle_mds_map) · [process-and-threading.md › 3.4 Daemon-level map handling (MDSDaemon::handle_mds_map)](../cpp-map/00-architecture/process-and-threading.md#34-daemon-level-map-handling-mdsdaemonhandle_mds_map) · [beacon-and-monitor.md › 3.16 How the MDS reacts to MDSMap changes](../cpp-map/01-cluster-integration/beacon-and-monitor.md#316-how-the-mds-reacts-to-mdsmap-changes)
- **Code:** `src/mds/MDSDaemon.cc:833-957 (MDSDaemon::handle_mds_map)`, `src/mds/MDSDaemon.cc:857-866 (MDSDaemon::handle_mds_map)`, `src/mds/Beacon.cc:244-256 (Beacon::notify_mdsmap)`

<a id="f-clu-058"></a>

#### F-CLU-058 · Rank-level map handling and peer change detection

*The single per-rank driver that starts each phase and detects peer ranks failing, restarting, recovering or stopping.*

`MDSRankDispatcher::handle_mds_map`: treats an old NULL state as STANDBY, asserts no map was missed, validates the transition (respawn if invalid), renames the messenger and pushes the incarnation on a state change; evicts sessions lacking `required_client_features` (state >= RECONNECT); caches `is_degraded`. From `up:resolve` on it detects peers newly down (`handle_mds_failure`), restarted (incarnation change while REPLAY/RESOLVE) or stopped-and-restarted. It then calls exactly one of `replay_start`, `resolve_start`, `reconnect_start`, `rejoin_start`, `clientreplay_start`, `active_start` (after `recovery_done`), `boot_create`, `boot_start`, `stopping_start`; runs snap-table recovery, the resolve and rejoin triggers, kicks waiters for cluster recovery, bootstrapping peers, recovered peers and `waiting_for_mdsmap`; while active raises the OSD epoch barrier and sets the journal fadvise hint (none if a follower exists, else DONTNEED).

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, osd, mds-ranks, internal
- **Map:** [rank-lifecycle-and-recovery.md › 3.5 Rank map handling (MDSRankDispatcher::handle_mds_map)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#35-rank-map-handling-mdsrankdispatcherhandle_mds_map) · [beacon-and-monitor.md › 3.16 How the MDS reacts to MDSMap changes](../cpp-map/01-cluster-integration/beacon-and-monitor.md#316-how-the-mds-reacts-to-mdsmap-changes)
- **Code:** `src/mds/MDSRank.cc:2294-2626 (MDSRankDispatcher::handle_mds_map)`, `src/mds/MDSRank.cc:2366-2420 (MDSRankDispatcher::handle_mds_map)`, `src/mds/MDSRank.cc:2423-2461 (MDSRankDispatcher::handle_mds_map)`

<a id="f-clu-060"></a>

#### F-CLU-060 · MDSMap epoch waits for client requests

*Requests that name a pool or create a snapshot wait until the MDS has at least the client's MDSMap epoch.*

Every client request carries the client's MDSMap epoch (`mdsmap_epoch` in the request head). Before validating a pool in a layout (open with O_CREAT and a layout, setlayout, setdirlayout) or the ALLOW_SNAPS flag for mksnap, the C++ MDS parks the request with `wait_for_mdsmap(epoch)` until its own map is at least that new; `handle_mds_map` wakes waiters for epochs <= the new one. The migrator uses the same queue. Replies carry the MDS's epoch.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Map:** [fsmap-and-mdsmap.md › 3.2 Epochs](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#32-epochs) · [process-and-threading.md › 3.8 Queues and waiters owned by MDSRank](../cpp-map/00-architecture/process-and-threading.md#38-queues-and-waiters-owned-by-mdsrank)
- **Code:** `src/mds/MDSRank.h:362 (MDSRank::wait_for_mdsmap)`, `src/mds/Server.cc:4959 (Server::handle_client_openc)`, `src/mds/Server.cc:11592 (Server::handle_client_mksnap)`, `src/mds/MDSRank.cc:2587-2595 (MDSRankDispatcher::handle_mds_map)`, `linux/fs/ceph/mds_client.c:3393 (create_request_message)`

<a id="f-clu-062"></a>

#### F-CLU-062 · File-system health checks derived from the map

*Map-side inputs of the file-system health checks the monitor derives from the FSMap.*

See F-ADM-082 for the checks. Map-side inputs: the checks are computed from rank states, the failed and damaged sets, max_mds and the standby pool. `standby_count_wanted` defaults to -1 and is auto-set to 1 by the monitor tick once an active rank and a standby or follower exist; `fs set <fs> standby_count_wanted <n>` (>= 0) changes it. `ceph status` summarises daemons and file systems from the FSMap.

- **Class:** POSIX `operational` · exposure `metrics`, `admin` · relied on by mon, mgr, admin, cephadm, rook
- **Gates:** standby_count_wanted (default -1, auto 1)
- **Map:** [fsmap-and-mdsmap.md › 5.4 Health checks generated from the maps](../cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#54-health-checks-generated-from-the-maps) · [beacon-and-monitor.md › 3.12 Health metrics → health checks](../cpp-map/01-cluster-integration/beacon-and-monitor.md#312-health-metrics--health-checks) · [rank-lifecycle-and-recovery.md › Health and cluster log](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#health-and-cluster-log)
- **Code:** `src/mds/MDSMap.cc:540-624 (MDSMap::get_health_checks)`, `src/mds/FSMap.cc:601-647 (FSMap::get_health_checks)`, `src/mds/MDSMap.cc:1146-1165 (MDSMap::check_health)`
- **Notes:** The per-rank detail of `FS_DEGRADED` ("is replaying journal", ...) is built into a local list and dropped; the daemon metric `MDS_HEALTH_DAMAGE` and the damaged-rank check share the code `MDS_DAMAGE`; one shared standby satisfies `standby_count_wanted = 1` on every file system (fsmap-and-mdsmap and beacon-and-monitor section 8).

### Boot, creation, replay and standby-replay

Loading a rank's objects, creating new ranks, journal replay and the standby-replay hot standby.

<a id="f-clu-063"></a>

#### F-CLU-063 · Boot stages for up:starting, up:replay and up:standby-replay

*The ordered loading of per-rank objects before a rank can replay or serve, and the error mapping of each step.*

`boot_start` is a four-step continuation chain. INITIAL: default layouts; in one gather `InoTable::load` (`mds<r>_inotable`), `SessionMap::load` (omap of `mds<r>_sessionmap`, 1024 keys per read; every loaded session becomes open), `MDLog::open` (JournalPointer `<0x400+r>.00000000`, journal header, end probe), `PurgeQueue::open` (`<0x500+r>`), rank 0 also `SnapServer::load` (`mds_snaptable`); for replay the purge queue and `OpenFileTable::load` start outside the gather; standby-replay opens neither. OPEN_ROOT: mdsdir inode `<0x100+r>.00000000.inode` (and its dirfrag for starting), the global snaprealm, rank 0 the root inode. PREPARE_LOG: starting appends an `ESubtreeMap`; replay waits for purge-queue recovery and journal replay. REPLAY_DONE: `validate_sessions` then `replay_done`. Errors: `-EAGAIN` in standby-replay respawns; `-EINVAL`/`-ENOENT` mark the rank damaged; `-EROFS` continues read-only; others suicide. IO completions respawn on `-EBLOCKLISTED`/`-ETIMEDOUT` first. `validate_sessions` marks the rank damaged if a session's preallocated inode range intersects the InoTable free set (tracker 16842).

- **Class:** POSIX `supporting` · exposure `on-disk` · relied on by osd, internal
- **Gates:** mds_sessionmap_keys_per_op (1024)
- **Map:** [rank-lifecycle-and-recovery.md › 3.6 Boot stages (MDSRank::boot_start)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#36-boot-stages-mdsrankboot_start) · [rank-lifecycle-and-recovery.md › RADOS objects (metadata pool unless noted)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#rados-objects-metadata-pool-unless-noted)
- **Code:** `src/mds/MDSRank.cc:1611-1742 (MDSRank::boot_start)`, `src/mds/MDSRank.cc:1744-1768 (MDSRank::validate_sessions)`, `src/mds/MDLog.cc:1076-1233 (MDLog::_recovery_thread)`, `src/mds/MDSContext.cc:95-120 (MDSIOContextBase::complete)`
- **Notes:** Object formats belong to [mds-tables](../cpp-map/07-storage-formats/mds-tables.md) and [rados-object-catalog](../cpp-map/07-storage-formats/rados-object-catalog.md). A purge-queue open failure after `wait_for_recovery` is registered delivers the raw error (fatal), before it yields `-EROFS` (tolerated) (rank-lifecycle gotcha 28). See also F-STORE-056 (storage view), F-STORE-037 (replay pipeline).

<a id="f-clu-064"></a>

#### F-CLU-064 · Creating a new rank (up:creating)

*Writes the initial per-rank objects (and, for rank 0, the root) for a rank that never existed.*

`boot_create` in one gather: `MDLog::create` (journal ino `0x200+r` in the metadata pool, header, JournalPointer `{front 0x200+r, back 0}`), an `ELid`; rank 0 only `create_empty_hierarchy` (root inode 1, mode 0755, uid/gid `mds_root_ino_uid/gid`, layout in the first data pool, root dirfrag `1.00000000` and inode `1.00000000.inode`, backtrace); `create_mydir_hierarchy` (mdsdir `0x100+r` with `stray0..stray9` at `0x600+10r+i`, mode 0500); `InoTable::reset` (free range `[(r+1)<<40, +2^40)`) and save; empty SessionMap save; `PurgeQueue::create`; rank 0 `SnapServer::reset` + save; then `ESubtreeMap`, flush, `wait_for_safe`, enable blocklist events. `creating_done` requests ACTIVE and syncs the snap client. If the creating holder dies, `FSMap::erase` forgets the rank and the next holder creates it again, rewriting the same objects without waiting for any blocklist epoch.

- **Class:** POSIX `supporting` · exposure `on-disk`, `cluster-wire` · relied on by mon, osd, admin, tools
- **Gates:** mds_root_ino_uid / mds_root_ino_gid (0/0); mds_journal_format (1); mds_default_dir_hash (2 = rjenkins)
- **Map:** [rank-lifecycle-and-recovery.md › 3.7 Creating a rank (up:creating, MDSRank::boot_create)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#37-creating-a-rank-upcreating-mdsrankboot_create)
- **Code:** `src/mds/MDSRank.cc:2189-2250 (MDSRank::boot_create)`, `src/mds/MDCache.cc:498-574 (MDCache::create_empty_hierarchy)`, `src/mds/MDLog.cc:267-304 (MDLog::create)`, `src/mds/InoTable.cc:29-44 (InoTable::reset_state)`, `src/mds/MDCache.cc:525-557 (MDCache::create_mydir_hierarchy)`
- **Notes:** The dirfrag commits set the header and the keys they hold; they do not clear unrelated omap keys of a recreated object (rank-lifecycle 3.7, derived from code). Object details: the root inode object is `1.00000000.inode` ("ceph fs volume v011" + InodeStore) with an explicit default layout on the first data pool and dl_dir_hash = mds_default_dir_hash; the root dirfrag is created empty and complete as a subtree root with dir_rep REP_ALL; each stray dirfrag and the mdsdir dirfrag are committed (the mdsdir has no backtrace) and the mdsdir inode is stored as `<0x100+r>.00000000.inode`. The monitor requires an empty metadata pool for `fs new` unless `--force`, so mkfs does not normally meet existing objects; `fs reset` marks old ranks stopped so their tables are reused. See also F-STORE-044 (journal creation), F-STORE-069 (InoTable), F-STORE-075 (SessionMap).

<a id="f-clu-065"></a>

#### F-CLU-065 · Restarting a stopped rank (up:starting)

*Brings back a rank that was cleanly stopped, appending to its capped journal without replay.*

The monitor promotes a standby for the lowest rank in `stopped` when `in < max_mds` (also after `fs set down false`). Boot follows the starting column: tables load, the old journal (capped and trimmed by the stop) is not replayed, `MDLog::append` positions at its end and a new `ESubtreeMap` is journaled; `starting_done` requests ACTIVE and syncs the snap client. `active_start` then opens the root (non-zero ranks discover it from rank 0) and populates the mdsdir; `recovery_done` is not called. If the starting holder dies the rank returns to `stopped`.

- **Class:** POSIX `supporting` · exposure `on-disk`, `cluster-wire` · relied on by mon, admin
- **Map:** [rank-lifecycle-and-recovery.md › 3.8 Starting a stopped rank (up:starting)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#38-starting-a-stopped-rank-upstarting)
- **Code:** `src/mds/MDSRank.cc:1770-1778 (MDSRank::starting_done)`, `src/mds/MDCache.cc:737-774 (MDCache::open_root)`
- **Notes:** `up:starting` assumes a clean journal tail and does not wait for any blocklist epoch (rank-lifecycle gotchas 19, 26).

<a id="f-clu-066"></a>

#### F-CLU-066 · up:replay: journal replay of a failed rank

*The up:replay state: rebuilding the rank from its journal before any other recovery stage.*

See F-STORE-037 (replay pipeline) and F-STORE-040 (replay errors). State-machine facts: up:replay starts after the fencing wait (F-CLU-037) and the boot stages (F-CLU-063); besides the cache, replay rebuilds sessions, completed request ids and created inos per session, completed cap flushes, in-progress truncates, inode ranges to purge, uncommitted leader and peer updates, ambiguous imports and fragment operations used by later phases; nothing is journaled during replay. After 30 s in up:replay the beacon carries MDS_ESTIMATED_REPLAY_TIME (F-ADM-081).

- **Class:** POSIX `supporting` · exposure `on-disk`, `metrics` · relied on by osd, admin, internal
- **Gates:** mds_log_skip_corrupt_events (dev); mds_log_skip_unbounded_events (dev)
- **Map:** [rank-lifecycle-and-recovery.md › 3.9 Replay and standby-replay](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#39-replay-and-standby-replay) · [rank-lifecycle-and-recovery.md › Health and cluster log](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#health-and-cluster-log)
- **Code:** `src/mds/MDLog.cc:1027-1061 (MDLog::replay)`, `src/mds/MDLog.cc:1417-1613 (MDLog::_replay_thread)`, `src/mds/Beacon.cc:559-571 (Beacon::notify_health)`, `src/mds/MDLog.cc:227-265 (MDLog::get_estimated_replay_finish_time)`

<a id="f-clu-067"></a>

#### F-CLU-067 · End of replay: journal reformat, writeable journal and the single-rank shortcut

*What happens when replay reaches the end of the journal and which state the rank asks for next.*

`replay_done`: a follower schedules its next pass; otherwise it asserts `read_pos == write_pos`, rewrites the journal through `MDLog::reopen` if its stream format is older than `mds_journal_format`, makes the journal writeable, trims the tail, upgrades the snap table format on rank 0, applies dev wipe options, and requests `up:reconnect` directly when `get_num_in_mds() == 1 && get_num_failed_mds() == 0` ("i am alone", also starting a snap client sync), otherwise `up:resolve`.

- **Class:** POSIX `supporting` · exposure `on-disk`, `cluster-wire` · relied on by mon, internal
- **Legacy parts** (old-data): Journal reformat through MDLog::reopen when the stream format is older than mds_journal_format; the snap table format upgrade on rank 0. New journals are created in mds_journal_format (dev option, default 1 = JOURNAL_FORMAT_RESILIENT, present since v0.82), so the end-of-replay rewrite runs only for format-0 journals written by older C++ releases. SnapServer::upgrade_format() changes the table only when its version is 0 or snaprealm_v2_since is unset, which the decoder produces for struct_v < 5 tables written before v13.1.0. A snap table created at this commit gets version >= 1 and snaprealm_v2_since in reset_state. Evidence: `src/mds/MDSRank.cc:1924-1928 (MDSRank::replay_done)`, `src/mds/MDSRank.cc:1935-1939 (MDSRank::replay_done)`, `src/mds/MDLog.cc:289 (MDLog::create)`, `src/common/options/mds.yaml.in:1241-1247 (mds_journal_format)`, `src/osdc/Journaler.h:78-79 (JOURNAL_FORMAT_LEGACY, JOURNAL_FORMAT_RESILIENT)`, `src/mds/SnapServer.h:42-57 (SnapServer::upgrade_format)`, `src/mds/SnapServer.cc:75 (SnapServer::reset_state)`, `src/mds/MDSTableServer.cc:79-82 (MDSTableServer::reset_state)`, `src/mds/SnapServer.cc:117-120 (SnapServer::decode_server_state)`.
- **Map:** [rank-lifecycle-and-recovery.md › 3.9 Replay and standby-replay](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#39-replay-and-standby-replay) · [resolve-and-rejoin.md › 3.1 Position in the recovery pipeline](../cpp-map/09-recovery/resolve-and-rejoin.md#31-position-in-the-recovery-pipeline)
- **Code:** `src/mds/MDSRank.cc:1897-1968 (MDSRank::replay_done)`
- **Notes:** A damaged second rank keeps a file system out of the shortcut and also blocks resolve until `mds repaired` (rank-lifecycle gotcha 2).

<a id="f-clu-068"></a>

#### F-CLU-068 · Standby-replay follower loop

*A follower tails the active rank's journal every second to keep a warm cache for fast takeover.*

Every `mds_replay_interval` (1 s) `standby_replay_restart` calls `Journaler::reread_head_and_probe` and then `_standby_replay_restart_finish`: if the active trimmed past what the follower had read (`old_read_pos < trimmed_pos`) it respawns; otherwise `standby_trim_segments` drops segments up to `expire_pos` (dirty objects marked clean without writing), trims the cache and replays the new events. The follower never journals, never opens the purge queue or the open file table, never talks to other ranks (messenger `mds.<gid>`), and its `inc` is 0. While a follower exists the active writes its journal without `FADVISE_DONTNEED`.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config` · relied on by mon, admin, osd
- **Gates:** ALLOW_STANDBY_REPLAY flag; mds_replay_interval (1 s)
- **Map:** [rank-lifecycle-and-recovery.md › 3.9 Replay and standby-replay](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#39-replay-and-standby-replay) · [beacon-and-monitor.md › 8. Edge cases, failure handling and gotchas](../cpp-map/01-cluster-integration/beacon-and-monitor.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDSRank.cc:1822-1895 (MDSRank::standby_replay_restart)`, `src/osdc/Journaler.cc:429-455 (Journaler::reread_head_and_probe)`, `src/mds/MDCache.cc:7644-7683 (MDCache::standby_trim_segment)`, `src/mds/MDSRank.cc:2597-2609 (MDSRankDispatcher::handle_mds_map)`
- **Notes:** Fragile: falling behind `trimmed_pos`, a journal rewrite or a missing object respawns the follower and loses its cache; a header re-read error other than `-EBLOCKLISTED` hits `ceph_assert` in `Journaler::_finish_reread_head_and_probe`; a corrupt event makes it report DAMAGED, which marks the rank damaged and blocklists the healthy active (rank-lifecycle gotcha 3). See also F-STORE-043 (journal side), F-CACHE-069 (cache maintenance).

<a id="f-clu-069"></a>

#### F-CLU-069 · Standby-replay takeover

*When the followed rank fails, the follower is promoted in place to `up:replay` and finishes replay from its warm cache.*

`find_replacement_for` returns the follower (unless frozen) and `promote` switches it to `up:replay` and sets `inc`; the MDS only notes the change ("Monitor activated us! Deactivating replay loop"). The current pass (plus one more if its timer was armed) finishes; then `replay_done` starts the final pass: wait for OSDMap >= `last_failure_osd_epoch`, re-read the header and probe, open the purge queue, load the open file table, and replay the remaining events as a normal replay.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, admin, kclient, libcephfs
- **Map:** [rank-lifecycle-and-recovery.md › 3.9 Replay and standby-replay](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#39-replay-and-standby-replay) · [beacon-and-monitor.md › 3.10 Failover and fencing](../cpp-map/01-cluster-integration/beacon-and-monitor.md#310-failover-and-fencing)
- **Code:** `src/mds/MDSRank.cc:1872-1893 (MDSRank::standby_replay_restart)`, `src/mds/FSMap.cc:911-922 (FSMap::find_replacement_for)`, `src/mds/MDSRank.cc:2423-2461 (MDSRankDispatcher::handle_mds_map)`

### Multi-rank recovery and resolve

Cross-rank recovery barriers, survivor behaviour and the resolve phase (subtree authority, two-phase updates, snap table).

<a id="f-clu-070"></a>

#### F-CLU-070 · Recovery set and cluster-wide resolve/rejoin barriers

*Recovering ranks and survivors move through resolve and rejoin in lockstep, synchronised only through the published MDSMap.*

The recovery set is `failed` + `damaged` + ranks in REPLAY..STOPPING, minus self (creating/starting ranks excluded), recomputed at `resolve_start` and at each new resolve round; a single-rank fs never computes it. Every rank evaluates on each map: when `is_resolving()` turns true (or a rank restarted), every rank >= RESOLVE runs `send_resolves`; when `is_rejoining()` turns true, every rank >= REJOIN runs `rejoin_send_rejoins`. Because the barriers require no rank in an earlier recovery state, the slowest replay gates everyone's resolve and every recovering rank's reconnect window (including its 45 s timeout) must end before any rejoin exchange. Resolve and rejoin have no timeouts; progress is driven only by messages, maps and IO.

- **Class:** POSIX `supporting` · exposure `mds-wire`, `cluster-wire` · relied on by mds-ranks, mon
- **Map:** [rank-lifecycle-and-recovery.md › 3.10 Multiple ranks: recovery set, barriers and survivors](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#310-multiple-ranks-recovery-set-barriers-and-survivors) · [resolve-and-rejoin.md › 3.1 Position in the recovery pipeline](../cpp-map/09-recovery/resolve-and-rejoin.md#31-position-in-the-recovery-pipeline) · [resolve-and-rejoin.md › 3.2 Recovery set, survivors and gather sets](../cpp-map/09-recovery/resolve-and-rejoin.md#32-recovery-set-survivors-and-gather-sets)
- **Code:** `src/mds/MDSMap.cc:1236-1243 (MDSMap::get_recovery_mds_set)`, `src/mds/MDSRank.cc:1781-1790 (MDSRank::calc_recovery_set)`, `src/mds/MDSMap.h:530-543 (MDSMap::is_resolving)`, `src/mds/MDSRank.cc:2486-2500 (MDSRankDispatcher::handle_mds_map)`

<a id="f-clu-071"></a>

#### F-CLU-071 · Survivor reactions to a peer failing and recovering

*What still-active ranks do when another rank fails, while they keep serving their own subtrees.*

Every rank >= RESOLVE that sees a peer newly down or restarted runs `handle_mds_failure(who)`: add it to `resolve_gather` and `rejoin_gather`, reset rejoin bookkeeping, notify the migrator and balancer, abort or keep in-flight peer and leader requests according to their 2PC state, retry requests waiting on it via `wait_for_active_peer`, cancel fragment operations that have not started refragmenting, kick `find_ino`/`open_ino` peers, and update snap table and scrub state. Operations needing the failed rank (remote auth pins, remote locks, witnesses) wait. When the peer reaches CLIENTREPLAY/ACTIVE/STOPPING (evaluated only on ranks that are themselves >= CLIENTREPLAY), `handle_mds_recovery` wakes waiters on its subtrees and `waiting_for_active_peer[who]`; ranks already >= REJOIN kick discovers and open_ino requests towards ranks newly >= REJOIN.

- **Class:** POSIX `supporting` · exposure `mds-wire` · relied on by mds-ranks, kclient, libcephfs
- **Map:** [rank-lifecycle-and-recovery.md › 3.10 Multiple ranks: recovery set, barriers and survivors](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#310-multiple-ranks-recovery-set-barriers-and-survivors) · [resolve-and-rejoin.md › 3.4.5 In-flight requests when a rank fails (handle_mds_fa...](../cpp-map/09-recovery/resolve-and-rejoin.md#345-in-flight-requests-when-a-rank-fails-handle_mds_failure) · [resolve-and-rejoin.md › 3.2 Recovery set, survivors and gather sets](../cpp-map/09-recovery/resolve-and-rejoin.md#32-recovery-set-survivors-and-gather-sets)
- **Code:** `src/mds/MDSRank.cc:2628-2654 (MDSRank::handle_mds_recovery)`, `src/mds/MDCache.cc:3006-3181 (MDCache::handle_mds_failure)`, `src/mds/MDCache.cc:3187-3240 (MDCache::handle_mds_recovery)`, `src/mds/MDSRank.cc:2507-2521 (MDSRankDispatcher::handle_mds_map)`
- **Notes:** An `OP_RENAMEPREP` from a failed srcdn auth is marked interrupted; the code comment says this "can cause mds to crash (does not affect integrity of FS)" (resolve-and-rejoin section 8).

<a id="f-clu-072"></a>

#### F-CLU-072 · Deferring messages to bootstrapping peers and dropping stale peer messages

*Inter-rank messaging rules around rank changes: queue messages for booting peers, piggyback maps, drop messages from old incarnations.*

`send_message_mds(m, rank)` drops the message if the rank is not up, parks it in `waiting_for_bootstrapping_peer[rank]` while the peer is CREATING/STARTING/REPLAY (its messenger name changes when it leaves those states) and resends when its state is > REPLAY, and first sends the current `MMDSMap` when the peer's known epoch is older. Incoming messages from an MDS whose rank is down in the local map, or that did not arrive on the connection for the rank's current address, are dropped (`is_stale_message`), except MDSMaps and an `MCacheExpire` from a down rank still at its old address.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [rank-lifecycle-and-recovery.md › 3.10 Multiple ranks: recovery set, barriers and survivors](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#310-multiple-ranks-recovery-set-barriers-and-survivors) · [process-and-threading.md › 3.11 Outbound messaging helpers (MDSRank.cc:1453-1571)](../cpp-map/00-architecture/process-and-threading.md#311-outbound-messaging-helpers-mdsrankcc1453-1571) · [process-and-threading.md › 3.9 Message dispatch path](../cpp-map/00-architecture/process-and-threading.md#39-message-dispatch-path)
- **Code:** `src/mds/MDSRank.cc:1476-1497 (MDSRank::send_message_mds)`, `src/mds/MDSRank.cc:1377-1415 (MDSRank::is_stale_message)`, `src/mds/MDSRank.cc:2533-2547 (MDSRankDispatcher::handle_mds_map)`
- **Notes:** See also F-ADM-004.

<a id="f-clu-073"></a>

#### F-CLU-073 · up:resolve entry and exit

*The multi-rank phase that settles everything left in doubt by the journals of the failed ranks.*

`resolve_start` runs `reopen_log` (uncommitted fragment operations: finish those whose commit was journaled, roll back the others with `EFragment OP_ROLLBACK`), `calc_recovery_set`, `MDCache::resolve_start` (non-root ranks set the root dirfrag authority to unknown so rank 0 claims it; `resolve_gather = recovery_set`; snapshot of journaled snap-client tids) and wakes `waiting_for_resolve`. `maybe_resolve_finish` waits for a subtree resolve from every rank in `resolve_gather`, disambiguates imports, journals pending `ECommitted`, trims unlinked inodes, recomputes auth bits and completes `resolve_done`, which requests `up:reconnect` and starts a snap client sync. A single-rank fs skips this state; `reconnect_start` then runs `reopen_log` itself.

- **Class:** POSIX `supporting` · exposure `mds-wire`, `cluster-wire`, `on-disk` · relied on by mds-ranks, mon
- **Map:** [rank-lifecycle-and-recovery.md › 3.11 Resolve (up:resolve)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#311-resolve-upresolve) · [resolve-and-rejoin.md › 3.6 Finishing resolve (maybe_resolve_finish)](../cpp-map/09-recovery/resolve-and-rejoin.md#36-finishing-resolve-maybe_resolve_finish)
- **Code:** `src/mds/MDSRank.cc:1976-1994 (MDSRank::resolve_start)`, `src/mds/MDCache.cc:2782-2800 (MDCache::resolve_start)`, `src/mds/MDCache.cc:3450-3474 (MDCache::maybe_resolve_finish)`, `src/mds/MDCache.cc:12778-12869 (MDCache::rollback_uncommitted_fragments)`

<a id="f-clu-074"></a>

#### F-CLU-074 · Subtree authority resolution (subtree resolves)

*Each rank claims the subtrees it owns so all ranks agree on authority after a failure.*

After its peer resolves complete (and, on a survivor, a snap cache sync), each rank sends `MMDSResolve` (0x200) with `subtrees` (claimed dirfrags and bounds, nested own subtrees collapsed), `ambiguous_imports` and `table_clients` (snap tids) to every recovery-set rank (recovering sender) or to the ranks currently in RESOLVE (survivor); it is deferred while any migration is in flight. Receivers apply claims with `adjust_bounded_subtree_auth` (force-fragmenting missing bound dirfrags), remember others' ambiguous imports, update the snap client cache, and remove the sender from `resolve_gather`. A resolve received before this rank is in RESOLVE is retried at `resolve_start` if it wants RESOLVE, else dropped.

- **Class:** POSIX `supporting` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [resolve-and-rejoin.md › 3.5 Resolve, step 2: subtree resolves](../cpp-map/09-recovery/resolve-and-rejoin.md#35-resolve-step-2-subtree-resolves) · [resolve-and-rejoin.md › MDS to MDS messages](../cpp-map/09-recovery/resolve-and-rejoin.md#mds-to-mds-messages)
- **Code:** `src/mds/MDCache.cc:2888-2995 (MDCache::send_subtree_resolves)`, `src/mds/MDCache.cc:3257-3433 (MDCache::handle_resolve)`, `src/messages/MMDSResolve.h:1-163 (MMDSResolve)`
- **Notes:** The MDS-to-MDS protocol is free to redesign; only Rust ranks will speak it.

<a id="f-clu-075"></a>

#### F-CLU-075 · Ambiguous subtree import disambiguation

*Decides deterministically who owns a subtree whose migration was interrupted by a failure.*

An import journaled (`EImportStart`) but not finished on a failed importer, or an import on a surviving importer in `IMPORT_ACKING` whose exporter failed, is ambiguous. A surviving importer reverses it if the exporter's resolve claims the subtree (no `EExport` on disk) and finishes it otherwise. A recovering importer, in `disambiguate_my_imports`, cancels and journals `EImportFinish(false)` if someone claimed it, else finishes it and journals `EImportFinish(true)`, then asserts no subtree is left with ambiguous authority. Migration protocol: [subtree-migration](../cpp-map/08-multi-mds/subtree-migration.md).

- **Class:** POSIX `supporting` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks, kclient
- **Map:** [resolve-and-rejoin.md › 3.7 Outcome matrices](../cpp-map/09-recovery/resolve-and-rejoin.md#37-outcome-matrices) · [resolve-and-rejoin.md › 3.6 Finishing resolve (maybe_resolve_finish)](../cpp-map/09-recovery/resolve-and-rejoin.md#36-finishing-resolve-maybe_resolve_finish)
- **Code:** `src/mds/MDCache.cc:3662-3801 (MDCache::disambiguate_my_imports)`, `src/mds/MDCache.cc:3351-3398 (MDCache::handle_resolve)`, `src/mds/Migrator.cc:622-630 (Migrator::handle_mds_failure_or_stop)`
- **Notes:** See also F-MULTI-023 (migration side).

<a id="f-clu-076"></a>

#### F-CLU-076 · Cross-rank two-phase update outcomes (peer resolves and ECommitted)

*Settles link, unlink, rmdir and rename operations that spanned ranks when a participant failed.*

The commit point is the leader's `EUpdate` (with `had_peers`) being durable. Recovering peers send `MMDSResolve` with `peer_requests` for every uncommitted `EPeerUpdate OP_PREPARE`; survivors send entries for prepared or committing peer requests whose leader is resolving. The leader answers `MMDSResolveAck` (0x201): COMMIT if it has the reqid in `uncommitted_leaders` (re-adding the peer so `ECommitted` waits for its `OP_COMMITTED`), ABORT otherwise (presumed abort); a survivor leader delays all answers while any listed `EUpdate` is still unsafe. A recovering peer journals `EPeerUpdate OP_COMMIT` and sends `OP_COMMITTED`, or rolls back from its saved rollback blob and journals `OP_ROLLBACK`. `finish_committed_leaders` journals `ECommitted` for leaders with no peers left. Unresolved entries pin both journals' tails.

- **Class:** POSIX `supporting` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks, internal
- **Map:** [resolve-and-rejoin.md › 3.4 Resolve, step 1: peer resolves (two-phase commit outc...](../cpp-map/09-recovery/resolve-and-rejoin.md#34-resolve-step-1-peer-resolves-two-phase-commit-outcomes) · [resolve-and-rejoin.md › 3.4.4 Leader-side commit tracking](../cpp-map/09-recovery/resolve-and-rejoin.md#344-leader-side-commit-tracking) · [resolve-and-rejoin.md › 3.7 Outcome matrices](../cpp-map/09-recovery/resolve-and-rejoin.md#37-outcome-matrices)
- **Code:** `src/mds/MDCache.cc:2832-2886 (MDCache::send_peer_resolves)`, `src/mds/MDCache.cc:3274-3341 (MDCache::handle_resolve)`, `src/mds/MDCache.cc:3476-3561 (MDCache::handle_resolve_ack)`, `src/mds/MDCache.cc:2512-2568 (MDCache::finish_committed_leaders)`, `src/messages/MMDSResolveAck.h:1-68 (MMDSResolveAck)`
- **Notes:** The `pending_leaders` gate is global: one unsafe leader update delays the peer resolves of every sender (resolve-and-rejoin section 6).

<a id="f-clu-077"></a>

#### F-CLU-077 · Rename-specific resolve handling

*Extra resolve rules for cross-rank renames: ambiguous witnesses and client caps of a renamed inode exported by a peer.*

When the srcdn auth of a cross-rank rename fails while the leader is active, a surviving witness records an ambiguous peer update and sends a peer resolve to the active leader; on commit the entry is removed and the rename continues, on abort the rollback path preserves the peer request. A survivor peer that was exporting the renamed inode sends its current client caps (`peer_inode_cap`) instead of the committing flag; the leader (in RESOLVE) assigns each a new cap id (`++last_cap_id`, `issue_seq = 1`) in the ack and later sends `CEPH_CAP_OP_IMPORT` with an AUTH peer during rejoin. Rename protocol: [rename](../cpp-map/03-request-path/rename.md).

- **Class:** POSIX `supporting` · exposure `mds-wire`, `client-wire` · relied on by mds-ranks, kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [resolve-and-rejoin.md › 3.4.6 Ambiguous peer updates (rename)](../cpp-map/09-recovery/resolve-and-rejoin.md#346-ambiguous-peer-updates-rename) · [resolve-and-rejoin.md › 3.4.2 Leader side (handle_resolve with peer_requests)](../cpp-map/09-recovery/resolve-and-rejoin.md#342-leader-side-handle_resolve-with-peer_requests) · [resolve-and-rejoin.md › 3.13.2 Cap matching, round 2 (process_imported_caps with ...](../cpp-map/09-recovery/resolve-and-rejoin.md#3132-cap-matching-round-2-process_imported_caps-with-self-no-longer-in-rejoin_gather)
- **Code:** `src/mds/MDCache.cc:2860 (MDCache::send_peer_resolves)`, `src/mds/MDCache.cc:3071-3077 (MDCache::handle_mds_failure)`, `src/mds/MDCache.cc:5565-5597 (MDCache::process_imported_caps)`, `src/mds/Server.cc:10913-10920 (Server::do_rename_rollback)`

<a id="f-clu-078"></a>

#### F-CLU-078 · Snap table recovery and the SERVER_READY gate

*How snap-table recovery gates other ranks' recovery.*

See F-STORE-082 for table 2PC recovery (finish_recovery, NOTIFY_PREP, AGREE re-sends, SERVER_READY). Recovery-state facts: the table server rank runs finish_recovery when it reaches up:resolve, and sends SERVER_READY to ranks newly reaching up:resolve; reconnect_gather_finish waits for the snap client to be synced before requesting up:rejoin, so a rank 0 failure during another rank's reconnect holds that rank in up:reconnect until the new rank 0 has recovered the table.

- **Class:** POSIX `supporting` · exposure `mds-wire` · relied on by mds-ranks, internal
- **Map:** [resolve-and-rejoin.md › 3.5.3 Snap table server and client recovery](../cpp-map/09-recovery/resolve-and-rejoin.md#353-snap-table-server-and-client-recovery) · [rank-lifecycle-and-recovery.md › 3.12 Reconnect (up:reconnect): the client window](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#312-reconnect-upreconnect-the-client-window)
- **Code:** `src/mds/MDSTableServer.cc:327-408 (MDSTableServer::finish_recovery)`, `src/mds/MDSTableClient.cc:53-149 (MDSTableClient::handle_request)`, `src/mds/SnapClient.cc:189-205 (SnapClient::refresh)`, `src/mds/Server.cc:1828-1842 (Server::reconnect_gather_finish)`

<a id="f-clu-079"></a>

#### F-CLU-079 · Post-resolve cache cleanup and authority bits

*After resolve the recovering rank drops orphaned inodes and recomputes which cached objects it is authoritative for.*

`trim_unlinked_inodes` removes every non-base inode without a parent dentry (heartbeat reset every `mds_heartbeat_reset_grace` items). `recalc_auth_bits(false)` walks every subtree: objects in auth subtrees get AUTH; objects in non-auth subtrees lose AUTH and get REJOINING, dirfrags lose COMPLETE, empty non-subtree-root dirfrags are closed, inodes are marked clean (dirty parent and scatter state cleared except own subtree roots). A survivor instead may send pending rejoins. None of this runs for a single-rank file system.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [resolve-and-rejoin.md › 3.6 Finishing resolve (maybe_resolve_finish)](../cpp-map/09-recovery/resolve-and-rejoin.md#36-finishing-resolve-maybe_resolve_finish) · [resolve-and-rejoin.md › 3.18 Single rank (max_mds = 1)](../cpp-map/09-recovery/resolve-and-rejoin.md#318-single-rank-max_mds--1)
- **Code:** `src/mds/MDCache.cc:3875-3900 (MDCache::trim_unlinked_inodes)`, `src/mds/MDCache.cc:3902-4018 (MDCache::recalc_auth_bits)`

### Client reconnect, rejoin and clientreplay

The client-facing recovery phases: reconnect window, rejoin of caps, locks and replicas across ranks, and ordered client replay.

<a id="f-clu-080"></a>

#### F-CLU-080 · up:reconnect: the client reconnect window and its timeout

*The window in which clients of the failed rank re-register their sessions, caps and locks before any request runs.*

`reconnect_start` runs `reopen_log` if coming from replay, enables blocklist events and kills sessions whose address is blocklisted (raising the OSD epoch barrier if any), puts every open session in `client_reconnect_gather` with `last_cap_renew = now`, and wakes reconnects that arrived early. Clients act when the map shows the rank in `up:reconnect` (10). `reconnect_tick` (every 5 s tick, skipped while laggy) returns while blocklist evictions are pending or the gather is empty, and otherwise waits at least `mds_reconnect_timeout` (45 s) from `reconnect_start` and until neither a reconnect message nor any message from a not-yet-reconnected session arrived for `mds_reconnect_timeout`/2 (cap flushes and replayed requests sent ahead of the reconnect therefore extend the window; it has no upper bound while activity continues); then each missing client gets a cluster-log warning and is evicted (blocklisted with `mds_session_blocklist_on_timeout`, default true, after swapping `delegated_inos` and `free_prealloc_inos` so lost async creates are purged), except sessions whose metadata has a `timeout` key, which are kept for reclaim. The gather also finishes when every client reconnected or was closed; `reconnect_gather_finish` then waits for the snap client sync and requests `up:rejoin`.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, admin, osd
- **Gates:** mds_reconnect_timeout (45 s); mds_session_blocklist_on_timeout (true); mds_deny_all_reconnect (false)
- **Map:** [rank-lifecycle-and-recovery.md › 3.12 Reconnect (up:reconnect): the client window](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#312-reconnect-upreconnect-the-client-window) · [rank-lifecycle-and-recovery.md › 3.23 Recovery timeline and timing knobs](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#323-recovery-timeline-and-timing-knobs)
- **Code:** `src/mds/MDSRank.cc:2007-2030 (MDSRank::reconnect_start)`, `src/mds/Server.cc:1563-1589 (Server::reconnect_clients)`, `src/mds/Server.cc:1844-1943 (Server::reconnect_tick)`, `src/mds/Server.cc:1828-1842 (Server::reconnect_gather_finish)`
- **Notes:** Message encoding and session states: [sessions](../cpp-map/02-client-protocol/sessions.md). `reconnect_evicting` is never reset. Operators shorten the window by evicting clients listed by `session ls reconnecting=true`. With `mds_deny_all_reconnect` every reconnect is answered CLOSE; once every session left in the gather has been denied, `reconnect_tick` evicts them without waiting for the timeout or the activity extension. See also F-PROTO-057 (acceptance rules), F-PROTO-055 (client-side ordering), F-PROTO-056 (message encoding), F-CLU-096 (reclaim gating).

<a id="f-clu-082"></a>

#### F-CLU-082 · Recording caps and snaprealms from reconnect messages

*Turns each client's reconnect payload into the per-inode cap state that rejoin later matches and distributes.*

For every part: `last_cap_renew = now`; each realm `{ino, seq, parent}` is recorded in `reconnected_snaprealms`; for each cap `last_cap_id` is raised to at least its `cap_id` and inodes being purged are skipped; an inode in cache and auth gets `CInode::reconnect_cap` (new cap keeps the client's `cap_id`, `wanted`, issued without revoke, `seq` reset) plus `add_reconnected_cap(realm_ino, snap_follows)`; an inode in cache but not auth goes to `cap_exports` for its authority (sent in the weak rejoin); an inode not in cache goes to `cap_imports` (resolved during rejoin). Dirty cap flushes with a new `client_tid` received during reconnect are OR-ed into `reconnected_caps[ino][client].dirty_caps` (snapflush flag for FLUSHSNAP). The client's parent ino + single-component name in each cap record is used later as an open hint.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [resolve-and-rejoin.md › 3.8 What reconnect hands to rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#38-what-reconnect-hands-to-rejoin) · [rank-lifecycle-and-recovery.md › 3.12 Reconnect (up:reconnect): the client window](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#312-reconnect-upreconnect-the-client-window)
- **Code:** `src/mds/Server.cc:1703-1767 (Server::handle_client_reconnect)`, `src/mds/CInode.cc:3676-3690 (CInode::reconnect_cap)`, `src/mds/Locker.cc:3410-3415 (Locker::handle_client_caps)`, `linux/fs/ceph/mds_client.c:4878-5072 (reconnect_caps_cb)`
- **Notes:** Cap semantics: [capabilities](../cpp-map/02-client-protocol/capabilities.md).

<a id="f-clu-084"></a>

#### F-CLU-084 · Admission of replayed and retried requests during reconnect

*Requests that clients replay or resend after a failover are queued for ordered re-execution instead of waiting for `up:active`.*

While in (or wanting) `up:reconnect`: a `CEPH_MSG_CLIENT_REQUEST` with `CEPH_MDS_FLAG_REPLAY` (1) or `CEPH_MDS_FLAG_ASYNC` (4) from an open session is marked `queued_for_replay` and appended to `replay_queue`; if `head.ino` is set and not already completed the ino is excluded from opening during rejoin (`add_replay_ino_alloc`). A retried request (`num_retry > 0`) is queued only if the session already has it in `completed_requests`, else it waits for active. Requests from missing or non-open sessions are dropped silently unless `mds_replay_unsafe_with_closed_session`. Session renewals are handled in any state; other session ops, reclaims, cap messages and cap releases wait for `up:clientreplay`; session close waits for active; other requests wait for active.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_replay_unsafe_with_closed_session (false, read at startup)
- **Map:** [rank-lifecycle-and-recovery.md › 3.12 Reconnect (up:reconnect): the client window](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#312-reconnect-upreconnect-the-client-window) · [process-and-threading.md › 3.9 Message dispatch path](../cpp-map/00-architecture/process-and-threading.md#39-message-dispatch-path)
- **Code:** `src/mds/Server.cc:319-420 (Server::dispatch)`, `src/mds/Server.cc:349-362 (Server::dispatch)`, `src/mds/Locker.cc:3386-3418 (Locker::handle_client_caps)`
- **Notes:** With mds_replay_unsafe_with_closed_session a carried head.ino of a not-completed request is moved from the free to the delegated preallocation set so that it is not purged. Retried requests queued for replay lose their embedded cap releases. Requests re-sent during reconnect are not flagged REPLAY, only retried. Kernels send new requests only to ranks in up:active or later and libcephfs only to active or stopping ranks; libcephfs kick_requests at ACTIVE re-sends nothing in practice (derived from code). See also F-NS-002 (intake gating), F-CLU-095 (clientreplay).

<a id="f-clu-085"></a>

#### F-CLU-085 · Rejoin message exchange: weak rejoin, strong rejoin and acks

*Rebuilds replica lists, nonces and lock states between a recovering rank and every other rank.*

`MMDSCacheRejoin` (0x202, v2) with op WEAK (1), STRONG (2) or ACK (3). A recovering rank sends WEAK to every recovery-set rank: weak dirfrags and dentries of the subtrees owned by the destination, dirty scatterlock data, root/mdsdir as weak inodes, and forwarded `cap_exports` with client instances and metadata. A survivor sends STRONG to each rank in REJOIN: replica nonces, lock states, `caps_wanted`, dentry linkage, plus its in-flight requests' remote auth pins, xlocks and wrlocks. Survivors answer WEAK immediately; a recovering rank sends its ACK (incremented nonces, `dirfrag_base`, `inode_base`, `inode_locks`, `imported_caps`) after its own gather finishes, built by walking every auth subtree. Survivors do not exchange rejoins with each other. There is no chunking: one message per rank pair of size proportional to the replicas.

- **Class:** POSIX `supporting` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [resolve-and-rejoin.md › 3.10 Who sends which rejoin message](../cpp-map/09-recovery/resolve-and-rejoin.md#310-who-sends-which-rejoin-message) · [resolve-and-rejoin.md › 3.11 Weak rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#311-weak-rejoin) · [resolve-and-rejoin.md › 3.14 Handling a rejoin ack](../cpp-map/09-recovery/resolve-and-rejoin.md#314-handling-a-rejoin-ack) · [resolve-and-rejoin.md › MDS to MDS messages](../cpp-map/09-recovery/resolve-and-rejoin.md#mds-to-mds-messages)
- **Code:** `src/mds/MDCache.cc:4088-4316 (MDCache::rejoin_send_rejoins)`, `src/mds/MDCache.cc:4488-4758 (MDCache::handle_cache_rejoin_weak)`, `src/mds/MDCache.cc:6168-6334 (MDCache::rejoin_send_acks)`, `src/mds/MDCache.cc:5057-5328 (MDCache::handle_cache_rejoin_ack)`, `src/messages/MMDSCacheRejoin.h:1-407 (MMDSCacheRejoin)`
- **Notes:** `up:rejoin` cannot be skipped (state graph). Survivors scan every inode in `inode_map` and `snap_inode_map` per weak rejoin (`rejoin_scour_survivor_replicas`) without heartbeat resets (resolve-and-rejoin section 3.20).

<a id="f-clu-086"></a>

#### F-CLU-086 · Recreating survivors' in-flight lock state on the recovering rank

*Strong rejoins carry the remote auth pins, xlocks and wrlocks that survivors' in-flight requests hold on the recovering rank's objects, so those requests can continue.*

For each auth pin in a strong rejoin the recovering rank creates (or reuses) a peer `MDRequest` (`request_start_peer(reqid, attempt)`) that pins the object; an xlock takes the versionlock xlock locally, sets `LOCK_XLOCK` and `get_xlock`; a wrlock sets the lock to `LOCK_MIX` and `get_wrlock(true)`; frozen auth pins call `freeze_auth_pin`. Missing objects are invented. A strong rejoin arriving outside `rejoin` aborts the daemon unless it wants REJOIN.

- **Class:** POSIX `supporting` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [resolve-and-rejoin.md › 3.12 Strong rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#312-strong-rejoin)
- **Code:** `src/mds/MDCache.cc:4788-5053 (MDCache::handle_cache_rejoin_strong)`

<a id="f-clu-087"></a>

#### F-CLU-087 · Matching reconnected caps to inodes: OpenFileTable prefetch and open by number

*Finds the inode behind every cap a client reported that was not in cache after replay.*

At rejoin start (once per daemon) the rank prefetches the OpenFileTable anchors (`mds<r>_openfiles.<i>`): directory inodes first (`open_ino` with the anchor chain as hint), optionally their dirfrags (`mds_oft_prefetch_dirfrags`), then file inodes; results naming another rank move caps to `cap_exports`, negatives go to `cap_imports_missing`. Remaining cap inodes are opened with `open_ino`: using the client's parent ino + single dentry name as a hint when present, otherwise asking peers in REJOIN or later and then reading the backtrace (`parent` xattr of `<ino>.00000000` in the default data pool, the backtrace's pool, then the metadata pool); dentry fetches are batched per dirfrag. Then `trim_non_auth` expires every unpinned non-auth dentry before weak rejoins are sent.

- **Class:** POSIX `supporting` · exposure `on-disk`, `client-wire` · relied on by kclient, libcephfs, osd, internal, ceph-fuse, ganesha
- **Gates:** mds_oft_prefetch_dirfrags (false); mds_dir_prefetch (true)
- **Map:** [resolve-and-rejoin.md › 3.9 Rejoin start and cap matching, round 1](../cpp-map/09-recovery/resolve-and-rejoin.md#39-rejoin-start-and-cap-matching-round-1) · [resolve-and-rejoin.md › 3.13.2 Cap matching, round 2 (process_imported_caps with ...](../cpp-map/09-recovery/resolve-and-rejoin.md#3132-cap-matching-round-2-process_imported_caps-with-self-no-longer-in-rejoin_gather)
- **Code:** `src/mds/MDCache.cc:5483-5649 (MDCache::process_imported_caps)`, `src/mds/OpenFileTable.cc:999-1226 (OpenFileTable::prefetch_inodes)`, `src/mds/MDCache.cc:9117-9162 (MDCache::_open_ino_backtrace_fetched)`, `src/mds/MDCache.cc:7401-7505 (MDCache::trim_non_auth)`
- **Notes:** See also F-STORE-089 (OpenFileTable prefetch).

<a id="f-clu-088"></a>

#### F-CLU-088 · Moving reconnected caps between ranks (IMPORT/EXPORT during rejoin)

*A cap a client reported to the wrong rank is moved to the inode's authority and the client is told with cap IMPORT and EXPORT messages.*

The rank that received the reconnect forwards the cap in its weak rejoin. The authority installs it: a survivor only if the client already has a session with it (`rejoin_import_cap`), a recovering auth after force-opening sessions; a new cap keeps the client's `cap_id`, gets `seq` 1 and an incremented `mseq`, and the client receives `CEPH_CAP_OP_IMPORT` with peer `{old cap_id, 0, 0, old rank}`. The authority lists the result in its ack `imported_caps`; the forwarding rank then sends `CEPH_CAP_OP_EXPORT` (head cap_id = client's old id, peer = new cap or `mds = -1` to drop it). IMPORT and EXPORT come from different ranks in either order; the kernel and libcephfs match by `cap_id`.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [resolve-and-rejoin.md › 3.11 Weak rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#311-weak-rejoin) · [resolve-and-rejoin.md › 3.14 Handling a rejoin ack](../cpp-map/09-recovery/resolve-and-rejoin.md#314-handling-a-rejoin-ack) · [resolve-and-rejoin.md › Messages to clients produced by resolve/rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#messages-to-clients-produced-by-resolverejoin)
- **Code:** `src/mds/MDCache.cc:4539-4690 (MDCache::handle_cache_rejoin_weak)`, `src/mds/MDCache.cc:5599-5639 (MDCache::process_imported_caps)`, `src/mds/MDCache.cc:5954-5973 (MDCache::do_cap_import)`, `linux/fs/ceph/caps.c:4154-4281 (handle_cap_export)`
- **Notes:** Forwarded caps for inodes a recovering auth cannot open are never answered: the forwarding rank sends no EXPORT and the client keeps a cap no rank tracks (resolve-and-rejoin section 8, derived from code). See also F-CAPS-036 (EXPORT/IMPORT messages).

<a id="f-clu-089"></a>

#### F-CLU-089 · Forced session open during rejoin

*Rejoin force-opens sessions for every client known from reconnect or from other ranks' weak rejoins.*

See F-PROTO-041. Rejoin-specific: once per rejoin, if rejoin_client_map is non-empty, prepare_force_open_sessions runs for those clients and the rank journals ESessions and flushes; on safe the sessions moved to OPENING become OPEN and receive CEPH_SESSION_OPEN (cap auths, supported features and metric spec for Mimic+ clients, FORCE_RO if read-only). Even a single rank journals an ESessions whenever any client reconnected.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The v1 CEPH_SESSION_OPEN without supported features or metric spec for sessions lacking CEPHFS_FEATURE_MIMIC (bit 8). Server::finish_force_open_sessions adds the supported-feature bitmap and metric spec only for sessions with CEPHFS_FEATURE_MIMIC (bit 8); for other sessions MClientSession falls back to a v1 message. The 7.3 kernel, both CentOS Stream samples and current libcephfs advertise bit 8; kernels up to v4.18 and Ceph clients up to 13.2.1 do not. Evidence: `src/mds/Server.cc:1130-1135 (Server::finish_force_open_sessions)`, `src/messages/MClientSession.h:101-105 (MClientSession::encode_payload)`, `linux/fs/ceph/mds_client.h:45-47 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:63 (CEPHFS_FEATURES_ALL)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`.
- **Map:** [resolve-and-rejoin.md › 3.13.2 Cap matching, round 2 (process_imported_caps with ...](../cpp-map/09-recovery/resolve-and-rejoin.md#3132-cap-matching-round-2-process_imported_caps-with-self-no-longer-in-rejoin_gather)
- **Code:** `src/mds/Server.cc:1059-1147 (Server::prepare_force_open_sessions)`, `src/mds/MDCache.cc:5456-5463 (MDCache::rejoin_open_sessions_finish)`

<a id="f-clu-090"></a>

#### F-CLU-090 · Choosing lock states from reconnected caps

*After rejoin the authority picks each inode lock's state from the caps clients hold, so caps and locks agree before serving.*

`choose_lock_states_and_reconnect_caps` visits every head inode: re-marks dirty rstat, calls `CInode::choose_lock_states(issued | reconnected dirty caps)` and queues inodes with a snaprealm for reconciliation. For filelock, nestlock, dirfragtreelock, authlock, xattrlock and linklock on the auth (unless xlocked or MIX): EXCL if the lock's GEXCL|GBUFFER bits are issued or GWR with GCACHE|GSHARED, MIX if only GWR, else MIX (replicated) or LOCK (not replicated) if dirty, else SYNC; a loner is chosen when an EXCL/WR cap is issued.

- **Class:** POSIX `supporting` · exposure `internal` · relied on by kclient, libcephfs, internal
- **Map:** [resolve-and-rejoin.md › 3.13.3 Lock states, file recovery, acks](../cpp-map/09-recovery/resolve-and-rejoin.md#3133-lock-states-file-recovery-acks) · [resolve-and-rejoin.md › 3.16 Lock-state recovery rules](../cpp-map/09-recovery/resolve-and-rejoin.md#316-lock-state-recovery-rules)
- **Code:** `src/mds/MDCache.cc:5698-5728 (MDCache::choose_lock_states_and_reconnect_caps)`, `src/mds/CInode.cc:3503-3545 (CInode::choose_lock_state)`
- **Notes:** Lock state machines: [lock-types-and-state-machines](../cpp-map/05-locking/lock-types-and-state-machines.md).

<a id="f-clu-091"></a>

#### F-CLU-091 · Scatterlock recovery and NEED_RECOVER

*Rules that carry dirty scatter data (dirstat/rstat) across rejoin and keep requests off locks whose state may be stale until the cluster recovers.*

Recovering replicas send scatterlock data for dirty subtree-root inodes in weak rejoins; survivors send their scatterlock state and dirty data in strong rejoins, and the recovering auth goes to MIX if any replica is in a MIX state. Acks re-send `inode_base` for every inode whose scatter content may have changed. The auth encodes MIX_SYNC/MIX_LOCK for replicas still in a gather and marks its lock NEED_RECOVER for MIX states; replica locks set from an ack are marked NEED_RECOVER always on a recovering rank and on a survivor unless previously SYNC. While the cluster is degraded, a non-replayed request needing an rdlock on a NEED_RECOVER lock drops its locks and waits for cluster recovery; `MLock` messages for objects still REJOINING are dropped.

- **Class:** POSIX `supporting` · exposure `mds-wire` · relied on by mds-ranks, internal
- **Map:** [resolve-and-rejoin.md › 3.16 Lock-state recovery rules](../cpp-map/09-recovery/resolve-and-rejoin.md#316-lock-state-recovery-rules)
- **Code:** `src/mds/ScatterLock.h:139-194 (ScatterLock::infer_state_from_strong_rejoin)`, `src/mds/SimpleLock.h:272-284 (SimpleLock::set_state_rejoin)`, `src/mds/Locker.cc:636-650 (Locker::acquire_locks)`
- **Notes:** See also F-CAPS-090 (lock waits on a degraded cluster).

<a id="f-clu-092"></a>

#### F-CLU-092 · Snaprealm reconciliation and snapflush rebuild at the end of rejoin

*Rejoin step that catches clients up on snap realms and rebuilds owed snapshot flushes.*

See F-SNAP-032. Recovery-specific: MDCache::open_snaprealms runs at the end of rejoin for the realms queued from reconnect messages; the first snapped inode of a cap whose snap_follows is older gets its locks set to LOCK_SNAP_SYNC with a wrlock until the owed flush arrives (rebuild_need_snapflush), and a FLUSHSNAP re-sent during reconnect marks the cap instead. Realms reported but not found are only logged. A survivor that sees a realm change in a rejoin ack sends SPLIT to clients with caps in the realm.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [resolve-and-rejoin.md › 3.15 Snaprealm reconnect and snapflush rebuild (open_snap...](../cpp-map/09-recovery/resolve-and-rejoin.md#315-snaprealm-reconnect-and-snapflush-rebuild-open_snaprealms) · [resolve-and-rejoin.md › Messages to clients produced by resolve/rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#messages-to-clients-produced-by-resolverejoin)
- **Code:** `src/mds/MDCache.cc:5982-6057 (MDCache::open_snaprealms)`, `src/mds/MDCache.cc:5651-5696 (MDCache::rebuild_need_snapflush)`, `src/mds/MDCache.cc:5730-5748 (MDCache::prepare_realm_split)`, `src/mds/MDCache.cc:10348-10362 (MDCache::notify_global_snaprealm_update)`
- **Notes:** See also F-CAPS-024 (FLUSHSNAP protocol), F-PROTO-061 (realm records in reconnect).

<a id="f-clu-093"></a>

#### F-CLU-093 · Rejoin cache repair: invented objects, stale fragmentation, missed link/unlink

*The authority fills gaps in its cache with placeholder objects fetched from RADOS and repairs replicas whose structure diverged.*

Objects named in strong rejoins but absent locally are invented (`REJOINUNDEF` inode with only its number, or frag `*` dirfrag with the default hash) and fetched top-down from their parent dirfrags before the gather finishes (`open_undef_inodes_dirfrags`). Survivors' dirfrags that do not match the auth's fragtree are mapped onto local leaves, and the replica refragments from the ack. A survivor whose primary linkage differs from the auth's missed an `MDentryLink`/`MDentryUnlink`; the auth re-replicates the inode's new location in its ack and the replica repairs its linkage in `handle_cache_rejoin_ack`. Snapshot dentries are trimmed from strong rejoins.

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks, osd, internal
- **Map:** [resolve-and-rejoin.md › 3.13.1 Open undefined inodes and dirfrags](../cpp-map/09-recovery/resolve-and-rejoin.md#3131-open-undefined-inodes-and-dirfrags) · [resolve-and-rejoin.md › 3.12 Strong rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#312-strong-rejoin) · [resolve-and-rejoin.md › 8. Edge cases, failure handling and gotchas](../cpp-map/09-recovery/resolve-and-rejoin.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDCache.cc:4760-4787 (MDCache::rejoin_invent_inode)`, `src/mds/MDCache.cc:6059-6149 (MDCache::open_undef_inodes_dirfrags)`, `src/mds/MDCache.cc:4930-4955 (MDCache::handle_cache_rejoin_strong)`
- **Notes:** Inconsistencies are handled by `ceph_assert` rather than repair in several places (non-empty `isolated_inodes` after an ack, dir hash mismatch for an opened undefined directory, unexpected states in `rejoin_open_ino_finish`) (resolve-and-rejoin section 8).

<a id="f-clu-094"></a>

#### F-CLU-094 · Rejoin on a single-rank file system

*With one rank, rejoin runs without messages but still pays the full cap-matching and cache-scan cost before serving.*

For `max_mds = 1`: resolve is skipped, the recovery set is empty, and no `disambiguate_my_imports`, `finish_committed_leaders`, `trim_unlinked_inodes` or `recalc_auth_bits(false)` runs. Rejoin still performs the OpenFileTable prefetch, `open_ino` of every reconnected cap inode not in cache, `trim_non_auth`, forced session opens (an `ESessions` event if any client reconnected), cap reconnect, lock state choice, file recovery identification and `open_snaprealms` (client `MClientSnap` messages) before `rejoin_done`.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, internal, ceph-fuse, ganesha
- **Map:** [resolve-and-rejoin.md › 3.18 Single rank (max_mds = 1)](../cpp-map/09-recovery/resolve-and-rejoin.md#318-single-rank-max_mds--1)
- **Code:** `src/mds/MDCache.cc:4065-4075 (MDCache::rejoin_start)`, `src/mds/MDCache.cc:5380-5405 (MDCache::rejoin_gather_finish)`

<a id="f-clu-095"></a>

#### F-CLU-095 · up:clientreplay: ordered re-execution of replayed requests

*Re-executes, one at a time in arrival order, requests whose unsafe replies clients already saw, before any new request is served.*

`rejoin_done` requests CLIENTREPLAY if `replay_queue` is non-empty or reclaims are pending, else ACTIVE (or STOPPED with no subtrees). `clientreplay_start` wakes `waiting_for_replay` and `queue_one_replay` moves the head of `replay_queue` to the finished queue; the next one is queued only when the current request reaches journal submission (replays never get an early reply), is cleaned up, or is dropped as already completed (a reply with the created ino in `extra_bl`) or duplicate. Replays skip the wait-for-cluster-recovery lock paths, reuse the client's reconnected cap for inodes they create, bypass QoS, and are logged as "failed to replay" on error. When the queue is empty and the reclaim gather is empty, the journal is made safe and ACTIVE is requested.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [rank-lifecycle-and-recovery.md › 3.14 Clientreplay (up:clientreplay)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#314-clientreplay-upclientreplay) · [rank-lifecycle-and-recovery.md › 7. Externally visible contract](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSRank.cc:2083-2127 (MDSRank::clientreplay_start)`, `src/mds/Server.cc:2143-2151 (Server::journal_and_reply)`, `src/mds/Server.cc:2636-2706 (Server::handle_client_request)`, `src/mds/MDCache.cc:5912-5947 (MDCache::try_reconnect_cap)`
- **Notes:** Completed-request detection per session belongs to [request-lifecycle](../cpp-map/03-request-path/request-lifecycle.md). Possible stall by code reading: a replayed request already completed whose session is not open returns before `queue_one_replay` (rank-lifecycle gotcha 10, unverified at runtime). See also F-NS-013 (handler-side rules), F-CAPS-059 (creates).

<a id="f-clu-096"></a>

#### F-CLU-096 · Reclaim gating of the clientreplay to active transition

*Sessions of NFS-Ganesha instances that did not reconnect hold the whole rank in `up:clientreplay` until reclaimed, closed or autoclosed.*

At reconnect timeout, missing sessions whose metadata has a `timeout` key (set by `ceph_set_session_timeout`, used by Ganesha) are kept in `client_reclaim_gather` instead of being evicted. A restarted instance opens a new session (allowed from clientreplay) and sends `CEPH_MSG_CLIENT_RECLAIM` with `CEPH_RECLAIM_RESET` and the same `uuid` (same auth name or EPERM); the old session is then killed or evicted with blocklisting. `maybe_clientreplay_done` requires the reclaim gather to be empty. With defaults a never-reclaimed session is evicted by `find_idle_sessions` after `session_autoclose` (300 s) from `last_cap_renew`, so the rank may stay in `up:clientreplay`, serving no new requests, for about 300 s; the `timeout` value is not consulted.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by ganesha, libcephfs, mon
- **Gates:** session_autoclose (300 s, per fs)
- **Map:** [rank-lifecycle-and-recovery.md › 3.14 Clientreplay (up:clientreplay)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#314-clientreplay-upclientreplay) · [rank-lifecycle-and-recovery.md › 3.12 Reconnect (up:reconnect): the client window](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#312-reconnect-upreconnect-the-client-window)
- **Code:** `src/mds/Server.cc:454-559 (Server::reclaim_session)`, `src/mds/MDSRank.cc:2107-2121 (MDSRank::maybe_clientreplay_done)`, `src/mds/Server.cc:1239-1290 (Server::find_idle_sessions)`, `src/client/Client.cc:18850-18858 (Client::set_session_timeout)`, `src/mds/MDSRank.cc:2075-2079 (MDSRank::rejoin_done)`
- **Notes:** Reclaim message semantics: [libcephfs-fuse-ganesha-expectations](../cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md). `rejoin_done` requests up:clientreplay instead of up:active while the reclaim gather is non-empty, and a close of such a session (by reclaim or kill) removes it from the gather. REQUEST_CLOSE waits for up:active, so a client closing sessions while a rank is held in up:clientreplay times out. A Ganesha restart that takes longer than session_autoclose loses the old session to eviction (blocklisted by default) rather than to reclaim. See also F-PROTO-063 (reclaim protocol), F-PROTO-046 (timeout key).

### Activation, post-recovery work and stopping

Work done when a rank becomes active (file size recovery, truncates, orphan purge, cap reissue) and the orderly stop of a rank.

<a id="f-clu-097"></a>

#### F-CLU-097 · Activation (recovery_done and active_start)

*The work a rank does when it enters `up:clientreplay` or `up:active`, before and as it starts serving new requests.*

`recovery_done` (from REJOIN) starts recovered truncates, orphan inode purges and file size recovery, then `populate_mydir`: fetch the mdsdir dirfrag and the header of each stray dirfrag, recreate missing `stray<i>` dentries via a journaled `EUpdate`, pin the strays, mark the cache open (all requests wait for it) and activate the stray manager and purge queue. `active_start` sets the active flag (QoS on), opens the root for CREATING/STARTING, registers the metrics handler (and the metric aggregator on rank 0), cleans per-segment open-file lists, handles leftover cap imports, wakes `waiting_for_replay`, reissues caps, wakes `waiting_for_active` (all requests and session closes that waited) and sets up the quiesce agent. From then on every map raises the OSD epoch barrier.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr
- **Legacy parts** (dead-code): The 'fragment was unreadable, recreating it now' branch of MDCache::populate_mydir. The 'fragment ... was unreadable, recreating it now' branch of populate_mydir requires an mdsdir dirfrag with fnode version 0 and STATE_BADFRAG. The only setter of STATE_BADFRAG in src/mds is CDir::go_bad, which with complete=true first sets a version-0 fnode to version 1, and Migrator::export_dir refuses system directories, so the state cannot arrive with an import either. Evidence: `src/mds/MDCache.cc:834-844 (MDCache::populate_mydir)`, `src/mds/CDir.cc:2333-2341 (CDir::go_bad)`, `src/mds/CDir.cc:2067-2094 (CDir::_omap_fetched)`, `src/mds/CDir.h:153-154 (CDir::MASK_STATE_EXPORTED)`, `src/mds/Migrator.cc:941-942 (Migrator::export_dir)`.
- **Map:** [rank-lifecycle-and-recovery.md › 3.15 Active (up:active) and recovery_done](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#315-active-upactive-and-recovery_done) · [resolve-and-rejoin.md › 3.17 After rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#317-after-rejoin)
- **Code:** `src/mds/MDSRank.cc:2129-2179 (MDSRank::active_start)`, `src/mds/MDCache.cc:821-905 (MDCache::populate_mydir)`
- **Notes:** Stray handling belongs to [strays-and-purge](../cpp-map/11-background-services/strays-and-purge.md). The "fragment was unreadable, recreating it now" branch of `populate_mydir` appears unreachable because `CDir::go_bad` never leaves version 0 (rank-lifecycle gotcha 31, derived from code). See also F-BG-021 (stray scan at activation).

<a id="f-clu-098"></a>

#### F-CLU-098 · Leftover reconnect caps and cap reissue at activation

*Caps whose inode was never found are answered with a stale EXPORT, and all caps are re-evaluated before serving.*

`export_remaining_imported_caps` sends, for each leftover `cap_imports` ino and client with a session, `MClientCaps(CEPH_CAP_OP_EXPORT, ino, realm 0, cap_id 0, mseq 0, osd_epoch_barrier)` with peer `{0, 0, 0, -1, 0}`, logs clog WARN "failed to reconnect caps for missing inodes:" and wakes cap-reconnect waiters. `reissue_all_caps` runs `Locker::eval` or `issue_caps` on every head inode with caps (frozen inodes deferred), which may grant or revoke caps with the current OSD epoch barrier.

- **Class:** POSIX `supporting` · exposure `client-wire`, `metrics` · relied on by kclient, libcephfs, admin, ceph-fuse, ganesha
- **Map:** [resolve-and-rejoin.md › 3.17 After rejoin](../cpp-map/09-recovery/resolve-and-rejoin.md#317-after-rejoin) · [rank-lifecycle-and-recovery.md › 3.15 Active (up:active) and recovery_done](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#315-active-upactive-and-recovery_done)
- **Code:** `src/mds/MDCache.cc:5873-5910 (MDCache::export_remaining_imported_caps)`, `src/mds/MDCache.cc:6351-6373 (MDCache::reissue_all_caps)`
- **Notes:** The stale EXPORT has `head.cap_id = 0`; both the kernel (`cap->cap_id != ex->cap_id`) and libcephfs ignore it, so clients keep caps for inodes the MDS no longer tracks and only the clog warning remains (resolve-and-rejoin section 8, derived from code).

<a id="f-clu-099"></a>

#### F-CLU-099 · File size and mtime recovery for writers that did not come back

*Recovery side of file size and mtime recovery for writers that did not reconnect.*

See F-CAPS-022. Recovery-specific: granted writeable ranges are journaled in inode_t::client_ranges. At the end of rejoin identify_files_to_recover puts every auth head regular file with a range whose client has no cap in rejoin_recover_q (filelock LOCK_PRE_SCAN) and every other one in rejoin_check_q; after activation start_files_to_recover moves the first set to LOCK_SCAN through file_recover (no rdlock, wrlock, xlock, lease or file caps, so stat/read/write block) and hands them to the RecoveryQueue.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk`, `metrics` · relied on by kclient, libcephfs, ceph-fuse, osd
- **Map:** [rank-lifecycle-and-recovery.md › 3.16 File size and mtime recovery (RecoveryQueue)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#316-file-size-and-mtime-recovery-recoveryqueue) · [resolve-and-rejoin.md › 3.13.3 Lock states, file recovery, acks](../cpp-map/09-recovery/resolve-and-rejoin.md#3133-lock-states-file-recovery-acks)
- **Code:** `src/mds/MDCache.cc:6444-6497 (MDCache::identify_files_to_recover)`, `src/mds/MDCache.cc:6542-6562 (MDCache::start_files_to_recover)`, `src/mds/RecoveryQueue.cc:31-238 (RecoveryQueue::advance)`, `src/mds/Locker.cc:6006-6036 (Locker::file_recover)`, `src/osdc/Filer.cc:185-399 (Filer::probe)`
- **Notes:** Possible auth-pin leak when an inode is re-enqueued while a probe restarts (rank-lifecycle section 4, derived from code). `identify_files_to_recover` stops marking caps clientwriteable at the first client without a cap (gotcha 15).

<a id="f-clu-100"></a>

#### F-CLU-100 · Re-executing interrupted truncates after recovery

*Recovery side of data-object truncation: truncates interrupted by a failover are re-run.*

See F-STORE-065. Recovery-specific: replay rebuilds per-segment truncating_inodes from EMetaBlob truncate_start/truncate_finish; start_recovered_truncates (from recovery_done) auth-pins each inode and re-runs _truncate_inode. Inodes owing a snapshot flush with Fb issued first move their filelock to LOCK_XLOCKSNAP. Recovered truncates do not re-send CEPH_CAP_OP_TRUNC; re-execution is idempotent at the OSD through truncate_seq.

- **Class:** POSIX `supporting` · exposure `on-disk` · relied on by osd, kclient, libcephfs
- **Gates:** filer_max_truncate_ops (128)
- **Map:** [rank-lifecycle-and-recovery.md › 3.17 Truncate execution and recovery](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#317-truncate-execution-and-recovery)
- **Code:** `src/mds/MDCache.cc:6847-6872 (MDCache::start_recovered_truncates)`, `src/mds/MDCache.cc:6640-6725 (MDCache::_truncate_inode)`, `src/osdc/Filer.cc:518-597 (Filer::truncate)`
- **Notes:** Truncate completions `ceph_assert(r == 0 || r == -ENOENT)` (rank-lifecycle gotcha 16). See also F-NS-062 (truncate request).

<a id="f-clu-101"></a>

#### F-CLU-101 · Purging orphaned preallocated and delegated inodes

*Recovery side of the purge of preallocated and delegated inode numbers whose session closed or was lost.*

See F-STORE-072. Recovery-specific: purges recorded by ESession events replayed before activation start from recovery_done (MDCache::start_purge_inodes); purges of sessions closed in clientreplay, active or stopping start immediately. At the end of the reconnect window each session that is evicted (not kept for reclaim) first has delegated_inos and free_prealloc_inos swapped, so after a restart (delegated_inos empty, free_prealloc_inos = prealloc_inos) every unused preallocated number of a client that did not come back is purged rather than released.

- **Class:** POSIX `supporting` · exposure `on-disk` · relied on by osd, kclient
- **Map:** [rank-lifecycle-and-recovery.md › 3.18 Purging orphaned preallocated inodes](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#318-purging-orphaned-preallocated-inodes)
- **Code:** `src/mds/MDCache.cc:6893-6928 (MDCache::purge_inodes)`, `src/mds/Server.cc:1524-1561 (Server::journal_close_session)`, `src/mds/Server.cc:935-942 (Server::_session_logged)`, `src/mds/Server.cc:1918-1919 (Server::reconnect_tick)`, `src/mds/MDSRank.cc:2175 (MDSRank::recovery_done)`
- **Notes:** Only object 0 is removed (code FIXME "handle non-default data pool and namespace"), so larger orphaned async-created files leak data objects (rank-lifecycle gotcha 12).

<a id="f-clu-102"></a>

#### F-CLU-102 · Stopping a rank (up:stopping to down:stopped)

*The orderly drain a rank performs when the monitor shrinks the file system.*

`stopping_start` disables QoS and calls `shutdown_start`. While stopping, client requests are still processed and forwarded as subtrees leave; new session opens are dropped silently. Each 5 s tick runs `shutdown_pass` until all hold: non-zero ranks export strays to rank 0 (at most 100 in flight) and every auth non-mdsdir subtree to its parent's authority (rank 0 if that is not active), ignoring pins when `max_mds == 0`; the cache is trimmed; remaining sessions get `terminate_sessions` (`CEPH_SESSION_CLOSE`); the journal is trimmed to one segment and capped with an `ELid`; no Objecter op is in flight. Then the purge queue is drained (clog "waiting for purge queue" each tick) and `stopping_done` requests `down:stopped`. The capped journal, tables and mdsdir stay in RADOS for a later `up:starting`.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `client-wire`, `on-disk` · relied on by mon, mgr, admin, cephadm, kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [rank-lifecycle-and-recovery.md › 3.19 Stopping (up:stopping → down:stopped)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#319-stopping-upstopping--downstopped) · [beacon-and-monitor.md › 3.11 Voluntary exits and special states](../cpp-map/01-cluster-integration/beacon-and-monitor.md#311-voluntary-exits-and-special-states)
- **Code:** `src/mds/MDSRank.cc:2252-2292 (MDSRank::stopping_start)`, `src/mds/MDCache.cc:8043-8234 (MDCache::shutdown_pass)`, `src/mds/MDSRank.cc:773-798 (MDSRankDispatcher::tick)`, `src/mds/PurgeQueue.cc:838-880 (PurgeQueue::drain)`
- **Notes:** If the stopping holder dies after capping its journal, the replacement replays only the `ELid`, finds no subtrees and completes the stop through `REJOIN -> STOPPED` (rank 0 with more than one replayed event is marked damaged instead) (rank-lifecycle 3.13, derived from code). See also F-PROTO-054 (session termination and last-rank eviction), F-BG-022 (stray export), F-BG-023 (purge queue drain), F-MULTI-049 (subtree hand-off), F-STORE-021 (journal capping).

### Failure exits

How the C++ MDS leaves when it must not continue: DAMAGED reports, respawn, suicide and write-error policy.

<a id="f-clu-104"></a>

#### F-CLU-104 · Reporting a rank as DAMAGED

*When a rank's metadata cannot be loaded or replayed, the MDS reports DAMAGED so no daemon is assigned to it until an operator repairs it.*

`MDSRank::damaged()` sets `want_state = down:damaged`, flushes the cluster log, refreshes health, sends a beacon and waits up to 0.95 x `mds_mon_shutdown_timeout` (5 s) for the ack **while holding `mds_lock`**, then respawns even without an ack. Triggers: table, sessionmap, journal pointer or header read/decode errors; `boot_start` `-EINVAL`/`-ENOENT`; `validate_sessions` inconsistency; corrupt or unbounded journal events or a missing journal object; unhandled journal write errors; rank 0 with more than one journal event but no subtrees after rejoin; failure to open the root or mdsdir inode/dirfrag or a stray dirfrag; a file size probe OSD error; `mds_standby_replay_damaged` (dev). The monitor side is described under "Damaged ranks and mds repaired".

- **Class:** POSIX `operational` · exposure `cluster-wire`, `metrics` · relied on by mon, admin, tools, kclient, libcephfs
- **Gates:** mds_mon_shutdown_timeout (5 s)
- **Map:** [rank-lifecycle-and-recovery.md › 3.20 Failure exits: damaged, respawn, suicide](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#320-failure-exits-damaged-respawn-suicide) · [beacon-and-monitor.md › 3.11 Voluntary exits and special states](../cpp-map/01-cluster-integration/beacon-and-monitor.md#311-voluntary-exits-and-special-states) · [process-and-threading.md › 3.16 Shutdown, suicide, respawn, damaged](../cpp-map/00-architecture/process-and-threading.md#316-shutdown-suicide-respawn-damaged)
- **Code:** `src/mds/MDSRank.cc:945-960 (MDSRank::damaged)`, `src/mds/MDSRank.cc:1630-1635 (MDSRank::boot_start)`, `src/mds/MDCache.cc:668-683 (C_MDS_RetryOpenRoot)`, `src/mds/MDSRank.cc:2061-2067 (MDSRank::rejoin_done)`
- **Notes:** If `damaged()` runs on the dispatch thread, or while it is blocked on `mds_lock`, the beacon ack cannot be processed and the wait times out; the code comment accepts that the monitor may miss the report (process-and-threading section 8). See also F-STORE-040 (replay errors), F-BG-057 (fatal damage), F-CLU-049 (monitor side).

<a id="f-clu-105"></a>

#### F-CLU-105 · Respawn: re-exec as a fresh standby

*The C++ MDS discards its identity by re-executing itself whenever it may no longer act for its rank.*

`MDSDaemon::respawn` dumps the recent in-memory log and `execv`s `/proc/self/exe` with the original arguments (same PID; `_exit(0)` with `mds_valgrind_exit`); the new process gets a new messenger nonce, mon session and gid and boots as a standby; its boot beacon fails the old gid through unique-name enforcement. Triggers: removed from the MDSMap (failed by the monitor, or after `down:stopped`); a different rank or an invalid transition in a map; any `MDSIOContextBase` completion with `-EBLOCKLISTED` or `-ETIMEDOUT`; journal write `-EBLOCKLISTED`; standby-replay `-EAGAIN` or falling behind `trimmed_pos`; `damaged()`; asok/tell `respawn` (after 1 s).

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, osd, admin, cephadm, rook
- **Gates:** mds_valgrind_exit (dev); rados_osd_op_timeout (0: no -ETIMEDOUT)
- **Map:** [rank-lifecycle-and-recovery.md › 3.20 Failure exits: damaged, respawn, suicide](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#320-failure-exits-damaged-respawn-suicide) · [process-and-threading.md › 3.16 Shutdown, suicide, respawn, damaged](../cpp-map/00-architecture/process-and-threading.md#316-shutdown-suicide-respawn-damaged) · [beacon-and-monitor.md › 3.11 Voluntary exits and special states](../cpp-map/01-cluster-integration/beacon-and-monitor.md#311-voluntary-exits-and-special-states)
- **Code:** `src/mds/MDSDaemon.cc:1021-1084 (MDSDaemon::respawn)`, `src/mds/MDSDaemon.cc:883-888 (MDSDaemon::handle_mds_map)`, `src/mds/MDSContext.cc:95-120 (MDSIOContextBase::complete)`, `src/mds/MDLog.cc:119-149 (C_MDL_WriteError)`
- **Notes:** Because respawn keeps the PID, a supervisor (systemd, cephadm, Rook) does not observe a process exit (process-and-threading section 7, inferred from exec semantics). `MDLog::_recovery_thread` calls `respawn()` on `-EBLOCKLISTED` without holding `mds_lock`. See also F-STORE-095 (fenced respawn path), F-ADM-033 (respawn command).

<a id="f-clu-106"></a>

#### F-CLU-106 · Suicide and orderly shutdown

*Process exit paths: signals, asok `exit`, fatal boot errors and unwritable compat.*

`MDSDaemon::suicide` (must hold `mds_lock`): set `stopping`, cancel the tick, unregister asok commands, send the DNE beacon (lock released, up to ~1 s), stop the beacon thread and MgrClient, then `MDSRankDispatcher::shutdown` (timer, MDLog threads, purge queue, metrics, finisher stopped without flushing pending contexts, Objecter, MonClient, progress thread, messenger) or, without a rank, timer/MonClient/messenger; `main` returns from `msgr->wait()` and deletes the daemon only on a clean shutdown (no rank, or rank in `stopped`). Triggers: SIGINT/SIGTERM, asok/tell `exit` (after 1 s), MonClient init/auth/rotating-key failure, an MDSMap compat the daemon cannot write, an unexpected `boot_start` error (including `-EAGAIN` outside standby-replay).

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by mon, cephadm, rook, admin
- **Map:** [process-and-threading.md › 3.16 Shutdown, suicide, respawn, damaged](../cpp-map/00-architecture/process-and-threading.md#316-shutdown-suicide-respawn-damaged) · [rank-lifecycle-and-recovery.md › 3.20 Failure exits: damaged, respawn, suicide](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#320-failure-exits-damaged-respawn-suicide)
- **Code:** `src/mds/MDSDaemon.cc:959-1019 (MDSDaemon::suicide)`, `src/mds/MDSRank.cc:804-872 (MDSRankDispatcher::shutdown)`, `src/ceph_mds.cc:241-255 (main)`, `src/mds/MDSRank.cc:1638-1642 (MDSRank::boot_start)`
- **Notes:** Possible shutdown deadlocks by code reading: `suicide()` unregisters asok hooks while holding `mds_lock` while hooks block on it; shutdown joins `mds-metrics`/`mds-ping` under `mds_lock`; `PurgeQueue::shutdown` joins its finisher under its lock. Between `MDSDaemon::stopping` and `MDSRank::stopping` (about 1 s) IO completions and timers still mutate state (process-and-threading section 8).

<a id="f-clu-107"></a>

#### F-CLU-107 · Write error policy

*Failure exits triggered by RADOS write errors.*

See F-STORE-100 (metadata write errors: ignore, read-only or respawn per mds_action_on_write_error) and F-STORE-015 (journal write errors: damaged). Failure-exit view: -EBLOCKLISTED always respawns (F-CLU-105); mds_action_on_write_error 2 or more respawns (the log says "suicide"); journal write errors other than -EBLOCKLISTED mark the rank damaged (F-CLU-104).

- **Class:** POSIX `operational` · exposure `config`, `metrics` · relied on by admin, osd, kclient, libcephfs
- **Gates:** mds_action_on_write_error (1)
- **Map:** [rank-lifecycle-and-recovery.md › 3.20 Failure exits: damaged, respawn, suicide](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#320-failure-exits-damaged-respawn-suicide) · [process-and-threading.md › 3.16 Shutdown, suicide, respawn, damaged](../cpp-map/00-architecture/process-and-threading.md#316-shutdown-suicide-respawn-damaged)
- **Code:** `src/mds/MDSRank.cc:973-991 (MDSRank::handle_write_error)`, `src/mds/MDLog.cc:119-149 (C_MDL_WriteError)`

### Daemon process, threads and dispatch

The ceph-mds process: messenger, MonClient, threads, the global lock, completion contexts, dispatch and periodic upkeep.

<a id="f-clu-108"></a>

#### F-CLU-108 · Process bootstrap (ceph-mds -i <id>)

*How one `ceph-mds` process starts: arguments, identity, messenger, MonClient and the main wait.*

`main`: `global_init` as an MDS daemon; optional CPU pinning to `mds_numa_node`; only the obsolete `--hot-standby <x>` extra argument is accepted; the id must be given and must not be empty or start with a digit; `pick_addresses(PUBLIC)`; optional fork/daemonize; one messenger `mds.-1` with a random nonce, cluster protocol 36; a 2-thread asio pool; `MonClient`; `MDSDaemon::init`; one-shot SIGINT/SIGTERM handlers (SIGHUP reopens logs); the main thread blocks in `msgr->wait()` until shutdown. `MDSDaemon::init` registers dispatchers, authenticates, waits for rotating keys (`rotating_keys_bootstrap_timeout` 30 s, up to `max_rotating_auth_attempts` 10, else suicide `-ETIMEDOUT`), subscribes `mdsmap` from epoch 0, sets up the admin socket, starts the timer and the beacon (`want_state = up:boot`) and the 5 s tick.

- **Class:** POSIX `operational` · exposure `config`, `cluster-wire` · relied on by cephadm, rook, tools, admin
- **Legacy parts** (deprecated-admin, msgr1): The obsolete `--hot-standby` argument; cluster protocol 36, which only msgr1 handshakes carry. main() accepts `--hot-standby <x>` only to log '--hot-standby is obsolete and has no effect', and cephadm, Rook and the mgr modules never pass it (grep). The cluster protocol 36 set with set_cluster_protocol is read only by AsyncMessenger::get_proto_version, which only ProtocolV1 (msgr1) calls; msgr2 connections carry no protocol number. Evidence: `src/ceph_mds.cc:126-127 (main)`, `src/ceph_mds.cc:177 (main)`, `src/msg/async/AsyncMessenger.cc:1099-1116 (AsyncMessenger::get_proto_version)`, `src/msg/async/ProtocolV1.cc:1540-1541 (ProtocolV1::send_connect_message)`, `src/msg/async/ProtocolV1.cc:2005-2006 (ProtocolV1::handle_connect_message_auth)`.
- **Gates:** mds_numa_node (-1); rotating_keys_bootstrap_timeout (30 s); max_rotating_auth_attempts (10)
- **Map:** [process-and-threading.md › 3.1 Process start (main)](../cpp-map/00-architecture/process-and-threading.md#31-process-start-main) · [process-and-threading.md › 3.2 MDSDaemon::init](../cpp-map/00-architecture/process-and-threading.md#32-mdsdaemoninit) · [beacon-and-monitor.md › 3.2 Daemon start until the first beacon](../cpp-map/01-cluster-integration/beacon-and-monitor.md#32-daemon-start-until-the-first-beacon)
- **Code:** `src/ceph_mds.cc:84-265 (main)`, `src/mds/MDSDaemon.cc:628-743 (MDSDaemon::init)`

<a id="f-clu-109"></a>

#### F-CLU-109 · Single public messenger with per-peer-type policies

*One messenger and one address vector serve clients, monitors, OSDs, mgr and peer MDSs.*

The MDS creates one async messenger (`ms_public_type` or `ms_type`), so the address vector the monitor records from beacons is the one clients and peers connect to. Policies: default (OSD, mgr) `lossy_client` requiring OSDREPLYMUX; MON `lossy_client` requiring UID|PGID64; MDS `lossless_peer` requiring UID; CLIENT `stateful_server` (lossless, server, standby, reset check) requiring nothing. No byte or message throttlers are attached to policies; inbound backpressure is the global `ms_dispatch_throttle_bytes` (100 MiB). msgr v1 handshakes use protocol 32 for clients and 36 between MDSs.

- **Class:** POSIX `supporting` · exposure `client-wire`, `cluster-wire`, `mds-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, mgr, osd, mds-ranks
- **Legacy parts** (msgr1): msgr v1 handshake protocol numbers (32 for clients, 36 between MDSs). The protocol numbers 32 and 36 are exchanged only in msgr1 connect and connect-reply messages, and the 7.3 kernel sends CEPH_MDSC_PROTOCOL only from messenger_v1.c. Baseline clients use msgr2: the kernel through the ms_mode=prefer-crc that mount.ceph adds, libcephfs by connecting to the first usable address of the vector, which daemons list as v2 before v1. The per-peer-type policies and required features are also enforced on msgr2 connections. Evidence: `src/msg/async/AsyncMessenger.cc:1099-1116 (AsyncMessenger::get_proto_version)`, `src/msg/async/ProtocolV1.cc:1540-1541 (ProtocolV1::send_connect_message)`, `src/msg/async/ProtocolV1.cc:2005-2006 (ProtocolV1::handle_connect_message_auth)`, `linux/net/ceph/messenger_v1.c:386-400 (prepare_write_connect)`, `src/mount/mount.ceph.c:306-309 (parse_new_dev)`, `src/msg/async/AsyncMessenger.cc:847-858 (AsyncMessenger::create_connect)`, `src/common/pick_address.cc:473-486 (pick_addresses)`, `src/msg/async/ProtocolV2.cc:2462-2467 (ProtocolV2::handle_client_ident)`.
- **Gates:** ms_type / ms_public_type; ms_dispatch_throttle_bytes (100 MiB); ms_async_op_threads (3)
- **Map:** [process-and-threading.md › 3.1 Process start (main)](../cpp-map/00-architecture/process-and-threading.md#31-process-start-main) · [process-and-threading.md › 7. Externally visible contract](../cpp-map/00-architecture/process-and-threading.md#7-externally-visible-contract)
- **Code:** `src/ceph_mds.cc:171-191 (main)`, `src/msg/async/ProtocolV2.cc:1662 (ProtocolV2::throttle_dispatch_queue)`
- **Notes:** Transport and authentication as clients see them: [transport-auth-and-client-library-contract](../cpp-map/00-architecture/transport-auth-and-client-library-contract.md) and [wire-protocol](../cpp-map/02-client-protocol/wire-protocol.md). See also F-PROTO-006 (client policy), F-PROTO-107 (peer-MDS policy), F-PROTO-008 (entity name).

<a id="f-clu-110"></a>

#### F-CLU-110 · MonClient session, keys and subscriptions

*The MDS's monitor session: authentication, service keys and the maps it subscribes to.*

The MonClient wants service keys for MON, OSD, MDS and MGR and acts as the messenger's auth client and server (cephx verification of clients happens on messenger threads). Subscriptions: `monmap` and `config` (from `authenticate`, so `mds_join_fs` and other options can come from the mon config database), `mdsmap` from epoch 0 (`MDSDaemon::init`), `mgrmap` when the daemon first appears in an FSMap (starting the MgrClient), `osdmap` through the Objecter (plus `MMonGetVersion` for `wait_for_latest_osdmap`). Mon caps needed (all in `profile mds`): `mds` all, `mon r`, `osd r`, `osd pool rmsnap`, `osd blocklist`, `log w`, config-key under `daemon-private/mds.<id>/`. A `MSG_MON_COMMAND` from a monitor is dropped with a clog warning.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `config` · relied on by mon, mgr, cephadm, rook
- **Legacy parts** (old-daemons): Discarding MSG_MON_COMMAND (MMonCommand) from pre-Luminous monitors. MDSDaemon::handle_core_message discards a MSG_MON_COMMAND from a monitor with the clog warning 'dropping `mds tell` command from legacy monitor'. Monitors up to v11.x forwarded `mds tell` to MDS daemons as MMonCommand; the MDS-side handling was removed in v12.0.2 (commit 2be6297d12e). No monitor code at this commit sends MMonCommand to an MDS: the only constructions are MonClient requests to monitors and an OSDMonitor-internal retry. Evidence: `src/mds/MDSDaemon.cc:1160-1163 (MDSDaemon::handle_core_message)`, `v11.2.0:src/mon/MDSMonitor.cc:1112-1125 (MDSMonitor::preprocess_command, mds tell)`, `src/mon/MonClient.cc:1365 (MonClient::_send_command)`, `src/mon/OSDMonitor.cc:4136 (OSDMonitor::prepare_pg_ready_to_merge)`.
- **Map:** [beacon-and-monitor.md › 3.15 Every other message or command the MDS sends to the ...](../cpp-map/01-cluster-integration/beacon-and-monitor.md#315-every-other-message-or-command-the-mds-sends-to-the-mons) · [beacon-and-monitor.md › 3.2 Daemon start until the first beacon](../cpp-map/01-cluster-integration/beacon-and-monitor.md#32-daemon-start-until-the-first-beacon)
- **Code:** `src/mds/MDSDaemon.cc:669-723 (MDSDaemon::init)`, `src/mon/MonClient.cc:638-639 (MonClient::authenticate)`, `src/mds/MDSDaemon.cc:890-896 (MDSDaemon::handle_mds_map)`, `src/mon/MonCap.cc:201-210 (MonCapGrant::expand_profile)`
- **Notes:** See also F-PROTO-103 (MonClient protocol duties).

<a id="f-clu-111"></a>

#### F-CLU-111 · Global mds_lock (FIFO fair mutex)

*The single process-wide lock that serialises every metadata and rank-state change in the C++ MDS.*

`mds_lock` is a `ceph::fair_mutex` (FIFO ticket lock: strict arrival-order hand-off, every unlock wakes all waiters, not recursive, no lockdep). It is taken by the dispatch thread for every message reaching `MDSDaemon`, by the timer thread for every callback, by the rank finisher for IO completions, by the progress thread, the log-trim, cache-trim and log-replay threads, admin commands, signal handling and several subsystem callbacks. Rule: whoever takes it must check `stopping` and either do nothing or never release it. Lock-held assertions are compiled only in debug builds. Details: [mds_lock-analysis](../cpp-map/00-architecture/mds_lock-analysis.md).

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [process-and-threading.md › 3.6 mds_lock](../cpp-map/00-architecture/process-and-threading.md#36-mds_lock) · [process-and-threading.md › 6.1 What runs under mds_lock](../cpp-map/00-architecture/process-and-threading.md#61-what-runs-under-mds_lock) · [process-and-threading.md › 6.3 Serialization points](../cpp-map/00-architecture/process-and-threading.md#63-serialization-points)
- **Code:** `src/mds/MDSDaemon.h:70-75 (MDSDaemon)`, `src/common/fair_mutex.h:15 (fair_mutex)`, `src/mds/MDSDaemon.cc:1088-1114 (MDSDaemon::ms_dispatch2)`

<a id="f-clu-112"></a>

#### F-CLU-112 · Completion contexts and the rank finisher

*How asynchronous IO and journal completions re-enter the locked world in the C++ MDS.*

`MDSContext` completions assert `mds_lock` and reset the heartbeat; `MDSIOContextBase::complete` takes `mds_lock` itself, drops the completion if the rank is stopping, respawns on `-EBLOCKLISTED`/`-ETIMEDOUT`, and is tracked in a global list used for `MDS_SLOW_METADATA_IO`; `MDSLogContextBase` also advances the journal safe position; `C_IO_Wrapper` re-queues on the finisher; `MDSGather` fans in. Objecter callbacks are wrapped in `C_OnFinisher` so they run one at a time on the `mds-rank-fin` thread; Journaler callbacks likewise, so journal commits complete in order.

- **Class:** POSIX `internal` · exposure `internal` · relied on by osd, internal
- **Map:** [process-and-threading.md › 3.7 Contexts: how asynchronous work re-enters mds_lock](../cpp-map/00-architecture/process-and-threading.md#37-contexts-how-asynchronous-work-re-enters-mds_lock)
- **Code:** `src/mds/MDSContext.h:38-212 (MDSContext)`, `src/mds/MDSContext.cc:95-147 (MDSIOContextBase::complete)`, `src/osdc/Journaler.cc:1620 (Journaler::wrap_finisher)`
- **Notes:** `queue_waiters` completes every context with 0, losing the waker's error code; once stopping, `MDSIOContextBase::complete` silently leaks contexts (process-and-threading section 8). See also F-STORE-011 (ordered journal completions).

<a id="f-clu-113"></a>

#### F-CLU-113 · Progress thread and rank waiter queues

*Per-rank queues where messages and requests park until a state, peer, map epoch or lagginess condition changes.*

Queues under `mds_lock`: `finished_queue` (contexts drained by `_advance_queues` at the end of each top-level dispatch or by the `mds-rank-progr` thread), `waiting_for_nolaggy` (deferred messages), `waiting_for_active`, `waiting_for_replay`, `waiting_for_resolve/_reconnect/_rejoin`, `waiting_for_active_peer[rank]` (`MDS_RANK_NONE` = cluster recovered), `waiting_for_bootstrapping_peer[rank]`, `waiting_for_mdsmap[epoch]`, `replay_queue`. Parked messages are re-dispatched with `C_MDS_RetryMessage` and re-run all checks, so per-connection processing order is not strict after parking.

- **Class:** POSIX `internal` · exposure `internal` · relied on by kclient, libcephfs, internal
- **Map:** [process-and-threading.md › 3.8 Queues and waiters owned by MDSRank](../cpp-map/00-architecture/process-and-threading.md#38-queues-and-waiters-owned-by-mdsrank) · [rank-lifecycle-and-recovery.md › 4. Data structures and invariants](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#4-data-structures-and-invariants)
- **Code:** `src/mds/MDSRank.cc:999-1035 (ProgressThread::entry)`, `src/mds/MDSRank.cc:1337-1352 (MDSRank::_advance_queues)`, `src/mds/MDSRank.h:251-270 (MDSRank::queue_waiter)`

<a id="f-clu-114"></a>

#### F-CLU-114 · Dispatcher chain, message priorities and ordering

*The order in which incoming messages reach the MDS and which ones bypass `mds_lock`.*

After decoding, OSD replies and watch-notify are fast-dispatched into the Objecter on messenger threads; everything else goes to one `ms_dispatch` thread through a priority queue (priority >= 64 strict, round-robin per connection within a priority). Dispatcher order: MonClient, Beacon (acks), MetricsHandler (client metrics, pings; after activation), MgrClient, Objecter (OSD maps acknowledged and passed on), MetricAggregator (rank 0), then `MDSDaemon` (takes `mds_lock`). Clients send at priority 127 (libcephfs metrics at 195); connection events are queued at 255 and overtake messages already queued for that connection. Debug hooks (`mds_thrash_exports`, `mds_thrash_fragments`) and `update_mlogger` run at the end of each top-level dispatch.

- **Class:** POSIX `internal` · exposure `internal` · relied on by kclient, libcephfs, mon, osd, mgr, internal
- **Gates:** mds_thrash_exports / mds_thrash_fragments (dev); ms_die_on_unhandled_msg (false)
- **Map:** [process-and-threading.md › 3.9 Message dispatch path](../cpp-map/00-architecture/process-and-threading.md#39-message-dispatch-path) · [process-and-threading.md › 7. Externally visible contract](../cpp-map/00-architecture/process-and-threading.md#7-externally-visible-contract)
- **Code:** `src/msg/DispatchQueue.cc:83 (DispatchQueue::enqueue)`, `src/mds/MDSDaemon.cc:661-664 (MDSDaemon::init)`, `src/msg/DispatchQueue.h:157-186 (DispatchQueue::queue_accept)`, `src/mds/MDSRank.cc:1058-1170 (MDSRank::_dispatch)`

<a id="f-clu-115"></a>

#### F-CLU-115 · Rank message validation and routing

*Which message types a rank accepts, from which peer types, and which subsystem handles them.*

`MDSDaemon::ms_dispatch2` handles core messages itself (MDSMap, OSDMap, `MRemoveSnaps`, `MCommand`, MonMap; legacy `MMonCommand` dropped) and filters by peer type. `MDSRankDispatcher::ms_dispatch` asserts that any message from an MDS is an `MMDSOp` and updates the client session's `last_seen`. `_dispatch` runs quiesce-DB messages first, drops stale peer messages, accepts only a whitelist (ports 0x2xx cache, 0x4xx migrator, client session/reconnect/reclaim/request/reply, peer request, heartbeat, table request, lock, inode file caps, scrub, client caps/caprelease/lease) and routes them to MDCache, Migrator, Server, MDBalancer, table client/server, Locker or ScrubStack. Rank-state gating is done by each subsystem, not centrally.

- **Class:** POSIX `internal` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, mon, mds-ranks, internal
- **Legacy parts** (old-daemons): Discarding MSG_MON_COMMAND (MMonCommand) from pre-Luminous monitors. MDSDaemon::handle_core_message discards MSG_MON_COMMAND from monitors with a clog warning; only monitors up to v11.x sent it (forwarded `mds tell`), and no monitor code at this commit sends MMonCommand to an MDS. MRemoveSnaps, MCommand, MDSMap, OSDMap and MonMap handling are current (the current monitor replies MRemoveSnaps to SERVER_OCTOPUS peers). Evidence: `src/mds/MDSDaemon.cc:1160-1163 (MDSDaemon::handle_core_message)`, `v11.2.0:src/mon/MDSMonitor.cc:1112-1125 (MDSMonitor::preprocess_command, mds tell)`, `src/mon/OSDMonitor.cc:4426-4430 (OSDMonitor::preprocess_remove_snaps)`.
- **Map:** [process-and-threading.md › 3.9 Message dispatch path](../cpp-map/00-architecture/process-and-threading.md#39-message-dispatch-path)
- **Code:** `src/mds/MDSDaemon.cc:1130-1169 (MDSDaemon::handle_core_message)`, `src/mds/MDSRank.cc:1037-1056 (MDSRankDispatcher::ms_dispatch)`, `src/mds/MDSRank.cc:1191-1231 (MDSRank::is_valid_message)`
- **Notes:** `MSG_REMOVE_SNAPS` dereferences `mds_rank` without a null check; any non-`MMDSOp` message from an MDS peer that reaches rank dispatch aborts the daemon (process-and-threading section 8). See also F-ADM-004 (security view), F-PROTO-017 (client message catalog).

<a id="f-clu-116"></a>

#### F-CLU-116 · Outbound messaging helpers and local delivery

*The send paths a rank uses for clients and peers, including counted pushes and self-addressed messages.*

`send_message_mds(m, rank)` (drop if not up, defer to bootstrapping peers, piggyback a newer MDSMap) and an address-based variant without checks; `forward_message_mds` never forwards a client request between MDSs but sends the client `MClientRequestForward(dest, num_fwd+1, client_must_resend=true)`; `send_message_client_counted` increments the session `cap_push_seq` (a client close is honoured only if its seq matches) and queues on `preopen_out_queue` when there is no connection. Messages to self go through the messenger's local delivery thread and are processed later on `ms_dispatch`.

- **Class:** POSIX `internal` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks, internal
- **Map:** [process-and-threading.md › 3.11 Outbound messaging helpers (MDSRank.cc:1453-1571)](../cpp-map/00-architecture/process-and-threading.md#311-outbound-messaging-helpers-mdsrankcc1453-1571) · [process-and-threading.md › 7. Externally visible contract](../cpp-map/00-architecture/process-and-threading.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSRank.cc:1453-1571 (MDSRank::send_message_mds)`, `src/msg/DispatchQueue.cc:101-137 (DispatchQueue::local_delivery)`
- **Notes:** Request forwarding and push-sequence semantics as clients see them belong to [request-lifecycle](../cpp-map/03-request-path/request-lifecycle.md) and [sessions](../cpp-map/02-client-protocol/sessions.md).

<a id="f-clu-117"></a>

#### F-CLU-117 · Tick and the shared timer

*The periodic upkeep of a rank and the timer that runs all periodic callbacks under `mds_lock`.*

`MDSDaemon::tick` runs every `mds_tick_interval` (5 s) on the shared `SafeTimer`, which holds `mds_lock` while running every callback. `MDSRankDispatcher::tick`: heartbeat reset; return if laggy; slow-op scan; wake the progress thread; flush the journal; in clientreplay/active/stopping clear laggy clients, find idle sessions, evict cap-revoke non-responders and run `Locker::tick`; in reconnect `reconnect_tick`; in active the balancer, stale freeze detection, snap server OSD check and scrub status (rank 0); in active/stopping `update_targets`; in stopping `shutdown_pass` and the purge-queue drain; finally `notify_health`. Other timer users: task status (2 s), standby-replay restart (1 s), retry timers of several subsystems.

- **Class:** POSIX `internal` · exposure `config` · relied on by mon, kclient, libcephfs, internal
- **Gates:** mds_tick_interval (5 s)
- **Map:** [process-and-threading.md › 3.12 Tick, timers, heartbeat and lagginess](../cpp-map/00-architecture/process-and-threading.md#312-tick-timers-heartbeat-and-lagginess) · [rank-lifecycle-and-recovery.md › 3.22 Periodic work by state (MDSRankDispatcher::tick)](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md#322-periodic-work-by-state-mdsrankdispatchertick)
- **Code:** `src/mds/MDSDaemon.cc:745-768 (MDSDaemon::tick)`, `src/mds/MDSRank.cc:710-802 (MDSRankDispatcher::tick)`, `src/common/Timer.cc:80 (CommonSafeTimer::timer_thread)`

<a id="f-clu-118"></a>

#### F-CLU-118 · Thread inventory of the daemon and its subsystems

*The threads one C++ MDS process runs and which of them take `mds_lock`.*

Without `mds_lock`: `msgr-worker-N` (3), `ms_local`, `io_context_pool` (2), MonClient/MgrClient/Objecter timers, `mds-beacon`, `log`, `service` (touches `heartbeat_file` when healthy), `mds-log-submit`, `mds-pq-fin` and the PurgeQueue timer, `mds-ping`. With `mds_lock`: `ms_dispatch`, `safe_timer`, `admin_socket` (most commands), `signal_handler`, `mds-rank-fin`, `mds-rank-progr`, `mds-log-trim` (every 1 s while active/stopping), `mds-cache-trim` (every 1 s, heap release every 10 s), `mds-log-replay` (per event), `mds-log-recvr`/`-reopen` (briefly), `mds-metrics`, `mds-q-db`, `mds-q-agt`, dmClock threads. Process-global object counters and the IO context list make "one rank per process" an implicit assumption.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Gates:** mds_cache_trim_interval (1 s); mds_log_trim_upkeep_interval (1000 ms)
- **Map:** [process-and-threading.md › 3.5 Thread inventory](../cpp-map/00-architecture/process-and-threading.md#35-thread-inventory) · [process-and-threading.md › 3.3 Object model and ownership](../cpp-map/00-architecture/process-and-threading.md#33-object-model-and-ownership)
- **Code:** `src/mds/MDSRank.cc:468-609 (MDSRank::MDSRank)`, `src/mds/MDLog.cc:64 (MDLog::MDLog)`, `src/mds/MDCache.cc:212 (MDCache::MDCache)`

<a id="f-clu-119"></a>

#### F-CLU-119 · MDS-originated client requests (MDSMetaRequest)

*Internal operations a rank performs by sending `MClientRequest`s to another rank.*

`issue_tid()` hands out per-process tids for internal requests (`reqid = mds.<rank>:<tid>`). Stray reintegration and migration send `MClientRequest(CEPH_MDS_OP_RENAME)` to another MDS and remember it in `internal_client_requests[tid]` (pinning the dentry with `PIN_PURGING`); the peer's safe `MClientReply` erases the entry. `Server::create_quota_realm` sends a fire-and-forget `SETXATTR ceph.quota` to the auth rank; its reply only logs "no pending request".

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks, internal
- **Map:** [process-and-threading.md › 3.18 MDS-originated requests (MDSMetaRequest)](../cpp-map/00-architecture/process-and-threading.md#318-mds-originated-requests-mdsmetarequest)
- **Code:** `src/mds/MDSMetaRequest.h:21-32 (MDSMetaRequest)`, `src/mds/StrayManager.cc:679-738 (StrayManager::reintegrate_stray)`, `src/mds/Server.cc:6262-6273 (Server::create_quota_realm)`, `src/mds/Server.cc:2730 (Server::handle_client_reply)`

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-clu-059"></a>F-CLU-059 | [F-PROTO-008](02-client-sessions-and-protocol.md#f-proto-008) · MDS entity name and rank identity on the wire |
| <a id="f-clu-061"></a>F-CLU-061 | [F-MULTI-019](07-multi-mds.md#f-multi-019) · Export-target publication (MMDSLoadTargets, MDSMap export_targets) |
| <a id="f-clu-081"></a>F-CLU-081 | [F-PROTO-057](02-client-sessions-and-protocol.md#f-proto-057) · Reconnect processing and outcomes |
| <a id="f-clu-083"></a>F-CLU-083 | [F-NS-084](04-namespace-and-posix-operations.md#f-ns-084) · Lock reinstatement after an MDS restart |
| <a id="f-clu-103"></a>F-CLU-103 | [F-PROTO-054](02-client-sessions-and-protocol.md#f-proto-054) · Session termination at rank shutdown |

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
