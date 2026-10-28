# Rust MDS: transaction isolation and visibility

> Draft 0, 2026-10-28. Normative for the G5 metadata execution model. Companion of [D3–D5](minimum-feature-set.md#3-working-assumptions) and [the store](store.md). G1–G3 keep one root shard, one stream, full images and a scalar sequence (`store.md` sections 1–5, `persistence-contract.md`). This document does not replace that store, and it does not authorize inode-range ownership. The compatibility engine is still one [`Engine`](../crates/engine/src/lib.rs) per shard. Every entry point takes `&mut self`, and [`Txn`](../crates/engine/src/engine/mod.rs) records touched inodes and images. That shape is not this model. Do not grow the G1–G3 scalar sequence into the multi-stream WAL.

## 1. Terms

| Term | Meaning |
|---|---|
| Metadata transaction | One client operation or one internal step (purge, split, migration) that reads and writes metadata atomically. |
| Publish | The moment a transaction's writes become visible to other transactions and to clients. |
| Commit token | `(JournalStreamId, seq)`. Sequence `N` durable on a stream means every lower sequence of that same stream is durable. It says nothing about another stream. |
| Conflicting capability | A client capability that lets the holder use metadata state this transaction changes. |

A database lock does not replace a capability revocation. Both are required. Capability messages stay ordered per `(client, inode)` in the per-session send queue (day-one hook, decision 7).

## 2. Transaction record

The planner builds this record before it mutates the namespace. The record is the unit of scheduling, locking, journaling and replay.

```text
MetadataTxn
  id:            TxnId
  participants:  Vec<ShardId>          // one entry for the common case
  read_set:      Vec<ReadDependency>   // object, version, what was read
  write_set:     Vec<WriteIntent>      // object, field group, exclusive or commutative
  locks:         Vec<LockRequest>      // resource, mode S / X / IS / IX
  mutations:     Vec<MetadataDelta>    // put, delete, or commutative field delta
  dependencies:  Vec<CommitToken>      // other durable facts this commit waits on
```

`ReadDependency` names the object version that validation must still find at journal submit. `MetadataDelta` names its class. Replay uses the class. It does not infer commutativity from the field name after the fact.

The engine's current `Txn` has none of the read set, write set, lock plan or dependency list. It mutates memory and then journals the resulting images. New storage code consumes `MetadataTxn`.

## 3. Pipeline

```text
resolve → plan → acquire locks → revoke conflicting caps → validate →
execute → journal → durable completion → publish → reply
```

A worker never blocks on a lock wait or on journal I/O. A transaction that must wait is parked. Another transaction runs on that worker.

`resolve` names the shards from the directory anchors and the inode location (store section 6). `plan` fills the record in section 2. A cross-subvolume link or rename returns `EXDEV` here, before locks (D4).

## 4. Isolation

Inside one shard, transactions whose write intents conflict are **commit-order serializable**.

1. A transaction reads a committed version, or its own write. It does not read another transaction's unpublished write.
2. Validation runs after locks are held and before journal submit. Every read-set version must still be the committed version. Otherwise the transaction restarts.
3. Two transactions with conflicting exclusive write intents do not both commit. Section 5 defines the conflicts.
4. Publish is one point per shard. It runs only after this transaction's commit token and every dependency token are durable.
5. P0 replies are safe (decision 5). The reply is part of publish. There is no early reply.

A transaction with one participant does not join a distributed protocol and does not wait for one.

A transaction with several participants uses two-phase commit (store section 7). No participant publishes until every participant has a durable prepare. Clients never observe one side of a cross-shard rename or link.

`readdir` takes a snapshot of committed directory state at plan time. A create that publishes after that snapshot is absent from that call and present in a later call. Two creates of different names are serializable with each other: both names exist after both publish, and neither create observes a torn parent statistic (section 6).

## 5. Locks

Lock order is coarse to fine. A transaction takes locks in this order and releases them after publish:

```text
shard → directory → dentry or inode → field group
```

Modes are shared (`S`), exclusive (`X`), intention-shared (`IS`) and intention-exclusive (`IX`).

| Operation | Locks |
|---|---|
| Create or unlink of one name | `IX` on the shard and the directory, `X` on that dentry |
| Two creates of different names | `IX` on the same directory, `X` on different dentries. The locks do not conflict |
| Setattr of an inode field group | `IX` on ancestors, `X` on that field group |
| Commutative statistic update | `IX` on the directory, a commutative write intent on the statistic. Not `X` |
| Lookup or getattr | `IS` / `S` on the objects read |

Field groups:

| Group | Examples | Write intent |
|---|---|---|
| Namespace | dentries, primary name, nlink when a link is added or removed | exclusive |
| Inode body | mode, owner, size, layout, xattrs, non-max timestamps | exclusive |
| Commutative statistics | `rbytes`, `rfiles`, `rsubdirs`, `rctime` as a maximum, directory mtime as a maximum | commutative delta |

Directory `nlink` on the wire stays 1 (client invariant). That field is not a concurrency counter.

## 6. Visibility

| Observer | Sees the write when |
|---|---|
| Issuing client | The reply is published. The reply carries the new state (read-your-writes). |
| Other metadata transactions | Publish, after durable completion. |
| Other clients | Publish, and only after conflicting capabilities were revoked or the holder was evicted. |
| A reader of an ancestor statistic | The same publish as the namespace mutation that produced the delta. |

Rules:

- One transaction does not publish a new dentry without its statistic delta, or a statistic delta without its dentry. If the dentry and the directory inode are in different shards, the create is cross-shard so those two publishes are one commit.
- Commutative deltas from concurrent transactions both apply. The merge is commutative, associative and idempotent under the replay identity of section 8.
- Two exclusive writers of the same field group: one commits, the other restarts.
- A lost durable delta is a bug. An acknowledged transaction's deltas are applied on replay, or the acknowledgement was not sent.
- A double-applied delta is a bug.

## 7. Capabilities

Order for a transaction that changes client-visible state:

1. Acquire locks.
2. Revoke conflicting capabilities. Park until the acknowledgements arrive, or until the holder is evicted.
3. Validate the read set.
4. Execute.
5. Journal.
6. On durable completion: publish, grant new capabilities, send the reply.

Concurrent transactions that modify one directory may share one revocation generation. None of them publishes while a conflicting capability is still outstanding. The concurrency proof (workplan G5) includes this wait. Overlap that exists only in the network or in journal completion does not satisfy the prototype.

Session state and the capability table are not inside the shard's mutable engine. A transaction asks the session service to revoke. The service assigns `push_seq` at send time.

## 8. Journal completion and replay

Submit of a record returns a commit token for the stream that stores it. `completed(token)` becomes true when that stream is durable through that sequence. Publish waits for the transaction token and for `dependencies`. Completion on stream A does not complete stream B.

Within one stream, a record depends only on earlier sequences of that stream and on tokens listed in `dependencies`. A non-commutative object stays locked until its token is durable, so a torn tail cannot force another stream to drop a published record.

Replay rules:

| Delta class | Replay |
|---|---|
| Put or delete | Apply by object version. A lower version never overwrites a higher one. The same `(object, version)` is applied once. |
| Commutative delta | Apply once, keyed by `(TxnId, object, field)`. Order among commutative deltas of one field does not change the result. A second apply of the same key is a no-op, not a second addition. |
| Anything else | Not commutative. Replay uses object version order and the exclusive lock rule above. |

`mtime` as a maximum is commutative. `nlink`, rename, mode and a raw assignment of `rstat` are not. A G5 record stores the delta, not an ancestor image. G1–G3 still store full images, including ancestors (`store.md` section 2).

## 9. What the prototype must show

Gate G5 in the [workplan](workplan.md):

- One shard, one hot directory, `create()` of distinct names, 1 to 16 workers.
- The metadata mutations overlap. The test fails if the engine still runs every create under one `&mut` critical section and only the waits overlap.
- Parent statistics, capability revocation, object versions and commit order are inside the test.
- After replay: unique names, unique inode numbers, `rstat` equal to the created set, no lost and no double-applied delta.
- First run against a mock journal that completes tokens out of order across the rules of section 8. Repeat later against the RADOS WAL.

## 10. Out of scope here

- Snapshot isolation across snapshot ids. Snapshots stay P1.
- A lock-free fast path. Locks are required. The read set still exists, so a later fast path can validate without a format change.
- The C++ peer-request and `up:resolve` machinery. Cross-shard commit is the protocol in store section 7.
