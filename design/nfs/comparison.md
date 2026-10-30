# NFS server design: comparison and recommendation

> Synthesis of the three option studies for the knfsd-replacement effort (ledger `current/nfs-server-design.md`, brief `design/nfs/requirements.md`, MDS-side companion `design/nfs/mds-adjustments.md`). Draft 1, 2026-10-30. Raw studies and their evidence stay in `current/nfs-server-design/`: `option-a-userspace-rust.raw.md`, `option-b-kernel-rust.raw.md`, `option-c-kernel-c.raw.md`, the citation checker `tools/check_cites.py` and the agents' own ledgers under `agent-ledgers/`. Sizes use the studies' scale: S up to two engineer-weeks, M one to two months, L three to six months, XL more than six months.

## 1. Decision status

- **User direction (2026-10-30):** ignore Ganesha where possible; assume kernel zero-copy exists; the single privilege domain is not a criterion; start with a userspace Rust NFS server and improve from there.
- **Recommended, awaiting the user's confirmation:**
  - D1. Option A-K: a userspace Rust NFS server, one process per VM, over the kernel CephFS client `fs/ceph` through one kernel mount per tenant on each host. The Rust CephFS client crate of the Option A study is deferred.
  - D2. One MDS session per tenant per host while the kernel client is used. The capsule carries NFS state only. This changes pinned decision P5 ("one MDS session per VM").
- **Rejected:** Option B as a product (not viable within three years, under 5 percent merge odds); C1, a clean rewrite (not upstreamable); libcephfs under a userspace server (C++ in the process, no state export, a dormant non-RESET reclaim that both MDS implementations refuse).
- **Kept as later options:** the Rust CephFS client crate, for per-VM sessions, lossless session takeover and custom transports; B2a, generated Rust XDR inside nfsd, as a goodwill track with the NFSD maintainers if the kernel route is ever needed.

## 2. Corrections to the brief, verified in the trees

- **F10, delegations on a CephFS re-export.** Both kernel studies found, and the tree confirms, that `kernel_setlease` returns EINVAL when a filesystem has no `setlease` operation (`fs/locks.c:2108-2116`). nfsd therefore grants no delegations on CephFS at all. `fs/ceph` has refused leases since 2020, and that commit names the fix: tie leases to caps as libcephfs does (Option C study, commit 496ceaf12432). The brief is updated.
- **F3, permission enforcement.** The daemon reports every session as unrestricted and parses no MDS caps string (`crates/mds/src/server.rs:925-938`). The Option C study adds why this is the tenant boundary for kernel-client routes too: with `no_subtree_check` nfsd accepts any dentry of the superblock, and `fs/ceph` resolves any inode number through LOOKUPINO. M1 in `mds-adjustments.md` is a day-one item for every option. nfsd 7.1 adds a `sign_fh` export option that stops guests forging handles (`fs/nfsd/nfsfh.c:147-191`).
- **Session scale.** The daemon documents a 1,024-session runtime limit per rank (`crates/mds/README.md:112`) and admits one journal record at a time (`HANDOVER.md:151`). Group commit and a higher session limit are prerequisites for any option; D2 removes the per-VM pressure.
- **Change attribute to userspace.** The statx change cookie is kernel-only (`fs/stat.c:706-707,756-759`). A userspace server over the kernel client cannot read the CephFS change attribute through statx. `fs/ceph` itself keeps it in the inode version (`fs/ceph/inode.c:52`, `fs/ceph/file.c:2533`).
- **Lock owners through the kernel client.** `fs/ceph` derives the MDS lock owner from the open file description owner (`fs/ceph/locks.c:111`). OFD locks on one descriptor per NFS lock owner give distinct MDS owners.
- **Handles through the kernel client.** `open_by_handle_at` needs CAP_DAC_READ_SEARCH (`fs/fhandle.c:306`); the CephFS export operations resolve handles by LOOKUPINO (`fs/ceph/export.c:149`).

## 3. Verdicts of the three studies

- **Option A, userspace Rust (study: 4,999 words, 66 source and 15 web citations).** One unprivileged process per VM with four new crates (`xdr`, `oncrpc`, `nfs`, `cephfs-client`) on the existing wire crates. Strengths: the capsule by construction, memory-safe parsing, in-process tests against the engine. Weaknesses: the most new code, with both the NFSv4.1 server and the CephFS client sized XL; one more host copy on reads than an in-kernel server on the same transport; msgr1 only until an msgr2 client exists. No reusable Rust NFSv4.1 server, NLM or NSM crate exists. Recommends NFSv4.1 first without delegations and level 1 migration by RESET reclaim.
- **Option B, in-kernel Rust.** Not viable as an upstream product within three years. Linux 7.3-rc6 has no Rust file under `fs/` or `net/` and no abstraction for sunrpc, sockets, VFS calls, locks, leases or exportfs. Kernel policy merges an abstraction only with an in-tree user, and a Rust duplicate of healthy C code needs the owning maintainers' consent. The NFSD maintainers' own Rust plan is generated XDR inside nfsd through xdrgen. A Rust panic still calls BUG. Out of tree means a patched fleet kernel against 558 commits a year under `rust/kernel/`.
- **Option C, kernel C (154 citations checked by script).** C1 is not upstreamable: nfsd is healthy and moving toward the rewrite's goals on its own. C2 is: the first product needs no kernel patch (one nfsd instance per VM network namespace, one `fs/ceph` mount with the tenant key), and the `fs/ceph` series PS2 to PS5 are S to M with in-tree precedent and named deciders. Weaknesses: about 81,000 lines of ring-0 C handle untrusted guest RPC; level 2 migration needs an nfsd state export (PS6, XL, low to moderate odds, no precedent); nfsd kernel threads are not charged to the VM's cgroup; four helper daemons per VM namespace.

## 4. Comparison matrix

Columns: A (userspace Rust with its own CephFS client), A-K (userspace Rust over the kernel client, the synthesis), B (in-kernel Rust), C2 (nfsd evolution). C1 is omitted as not upstreamable.

| Rubric | A | A-K | B | C2 |
|---|---|---|---|---|
| R1 contract | per-VM process in the VM's tap namespace | same | nfsd per VM namespace | same as B |
| R2 identity, enforcement | proxy checks plus MDS policy; MDS `path=` needed | VFS and `ceph_permission` enforce under the request's filesystem uid; MDS `path=` needed | VFS; MDS `path=` needed | same as B; `sign_fh` against forged handles |
| R3 crash scope, parser | one VM's mount; about 25,000-35,000 lines of Rust | same | host; Rust panic is BUG | host; about 81,000 lines of C |
| R4 copies, write / read | 2 / 3 today; 0 / 1 with vhost-user | 2 / 2 with splice, like nfsd; page cache shared per tenant | 2 / 2; page cache shared only with per-tenant mounts | same as B |
| R5 migration | full capsule; level 2 by construction; Ceph state by takeover | NFS capsule complete; Ceph caps and locks re-created on the destination with a lock window | level 1; level 2 needs an nfsd state export | same as B (PS6, XL) |
| R6 Ceph client | new XL client; msgr1 only | `fs/ceph`; msgr2; delegations need PS2; change attribute needs a kernel change | `fs/ceph` behind missing abstractions | `fs/ceph` plus PS2-PS5 |
| R7 MDS items | M1, M3, M6 day one; per-VM sessions; M4 later | M1, M3 day one; per-tenant sessions; M4 only with the Rust client | M1, M2, M3, M6, M7 | same as B; lock reservation on reclaim |
| R8 performance | one extra read copy; syscalls per RPC | nfsd-equal copies; syscalls per RPC | lowest CPU per byte | same as B |
| R9 time to product | NFS server XL plus client XL | NFS server XL (v4.1) or L (v3); no client to write | three to four years | product now with zero new code |
| R10 upstream | new project; Ceph for MDS semantics | new project plus `fs/ceph` series on ceph-devel | six-step abstraction ladder, under 5 percent | PS5, PS2, PS3, PS4 within about 12 months; PS6 uncertain |
| R11 operations | package deploy; per-VM process, cgroup, metrics | same | patched fleet kernel | per-VM namespace with four helpers; kthreads unaccounted until PS8 |

## 5. Why A-K is the synthesis

The Option A study assumed a new Rust CephFS client and sized it XL, the critical path of the whole option. The Option C study showed that the kernel client already supplies, on every host, what that crate would have to reproduce: handles, caps, locks, msgr2, permission enforcement in the VFS, and the page cache. A userspace Rust server can use all of that through the VFS, so the client crate leaves the first release and the NFS server becomes the only XL item.

What A-K keeps from Option A: the per-VM process, the Rust parser, the NFS state as a serializable capsule, in-process tests, package deployment and transport freedom. What it takes from C2: `fs/ceph` as the client and the `fs/ceph` series PS2 to PS5, which the project can drive on ceph-devel. What it gives up until the Rust client exists: per-VM MDS sessions, lossless session takeover, and the Ceph half of a level 2 capsule.

The mechanics, each verified in the tree:

- **Handles.** `name_to_handle_at` and `open_by_handle_at` with CAP_DAC_READ_SEARCH; the kernel's CephFS export operations resolve by LOOKUPINO.
- **Identity and permissions.** The request thread switches its filesystem uid and gid to the squashed NFS caller; the VFS and `ceph_permission` enforce. The proxy carries no permission code. M1 in the MDS is defence in depth and the tenant boundary.
- **Data.** Reads splice from the page cache to the socket; writes splice from the socket into the page cache. These are nfsd's copy counts, and the page cache is shared by all of a tenant's VMs on the host. Kernel zero-copy on the host socket applies unchanged.
- **Locks.** One open file description per NFS lock owner, with OFD locks, gives distinct MDS lock owners. Blocking locks wait in a thread or are retried.
- **Delegations.** None until PS2 lands in `fs/ceph`. A server may always decline delegations.
- **Change attribute.** Not readable from userspace today. Two fixes: a small `fs/ceph` ioctl that returns the MDS change attribute, or a VFS change that exposes the statx change cookie to privileged callers. Until one lands, ctime stands in.
- **Migration.** The NFS capsule moves intact. The destination reopens every file by handle before the source closes it, which avoids stray purge of unlinked open files, then re-takes the locks. Between the source release and the destination acquire another client can take a lock. Closing that window needs per-VM sessions and session takeover, which is the Rust client's job later.

## 6. Data path and copies

Verified in the kernel tree, independent of the studies:

| Host copies per I/O | nfsd in the kernel | userspace with read and write | userspace with splice (A-K) |
|---|---|---|---|
| WRITE, socket to OSD | 2: socket to request pages (`net/sunrpc/svcsock.c:339-360`), pages to page cache; page cache to OSD socket by page reference | 2, or 1 with zero-copy send | 2 |
| READ, OSD to guest | 2: OSD socket to page cache; splice to the tap (`fs/nfsd/vfs.c:1086`); the vhost copy into the guest ring is common to all | 3 | 2 |

The guest's own copies and the vhost-net copy (`drivers/vhost/net.c:38-40`, zero-copy TX off by default) are identical for every option. With a vhost-user transport only a userspace backend reads guest memory directly without a new kernel transport. Strict-I/O latency is one MDS round trip plus one WAL commit per SETATTR or COMMIT for every option, and single-record admission makes the WAL the shared bottleneck.

## 7. Isolation and migration

- **Isolation.** The user has set the privilege domain aside as a criterion. The tenant boundary in every option is the MDS `path=` enforcement (M1) plus the OSD namespace caps, because the kernel client resolves any inode number for any handle. A-K adds crash scope per VM and a Rust parser, which remain useful for availability.
- **Level 1.** The destination presents the same server scope and identity, the guest sees a server reboot and reclaims in a per-VM grace. For A-K the NFS state travels in the capsule, so even level 1 needs no reclaim of NFS state; only the Ceph-side locks are re-taken.
- **Level 2.** For A-K the NFS half is by construction. The Ceph half needs per-VM sessions and takeover (M4), that is, the Rust client. For C2 it needs PS6, an XL nfsd series with no precedent.
- **Lock-steal window.** Present in level 1 for every option without takeover. The Option C study proposes an MDS lock reservation for the reclaiming session; this is a smaller MDS feature than takeover and serves A-K too. Add it to `mds-adjustments.md` as M9 when scheduled.

## 8. Upstream strategy for A-K

1. **PS5, `fs/ceph` export flags and `commit_metadata` (S).** To ceph-devel, Cc linux-nfs. First submission; a correctness fix for every knfsd re-export user.
2. **PS2, `fs/ceph` leases tied to caps (M).** RFC to ceph-devel and linux-fsdevel, read leases first, with multi-host recall tests on the dev cluster. Required for delegations in any kernel-client route. The 2020 commit by Jeff Layton names this design.
3. **Change-attribute exposure (S).** Either a `fs/ceph` ioctl or a statx policy change; the second touches VFS and needs linux-fsdevel.
4. **PS4, asynchronous lock grant (M).** Optional for A-K, because the userspace server can wait in a thread. Later.
5. **PS3, uuid and timeout reclaim in `fs/ceph` (M).** Not needed with per-tenant mounts. Only with per-VM mounts.
6. **The Rust NFS server.** Crates in the rust-mds repository first, because they test against the engine and share the wire crates. `xdr` and `oncrpc` to crates.io with RFC test vectors and fuzz targets. A separate repository under the Ceph organisation later, with `ceph-nvmeof` as precedent. The licence of rust-mds is undeclared and must be settled first; `osdc` contains an LGPL-2.1 CRUSH port.
7. **Ceph upstream.** Session takeover and lock reservation semantics, when the Rust client exists, so the C++ MDS could serve the same proxy.
8. **Review hygiene.** The workspace Git identity matches a listed `fs/ceph` maintainer. Each `fs/ceph` series needs an independent co-maintainer review, and the nfsd side needs two Reviewed-by tags by its maintainer profile.

## 9. Phased plan for A-K

- **Phase 0, no new code.** knfsd on one host over per-tenant kernel mounts, as the baseline and as the test bed for the MDS items. This replaces the Ganesha experiment.
- **Phase 1, foundations.** MDS M1 path scope with default off, M3 change-attribute test, M6 many-sessions benchmark. Kernel PS5.
- **Phase 2, first product.** The Rust NFSv4.1 server without delegations over the kernel client: `xdr` (A1, M), `oncrpc` (A2, M), NFSv4.1 COMPOUND (A5, XL), NFSv4.1 state (A8, L), process and sandbox (A10, M), capsule and level 1 handover (A11, M), plus a new VFS backend K1 (handles, filesystem-uid switching, OFD lock owners, splice I/O; M) and K2, the change-attribute source (S after the kernel change). NFSv4.1 first because it needs one port and no route into the guest; NFSv3 with NLM and NSM (A3, A4) follows if customers need it.
- **Phase 3, delegations.** Kernel PS2, then backchannel and delegations on leases (A9, M).
- **Phase 4, migration.** Level 2 for the NFS state (A12, L) with Ceph-side re-creation; MDS M7 eviction interface; MDS lock reservation on reclaim.
- **Phase 5, optional.** The Rust CephFS client (A6, A7, A16) for per-VM sessions and takeover (M4); a vhost-user transport (A15).

## 10. Open questions for the user

1. Confirm D1 and D2.
2. NFSv4.1 first, as recommended, or NFSv3 first?
3. Change attribute: kernel change now, or ctime until it lands?
4. Delegations in the first release, which puts PS2 on the critical path, or later?
5. Project home and licence for the Rust NFS crates.
6. Should B2a, generated Rust XDR for nfsd, run as a goodwill track with the NFSD maintainers, or stay on the watch list?
