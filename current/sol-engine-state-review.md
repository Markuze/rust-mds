# sol-engine-state-review
Task: Independent ENGINE RECOVERY-STATE review of checkpoint/restore/replay, topology, allocation, sessions, lifecycle and reconnect. Verify store schema 2 compatibility. No production edits, live actions, commits or subagents.
Status: ACCEPT for the engine recovery-state package. All three original findings and the remaining P2 are closed. Final independent verification passed 42 focused tests and the original rejection probes plus valid-incarnation/replacement checks. No remaining defect found in the assigned scope. Store schema 2 remains compatible.
Context: Sole writable file is this report. Parallel reviewer owns execution/permit/commit-output boundaries. Build directory: target-sol-engine-state-review. Read updated engine ledger, final persistence contract and recovery reference audit. Inspected materialized state/replay, allocator, session, lifecycle, topology and reconnect implementation. Used direct helpers only to prepare fixtures; failure probes use checked restore/replay and production step. No source edits, live actions, commits, or subagents. Prior store report remains unchanged.
Next: Parent can combine this state acceptance with the separate admission/visibility acceptance for G2 integration. Daemon implementation and live G2/G3 gates remain separate.
Log:
- [2026-10-22] Final verdict ACCEPT. CARGO_TARGET_DIR=target-sol-engine-state-review cargo test -p engine --test review_regressions --test durability --locked passed: 22 + 20 tests, 0 failed/ignored. No full engine, stress, workspace or store rerun in this revision.
- [2026-10-22] Independently recompiled the original remaining-P2 probes with corrected rejection expectations. Bare-receipt checkpoint restore and bare RecoverSize retirement now fail. Failed replay preserves checkpoint, submitted/durable cursors, empty journal and nonfailed status. A valid superseding intent then replays and restores successfully.
- [2026-10-22] Additional final probes reject insufficient bounds, wrong inode, wrong inode version and wrong truncate sequence. A newer live incarnation with an old receipt restores unchanged; the exact retired key remains invalid. Existing tests also verify actual newer-incarnation range-only replay, real recovered metadata/range retirement and atomic recovery-to-purge replacement.
- [2026-10-22] Started final requested re-review. Read latest author ledger and located shared validation plus five new regressions. Sole writable file remains this report.
- [2026-10-22] Re-review command: CARGO_TARGET_DIR=target-sol-engine-state-review cargo test -p engine --test review_regressions --test durability --locked. Result: 17 + 20 passed, 0 failed/ignored. No broad engine/stress/store rerun.
- [2026-10-22] Additional stdin Rust probe passed: reconnect another writer's partial chunk before missing-client cleanup; retain both linked-file recovery jobs; install/close a second incarnation of the same client; preserve both receipts without duplicate jobs; reject stale-key cleanup; restore the resulting checkpoint unchanged.
- [2026-10-22] Independent pointwise allocator oracle passed all 660 interval/owner/floor cases. Cases cover adjacent/split old free and same-owner delegated sources, holes, different owners, retired numbers and extension above the old floor. Final overlap/range validation remains shared by restore and replay.
- [2026-10-22] Two probes confirmed the remaining P2 finding: restore accepts a receipt with an abandoned writer bound and no job; replay accepts bare retirement of that bound's existing RecoverSize job. No production files changed. Probe binaries reside under target-sol-engine-state-review.
- [2026-10-22] Started requested re-review. Read updated author ledger, all 17 review_regressions and revised durable/step/reconnect code. Prior findings below remain as historical rationale until the re-review outcome is recorded.
- [2026-10-22] Created ledger before other edits.
- [2026-10-22] CARGO_TARGET_DIR=target-sol-engine-state-review cargo test -p engine --test durability --locked: 20 passed, 0 failed/ignored. Did not duplicate full engine/stress runs.
- [2026-10-22] CARGO_TARGET_DIR=target-sol-engine-state-review cargo test -p store --test checkpoint --locked: five passed, 0 failed/ignored. This checks final engine field compatibility with schema 2, including old_layouts.
- [2026-10-22] Compiled review probes from stdin with rustc against the built engine library. Wrote only binaries under the assigned target directory. Probes confirmed all three findings below. Also confirmed the ordinary lost-safe-reply CREATE retry returns the original inode, create extra and parent/target trace.
- [2026-10-22] Review complete: CHANGES REQUIRED. No workspace or live gate claimed.

## Final acceptance

The retained-writer invariant now resides in shared validate_state, after session and job validation. Every retained range needs a live client session or a valid covering RecoverSize intent. The coverage predicate checks inode identity, maximum bound, inode version and truncate sequence. restore validates before constructing an engine; replay validates the complete candidate before replacing live state. Retirement-only records can no longer bypass the check. The rule permits a distinct newer live SessionKey without treating an old receipt as ownership of that newer key.

All requested rejection and valid-transition cases passed independently. Existing valid completion, replacement, purge, truncate ordering, hard-link/stray and interrupted-cleanup cases remain passing in the 42-test focused run. Earlier partial-reconnect/repeated-incarnation probes and the 660-case allocator oracle are recorded above as prior re-review evidence, not rerun claims.

No public signature or persistent field changed in this correction. Store schema 2 remains compatible. This ACCEPT is limited to engine recovery state. It does not claim daemon integration, live storage, mounted-client recovery, or G2/G3 gate completion. No production edits, live actions, commits or subagents occurred.

## Previous re-review outcome (historical)

### Original findings resolved

1. Production close now scans durable ranges after volatile caps/locks. It journals recovery intents before the terminal receipt. Interrupted cleanup reuses an existing covering intent. The gather queues unclaimed strays for permitted purge steps. A claimed stray survives. Purge waits for prior truncate work and atomically replaces recovery intents while retaining all layouts and maximum bounds. The supplied regressions cover callback loss, replay and completion. Additional partial-reconnect and repeated-incarnation probes passed.
2. Replay now checks both free and delegated range provenance. Same-owner delegations may combine with old free space; another owner's delegation and burned holes cannot supply numbers. The independent 660-case oracle found no alternate interval partition bypass. Restore also rejects overlap and live/job inode numbers beyond the allocator floor.
3. Shared topology validation now requires a primary path to root for linked non-root inodes, rejects directory remotes and rejects cycles/unrooted linked subtrees. Valid hard-link promotion and explicit strays still restore/replay and round-trip correctly.

### P2 — Shared state validation does not protect closed-writer bounds (now resolved)

Locations: crates/engine/src/engine/durable.rs:342-352 (receipt validation), :353-410 (job validation), :548-552 and :585-600 (job retirement/final replay validation).

The new terminal-record check rejects a receipt that abandons an uncovered bound in that same replay record. However, validate_state does not enforce the corresponding invariant for a materialized checkpoint or after a later job retirement. It validates existing jobs, but does not require a job for a retained bound whose client has no live session.

Verified minimal reproductions:

- Export a checkpoint with a writable file. Remove its session and add a valid cleanup receipt at covered_seq. Keep the inode's client_ranges entry and no jobs. Engine::restore returns Ok. The same inconsistent state rejected by the new terminal replay guard is accepted through checkpoint-only restore.
- Start with a valid production checkpoint after restored missing-writer cleanup: it contains the receipt, retained bound and covering RecoverSize intent. Restore it. Replay the next LifecycleComplete record containing only completed_jobs=[intent.id], with no inode image or replacement intent. replay_record returns Ok. The client has no session; its range remains; the covering job is gone. A completed gather can now expose stale metadata with no recovery work.

Required change: enforce the provable closed/orphan-writer invariant in shared candidate/checkpoint validation, or add equivalent restore and retirement checks. A retained range with no live client session needs a valid covering durable recovery intent. Account for newer live SessionKeys of the same client id; an old receipt alone must not make a newer incarnation's legitimate range invalid. Permit atomic replacement by a new recovery intent or full purge, and permit valid recovered metadata/range retirement.

Minimal regressions: reject the two states above and assert that failed replay leaves the checkpoint unchanged. Keep positive cases with old cleanup receipts plus a newer live incarnation's writer range, a covering pending recovery job, and atomic RecoverSize-to-Purge replacement. No schema change is needed.

### Evidence limits

The final persistent type shapes and enum variants remain unchanged, so store schema 2 remains compatible. The prior five codec tests remain relevant; no full store rerun was needed. This re-review does not qualify native storage, daemon recovery joins, mounted clients, or G2/G3. Pure-ack and eviction-reset behavior remain the parallel reviewer's scope.

## Original findings (historical; original reproductions now fixed)

### 1. P1 — Restored missing-session cleanup ignores persistent writer ranges and strays

Locations: crates/engine/src/engine/step.rs:217-241; crates/engine/src/engine/reconnect.rs:131-140. Related: crates/engine/src/engine/client_caps.rs:347-391.

Checked restore correctly creates no runtime caps. However, close_step enumerates only Session.caps and Session.lock_inos before writing its terminal cleanup record. It never examines restored InodeCore.client_ranges. remove_cap also returns immediately when no runtime cap exists. Therefore a missing client, or a Closing client interrupted before cleanup, receives a durable terminal cleanup receipt without a RecoverSize intent for its persisted write bound.

Verified reproduction: create a linked writable file, checkpoint, restore without any WAL history or caps, then use step(CloseSession { key, kill: true }) and commit/drain. The session disappears and session_cleanup_seq(key) returns a receipt. The file still contains that client's durable write range. The checkpoint contains no lifecycle job. Nothing blocks later file operations on size-recovery grounds. The engine can expose the pre-crash size without probing the fenced missing writer's data.

A related verified case uses an unlinked-open file. After restore, missing-session cleanup and a completed gather leave the inode resident, with no caps, no purge intent and an idle engine. finish_reconnect evaluates caps and existing jobs, but does not schedule retained strays that no client reconstructed.

Required change: drive recovered cleanup from persistent writer bounds, not only runtime caps. Commit the necessary recovery/range decisions before the terminal receipt. After the complete reconnect/eviction gather, schedule unclaimed retained strays through durable purge work. Do not purge a stray before other gather members can reconstruct its caps.

Missing regressions: checkpoint with a linked writer and an unlinked-open inode; restore with no caps; resume Closing or evict the missing Open key through production step. Require a committed RecoverSize intent retaining the original bound before cleanup proof, then journaled result/retirement. Require the unclaimed stray to receive durable purge work after gather completion. Repeat with a crash between cleanup records.

### 2. P2 — Checked replay can reintroduce consumed inode numbers through delegated ranges

Location: crates/engine/src/engine/durable.rs:402-426. Related final-state check: :131-160.

The allocator transition check proves provenance only for the new free ranges. It does not check new delegated ranges against the old free/delegated state. Final-state validation checks overlap with current inodes/jobs and the floor, but cannot identify consumed or burned numbers that are no longer present. A malformed record can therefore put an unavailable number below the old floor back into a delegation. It can also change the owner of an existing delegation without a valid transition.

Verified reproduction: restore a valid checkpoint with one consumed/burned hole below the allocator floor. Replay the next ClientRanges record with unchanged free ranges/floor and a new delegation covering that hole. replay_record returns Ok, and ino_allocator().delegated_to(client) now contains the unavailable number.

Required change: validate provenance for both free and delegated ranges. Below the old floor, a delegated range must come from old free space or an appropriate surviving same-owner delegation. Retired/consumed ranges must never reappear. Preserve valid delegation retirement and new globally reserved space.

Missing regression: allocate and retire an inode, complete its purge, checkpoint without old WAL, then try to reintroduce its number as a delegation in the next record. Require an error and an unchanged engine checkpoint. Also reject transfer of an existing delegation to another client. This is validation of the existing persistent delegation hook, not a request to enable async creates.

### 3. P2 — Checked restore accepts linked inodes without a primary path

Location: crates/engine/src/engine/durable.rs:234-258.

The primary-link equality test permits None == None. The link-count test then accepts a non-root inode whose only links are remote. The ancestor walk also accepts termination at that inode instead of root. This is not a valid materialized root-shard namespace: primary promotion must leave one primary link for a linked inode, and authorization_path requires that chain.

Verified reproduction: start with an exported checkpoint containing root/f. Change its root dentry to primary=false, move the inode's parent pair into remote_links, and clear parent. Keep nlink=1. Engine::restore returns Ok; lookup_name resolves f, but parent_of(f) is None and no primary authorization path exists. Later last-link removal cannot capture the required former path.

Required change: require a primary link for each linked non-root inode and require its primary chain to reach root. Reject unsupported directory remote linkage. Preserve the explicit exception for valid unlinked strays with a persisted former authorization path.

Missing regressions: reject a remote-only linked file, a remote-only directory, and a linked subtree whose primary chain terminates at an unlinked directory. Check both checkpoint restore and final candidate validation during replay. Retain valid hard-link promotion and stray round-trip tests.

## Verified behavior and scope limits

- Materialized export carries full cores, explicit primary/remote links, allocator state, incarnation-scoped request/flush histories and empty-history watermarks, cleanup receipts, lifecycle jobs and identifier seeds. Store schema 2 covers the final fields, including stray_auth_path, stable_subvolume_id and old_layouts. No schema mismatch found.
- restore validates before returning an engine. replay_record builds and validates a candidate before replacing the existing state. It requires the next sequence and checks explicit inode versions, nonzero stable identity preservation and missing references. The findings above identify gaps in those checks, not partial mutation on an already rejected record.
- Identical install_session_key preserves histories and watermarks. Different live incarnations and retired exact keys fail. Existing terminal receipt replay tests pass.
- Existing lifecycle tests verify intent-before-work, retained jobs/ranges through completion commit, old layouts, duplicate completion, dirty-writer probe supersession and result bounds. Request execution blocks during size recovery. No separate valid-production stale-version path was found that permanently parks an existing job; finding 1 concerns missing job creation during restored cleanup.
- Reconnect chunks validate all supplied inode/realm/lock inputs before installation. Repeated inode reconstruction replaces locks, authenticates their client field, preserves cap/realm context, and raises the cap-id seed. Gather completion checks all installed session keys. Byte/count budgets and current credentials remain daemon policy.
- The ordinary lost-safe-reply O_EXCL CREATE retry (retry_attempt=1, WANT_DENTRY, not the unsafe-result REPLAY flag) returned the original inode, create extra and valid parent/target trace in a focused restored-engine probe. A range transaction can precede that reply. The daemon must still drain such retries before ordinary mutations can change their trace inputs. No unsafe-reply feature was demanded: C++ Client.cc:2852-2855 sets REPLAY for a request that already received an unsafe reply.
- This review does not qualify daemon joins, native storage, mounted-client recovery, G2 or G3. It does not replace the parallel execution/permit/journal-output review.
