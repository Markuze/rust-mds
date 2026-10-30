# Per-host NFS proxy over the kernel CephFS client: outline and benefits over Ganesha

> Outline of the chosen direction A-K (`design/nfs/comparison.md`, decisions D1 and D2, accepted in principle by the user on 2026-10-30). Draft 1, 2026-10-30. Companion documents: `design/nfs/requirements.md` (pinned decisions, facts, steering), `design/nfs/mds-adjustments.md` (M1-M9). Sizes: S up to two engineer-weeks, M one to two months, L three to six months, XL more than six months.

## 1. The approach in one paragraph

Every hypervisor runs one NFS proxy process per VM. The proxy is a userspace Rust NFS server. It serves one guest over a well-known link-local address that the host intercepts on that VM's own tap device, so the mount command is the same on every host. The proxy reaches CephFS through the host kernel's own CephFS client, `fs/ceph`, over one kernel mount per tenant per host, mounted with that tenant's cephx key. The Rust MDS serves the metadata; the OSDs serve the data. There is no gateway tier, no virtual IP, no grace cluster and no Ganesha.

## 2. Deployment shape

- **Per host:** one host agent (control plane), one kernel CephFS mount per tenant that has a VM on the host, one proxy process per VM with an attached volume.
- **Per tenant:** one cephx identity with MDS caps restricted by `path=` to the tenant's subtree and OSD caps restricted to the tenant's pool namespace. The MDS and the OSDs enforce the tenant boundary; the proxy holds no secret.
- **Per VM:** one proxy process inside the VM's tap network namespace, one capsule of NFS state, one cgroup. The process holds CAP_DAC_READ_SEARCH for handle decoding and the capability to switch its filesystem uid per request, and nothing else.
- **Guest side:** a stock image. `mount -t nfs -o vers=4.1 169.254.x.y:/vol-id /mnt` with `sec=sys`. No agent, no Kerberos, no TLS, no idmapd configuration.
- **Attach model:** attaching a volume to a VM, like attaching a disk, mounts the tenant filesystem on the host if needed, starts the VM's proxy, and exports the volume. Detaching stops the proxy.

## 3. Components

| Component | Role | Size |
|---|---|---|
| `xdr` crate | XDR runtime and a generator from the RFC `.x` specifications; one fuzz target per decoder | M |
| `oncrpc` crate | record marking, RPC headers, AUTH_SYS, forward and backchannel; rpcbind only for NFSv3 | M |
| `nfs` crate, NFSv4.1 | COMPOUND processing, pseudo-root, attributes, READDIR, client records, sessions, slot tables with reply cache, stateids and owners, leases, per-VM grace, backchannel | XL plus L |
| `nfs` crate, NFSv3 | NFSv3, MOUNT, NLM, NSM; later, if customers need it | L plus M |
| VFS backend | handles through `open_by_handle_at`; permission enforcement by switching the request's filesystem uid and gid after squash rules; OFD locks on one descriptor per NFS lock owner; READ and WRITE by splice through the page cache; statx attributes, xattrs and POSIX ACLs; the change-attribute source | M |
| Capsule | the versioned, serializable NFS state of one VM: server identity and write verifier, client records, sessions and cached replies, stateids, lock-owner map, delegation records; quiesce, capture, import | M, later L for level 2 |
| Process and sandbox | per-VM process, seccomp and Landlock, cgroup limits, metrics, admin socket | M |
| Host agent | attach and detach, tenant mounts, key provisioning, namespace and tap setup, restart and migration orchestration | M |

## 4. Data and control flow

- **Metadata.** Guest RPC enters the proxy, which decodes it in Rust and issues VFS calls on the tenant mount. `fs/ceph` turns them into MDS requests or answers them from caps it holds. The Rust MDS answers after its WAL commit (safe replies only), so acknowledged metadata is durable.
- **Data.** READ splices from the page cache to the guest socket. WRITE splices from the guest socket into the page cache; a stable write or COMMIT ends with fsync, which `fs/ceph` turns into an OSD write plus a synchronous cap flush. The copy counts equal nfsd's, and host-socket zero-copy applies unchanged.
- **Locks.** NFS byte-range locks become OFD locks on a per-owner descriptor, which `fs/ceph` sends to the MDS with a distinct owner. Blocking requests wait in a thread.
- **Delegations.** None until the `fs/ceph` lease operation tied to caps exists. Then a delegation is a kernel lease; a cap revoke by the MDS breaks the lease, and the proxy recalls over the backchannel.
- **Cache coherence.** The kernel client's caps keep the host page cache coherent with every other CephFS client. All of a tenant's VMs on the host share that cache.

## 5. Identity, enforcement and isolation

- The VM's identity is where the socket lives: the proxy listens inside the VM's tap namespace, so every connection it accepts is that VM.
- The guest's uid and gid are squashed per volume policy and then enforced by the host VFS and `ceph_permission`, exactly as for knfsd. The proxy contains no permission code.
- The tenant boundary is Ceph's: the tenant key's `path=` and namespace caps, enforced by the MDS (M1 in `mds-adjustments.md`, a day-one item, because the engine enforces nothing today) and by the OSDs.
- A proxy crash stops one VM's mount; the host agent restarts the process and the guest reclaims. Other VMs do not notice.

## 6. Migration

- **The capsule is the unit.** The NFS state of a VM moves as data. The destination proxy presents the same server identity and address.
- **Level 1, first.** The destination imports client records, the guest reconnects and reclaims opens and locks in a per-VM grace. On the Ceph side the destination reopens every file by handle before the source closes it, which prevents stray purge of unlinked open files, then re-takes the locks. Another client can take a lock in that window; the MDS lock reservation on reclaim (M9) closes it.
- **Level 2, later.** The destination imports the full capsule, cached replies included, and the guest sees only a TCP reconnect. The NFS half is by construction. The Ceph half, caps and locks that move without a gap, needs per-VM sessions and session takeover (M4), which is the Rust CephFS client's job if it is ever built.

## 7. Kernel and MDS work this needs

`fs/ceph` series, driven on ceph-devel with an independent co-maintainer review:

1. Export operation flags and `commit_metadata` (S). First submission; a correctness fix for every knfsd re-export user.
2. A `setlease` operation tied to caps (M), modelled on libcephfs delegations. Required for any delegation. Read leases first.
3. Exposure of the MDS change attribute to userspace (S): an ioctl, or a statx policy change on linux-fsdevel. Until then ctime stands in.
4. Asynchronous lock grant (M), optional, because the proxy can wait in a thread.
5. uuid and timeout reclaim (M), only if per-VM mounts are ever used.

Rust MDS items (`mds-adjustments.md`): M1 enforcing session policy with `path=` scope on every operation including LOOKUPINO (day one, default off for other peers); M3 change-attribute test; M6 many-sessions benchmark; M7 admin eviction interface; M9 lock reservation on reclaim; M4 session takeover later with the Rust client.

## 8. Phases

0. **Baseline, no code.** knfsd on one host over per-tenant kernel mounts. It is the performance reference and the test bed for M1, M3 and M6.
1. **Foundations.** M1, M3, M6 in the MDS; kernel series 1.
2. **First product.** NFSv4.1 without delegations: `xdr`, `oncrpc`, the NFSv4.1 server and state, the VFS backend, process and sandbox, capsule with level 1 handover, host agent. NFSv4.1 first because it needs one port, no route into the guest, and no rpcbind, NLM or NSM.
3. **Delegations.** Kernel series 2, then backchannel recalls on leases.
4. **Migration.** Level 2 for the NFS state; M7 and M9 in the MDS. NFSv3 with NLM and NSM here if needed.
5. **Optional.** The Rust CephFS client for per-VM sessions and takeover; a vhost-user transport for guest-memory zero-copy.

## 9. Verification

pynfs against the proxy; the Linux client matrices for reboot recovery, slot reuse and reconnect; the offline NFSv4.1 wire decoder in `scripts/`; fuzzing of every decoder and of COMPOUND sequences; in-process tests of MDS semantics against the `engine` crate; the knfsd baseline for throughput and CPU per byte.

## 10. Benefits over Ganesha

Ganesha here means the mgr/nfs deployment: a centralized gateway tier behind an ingress virtual IP, a RADOS-backed grace database for node takeover, one libcephfs client per daemon with caps `allow rw path=/`, and tenant isolation by export configuration.

| Benefit | Ganesha | This approach |
|---|---|---|
| Data path | two network hops per byte, gateway NICs are the ceiling | one hop; throughput scales with host count; no tier to size |
| Failure domain | a gateway failure puts every client of that gateway into grace, with new opens and locks blocked for the grace period | a proxy failure affects one VM; a host failure takes its guests with it, so there is nothing to fail over |
| Tenant isolation | one client with `allow rw path=/`; isolation is export configuration | one key per tenant with `path=` and namespace caps, enforced by the MDS and the OSDs; one process per VM |
| Client identity | network clients authenticated by address, Kerberos or TLS | the VM's tap namespace; no authentication protocol in the guest |
| Noisy neighbours | all tenants share one MDS session per daemon; fairness needs per-session throttles (design decision 7) | one session per tenant per host; cgroup limits per VM |
| Caching | at the remote gateway | in the host page cache, shared by the tenant's VMs on that host, coherent through caps |
| Delegations | disabled in the cephadm template; recall crosses the network | local recall latency makes them practical once the kernel lease operation exists |
| Migration | the server address never moves, so migration is free, but there is no state export | NFS state moves as a capsule; level 2 removes the reclaim storm; requires orchestration |
| Operations | ingress, keepalived, grace cluster, recovery database, cephadm nfs service | a host package and a per-VM process tied to volume attach |
| Guest-facing parser | C, shared by all clients of the gateway | Rust, one process per VM |
| Protocol surface | every client and every authentication flavour | NFSv4.1 over one port first; NFSv3 only on demand |
| Ceph client | libcephfs, C++, one client per daemon, dormant non-RESET reclaim | the in-tree kernel client, msgr2 included, already the Rust MDS's primary peer |
| Co-design with the MDS | libcephfs and the C++ MDS release cycles | the Rust MDS and the proxy evolve together; kernel changes only in `fs/ceph`, which the project can drive |
| Zero-copy | none | nfsd-equal copies by splice today; vhost-user later |

## 11. What this approach gives up

- **New code.** An NFSv4.1 server is XL. Ganesha exists today.
- **Kernel dependencies.** Delegations and the change attribute need `fs/ceph` patches to land, or ship without delegations and with ctime.
- **A fleet of small servers.** Thousands of proxies instead of a few gateways; the host agent must be as reliable as the proxy.
- **Migration is not free.** It needs orchestration, and the Ceph-side lock window stays open until M9 or takeover exists.
- **Per-VM sessions and lossless takeover** wait for the Rust CephFS client, if it is ever needed.
