# glm-daemon-e1-d5-startup-review

Task: NEW independent daemon startup/storage integration review after the accepted r4 joined dependency. Verify the frozen D5 startup candidate at /mnt/workspace/sol-daemon-e1-d5-20261026.g5o2cugy, first checking the exact69 mechanical promotion against the accepted r4 joined map, then semantic startup/config/D5/fencing review plus one focused execution inventory on a fresh pinned copy. No fixes, promotion, Git or live actions.

## Decision

**ACCEPT (scoped).**

Scope of this acceptance:

- The exact69-path mechanical promotion against the accepted r4 joined reference is verified, and the working PRIMARY tree carries exactly that engine/store content plus the 16-path daemon delta.
- The 16-path daemon delta implements the startup/configuration/one-EngineConfig/adoption/D5 duties of plan `7edcc0623ca128df981e5c77b008615d4d11c68dab631dddac554fce76b25a56` sections 3, 4, 6 and 9. Startup, terminal-ownership, authority, retry-limit and D5-fencing semantics guard the recorded invariants.
- One focused execution inventory ran on a fresh member-exact pinned copy: 29 tests passed in debug and 16 async actual-worker tests passed again in optimized. All 160 original test IDs are preserved unchanged; exactly 19 new IDs exist.

Not in scope of this acceptance and still open work for their own owners: native store fault trials on the current binary, mounted-client G3 qualification, live upgrade, any cluster operation, workspace-wide combined checks, and engine internals beyond the accepted r4 scopes (engine/store retain their independent acceptance; this review only consumed them).

## Candidate identity

| Item | SHA256 |
|---|---|
| Author candidate 667 Cargo/crate input map | `7a901827b32455aa234912a3f7f00eb92c02e046feef28b48702c611f3eb0aa4` |
| Qualified source archive, 668 entries | `e65c7b5e38c77bc9532a1e69fcf3d2cfee092fa18ff4851d7f1e142e14e35c52` |
| PRIMARY preimage archive, 649 entries | `42a561fa7eabdfdffffc70dd0b1eb132665a5df3eef66e30d4eeb00ed960263a` |
| Promotion receipt file | `bb6650089858df0bb54de82e391902ad5321dc1411c58267008f3e206b505785` |
| Accepted69 applied-map reference | `6da20506bdcafabf04b0c461e431d0a7c168cb96c08024e83f50e993bcb65d9b` |
| Accepted joined 656 map | `89682736218450d3f0aad16a8a6a84a7546823a7eb9ec5243de10b1a97e2943f` |
| Merged stable.rs (isolated base) | `2cb747b0f997dd87cfbe0c7720f7c98e9010573561682589c1210906a5b011d6` |
| Root capture testdata/maps/fsmap.bin | `84f149cb0ca619006f6f6e9ac7117d5749b9828ee826ba4c8c695149944dfc47` |
| Immutable plan | `7edcc0623ca128df981e5c77b008615d4d11c68dab631dddac554fce76b25a56` |

## Phase A — pins PASS

16 author SEAL pins verified against the frozen root. evidence.tar.gz carries 494 members; the canonical bundle map including the `bundle-members.json` full record (fs_mode 0o664) recomputes the sealed digest. qualified-source.tar.gz carries 668 members (667 map + the fsmap.bin capture, which lives in dependency context). primary-preimages.tar.gz carries 649 members = `primary-base.json`. supplemental archive carries 669 members. The expanded `qualified-source/` tree equals the 667 map. Receipt: `provenance/pin-verification.json`.

## Phase B — exact69 promotion PASS

Working-tree crane comparison proved:

- The current PRIMARY tree equals the qualified 667 map byte-for-byte.
- The candidate 69-path set matches the accepted applied-map reference `6da20506...` exactly: 22 paths byte-equal, 32 paths byte-different (each current != preimage, and delta old == preimage for the 13 daemon supersessions), 15 added. No removals.
- Daemon delta is exactly 16 paths (13 modified + 3 added: `persistence/e1_tests.rs`, `server/runtime/ownership.rs`, `server/runtime/tests/d5.rs`).
- 587 context paths: 582 unchanged, 13 documented daemon supersessions, 10 context rows superseded by the delta, 13 differing context files preserved from the primary (never copied from the candidate island), 8 primary-only rows accounted, groups 51 engine / 18 store / 25 mds / 573 dependency-context + 1 root capture.
- They also retained `testdata/maps/fsmap.bin` (in dependency context and archive member 668, outside the qualified source).

Receipt: `provenance/promotion-verification.json`.

## Phase C — strict r4 recheck and fixture retention PASS

- The accepted r4 candidate manifest records 2,872 engine records. The current-path check achieved 2,848 matches and exactly 24 mismatches. Every mismatch is a `crates/engine/**` path that exists in the exact r4 preimage, and the current bytes equal the accepted r4 result. This confirms the candidate binds the accepted delta rather than reusing unknown content. The recorded pre-check failure (2,848/2,872) stays what it is; no newer check re-labelled it. Receipt: `provenance/strict-recheck.json`.
- Merged `stable.rs` retains all 20 fixture assertions: the Legacy2 seed (created in `bootstrap_plan()`) and the later-owner adoption flow (`publish_ready`, `acquire_all`, `upgrade_retry_targets`, `shard_epoch/incarnation/gid += 1` in `install_session_key`, and `Engine::restore`). Production prefix `656779f54cde...` and the sixteen retained reference snapshots verified. Receipt: `provenance/fixture-retention.json`.

## Phase D — semantic review (16 delta paths, plan 3/4/6/9)

No blocking finding. Verified semantics per duty:

**Strict configuration (config.rs `retry_limits`).** Present-and-invalid values are errors, never defaults. Values trim and must be ASCII digits only; signs, zero, non-digits, u64 overflow are rejected before `usize` conversion. `max_completions` requires `count+1` representable; `max_bytes` requires `count+8388608` representable. Precedence is `[mds.<id>]` then `[mds]` then `[global]` then default. Missing keys take canonical defaults. `--upgrade-retry-targets` is a valueless opt-in flag; an inline `=value` bails, default false. Carve-outs (`number` helper for other keys) are unchanged.

**One immutable EngineConfig.** `maybe_start_storage` builds one `EngineConfig` at lines 499–507 (retry limits from the parsed persistence config, data pools from the map, `max_record_bytes` minus framing overhead), stores a clone on the daemon before worker spawn, and moves the same value into `Start.config`. The seed restores `Engine::new(start.config.clone(),)` and the upgrade coordinator receives `&start.config`. A Loaded event restores the server from the stored `startup_config` snapshot (context error — no default fallback — if missing, which is unreachable in production because the clone is set before the worker spawns). `from_loaded` re-derives only two fields deterministically (`ino_lease` from the loaded initial range, `max_record_bytes` from the checked `Limits`); this pattern predates the delta and passes the retained snapshot by value, so one-config identity holds.

**A terminal semantics.** The completing seed process (fresh Exclusive-state seed or Preparing completion) gets `UpgradeRequired` before any recovery target upgrade; the daemon records `Phase::Failed` + `fatal_error` with an explicit UpgradeRequired explanation and exits through the normal shutdown path (DNE beacon, output paused, no server, no service, no intent, no records). It fails identically with `--upgrade-retry-targets`. `Completed seed worker event` is tagged `Failed(UpgradeRequired)`, owner None, operation tagged, and the event channel then closes with no Loaded (e1_tests verified with both opt-in values, actual worker).

**Later owner B.** Eligibility computes from the pre-acquisition snapshot format/control/sessions; a predecessor with the same GID or a lower/equal incarnation gives `UpgradeRequired` (same-owner/no-option refusal included). A genuine later owner (different GID, higher incarnation) re-checks fenced writers (`prove_writers` over the format/control/sessions writer sets plus prepared_by), acquires format/control/sessions with independent guards, and only passes the eligibility gate under explicit opt-in. Barrier raise precedes the first write (worker-level `barrier < first write` assertion; e1 test verified with actual worker).

**Upgrading resume retains exact handles.** `retry_unknown!` retries an operation at most 8 times at 250 ms on `UnknownOutcome` and returns the error afterwards; timeouts never become a new ID or a rollback claim. The upgrade identity comes from persisted `format.retry_upgrade.id` when present, else `operation_id(gid, operation)`; appends reuse `operation_id(owner.gid, seq)`; repeated attempts replay identical arguments through the retained `BootstrapStore`/`SessionRepository` handles. Uncertain mutations stay recorded in `Attempts` exactly (bounded at 128, `Capacity` above; exact native args on the active slot are retained as uncertain.

**Normal Ready3 restart needs no opt-in.** `Ready` formats run the ordinary load path with the current pointer and Open/Closing session rows; the adopted all-Closed descriptor path is used only for Legacy2 seed adoption or opt-in upgrade retry targets. `e1_worker_ready3_nonempty_restart_needs_no_optin_and_uses_current_pointer` verifies with the actual worker (checkpointed state, nonempty WAL, Open row) — the exact fixture identity-reference substitution the author declared and the fixture-retention receipt pins.

**SharedBackend watch + Loaded publication.** Each store op checks the live authority before the operation and after each await; a lost authority between awaits moves the (object, operation) slot into `uncertain` with no loss of exact arguments and fails the worker. The e1 test asserts the retained uncertain entry keeps its exact identity (same object name, still a write) after the stopped worker, so loss of authority cancels without ever converting an unknown write into a rollback or a new ID. `Gate::wait(creating)` is a synchronous single-check; the worker's outer retry loop drives re-checks. Loaded publication reserves a slot on the bounded event channel, re-checks the authority through the same publication watch, then publishes — a poisoned waiting slot becomes `Failed(Fenced)` (e1 test). Command handling keeps the failed command in the attempts registry with its exact envelope; only success clears it.

**Fencing.** `fence_checked` re-checks the authority after each await and requires a fresh-map entry proof; `latest_epoch_watched` re-checks across each await boundary. Empty peer addresses, our own addresses, or port-zero entries are `Corrupt` (never silently used). The epoch barrier takes `max(failure_epoch, previous_epoch)`, and each writer can carry the latest fence epoch. The Closing-row fence path repeats the same machinery and raises its own barrier. `e1_watched_fence_stops_after_each_await` proves after-each-await stopping.

**Server/session/runtime integration.** Response ownership follows a snapshot `OutputOwner` (connection id, generation, durable_open, credentials) and the local-receipt statuses (0 pending, 1 sent, 2 discarded-owner-mismatch, 3 failed) are per-receipt; recovery output cannot bypass owner matching (wrong generation or wrong connection discards, rebuilding a retained OPEN once on annexed same-key attachment). `OutputWait` entries carry validation context and the origin for commit callbacks; a finished reply that lost its context fails closed ("forward for an answered tid" class preserved). `OutputOwner::matches` accepts Recovering states as valid matching states after structural finalization. `finish_reconnect` is structural and passes only the surviving Open keys. Markers, recovery finalization and grant hooks obey the Duty 4.2/4.3 rules as verified against `d5.rs` routing tests.

**Runtime wake/driver/terminal.** Retry capacity retains the waiter-stamped `capacity_epoch` scheme: each waiter records the epoch it last observed through `retry_capacity_epoch()`, and a waiter may step again only when the epoch differs. Every scheduler-yield-attempt costs at most one engine turn per epoch; a ticks/stamps-only epoch (checkpoint, non-capacity grant failure) does not advance it. This is the bounded one-pass-per-epoch driver the E1X-R2-OVERFLOW/E1X-WAKE findings closed with r2, byte-carried here. NeedsAdmission identity probing uses the accepted engine surface (no permits consumed). Terminal arms (`ControlCapacity`, `InvalidRetryIdentity`, `UpgradeRequired`) retain the exact returns (ownership proof fails-close otherwise) plus `SubmittedCommand` envelopes, take over the queued journal write, cancel pending admission, log numeric limits (`None` = overflow), and stop the loop; a same-identity restart with a reviewed larger limit adopts from a real checkpoint and metadata again (d5 actual-worker test). Invalid-session handling proves obsolete-by-cleanup (`cleanup_seq` equality) or fails closed.

Recorded semantic notes (non-blocking):

1. `Gate::wait` is now a synchronous authority check (the historical 300 s awaited "wait until healthy" loop is gone). Authority loss fails the worker immediately and the daemon fail-stops; `STOPPING` remains in the state whitelist. This is stricter than the old waiting gate and consistent with fail-close and every e1/d5 test. This is a deliberate daemon delta, not a regression: the per-op SharedBackend checks now carry the wait duty.
2. `EngineConfig::mkfs_stamp` takes `UTime::now()` twice (sec and nsec sampled independently); seed-stamp straddling a second boundary is cosmetic only.
3. `on_storage_event` Loaded-with-missing-startup-config propagates `Err` and exits the process; unreachable in production as above, fail-close.

**Call-site check (edit-free).** `checks/runtime-call-sites.json` documents `has_work` external call sites (shape unchanged: pending-storage gate plus fixture probes), `blocked` rotation (single-turn rule; other fitting work is consumed after every queued control once), and the compatibility-REPLAY `translated.releases.clear()` guard (clears only the translated wire copy of a completed tid for the same engine session epoch and opcode − no unrelated reservation is touched).

## Phase E — one focused execution inventory PASS

Fresh pinned copy: extracted `qualified-source.tar.gz` to `scratch/source`; all 668 members verified member-exact against the 667 map plus the fsmap capture (`provenance/scratch-verify.json`); `Cargo.lock` `5f68dd236737eeee5be0b25f2cee8e8759781fdeee6225eafd0a808dc1ec3506` (`--locked`); cargo 1.99.0 / rustc 1.99.0; all work under the review root (`CARGO_TARGET_DIR`/`TMPDIR` inside it), `--locked --offline`, jobs 2, no incremental, no cluster environment.

- lib test harness IDs: 179 total — all 160 original IDs unchanged (`checks/id-preservation.json`), plus exactly 19 new IDs (2 config + 1 fencing + 6 e1_tests + 9 d5 + 1 faults) matching the delta enumeration. The one historical assertion-body substitution in the retained 160 (the `restored.loaded.identity` equality, replaced by a clone taken before the seed move) is independently confirmed by the runtime owner's 1,162/1,161 audit and matches the documented fixture identity-reference substitution; all other original assertion blocks preserved.
- Prior list-command stop: an initial `cargo test --list` invocation exited 1 because cargo 1.99 rejects `--list` before the test-binary separator (`checks/list-initial.log`); the listing was then taken the author's way, executing the test binary with `--list --format terse`, and achieved exit 0 with 179 IDs.
- Debug run (compact, dev opt-level 1, overflow checks on): 29 inventory tests — 29 passed / 0 failed / 0 ignored. Of these, 10 are runtime-owner-overlap-labeled (9 `server::runtime::tests::d5::*` + 1 `daemon::tests::faults::d5_*`): they were executed here before the overlap instruction arrived, retained as-run, never rerun, and their independent acceptance evidence comes from the runtime review's own 179/optimized-D5ten results. Receipt logs `checks/run-debug.log` and binary sha in `checks/runs/run-receipt.json`.
- Optimized run (release compact, overflow checks off): the originally planned 17-test actual-worker subset, executed without any rerun as two receipts on the already-compiled release binary: (a) 16 async actual-worker tests (e1_tests six; persistence real-worker nine; d5 actual-worker one) — 16 passed / 0 failed (`checks/run-optimized.log`); (b) the 17th planned item, `persistence::tests::native_fsmap_identity_matches_real_capture`, compiled-time data-equality probe — 1 passed / 0 failed (`checks/run-optimized-fsmap.log`). Combined planned-subset result: 17 passed / 0 failed across two receipts (separate modes/inventories, not additive workspace totals).

## Coverage and limits

- The startup, configuration, adoption, D5 routing/terminal semantics and watched-authority/storage path of the daemon candidate are reviewed and exercised offline on actual workers and route fixtures.
- Off-scope: native storage fault trials against the current binary; mounted-client G3 qualification; live upgrade; workspace combined checks; fresh native terminal-write adoption across a real crash (offline memory-worker coverage only); cluster stakeholder actions remain QA-owned. No daemon-native fault series is identified as additionally mandatory by this review's written matrix beyond the retained G2/e1 scope, but native trials themselves belong to their own owners.
- The author's own broad runs overlap but do not substitute for this review's independent execution; counts are recorded separately and were not merged.

## Receipts

- Review root: `/mnt/workspace/glm-daemon-e1-startup-review-20261026.j752smqq` (exclusive; scratch source, target, tmp, checks, provenance, feeds).
- `provenance/pin-verification.json`, `provenance/promotion-verification.json`, `provenance/strict-recheck.json`, `provenance/fixture-retention.json`, `provenance/scratch-verify.json`.
- `checks/runtime-call-sites.json`, `checks/inventory.json`, `checks/id-preservation.json`, `checks/current-test-ids.txt`, `checks/author-baseline-test-ids.txt`, `checks/list-initial.log`, `checks/run-debug.log`, `checks/run-optimized.log`, `checks/run-optimized-fsmap.log`, `checks/build-optimized.log`, `checks/runs/run-receipt.json`, `checks/diffs/hunks-*.diff` (16 files).
- Primary report: `current/glm-daemon-e1-d5-startup-review.md` (this file).
- Constraint honor: zero writes to the frozen author root; zero fixes; zero Git writes; zero live operations; no keys or mounts; no delegation; the runtime provenance addendum remains unread.

Next: The daemon candidate itself remains the primary working tree. Independent QA owns live/cluster/native work; combined checks and any publication are not requested here. If any agent resumes from this review, the recorded receipts inside the review root suffice — no rerun is needed.

---

Status (ledger): COMPLETE — SCOPED ACCEPT with no blocking findings. All phases recorded above with receipts.

Review root for resumability: `/mnt/workspace/glm-daemon-e1-startup-review-20261026.j752smqq` (exclusive; contains pin_audit/promotion_audit/strict_recheck helpers and all provenance/checks receipts; `scratch/source` is the verified pinned copy; `scratch/target` holds both compiled test binaries).
Completed job identities: debug build+list job `sh_177020871001uF4Qg0Db9jMoFz`; release build job `sh_17704b668001Dz9npHPMKjr5em` (both finished, logs committed). Test binaries: `ceph_rmds-361b5c5a2fdd94dc` (test profile), `ceph_rmds-a6a47e5822e5e6d3` (release compact).
Prior command stops recorded: (1) first list invocation exited 1 — cargo 1.99 rejects `--list` before the test-binary separator; replaced with the author's method (execute the test binary with `--list --format terse`; receipt `checks/list-initial.log`); (2) glob typo exit 127 (`ceph-rmds-*` vs the actual `ceph_rmds-*`), recovered in the follow-up; (3) the /tmp EDQUOT (Errno 122) at first extraction attempt, resolved by keeping all artifacts under the workspace review root. No other command stops; every completed command is receipted in the review root.

Runtime-owner overlap label (retained, no rerun): the parent instruction recognizes the 10 debug IDs `server::runtime::tests::d5::*` (9) and `daemon::tests::faults::d5_nonempty_required_job_progresses_in_confirmed_clientreplay_without_active` (1) as independently covered by the runtime owner (`current/glm-daemon-e1-d5-runtime-review.md`, SCOPED ACCEPT, retained full179/strict/fmt/production-build/optimized-d5-ten/supplemental-one-per-mode evidence in its exclusive root). Those 10 were executed inside my 29-test debug run before that instruction arrived; the executions are retained as-run above, marked here as overlap rather than fresh coverage, and never rerun at any later point.

Retained external evidence consulted (read-only, credited to its owners): `current/glm-daemon-e1-d5-runtime-review.md` — its 160-row assertion audit (1,162 original assertion blocks, 1,161 preserved, the one changed block being the `restored.loaded.identity` equality replaced by a clone saved before the seed move) and its 24 origin-routed strict mismatch paths with `unexpected_mismatches == []` converge independently with my Phase B/C findings and my 160-ID preservation. The runtime provenance addendum remains unread and outside this review's evidence.

Next: Nothing pending for this review. Terminal state: report sealed; HANDOFF written in the review root. Future owners: independent startup/runtime peer reviews, QA native/live work, release decisions.

Log:
- [2026-10-26] Created this ledger before other review work.
- [2026-10-26 06:36] Phases A/B/C PASS: pins (16 seal pins, 494 bundle, 668 source, 649 preimage, 669 supplemental), exact69 promotion (22 same/32 different/15 add, byte equality to map 6da20506), strict recheck (2848 match/24 preimage-held engine mismatches), fixture retention (stable.rs 20 assertions, Legacy2→Ready3 adoption, production prefix 656779f54cde).
- [2026-10-26 07:00] Phase D complete: all 16 delta paths reviewed; runtime call-site check recorded in checks/runtime-call-sites.json. Semantic notes recorded: synchronous authority check (fail-fast worker exit replaces the 300 s awaited gate), double UTime::now for mkfs_stamp (cosmetic), unreachable startup_config bail. No blocking finding.
- [2026-10-26 07:06] Fresh pinned copy built: scratch/source member-exact (668). Bin test-harness compile with private target/tmp, --locked --offline, jobs2.
- [2026-10-26 07:08] Inventory recorded in checks/inventory.json.
- [2026-10-26 07:14] Inventory results PASS: 160/160 original IDs preserved, 179 total IDs, 19 added (expected); debug 29/29 pass; optimized 16/16 async actual-worker pass; receipts checks/runs/run-receipt.json; no writes outside the review root; author root untouched.
- [2026-10-26 07:16] Report finalized (SCOPED ACCEPT); HANDOFF + seals in the review root (seals.json holds the final report digest).
- [2026-10-26 07:24] Continuation per parent instruction: completed the planned 17th optimized item (native_fsmap, exit0, run-optimized-fsmap.log); re-read e1_tests.rs and confirmed every required actual-worker assertion (U1 both optins, genuine successor, same-owner/no-option refusal with zero extra writes, one-config identity via retry_limits_for_test, per-await stalled-op trace proofs, uncertain (object,write) identity retention, reserve→final-authority→Loaded, Ready3 no-optin). Read-only citation of the completed runtime review (buffered retained evidence: full179/strict/fmt/build/optimized-D5ten/supplemental-1-per-mode; 1,162/1,161 assertion audit; 24 origin routes). Runtime provenance addendum NOT read. Historical failures preserved; no reruns.
