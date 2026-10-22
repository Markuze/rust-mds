# progress_ledger

**PAUSED — wait for explicit user resume.** All current implementation/review assignments finished. No implementation, tests, G2 retry, G3 or live operations may start. Only handover preparation/independent review and the previously requested commit/push checkpoint remain authorized. Portable state, exact continuations and post-resume commands: [`current/sol-handover.md`](current/sol-handover.md). Coordinator: `current/sol-orchestration.md`.

Release-status checkpoint: platform `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`, functional `ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93` and QA `805a8b0f1c8388548f0c8adf869aa28c0e70b926` are normally committed/pushed. Actual remote main matched local/tracking QA HEAD, ahead/behind 0/0. All 640 committed/working Cargo inputs retain accepted QA/release hashes. The remaining 27 intended status/handovers/task-review ledgers form the final documentation checkpoint at branch HEAD. This text freezes before that commit; the release agent's final report supplies its actual SHA, final normal-push/remote confirmation and clean-tree result. Publication reuses recorded evidence and does not resume tests, implementation, G2 retry, G3 or live work.

## Current gate evidence (canonical)

Updated 2026-10-22 for the coordinated run at base `f54d298`. The table below is the source of truth for gate evidence. Historical entries below do not establish a current pass. QA owns fresh tests and live-cluster actions. Design files define requirements, not execution results.

| Gate/check | Commit/worktree | Environment | Command/evidence | Result | Skips/expected failures | Artifact |
|---|---|---|---|---|---|---|
| G0 contract | f54d298 + design edits | Independent source review, 2026-10-22 | `design/persistence-contract.md` amended boundaries | ACCEPT; all five findings closed, normative API frozen | Target interfaces, not implementation acceptance | `current/sol-contract-review.md` |
| G0 isolated unit baseline | f54d298 archived source, exclusive target below | QA; pinned peers below | `CARGO_TARGET_DIR=$T cargo test --manifest-path $B/Cargo.toml --workspace --locked -- --nocapture` | 347 passed, 0 failed, 2 ignored; reviewer checked source identity and log counts | Ignored dump/extract_corpus; optional kernel-v1-mdsv1 fixture missing; local keys supplied only in process memory | `$A/f54d298-unit-with-local-fixtures.log`; `current/sol-verification-review.md` |
| G0 isolated build / fmt | same isolated f54d298 | QA | `CARGO_TARGET_DIR=$T cargo build --manifest-path $B/Cargo.toml -p rmds`; `cargo fmt --manifest-path $B/Cargo.toml --all -- --check` | Both exit 0 | Not a concurrent-worktree integration result | `$A/f54d298-build.log`, `$A/f54d298-fmt.log` |
| G0 isolated lint | same isolated f54d298 | QA | `CARGO_TARGET_DIR=$T cargo clippy --manifest-path $B/Cargo.toml --workspace --all-targets`; repeat with `-- -D warnings` | Default exit 0 with warnings; strict exit 101 | Baseline warnings remain; strict CI is not green | `$A/f54d298-clippy.log`, `$A/f54d298-clippy-strict.log` |
| G0 original smoke | same isolated f54d298 | QA; fscid 42, pools 84/85 | `CARGO_TARGET_DIR=$T RMDS_RUN_DIR=/mnt/workspace/devcluster-data/sol-f54d298 timeout 360 bash $B/scripts/smoke.sh --keep` | Exit 0; kernel/FUSE I/O and cross-client visibility PASS | Fresh-filesystem MemJournal prototype only; no restart/failover proof | `$A/f54d298-smoke.log` |
| G0 native Objecter | same isolated f54d298 | QA; Ceph 20.2.0 | `CARGO_TARGET_DIR=$T timeout 180 cargo run --manifest-path $B/Cargo.toml -p osdc --example rados_live` | Exit 0; PASS, including blocklist add/rm | Not guarded-WAL integration evidence | `$A/f54d298-rados.log` |
| G0 C++ reference | Packaged Ceph 20.2.0 optimized MDS | QA; fscid 39, pools 78/79 | `source devcluster/env.sh; timeout 160 devcluster/start.sh --mds cpp`; kernel smoke; Ganesha/NFSv4.1 smoke | Startup active with msgr2; kernel and Ganesha smoke exit 0; explicit MDS keyring fixes bad_method | Ganesha helper is graceless; no G3 reclaim qualification | `$A/cpp-start.log`, `$A/cpp-smoke.log`, `$A/ganesha-start.log`, `$A/ganesha-smoke.log`; exact commands in `current/sol-verification.md` |
| G0 benchmark samples | C++ package / isolated f54d298 | QA; 100 single-client samples per class | `sudo timeout 120 python3 scripts/metadata-bench.py MOUNT --label cpp\|rust --count 100` | Both exit 0 | Rust MemJournal vs durable C++; profiles differ; no performance-ratio claim | `$A/cpp-bench.json`, `$A/f54d298-rust-bench.json` |
| G0 QA scripts / CI | Coordinated QA package; isolated base binary for earlier final smoke | Independent correction re-review | 29 script tests pass normally and with PYTHONOPTIMIZE=1; shell/Python syntax checks pass | ACCEPT for scoped partial tooling; all four review findings resolved | GitHub workflow unrun; partial controller returns 77, not G2/G3; no live recovery-mode Ganesha startup | `current/sol-verification-review.md`; `/tmp/opencode/sol-verification-fixes/`; `$A/final-script-smoke.log` |
| G1 engine F1 / admission / failure / visibility | Coordinated engine changes vs f54d298 | Independent re-review, target-sol-engine-review | Durability + review_regressions suites; three independent probes | ACCEPT; prior admission-wakeup and eviction-marker findings closed; 37 existing focused tests and all three probes pass | Scoped re-review, not final full-suite or integration qualification; earlier F1 133 tests + doctest/stress evidence is historical | `current/sol-engine-review.md` |
| Engine recovery-state package | Coordinated engine changes; schema 2 compatible | Independent final reviewer, target-sol-engine-state-review | `cargo test -p engine --test review_regressions --test durability --locked`; independent rejection/valid-transition probes | Final ACCEPT; 22 + 20 = 42 focused tests pass, 0 failed/ignored; all state findings closed | No final full engine/workspace/store/live rerun by this reviewer | `current/sol-engine-state-review.md` |
| Engine author full-suite check | Author's engine package revision, target-sol-engine | Author report, not independent full-suite rerun | `cargo test -p engine --quiet`; default and strict crate clippy | 175 unit/integration + 1 doctest pass; default clippy exit 0 with two baseline warnings; strict exit 101 | Author-only evidence; no scaled stress, workspace or live gate rerun | `current/sol-engine.md` |
| Store WAL/checkpoint core | Corrected coordinated store package | Independent correction reviewer, target-sol-store-wal-review | `cargo test -p store --locked`; `cargo clippy -p store --all-targets --no-deps --locked -- -D warnings` | ACCEPT corrected OMAP short-page pagination; 50 tests pass, 0 failed/ignored; store-only strict clippy passes | Finite actor timeout, bounded uncertainty handling and admission pressure remain daemon integration obligations; no live G2 | `current/sol-store-wal-review.md` |
| Store authority / recovery-state / schema | Coordinated store package at authority-review revision | Independent reviewer, target-sol-store-state-review | `cargo test -p store --locked`; store-only strict clippy | ACCEPT; 40 tests pass, 0 failed/ignored; strict clippy passes | Separate core correction review supplies later 50-test evidence and actual bootstrap scan-loop regressions; no daemon join/live qualification | `current/sol-store-state-review.md`, `current/sol-store-wal-review.md` |
| G1 RADOS package | Coordinated rados.rs changes vs f54d298 | Independent reviewer, target-sol-rados-review | `cargo test -p rmds rados::tests` | ACCEPT; 13 passed, 0 failed, 0 ignored | Durable intent/completion integration and live evidence remain required | `current/sol-rados-review.md` |
| G1 platform package | Coordinated changes vs f54d298 | Independent source/offline re-review | `CARGO_TARGET_DIR=target-sol-platform-review cargo test -p monc -p osdc` | ACCEPT original fixes, native listing and opt-in probe; 24 monc tests, 70 osdc tests and one doc test pass | Scoped package acceptance, not a completed G1 gate | `current/sol-platform-review.md` |
| Native platform primitives | Modified tree at HEAD f54d298; native sources/manifests unchanged during probe | QA on Ceph 20.2.0, existing data pool 87; retained evidence independently ACCEPTED | `CARGO_TARGET_DIR=target-sol-verification timeout --kill-after=15 360 cargo run -p osdc --example rados_live -- --platform-probe --pool cephfs.a.data --conf "$CEPH_CONF" --keyring "$RMDS_KEYRING" --id rust --inc 1` after sourcing devcluster/env.sh | Exit 0; get_version newest 402/oldest 1 and checked wait 402; CMPXATTR/ASSERT_VER successes and raw/checked failures -125/-34 preserve exact data/tail/version; all-namespace scan 8 pages, 6 empty continuations, zero restarts, both sentinels found | No prerequisites skipped; stable epoch only, nonempty locators untested; not bootstrap, format hygiene, durable WAL/checkpoint or G1-G3 qualification | `/tmp/opencode/sol-platform-live-20261022.YcfShl/`; `current/sol-platform-review.md` |
| G1 daemon original protocol/fencing package | Earlier correction package, before G2 integration | Independent reviewer, target-sol-daemon-review | `cargo test -p rmds`; `cargo build -p rmds` | ACCEPT original findings and N1-N3; 59 tests pass, 0 failed/ignored; build passes | Earlier scoped evidence; later integration reviews recorded separately below | `current/sol-daemon-review.md` |
| Daemon G2 persistence integration | Reviewed persistence package before final runtime corrections | Independent reviewer, target-sol-daemon-store-review | `cargo test -p rmds --locked`; `cargo clippy -p rmds --all-targets --no-deps --locked` | ACCEPT authority, data flow, uncertainty, checkpoint/session join and lifecycle integration; 71 tests pass, 0 failed/ignored; default daemon-only clippy passes with dead-code warnings | Real store API with atomic compound memory backend, not native RADOS durability evidence | `current/sol-daemon-store-review.md` |
| Daemon G2 runtime corrections | Corrected integrated package, G2-R1 through G2-R4 | Independent final reviewer, target-sol-daemon-review | `cargo test -p rmds --locked g2_r -- --nocapture`; `cargo test -p rmds --locked`; `cargo build -p rmds --locked` | ACCEPT; 11 focused + 82 full tests pass, 0 failed/ignored; production build passes | Shared working-tree package evidence, not an isolated combined-tree or live pass; reviewer did not rerun clippy | `current/sol-daemon-review.md`, focused G2 correction section |
| Daemon author correction checks | Same author's correction package, target-sol-daemon | Author report, not independent lint rerun | `cargo test -p rmds --locked`; `cargo build -p rmds --locked`; `cargo clippy -p rmds --all-targets --no-deps --locked` | 82 tests/build/default crate clippy pass; existing dead-code warnings; owned formatting/whitespace checks pass | Author pre-fix reds are reported, not independently repeated; no workspace or live result | `current/sol-daemon.md` |
| QA complete-G2 / partial-G3 harness | Corrected executable harness and shared namespace oracle | Independent final correction review | 11 selected tests normally and with PYTHONOPTIMIZE=1; independent hidden-child/type-error and omitted-directory probes | ACCEPT complete clean-restart application harness and partial G3 foundation; both oracle P1 cases closed; 11 tests pass in each run | Tooling only; G3 has no full PASS path; reviewer did not repeat the final full 67-test author suite or run live operations | `current/sol-verification-review.md`, expected-directory coverage correction section |
| QA harness author checks | Final oracle correction package | Author report | Guarded unittest discovery normally and with PYTHONOPTIMIZE=1 | 67 tests pass in each run | Author-only full suite; no native/live gate result | `current/sol-verification.md`; `/tmp/opencode/sol-g2-enumeration-coverage-20261022/` |
| Final combined offline tests | Accepted modified tree recorded at HEAD f54d298; exact source/binary below | QA, target-sol-verification; retained evidence independently accepted | `cargo test --workspace --locked -- --nocapture`; guarded focused tooling tests | 524 passed, 0 failed, 2 ignored including doctests; 67 tooling tests pass; reviewer independently recounted 42 Rust result groups | Ignored dump/extract_corpus; optional kernel-v1-mdsv1 capture/corpus absent with explicit early return; not a pristine f54d298 result | `$I/unit.log`, `$I/tooling-unit.log`; `current/sol-daemon-store-review.md` |
| Final combined build / fmt | Same accepted-tree snapshot | QA; retained evidence review, not independent rerun | `cargo build -p rmds --locked`; `cargo fmt --all --check` | Both exit 0; no formatter writes | Build evidence tied to exact binary below | `$I/build.log`, `$I/fmt.log`, `$I/offline-results.txt` |
| Final combined lint | Same accepted-tree snapshot | QA; retained evidence independently checked | `cargo clippy --workspace --all-targets --locked`; repeat with `-- -D warnings` | Default exit 0 with warnings; strict exit 101 on recorded baseline diagnostics | 46 generated denc identity_op diagnostics and engine for_kv_map/collapsible_if; no strict-lint/CI-green claim | `$I/clippy.log`, `$I/clippy-strict.log` |
| Native fresh Rust setup / bootstrap | Same reviewed binary; separately authorized setup, not part of restart trial | QA; fs44/meta88/data89; scoped retained evidence ACCEPTED | One authorized `devcluster/newfs.sh`; approved daemon `-i rust -c "$CEPH_CONF" --keyring "$RMDS_KEYRING" --mds-join-fs a` | Both fresh pools independently listed empty; standby -> creating -> active; native Ready and initial checkpoint generation1/coverage1 | Sole setup recreation; preserve this identity for every later trial; not complete G2 | `$I/fresh-setup.log`, `$I/bootstrap-daemon.log`, `$I/store-bootstrap.json`; exact command in QA ledger |
| Native clientless restart observation | Preserved fs44/meta88/data89, same binary | QA; retained evidence independently accepted | Private-namespace G2 trial: clean unmount/audit, owned pidfd TERM, direct restart | 17 acknowledged operations / 10 prior inode identities recorded; native replay336 -> reconnect337 -> rejoin338 -> active339; checked load with zero Open sessions | Pre-stop history and legal startup only; no post-restart equality/nonreuse proof | `$I/g2-trial-isolated/history.jsonl`, `$I/g2-trial-isolated/restart.log`; `current/sol-daemon-store-review.md` |
| Complete G2 application trial | Same fs44/meta88/data89; no reset/pool change | Exclusive QA; private mount namespace | `RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh g2 "$PID" "$MNT" "$OUT" --owned-mount --rust-filesystem --expected-fscid 44 --binary "$BIN" --keyring "$KEYRING"` | EXIT 1 at cold_remount: Linux recycled legacy mountinfo ID323; COMPLETE G2 NOT PASSED | Stopped before state comparison and 16 future inode checks; no result.json/PASS. Earlier host-namespace attempt exited 2 before workload due propagated mounts | `$I/g2-isolated-result.txt`, `$I/g2-trial-isolated/error.json`; exact wrapper in QA ledger |
| Durable kernel/FUSE smoke | Same preserved fs44/meta88/data89 and reviewed binary | QA; retained evidence independently accepted | Private-namespace `scripts/smoke.sh` with RMDS_BIN set to reviewed binary and RMDS_RUN_DIR=sol-durable-smoke-20261022-2014 | Exit 0; both 8 MiB I/O/read-back, metadata and cross-client visibility pass; no newfs/reset | Normal-operation smoke, not lost-history comparison or mounted takeover. Non-fatal FUSE admin-socket path-length warning; admin socket not qualified | `$I/durable-smoke.log`, `$I/durable-smoke-daemon.log`, `$I/durable-smoke-result.txt` |
| Unique mount identity correction | Tooling-only follow-up; original failed evidence retained | Author verification plus independent focused review | Required boot UUID + kernel 64-bit UNIQ-ID, fail-closed parser/cleanup and full-controller same/different identity models | ACCEPT; author 83 tests normally/optimized; independent 21 tests normally/optimized. No corrected live retry | Complete G2 still NOT PASSED; retry PAUSED until user resumes and sole QA owner is assigned. Preserve fs44/meta88/data89 and old artifacts | `current/sol-verification-review.md`; `/tmp/opencode/sol-unique-mount-correction-20261022/` |
| First platform publication | Exactly 13 platform paths; preserved baseline parent f54d298 | Assigned release owner | Isolated candidate build/tests; normal signed-off commit and `git push --porcelain origin HEAD:refs/heads/main` | COMMITTED/PUSHED a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22, `platform: add checked fencing and native pool enumeration`; local/tracking/actual remote main match | Baseline parent now published; all 640 Cargo inputs unchanged across HEAD. That historical release snapshot left 100 intended paths (43 modified, 57 untracked) uncommitted; all-changes request was OPEN at that point | `current/sol-release.md`; `/tmp/opencode/sol-release-platform-20261022.chxm820v/` |
| Paused checkpoint release | All current assignments complete; accepted handover; unchanged tested source | Design prepared handover, independent reviewer accepted, designated release owner publishes | User authorized all remaining reviewed groups without new tests/build/live work | Functional ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93 (53 paths) and QA 805a8b0f1c8388548f0c8adf869aa28c0e70b926 (23 paths) normally pushed; actual remote matched QA HEAD, ahead/behind 0/0. Final 27 docs/ledgers checkpoint is branch HEAD | Final release report supplies actual final documentation SHA, remote equality and clean-tree result after text freezes; publication does not resume progress or complete G2/G3 | `current/sol-handover.md`, `current/sol-release.md`, `current/sol-orchestration.md` |
| G0 / G1-G3 integration | Accepted modified code with recorded combined offline and scoped native results | All lanes PAUSED | `design/workplan.md` full gates | G0 partial; no full G1-G3 gate completion declared; complete G2 application NOT PASSED, G3 unqualified | Strict baseline lint remains; unique-mount fix accepted, retry awaits explicit resume. Scoped native evidence is not a full storage fault matrix or release completion | Per-owner `current/` ledgers |
| Historical prototype | 9743767 / 1e7c974 | 2026-10-13 dev cluster | smoke, rados_live, pjdfstest; log below | Recorded passing prototype gates; 8,769 pjdfstest passes | 28 TODO + 1 Y2038 client-wire case | Historical log below; full artifacts not recorded here |
| Historical codec | 66718c1 | 2026-10-13 | store roundtrip/replay/torn-tail tests; log below | Recorded passing codec tests | Does not prove production corruption/recovery rules | Historical log below |

Evidence paths: `A=/tmp/opencode/sol-g0`, `B=$A/base-source`, `T=/mnt/workspace/rust-mds/target-sol-verification-f54d298`. Base is `f54d2988ebaeeb8311314fe6894ee030a80e955d`. Review checked 1,722 archived tracked files against their base Git blobs: zero mismatches. Authoritative Rust baseline evidence uses the isolated `f54d298-*` artifacts. **All earlier `immutable-*`, `base-build.log`, and results from the shared baseline target are invalid contaminated-source evidence. Do not promote them.** See `current/sol-verification.md` for provenance and commands. Review inspected the live-result records; it did not independently rerun live tests.

Pinned environment: Ceph 20.2.0 `69f84cc2651aa259a15bc192ddaabd3baba07489`; rustc 1.99.0 `b940084d7`; kernel `7.0.0-1012-ibm`. C++ used the packaged optimized binary, not the source checkout. Rust used dev profile, opt-level 1.

Integrated evidence: `I=/tmp/opencode/sol-integrated-g2-20261022-2014/`. All combined Cargo commands used `CARGO_TARGET_DIR=target-sol-verification`. Approved executable: `/mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds`, SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`. The independent reviewer checked that binary hash, all 640 Cargo input hashes across QA/release/current snapshots, all 1,784 archived source files and 14 retained evidence hashes. Production source/manifests and scripts were unchanged during the run; only status documents changed. These are accepted modified-tree results recorded before the later platform publication, not an unmodified f54d298 rerun. The normal platform commit changed HEAD attribution without changing those 640 Cargo inputs. The subsequent tooling-only correction is separate and not covered by the recorded harness result.

Native state accepted from retained raw control data: after the failed application trial, Ready/tail36/checkpoint generation4/coverage34/trim34; after same-filesystem smoke, owner epoch3/incarnation342/gid16577, tail83, checkpoint generation7/coverage82, trim82, remaining WAL key83. These positions prove observations of publication/trim, not exact replay counts, acknowledged-history equality or crash-boundary safety. Preserved identity: cluster `3ceca2c6-851f-41cf-b538-1cfded4f1e82`, filesystem44, metadata88, data89. Final QA audit found no MDS or test mounts, original mon/mgr/OSDs alive and 17 PGs active+clean. No reset/newfs/pool change occurred inside or after the restart trial. Retry must retain this identity and the original failed-trial artifacts. Complete G2 still needs positive cold-client identity, the full saved-state comparison, 16 acknowledged future inode allocations disjoint from all 10 saved identities, and the final old+new comparison.

Native platform evidence is separate from the isolated baseline. In `/tmp/opencode/sol-platform-live-20261022.YcfShl/`, the reviewer matched all 545 archived native files against before/after manifests: zero mismatches. Only unrelated `crates/store/src/sessions.rs` changed during the run. The complete scan covered all eight PGs with acting primaries {0,1,2}; this is supported by numeric-PG traversal and retained PG metadata, not a per-request primary trace. Both unique default/non-default namespace sentinels were owner-guard removed and independently confirmed ENOENT (rados CLI exit 1, errno 2). Probe assertions establish unchanged content, tail and nonzero user_version after rejected compounds; the log does not print those version numbers. Retained filesystem before/after JSON is identical: fscid 43, metadata pool 86, data pool 87. All 17 PGs remained active+clean. No reset/newfs, pool changes or broad cleanup occurred. No probe/MDS process or test mount remained. Important artifacts include `probe.log`, `result.txt`, `native-source.tar`, `changed-during-run.json`, `cleanup-independent-stat.json` and `data-pool-pg-summary.json`. Platform native validation is complete only for this reviewed scope; live MapChanged and nonempty-locator coverage remain separate.

Current implementation: production `Server<DaemonJournal>` owns `Engine<DaemonJournal>`. Request/cap/release/tick/close/lifecycle inputs use `EngineInput` / `RecordPermit` / `EngineStep`. Explicit owner/operation-tagged storage completions advance the contiguous committed prefix and deliver `JournalEvent` before the next step. `store` supplies native guarded stable WAL, immutable materialized checkpoint publication/trim, allocator reservations and durable rank sessions. Startup uses native FSMap identity, predecessor fencing, separate shard/session acquisition and checked restore/replay. `MemJournal` daemon compatibility entry points are test-only. Missing/foreign/corrupt recovery state never becomes a fresh namespace.

Engine/store and daemon persistence/runtime packages have scoped independent acceptance. Combined offline results and scoped native bootstrap/clientless restart/smoke have accepted retained evidence. The complete G2 application result remains failed; its unique-mount tooling correction is now independently accepted, but no retry ran. All implementation/live progress is PAUSED until explicit user resume. Nonempty Open-session gathers and reconnect packets remain held for G3. No checkpoint GC, dynamic lease-refill worker or active-messenger outbound capacity feedback is provided. This handover changes documentation only; normative contracts and Cargo source stay unchanged. Historical f54d298 results describe the old prototype. Platform, functional and QA publication is confirmed above; the final docs checkpoint is branch HEAD, with actual final publication/clean-tree confirmation in the release report. The designated release owner performs only that Git checkpoint; it does not resume progress. Host-specific artifacts and target binary remain locations, not claims of committed deliverables; retention/resume checks are in `current/sol-handover.md`.

## Historical build ledger (2026-10-13)

Task: Continue the Rust MDS build per HANDOVER.md. 5.1 (engine wiring, gate
`scripts/smoke.sh`) and 5.2 (OSD client in the daemon) are done. This ledger
tracks agent progress.

Status:
- **5.1 COMPLETE** (2026-10-13 13:55): `scripts/smoke.sh` passes; three kernel-visible cap
  bugs fixed against the golden kernel-v1 captures (see decision log 8.1).
- **5.2 COMPLETE** (2026-10-13 15:30, second agent): Objecter wired into the daemon
  (one messenger identity, osdmap subscription, client_inc from the MDSMap incarnation,
  `Fenced` exits). Effects run RADOS work via `crates/mds/src/rados.rs`: PurgeInode
  (verified: data pool empty after a smoke run), TruncateData (TRIMTRUNC per object),
  RecoverFileSize (probe + log; engine callback pending), EvictClient (monitor blocklist
  through new `MMonCommand`/`MonClient::command`, verified in rados_live, then session kill).
- **5.1 tail DONE** (2026-10-13 17:05, first agent): pjdfstest on a kernel mount:
  8769 pass, 29 not-ok = 28 suite `# TODO`s (known Linux behaviors) + 1 Y2038 assertion
  (utimensat/09.t: the kernel client truncates tv_sec to u32 on the wire,
  `ceph_encode_timespec64`; fails identically against C++). One more engine bug fixed
  via rename/24.t: `HoldKind::RenameSrcWrlock` recalls `Fs Fx` on the rename source
  parent from every holder incl. the renamer, so the revoke/re-grant cycle refreshes
  the client's cached `nsubdirs` (commit 1e7c974).
- Gates re-run after 5.2 and the rename fix: workspace tests pass, fmt clean, smoke.sh
  PASS, `rados_live` PASS (now including blocklist add/rm).
- Committed: 9743767 (engine wiring + smoke gate + 5.2 RADOS work), 1e7c974 (rename
  source-dirstat refresh + pjdfstest result).

Context:
- Deviations logged in decision log 8.1 (2026-10-13 "Eviction ordering without
  persistence"): no OSDMap wait/epoch barrier before the kill; RecoverFileSize result
  not applied. Both land with 5.3.
- `MonClient::command` fails in-flight commands on session loss (caller retries); tids
  are per-MonClient-task.
- Build with `CARGO_TARGET_DIR=target-monitor` for agent work; smoke.sh builds into
  ./target itself.
- Dev cluster up (data kept). Between daemon runs use `devcluster/newfs.sh`.
- A stale hung ceph-fuse mount wedged scripts on Oct 13 morning (realpath on a dead
  FUSE mount blocks in D state); `devcluster/umount.sh` now resolves the mountpoint
  lexically. `mount-fuse.sh` still uses realpath (line 14, 20) - safe unless a stale
  mount is present; fix if it bites again.

Historical next steps (superseded by G0-G3 in design/workplan.md):
1. DONE: WAL record encoding. New crate `crates/store` (engine stays wire-free):
   versioned denc encoding of JournalRecord v1 (`record.rs`, plain functions,
   orphan rule) and the segment framing of design 3 (`wal.rs`:
   `u32 len | u32 crc32c | u64 seq | u64 txid | body`; `read_segment` stops at
   the first torn/corrupt/out-of-sequence record). Tests: workload round trip,
   replay rebuilds an identical engine, torn tail, crc corruption, unknown
   same-version tail fields skipped. Commit 66718c1.
2. Historical proposal: the RADOS objects - `rmds.format` marker, `rmds.shardmap` (owner
   epochs), the Objecter-based WAL appender (group commit, `cmpxattr(rmds.owner)`
   guard on every append), then checkpointing into inode/dir omaps and replay
   from RADOS. Gate: kill -9 under load, no acknowledged metadata lost.
   Entry points for step 2: `design/store.md` sections 1-3; the Objecter API in
   `crates/osdc/src/objecter.rs` and `examples/rados_live.rs` (wiring); the
   engine's `Journal` trait (`crates/engine/src/journal.rs`) is the interface
   the RADOS journal must implement; the daemon holds it in
   `crates/mds/src/server.rs` (`Server::new` creates `Engine::new(cfg,
   MemJournal::new())` today - swap in the RADOS journal behind an async
   commit); `crates/mds/src/rados.rs` already has the Objecter handle plumbing
   from 5.2.
3. Then failover: replay, reconnect, rejoin.

## Handover notes for a new agent (any harness)

Read order: AGENTS.md (repo guide: commands, conventions, gotchas) ->
HANDOVER.md (status) -> this file -> design/store.md (what you build next).
Historical observation: the tree was clean at 66718c1. This is not a current worktree claim.

Environment facts:
- Dev cluster is UP (1 mon, 1 mgr, 3 memstore OSDs; data kept in
  /mnt/workspace/devcluster-data). Between daemon runs: `devcluster/newfs.sh`.
  Never kill OSDs; stop only with `devcluster/stop.sh` (memstore loses data).
- No MDS is running. `./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring
  $RMDS_KEYRING` after `source devcluster/env.sh`.
- The C++ reference MDS (`start.sh --mds cpp`) currently FAILS to start (msgr2
  cephx "bad_method"; keys verified matching). Unresolved; fix if reference
  diffs are needed.
- Use `CARGO_TARGET_DIR=target-monitor` (or any target-*) for agent builds;
  `scripts/smoke.sh` builds into ./target itself.
- pjdfstest lives at /mnt/workspace/pjdfstest, prebuilt (no autotools here;
  config.h is hand-written - see "pjdfstest tooling" below).
- If a fuse mount hangs scripts, `devcluster/umount.sh <mnt>` handles it.
- One agent may be working in parallel (zellij sessions mds_peer/mds_charm);
  check `find crates -newermt <last-commit-time> -type f` and the ledger mtime
  before starting, and re-read files before editing if it was active.

pjdfstest tooling: autotools are absent; /mnt/workspace/pjdfstest is built with a
hand-written config.h (define HAVE_STRUCT_STAT_ST_{ATIM,MTIM,CTIM} or ns assertions
report "unknown") and `cc -O2 -o pjdfstest pjdfstest.c`. Run as root from a directory
on the mount: `cd <mnt> && sudo prove -rv /mnt/workspace/pjdfstest/tests`.

Log:
- [2026-10-13 06:48] Ledger created; inherited WIP assessed (compiles, 328 tests pass).
- [2026-10-13 07:00-13:50] Live gate debugging: smoke.sh line-87 ENOENT after drop_caches.
  Wire-diffed against golden kernel-v1 captures; three bugs found and fixed (see above).
- [2026-10-13 13:55] scripts/smoke.sh PASS. Full workspace tests green. fmt clean.
  Not committed. Daemon stopped, mounts cleaned, cluster data kept.
- [2026-10-13 15:30] 5.2 done (second agent): osdc in the daemon, RADOS effects,
  mon command + blocklist. 343 tests, smoke PASS, rados_live PASS. Not committed.
  Umount.sh realpath fix from the morning wedge kept.
- [2026-10-13 16:45] First agent committed 9743767 (everything: 5.1 + 5.2 + fmt churn).
- [2026-10-13 17:05] 5.1 tail done: pjdfstest 8769 pass (28 TODO + 1 Y2038 wire limit).
  rename/24 fix (RenameSrcWrlock) committed as 1e7c974. utimensat/08 was a helper-build
  artifact (missing ns stat defines), not an MDS bug. C++ MDS bring-up is currently
  broken (msgr2 auth `bad_method` in mds.a; keys verified matching) - worth fixing for
  future reference diffs.
- [2026-10-13 17:35] 5.3 started: `crates/store` with the WAL record encoding (v1)
  and segment framing; round-trip/replay/torn-tail tests green; workspace green.
