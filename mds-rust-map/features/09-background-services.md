# Background services: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [strays-and-purge.md](../cpp-map/11-background-services/strays-and-purge.md) · [scrub-and-damage.md](../cpp-map/11-background-services/scrub-and-damage.md)

## 1. Scope

This category covers the C++ MDS background services that run beside the request path: deferred deletion through per-rank stray directories, the purge decision and its triggers, hard-link reintegration and stray migration, the RADOS-journaled PurgeQueue that deletes file and directory objects, and the stray and purge work done at activation and rank stop. The purge of delegated and preallocated inode numbers on session close does not use the PurgeQueue and is catalogued in 06-journal-and-storage. Sources are the fact-checked map pages strays-and-purge.md and scrub-and-damage.md.

It also covers forward scrub (ScrubStack: commands, tags, traversal, checks, repairs, data uninlining, multi-rank forwarding and completion) and the DamageTable (where damage is detected, fatal damage, damage ls/rm, and the EIO and readdir behaviour it causes; the MDS_DAMAGE health check is catalogued in 11-security-admin-and-observability).

Excluded and owned elsewhere: the unlink, rmdir, rename, LOOKUPINO and setattr-truncate request handlers (04-namespace-and-posix-operations); cap issue mechanics and delegated inode numbers (03-caps-leases-and-locking); past-parent snapshot recording and rstat propagation (08-snapshots-quotas-subvolumes); the cache read-only mode on write errors and the InoTable (06-journal-and-storage); the down:damaged rank state and the stopping state machine (01-cluster-membership-and-ha); MDSAuthCaps checks via stray_prior_path, flush_path, perf counters and generic health warnings (11-security-admin-and-observability); offline repair tools (cephfs-data-scan, cephfs-journal-tool, first-damage.py) are described on the mgr-tools map page.

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

58 features: 57 used by current peers (12 of them with legacy parts) and 1 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**Stray directories and deferred deletion**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-001](#f-bg-001) | Per-rank stray directories (~mdsN/stray0..stray9) *(legacy parts)* | internal | on-disk | internal, tools |
| [F-BG-002](#f-bg-002) | Deferred deletion: unlink, rmdir and rename-over move the inode to a stray | required | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-BG-003](#f-bg-003) | Stray directory rotation per log segment and pre-fragmentation | internal | config | internal |
| [F-BG-004](#f-bg-004) | ENOSPC on unlink when the current stray dirfrag is full | operational | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-BG-005](#f-bg-005) | Stray dentry replication to other ranks | internal | mds-wire | mds-ranks |

**Stray evaluation and purge eligibility**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-006](#f-bg-006) | Purge eligibility decision (eval_stray) | supporting | internal | kclient, libcephfs, ceph-fuse, ganesha |
| [F-BG-007](#f-bg-007) | Stray re-evaluation triggers (NOTIFYREF hooks and delayed list) | internal | config | internal |
| [F-BG-008](#f-bg-008) | Prompt cap release for unlinked inodes (nlink 0 grant with Ls) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-BG-009](#f-bg-009) | Snapshot-aware purge decision (past-parent snaps, realm SnapContext) | extension | cluster-wire | osd, internal |
| [F-BG-010](#f-bg-010) | Truncate-only purge for multiversion files (TRUNCATE_FILE) | extension | on-disk, cluster-wire | osd, internal |

**Hard-linked strays: reintegration and migration**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-011](#f-bg-011) | Hard-link reintegration (silent link-merge rename) | internal | mds-wire | internal, mds-ranks |
| [F-BG-012](#f-bg-012) | Stray migration between ranks | internal | mds-wire | mds-ranks |

**PurgeQueue: persistence and execution**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-013](#f-bg-013) | PurgeQueue journal of PurgeItems *(legacy parts)* | internal | on-disk | internal, tools |
| [F-BG-014](#f-bg-014) | Durable deletion intent before stray metadata removal | internal | on-disk | internal |
| [F-BG-015](#f-bg-015) | File data and backtrace object deletion (PURGE_FILE) | supporting | cluster-wire, on-disk | osd, kclient, libcephfs |
| [F-BG-016](#f-bg-016) | Directory object deletion (PURGE_DIR) | supporting | cluster-wire, on-disk | osd, internal |
| [F-BG-017](#f-bg-017) | Purge throttling and pacing | operational | config | admin, osd |
| [F-BG-018](#f-bg-018) | Parallel execution with contiguous expire_pos progress | internal | config | internal |
| [F-BG-019](#f-bg-019) | Purge recovery and at-least-once execution | supporting | on-disk | internal |
| [F-BG-020](#f-bg-020) | PurgeQueue error handling (read-only queue) | internal | config | internal |

**Rank lifecycle: activation scan and rank stop**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-021](#f-bg-021) | Activation stray scan | internal | internal | internal |
| [F-BG-022](#f-bg-022) | Rank stop: export all strays to rank 0 | operational | mds-wire | mon, cephadm, rook, admin, mds-ranks |
| [F-BG-023](#f-bg-023) | Rank stop: drain the PurgeQueue | operational | metrics | mon, cephadm, rook, admin |

**Stray visibility to clients and operators**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-025](#f-bg-025) | Purging inodes are invisible to clients (ENOENT / ESTALE) | supporting | client-wire | kclient, libcephfs, ganesha, ceph-fuse |
| [F-BG-026](#f-bg-026) | Namespace rules for stray and unlinked directories | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-BG-027](#f-bg-027) | dump stray admin command | operational | admin | admin, tools |

**Forward scrub: control, scheduling and reporting**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-028](#f-bg-028) | scrub start command *(legacy parts)* | operational | admin | admin |
| [F-BG-029](#f-bg-029) | Scrub tags and the scrub_tag object xattr | operational | on-disk, admin | admin, tools, osd |
| [F-BG-030](#f-bg-030) | tag path command | operational | admin | admin, tools, osd |
| [F-BG-031](#f-bg-031) | Scrub traversal order and scheduling (ScrubStack) *(legacy parts)* | internal | internal | internal |
| [F-BG-032](#f-bg-032) | Scrub concurrency throttle | operational | config | admin |
| [F-BG-033](#f-bg-033) | Incremental scrub and the force option | operational | admin, on-disk | admin |
| [F-BG-034](#f-bg-034) | Scrub stamps on inodes and dirfrags *(legacy parts)* | internal | on-disk | internal |
| [F-BG-035](#f-bg-035) | Scrubbing ~mdsdir / ~mdsN and scrub_mdsdir (stray loading) *(legacy parts)* | operational | admin | admin |
| [F-BG-036](#f-bg-036) | Scrub pause, resume and abort | operational | admin, mds-wire | admin, mds-ranks |
| [F-BG-037](#f-bg-037) | Scrub status, summary and mgr task status *(legacy parts)* | operational | admin, metrics, cluster-wire | admin, mgr |
| [F-BG-038](#f-bg-038) | Scrub counters and scrub purge_status *(legacy parts)* | operational | admin, metrics, config | admin, mgr |

**Scrub checks, repairs and data uninlining**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-039](#f-bg-039) | Backtrace validation | operational | on-disk, admin | admin, osd |
| [F-BG-040](#f-bg-040) | Backtrace repair | operational | on-disk | admin, osd |
| [F-BG-041](#f-bg-041) | Journal trim after repairs | internal | internal | internal |
| [F-BG-042](#f-bg-042) | InoTable "inode wrongly marked free" check and repair | operational | on-disk | internal |
| [F-BG-043](#f-bg-043) | Base inode on-disk validation (root, ~mdsN) *(legacy parts)* | internal | on-disk | internal |
| [F-BG-044](#f-bg-044) | Directory dirstat/rstat validation and repair | operational | internal | internal |
| [F-BG-045](#f-bg-045) | Dirfrag fragstat/rstat validation and repair | operational | on-disk | internal |
| [F-BG-046](#f-bg-046) | Dentry snapshot-bound check during scrub | operational | admin | admin, tools |
| [F-BG-047](#f-bg-047) | Hard-link resolvability check | operational | admin | admin |

**Multi-rank scrub coordination**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-049](#f-bg-049) | Scrub work forwarding to authoritative ranks (MMDSScrub) | internal | mds-wire | mds-ranks |
| [F-BG-050](#f-bg-050) | Scrub completion protocol (MMDSScrubStats epochs) *(legacy parts)* | internal | mds-wire | mds-ranks |
| [F-BG-051](#f-bg-051) | Scrub behaviour on rank failure and restart | operational | internal | admin, mds-ranks |

**Damage detection, DamageTable and its effects**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-BG-052](#f-bg-052) | DamageTable registry *(legacy parts)* | operational | config | admin, mon, internal |
| [F-BG-053](#f-bg-053) | Dirfrag load damage detection | supporting | metrics | admin, mon, internal |
| [F-BG-054](#f-bg-054) | Dentry snapshot-bound corruption check at load and commit | operational | config, metrics | admin, tools |
| [F-BG-056](#f-bg-056) | Duplicate primary inode detection on dirfrag load | supporting | metrics | kclient, libcephfs, admin |
| [F-BG-057](#f-bg-057) | Fatal damage takes the rank down:damaged | operational | cluster-wire | mon, admin |
| [F-BG-059](#f-bg-059) | damage ls and damage rm commands *(legacy parts)* | operational | admin | admin, tools, mon, rook |
| [F-BG-060](#f-bg-060) | EIO for requests that touch known damage | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-BG-061](#f-bg-061) | readdir behaviour on damaged directories | required | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Legacy entries**

| ID | Feature | Legacy reasons | Relied on by |
|---|---|---|---|
| [F-BG-048](#f-bg-048) | Data uninlining during scrub | deprecated-admin, old-data | kclient, libcephfs, osd |

## 3. Features

### Stray directories and deferred deletion

How the C++ MDS turns the removal of a primary name into a metadata-only move into per-rank stray directories, and the limits of that layout.

<a id="f-bg-001"></a>

#### F-BG-001 · Per-rank stray directories (~mdsN/stray0..stray9)

*Each rank keeps ten stray directories under its private mdsdir to hold inodes whose last primary name was removed.*

Rank r owns the mdsdir `~mds<r>` (ino 0x100+r) with ten stray directories stray0..stray9 (ino `0x600+10*r+i`, NUM_STRAY = 10). A stray dentry is named by the inode number in lowercase hex without prefix and is stored as omap key "`<ino hex>_head`" (primary dentry with embedded inode) in the stray dirfrag objects `<0x600+10r+i>.<frag>` of the metadata pool. The stray directories are created at mkfs by create_mydir_hierarchy (empty, committed dirfrags with backtraces). At every activation populate_mydir looks each one up, recreates a missing one with _create_system_file, pins it (PIN_STRAY, STATE_STRAYPINNED) and fetches every leaf dirfrag fnode not yet loaded. A non-null stray dentry is always primary, and only the rank owning the stray directory purges it.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal, tools
- **Legacy parts** (old-data): populate_mydir accepting a dentry named 'stray' for stray index 0 (the 'legacy dentry name' of the notes). The fallback serves file systems whose mdsdir still holds a single stray directory named 'stray' (commit fc212548aea, 2010-11-23, 'allow for old fs's with stray instead of stray0'). create_mydir_hierarchy names the directories stray0 to stray9 at this commit. Evidence: `src/mds/MDCache.cc:853-855 (MDCache::populate_mydir)`, `src/mds/MDCache.cc:537-543 (MDCache::create_mydir_hierarchy)`. Also: git fc212548aea (mds: allow for old fs's with stray instead of stray0).
- **Map:** [strays-and-purge.md › 3.1 Stray directories: numbering, naming, sharding, rotation](../cpp-map/11-background-services/strays-and-purge.md#31-stray-directories-numbering-naming-sharding-rotation) · [strays-and-purge.md › 4. Data structures and invariants](../cpp-map/11-background-services/strays-and-purge.md#4-data-structures-and-invariants) · [strays-and-purge.md › 5. Interfaces](../cpp-map/11-background-services/strays-and-purge.md#5-interfaces)
- **Code:** `src/mds/mdstypes.h:34-59 (MDS_INO_STRAY)`, `src/mds/MDCache.cc:525-571 (MDCache::create_mydir_hierarchy)`, `src/mds/MDCache.cc:821-905 (MDCache::populate_mydir)`, `src/mds/CInode.cc:1170-1175 (CInode::name_stray_dentry)`
- **Notes:** The legacy dentry name "stray" is accepted for index 0. A damaged stray dirfrag is not tolerated at activation: populate_mydir asserts !STATE_BADFRAG and relies on the DamageTable having already marked the rank damaged (F-BG-057). Stray directories cannot be exported to another rank (except a directory under a stray dir to the rank owning that stray dir, Migrator::export_dir), and the balancer ignores stray subtrees.

<a id="f-bg-002"></a>

#### F-BG-002 · Deferred deletion: unlink, rmdir and rename-over move the inode to a stray

*Removing the last primary name of an inode moves it into the leader rank's current stray directory and replies without deleting any data.*

The leader of an UNLINK/RMDIR of a primary dentry, or of a RENAME whose destination is a primary dentry and not a link merge, calls prepare_stray_dentry to add a null dentry "`<ino hex>`" in its own current stray dir, wrlocks the stray dir inode's filelock and nestlock and xlocks the stray dentry. _unlink_local projects the inode: stray_prior_path is set to the old path, ctime and change_attr are bumped, nlink is decremented (CInode::STATE_ORPHAN when it reaches 0), rstat moves from the old parent to the stray dirfrag (predirty_journal_parents -1/+1) and the backtrace is marked for rewrite. EUpdate "unlink_local" (or "rename") journals the stray dentry as primary with dirty_parent, so the backtrace on object 0 later points at `~mdsN/strayK/<ino>`. After the commit _unlink_local_finish sends MDentryUnlink to replicas, replies, moves the stray dentry to the bottom of the main LRU and calls notify_stray. Recursive stats and quota usage of the old parent drop at unlink time; pool usage drops only when the PurgeQueue deletes the objects.

- **Class:** POSIX `required` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [strays-and-purge.md › 3.2 How an inode becomes a stray](../cpp-map/11-background-services/strays-and-purge.md#32-how-an-inode-becomes-a-stray) · [strays-and-purge.md › 1. Purpose and responsibilities](../cpp-map/11-background-services/strays-and-purge.md#1-purpose-and-responsibilities) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract) · [namespace-mutations.md](../cpp-map/03-request-path/namespace-mutations.md) · [rename.md](../cpp-map/03-request-path/rename.md)
- **Code:** `src/mds/Server.cc:3595-3631 (Server::prepare_stray_dentry)`, `src/mds/Server.cc:8738-8826 (Server::_unlink_local)`, `src/mds/Server.cc:8881-8887 (Server::_unlink_local_finish)`, `src/mds/Server.cc:9449-9459 (Server::handle_client_rename)`, `src/mds/MDCache.cc:912-923 (MDCache::get_stray_dir)`
- **Notes:** The request handlers themselves are owned by the namespace category (F-NS entries for unlink, rmdir, rename); this entry covers only the stray side. The code notes that respond_to_request() drops locks before notify_stray, so stray reintegration can race with the unlinking request. stray_prior_path is also used by MDSAuthCaps path checks for unlinked inodes (owned by 11-security-admin-and-observability). The unlink-time snaprealm with past-parent snaps is owned by 08-snapshots-quotas-subvolumes (see F-BG-009 for how purge uses it). See also F-NS-045, F-NS-047, F-NS-050.

<a id="f-bg-003"></a>

#### F-BG-003 · Stray directory rotation per log segment and pre-fragmentation

*Selects which of the ten stray directories receives new strays and splits or merges a future one ahead of use.*

stray_index changes only in MDCache::advance_stray, which MDLog::_start_new_segment calls whenever a new journal segment starts (not while the rank is stopping), so all strays of one segment go to one stray dir, spread over its dirfrags by the hash of the stray name (get_stray_dir). When the index advances and no stray dir is fragmenting, advance_stray picks the dir 3 slots ahead and queues a fast split (mds_bal_split_bits = 3) for each of its dirfrags with more than mds_bal_split_size (10000) entries, or a merge where should_merge() holds; the rotation skips the dir being fragmented. The normal balancer path (MDBalancer::maybe_fragment) skips stray dirs, and directories inside stray dirs are never split or merged (can_fragment). The stray dirfrag stays auth-pinned from enqueue until the purge metadata removal is logged, so it cannot freeze or fragment underneath a pending purge.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_bal_split_size (10000); mds_bal_merge_size (50); mds_bal_split_bits (3)
- **Map:** [strays-and-purge.md › 3.1 Stray directories: numbering, naming, sharding, rotation](../cpp-map/11-background-services/strays-and-purge.md#31-stray-directories-numbering-naming-sharding-rotation) · [strays-and-purge.md › 6. Concurrency and mds_lock](../cpp-map/11-background-services/strays-and-purge.md#6-concurrency-and-mds_lock) · [dirfrags-and-fragmentation.md](../cpp-map/04-cache/dirfrags-and-fragmentation.md)
- **Code:** `src/mds/MDCache.cc:776-819 (MDCache::advance_stray)`, `src/mds/MDLog.cc:386-389 (MDLog::_start_new_segment)`, `src/mds/MDBalancer.cc:1296-1302 (MDBalancer::maybe_fragment)`, `src/mds/MDCache.cc:11884-11888 (MDCache::can_fragment)`
- **Notes:** The code comment says the previous stray dir is avoided because in-flight requests may still use it. The single current stray dir is a placement policy (map section 6 notes no data conflict requires it).

<a id="f-bg-004"></a>

#### F-BG-004 · ENOSPC on unlink when the current stray dirfrag is full

*Unlink, rmdir and rename-over fail with ENOSPC when the chosen stray dirfrag already holds the per-dirfrag entry limit.*

prepare_stray_dentry runs check_fragment_space on the chosen stray dirfrag (skipped for replayed requests). When that dirfrag already has mds_bal_fragment_size_max entries (default 100000), the UNLINK, RMDIR or RENAME is answered with ENOSPC. The same check (plus check_dir_max_entries) applies to the destination stray dirfrag of a stray migration, including shutdown migration to rank 0 (F-BG-022). Pre-splitting by advance_stray mitigates it only when can_fragment allows a split (not while the cache is read-only, the cluster is degraded, the stray dir inode is quiesced, the dirfrag is being scrubbed, or it is already fragmenting).

- **Class:** POSIX `operational` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_bal_fragment_size_max (100000)
- **Map:** [strays-and-purge.md › 3.1 Stray directories: numbering, naming, sharding, rotation](../cpp-map/11-background-services/strays-and-purge.md#31-stray-directories-numbering-naming-sharding-rotation) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract) · [strays-and-purge.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/strays-and-purge.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Server.cc:3605-3609 (Server::prepare_stray_dentry)`, `src/mds/Server.cc:3563-3576 (Server::check_fragment_space)`, `src/mds/Server.cc:9514-9518 (Server::handle_client_rename)`, `linux/fs/ceph/dir.c:1256-1301 (ceph_async_unlink_cb)`
- **Notes:** ENOSPC is not a POSIX unlink error; it follows from the shared per-rank stray directory layout. The kernel async unlink cannot return it to the caller: ceph_async_unlink_cb logs "failure path=... result=...", sets a mapping error on the parent and the inode, and drops the dentry. Mass deletion while a backlog of open or snapshotted strays exists is the scenario named on the map page. See also F-CACHE-080 (the per-dirfrag limit), F-NS-020.

<a id="f-bg-005"></a>

#### F-BG-005 · Stray dentry replication to other ranks

*Replicas of an unlinked dentry learn the new stray linkage, hand their client caps to the auth, and expire quickly so the auth can purge.*

After the unlink/rename commit, MDentryUnlink to the dentry's replicas carries a replica of the stray dentry (straybl) and snapbl. Replicas relink the inode under the replicated stray dentry and, if they hold client caps for it, export those caps to the auth (migrator->export_caps), so the auth sees every cap that blocks the purge. Replica stray dentries live in bottom_lru, which every trim pass empties as far as the trim throttle allows; the resulting MCacheExpire reaches dentry_remove_replica on the auth, which re-evaluates the stray. purge() asserts the stray dentry is no longer replicated. Ranks expiring replicas of a stopping rank's mdsdir (expire_recursive) refuse to expire strays that still have linkage, because they may be the target of a shutdown migration rename.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [strays-and-purge.md › 3.2 How an inode becomes a stray](../cpp-map/11-background-services/strays-and-purge.md#32-how-an-inode-becomes-a-stray) · [strays-and-purge.md › 3.3 Evaluation triggers: state bits, NOTIFYREF and the de...](../cpp-map/11-background-services/strays-and-purge.md#33-evaluation-triggers-state-bits-notifyref-and-the-delayed-list) · [strays-and-purge.md › 5. Interfaces](../cpp-map/11-background-services/strays-and-purge.md#5-interfaces)
- **Code:** `src/mds/MDCache.cc:11543-11578 (MDCache::send_dentry_unlink)`, `src/mds/MDCache.cc:11600-11627 (MDCache::handle_dentry_unlink)`, `src/mds/MDCache.cc:11304-11345 (MDCache::encode_replica_stray)`, `src/mds/MDCache.cc:7918-7931 (MDCache::dentry_remove_replica)`, `src/mds/CDir.cc:352-366 (CDir::adjust_dentry_lru)`
- **Notes:** Cache replication primitives (MDentryUnlink, MCacheExpire) are owned by 05-metadata-cache-and-dirfrags; this entry covers the stray payload and the cap export. See also F-CACHE-056 (link/unlink push).

### Stray evaluation and purge eligibility

When a stray may be deleted: the eval_stray decision, the events that re-run it, the client cap behaviour that lets it run early, and the snapshot rules.

<a id="f-bg-006"></a>

#### F-BG-006 · Purge eligibility decision (eval_stray)

*Decides for each stray whether to wait, purge, truncate only, or try to reintegrate it, which is what keeps unlinked-but-open files usable.*

eval_stray (re-entrancy guarded by CDentry::STATE_EVALUATINGSTRAY) acts only on auth stray dentries with a primary linkage once StrayManager::activate has run. For nlink == 0 it first prunes past-parent snapids that no longer exist and old_inodes no live snap covers; directories then get clear_dirty_bits_for_stray (asserts every head dentry is null, drops leases and unreferenced dentries, marks dentries, inodes and the dirfrag clean when the dir has no snaprealm) and stale snapped remote dentries are unlinked. It returns without action while the stray dentry is replicated, has client leases, the inode has any client cap (including caps of stale, not yet evicted sessions), the inode needs or is doing file-size recovery, or the inode or dentry has references beyond dirty pins. Otherwise it enqueues PURGE_FILE or PURGE_DIR (closing dirfrags first for directories), or TRUNCATE_FILE for multiversion files with past-parent snaps (F-BG-010); for nlink > 0 it goes to reintegration or migration (F-BG-011).

- **Class:** POSIX `supporting` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [strays-and-purge.md › 3.4 eval_stray: the decision](../cpp-map/11-background-services/strays-and-purge.md#34-eval_stray-the-decision) · [strays-and-purge.md › 4. Data structures and invariants](../cpp-map/11-background-services/strays-and-purge.md#4-data-structures-and-invariants) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract)
- **Code:** `src/mds/StrayManager.cc:447-562 (StrayManager::_eval_stray)`, `src/mds/StrayManager.cc:571-581 (StrayManager::eval_stray)`, `src/mds/MDCache.cc:14004-14017 (MDCache::clear_dirty_bits_for_stray)`, `src/mds/CDir.cc:805-849 (CDir::try_remove_dentries_for_stray)`, `src/mds/Locker.cc:4363-4371 (Locker::remove_client_cap)`
- **Notes:** An unlinked file that is still open stays fully usable (read, write, getattr, setattr, max_size growth) because any cap blocks the purge. A crashed client pins its unlinked open files until its session is evicted. When the last cap of an unlinked inode goes away, Locker::remove_client_cap skips the usual NEEDSRECOVER size recovery. Other references also block purge: PIN_TRUNCATING of an ordinary truncate still running, a scrub queue pin, and the pin of an in-flight reintegration request ("too many inode refs"). See also F-NS-048 (POSIX contract).

<a id="f-bg-007"></a>

#### F-BG-007 · Stray re-evaluation triggers (NOTIFYREF hooks and delayed list)

*Re-runs the purge decision when the last reference to a stray goes away, plus a fixed set of other events.*

CDentry::STATE_STRAY and CInode::STATE_ORPHAN alias MDSCacheObject::STATE_NOTIFYREF, so every put() calls the object's _put(), which calls maybe_eval_stray(delay=true) once only dirty pins remain; that appends the dentry to delayed_eval_stray, drained at the start of every MDCache::trim (upkeep thread, every mds_cache_trim_interval = 1 s). Immediate triggers: notify_stray after unlink/rename, LRU trim of an auth stray dentry (trim_inode, which runs only under cache pressure), replica expiry (dentry_remove_replica), _truncate_stray_logged, and the activation scan; a dirfrag commit of a snapshotted dir under a stray dir queues a delayed one. maybe_eval_stray returns early for nlink > 0, base inodes, a read-only cache, rank states up to up:rejoin, and dentries already PURGING, so hard-linked strays are evaluated only via notify_stray and eval_remote. Snapshot deletion does not trigger re-evaluation.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_cache_trim_interval (1 s)
- **Map:** [strays-and-purge.md › 3.3 Evaluation triggers: state bits, NOTIFYREF and the de...](../cpp-map/11-background-services/strays-and-purge.md#33-evaluation-triggers-state-bits-notifyref-and-the-delayed-list) · [strays-and-purge.md › 6. Concurrency and mds_lock](../cpp-map/11-background-services/strays-and-purge.md#6-concurrency-and-mds_lock) · [mdcache-lifecycle-trim-and-memory.md](../cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md)
- **Code:** `src/mds/CInode.cc:991-995 (CInode::_put)`, `src/mds/CDentry.cc:646-656 (CDentry::_put)`, `src/mds/MDCache.cc:13983-14002 (MDCache::maybe_eval_stray)`, `src/mds/StrayManager.cc:374-409 (StrayManager::advance_delayed)`, `src/mds/MDCache.cc:7036-7037 (MDCache::trim)`
- **Notes:** STRAY/ORPHAN are set only by prepare_stray_dentry, _unlink_local/_rename_prepare and scan_stray_dir; they are not among the exported inode state bits, so a stray migrated to another rank, and stray dirfrags loaded by scrub or dump stray, get no ref-drop trigger there (code reading). Evaluation latency after the last cap release is up to mds_cache_trim_interval.

<a id="f-bg-008"></a>

#### F-BG-008 · Prompt cap release for unlinked inodes (nlink 0 grant with Ls)

*Tells clients an inode was deleted so they drop it and release caps, letting the purge proceed without waiting for cache trimming or recall.*

Locker::issue_caps adds CEPH_CAP_LINK_SHARED (Ls) to the wanted set of nlink == 0 inodes "to make sure they release caps ASAP", so clients receive an MClientCaps grant carrying nlink = 0 with Ls. On a grant with Ls but not Lx and nlink 0, the kernel sets i_nlink = 0 and prunes all dentry aliases (invalidate_aliases), so the VFS evicts the inode and releases its caps on last close (ceph_evict_inode); libcephfs does the same through _try_to_trim_inode. The kernel also embeds cap releases in the unlink request (ceph_drop_caps_for_unlink drops Ls/Lx and, when i_nlink == 1, all unwanted caps except p), so an unopened file can be purged right after the unlink commits.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [strays-and-purge.md › 5. Interfaces](../cpp-map/11-background-services/strays-and-purge.md#5-interfaces) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract) · [capabilities.md](../cpp-map/02-client-protocol/capabilities.md) · [kclient-expectations.md](../cpp-map/02-client-protocol/kclient-expectations.md)
- **Code:** `src/mds/Locker.cc:2652-2654 (Locker::issue_caps)`, `linux/fs/ceph/caps.c:3664-3669 (handle_cap_grant)`, `linux/fs/ceph/caps.c:4892 (ceph_drop_caps_for_unlink)`, `src/client/Client.cc:6161-6167 (Client::handle_cap_grant)`
- **Notes:** The cap issue/revoke machinery is owned by 03-caps-leases-and-locking; this entry covers the stray-specific rule. Clients that leak caps (FUSE inode cache, NFS servers holding handles) keep num_strays high; cap recall pressure is the only lever.

<a id="f-bg-009"></a>

#### F-BG-009 · Snapshot-aware purge decision (past-parent snaps, realm SnapContext)

*Keeps strays that ancestor snapshots still need and makes OSDs preserve snapshot clones when stray data is deleted.*

_eval_stray consults only the past-parent snaps of the inode's snaprealm, which the snapshot code records at unlink time because the new parent ~mdsN/strayK has no snapshots. A stray directory whose realm still has past-parent snaps is never purged (no dirty-bit clearing either); if it has CInode::STATE_MISSINGOBJS (an earlier dirfrag commit found its object gone), the MDS logs clog error "previous attempt at committing dirfrag of ino ... has failed, missing object" and calls handle_write_error(-ENOENT). A file with past-parent snaps but no old_inodes is fully purged (PURGE_FILE) with the inode realm SnapContext, which includes the surviving past-parent snaps, so OSDs keep the clones the snapshots need, while the snapshot metadata view lives in the COWed dentry of the old directory. Snapshots taken on a directory itself are handled earlier by rmdir (ENOTEMPTY in the unlocked fast path).

- **Class:** POSIX `extension` · exposure `cluster-wire` · relied on by osd, internal
- **Map:** [strays-and-purge.md › 3.4 eval_stray: the decision](../cpp-map/11-background-services/strays-and-purge.md#34-eval_stray-the-decision) · [strays-and-purge.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/strays-and-purge.md#8-edge-cases-failure-handling-and-gotchas) · [snapshots.md](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md)
- **Code:** `src/mds/StrayManager.cc:487-497 (StrayManager::_eval_stray)`, `src/mds/SnapRealm.cc:86-119 (SnapRealm::build_snap_set)`, `src/mds/SnapRealm.cc:511-527 (SnapRealm::prune_past_parent_snaps)`, `src/mds/CDir.cc:2912-2915 (CDir::_committed)`, `src/mds/CInode.cc:631-654 (CInode::record_snaprealm_past_parent)`
- **Notes:** Snapshot deletion never triggers re-evaluation (no code path calls it); such strays are reconsidered only on a later ref drop, trim of the loaded dentry, or activation scan. The rmdir fast-path snapshot check sits after an early return taken when the filelock is cached, and the locked re-check _dir_is_nonempty compares only fragstats (map 3.4). Recording past_parent_snaps is owned by 08-snapshots-quotas-subvolumes.

<a id="f-bg-010"></a>

#### F-BG-010 · Truncate-only purge for multiversion files (TRUNCATE_FILE)

*Deletes the head data of an unlinked file that snapshots still reference through old_inodes, keeping its inode and backtrace.*

A stray file whose realm has past-parent snaps and whose inode has old_inodes is enqueued as TRUNCATE_FILE when size > 0 (size bound as for PURGE_FILE, realm snapc required); with size == 0 nothing is enqueued. The executor deletes objects 1..N-1 (PURGE_OP_RANGE, only if N > 1) and zeroes object 0 over [0, object_size) with Filer::zero keep_first (PURGE_OP_ZERO), since object 0 carries the backtrace, using the realm SnapContext. When the item is durable the MDS journals EUpdate "purge_stray truncate" with size, max_size_ever, truncate_size and truncate_from set to 0, client_ranges cleared and the client-writeable flag cleared, then clears PURGING and re-evaluates; while the snapshots exist that evaluation sees size 0 and the stray stays.

- **Class:** POSIX `extension` · exposure `on-disk`, `cluster-wire` · relied on by osd, internal
- **Map:** [strays-and-purge.md › 3.4 eval_stray: the decision](../cpp-map/11-background-services/strays-and-purge.md#34-eval_stray-the-decision) · [strays-and-purge.md › 3.6 Persisting the item, then removing the stray metadata](../cpp-map/11-background-services/strays-and-purge.md#36-persisting-the-item-then-removing-the-stray-metadata) · [strays-and-purge.md › 3.7 PurgeQueue executor](../cpp-map/11-background-services/strays-and-purge.md#37-purgequeue-executor)
- **Code:** `src/mds/StrayManager.cc:532-542 (StrayManager::_eval_stray)`, `src/mds/StrayManager.cc:747-778 (StrayManager::truncate)`, `src/mds/StrayManager.cc:192-217 (StrayManager::_purge_stray_purged)`, `src/mds/StrayManager.cc:780-798 (StrayManager::_truncate_stray_logged)`, `src/osdc/Filer.cc:117-151 (Filer::zero)`
- **Notes:** The RANGE and ZERO ops are issued concurrently in one gather; with stripe_count > 1 the zero range also maps partly onto objects 1..stripe_count-1, where Filer::zero sends ZERO; the OSD treats ZERO of a missing or whiteout object as a no-op, so the race is harmless (code reading plus PrimaryLogPG). A crash after the item is durable but before "purge_stray truncate" is safe re-enqueues the truncate. Ordinary truncate()/O_TRUNC of linked files does not use the PurgeQueue (owned by 04-namespace-and-posix-operations).

### Hard-linked strays: reintegration and migration

Strays that still have hard links are moved back under a remaining name, or to the rank that can do that, by MDS-originated rename requests.

<a id="f-bg-011"></a>

#### F-BG-011 · Hard-link reintegration (silent link-merge rename)

*Moves an inode whose primary name was removed but that still has hard links back under one of its remaining names.*

For a stray with nlink > 0, eval_remote/_eval_stray_remote picks a cached, head, non-projected remote dentry from remote_parents, preferring an auth one whose dir can be auth-pinned; with none in cache nothing happens, because no on-disk back-pointer leads from an inode to its remote dentries. reintegrate_stray sends an MClientRequest(CEPH_MDS_OP_RENAME) with destination = the remote dentry and source = the stray dentry through the messenger to the remote dentry's auth (possibly itself), tracked in internal_client_requests with PIN_PURGING and reintegration_reqid. The rename is a link merge with silent set (source dir is a stray dir): the remote dentry becomes primary and the stray dentry null, and nlink, the inode ctime/change_attr and the directory mtimes are unchanged (no PREDIRTY_DIR). Triggers are notify_stray after the unlink and eval_remote whenever a remote dentry is linked in cache (dirfrag load, readdir, link, rename, path_traverse, open_remote_dentry) or appears in a reply trace.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by internal, mds-ranks
- **Map:** [strays-and-purge.md › 3.9 Hard links: remote evaluation, reintegration and migr...](../cpp-map/11-background-services/strays-and-purge.md#39-hard-links-remote-evaluation-reintegration-and-migration) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract) · [rename.md](../cpp-map/03-request-path/rename.md)
- **Code:** `src/mds/StrayManager.cc:583-609 (StrayManager::eval_remote)`, `src/mds/StrayManager.cc:625-677 (StrayManager::_eval_stray_remote)`, `src/mds/StrayManager.cc:679-706 (StrayManager::reintegrate_stray)`, `src/mds/Server.cc:9373-9381 (Server::handle_client_rename)`, `src/mds/Server.cc:10068-10098 (Server::_rename_prepare)`
- **Notes:** Without reintegration the inode stays reachable through its remote dentries via open_ino (backtrace pointing into the stray dir). MDS-originated requests have no session (check_access passes), bypass dmClock, get no early reply, and Server::handle_client_reply ignores their result. If the target must forward the request it is dropped ("request_forward drop ... was from mds"); messages to a down rank or a rank in up:reconnect are dropped; internal_client_requests has no timeout, so the stray stays pinned with reintegration_reqid set until the originating daemon restarts (code reading). The activation scan never evaluates hard-linked strays; they stay in the stray dir until a remote dentry is touched. strays_reintegrated counts RENAMEs sent, not completed.

<a id="f-bg-012"></a>

#### F-BG-012 · Stray migration between ranks

*Moves a stray into the same stray slot of another rank, to let that rank reintegrate it or because this rank is stopping.*

migrate_stray(dn, to) sends an MDS-originated MClientRequest(CEPH_MDS_OP_RENAME) from `~mds<s>/strayK/<ino>` to `~mds<to>/strayK/<ino>` (MDS_INO_STRAY(to, K)). It is used when the stray is auth here but the chosen remote dentry is auth on another rank, and for every stray of a stopping non-zero rank (destination rank 0, F-BG-022). It runs as a cross-rank rename two-phase commit with inode export; the source rank is a witness and _commit_peer_rename detects migrated_stray. The destination stray dirfrag goes through check_fragment_space and check_dir_max_entries, so the migration can fail with ENOSPC. A rank that is auth for the remote dentry and holds only a replica of the stray reintegrates directly with a loopback RENAME instead, with the stray's rank as witness and no migration.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [strays-and-purge.md › 3.9 Hard links: remote evaluation, reintegration and migr...](../cpp-map/11-background-services/strays-and-purge.md#39-hard-links-remote-evaluation-reintegration-and-migration) · [strays-and-purge.md › 5. Interfaces](../cpp-map/11-background-services/strays-and-purge.md#5-interfaces) · [rename.md](../cpp-map/03-request-path/rename.md)
- **Code:** `src/mds/StrayManager.cc:708-738 (StrayManager::migrate_stray)`, `src/mds/Server.cc:10838-10941 (Server::_commit_peer_rename)`, `src/mds/Server.cc:9514-9518 (Server::handle_client_rename)`, `src/mds/StrayManager.cc:658-661 (StrayManager::_eval_stray_remote)`, `src/mds/CInode.h:391-393 (CInode::MASK_STATE_EXPORTED)`
- **Notes:** The destination does not evaluate the migrated stray explicitly and gets no STRAY/ORPHAN flags, so a migrated stray whose caps were still held when the source expired its replica can stay unpurged until cache pressure, eval_remote or a restart (code reading). strays_migrated counts RENAMEs sent.

### PurgeQueue: persistence and execution

The per-rank RADOS-journaled queue that makes deletion durable before metadata removal and deletes data, backtrace and dirfrag objects asynchronously with throttling.

<a id="f-bg-013"></a>

#### F-BG-013 · PurgeQueue journal of PurgeItems

*Per-rank RADOS-journaled queue that durably records what to delete, decoupling fast metadata removal from slow object deletion.*

Each rank has a Journaler "pq" at inode 0x500+rank (objects `<0x500+r>.00000000` header and `<0x500+r>.<n>` data) in the metadata pool with the default file layout (4 MiB objects) and JOURNAL_FORMAT_RESILIENT framing, created in up:creating, or on open when the header is missing ("assuming this is an upgrade"). Entries are PurgeItems encoded ENCODE_START(2, 1): action u8 (NONE 0, PURGE_FILE 1, TRUNCATE_FILE 2, PURGE_DIR 3), ino, size u64, file_layout_t, old_pools `vector<int64>`, SnapContext {seq, snaps}, fragtree_t and stamp (v2), followed by padding. The same PurgeQueue instance appends items (push) and reads them back (_consume) to execute them; whole journal objects below expire_pos are trimmed after header writes. The queue has its own mutex, finisher thread (mds-pq-fin), SafeTimer, Filer and Journaler; it takes mds_lock only for the item-persisted completions and the on_error handler, both run on mds-pq-fin.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal, tools
- **Legacy parts** (old-data): PurgeItem struct_v 1 decoding: the 12.x/13.2.0-13.2.1 layout without stamp, and the trial decode of the v13.2.2 layout with stamp and pad first. Releases 12.x and 13.2.0-13.2.2 wrote struct_v 1 (13.2.2 with stamp and pad first), and 13.2.3 onward, including this commit, write struct_v 2. Queue creation on open when the header is missing ('assuming this is an upgrade') is also reached by the documented alternate-pool recovery, because fs new --recover marks rank 0 failed instead of creating it, so that path is current. Evidence: `src/mds/PurgeQueue.cc:63-104 (PurgeItem::decode)`, `src/mds/PurgeQueue.cc:45-47 (PurgeItem::encode, ENCODE_START(2, 1))`, `v12.2.0:src/mds/PurgeQueue.cc:33`, `v13.2.0:src/mds/PurgeQueue.cc:33 (ENCODE_START(1, 1))`, `src/mds/PurgeQueue.cc:251-255 (PurgeQueue::open)`, `src/mds/FSMap.cc:485-490 (FSMap::create_filesystem)`, `doc/cephfs/disaster-recovery-experts.rst:424-425`. Also: v13.2.2:src/mds/PurgeQueue.cc:40-54 (PurgeItem::encode, v1 with stamp; pad first); v13.2.3:src/mds/PurgeQueue.cc (ENCODE_START(2, 1)); git b73d1989bcb (mds: handle bad purge queue item encoding; 'fixes a bad backport in v13.2.2').
- **Map:** [strays-and-purge.md › 3.7 PurgeQueue executor](../cpp-map/11-background-services/strays-and-purge.md#37-purgequeue-executor) · [strays-and-purge.md › 4. Data structures and invariants](../cpp-map/11-background-services/strays-and-purge.md#4-data-structures-and-invariants) · [strays-and-purge.md › 5. Interfaces](../cpp-map/11-background-services/strays-and-purge.md#5-interfaces) · [rados-object-catalog.md](../cpp-map/07-storage-formats/rados-object-catalog.md)
- **Code:** `src/mds/PurgeQueue.h:43-81 (PurgeItem)`, `src/mds/PurgeQueue.cc:45-105 (PurgeItem::encode)`, `src/mds/PurgeQueue.cc:334-355 (PurgeQueue::create)`, `src/mds/PurgeQueue.cc:360-399 (PurgeQueue::push)`, `src/mds/mdstypes.h:45 (MDS_INO_PURGE_QUEUE)`
- **Notes:** cephfs-journal-tool --journal=purge_queue supports journal inspect/export/reset and event get/splice; erasing a region writes a NONE PurgeItem whose pad_size bytes of 0xff fill the gap, and the executor drops NONE or unknown items ("Invalid item ... dropping it") without advancing expire_pos for them. The decoder also accepts the bad struct_v 1 encoding introduced by v13.2.2 by trial decode. The single mds-pq-fin thread runs both mds_lock-taking completions and executor progress, so a contended mds_lock stalls purge I/O completions.

<a id="f-bg-014"></a>

#### F-BG-014 · Durable deletion intent before stray metadata removal

*Removes the stray dentry and inode from the namespace only after the purge item is durable in the PurgeQueue journal.*

enqueue removes the inode from the scrub stack, sets STATE_PURGING on dentry and inode, clears dirty_parent for a full purge (so segment expiry does not rewrite a backtrace about to be deleted), takes PIN_PURGING and auth-pins the stray dirfrag (waiting on WAIT_UNFREEZE if needed). push() appends the item and registers wait_for_flush(C_IO_PurgeStrayPurged); if the consumer took nothing, a one-shot timer (mds_purge_queue_busy_flush_period, 1 s) flushes the journal so strays are released while the executor is busy. When durable, _purge_stray_purged (under mds_lock) aborts the daemon on any reference taken after enqueue ("rogue reference to purging inode"), projects a null linkage and the stray dirfrag fnode decrement (nfiles or nsubdirs, rstat minus accounted_rstat) without Locker locks, and journals EUpdate "purge_stray" (stray dir context, dirty stray dirfrag, null dentry, destroyed_inodes = [ino]). _purge_stray_logged unlinks and removes the inode and auth-unpins the stray dirfrag; the omap key is removed at the next dirfrag commit, and replay of the event drops the inode and notes it in the OpenFileTable.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal
- **Gates:** mds_purge_queue_busy_flush_period (1 s, dev)
- **Map:** [strays-and-purge.md › 3.5 Enqueue and PurgeItem construction](../cpp-map/11-background-services/strays-and-purge.md#35-enqueue-and-purgeitem-construction) · [strays-and-purge.md › 3.6 Persisting the item, then removing the stray metadata](../cpp-map/11-background-services/strays-and-purge.md#36-persisting-the-item-then-removing-the-stray-metadata) · [strays-and-purge.md › 4. Data structures and invariants](../cpp-map/11-background-services/strays-and-purge.md#4-data-structures-and-invariants)
- **Code:** `src/mds/StrayManager.cc:302-342 (StrayManager::enqueue)`, `src/mds/PurgeQueue.cc:360-399 (PurgeQueue::push)`, `src/mds/StrayManager.cc:182-257 (StrayManager::_purge_stray_purged)`, `src/mds/StrayManager.cc:259-300 (StrayManager::_purge_stray_logged)`, `src/mds/journal.cc:1753-1775 (EMetaBlob::replay)`
- **Notes:** The code comment "CHEAT. there's no real need to journal our intent to purge, since that is implicit in the dentry's presence and non-use in the stray dir" states the recovery model: the on-disk stray dentry is the durable intent. Inode numbers of purged inodes are not returned to the InoTable. strays_enqueued counts items once persisted. Journaler::append_entry can block on write_buf_throttle (about 3.5 GiB) while push() holds mds_lock and the PQ lock (practically unreachable). See also F-STORE-031 (destroyed inode records).

<a id="f-bg-015"></a>

#### F-BG-015 · File data and backtrace object deletion (PURGE_FILE)

*Deletes all RADOS objects of a purged regular file, symlink or special file, honouring layout, old pools and snapshots.*

For a regular file the item size is max(size, max of client_ranges, max_size_ever), covering objects a writer may have created up to its granted max_size and zero-length stripe objects left by earlier truncates; other file types use 0. The executor deletes objects "`<ino hex>.%08x`" 0..N-1 with N = Striper::get_num_objects(layout, size) via Filer::purge_range (at most filer_max_purge_ops per item in flight, -ENOENT ignored) in the layout pool and pool namespace, with the inode realm SnapContext. It also removes `<ino>.00000000` in the default namespace (the backtrace object) when size == 0 or the layout has a pool namespace, and `<ino>.00000000` in each old pool. -EBLOCKLISTED triggers on_error (respawn); every other result, including real errors, counts as done and the item is expired. All MDS OSD ops carry FULL_FORCE, so purge proceeds on full pools.

- **Class:** POSIX `supporting` · exposure `cluster-wire`, `on-disk` · relied on by osd, kclient, libcephfs
- **Map:** [strays-and-purge.md › 3.5 Enqueue and PurgeItem construction](../cpp-map/11-background-services/strays-and-purge.md#35-enqueue-and-purgeitem-construction) · [strays-and-purge.md › 3.7 PurgeQueue executor](../cpp-map/11-background-services/strays-and-purge.md#37-purgequeue-executor) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract) · [rados-and-osd-interactions.md](../cpp-map/07-storage-formats/rados-and-osd-interactions.md)
- **Code:** `src/mds/StrayManager.cc:101-157 (StrayManager::purge)`, `src/mds/PurgeQueue.cc:631-714 (PurgeQueue::_execute_item)`, `src/mds/PurgeQueue.cc:554-629 (PurgeQueue::_commit_ops)`, `src/osdc/Filer.cc:424-494 (Filer::purge_range)`, `src/osdc/Striper.cc:350-364 (Striper::get_num_objects)`
- **Notes:** Object naming and striping are shared with the clients that write file data, so the deletion must follow the same naming. Errors such as -EIO or -EPERM (OSD caps) leave objects behind silently. The gather reports only the first negative sub-result, so an earlier -ENOENT hides a later -EBLOCKLISTED; for N == 1 a single Objecter::remove reports -ENOENT to the gather (harmless).

<a id="f-bg-016"></a>

#### F-BG-016 · Directory object deletion (PURGE_DIR)

*Deletes the dirfrag objects and the backtrace object of a purged, empty directory.*

A PURGE_DIR item carries the directory's dirfragtree; the executor removes `<ino>.<frag>` for each fragtree leaf (if fragmented) plus `<ino>.00000000`, which also holds the directory backtrace, in the metadata pool with a null SnapContext; frag objects that never existed return -ENOENT. Entries of an unlinked directory are never purged individually: emptiness is established by rmdir under the directory's filelock rdlock (_dir_is_nonempty, fragstat comparison) and clear_dirty_bits_for_stray asserts every head dentry is null. Fetching a dirfrag of an unlinked directory without a snaprealm does no RADOS read (complete and empty). A later dirfrag commit that hits -ENOENT is accepted for never-committed dirfrags under a stray dir, and sets STATE_MISSINGOBJS when the inode has a snaprealm.

- **Class:** POSIX `supporting` · exposure `cluster-wire`, `on-disk` · relied on by osd, internal
- **Map:** [strays-and-purge.md › 3.7 PurgeQueue executor](../cpp-map/11-background-services/strays-and-purge.md#37-purgequeue-executor) · [strays-and-purge.md › 3.11 Startup, replay and recovery of purges](../cpp-map/11-background-services/strays-and-purge.md#311-startup-replay-and-recovery-of-purges) · [strays-and-purge.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/strays-and-purge.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/PurgeQueue.cc:631-714 (PurgeQueue::_execute_item)`, `src/mds/StrayManager.cc:101-157 (StrayManager::purge)`, `src/mds/CDir.cc:1583-1597 (CDir::fetch)`, `src/mds/CDir.cc:2810-2817 (CDir::_committed)`, `src/mds/Server.cc:9262-9287 (Server::_dir_is_nonempty)`

<a id="f-bg-017"></a>

#### F-BG-017 · Purge throttling and pacing

*Limits how many purge items and object deletions each rank runs at once, scaled by data-pool PG count and max_mds.*

_can_consume refuses when the queue is read-only, when ops_in_flight >= max_purge_ops, or when in-flight items >= mds_max_purge_files (64); one item is always admitted when nothing is in flight unless mds_max_purge_files is 0, which pauses purging. max_purge_ops = (sum of pg_num of the MDSMap data pools in the current OSDMap / max_mds) * mds_max_purge_ops_per_pg (0.5), capped by mds_max_purge_ops (8192, 0 = no cap); it is recomputed in the MDSRank constructor, on every OSDMap, on max_mds changes and on config changes (all three options are runtime-tracked). Op cost counts objects, not concurrent RADOS ops: PURGE_DIR 1 + fragtree leaves, files get_num_objects(layout, size) (1 if size 0) plus old_pools except for TRUNCATE_FILE; actual per-item concurrency is filer_max_purge_ops (10). The budget is per rank and shared by all directories and subvolumes, with no fairness or per-subvolume accounting.

- **Class:** POSIX `operational` · exposure `config` · relied on by admin, osd
- **Gates:** mds_max_purge_files (64; 0 pauses); mds_max_purge_ops (8192); mds_max_purge_ops_per_pg (0.5); filer_max_purge_ops (10)
- **Map:** [strays-and-purge.md › 3.7 PurgeQueue executor](../cpp-map/11-background-services/strays-and-purge.md#37-purgequeue-executor) · [strays-and-purge.md › 3.8 Throttle sizing, configuration and drain](../cpp-map/11-background-services/strays-and-purge.md#38-throttle-sizing-configuration-and-drain) · [strays-and-purge.md › 5. Interfaces](../cpp-map/11-background-services/strays-and-purge.md#5-interfaces)
- **Code:** `src/mds/PurgeQueue.cc:429-461 (PurgeQueue::_can_consume)`, `src/mds/PurgeQueue.cc:401-427 (PurgeQueue::_calculate_ops)`, `src/mds/PurgeQueue.cc:782-816 (PurgeQueue::update_op_limit)`, `src/mds/PurgeQueue.cc:818-836 (PurgeQueue::handle_conf_change)`
- **Notes:** A single huge file can push pq_executing_ops far above max_purge_ops (doc example pq_executing_ops 56655 with pq_executing 1) and occupies the queue for a long time. max_mds == 0 (fs set `<fs>` down true) divides by zero in double and casts inf/NaN to uint64_t, which is undefined behaviour (code reading).

<a id="f-bg-018"></a>

#### F-BG-018 · Parallel execution with contiguous expire_pos progress

*Executes purge items in parallel while persisting progress only as a contiguous prefix of the journal.*

The in_flight map is keyed by the journal offset just after each entry; when the oldest in-flight item completes, expire_pos jumps to the highest completed offset below the next still-running item (pending_expire keeps out-of-order completions), and other completions are only recorded. The Journaler header, which persists expire_pos, is written when the queue goes idle or journaler_write_head_interval (15 s) has elapsed, and the header-write completion trims whole journal objects below it. A long-running item at the head therefore holds back expiry and trimming, but not execution, of later items. Completion updates pq_executing, pq_executing_ops, pq_executed, pq_executed_ops and pq_item_in_journal.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** journaler_write_head_interval (15 s)
- **Map:** [strays-and-purge.md › 3.7 PurgeQueue executor](../cpp-map/11-background-services/strays-and-purge.md#37-purgequeue-executor) · [strays-and-purge.md › 4. Data structures and invariants](../cpp-map/11-background-services/strays-and-purge.md#4-data-structures-and-invariants)
- **Code:** `src/mds/PurgeQueue.cc:716-780 (PurgeQueue::_execute_item_complete)`, `src/mds/PurgeQueue.cc:474-536 (PurgeQueue::_consume)`, `src/osdc/Journaler.cc:1380-1421 (Journaler::_trim)`
- **Notes:** pq_item_in_journal divides by the encoded size of an empty PurgeItem plus envelope, so it overestimates the remaining item count.

<a id="f-bg-019"></a>

#### F-BG-019 · Purge recovery and at-least-once execution

*Resumes purging after a restart or failover without operator action, re-executing deletes that may already have happened.*

PurgeQueue::open recovers the Journaler; if the committed write_pos is behind the probed one, _recover reads forward to drop a partial tail entry, then reading resumes at the persisted expire_pos; -ENOENT creates the queue and other errors make it read-only. In up:replay the queue is opened asynchronously and waited for in parallel with MDLog replay; standby-replay opens it only on the takeover pass. Strays still on disk are re-evaluated by the activation scan (F-BG-021), so an item whose EUpdate "purge_stray" was not yet safe is enqueued and executed again, and items between the persisted expire_pos and real progress are re-executed; deletes are idempotent with -ENOENT tolerated. PurgeQueue::activate (from populate_mydir) computes purge_item_journal_size and starts consuming remaining entries.

- **Class:** POSIX `supporting` · exposure `on-disk` · relied on by internal
- **Map:** [strays-and-purge.md › 3.11 Startup, replay and recovery of purges](../cpp-map/11-background-services/strays-and-purge.md#311-startup-replay-and-recovery-of-purges) · [strays-and-purge.md › 1. Purpose and responsibilities](../cpp-map/11-background-services/strays-and-purge.md#1-purpose-and-responsibilities)
- **Code:** `src/mds/PurgeQueue.cc:242-332 (PurgeQueue::open)`, `src/mds/PurgeQueue.cc:202-231 (PurgeQueue::activate)`, `src/mds/MDSRank.cc:1666-1719 (MDSRank::boot_start)`, `src/mds/MDSRank.cc:1883-1884 (MDSRank::standby_replay_restart)`
- **Notes:** Until re-evaluation, a reloaded inode whose item was already durable is not PURGING, so LOOKUPINO (NFS handles) can still reach it while its objects are being deleted (code reading). Standby-replay: push() asserts !journaler.is_readonly() and the Journaler starts read-only, so a stray enqueued before the takeover open finished would abort the daemon (code reading; resolve/reconnect/rejoin normally take longer). purge_item_journal_size is uninitialized before activate(); a change of mds_max_purge_files during replay..rejoin would execute items and divide by it (code reading).

<a id="f-bg-020"></a>

#### F-BG-020 · PurgeQueue error handling (read-only queue)

*Puts the purge queue read-only on journal errors and respawns on blocklisting; all other purge errors are ignored.*

A Journaler load, read or decode error calls _go_readonly: it queues on_error, which runs MDSRank::handle_write_error (mds_action_on_write_error: 0 ignore, 1 force the cache read-only (default), 2 respawn), sets the Journaler read-only and fails waiters with the error. A boot that sees -EROFS from wait_for_recovery continues read-only. A read-only queue never consumes, and push() completes its context with -EROFS. -EBLOCKLISTED from a purge op queues on_error and leaves the item in flight; the respawn normally wins.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_action_on_write_error (1)
- **Map:** [strays-and-purge.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/strays-and-purge.md#8-edge-cases-failure-handling-and-gotchas) · [strays-and-purge.md › 6. Concurrency and mds_lock](../cpp-map/11-background-services/strays-and-purge.md#6-concurrency-and-mds_lock) · [rados-and-osd-interactions.md](../cpp-map/07-storage-formats/rados-and-osd-interactions.md)
- **Code:** `src/mds/PurgeQueue.cc:463-472 (PurgeQueue::_go_readonly)`, `src/mds/PurgeQueue.cc:604-626 (PurgeQueue::_commit_ops)`, `src/mds/MDSRank.cc:973-991 (MDSRank::handle_write_error)`, `src/mds/MDSContext.cc:95-103 (MDSIOContextBase::complete)`, `src/common/fair_mutex.h:24-30 (fair_mutex::lock)`
- **Notes:** Bugs found by code reading, not reproduced: push() on a read-only queue completes C_IO_PurgeStrayPurged synchronously under the caller's mds_lock; MDSIOContextBase::complete locks the non-recursive fair_mutex again and the thread deadlocks (and finish asserts r == 0 || -ENOENT). It is reachable with the default action 1 because a read-only cache only gates maybe_eval_stray, not advance_delayed, C_RetryEnqueue or _truncate_stray_logged. The PurgeQueue and its Journaler share one on_error context, so a second error path can use it after it was freed; a second -EBLOCKLISTED queues a null on_error. A decode error in _consume still executes the partially decoded item. The cache read-only mode itself is owned by 06-journal-and-storage.

### Rank lifecycle: activation scan and rank stop

Stray and purge work tied to rank state changes (activation, stopping). The purge of delegated and preallocated inode numbers on session close is F-STORE-072.

<a id="f-bg-021"></a>

#### F-BG-021 · Activation stray scan

*Loads every stray of the rank at activation and re-runs the purge decision for each, which is how pending purges survive restarts.*

populate_mydir runs from recovery_done (after replay), from active_start via open_root after up:creating/up:starting, and when a rank imports its own mdsdir; at its end it seeds num_strays from the sum of stray dirfrag fragstats, calls StrayManager::activate and scan_stray_dir. scan_stray_dir walks every stray inode and dirfrag in order, fully fetching incomplete dirfrags and waiting on unfreeze, sets STATE_STRAY on each dentry, ORPHAN on nlink == 0 primary inodes, and calls maybe_eval_stray for every primary stray. The whole stray population is loaded into cache under mds_lock, one dirfrag (up to mds_bal_fragment_size_max entries) per pass, without heartbeat resets. Hard-linked strays are not evaluated by it.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [strays-and-purge.md › 3.11 Startup, replay and recovery of purges](../cpp-map/11-background-services/strays-and-purge.md#311-startup-replay-and-recovery-of-purges) · [strays-and-purge.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/strays-and-purge.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDCache.cc:821-905 (MDCache::populate_mydir)`, `src/mds/MDCache.cc:10388-10455 (MDCache::scan_stray_dir)`, `src/mds/MDSRank.cc:2166-2179 (MDSRank::recovery_done)`, `src/mds/MDSRank.cc:2137-2139 (MDSRank::active_start)`
- **Notes:** Startup cost is O(number of strays) under mds_lock. recovery_done also restarts recovered truncates and delegated-ino purges (F-STORE-072). See also F-CLU-097 (activation).

<a id="f-bg-022"></a>

#### F-BG-022 · Rank stop: export all strays to rank 0

*A stopping non-zero rank hands every stray to rank 0 before it can leave up:stopping.*

MDCache::shutdown_pass first calls shutdown_export_strays and reports "waiting for strays to migrate" while it returns false. It walks all stray dirfrags with a cursor (fetching incomplete ones), inserts each non-null stray into shutdown_exporting_strays and migrates non-purging ones with migrate_stray(dn, 0), at most MAX_EXPORTING = 100 in flight (none started while 2/3 are in flight); if rank 0 is not active it stops at the first non-purging stray and waits. Entries leave the set through shutdown_export_stray_finish from _purge_stray_logged, _truncate_stray_logged and _commit_peer_rename, and the whole set is cleared if rank 0 fails. Rank 0 never exports strays; it purges them and drains its own queue.

- **Class:** POSIX `operational` · exposure `mds-wire` · relied on by mon, cephadm, rook, admin, mds-ranks
- **Map:** [strays-and-purge.md › 3.10 Rank stop: exporting strays and draining](../cpp-map/11-background-services/strays-and-purge.md#310-rank-stop-exporting-strays-and-draining) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract) · [rank-lifecycle-and-recovery.md](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md)
- **Code:** `src/mds/MDCache.cc:8236-8351 (MDCache::shutdown_export_strays)`, `src/mds/MDCache.cc:8043-8148 (MDCache::shutdown_pass)`, `src/mds/MDCache.h:852-855 (MDCache::shutdown_export_stray_finish)`, `src/mds/MDCache.cc:3176-3178 (MDCache::handle_mds_failure)`
- **Notes:** A migration RENAME that fails (e.g. ENOSPC on rank 0's stray dirfrag) or is lost is never retried, and the walk skips inos already in the set, so unless the stray is later purged locally (not possible for a hard-linked stray) the rank stays in up:stopping until rank 0 fails (code reading). Rank 0 becomes the sink of every stopping rank's strays. The stopping state machine itself is owned by 01-cluster-membership-and-ha.

<a id="f-bg-023"></a>

#### F-BG-023 · Rank stop: drain the PurgeQueue

*A stopping rank finishes all queued purges before reporting down:stopped, with progress in the cluster log.*

After shutdown_pass returns true, MDSRankDispatcher::tick calls PurgeQueue::drain each tick; it returns true when nothing is in flight and read_pos == write_pos (or the queue is read-only). The first call lifts max_purge_ops to 0xffff and records drain_initial; while not drained the rank logs "MDS rank `<r>` waiting for purge queue (`<progress>/<total>` `<n>` files purging)" at info level to the cluster log every tick (progress and total are PurgeQueue journal bytes). The rank goes to down:stopped only after the drain completes, so reducing max_mds waits for every purge of the stopping rank.

- **Class:** POSIX `operational` · exposure `metrics` · relied on by mon, cephadm, rook, admin
- **Map:** [strays-and-purge.md › 3.8 Throttle sizing, configuration and drain](../cpp-map/11-background-services/strays-and-purge.md#38-throttle-sizing-configuration-and-drain) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract)
- **Code:** `src/mds/PurgeQueue.cc:838-881 (PurgeQueue::drain)`, `src/mds/MDSRank.cc:774-798 (MDSRankDispatcher::tick)`
- **Notes:** drain() lifts the limit only on its first call; a later OSDMap or max_mds change recomputes the normal limit while the rank is still draining.

### Stray visibility to clients and operators

What clients and administrators can observe about strays and purging inodes: error codes, namespace rules and the dump stray command.

<a id="f-bg-025"></a>

#### F-BG-025 · Purging inodes are invisible to clients (ENOENT / ESTALE)

*Once a stray is enqueued for purge, every client-facing lookup path treats it as gone.*

From enqueue (STATE_PURGING on dentry and inode) path traversal returns ENOENT for the dentry and ESTALE for the inode; LOOKUPINO, try_get_auth_inode and find_ino_peers return ESTALE; readdir and snapdiff skip the dentry; discover refuses to replicate it; and client reconnect ignores caps and snaprealms on it. Before PURGING, LOOKUPINO of a stray inode succeeds. Private inos (~mdsN, stray dirs) return ESTALE, and LOOKUPPARENT (want_parent) of an inode whose parent is a stray dir returns ESTALE.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ganesha, ceph-fuse
- **Map:** [strays-and-purge.md › 3.5 Enqueue and PurgeItem construction](../cpp-map/11-background-services/strays-and-purge.md#35-enqueue-and-purgeitem-construction) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract)
- **Code:** `src/mds/MDCache.cc:8488-8489 (MDCache::path_traverse)`, `src/mds/MDCache.cc:8633-8634 (MDCache::path_traverse)`, `src/mds/Server.cc:4450-4459 (Server::handle_client_lookup_ino)`, `src/mds/Server.cc:5959-5962 (Server::try_get_auth_inode)`, `src/mds/Server.cc:1708 (Server::handle_client_reconnect)`
- **Notes:** The kernel fh_to_dentry adds its own rule: ESTALE if nlink == 0 and the file is not open locally (linux/fs/ceph/export.c:177-208). libcephfs ll_lookup_inode returns a cached inode or whatever LOOKUPINO returns with no nlink check, so Ganesha can obtain a handle to an unlinked, not-yet-purging inode. The general LOOKUPINO handler and its error mapping are owned by 04-namespace-and-posix-operations.

<a id="f-bg-026"></a>

#### F-BG-026 · Namespace rules for stray and unlinked directories

*Rejects client operations inside the MDS private directories and creation inside directories that have been removed.*

Non-MDS requests whose parent is a system inode other than root (~mdsN, the stray dirs) fail with EROFS. Creating in, or renaming into, a directory that has been unlinked (its inode sits in a stray dir) fails with ENOENT. MDS-originated renames are checked separately: both path bases must be stray dirs (migration) or the destination must be a remote dentry of the source inode (reintegration), else EINVAL; a client rename between two hard links of one inode is a no-op returning 0.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [strays-and-purge.md › 3.2 How an inode becomes a stray](../cpp-map/11-background-services/strays-and-purge.md#32-how-an-inode-becomes-a-stray) · [strays-and-purge.md › 3.9 Hard links: remote evaluation, reintegration and migr...](../cpp-map/11-background-services/strays-and-purge.md#39-hard-links-remote-evaluation-reintegration-and-migration) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:4025-4037 (Server::rdlock_path_xlock_dentry)`, `src/mds/Server.cc:4131-4143 (Server::rdlock_two_paths_xlock_destdn)`, `src/mds/Server.cc:9373-9381 (Server::handle_client_rename)`, `src/mds/Server.cc:9405-9408 (Server::handle_client_rename)`
- **Notes:** See also F-NS-051 (rename validation).

<a id="f-bg-027"></a>

#### F-BG-027 · dump stray admin command

*Lists every stray of a rank with its prior path and client caps, for operators diagnosing space that is not reclaimed.*

asok and "ceph tell `mds.<id>` dump stray" run scan_stray_dir with a formatter: it fully loads all stray dirfrags and emits an array named "strays" in code (opened only when the first stray is found; the JSON formatter drops the name of a top-level section, so the output is a bare [...], and empty output when there are no strays) with, per primary stray, keys "ino: " (integer) and "stray_prior_path: " (the trailing ": " is part of the key names), followed by CInode::dump(DUMP_CAPS) fields: client_caps [{client_id, pending, issued, wanted, last_sent}], loner, want_loner, mds_caps_wanted and the always-dumped is_subvolume. Like the activation scan it sets STRAY/ORPHAN on what it loads, but it does not evaluate.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin, tools
- **Map:** [strays-and-purge.md › 3.13 Admin visibility](../cpp-map/11-background-services/strays-and-purge.md#313-admin-visibility) · [strays-and-purge.md › 7. Externally visible contract](../cpp-map/11-background-services/strays-and-purge.md#7-externally-visible-contract) · [admin-commands.md](../cpp-map/13-admin-and-config/admin-commands.md)
- **Code:** `src/mds/MDSDaemon.cc:592-596 (MDSDaemon::set_up_admin_socket)`, `src/mds/MDSRank.cc:3117-3123 (MDSRankDispatcher::handle_asok_command)`, `src/mds/MDCache.cc:10429-10432 (MDCache::scan_stray_dir)`, `src/mds/CInode.cc:5458-5484 (CInode::dump)`
- **Notes:** The example in doc/cephfs/scrub.rst differs from the code: it shows key "ino" with a hex string and "stray_prior_path" without the trailing ": ". The command forces a full load of all stray dirfrags. Stray and purge perf counters (mds_cache num_strays..., purge_queue pq_*) and the stray count in the MDS_CACHE_OVERSIZED text are catalogued by 11-security-admin-and-observability. Documented in troubleshooting (`ceph tell mds.<fs>:0 dump stray`). Command plane: F-ADM-047.

### Forward scrub: control, scheduling and reporting

The online, admin-triggered namespace walk (ScrubStack): commands, tags, traversal order, throttle, incremental mode, pause/abort and status output.

<a id="f-bg-028"></a>

#### F-BG-028 · scrub start command

*Starts an asynchronous online scrub from a path on rank 0 with force, recursive, repair and scrub_mdsdir options and an optional tag.*

"scrub start `<path>` [force|recursive|repair|scrub_mdsdir ...] [tag]" (asok or ceph tell `mds.<fs>:0`) fails with -EXDEV "Not rank 0" on other ranks and -EINVAL "MDS is not active" unless up:active; it hops to the MDS finisher and runs MDCache::enqueue_scrub. Paths: ~mdsdir is the executing rank's mdsdir, ~mdsN rank N's, other strings must be absolute (a relative path traverses from ino 0 and fails with -ESTALE); scrub_mdsdir is honoured only with path "/"; an empty tag becomes a random UUID marked internal. The target is resolved by internal request CEPH_MDS_OP_ENQUEUE_SCRUB (0x01503) with MDS_TRAVERSE_DISCOVER | MDS_TRAVERSE_RDLOCK_PATH, so rank 0 can start at a path another rank is auth for. The reply is {"return_code", "scrub_tag", "mode": "asynchronous"} (tag and mode only on success) with command rc always 0 once the rank/state checks pass; -EBUSY (inode already scrubbing), -EEXIST (tag active), -EAGAIN (abort in progress) and traversal errors appear only in return_code.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin
- **Legacy parts** (dead-code): (1) The registered 'scrub_path' command, which has no handler and gets -ENOSYS from the default branch. (2) The 'scrub_start' alias accepted by MDSRankDispatcher::handle_asok_command but never registered. Commit 8a2f2625799 (2019, 'Merge scrub_start and scrub start') replaced the scrub_path handler with the scrub start branch but left the scrub_path registration. The admin socket rejects an unregistered prefix with -EINVAL before any hook runs, so the scrub_start branch cannot execute. No mgr module, cephadm, Rook or qa task uses either name. Evidence: `src/mds/MDSDaemon.cc:349-355 (MDSDaemon::set_up_admin_socket)`, `src/mds/MDSRank.cc:2889-2890 (MDSRankDispatcher::handle_asok_command)`, `src/mds/MDSRank.cc:3142-3143 (MDSRankDispatcher::handle_asok_command)`, `src/common/admin_socket.cc:584-590 (AdminSocket::execute_command)`. Also: git 8a2f2625799 (mds: convert 'scrub {start,abort,pause,resume,status}').
- **Map:** [scrub-and-damage.md › 3.1 Starting a scrub](../cpp-map/11-background-services/scrub-and-damage.md#31-starting-a-scrub) · [scrub-and-damage.md › 5.2 Admin commands (asok and ceph tell mds.<id>)](../cpp-map/11-background-services/scrub-and-damage.md#52-admin-commands-asok-and-ceph-tell-mdsid) · [scrub-and-damage.md › 7. Externally visible contract](../cpp-map/11-background-services/scrub-and-damage.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSRank.cc:2889-3001 (MDSRankDispatcher::handle_asok_command)`, `src/mds/MDSRank.cc:3219 (MDSRank::command_scrub_start)`, `src/mds/MDCache.cc:13288-13356 (MDCache::enqueue_scrub)`, `src/mds/MDCache.cc:13358-13386 (MDCache::enqueue_scrub_work)`, `src/mds/ScrubStack.cc:169-208 (ScrubStack::enqueue)`
- **Notes:** Tell commands require an MDS cap that allow_all(). "scrub_path" is registered but returns -ENOSYS, and the "scrub_start" alias handled in code is not registered. An _enqueue failure after the tag was inserted leaves the tag in scrubbing_map until the completion protocol retires it. Documented operator workflow: "scrub start / recursive,repair,force" after disaster recovery (doc/cephfs/disaster-recovery-experts.rst). Command plane: F-ADM-048.

<a id="f-bg-029"></a>

#### F-BG-029 · Scrub tags and the scrub_tag object xattr

*Names each scrub for status and completion and, for user tags, stamps every scrubbed inode's first object so offline tools can skip it.*

Each scrub has a tag (user-given, or a generated UUID marked internal) and a ScrubHeader shared by everything queued under it on one rank: force, recursive, repair, scrub_mdsdir, origin ino, num_pending, epoch_last_forwarded, repaired flag and uninline counters; a tag already active returns -EEXIST. For user tags only, validation also issues a fire-and-forget setxattr "scrub_tag" = ceph-encoded std::string (u32 length + bytes) with an empty SnapContext on `<ino hex>.00000000` (file layout pool for files, metadata pool for directories), which creates the object if absent. cephfs-data-scan --filter-tag `<tag>` makes the untagged-only scans (scan_inodes, scan_frags, cleanup) skip first objects whose scrub_tag equals the tag, through the cls cephfs PGLS filter that reads OSD attr _scrub_tag.

- **Class:** POSIX `operational` · exposure `on-disk`, `admin` · relied on by admin, tools, osd
- **Map:** [scrub-and-damage.md › 3.2 Scrub header, tags and origin](../cpp-map/11-background-services/scrub-and-damage.md#32-scrub-header-tags-and-origin) · [scrub-and-damage.md › 5.5 RADOS objects and attributes touched](../cpp-map/11-background-services/scrub-and-damage.md#55-rados-objects-and-attributes-touched) · [scrub-and-damage.md › 7. Externally visible contract](../cpp-map/11-background-services/scrub-and-damage.md#7-externally-visible-contract) · [mgr-tools-and-orchestration.md](../cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md)
- **Code:** `src/mds/ScrubHeader.h:41-136 (ScrubHeader)`, `src/mds/CInode.cc:4966-4977 (ValidationContinuation::_start)`, `src/cls/cephfs/cls_cephfs.cc:129-190 (PGLSCephFSFilter)`, `src/tools/cephfs/DataScan.cc:734-746 (DataScan::forall_objects)`
- **Notes:** A setxattr on a missing first object creates an empty object carrying only the tag. See also F-STORE-063 (object 0 xattr in the object catalog), F-BG-030 (tag path).

<a id="f-bg-030"></a>

#### F-BG-030 · tag path command

*Tags every inode under a path (forced recursive scrub without repair) for use with cephfs-data-scan --filter-tag.*

"tag path `<path>` `<tag>`" calls enqueue_scrub(path, tag, force=true, recursive=true, repair=false, scrub_mdsdir=false) synchronously on the admin-socket thread: it takes mds_lock for the enqueue and then waits on a C_SaferCond with mds_lock released until the enqueue request completes, not until the scrub finishes. It requires rank 0 but, unlike scrub start, does not check that the rank is active.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin, tools, osd
- **Map:** [scrub-and-damage.md › 3.1 Starting a scrub](../cpp-map/11-background-services/scrub-and-damage.md#31-starting-a-scrub) · [scrub-and-damage.md › 5.2 Admin commands (asok and ceph tell mds.<id>)](../cpp-map/11-background-services/scrub-and-damage.md#52-admin-commands-asok-and-ceph-tell-mdsid)
- **Code:** `src/mds/MDSRank.cc:3244 (MDSRank::command_tag_path)`, `src/mds/MDSDaemon.cc:384-388 (MDSDaemon::set_up_admin_socket)`
- **Notes:** Same output convention as scrub start (rc 0, errors in return_code). Command plane: F-ADM-049.

<a id="f-bg-031"></a>

#### F-BG-031 · Scrub traversal order and scheduling (ScrubStack)

*Walks the namespace below the scrub root breadth-first, queuing dirfrags and inodes and dispatching their checks.*

ScrubStack keeps intrusive lists scrub_stack and scrub_waiting of CInode and CDir objects; each queued object holds PIN_SCRUBQUEUE, a queued dirfrag also an auth pin until its scan finishes, and an inode can be in only one scrub (-EBUSY). Dirfrags are pushed to the front and child inodes found in a dirfrag to the back (breadth-first); the root goes to the bottom for recursive scrubs, forwarded work to the top. kick_off_scrubs runs under mds_lock after every enqueue, completion, retry and resume: a file inode gets an uninline request plus asynchronous validation; a directory inode instantiates all leaf dirfrags, fetches unloaded fnode headers, skips damaged dirfrags, forwards non-auth ones, queues the rest and then validates itself concurrently with its subtree; a dirfrag is fully fetched, its dentries are checked and children enqueued, check_rstats runs and MDBalancer::maybe_fragment may queue a split or merge. Scrubbing dirfrags refuse fragmentation, PURGING inodes are not queued and snapshotted dentries are not descended into.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Legacy parts** (deprecated-admin, old-data): The CEPH_MDS_OP_UNINLINE_DATA request that kick_off_scrubs dispatches for every file inode, and the uninline_finished() call it makes for finished directory inodes. The request is the uninlining step of F-BG-048, which converts data only for inodes that carry inline data. Inline data exists only under the deprecated inline_data flag or on inodes written by older releases. Evidence: `src/mds/ScrubStack.cc:289-305 (ScrubStack::kick_off_scrubs)`, `src/mds/ScrubStack.cc:1397-1410 (ScrubStack::uninline_data)`. Also: git 8970fef4ecc and companions, merged by 29650c2e009 (PR #44359 'mds-uninline-file-during-scrub', first tagged v20.0.0).
- **Map:** [scrub-and-damage.md › 3.3 The scrub "stack" and the scheduler loop](../cpp-map/11-background-services/scrub-and-damage.md#33-the-scrub-stack-and-the-scheduler-loop) · [scrub-and-damage.md › 3.4 Directory inode: scrub_dir_inode](../cpp-map/11-background-services/scrub-and-damage.md#34-directory-inode-scrub_dir_inode) · [scrub-and-damage.md › 3.5 Dirfrag: scrub_dirfrag](../cpp-map/11-background-services/scrub-and-damage.md#35-dirfrag-scrub_dirfrag) · [scrub-and-damage.md › 4. Data structures and invariants](../cpp-map/11-background-services/scrub-and-damage.md#4-data-structures-and-invariants)
- **Code:** `src/mds/ScrubStack.cc:72-104 (ScrubStack::_enqueue)`, `src/mds/ScrubStack.cc:242-326 (ScrubStack::kick_off_scrubs)`, `src/mds/ScrubStack.cc:373-489 (ScrubStack::scrub_dir_inode)`, `src/mds/ScrubStack.cc:504-573 (ScrubStack::scrub_dirfrag)`, `src/mds/MDBalancer.cc:1296 (MDBalancer::maybe_fragment)`
- **Notes:** Breadth-first order loads and pins all inodes of every scrubbed dirfrag at once, so a scrub of a wide tree can pin a large part of the namespace; the dentry loop runs under mds_lock without heartbeat resets. Directory validation is not post-order. A non-recursive scrub of a directory still fetches all its dirfrags and checks their stats. Overlapping scrubs silently skip inodes already scrubbing under another tag (-EBUSY ignored).

<a id="f-bg-032"></a>

#### F-BG-032 · Scrub concurrency throttle

*Bounds the asynchronous scrub work in progress per rank.*

mds_max_scrub_ops_in_progress (default 5, read on every loop) caps scrubs_in_progress, which counts asynchronous inode validations plus objects parked in scrub_waiting by C_RetryScrub. Synchronous work (queuing a directory's dirfrags, scanning a complete dirfrag) and uninline requests are not counted, so one pass can process many dirfrags, and the state can become IDLE while uninline requests are still in flight. The limit is per rank and shared by all tags.

- **Class:** POSIX `operational` · exposure `config` · relied on by admin
- **Gates:** mds_max_scrub_ops_in_progress (5)
- **Map:** [scrub-and-damage.md › 3.3 The scrub "stack" and the scheduler loop](../cpp-map/11-background-services/scrub-and-damage.md#33-the-scrub-stack-and-the-scheduler-loop) · [scrub-and-damage.md › 5.6 Configuration options](../cpp-map/11-background-services/scrub-and-damage.md#56-configuration-options) · [scrub-and-damage.md › 6. Concurrency and mds_lock](../cpp-map/11-background-services/scrub-and-damage.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/ScrubStack.cc:242-326 (ScrubStack::kick_off_scrubs)`, `src/mds/ScrubStack.cc:210 (ScrubStack::add_to_waiting)`, `src/mds/ScrubStack.cc:228-240 (C_RetryScrub)`

<a id="f-bg-033"></a>

#### F-BG-033 · Incremental scrub and the force option

*Skips dentries unchanged since the dirfrag's last recursive scrub unless force is given.*

Without force, a recursive dirfrag scan skips dentries whose version is <= the dirfrag's last_recursive scrub version unless remote_d_type is DT_DIR; since remote_d_type is 0 for primary dentries, unchanged subdirectories are skipped as well. force has no other effect. last_recursive comes from CDir::scrub_info_t, initialised from fnode recursive_scrub_version/stamp, and the in-memory value drives the skip even before it is persisted.

- **Class:** POSIX `operational` · exposure `admin`, `on-disk` · relied on by admin
- **Map:** [scrub-and-damage.md › 3.5 Dirfrag: scrub_dirfrag](../cpp-map/11-background-services/scrub-and-damage.md#35-dirfrag-scrub_dirfrag) · [scrub-and-damage.md › 3.15 Scrub stamps persistence](../cpp-map/11-background-services/scrub-and-damage.md#315-scrub-stamps-persistence) · [scrub-and-damage.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/scrub-and-damage.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/ScrubStack.cc:535-541 (ScrubStack::scrub_dirfrag)`, `src/mds/CDir.cc:3812 (CDir::scrub_finished)`, `src/mds/CDentry.h:81-105 (linkage_t)`

<a id="f-bg-034"></a>

#### F-BG-034 · Scrub stamps on inodes and dirfrags

*Records when and at which version each inode and dirfrag was last scrubbed, persisted lazily.*

Inodes carry last_scrub_version/last_scrub_stamp in inode_t and dirfrags carry recursive_scrub_version/stamp and localized_scrub_version/stamp in fnode_t; scrub updates the in-memory scrub_info (CInode::scrub_info_t, CDir::scrub_info_t) and marks it dirty. project_inode and project_fnode copy dirty stamps into the next projected inode/fnode; there is no dedicated write, so stamps reach disk only with the object's next unrelated update. An inode's scrub completes (common_finished stamps it and decrements num_pending) only when both validation and uninline have finished; abort discards dirty dirfrag stamps.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal
- **Legacy parts** (deprecated-admin, old-data): The uninline half of inode scrub completion: CInode::uninline_finished() clears uninline_in_progress and calls common_finished(), which stamps the inode only when validation has also finished. uninline_in_progress is cleared only by the uninlining step of F-BG-048. That step converts data only for inodes that carry inline data, which exist only under the deprecated inline_data flag or from older releases. Evidence: `src/mds/CInode.cc:5560-5583 (CInode::common_finished, CInode::uninline_finished)`. Also: git 8970fef4ecc and companions, merged by 29650c2e009 (PR #44359 'mds-uninline-file-during-scrub', first tagged v20.0.0).
- **Map:** [scrub-and-damage.md › 3.15 Scrub stamps persistence](../cpp-map/11-background-services/scrub-and-damage.md#315-scrub-stamps-persistence) · [scrub-and-damage.md › 3.6 Non-directory inode: scrub_file_inode and uninline](../cpp-map/11-background-services/scrub-and-damage.md#36-non-directory-inode-scrub_file_inode-and-uninline) · [scrub-and-damage.md › 4. Data structures and invariants](../cpp-map/11-background-services/scrub-and-damage.md#4-data-structures-and-invariants)
- **Code:** `src/mds/CInode.cc:485-490 (CInode::project_inode)`, `src/mds/CDir.cc:1409-1416 (CDir::project_fnode)`, `src/mds/CInode.cc:5560-5583 (CInode::common_finished)`, `src/mds/CDir.h:94-111 (CDir::scrub_info_t)`

<a id="f-bg-035"></a>

#### F-BG-035 · Scrubbing ~mdsdir / ~mdsN and scrub_mdsdir (stray loading)

*Scrubs the MDS private directories, which loads stray dirfrags and is documented as the way to make the MDS evaluate strays.*

~mdsdir names the executing rank's mdsdir, which is always rank 0's because scrub start runs only there; ~mdsN names rank N's, which rank 0 opens with open_foreign_mdsdir and forwards with OP_QUEUEINO. scrub_mdsdir with path "/" pushes rank 0's own mdsdir on top and the root to the bottom; other ranks' mdsdirs are not included. doc/cephfs/scrub.rst presents "scrub start ~mdsdir recursive" as the way to evaluate strays, but ScrubStack has no stray hook: it loads the stray dirfrags without STRAY/ORPHAN flags and skips PURGING inodes, and evaluation happens only when the loaded stray dentries are trimmed from the main LRU (trim_inode -> maybe_eval_stray), i.e. under cache pressure. Uninlining is skipped when the scrub origin is an mdsdir.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin
- **Legacy parts** (deprecated-admin, old-data): Skipping uninlining (record_uninline_skipped) when the scrub origin is an mdsdir. The skip is a branch of MDCache::uninline_data_work, the uninlining step of F-BG-048. It is reached only for inodes that still carry inline data. Evidence: `src/mds/MDCache.cc:13503-13514 (MDCache::uninline_data_work)`. Also: git 8970fef4ecc and companions, merged by 29650c2e009 (PR #44359 'mds-uninline-file-during-scrub', first tagged v20.0.0).
- **Map:** [scrub-and-damage.md › 3.1 Starting a scrub](../cpp-map/11-background-services/scrub-and-damage.md#31-starting-a-scrub) · [scrub-and-damage.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/scrub-and-damage.md#8-edge-cases-failure-handling-and-gotchas) · [strays-and-purge.md › 3.13 Admin visibility](../cpp-map/11-background-services/strays-and-purge.md#313-admin-visibility)
- **Code:** `src/mds/MDCache.cc:13313-13356 (MDCache::enqueue_scrub)`, `src/mds/ScrubStack.cc:169-208 (ScrubStack::enqueue)`, `src/mds/ScrubStack.cc:80-95 (ScrubStack::_enqueue)`, `src/mds/MDCache.cc:7333-7339 (MDCache::trim_inode)`, `src/mds/MDCache.cc:8474-8476 (MDCache::path_traverse)`

<a id="f-bg-036"></a>

#### F-BG-036 · Scrub pause, resume and abort

*Lets an operator pause, resume or abort all scrubs from rank 0.*

ScrubStack has states IDLE, RUNNING, PAUSING and PAUSED plus a clear_stack abort flag. "scrub abort" (rank 0, active) broadcasts OP_ABORT, sets clear_stack and, in RUNNING or PAUSING, defers completion until scrubs_in_progress reaches 0; then abort_pending_scrubs calls scrub_aborted on every queued or parked object (auth-unpinning dirfrags) and clears remote_scrubs; enqueue returns -EAGAIN meanwhile and headers stay until the completion protocol retires them. "scrub pause" broadcasts OP_PAUSE, fails with -EINVAL during an abort, goes RUNNING/PAUSING -> PAUSING with deferred completion or straight to PAUSED; new scrub start requests are queued but not processed. "scrub resume" broadcasts OP_RESUME; PAUSING -> RUNNING (pending pause waiters get -ECANCELED), PAUSED -> RUNNING with a kick. Replies: abort and pause {"message", "return_code"} after the local drain; resume {"return_code"}.

- **Class:** POSIX `operational` · exposure `admin`, `mds-wire` · relied on by admin, mds-ranks
- **Map:** [scrub-and-damage.md › 3.13 Pause / abort / resume](../cpp-map/11-background-services/scrub-and-damage.md#313-pause--abort--resume) · [scrub-and-damage.md › 5.2 Admin commands (asok and ceph tell mds.<id>)](../cpp-map/11-background-services/scrub-and-damage.md#52-admin-commands-asok-and-ceph-tell-mdsid)
- **Code:** `src/mds/ScrubStack.cc:878 (ScrubStack::scrub_abort)`, `src/mds/ScrubStack.cc:906 (ScrubStack::scrub_pause)`, `src/mds/ScrubStack.cc:936 (ScrubStack::scrub_resume)`, `src/mds/ScrubStack.cc:836-857 (ScrubStack::abort_pending_scrubs)`, `src/mds/ScrubStack.cc:866 (ScrubStack::send_state_message)`
- **Notes:** scrub_resume() is declared bool, so its -EINVAL is reported as return_code 1. Peers receive one-shot messages without acks, and ranks that come up later are not paused. A deferred pause/abort waits for parked objects too (unfreeze, cluster recovery, remote acks). abort_pending_scrubs clears the lists without putting PIN_SCRUBQUEUE, so aborted objects stay pinned in cache (code reading). Command plane: F-ADM-048.

<a id="f-bg-037"></a>

#### F-BG-037 · Scrub status, summary and mgr task status

*Reports scrub state and active tags to operators, the cluster log and ceph status.*

"scrub status" (any rank) returns {status, scrubs, scrub_stats}: status is one of "no active scrubs running", "IDLE (waiting for more scrubs)", "scrub active (N inodes in the stack)", "ABORTING (...)", "PAUSING (...)", "PAUSED (...)", "PAUSING+ABORTING (...)", "PAUSED+ABORTING (...)" (N includes parked objects); scrubs maps each tag to path (or `#<ino>`), tag and options (comma list of recursive, repair, force, scrub_mdsdir); scrub_stats (rank 0 only) gives per tag start_time, path and uninline_started/passed/failed/skipped summed over ranks. scrub_summary ("idle", "idle+waiting", "active", "aborting", "paused", "pausing", "paused+aborting", "pausing+aborting", plus " paths [p1,p2]") goes to the cluster log as "scrub summary: ..." on every state change, enqueue and tag completion, and to the mgr as ServiceMap task-status key "scrub status" every mds_task_status_update_interval (2 s) while not idle, shown by ceph status under "task status".

- **Class:** POSIX `operational` · exposure `admin`, `metrics`, `cluster-wire` · relied on by admin, mgr
- **Legacy parts** (deprecated-admin, old-data): The scrub_stats section of scrub status: per-tag start_time, path and uninline_started/passed/failed/skipped summed over ranks. The section dumps scrub_counters_t, whose only counters are the uninline counters of the scrub-uninline feature (first tagged v20.0.0). No mgr module, cephadm or Rook reads scrub status output, and the task status sent to the mgr carries no uninline data. Evidence: `src/mds/ScrubStack.cc:797-830 (ScrubStack::scrub_status)`, `src/mds/ScrubStack.h:148-155 (scrub_counters_t)`. Also: git 9459766868a (mds: dump scrub stats during scrub status), merged by 29650c2e009 (PR #44359); searched src/pybind/mgr and rook (*.go) for scrub_stats, uninline and purge_status: no hits.
- **Gates:** mds_task_status_update_interval (2 s, dev)
- **Map:** [scrub-and-damage.md › 3.14 Status, summaries and counters](../cpp-map/11-background-services/scrub-and-damage.md#314-status-summaries-and-counters) · [scrub-and-damage.md › 5.8 Health and cluster-log messages](../cpp-map/11-background-services/scrub-and-damage.md#58-health-and-cluster-log-messages) · [scrub-and-damage.md › 7. Externally visible contract](../cpp-map/11-background-services/scrub-and-damage.md#7-externally-visible-contract)
- **Code:** `src/mds/ScrubStack.cc:715-834 (ScrubStack::scrub_status)`, `src/mds/ScrubStack.cc:661-712 (ScrubStack::scrub_summary)`, `src/mds/ScrubStack.cc:959-973 (ScrubStack::clog_scrub_summary)`, `src/mds/MDSRank.cc:4308-4346 (MDSRank::get_task_status)`
- **Notes:** QA polls status for "no active scrubs running" (qa/tasks/cephfs/filesystem.py:1883-1907). The options string omits the separator after force, so force plus scrub_mdsdir alone prints "forcescrub_mdsdir". scrub_summary() returns a string_view into a destroyed CachedStackStringStream and works only because the stream buffer is cached per thread. Command plane: F-ADM-048. See also F-ADM-085 (task status transport to the mgr).

<a id="f-bg-038"></a>

#### F-BG-038 · Scrub counters and scrub purge_status

*Keeps per-tag uninline statistics on rank 0 and per-rank scrub perf counters.*

Rank 0 creates per-tag counters at enqueue and sums uninline started/passed/failed/skipped from peers' MMDSScrubStats acks; "scrub purge_status `<tag|all>`" (rank 0) drops them, and rank 0's tick drops counters older than mds_scrub_stats_review_period days (default 1, range 1-60). Validation also bumps mds perf counters scrub_backtrace_fetch, scrub_set_tag, scrub_backtrace_repaired, scrub_inotable_repaired, scrub_dir_inodes, scrub_file_inodes, scrub_dir_base_inodes and scrub_dirfrag_rstats, plus mds_cache uninline_started/uninline_succeeded/uninline_write_failed and ireq_enqueue_scrub/ireq_flush/ireq_fragstats/ireq_inodestats.

- **Class:** POSIX `operational` · exposure `admin`, `metrics`, `config` · relied on by admin, mgr
- **Legacy parts** (deprecated-admin, old-data): The per-tag uninline counters on rank 0 (scrub_counters_t), the scrub purge_status command and mds_scrub_stats_review_period that clear them, and the mds_cache counters uninline_started, uninline_succeeded and uninline_write_failed. The per-tag counters, purge_status, the review period and the uninline perf counters arrived with the scrub-uninline feature (first tagged v20.0.0) and count only inodes that carry inline data. The mds scrub_* validation counters and the ireq_* counters are current. Evidence: `src/mds/ScrubStack.cc:120-167 (ScrubStack::purge_scrub_counters, purge_old_scrub_counters, init_scrub_counters)`, `src/mds/ScrubStack.h:148-155 (scrub_counters_t)`, `src/mds/MDCache.cc:13953-13957 (MDCache::register_perfcounters)`. Also: git 73c4541da3b (mds: add manual and auto purging scrub stats feature), merged by 29650c2e009 (PR #44359).
- **Gates:** mds_scrub_stats_review_period (1 day)
- **Map:** [scrub-and-damage.md › 3.14 Status, summaries and counters](../cpp-map/11-background-services/scrub-and-damage.md#314-status-summaries-and-counters) · [scrub-and-damage.md › 5.7 Perf counters](../cpp-map/11-background-services/scrub-and-damage.md#57-perf-counters)
- **Code:** `src/mds/ScrubStack.cc:120-167 (ScrubStack::purge_scrub_counters)`, `src/mds/MDSRank.cc:3269 (MDSRank::command_scrub_purge_status)`, `src/mds/MDSRank.cc:3814-3830 (MDSRank::create_logger)`, `src/mds/MDCache.cc:13939-13958 (MDCache::register_perfcounters)`
- **Notes:** mds_scrub_stats_review_period is read at construction and is not in get_tracked_keys(), so runtime changes are ignored. The mds scrub_* counters are registered as u64 gauges (add_u64) but incremented. Command plane: F-ADM-048.

### Scrub checks, repairs and data uninlining

What scrub compares between RADOS and memory for each object, what it can repair, the journal trim after repairs, and the conversion of inline file data.

<a id="f-bg-039"></a>

#### F-BG-039 · Backtrace validation

*Checks that each scrubbed inode's on-disk backtrace (parent xattr) matches its in-memory ancestry, the data lookup-by-ino and hard links depend on.*

For every scrubbed inode, validate_disk_state reads getxattr "parent" of `<ino hex>.00000000` in get_backtrace_pool() (file layout pool for files, metadata pool for directories), decodes it and compares it with build_backtrace of the in-memory parent chain using inode_backtrace_t::compare. It passes if equal or if memory is newer and not divergent. A read error fails unless the inode is an mdsdir or has a pending backtrace write (dirty_parent); a decode error fails unless dirty_parent; divergence or an on-disk-newer backtrace passes if (divergent and (dirty_parent or nlink == 0)) or any ancestor is a replica. An unrepaired failure records BACKTRACE damage (notify_remote_damaged, fatal result ignored) and clog warning "Scrub error on inode `<ino>` (`<path>`) see `<daemon>` log and damage ls output for details", with the validated_data JSON at derr level.

- **Class:** POSIX `operational` · exposure `on-disk`, `admin` · relied on by admin, osd
- **Map:** [scrub-and-damage.md › 3.7 Validation: CInode::validate_disk_state](../cpp-map/11-background-services/scrub-and-damage.md#37-validation-cinodevalidate_disk_state) · [scrub-and-damage.md › 3.8 Recording results: _validate_inode_done](../cpp-map/11-background-services/scrub-and-damage.md#38-recording-results-_validate_inode_done) · [scrub-and-damage.md › 3.9 What scrub checks: summary](../cpp-map/11-background-services/scrub-and-damage.md#39-what-scrub-checks-summary) · [scrub-and-damage.md › 5.5 RADOS objects and attributes touched](../cpp-map/11-background-services/scrub-and-damage.md#55-rados-objects-and-attributes-touched)
- **Code:** `src/mds/CInode.cc:4980 (ValidationContinuation::_start)`, `src/mds/CInode.cc:5003 (ValidationContinuation::_backtrace)`, `src/mds/inode_backtrace.cc:127 (inode_backtrace_t::compare)`, `src/mds/ScrubStack.cc:575-628 (ScrubStack::_validate_inode_done)`, `src/mds/CInode.cc:5585 (CInode::get_backtrace_pool)`
- **Notes:** Backtraces are what make lookup-by-ino (NFS file handles, Ganesha) and hard-link resolution work; this entry covers only the scrub check. Regular file metadata (size, layout, data objects) is not checked by scrub.

<a id="f-bg-040"></a>

#### F-BG-040 · Backtrace repair

*Rewrites a failed backtrace when scrub runs with repair.*

With repair, a failed backtrace logs clog warning "bad backtrace on inode `<ino>`(`<path>`), rewriting it", calls mark_dirty_parent on the current log segment so CInode::_store_backtrace writes parent (and layout) when that segment expires (no journal event), removes the BACKTRACE damage entry and increments scrub_backtrace_repaired. The header is marked repaired, which triggers the journal trim when the tag finishes (F-BG-041).

- **Class:** POSIX `operational` · exposure `on-disk` · relied on by admin, osd
- **Map:** [scrub-and-damage.md › 3.10 Repair mode](../cpp-map/11-background-services/scrub-and-damage.md#310-repair-mode) · [scrub-and-damage.md › 5.4 Journal events](../cpp-map/11-background-services/scrub-and-damage.md#54-journal-events)
- **Code:** `src/mds/CInode.cc:5087-5099 (ValidationContinuation::_backtrace)`, `src/mds/CInode.cc:1477 (CInode::_store_backtrace)`

<a id="f-bg-041"></a>

#### F-BG-041 · Journal trim after repairs

*Pushes repaired metadata out of the journal to RADOS when a repairing scrub tag finishes.*

When a rank retires a tag whose header saw a repair, it calls MDLog::trim_all() (trim_to(0)), which starts expiry of every segment except the newest, up to the first segment with events not yet safe, writing dirty metadata including rewritten backtraces. It is best effort: the journal is not flushed first, trim_to returns -EAGAIN (ignored) at a segment with pending events, and repairs logged in the still-current segment are not forced out. C_MDC_ScrubRepaired holds num_pending on the header while a repair request runs, so the tag does not finish before the repair completes.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [scrub-and-damage.md › 3.10 Repair mode](../cpp-map/11-background-services/scrub-and-damage.md#310-repair-mode) · [scrub-and-damage.md › 3.12.3 Completion protocol](../cpp-map/11-background-services/scrub-and-damage.md#3123-completion-protocol)
- **Code:** `src/mds/ScrubStack.cc:1231-1232 (ScrubStack::handle_scrub_stats)`, `src/mds/ScrubStack.cc:1370-1371 (ScrubStack::advance_scrub_status)`, `src/mds/MDLog.h:165 (MDLog::trim_all)`, `src/mds/MDLog.cc:855-906 (MDLog::trim_to)`, `src/mds/MDCache.cc:13585 (C_MDC_ScrubRepaired)`

<a id="f-bg-042"></a>

#### F-BG-042 · InoTable "inode wrongly marked free" check and repair

*Detects inode numbers in use that the rank's InoTable lists as free, and removes them from the free set with repair.*

After the backtrace stage, validation checks inotable->is_marked_free(ino) against the local rank's free and projected_free sets; a hit logs clog error "scrub: inode wrongly marked free: `<ino>`". With repair, InoTable::repair erases the ino from free/projected_free and bumps the version, followed by save() (write_full of `mds<rank>_inotable`), but only when projected_version == version; otherwise clog "Cannot repair inotable while other operations are in progress". Success logs "inode table repaired for inode: `<ino>`" and increments scrub_inotable_repaired.

- **Class:** POSIX `operational` · exposure `on-disk` · relied on by internal
- **Map:** [scrub-and-damage.md › 3.7 Validation: CInode::validate_disk_state](../cpp-map/11-background-services/scrub-and-damage.md#37-validation-cinodevalidate_disk_state) · [scrub-and-damage.md › 3.10 Repair mode](../cpp-map/11-background-services/scrub-and-damage.md#310-repair-mode) · [mds-tables.md](../cpp-map/07-storage-formats/mds-tables.md)
- **Code:** `src/mds/CInode.cc:5103-5127 (ValidationContinuation::_backtrace)`, `src/mds/InoTable.cc:215 (InoTable::repair)`
- **Notes:** See also F-STORE-074 (InoTable replay).

<a id="f-bg-043"></a>

#### F-BG-043 · Base inode on-disk validation (root, ~mdsN)

*Compares the persisted root and mdsdir inodes with their in-memory copies.*

For base directories validation creates a shadow system inode (create_unlinked_system_inode) and fetches the persisted base-inode object `<ino>.00000000.inode` (and the legacy inode xattr); it fails if the on-disk version is newer, else uses inode_t::compare (32 fields equal at the same version; older_is_consistent otherwise). A failure is only logged: inode.checked is set only for base inodes, which have no parent dentry, so no damage entry is created, and there is no repair. Other directories get no on-disk inode comparison, and files stop after the backtrace and InoTable checks.

- **Class:** POSIX `internal` · exposure `on-disk` · relied on by internal
- **Legacy parts** (old-data): The read of the legacy 'inode' xattr on <ino>.00000000 that CInode::fetch issues next to the <ino>.00000000.inode read, and the fallback to it in CInode::_fetched. The code comment calls the xattr form the old on-disk format. CInode::store, cephfs-data-scan and cephfs-journal-tool write only <ino>.00000000.inode objects at this commit. Evidence: `src/mds/CInode.cc:1369-1376 (CInode::fetch)`, `src/mds/CInode.cc:1381-1393 (CInode::_fetched)`, `src/mds/CInode.cc:1291 (CInode::store)`, `src/tools/cephfs/DataScan.cc:412`, `src/tools/cephfs/DataScan.cc:478`, `src/tools/cephfs/JournalTool.cc:1141`.
- **Map:** [scrub-and-damage.md › 3.7 Validation: CInode::validate_disk_state](../cpp-map/11-background-services/scrub-and-damage.md#37-validation-cinodevalidate_disk_state) · [scrub-and-damage.md › 3.8 Recording results: _validate_inode_done](../cpp-map/11-background-services/scrub-and-damage.md#38-recording-results-_validate_inode_done)
- **Code:** `src/mds/CInode.cc:5143 (ValidationContinuation::validate_directory_data)`, `src/mds/CInode.cc:5165 (ValidationContinuation::_inode_disk)`, `src/include/cephfs/types.h:1254 (inode_t::compare)`
- **Notes:** The fetch result rval is recorded but not checked: if neither the .inode object nor the legacy xattr exists, the shadow keeps its defaults (version 1) and the comparison normally passes (code reading).

<a id="f-bg-044"></a>

#### F-BG-044 · Directory dirstat/rstat validation and repair

*Checks that a directory inode's file counts and recursive stats equal the sum over its dirfrags, and forces a re-gather with repair.*

If any dirfrag of the directory is a subtree root, internal request CEPH_MDS_OP_RDLOCK_FRAGSSTATS (0x01507) first rdlocks dirfragtreelock, nestlock and filelock to force a scatter-gather. _dirfrags sums accounted_fragstat and accounted_rstat of all cached dirfrags (rsubdirs + 1 for self, rsnaps + the number of snaps in the projected srnode) and compares them with the inode dirstat/rstat via same_sums; any damaged dirfrag fails the check. With repair, CEPH_MDS_OP_REPAIR_INODESTATS (0x01505) rdlocks dirfragtreelock, wrlocks nestlock and filelock, opens or fetches every leaf (-EIO if one is damaged), sets STATE_REPAIRSTATS, marks both scatterlocks updated, drops the locks and rdlocks all three to force the gather. A mismatch is forgiven if the inode, a local auth dirfrag, or (per the last OP_QUEUEDIR_ACK) a remote dirfrag is dirty; failures only produce cluster-log warnings, never damage entries.

- **Class:** POSIX `operational` · exposure `internal` · relied on by internal
- **Map:** [scrub-and-damage.md › 3.7 Validation: CInode::validate_disk_state](../cpp-map/11-background-services/scrub-and-damage.md#37-validation-cinodevalidate_disk_state) · [scrub-and-damage.md › 3.10 Repair mode](../cpp-map/11-background-services/scrub-and-damage.md#310-repair-mode) · [scrub-and-damage.md › 5.3 Internal request op codes](../cpp-map/11-background-services/scrub-and-damage.md#53-internal-request-op-codes) · [rstats-quotas-and-subvolumes.md](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md)
- **Code:** `src/mds/CInode.cc:5192 (ValidationContinuation::check_dirfrag_rstats)`, `src/mds/CInode.cc:5201 (ValidationContinuation::_dirfrags)`, `src/mds/MDCache.cc:13704-13809 (MDCache::repair_inode_stats_work)`, `src/mds/MDCache.cc:13811-13843 (MDCache::rdlock_dirfrags_stats_work)`, `src/mds/CInode.cc:2715 (CInode::finish_scatter_gather_update)`
- **Notes:** CInode::finish_scatter_gather_update overwrites the inode dirstat/rstat with the dirfrag sums whenever all leaf dirfrags are cached, with or without repair; STATE_REPAIRSTATS only replaces the "unmatched fragstat"/"inconsistent rstat" clog errors and the mds_verify_scatter assert with a debug message. The repair's real effect is forcing the gather. REPAIR_INODESTATS returns -ESTALE when not auth and -ENOTDIR for a non-directory. These stats are what clients see as directory sizes, ceph.dir.r* vxattrs and quota usage (rstat propagation is owned by 08-snapshots-quotas-subvolumes). See also F-SNAP-041 (stat mismatch repair).

<a id="f-bg-045"></a>

#### F-BG-045 · Dirfrag fragstat/rstat validation and repair

*Checks that each dirfrag's stored stats equal the sum over its entries, and recomputes and journals them with repair.*

scrub_local calls check_rstats(true), which requires a complete, auth, unfrozen dirfrag and compares fnode fragstat and rstat with sums over head dentries (primary inode: accounted_rstat and nsubdirs/nfiles; remote: nfiles), mtime and rctime compared with <=. A mismatch without repair logs clog warning "Scrub error on dir ...". With repair, internal request CEPH_MDS_OP_REPAIR_FRAGSTATS (0x01504) auth-pins the dirfrag, rdlocks dirfragtreelock, wrlocks the directory inode's nestlock and filelock, fetches if incomplete, recomputes from projected linkages (keeping newer mtime, change_attr and rctime), marks the scatterlocks updated and journals EUpdate("repair_dirfrag"); scrub then removes all DENTRY damage entries of that dirfrag from the table.

- **Class:** POSIX `operational` · exposure `on-disk` · relied on by internal
- **Map:** [scrub-and-damage.md › 3.5 Dirfrag: scrub_dirfrag](../cpp-map/11-background-services/scrub-and-damage.md#35-dirfrag-scrub_dirfrag) · [scrub-and-damage.md › 3.10 Repair mode](../cpp-map/11-background-services/scrub-and-damage.md#310-repair-mode) · [scrub-and-damage.md › 5.4 Journal events](../cpp-map/11-background-services/scrub-and-damage.md#54-journal-events)
- **Code:** `src/mds/CDir.cc:238-314 (CDir::check_rstats)`, `src/mds/CDir.cc:3836 (CDir::scrub_local)`, `src/mds/MDCache.cc:13597-13702 (MDCache::repair_dirfrag_stats_work)`, `src/mds/ScrubStack.cc:504-573 (ScrubStack::scrub_dirfrag)`
- **Notes:** The repair does not check the damage table: if a dirfrag's full fetch fails during scrub (go_bad marks it BADFRAG and complete, and C_RetryScrub ignores the error), the scan continues with whatever dentries are cached and repair journals stats recomputed from them (code reading). Removing the DENTRY entries leaves the corrupt omap values on disk (commits are incremental); they are re-detected on the next load. Non-auth -> -ESTALE. See also F-SNAP-041 (stat mismatch repair).

<a id="f-bg-046"></a>

#### F-BG-046 · Dentry snapshot-bound check during scrub

*Flags dentries whose first/last snapshot bounds are impossible while scrubbing a dirfrag.*

In a recursive dirfrag scan, CDentry::scrub(next_seq) with next_seq = global snaprealm newest seq + 1 checks each non-projected dentry: first > next_seq or first > last calls dir->go_bad_dentry(last, name), which registers DENTRY damage (fatal for the rank's own mdsdir and stray dentries) and logs clog warning "Scrub error on dentry ... see damage ls". Staleness against the realm's snaps is computed but not acted on. Scrub cannot repair these dentries; the offline tool first-damage.py finds the first > next snap id case (not first > last).

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin, tools
- **Map:** [scrub-and-damage.md › 3.5 Dirfrag: scrub_dirfrag](../cpp-map/11-background-services/scrub-and-damage.md#35-dirfrag-scrub_dirfrag) · [scrub-and-damage.md › 3.9 What scrub checks: summary](../cpp-map/11-background-services/scrub-and-damage.md#39-what-scrub-checks-summary) · [scrub-and-damage.md › 3.17 Where damage is detected](../cpp-map/11-background-services/scrub-and-damage.md#317-where-damage-is-detected)
- **Code:** `src/mds/CDentry.cc:771-804 (CDentry::scrub)`, `src/mds/CDir.cc:2309 (CDir::go_bad_dentry)`

<a id="f-bg-047"></a>

#### F-BG-047 · Hard-link resolvability check

*Verifies during scrub that the target of each hard link can be found by inode number.*

For every remote dentry met in a recursive scan whose target ino is not in cache and not already known as damaged, identify_remote_link_damage calls open_remote_dentry (backtrace-based open_ino) fire-and-forget; the dirfrag scan does not wait, so the result can arrive after the tag finished. On failure MDCache::_open_remote_dentry_finish sets CDentry::STATE_BADREMOTEINO on the dentry and records BACKTRACE damage for the target ino (fatal result honoured). The damage is not repairable by scrub ("can identify but not repair", doc/cephfs/scrub.rst).

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin
- **Map:** [scrub-and-damage.md › 3.5 Dirfrag: scrub_dirfrag](../cpp-map/11-background-services/scrub-and-damage.md#35-dirfrag-scrub_dirfrag) · [scrub-and-damage.md › 3.9 What scrub checks: summary](../cpp-map/11-background-services/scrub-and-damage.md#39-what-scrub-checks-summary) · [scrub-and-damage.md › 3.17 Where damage is detected](../cpp-map/11-background-services/scrub-and-damage.md#317-where-damage-is-detected)
- **Code:** `src/mds/ScrubStack.cc:491-502 (ScrubStack::identify_remote_link_damage)`, `src/mds/MDCache.cc:9026-9053 (MDCache::_open_remote_dentry_finish)`
- **Notes:** See also F-CACHE-045 (hard-link resolution).

### Multi-rank scrub coordination

How rank 0 forwards scrub work to authoritative ranks and decides when a tag has finished across all ranks (MDS-to-MDS messages).

<a id="f-bg-049"></a>

#### F-BG-049 · Scrub work forwarding to authoritative ranks (MMDSScrub)

*Sends dirfrags and inodes that another rank is authoritative for to that rank and waits for its acknowledgement.*

MMDSScrub (MSG_MDS_SCRUB = 135, head 2, compat 2) encodes int32 op, inodeno_t ino, fragset_t frags, string tag, inodeno_t origin and uint32 flags (INTERNAL_TAG 1, FORCE 2, RECURSIVE 4, REPAIR 8, DIRTY_REMOTE_DIRFRAG 16). scrub_dir_inode sends one OP_QUEUEDIR per auth rank for non-auth leaf dirfrags and parks the inode; the receiver enqueues the leaf frags it is auth for on top, creating its own header for the tag, and replies OP_QUEUEDIR_ACK with the frags queued and the dirty flag; when all acks are in, the sender records queued_frags, sets epoch_last_forwarded and re-runs scrub_dir_inode, re-forwarding frags not queued. validate_inode_auth sends a non-auth inode with OP_QUEUEINO and parks it until OP_QUEUEINO_ACK, after which the receiving rank owns that inode's scrub. Ambiguous auth waits on WAIT_SINGLEAUTH and a degraded cluster on recovery.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [scrub-and-damage.md › 3.12.1 Work forwarding](../cpp-map/11-background-services/scrub-and-damage.md#3121-work-forwarding) · [scrub-and-damage.md › 5.1 Inter-MDS messages](../cpp-map/11-background-services/scrub-and-damage.md#51-inter-mds-messages) · [scrub-and-damage.md › 6. Concurrency and mds_lock](../cpp-map/11-background-services/scrub-and-damage.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/ScrubStack.cc:328-371 (ScrubStack::validate_inode_auth)`, `src/mds/ScrubStack.cc:434-455 (ScrubStack::scrub_dir_inode)`, `src/mds/ScrubStack.cc:1004-1170 (ScrubStack::handle_scrub)`, `src/messages/MMDSScrub.h:24-147 (MMDSScrub)`
- **Notes:** The dirty-remote-dirfrag flag is taken only from the last OP_QUEUEDIR_ACK to arrive. remote_scrubs allows one entry per inode (asserted, "FIXME: parallel scrubs?"), and an OP_QUEUEINO that hits -EBUSY overwrites the existing scrub's queued_frags.

<a id="f-bg-050"></a>

#### F-BG-050 · Scrub completion protocol (MMDSScrubStats epochs)

*Lets rank 0 decide that a scrub tag has finished on every rank and collect uninline results.*

Rank 0's tick (mds_tick_interval, 5 s, while up:active and the beacon is not laggy) runs advance_scrub_status, which sends MMDSScrubStats (MSG_MDS_SCRUB_STATS = 0x503, head 2, compat 1: epoch, scrubbing tags, update_scrubbing, aborting; v2 adds uninline failures by tag and errno, paths and counters) to every up rank; each replies with its locally active tags and, when it finished tags, its uninline failures and counters. A tag is active on a rank while num_pending > 0 or epoch_last_forwarded >= scrub_epoch. With more than one up rank, tags are finished only when every slot up to the highest up rank acked the current epoch and the previous epoch was fully acked too, and only tags inactive on rank 0 and reported by no rank. Finishing a tag moves all ranks' uninline failures into rank 0's DamageTable as UNINLINE_FILE entries, logs "scrub summary", erases the tag and trims the journal if repaired.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Legacy parts** (old-daemons, deprecated-admin, old-data): (1) Decoding MMDSScrubStats v1, which has no uninline payload. (2) The v2 uninline payload (failures by tag and errno, paths, counters) and its transfer into rank 0's DamageTable as UNINLINE_FILE entries. MDS daemons up to 19.2.x send MMDSScrubStats v1, and v2 arrived with the scrub-uninline feature in v20.0.0. The new MDS does not share a file system with C++ MDS daemons. The v2 payload carries results of the uninlining step of F-BG-048 only. Evidence: `src/messages/MMDSScrubStats.h:22-23 (HEAD_VERSION 2, COMPAT_VERSION 1)`, `src/messages/MMDSScrubStats.h:60-70 (MMDSScrubStats::decode_payload)`, `v19.2.7:src/messages/MMDSScrubStats.h:21 (HEAD_VERSION = 1)`, `src/mds/ScrubStack.cc:1254-1268 (ScrubStack::move_uninline_failures_to_damage_table)`. Also: git 56caef2ceb5 (mds: collect and propagate scrub status to rank 0), first tagged v20.0.0.
- **Gates:** mds_tick_interval (5 s)
- **Map:** [scrub-and-damage.md › 3.12.3 Completion protocol](../cpp-map/11-background-services/scrub-and-damage.md#3123-completion-protocol) · [scrub-and-damage.md › 3.12.2 Per-rank state](../cpp-map/11-background-services/scrub-and-damage.md#3122-per-rank-state) · [scrub-and-damage.md › 5.1 Inter-MDS messages](../cpp-map/11-background-services/scrub-and-damage.md#51-inter-mds-messages)
- **Code:** `src/mds/ScrubStack.cc:1254-1372 (ScrubStack::advance_scrub_status)`, `src/mds/ScrubStack.cc:1172-1252 (ScrubStack::handle_scrub_stats)`, `src/messages/MMDSScrubStats.h:21-188 (MMDSScrubStats)`, `src/mds/MDSRank.cc:763-766 (MDSRankDispatcher::tick)`
- **Notes:** Liveness depends on every up rank acking two consecutive epochs: a hung but up rank keeps tags listed forever, and a failed rank numbered below the highest up rank keeps a stale slot so no tag finishes until a daemon takes that rank again (code reading). scrub_epoch starts at 2; scrub_any_peer_aborting keeps the protocol running after an abort even with no tags.

<a id="f-bg-051"></a>

#### F-BG-051 · Scrub behaviour on rank failure and restart

*Defines what happens to running scrubs when a rank fails or restarts.*

MDSRank::handle_mds_failure (from up:resolve onwards) calls ScrubStack::handle_mds_failure: if rank 0 failed, local scrubs are aborted; otherwise the failed rank is removed from every remote_scrubs gather set and inodes with an empty set are unparked, re-evaluate authority, wait for recovery and re-forward. A cleanly stopped rank does not trigger this path. All scrub state (stack, tags, counters) and the damage table are in memory, so a restarted or failed-over rank 0 does not resume scrubs.

- **Class:** POSIX `operational` · exposure `internal` · relied on by admin, mds-ranks
- **Map:** [scrub-and-damage.md › 3.12.4 Rank failures](../cpp-map/11-background-services/scrub-and-damage.md#3124-rank-failures) · [scrub-and-damage.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/scrub-and-damage.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/ScrubStack.cc:1374-1395 (ScrubStack::handle_mds_failure)`

### Damage detection, DamageTable and its effects

The per-rank in-memory registry of unreadable or invalid metadata, where entries come from, which damage takes the rank down, and how damage shows up in requests, health and admin commands.

<a id="f-bg-052"></a>

#### F-BG-052 · DamageTable registry

*Per-rank in-memory list of metadata known to be unreadable or invalid, used to fail requests fast and to raise health errors.*

One DamageTable per rank, protected by mds_lock and never persisted, holds DIRFRAG entries keyed (ino, frag), DENTRY entries keyed (ino, frag) -> (dname, snapid), BACKTRACE entries keyed by ino and UNINLINE_FILE entries keyed by ino with rank, errno and scrub_tag, each with a path. Every entry gets a random 32-bit id indexed in by_id for damage ls/rm; a first report wins and later reports of the same key change nothing. Entries are removed only by damage rm, scrub backtrace repair and scrub dirfrag-stats repair; after a restart or failover the table is empty and damage is rediscovered on the next access or scrub. The size is bounded by mds_damage_table_max_entries (10000); beyond it every report is fatal (F-BG-057).

- **Class:** POSIX `operational` · exposure `config` · relied on by admin, mon, internal
- **Legacy parts** (deprecated-admin, old-data): UNINLINE_FILE entries (UninlineDamage, DamageTable::notify_uninline_failed). ScrubStack::move_uninline_failures_to_damage_table is the only caller of notify_uninline_failed. The entries record failures of the uninlining step of F-BG-048 only. Evidence: `src/mds/DamageTable.h:41 (DAMAGE_ENTRY_UNINLINE_FILE)`, `src/mds/DamageTable.cc:131-160 (UninlineDamage)`, `src/mds/DamageTable.cc:284-308 (DamageTable::notify_uninline_failed)`, `src/mds/ScrubStack.cc:1265 (ScrubStack::move_uninline_failures_to_damage_table)`.
- **Gates:** mds_damage_table_max_entries (10000)
- **Map:** [scrub-and-damage.md › 3.16 DamageTable](../cpp-map/11-background-services/scrub-and-damage.md#316-damagetable) · [scrub-and-damage.md › 4. Data structures and invariants](../cpp-map/11-background-services/scrub-and-damage.md#4-data-structures-and-invariants)
- **Code:** `src/mds/DamageTable.h:125 (DamageTable)`, `src/mds/DamageTable.cc:168 (DamageEntry::DamageEntry)`, `src/mds/DamageTable.cc:175-238 (DamageTable::notify_dentry)`, `src/mds/DamageTable.cc:284-308 (DamageTable::notify_uninline_failed)`
- **Notes:** Random ids can collide (about 1% at 10000 entries); a colliding insert overwrites the older by_id entry while it stays in its type index. remove_dirfrag_damage_entry has no caller. Inode/fnode damage_flags (DAMAGE_STATS, DAMAGE_RSTATS, DAMAGE_FRAGTREE) are a separate persisted field written by cephfs-data-scan that the MDS only encodes, decodes and dumps; the enumerators are 0, 1, 2 but OR-ed as bit masks.

<a id="f-bg-053"></a>

#### F-BG-053 · Dirfrag load damage detection

*Marks a directory fragment or a single entry damaged when its RADOS object is missing or does not decode.*

When a dirfrag omap read returns an empty header (object missing), CDir::_omap_fetched logs clog error "dir `<ino>` object missing on disk; some files may be lost" and calls go_bad; an undecodable fnode or trailing bytes after it log a warning and call go_bad. go_bad registers DIR_FRAG damage and, for a full fetch, gives the dirfrag a synthetic version-1 fnode if it had none, sets STATE_BADFRAG (which also prevents fragmentation) and marks it complete, then completes WAIT_COMPLETE waiters with -EIO. An undecodable dentry value (including an unknown marker) logs "Corrupt dentry ..." and registers DENTRY damage via go_bad_dentry while the rest of the dirfrag loads. Any other read error asserts (r must be 0, -ENOENT or -ENODATA); -EBLOCKLISTED and -ETIMEDOUT respawn earlier in MDSIOContextBase::complete.

- **Class:** POSIX `supporting` · exposure `metrics` · relied on by admin, mon, internal
- **Map:** [scrub-and-damage.md › 3.17 Where damage is detected](../cpp-map/11-background-services/scrub-and-damage.md#317-where-damage-is-detected) · [scrub-and-damage.md › 3.18 How damage affects request handling](../cpp-map/11-background-services/scrub-and-damage.md#318-how-damage-affects-request-handling)
- **Code:** `src/mds/CDir.cc:2056-2097 (CDir::_omap_fetched)`, `src/mds/CDir.cc:2204-2224 (CDir::_omap_fetched)`, `src/mds/CDir.cc:2309 (CDir::go_bad_dentry)`, `src/mds/CDir.cc:2323-2347 (CDir::go_bad)`, `src/mds/mdstypes.cc:795 (dentry_key_t::decode_helper)`
- **Notes:** With the non-default mds_dir_prefetch = false, a keyed dentry fetch that hits a missing object or corrupt fnode calls go_bad(false), which wakes only WAIT_COMPLETE, so requests parked in waiting_on_dentry are not resumed (code reading). Damage to the rank's own mdsdir dirfrag is not fatal and is not recreated: the populate_mydir recreate branch needs version 0 together with BADFRAG, which go_bad never produces (code reading). An omap key without "_" trips an assert in dentry_key_t::decode_helper, called outside the try block. go_bad on a fresh dir sets fnode version 1 without raising projected_version (impact unverified). See also F-STORE-051 (dirfrag fetch), F-BG-056 (duplicate primary inode), F-BG-060 (EIO to requests).

<a id="f-bg-054"></a>

#### F-BG-054 · Dentry snapshot-bound corruption check at load and commit

*Catches dentries with impossible first/last snapshot ids whenever they are added to cache or about to be written.*

CDentry::check_corruption runs with load=true on every CDir::add_null_dentry, add_primary_dentry and add_remote_dentry (fetch, replay, creation, replicas) and with load=false in CDir::_omap_commit before writing a dentry not already flagged corrupt_first_loaded. It flags first > last, or first > snapclient last_seq + 1 when the snap client is synced. If mds_go_bad_corrupt_dentry (default true) the dentry is registered as DENTRY damage via go_bad_dentry; before a commit, if mds_abort_on_newly_corrupt_dentry (default true), the MDS logs clog error "MDS abort because newly corrupt dentry to be committed: ..." and calls MDSRank::abort.

- **Class:** POSIX `operational` · exposure `config`, `metrics` · relied on by admin, tools
- **Gates:** mds_go_bad_corrupt_dentry (true, runtime); mds_abort_on_newly_corrupt_dentry (true)
- **Map:** [scrub-and-damage.md › 3.17 Where damage is detected](../cpp-map/11-background-services/scrub-and-damage.md#317-where-damage-is-detected) · [scrub-and-damage.md › 5.6 Configuration options](../cpp-map/11-background-services/scrub-and-damage.md#56-configuration-options)
- **Code:** `src/mds/CDentry.cc:806-827 (CDentry::check_corruption)`, `src/mds/CDir.cc:391 (CDir::add_null_dentry)`, `src/mds/CDir.cc:2646-2648 (CDir::_omap_commit)`, `src/mds/CDentry.cc:771 (CDentry::scrub)`
- **Notes:** CDentry::scrub applies the same checks (F-BG-046). The offline tool first-damage.py targets such dentries in the C++ on-disk format.

<a id="f-bg-056"></a>

#### F-BG-056 · Duplicate primary inode detection on dirfrag load

*Refuses to link a second primary dentry for an inode that is already cached under another name.*

When a loaded primary dentry names an ino that is already in cache under another dentry, CDir::_load_dentry logs clog error "loaded dup inode `<ino>` ... already exists at ..." and does not link it, so the name is absent from lookups and listings. No damage-table entry is created, so only the cluster log shows the loss. mds_hack_allow_loading_invalid_metadata (default false, test aid) links it anyway.

- **Class:** POSIX `supporting` · exposure `metrics` · relied on by kclient, libcephfs, admin
- **Gates:** mds_hack_allow_loading_invalid_metadata (false)
- **Map:** [scrub-and-damage.md › 3.17 Where damage is detected](../cpp-map/11-background-services/scrub-and-damage.md#317-where-damage-is-detected)
- **Code:** `src/mds/CDir.cc:2029-2044 (CDir::_load_dentry)`

<a id="f-bg-057"></a>

#### F-BG-057 · Fatal damage takes the rank down:damaged

*Damage to a rank's own stray structures or the root directory, or table overflow, stops the rank until an operator repairs it.*

notify_dentry is fatal for dentries in the rank's own mdsdir or stray dirs; notify_dirfrag for frags of the rank's own stray dirs or of the root directory (on any rank); every notify_* when by_id.size() > mds_damage_table_max_entries, checked before the duplicate test so even a repeat report is fatal once over the limit. CDir::go_bad, CDir::go_bad_dentry (and so scrub's dentry check) and MDCache::_open_remote_dentry_finish honour the result by calling MDSRank::damaged(), which sets want-state down:damaged, flushes the cluster log, sends a beacon and waits up to mds_mon_shutdown_timeout (5 s) for the ack under mds_lock, then respawns. No daemon takes a damaged rank until an operator fixes the metadata and runs "ceph mds repaired"; its metadata is unavailable meanwhile. Scrub's backtrace results and the uninline failures ignore the fatal result, so on overflow those reports are dropped silently.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, admin
- **Gates:** mds_damage_table_max_entries (10000); mds_mon_shutdown_timeout (5 s); mon command: ceph mds repaired
- **Map:** [scrub-and-damage.md › 3.16 DamageTable](../cpp-map/11-background-services/scrub-and-damage.md#316-damagetable) · [scrub-and-damage.md › 3.17 Where damage is detected](../cpp-map/11-background-services/scrub-and-damage.md#317-where-damage-is-detected) · [scrub-and-damage.md › 3.18 How damage affects request handling](../cpp-map/11-background-services/scrub-and-damage.md#318-how-damage-affects-request-handling) · [rank-lifecycle-and-recovery.md](../cpp-map/09-recovery/rank-lifecycle-and-recovery.md) · [beacon-and-monitor.md](../cpp-map/01-cluster-integration/beacon-and-monitor.md)
- **Code:** `src/mds/DamageTable.cc:181-189 (DamageTable::notify_dentry)`, `src/mds/DamageTable.cc:214-219 (DamageTable::notify_dirfrag)`, `src/mds/DamageTable.cc:308 (DamageTable::oversized)`, `src/mds/MDSRank.cc:945 (MDSRank::damaged)`, `src/mds/CDir.cc:2323 (CDir::go_bad)`
- **Notes:** Damage to the rank's own mdsdir dirfrag is not fatal. Other MDSRank::damaged() triggers (unloadable tables, sessionmap or journal, root open failure, ...) and the down:damaged rank state are owned by 01-cluster-membership-and-ha.

<a id="f-bg-059"></a>

#### F-BG-059 · damage ls and damage rm commands

*Lets operators list a rank's damage entries and clear them after fixing the metadata.*

"damage ls" (asok or tell, any rank, per-rank table) dumps a JSON array of entries with damage_type (dir_frag, dentry, backtrace, uninline), id, ino and type-specific fields: frag; frag, dname and snap_id; rank, errno (string) and scrub_tag; plus path. "damage rm `<damage_id>`" erases by id; a missing id is a silent no-op and a missing argument returns -EINVAL. Only the table entry is removed: STATE_BADFRAG and STATE_BADREMOTEINO stay on cached objects, so after rm a BADFRAG dirfrag (cached complete and normally empty) gives -ENOENT instead of -EIO until it is trimmed and refetched, and committing it would hit the dirfrag stat guard and turn the rank read-only if the object is missing.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin, tools, mon, rook
- **Legacy parts** (deprecated-admin, old-data): The 'uninline' damage_type in damage ls with its rank, errno and scrub_tag fields, and the note on its errno value. Entries of this type are created only from scrub uninline failures (F-BG-052). The other damage types are current. Evidence: `src/mds/DamageTable.cc:151-160 (UninlineDamage::dump)`, `src/mds/ScrubStack.cc:1265 (ScrubStack::move_uninline_failures_to_damage_table)`.
- **Map:** [scrub-and-damage.md › 3.16 DamageTable](../cpp-map/11-background-services/scrub-and-damage.md#316-damagetable) · [scrub-and-damage.md › 5.2 Admin commands (asok and ceph tell mds.<id>)](../cpp-map/11-background-services/scrub-and-damage.md#52-admin-commands-asok-and-ceph-tell-mdsid) · [scrub-and-damage.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/scrub-and-damage.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDSRank.cc:3103-3113 (MDSRankDispatcher::handle_asok_command)`, `src/mds/DamageTable.cc:343 (DamageTable::dump)`, `src/mds/DamageTable.cc:353-369 (DamageTable::erase)`, `src/mds/MDSDaemon.cc:484-492 (MDSDaemon::set_up_admin_socket)`
- **Notes:** reported_at is recorded but not dumped. Erasing one DENTRY entry erases the whole per-dirfrag dentry map from the lookup index while the other ids stay listed. UNINLINE_FILE entries show the global errno rather than the failure errno (code reading). Damage ids are random 32-bit values; an unknown id in damage rm returns rc 0. The table is not persisted, so it is empty after a restart or failover. Documented operator procedure: `ceph tell mds.<id> damage ls|rm` (damaged-rank.rst). Command plane: F-ADM-050.

<a id="f-bg-060"></a>

#### F-BG-060 · EIO for requests that touch known damage

*Fails client requests fast with EIO when their path crosses metadata in the damage table.*

MDCache::path_traverse returns -EIO when it reaches a dirfrag in the table (before fetching it or when looking up the next component), when the next component is a dentry in the table (exact (dirfrag, name, snapid) match), and when it follows an uncached remote dentry whose target ino has BACKTRACE damage. Requests waiting on a full dirfrag fetch that turns out damaged are woken with -EIO; C_MDS_RetryRequest ignores the code, so the request retries and hits these checks. Damage entries never put the MDS into read-only mode.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [scrub-and-damage.md › 3.18 How damage affects request handling](../cpp-map/11-background-services/scrub-and-damage.md#318-how-damage-affects-request-handling) · [scrub-and-damage.md › 7. Externally visible contract](../cpp-map/11-background-services/scrub-and-damage.md#7-externally-visible-contract)
- **Code:** `src/mds/MDCache.cc:8589-8593 (MDCache::path_traverse)`, `src/mds/MDCache.cc:8622-8628 (MDCache::path_traverse)`, `src/mds/MDCache.cc:8705-8712 (MDCache::path_traverse)`, `src/mds/MDCache.cc:8804-8810 (MDCache::path_traverse)`, `src/mds/CDir.cc:2346 (CDir::go_bad)`
- **Notes:** path_traverse does not fetch a dirfrag recorded as damaged. See also F-CACHE-041 (path traversal).

<a id="f-bg-061"></a>

#### F-BG-061 · readdir behaviour on damaged directories

*Lists what can be listed of a damaged directory without returning an error.*

handle_client_readdir and try_open_auth_dirfrag do not consult the damage table. A BADFRAG dirfrag is marked complete, so readdir succeeds and lists only the dentries that happened to be cached already (normally none). A dentry whose value failed to decode is simply absent, and a remote dentry with STATE_BADREMOTEINO is skipped. readdir never returns EIO for damage.

- **Class:** POSIX `required` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [scrub-and-damage.md › 3.18 How damage affects request handling](../cpp-map/11-background-services/scrub-and-damage.md#318-how-damage-affects-request-handling) · [scrub-and-damage.md › 7. Externally visible contract](../cpp-map/11-background-services/scrub-and-damage.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:5165-5184 (Server::handle_client_readdir)`, `src/mds/Server.cc:5264-5270 (Server::handle_client_readdir)`, `src/mds/CDir.cc:2213-2223 (CDir::_omap_fetched)`, `src/mds/Server.cc:4231 (Server::try_open_auth_dirfrag)`
- **Notes:** Established by code reading.

### Legacy entries

These entries serve only clients older than the [baseline](../README.md#baseline-current-peers), msgr1 mounts or older C++ releases, or describe code that cannot run. Each keeps its ID and group.

<a id="f-bg-048"></a>

#### F-BG-048 · Data uninlining during scrub

*Converts inline file data into a RADOS object for every scrubbed file, revoking client caps on each file it visits.*

Every scrubbed non-directory inode gets internal request CEPH_MDS_OP_UNINLINE_DATA (0x0150b, no early reply), which acquires xlocks on authlock, filelock and versionlock before testing for inline data, so every scrubbed file goes through exclusive lock states and conflicting client caps are revoked. Inline data exists only on inodes created while the inline_data fs flag was set by a client with CEPH_FEATURE_MDS_INLINE_DATA. For such an inode the MDS issues create(non-exclusive), then cmpxattr("inline_version", GT, U64) + write(0, inline data, truncate_size, truncate_seq) + setxattr("inline_version", "18446744073709551615") on `<ino hex>.00000000` in the layout pool with the realm snapc, and on success journals EUpdate("uninline") with inline_data.version = CEPH_INLINE_NONE, ctime and change_attr bumped and parents predirtied. Write failures are aggregated per tag and errno and become UNINLINE_FILE damage entries in rank 0's table when the tag finishes. Uninline requests are not counted by mds_max_scrub_ops_in_progress.

- **Class:** POSIX `extension` · exposure `on-disk`, `client-wire` · relied on by kclient, libcephfs, osd
- **Group:** Scrub checks, repairs and data uninlining
- **Legacy** (deprecated-admin, old-data): Inline data exists only on inodes created while the inline_data flag was set, which the monitor marks deprecated and which no mgr module, cephadm or Rook sets, or on inodes written by older releases, which the new MDS never reads. The step was added to move such data into RADOS objects (commit 8970fef4ecc, PR #44359, first tagged v20.0.0); at this commit it runs for every scrubbed file and takes the authlock, filelock and versionlock xlocks before it tests for inline data. Passing mentions of the step also appear in F-BG-029, F-BG-032 and F-BG-057. Evidence: `src/mds/MDCache.cc:13487-13516 (MDCache::uninline_data_work)`, `src/mds/ScrubStack.cc:289-295 (ScrubStack::kick_off_scrubs)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mon/FSCommands.cc:471-478 (inline_data: --yes-i-really-really-mean-it, 'deprecated')`, `src/mds/MDSMap.cc:616-622 (FS_INLINE_DATA_DEPRECATED)`, `doc/cephfs/experimental-features.rst:24-28`. Also: git 8970fef4ecc and companions, merged by 29650c2e009 (PR #44359 'mds-uninline-file-during-scrub', first tagged v20.0.0); searched src/pybind/mgr, src/cephadm and rook (*.go) for inline_data/uninline: the only Rook hits are RGW zone placement.
- **Gates:** fs flag inline_data; CEPH_FEATURE_MDS_INLINE_DATA
- **Map:** [scrub-and-damage.md › 3.11 Data uninlining](../cpp-map/11-background-services/scrub-and-damage.md#311-data-uninlining) · [scrub-and-damage.md › 5.5 RADOS objects and attributes touched](../cpp-map/11-background-services/scrub-and-damage.md#55-rados-objects-and-attributes-touched) · [scrub-and-damage.md › 7. Externally visible contract](../cpp-map/11-background-services/scrub-and-damage.md#7-externally-visible-contract) · [scrub-and-damage.md › 8. Edge cases, failure handling and gotchas](../cpp-map/11-background-services/scrub-and-damage.md#8-edge-cases-failure-handling-and-gotchas) · [lock-types-and-state-machines.md](../cpp-map/05-locking/lock-types-and-state-machines.md)
- **Code:** `src/mds/ScrubStack.cc:1397-1417 (ScrubStack::uninline_data)`, `src/mds/MDCache.cc:13487 (MDCache::uninline_data_work)`, `src/mds/MDCache.cc:13429 (C_IO_DataUninlined::finish)`, `src/mds/MDCache.cc:13395 (C_MDC_DataUninlinedSubmitted::finish)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`
- **Notes:** Bugs by code reading: the cmpxattr operand is an encoded InodeStoreBase, not a u64 (the OSD reads its first 8 bytes, about 4.4e17), so the guard fails with -ECANCELED against the u64::MAX written by an earlier MDS uninline and is recorded as a failure; when the origin is an mdsdir and the file has inline data, uninline_data_work responds without uninline_finished() or auth_unpin(), so the tag never finishes; notify_uninline_failed stores the global errno instead of its argument. Clients use the same inline_version protocol (kclient ceph_uninline_data with cpu_to_le64, libcephfs Client::uninline_data with encode(u64)) and write the decimal of their own version; the kernel treats -ECANCELED as success. In the xlock states clients keep only Fc/Fb and no As/Ax. Inline data is deprecated since Octopus. See also F-STORE-067 (RADOS ops of uninlining).

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-bg-024"></a>F-BG-024 | [F-STORE-072](06-journal-and-storage.md#f-store-072) · Release of preallocated and delegated numbers at session close (EPurged) |
| <a id="f-bg-055"></a>F-BG-055 | [F-STORE-061](06-journal-and-storage.md#f-store-061) · Directory backtrace verification on fetch |
| <a id="f-bg-058"></a>F-BG-058 | [F-ADM-078](11-security-admin-and-observability.md#f-adm-078) · MDS_DAMAGE and MDS_READ_ONLY state warnings |

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
