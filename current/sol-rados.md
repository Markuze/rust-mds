# sol-rados
Task: Implement and test general Ceph file striping in purge, truncate extents, object-local truncate size, and size/mtime probe mapping. Improve error propagation and expose usable async Results. Exclusive scope: crates/mds/src/rados.rs and current/sol-rados.md. No live cluster calls.
Status: Complete. All 13 owned-file tests pass. CARGO_TARGET_DIR=target-sol-rados cargo test -p rmds passes all 33 tests after concurrent owners fixed their compilation errors. Formatting and diff whitespace checks pass. No live tests or commits.
Context: Base f54d2988ebaeeb8311314fe6894ee030a80e955d. Only rados.rs and this ledger changed by this agent. C++ Striper.cc/Filer.cc/fs_types.h define mapping. Object period is OS*SC. Truncate uses constant-memory iteration and object-local prefix differences. Recovery scans all candidate objects, maps the last byte, and takes maximum size/mtime. Empty existing objects contribute mtime but size 0. Zero max_size has no candidates. Layout validation returns Rados(-22); impossible object size/mapped bound/overflow returns Rados(-5). Raw operate replies check overall and per-op negative results locally. Platform operate_checked now exists and can replace operate+check_reply during integration; no dependency on that new method was introduced here.
Next: Parent integrates these awaitable primitives into the durable job executor. Persist full inputs before execution; retain intent on error; journal retirement/recovered metadata on success; reject stale owner/version/sequence completions. Keep old layouts/pools/namespaces and snapshot-context extension inputs in the durable contract. These compatibility spawn wrappers only log and do not implement durable completion.

## Integration signatures

```rust
pub async fn purge_inode(objecter: &Objecter, ino: u64, layout: &FileLayout, size: u64)
    -> Result<(), OpError>;
pub async fn truncate_data(objecter: &Objecter, ino: u64, layout: &FileLayout,
    truncate_seq: u32, size: u64, from: u64) -> Result<(), OpError>;
pub async fn recover_file_size(objecter: &Objecter, ino: u64, layout: &FileLayout,
    max_size: u64) -> Result<Option<(u64, denc::UTime)>, OpError>;
```

`Ok(None)` means no candidate object exists (or max_size is zero). `Ok(Some((0, mtime)))` means at least one empty object exists. Probe errors propagate, including pool, timeout, fencing, and OSD failures. Purge ignores only missing objects. Truncate checks overall and per-op OSD errors. Invalid layouts use EINVAL (-22); inconsistent object sizes or mapped ends beyond the retained bound use EIO (-5). Job retries may repeat the complete purge/truncate range after partial success. Callers must fence lost writers and drain conflicting destructive work first. Existing spawn signatures are unchanged.
Log:
- [2026-10-22] Started the exclusive RADOS layout task. No cluster access or child agents.
- [2026-10-22] Baseline: CARGO_TARGET_DIR=target-sol-rados cargo test -p rmds rados::tests passed 4 tests. Added reference regressions; 3 failed before implementation (object count, striped overlap, local truncate size).
- [2026-10-22] After implementation, isolated rustc --test of rados.rs with Cargo-built dependencies passed 12 tests. Tests include fixed C++-derived vectors, independent small-layout byte mapping, sparse/empty probes, both OSD error layers, invalid layouts, and u64::MAX ranges.
- [2026-10-22] Focused Cargo test initially blocked by daemon.rs missing new SessionOutcome arms. Broad cargo test -p rmds later blocked by concurrent server.rs test errors E0609 (InodeCore.core) and E0521 (borrowed WireFilePath argument). No files outside scope were changed to bypass these errors.
- [2026-10-22] Final isolated owned-file suite: 13 passed, 0 failed. rustfmt --edition 2024 --check crates/mds/src/rados.rs and git diff --check -- crates/mds/src/rados.rs current/sol-rados.md passed.
- [2026-10-22] Final broad CARGO_TARGET_DIR=target-sol-rados cargo test -p rmds: 33 passed, 0 failed, 0 ignored. Five existing dead-code warnings remain in config.rs/server.rs/session.rs. No warning originates in rados.rs.
