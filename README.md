# Next-Generation CephFS MDS – Architecture Proposal

*Author: Igor Golikov*

## Goal

Redesign the MDS as a highly concurrent, multi-threaded metadata execution engine, scaling across CPU cores within a single MDS and across multiple MDS ranks.

Today, metadata processing within a rank is largely serialized by `mds_lock`, while each rank maintains a shared journal. Adding CPU cores does not necessarily increase throughput.

## Core Design Principles

- **Sharding by design** – Subtrees/dirfrags are first-class shards with independent ownership, placement and recovery. Shards are units of distribution, not necessarily locking boundaries.
- **Fine-grained concurrency** – Replace the global `mds_lock` with hierarchical transaction locking. Independent operations should run concurrently, even within the same directory.
- **Database-inspired transaction engine** – Similar to database query planning, metadata operations declare their read/write dependencies and required locks. A scheduler decides which transactions can execute concurrently.
- **Sharded journaling** – Independent journal streams allow concurrent persistence and parallel recovery. Journal ordering follows transaction dependencies rather than a global serialization point.
- **Preserve filesystem semantics** – Maintain atomic namespace operations, consistency, snapshots, quotas and client capability correctness.

## Proposed Architecture

### Shard Manager

Manages subtree/dirfrag ownership, placement, splitting, migration and recovery. Each shard has a stable identity independent of its MDS rank.

### Metadata Transaction Engine

Plans and executes metadata operations through: resolve → plan → acquire locks → validate → execute → journal → commit.

Supports asynchronous execution and dependency tracking.

### Hierarchical Lock Manager

Database-style multi-granularity locking (S/X/IS/IX) over shards, dirfrags, dentries, inodes and inode metadata fields.

Separates distributed metadata locks, logical transaction locks and short-lived in-memory latches.

### Concurrent Execution Scheduler

Worker pool executing independent metadata transactions across CPU cores, without blocking threads on lock waits or journal I/O.

### Sharded Journal Engine

Per-shard logical journals with concurrent submission, batching, dependency ordering and parallel replay.

## Example: Same-Directory Concurrency

Two clients execute:

```text
create("/project/a")
create("/project/b")
```

Both transactions can acquire intention locks on the directory and exclusive locks on different dentries, allowing concurrent execution.

Shared parent metadata (timestamps, statistics, quotas) must be handled without serializing every create. Transactional deltas and commutative updates can help where semantics permit.

## Cross-Shard Operations

Operations such as cross-shard rename require distributed transaction coordination, potentially using a two-phase-commit-style protocol.

Single-shard operations should remain independent of distributed coordination.

## Proposed Implementation Phases

### Phase 0 – Design & Baseline

Define concurrency/consistency invariants, analyze `mds_lock` dependencies and benchmark current metadata throughput, CPU utilization, journaling and recovery.

### Phase 1 – Shard-Based Multi-Threading

Introduce the shard manager and worker pool. Enable parallel execution across independent shards, initially retaining coarse locking within a shard.

**Target:** throughput scales with CPU cores across independent subtrees.

### Phase 2 – Fine-Grained Transactions

Implement the transaction planner, hierarchical locking, concurrent metadata cache and optimistic validation.

**Target:** independent creates/unlinks in the same directory execute concurrently.

### Phase 3 – Sharded Journaling

Introduce independent journal streams, concurrent commits, transaction dependency ordering and parallel replay.

**Target:** remove rank-wide journal bottlenecks and improve recovery scalability.

### Phase 4 – Cross-Shard Transactions & Migration

Implement atomic cross-shard operations, ownership epochs, fencing, dynamic shard migration and shard-level recovery.

### Phase 5 – Hardening & Optimization

Stress-test snapshots, quotas, caps, failovers and thousands of clients. Optimize lock contention, journal batching, scheduler fairness and tail latency.

## Success Metrics

- Metadata throughput vs. CPU cores and MDS ranks
- Same-directory create/unlink scalability
- Lock contention and transaction wait time
- Journal throughput and commit latency
- p99 metadata latency
- Parallel recovery and shard migration time

## Key Architectural Shift

Decouple three concerns that are currently closely tied together:

> **Metadata ownership ≠ Concurrency boundary ≠ Journal boundary**

The ultimate objective is to serialize only metadata operations that genuinely conflict, rather than all operations executing on the same MDS rank or subtree.

---

## Repository contents

| Path | Content |
|---|---|
| [`mds-rust-map/`](mds-rust-map/README.md) | Reference set, facts only: a map of the C++ MDS and its peers, and a catalog of 935 features. |
| [`design/minimum-feature-set.md`](design/minimum-feature-set.md) | The minimum feature set of the Rust MDS, its working assumptions and the decision log. |
| [`design/minimum-feature-set-catalog.md`](design/minimum-feature-set-catalog.md) | Every catalog entry with its tier and reason, by area (generated from [`minimum-feature-set.json`](design/minimum-feature-set.json)). |
| [`design/workplan.md`](design/workplan.md) | The workplan: stages, gates, workstreams and risks (draft for discussion). |
| [`design/store.md`](design/store.md) | Shard model and metadata store in RADOS: WAL, inode and directory objects, ownership epochs, recovery (draft for review). |
