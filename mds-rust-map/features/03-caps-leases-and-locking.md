# Capabilities, leases and locking: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [capabilities.md](../cpp-map/02-client-protocol/capabilities.md) · [leases-lock-caches-and-async-dirops.md](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md) · [lock-types-and-state-machines.md](../cpp-map/05-locking/lock-types-and-state-machines.md) · [acquiring-locks.md](../cpp-map/05-locking/acquiring-locks.md)

## 1. Scope

This category covers what the C++ MDS does to let clients cache and mutate metadata safely: the capability (cap) model and its wire protocol (issue, revoke, flush, release, max_size, truncate notification, snapflush, stale handling, late-revoke detection, cache-pressure recall, cap migration messages), dentry leases and directory-wide validity, lock caches with the kernel's asynchronous create/unlink and delegated inode numbers, the metadata lock types and their state machines, and lock acquisition (rdlock/wrlock/xlock, ordering, implicit locks, local and remote auth pins, remote locks, retries). Sources are the fact-checked map pages capabilities.md, leases-lock-caches-and-async-dirops.md, lock-types-and-state-machines.md and acquiring-locks.md, plus the recall section of sessions.md.

Excluded and owned elsewhere: session lifecycle, counted messages, reconnect message contents and feature negotiation (02-client-sessions-and-protocol); the recovery state machine and resolve (01-cluster-membership-and-ha); request lifecycle, unsafe/safe replies, completed-request replay, getattr batching and per-op handlers (04-namespace-and-posix-operations); path traversal, cache replication and dirfrag split/merge (05-metadata-cache-and-dirfrags); osdmap epoch barriers, OpenFileTable, InoTable and SessionMap storage (06-journal-and-storage); subtree export/import (07-multi-mds); dirstat/rstat propagation and snapshot semantics (08-snapshots-quotas-subvolumes); quiesce, quarantine and client metrics (10-qos-metrics-quiesce-quarantine); perf-counter and admin frameworks (11-security-admin-and-observability). Where an entry touches those mechanisms it describes only the cap or lock side and links to the owning page.

Key status fact: since commit 9b4a93f2b7b the C++ MDS never creates a lock cache, so it never grants Dc/Dr and kernel async create/unlink never happen (confirmed by a 3-investigator panel by code reading and git history; no test covers it; not observed at runtime).

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

87 features: 86 used by current peers (9 of them with legacy parts) and 1 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**Capability model and allowed caps**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-001](#f-caps-001) | Cap bit model and composite masks | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-002](#f-caps-002) | Open mode to wanted caps | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-003](#f-caps-003) | Capability record, cap ids and sequence numbers | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-004](#f-caps-004) | Lock-state-derived allowed caps *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-005](#f-caps-005) | Loner selection (exclusive client) | internal | internal | internal |
| [F-CAPS-006](#f-caps-006) | Wanted-cap-driven stable-state policy | internal | internal | internal |
| [F-CAPS-008](#f-caps-008) | Lazy I/O cap (Fl) | extension | client-wire | kclient, libcephfs |

**Cap issue, revocation and client cap messages**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-009](#f-caps-009) | Caps in request replies *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-010](#f-caps-010) | New caps on open, create, mknod and mkdir | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-011](#f-caps-011) | Cap grant and revoke messages (GRANT/REVOKE) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-013](#f-caps-013) | Client cap message processing (handle_client_caps) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-014](#f-caps-014) | Wanted caps tracking (issue_seq rule) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-015](#f-caps-015) | Xattr blob and inline data carried on caps *(legacy parts)* | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-016](#f-caps-016) | Truncate notification (CEPH_CAP_OP_TRUNC) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Dirty metadata flush, max_size and file size recovery**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-018](#f-caps-018) | Dirty metadata flush and FLUSH_ACK *(legacy parts)* | supporting | client-wire, on-disk | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-019](#f-caps-019) | Flush idempotency (completed_flushes, oldest_flush_tid) | supporting | client-wire, on-disk, metrics | kclient, libcephfs, ceph-fuse, ganesha, mon |
| [F-CAPS-020](#f-caps-020) | Synchronous flush and journal-flush heuristics | supporting | client-wire, config | kclient, libcephfs, ganesha, ceph-fuse |
| [F-CAPS-021](#f-caps-021) | max_size and per-client writeable ranges (client_ranges) | supporting | client-wire, on-disk, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-022](#f-caps-022) | File size recovery after loss of a writer | supporting | cluster-wire, on-disk | osd, kclient, libcephfs, internal, ceph-fuse |

**Snapshot flush (snapflush)**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-023](#f-caps-023) | Snapshot COW of cap state and SNAP_SYNC gating | extension | on-disk | kclient, libcephfs, ceph-fuse |
| [F-CAPS-024](#f-caps-024) | FLUSHSNAP / FLUSHSNAP_ACK protocol | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse |
| [F-CAPS-025](#f-caps-025) | Null snapflush inference | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse |
| [F-CAPS-026](#f-caps-026) | Snapflush nudge | extension | config | kclient, libcephfs, ceph-fuse |
| [F-CAPS-027](#f-caps-027) | XLOCKSNAP: truncate with pending snapflush | extension | internal | kclient, libcephfs |

**Cap release, staleness, recall and late revocation**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-028](#f-caps-028) | Batched cap release (MClientCapRelease) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-029](#f-caps-029) | Cap and dentry-lease releases embedded in requests | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-030](#f-caps-030) | Stale-session cap handling *(legacy parts)* | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha, osd |
| [F-CAPS-031](#f-caps-031) | Late revocation warnings, health and eviction | operational | metrics, config | mon, mgr, admin |
| [F-CAPS-032](#f-caps-032) | Cache-pressure cap recall (CEPH_SESSION_RECALL_STATE) | operational | client-wire, metrics, config, admin | kclient, libcephfs, ceph-fuse, ganesha, mon, mgr |
| [F-CAPS-034](#f-caps-034) | Cap re-establishment after MDS failover (cap side) | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Caps across multiple ranks**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-035](#f-caps-035) | Replica caps and MInodeFileCaps | internal | mds-wire | mds-ranks, kclient, libcephfs |
| [F-CAPS-036](#f-caps-036) | Cap migration to clients (EXPORT / IMPORT) | supporting | client-wire, mds-wire | kclient, libcephfs, ceph-fuse, ganesha, mds-ranks |

**Dentry leases and directory-wide validity**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-038](#f-caps-038) | Dentry lease issue | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-039](#f-caps-039) | Negative dentry leases | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-040](#f-caps-040) | CEPH_LEASE_PRIMARY_LINK hint | supporting | client-wire | kclient, libcephfs |
| [F-CAPS-042](#f-caps-042) | Lease renewal (CEPH_MDS_LEASE_RENEW) | supporting | client-wire | kclient |
| [F-CAPS-043](#f-caps-043) | Lease revocation on dentry change | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-044](#f-caps-044) | Lease expiry and cleanup | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-045](#f-caps-045) | Infinite leases on snapshot listings | extension | client-wire | kclient, libcephfs |
| [F-CAPS-046](#f-caps-046) | Directory-wide dentry validity via Fs | supporting | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Lock caches, async directory operations and delegated inode numbers**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-047](#f-caps-047) | Directory-operation cap bits Dc and Dr | supporting | client-wire | kclient |
| [F-CAPS-048](#f-caps-048) | Lock cache creation | internal | config | kclient, internal |
| [F-CAPS-049](#f-caps-049) | Lock cache attachment | internal | internal | kclient, internal |
| [F-CAPS-050](#f-caps-050) | Lock cache invalidation and lifecycle | internal | internal | kclient, internal |
| [F-CAPS-051](#f-caps-051) | Forced auth pins for lock-cache requests | internal | internal | kclient, internal |
| [F-CAPS-052](#f-caps-052) | Forcing async-operation flush for readers | supporting | internal | kclient, internal |
| [F-CAPS-053](#f-caps-053) | Per-session inode-number preallocation | internal | on-disk, config | internal, kclient, libcephfs |
| [F-CAPS-054](#f-caps-054) | Inode-number delegation to clients | supporting | client-wire, config | kclient |
| [F-CAPS-057](#f-caps-057) | Asynchronous create (kclient) | supporting | client-wire | kclient |
| [F-CAPS-058](#f-caps-058) | Asynchronous unlink (kclient) | supporting | client-wire | kclient |
| [F-CAPS-059](#f-caps-059) | Replay of unsafe and async creates after failover | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |

**Lock types and state machines**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-061](#f-caps-061) | Per-object lock catalogue *(legacy parts)* | internal | mds-wire | internal, mds-ranks |
| [F-CAPS-062](#f-caps-062) | SimpleLock state machine (sm_simplelock) *(legacy parts)* | internal | internal | internal, mds-ranks |
| [F-CAPS-063](#f-caps-063) | ScatterLock state machine (nestlock, dirfragtreelock) *(legacy parts)* | internal | internal | internal, mds-ranks |
| [F-CAPS-064](#f-caps-064) | FileLock state machine (sm_filelock) | internal | internal | internal, mds-ranks |
| [F-CAPS-065](#f-caps-065) | XSYN: rdlock while a loner keeps buffered data | internal | internal | internal |
| [F-CAPS-066](#f-caps-066) | Local locks (versionlock, quiescelock) | internal | internal | internal |
| [F-CAPS-067](#f-caps-067) | Gather engine (eval_gather) and cap revocation gather | supporting | internal | kclient, libcephfs, ceph-fuse, ganesha, internal |
| [F-CAPS-068](#f-caps-068) | Lock-mode grant rules (rdlock / wrlock / xlock compatibility) | supporting | internal | internal |
| [F-CAPS-069](#f-caps-069) | Lock-request-driven state changes and journal nudging | internal | internal | internal |
| [F-CAPS-070](#f-caps-070) | Scatter data exchange and writebehind | internal | mds-wire, on-disk | internal, mds-ranks |
| [F-CAPS-071](#f-caps-071) | Scatter nudging | internal | config | internal, mds-ranks |
| [F-CAPS-072](#f-caps-072) | Inter-MDS lock protocol (MLock) *(legacy parts)* | internal | mds-wire | mds-ranks |
| [F-CAPS-073](#f-caps-073) | Replica-initiated lock state requests | internal | mds-wire | mds-ranks |
| [F-CAPS-074](#f-caps-074) | Lock state across replication, migration and rejoin | internal | mds-wire | mds-ranks |

**Lock acquisition**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-075](#f-caps-075) | acquire_locks and lock vectors | supporting | internal | internal |
| [F-CAPS-076](#f-caps-076) | Per-object lock order | internal | internal | internal |
| [F-CAPS-077](#f-caps-077) | Cross-object ordering rules and no upgrades | internal | internal | internal |
| [F-CAPS-078](#f-caps-078) | Ancestor snaplock/policylock try-rdlocks and layout inheritance | supporting | internal | internal |
| [F-CAPS-079](#f-caps-079) | Path traversal locking and path helpers | internal | internal | internal |
| [F-CAPS-080](#f-caps-080) | Implicit versionlock (pipelining and peer rollback) | internal | internal | internal, mds-ranks |
| [F-CAPS-081](#f-caps-081) | Implicit quiescelock wrlock | extension | internal | kclient, libcephfs, ceph-fuse, ganesha, mgr, admin |
| [F-CAPS-082](#f-caps-082) | Local auth pins on locked objects | internal | internal | internal |
| [F-CAPS-083](#f-caps-083) | Remote auth pins and the rename frozen auth pin | internal | mds-wire | mds-ranks |
| [F-CAPS-084](#f-caps-084) | Remote xlocks and remote wrlocks | internal | mds-wire | mds-ranks |
| [F-CAPS-085](#f-caps-085) | Remote lock teardown and peer failure | internal | mds-wire | mds-ranks |
| [F-CAPS-087](#f-caps-087) | FIFO lock waiters with synchronous wake-up | internal | internal | internal |
| [F-CAPS-088](#f-caps-088) | Non-blocking lock helpers | internal | internal | internal |
| [F-CAPS-089](#f-caps-089) | Freeze versus auth-pin deadlock breaker | internal | config | mds-ranks |
| [F-CAPS-090](#f-caps-090) | Lock waits on a degraded cluster | internal | internal | internal, mds-ranks |
| [F-CAPS-091](#f-caps-091) | xlock lifecycle | supporting | internal | internal |
| [F-CAPS-092](#f-caps-092) | XLOCKDONE: read-your-own-unsafe-writes | supporting | client-wire, config | kclient, libcephfs, ceph-fuse, ganesha |
| [F-CAPS-093](#f-caps-093) | Early-reply rdlock release and journal flush policy | internal | internal | internal |
| [F-CAPS-094](#f-caps-094) | Lock release at safe reply and cleanup | internal | internal | internal |

**Observability**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-CAPS-095](#f-caps-095) | Cap and lease counters and dumps | operational | metrics, admin | mgr, admin, tools |
| [F-CAPS-096](#f-caps-096) | Lock state dumps and lock-related op events | operational | admin | admin, tools |

**Legacy entries**

| ID | Feature | Legacy reasons | Relied on by |
|---|---|---|---|
| [F-CAPS-007](#f-caps-007) | Cap restrictions for old clients (connection feature bits) | old-clients | kclient, libcephfs, ceph-fuse |

## 3. Features

### Capability model and allowed caps

The cap bit layout, the per-client Capability record, and how lock states, the loner choice and client feature bits decide which cap bits each client may hold.

<a id="f-caps-001"></a>

#### F-CAPS-001 · Cap bit model and composite masks

*Fixed 32-bit cap mask layout shared by the MDS and every client, mapping generic cap letters onto the four cap-bearing inode locks.*

Bit 0 is CEPH_CAP_PIN (p). Generic bits s=1, x=2, c=4, r=8, w=16, b=32, a=64 (WREXTEND), l=128 (LAZYIO) are shifted per lock: authlock by 2 (As 0x4, Ax 0x8: mode, uid, gid, btime, fscrypt_auth), linklock by 4 (Ls 0x10, Lx 0x20: nlink), xattrlock by 6 (Xs 0x40, Xx 0x80: xattr blob and version), filelock by 8 with 8 bits (Fs 0x100 ... Fl 0x8000: size, times, truncate seq/size, layout, inline data, dirstat and file data access). Composite masks (ANY_SHARED, ANY_RD, ANY_EXCL, ANY_FILE_RD, ANY_FILE_WR, ANY_WR, CEPH_CAP_ANY) and getattr masks CEPH_STAT_CAP_* (mode/uid/gid As, nlink Ls, size/times/layout Fs, xattrs Xs, inline data Fs|Fr, rstat = the Fa bit) are defined in ceph_fs.h. On directories Fc and Fr are reinterpreted as DIR_CREATE (Dc) and DIR_UNLINK (Dr), and CEPH_CAP_ANY_DIR_OPS = Fc|Fr|Fa|Fl (0xCC00). Cap strings such as pAsxLsXsxFsxcrwb are rendered by ccap_string in logs and dumps.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.1 The cap bit model](../cpp-map/02-client-protocol/capabilities.md#31-the-cap-bit-model) · [lock-types-and-state-machines.md › 3.2 Lock types per object](../cpp-map/05-locking/lock-types-and-state-machines.md#32-lock-types-per-object)
- **Code:** `src/include/ceph_fs.h:824-861 (CEPH_CAP_PIN)`, `src/include/ceph_fs.h:885-912 (CEPH_CAP_ANY_SHARED)`, `src/mds/SimpleLock.cc:127-142 (SimpleLock::get_cap_shift)`, `src/mds/mdstypes.h:103 (ccap_string)`
- **Notes:** Fa (GWREXTEND) is in no per-state cap column of any lock table, so it is never granted (it appears only in sm_filelock.allowed_ever_auth, locks.c:134). nestlock, dirfragtreelock, snaplock, policylock, flocklock and the local locks carry no cap bits (shift 0). CEPH_CAP_LOCKS = IFILE|IAUTH|ILINK|IXATTR is the mask passed to Locker::eval whenever caps may change.

<a id="f-caps-002"></a>

#### F-CAPS-002 · Open mode to wanted caps

*Translation of open(2) flags into a file mode and the cap bits a client wants for it, computed the same way by the MDS and the clients.*

ceph_flags_to_mode maps O_* flags to CEPH_FILE_MODE_PIN (0), RD (1), WR (2), RDWR (3), plus LAZY (4); O_DIRECTORY yields PIN. ceph_caps_for_mode gives the wanted caps: always p; RD adds Fs Fr Fc; WR adds Fx Fw Fb As Ax Xs Xx; LAZY adds Fl. The MDS uses it in issue_new_caps to set (new cap) or extend (existing cap) the cap's wanted mask on open and create. Lx is never wanted by any mode, so linklock normally stays SYNC.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.1 The cap bit model](../cpp-map/02-client-protocol/capabilities.md#31-the-cap-bit-model) · [capabilities.md › 3.8 Wanted caps](../cpp-map/02-client-protocol/capabilities.md#38-wanted-caps)
- **Code:** `src/common/ceph_fs.cc:116 (ceph_flags_to_mode)`, `src/common/ceph_fs.cc:143 (ceph_caps_for_mode)`, `src/mds/Locker.cc:2436 (Locker::issue_new_caps)`
- **Notes:** libcephfs Inode::caps_wanted applies ceph_caps_for_mode to directories too, so a directory held open with open() rather than opendir() lists Fr|Fc (the same bits as Dr|Dc) in its wanted mask (by code reading). The kernel computes wanted from open file modes with idle timeouts caps_wanted_delay_min 5 s and caps_wanted_delay_max 60 s. See also F-NS-030 (OPEN handler).

<a id="f-caps-003"></a>

#### F-CAPS-003 · Capability record, cap ids and sequence numbers

*Per-(inode, client) cap state with pending, issued and wanted bits, revoke history and the seq / issue_seq / mseq counters that keep the asynchronous cap protocol consistent.*

Each head inode keeps one Capability per client holding a cap (snapshot inodes have none). Fields: cap_id (from a per-rank counter last_cap_id that is not persisted; always 1 for a cap created by the request that created the inode), _pending (bits last sent), _issued (pending plus bits whose revocation is not yet acknowledged), _revokes (history {before, seq, last_issue} per revocation), _wanted, last_sent (bumped by every reply cap, GRANT, REVOKE, max_size GRANT and IMPORT), last_issue (set only when the cap travels in a reply or IMPORT), mseq (bumped when the auth cap migrates), cap_gen (session generation, for staleness). confirm_receipt(seq, caps) clears the history when seq == last_sent (the client may voluntarily drop bits) and otherwise only trims history older than seq, never shrinking pending on a stale ack. Messages naming another cap_id or an older mseq refer to a previous incarnation and are ignored. Capability objects are never journaled or persisted.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.2 The per-(inode, client) Capability object](../cpp-map/02-client-protocol/capabilities.md#32-the-per-inode-client-capability-object) · [capabilities.md › 4. Data structures and invariants](../cpp-map/02-client-protocol/capabilities.md#4-data-structures-and-invariants)
- **Code:** `src/mds/Capability.h:74 (Capability)`, `src/mds/Capability.h:150 (Capability::issue)`, `src/mds/Capability.cc:192 (Capability::confirm_receipt)`, `src/mds/CInode.cc:3613 (CInode::add_client_cap)`
- **Notes:** last_cap_id restarts at 0 on a new MDS incarnation and is raised to the largest cap_id reported in client reconnects (src/mds/Server.cc:1728). Seq comparisons in confirm_receipt and _do_cap_release are plain integer comparisons, not wrap-aware ceph_seq_cmp. The kernel stores the last seq per cap, treats a GRANT/REVOKE with seq <= cap->seq as pre-IMPORT, and updates issue_seq only from replies and IMPORT (linux/fs/ceph/caps.c:777, :3622), so seqs must increase per cap per MDS in delivery order. Capability::encode is used only by ceph-dencoder.

<a id="f-caps-004"></a>

#### F-CAPS-004 · Lock-state-derived allowed caps

*Per-client allowed cap set computed from the current states of authlock, linklock, xattrlock and filelock; the mechanism that keeps client caches coherent.*

Every lock state row in locks.c has four cap columns: caps (any client), loner_caps (extra for the loner), xlocker_caps (extra for the client whose request xlocked the lock) and replica_caps (what a non-auth rank may issue). SimpleLock::gcaps_allowed gives non-loners only caps in loner-mode states and loner_caps|caps otherwise; replicas always get replica_caps. Locker::get_allowed_caps composes p | sum(lock bits << shift), uses the loner view for the loner, ORs xlocker caps masked to s|x|c|r (filelock) or s|x (others) for the xlocking client, masks CEPH_CAP_ANY_DIR_OPS out of directory caps (adding back lock_cache_allowed only if Fx is allowed), and withholds Fr|Fw from old clients (F-CAPS-007); the quarantine and quiesce masks are applied too. Examples: filelock SYNC gives Fscrl (replicas Fscr), MIX gives Frwl (replicas Fr), EXCL gives the loner Fsxcrwb, LOCK leaves Fcb; simple locks give s in SYNC and the loner sx in EXCL.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): The old-client step of the allowed-caps computation: Locker::get_allowed_caps, and CInode::get_caps_allowed_for_client at reply time, remove Fr|Fw when the inode has inline data and the cap is NOINLINE, or the layout has a pool namespace and the cap is NOPOOLNS (the F-CAPS-007 restriction). Only caps of sessions whose connection lacks CEPH_FEATURE_MDS_INLINE_DATA (bit 40) or CEPH_FEATURE_FS_FILE_LAYOUT_V2 (bit 58) are marked NOINLINE or NOPOOLNS and lose Fr|Fw. The 7.3 kernel advertises both bits, bit 58 under its alias CRUSH_TUNABLES5. libcephfs advertises CEPH_FEATURES_ALL, which contains both. Upstream kernels advertise bit 58 from v4.5. Every other step of the allowed-caps computation (lock states, loner and xlocker views, dir-op mask, quarantine and quiesce masks) is current. Evidence: `src/mds/Locker.cc:2569-2573 (Locker::get_allowed_caps)`, `src/mds/CInode.cc:3823-3839 (CInode::get_caps_allowed_for_client)`, `src/mds/Capability.cc:173-181 (Capability::Capability)`, `linux/include/linux/ceph/ceph_features.h:163-165 (CRUSH_TUNABLES5 = FS_FILE_LAYOUT_V2)`, `linux/include/linux/ceph/ceph_features.h:207 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `linux/include/linux/ceph/ceph_features.h:217 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:223 (CEPH_FEATURES_ALL)`, `src/include/ceph_features.h:236 (CEPH_FEATURES_ALL)`, `src/include/ceph_features.h:263 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `linux-v4.5/include/linux/ceph/ceph_features.h:125 (CEPH_FEATURES_SUPPORTED_DEFAULT)`.
- **Map:** [capabilities.md › 3.3 From lock states to allowed caps](../cpp-map/02-client-protocol/capabilities.md#33-from-lock-states-to-allowed-caps) · [lock-types-and-state-machines.md › 3.5 Permission and allowed-cap predicates](../cpp-map/05-locking/lock-types-and-state-machines.md#35-permission-and-allowed-cap-predicates) · [lock-types-and-state-machines.md › 3.13 How lock state drives cap issuance and visibility](../cpp-map/05-locking/lock-types-and-state-machines.md#313-how-lock-state-drives-cap-issuance-and-visibility)
- **Code:** `src/mds/locks.c:15-146 (simplelock)`, `src/mds/SimpleLock.h:523 (SimpleLock::gcaps_allowed)`, `src/mds/CInode.cc:3761 (CInode::get_caps_allowed_by_type)`, `src/mds/Locker.cc:2528 (Locker::get_allowed_caps)`, `src/mds/CInode.cc:3798 (CInode::get_caps_allowed_for_client)`
- **Notes:** CInode::get_caps_allowed_for_client (reply-time caps) adds xlocker extras only for the loner, while Locker::get_allowed_caps (issue_caps) adds them for any xlocking client. is_loner_mode() reads the current state's flag even when evaluating the next state's columns (src/mds/SimpleLock.h:517). Clients rely on: at most one client holding any x bit of a lock; every s bit revoked before an xlock is granted; MIX giving Fr|Fw without Fc/Fb (uncached shared I/O). The quarantine mask (p only) and quiesce mask (ANY_RD|Fc|Fb|p) are owned by 10-qos-metrics-quiesce-quarantine.

<a id="f-caps-005"></a>

#### F-CAPS-005 · Loner selection (exclusive client)

*Choice of at most one client per inode that may receive exclusive (x) and buffered caps.*

CInode::calc_ideal_loner returns no loner when the file system is read-only or any replica rank reports wanted caps (mds_caps_wanted non-empty); otherwise the unique non-stale cap holder that, for a file, wants ANY_WR|Fr, or, for a directory, any non-stale holder while the directory has no subtree-root or exporting dirfrag; two candidates means no loner. choose_ideal_loner sets want_loner_cap; switching loners requires try_drop_loner, which succeeds only once the old loner's issued caps fit what non-loners may hold. set_loner_cap copies the loner into excl_client of the four cap locks, which lets the loner's requests wrlock in EXCL (XCL permission). Locker::eval re-runs the selection before and after evaluating the cap locks and retries with all locks when the loner changes.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [capabilities.md › 3.3 From lock states to allowed caps](../cpp-map/02-client-protocol/capabilities.md#33-from-lock-states-to-allowed-caps) · [lock-types-and-state-machines.md › 3.5 Permission and allowed-cap predicates](../cpp-map/05-locking/lock-types-and-state-machines.md#35-permission-and-allowed-cap-predicates) · [leases-lock-caches-and-async-dirops.md › 3.2.2 How Dc/Dr are granted](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#322-how-dcdr-are-granted)
- **Code:** `src/mds/CInode.cc:3415 (CInode::calc_ideal_loner)`, `src/mds/CInode.cc:3438 (CInode::choose_ideal_loner)`, `src/mds/CInode.cc:3476 (CInode::try_drop_loner)`, `src/mds/CInode.cc:3467 (CInode::set_loner_cap)`, `src/mds/Locker.cc:1503 (Locker::eval)`
- **Notes:** Its effect is visible only through which caps are granted (F-CAPS-004): without a loner, writers get MIX (synchronous I/O) instead of EXCL (buffered). A directory with a subtree-root dirfrag (num_subtree_roots > 0, e.g. the file system root or an export-pinned directory) or an exporting dirfrag gets no loner and file_eval never moves its filelock to EXCL, so clients never get Fx (and so never Dc/Dr) there (src/mds/CInode.cc:916, by code reading). On replicas get_caps_issued resets loner_cap to -1 as a side effect (src/mds/CInode.cc:3850).

<a id="f-caps-006"></a>

#### F-CAPS-006 · Wanted-cap-driven stable-state policy

*Policy that chooses the stable state of each lock (EXCL, MIX, SYNC, LOCK) from what clients want and what requests hold, deciding whether clients do cached, buffered or synchronous I/O.*

On the auth, try_eval/eval route stable locks to file_eval (filelock), scatter_eval (nestlock, dirfragtreelock) or simple_eval (other locks). file_eval leaves EXCL when the loner neither wants nor holds Fx|Fw|Fb, another client wants Fx|Fw|Fr, or a directory has more than one non-stale cap (to MIX if someone wants Fw or waits for a wrlock, else to SYNC once wrlocks drain); moves to EXCL when not rdlocked and a target loner exists (file: wanting Fx|Fw|Fb; directory: no subtree-root or exporting dirfrag); to MIX when scatter is wanted or nobody is loner and someone wants Fw; to SYNC when not wrlocked, without wrlock waiters and nobody wants Fw. simple_eval moves a cap lock to EXCL when the target loner wants its x bit and otherwise back to SYNC; non-cap simple locks always drift back to SYNC. Result for a regular file: one writer EXCL (Fsxcrwb), several writers or a writer plus readers MIX (Frw), readers only SYNC (Fscr). Evaluation is deferred while the object is frozen or (except DN/ISNAP/IPOLICY and scatter_wanted) freezing; on a read-only file system every policy forces SYNC and no loner is chosen.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [capabilities.md › 3.3 From lock states to allowed caps](../cpp-map/02-client-protocol/capabilities.md#33-from-lock-states-to-allowed-caps) · [lock-types-and-state-machines.md › 3.7 Stable-state policy: try_eval, eval, simple_eval, sca...](../cpp-map/05-locking/lock-types-and-state-machines.md#37-stable-state-policy-try_eval-eval-simple_eval-scatter_eval-file_eval)
- **Code:** `src/mds/Locker.cc:5697 (Locker::file_eval)`, `src/mds/Locker.cc:4867 (Locker::simple_eval)`, `src/mds/Locker.cc:5323 (Locker::scatter_eval)`, `src/mds/Locker.cc:1605 (Locker::try_eval)`
- **Notes:** Visible to clients only as the caps they are granted. simple_eval uses the loner's wanted bits, not the union of all clients (src/mds/Locker.cc:4893-4897). other_wanted includes replicas' mds_caps_wanted.

<a id="f-caps-008"></a>

#### F-CAPS-008 · Lazy I/O cap (Fl)

*Optional relaxed-coherency file cap for files a client switches to lazy I/O (kernel CEPH_IOC_LAZYIO ioctl; libcephfs ceph_lazyio() or client_force_lazyio, which adds CEPH_O_LAZY at open). Linux has no O_LAZY flag.*

Fl is allowed in filelock SYNC, MIX and the MIX/SYNC transitional states, never in LOCK, EXCL or XSYN. It is not in the "liked" set, so it is issued only to clients whose wanted mask includes it (CEPH_FILE_MODE_LAZY). In MIX the kclient reads through its page cache if it holds Fc or Fl, otherwise synchronously from the OSDs.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs
- **Map:** [capabilities.md › 3.3 From lock states to allowed caps](../cpp-map/02-client-protocol/capabilities.md#33-from-lock-states-to-allowed-caps) · [lock-types-and-state-machines.md › 3.4.3 sm_filelock (src/mds/locks.c:84-146): IFILE](../cpp-map/05-locking/lock-types-and-state-machines.md#343-sm_filelock-srcmdslocksc84-146-ifile)
- **Code:** `src/common/ceph_fs.cc:143-159 (ceph_caps_for_mode)`, `src/mds/locks.c:84-146 (filelock)`, `src/mds/CInode.cc:3737 (CInode::get_caps_liked)`, `linux/fs/ceph/file.c:2196-2207 (ceph_read_iter)`
- **Notes:** Relaxes POSIX read-after-write coherency between clients by design.

### Cap issue, revocation and client cap messages

How caps reach clients (replies, open/create, GRANT/REVOKE, TRUNC), the MClientCaps wire format and how the MDS processes client cap messages.

<a id="f-caps-009"></a>

#### F-CAPS-009 · Caps in request replies

*Every inode in a reply trace or readdir entry carries a cap record that creates or extends the requesting client's cap.*

CInode::encode_inodestat creates a cap if none exists unless no_caps holds (snapshot inode not auth, session stale, readdir crossing into another snaprealm, inode frozen, caps being exported). For head inodes it issues (wanted|liked) & allowed with issue_norevoke (a reply never revokes), sets last_issue = last_sent and encodes ceph_mds_reply_cap {caps, wanted (u32), cap_id (u64), seq, mseq (u32), realm (u64), flags (u8)} with CEPH_CAP_FLAG_AUTH (1) iff this rank is auth. "Liked" caps are CEPH_CAP_ANY & ~Fl for files and p|ANY_EXCL|ANY_SHARED for directories, so the MDS pre-grants everything allowed except lazy I/O and directory file-data bits. max_size in the reply is min(committed, projected) client range; inline data and the xattr blob are included for a new cap, an older client version, or when the getattr mask asks for them. Snapshot inodes get bits from the locks' CAP_ANY view without a Capability object (seq, mseq and realm 0; the kernel only ORs them into i_snap_caps).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (deprecated-admin): Inline file data in the reply: CInode::encode_inodestat sends the inline content for a new cap, a cap whose client_inline_version is older, or a getattr that asks for Fr. This has content only for inodes that carry inline data; for every other inode the reply carries inline_version CEPH_INLINE_NONE and no data. An inode carries inline data only if the file system's inline_data flag was enabled when it was created and the creating client advertised bit 40; otherwise its inline_version is CEPH_INLINE_NONE. The flag defaults to off, and enabling it needs ceph fs set <fs> inline_data true --yes-i-really-really-mean-it, prints a deprecation message and raises the FS_INLINE_DATA_DEPRECATED health warning. No mgr module, cephadm, ceph.git tool or Rook v1.21.0 sets the flag. The 7.3 kernel and libcephfs still handle inline data when an MDS sends it. Evidence: `src/mds/CInode.cc:4079-4090 (CInode::encode_inodestat)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mds/MDSMap.h:647 (MDSMap::inline_data_enabled)`, `src/mon/FSCommands.cc:471-479 (inline_data)`, `src/mds/MDSMap.cc:616-622 (MDSMap::get_health_checks)`, `src/mds/MDSRank.cc:2615-2616 (MDSRankDispatcher::handle_mds_map)`, `linux/fs/ceph/caps.c:3835-3837 (handle_cap_grant)`. Also: search: grep -rn inline_data over src/pybind/mgr, src/cephadm, src/python-common, src/tools and rook v1.21.0 finds no CephFS inline_data setting (Rook matches are RGW zone placement fields, src/tools only resets inodes to CEPH_INLINE_NONE).
- **Map:** [capabilities.md › 3.4 Issuing caps](../cpp-map/02-client-protocol/capabilities.md#34-issuing-caps) · [capabilities.md › 5.3 Other wire structs](../cpp-map/02-client-protocol/capabilities.md#53-other-wire-structs)
- **Code:** `src/mds/CInode.cc:3959 (CInode::encode_inodestat)`, `src/mds/CInode.cc:4208-4253 (CInode::encode_inodestat)`, `src/mds/CInode.cc:3737 (CInode::get_caps_liked)`, `src/mds/Server.cc:2491 (Server::set_trace_dist)`
- **Notes:** A STATE_NEW cap whose session went stale still gets p so the open/create reply returns it (src/mds/CInode.cc:4224-4233). encode_inodestat checks the remaining readdir buffer before creating a cap, so entries that do not fit get none. cap_id and wanted of snapshot caps are left unset.

<a id="f-caps-010"></a>

#### F-CAPS-010 · New caps on open, create, mknod and mkdir

*Cap creation and lock presetting on opens and on newly created inodes, so the creating client immediately owns the new inode.*

Locker::issue_new_caps (called from open, openc, O_TRUNC open, mknod of regular files and mkdir) reuses a reconnected cap for a replayed request that allocated the inode, else creates a cap (cap_id 1 for a new inode) with wanted = ceph_caps_for_mode(mode) and STATE_NEW, or ORs the mode's caps into an existing cap's wanted; then, with cap messages suppressed, it re-evaluates the cap locks (auth) or sends MInodeFileCaps (replica), flushing the journal if a wanted bit is blocked by a locked lock. The bits travel in the reply. openc presets authlock and xattrlock to EXCL and, when opened for write, client_ranges[client] = [0, layout.stripe_unit]; mknod of a regular file and mkdir issue with mode RDWR, set filelock, authlock and xattrlock to EXCL and reset wanted to 0; mknod also sets the stripe_unit range.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.4 Issuing caps](../cpp-map/02-client-protocol/capabilities.md#34-issuing-caps) · [lock-types-and-state-machines.md › 3.8 Transition primitives](../cpp-map/05-locking/lock-types-and-state-machines.md#38-transition-primitives)
- **Code:** `src/mds/Locker.cc:2436 (Locker::issue_new_caps)`, `src/mds/Server.cc:5015-5021 (Server::handle_client_openc)`, `src/mds/Server.cc:7646-7663 (Server::handle_client_mknod)`, `src/mds/Server.cc:7766-7776 (Server::handle_client_mkdir)`
- **Notes:** The kclient async create fabricates cap_id 1, seq 0, caps CEPH_CAP_ALL_FILE and max_size = stripe_unit for its provisional inode (linux/fs/ceph/file.c:712); the MDS's cap_id 1, preset EXCL locks and [0, stripe_unit] range match it. The initial range is written into the not-yet-journaled inode, so the reply carries it before the create is durable. Opening a file in a snapshot sets mdr->snap_caps, which nothing reads (src/mds/Server.cc:4796).

<a id="f-caps-011"></a>

#### F-CAPS-011 · Cap grant and revoke messages (GRANT/REVOKE)

*The MDS pushes cap changes with CEPH_CAP_OP_GRANT when it only adds bits and CEPH_CAP_OP_REVOKE whenever it removes bits.*

Locker::issue_caps computes allowed, pending and wanted per cap. Without revocation it skips new, suppressed or stale caps. When a revocation is needed on a stale session it silently reduces pending and queues C_Locker_RevokeStaleCap (F-CAPS-030). If pending exceeds allowed, or wanted&allowed bits are missing, or the cap is invalid after a stale/resume cycle, the new pending is (wanted|liked)&allowed&pending when revoking (a revoking message never adds bits) or (wanted|liked)&allowed otherwise; op = REVOKE if any bit is removed, else GRANT. A REVOKE puts the cap at the tail of the rank-wide and per-client revoking lists with a timestamp. The message carries seq = last_sent, issue_seq = last_issue, mseq, caps, wanted, realm, osd_epoch_barrier and inode fields from encode_cap_message (projected values only for the client that xlocked the lock or holds Fx). If nlink == 0, Ls is added to wanted so clients learn of the unlink quickly.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.4 Issuing caps](../cpp-map/02-client-protocol/capabilities.md#34-issuing-caps) · [capabilities.md › 3.5 Revocation and gather](../cpp-map/02-client-protocol/capabilities.md#35-revocation-and-gather) · [capabilities.md › 7. Externally visible contract](../cpp-map/02-client-protocol/capabilities.md#7-externally-visible-contract)
- **Code:** `src/mds/Locker.cc:2578 (Locker::issue_caps)`, `src/mds/Locker.cc:2622-2707 (Locker::issue_caps)`, `src/mds/CInode.cc:4432 (CInode::encode_cap_message)`, `linux/fs/ceph/caps.c:3809 (handle_cap_grant)`
- **Notes:** The kernel answers every REVOKE with a cap message, even when it holds none of the revoked bits, after writeback for Fb or page-cache invalidation for Fc (linux/fs/ceph/caps.c:3596-3608), so lock gathers wait for client I/O. A GRANT may carry only a max_size or field update with unchanged caps. issue_caps may send two messages in one call: a GRANT re-issuing pending to an invalid cap, then the REVOKE. Callers include every lock transition primitive, Locker::eval, file_update_finish, resume_stale_caps and MDCache::reissue_all_caps at active_start.

<a id="f-caps-013"></a>

#### F-CAPS-013 · Client cap message processing (handle_client_caps)

*Handling of client UPDATE/FLUSH/DROP/RELEASE/RENEW cap messages: acks of revocations, wanted changes and dirty metadata.*

Messages that arrive before clientreplay are parked (during reconnect the dirty bits of re-sent flushes are recorded for lock-state choice). A non-zero flush tid already completed is re-acked at once (F-CAPS-019). The handler drops messages for inodes not in cache (in clientreplay it waits for a replayed create), from clients without a cap, with an older mseq, or (except FLUSHSNAP) with another cap_id; it defers on WAIT_UNFREEZE when the inode is frozen or freezing without auth pins. Op values other than FLUSHSNAP are not distinguished. The client's caps (the kernel sends its implemented set) are masked with issued, then confirm_receipt(seq, caps); revoked dir-op bits invalidate lock caches; null snapflushes are inferred; adjust_cap_wanted; on the auth, dirty data is journaled (_do_cap_update) and acked after commit; then eval(CEPH_CAP_LOCKS) and issue_caps if wanted bits are not pending.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.6 Client cap messages: handle_client_caps](../cpp-map/02-client-protocol/capabilities.md#36-client-cap-messages-handle_client_caps)
- **Code:** `src/mds/Locker.cc:3386 (Locker::handle_client_caps)`, `src/mds/Locker.cc:3363 (Locker::should_defer_client_cap_frozen)`, `src/mds/Locker.cc:3610-3613 (Locker::handle_client_caps)`
- **Notes:** For a reconnected cap that never had a message sent (last_seq == 0) holding Fw|Fb, max_size is pushed immediately (src/mds/Locker.cc:3685). Flushes sent to a non-auth rank, for an uncached inode or with a mismatching cap_id are dropped without ack; clients re-send flushes after IMPORT or reconnect. Cap traffic bypasses the dmClock QoS scheduler, which only queues MClientRequests.

<a id="f-caps-014"></a>

#### F-CAPS-014 · Wanted caps tracking (issue_seq rule)

*Advisory per-cap wanted mask that drives lock policy, with a rule that a stale message cannot shrink it.*

adjust_cap_wanted(cap, wanted, issue_seq) replaces the wanted mask only if issue_seq == last_issue; otherwise it only adds bits, so a stale update cannot undo a wanted increase made by a later reply (for example from open). After a change, a replica re-sends MInodeFileCaps; the auth prioritizes file recovery if the inode is recovering and Fr|Fw is wanted, and logs an EOpen when the OpenFileTable asks for it. Stale sessions' wanted bits are ignored by loner selection and lock policy.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.8 Wanted caps](../cpp-map/02-client-protocol/capabilities.md#38-wanted-caps)
- **Code:** `src/mds/Locker.cc:3247 (Locker::adjust_cap_wanted)`, `src/mds/Capability.cc:265 (Capability::set_wanted)`
- **Notes:** If a cap it needs is not in the MDS's wanted view, the kernel re-registers it by sending an OPEN request (ceph_renew_caps, linux/fs/ceph/file.c:319), retried every 5 s. Clients with CEPHFS_FEATURE_LAZY_CAP_WANTED (bit 11) re-request wanted caps after a stale period (F-CAPS-030). OpenFileTable membership driven by notable caps is owned by 06-journal-and-storage.

<a id="f-caps-015"></a>

#### F-CAPS-015 · Xattr blob and inline data carried on caps

*Shipping the xattr blob and inline file data inside cap messages and replies, versioned per client to avoid resending.*

encode_cap_message includes the xattr blob only if the cap's pending has Xs and xattr_version is newer than the cap's client_xattr_version, and inline data when inline_version is newer than client_inline_version. Replies include them for new caps, older client versions, or when the getattr mask asks (Xs, Fr). On a flush with dirty Xx, decode_new_xattrs applies the blob only if it is non-empty and its version is newer; if its total size exceeds mdsmap max_xattr_size the blob is ignored but xattr_version still advances. Dirty Fw with a newer inline_version applies inline data.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (deprecated-admin): The inline-data half: encode_cap_message ships inline content when the inode's inline_version is newer than the cap's client_inline_version, and a dirty-Fw flush with a newer inline_version replaces the inode's inline data. Both act only on inodes that carry inline data; for other inodes a cap message carries inline_version CEPH_INLINE_NONE once per cap and 0 afterwards, and the flush comparison never succeeds. Inline data exists only on inodes created while the deprecated inline_data file system flag is enabled (default off, FS_INLINE_DATA_DEPRECATED health warning when on, set by no baseline peer). The 7.3 kernel uninlines a file when it opens it for writing, and libcephfs uninlines before writes that do not fit inline. The xattr blob half of the entry is current. Evidence: `src/mds/CInode.cc:4467-4473 (CInode::encode_cap_message)`, `src/mds/Locker.cc:4024-4031 (Locker::_update_cap_fields)`, `src/mds/Server.cc:3758-3760 (Server::prepare_new_inode)`, `src/mon/FSCommands.cc:471-479 (inline_data)`, `src/mds/MDSMap.cc:616-622 (MDSMap::get_health_checks)`, `linux/fs/ceph/file.c:261-262 (ceph_init_file_info)`, `src/client/Client.cc:12917-12922 (Client::_write)`.
- **Map:** [capabilities.md › 3.4 Issuing caps](../cpp-map/02-client-protocol/capabilities.md#34-issuing-caps) · [capabilities.md › 3.7 Dirty metadata flush and FLUSH_ACK](../cpp-map/02-client-protocol/capabilities.md#37-dirty-metadata-flush-and-flush_ack)
- **Code:** `src/mds/CInode.cc:4432-4501 (CInode::encode_cap_message)`, `src/mds/Locker.cc:3855 (Locker::decode_new_xattrs)`
- **Notes:** Because an oversize flushed blob still bumps xattr_version, later grants overwrite the client's cached xattrs. The kclient satisfies getxattr/listxattr locally with Xs (linux/fs/ceph/xattr.c:1017) and setxattr locally with Xx when the blob fits max_xattr_size.

<a id="f-caps-016"></a>

#### F-CAPS-016 · Truncate notification (CEPH_CAP_OP_TRUNC)

*Push of a shrinking truncation to every cap holder once it is journaled.*

Locker::issue_truncate sends CEPH_CAP_OP_TRUNC with the current pending caps, without incrementing seq, and with truncate_seq, truncate_size and size from encode_cap_message to every cap holder after a truncation to a smaller size commits (C_MDS_inode_update_finish), then calls check_inode_max_size. truncate_seq starts at 1 for new inodes. The kernel accepts a newer truncate_seq (and queues page-cache truncation) or a larger size with the same seq.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.10 Truncation notification](../cpp-map/02-client-protocol/capabilities.md#310-truncation-notification)
- **Code:** `src/mds/Locker.cc:2717 (Locker::issue_truncate)`, `src/mds/Server.cc:5360 (C_MDS_inode_update_finish)`, `linux/fs/ceph/inode.c:808 (ceph_fill_file_size)`
- **Notes:** The kernel warns "the MDS should have revoked these caps" if truncate_seq rises while it still holds Fr or Fl (linux/fs/ceph/inode.c:837-839); the C++ MDS satisfies this because SETATTR(size) and O_TRUNC open xlock filelock, whose xlock states allow only Fcb. See also F-NS-062, F-STORE-065.

### Dirty metadata flush, max_size and file size recovery

Clients with exclusive caps write metadata back through cap flushes; writers are bounded by journaled max_size ranges, and file size is recovered from the OSDs when a writer disappears.

<a id="f-caps-018"></a>

#### F-CAPS-018 · Dirty metadata flush and FLUSH_ACK

*Clients holding exclusive caps change metadata locally and flush it to the auth MDS, which journals it and acknowledges only when durable.*

The client sends dirty bits and a flush tid. _update_cap_fields applies: any dirty bit -> ctime (and rctime) if newer and change_attr if larger (FS_CHANGE_ATTR); Fw -> mtime if newer; Fx -> mtime if different (may move backwards), atime if different, time_warp_seq if newer; Fw|Fx on a regular file -> size only if larger (truncation is a SETATTR request) and fscrypt_file; Fw -> inline data if newer; Ax -> uid, gid, mode, btime (FS_BTIME), fscrypt_auth; Xx -> xattrs (F-CAPS-015). _do_cap_update projects the inode, wrlock_forces filelock (Fx|Fw dirty or a max_size change), authlock (Ax) and xattrlock (new blob) for the journal write, journals EUpdate("cap update") with the client flush tid and oldest_flush_tid, and file_update_finish sends FLUSH_ACK {seq = client's, caps, dirty = flushed bits, issue_seq = last_issue, tid, oldest_flush_tid} after the event is safe, records the tid as completed, drops the forced wrlocks and re-issues caps. Flushes are honoured only on the auth rank.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (deprecated-admin, old-clients): (1) Fw -> inline data if newer: applies only to inodes that carry inline data (deprecated-admin). (2) The connection-feature gates on change_attr (FS_CHANGE_ATTR) and btime (FS_BTIME): a flush from a connection without feature bit 59 has both fields ignored (old-clients). Inline data exists only on inodes created while the deprecated inline_data file system flag is enabled. Feature bit 59 is shared by FS_BTIME, FS_CHANGE_ATTR and MSG_ADDR2; the 7.3 kernel advertises it as MSG_ADDR2 and libcephfs in CEPH_FEATURES_ALL, so baseline flushes always have change_attr and btime applied. Upstream kernels advertise bit 59 from v5.3 according to the comment in src/include/ceph_features.h (v5.0 does not, v5.4 does). Evidence: `src/mds/Locker.cc:4024-4031 (Locker::_update_cap_fields)`, `src/mds/Locker.cc:3985 (Locker::_update_cap_fields)`, `src/mds/Locker.cc:3995-3996 (Locker::_update_cap_fields)`, `src/mds/Locker.cc:4067 (Locker::_update_cap_fields)`, `linux/include/linux/ceph/ceph_features.h:166-168 (FS_BTIME, FS_CHANGE_ATTR, MSG_ADDR2)`, `linux/include/linux/ceph/ceph_features.h:219 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:180-182 (FS_BTIME)`, `src/include/ceph_features.h:238-240 (CEPH_FEATURES_ALL)`, `linux-v5.0/include/linux/ceph/ceph_features.h:178 (CEPH_FEATURES_SUPPORTED_DEFAULT, no MSG_ADDR2)`, `linux-v5.4/include/linux/ceph/ceph_features.h:214 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/mon/FSCommands.cc:471-479 (inline_data)`.
- **Map:** [capabilities.md › 3.7 Dirty metadata flush and FLUSH_ACK](../cpp-map/02-client-protocol/capabilities.md#37-dirty-metadata-flush-and-flush_ack) · [lock-types-and-state-machines.md › 3.3 The state-table schema](../cpp-map/05-locking/lock-types-and-state-machines.md#33-the-state-table-schema)
- **Code:** `src/mds/Locker.cc:3978 (Locker::_update_cap_fields)`, `src/mds/Locker.cc:4083 (Locker::_do_cap_update)`, `src/mds/Locker.cc:2380 (Locker::file_update_finish)`, `src/mds/Locker.cc:1929 (Locker::wrlock_force)`
- **Notes:** No MDSAuthCaps check is made on flushed values and caller_uid/gid are ignored; the dirty mask is not checked against the bits the cap holds, so any client holding a cap with a matching cap_id can push uid/gid/mode or xattrs by marking Ax/Xx dirty (inferred from code reading, not tested). atime is applied only with dirty Fx (src/mds/Locker.cc:4033), so a kernel atime-forward change flushed as dirty Fw is not applied (code reading of both sides, not tested). Forced wrlocks keep the locks out of wrlock-free states (e.g. SYNC) so other clients cannot read pre-commit values. The kernel treats FLUSH_ACK as cumulative per inode (all non-snap flushes with tid <= T become clean, linux/fs/ceph/caps.c:3927), so acks for one inode must follow journal order. can_force_wrlock is consulted only for a client's max_size change (F-CAPS-021).

<a id="f-caps-019"></a>

#### F-CAPS-019 · Flush idempotency (completed_flushes, oldest_flush_tid)

*Exactly-once application of client metadata flushes across retransmission and MDS failover.*

Acked flush tids are kept per session in session_info_t::completed_flushes (persisted in the sessionmap object) and journaled with the cap update, so replay re-adds them. A re-sent flush whose tid is present is acked immediately without reapplying it (this ack has issue_seq 0 and no oldest_flush_tid; a FLUSHSNAP stops there, other ops continue as an update with dirty 0). Each message's oldest_flush_tid trims tids below it. If a client never advances it and the set reaches mds_max_completed_flushes << num_trim_flushes_warnings, a clog warning "client.X does not advance its oldest_flush_tid" is emitted with exponential backoff and MDS_CLIENT_OLDEST_TID(_MANY) health is raised.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk`, `metrics` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon
- **Gates:** mds_max_completed_flushes (default 100000)
- **Map:** [capabilities.md › 3.7 Dirty metadata flush and FLUSH_ACK](../cpp-map/02-client-protocol/capabilities.md#37-dirty-metadata-flush-and-flush_ack) · [sessions.md › 3.13 completed_requests and completed_flushes](../cpp-map/02-client-protocol/sessions.md#313-completed_requests-and-completed_flushes)
- **Code:** `src/mds/Locker.cc:3426-3473 (Locker::handle_client_caps)`, `src/mds/journal.cc:1798 (EMetaBlob::replay)`, `src/mds/Beacon.cc:409 (Beacon::notify_health)`, `src/mds/mdstypes.h:382 (session_info_t)`
- **Notes:** The kernel re-sends pending flushes with their original tids after IMPORT or reconnect (ceph_kick_flushing_inode_caps, linux/fs/ceph/caps.c:3855). Sessionmap persistence is owned by 06-journal-and-storage. See also F-STORE-030 (journal side).

<a id="f-caps-020"></a>

#### F-CAPS-020 · Synchronous flush and journal-flush heuristics

*Rules for when a cap flush forces the MDS journal to be flushed, which determine fsync latency.*

CEPH_CLIENT_CAPS_SYNC (flag 1, set by fsync) forces an MDLog flush. Otherwise _need_flush_mdlog flushes only if newly wanted bits are blocked by an unstable locked lock, the dirty bits' locks are unstable and locked (someone is waiting), or max_size grew. Without that, the FLUSH_ACK waits for the periodic flush in MDSRankDispatcher::tick every mds_tick_interval (5 s), another explicit flush, or the Journaler flushing a completed journal object when the write position crosses an object boundary.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ganesha, ceph-fuse
- **Gates:** mds_tick_interval (5 s)
- **Map:** [capabilities.md › 3.6 Client cap messages: handle_client_caps](../cpp-map/02-client-protocol/capabilities.md#36-client-cap-messages-handle_client_caps)
- **Code:** `src/mds/Locker.cc:3231 (Locker::_need_flush_mdlog)`, `src/mds/MDSRank.cc:726 (MDSRankDispatcher::tick)`, `src/osdc/Journaler.cc:632-641 (Journaler::append_entry)`
- **Notes:** The kernel sets CEPH_CLIENT_CAPS_SYNC on fsync and waits for the ack (linux/fs/ceph/caps.c:2359); libcephfs sends FLAG_SYNC too (src/client/Client.cc:4424).

<a id="f-caps-021"></a>

#### F-CAPS-021 · max_size and per-client writeable ranges (client_ranges)

*Journaled per-client byte range that a client may write to without asking the MDS, exposed to it as max_size.*

inode_t::client_ranges maps client -> {range{first, last}, follows} and is journaled with the inode; clients are told min(committed, projected), so they never write beyond a durable limit (exception: the initial [0, stripe_unit] range of an inode whose create is not yet journaled). calc_new_max_size = round_up((size+1)*2, capped at size + mds_client_writeable_range_max_inc_objs objects, to the layout period). On a client update for a cap with Fw|Fb|Fx issued or wanted, an explicit m->max_size grows the range, otherwise it grows to fit the reported size; without write caps the range is removed. If filelock cannot be wrlocked (can_force_wrlock lets the loner or xlocker proceed in EXCL_SYNC/EXCL_LOCK), the MDS moves it towards EXCL or LOCK and waits (C_MDL_CheckMaxSize). check_inode_max_size recomputes ranges for writers (only growing them), journals EOpen or EUpdate with wrlock_force(filelock) and flushes the log if a range grew; share_inode_max_size sends a GRANT (seq+1, unchanged pending) to caps with Fw|Fb pending.

- **Class:** POSIX `supporting` · exposure `client-wire`, `on-disk`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_client_writeable_range_max_inc_objs (default 1024)
- **Map:** [capabilities.md › 3.9 max_size and client_ranges](../cpp-map/02-client-protocol/capabilities.md#39-max_size-and-client_ranges)
- **Code:** `src/mds/Locker.cc:2980 (Locker::calc_new_max_size)`, `src/mds/Locker.cc:3077 (Locker::check_inode_max_size)`, `src/mds/Locker.cc:3191 (Locker::share_inode_max_size)`, `src/mds/Locker.cc:4102-4158 (Locker::_do_cap_update)`, `linux/fs/ceph/caps.c:2905 (try_get_cap_refs)`
- **Notes:** Kernel: a write needs Fw and endoff <= max_size, else it sends an UPDATE with max_size and sleeps until a GRANT raises it; it accepts max_size from a GRANT only on the auth cap and only if the new caps include a file-write bit (linux/fs/ceph/caps.c:3727); it also reports size proactively at max_size or half the last increment. share_inode_max_size skips suppressed caps (issue_new_caps relies on the reply). On snapshot COW, ranges with follows >= last are trimmed from the snapped copy. The range bounds file size recovery (F-CAPS-022).

<a id="f-caps-022"></a>

#### F-CAPS-022 · File size recovery after loss of a writer

*Recovery of a file's size and mtime by probing its data objects when a client that could write disappears.*

A client that loses its caps while holding a writeable range (stale revoke, session kill, cap removal with kill) marks the inode STATE_NEEDSRECOVER; the next filelock transition queues recovery. After an MDS failover, files with client_ranges whose clients did not reconnect are put in filelock PRE_SCAN by identify_files_to_recover; file_recover moves them to SCAN, revoking all file caps first. RecoveryQueue probes RADOS objects up to the range limit, then _recovered calls check_inode_max_size(force_wrlock) with the recovered size and mtime and Locker::eval; eval_gather waits while the inode is RECOVERING and then moves SCAN to LOCK. No rdlock, wrlock or xlock of the filelock is possible meanwhile; requests that need it move the inode to the front of the recovery queue.

- **Class:** POSIX `supporting` · exposure `cluster-wire`, `on-disk` · relied on by osd, kclient, libcephfs, internal, ceph-fuse
- **Gates:** mds_max_file_recover (32); mds_client_writeable_range_max_inc_objs (1024)
- **Map:** [capabilities.md › 3.9 max_size and client_ranges](../cpp-map/02-client-protocol/capabilities.md#39-max_size-and-client_ranges) · [lock-types-and-state-machines.md › 3.14 Special-purpose states](../cpp-map/05-locking/lock-types-and-state-machines.md#314-special-purpose-states)
- **Code:** `src/mds/Locker.cc:2757 (Locker::revoke_stale_cap)`, `src/mds/MDCache.cc:6444 (MDCache::identify_files_to_recover)`, `src/mds/Locker.cc:6006 (Locker::file_recover)`, `src/mds/RecoveryQueue.cc:232 (RecoveryQueue::_recovered)`, `src/mds/Locker.cc:1322-1333 (Locker::eval_gather)`, `src/mds/RecoveryQueue.cc:31-238 (RecoveryQueue::advance)`
- **Notes:** When an inode has no client_ranges/max_size no probe is needed (src/mds/RecoveryQueue.cc:122-123). RecoveryQueue runs at most mds_max_file_recover (32) probes at a time, prioritising files a request waits for; each probe stats one period at a time from max_size down to offset 0 (collecting the largest mtime); the result is applied as size = max(recorded, probed) and mtime = max, journaled as EOpen or EUpdate, so recovery never shrinks a file and a probed size 0 changes nothing; a probe OSD error other than -EBLOCKLISTED marks the rank damaged. The same queue serves caps lost through stale-session revoke or session kill outside recovery. Perf counters mds_cache.recovery_*. See also F-CLU-099 (after failover), F-STORE-066 (RADOS probe), F-CAPS-021 (client_ranges).

### Snapshot flush (snapflush)

Cap and lock machinery that makes clients deliver the metadata a file had at snapshot time; snapshot semantics themselves belong to 08-snapshots-quotas-subvolumes.

<a id="f-caps-023"></a>

#### F-CAPS-023 · Snapshot COW of cap state and SNAP_SYNC gating

*When a snapshot splits an inode that clients may still be writing, the snapped copy stays unreadable until those clients flush its metadata.*

MDCache::cow_inode records, for every cap that has (or, with STATE_NEEDSNAPFLUSH from reconnect, is assumed to have) ANY_WR and client_follows < last: oldin->client_snap_caps += client, cap->client_follows = last, and head->client_need_snapflush[snapid] += client for each realm snap in [oldin->first, last] (auth-pinning head and snapped inode). The snapped inode's four cap locks are set to LOCK_SNAP_SYNC with a forced wrlock and an auth pin; it is journaled with the need_snapflush dentry flag and added to need_snapflush_inodes. Readers of the snapshot wait in rdlock_start, which rdlocks the head inode's filelock as anonymous to force writers to flush. When client_snap_caps becomes empty, file_update_finish drops the wrlocks and eval_cap_gather moves the locks to SYNC. After failover the expectations are rebuilt from reconnect snap_follows (rebuild_need_snapflush).

- **Class:** POSIX `extension` · exposure `on-disk` · relied on by kclient, libcephfs, ceph-fuse
- **Map:** [capabilities.md › 3.11 Snapshots: snapflush](../cpp-map/02-client-protocol/capabilities.md#311-snapshots-snapflush) · [lock-types-and-state-machines.md › 3.14 Special-purpose states](../cpp-map/05-locking/lock-types-and-state-machines.md#314-special-purpose-states) · [snapshots.md › 3.4 Copy-on-write of metadata](../cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#34-copy-on-write-of-metadata)
- **Code:** `src/mds/MDCache.cc:1573 (MDCache::cow_inode)`, `src/mds/MDCache.cc:1638-1676 (MDCache::cow_inode)`, `src/mds/Locker.cc:1822-1837 (Locker::rdlock_start)`, `src/mds/Locker.cc:2413-2427 (Locker::file_update_finish)`, `src/mds/MDCache.cc:5651 (MDCache::rebuild_need_snapflush)`
- **Notes:** Snapshot semantics (realms, snap ids, metadata COW) are owned by 08-snapshots-quotas-subvolumes; this entry covers the cap and lock side only. Pending-snapflush auth pins block subtree freezing. See also F-SNAP-026.

<a id="f-caps-024"></a>

#### F-CAPS-024 · FLUSHSNAP / FLUSHSNAP_ACK protocol

*Client message that delivers the metadata a file had at snapshot time, and the MDS acknowledgement.*

The client sends CEPH_CAP_OP_FLUSHSNAP (cap_id 0, seq 0, mseq of its auth cap, snap_follows = capsnap.follows, dirty, values as of the snapshot, tid). Non-auth ranks ignore it without an ack. The auth resolves snap = realm->get_snap_following(follows), null-flushes older pending snaps (F-CAPS-025) and, if the snap maps to the head or the client is listed in client_need_snapflush[snap], journals EUpdate("snap flush") via _do_snap_update into the snapped inode (or the matching old_inodes entry), adjusting client_ranges; otherwise, or if the snap was deleted, it only acks. FLUSHSNAP_ACK (cap_id, seq, issue_seq and mseq 0) echoes snap_follows and the tid and is sent after commit. A FLUSHSNAP from a client with no cap on the head inode is dropped.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse
- **Map:** [capabilities.md › 3.11 Snapshots: snapflush](../cpp-map/02-client-protocol/capabilities.md#311-snapshots-snapflush)
- **Code:** `src/mds/Locker.cc:3530-3588 (Locker::handle_client_caps)`, `src/mds/Locker.cc:3879 (Locker::_do_snap_update)`, `linux/fs/ceph/caps.c:1583 (__send_flush_snap)`
- **Notes:** The kernel requires FLUSHSNAP_ACK to echo snap_follows and the FLUSHSNAP tid (linux/fs/ceph/caps.c:4075).

<a id="f-caps-025"></a>

#### F-CAPS-025 · Null snapflush inference

*MDS-side conclusion that a client will send no FLUSHSNAP, so pending snapshot gates can be released.*

A client sends FLUSHSNAP only when it has dirty snapshot metadata. After confirm_receipt, if the cap no longer has ANY_FILE_WR issued and (for MClientCaps updates only) the message lacks CEPH_CLIENT_CAPS_PENDING_CAPSNAP (flag 4), _do_null_snapflush journals an empty snap update for each pending snap of that client. Request-embedded releases and cap removal do not check the flag. A FLUSHSNAP for a later snap also null-flushes the older ones.

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse
- **Map:** [capabilities.md › 3.11 Snapshots: snapflush](../cpp-map/02-client-protocol/capabilities.md#311-snapshots-snapflush)
- **Code:** `src/mds/Locker.cc:3342 (Locker::_do_null_snapflush)`, `src/mds/Locker.cc:3632 (Locker::handle_client_caps)`, `src/mds/Locker.cc:3784 (Locker::process_request_cap_release)`, `src/mds/Locker.cc:4349 (Locker::remove_client_cap)`
- **Notes:** libcephfs sets FLAG_PENDING_CAPSNAP (src/client/Client.cc:4217).

<a id="f-caps-026"></a>

#### F-CAPS-026 · Snapflush nudge

*Periodic forcing of clients that delay their snapshot flush.*

caps_tick re-examines need_snapflush_inodes entries older than mds_freeze_tree_timeout / 3 (10 s by default); snapflush_nudge kicks a non-SYNC stable lock on the head inode with _rdlock_kick so the client's write caps are revoked, which makes the client send its FLUSHSNAP.

- **Class:** POSIX `extension` · exposure `config` · relied on by kclient, libcephfs, ceph-fuse
- **Gates:** mds_freeze_tree_timeout (30 s, dev)
- **Map:** [capabilities.md › 3.11 Snapshots: snapflush](../cpp-map/02-client-protocol/capabilities.md#311-snapshots-snapflush)
- **Code:** `src/mds/Locker.cc:3288 (Locker::snapflush_nudge)`, `src/mds/Locker.cc:4419-4435 (Locker::caps_tick)`

<a id="f-caps-027"></a>

#### F-CAPS-027 · XLOCKSNAP: truncate with pending snapflush

*Special filelock state that makes a truncation wait until buffered data has been flushed for a pending snapshot.*

MDCache::truncate_inode, when the inode has pending snapflushes and some client holds Fb, sets the already-xlocked filelock to LOCK_XLOCKSNAP (caps: only Fc) and retries on WAIT_STABLE; when the xlock is dropped, eval_gather waits until no Fb is issued before moving the lock to LOCK.

- **Class:** POSIX `extension` · exposure `internal` · relied on by kclient, libcephfs
- **Map:** [lock-types-and-state-machines.md › 3.14 Special-purpose states](../cpp-map/05-locking/lock-types-and-state-machines.md#314-special-purpose-states)
- **Code:** `src/mds/ScatterLock.h:59 (ScatterLock::set_xlock_snap_sync)`, `src/mds/MDCache.cc:6598-6603 (MDCache::truncate_inode)`, `src/mds/Locker.cc:1445-1457 (Locker::eval_gather)`
- **Notes:** The Fb check computes pending with logical || (0 or 1), so only the Fb bit test is meaningful (src/mds/Locker.cc:1447-1454).

### Cap release, staleness, recall and late revocation

How caps are given back (release, embedded releases, recall), what happens when a session goes stale or a client ignores revocations, and how caps are rebuilt after a failover.

<a id="f-caps-028"></a>

#### F-CAPS-028 · Batched cap release (MClientCapRelease)

*Clients give up caps in batches; the MDS removes them unless the cap was re-issued after the release was sent.*

CEPH_MSG_CLIENT_CAPRELEASE (0x313, v2) carries ceph_mds_cap_item {ino, cap_id (u64), migrate_seq, issue_seq (u32)}[] plus (v2) osd_epoch_barrier. handle_client_cap_release waits for replay if the rank is not clientreplay/active/stopping, applies the barrier, processes each item and accounts the releases for recall (Session::notify_cap_release). _do_cap_release ignores unknown inodes or caps, a mismatched cap_id or an older mseq, defers on frozen inodes, and if issue_seq < last_issue keeps the cap (it was re-issued after the client released it) and only runs clean_revoke_from(issue_seq) and eval_cap_gather; otherwise remove_client_cap: null snapflush, invalidate lock caches, unlink from the session/revoking/realm lists, drop the loner, release that client's fcntl/flock locks and wake waiters, clear the client's writeable range (NEEDSRECOVER if killed) and re-evaluate the locks.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.12 Cap release](../cpp-map/02-client-protocol/capabilities.md#312-cap-release)
- **Code:** `src/mds/Locker.cc:4258 (Locker::handle_client_cap_release)`, `src/mds/Locker.cc:4306 (Locker::_do_cap_release)`, `src/mds/Locker.cc:4345 (Locker::remove_client_cap)`, `src/mds/CInode.cc:3625 (CInode::remove_client_cap)`
- **Notes:** The kernel batches one page of items (170 with 4 KiB pages) and flushes when a page fills, on CEPH_SESSION_FLUSHMSG, from periodic work, or when it receives a GRANT/REVOKE/IMPORT for an inode it does not have, replying with a release that uses the message's seq as issue_seq (linux/fs/ceph/mds_client.c:2502, linux/fs/ceph/caps.c:4549-4690). Session close or kill removes all caps through remove_client_cap. See also F-PROTO-092 (MClientCapRelease encoding).

<a id="f-caps-029"></a>

#### F-CAPS-029 · Cap and dentry-lease releases embedded in requests

*Clients drop caps and dentry leases inside an MClientRequest so the MDS does not have to revoke them from the requester.*

MClientRequest carries ceph_mds_request_release {ino, cap_id (u64), caps, wanted, seq, issue_seq, mseq, dname_seq, dname_len (u32)} + dname records. Server::handle_client_request applies them through process_request_cap_release before dispatch and before any lock is taken, except for replayed requests: it removes the named dentry lease (without comparing dname_seq), applies the cap_id/mseq/frozen checks, confirm_receipt(seq, caps), null-snapflush inference, adjust_cap_wanted and eval with the cap suppressed, and records mdr->cap_releases[vino] = last_sent. At request cleanup kick_cap_releases re-issues caps whose last_sent is unchanged, so suppressed grants are not lost. The kernel drops, for example, Fs|Ax|Xx on the parent for create/mknod/mkdir unless it holds Fx, Fs|Xx on the parent and Ls|Lx on the victim for unlink, Ls|Lx on the source inode for rename, Xs for setxattr.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.12 Cap release](../cpp-map/02-client-protocol/capabilities.md#312-cap-release) · [leases-lock-caches-and-async-dirops.md › 3.1.4 Lifecycle: renew, release, revoke, expiry](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#314-lifecycle-renew-release-revoke-expiry) · [acquiring-locks.md › 7. Externally visible contract](../cpp-map/05-locking/acquiring-locks.md#7-externally-visible-contract)
- **Code:** `src/mds/Locker.cc:3714 (Locker::process_request_cap_release)`, `src/mds/Locker.cc:3833-3844 (Locker::kick_cap_releases)`, `src/mds/Server.cc:2716-2724 (Server::handle_client_request)`, `linux/fs/ceph/caps.c:4934 (ceph_encode_inode_release)`, `linux/fs/ceph/caps.c:5025 (ceph_encode_dentry_release)`
- **Notes:** The cap part is deferred until unfreeze when should_defer_client_cap_frozen says so (inode frozen, or freezing with no auth pins while its dirfrag is not auth-pinned by a lock cache); rdlock_path_pin_ref therefore refuses a freezing target it has not pinned. Because the requester's own Fs/lease is dropped first, the SYNC -> LOCK gather its request triggers only waits for other clients. See also F-PROTO-070 (record encoding and client behaviour).

<a id="f-caps-030"></a>

#### F-CAPS-030 · Stale-session cap handling

*What happens to a client's caps when its session stops renewing: forced revocation, eviction of writers, and re-issue on resume.*

When the session is marked STALE (session lifecycle owned by 02-client-sessions-and-protocol), revoke_stale_caps bumps Session::cap_gen so all its caps become invalid, walks its notable caps and, for caps being revoked, returns false (the caller evicts the client, blocklisting per mds_session_blocklist_on_timeout) if any revoking bit is in ANY_WR, otherwise force-acks with cap->revoke(), invalidates lock caches for dir-op bits and marks NEEDSRECOVER if the client had a writeable range; then it re-evaluates gathers, the session's leases are dropped and the client is sent CEPH_SESSION_STALE. While stale, loner selection and wanted ignore the session, replies issue no caps, and later revocations go through revoke_stale_cap (evict if ANY_WR, else force-revoke). On REQUEST_RENEWCAPS from a stale session resume_stale_caps re-evaluates (only notable caps for CEPHFS_FEATURE_LAZY_CAP_WANTED clients); invalid caps are re-issued with a GRANT, followed by a REVOKE if bits must also go.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha, osd
- **Legacy parts** (old-clients): resume_stale_caps for a session without CEPHFS_FEATURE_LAZY_CAP_WANTED (bit 11): the loop does not stop at the first non-notable cap, so every cap of the session is re-evaluated and re-issued. This is the only use of CEPHFS_FEATURE_LAZY_CAP_WANTED in src/mds. The 7.3 kernel, the CentOS Stream 8 and 9 samples and libcephfs advertise bit 11, so their stale-session resume walks only notable caps. Kernels before v5.0 and Ceph clients before 13.2.7 lack the bit. Evidence: `src/mds/Locker.cc:2849-2855 (Locker::resume_stale_caps)`, `linux/fs/ceph/mds_client.h:49 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs8-kernel/fs/ceph/mds_client.h:42 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `cs9-kernel/fs/ceph/mds_client.h:47 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `src/mds/cephfs_features.h:66 (CEPHFS_FEATURES_ALL)`, `src/mds/cephfs_features.h:105 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `linux-v5.0/fs/ceph/mds_client.h:25-29 (CEPHFS_FEATURES_CLIENT_SUPPORTED)`, `v13.2.7:src/mds/cephfs_features.h:26 (CEPHFS_FEATURE_LAZY_CAP_WANTED)`.
- **Gates:** mds_session_blocklist_on_timeout (default true); CEPHFS_FEATURE_LAZY_CAP_WANTED (bit 11); defer_client_eviction_on_laggy_osds (default false)
- **Map:** [capabilities.md › 3.13 Stale sessions](../cpp-map/02-client-protocol/capabilities.md#313-stale-sessions) · [lock-types-and-state-machines.md › 8. Edge cases, failure handling and gotchas](../cpp-map/05-locking/lock-types-and-state-machines.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Locker.cc:2778 (Locker::revoke_stale_caps)`, `src/mds/Locker.cc:2742 (Locker::revoke_stale_cap)`, `src/mds/Locker.cc:2845 (Locker::resume_stale_caps)`, `src/mds/Server.cc:1299-1305 (Server::find_idle_sessions)`
- **Notes:** The kernel bumps s_cap_gen on CEPH_SESSION_STALE and treats invalid caps as p only (linux/fs/ceph/mds_client.c:4628). revoke_stale_cap does not call eval_lock_caches, so a lock cache whose Dc/Dr is revoked from a stale cap keeps its cap reference until the cap is removed (by code reading). There is no lock-level timeout for live clients (F-CAPS-031). See also F-PROTO-043 (stale detection and the STALE message).

<a id="f-caps-031"></a>

#### F-CAPS-031 · Late revocation warnings, health and eviction

*Detection and reporting of clients that do not answer cap revocations, with optional eviction.*

Caps with an outstanding REVOKE sit on Locker::revoking_caps and revoking_caps_by_client ordered by last_revoke_stamp. caps_tick (each Locker::tick) walks the oldest (at most 100 per tick) and logs "client.X isn't responding to mclientcaps(revoke), ino I pending P issued Q, sent N seconds ago" when the age exceeds session_timeout * 2^num_revoke_warnings. The beacon uses get_late_revoking_clients(session_timeout) to raise MDS_CLIENT_LATE_RELEASE ("Client NAME failing to respond to capability release") or MDS_CLIENT_LATE_RELEASE_MANY above mds_health_summarize_threshold clients. Server::evict_cap_revoke_non_responders evicts such clients after mds_cap_revoke_eviction_timeout (default 0 = disabled), blocklisting per mds_session_blocklist_on_evict, unless deferred for laggy OSDs (counter cap_revoke_eviction).

- **Class:** POSIX `operational` · exposure `metrics`, `config` · relied on by mon, mgr, admin
- **Gates:** mds_cap_revoke_eviction_timeout (default 0, off); mds_session_blocklist_on_evict (default true); mds_health_summarize_threshold (default 10); MDSMap session_timeout (default 60 s)
- **Map:** [capabilities.md › 3.14 Late revocation: warnings, health, eviction](../cpp-map/02-client-protocol/capabilities.md#314-late-revocation-warnings-health-eviction)
- **Code:** `src/mds/Locker.cc:4415 (Locker::caps_tick)`, `src/mds/Locker.cc:4379 (Locker::get_late_revoking_clients)`, `src/mds/Beacon.cc:347-379 (Beacon::notify_health)`, `src/mds/Server.cc:1361 (Server::evict_cap_revoke_non_responders)`
- **Notes:** Without eviction, a live client that never acks blocks the lock transition indefinitely (no lock-level timeout). See also F-ADM-075 (late-release health), F-PROTO-048 (cap-revoke eviction).

<a id="f-caps-032"></a>

#### F-CAPS-032 · Cache-pressure cap recall (CEPH_SESSION_RECALL_STATE)

*The MDS asks clients holding many caps to trim to a lower limit so its cache can shrink.*

Server::recall_client_state runs every mds_cache_trim_interval (1 s) on the upkeep thread (active, clientreplay, stopping) and for the `cache drop` command. Candidates: sessions with num_caps > mds_max_caps_per_client (1 Mi), sessions whose session_cache_liveness < num_caps >> mds_session_cache_liveness_magnitude, or all sessions when trimming. Each gets a new limit of num_caps minus at most mds_recall_max_caps (30000), bounded below by mds_min_caps_per_client (100); per-session and global decay-counter throttles (mds_recall_max_decay_threshold, mds_recall_global_max_decay_threshold, 128 Ki each) cap the recall rate. The MDS sends CEPH_SESSION_RECALL_STATE with head.max_caps = new limit; clients trim and send cap releases, accounted by notify_cap_release. MDS_CLIENT_RECALL(_MANY) health is raised when recall_caps exceeds mds_recall_warning_threshold (256 Ki) and num_caps > mds_min_caps_working_set (10000).

- **Class:** POSIX `operational` · exposure `client-wire`, `metrics`, `config`, `admin` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon, mgr
- **Gates:** mds_max_caps_per_client (1 Mi); mds_recall_max_caps (30000); mds_min_caps_per_client (100); mds_recall_max_decay_threshold (128 Ki); mds_recall_global_max_decay_threshold (128 Ki); mds_recall_warning_threshold (256 Ki)
- **Map:** [sessions.md › 3.12 Cache-pressure recall (CEPH_SESSION_RECALL_STATE)](../cpp-map/02-client-protocol/sessions.md#312-cache-pressure-recall-ceph_session_recall_state) · [capabilities.md › 3.12 Cap release](../cpp-map/02-client-protocol/capabilities.md#312-cap-release)
- **Code:** `src/mds/Server.cc:1972-2103 (Server::recall_client_state)`, `src/mds/SessionMap.cc:1033-1066 (Session::notify_cap_release)`, `src/mds/Beacon.cc:382-432 (Beacon::notify_health)`, `linux/fs/ceph/mds_client.c:2345-2365 (ceph_trim_caps)`
- **Notes:** Not a staging feature of this category's items; catalogued here because cap recall belongs to this category (described on the sessions page). Session::recall_limit is never assigned. Counters session_recall_throttle, session_recall_throttle2o, global_recall_throttle. libcephfs exposes cache-pressure callbacks to NFS-Ganesha. See also F-ADM-075 (MDS_CLIENT_RECALL health).

<a id="f-caps-034"></a>

#### F-CAPS-034 · Cap re-establishment after MDS failover (cap side)

*Rebuilding caps and lock states from what clients report when a rank restarts.*

Caps are memory-only and are rebuilt from client reconnect messages (message format F-PROTO-056; acceptance F-PROTO-057). CInode::reconnect_cap creates a cap with the client's cap_id, issue_norevoke(issued), last_sent = last_issue = 0 and mseq 0, or merges into an existing cap (wanted and issued ORed, seq bumped); it is the only path that takes issued bits from a client without masking. Dirty bits of flushes re-sent during reconnect are recorded. The recovery steps that use these caps are F-CLU-082 (recording), F-CLU-088 (moving caps to the authority with IMPORT/EXPORT), F-CLU-090 (lock states chosen from the caps) and F-CLU-098 (stale EXPORT for caps of missing inodes, reissue_all_caps at activation); snapflush expectations are rebuilt from snap_follows (F-SNAP-032).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [capabilities.md › 3.16 Recovery interplay (summary)](../cpp-map/02-client-protocol/capabilities.md#316-recovery-interplay-summary) · [lock-types-and-state-machines.md › 3.15 Lock state across replication, migration and recovery](../cpp-map/05-locking/lock-types-and-state-machines.md#315-lock-state-across-replication-migration-and-recovery) · [resolve-and-rejoin.md › 3.9 Rejoin start and cap matching, round 1](../cpp-map/09-recovery/resolve-and-rejoin.md#39-rejoin-start-and-cap-matching-round-1)
- **Code:** `src/mds/CInode.cc:3676-3690 (CInode::reconnect_cap)`, `src/mds/CInode.cc:3503-3544 (CInode::choose_lock_states)`, `src/mds/MDCache.cc:5698 (MDCache::choose_lock_states_and_reconnect_caps)`, `src/mds/MDCache.cc:6351 (MDCache::reissue_all_caps)`, `src/mds/MDCache.cc:5873 (MDCache::export_remaining_imported_caps)`
- **Notes:** The kernel zeroes seq/issue_seq/mseq and drops CEPH_CAP_ANY_DIR_OPS from directory caps in its reconnect record (linux/fs/ceph/mds_client.c:4919-4937); libcephfs reports only bits it was granted. Flushes re-sent early during reconnect must be accepted (ceph_early_kick_flushing_caps, linux/fs/ceph/caps.c:2696). last_cap_id is raised to the largest reported cap_id.

### Caps across multiple ranks

Caps issued by replica ranks, the forwarding of wanted caps to the auth rank, and the client-visible cap migration messages.

<a id="f-caps-035"></a>

#### F-CAPS-035 · Replica caps and MInodeFileCaps

*Caps issued by non-auth ranks and the forwarding of their clients' wanted caps to the auth rank.*

A non-auth rank issues only the replica column (e.g. Fscr in filelock SYNC, Fr in MIX, As/Ls/Xs in SYNC) and never CEPH_CAP_FLAG_AUTH; dirty flushes are honoured only on the auth. request_inode_file_caps sends MInodeFileCaps (MSG_MDS_INODEFILECAPS 0x301, {ino, caps}, v1) with the union of local clients' wanted & allowed_ever (minus p) to the auth whenever it changes (waiting for single auth or rejoin). handle_inode_file_caps stores it in mds_caps_wanted[rank] and re-evaluates; any replica wanted disables the loner and counts as other_wanted in file_eval.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks, kclient, libcephfs
- **Map:** [capabilities.md › 3.15 Multiple MDS ranks: replica caps, export/import](../cpp-map/02-client-protocol/capabilities.md#315-multiple-mds-ranks-replica-caps-exportimport) · [lock-types-and-state-machines.md › 3.11 Inter-MDS lock protocol (MLock)](../cpp-map/05-locking/lock-types-and-state-machines.md#311-inter-mds-lock-protocol-mlock)
- **Code:** `src/mds/Locker.cc:2899 (Locker::request_inode_file_caps)`, `src/mds/Locker.cc:2931 (Locker::handle_inode_file_caps)`, `src/mds/CInode.cc:3878 (CInode::get_caps_wanted)`
- **Notes:** The kclient and libcephfs send a getattr to the auth rank when they want an s bit while holding any x cap, or ask for RSTAT or Xs, because a replica would have to REQRDLOCK and force the auth out of EXCL (lock-types page section 7 item 5).

<a id="f-caps-036"></a>

#### F-CAPS-036 · Cap migration to clients (EXPORT / IMPORT)

*Client-visible messages that move a client's auth cap from one rank to another.*

When caps move (subtree export, cross-rank rename, resolve/rejoin), the exporter sends CEPH_CAP_OP_EXPORT with seq 0 and peer {cap_id, issue_seq, mseq, mds, flags} naming the importer, encoded at the size offset of the body; the importer merges the Capability::Export, adopts mseq+1, and sends CEPH_CAP_OP_IMPORT (MDCache::do_cap_import) with the snap trace, seq = last_sent, issue_seq = seq, and peer = {exporter cap_id, seq, mseq, rank, CEPH_CAP_FLAG_AUTH}; CEPH_CAP_FLAG_RELEASE asks the client to release the peer's cap. EXPORT with peer.mds = -1 makes the client drop the cap. Caps created by a subtree import are STATE_IMPORTING until import_finish so import_reverse can remove them.

- **Class:** POSIX `supporting` · exposure `client-wire`, `mds-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mds-ranks
- **Map:** [capabilities.md › 3.15 Multiple MDS ranks: replica caps, export/import](../cpp-map/02-client-protocol/capabilities.md#315-multiple-mds-ranks-replica-caps-exportimport) · [subtree-migration.md › 3.11 Client-visible cap migration](../cpp-map/08-multi-mds/subtree-migration.md#311-client-visible-cap-migration)
- **Code:** `src/mds/Migrator.cc:1824 (Migrator::finish_export_inode_caps)`, `src/mds/Migrator.cc:3532 (Migrator::finish_import_inode_caps)`, `src/mds/MDCache.cc:5954 (MDCache::do_cap_import)`, `linux/fs/ceph/caps.c:4212-4234 (handle_cap_export)`, `linux/fs/ceph/caps.c:4342-4357 (handle_cap_import)`
- **Notes:** The kernel adds every IMPORTed cap with CEPH_CAP_FLAG_AUTH, so only the auth rank may send IMPORT; on EXPORT it creates a placeholder cap with seq = issue_seq = peer.issue_seq - 1 and mseq = peer.mseq, so the importer's IMPORT must carry seq == peer.issue_seq, and it logs "mismatched seq/mseq" otherwise. On IMPORT of the auth cap it re-sends pending flushes and capsnaps with their original tids and re-requests wanted and max_size. Ordering with CEPH_SESSION_FLUSHMSG and the export state machine are owned by 07-multi-mds. See also F-CLU-088 (rejoin cap moves), F-MULTI-012 (EXPORT in subtree migration), F-MULTI-013 (IMPORT in subtree migration).

### Dentry leases and directory-wide validity

Per-dentry leases (issue, encoding, renew, revoke, expiry) and the client-side rule that Fs on a directory validates all of its cached dentries.

<a id="f-caps-038"></a>

#### F-CAPS-038 · Dentry lease issue

*Time-limited permission for one client to trust one cached dentry (positive or negative) without asking the MDS.*

Locker::issue_client_lease appends a lease record for the dentry of every reply trace and every readdir entry. A real lease (mask CEPH_LEASE_VALID, duration 30000 ms, seq from a per-lease counter seeded from Session::lease_seq) is issued only if the request is not on a snapshot, the dentry lock can_lease for the client (SYNC on auth or replica; LOCK_SYNC or XLOCKDONE for the xlocking client), the parent is not a stray directory, the parent directory's filelock is not leasable (not SYNC and not the client's own XLOCKDONE), and the client has neither Fs nor Fx pending on the directory; otherwise a record with mask 0, duration 0, seq 0 is sent. A ClientLease is kept per (dentry, client) on the dentry, the session and a rank-wide TTL list; the first lease sets LEASED on the dentry lock and pins the dentry (not trimmed; a stray with leases is not purged).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.1.2 When a lease is issued (Locker::issue_client_lease)](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#312-when-a-lease-is-issued-lockerissue_client_lease) · [leases-lock-caches-and-async-dirops.md › 3.1.1 State](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#311-state)
- **Code:** `src/mds/Locker.cc:4542-4589 (Locker::issue_client_lease)`, `src/mds/SimpleLock.h:353 (SimpleLock::can_lease)`, `src/mds/CDentry.cc:598 (CDentry::add_client_lease)`, `src/mds/Server.cc:2546 (Server::set_trace_dist)`, `src/mds/Server.cc:5304 (Server::handle_client_readdir)`
- **Notes:** The duration is hard-coded: MDCache::client_lease_durations {5, 30, 300} s with pool 1 (30 s) always used ("fixme.. do something smart!"). set_trace_dist encodes the directory inode and its caps before the lease, so caps granted in the same reply suppress the lease. Leases are never issued in the final reply if an early reply was sent. Replica ranks issue leases too (DN lock SYNC is ANY). Only CDentry::lock ever carries LEASED; the is_leased() branches in inode-lock transitions are never taken. Real leases are thus only issued while the directory filelock is not SYNC (rationale is an inference, no code comment). See also F-PROTO-083 (lease record encoding).

<a id="f-caps-039"></a>

#### F-CAPS-039 · Negative dentry leases

*Leases on null dentries that let clients cache the non-existence of a name.*

An -ENOENT lookup with CEPH_MDS_FLAG_WANT_DENTRY (2) returns a trace with the null dentry (rdlock_path_pin_ref sets mdr->tracedn), which can carry a real lease with mask CEPH_LEASE_VALID and no PRIMARY_LINK.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.1.2 When a lease is issued (Locker::issue_client_lease)](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#312-when-a-lease-is-issued-lockerissue_client_lease)
- **Code:** `src/mds/Server.cc:3906-3910 (Server::rdlock_path_pin_ref)`, `src/mds/Locker.cc:4554-4564 (Locker::issue_client_lease)`, `linux/fs/ceph/inode.c:1766-1773 (ceph_fill_trace)`
- **Notes:** The kclient keeps negative dentries only if have_lease = (the dir cap in the reply has Fs) || duration_ms != 0.

<a id="f-caps-040"></a>

#### F-CAPS-040 · CEPH_LEASE_PRIMARY_LINK hint

*Lease mask bit telling the client that a dentry is the inode's primary link.*

Real leases on dentries whose linkage, as seen by the client (projected if it may read projected state), is primary carry CEPH_LEASE_PRIMARY_LINK (4); remote and null linkages do not. Since commit 907b1f41e5e (2022, tracker 54701) mask-0 records never carry it. The kclient sets or clears CEPH_DENTRY_PRIMARY_LINK from every lease record it applies and uses it as a precondition for async unlink and to choose the path hint in cap reconnect; libcephfs marks the dentry primary.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.1.2 When a lease is issued (Locker::issue_client_lease)](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#312-when-a-lease-is-issued-lockerissue_client_lease) · [leases-lock-caches-and-async-dirops.md › 3.1.7 Client behaviour differences](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#317-client-behaviour-differences)
- **Code:** `src/mds/Locker.cc:4554 (Locker::issue_client_lease)`, `src/mds/Locker.cc:4584 (Locker::issue_client_lease)`, `linux/fs/ceph/inode.c:1436-1441 (__update_dentry_lease)`, `linux/fs/ceph/dir.c:1332-1334 (get_caps_for_async_unlink)`, `linux/fs/ceph/mds_client.c:4863 (d_find_primary)`
- **Notes:** By code reading, a client holding Fs/Fx on a directory (a precondition for async unlink) only receives mask-0 records for its dentries, so the kernel async-unlink test fails unless the flag survived from an earlier real lease (leases page G2).

<a id="f-caps-042"></a>

#### F-CAPS-042 · Lease renewal (CEPH_MDS_LEASE_RENEW)

*Kernel-client request to extend a dentry lease before it expires.*

MClientLease (CEPH_MSG_CLIENT_LEASE 0x311) payload is the packed ceph_mds_lease {u8 action, le16 mask, le64 ino (parent directory), le64 first, le64 last, le32 seq, le32 duration_ms} followed by string dname, with no versioned envelope. The kclient sends RENEW (3) on d_revalidate after half the duration; handle_client_lease resolves the dentry and the client's lease (silently dropping the message if either is missing) and, if the dentry lock can still be leased, replies with a copy of the message with duration_ms = 30000 and seq = ++lease seq and resets the server ttl; otherwise it sends nothing. libcephfs never renews.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.1.4 Lifecycle: renew, release, revoke, expiry](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#314-lifecycle-renew-release-revoke-expiry) · [leases-lock-caches-and-async-dirops.md › 3.1.3 Encodings](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#313-encodings)
- **Code:** `src/mds/Locker.cc:4472 (Locker::handle_client_lease)`, `src/messages/MClientLease.h:24-96 (MClientLease)`, `linux/fs/ceph/dir.c:1865 (dentry_lease_is_valid)`, `linux/fs/ceph/mds_client.c:6101-6117 (handle_lease)`
- **Notes:** RENEW re-checks neither the directory filelock nor the client's Fs/Fx and ignores the client's seq. Any other action value (including REVOKE) from a client that holds a lease on the named dentry reaches ceph_abort() (src/mds/Locker.cc:4535-4537). The kernel accepts the reply only if the lease session, lease_gen == s_cap_gen and an outstanding renew match. The MDS decoder ignores trailing bytes (the kernel pads NAME_MAX). See also F-PROTO-093 (encoding).

<a id="f-caps-043"></a>

#### F-CAPS-043 · Lease revocation on dentry change

*Before a dentry's linkage changes, every client lease on it (on every rank) is revoked and acknowledged or expired.*

Any change of a dentry's linkage xlocks its DN lock; simple_lock (SYNC -> SYNC_LOCK) calls revoke_client_leases if the lock is LEASED, sending MClientLease REVOKE (1) {mask 1, ino = parent dir, first = diri->first, last = CEPH_NOSNAP, dname, seq} to every holder as a counted message, and eval_gather completes only when no lease remains. REVOKE_ACK (4; the kclient reuses the message and may substitute its own newer seq) and RELEASE (2; libcephfs echoes seq and mask) remove the lease only if seq matches; otherwise the change waits for TTL expiry. Replicas receiving LOCK_AC_LOCK revoke the leases they issued and ack only after they are gone, so replica leases also hold back the auth's xlock.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.1.5 Revocation when a dentry must change](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#315-revocation-when-a-dentry-must-change) · [leases-lock-caches-and-async-dirops.md › 3.1.4 Lifecycle: renew, release, revoke, expiry](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#314-lifecycle-renew-release-revoke-expiry)
- **Code:** `src/mds/Locker.cc:5100-5103 (Locker::simple_lock)`, `src/mds/Locker.cc:4592 (Locker::revoke_client_leases)`, `src/mds/Locker.cc:4503-4511 (Locker::handle_client_lease)`, `src/mds/Locker.cc:4788-4800 (Locker::handle_simple_lock)`, `src/mds/CDentry.cc:618 (CDentry::remove_client_lease)`
- **Notes:** Timing contract: a dentry change waits until every lease holder on every rank acks or releases, or until server-side expiry (at most 30 s after the last issue or renew, plus up to mds_cache_trim_interval). The kclient answers REVOKE even when it no longer has the dentry; libcephfs asserts the action is REVOKE and drops its lease only if the mask intersects CEPH_LEASE_VALID. See also F-PROTO-093 (encoding).

<a id="f-caps-044"></a>

#### F-CAPS-044 · Lease expiry and cleanup

*Server-side removal of expired leases and of all leases of stale, closed or killed sessions.*

MDCache::trim_client_leases removes leases whose ttl has passed from the head of each TTL-sorted list, every mds_cache_trim_interval (1 s) on the upkeep thread, only while the rank is active, clientreplay or stopping. All leases of a session are removed when it goes stale (remove_stale_leases, after a successful revoke_stale_caps) and when it is closed or killed (_session_logged); leases on null dentries of a directory that became a stray are removed. Removing the last lease on a dentry clears LEASED and re-runs eval_gather if the lock is gathering. Leases are memory-only; clients invalidate theirs on CEPH_SESSION_STALE and on reconnect by bumping their cap generation.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_cache_trim_interval (1 s)
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.1.4 Lifecycle: renew, release, revoke, expiry](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#314-lifecycle-renew-release-revoke-expiry) · [leases-lock-caches-and-async-dirops.md › 4. Data structures and invariants](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#4-data-structures-and-invariants)
- **Code:** `src/mds/MDCache.cc:7933 (MDCache::trim_client_leases)`, `src/mds/MDCache.cc:14724-14736 (MDCache::upkeep_main)`, `src/mds/Locker.cc:2872 (Locker::remove_stale_leases)`, `src/mds/Server.cc:991-996 (Server::_session_logged)`
- **Notes:** Server ttl >= client ttl: the MDS measures from issue time, the kernel from request start and libcephfs from send time, with the same duration. A rank in other states keeps expired leases and their dentries stay pinned (leases page G15).

<a id="f-caps-045"></a>

#### F-CAPS-045 · Infinite leases on snapshot listings

*Snapshot directory entries are handed out with a never-expiring lease.*

LSSNAP replies encode LeaseStat(CEPH_LEASE_VALID, -1, 0) for every snapshot entry (duration 0xFFFFFFFF ms), not tracked server side and never revoked.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.1.2 When a lease is issued (Locker::issue_client_lease)](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#312-when-a-lease-is-issued-lockerissue_client_lease)
- **Code:** `src/mds/Server.cc:11545 (Server::handle_client_lssnap)`

<a id="f-caps-046"></a>

#### F-CAPS-046 · Directory-wide dentry validity via Fs

*Clients trust all cached dentries of a directory, and answer ENOENT locally for a complete directory, while they hold Fs on it; the MDS guarantees this by revoking Fs before any namespace change.*

The MDS has no directory-lease object. kclient: i_shared_gen is bumped when Fs is newly issued and the directory is marked not complete when Fs is gained or lost; every applied lease record (even mask 0) stamps the dentry with the parent's shared gen; a dentry is valid if its own lease is valid or if the parent holds Fs and the stamp equals i_shared_gen; with Fs and a complete directory a negative lookup is answered locally. libcephfs uses shared_gen and I_COMPLETE the same way. MDS side: every namespace change in a directory wrlocks the directory inode's filelock (and nestlock); SYNC, the only state giving Fs to non-loner clients, has no wrlock, so the lock moves to LOCK/MIX (or is EXCL for the loner), revoking Fs from all other clients first. Lock-derived directory caps never include Fc, Fr, Fa, Fl.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.1.6 Directory-wide validity (Fs) and directory complete...](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#316-directory-wide-validity-fs-and-directory-completeness) · [lock-types-and-state-machines.md › 7. Externally visible contract](../cpp-map/05-locking/lock-types-and-state-machines.md#7-externally-visible-contract)
- **Code:** `src/mds/MDCache.cc:8649-8656 (MDCache::path_traverse)`, `linux/fs/ceph/caps.c:598-611 (__check_cap_issue)`, `linux/fs/ceph/dir.c:1942 (dir_lease_is_valid)`, `linux/fs/ceph/dir.c:835-860 (ceph_lookup)`, `src/client/Client.cc:7905-7930 (Client::_lookup)`
- **Notes:** A readdir (filelock rdlock) and a create (filelock wrlock) in the same directory force a lock transition each time they alternate; directories never use XSYN.

### Lock caches, async directory operations and delegated inode numbers

The kernel client's asynchronous create/unlink path: Dc/Dr directory caps backed by MDS lock caches, preallocated and delegated inode numbers, and their replay and purge. Since commit 9b4a93f2b7b the C++ MDS never creates a lock cache, so this path is dormant against it (see the notes of each entry).

<a id="f-caps-047"></a>

#### F-CAPS-047 · Directory-operation cap bits Dc and Dr

*Directory cap bits that would allow a client to create or unlink names asynchronously.*

On directory caps CEPH_CAP_DIR_CREATE (Dc = Fc, 0x400) and CEPH_CAP_DIR_UNLINK (Dr = Fr, 0x800) mean "may create / unlink without waiting". get_allowed_caps and get_caps_allowed_for_client mask CEPH_CAP_ANY_DIR_OPS (0xCC00) out of lock-derived directory caps and add back Capability::lock_cache_allowed only when Fx is allowed (the client is the directory's loner with filelock EXCL). Since a directory's liked set is p|ANY_EXCL|ANY_SHARED, Dc/Dr are issued only if wanted; the kclient wants ANY_DIR_OPS and Fx on directories it wrote recently when mounted with async dirops. Revoking Dc/Dr (ack, release, stale) invalidates the attached lock caches.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.2.2 How Dc/Dr are granted](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#322-how-dcdr-are-granted) · [capabilities.md › 3.17 Directory caps and lock caches (summary)](../cpp-map/02-client-protocol/capabilities.md#317-directory-caps-and-lock-caches-summary) · [leases-lock-caches-and-async-dirops.md › 5.2 Cap bits](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#52-cap-bits)
- **Code:** `src/mds/Locker.cc:2563-2567 (Locker::get_allowed_caps)`, `src/mds/CInode.cc:3818-3822 (CInode::get_caps_allowed_for_client)`, `src/mds/Capability.h:387 (Capability::lock_cache_allowed)`, `src/include/ceph_fs.h:908-912 (CEPH_CAP_DIR_CREATE)`, `linux/fs/ceph/caps.c:1047-1059 (__ceph_caps_file_wanted)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS. libcephfs never uses Dc/Dr, but may list Fr|Fc in wanted for a directory opened with open() (by code reading). Clients never get Fx, hence never Dc/Dr, on directories with subtree-root or exporting dirfrags (F-CAPS-005).

<a id="f-caps-048"></a>

#### F-CAPS-048 · Lock cache creation

*MDS-internal object that keeps the locks and auth pins of one client's create or unlink stream in one directory, advertised to the client as Dc/Dr.*

An MDLockCache (a MutationImpl) is keyed by (client cap on the directory, opcode CREATE or UNLINK). create_lock_cache is called from handle_client_openc (depth-1 path) and handle_client_unlink (primary link, not rmdir, depth-1) when mds_allow_async_dirops is set, for sync and async requests alike. It is a no-op unless: the request has no cache yet; the directory is auth; there are no peer requests; the client has a cap on the directory; no cache for that opcode exists on the cap; cap->issued() already contains the op's bit; every object the request auth-pinned can still be pinned; every open dirfrag is auth, pinnable and without freezing/frozen inodes; all locks held on the directory and ancestors are stable. It then sets lock_cache_allowed, auth-pins every open dirfrag with disable_frozen_inode, pins or auth-pins the directory and ancestors, and moves from the request into the cache the rdlocks on the directory and ancestors (snaplock chain; policylock chain and directory authlock for CREATE) and the wrlocks on the directory (filelock, nestlock, quiescelock), marking them CACHED.

- **Class:** POSIX `internal` · exposure `config` · relied on by kclient, internal
- **Gates:** mds_allow_async_dirops (default true; gates the create_lock_cache calls and the openc_response_t format)
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.2.3 Creation (Locker::create_lock_cache, src/mds/Locker...](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#323-creation-lockercreate_lock_cache-srcmdslockercc1092-lockercreate_lock_cache) · [leases-lock-caches-and-async-dirops.md › 3.2.1 What a lock cache is](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#321-what-a-lock-cache-is) · [acquiring-locks.md › 3.12 Lock caches](../cpp-map/05-locking/acquiring-locks.md#312-lock-caches)
- **Code:** `src/mds/Locker.cc:1092-1245 (Locker::create_lock_cache)`, `src/mds/Locker.cc:1124-1128 (Locker::create_lock_cache)`, `src/mds/Mutation.h:540 (MDLockCache)`, `src/mds/Server.cc:4987-4988 (Server::handle_client_openc)`, `src/mds/Server.cc:8714-8715 (Server::handle_client_unlink)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS. History: in the original design (9b419b91f1b, 2019) the first synchronous create/unlink created the cache with no issued check, which is how Dc/Dr were first granted; a86ee496be9 (PR #61250, merged 2025-01-27) wrapped both call sites in mds_allow_async_dirops as a workaround because "the lock cache is buggy" (tracker 65607). The Server member mds_allow_async_dirops starts as true and is updated only in handle_conf_change, so a value set before the rank starts is not applied until it changes at runtime (leases page G6, unverified at runtime). Lock caches are memory-only.

<a id="f-caps-049"></a>

#### F-CAPS-049 · Lock cache attachment

*Later create or unlink requests of the same client in the same directory reuse the cached locks.*

rdlock_path_xlock_dentry adds MDS_TRAVERSE_CHECK_LOCKCACHE for depth-1 paths (the kernel builds DIRINO/NAME for create and unlink); path_traverse attaches the first attachable cache on the client's cap with the same opcode (find_and_attach_lock_cache, ref++), regardless of CEPH_MDS_FLAG_ASYNC. With a cache attached the request skips try_rdlock_snap_layout (copying dir_layout from the cache) and the tail's parent filelock/nestlock wrlocks and authlock rdlock; is_rdlocked/is_wrlocked report cached locks as held; it only xlocks the dentry (plus the versionlock wrlock) and, for unlink, the target's linklock/snaplock and the stray locks. mknod, mkdir, symlink, link and rmdir disable lock caches; rename never sets the flag. Requests with a lock cache cannot be batched; at completion _drop_locks puts the cache reference.

- **Class:** POSIX `internal` · exposure `internal` · relied on by kclient, internal
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.2.4 Attachment and effect on request locking](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#324-attachment-and-effect-on-request-locking) · [acquiring-locks.md › 3.12 Lock caches](../cpp-map/05-locking/acquiring-locks.md#312-lock-caches)
- **Code:** `src/mds/Locker.cc:1247 (Locker::find_and_attach_lock_cache)`, `src/mds/MDCache.cc:8491-8497 (MDCache::path_traverse)`, `src/mds/Server.cc:3996 (Server::rdlock_path_xlock_dentry)`, `src/mds/Mutation.cc:93-109 (MutationImpl::is_rdlocked)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS. This code is therefore never exercised.

<a id="f-caps-050"></a>

#### F-CAPS-050 · Lock cache invalidation and lifecycle

*Triggers that take a lock cache away and the release of its locks and pins.*

invalidate_lock_cache marks the cache invalidating, detaches it from the dirfrag lists (keeping its auth pins), clears the bit from lock_cache_allowed and, if the client still has it issued, issue_caps revokes Dc/Dr; once the bit is gone or the cap removed, the cap's reference is dropped. Triggers: the client losing Dc/Dr (cap ack, embedded release, stale with a revocation in flight, via eval_lock_caches), cap removal (remove_client_cap), subtree freeze for export, dirfrag freeze for fragmentation, inode freeze (cross-rank rename), any state change of a cached lock that must gather (simple_sync, simple_excl, simple_lock, simple_xlock, scatter_mix, file_excl, file_xsyn, replica LOCK/SYNC/MIX handlers), a quiescelock xlock, and indirectly any loss of Fx. When the last reference goes, put_lock_cache re-enables frozen inodes and queues C_MDL_DropCache, which drops the locks, auth pins and pins.

- **Class:** POSIX `internal` · exposure `internal` · relied on by kclient, internal
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.2.5 Invalidation triggers](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#325-invalidation-triggers) · [leases-lock-caches-and-async-dirops.md › 3.2.6 Lifecycle](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#326-lifecycle)
- **Code:** `src/mds/Locker.cc:1024 (Locker::invalidate_lock_cache)`, `src/mds/Locker.cc:1058 (Locker::eval_lock_caches)`, `src/mds/Locker.cc:998 (Locker::put_lock_cache)`, `src/mds/CDir.cc:3359-3368 (CDir::freeze_tree)`, `src/mds/Locker.cc:5655 (Locker::local_xlock_start)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS. An invalidating cache stays attachable until the client acks the revoke and blocks creation of a new cache for that opcode until freed (G9). A cache invalidated while its client is stale keeps its cap reference, locks and auth pins until the cap is removed (G18, code reading). Lock-cache wrlocks can stall EXCL -> SYNC of the directory filelock (G8).

<a id="f-caps-051"></a>

#### F-CAPS-051 · Forced auth pins for lock-cache requests

*Requests attached to a lock cache may auth-pin objects inside a freezing directory to avoid deadlocking with the client's held Dc/Dr references.*

In acquire_locks, if an object cannot be auth-pinned (freezing) but its parent dirfrag belongs to the attached cache's directory, the request auth-pins it anyway, because the client holds Dc/Dr references for in-flight async operations until they are answered and waiting on the freeze would deadlock. should_defer_client_cap_frozen likewise does not defer cap messages for a freezing inode without auth pins whose dirfrag is auth-pinned by a lock cache.

- **Class:** POSIX `internal` · exposure `internal` · relied on by kclient, internal
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.2.4 Attachment and effect on request locking](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#324-attachment-and-effect-on-request-locking) · [acquiring-locks.md › 3.3 acquire_locks step by step](../cpp-map/05-locking/acquiring-locks.md#33-acquire_locks-step-by-step)
- **Code:** `src/mds/Locker.cc:448-462 (Locker::acquire_locks)`, `src/mds/Locker.cc:3363-3383 (Locker::should_defer_client_cap_frozen)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS. Freezes (export, fragmentation, rename) wait for such requests (G10).

<a id="f-caps-052"></a>

#### F-CAPS-052 · Forcing async-operation flush for readers

*Another client that reads a directory with an active lock cache first forces the async client to finish its in-flight operations.*

When a request rdlocks a dentry whose parent filelock is cached by a lock cache not attached to it, path_traverse adds a wrlock on that parent filelock before the dentry rdlock. For another client the wrlock cannot be granted in EXCL, so wrlock_start drives the lock out of EXCL, revoking Fx (and with it Dc/Dr) from the async client; the kclient acks only after its in-flight async operations are answered, so the reader observes their results.

- **Class:** POSIX `supporting` · exposure `internal` · relied on by kclient, internal
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.2.4 Attachment and effect on request locking](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#324-attachment-and-effect-on-request-locking) · [leases-lock-caches-and-async-dirops.md › 3.8 Same-directory concurrency](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#38-same-directory-concurrency)
- **Code:** `src/mds/MDCache.cc:8657-8662 (MDCache::path_traverse)`, `src/mds/MDCache.cc:8780-8785 (MDCache::path_traverse)`, `src/mds/Locker.cc:1981 (Locker::wrlock_start)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS.

<a id="f-caps-053"></a>

#### F-CAPS-053 · Per-session inode-number preallocation

*The per-session preallocated inode-number set from which delegated ranges are carved.*

See F-STORE-071 (preallocation and refill) and F-STORE-070 (allocation in create paths). Delegation-specific: each session holds prealloc_inos (durable in session_info_t, journaled in EMetaBlob), free_prealloc_inos, delegated_inos (memory only) and pending_prealloc_inos; delegated_inos is the part handed to the client (F-CAPS-054) and does not count toward the refill threshold (free + pending < mds_client_prealloc_inos / 2). Session::take_ino(head.ino) removes a client-named number from delegated_inos or free_prealloc_inos and asserts if it is in prealloc_inos but in neither set.

- **Class:** POSIX `internal` · exposure `on-disk`, `config` · relied on by internal, kclient, libcephfs
- **Gates:** mds_client_prealloc_inos (default 1000)
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.3.2 Allocation and refill (Server::prepare_new_inode, s...](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#332-allocation-and-refill-serverprepare_new_inode-srcmdsservercc3637-serverprepare_new_inode) · [leases-lock-caches-and-async-dirops.md › 3.3.1 Per-session state](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#331-per-session-state) · [mds-tables.md › 3.2.3 Allocation in the request path](../cpp-map/07-storage-formats/mds-tables.md#323-allocation-in-the-request-path)
- **Code:** `src/mds/Server.cc:3637 (Server::prepare_new_inode)`, `src/mds/SessionMap.h:179 (Session::take_ino)`, `src/mds/Server.cc:3767 (Server::journal_allocated_inos)`, `src/mds/Server.cc:3780 (Server::apply_allocated_inos)`, `src/mds/Server.cc:2315-2318 (Server::early_reply)`
- **Notes:** After replay validate_sessions asserts prealloc_inos == free_prealloc_inos and that none is free in the InoTable (rank marked damaged otherwise). Prealloc inos are not used while a session is opening (force-opened during rejoin), so such creates cannot early-reply (G14). InoTable and SessionMap storage: F-STORE-069, F-STORE-075.

<a id="f-caps-054"></a>

#### F-CAPS-054 · Inode-number delegation to clients

*The MDS hands a slice of a session's preallocated inode numbers to the client so it can name async-created files itself.*

Server::set_reply_extra_bl, for create replies when mds_allow_async_dirops is set and the session has CEPHFS_FEATURE_DELEG_INO, encodes openc_response_t; if mds_client_delegate_inos_pct != 0 and the request is not queued for replay, with frac = 100 / pct (integer division) it tops delegated_inos up to mds_client_prealloc_inos / frac (500 by default) from free_prealloc_inos whenever fewer than half of that remain (250), and the reply carries only the newly delegated intervals. Delegation is a memory-only move; delegations never survive an MDS restart or a client reconnect (the kernel destroys s_delegated_inos when it reconnects). On a session's first create there is nothing to delegate yet (refill pending).

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient
- **Gates:** CEPHFS_FEATURE_DELEG_INO (bit 13, same as CEPHFS_FEATURE_OCTOPUS); mds_allow_async_dirops (default true); mds_client_delegate_inos_pct (default 50); mds_client_prealloc_inos (default 1000)
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.3.3 Delegation (Server::set_reply_extra_bl, src/mds/Ser...](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#333-delegation-serverset_reply_extra_bl-srcmdsservercc2591-serverset_reply_extra_bl) · [sessions.md › 3.14 Preallocated and delegated inode numbers](../cpp-map/02-client-protocol/sessions.md#314-preallocated-and-delegated-inode-numbers)
- **Code:** `src/mds/Server.cc:2591-2605 (Server::set_reply_extra_bl)`, `src/mds/SessionMap.h:197 (Session::delegate_inos)`, `linux/fs/ceph/mds_client.c:622-705 (ceph_parse_deleg_inos)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS. The MDS still delegates inos on create replies and purges them at session close although no client can use them (F-STORE-072). Kernel: ranges starting below 0x1000 are ignored; more than CEPH_MAX_DELEG_INOS (8192) per reply or session is -EIO; duplicates warn; 32-bit kernels ignore delegations. libcephfs logs and discards delegated_inos (src/client/Client.cc:2158-2171). mds_client_delegate_inos_pct values 51-100 delegate the whole prealloc target and values above 100 divide by zero on the next create reply (no upper bound; code reading, G16); the option is flagged runtime but not tracked, so a runtime change is ignored until the rank is re-created.

<a id="f-caps-057"></a>

#### F-CAPS-057 · Asynchronous create (kclient)

*Kernel clients return from open(O_CREAT) before the MDS has processed the create, using Fx|Dc on the directory and a delegated inode number.*

Kernel side (nowsync, the default mount option in this kernel): ceph_atomic_open goes async if the parent has an auth cap with Fx|Dc issued, the session has delegated inos, i_cached_layout is valid (filled only after a synchronous create whose reply carried the created ino while Dc was held), the local MDS-auth-caps check passes and the dentry is known absent. It sends CEPH_MDS_OP_CREATE with CEPH_MDS_FLAG_ASYNC (4), O_EXCL added, head.ino = delegated ino, no layout and a parent release (drop Fs|Ax|Xx unless Fx), and returns at once with a provisional inode (caps CEPH_CAP_ALL_FILE, cap_id 1, max_size stripe_unit, truncate_seq 1) that may write object 0 before the MDS processes the create. The MDS does not branch on the flag: handle_client_openc runs the normal create (lock cache, delegated ino, EXCL locks, [0, stripe_unit] range, EUpdate openc, early reply). The first reply runs ceph_async_create_cb; errors (EEXIST, ENOSPC, EACCES/EPERM, EROFS, ESTALE, EINVAL, ENAMETOOLONG) shut the inode down and set a mapping error on the directory.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient
- **Gates:** kclient mount option nowsync (default) / wsync; Dc on the directory cap (lock cache)
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.4 Async create, end to end](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#34-async-create-end-to-end) · [leases-lock-caches-and-async-dirops.md › 3.4.2 Client preconditions (kclient)](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#342-client-preconditions-kclient) · [kclient-expectations.md › 3.13 Async dirops and delegated inodes](../cpp-map/02-client-protocol/kclient-expectations.md#313-async-dirops-and-delegated-inodes)
- **Code:** `linux/fs/ceph/file.c:525 (try_prep_async_create)`, `linux/fs/ceph/file.c:616 (ceph_async_create_cb)`, `linux/fs/ceph/file.c:672-790 (ceph_finish_async_create)`, `src/mds/Server.cc:4895 (Server::handle_client_openc)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS. The kernel falls back to a synchronous CREATE at linux/fs/ceph/file.c:545. Other facts: -EJUKEBOX at first submission makes the kernel restore the ino and retry synchronously; after a forward to a rank that is not active, or without an open session, the async op is dropped silently (G19, code reading). The MDS never returns -EDQUOT; the kernel checks the max-files quota before sending.

<a id="f-caps-058"></a>

#### F-CAPS-058 · Asynchronous unlink (kclient)

*Kernel clients return from unlink() before the MDS has processed it, using Fx|Dr on the directory.*

Kernel side: ceph_unlink goes async (never for rmdir) when Fx|Dr is issued on the parent, the dentry's lease_shared_gen equals the parent's i_shared_gen, the dentry has CEPH_DENTRY_PRIMARY_LINK and the local MAY_WRITE check passes. It sends CEPH_MDS_OP_UNLINK with CEPH_MDS_FLAG_ASYNC dropping Ls|Lx (plus unwanted caps if nlink == 1) on the victim, decrements nlink and d_deletes locally, and serializes later create/mknod/mkdir/symlink/link/rename of the same name until the first reply (ceph_wait_on_conflict_unlink). The MDS runs the normal handle_client_unlink (lock cache attach, stray dentry, xlocks, EUpdate unlink_local, early reply); create_lock_cache is only called for primary links. Errors set mapping errors on the parent and the inode and drop the dentry.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient
- **Gates:** kclient mount option nowsync (default) / wsync; Dr on the directory cap (lock cache)
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.5 Async unlink, end to end](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#35-async-unlink-end-to-end)
- **Code:** `linux/fs/ceph/dir.c:1309 (get_caps_for_async_unlink)`, `linux/fs/ceph/dir.c:1256 (ceph_async_unlink_cb)`, `src/mds/Server.cc:8572 (Server::handle_client_unlink)`, `src/mds/Server.cc:8738 (Server::_unlink_local)`
- **Notes:** Status of the lock-cache path (finding recorded in _staging/findings-lock-cache.json, confirmed by a 3-investigator panel; evidence level: code reading of the MDS, the Linux 7.3-rc6 kernel client and libcephfs, cross-checked with git history; no test covers it and it was not observed at runtime): since commit 9b4a93f2b7b (2024-03-05; in squid v19.1.0 and later, v20.x and main, not in reef v18.2.x) Locker::create_lock_cache returns unless the client's directory cap already has Dc/Dr issued (src/mds/Locker.cc:1124-1128), while Dc/Dr are allowed on a directory cap only from Capability::lock_cache_allowed, which only create_lock_cache sets (src/mds/Locker.cc:1187, :2563-2567). No other path seeds the bits for unmodified clients (the kernel clears CEPH_CAP_ANY_DIR_OPS in its reconnect record; libcephfs reports only granted bits). So the C++ MDS never creates a lock cache, never grants Dc/Dr, and kernel async create/unlink never happen: those operations always go synchronously to the MDS. The kernel falls back to a synchronous UNLINK at linux/fs/ceph/dir.c:1316. Independently, the PRIMARY_LINK rule (F-CAPS-040) would also block it for clients holding Fs/Fx on the directory (G2, code reading).

<a id="f-caps-059"></a>

#### F-CAPS-059 · Replay of unsafe and async creates after failover

*Creates that a client saw acknowledged (or sent async) are replayed after an MDS restart and recreate the same inode numbers.*

During up:reconnect, Server::dispatch queues requests with CEPH_MDS_FLAG_REPLAY or CEPH_MDS_FLAG_ASYNC for clientreplay (requests of non-open sessions are dropped) and registers a non-zero head.ino of a not-completed request with add_replay_ino_alloc so rejoin does not try to open it. Completed REPLAY requests are answered from completed_requests with result 0 and the created ino in extra_bl (no trace); a retried completed CREATE is re-dispatched as an open of the created inode. A not-completed replayed or async create re-executes and recreates the same ino from the session's prealloc set (all free after restart), reattaching the reconnected cap (try_reconnect_cap). REPLAY requests never early-reply and get no trace; requests queued for replay get no new caps beyond a reconnected one and no new delegations. With mds_replay_unsafe_with_closed_session such requests of sessions that did not reconnect are still replayed; their head.ino is moved from free to delegated so it is released rather than purged and reallocated from the InoTable.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_replay_unsafe_with_closed_session (default false, startup)
- **Map:** [leases-lock-caches-and-async-dirops.md › 3.6 MDS failure, reconnect and replay](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#36-mds-failure-reconnect-and-replay) · [request-lifecycle.md › 3.18 Clientreplay of client requests](../cpp-map/03-request-path/request-lifecycle.md#318-clientreplay-of-client-requests)
- **Code:** `src/mds/Server.cc:340-375 (Server::dispatch)`, `src/mds/Server.cc:2659-2690 (Server::handle_client_request)`, `src/mds/MDCache.cc:5912 (MDCache::try_reconnect_cap)`, `src/mds/Server.cc:2325-2328 (Server::early_reply)`, `linux/fs/ceph/mds_client.c:4730-4760 (replay_unsafe_requests)`
- **Notes:** Completed-request tables and clientreplay are owned by 04-namespace-and-posix-operations and 01-cluster-membership-and-ha; this entry covers the client-named inode number part. The kernel resends unsafe requests with REPLAY and head.ino set and embedded releases stripped, then older unanswered requests (async ones keep ASYNC); libcephfs sets head.ino on replay too. See also F-CLU-095, F-CLU-084.

### Lock types and state machines

The per-object metadata locks, their state tables, the gather engine that completes transitions and the MDS-to-MDS lock protocol. Clients never see lock states; they see caps, leases, reply ordering and latency.

<a id="f-caps-061"></a>

#### F-CAPS-061 · Per-object lock catalogue

*The fixed set of metadata locks on every inode and dentry and the metadata slice each protects.*

Each dentry has lock (DN 1, simplelock: name linkage, leases) and versionlock (DVERSION 2, local). Each inode has quiescelock (IQUIESCE 16, local), snaplock (ISNAP 32: snaprealm), policylock (IPOLICY 64: layout, quota, export pins, flags, optmetadata), filelock (IFILE 128, filelock table: size, times, layout, client_ranges, inline data, dirstat, F caps), nestlock (INEST 256, scatter: rstat), dirfragtreelock (IDFT 512, scatter: fragment tree), authlock (IAUTH 1024: mode, uid, gid, fscrypt_auth; A caps), linklock (ILINK 2048: nlink; L caps), xattrlock (IXATTR 4096: xattrs; X caps), flocklock (IFLOCK 8192: fcntl/flock tables) and versionlock (IVERSION 16384, local). The numeric value is the per-object acquisition order. Each lock's protected fields are carried to replicas in encode_lock_state payloads. Lock states are never journaled or stored in RADOS.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by internal, mds-ranks
- **Legacy parts** (dead-code): The dead machinery listed in the notes: the sm_t careful column with SimpleLock::gcaps_careful and CInode::get_caps_careful (no caller), SimpleLock::req_rdlock_from_auth, ScatterLock::can_scatter_pin, SimpleLock::do_import and Locker::eval_scatter_gathers (no callers), the LOCK_AC_REQUNSCATTER handler (no sender), the LOCK_AC_FOR_REPLICA/LOCK_AC_FOR_AUTH macros (unused), MLock::reqid (never set), the LOCK_REMOTEXLOCK row (never assigned), LOCK_LOCK_MIX (declared only), the TSYN states and file_recover's replica LOCK message (commented out). None of the listed items has a caller, sender or assignment at this commit, so no peer can reach them. scatter_tempsync aborts on entry and its only call site in _rdlock_kick is commented out, so no lock reaches a TSYN state. LOCK_LOCK_MIX still shifts the numbers of all later lock states, which cross ranks only in C++ MDS-to-MDS encodings. Evidence: `src/mds/locks.h:26 (sm_t::careful)`, `src/mds/SimpleLock.h:535 (SimpleLock::gcaps_careful)`, `src/mds/CInode.cc:3772 (CInode::get_caps_careful)`, `src/mds/SimpleLock.h:314 (SimpleLock::req_rdlock_from_auth)`, `src/mds/ScatterLock.h:45 (ScatterLock::can_scatter_pin)`, `src/mds/SimpleLock.h:564 (SimpleLock::do_import)`, `src/mds/Locker.cc:1693 (Locker::eval_scatter_gathers)`, `src/mds/locks.h:118 (LOCK_AC_REQUNSCATTER)`, `src/mds/Locker.cc:6229 (Locker::handle_file_lock)`, `src/mds/locks.h:122-123 (LOCK_AC_FOR_REPLICA, LOCK_AC_FOR_AUTH)`, `src/messages/MLock.h:78 (MLock::set_reqid)`, `src/mds/locks.c:35 (simplelock LOCK_REMOTEXLOCK row)`, `src/mds/locks.h:74 (LOCK_LOCK_MIX)`, `src/mds/Locker.cc:1737-1740 (Locker::_rdlock_kick)`, `src/mds/Locker.cc:5551 (Locker::scatter_tempsync)`, `src/mds/Locker.cc:6017-6024 (Locker::file_recover)`. Also: search: grep -rn over src/mds and src/messages finds only definitions, table rows or name strings for get_caps_careful, req_rdlock_from_auth, can_scatter_pin, do_import, eval_scatter_gathers, LOCK_AC_FOR_*, set_reqid, LOCK_REMOTEXLOCK and LOCK_LOCK_MIX, and no sender of LOCK_AC_REQUNSCATTER.
- **Map:** [lock-types-and-state-machines.md › 3.2 Lock types per object](../cpp-map/05-locking/lock-types-and-state-machines.md#32-lock-types-per-object) · [lock-types-and-state-machines.md › 4.1 Objects](../cpp-map/05-locking/lock-types-and-state-machines.md#41-objects)
- **Code:** `src/include/ceph_fs.h:330-356 (CEPH_LOCK_DN)`, `src/mds/SimpleLock.h:46 (LockType::LockType)`, `src/mds/CInode.h:1153-1163 (CInode)`, `src/mds/CDentry.h:384-385 (CDentry)`, `src/mds/CInode.cc:2315 (CInode::encode_lock_state)`
- **Notes:** Granularity: filelock lumps size, mtime, atime, layout, client ranges, inline data, dirstat and all file caps, a source of false conflicts (lock-types page section 6). Dead or unimplemented machinery (staging lock-states:34): sm_t.careful/gcaps_careful/get_caps_careful, SimpleLock::req_rdlock_from_auth, ScatterLock::can_scatter_pin, SimpleLock::do_import, eval_scatter_gathers, the sender of LOCK_AC_REQUNSCATTER, LOCK_AC_FOR_*, MLock::reqid, LOCK_REMOTEXLOCK, LOCK_LOCK_MIX (declared only, yet it shifts every later state number), the TSYN family and file_recover's replica message. doc/dev/mds_internals/locking.rst is incomplete and partly wrong (no IQUIESCE, wrong LOCK semantics).

<a id="f-caps-062"></a>

#### F-CAPS-062 · SimpleLock state machine (sm_simplelock)

*Shared-read / exclusive-write state machine used by most inode locks and the dentry lock.*

Used by DN, IAUTH, ILINK, IXATTR, ISNAP, IFLOCK and IPOLICY. Stable states: SYNC (rdlock on auth and replicas, leases, s caps, replicas issue s), LOCK (no rdlock or wrlock; waypoint to an xlock), EXCL (the loner gets sx; the loner's requests may wrlock). Transitional: SYNC_LOCK, LOCK_SYNC, EXCL_SYNC, EXCL_LOCK, SYNC_EXCL, LOCK_EXCL, SNAP_SYNC; xlock path LOCK -> LOCK_XLOCK -> PREXLOCK -> XLOCK -> XLOCKDONE, returning to SYNC. Primitives simple_sync, simple_lock, simple_excl, simple_xlock. Replicas only hold SYNC, SYNC_LOCK and LOCK. It is the only class that can be xlocked remotely. allowed_ever_auth s|x, allowed_ever_replica s.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, mds-ranks
- **Legacy parts** (dead-code): The LOCK_REMOTEXLOCK row of the simplelock table, which no code assigns. grep over src/mds finds LOCK_REMOTEXLOCK only in the enum, the simplelock table row and the state-name switch, so no lock enters that state. A remote xlock leaves the replica in LOCK. The rest of sm_simplelock is live. Evidence: `src/mds/locks.h:67 (LOCK_REMOTEXLOCK)`, `src/mds/locks.c:35 (simplelock)`, `src/mds/SimpleLock.h:105 (SimpleLock::get_state_name)`.
- **Map:** [lock-types-and-state-machines.md › 3.4.1 sm_simplelock (src/mds/locks.c:15-45): DN, IAUTH, I...](../cpp-map/05-locking/lock-types-and-state-machines.md#341-sm_simplelock-srcmdslocksc15-45-dn-iauth-ilink-ixattr-isnap-iflock-ipolicy) · [lock-types-and-state-machines.md › 3.9 State diagrams](../cpp-map/05-locking/lock-types-and-state-machines.md#39-state-diagrams) · [lock-types-and-state-machines.md › 3.8 Transition primitives](../cpp-map/05-locking/lock-types-and-state-machines.md#38-transition-primitives)
- **Code:** `src/mds/locks.c:15-45 (simplelock)`, `src/mds/Locker.cc:4922 (Locker::simple_sync)`, `src/mds/Locker.cc:5075 (Locker::simple_lock)`, `src/mds/Locker.cc:5018 (Locker::simple_excl)`, `src/mds/Locker.cc:5163 (Locker::simple_xlock)`
- **Notes:** The LOCK_REMOTEXLOCK row is never assigned (a remote xlock leaves the replica in LOCK). _finish_xlock also puts flocklock (no caps) into EXCL when the xlocker is the target loner; simple_eval then syncs it back.

<a id="f-caps-063"></a>

#### F-CAPS-063 · ScatterLock state machine (nestlock, dirfragtreelock)

*State machine for metadata that several ranks update concurrently and the inode auth periodically sums.*

Stable: SYNC (readable everywhere, unwritable), LOCK (auth-only wrlock, unreadable), MIX (wrlock on every rank, unreadable). Transitions SYNC_LOCK, LOCK_SYNC, SYNC_MIX/SYNC_MIX2, MIX_SYNC, MIX_LOCK/MIX_LOCK2 (two-stage: replicas are messaged only after the local stage), SNAP_SYNC; LOCK -> MIX happens directly without a gather. scatter_eval keeps a replicated nestlock in MIX and an unreplicated one in LOCK ("keep INEST writable at all times"); dirfragtreelock goes to SYNC unless the inode has subtree-root or exporting dirfrags. No caps are ever issued for these locks. The TSYN (temp-sync) family is unimplemented (scatter_tempsync aborts).

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, mds-ranks
- **Legacy parts** (dead-code): The TSYN (temp-sync) family: the TSYN_LOCK, TSYN, LOCK_TSYN, MIX_TSYN and TSYN_MIX rows of the scatterlock table, Locker::scatter_tempsync, and the TSYN branches in simple_sync, simple_lock, scatter_mix, eval_gather and handle_file_lock. scatter_tempsync aborts before it sets any TSYN state, and its only call site in _rdlock_kick is commented out, so no lock reaches a TSYN state. The TSYN branches elsewhere test states that never occur. The SYNC, LOCK and MIX machinery of the entry is live. Evidence: `src/mds/locks.c:64-71 (scatterlock)`, `src/mds/Locker.cc:1737-1740 (Locker::_rdlock_kick)`, `src/mds/Locker.cc:5544-5551 (Locker::scatter_tempsync)`, `src/mds/Locker.cc:5557-5590 (Locker::scatter_tempsync)`, `src/mds/Locker.cc:4934 (Locker::simple_sync)`, `src/mds/Locker.cc:5095 (Locker::simple_lock)`, `src/mds/Locker.cc:5839 (Locker::scatter_mix)`, `src/mds/Locker.cc:1415 (Locker::eval_gather)`, `src/mds/Locker.cc:6153-6160 (Locker::handle_file_lock)`.
- **Map:** [lock-types-and-state-machines.md › 3.4.2 sm_scatterlock (src/mds/locks.c:53-82): INEST, IDFT](../cpp-map/05-locking/lock-types-and-state-machines.md#342-sm_scatterlock-srcmdslocksc53-82-inest-idft) · [lock-types-and-state-machines.md › 3.9 State diagrams](../cpp-map/05-locking/lock-types-and-state-machines.md#39-state-diagrams)
- **Code:** `src/mds/locks.c:53-82 (scatterlock)`, `src/mds/Locker.cc:5323 (Locker::scatter_eval)`, `src/mds/Locker.cc:5805 (Locker::scatter_mix)`, `src/mds/Locker.cc:5544 (Locker::scatter_tempsync)`
- **Notes:** Undefined rows (code reading): replicas of nestlock/dirfragtreelock are put into MIX_SYNC2, which has no row in scatterlock[] (all-zero: stable with no permissions) until the auth's AC_SYNC arrives; the table allows xlock in LOCK but has no XLOCK rows, reachable only through the `lock path` admin command, which then reaches ceph_abort() (lock-types page section 8 item 1).

<a id="f-caps-064"></a>

#### F-CAPS-064 · FileLock state machine (sm_filelock)

*State machine of filelock, combining scatter states with loner, xlock, snapshot and recovery states; it decides all F caps.*

Stable: SYNC (everyone Fscrl, replicas Fscr), LOCK (auth wrlock only; clients keep Fcb), MIX (wrlock on every rank; clients Frwl, replicas Fr), EXCL (loner Fsxcrwb; nobody else gets file caps), XSYN (loner keeps Fcb; the auth may rdlock). Transitional *_SYNC, *_LOCK, *_MIX, *_EXCL, *_XSYN states; xlock states (an XLOCK returns to LOCK, unlike simplelock); XLOCKSNAP; PRE_SCAN/SCAN (size recovery). On directories it also scatter-gathers dirstat. Primitives file_excl, scatter_mix, file_xsyn, file_recover, simple_sync, simple_lock, simple_xlock. allowed_ever_auth is all eight bits, allowed_ever_replica s|c|r|l.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, mds-ranks
- **Map:** [lock-types-and-state-machines.md › 3.4.3 sm_filelock (src/mds/locks.c:84-146): IFILE](../cpp-map/05-locking/lock-types-and-state-machines.md#343-sm_filelock-srcmdslocksc84-146-ifile) · [lock-types-and-state-machines.md › 3.4.5 Stable states at a glance](../cpp-map/05-locking/lock-types-and-state-machines.md#345-stable-states-at-a-glance) · [lock-types-and-state-machines.md › 3.9 State diagrams](../cpp-map/05-locking/lock-types-and-state-machines.md#39-state-diagrams)
- **Code:** `src/mds/locks.c:84-146 (filelock)`, `src/mds/Locker.cc:5900 (Locker::file_excl)`, `src/mds/Locker.cc:5805 (Locker::scatter_mix)`, `src/mds/Locker.cc:6040 (Locker::handle_file_lock)`
- **Notes:** scatter_mix from LOCK does not gather: it sets MIX immediately, so clients may still hold Fc/Fb (allowed in LOCK, not in MIX) while the revocation is in flight (lock-types page section 8 item 26). Fa is never issued; Fl only in the SYNC/MIX family.

<a id="f-caps-065"></a>

#### F-CAPS-065 · XSYN: rdlock while a loner keeps buffered data

*filelock state that lets the auth read a file's metadata while its exclusive writer keeps its page cache and buffers.*

When a request on the auth rdlocks a regular file's filelock that is in EXCL with a target loner (not anonymous, not a directory), _rdlock_kick calls file_xsyn (EXCL -> EXCL_XSYN -> XSYN) instead of simple_sync: the loner keeps Fcb and loses Fs/Fx/Fr/Fw, the auth may rdlock, and the loner's requests may still wrlock. file_eval later leaves XSYN (file_excl accepts XSYN as a source).

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [lock-types-and-state-machines.md › 3.14 Special-purpose states](../cpp-map/05-locking/lock-types-and-state-machines.md#314-special-purpose-states) · [lock-types-and-state-machines.md › 3.13 How lock state drives cap issuance and visibility](../cpp-map/05-locking/lock-types-and-state-machines.md#313-how-lock-state-drives-cap-issuance-and-visibility)
- **Code:** `src/mds/Locker.cc:1742-1748 (Locker::_rdlock_kick)`, `src/mds/Locker.cc:5966 (Locker::file_xsyn)`
- **Notes:** Example: another client stats a file that the loner is writing, on the auth rank (lock-types page section 3.13).

<a id="f-caps-066"></a>

#### F-CAPS-066 · Local locks (versionlock, quiescelock)

*Rank-local locks that are never replicated, used for projected-version pipelining and the quiesce barrier.*

LocalLockC is always in LOCK and is never replicated, carried in MLock or evaluated. wrlocks are shared; an xlock excludes wrlocks; a queued xlocker blocks new wrlockers (can_wrlock = not xlocked and no WAIT_XLOCK waiter). rdlock is never used (asserted). local_xlock_start invalidates lock caches holding the lock; local_xlock_finish wakes waiters and, for quiescelock, re-evaluates the inode's cap locks with caps_imported so blocked requests run before caps are re-issued. versionlock's last_wrlock_client decides projected-value visibility in replies.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [lock-types-and-state-machines.md › 3.4.4 sm_locallock (src/mds/locks.c:149-160): IVERSION, D...](../cpp-map/05-locking/lock-types-and-state-machines.md#344-sm_locallock-srcmdslocksc149-160-iversion-dversion-iquiesce) · [lock-types-and-state-machines.md › 3.10 Lock requests that drive transitions; xlock lifecycl...](../cpp-map/05-locking/lock-types-and-state-machines.md#310-lock-requests-that-drive-transitions-xlock-lifecycle-local-locks)
- **Code:** `src/mds/LocalLockC.h:22-64 (LocalLockC)`, `src/mds/locks.c:149-160 (locallock)`, `src/mds/Locker.cc:5606-5689 (Locker::local_wrlock_grab)`, `src/mds/CInode.cc:4052 (CInode::encode_inodestat)`
- **Notes:** See also F-QOS-045 (quiescelock use).

<a id="f-caps-067"></a>

#### F-CAPS-067 · Gather engine (eval_gather) and cap revocation gather

*The completion test for lock transitions: a lock stays in its transitional state until every conflicting holder, client cap, lease and replica has let go.*

A transition primitive sets the transitional state, revokes conflicting caps (issued_caps_need_gather compares issued bits with the transitional state's columns), revokes leases, messages replicas and auth-pins the parent. eval_gather completes the transition when: no replica ack is outstanding; each held rdlock/wrlock/xlock/lease is permitted (ANY, or AUTH on the auth) by the next state; no scatter writebehind is in flight; no issued (not just pending) loner/other/xlocker cap bit exceeds the next state's columns; the state is not SYNC_MIX2/MIX_SYNC2; and (filelock) no file recovery is pending. Replicas then send LOCKACK/MIXACK/SYNCACK; the auth runs the two-stage MIX_LOCK, a writebehind, or start_scatter + AC_MIX / AC_SYNC with data; finally it sets the next state, auth-unpins, may drop the loner, wakes WAIT_STABLE|WAIT_WR|WAIT_RD|WAIT_XLOCK waiters, calls try_eval and issues caps. Triggers: the last rdlock/wrlock/xlock released, cap ack/flush/release, snapflush completion, stale forced revoke, last lease released or expired, replica acks, recovery completion.

- **Class:** POSIX `supporting` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, ganesha, internal
- **Map:** [lock-types-and-state-machines.md › 3.6 The gather engine: Locker::eval_gather](../cpp-map/05-locking/lock-types-and-state-machines.md#36-the-gather-engine-lockereval_gather) · [capabilities.md › 3.5 Revocation and gather](../cpp-map/02-client-protocol/capabilities.md#35-revocation-and-gather)
- **Code:** `src/mds/Locker.cc:1271-1501 (Locker::eval_gather)`, `src/mds/CInode.cc:3912 (CInode::issued_caps_need_gather)`, `src/mds/Locker.cc:1668 (Locker::eval_cap_gather)`
- **Notes:** Client acks are on the critical path of every conflicting operation, and there is no lock-level timeout. Transitional columns define what clients keep during a transition (e.g. filelock EXCL_SYNC keeps Fscr for the loner while Fx|Fw|Fb are revoked).

<a id="f-caps-068"></a>

#### F-CAPS-068 · Lock-mode grant rules (rdlock / wrlock / xlock compatibility)

*Which lock modes can be held together, derived from the permission columns of the current state.*

There is no static compatibility matrix: a mode is grantable when the current state's column allows it: ANY (auth or replica), AUTH (auth only), XCL (only the client recorded as xlocker; for wrlock and force-wrlock also the loner excl_client), REQ (never here; a replica asks the auth), 0 (never). Consequences: readers need SYNC (or XSYN/TSYN on the auth), writers need LOCK, MIX or loner states, so rdlocks and wrlocks of different clients never coexist except filelock XSYN; wrlocks are shared counters; there is one active xlocker; after an early reply (XLOCKDONE) the xlocking client may rdlock, lease, read projected values and start the next xlock (except on filelock, tracker 49132); in EXCL the loner's requests may wrlock without a transition.

- **Class:** POSIX `supporting` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.5 Compatibility of rdlock, wrlock and xlock](../cpp-map/05-locking/acquiring-locks.md#35-compatibility-of-rdlock-wrlock-and-xlock) · [lock-types-and-state-machines.md › 3.3 The state-table schema](../cpp-map/05-locking/lock-types-and-state-machines.md#33-the-state-table-schema)
- **Code:** `src/mds/locks.h:30-34 (ANY)`, `src/mds/SimpleLock.h:353-389 (SimpleLock::can_rdlock)`, `src/mds/SimpleLock.cc:33-41 (SimpleLock::get_xlock)`
- **Notes:** POSIX atomicity of namespace operations (rename, link, create) relies on these exclusions together with the lock sets each handler declares.

<a id="f-caps-069"></a>

#### F-CAPS-069 · Lock-request-driven state changes and journal nudging

*How rdlock, wrlock and xlock requests kick lock transitions, and why waiting on a lock flushes the journal.*

rdlock_start: if not grantable, _rdlock_kick on a stable auth lock (simple_sync, or file_xsyn for a buffering loner) or LOCK_AC_REQRDLOCK from a replica; waits WAIT_RD (auth, stable) or WAIT_STABLE; snapshot reads use the anonymous client. wrlock_start: grants if can_wrlock and, when the inode has subtree-root/exporting dirfrags or scatter_wanted, the state is MIX; otherwise scatter_mix or simple_lock on the auth, LOCK_AC_REQSCATTER from a replica; waits WAIT_STABLE. xlock_start: on the auth moves LOCK (or the same client's XLOCKDONE) to LOCK_XLOCK via simple_xlock, other stable states first to LOCK; grants when this request started the xlock and can_xlock holds; replicas of simple locks request a remote xlock. Every wait calls nudge_log, which flushes the MDLog when the lock is auth, unstable, locked and has waiters, because holders of early-replied mutations release only at journal commit; simple_sync with wrlocks and the replica MLock handlers flush too. Requests on a recovering filelock prioritize its file recovery.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [lock-types-and-state-machines.md › 3.10 Lock requests that drive transitions; xlock lifecycl...](../cpp-map/05-locking/lock-types-and-state-machines.md#310-lock-requests-that-drive-transitions-xlock-lifecycle-local-locks) · [acquiring-locks.md › 3.4 Per-mode start and finish functions](../cpp-map/05-locking/acquiring-locks.md#34-per-mode-start-and-finish-functions) · [lock-types-and-state-machines.md › 6. Concurrency and mds_lock](../cpp-map/05-locking/lock-types-and-state-machines.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/Locker.cc:1731-1853 (Locker::rdlock_start)`, `src/mds/Locker.cc:1981-2043 (Locker::wrlock_start)`, `src/mds/Locker.cc:2137-2233 (Locker::xlock_start)`, `src/mds/Locker.cc:1855 (Locker::nudge_log)`

<a id="f-caps-070"></a>

#### F-CAPS-070 · Scatter data exchange and writebehind

*Lock-side protocol that collects per-dirfrag deltas from all ranks into the inode and journals the result.*

For filelock on directories (dirstat), nestlock (rstat) and dirfragtreelock (fragment tree), dirfrag auths accumulate deltas while the lock is MIX and the inode auth sums them on gather. Replicas acking MIX_LOCK or MIX_SYNC send encode_lock_ifile/inest/idft of their auth fragments with a dirty flag and start_flush; the auth marks the lock dirty when data is pending. A dirty unflushed lock makes eval_gather run scatter_writebehind: forced wrlock, project the inode with a new version, finish_scatter_gather_update (sum deltas with version epochs, validate, clamp negatives), FLUSHING, journal EUpdate("scatter_writebehind") and flush; on commit: apply, FLUSHED, unpin PIN_DIRTYSCATTERED, leave the LogSegment dirty list, send LOCK_AC_LOCKFLUSHED to replicas in MIX_LOCK/MIX_LOCK2/MIX_EXCL, drop the wrlock to continue the gather. For dirfragtreelock replicas send their tree and auth frags and the auth forces those frags to leaves. Replicas receiving AC_MIX/AC_SYNC data journal accounted-stat resyncs for their lagging fragments.

- **Class:** POSIX `internal` · exposure `mds-wire`, `on-disk` · relied on by internal, mds-ranks
- **Map:** [lock-types-and-state-machines.md › 3.12 Scatter/gather of dirstat, rstat and dirfragtree](../cpp-map/05-locking/lock-types-and-state-machines.md#312-scattergather-of-dirstat-rstat-and-dirfragtree) · [rstats-quotas-and-subvolumes.md › 3.6 Scatter-gather of dirstat and rstat across dirfrags a...](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#36-scatter-gather-of-dirstat-and-rstat-across-dirfrags-and-ranks)
- **Code:** `src/mds/Locker.cc:5260 (Locker::scatter_writebehind)`, `src/mds/CInode.cc:2624 (CInode::finish_scatter_gather_update)`, `src/mds/CInode.cc:1938 (CInode::encode_lock_ifile)`, `src/mds/CInode.cc:1884-1928 (CInode::decode_lock_idft)`, `src/mds/CInode.cc:2521-2595 (CInode::finish_scatter_update)`
- **Notes:** What dirstat and rstat count, their eager propagation in predirty_journal_parents and their freshness are owned by 08-snapshots-quotas-subvolumes; dirfrag split/merge by 05-metadata-cache-and-dirfrags. Mismatches are logged ("unmatched fragstat", "inconsistent rstat") and repaired by trusting the fragments; they are fatal only with mds_verify_scatter (dev). "Marking stale" is only a log message. See also F-SNAP-037 (what the gather sums), F-CACHE-091 (fragtree propagation).

<a id="f-caps-071"></a>

#### F-CAPS-071 · Scatter nudging

*Periodic and journal-driven forcing of dirty scatterlocks so deferred statistics get summed and journal segments can be trimmed.*

mark_updated_scatterlock appends a dirty lock to Locker::updated_scatterlocks with a timestamp. scatter_tick, from Locker::tick every mds_tick_interval (5 s) on clientreplay/active/stopping ranks (not while the beacon is laggy), nudges locks older than mds_scatter_nudge_interval (5 s) and flushes the journal. scatter_nudge on the auth moves a replicated non-MIX lock to MIX, else a non-LOCK lock to LOCK, else to SYNC (at most two cycles), waiting if frozen or unstable; on a replica it sends LOCK_AC_NUDGE. LogSegment::try_to_expire nudges every inode on the segment's dirty_dirfrag_* lists, so a segment cannot be trimmed until its scattered data is written behind.

- **Class:** POSIX `internal` · exposure `config` · relied on by internal, mds-ranks
- **Gates:** mds_scatter_nudge_interval (5 s); mds_tick_interval (5 s)
- **Map:** [lock-types-and-state-machines.md › 3.12 Scatter/gather of dirstat, rstat and dirfragtree](../cpp-map/05-locking/lock-types-and-state-machines.md#312-scattergather-of-dirstat-rstat-and-dirfragtree) · [rstats-quotas-and-subvolumes.md › 3.5 Lazy propagation: dirty rstat, updated scatterlocks, ...](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#35-lazy-propagation-dirty-rstat-updated-scatterlocks-tick-nudge-journal-expiry)
- **Code:** `src/mds/Locker.cc:5381 (Locker::mark_updated_scatterlock)`, `src/mds/Locker.cc:5404 (Locker::scatter_nudge)`, `src/mds/Locker.cc:5517 (Locker::scatter_tick)`, `src/mds/journal.cc:200-214 (LogSegment::try_to_expire)`
- **Notes:** scatter_tick stops at the first entry younger than the interval and requeued entries keep their old stamp, so the list is not strictly ordered. On frozen or ambiguous-auth inodes without a context scatter_nudge just requeues ("not ideal.. starvation prone.."). See also F-SNAP-036 (deferred stat propagation), F-SNAP-038 (segment expiry waits for stat folding).

<a id="f-caps-072"></a>

#### F-CAPS-072 · Inter-MDS lock protocol (MLock)

*MDS-to-MDS message that keeps replica lock states in step with the auth rank.*

MSG_MDS_LOCK (0x300, v1): asker, action, reqid (never set), lock_type, MDSCacheObjectInfo {ino, dirfrag, dname, snapid}, lockdata. Auth -> replicas: SYNC (-1), MIX (-2), LOCK (-3), LOCKFLUSHED (-4); replica -> auth: SYNCACK (1), MIXACK (2), LOCKACK (3), REQSCATTER (7), REQUNSCATTER (8, never sent), NUDGE (9), REQRDLOCK (10). Only the auth initiates state changes; replicas follow and ack after their local gathers. send_lock_message skips ranks below REJOIN while the cluster is degraded, yet init_gather adds all replicas to the gather set. Messages for objects not in cache are dropped; while the receiver is in rejoin, messages for rejoining objects are dropped and never retransmitted.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Legacy parts** (dead-code): LOCK_AC_REQUNSCATTER (8): handle_file_lock has a case for it, but no code sends it. MLock::reqid: encoded and decoded in every MLock but never set. grep over src/mds finds LOCK_AC_REQUNSCATTER only in locks.h, the action-name switch and the handle_file_lock case, and finds no caller of MLock::set_reqid. MLock is exchanged only between C++ MDS ranks. The other actions are live. Evidence: `src/mds/locks.h:118 (LOCK_AC_REQUNSCATTER)`, `src/mds/SimpleLock.h:177 (SimpleLock::get_lock_action_name)`, `src/mds/Locker.cc:6229-6243 (Locker::handle_file_lock)`, `src/messages/MLock.h:31 (MLock::reqid)`, `src/messages/MLock.h:78 (MLock::set_reqid)`, `src/messages/MLock.h:88 (MLock::decode_payload)`, `src/messages/MLock.h:97 (MLock::encode_payload)`.
- **Map:** [lock-types-and-state-machines.md › 3.11 Inter-MDS lock protocol (MLock)](../cpp-map/05-locking/lock-types-and-state-machines.md#311-inter-mds-lock-protocol-mlock) · [lock-types-and-state-machines.md › 5.1 Messages](../cpp-map/05-locking/lock-types-and-state-machines.md#51-messages)
- **Code:** `src/messages/MLock.h:24-107 (MLock)`, `src/mds/locks.h:107-120 (LOCK_AC_SYNC)`, `src/mds/Locker.cc:4693 (Locker::handle_lock)`, `src/mds/Locker.cc:134 (Locker::send_lock_message)`
- **Notes:** MDS-to-MDS only (ALLOW_MESSAGES_FROM MDS); never seen by clients, monitors or OSDs. handle_simple_lock ignores unknown actions; handle_file_lock aborts on them. A gather including a rank below REJOIN finishes only when that rank rejoins.

<a id="f-caps-073"></a>

#### F-CAPS-073 · Replica-initiated lock state requests

*Replica ranks ask the auth rank to change a lock state so they can rdlock or wrlock locally.*

A replica that cannot rdlock sends LOCK_AC_REQRDLOCK; the auth (if not SYNC and not frozen) runs simple_sync on a stable lock or retries the message once the lock is stable, else drops it (the replica waits on its own WAIT_STABLE). A replica that cannot wrlock sends LOCK_AC_REQSCATTER; the auth runs scatter_mix on a stable lock (even while freezing, since the requester holds an auth pin) or sets scatter_wanted on an unstable one. Replicas send LOCK_AC_NUDGE from scatter_nudge. Requesters hold remote auth pins so the auth cannot freeze meanwhile.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [lock-types-and-state-machines.md › 3.11 Inter-MDS lock protocol (MLock)](../cpp-map/05-locking/lock-types-and-state-machines.md#311-inter-mds-lock-protocol-mlock) · [acquiring-locks.md › 3.9 Multi-MDS locking](../cpp-map/05-locking/acquiring-locks.md#39-multi-mds-locking)
- **Code:** `src/mds/Locker.cc:1731 (Locker::_rdlock_kick)`, `src/mds/Locker.cc:4741-4762 (Locker::handle_reqrdlock)`, `src/mds/Locker.cc:6211-6227 (Locker::handle_file_lock)`
- **Notes:** handle_reqrdlock's unfreeze wait is ineffective: SimpleLock::add_waiter shifts WAIT_UNFREEZE into the lock-waiter half, so only WAIT_STABLE wakes it (lock-types page section 8 item 28). A REQSCATTER that reaches the auth in MIX_SYNC sets scatter_wanted, so the auth scatters again right after reaching SYNC (code reading).

<a id="f-caps-074"></a>

#### F-CAPS-074 · Lock state across replication, migration and rejoin

*Encodings and rules that carry lock states between ranks when objects are replicated, exported, renamed across ranks or recovered.*

New replicas receive each lock's replica_state (bare s16; NEED_RECOVER if created while the auth recovers). Export sends SimpleLock::encode (state + gather_set) for nine inode locks plus loner_cap and the dentry lock, and export_twiddle turns the exporter's copy into replica state; during export the exporter holds lock_scatter_gather (WRLOCK|STATE_PIN) on the exported dirfrag inode's filelock and nestlock; the importer re-marks dirty scatterlocks and re-evaluates snaplock/policylock. In rename inode migration xlock_export converts xlocks to replica LOCK and xlock_import re-pins them on the importer. Rejoin: survivors report scatter states (MIX_SYNC/MIX_LOCK while the recovering rank is in the gather set, NEED_RECOVER for MIX*), the recovering auth infers MIX (or LOCK for non-directory filelocks) and re-creates survivors' xlocks/wrlocks; rdlocks on NEED_RECOVER locks wait for cluster recovery. Replica DN locks mirror the auth (LOCK is not readable, so replica path traversal waits WAIT_RD); the DN payload carries linkage but replicas only adopt first, and linkage changes travel in MDentryLink/MDentryUnlink.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [lock-types-and-state-machines.md › 3.15 Lock state across replication, migration and recovery](../cpp-map/05-locking/lock-types-and-state-machines.md#315-lock-state-across-replication-migration-and-recovery) · [lock-types-and-state-machines.md › 4.1 Objects](../cpp-map/05-locking/lock-types-and-state-machines.md#41-objects) · [resolve-and-rejoin.md › 3.16 Lock-state recovery rules](../cpp-map/09-recovery/resolve-and-rejoin.md#316-lock-state-recovery-rules)
- **Code:** `src/mds/CInode.cc:4540-4599 (CInode::_encode_locks_full)`, `src/mds/SimpleLock.h:551 (SimpleLock::export_twiddle)`, `src/mds/ScatterLock.h:139-175 (ScatterLock::infer_state_from_strong_rejoin)`, `src/mds/Locker.cc:2324-2346 (Locker::xlock_export)`, `src/mds/CDentry.cc:548-588 (CDentry::decode_lock_state)`
- **Notes:** Raw state numbers (implicit locks.h enum) cross ranks only in these encodings. Server::_rename_apply and _commit_peer_rename iterate mdr->locks from &in->versionlock, relying on CInode member layout (acquiring-locks page section 8). Recovery stages are owned by 01-cluster-membership-and-ha and subtree export by 07-multi-mds.

### Lock acquisition

How requests obtain locks: acquire_locks, ordering rules, implicit locks, auth pins, remote locks, retries, and release around the unsafe and safe replies.

<a id="f-caps-075"></a>

#### F-CAPS-075 · acquire_locks and lock vectors

*The main entry point through which requests declare and obtain their locks without ever blocking a thread.*

Callers build a LockOpVec (RDLOCK 1, WRLOCK 2, XLOCK 4, REMOTE_WRLOCK 8 with a target rank, STATE_PIN 16) in the order the locks must be taken across objects. acquire_locks(mdr, lov, auth_pin_freeze, auth_pin_nonblocking, skip_quiesce): pass 1 adds implicit locks (versionlock, quiescelock) and collects objects to pin; sort_and_merge; check pinnability of all objects (dropping everything and waiting on WAIT_UNFREEZE or WAIT_SINGLEAUTH if one is not pinnable, or aborting with auth_pin_nonblocking); take local auth pins; send OP_AUTHPIN to other ranks and return; then acquire each lock in order, skipping held ones. On the first lock that is not grantable it registers a waiter and returns false, keeping the locks already held (partial holds). Held locks live in MutationImpl::locks, a set ordered by lock address.

- **Class:** POSIX `supporting` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.3 acquire_locks step by step](../cpp-map/05-locking/acquiring-locks.md#33-acquire_locks-step-by-step) · [acquiring-locks.md › 3.1 The lock vector and the held-lock set](../cpp-map/05-locking/acquiring-locks.md#31-the-lock-vector-and-the-held-lock-set)
- **Code:** `src/mds/Locker.cc:245-669 (Locker::acquire_locks)`, `src/mds/Mutation.h:74-141 (MutationImpl::LockOp)`, `src/mds/Locker.h:71-75 (Locker::acquire_locks)`
- **Notes:** Every client metadata operation goes through it. Other acquisition paths: try_rdlock_snap_layout, rdlock_try_set, wrlock_try, wrlock_force / local_wrlock_grab and lock caches. A merged RDLOCK|WRLOCK entry loses the rdlock (no current caller). Release iterates in address order and may synchronously run other requests' handlers.

<a id="f-caps-076"></a>

#### F-CAPS-076 · Per-object lock order

*Locks on the same object are always taken in ascending lock-type order.*

sort_and_merge sorts each consecutive run of entries on the same object by CEPH_LOCK_* value (DN < DVERSION; IQUIESCE < ISNAP < IPOLICY < IFILE < INEST < IDFT < IAUTH < ILINK < IXATTR < IFLOCK < IVERSION) and merges adjacent entries for the same lock by OR-ing flags; an XLOCK replaces all other modes. Within one call the implicit quiescelock is therefore first on its inode and versionlock last.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.6 Lock ordering and deadlock avoidance](../cpp-map/05-locking/acquiring-locks.md#36-lock-ordering-and-deadlock-avoidance) · [acquiring-locks.md › 3.3 acquire_locks step by step](../cpp-map/05-locking/acquiring-locks.md#33-acquire_locks-step-by-step)
- **Code:** `src/mds/Mutation.cc:132-177 (MutationImpl::LockOpVec::sort_and_merge)`, `src/include/ceph_fs.h:330-348 (CEPH_LOCK_DN)`
- **Notes:** Entries for one object that are not adjacent in the caller's vector are neither sorted together nor merged (a convention, not checked).

<a id="f-caps-077"></a>

#### F-CAPS-077 · Cross-object ordering rules and no upgrades

*Deadlock-avoidance rules for locks on different objects.*

O1 caller order across objects; O3 path components top-down (parent directory locks, dentry, child snaplock), one acquire_locks per step; O4 two-path operations order directory inodes and dentries: objects under the fs root before those under ~mdsN, then by depth, then by directory ino, dentries in one directory by name, non-directory inodes by ino; O5 target-inode locks after the path, stray locks last; O7 no upgrades (a request never xlocks a lock it rdlocked in an earlier call; code that needs a read-only check first drops its locks); O8 a later call must not add a lock that sorts before one already held on the same object, except the implicit quiescelock, which is safe because its failure drops everything. Violations show up as deadlocks, not errors; e.g. getattr restarts the request if its filelock decision changes between retries.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.6 Lock ordering and deadlock avoidance](../cpp-map/05-locking/acquiring-locks.md#36-lock-ordering-and-deadlock-avoidance)
- **Code:** `src/mds/MDCache.cc:8425-8441 (MDCache::path_traverse)`, `src/mds/Server.cc:4072-4221 (Server::rdlock_two_paths_xlock_destdn)`, `src/mds/Mutation.cc:470-481 (MDRequestImpl::compare_paths)`, `src/mds/Server.cc:6512-6529 (Server::handle_client_setvxattr)`
- **Notes:** Deadlock freedom of rename and link depends on these rules (commit 246f647566 introduced the two-path ordering). doc/dev/mds_internals/locking.rst ends with "TODO: Explain locking order and path traversal locking."

<a id="f-caps-078"></a>

#### F-CAPS-078 · Ancestor snaplock/policylock try-rdlocks and layout inheritance

*Bottom-up, try-only read locks on every ancestor's snapshot and policy locks, which also supply the inherited file layout.*

try_rdlock_snap_layout walks from the request's base inode to the root rdlocking each snaplock (stopping at one already held) and, with want_layout, each policylock until an ancestor whose projected inode has a layout, copying that layout into mdr->dir_layout for new files. It takes locks only when immediately grantable, without kicking and without auth pins (so it works on replicas and does not block freezes); on the first failure it drops all locks and local auth pins and waits WAIT_RD. It records dir_root and dir_depth used for two-path ordering. Ancestor snap/policy rdlocks are held until the safe reply (they survive the early reply), so mksnap, setlayout and quota changes anywhere above wait for unsafe requests below.

- **Class:** POSIX `supporting` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.7 Path locking helpers](../cpp-map/05-locking/acquiring-locks.md#37-path-locking-helpers) · [acquiring-locks.md › 3.6 Lock ordering and deadlock avoidance](../cpp-map/05-locking/acquiring-locks.md#36-lock-ordering-and-deadlock-avoidance)
- **Code:** `src/mds/Locker.cc:157-227 (Locker::try_rdlock_snap_layout)`, `src/mds/MDCache.cc:8497-8502 (MDCache::path_traverse)`
- **Notes:** It inserts directly into mdr->locks without updating last_locked. Operations that xlock snaplock/policylock take the xlock on their target first and then try-rdlock the parent ancestry.

<a id="f-caps-079"></a>

#### F-CAPS-079 · Path traversal locking and path helpers

*Locking flags of MDCache::path_traverse and the Server helpers that lock a path for an operation.*

path_traverse takes per-component locks according to flags: RDLOCK_SNAP/SNAP2 (ancestor try-rdlocks, child snaplocks), RDLOCK_PATH (rdlock each dentry; also wrlock the parent filelock when another client's lock cache holds it), XLOCK_DENTRY (tail: wrlock parent filelock + nestlock, optional RDLOCK_AUTHLOCK, xlock the dentry), CHECK_LOCKCACHE, WANT_DIRLAYOUT, PATH_LOCKED (peers do not wait on unreadable dentries). Waits inside traversal drop all locks; without RDLOCK_PATH an unreadable dentry makes the traversal wait WAIT_RD (counter traverse_lock). Helpers: rdlock_path_pin_ref (inode ops; auth-pins the target and refuses a freezing target it has not pinned), rdlock_path_xlock_dentry (create/unlink style; EEXIST/ENOENT checks), rdlock_two_paths_xlock_destdn (rename and link: both paths depth 1, ordered per F-CAPS-077, remote wrlocks for a source dirfrag on another rank, auth_pin_freeze for a non-auth primary source), xlock_policylock (xlock policylock (+snaplock) then try-rdlock the parent ancestry). PATH_LOCKED and ALL_LOCKED make retries idempotent.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.7 Path locking helpers](../cpp-map/05-locking/acquiring-locks.md#37-path-locking-helpers) · [acquiring-locks.md › 3.14 Worked examples](../cpp-map/05-locking/acquiring-locks.md#314-worked-examples)
- **Code:** `src/mds/MDCache.cc:8443-8891 (MDCache::path_traverse)`, `src/mds/Server.cc:3879-3946 (Server::rdlock_path_pin_ref)`, `src/mds/Server.cc:3960-4057 (Server::rdlock_path_xlock_dentry)`, `src/mds/Server.cc:5930-5954 (Server::xlock_policylock)`
- **Notes:** Path resolution, forwarding and discover are owned by 05-metadata-cache-and-dirfrags; per-operation lock sets are listed in the acquiring-locks worked examples. See also F-NS-005 (request-side helpers), F-CACHE-041 (path_traverse).

<a id="f-caps-080"></a>

#### F-CAPS-080 · Implicit versionlock (pipelining and peer rollback)

*versionlock is added automatically next to every xlock, shared on the leader and exclusive on a peer.*

For an xlock on an auth dentry lock the leader adds a wrlock on the dentry's versionlock and a peer adds an xlock; for an xlock on an inode lock other than IVERSION/IQUIESCE on an auth inode the leader adds a versionlock wrlock and a peer an xlock. Leader wrlocks are shared and let several projected updates of one object pipeline into the journal; a peer's xlock blocks every other journaled update of that object until the cross-rank operation ends ("this makes rollback safe"). setattr that sets ctime also wrlocks versionlock; predirty_journal_parents grabs ancestor versionlocks with local_wrlock_grab.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, mds-ranks
- **Map:** [acquiring-locks.md › 3.3 acquire_locks step by step](../cpp-map/05-locking/acquiring-locks.md#33-acquire_locks-step-by-step) · [lock-types-and-state-machines.md › 3.2 Lock types per object](../cpp-map/05-locking/lock-types-and-state-machines.md#32-lock-types-per-object)
- **Code:** `src/mds/Locker.cc:318-357 (Locker::acquire_locks)`

<a id="f-caps-081"></a>

#### F-CAPS-081 · Implicit quiescelock wrlock

*Lock-acquisition side of the implicit quiescelock wrlock.*

See F-QOS-045. Lock-acquisition specifics: the implicit wrlock is added in Locker::acquire_locks right after the first xlock, wrlock or remote wrlock on a cap-bearing inode lock (authlock, linklock, filelock, xattrlock) of the call, at most one per call; when the quiescelock is unavailable, handle_quiesce_failure drops all locks and local auth pins, sends OP_FINISH to all peers and waits, so no request waits on a quiesced inode while holding path locks. Fragmentation does not pass skip_quiesce.

- **Class:** POSIX `extension` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, admin
- **Map:** [acquiring-locks.md › 3.11 quiescelock interplay](../cpp-map/05-locking/acquiring-locks.md#311-quiescelock-interplay) · [quiesce.md › 3.13 The quiescelock and how it blocks mutations](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#313-the-quiescelock-and-how-it-blocks-mutations)
- **Code:** `src/mds/Locker.cc:338-387 (Locker::acquire_locks)`, `src/mds/Locker.cc:679-685 (Locker::handle_quiesce_failure)`, `src/mds/LocalLockC.h:33-39 (LocalLockC::can_wrlock)`
- **Notes:** Only one quiescelock is added per call: a two-directory rename gates only the first-ordered parent directory and one of srci/oldin (whether quiesce relies on more is undocumented; unverified). Lock caches hold the parent directory's quiescelock wrlock on behalf of async dirops. Policy vxattrs take no quiescelock, but their policylock xlock waits on quiesce_inode's policylock rdlock. See also F-CAPS-066 (local locks).

<a id="f-caps-082"></a>

#### F-CAPS-082 · Local auth pins on locked objects

*Every lock taken through acquire_locks on an auth object also pins the object against freezing (export, fragmentation, rename migration).*

Every non-local lock taken through acquire_locks on an auth object, including plain rdlocks, requires an auth pin; on a replica the object is (remotely) pinned only when the lock may have to be requested from the auth. Pinnability of all objects of a call is checked before any lock of that call: an inode is pinnable if auth, not freezing/frozen and its dirfrag is pinnable; a dirfrag if auth and not in a freezing/frozen dir or tree. Failure drops everything and waits WAIT_UNFREEZE (events such as "failed to authpin, subtree is being exported"). Local-lock wrlocks and try_rdlock_snap_layout rdlocks take no pin; unstable locks hold their own auth pin, so freezes also wait for pending lock transitions.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.8 Frozen and freezing objects](../cpp-map/05-locking/acquiring-locks.md#38-frozen-and-freezing-objects) · [acquiring-locks.md › 3.3 acquire_locks step by step](../cpp-map/05-locking/acquiring-locks.md#33-acquire_locks-step-by-step)
- **Code:** `src/mds/Locker.cc:411-509 (Locker::acquire_locks)`, `src/mds/CInode.cc:3094-3111 (CInode::can_auth_pin)`, `src/mds/CDir.cc:3528-3555 (CDir::can_auth_pin)`, `src/mds/CDentry.cc:459-463 (CDentry::can_auth_pin)`
- **Notes:** See also F-CACHE-028 (auth pin accounting).

<a id="f-caps-083"></a>

#### F-CAPS-083 · Remote auth pins and the rename frozen auth pin

*Pinning objects whose authority is another rank, including the special freeze of a renamed inode.*

For objects whose auth is another rank the leader sends MMDSPeerRequest OP_AUTHPIN (3) with the complete set of objects at that rank, an optional authpin_freeze (carried in the object_info field), and FLAG_NONBLOCKING or FLAG_NOTIFYBLOCKING. The peer fails with FLAG_EROFS on a read-only fs or FLAG_WOULDBLOCK for nonblocking requests, or waits for unfreeze (sending FLAG_REQBLOCKED if asked, so the leader drops its locks and keeps waiting), then pins everything and replies OP_AUTHPINACK (-3) with its full pinned list; EROFS/WOULDBLOCK become -EROFS/-EAGAIN for the leader. For a rename whose source dentry is not auth and is a primary link, the source inode's auth freeze_auth_pins it (auth pin, freeze_inode(1), STATE_FROZENAUTHPIN) so nobody can auth-pin it between lock acquisition and rename prep (ABBA deadlock). Several OP_AUTHPIN requests (one per rank) may be outstanding.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [acquiring-locks.md › 3.9 Multi-MDS locking](../cpp-map/05-locking/acquiring-locks.md#39-multi-mds-locking) · [acquiring-locks.md › 3.8 Frozen and freezing objects](../cpp-map/05-locking/acquiring-locks.md#38-frozen-and-freezing-objects)
- **Code:** `src/mds/Locker.cc:512-558 (Locker::acquire_locks)`, `src/mds/Server.cc:3297-3421 (Server::handle_peer_auth_pin)`, `src/mds/Server.cc:3423-3486 (Server::handle_peer_auth_pin_ack)`, `src/mds/Mutation.cc:341-353 (MDRequestImpl::freeze_auth_pin)`
- **Notes:** A peer lacking the object or not its auth replies with an empty pin list and no error; the leader recomputes authority. handle_peer_auth_pin_ack is the only place that clears remote_auth_pinned records, so after a quiesce failure stale pin records survive (acquiring-locks page section 8; impact on the rename frozen pin unverified).

<a id="f-caps-084"></a>

#### F-CAPS-084 · Remote xlocks and remote wrlocks

*Taking an exclusive or shared write lock on another rank's authoritative copy of an object.*

A leader xlocks a simple lock (DN, IAUTH, ILINK, IXATTR, ISNAP, IFLOCK, IPOLICY) of a non-auth object with OP_XLOCK (1); the auth runs acquire_locks(skip_quiesce) on its peer request (so the object's versionlock is xlocked), stays in XLOCK (peers never early-reply) and answers OP_XLOCKACK (-1) carrying encode_locked_state; the leader's replica stays in LOCK and records the xlock; OP_UNXLOCK (2) releases it. Cross-rank rename and link remote-wrlock the source directory's filelock/nestlock at the source dirfrag's auth with OP_WRLOCK (8) / OP_WRLOCKACK (-8) / OP_UNWRLOCK (9); when a local wrlock on the same lock is also needed it is taken with wrlock_try, and on failure the remote wrlock is released and re-requested so the gather can finish. Remote locks are obtained one round trip at a time, each followed by a full re-dispatch; a non-auth OP_XLOCK is dropped without reply.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [acquiring-locks.md › 3.9 Multi-MDS locking](../cpp-map/05-locking/acquiring-locks.md#39-multi-mds-locking) · [lock-types-and-state-machines.md › 3.10 Lock requests that drive transitions; xlock lifecycl...](../cpp-map/05-locking/lock-types-and-state-machines.md#310-lock-requests-that-drive-transitions-xlock-lifecycle-local-locks)
- **Code:** `src/mds/Locker.cc:2198-2232 (Locker::xlock_start)`, `src/mds/Locker.cc:2086-2131 (Locker::remote_wrlock_start)`, `src/mds/Server.cc:3187-3248 (Server::dispatch_peer_request)`, `src/mds/Server.cc:3109-3146 (Server::handle_peer_request_reply)`, `src/mds/Locker.cc:605-615 (Locker::acquire_locks)`
- **Notes:** set_xlocks_done asserts every xlocked object is auth; this holds because remote xlocks are only taken by link/unlink/rename flows whose journaled peers suppress the early reply (inferred, not asserted). MMDSPeerRequest is MDS-to-MDS only (HEAD_VERSION = COMPAT_VERSION = 1).

<a id="f-caps-085"></a>

#### F-CAPS-085 · Remote lock teardown and peer failure

*Releasing locks held on other ranks and handling a peer rank that fails mid-request.*

_drop_locks sends OP_DROPLOCKS (11) to every rank where remote xlocks or wrlocks were held (if it is at least up:rejoin); the peer drops the locks but keeps its auth pins and peer request. request_drop_remote_locks sends OP_FINISH (17), with FLAG_ABORT when the leader was killed before committing, to every rank in more()->peers, forgets foreign xlocks and remote wrlocks locally and clears peers. When a peer rank fails, non-committing leader requests drop it from waiting_on_peer, forget an in-flight remote lock aimed at it and retry when it is active; peer requests from a failed leader without a prepare are aborted or finished, and prepared ones wait for resolve.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [acquiring-locks.md › 3.9 Multi-MDS locking](../cpp-map/05-locking/acquiring-locks.md#39-multi-mds-locking) · [acquiring-locks.md › 8. Edge cases, failure handling and gotchas](../cpp-map/05-locking/acquiring-locks.md#8-edge-cases-failure-handling-and-gotchas) · [resolve-and-rejoin.md › 3.4.5 In-flight requests when a rank fails (handle_mds_fa...](../cpp-map/09-recovery/resolve-and-rejoin.md#345-in-flight-requests-when-a-rank-fails-handle_mds_failure)
- **Code:** `src/mds/Locker.cc:796-855 (Locker::_drop_locks)`, `src/mds/Locker.cc:687-740 (Locker::request_drop_remote_locks)`, `src/mds/Server.cc:3027-3048 (Server::handle_peer_request)`, `src/mds/MDCache.cc:3006-3131 (MDCache::handle_mds_failure)`
- **Notes:** Resolve of prepared peer updates is owned by 01-cluster-membership-and-ha. A killed request waiting on peers is only marked aborted and finishes when the peer replies.

<a id="f-caps-087"></a>

#### F-CAPS-087 · FIFO lock waiters with synchronous wake-up

*Waiters on a lock are woken in arrival order, inside the call stack that released or stabilized the lock.*

Lock waiter bits (WAIT_RD 1, WAIT_WR 2, WAIT_XLOCK = WAIT_STABLE 4, WAIT_REMOTEXLOCK 8) are shifted into the upper half of each cache object's 128-bit wait mask (bit 64 + a 4-bit slot per lock type) and always added with WAIT_ORDERED, which gives a global increasing sequence; take_waiting returns them in sequence order and finish_contexts completes them synchronously in the caller's stack (e.g. inside eval_gather or another request's _drop_locks); unordered waiters (unfreeze, single auth) get sequence 0 and run first. A woken request that fails again re-queues behind newer waiters.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.2 The re-entrant execution model: no thread ever blocks](../cpp-map/05-locking/acquiring-locks.md#32-the-re-entrant-execution-model-no-thread-ever-blocks) · [lock-types-and-state-machines.md › 4.1 Objects](../cpp-map/05-locking/lock-types-and-state-machines.md#41-objects)
- **Code:** `src/mds/SimpleLock.h:83-89 (SimpleLock)`, `src/mds/SimpleLock.h:230-250 (SimpleLock::add_waiter)`, `src/mds/MDSCacheObject.h:267-280 (MDSCacheObject::add_waiter)`, `src/mds/MDSCacheObject.cc:97-114 (MDSCacheObject::take_waiting)`
- **Notes:** Head-of-line blocking: a transitional state blocks all new incompatible requests, so one xlock waiter stalls later readers even while it waits for an unresponsive client. See also F-CACHE-033 (waiter queues on cache objects).

<a id="f-caps-088"></a>

#### F-CAPS-088 · Non-blocking lock helpers

*Lock primitives that never wait, used where waiting is not allowed or a lock must be held regardless of state.*

rdlock_try (check with one kick, take nothing); rdlock_try_set (take a set of rdlocks, or register WAIT_STABLE|WAIT_RD and drop everything; used by export); wrlock_try (take if grantable, else simple_lock only if stable, auth, clean, not scatter_wanted and without subtree dirfrags, never starting a scatter or writebehind because the caller may have a log entry open); wrlock_force (take regardless of state, for locks held during a journal write: cap updates, max_size, export, fragmentation, rstat propagation, writebehind); local_wrlock_grab (assert and grant).

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.4 Per-mode start and finish functions](../cpp-map/05-locking/acquiring-locks.md#34-per-mode-start-and-finish-functions) · [acquiring-locks.md › 3.13 Opportunistic ancestor locks (rstat propagation)](../cpp-map/05-locking/acquiring-locks.md#313-opportunistic-ancestor-locks-rstat-propagation)
- **Code:** `src/mds/Locker.cc:1775-1789 (Locker::rdlock_try)`, `src/mds/Locker.cc:1888-1924 (Locker::rdlock_try_set)`, `src/mds/Locker.cc:1947-1979 (Locker::wrlock_try)`, `src/mds/Locker.cc:1929-1945 (Locker::wrlock_force)`, `src/mds/Locker.cc:5606 (Locker::local_wrlock_grab)`
- **Notes:** rdlock_try_set with a lock already rdlocked by the request would leak one rdlock reference (no current caller does so).

<a id="f-caps-089"></a>

#### F-CAPS-089 · Freeze versus auth-pin deadlock breaker

*Timeout that cancels an export or fragmentation whose freeze is stuck behind requests holding remote auth pins.*

A request that holds a remote auth pin and must wait on a freezing object calls notify_freeze_waiter, incrementing num_remote_waiters of the fragmentation or of the export rooted at the freezing tree. Each MDSRankDispatcher::tick on an active rank runs find_stale_fragment_freeze and find_stale_export_freeze: if a freeze's cumulative auth-pin count did not change for mds_freeze_tree_timeout and there are remote waiters (or the parent directory is also freezing), the fragmentation or export is cancelled. The peer side does the same when an OP_AUTHPIN must wait.

- **Class:** POSIX `internal` · exposure `config` · relied on by mds-ranks
- **Gates:** mds_freeze_tree_timeout (30 s, dev)
- **Map:** [acquiring-locks.md › 3.8 Frozen and freezing objects](../cpp-map/05-locking/acquiring-locks.md#38-frozen-and-freezing-objects)
- **Code:** `src/mds/Locker.cc:758-779 (Locker::notify_freeze_waiter)`, `src/mds/Migrator.cc:268-317 (Migrator::find_stale_export_freeze)`, `src/mds/MDCache.cc:12169-12215 (MDCache::find_stale_fragment_freeze)`, `src/mds/MDSRank.cc:750-751 (MDSRankDispatcher::tick)`
- **Notes:** Export and fragmentation themselves are owned by 07-multi-mds and 05-metadata-cache-and-dirfrags. notify_freeze_waiter is called from acquire_locks, rdlock_path_pin_ref, repair_dirfrag_stats_work and Server::handle_peer_auth_pin. Cancelled fragment operations are not re-queued. dump_export_states shows last_cum_auth_pins, last_cum_auth_pins_change and num_remote_waiters for DISCOVERING/FREEZING exports. See also F-MULTI-024 (export side), F-CACHE-029 and F-CACHE-030 (freezing).

<a id="f-caps-090"></a>

#### F-CAPS-090 · Lock waits on a degraded cluster

*Locks that wait for cluster recovery while some rank is failed or recovering.*

While the cluster is degraded: an xlock on ISNAP or IPOLICY by a leader not queued for replay drops locks and local pins and waits for cluster recovery if the (auth) object has a replica on a rank below active, or the (non-auth) lock is not yet xlocked ("to guarantee replayed requests and mksnap/setlayout get processed in proper order"); an rdlock on a NEED_RECOVER lock drops locks (keeping local pins) and waits unless the request is a queued replay (when not degraded the flag is simply cleared); remote lock and auth-pin requests wait for the peer to be clientreplay, active or stopping; lock messages to ranks below up:rejoin are skipped.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal, mds-ranks
- **Map:** [acquiring-locks.md › 3.3 acquire_locks step by step](../cpp-map/05-locking/acquiring-locks.md#33-acquire_locks-step-by-step) · [acquiring-locks.md › 8. Edge cases, failure handling and gotchas](../cpp-map/05-locking/acquiring-locks.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Locker.cc:275-308 (Locker::acquire_locks)`, `src/mds/Locker.cc:636-651 (Locker::acquire_locks)`, `src/mds/SimpleLock.h:272-285 (SimpleLock::set_state_rejoin)`
- **Notes:** See also F-CLU-091.

<a id="f-caps-091"></a>

#### F-CAPS-091 · xlock lifecycle

*How an exclusive lock is obtained on the auth, held across the journal write and released.*

xlock_start drives LOCK_XLOCK -> PREXLOCK (gathered) -> XLOCK with get_xlock(mdr, client) (one xlock_by mutation, num_xlock, xlock_by_client). When the request finishes, xlock_finish calls put_xlock; at num_xlock == 0 _finish_xlock either jumps straight to EXCL (lock types other than DN/ISNAP/IPOLICY, with no rdlocks, wrlocks or leases, not XLOCKSNAP, when the xlocker is the inode's target loner or anonymous), skipping a SYNC round trip, or runs eval_gather to the table's next state (SYNC for simple locks, sending LOCK_AC_SYNC with the new value to replicas; LOCK for filelock). cancel_locking handles a request dropping an in-flight xlock (PREXLOCK -> _finish_xlock; LOCK_XLOCK -> XLOCKDONE then eval_gather).

- **Class:** POSIX `supporting` · exposure `internal` · relied on by internal
- **Map:** [lock-types-and-state-machines.md › 3.10 Lock requests that drive transitions; xlock lifecycl...](../cpp-map/05-locking/lock-types-and-state-machines.md#310-lock-requests-that-drive-transitions-xlock-lifecycle-local-locks) · [acquiring-locks.md › 3.4 Per-mode start and finish functions](../cpp-map/05-locking/acquiring-locks.md#34-per-mode-start-and-finish-functions)
- **Code:** `src/mds/Locker.cc:2137-2233 (Locker::xlock_start)`, `src/mds/Locker.cc:2235-2260 (Locker::_finish_xlock)`, `src/mds/Locker.cc:2262-2322 (Locker::xlock_finish)`, `src/mds/SimpleLock.cc:33-65 (SimpleLock::get_xlock)`, `src/mds/Locker.cc:858-876 (Locker::cancel_locking)`

<a id="f-caps-092"></a>

#### F-CAPS-092 · XLOCKDONE: read-your-own-unsafe-writes

*Between the unsafe and the safe reply, the client that made a change may already use it while other clients wait for durability.*

At an early (unsafe) reply set_xlocks_done moves every xlock of the request to XLOCKDONE and clears xlock_by but keeps xlock_by_client (rename keeps its dentry xlocks in LOCK_XLOCK so dentries with projected linkage get no new replicas). In XLOCKDONE the same client may read, rdlock, take a lease, read projected values, hold xlocker_caps (s) and start its next xlock (pipelining; disabled for filelock, tracker 49132, so back-to-back size/mtime setattrs serialize on the journal), while other clients wait until the request is safe. encode_inodestat shows projected inode fields only to the client that xlocks the corresponding lock or is the loner (versionlock: the last wrlocking client), and projected dentry linkage only to the xlocking client or request. Early reply is skipped (one safe reply, journal flushed immediately) when mds_early_reply is false, no_early_reply is set (SETDIRLAYOUT and the ceph.dir.layout, ceph.quota, ceph.quiesce.block, ceph.dir.subvolume, ceph.dir.subvolume.snaps.visible vxattrs), peers journaled, an ino was allocated from the InoTable, or the request came from an MDS or is a replay.

- **Class:** POSIX `supporting` · exposure `client-wire`, `config` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** mds_early_reply (default true)
- **Map:** [acquiring-locks.md › 3.10 Early reply and lock release timeline](../cpp-map/05-locking/acquiring-locks.md#310-early-reply-and-lock-release-timeline) · [lock-types-and-state-machines.md › 3.13 How lock state drives cap issuance and visibility](../cpp-map/05-locking/lock-types-and-state-machines.md#313-how-lock-state-drives-cap-issuance-and-visibility) · [acquiring-locks.md › 7. Externally visible contract](../cpp-map/05-locking/acquiring-locks.md#7-externally-visible-contract)
- **Code:** `src/mds/Locker.cc:781-794 (Locker::set_xlocks_done)`, `src/mds/SimpleLock.cc:43-50 (SimpleLock::set_xlock_done)`, `src/mds/Server.cc:2300-2328 (Server::early_reply)`, `src/mds/CInode.cc:4043-4053 (CInode::encode_inodestat)`, `src/mds/CDentry.h:214 (CDentry::use_projected)`
- **Notes:** The kclient completes the syscall on the first reply and keeps the request until the safe reply. Other clients do not observe another client's non-durable metadata through replies or caps; one exception found is MClientQuota, filled from the projected rstat (lock-types page section 7 item 4). The unsafe/safe reply protocol is owned by 04-namespace-and-posix-operations.

<a id="f-caps-093"></a>

#### F-CAPS-093 · Early-reply rdlock release and journal flush policy

*Which locks an early-replied request gives up before its journal entry commits, and when the journal is flushed.*

After submitting the journal entry of an early-replied request, Server::journal_and_reply calls handle_locks_for_early_reply: it releases every rdlock except snaplock and policylock rdlocks (kept "to make later mksnap/setlayout (at other mds) wait for this unsafe request") and flushes the journal once, only if a held auth lock is unstable, locked and has waiters. Without an early reply the log is flushed immediately; a request replayed after failover only queues the next replay op. Wrlocks and xlocks stay until the request is safe; anyone waiting on them triggers nudge_log.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.10 Early reply and lock release timeline](../cpp-map/05-locking/acquiring-locks.md#310-early-reply-and-lock-release-timeline) · [lock-types-and-state-machines.md › 3.10 Lock requests that drive transitions; xlock lifecycl...](../cpp-map/05-locking/lock-types-and-state-machines.md#310-lock-requests-that-drive-transitions-xlock-lifecycle-local-locks)
- **Code:** `src/mds/Server.cc:2124-2156 (Server::journal_and_reply)`, `src/mds/Locker.cc:906-939 (Locker::handle_locks_for_early_reply)`
- **Notes:** Determines how long other clients wait behind an unsafe request (journal commit latency).

<a id="f-caps-094"></a>

#### F-CAPS-094 · Lock release at safe reply and cleanup

*Order in which a finished request gives up its locks, auth pins and lock-cache reference.*

reply_client_request calls request_drop_non_rdlocks before building the safe reply ("drop non-rdlocks before replying, so that we can issue leases"): OP_FINISH to peers, then all xlocks, wrlocks and remote wrlocks, each release possibly waking and running waiting requests synchronously; the trace with leases and caps is included only if no unsafe reply was sent. request_cleanup then drops the remaining rdlocks and the lock-cache reference, local auth pins, runs kick_cap_releases and drops cache pins; request_forward and request_kill also clean up. drop_lock releases one held lock in whatever mode it is held (quiesce_inode sheds its cap locks once it holds the quiescelock); drop_locks_for_fragment_unfreeze releases everything except dirfragtreelock.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [acquiring-locks.md › 3.10 Early reply and lock release timeline](../cpp-map/05-locking/acquiring-locks.md#310-early-reply-and-lock-release-timeline)
- **Code:** `src/mds/Server.cc:2438-2453 (Server::reply_client_request)`, `src/mds/MDCache.cc:10124-10135 (MDCache::request_cleanup)`, `src/mds/Locker.cc:941-966 (Locker::drop_lock)`, `src/mds/Locker.cc:968-984 (Locker::drop_locks_for_fragment_unfreeze)`

### Observability

Counters, dumps and admin commands that expose caps, leases and locks.

<a id="f-caps-095"></a>

#### F-CAPS-095 · Cap and lease counters and dumps

*Perf counters and admin output describing caps, leases and delegated inodes.*

Perf counters: mds.caps and mds.inodes_with_caps; mds_mem.cap, cap+, cap- (live, added and removed Capability objects); mds.handle_client_caps, handle_client_caps_dirty, handle_client_cap_release, process_request_cap_release, handle_inode_file_caps; mds.ceph_cap_op_grant, ceph_cap_op_revoke, ceph_cap_op_trunc, ceph_cap_op_flush_ack, ceph_cap_op_flushsnap_ack; mds_server.cap_revoke_eviction and cap_acquisition_throttle. No counter is dedicated to leases, lock caches or delegations. Admin: dump inode shows client_caps [{client_id, pending, issued, wanted, last_sent}], loner, want_loner and mds_caps_wanted; session ls / client ls show num_caps, num_leases, num_completed_flushes, recall/release counters, delegated_inos and prealloc_inos.

- **Class:** POSIX `operational` · exposure `metrics`, `admin` · relied on by mgr, admin, tools
- **Map:** [capabilities.md › 5.6 Perf counters and health](../cpp-map/02-client-protocol/capabilities.md#56-perf-counters-and-health) · [capabilities.md › 5.7 Admin commands](../cpp-map/02-client-protocol/capabilities.md#57-admin-commands) · [leases-lock-caches-and-async-dirops.md › 5.7 Admin commands](../cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#57-admin-commands)
- **Code:** `src/mds/MDSRank.cc:3743-3762 (MDSRank::create_logger)`, `src/mds/CInode.cc:5458 (CInode::dump)`, `src/mds/SessionMap.cc:647-689 (Session::dump)`, `src/pybind/mgr/status/module.py:78`
- **Notes:** mds_mem.cap is read by the mgr status module, the dashboard and telemetry. The perf-counter and admin-command frameworks are owned by 11-security-admin-and-observability.

<a id="f-caps-096"></a>

#### F-CAPS-096 · Lock state dumps and lock-related op events

*Admin visibility into lock states and into why a request is waiting.*

dump inode / dump cache print per lock gather_set, the state name (SimpleLock::get_state_name strings), type, is_leased, num_rdlocks, num_wrlocks, num_xlocks and xlock_by (SYNC unlocked locks are dumped empty); debug logs mark leased and cached locks. ops / dump_ops_in_flight / dump_blocked_ops show acquire_locks events ("failed to rdlock, waiting", "failed to xlock, waiting", "failed to authpin, subtree is being exported", "requesting remote authpins", "failed to acquire quiesce lock", "acquired locks"); ops flags=locks dumps each request's held locks {object, object_string, lock, flags, wrlock_target}. The lock path test command that takes arbitrary locks is F-ADM-053.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin, tools
- **Map:** [lock-types-and-state-machines.md › 5.6 Admin commands](../cpp-map/05-locking/lock-types-and-state-machines.md#56-admin-commands) · [acquiring-locks.md › Admin commands](../cpp-map/05-locking/acquiring-locks.md#admin-commands)
- **Code:** `src/mds/SimpleLock.cc:71-90 (SimpleLock::dump)`, `src/mds/CInode.cc:5376-5420 (CInode::dump)`, `src/mds/Mutation.cc:560-576 (MDRequestImpl::_dump)`, `src/mds/MDCache.cc:14497-14604 (MDCache::dispatch_lock_path)`
- **Notes:** There is no lock-state or lock-wait counter (only traverse_lock). QA tests consume the op event strings.

### Legacy entries

These entries serve only clients older than the [baseline](../README.md#baseline-current-peers), msgr1 mounts or older C++ releases, or describe code that cannot run. Each keeps its ID and group.

<a id="f-caps-007"></a>

#### F-CAPS-007 · Cap restrictions for old clients (connection feature bits)

*Withholding file read/write caps or quota pushes from clients whose connection lacks inline-data, pool-namespace or quota support.*

A new Capability records STATE_NOINLINE, STATE_NOPOOLNS and STATE_NOQUOTA when the client connection lacks CEPH_FEATURE_MDS_INLINE_DATA, CEPH_FEATURE_FS_FILE_LAYOUT_V2 or CEPH_FEATURE_MDS_QUOTA. get_allowed_caps then drops Fr|Fw if the inode has inline data and the cap is NOINLINE, or the layout has a pool namespace and the cap is NOPOOLNS; NOQUOTA caps get no quota pushes. CInode::get_caps_allowed_for_client applies the same rule from connection features when no Capability exists yet (reply encoding).

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse
- **Group:** Capability model and allowed caps
- **Legacy** (old-clients): The NOINLINE, NOPOOLNS and NOQUOTA cap states and the reply-time connection check apply only to connections that lack CEPH_FEATURE_MDS_INLINE_DATA (bit 40), CEPH_FEATURE_FS_FILE_LAYOUT_V2 (bit 58) or CEPH_FEATURE_MDS_QUOTA (bit 47). The 7.3 kernel, the CentOS Stream 8 and 9 samples and libcephfs (CEPH_FEATURES_ALL, all three bits since at least 10.2.0) advertise all three bits. The newest upstream kernels that lack one are v4.16 (bit 47 is first advertised in v4.17) and v4.4 (bit 58 is first advertised in v4.5). Evidence: `src/mds/Capability.cc:173-181 (Capability::Capability)`, `src/mds/Locker.cc:2569-2573 (Locker::get_allowed_caps)`, `src/mds/CInode.cc:3832-3838 (CInode::get_caps_allowed_for_client)`, `src/mds/MDCache.cc:2145-2146 (MDCache::broadcast_quota_to_client)`, `linux/include/linux/ceph/ceph_features.h:207 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `linux/include/linux/ceph/ceph_features.h:212 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `linux/include/linux/ceph/ceph_features.h:217 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `cs8-kernel/include/linux/ceph/ceph_features.h:207-217 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `cs9-kernel/include/linux/ceph/ceph_features.h:207-217 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `src/include/ceph_features.h:223 (CEPH_FEATURES_ALL)`, `src/include/ceph_features.h:231 (CEPH_FEATURES_ALL)`, `src/include/ceph_features.h:236 (CEPH_FEATURES_ALL)`, `src/include/ceph_features.h:263 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `v10.2.0:src/include/ceph_features.h:157-181 (CEPH_FEATURES_ALL)`, `linux-v4.13/include/linux/ceph/ceph_features.h:177-211 (CEPH_FEATURES_SUPPORTED_DEFAULT, no MDS_QUOTA)`, `linux-v4.17/include/linux/ceph/ceph_features.h:207 (CEPH_FEATURES_SUPPORTED_DEFAULT)`, `linux-v4.5/include/linux/ceph/ceph_features.h:125 (CEPH_FEATURES_SUPPORTED_DEFAULT)`.
- **Gates:** CEPH_FEATURE_MDS_INLINE_DATA (bit 40); CEPH_FEATURE_FS_FILE_LAYOUT_V2 (bit 58); CEPH_FEATURE_MDS_QUOTA (bit 47)
- **Map:** [capabilities.md › 3.2 The per-(inode, client) Capability object](../cpp-map/02-client-protocol/capabilities.md#32-the-per-inode-client-capability-object) · [capabilities.md › 3.3 From lock states to allowed caps](../cpp-map/02-client-protocol/capabilities.md#33-from-lock-states-to-allowed-caps)
- **Code:** `src/mds/Capability.cc:173-181 (Capability::Capability)`, `src/mds/Locker.cc:2569 (Locker::get_allowed_caps)`, `src/mds/CInode.cc:3798 (CInode::get_caps_allowed_for_client)`

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-caps-012"></a>F-CAPS-012 | [F-PROTO-091](02-client-sessions-and-protocol.md#f-proto-091) · MClientCaps encoding (v14, xattrs in the middle segment) |
| <a id="f-caps-017"></a>F-CAPS-017 | [F-PROTO-102](02-client-sessions-and-protocol.md#f-proto-102) · NFS-Ganesha delegations built on caps |
| <a id="f-caps-033"></a>F-CAPS-033 | [F-NS-034](04-namespace-and-posix-operations.md#f-ns-034) · Readdir cap-acquisition throttle |
| <a id="f-caps-037"></a>F-CAPS-037 | [F-MULTI-020](07-multi-mds.md#f-multi-020) · Non-auth cap migration to the auth rank (MExportCaps, MGatherCaps) |
| <a id="f-caps-041"></a>F-CAPS-041 | [F-PROTO-083](02-client-sessions-and-protocol.md#f-proto-083) · DirStat and LeaseStat records |
| <a id="f-caps-055"></a>F-CAPS-055 | [F-PROTO-087](02-client-sessions-and-protocol.md#f-proto-087) · CREATE reply: created inode and delegated inode ranges |
| <a id="f-caps-056"></a>F-CAPS-056 | [F-NS-041](04-namespace-and-posix-operations.md#f-ns-041) · Client-specified inode number (head.ino) on create |
| <a id="f-caps-060"></a>F-CAPS-060 | [F-STORE-072](06-journal-and-storage.md#f-store-072) · Release of preallocated and delegated numbers at session close (EPurged) |
| <a id="f-caps-086"></a>F-CAPS-086 | [F-NS-004](04-namespace-and-posix-operations.md#f-ns-004) · Re-entrant handler execution (restart from the top) |

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
