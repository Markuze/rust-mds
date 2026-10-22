# sol-daemon-store-review
Task: Independent Sol Ultrafast review of NEW daemon G2 persistence integration. Review authority and persistent data flow. No source edits, live actions, subagents or commits.
Status: ACCEPT for the assigned daemon persistence implementation. ACCEPT EVIDENCE for scoped native bootstrap, clientless startup/restart, checkpoint/trim observations and normal kernel/FUSE smoke. Complete G2 application comparison remains BLOCKED by tooling; G3 remains unqualified. Reviewed retained final offline results: 524 passed, 0 failed, 2 ignored; build/fmt/default clippy pass; strict clippy fails on recorded baseline diagnostics.
Context: Only this report was edited. Tests used target-sol-daemon-store-review. Reviewed persistence.rs, journal.rs, server/runtime.rs, test_backend.rs and required config/daemon/fencing/rados/session boundaries. Traced engine restore/replay, committed-job eligibility and completion guards as integration interfaces. Parallel reviewer owns step/permit/protocol/admission/phase execution. Store package acceptance does not substitute for daemon or live gate acceptance.
Next: After separate acceptance of the unique-mount correction, QA must run a new complete G2 trial on preserved fscid44/meta88/data89. Require cold state comparison and 16 future inode nonreuse checks. Do not reset/recreate the filesystem or promote the failed trial to PASS. Nonempty G3 gathers remain held.
Log:
- [2026-10-22] Created task ledger before inspection.
- [2026-10-22] Read author ledger, daemon/store READMEs, final persistence contract and both store reviews. Reviewed all new persistence modules and their tests. Inspected actual production timeout, messenger identity and storage-worker construction.
- [2026-10-22] Independent CARGO_TARGET_DIR=target-sol-daemon-store-review cargo test -p rmds --locked passed: 71 passed, 0 failed, 0 ignored. Test build reported four dead-code warnings.
- [2026-10-22] Independent daemon-only clippy passed. Production build reported six dead-code warnings; the test build reported four. No strict-lint or workspace-clean claim.
- [2026-10-22] Completed authority, data-flow, uncertainty, checkpoint/session join and lifecycle boundary review. Verdict ACCEPT within scope. No live actions or source changes.
- [2026-10-22] Started independent read-only evidence review. Sole writable file remains this report. No new code/tests/live calls are authorized.
- [2026-10-22] Completed retained-evidence review. Verified all 640 Cargo input hashes across QA snapshots/archive, release snapshot and current source. Verified 14 recorded evidence hashes and the approved binary hash. Recounted 42 Rust test-result groups as 524/0/2. Decoded native raw control xattrs independently. ACCEPT EVIDENCE within the limits below. No tests, code changes, cluster calls, signals or active script-diff review.

## Native evidence review: ACCEPT EVIDENCE

Evidence root: `/tmp/opencode/sol-integrated-g2-20261022-2014/`.

The retained records support the scoped claims. They do **not** establish a complete G2 application PASS. There is no evidence inconsistency that requires invalidating the native startup/smoke observations.

### Source and binary provenance

- `tree-before.sha256.json`, `tree-offline-after.sha256.json` and `tree-final.sha256.json` contain the same 640 Cargo inputs with identical content hashes. All 640 also match the release agent's `primary-cargo-inputs-before.json` and current files. The full 1,784-file `accepted-source.tar` matches the initial QA manifest exactly. Only recorded status documents changed during QA. I did not inspect the active tooling correction diff.
- The retained source includes the accepted production native backend and the later independently accepted runtime corrections G2-R1–R4. `test_backend` is gated by `#[cfg(test)]`. Production constructs `RadosBackend` with the shared Objecter; the retained daemon logs also show actual monitor/OSD connections and native persisted objects. This is not a simulated-backend run.
- Initial/final binary records, the trial's `daemon.json`, and the current executable agree on SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`. The build log completes successfully before native bootstrap.
- QA's recorded `head.txt` is `f54d2988ebaeeb8311314fe6894ee030a80e955d`. Current HEAD is `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`, with that baseline as its parent. I verified the 13 committed platform paths against QA's hashes; all match. The publication changes Git attribution, not the tested Cargo content. These are accepted modified-tree results, not results for the unmodified baseline alone.
- All 14 entries in `evidence-files.sha256.json` match their retained files. This includes the trial history/error/restart log, smoke logs, raw-result summary and offline logs.

### Offline results

I independently summed the 42 `test result: ok` groups in `unit.log`: **524 passed, 0 failed, 2 ignored**, including doctests. The final daemon group contains the 82-test correction revision. The ignored tests are `dump` and `extract_corpus`. One optional kernel-v1-mdsv1 capture/corpus test logs a missing-fixture early return; a harness pass does not mean that absent fixture was exercised.

`offline-results.txt` and command logs agree: workspace tests, daemon build, fmt check and default workspace/all-target clippy exit 0. Strict clippy exits 101. It reports 46 generated denc `identity_op` errors and the two recorded engine lints (`for_kv_map`, `collapsible_if`). Default clippy also retains monc `type_complexity` and daemon dead-code warnings. No strict-lint or CI-green claim is accepted. The separate retained tooling log reports 67 tests and OK. I ran no new tests or builds for this evidence review.

### Native subclaims established

| Subclaim | Retained evidence and accepted scope |
|---|---|
| Separate fresh setup | `fresh-setup-command.txt` explicitly records setup outside the trial. `fresh-setup.log` creates meta88/data89 and fs44 at 20:17:48. Both fresh all-namespace pool listings are empty. Initial fs44 has no MDS entries. This is the sole setup/recreation represented in this run. |
| Real Ready bootstrap | `bootstrap-daemon.log:13-24` shows standby -> creating332 -> active333, actual OSD connections and checked persistent load with zero restored Open sessions. Native monitor observations agree. Raw format identity decodes to cluster `3ceca2c6-851f-41cf-b538-1cfded4f1e82`, fs44/meta88; phase is Ready. Initial shard owner is epoch1/incarnation332/gid16461. Checkpoint generation1 covers sequence1; tail and trim are1. |
| Pre-stop acknowledged history | `g2-trial-isolated/history.jsonl` has matching intent/ack sequences0–16 and a complete marker for17 steps. Its10 distinct creation inode results exactly match the allocation manifest: 1099511627776–1099511627785. Operations include the hard link, rename-overwrite, deletions and binary/empty user xattrs. `oracle-output.jsonl` contains record output only. This establishes the recorded pre-stop history, not its survival/equality. |
| Clientless native restart | Trial identity-before/after records are identical: fs44/meta88/data89 and the same cluster fsid. The retained archived controller performs normal unmount/audit before the pidfd TERM and launches the selected executable directly. `stop-intent.json` and `stop-sent.json` identify original PID1608362/gid16461. Restart PID1629466/gid16519 reaches checked persistent load with zero Open sessions, then replay336 -> reconnect337 -> rejoin338 -> active339 (`restart.log:16-28`). A new kernel client subsequently opens. There is no Creating transition during restart. |
| Native checkpoint publication and trim observations | Independently decoded post-trial raw xattrs agree with `store-after-trial.json`: owner epoch2/incarnation336/gid16519, tail36, checkpoint generation4/coverage34 at `rmds.checkpoint.0.1.4.manifest`, trim34. The object listing retains checkpoint generations1–4 and stable format/WAL/session/allocator objects. Remaining WAL keys are35 and36. This supports published coverage/trim and predecessor checkpoint retention; it does not prove an exact replay-record count or a crash at a specific checkpoint boundary. |
| Same-filesystem kernel/FUSE smoke | `durable-smoke-result.txt` records exit0. `durable-smoke.log` records kernel I/O, FUSE I/O, cross-client visibility and PASS. The archived smoke performs8MiB write/read-back checks for each client plus metadata operations. Its daemon loads persistent state and follows replay342 -> reconnect343 -> rejoin344 -> active345 before serving. Identity snapshots before/after retain fs44/meta88/data89 and the original filesystem creation timestamp. No setup/reset/newfs call exists in the retained trial or smoke paths. |
| Final native positions and cleanup | Final raw xattrs independently decode as owner epoch3/incarnation342/gid16577, tail83, checkpoint generation7/coverage82 at `rmds.checkpoint.0.3.7.manifest`, trim82. The remaining WAL key is83. Native filesystem snapshots remain fs44/meta88/data89. Retained final all-namespace mount audit is empty; process snapshots contain the original mon/mgr/three OSD PIDs and system Ganesha, but no MDS/FUSE test child. PGs remain17 active+clean. Expected offline/failed-rank HEALTH_ERR after MDS cleanup is not a storage-test failure. |

The retained raw dumps cover control xattrs and object/key listings, not complete checkpoint payloads, rank-session rows or an independent live fencing trace. Checked native load exercises the source's dependency validation. Do not describe this evidence as a new independent byte-level audit of every native checkpoint or predecessor proof.

### Native diagnostics are not hidden

The three daemon logs contain initial beacon-lag warnings followed by successful beacon acknowledgment, and INFO-level client EOF connection faults at client teardown. They contain no store failure, corruption, failed phase, panic or fail-stop diagnostic. I found no native storage error hidden by the smoke exit0.

There is one real non-fatal client diagnostic: `durable-smoke.log:95` reports that the generated ceph-fuse admin-socket pathname exceeds107 bytes. FUSE then mounts and completes its I/O checks. This qualifies normal mounted I/O, **not** a functioning FUSE admin socket. Repeated ceph-smartctl sudoers warnings are also visible. The smoke deadline record says the controller had already exited and no signal was sent.

### Why complete G2 remains blocked

The host-namespace invocation exits2 before workload admission because propagated peer mounts prevent its cold-client proof. The private-namespace invocation reaches native active after restart, then exits1 at `cold_remount`: `error.json` records `cold remount retained the old mount ID`, original ID323. The archived controller raises this error immediately after the mount record comparison and **before** `history("verify")` or `history("allocate")`. The retained trial directory contains no `result.json`, `timings.json`, `client-state.json` or `future-history.jsonl`.

The error is consistent with reusable mountinfo IDs. It is not a native replay failure. However, the retained data does not independently prove a non-recycled unique mount identity for that remount. The separate smoke does not replace the missing acknowledged-history comparison. No G2 PASS or16-future-inode nonreuse result can be inferred from a successful remount, later smoke or monotonic-looking WAL positions.

Required next evidence: after the separately reviewed unique-mount guard correction, run a new authorized trial on **preserved fs44/meta88/data89**. Require positive cold-client identity, the complete post-restart namespace/data/link/xattr/metadata comparison,16 acknowledged future allocations disjoint from all10 prior allocations (including the four absent/overwritten identities), and the final old+new comparison. Keep the current failed-trial artifacts unchanged. Broader crash-boundary/fencing/uncertain-reply cases and nonempty G3 reconnect/locks/dirty-cap/flush-ack/unlink-open/Ganesha recovery remain separate unqualified work.

This evidence review used local retained-file reads, hash/JSON/binary decoding and read-only Git metadata only. It ran no tests, live cluster calls, signals, mounts, commits or subagents and changed only this report.

## Implementation review verdict: ACCEPT

No ranked correctness finding remains in the assigned storage-integration scope. This verdict is not acceptance of the parallel protocol/step review or a native durability gate.

### Authority and bootstrap

- `crates/mds/src/daemon.rs:384-458` requires a current monitor assignment, healthy beacon, supported startup state and a unique native FSMap match before starting storage. The persisted writer uses the shared messenger's actual address vector. The same messenger serves Objecter and monitor traffic. The rank incarnation is installed before storage starts (`:749-753`).
- `crates/mds/src/persistence.rs:249-327` reads format, shard and session authorities before acquisition/replay. It collects their actual writers and retained predecessor evidence. `prove_writers` fences every persisted address, including nonce, queries a fresh OSDMap, waits through the recorded/failure epoch and raises the barrier (`:181-232`, `fencing.rs:90-110`). Failure epoch zero does not bypass predecessor fencing. Missing/unusable identity fails closed.
- Fresh construction exists only under monitor-confirmed Creating. It passes a deterministic root/range/bootstrap identity to the real `BootstrapStore::begin_bootstrap`, which performs native all-namespace enumeration before the first format write. Starting/replay with no format, or with Preparing, fails. Preparing takeover preserves the persisted plan. Resume can create exact missing planned dependencies, but cannot acquire their separate guards. The daemon explicitly acquires shard and sessions, then verifies/resumes and publishes Ready (`persistence.rs:328-415`). No format or WAL compare purports to authorize a different object's mutation.
- The initial allocator receipt precedes Ready through the accepted bootstrap plan. `validate_ready` checks the retained reservation/high watermark. The daemon restores allocator state; it does not reconstruct free space from the initial range or reset allocation on restart. This package does not add a dynamic lease-refill worker. Exhaustion therefore does not authorize reuse.

### Uncertainty, replay and session state

- Production sets `ObjecterConfig::op_timeout = Some(cfg.persistence.op_timeout)` (`daemon.rs:189-194`). Configuration rejects zero (`config.rs:265-274`). The worker awaits the actor result rather than repeatedly dropping its receiver. It serializes native storage commands. Unknown results retain exact append bytes/tail/id, acquisition arguments, session action/revision or checkpoint generation/descriptor (`persistence.rs:87-101`, `:523-636`). The retry loop makes one initial attempt plus at most eight retries. After exhaustion it fails closed. No uncertain append advances the local durable tail or generates a Committed event.
- Gate waits park storage while health is unavailable. Each native operation has the actor timeout. Store scan/count/byte bounds and retry bounds limit an active storage attempt. The current code does not use a caller-only timeout as cancellation proof. Process failure terminates the runtime; a replacement must inspect/fence/recover, not invent a replacement batch or namespace.
- `journal.rs:28-58` starts submission-closed. `server/runtime.rs:175-238` uses checked checkpoint restore, strict ordered uncovered replay and an exact frozen-tail check before enabling submission. It joins rank history with shard histories and cleanup receipts. Missing/mismatched rows, Closed without a matching receipt, an Open receipt or a duplicate live client fails startup. Installation preserves the identical durable SessionKey. Closed tombstones are never installed as live sessions (`:239-290`). Range-only replay uses guarded rank incarnation history; the engine still validates each replay candidate.
- New OPEN waits for the durable rank row before installation/output. Closing persists before engine cleanup. Closed follows the terminal shard receipt and preserves its exact sequence (`runtime.rs:664-734`, `:796-909`). Restored Closing peers are fenced before cleanup resumes (`persistence.rs:421-439`). Nonempty restored Open gathers remain held, as explicitly required for this G2 package.
- Storage callbacks require current membership, owner and operation identity. Unknown/stale operation callbacks cannot advance the journal or publish session output. Guard rejection, corruption, missing state and exhausted uncertainty reach fail-stop policy, not log-and-continue (`daemon.rs:462-525`, `runtime.rs:609-751`). Checkpoint Capacity is the deliberate exception: it pauses ordinary execution without discarding durable state.

### Checkpoints and lifecycle

- The daemon exports a quiescent committed engine cut and pauses execution during its checkpoint command (`runtime.rs:493-544`). The worker requires coverage equal to its durable tail. It writes immutable state/manifest, publishes the guarded descriptor, then performs bounded exact-descriptor trim (`persistence.rs:603-634`). Only the completed trim callback releases the journal retention budget. Initial trim runs before recovered mutation admission.
- WAL admission reserves against retained keys and maximum framed-record bytes, not current outbox memory. Retained checkpoint Capacity sets persistent admission pressure. It does not release WAL capacity or silently collect old generations. `crates/mds/README.md:62-67` gives nondestructive recovery instructions: increase the relevant budget and restart with the same identity/pools; do not delete metadata objects.
- Lifecycle scheduling reads committed engine job state, not speculative effects (`runtime.rs:433-446`, `:949-967`). `daemon.rs:592-643` requires serving/reconnect readiness. Thus no purge starts while a nonempty restored gather is held. Engine eligibility also requires recovery readiness and absence of caps for purge. Recovery probes wait until all same-inode truncate intents retire, which occurs only after their I/O results and completion commits. Truncate operations retain their sequence guard.
- The executor uses full retained layouts, namespaces and range bounds. It preflights at most 4096 candidate objects and 64 layouts before I/O (`rados.rs:253-343`). At most four attempts run concurrently. Job id/owner/attempt identity deduplicates callbacks. Failed attempts retain intent and use capped retry delays. Successful results enter a new engine transaction; they do not directly retire a job. Size recovery checks the retained inode version and truncate sequence before applying metadata. A failed completion commit stops service. Superseded/retired job callbacks cannot mutate a replacement job.

## Fault-test assessment

The new persistence tests call the actual store API and real daemon journal/runtime path. They are not synthetic successful store events. `test_backend.rs:42-165` commits a private single-object image only after every guard succeeds; it offers no cross-object atomicity. It injects a lost append response after durable mutation. Its OMAP reader deliberately returns short pages. Tests verify that an uncertain append causes one durable append and no reply before the tagged commit event.

The startup tests exercise preserved filesystem identity, persisted predecessor nonces, barrier-before-first-acquisition-write, failed fence with zero authority writes, missing/wrong/corrupt state and a crash between independent shard/session acquisitions. The real worker test follows durable Open, WAL commit, Closing, cleanup receipt, Closed, checkpoint/trim and checkpoint-only restart. Stale-owner and failed-append events release no success. Retained-generation Capacity stops work without retry spin.

Limits of this oracle: its pool objects use the default namespace and its map is static. Its fencing backend is an in-memory evidence recorder. It does not simulate a real Objecter actor timeout, native map remap, transport fencing or disk durability. The accepted store tests separately cover scripted all-namespace scans, empty continuations, map-change full restart, pagination and append cancellation. None of these offline tests qualify the native daemon integration. Lifecycle tests here mainly verify preflight/mapping; native partial-I/O/crash execution remains QA work. I found no source-level blocker in those traced interfaces.

## Independent verification

```text
CARGO_TARGET_DIR=target-sol-daemon-store-review cargo test -p rmds --locked
PASS: 71 passed, 0 failed, 0 ignored.

CARGO_TARGET_DIR=target-sol-daemon-store-review cargo clippy -p rmds --all-targets --no-deps --locked
PASS with warnings.
```

Warnings: unused `Config.cluster`; unused compatibility `spawn_purge`, `spawn_truncate`, `spawn_recover`; production-unused `Session::push_seq`, `SessionMap::iter_mut` and `SessionMap::remove`. Clippy groups the last two methods into one warning. No warnings were suppressed or fixed by this review.

No source edits, commits, live actions or subagents occurred. QA must still prove same-filesystem native bootstrap/restart, real uncertain completion/reconciliation, and eventual lifecycle crash recovery. G3 nonempty reconnect reconstruction and mounted-client failover remain unqualified.
