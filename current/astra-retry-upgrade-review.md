# astra-retry-upgrade-review
Task: New independent STORE UPGRADE CONTRACT review. Audit proposed design/recovery-contract.md sections 3.5–3.8 against stable store source and accepted persistence contracts. Return SCOPED ACCEPT format/upgrade contract or ranked concrete CHANGES REQUIRED before implementation.
Status: SCOPED ACCEPT sections 3.5–3.8, U1/U2 format/upgrade contract. Both findings are closed. The two-process fresh boundary matches existing acquisition rules. The checked legacy loader now specifies historical SessionKey binding before range-only replay and final clean proof through T. Source-only contract acceptance; no implementation or live acceptance.
Context: Sole writable file is this ledger/report. Parent orchestrates. No production or design edits, builds, tests, live actions, Git writes, or subagents. Store source is stable. Use committed a6ae05a for old daemon startup mechanics if needed. Another reviewer owns the detailed E1 proof in sections 3.2–3.4. Prior store acceptance and retained G2 evidence remain scoped historical evidence.
Next: Parent combines this scoped acceptance with the separate sections 3.2–3.4/R4 acceptance before assigning E1 implementation. Later implementation review and live execution require their own assignments. No format/upgrade finding remains open in this report.
Log:
- [2026-10-25] Completed narrow U1/U2 re-review: SCOPED ACCEPT sections 3.5–3.8. Rechecked bootstrap resume/publication, each acquisition's monotonic identity checks, full session-row loading, committed a6ae05a historical key selection and checked engine replay. Cases 12–13 specify the requested positive/negative boundaries. Reviewed no active R4 policy and ran no tests, builds, live actions, Git writes or subagents. Edited only this report.
- [2026-10-25] Started the requested narrow U1/U2 re-review. Read this report first, then current/sol-design.md and revised recovery-contract sections 3.5–3.8 plus acceptance cases 12–13. Only this report is writable. Source-only review; no implementation, tests, builds, live actions, Git writes or subagents.
- [2026-10-25] Created the review ledger before other edits. Review evidence will come from source inspection only.
- [2026-10-25] Read repository assignments, handovers, build conventions, requirements, accepted persistence/store contracts, the complete proposed recovery addendum, store README, prior state acceptance and parallel review scope. Read stable store/bootstrap/session/framing/record implementations. Read committed a6ae05a:crates/mds/src/persistence.rs for old startup behavior. Performed no build, test, live action or Git write.
- [2026-10-25] Traced the committed old daemon's historical range-owner binding in server/runtime.rs and its journal initialization. Checked engine restore/replay validation, allocator reservations, backend result mapping and denc framing. Completed version, guard, API, capacity and crash-cut analysis. Recorded two P2 contract findings below. Edited only this report. No tests, builds, live calls, Git writes or subagents.

## Current narrow re-review verdict — 2026-10-25

**SCOPED ACCEPT — sections 3.5–3.8, U1/U2 format/upgrade contract.** Both original findings are closed. The prior version, downgrade, per-object guard, candidate and unknown-outcome analysis below remains in force. Whole E1 still requires the other reviewer's sections 3.2–3.4/R4 acceptance.

### U1 closed: the seed-completing process stops before service load

Sections 3.5 and 3.6 U1 now specify an exact two-process boundary. The process that completes or permissibly resumes Preparing publishes and reconciles the frozen Legacy2 Ready seed. It returns `Err(UpgradeRequired)` before `Loaded`, even with the upgrade option. Its tagged startup failure leads to normal shutdown while unready. It creates no upgrade candidate or intent.

A later monitor-assigned process must have a higher incarnation and different GID than the actual predecessors. It supplies explicit opt-in, fences them and independently acquires format, WAL and sessions. Eligibility uses the pre-acquisition snapshots. Reconciliation of a CAS already issued by that attempt remains valid; a same-identity fresh retry is not a substitute for takeover.

This matches the source acquisition checks in `bootstrap.rs:456–480`, `stable.rs:361–397` and `sessions.rs:146–174`. It needs no invented token, self-blocklist or same-owner exception. The later WAL acquisition advances the current shard epoch. Candidate generation then advances the source checkpoint generation; these are separate counters. The immutable seed and its original owner/name stay unchanged.

The Preparing interruption path also remains coherent:

- Before Ready commits, a permitted creating replacement resumes the persisted original plan. `resume_bootstrap` verifies or creates its deterministic dependencies and requires independent current WAL/session guards (`bootstrap.rs:514–588`). It does not rewrite original objects under the replacement owner.
- The process that eventually publishes Ready is the seed-completing process for U1. It must stop at the same UpgradeRequired boundary. A further eligible assignment performs adoption.
- If Ready committed before the completing process stopped or received its result, the later process observes existing Ready/Legacy2 and follows the opt-in takeover path. An uncertain publication is reconciled before any dependent action (`bootstrap.rs:590–648`).

Case 12 explicitly checks both option settings, the no-Loaded/no-service boundary, later assigned takeover, byte preservation and rejection of A's prepared operations after B acquires each authority. These are test requirements, not executed evidence. The old fresh service-load case now occurs in B after adoption, as case 12 states.

### U2 closed: historical keys precede checked range-only replay

Section 3.6 U2 and section 3.7 step 2 now load the complete guarded rank history before legacy restore/replay. The history uses full SessionKeys and includes Closed rows. `SessionRepository::load_sessions` already returns those rows; its latest-client index check does not discard older keys (`sessions.rs:205–264`).

The specified selection matches `a6ae05a:crates/mds/src/server/runtime.rs:210–242`: preserve an existing historical binding; otherwise exclude keys with checkpoint/replayed receipts and Closed lifetimes ending before the record sequence. Select the earliest eligible cleanup sequence, with an absent sequence last. This does not select the latest client index or substitute the daemon owner.

For the required reused-client sequence `s1 < t1 < s2 < t2=T`, both Closed rows are initially available. At s1 the earliest eligible endpoint is t1, so the loader binds K1. The actual terminal record at t1 removes K1 and records its receipt. At s2 that receipt excludes K1, so the loader binds K2. Its actual terminal record at t2 removes K2. A checkpoint receipt for K1 excludes it from the first replayed record as well. Thus case 13 has a defined historical key at each range-only record; the latest index naming K2 cannot change the first selection.

Validation stays active throughout. Explicit completion/flush keys pass through `install_session_key`; a different bound incarnation fails. Terminal replay checks the exact closed incarnation and retains the real receipt sequence (`engine/durable.rs:577–610`). Missing eligible history supplies no invented key. A naked writer range fails the existing validator; a congruent existing RecoverSize intent keeps its accepted coverage rule. A mismatched rank cleanup sequence fails the final receipt join. The new text does not exempt records from these checks or erase state to make the join pass.

Only after reaching the actual frozen T does the loader require all rank rows Closed, an exact row/receipt join and no engine sessions/request/flush history. Intermediate historical bindings are allowed and are retired only by checkpoint receipts or actual terminal records. Valid Legacy2 range-only history therefore reaches clean export instead of failing early merely because its completion key is absent. Surviving valid Open/Closing state still returns UpgradeRequired.

Case 13 covers the original positive source, reused X under K1/K2, missing required history, an expired lifetime bound, wrong receipt and a surviving Open/Closing row. It also requires unchanged namespace, allocator, jobs and receipts. The pre-publication crash rows now reuse this loader. Post-publication recovery still loads the persisted candidate without requiring already-trimmed legacy records.

### Scope and evidence

No remaining U1/U2 contract defect was found. This re-review preserved the prior coherent downgrade and unknown-outcome proof. It did not re-review the active sections 3.2–3.4/R4 capacity policy. Only this report changed. Source inspection included one read-only `git show`; no implementation, tests, builds, live actions, Git writes or subagents ran. No live upgrade is authorized. Prior accepted packages and old-version G2 evidence retain their scope.

## Initial verdict — historical; U1/U2 now closed

**CHANGES REQUIRED — format/upgrade contract only.** References below use the reviewed 317-line `design/recovery-contract.md`. This review covers sections 3.5–3.8 and their clean-export/startup prerequisites. The separate reviewer owns detailed ordinary E1 peer, retention and accounting proof.

| Rank | Finding | Contract location |
|---|---|---|
| U1 / P2 | The immediate fresh-bootstrap upgrade has no specified ownership path that satisfies the different-writer rule. | Sections 3.5, 3.6 and 3.7 step 1; lines 195, 206–227, 235 |
| U2 / P2 | The store-owned legacy loader needs historical session binding before replaying range-only records. Final all-Closed checks do not supply that prerequisite. | Sections 3.3, 3.6 and 3.7 step 2; lines 144–150, 223, 236 |

These are concrete startup/upgrade specification gaps. I found no additional unsafe downgrade or cross-object publication transition in the stated protocol. Correcting them does not require a general transaction framework or a legacy-history repair policy.

## Original U1 / P2 — Resolve fresh bootstrap versus mandatory new ownership (closed)

Section 3.5 says a fresh Legacy2 root seed can immediately take the empty transition before OPEN. Section 3.7 step 1 requires independent acquisition with monotonic new ownership. It also requires a different incarnation/GID from every pre-transition legacy writer. The additional `acquire_format` API retains the existing acquisition fencing contract.

Actual source makes the conflict concrete:

- At `a6ae05a:crates/mds/src/persistence.rs:259–283`, fresh bootstrap uses the currently assigned `start.writer` and its incarnation/GID. It does not create an earlier, separately assigned writer.
- `crates/store/src/bootstrap.rs:145–178` puts that writer in both initial WAL and session authorities. `begin_bootstrap` uses it for the format authority at `:423–433`. Publishing Ready changes the format revision/phase, not the writer (`:594–632`).
- `acquire_preparing` rejects an equal incarnation or GID (`bootstrap.rs:456–462`). `StableStore::open` requires a higher incarnation and a different GID (`stable.rs:361–369`). `acquire_sessions` has the corresponding checks (`sessions.rs:146–153`). Generalizing the format phase does not change those ownership requirements.

Thus a fresh process A reaches Ready/Legacy2 with all three authorities owned by A. It cannot then acquire them from A using the specified APIs. Fencing A would fence its own backend. Inventing another owner token would fail the membership contract. Treating the fresh path as an implicit same-owner exception would leave the crucial stale-handle rule undefined.

**Minimal correction:** Remove the immediate same-start claim. Specify that fresh setup can finish the frozen Legacy2 seed while remaining unready, then requires a subsequent assigned incarnation/GID with explicit upgrade opt-in before OPEN. This preserves the existing acquisition APIs and the different-owner proof. If immediate same-start adoption is required instead, specify and review its narrowly checked fresh-only exception explicitly; do not weaken the existing-filesystem rule by implication.

**Required acceptance case:** Run the specified fresh path with one assigned writer and show the exact expected stop/continuation boundary. Then use the next assigned writer, preserve every seed byte, acquire each authority, and finish adoption before service. Assert that the old writer's prepared operations fail their guards. This is a requested future test, not an executed result.

## Original U2 / P2 — Carry the old range-only replay prerequisite into the store loader (closed)

The coordination method now owns checked legacy restore/replay itself. Section 3.7 step 2 describes replay through T and then the rank-row/receipt checks. It does not specify the rank-history binding that the existing production loader performs **before each replay record**. Raw `Engine::restore` plus `replay_record` is insufficient for some valid, fully drained Legacy2 sources.

Actual source:

- Existing-inode OPEN/CREATE can journal only a client-range change. `crates/engine/src/engine/ops_attr.rs:252–280,299–303` adds a completion in the truncate branch, but the range-only branch creates `RecordKind::ClientRanges` without that completion.
- `Engine::replay_record` automatically installs keys only from a completion or client flush (`crates/engine/src/engine/durable.rs:577–591`). Its shared final validator rejects a writer range with neither a live engine session nor a covering recovery intent (`:406–419,604–610`).
- The accepted daemon already handles this prerequisite at `a6ae05a:crates/mds/src/server/runtime.rs:210–242`. It loads rank rows first. Before replaying a range-bearing image, it binds a missing historical owner from those rows. It excludes retired keys and bounds a Closed key's historical lifetime by `cleanup_seq`. Final lifecycle/history checks follow replay at `:244–268`.

**Concrete valid source sequence, derived from source and not executed here:**

1. Checkpoint C contains a resident file and no session for client X.
2. A later durable rank Open allocates key K for X. An existing-file writable OPEN commits a range-only WAL record, with no completion/flush key.
3. Normal durable cleanup later retires K. Its terminal receipt is at T, and the rank row is Closed with that same sequence. All rank rows are Closed at the upgrade cut.

This source satisfies the intended final clean boundary. A raw replay of the first range-only record fails before it reaches T. Rejecting the source as corrupt is incorrect. Rejecting every such source as UpgradeRequired would also reject a supported, already-drained legacy history. Moving the new completion field into future records does not fix these existing bytes.

**Minimal correction:** State that the store upgrade loader reads the independently guarded rank history before legacy replay. Require the existing lifetime-bounded historical session binding, or a factored equivalent, during replay. This binding must not create a durable Open row, synthesize cleanup, drop histories or choose a replacement incarnation. The actual legacy terminal records must remove the temporary historical sessions. Only after replay reaches T may the empty-engine-session export and exact Closed/receipt bijection prove adoption. Keep this entirely inside the checked store coordination path; add no caller-supplied `clean` assertion.

**Required acceptance case:** Use the three-step source above, with C.coverage < T, and also reuse client X under successive durable SessionKeys. Verify the correct historical key for each range-only record, actual terminal retirement, unchanged namespace/allocator/jobs/receipts and successful clean export. Keep negative cases for missing rank history, wrong receipt sequence and a surviving Open/Closing key. These cases are not executed by this review.

## Version and downgrade audit

The proposed version boundary is suitable, subject to implementation review:

- Keep outer framing version 1, FrameKind values and identity layout unchanged (`framing.rs:8–30,68–103`). Change the semantic frames exactly as specified: WAL 3/3, checkpoint 2/2, checkpoint-session 2/2 and completion 2/2. Append the option field after the old completion watermark (`record.rs:696–725`). The frame selects engine recovery semantics; the outer checksum includes the changed body.
- The old Authority decoder checks CRC and then rejects a leading byte other than 1 as `UnsupportedVersion` (`stable.rs:1020–1032`). Format inspection reaches that decoder before phase interpretation (`bootstrap.rs:380–391`). WAL and rank-session inspection also use it (`stable.rs:302–335`; `sessions.rs:112–133`). This fence works even when WAL is empty and no completion exists.
- A version byte alone cannot revoke an already-prepared old WAL operation. Current WAL mutation guards compare identity and owner only (`stable.rs:973–976`). The required different owner, verified predecessor fencing and new full-authority guards close that race for existing-filesystem adoption. U1 concerns the separately promised fresh path.
- Required semantics must survive every authority constructor and revision. Relevant sites are `stable.rs:373–385`, `bootstrap.rs:465–480,809–815` and `sessions.rs:156–174,449–470,506–511`. Session staging clones its prior control, but reconstruction helpers must also preserve the new field.
- Exact supported version pairs must remain strict. Current `get_frame` rejects a mismatched pair, and `store_decode_error` preserves incompatible/unsupported-schema errors (`record.rs:790–805`). New dual-version decoding must keep that distinction. `BootstrapPlan::decode` currently collapses checkpoint errors to Corrupt (`bootstrap.rs:304–305`); the proposed preservation requirement correctly identifies that migration site.
- Legacy completion v1 has no truthful target field. It must decode as unknown None, without new residency checks. Valid legacy missing-target histories remain load-only and ultimately UpgradeRequired when still live. Semantics-3 missing/contradictory required references are damage. Ordinary Ready3 load must reject legacy nested frames and uncovered legacy WAL.

## Bootstrap, APIs and guarded snapshots

- Freeze all original deterministic encodings, including the initial Authority-v1 objects, not only the root checkpoint body. `BootstrapPlan::validate` rebuilds the whole plan through `new` (`bootstrap.rs:236–248`). `new` constructs root bytes, manifest, allocator receipt, WAL and session xattrs (`:113–222`). `encode` also serializes the root separately (`:251–277`). The contract's unchanged-plan requirement covers all of these sites.
- Preserve Ready dependency verification and the initial allocator receipt (`bootstrap.rs:654–698`). The upgrade must not regenerate root state or reset global allocation. Existing allocator reservations are independent monotonic CAS operations (`allocator.rs:33–123`); an upgrade guard is not allocation authority.
- `acquire_format` can generalize the existing guarded acquisition to the new phases. Its identity/full-authority/phase guard is local to `rmds.format`. The absent upgrade value is protected by an unchanged authority revision, not by comparing a missing xattr with empty bytes. Existing format mutations change that revision.
- `inspect_format` currently reads plan data and xattrs separately (`bootstrap.rs:369–399`). The proposed atomic snapshot or revision-bracketed reads are required. Optional-intent absence must not be confused with a missing format object. Backend helpers currently map ENOENT/ENODATA to MissingState (`backend.rs:55–60,103–109`).
- StableStore and SessionRepository own separate pending work (`stable.rs:269–294,964–969`; `sessions.rs:73–88,136–145`). The coordination method must reject unresolved work and use actual per-object snapshots. It cannot trust public inspection fields alone. `OpenedShard` has a private frozen snapshot (`stable.rs:107–123`); successful promotion must return the refreshed snapshot and refreshed session control.
- On a same-attempt retry, reconcile a recognized completed promotion under the retained id before treating an old input snapshot as a foreign-owner conflict. The contract's unknown-outcome rules require this behavior. It must not use a stale legacy authority to re-encode an upgraded object. A replacement instead independently acquires each observed actual authority.
- The callback requirement applies to inner reads, writes, listing pages and reconciliation awaits. The independent acquisition phase also needs the step-1 membership checks. A single outer daemon `Gate.wait` is insufficient; the committed retry wrapper checks only outside each multi-I/O store call (`a6ae05a:persistence.rs:86–101`). This is an implementation obligation already stated by the contract.

## Candidate, capacity and crash-cut audit

The stated order has a coherent durable recovery point at each cut:

| Cut | Required authoritative state and recovery |
|---|---|
| Candidate state or manifest write; no format intent | Legacy source remains authoritative. Recheck the actual source cut. Exclusive creation may reuse only identical bytes. An orphan is not clean-boundary proof. |
| Format barrier committed; sessions/WAL legacy | Authority v2 plus Upgrading and the exact plan stop old startup. Resume source verification and the persisted target, without allocating another upgrade id. |
| Sessions promoted; WAL legacy | Reacquire each actual mixed authority independently. Preserve rows/counter. Check source through T and compare the persisted candidate before promotion. U2 supplies the legacy replay prerequisite. |
| WAL promotion unknown | The authority and target pointer change in one operation on the WAL object. Exact old state permits identical retry; exact promoted state proves success. A half-state is invalid. |
| WAL promoted; trim partial or complete | Load the persisted target at T. Its immutable bytes and durable intent carry the completed source-proof result. Do not require deleted legacy WAL for restart or final logical checks. |
| Final Ready CAS unknown | Reconcile the exact Ready authority/phase/origin plan, or retry unchanged Upgrading. Release no service from an unknown outcome. |
| Replacement at any post-barrier cut | Fence current writers, retained predecessors and the candidate producer. Acquire actual authorities independently while preserving each version. Publish/read the original candidate under its original framed owner/name. |
| Later ordinary Ready3 restart | Use the current checkpoint pointer and uncovered v3 tail. Retained origin intent is historical. New Open rows are valid; all-Closed applies only to adoption. |

The same-object WAL promotion is the important atomic boundary. Format and session CAS operations do not guard it, and final Ready cannot atomically guard the other objects. The stated exclusivity, fencing, paused local workers, per-object guards and reinspection supply that proof.

Normal publication currently rejects a new descriptor whose name uses another owner epoch (`stable.rs:891–898`). The explicit upgrade-only exception is necessary for persisted predecessor candidates. Existing checkpoint loading validates descriptor/name/frame owner and the full manifest/state checksum chain (`:812–873`). Keep normal publication's restriction; do not rewrite a predecessor candidate under the replacement owner.

Candidate generation must use checked source.generation + 1, with coverage T. Existing writing enforces both generation and retained-object backlog limits (`stable.rs:650–782`), including orphan objects and already-existing planned names. Exclusive-create retry verifies exact bytes (`:784–798`). Use those rules for the candidate, including a crash after only its state object. A pre-barrier replacement may create its own candidate name after a fresh proof; after the barrier only the persisted descriptor is eligible. Capacity stops progress without deleting metadata. An already-persisted candidate does not require another generation on takeover.

The proposed intent encoding is bounded and can use existing writer/descriptor codecs. With their current 4096-byte writer-body and 256-byte descriptor-name limits, the stated length-prefixed layout is at most 4703 bytes, below the independent 8192-byte total limit (`stable.rs:161–173,1067–1087`). Still enforce that total limit on encode/decode, reject unknown versions, check CRC and exact trailing length, and validate source/target/producer relationships before the format barrier. No unchecked `clean` boolean is needed.

## Evidence limits and integration disposition

This review ran no tests, builds or live operations. The source scenarios above are not reproduced test results. Git use was read-only `git show` of committed old daemon files. The only changed file is this report.

After U1/U2 contract correction, implementation still needs the specified crash/unknown-outcome, archived-decoder, mixed-authority, candidate-takeover and clean-source tests. The startup option remains false by default. Neither an upgrade result nor an ACTIVE map grants service; refreshed checked load, receipt join, both accepted E2 boundaries and confirmed monitor service gates remain required.

This verdict changes no earlier accepted store/engine package or old-version G2 evidence. It authorizes no live upgrade and makes no fresh claim about fs44/meta88/data89.
