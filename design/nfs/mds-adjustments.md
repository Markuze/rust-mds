# Rust MDS adjustments for a per-host NFS proxy

> Companion to `design/nfs/requirements.md` (rubric item R7). Draft 1, 2026-10-30. This note is option-independent: every candidate adjustment is needed, or not needed, for the same reasons whether the proxy is userspace Rust, in-kernel Rust or kernel C. It records the current state of the engine and daemon with sources, a design sketch per item, and which options need which item. It proposes; it decides nothing. Decisions go to `design/minimum-feature-set.md` section 8.1.

## 1. Summary

| Id | Adjustment | Current state | Needed by | Size |
|---|---|---|---|---|
| M1 | Enforcing session policy (path scope, root squash, mode bits, chown rules, sticky directories) | Absent. The engine trusts caller ids. No MDS caps grammar parser exists. | A: day one (trust anchor). B, C: defence in depth, later. | L |
| M2 | Transactional inode-number index | Planned, not built. Live inodes resolve in memory. | All, only with more than one rank or shard. | M |
| M3 | Change attribute on every mutation path | Present on namespace, xattr, setattr and cap-flush paths. | All. Test only. | S |
| M4 | Session takeover (per-session reconnect while the rank stays active) | Absent. Rank-wide reconnect, fencing and reclaim plumbing exist as parts. | A: level 1 can start without it; level 2 needs it. B, C: only with uuid reclaim in `fs/ceph`. | L |
| M5 | Lean non-caching session mode | Absent. | A only, optional. | M |
| M6 | Many small sessions | Per-session cap recall hint exists. No scale measurement. | All (one session per VM). | M, mostly test |
| M7 | Admin eviction interface | Absent. The daemon has no admin socket. | All. B, C: required for level 1 without uuid reclaim. | M |
| M8 | uuid and timeout reclaim from any session class | Reclaim reply plumbing exists in the daemon. Acceptance rules unverified. | B, C with the `fs/ceph` reclaim patch. | S |
| M9 | Lock reservation on reclaim | Absent. Decision 6 chose no recovery hold; option (c), per-inode lock reservation, was left open. | All options at migration level 1, including A-K. | M |

## 2. M1 Enforcing session policy

**Current state.** The engine trusts `caller_uid` and `caller_gid` in every request and returns EACCES nowhere; EPERM appears twice, for MKSNAP and one setattr rule [src: crates/engine/src/engine/mod.rs:800, crates/engine/src/engine/ops_attr.rs:68]. The daemon decodes the caps string from the cephx ticket but no code parses the MDS caps grammar or applies `path=`, `uid=` or `root_squash` [src: crates/auth/src/types.rs:40; no other match for `path=` under `crates/`]. The workplan lists "Enforce complete authenticated MDS caps, including handle lookup and strays" in a later stage [src: design/workplan.md:60]. The C++ MDS checks UNIX mode bits only when a grant carries a `uid=` restriction, and then applies owner, group and other bits, chown and chgrp rules [src: /home/ubuntu/ceph/src/mds/MDSAuthCaps.cc:340-392]. The inode record holds `mode`, `uid`, `gid` and `nlink` [src: crates/engine/src/inode.rs:30-35], so the engine has every input the check needs.

**Why.** An NFS client asserts its uid and gid. A proxy can check permissions itself, as libcephfs does with `client_permissions`, but then the proxy is the trust anchor and a proxy bug is a tenant breach. With the check in the MDS, the tenant key and the session policy bound what any proxy can do (P3, P10).

**Sketch.**
- A `SessionPolicy` attached at session open: root scope as an inode plus path prefix, `root_squash`, `enforce_modes`, optional uid and gid allow lists, and the vxattr, snapshot and full flags of the C++ grammar.
- Source of the policy: parse the cephx MDS caps grammar for the C++ fields, and add the Rust-only `enforce_modes` from daemon configuration keyed by entity name, because the monitor validates the MDS caps grammar and rejects unknown tokens. Open question: whether a session metadata key is acceptable instead.
- One `check_access(policy, inode, caller, mask)` with the C++ semantics generalized to any caller uid, plus the sticky-directory rule for unlink and rename that the C++ MDS leaves to clients. Call sites mirror the C++ `check_access` callers, one per operation, inside the engine before the journal record is built.
- Default off. Kernel and fuse sessions keep today's behaviour unless their entity matches a configured policy.

**Size.** L: grammar parser, policy type, one check per operation, tests per operation. Risk: behavioural change for existing peers if the default is wrong, so the default must be off and the test suite must prove it.

## 3. M2 Transactional inode-number index

**Current state.** The design requires an index from inode number to shard, location and primary parent, updated in the same transaction as the namespace change, for LOOKUPINO, LOOKUPPARENT, LOOKUPNAME and reconnect [src: design/minimum-feature-set.md, bullet "A transactional inode-number index"]. The engine answers the three lookups from its in-memory map today [src: crates/engine/src/engine/ops_read.rs:125-180]. Inode numbers are never reused [src: design/minimum-feature-set.md, bullet "Inode numbers"].

**Why.** NFS handles arrive at any rank and for any inode, cached or not. With one root shard and a fully loaded namespace the in-memory map suffices. With a second rank or an unloaded shard it does not.

**Sketch.** As designed. No change for the proxy beyond ordering: the index becomes necessary at the first multi-rank or lazy-load milestone, not at the first proxy release.

**Size.** M, already in the plan.

## 4. M3 Change attribute

**Current state.** The engine increments `change_attr` on directory mutation and on ctime changes [src: crates/engine/src/engine/ops_ns.rs:109-126], on xattr change [src: crates/engine/src/engine/ops_xattr.rs:144] and on setattr [src: crates/engine/src/engine/ops_attr.rs:156,259]. A cap flush raises it to the client's reported value [src: crates/engine/src/engine/client_caps.rs:272-273], which is how data writes reach it: the client increments the attribute on write and reports it in the flush, as the kernel client and libcephfs do.

**Why.** NFSv4 clients validate every cache with the change attribute. nfsd takes it from the inode version that `fs/ceph` fills from this field. A missed increment is a stale guest cache.

**Sketch.** No engine change. Add a test that every mutating operation and every cap flush with dirty file caps advances the attribute, and that a proxy-side client crate increments it on every write before the flush.

**Size.** S.

## 5. M4 Session takeover

**Current state.** Reconnect is a rank-wide recovery phase: `reconnect_chunk`, `reconnect_cap`, `reset_reconnect`, `finish_reconnect(keys)` [src: crates/engine/src/engine/reconnect.rs:25-138, crates/engine/src/engine/client_caps.rs:557-617]. `finish_reconnect` seals the recovery gather and grants nothing by itself [src: crates/engine/src/engine/reconnect.rs:134-138]. The daemon runtime checks blocklist state and raises the OSD epoch barrier during recovery [src: crates/mds/src/server/runtime/recovery.rs:754,999,1105,1132,1338]. The daemon routes `CLIENT_RECLAIM_REPLY` through an owner-tagged pending output [src: crates/mds/src/session.rs:38-40,299-304]. Ganesha's reclaim today is RESET: the MDS kills and blocklists the session with the same uuid and the new session starts empty [src: mds-rust-map/cpp-map/02-client-protocol/libcephfs-fuse-ganesha-expectations.md, section 3.16.2].

**Why.** With RESET, Ceph-level locks and caps drop between release and NFS reclaim, and native clients can take them in that window (decision 6 gap). With takeover they never drop. Level 2 migration also needs the session's completed-request table and flush tids to continue, which RESET discards.

**Sketch.**
1. The destination proxy opens a session with the same cephx entity and sends a reclaim with the uuid and a takeover flag. The MDS verifies the entity name as the C++ MDS does for RESET.
2. Fencing. If the source connection is already closed, the session is open without a connection (the C++ MDS keeps such sessions; the map page lists this as required behaviour, item 14) and no blocklist is needed. If the source connection is alive, the MDS blocklists the source address and raises the OSD epoch barrier, as eviction does today.
3. Per-session reconnect while the rank stays active. The destination sends the caps and locks it holds, from the capsule. The engine reconciles them with the session's recorded state using the `reconnect_cap` logic for one session key, with the affected inodes frozen for the duration. The MDS record is authoritative: undeclared caps are released, unknown caps are refused.
4. Continuity. The session keeps its completed-request table, flush tids and tid counter. The outbound message sequence restarts at zero on the new connection; the proxy resets its push sequence at takeover. Open question: continue the sequence from the capsule instead.
5. Cooperative handover needs no new message. The source flushes, then closes its connection. The destination takes over. Fencing applies only to a source that is still alive or unreachable.

**Wire.** The takeover flag is a new bit in the reclaim `flags` field, meaningful to the Rust MDS and the proxy only. The C++ MDS is not a target (P8). Open question: the exact bit and the behaviour of a C++ MDS that receives it.

**Size.** L: engine entry point for one-session reconciliation, daemon flow, fencing rules, tests with a live kernel client holding conflicting caps during takeover.

## 6. M5 Lean non-caching session mode

**Current state.** The engine issues dentry leases and directory caps to every session by the C++ rules [src: crates/engine/src/engine/caps_eval.rs]. A proxy that keeps no dentry or attribute cache beyond its pinned set receives grants it never uses and must answer revocations for them.

**Sketch.** A session metadata key declared at open, for example `nocache=1`. For such sessions the engine issues no dentry leases and only the caps the proxy asks for: pin and auth-shared for handles, file caps for I/O. Kernel and fuse clients never set the key.

**Needed by.** Option A only. Optional; measure before building.

**Size.** M.

## 7. M6 Many small sessions

**Current state.** One MDS session per VM (P5) means thousands of sessions per rank. The engine has a per-session cap count with a recall hint at `max_caps_per_client`, default 2^20 [src: crates/engine/src/engine/mod.rs:75-100,1562-1573]. Reconnect cost grows with sessions times caps. No measurement exists for session counts above a handful.

**Sketch.** A benchmark with N sessions holding caps on shared and private files: open, cap fan-out on a shared file, rank restart with reconnect, eviction. Fix what the benchmark finds. Set the per-VM cap budget low by default.

**Size.** M, mostly test and measurement.

## 8. M7 Admin eviction interface

**Current state.** The daemon has no admin socket [src: AGENTS.md, Live checks]. Eviction and blocklisting happen only inside recovery flows.

**Why.** A kernel-client proxy that cannot send uuid reclaim leaves a stale session behind when its host dies mid-migration. The session holds caps and locks until the MDS auto-closes it. An operator or the orchestrator must be able to evict a session by entity or address, with blocklist, at migration time. Operators need the same interface for every option.

**Sketch.** A local admin socket with `session ls`, `session evict <id|addr> [--blocklist]`, and the checkpoint request that SIGUSR1 triggers today. The monitor-side `ceph tell mds.*` path depends on the MDSMap decode workaround and stays out of scope.

**Size.** M.

## 9. M8 Reclaim from any session class

**Current state.** The daemon handles reclaim replies and the design requires uuid and timeout semantics for Ganesha [src: crates/mds/src/session.rs:38-40,299-304; design/minimum-feature-set.md section 4.6]. Whether acceptance depends on libcephfs-only session metadata is unverified.

**Why.** If `fs/ceph` gains uuid and timeout session metadata and `MClientReclaim`, kernel-class sessions must receive the same treatment.

**Sketch.** Verify that reclaim matching uses only uuid and entity name. Add a test with kernel-style session metadata plus uuid.

**Size.** S.

## 10. M9 Lock reservation on reclaim

**Current state.** Decision 6 in `design/minimum-feature-set.md` chose no recovery hold for absent sessions and recorded option (c), a per-inode reservation of the absent session's locks, as possible later. On migration at level 1 the source releases a VM's Ceph-level locks before the destination re-takes them, so another client can take a lock in between (the `reexport.rst` reboot-recovery limit; Option C study, R5).

**Why.** With the kernel client as the proxy's Ceph client, the MDS session is per tenant per host and no takeover of a whole session is possible (D2). A reservation keyed on the lock owner closes the window without session takeover.

**Sketch.** A reclaim request names the owners whose locks the destination will re-take within a bounded time. The MDS records a reservation per inode and owner; a conflicting lock from any other session waits or fails with EAGAIN until the reservation is consumed by the re-take or expires. The reservation is journaled with the session record, so a rank restart keeps it. Expiry equals the NFS grace period.

**Size.** M.

## 11. Ordering

1. M3 test and M6 benchmark first: they are cheap and they can change the plan.
2. M1 before the first tenant-facing release of any option, with default off.
3. M7 with the first migration experiment.
4. M9 with the first level 1 migration; M4 when level 2 for the Ceph half is scheduled, which needs the Rust CephFS client.
5. M5 only if Option A measurement shows revocation traffic from unused grants.
6. M2 at the first multi-rank or lazy-load milestone, as already planned.
