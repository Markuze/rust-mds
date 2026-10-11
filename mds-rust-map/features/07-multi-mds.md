# Multiple active MDS ranks: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [subtree-migration.md](../cpp-map/08-multi-mds/subtree-migration.md) · [balancer-and-pinning.md](../cpp-map/08-multi-mds/balancer-and-pinning.md)

## 1. Scope

This category covers running several active MDS ranks in one file system: the subtree export/import protocol (Migrator) that moves authority over dirfrag subtrees between ranks, including its client-visible effects (cap EXPORT/IMPORT messages, the FLUSHMSG barrier, sessions opened on the importer without a client request, request blocking and MClientRequestForward, MDSMap export_targets) and its journaling and failure handling; popularity accounting and the default heartbeat-driven balancer, bal_rank_mask and the balance_automate flag; the Mantle Lua balancer; export pins (ceph.dir.pin) and ephemeral pins (ceph.dir.pin.distributed, ceph.dir.pin.random), whose mgr/volumes pin commands are catalogued in 11-security-admin-and-observability; subtree hand-off by a stopping rank; and hot-directory replication (MDirUpdate). Sources: [subtree migration](../cpp-map/08-multi-mds/subtree-migration.md) and [balancer and pinning](../cpp-map/08-multi-mds/balancer-and-pinning.md).

Exposure summary: the MDS-to-MDS messages (MExportDir*, MExportCaps/Ack, MGatherCaps, MHeartbeat, MDirUpdate) are exchanged only between ranks and no other peer decodes them; unmodified clients see cap EXPORT/IMPORT, FLUSHMSG, unsolicited CEPH_SESSION_OPEN from the importer, request forwards, DirStat auth/dist changes, the MDSMap export_targets list and the ceph.dir.pin* vxattrs; the monitor receives MMDSLoadTargets and owns balance_automate, bal_rank_mask, balancer and max_mds; the mgr volumes module sets pins through libcephfs; OSDs see only the Mantle object read and the journal/dirfrag I/O.

Excluded and owned elsewhere: the subtree map, path traversal, replica/expire primitives and dirfrag split/merge including the balancer's fragmentation triggers and the mds_bal_fragment_size_max ENOSPC limit (05-metadata-cache-and-dirfrags); the resolve/rejoin recovery stages and the stopping state (01-cluster-membership-and-ha); cross-rank rename's single-inode migration (04-namespace-and-posix-operations); session and cap message formats (02-client-sessions-and-protocol, 03-caps-leases-and-locking); journal event framework (06-journal-and-storage); quiesce and quarantine (10-qos-metrics-quiesce-quarantine); the admin commands export dir, dump_export_states, get subtrees and dump loads, the migration and balancer perf counters, and kill-point/thrash test options (11-security-admin-and-observability).

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

49 features: 49 used by current peers (4 of them with legacy parts) and 0 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**Subtree migration protocol (MDS-to-MDS)**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-001](#f-multi-001) | Subtree authority hand-off protocol (export/import state machines) *(legacy parts)* | internal | mds-wire | mds-ranks, internal |
| [F-MULTI-002](#f-multi-002) | Export preconditions and silent refusals | internal | internal | internal, admin |
| [F-MULTI-003](#f-multi-003) | Subtree freeze and auth-pin drain | internal | internal | internal |
| [F-MULTI-004](#f-multi-004) | Lock set held during migration | internal | internal | internal, mds-ranks |
| [F-MULTI-005](#f-multi-005) | Prep phase: spanning-tree replication and bound opening | internal | mds-wire | mds-ranks, internal |
| [F-MULTI-006](#f-multi-006) | Metadata and cap transfer encoding (MExportDir) | internal | mds-wire | mds-ranks, internal |
| [F-MULTI-007](#f-multi-007) | Bystander warning and notify | internal | mds-wire | mds-ranks, internal |
| [F-MULTI-008](#f-multi-008) | Cache-expire routing during migration | internal | mds-wire | mds-ranks, internal |
| [F-MULTI-009](#f-multi-009) | Empty import return | internal | internal | internal |
| [F-MULTI-010](#f-multi-010) | Export queue and in-flight size throttle | internal | config | internal |
| [F-MULTI-011](#f-multi-011) | Export splitting of large subtrees | internal | config | internal |

**Client-visible effects of migration**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-012](#f-multi-012) | Cap EXPORT message to clients (CEPH_CAP_OP_EXPORT) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-MULTI-013](#f-multi-013) | Cap IMPORT message to clients and auth-cap switch (CEPH_CAP_OP_IMPORT) *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-MULTI-014](#f-multi-014) | Migrate sequence (mseq) and stale cap-message handling | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-MULTI-015](#f-multi-015) | Writeable-range reconciliation on cap import | supporting | internal | kclient, libcephfs, ceph-fuse, ganesha |
| [F-MULTI-016](#f-multi-016) | Client session flush barrier before shipping (FLUSHMSG) *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-MULTI-017](#f-multi-017) | Forced client session open on the importer | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-MULTI-018](#f-multi-018) | Request blocking and forwarding around migration | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-MULTI-019](#f-multi-019) | Export-target publication (MMDSLoadTargets, MDSMap export_targets) | supporting | cluster-wire, client-wire | mon, kclient, libcephfs, ceph-fuse, ganesha, admin, mds-ranks |
| [F-MULTI-020](#f-multi-020) | Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps) | supporting | mds-wire, client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |

**Migration durability, cancellation and failure handling**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-021](#f-multi-021) | Migration journaling (EImportStart, EExport, EImportFinish) | supporting | on-disk | internal, tools |
| [F-MULTI-022](#f-multi-022) | Export cancellation and reversal | internal | mds-wire, on-disk | mds-ranks, internal |
| [F-MULTI-023](#f-multi-023) | Rank failure during migration and resolve interplay | internal | mds-wire, on-disk | mds-ranks, internal |
| [F-MULTI-024](#f-multi-024) | Freeze deadlock breaker (mds_freeze_tree_timeout) | internal | config | internal, mds-ranks |
| [F-MULTI-025](#f-multi-025) | Quiesce and quarantine guards on migration | internal | internal | internal, mgr |

**Load accounting**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-026](#f-multi-026) | Decayed popularity counters (DecayCounter, meta_load) | internal | config | internal |
| [F-MULTI-027](#f-multi-027) | Popularity hits on metadata operations | internal | internal | internal |
| [F-MULTI-028](#f-multi-028) | Popularity transfer on export, import, subtree change, rename and split | internal | mds-wire | internal, mds-ranks |
| [F-MULTI-029](#f-multi-029) | Per-rank load (mds_load_t) and mds_bal_mode | internal | config, metrics | internal |

**Default balancer**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-030](#f-multi-030) | balance_automate file-system flag | operational | cluster-wire, admin | mon, admin |
| [F-MULTI-031](#f-multi-031) | Heartbeat epoch exchange (MHeartbeat) | operational | mds-wire, config | mds-ranks, internal |
| [F-MULTI-032](#f-multi-032) | Default rebalance target computation (prep_rebalance) | operational | config | internal |
| [F-MULTI-033](#f-multi-033) | Export selection (try_rebalance, find_exports) | operational | config | internal |
| [F-MULTI-034](#f-multi-034) | Idle import return (mds_bal_idle_threshold) | operational | config | admin |
| [F-MULTI-035](#f-multi-035) | bal_rank_mask (balancer target ranks) | operational | cluster-wire, admin | mon, admin |

**Mantle Lua balancer**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-036](#f-multi-036) | Mantle Lua balancer policy | operational | cluster-wire, admin, config | mon, osd, admin |
| [F-MULTI-037](#f-multi-037) | Mantle cluster log messages | operational | admin | admin, tools |

**Export pins (ceph.dir.pin)**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-038](#f-multi-038) | Set and remove ceph.dir.pin | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, mgr, rook |
| [F-MULTI-039](#f-multi-039) | Pin read-back (InodeStat dir_pin, GETVXATTR) *(legacy parts)* | extension | client-wire | kclient, libcephfs, ceph-fuse |
| [F-MULTI-040](#f-multi-040) | Pin inheritance and precedence resolution | extension | internal | kclient, libcephfs, ceph-fuse, mgr, rook, admin |
| [F-MULTI-041](#f-multi-041) | Pin enforcement and aux subtrees (handle_export_pins) | extension | internal | kclient, libcephfs, ceph-fuse, mgr, rook, admin |
| [F-MULTI-042](#f-multi-042) | Delayed pins (target rank >= max_mds) | extension | internal | admin |
| [F-MULTI-043](#f-multi-043) | Pins veto balancer exports | operational | internal | admin |

**Ephemeral pins**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-045](#f-multi-045) | Distributed ephemeral pins (ceph.dir.pin.distributed) | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, mgr, rook |
| [F-MULTI-046](#f-multi-046) | Random ephemeral pins (ceph.dir.pin.random) | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, mgr |
| [F-MULTI-047](#f-multi-047) | Consistent hash into a rank bucket (hash_into_rank_bucket) | internal | internal | internal |
| [F-MULTI-048](#f-multi-048) | Ephemeral pin redistribution on max_mds and config changes | extension | config | mon, admin |

**Rank shrink and hot-directory replication**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-MULTI-049](#f-multi-049) | Subtree hand-off by a stopping rank (max_mds decrease) | operational | mds-wire, admin | mon, admin, mds-ranks |
| [F-MULTI-050](#f-multi-050) | Hot-directory replication (REP_ALL, MDirUpdate) | operational | mds-wire, client-wire, config | kclient, mds-ranks |

## 3. Features

### Subtree migration protocol (MDS-to-MDS)

The two-party export/import protocol that moves authority over a dirfrag subtree between ranks, and the internal mechanisms it uses (freeze, locks, prep, transfer, bystanders, expires, throttling). All messages here are MDS-to-MDS (port MDS_PORT_MIGRATOR 0x400) and are not decoded by any non-MDS peer.

<a id="f-multi-001"></a>

#### F-MULTI-001 · Subtree authority hand-off protocol (export/import state machines)

*Moves authority over one dirfrag and everything nested below it (down to nested subtree roots, the bounds) from an exporter rank to an importer rank; the only way authority changes between live ranks.*

The exporter runs EXPORT_LOCKING, DISCOVERING, FREEZING, PREPPING, WARNING, EXPORTING, LOGGINGFINISH, NOTIFYING (plus CANCELLING/CANCELLED); the importer runs IMPORT_DISCOVERING, DISCOVERED, PREPPING, PREPPED, LOGGINGSTART, ACKING, FINISHING, ABORTING. Messages (port MDS_PORT_MIGRATOR 0x400, accepted only from MDS peers, tid of the internal CEPH_MDS_OP_EXPORTDIR 0x01501 request in the header): MExportDirDiscover/Ack 0x449/0x450, Cancel 0x451, Prep/PrepAck 0x452/0x453, MExportDir/Ack 0x456/0x457, Notify/NotifyAck 0x458/0x459, Finish 0x460 (bool last); 0x454/0x455 are defined but unused. The dir_auth pair goes (E,-2) -> (E,E) when frozen -> (E,I) at EXPORTING; the importer sets (I,E) when it decodes MExportDir, (I,I) after cap import and (I,-2) at unfreeze; a pair with a second value is ambiguous and requests on such objects wait. Only the dirfrag moves: the base directory inode stays with its own auth, and the bounds are not transferred. At most one export and one import exist per dirfrag (asserted); without bystanders the importer receives only MExportDirFinish(last=true).

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks, internal
- **Legacy parts** (dead-code): Unused message ids 0x454 and 0x455. MSG_MDS_EXPORTDIRWARNING (0x454) and MSG_MDS_EXPORTDIRWARNINGACK (0x455) exist only as definitions in Message.h; their message classes were removed in 2010 (commit 7d23bb4e3a6, 'msg: Remove dead MExportDirWarning/Ack') and a grep of src/ finds no decoder or handler that references the ids. Evidence: `src/msg/Message.h:195-196 (MSG_MDS_EXPORTDIRWARNING, MSG_MDS_EXPORTDIRWARNINGACK)`.
- **Map:** [subtree-migration.md › 3.1 Vocabulary and the dir_auth pair](../cpp-map/08-multi-mds/subtree-migration.md#31-vocabulary-and-the-dir_auth-pair) · [subtree-migration.md › 3.3 Exporter state machine](../cpp-map/08-multi-mds/subtree-migration.md#33-exporter-state-machine) · [subtree-migration.md › 3.4 Importer state machine](../cpp-map/08-multi-mds/subtree-migration.md#34-importer-state-machine) · [subtree-migration.md › 3.5 Message sequence (successful export)](../cpp-map/08-multi-mds/subtree-migration.md#35-message-sequence-successful-export) · [subtree-migration.md › 5.1 MDS-to-MDS messages (port MDS_PORT_MIGRATOR = 0x400, ...](../cpp-map/08-multi-mds/subtree-migration.md#51-mds-to-mds-messages-port-mds_port_migrator--0x400-accepted-only-from-mds-peers-mdsrankcc1241-1244)
- **Code:** `src/mds/Migrator.cc:920-1010 (Migrator::export_dir)`, `src/mds/Migrator.cc:1159-1340 (Migrator::dispatch_export_dir)`, `src/mds/Migrator.cc:2830-2918 (Migrator::handle_export_dir)`, `src/mds/Migrator.cc:2312-2403 (Migrator::export_finish)`, `src/mds/Migrator.cc:3250-3357 (Migrator::import_finish)`
- **Notes:** Initiators: balancer (export_dir_nicely, queued), export pins and ephemeral pins (export_dir directly), admin `export dir` (direct), stopping rank (queued), empty-import return (direct), mds_thrash_exports (queued). Handlers match messages on (dirfrag, tid, peer) and ignore mismatches, except MExportDir, MExportDirAck, MExportDirFinish and the first pass of MExportDirPrep, which assert the expected state (relying on lossless ordered MDS-MDS connections); MExportDirCancel for an import past PREPPED aborts the MDS. doc/dev/mds_internals/exports.rst is outdated (code reading, map section 8). The Migrator writes no RADOS objects directly.

<a id="f-multi-002"></a>

#### F-MULTI-002 · Export preconditions and silent refusals

*The set of conditions under which a requested export of a dirfrag is not started; refusals are logged only, so the caller (balancer, pin logic, admin command) is not told.*

Migrator::export_dir asserts the dirfrag is auth and dest != self, then refuses when: the dirfrag is pinned to another rank and has head items (not checked while the rank is stopping); the rank is not active or stopping; the cache is read-only; dest is not active; the cluster is degraded (any rank failed/damaged or in replay..clientreplay); the directory inode is a system inode (ino < MDS_INO_SYSTEM_BASE 4096: root 1, .ceph 2, lost+found 4, ~mdsN 256-511, stray dirs 1536-4095); the dirfrag is frozen/freezing or already exporting; its parent is a stray dir not under ~mds<dest>; the inode is quiesced or under quarantine. On success it hits dest as an export target, takes a Migrator auth pin, sets STATE_EXPORTING and starts the internal EXPORTDIR request in EXPORT_LOCKING.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, admin
- **Map:** [subtree-migration.md › 3.2 Starting an export: preconditions, queue, throttle, s...](../cpp-map/08-multi-mds/subtree-migration.md#32-starting-an-export-preconditions-queue-throttle-split) · [balancer-and-pinning.md › 3.8 Hand-off to the Migrator and export targets](../cpp-map/08-multi-mds/balancer-and-pinning.md#38-hand-off-to-the-migrator-and-export-targets)
- **Code:** `src/mds/Migrator.cc:920-1010 (Migrator::export_dir)`, `src/mds/CDir.cc:2928-2946 (CDir::is_exportable)`, `src/mds/MDSMap.cc:1287 (MDSMap::is_degraded)`
- **Notes:** The root dirfrag cannot be exported, but children of the root with ordinary inode numbers can; directories inside a stray dir can go only to the rank that owns that stray dir. Admin `export dir` returns 0 even when export_dir refuses (map section 8).

<a id="f-multi-003"></a>

#### F-MULTI-003 · Subtree freeze and auth-pin drain

*Quiesces all activity on the migrating region so that cache contents, lock states and client caps can be shipped as a consistent snapshot; its cost is that every request on the region waits for the whole protocol.*

CDir::freeze_tree auth-pins the root, attaches a shared freeze_tree_state to every cached nested dirfrag that is not a subtree root, sums their auth pins and invalidates lock caches holding auth pins there; from then on CDir::can_auth_pin fails with ERR_EXPORTING_TREE in the region and the freeze completes when only its own pin remains. The exporter keeps the freeze open with a Migrator auth pin until MExportDirDiscoverAck and stays frozen until export_finish; the importer freezes its replica region (non-auth _freeze_tree) from PREPPED until import_finish(last). While frozen: requests needing an auth pin wait on WAIT_UNFREEZE ("failed to authpin, subtree is being exported"), requests on ambiguous non-auth objects or that would forward wait on WAIT_SINGLEAUTH, client cap messages and cap releases on frozen inodes are deferred, and max_size checks, cap re-issue and lock evaluation wait. Path traversal continues except opening dirfrags of frozen inodes or adding null dentries in frozen dirs.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [subtree-migration.md › 3.8 Freezing and auth pins](../cpp-map/08-multi-mds/subtree-migration.md#38-freezing-and-auth-pins) · [subtree-migration.md › 6. Concurrency and mds_lock](../cpp-map/08-multi-mds/subtree-migration.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/CDir.cc:3345-3444 (CDir::freeze_tree)`, `src/mds/CDir.cc:3528-3555 (CDir::can_auth_pin)`, `src/mds/Locker.cc:476-495 (Locker::acquire_locks)`, `src/mds/Locker.cc:3363-3384 (Locker::should_defer_client_cap_frozen)`, `src/mds/MDCache.cc:8893-8899 (MDCache::maybe_request_forward_to_auth)`
- **Notes:** Client-visible only as latency (requests block without error, then are forwarded; see the request-forwarding entry). Map section 6 lists the full-subtree freeze for the multi-round-trip protocol as a serialization point that is "partly" inherent. Log segments containing the frozen region cannot expire until unfreeze (LogSegment::try_to_expire waits for WAIT_UNFREEZE). See also F-CACHE-030 (freeze_tree).

<a id="f-multi-004"></a>

#### F-MULTI-004 · Lock set held during migration

*The locks on the base directory inode, bound inodes and ancestors that keep fragstat/rstat accounting, fragmentation and snaprealms stable while authority moves.*

In EXPORT_LOCKING the exporter takes wrlock + STATE_PIN (lock_scatter_gather, with set_scatter_wanted so they go to MIX when the inode is local) on the base dir inode filelock and nestlock, rdlocks its dirfragtreelock, rdlock_try_set on the dirfragtreelock of every would-be bound inode and try_rdlock_snap_layout on the snaplock of the base inode and all ancestors; these helpers never block while holding locks (they drop everything and retry). The locks are released at the discover ack; after the freeze export_try_grab_locks re-takes them non-blockingly into a MutationImpl and wrlock_force-s filelock/nestlock, and cancels the export if it cannot. The importer wrlock_force-s the same two locks from PREPPED until finish or reverse. The scatter-gather prevents a third rank that is auth for the base inode from counting the dirfrag's fragstat/rstat twice or not at all during the move; snapshots on ancestors (snaplock xlock), fragmentation of base or bounds and gathers on the base inode wait for the export.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, mds-ranks
- **Map:** [subtree-migration.md › 3.9 Locks held during migration](../cpp-map/08-multi-mds/subtree-migration.md#39-locks-held-during-migration) · [subtree-migration.md › 3.6 Step-by-step walkthrough](../cpp-map/08-multi-mds/subtree-migration.md#36-step-by-step-walkthrough)
- **Code:** `src/mds/Migrator.cc:1206-1268 (Migrator::dispatch_export_dir)`, `src/mds/Migrator.cc:877-913 (Migrator::export_try_grab_locks)`, `src/mds/Migrator.cc:2637 (Migrator::handle_export_prep)`
- **Notes:** Locker::xlock_export/xlock_import are used only by cross-rank rename's single-inode migration (map 3.9). Lock mechanics are owned by 03-caps-leases-and-locking.

<a id="f-multi-005"></a>

#### F-MULTI-005 · Prep phase: spanning-tree replication and bound opening

*Makes the importer hold replicas of everything between the exported dirfrag and each bound, open every bound dirfrag and freeze its replica region before metadata is shipped.*

MExportDirPrep carries dirfrag, basedir (replica encoding of the exported dirfrag, which adds the importer as a replica), list<dirfrag_t> bounds, list<bufferlist> traces and set<rank> bystanders. Each trace (ENCODE_START(1,1)) holds replicas of (dentry, inode) and intermediate dirfrags from a bound's inode up to the first dirfrag already sent or the base, without repeats; on the exporter each bound gets PIN_EXPORTBOUND/STATE_EXPORTBOUND. The importer decodes the traces, takes get_stickydirs on every bound inode, fetches missing bound dirfrags from their auth with open_remote_dirfrag (MDiscover) and re-runs the handler until all are open, pins them (PIN_IMPORTBOUND), wrlock_force-s filelock/nestlock of the base inode, sets the subtree auth to (E,I) with adjust_bounded_subtree_auth, freezes the replica region and replies MExportDirPrepAck(success). It fails the prep when not active, read-only, or the two locks are not wrlockable for the loner.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks, internal
- **Map:** [subtree-migration.md › 3.6 Step-by-step walkthrough](../cpp-map/08-multi-mds/subtree-migration.md#36-step-by-step-walkthrough) · [subtree-migration.md › 5.1 MDS-to-MDS messages (port MDS_PORT_MIGRATOR = 0x400, ...](../cpp-map/08-multi-mds/subtree-migration.md#51-mds-to-mds-messages-port-mds_port_migrator--0x400-accepted-only-from-mds-peers-mdsrankcc1241-1244)
- **Code:** `src/mds/Migrator.cc:1489-1583 (Migrator::export_frozen)`, `src/mds/Migrator.cc:1438-1487 (Migrator::encode_export_prep_trace)`, `src/mds/Migrator.cc:2637 (Migrator::handle_export_prep)`, `src/mds/Migrator.cc:2596-2635 (Migrator::decode_export_prep_trace)`
- **Notes:** Before the prep, the importer handles MExportDirDiscover by discovering the base dir inode from its auth if not cached (path_traverse with MDS_TRAVERSE_DISCOVER|PATH_LOCKED|IMPORT); -EAGAIN (a quiesced ancestor) gives a NACK, any other error asserts.

<a id="f-multi-006"></a>

#### F-MULTI-006 · Metadata and cap transfer encoding (MExportDir)

*Ships every cached object of the migrating region (dirfrags, dentries, inodes with xattrs and snap state, lock states, file locks, popularity, replica maps, dirty bits) and all client caps on exported inodes to the importer.*

MExportDir holds dirfrag, vector<dirfrag_t> bounds, export_data and client_map (map<client_t, entity_inst_t> then map<client_t, client_metadata_t>). export_data is a pre-order list of dirfrag records: dirfrag_t, CDir::encode_export (first, fnode, dirty_old_rstat, committed_version, state, dir_rep, pop_me, pop_auth_subtree, dir_rep_by, replicas), then dentries with name, last, CDentry::encode_export and a marker 'N' (null), 'l' (remote: ino, d_type, alternate_name) or 'i' (primary inode). The inode record carries CInode::encode_export (ENCODE_START(5,4): inode_t, symlink, dirfragtree, xattrs, old_inodes, damage flags, snap blob, state, pop, replicas, fragstat/rstat of bound dirfrags, the full state of all nine locks and loner_cap, fcntl and flock tables) and a caps record: map<client_t, Capability::Export{cap_id, wanted, issued, pending, client_follows, seq, mseq+1, last_issue_stamp, state}> plus mds_caps_wanted. The importer marks dirty objects dirty in its own current log segment (it re-journals, the exporter marks them clean without writing), keeps only listed state bits (inode: DIRTY, NEEDSRECOVER, DIRTYPARENT, DIRTYPOOL, DISTEPHEMERALPIN, RANDEPHEMERALPIN), and becomes auth while the exporter becomes a replica with EXPORT_NONCE 1.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks, internal
- **Map:** [subtree-migration.md › 3.7 What state is transferred](../cpp-map/08-multi-mds/subtree-migration.md#37-what-state-is-transferred) · [subtree-migration.md › 3.6 Step-by-step walkthrough](../cpp-map/08-multi-mds/subtree-migration.md#36-step-by-step-walkthrough)
- **Code:** `src/mds/Migrator.cc:1899-1985 (Migrator::encode_export_dir)`, `src/mds/Migrator.cc:1779-1822 (Migrator::encode_export_inode)`, `src/mds/CInode.cc:4656 (CInode::encode_export)`, `src/mds/Capability.cc:35 (Capability::Export::encode)`, `src/mds/Migrator.cc:3432 (Migrator::decode_import_inode)`
- **Notes:** Not transferred: the base directory inode, bound dirfrags, client dentry leases (they stay on the exporter's now-replica dentries and are revoked through the replica lock protocol), per-session completed-request lists, open-file-table entries (each rank's table follows cap add/remove). import_state_t::updated_scatterlocks is filled but never consumed. The fcntl/flock tables and xattrs carried here are what keep POSIX locks and attributes intact across an authority change. Perf counters exported_inodes/imported_inodes count every dentry, null and remote included.

<a id="f-multi-007"></a>

#### F-MULTI-007 · Bystander warning and notify

*Keeps other ranks that replicate the exported dirfrag informed of the temporarily ambiguous authority so their cache-expire and lock messages reach the right rank.*

Bystanders are the replicas of the base dirfrag other than the importer (replicas of nested objects imply a replica of the base dirfrag). After MExportDirPrepAck the exporter sends MExportDirNotify(base, ack=true, old_auth=(E,-2), new_auth=(E,I), bounds) to each one (only clientreplay/active/stopping ones when degraded); after EExport is safe it sends old=(E,I), new=(I,-2); an exporter abort after warning sends old=(E,I), new=(E,-2); the importer sends the same abort on reversal and old=(E,I), new=(I,-2) without ack request when it finishes after an exporter failure. A bystander that caches the dir with authority old_auth applies adjust_bounded_subtree_auth(new_auth) and try_subtree_merge, and replies MExportDirNotifyAck(dirfrag, new_auth) when asked, even if it does not cache the dir. The exporter sends MExportDir only after all warning acks; the importer may send cap IMPORTs early on MExportDirFinish(last=false) while bystanders are still acking the final notify.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks, internal
- **Map:** [subtree-migration.md › 3.6 Step-by-step walkthrough](../cpp-map/08-multi-mds/subtree-migration.md#36-step-by-step-walkthrough) · [subtree-migration.md › 5.1 MDS-to-MDS messages (port MDS_PORT_MIGRATOR = 0x400, ...](../cpp-map/08-multi-mds/subtree-migration.md#51-mds-to-mds-messages-port-mds_port_migrator--0x400-accepted-only-from-mds-peers-mdsrankcc1241-1244)
- **Code:** `src/mds/Migrator.cc:1622-1682 (Migrator::handle_export_prep_ack)`, `src/mds/Migrator.cc:2210-2249 (Migrator::export_logged_finish)`, `src/mds/Migrator.cc:2259-2310 (Migrator::handle_export_notify_ack)`, `src/mds/Migrator.cc:3744-3782 (Migrator::handle_export_notify)`
- **Notes:** Exists only because cross-rank replicas and cache expires exist. handle_export_notify_ack calls hit_export_target with the sender, so bystanders that ack are also published as export targets for a while (Migrator.cc:2262-2266). MExportDirNotifyAck is matched by state and sender membership only (no tid check). Failed bystanders are removed from the wait sets.

<a id="f-multi-008"></a>

#### F-MULTI-008 · Cache-expire routing during migration

*Ensures replica ranks' MCacheExpire messages for objects in a migrating region are honoured by exactly the rank that ends up authoritative.*

Replicas send MCacheExpire for a dentry to both ranks of an ambiguous authority pair, except that the importer (auth.second) sends none while its container is importing. The receiver queues expires in delayed_expire[dir][from] when the container is not auth or is exporting in WARNING, EXPORTING, LOGGINGFINISH or NOTIFYING. Delayed expires are processed when the rank ends up authoritative (export_try_cancel in WARNING, export_reverse, import_finish) and discarded when it does not (export_finish, import_reverse_unfreeze). Nonces make this safe: the exporter becomes a replica with EXPORT_NONCE 1 and the importer inherits the exporter's replica maps and nonces, so only the side holding the matching nonce honours an expire.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks, internal
- **Map:** [subtree-migration.md › 3.14 Cache expires during migration](../cpp-map/08-multi-mds/subtree-migration.md#314-cache-expires-during-migration)
- **Code:** `src/mds/MDCache.cc:7222-7236 (MDCache::trim_dentry)`, `src/mds/MDCache.cc:7705-7733 (MDCache::handle_cache_expire)`
- **Notes:** The expire and replica mechanism itself is owned by 05-metadata-cache-and-dirfrags; this entry is only the migration-time routing. See also F-CACHE-058 (MCacheExpire).

<a id="f-multi-009"></a>

#### F-MULTI-009 · Empty import return

*Sends an imported dirfrag that turns out empty, or an empty non-auth subtree found by cache trim, back to the rank that is auth for its directory inode.*

At the end of import_finish, if the imported dirfrag is empty and its inode is not local, export_empty_import exports it to the inode's authority; MDCache::trim does the same for empty non-auth imported subtrees. These exports call export_dir directly (no queue or budget). Combined with the pin rule that empty dirfrags are not moved by pins, a newly pinned empty directory stays on its parent's rank until it has entries.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [subtree-migration.md › 3.6 Step-by-step walkthrough](../cpp-map/08-multi-mds/subtree-migration.md#36-step-by-step-walkthrough) · [balancer-and-pinning.md › 3.12 Export pins (ceph.dir.pin)](../cpp-map/08-multi-mds/balancer-and-pinning.md#312-export-pins-cephdirpin)
- **Code:** `src/mds/Migrator.cc:231-266 (Migrator::export_empty_import)`, `src/mds/MDCache.cc:7059-7067 (MDCache::trim)`
- **Notes:** See also F-CACHE-066 (empty-subtree trimming).

<a id="f-multi-010"></a>

#### F-MULTI-010 · Export queue and in-flight size throttle

*Limits how much metadata a rank migrates at once for balancer and rank-shrink exports.*

Migrator::export_dir_nicely appends (dirfrag, dest) to export_queue; maybe_do_queued_export starts queued exports while total_exporting_size (sum of approx_size of exports past LOCKING) < 2 * mds_max_export_size and the remaining budget is >= mds_max_export_size * (num_locking_exports + 1), because an export still in LOCKING has unknown size. The queue is drained again after every finish or cancel; clear_export_queue() empties it and bumps export_queue_gen (called by the balancer before each rebalance and by shutdown_pass). Direct export_dir callers (pins, admin export dir, empty-import return, split children) bypass the queue and budget.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_max_export_size (size, default 20 MiB, dev, runtime)
- **Map:** [subtree-migration.md › 3.2 Starting an export: preconditions, queue, throttle, s...](../cpp-map/08-multi-mds/subtree-migration.md#32-starting-an-export-preconditions-queue-throttle-split) · [balancer-and-pinning.md › 3.8 Hand-off to the Migrator and export targets](../cpp-map/08-multi-mds/balancer-and-pinning.md#38-hand-off-to-the-migrator-and-export-targets)
- **Code:** `src/mds/Migrator.cc:819-856 (Migrator::export_dir_nicely)`
- **Notes:** maybe_do_queued_export uses a function-static re-entrancy flag (process-wide, not per rank).

<a id="f-multi-011"></a>

#### F-MULTI-011 · Export splitting of large subtrees

*Breaks an export whose estimated size exceeds mds_max_export_size into several smaller child exports.*

After its locks are held every export runs maybe_split_export: an iterative DFS estimates size in abstract units (dirfrag 800, per dentry name length, null dentry +1, remote dentry +10, primary inode +1000 plus 80 per client cap); nested dirfrags that are exporting or freezing/frozen are skipped and make ancestor levels incomplete, whose complete children become separate results. If the result is not the whole dir, each result becomes its own export (own EXPORT_LOCKING entry and internal request, started directly, outside queue and budget) sharing an export_base_t parent, and the original export is cancelled; child_export_finish requeues the original at the queue front once all children finished, if at least one succeeded (or all were already exporting) and export_queue_gen is unchanged.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_max_export_size (split threshold)
- **Map:** [subtree-migration.md › 3.2 Starting an export: preconditions, queue, throttle, s...](../cpp-map/08-multi-mds/subtree-migration.md#32-starting-an-export-preconditions-queue-throttle-split)
- **Code:** `src/mds/Migrator.cc:1016-1146 (Migrator::maybe_split_export)`, `src/mds/Migrator.cc:1342-1356 (Migrator::child_export_finish)`
- **Notes:** Size units are heuristic, not bytes. Splitting applies to every export, including pin and admin exports.

### Client-visible effects of migration

What unmodified clients and the monitor see when authority moves: cap EXPORT/IMPORT messages, the session flush barrier, sessions opened on the importer without a client request, request blocking and forwarding, and the MDSMap export_targets list.

<a id="f-multi-012"></a>

#### F-MULTI-012 · Cap EXPORT message to clients (CEPH_CAP_OP_EXPORT)

*Tells each client holding a cap on a migrated inode that the exporting rank no longer holds it and where the cap went; kclient and libcephfs move or drop the cap accordingly.*

Sent by the exporter in export_finish (finish_export_inode_caps) to every cap holder, after which all caps are removed: MClientCaps op 3 with head.cap_id = exporter's cap id, migrate_seq = exporter mseq m, seq/issue_seq 0, caps/wanted 0, and the packed peer block ceph_mds_cap_peer {le64 cap_id; le32 issue_seq; le32 mseq; le32 mds; u8 flags} placed in the body where size/max_size would be (zero-padded). peer = importer's Capability::Import (cap_id, last_seq+1, m+1) and importer rank, flags 0; peer.mds = -1 when the importer reported cap_id 0 (no session), which tells the client to drop the cap. The message is counted (bumps the session push_seq) and carries osd_epoch_barrier. kclient ignores it unless it holds a cap from that rank with the same cap_id; if it has no cap from the target it opens a session to the target and adds a placeholder cap (seq = issue_seq-1, mseq = peer.mseq, issued = exported bits) carrying the in-flight cap flushes, then removes the exporter's cap without sending a release.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [subtree-migration.md › 3.11 Client-visible cap migration](../cpp-map/08-multi-mds/subtree-migration.md#311-client-visible-cap-migration) · [subtree-migration.md › 7. Externally visible contract](../cpp-map/08-multi-mds/subtree-migration.md#7-externally-visible-contract)
- **Code:** `src/mds/Migrator.cc:1824-1848 (Migrator::finish_export_inode_caps)`, `src/include/ceph_fs.h:936-942 (struct ceph_mds_cap_peer)`, `src/messages/MClientCaps.h:324-328 (MClientCaps::encode_payload)`, `linux/fs/ceph/caps.c:4154 (handle_cap_export)`, `src/client/Client.cc:5872-5915 (Client::handle_cap_export)`
- **Notes:** libcephfs keys EXPORT handling on peer.cap_id instead of peer.mds: with peer.cap_id == 0 it removes the cap and sets I_CAP_DROPPED if it had wanted or issued bits. EXPORT and IMPORT travel on different connections and may arrive in either order. Cross-rank rename (owned by 04-namespace-and-posix-operations) sends the same EXPORT/IMPORT pair with FLAG_AUTH. See also F-CAPS-036 (cap migration messages).

<a id="f-multi-013"></a>

#### F-MULTI-013 · Cap IMPORT message to clients and auth-cap switch (CEPH_CAP_OP_IMPORT)

*Gives each client the importer's cap on a migrated inode, with full inode state, and makes it the client's auth cap.*

After the exporter's MExportDirFinish the importer merges each exported cap (Capability::merge: pending/issued unioned and re-issued, wanted unioned, auth caps take the new mseq) and MDCache::do_cap_import sends MClientCaps op 4: head.cap_id = importer cap id, migrate_seq = m+1, seq = cap last_seq (incremented first if 0), issue_seq = last_issue, caps = pending, wanted, snaprealm ino, full inode fields as in a grant plus snap trace; the peer block {exporter cap_id, exporter seq, m, exporter rank, CEPH_CAP_FLAG_AUTH (1)} is appended after the flock blob when the connection has CEPH_FEATURE_EXPORT_PEER. The client switches its auth cap only if ceph_seq_cmp(old_auth.mseq, mseq) < 0 and removes a cap from peer.mds whose cap_id equals peer.cap_id; on import of the auth cap kclient re-sends in-flight cap flushes, resets i_requested_max_size so max_size is re-requested, and re-checks wanted. The message is queued in preopen_out_queue if the session has no connection yet.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): IMPORT messages without the peer block (v2), and caps messages without the flock blob (v1), for connections lacking CEPH_FEATURE_EXPORT_PEER or CEPH_FEATURE_FLOCK. MClientCaps::encode_payload stops at message v1 for a connection without FLOCK and at v2, without the IMPORT peer block, for a connection without EXPORT_PEER (bit 37). The 7.3 kernel, the CentOS Stream samples and current libcephfs advertise both; no Linux kernel since 3.14 and no Ceph client since v0.75 lacks EXPORT_PEER. Evidence: `src/messages/MClientCaps.h:348-360 (MClientCaps::encode_payload)`, `src/include/ceph_features.h:89 (FLOCK, kernel 2.6.36)`, `src/include/ceph_features.h:141 (EXPORT_PEER, kernel 3.14)`, `linux/include/linux/ceph/ceph_features.h:182 (CEPH_FEATURES_SUPPORTED_DEFAULT, FLOCK)`, `linux/include/linux/ceph/ceph_features.h:205 (CEPH_FEATURES_SUPPORTED_DEFAULT, EXPORT_PEER)`.
- **Gates:** CEPH_FEATURE_EXPORT_PEER (bit 37) for the peer block
- **Map:** [subtree-migration.md › 3.11 Client-visible cap migration](../cpp-map/08-multi-mds/subtree-migration.md#311-client-visible-cap-migration) · [subtree-migration.md › 3.6 Step-by-step walkthrough](../cpp-map/08-multi-mds/subtree-migration.md#36-step-by-step-walkthrough)
- **Code:** `src/mds/MDCache.cc:5954-5973 (MDCache::do_cap_import)`, `src/mds/Migrator.cc:3250-3357 (Migrator::import_finish)`, `src/mds/Capability.h:286-308 (Capability::merge)`, `linux/fs/ceph/caps.c:4288 (handle_cap_import)`, `linux/fs/ceph/caps.c:753-761 (ceph_add_cap)`
- **Notes:** libcephfs looks for the peer cap only when peer.cap_id != 0 and uses the same ceph_seq_cmp(auth_cap->mseq, mseq) < 0 test (Client::add_update_cap). With FLAG_AUTH a seq/mseq mismatch is only logged by kclient. CEPH_CAP_FLAG_RELEASE (2) is used instead for non-auth MExportCaps imports (separate entry). See also F-CAPS-036 (cap migration messages).

<a id="f-multi-014"></a>

#### F-MULTI-014 · Migrate sequence (mseq) and stale cap-message handling

*Lets the MDS and clients tell cap messages that predate a migration from current ones.*

Each auth cap migration raises the cap's migrate sequence by one (Capability::Export carries exporter mseq + 1). Clients put the cap's mseq in outgoing cap updates and releases; the MDS drops updates and releases whose mseq is older than its current cap's, and silently drops cap messages for inodes or caps it no longer has after a migration. Clients therefore re-send flushes and re-check wanted/max_size after an IMPORT, since earlier updates may have been dropped for stale mseq.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [subtree-migration.md › 3.11 Client-visible cap migration](../cpp-map/08-multi-mds/subtree-migration.md#311-client-visible-cap-migration) · [subtree-migration.md › 7. Externally visible contract](../cpp-map/08-multi-mds/subtree-migration.md#7-externally-visible-contract)
- **Code:** `src/mds/Locker.cc:3521-3525 (Locker::handle_client_caps)`, `src/mds/Locker.cc:4325-4328 (Locker::_do_cap_release)`, `linux/fs/ceph/caps.c:1303 (encode_cap_msg)`, `linux/fs/ceph/caps.c:3748-3760 (handle_cap_grant)`
- **Notes:** The mseq field itself belongs to the cap message format owned by 03-caps-leases-and-locking; this entry is its migration semantics.

<a id="f-multi-015"></a>

#### F-MULTI-015 · Writeable-range reconciliation on cap import

*Preserves the file-size recovery guarantee for open-for-write files whose writer's cap could not be carried over to the importer.*

When the importer creates caps from an auth export (finish_import_inode_caps), clients present in the inode's client_ranges get Capability STATE_CLIENTWRITEABLE; if any client in client_ranges has no imported cap (no exported cap, or its session was filtered out as blocklisted), the inode gets CInode::STATE_NEEDSRECOVER so file size/mtime recovery runs later. Capability::merge ORs the exported state bits CLIENTWRITEABLE, NOINLINE, NOPOOLNS and NOQUOTA into the importer's cap; newly created caps are marked STATE_IMPORTING until import_finish.

- **Class:** POSIX `supporting` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [subtree-migration.md › 3.6 Step-by-step walkthrough](../cpp-map/08-multi-mds/subtree-migration.md#36-step-by-step-walkthrough) · [subtree-migration.md › 3.7 What state is transferred](../cpp-map/08-multi-mds/subtree-migration.md#37-what-state-is-transferred)
- **Code:** `src/mds/Migrator.cc:3532 (Migrator::finish_import_inode_caps)`, `src/mds/Capability.h:289-308 (Capability::merge)`
- **Notes:** File recovery (size probing) is owned by 03-caps-leases-and-locking / 09-background-services; this entry is the migration-side trigger.

<a id="f-multi-016"></a>

#### F-MULTI-016 · Client session flush barrier before shipping (FLUSHMSG)

*Makes the exporter wait until every client with caps in the region has drained messages in both directions before the region is shipped.*

In export_frozen the exporter collects export_client_set (every client with a cap on a primary inode in the region) and Server::flush_client_sessions sends CEPH_SESSION_FLUSHMSG(seq = session push_seq) to each such session that is open, connected and has CEPH_FEATURE_EXPORT_PEER; MDS_RANK_NONE is added to warning_ack_waiting until all CEPH_SESSION_FLUSHMSG_ACK arrive, and MExportDir is not sent before that. The client flushes pending cap releases and acks with the same seq; the ack completes every flush waiter registered at or below that seq. In effect every message the exporter sent to those clients before FLUSHMSG has been received, and every client message (including cap releases) sent before the ack has reached the exporter (inference from per-connection ordering, stated on the map page).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Skipping the FLUSHMSG barrier for sessions whose connection lacks CEPH_FEATURE_EXPORT_PEER. Server::flush_session returns without sending FLUSHMSG when the session's connection lacks CEPH_FEATURE_EXPORT_PEER. Every baseline client advertises EXPORT_PEER, so for open, connected sessions the barrier always runs; clients without the feature predate Linux 3.14 and Ceph v0.75. Evidence: `src/mds/Server.cc:897-907 (Server::flush_session)`, `src/include/ceph_features.h:141 (EXPORT_PEER, kernel 3.14)`, `linux/include/linux/ceph/ceph_features.h:205 (CEPH_FEATURES_SUPPORTED_DEFAULT, EXPORT_PEER)`.
- **Gates:** CEPH_FEATURE_EXPORT_PEER (bit 37); sessions without it, not open or not connected are not flushed
- **Map:** [subtree-migration.md › 3.11 Client-visible cap migration](../cpp-map/08-multi-mds/subtree-migration.md#311-client-visible-cap-migration) · [subtree-migration.md › 5.2 Client and monitor messages](../cpp-map/08-multi-mds/subtree-migration.md#52-client-and-monitor-messages)
- **Code:** `src/mds/Migrator.cc:1585-1620 (Migrator::get_export_client_set)`, `src/mds/Server.cc:897-923 (Server::flush_client_sessions)`, `src/mds/SessionMap.h:275-278 (Session::wait_for_flush)`, `linux/fs/ceph/mds_client.c:4637-4644 (handle_session)`, `src/client/Client.cc:2779-2785 (Client::handle_client_session)`
- **Notes:** The FLUSHMSG/FLUSHMSG_ACK session ops themselves are owned by 02-client-sessions-and-protocol (PROTO); this entry is their use by migration. Code comment intent: "make sure any new instantiations of caps are flushed out" (Migrator.cc:1570).

<a id="f-multi-017"></a>

#### F-MULTI-017 · Forced client session open on the importer

*Subtree import force-opens sessions on the importer for every client named by exported caps.*

See F-PROTO-041. Import-specific: handle_export_dir calls prepare_force_open_sessions with the exported client map and metadata; the projected SessionMap version is recorded as cmapv in EImportStart, and finish_force_open_sessions runs when EImportStart is safe; the unsolicited OPENs are queued until the client connects (flushed in MDSDaemon::ms_handle_accept). On reversal from ACKING, forced sessions the client never opened and whose state is unchanged are closed (close_forced_opened_sessions, journaled).

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [subtree-migration.md › 3.11 Client-visible cap migration](../cpp-map/08-multi-mds/subtree-migration.md#311-client-visible-cap-migration) · [subtree-migration.md › 3.6 Step-by-step walkthrough](../cpp-map/08-multi-mds/subtree-migration.md#36-step-by-step-walkthrough)
- **Code:** `src/mds/Server.cc:1059-1104 (Server::prepare_force_open_sessions)`, `src/mds/Server.cc:1106-1154 (Server::finish_force_open_sessions)`, `src/mds/Server.cc:1156-1177 (Server::close_forced_opened_sessions)`, `src/mds/Server.cc:656-686 (Server::handle_client_session)`, `src/mds/Migrator.cc:3191-3235 (Migrator::import_logged_start)`
- **Notes:** Bug by code reading (map section 8): a reversal from IMPORT_LOGGINGSTART does not decrement importing_count, because import_state.session_map is only filled in import_logged_start, so such sessions keep ignoring REQUEST_CLOSE and are skipped by kill_session and idle eviction; without bystanders the pending import_logged_start then hits ceph_assert(it != import_state.end()).

<a id="f-multi-018"></a>

#### F-MULTI-018 · Request blocking and forwarding around migration

*Defines what clients see for requests on a migrating subtree: they block without error and, once authority moved, are bounced to the new auth rank.*

Requests that need an auth pin in a freezing/frozen region wait on WAIT_UNFREEZE; requests that would forward to an ambiguous auth wait on WAIT_SINGLEAUTH. After export_finish the collected waiters are re-queued and, finding the dirfrag no longer auth, the request is forwarded with MDSRank::forward_message_mds, which always sends MClientRequestForward to the client (the client resends to the new rank) instead of forwarding non-idempotent requests between ranks. Reply traces after the move carry the new dirfrag auth in DirStat, which clients use for later routing.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [subtree-migration.md › 3.8 Freezing and auth pins](../cpp-map/08-multi-mds/subtree-migration.md#38-freezing-and-auth-pins) · [subtree-migration.md › 5.2 Client and monitor messages](../cpp-map/08-multi-mds/subtree-migration.md#52-client-and-monitor-messages) · [subtree-migration.md › 7. Externally visible contract](../cpp-map/08-multi-mds/subtree-migration.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSRank.cc:1504-1527 (MDSRank::forward_message_mds)`, `src/mds/MDCache.cc:8893-8899 (MDCache::maybe_request_forward_to_auth)`, `src/mds/Server.cc:3927-3939 (Server::rdlock_path_pin_ref)`
- **Notes:** Completed-request tracking is per session per rank and is not migrated (comment MDSRank.cc:1510-1517). Synchronous NFS operations through Ganesha see the block as latency. The forward message encoding is owned by 02-client-sessions-and-protocol. See also F-PROTO-076, F-CACHE-044.

<a id="f-multi-019"></a>

#### F-MULTI-019 · Export-target publication (MMDSLoadTargets, MDSMap export_targets)

*Publishes, through the monitor, the ranks each rank is exporting to, so clients can pre-open sessions to them; the exporter waits for its own entry before exporting.*

MDSRank::hit_export_target(rank, amount) hits a per-rank DecayCounter (half-life mds_bal_target_decay, default 10 s; default amount 100/half_life = 10, num_exported_inodes+1 after MExportDir); the Migrator hits dest at several protocol steps and also bystanders that ack notifies. Every tick while active or stopping (not laggy) update_targets drops ranks whose counter is <= 0.01 and sends MMDSLoadTargets (0x20a; PaxosServiceMessage prefix, u64 global_id, set<int32> targets) when a target was dropped, a local target is missing from this daemon's mds_info_t::export_targets, or the map set is larger. The monitor accepts it only from sessions with mds x cap, ignores unchanged sets, stores the set for the gid if it holds a rank, bumps the FSMap epoch and never replies. dispatch_export_dir waits for an MDSMap where dest is in its export_targets, dropping locks between attempts, and cancels after 4 maps (count > 3). Clients read per-rank export_targets: libcephfs Client::connect_mds_targets opens sessions to targets in clientreplay/active/stopping when a session opens, when the rank becomes active and before reconnect; kclient opens sessions to the targets of a laggy rank and reconnects to a target in up:reconnect even without an existing session.

- **Class:** POSIX `supporting` · exposure `cluster-wire`, `client-wire` · relied on by mon, kclient, libcephfs, ceph-fuse, ganesha, admin, mds-ranks
- **Gates:** mds_bal_target_decay (default 10, runtime); mds_tick_interval (default 5 s); mon cap mds x (profile mds)
- **Map:** [balancer-and-pinning.md › 3.8 Hand-off to the Migrator and export targets](../cpp-map/08-multi-mds/balancer-and-pinning.md#38-hand-off-to-the-migrator-and-export-targets) · [subtree-migration.md › 3.11 Client-visible cap migration](../cpp-map/08-multi-mds/subtree-migration.md#311-client-visible-cap-migration) · [balancer-and-pinning.md › 7.3 Monitor](../cpp-map/08-multi-mds/balancer-and-pinning.md#73-monitor)
- **Code:** `src/mds/MDSRank.cc:611-667 (MDSRank::update_targets)`, `src/mds/Migrator.cc:1184-1198 (Migrator::dispatch_export_dir)`, `src/mon/MDSMonitor.cc:936-953 (MDSMonitor::prepare_offload_targets)`, `linux/fs/ceph/mds_client.c:5951-6012 (check_new_map)`, `src/client/Client.cc:2095-2113 (Client::connect_mds_targets)`
- **Notes:** The kclient reconnect to an export target in up:reconnect covers an importer crash after EImportStart, when the client may not yet have a session there. A first export to a new destination costs a mon round trip (next tick, Paxos commit, map delivery). export_targets is visible in `ceph fs dump`. The MDSMap/FSMap structure is owned by 01-cluster-membership-and-ha. See also F-CLU-002 (MDSMap), F-CLU-006 (kernel MDSMap decode).

<a id="f-multi-020"></a>

#### F-MULTI-020 · Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps)

*Moves client caps that a replica rank issued (read-only opens served by a replica) on a single inode to the inode's auth rank.*

Triggers: a replica learns the inode was unlinked into a stray dir (MDCache::handle_dentry_unlink), or the auth sends MGatherCaps{ino} (0x472) to every replica when broadcasting quota and the replica holds caps, is non-auth, unambiguous and not already exporting caps. The replica sends MExportCaps (0x470, v2 compat 1: ino, cap_bl without mds_wanted, client_map, client_metadata_map) to the auth. The auth auth-pins the inode, force-opens sessions, journals ESessions and on safe merges each cap without changing mseq and immediately sends IMPORT with CEPH_CAP_FLAG_RELEASE (2); for clients whose session on the auth has no connection (or was filtered as blocklisted, cap_id 0) it replies MExportCapsAck (0x471: ino, map<client_t, Import>, map<client_t, u64 peer cap id>), and the replica sends EXPORT to exactly those clients and removes their caps. Clients that got the IMPORT drop the replica cap and queue a cap release to the replica.

- **Class:** POSIX `supporting` · exposure `mds-wire`, `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [subtree-migration.md › 3.13 Single-inode cap migration (MExportCaps, MGatherCaps)](../cpp-map/08-multi-mds/subtree-migration.md#313-single-inode-cap-migration-mexportcaps-mgathercaps)
- **Code:** `src/mds/Migrator.cc:3785-3937 (Migrator::export_caps)`, `src/mds/MDCache.cc:2184-2188 (MDCache::broadcast_quota_to_client)`, `src/mds/Migrator.cc:3840-3854 (Migrator::handle_gather_caps)`
- **Notes:** Known gap (code reading, FIXME in code): handle_export_caps silently drops the message if the inode cannot be auth-pinned (frozen/freezing), and the replica keeps the caps (Migrator.cc:3877-3880). These messages carry no tid. Replica caps themselves are owned by 03-caps-leases-and-locking. The quota trigger exists because only the auth rank sends MClientQuota pushes (purpose inferred from code; F-SNAP-045). See also F-CAPS-035 (replica caps), F-CAPS-036 (EXPORT/IMPORT).

### Migration durability, cancellation and failure handling

Journal events that make a migration crash-safe, per-state cancellation and reversal, behaviour when any rank fails during a migration, and the guards that cancel exports that would deadlock (freeze timeout, quiesce, quarantine).

<a id="f-multi-021"></a>

#### F-MULTI-021 · Migration journaling (EImportStart, EExport, EImportFinish)

*Makes a migration crash-safe: the importer journals a full copy of the imported metadata and sessions before acknowledging, and the exporter journals the hand-off only after that.*

EImportStart (EVENT_IMPORTSTART 4, ENCODE_START(4,3): stamp, base, metablob with every imported dirfrag and dentry flagged importing, bounds, cmapv, client_map with metadata, from) is submitted and flushed on MExportDir; the ack is sent from its log-safe callback. EExport (EVENT_EXPORT 3, ENCODE_START(4,3): stamp, metablob with contexts and clean base/bounds only, base, bounds, target) is submitted and flushed on MExportDirAck. EImportFinish (EVENT_IMPORTFINISH 5, ENCODE_START(3,3): stamp, base, success) ends the import (success) or records a reversal (failure). In-flight imports (LOGGINGSTART..FINISHING) appear as ambiguous_subtrees in every ESubtreeMap. Replay: EImportStart registers an ambiguous import with auth (me,me) and replays sessions if sessionmap.version < cmapv; EExport sets the region to UNDEF; EImportFinish finishes or cancels the ambiguous import. Ordering: the exporter waits for its whole journal to be safe before sending MExportDir, keeps the region dirty until export_finish, and marks it clean without writing once EExport is safe; the importer re-dirties everything in its current segment.

- **Class:** POSIX `supporting` · exposure `on-disk` · relied on by internal, tools
- **Map:** [subtree-migration.md › 3.10 Journaling and ambiguous authority](../cpp-map/08-multi-mds/subtree-migration.md#310-journaling-and-ambiguous-authority)
- **Code:** `src/mds/journal.cc:3104-3336 (EExport::replay)`, `src/mds/Migrator.cc:1699-1768 (Migrator::export_go)`, `src/mds/Migrator.cc:2052-2100 (Migrator::handle_export_ack)`, `src/mds/MDCache.cc:2624-2660 (MDCache::create_subtree_map)`
- **Notes:** cephfs-journal-tool decodes these encodings. EImportFinish replay without a matching ambiguous import logs a clog error and marks the rank damaged (journal.cc:3294-3301). A large dirty subtree migration costs a large journal write and later dirfrag commits on the importer. Journal event framework owned by 06-journal-and-storage. See also F-STORE-033 (record formats and replay).

<a id="f-multi-022"></a>

#### F-MULTI-022 · Export cancellation and reversal

*Unwinds an export or import that cannot complete, leaving the exporter authoritative and clients holding their exporter caps.*

export_try_cancel per state: LOCKING -> CANCELLED, kill request; DISCOVERING/FREEZING -> CANCELLED, unfreeze, MExportDirCancel; PREPPING -> also unpin bounds and drop locks; WARNING -> CANCELLING with abort notifies to bystanders (old (E,I), new (E,-2)), delayed expires processed, unfreeze; EXPORTING (only when the importer failed) -> export_reverse; LOGGINGFINISH/NOTIFYING -> nothing ("we were successful"). export_reverse aborts every exported object (drop PIN_TEMPEXPORTING, clear STATE_EXPORTINGCAPS), restores (E,E), unfreezes and re-issues caps postponed by stale-cap handling; nothing had been sent to clients. Importer handle_export_cancel erases DISCOVERING..PREPPED state (PREPPED: auth back to (E,-2), unfreeze) and aborts the MDS for later states. import_reverse restores the exporter's authority, clears auth/dirty/lock state, in ACKING removes caps created by the import, decrements importing on recorded sessions and closes forced sessions the client never opened, journals EImportFinish(false), notifies bystanders and unfreezes after their acks.

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks, internal
- **Map:** [subtree-migration.md › 3.12 Cancellation and failure handling](../cpp-map/08-multi-mds/subtree-migration.md#312-cancellation-and-failure-handling)
- **Code:** `src/mds/Migrator.cc:334-458 (Migrator::export_try_cancel)`, `src/mds/Migrator.cc:2132-2202 (Migrator::export_reverse)`, `src/mds/Migrator.cc:2540-2573 (Migrator::handle_export_cancel)`, `src/mds/Migrator.cc:2978-3106 (Migrator::import_reverse)`
- **Notes:** MExportDirCancel is sent only when notify_peer and (cluster not degraded or peer in clientreplay/active/stopping). Crash points: mds_kill_export_at 1-13 and mds_kill_import_at 1-10 (dev options; qa test_exports.py drives import 1, 3, 5 and export 9, 10). Known bug by code reading: reversal from IMPORT_LOGGINGSTART leaves PIN_IMPORTINGCAPS references and session importing counts behind (see forced-session entry).

<a id="f-multi-023"></a>

#### F-MULTI-023 · Rank failure during migration and resolve interplay

*Decides the outcome of every in-flight migration when the importer, exporter, a bystander or any other rank fails or stops.*

Migrator::handle_mds_failure_or_stop (from MDCache::handle_mds_failure and from handle_mds_map for ranks that restarted into starting/active or became stopped): exporters cancel exports whose importer failed and cancel every export in LOCKING..PREPPING when any rank fails; exporters drop failed bystanders from wait sets. Importers drop state for DISCOVERING..PREPPING, abort from PREPPED, import_reverse from LOGGINGSTART, register an ambiguous import and wait for the exporter's resolve in ACKING, and finish (with bystander notify) in FINISHING. A rank withholds its subtree resolve while any export or import is in progress (resolves_pending, sent from maybe_send_pending_resolves). A surviving importer in ACKING reverses if the recovered exporter claims the subtree in MMDSResolve, else finishes. A recovering exporter claims the region iff EExport did not reach its journal; a recovering importer announces ambiguous imports and keeps them unless another rank claims them.

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by mds-ranks, internal
- **Map:** [subtree-migration.md › 3.12 Cancellation and failure handling](../cpp-map/08-multi-mds/subtree-migration.md#312-cancellation-and-failure-handling) · [subtree-migration.md › 3.10 Journaling and ambiguous authority](../cpp-map/08-multi-mds/subtree-migration.md#310-journaling-and-ambiguous-authority) · [resolve-and-rejoin.md › 1. Purpose and responsibilities](../cpp-map/09-recovery/resolve-and-rejoin.md#1-purpose-and-responsibilities)
- **Code:** `src/mds/Migrator.cc:486-667 (Migrator::handle_mds_failure_or_stop)`, `src/mds/MDCache.cc:2892-2898 (MDCache::send_subtree_resolves)`, `src/mds/MDCache.cc:3352-3397 (MDCache::handle_resolve)`, `src/mds/MDCache.cc:3695-3748 (MDCache::disambiguate_my_imports)`
- **Notes:** Any rank failure aborts early-stage exports on every exporting rank, not only exports to the failed rank (Migrator.cc:519-528). The resolve stage and ambiguous-import disambiguation are owned by 01-cluster-membership-and-ha (recovery state machine); this entry is the migration side. See also F-CLU-075 (disambiguation in resolve), F-CLU-074.

<a id="f-multi-024"></a>

#### F-MULTI-024 · Freeze deadlock breaker (mds_freeze_tree_timeout)

*Export side of the freeze deadlock breaker.*

See F-CAPS-089. Export-specific: Migrator::find_stale_export_freeze checks exports in DISCOVERING or FREEZING on every tick while active and cancels an export whose freeze_tree_state->auth_pins has not changed for mds_freeze_tree_timeout (30 s) while num_remote_waiters > 0 or the parent dir is itself freezing.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal, mds-ranks
- **Gates:** mds_freeze_tree_timeout (float, default 30 s, dev)
- **Map:** [subtree-migration.md › 3.8 Freezing and auth pins](../cpp-map/08-multi-mds/subtree-migration.md#38-freezing-and-auth-pins)
- **Code:** `src/mds/Migrator.cc:268-318 (Migrator::find_stale_export_freeze)`, `src/mds/Locker.cc:758-778 (Locker::notify_freeze_waiter)`
- **Notes:** dump_export_states shows last_cum_auth_pins, last_cum_auth_pins_change and num_remote_waiters for DISCOVERING/FREEZING exports.

<a id="f-multi-025"></a>

#### F-MULTI-025 · Quiesce and quarantine guards on migration

*Migration view of the export refusals that quiesce and quarantine impose.*

See F-QOS-049 (quiesce) and F-QOS-066 (quarantine). Migration-specific: export pins that point elsewhere for a quiesced subtree are refused and retried on each tick until the quiesce is released.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, mgr
- **Map:** [subtree-migration.md › 3.8 Freezing and auth pins](../cpp-map/08-multi-mds/subtree-migration.md#38-freezing-and-auth-pins) · [balancer-and-pinning.md › 8. Edge cases, failure handling and gotchas](../cpp-map/08-multi-mds/balancer-and-pinning.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Migrator.cc:320-332 (Migrator::quiesce_overdrive_export)`, `src/mds/Migrator.cc:920-960 (Migrator::export_dir)`
- **Notes:** Quiesce and quarantine are owned by 10-qos-metrics-quiesce-quarantine; quiesce is driven by mgr/volumes snapshot workflows.

### Load accounting

Decayed popularity counters hit by metadata operations and the per-rank load derived from them. They feed the default balancer and Mantle and travel with exports; no client, mon or OSD reads them.

<a id="f-multi-026"></a>

#### F-MULTI-026 · Decayed popularity counters (DecayCounter, meta_load)

*Exponentially decayed hit counters per dirfrag and inode that quantify metadata load for the balancer.*

DecayCounter keeps one double and the last decay time; each access computes val = val*exp(el*k) + delta with k = ln(0.5)/half_life, and results below 0.01 snap to 0. Half-life mds_decay_halflife (5 s) is read only at MDCache construction; a steady r hits/s converges to about 7.21*r. Vectors have five types IRD, IWR, READDIR, FETCH, STORE; dirfrag scalar meta_load = IRD + 2*IWR + READDIR + 2*FETCH + 4*STORE. Each dirfrag has pop_me, pop_nested, pop_auth_subtree, pop_auth_subtree_nested and pop_lru_subdirs (MRU list of child dir inodes hit within the same auth subtree); CInode::pop holds only IRD/IWR. Encoding (v5/compat 4) decays and writes only val; decode restarts the decay clock with the receiver's rate.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal
- **Gates:** mds_decay_halflife (default 5 s, startup only)
- **Map:** [balancer-and-pinning.md › 3.2 Popularity accounting](../cpp-map/08-multi-mds/balancer-and-pinning.md#32-popularity-accounting)
- **Code:** `src/common/DecayCounter.cc:71 (DecayCounter::decay)`, `src/mds/mdstypes.h:686 (dirfrag_load_vec_t::meta_load)`, `src/mds/CDir.h:749-756 (CDir::pop_me)`
- **Notes:** CInode::pop is hit, exported and zeroed but no balancing decision reads it. No balancer state is stored in RADOS or the journal; popularity is carried only in exports.

<a id="f-multi-027"></a>

#### F-MULTI-027 · Popularity hits on metadata operations

*Charges every metadata operation, dirfrag fetch and dirfrag commit to the counters of the touched dirfrag and all its ancestors up to the root.*

hit_inode(in, type) hits in->pop and calls hit_dir on the primary parent dirfrag; hit_dir ignores stray dirfrags, hits pop_me, evaluates fragmentation and replication triggers, then walks to the root: every level gets pop_nested; if the start is auth, levels inside the starting auth subtree get pop_auth_subtree and push their inode to the front of the parent's pop_lru_subdirs, and every level gets pop_auth_subtree_nested. Hit sites: getattr/lookup (IRD), open (IWR if write mode else IRD), create/mknod/mkdir/symlink commit (IWR), setattr/setxattr/vxattr commit (IWR), readdir (READDIR weighted by entries), link, unlink/rmdir, rename (IWR on dirs), cap flush write-back (IWR), dirfrag fetch (FETCH) and commit (STORE). Hits on replica metadata update only pop_me and pop_nested.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [balancer-and-pinning.md › 3.2 Popularity accounting](../cpp-map/08-multi-mds/balancer-and-pinning.md#32-popularity-accounting)
- **Code:** `src/mds/MDBalancer.cc:1287 (MDBalancer::hit_inode)`, `src/mds/MDBalancer.cc:1325-1408 (MDBalancer::hit_dir)`, `src/mds/Server.cc:4412 (Server::handle_client_getattr)`
- **Notes:** Every operation updates O(depth) counters including the root dirfrag's, all under mds_lock (map section 6, serialization point 1, marked not inherent: additive and tolerant to staleness).

<a id="f-multi-028"></a>

#### F-MULTI-028 · Popularity transfer on export, import, subtree change, rename and split

*Keeps ancestor counters consistent when load moves between ranks or subtrees are created, merged, renamed or fragmented.*

At EXPORTING subtract_export removes the exported dirfrag's pop_auth_subtree from pop_nested and pop_auth_subtree_nested of all ancestors; CDir::encode_export ships pop_me and pop_auth_subtree, and CDir::finish_export zeroes them; the importer restores them in CDir::decode_import and add_import adds them to its ancestors; imported child dir inodes are appended to pop_lru_subdirs. adjust_subtree_auth subtracts a new subtree root's pop_auth_subtree from ancestors up to the enclosing root and a merge adds it back; directory renames call adjust_pop_for_rename on old and new parents; a split gives each child a copy of all four vectors scaled by 1/2^bits ("approximation").

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by internal, mds-ranks
- **Map:** [balancer-and-pinning.md › 3.2 Popularity accounting](../cpp-map/08-multi-mds/balancer-and-pinning.md#32-popularity-accounting) · [subtree-migration.md › 3.7 What state is transferred](../cpp-map/08-multi-mds/subtree-migration.md#37-what-state-is-transferred)
- **Code:** `src/mds/MDBalancer.cc:1419-1476 (MDBalancer::subtract_export)`, `src/mds/CDir.cc:2948-2979 (CDir::encode_export)`, `src/mds/MDCache.cc:1030-1038 (MDCache::adjust_subtree_auth)`, `src/mds/CDir.cc:1053-1084 (CDir::split)`

<a id="f-multi-029"></a>

#### F-MULTI-029 · Per-rank load (mds_load_t) and mds_bal_mode

*Reduces a rank's counters and process statistics to one load number exchanged in heartbeats.*

get_load builds mds_load_t {auth, all: dirfrag_load_vec_t; req_rate, cache_hit_rate, queue_len, cpu_load_avg}: auth = sum over root dirfrags of pop_auth_subtree_nested, all = sum of pop_nested, req_rate = delta of perf counter mds.request per second, cpu_load_avg = delta of utime+stime ticks from /proc/self/stat per second, cache_hit_rate = delta traverse_hit / delta traverse, queue_len = messenger dispatch queue length; rates refresh only if at least 1 s passed. mds_load(mode): mode 0 (default) = 0.8*auth.meta_load + 0.2*all.meta_load + req_rate + 10*queue_len; mode 1 = req_rate + 10*queue_len; mode 2 = cpu_load_avg; other values abort. send_heartbeat sets perf gauges load_cent = 100*mds_load and q = queue_len, so they refresh only while epochs run.

- **Class:** POSIX `internal` · exposure `config`, `metrics` · relied on by internal
- **Gates:** mds_bal_mode (default 0, dev)
- **Map:** [balancer-and-pinning.md › 3.3 Rank load: mds_load_t and get_load](../cpp-map/08-multi-mds/balancer-and-pinning.md#33-rank-load-mds_load_t-and-get_load)
- **Code:** `src/mds/MDBalancer.cc:345-412 (MDBalancer::get_load)`, `src/mds/MDBalancer.cc:324-343 (mds_load_t::mds_load)`
- **Notes:** Reads perf counters back from the logger; `perf reset` or disabled perf counters alter balancing (ADM staging note config-perf-health:23).

### Default balancer

The dynamic subtree partitioning balancer: a heartbeat epoch driven by rank 0, a global transfer plan computed identically on every rank, and per-rank export selection. Off unless the balance_automate file-system flag is set.

<a id="f-multi-030"></a>

#### F-MULTI-030 · balance_automate file-system flag

*The MDSMap flag that switches the default balancer (and everything that runs inside its heartbeat epochs) on or off.*

MDSMap flag CEPH_MDSMAP_BALANCE_AUTOMATE (1<<8), set with `ceph fs set <fs> balance_automate <bool>`; it is not in CEPH_MDSMAP_DEFAULTS and is never set implicitly, so it is off for new and upgraded file systems. Only rank 0's tick checks it (to start epochs); pins, fragmentation and hot-directory replication do not depend on it, while Mantle and the idle-import return run only inside epochs.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by mon, admin
- **Gates:** ceph fs set <fs> balance_automate <bool>
- **Map:** [balancer-and-pinning.md › 3.5 Gating flags and knobs](../cpp-map/08-multi-mds/balancer-and-pinning.md#35-gating-flags-and-knobs) · [balancer-and-pinning.md › 7.3 Monitor](../cpp-map/08-multi-mds/balancer-and-pinning.md#73-monitor)
- **Code:** `src/mon/FSCommands.cc:764-778 (SetHandler::handle, var balance_automate)`, `src/mds/MDSMap.h:251-258 (MDSMap::allows_balance_automate)`, `src/mds/MDBalancer.cc:279-310 (MDBalancer::tick)`
- **Notes:** doc/cephfs/mantle.rst never mentions the flag, so its walkthrough does nothing on a file system where it is off; qa multi-rank workload suites set balance_automate false.

<a id="f-multi-031"></a>

#### F-MULTI-031 · Heartbeat epoch exchange (MHeartbeat)

*Gives every rank the same per-epoch view of all ranks' loads and import maps so each can compute the global plan.*

Rank 0 starts an epoch from tick when balance_automate is set, it is active, mds_bal_interval > 0 and at least that many seconds passed (and the mds_bal_max/mds_bal_max_until test counters allow); tick runs every mds_tick_interval only while active and not laggy. send_heartbeat returns while the cluster is degraded; rank 0 increments beat_epoch and clears mds_load; each rank stores its own get_load, builds import_map[from] += pop_auth_subtree.meta_load over auth subtrees whose inode is auth elsewhere, and sends MHeartbeat (0x500: mds_load_t v2/2, __s32 beat, map<int32, float> import_map) to every other up rank. Receivers drop it unless active and not degraded; a message from rank 0 makes the receiver adopt rank 0's beat and send its own heartbeat; rank 0 ignores other beats. Once mds_load holds num_in_mds entries and at least one rank is in bal_rank_mask, the rank runs Mantle (if configured) or prep_rebalance.

- **Class:** POSIX `operational` · exposure `mds-wire`, `config` · relied on by mds-ranks, internal
- **Gates:** balance_automate flag; mds_bal_interval (default 10 s; 0 disables epochs); mds_bal_max / mds_bal_max_until (dev, default -1)
- **Map:** [balancer-and-pinning.md › 3.4 Heartbeat exchange](../cpp-map/08-multi-mds/balancer-and-pinning.md#34-heartbeat-exchange) · [balancer-and-pinning.md › 5.1 Messages](../cpp-map/08-multi-mds/balancer-and-pinning.md#51-messages)
- **Code:** `src/mds/MDBalancer.cc:454-582 (MDBalancer::send_heartbeat)`, `src/messages/MHeartbeat.h:52-64 (MHeartbeat::encode_payload)`
- **Notes:** About N^2 messages per epoch. A rank in up:stopping is in but ignores heartbeats, so no rank rebalances while any rank is stopping. Rank-0 failover restarts beat numbering at 1 (handle_mds_failure(0) clears mds_last_epoch_under_map). Option text for mds_bal_max_until says it stops the balancer after that time; the code re-enables epochs after it (map 3.5). The planner assumes the in-set is exactly 0..n-1 (mds_load.at(i) would throw on a gap; by code reading).

<a id="f-multi-032"></a>

#### F-MULTI-032 · Default rebalance target computation (prep_rebalance)

*Computes, identically on every rank, which ranks are exporters and importers and how much load each exporter sends to each importer.*

prep_rebalance clears the Migrator export queue, rescales all ranks' mds_load by load_fac = my auth.meta_load / my mds_load (1.0 if 0), sets target_load = total / |bal_rank_mask|, and applies hysteresis: masked ranks below target_load*(1+mds_bal_min_rebalance 0.1) record the epoch; a rank just recorded, or recorded fewer than mds_bal_overload_epochs (2) beats ago, stops; a rank never recorded is immediately overloaded. Masked ranks below target are importers, others exporters (unmasked ranks have target 0). Matching moves min(maxex, maxim) first against import history (exporters largest first, to ranks they imported from), then on odd beats largest exporters to least-loaded importers and on even beats smallest exporters to least-loaded importers, stopping below 0.001; each rank records targets only where it is the exporter and calls try_rebalance.

- **Class:** POSIX `operational` · exposure `config` · relied on by internal
- **Gates:** mds_bal_min_rebalance (0.1, dev); mds_bal_overload_epochs (2, dev); bal_rank_mask; mds_thrash_exports (non-zero disables planning)
- **Map:** [balancer-and-pinning.md › 3.6 Target computation (prep_rebalance)](../cpp-map/08-multi-mds/balancer-and-pinning.md#36-target-computation-prep_rebalance)
- **Code:** `src/mds/MDBalancer.cc:762-933 (MDBalancer::prep_rebalance)`, `src/mds/MDBalancer.cc:584-603 (MDBalancer::try_match)`
- **Notes:** Amounts are in each exporter's own rescaled units; ranks with different request/queue mixes are compared in a distorted unit (map section 8). Import-history matching does not check the mask. A busy rank exports in the first epoch it sees (no hysteresis on first overload).

<a id="f-multi-033"></a>

#### F-MULTI-033 · Export selection (try_rebalance, find_exports)

*Turns this rank's per-target load amounts into concrete dirfrags queued for export.*

Candidates are full-auth unambiguous subtree roots minus mdsdir subtrees, subtrees whose inode has its own pin/distributed policy/random state, and frozen/freezing ones. Per target: skip if amount < MIN_OFFLOAD (10) or amount*10*|targets| < target_load; pass A re-exports subtrees imported from that target with pop <= remaining; pass B re-exports other candidates with MIN_REEXPORT (5) < pop <= remaining; pass C walks candidates by popularity and calls find_exports. find_exports iterates pop_lru_subdirs (MRU first) over non-subtree-root dirfrags: pop within (need*0.8, need*1.2) is taken at once, bigger items are recursed into (unreplicated before replicated), smaller items >= midchunk are taken largest first; it returns if need < amount*mds_bal_min_start and aborts the round after 0.1 s ("balancer runs too long"). Chosen dirfrags go to Migrator::export_dir_nicely.

- **Class:** POSIX `operational` · exposure `config` · relied on by internal
- **Gates:** mds_bal_min_start (0.2); mds_bal_need_min/need_max (0.8/1.2); mds_bal_midchunk/minchunk (0.3/0.001)
- **Map:** [balancer-and-pinning.md › 3.7 Export selection (try_rebalance, find_exports)](../cpp-map/08-multi-mds/balancer-and-pinning.md#37-export-selection-try_rebalance-find_exports)
- **Code:** `src/mds/MDBalancer.cc:984-1152 (MDBalancer::try_rebalance)`, `src/mds/MDBalancer.cc:1154-1285 (MDBalancer::find_exports)`
- **Notes:** MIN_OFFLOAD, MIN_REEXPORT and the 0.1 s budget are hard-coded. Subtree roots, including pinned aux subtrees, are never candidates inside find_exports. Queued exports are discarded at the start of each round.

<a id="f-multi-034"></a>

#### F-MULTI-034 · Idle import return (mds_bal_idle_threshold)

*Sends imported subtrees with negligible load back to the rank that owns their parent.*

In try_rebalance, when mds_bal_idle_threshold > 0, a candidate subtree whose pop_auth_subtree.meta_load is below it, that is not the root and whose parent inode is auth on another rank is exported back to that rank immediately via export_dir_nicely.

- **Class:** POSIX `operational` · exposure `config` · relied on by admin
- **Gates:** mds_bal_idle_threshold (default 0 = off); balance_automate flag
- **Map:** [balancer-and-pinning.md › 3.7 Export selection (try_rebalance, find_exports)](../cpp-map/08-multi-mds/balancer-and-pinning.md#37-export-selection-try_rebalance-find_exports)
- **Code:** `src/mds/MDBalancer.cc:984-1152 (MDBalancer::try_rebalance)`
- **Notes:** Runs only inside a heartbeat epoch and only on ranks that reach try_rebalance (underloaded ranks return earlier in prep_rebalance).

<a id="f-multi-035"></a>

#### F-MULTI-035 · bal_rank_mask (balancer target ranks)

*Restricts which ranks the default balancer moves load to, so operators can keep some ranks for pinned subtrees.*

MDSMap string, default "-1", set with `ceph fs set <fs> bal_rank_mask <hex|-1|all|0|0x0>`; the mon validates with MDSMap::hex2bin (hex, at most 256 bits, at least one bit set; 0/0x0/-1/all special) and returns EINVAL with its error string. At the MDS "0"/"0x0" means an empty mask (no planning at all, Mantle included); -1/all/empty/unparsable means all 256 bits with mask count = max_mds; otherwise bit i = rank i and the count is the number of set bits. Unmasked ranks are never importers and have target 0, so they shed all unpinned load.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin` · relied on by mon, admin
- **Gates:** ceph fs set <fs> bal_rank_mask; MDSMap extended encoding v17 field bal_rank_mask
- **Map:** [balancer-and-pinning.md › 3.5 Gating flags and knobs](../cpp-map/08-multi-mds/balancer-and-pinning.md#35-gating-flags-and-knobs) · [balancer-and-pinning.md › 3.16 Pins versus the default balancer](../cpp-map/08-multi-mds/balancer-and-pinning.md#316-pins-versus-the-default-balancer)
- **Code:** `src/mon/FSCommands.cc:506-526 (SetHandler::handle, var bal_rank_mask)`, `src/mds/MDSMap.cc:1376-1449 (MDSMap::update_num_mdss_in_rank_mask_bitset)`, `src/mds/MDSMap.cc:1409 (MDSMap::hex2bin)`
- **Notes:** Quirks (map section 8): target_load divides by the popcount even for ranks that do not exist (0xff with 2 ranks divides by 8); hex longer than 16 digits is parsed in 16-digit chunks each widened to 64 bits, so masks above rank 63 are shifted unless the length is a multiple of 16; stoull accepts a valid prefix of a chunk. Printed by `fs dump`/`fs get`.

### Mantle Lua balancer

An optional Lua policy, stored as a RADOS object and named in the MDSMap, that replaces the default "how much load to send to whom" decision. Documented as experimental.

<a id="f-multi-036"></a>

#### F-MULTI-036 · Mantle Lua balancer policy

*Replaces the default "how much to send to whom" decision with a Lua script read from a RADOS object named in the MDSMap.*

When the MDSMap balancer string (set with `ceph fs set <fs> balancer <object>`, no validation, empty unsets; MDSMap extended v11) is non-empty, handle_heartbeat calls mantle_prep_rebalance; on a name change localize_balancer reads the whole object from the metadata pool (default namespace) and waits up to mds_bal_interval/2 s (integer) under mds_lock; timeout gives -ETIMEDOUT, read errors are returned. Each epoch a fresh lua_State (Lua >= 5.3; libraries _G, coroutine, string, math, table, utf8) gets globals whoami and mds[0..n-1] with keys auth.meta_load, all.meta_load, req_rate, queue_len, cpu_load_avg, plus BAL_LOG(level, ...); the chunk must return a table of integer rank keys to numbers with exactly n entries, else -EINVAL. On success try_rebalance runs with those targets (no mask, hysteresis or fresh target_load); on failure the default prep_rebalance runs.

- **Class:** POSIX `operational` · exposure `cluster-wire`, `admin`, `config` · relied on by mon, osd, admin
- **Gates:** MDSMap balancer string; balance_automate flag (Mantle runs only inside epochs); non-empty bal_rank_mask
- **Map:** [balancer-and-pinning.md › 3.9 Mantle (Lua policy)](../cpp-map/08-multi-mds/balancer-and-pinning.md#39-mantle-lua-policy) · [balancer-and-pinning.md › 5.2 Journal events and RADOS objects](../cpp-map/08-multi-mds/balancer-and-pinning.md#52-journal-events-and-rados-objects)
- **Code:** `src/mds/MDBalancer.cc:935-980 (MDBalancer::mantle_prep_rebalance)`, `src/mds/MDBalancer.cc:419-452 (MDBalancer::localize_balancer)`, `src/mds/Mantle.cc:50-114 (Mantle::balance)`, `src/mon/FSCommands.cc:494-505 (SetHandler::handle, var balancer)`
- **Notes:** Documented as experimental, not for production (doc/cephfs/mantle.rst). Hazards by code reading (map section 8): the wait has no predicate (a completion before the wait becomes -ETIMEDOUT after the full timeout; a spurious wakeup could cache a partial script and leave a dangling C_SafeCond); mds_bal_interval = 1 gives a 0 s timeout; Lua runs with no CPU or memory limit on the dispatch thread and the base library still provides dofile/loadfile; a script returning >= 10 for its own rank can reach ceph_assert(dest != whoami) in Migrator::export_dir; Lua 1-based array literals shift ranks; updating the object without renaming is not noticed. Example policy src/mds/balancers/greedyspill.lua.

<a id="f-multi-037"></a>

#### F-MULTI-037 · Mantle cluster log messages

*Reports Mantle load failures and script changes in the cluster log, where qa tests look for them.*

On a non-zero mantle_prep_rebalance result handle_heartbeat logs WRN "using old balancer; mantle failed for balancer=<name> : <strerror>" (every epoch while failing); after a successful load rank 0 logs INF "mantle balancer version changed: <name>". "balancer runs too long" from find_exports goes only to the daemon log.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin, tools
- **Map:** [balancer-and-pinning.md › 5.5 Admin commands and cluster log](../cpp-map/08-multi-mds/balancer-and-pinning.md#55-admin-commands-and-cluster-log)
- **Code:** `src/mds/MDBalancer.cc:571-579 (MDBalancer::handle_heartbeat)`
- **Notes:** qa/tasks/cephfs/test_mantle.py matches these strings but is not referenced by any qa suite and does not set balance_automate (map section 8).

### Export pins (ceph.dir.pin)

Static placement of directory subtrees on ranks through the ceph.dir.pin virtual xattr, its resolution through ancestors, per-tick enforcement, and the mgr/volumes commands that set pins.

<a id="f-multi-038"></a>

#### F-MULTI-038 · Set and remove ceph.dir.pin

*Lets users and tools pin a directory subtree to a rank by setting a virtual xattr on the directory.*

SETXATTR (0x01105) / RMXATTR (0x01106) of ceph.dir.pin is routed to the inode's auth rank (ESTALE if not cached or purging) and handle_client_setvxattr checks in order: MDS cap lacks p (MAY_SET_VXATTR) -> EACCES; not a directory or the root -> EINVAL; removal when already -1 -> success, nothing journaled; value not an int32 (boost::lexical_cast) -> EINVAL; value < 0 -> stored as -1 (unpinned); value >= MAX_MDS (256) -> EDOM. Then it xlocks policylock (rdlocks snaplock and parent snap/layout), projects the inode, sets inode_t::export_pin, bumps change_attr and ctime (rctime raised), journals EUpdate("set vxattr layout") and replies; the pin takes effect when the commit pops the projected inode (maybe_export_pin) and is enforced on the next tick. Replica ranks receive export_pin, flags and export_ephemeral_random_pin through the policylock state.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, mgr, rook
- **Gates:** mds_bal_export_pin (default true; false makes every pin resolve to none); MDS auth cap flag p
- **Map:** [balancer-and-pinning.md › 3.12 Export pins (ceph.dir.pin)](../cpp-map/08-multi-mds/balancer-and-pinning.md#312-export-pins-cephdirpin) · [balancer-and-pinning.md › 7.1 Linux kernel client](../cpp-map/08-multi-mds/balancer-and-pinning.md#71-linux-kernel-client)
- **Code:** `src/mds/Server.cc:6723-6755 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:5930 (Server::xlock_policylock)`, `src/mds/CInode.cc:522-552 (CInode::pop_and_dirty_projected_inode)`, `src/mds/CInode.cc:2250-2310 (CInode::encode_lock_ipolicy)`, `linux/fs/ceph/xattr.c:1107-1174 (ceph_sync_setxattr)`
- **Notes:** Only RMXATTR removes: a SETXATTR without a value (kclient sends CEPH_XATTR_REMOVE, libcephfs likewise) fails to parse and returns EINVAL (map 8.24). A rank >= max_mds but < 256 is accepted and parked (delayed pins). The setxattr returns on the early (unsafe) reply before EUpdate commits (mds_early_reply default true), so the pin is asynchronous. inode_t::export_pin is persisted (inode encoding v15+). The vxattr dispatch framework is owned by 04-namespace-and-posix-operations. See also F-ADM-094 (mgr/volumes pin commands).

<a id="f-multi-039"></a>

#### F-MULTI-039 · Pin read-back (InodeStat dir_pin, GETVXATTR)

*Lets clients read the pin settings of a directory.*

Every InodeStat (struct v2+) carries dir_pin = the inode's own export_pin (-1 when unset; inherited and ephemeral placement are not reflected); kclient and libcephfs serve getxattr ceph.dir.pin from that cached value after forcing a GETATTR, updating it only from auth replies or newer versions; the name is hidden from listxattr. ceph.dir.pin.random and ceph.dir.pin.distributed are read with CEPH_MDS_OP_GETVXATTR (0x00106) on the auth rank, which replies in reply_extra_bl (ENCODE_START(1,1) + string): random as default double formatting (e.g. 0.01, 0), distributed as 0/1, pin as integer; non-directories get ENODATA.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Legacy parts** (old-clients): Client-side EOPNOTSUPP to ENODATA fallback towards an MDS without CEPHFS_FEATURE_OP_GETVXATTR (bit 17); the unversioned InodeStat without dir_pin for sessions without CEPHFS_FEATURE_REPLY_ENCODING (bit 9). The kernel (r_feature_needed) and libcephfs (make_request feature_needed) fail a GETVXATTR locally with EOPNOTSUPP, mapped to ENODATA, only when the MDS session lacks CEPHFS_FEATURE_OP_GETVXATTR (bit 17); the baseline MDS advertises bit 17, and C++ MDS releases before 16.2.11, 17.2.6 and 18.2.0 lack it. dir_pin is missing only from the unversioned InodeStat that the MDS sends to sessions without CEPHFS_FEATURE_REPLY_ENCODING (bit 9); the 7.3 kernel and current libcephfs have bit 9 and receive InodeStat v9. Evidence: `linux/fs/ceph/mds_client.c:3718-3722 (__do_request)`, `linux/fs/ceph/inode.c:3061 (ceph_do_getvxattr)`, `linux/fs/ceph/xattr.c:1002-1006 (__ceph_getxattr)`, `src/client/Client.cc:2340 (Client::make_request)`, `src/client/Client.cc:8463-8470 (Client::_getvxattr)`, `src/mds/cephfs_features.h:48 (CEPHFS_FEATURE_OP_GETVXATTR)`, `src/mds/cephfs_features.h:74 (CEPHFS_FEATURES_ALL)`, `src/mds/CInode.cc:4315-4316 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4371-4427 (CInode::encode_inodestat)`, `linux/fs/ceph/mds_client.h:48 (CEPHFS_FEATURES_CLIENT_SUPPORTED, REPLY_ENCODING)`.
- **Gates:** CEPHFS_FEATURE_OP_GETVXATTR (17) for random/distributed; without it kclient fails locally with EOPNOTSUPP mapped to ENODATA
- **Map:** [balancer-and-pinning.md › 7.1 Linux kernel client](../cpp-map/08-multi-mds/balancer-and-pinning.md#71-linux-kernel-client) · [balancer-and-pinning.md › 7.2 libcephfs, ceph-fuse, NFS-Ganesha](../cpp-map/08-multi-mds/balancer-and-pinning.md#72-libcephfs-ceph-fuse-nfs-ganesha)
- **Code:** `src/mds/CInode.cc:4361 (CInode::encode_inodestat)`, `src/mds/Server.cc:7480-7491 (Server::handle_client_getvxattr)`, `linux/fs/ceph/mds_client.c:185-190 (parse_reply_info_in)`, `linux/fs/ceph/xattr.c:985-1006 (__ceph_getxattr)`
- **Notes:** The mgr never reads pins back. The InodeStat encoding is owned by 02-client-sessions-and-protocol.

<a id="f-multi-040"></a>

#### F-MULTI-040 · Pin inheritance and precedence resolution

*Determines the effective rank of any directory from its own and its ancestors' pin settings.*

CInode::get_export_pin(inherit) returns none when mds_bal_export_pin is false; otherwise it walks towards the root, stopping at system inodes (so the root is never consulted), inodes without a parent dentry and unlinked directories. Precedence, closest first: own export_pin >= 0; own distributed flag (each own dirfrag f -> hash(ino, f)); own random state when the first policy-bearing ancestor above is random -> hash(own ino); nearest ancestor explicit pin; nearest ancestor distributed flag -> hash(that ino, its dirfrag holding the path); nearest random-pinned ancestor under a random policy -> hash(its ino); otherwise none (balancer-managed). inherit=false reports only the inode's own setting (used by queueing, try_rebalance exclusion and get subtrees). Sentinels: MDS_RANK_NONE -1, MDS_RANK_EPHEMERAL_DIST -2, MDS_RANK_EPHEMERAL_RAND -3. Pins live in the directory's own inode_t, so they move with the directory on rename; inherited placement changes with the new ancestors.

- **Class:** POSIX `extension` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, mgr, rook, admin
- **Map:** [balancer-and-pinning.md › 3.12 Export pins (ceph.dir.pin)](../cpp-map/08-multi-mds/balancer-and-pinning.md#312-export-pins-cephdirpin)
- **Code:** `src/mds/CInode.cc:5781 (CInode::get_export_pin)`, `src/mds/CDir.cc:2928-2946 (CDir::get_export_pin)`, `src/include/cephfs/types.h:75-77 (MDS_RANK_EPHEMERAL_DIST)`
- **Notes:** The "closest parent wins" rule is documented to users in doc/cephfs/multimds.rst. Clients only set values; they observe the effect through which rank answers.

<a id="f-multi-041"></a>

#### F-MULTI-041 · Pin enforcement and aux subtrees (handle_export_pins)

*Moves pinned dirfrags to their target rank and keeps locally pinned dirfrags as separate subtrees, on every tick.*

maybe_export_pin (on pin commit, policylock update, dirfrag open, import, replay, config or max_mds change) puts an inode in export_pin_queue if any auth dirfrag needs work. Each tick handle_export_pins pass 1, per queued inode and auth dirfrag: distributed and below the minimum frag bits -> fast split; target this rank -> make the dirfrag a subtree root marked CDir::STATE_AUXSUBTREE; target another rank -> export_dir if it has head items; target none -> clear AUXSUBTREE and try_subtree_merge; frozen/creating dirfrags retry later. Pass 2 recomputes the inherited pin of every auth subtree root and exports it when the pin is a valid rank other than this one (moving a subtree back under the nearest pinned ancestor after its own pin is removed). AUXSUBTREE roots are never merged into their parent subtree, are kept across fragmentation, and are cleared on export, on pin removal and by trim of empty clean pinned subtrees.

- **Class:** POSIX `extension` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, mgr, rook, admin
- **Gates:** mds_bal_export_pin (default true); mds_tick_interval (default 5 s)
- **Map:** [balancer-and-pinning.md › 3.12 Export pins (ceph.dir.pin)](../cpp-map/08-multi-mds/balancer-and-pinning.md#312-export-pins-cephdirpin)
- **Code:** `src/mds/MDBalancer.cc:152-277 (MDBalancer::handle_export_pins)`, `src/mds/CInode.cc:5602-5671 (CInode::maybe_export_pin)`, `src/mds/MDCache.cc:1069-1072 (MDCache::try_subtree_merge_at)`
- **Notes:** Empty dirfrags are not moved by pins and their inode is rescanned each tick until trimmed. A refused export (target not active, quiesced, etc.) is retried every tick. Removing a pin does not move data back unless an ancestor has an effective pin. With mds_bal_export_pin=false existing AUXSUBTREE markers are no longer cleared by the tick. Pass 2 walks to the root for every auth subtree each tick (cost linear in subtrees x depth). Aux subtrees in the subtree map are also catalogued by 05-metadata-cache-and-dirfrags. See also F-CACHE-040 (aux subtrees).

<a id="f-multi-042"></a>

#### F-MULTI-042 · Delayed pins (target rank >= max_mds)

*Accepts a pin to a rank that does not exist yet and applies it when max_mds grows.*

In handle_export_pins pass 1 an inode whose own pin is >= max_mds is moved to export_pin_delayed_queue (STATE_DELAYEDEXPORTPIN); MDCache::handle_mdsmap rescans the delayed queue on every MDSMap and requeues inodes once max_mds exceeds their pin. A pin to a rank < max_mds that is not active makes export_dir refuse every tick.

- **Class:** POSIX `extension` · exposure `internal` · relied on by admin
- **Map:** [balancer-and-pinning.md › 3.12 Export pins (ceph.dir.pin)](../cpp-map/08-multi-mds/balancer-and-pinning.md#312-export-pins-cephdirpin) · [balancer-and-pinning.md › 8. Edge cases, failure handling and gotchas](../cpp-map/08-multi-mds/balancer-and-pinning.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDBalancer.cc:152-277 (MDBalancer::handle_export_pins)`, `src/mds/MDCache.cc:14647-14687 (MDCache::handle_mdsmap)`
- **Notes:** qa test_phantom_rank covers this. No error is returned to the client for an out-of-range rank below 256.

<a id="f-multi-043"></a>

#### F-MULTI-043 · Pins veto balancer exports

*Keeps the default balancer and Mantle from moving pinned subtrees.*

try_rebalance never selects a subtree root whose inode has its own pin, distributed policy or random state, and find_exports never selects subtree roots; Migrator::export_dir refuses any non-empty dirfrag whose effective (inherited, hash-resolved) pin points to another rank (CDir::is_exportable), except while the rank is stopping. Heartbeat import maps include pinned imports, so import-history matching can plan transfers that cannot be satisfied.

- **Class:** POSIX `operational` · exposure `internal` · relied on by admin
- **Map:** [balancer-and-pinning.md › 3.16 Pins versus the default balancer](../cpp-map/08-multi-mds/balancer-and-pinning.md#316-pins-versus-the-default-balancer)
- **Code:** `src/mds/CDir.cc:2928-2946 (CDir::is_exportable)`, `src/mds/Migrator.cc:920-928 (Migrator::export_dir)`

### Ephemeral pins

Hash-based placement policies (ceph.dir.pin.distributed, ceph.dir.pin.random) that spread dirfrags or directories over ranks with a consistent hash over max_mds.

<a id="f-multi-045"></a>

#### F-MULTI-045 · Distributed ephemeral pins (ceph.dir.pin.distributed)

*Spreads a directory's dirfrags, and the child subtrees under them, over all ranks by a consistent hash; used to spread subvolumes of a group.*

Stored as bit 0 (F_EPHEMERAL_DISTRIBUTED_PIN) of inode_t::flags; set/remove like ceph.dir.pin (directory, not root; value via strict_strtob: true/false case-insensitive or base-10 integer, non-zero true, else EINVAL; removing when unset is a no-op). The directory is fragmented to at least n bits with 2^n >= mds_export_ephemeral_distributed_factor (2) * max_mds (0 when max_mds <= 1; max_mds 2 -> 2 bits, 3 -> 3, 5 -> 4): each auth dirfrag below the minimum is fast-split with max(mds_bal_split_bits, minimum) bits and merges below n bits are blocked. Each dirfrag f goes to hash_into_rank_bucket(ino, f); children follow the dirfrag holding their dentry and are not separate subtrees unless they have their own pin. CInode::STATE_DISTEPHEMERALPIN is recomputed from the flag and travels with exports.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, mgr, rook
- **Gates:** mds_export_ephemeral_distributed (default true); mds_export_ephemeral_distributed_factor (default 2, 1-100); mds_bal_export_pin
- **Map:** [balancer-and-pinning.md › 3.13 Distributed ephemeral pins (ceph.dir.pin.distributed)](../cpp-map/08-multi-mds/balancer-and-pinning.md#313-distributed-ephemeral-pins-cephdirpindistributed)
- **Code:** `src/mds/Server.cc:6795-6828 (Server::handle_client_setvxattr)`, `src/mds/MDCache.cc:14677-14686 (MDCache::handle_mdsmap)`, `src/mds/MDBalancer.cc:648-652 (MDBalancer::queue_split)`, `src/mds/CDir.cc:3906-3914 (CDir::should_merge)`
- **Notes:** Children whose names hash into the same dirfrag share a rank; a tiny directory is force-split anyway. On a max_mds change, ephemeral pins are re-evaluated before the minimum frag bits are recomputed (map 8.16). doc/cephfs/multimds.rst says ephemeral policies are off by default "in Octopus" but the options default to true. The forced split mechanism itself is owned by 05-metadata-cache-and-dirfrags.

<a id="f-multi-046"></a>

#### F-MULTI-046 · Random ephemeral pins (ceph.dir.pin.random)

*Pins a random fraction of the directories below a policy directory to hash-chosen ranks.*

Policy inode_t::export_ephemeral_random_pin (double): set on a directory (not root) via lexical_cast<double> (EINVAL on parse error), < 0 or > 1 -> EDOM, > mds_export_ephemeral_random_max (0.01) -> EINVAL; removal sets 0. Setting or removing it re-evaluates nothing immediately. When a directory inode is created by mkdir or instantiated from a dirfrag fetch below the policy, maybe_ephemeral_rand rolls uniform(0,1) <= min(policy, random_max) of the nearest random policy (0 if an explicit pin or distributed policy is closer); on success the inode gets in-memory STATE_RANDEPHEMERALPIN and all its auth dirfrags are exported to or kept as aux subtrees on hash_into_rank_bucket(ino). The state is journaled as EMetaBlob fullbit STATE_EPHEMERAL_RANDOM (1<<4), carried in exports and replica encodings, and re-rolled when the inode is reloaded after its journal entries were trimmed.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, mgr
- **Gates:** mds_export_ephemeral_random (default true); mds_export_ephemeral_random_max (default 0.01, 0-1); mds_bal_export_pin
- **Map:** [balancer-and-pinning.md › 3.14 Random ephemeral pins (ceph.dir.pin.random)](../cpp-map/08-multi-mds/balancer-and-pinning.md#314-random-ephemeral-pins-cephdirpinrandom)
- **Code:** `src/mds/Server.cc:6756-6794 (Server::handle_client_setvxattr)`, `src/mds/CInode.cc:5713 (CInode::maybe_ephemeral_rand)`, `src/mds/CInode.cc:5851-5881 (CInode::get_ephemeral_rand)`, `src/mds/journal.cc:636-645 (EMetaBlob::fullbit::update_inode)`
- **Notes:** NaN passes validation and behaves as no policy; lowering random_max lowers the effective probability of existing policies; with the default cap a small tree is usually not pinned at all. Removing the random policy above a random-pinned directory clears its state in pass 2 but does not requeue it, so its aux subtree stays (map 8.13). A receiving replica only ever sets the random bit, never clears it.

<a id="f-multi-047"></a>

#### F-MULTI-047 · Consistent hash into a rank bucket (hash_into_rank_bucket)

*Maps an inode (and optionally a dirfrag) to a rank so that changing max_mds moves as few placements as possible.*

h = rjhash64(ino); for a non-root frag h = rjhash64(h + rjhash64(fg.value())) where value() is the 24-bit left-aligned frag value without depth; then the Lamping-Veach jump consistent hash over max_mds buckets (h = h*2862933555777941757 + 1; j = (b+1) * (2^31 / ((h >> 33) + 1))). Returns MDS_RANK_NONE when max_mds is 0. The bucket count is max_mds, not the set of active ranks; frags of different depth with equal value() hash identically, so the leftmost child of a split keeps its parent's rank.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [balancer-and-pinning.md › 3.15 Consistent hash (MDCache::hash_into_rank_bucket)](../cpp-map/08-multi-mds/balancer-and-pinning.md#315-consistent-hash-mdcachehash_into_rank_bucket)
- **Code:** `src/mds/MDCache.cc:948-967 (MDCache::hash_into_rank_bucket)`, `src/include/hash.h:28 (rjhash64)`
- **Notes:** Because the hash uses max_mds, during resizing or rank failure pins may point at ranks that cannot accept exports; exports are refused and retried each tick. The exact function is not a contract (map 7.7).

<a id="f-multi-048"></a>

#### F-MULTI-048 · Ephemeral pin redistribution on max_mds and config changes

*Re-places ephemerally pinned dirfrags when the number of ranks or the ephemeral options change.*

MDCache::handle_mdsmap rescans the delayed pin queue on every MDSMap; when max_mds changes it calls maybe_export_pin for every ephemerally pinned inode (export_ephemeral_pins) and recomputes export_ephemeral_dist_frag_bits. Changing mds_export_ephemeral_distributed or mds_export_ephemeral_random at runtime calls maybe_export_pin(true) for all ephemeral pins; an inode whose ephemeral pin now resolves to a rank or none clears its ephemeral state and, if needed, requeues to clear its aux subtrees.

- **Class:** POSIX `extension` · exposure `config` · relied on by mon, admin
- **Gates:** ceph fs set <fs> max_mds; mds_export_ephemeral_distributed; mds_export_ephemeral_random
- **Map:** [balancer-and-pinning.md › 3.12 Export pins (ceph.dir.pin)](../cpp-map/08-multi-mds/balancer-and-pinning.md#312-export-pins-cephdirpin) · [balancer-and-pinning.md › 3.13 Distributed ephemeral pins (ceph.dir.pin.distributed)](../cpp-map/08-multi-mds/balancer-and-pinning.md#313-distributed-ephemeral-pins-cephdirpindistributed)
- **Code:** `src/mds/MDCache.cc:14650-14675 (MDCache::handle_mdsmap)`, `src/mds/MDCache.cc:232-251 (MDCache::handle_conf_change)`
- **Notes:** Re-evaluation on a max_mds change runs before the new minimum frag bits are computed, so it still uses the previous minimum (MDCache.cc:14666-14686). qa test_ephemeral_pin_grow_mds / shrink_mds check minimal movement.

### Rank shrink and hot-directory replication

Subtree hand-off when max_mds is reduced, and popularity-driven replication of read-hot dirfrags to every active rank.

<a id="f-multi-049"></a>

#### F-MULTI-049 · Subtree hand-off by a stopping rank (max_mds decrease)

*Empties a rank that is being stopped by exporting all its subtrees to other ranks, regardless of pins.*

MDCache::shutdown_pass on a stopping rank other than 0 clears the export queue, runs handle_export_pins itself (the stopping rank does not run MDBalancer::tick), then collects every auth subtree except ~mdsN, frozen/freezing, ambiguous, already-exporting and (while max_mds > 0) ephemerally pinned ones, and sends each with export_dir_nicely (queued, throttled) to the authority of its parent inode, replacing any destination > 0 that is not up:active with rank 0. export_dir ignores pins while the rank is stopping. Because the stopping rank itself is not active, subtrees whose parent inode it also owns go to rank 0; if rank 0 is not active the export is refused and retried on the next pass.

- **Class:** POSIX `operational` · exposure `mds-wire`, `admin` · relied on by mon, admin, mds-ranks
- **Gates:** ceph fs set <fs> max_mds <n>
- **Map:** [subtree-migration.md › 1. Purpose and responsibilities](../cpp-map/08-multi-mds/subtree-migration.md#1-purpose-and-responsibilities) · [balancer-and-pinning.md › 3.12 Export pins (ceph.dir.pin)](../cpp-map/08-multi-mds/balancer-and-pinning.md#312-export-pins-cephdirpin) · [balancer-and-pinning.md › 8. Edge cases, failure handling and gotchas](../cpp-map/08-multi-mds/balancer-and-pinning.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDCache.cc:8043-8095 (MDCache::shutdown_pass)`, `src/mds/Migrator.cc:920-930 (Migrator::export_dir)`
- **Notes:** Reducing max_mds completes only after the stopping rank exported everything; no rank rebalances while any rank is stopping (it ignores heartbeats). The stopping state and transition to down:stopped are owned by 01-cluster-membership-and-ha.

<a id="f-multi-050"></a>

#### F-MULTI-050 · Hot-directory replication (REP_ALL, MDirUpdate)

*Replicates read-hot dirfrags to every active rank and advertises the replicas so clients can spread reads.*

In hit_dir, at most once per sample period, for an auth unambiguous dirfrag that can still replicate: when pop_auth_subtree[type] + 10*pop_me[IRD or READDIR] >= mds_bal_replicate_threshold (8000) it sets dir_rep = REP_ALL, sends MDirUpdate (0x206, v1: from_mds, dirfrag, dir_rep, discover = 5, dir_rep_by, path) to every active rank not yet replicating it, and reduces the local IRD counters; below mds_bal_unreplicate_threshold (0 by default, i.e. never) on a replicated non-root dirfrag it sets REP_NONE and tells replicas. A receiver without the dirfrag discovers the path (retried while tried_discover < 5) and becomes a replica. Clients see replicas in the DirStat dist list (replicas plus auth, from the auth only, unless mds_forward_all_requests_to_auth); kclient stores up to 4 dist entries per frag and sends USE_ANY_MDS requests routed by dentry name hash to a random listed active rank.

- **Class:** POSIX `operational` · exposure `mds-wire`, `client-wire`, `config` · relied on by kclient, mds-ranks
- **Gates:** mds_bal_replicate_threshold (8000); mds_bal_unreplicate_threshold (0); mds_bal_sample_interval (3 s); mds_forward_all_requests_to_auth (default false; true suppresses dist)
- **Map:** [balancer-and-pinning.md › 3.10 Hot-directory replication](../cpp-map/08-multi-mds/balancer-and-pinning.md#310-hot-directory-replication) · [balancer-and-pinning.md › 5.1 Messages](../cpp-map/08-multi-mds/balancer-and-pinning.md#51-messages)
- **Code:** `src/mds/MDBalancer.cc:1325-1408 (MDBalancer::hit_dir)`, `src/mds/MDCache.cc:11348-11394 (MDCache::send_dir_updates)`, `src/mds/MDCache.cc:11396-11439 (MDCache::handle_dir_update)`, `linux/fs/ceph/mds_client.c:1466-1486 (__choose_mds)`
- **Notes:** dir_rep is not persisted to RADOS; the root dirfrag is REP_ALL only when the file system is created, so after a restart it starts REP_NONE. Replication is never re-evaluated once every active rank replicates the dirfrag. qa check-counter expects mds_cache.dir_update/dir_update_receipt >= 3 with mds_bal_replicate_threshold 1. The DirStat dist encoding and replica primitives are owned by 05-metadata-cache-and-dirfrags. See also F-CACHE-079.

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-multi-044"></a>F-MULTI-044 | [F-ADM-094](11-security-admin-and-observability.md#f-adm-094) · mgr/volumes: subvolume and group pinning |

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
