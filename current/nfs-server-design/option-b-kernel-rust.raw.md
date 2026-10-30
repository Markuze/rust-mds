# Option B: in-kernel NFS server in Rust (Rust-for-Linux)

Option study for the per-host NFS server ("the proxy") against `design/nfs/requirements.md` (draft 1), 2026-10-30. Tag prefixes: `K/` is `/mnt/workspace/ceph-client` (Linux 7.3.0-rc6 with git history), `M/` is `/mnt/workspace/rust-mds`, `C/` is `/home/ubuntu/ceph`. `[src: K commit <id>]` cites a `K/` commit.

## Summary

1. Option B builds `rnfsd`, a GPL Rust kernel module that serves NFSv3 and NFSv4.1 to one host's guests and reaches CephFS only through the VFS and `fs/ceph`. [judgment]
2. Verdict: B is not viable as an upstream product on a two-to-three-year horizon. [judgment]
3. Linux 7.3-rc6 has no Rust file under `fs/` or `net/` and no Rust abstraction for sunrpc, sockets, VFS operations, locks, leases or exportfs. [src: K/fs and K/net contain no *.rs file; K/rust/kernel/fs.rs:7-11; K/rust/bindings/bindings_helper.h:29-100]
4. Kernel policy merges an abstraction only together with an in-tree user, and a Rust duplicate of existing C code needs the owning maintainers' consent. [web: https://rust-for-linux.com/out-of-tree-modules, accessed 2026-10-30; web: https://rust-for-linux.com/rust-reference-drivers, accessed 2026-10-30]
5. The NFSD maintainers plan Rust inside nfsd through xdrgen-generated code [src: K commit 5bc37b759ec0]; nothing indicates a plan for a second server. [judgment]
6. Sub-variant B2 (Rust components inside nfsd) is viable and matches that plan, but it is a memory-safety track inside Option C2, not a product. [judgment]
7. Out of tree, B means a forked kernel, because the abstractions must live in `rust/kernel/` and distribution kernels may ship without `CONFIG_RUST`. [src: K/Documentation/rust/general-information.rst:80-81; /boot/config-7.0.0-1012-ibm]
8. Rust removes most memory-safety bug classes in safe code, but a Rust panic calls `BUG()` [src: K/rust/kernel/lib.rs:178-183], and any server fault still hits every guest on the host. [judgment]
9. The kernel is the natural place for zero-copy reads, but one `fs/ceph` superblock per VM (one MDS session per VM, P5) removes page-cache sharing between a tenant's VMs. [src: K/fs/ceph/super.c:1249-1270, 1340-1341]
10. The watch list in "Upstream plan" names the events that would make B viable. [judgment]

## Architecture

### Components

- `rnfsd.ko` (Rust, GPL-2.0): RPC program handlers for NFSv3, MOUNT, NFSv4.1 (COMPOUND, sessions, state, grace, backchannel) and an RPCBIND responder. One `VmCtx` object per VM owns the export set, the credential policy, the slot tables and the capsule. A generic netlink family carries control commands. [judgment]
- Rust abstractions in `rust/kernel/` for every C API that `rnfsd` calls. An abstraction is a safe Rust API that wraps one C kernel API. Kernel policy does not let a leaf module call C bindings directly. [src: K/Documentation/rust/general-information.rst:80-81, 103]
- Reused C code: the sunrpc server transport and thread pool (`svc_recv`, `svc_process`), the VFS, the page cache, `fs/locks.c`, exportfs, `fs/ceph` and libceph. [src: K/net/sunrpc/svc_xprt.c:919; K/net/sunrpc/svc.c:1645]
- `rnfsd-ctl` (userspace daemon): mounts one `fs/ceph` superblock per VM with the tenant's cephx key and `noshare`, so each VM gets its own libceph client and MDS session. It creates one listener socket per VM and passes it into the kernel, as nfsd does with `svc_addsock`. [src: K/fs/ceph/super.c:1340-1341; K/fs/ceph/super.c:829; K/net/sunrpc/svcsock.c:1519; K/fs/nfsd/nfsctl.c:749]

Rejected alternative: a Rust TCP transport. It needs socket abstractions. Andrew Lunn answered the 2023 Rust socket RFC that netdev has "no interest in code to support out of tree modules". [web: https://lkml.rescloud.iu.edu/2308.2/02204.html, accessed 2026-10-30]

### Data flow and control flow

- Request path: guest, virtio-net, vhost-net, host-only interface, per-VM listener, sunrpc thread. C code parses the record marker, the RPC header and AUTH_SYS. Then `rnfsd` runs `pc_decode` (Rust XDR), `pc_func` and `pc_encode`, the `svc_procedure` contract of nfsd. [src: K/include/linux/sunrpc/svc.h:428-445; K/fs/nfsd/nfssvc.c:993-1067]
- `pc_func` finds the `VmCtx` from the connection's bound interface, overrides the credentials, and calls the VFS on that VM's `fs/ceph` superblock. [judgment]
- Callback path: an MDS cap revoke in `fs/ceph` breaks a VFS lease (shared work S1), the lease manager callback `lm_break` runs in `rnfsd`, and `rnfsd` sends CB_RECALL on the NFSv4.1 backchannel, as nfsd does with `nfsd_break_deleg_cb`. [src: K/fs/locks.c:1734; K/fs/nfsd/nfs4state.c:6229] [judgment]
- Control path: `rnfsd-ctl` sends genetlink commands: add VM, add listener, add export, quiesce, capture and import a capsule. [judgment]

### Process and privilege model

All of `rnfsd`, the abstractions, sunrpc, the VFS and `fs/ceph` run in ring 0, in the hypervisor kernel. `rnfsd` builds the credentials of each request from AUTH_SYS and applies the VM's squash policy. Workers are sunrpc kernel threads in the root cgroup, or per-VM user workers like `vhost_task`, which belong to the VM's process and cgroup. [src: K/kernel/vhost_task.c:123-154] [judgment]

```
 guest VM (stock image)           host kernel: one privilege domain
 +----------------+  host-only   +-------------------------------------------------+
 | NFSv3 / 4.1    |  virtio-net  | per-VM listener 169.254.x.y:2049 (bound to dev) |
 | sec=sys        |------------->| sunrpc svc (C): TCP, RPC header, AUTH_SYS       |
 +----------------+  vhost-net   |        | pc_decode / pc_func / pc_encode        |
                                 | rnfsd.ko (Rust): XDR, NFSv3+MOUNT, NFSv4.1,     |
                                 |   VmCtx per VM {exports, creds, slots, capsule} |
                                 |   rust/kernel abstractions (sunrpc, VFS, locks, |
                                 |   leases, exportfs, cred, netlink)              |
                                 |        | VFS calls under overridden creds       |
                                 | VFS + page cache + fs/locks.c + exportfs (C)    |
                                 | fs/ceph superblock per VM, noshare (C)          |
                                 +--------|----------------------|-----------------+
                                          | MDS session          | OSD sessions
                                          v (tenant cephx key)   v
                                      Rust MDS               RADOS OSDs
 rnfsd-ctl (userspace) --genetlink--> rnfsd ; --mount(2)--> fs/ceph per VM
```

### Abstractions: what exists, what must be written, who approves

| C API area | In 7.3-rc6 | Work | Approvers |
|---|---|---|---|
| sunrpc server: `svc_program`, `svc_rqst`, `xdr_stream`, listener add, backchannel `rpc_clnt` | none | L | NFSD, SUNRPC: Lever, Layton [src: K/MAINTAINERS:14183-14206]; NFS client maintainers Myklebust, Schumaker co-own `net/sunrpc/` [src: K/MAINTAINERS:19339-19354] |
| VFS consumer calls: lookup, create, link, rename, setattr, getattr, open, iter read/write, splice, fsync, readdir, xattr, ACL | `File` (fget, cred, flags) and `Kiocb` only [src: K/rust/kernel/fs/file.rs:186-347] | XL | VFS: Viro, Brauner; reviewer Kara [src: K/MAINTAINERS:10061-10076] |
| POSIX locks with `lm_grant`; leases with `lm_break` | none | M + M | FILE LOCKING: Layton, Lever [src: K/MAINTAINERS:10039-10048] |
| exportfs encode and decode | none | S | EXPORTFS: Lever, Layton; reviewer Goldstein [src: K/MAINTAINERS:10081-10091] |
| credentials: build, override, groups | read-only `euid`, `secid` [src: K/rust/kernel/cred.rs:28-70] | M | CREDENTIALS: Moore [src: K/MAINTAINERS:6943-6951] |
| genetlink commands | multicast only [src: K/rust/kernel/net/netlink.rs:5-9] | S | NETWORKING [GENERAL]: Miller, Dumazet, Kicinski, Abeni [src: K/MAINTAINERS:19018-19072] |
| VM-owned workers | none | S, optional | VHOST: Tsirkin, Wang, Pérez [src: K/MAINTAINERS:28971-28985] |
| workqueue, locks, `Arc`, xarray, rbtree, tracepoints, debugfs | exist [src: K/rust/kernel/workqueue.rs:248; K/rust/kernel/tracepoint.rs] | none | RUST: Ojeda and reviewers [src: K/MAINTAINERS:23958-23987] |

"none" means that no file under `K/rust/kernel/` wraps the area and `K/rust/bindings/bindings_helper.h:29-100` does not include its header directly. [src: K/rust/kernel; K/rust/bindings/bindings_helper.h:29-100] Work sizes are [judgment].

### Sub-variant B2: Rust components inside nfsd

- B2a, generated Rust XDR. The seam is the `pc_decode` and `pc_encode` function pointers; the NFSv4 entry point is `nfs4svc_decode_compoundargs`. [src: K/include/linux/sunrpc/svc.h:432-437; K/fs/nfsd/nfs4xdr.c:6759-6787] The xdrgen README lists "Generate kernel Rust code as well as C code" as future work. [src: K/tools/net/sunrpc/xdrgen/README:259] The xdrgen commit expects automatic conversion to ease a move to Rust. [src: K commit 4b132aacb076] The NLMv4 commit says "We would eventually like xdrgen to generate Rust code as well, making the conversion of the kernel's NFS stacks to use Rust just a little easier for us", with Jeff Layton's Reviewed-by. [src: K commit 5bc37b759ec0] The nfsd-7.2 pull names "groundwork for generation of Rust code for this layer instead of C". [src: K commit 3dc7c001169d] Precedent: the DRM panic QR generator is Rust with a C ABI that C calls, behind `depends on RUST`. [src: K/drivers/gpu/drm/drm_panic_qr.rs:941; K/drivers/gpu/drm/drm_panic.c:706; K/drivers/gpu/drm/Kconfig:101-103] Needed: a Rust `xdr_stream` abstraction (S), an xdrgen Rust backend (M), and nfsd that still builds without Rust. xdrgen emits the C twin and the Rust twin from one `.x` specification, so the duplicate costs little. [judgment] Acceptance odds: moderate, on the maintainers' schedule. [judgment]
- B2b, a Rust NFSv4 state module. `nfs4state.c` has 10,555 lines and couples to leases, locks, the file cache and callbacks. [src: K/fs/nfsd/nfs4state.c] The nfsd-7.3 pull lists "more use-after-free fixes in the NFSv4 state revocation paths". [src: K commit d141ec2825b4] Rust would help most here, but every lease, lock and callback crosses into C. Acceptance odds within three years: low. [judgment]
- B2 does not deliver VM identity, per-VM exports, capsules or per-VM grace. Those need nfsd C patches, which are Option C2. [judgment]

## R1 Guest-visible contract

- Endpoint: `rnfsd-ctl` puts the well-known link-local address on each VM's host-only interface and binds one listener per interface with `SO_BINDTODEVICE`. [judgment] TCP allows the same address and port on different bound devices. Accepted sockets inherit the bound interface. [src: K/net/ipv4/inet_connection_sock.c:165-168, 1227]
- A single unbound listener does not record the ingress interface on the accepted socket. [src: K/include/net/inet_sock.h:115-127] A main tap is usually a bridge or OVS port, so the local stack sees the bridge, not the tap; per-tap identity then needs tc or eBPF redirection. The host-only NIC (P1) avoids that. [judgment]
- NFSv3 needs RPCBIND on port 111 per VM, because stock clients look up the MOUNT and NLM ports. [src: K/fs/nfs/super.c:873; K/fs/lockd/host.c:490] `rnfsd` answers RPCBIND for its own programs. [judgment]
- Export naming: MOUNT accepts `vol-<id>`; NFSv4.1 shows a pseudo-root with one entry per volume of that VM. [judgment]
- `sec=sys`: sunrpc's AUTH_SYS parser fills `rq_cred`. [src: K/include/linux/sunrpc/svc.h:209]
- NFSv4.0 and 4.2 receive `NFS4ERR_MINOR_VERS_MISMATCH`. [judgment]
- NLM: in-kernel lockd binds one global NFS server through `nlmsvc_ops` and keys hosts by address and network namespace. [src: K/fs/lockd/svc.c:50; K/include/linux/lockd/bind.h:28-35; K/fs/lockd/host.c:389-391] Reuse of lockd therefore needs unique host-assigned client addresses on the host-only network or a network namespace per VM. A Rust NLM and NSM avoids both limits and costs an M work package. [judgment]

## R2 Identity and tenant isolation

- VM identity enters at the listener's bound interface. `rnfsd-ctl` maps the interface to the VM, the tenant and the VM's superblock. [judgment]
- Permission checks run in the host kernel. `rnfsd` overrides the credentials. The VFS and `ceph_permission` check mode bits and ACLs after an `As` cap refresh. [src: K/fs/ceph/inode.c:3101-3113] `fs/ceph` also sends the caller uid and gids to the MDS, so MDS session policy can double-check. [src: K/fs/ceph/mds_client.c:3134-3137, 3204-3205]
- Tenant keys: `rnfsd-ctl` mounts with the tenant key. The key then sits in kernel memory with the keys of every other tenant on the host. [judgment]
- Weakness against P3: Ceph confines each session to its path, but the kernel code that binds a guest connection to a session is `rnfsd` logic. A routing bug can send VM A's request through tenant B's legitimate session, and Ceph cannot see it. Per-VM type ownership in Rust (a `VmCtx` that owns only its superblock reference) makes such a bug less likely, not impossible. [judgment]
- Handles: a guest can forge an inode number. `rnfsd` decodes only on the VM's own mount and adds a per-VM MAC, like nfsd's `sign_fh` export option. [src: K/fs/nfsd/nfsfh.c:297-303] The MDS must still refuse LOOKUPINO results outside the session's path (R7). [judgment]
- A compromise of any ring-0 component reaches all guest memory and all tenant keys on the host. Option C has the same reach. [judgment]

## R3 Crash containment and attack surface

- What a bug takes down: the host kernel, and with it every guest's NFS service. A Rust panic prints the message and calls `BUG()`. [src: K/rust/kernel/lib.rs:178-183] Overflow checks are on by default, so an unchecked arithmetic error on guest input becomes a panic. [src: K/lib/Kconfig.debug:3594-3603]
- What Rust changes: code that uses sound abstractions cannot cause undefined behaviour outside `unsafe` blocks, so use-after-free, double-free and data-race bugs cannot occur there, and an out-of-bounds index panics instead of corrupting memory. [src: K/Documentation/rust/general-information.rst:105-112] nfsd, sunrpc and lockd received 25 commits in one year whose subjects name use-after-free, out-of-bounds, overflow, double-free, underflow or NULL-dereference fixes; 14 name use-after-free. [src: K git log --no-merges --since=2025-10-01 -- fs/nfsd net/sunrpc fs/lockd]
- What Rust does not change: logic bugs (stateid checks, squash rules, the R2 routing bug), deadlocks, resource exhaustion, panics, and bugs in `unsafe` abstraction code. The first Rust kernel CVE (CVE-2025-68260) was a data race in an `unsafe` list removal in Rust binder that corrupted list pointers and crashed the kernel. [src: K commit 3e0ae02ba831] [web: https://www.phoronix.com/news/First-Linux-Rust-CVE, accessed 2026-10-30]
- C code stays on the path: the sunrpc server core (11,496 lines), `fs/ceph` (34,494 lines) and libceph (25,476 lines) of C. [src: K/net/sunrpc/{svc,svc_xprt,svcsock,svcauth,svcauth_unix,xdr,cache}.c; K/fs/ceph; K/net/ceph]
- Untrusted parsing: the record marker, the RPC header and AUTH_SYS stay in C. The NFS, MOUNT and RPCBIND XDR moves to Rust. That replaces the role of nfsd's 10,002 lines of XDR C. [src: K/fs/nfsd/{nfs4xdr,nfs3xdr,nfsxdr,nfs4xdr_gen}.c] An estimate for `rnfsd`: 30,000 to 45,000 Rust lines plus 10,000 to 20,000 abstraction lines. [judgment]

## R4 Zero-copy readiness

Read path, OSD to guest, with the `Fc` cap held:
1. libceph copies the OSD reply from socket buffers into page-cache folios (copy 1). [src: K/net/ceph/messenger_v2.c:79-115]
2. `fs/ceph` splice read hands page references to the reply pages, no copy. Without `Fc` it falls back to `copy_splice_read` (extra copy). [src: K/fs/ceph/file.c:2323-2371; K/fs/nfsd/vfs.c:1010-1036]
3. sunrpc sends with `MSG_SPLICE_PAGES`, no copy. [src: K/net/sunrpc/svcsock.c:1245-1253]
4. tun copies the frame into the guest receive buffer (copy 2). [src: K/drivers/net/tun.c:2192-2201]

Write path, guest to OSD:
1. vhost-net copies guest transmit buffers into host packet buffers (copy 1), unless the `experimental_zcopytx` parameter, default 0, is set. [src: K/drivers/vhost/net.c:38-40]
2. sunrpc receives into request pages with `sock_recvmsg` (copy 2). [src: K/net/sunrpc/svcsock.c:339-360]
3. A buffered write copies into the page cache (copy 3). A direct mode avoids it, as nfsd's `NFSD_IO_DIRECT` does. [src: K/Documentation/filesystems/nfs/nfsd-io-modes.rst:21-27]
4. libceph sends pages with `MSG_SPLICE_PAGES`. CRC mode reads the data. Secure mode encrypts into new pages (a copy). [src: K/net/ceph/messenger_v2.c:159-170, 1633-1673]

Why the kernel is the natural place: page-cache pages go to the socket without a copy, and a future vhost-style transport could map guest pages for the server in the same address space. That transport does not exist today. [judgment] Page-cache sharing inside a tenant works only when the tenant's VMs share one superblock. `noshare` per-VM mounts give separate page caches. [src: K/fs/ceph/super.c:1249-1270, 1340-1341]

## R5 Migration readiness

- Server state in `rnfsd` (Rust-owned): NFSv4.1 client records, sessions with slot tables and cached replies, stateids, open and lock owners, the NFSv3 duplicate reply cache (DRC), and per-VM grace state. [judgment]
- VFS state (C objects on the source host): open files, POSIX locks and leases. `fs/ceph` derives the MDS lock owner from the owner pointer XOR a per-boot secret, so a destination host cannot present the same owner. [src: K/fs/ceph/locks.c:17-27, 111]
- `fs/ceph` session state: caps, cap sequence numbers, dirty data and flush tids. `fs/ceph` has no session export or import. [judgment]
- Quiesce and capture: pause the VM's listener, drain requests, recall delegations, `sync_filesystem` on the VM's superblock, then encode the Rust state as an XDR capsule. Two-phase commit: the destination validates and prepares, the source commits and closes its session. [judgment]
- Level 1: the destination starts grace for that VM only. nfsd scopes grace per network namespace, so a fresh server can do per-VM grace more easily. [src: K/fs/nfs_common/grace.c:91-97] [judgment] The re-export document notes that the source server is not in grace when a re-export server restarts. [src: K/Documentation/filesystems/nfs/reexport.rst:28-35] Another client can take a lock before the guest reclaims it. The MDS must hold the old session's locks for the uuid until reclaim ends (R7). [judgment]
- Level 2: the capsule carries sessions and slots. The guest reconnects with the same session ID and replays in-flight requests against the cached replies. The destination re-creates opens and locks. That needs MDS session takeover or lock-owner adoption, plus explicit stable owner IDs in `fs/ceph` (shared work S5). [judgment]
- Neither nfsd nor `fs/ceph` exports state today, so B designs capture and import from scratch. [judgment] Rust helps the server part: one owned type tree per VM can be walked and encoded completely. nfsd instead shares `nfs4_file` between all clients of a network namespace. [src: K/fs/nfsd/state.h:744-776] Rust does not help with VFS or `fs/ceph` state. [judgment]

## R6 CephFS client integration

- Client: `fs/ceph` through the VFS, no new Rust client. [judgment]
- Delegations: today `kernel_setlease()` returns `-EINVAL` without a `->setlease` operation, and `fs/ceph` defines none, so nfsd grants no delegation on CephFS. [src: K/fs/locks.c:2108-2115; K commit 2b10994be716; K/fs/nfsd/nfs4state.c:6810-6815] Shared work S1 adds `setlease` on caps, modelled on libcephfs `Delegation`: read needs `PIN|Fs|Fr|Fc|Xs|Ls|As`, write adds `Fx|Fw|Fb`. [src: C/src/client/Delegation.cc:41-61] `rnfsd` consumes it with its own lease manager operations. [judgment]
- Locks: `fs/ceph` sends POSIX locks to the MDS with an owner. [src: K/fs/ceph/locks.c:111-125] It has no `FOP_ASYNC_LOCK`, so nfsd and lockd turn blocking locks into polling. [src: K/fs/nfsd/nfs4state.c:9144-9148; K/fs/lockd/svclock.c:468-471] Shared work S3 adds an asynchronous grant. [judgment]
- Handles: `exportfs` calls `ceph_encode_fh` and `ceph_fh_to_dentry`, which sends LOOKUPINO. Snapshot handles exist. [src: K/fs/ceph/export.c:22-35, 249, 621-625]
- uuid reclaim: absent in `fs/ceph`; shared work S2. [src: M/design/nfs/requirements.md:43 (F11)]
- Change attribute: `fs/ceph` reports `STATX_CHANGE_COOKIE` from the MDS `change_attr`. [src: K/fs/ceph/inode.c:1110, 3181-3183]
- Strict I/O: COMMIT and stable WRITE call `vfs_fsync_range`, as in nfsd [src: K/fs/nfsd/vfs.c:1703], and `ceph_fsync` writes data, flushes caps, flushes the MDS log, waits for unsafe requests and waits for the flush acknowledgement. [src: K/fs/ceph/caps.c:2522-2554] The Rust MDS sends safe replies only, so a returned SETATTR is durable. [src: M/design/nfs/requirements.md:37 (F8)]

## R7 Rust MDS requirements

Day one:
- Path-scope enforcement for cephx `path=` caps, including LOOKUPINO, LOOKUPPARENT and LOOKUPNAME results outside the scope. Today the MDS applies no restriction: "per-path cap strings are both reported as unrestricted". [src: M/crates/mds/src/server.rs:925-938] P3 depends on it. [judgment]
- `change_attr` on every mutation path, because NFSv4 cache validation reads it. [src: K/fs/ceph/inode.c:3181-3183; M/design/nfs/requirements.md:33 (F4)]
- Many small sessions: one libceph client per VM, each with its own monitor, OSD and MDS sessions. [src: K/fs/ceph/super.c:829; K/net/ceph/ceph_common.c:741-744]
- uuid, `timeout` and RESET reclaim, already planned. [src: M/design/workplan.md:54]
- An admin eviction API for VM deletion and fencing. Per-VM clients have their own messenger nonce [src: K/net/ceph/messenger.c:1686-1690], so a blocklist entry fences one VM only. [judgment]

Later: session takeover with blocklist and OSD epoch barrier for level 2 and for a lock-safe level 1; mode-bit, root-squash and sticky-directory policy as defence in depth; the transactional inode index for multi-rank LOOKUPINO [src: M/design/nfs/requirements.md:31 (F2)]. B does not need a lean non-caching session mode, because `fs/ceph` caching is the point of B. [judgment]

## R8 Performance

- Strict-I/O latency: equal to a knfsd re-export of `fs/ceph`. A stable WRITE costs the OSD write plus one cap flush (one MDS round trip and one WAL commit). No user and kernel crossing is added. [src: K/fs/ceph/caps.c:2522-2554] [judgment]
- Metadata round trips: dentry cache and caps serve repeated LOOKUP and GETATTR. A cold directory handle costs LOOKUPINO plus one LOOKUPPARENT and one LOOKUPNAME per uncached ancestor, because exportfs reconnects directory dentries to the root. [src: K/fs/exportfs/expfs.c:120-139, 209-221, 474-486] After a host reboot this hits every mounted guest. [judgment]
- Throughput and CPU per byte: B has the lowest expected CPU cost per byte of the three options, with splice reads and no system calls. [judgment]
- Memory per VM: each sunrpc thread pins two page arrays of `max_payload/4 KiB + 3` pages, about 8 MiB at the 4 MiB default on hosts with at least 16 GB. [src: K/include/linux/sunrpc/svc.h:181-184; K/net/sunrpc/svc.c:552-574; K/fs/nfsd/nfsd.h:44; K/fs/nfsd/nfssvc.c:560-574] Threads are shared between VMs. Each VM also pays for one libceph client and its caches; the size needs a measurement. [judgment]

## R9 Completeness and time to first product

- Exists: `fs/ceph` with caps, export operations and MDS locks; the sunrpc server; nfsd as a GPL reference that a GPL kernel module may port. The P8 wall still applies: nothing from `rnfsd` may flow into the MDS tree. [judgment]
- B shares no code with the Rust MDS crates, because the kernel cannot use the `std` and async crates. [judgment]
- Must be built: the abstraction layer (W1-W5), the server (W6-W8), the control plane, the capsule and the tests, plus the shared `fs/ceph` and MDS work (see "Work packages"). [judgment]
- Time to first product: three to four years for a team of four to six kernel Rust engineers. [judgment] Comparison: ksmbd merged in August 2021 because Linux "lacked a kernel server" for SMB, and dropped its experimental warning in September 2023. [src: K commit e24c567b7ecf; K commit f5069159f32c] Rust binder took about 23 months from RFC (2023-11-01) to the 6.18 merge, with maintainer support. [web: https://lkml.iu.edu/hypermail/linux/kernel/2311.0/00612.html, accessed 2026-10-30; src: K commit eafedbc7c050]

## R10 Upstream path

- Target trees: the NFSD tree (`cel/linux`) for the server; the VFS, file-locking, credentials and netdev trees for the abstractions. [src: K/MAINTAINERS:14183-14206]
- Deciders: the NFSD and SUNRPC maintainers for the server and sunrpc abstractions, the VFS maintainers for `rust/kernel/fs`, netdev for `rust/kernel/net`. [src: K/MAINTAINERS:10061-10076, 19018-19072] Each subsystem decides how it deals with Rust. [web: https://rust-for-linux.com/rust-kernel-policy, accessed 2026-10-30]
- Precedent for: Rust is no longer experimental. [src: K commit 9fa7153c31a3; web: https://lwn.net/Articles/1050174/, accessed 2026-10-30] Rust binder coexists with C binder as a mutually exclusive option. [src: K/drivers/android/Kconfig:17-21] Brauner merged the Rust `File` abstraction with "Let's see how it holds up with actual users". [src: K commit 5bb6ba448fe3] Torvalds wrote that a maintainer who refuses Rust gets "no *say* on the Rust code", and that C changes leave the fallout to the Rust side. [web: https://lkml.iu.edu/hypermail/linux/kernel/2502.2/08504.html, accessed 2026-10-30]
- Precedent against: Andrew Lunn: "you cannot just rewrite this existing code in Rust because it is core code"; "What you can however do is implement something new, which the kernel does not have". [web: https://lkml.rescloud.iu.edu/2308.2/02204.html, accessed 2026-10-30] At LSFMM+BPF 2024, Ts'o called Rust bindings "a second-class citizen" and said he would not fix them. [web: https://lwn.net/SubscriberLink/978738/f03a02cf8abcb751/, accessed 2026-10-30] Wedson Almeida Filho, who presented the VFS abstractions there, left the project in August 2024. [web: https://lwn.net/SubscriberLink/978738/f03a02cf8abcb751/, accessed 2026-10-30; web: https://lwn.net/Articles/987635, accessed 2026-10-30] No Rust filesystem is in tree. [src: K/fs (no *.rs)]
- Expected objections: a duplicate NFS server; improve nfsd instead; features unavailable on kernels without `CONFIG_RUST`; reviewer bandwidth, since NFSD maintainers call themselves "the less scalable resource" and want two Reviewed-by tags. [src: K/Documentation/filesystems/nfs/nfsd-maintainer-entry-profile.rst:315-324]
- Maintenance cost if upstream rejects: see "Upstream plan".

## R11 Operability

- Deployment: a kernel module plus `rnfsd-ctl` on a kernel with `CONFIG_RUST=y`. The Ubuntu 26.04 IBM-cloud kernel on the dev machine ships `# CONFIG_RUST is not set`. [src: /boot/config-7.0.0-1012-ibm] Debian enabled Rust for its "forky" kernels. [web: https://lwn.net/Articles/1050174/, accessed 2026-10-30]
- Upgrade cadence: out of tree, every kernel release; upstream, with the kernel. A module reload loses all in-kernel state, so every VM sees a server reboot, unless the level-2 capsule path is reused for module handover. A kernel upgrade is a host reboot after live migration (P4). [judgment]
- Observability: tracepoint and debugfs abstractions exist; netlink statistics need W9. [src: K/rust/kernel/tracepoint.rs; K/rust/kernel/debugfs.rs]
- Restart recovery: a fault is a kernel oops, not a process restart. Recovery means a host reboot, session reclaim (S2) and grace for every guest. [judgment]
- Per-tenant resource control: page-cache pages added by kernel threads go to the root memory cgroup unless `set_active_memcg()` selects one. [src: K/mm/memcontrol.c:1172-1180] `rnfsd` sets the VM's memory cgroup per request and uses per-VM slot limits. Per-VM user workers like `vhost_task` charge CPU to the VM. [src: K/kernel/vhost_task.c:123-154] [judgment]

## Shared work packages

Shared with Option C (kernel). Option A uses its own Rust client and does not need S1-S5. [src: M/design/nfs/requirements.md:52]
- S1 `fs/ceph` `setlease` tied to caps: grant only with the needed caps issued, pin them with cap references, break the lease on revoke, acknowledge the revoke after lease return, bound the wait (M). [src: C/src/client/Delegation.cc:41-131]
- S2 `fs/ceph` uuid and `timeout` session metadata and `MClientReclaim` RESET (M). [src: M/design/nfs/requirements.md:43 (F11)]
- S3 `fs/ceph` asynchronous blocking-lock grant with `FOP_ASYNC_LOCK` and `lm_grant` (M). [src: K/fs/locks.c:2495-2511]
- S4 `fs/ceph` export flags (`EXPORT_OP_REMOTE_FS`, `EXPORT_OP_NOATOMIC_ATTR`, `EXPORT_OP_FLUSH_ON_CLOSE`) and `commit_metadata` (S). [src: K/include/linux/exportfs.h:284-296]
- S5 `fs/ceph` explicit stable lock-owner IDs for level 2 (M). [src: K/fs/ceph/locks.c:17-27]
- S6 sunrpc: expose a connection's bound interface (S). [judgment]

`rnfsd` consumes S1 through `kernel_setlease()` with its own lease manager operations, S3 through `vfs_lock_file()` with `FL_SLEEP` and its own `lm_grant`, S2 through mount options that `rnfsd-ctl` sets, and S4 through exportfs and `commit_metadata`. [src: K/fs/locks.c:2108-2115, 2513-2520] [judgment]

Shared with all three options [judgment]:
- S7 Rust MDS adjustments in R7.
- S8 Option-neutral capsule format in XDR, guest contract, control-plane API.
- S9 Test assets: pynfs, Ganesha as oracle, knfsd re-export as baseline.

The git user of the checkout is an `fs/ceph` maintainer, so S1-S5 have a realistic upstream path. [src: K/MAINTAINERS:6037-6040; M/current/nfs-server-design.md:4]

## Work packages

| Id | Work package | Size | Depends on |
|---|---|---|---|
| W1 | sunrpc server abstractions and backchannel client | L | S6 |
| W2 | VFS consumer abstractions | XL | none |
| W3 | Lock and lease abstractions | M | S1, S3 |
| W4 | exportfs and credential abstractions; per-VM handle MAC | M | W2 |
| W5 | xdrgen Rust backend and generated types (equal to B2a) | M | none |
| W6 | NFSv3, MOUNT, RPCBIND in `rnfsd` | L | W1, W2, W4, W5 |
| W7 | NLM and NSM in Rust, or lockd reuse with unique addresses | M | W3, W6 |
| W8 | NFSv4.1: COMPOUND, sessions, state, per-VM grace, delegations | XL | W1-W5, S1-S3 |
| W9 | `rnfsd-ctl`, genetlink family, per-VM mounts and listeners | M | W1, S2 |
| W10 | Capsule level 1 | L | W8, W9, S2, S7 |
| W11 | Capsule level 2 with MDS session takeover | XL | W10, S5, S7 |
| W12 | Resource control and observability | M | W9 |
| W13 | Tests: pynfs, Connectathon, fstests over NFS, KUnit and syzkaller XDR fuzzing | L | W6, W8 |
| W14 | Out-of-tree carry: patched kernel, rebase per release, CI | L, recurring | all |

Sizes are relative. First product (level 1, no level 2): W1-W10, W12, W13, S1-S4, S7. [judgment]

## Upstream plan

| Step | Submission | Odds | Elapsed time | Expected objections |
|---|---|---|---|---|
| 1 | S1-S5 to ceph-devel | 70-85% | 12-18 months | MDS behaviour on revoke timeouts; test coverage |
| 2 | W5 plus a Rust `xdr_stream`, first used by lockd's generated NLM code | 40-60% | 12-24 months | nfsd must build without Rust; generated-code performance |
| 3 | sunrpc svc abstractions with an in-tree Rust RPC service | 20-35% | 24-36 months | no wanted user; two client and server maintainer groups |
| 4 | Lock and lease abstractions with that user | 15-25% | 24-36 months | callback lifetimes across C and Rust |
| 5 | VFS consumer abstractions | 10-20% | 24-48 months | the LSFMM 2024 objections: naming, lifetimes, who fixes bindings |
| 6 | `rnfsd` as a reference server or a second server | under 5% in 3 years | 36-60 months | duplicate of nfsd; reviewer bandwidth; Rust-only features |

All odds and times are [judgment]. Steps 1 and 2 serve Option C2 as well. Step 3 needs a user that NFSD wants. Step 6 meets the "no duplicate drivers" rule, whose exception is a reference implementation that the subsystem maintainers choose. [web: https://rust-for-linux.com/rust-reference-drivers, accessed 2026-10-30]

Out-of-tree cost meanwhile:
- Abstractions cannot ship inside the module: the bindings lack sunrpc and socket headers, and abstractions for out-of-tree users "cannot be merged". [src: K/rust/bindings/bindings_helper.h:29-100] [web: https://rust-for-linux.com/out-of-tree-modules, accessed 2026-10-30] The fleet runs a patched kernel. [judgment]
- Churn to absorb: 558 non-merge commits touched `rust/kernel/` in one year; 83 of their subjects name a rename, removal, replacement or move. [src: K git log --no-merges --since=2025-10-01 -- rust/kernel] Nine mandatory VFS porting notes arrived in the same year. [src: K git log -p --since=2025-10-01 -- Documentation/filesystems/porting.rst]
- Staffing: one to two engineers full time for rebases and security backports. [judgment] The bcachefs move to DKMS shows the risk of an out-of-tree filesystem that depends on internal APIs. [web: https://lwn.net/Articles/1040120, accessed 2026-10-30]

Watch list:
1. A merged xdrgen Rust backend used by lockd or nfsd. [src: K/tools/net/sunrpc/xdrgen/README:259]
2. Any Rust RPC service or sunrpc abstraction in `cel/linux`. [judgment]
3. Rust VFS consumer abstractions with an in-tree user. [judgment]
4. Rust socket abstractions in netdev; the 2026 netdevice RFC is not one. [web: https://lwn.net/Articles/1066512/, accessed 2026-10-30]
5. Distribution kernels with `CONFIG_RUST=y` by default. [src: /boot/config-7.0.0-1012-ibm]
6. Rust as a hard dependency of core code; in 2025 Airlie said core dependencies on Rust "shouldn't happen for another year or two". [web: https://lwn.net/Articles/1050174/, accessed 2026-10-30]
7. A public NFSD statement that accepts Rust-only features or a reference server. [judgment]

## Risks

1. Abstraction rejection or stall blocks the whole option (high). [judgment]
2. Maintainer bandwidth: NFSD, VFS and netdev review time is the limiting resource. [src: K/Documentation/filesystems/nfs/nfsd-maintainer-entry-profile.rst:315-318]
3. A fault takes down all guests on the host; panics replace silent corruption with an outage. [src: K/rust/kernel/lib.rs:178-183]
4. Cross-tenant routing bugs bypass Ceph enforcement (R2). [judgment]
5. Unsound `unsafe` code at the C boundary, as in the first Rust CVE. [src: K commit 3e0ae02ba831]
6. Fleet kernels without `CONFIG_RUST`. [src: /boot/config-7.0.0-1012-ibm]
7. Per-VM libceph clients multiply monitor and OSD connections. [src: K/net/ceph/ceph_common.c:741-744]
8. Level 2 needs `fs/ceph` session export, which no upstream design covers. [judgment]
9. Competing with the NFSD maintainers' own Rust plan damages the relationship that C2 and B2 need. [judgment]

## Open questions

- F10 correction: in 7.3-rc6 `kernel_setlease()` no longer falls back to `generic_setlease()`. It returns `-EINVAL`, so nfsd grants no delegations on CephFS at all. [src: K/fs/locks.c:2108-2115; K commit 2b10994be716]
- P5 and P6 conflict for kernel options: one MDS session per VM forces a superblock per VM, which removes the desirable page-cache sharing. Is one session per tenant per host acceptable for kernel options? [src: K/fs/ceph/super.c:1249-1270, 1340-1341]
- P1: is the host-only NIC the preferred P1 variant for kernel options? On a bridged main tap the local stack does not see the tap, and overlapping guest addresses confuse lockd's host table. [src: K/include/net/inet_sock.h:115-127; K/fs/lockd/host.c:389-391] [judgment]
- P3: is ring-0 co-location of all tenant keys acceptable? Option A avoids it. [judgment]
- Should a future transport include NFS over RDMA (svcrdma) for guests with an RDMA function? It removes host copies but needs non-stock guests. [judgment]
- If the synthesis picks C2, should B2a (generated Rust XDR) join C2 as a parallel track? [judgment]
