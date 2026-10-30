# Option A: userspace Rust NFS proxy

> Option study for the per-host NFS server ("the proxy"). Brief: `design/nfs/requirements.md`, draft 1. Author: Claude Opus 5.5, 2026-10-30. Desk study: no build, test, Git write or cluster action.
>
> Tag paths: a path without a prefix is relative to `/mnt/workspace/rust-mds`. `linux/` is `/mnt/workspace/ceph-client/` (Linux 7.3.0-rc6). `ceph/` is `/home/ubuntu/ceph/`. `map` is `mds-rust-map/cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md`. Web sources were accessed on 2026-10-30.

## Summary

1. Option A runs one unprivileged Rust process per VM on each hypervisor. It contains a new NFS stack (ONC RPC, XDR, NFSv3, MOUNT, NLM (Network Lock Manager), NSM (Network Status Monitor), rpcbind, NFSv4.1) and a new CephFS client on the existing `proto`, `msgr`, `monc`, `auth` and `osdc` crates. [judgment]
2. Each process owns one capsule (`MountState`): the NFS state, the CephFS session identity and the cap (CephFS capability) and lock inventory of one VM. [judgment]
3. Strengths: memory-safe parsing of guest RPC, crash and resource containment per VM, upgrades by local capsule handover, full control of the state that level 2 must move, and in-process tests against the Rust MDS `engine` crate. [judgment]
4. Weaknesses: the most new code of the three options, one more host copy on reads than an in-kernel server on the same TCP transport, and per-VM copies of cluster maps and OSD connections. [judgment]
5. P3 needs MDS-side path-scope checks on day one; the Rust MDS has none [src: HANDOVER.md:149]. Lossless handover needs a new session takeover, in which the MDS moves one session's caps and locks to a new session. libcephfs has a dormant non-RESET reclaim mode that both MDS implementations reject [src: ceph/src/client/Client.cc:18936-18977; ceph/src/mds/Server.cc:491-496; crates/mds/README.md:318-319]. Takeover extends that mode with fencing. [judgment]
6. Shortest path to a product: NFSv4.1 without delegations, level 1 by RESET reclaim (the new session asks the MDS to kill the old session with the same `uuid`). NFSv3 with NLM and NSM follows. [judgment]
7. Upstream: crates in the rust-mds repository first, protocol crates on crates.io, takeover semantics to Ceph. No kernel submission is expected. [judgment]

## Architecture

### Components

| Component | Responsibility | Builds on |
|---|---|---|
| `xdr` (new) | Build-time generator from RFC `.x` files; runtime that decodes `opaque<>` as `Bytes` views and checks lengths before allocation | nothing reusable (R9) |
| `oncrpc` (new) | Record marking with fragment and message limits; CALL and REPLY; AUTH_SYS and AUTH_NONE; calls in both directions on one TCP stream; UDP for rpcbind, MOUNT and NSM; client role for NLM and NSM callbacks | `xdr` |
| `nfs` (new) | NFSv3, MOUNT v3, NLM v4, NSM, rpcbind; NFSv4.1 COMPOUND, sessions, slots, replay cache, state tables, grace, backchannel, delegations | `oncrpc` |
| `cephfs-client` (new) | MDS sessions, requests, caps (CephFS capabilities), flushes, locks, reconnect, reclaim, inode table, striped file I/O | `proto`, `msgr`, `monc`, `auth`, `osdc`, `denc` |
| `nfs-proxy` (new binary) | Per-VM process: sandbox, listener sockets, capsule export and import, metrics, admin socket | all above |
| Host agent (new, shared) | Per-host control plane: VM attach, keys, steering, migration orchestration | — |

**`nfs` crate.**
- NFSv3 clients find MOUNT and NLM through rpcbind. The Linux NLM client sets `RPC_CLNT_CREATE_AUTOBIND` [src: linux/fs/lockd/host.c:489-491]. The proxy therefore serves rpcbind (program 100000), NFS (100003), MOUNT (100005), NLM (100021) and NSM (100024) on the link-local address. [judgment]
- NLM grants a blocked lock with a server-to-client `NLMPROC_GRANTED_MSG` call [src: linux/fs/lockd/svclock.c:881]. After a server restart, NSM `SM_NOTIFY` makes the client reclaim its locks [src: linux/fs/lockd/host.c:578-585]. Both need a connection from the proxy into the guest; NFSv4.1 needs none (P7). [judgment]
- NFS runs on TCP only, because the Linux client disables NFS over UDP by default [src: linux/fs/nfs/Kconfig:203-210]. rpcbind, MOUNT and NSM also answer on UDP. [judgment]
- NFSv3 keeps a duplicate request cache (DRC) for non-idempotent procedures, keyed by XID, client port and procedure. [judgment]
- NFSv4.1 has a COMPOUND dispatcher with current and saved filehandles. SEQUENCE checks a per-session slot table; the Linux client asks for 64 slots [src: linux/fs/nfs/nfs4session.h:12]. The replay cache stores each slot's encoded reply. State tables (client IDs, open, lock and delegation stateids, owners) live in an arena with stable 64-bit keys. Grace runs per capsule. [judgment]
- The backchannel uses the fore connection, because the Linux client creates sessions with `SESSION4_BACK_CHAN` [src: linux/fs/nfs/nfs4proc.c:9275]. Minor versions 0 and 2 get `NFS4ERR_MINOR_VERS_MISMATCH`. [judgment]
- Linux 7.3 requests directory delegations by default and stops after `NOTSUPP` [src: linux/fs/nfs/delegation.c:33; linux/fs/nfs/nfs4proc.c:4403-4418,4497-4499]. The first release answers `NOTSUPP`. Later they map to directory `Fs` caps, which the MDS revokes before each namespace change [src: design/minimum-feature-set.md:268]. [judgment]

**`cephfs-client` crate.**
- One MonClient, one messenger and one Objecter per process. The OSD blocklist matches one client instance (address and nonce) or a whole IP [src: ceph/src/osd/OSDMap.cc:1495-1508]. The MDS session and the OSD I/O of one VM must therefore leave from the same messenger instance, or eviction cannot fence that VM's writes. [judgment]
- Session metadata: `uuid`, `timeout`, `root` (volume path), plus `hostname`, `vm_id` and `proxy_version` for operators. [judgment]
- The inode table holds the pinned set (inodes with open, lock or delegation state, or in use by a request) and a bounded LRU with shared caps for attributes. The proxy drops unpinned inodes at once, so it honours `RECALL_STATE`; ceph-fuse and Ganesha release only as fast as their users do [src: map:726]. [judgment]
- The core is sans-I/O like the engine (F8): a deterministic `step(input) -> effects` machine. Tests drive it against the real `engine` crate in one process. [judgment]
- Data path: a new striper (none exists, F6), object names from `file_object_name` [src: crates/osdc/src/lib.rs:66], `truncate_seq` and `truncate_size` on writes [src: crates/osdc/src/ops.rs:206-213], the realm snap context, and `honor_full = true`. The MDS default is false and adds `FULL_FORCE` [src: crates/osdc/src/objecter.rs:116-118,138,1439-1441].

**Capsule.** `MountState` is versioned plain data. It uses the `denc` versioned framing, which skips unknown tails [src: crates/denc/src/lib.rs:335-336,393]. It holds identifiers, not pointers, and timers as remaining durations. It never holds keys or data buffers. R5 lists the contents. [judgment]

**Process and privilege model.**
- Default: one process per VM. A per-tenant process could share caches, but one guest could then stop the tenant's other mounts, and capsule export would need partitions inside the process. P5 needs one session and one messenger per VM anyway. [judgment]
- Sandbox: an unprivileged user namespace; a seccomp allow-list; Landlock with no filesystem access except a state directory, TCP connect only to Ceph ports [src: linux/include/uapi/linux/landlock.h:479-480] and abstract-socket scoping [src: linux/include/uapi/linux/landlock.h:506]; one cgroup v2 leaf per VM; no core dumps. [judgment]
- The process runs in a storage network namespace that reaches only the Ceph public network. The agent creates the listener sockets in the VM's namespace and passes them by `SCM_RIGHTS`. The proxy needs no `CAP_NET_ADMIN`. For NFSv3 callbacks the agent also brokers outbound sockets into the VM namespace. [judgment]
- The first release uses epoll through tokio, not io_uring. Google reported that 60% of submissions to its vulnerability reward programme involved io_uring, and it disabled io_uring on its production servers [web: https://www.phoronix.com/news/Google-Restricting-IO_uring].

**Transport.**
- A listener without a bound device gives accepted sockets no ingress-interface identity [src: linux/include/net/inet_sock.h:115-127; linux/net/ipv4/inet_connection_sock.c:1227]. The VM identity must come from where the socket lives:
  1. Default: a per-VM network namespace. It holds the tap of the host-only NIC, or a veth that receives link-local traffic from the VM's main tap through an OVS flow or a tc redirect. [judgment]
  2. Per-tap listeners with `SO_BINDTODEVICE`. Sockets on different devices can share one address and port [src: linux/net/ipv4/inet_connection_sock.c:165-168]. This suits routed taps. [judgment]
  3. tc-bpf steering with `bpf_sk_assign` into a per-VM socket [src: linux/net/core/filter.c:7981]. [judgment]
- The zero-copy successor is a vhost-user-net backend (R4). [judgment]

**Control plane.** The agent offers a local API: create a VM mount (tenant, volumes, listener sockets, key, limits), attach and detach a volume (detach revokes that volume's NFS state and releases its caps), export and import a capsule, evict, dump. The control plane issues one cephx key per tenant: `mds 'allow rw path=<tenant root>'` and `osd 'allow rw pool=<data> namespace=<tenant>'`. mgr/volumes already creates namespace-isolated subvolumes [src: ceph/src/pybind/mgr/volumes/module.py:151]. The agent passes the key once through a sealed memfd. The destination agent gets the key from the control plane after a placement check. On day one the agent evicts a dead proxy's session by RESET reclaim under the tenant key [src: crates/mds/README.md:316-333]. [judgment]

**Observability.** Per-VM counters and latency histograms per NFS operation, MDS request and OSD operation, scraped by the agent; tracing spans with VM and tenant labels; one admin socket per process that dumps sessions, slots, stateids, caps and locks; `MClientMetrics` to rank 0 [src: crates/proto/src/lib.rs:23]; session metadata that names the VM. [judgment]

```
 guest VM: stock NFS client --> 169.254.x.y:2049 (v3 also: 111, NLM, NSM)
      | virtio-net (main NIC tap, or host-only NIC)
======|==================================================== hypervisor
      v
 [steering into per-VM netns: OVS flow or tc redirect]   host agent <-> control plane
      | listener fds (SCM_RIGHTS)                          | key memfd, volumes, capsules
      v                                                    v
 +-- nfs-proxy, one per VM: userns, seccomp, Landlock, cgroup, storage netns ---+
 |  oncrpc: record marking, AUTH_SYS, backchannel     capsule: MountState     |
 |  nfs: NFSv3, MOUNT, NLM, NSM, rpcbind | NFSv4.1 COMPOUND, slots, stateids |
 |  cephfs-client: session, caps, locks, inode LRU, striper (sans-I/O core)  |
 |  proto | msgr | monc | auth (tenant cephx) | osdc Objecter                |
 +-----------|-----------------------------------|---------------------------+
             | one MDS session per VM            | OSD ops, same messenger
             v                                   v
       Rust MDS ranks                      OSDs (tenant pool namespace)
```

Data flow (NFSv4.1 WRITE): socket, `oncrpc` frame, `xdr` decode (payload stays a `Bytes` view), slot check, stateid check, caps check (`Fw`, `max_size`), striper, Objecter writes, reply, slot cache. Control flow (COMMIT): the client sends a cap UPDATE with `FLAG_SYNC` and replies after FLUSH_ACK [src: map:291].

## R1 Guest-visible contract

- Endpoint: 169.254.x.y on every host, inside the VM's namespace. [judgment]
- Export naming: the NFSv4.1 pseudo-root `/` lists the VM's attached volume IDs. NFSv3 MOUNT accepts `/<volume-id>`. The agent maps a volume ID to {fscid, subvolume path, root inode}. [judgment]
- Filehandles carry {format, volume ID, fscid, ino, snapid} plus a keyed MAC. They fit NFSv3's 64 bytes and NFSv4's 128 bytes [src: linux/include/uapi/linux/nfs3.h:13; linux/include/linux/nfs.h:28]. Inode numbers never repeat (F2), so no generation number is needed. The MAC rejects forged handles before an MDS round trip; the MDS path scope stays the trust anchor. [judgment]
- `sec=sys`: AUTH_SYS only. RPCSEC_GSS gets an RPC authentication error. SECINFO lists AUTH_SYS only. RPC-with-TLS is not offered. [judgment]
- NFSv3: 21 procedures, MOUNT v3, NLM v4 with blocking locks, NSM, rpcbind. NFSv4.1: sessions, state, grace and backchannel; delegations in the second release. [judgment]

## R2 Identity and tenant isolation

- The VM identity enters only through socket placement. A guest cannot reach another VM's proxy. AUTH_SYS uid and gid serve only POSIX checks inside the tenant. [judgment]
- Tenant scope: the MDS must enforce the cephx `path=` grant on every request, including LOOKUPINO. The Rust MDS does not check MDS auth caps today [src: HANDOVER.md:149]. The OSDs enforce the pool namespace. [judgment]
- Permission checks (P10), day one, in the proxy: mode bits, POSIX ACLs from `system.posix_acl_*`, root squash per volume. libcephfs checks the same way (F3). Attributes cached under `As` caps keep the checks coherent. [judgment]
- For MDS-side checks: they are atomic with the mutation, need no cached attributes, and catch proxy logic bugs. Against relying on them alone: a compromised proxy can claim any `caller_uid` (F3), so MDS mode-bit checks do not stop it. Only the cephx scope and MDS session policy (squash, narrowed path scope) limit a compromised proxy. Recommendation: proxy checks first, MDS session policy next, MDS mode-bit checks as defence in depth. [judgment]
- Keys: one per tenant, in proxy memory only (Architecture, control plane). [judgment]
- Reach after compromise: a guest reaches only its proxy's parser. A compromised proxy holds the tenant key. It reaches that tenant's data inside the cephx scope, and no other tenant. A per-session path scope (R7) narrows this to the VM's attached volumes. A compromised agent reaches every tenant on the host, like a compromised host kernel in options B and C. [judgment]

## R3 Crash containment and attack surface

- A panic, abort or memory overrun stops one VM's mount. The agent restarts the process; the guest sees a server reboot and reclaims (R11). Other VMs and the host stay up. In options B and C the parser runs in the host kernel, where a fault can stop every VM. [judgment]
- Untrusted input: record marking, the RPC header, AUTH_SYS, the XDR of five programs, and the protocol state machines. All of it is Rust. Handwritten protocol logic is about 25,000-35,000 lines [judgment]. For scale, Linux `fs/nfsd` has 56,440 lines of C, `fs/lockd` 10,701 and core `net/sunrpc` 29,947 [src: line counts of linux/fs/nfsd/*.[ch], linux/fs/lockd/*.c, linux/net/sunrpc/*.c].
- Rules: no panic on input paths (`/mnt/workspace/AGENTS.md` section 9.3); one fuzz target per decoder and per COMPOUND sequence; memory limits per connection and per capsule. [judgment]
- Precedent: QEMU 8.0 removed its C virtiofsd in favour of the Rust virtiofsd [web: https://yhbt.net/lore/virtio-fs/20230216182628.126139-5-dgilbert@redhat.com/].
- The vhost-user future adds virtqueue descriptors and a userspace TCP stack to the untrusted surface. [judgment]

## R4 Zero-copy readiness

Write path, guest memory to the OSD:

| Step | First release: TCP on virtio-net | vhost-user future |
|---|---|---|
| Guest page cache to guest socket buffer | copy: the guest RPC client sends a bvec without `MSG_SPLICE_PAGES` [src: linux/net/sunrpc/socklib.c:175-181; linux/net/sunrpc/xprtsock.c:838] | same (guest-owned) |
| Guest TX ring to host socket buffer | copy in vhost-net; zero-copy TX is off by default [src: linux/drivers/vhost/net.c:38-40], and local delivery copies zero-copy fragments anyway [src: linux/net/ipv4/ip_input.c:229-235] | none: the backend reads guest memory through the vhost-user memory table, as passt does [web: https://archives.passt.top/passt-dev/20250121182827.652400c8@elisabeth/T] |
| Host socket to proxy buffer | copy (`recvmsg`) | none: userspace TCP parses in guest memory |
| Proxy to OSD socket | copy, or none with `MSG_ZEROCOPY` above about 10 KB [src: linux/Documentation/networking/msg_zerocopy.rst:22-25] or `IORING_OP_SEND_ZC` [src: linux/include/uapi/linux/io_uring.h:304-305] | none: send from pinned guest pages |

Host copies: two mandatory and one avoidable today, none in the future. The msgr1 data CRC adds one read pass. [judgment]

Read path, OSD to guest memory:

| Step | First release | vhost-user future |
|---|---|---|
| OSD socket to proxy buffer | copy; `TCP_ZEROCOPY_RECEIVE` needs page-aligned payloads [src: linux/include/uapi/linux/tcp.h:128] | copy straight into guest RX buffers |
| Proxy to tap | copy; `MSG_ZEROCOPY` to a tun device gets a deferred copy [src: linux/Documentation/networking/msg_zerocopy.rst:238-246; linux/drivers/net/tun.c:1108] | none |
| Host socket buffer to guest RX ring | copy: vhost-net receives through the tap socket [src: linux/drivers/vhost/net.c:1242-1251; linux/drivers/net/tun.c:2201] | none |
| Guest socket buffer to page cache | copy (guest) | same |

Host copies: three today, one in the future. [judgment]

- Userspace cannot remove the guest's own copies; neither can B or C. It cannot avoid the two tap copies while the guest speaks TCP through vhost-net. It cannot land OSD payloads in guest-posted buffers without NIC DMA into guest memory. io_uring zero-copy receive exists [src: linux/include/uapi/linux/io_uring.h:315,717], but it needs header split and a registered area: a research item. [judgment]
- An in-kernel server on the same transport saves the proxy-to-tap copy on reads, because nfsd and libceph send page references [src: linux/net/sunrpc/svcsock.c:742; linux/net/ceph/messenger_v2.c:168]. On writes, `MSG_ZEROCOPY` brings the proxy level with it. [judgment]
- The path stays open: `xdr` keeps WRITE payloads as views, READ replies keep data as separate segments, and the striper takes scatter lists. [judgment]
- Page-cache sharing within a tenant: P5 gives each VM its own session, and cache validity is per session. A per-tenant shared segment (memfd) can serve a VM only while that VM's session holds `Fr` and `Fc`. This is later work. [judgment]

## R5 Migration readiness

| Capsule part | Level 1 (reclaim) | Level 2 adds |
|---|---|---|
| Header | format version, capsule ID, generation, VM ID, checksum | — |
| Server identity | server owner major ID and server scope (fleet constants), write verifier | minor ID |
| CephFS session | `uuid`, `timeout`, old messenger address and nonce, epoch barrier | cap and lock inventory for checks |
| NFSv3 | NSM state number, monitored guest names, lock inventory | DRC, NLM blocked queue, GRANTED cookies |
| NFSv4.1 | client records (co_ownerid, verifier, principal), open and lock inventory | sessions, slot tables with cached replies, stateids and seqids, owners, delegations, lease remainders |
| Lock owners | NFS owner to 64-bit CephFS owner (F5) | — |

- The server scope must be the same on every host. If it changes, the Linux client reclaims without grace [src: linux/fs/nfs/nfs4proc.c:8892-8898; linux/fs/nfs/nfs4state.c:2039-2040].
- Quiesce: the source stops reading guest sockets, completes in-flight requests (replies enter slot caches and the DRC), drains OSD writes, flushes dirty caps with `FLAG_SYNC`, and serializes the capsule. During live migration this runs in the VM's final pause. [judgment]
- Two-phase commit: phase 1 is the prepared capsule; the source can still abort and resume. Phase 2 is the MDS commit, when the destination's session takes over. After that commit the source is fenced and discards its state. [judgment]
- Level 1 today: the destination sends RESET reclaim for the old `uuid`, which the Rust MDS implements [src: crates/mds/README.md:316-333]. The MDS fences the old session and drops its caps and locks. A new capsule generation makes old session IDs and stateids fail, so the guest reclaims in a grace period for this VM only. Grace ends when the guest sends RECLAIM_COMPLETE (NFSv4.1) or has reclaimed the lock inventory (NLM). [judgment]
- RESET has two gaps. Another VM can take a lock before the guest reclaims it. An unlinked file that only this VM holds open can be purged when its caps go, because unlink-while-open uses strays [src: HANDOVER.md:138]. [judgment]
- Takeover closes both gaps. The MDS fences the old address and raises the OSD epoch barrier (the map epoch a client must reach before it sends OSD operations). Then it moves caps, locks (new session, same owners) and realm membership to the new session in one write-ahead log (WAL) record. [judgment] libcephfs has a dormant non-RESET reclaim mode with a `FLAG_FINISH` step, but its client treats a blocklisted target as lost [src: ceph/src/client/Client.cc:18936-18977]. Takeover therefore needs a new flag with fence-then-transfer semantics. [judgment]
- Level 2 installs the full capsule with the same IDs. The guest sees only broken TCP connections; it reconnects and replays from the slot caches. TCP_REPAIR can even move the connections [src: linux/include/uapi/linux/tcp.h:112-122], as CRIU does [web: https://www.criu.org/TCP_connection]. [judgment]
- Day-one rules for level 2: every NFS-visible identifier derives from the capsule ID and generation, never from the host or process; reply caches store encoded bytes; state uses IDs, not references; one owner task per capsule makes quiesce one drain; timers are relative. [judgment]
- Not encapsulated: kernel TCP state (without TCP_REPAIR), unread socket bytes (the guest retransmits at RPC level), MDS-side caps and locks (only the MDS moves them), cephx tickets and keys, host caches. [judgment]

## R6 CephFS client integration

The proxy is its own CephFS client. It does not use `fs/ceph`. [judgment]

| Behaviour | Need | Size |
|---|---|---|
| Session open with metadata, renewal, STALE, RECALL_STATE, FLUSHMSG ack, FORCE_RO, REJECT, CLOSE with push sequence [src: map:116-154] | must | M |
| Requests: tids, `oldest_client_tid`, forwards, safe and unsafe replies, REPLAY flag, embedded releases [src: map:176-214] | must | M |
| Caps: GRANT, REVOKE, IMPORT, EXPORT, TRUNC, FLUSH_ACK; UPDATE with `FLAG_SYNC`; releases; `max_size`; `truncate_seq`; epoch barrier [src: map:275-309] | must | L |
| Reconnect after MDS failover with caps, locks and realms [src: map:351-388] | must | M |
| `uuid` and `timeout` metadata, RESET reclaim [src: map:405-441] | must | S |
| File locks by owner, blocking waits for NLM, interrupt [src: map:347-349; crates/engine/src/request.rs:46-51] | must | S |
| Striped I/O with snap context and holes (F6) | must | M |
| Quota checks from rstat; CephFS enforces quotas in clients only [src: map:339]; the Rust MDS sends no quota pushes yet [src: HANDOVER.md:149] | must | S |
| Snap trace decoding, read-only | must | S |
| Delegations on caps: read needs `Fr Fc Fs As Ls Xs`, write adds `Fx Fw Fb` [src: map:451] | second release | M |
| Dentry caching beyond the LRU, object cache, FUSE, inline data, snapshot writes, async dirops, fscrypt | omit | — |

- Handles resolve through LOOKUPINO (F1). NFSv4 LOOKUPP uses LOOKUPPARENT. [judgment]
- Change attribute: the client increments it on writes and flushes it with caps; the engine keeps the maximum [src: crates/engine/src/engine/client_caps.rs:272-273]. The engine also advances it in setattr, xattr changes, directory updates and ctime updates [src: crates/engine/src/engine/ops_attr.rs:156,259; crates/engine/src/engine/ops_xattr.rs:144; crates/engine/src/engine/ops_ns.rs:109-127]. A full audit stays open (F4).
- Strict I/O: every WRITE, also UNSTABLE, reaches the OSDs before the reply. COMMIT is one `FLAG_SYNC` flush. SETATTR is one MDS request; its safe reply is durable, so no extra sync follows. [judgment]
- Size: about 10,000-15,000 Rust lines on top of `proto`. libcephfs `Client.{cc,h}` has 21,878 lines [src: line count of ceph/src/client/Client.cc and Client.h]. [judgment]

## R7 Rust MDS requirements

| Candidate | Option A | Reason |
|---|---|---|
| Session policy: path scope | day one | P3; no MDS auth-cap checks [src: HANDOVER.md:149] |
| Session policy: root squash, mode bits, chown, sticky | later | the proxy checks first (R2) |
| Change attribute on every path | day one (audit) | NFSv4.1 cache validation; partly present (R6) |
| Many small sessions | day one at scale | 1,024 sessions per rank [src: crates/mds/README.md:112]; one CAS-guarded object per rank holds all session records [src: design/persistence-contract.md:250]; tombstones stay [src: design/persistence-contract.md:295] |
| Session takeover | second step | lossless level 1, upgrades, level 2; non-RESET reclaim rejected [src: ceph/src/mds/Server.cc:491-496]; at most 128 reclaim waiters [src: crates/mds/README.md:333] |
| Admin eviction API | later | RESET reclaim from the agent covers day one |
| Lean non-caching session mode | later | the proxy releases unwanted caps itself |
| Transactional inode index | with multiple ranks | one rank today; LOOKUPINO works [src: crates/engine/src/engine/ops_read.rs:125-152] |

Production admits one journal record at a time [src: HANDOVER.md:151; crates/mds/README.md:53-54]. Every synchronous NFS operation of every VM then queues behind one WAL write per rank. Group commit is a shared prerequisite. [judgment]

## R8 Performance

| NFS operation | MDS round trips, Option A | Ganesha with FSAL_CEPH |
|---|---|---|
| GETATTR, ACCESS | 0 with cached shared caps, else 1 | same |
| LOOKUP | 1 (no dentry cache at first) | 0 or 1 |
| CREATE with mode and owner | 1: owner travels in the request head [src: map:201] | lookup, CREATE, SETATTR and sync [src: map:532] |
| REMOVE, RENAME, LINK, MKDIR | 1 | lookups, then 1 [src: map:246] |
| SETATTR | 1 | 1, plus a flush when the change was buffered [src: map:505] |
| Stable WRITE | OSD write, then 1 `FLAG_SYNC` flush (DATA_SYNC: only when the size grows) | OSD write, then fsync |
| LOCK | 1 | 1, plus an OPEN per lock stateid [src: map:538] |

- Strict-I/O latency is one MDS round trip plus one WAL commit per SETATTR or COMMIT. Single-record admission makes the WAL the bottleneck for every option. [judgment]
- Throughput per host: bounded by the NIC and the copies. Expect about 0.2 to 0.5 CPU cores per GB/s, by direction, before zero-copy work. [judgment]
- CPU per small operation: syscalls and RPC handling dominate; batching and, later, io_uring reduce them. [judgment]
- Memory per VM: process baseline, one OSDMap copy, the inode LRU, slot caches and I/O buffers. Expect 30 to 100 MB, bounded by `memory.max`. An in-kernel server shares maps and page cache. [judgment]
- Connections: one messenger per VM gives one TCP connection per VM per OSD touched. P5 and per-session fencing cause this, so B and C pay it too with one session per VM. [judgment]

## R9 Completeness and time to first product

| Asset | State | Use |
|---|---|---|
| `proto` | every MClient message, encode and decode [src: crates/proto/src/request.rs:933-960; crates/proto/src/reply.rs:1251-1260] | reuse |
| `msgr` | msgr1 client and server; msgr2 missing [src: HANDOVER.md:190]; a messenger starts only through `bind` [src: crates/msgr/src/messenger.rs:163,172] | reuse; add a start without listener |
| `monc`, `auth` | MonClient, cephx client side, tickets [src: crates/monc/src/lib.rs:1-9; crates/auth/src/lib.rs:1-13] | reuse |
| `osdc` | Objecter with epoch barrier [src: crates/osdc/src/objecter.rs:646]; msgr1 OSD addresses only [src: HANDOVER.md:182]; no striper | reuse; add striper |
| `engine` | in-process MDS | client tests |
| `nfsserve` 0.11.0 (BSD-3-Clause, 2026-04-01) and its fork `nfs3_server` 0.11.1 | NFSv3 "incomplete but very functional", partial MOUNT, portmap GETPORT only, no NLM, no NFSv4 [web: https://docs.rs/crate/nfsserve/latest; https://github.com/huggingface/nfsserve; https://docs.rs/nfs3_server] | reference |
| `fernfs` 0.1.5 (BSD-3-Clause) | 21 NFSv3 procedures, MOUNT, PORTMAP [web: https://docs.rs/crate/fernfs/latest] | review for NFSv3 reuse |
| `embednfs` (MIT) | NFSv4.1 for macOS over localhost; no backchannel, no delegations [web: https://docs.rs/crate/embednfs/0.1.5] | reference |
| nfs4j (Java, LGPL) | NFSv3, 4.0, 4.1 and pNFS, used by dCache [web: https://github.com/dcache/nfs4j] | design precedent |
| `xdrgen` 0.4.4; `onc-rpc` 0.3.3; `onc-rpc-wire` 0.2.1 | types only, last release 2017 [web: https://docs.rs/crate/xdrgen/latest]; no fragmented messages [web: https://docs.rs/crate/onc-rpc/latest]; wire types and record marking, 2026 [web: https://docs.rs/crate/onc-rpc-wire/latest] | insufficient; evaluate `onc-rpc-wire` |
| `io-uring` 0.7.15; `tokio-uring` 0.5.0; `compio` 0.19.1 | tokio-rs, 2026-09-07 [web: https://lib.rs/crates/io-uring]; 2024-05-27 [web: https://lib.rs/crates/tokio-uring]; 2026-06-14 [web: https://lib.rs/crates/compio]; glommio and monoio reportedly without release since 2024 [web: https://github.com/gvozdetsky/ruxen/issues/272] | later; `io-uring` directly |
| `vhost-user-backend` 0.24.0; virtiofsd 1.14.0; `smoltcp` 0.14.0 | rust-vmm, 2026-10-22 [web: https://lib.rs/crates/vhost-user-backend]; Rust vhost-user precedent [web: https://lib.rs/crates/virtiofsd]; userspace TCP, 2026-08-17 [web: https://crates.io/api/v1/crates/smoltcp] | zero-copy future |

No Rust crate covers NFSv4.1 server state, NLM or NSM. The generator must be new. The kernel's `xdrgen` shows the method [src: linux/tools/net/sunrpc/xdrgen], but it is GPL and emits C; we reuse only the idea. [judgment]

## R10 Upstream path

Target: the rust-mds repository first, then crates.io for the protocol crates; the proxy follows the Rust MDS. The Upstream plan gives deciders, precedent, submissions, objections and cost. [judgment]

## R11 Operability

- Deployment: one host package (agent and `nfs-proxy`), independent of the host kernel. [judgment]
- Upgrade: start the new binary beside the old one and move each capsule locally, one VM at a time, without a host reboot. Level 1 first, level 2 later. The capsule version gates mixed versions. [judgment]
- Restart (P4): the agent restarts the process. It reads its level-1 record from host tmpfs (client records, NSM state, server identity, `uuid`), sends RESET reclaim and opens grace for that VM. Host tmpfs suffices, because a host reboot stops the VM too. A crash changes the write verifier; a cooperative handover keeps it, because data and caps are flushed. [judgment]
- Bulk restarts must stay under 128 reclaim waiters per rank [src: crates/mds/README.md:333]. [judgment]
- Resource control: a cgroup v2 leaf per VM (CPU, memory, pids) under a tenant slice; operation and byte token buckets in the proxy; tc shaping on the VM veth. [judgment]
- Observability: see Architecture. [judgment]

## Shared work packages

Shared with options B and C [judgment]:
- SW1 MDS path-scope enforcement and per-session policy (M).
- SW2 change-attribute audit of all mutation paths (S).
- SW3 session scaling: higher limit, sharded session table, tombstone collection (M).
- SW4 group commit, several records in flight (L).
- SW5 session takeover: a fence-then-transfer reclaim flag and per-session reconnect while the rank is active (L).
- SW6 host agent and control plane (L).
- SW7 per-VM link-local steering (M).
- SW8 qualification: pynfs, Connectathon, Ganesha as oracle, migration fault tests (L).
- SW9 one filehandle and server-identity specification for all options, so a VM can move between hosts that run different options (S).

## Work packages

| ID | Package | Size | Depends on |
|---|---|---|---|
| A1 | `xdr` generator and runtime, fuzz targets | M | — |
| A2 | `oncrpc`: record marking, AUTH_SYS, both directions, UDP, rpcbind | M | A1 |
| A3 | NFSv3, MOUNT, DRC | L | A2, A6 |
| A4 | NLM v4, NSM, reverse callbacks, grace | M | A3, SW7 |
| A5 | NFSv4.1 COMPOUND, pseudo-root, attributes, READDIR | XL | A2, A6 |
| A6 | `cephfs-client` core: sessions, requests, caps, flushes, locks, reconnect, reclaim | XL | — |
| A7 | Data path: striper, OSD I/O, `max_size`, holes, client Objecter mode | M | A6 |
| A8 | NFSv4.1 state: client IDs, sessions, slots, replay cache, stateids, leases, grace | L | A5 |
| A9 | Backchannel and delegations on caps | M | A8 |
| A10 | `nfs-proxy` process, sandbox, fd passing, metrics, admin socket | M | A2 |
| A11 | Capsule format, quiesce, level 1 handover | M | A4 or A8, SW6 |
| A12 | Level 2 handover, optional TCP_REPAIR | L | A11, SW5 |
| A13 | Permission checks in the proxy: mode, POSIX ACL, root squash | M | A6 |
| A14 | `MSG_ZEROCOPY` and io_uring send | M | A7 |
| A15 | vhost-user-net backend with userspace TCP | XL | A14 |
| A16 | msgr2 client with secure mode for MDS and OSD connections; not shared with B and C, whose kernel client has it | L | — |

Sizes: S up to two engineer-weeks, M one to two months, L three to six months, XL more than six months. [judgment]

- NFSv3-only product: A1, A2, A6, A7, A3, A13, A10, A11, then A4; plus SW1, SW3, SW6, SW7, SW8. A6 is the critical path. NFSv3 without NLM ships earlier but breaks applications that lock. [judgment]
- NFSv4.1 product: A1, A2, A6, A7, A5, A8, A13, A10, A11; plus SW1, SW2, SW3, SW6, SW7, SW8. It adds about one XL and one L to the NFSv3 core, but needs no NLM, NSM, rpcbind or reverse route. [judgment]
- Recommendation: NFSv4.1 first, without delegations; cephadm's Ganesha also runs without them [src: map:454]. The link-local steering then carries one TCP port. NFSv3 with NLM follows. [judgment]

## Upstream plan

- Phase 1 home: crates in the rust-mds repository. The client reuses six local crates, and takeover changes `proto`, `engine`, `mds` and the client together; one CI covers both sides. [judgment]
- Phase 2 home: `xdr`, `oncrpc` and the NFS protocol types as independent crates on crates.io; the proxy stays with the Rust MDS and follows it if it joins the Ceph organisation. Precedent: `ceph/ceph-nvmeof`, a separate LGPL-3.0 gateway repository in that organisation [web: https://github.com/ceph/ceph-nvmeof].
- Deciders: the CephFS component lead (Venky Shankar [web: https://ceph.io/en/community/team/]) and the Ceph Executive Council (Dan van der Ster, Neha Ojha, Patrick Donnelly [web: https://docs.ceph.com/en/latest/governance/]); on crates.io, the project maintainers.
- Licensing: rust-mds declares no licence: no LICENSE file and no `license` field in any `Cargo.toml` [src: Cargo.toml; crates/*/Cargo.toml]. `osdc` ports the LGPL-2.1 CRUSH mapper [src: crates/osdc/src/crush/mapper.rs:1-4], so the Ceph-facing crates should be LGPL-2.1-or-later, compatible with Ceph's LGPL-2.1-or-LGPL-3 core [src: ceph/COPYING:8]. The protocol crates derive from neither Ceph nor the kernel and can be Apache-2.0 OR MIT. The NFSv4.1 XDR comes from RFC 5662 under the Simplified BSD licence [web: https://www.rfc-editor.org/rfc/rfc5662.html]. RFC 1813 carries no copyright notice and does not define all of NLM [src: linux/Documentation/sunrpc/xdr/nlm4.x:3-9]. Write NLM and NSM `.x` files from the X/Open specification, not from the kernel tree (P8). [judgment]
- Ganesha (LGPL-3 [src: /usr/share/doc/nfs-ganesha/copyright:10]): a Rust FSAL is no product path. Ganesha's cache, thread model and recovery backends assume a shared gateway, and its state cannot become a capsule. It is a stepping stone: one Ganesha with FSAL_CEPH per VM is a working per-host baseline today and an oracle for MDS behaviour. Takeover reclaim would also let Ganesha keep locks across node takeover. [judgment]
- Ceph upstream: takeover semantics (a reclaim flag that fences the old session and keeps its caps and locks, a feature bit, the fencing order), so the C++ MDS can serve the same proxy; and per-session path-scope policy. Kernel: nothing expected; client bugs found in tests go to linux-nfs. [judgment]
- First three submissions [judgment]:
  1. Ceph: a design RFC on the developer list and a pull request with `doc/dev` text for a takeover reclaim flag beside the dormant non-RESET mode, plus `src/test/libcephfs/reclaim.cc` cases. Expected objection: MDS complexity and moving rights between sessions. Answer: the same `auth_name` check as RESET, and fencing before transfer.
  2. crates.io: `xdr` and `oncrpc` with RFC test vectors and fuzz targets. Invite the `nfsserve`, `nfs3_server` and `fernfs` maintainers to share them.
  3. rust-mds: the MDS path-scope patch (SW1) with the `cephfs-client` crate and its in-process tests against `engine`; then the NFSv4.1 proxy with published pynfs results. pynfs is maintained on git.linux-nfs.org; a March 2026 post names Calum Mackay as maintainer [web: https://yhbt.net/lore/linux-nfs/33de50bc-5cca-4071-ad32-05a82da89c77@oracle.com/].
- Other objections: "Ganesha exists"; "the Rust MDS is not Ceph upstream"; "per-VM sessions overload the MDS". Cost if Ceph rejects takeover: the feature stays in the Rust MDS, which this project owns; the C++ MDS is no target (P8). [judgment]

## Risks

1. Scope: the NFSv4.1 server and the CephFS client are both XL. Mitigation: NFSv4.1 without delegations first. [judgment]
2. Correctness against the real Linux client (reboot recovery, slot reuse). Mitigation: pynfs and kernel-client test matrices. [judgment]
3. MDS scale: 1,024 sessions per rank and one record in flight cap the product [src: crates/mds/README.md:112; HANDOVER.md:151].
4. CPU cost of three host copies per read until vhost-user exists. [judgment]
5. OSD connection fan-out from per-VM messengers. [judgment]
6. msgr1 only. Clusters that require secure mode need A16 first. msgr1 is still bound by default [src: ceph/src/common/options/global.yaml.in:1270-1274].
7. Level 1 by RESET can lose locks and open-unlinked files until takeover exists (R5). [judgment]
8. The rust-mds licence is undecided [src: Cargo.toml].
9. CephFS enforces quotas in clients [src: map:339]; a compromised proxy can exceed them. [judgment]

## Open questions

1. P5 against P6: one session per VM makes cache validity per session, so tenant-wide page-cache sharing needs per-session `Fc` tracking in every option. Is the sharing goal worth that cost? [judgment]
2. P3 and capacity: should the MDS bound `max_size` grants by the remaining quota, so that capacity isolation does not depend on the proxy? [judgment]
3. P1 and P7 for NFSv3: NLM and NSM need a route into the guest. Must the first NFSv3 release support locking? [judgment]
4. Which VM density per rank must the first product reach, given the 1,024-session limit? [judgment]
5. Should filehandles and server identity be identical across options A, B and C (SW9)? [judgment]
6. Who issues tenant keys: a Ceph mgr module or the cloud control plane? [judgment]
