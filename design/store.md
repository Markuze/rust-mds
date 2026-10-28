# Rust MDS: metadata store

> Selected design, 2026-10-22. Stable guarded write-ahead log (WAL) objects and immutable checkpoints replace the rotating-segment draft. The exact first-milestone interfaces and failure rules are in [persistence-contract.md](persistence-contract.md). These are implementation requirements, not completed features. The 2026-10-28 identity revision is in sections 6 and 7. A shard is not a subvolume, and an inode number is not ownership. Sections 1–5 stay the G1–G3 object roles.

2026-10-25 extension: [recovery-contract.md](recovery-contract.md) has scoped E2/D6/wire-REPLAY acceptance. Its ordinary E1 clean-session format transition is independently accepted as a contract: sections 3.2–3.4 and engine/configuration joins in [current/astra-recovery-contract-review.md](../current/astra-recovery-contract-review.md), and sections 3.5–3.8 in [current/astra-retry-upgrade-review.md](../current/astra-retry-upgrade-review.md). The transition preserves these object roles and original bootstrap bytes. It adds required authority semantics 3, a resumable Upgrading phase and one new immutable checkpoint. This contract acceptance is separate from any offline implementation acceptance. It is not live-upgrade authorization or accepted schema-3 evidence.

## 1. Object roles and authority

All metadata objects use the `rmds.` prefix in the filesystem metadata pool's default namespace. Do not modify shared snap_schedule, cephfs-mirror or ceph-csi objects. Bootstrap must validate an explicit allowlist, not merely accept any non-Rust object.

| Object | Role |
|---|---|
| `rmds.format` | Format/features, cluster fsid, filesystem id, metadata-pool identity, bootstrap id and Preparing/Ready state. |
| `rmds.shardmap` | Discovery/routing projection. Never an independent write grant. |
| `rmds.wal.<shard>.0` | Stable stream-0 object and authoritative shard control: owner token, registry (only stream 0 initially), durable tail and checkpoint descriptor. |
| `rmds.wal.<shard>.<stream>` | Later registered streams, each with its checked token and tail. Not required in G1-G3. |
| `rmds.checkpoint.<shard>.<epoch>.<generation>.*` | Immutable materialized engine state and manifest. |
| `rmds.inoalloc` | Global range reservations through compare-and-swap (CAS). |
| `rmds.sessions.<rank>` | Durable rank session identity/lifecycle, guarded by rank incarnation. |

Client data objects retain `<ino hex>.<objno 8 hex>` naming. No backtraces or C++ metadata formats are written.

## 2. First implementation: one root shard and one stream

Shard 0 stays on rank 0. The assigned MDSMap holder validates prior-owner fencing, then acquires the stable control object through CAS. The owner token includes shard epoch, rank, MDSMap incarnation and daemon gid. The control object is the sole ownership authority. Rebuild a stale shard-map projection from that control state, never the reverse.

Control metadata persists the current writer's beaconed address vector/nonce and predecessor fencing evidence. Read-only inspection discovers this metadata before acquisition. Fence every current writer found in format/shard/session controls; latest-map confirmation alone, or failure epoch zero, is not proof. Acquire shard and rank-session guards separately under each object's own CAS; no service or state mutations start until both succeed. A replacement resumes partial acquisition without resetting histories or incarnation counters.

Use full inode images, ordered dentry updates and a scalar sequence. Keep ancestor images initially. This temporarily defers the eventual multi-stream delta design. A short global execution pause until each transaction commits is acceptable. Output must never read future uncommitted state. Failure stops reads as well as writes until durable reload.

`up:creating` runs idempotent Preparing/Ready bootstrap. Persist allocator reservation, root state, control, session object and routing projection before Ready. Starting/replay requires an existing compatible Ready format. Missing or foreign state is an error, not permission to create an empty namespace.

Platform must supply bounded native `Objecter::list_pool_page` across all namespaces; omap paging cannot list pool objects. Restart the scan on map-layout change and fail on unsupported listing. The initial shared-object allowlist is empty: bootstrap supports a fresh pool; resume accepts only exact objects from its durable plan. Persist Preparing's bootstrap id, root seed, range and immutable object names before dependencies. Take over Preparing with the format object's own writer/revision guard after fencing. Ready publication also compares that format guard, not the WAL guard. See the contract for acquisition/reconciliation signatures and crash ordering.

## 3. Atomic bounded WAL batches

Use omap keys `r/<16 lower-case hex seq>`. Each value contains one complete versioned transaction. Checksum the identity, framing, sequence, owner generation and body. Keep completed requests and flush history in the mutation transaction, scoped by durable session incarnation.

One compound RADOS operation compares owner and expected tail, writes a bounded batch, and advances tail plus last-batch identity. Check transport, overall and suboperation results. Only then report commit. Keep `checkpointed <= durable <= submitted`. Start immediately when the first waiter is ready; group work already queued without a timer-only delay. One batch is in flight per stream.

Unknown outcomes freeze admission. Reconcile owner, tail and batch identity; retry identical bytes and identity only. Never allocate a replacement sequence. Missing or invalid records below durable tail are corruption. The existing segment codec's valid-prefix behavior is not the production recovery rule.

Bound record size, queue count/bytes, in-flight work, WAL bytes/keys and checkpoint backlog. Admission pressure must stop memory and omap growth. Control events retain reserved capacity.

## 4. Immutable checkpoint publication and trim

Pause admission and drain to sequence L. Export a coherent materialized engine state with no speculative images. Include inode cores, ordered directory entries, primary and remote linkage, stray authorization paths, allocator state, retry/flush histories and watermarks, snap realms, writable ranges and lifecycle jobs. A checkpoint is not a saved list of WAL records.

Write immutable objects and a manifest with identity, coverage, versions, lengths and checksums. Use exclusive create or verify identical retry content. Publish its descriptor in the stable control object under owner and expected-generation guards. Trim only covered WAL keys, in bounded operations that compare owner and published descriptor. A publication/trim crash leaves redundant history, not lost state.

Recovery selects the published checkpoint and replays exactly its uncovered durable history. Missing published objects are damage. Unpublished objects are orphans. Initially retain old checkpoint generations. Apply backlog pressure. Before garbage collection, add guarded irrevocable retirement records; stale listings never authorize deletion. Do not write mutable checkpoint inode/dir omaps.

## 5. Allocator, sessions and lifecycle

Reserve inode ranges globally before publishing them in the shard WAL. Lost reservations waste numbers; they never permit reuse. Numbers stay below 2^48 and outside [0x100, 0x1000). Preserve the durable delegation hook.

Rank session records own identity, features, metadata, authenticated context, uuid, timeout and Open/Closing/Closed state. Shard records own retry/flush histories. Persist rank Open before OPEN reply or shard work. Persist Closing before shard cleanup, then Closed after cleanup commits. Recovery resumes interrupted close. Use durable session incarnations; never let a reused client id inherit old history. See the contract for exact ordering.

The engine production path uses `step(input, permit)` with at most one record per step; close/release/tick/commit-triggered work retains explicit internal continuations. Commit callbacks never submit. Mark rank Closed only after the terminal `SessionCleanupCommitted` receipt, not after a helper returns or runtime caps disappear. Session installation preserves an identical restored key and its histories. Unknown/mismatched keys fail recovery.

Persist `stray_auth_path` and `stable_subvolume_id` in `InodeCore` and full WAL images, not only checkpoints. Last unlink and rename-over-target journal the pre-operation primary authorization path with dentry removal. A retained stray keeps that path, and a nonzero stable identity never changes. Checkpoint-at-L/unlink-at-L+1 recovery must verify both fields.

Persist full purge/truncate/size-recovery intent before execution. Retain it until completion commits. Include layouts, old pools/namespaces, maximum extent, truncate sequence, and recovery version checks. Resume jobs only after verified fencing and reconnect. Purge waits for cap release. Data-object operations need address fencing; metadata owner guards alone do not fence them.

## 6. Recovery and later gates

Completion events drive fence wait, ownership acquisition, checkpoint/WAL load, sessions, reconnect, rejoin, the empty clientreplay hook and serving. No phase substitutes an empty namespace. A cold standby never acquires write ownership before promotion.

The 2026-10-28 identity decision replaces the subvolume-as-shard target. G1–G3 stay one root shard and one stream. They do not create a shard for a subvolume, and they do not treat an inode number as ownership.

| Concept | Responsibility |
|---|---|
| `SubvolumeId` | Tenant boundary. The inode number of the directory flagged `ceph.dir.subvolume`, or 0. Quotas, snapshot realm, auth and `subvolume_id`. |
| `ShardId` | Metadata ownership, placement and recovery. Not a subvolume, and not a function of an inode number. |
| `InodeId` | Stable object identity. Never reused. Never rewritten because a subtree moved. |
| `OwnerEpoch` | Fencing generation of one shard. The stream-0 control object remains the authority. |
| `JournalStreamId` | One logical persistence stream of one shard. G1–G3 register stream 0 only. |

A later shard owns a subtree, or a hash range of one directory. A subvolume may contain many shards. Setting `ceph.dir.subvolume` records `SubvolumeId` and a snap realm on the directory's current shard. It does not create a shard and does not copy a subtree. Clients still see one frag.

An allocation lease is a home hint. It does not own the inodes it issued. The home record stores the authoritative inode or one forward `{ShardId, OwnerEpoch}`. Lookup follows that one forward. A later move updates the same forward. A hard link does not move the inode. `EXDEV` applies when `SubvolumeId` differs. Link and rename across shards inside one subvolume are atomic and do not change inode numbers. Do not use an unbounded global inode exception list.

The child shard owns inodes inside its anchor. The parent keeps a boundary copy for lookup and getattr. These are G4 requirements, not first-milestone APIs.

Before G5 multi-stream work, prove the rules in [transactions.md](transactions.md): conflict dependencies, aggregate delta deduplication, deletion and tombstone semantics, coherent checkpoint coverage vectors and durable routing changes. Different object names do not guarantee different placement groups. Before G6 migration, freeze registration, fence every registered stream, record final tails, and drain destructive I/O before importer service. Stream 0 remains authority; shard-map owner fields remain projections.

## 7. Cross-shard namespace, after G3

This protocol is not a G1–G3 object layout. A single-shard operation does not write a prepare record. The C++ peer-request machinery and `up:resolve` stay out.

Participants each append to their own stream. The coordinator is the destination shard, the shard that owns the surviving name. An ownership move also includes the home shard. The home shard is the only writer of the forward.

1. **Prepare.** Each participant appends a prepare record with the images it will publish. A new inode copy is `incoming` and is invisible to lookup. This step is durable before the next step.
2. **Commit.** After every prepare token is durable, each participant appends a commit record and then publishes. The home forward is installed in this step. The reply waits until every participant's commit token is durable.
3. **Drop the old copy.** The previous owner then deletes the record it no longer serves. If the home key is that record, the commit replaces it with the forward. Recovery deletes a stale copy when the home forward names another shard. Recovery deletes an `incoming` record that has no commit on any participant, and finishes a transaction that has a commit on any participant.

A create whose dentry and directory inode are in different shards uses the same protocol. The dentry put and the directory's statistic delta publish together. Neither is visible alone.

Split and merge use this prepare, commit and drop sequence. They do not renumber inodes, and they do not change client-visible paths. Live migration moves a whole shard to another rank and increments `OwnerEpoch`. `ShardId` does not change, so the home forward stays valid.
