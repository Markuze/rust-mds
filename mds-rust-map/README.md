# Rust MDS reference set

> Entry point of the reference material for a new CephFS metadata server written in Rust. Sources: ceph.git at commit `5f7e383ae6d` (2026-10-09), the Linux kernel CephFS client snapshot 7.3.0-rc6 (master `af32da41b03`, 2026-10-09), and the external sources named in individual pages. Related: [C++ MDS project map](cpp-map/README.md), [Glossary](GLOSSARY.md).

## Purpose

This directory holds the reference material for a team building a new CephFS metadata server (MDS) in Rust. It is written to be moved, as a whole, into the Rust project's repository, so every link inside it is relative.

It has two parts, both facts only:

1. **C++ MDS project map** ([`cpp-map/`](cpp-map/README.md)): 46 pages of facts about the existing C++ MDS (`ceph-mds`) and the unmodified peers around it, plus an architecture overview. The map describes what the code does, including its bugs, and contains no design. The [Glossary](GLOSSARY.md) defines the terms it uses.
2. **Features catalog** ([`features/`](features/README.md)): 935 entries in 11 categories that describe what the C++ MDS does. Each entry gives its POSIX class, what it exposes (client wire, cluster wire, MDS-to-MDS wire, on-disk, admin, config, metrics), the peers that rely on it and code references. A [by-peer view](features/by-peer.md), a [legacy view](features/legacy.md) and a machine-readable [`features.json`](features/features.json) come with it. Entries that serve only older clients, msgr1 mounts or older C++ releases are marked legacy and listed last. The catalog is input for choosing the minimal feature set and makes no recommendations.

The design of the Rust MDS is not part of this set.

## Goals and constraints of the Rust MDS

The new MDS must:

- **Drop into a current Ceph deployment.** Current versions of the Linux kernel client, ceph-fuse and libcephfs, NFS-Ganesha (through libcephfs), the OSDs, the monitors, the mgr modules, and cephadm or Rook stay unchanged. A change to a peer is possible where it brings a tangible benefit. Each such change is discussed case by case.
- **Serve current peers only.** There is no backward compatibility with older client or daemon versions. Paths that serve only those versions are out of scope. Examples are msgr1, encodings and fallbacks for old clients, and formats of older C++ releases.
- **Not be backward compatible with the C++ MDS.** It does not read on-disk data written by the C++ MDS, and it never runs together with C++ MDS daemons in one file system. Its on-disk format is free.
- **Be POSIX compliant** with a **minimal feature set**.
- **Be highly concurrent**, with no global lock, and treat **each subvolume as a fully autonomous shard**.

Use cases that motivate it: per-subvolume SLAs and rate limiting, multi-tenant journal replay, and scaling synchronous NFS (NFS-Ganesha) writes across CPU cores.

### Baseline: current peers

Statements about "the kernel", "libcephfs" and the other peers refer to these versions unless a page names another:

| Peer | Version taken as current | How it connects |
|---|---|---|
| Linux kernel client | the 7.3.0-rc6 snapshot named under [Provenance](#provenance) | msgr2. `mount.ceph` passes `ms_mode=prefer-crc` for the new device syntax (since 17.2.0), and Rook does so for ceph-csi when `requireMsgr2` is set, as in its example cluster; the CRD default leaves it unset ([transport 1](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#1-purpose-and-responsibilities)). |
| libcephfs, ceph-fuse, cephfs-mirror, mgr modules, monitors, OSDs, cephadm, tools | ceph.git `main` at `5f7e383ae6d` | msgr2 (the C++ messenger prefers the v2 address) |
| NFS-Ganesha | V15.7, FSAL_CEPH over libcephfs at `5f7e383ae6d` | through libcephfs |
| Rook | v1.21.0 | configures ceph-csi and the daemons |

Behaviour that only older clients, msgr1 mounts or older C++ releases need is still described where the C++ code carries it, but it is marked as legacy:

- [kclient expectations 7.1](cpp-map/02-client-protocol/kclient-expectations.md#71-client-generations-and-protocol-fallbacks) records the older client generations and their fallbacks.
- In the catalog, the `legacy` field marks each entry used only by those clients or releases ([features README](features/README.md)).

The mark states who uses a path. By the constraint above, such paths are out of scope.

Where the map documents the facts each constraint depends on:

| Constraint or use case | Pages |
|---|---|
| Unmodified current kernel client | [kclient expectations](cpp-map/02-client-protocol/kclient-expectations.md), [transport 7.2](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#72-linux-kernel-client-over-msgr2), [wire protocol](cpp-map/02-client-protocol/wire-protocol.md), [sessions](cpp-map/02-client-protocol/sessions.md), [capabilities](cpp-map/02-client-protocol/capabilities.md) |
| Unmodified libcephfs, ceph-fuse and NFS-Ganesha | [libcephfs, ceph-fuse and NFS-Ganesha expectations](cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md), [sessions 3.9 (reclaim)](cpp-map/02-client-protocol/sessions.md#39-reclaim-protocol-nfs-ganesha-via-libcephfs) |
| Unmodified monitors | [beacon and monitor](cpp-map/01-cluster-integration/beacon-and-monitor.md), [FSMap and MDSMap](cpp-map/01-cluster-integration/fsmap-and-mdsmap.md), [auth and MDS caps](cpp-map/01-cluster-integration/auth-and-mds-caps.md) |
| Unmodified OSDs | [RADOS and OSD interactions](cpp-map/07-storage-formats/rados-and-osd-interactions.md), [RADOS object catalog](cpp-map/07-storage-formats/rados-object-catalog.md) (data-object names and backtraces that clients and tools also read) |
| Unmodified mgr modules, cephadm and Rook | [mgr, tools and orchestration](cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md), [admin commands](cpp-map/13-admin-and-config/admin-commands.md), [config, perf counters and health](cpp-map/13-admin-and-config/config-perf-counters-and-health.md), [metrics](cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md) |
| No C++ on-disk compatibility | [RADOS object catalog 3.15](cpp-map/07-storage-formats/rados-object-catalog.md#315-offline-tools-and-the-objects-they-use) and [mgr, tools and orchestration 3.10](cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md#310-offline-repair-and-inspection-tools-channel-d) (tools that read C++ formats), [QA 1.2](cpp-map/14-testing/qa-and-conformance-suites.md#12-channel-classes) (tests that depend on them) |
| POSIX compliance | [POSIX semantics contract](cpp-map/03-request-path/posix-semantics-contract.md), [QA 7.5](cpp-map/14-testing/qa-and-conformance-suites.md#75-external-conformance-suites) |
| No global lock, autonomous subvolumes | [`mds_lock` analysis](cpp-map/00-architecture/mds_lock-analysis.md), especially sections [4](cpp-map/00-architecture/mds_lock-analysis.md#4-rank-wide-singletons-that-couple-independent-subvolumes) and [6](cpp-map/00-architecture/mds_lock-analysis.md#6-coupling-that-would-remain-even-with-perfect-locking) |
| Per-subvolume SLA and rate limiting | [QoS scheduler 8](cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#8-edge-cases-failure-handling-and-gotchas), [metrics 3.7](cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md#37-subvolume-metrics-end-to-end), [rstats, quotas and subvolumes](cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md) |
| Multi-tenant journal replay | [MDLog and Journaler](cpp-map/06-journal/mdlog-and-journaler.md), [events and replay](cpp-map/06-journal/events-and-replay.md), [rank lifecycle](cpp-map/09-recovery/rank-lifecycle-and-recovery.md) |
| Synchronous NFS writes across cores | [`mds_lock` analysis 5.d](cpp-map/00-architecture/mds_lock-analysis.md#5d-synchronous-writes-and-fsync-from-nfs-ganesha-clients-in-different-subvolumes), [libcephfs expectations 3.16](cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#316-nfs-ganesha-specifics) |

## Provenance

- **C++ source**: ceph.git, branch `main`, commit `5f7e383ae6dcb3566968c07454a751131e85355e` ("Merge pull request #72376 from ronen-fr/wip-rf-absent-boot", 2026-10-09). Every code citation without a prefix refers to this commit.
- **Linux kernel client**: a snapshot of `fs/ceph`, `include/linux/ceph` and `net/ceph`. Its `Makefile` says 7.3.0-rc6, and it is master commit `af32da41b03` (2026-10-09). Citations carry the prefix `linux/`.
- **External sources**: pages that needed code outside these two trees downloaded it and name the exact version in their source lines: older Linux tags and two CentOS Stream kernels, used only to tell current from legacy paths ([kclient expectations 7.1](cpp-map/02-client-protocol/kclient-expectations.md#71-client-generations-and-protocol-fallbacks)), nfs-ganesha V15.7 and libfuse ([libcephfs expectations](cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md), [POSIX contract](cpp-map/03-request-path/posix-semantics-contract.md)), Rook v1.21.0 ([mgr, tools and orchestration](cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md)), and fstests, pjdfstest, LTP and pynfs ([QA and conformance suites](cpp-map/14-testing/qa-and-conformance-suites.md)).

## Method

1. **Deep reading.** One agent per subsystem read the code of its topic. Each wrote a page from the common template and a staging file (`_staging/<item>.json`) listing features, serialization points, external contract items, design notes and open questions.
2. **Adversarial fact-check.** A separate agent re-checked every claim and citation of every page against the source and fixed errors in place. [Verification](#verification) gives the counts.
3. **Coverage and consistency critics.** Reviewers looked for code and behaviour that no page covered and for contradictions between pages.
4. **Gap fills.** The missing subjects became new pages, each with its own fact-check: the [transport, auth and client-library contract](cpp-map/00-architecture/transport-auth-and-client-library-contract.md), the [POSIX semantics contract](cpp-map/03-request-path/posix-semantics-contract.md) and the [QA and conformance suite inventory](cpp-map/14-testing/qa-and-conformance-suites.md). The [`mds_lock` analysis](cpp-map/00-architecture/mds_lock-analysis.md) consolidates the serialization points of all staging files and section 6 of every page.
5. **Panel.** Three independent investigators examined the most consequential finding: lock caches are never created, so the kernel's async create and unlink never happen against this MDS. Status **confirmed**: by code reading of the MDS, the kernel client and libcephfs, and by git history; no test covers it and it was not observed at runtime (`_staging/findings-lock-cache.json`, [leases 8 G1](cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#8-edge-cases-failure-handling-and-gotchas)).

## Conventions

- **Citations**: code is cited as `path:line (Symbol)`, ranges as `path:first-last`, against `5f7e383ae6d`; kernel code as `linux/path:line (function)` against `af32da41b03`. Line numbers are valid only at those commits; after a move, find the code again by the symbol.
- **Page template**: every topic page uses these headings, in this order: `1. Purpose and responsibilities`, `2. Source map`, `3. How it works`, `4. Data structures and invariants`, `5. Interfaces`, `6. Concurrency and mds_lock`, `7. Externally visible contract`, `8. Edge cases, failure handling and gotchas`. The title is followed by a quote line that links the [map README](cpp-map/README.md) and names the sources and related pages. The [`mds_lock` analysis](cpp-map/00-architecture/mds_lock-analysis.md) uses its own structure.
- **Diagrams**: Mermaid, inside the pages.
- **Evidence markers**: "(unverified)" marks a claim that could not be confirmed. "Inferred", "by code reading" and "derived from code" mark claims established only by reading code, never observed at runtime.
- **Facts only**: pages under `cpp-map/` and `features/` describe the existing code and its peers. The set contains no design for the Rust MDS.

## Verification

Results of the fact-check pass. "Lines" is the page length when it was checked; later gap fills changed some pages.

| Page | Lines | Corrections applied by the fact-checker | Substantive issues fixed |
|---|---:|---:|---:|
| [process-and-threading](cpp-map/00-architecture/process-and-threading.md) | 670 | 17 | 10 |
| [fsmap-and-mdsmap](cpp-map/01-cluster-integration/fsmap-and-mdsmap.md) | 667 | 15 | 7 |
| [beacon-and-monitor](cpp-map/01-cluster-integration/beacon-and-monitor.md) | 676 | 14 | 6 |
| [auth-and-mds-caps](cpp-map/01-cluster-integration/auth-and-mds-caps.md) | 624 | 16 | 9 |
| [mgr-tools-and-orchestration](cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md) | 663 | 20 | 11 |
| [wire-protocol](cpp-map/02-client-protocol/wire-protocol.md) | 1048 | 26 | 9 |
| [sessions](cpp-map/02-client-protocol/sessions.md) | 753 | 21 | 10 |
| [capabilities](cpp-map/02-client-protocol/capabilities.md) | 683 | 21 | 8 |
| [leases-lock-caches-and-async-dirops](cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md) | 613 | 18 | 6 |
| [kclient-expectations](cpp-map/02-client-protocol/kclient-expectations.md) | 845 | 17 | 7 |
| [libcephfs-fuse-ganesha-expectations](cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md) | 677 | 23 | 10 |
| [request-lifecycle](cpp-map/03-request-path/request-lifecycle.md) | 886 | 21 | 7 |
| [namespace-mutations](cpp-map/03-request-path/namespace-mutations.md) | 661 | 21 | 10 |
| [rename](cpp-map/03-request-path/rename.md) | 621 | 20 | 12 |
| [attributes-and-xattrs](cpp-map/03-request-path/attributes-and-xattrs.md) | 663 | 16 | 8 |
| [lookup-readdir-open-and-file-locks](cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md) | 668 | 23 | 11 |
| [cinode-and-inode-types](cpp-map/04-cache/cinode-and-inode-types.md) | 734 | 26 | 12 |
| [cdir-cdentry-and-cache-objects](cpp-map/04-cache/cdir-cdentry-and-cache-objects.md) | 762 | 25 | 11 |
| [mdcache-subtrees-traversal-replication](cpp-map/04-cache/mdcache-subtrees-traversal-replication.md) | 774 | 26 | 12 |
| [mdcache-lifecycle-trim-and-memory](cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md) | 705 | 15 | 9 |
| [dirfrags-and-fragmentation](cpp-map/04-cache/dirfrags-and-fragmentation.md) | 536 | 20 | 8 |
| [lock-types-and-state-machines](cpp-map/05-locking/lock-types-and-state-machines.md) | 958 | 24 | 12 |
| [acquiring-locks](cpp-map/05-locking/acquiring-locks.md) | 659 | 15 | 6 |
| [mdlog-and-journaler](cpp-map/06-journal/mdlog-and-journaler.md) | 693 | 20 | 11 |
| [events-and-replay](cpp-map/06-journal/events-and-replay.md) | 603 | 24 | 14 |
| [rados-object-catalog](cpp-map/07-storage-formats/rados-object-catalog.md) | 709 | 25 | 13 |
| [mds-tables](cpp-map/07-storage-formats/mds-tables.md) | 799 | 15 | 5 |
| [rados-and-osd-interactions](cpp-map/07-storage-formats/rados-and-osd-interactions.md) | 725 | 19 | 7 |
| [subtree-migration](cpp-map/08-multi-mds/subtree-migration.md) | 641 | 22 | 8 |
| [balancer-and-pinning](cpp-map/08-multi-mds/balancer-and-pinning.md) | 682 | 13 | 6 |
| [rank-lifecycle-and-recovery](cpp-map/09-recovery/rank-lifecycle-and-recovery.md) | 908 | 17 | 10 |
| [resolve-and-rejoin](cpp-map/09-recovery/resolve-and-rejoin.md) | 708 | 15 | 10 |
| [snapshots](cpp-map/10-snapshots-quotas-subvolumes/snapshots.md) | 637 | 19 | 13 |
| [rstats-quotas-and-subvolumes](cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md) | 767 | 29 | 9 |
| [strays-and-purge](cpp-map/11-background-services/strays-and-purge.md) | 605 | 19 | 6 |
| [scrub-and-damage](cpp-map/11-background-services/scrub-and-damage.md) | 589 | 21 | 6 |
| [qos-dmclock-scheduler](cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md) | 560 | 13 | 7 |
| [metrics-and-subvolume-metrics](cpp-map/12-qos-metrics-quiesce-quarantine/metrics-and-subvolume-metrics.md) | 716 | 14 | 7 |
| [quiesce](cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md) | 670 | 20 | 5 |
| [quarantine](cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md) | 477 | 15 | 7 |
| [admin-commands](cpp-map/13-admin-and-config/admin-commands.md) ¹ | 556 | 20 | 15 |
| [config-perf-counters-and-health](cpp-map/13-admin-and-config/config-perf-counters-and-health.md) | 848 | 23 | 9 |
| **Total (42 pages)** | **29,439** | **823** | **379** |

¹ `admin-commands.md` then received a second full verification pass.

Not in the table: the [`mds_lock` analysis](cpp-map/00-architecture/mds_lock-analysis.md) and the three gap-fill pages ([transport, auth and client-library contract](cpp-map/00-architecture/transport-auth-and-client-library-contract.md), [POSIX semantics contract](cpp-map/03-request-path/posix-semantics-contract.md), [QA and conformance suites](cpp-map/14-testing/qa-and-conformance-suites.md)) were verified in the final step, after this table was compiled.

## Directory tree

Generated with `find` when this README was written; `_staging/` is left out and described [below](#staging-files).

```text
mds-rust-map/
├── README.md                                              entry point of the reference set (this page)
├── GLOSSARY.md                                            CephFS and MDS terms, each linked to the page that explains it
├── cpp-map/                                               C++ MDS project map (facts only)
│   ├── README.md                                          architecture overview, create walkthrough, reading orders, section index
│   ├── 00-architecture/
│   │   ├── mds_lock-analysis.md                           what mds_lock protects, 268 serialization points, rank-wide singletons
│   │   ├── process-and-threading.md                       process anatomy, threads, dispatch, contexts, timers
│   │   └── transport-auth-and-client-library-contract.md  msgr2 (msgr1 as legacy), cephx, MonClient, MgrClient, Objecter, per-peer checklist
│   ├── 01-cluster-integration/
│   │   ├── auth-and-mds-caps.md                           cephx as seen by the MDS, MDS cap language and checks
│   │   ├── beacon-and-monitor.md                          beacons, rank assignment, failover, fencing
│   │   ├── fsmap-and-mdsmap.md                            FSMap, MDSMap, MDS states, compat sets, fs and mds commands
│   │   └── mgr-tools-and-orchestration.md                 mgr modules, cephadm, Rook, exporter, offline tools
│   ├── 02-client-protocol/
│   │   ├── capabilities.md                                cap model, issue and revoke, flush, max_size, snapflush
│   │   ├── kclient-expectations.md                        what the Linux kernel client requires from an MDS
│   │   ├── leases-lock-caches-and-async-dirops.md         dentry leases, lock caches, delegated inos, async dirops
│   │   ├── libcephfs-fuse-ganesha-expectations.md         what libcephfs, ceph-fuse and NFS-Ganesha require
│   │   ├── sessions.md                                    sessions, eviction, reclaim, reconnect, SessionMap
│   │   └── wire-protocol.md                               client messages, op codes, encodings, feature bits
│   ├── 03-request-path/
│   │   ├── attributes-and-xattrs.md                       setattr, truncate, xattrs, ceph.* virtual xattrs
│   │   ├── lookup-readdir-open-and-file-locks.md          getattr, lookup, open, readdir, fcntl and flock
│   │   ├── namespace-mutations.md                         create, mknod, mkdir, symlink, link, unlink, rmdir
│   │   ├── posix-semantics-contract.md                    POSIX contract per call and departures from POSIX
│   │   ├── rename.md                                      rename, including the cross-rank two-phase commit
│   │   └── request-lifecycle.md                           request intake, MDRequest, replies, forwarding, peer requests
│   ├── 04-cache/
│   │   ├── cdir-cdentry-and-cache-objects.md              CDir, CDentry, MDSCacheObject, dirfrag commit and fetch
│   │   ├── cinode-and-inode-types.md                      CInode, inode_t, projection, backtraces, per-inode locks
│   │   ├── dirfrags-and-fragmentation.md                  fragtrees, split and merge, recovery
│   │   ├── mdcache-lifecycle-trim-and-memory.md           pins, LRU trimming, memory limit, base hierarchy, stopping
│   │   └── mdcache-subtrees-traversal-replication.md      subtree map, authority, path traversal, replication, open_ino
│   ├── 05-locking/
│   │   ├── acquiring-locks.md                             acquire_locks, ordering, auth pins, remote locks
│   │   └── lock-types-and-state-machines.md               SimpleLock, ScatterLock, LocalLock state tables
│   ├── 06-journal/
│   │   ├── events-and-replay.md                           journal events, EMetaBlob, replay
│   │   └── mdlog-and-journaler.md                         the per-rank journal, segments, expiry, standby-replay
│   ├── 07-storage-formats/
│   │   ├── mds-tables.md                                  InoTable, table 2PC and snap table, OpenFileTable
│   │   ├── rados-and-osd-interactions.md                  the MDS as a RADOS client, fencing, epoch barrier, full pools
│   │   └── rados-object-catalog.md                        every RADOS object in the metadata and data pools
│   ├── 08-multi-mds/
│   │   ├── balancer-and-pinning.md                        MDBalancer, Mantle, export and ephemeral pins
│   │   └── subtree-migration.md                           Migrator export and import
│   ├── 09-recovery/
│   │   ├── rank-lifecycle-and-recovery.md                 rank state machine, replay to active, stopping, failure exits
│   │   └── resolve-and-rejoin.md                          resolve and rejoin protocols
│   ├── 10-snapshots-quotas-subvolumes/
│   │   ├── rstats-quotas-and-subvolumes.md                rstat and dirstat, quotas, subvolumes
│   │   └── snapshots.md                                   snap realms, snap table, COW, snapdiff and blockdiff
│   ├── 11-background-services/
│   │   ├── scrub-and-damage.md                            forward scrub, repair, DamageTable
│   │   └── strays-and-purge.md                            stray directories and the PurgeQueue
│   ├── 12-qos-metrics-quiesce-quarantine/
│   │   ├── metrics-and-subvolume-metrics.md               client metrics, rank-0 aggregation, subvolume metrics
│   │   ├── qos-dmclock-scheduler.md                       dmClock QoS scheduler
│   │   ├── quarantine.md                                  subvolume quarantine
│   │   └── quiesce.md                                     QuiesceDb, agents, quiescelock
│   ├── 13-admin-and-config/
│   │   ├── admin-commands.md                              admin socket and tell commands
│   │   └── config-perf-counters-and-health.md             options, perf counters, health checks, cluster log
│   └── 14-testing/
│       └── qa-and-conformance-suites.md                   qa suites, unit tests and external conformance suites
└── features/                                              feature catalog (facts only): one page per category, by-peer and legacy views, features.json
    ├── README.md                                          how to read the catalog, vocabulary, counts by category, exposure and peer
    ├── 01-cluster-membership-and-ha.md                    features: cluster membership and high availability
    ├── 02-client-sessions-and-protocol.md                 features: client sessions and protocol
    ├── 03-caps-leases-and-locking.md                      features: capabilities, leases and locking
    ├── 04-namespace-and-posix-operations.md               features: namespace and POSIX operations
    ├── 05-metadata-cache-and-dirfrags.md                  features: metadata cache and directory fragments
    ├── 06-journal-and-storage.md                          features: journal and storage
    ├── 07-multi-mds.md                                    features: multiple active MDS ranks
    ├── 08-snapshots-quotas-subvolumes.md                  features: snapshots, quotas and subvolumes
    ├── 09-background-services.md                          features: background services
    ├── 10-qos-metrics-quiesce-quarantine.md               features: QoS, metrics, quiesce and quarantine
    ├── 11-security-admin-and-observability.md             features: security, administration and observability
    ├── by-peer.md                                         the current features each peer relies on (kclient, libcephfs, Ganesha, mon, mgr, OSD, ...)
    ├── features.json                                      all 935 entries with their legacy marks, and the retired IDs, machine-readable
    └── legacy.md                                          entries that serve only older clients, msgr1 mounts or older releases, with evidence
```

### Staging files

`_staging/` holds machine-readable notes in JSON, written alongside the pages; the pages can be read without them.

- **Per-topic files** (`<item>.json`, 42 of them, one per original map page). They were written before the [baseline](#baseline-current-peers) was set, so some notes treat older clients as requirements; where a note and a page disagree, the page is right. Each file is one object with the keys `item`, `pages` (the page it feeds), `features` (1675 in total), `serialization_points` (665), `external_contract` (1076), `rust_implications` (765 design notes, kept out of the map pages on purpose) and `open_questions` (379).
- **`findings-lock-cache.json`**: the claim, final status, basis, refinements, evidence and updated pages of the panel described in [Method](#method).
- **`features-*.json`**: the source of the feature catalog, one file per category, each with its list of retired IDs. The pages in `features/` and `features/features.json` are generated from them. Each entry's `staging_refs` points back to the `features` entries of the per-topic files above.

## Notable observations

The fifteen findings that matter most to a re-implementation, taken from section 8 of the pages and from the staging open questions. Each carries its evidence level.

1. **One lock per rank.** All metadata work of a rank runs under one FIFO `mds_lock`, so a rank uses at most one core for metadata; of 268 serialization points, 116 are implementation artifacts, 112 partly required and 40 inherent. *Evidence: code reading.* [`mds_lock` analysis, summary](cpp-map/00-architecture/mds_lock-analysis.md#summary)
2. **Rank-wide singletons couple subvolumes.** One journal per rank (an fsync in one subvolume flushes every subvolume's buffered events), one SessionMap version chain whose completions must apply in projection order, and one InoTable, current stray directory, PurgeQueue, OpenFileTable and LRU per rank; the snap table and QuiesceDb leader serve the whole file system. *Evidence: code reading.* [`mds_lock` analysis 4](cpp-map/00-architecture/mds_lock-analysis.md#4-rank-wide-singletons-that-couple-independent-subvolumes)
3. **Some coupling remains with perfect locking.** Recursive stats and quotas aggregate up to `/volumes` and `/`, snapshot ids are file-system-wide, sessions and caps on shared ancestors span subvolumes, and inode numbers are client-visible and unique per file system. *Evidence: code reading.* [`mds_lock` analysis 6](cpp-map/00-architecture/mds_lock-analysis.md#6-coupling-that-would-remain-even-with-perfect-locking)
4. **No async directory operations against the C++ MDS.** Since squid v19.1.0, `Locker::create_lock_cache` requires `Dc`/`Dr` to be issued already, but only a lock cache makes them issuable, so the kernel's default async create and unlink always fall back to synchronous requests; panel status: confirmed, not covered by any test, not observed at runtime. *Evidence: confirmed by git history.* [leases 8 G1](cpp-map/02-client-protocol/leases-lock-caches-and-async-dirops.md#8-edge-cases-failure-handling-and-gotchas)
5. **Feature bit 22 has two meanings.** This C++ tree defines it as `CEPHFS_FEATURE_CHARMAP`, Linux 7.1 and later as `CEPHFS_FEATURE_SUBVOLUME_METRICS`, so the C++ MDS treats new kernels as charmap-capable and the kernel enables subvolume metrics toward any MDS that advertises the bit. *Evidence: code reading of both trees, confirmed by release history.* [kclient expectations 7.1.4](cpp-map/02-client-protocol/kclient-expectations.md#714-the-bit-22-conflict)
6. **Clients can crash the C++ MDS.** Assertions or aborts follow from a `head.ino` that is neither preallocated nor free, an unsafe `MClientReply` sent by a client, an unknown lease action, an `MClientCaps` whose `xattr_len` differs from its middle segment, an owner id of -1 on a new inode, or swapped `FILE_BLOCKDIFF` arguments. *Evidence: code reading.* [wire protocol 8](cpp-map/02-client-protocol/wire-protocol.md#8-edge-cases-failure-handling-and-gotchas)
7. **The MDS does not check POSIX permissions.** The kernel VFS or libcephfs enforce mode bits, ACLs, the sticky bit and ownership rules; the MDS checks only MDS auth caps against client-asserted ids, accepts cap-flushed `Ax`/`Xx` changes without checks, and skips `check_access` for `RMXATTR`, `GETVXATTR`, file locks, `FILE_BLOCKDIFF` and `SNAP_METADATA`. *Evidence: code reading.* [POSIX contract 1](cpp-map/03-request-path/posix-semantics-contract.md#1-purpose-and-responsibilities)
8. **Only clients enforce quotas.** The MDS never returns `EDQUOT`; it pushes usage with `MClientQuota` at thresholds (7/8 of the limit, 1/16 of the headroom), so several writers can overshoot. *Evidence: code reading.* [rstats 3.11](cpp-map/10-snapshots-quotas-subvolumes/rstats-quotas-and-subvolumes.md#311-enforcement-clients-only)
9. **NFS metadata is synchronous and shares one journal flush.** NFS-Ganesha follows every SETATTR with `ceph_ll_sync_inode` and maps COMMIT to it; each costs a sync cap flush or `REQUEST_FLUSH_MDLOG` plus a wait for the safe reply, every flush writes all buffered events of the rank, and by default all exports of a file system share one session per rank. *Evidence: code reading (nfs-ganesha V15.7).* [libcephfs expectations 1](cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#1-purpose-and-responsibilities), [`mds_lock` analysis 5.d](cpp-map/00-architecture/mds_lock-analysis.md#5d-synchronous-writes-and-fsync-from-nfs-ganesha-clients-in-different-subvolumes)
10. **dmClock QoS is not a per-subvolume SLA.** It is off by default, keys on the first three components of the client-asserted mount root, works per rank and in memory only, has no capacity model, and every admitted request still runs under `mds_lock`. *Evidence: code reading.* [QoS scheduler 8](cpp-map/12-qos-metrics-quiesce-quarantine/qos-dmclock-scheduler.md#8-edge-cases-failure-handling-and-gotchas)
11. **Recovery is rank-wide.** Replay applies one totally ordered journal per rank under `mds_lock`, and the reconnect window, one-at-a-time clientreplay and pending NFS-Ganesha reclaims (up to `session_autoclose`, 300 s) hold back every subvolume of the rank. *Evidence: code reading.* [rank lifecycle 8](cpp-map/09-recovery/rank-lifecycle-and-recovery.md#8-edge-cases-failure-handling-and-gotchas)
12. **The unmodified monitor can skip the fencing wait.** `FSMap::erase` can overwrite `last_failure_osd_epoch` with 0 when a standby-replay daemon is removed in the same pending map as a failed rank holder, so the replacement may start replay without waiting for the blocklist epoch. *Evidence: code reading.* [beacon and monitor 8](cpp-map/01-cluster-integration/beacon-and-monitor.md#8-edge-cases-failure-handling-and-gotchas)
13. **Unmodified clients assert on unexpected MDS output.** libcephfs aborts on unknown session ops, on trailing bytes in `MClientReply`, `MClientSnap` and `MClientQuota`, and on a POSIX-style directory link count; several kernel `BUG_ON`s are reachable from MDS replies. *Evidence: code reading.* [libcephfs expectations 1](cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md#1-purpose-and-responsibilities), [cinode 8](cpp-map/04-cache/cinode-and-inode-types.md#8-edge-cases-failure-handling-and-gotchas), [kclient expectations 8](cpp-map/02-client-protocol/kclient-expectations.md#8-edge-cases-failure-handling-and-gotchas)
14. **Offline tools and part of QA read C++ on-disk formats.** `cephfs-journal-tool`, `cephfs-data-scan`, `cephfs-table-tool`, `cephfs-meta-injection` and `first-damage.py` parse journal, dirfrag, table and backtrace encodings, and QA tests in channel F depend on them. *Evidence: code reading.* [mgr, tools and orchestration 1](cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md#1-purpose-and-responsibilities), [QA 1.2](cpp-map/14-testing/qa-and-conformance-suites.md#12-channel-classes)
15. **The kernel's msgr2 client is stricter than the C++ peers.** It supports only msgr2.1 and never negotiates compression. It requires the first v2 address in `SERVER_IDENT` to equal the MDSMap address byte for byte. It requires `MSG_AUTH` among the server's features unless mounted with `nocephx_sign_messages`. It treats a `WAIT` frame, `SESSION_RESET` without `full` and a message-sequence gap as protocol errors, where C++ clients handle them. A `KEEPALIVE2` from the server is also a protocol error. *Evidence: code reading.* [transport 3.4](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#34-msgr20-and-msgr21), [transport 7.2](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#72-linux-kernel-client-over-msgr2)

## Further reading

Upstream documents relevant to compatibility, as paths in ceph.git at commit `5f7e383ae6d`. They are not part of this directory; in a ceph.git clone, `git show 5f7e383ae6d:<path>` prints one. Several of them disagree with the code; the last column points to the page that compares them.

| Document | Subject | Covered in the map |
|---|---|---|
| `doc/cephfs/posix.rst` | where CephFS departs from POSIX | [POSIX contract 7.2](cpp-map/03-request-path/posix-semantics-contract.md#72-departures-documented-in-doccephfsposixrst) checks each statement against the code |
| `doc/cephfs/kernel-features.rst` | kernel versions per CephFS feature | [kclient expectations 7.1](cpp-map/02-client-protocol/kclient-expectations.md#71-client-generations-and-protocol-fallbacks) (the document has no feature-bit list) |
| `doc/cephfs/upgrading.rst` | manual MDS upgrade procedure | it says the MDS cluster has no built-in versioning, while compat sets gate rank assignment ([FSMap and MDSMap 3.5](cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#35-compatset-mechanics)); cephadm runs the procedure ([mgr 3.8](cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md#38-cephadm-deploying-and-upgrading-mds-daemons)) |
| `doc/cephfs/file-layouts.rst` | layout virtual xattrs | [attributes and xattrs 3.8](cpp-map/03-request-path/attributes-and-xattrs.md#38-virtual-xattrs-set-and-remove-srcmdsservercc6342-serverhandle_client_setvxattr) |
| `doc/cephfs/lazyio.rst` | LazyIO | it describes libcephfs only; the kernel implements it too (`linux/fs/ceph/ioctl.c:248 (ceph_ioctl_lazyio)`); MDS side: the `Fl` cap ([capabilities 3.1](cpp-map/02-client-protocol/capabilities.md#31-the-cap-bit-model)) |
| `doc/cephfs/app-best-practices.rst` | costs of `stat`, hard links, large directories and working sets | [capabilities](cpp-map/02-client-protocol/capabilities.md), [lookup 3.2](cpp-map/03-request-path/lookup-readdir-open-and-file-locks.md#32-getattr-lookup-and-lookupsnap-serverhandle_client_getattr), [cache lifecycle](cpp-map/04-cache/mdcache-lifecycle-trim-and-memory.md) |
| `doc/cephfs/full.rst` | behaviour of a full cluster | it says metadata operations other than deletes and truncates get `ENOSPC`; the code gates a fixed list of ops on the metadata pool's full flag, and clients see `EACCES` ([RADOS and OSD 3.8](cpp-map/07-storage-formats/rados-and-osd-interactions.md#38-full-cluster-behaviour-enospc-paths)) |
| `doc/cephfs/fscrypt.rst` | fscrypt from the user's side | [attributes and xattrs 3.3.6](cpp-map/03-request-path/attributes-and-xattrs.md#336-fscrypt-truncate-last-block-read-modify-write), [kclient expectations 3.19](cpp-map/02-client-protocol/kclient-expectations.md#319-fscrypt-and-alternate-names) |
| `doc/cephfs/charmap.rst` | name normalization and case folding | it says the kernel does not understand charmap; the C++ MDS lets kernels that set bit 22 create entries in charmap directories ([kclient expectations 7.1.4](cpp-map/02-client-protocol/kclient-expectations.md#714-the-bit-22-conflict)) |
| `doc/cephfs/mds-states.rst` | MDS states | it spells `up:standby_replay`, the code `up:standby-replay` ([rank lifecycle 8](cpp-map/09-recovery/rank-lifecycle-and-recovery.md#8-edge-cases-failure-handling-and-gotchas), [FSMap and MDSMap 3.3](cpp-map/01-cluster-integration/fsmap-and-mdsmap.md#33-mds-daemon-and-rank-states)) |
| `doc/cephfs/capabilities.rst` | capabilities for users | [capabilities](cpp-map/02-client-protocol/capabilities.md) |
| `doc/cephfs/client-auth.rst` | MDS caps and path restriction | [auth and MDS caps 3.4](cpp-map/01-cluster-integration/auth-and-mds-caps.md#34-the-mds-cap-language) |
| `doc/cephfs/multimds.rst` | multiple active ranks and pinning | it says ephemeral pins are off by default "in Octopus"; both options default to true ([balancer and pinning 8](cpp-map/08-multi-mds/balancer-and-pinning.md#8-edge-cases-failure-handling-and-gotchas)) |
| `doc/cephfs/fs-volumes.rst` | mgr/volumes, subvolumes, quiesce, quarantine | it says quarantine fails closed when the subvolume root is not cached; the code fails open ([quarantine 8](cpp-map/12-qos-metrics-quiesce-quarantine/quarantine.md#8-edge-cases-failure-handling-and-gotchas)); [mgr 3.2](cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md#32-mgrvolumes), [quiesce](cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md) |
| `doc/dev/msgr2.rst` | msgr2.0 and msgr2.1 | it lists a `TAG_CLOSE` frame the code lacks, leaves `AUTH_SIGNATURE` out of its frame list, and says `REVISION_1` is the only feature while the code also defines `COMPRESSION` (`src/include/msgr.h:53-54 (DEFINE_MSGR2_FEATURE)`) ([transport 3.4 and 8](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#8-edge-cases-failure-handling-and-gotchas)) |
| `doc/dev/network-protocol.rst` | msgr1 (legacy) | banner order and several tags differ from the code ([transport 3.3 and 8](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#33-msgr1-legacy-protocol)) |
| `doc/dev/network-encoding.rst` | primitive encodings | [transport 3.10](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#310-encoding-framework-and-golden-data), [wire protocol 3.2](cpp-map/02-client-protocol/wire-protocol.md#32-encoding-primitives) |
| `doc/dev/cephx_protocol.rst` | cephx code walk-through (2012) | it refers to the removed `SimpleMessenger` ([transport 8](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#8-edge-cases-failure-handling-and-gotchas)) |
| `doc/dev/cephx.rst` | cephx message formats | it shows the rotating-key reply as one key and says services keep the current and the prior key; the code sends the whole set of rotating secrets and services keep three ([transport 3.6 and 8](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#36-cephx-on-the-service-side)) |
| `doc/dev/session_authentication.rst` | msgr1 message signatures (legacy) | it says `cephx_sign_messages` defaults to false; the default is true (`src/common/options/global.yaml.in:2302 (cephx_sign_messages)`) ([transport 3.6](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#36-cephx-on-the-service-side)) |
| `doc/dev/encoding.rst` | versioned encode and decode rules | [transport 3.10](cpp-map/00-architecture/transport-auth-and-client-library-contract.md#310-encoding-framework-and-golden-data) |
| `doc/dev/corpus.rst` | encoding corpus and `readable.sh` | `readable.sh` also honours `backward_incompat` markers, which the document does not mention (`src/test/encoding/readable.sh:118-123 (test_object)`); [QA 3.6](cpp-map/14-testing/qa-and-conformance-suites.md#36-ceph-dencoder-and-the-object-corpus) |
| `doc/dev/cephfs-fscrypt.rst` | fscrypt internals: names, blocks, truncate handshake | [kclient expectations 3.19](cpp-map/02-client-protocol/kclient-expectations.md#319-fscrypt-and-alternate-names), [attributes and xattrs 3.3.6](cpp-map/03-request-path/attributes-and-xattrs.md#336-fscrypt-truncate-last-block-read-modify-write) |
| `doc/dev/cephfs-mirroring.rst` | cephfs-mirror design | it says data is synchronized by bulk copying; the daemon uses snapdiff and blockdiff when it can ([mgr 3.4](cpp-map/01-cluster-integration/mgr-tools-and-orchestration.md#34-snapshot-mirroring)) |
| `doc/dev/cephfs-reclaim.rst` | session reclaim interface | [sessions 3.9](cpp-map/02-client-protocol/sessions.md#39-reclaim-protocol-nfs-ganesha-via-libcephfs) |
| `doc/dev/cephfs-snapshots.rst` | snapshot internals | it does not mention the realms created for quota and subvolume roots ([snapshots 3.2](cpp-map/10-snapshots-quotas-subvolumes/snapshots.md#32-snap-realms)) |
| `doc/dev/mds_internals/index.rst` | index of the MDS developer documents | — |
| `doc/dev/mds_internals/data-structures.rst` | MDS cache data structures | [cinode](cpp-map/04-cache/cinode-and-inode-types.md), [cdir and cdentry](cpp-map/04-cache/cdir-cdentry-and-cache-objects.md) |
| `doc/dev/mds_internals/exports.rst` | subtree export | outdated against the code ([subtree migration 8](cpp-map/08-multi-mds/subtree-migration.md#8-edge-cases-failure-handling-and-gotchas)) |
| `doc/dev/mds_internals/locking.rst` | the Locker | incomplete: omits `IQUIESCE` and misdescribes `LOCK_LOCK` ([lock types 8](cpp-map/05-locking/lock-types-and-state-machines.md#8-edge-cases-failure-handling-and-gotchas)) |
| `doc/dev/mds_internals/quiesce.rst` | the quiesce protocol | it says every operation takes the `quiescelock`; the code adds it once per `acquire_locks` call ([quiesce 8](cpp-map/12-qos-metrics-quiesce-quarantine/quiesce.md#8-edge-cases-failure-handling-and-gotchas)) |
