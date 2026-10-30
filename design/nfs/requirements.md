# NFS server design: requirements and evaluation rubric

> Shared brief for the knfsd-replacement design effort (ledger `current/nfs-server-design.md`). Draft 1, 2026-10-30. This document pins the decisions already taken and the questions every option study must answer. It is the single source of truth for the three option studies. A study does not reopen a pinned decision. It may record an objection in its "Open questions" section.

## 1. Problem

Cloud guests want NFS mounts, not CephFS mounts. Today's answer is NFS-Ganesha with FSAL_CEPH: a centralized gateway tier behind an ingress virtual IP, a RADOS-backed grace database for node takeover, and one libcephfs client per Ganesha daemon with caps `allow rw path=/`. Tenant isolation is Ganesha's export configuration. The data path costs two network hops per byte. Failover is an NFS grace period for every client of the failed gateway. One tenant's load can starve the others inside the shared session.

The target is a per-host NFS server, called "the proxy" below. It runs on every hypervisor, serves only the guests on that host, speaks CephFS to the Rust MDS and RADOS to the OSDs. It replaces the role that knfsd would play if knfsd re-exported a kernel CephFS mount. The Rust MDS in `/mnt/workspace/rust-mds` may be adjusted for it.

## 2. Pinned decisions

| Id | Decision | Rationale |
|---|---|---|
| P1 | Guest-visible contract: a plain NFS mount with `sec=sys`, NFSv3 or NFSv4.1, to a well-known link-local address that the host intercepts on the VM's own tap device (the metadata-service pattern), or to a host-only virtio-net NIC. The export name is a volume identifier. The address is identical on every host. | Stock guest images, no guest agent, migration invisible at the address level. |
| P2 | vsock is not a first-release transport. The mainline NFS client has no AF_VSOCK transport (no vsock references under `/mnt/workspace/ceph-client/net/sunrpc` or `fs/nfs`). | It cannot be imposed on stock guests. |
| P3 | Identity: the host channel (tap device or NIC) identifies the VM. No Kerberos, no TLS, no AUTH_SYS trust across a network. Each tenant has its own cephx key with MDS `path=` restrictions and OSD pool-namespace restrictions. The MDS and the OSDs enforce tenant scope. | Isolation enforced by Ceph, not by proxy correctness. |
| P4 | Fate sharing: the proxy lives and dies with its host. No NFS high-availability tier, no ingress, no grace cluster. A proxy process restart is local. | Removes Ganesha's hardest operational machinery. |
| P5 | Migration-ready, not first. The state of a client mount is encapsulated as a movable object, the "capsule". While the kernel CephFS client serves as the proxy's client (decision D1), the MDS session is one per tenant per host and the capsule carries NFS state only; one MDS session per VM returns with the Rust CephFS client, if built (revised 2026-10-30, D2, accepted in principle). Level 1 (reclaim-based handover, the guest sees a server reboot) comes first. Level 2 (transparent state migration, replay caches included) comes later. The design must not preclude level 2. | VM live migration moves the guest to another proxy. |
| P6 | Zero-copy techniques are out of scope, but the architecture must expose a path: no mandatory copy between guest memory and the OSD write or read. Page-cache sharing among one tenant's guests on a host is desirable. | User requirement. |
| P7 | Protocols: NFSv3 with MOUNT, NLM and NSM; NFSv4.1 with sessions, state, grace, backchannel and delegations. NFSv4.0 is excluded. NFSv4.2 and pNFS come later. Delegations map to CephFS caps. | Cloud offerings use v3 or v4.1. The v4.1 backchannel needs no reverse route into the guest. |
| P8 | The Rust MDS is adjustable. The C++ MDS is not a target. The kernel CephFS client `fs/ceph` may be patched upstream. License wall: GPL kernel code is read for behaviour only. Nothing from it may be translated into the Rust tree. | Project rules in `AGENTS.md`. |
| P9 | Upstreaming is mandatory. Every option names its upstream, the maintainers or community that decide, precedent, and a realistic acceptance path as a sequence of submissions. | User requirement. |
| P10 | Permission enforcement happens on the server side of the NFS boundary, because the guest is untrusted. Where it happens (proxy, host kernel VFS, or MDS) is an option-specific answer. | NFS clients assert uid and gid. |

## 3. Facts established, with sources

Rust MDS and engine (`/mnt/workspace/rust-mds`):

- F1. LOOKUPINO, LOOKUPPARENT and LOOKUPNAME are implemented: `crates/engine/src/engine/ops_read.rs:125-180`. NFS handles of {fscid, ino, snapid} resolve against this engine.
- F2. Inode numbers are never reused, by design decision, because NFS handles carry no generation number: `design/minimum-feature-set.md`, bullet "Inode numbers" in section 4. A transactional inode-number index for any-rank resolution is planned, not built.
- F3. The engine performs no permission enforcement. It trusts `caller_uid` and `caller_gid`. It returns EACCES nowhere and EPERM in two places (`crates/engine/src/engine/mod.rs:800`, `crates/engine/src/engine/ops_attr.rs:68`). The C++ MDS checks mode bits only under a cap with a `uid=` restriction (`/home/ubuntu/ceph/src/mds/MDSAuthCaps.cc:340-392`). libcephfs and the kernel client check client-side (`client_permissions`, default true).
- F4. `change_attr` exists and advances in setattr (`crates/engine/src/engine/ops_attr.rs:156`). Other mutation paths are unverified.
- F5. File locks carry an owner: `crates/engine/src/request.rs:46-51`.
- F6. The OSD client crate has an Objecter and OSD ops but no file striper. The daemon touches file data only for truncate and purge (`crates/osdc/src/ops.rs:432`).
- F7. Ganesha facts, from `design/minimum-feature-set.md` section 4.6 and `mds-rust-map/cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md` section 3.16: one libcephfs client per Ganesha daemon; the session multiplexes many NFS clients and uids; every SETATTR and COMMIT is a sync; handles carry {ino, snapid, fscid}; LOOKUPINO goes to a random rank; reclaim uses session metadata `uuid` and `timeout=300` with `MClientReclaim` RESET; delegations are built client-side on caps and the cephadm template disables them.
- F8. Engine calling model: wire-free `EngineInput`, `Engine::step`, one journal record per mutation, safe replies after commit, effects through a per-session send queue (`crates/mds/README.md`, `design/persistence-contract.md`). Durable sessions, locks reinstated at reconnect and a completed-request table exist (`design/recovery-contract.md`, `design/workplan.md`).

Linux kernel (`/mnt/workspace/ceph-client`, 7.3.0-rc6):

- F9. `fs/ceph/export.c` implements `encode_fh`, `fh_to_dentry`, `get_parent` and `get_name` on LOOKUPINO, LOOKUPPARENT and LOOKUPNAME (lines 149, 249, 332, 571, 621-625), plus snapshot handles (`ceph_nfs_snapfh`, line 29). knfsd can re-export a kernel CephFS mount today.
- F10 (corrected 2026-10-30). `fs/ceph` defines no `setlease` operation and sets no `export_operations` flags. In this tree `kernel_setlease` returns EINVAL when the file operations have no `setlease` (`fs/locks.c:2108-2116`), so nfsd grants no NFSv4 delegations on a CephFS re-export at all. Draft 1 of this brief stated a fallback to `generic_setlease`; that was wrong for this kernel. A `fs/ceph` `setlease` operation tied to cap revocation is therefore required to obtain delegations, not only to make them safe.
- F11. The kernel client sends no `MClientReclaim` and no `uuid` or `timeout` session metadata (map page section 3.17, line 749).
- F12. The NFS client implements NFSv4 migration recovery (`fs/nfs/nfs4state.c:2144`, `nfs4_handle_migration`, `NFS4ERR_MOVED` handling) and the `nconnect`, `clientaddr` and `xprtsec` mount options (`fs/nfs/fs_context.c:170,192,225`). knfsd has `xprtsec` export support (`fs/nfsd/export.h`).
- F13. `Documentation/filesystems/nfs/reexport.rst` lists the known re-export limits: reboot recovery, filehandle limits, open DENY bits ignored.
- F14. nfsd honours `EXPORT_OP_FLUSH_ON_CLOSE`, `EXPORT_OP_NOATOMIC_ATTR` and `EXPORT_OP_REMOTE_FS` (`fs/nfsd/filecache.c:327`, `fs/nfsd/nfsfh.c:338`, `fs/nfsd/vfs.c:1461`). The CephFS client sets none of them.
- F15. knfsd is network-namespace aware: `struct nfsd_net` (`fs/nfsd/netns.h:74`) holds per-namespace state, and `fs/nfsd/nfsctl.c` resolves it per net in 15 places. One nfsd instance per network namespace is possible.
- F16. Kernel `MAINTAINERS`: CEPH DISTRIBUTED FILE SYSTEM CLIENT is maintained by Ilya Dryomov, Alex Markuze and Viacheslav Dubeyko (list ceph-devel). KERNEL NFSD, SUNRPC is maintained by Chuck Lever and Jeff Layton, with reviewers NeilBrown, Olga Kornievskaia, Dai Ngo and Tom Talpey (list linux-nfs).

## 4. The three options

- Option A, userspace Rust proxy: a new NFS server crate (ONC RPC, XDR, NFSv3, MOUNT, NLM, NSM, NFSv4.1), a new CephFS client crate on the existing wire crates (`proto`, `msgr`, `monc`, `auth`, `osdc`), the capsule as a first-class serializable object, one process per VM or per tenant, sandboxed.
- Option B, in-kernel Rust NFS server: a Rust-for-Linux module that serves NFS from the host kernel and uses `fs/ceph` as the CephFS client through VFS and sunrpc bindings.
- Option C, knfsd-next in C. Two sub-variants, both analysed. C1: a clean rewrite of the NFS server in C. C2: an evolution of nfsd, sunrpc and `fs/ceph` by incremental upstream patches (VM identity transport or export matching, nfsd state export and import, a `fs/ceph` lease operation tied to caps, uuid reclaim, asynchronous lock grant, zero-copy transport hooks). The study states which sub-variant is upstreamable and why.

## 5. Evaluation rubric

Every option study answers R1 to R11 with evidence.

- R1 Guest-visible contract (P1, P7): how the option implements the link-local or host-only endpoint, export naming, `sec=sys`, NFSv3 and NFSv4.1.
- R2 Identity and tenant isolation (P3, P10): where the VM identity enters, where permission checks run, how tenant keys are provisioned and confined, what a compromised component can reach.
- R3 Crash containment and attack surface: what a proxy bug takes down; the size and language of the code that parses untrusted guest RPC.
- R4 Zero-copy readiness (P6): the data path from guest memory to the OSD write and from the OSD read to guest memory, with every copy named; page-cache sharing within a tenant.
- R5 Migration readiness (P5): where the capsule state lives; how quiesce, capture, two-phase commit and import work; what level 2 needs; what the option cannot encapsulate.
- R6 CephFS client integration: `fs/ceph` or a new Rust client; delegations on caps; locks by owner; handles; uuid reclaim; change attribute; strict-I/O sync behaviour.
- R7 Rust MDS requirements: the MDS adjustments the option needs, chosen from this candidate set: enforcing session policy (path scope, root squash, mode bits, chown rules, sticky directories); transactional inode index; change attribute on every mutation path; session takeover (per-session reconnect while the rank stays active, fencing by blocklist and OSD epoch barrier, cooperative handover); lean non-caching session mode; many small sessions; admin eviction API.
- R8 Performance: strict-I/O latency (sync after SETATTR and COMMIT), metadata round trips per NFS operation, throughput per host, CPU per byte, memory per VM.
- R9 Completeness and time to first product: what exists, what must be built, as work packages with relative size (S, M, L, XL) and dependencies.
- R10 Upstream path (P9): target tree or project, deciders, precedent, sequence of submissions, expected objections, and the maintenance cost if upstream rejects.
- R11 Operability: deployment and upgrade cadence, observability, proxy restart recovery, per-tenant resource control.

## 6. Verification assets available

- Ganesha with FSAL_CEPH on the dev cluster: `devcluster/ganesha.sh` (NFSv4.1/4.2 on port 12049), the reference peer and the oracle for MDS-side behaviour.
- Kernel client mounts: `devcluster/mount-kernel.sh`; a knfsd re-export of such a mount is the baseline to beat.
- Offline NFSv4.1 wire decoder: `scripts/g3_wire_nfs41.py` with fixtures under `scripts/tests/fixtures/g3-wire-nfs41/`.
- pynfs against a Ganesha export is in the design test plan (`design/minimum-feature-set.md`, section 9).

## 7. Steering updates after draft 1

Recorded 2026-10-30 from the user, after the studies started. They bind the synthesis; they do not rewrite the pinned table until the user confirms the proposed changes.

- Ignore Ganesha where possible. The Ganesha-per-host experiment is dropped; knfsd over per-tenant kernel mounts is the zero-code baseline.
- Assume kernel zero-copy on the host socket exists. Copy counts are not the deciding criterion.
- The single privilege domain is not a criterion for the user.
- Direction: start with a userspace Rust NFS server and improve from there.
- Proposed change to P5, awaiting confirmation: while the kernel CephFS client serves as the proxy's client, the MDS session is one per tenant per host, and the capsule carries NFS state only. See `design/nfs/comparison.md` section 1.

## 8. Deliverable format for an option study

One Markdown file with these sections, in this order: Summary (ten lines maximum); Architecture (components, data flow, control flow, process and privilege model, one ASCII diagram); R1 to R11, each with evidence; Shared work packages (what this option shares with the other two); Work packages; Upstream plan; Risks; Open questions.

Tag every factual claim: `[src: path:line]` for local source, `[web: URL, accessed date]` for web sources, `[judgment]` for the author's assessment. Write in the active voice with short sentences (ASD-STE100, `/mnt/workspace/AGENTS.md` section 0). Do not reopen pinned decisions. Do not modify any repository file, run builds or tests, or touch the dev cluster.
