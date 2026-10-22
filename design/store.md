# Rust MDS: metadata store

> Selected design, 2026-10-22. Stable guarded write-ahead log (WAL) objects and immutable checkpoints replace the rotating-segment draft. The exact first-milestone interfaces and failure rules are in [persistence-contract.md](persistence-contract.md). These are implementation requirements, not completed features.

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

The eventual shard model remains subvolume-based. The child owns its root inode; the parent holds a versioned boundary reference. Adoption needs quiescence, a crash-safe visibility switch and an explicit policy for existing cross-boundary hard links. Do not use an unbounded global inode exception list. These are G4 requirements, not first-milestone APIs.

Before G5 multi-stream work, prove conflict dependencies, aggregate delta deduplication, deletion/tombstone semantics, coherent checkpoint coverage vectors and durable routing changes. Different object names do not guarantee different placement groups. Before G6 migration, freeze registration, fence every registered stream, record final tails, and drain destructive I/O before importer service. Stream 0 remains authority; shard-map owner fields remain projections.
