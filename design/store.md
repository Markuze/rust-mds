# Rust MDS: shard model and metadata store

> Draft 0 for review, 2026-10-11. One level below [the minimum feature set](minimum-feature-set.md): how shards, the per-shard WAL and the metadata store sit in RADOS. It fixes the [day-one design hooks](minimum-feature-set.md#51-day-one-design-hooks) that touch the on-disk format. Constraints from the decision log: no C++ on-disk compatibility (D2), shard = subvolume (D3), EXDEV across shards (D4), safe replies only (decision 5), a WAL sharded below the shard, ownership epochs checked by the OSDs (decision 4), plain RADOS operations only (no object classes).

## 1. Objects at a glance

All objects live in the file system's metadata pool, in the default namespace, with the prefix `rmds.` so they never collide with mgr/snap_schedule, cephfs-mirror or ceph-csi objects (F-STORE-102).

| Object | One per | Content |
|---|---|---|
| `rmds.format` | file system | Format marker: format version, fsid, creation time. Read before anything else; a pool without it, but with other CephFS objects, is not ours (decision 2). |
| `rmds.shardmap` | file system | omap: shard id → shard record (root inode, mount point, owner rank, owner epoch, state, inode ranges). Updated with `assert_version`. |
| `rmds.inoalloc` | file system | The next free inode number; ranges are leased to shards by compare-and-swap. |
| `rmds.sessions.<rank>` | rank | omap: client id → session record (metadata incl. `uuid`, `timeout`, auth name, state). |
| `rmds.wal.<shard>.<stream>.<seg>` | WAL segment | Appended WAL records (section 3). |
| `rmds.walhead.<shard>.<stream>` | WAL stream | First live segment, checkpointed sequence number, owner epoch. |
| `rmds.inode.<shard>.<bucket>` | inode bucket | omap: inode number → inode record (section 4). |
| `rmds.dir.<shard>.<dirino>.<part>` | directory partition | omap: `(hash24, name)` → dentry (section 4). |
| `rmds.stray.<shard>` | shard | omap: inode number → stray record; doubles as the durable purge queue (section 6). |

Data-pool objects keep the client-defined names `<ino hex>.<objno 8 hex>` (F-STORE-064). The MDS writes no backtraces (F-STORE-058 is OUT).

## 2. Shards and ownership

- **Identity.** A shard has a stable 64-bit id, independent of ranks. Shard 0 is the root shard: everything outside subvolumes. A subvolume shard is created when `ceph.dir.subvolume` is set on a directory (F-SNAP-050), including when the directory is not empty: its existing subtree is adopted (section 7).
- **Shard record** in `rmds.shardmap`: root inode, parent shard and mount point (parent directory inode and name), owner rank, owner epoch, state (`active`, `adopting`, `migrating`, `deleting`), and the inode ranges leased to the shard.
- **Ownership epoch.** Every change of owner (takeover after a failure, migration, rank stop) increments the shard's owner epoch in `rmds.shardmap`. The epoch is also stored as the `rmds.owner` xattr of the stream's current WAL segment. **Every WAL append carries `cmpxattr(rmds.owner == my_epoch)`** in the same RADOS operation, so the OSDs refuse appends from a deposed owner (`-ECANCELED`) even if the monitor's blocklist has not reached them yet. This is the fencing backstop and the safety net for live migration.
- **Takeover.** The new owner waits for the OSDMap epoch carrying the failed daemon's blocklist. It increments the owner epoch in `rmds.shardmap`, sets `rmds.owner` on the last segment of each stream with `cmpxattr` on the old value (which seals it), and replays. New appends go to new segments that it creates with its own epoch.
- **Inode numbers belong to one shard forever.** Hard links and renames never cross shards (EXDEV), so an inode never moves. The single exception is the adoption of a subtree at shard creation, recorded as an exception list in the new shard's record. Any rank resolves inode → shard from the range table in `rmds.shardmap` without I/O (4.6 hot path), then shard → owner rank.

## 3. The WAL

- **Streams.** A shard has one or more streams; the count is in the shard record and adapts to load. A stream is a sequence of segment objects, each up to 4 MiB, created with an exclusive create plus `rmds.owner`. Streams of one shard land in different PGs because their object names differ.
- **Record.** Each transaction is one self-contained record in exactly one stream:

  ```text
  u32 len | u32 crc32c(body) | u64 seq (per stream) | u64 txid | body
  body = ENCODE_START(1, 1):
    u32 client (0 for internal) | u64 tid | (completed-request result: i32, created ino)
    vec<mutation>
  mutation = { object: inode(ino) | dentry(dirino, hash24, name) | stray(ino) | session-free...,
               version: u64,          // per-object version after this mutation
               op: put(record) | delete | delta(field deltas) }
  ```

  - The completed-request entry sits in the same record as the mutation (5.1 hook).
  - Inode and dentry puts carry full new values, so replay never needs the old state.
  - Directory aggregates (size, link count, rstat, mtime as a maximum) are **deltas**: commutative, so they need no lock and no replay order.
  - Mutations carry per-object versions; replay applies an object's mutations in version order, in parallel across objects and streams.
  - Records are small: only changed fields, never ancestor images.
  - Reserved for later: snapshot fields (`first`/`last` on dentries, old inode versions), ino reservations for replayed creates, async-create delegations.
- **Routing.** A transaction goes to the stream of its primary inode's hash, so all cap flushes of one file stay in one stream and FLUSH_ACKs keep the kernel's cumulative order (F-CAPS-018).
- **Commit.** Group commit per stream: one `append` op (with the owner guard) carries every record that is ready. A commit starts immediately when someone waits (any request, `FLAG_SYNC` flush, `REQUEST_FLUSH_MDLOG`); there is no timer delay, and an idle-time flush bound of about 5 s holds anyway (F-STORE-009).
- **Locks until durable.** A non-commutative object stays locked until its record is durable. A record therefore never depends on a record that might be lost, and a torn tail in one stream cannot orphan records in another.
- **End of log.** At replay the end of each stream is the first record whose length, crc or sequence number is wrong (no prezeroing, no probing).
- **Checkpoint and trim.** A background task per stream applies committed records to the store objects (section 4) and then records the checkpointed sequence number in `rmds.walhead`; whole segments below it are deleted. Store values carry their object version, so a stale replay never overwrites newer state.

## 4. The metadata store

- **Inode table.** `rmds.inode.<shard>.<bucket>` objects, bucket = hash(ino) mod the shard's bucket count, a power of two that starts at 16 and doubles by splitting. The omap key is the inode number; the value is the inode record:
  - every field clients see in InodeStat: mode, uid, gid, nlink, rdev, size, times, change_attr, version, time_warp_seq, truncation state, layout, quota, flags, xattrs (or a pointer to a separate xattr object when large), symlink target, directory stats;
  - the primary parent (directory inode and name), which serves LOOKUPPARENT, LOOKUPNAME and the absolute path needed for MDS auth-cap checks;
  - writeable ranges per client, for size recovery after a failover (F-STORE-084).
- **Directories.** Entries live in `rmds.dir.<shard>.<dirino>.<part>` objects, omap key `hash24 (8 hex digits) + name`, so key order is the readdir order the kernel requires (F-CACHE-077). The value is the dentry: inode number, type, and room for snapshot bounds. A directory starts with one partition and is split by hash range above about 100 000 keys (the OSD warns at 200 000). Partitions are internal; clients always see one frag (D5). The directory's own partition map is part of its inode record.
- **Lookup by inode number** is a direct read of the inode table, from any rank after inode → shard → owner resolution. The table is updated in the same record as the namespace change, so new files are resolvable at once (F-STORE-062).
- **Base hierarchy.** `/` is inode 1 in shard 0, with an explicit default layout (F-NS-072). No `~mdsN` directories exist.

## 5. Inode numbers

- `rmds.inoalloc` holds the next unleased number. A shard leases ranges (for example 2^20 numbers) with compare-and-swap, records the lease in its shard record, and allocates from it in memory. A crash loses at most the unused part of a lease, so numbers are never reused (F-STORE-073).
- Numbers start at `0x10000000000` and stay below 2^48 (F-PROTO-100); [0x100, 0x1000) is never used.
- Later: a session may receive a durable sub-range for kernel async creates, recorded in a WAL record and purged if unused after the session is lost (decision 5 hooks).

## 6. Strays and purge

- Unlinking the last link of an inode that clients still hold open moves it into the shard's stray table (`rmds.stray.<shard>`), in the same WAL record as the unlink. The stray keeps its prior path for auth checks. It does not count toward the shard's rstat or quota (F-BG-001/002).
- When no client holds caps anymore (and not before the shard's reconnect phase has ended), the stray record becomes a purge item. The purger deletes the data objects up to the largest size a client may have written, with `FULL_FORCE`, then removes the stray and inode records in one WAL record. Purge is idempotent, so execution is at least once after crashes (F-BG-014/015/019).
- A per-shard purger with a budget shared fairly across shards (F-BG-017, later).

## 7. Shard-root operations

These are the only operations that touch two shards: the parent shard holds the dentry, the child shard holds everything below.

- **Create a shard** (`setxattr ceph.dir.subvolume 1`, on a directory that may already hold an entry such as mgr/volumes' `<uuid>`):
  1. a WAL record in the parent shard creates the shard record (state `adopting`), and marks the directory as a shard root;
  2. the subtree's inode and dentry records are copied into the new shard's objects, and the adopted inode numbers are recorded as exceptions in the shard record;
  3. a record in the new shard marks it `active`, and the parent removes the copied records.

  Each step is idempotent and resumable from the shard state, so a crash at any point either finishes or rolls back on takeover.
- **Rename a shard root** (subvolume removal moves it into `/volumes/_deleting`): a single record in the parent shard. The mount point lives in the shard record, so the child shard is not touched.
- **Remove a shard**: once the subvolume is empty (after mgr/volumes' purge), `rmdir` of the root deletes the shard record and its objects.

## 8. Recovery

1. Wait for the OSDMap epoch that blocklists the previous holder (`last_failure_osd_epoch`).
2. Read `rmds.format`, then `rmds.shardmap`: the shards whose owner rank is this rank.
3. Per shard, in parallel: take ownership (section 2), replay every stream from its checkpoint to its end, and rebuild the in-memory state. Inodes and directories load lazily.
4. Load `rmds.sessions.<rank>`; reconnect clients; rejoin (cap re-attachment per shard).
5. Go active.

The per-shard clientreplay phase exists from the start but stays empty until early replies (decision 5).

## 9. Open points

1. Bucket and partition sizes, and when to split them.
2. xattr storage: inline up to some size, separate objects beyond.
3. Whether checkpointing writes whole inode records or field patches (whole records are simpler; patches cut write amplification for strict-I/O workloads).
4. Where the shard map lives once there are many ranks: one object, or one object per shard with a small index.
