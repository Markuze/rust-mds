# sol-daemon
Task: Add the explicitly assigned narrow strict-CI cleanup to the completed D6-R1 candidate. Reproduce current daemon -D warnings failures and fix only Config.cluster, three obsolete rados spawn wrappers and production-unused Session::push_seq/SessionMap::iter_mut. Preserve all R1 and accepted recovery/configuration behavior. No outside-code/live/Git writes/commits/subagents.
Status: Combined R1/strict-CI author candidate is complete and HELD for joint independent review. Current strict-Clippy red reproduced the six approved sites. Final exact -D warnings command passes with no warnings. Full daemon remains 160 passed, 0 failed/ignored; no tests added or removed. Config/data/D6/session/G2/G3 checks, production build, all 20 daemon Rust format checks and scoped whitespace pass. All 132 source/contract paths, modes and hashes stayed unchanged across final verification. R1 behavior/evidence and every checked data/configuration path are preserved. Prior acceptance scopes and nonempty D5 hold remain unchanged. No independent R1/lint acceptance or release is claimed.
Context: Own crates/mds/** and current/sol-daemon.md only. Use CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon and Sol max. Preserve R1 ownership/accounting, every old artifact, the reviewed binary/tooling and all isolated release snapshots. Monitor CommandWaiter and the store test-only E2 migration are independently accepted external inputs; do not edit them. E1 engine/store work remains isolated. No broad refactor, fabricated usage, dead-code allowance or lint-gate change is authorized.
Next: Hold all assigned files. Coordinator assigns joint independent review of the R1 guard and the exact six strict-CI sites against the new combined manifest/archive. Preserve old R1 evidence and all release snapshots. Only a later explicit release-owner assignment may prepare a combined candidate for publication. No E1/D5/nonempty activation, live action, snapshot edit or Git work is authorized here.
Log:
- [2026-10-25] FINAL combined verification PASS with CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon: cargo clippy -p rmds --all-targets --no-deps --locked -- -D warnings = exit 0, no warnings. Existing test filters pass: config::tests 4, rados::tests 14, g3_d6_ 26, session::tests 15, g2_r 11, g2_fault_ 4 and g3_ 70. Full cargo test -p rmds --locked --quiet = 160 passed, 0 failed/ignored. Filters overlap. No tests added or removed. Production cargo build -p rmds --locked is warning-free. All 20 daemon Rust sources pass owned rustfmt check; scoped whitespace passes. No workspace/strict-dependency/live claim.
- [2026-10-25] FINAL combined provenance PASS: 129 Cargo inputs / 132 source-contract paths stayed unchanged in path, full regular-file mode and SHA256 through final checks. Combined fingerprint: 6d46a8ffb9132e567adf1fbb3f1f8ad9ae7b3f766f95f297bd200757376fe4d8. Combined release manifest SHA256: 7b4b184505c6172fa522f5355f9ffcea2580fe2de1dbbe4c52470d4a4525c2e4. Archive combined-source.tar.gz and canonical release-source-path-mode-sha256.txt are in /tmp/opencode/sol-daemon-r1-strict-ci-20261025.aKufTD/. Machine proof confirms only config.rs/rados.rs/session.rs changed from completed R1, exactly as approved. Every other source/contract, outside crate, manifest and test remains unchanged. Verified all original R1 artifact-inventory hashes, including genuine red/green evidence. Files are HELD for joint independent review.
- [2026-10-25] Fresh cleanup evidence: /tmp/opencode/sol-daemon-r1-strict-ci-20261025.aKufTD/. Current strict command cargo clippy -p rmds --all-targets --no-deps --locked -- -D warnings exited 101 before edits: Config.cluster config.rs:100; spawn_purge/spawn_truncate/spawn_recover rados.rs:348/357/373; Session::push_seq session.rs:305; SessionMap::iter_mut session.rs:593. Production has six errors; test target has four. All 132 pre-cleanup paths/modes/hashes stayed unchanged across reproduction/use inspection. The 129-input fingerprint matches completed R1: a3665910ca276c03b3c75861be406502c22a92c1f13061210c6e2d8433eba69a.
- [2026-10-25] Use proof retained in uses-*.txt. Config.cluster has zero readers even in tests, so remove only the Config field/initializer, not the local --cluster variable. The local value still selects /etc/ceph/{cluster}.conf and expands configured/default keyrings; CLI/env precedence and startup log fields are unchanged. Both session helper getters have test callers only; cfg(test) preserves those callers and the production push counter/validation. Workspace Rust search found only the three spawn wrapper declarations, no callers. The accepted Daemon::drive_jobs already awaits rados::execute_lifecycle and routes tagged completion through on_control_event -> lifecycle_completed -> EngineInput::LifecycleCompleted. Keep that executor, checked purge/truncate/probe primitives, error handling and tests byte-identical. No invented use, lint allowance, gate weakening or unrelated API removal.
- [2026-10-25] Coordinator explicitly added the six remaining strict-CI diagnostic sites to the R1 candidate. The three integration-obsoleted fire-and-forget wrappers may be removed only after proving no callers and replacement by the accepted checked executor. Test-only configuration/helper fields may use cfg(test) only after complete use inspection. Preserve exact --cluster/default configuration/keyring behavior. No new tests are required for dead-wrapper/cfg cleanup; retain all existing R1/G2 evidence. No snapshot or Git coordination is authorized.
- [2026-10-25] FINAL R1 validation ran once after source stabilized, with CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon. g3_d6_ = 26 passed; session::tests = 15 passed; g2_r = 11 passed; g2_fault_ = 4 passed; g3_ = 70 passed; full -p rmds --locked --quiet = 160 passed. Every filter has 0 failed/ignored; counts overlap. Production build and daemon-only --all-targets --no-deps clippy pass with the existing six production/four test dead-code groups only. All 20 daemon Rust sources pass scoped rustfmt; scoped git diff --check passes. No new lint, workspace/strict check or live claim.
- [2026-10-25] FINAL provenance PASS: 129 Cargo inputs and 132 source/contract paths have unchanged modes/content hashes across final checks. Post-fix fingerprint: a3665910ca276c03b3c75861be406502c22a92c1f13061210c6e2d8433eba69a. Release manifest SHA256: ada321fbb0c9b54e38ec743ef6019154a608499f6739aa7cc6f6fefa85f9386a. Fresh source archive and executables are retained under /tmp/opencode/sol-daemon-d6-r1-20261025.D7tKz0/. Exactly five owned source/doc paths differ from the initial manifest. The two unrelated external paths are documented in source-delta.json. Engine sources, Cargo.lock/manifests, D6 cleanup/timer source and G2 fault sources are unchanged. The original independent probe text/executable/log/release manifest still match their initial hashes.
- [2026-10-25] Implemented reclaim-specific PendingOutput at Session::send_raw, so existing-target success, absent-target success and every error reply retain the same output guard. It captures client, runtime generation, durable SessionKey/open_id, connection ID, address and credentials. Session::transmit requires exact equality plus Open/Stale immediately before ClientLink handoff. Normal/recovery Open callbacks update the observed durable decision tag, including a new committed attachment refused by current service policy. No engine/store API or D6 target-cleanup change.
- [2026-10-25] Initial unchanged six-test R1 matrix is green (6 passed). Expanded eight-test R1 run passed, four reclaim session-output tests passed, then the additional refused-attachment production case passed. Tests exercise actual bounded queue drain and transport calls, not only ownership helpers. New checks cover retired CLOSE funnels, valid Stale, isolated owner fields, exact byte/count release, reuse of the freed slot, no fake immediate-send acceptance, per-session/rank overflow and sticky immediate/queued transport failure. Final formatting preceded the latest production-case run.
- [2026-10-25] Fresh evidence: /tmp/opencode/sol-daemon-d6-r1-20261025.D7tKz0/. Initial 129-input fingerprint matches the reviewed pre-fix tree: 3fc3c84314b2727b5ffe6b642ed06b6e9add5000636cbd15c1472c5222e0c115. Exact production-dispatch/durable-OPEN/32-drain regression failed at "old reclaim completion must not migrate to a replacement attachment after enqueue". R1 matrix: 1 passed / 5 failed, 0 ignored. Same-connection new OPEN, Closing, immediate absent reply and valid-new-attempt cases also fail; the unchanged positive FIFO/byte/push test passes. No test setup/compiler failure occurred.
- [2026-10-25] Provenance qualification: an external monc CommandWaiter type alias appeared after the first red build. Repeated the six tests, again 1 passed / 5 failed. The repeat before/after manifests then detected an external store/stable.rs test-only E2 caller migration. No daemon production source changed in either red interval. No stable whole-tree red fingerprint is claimed. Preserve both manifest pairs/logs/executables; use fresh stable post-fix before/after manifests for final evidence. This owner made no outside-code edit or Git write.
- [2026-10-25] Coordinator released primary daemon paths for D6-R1 only. Re-read current/sol-daemon-review.md lines 388-598 and retained independent-probes.txt/provenance. Confirmed the exact 40-output/32-drain/durable replacement OPEN interval. Prior D1-D4/clientless E2 and G2 fault acceptance remain unchanged. The old isolated candidate is SUPERSEDED for release; no snapshot or Git coordination is permitted. Preserve fs44/meta88/data89, both acknowledged namespaces, reviewed binary/tooling and all prior evidence without live access.
- [2026-10-25] Final source-stability check PASS: all 129 source/manifest inputs are byte-identical before/after verification; fingerprint remains 3fc3c84314b2727b5ffe6b642ed06b6e9add5000636cbd15c1472c5222e0c115. Seventeen daemon Rust paths and the unchanged proto reconnect comment pass scoped rustfmt. Scoped git diff --check passes after README/ledger updates. Cargo.lock and both relevant manifests have no diff. Logs include d6.log, g2-fault.log, g2-regressions.log, g3.log, daemon-full.log, build.log, clippy.log, fmt.log, diff-check.log and before/after SHA256 manifests. Package is frozen for independent review; no pending source correction or dependency blocker remains.
- [2026-10-25] FINAL D6/G2-fault Cargo checks in target-sol-daemon: cargo test -p rmds --locked g3_d6_ -- --nocapture = 17 passed; g2_fault_ = 4 passed; g2_r = 11 passed; g3_ = 61 passed; full --quiet = 147 passed. All have 0 failed/ignored. Filters overlap and are not additive. cargo build -p rmds --locked and cargo clippy -p rmds --all-targets --no-deps --locked pass. Production reports the existing six dead-code groups (Config.cluster, spawn_purge/spawn_truncate/spawn_recover, Session.push_seq, SessionMap.iter_mut); tests report four. Removed the new match-like-match and lexical-lock-scope lint findings. No new unused production item, broad lint disable, strict/workspace or live check.
- [2026-10-25] Retained final logs/provenance at /tmp/opencode/sol-daemon-d6-g2-fault-20261025.9LwREj/. HEAD read as 85a37c8d615992b4600d7467fdedd514fecfac03. The 129 Rust-source/manifest inputs have fingerprint 3fc3c84314b2727b5ffe6b642ed06b6e9add5000636cbd15c1472c5222e0c115 before final checks. Preserve the manifest and logs for review. No Git write, commit, branch change or live action ran.
- [2026-10-25] Expanded D6 coverage reached 15 passing tests, then added clamp/blocklist cases. Genuine follow-up reds found duplicate live UUID acceptance at checked restore, a same-connection changed authorization context inheriting a reply, unknown request activity extending the clock, and a valid authenticated renewal behind 65 held messages missing timer accounting. Fixed each owned boundary. Held activity now validates at receive time without admission/output/state changes. Existing STOPPING engine/checkpoint stepping is preserved; only its active-session timers stay disabled.
- [2026-10-25] New fixture-only failures are not source findings: the real Nautilus+ OSDMap needs ANY-type blocklist entries; WAL-only replay can reset the volatile cap-ID cursor while preserving all namespace/allocator/range/history/job/receipt fields. The fault comparison now checks every required field and explicitly excludes that volatile cursor across restart. Namespace tests use a committed inherited fault-data namespace and an unrelated sentinel. Within a running process, failed data I/O leaves the complete engine checkpoint unchanged.
- [2026-10-25] D6 baseline feature red: g3_d6_ ran 4 tests, 0 passed / 4 failed. Intended failures: FINISH incorrectly replied; pending OPEN did not reserve UUID; ordinary idle session stayed Open at 60 s; RESET produced no durable target fence. Added runtime/d6.rs, bounded 128 aggregate reclaim waiters, requester key/generation/connection/open-id checks, UUID admission reservation over durable rows plus pending OPEN, strict decimal timer policy and full-union fence reuse. All four tests now pass. An intermediate full daemon run passed 130 tests.
- [2026-10-25] Four g2_fault_ tests pass: actual worker restart at durable Closing, cleanup decision and terminal receipt; six checked join negatives; real Objecter/executor partial purge/truncate/probe failure then metadata/runtime recreation and retry; actual completion WAL before/failure/after-durable callback cuts plus old-owner/old-attempt/duplicate control results. Added test-only Worker/Peer harness and deterministic OSD transport. No production fault seam or engine/store changes. Initial test compile field/type errors, zero-size purge/one-object probe setup and an incorrect expectation that daemon on_storage_event itself returns fatal are fixture/oracle setup, not production red evidence. Setup now commits real size/range changes and revoke acknowledgments; fatal is checked at the actual daemon failure boundary.
- [2026-10-25] Read the coordinated AGENTS change: both E1 contract scopes are now accepted. E1 source remains outside this assignment and primary-tree verification stays on accepted E2. No upgrade, D5 or nonempty-finalization permission follows from contract acceptance.
- [2026-10-25] Assigned D6 and both audited G2 fault-test families after independent D1-D4/clientless-E2 acceptance. Fresh cargo test -p rmds --locked --quiet in target-sol-daemon: 126 passed, 0 failed/ignored, four existing test warning groups. Same mds/own-ledger/resolution-only Cargo.lock ownership. No further proto change planned. No engine/store/platform/tooling or live/Git-write/subagent permission. Preserve accepted source boundaries, fs44/meta88/data89 and all retained evidence.
- [2026-10-25] FINAL correction verification, all Cargo commands with CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon: cargo test -p rmds --locked g3_c -- --nocapture = 7 passed; g3_ -- --nocapture = 44 passed; g2_r -- --nocapture = 11 passed; --quiet = 126 passed. All have 0 failed/ignored. cargo build -p rmds --locked and cargo clippy -p rmds --all-targets --no-deps --locked pass. Build/clippy report six existing dead-code groups: Config.cluster, three log-only rados wrappers, Session.push_seq and SessionMap.iter_mut. Tests report the first four groups. No new lint or dependency compile blocker. Ten scoped Rust files (nine daemon files plus proto reconnect comment) pass rustfmt --check --edition 2024 --config skip_children=true. git diff --check -- crates/mds crates/proto/src/reconnect.rs Cargo.lock current/sol-daemon.md passes. No workspace/strict/live checks ran.
- [2026-10-25] Repeated the delayed-marker red: 0 passed / 1 failed at the intended returned-permit assertion, with 125 filtered out. reserve failure now distinguishes a busy adapter from WAL capacity. Controls remains pending without checkpoint_wanted/pressure. The unchanged regression then passed: returning the held permit schedules one fresh marker turn, establishes readiness, changes no sequence and transfers no storage command. The full 126-test pass includes the revised persistence scheduler and both lint corrections.
- [2026-10-25] Resume checkpoint: delayed-marker-permit test reproduced a real failure at "a returned permit needs no unrelated checkpoint callback". A held RecordPermit must retain Controls without requesting a checkpoint. The revised persistence test scheduler now drains both pending storage and runnable production work. Full verification is pending after that fixture change and two lint corrections. The prior 125/43 counts do not include this new regression.
- [2026-10-25] E2 migration red: g3_e2_empty_checked_restore_needs_fresh_admitted_marker_without_commit_callback failed because structural finish returned readiness immediately. Added RecoveryDrain::{Reconstruction,Controls,GrantsReleased}, private marker obligation in pump, fresh RecordPermit execution and independent readiness/output drain. The marker submits no WAL record in the empty case and returns its permit. Clientless fixtures now exercise this production path, not a compatibility readiness override. A fresh full run after migration passed 117 tests before C1-C3 additions.
- [2026-10-25] Genuine C1 red: authenticated loopback clients reached Daemon::on_msgr_event/on_client_raw after an all-Closed ACTIVE load; v3 reconnect remained in reconnect_held. The separate nonempty early-map-race test already passed. Fixed checked terminal dispatch and raw-queue consumers for clientless/all-Closed startup, including packets queued before load. Closed/late uses ordered CLOSE; unknown uses sessionless REJECT. A current active key waits for verified fence and durable Closed; an obsolete normal CLOSE sequence can join this fence.
- [2026-10-25] Genuine C2 reds: a first v3 final at t0+44 left the deadline at 45 rather than 66.5 s; a later first final failed to extend it to the 90 s hard bound. Record valid chunk activity before first-final gather removal. Tests now cover first v3, empty v5 final after partial, remaining member at 65 s, completed-member duplicates and the fixed hard bound.
- [2026-10-25] Genuine C3 red: a failed renewal send left fatal_error unset in a pump with another key's durable Closing ready for cleanup. Return failure from renewal output, check fatal/output immediately after drive_recovery, and stop before another engine/worker turn. The test also fills the paused funnel to 1024 messages. Both cases now retain the other key/receipt state and submit/transfer nothing after output failure.
- [2026-10-25] Focused corrections passed 6 tests with g3_c (includes the existing cold-follower test). Expanded G3 run passed 43 tests. Additional coverage checks CREATING/STARTING/already-ACTIVE loads, no-callback empty marker, retained RecoverSize progress before release, and actual-worker mkdir completion/history before restart. No engine/store changes or owner polling occurred.
- [2026-10-25] User assigned G3-C1/C2/C3 and the now-required clientless E2 caller migration. Reviewer released files. Own mds, this ledger, resolution-only Cargo.lock and only the existing ceded proto comment; no engine/store or report edits. Retain target-sol-daemon and the explicit nonempty finalization hold. Do not treat concurrent fixture setup failures as source-regression evidence or add a compatibility readiness bypass.
- [2026-10-25] FINAL D1-D4 verification after receive-clock preservation: cargo test -p rmds --locked g3_ = 34 passed; cargo test -p rmds --locked g2_r -- --nocapture = 11 passed; cargo test -p rmds --locked --quiet = 116 passed, 0 failed/ignored. cargo build -p rmds --locked and cargo clippy -p rmds --all-targets --no-deps --locked pass. All use target-sol-daemon. Clippy/build report only existing Config.cluster, three log-only rados wrappers, Session.push_seq and SessionMap.iter_mut dead code; tests report the first four groups. No strict/workspace lint or live claim. Nine touched Rust files pass rustfmt --check with edition 2024/skip_children=true; scoped git diff --check passes.
- [2026-10-25] Received coordinated AGENTS update and read the revised independent report: E2, D6 and matching wire REPLAY have scoped contract acceptance. This does not authorize widening this first D1-D4 package or treating old finish_reconnect as E2. Keep nonempty finalization closed pending the engine implementation and coordinated caller migration. Ordinary E1/clean-session upgrade policy remains under separate review; no live upgrade is authorized here.
- [2026-10-25] Full daemon run reached 116 pass, 0 fail/ignore; build and daemon-only clippy passed with only six existing production dead-code groups (four in tests). Owned rustfmt and whitespace checks passed. Subsequent small audit edit preserves daemon receive timestamps through both raw queues and the shared dispatch boundary; its focused deadline test passed. Repeat final verification after this edit.
- [2026-10-25] Final audit: late current reconnect now sets Closing.reply and retains CLOSE until Closed, including during Closed publication. Unmatched late reconnect uses a bounded terminal. Matching terminal responses reuse the existing connection funnel. EvictClient uses the passed Runtime while detached from Server. Captured four genuine pre-fix failures; all now pass. Full daemon run after those fixes: 106 passed, 0 failed/ignored. The laggy fixture refresh correction is verified.
- [2026-10-25] Captured and fixed four further genuine failures: unhealthy REJOIN advanced its requested state; post-final same-connection credential generation could ignore a second stream; changed rank incarnation left control permission open; a required-feature map change after attachment still allowed staged reconstruction. Tests now recheck current owner/self blocklist, phase health, full final Token and current attachment authorization/features. The offline fixture now uses the same gid in membership and loaded owner, as production requires.
- [2026-10-25] Focused G3 run reached 33 pass, including 65536 retained locks with atomic rejection of the next chunk, deadline versus pending address write, feature changes at completion and after silent attachment, and explicit no-stale-GRANT behavior. Added unlinked-open/hard-link reconstruction test passed separately. Its fixture now acknowledges actual revoke messages before checkpoint; initial missing-reply/setup and field-name compile errors are not pre-fix product evidence.
- [2026-10-25] First final clippy check completed with existing dead code plus new style/API warnings. Corrected the new collapsible-if, clone-on-copy and test initializer warnings. Documented narrowly scoped argument-count allowances at the recovery input boundary. Production now uses separate permissions; pause_data_work is a compatibility test helper. Final clippy/build/test checks remain next.
- [2026-10-25] Resume checkpoint: actual native store worker over MemoryBackend passed g3_real_worker_persists_attachment_union_before_production_reconnect_dispatch (1 pass). The test verifies persisted address union and authorization context before delivering the tagged Open result. Pending audit: late reconnect CLOSE must wait for Closed on its current funnel; terminal responses must not bypass an existing connection funnel; EvictClient effects must use the passed Runtime while Server.runtime is temporarily taken.
- [2026-10-25] Implemented collection-only recovery in server/runtime/recovery.rs. Sticky nonempty finalization barrier, silent durable attachment, connection/generation checks, atomic translated chunks, authoritative realm corrections, retained budgets, 45/90-second default deadline and full-union missing-key fences are present. Last focused G3 run: 19 pass. Last full run: 100 pass / 1 fail (laggy fixture permission refresh). Both counts precede the new actual-worker test and final audit corrections.
- [2026-10-25] Before production edits, full daemon baseline passed 82 tests. Two g3_ regressions then failed at the expected boundaries: early retry did not start attachment, and production reconnect dispatch did not reconstruct caps. Both pass after integration. No E1 retry policy or E2 engine API change is used. Cleanup GRANT effects during nonempty recovery fail closed, not buffer stale grants.
- [2026-10-25] Explicit source thaw for D1/D2/D3/D4. Own crates/mds/**, dependency-resolution-only Cargo.lock, this ledger and only the mixed caps/realms comment in crates/proto/src/reconnect.rs. Preserve fs44/meta88/data89 and all evidence without any cluster access. Retain finalization barrier for nonempty recovery; D5 retry/drain and D6 reclaim/active timers remain next-package work. Target target-sol-daemon.
- [2026-10-22 20:08] FINAL correction verification with CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon: cargo test -p rmds --locked g2_r = 11 pass; cargo test -p rmds --locked = 82 pass, 0 fail/ignore; cargo build -p rmds --locked passes; cargo clippy -p rmds --all-targets --no-deps --locked passes. Clippy/build report six dead-code warning groups; tests report four. No new lint, dependency compile failure or live action. rustfmt --check --edition 2024 --config skip_children=true for the five touched Rust files and git diff --check -- crates/mds current/sol-daemon.md Cargo.lock pass.
- [2026-10-22 20:08] Correction scope: daemon.rs, server.rs, server/runtime.rs, session.rs, new server/runtime/tests.rs, crates/mds/README.md and this ledger. No engine/store, persistence implementation, journal adapter, manifests, or Cargo.lock edits in this correction. Existing uncommitted work remains intact. No commits, subagents, branch changes or live calls.
- [2026-10-22 20:08] R1 binds raw dispatch to the durable row's credentials, exact active address/nonce and recorded runtime generation. Pending attachment stops dependent input for that client. Unmatched requests/caps/session controls are dropped before engine admission or output. Same-address resets with unchanged credentials remain admitted. Owned Open completion must match its captured runtime generation; a mismatch cancels pending input/reservations and fail-stops, never publishes into the replacement generation. Delayed completion tests retain both A and B in the durable fencing union. The daemon test proves B-only blocklisting cannot release rights; both addresses and the applied epoch are required.
- [2026-10-22 20:08] R2 retains requested CLOSE seq through Drain. It rechecks current transmitted push_seq and absence of queued counted output immediately before submitting Closing. Invalidated CLOSE enters WaitSeq: runtime admission stays stopped; durable Open, caps, histories and SessionKey remain intact; no Closing/cleanup/CLOSE is sent. A current-seq retry is accepted in that state. Verified fencing can replace the sequence requirement. Engine execution pauses during the validated Closing write. Non-counted terminal backlog still drains FIFO before exactly one CLOSE after Closed.
- [2026-10-22 20:08] R3 reserves each distinct new OPEN by retaining new_session on its pending operation. Budget = live durable keys + pending-new operations, at most 1024. Duplicate pending OPEN/attachment consumes no new slot. Capacity drops a new OPEN before the repository command, with no row to discard. Successful Open atomically converts reservation to live key; Closed releases it. An unrelated owner/op callback leaves the real reservation held. Owned failure, cancellation and stale-generation fail-stop release pending reservations and byte accounting. The interleaving test fills 1024 slots, installs two, attempts more OPENs, checks every completion, closes one and reuses exactly one slot.
- [2026-10-22 20:08] R4 rotates a Busy control behind other controls and retains the pass count across bounded pump turns. Only a full all-Busy pass parks the runtime. New input/commit or actual execution progress resets the pass. The regression uses a real RecoverSize intent, 37 dirty updates without write caps, then LifecycleCompleted. One-step has_work/pump turns start and commit the completion without Tick/new client input. Deferred updates then drain (unmatched caps produce no false ack). The all-Busy pass preserves input bytes/permits and stops scheduling. A record submitted on the last turn now transfers to the worker before pump returns, preventing an outbox-only lost wakeup.
- [2026-10-22] Correction baseline: cargo test -p rmds --locked --quiet = 71 passed. Targeted red command cargo test -p rmds --locked g2_r -- --nocapture = 0 pass / 5 fail at the intended boundaries: R1 submitted seq 3 instead of 2 from unattached B; R2 obsolete CLOSE emitted Closing after delayed FLUSH_ACK; R2 paused counted output emitted Closing; R3 interleaved sessions exceeded live+reserved limit; R4 no lifecycle append after the normal has_work/pump scheduler. Initial test-only compile/setup errors (OpenArgs namespace, nonexistent iterator, wrong pool) were corrected before these genuine red reproductions. No production behavioral fix preceded this red run.
- [2026-10-22] Read full current/sol-daemon-review.md and accepted current/sol-daemon-store-review.md. Assigned G2-R1/R2 P1 and G2-R3/R4 P2 corrections only. No engine/store source, live actions, commits or subagents. Use target-sol-daemon and the selected Sol Ultrafast model.
- [2026-10-22] G2 final offline verification: cargo test -p rmds = 71 pass, 0 fail/ignore; cargo build -p rmds passes; cargo clippy -p rmds --all-targets --no-deps passes with dead-code warnings. All cargo commands used target-sol-daemon. Owned Rust files formatted with rustfmt --edition 2024; git diff --check -- crates/mds Cargo.lock current/sol-daemon.md passed. No workspace or live tests ran.
- [2026-10-22] Actual compound-operation tests now cover fresh bootstrap, same-identity checkpoint-only restart, missing/foreign/corrupt fail-closed startup, failed fencing with zero authority writes, barrier-before-acquisition, separate partial shard/session acquisition, delayed durable OPEN/CLOSE, stale owner completion, failed append output discard, identical unknown append reconciliation, queued-work wakeup without another client packet, checkpoint-generation pressure, paused-input nonspinning and held nonempty restore. The original daemon tests still use a mocked asynchronous completion transport and are not native durability evidence.
- [2026-10-22] Audit fixes: has_work honors paused normal execution and keyed CLOSE drain readiness. Fenced CLOSE cancels pre-engine normal requests like normal CLOSE. Blocklisting during durable OPEN completion sends rejection and retains cleanup ownership, without replacing an independently valid address. Restored Closing peers receive verified fencing before cleanup. Missing range-only replay owners bind from guarded rank incarnation history; engine validation remains authoritative. Lifecycle state stays in engine, not a duplicate ever-growing completed-job map. Restored cleanups enter the reserved control queue in bounded batches.
- [2026-10-22] New finite lifecycle preflight permits at most 4096 aggregate candidate objects and 64 layouts per attempt, before I/O. Four jobs run at once. Job entries remain deduplicated until completion commits. Oversized/transient failures retain durable intent and use capped retry delays. No work runs before confirmed service/reconnect readiness.
- [2026-10-22] Production check passed during migration. First full test run had 60 pass / 1 fail: FSMap next_filesystem_id was incorrectly decoded as i32. Corrected it to u64 and verified the real capture. New test_backend initially failed compilation on a partial move; stored has_write before consuming ops. No dependency implementation workarounds were added.
- [2026-10-22] G2 assignment: real root-shard persistence and clientless restart. Use finite Objecter actor timeout. Gate all mutation on persisted predecessor fencing and separate authority acquisition. Keep nonempty restored reconnect gather held. QA owns native/live validation.
- [2026-10-22 18:28] N1 red run: blocklisted_reopen_after_eviction_is_rejected_without_another_map_event failed because the evicted session reopened. N2/N3 red run: session::tests had 6 pass / 3 fail; queued CLOSE lost its owner, terminal failure disappeared, and duplicate OPEN carried seq 1 instead of 3. These are actual pre-fix regression results against the reviewed paths.
- [2026-10-22 18:28] N1 now checks the currently applied Objecter blocklist at dispatch, before OPEN/reconnect installation or rebinding. OPEN/reconnect rejection uses a non-admitted Session funnel and the standard blocklisted flag/reason. Other messages from the blocked address are dropped. A matching existing runtime session enters fencing; a different valid address's session is not rebound or evicted. Held OPEN input uses the same fresh dispatch check.
- [2026-10-22 18:28] N2 transfers a retired runtime session into a bounded terminal-funnel owner when output remains queued or failed. CLOSE stays behind preceding output. That client id cannot OPEN while its CLOSE funnel remains pending. Removal of runtime state never removes terminal output or clears the rank-wide sticky failure flag. Terminal enqueue/transmission/capacity failure closes all admission and makes the daemon fail-stop.
- [2026-10-22 18:28] N3 recognizes OPEN at the common Session::send_raw boundary and stores it as typed pending output. Transmission assigns seq from the accepted-transmission push counter after preceding queued pushes. Later queued pushes are excluded. RENEWCAPS stays encoded with its original client echo sequence.
- [2026-10-22 18:28] Final CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo test -p rmds: 59 passed, 0 failed, 0 ignored. Eight new regressions cover eviction/reopen without a new map event, held OPEN, valid-address rebinding protection, missing blocked mutation via later lookup, paused/unpaused CLOSE backlog, immediate/queued terminal send failure, daemon failure after removal, partial-drain OPEN seq, rejection ownership and terminal capacity overflow. Build and cargo clippy -p rmds --all-targets --no-deps pass; only Config.cluster and production-unused push_seq/iter_mut warnings remain.
- [2026-10-22] Read the complete daemon re-review and related G3 session/output checklist. Limit this correction package to owned daemon/session/server files and this ledger.
- [2026-10-22 18:00] Read daemon review and accepted final persistence contract. New assignment adds G1 fencing, map-version queries, barriers, membership/beacon gating, and bounded ordinary admission. No live operations, subagents, commits, or shared-manifest edits.
- [2026-10-22 00:00] Read handover, build brief, repository guide, and review workplan.
- [2026-10-22 17:35] Initial pre-fix session regression run could not compile because concurrent rados.rs work temporarily compared iterator outputs with Vec fixtures. No successful before-fix reproduction is claimed. Later session regressions passed.
- [2026-10-22 17:35] cargo test -p rmds: 35 passed, 0 failed, 0 ignored. Includes five new server behavior tests and three new session behavior tests. Tests cover malformed xattrs, deferred wrong-parent injection, retransmission/completed retry, manual commit/flush, bounded parked requests, close cleanup, sessionless renewal, duplicate OPEN notification/seq/stale rules, and malformed op handling.
- [2026-10-22 17:35] cargo build -p rmds passed. cargo clippy -p rmds --all-targets passed with warnings (generated denc identity operations; engine/monc warnings; existing daemon dead code and clone-on-copy). Removed the two new style warnings. git diff --check passed. Formatted only owned server.rs/session.rs/daemon.rs.
- [2026-10-22 18:06] Review correction before-fix run was blocked by the concurrent engine transition (missing durable/step modules, SessionKey replay mismatch). Did not edit engine. Once the owner supplied those modules, all review regressions passed: dirty valid-xattr payloads with GRANT/unknown cap opcodes cannot mutate state; truncated raw caps/releases fail-stop while malformed metrics remain tolerant; two deterministic expired sessions stop at the first invalid timer reply.
- [2026-10-22 18:06] New fencing.rs uses MonClient::get_version with bounded timeout, ObjecterStatus.latest_map_version, and wait_for_map_timeout. Every successful eviction observes the exact blocklist entry in a fresh applied map. Negative commands, failed waits, missing entries, and stale events never remove rights. CheckLatestMap failures retry without feeding cached epochs or blocking the event loop. External client blocklists use the same path; self blocklists exit.
- [2026-10-22 18:06] Bounds: 1024 held client messages / 16 MiB; 16 MiB aggregate paused output / 1024 messages per session; 1024 sessions/fences; 8 fencing workers; one latest-map query; 32 held inputs/output messages per turn. Reserved bounded channels carry eviction requests and control completions. Paused output preserves per-session order and increments push_seq only on accepted transmission. Session removal releases its paused-output budget. Failed enqueue/transmission stops service.
- [2026-10-22 18:06] Incoming cap/release barriers raise the monotonic Objecter/server barrier. All outgoing caps, FLUSH_ACK and FLUSHSNAP_ACK carry it. REQUEST_FLUSH_MDLOG captures the submitted memory-journal tail and retains that target until the reported committed prefix reaches it; there is no synthetic wire acknowledgment or persistence claim.
- [2026-10-22 18:06] cargo test -p rmds: 51 passed, 0 failed, 0 ignored. Includes real daemon dispatch, failed/stale fencing completions, successful cleanup after health recovery, external/self blocklists, queue overflow, paused timers/output, stale session generations, barrier propagation, and engine failure. Offline daemon tests bind an ephemeral local messenger and configure no monitor addresses; no Ceph cluster is contacted.
- [2026-10-22 18:06] cargo build -p rmds passed. cargo clippy -p rmds --all-targets --no-deps passed; only existing dead-code warnings remain (Config.cluster, SessionState::Closing, Session.client, production-unused push_seq/iter_mut). Formatted owned daemon/session/server/fencing files.

## Combined D6-R1 and strict-CI candidate: joint review handoff

The coordinator added only the six approved warning sites after receiving R1.
This is not a broad cleanup. Current strict Clippy first failed at exactly:

| Current pre-cleanup site | Minimal correction and use proof |
| --- | --- |
| `Config.cluster`, `config.rs:100` | No reader exists, including tests. Remove the internal stored field and its initializer. Keep the parser's local `cluster`, CLI option and all uses. Adding cfg(test) would leave a genuinely unused test field. |
| `rados::spawn_purge`, `rados.rs:348` | Workspace Rust search found only its declaration. Remove this uncalled fire-and-forget compatibility wrapper. |
| `rados::spawn_truncate`, `rados.rs:357` | Same no-caller proof. Remove only this wrapper, not checked truncate execution. |
| `rados::spawn_recover`, `rados.rs:373` | Same no-caller proof. The old wrapper only logged the result; durable completion already uses the checked executor. Remove only the obsolete wrapper. |
| `Session::push_seq`, `session.rs:305` | Getter calls occur only in cfg(test) modules. Add cfg(test). Keep the production field, increments, OPEN sequencing and CLOSE validation unchanged. |
| `SessionMap::iter_mut`, `session.rs:593` | Its sole caller is `server.rs`'s cfg(test) timer regression. Add cfg(test). Preserve that test and production session-map behavior. |

The actual red is `strict-before.log`: exit 101, six production errors and four
test-target errors. All 132 source/contract paths stayed unchanged across the
red/use-inspection interval. `uses-*.txt` retains the searches.

### Behavior and test preservation

`Config` is internal daemon settings, not a wire/persistent format. Removing
unread storage does not remove --cluster support. The local value still starts
as `ceph`, accepts the same option forms, selects `/etc/ceph/{cluster}.conf`
after CLI/CEPH_CONF precedence, and expands `$cluster` in configured/default
keyrings. Explicit conf/keyring options, section precedence, other metavariables
and startup logging are unchanged. No pointless read or new logging was added.

The accepted lifecycle route is unchanged: `Daemon::drive_jobs` reads committed
jobs, awaits `rados::execute_lifecycle`, and sends owner/operation/job-tagged
results. `on_control_event` validates the attempt. Success enters
`Server::lifecycle_completed` and the admitted EngineInput::LifecycleCompleted;
failure retains the durable job for retry. `purge_inode`, `truncate_data`,
`recover_file_size`, layout/budget/error checks and all data tests remain intact.
The removed wrappers had no callers and supplied no completion transaction.

`behavior-preservation-proof.txt` compares all bytes to the archived completed-R1
source. Config differs only by two storage lines. Rados differs only by the
three-wrapper block and its comment. Session differs only by two cfg(test)
attributes. Thus the R1 guard, all session tests and all checked data/configuration
code are unchanged. `lint-only.patch` contains that exact three-file delta.
No tests were added, removed or invented to make unused code appear used.
No dead-code allowance, lint configuration change or weakened strict gate exists.
Wire behavior, durable scheduling/completion and the nonempty D5 hold are intact.

### Combined final checks

Every Cargo command used `CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon`.

| Check | Actual result |
| --- | --- |
| `cargo clippy -p rmds --all-targets --no-deps --locked -- -D warnings` | PASS, no warnings |
| Existing configuration / data primitive tests | 4 / 14 passed |
| D6 / session-output tests | 26 / 15 passed |
| Preserved G2 / accepted G2 fault tests | 11 / 4 passed |
| Total G3 subset | 70 passed |
| Full daemon tests | 160 passed, 0 failed/ignored; unchanged count |
| Production build | PASS, no warnings |
| All 20 daemon Rust format checks / scoped whitespace | PASS |

Filters overlap. No combined-workspace, strict-dependency, native or mounted
qualification follows. E1 remains isolated; nonempty finalization remains held.

### Fresh combined provenance

Evidence directory: `/tmp/opencode/sol-daemon-r1-strict-ci-20261025.aKufTD/`.
The pre-cleanup manifest exactly matches the completed R1 candidate. Final
before/after manifests prove **132 unchanged paths/modes/hashes** across strict
Clippy, all checks and the build. The source scope has 129 Cargo inputs.

- Combined input fingerprint: `6d46a8ffb9132e567adf1fbb3f1f8ad9ae7b3f766f95f297bd200757376fe4d8`.
- Canonical manifest: `release-source-path-mode-sha256.txt`.
- Manifest SHA256: `7b4b184505c6172fa522f5355f9ffcea2580fe2de1dbbe4c52470d4a4525c2e4`.
- Combined source archive: `combined-source.tar.gz`.
- Archive SHA256: `b3a54565a845092aad91fb6cf76b8b1edcd4e6a5659f00dd7f45f067254ca14b`.
- Exact commands/status/timing: `final-commands.json`; logs: `final-*.log`.
- Delta/use proof: `cleanup-source-delta.json`, `lint-only.patch`, `uses-*.txt` and `behavior-preservation-proof.txt`.
- Executables: `combined-daemon-tests` and `combined-ceph-rmds`.

All original R1 artifacts still match their original inventory, including real
red/green executables and logs. See retained-r1-before/after.sha256 and
retained-r1-stability.txt. No monitor/store accepted edit, other outside source,
Cargo.lock/manifest, live state, reviewed binary/tooling or isolated release
snapshot changed in this cleanup. No Git write, commit or subagent ran.

This combined manifest identifies the new cohesive author candidate only.
Independent review must cover both R1 and these six sites before any later
explicitly assigned release work. Hold all assigned files after this handoff.

## D6-R1-only correction: historical pre-lint handoff

### Exact guard and transport boundary

`crates/mds/src/session.rs` adds `PendingOutput::Reclaim { raw, owner }`.
The common `Session::send_raw` boundary wraps every CLIENT_RECLAIM_REPLY.
This includes existing-target success, absent-target success and all errors.
No response-specific call site can enqueue an unguarded reclaim reply.

The owner captures client ID, runtime generation, durable SessionKey/open_id,
connection ID, active address, authenticated name and authorization-context bytes.
`Session::durable_open` is an output tag, not admission authority. Tagged normal
and recovery Open callbacks update it. The normal callback observes a newer
durable decision even if blocklist/feature policy refuses its attachment.
A same-connection OPEN still changes open_id and invalidates the old output.
Legacy test-only funnels can have no durable tag; production attachment sets it.

`Session::transmit` checks exact owner equality and current Open/Stale state
immediately before ClientLink::send_raw. It also requires the captured address
to match both the funnel address and current link address. This task owns the
funnel exclusively. No await occurs between this check and transport handoff.
Opening, Recovering, Closing, Fencing and Closed cannot receive queued reclaim.
The original runtime completion checks, including features/blocklist, remain.

`Transmission::{Sent,Discarded,Failed}` keeps discard distinct from acceptance.
A queued discard removes one FIFO entry, releases its exact charged bytes and
uses one drain-budget turn. It sends nothing and changes no push sequence.
The drain continues with unrelated output. An immediate discard returns false,
without latching failure, because no queue/transport accepted it. A real capacity
or transport failure keeps the existing sticky rank failure. Failure is checked
before ownership, so an obsolete entry cannot hide or clear an earlier failure.

No queue reset, retag of old entries or new reclaim-attempt registry exists.
No engine/store API, wire schema, D6 cleanup/timer algorithm or G2 fault source
changed. Completed target receipts/Closed rows stay unchanged by suppression.
An obsolete entry cannot be reassigned to a new waiter or consume its response.

### Owned delta

| Path | R1-only change |
| --- | --- |
| `crates/mds/src/session.rs` | Guarded reclaim output, current durable tag, explicit transmission outcomes and four actual queue/transport tests. |
| `crates/mds/src/server/runtime.rs` | Record normal durable OPEN decision on an existing funnel and on first successful publication. No cleanup/readiness change. |
| `crates/mds/src/server/runtime/recovery.rs` | Record the durable decision before obsolete-attachment checks and on a newly installed recovery funnel. No gather/finalization change. |
| `crates/mds/src/server/runtime/tests/d6.rs` | Nine production-dispatch R1 regressions and narrow test helpers. |
| `crates/mds/README.md` | Describe queued ownership, explicit discard accounting and historical pre-R1 counts. |
| `current/sol-daemon.md` | This checkpoint and correction evidence. |

### Red/green and preserved coverage

The original six R1 tests ran before any owned production correction. Exact
reproduction fails at the independent assertion: "old reclaim completion must
not migrate to a replacement attachment after enqueue". The matrix gave
**1 passed / 5 failed**. The positive FIFO/byte/push test already passed.
The other failures prove same-connection new OPEN, queued Closing, immediate
absent response and valid-new-attempt confusion. These unchanged tests pass
after the fix. The loops also cover Fencing and all immediate error categories.

The exact sequence retains 40 older renewal replies, completes target Closing,
actual cleanup and tagged Closed, drains 32 replies, then uses production raw
OPEN dispatch and tagged Open completion to change requester connection/context/
open_id. Its remaining eight renewals still reach the replacement in order.
Only the obsolete reclaim reply disappears. The old target receipt stays fixed.

The new-attempt regression completes a second target at epoch 13 while the old
epoch-9 response waits. A nine-entry turn drains eight renewals and discards only
the old response. A one-entry next turn delivers exactly the new response.
Additional production cases cover normal Closing and Fencing, a retired CLOSE
funnel, valid Stale, unchanged ordered counted pushes, and a committed new OPEN
whose attachment is refused after blocklisting. The session tests isolate each
owner field, exercise actual bounded queue/transport handoff, release/reuse one
of 1024 count slots, check exact 16 MiB charging and retain sticky failure for
immediate/queued transport refusal. They do not only compare helper tokens.

Final results (overlapping filters):

| Check | Result |
| --- | --- |
| D6 (`g3_d6_`) | 26 passed: prior 17 plus 9 R1 cases |
| Session output (`session::tests`) | 15 passed: prior 11 plus 4 reclaim cases |
| Preserved G2 (`g2_r`) | 11 passed |
| Accepted G2 fault families (`g2_fault_`) | 4 passed, unchanged source |
| Total G3 (`g3_`) | 70 passed |
| Full daemon | 160 passed; 0 failed/ignored |
| Production build / daemon-only default clippy | PASS; existing dead code only |
| All 20 daemon Rust sources / scoped whitespace | PASS |

### Provenance and evidence

Evidence directory: `/tmp/opencode/sol-daemon-d6-r1-20261025.D7tKz0/`.
`final-commands.json` gives exact commands, status and timing. Final logs use
`final-*.log`. `release-source-path-mode-sha256.txt` is the new 132-path source
manifest. `post-fix-source.tar.gz` preserves those exact bytes/modes. Retained
post-fix test/daemon executables are `post-fix-daemon-tests` and
`post-fix-ceph-rmds`. This source manifest supersedes the pre-fix manifest only
for identification; it is not release authorization or independent acceptance.

The initial manifest matched the reviewed 129-input fingerprint
`3fc3c84314b2727b5ffe6b642ed06b6e9add5000636cbd15c1472c5222e0c115`.
External `crates/monc/src/monclient.rs` type-alias work appeared after the first
red build. The repeated red then overlapped an external test-only E2 caller edit
in `crates/store/src/stable.rs`. Both before/after pairs and red executables/logs
are retained. Daemon production did not change during either red, but no stable
whole-tree red claim is made. This owner neither made nor reverted those edits.
`source-delta.json` distinguishes them from the five owned source/doc paths.

The complete final verification window has **132 unchanged paths/modes/hashes**.
Its 129-input fingerprint is
`a3665910ca276c03b3c75861be406502c22a92c1f13061210c6e2d8433eba69a`.
Its source-manifest SHA256 is
`ada321fbb0c9b54e38ec743ef6019154a608499f6739aa7cc6f6fefa85f9386a`.
The initial old review artifacts still match retained-evidence-before/after.
All earlier author/reviewer evidence and the isolated pre-fix snapshot remain
untouched. No live action, Git write, commit, snapshot edit or subagent ran.

Nonempty finalization stays held for E1/D5. Prior D1-D4/clientless E2/G2 fault
acceptance remains unchanged. Coordinator must obtain independent R1 re-review
before any later explicitly assigned post-fix release-candidate work.

## Pre-R1 D6 and audited G2 faults: historical handoff

### Source delta and private hooks

- `crates/mds/src/server/runtime/d6.rs` is the new bounded policy module. It owns UUID checks, RESET waiters, target fence joins, requester revalidation and ACTIVE timers. It reuses `RecoveryFence` and the existing one-record Closing/cleanup/Closed chain. It adds no engine/store/wire type.
- `server/runtime.rs` initializes D6 state, rejects duplicate live UUIDs at checked load, reserves pending OPEN UUIDs, freezes target attachment, passes original receive times, notifies reclaim completion only after tagged Closed, and exposes `tick_serving(sessions, active)`. Direct old handler/tick wrappers are test-only. The nonempty exclusions in structural finish and marker readiness are unchanged.
- `server/runtime/recovery.rs` includes D6 targets in the existing full-union fence consumer and initializes the runtime activity clock at validated final reconnect. The 45/90-second gather algorithm and no-extra-Ganesha-hold rule are unchanged.
- `server.rs`/`daemon.rs` carry receive times through production dispatch. Current attachment also requires the messenger connection ID. `note_held_activity` validates eligible held traffic and changes only its monotonic clock. The real receive regression places a renewal behind 65 held messages, beyond one 32-message turn. `tick_serving` preserves STOPPING engine/checkpoint progress while session expiry runs only in healthy confirmed ACTIVE.
- `session.rs` adds `renew_at` and refreshes metadata/features/activity only after durable OPEN publication. Every output still uses the existing session funnel. Ordinary STALE retains engine caps/locks; only valid renewal resumes Open. No compatibility stale-cap API is called.
- `persistence.rs` and `main.rs` add test-module declarations only. `test_runtime.rs` supplies a test-only production worker/Peer harness. `test_backend/data.rs` supplies deterministic OSD transport below the actual Objecter; `test_backend.rs` only declares it. Daemon tests override only the offline monitor global-ID observation. They never override checked recovery readiness.
- `server/runtime/tests/d6.rs`, `daemon/tests/faults.rs` and `persistence/fault_tests.rs` hold the new regressions. README describes the exact protocol, timer policy and test boundaries. Cargo.lock/manifests and proto have no new D6 edits. Engine/store/platform/tooling sources remain untouched.

### D6 protocol and bounds

Only current durable Open/Stale requesters are eligible. FINISH alone is silent;
mixed FINISH, non-RESET or empty RESET UUID returns EINVAL. Wrong target auth
name returns EPERM without cleanup. An absent live UUID returns 0/epoch0/empty
addresses, which libcephfs converts to ENOENT. Closed tombstones are not targets.

Existing targets freeze admission and wait for pending identity writes. The
saved complete address union then uses latest-map verified fencing/barriers.
Existing Closing/fencing and duplicate reclaims join one cleanup. Both verified
fencing and durable Closed are required before the ordered reply. Completion
rechecks requester key, generation, connection, durable OPEN identity, lifecycle,
credentials, address, current feature policy and blocklist. Disappeared/replaced
requesters inherit nothing. The aggregate bound is 128 waiters and thus at most
128 targets. Repeated input from an unchanged requester does not add a waiter.
Overflow and output failure remain sticky fail-stop boundaries.

UUID admission includes live Open/Closing rows and pending OPEN reservations.
Same-key metadata updates are allowed. Both old and requested UUIDs stay reserved
until the update decision is durable. Checked restore rejects duplicate live
UUID ownership. Closed completion releases the live reservation.

Timers use monotonic valid-current activity, not persisted stamps. Before S they
retain the session. At S they check autoclose first, then ordinary STALE or the
timeout override. S/A are clamped to one second. Timeout text is strict trimmed
ASCII decimal u32. Zero/malformed still receive timeout-session treatment and
expire at max(S,A); positive P expires at max(S,min(A,P)). Malformed text remains
unchanged and produces a diagnostic. Tests cover S60/A300 absent/0/malformed/
P30/P120/P600, whitespace/sign/overflow, minimum maps, renewal before expiry and
after STALE, held/old/unknown traffic, retained caps/locks, failed fences, reordered
completion, beacon loss, STOPPING and sticky output failure.

### G2 fault-family evidence

| Test | Actual boundaries and oracle |
| --- | --- |
| `g2_fault_closing_restart_cuts` | Real worker durable Closing before cleanup; durable range/job decision without terminal receipt; durable terminal receipt before Closed callback. New-owner `worker -> open -> Loaded -> from_loaded -> pump` fences persisted Closing peers, preserves key/counter/history/bounds/job ID, reaches one exact receipt/Closed and reloads checkpoint-only Closed. The terminal-receipt cut appends no second cleanup. |
| `g2_fault_recovery_join_rejects_mismatches` | Six cases over real worker-loaded state: orphan shard history, Open with receipt, Closed without receipt, wrong receipt sequence, wrong incarnation, and duplicate live incarnations. Checked join fails before service/output/work. Invalid join vectors are deliberate negative fixture edits, not fabricated positive callbacks. |
| `g2_fault_lifecycle_partial_io_retry` | Normal dispatched operations create actual durable Purge/Truncate/RecoverSize intents. The real Objecter/executor succeeds on one REMOVE/TRIMTRUNC/STAT then receives EIO. Metadata stays unchanged; partial data survives worker/runtime recreation. New-owner retry uses the same job/bound/layout/namespace. Purge retry handles prior REMOVE's ENOENT; truncate checks bytes/local sizes/sequence; probe error yields no zero-size success and retry checks independent size/mtime. An unrelated namespace sentinel remains intact. |
| `g2_fault_lifecycle_completion_cuts_and_late_events` | Actual executor success enters `Daemon::on_control_event`. Old owner/attempt and duplicate current callbacks are tested. Cuts occur before completion append, at failed append, after durable append before callback, and after callback. Reload retains bounds/jobs for non-durable cuts and applies metadata/retirement once for durable cuts. Retired jobs do not reschedule after normal E2 readiness. |

The four baseline feature reds and four follow-up D6 audit reds are real before/
after evidence. G2 fixture/oracle setup failures are recorded separately above;
the audited families established no production lifecycle/persistence defect.
Native faults, mounted-client G3 and host/OSD power loss remain unclaimed.

### Remaining coupling

E1 contract acceptance does not mean E1 source or upgrade ran. D5 still needs
the reviewed E1 dependency cut, all-surviving-key structural join, protected
retry/control/job drain, marker execution and ordered output before ACTIVE.
The sticky nonempty hold remains, even after all original keys reach Closed.
This D6 package does not implement a reclaim hold after reconnect or add a
native fault seam. Preserve fs44/meta88/data89, both acknowledged namespaces,
the reviewed binary/tooling and all prior evidence. Independent review and an
explicit sole-QA live assignment remain mandatory.

## Accepted D1-D4/clientless E2 handoff (historical before D6)

### Files

- `crates/mds/src/config.rs`: positive finite `mds_reconnect_timeout`, default 45 s, with representable 2R.
- `crates/mds/src/daemon.rs`: separate permissions, confirmed phases, timestamped raw holding queues, deadline wakeup, recovery fences and offline fixtures.
- `crates/mds/src/server.rs`: recovery dispatch, required CephFS bits and recovery eviction routing.
- `crates/mds/src/server/runtime.rs`: attachment purpose, scheduler integration, grant/finalization barrier and durable Closed join.
- New `crates/mds/src/server/runtime/recovery.rs`: bounded collection/attachment/reset/deadline/realm/fence state machine.
- `crates/mds/src/session.rs`: connection identity, silent Recovering attachment and ordered terminal routing.
- `crates/mds/src/server/runtime/tests.rs`: production fixture restore and original two dispatch regressions.
- New `crates/mds/src/server/runtime/tests/recovery.rs`: reconstruction, identity, generation, lock/realm/budget, fence and barrier regressions.
- `crates/mds/src/persistence.rs`: E2 fixture/scheduler migration and actual-store-worker restart/attachment test; production persistence code is unchanged.
- `crates/mds/README.md`: exact collection-only boundary, APIs, budgets and next-package hook.
- `crates/proto/src/reconnect.rs`: only the ceded mixed v5 caps/realms comment. No decoder change.
- `current/sol-daemon.md`: this task ledger. No Cargo.lock/manifest changes in G3.

### APIs and limits

- `ClientLink::connection_id`, `SessionState::Recovering`, `SessionMap::{attach_recovery,recovery_terminal}`. Recovery starts push sequence zero. Post-final transport replacement preserves its funnel sequence and reconstructed rights. A different attachment Token cannot start a second reconstruction stream; it requires fenced close.
- `Server::{set_runtime_permissions,allow_recovery_collection,confirm_reconnect,reconnect_deadline,expire_reconnect,recovery_accepted,recovery_input,recovery_connection_gone,recovery_pending,recovery_status}`. Raw dispatch preserves original daemon receive time. Connection IDs plus local generations isolate incomplete reconstruction. Old committed attachment results retain the address union without publication.
- `Server::recovery_structural_done` exposes the first E2 boundary. `RecoveryDrain::{Reconstruction,Controls,GrantsReleased}` separates structural finish, admitted marker execution and drain readiness. `try_finish_recovery`/`recovered` require both boundaries and drained required work/output. Serving also requires confirmed ACTIVE and authority. The empty marker returns its unused permit without a WAL callback. A busy adapter retains the obligation without requesting a checkpoint.
- `RecoveryFence { owner, key, attachment, addresses }` and `Server::{recovery_fences,matches_recovery_fence,finish_recovery_eviction}`. The optional attachment is `(connection_id, runtime_generation)`. Daemon proof requires every frozen address in the checked current map and a nonzero epoch. Missing keys need no fake Session. Closing -> committed cleanup receipt -> Closed remains the only removal chain.
- Runtime budgets: 32 MiB charged aggregate; incoming message at most 1 MiB including envelope charge; 1024 held inputs split into 128 reconnect chunks and 896 early inputs; 16384 applied caps; 65536 applied locks; 4096 reported realms; 4096 child realms per SPLIT; 64 peer addresses/nonces per key. Applied charges remain after raw release. A replacement charges its new size. Corrections consume the remaining byte budget as they are retained. The existing 1024 session-slot reservation limit remains shared with G2.
- Existing raw ordinary/reconnect holding queues each retain their own 1024-message / 16 MiB bound. Paused output retains its separate 16 MiB rank / 1024-message per-funnel limit and 1024 terminal owners. These limits do not bound active messenger backlog or process memory end to end.
- Reconnect: `deadline = min(t0 + 2R, max(t0 + R, L + R/2))`, with t0 from confirmed RECONNECT and L from valid missing-member receive activity. Default R=45 s and hard bound=90 s; configured hard bound is 2R. Reset/replacement never restarts t0. Expiry includes timeout sessions and adds no extra reclaim hold.

### Genuine before/after evidence

| Boundary | Genuine pre-fix result | Final coverage/result |
| --- | --- | --- |
| C1 late receive | Authenticated loopback v3 reconnect remained in reconnect_held after all-Closed ACTIVE load. | Three C1 tests pass: v3/v5 known/unknown immediate/pre-load terminal FIFO; separate nonempty map race; current active attachment waits for verified fence and durable Closed, including WaitSeq. |
| C2 activity | Two tests failed: first final at 44 s left 45 s instead of 66.5 s; later first final failed to reach the 90 s hard bound. | Both C2 tests pass. They cover v3, empty v5 final, remaining valid input at 65 s, completed-member duplicates and fixed hard cap. |
| C3 fail-stop | Failed renewal left fatal_error unset while another key's durable Closing cleanup was runnable. | One C3 test passes both immediate send and paused-funnel capacity failures. Neither case submits cleanup, transfers a worker command, changes the receipt or sends later output. |
| E2 empty drain | Structural finish incorrectly reported readiness. | Empty marker test passes through a reserved fresh step with no WAL callback and returns its unused permit. |
| E2 delayed permit | Returned permit triggered an unrelated checkpoint instead of completing the marker. | Delayed-permit test passes in one marker turn with no sequence change or storage command. |

The review's 8-pass/26-fail G3 and 7-pass/4-fail G2 runs stopped in fixture setup
after the concurrent E2 postcondition change. They are historical coupling
evidence, not C1/C2/C3 reds. The actual-worker attachment test now also requires
its setup mkdir reply and durable completion history before restart.

### Completed clientless E2 join and required next package

`from_loaded` initializes Reconstruction for every checked load. Clientless
`try_finish_recovery` takes structural `finish_reconnect(&[])`, then `pump`
executes `RecoveryControlsDrained` with a fresh reserved permit. No fixture sets
readiness or calls a legacy bypass. `drive_phase` permits REJOIN at structural
completion and uses drain readiness for CLIENTREPLAY/ACTIVE. CREATING/STARTING
and already-ACTIVE loads take both boundaries too. `ready_jobs` permits eligible
clientless retained work after structural finish. Required Truncate/RecoverSize
completion commits precede the marker. Independent background Purge adds no
extra marker hold.

D5 must replace the nonempty exclusions in `try_finish_recovery` and
`recovery_marker_ready` only after its full drain is reviewed. Pass all surviving
Open keys from `Recovery::members`, including completed members, not `gather`.
Each excluded key must already have reached durable Closed. Account for the
protected `Recovery::early` cut outside `Runtime::control`, durable attachments,
missing-key cleanup, dirty control commits/deduplication, committed recovery
retries and their eligible required jobs. A reset or queue removal is not drain
completion. Keep Busy-control dependency progress and one-record admission.
Ordinary held requests must not block the control cut. Execute the release
marker only after that cut, then drain ordered recovery output before confirmed
ACTIVE and ordinary admission. `consume_step` still rejects nonempty GRANT
instead of retaining a payload computed before dirty commits. Even all-Closed
completion of an initially nonempty load cannot bypass this hold.

D6 UUID/RESET/FINISH and active timeout timers remain unimplemented here. They
must use the retained session identity/metadata, current attachment generations,
durable session updates and verified cleanup chain. Ordinary E1 and upgrade
behavior remain separate. No additional audited G2 fault tests, mounted G3 or
full G2 fault qualification is claimed. Preserve fs44/meta88/data89, reviewed
tooling/binary and all evidence. Independent review precedes any sole-QA live
assignment.

## Historical G2 integration surfaces

### N1-N3 correction delivery semantics

- Touched only crates/mds/src/session.rs, server.rs, daemon.rs and this ledger for this correction package. Added Server::is_blocklisted and SessionMap::reject_blocklisted. No store/engine, rados, manifest, live-cluster, commit or subagent work.
- Admission checks the applied map for the incoming link's exact address/nonce. A rejected connection never enters the runtime session map or engine. Rejection funnels use generation 0 (non-admitted); they do not consume the monotonically allocated runtime session generation. A permitted new address can OPEN through the normal path. Blocked duplicate OPEN cannot replace that valid session's link, address or generation.
- A normal CLOSE first stops runtime admission. SessionMap then owns the old funnel in a separate terminal set. Old non-counted/countable messages and the final CLOSE drain in FIFO order through the same Session transmit function. The terminal set has at most 1024 funnels and shares the rank-wide 16 MiB output byte budget; each funnel still has the 1024-message limit. The existing bounded drain budget covers terminal and active output together, with terminal progress first.
- While a CLOSE terminal is pending, repeated OPEN for that client id returns Done without replacing the link, creating engine state or allocating a new generation. After the terminal's complete queue is accepted, its owner is retired and a later OPEN can allocate the next generation. Rejection terminals do not reserve an admitted client id or block an independently permitted clean OPEN.
- Here, accepted transmission means ClientLink::send_raw returned true. The messenger then owns delivery/retry of those bytes. This is not proof of client receipt or durable session close. The active messenger capacity-feedback limitation below remains unchanged.
- Runtime remove is an explicit verified-fencing discard operation. It cannot remove a terminal CLOSE owner. Any prior enqueue/transmission failure survives all runtime removals through a rank-owned sticky atomic flag. Terminal failures retain their owner; terminal-capacity exhaustion explicitly discards the newly retired funnel only after setting fail-stop. No later input, output drain or control cleanup may resume service after that flag. Final process teardown releases held byte budgets.
- All OPEN output, including already encoded OPEN passed to send_raw, becomes typed pending output. Its seq is filled at its own transport position. push_seq increments only after accepted CAPS/SNAP/LEASE/QUOTA transmissions. RENEWCAPS echo seq is not rewritten.

- New fencing module: Membership, Eviction, ClusterFence, FenceBackend, fence, latest_epoch and CONTROL_TIMEOUT. These are daemon-local types, not alternative store ownership tokens.
- Server::set_evict_channel now takes bounded Sender<Eviction>. Added request_eviction, finish_eviction, raise_epoch_barrier, epoch_barrier, poll_commits, pause_data_work, resume_data_work and log_status. kill_session is private to the server; the daemon can call only the verified completion boundary. fatal_error also observes Engine::is_failed.
- Session adds a monotonic runtime generation and a paused ordered output queue. SessionState adds Fencing. SessionMap adds pause_output, drain_output, output_failed and queued_output_bytes. Paused output uses one rank-wide byte budget. These runtime generations are not durable SessionKey incarnations.
- daemon.rs extracts dispatch_client_raw for the actual decode/admission boundary and offline tests. Its control envelopes carry operation/membership identities. Ordinary service requires matching current monitor identity, map up/rank/incarnation, no map lag marker, and a healthy beacon. Reconnect bypasses the ordinary held queue, but the prototype still denies reconnect until G3 installs reconstruction.

## Historical G1 handoff: required integration changes and limits

Items 1-5 below describe the pre-G2 handoff. The G2 package now implements them
for clientless recovery. Item 6 remains a platform limitation. Nonempty G3 gather
reconstruction remains explicitly unimplemented.

1. Replace Engine<MemJournal> and direct request/cap/release/tick/close calls with the accepted EngineInput/EngineStep/RecordPermit protocol. Current compatibility journal_committed can run already-admitted parked work; production journal_event must only materialize output before another admitted step. Reserve bounded outbox capacity and maximum record bytes before each transaction.
2. Install durable SessionKey from the rank session repository. Persist Open/Closing/Closed in contract order. Replace immediate close/kill cleanup with terminal SessionCleanupCommitted receipts before marking rank Closed or sending CLOSE. Current runtime generations do not satisfy this durable join.
3. Wire store append/failure events with OwnerToken plus operation id. Inspect and fence the union of persisted predecessor writer addresses before acquiring/loading shard and session authorities. Honor last_failure_osd_epoch and retained predecessor evidence; zero never proves fencing.
4. Effect::Lifecycle currently fails closed with an explicit unsupported-executor reason. The future executor must await accepted rados.rs Result primitives, retain failed intents, and return EngineInput::LifecycleCompleted under the current owner generation. Do not use log-only compatibility wrappers to retire jobs. Legacy work scheduling pauses with service health and has bounded held storage.
5. Replace immediate next_request recovery transitions with completion-driven load/reconnect/rejoin/clientreplay. Apply the independent G3 checklist. No restart, reconnect, durable retry, bootstrap, checkpoint, or mounted-client qualification is claimed here.
6. Connection::send exposes closed/accepted only. Daemon-held input, paused output and control queues are bounded, but active messenger backlog has no capacity/queued-byte feedback. Request a platform back-pressure/queue-accounting API before claiming an end-to-end outbound memory bound.

## G2 delivery boundary

- `DaemonJournal`: closed replay cursor, one reserved bounded record, one in-flight append, contiguous explicit commit/failure. Submission performs encoding only. No compatibility polling runs in production.
- `persistence`: read-only format/control/session inspection; persisted writer fencing and map barrier; separate shard/session acquisition; creating-only deterministic bootstrap; strict checkpoint/tail load; serialized session/WAL/checkpoint worker. Unknown outcomes retain identical arguments for up to eight retries, then fail-stop. Commands/completions carry OwnerToken and operation id.
- `Server<DaemonJournal>`: all request/cap/release/tick/close/lifecycle mutation paths use EngineInput/RecordPermit/EngineStep. JournalEvent materializes output before another admitted step. Durable row joins preserve SessionKey, retry state and cleanup receipts. New OPEN output waits for durable Open. Closed follows terminal cleanup commit. Rank Closing recovery fences client peer addresses before releasing rights.
- Checkpoints use coherent quiescent cuts and immutable write -> publish -> bounded trim. SIGUSR1 requests a checkpoint. Periodic checkpoints use record/time thresholds. Capacity pauses admission with an explicit increase-budget/restart diagnostic. No checkpoint GC exists.
- Native startup discovers filesystem id through FSMap, configures finite Objecter actor operation timeout, and fails closed on missing/foreign/corrupt recovery state. Membership requests wait for load and monitor-confirmed phase changes. Nonempty Open gather and reconnect packets remain bounded and held for G3.
- `crates/mds/README.md` documents normal run command, six private settings/defaults, signal behavior, capacity recovery, lifecycle bounds and G3 limits. No denc or platform API changes were needed.
- Remaining warnings: Config.cluster, retained log-only rados compatibility wrappers, production-unused Session.push_seq/iter_mut/remove. The wrappers have no production callers. Active messenger queue feedback and G3 client reconstruction remain outside this package. Native/live G2 qualification is pending QA.
