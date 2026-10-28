# rust-mds: Repository Guide

## Coordinated resume — 2026-10-25

**The user explicitly resumed coordinated work on 2026-10-25:** "you may continue work, today you may use Sol/Astra ultrafast max". **Coordinator ACCEPTS the conservative G2 root-shard STORAGE-RESTART gate on layered evidence:** accepted retained native compound/bootstrap/application evidence, independently closed A/B fault families, accepted current primary source and fresh corrected combined checks. The written matrix identifies no additional mandatory native fault series. This is not a fresh current-binary native PASS or cluster query. G0/G1 are not automatically passed; full G3 remains open. Both E1 contract scopes and prior E2/D6/wire-REPLAY acceptance remain unchanged. No live upgrade is authorized. Read `current/sol-handover.md` and `current/sol-orchestration.md` for assignments. Commands still require the assigned owner and safety checks.

The parent orchestrates only. Implementers and independent reviewers may use either verified variant: `openai/gpt-6.1-sol-ultrafast#max` or `openai/gpt-6-astra-ultrafast#max`. Exactly one assigned QA owner controls live operations. This update records accepted retained evidence, not a fresh cluster query by the documentation owner. Preserve fs44/meta88/data89, the reviewed binary/tooling, both acknowledged namespaces and all original/retry evidence; no newfs/reset for recovery gates.

A CephFS metadata server (MDS) written in Rust from scratch (no librados, no C++ Ceph libraries), targeting the unmodified Linux kernel client, ceph-fuse/libcephfs and Ceph 20.2 monitors. Read `HANDOVER.md` first (current status, what is done, next steps in order), then `design/build-brief.md` (conventions the build follows) and `design/minimum-feature-set.md` (requirements, decision log 8.1, client invariants section 7, day-one hooks 5.1).

## Commands

```sh
cargo test --workspace                 # current results belong in progress_ledger.md; old counts are historical
cargo test -p <crate>                  # denc | auth | msgr | monc | proto | engine | osdc | store | rmds
cargo build -p rmds                    # the daemon binary: target/debug/ceph-rmds
cargo fmt --all                        # rustfmt, default settings (no rustfmt.toml)
cargo clippy --workspace --all-targets # record actual warnings and failures; QA owns baseline
```

- With several agents working at once, give each its own `CARGO_TARGET_DIR` (e.g. `target-<name>`; the pattern `target-*` is git-ignored) to avoid the build-dir lock.
- Tests that need the dev cluster or its keys skip with a note when absent. Historical offline tests passed; do not treat this as fresh evidence.
- The 2026-10-22 coordinated run uses explicit file assignments and one QA live-cluster owner. The original build-brief ownership table is historical. Do not run live actions without assignment. G0-G3 contracts are in `design/persistence-contract.md`; gates are in `design/workplan.md`.
- Engine/store and daemon persistence/runtime packages retain independent acceptance. Recorded 2026-10-22 accepted-tree QA: 524 passed, 0 failed, 2 ignored; build/fmt/default clippy pass, strict baseline lint failed. The 2026-10-25 old-version G2 application retry passed 17 acknowledged operations, 10 saved inode identities, cold comparison, 16 future nonreuse checks and final comparison. Retained final state: Ready/tail165/checkpoint15/coverage165/trim165, seven Closed sessions. Positive symlink persistence was not exercised. The original legacy-ID323 failure remains historical. Preserve binary SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`, both namespaces and all evidence. No upgrade or fresh live observation is claimed.
- Engine E2 and daemon D1–D4/clientless E2 remain accepted. `current/sol-daemon-review.md` ACCEPTS D6/R1 and the six-site cleanup: 160 daemon tests, warning-free strict daemon Clippy/build/fmt and 132 stable source hashes. `current/astra-g2-fault-matrix.md` closes A/B at the **pre-R1** revision: independent 4 new fault/11 prior G2/147 full daemon tests. Counts overlap and are not added to the fresh workspace result. These scopes support the coordinator's layered G2 decision; neither becomes a new native trial.
- Primary corrections remain ACCEPTED: monc alias (`current/astra-monc-lint-review.md`) and cfg(test)-only store E2 migration (`current/astra-store-e2-fixture-review.md`). Their focused evidence remains separate from the corrected combined result below.
- The **superseded, pre-fix** candidate `/tmp/opencode/sol-release-e2-d6-20261025.bl5z3why/` remains failed: 586 passed/1 failed/2 ignored, four optional early returns, store integration/doctests unreached and strict monc failure. The original native ID323 trial also remains failed. New evidence does not relabel either failure.
- **Corrected combined checks PASS** in `/tmp/opencode/sol-release-primary-corrected-20261025.2h7ofr85/`, on `dd4cc57` plus 34 reviewed source paths: all six commands exit0; 612 passed/0 failed/2 ignored across 43 harnesses. Four optional bodies return early; store integration and all doctest groups completed. Daemon build, fmt check, verified no-op fmt, default and strict workspace Clippy pass without warnings. All 1,802 candidate files and 649 primary Cargo inputs stayed stable. Exact commands, artifacts and optional-body limits are in `progress_ledger.md`. Layered G2 acceptance does not supply positive native symlink proof, E1 adoption or G3 mounted same-wire-tid qualification; it does not requalify the preserved old binary.
- G3 **application tooling** retains historical independent29/author166 per Python mode. Fault-v1 interface ACCEPT remains at SHA256 `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523`. Original11-path68/234/red-probe/timeout/retry evidence and original FC1–6 review stay unchanged. Completed `current/astra-g3-fault-corrections-review.md` is **CHANGES REQUIRED** for residual P1 G3-FC1-C1 readiness/publication and G3-FC45-C1 mutable worker receipts. Original FC2/FC3 defects and FC6 close only in the recorded offline scope; FC1/4/5 and Reset/UuidOpen provider binding remain open. Manifestdca2d63b/archive0568b6dd stays frozen at `/mnt/workspace/sol-g3-fault-corrections-freeze-20261026.z67rpxas/` for active sole-QA verification. Residual source corrections are **NOT YET ASSIGNED**. Exact independent90/16/8/3 passes, new6 red/control results and1,521/784 pins are canonical in `progress_ledger.md`. The valid manifest is under that workspace root, not the retained empty temporary output.
- **Combined256 remains FAILED in both Python modes:**219 passing IDs/37 blocked baseline IDs,6 failures/240 subtest errors/246 Popen denials. This is **not a fresh166-baseline PASS**. Sole QA now has verification-only ownership in NEW `current/sol-g3-fault-baseline-verification.md`: qualify exact existing offline child commands, then run unchanged166+90 in both modes. Sources/failed evidence stay frozen. The new verification is ACTIVE/unread and has no returned result; no arbitrary children or native actions are authorized.
- Completed P1/P2/P3 independent reviews all return **CHANGES REQUIRED**: Ceph three P2 WCEP-1–3; NFS four P1 NFS41-1–4/two P2 NFS41-5–6; observers four P1 P3-OBS-1–4/three P2 P3-OBS-5–7. Passing focused78/39/127 per mode do not close the new failed probes. Original authors now own r2 corrections in NEW `current/g3-wire-ceph-corrections-r2.md`, `current/g3-wire-nfs41-corrections-r2.md` and `current/g3-provider-observers-corrections-r2.md`; active outputs stay unread. Preserve original49f8f7e6/8fd40b43/5ef0d9f4 freezes, author ledgers/reviews and all failed evidence. Canonical rows retain exact counts/pins; P3 remains Fixture/Unqualified only. None is installed-ready/native accepted. P4/P5/probes/hooks stay held/dependent; native flags false/construction disabled; full A4/B3/R/N and G3 remain unqualified.
- Retained **r1** E1 STATE-1–4 and unmerged corrected-store dependency ACCEPT remain pinned to engine7bfce917/storebadbc7e2: state65 focused/adapted11/11/seven probes/overlapping release overflow, strict/fmt/release build and2,092/199 pins; store74/three validation/two config probes and2,092/685 pins. Original10/11 and old3f421 failures/scopes remain unchanged. Retained execution-r1 **CHANGES REQUIRED** records P2 E1X-WAKE-1/2; original E1X-1–3 reproductions were fixed. Producer/consumer agreement on `retry_capacity_epoch` and parent authorization preceded r2. The new r2 decisions below do not relabel old results.
- Completed isolated `current/astra-engine-e1-wakes-r2-review.md` is **CHANGES REQUIRED** at retained manifest `131ae170c7c4436e1cecb1f6925228998535c357235d2b68fa7e2dad8e8400e1`/map09cbb464. P2 E1X-R2-OVERFLOW loses new/already-owned CloseSession and later dirty controls after callback Capacity; P3 E1X-R2-COMPAT clears another continuation's reservation during compatibility REPLAY. Seeded debug/release each2 pass/3 fail; public3 pass/1 fail. Independent251 engine tests+doctest/strict/fmt pass. Original WAKE-1/2 close; reviewed four-file delta/seven fresh state probes preserve STATE-1–4. Startup2,327/2,327 matched; final strict2,326/2,327 mismatch remains failed, solely the assigned consumer-plan47e1b384→7edcc062 update. Source/other evidence stayed stable. Retain `/tmp/opencode/sol-e1-shared-20261025.0lt2912d/engine-e1-wakes-20261026-r2/HANDOFF.md`, author overflow/unmerged74, distinct unchanged-wake2/1/adapted3/3 and2,171-record r1 archive. The completed r3 decision below does not relabel these failures.
- Completed isolated `current/astra-engine-e1-ownership-r3-review.md` is **CHANGES REQUIRED** at0ca06283/map31814a0a for P3 E1X-R3-COMPAT-CLOSE: compatibility close changes a later cap/range/inode version after Capacity without a record. Ordinary public Journal epoch4 gives0/1; boundary5 gives4/1 per mode. Exact public4/overflow5 close both r2 findings;259+doctest/strict/fmt/release/STATE7/ownership5 pass and2,515 candidate/2,278 baseline pins match. Prior wake/STATE closures and consumer section9 remain valid. Original isolated author owns narrow r4 in NEW `current/sol-engine-e1-compat-close-r4.md`; stop compatibility cleanup after any latched failure, preserve r3 preimages/old evidence and both red probes. No new API/store/daemon change is assigned. Active r4 output stays unread; no full E1 acceptance/promotion/D5 permission exists.
- Exact-r2 **CONSUMER API ACCEPT** in PRIMARY `current/sol-daemon-e1-d5-plan.md` section9 remains interface-only, conditional on implementation acceptance/promotion. Prior r2 dependency74/strict/fmt/two-probe acceptance and653/66 pins keep their old scope. **Final r3 STORE+FIXTURE DEPENDENCY SCOPED ACCEPT** remains complete in PRIMARY `current/astra-e1-r3-joined-review.md`: one actual74 includes fixture/UPG1; strict/fmt/two config probes pass;655 inputs/2,515 r3 records stay stable;68-path reference inventory6b4bb67e. Public API/persistence source identity is unchanged, not full E1 or automatic r4 acceptance. Initial quota link ran zero harnesses without a durable Cargo exit; packaging stops/failed20-byte archive remain retained. Exact maps/usable archive are in `progress_ledger.md`. Fixture SOURCE/independent R1-only1 test/full stable.rs2cb747b0/patch7e6540e5 and all prior failures stay pinned. R4 independent execution acceptance, dependency reconciliation and coordinator code assignment precede promotion/D5.
- Prior checkpoint publication completed: platform `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`, functional `ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93`, QA `805a8b0f1c8388548f0c8adf869aa28c0e70b926`, and final docs `a6ae05a10bfefdcea8dae75cd29597c84a3fadcd`. Generator `85a37c8d615992b4600d7467fdedd514fecfac03` remains preserved. `current/sol-release.md` now reports the ten-path QA milestone normally COMMITTED/PUSHED as `dd4cc57ac1b0c44b60cf33e3ec88bfa539b9c736`, `qa: verify mounted recovery with persistent history oracles`. Approved bytes/modes and local/tracking/actual remote equality matched; ahead/behind was 0/0 and the index was empty at publication. That snapshot left 62 unstaged primary paths (41 modified/21 untracked); later ledger creation can change counts. The future fault-controller proposal was excluded. Remaining source/docs are not all published; the all-changes request is incomplete. This owner makes no fresh Git claim. Only the designated release owner stages/commits/pushes after authorization.
- **Source publication COMMITTED/PUSHED, 2026-10-26:** `18e1b10c241a30ae70807540495bcde46c8dfdf2`, `build: clear remaining strict Rust lint warnings` (6 paths), then `f3fabfdfe52d44ebc2918690bf1ddbd8e48cbff4`, `mds: gate recovery controls and bind reclaim output` (32 paths); union34/overlap4 on preserved dd4cc57/history. Independent split review SCOPED ACCEPTS the intermediate-only cfg(test) annotation; commit2 removes it. Normal sign-offs/hooks/push passed. At00:37 UTC the release owner reported actual remote/local/tracking f3fabfd,0/0,empty index, all1,802 committed entries matching the approved candidate and649 working Cargo inputs stable. Accepted intermediate524/0/2 five-command and final612/0/2 six-command proofs were reused, not rerun; both retain four optional bodies/two ignored helpers. Frozen publication artifacts are in `progress_ledger.md`. Initial lint-intermediate and586/1/2 failures stay failed. The remaining snapshot was16 modified+25 untracked=41 unstaged paths; QA work can change counts. Docs/ledgers/proposals/newQA/isolated E1 were excluded; all-changes publication remains incomplete. These are release-report facts, not fresh Git observations here.
- Next: receive frozen engine r4/P1–P3 r2 corrections and assign independent delta reviews. Reconcile final engine dependency pins/scoped consumer API before authorized promotion/startup-D5 assignment. Receive sole QA's unchanged256 verification before assigning FC1-C1/FC45-C1 source corrections; these remain NOT YET ASSIGNED. Do not inspect active author/QA outputs or poll verification here. P4/P5/probes/hooks remain held/dependent; no native permission follows. Publicationf3fabfd/18e1b10/historical41-path counts stay release facts; all-changes publication is incomplete. No live upgrade/G3 action is authorized.

### Live checks (need the dev cluster)

```sh
devcluster/start.sh --mds none     # create/start 1 mon, 1 mgr, 3 OSDs, no MDS (idempotent)
source devcluster/env.sh           # CEPH_CONF, CEPH_KEYRING, RMDS_KEYRING, MON_V1/MON_V2, FSID, ...
cargo run -p osdc --example rados_live    # live RADOS I/O as mds.rust; prints PASS
./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring $RMDS_KEYRING &   # run the daemon
scripts/smoke.sh [--keep] [--release]     # end-to-end gate: rank 0 active, kernel+fuse mounts, I/O
```

- Production now uses native persistent load and fails closed on missing/foreign/corrupt state; empty replay was historical base `f54d298` behavior. `devcluster/newfs.sh` requires separate QA fresh-setup authorization. Never use it inside persistence/recovery gates: keep filesystem identity and pools unchanged.
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
| `store` | Primary schema-2 guarded WAL/checkpoint/session packages and cfg(test) E2 fixture accepted. Conservative root-shard G2 STORAGE-RESTART gate accepted on layered evidence, not a fresh native PASS. E1 remains isolated, not promoted. |
| `mds` (package `rmds`, bin `ceph-rmds`) | Production `Server<DaemonJournal>` / `Engine<DaemonJournal>`, step scheduler, tagged native store worker, durable OPEN/CLOSE and lifecycle completion. D1–D4/clientless E2 and D6/R1/six-site cleanup accepted; nonempty finalization remains held pending E1/D5. |

Production data flow: client message → wire-free `EngineInput` → `Engine::step` with at most one `RecordPermit` → `DaemonJournal` encoding/outbox → native guarded store worker → owner/operation-tagged completion → `JournalEvent` → safe effects through `Session::send` in order. Commit callbacks materialize output before another admitted step. Daemon `MemJournal` compatibility entry points are test-only. See `crates/mds/README.md`, `design/persistence-contract.md` and `HANDOVER.md` section 5.3.

Engine calling model today: one `Engine` per shard, `&mut self`, never blocks, never does I/O. Every mutation becomes one self-contained `JournalRecord`. Replies are released only after commit (safe replies only). Effects include replies, cap messages, realm splits, recalls, and daemon work (purge, truncate, evict). That calling model is the compatibility foundation. The G5 target is `design/transactions.md`: planned transactions, hierarchical locks, and per-stream commit tokens. Do not grow the G1–G3 scalar sequence into the multi-stream WAL. A shard is not a subvolume (`design/store.md` section 6, decision D3).

Current durable defaults: one root shard/stream; 30 s Objecter actor timeout; checkpoint every 30 s or 16 records; 1 MiB maximum framed record; 64 MiB retained WAL; 64 retained checkpoint generations. SIGUSR1 requests a checkpoint. No checkpoint GC or dynamic lease-refill worker exists. Generation/WAL pressure pauses admission; increase the documented budget and restart with the same identity, never delete metadata. Active messenger backlog has no capacity feedback; daemon queue bounds are not an end-to-end outbound memory bound.

Reference material for exact C++ behavior: `mds-rust-map/` (facts-only map of the C++ MDS and a 935-entry feature catalog; start at its README) and the Ceph `main` source at `/home/ubuntu/ceph`. The Linux kernel client tree is checked out at `/mnt/workspace/ceph-client` (7.3.0-rc6); the dev machine runs `7.0.0-1012-ibm`, which is older. The differences are bug fixes and MDS-irrelevant features only; the wire protocol is the same, so either tree serves as reference.

## Conventions specific to this repository

- Doc comments state the wire format implemented and cite the C++ file and symbol followed.
- New decisions go into the decision log (`design/minimum-feature-set.md` section 8.1).
- Day-one hooks must stay intact (design 5.1): connection reading separate from execution; every client message through the per-session funnel; self-contained versioned WAL records; the clientreplay phase exists; the allocator can delegate inode ranges; shard ownership epochs are checked on every WAL append.
- Client invariants (design section 7) must never be violated. Violations crash or hang unmodified clients (a forward for an answered tid, a wrong dentry-trace parent, directory `nlink` other than 1, trailing bytes in replies). Check `MClientReply::check_invariants_for(&req)` before sending.
- **License caution:** the kernel's `net/ceph` and `fs/ceph` are GPL-2.0. You may read kernel code to understand client behavior. Never translate it into this codebase. The C++ Ceph sources (LGPL) and `mds-rust-map/` are the references.
- Generated files (do not hand-edit): `crates/denc/src/features.rs`, `design/minimum-feature-set-catalog.md` and its JSON.
- Run `cargo fmt --all` before committing. The corrected final candidate's formatter was a verified no-op; respect active-tree ownership and freeze rules.

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
