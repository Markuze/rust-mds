# rust-mds: Repository Guide

A CephFS metadata server (MDS) written in Rust from scratch (no librados, no C++ Ceph libraries), targeting the unmodified Linux kernel client, ceph-fuse/libcephfs and Ceph 20.2 monitors. Read `HANDOVER.md` first (current status, what is done, next steps in order), then `design/build-brief.md` (conventions the build follows) and `design/minimum-feature-set.md` (requirements, decision log 8.1, client invariants section 7, day-one hooks 5.1).

## Commands

```sh
cargo test --workspace                 # everything (verified 2026-10-12: 328 pass, 0 fail, 2 ignored)
cargo test -p <crate>                  # one crate: denc | auth | msgr | monc | proto | engine | osdc | rmds
cargo build -p rmds                    # the daemon binary: target/debug/ceph-rmds
cargo fmt --all                        # rustfmt, default settings (no rustfmt.toml)
cargo clippy --workspace --all-targets # clean except dead-code warnings in the mds stub
```

- With several agents working at once, give each its own `CARGO_TARGET_DIR` (e.g. `target-<name>`; the pattern `target-*` is git-ignored) to avoid the build-dir lock.
- Tests that need the dev cluster or its keys skip with a note when absent. Offline `cargo test --workspace` fully passes.

### Live checks (need the dev cluster)

```sh
devcluster/start.sh --mds none     # create/start 1 mon, 1 mgr, 3 OSDs, no MDS (idempotent)
source devcluster/env.sh           # CEPH_CONF, CEPH_KEYRING, RMDS_KEYRING, MON_V1/MON_V2, FSID, ...
cargo run -p osdc --example rados_live    # live RADOS I/O as mds.rust; prints PASS
./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring $RMDS_KEYRING &   # run the daemon
scripts/smoke.sh [--keep] [--release]     # end-to-end gate: rank 0 active, kernel+fuse mounts, I/O
```

- Between daemon runs use `devcluster/newfs.sh`. After any MDS held rank 0, the next one gets `up:replay`. With no persistence yet, it then serves an empty namespace.
- Other scripts: `stop.sh` (keeps data), `reset.sh` (new cluster, new keys), `status.sh`, `mount-kernel.sh <dir> [v1|v2]`, `mount-fuse.sh <dir>`, `umount.sh <dir>`, `smoke-test.sh <mnt>`, `ganesha.sh start|stop`. `devcluster/README.md` documents every path, port and option.
- Daemon logging goes through the `RMDS_LOG` env var (e.g. `RMDS_LOG=debug`). The daemon has no admin socket yet.

## Test layers and golden data

- Tests are inline unit tests plus per-crate `crates/*/tests/`. Wire code uses byte-level expectations. Prefer golden bytes from real Ceph (`testdata/`, `ceph-dencoder`) over hand-made fixtures.
- `testdata/` holds real captures (kernel/fuse/boot traffic against the C++ MDS), maps and `dencoder-types.txt`, with regeneration scripts (`capture.sh`, `dump-maps.sh`, `msgr1_split.py`). See `testdata/README.md`.
- Some golden tests shell out to `/usr/bin/ceph-dencoder`. Three tests decrypt captured cephx exchanges with keys from the git-ignored `.env` at the repo root (`cp .env.example .env` and fill it in). These tests skip when a value is missing. Never commit keys.
- `crates/osdc/testdata/` fixtures each carry their own `gen_*.sh` regeneration script. `crates/proto/tests/common/` has a minimal pcap/TCP-reassembly reader.
- `crates/engine` includes a randomized multi-client stress test that recomputes statistics and checks journal replay.

## Architecture

Cargo workspace (edition 2024, Rust 1.99, resolver 3), all crates under `crates/`:

| Crate | Role |
|---|---|
| `denc` | Ceph wire encoding: `Encode`/`Decode` traits, versioned-struct framing, `EntityAddr`, `EntityName`, feature bits, message envelope. `src/features.rs` is **generated** by `tools/gen_features.py` from ceph's `ceph_features.h`. Do not edit it by hand |
| `auth` | cephx: keyrings, monitor handshake, service tickets, authorizers, rotating secrets, msgr1 message signatures |
| `msgr` | tokio messenger, msgr1 client and server only. msgr2 sits behind internal frame traits |
| `monc` | MonClient (hunting, auth, ticket renewal, subscriptions), MDSMap decoding, beacons, `CephConf`. `examples/probe.rs` is a minimal daemon |
| `proto` | all `MClient*` wire messages. Byte-identical decode/re-encode of captured traffic, ceph-dencoder cross-checks, fuzzed |
| `engine` | in-memory metadata engine (namespace, POSIX, caps, locks, sessions, journal). **No dependency on the wire crates** |
| `osdc` | OSDMap, CRUSH, MOSDOp, Objecter for the MDS's own RADOS I/O |
| `mds` (package `rmds`, bin `ceph-rmds`) | the daemon: `config.rs`, `daemon.rs` (membership, state machine), `session.rs` (per-session send funnel), `server.rs` (**stub**: answers -ENOSYS) |

Data flow: client message → `mds` decodes it to `engine::Request` → `Engine::handle_request` returns `Vec<Effect>` → the daemon translates effects back to `proto` messages → sent through `Session::send` in the returned order. The engine API is documented at `crates/engine/src/lib.rs`. The daemon translation step is the current work item (`HANDOVER.md` section 5.1).

Engine calling model: one `Engine` per shard, `&mut self`, never blocks, never does I/O. Every mutation becomes one self-contained `JournalRecord`. Replies are released only after commit (safe replies only). Effects include replies, cap messages, realm splits, recalls, and daemon work (purge, truncate, evict).

Reference material for exact C++ behavior: `mds-rust-map/` (facts-only map of the C++ MDS and a 935-entry feature catalog; start at its README) and the Ceph `main` source at `/home/ubuntu/ceph`. The Linux kernel client tree is checked out at `/mnt/workspace/ceph-client` (7.3.0-rc6); the dev machine runs `7.0.0-1012-ibm`, which is older. The differences are bug fixes and MDS-irrelevant features only; the wire protocol is the same, so either tree serves as reference.

## Conventions specific to this repository

- Doc comments state the wire format implemented and cite the C++ file and symbol followed.
- New decisions go into the decision log (`design/minimum-feature-set.md` section 8.1).
- Day-one hooks must stay intact (design 5.1): connection reading separate from execution; every client message through the per-session funnel; self-contained versioned WAL records; the clientreplay phase exists; the allocator can delegate inode ranges; shard ownership epochs are checked on every WAL append.
- Client invariants (design section 7) must never be violated. Violations crash or hang unmodified clients (a forward for an answered tid, a wrong dentry-trace parent, directory `nlink` other than 1, trailing bytes in replies). Check `MClientReply::check_invariants_for(&req)` before sending.
- **License caution:** the kernel's `net/ceph` and `fs/ceph` are GPL-2.0. You may read kernel code to understand client behavior. Never translate it into this codebase. The C++ Ceph sources (LGPL) and `mds-rust-map/` are the references.
- Generated files (do not hand-edit): `crates/denc/src/features.rs`, `design/minimum-feature-set-catalog.md` and its JSON.
- Run `cargo fmt --all` before committing. The tree is not currently fmt-clean everywhere.

## Gotchas

- Ceph 20.2.0's `MDSMap::decode` crashes outside a daemon context (`g_ceph_context` is NULL). This breaks `ceph tell mds.*`, Ganesha FSAL_CEPH and ceph-dencoder's MDSMap/FSMap types. Workaround: `devcluster/gctx-shim.c` through `$GCTX_SHIM`/`LD_PRELOAD` (env.sh exports it). Use the JSON dumps in `testdata/maps/` as the decoded MDSMap reference instead.
- Dev cluster OSDs use **memstore**. A killed or crashed OSD loses its data. Stop the cluster only with `stop.sh`. Otherwise run `reset.sh` and `start.sh`. `osd_fast_shutdown = false` is already set for clean stops.
- `HEALTH_ERR 1 filesystem is offline` with no MDS running is expected.
- tshark and tcpdump cannot read files under `/mnt/workspace` (AppArmor). Copy pcaps to `/tmp` first.
- Every `sudo` call prints a harmless smartctl warning. It comes from ceph-base's sudoers drop-in.
- The distro `nfs-ganesha.service` keeps port 2049. `devcluster/ganesha.sh` runs its own instance on port 12049.
- ceph-fuse always talks msgr2 to the monitor, but msgr1 to an MDS that publishes a v1 address only. The kernel client uses v1 unless mounted with `ms_mode=`. The Rust MDS binds msgr1 only (decision 4). Free ports for it are 7402-7499.
- The engine has no clock. The daemon must fill a zero request stamp.
- The code sends newer reply versions than C++ 20.2 does (InodeStat v9, caps v14, SnapRealmInfoNew v2). Both 20.2 and main clients accept them.
- First-time machine setup (Ubuntu 26.04): `sudo apt-get install -y ceph-mon ceph-mgr ceph-osd ceph-mds ceph-fuse ceph-common ceph-base gcc`. The gcc build makes the gctx shim. Optional: `tcpdump tshark nfs-ganesha nfs-ganesha-ceph`.

---

# Generic Systems Agent Operating Guide

Canonical rules for AI coding agents working on open-source systems software.
This guide applies to the Linux kernel, Ceph, and low-level C, C++, and Rust projects.

---

## 0. Operating Philosophy & Writing Style

### Tradeoff
Bias toward caution over speed. Correctness and stability take priority over velocity.

### Writing Style (ASD-STE100)
All agent output must follow ASD-STE100 (Simplified Technical English) principles:
- Write in the active voice.
- Keep sentences short. Express one idea per sentence.
- Use consistent terminology. Do not use synonyms for technical terms.
- Define domain terms on first use.
- Remove filler words.
- Preserve technical precision. State numbers, types, and invariants explicitly.
- Apply this style to documentation, code comments, commit messages, status reports, and replies.

---

## 1. Identity & State Ledger

Maintain a task ledger in `current/<task_name>.md`.
The ledger provides a machine-portable state checkpoint.
If the agent session stops, another agent can resume work from this file.

### Startup Sequence
1. Check if `current/<task_name>.md` exists.
2. If the ledger exists, read it first. Resume from the `Status` and `Next` fields.
3. If the ledger does not exist, create it before making any edits.

### Ledger Format
```markdown
# <task_name>
Task: Verbatim description of requested work.
Status: Current status. List verified changes and in-flight tasks.
Context: Touched files, key technical decisions, and rejected approaches.
Next: Immediate next step if execution stops now.
Log:
- [YYYY-MM-DD HH:MM] Checkpoint entry.
```

Update the ledger at meaningful milestones: after planning, after running tests, and after finishing the task.

---

## 2. Subsystem Orientation & Modularity

- Read relevant subsystem documentation before you explore the source tree.
- Do not inspect directory trees without an explicit search target.
- Identify module boundaries, public interfaces, and private subsystem internals.
- Maintain modularity. Do not expose internal structures across subsystem boundaries.
- Keep components loosely coupled and cohesive.

---

## 3. Think Before Coding

Do not make assumptions. Surface tradeoffs explicitly.

Before you write code:
- State your assumptions to the user. Ask questions if requirements are ambiguous.
- If multiple designs exist, present the tradeoffs. Do not choose in silence.
- If a simpler design exists, explain it. Propose the simpler design.
- If code behavior is unclear, stop. Request clarification before starting work.

---

## 4. Simplicity First

Write the minimum amount of code to solve the problem.

- Implement only requested features.
- Do not build speculative abstractions or generic wrappers for single-use logic.
- Do not add unused configuration options.
- Do not write defensive error handling for conditions that are mathematically impossible.
- Keep implementations compact and direct.

---

## 5. Surgical Changes & Upstream Etiquette

Touch only the lines required for the task.

- Do not reformat adjacent lines, comments, or whitespace.
- Do not refactor existing working code unless requested.
- Match existing repository indentation, naming conventions, and idioms.
- Remove only dead code and unused imports that your own changes introduced.
- Ensure that every modified line traces directly to the user request.

---

## 6. Goal-Driven Execution

Convert every task into verifiable technical goals.

- Transform bug reports into reproduction tests. Make the tests pass.
- Transform feature requests into unit or integration tests.
- Execute tests before and after modifying code.
- Follow a linear execution plan:
  1. Define the step.
  2. Implement the change.
  3. Verify with compilers, tests, or linters.

---

## 7. Concurrency, Synchronization & Memory Models

Systems software requires explicit concurrency controls.

### 7.1 Lock Hierarchies
- Define and document a strict global lock acquisition order.
- Acquire nested locks in order to prevent deadlocks.
- Release locks in reverse acquisition order.
- Never hold a lock during long-running I/O operations or network calls.
- Use lock annotations (such as Linux kernel `lockdep`) to detect inversions.

### 7.2 Read-Copy-Update (RCU) & Hazard Pointers
- Use RCU for read-heavy shared data structures.
- Keep RCU read-side critical sections short.
- Never sleep, block, or yield inside an RCU read-side critical section.
- Defer memory reclamation until all readers exit grace periods (`call_rcu`, `synchronize_rcu`).

### 7.3 Memory Ordering & Atomics
- Avoid data races. Access shared mutable memory through atomic primitives.
- Choose explicit memory orderings:
  - Use `relaxed` only for independent counters.
  - Use `acquire` and `release` to synchronize data across threads.
  - Use `seq_cst` (sequentially consistent) only when total global ordering is required.
- Document why the selected memory ordering is safe.

### 7.4 Thread-Per-Core & Reactor Architectures (e.g., Ceph Crimson / Seastar)
- Reactor threads must never block.
- Never invoke blocking system calls (`read`, `write`, `sleep`) on reactor threads.
- Never use blocking mutexes (`std::mutex`, `pthread_mutex_t`) inside reactor tasks.
- Chain asynchronous operations using non-blocking futures, promises, and continuation callbacks.
- Partition mutable state across CPU cores to avoid cross-core lock contention.

---

## 8. Distributed Systems & Fault Tolerance (e.g., Ceph)

Distributed storage engines must handle partial failure modes.

### 8.1 Failure Domains & Partitions
- Assume nodes, networks, and disks fail independently.
- Detect network partitions and prevent split-brain states.
- Bound timeout durations. Do not wait indefinitely for remote peer responses.

### 8.2 Idempotency & Message Handling
- RPC (Remote Procedure Call) handlers must be idempotent.
- Design protocols to handle duplicate, delayed, or out-of-order network messages.
- Discard obsolete messages safely.

### 8.3 Epochs, Generations & Leases
- Check cluster configuration epochs (e.g., Ceph `OSDMap` epoch) on every client request.
- Drop or route messages that contain stale epoch numbers.
- Verify lease validity before performing authoritative writes or metadata updates.

### 8.4 Wire Serialization & Backward Compatibility
- Maintain binary backward and forward compatibility for on-wire structures.
- Support rolling upgrades. Nodes running older protocol versions must parse messages safely.
- Never alter field order or delete existing fields in serialized buffers without version gates.

### 8.5 Crash Consistency
- Maintain write-ahead logs (WAL) or atomic transaction journals for storage metadata.
- Never leave persistent storage in an unrecoverable state after power loss.
- Flush disk write caches (`fsync`, `fdatasync`, or NVMe flush commands) at transaction boundaries.

---

## 9. Language-Specific Engineering Rules

### 9.1 C (Linux Kernel Standards)
- Follow Linux kernel coding style (`Documentation/process/coding-style.rst`).
- Use 8-character tabs for indentation.
- Use lowercase identifiers with underscore separators.
- Do not use floating-point arithmetic in kernel code.
- Check every memory allocation (`kmalloc`, `kzalloc`, `vmalloc`).
- Unwind errors using structured `goto` statements in reverse order of resource acquisition:
  ```c
  int ret = -ENOMEM;
  res_a = alloc_a();
  if (!res_a)
      goto err_out;
  res_b = alloc_b();
  if (!res_b)
      goto err_free_a;

  return 0;

  err_free_a:
      free_a(res_a);
  err_out:
      return ret;
  ```
- Use endian-specific types (`__le16`, `__le32`, `__be32`) and explicit conversion macros (`cpu_to_le32`).

### 9.2 C++ (Ceph & Distributed Systems Standards)
- Target modern C++ (C++17 or C++20).
- Manage resource lifetimes with RAII (Resource Acquisition Is Initialization).
- Avoid heap allocations on the hot I/O data path. Use buffer views, intrusive containers, or memory pools.
- Enforce exception safety. Provide strong or no-throw guarantees on state changes.
- Express ownership with smart pointers (`std::unique_ptr` for exclusive ownership, intrusive pointers for ref-counted objects).
- Pass large read-only objects by `const&` or `std::string_view` / `std::span`.

### 9.3 Rust (Kernel & Systems Standards)
- Document every `unsafe` block with a `// SAFETY:` comment.
- Prove in the comment that caller preconditions and safety invariants are satisfied:
  ```rust
  // SAFETY: `ptr` is non-null, properly aligned, and points to an initialized `DeviceHeader`
  // that remains valid for the duration of the `'a` lifetime.
  unsafe { &*ptr }
  ```
- Do not panic in library or kernel routines. Replace `.unwrap()` and `.expect()` with explicit error handling (`Result<T, E>`).
- Enforce memory safety and ownership boundaries across thread channels.
- Use `#[repr(C)]` on structures passed across FFI (Foreign Function Interface) boundaries.

---

## 10. Verification, Sanitizers & Static Analysis

Validate code before committing.

- Enable strict compiler diagnostic flags:
  - GCC / Clang: `-Wall -Wextra -Werror -Wpedantic`.
  - Rust: `#![deny(warnings)]`, `#![warn(missing_docs)]`.
- Run dynamic sanitizers:
  - **ASan** (AddressSanitizer): Detects out-of-bounds access and use-after-free.
  - **TSan** (ThreadSanitizer): Detects data races and thread synchronization defects.
  - **UBSan** (UndefinedBehaviorSanitizer): Detects integer overflow, null pointer dereferences, and alignment violations.
- Run static analysis tools:
  - Linux kernel: `sparse`, `smatch`.
  - C++: `clang-tidy`, `cppcheck`.
  - Rust: `cargo clippy --all-targets -- -D warnings`.
- Write regression unit tests and fuzzing harnesses (e.g., `libFuzzer`, `syzkaller`).

---

## 11. Git Discipline & Patch Etiquette

Create clean, reviewable patch series that follow open-source project standards.

- Keep commits atomic. Each commit must perform one logical change.
- Never mix functional bug fixes with stylistic cleanups.
- Prefix commit subject lines with the affected subsystem:
  - `net/ipv4: fix race condition in socket bind`
  - `osd: validate peer epoch before peering transition`
  - `driver/nvme: add safety comment for ring buffer mapping`
- Write descriptive commit bodies. Explain:
  1. The problem being solved.
  2. The root cause.
  3. The chosen fix and design rationale.
- Add standard sign-off trailers (`Signed-off-by: Full Name <email@example.com>`).
- Verify that every commit in a branch builds and passes tests. Do not break `git bisect`.
