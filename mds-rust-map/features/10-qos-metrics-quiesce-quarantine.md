# QoS, metrics, quiesce and quarantine: feature catalog

> Part of the [MDS Rust reference set](../README.md) · [Features index](../features/README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers as in the [baseline](../README.md#baseline-current-peers). Map pages: [qos-dmclock-scheduler.md](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md) · [metrics-and-subvolume-metrics.md](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md) · [quiesce.md](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md) · [quarantine.md](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md)

## 1. Scope

This catalog covers four optional CephFS MDS facilities documented in the map directory cpp-map/12-qos-metrics-quiesce-quarantine: the dmClock QoS scheduler for client metadata requests (qos-dmclock-scheduler.md), client and subvolume metrics with rank-0 aggregation and the ceph-mgr perf query interface (metrics-and-subvolume-metrics.md), quiesce of subvolume trees through a replicated quiesce database (quiesce.md), and subvolume quarantine (quarantine.md).

For each facility it lists the client-visible signals (feature bits 14, 22 and 24, metric_spec, InodeStat and MClientCaps fields, MClientMetrics, MQuarantineDisable, cap masks, EACCES and request parking), the cluster contracts (MDSMap quiesce leader fields and compat bit 12, MMgrConfigure/MMgrReport perf queries, labeled counters), the MDS-to-MDS messages (MMDSMetrics, MMDSPing, MMDSQuiesceDbListing/Ack, policylock replication of the quarantine flag), and the admin commands and options. QoS and metrics keep no persistent state; quiesce persists only the ceph.quiesce.block inode flag; quarantine persists one optmetadata entry on the subvolume root.

Not covered here, and owned elsewhere: client feature bits and session handling in general (02-client-sessions-and-protocol), lock types, lock acquisition and caps in general (03-caps-leases-and-locking), vxattrs and charmap (04-namespace-and-posix-operations), fragmentation (05-metadata-cache-and-dirfrags), journaling (06-journal-and-storage), subtree migration (07-multi-mds), subvolume and quota semantics (08-snapshots-quotas-subvolumes), and MDS auth caps including the q/Q flags, counter dump, perf counters, the volumes module (fs quiesce, fs subvolume quarantine), the stats module and cephfs-top, and the lock path and quiesce path test commands (11-security-admin-and-observability). Entries that touch those mechanisms describe only the part specific to this category and link to the owning map page.

Cross-category review (2026-10-11): entries that duplicated an entry of the owning category were retired (listed under "retired" in features.json with the entry they were merged into) or shortened to pointers whose description starts with "See F-XXX-NNN." and keeps only the facts specific to this category.

## 2. Summary

72 features: 72 used by current peers (2 of them with legacy parts) and 0 legacy entries, listed last. Columns: POSIX class, exposure and the peers that rely on the feature. [Section 4](#4-legend) defines them and the legacy reasons.

**dmClock QoS scheduler**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-001](#f-qos-001) | QoS admission gate for client metadata requests | operational | config, admin | kclient, libcephfs, ceph-fuse, ganesha, admin |
| [F-QOS-002](#f-qos-002) | QoS bucket classification from the session mount root | operational | client-wire, admin | kclient, libcephfs, ceph-fuse, ganesha, mgr, cephfs-mirror |
| [F-QOS-003](#f-qos-003) | QoS per-op cost table | operational | config | admin |
| [F-QOS-004](#f-qos-004) | dmClock tag-based scheduling per bucket | operational | internal | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-005](#f-qos-005) | QoS scheduler threads and hand-off to request processing | operational | metrics | admin, internal |
| [F-QOS-006](#f-qos-006) | Default QoS parameters (mds_dmclock_* options) | operational | config | mon, admin |
| [F-QOS-007](#f-qos-007) | Per-bucket QoS overrides (qos set, qos rm) | operational | admin | admin |
| [F-QOS-008](#f-qos-008) | QoS inspection commands and per-bucket statistics | operational | admin | admin |
| [F-QOS-009](#f-qos-009) | QoS bucket state lifecycle tied to sessions | internal | internal | admin, internal |
| [F-QOS-010](#f-qos-010) | QoS enable, disable and drain on rank state and option changes | operational | config | admin, internal |

**Client metrics protocol**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-011](#f-qos-011) | Client metrics negotiation (feature bit 14 and metric_spec) *(legacy parts)* | operational | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-012](#f-qos-012) | MClientMetrics ingestion of per-client metrics (types 0..9) *(legacy parts)* | operational | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-013](#f-qos-013) | Tolerant decoding of unknown metric types | supporting | client-wire | kclient, libcephfs |
| [F-QOS-014](#f-qos-014) | Metrics accepted on any active rank (client send rules) | operational | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-015](#f-qos-015) | Subvolume id in InodeStat and cap flush acks | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-016](#f-qos-016) | Subvolume I/O metrics (type 16) ingestion and path resolution | operational | client-wire, metrics | kclient, libcephfs, ceph-fuse, ganesha, mgr, admin |
| [F-QOS-017](#f-qos-017) | Feature bit 22 enables kernel subvolume metrics | operational | client-wire | kclient |

**Cross-rank metrics aggregation**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-018](#f-qos-018) | Per-rank metrics forwarding to rank 0 (MMDSMetrics) | internal | mds-wire, config | mds-ranks |
| [F-QOS-019](#f-qos-019) | Rank-0 ping sequence and delayed-rank detection | internal | mds-wire, config | mds-ranks, mgr |
| [F-QOS-020](#f-qos-020) | Client session lifecycle in metrics (REFRESH and REMOVE) | operational | mds-wire, metrics | mds-ranks, tools, admin |
| [F-QOS-021](#f-qos-021) | Metrics state culling on mdsmap changes | internal | metrics | mgr, internal |
| [F-QOS-022](#f-qos-022) | Rank telemetry: CPU usage and open requests (mds_rank_perf) | operational | metrics, mds-wire | tools, admin, mds-ranks |
| [F-QOS-023](#f-qos-023) | Subvolume quota_bytes and used_bytes reporting | operational | metrics | tools, admin, mgr |

**Metric counters and mgr reporting**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-024](#f-qos-024) | Per-client labeled metric counters (mds_client_metrics) | operational | metrics | tools, admin |
| [F-QOS-025](#f-qos-025) | Subvolume sliding-window metrics (mds_subvolume_metrics) | operational | metrics, mds-wire, config | tools, admin, mgr, mds-ranks |
| [F-QOS-026](#f-qos-026) | Eviction of idle subvolume metrics | operational | metrics | tools, admin |
| [F-QOS-027](#f-qos-027) | Labeled metric counter naming and export | operational | metrics, admin | tools, admin, mgr |
| [F-QOS-028](#f-qos-028) | MDS perf query configuration from ceph-mgr | operational | cluster-wire | mgr |
| [F-QOS-029](#f-qos-029) | MDS perf query key construction and counter pairs | operational | cluster-wire | mgr |
| [F-QOS-030](#f-qos-030) | MDS perf report to ceph-mgr (MMgrReport) | operational | cluster-wire | mgr |

**Quiesce database and coordination**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-032](#f-qos-032) | Quiesce DB leader and membership from the MDSMap | operational | cluster-wire | mon, mgr, mds-ranks |
| [F-QOS-033](#f-qos-033) | MDS compat incompat feature 12 (quiesce subvolumes) | operational | cluster-wire | mon |
| [F-QOS-034](#f-qos-034) | Quiesce sets and request operations | extension | admin | mgr, admin |
| [F-QOS-035](#f-qos-035) | Quiesce set and member state machine | extension | admin | mgr, admin |
| [F-QOS-036](#f-qos-036) | Quiesce timeout and expiration | extension | admin | mgr, admin |
| [F-QOS-037](#f-qos-037) | Quiesce await semantics | extension | admin | mgr, admin |
| [F-QOS-038](#f-qos-038) | Quiesce if_version optimistic concurrency | extension | admin | mgr, admin |
| [F-QOS-039](#f-qos-039) | Quiesce root URI sanitization | extension | admin | mgr, admin |
| [F-QOS-040](#f-qos-040) | Quiesce DB replication between ranks | internal | mds-wire | mds-ranks |
| [F-QOS-041](#f-qos-041) | Per-rank quiesce agent | internal | internal | mds-ranks |
| [F-QOS-042](#f-qos-042) | Quiesce on degraded file systems and non-active ranks | operational | admin | mgr, admin |

**Quiescing the inode tree**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-043](#f-qos-043) | Tree quiesce request (quiesce_path) | internal | internal | internal |
| [F-QOS-044](#f-qos-044) | Per-inode quiesce request (quiesce_inode) | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-045](#f-qos-045) | quiescelock and parking of mutations | extension | internal | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-046](#f-qos-046) | Cap masking while quiesced | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-047](#f-qos-047) | Quiescing inodes that appear during a quiesce | internal | internal | internal |
| [F-QOS-048](#f-qos-048) | Quiesce and directory fragmentation | internal | internal | internal |
| [F-QOS-049](#f-qos-049) | Quiesce and subtree migration | internal | internal | mds-ranks |
| [F-QOS-050](#f-qos-050) | Quiesce release and cancel cascade | internal | internal | internal |
| [F-QOS-051](#f-qos-051) | Quiesce dispatch throttle and heartbeat resets | operational | config | admin |
| [F-QOS-052](#f-qos-052) | ceph.quiesce.block vxattr | extension | client-wire, on-disk | kclient, libcephfs, ceph-fuse, admin |

**Quiesce admin interfaces**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-053](#f-qos-053) | quiesce db command | extension | admin | mgr, admin, tools |
| [F-QOS-057](#f-qos-057) | Quiesce observability: counters and op events | operational | metrics, admin | admin, tools |

**Quarantine control**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-058](#f-qos-058) | quarantine enable and disable commands | extension | admin | mgr, admin, tools |
| [F-QOS-060](#f-qos-060) | Persistent quarantine flag in inode optmetadata | extension | on-disk | mds-ranks, internal |
| [F-QOS-061](#f-qos-061) | In-progress quarantine state and operation ordering | internal | internal | internal |
| [F-QOS-062](#f-qos-062) | Per-subvolume quarantine operation registry | internal | admin | admin, internal |
| [F-QOS-063](#f-qos-063) | Quarantine undo on failure | internal | internal | internal |
| [F-QOS-064](#f-qos-064) | Quarantine cap-revocation walk | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-065](#f-qos-065) | Quarantine propagation to replica ranks | internal | mds-wire | mds-ranks |
| [F-QOS-066](#f-qos-066) | Subtree export refused inside a quarantined subvolume | internal | internal | mds-ranks, admin |
| [F-QOS-067](#f-qos-067) | Quarantine re-enforcement after restart | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |

**Quarantine enforcement**

| ID | Feature | POSIX | Exposure | Relied on by |
|---|---|---|---|---|
| [F-QOS-068](#f-qos-068) | Quarantine enforcement predicate | internal | internal | internal |
| [F-QOS-069](#f-qos-069) | MDS auth cap flags q and Q (quarantine access) | extension | cluster-wire, admin | mon, libcephfs, ceph-fuse, ganesha, admin |
| [F-QOS-070](#f-qos-070) | Quarantine client feature bit 24 | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha, mon |
| [F-QOS-071](#f-qos-071) | EACCES for quarantine-aware sessions | extension | client-wire | libcephfs, ceph-fuse, ganesha, mgr |
| [F-QOS-072](#f-qos-072) | Request parking for quarantine-unaware sessions | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-073](#f-qos-073) | Snapshot operation parking under quarantine | extension | client-wire | kclient |
| [F-QOS-074](#f-qos-074) | Cap restriction to PIN under quarantine | extension | client-wire | kclient, libcephfs, ceph-fuse, ganesha |
| [F-QOS-075](#f-qos-075) | EQUARANTINED errno in MClientCaps | extension | client-wire | libcephfs, ceph-fuse, ganesha |
| [F-QOS-076](#f-qos-076) | QUARANTINE marker in InodeStat | extension | client-wire | libcephfs, ceph-fuse |
| [F-QOS-077](#f-qos-077) | MQuarantineDisable notification | extension | client-wire | libcephfs, ceph-fuse, ganesha |

## 3. Features

### dmClock QoS scheduler

The optional admission stage that delays client metadata requests per QoS bucket on each rank. It adds latency only: no message, field, feature bit or error code exists for it, and it is off by default.

<a id="f-qos-001"></a>

#### F-QOS-001 · QoS admission gate for client metadata requests

*Optionally routes each client MClientRequest through a per-bucket dmClock queue before normal request handling, delaying but never failing it.*

Server::dispatch hands every CEPH_MSG_CLIENT_REQUEST that survived the reconnect, clientreplay and not-active handling to MDSDmclockScheduler::handle_client_request. The request is queued only if QoS is enabled, its original source is a client, the rank is up:active, a VolumeInfo exists for the session's volume with its updated flag set, and the effective reservation, weight and limit are all > 0; otherwise it goes straight to Server::handle_client_request. Only MClientRequest is gated: caps, cap releases, leases, session, reconnect and reclaim messages, client metrics, all MDS-to-MDS traffic and internal ops (scrub, fragment, export, quiesce, quarantine) never pass the gate. MDS-originated client requests (stray reintegration and migration renames) and requests reaching the gate in up:clientreplay or up:stopping bypass it; requests parked with wait_for_active pass through it once the rank is active. The only client-visible effect is added latency: no message, field, feature bit or error code exists for QoS.

- **Class:** POSIX `operational` · exposure `config`, `admin` · relied on by kclient, libcephfs, ceph-fuse, ganesha, admin
- **Gates:** mds_dmclock_enable (bool, default false)
- **Map:** [qos-dmclock-scheduler.md › 3.2 Admission gate](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#32-admission-gate) · [qos-dmclock-scheduler.md › 7. Externally visible contract](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#7-externally-visible-contract)
- **Code:** `src/mds/Server.cc:404-406 (Server::dispatch)`, `src/mds/MDSDmclockScheduler.cc:81-95 (MDSDmclockScheduler::handle_client_request)`, `src/mds/MDSDmclockScheduler.cc:69-79 (MDSDmclockScheduler::enqueue_client_request)`, `src/mds/StrayManager.cc:697-705 (StrayManager::reintegrate_stray)`
- **Notes:** Gated requests can be overtaken by later caps, lease and session messages of the same connection, which are handled immediately; without QoS a connection's messages are handled in arrival order. Whether any client depends on that order was not analysed (unverified). get_volume_id asserts a non-null session, whereas Server::handle_client_request drops a request that has none (code reading). Merged in 2026-07 (commit 19b5342824f); the only test is the QA facet qa/suites/fs/workload/tasks/5-dmclock/yes.yaml, which enables it with defaults.

<a id="f-qos-002"></a>

#### F-QOS-002 · QoS bucket classification from the session mount root

*Assigns each session to a QoS bucket ("volume") derived from the client-supplied mount root, not from the inode a request touches.*

The volume id is the session's client_metadata["root"], truncated to its first 3 path components when it is deeper (SUBVOL_ROOT_DEPTH = 3; filepath parsing skips empty components), returned byte for byte when its depth is 3 or less, and "" when the key is absent or empty. Examples: /volumes/_nogroup/sv1/<uuid> gives /volumes/_nogroup/sv1, / gives /, /home/alice/projects/x gives /home/alice/projects. All sessions with the same value share one bucket on a rank; the path a request operates on is never consulted, and CInode::get_subvolume_id (used by subvolume metrics) is not used. The claimed root is validated at session open (non-empty, starts with /, MDSAuthCaps::path_capable, otherwise CEPH_SESSION_REJECT), so path-restricted clients can only join buckets inside their allowed path; a missing root key is not checked.

- **Class:** POSIX `operational` · exposure `client-wire`, `admin` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, cephfs-mirror
- **Map:** [qos-dmclock-scheduler.md › 3.3 Classification: session to volume id](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#33-classification-session-to-volume-id) · [qos-dmclock-scheduler.md › 8. Edge cases, failure handling and gotchas](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDSDmclockScheduler.cc:42-51 (MDSDmclockScheduler::convert_subvol_root)`, `src/mds/MDSDmclockScheduler.cc:53-61 (MDSDmclockScheduler::get_volume_id)`, `src/mds/MDSDmclockScheduler.h:348 (SUBVOL_ROOT_DEPTH)`, `src/mds/Server.cc:775-799 (Server::handle_client_session)`, `linux/fs/ceph/mds_client.c:1651-1656 (create_session_full_msg)`
- **Notes:** The kernel sends the canonicalized device-string path or "/". libcephfs sends mount_root (overridable with the client_metadata option) without canonicalization, and ceph-fuse passes -r verbatim, so a trailing slash makes a different bucket. Root-mounted clients (mgr volumes libcephfs mounts, cephfs-mirror, NFS-Ganesha with the nfs module default cmount_path=/) all share the "/" bucket. Sessions opened by Client::start_reclaim carry no root and stay in the "" bucket (code reading, not reproduced). doc/cephfs/mds-qos.rst says each subvolume gets its own allocation; the code buckets by mount-root prefix per rank.

<a id="f-qos-003"></a>

#### F-QOS-003 · QoS per-op cost table

*Charges each queued request a fixed cost of 1 for 11 read-type ops and 3 for every other op.*

get_op_cost returns 1 for LOOKUP, LOOKUPHASH, LOOKUPPARENT, LOOKUPINO, LOOKUPNAME, GETATTR, READDIR, OPEN, LOOKUPSNAP, LSSNAP and GETFILELOCK, and 3 for everything else, including the read-only GETVXATTR, READDIR_SNAPDIFF and FILE_BLOCKDIFF. The cost is fixed when the request is enqueued and does not depend on request size, outcome, or the number of dirfrags or ranks involved. dmClock uses it as a multiplier of the per-request tag increment, so reservation, weight and limit are in cost units per second.

- **Class:** POSIX `operational` · exposure `config` · relied on by admin
- **Map:** [qos-dmclock-scheduler.md › 3.4 Cost model](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#34-cost-model)
- **Code:** `src/mds/MDSDmclockScheduler.cc:745-780 (MDSDmclockScheduler::get_op_cost)`
- **Notes:** The user docs say read-like ops cost 1 and mutating ops 3; the code also charges 3 to GETVXATTR and the snapdiff and blockdiff reads. The table is compiled in, not configurable.

<a id="f-qos-004"></a>

#### F-QOS-004 · dmClock tag-based scheduling per bucket

*Orders and rate-limits queued requests per bucket with dmClock reservation, proportion (weight) and limit tags.*

The MDS instantiates crimson::dmclock::PushPriorityQueue<VolumeId, ClientRequest> with immediate tag calculation, AtLimit::Wait (over-limit requests are delayed, never rejected), anticipation timeout 0, a can_handle function that always returns true, and ReqParams {0, 0} (no distributed dmClock information). Tags use the wall-clock (CLOCK_REALTIME) enqueue time t and cost c: R = max(t, R_prev + c/reservation), P = max(t, P_prev + c/weight), L = max(t, L_prev + c/limit). Each scheduling pass dispatches the bucket with the smallest due R tag, else the ready bucket (L due) with the smallest P, else arms a wake-up at min(R, L). Each bucket is one FIFO, so requests of all sessions of a bucket leave in arrival order. Client records idle for 300 s get a prop_delta when they return, records idle for 600 s are erased (at most 2000 per 60 s pass); these constants are compile-time.

- **Class:** POSIX `operational` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [qos-dmclock-scheduler.md › 3.6 dmClock as instantiated by the MDS](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#36-dmclock-as-instantiated-by-the-mds) · [qos-dmclock-scheduler.md › 3.7 Effective behaviour of this integration (analysis fro...](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#37-effective-behaviour-of-this-integration-analysis-from-code-not-measured)
- **Code:** `src/mds/MDSDmclockScheduler.h:57 (Queue)`, `src/dmclock/src/dmclock_server.h:136-184 (RequestTag)`, `src/dmclock/src/dmclock_server.h:1110-1181 (do_next_request)`, `src/dmclock/src/dmclock_server.h:1565-1580 (PushPriorityQueue)`
- **Notes:** Analysis from code, not measured: there is no capacity model, so reservation and weight only reorder requests that are eligible at the same moment; a backlogged bucket is capped near limit cost units/s per rank (about max(reservation, limit) when the defaults set reservation > limit). Tags are fixed at insertion, so raising a limit does not speed up an existing backlog. The internal FIFO and the dmClock deques are unbounded and there is no back-pressure to clients; the messenger dispatch throttle is released before queueing. A wall-clock step backwards stalls queued requests. request_completed() is never called. The sched-ahead thread re-polls instead of sleeping while requests wait on future tags.

<a id="f-qos-005"></a>

#### F-QOS-005 · QoS scheduler threads and hand-off to request processing

*Drains and schedules the QoS queues on three extra threads per rank and hands eligible requests back to the normal request path under mds_lock.*

Every MDSRank starts a scheduler thread (moves the internal FIFO into dmClock and applies parameter updates), the dmClock sched-ahead thread and the dmClock cleaner thread, also when QoS is disabled. The hand-off (submit_request_to_mds) updates the bucket statistics, takes mds_lock, calls Server::handle_client_request and releases mds_lock while still holding the dmClock data mutex, so at most one hand-off is in progress at any time. Only the initial dispatch is gated: lock waits, journaling and C_MDS_RetryRequest retries are not charged again, but message-level retries (C_MDS_RetryMessage through MDSRank::retry_dispatch) and requests resent after MClientRequestForward are queued and charged again. Time in the QoS queues counts toward op age (the op's initiated time is the message receive stamp) and slow-request warnings, while still-queued requests appear in no dump_ops_in_flight output and no perf counter (mds.request counts at hand-off).

- **Class:** POSIX `operational` · exposure `metrics` · relied on by admin, internal
- **Map:** [qos-dmclock-scheduler.md › 3.5 The scheduler thread](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#35-the-scheduler-thread) · [qos-dmclock-scheduler.md › 3.8 Hand-off to the request path](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#38-hand-off-to-the-request-path) · [qos-dmclock-scheduler.md › 6. Concurrency and mds_lock](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#6-concurrency-and-mds_lock)
- **Code:** `src/mds/MDSDmclockScheduler.cc:490-545 (MDSDmclockScheduler::process_request_handler)`, `src/mds/MDSDmclockScheduler.cc:97-113 (MDSDmclockScheduler::submit_request_to_mds)`, `src/dmclock/src/dmclock_server.h:1761-1792 (run_sched_ahead)`, `src/mds/MDCache.cc:9837 (MDCache::request_start)`
- **Notes:** The hand-off skips MDSRank::_dispatch bookkeeping (heartbeat_reset, laggy deferral, _advance_queues) for the second half. The map lists nine concurrency defects found by reading (section 6.5), including a teardown order in which the scheduler outlives server and mdcache, and a lost wake-up window at shutdown; none reproduced.

<a id="f-qos-006"></a>

#### F-QOS-006 · Default QoS parameters (mds_dmclock_* options)

*Cluster-wide on/off switch plus the reservation, weight and limit applied to every bucket without an override.*

mds_dmclock_enable (bool, default false) and mds_dmclock_reservation, mds_dmclock_weight, mds_dmclock_limit (float, default 1000.0 each, runtime, no min or max) are tracked keys; the change handler runs on the mds-rank-fin finisher under mds_lock, enables or disables QoS when the enable flag changes and reloads the defaults. Defaults apply per bucket per rank, including the shared "/" and "" buckets, so enabling QoS with defaults caps every bucket at about 1000 cost units/s per rank (about 333 creates/s or 1000 lookups/s). A default <= 0 or nan makes the gate validity check fail, so new requests of default-using buckets bypass QoS.

- **Class:** POSIX `operational` · exposure `config` · relied on by mon, admin
- **Gates:** mds_dmclock_enable; mds_dmclock_reservation; mds_dmclock_weight; mds_dmclock_limit
- **Map:** [qos-dmclock-scheduler.md › 3.9 QoS parameters: defaults, overrides, validity](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#39-qos-parameters-defaults-overrides-validity) · [qos-dmclock-scheduler.md › 5.3 Config options](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#53-config-options)
- **Code:** `src/common/options/mds.yaml.in:1850-1885 (mds_dmclock_enable)`, `src/mds/MDSDmclockScheduler.cc:651-688 (MDSDmclockScheduler::handle_conf_change)`, `src/mds/MDSRank.cc:4293-4305 (MDSRankDispatcher::handle_conf_change)`, `src/mds/MDSRank.cc:4168-4171 (MDSRankDispatcher::get_tracked_keys)`
- **Notes:** Code reading, not reproduced: a nan default trips ceph_assert(stored == config) in handle_conf_change; a default set to 0 while requests sit in the FIFO gives them +inf tags (assert in builds without NDEBUG, otherwise the bucket stalls); the dmClock threads read the defaults through a pointer while the finisher writes them (data race). The option text "for each MDS client" is inaccurate (it is per bucket). No mgr module, cephadm or Rook code sets these options. See also F-ADM-061 (config observer).

<a id="f-qos-007"></a>

#### F-QOS-007 · Per-bucket QoS overrides (qos set, qos rm)

*Admin commands that set or clear a reservation/weight/limit triple for one bucket on one rank.*

"qos set <path> <reservation> <weight> <limit>" (integers, each >= 1, reservation <= limit) and "qos rm <path>" require an existing VolumeInfo whose id equals path exactly (otherwise "volume_info doesn't exist (<path>)"); they only enqueue an update that the scheduler thread applies later, so "qos get" right afterwards may show old values. Output is results{return_code: 1} on success and 0 on failure plus a status string ("malformed path", "reservation cannot be greater than limit", ...); the command's integer result is 0 in all cases, and a wrong argument type gives -EINVAL. Overrides live only in memory and per rank: they are lost on restart, failover, or when the bucket's last session on that rank closes, and they cannot be set before a client with that root has a session on the rank. ceph tell needs an unrestricted allow * MDS cap; a daemon without a rank answers mds_not_active.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin
- **Map:** [qos-dmclock-scheduler.md › 5.4 Admin commands](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#54-admin-commands) · [qos-dmclock-scheduler.md › 3.9 QoS parameters: defaults, overrides, validity](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#39-qos-parameters-defaults-overrides-validity)
- **Code:** `src/mds/MDSDaemon.cc:597-618 (MDSDaemon::set_up_admin_socket)`, `src/mds/MDSDmclockScheduler.cc:782-832 (MDSDmclockScheduler::_process_asok_qos_set)`, `src/mds/MDSDmclockScheduler.cc:842-860 (MDSDmclockScheduler::_process_asok_qos_rm)`, `src/mds/MDSRank.cc:3124-3135 (MDSRankDispatcher::handle_asok_command)`
- **Notes:** No mgr module, cephadm, the volumes plugin or the dashboard calls these commands (grep of src/pybind). Path matching is an exact string comparison against the computed id. MDSDmclockScheduler::read_xattrs is declared (MDSDmclockScheduler.h:392) but has no definition and no caller: there is no vxattr-based QoS configuration. Documented in doc/cephfs/mds-qos.rst. Command plane: F-ADM-056.

<a id="f-qos-008"></a>

#### F-QOS-008 · QoS inspection commands and per-bucket statistics

*dump qos and qos get report QoS state, effective parameters and per-bucket queueing statistics.*

"dump qos" returns qos_info{qos_state{qos_enabled[, state, default_reservation, default_weight, default_limit, mds_dmclock_queue_size, inflight_requests]}, volume_infos[{volume_id, use_default[, reservation, weight, limit], throttle, latency_avg, inflight_requests, session_cnt, session_list}]}, listing buckets even while QoS is disabled; "qos get <path>" returns one bucket as volume_qos_info. Statistics are updated at hand-off: latency is the time spent in the FIFO plus the dmClock queue (excluding the mds_lock wait and processing), latency_avg is a running mean halved at 1000 samples, throttle is a DecayCounter (60 s half-life) incremented when latency > 1/limit and inflight > limit, and inflight_requests counts requests inside dmClock, not requests executing in the MDS. mds_dmclock_queue_size is the size of the internal FIFO, not of the dmClock queue. No perf counter, health warning or mgr metric exists for QoS.

- **Class:** POSIX `operational` · exposure `admin` · relied on by admin
- **Map:** [qos-dmclock-scheduler.md › 3.12 Statistics](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#312-statistics) · [qos-dmclock-scheduler.md › 5.4 Admin commands](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#54-admin-commands) · [qos-dmclock-scheduler.md › 5.5 Perf counters, health, mgr metrics](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#55-perf-counters-health-mgr-metrics)
- **Code:** `src/mds/MDSDmclockScheduler.cc:163-190 (MDSDmclockScheduler::dump)`, `src/mds/MDSDmclockScheduler.cc:231-262 (MDSDmclockScheduler::hit_volume_throttle)`, `src/mds/MDSDmclockScheduler.cc:871-889 (MDSDmclockScheduler::process_asok_qos_get)`, `src/mds/MDSDmclockScheduler.h:171-300 (VolumeInfo)`
- **Notes:** Command plane: F-ADM-056.

<a id="f-qos-009"></a>

#### F-QOS-009 · QoS bucket state lifecycle tied to sessions

*Creates per-bucket QoS state when the first session of a bucket appears on a rank and erases it when the last one closes.*

add_session runs on journaled session open (Server::_session_logged), on force-opened or imported sessions (finish_force_open_sessions) and on accepted reconnect, and, when QoS is enabled on an active rank, for every OPEN or STALE session; it creates a VolumeInfo with use_default=true and enqueues its first parameter update. remove_session runs on journaled close and kill (evict, blocklist, reclaim) and erases the VolumeInfo, including any override, when its session set becomes empty, asserting that none of its requests are inside dmClock. Both hooks return early while QoS is disabled, so entries survive a disable/enable cycle and accumulate stale session ids. The dmClock client record is left to the 600 s idle cleaner.

- **Class:** POSIX `internal` · exposure `internal` · relied on by admin, internal
- **Map:** [qos-dmclock-scheduler.md › 3.10 Volume-info lifecycle](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#310-volume-info-lifecycle) · [qos-dmclock-scheduler.md › 4. Data structures and invariants](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#4-data-structures-and-invariants)
- **Code:** `src/mds/Server.cc:964 (Server::_session_logged)`, `src/mds/Server.cc:1128 (Server::finish_force_open_sessions)`, `src/mds/MDSDmclockScheduler.cc:418-457 (MDSDmclockScheduler::add_session)`, `src/mds/MDSDmclockScheduler.cc:350-368 (MDSDmclockScheduler::delete_session_from_volume_info)`
- **Notes:** Code reading, not reproduced: evicting or closing the last session of a bucket while its requests sit in dmClock hits ceph_assert(inflight == 0) and aborts the MDS, which is typical for a client being throttled when it is evicted; a bucket erased and re-created while one of its requests is still in the FIFO is tagged through a stale or freed record.

<a id="f-qos-010"></a>

#### F-QOS-010 · QoS enable, disable and drain on rank state and option changes

*Turns QoS on when a rank becomes active and off, with a draining sequence, when the rank starts stopping or the option is cleared.*

MDSRank::active_start marks the scheduler active and enables QoS if mds_dmclock_enable is true (adding all OPEN and STALE sessions); stopping_start disables it before exports and evictions begin. disable_qos_feature, called with mds_lock held, clears the enabled flag (new requests bypass), then repeatedly drops mds_lock, sleeps 1 s and re-takes it until the FIFO is empty, and finally flushes the remaining dmClock requests by handing them off directly (cancel_inflight_request); it therefore blocks its caller, the finisher thread or the messenger dispatch thread inside handle_mds_map, for at least 2 s. Requests queued before the disable are still rate-limited while draining.

- **Class:** POSIX `operational` · exposure `config` · relied on by admin, internal
- **Gates:** mds_dmclock_enable
- **Map:** [qos-dmclock-scheduler.md › 3.11 Enabling, disabling and rank-state changes](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#311-enabling-disabling-and-rank-state-changes) · [qos-dmclock-scheduler.md › 6.5 Concurrency defects found by reading (not reproduced)](../cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#65-concurrency-defects-found-by-reading-not-reproduced)
- **Code:** `src/mds/MDSRank.cc:2134-2135 (MDSRank::active_start)`, `src/mds/MDSRank.cc:2256-2257 (MDSRank::stopping_start)`, `src/mds/MDSDmclockScheduler.cc:567-579 (MDSDmclockScheduler::enable_qos_feature)`, `src/mds/MDSDmclockScheduler.cc:607-635 (MDSDmclockScheduler::disable_qos_feature)`, `src/mds/MDSDmclockScheduler.cc:581-605 (MDSDmclockScheduler::cancel_inflight_request)`
- **Notes:** Code reading, not reproduced: cancel_inflight_request holds volume_info_lock and re-acquires it through hit_volume_throttle (non-recursive std::mutex), so disabling QoS while any bucket is held at its limit self-deadlocks the finisher or the dispatch thread (the daemon then dispatches no message); it also inverts the volume_info_lock/data_mtx order. The disable drops mds_lock inside callers that assume it is held throughout (handle_mds_map, the conf-change finisher, ~MDSDaemon).

### Client metrics protocol

How clients learn that the MDS accepts metrics, what they send in CEPH_MSG_CLIENT_METRICS and how each active rank ingests it, plus the subvolume id the MDS puts in replies so that clients can tag I/O per subvolume.

<a id="f-qos-011"></a>

#### F-QOS-011 · Client metrics negotiation (feature bit 14 and metric_spec)

*Tells clients in CEPH_SESSION_OPEN whether the MDS accepts metrics and which metric types it wants.*

For sessions with CEPHFS_FEATURE_MIMIC, the CEPH_SESSION_OPEN sent on session open, on import force-open and on reconnect completion carries supported_features (bits 0..24 in this tree, including bit 14 CEPHFS_FEATURE_METRIC_COLLECT) and metric_spec = CEPHFS_METRIC_FEATURES_ALL (metric types 0..16). metric_spec_t is ENCODE_START(1,1) around a feature_bitset_t (u32 byte length, then little-endian 64-bit blocks) in MClientSession v4+. The kernel sends metrics only on sessions whose OPEN carried bit 14 and ignores metric_spec; libcephfs sends only while rank 0 is up:active, has an open session and advertised bit 14, and sends each type (0..9 and 16) only if its bit is set in the MDS metric_spec, unless client_collect_and_send_global_metrics=true. The client's own advertised metric_spec is stored in client_metadata_t, persisted with session_info_t, and used only for display (session ls / client ls).

- **Class:** POSIX `operational` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): OPEN without supported_features and metric_spec for sessions lacking CEPHFS_FEATURE_MIMIC. The 7.3 kernel and libcephfs at this commit advertise bit 8, so every OPEN they receive (session open, import force-open, reconnect) carries supported_features with bit 14 and metric_spec. An OPEN without both fields (MClientSession v1) goes only to sessions without bit 8: kernels up to v4.18 and Ceph clients up to 13.2.1, none of which send metrics. Evidence: `src/mds/Server.cc:971-974 (Server::_session_logged)`, `src/mds/Server.cc:1131-1134 (Server::finish_force_open_sessions)`, `src/mds/Server.cc:1694-1697 (Server::handle_client_reconnect)`, `src/messages/MClientSession.h:101-106 (MClientSession::encode_payload)`, `linux/fs/ceph/mds_client.h:47 (CEPHFS_FEATURE_MIMIC advertised)`, `src/client/Client.cc:448-452 (myfeatures)`.
- **Gates:** CEPHFS_FEATURE_METRIC_COLLECT (session feature bit 14); CEPHFS_FEATURE_MIMIC (MClientSession v4+ fields); client_collect_and_send_global_metrics (libcephfs, default false); kernel module parameter disable_send_metrics
- **Map:** [metrics-and-subvolume-metrics.md › 3.2 Negotiation: which client sends what, to which rank](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#32-negotiation-which-client-sends-what-to-which-rank) · [metrics-and-subvolume-metrics.md › 7.1 Linux kernel client (linux/fs/ceph)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#71-linux-kernel-client-linuxfsceph) · [metrics-and-subvolume-metrics.md › 7.2 libcephfs / ceph-fuse / NFS-Ganesha (src/client/Clien...](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#72-libcephfs--ceph-fuse--nfs-ganesha-srcclientclientcc)
- **Code:** `src/mds/Server.cc:310 (Server::Server)`, `src/mds/Server.cc:973 (Server::_session_logged)`, `src/mds/Server.cc:668-677 (Server::handle_client_session)`, `src/mds/cephfs_features.h:84-102 (CEPHFS_METRIC_FEATURES_ALL)`, `src/client/Client.cc:7640-7660 (Client::collect_and_send_global_metrics)`
- **Notes:** The "already opened" OPEN reply sent to a NOTIFY_SESSION_STATE client that re-sends REQUEST_OPEN carries an empty metric_spec, which libcephfs adopts; on the rank-0 session it then sends empty MClientMetrics until another full OPEN arrives (code inspection). Without MIMIC, MClientSession is encoded as v1 and carries neither field. See also F-PROTO-020 (feature bitmap), F-PROTO-021.

<a id="f-qos-012"></a>

#### F-QOS-012 · MClientMetrics ingestion of per-client metrics (types 0..9)

*Accepts CEPH_MSG_CLIENT_METRICS on every active rank and keeps the latest per-client cap, lease, latency, open-file and I/O-size values.*

MClientMetrics (type 29, v1, compat 1) carries a le32 item count, then per item a le32 type and an ENCODE_START envelope (u8 struct_v, u8 struct_compat, le32 struct_len, body). Types: 0 CAP_INFO (hits, misses, total), 1-3 READ/WRITE/METADATA_LATENCY (v2: cumulative latency sum as utime, running mean, Welford sq_sum in ns^2, count), 4 DENTRY_LEASE (hits, misses, total), 5 OPENED_FILES, 6 PINNED_ICAPS, 7 OPENED_INODES (each value plus total_inodes), 8-9 READ/WRITE_IO_SIZES (ops, bytes); ids 10..15 exist only as metric_spec bits. MetricsHandler is registered as a messenger dispatcher (PRIORITY_HIGH, outside mds_lock) only in active_start, so earlier messages are dropped. Values are stored per entity_inst_t and overwrite the previous ones (the MDS does not accumulate); payloads from clients without a tracked session are ignored, and nothing is ever replied.

- **Class:** POSIX `operational` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Legacy parts** (old-clients): Decoding version-1 READ/WRITE/METADATA_LATENCY payloads (lat only). The 7.3 kernel (since v5.18) and libcephfs at this commit send READ/WRITE/METADATA_LATENCY payloads as version 2 with mean, sq_sum and count. Version-1 latency payloads, decoded with sq_sum and count left uninitialized, come only from kernels v5.9-v5.17, CentOS Stream 8 and Ceph clients 16.2.0-16.2.10 and 17.2.0-17.2.3. Evidence: `src/include/cephfs/metrics/Types.h:184-194 (ReadLatencyPayload::decode)`, `src/include/cephfs/metrics/Types.h:235-245 (WriteLatencyPayload::decode)`, `src/include/cephfs/metrics/Types.h:286-296 (MetadataLatencyPayload::decode)`, `src/include/cephfs/metrics/Types.h:176 (ReadLatencyPayload::encode, version 2)`, `linux/fs/ceph/metric.c:181-208 (ceph_mdsc_send_metrics, ver 2)`, `linux-v5.15/fs/ceph/metric.c:62 (ver 1)`, `linux-v5.18/fs/ceph/metric.c:67 (ver 2)`, `cs8-kernel/fs/ceph/metric.c:62 (ver 1)`, `v16.2.10:src/include/cephfs/metrics/Types.h:140 (ENCODE_START(1, 1))`, `v17.2.3:src/include/cephfs/metrics/Types.h:140 (ENCODE_START(1, 1))`.
- **Map:** [metrics-and-subvolume-metrics.md › 3.3 MClientMetrics wire format (client → MDS)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#33-mclientmetrics-wire-format-client--mds) · [metrics-and-subvolume-metrics.md › 3.4 Ingestion on each rank (MetricsHandler)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#34-ingestion-on-each-rank-metricshandler)
- **Code:** `src/messages/MClientMetrics.h:12-59 (MClientMetrics)`, `src/include/cephfs/metrics/Types.h:878-924 (ClientMetricMessage::decode)`, `src/mds/MetricsHandler.cc:418-444 (MetricsHandler::handle_client_metrics)`, `src/mds/MetricsHandler.cc:198-374 (MetricsHandler::handle_payload)`
- **Notes:** A v1 latency encoding leaves sq_sum and count uninitialized. A payload processed after remove_session marked the entry REMOVE cancels the removal and nothing removes the entry later (code inspection). handle_client_metrics reads the Session through the connection priv without mds_lock or a reference. See also F-PROTO-096 (encoding and send rules).

<a id="f-qos-013"></a>

#### F-QOS-013 · Tolerant decoding of unknown metric types

*Skips unknown metric types by struct_len so that newer clients do not break their connection.*

An unknown type id decodes as UnknownPayload, which skips struct_len bytes. A known type whose struct_compat exceeds the supported version throws; any decode exception in MClientMetrics::decode_payload makes decode_message return null, and both ProtocolV1 and ProtocolV2 then fault the connection, resetting the client's session socket. The kernel never sends metrics to an MDS without bit 14 because such an MDS might not decode them.

- **Class:** POSIX `supporting` · exposure `client-wire` · relied on by kclient, libcephfs
- **Map:** [metrics-and-subvolume-metrics.md › 3.3 MClientMetrics wire format (client → MDS)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#33-mclientmetrics-wire-format-client--mds) · [metrics-and-subvolume-metrics.md › 7.1 Linux kernel client (linux/fs/ceph)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#71-linux-kernel-client-linuxfsceph)
- **Code:** `src/include/cephfs/metrics/Types.h:537-558 (UnknownPayload)`, `src/msg/Message.cc:1014-1030 (decode_message)`, `src/msg/async/ProtocolV2.cc:1451-1453 (ProtocolV2::handle_message)`, `linux/fs/ceph/metric.c:345-349 (metric_get_session)`

<a id="f-qos-014"></a>

#### F-QOS-014 · Metrics accepted on any active rank (client send rules)

*Aggregation consequence of the client send rules: which rank receives each client's metrics.*

See F-PROTO-096 for the send rules. Metrics-side facts: libcephfs, ceph-fuse and NFS-Ganesha send to rank 0 only (every Client::tick, client_tick_interval 1 s, also when empty); the kernel sends every second to the last session whose CEPH_SESSION_OPEN carried bit 14 (falling back to the lowest-numbered usable session) while that rank is up:active. Every rank that holds a session for a client but does not receive its metrics forwards zeroed values every interval (rank != 0 for libcephfs, every rank except the bound one for the kernel), which makes values without an updated flag flap on rank 0 (F-QOS-029).

- **Class:** POSIX `operational` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [metrics-and-subvolume-metrics.md › 3.2 Negotiation: which client sends what, to which rank](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#32-negotiation-which-client-sends-what-to-which-rank) · [metrics-and-subvolume-metrics.md › 3.1 Pipeline overview](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#31-pipeline-overview)
- **Code:** `src/client/Client.cc:7640-7764 (Client::collect_and_send_global_metrics)`, `linux/fs/ceph/metric.c:91-331 (ceph_mdsc_send_metrics)`, `linux/fs/ceph/metric.c:334-405 (metric_get_session)`, `linux/fs/ceph/mds_client.c:74 (ceph_metric_bind_session)`
- **Notes:** As their own metric_spec libcephfs advertises CEPHFS_METRIC_FEATURES_ALL and the kernel CEPHFS_METRIC_SPEC_CLIENT_SUPPORTED, both types 0..16.

<a id="f-qos-015"></a>

#### F-QOS-015 · Subvolume id in InodeStat and cap flush acks

*Sends each inode's enclosing subvolume id to clients so that they can tag data I/O per subvolume.*

CInode::get_subvolume_id returns the inode number of the outermost enclosing subvolume root (snaprealm with srnode.is_subvolume(), set by the ceph.dir.subvolume vxattr), or 0. It is appended as u64 subvolume_id to InodeStat struct v9 (only for sessions with CEPHFS_FEATURE_REPLY_ENCODING, bit 9) and is the MClientCaps v13 field subvolume_id, which the MDS fills only on FLUSH_ACK and FLUSHSNAP_ACK. The kernel parses the InodeStat field, assigns it once per inode and WARN_ON_ONCEs if a different non-zero id arrives (it keeps the old id and ignores X to 0); libcephfs records it from replies and flush acks and forgets inodes when it sends their cap releases. An inode's id changes only when ceph.dir.subvolume is set or cleared on an ancestor, because cross-subvolume rename and link fail with EXDEV and nested subvolumes with EINVAL.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Gates:** CEPHFS_FEATURE_REPLY_ENCODING (bit 9) for InodeStat v9
- **Map:** [metrics-and-subvolume-metrics.md › 3.7 Subvolume metrics end to end](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#37-subvolume-metrics-end-to-end) · [metrics-and-subvolume-metrics.md › 7.1 Linux kernel client (linux/fs/ceph)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#71-linux-kernel-client-linuxfsceph) · [rstats-quotas-and-subvolumes.md](../cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md)
- **Code:** `src/mds/CInode.cc:5597-5600 (CInode::get_subvolume_id)`, `src/mds/SnapRealm.cc:150-154 (SnapRealm::check_cache)`, `src/mds/Locker.cc:2501-2506 (Locker::maybe_set_subvolume_id)`, `linux/fs/ceph/inode.c:891-912 (ceph_inode_set_subvolume)`
- **Notes:** The kernel does not consume MClientCaps.subvolume_id. The subvolume flag and the EXDEV/EINVAL rules belong to 08-snapshots-quotas-subvolumes; this entry records the metrics use of the id. See also F-SNAP-054 (subvolume-side facts), F-SNAP-050 (subvolume flag).

<a id="f-qos-016"></a>

#### F-QOS-016 · Subvolume I/O metrics (type 16) ingestion and path resolution

*Turns per-subvolume I/O samples keyed by subvolume inode number into per-path samples on the receiving rank.*

Type 16 SUBVOLUME_METRICS carries u64 n followed by n envelopes (v1, compat 1, length 56) of AggregatedIOMetrics {u64 subvolume_id, u32 read_count, u32 write_count, u64 read_bytes, u64 write_bytes, u64 read_latency_us, u64 write_latency_us, u64 time_stamp}; the latencies are sums over the period and clients send time_stamp 0. For each element the rank resolves the path with MDSRank::get_path (takes mds_lock, uses only a cached head inode); an unknown inode gives an empty path and the sample is dropped silently. Samples are appended per path with the MDS receive time and, at every update interval, rolled up into one SubvolumeMetric per path: sums of ops and bytes and an ops-weighted average of the per-report latency sums (in microseconds). Subvolume samples are accepted from any session, without the per-client tracking check.

- **Class:** POSIX `operational` · exposure `client-wire`, `metrics` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mgr, admin
- **Map:** [metrics-and-subvolume-metrics.md › 3.7 Subvolume metrics end to end](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#37-subvolume-metrics-end-to-end) · [metrics-and-subvolume-metrics.md › 3.3 MClientMetrics wire format (client → MDS)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#33-mclientmetrics-wire-format-client--mds)
- **Code:** `src/include/cephfs/metrics/Types.h:643-745 (AggregatedIOMetrics)`, `src/include/cephfs/metrics/Types.h:747-788 (SubvolumeMetricsPayload)`, `src/mds/MetricsHandler.cc:376-412 (MetricsHandler::handle_payload)`, `src/mds/MetricsHandler.cc:578-645 (MetricsHandler::aggregate_subvolume_metrics)`, `src/mds/MDSRank.cc:4103 (MDSRank::get_path)`
- **Notes:** The averaged latency is an ops-weighted mean of per-report latency sums, not a per-op mean. libcephfs sends zero-I/O entries for subvolumes whose new inodes it learned from replies (code inspection); the kernel skips empty entries. A kernel client bound to a rank that never cached the subvolume root loses its samples. Clients clamp: libcephfs latency to [1, 2^32-1] us and size to 2^31-1 per op, the kernel op counts to U32_MAX.

<a id="f-qos-017"></a>

#### F-QOS-017 · Feature bit 22 enables kernel subvolume metrics

*Metrics side of the bit 22 conflict: the kernel enables subvolume metrics when an MDS advertises bit 22.*

See F-PROTO-025 for the numbering conflict. Metrics side: the kernel names bit 22 CEPHFS_FEATURE_SUBVOLUME_METRICS; a CEPH_SESSION_OPEN that carries it enables the kernel's subvolume tracker and type 16 in its metrics, an OPEN without it disables and clears the tracker, and while enabled the kernel skips sessions without bit 22 when choosing a metrics target. The C++ MDS advertises bit 22 (as CHARMAP), so kernels enable subvolume metrics and send type-16 items, which this tree decodes (F-QOS-016).

- **Class:** POSIX `operational` · exposure `client-wire` · relied on by kclient
- **Gates:** session feature bit 22
- **Map:** [metrics-and-subvolume-metrics.md › 7.1 Linux kernel client (linux/fs/ceph)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#71-linux-kernel-client-linuxfsceph) · [metrics-and-subvolume-metrics.md › 8. Edge cases, failure handling and gotchas](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/cephfs_features.h:53 (CEPHFS_FEATURE_CHARMAP)`, `src/mds/Server.cc:4876-4893 (Server::can_handle_charmap)`, `linux/fs/ceph/mds_client.h:40 (CEPHFS_FEATURE_SUBVOLUME_METRICS)`, `linux/fs/ceph/mds_client.c:4581-4584 (handle_session)`
- **Notes:** libcephfs gates subvolume metrics on metric_spec bit 16, not on bit 22.

### Cross-rank metrics aggregation

Every active rank forwards client and subvolume data to rank 0 every 2 s; rank 0 tracks rank liveness with its own ping sequence. These are MDS-to-MDS messages.

<a id="f-qos-018"></a>

#### F-QOS-018 · Per-rank metrics forwarding to rank 0 (MMDSMetrics)

*Every active rank periodically sends its latest per-client metrics, subvolume aggregates and rank telemetry to rank 0.*

The mds-metrics thread runs update_rank0 every mds_metrics_update_interval (2 s, runtime): it samples rank telemetry, returns early if rank 0's address is unknown, builds metrics_message_t v3 {u64 seq, i32 rank, map<entity_inst_t, Metrics>, vector<SubvolumeMetric> (v2), RankPerfMetrics (v3)}, resets REFRESH entries to zeros and erases REMOVE entries, fetches used_bytes per subvolume, and sends MMDSMetrics (MSG_MDS_METRICS 0x501) to rank 0, which gets its own through loopback. Metrics is DENC(4,1) with a u32 update_type followed by the per-type structs. Rank 0 therefore sees a client's last report of the interval, zeros for fields without an updated flag (cap hits and misses) when none arrived, and keeps its previous value for guarded fields. Rank 0 applies REFRESH and REMOVE updates; any other update_type hits ceph_abort.

- **Class:** POSIX `internal` · exposure `mds-wire`, `config` · relied on by mds-ranks
- **Gates:** mds_metrics_update_interval (2 s)
- **Map:** [metrics-and-subvolume-metrics.md › 3.5 Forwarding to rank 0 (update_rank0, MMDSMetrics) and ...](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#35-forwarding-to-rank-0-update_rank0-mmdsmetrics-and-the-ping-protocol) · [metrics-and-subvolume-metrics.md › 3.6 Aggregation on rank 0 (MetricAggregator)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#36-aggregation-on-rank-0-metricaggregator) · [metrics-and-subvolume-metrics.md › 4. Data structures and invariants](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#4-data-structures-and-invariants)
- **Code:** `src/mds/MetricsHandler.cc:93-103 (MetricsHandler::init)`, `src/mds/MetricsHandler.cc:483-576 (MetricsHandler::update_rank0)`, `src/mds/MDSPerfMetricTypes.h:405-473 (metrics_message_t)`, `src/messages/MMDSMetrics.h:10-48 (MMDSMetrics)`, `src/mds/MetricAggregator.cc:741-777 (MetricAggregator::handle_mds_metrics)`
- **Notes:** While rank 0's address is unknown, the per-path sample vectors are not cleared and grow by one element per client per second. MMDSMetrics.rank is trusted as sent. Shutdown joins the mds-metrics thread under mds_lock while its last update_rank0 may need mds_lock (unverified whether it hangs in practice).

<a id="f-qos-019"></a>

#### F-QOS-019 · Rank-0 ping sequence and delayed-rank detection

*Rank 0 pings every active rank, accepts their updates only with a current sequence number and reports ranks that stopped answering as delayed.*

The rank-0 mds-ping thread sends MMDSPing (MSG_MDS_PING 0x502, version_t seq, starting at 1 per rank) to every up:active rank including itself every mds_ping_interval (5 s); a rank copies the seq into its next MMDSMetrics. Rank 0 accepts an update only if its seq is outstanding or already acked for that rank (repeats of the acked seq are accepted) and otherwise drops the whole message. A rank is lagging when the time since the send time of its last acked ping exceeds mds_ping_grace (15 s); lagging active ranks are reported to the mgr as rank_metrics_delayed (delayed_ranks in ceph fs perf stats). When rank 0 is not active or its address changes, the other ranks reset to seq 0, and their updates are rejected until the new rank 0 pings them.

- **Class:** POSIX `internal` · exposure `mds-wire`, `config` · relied on by mds-ranks, mgr
- **Gates:** mds_ping_interval (5 s); mds_ping_grace (15 s)
- **Map:** [metrics-and-subvolume-metrics.md › 3.5 Forwarding to rank 0 (update_rank0, MMDSMetrics) and ...](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#35-forwarding-to-rank-0-update_rank0-mmdsmetrics-and-the-ping-protocol) · [metrics-and-subvolume-metrics.md › 8. Edge cases, failure handling and gotchas](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/MDSPinger.cc:19-104 (MDSPinger::send_ping)`, `src/mds/MetricsHandler.cc:446-481 (MetricsHandler::handle_mds_ping)`, `src/messages/MMDSPing.h:10-50 (MMDSPing)`, `src/mds/MetricAggregator.cc:82-116 (MetricAggregator::init)`
- **Notes:** Any MDS can send MMDSPing and set next_seq; there is no sender validation. If a rank is replaced between two mdsmaps that rank 0 processes, rank 0 keeps pinging the old address (whether rank 0 can skip those epochs is unverified). After a rank-0 failover the first ping round usually finds no active ranks, so real pings start one interval later and updates in between are lost.

<a id="f-qos-020"></a>

#### F-QOS-020 · Client session lifecycle in metrics (REFRESH and REMOVE)

*Tracks client sessions per rank so that rank 0 creates and forgets per-client metric state as sessions open and close.*

add_session (under mds_lock, from session open, import force-open and reconnect completion) creates an entry with update_type REFRESH, or resets an existing entry to REFRESH, cancelling a pending REMOVE. remove_session (journaled close or kill) erases the entry locally if no update with a non-zero seq was sent since it was added, and otherwise zeroes it and marks it REMOVE so that the next update tells rank 0 to forget the client. On rank 0 a REMOVE deletes the client's labeled counters, decrements num_clients and erases its perf-query keys; a REMOVE for an untracked client is a no-op.

- **Class:** POSIX `operational` · exposure `mds-wire`, `metrics` · relied on by mds-ranks, tools, admin
- **Map:** [metrics-and-subvolume-metrics.md › 3.4 Ingestion on each rank (MetricsHandler)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#34-ingestion-on-each-rank-metricshandler) · [metrics-and-subvolume-metrics.md › 3.6 Aggregation on rank 0 (MetricAggregator)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#36-aggregation-on-rank-0-metricaggregator)
- **Code:** `src/mds/MetricsHandler.cc:127-180 (MetricsHandler::add_session)`, `src/mds/Server.cc:968 (Server::_session_logged)`, `src/mds/MetricAggregator.cc:663-739 (MetricAggregator::remove_metrics_for_rank)`
- **Notes:** A REMOVE that rank 0 rejects (bad seq) or that is lost is never retried, so the client's counters linger. QA asserts that per-client counters disappear on disconnect (qa/tasks/cephfs/test_admin.py:22-45).

<a id="f-qos-021"></a>

#### F-QOS-021 · Metrics state culling on mdsmap changes

*Rank 0 drops metric state of ranks that left the active set; other ranks re-target rank 0 when it moves.*

MetricAggregator::notify_mdsmap (called from handle_mds_map) compares the recorded active ranks with the mdsmap's up:active set, counting up:stopping and up:clientreplay as inactive. For departed ranks it calls cull_metrics_for_rank (decrements num_clients and erases query keys per client, drops the rank's client set, deletes mds_rank_perf{rank}) and resets the ping state, so late updates from them are rejected; new ranks get an address and an empty client set. MetricsHandler::notify_mdsmap on every rank clears the rank-0 address and resets seq when rank 0 is not up:active, and resets seq when rank 0's address changes.

- **Class:** POSIX `internal` · exposure `metrics` · relied on by mgr, internal
- **Map:** [metrics-and-subvolume-metrics.md › 3.6 Aggregation on rank 0 (MetricAggregator)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#36-aggregation-on-rank-0-metricaggregator) · [metrics-and-subvolume-metrics.md › 3.5 Forwarding to rank 0 (update_rank0, MMDSMetrics) and ...](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#35-forwarding-to-rank-0-update_rank0-mmdsmetrics-and-the-ping-protocol)
- **Code:** `src/mds/MetricAggregator.cc:826-863 (MetricAggregator::notify_mdsmap)`, `src/mds/MetricAggregator.cc:804-815 (MetricAggregator::cull_metrics_for_rank)`, `src/mds/MetricsHandler.cc:451-481 (MetricsHandler::notify_mdsmap)`
- **Notes:** Cull leak: cull_metrics_for_rank does not delete the per-client labeled counters; if the same (client, rank) refreshes later, a second counter set is created and renamed with a -<pointer> suffix (src/common/perf_counters.cc:42-47).

<a id="f-qos-022"></a>

#### F-QOS-022 · Rank telemetry: CPU usage and open requests (mds_rank_perf)

*Publishes each rank's process CPU percentage and number of in-flight ops as labeled counters.*

At every update interval a rank samples cpu_usage = round(100 x delta(utime+stime ticks) / CLK_TCK / delta t) from /proc/self/stat (100 means one full core; the first sample is 0) and open_requests = the op tracker's ops in flight (0 when op tracking is disabled), and sends them as RankPerfMetrics {u64 cpu_usage_percent, u64 open_requests} in MMDSMetrics. Ranks other than 0 expose their own mds_rank_perf{rank=N}; rank 0 creates or updates mds_rank_perf{rank} for every reporting rank. Both counters are PRIO_USEFUL (ids 40001-40002).

- **Class:** POSIX `operational` · exposure `metrics`, `mds-wire` · relied on by tools, admin, mds-ranks
- **Map:** [metrics-and-subvolume-metrics.md › 3.5 Forwarding to rank 0 (update_rank0, MMDSMetrics) and ...](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#35-forwarding-to-rank-0-update_rank0-mmdsmetrics-and-the-ping-protocol) · [metrics-and-subvolume-metrics.md › 5.3 Perf counters](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#53-perf-counters)
- **Code:** `src/mds/MetricsHandler.cc:673-751 (MetricsHandler::sample_cpu_usage)`, `src/mds/MetricAggregator.cc:779-802 (MetricAggregator::update_rank_perf_metrics)`, `src/mds/MetricsHandler.cc:64-89 (MetricsHandler::init)`, `src/mds/MDSPerfMetricTypes.h:310-332 (RankPerfMetrics)`

<a id="f-qos-023"></a>

#### F-QOS-023 · Subvolume quota_bytes and used_bytes reporting

*Reports each active subvolume's quota and used bytes alongside its I/O metrics.*

MDCache::broadcast_quota_to_client (under mds_lock, on active or stopping ranks, for auth unfrozen inodes inside a subvolume whose quota is enabled or changed; reached from predirty_journal_parents ancestor walks, nest scatter-gather completion and setxattr ceph.quota.*) feeds a per-rank cache subvolume_id -> {quota_bytes, used_bytes, last_activity}; quota_bytes is overwritten only when > 0 or when the quota is removed. At each update interval used_bytes is read as the projected rstat.rbytes of the locally cached subvolume root (0 if not cached), taking mds_lock once per subvolume, and cache entries idle for more than 2 x subv_metrics_window_interval are evicted. Both values travel in SubvolumeMetric v2 and are reported only for subvolumes that had I/O samples.

- **Class:** POSIX `operational` · exposure `metrics` · relied on by tools, admin, mgr
- **Map:** [metrics-and-subvolume-metrics.md › 3.4 Ingestion on each rank (MetricsHandler)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#34-ingestion-on-each-rank-metricshandler) · [metrics-and-subvolume-metrics.md › 3.7 Subvolume metrics end to end](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#37-subvolume-metrics-end-to-end)
- **Code:** `src/mds/MDCache.cc:2103-2135 (MDCache::broadcast_quota_to_client)`, `src/mds/MetricsHandler.cc:647-671 (MetricsHandler::maybe_update_subvolume_quota)`, `src/mds/MDSRank.cc:4112 (MDSRank::get_inode_rbytes)`
- **Notes:** The cache is keyed by subvolume id, not by quota inode, so any quota-enabled directory inside the subvolume (including a nested user quota) overwrites quota_bytes; the last broadcast wins. quota_bytes is known only on the rank that is auth for the quota inode, and other ranks may report a stale replica rbytes or 0. Rank 0 takes both values from the sample with the largest time_stamp, which comes from different hosts' steady clocks, so values can flap between ranks' views. QA asserts quota_bytes equals the provisioned size and that used_bytes grows and shrinks (qa/tasks/cephfs/test_subvolume.py:288-420).

### Metric counters and mgr reporting

How rank 0 publishes the aggregated metrics: labeled perf counters (counter dump, ceph-exporter, MMgrReport) and the mgr "MDS perf query" protocol consumed by the stats module (ceph fs perf stats) and cephfs-top.

<a id="f-qos-024"></a>

#### F-QOS-024 · Per-client labeled metric counters (mds_client_metrics)

*Rank 0 publishes per-(client, rank) labeled perf counters and a num_clients gauge.*

At aggregator init rank 0 creates mds_client_metrics{fs_name, id=<mds global_id>} with num_clients (u64, PRIO_CRITICAL), which counts (client, rank) pairs, not distinct clients. On the first REFRESH of a (client, rank) pair it creates mds_client_metrics-<fs_name>{client="client.<id>", rank="<rank>"} with cap_hits, cap_miss, avg_read_latency, avg_write_latency, avg_metadata_latency, dentry_lease_hits, dentry_lease_miss, opened_files, opened_inodes, pinned_icaps, total_inodes, total_read_ops, total_read_size, total_write_ops and total_write_size (PRIO_CRITICAL, ids 20001-20015), and updates them on every REFRESH. The set is deleted when a REMOVE for that pair arrives.

- **Class:** POSIX `operational` · exposure `metrics` · relied on by tools, admin
- **Map:** [metrics-and-subvolume-metrics.md › 3.6 Aggregation on rank 0 (MetricAggregator)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#36-aggregation-on-rank-0-metricaggregator) · [metrics-and-subvolume-metrics.md › 5.3 Perf counters](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#53-perf-counters)
- **Code:** `src/mds/MetricAggregator.cc:82-116 (MetricAggregator::init)`, `src/mds/MetricAggregator.cc:390-661 (MetricAggregator::refresh_metrics_for_rank)`, `src/mds/MetricAggregator.cc:26-63 (l_mds_client_metrics_start)`
- **Notes:** Quirks (code reading): avg_write_latency and avg_metadata_latency are time counters written with PerfCounters::set, which ignores non-u64 counters, so they stay 0; avg_read_latency is tset(utime_t(mean.sec*100, mean.nsec/1000000)), not a faithful conversion of the mean; opened_inodes is filled with total_inodes. Ranks that hold a session but receive no metrics show permanent zeros in their (client, rank) set. The corresponding perf-query pairs (F-QOS-029) are correct. See also F-ADM-068 (family list), F-QOS-027 (export names).

<a id="f-qos-025"></a>

#### F-QOS-025 · Subvolume sliding-window metrics (mds_subvolume_metrics)

*Rank 0 computes per-subvolume IOPS, throughput and latency over a sliding window and publishes them as labeled counters.*

For each SubvolumeMetric in an accepted MMDSMetrics, rank 0 creates on first sight mds_subvolume_metrics{fs_name, subvolume_path} and a sliding window of W = subv_metrics_window_interval seconds (30, min 30, read once at init), adds the sample stamped with its own steady clock, then prunes every tracked path to the last W seconds. Over the remaining samples, with D = floor(t_last - t_first) seconds forced to at least 1: avg_read_iops and avg_write_iops = sum of ops / D, avg_read_tp_Bps and avg_write_tp_Bps = sum of bytes / D, avg_read_lat_msec and avg_write_lat_msec = ops-weighted latency / 1000 (PRIO_CRITICAL), and quota_bytes and used_bytes (PRIO_USEFUL, UNIT_BYTES); ids 30001-30008. The same values feed perf-query counters subv_* (ids 16..23) as (value, 0) pairs.

- **Class:** POSIX `operational` · exposure `metrics`, `mds-wire`, `config` · relied on by tools, admin, mgr, mds-ranks
- **Gates:** subv_metrics_window_interval (30 s, level dev)
- **Map:** [metrics-and-subvolume-metrics.md › 3.7 Subvolume metrics end to end](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#37-subvolume-metrics-end-to-end) · [metrics-and-subvolume-metrics.md › 5.3 Perf counters](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#53-perf-counters)
- **Code:** `src/mds/MetricAggregator.cc:173-388 (MetricAggregator::refresh_subvolume_metrics_for_rank)`, `src/mgr/MDSPerfMetricTypes.h:493-551 (SlidingWindowTracker)`, `src/mds/mdstypes.h:812-848 (SubvolumeMetric)`
- **Notes:** D counts n samples over n-1 gaps, so rates are inflated by about n/(n-1) (factor 2 with one sample). Per-rank attribution of subvolume I/O is lost (keys always use rank "0"). QA asserts the label subvolume_path == "/volumes/_nogroup/<name>" and the presence of all 8 counters (qa/tasks/cephfs/test_subvolume.py:279-286).

<a id="f-qos-026"></a>

#### F-QOS-026 · Eviction of idle subvolume metrics

*Deletes a subvolume's counters and query keys once no sample for it arrived within the window.*

Pruning runs on every accepted MMDSMetrics for all tracked paths, not only those in the message; when a path's window becomes empty its mds_subvolume_metrics set is removed from the perf counter collection, matching perf-query keys are erased and the tracker is dropped. Counters therefore exist only while samples for the subvolume arrived in the last W seconds. The documented intent is that only data I/O creates samples (doc/cephfs/metrics.rst:109-119); the kernel follows it, while libcephfs also sends zero-I/O samples that keep the set alive with zero rates. Aggregator shutdown deletes the per-client, rank and num_clients counters but not the mds_subvolume_metrics sets.

- **Class:** POSIX `operational` · exposure `metrics` · relied on by tools, admin
- **Map:** [metrics-and-subvolume-metrics.md › 3.7 Subvolume metrics end to end](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#37-subvolume-metrics-end-to-end) · [metrics-and-subvolume-metrics.md › 7.4 ceph-exporter, Prometheus, dashboards, QA](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#74-ceph-exporter-prometheus-dashboards-qa)
- **Code:** `src/mds/MetricAggregator.cc:217-240 (MetricAggregator::refresh_subvolume_metrics_for_rank)`, `src/mds/MetricAggregator.cc:118-156 (MetricAggregator::shutdown)`
- **Notes:** QA asserts eviction after the window (qa/tasks/cephfs/test_subvolume.py:288-420).

<a id="f-qos-027"></a>

#### F-QOS-027 · Labeled metric counter naming and export

*Names and labels under which the metric counter sets are exported.*

See F-ADM-069 (counter dump and schema) and F-ADM-070 (ceph-exporter). Metrics-specific: the labeled sets appear only in counter dump and counter schema (not in perf dump); ceph-exporter names their series ceph_<promethized set name>_<counter> with the counter labels plus ceph_daemon, so mds_client_metrics-<fs> becomes ceph_mds_client_metrics_<fs>_...; counters at or above mgr_stats_threshold (5) also travel in MMgrReport, but the mgr prometheus module exports only unlabeled counters. Set names, label keys, counter names, priorities and units are documented in doc/cephfs/metrics.rst and asserted by QA (counter dump in qa/tasks/cephfs/test_subvolume.py and test_admin.py).

- **Class:** POSIX `operational` · exposure `metrics`, `admin` · relied on by tools, admin, mgr
- **Map:** [metrics-and-subvolume-metrics.md › 3.9 Consumers](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#39-consumers) · [metrics-and-subvolume-metrics.md › 7.4 ceph-exporter, Prometheus, dashboards, QA](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#74-ceph-exporter-prometheus-dashboards-qa)
- **Code:** `src/common/ceph_context.cc:558-562 (CephContext::_do_command)`, `src/exporter/DaemonMetricCollector.cc:101-160 (DaemonMetricCollector::parse_asok_metrics)`, `src/exporter/util.cc:47-62 (promethize)`
- **Notes:** See also F-ADM-068 (labeled counter families).

<a id="f-qos-028"></a>

#### F-QOS-028 · MDS perf query configuration from ceph-mgr

*Receives the set of regex-keyed metric queries from the mgr in MMgrConfigure for evaluation on rank 0.*

MMgrConfigure v4 (compat 1) carries u32 stats_period, u32 stats_threshold, an empty OSD query map and, for SERVER_PACIFIC peers, optional MetricConfigMessage {u32 type=1, MDSConfigPayload DENC(1,1){map<MDSPerfMetricQuery, set<MDSPerfMetricLimit>>}}; a config type other than 0 or 1 decodes as UnknownConfigPayload, which calls ceph_abort in the receiving daemon. MgrClient::handle_mgr_configure hands the queries to MetricAggregator::set_perf_queries on rank 0 (MgrClient does not store them), which replaces the query set, keeps accumulated values for queries still present and ignores MDSPerfMetricLimit. The mgr sends MMgrConfigure to every MDS connection whenever queries are added, removed or re-registered; the stats module calls reregister_mds_perf_queries once a report newer than a new rank-0 GID's up:active transition arrives, because a daemon that becomes rank 0 later has missed the earlier query set.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mgr
- **Map:** [metrics-and-subvolume-metrics.md › 3.8 Rank 0 ↔ mgr: the MDS perf query protocol](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#38-rank-0--mgr-the-mds-perf-query-protocol) · [metrics-and-subvolume-metrics.md › 7.3 ceph-mgr (what an MDS must do to feed the unmodified ...](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#73-ceph-mgr-what-an-mds-must-do-to-feed-the-unmodified-mgr)
- **Code:** `src/messages/MMgrConfigure.h:27-87 (MMgrConfigure)`, `src/mgr/MetricTypes.h:218-232 (MDSConfigPayload)`, `src/mgr/MgrClient.cc:468-499 (MgrClient::handle_mgr_configure)`, `src/mds/MetricAggregator.cc:865-878 (MetricAggregator::set_perf_queries)`, `src/pybind/mgr/stats/fs/perf_stats.py:211-241 (FSPerfStats.notify_fsmap)`
- **Notes:** mgr ghost queries: a report that still carries keys for a just-removed query re-inserts it with no ids (src/mgr/MetricCollector.cc:158,178); remove_query searches by id, so it stays, and get_queries puts it into every later MMgrConfigure until the mgr restarts (code inspection). See also F-ADM-106 (mgr/stats module).

<a id="f-qos-029"></a>

#### F-QOS-029 · MDS perf query key construction and counter pairs

*Builds metric keys from regex capture groups over rank, client identity and subvolume path, and fills a 16-byte counter pair per requested counter.*

A query is a key descriptor (varint count of sub-keys {u8 type MDS_RANK=0, CLIENT_ID=1, SUBVOLUME_PATH=2; ECMAScript std::regex with at least one capture group}) plus counter descriptors (ids 0..23); a key is the list of capture groups 1..n of regex_search per sub-key, and a query matches only if every sub-key yields a value. For client refreshes MDS_RANK is matched against the reporting rank and CLIENT_ID against "client.<global_id> <addrtype>:<ip>:<port>/<nonce>" (SUBVOLUME_PATH does not match); for subvolume refreshes MDS_RANK is always "0" without a regex check, CLIENT_ID never matches and SUBVOLUME_PATH is matched against the subvolume root path. Pairs: cap_hit (hits, misses), read/write/metadata_latency (sec, nsec of the cumulative sum), dentry_lease (hits, misses), opened_files, pinned_icaps, opened_inodes (value, total_inodes), read/write_io_sizes (ops, bytes), avg_*_latency (mean sec, nsec), stdev_*_latency (sq_sum in ns^2, count), subv_* (value, 0). Keys are created even for queries with zero counter descriptors, and a counter that was not updated keeps its previous value.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mgr
- **Map:** [metrics-and-subvolume-metrics.md › 3.8 Rank 0 ↔ mgr: the MDS perf query protocol](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#38-rank-0--mgr-the-mds-perf-query-protocol) · [metrics-and-subvolume-metrics.md › 3.6 Aggregation on rank 0 (MetricAggregator)](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#36-aggregation-on-rank-0-metricaggregator) · [metrics-and-subvolume-metrics.md › 7.3 ceph-mgr (what an MDS must do to feed the unmodified ...](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#73-ceph-mgr-what-an-mds-must-do-to-feed-the-unmodified-mgr)
- **Code:** `src/mgr/MDSPerfMetricTypes.h:321-348 (MDSPerfMetricQuery::get_key)`, `src/mds/MetricAggregator.cc:618-653 (MetricAggregator::refresh_metrics_for_rank)`, `src/mds/MetricAggregator.cc:322-337 (MetricAggregator::refresh_subvolume_metrics_for_rank)`, `src/mgr/MDSPerfMetricTypes.cc:22-92 (pack_counter)`
- **Notes:** When decode meets an unsupported sub-key or invalid regex it clears the descriptor and leaves the rest unread, misaligning the counter descriptors until DENC_FINISH (the mgr validates regexes with the same std::regex, so it never sends such a query). A query made only of MDS_RANK sub-keys also matches subvolume refreshes with key [["0"]]. Queries keyed only by CLIENT_ID merge all ranks into one key, so their cap_hit pair alternates between the real value and (0,0) from ranks that hold a session but receive no metrics.

<a id="f-qos-030"></a>

#### F-QOS-030 · MDS perf report to ceph-mgr (MMgrReport)

*Rank 0 attaches the current values of every configured query to each MMgrReport.*

Every stats_period (the mgr's mgr_stats_period, 5 s by default, raised at runtime when mgr_stats_period_autotune detects a backed-up mgr) MgrClient::_send_report on the rank-0 daemon fills the last field of MMgrReport v9 with MetricReportMessage {u32 type=1, MDSMetricPayload DENC(1,1){MDSPerfMetricReport{map<query, MDSPerfMetrics{counter descriptors, map<key, 16 bytes per counter>}>, set<i32> rank_metrics_delayed}}} for SERVER_PACIFIC peers; missing counters are packed as (0,0). All current keys are re-reported every period with nothing reset. The mgr matches reports to queries by full equality, unpacks counters only when the descriptor order matches, overwrites per-key values, moves them out on each get_mds_perf_counters call, and replaces its single collector-wide delayed_ranks with each report's set. A report type other than 0 or 1 makes the mgr abort (UnknownMetricPayload).

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mgr
- **Map:** [metrics-and-subvolume-metrics.md › 3.8 Rank 0 ↔ mgr: the MDS perf query protocol](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#38-rank-0--mgr-the-mds-perf-query-protocol) · [metrics-and-subvolume-metrics.md › 7.3 ceph-mgr (what an MDS must do to feed the unmodified ...](../cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#73-ceph-mgr-what-an-mds-must-do-to-feed-the-unmodified-mgr)
- **Code:** `src/mds/MetricAggregator.cc:880-914 (MetricAggregator::get_perf_reports)`, `src/mgr/MgrClient.cc:341-453 (MgrClient::_send_report)`, `src/messages/MMgrReport.h:142-190 (MMgrReport)`, `src/mgr/MetricCollector.cc:142-186 (MetricCollector::process_reports_generic)`, `src/mgr/ActivePyModules.cc:1715-1765 (ActivePyModules::get_mds_perf_counters)`
- **Notes:** Each file system's rank 0 reports to the same mgr collector: per-key values merge, delayed_ranks is overwritten by whichever report arrived last, and subvolume keys of equal paths from different file systems collide. The mgr accepts MMgrReport only from daemons whose metadata it has (src/mgr/DaemonServer.cc:804-838).

### Quiesce database and coordination

The cluster-wide part of quiesce: a leader rank chosen by the monitor holds an in-memory database of quiesce sets, replicates it to the other active ranks, aggregates their per-root acknowledgements and answers the quiesce db command.

<a id="f-qos-032"></a>

#### F-QOS-032 · Quiesce DB leader and membership from the MDSMap

*The monitor names the quiesce DB leader and member daemons in every MDSMap, which is how MDS daemons and the mgr find the leader.*

MDSMonitor::assign_quiesce_db_leader runs in encode_pending for every MDSMap changed in the proposal: members are the gids of all daemons that hold a rank and are up:active, and the leader is the one with the lowest rank (MDS_GID_NONE if there is none). The fields qdb_cluster_leader (mds_gid_t) and qdb_cluster_members are encoded in the MDSMap extended encoding ev >= 19 and dumped as qdb_leader and qdb_cluster. On every new map each MDS, in any state, builds its membership (epoch = mdsmap epoch) and participates iff its gid is a member: the mds-q-db thread starts on first participation; a non-member wipes its DB and refuses requests with ENOTTY; a demoted leader completes pending awaits with EINPROGRESS and pending requests with ENOTTY. The mgr reads qdb_leader from the fs map to route fs quiesce.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon, mgr, mds-ranks
- **Map:** [quiesce.md › 3.2 Cluster membership and leader selection](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#32-cluster-membership-and-leader-selection) · [quiesce.md › 5.6 Map fields, compat and vxattr](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#56-map-fields-compat-and-vxattr) · [quiesce.md › 7. Externally visible contract](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#7-externally-visible-contract)
- **Code:** `src/mon/MDSMonitor.cc:188-234 (MDSMonitor::assign_quiesce_db_leader)`, `src/mds/MDSMap.cc:846-847 (MDSMap::encode)`, `src/mds/MDSRankQuiesce.cc:258-370 (MDSRank::quiesce_cluster_update)`, `src/mds/QuiesceDbManager.cc:192-296 (QuiesceDbManager::update_membership)`
- **Notes:** The monitor comment says "lowest rank with the highest state up to ACTIVE", but only up:active ranks become members; up:stopping ranks are not members (their subtrees are not quiesced) and a stopping rank does not make the map degraded (code reading). The membership's fs_id is never set (affects F-QOS-039).

<a id="f-qos-033"></a>

#### F-QOS-033 · MDS compat incompat feature 12 (quiesce subvolumes)

*The quiesce compat bit: a file-system incompat feature an MDS must advertise to serve file systems created with it.*

See F-CLU-014 (compat sets) and F-CLU-015 (standby eligibility). Quiesce-specific: MDS_FEATURE_INCOMPAT_QUIESCE_SUBVOLUMES (incompat 12, "quiesce subvolumes") is in MDSMap::get_compat_set_all and get_compat_set_default, so file systems created at this version carry it and a daemon that does not advertise bit 12 is not assigned a rank in them.

- **Class:** POSIX `operational` · exposure `cluster-wire` · relied on by mon
- **Gates:** MDS incompat compat feature 12
- **Map:** [quiesce.md › 5.6 Map fields, compat and vxattr](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#56-map-fields-compat-and-vxattr) · [quiesce.md › 7. Externally visible contract](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSMap.h:56 (MDS_FEATURE_INCOMPAT_QUIESCE_SUBVOLUMES)`, `src/mds/MDSMap.cc:81 (MDSMap::get_compat_set_all)`, `src/mds/MDSMap.cc:100 (MDSMap::get_compat_set_default)`, `src/mds/FSMap.cc:818 (FSMap::get_available_standby)`
- **Notes:** Compat sets in general belong to 01-cluster-membership-and-ha; this entry records only the quiesce bit.

<a id="f-qos-034"></a>

#### F-QOS-034 · Quiesce sets and request operations

*The leader keeps named quiesce sets of root paths and processes include, exclude, reset, release, query and cancel requests on them.*

The DB is {time_zero, epoch, set_version, sets}; a QuiesceSet has version (the DB set_version of its last change), rstate {state, at_age}, timeout, expiration and members root -> {rstate, excluded}. Members are never deleted (only excluded) and sets are never deleted except by a full DB wipe. A request combines roots_op (INCLUDE_OR_QUERY, EXCLUDE_OR_CANCEL, RESET_OR_RELEASE) with whether roots are given, plus optional set_id, if_version, timeout, expiration and await. Semantics: include or reset of an unknown id creates the set (a random id from one 32-bit mt19937 draw printed with {:x}); mutating or awaiting an unknown id gives ENOENT; a plain query of an unknown id returns 0 with an empty response; cancel without set id cancels every active set; release is allowed only from QUIESCED (EPERM otherwise, no-op on RELEASING or RELEASED); non-reset changes to an inactive set and includes into a RELEASING set give EPERM; reset quiesces the listed roots again and excludes all others. Removing the last included member makes the set CANCELED, and any change of members interrupts pending awaits on the set with EINTR (ECANCELED if the set became CANCELED).

- **Class:** POSIX `extension` · exposure `admin` · relied on by mgr, admin
- **Map:** [quiesce.md › 3.3 The DB data model](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#33-the-db-data-model) · [quiesce.md › 3.5 Requests (QuiesceDbRequest) and their semantics](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#35-requests-quiescedbrequest-and-their-semantics)
- **Code:** `src/mds/QuiesceDb.h:213-312 (QuiesceSet)`, `src/mds/QuiesceDb.h:342-547 (QuiesceDbRequest)`, `src/mds/QuiesceDbManager.cc:584-683 (QuiesceDbManager::leader_process_request)`, `src/mds/QuiesceDbManager.cc:685-843 (QuiesceDbManager::leader_update_set)`
- **Notes:** control.flags (VERBOSE, EXCLUSIVE) exist but are never read. Terminal sets and excluded members accumulate until the DB is wiped and are replicated in full to new peers.

<a id="f-qos-035"></a>

#### F-QOS-035 · Quiesce set and member state machine

*Derives each member's and each set's state from the acknowledgements of all member ranks.*

State values (as encoded): INVALID 0, QUIESCING 1, QUIESCED 2, RELEASING 3, RELEASED 4, EXPIRED 5, FAILED 6, CANCELED 7 and TIMEDOUT 8 (the last two for sets only); aggregation uses min and max, so the order is part of the semantics. For each member the leader takes every peer's (including its own) last reported state, defaulting to the requested state when a peer reported nothing for the root; a reported EXPIRED or FAILED is adopted by the member and the set, a lower effective state rolls the member back (for example QUIESCED to QUIESCING when a new rank joins), and otherwise the member advances only when every peer has acked the current {epoch, set version}. The set follows the minimum member state, except that a RELEASING set counts reports of QUIESCED or RELEASED as RELEASED. Agents are asked for QUIESCING while a set is QUIESCING or QUIESCED and for RELEASING while it is RELEASING.

- **Class:** POSIX `extension` · exposure `admin` · relied on by mgr, admin
- **Map:** [quiesce.md › 3.4 Set state machine](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#34-set-state-machine) · [quiesce.md › 3.6 Leader upkeep: deriving member and set state from acks](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#36-leader-upkeep-deriving-member-and-set-state-from-acks)
- **Code:** `src/mds/QuiesceDb.h:35-108 (QuiesceState)`, `src/mds/QuiesceDb.h:236-305 (QuiesceSet::get_effective_member_state)`, `src/mds/QuiesceDbManager.cc:926-989 (QuiesceDbManager::check_peer_reports)`, `src/mds/QuiesceDbManager.cc:991-1114 (QuiesceDbManager::leader_upkeep_set)`
- **Notes:** A member is QUIESCED only when every member rank reports it quiesced. Every MDSMap epoch change makes all acks stale until each agent acks the new epoch (one round trip per map change). Adding a rank during a quiesce rolls QUIESCED members back and restarts their timeout.

<a id="f-qos-036"></a>

#### F-QOS-036 · Quiesce timeout and expiration

*Bounds how long a set may take to become quiesced and how long it may stay quiesced.*

timeout (per set, applied to each member) runs from the member's inclusion, reset or rollback: a QUIESCING member with at_age + timeout <= now makes the set TIMEDOUT, leaving the member state unchanged. expiration bounds the time in QUIESCED or RELEASING, measured from reaching QUIESCED and restarted by each successful quiesce-await; when it elapses the set becomes EXPIRED, and a release keeps at_age so the expiration clock keeps running. Both default to 0: a new set without a timeout is TIMEDOUT on the first leader pass, and a QUIESCED set with expiration 0 expires right after its awaits complete. All times are ages relative to the leader's time_zero (coarse_real_clock, unsigned 64-bit nanoseconds), and replicas set time_zero = now - db_age from each listing, so clocks need not be synchronized.

- **Class:** POSIX `extension` · exposure `admin` · relied on by mgr, admin
- **Map:** [quiesce.md › 3.3 The DB data model](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#33-the-db-data-model) · [quiesce.md › 3.6 Leader upkeep: deriving member and set state from acks](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#36-leader-upkeep-deriving-member-and-set-state-from-acks) · [quiesce.md › 8. Edge cases, failure handling and gotchas](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/QuiesceDbManager.cc:700-751 (QuiesceDbManager::leader_update_set)`, `src/mds/QuiesceDb.h:244-251 (QuiesceSet)`, `src/mds/QuiesceDb.h:710-726 (interval_saturate_add)`
- **Notes:** --release ignores --timeout and --expiration (they are applied only in the non-release branch), although doc/cephfs/fs-volumes.rst:1740-1743 says any command to an active set may update them. A backwards wall-clock step before time_zero underflows get_age, and TTLs use non-saturating addition (code reading).

<a id="f-qos-037"></a>

#### F-QOS-037 · Quiesce await semantics

*Lets a request wait until its set is QUIESCED (or RELEASED for a release), with a defined error code for every other outcome.*

--await (unbounded) or --await-for <secs> keeps a request pending after its update. A pending quiesce-await completes with 0 when the set is QUIESCED (and then restarts the expiration clock), EPERM on RELEASING or RELEASED, ECANCELED on CANCELED, ETIMEDOUT on EXPIRED or TIMEDOUT, EBADF on FAILED, ENOENT if the set vanished, EINPROGRESS when the await interval elapses or leadership is lost, and EINTR (or ECANCELED) when another request changes the set's members; a release-await completes with 0 on RELEASED. A quiesce-await submitted on a set already beyond QUIESCED returns EPERM at once. Awaits on QUIESCED and RELEASED are completed before the expiration check of the same pass, so with expiration 0 a quiesce-await still succeeds ("barriers").

- **Class:** POSIX `extension` · exposure `admin` · relied on by mgr, admin
- **Map:** [quiesce.md › 3.5 Requests (QuiesceDbRequest) and their semantics](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#35-requests-quiescedbrequest-and-their-semantics) · [quiesce.md › 7. Externally visible contract](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#7-externally-visible-contract)
- **Code:** `src/mds/QuiesceDbManager.cc:661-678 (QuiesceDbManager::leader_process_request)`, `src/mds/QuiesceDbManager.cc:1078-1096 (QuiesceDbManager::leader_upkeep_set)`, `src/mds/QuiesceDbManager.cc:1116-1172 (QuiesceDbManager::leader_upkeep_awaits)`
- **Notes:** --cancel --await returns EPERM after a successful cancel (documented in fs-volumes.rst). Release-await success means every member rank has killed its quiesce ops; parked requests are retried and caps re-issued asynchronously afterwards, so client I/O may resume slightly later.

<a id="f-qos-038"></a>

#### F-QOS-038 · Quiesce if_version optimistic concurrency

*Makes a quiesce request conditional on the set's current version.*

With --if-version N on an existing set the request fails with ESTALE unless N equals the set's version; if_version 0 therefore fails on any existing set and works as create-only. if_version > 0 without a set id gives EINVAL. Every set change sets the set's version to db.set_version + 1 (several changes in one leader pass share a version), and the reply shows the DB set_version and each set's version.

- **Class:** POSIX `extension` · exposure `admin` · relied on by mgr, admin
- **Map:** [quiesce.md › 3.5 Requests (QuiesceDbRequest) and their semantics](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#35-requests-quiescedbrequest-and-their-semantics) · [quiesce.md › 7. Externally visible contract](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#7-externally-visible-contract)
- **Code:** `src/mds/QuiesceDbManager.cc:627-650 (QuiesceDbManager::leader_process_request)`, `src/mds/QuiesceDbManager.cc:685-691 (QuiesceDbManager::leader_update_set)`

<a id="f-qos-039"></a>

#### F-QOS-039 · Quiesce root URI sanitization

*Normalizes quiesce roots to file:/path or inode:N before storing them.*

Each root is parsed as a URI reference and normalized; no scheme means file, and any scheme other than file or inode gives EINVAL. An authority must equal the fs name or the numeric fs id and is then removed; repeated slashes collapse, trailing slashes are dropped, file: paths get a leading /, and inode: paths must be a canonical non-zero decimal uint64. Members are stored and reported in this form (for example file:/volumes/_nogroup/sub1/<uuid>), which is visible in the JSON reply.

- **Class:** POSIX `extension` · exposure `admin` · relied on by mgr, admin
- **Map:** [quiesce.md › 3.5 Requests (QuiesceDbRequest) and their semantics](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#35-requests-quiescedbrequest-and-their-semantics) · [quiesce.md › 8. Edge cases, failure handling and gotchas](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/QuiesceDbManager.cc:494-582 (QuiesceDbManager::sanitize_roots)`, `src/mds/MDSRankQuiesce.cc:509-520 (MDSRank::quiesce_agent_setup)`
- **Notes:** Numeric fs-id authorities are rejected because the MDS never fills in fs_id (only the literal -1 matches). inode: roots are accepted by the DB, but the agent turns them into a relative path with base ino 0, so path_traverse returns -ESTALE and the member becomes FAILED (code reading). Tests: src/test/mds/TestQuiesceDb.cc:620-727 (RootSanitization).

<a id="f-qos-040"></a>

#### F-QOS-040 · Quiesce DB replication between ranks

*The leader replicates set changes to every member rank and collects their per-root acknowledgements.*

MMDSQuiesceDbListing (MSG_MDS_QUIESCE_DB_LISTING 0x505, payload {origin gid, {db_version{epoch u32, set_version u64}, db_age u64 ns, sets}}) goes from the leader to each peer with the sets newer than the peer's last ack, at most once per second per peer and only during leader passes (no idle retransmission). MMDSQuiesceDbAck (0x506, payload {origin gid, diff map tagged with the processed DB version}) goes from each agent to the leader and lists only roots whose actual state differs from the requested one. A replica rejects listings whose epoch differs from its membership epoch (ESTALE) and discards its whole DB on a lower set_version. A new leader, and an existing leader when a member joins, first sends discovery listings (set_version 0, every 1 s) to unknown peers, adopts the highest uploaded DB (with set_version + 1) and processes no request, ack or timer until all of them answered; there is no timeout. All structs are ENCODE_START(1,1) envelopes and are MDS-to-MDS only.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [quiesce.md › 3.7 Replication protocol (leader <-> replicas)](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#37-replication-protocol-leader--replicas) · [quiesce.md › 3.8 Wire encodings of the two internal messages](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#38-wire-encodings-of-the-two-internal-messages) · [quiesce.md › 5.1 Messages](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#51-messages)
- **Code:** `src/mds/QuiesceDbManager.cc:845-924 (QuiesceDbManager::leader_upkeep_db)`, `src/mds/QuiesceDbManager.cc:298-342 (QuiesceDbManager::replica_upkeep)`, `src/mds/QuiesceDbManager.cc:344-408 (QuiesceDbManager::leader_bootstrap)`, `src/mds/QuiesceDbEncoding.h:18-220 (encode)`, `src/messages/MMDSQuiesceDbListing.h:22-58 (MMDSQuiesceDbListing)`
- **Notes:** quiesce_dispatch runs before the stale-message check and the peer-type filter, and the manager trusts the payload origin (only membership is checked). A listing a replica rejected for epoch skew is not requested again; it arrives only with a later leader pass. A silent member blocks a bootstrapping leader until membership changes (code reading).

<a id="f-qos-041"></a>

#### F-QOS-041 · Per-rank quiesce agent

*Each active rank turns the replicated DB into quiesce or release actions on its own cache and reports the actual state per root.*

After each DB change the manager computes a root-centric quiesce map from active sets (root -> {requested state, ttl}; a root in several sets gets the minimum state and maximum ttl; roots of terminal sets are absent, meaning release) and hands it to the agent thread mds-q-agt. For each root the agent submits one quiesce_path request when the root should be quiesced (under mds_lock; with a degraded MDSMap the root fails with EPERM), cancels it with request_kill when the root is releasing or disappears, and reports QUIESCED once the quiesce completed, RELEASED after a successful cancel, EXPIRED when the cancel found no request or the result was lost, and FAILED on an error. Acks are sent asynchronously and are delivered locally on the leader; the reported TTL is never acted on.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks
- **Map:** [quiesce.md › 3.9 The quiesce map given to agents](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#39-the-quiesce-map-given-to-agents) · [quiesce.md › 3.10 The per-rank agent](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#310-the-per-rank-agent)
- **Code:** `src/mds/QuiesceDbManager.cc:1196-1225 (QuiesceDbManager::calculate_quiesce_map)`, `src/mds/QuiesceAgent.cc:33-88 (QuiesceAgent::db_update)`, `src/mds/QuiesceAgent.cc:90-199 (QuiesceAgent::agent_thread_main)`, `src/mds/QuiesceAgent.h:140-161 (TrackedRoot::get_actual_state)`, `src/mds/MDSRankQuiesce.cc:428-621 (MDSRank::quiesce_agent_setup)`
- **Notes:** Agents of different ranks act independently; the only cross-rank coupling is the leader's min-over-ranks aggregation. A root is acked QUIESCED in the pass that cancels it, which the leader maps to RELEASED for a releasing set.

<a id="f-qos-042"></a>

#### F-QOS-042 · Quiesce on degraded file systems and non-active ranks

*Cancels all active sets while the file system is degraded and keeps non-active ranks from quiescing.*

When the MDSMap is degraded (failed or damaged ranks, or any daemon in REPLAY..CLIENTREPLAY) and this MDS is the leader, quiesce_cluster_update injects a cancel-all request at the front of the queue on every map epoch, and agents refuse new quiesces with EPERM (reported as FAILED) until the file system is healthy. A rank that is not up:active replaces its agent callback with a responder that reports QUIESCING roots as FAILED if the map is degraded, QUIESCED if the rank is standby-replay, and otherwise drops them, and resets its agent asynchronously (all roots dropped, all quiesce requests cancelled). Leaving membership wipes the local DB.

- **Class:** POSIX `operational` · exposure `admin` · relied on by mgr, admin
- **Map:** [quiesce.md › 3.2 Cluster membership and leader selection](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#32-cluster-membership-and-leader-selection) · [quiesce.md › 3.17 Release, cancel and teardown](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#317-release-cancel-and-teardown)
- **Code:** `src/mds/MDSRankQuiesce.cc:310-331 (MDSRank::quiesce_cluster_update)`, `src/mds/MDSRankQuiesce.cc:333-364 (MDSRank::quiesce_cluster_update)`, `src/mds/MDSRankQuiesce.cc:513-518 (MDSRank::quiesce_agent_setup)`
- **Notes:** With the current monitor rule non-active ranks are never members, so the responder is dormant except in a short window between reset_agent_callback and update_membership (code reading; not observed in tests).

### Quiescing the inode tree

What one rank does for one quiesced root: internal requests that walk the cached tree, the per-inode quiescelock that parks mutations, cap masking that stops new client writes, and the interplay with fragmentation and migration. Lock types, lock acquisition and caps in general belong to 03-caps-leases-and-locking.

<a id="f-qos-043"></a>

#### F-QOS-043 · Tree quiesce request (quiesce_path)

*An internal request that quiesces everything under one root on this rank and holds the quiesce until it is killed.*

MDCache::quiesce_path starts internal op CEPH_MDS_OP_QUIESCE_PATH (0x01508, name quiesce_path). dispatch_quiesce_path answers -EAGAIN unless the rank is up:active, traverses with DISCOVER, RDLOCK_PATH and WANT_INODE (discovering missing components from their auth ranks and keeping rdlocks on every path dentry), ends with the traverse error (for example -ENOENT, or -ENOTDIR for an intermediate symlink), completes at once for a non-auth root when mds_cache_quiesce_splitauth is false, and otherwise starts a quiesce_inode op for the root. When the root and, transitively, all its cached descendants are quiesced, it completes its finisher with 0 but is not finished: it keeps its path locks and owns all child quiesce_inode ops until request_kill. A failure recorded by any child fails the whole request.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Gates:** mds_cache_quiesce_splitauth (default true)
- **Map:** [quiesce.md › 3.11 Quiescing a tree: quiesce_path](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#311-quiescing-a-tree-quiesce_path) · [quiesce.md › 5.2 Internal request op codes](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#52-internal-request-op-codes)
- **Code:** `src/mds/MDCache.cc:14391-14478 (MDCache::dispatch_quiesce_path)`, `src/mds/MDCache.cc:14480-14495 (MDCache::quiesce_path)`, `src/mds/MDCache.h:548-635 (QuiesceStatistics)`
- **Notes:** Admin entry point for a local tree quiesce: quiesce path (F-ADM-053).

<a id="f-qos-044"></a>

#### F-QOS-044 · Per-inode quiesce request (quiesce_inode)

*An internal request per cached inode that takes the inode's quiesce locks, forces cap revocation and recurses into cached children.*

CEPH_MDS_OP_QUIESCE_INODE (0x01509) acquires with skip_quiesce an xlock on quiescelock and an rdlock on policylock (both kept for the whole quiesce) and, on the auth, a filelock xlock plus authlock, linklock and xattrlock rdlocks; with splitauth (default) the auth drops the cap-related locks after acquisition, without it the rdlocks are kept and a non-auth dirfrag fails with -EPERM. The filelock xlock passes through states that allow only Fc and Fb, revoking Fs, Fr, Fw, Fx and Fl while leaving cached reads and buffered dirty data, and the rdlocks revoke Ax, Lx and Xx, so acquisition waits for clients to acknowledge the revocations. For a directory it starts a child op for every cached head dentry not yet visited and completes only after all children completed (bottom-up); uncached inodes are not walked. An inode already quiesced by another op makes the request return -EINPROGRESS, which fails the path (nested roots in different sets, duplicate quiesce path).

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [quiesce.md › 3.12 Quiescing one inode: quiesce_inode](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#312-quiescing-one-inode-quiesce_inode) · [quiesce.md › 6.2 Serialization points](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#62-serialization-points)
- **Code:** `src/mds/MDCache.cc:14057-14358 (MDCache::dispatch_quiesce_inode)`, `src/mds/locks.c:101-106`, `src/mds/Locker.cc:312-315 (Locker::acquire_locks)`
- **Notes:** QA checks that each completed quiesce_inode holds exactly iquiesce (x) and ipolicy (r), plus the cap-lock rdlocks without splitauth, and that no client holds Ax, Lx, Xx, Fx or Fw (qa/tasks/cephfs/test_quiesce.py:229-318). Every quiesced inode costs one MDRequest plus held locks and cannot be trimmed; replica copies are not auth-pinned. An unresponsive client stalls the quiesce until it is evicted or the set times out.

<a id="f-qos-045"></a>

#### F-QOS-045 · quiescelock and parking of mutations

*A per-inode local lock whose xlock by a quiesce op makes new write and exclusive locks of cap-related locks wait.*

CInode::quiescelock is a local lock of type CEPH_LOCK_IQUIESCE (1<<4, the lowest inode lock type, name iquiesce), not replicated: each rank has its own. A wrlock is allowed only when it is not xlocked and no xlock is waiting (writer preference for the quiesce); an xlock needs no wrlocks. Unless skip_quiesce is passed, Locker::acquire_locks adds a quiescelock wrlock for the inode of the first xlock, wrlock or remote wrlock of a cap-related lock (IAUTH, ILINK, IFILE, IXATTR) in the caller's lock list, at most one per call, sorted before that inode's other locks; rdlocks never add it. If the quiescelock cannot be taken, the request drops all its locks (local and remote) and auth pins and waits (op event "failed to acquire quiesce lock"), so that it does not hold the parent snaplock and policylock rdlocks that mksnap needs. On release, waiters are woken before caps are re-evaluated. skip_quiesce is used by quiesce_inode, lock path, export locking, the quarantine root op and peer OP_XLOCK/OP_WRLOCK from a leader rank.

- **Class:** POSIX `extension` · exposure `internal` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [quiesce.md › 3.13 The quiescelock and how it blocks mutations](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#313-the-quiescelock-and-how-it-blocks-mutations) · [quiesce.md › 3.14 What is blocked and what proceeds while quiesced](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#314-what-is-blocked-and-what-proceeds-while-quiesced) · [lock-types-and-state-machines.md › 3.10 Lock requests that drive transitions; xlock lifecycl...](../cpp-map/05-locking/lock-types-and-state-machines.md#310-lock-requests-that-drive-transitions-xlock-lifecycle-local-locks)
- **Code:** `src/mds/LocalLockC.h:33-39 (LocalLockC::can_wrlock)`, `src/mds/Locker.cc:267-387 (Locker::acquire_locks)`, `src/mds/Locker.cc:671-685 (Locker::handle_quiesce_failure)`, `src/mds/Locker.cc:5668-5689 (Locker::local_xlock_finish)`, `src/mds/Server.cc:3226-3234 (Server::dispatch_peer_request)`
- **Notes:** Blocked while quiesced: create, mknod, mkdir, symlink, link, unlink, rmdir and rename in quiesced directories, setattr, O_TRUNC opens, user xattr changes and vxattrs needing a policylock xlock. Proceeding: getattr, lookup, readdir, open without truncation, mksnap, cap-snap and dirty-data flushes, fcntl/flock locks. Only the first cap-related wr/xlock of a call gets the implicit wrlock, while doc/dev/mds_internals/quiesce.rst says every such lock does; a multi-inode operation whose first cap-related lock is outside the quiesced tree could still wrlock a quiesced directory's filelock (cross-subvolume renames fail with EXDEV, narrowing this; code reading, untested). See also F-CAPS-081 (lock-acquisition details), F-CAPS-066 (local lock type).

<a id="f-qos-046"></a>

#### F-QOS-046 · Cap masking while quiesced

*Limits the caps issued on a quiesced inode to read, cache, buffer and pin, so clients cannot start new writes or take exclusive caps.*

While CInode::is_quiesced() (quiescelock xlocked by a quiesce_inode op), get_caps_quiesce_mask = CEPH_CAP_ANY_RD | Fc | Fb | p (that is p As Ls Xs Fs Fr Fc Fb) is ANDed into the liked, allowed-by-type (hence Locker::get_allowed_caps and InodeStat caps), careful, xlocker and wanted masks, so lock state machines do not move toward EXCL or MIX. Clients keep Fb and can still flush dirty data and cap updates (an Fb holder can still get max_size raised), but writes, fallocate and copy_file_range wait in the client for Fw. Lock-cache cap bits are added only when Fx is allowed and taking the quiescelock xlock invalidates existing lock caches, so kernel async creates and unlinks fall back to synchronous requests, which park.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [quiesce.md › 3.13 The quiescelock and how it blocks mutations](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#313-the-quiescelock-and-how-it-blocks-mutations) · [quiesce.md › 7. Externally visible contract](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#7-externally-visible-contract)
- **Code:** `src/mds/CInode.cc:3726-3744 (CInode::get_caps_quiesce_mask)`, `src/mds/CInode.cc:3761-3796 (CInode::get_caps_allowed_by_type)`, `src/mds/CInode.cc:3900-3905 (CInode::get_caps_wanted)`, `src/mds/Locker.cc:5654-5656 (Locker::local_xlock_start)`, `src/mds/CInode.cc:3818-3821 (CInode::get_caps_allowed_for_client)`
- **Notes:** An xlock taken by "lock path quiesce:x" blocks mutations but does not mask caps. Kernel ceph_write_iter needs Fw (linux/fs/ceph/file.c:2477); libcephfs Client::_write waits in Client::get_caps; for NFS-Ganesha, worker threads block (inferred). Revoking Fw while keeping Fb avoids forced write-back, which the code comment says could make the quiesce time out.

<a id="f-qos-047"></a>

#### F-QOS-047 · Quiescing inodes that appear during a quiesce

*Quiesces inodes that are linked into an already-quiesced directory after the walk passed it.*

CDir::link_inode_work, run whenever a primary inode is linked into a dirfrag (dirfrag fetch, replica decode, import, rejoin, projected-linkage application), calls MDCache::add_quiesce if the directory inode is quiesced; add_quiesce starts a quiesce_inode op for the head child under the same quiesce_path request, with a no-op finisher outside any gather. For example, find on a quiesced subvolume after a cache drop quiesces every loaded inode (qa/tasks/cephfs/test_quiesce.py:579-612).

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [quiesce.md › 3.15 New inodes during a quiesce and interplay with fragm...](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#315-new-inodes-during-a-quiesce-and-interplay-with-fragmentation)
- **Code:** `src/mds/CDir.cc:654-657 (CDir::link_inode_work)`, `src/mds/MDCache.cc:14360-14389 (MDCache::add_quiesce)`

<a id="f-qos-048"></a>

#### F-QOS-048 · Quiesce and directory fragmentation

*Keeps fragmentation from deadlocking with or escaping a quiesce.*

MDCache::can_fragment refuses split and merge of a quiesced directory. For each dirfrag a quiesce_inode walks, and whenever the quiesce_path traverse must wait, quiesce_overdrive_fragmenting_async aborts a live fragment operation on that dirfrag that is not yet fully frozen; dispatch_fragment_dir then requeues the split or merge, unfreezes and finishes. dispatch_fragment_dir also aborts itself if the directory inode would block on the quiescelock before it takes its locks, because blocking with frozen dirfrags would deadlock with quiesce ops waiting on them. Quiesced auth inodes keep their auth pins, so freezes that cover them cannot complete until release.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [quiesce.md › 3.15 New inodes during a quiesce and interplay with fragm...](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#315-new-inodes-during-a-quiesce-and-interplay-with-fragmentation)
- **Code:** `src/mds/MDCache.cc:14019-14055 (MDCache::quiesce_overdrive_fragmenting_async)`, `src/mds/MDCache.cc:11893-11896 (MDCache::can_fragment)`, `src/mds/MDCache.cc:12310-12320 (MDCache::dispatch_fragment_dir)`
- **Notes:** QA exercises merge/split churn during quiesce (qa/tasks/cephfs/test_quiesce.py:614-650). Fragmentation itself belongs to 05-metadata-cache-and-dirfrags. See also F-CACHE-093 (fragment op cancellation).

<a id="f-qos-049"></a>

#### F-QOS-049 · Quiesce and subtree migration

*Refuses or cancels subtree exports that touch a quiesced tree.*

Migrator::export_dir refuses dirfrags whose inode is quiesced; quiesce_overdrive_export, called for every dirfrag a quiesce_inode walks, cancels an export of that dirfrag that is still at or before EXPORT_FREEZING; dispatch_export_dir cancels if the directory would block on the quiescelock (its own acquire_locks uses skip_quiesce). On the importer, path_traverse with MDS_TRAVERSE_IMPORT returns -EAGAIN instead of discovering below a quiesced non-auth directory and the importer NAKs the discover, so the exporter cancels; imported inodes under a quiesced parent are quiesced through add_quiesce. request_forward of an internal op completes it with -EXDEV.

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks
- **Map:** [quiesce.md › 3.16 Interplay with subtree migration](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#316-interplay-with-subtree-migration)
- **Code:** `src/mds/Migrator.cc:320-332 (Migrator::quiesce_overdrive_export)`, `src/mds/Migrator.cc:954-956 (Migrator::export_dir)`, `src/mds/Migrator.cc:1207-1219 (Migrator::dispatch_export_dir)`, `src/mds/Migrator.cc:2484-2495 (Migrator::handle_export_discover)`, `src/mds/MDCache.cc:8574-8577 (MDCache::path_traverse)`
- **Notes:** If the importer already has the base inode cached, no quiesce check is made. Subtree migration belongs to 07-multi-mds. See also F-MULTI-025.

<a id="f-qos-050"></a>

#### F-QOS-050 · Quiesce release and cancel cascade

*Releases a root by killing its quiesce_path request, which kills every child op and drops their locks.*

The agent's cancel_request (under mds_lock) calls MDCache::request_kill on the quiesce_path request if it still exists and returns 0, otherwise ENOENT (the root is then reported EXPIRED instead of RELEASED). request_kill completes a still-pending finisher with -ECANCELED, and request_cleanup kills every op in the request's quiesce_ops (resetting the heartbeat every mds_heartbeat_reset_grace ops) and drops the path locks; each killed quiesce_inode drops its quiescelock xlock and policylock rdlock, which wakes parked requests and re-issues caps. A C_MDS_QuiescePath destroyed without finishing completes its finisher with -ECANCELED. On rank shutdown the agent is shut down with mds_lock released and the manager joins its thread.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [quiesce.md › 3.17 Release, cancel and teardown](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#317-release-cancel-and-teardown)
- **Code:** `src/mds/MDCache.cc:10155-10212 (MDCache::request_kill)`, `src/mds/MDCache.cc:10064-10153 (MDCache::request_cleanup)`, `src/mds/MDSRankQuiesce.cc:581-608 (MDSRank::quiesce_agent_setup)`, `src/mds/MDCache.h:610-615 (C_MDS_QuiescePath)`

<a id="f-qos-051"></a>

#### F-QOS-051 · Quiesce dispatch throttle and heartbeat resets

*Rate-limits quiesce_inode dispatch and keeps the daemon heartbeat alive during large walks.*

Before taking locks each quiesce_inode checks quiesce_counter, a DecayCounter with half-life mds_cache_quiesce_decay_rate (1 s): above mds_cache_quiesce_threshold (512K) it retries after mds_cache_quiesce_sleep (200 ms) on the timer, otherwise it records a hit. mds_cache_quiesce_delay (dev, 0 ms) delays the root op and each child op, and heartbeat_reset is called every mds_heartbeat_reset_grace (1000) dispatched or killed quiesce ops. The three throttle options are applied at runtime through MDCache::handle_conf_change; splitauth and delay are read on every dispatch_quiesce_path. The DB protocol uses hard-coded constants: discovery interval 1 s (+100 ms), repeated-update interval 1 s, maximum db thread wait 10 s.

- **Class:** POSIX `operational` · exposure `config` · relied on by admin
- **Gates:** mds_cache_quiesce_threshold; mds_cache_quiesce_sleep; mds_cache_quiesce_decay_rate; mds_cache_quiesce_delay; mds_cache_quiesce_splitauth; mds_heartbeat_reset_grace
- **Map:** [quiesce.md › 5.4 Configuration options](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#54-configuration-options) · [quiesce.md › 3.12 Quiescing one inode: quiesce_inode](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#312-quiescing-one-inode-quiesce_inode)
- **Code:** `src/mds/MDCache.cc:14166-14174 (MDCache::dispatch_quiesce_inode)`, `src/common/options/mds.yaml.in:77-94 (mds_cache_quiesce_delay)`, `src/common/options/mds.yaml.in:167-193 (mds_cache_quiesce_decay_rate)`, `src/mds/MDCache.cc:264-272 (MDCache::handle_conf_change)`

<a id="f-qos-052"></a>

#### F-QOS-052 · ceph.quiesce.block vxattr

*A persistent per-inode flag that exempts an inode, and for a directory its whole subtree, from quiesce.*

Setting ceph.quiesce.block (a bool) requires MAY_SET_VXATTR, xlocks policylock and snaplock, projects inode_t::flags |= F_QUIESCE_BLOCK (1<<1) and journals it without early reply; the flag is persisted with the inode (inode_t v16+ flags) and is read back with getvxattr. A quiesce_inode op that finds the flag drops its locks but keeps its auth pins, counts inodes_blocked, completes with 0 and does not recurse, so a blocked directory exempts its whole subtree. Setting the flag waits for any active quiesce of the inode (policylock xlock versus the quiesce's rdlock). Clients reach it through the generic ceph.* xattr passthrough (kernel SETXATTR and GETVXATTR).

- **Class:** POSIX `extension` · exposure `client-wire`, `on-disk` · relied on by kclient, libcephfs, ceph-fuse, admin
- **Map:** [quiesce.md › 3.18 Admin and management interface](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#318-admin-and-management-interface) · [quiesce.md › 5.6 Map fields, compat and vxattr](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#56-map-fields-compat-and-vxattr)
- **Code:** `src/mds/Server.cc:6500-6545 (Server::handle_client_setvxattr)`, `src/mds/Server.cc:7443-7444 (Server::handle_client_getvxattr)`, `src/include/cephfs/types.h:783 (F_QUIESCE_BLOCK)`, `src/mds/MDCache.cc:14246-14256 (MDCache::dispatch_quiesce_inode)`, `linux/fs/ceph/xattr.c:1208-1210 (__ceph_setxattr)`
- **Notes:** Shown as quiesce_block in inode dumps and qblock in inode prints. Virtual xattr handling in general belongs to 04-namespace-and-posix-operations.

### Quiesce admin interfaces

Commands that drive or inspect quiesce: quiesce db (used by ceph fs quiesce), the debug commands quiesce path and lock path, and the observability that exists for quiesce.

<a id="f-qos-053"></a>

#### F-QOS-053 · quiesce db command

*The admin and mgr entry point to the quiesce DB; the leader answers with a JSON listing of sets.*

"quiesce db [<roots>...] [--set-id] [--timeout] [--expiration] [--await-for|--await] [--if-version] [--include|--exclude|--reset|--release|--query|--cancel] [--all]" is registered for asok and tell (tell needs allow *) and runs without mds_lock. More than one op, release or cancel with roots, cancel without --set-id or --all, reset without set id or roots, and query with roots give -EINVAL; no op means include (query when no roots are given); durations are float seconds. The reply is always json-pretty {epoch, leader, set_version, sets: {<id>: {version, age_ref, state{name, age}, timeout, expiration, members{<root>: {excluded, state{name, age}}}}}}, holding the requested set (also on error), all sets for a query without id, nothing for cancel-all, and no inactive sets other than the requested one unless --all. A non-member or non-leader answers -ENOTTY with {epoch, leader} from its MDSMap so that the caller can retry at the leader; otherwise the return code is the negated errno with an empty status string.

- **Class:** POSIX `extension` · exposure `admin` · relied on by mgr, admin, tools
- **Map:** [quiesce.md › 3.18 Admin and management interface](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#318-admin-and-management-interface) · [quiesce.md › 5.3 Admin commands](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#53-admin-commands) · [quiesce.md › 7. Externally visible contract](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#7-externally-visible-contract)
- **Code:** `src/mds/MDSRankQuiesce.cc:49-242 (MDSRank::command_quiesce_db)`, `src/mds/MDSDaemon.cc:549-566 (MDSDaemon::set_up_admin_socket)`, `src/mds/QuiesceDbManager.cc:443-452 (QuiesceDbManager::complete_requests)`
- **Notes:** After a demotion the reply's leader is the answering MDS itself, so the mgr retry returns to the same MDS, which then answers with the map's leader (bounded by 5 retries). The JSON layout, state names, member key format (file:/path), set id format and error codes are user-visible API documented in doc/cephfs/fs-volumes.rst:1625-2033. Without a quiesce DB manager the command returns -EFAULT. Error codes returned in the reply as negated errnos: EINVAL, ENOENT, EPERM, ESTALE, EINPROGRESS, ETIMEDOUT, ECANCELED, EINTR, EBADF, ENOTTY; the leader replies from thread mds-q-db. Command plane: F-ADM-054; mgr/volumes caller: F-ADM-101. See also F-ADM-101 (mgr/volumes fs quiesce).

<a id="f-qos-057"></a>

#### F-QOS-057 · Quiesce observability: counters and op events

*Exposes quiesce progress through two request counters and op events in dump_ops_in_flight.*

The mds_cache perf counters ireq_quiesce_path and ireq_quiesce_inode count finished or killed quiesce_path and quiesce_inode requests. dump_ops_in_flight shows quiesce ops with flag points such as "quiescing root", "quiescing children", "quiesce complete", "quiesce complete for non-auth inode", "quiesce complete for non-auth tree", "quiesce blocked" and "failed to acquire quiesce lock", plus the locks they hold; QA asserts on four of them (qa/tasks/cephfs/test_quiesce.py:124-127). There are no gauges for active sets, quiesced inodes or parked requests and no health warnings; the log subsystem is mds_quiesce.

- **Class:** POSIX `operational` · exposure `metrics`, `admin` · relied on by admin, tools
- **Map:** [quiesce.md › 5.5 Perf counters](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#55-perf-counters) · [quiesce.md › 5.2 Internal request op codes](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#52-internal-request-op-codes)
- **Code:** `src/mds/MDCache.cc:13935-13938 (MDCache::register_perfcounters)`, `src/mds/MDCache.cc:10186-10190 (MDCache::request_kill)`, `src/mds/MDCache.cc:9940-9942 (MDCache::request_finish)`

### Quarantine control

Turning quarantine on and off for one subvolume root: commands, the persisted flag, the in-progress state, multi-rank propagation and failure handling. Quarantine reuses the quiesce tree walk in a lock-free mode.

<a id="f-qos-058"></a>

#### F-QOS-058 · quarantine enable and disable commands

*Admin commands that mark or unmark one subvolume root as quarantined and reply when the operation has completed.*

"quarantine enable path=<abs path>" and "quarantine disable path=<abs path>" (asok and tell; tell needs allow *) start internal op CEPH_MDS_OP_QUARANTINEDIR_AUTH, which resolves the path wanting auth and requires a subvolume root (a directory whose snaprealm's subvolume ino is itself: v2 subvolume roots, not <uuid> data directories, groups or files). The reply comes after the root's flag change is journaled and the local cap walk is done: JSON {return_code, operation: "enable"|"disable", status: "successful"|"failed"[, error]} with result 0 (done, or already in the requested state with nothing journaled), -EINVAL (not a subvolume root), -EBUSY (another operation on the root in flight), -ENOENT (path missing or root vanished), -EXDEV (this rank is not auth for the root; there is no forwarding) or -ECANCELED (op killed). A daemon without a rank returns mds_not_active.

- **Class:** POSIX `extension` · exposure `admin` · relied on by mgr, admin, tools
- **Map:** [quarantine.md › 3.3 Admin entry points](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#33-admin-entry-points) · [quarantine.md › 5.1 Admin and mgr commands](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#51-admin-and-mgr-commands)
- **Code:** `src/mds/MDSDaemon.cc:568-577 (MDSDaemon::set_up_admin_socket)`, `src/mds/MDSRankQuarantine.cc:46-94 (MDSRank::command_quarantine_dir)`, `src/mds/MDCache.cc:15100-15187 (MDCache::quarantine_dir_auth)`
- **Notes:** A relative or empty path resolves through base ino 0 and is expected to end in -ESTALE after a RADOS round trip (unverified at runtime). A rank that is not active (replay, standby-replay, rejoin) also starts the operation and would assert in MDLog if it reached the journal (inferred, untested). Introduced May-Oct 2026 (44 commits). The internal op code is CEPH_MDS_OP_QUARANTINEDIR_AUTH (0x150c); the command runs under mds_lock and its rc equals return_code. The inode's authority follows the dirfrag holding its dentry, so -EXDEV comes from the rank that is not auth for that dirfrag. Command plane: F-ADM-055; mgr/volumes caller: F-ADM-102. See also F-ADM-102 (mgr/volumes quarantine command).

<a id="f-qos-060"></a>

#### F-QOS-060 · Persistent quarantine flag in inode optmetadata

*Stores quarantine state as an optmetadata entry of kind QUARANTINE on the subvolume root inode, journaled with an EUpdate.*

The flag is an inode_t::optmetadata entry of kind QUARANTINE = 2 holding quarantine_md_t (ENCODE_START(1,1): bool enabled, u64 quarantined_at_sec, u32 quarantined_at_nsec); has_quarantined() means the entry exists with enabled true, and disable removes the entry. The optmetadata container is ENCODE_START(1,1) plus a vector of {u64 kind, payload in its own envelope} kept sorted by kind, with kinds above 3 kept as opaque blobs; it is carried in inode_t encoding v20+, InodeStat v8+ and policylock payload v4. MDCache::quarantine_inode xlocks the root's authlock, linklock, xattrlock, filelock, snaplock and policylock (skip_quiesce), projects set_quarantine or del_quarantine together with change_attr++, ctime = now and an rctime update, and journals EUpdate "quarantine flag add" or "quarantine flag del" (root inode plus its primary dentry) without early reply. The flag survives restart and failover through ordinary metablob replay; there is no FSMap or MDSMap state, no RADOS object of its own, no config option and no perf counter.

- **Class:** POSIX `extension` · exposure `on-disk` · relied on by mds-ranks, internal
- **Map:** [quarantine.md › 3.1 The three layers of state](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#31-the-three-layers-of-state) · [quarantine.md › 4. Data structures and invariants](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#4-data-structures-and-invariants) · [quarantine.md › 5.3 Journal events and RADOS objects](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#53-journal-events-and-rados-objects)
- **Code:** `src/include/cephfs/types.h:489-529 (quarantine_md_t)`, `src/include/cephfs/types.h:915-929 (inode_t::has_quarantined)`, `src/mds/MDCache.cc:15189-15275 (MDCache::quarantine_inode)`, `src/mds/Server.cc:5403-5416 (Server::journal_quarantine_inode)`, `src/include/cephfs/types.h:580-589 (optmetadata_singleton::get_kind)`
- **Notes:** optmetadata kind 3 (equal to _MAX) is mishandled: get_kind tests > instead of >=, indexing one past the table (undefined behaviour, code reading). Enable and disable change the root's ctime, change_attr (NFSv4 change attribute) and rctime. The real quarantined_at is visible only through dump inode. The optmetadata container is shared with charmap (kind 1); inode encoding belongs to 05-metadata-cache-and-dirfrags and 06-journal-and-storage.

<a id="f-qos-061"></a>

#### F-QOS-061 · In-progress quarantine state and operation ordering

*Marks the subvolume root in memory as being quarantined or unquarantined, so that enforcement starts or stops before the flag is journaled.*

After validation the auth rank sets CInode::quarantine_op on the subvolume root (NONE 0, ADD 1, DEL 2); the journal finisher or an abort clears it. The enforcement predicate treats ADD as quarantined and DEL as not quarantined, so enforcement starts when an enable is accepted and stops when a disable is accepted, before the EUpdate is safe; a crash in between reverts to the persisted state. The operation then queues a QUARANTINEDIR_WORK op on the finisher, which queues the QUARANTINE_INODE op for the root (flag change and journal) and starts the cap-revocation walk; the command replies only after both complete. Replica ranks never see quarantine_op.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [quarantine.md › 3.1 The three layers of state](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#31-the-three-layers-of-state) · [quarantine.md › 3.4 Enable / disable state machine (auth rank)](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#34-enable--disable-state-machine-auth-rank) · [quarantine.md › 3.5 Step-by-step flow on the auth rank](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#35-step-by-step-flow-on-the-auth-rank)
- **Code:** `src/mds/CInode.h:687-705 (CInode::is_being_quarantined)`, `src/mds/MDCache.cc:15175 (MDCache::quarantine_dir_auth)`, `src/mds/MDCache.cc:15054-15098 (MDCache::quarantine_work)`, `src/mds/Server.cc:5386-5400 (C_MDS_quarantine_inode_update_finish::finish)`

<a id="f-qos-062"></a>

#### F-QOS-062 · Per-subvolume quarantine operation registry

*Allows at most one enable or disable per subvolume root at a time on a rank and rejects concurrent ones with EBUSY.*

MDSRank::qtine_ops maps subvolume root inode numbers to a QuarantineTracker under qtine_mutex; register_quarantine_mgr fails with -EBUSY while a non-complete tracker exists and replaces a complete one. The tracker counts two work items (root flag journal, local walk) with add_work, seal and work_done (which fires the callback with 0 once sealed and pending reaches 0) and complete(r) (which fires it at once); the callback fires at most once, unregisters the root, undoes on error and sends the JSON reply. The internal ops are created "continuous", which exempts them from slow-op warnings, and print as ??? in dump_ops_in_flight.

- **Class:** POSIX `internal` · exposure `admin` · relied on by admin, internal
- **Map:** [quarantine.md › 3.5 Step-by-step flow on the auth rank](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#35-step-by-step-flow-on-the-auth-rank) · [quarantine.md › 3.11 Failure handling and undo](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#311-failure-handling-and-undo)
- **Code:** `src/mds/QuarantineManager.h:34-132 (QuarantineTracker)`, `src/mds/MDSRankQuarantine.cc:96-127 (MDSRank::register_quarantine_mgr)`, `src/mds/MDCache.cc:9882-9890 (MDCache::request_start_internal)`
- **Notes:** Killing the QUARANTINE_INODE op with op kill leaves the command unanswered, the registry entry busy (-EBUSY for that subvolume until restart) and quarantine_op set. The same happens if the root stops being auth on this rank between validation and quarantine_inode (inferred, untested race). The tracker refcount and statistics are never read; QuarantineManager.cc is empty.

<a id="f-qos-063"></a>

#### F-QOS-063 · Quarantine undo on failure

*Clears the in-progress mark and wakes parked requests when an enable or disable fails.*

abort_quarantine_op(ino, op) runs from the tracker callback when r < 0 and the tracker was registered: it clears quarantine_op if it still equals op and calls finish_waiting(WAIT_QUARANTINE) on the root so that parked requests retry. Failures before registration (missing path, not a subvolume root, -EBUSY, -EXDEV) have nothing to undo; a root or tracker that vanished in later steps completes with -ENOENT and aborts. A failed or cancelled walk (for example -EAGAIN in up:clientreplay) is ignored: the command reports success without proactive cap revocation. The journal write cannot fail in this path (C_MDS_inode_update_finish asserts r == 0).

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [quarantine.md › 3.11 Failure handling and undo](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#311-failure-handling-and-undo)
- **Code:** `src/mds/MDCache.cc:15039-15052 (MDCache::abort_quarantine_op)`, `src/mds/MDSRankQuarantine.cc:69-91 (MDSRank::command_quarantine_dir)`
- **Notes:** Requests that parked in rdlock_path_pin_ref or rdlock_path_xlock_dentry are not re-checked when woken (PATH_LOCKED), so a wake-up by the abort of a failed disable lets them proceed although the flag is still set.

<a id="f-qos-064"></a>

#### F-QOS-064 · Quarantine cap-revocation walk

*Walks the subvolume tree on each rank and re-evaluates caps on every cached inode, so that the restriction takes effect, or is lifted, at once.*

start_quarantine_cap_revocation starts a quiesce_path request on filepath(root ino) in quarantine mode (a non-auth root is walked regardless of splitauth). Each quiesce_inode in quarantine mode takes no locks and skips the quiesce throttle: unless the inode is the .meta file directly under the subvolume root, it runs Locker::eval and, if that issued nothing, Locker::issue_caps, which recomputes caps with the quarantine mask; on disable it also sends MQuarantineDisable to quarantine-aware cap holders and wakes WAIT_QUARANTINE waiters; for a directory it recurses into auth dirfrags only. When the root op finishes, the walk's quiesce_path request is killed from the finisher; the auth's completion ignores the result. On the auth the walk starts before the flag is journaled.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [quarantine.md › 3.6 The cap-revocation walk (quiesce machinery in quarant...](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#36-the-cap-revocation-walk-quiesce-machinery-in-quarantine-mode) · [quiesce.md › 3.11 Quiescing a tree: quiesce_path](../cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#311-quiescing-a-tree-quiesce_path)
- **Code:** `src/mds/MDCache.cc:6499-6524 (MDCache::start_quarantine_cap_revocation)`, `src/mds/MDCache.cc:14082-14162 (MDCache::dispatch_quiesce_inode)`, `src/mds/MDCache.cc:15022-15037 (MDCache::schedule_quarantine_cleanup)`, `src/mds/MDCache.h:606-635 (C_MDS_QuiescePath)`
- **Notes:** Coverage gap: each rank descends only through dirfrags it is auth for, so inodes in a subtree delegated to another rank below a dirfrag of this rank are visited by neither walk; their caps are masked only at their next issue_caps (requests are still gated). Directories with visited children are processed twice (second eval and second MQuarantineDisable). A fully cached tree is typically walked recursively inside one mds_lock hold (inferred).

<a id="f-qos-065"></a>

#### F-QOS-065 · Quarantine propagation to replica ranks

*Carries the quarantine flag to other ranks through ordinary policylock replication and starts a local walk there.*

quarantine_inode xlocks replicated locks, so the auth sends LOCK_AC_LOCK and gathers LOCKACKs (a replica acks only after its local rdlocks on the lock are released); for snaplock and policylock xlocks while the cluster is degraded with a replica rank below active, acquisition waits for full recovery. Releasing the xlock sends LOCK_AC_SYNC with the policylock payload v4, which includes optmetadata; the replica's decode_lock_ipolicy detects a change of has_quarantined on a subvolume root and calls handle_quarantine_policy_update, which starts a local walk whose completion nobody waits for. The auth replies without waiting for replicas, and a replica serves the subvolume normally until it processes LOCK_AC_SYNC.

- **Class:** POSIX `internal` · exposure `mds-wire` · relied on by mds-ranks
- **Map:** [quarantine.md › 3.9 Multi-rank behavior](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#39-multi-rank-behavior)
- **Code:** `src/mds/CInode.cc:2250-2265 (CInode::encode_lock_ipolicy)`, `src/mds/CInode.cc:2269-2313 (CInode::decode_lock_ipolicy)`, `src/mds/MDCache.cc:6526-6540 (MDCache::handle_quarantine_policy_update)`, `src/mds/Locker.cc:275-306 (Locker::acquire_locks)`
- **Notes:** This replaced dedicated MMDSQuarantine messages, EQuarantineStart/Finish/Cancel events and op 0x0150d (commit 24d79f3a038). Requests parked on a replica hold rdlocks on its root snaplock and are woken only by that replica's DEL walk, which needs the auth's xlock, so a disable can wedge until those clients are evicted (inferred, untested).

<a id="f-qos-066"></a>

#### F-QOS-066 · Subtree export refused inside a quarantined subvolume

*Prevents migrating subtrees rooted at or inside a quarantined subvolume.*

Migrator::export_dir silently refuses an export whose root dirfrag's inode is under quarantine (the subvolume root's own dirfrags and every dirfrag inside), also during up:stopping, so balancer exports, export pins and rank shutdown of such subtrees wait until the quarantine is lifted. An export rooted at an ancestor dirfrag (for example /volumes/<group>) is not refused and carries the subvolume root and nested dirfrags along; the persisted flag travels with the inode, the in-memory quarantine_op does not (inferred, untested).

- **Class:** POSIX `internal` · exposure `internal` · relied on by mds-ranks, admin
- **Map:** [quarantine.md › 3.9 Multi-rank behavior](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#39-multi-rank-behavior) · [quarantine.md › 8. Edge cases, failure handling and gotchas](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Migrator.cc:957-959 (Migrator::export_dir)`
- **Notes:** QA tear-down disables quarantine and removes export pins before reducing max_mds (qa/tasks/cephfs/test_quarantine.py:1105-1155). See also F-MULTI-025.

<a id="f-qos-067"></a>

#### F-QOS-067 · Quarantine re-enforcement after restart

*Applies the quarantine cap mask again to all reconnected caps when a rank becomes active.*

The persisted flag is restored by replaying the EUpdate metablob or loaded from the parent dirfrag; quarantine_op and the operation registry start empty, and there is no quarantine-specific replay code. MDSRank::active_start calls MDCache::reissue_all_caps, which runs eval and issue_caps for every inode with caps and so applies the quarantine mask to caps that clients reconnected with. Standby-replay daemons replay the same EUpdate. An operation interrupted before its EUpdate was safe leaves the flag unchanged and has to be retried by the administrator.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [quarantine.md › 3.10 Restart, failover and replay](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#310-restart-failover-and-replay)
- **Code:** `src/mds/MDCache.cc:6351-6373 (MDCache::reissue_all_caps)`, `src/mds/MDSRank.cc:2159 (MDSRank::active_start)`

### Quarantine enforcement

How a quarantined subvolume is enforced per client session: recovery clients are those with q/Q MDS caps; quarantine-aware clients (feature bit 24) get EACCES, unaware clients (the kernel client) have their requests parked; all non-recovery sessions are reduced to the PIN cap.

<a id="f-qos-068"></a>

#### F-QOS-068 · Quarantine enforcement predicate

*Decides for any inode whether it lies inside a quarantined subvolume.*

CInode::is_under_quarantine walks the snaprealm chain from the inode: for each realm's subvolume ino (the outermost enclosing subvolume), if that inode is cached it returns false when a disable is in flight, and true when an enable is in flight or the persisted flag is set; if the subvolume root is not cached it returns false (fail-open); otherwise it continues with the parent realm and returns false at the root realm. Coverage is every inode whose realm chain reaches the root, including snapshot views (.snap) at any depth. It runs on hot paths (every check_access, every Locker::issue_caps, which also builds the inode's full path to check q/Q, every encode_inodestat and loner changes) whether or not anything is quarantined.

- **Class:** POSIX `internal` · exposure `internal` · relied on by internal
- **Map:** [quarantine.md › 3.2 The enforcement predicate CInode::is_under_quarantine()](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#32-the-enforcement-predicate-cinodeis_under_quarantine)
- **Code:** `src/mds/CInode.cc:5913-5944 (CInode::is_under_quarantine)`, `src/mds/SnapRealm.cc:150-154 (SnapRealm::check_cache)`
- **Notes:** doc/cephfs/fs-volumes.rst:1620-1622 says an uncached root fails closed; the code fails open (commit 630bf842c11). In practice the root is cached whenever a descendant is. Unlinked-but-open files in stray directories are outside the realm chain and not covered (inferred). Removing ceph.dir.subvolume from a quarantined root silently stops enforcement, and disable returns -EINVAL until the flag is set again.

<a id="f-qos-069"></a>

#### F-QOS-069 · MDS auth cap flags q and Q (quarantine access)

*Quarantine use of the q and Q MDS cap flags.*

See F-ADM-015 for the flags. Quarantine-specific: the flags are consulted in two places, the per-request check for quarantine-aware sessions (F-QOS-071) and the cap restriction on every cap issue (F-QOS-074); neither is implied by allow *, so recovery clients need an explicit q or Q grant.

- **Class:** POSIX `extension` · exposure `cluster-wire`, `admin` · relied on by mon, libcephfs, ceph-fuse, ganesha, admin
- **Map:** [quarantine.md › 4. Data structures and invariants](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#4-data-structures-and-invariants) · [quarantine.md › 7.2 Monitor](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#72-monitor) · [auth-and-mds-caps.md › 3.4 The MDS cap language](../cpp-map/01-cluster-integration/auth-and-mds-caps.md#34-the-mds-cap-language)
- **Code:** `src/mds/MDSAuthCaps.h:53-56 (MDSCapSpec)`, `src/mds/MDSAuthCaps.cc:121-137 (MDSCapParser)`, `src/mds/MDSAuthCaps.h:324-355 (MDSAuthCaps::quarantine_access_in_caps)`, `src/mds/MDSAuthCaps.cc:266-405 (MDSAuthCaps::is_capable)`, `src/mon/AuthMonitor.cc:1491`
- **Notes:** The docs describe q as one quarantined path and Q as all; the code evaluates both identically, scoped by the grant's path and fsname.

<a id="f-qos-070"></a>

#### F-QOS-070 · Quarantine client feature bit 24

*Client feature bit that selects between EACCES enforcement (aware clients) and request parking (unaware clients).*

CEPHFS_FEATURE_QUARANTINE = 24, named "quarantine", is in CEPHFS_FEATURES_ALL, so the MDS and this release's libcephfs, ceph-fuse and NFS-Ganesha advertise it. The Linux kernel client snapshot neither defines nor advertises bit 24 and is always treated as unaware. The bit also decides who receives MQuarantineDisable, and its name is accepted wherever CephFS feature names are, for example fs required_client_features.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha, mon
- **Gates:** CEPHFS_FEATURE_QUARANTINE (session feature bit 24)
- **Map:** [quarantine.md › 5.4 Config options, perf counters, feature bits, error codes](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#54-config-options-perf-counters-feature-bits-error-codes) · [quarantine.md › 7.3 Linux kernel client (never quarantine-aware)](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#73-linux-kernel-client-never-quarantine-aware)
- **Code:** `src/mds/cephfs_features.h:55 (CEPHFS_FEATURE_QUARANTINE)`, `src/mds/cephfs_features.cc:39 (feature_names)`, `linux/fs/ceph/mds_client.h:25-61`
- **Notes:** Older libcephfs, ceph-fuse and Ganesha builds without bit 24 behave like the kernel client (their requests are parked). See also F-PROTO-021 (per-bit gated behaviour).

<a id="f-qos-071"></a>

#### F-QOS-071 · EACCES for quarantine-aware sessions

*Fails every metadata request of a quarantine-aware client without q/Q on an inode under quarantine with -EACCES.*

Server::check_access sets check_qtine when the inode is under quarantine and the session has feature bit 24, and Session::check_access passes it to MDSAuthCaps::is_capable, whose false result becomes -EACCES. check_access runs in essentially every request handler on the target and/or parent inode (with mask 0 for lookups by ino), so all metadata operations of such clients inside the subvolume fail, including snapshot listing and a mount at a path inside it. Batched getattr and lookup followers skip the checks and inherit the batch head's result: success if the head has q, -EACCES (even for a q follower) if the head is an aware client without q, and an indefinite wait if the head is a parked unaware request.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by libcephfs, ceph-fuse, ganesha, mgr
- **Map:** [quarantine.md › 3.7 Enforcement on client requests](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#37-enforcement-on-client-requests) · [quarantine.md › 7.4 libcephfs, ceph-fuse and NFS-Ganesha](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#74-libcephfs-ceph-fuse-and-nfs-ganesha) · [quarantine.md › 3.12 How the C++ userspace client consumes the signals (c...](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#312-how-the-c-userspace-client-consumes-the-signals-context)
- **Code:** `src/mds/Server.cc:3500-3520 (Server::check_access)`, `src/mds/SessionMap.cc:1109-1172 (Session::check_access)`, `src/mds/Server.cc:149-159 (Batch_Getattr_Lookup::_respond)`, `src/client/Client.cc:10039-10041`
- **Notes:** The C++ client's readdir tolerates -EACCES from _getattr on a quarantined child and uses cached attributes (commit 630bf842c11); I/O on an already-open file waits for caps instead of failing (code reading). Ganesha maps -EACCES to an NFS access error (inferred, untested). Renaming the subvolume root is gated on its parent: aware clients get -EACCES from check_access on the source inode, unaware clients can rename it (inferred, untested). Batching is enabled by mds_allow_batched_ops (default true). See also F-ADM-018 (per-request access checks).

<a id="f-qos-072"></a>

#### F-QOS-072 · Request parking for quarantine-unaware sessions

*Holds requests of clients without feature bit 24 on a quarantined subvolume inside the MDS, without reply, until the quarantine is lifted.*

Server::check_quarantine_block returns false for internal requests, aware sessions and inodes not under quarantine; otherwise it adds C_MDS_RetryRequest as a WAIT_QUARANTINE waiter on the first quarantined (or being-quarantined) subvolume root up the realm chain and the caller returns without replying. Parking sites: rdlock_path_pin_ref on the target (getattr, lookup, open, readdir, setattr, setlayout, normal setxattr and removexattr, getvxattr, file locks, snapdiff, blockdiff, snapshot metadata), rdlock_path_xlock_dentry on the parent (create, mknod, mkdir, symlink, unlink, rmdir, link by ino), rdlock_two_paths_xlock_destdn on both parents (rename, path-based link) and handle_client_lookup_ino (lookupino, lookuphash, lookupname, lookupparent). Parked requests keep the path locks they already hold (dentry rdlocks, snaplock rdlocks up to /, and for creates parent filelock and nestlock wrlocks plus a dentry xlock) and are woken by the disable walk visiting the root or by an abort; requests woken in the first two sites are not re-checked. Kernel system calls block (killable) without timeout, and a kernel mount inside the subvolume fails with -ETIMEDOUT after mount_timeout (60 s, inferred).

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [quarantine.md › 3.7 Enforcement on client requests](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#37-enforcement-on-client-requests) · [quarantine.md › 7.3 Linux kernel client (never quarantine-aware)](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#73-linux-kernel-client-never-quarantine-aware) · [quarantine.md › 8. Edge cases, failure handling and gotchas](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#8-edge-cases-failure-handling-and-gotchas)
- **Code:** `src/mds/Server.cc:3529-3556 (Server::check_quarantine_block)`, `src/mds/Server.cc:3924 (Server::rdlock_path_pin_ref)`, `src/mds/Server.cc:4022 (Server::rdlock_path_xlock_dentry)`, `src/mds/Server.cc:4127-4128 (Server::rdlock_two_paths_xlock_destdn)`, `src/mds/Server.cc:4465 (Server::handle_client_lookup_ino)`
- **Notes:** Not parked: ceph.* vxattr set/remove and SETDIRLAYOUT addressed by ino, and LOOKUPINO with a snapid; an allow * kernel client can therefore remove ceph.dir.subvolume from a quarantined root (inferred). Held locks can wedge an enable (an unaware request parked between acceptance and the root xlocks holds the root snaplock rdlock, so the command never answers and later ones get -EBUSY) and a disable with replicas; evicting the client unblocks both (inferred, untested). Parked requests are ordinary tracked ops and can raise slow-request warnings.

<a id="f-qos-073"></a>

#### F-QOS-073 · Snapshot operation parking under quarantine

*Parks lssnap, mksnap, rmsnap and renamesnap of unaware clients on quarantined subvolumes.*

handle_client_lssnap, handle_client_mksnap, handle_client_rmsnap and handle_client_renamesnap call check_quarantine_block on the directory and re-run it on every retry, so kernel clients can neither list nor create, remove or rename snapshots (ls, mkdir, rmdir and rename in .snap) of a quarantined subvolume; commit c05f3db248a added this because kernel clients could otherwise destroy recovery snapshots. Snapdir paths are traversed without RDLOCK_PATH or RDLOCK_SNAP, so these parked requests hold no path locks. Aware clients get -EACCES from check_access instead.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient
- **Map:** [quarantine.md › 3.7 Enforcement on client requests](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#37-enforcement-on-client-requests)
- **Code:** `src/mds/Server.cc:11485 (Server::handle_client_lssnap)`, `src/mds/Server.cc:11608 (Server::handle_client_mksnap)`, `src/mds/Server.cc:11797 (Server::handle_client_rmsnap)`, `src/mds/Server.cc:11934 (Server::handle_client_renamesnap)`

<a id="f-qos-074"></a>

#### F-QOS-074 · Cap restriction to PIN under quarantine

*Issues only CEPH_CAP_PIN on inodes under quarantine to sessions without q/Q.*

CInode::get_caps_quarantine_mask returns CEPH_CAP_PIN when the inode is under quarantine and the session lacks quarantine access for its path, and CEPH_CAP_ANY otherwise; the mask is ANDed into the any, loner and xlocker allowed masks, the careful and xlocker masks, get_caps_allowed_for_client, try_drop_loner and the snapped-inode branch of encode_inodestat, and Locker::get_allowed_caps caches the masks per q status. Directories therefore lose Fs, Fx, As, Xs and Ls too. Clients receive ordinary REVOKE messages down to PIN and flush dirty data and metadata as usual: cap flushes and releases are not gated, dentry leases are not restricted, and OSD access is untouched (a client that already holds valid caps or writes objects directly with its OSD caps is not stopped, inferred).

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by kclient, libcephfs, ceph-fuse, ganesha
- **Map:** [quarantine.md › 3.8 Enforcement on capabilities and replies](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#38-enforcement-on-capabilities-and-replies) · [quarantine.md › 7.5 OSDs](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#75-osds)
- **Code:** `src/mds/CInode.cc:3712-3723 (CInode::get_caps_quarantine_mask)`, `src/mds/CInode.cc:3798-3816 (CInode::get_caps_allowed_for_client)`, `src/mds/Locker.cc:2528-2575 (Locker::get_allowed_caps)`, `src/mds/Locker.cc:2578-2715 (Locker::issue_caps)`
- **Notes:** Caps issued before a change persist until the next evaluation (walk, lock change, cap request, reissue_all_caps). The cap walk does not proactively revoke caps on the .meta file, but any later issue_caps restricts them.

<a id="f-qos-075"></a>

#### F-QOS-075 · EQUARANTINED errno in MClientCaps

*Tags cap grants and revokes on quarantined inodes with oserrno -1000 for sessions without q/Q.*

MClientCaps HEAD_VERSION 14 appends int32 oserrno after subvolume_id. In Locker::issue_caps, every CEPH_CAP_OP_GRANT or REVOKE built by the main issue/revoke branch for an inode under quarantine carries -EQUARANTINED (EQUARANTINED = 1000) for sessions without q/Q and 0 for sessions with it; the stale-cap re-issue branch, cap import/export and flush acks never set it, and -EQUARANTINED is never a request result. The C++ client marks an inode quarantined on a revoke with -EQUARANTINED and clears the mark on a grant beyond PIN with oserrno 0; the kernel decodes MClientCaps only up to its v12 fields and ignores oserrno.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by libcephfs, ceph-fuse, ganesha
- **Map:** [quarantine.md › 3.8 Enforcement on capabilities and replies](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#38-enforcement-on-capabilities-and-replies) · [quarantine.md › 5.2 Messages](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#52-messages)
- **Code:** `src/mds/Locker.cc:2697-2704 (Locker::issue_caps)`, `src/messages/MClientCaps.h:311-321 (MClientCaps::encode_payload)`, `src/include/rados.h:515 (EQUARANTINED)`, `src/client/Client.cc:6226-6241 (Client::handle_cap_grant)`, `linux/fs/ceph/caps.c:4535-4538 (ceph_handle_caps)`

<a id="f-qos-076"></a>

#### F-QOS-076 · QUARANTINE marker in InodeStat

*Marks the quarantined subvolume root in replies with an optmetadata entry of kind QUARANTINE.*

encode_inodestat adds an optmetadata entry of kind QUARANTINE (2) when the inode itself has the persisted flag (only the subvolume root), with a default quarantine_md_t (enabled true, timestamp 0: the real time is not sent); descendants carry no marker. It is emitted inside InodeStat v9 (sessions with CEPHFS_FEATURE_REPLY_ENCODING) between fscrypt_file and subvolume_id. libcephfs stores optmetadata verbatim and its Inode::is_under_quarantine walks client-side parents for this marker or a -EQUARANTINED cap errno; the kernel skips optmetadata as an opaque versioned blob.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by libcephfs, ceph-fuse
- **Map:** [quarantine.md › 3.8 Enforcement on capabilities and replies](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#38-enforcement-on-capabilities-and-replies) · [quarantine.md › 7.3 Linux kernel client (never quarantine-aware)](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#73-linux-kernel-client-never-quarantine-aware)
- **Code:** `src/mds/CInode.cc:4120-4141 (CInode::encode_inodestat)`, `src/client/Inode.cc:913-924 (Inode::is_under_quarantine)`, `linux/fs/ceph/mds_client.c:263-271 (parse_reply_info_in)`

<a id="f-qos-077"></a>

#### F-QOS-077 · MQuarantineDisable notification

*When the quarantine code sends MQuarantineDisable.*

See F-PROTO-097 for the message. Quarantine-specific: on disable the cap-revocation walk sends it to every quarantine-aware session holding a cap on each visited inode, and the QUARANTINE_INODE finisher sends it once more for the root; directories whose walk op had children get it twice. The C++ client's handle_quarantine_disable clears its quarantine mark and signals cap waiters.

- **Class:** POSIX `extension` · exposure `client-wire` · relied on by libcephfs, ceph-fuse, ganesha
- **Map:** [quarantine.md › 5.2 Messages](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#52-messages) · [quarantine.md › 3.6 The cap-revocation walk (quiesce machinery in quarant...](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#36-the-cap-revocation-walk-quiesce-machinery-in-quarantine-mode) · [quarantine.md › 7.4 libcephfs, ceph-fuse and NFS-Ganesha](../cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#74-libcephfs-ceph-fuse-and-nfs-ganesha)
- **Code:** `src/messages/MQuarantineDisable.h:22-57 (MQuarantineDisable)`, `src/mds/MDCache.cc:15291-15303 (MDCache::start_revoke_caps_for_inode)`, `src/client/Client.cc:5718-5731 (Client::handle_quarantine_disable)`, `src/mds/Server.cc:860-864 (Server::handle_client_session)`
- **Notes:** See also F-QOS-064 (cap-revocation walk), F-QOS-075 (EQUARANTINED errno).

### Retired IDs

These IDs were merged into other entries during the review. They are never reused; the anchors below keep links to them working.

| Retired ID | Merged into |
|---|---|
| <a id="f-qos-031"></a>F-QOS-031 | [F-ADM-106](11-security-admin-and-observability.md#f-adm-106) · mgr/stats: fs perf stats (and cephfs-top) |
| <a id="f-qos-054"></a>F-QOS-054 | [F-ADM-101](11-security-admin-and-observability.md#f-adm-101) · mgr/volumes: fs quiesce |
| <a id="f-qos-055"></a>F-QOS-055 | [F-ADM-053](11-security-admin-and-observability.md#f-adm-053) · lock path and quiesce path test hooks |
| <a id="f-qos-056"></a>F-QOS-056 | [F-ADM-053](11-security-admin-and-observability.md#f-adm-053) · lock path and quiesce path test hooks |
| <a id="f-qos-059"></a>F-QOS-059 | [F-ADM-102](11-security-admin-and-observability.md#f-adm-102) · mgr/volumes: subvolume quarantine |

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
