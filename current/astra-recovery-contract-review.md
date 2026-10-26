# astra-recovery-contract-review

Task: Independently review the small G3 addendum in design/recovery-contract.md, its minimum-feature-set decision, and the original requirements/C++ references. Review E2 control-drain and one-step semantics, E1 creator retry identity and proposed -EINTR compatibility, and D6 timeout/reconnect policy. Return ACCEPT or ranked CHANGES REQUIRED with exact sections and minimal resolutions. Source-only work; no code, live calls, tests, or subagents. Edit only this report.
Re-review task: Independently review revised E1 sections 3.2-3.4 and engine/format interface coherence. Check retained-target ordinary replies, durable completion retirement, all retention/purge paths, numeric RetryLimits and preflight/liveness, and clean legacy export. Read the required acceptance cases. Edit this report only; no production/design edits, tests/builds/live/commits/subagents. The separate store-state reviewer owns the detailed sections 3.5-3.8 audit.
R4 re-review task: Independently review the coordinator-selected terminal control-capacity policy, returned input/permit ownership, sticky engine failure, synchronous daemon stop, retained dirty/lifecycle obligations, startup RetryLimits configuration and the engine_config upgrade argument. Read current sections 3.3-3.4 and acceptance cases 11, 14-15. Edit this report only; source-only, no production/design edits, tests/builds/live/Git writes/subagents.
Status: COMPLETE. SCOPED ACCEPT whole E1 engine/ordinary retry contract, sections 3.2-3.4 and engine/configuration interface joins. R4 is CLOSED. The selected terminal capacity outcome preserves the dirty-control cut and removes the indefinite capacity wait. Prior E2/D6/wire-REPLAY acceptance remains in force. current/astra-retry-upgrade-review.md separately accepts sections 3.5-3.8/U1/U2. This is contract acceptance only.
Context: This reviewer authored current/astra-g3-integration.md, but did not author the addendum. The user selected a clean-session boundary on 2026-10-25. The two reports now supply the engine and upgrade scopes for whole E1 contract acceptance. The sole writable file is this report. Implementation, gate evidence and live-upgrade authorization remain separate.
Next: Coordinator can combine both scoped acceptances and assign E1 implementation with shared-interface sequencing. Design owner can reconcile the accepted section 3.4 engine_config argument into the section 3.6 prototype. Implementation review must verify cases 11, 14-15 and the ownership/order obligations below. No contract finding remains open in this report.
Log:
- [2026-10-25] Created this report before source review.
- [2026-10-25] Read the addendum, touched decision text, original recovery requirements, Rust callers, and C++/kernel peer paths. Verified the REPLAY trace restriction, direct-error handling, RecoverSize supersession, production restore path, and timer defaults.
- [2026-10-25] Completed the review below. Edited only this report. Ran no tests, builds, live calls, Git operations, or subagents.
- [2026-10-25] Started the requested R1/R3 and E2/D6 separability re-review. Read the revised current/sol-design.md and design/recovery-contract.md. The blanket EINTR policy is withdrawn; section 3.3 remains owner-blocked.
- [2026-10-25] Completed scoped re-review. Rechecked the wire REPLAY peer paths, current engine signatures/step/job interfaces, and named production restore/readiness callers. Accepted E2/D6 and section 3.1 independently. R2 remains open. Edited only this report; no production edits, tests, builds, live calls, Git operations, or subagents.
- [2026-10-25] Started the revised E1 engine/ordinary-retry contract review. The separate store-state review owns the detailed cross-object upgrade audit. Prior scoped acceptances remain in force.
- [2026-10-25] Completed source-only E1 review of the 317-line revised contract and all final acceptance cases. Read current engine completion/step/replay/checkpoint/purge/control paths and rechecked the C++/kernel negative-target consumers. Found R4: a legal dirty xattr flush can require persistent charge above the full base budget while its freeing watermark request is gated behind that flush. The transient reserve cannot fund this committed growth. Edited only this report. Ran no tests, builds, live calls, Git commands, commits or subagents.
- [2026-10-25] Started the requested R4 re-review. The selected resolution is terminal control-capacity failure with a reviewed same-identity restart. Preserve prior scope conclusions and independently check the changed policy and configuration interface.
- [2026-10-25] Completed source-only R4 re-review of the 372-line contract, including full cases 11, 14-15. Rechecked engine failure/step/cursor/cap/lifecycle paths, daemon pump/cancellation/output/job/storage gates, startup parsing and both EngineConfig construction sites. Read the independent U1/U2 acceptance. R4 is closed; scoped whole-E1 engine/ordinary-retry contract acceptance follows. The borrowed configuration argument adds no upgrade-algorithm blocker. Edited only this report. Ran no tests, builds, live calls, Git commands, commits or subagents.

## Current R4 re-review verdict — 2026-10-25

**SCOPED ACCEPT whole E1 engine/ordinary retry contract; R4 CLOSED.** No remaining specification blocker was found in sections 3.2-3.4 or their engine/configuration interface joins. The earlier ordinary-reply, completion-coverage, durable-retention and clean-export conclusions remain valid. The terminal policy resolves their sole remaining finding.

References in this section identify the inspected 372-line `design/recovery-contract.md`. Engine paths below are relative to `crates/engine/src/`; daemon paths are relative to `crates/mds/src/`. These are source observations and implementation obligations. The proposed E1 fields, errors and configuration plumbing are not accepted implementation by this review.

| Reviewed boundary | Disposition |
|---|---|
| Terminal error, ownership and sticky failure, sections 3.3-3.4, lines 144-156 and 193-201 | ACCEPT. The valid blocked control has an explicit terminal outcome. |
| Ordinary pressure, accounting and independent bounds, lines 183-191 | ACCEPT. Ordinary waiting keeps its actual-wake and fitting-retirement rules. |
| Startup parsing and one limit snapshot, lines 203-212 | ACCEPT. The named startup paths can carry the same validated limits. |
| Cases 11, 14-15, lines 366 and 369-370 | Sufficient requirements for the changed policy. They were not executed. |
| Sections 3.5-3.8/U1/U2 | Preserve the separate SCOPED ACCEPT in `current/astra-retry-upgrade-review.md`. The configuration join adds no blocker. |

### R4 closure: the original cycle now has a terminal outcome

The original case has no internal capacity-release event. All reconnect keys are final. Its fitting watermark Request remains behind the control-drain gate. A 4 KiB xattr value adds at least 8 KiB across the live and committed cores. Starting at `67,108,864 - 1,024` bytes gives a projected charge of at least **67,116,032 bytes**, or 7,168 bytes over the base limit. Fitting record bytes, completion count and transient copies cannot authorize that commit.

Sections 3.3-3.4 now require this otherwise valid control to return `ControlCapacity` before mutation. The engine latches failure, returns the untouched control and unused permit, and emits no effects/notices. It does not wait for the held watermark. Empty/resume calls stay Failed. Supplied inputs, including a marker, return owned and unexecuted. Case 14 reproduces the exact combined state and requires the synchronous daemon stop.

This resolves the former progress claim by making the availability loss explicit. It does not claim that the failed engine drains its controls. The accepted section 2 cut still requires commit/deduplication or fenced durable cleanup before grant release. Capacity failure establishes none of these facts. Shutdown cannot turn queue removal, a read-only floor, checkpoint, timer, eviction or a synthetic close into drain proof.

For this particular byte-overflow case, an adequate reviewed larger base budget permits a replacement to recover the same durable identity and apply the retransmitted flush once. The ACK follows commit. The normal control cut and marker then precede the fitting watermark Request. The replacement uses a newly assigned/fenced writer; it does not reuse the failed writer token. A control that exceeds the fixed transient or record bound still fails its independent check. Raising base bytes does not promise to cure that different case.

### Input ownership, preflight and lifecycle work

The new contract specifies a sufficient ownership boundary for both newly supplied and engine-owned controls:

- A dirty `CapUpdate` always returns `Some(input)`. A resume through `step(None, permit)` retains the original non-request classification. It cannot become ordinary `RetryCapacity` work merely because the external argument is None.
- Multi-item work preserves the current unexecuted item and remaining cursor before failure cleanup. Previously committed items remain committed. Internal work without an external input may return None only while its durable job/range obligation remains intact.
- Identity, flush-duplicate and lifecycle-guard checks precede charge preflight without side effects. A proven duplicate adds no dirty-metadata growth. Any separate range growth still needs its own check.
- Preflight precedes metadata, range, cap sequence/rights, history, lifecycle retirement and WAL changes. Pending/unknown transactions keep their existing ordering. The engine cannot speculate through a pending append to invent an admissible projected state.
- The error reports checked projected committed slots/bytes and additional transient bytes. None means overflow. Its latched diagnostic remains available for supplied inputs after failure. The existing `JournalError::Capacity` is sufficient; the public error needs no storage event or persistent field.

Current source shows where these requirements must be implemented:

1. `engine/step.rs:22-34` clears `continuation` and `release_cursor`. The capacity path must recover their ownership before calling destructive cleanup. The generic failed branch at `:114-115` currently consumes a supplied input; the new capacity-specific branch must return it instead. `:165-205` stores/takes continuations and separates the release cursor from the placeholder input. Returning only that placeholder would lose the remaining items.
2. `engine/client_caps.rs:104-150` trims flush history and changes cap state before the current mutation helper. `:208-251,291-301` applies fields and submits the flush. Checking only in `submit` would be too late. The specified pure validation/preflight must precede those effects. The earlier E2 dirty-control test at `crates/engine/tests/recovery_controls.rs:154-260` confirms the relevant delayed-commit and marker dependencies; I read it only.
3. `engine/step.rs:242-280` and `engine/client_caps.rs:344-417` perform multi-turn close/range cleanup. Their current pre-submit cap/request changes need the same preflight ordering for a capacity-failing unit. No terminal receipt can substitute for a failed unit.
4. `engine/durable.rs:725-776` validates a lifecycle result, changes metadata and submits job retirement. The new preflight must run after guard validation but before those changes. Failure keeps the durable intent and ranges available to checked recovery. `pending_jobs` and `emit_job` already check failure at `:641-685`; later callbacks cannot release failed-engine work.

These are specified implementation changes, not an unresolved input-ownership interface. One returning `EngineInput`, an unused linear permit and private retained cursor/diagnostic state can express the boundary.

### Synchronous daemon failure and bounded terminal retention

Lines 197-199 require the fatal gate in the same pump error turn. The daemon must retain the returned control before cancellation, return the permit once, and stop the loop before another engine step, command/outbox transfer, output or job scheduling. The bounded terminal hold reuses the input's admission charge. It does not allocate an unbounded second copy. Other protected queues and uncertain-operation identities remain owned through controlled shutdown.

The fatal diagnostic retains the capacity classification and logs numeric configured/required bounds or explicit overflow. Logging the whole returning error with Debug would expose client input. Both the log path and Display must omit that payload, as line 197 requires.

Three source joins make this ordering important:

- `server/runtime.rs:779-789` subtracts a dequeued input's byte charge. A resume currently carries `(None, 0)`. The new implementation must carry the active input/cursor charge across engine ownership and transfer that charge to the terminal hold. Re-enqueuing a returned continuation with zero charge would violate the stated bound.
- `Runtime::cancel_pending` at `:121-132` clears protected controls and operation IDs. `fail_runtime` at `:567-573` calls it. `pump` temporarily owns Runtime outside `self.runtime` at `:661`; its fatal tail calls cancellation again at `:833-847`. The new path must preserve the terminal hold and uncertain identities in both ownership locations. The ordinary Busy branch and the generic Failed-to-Unavailable branch at `:590-592` are not the required capacity handler.
- `daemon.rs:306-326` calls phase/control/job drivers after `pump`, before a later explicit failure check. Their permission predicates consult `check_failure` through `control_allowed` (`:712-724,837-913`). A same-turn fatal latch therefore has a concrete propagation path. `Server::fatal_error` also observes engine failure (`server.rs:713-717`). `ready_jobs`, readiness and delayed output/completion paths must retain that gate; a later Tick is insufficient.

The unknown-outcome clause is coherent with this stop. `engine/step.rs:137-141` excludes another step behind a pending record. `DaemonJournal::reserve` also excludes queued/in-flight writes (`journal.rs:60-73`). The failed preflight submits no new record. Already submitted storage or data work can still have an uncertain result. It retains its exact operation identity and the existing reconciliation rules; stopping service cannot prove that an issued write was absent or committed. The worker's retained append/repository operations (`persistence.rs:519-601`) and the store pending-state contract remain authoritative. Case 14 explicitly separates this cut from its quiescent capacity-failure case. No delayed completion may release output or restart the failed engine.

### Accounting, ordinary pressure and startup configuration

The default committed limits remain 65,536 completions and 67,108,864 bytes. The single active lane has hard bounds of 65,537 slots and 75,497,472 bytes. Persistent growth still fits the base. Additional active copies still fit the fixed 8 MiB allowance. Count, committed bytes, transient bytes and record size remain independent.

At a full completion count, a byte-only control can fit if it creates no completion and satisfies the byte/transient checks. A full byte budget fails growing controls even with spare slots. Transient-only overflow is terminal for a valid non-request control. Checked arithmetic gives explicit overflow diagnostics. Case 15 covers these dimensions independently.

Ordinary Requests keep `RetryCapacity` without failing the engine. Lines 185 and 191 require bounded non-runnable pressure wait, actual capacity-change wakeup, and revalidation. Fitting controls can revoke ordinary pre-mutation reservations. After grant release, fitting watermark-bearing work and independently initiated terminal close can pass a pressure-blocked owned Request. No such bypass opens the Request gate during Control drain. Case 11 now tests this separately from terminal control failure.

Startup configuration is sufficient and operationally reachable:

| Path | Source observation and accepted migration |
|---|---|
| Parse and reject before startup | `config.rs:19-75` supplies the stated key normalization and section precedence. `config::from_args` runs before daemon/runtime startup (`main.rs:33-40`). Add both positive ASCII-decimal values and all checked conversions/additions there. |
| Strict numeric grammar | The existing generic `number` helper at `config.rs:240-244` calls `parse` directly. It is insufficient by itself for the new no-sign grammar. Trim, require nonempty ASCII digits, parse u64, convert to usize, reject zero, and check count+1 / bytes+8 MiB. A malformed specific-section value must not fall through to a lower section/default. |
| Persistence/startup snapshot | Add `PersistenceConfig::retry_limits` (`config.rs:115-132`). Copy the validated values into the EngineConfig at `daemon.rs:465-487`, which enters `Start.config` (`persistence.rs:146-153`). |
| Bootstrap and checked upgrade | The seed already uses `start.config.clone()` (`persistence.rs:259-278`). Preserve the limits while selecting the explicit Legacy2 seed semantics. Supply `&Start.config` to the new coordinator argument. Its source engines and candidate checks use the same snapshot. |
| Restore/replay/serving | Populate the second EngineConfig at `daemon.rs:519-540`. `Server::from_loaded` overrides only lease and maximum record size before passing config to restore (`server/runtime.rs:219-235`). Preserve retry limits through those overrides and every replay/serving engine. |

Configured limits are runtime policy, not persisted format bytes. Valid legacy fields keep their legacy interpretation. Over-budget semantics-3 recovery returns Capacity without deletion or corruption classification. Neither default reconstruction nor a blanket restore-error-to-Corrupt mapping can satisfy these requirements. The snapshot requirement closes the risk of successful larger-limit source checking followed by default-limit service restore.

### Upgrade join and acceptance scope

Section 3.4 line 212 explicitly adds `engine_config: &EngineConfig` immediately before `upgrade_id`. The frozen section 3.6 prototype at lines 253-257 awaits textual reconciliation. This addendum is unambiguous. The store already depends on engine (`crates/store/Cargo.toml:8`), and EngineConfig is an owned, cloneable, wire-free configuration (`engine/mod.rs:48-99`). The coordinator can clone the borrowed snapshot for its owned restore calls. It gains neither an unchecked clean-state assertion nor authority to serve.

The argument changes no format version, candidate identity, U1 two-process boundary, U2 historical session binding, per-object guard, unknown-outcome rule or adoption condition. No actual upgrade-algorithm coupling requires reopening the separate acceptance. The design owner can merge the argument into the prototype during the assigned reconciliation.

The current acceptance combines this R4 closure with the prior E1 engine/ordinary-retry proof below. The independent upgrade report supplies its separate accepted scope. E2, D6 and matching wire REPLAY retain their earlier acceptance. All earlier findings in this report are now historical and closed at the contract level.

Only this report changed. This review used source reads and manual reasoning. It ran no tests, builds, live operations, Git commands, commits or subagents. Cases 11, 14-15 remain implementation acceptance requirements. No new gate evidence, implementation acceptance or live-upgrade authorization follows from this verdict.

## Prior revised E1 engine/ordinary-retry review — historical, R4 now closed

**Historical verdict: CHANGES REQUIRED, R4, P2, recovery-progress specification blocker.** The current re-review above closes R4 and supersedes this hold. The ordinary-reply, retention and interface analysis below remains part of the accepted proof. Prior E2/D6/wire-REPLAY acceptance remains in force.

References below identify the 317-line `design/recovery-contract.md` and current engine source inspected in this review. The current engine includes the accepted E2 phase implementation. Earlier source locations below remain historical. I read sections 3.5-3.8 for interface coherence. The separate report `current/astra-retry-upgrade-review.md` owns their detailed cross-object audit.

Rust engine module references use `crates/engine/src/engine/`; `durability.rs` uses `crates/engine/src/`. Store and daemon references use `crates/store/src/` and `crates/mds/src/`. C++ paths use `/home/ubuntu/ceph/src/`; Linux paths use `/mnt/workspace/ceph-client/`. Linux source supplied behavior facts only.

| Scope | Result |
|---|---|
| Ordinary unchanged/changed identity replies, section 3.2 | No additional blocker. The nonempty capless original target supplies the missing peer-safe error trace. |
| Completion coverage and identity, section 3.3 | No additional blocker. The separate target field covers existing-inode mutations without inventing creation. |
| Canonical history, retirement and purge, section 3.4 lines 154-167 | No additional blocker. The proposed invariants close the volatile-floor and lost-callback retention traps. |
| Accounting and control progress, section 3.4 lines 169-175 | Numeric accounting is implementable. R4 blocks the specified pressure policy at recovery control drain. |
| Engine/format interface join | No additional blocker in this scope. Detailed store-transition acceptance remains separate. |

### R4 — P2: protected dirty controls need persistent headroom or a defined pressure outcome

**Exact locations:** section 3.4 lines 169-175, especially the committed 64 MiB ceiling and park-on-`RetryCapacity` rule at line 175. The dependency is section 2's Control drain row at line 30 and ordered dirty-control cut at line 57. Section 5 cases 2 and 11 need the combined regression below.

The 8 MiB allowance is expressly **transient**. A completed transaction must return to `max_bytes`. Checking a dirty control only when it arrives can therefore leave it with no admissible commit, even when its record and transient copies are small. The contract does not reserve persistent capacity for metadata that a client can already dirty under issued rights. Preflight of the immediate cap grant sees no such metadata growth: caps themselves are outside this charge.

Concrete allowed sequence:

1. A semantics-3 engine has canonical histories and referenced metadata charged at `64 MiB - 1 KiB`. Counts remain below 65,536. Normal admitted work can reach this state under the stated limits. A referenced file `I` has an empty xattr value. Its Open client has valid xattr-exclusive rights. No rule requires a future-growth reservation for those rights, or when `I` first becomes referenced.
2. The client changes that value to 4 KiB locally. A new dirty flush is protected across MDS recovery. Its flush tid is not completed. All surviving Open keys finish reconnect. There is no missing key awaiting eviction and no job that can free history.
3. The update adds at least 8 KiB of charge for the value in the live and committed cores. The value is well below the default 64 KiB xattr limit. The record and additional transient copies fit their independent limits. The resulting committed charge is nevertheless at least `64 MiB + 7 KiB`. Section 3.4 forbids that commit and directs the daemon to park the unchanged control on `RetryCapacity`.
4. A new completion-bearing request with a fitting watermark could retire enough history. But every Request remains gated during Control drain. The grant-release marker must wait for the protected dirty control to commit/deduplicate or undergo fenced terminal cleanup. The freeing request thus waits for the flush, while the flush waits for space that the request could free.
5. A read-only floor, checkpoint, cap release or output delivery cannot retire these references. A terminal close is not required to arrive from this fully reconnected client. D6 ACTIVE timers do not run in this phase. Merely allowing a close to bypass pressure does not schedule one. With healthy storage and no further external action, the specified wait has no capacity-changing event.

This is a control-drain cycle, not a large-record or allocator-RSS concern. It also does not depend on a malformed retry or a client retaining a vanished inode. A reviewed restart with larger limits is an external remedy; it does not establish the ordinary control progress claimed by line 175.

Current source confirms the dependency:

- `engine/client_caps.rs:198-251,291-301` applies the dirty xattr map and records a `ClientFlush`. That record has no `CompletedRequest`, so it cannot advance the proposed durable request floor.
- `engine/step.rs:148-160` rejects Requests until grant release. `engine/reconnect.rs:160-173` opens Control drain only. Moving the blocked control out of `continuation` does not open the Request gate.
- `crates/engine/tests/recovery_controls.rs:154-260` already models a protected early dirty xattr flush, omitted dirty rights in reconnect, delayed commit and a blocked marker. I read this test; I did not run it. Adding the proposed full retry budget exposes the extra dependency.

**Minimal resolution:** Specify how persistent control-growth capacity is secured before ordinary admission can consume it. A headroom reservation must be charged within `max_bytes`, cover already-issued variable-metadata rights and protected controls, and remain valid across reconnect. Cover a newly referenced inode that already has such rights. Checking only at flush receipt is too late. Keep the one-active-transaction transient reserve separate.

If the intended policy instead stops recovery and requires a larger-limit restart, specify that terminal capacity outcome and its daemon action explicitly. Do not leave this case in a normal wait that has no wake source. The owner must select the policy; this review does not add automatic eviction or relax the accepted control cut.

**Required regression:** Place this legal xattr-growth flush in Control drain near the byte limit, with every key final and a fitting watermark request held outside the engine. Verify the selected reservation/progress or explicit capacity-failure path. Require no premature FLUSH_ACK, grant marker, volatile retirement, discarded dirty input or repeated polling. A grant-released test where a watermark fits does not cover this case.

### Ordinary reply proof and R2 disposition

The clean-session deployment boundary is sufficient to establish resident originals **when combined with the specified prospective retention invariant**. It does not repair legacy histories. A checked history-free starting cut has no target dependencies. Every later affected completion installs an explicit target with the mutation. Purge must preserve that target until durable retirement. Replay recreates the same dependency after a lost callback. These steps cover every promised completion class in section 3.3.

The earlier R2 missing-original policy gap is therefore resolved for the proposed semantics-3 service scope. Valid legacy Open/Closing histories remain incompatible and return `UpgradeRequired`. An accepted deployment choice alone does not prove the actual stored cut or authorize its upgrade.

Rechecked peer behavior:

- C++ `client/Client.cc:1781-1811,1848-1868,1871-1915,1944-1973` distinguishes the unsafe/REPLAY shortcut, the unsafe ordinary empty-trace dereference, and the nonempty target-only branch. The last branch avoids the old request-dentry directory for these creator/open opcodes. Its final dentry-null check is not a directory dereference.
- `add_update_inode` accepts zero caps at `:1244-1245`. `make_request` invokes successful trace verification only for nonnegative results at `:2378-2403`. `_create` exits on error before creating a file handle at `:16214-16233`. Existing-inode `_open` also creates a handle only on success and drops its pending open reference on error at `:11189-11218`.
- Linux `fs/ceph/mds_client.c:4152-4187,4218-4229` processes a valid target trace even when the request result is negative. `inode.c:1710-1743` fills it with fmode -1 and skips dentry insertion. Zero caps do not allocate a cap at `:1063-1069`. `file.c:957-1005` distinguishes CREATE's special ENOENT follow-up from EINTR's error exit.

These facts support the uniform **SAFE EINTR, original target only** rule. They do not establish a detached-request reachability reproducer or installed-peer qualification. The error is application-visible after a committed mutation. A new application tid remains a separate operation.

The unchanged rows also preserve identity correctly. Trace the requested hard link even if another link is primary. An ancestor rename does not change an inode-relative request base. Direct-inode OPEN may return a target-only success; a named creator cannot be relabeled as that case. Missing/replaced name and untraceable parent use retained `I`, never replacement `J` or another parent. A removed directory retains wire `nlink=1`; symlink contents and stable subvolume identity remain available without a live parent.

The explicit no-rights rule needs deliberate implementation. Current `materialize_reply` always uses `issue=true` and derives a snap trace (`engine/mod.rs:900-921`). Current `inode_stat(..., false)` avoids issuing caps, but still advertises a durable range after grant release and returns the durable directory link count (`caps_eval.rs:446-557`). Section 3.2 already requires zero `max_size`, zero caps/wanted/cap-id/seq/mseq/flags, empty snap trace and directory normalization. Setting only `result=-EINTR` or only `issue=false` will not meet it. These are implementation obligations, not another contract finding.

### Completion coverage and matching order

Section 3.3 covers both actual construction sites for new creators: `ops_ns.rs:407-436` and `:538-561`. They record the allocation, target inode and completion together. MKDIR delegation from MKNOD must retain the actual request opcode.

Existing CREATE delegates to `op_open`. That function currently creates a completion only in the truncate branch (`ops_attr.rs:253-272`), then can create a range-only transaction at `:274-280`. The revised placement after deciding whether any transaction exists covers:

- truncate with nonzero or zero prior size;
- existing-inode CREATE and OPEN with only a durable range change;
- combined truncate/range work with one completion;
- a purely volatile open with no completion or added WAL transaction.

The retry's own range-extension transaction must keep its captured completion context and must not execute the new-completion path. `created_ino=None` remains None. CREATE extra comes only from recorded creation; `reply_target_ino` is never an extra-field substitute. Non-CREATE opcodes carry no CREATE extra. Recorded result replaces today's literal-zero duplicate response.

The identity check must move ahead of the current conditional retry-counter lookup, trim and releases (`mod.rs:542-579`). It must check ordinary zero-counter duplicates too. In-flight identity checks need the original op as well as client/tid. The new returning error variants fit the existing ownership shape in `durability.rs:137-148`. Daemon validation must preserve the original request validation context when rejecting an op mismatch. None of these changes needs a public reply cache or a wire-dependent engine type.

### Durable history and target lifetime

The canonical-table rule is sufficient and implementable:

1. The observed floor may advance without deleting canonical entries. Commit and replay use the same monotonic durable floor, trim and insertion rule. A new record must validate its watermark before mutation. A known completion hit precedes floor rejection. An unknown old tid cannot execute anew.
2. Completion/reference installation precedes every deferred reply, lifecycle effect or purge evaluation. Current deferred completion handling uses the volatile floor (`mod.rs:1314-1320`); it must change. Do not rely on incidental deferred-vector ordering. Current commit processing removes holds before releasing deferred output (`:1226-1252`). The new reply-context pin must remain valid through materialization.
3. An unknown append outcome keeps old references. Replay of a durable append derives the new canonical state without needing its lost callback. Matching retries capture both original identities before a later permitted retirement.
4. `state_image` currently exports the observed floor (`durable.rs:66-78`). Semantics 3 must export the durable floor and all canonical entries. Restore initializes both floors from it. No checkpoint construction/publication or output callback is a retirement event.
5. Terminal `closed_sessions` commit retires the old incarnation. Earlier cap removal and rank Closing do not. Current `process_commits` and replay already have the relevant terminal boundary (`mod.rs:1215-1224`; `replay.rs:123-126`). A new incarnation cannot receive the old reply context.

The replay candidate is checked after applying durable trim and terminal retirement. Current `durable.rs:604-610` already has this whole-candidate validation point. Add semantics-3 required-target/residency/Purge checks there and during checkpoint restore. Preserve legacy validation separately. Do not infer a missing target from images, allocation order or a current path.

### Purge and lifecycle coverage

Section 3.4 must be implemented at the final removal point and all its callers. The inspected paths are:

| Path | Current source | Required retention effect |
|---|---|---|
| Immediate last-link removal, RMDIR, rename overwrite | `ops_ns.rs:177-242,632-686,771-797` | Keep the stray and its saved authorization path while any canonical reference or admitted reply pin survives. The immediate purge is in `remove_link`, after `unlink_dentry`. |
| Last cap, range cleanup, close | `client_caps.rs:344-417`; `step.rs:242-280` | Preserve history until terminal commit. Include retention in the `doomed` decision at `client_caps.rs:390-395`; otherwise cleanup can skip the range/recovery transaction while the new purge guard retains the inode. |
| Commit-triggered and restored unclaimed strays | `mod.rs:1242-1251`; `reconnect.rs:12-17`; `step.rs:207-215` | Filter/recheck references and temporary reply pins. Neither recovery boundary retires them. |
| Final metadata removal and data intent | `caps_eval.rs:698-782` | A pin prevents both removal and Purge intent. Do not replace a required RecoverSize job with Purge while pinned. |
| Job exposure/completion and replay | `durable.rs:356-420,550-565,641-776`; `replay.rs:99-111` | Reject surviving-reference/Purge coexistence. Preserve independent truncate/probe guards and range-owner congruence. |

The existing `doomed` shortcut is an important implementation trap, not an omitted contract path: section 3.4 explicitly covers session cleanup and every eligibility path. It must not leave an ownerless range without a recovery intent at terminal close. A retained target can still complete Truncate/RecoverSize under their guards. Namespace, statistics and quota removal occur once at unlink. Retirement commits first; a fresh admitted turn rechecks references/pins before ordinary purge.

### Numeric limits and scheduler obligations apart from R4

The accounting definition does not require knowledge of Rust allocator internals. Buffer capacities and element sizes are observable. BTreeMap/Set entries use the specified logical charge, not an inaccessible node allocation size. Byte buffers use their explicit capacity rule; other Vec storage and nested allocations use the recursive rule. This is an accounting ceiling, not exact RSS or retained data size.

- Count all canonical completions, including non-target operations. Charge the derived index and each distinct referenced target once. Linked directory entries, both core copies, xattrs, layouts, ranges, symlink and authorization-path buffers are included. Mutating a referenced target can consume budget without adding a target reference.
- At defaults, the base limits are 65,536 slots and 67,108,864 charged bytes. One active transaction/materialization may reach 65,537 slots and 75,497,472 bytes. Configured arithmetic must be checked. The allowance cannot multiply across parked requests.
- A fitting watermark transaction can retain the old canonical entries until commit and use the active extra slot. Projected durable trim cannot authorize early purge. A reservation must account for any admitted reply pin that still retains a target after canonical retirement.
- Parked pre-mutation reservations stay in the base budget and must be revalidated. A pressure-blocked owned input needs a non-runnable wait classification and an actual capacity-change wakeup. It must not keep returning `NeedsAdmission` on unchanged state.
- The existing blanket Busy predicate (`step.rs:137-141`) blocks all Requests behind parked work and all input behind `continuation`. Section 3.4's pressure-bypass requirement therefore requires a scheduler change. A pressure-blocked continuation cannot remain the exclusive runnable continuation while claiming that fitting retirement work can bypass it. Preserve one pending record, op/tid identity, reply pins and output materialization order. These are specified implementation obligations. They can be implemented within the existing signatures after grant release; they do not solve R4's earlier phase gate.

Recovery above configured semantics-3 limits correctly returns `Capacity` without deletion. Insufficient transient reserve can reject a transaction before side effects. These deliberate limits can pause new mutations. R4 specifically concerns an already-protected control that blocks the phase needed by its capacity-release dependency.

### Load-only export and format interface coherence

The proposed enum belongs in engine; store selects it from the semantic frame. There is no engine wire dependency. Adding the checkpoint/record tag, completion target, config limits and returning errors does not require changing the retained `step`, `journal_event`, `restore` or `replay_record` signatures.

`checkpoint_for_retry_upgrade(&self)` can use the existing materialized state fields (`durability.rs:44-68`; `engine/durable.rs:52-104`). It must check full state and no engine sessions, not merely empty completion vectors. Its returned checkpoint changes only the semantics tag. The loaded engine remains Legacy2 and unready. Store owns rank-row/receipt validation and publication; no unchecked `clean` boolean crosses that boundary. The load-only journal must report the checked tail before the helper compares sequence positions.

The equality check includes `covered_seq`, all inode cores/linkage/entries, allocator free/delegated ranges and floor, jobs, `next_job_id`, `last_cap_id` and cleanup receipts. Sessions must already be empty. The helper cannot clear a session or history to make this comparison pass.

The existing bootstrap caller supplies an `EngineCheckpoint` to `BootstrapPlan::new` (`mds/persistence.rs:261-278`; `store/bootstrap.rs:67-113`). That shape can carry an explicitly constructed Legacy2 root-only seed. Constructor/fixture defaults must be deliberate, and the frozen legacy bootstrap encoder must not silently encode a normal semantics-3 image as legacy. Section 3.5 already requires this migration. Restore itself creates no root.

The existing completion decoder has no original-target field (`store/record.rs:696-725`), and the legacy validator does not require creator residency (`engine/durable.rs:326-335`). Preserve that legacy validity. Load/replay may inspect it without output; the clean-session gate returns `UpgradeRequired`. The new affected completion cannot silently accept None. Exact new/old semantic frame pairs and preserved `UnsupportedVersion` distinguish incompatibility from damaged new state. Ordinary Ready3 recovery can have new Open histories; it must not repeat adoption's all-Closed condition.

These interface conclusions do not accept the separate per-object authority/crash proof. That review remains owned by `current/astra-retry-upgrade-review.md`.

### Required cases and re-review boundary

I read all eleven cases in section 5. Cases 3 and 7 cover creator/existing-inode identity and exact extra. Case 8 covers durable-floor retention, lost callbacks, shared references and later purge. Cases 9-10 cover the legacy/clean-export interface and preserve the separate store audit. Case 11 covers count/byte limits and ordinary Ready3 restart. Cases 1-2, 4-6 retain the prior E2/D6/REPLAY boundaries.

Add the combined R4 full-byte-budget/early-dirty-control case to cases 2/11. Package review must also exercise a pressure-blocked owned continuation with fitting retirement work, and the retained-target `doomed` range-cleanup path identified above. These are concrete checks of the stated obligations.

Re-review can focus on R4 and the resulting accounting/admission interface changes. No new blanket empty-error proof, historical reply image, general upgrade, snapshot or multi-rank work is required. E1 engine-scope acceptance remains withheld pending R4. Whole E1 also needs the independent store-state acceptance. No implementation, mounted-client gate or live upgrade is accepted by this report.

## Prior scoped re-review — 2026-10-25

**SCOPED ACCEPT: E2, D6, and the matching wire REPLAY rule.** The E2 engine package can proceed independently of the ordinary E1 case-policy decision.

The ordinary E1/R2 statements in this prior review describe its earlier contract revision. The revised E1 review above now controls that scope. The E2/D6/wire-REPLAY acceptance remains in force.

This verdict covers the revised `design/recovery-contract.md` inspected with `current/sol-design.md`. Section/line references in this re-review refer to that 178-line contract revision. The initial review below retains its earlier source locations and findings as history.

| Scope | Disposition | Accepted boundary |
|---|---|---|
| R1 | CLOSED | Section 3.1, lines 85-89; separate acceptance case 4 at line 174 |
| R3 | CLOSED | Section 2, lines 33 and 66-77; startup acceptance case 5 at line 175 |
| E2 | ACCEPT | Section 2, lines 13-77, including grant suppression, control progress and caller migration |
| D6 | ACCEPT | Section 4, lines 130-167, including timer policy and recovery retention rules |
| R2 / ordinary E1 | OPEN | Sections 3.2-3.3 are outside this acceptance. The separate case proof and owner decision still apply. |

No additional specification blocker prevents this separation. The contract/status text still records the pre-re-review consumer hold. The parent can narrow that hold to the scope above. That status reconciliation does not require another engine API or a resolved ordinary E1 response policy.

### R1 closure: recorded-result safe completion

Section 3.1 now requires the original op/tid and recorded result, empty target/dentry/snap traces, and no reopen, cap grant or writer-range grant. Rename, replacement and purge do not change that response. CREATE preserves the recorded inode in its opcode-valid extra; non-CREATE operations carry no CREATE extra. This addresses R1 directly.

The decisive peer order remains `Client::handle_client_reply` → `insert_trace` → safe completion (`/home/ubuntu/ceph/src/client/Client.cc:3060-3062,3079-3112`). For `got_unsafe`, `insert_trace` checks the empty trace and returns **before** the ordinary empty-trace dentry access (`:1781-1788`). The revised rule therefore does not depend on the ordinary request-dentry proof left open in R2. Kernel `fs/ceph/mds_client.c:4117-4138` likewise completes the already-answered request without revising its result. C++ `mds/Server.cc:2657-2684` retains the separate completed-REPLAY shortcut.

This acceptance requires the matching durable session/tid/original-op identity stated before section 3.1. It does not permit relabeling an ordinary lost-safe retry as REPLAY. It does not accept the proposed ordinary error/trace rows in section 3.2.

### R3 closure: signatures and postconditions agree

The revised contract keeps both engine function signatures and the boolean `try_finish_recovery` signature. It adds the unit `EngineInput::RecoveryControlsDrained` variant. The changed postconditions are now explicit:

1. Every production checked restore starts in Reconstruction. This includes fresh Ready bootstrap and a clientless load completed after an ACTIVE map. The direct non-restored bootstrap-constructor exception cannot bypass the production restore path.
2. `finish_reconnect` establishes structural completion only. Its success or `Idle` result does not establish grant release or service readiness.
3. The marker executes through `step`, with the same permit/continuation ownership rules as other inputs.
4. `try_finish_recovery == true` and `recovered == true` mean both boundaries executed and required recovery work/output drained. They can become true before ACTIVE is requested. Confirmed ACTIVE and authority gates separately authorize serving.
5. Structural completion permits requesting REJOIN while full drain readiness is still false. Remaining recovery replies/writes cause the daemon to request and confirm CLIENTREPLAY. This removes the potential cycle of waiting for clientreplay drain before requesting the phase needed to drain it.

Lines 70-73 name the production loader, `from_loaded`, `try_finish_recovery`, `pump`, `drive_phase`, and their fixtures. The migration matches the inspected call boundaries: `persistence::open` loads Ready state, `from_loaded` uses `Engine::restore`, and restore initializes the recovery gate closed. The current `try_finish_recovery` still demonstrates why the semantic migration matters: structural success alone sets its readiness flag. This review accepts the specified migration, not the current caller implementation.

The phase rules preserve the legal creating/starting transition to ACTIVE. Replay retains RECONNECT → REJOIN → optional CLIENTREPLAY → ACTIVE. The already-confirmed ACTIVE-map case must still complete internal recovery before Serving. A nonempty gather cannot take the empty fast path. There is no contradictory API signature or circular readiness definition in the revision.

### Exact empty marker progress

Line 75 defines a truly empty case: initialized load and completed initial cleanup/store work, with no Open keys, controls, jobs or stray work.

| Call | Result | Gate/ownership |
|---|---|---|
| `finish_reconnect(&[])` | Empty effects/notices, `Idle`, no unused permit | Structural completion only; grants remain closed |
| `step(Some(RecoveryControlsDrained), None)` | `NeedsAdmission` | Engine owns the retained marker; grants remain closed |
| `step(None, Some(permit))` | Empty effects/notices, `Idle`, same unused permit | Marker executes once; grants release; adapter receives its permit back |

The last turn needs no WAL append, commit callback, client packet or Tick. The owner schedules it from `NeedsAdmission`. Supplying the reserved permit with the initial marker is also valid. If capacity is unavailable, the caller retains the obligation and remains unready. Duplicate marker submission cannot replace a retained continuation.

These rules fit `RecordPermit`, `EngineStep`, `StepState` and `StepError::Busy` in `crates/engine/src/durability.rs:86-146`. The general rule allowing one follow-on transaction does not conflict with this empty case: the empty case has no such work. Zero sessions with retained work use the normal control/job boundaries instead.

### E2 progress and committed-output check

The structural seal and grant gate have separate jobs. Restore closes grants immediately. Structural finish closes reconstruction and opens eligible control/job progress. Only the admitted marker releases grants.

- Lines 45-47 cover evaluation, session resume, reply caps, range sharing, cleanup and journal callbacks. They prohibit hidden grant-state/sequence changes and prebuilt deferred grant payloads. Range-only notifications retain identities and later use committed values. Revokes remove effective rights; TRUNC and acknowledgments cannot add rights or enlarge advertised ranges. FLUSH_ACK follows commit or durable duplicate proof.
- Lines 49-53 allow eligible retained jobs and their admitted completions before grant release or ACTIVE. RecoverSize still waits for issued writer rights and conflicting truncate work. Revoke acknowledgments remain runnable. A dirty writer acknowledgment can commit its metadata and supersede the old guarded probe in that same record. Existing retirement/intent fields at `engine/mod.rs:1073-1095` support that dependency.
- A dirty control without issued writer rights can remain Busy outside the engine while the daemon runs the needed revoke/truncate/probe/completion work. `engine/step.rs:114-126` supplies the existing rejection shape; `engine/durable.rs:641-670,725-776` supplies eligibility and guarded completion. After the completion commits, the retained control can retry. It does not need a grant to make this progress.
- Line 57 requires the daemon's entire ordered control cut, including protected input outside the engine, to complete or undergo fenced durable cleanup. Ordinary requests do not block this cut. Grant-only deferred evaluation is not runnable work, so it cannot spin or starve controls.
- Lines 59-62 preserve pending-commit/continuation exclusion, one-record admission, effect materialization before another step, duplicate handling and terminal failure behavior.

No circular dependency is required by these rules for the specified RecoverSize/dirty-control cases. Failed storage/probe/fencing work correctly remains a wait, not invented completion. Implementation must carry the phase eligibility through both engine job queries and daemon scheduling. In particular, the current `Server::ready_jobs` readiness guard must be migrated consistently with lines 49 and 71; keeping a `recovered`-only guard would recreate the deadlock the contract forbids.

### D6 acceptance and separation

The revised D6 policy retains the timer rules checked in the initial review. It adds the map-default source citation. The linked decision now correctly states configured `2*R`, with 90 seconds as the default hard limit.

ACTIVE uses map S/A, strict timeout parsing, no ordinary STALE for timeout sessions, and fenced expiry. Reconnect uses confirmed-RECONNECT `t0`, validated missing-member activity, the bounded deadline, and durable missing-key cleanup. Timeout metadata adds no extra Ganesha recovery hold. Partial resets and post-final transport replacement retain the specified identity/control protections.

D6 does not need an ordinary creator reply policy. Activity eligibility can be validated when input is received; it does not depend on choosing the reply for a changed path. Fenced missing-session cleanup and retained job/control progress use the accepted E2/lifecycle boundary. An ACTIVE map during load still does not bypass the serving-readiness rule. No D6 timer or transport event can stand in for control drain or delete a protected input as if it had committed.

### Precise integration dependency

The separation is safe for implementation because E2 supplies phase/commit/grant ordering, D6 supplies timer/retention decisions, and section 3.1 supplies a history-only safe completion. None requires choosing a replacement-target or missing-parent ordinary response. The E2 engine package can be implemented and reviewed through its control, lifecycle and marker interfaces while the E1 case policy is unresolved.

Two integration obligations remain explicit:

1. **E2 engine ↔ daemon callers:** Integrating the changed structural-finish postcondition requires the named R3 migration. Keeping the old caller interpretation is not an accepted consumer of E2.
2. **Recovery service readiness ↔ ordinary E1:** After grant release, required committed retries still need peer-valid responses and ordered output drain before full readiness. The marker, a timer expiry, or wire-REPLAY handling cannot count an unresolved ordinary retry as completed. This join blocks full E1/nonempty recovery qualification, not the independent E2/D6 source package.

Section 3.3 still leaves replacement-target, missing-parent-plus-missing-original, and missing recorded target-identity policy unresolved. The separate independent case-proof assignment owns those questions. This scoped review neither closes R2 nor accepts the ordinary matrix by implication.

## Initial review — historical verdict and findings

The initial CHANGES REQUIRED verdict below is superseded for R1/R3 and the E2/D6 scope by the scoped acceptance above. Its ordinary E1 proof concerns remain part of the open R2 history.

**CHANGES REQUIRED.** These findings concern the proposed contract, not an implementation acceptance.

| Rank | Finding | Exact contract location | Classification |
|---|---|---|---|
| R1 / P1 | A genuine REPLAY must not receive the proposed reconstructed trace. | `design/recovery-contract.md` section 3, lines 68-79; section 5 case 3, line 136 | Protocol blocker |
| R2 / P2 | The blanket empty `-EINTR` fallback lacks the required complete peer-safety proof. | Section 3, lines 74-89; linked decision at `design/minimum-feature-set.md:566` | Compatibility-proof blocker |
| R3 / P2 | The changed `finish_reconnect` postcondition needs explicit migration of production bootstrap/clientless callers. | Section 2, lines 31, 35-39 and 55-62 | Integration-specification blocker |

The E2 control-drain design is otherwise coherent. The D6 timer policy has no blocking conflict with the checked sources or decision 6. These scoped conclusions do not accept the complete addendum.

### Initial source scope

Line references identify the files inspected during this review. C++ paths below are relative to `/home/ubuntu/ceph/src/`. Kernel paths are relative to `/mnt/workspace/ceph-client/`. Kernel source supplies peer behavior only. No kernel code was translated.

The original requirements are `design/persistence-contract.md` sections 2-4 and 7-8, `current/sol-recovery-review.md` sections 2-6, and `current/astra-g3-integration.md` E1/E2/D6. G2 application acceptance remains recorded in `current/astra-g2-evidence.md`. This report supplies source evidence, not installed-peer or mounted-client results.

### R1 — P1: separate wire REPLAY from an unacknowledged retry

Section 3 line 68 applies the same successful traced-reply table to ordinary retries and REPLAY. This contradicts libcephfs.

- `client/Client.cc:2852-2855`, `Client::send_request`, sets REPLAY when `request->got_unsafe` is true. `:3555-3575` separately resends old requests that have no unsafe reply. A lost safe-only reply does not itself cause REPLAY.
- `Client::handle_client_reply`, `:3060-3062`, calls `insert_trace` before handling safe completion.
- `Client::insert_trace`, `:1781-1785`, asserts that the trace is empty when `got_unsafe` is true. Either successful trace row in the proposed table violates that assertion on a genuine REPLAY.
- The kernel agrees on the distinction. `fs/ceph/mds_client.c:3540-3574` sets REPLAY after `GOT_UNSAFE`. At `:4117-4138`, the subsequent safe reply completes the request without reconsidering its result. The comment explicitly rejects changing the return value already delivered to the caller.
- C++ `mds/Server.cc:2657-2684` has a separate completed-REPLAY shortcut. It sends the safe completion without rebuilding a target/dentry trace. The lookup reconstruction at `:2686-2692` belongs to the other branch.

Safe-only Rust service does not normally produce the unsafe acknowledgment that causes genuine REPLAY. That makes the phrase “REPLAY-flagged lost-safe-reply paths” inaccurate. It does not make the proposed traced REPLAY response peer-compatible.

**Minimal resolution:** Split the two branches after the durable key/tid/op match. A matching committed REPLAY returns a safe completion with the recorded result and an empty target/dentry trace. It must not reopen the inode, grant new caps, or replace success with a changed-path error. Keep opcode-valid extra encoding consistent with the recorded creation. Apply creator trace reconstruction only to the ordinary retry that has no prior reply. Preserve the original wire op in both branches. Split section 5 case 3 into these two peer states.

No new request type or history image is needed for this correction.

The other ordinary-retry success rules are appropriate target constraints: match the recorded op before history trim; use exactly the recorded inode at the requested parent/name; accept an unchanged requested hard link or a rename of an ancestor; never substitute a new primary link. CREATE extra must identify that same inode. MKDIR/MKNOD/SYMLINK must not carry CREATE extra. Reconstruction must preserve the original completion and must wait for any new writer-range commit. These rules address identity and at-most-once execution without rerunning creator effects.

### R2 — P2: complete the empty-error proof and state its observable semantics

The source supports a narrower conclusion than the blanket policy in lines 74-89.

#### What the peer sources establish

For an ordinary request with no request dentry, or with an attached request dentry, the inspected negative-reply path returns an application-visible error. It is not a special “mutation succeeded, reconstruction interrupted” protocol result.

- `Client::make_request`, `client/Client.cc:2378-2403`, reads the wire result, verifies the trace only for nonnegative results, and returns the error. `_create` exits on the error at `:16214-16217`; `_mknod` and `_mkdir` return it at `:16064-16069` and `:16316-16322`.
- The kernel returns the reply result through `ceph_mdsc_wait_request`, `fs/ceph/mds_client.c:3985-3987`. `ceph_atomic_open`, `fs/ceph/file.c:957-979`, exits on the error. `ceph_mknod`, `fs/ceph/dir.c:977-988`, skips successful-create processing and returns the error. These inspected Ceph paths do not automatically repeat a mutation on `-EINTR`.
- C++ `Client::verify_reply_trace`, `:2215-2220`, can itself return `-EINTR` after a successful mutation reply resolves to a different created inode. Thus a committed mutation followed by an application-visible error is not inherently impossible. This local check does not prove every proposed direct server-error case.

The required at-most-once distinction is:

1. Keep durable `op/result/created_ino` unchanged. A recorded result of 0 still records a committed success.
2. The ordinary retry may report a separate reconstruction error only under an explicitly accepted error policy. The client sees `EINTR`, not the distinction in item 1.
3. Matching the same session incarnation/tid before trimming can prevent a second execution of that wire request. It cannot prevent an application or runtime from issuing another operation after `EINTR`. A new libcephfs request gets a new tid at `Client.cc:2260-2262`. If the name is now absent, that new CREATE can create another inode. An application-level retry loop is therefore outside the same-tid guarantee.
4. A safe reply already received unregisters the request (`Client.cc:3101-3112`; kernel `mds_client.c:4119-4121`). An ordinary lost-safe retry has no such acknowledgment. A genuine REPLAY has an earlier unsafe result; R1 must preserve that result. Neither case permits changing acknowledged namespace history or rewriting the completion as failed.

#### The missing safety proof

The addendum relies on `make_request` skipping success verification. That function is not the first consumer of the reply. `handle_client_reply` first calls `insert_trace` for negative replies too. Its empty-trace path reads `request->dentry()->dir->parent_inode` at `Client.cc:1788-1795`, before testing the result at `:1797`.

For the detached request-dentry state discussed by the addendum's own source basis, an empty negative reply does not bypass that dereference. A request's `DentryRef` preserves the object lifetime (`client/MetaRequest.cc:59-64`); it does not by itself preserve the directory link. `client/Dentry.h:71-77` clears that link in `detach`. The rename insertion path can detach an old dentry (`Client.cc:1323-1328`, `:3871-3879`).

This does **not** prove that every moved server inode leaves a detached client request dentry. Those are different states. It does show that the cited negative-result check is insufficient to certify all changed/missing-parent cases. This review has not established a reachability exclusion for that client state across every supported retry race.

**Minimal resolution:** Scope the proposed error policy to unacknowledged, non-REPLAY retries. State the actual application error and the per-session/tid limit above. Supply the missing source argument that supported fallback cases retain an attached request dentry, or resolve the uncovered case with a peer-compatible response policy. Correct the source rationale to include `insert_trace` before `make_request`. Keep the universal fallback pending until that proof is complete. Passing `check_invariants_for` is not a substitute, and a new meaning for `EINTR` is not a protocol fix.

### R3 — P2: name and migrate the production callers

The core two-boundary rule is clear. The fresh-bootstrap exception and migration obligation are not explicit enough for the unchanged signature.

Current production facts:

- `crates/mds/src/persistence.rs:410-448` loads the Ready checkpoint even after publishing a fresh bootstrap.
- `Server::from_loaded`, `crates/mds/src/server/runtime.rs:207-209`, calls `Engine::restore`. Restore sets recovery readiness false (`crates/engine/src/engine/durable.rs:115-120`). Production fresh startup therefore does not simply keep a serving `Engine::new` instance.
- `try_finish_recovery`, `runtime.rs:890-906`, calls `finish_reconnect(&[])` and sets `r.recovered = true` immediately on success.
- `daemon.rs:547-575` uses that result for CREATING/STARTING, RECONNECT, and an already-confirmed ACTIVE map after clientless load. `runtime.rs:642-647` uses `r.recovered` for normal admission.

With the proposed postcondition, these callers still compile, but `finish_reconnect` success means only Control drain. Leaving their readiness interpretation unchanged can request ACTIVE with the grant gate closed and leave later requests Busy. Line 31's bootstrap shorthand must not be read as an exemption for this restore path.

**Minimal resolution:** Add one mandatory caller-migration paragraph. Every production checked restore, including fresh Ready bootstrap and clientless restart, must perform structural finish, execute the marker through the permit/continuation path, and drain the required recovery work/output before reporting service readiness. Name `from_loaded`, `try_finish_recovery`, `drive_phase`, and their fixtures. Track structural completion separately from grant release and service readiness. An empty set of controls still executes the marker. Clarify that any serving-initialization exception applies only to the direct bootstrap constructor, not a subsequent restore of its checkpoint. Keep the existing legal monitor transitions for creating and replay startup.

The acceptance cases must explicitly include fresh production bootstrap, empty clientless restart, and load completion after an ACTIVE map. The unit marker and existing `EngineStep` remain sufficient; a new notice is not required.

### E2 checks with no additional specification blocker

Sections 2 and 5 otherwise address the concrete early-flush hazard:

- **One-step release:** Busy returns the exact marker and permit. No-permit acceptance retains one continuation. The caller resumes it with `step(None, permit)`. The marker submits no record; follow-on work can consume at most one permit. Duplicate finish/marker rules do not repeat evaluation or reopen reconstruction. `Idle` is explicitly not proof of release.
- **Control cut:** Line 55 includes retained inputs outside the engine, early controls, attachments, cleanup, and jobs needed to complete those controls. Queue deletion or reset is not completion. Held ordinary requests do not obstruct the cut.
- **Committed output:** Lines 43-45 cover cleanup, session resume, reply caps, range notifications, and commit callbacks. Deferred identities replace prebuilt grants. This matters because `engine/mod.rs:1218-1226` runs deferred output/evaluation at commit, and `:1293-1296` sends max-size notifications. The rule also retains range-only notification when the cap mask does not change. FLUSH_ACK still follows commit.
- **RecoverSize progress:** The blocked dirty input remains outside the engine. Eligible revoke acknowledgments, truncate work, probes, and their admitted completion can pass it. This matches the existing Busy shape in `engine/step.rs:114-126` and the control rotation in `server/runtime.rs:674-693`. Grant release or ACTIVE is not a prerequisite for the required job execution.
- **Probe supersession:** `engine/mod.rs:1073-1095` already records old-job retirement and a new guarded job in the writer's flush transaction. It keeps layouts and raises the bound to at least the old bound/current client range. `engine/durable.rs:641-670,725-776` supplies writer/truncate eligibility, guarded result application, and committed job retirement. Section 2 uses these fields without weakening the guards. A failed probe remains pending.
- **Runnable work:** Grant-only deferred evaluation cannot keep requesting permits. Revokes, controls, and job completions remain runnable. The daemon must wake on their completion. New stray purge remains held under the stated phase rules.

These are sufficient target obligations, subject to R3. They do not establish that the future grant gate or daemon scheduler implements them.

### D6 checks: no blocking finding

The timer formulas follow the selected policy. The map defaults come from `mds/MDSMap.h:605-606`: session timeout 60 seconds and autoclose 300 seconds. `common/options/mds.yaml.in:436-444` defines the separate reconnect default of 45 seconds. Existing daemon file-config precedence is `[mds.<id>]`, `[mds]`, `[global]` (`crates/mds/src/config.rs:64-74`). The proposed positive-finite check, representable `2*R`/deadline arithmetic, and per-attempt snapshot are sufficient configuration rules. Reading and validating the option remains a target implementation obligation.

| ACTIVE metadata, S=60/A=300 | Required behavior |
|---|---|
| No timeout key | STALE at 60; autoclose at 300 from the last renewal for an already Stale session |
| Zero or malformed value | No ordinary STALE; fenced eviction at 300 |
| P=30 | Fenced eviction at 60 |
| P=120 | Fenced eviction at 120 |
| P=300 or P=600 | Fenced eviction at 300 |

`Server::find_idle_sessions`, `mds/Server.cc:1239-1294`, checks the ordinary timeout first, then map autoclose, then the override. `:1310-1325` measures stale autoclose from the last renewal, not STALE entry. The addendum's `max(S,min(A,P))` and `max(S,A)` formulas reflect that order. Strict decimal parsing, malformed-as-zero treatment, and the diagnostic are explicit Rust choices. They preserve timeout-session treatment and do not make zero immune to autoclose.

Open-session activity uses the later valid renewal/traffic time. Stale-session autoclose uses the stated renewal basis. Healthy ACTIVE gating, received-traffic accounting, generation validation, and fencing prevent a timer from authorizing unsafe rights removal. The touched minimum-feature-set section 4.2 at line 179 agrees with these rules.

For reconnect, `Server::reconnect_tick`, `Server.cc:1870-1890`, supplies the base interval and recent-activity half interval. The proposed hard bound is explicit Rust policy:

- No activity: deadline `t0 + 45 s`.
- Last eligible activity at `t0 + 44 s`: deadline `t0 + 66.5 s`.
- Continuous eligible activity: deadline stops at `t0 + 90 s`.

Confirmed RECONNECT starts the clock. Partial/reset/replacement traffic cannot reset `t0` or the budget. Completed-member duplicates do not extend it. Expiry freezes missing keys and begins fenced durable cleanup; it does not release rights. These rules preserve decision 6 (`design/minimum-feature-set.md:557`): no extra Ganesha/timeout-session reclaim hold. The C++ extra hold at `Server.cc:1900-1909` is deliberately excluded by that accepted decision.

### Initial nonblocking improvements and re-review boundary

- In the linked decision at `design/minimum-feature-set.md:566`, describe 90 seconds as the **default** hard input-window limit. Section 4 defines the configured limit as `2*R`.
- Add the `MDSMap.h` default citation beside the 60/300-second values.

These are clarity improvements. R1-R3 are the required changes. The corrected review must cover the ordinary-retry/REPLAY split, the complete negative-reply path and application-error semantics, and the named production caller migration. Keep contract acceptance separate from implementation review and later sole-QA evidence.
