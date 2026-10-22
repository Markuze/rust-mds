# Rust MDS paused checkpoint

**PAUSED — 2026-10-22. Wait for explicit user resume.** All current implementation/review assignments finished. Do not start implementation, tests, live operations, G2 retry or G3. Only handover preparation/independent review and the previously requested commit/push checkpoint remain authorized. This file records state; it does not authorize its commands now.

## 1. User requirements and read order

- The parent only orchestrates. Sol agents implement; separate Sol agents review. Use exact model/variant **`openai/gpt-6.1-sol-ultrafast#max`** for every future worker/reviewer continuation or invocation.
- Preserve independent review, disjoint file ownership, per-agent Cargo target directories and one explicitly assigned live QA owner. Do not spawn workers or resume implementation automatically from this handover.
- User requested **commit and push all intended changes**. Confirmed source/QA publication and the final branch-HEAD documentation checkpoint are recorded in section 7. The release agent's final report supplies final remote/clean-tree confirmation; do not infer it from the platform commit alone. Never include ignored keys/build output or force-push/reset user work.
- Latest user request: **"when the agents fininsh pause progress and create a handover document"**. Finishing release does not resume implementation/live work.

Repository: `/mnt/workspace/rust-mds`. First read `AGENTS.md`, this file, `progress_ledger.md`, then `current/sol-orchestration.md` (assignments and continuation handles). Read `design/persistence-contract.md`, `design/store.md`, `design/workplan.md` and `design/minimum-feature-set.md` before changing design/code. Contracts are accepted and unchanged. The original `design/build-brief.md` ownership table is historical. External review plan: `/home/ubuntu/.opencode/plan/rust-mds-design-review-workplan.md`; the repository contracts/gates control implementation.

## 2. Implemented and independently accepted

One rank, root shard 0, stream 0, scalar sequence, full images and ordered replay. Selected storage is a stable guarded WAL object with bounded omap batches plus immutable **materialized** checkpoints. This is not a MemJournal façade.

- `engine`: wire-free/I/O-free metadata state. `EngineInput` / `RecordPermit` / `EngineStep` admits at most one record. Explicit `JournalEvent` releases committed output before another step. Failure stops service. Checked restore/replay preserves linkage, allocator provenance, durable session keys/histories/cleanup receipts, stray authorization path, stable subvolume identity and complete lifecycle jobs.
- `store`: schema **2**. Native Objecter backend; owner/tail-guarded WAL batches, exact uncertain-outcome retries, checked frozen-tail pagination, Preparing/Ready bootstrap, separate authority acquisition, allocator reservation and durable rank sessions. Immutable checkpoint write -> guarded manifest publication -> bounded trim. Missing/foreign/corrupt recovery state fails closed.
- `mds`: production `Server<DaemonJournal>` owns `Engine<DaemonJournal>`. Native FSMap discovery, persisted predecessor fencing, finite Objecter actor timeout, tagged serialized storage worker, durable OPEN/Closing/cleanup-receipt/Closed ordering, validated transport attachment and bounded admission. Lifecycle results commit through engine steps. Clientless recovery retains monitor-confirmed rejoin. Daemon MemJournal compatibility paths are test-only.
- `monc/osdc`: native get_version, checked map wait/results, guarded compound result cardinality and bounded native all-namespace PG listing. `rados.rs` uses reviewed striped purge/truncate/probe Result primitives.
- QA tools: complete G2 clean-restart **application harness** and intentionally partial G3 foundation. Both directory-oracle false-passes were fixed. Boot UUID + kernel 64-bit UNIQ-ID now proves mount identity; no legacy-ID fallback. Normal unmount, `noshare` and no-old-mount audits remain required.

**G3 is not implemented/qualified:** nonempty restored Open-session gathers and bounded reconnect input remain held. Do not run mounted-client failover merely because its partial harness exists. The next G3 work must reconstruct mixed v5 caps/realms, locks, session gather/eviction and recovery replies before ordinary admission.

### Independent reports

| Boundary | Report / accepted scope |
|---|---|
| Persistence contract | `current/sol-contract-review.md`: all five contract findings closed. |
| Engine visibility/admission | `current/sol-engine-review.md`: fixes accepted; three probes and 37 focused tests pass. |
| Engine recovery state | `current/sol-engine-state-review.md`: 42 focused tests/probes pass; schema 2 compatible. Author separately reports 175 engine tests + doctest. |
| Store core | `current/sol-store-wal-review.md`: corrected short OMAP pages accepted; independent 50 tests and store-only strict clippy pass. |
| Store authority/schema | `current/sol-store-state-review.md`: accepted; later core review supplies requested bootstrap pagination regressions. |
| Platform/data | `current/sol-platform-review.md`, `current/sol-rados-review.md`: scoped API/native evidence and striped operations accepted. |
| Daemon | `current/sol-daemon-store-review.md`: implementation and scoped native evidence accepted; earlier 71-test run. `current/sol-daemon-review.md`: runtime corrections accepted, 11 focused + 82 full tests/build pass. |
| QA/oracle/mount identity | `current/sol-verification-review.md`: harness/oracle corrections accepted; latest **21 tests normally and optimized** pass independently. Author reports **83 tests in each mode**, not an independent full-suite rerun. No live retry. |
| G3 protocol facts | `current/sol-recovery-review.md`: mandatory rejoin, mixed v5 chunks, prior-owner proof, gather and reclaim acceptance checklist. |

## 3. Exact gate truth and provenance

- **Accepted modified-tree QA:** 524 passed, 0 failed, 2 ignored including doctests; daemon build, fmt check and default workspace/all-target clippy pass. Strict clippy exits **101** on 46 generated denc `identity_op` diagnostics and engine `for_kv_map` / `collapsible_if`. Default clippy retains warnings. Optional kernel-v1-mdsv1 capture/corpus is absent; dump/extract_corpus are ignored. Do not claim strict CI green.
- QA ran the combined checks. Independent retained-evidence review recounted 42 test groups and checked the source/binary/evidence hashes; it did **not** rerun native tests. This is the accepted modified source, not pristine baseline `f54d298` (whose isolated historical count is 347/0/2).
- **Native subclaims PASS:** separate fresh Rust empty-pool setup reached Ready/active; clientless restart reached replay -> reconnect -> rejoin -> active with zero Open sessions; same-filesystem kernel/FUSE 8 MiB read/write and cross-client smoke passed without reset. Native publication/trim observations are accepted.
- **Complete G2 NOT PASSED:** the first private-namespace trial recorded 17 acknowledged operations and 10 inode identities, cleanly unmounted, restarted and remounted. Linux reused legacy mountinfo ID **323**. The controller exited **1 before the saved-state comparison or 16 future-inode nonreuse checks**. There is no G2 PASS/result artifact. A prior host-namespace attempt exited 2 before workload because mounts propagated to peers. Neither result becomes a pass after fixing tooling.
- Unique-mount correction is now independently **ACCEPTED**, but no corrected live retry ran. It requires `(host boot UUID, nonzero kernel u64 UNIQ-ID)` and fails closed on missing/malformed support. The pause, not an outstanding correction review, now prevents retry.
- No full G1-G3 gate completion is declared. Complete G2 application comparison, broader storage fault/uncertainty tests and full G3 remain open. Normal smoke does not substitute for acknowledged-history equality.

Reviewed binary: `/mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds`.

SHA256: **`330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`**.

640 Cargo source/manifest inputs match the accepted QA/release snapshots. Platform publication changed HEAD, not their tested contents. The later unique-mount tooling correction has separate offline evidence and did not rebuild/change the daemon or accepted oracle.

### Evidence locations (host-specific, not claimed committed)

| Location | Contents/retention |
|---|---|
| `/tmp/opencode/sol-integrated-g2-20261022-2014/` | Accepted source archive/manifests, combined test/build/fmt/clippy logs, binary hash, native raw control dumps, original failed G2 history/restart/error and durable smoke. **Retain unchanged.** |
| `/tmp/opencode/sol-unique-mount-correction-20261022/` | Unique-ID correction source/test evidence. Retain with the independent report. |
| `/tmp/opencode/sol-g2-enumeration-coverage-20261022/` | Earlier accepted namespace-oracle correction evidence. |
| `/tmp/opencode/sol-platform-live-20261022.YcfShl/` | Earlier accepted native version/compound/listing/sentinel cleanup probe. |
| `/tmp/opencode/sol-g0/f54d298-*` | Isolated historical prototype baseline only. `immutable-*`/shared-target results were invalidated; never promote them. |
| `/tmp/opencode/sol-release-platform-20261022.chxm820v/` | Isolated platform-only commit candidate and release hash evidence. |

These absolute paths require the original host. `/tmp` data and the reviewed target binary may disappear. Verify availability/hash after resume. If missing, stop and re-establish equivalent reviewed provenance through assigned owners; do not silently rebuild, recreate the filesystem or claim the old evidence was committed. No keys belong in artifacts.

## 4. Preserved cluster and limits

Cluster fsid **`3ceca2c6-851f-41cf-b538-1cfded4f1e82`**; filesystem `a`, **fscid 44**, metadata pool **88**, data pool **89**. Preserve all three identities. **No newfs/reset/pool recreation for retry.** Memstore OSDs must stay alive; this evidence qualifies MDS behavior, not host/OSD power-loss durability.

Last reported state, not a new observation: no MDS or CephFS/NFS test mounts; original mon/mgr/three OSDs alive, 17 PGs active+clean. QA positively identified and removed the old unmounted FUSE child. System Ganesha remained unchanged. Expected filesystem-offline/failed-rank health errors occur with no MDS. Final control: owner epoch3/incarnation342/gid16577, tail **83**, checkpoint generation **7**, coverage/trim **82**, remaining WAL key83. Do not infer exact replay counts or equality from positions.

Preserve original acknowledged namespace **`restart-history-1629333-1792700434474727536`** and all its data. It is referenced by the retained failed-trial manifest/history. A retry uses a new unique namespace/artifact directory; it must not delete or overwrite earlier history.

Known limits: one shard/stream; no checkpoint GC (64-generation default can pause admission), no dynamic allocator lease refill, no active-messenger queue feedback/end-to-end outbound memory bound. Defaults: 30 s Objecter actor timeout; checkpoint every 30 s or 16 records; 1 MiB max framed record; 64 MiB retained WAL. SIGUSR1 requests a quiescent checkpoint. Capacity recovery is a reviewed budget increase and same-identity restart, never metadata deletion. At most four lifecycle attempts run, each preflight-limited to 4096 candidate operations/64 layouts. Strict baseline lints and GitHub workflow execution remain unresolved/unverified. Ganesha recovery-mode startup/reclaim, nonempty reconnect, precise same-tid reply-loss windows, locks, dirty caps, FLUSH_ACK, unlink-open, native map-change listing and nonempty locators remain unqualified. FUSE smoke's long admin-socket path warning means its admin socket is not qualified.

## 5. Continuations and ownership

`current/sol-orchestration.md` is the authoritative assignment/handle ledger. Session handles are specific to this harness; if unavailable, assign equivalent separate implementers/reviewers after resume. Never reuse another lane's files without explicit transfer. No agent should continue implementation from this table while paused.

| Lane | Continuation handle | Owned paths |
|---|---|---|
| Design | `ses_e9b5b45a1ffeft6J7PLYxjMd07` | HANDOVER/AGENTS status, progress ledger, design Markdown, own ledger; this handover explicitly assigned. |
| Engine | `ses_e9b5afcd8ffeHMB2khzPK5BZgo` | `crates/engine/**`, own ledger. |
| Platform | `ses_e9b5ab5f1ffeQkTJRlH4PavXU1` | `crates/monc/**`, `crates/osdc/**`, own ledger. |
| Verification | `ses_e9b5a5136ffe7J3jMWfiRcU95P` | `scripts/**`, `devcluster/**`, `.github/**`, own ledger; last sole cluster owner. |
| Data layout | `ses_e9b59d2f3ffeCzV2IZfcb9cxfX` | Own ledger only; rados.rs transferred to daemon. |
| Store | `ses_e9b4c28a6ffeysCMnyAYgmbM3P` | `crates/store/**`, own ledger. |
| Daemon | `ses_e9b57d362ffeji0YGsc5Yfdt0Y` | `crates/mds/**`, dependency-resolution Cargo.lock only, own ledger. |
| Release | `ses_e9acbd67affeFR9EoCquFT0Vaw` | Release ledger, authorized isolated snapshots/staging/commit/push only. |

Reviewer handles: contract `ses_e9b541127ffes9HTFev75X78qx`; engine execution `ses_e9b50c7a7ffe09nxCBJ5fWM5y6`; engine/store state `ses_e9b23a135ffeR5ldHu86L7qvqD`; store core/daemon persistence `ses_e9b23ea4cffe0wnqCX2QYJ8x73`; platform `ses_e9b4da8a0ffel40jlhqlqy6PsF`; daemon runtime `ses_e9b4d70daffeLWWfkN0B7LQ1a5`; tooling `ses_e9b496bc0ffeTOZg1HZtvmhIOf`; data layout `ses_e9b509525ffeaNer9I75zmHw2z`; G3 references `ses_e9b574a5affeOA3oDRSzp1Y7IX`. Reports above define their accepted scopes. Do not equate a report's older historical findings with its final verdict.

## 6. Resume sequence — commands are NOT authorized while paused

1. Obtain explicit user resume. Read paused status and release ledger first. The coordinator assigns **one** QA runner. Start with read-only process/mount/monitor identity checks: verify cluster/fs44/pools88/89, original OSDs, no conflicting MDS/mounts and retained artifact/binary hashes. Do not start/retry automatically or kill an unknown process.
2. Retain old evidence and namespace. Confirm required capabilities and supported boot UUID/UNIQ-ID interface. Use `scripts/VERIFICATION.md` and the last QA owner for safe target ownership. If any preserved identity or binary proof differs, stop for coordination; do not use disposable G0 live CI or newfs.
3. After that QA assignment authorizes live retry, start the existing reviewed binary on the **existing** filesystem and wait for checked clientless active. Record its real new active PID (`ACTIVE_PID`); do not reuse the old PID from the failed trial. Example startup from `/mnt/workspace/rust-mds`:

```sh
source devcluster/env.sh
RMDS_LOG=info /mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds \
  -i rust -c "$CEPH_CONF" --keyring "$RMDS_KEYRING" --mds-join-fs a
```

4. Use the accepted private-namespace command from `scripts/VERIFICATION.md`, replacing `sol-g2-retry-NEW` with a **new nonexistent** artifact directory under an existing local parent. The assigned QA runner supplies verified `ACTIVE_PID`. This is documentation only:

```sh
source devcluster/env.sh
sudo env RMDS_LOG=info unshare --mount --propagation private bash -s -- \
  "$ACTIVE_PID" /mnt/workspace/devcluster-data/sol-g2-retry/mnt-kernel \
  /tmp/opencode/sol-g2-retry-NEW \
  /mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds "$RMDS_KEYRING" <<'SH'
set -euo pipefail
cd /mnt/workspace/rust-mds
PID=$1 MNT=$2 OUT=$3 BIN=$4 KEYRING=$5
devcluster/mount-kernel.sh "$MNT" v1
exec env RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh \
  g2 "$PID" "$MNT" "$OUT" --owned-mount --rust-filesystem \
  --expected-fscid 44 --binary "$BIN" --keyring "$KEYRING"
SH
```

5. Independently review the new result before changing gate status. Exit 0 requires cold-client proof, exact saved namespace/data/link/xattr/metadata equality, 16 acknowledged new inode numbers disjoint from all saved/deleted identities and each other, and final old+new comparison. Exit1/2/77 is not PASS. Preserve all artifacts. Success can leave the restarted daemon and owned cold mount inside the private namespace; inspect reported identities before any later cleanup. No force/lazy cleanup or OSD signals.
6. Only then assign the next proper G3 implementation/review against `current/sol-recovery-review.md`. Nonempty gathers must not be bypassed to make the partial harness run. Full mounted-client/Ganesha/locks/dirty-cap/retry qualification and storage crash matrix remain separate work.

## 7. Paused release checkpoint — final documentation at branch HEAD

Normally committed/pushed and confirmed: **`a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`**, `platform: add checked fencing and native pool enumeration` (13 paths); **`ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93`**, `mds: persist root-shard state with guarded WAL and checkpoints` (53 paths); **`805a8b0f1c8388548f0c8adf869aa28c0e70b926`**, `qa: add guarded restart verification and CI lanes` (23 paths, including the accepted unique-mount correction). Actual remote main matched local/tracking QA HEAD, ahead/behind 0/0. Baseline `f54d2988ebaeeb8311314fe6894ee030a80e955d` remains intact and published. All 640 committed/working Cargo inputs retain accepted QA/release hashes; no tests/build/fmt/live actions ran for final publication.

**Final publication is explicitly authorized after independent handover acceptance.** The remaining 27 intended status/handovers/task-review ledger paths form the final documentation checkpoint at branch HEAD, subject `docs: record paused durability checkpoint and handover`. This release-status text freezes before that commit. The release agent's final report must supply its actual SHA, normal-push result, remote main equality and clean intended working-tree status. This paragraph does not claim final remote confirmation before observation. No intended non-ignored repository change is excluded; ignored keys/build output and external evidence stay outside Git.

No self-referential final-document SHA is required. `current/sol-release.md` records the confirmed intermediate publication and final checkpoint protocol. A future orchestrator can identify the final checkpoint with read-only `git log -1 --format='%H %s'`, `git status --short` and remote main verification (`git ls-remote --exit-code origin refs/heads/main`). No files change after final publication verification. This is a paused checkpoint with incomplete G2, no corrected live retry, held nonempty G3 gathers and documented strict baseline lint failures. Release completion leaves implementation/live progress **PAUSED**.
