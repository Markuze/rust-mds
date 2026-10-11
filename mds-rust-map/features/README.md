# CephFS MDS features catalog

> Part of the [MDS Rust reference set](../README.md). Facts about the C++ MDS at ceph.git commit 5f7e383ae6d; peers at the versions of the [baseline](../README.md#baseline-current-peers).

## 1. Purpose

This catalog lists, as facts only, what the C++ MDS does. It is input for deciding the minimal feature set of a new MDS. That MDS is not backward compatible with the C++ MDS, but it must work with current, unmodified peers: the Linux kernel client, ceph-fuse and libcephfs, NFS-Ganesha through libcephfs, OSDs, monitors, mgr modules, cephadm and Rook, at the versions of the [baseline](../README.md#baseline-current-peers). Entries that serve only older clients, msgr1 mounts or older releases are marked legacy (section 3), listed last on each page and collected in [legacy.md](legacy.md). Each entry says what the C++ MDS does, who relies on it and what it exposes. The catalog makes no recommendations: it does not say what to keep, drop, defer or redesign.

## 2. How it was built

- **Sources.** The verified pages of the [C++ MDS project map](../cpp-map/README.md) and the per-item staging notes in [../_staging](../_staging). The staging notes were written before the map pages were fact-checked. Where a note and a page disagree, the catalog follows the page.
- **Writers.** One writer per category. Each writer produced one catalog JSON file, and the matching Markdown page was rendered from it.
- **Review.** One combined review pass covered all categories. It ran the catalog checker, resolved duplicates across categories, checked consistency of classes and exposure, spot-checked entries against the code and searched for gaps. Results:

| Review item | Result |
|---|---|
| Duplicate entries resolved | 110: 54 retired into the owning entry (each file's "retired" list), 56 turned into pointers whose description starts with "See F-XXX-NNN." |
| Entries added | 0 |
| Entries reclassified | 107 |
| Spot checks against the code | 49 |
| Corrections from spot checks | 10 |
| See-also cross-references added | about 140 |
| Live features | 935 (down from 989) |
| Checker result | 0 errors, 11 warnings (non-contiguous IDs after retirement) |

**Legacy marking (2026-10-11).** After the baseline was set to current peers, six reviewers read every live entry against the code of the baseline peers: Linux 7.3, ceph `main`, NFS-Ganesha V15.7 and Rook v1.21.0. They marked 12 entries as legacy and 152 more as having legacy parts, each with evidence that current peers do not take the path. The same pass corrected 15 entries, including the msgr1 statements of F-PROTO-001, 002, 005, 013 and 026. [legacy.md](legacy.md) lists the marked entries and counts them by reason.

**Limits.** Apart from the 49 spot checks and the legacy marking, entries were not individually re-checked against the code. The map pages they summarise were fact-checked against the code. When an entry and its map page disagree, the map page is the more reliable source.

## 3. How to read an entry

Each category page has four sections: 1 Scope (what the category covers and which category owns excluded topics), 2 Summary (one table row per feature, by group), 3 Features (the full entries) and 4 Legend. Each entry has these fields. The JSON field names are given in brackets.

| Field | Meaning |
|---|---|
| ID [`id`] | `F-<CODE>-<NNN>`, for example F-NS-001. The anchor is the lower-case ID (`#f-ns-001`). |
| Group [`group`] | The subsection of the category the entry belongs to. |
| Name [`name`] | Short title. |
| Summary [`summary`] | One sentence, shown in italics under the title. |
| Description [`description`] | What the C++ MDS does: mechanism, wire fields, on-disk objects, defaults and error codes. A pointer entry starts with "See F-XXX-NNN." and keeps only the facts specific to its own category. |
| Class [`posix`] | One POSIX class (see below). |
| Exposure [`exposure`] | One or more exposure values (see below). |
| Relied on by [`relied_on_by`] | One or more peers that depend on the behaviour (see below). |
| Gates [`gates`] | Configuration options, feature bits, map flags or other conditions that switch or tune the behaviour, with defaults. Empty when nothing gates it. |
| Map [`map_refs`] | Links to the map sections the entry summarises. |
| Code [`code_refs`] | Citations in the form `path:line (symbol)` into ceph.git, or `linux/fs/ceph/<file>:<line> (symbol)` into the kernel client. |
| Notes [`notes`] | Edge cases, departures from POSIX, code-reading findings and "See also" cross-references. |
| Legacy [`legacy`] | Only on a legacy entry, one that serves only peers outside the baseline. Fields: `reasons` (see below); `note`, which says who uses the path and what current peers do instead; `evidence`, code citations; and sometimes `evidence_notes`, commits and searches. |
| Legacy parts [`legacy_parts`] | On a current entry that also contains a legacy path. Same fields as `legacy`; `note` names the legacy sub-paths. |

### POSIX classes

| Class | Meaning |
|---|---|
| `required` | Implements a POSIX-specified operation, attribute, error code or guarantee that applications see directly. |
| `supporting` | Not POSIX itself, but POSIX behaviour through unmodified clients depends on it (sessions, caps, leases, journaling for durability). |
| `extension` | CephFS-specific feature visible to applications or administrators beyond POSIX (snapshots, quotas, layouts, vxattrs, subvolumes, fscrypt, charmap). |
| `operational` | Cluster integration, high availability, administration, observability or tuning. |
| `internal` | Implementation mechanism of the C++ MDS with no external contract of its own. |

### Exposure values

| Exposure | Meaning |
|---|---|
| `client-wire` | Client protocol: MClient* messages, their fields, feature bits, reply encodings. |
| `cluster-wire` | Protocols with monitors, mgr and OSDs (beacons, FSMap, MMgrReport, MOSDOp, ...). |
| `mds-wire` | MDS-to-MDS messages between ranks (only Rust daemons will speak these, so they are free to redesign). |
| `on-disk` | Objects, omap and xattrs the MDS stores in RADOS. |
| `admin` | Admin socket, ceph tell, ceph fs / mds CLI commands and their output. |
| `config` | Configuration options. |
| `metrics` | Perf counters, health warnings, mgr metrics and cluster log. |
| `internal` | No external surface. |

### Legacy reasons

| Reason | Meaning |
|---|---|
| `old-clients` | Used only by clients older than the baseline (including the CentOS Stream kernel samples), or by client paths taken only towards an MDS that lacks a feature the C++ MDS advertises. |
| `msgr1` | The msgr1 protocol or one of its features, such as message signatures. Current mounts use msgr2. |
| `old-data` | Reads or upgrades on-disk or journal formats written by older Ceph releases (the C++ MDS or older mgr modules). |
| `old-daemons` | Interoperation with older MDS, monitor or OSD daemons (mixed-version clusters, upgrades). |
| `deprecated-admin` | Commands, options, health checks or metrics marked deprecated or obsolete that no current peer uses. |
| `dead-code` | C++ MDS code that cannot run at ceph.git commit 5f7e383ae6d. |

### Relied-on-by values

| Peer | Meaning |
|---|---|
| `kclient` | Linux kernel CephFS client (fs/ceph). |
| `libcephfs` | Userspace client library src/client (libcephfs API users). |
| `ceph-fuse` | ceph-fuse (built on the same src/client code). |
| `ganesha` | NFS-Ganesha FSAL_CEPH (via libcephfs). |
| `smb` | Samba vfs_ceph (via libcephfs). |
| `cephfs-mirror` | cephfs-mirror daemon (via libcephfs). |
| `mon` | Ceph monitors (MDSMonitor, FSMap, auth). |
| `mgr` | ceph-mgr and its modules (volumes, nfs, status, dashboard, prometheus, stats, mds_autoscaler, snap_schedule, ...). |
| `cephadm` | cephadm orchestrator. |
| `rook` | Rook operator. |
| `osd` | OSDs (RADOS operations, blocklists, snap contexts). |
| `tools` | cephfs-journal-tool, cephfs-data-scan, cephfs-table-tool, cephfs-top, cephfs-shell and similar tools. |
| `admin` | Operators and scripts using admin commands, logs and health output. |
| `mds-ranks` | Other MDS ranks of the same filesystem. |
| `internal` | Only the MDS itself. |

## 4. Categories

### Current and legacy entries

| Category | Features | Current | of which with legacy parts | Legacy |
|---|---|---|---|---|
| [Cluster membership and high availability](01-cluster-membership-and-ha.md) | 114 | 114 | 18 | 0 |
| [Client sessions and protocol](02-client-sessions-and-protocol.md) | 100 | 95 | 38 | 5 |
| [Capabilities, leases and locking](03-caps-leases-and-locking.md) | 87 | 86 | 9 | 1 |
| [Namespace and POSIX operations](04-namespace-and-posix-operations.md) | 91 | 91 | 17 | 0 |
| [Metadata cache and directory fragments](05-metadata-cache-and-dirfrags.md) | 87 | 87 | 12 | 0 |
| [Journal and storage](06-journal-and-storage.md) | 104 | 102 | 21 | 2 |
| [Multiple active MDS ranks](07-multi-mds.md) | 49 | 49 | 4 | 0 |
| [Snapshots, quotas and subvolumes](08-snapshots-quotas-subvolumes.md) | 54 | 52 | 6 | 2 |
| [Background services](09-background-services.md) | 58 | 57 | 12 | 1 |
| [QoS, metrics, quiesce and quarantine](10-qos-metrics-quiesce-quarantine.md) | 72 | 72 | 2 | 0 |
| [Security, administration and observability](11-security-admin-and-observability.md) | 119 | 118 | 13 | 1 |
| **Total** | **935** | **923** | **152** | **12** |

### POSIX classes by category

Counts are live features, without retired IDs, legacy entries included. Each feature has exactly one POSIX class.

| Code | Category | Features | required | supporting | extension | operational | internal |
|---|---|---|---|---|---|---|---|
| CLU | [Cluster membership and high availability](01-cluster-membership-and-ha.md) | 114 | 0 | 41 | 1 | 59 | 13 |
| PROTO | [Client sessions and protocol](02-client-sessions-and-protocol.md) | 100 | 0 | 73 | 7 | 15 | 5 |
| CAPS | [Capabilities, leases and locking](03-caps-leases-and-locking.md) | 87 | 0 | 41 | 8 | 4 | 34 |
| NS | [Namespace and POSIX operations](04-namespace-and-posix-operations.md) | 91 | 46 | 20 | 12 | 4 | 9 |
| CACHE | [Metadata cache and directory fragments](05-metadata-cache-and-dirfrags.md) | 87 | 8 | 14 | 4 | 10 | 51 |
| STORE | [Journal and storage](06-journal-and-storage.md) | 104 | 3 | 31 | 8 | 24 | 38 |
| MULTI | [Multiple active MDS ranks](07-multi-mds.md) | 49 | 0 | 10 | 8 | 11 | 20 |
| SNAP | [Snapshots, quotas and subvolumes](08-snapshots-quotas-subvolumes.md) | 54 | 1 | 1 | 40 | 3 | 9 |
| BG | [Background services](09-background-services.md) | 58 | 4 | 8 | 3 | 26 | 17 |
| QOS | [QoS, metrics, quiesce and quarantine](10-qos-metrics-quiesce-quarantine.md) | 72 | 0 | 1 | 25 | 29 | 17 |
| ADM | [Security, administration and observability](11-security-admin-and-observability.md) | 119 | 0 | 2 | 15 | 99 | 3 |
| | **Total** | **935** | **62** | **242** | **131** | **284** | **216** |

### Exposure by category

An entry can have several exposure values, so a row adds up to more than the category's feature count.

| Category | client-wire | cluster-wire | mds-wire | on-disk | admin | config | metrics | internal |
|---|---|---|---|---|---|---|---|---|
| Cluster membership and high availability | 29 | 66 | 18 | 18 | 25 | 13 | 9 | 7 |
| Client sessions and protocol | 87 | 15 | 5 | 4 | 6 | 17 | 5 | 1 |
| Capabilities, leases and locking | 39 | 1 | 10 | 9 | 3 | 14 | 4 | 29 |
| Namespace and POSIX operations | 76 | 5 | 8 | 32 | 3 | 8 | 2 | 4 |
| Metadata cache and directory fragments | 24 | 0 | 12 | 20 | 1 | 17 | 4 | 34 |
| Journal and storage | 9 | 14 | 5 | 69 | 6 | 10 | 5 | 11 |
| Multiple active MDS ranks | 13 | 4 | 12 | 8 | 5 | 12 | 1 | 12 |
| Snapshots, quotas and subvolumes | 37 | 6 | 8 | 23 | 2 | 7 | 2 | 1 |
| Background services | 8 | 6 | 7 | 17 | 13 | 10 | 6 | 6 |
| QoS, metrics, quiesce and quarantine | 21 | 6 | 7 | 2 | 17 | 8 | 11 | 13 |
| Security, administration and observability | 39 | 23 | 2 | 8 | 59 | 19 | 27 | 0 |
| **Total** | **382** | **146** | **94** | **210** | **140** | **135** | **76** | **118** |

### Peers

[by-peer.md](by-peer.md) lists, for each peer, every current feature it relies on; legacy entries are left out. A feature can name several peers. The counts below are current entries.

| Peer | Features |
|---|---|
| [kclient](by-peer.md#kclient) | 412 |
| [libcephfs](by-peer.md#libcephfs) | 423 |
| [ceph-fuse](by-peer.md#ceph-fuse) | 341 |
| [ganesha](by-peer.md#ganesha) | 324 |
| [smb](by-peer.md#smb) | 2 |
| [cephfs-mirror](by-peer.md#cephfs-mirror) | 21 |
| [mon](by-peer.md#mon) | 150 |
| [mgr](by-peer.md#mgr) | 156 |
| [cephadm](by-peer.md#cephadm) | 46 |
| [rook](by-peer.md#rook) | 54 |
| [osd](by-peer.md#osd) | 88 |
| [tools](by-peer.md#tools) | 136 |
| [admin](by-peer.md#admin) | 246 |
| [mds-ranks](by-peer.md#mds-ranks) | 139 |
| [internal](by-peer.md#internal) | 234 |

## 5. Reading the catalog against the new MDS's constraints

These are facts about the catalog fields measured against the stated constraints (drop-in for current, unmodified peers, no shared data or file system with C++ MDS daemons). They are not a selection. The counts leave out the 12 legacy entries.

- **What unmodified peers expect.** An entry whose exposure includes `client-wire` or `cluster-wire`, or whose relied-on-by list includes `kclient`, `libcephfs`, `ceph-fuse`, `ganesha`, `mon`, `mgr`, `cephadm`, `rook` or `osd`, describes behaviour that an unmodified peer expects from the MDS. 488 current entries have `client-wire` or `cluster-wire` exposure, 644 name one of those peers, and 649 meet either condition.
- **What is not a compatibility constraint.** `on-disk` and `mds-wire` exposure do not tie the new MDS to the C++ MDS, because the new MDS never reads C++ MDS data and never shares a file system with C++ MDS daemons. The exception is an entry that says another peer or tool reads the object or message, for example a mgr module, cephfs-mirror or a cephfs-* tool. Such entries say so in their description, relied-on-by list or notes. 253 current entries have only `on-disk`, `mds-wire` or `internal` exposure.
- **No external contract.** Current entries with POSIX class `internal` (214) describe how the C++ MDS implements something. They have no external contract of their own. The behaviour they serve is described by the entries they point to.
- **Legacy entries and parts.** The `legacy` and `legacy_parts` fields mark what serves only peers outside the baseline. By the stated constraints those paths are out of scope ([legacy.md](legacy.md)).
- **Supporting entries.** Current `supporting` entries (236) are not POSIX operations. Unmodified clients depend on them to get POSIX behaviour, for example sessions, caps, leases and the safe-reply durability point.

## 6. Machine-readable form

[features.json](features.json) is generated by merging all category catalogs. Its top-level keys:

| Key | Content |
|---|---|
| `description` | One-line description of the file. |
| `source` | `ceph_commit`, `ceph_commit_date`, `kernel_client` (the kernel snapshot cited as `linux/...`) and `baseline_peers`. |
| `vocabulary` | `posix`, `exposure`, `relied_on_by` and `legacy`: lists of `{value, meaning}` (the tables in section 3). |
| `categories` | One object per category: `category`, `code`, `title`, `file`, `count`, `map_pages`. |
| `features` | The 935 live entries. Fields: `id`, `group`, `name`, `summary`, `description`, `map_refs`, `code_refs`, `relied_on_by`, `posix`, `exposure`, `gates`, `notes`, `category`, and `legacy` or `legacy_parts` where they apply. |
| `retired` | `{id, merged_into}` for each of the 54 retired IDs. |

Current features the kernel client relies on:

```sh
jq -r '.features[] | select(.legacy == null) | select(.relied_on_by | index("kclient")) | "\(.id)\t\(.name)"' features.json
```

Features with client-wire exposure:

```sh
python3 -c 'import json; [print(f["id"], f["name"], sep="\t") for f in json.load(open("features.json"))["features"] if "client-wire" in f["exposure"]]'
```

## 7. ID rules

- An ID is `F-<CODE>-<NNN>`, where CODE is the category code in section 4.
- IDs are stable. They are never renumbered or reused. Because entries were retired, the numbers in a category are not contiguous.
- A retired ID is listed in features.json `retired` (and in its category file's `retired` list), together with the ID it was merged into. A reference to a retired ID resolves through that list.
- A pointer entry, whose description starts with "See F-XXX-NNN.", is still live. It keeps its ID and the facts specific to its category.
