# Handover: Rust CephFS MDS

**Coordinator ACCEPTS the conservative G2 root-shard STORAGE-RESTART gate on layered evidence:** accepted retained native compound/bootstrap/application evidence, independently closed A/B fault families, accepted current primary source and fresh corrected combined checks. The written matrix identifies no additional mandatory native fault series. This is not a fresh current-binary native PASS, cluster query, positive native symlink proof, E1 adoption or G3 mounted same-wire-tid qualification. G0/G1 are not automatically passed; full G3 remains open. Existing contract/E2/D6/application scopes and publication f3fabfd after lint18e1b10 remain unchanged. All five new independent reviews return CHANGES REQUIRED. Engine r4 and P1/P2/P3 r2 corrections are assigned. Final r3 store+fixture dependency and consumer API retain scoped acceptance; r4 execution acceptance and dependency reconciliation precede promotion/D5. FC residual corrections are NOT YET ASSIGNED; its source stays frozen for active unread QA verification. Both old combined256 runs remain FAILED; no new result or fresh166-baseline PASS is received. Sole QA retains live ownership. Preserve fs44/meta88/data89, old binary330cf3ee, both acknowledged namespaces and all evidence; no reset/newfs. Read [`current/sol-handover.md`](current/sol-handover.md) and the coordinator ledger before assigned work.

> This 2026-10-26 checkpoint reads the five completed independent reports and coordinator Status/Next/tables/final five entries. Active engine r4/provider r2 author outputs and sole-QA verification were not opened or polled. All execution/preservation/publication claims are report-based, not fresh Git/cluster/test observations here. This owner ran no tests, builds, live actions, Git commands or delegation. Historical isolated `f54d298` evidence stays separate. Read [`design/persistence-contract.md`](design/persistence-contract.md), its accepted [recovery contract](design/recovery-contract.md), [`design/workplan.md`](design/workplan.md), [`design/store.md`](design/store.md), then [`design/minimum-feature-set.md`](design/minimum-feature-set.md). The build brief's original ownership table is historical.
>
> Design revision 2026-10-28: D3, D4 and D5 no longer treat a subvolume as a shard. The current cycle remains G0–G3 on the selected single-shard store. The target execution model is [`design/transactions.md`](design/transactions.md), proved at workplan G5. Do not encode inode-range ownership into later placement.

## 1. Where things stand

**Accepted modified-tree QA, 2026-10-22:** 524 tests passed, 0 failed, 2 ignored including doctests; build, fmt check and default workspace clippy passed. Strict clippy exited 101 on the recorded generated-denc and engine baseline diagnostics. Independent retained-evidence review verified source/binary hashes and recounted the results; it did not rerun tests/live operations. This historical result stays unchanged. The coordinator now accepts G2 on the layered evidence below; that decision does not automatically pass G0/G1 or close full G3.

Historical isolated prototype baseline: unmodified `f54d298` had 347 passed, 0 failed, 2 ignored and passing build/fmt/native RADOS/kernel+FUSE smoke. Its counts and MemJournal behavior are not the new integrated results.

The amended persistence contract remains independently ACCEPTED. Both E1 contract scopes remain accepted in `current/astra-recovery-contract-review.md` (R4 closed) and `current/astra-retry-upgrade-review.md` (U1/U2 closed), alongside E2/D6/wire REPLAY. Accepted E1 specifies `reply_target_ino`, committed-watermark/terminal-cleanup retention, capless original-target EINTR, terminal `ControlCapacity` and the user-selected clean-session transition to required semantics 3. Authority v2 supplies the downgrade fence; the original bootstrap bytes and outer framing stay unchanged. U1 stops the Legacy2 seed-completing process with `UpgradeRequired` before `Loaded`; a later assigned opt-in process performs adoption. U2 binds lifetime-bounded historical `SessionKey` values before legacy range-only replay and applies final clean checks only after tail T. These are contract requirements, not joined E1 implementation or live-upgrade evidence. Earlier engine/store checks remain historical: engine state review ran 42 focused tests; its author reported 175 tests plus one doctest. Store core review passed 50 tests and store-only strict clippy. The 2026-10-22 combined result stays separate from the fresh corrected result below.

Historical daemon G2 persistence integration is independently ACCEPTED (71 tests and default daemon-only clippy at that review revision). Runtime corrections G2-R1 through G2-R4 are independently ACCEPTED (11 focused tests, 82 full daemon tests and production build). At that revision, the author separately reported default crate clippy and owned formatting checks passed with existing dead-code warnings. QA's complete-G2 application harness and partial-G3 foundation were ACCEPTED after both namespace-oracle fixes; that review ran 11 selected tests in each Python mode, while the author ran 67 tests. These are scoped offline checks, not native durability qualification.

**New scoped implementation acceptance, 2026-10-25:** engine E2 is independently ACCEPTED with 185 tests + one doctest, three probes and strict crate clippy (`current/sol-engine-review.md`). Daemon D1–D4/clientless E2 migration is independently ACCEPTED with 44 G3, 11 G2, 10 persistence tests, one phase test, three probes and build/clippy/fmt checks (`current/sol-daemon-review.md`). Daemon filters overlap; do not add their counts. Neither report supplies new combined-workspace or native evidence.

**D6/R1 and strict-CI cleanup: SCOPED ACCEPT** in `current/sol-daemon-review.md`. D6-R1 is closed; the exact six cleanup sites are accepted. Independent verification passed 160 daemon tests, warning-free strict daemon clippy/build and owned fmt. All 132 source paths/hashes stayed stable. Evidence: `/tmp/opencode/sol-daemon-r1-strict-ci-20261025.aKufTD/`. This does not supply a workspace or native pass.

**Both remaining G2 fault families are CLOSED** in `current/astra-g2-fault-matrix.md`: interrupted Closing joins and lifecycle executor/retry/completion cuts. Independent 4 new fault tests, 11 prior G2 regressions and 147 full daemon tests passed at the **pre-R1** revision. Counts overlap; that review is not fresh 160-test evidence and is not added to the new workspace count. The matrix's lines 96–102, 117–128 and stopping rule at line224 support layered G2 acceptance without an additional mandatory native fault series. Deterministic backends do not become native OSD fault trials.

Primary corrections are independently ACCEPTED: the monc private alias (`current/astra-monc-lint-review.md`, verified retained 24-test result and independent strict focused clippy), and the cfg(test)-only store E2 fixture (`current/astra-store-e2-fixture-review.md`, verified 50-test author suite and one independent focused test). Store production bytes are unchanged. The focused fixture test is part of those 50, not an extra test.

**The superseded combined source candidate is NOT PASSED:** `/tmp/opencode/sol-release-e2-d6-20261025.bl5z3why/` predates the latest corrections. It recorded 586 passed/1 failed/2 ignored and four optional early returns. Store integration tests/doctests were not reached. Build/fmt/default clippy passed; strict clippy failed at monc. New results do not relabel that failure.

**Fresh corrected combined checks PASS:** `/tmp/opencode/sol-release-primary-corrected-20261025.2h7ofr85/`, on `dd4cc57` plus 34 reviewed source paths. All six commands exited0 without warnings: 612 passed/0 failed/2 ignored across 43 harnesses (9 unit/380 passes, 26 integration/229 passes plus2 ignored, 8 doctest/3 passes). Four optional test bodies returned early; two corpus helpers remain ignored and three of four corpus pairs were compared. Store integration and all doctest groups completed. Daemon build, fmt check, verified byte/mode no-op fmt, default and strict workspace Clippy passed. All 1,802 candidate files and 649 primary Cargo inputs stayed stable. The canonical table in `progress_ledger.md` records exact commands, logs, overlay SHA256 and optional-body limits. This supplies current-source offline evidence for the layered G2 decision, not a fresh native trial.

**G3 application tooling/interface acceptances keep their historical scopes:** independent29/author166 per mode; interface SHA256 `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523`. Original FC1–6 review and old11-path68/234/red-probe/timeout/retry evidence stay unchanged. Completed `current/astra-g3-fault-corrections-review.md` is CHANGES REQUIRED for residual P1 G3-FC1-C1 readiness/publication and G3-FC45-C1 mutable decoded receipts. Original FC2/FC3 defects and FC6 close in the recorded offline scope; FC1/4/5 and Reset/UuidOpen provider binding remain open. The12-pathdca2d63b/0568b6dd freeze remains unchanged for active sole-QA verification. Residual source corrections are **NOT YET ASSIGNED**. Exact passes/new failed probes/pins are canonical. The valid manifest remains under the workspace freeze root, not the empty temporary output. **Both old combined256 runs remain FAILED; no fresh166-baseline PASS exists.** New unchanged166+90 allowlisted verification is ACTIVE/unread with no received result. No arbitrary child/native action is authorized.

P1/P2/P3 independent reviews all return **CHANGES REQUIRED**: Ceph three P2 WCEP-1–3; NFS four P1 NFS41-1–4/two P2 NFS41-5–6; observers four P1 P3-OBS-1–4/three P2 P3-OBS-5–7. Focused78/39/127 passes per mode do not close the new failed probes. Original authors now own r2 corrections in new ledgers listed in `current/sol-handover.md`; active outputs stay unread. Original freezes/ledgers/reviews and earlier70/77,38/status-position and79/three-subtest failures remain preserved. P3 stays Fixture/Unqualified only; none is installed-ready/native accepted. P4/P5/probes/hooks remain held/dependent. Native/full-G3 flags stay false, construction disabled and full A4/B3/R/N unfinished.

**Current production path:** `Server<DaemonJournal>` owns `Engine<DaemonJournal>`. It submits one admitted record through `EngineInput` / `RecordPermit` / `EngineStep`. A native store worker writes guarded stable WAL records, delivers explicit tagged completions, and publishes immutable materialized checkpoints before bounded trim. Durable rank Open precedes OPEN; Closing precedes terminal engine cleanup; Closed follows its committed receipt. Startup discovers filesystem identity through FSMap, fences persisted predecessors, acquires separate shard/session authorities, and uses checked restore/replay. Daemon `MemJournal` compatibility entry points are test-only. Missing/foreign/corrupt recovery state fails closed.

Historical 2026-10-22 setup/trial: one separately authorized fresh Rust **filesystem44 / metadata88 / data89** reached native Ready/active. Its 17-operation/10-inode trial cleanly restarted, but the controller exited 1 at recycled legacy mountinfo ID323 before comparison/nonreuse. That failure remains unchanged. The boot UUID + kernel 64-bit UNIQ-ID correction is **ACCEPTED**: 83 author/21 independent tests in each Python mode.

Separate historical kernel/FUSE smoke passed on that same filesystem without reset/newfs. Its final positions were tail83, checkpoint generation7/coverage82, trim82. These remain scoped normal-operation observations, not a storage-fault or G3 pass.

**2026-10-25 retry: ACCEPTED application evidence** (`current/astra-g2-evidence.md`). The controller exited 0/result PASS on the preserved filesystem. It completed 17 acknowledged operations, saved 10 inode identities, proved a cold client with changed boot-scoped UNIQ-ID, compared saved state, allocated 16 distinct nonreused future inodes and passed the final old-plus-new comparison. The original namespace passed before/after verification. Positive symlink persistence and precise same-wire-tid reply loss were not exercised. This was clean client close and MDS TERM, not mounted-client failover or the full storage-fault matrix.

Retained final state: Ready, owner epoch5/incarnation354/gid16766, tail165, checkpoint generation15/coverage165/trim165, empty WAL keys and seven Closed sessions. Cleanup snapshots show no MDS/standby/FUSE/CephFS mounts, original mon/mgr/three OSDs alive and 17 PGs active+clean. These are accepted report observations, not a new query. Nonempty G3 finalization remains held pending E1 implementation and daemon D5, not contract re-review.

Integrated evidence: `/tmp/opencode/sol-integrated-g2-20261022-2014/`. Accepted retry evidence: `/tmp/opencode/sol-g2-resume-20261025-1736/`, recorded HEAD `a6ae05a`. Reviewed binary SHA256: `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`. The retry reviewer matched all 640 archived Cargo inputs and accepted corrected tools against before/after manifests. This documentation owner changes no Cargo code or tooling. The native/application evidence remains old-version evidence within the layered G2 acceptance. Neither that decision nor E1 contract acceptance upgrades this state or requalifies the old binary.

The prior checkpoint was published as platform **`a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`** (13 paths), functional **`ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93`** (53 paths), QA **`805a8b0f1c8388548f0c8adf869aa28c0e70b926`** (23 paths), and docs **`a6ae05a10bfefdcea8dae75cd29597c84a3fadcd`**, `docs: record paused durability checkpoint and handover` (27 paths). User/coordinator report final push/remote equality and a clean startup tree before resume-ledger changes. Baseline f54d298 remains preserved. These publication facts do not complete G2/G3 or establish today's cluster state. The user resumed work separately on 2026-10-25.

`current/sol-release.md` reports the ten-path QA milestone normally **COMMITTED/PUSHED** as **`dd4cc57ac1b0c44b60cf33e3ec88bfa539b9c736`**, `qa: verify mounted recovery with persistent history oracles`, on preserved generator parent `85a37c8d615992b4600d7467fdedd514fecfac03`. Exact approved bytes/modes and local/tracking/actual remote equality matched; ahead/behind was 0/0 and the index was empty at publication. That snapshot left 62 unstaged primary paths (41 modified/21 untracked); later ledger creation can change counts. The future fault-controller proposal was excluded. Remaining source/docs are not all published; the all-changes request is incomplete. Only the release owner may publish later authorized milestones. This owner makes no fresh Git claim.

Retained E1 **r1** evidence is pinned to `7bfce91702deee435a84d3c66c6cb9c65534d99d595a51b852de487da93c7382`. Isolated work root `/tmp/opencode/sol-e1-shared-20261025.0lt2912d/source` now has narrow r4 work assigned; active outputs were not inspected. Retained r3 preimages/reports/archives stay preserved. Primary E2 is unaffected; declared-frozen E2 preparation is **not archived E2 review proof**. Original3f421 author212+doctest/strict/fmt and findings remain preserved: P1 E1X-1 parser peak, E1X-2 captured-context budget, E1X-3 owned-waiter selection and STATE-1 acknowledged-WAL checked-replay failure; P2 STATE-2 receipt incarnation reuse, STATE-3 overflow and STATE-4 contradictory completion. R1 author233+doctest/probes/post-freeze74 retain their evidence scopes. R1 **STATE-1–4 SCOPED ACCEPT** retains65 focused/adapted11/11/seven probes/overlapping release overflow, strict/fmt/release build and2,092/199 pins; unchanged original10/11 remains a failure. R1 **unmerged corrected-store-dependency SCOPED ACCEPT** retains74/three validation/two config probes and2,092/685 pins at engine7bfce917/storebadbc7e2, UPG1 closed. Old3f421 store74/strict/fmt/two-probe proof and earlier provisional70/author74 results remain unchanged. Retained execution-r1 **CHANGES REQUIRED** records P2 E1X-WAKE-1/2; original E1X-1–3 reproductions were fixed. The completed r2 review closes those wake findings without relabeling old results.

**Completed r2 execution review is CHANGES REQUIRED:** P2 E1X-R2-OVERFLOW loses new/already-owned CloseSession and later dirty controls after callback Capacity; P3 E1X-R2-COMPAT clears another continuation's reservation during compatibility REPLAY. Seeded debug/release each2 pass/3 fail; public3 pass/1 fail. Independent251 engine tests+doctest/strict/fmt pass. Original WAKE-1/2 close; the reviewed four-file delta/seven fresh state probes preserve STATE-1–4. Retained manifest `131ae170c7c4436e1cecb1f6925228998535c357235d2b68fa7e2dad8e8400e1`/map09cbb464 and handoff `/tmp/opencode/sol-e1-shared-20261025.0lt2912d/engine-e1-wakes-20261026-r2/HANDOFF.md` keep their pins. Author overflow/unmerged74, unchanged-wake2/1 versus adapted3/3 and2,171 archived r1 records keep their scopes. Startup2,327 records matched; final strict2,326/2,327 remains failed, solely assigned consumer-plan47e1b384→7edcc062. Source/other evidence stayed stable; full hashes are in `progress_ledger.md`. The completed r3 decision does not relabel these failures.

**Completed r3 execution review is CHANGES REQUIRED** for P3 E1X-R3-COMPAT-CLOSE at0ca06283/map31814a0a. Compatibility close changes a later cap/range/inode version after Capacity without a record. Public Journal epoch4 and exhausted-counter probes fail in both modes. Exact public4/overflow5 close both r2 findings;259+doctest/strict/fmt/release/STATE7/ownership5 and2,515/2,278 pins pass. Consumer section9 and prior wake/STATE scopes remain valid. Original isolated author owns narrow r4 in NEW `current/sol-engine-e1-compat-close-r4.md`; no API/store/daemon change is assigned. Preserve r3 preimages and both red probes. Active r4 stays unread; no full E1 acceptance/promotion/D5 permission follows.

Fixture **SOURCE** acceptance in `current/astra-e1-fixture-merge-review.md` remains pinned to `/tmp/opencode/sol-e1-fixture-merge-20261026.lz4yvr6q/`: two hunks/20 assertions, independent1 test R1-only, full stable.rs2cb747b0/patch7e6540e5 unchanged. Prior r2 joined dependency/API decisions and all failures keep their scopes. PRIMARY `current/astra-e1-r3-joined-review.md` **SCOPED ACCEPTS final r3 STORE+FIXTURE DEPENDENCY** at `/tmp/opencode/astra-e1-r3-joined-review-20261026.ah72nhxn/`: one actual74 includes fixture/UPG1; strict/fmt/two config probes pass;655 inputs/2,515 r3 records stable;68-path reference map6b4bb67e. Public API/persistence source identity is unchanged, not full engine or automatic r4 acceptance. Initial quota link ran zero harnesses without a durable Cargo exit; setup/packaging stops and failed20-byte archive stay retained. Exact maps/usable verified archive are canonical. Consumer section9 remains conditional interface acceptance. R4 execution acceptance, dependency reconciliation and coordinator code assignment precede promotion/D5; prior reports stay immutable.

**Source publication COMMITTED/PUSHED, 2026-10-26:** `18e1b10c241a30ae70807540495bcde46c8dfdf2`, `build: clear remaining strict Rust lint warnings` (6 paths), then `f3fabfdfe52d44ebc2918690bf1ddbd8e48cbff4`, `mds: gate recovery controls and bind reclaim output` (32 paths); union34/overlap4. The dd4cc57 parent and all earlier history remain preserved. `current/astra-source-split-review.md` SCOPED ACCEPTS the split; the temporary cfg(test) attribute exists only in the intermediate commit and commit2 removes it. Normal sign-offs/hooks/push passed. At00:37 UTC the release owner verified actual remote/local/tracking f3fabfd,0/0,empty index, all1,802 committed entries matching the approved candidate and649 working Cargo inputs stable. Intermediate524/0/2 five-command and final612/0/2 six-command proofs were reused without test/formatter reruns; four optional bodies/two ignored helpers remain. The failed initial intermediate and586/1/2 candidate stay failed. Frozen25-file publication proof and checksum manifest are recorded in `progress_ledger.md`. Last remaining inventory was16 modified+25 untracked=41 unstaged paths; QA work can change counts. Docs/ledgers/proposals/newQA/isolated E1 were excluded; all-changes publication remains incomplete. These are release-report facts, not fresh Git observations here.

QA's native platform probe passed on existing data pool 87; independent review accepted retained evidence at `/tmp/opencode/sol-platform-live-20261022.YcfShl/`. It verified get_version/checked map wait at epoch 402, successful and rejected CMPXATTR/ASSERT_VER compounds (-125/-34) with unchanged data/tail/version after rejection, and a complete all-namespace scan: 8 pages, 6 empty continuations, all eight PGs across three primaries, both sentinels found and independently confirmed absent after cleanup. Native sources/manifests stayed unchanged; only an unrelated store session file changed. Filesystem 43/pools 86/87 stayed unchanged, with 17 PGs active+clean. This completes scoped platform native validation, not bootstrap, format hygiene, durability or any G1-G3 gate. Live map-change and nonempty-locator coverage remain untested.

QA restored packaged C++ MDS startup with an explicit MDS keyring. C++ kernel and Ganesha/NFSv4.1 smoke passed. The graceless Ganesha helper does not qualify recovery/reclaim. The canonical evidence table in `progress_ledger.md` records exact commands, artifacts, peer versions and limits. Authoritative Rust baseline logs are `/tmp/opencode/sol-g0/f54d298-*`; old `immutable-*` and shared-target evidence are invalid. Independent review checked archived source identity and counts, but did not rerun live checks.

- `ceph-rmds -i rust` authenticates to the Ceph 20.2 monitor over msgr1 with cephx and gets the rotating `mds` secrets. It beacons and goes standby → `up:creating` → `up:active` as rank 0 of file system `a` in about a second.
- A kernel 7.0 client mounting without `ms_mode` and ceph-fuse both do full I/O: 8 MiB write/read-back after `drop_caches`, rename, hard link, readdir, unlink, rmdir, cross-client visibility.
- Getting there needed three kernel-visible correctness fixes (decision 2026-10-13 in `design/minimum-feature-set.md` 8.1): mutation replies must drop `Fs Fx` from the parent directory's cap (the creator keeps them), traced replies must not re-grant a cap's liked set once issued, and OPEN replies must trace the final dentry. Without them the kernel answers lookups from stale negative dentries after cache eviction.
- Progress is tracked in [`progress_ledger.md`](progress_ledger.md); keep it current.

Historical fresh-filesystem prototype procedure (QA only; not a recovery test):

```sh
source devcluster/env.sh
cargo build -p rmds
./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring $RMDS_KEYRING &
ceph fs dump | grep mds.rust            # mds.rust{0:...} state up:active ... addr v1:127.0.0.1:...
devcluster/mount-kernel.sh /tmp/m v1    # historical prototype supports metadata I/O
kill %1; devcluster/newfs.sh            # stop; recreate fs "a" so the next MDS gets up:creating
```

**Historical base f54d298 only:** the daemon used `Engine<MemJournal>`, store provided codecs only, and replay created an empty namespace. The historical ledger also records 8,769 pjdfstest passes, 28 TODO failures and one Y2038 client-wire case. These statements do not describe the new production persistence implementation or establish its qualification.

## 2. Repository map

| Path | What |
|---|---|
| `README.md` | The architecture proposal (Igor Golikov), plus the repository contents table |
| `design/minimum-feature-set.md` | Minimum feature set: tiers P0/P1/P2/OUT, working assumptions D1-D18, per-area requirements, invariants, the **decision log (8.1)** and the day-one design hooks (5.1) |
| `design/minimum-feature-set.json`, `design/minimum-feature-set-catalog.md` | All 935 catalog entries classified, with reasons (generated; regenerate from the classification JSON, do not edit by hand) |
| `design/workplan.md` | Current G0–G7 gates. The first cycle ends at G3. G5 proves fine-grained transactions |
| `design/store.md` | Selected stable guarded WAL. Later identity: a shard is not a subvolume |
| `design/transactions.md` | Isolation, visibility, locks and per-stream commit tokens. Normative for G5 |
| `design/persistence-contract.md` | First-milestone engine/store/daemon interfaces and failure rules |
| `design/recovery-contract.md` | Both E1 contract scopes ACCEPTED (R4/U1/U2 closed); prior E2/D6/wire REPLAY acceptance unchanged. Not implementation or live-upgrade evidence. |
| `design/build-brief.md` | Conventions and decisions the build followed |
| `mds-rust-map/` | Reference set, facts only: map of the C++ MDS and its peers, feature catalog. Start at `mds-rust-map/README.md` |
| `crates/` | The Rust workspace (section 3) |
| `devcluster/` | Scripts for the single-node Ceph 20.2 dev cluster (section 4) |
| `testdata/` | Real traffic between the C++ MDS and kernel/ceph-fuse clients and the monitor, maps, ceph-dencoder type list (`testdata/README.md`) |
| `scripts/smoke.sh` | Historical passing prototype gate; QA must rerun: rank 0 active, kernel/FUSE mounts and I/O |
| `tools/gen_features.py` | Generates `crates/denc/src/features.rs` from Ceph's `ceph_features.h` |
| `.env.example` | Variables of the git-ignored `.env`: the dev cluster's cephx keys at capture time, for the golden tests (section 4) |

## 3. Crates

Build and test everything: `cargo test --workspace`. Historical counts below are marked; current package evidence belongs in `progress_ledger.md`, not a combined-tree pass claim. Some golden tests need `/usr/bin/ceph-dencoder`, three need capture keys in `.env`, and live examples need the dev cluster. Use a separate `CARGO_TARGET_DIR` per agent. QA records actual commands, skips and results.

| Crate | Status | Tests | Notes |
|---|---|---|---|
| `denc` | done | 8 | Encoding traits, framing, `EntityAddr` (legacy and ADDR2), `EntityName`, `UTime`, `Uuid`, feature constants, message envelope (`RawMessage`, `Message`) |
| `auth` | done | part of 54 | cephx: AES (type 1) and AES256KRB5 (type 2) keys, keyring, monitor handshake, service tickets, authorizers (build and verify, with the CEPHX_V2 challenge), rotating secrets, msgr1 message signatures |
| `msgr` | done for msgr1 | part of 54 | tokio messenger: msgr1 client and server, lossless server and lossy client policies, signatures, keepalive, reconnect. msgr2 not implemented; the protocol sits behind internal frame traits for it |
| `monc` | prior package and private alias ACCEPTED | 24 retained tests verified; independent strict focused clippy | MonClient, monitor maps/beacons and configuration. Alias expands to the unchanged private type; no workspace-green inference. |
| `proto` | done | 80 | Every CephFS client message (sessions, requests, replies with InodeStat v9, caps v12/v14, reconnect v3/v5, snap, quota, lease, reclaim, metrics). Byte-identical decode and re-encode of all captured traffic; ceph-dencoder cross-checks; fuzzed |
| `engine` | prior packages and E2 ACCEPTED | E2: 185 + doctest, three independent probes | Wire-free state, one-record step protocol, committed output, checked restore/replay and E2 control/grant boundaries. Strict crate clippy passes; no E1 source or combined-workspace claim. |
| `osdc` | platform ACCEPTED; scoped native probe passed | 70 + doctest independent | OSDMap/CRUSH/Objecter, checked compounds, bounded native all-namespace listing and checked map waits. |
| `store` | primary packages and cfg(test) E2 fixture ACCEPTED | fixture: 50 verified author tests; one independent focused test | Layered G2 accepted. Final r3 merged dependency SCOPED ACCEPT: one actual74/strict/fmt/two config probes,655 stable inputs. Prior r2/unmerged/R1 focused proof retains its scope. R4 execution/dependency reconciliation/promotion remain pending. |
| `mds` (`ceph-rmds`) | D1–D4/clientless E2 and D6/R1/six-site cleanup ACCEPTED | latest: 160 independent tests, strict crate clippy/build/fmt | 132 source hashes stable. Pre-R1 fault review: 4/11/147 overlapping counts. Fresh workspace612/0/2 supports layered G2 acceptance; nonempty D5 stays held. |

### 3.1 Engine and OSD client

**engine:** accepted offline packages, including E2. Author and independent test scopes differ; see the canonical evidence table. E2 strict crate clippy passes. Fresh corrected workspace strict Clippy passes without warnings; the historical workspace strict-lint failures remain separate results. Isolated E1 corrections are not adopted.

An I/O-free metadata engine for one shard. It has no dependency on wire crates. The daemon translates its inputs and effects. Its API is documented at `crates/engine/src/lib.rs`.

- **Production entry points:** `Engine::step` consumes `EngineInput` and a one-record `RecordPermit`; `journal_event` materializes committed output before another admitted step. Checked checkpoint/restore/replay and session-key installation preserve durable state. Direct legacy request/cap/close helpers are not the daemon production path.
- **Effects:** client replies/caps/realm notifications and recall/eviction progress stay ordered through the session funnel. Durable lifecycle work uses committed job state and returns completion through a new admitted engine transaction.
- **Covered:**
  - all P0 namespace operations with the C++ error order and their effects on times, change_attr, nlink, dirstat and exact rstat;
  - hard links, with promotion of a remaining link to primary;
  - unlink-while-open through strays;
  - inode-number range leases, with an API to delegate ranges to a session;
  - readdir in rjenkins hash order, checked against Ceph's own hashes, with paging;
  - xattrs and the `ceph.*` vxattrs (quota opens a realm and emits SPLIT; EXDEV across subvolumes);
  - fcntl and flock locks;
  - the caps model (two-step revoke and ack, flushes with FLUSH_ACK after commit, `max_size` journaled before it is granted, stale sessions and eviction);
  - one self-contained journal record per mutation, with replies only after commit;
  - retries answered from the completed-request table;
  - replay;
  - a randomized multi-client stress test that recomputes statistics and checks replay.
- **Open problems:**
  - dentry leases (always null), MClientQuota pushes, MDS auth-cap checks, snapshots, early replies and async dirops are not implemented (the hooks are in place);
  - current persistence uses one root shard/stream, full ancestor images and ordered replay; multiple streams/deltas are later gates;
  - production admits one record at a time; it prioritizes a simple committed-visibility proof over concurrency;
  - creator caps differ slightly from C++ (no `max_size` in an MKNOD reply, no `Ax`/`Xx` on read-only creates or new directories), which costs an extra round trip;
  - the readdir byte budget is an estimate;
  - the engine has no clock: the daemon must fill a zero request stamp.

**osdc:** accepted platform package and separately accepted retained native probe evidence. The historical base `rados_live` workload authenticated as `mds.rust` over msgr1 and covered:
- write_full, read, stat;
- exclusive create (`-EEXIST`);
- xattrs, omap (paging, header, by keys, rm, clear);
- CMPXATTR (`-ECANCELED`) and ASSERT_VER (`-ERANGE`), the ownership-guard primitives of `design/store.md`;
- trimtrunc, append, zero, remove.

What it covers:
- **API:**
  - `OsdMap` (`decode`, `apply_incremental`, `pg_to_up_acting_osds`, `map_object`, `is_blocklisted`) and `CrushMap::do_rule`;
  - `MOsdMap`, `MOsdOp`, `MOsdOpReply`, `MOsdBackoff`, and the `ObjectOperation` builder;
  - `Objecter::start(transport, ObjecterConfig { client_inc, .. })`, with `operate`/`submit`, `write_full`/`read`/`stat`/`remove`, `handle_osd_map`, `set_epoch_barrier`, `wait_for_map`;
  - events: `RequestMap`, `CheckLatestMap`, `MapApplied`, `Blocklisted`, `Fenced`;
  - `MsgrTransport::new(messenger)` binds it to the daemon's single messenger, and `file_object_name`.
- **Golden checks** (fixtures in `crates/osdc/testdata`, each with its regeneration script):
  - CRUSH matches `crushtool --test` for every bucket algorithm, firstn/indep, MSR rules, five tunable profiles, device classes and choose_args;
  - 130 OSDMap epochs from two histories: applying incrementals reproduces every full map exactly;
  - placement matches `osdmaptool --test-map-pgs-dump-all` and `ceph osd map`;
  - 46 MOSDOp/MOSDOpReply messages captured from librados re-encode byte-identically, and ceph-dencoder accepts our v8 and v9 encodings.
- **Mock tests** (17) for the Objecter:
  - resend on primary change, PAUSERD/PAUSEWR, FULL ignored (FULL_FORCE on every op);
  - epoch barrier, pg split, `-EAGAIN`/redirect, `-EBLOCKLISTED`, backoff;
  - resend in tid order after a reset, laggy pings.
- **Bug found and fixed:** `reqid.inc` must carry the MDSMap incarnation (`client_inc`). Otherwise a restarted MDS could receive the stale results of the previous instance's requests.
- **Open problems:**
  - `CheckLatestMap` now uses bounded native monitor version queries and explicit map-wait completion; live MapChanged listing and nonempty locators remain unqualified;
  - OSD connections use only the OSD's msgr1 address;
  - watch/notify, balanced reads and EC direct reads are not implemented;
  - helpers implemented locally that belong in `denc`: varints, `f32`, raw crc32c, the legacy-compat frame check, and `Ord` for `EntityAddr`.


### 3.2 Open problems reported per crate

- **auth/msgr/monc:**
  - msgr2 is missing;
  - ticket renewal and rotating-secret refresh are implemented but not yet observed live (needs a run of more than 45 minutes);
  - the MonClient hunts one monitor at a time;
  - MMonCommand/MonClient::command exist; MLog and MgrClient remain work;
  - AES256KRB5 is untested against a live cluster that uses it.
- **proto:**
  - reconnect, lease, snap, quota and reclaim messages are checked only through ceph-dencoder, since no real traffic was captured;
  - we send InodeStat v9, caps v14 and SnapRealmInfoNew v2, where Ceph 20.2 sends v8, v12 and v1. Both 20.2 and `main` clients accept ours.

## 4. Dev environment

- **Machine:** Ubuntu 26.04, 32 cores, 125 GB RAM, Rust 1.99, passwordless sudo.
- **Ceph:** the 20.2.0 distro packages (`ceph-mon`, `ceph-mgr`, `ceph-osd`, `ceph-mds`, `ceph-fuse`, `ceph-common` with `ceph-dencoder`, 675 types), plus nfs-ganesha 6.5 and tcpdump/tshark.
- **Ceph `main` source:** `/home/ubuntu/ceph`, at the commit the map was written against. It is not built.
- **Cluster** (`devcluster/README.md` has the details):
  - 1 mon (v2 3300, v1 6789), 1 mgr, 3 memstore OSDs, pool size 3, cephx everywhere, data in `/mnt/workspace/devcluster-data`;
   - filesystem/mount state is controlled by QA; old snapshots are not current observations. Fresh Rust setup is separately authorized before, not inside, an identity-preserving restart trial;
  - `source devcluster/env.sh` exports `CEPH_CONF`, `CEPH_KEYRING`, `RMDS_KEYRING` (entity `mds.rust`, with the caps cephadm gives an MDS) and the monitor addresses;
  - scripts: `start.sh [--mds none|cpp|cpp-v1]`, `stop.sh`, `reset.sh`, `status.sh`, `newfs.sh`, `mount-kernel.sh <dir> v1|v2`, `mount-fuse.sh <dir>`, `umount.sh <dir>`, `smoke-test.sh <mnt>` (I/O checks on an existing mount), `ganesha.sh start|stop` (NFSv4.1 on port 12049).
- **Quirks:**
  - Ceph 20.2.0's `MDSMap::decode` crashes in programs that never call `global_init`. That breaks `ceph tell mds.*`, Ganesha's FSAL_CEPH and ceph-dencoder's MDSMap/FSMap types. `devcluster/gctx-shim.c` (`$GCTX_SHIM`, `LD_PRELOAD`) works around it for the first two.
  - memstore OSDs need `osd_fast_shutdown = false` (already set) to keep data across a restart.
  - `HEALTH_ERR` with no MDS running is expected.
  - ceph-fuse always talks msgr2 to the monitor, but msgr1 to an MDS that only publishes v1.
  - tshark cannot read under `/mnt/workspace` (AppArmor).
  - `/etc/sudoers.d/ceph-smartctl` (from `ceph-base`) makes every sudo call print a harmless warning.
  - The distro's `nfs-ganesha.service` (port 2049), started when the package was installed, is still running. `devcluster/ganesha.sh` uses its own instance on port 12049.
- **Keys:** the throwaway cephx secrets of the dev cluster at capture time (client.admin, mds.a and the rotating mds secret) are in `.env` at the workspace root, which git ignores; `.env.example` lists the variables. Three golden tests use them to decrypt the captured cephx exchanges and skip, with a note, when `.env` is missing. Never commit keys; the cluster's own keyrings stay in `/mnt/workspace/devcluster-data/keyring/`, outside the repository.

### 4.1 Cluster operations

All scripts are in `devcluster/`; `devcluster/README.md` has every option, path and port. Commands below run from the repository root as the normal user (scripts call sudo themselves where needed).

**First-time setup on a new machine** (Ubuntu 26.04, Ceph 20.2 from the distro):

```sh
sudo apt-get install -y ceph-mon ceph-mgr ceph-osd ceph-mds ceph-fuse ceph-common ceph-base gcc
# optional: captures and NFS
echo 'wireshark-common wireshark-common/install-setuid boolean false' | sudo debconf-set-selections
sudo apt-get install -y tcpdump tshark nfs-ganesha nfs-ganesha-ceph
```

`gcc` builds the `gctx-shim` that works around the Ceph 20.2 `MDSMap::decode` crash (section 4); `start.sh` warns if it cannot.

**Bring-up:**

```sh
devcluster/start.sh --mds none   # creates the cluster on first use (new fsid and keys), then starts
                                  # 1 mon, 1 mgr, 3 OSDs; idempotent; no MDS so mds.rust can take rank 0
source devcluster/env.sh          # CEPH_CONF, CEPH_KEYRING, RMDS_KEYRING, MON_V1/MON_V2, FSID, ...
# Fresh filesystem setup requires separate QA authorization. Never run newfs inside a restart trial.
```

`start.sh --mds cpp` brings up the C++ reference MDS `mds.a` instead (msgr2 and msgr1). `--mds cpp-v1` binds it to msgr1 only.

**Run the Rust MDS:**

```sh
cargo build -p rmds
RMDS_LOG=info ./target/debug/ceph-rmds -i rust -c $CEPH_CONF --keyring $RMDS_KEYRING \
    > /mnt/workspace/devcluster-data/log/ceph-rmds.log 2>&1 &
ceph fs dump | grep mds.rust      # expect: mds.rust{0:<gid>} state up:active ... addr v1:127.0.0.1:<port>
devcluster/mount-kernel.sh /mnt/k v1   # kernel over msgr1 (v2 = ms_mode=prefer-crc, needs msgr2 in the MDS)
devcluster/mount-fuse.sh /mnt/f
```

**Status queries:**

| Question | Command |
|---|---|
| Everything at once (daemons, health, fs, mounts) | `devcluster/status.sh` |
| Cluster health | `ceph -s`; `ceph health detail` (`HEALTH_ERR 1 filesystem is offline` is expected while no MDS runs) |
| Who holds rank 0, in which state, at which address; fs compat set | `ceph fs dump` (look for `mds.rust{0:...} state up:active` and `63=rust mds format v0`) |
| Is our key valid | `ceph -n mds.rust -k $RMDS_KEYRING mds stat` |
| Why a beacon or an authentication was refused | `grep -E 'beacon|mds.rust|cephx' $CEPH_LOG_DIR/ceph-mon.a.log` (the mon logs at `debug_mon 10`, `debug_auth 10`) |
| What the Rust MDS is doing | its log (above); `RMDS_LOG=debug` for more; it has no admin socket yet |
| Clients and mounts | `mount | grep -E 'ceph|nfs'`; `sudo dmesg | grep -E 'ceph|libceph'` for kernel-client errors |
| OSDs and pools | `ceph osd tree`, `ceph osd lspools`, `rados -p cephfs.a.data ls` |

**Teardown:**

```sh
devcluster/umount.sh /mnt/k; devcluster/umount.sh /mnt/f   # escalates to umount -f / -l if the MDS is gone
kill %1   # or: pkill -TERM -x ceph-rmds; SIGTERM sends a down:dne beacon, so the rank fails over at once
devcluster/stop.sh    # stops MDS, OSDs, mgr, mon; keeps all data; start.sh resumes it
devcluster/reset.sh   # stop and delete /mnt/workspace/devcluster-data: the next start.sh makes a new
                      # cluster with a new fsid and NEW KEYS (the capture keys in .env stay valid for testdata/)
```

The historical prototype used `devcluster/newfs.sh` between fresh-filesystem runs because replay served an empty namespace. Do not use this procedure for G2/G3: preserve filesystem identity and pools. Only QA controls the shared cluster. OSDs use memstore; keep them alive during MDS-crash tests and stop them cleanly with `stop.sh`.

## 5. Next steps, in order

### 5.1 Historical prototype wiring — DONE 2026-10-13

`scripts/smoke.sh` passes: rank 0 active, kernel (msgr1) and ceph-fuse mounts, full I/O cycle.
The ledger records the pjdfstest tail as done on 2026-10-13: 8,769 passes, 28 TODO failures and one Y2038 client-wire case. Steps below describe historical prototype APIs/policy, not current production instructions. Current step/session/recovery behavior is in section 1 and `crates/mds/README.md`.

1. Hold an `engine::Engine` (created on `up:creating`, with the data pool from the MDSMap; `Engine::set_pools` on map changes).
2. Translate `proto::MClientRequest` into `engine::Request`:
   - op and arguments;
   - `FilePath`s;
   - caller and owner ids, and `stamp`;
   - flags, retry attempt and `oldest_client_tid`;
   - the embedded releases;
   - initial xattrs from the data payload.

   Likewise translate `MClientCaps` into `engine::CapUpdate`, and `MClientCapRelease` into release items.
3. Translate `engine::Effect` back into wire messages, in the returned order, through `Session::send`:
   - `Reply` → `proto::MClientReply` (engine `InodeStat`, `DirFragInfo` and `LeaseStat` → proto types; snap trace in `SnapFormat::for_client(&session.features)`; readdir, create, filelock and getvxattr extras);
   - `Caps` → `MClientCaps`;
   - `RealmSplit` → `MClientSnap`;
   - `Recall` → `RECALL_STATE`.
4. Before sending a reply, check `MClientReply::check_invariants_for(&req)` and log violations (design section 7 lists the client crash triggers).
5. Sessions: `Engine::open_session` / `close_session` from `session_opened` / `session_closed`. Add renewal staleness and autoclose timers (`Engine::session_stale`/`session_resume`; `session_timeout` 60 s and `session_autoclose` 300 s from the MDSMap).
6. `cap_auths_for`: parse `Connection::peer_caps()` (the MDS cap string) into `MdsCapAuth`. An empty list is acceptable while testing as `client.admin`.
7. Deny `MClientReconnect` with `CLOSE` until persistence exists. Answer `MClientReclaim` with result 0 and no address (Ganesha treats that as ENOENT).
8. Run `scripts/smoke.sh`, then pjdfstest on a kernel mount.

### 5.2 OSD client and current durable data-work boundary

The Objecter uses the same messenger identity as monitor/client traffic (`MsgrTransport::new(msgr)`). It follows subscribed OSDMaps and takes `client_inc` from the MDSMap rank incarnation. Self-blocklist/fencing is fatal. Reviewed `rados.rs` Result primitives use Ceph striping and check OSD errors; current durable workers, not log-only compatibility wrappers, call them:

- `PurgeInode` deletes objects `<ino hex>.<objno>` 0..N in the layout pool (`-ENOENT` ignored); verified end-to-end: the data pool is empty after a smoke run's unlinks.
- `TruncateData` sends `TRIMTRUNC` (object-local size, `truncate_seq`) to every object overlapping `[size, from)`.
- Size recovery retains durable ranges/layouts and applies size/mtime only through a checked completion transaction. Failed attempts retain intents.
- Eviction fences the durable peer-address union, confirms exact applied blocklist entries, raises the epoch barrier, then performs durable cleanup. Failed fencing never releases rights.
- Four lifecycle attempts may run at once. Each attempt preflights at most 4096 candidate object operations and 64 layouts. Oversized/failed work remains durable with capped retry delays. No data work runs before recovery/service readiness.

### 5.3 Resumed coordinated build cycle: G0-G3

Follow `design/persistence-contract.md`. The user selected one stable guarded WAL object per stream and immutable checkpoints. Start with root shard 0, stream 0, full images and ordered replay. The control object is ownership authority; the shard map is only a routing projection.

1. G0 partial: contract, corrected partial QA tooling and scoped native platform validation ACCEPTED; isolated baseline and repaired C++ reference evidence recorded. Fresh corrected combined checks, including strict workspace Clippy, passed without warnings. G2 acceptance does not automatically pass G0; GitHub CI execution is unverified. The canonical evidence table is in `progress_ledger.md`.
2. G1: safety packages accepted and combined offline QA passed at the recorded source snapshot; no separate full-gate completion declaration is made here.
3. G2: coordinator ACCEPTS the conservative root-shard STORAGE-RESTART gate on retained native compound/bootstrap/application evidence, independently closed A/B fault families, accepted primary source and fresh corrected612/0/2 combined checks. The written matrix requires no additional mandatory native fault series. The original ID323 trial and superseded586/1/2 workspace candidate remain failed. This is layered acceptance, not a fresh current-binary native PASS/query, positive native symlink proof, E1 adoption or G3 mounted same-wire-tid qualification. G0/G1 are not automatically passed.
4. G3: receive frozen engine r4/P1–P3 r2 corrections and assign independent delta reviews. Reconcile final engine dependency pins and scoped consumer API before authorized promotion/startup-D5 assignment. Receive sole QA's unchanged256 verification before assigning FC residual source corrections; they remain NOT YET ASSIGNED. Both old256 runs remain FAILED, not fresh166-baseline proof. Active author/QA outputs stay unread; do not poll verification here. P4/P5/probes/hooks stay held/dependent; native construction disabled, full A4/B3/R/N and G3 open. No live upgrade/action is authorized.

Source publication is complete at `f3fabfdfe52d44ebc2918690bf1ddbd8e48cbff4` after lint18e1b10, not blocked or pending. Remaining docs/ledgers/proposals/newQA/isolated E1 need later review and publication authorization; no automatic publication follows. All-changes publication remains incomplete. This owner makes no fresh Git claim.

The native daemon uses the failure-aware journal and step path. This status-only assignment changes no code/tooling, accepted semantics/APIs or reviewer reports. It runs no tests/builds/live actions/Git commands. The accepted recovery contract is not E1 source, native upgrade or G3 evidence. Sole QA owns any later live trial under explicit assignment and fresh safety checks. Never serve speculative reads after failure or replace missing recovery state with an empty namespace. Preserve both acknowledged namespaces, original/retry evidence and filesystem identity.

Current defaults (`crates/mds/README.md`): Objecter actor timeout 30 s; checkpoint interval 30 s or 16 committed records; maximum framed record 1 MiB; retained WAL budget 64 MiB; retained checkpoint-generation limit 64. SIGUSR1 requests a quiescent immutable checkpoint. There is no checkpoint garbage collector. Capacity pauses ordinary admission; recovery is an explicit budget increase and same-identity restart, never metadata deletion. Initial inode-range allocation has no dynamic refill worker. Active messenger outbound backlog has no capacity feedback, so no end-to-end outbound memory bound is claimed.

Independent re-review ACCEPTED all five contract amendments. They cover persisted predecessor addresses/fencing, native bounded pool listing and format-object guards, rank-session acquisition/recovery join, one-record engine step/resume with durable cleanup completion, and WAL persistence of stray path/stable subvolume identity. Keep these normative interfaces stable. Native fresh bootstrap now has scoped accepted evidence; concurrent initialization, broader failure injection and complete G2/G3 qualification are not implied.

### 5.4 After that

Follow `design/workplan.md`:
- G4: subvolume tenancy (`SubvolumeId` is not a shard), authorization and bounded indexed state;
- G5: measured concurrency ([`design/transactions.md`](design/transactions.md)) and dependency-safe multiple streams;
- M0b: msgr2 after G1 platform interfaces settle;
- G6: multiple ranks and migration;
- G7: operability and release qualification. Build earlier dependencies when needed.

Open decisions are in `design/minimum-feature-set.md` section 8, items 10-12, including the project license. Items 8 and 9 have a direction as of 2026-10-28.

## 6. Rules that matter

- **License:** the kernel's `net/ceph` and `fs/ceph` are GPL-2.0. Read them to understand client behaviour, but never translate their code. The C++ Ceph sources (LGPL) and `mds-rust-map/` are the code references.
- **Decisions are logged:** `design/minimum-feature-set.md` 8.1. Add new ones there.
- **Day-one hooks** (design 5.1) must stay intact; they are what keeps later features from requiring a rewrite:
  - connection reading is separate from execution;
  - every client message goes through the per-session funnel;
  - WAL records are self-contained and versioned;
  - the clientreplay phase exists;
  - the allocator can delegate inode ranges;
  - shard ownership is checked on append, checkpoint publication, trim and later handoff. The stable control object is the authority (`design/store.md` sections 2 and 3).
- **Client invariants** (design section 7): a violation crashes or hangs unmodified clients. Examples: a forward for a tid that already got a reply, a wrong dentry-trace parent, directory `nlink` other than 1, trailing bytes in replies, a health metric with id 0.
- **Golden data first:** check encodings against `testdata/` captures and `ceph-dencoder` before trying them on a live client.
