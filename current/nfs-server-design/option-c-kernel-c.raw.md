# Option C: knfsd-next in C (C1 rewrite, C2 upstream evolution)

Option study C for the per-host NFS proxy, written against `rust-mds/design/nfs/requirements.md` (draft 1), 2026-10-30.

Tag conventions. Kernel paths are relative to `/mnt/workspace/ceph-client` (Linux 7.3.0-rc6). `ceph:` means `/home/ubuntu/ceph`. `rmds:` means `/mnt/workspace/rust-mds`. `git <hash>` is a commit in the local kernel history. Release numbers come from ancestry tests against the "Linux 7.x" release commits. `profile` means `Documentation/filesystems/nfs/nfsd-maintainer-entry-profile.rst`. `git counts` means `git rev-list --no-merges --count` between consecutive release commits for one path. `wc` and `grep` name commands run on the tree.

Terms. A *netns* is a network namespace. A *cap* is a CephFS capability that the MDS grants to a session. A *lease* is a VFS file lease; nfsd backs each NFSv4 *delegation* with one. The *DRC* is nfsd's duplicate reply cache. The *capsule* is the movable per-VM mount state (P5). *PS1*-*PS8* are the C2 patch series under "Work packages".

## Summary

- C2 is upstreamable. C1 is not: nfsd is healthy, and the kernel rarely accepts a second implementation of a healthy subsystem [judgment; evidence under "C1"].
- The first C2 product needs no kernel patch: one nfsd instance per VM netns and one `fs/ceph` mount per VM with the tenant key [src: fs/nfsd/netns.h:74; fs/ceph/super.c:1340].
- Brief fact F10 is stale: since 7.0, `kernel_setlease` returns `-EINVAL` for a NULL `->setlease`, so nfsd grants no delegations on CephFS [src: fs/locks.c:2108-2115; git 2b10994be716].
- PS2 (leases tied to caps) is therefore mandatory for P7. The maintainers named this design in 2020 [src: git 496ceaf12432].
- The `fs/ceph` series PS2-PS5 (shared with Option B) are S to M, and each has in-tree precedent [judgment].
- Level 1 migration needs userspace orchestration plus PS3. Level 2 needs PS6 (XL), an `fs/ceph` session handover (L) and an MDS handover (L) [judgment].
- Main weakness: about 81,000 lines of ring-0 C handle untrusted guest RPC; one bug exposes every tenant on the host [src: line counts in R3].
- P5 (one MDS session per VM) prevents page-cache sharing (P6) in C2 [src: fs/ceph/super.c:1249-1275,1340].

## Architecture

### Components (C2)

1. **Host agent** (userspace, root). It creates one netns per VM, places the VM's NFS NIC in it, mounts `fs/ceph`, configures nfsd over netlink, answers upcalls and drives migration [judgment].
2. **Per-VM nfsd instance.** `struct nfsd_net` holds the per-netns export caches, client tables, sessions, grace, client tracking, DRC, threads, versions, scope and file-handle key [src: fs/nfsd/netns.h:74-254]. Per-net thread pools exist since 2012, chosen as "simpler and non-racy" [src: git 9dd9845f084c].
3. **Per-netns helpers.** lockd keeps per-net hosts and grace [src: fs/lockd/netns.h:9-20]. rpc.mountd, rpc.statd, rpcbind and nfsdcld run in each netns [judgment]. The kernel reaches rpcbind through a per-netns abstract socket [src: net/sunrpc/rpcb_clnt.c:39,274-281,357].
4. **Per-VM `fs/ceph` mount** with the tenant cephx key and `noshare`, so each VM owns one MDS session [src: include/linux/ceph/libceph.h:32; fs/ceph/super.c:1340-1341].
5. **Rust MDS and OSDs.** They enforce tenant scope with cephx `path=` and pool-namespace caps (P3) [src: rmds:design/nfs/requirements.md:17].

### Data and control flow

The guest sends ONC RPC over TCP to the well-known address. The traffic reaches netns vm-n through the host-only NIC or a redirect (R1). An nfsd thread of that netns decodes the call, applies export squash rules and calls the VFS on the per-VM mount. `fs/ceph` answers from caches under caps, or it sends MDS requests and OSD operations. The agent controls every instance through netns-aware generic-netlink families [src: fs/nfsd/netlink.c:243; net/sunrpc/netlink.c:90; fs/lockd/netlink.c:40]. Since 7.2, export and ip-map upcalls also use netlink [src: Documentation/netlink/specs/nfsd.yaml:528; Documentation/netlink/specs/sunrpc_cache.yaml:99-126; git 912fc994b920].

### Process and privilege model

All guest-facing protocol code runs in ring 0 as kernel threads [src: net/sunrpc/svc.c:720-762]. The helpers run as root inside the VM netns. The agent confines each helper to a mount namespace that shows only its VM's mount, because a compromised mountd can export any visible path [judgment]. Tenant secrets live in the host kernel [judgment].

```
 guest VM n (stock image)                        hypervisor host
 +--------------------+  tap-n: host-only NIC inside netns vm-n,
 | NFS v3 / v4.1      |  or a tc redirect of the link-local address
 | client, sec=sys    |----virtio-net/vhost----+
 +--------------------+                        |
    +------------------------ netns vm-n ------v---------------------+
    | nfsd instance (struct nfsd_net): clients, sessions, slots,     |
    | stateids, DRC, grace, fh_key, scope    lockd statd mountd      |
    |                                        rpcbind nfsdcld         |
    +---------------------------+------------------------------------+
                                | VFS: exportfs, leases, POSIX locks
    +---------------------------v------------------------------------+
    | fs/ceph mount vm-n (noshare, tenant key, root = tenant subtree)|
    | caps, page cache, one MDS session (uuid = vm-n after PS3)      |
    +------------+-------------------------------------+-------------+
                 | msgr: MDS ops, caps, file locks     | msgr: RADOS I/O
          Rust MDS (path= scope, takeover)       OSDs (namespace caps)
 host agent (userspace): netns and tap setup, mounts, nfsd netlink
 configuration, upcall replies, nfsdcld and NSM records, migration
```

### C1: a clean rewrite in C

*What.* A new in-kernel server for NFSv3, MOUNT, NLM, NSM and NFSv4.1 only. One capsule object per VM holds all state and serializes by design. Identity comes from the transport, decoders come from xdrgen, and workers run in the VM's cgroup [judgment].

*Why.* nfsd's structure obstructs the goals in four places. State is spread over per-net tables and lists and is interleaved with VFS objects [src: fs/nfsd/netns.h:100-141; fs/nfsd/state.h:522-600]. Identity is the source address, because `ip_map` keys on an address only [src: net/sunrpc/svcauth_unix.c:101-107,967-971]. The DRC keys on address and port [src: fs/nfsd/nfscache.c:103-110]. Workers are kthreads, and fs/nfsd, fs/lockd and net/sunrpc/svc*.c contain no memcg or cgroup code [src: grep; net/sunrpc/svc.c:740].

*Verdict: not upstreamable* [judgment]. Each release from 6.7 to 7.3-rc6 added 16 to 132 non-merge commits to fs/nfsd [src: git rev-list counts]. The maintainers already move nfsd toward the C1 goals: RPC-with-TLS (6.4), dynamic threads (7.0), signed handles and generated NLM4 decoders (7.1), netlink upcalls (7.2) and netlink statistics (7.3) [src: git b3cbf98e2fdf, 1c87a0c39a86, 2a83ffc55750, 912fc994b920, 36ed32f46e2a; fs/lockd/nlm4xdr_gen.c]. The kernel takes a second implementation when the incumbent is weak or its owners lead. The 7.1 ntfs resurrection cited ntfs3's state and 326 against 273 passing xfstests [src: git cdd4dc3aebea]. Android's own team wrote Rust Binder and still wrote "We're generally not proponents of rewrites" [src: git eafedbc7c050]. For SCSI targets there was room for only one implementation, and SCST never merged [web: https://lwn.net/Articles/420691/, accessed 2026-10-30; web: https://lwn.net/Articles/424004/, accessed 2026-10-30]. Two NFS servers would also contend for port 2049, lockd, `fs/nfs_common` and nfs-utils [judgment]. C1 survives only as an out-of-tree module or as a design reference for C2 [judgment].

## R1 Guest-visible contract

- Host-only NIC: tun creates its device in the opener's netns, so QEMU can create the tap inside netns vm-n [src: drivers/net/tun.c:2845,2939,3551-3554]. nfsd listens there on the well-known link-local address [src: Documentation/netlink/specs/nfsd.yaml:485]. Every netns on every host uses the same address (P1) [judgment].
- The VM's own tap: a tc ingress redirect moves traffic for the link-local address over a veth into netns vm-n, with no kernel patch [judgment]. nfsd listeners cannot bind to a device: the netlink listener takes only addresses [src: Documentation/netlink/specs/nfsd.yaml:163-169,485-492; grep: no SO_BINDTODEVICE in net/sunrpc]. Only an IPv6 link-local scope id ties an address to an interface [judgment].
- Versions are per netns, so the agent enables v3 and v4.1 and disables v4.0 [src: fs/nfsd/netns.h:180-181; Documentation/netlink/specs/nfsd.yaml:468].
- Export naming: a CephFS export requires `fsid=` or a uuid, because `fs/ceph` uses an anonymous device [src: fs/nfsd/export.c:1131-1137; fs/ceph/super.c:1240]. The agent derives it from the volume identifier on every host [judgment].
- NFSv3: MOUNT through rpc.mountd, NLM through lockd and NSM through rpc.statd, all per netns [src: fs/lockd/netns.h:9-20; judgment].
- NFSv4.1: sessions, with the backchannel on the fore channel; the callback program is per netns since 7.1 [src: git 42cc13995967]. Delegations need PS2 (R6).

## R2 Identity and tenant isolation

- Identity enters at the netns boundary, because an instance receives only its VM's packets [judgment]. `svcauth_unix_set_client` maps the source address inside the per-net cache, so any address in netns vm-n means VM n [src: net/sunrpc/svcauth_unix.c:938-1008].
- Permission checks run in the host VFS as the caller, after nfsd applies squash rules [src: fs/nfsd/auth.c:22]. `ceph_permission` calls `generic_permission` [src: fs/ceph/inode.c:3101-3113]. MDS checks are defence in depth (R7).
- Handle forgery: with `no_subtree_check`, nfsd accepts any dentry of the superblock [src: fs/nfsd/nfsfh.c:31-39], and `ceph_fh_to_dentry` resolves any inode number with LOOKUPINO [src: fs/ceph/export.c:149-150]. The MDS `path=` check is therefore the tenant boundary (P3). The `sign_fh` export option (7.1) adds a per-netns SipHash MAC that guests cannot forge [src: fs/nfsd/nfsfh.c:147-191; git 2a83ffc55750].
- Keys: the agent obtains one cephx key per tenant and hands it to the kernel at mount time [judgment].
- Reach of a compromise: an exploit of nfsd, sunrpc or lockd owns the host kernel, every mounted tenant key and every VM on the host [judgment]. A confined helper can change only its own netns exports [judgment].

## R3 Crash containment and attack surface

- Untrusted guest RPC meets ring-0 C: fs/nfsd 56,440 lines, fs/lockd 11,603 lines, sunrpc server side 13,373 lines [src: wc of fs/nfsd/*.[ch], fs/lockd/*.[ch], net/sunrpc/{svc*,svcauth*,cache,xdr}.c].
- A bug oopses or panics the host and kills all its VMs, not one [judgment]. rpcbind, rpc.mountd and rpc.statd also parse guest RPC, in userspace C as root [judgment].
- The bug class is current. Four nfsd use-after-free fixes in state revocation carry the date 2026-07-09. An `fs/ceph` fix for a pre-auth out-of-bounds read carries 2026-05-29 [src: git log fs/nfsd "Prevent client use-after-free"; git 4dbc71bcaf9a].
- Mitigations: xdrgen decoders [src: fs/nfsd/nfs4xdr_gen.c; fs/lockd/nlm4xdr_gen.c], unused versions off, `sign_fh`, fuzzing [judgment].
- C1 adds a new, unaudited parser of the same size class [judgment].

## R4 Zero-copy readiness

- READ: the OSD reply lands in the page cache with one socket receive copy [judgment]. nfsd splices page references from the page cache [src: fs/nfsd/vfs.c:1011-1043,1086-1105,1605-1606], but `fs/ceph` splices only while it holds `Fc` and copies otherwise [src: fs/ceph/file.c:2340-2371]. nfsd sends with `MSG_SPLICE_PAGES`, without a copy [src: net/sunrpc/svcsock.c:1245-1285]. vhost-net copies into guest buffers [judgment].
- WRITE: vhost-net copies guest buffers into skbs [judgment]. `svc_tcp_read_msg` copies them into `rq_pages` [src: net/sunrpc/svcsock.c:339-370]. Buffered writes copy into the page cache; `NFSD_IO_DIRECT` avoids that copy [src: fs/nfsd/vfs.c:1376,1502-1511], because `ceph_direct_read_write` takes page references [src: fs/ceph/file.c:1519,1603]. The `fs/ceph` sync-write path copies [src: fs/ceph/file.c:1760,2034]. libceph sends data pages with `MSG_SPLICE_PAGES` [src: net/ceph/messenger_v1.c:80-98; net/ceph/messenger_v2.c:159-167].
- Path to zero copy: a `svc_xprt_class` with `xpo_result_payload` places payload pages itself, as svcrdma does [src: include/linux/sunrpc/svc_xprt.h:15-31; net/sunrpc/xprtrdma/svc_rdma_transport.c:94]. The architecture adds no mandatory copy (P6) [judgment]. See PS7.
- Page-cache sharing: `fs/ceph` shares a superblock only between mounts with identical options, and `noshare` disables sharing [src: fs/ceph/super.c:1249-1275,1340]. One session per VM therefore means one page cache per VM (open question 2).

## R5 Migration readiness

- Capsule location: the VM's `nfsd_net` [src: fs/nfsd/netns.h:74-254], the per-VM mount (caps, locks, dirty pages) and its MDS session. Slot replay caches hold up to 2,048 bytes per slot [src: fs/nfsd/state.h:355-376].
- Level 1 (the guest sees a server reboot): the agent copies the VM's nfsdcld and NSM records to the destination. It reuses the scope, the `fsid=` values and the `fh_key` there [judgment]. The scope is settable and serves as server owner and server scope [src: fs/nfsd/nfssvc.c:767-768; fs/nfsd/nfs4xdr.c:5683,5734]. On the destination, sm-notify tells the guest to reclaim its NLM locks [judgment]. nfsdcld must run per netns, because legacy tracking refuses non-initial namespaces [src: fs/nfsd/nfs4recover.c:523-526]. It uploads known clients at grace start [src: fs/nfsd/nfs4recover.c:711-767]. With nfsdcld, grace ends when every known client sends RECLAIM_COMPLETE, which with one client is almost immediate [src: fs/nfsd/nfs4recover.c:1387; fs/nfsd/nfs4state.c:2943-2965]. lockd grace always runs its full timer [src: fs/lockd/svc.c:101-112]. The destination mount RESET-reclaims the source session by uuid (PS3), with blocklist fencing and an OSD epoch barrier [src: ceph:src/mds/Server.cc:523-559; net/ceph/osd_client.c:2628-2640].
- Level 1 gap: after the source session dies and before the guest reclaims, a client on another host can take the guest's locks. This is the `reexport.rst` reboot-recovery limit [src: Documentation/filesystems/nfs/reexport.rst:25-35]. An MDS lock reservation for the reclaiming session closes it (R7).
- Level 2: PS6 plus `fs/ceph` session handover plus MDS cooperative handover. The destination instance is new, so import restores a whole `nfsd_net`, including the boot time that clientid checks use [src: fs/nfsd/nfs4state.c:2697,3118].
- Not encapsulable: host TCP sockets, so the guest sees a reset while its v4.1 session survives through slot replay [judgment]; v3 retransmissions after reconnect miss the DRC, whose key includes the port [src: fs/nfsd/nfscache.c:103-110]. Import must re-create files, POSIX locks and leases [judgment].

## R6 CephFS client integration

- Client: `fs/ceph` through the VFS and exportfs (F9) [src: fs/ceph/export.c:620-626].
- Delegations on caps: none today (open question 1) [src: fs/locks.c:2108-2115]; PS2 adds them.
- Locks by owner: `fs/ceph` sends `secure_addr(owner)` with bit 63 set, a per-host value [src: fs/ceph/locks.c:17-26,111]; the engine keeps owners (F5).
- Handles: 8, 16 or 28 bytes of CephFS data after the nfsd fsid, without a generation number [src: fs/ceph/export.c:15-35]. F2 is required.
- uuid reclaim: absent; session metadata is hostname, kernel_version, entity_id and root [src: fs/ceph/mds_client.c:1651-1655]. PS3 adds it.
- Change attribute: `fs/ceph` reports the MDS `change_attr` as the change cookie [src: fs/ceph/inode.c:1109-1110,3181-3183]. F4 must cover every mutation path.
- Strict I/O: on a `sync` export, nfsd falls back to `sync_inode_metadata` [src: fs/nfsd/vfs.c:368-385], and `ceph_write_inode` waits for the cap flush only [src: fs/ceph/caps.c:2575-2602]. `ceph_fsync` also flushes the MDS log and waits for unsafe requests [src: fs/ceph/caps.c:2405,2522-2545]. The Rust MDS sends only safe replies (F8), so the fallback suffices; PS5 covers early-reply MDSs [judgment].

## R7 Rust MDS requirements

- Path scope: **required** on every operation, LOOKUPINO included, because nfsd resolves forged or stale handles through it (R2). The design plans it [src: rmds:design/minimum-feature-set.md:340-345].
- Root squash, mode bits, chown and sticky rules: defence in depth; the host VFS enforces them [src: fs/ceph/inode.c:3101-3113].
- Transactional inode index: **required** for handle resolution after cache eviction or host restart (F2) [src: fs/ceph/export.c:149-150].
- `change_attr` on every mutation path: **required**, because `fs/ceph` passes it to guests as the change attribute (R6) [src: fs/ceph/inode.c:3181-3183].
- Session takeover: RESET reclaim exists and keys on uuid metadata, not on client class [src: rmds:crates/mds/src/server/runtime/d6.rs:237-300]. Level 1 needs a reclaim that reserves the old session's locks; level 2 needs cooperative handover of caps and locks [judgment]. Per-session reconnect while the rank is active would also help the 7.2 `fs/ceph` reset, which cannot reconnect today [src: git b52861897be8].
- Revoke patience: with PS2, cap revokes may wait about two NFS lease periods; Ganesha sets the libcephfs delegation timeout to twice the lease plus 5 s, and a missed timeout force-unmounts the client [src: rmds:mds-rust-map/cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md:455; ceph:src/client/Delegation.cc:12-25].
- Lean non-caching mode: not needed [judgment].
- Many small sessions: **required**, one per VM per host (P5).
- Admin eviction API: **required** for VM teardown and fencing [judgment].

## R8 Performance

- Strict I/O: SETATTR costs one MDS round trip and one WAL commit; a stable WRITE costs the OSD write plus one cap flush with a WAL commit [judgment]. UNSTABLE writes plus COMMIT batch under `Fb`, unlike Ganesha's synchronous writes [src: rmds:mds-rust-map/cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md:494-506].
- Metadata round trips: caps and dentry leases often make GETATTR and LOOKUP free; an uncached handle costs one LOOKUPINO [judgment].
- Throughput and CPU: no user/kernel crossing, splice reads, and two host-side copies per buffered write or one with `NFSD_IO_DIRECT` (R4) [judgment].
- Memory per VM: each thread keeps call and reply page arrays filled [src: net/sunrpc/svc_xprt.c:708-735; net/sunrpc/svc.c:552-576], about twice the global block size [src: fs/nfsd/nfsctl.c:874; fs/nfsd/nfsd.h:44]. One minimum thread (7.0) keeps an idle VM near 2 MiB at a 1 MiB block size [src: fs/nfsd/netns.h:155; judgment]. Helpers and `fs/ceph` caches come on top [judgment].

## R9 Completeness and time to first product

knfsd already re-exports `fs/ceph` (F9) from per-netns instances (F15). The first product is WP0 plus the required MDS items; see "Work packages" [judgment].

## R10 Upstream path

Deciders: the CEPH maintainers (Ilya Dryomov, Alex Markuze, Viacheslav Dubeyko) and the NFSD/SUNRPC, FILE LOCKING and EXPORTFS maintainers (Chuck Lever, Jeff Layton) [src: MAINTAINERS:6037-6047,14183-14210,10039-10048,10081-10091]. Sequence and costs: see "Upstream plan".

## R11 Operability

- Deployment: modules, nfs-utils and the agent. Server upgrades follow the host kernel cadence [judgment]. The Live Update Orchestrator (6.19) keeps VMs across kexec but not nfsd or `fs/ceph` state, so guests then see a server reboot; only memfd registers a preservation handler today [src: kernel/liveupdate/luo_core.c; mm/memfd_luo.c; grep of liveupdate users; git 9e2fd062fa17].
- Observability: per-netns netlink statistics (7.3), tracepoints and `fs/ceph` debugfs [src: Documentation/netlink/specs/nfsd.yaml:605; git 7e1f9e2cd2d0].
- Restart recovery: a per-VM instance restart costs a short grace (R5) [judgment]. A host crash kills its VMs (P4).
- Per-tenant control: threads, minimum threads, versions, lease and grace, exports and admin revocation are settable per netns [src: fs/nfsd/netns.h:143-155,180-181; Documentation/netlink/specs/nfsd.yaml:442,573-591]. The client limit and DRC size are per netns but derive from host memory [src: fs/nfsd/nfs4state.c:5566; fs/nfsd/nfscache.c:185]. The VM's cgroup does not pay for nfsd CPU or memory (PS8).

## Shared work packages

- With Option B: PS2-PS5 (`fs/ceph`), because B also uses `fs/ceph` as its CephFS client; the per-VM netns endpoint also fits B [judgment].
- With Options A and B: every R7 item, the level-1 MDS takeover protocol (uuid reclaim, fencing, lock reservation), and the test assets [src: rmds:design/nfs/requirements.md:72-77].

## Work packages

| WP | Content | Size | Depends on |
|---|---|---|---|
| WP0 | Agent, per-VM netns nfsd, per-VM mount, helpers (first product) | M | MDS path scope, inode index |
| WP1 = PS5 | Export flags and `commit_metadata` | S | none |
| WP2 = PS2 | `ceph_setlease` tied to caps | M | MDS revoke patience |
| WP3 = PS3 | uuid, timeout and RESET reclaim | M | MDS RESET reclaim (exists) |
| WP4 = PS4 | Asynchronous lock grant | M | none |
| WP5 | Level-1 migration orchestration | M | WP0, WP3, MDS lock reservation |
| WP6 = PS6 | nfsd state export/import, `fs/ceph` handover (L), MDS handover (L) | XL | WP2, WP3, WP5 |
| WP7 = PS7 | Zero-copy transport hooks | L | a change of P2 |
| WP8 = PS8 | Per-tenant cgroup accounting | M | none |
| WP-M | Rust MDS items of R7 | L | none |

Shortest path: WP0 plus WP-M gives v3 and v4.1 with polled blocking locks and level-1 migration by a clean source unmount [judgment]. It grants no delegations, which a server may always decline [judgment]. The first patch series is PS5, a small correctness fix; PS2 is the first feature series [judgment].

**PS1, VM identity (S, userspace only).** Design: one nfsd instance per VM netns, no kernel change [judgment]. `ip_map` and the export caches are already per net [src: net/sunrpc/svcauth_unix.c:947,969; fs/nfsd/netns.h:77-78]. Rejected: one instance that matches exports by ingress interface. It needs device-bound listeners and an `ip_map` keyed by interface and address, and it shares grace, DRC and client tables across VMs [src: net/sunrpc/svcauth_unix.c:101-107; judgment]. LOCALIO (6.12) is the closest precedent for host-local identity. A nonce handshake on the NFS connection proves co-location. It merged with Jeff Layton's review and Chuck Lever's ack [src: git fa4983862e50; Documentation/filesystems/nfs/localio.rst:7-30]. It needs the client in the same kernel, so it cannot serve guests [judgment]. Its authors dropped address matching because it "didn't handle use of iptables or containers" [src: Documentation/filesystems/nfs/localio.rst:23-27]. vsock: the 2015 client RFC and the 2017 nfs-utils series did not merge, and the reachable archives show no blocking objection [src: grep: no vsock in net/sunrpc, fs/nfs, fs/nfsd; web: https://lwn.net/Articles/647516/, accessed 2026-10-30; web: https://lore.kernel.org/all/20170919093140.GF9536@redhat.com/T/, accessed 2026-10-30, search listing only]. In 2016 libvirt's Daniel Berrangé preferred one isolated Ganesha per VM, for cgroup limits and SELinux labels [web: https://www.mail-archive.com/libvir-list@redhat.com/msg129827.html, accessed 2026-10-30]. The effort moved to virtio-fs [src: git a62a8ef9d97d]. vsock is netns-aware since 7.0, so a later per-VM vsock listener is plausible [src: git eafb64f40ca4, a69686327e42; judgment]. Deciders for sunrpc changes: Lever and Layton; client side: Trond Myklebust and Anna Schumaker; vsock: Stefano Garzarella [src: MAINTAINERS:14183-14189,19339-19341,29148-29149].

**PS2, `fs/ceph` leases tied to caps (M).** Today `ceph_file_fops` has no `->setlease` [src: fs/ceph/file.c:3294-3310; git 51e49111c00b], so `nfs4_set_delegation` fails at `kernel_setlease` [src: fs/nfsd/nfs4state.c:6810]. Design, after libcephfs: grant `F_RDLCK` only while issued caps cover Pin, As, Ls, Xs, Fs, Fr and Fc, and `F_WRLCK` only with Fx, Fw and Fb as well [src: ceph:src/client/Delegation.cc:41-61]. Take cap references that pin those caps [src: ceph:src/client/Delegation.cc:67-73], call `generic_setlease`, then re-check the caps, as `nfs4_add_lease` re-checks its delegation [src: fs/nfs/nfs4proc.c:7815-7832]. While the references exist, `ceph_check_caps` does not ack a revoke of those caps [src: fs/ceph/caps.c:2196-2200]. `F_UNLCK` drops lease and references, and the next `ceph_check_caps` acks [src: fs/ceph/caps.c:2047]. Recall: the revocation branch of `handle_cap_grant` queues work when a revoked bit backs a lease [src: fs/ceph/caps.c:3764-3787]. The work calls a non-blocking `__break_lease`, as the NFS client does on recall [src: include/linux/filelock.h:32-39; fs/nfs/delegation.c:1194-1195], and nfsd's `lm_break` sends CB_RECALL [src: fs/nfsd/nfs4state.c:6139-6164]. Session loss must break leases at once [judgment]. Directory leases stay refused [src: git ffb321045b0f]. Deciders: CEPH and FILE LOCKING maintainers on ceph-devel and linux-fsdevel [src: MAINTAINERS:6037-6047,10039-10048]. Precedent: Jeff Layton wrote in 2020 that leases "are not broken when caps are revoked" and "could eventually be implemented similarly to how we did them in libcephfs" [src: git 496ceaf12432]; `cifs_setlease` ties leases to oplocks [src: fs/smb/client/cifsfs.c:1143-1170]. Objections: cap-loss races, revoke delay against MDS eviction policy, write-delegation flush ordering, multi-host tests [judgment].

**PS3, `fs/ceph` uuid, timeout and reclaim (M).** New mount options `uuid=`, `session_timeout=` and `reclaim` [judgment]. With `reclaim`, the mount opens sessions without uuid and requires RECLAIM_CLIENT. It sends `MClientReclaim{uuid, RESET}` to every rank. It raises the OSD epoch barrier to the largest reply epoch. It then reopens the sessions with `uuid` and `timeout` metadata. This follows the libcephfs sequence [src: ceph:src/client/Client.cc:18632-18642,18861-18959] and reuses a libceph helper [src: net/ceph/osd_client.c:2628-2640]. The C++ MDS evicts and blocklists the target [src: ceph:src/mds/Server.cc:523-559]; both MDSs refuse a reclaim without RESET [src: ceph:src/mds/Server.cc:491-497; rmds:crates/mds/src/server/runtime/d6.rs:237-300]. The blocklisted source mount must not auto-recover, so `recover_session=clean` stays off [src: fs/ceph/super.c:216; judgment]. The 7.2 reset state machine supplies quiesce and drain [src: fs/ceph/mds_client.h:84-123]. Deciders: CEPH maintainers. Precedent: Ganesha (F7); Ceph's reclaim design names the motive, an NFS server "racing against its MDS session timeout" [web: https://docs.ceph.com/en/latest/dev/cephfs-reclaim/, accessed 2026-10-30]. Objections: one user; the MDS checks only that both sessions share an auth name, so any holder of the tenant key can evict a sibling VM [src: ceph:src/mds/Server.cc:500-507]; the mount must not wait without bound for all ranks [judgment].

**PS4, `fs/ceph` asynchronous lock grant (M).** Today `ceph_lock` blocks the caller for `F_SETLKW` [src: fs/ceph/locks.c:125-130,270-271]. Without `FOP_ASYNC_LOCK`, nfsd never sets `FL_SLEEP` [src: fs/nfsd/nfs4state.c:9144-9148], and lockd answers blocking requests with BLOCKED or DENIED [src: fs/lockd/svclock.c:468-471,545-548]. Guests poll: the NLM client every 30 s, the v4 client with 1-30 s backoff [src: fs/lockd/clntproc.c:27,572; fs/nfs/nfs4proc.c:7665-7666]. Design: set `FOP_ASYNC_LOCK` [src: include/linux/fs.h:1978]. For an `FL_SLEEP` caller with `lm_grant` (lockd [src: fs/lockd/svclock.c:799]), submit SETFILELOCK with `wait=1` asynchronously and return `FILE_LOCK_DEFERRED`. On the grant, record the local POSIX lock and call `lm_grant`; unlock if it refuses; cancel with `CEPH_LOCK_FCNTL_INTR` [src: fs/ceph/locks.c:166-167]. nfsd has `lm_notify` but no `lm_grant` [src: fs/nfsd/nfs4state.c:8739-8746], so the v4.1 path needs a notify-on-grant design [judgment]. Deciders: CEPH and FILE LOCKING maintainers. Precedent: DLM asynchronous POSIX locks [src: fs/dlm/plock.c:150-168] and `FOP_ASYNC_LOCK` itself [src: git 8cf9a01edc21]. Objections: cancellation races and waits pending across MDS reconnect [judgment].

**PS5, export flags and `commit_metadata` (S).** `ceph_export_ops` sets no flags and no `commit_metadata` [src: fs/ceph/export.c:620-626]. Set `EXPORT_OP_NOATOMIC_ATTR` and `EXPORT_OP_NOWCC`, because another host can change an inode between pre- and post-operation attributes [judgment]; nfsd then reports non-atomic change_info and omits WCC data [src: fs/nfsd/nfs4proc.c:435; fs/nfsd/nfsfh.c:338-344]. Leave `EXPORT_OP_NOLOCKS` (CephFS locks are coherent) and `EXPORT_OP_CLOSE_BEFORE_UNLINK` (no silly rename) unset [judgment]. Leave `EXPORT_OP_FLUSH_ON_CLOSE` unset, because `fs/ceph` has no `->flush` [src: fs/ceph/file.c:3294-3310]. `EXPORT_OP_REMOTE_FS` only changes loopback throttling [src: fs/nfsd/vfs.c:1460-1469]. Add a `commit_metadata` that flushes caps and the MDS log, for early-reply MDSs [src: fs/ceph/caps.c:2405]. From `reexport.rst`: the `fsid=` rule applies [src: fs/nfsd/export.c:1131-1137], the reboot-recovery limit applies to proxy restarts, handle size is no issue, and DENY bits act only inside one instance [src: Documentation/filesystems/nfs/reexport.rst:25-35,107-117]. Deciders: CEPH maintainers, with EXPORTFS review [src: MAINTAINERS:10081-10091]. Precedent: the NFS client's flags [src: fs/nfs/export.c:159-165].

**PS6, nfsd state export and import (XL).** nfsd-internal state is serializable as data. It covers client identity and flags, sessions with channel attributes and slot replay bytes, stateids with seqids, owners and share bitmaps. It also covers delegation records, DRC entries, timers, boot time, write verifier, scope and `fh_key` [src: fs/nfsd/state.h:355-600; fs/nfsd/netns.h:143-252]. VFS state that import must re-create: open files in the filecache, POSIX locks through `vfs_lock_file`, leases through `kernel_setlease` (PS2) [judgment]. Interface: admin-only netlink commands FREEZE, EXPORT, IMPORT and THAW per netns, because the maintainers want new settings in netlink first [src: profile:121-123]. Order: optional delegation recall while the VM runs; VM pause; freeze (stop admission, drain threads, drop transports); `fs/ceph` sync and session export; state export; MDS takeover prepare, in which the destination session receives the reserved locks and the source is fenced; commit at the MDS reply, which makes the MDS the commit point of a two-phase protocol, else thaw the source; import into the fresh destination netns; start without grace; resume the VM [judgment]. Deciders: Lever and Layton, with two Reviewed-by tags [src: profile:323-326]. Precedent: none in tree, where nfsd history holds only referrals [src: git log fs/nfsd]. Web searches found no server-side state migration work for nfsd [judgment]. The client side supports transparent state migration [src: git 8dcbec6d20eb]. A read-only per-client netlink dump is under review [web: https://ratatoskr.run/linux-nfs/2026/09/17502278/t, accessed 2026-10-30]. Virtuozzo 7 does not even migrate containers with an NFS client inside [web: https://ftp.fau.de/openvz/virtuozzo/releases/7.0-rtm/x86_64/iso/readme.txt, accessed 2026-10-30]. Objections: an internal-state format becomes ABI; import is a privileged attack surface; `nfs4state.c` alone has 10,555 lines; maintainers may prefer protocol-level migration [src: wc fs/nfsd/nfs4state.c; judgment]. Odds: low to moderate, better if the series also serves host live update [judgment].

**PS7, zero-copy hooks (L).** Existing pieces: splice reads, `MSG_SPLICE_PAGES` sends, `NFSD_IO_DIRECT` and transport classes (R4). A vhost-style or shared-memory transport needs a `svc_xprt_class` whose `xpo_recvfrom` maps guest WRITE pages into `rq_pages` and whose `xpo_result_payload` places READ pages in guest buffers [src: include/linux/sunrpc/svc_xprt.h:15-31; judgment]. Pages must stay stable until sent, as `MSG_SPLICE_PAGES` requires [src: net/sunrpc/svcsock.c:1245-1248]. Stock guests lack a matching client transport (P1, P2) [judgment]. Deciders: NFSD/SUNRPC, NFS client and VSOCK maintainers [src: MAINTAINERS:14183,19339,29148]. Objection: no stock-guest user [judgment].

**PS8, per-tenant resource control (M).** Per netns, nfsd separates threads, versions, lease and grace, exports, client limit, DRC, client tracking, statistics and admin revocation (R11). It gives no memory safety or privilege separation, because all instances share one kernel [judgment]. It gives no CPU or memory accounting, because workers are kthreads [src: net/sunrpc/svc.c:740]. Design: a netlink attribute that starts a netns's threads in a named cgroup, plus `__GFP_ACCOUNT` on per-client allocations [judgment]. The cgroup core refuses to move kthreads with `PF_NO_SETAFFINITY` [src: kernel/cgroup/cgroup.c:3144-3153]. Deciders: NFSD and cgroup maintainers. Objection: kthreads in tenant cgroups can stall shared work [judgment].

## Upstream plan

**Which sub-variant is upstreamable: C2.** Each series extends an existing interface, has in-tree precedent and has named deciders [judgment]. C1 duplicates a healthy subsystem ("C1").

Sequence of submissions [judgment]:

1. PS5 to ceph-devel, Cc linux-nfs and Jeff Layton. One cycle.
2. PS2 as an RFC to ceph-devel and linux-fsdevel, with multi-host recall tests on the dev cluster; read leases first, write leases second. Two to four cycles.
3. PS3 to ceph-devel. Two to three cycles.
4. PS4 to ceph-devel and linux-fsdevel. Two to three cycles.
5. PS6: first a design-document patch, which the profile prefers to cover letters [src: profile:289-291]; then staged series: read-only dump, freeze and thaw, v4.1 clients and sessions, opens and locks, delegations, v3 DRC and NLM. Four to eight cycles, uncertain.
6. PS8; PS7 only after P2 changes.

Mechanics. Releases come about every nine weeks [src: git release commits: 7.0 2026-04-12, 7.1 2026-06-14, 7.2 2026-08-16]. nfsd patches soak three to four weeks in nfsd-testing [src: profile:344-357]. `fs/ceph` patches reach Linus through Ilya Dryomov's pulls, and the 7.2 reset series carried Viacheslav Dubeyko's review [src: git 5422e496b313, b52861897be8]. The workspace Git identity (`git config user.name`) matches a listed CEPH maintainer, so PS2-PS5 need an independent co-maintainer review [src: MAINTAINERS:6037-6041; judgment]. The same maintainers merged new session machinery in 2026: subvolume metrics (7.1) and manual session reset (7.2) [src: git b1137e0b3d4b, 5422e496b313]. Jeff Layton maintained `fs/ceph` from 2019 to 2022 and wrote libcephfs delegations, so he is the natural cross-list reviewer [src: git 1edd1fecae5e, af7dc8e5124d; ceph git fad99776195]. Expected elapsed time: PS2-PS5 within 12 months; PS6 18 to 24 months, if accepted [judgment].

Cost of carrying. PS2-PS5 are small, in a tree with 7 to 35 commits per release, so rebases are cheap [src: git counts; judgment]. PS6 lives in a subsystem with 16 to 132 commits per release and with VFS churn such as the 2026 lease changes [src: git counts; git 2b10994be716]. Carrying it needs a custom host kernel and about one engineer [judgment].

## Risks

1. A C parser bug in nfsd, sunrpc or lockd compromises the host and every tenant on it (R3) [judgment].
2. PS6 rejection or delay leaves level 2 out of reach or out of tree [judgment].
3. PS2 race bugs serve stale data under a delegation; read leases first reduce the exposure [judgment].
4. The level-1 lock-steal window stays open until the MDS reserves locks on reclaim (R5) [judgment].
5. Ring-0 debugging and the kernel upgrade cadence slow operations [judgment].
6. Unaccounted kthreads let one tenant starve others until PS8 lands [judgment].
7. Maintainers are "the less scalable resource" [src: profile:316-319].
8. Four helper daemons per VM netns add operational load [judgment].
9. Upstream `fs/ceph` features need a C++ MDS implementation too, which P8 excludes (open question 6) [judgment].

## Open questions

1. F10 is stale. In 7.3, `kernel_setlease` returns `-EINVAL` without `->setlease` [src: fs/locks.c:2108-2115]; line 2045 is `generic_setlease` itself; `fs/ceph` refused leases since 2020 [src: git 496ceaf12432]. No coherence hole exists today; delegations are absent. The brief needs a correction.
2. P5 against P6: one session per VM forbids a shared superblock, so a tenant's guests share no page cache in C2 (R4) [src: fs/ceph/super.c:1249-1275,1340]. Should a read-mostly per-tenant mount be an opt-in exception?
3. Does "transparent" level 2 allow a guest-visible TCP reset (R5)?
4. Does level 2 cover NFSv3, whose DRC keys on the source port (R5) [src: fs/nfsd/nfscache.c:103-110]?
5. One key per tenant (P3) plus uuid reclaim lets any VM of a tenant evict a sibling's session (PS3) [src: ceph:src/mds/Server.cc:500-507]. Should each VM get its own cephx entity under the tenant's caps?
6. Lock reservation and cooperative handover are new MDS protocol. `fs/ceph` maintainers test against the C++ MDS, which P8 excludes [judgment]. Who implements the Ceph upstream side?
7. Should the agent implement MOUNT, NSM and client tracking itself, to remove four daemons per VM?
