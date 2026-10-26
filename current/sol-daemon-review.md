# sol-daemon-review
Task: Independently review the frozen combined D6-R1 correction and exact six-site strict-CI cleanup. Rerun the original queued-reply reproducer. Return SCOPED ACCEPT D6 including R1/lint or concrete findings. Preserve prior acceptances and the nonempty D5 hold.
Status: SCOPED ACCEPT D6, including D6-R1 and the exact six-site strict-CI cleanup. D6-R1 is closed. The unchanged original independent reproducer passes. Independent results: 26 D6 tests, 15 session-output tests and one full 160-test daemon run pass. Strict daemon clippy, production build, owned formatting and whitespace checks pass without warnings. All 132 source paths/modes/hashes stayed stable. Prior acceptances and the nonempty D5 hold remain in force.
Context: Only this source-tree ledger is writable. Offline checks and stdin probes use target-sol-daemon-review. Primary E2, the separately accepted monc alias and store test fixture are frozen; E1 remains isolated. No production/test edits, live operations, Git writes, commits or subagents. Full G2 fault accounting remains separately owned.
Next: Coordinator can use the combined scoped acceptance and the tested release manifest in target-sol-daemon-review/r1-strict-independent-20261025.uf2nihyo/. Nonempty D5 integration and live G3 qualification remain separate assignments.
Log:
- [2026-10-25] Completed combined re-review: SCOPED ACCEPT D6 including R1/lint. Exact original stdin reproducer passes with zero old replies on the replacement. Focused 26 D6/15 session-output and full 160 daemon tests pass, 0 failed/ignored. Strict daemon clippy/build are warning-free; 20-file owned rustfmt and scoped whitespace pass. Independently matched the 132-path primary tree, modes and hashes to the combined archive before/after checks. Verified the exact three-file lint delta and retained R1 evidence inventories. Details and release provenance follow below.
- [2026-10-25] Started combined R1/strict-CI re-review. Author reports 160 unchanged tests and warning-free strict daemon clippy/build. Combined evidence: /tmp/opencode/sol-daemon-r1-strict-ci-20261025.aKufTD/. Original R1 red evidence: /tmp/opencode/sol-daemon-d6-r1-20261025.D7tKz0/. Independent checks are pending.
- [2026-10-25] Completed independent D6 review: CHANGES REQUIRED, P2 D6-R1. Build and daemon-only clippy pass with existing six production/four test dead-code groups. All 20 daemon Rust sources plus proto reconnect pass scoped rustfmt; scoped whitespace passes. All 129 author source inputs and 132 reviewer path/mode/hash entries remain unchanged. The appended D6 report contains the exact failure, correction requirement, commands and release provenance. No source correction, live action, Git write or subagent ran.
- [2026-10-25] Verified all 129 author input hashes and the 3fc3c84314b2727b5ffe6b642ed06b6e9add5000636cbd15c1472c5222e0c115 fingerprint against primary source. Saved a 132-path mode/hash manifest under target-sol-daemon-review/d6-independent-20261025.bjbe9khn/. Independent g3_d6_ result: 17 passed. Four stdin probes: 3 passed / 1 failed. The failure delivers an old queued RESET success to a replacement requester connection/context after durable OPEN. Self-reset, committed UUID change/Closed tombstone and synchronous STALE output-failure cuts pass.
- [2026-10-25] Started independent frozen D6 review. Read the author handoff and daemon D6 README. Evidence directory: /tmp/opencode/sol-daemon-d6-g2-fault-20261025.9LwREj/. Source review, manifest verification and independent tests are pending.
- [2026-10-25] Completed frozen correction re-review: ACCEPT. C1-C3 regressions pass against stable E2. Production build, daemon-only clippy and scoped format/diff checks pass. Verified required-job and terminal-output readiness through production APIs. Recorded exact results and probe artifacts below.
- [2026-10-25] Independently replayed the original C1-C3 branches from stdin with current E2 fixtures: four intended regression failures. The old structural-readiness postcondition also failed its exact check. A delayed-permit single-guard control passed the existing test; a stronger pressure/checkpoint-state probe failed that control and passed corrected source. Three final additional probes pass. Initial new-incarnation probe setup needed its retained RecoverSize completion; those setup failures are recorded separately.
- [2026-10-25] Started frozen correction re-review. Read the prior report, latest author handoff, daemon README and independent engine E2 ACCEPT. The author reports 44 G3, 11 G2 and 126 full daemon passes; fresh reviewer checks are pending.
- [2026-10-25] Completed source review and independent checks. Recorded three source-path blockers separately from concurrent E2 fixture failures. Only this report was edited.
- [2026-10-25] Started the D1-D4 review. Read the existing review ledger, HANDOVER, author handoff, daemon README, integration assignment, recovery checklist and coordinator scope.
- [2026-10-22] Started focused G2 correction re-review. Read updated author ledger and the original G2-R1 through G2-R4 findings.
- [2026-10-22] Started G2 runtime review. Read author ledger, crates/mds/README.md, final persistence contract, and prior accepted fencing report.
- [2026-10-22] Read HANDOVER, author ledger, and recovery boundary review. Created review ledger.
- [2026-10-22] Read repository guide, build brief, G1 scope, client invariants, and relevant C++ session handling. Reviewed the three assigned diffs against f54d298. Ignored rados.rs diff.
- [2026-10-22] CARGO_TARGET_DIR=target-sol-daemon-review cargo test -p rmds: PASS, 35 passed, 0 failed, 0 ignored. Five dead-code warnings. No live operations, production edits, commits, or subagents.

## Verdict: CHANGES REQUIRED

### 1. P1 — Invalid cap opcodes can still mutate metadata

Location: `crates/mds/src/server.rs:685-695` (`cap_update_from_proto`, lines 453-489).

The new admission check validates the xattr blob only. It does not validate the cap opcode. `proto::CapOp` accepts any u32. A server-only opcode such as GRANT, or an unknown opcode, reaches `Engine::handle_cap_update`. The engine treats every opcode except FLUSHSNAP as an ordinary update. With a matching cap id and sequence, this can change wanted/issued state, apply dirty metadata, append a record, and produce a flush acknowledgment. This does not meet this package's malformed-cap-before-mutation rule. This is an unclosed admission gap, not a regression introduced by the Result conversion.

Required change: Check supported client cap opcodes before calling the engine. Use the accepted rank fail-stop for invalid opcodes. Do not mutate cap, inode, journal, or completed-flush state first.

Minimal regression: Start with a real granted cap. Send a server-only opcode and an unknown opcode with valid ids, dirty metadata, and well-formed xattrs. Assert fatal state, unchanged inode/cap/journal state, and no output. Keep legal client UPDATE/FLUSH/DROP/RELEASE/RENEW/FLUSHSNAP behavior covered.

### 2. P2 — Undecodable cap packets bypass the new fail-stop

Location: `crates/mds/src/daemon.rs:458-465`.

The raw-message decode error branch calls `protocol_error` only for CLIENT_SESSION. A truncated CLIENT_CAPS or CLIENT_CAPRELEASE packet is logged and dropped. The rank remains active. The server-level malformed-xattr test does not cover this path because it passes an already decoded MClientCaps. A dropped flush can remain unanswered while later requests continue. The advertised malformed-cap policy therefore depends on where decoding fails.

Required change: Route malformed cap/cap-release decode failures through the same accepted fail-stop policy. Do not apply this blanket policy to metrics, which must remain tolerant.

Minimal regression: Deliver truncated raw CLIENT_CAPS and CLIENT_CAPRELEASE messages through daemon dispatch. Assert fatal state, no metadata changes, and no later client output/admission. Keep malformed metrics tolerance separate.

### 3. P2 — The session timer can send after fatal validation failure

Location: `crates/mds/src/server.rs:953-987`, especially `send_effects` at lines 968 and 986.

The completion, engine-tick, and recall sections return after `send_effects` sets `fatal_error`. The session timer loop does not. Session close/stale processing can unblock parked requests and return reply effects. If validation rejects one, the loop can continue to another expired session and directly send STALE at line 984. It can also close another session and release its state. The daemon's outer fatal check runs only after the entire tick returns. The send_effects guard cannot protect direct timer sends.

Required change: Return from tick immediately after either timer `send_effects` call sets fatal state. Ensure later timer iterations cannot mutate sessions or send messages.

Minimal regression: Use at least two expired sessions. Inject an invalid reply in the effects released by the first timer transition. Assert no send or session transition occurs after the fatal boundary, including STALE for the next session. Make timer ordering deterministic in the test.

## Reviewed behavior that is acceptable

- Pending contexts are inserted before engine admission. The 4096-context limit stops before engine mutation. Duplicate in-flight tids do not overwrite the first context. A completed retry obtains a new context.
- The compact path preserves exactly the component-count and parent checks used by the current validator. The copied request head preserves READDIR max_entries. The retained context omits data, releases, and group payloads.
- Reply dispatch removes one context and validates op, tid, parent, READDIR limits, and request-independent invariants before Session::send. A missing context for an existing session fails closed. Effects for an absent session are dropped.
- Close/kill clears contexts. The engine suppresses old deferred replies by session epoch, including after the same client id opens again. No separate daemon generation type is required for this initial package.
- Malformed initial create xattrs return EINVAL before engine admission. Malformed decoded cap xattrs stop before engine mutation.
- Sessionless renewals produce no REJECT. Duplicate OPEN notifications require NOTIFY_SESSION_STATE, apply only to Open state, and carry the existing push sequence. No push-sequence reset was found.
- REQUEST_FLUSH_MDLOG invokes the current Journal::flush and consumes only reported commits. It sends no synthetic acknowledgment. MemJournal is not durable.
- The per-session send funnel remains intact in the changed send paths. The outer daemon event loop checks fatal state after each event. The timer gap above is the remaining within-event propagation issue.

## Evidence limits

The three findings follow source paths. Their proposed regressions were not added or executed because this review owns only this ledger. Existing 35 tests pass and do not cover these paths. No durability, complete G1, G2, G3, or live-client claim is made. Full fencing, durable sessions, reconnect orchestration, and rados.rs review remain outside this assignment.

## Re-review: original corrections ACCEPT; new package CHANGES REQUIRED

### Fresh evidence

- [2026-10-22] Read updated current/sol-daemon.md and inspected new fencing, health/admission, session queue, barrier, and cleanup paths. Read relevant Objecter APIs without editing them. Checked C++ Server::handle_client_session and MDSRank::evict_client.
- `CARGO_TARGET_DIR=target-sol-daemon-review cargo test -p rmds`: PASS, 51 passed, 0 failed, 0 ignored. Three dead-code warnings. This includes all original-finding regressions.
- `CARGO_TARGET_DIR=target-sol-daemon-review cargo build -p rmds`: PASS. Five dead-code warnings.
- Concurrent engine dependencies rebuilt successfully. No compile blocker occurred. These commands exercised the current shared working tree, not isolated immutable dependency revisions. No regression red run is claimed.
- No source edits, live operations, commits, or subagents. New regressions below are requirements, not tests executed by this reviewer.

### Original corrections: ACCEPT

1. `server.rs:801-826` validates client cap opcodes before dirty metadata or barrier handling. The regression uses matching cap fields and well-formed dirty xattrs. Legal client opcode coverage also passes.
2. `daemon.rs:853-863` fail-stops malformed raw cap/session/cap-release packets. The raw-dispatch regression passes and preserves metrics tolerance.
3. `server.rs:1126-1176` stops timer work at the first fatal boundary. The deterministic two-session regression passes. Autoclose now requests fencing instead of directly killing the session.

### N1. P1 — A blocklisted connection can reopen immediately after eviction

Location: `crates/mds/src/daemon.rs:782-795`, `:866-889`; `crates/mds/src/session.rs:262-293`.

Cleanup removes the runtime session but does not close or reject the old connection. A later REQUEST_OPEN from that same connection reaches SessionMap and creates a new generation. Neither dispatch nor OPEN checks the current OSDMap for the connection's address. The blocklist scan runs only on Objecter events. Without another map event, the reopened session remains admitted and can obtain caps, locks, and metadata mutations. The new generation protects against a stale worker deleting it, but does not authorize this blocklisted client to return. Held old OPEN messages can trigger the same path after health recovery.

Reference: `/home/ubuntu/ceph/src/mds/Server.cc:728-744` rejects OPEN when the current map blocklists the client address. It sends REJECT with the blocklisted flag and the compatible `blocklisted (blacklisted)` reason.

Required change: Check current blocklist evidence before opening or rebinding a client connection. Reject a blocklisted address without installing an engine/runtime session. Preserve the send funnel for that rejection. Do not depend on a future MapApplied event to evict the client again. A new valid messenger identity/address must follow the intended clean-recovery path.

Minimal regression: Complete an eviction with the address still blocklisted. Reuse the old connection to send OPEN, then a mutation, without any intervening map event. Assert no new session, cap, lock, or mutation. Repeat with an old OPEN in held input. Check the blocklisted rejection flag/reason and a permitted non-blocklisted address separately.

### N2. P1 — CLOSE disappears when its funnel has queued output

Location: `crates/mds/src/session.rs:335-338`, `:93-114`, `:370-374`.

REQUEST_CLOSE calls s.send(CLOSE), then immediately removes the session. If output is paused or any older output remains queued, send only appends CLOSE to the queue. Removal drops the queue, including CLOSE. The client never receives its close response and can wait indefinitely during unmount. This path is reachable after health recovery: output drains only 32 messages per turn, while held CLOSE input can be released with output still pending. on_mdsmap also releases held input before the outer loop updates the paused-output state.

Required change: Retain the close response and its ordered funnel until transmission completes, or use an equivalent bounded terminal-output state. Keep admission closed while it drains. Do not bypass earlier output. A transmission/enqueue failure must remain visible after runtime session removal; removing the session must not erase output_failed.

Minimal regression: Open a session, pause output, and queue a non-counted message. Accept CLOSE at the current push sequence. Resume/drain and assert the older message then exactly one CLOSE reach the link. Repeat with an unpaused but nonempty backlog. Inject terminal send failure and verify the daemon cannot resume serving by deleting the failed session.

### N3. P2 — A queued duplicate OPEN advertises an obsolete push sequence

Location: `crates/mds/src/session.rs:252-258`, `:97-114`, `:119-127`.

The notification OPEN captures s.push_seq when constructed. Counted messages already in the queue have not yet incremented that field. They transmit before OPEN and increment push_seq, but OPEN retains the earlier encoded sequence. For example, queue one CAPS while paused at sequence 0, enqueue a duplicate OPEN notification, then drain. The wire order is CAPS followed by OPEN(seq=0), although the funnel is now at sequence 1. The existing duplicate-OPEN test covers only immediate transmission; the paused-funnel test covers only renewals, whose sequence is a client-provided echo.

Required change: Bind an OPEN notification's sequence to its actual ordered position. For example, retain a typed pending notification until transmission, or account for preceding queued counted messages without changing the accepted-transmission counter. Do not rewrite RENEWCAPS echo sequences.

Minimal regression: Queue counted messages, then duplicate OPEN with NOTIFY_SESSION_STATE. Drain in multiple turns. Assert OPEN.seq equals the number of counted messages before it, including any previously transmitted baseline. Add another counted message after OPEN and assert it is excluded from that OPEN sequence.

### Accepted new mechanisms and explicit boundaries

- Workers do not block daemon dispatch. Command, version-query, status-query, and map waits have timeouts. Completion processing validates membership, operation, session generation, address, and Fencing state before cleanup.
- Failed fencing retains runtime rights and retries. A command failure can succeed only through fresh applied-map blocklist evidence. Command success alone cannot authorize cleanup. Production latest_epoch rejects zero or a version below applied/advertised maps. fence checks successful wait progress, then address presence. drive_control rechecks current presence before cleanup.
- Rank/incarnation reassignment exits rather than reusing old authority. Fencing sessions reject OPEN, renewals, CLOSE, and ordinary requests while present. The post-removal admission gap is N1.
- Objecter CheckLatestMap keeps a single active query and retries failure. The Objecter command consumes its covered tid boundary and emits a new query for uncovered operations. No source path was found that strands those uncovered operations through successful coalescing.
- Autoclose and external blocklists enter the verified eviction chain. Cleanup publishes barriers before engine close. Self-address blocklist events stop service. The prototype's immediate cleanup is not a durable cleanup receipt and is explicitly not claimed as one.
- Held input, paused output, held legacy data work, sessions/fences, channels, and live workers have explicit limits. Overflow fails closed. Active messenger outbound backlog still lacks byte/capacity feedback; the author correctly does not claim an end-to-end outbound memory bound.
- Output failure is checked between effects and before outer queue draining. Fatal state prevents held input/output release. N2 identifies a terminal-session removal that can erase queued output/failure state before those checks.
- New cap output includes the current server barrier. Incoming cap/release barriers raise the monotonic server/Objecter barrier before engine handling. The existing barrier regression passes.
- Beacon/map lag pauses ordinary admission, timer execution, output, and legacy work scheduling. Compatibility poll_commits can still resume previously admitted engine work. The author explicitly lists replacement with EngineStep/journal_event as pending integration. This review does not turn that declared boundary, durable session/storage work, or G3 reconstruction into additional package findings.

## Final focused re-review: ACCEPT

- [2026-10-22] Read the updated author ledger. Inspected N1-N3 corrections and their dispatch, retirement, failure, and sequence boundaries in daemon.rs, server.rs, and session.rs. No remaining blocker found in this correction package.
- N1 closed: `daemon.rs:865-880` checks the incoming link against the currently applied blocklist before runtime installation or rebinding. Held traffic uses this same dispatch. Rejection uses a non-admitted bounded funnel. A blocked old link cannot replace or evict a different permitted address's runtime session. Reopen-without-map-event, held OPEN, rejection flag/reason, blocked mutation, and permitted-address tests pass.
- N2 closed: `session.rs:430-462` transfers queued CLOSE output to a terminal owner. `:295-329` drains terminal output in order within the shared turn budget. `:371-377` blocks same-client OPEN until that owner retires. Terminal capacity and bytes/count budgets fail closed. `:139-147`, `:331-333`, and `:490-503` make failure sticky across runtime retirement and explicit capacity discard. Dispatch stops before engine lifecycle hooks when terminal output fails. Tests cover paused and unpaused backlog, exactly one CLOSE, reopen ordering, immediate/queued send failure, rejection ownership, capacity failure, and daemon failure after runtime removal.
- N3 closed: `session.rs:117-130` recognizes every OPEN sent through the common raw funnel. `:171-189` assigns its sequence at transmission, after earlier accepted pushes and before later pushes. Partial-drain regression includes a transmitted baseline and later counted output. Renewal echo remains unchanged. Tests pass.
- Delivery boundary remains explicit: accepted means ClientLink::send_raw returned true. It does not prove peer receipt or durable session close. No retry or duplicate terminal send was introduced at this boundary. Active messenger backlog remains outside the stated bounded queues.

Fresh independent commands:

- `CARGO_TARGET_DIR=target-sol-daemon-review cargo test -p rmds`: PASS, 59 passed, 0 failed, 0 ignored. One existing Config.cluster dead-code warning.
- `CARGO_TARGET_DIR=target-sol-daemon-review cargo build -p rmds`: PASS. Three dead-code warnings: Config.cluster, Session::push_seq, SessionMap::iter_mut.

The tests rebuilt the current engine dependency successfully. No concurrent compile blocker occurred. This is working-tree evidence, not an immutable dependency snapshot. Author-reported red runs were not independently repeated. No source edits, live operations, commits, or subagents. Durable session/cleanup and engine/store integration remain the explicit next-package boundaries, not blockers for this acceptance.

## G2 runtime/protocol/engine-execution review: CHANGES REQUIRED

Scope: the new daemon runtime, journal adapter, dispatch, session lifecycle, phase policy, and lifecycle-worker scheduling. Accepted engine/store implementations were read only to establish API semantics. This review does not duplicate the parallel audit of storage authority, allocator, timeout, or checkpoint internals.

### G2-R1. P1 — A new connection address can use a session before durable attachment

Location: `crates/mds/src/daemon.rs:1275-1308`, especially `:1303-1308`; attachment policy at `crates/mds/src/server/runtime.rs:836-878` and `:942-947`.

Dispatch checks whether the incoming address is blocklisted. It does not check whether that address is the admitted session's address, or whether its attachment has committed. For non-session messages it checks only client id and `is_open()`. It then discards the incoming link and calls the engine under the existing SessionKey.

Thus an authenticated connection B with the same client global id can send requests or cap updates while the admitted/durable session still identifies address A. B need not complete OPEN. B's writer address never enters the persisted address union. Later eviction fences A and the stored union, but not B, before it releases rights and schedules recovery/purge. Merely rejecting already-blocklisted B does not close this gap. The final contract requires changed identity/address metadata to commit before dependent admission (section 7).

Required change: Bind all session-dependent input to an admitted transport/address. If the incoming connection needs attachment, keep its input outside the engine until durable attachment completes. Reject/drop unmatched input if this package does not support that path. Do not silently route it under A's key. Retain the accepted same-address transport-reset behavior.

Minimal regression: Durably open A. Deliver a mutation and a dirty cap packet from non-blocklisted B with the same global id, without completing attachment. Assert no engine input, record, rights change, or reply on A. Delay B's attachment and repeat. If attachment succeeds, assert B is in the durable fencing union before admission. Evict and verify both addresses must be fenced before cleanup.

### G2-R2. P1 — Normal CLOSE never revalidates its sequence after admitted work drains

Location: `crates/mds/src/server/runtime.rs:880-897`, `:514-527`; `crates/mds/src/session.rs:266-273`.

`begin_close` compares the request sequence with the transmitted push counter immediately, before the runtime drains admitted cap/release work and engine continuations. `Closing` does not retain that request sequence. When Drain becomes empty, pump submits `CommandKind::Closing` without checking the counter again.

An already-admitted cap flush can commit after `begin_close` and emit a counted FLUSH_ACK. Other admitted work can also emit counted cap output. The runtime then persists Closing and removes rights using a CLOSE sequence that no longer covers its output. Paused counted output ahead of CLOSE has the same problem. This bypasses the contract's drain -> validate current push_seq -> persist Closing order. Once the wire session is Closing, a replacement CLOSE is ignored by `begin_close`, so there is no later validation path.

Required change: Preserve the close request's sequence through Drain. Validate it at the actual close boundary, after relevant admitted work/output is accounted for. If new pushes invalidate it, wait for a matching close request or complete verified fencing. Do not publish Closing/cleanup based only on the initial comparison.

Minimal regression: Delay a record, queue a cap input that will emit a counted acknowledgment, then submit CLOSE at the current sequence. Complete/drain the cap input. Assert no rank Closing write or rights release from the obsolete sequence. Supply a current CLOSE and verify the full durable cleanup/Closed/CLOSE chain. Also cover counted paused output; retain the accepted non-counted terminal-output test.

### G2-R3. P2 — Pending OPENs do not reserve a live-session slot

Location: `crates/mds/src/daemon.rs:1293-1301`; `crates/mds/src/server/runtime.rs:831-878`, `:664-700`.

The dispatch limit counts only installed runtime sessions. Async new OPENs have no installed session until their completion. The separate opening limit also permits 1024 entries. While completions install sessions, new OPENs can refill the opening set. Once installed sessions reach MAX_SESSIONS, previously admitted completions still install without a limit check. The rank can therefore exceed its stated 1024-session bound (up to 2047 with these limits). The fencing table retains its independent 1024 limit.

Required change: Reserve a live-session slot before a new durable OPEN command. Count installed and pending-new sessions together. A duplicate attachment must not consume another slot. Keep the reservation through completion/failure. Do not solve this by dropping a successfully persisted Open row after the write.

Minimal regression: Hold OPEN completions, admit up to the live-session budget, then interleave completions and further new OPENs. Assert installed plus reserved-new sessions never exceeds MAX_SESSIONS, and overflow is handled before the repository mutation. Cover duplicate attachment and slot release after a completed close.

### G2-R4. P2 — One Busy control input suppresses an already-runnable completion

Location: `crates/mds/src/server/runtime.rs:392-405`, `:578-589`, `:962-968`.

A Busy result requeues that input at the back, sets one rank-wide `blocked` flag, and stops pump. `has_work` then returns false even when a different control input is already queued and can make progress. This is reachable with a dirty cap update waiting behind an active RecoverSize job: the accepted engine explicitly returns Busy for that input. When the job completion arrives, the queue can become `[blocked cap update, LifecycleCompleted]`. Pump rotates the cap input to the back, then declares the whole runtime blocked. The completion now at the front does not get an immediate scheduling turn. Progress waits for an unrelated input or the one-second Tick to clear the flag.

Required change: Distinguish a blocked input from a blocked runtime. Make a bounded pass over other eligible control inputs, or otherwise keep an immediate wakeup when an executable completion remains. Avoid spinning when every input waits on the same dependency.

Minimal regression: Arrange the RecoverSize/dirty-update state, enqueue the successful lifecycle completion behind the Busy update, and run the normal pump/has_work scheduler without Tick or another client packet. Assert the completion transaction starts and commits, then the deferred cap input progresses. Also assert the all-Busy case does not spin.

### Verified integration properties

- Production is `Engine<DaemonJournal>`. The old request/cap/release/tick/close helpers are restricted to `Server<MemJournal>` test implementations. Requests, cap inputs, releases, close cleanup, Tick, and lifecycle completions use EngineInput/RecordPermit/EngineStep. Configuration setters and checked session-key installation do not submit records.
- The adapter reserves one record with the same body limit used by EngineConfig. It holds one queued/in-flight append. Submission encodes without I/O. Explicit completion advances the contiguous prefix before JournalEvent. EngineStep output is materialized before a fresh admitted step. No production one-second journal polling remains.
- Storage waits run in a worker, not the daemon dispatcher. Storage completions match owner and operation; daemon delivery also checks membership. Lifecycle callbacks match owner/job/attempt and enter the engine as queued completion input. Failure suppresses later replies/work. G2-R4 is a scheduling defect, not a missing journal completion event.
- New OPEN waits for the repository completion before key installation, OPEN output, and request admission. Cleanup reaches Closed only after the engine's terminal cleanup receipt. Recovery validates Open/Closing/Closed joins and does not reinstall Closed tombstones. G2-R1 and R2 are the remaining high-level admission/close gaps.
- Creating/start/replay no longer construct a new empty serving namespace as a fallback. Phase requests use loaded readiness and legal monitor transitions. The clientless path retains rejoin and the internal clientreplay hook. A nonempty restored gather remains held, as declared for this G2 package.
- Prior N1-N3 mechanics remain: dispatch-time blocklist rejection, retained terminal funnels, sticky output failure, and OPEN sequence assignment at transmission. Request-specific deferred reply validation remains before Session::send. Session closure now needs the additional G2-R2 drain-time validation.
- README and configuration identify native persistence as the production path. They state finite timeout/budget defaults, generation exhaustion, no checkpoint GC, and the active-messenger backlog limit. No MemJournal durability default or live qualification claim was found.

### Fresh evidence and oracle limits

- `CARGO_TARGET_DIR=target-sol-daemon-review cargo test -p rmds`: PASS, **71 passed, 0 failed, 0 ignored**. Four dead-code warnings.
- `CARGO_TARGET_DIR=target-sol-daemon-review cargo build -p rmds`: PASS. Six dead-code warnings.
- `CARGO_TARGET_DIR=target-sol-daemon-review cargo clippy -p rmds --all-targets --no-deps`: PASS. Existing dead-code warnings only. It briefly waited for this review's build-directory lock. No dependency compile failure occurred.
- The persistence tests run real store APIs against a separate atomic compound-operation memory backend. The backend applies comparisons before publishing its private object image and can lose an append reply after commit. This is useful offline integration evidence. It is not native RADOS evidence. Several older protocol regressions still use MemJournal or fabricated asynchronous completions; they do not independently validate every new production-runtime boundary.
- The new findings follow source-level execution paths. Their regressions were not added or run because only this report is writable. No red-run claim is made. Shared engine/store dependencies rebuilt successfully; evidence applies to the current working tree, not an isolated immutable revision.
- No source edits, live operations, commits, or subagents. No full G1/G2 live gate or G3 mounted-client claim. Detailed storage internals remain the parallel reviewer's scope.

## Focused G2 correction re-review: ACCEPT

- [2026-10-22] Read the latest author ledger. Inspected daemon.rs, server.rs, server/runtime.rs, session.rs, and server/runtime/tests.rs at the four reported boundaries. No remaining blocker found in this runtime correction package.
- **G2-R1 closed.** `daemon.rs:1291-1307` gates dependent raw input through `is_attached`. `server/runtime.rs:418-445` requires committed attachment, the active address/nonce, matching credentials, and the recorded runtime generation. Pending attachment blocks dependent input. `:776-824` checks the captured generation before publishing Open. A mismatched owned completion fails closed and cancels pending state. Tests cover unattached B, delayed B attachment, changed credentials, a same-address reset, stale generation, no premature mutation/output, and the durable A+B fencing union. The daemon regression requires both addresses in the applied blocklist before cleanup. The same-address messenger reconnect contract retains its connection handle; this correction does not introduce a new durable address for it.
- **G2-R2 closed.** `server/runtime.rs:590-624` rechecks the retained request sequence after drain and before the Closing command. `Session::close_ready` also rejects queued counted output. WaitSeq retains rights and the durable Open row. `:1016-1045` accepts a current retry; verified fencing removes the sequence requirement. The runtime pauses engine execution while the validated Closing write is pending. The existing terminal cleanup receipt -> rank Closed -> final CLOSE chain remains. Tests cover delayed counted FLUSH_ACK, obsolete retry, current retry, paused counted output, non-counted terminal FIFO, and verified fencing from WaitSeq.
- **G2-R3 closed.** `server/runtime.rs:95-114`, `:938-955`, and `:999-1014` account for live keys plus pending-new operations before the repository command. Duplicate pending OPEN and attachment do not allocate another slot. Completion converts the reserved operation to a live key. Closed removes the live key. Unrelated owner/op completions cannot consume the current reservation. Owned failure, cancellation, and stale-generation fail-stop clear pending reservations and byte accounting. The interleaving regression checks the limit at every completion and reuses one slot after close.
- **G2-R4 closed.** `server/runtime.rs:676-693` rotates Busy controls through a bounded pass that persists across turns. Only a complete all-Busy pass parks execution. Input, completion, or execution progress resets that pass. `:704-713` transfers a last-turn submission to the worker before returning, so an outbox-only write cannot lose its wakeup. The regression uses a real RecoverSize intent and 37 Busy updates. One-step has_work/pump turns execute the lifecycle completion without Tick or new client input. It also checks the all-Busy stop condition, input byte retention, eventual job retirement, and deferred control drain.
- The correction retains owner/operation matching and one-record admission. It does not change the storage worker, journal adapter, authority acquisition, or accepted storage contract. Full G3 reconstruction remains held. Active messenger backlog still lacks capacity feedback. Neither limit is represented as implemented or live-qualified.

Fresh independent verification, all with `CARGO_TARGET_DIR=target-sol-daemon-review`:

- `cargo test -p rmds --locked g2_r -- --nocapture`: PASS, **11 passed**, 0 failed/ignored, 71 filtered out.
- `cargo test -p rmds --locked`: PASS, **82 passed**, 0 failed/ignored.
- `cargo build -p rmds --locked`: PASS.

Tests report four existing dead-code warnings. The production build reports six warning groups. No dependency compile blocker occurred. Clippy was not rerun in this focused review; the author's current clippy result remains author evidence. The regression oracle uses the production runtime/engine with explicit delayed command completions; it is not native RADOS evidence. The five author-reported pre-fix reds were not independently repeated. No source edits, live operations, commits, or subagents. Acceptance covers runtime code review and the offline checks above, not the native G2 or mounted-client gates.

## D1-D4 collection-only G3 review: CHANGES REQUIRED

Reviewed on 2026-10-25. `git rev-parse HEAD` returned `a6ae05a10bfefdcea8dae75cd29597c84a3fadcd`. Scope is the working daemon diff, both new recovery source/test files, daemon README, and the mixed-chunk comment in `crates/proto/src/reconnect.rs`. The proto diff changes no decoder. The persistence.rs diff adds only a test. Accepted storage authority, journal and fencing implementations remain the dependencies of this review. Concurrent engine E2 is not reviewed here.

Nonempty finalization is deliberately held. That limit is not a finding. The three findings below concern collection, terminal routing and fail-stop behavior that this package claims to implement.

### G3-C1. P2 — Late reconnect outside a nonempty recovery has no consumer

Locations: `crates/mds/src/daemon.rs:864-871`, `:908-927`, `:1449-1460`; downstream `:1610-1648` and `crates/mds/src/server/runtime.rs:1306-1309`.

`recovery_input_allowed` requires `Server::recovery_pending`, which requires an initially nonempty Open gather. Otherwise `on_client_raw` places every reconnect in `reconnect_held` whenever rank membership exists. That includes healthy confirmed ACTIVE after a clientless restart. Only the nonempty recovery branch of `release_held` consumes this queue. The serving branch consumes `held`, not `reconnect_held`.

Thus a reconnect for a retained Closed row, or an unknown session, can remain unanswered indefinitely after an all-Closed load. This includes a client that missed terminal output before restart. Repeated packets can exhaust the queue and fail-stop an otherwise serving rank. Bypassing that queue alone is insufficient: ordinary dispatch drops sessionless reconnects, and the normal runtime handler ignores attached reconnects. The new CLOSE/REJECT policy exists only inside `stage_recovery`.

Required change: Restrict the raw reconnect hold to an actual startup/map-confirmation or health wait. Add lifecycle-aware terminal routing once checked state establishes that reconnect collection is not pending. Return ordered CLOSE for Closed/late reconnect and the defined sessionless response for unknown reconnect. Preserve silent sessionless renewal and the real requested-versus-confirmed race.

Minimal regression: Use the daemon receive path, not only `dispatch_client_raw`. Load only Closed rows and reach healthy confirmed ACTIVE. Send a reconnect for a Closed key and one for an unknown client. Assert bounded ordered terminal responses, no engine/session installation, and no permanently retained reconnect input. Repeat for a reconnect queued before load completes. Retain the nonempty requested-RECONNECT/map-race test.

This is an unimplemented D2 boundary. The old G2 package intentionally held reconnects; its earlier acceptance does not establish the new terminal behavior.

### G3-C2. P2 — A first valid final chunk does not extend the remaining gather's deadline

Location: `crates/mds/src/server/runtime/recovery.rs:1098-1110`; activity filter at `:951-960`.

`apply_reconnect` calls `note_recovery_activity` only when `progress.complete` is false. A first valid final chunk instead removes its key from `gather` without recording its receive time. This loses valid missing-member activity for all other missing keys. Calling the helper after removal would also fail its `gather.contains` test.

Reproduction: Restore two Open keys A and B. Confirm RECONNECT at t0 with R=45 s. A supplies its first valid final chunk at t0+44 s. B remains missing. The accepted activity rule requires a deadline of t0+66.5 s. The implementation leaves t0+45 s and starts B's eviction at that earlier time. The explicit exclusion for completed-member duplicate finals does not exclude A's first final. See `design/recovery-contract.md:151-159`; C++ `Server::handle_client_reconnect` also updates activity before removing a final member (`/home/ubuntu/ceph/src/mds/Server.cc:1757-1766`).

Required change: Record validated activity for the first final while its key is still missing. Preserve no-extension behavior for duplicate finals, malformed packets and old generations. Keep the 2R hard bound.

Minimal regression: Use two restored keys and a controlled receive clock. Finalize A at 44 s, assert the 66.5 s deadline, then accept B before that deadline. Cover v3 and an empty v5 final after partial reconstruction. A duplicate A final must not extend the deadline again.

### G3-C3. P2 — Renewal output failure can fall through into another engine/storage turn

Locations: `crates/mds/src/server/runtime/recovery.rs:933-945`; `crates/mds/src/server/runtime.rs:580-607`, `:608-624`, `:709-750`.

The recovery renewal path ignores `Session::send`'s return value. Failed transmission or queue exhaustion sets the sticky rank output-failure flag, but `recovery_turn` continues and returns success. After `drive_recovery`, `pump` does not recheck failure before flushing commands or executing engine controls. It checks only `fatal_error` between steps and checks `sessions.output_failed` at its final exit.

With a queued renewal and another key's durable Closing ready for engine cleanup, a failed RENEWCAPS send can therefore be followed by `CloseSession`, a new journal submission and worker transfer within the same pump call. The outer daemon failure check runs too late. Bounded staged input and delayed callbacks can place these obligations in the same turn. Later sends have their own sticky guard, but that does not stop the new engine/storage work.

Required change: Propagate renewal-send failure out of recovery processing. Recheck fatal/output failure immediately after `drive_recovery` and before another engine/worker turn. Cancel pending admission through the existing fail-stop path. Preserve the durable rights/receipt rules; failure is not cleanup authority.

Minimal regression: In a production runtime fixture, retain a renewal on an attached key and a second key's verified, durable Closing cleanup obligation. Make the renewal link fail, then run the owning pump. Assert sticky failure, no subsequent engine step, no new journal submission/worker command, and no later output. Also cover paused-output capacity failure. The existing successful-renewal and explicit GRANT-rejection tests do not cover this boundary.

### Reviewed properties and required next-package migration

- Separate control, collection, output, data and ordinary permissions use current membership and loaded owner identity. They check beacon health, self blocklisting and failure. Requested ACTIVE alone does not admit ordinary traffic. Cold standby/standby-replay are excluded from storage startup. Worker completion envelopes retain owner/operation checks.
- Recovery attachment uses the existing durable Open command. Recovery completion preserves SessionKey and histories, installs Recovering silently, and sends no OPEN. A stale attachment result retains its committed address union without installing its funnel. Credentials, required CephFS bits and current attachment authorization are rechecked. Reset removes only incomplete reconstruction; final rights survive transport loss. A replacement cannot start a second stream under a different token.
- All lock blobs are decoded before one `reconnect_chunk` call. The adapter supplies authenticated lock client IDs. Caps/locks replace per inode. Retained applied counts/bytes remain charged after raw release. Realm corrections derive target realms and child relationships from authoritative engine APIs and use the session's snap encoding and ordered send funnel.
- `Runtime::gather` tracks missing keys. `Recovery::members` retains surviving Open keys, including completed members, until Closed. `Recovery::nonempty` is sticky. `try_finish_recovery` excludes that load even after every member closes; it cannot accidentally invoke a changed engine `finish_reconnect` for nonempty recovery. No legacy reconnect-done bypass was found.
- Missing-key fencing waits for pending identity writes, freezes the persisted union, verifies every address in the applied map and raises a nonzero barrier. Matching owner/key/attachment checks protect the proof. Closing, engine cleanup receipt and Closed retain their existing order. Gather removal on eviction remains at tagged Closed completion. Failure retains rights and membership. The deadline has an explicit event-loop wakeup and no timeout-session extra hold; G3-C2 corrects its activity accounting.
- The G2 close sequence, pending-new reservation and Busy-control mechanisms remain in place. This review does not turn the concurrent engine API change into a regression verdict on their accepted implementation.
- **Mandatory migration remains explicit:** README lines 200-222 and the author handoff name `from_loaded`, `try_finish_recovery`, `pump`, `drive_phase`, `ready_jobs` and fixtures. The E2 consumer must distinguish structural completion, marker execution, drain readiness and confirmed service. It must pass all surviving keys, execute the protected control/job drain and release marker, then drain output before ordinary admission. The clientless/fresh/already-ACTIVE callers must migrate too. Their current `r.recovered = true` after structural finish is the old G2 postcondition, not valid E2 integration. Keep the nonempty barrier until that coordinated package is reviewed.

### Fresh independent checks and oracle limits

All Cargo commands used `CARGO_TARGET_DIR=target-sol-daemon-review`.

| Command | Actual result |
| --- | --- |
| `cargo test -p rmds --locked g3_ -- --nocapture` | Exit 101: **8 passed, 26 failed, 0 ignored**, 82 filtered out |
| `cargo test -p rmds --locked g2_r -- --nocapture` | Exit 101: **7 passed, 4 failed, 0 ignored**, 105 filtered out |
| `cargo build -p rmds --locked` | PASS |
| `cargo clippy -p rmds --all-targets --no-deps --locked` | PASS, existing dead-code warnings |
| `rustfmt --check --edition 2024 --config skip_children=true` on the nine touched daemon Rust files | PASS |
| `git diff --check a6ae05a -- crates/mds crates/proto/src/reconnect.rs` | PASS |

There was no compile error. Twenty-five G3 failures and all four G2 failures stop at `server/runtime/tests.rs:254`, where the pre-restart create helper finds no reply. The remaining G3 failure stops at the analogous mkdir helper in `server/runtime/tests/recovery.rs:1024`. These failures precede the target reconstruction assertions. The active E2 handoff in `current/sol-engine.md:9` changes `finish_reconnect` to structural completion only. The daemon and these fixtures still omit the release marker. This matches the required caller migration above. No unfinished engine internals were audited or changed, and no workaround was applied. A coherent dependency cut is required before claiming a fresh full daemon pass.

The eight passing G3 tests include five daemon phase/fence/permission tests, timeout configuration, required-feature/authentication checks, and the actual store-worker attachment test. The latter independently reads the persisted address union/context before delivering Open completion. It does not assert that its setup mkdir received a reply, so its current pass proves the attachment boundary, not namespace/history setup under E2. Most other regressions use the production DaemonJournal/runtime with explicit delayed completions, not native RADOS. Some checks inject an EngineStep to test an output boundary; they do not establish engine-generated recovery ordering.

The three new findings are source-path reproductions. No new tests were written or executed for them because this assignment permits edits only to this report. Author-reported pre-fix reds and 116/34/11 passing counts were not independently reproduced at an immutable dependency revision. The shared engine changed during this review; successful later builds do not repair the recorded failed runs. Tests emitted four existing warning groups; production build/clippy emitted six. No full-suite, strict-workspace-lint, native, mounted-client or complete G3 pass is claimed. No production edits, live actions, commits or subagents ran.

## Frozen D1-D4 correction re-review: ACCEPT

Reviewed on 2026-10-25. **ACCEPT for D1-D4 collection and the clientless E2 production caller migration.** G3-C1, G3-C2 and G3-C3 are closed. No blocking finding remains in this scope. Engine E2 has its separate independent acceptance in `current/sol-engine-review.md`.

The author froze the correction package. Current HEAD is `85a37c8d615992b4600d7467fdedd514fecfac03`; the scope baseline remains `a6ae05a`. Source inspection includes the new recovery source/tests and the corrected production callers. The proto change remains comment-only. Production persistence code remains unchanged; its fixtures and scheduler now consume E2. This verdict preserves earlier G2 acceptances.

### C1 closed: checked terminal routing and current rights

- `crates/mds/src/daemon.rs:879-885,936-945,1475-1499` separates reconnect terminal permission from nonempty collection permission. Checked clientless/all-Closed loads consume both immediate and previously held reconnects. Startup, map and health waits remain bounded. Cold unassigned followers have no terminal authority. Requested RECONNECT does not authorize chunk application before confirmation.
- `daemon.rs:1649-1656` routes decoded reconnect before the ordinary attachment/session filters. `server/runtime.rs:1283-1339` sends ordered CLOSE for a known late/Closed client and sessionless REJECT for an unknown client. `session.rs:312-345` reuses the matching connection funnel or retains a bounded non-admitted funnel. It preserves older output and sticky capacity failure. No orphan reconnect queue remains on the reviewed clientless receive path.
- A current live attachment enters `WaitFence`. An obsolete normal CLOSE in `Drain`/`WaitSeq` can join that fence. The packet itself releases no rights and submits no cleanup. `runtime.rs:1232-1273` accepts only the matching Fencing generation/address. The daemon still checks current membership, full persisted address-union blocklisting and the applied epoch before this call (`daemon.rs:1333-1359`). Closing, committed cleanup and tagged Closed precede terminal CLOSE (`runtime.rs:982-1013`; `session.rs:362-379`). Repeated late packets do not queue another eviction.
- Authenticated loopback regressions reach `on_msgr_event` and `on_client_raw`. They cover v3/v5, known Closed/unknown clients, immediate/pre-load packets, FIFO output and the separate nonempty early-map race. The runtime regression covers active current attachments and invalidated normal CLOSE sequences. An additional independent probe verifies that obsolete connections cannot rebind or evict the current attachment. After durable close and a new incarnation, an old fence and old v3/v5 reconnects leave the fresh key, generation and cap intact.

### C2 closed: first-final activity before removal

`crates/mds/src/server/runtime/recovery.rs:1092-1106` records validated activity before removing the first final's key from `gather`. Duplicate finals return before this update (`:980-994`). The activity filter still requires the current attached token, missing membership and an unexpired window (`:953-963`). The deadline remains bounded by `min(t0+2R, max(t0+R, L+R/2))` (`:96-100`).

Both correction tests pass. A first v3 final, and an empty v5 final after a partial chunk, move the remaining member's deadline from 45 s to 66.5 s at receive time 44 s. A completed-member duplicate does not extend it. The remaining member can complete at 65 s. Further eligible activity reaches, but does not exceed, the 90 s hard bound. Existing stale-generation, malformed-chunk and pending-attachment deadline regressions also pass in the 44-test G3 run.

### C3 closed: synchronous failure before the next turn

`crates/mds/src/server/runtime/recovery.rs:933-940` returns an error on renewal-send failure. `drive_recovery` immediately fails the journal/engine and cancels pending admission (`:736-750`). `server/runtime.rs:645-660` rechecks fatal/output state before taking the runtime, transferring commands or executing another engine step. The step loop and final transfer retain the same checks (`:828-845`).

The regression passes for both immediate transmission failure and a full 1024-message paused funnel. Another key already has durable Closing and runnable cleanup. Failure sets sticky fatal state in that pump call. It causes no later engine submission, worker transfer, cleanup receipt or output. The other key remains present. Failure does not manufacture durable Closed or authorize rights release. Already-issued storage operations retain their existing fail-stop/reload outcome rules; a late callback cannot reopen this runtime.

### Clientless E2 migration accepted

- **Every checked load starts closed.** `server/runtime.rs:352-375` initializes `RecoveryDrain::Reconstruction` and `recovered=false`. This includes Ready bootstrap and a load after an ACTIVE map. The migrated runtime, persistence and daemon fixtures use this production path. No fixture sets readiness directly.
- **Structural completion is separate.** `runtime.rs:1047-1069` calls `finish_reconnect(&[])` only for initialized clientless state after Closing cleanup. Success records `Controls` and consumes its step result. It does not report readiness. `recovery_structural_done` exposes this boundary to the daemon.
- **The release marker needs fresh admission.** `runtime.rs:423-434,750-827` retains the `RecoveryControlsDrained` obligation in `Controls`. This caller selects the contract's initially-reserved-permit form. It never submits the marker without a permit, so it cannot resubmit an engine-owned permitless marker. Earlier engine continuations resume with `input=None`. Marker execution records `GrantsReleased`; `consume_step` returns its unused permit to the adapter (`:585-588`). The exact empty case needs no WAL append, callback, Tick or checkpoint.
- **Unavailable admission remains a wait.** A held adapter permit leaves the obligation pending and makes `has_work` false (`:525-535,764-768`). Returning that permit makes the marker runnable. The additional probe repeats this wait eight times and checks both `checkpoint_wanted=false` and `pressure=false`. One fresh turn then releases the marker. Eight later readiness/pump calls stay idle, preserve the sequence and transfer no command. A new reservation succeeds.
- **Zero sessions do not skip required work.** Marker readiness requires a drained daemon cut, no live keys, engine quiescence and no eligible non-Purge lifecycle intent. `ready_jobs` uses structural completion rather than `recovered` (`:1340-1348`). The accepted engine exposes retained Truncate/RecoverSize dependencies during this phase. Pending completion commits prevent quiescence. The clientless size-job regression runs and commits RecoverSize before marker release. Independent background Purge does not add a new hold. New post-marker engine work still drains before readiness.
- **Output acceptance is a separate boundary.** `runtime.rs:435-446` requires executed release, drained work and zero queued output without failure. The independent terminal-output probe executes the marker while output is paused. Readiness remains false until the funnel accepts CLOSE. No commit or new engine step is needed to observe the drained output.
- **Monitor transitions stay legal.** `daemon.rs:580-655` uses structural completion to request REJOIN. It uses drain readiness for CLIENTREPLAY/ACTIVE and confirmed ACTIVE for service. CREATING/STARTING use their legal direct ACTIVE request only after both boundaries. An already-ACTIVE load still waits for both boundaries. Required data/output permissions are available during clientless drain (`:886-914`), and the loop rechecks phase readiness after output drain (`:319-328`). The monitor-phase test and all three startup-mode cases pass.
- **Nonempty finalization remains held.** The sticky `Recovery::nonempty` exclusion remains in both `try_finish_recovery` and `recovery_marker_ready`. Even durable closure of every original member cannot enter this clientless path. Collection sends no final OPEN or GRANT. D5 must separately join all surviving keys, protected early controls/retries, required jobs and ordered output. D6/reclaim, E1 and mounted-client G3 remain separate assignments.

### Independent checks on corrected source

All Cargo commands used `CARGO_TARGET_DIR=target-sol-daemon-review`.

| Command | Result |
| --- | --- |
| `cargo test -p rmds --locked g3_ -- --nocapture` | **44 passed**, 0 failed/ignored, 82 filtered out |
| `cargo test -p rmds --locked g2_r -- --nocapture` | **11 passed**, 0 failed/ignored, 115 filtered out |
| `cargo test -p rmds --locked recovery_phase_requests_wait_for_monitor_confirmation -- --nocapture` | **1 passed**, 0 failed/ignored, 125 filtered out |
| `cargo test -p rmds --locked persistence::tests -- --nocapture` | **10 passed**, 0 failed/ignored, 116 filtered out |
| `cargo build -p rmds --locked` | PASS |
| `cargo clippy -p rmds --all-targets --no-deps --locked` | PASS with existing dead-code warnings |
| `rustfmt --check --edition 2024 --config skip_children=true` on the nine daemon Rust paths and proto reconnect | PASS |
| `git diff --check a6ae05a -- crates/mds crates/proto/src/reconnect.rs` | PASS |

The persistence filter checks the migrated worker scheduler and Ready fixture. Its G3 attachment test overlaps the G3 filter; counts are not additive. That test now asserts its setup mkdir reply and committed history before restart (`persistence.rs:1289-1312`). Tests emit four existing dead-code groups; build/clippy emit six. No dependency compile failure occurred. The author's 126-test full-suite result remains author evidence; this review ran the relevant subsets above.

Three additional tests compiled from stdin against the same dependencies pass:

```sh
target-sol-daemon-review/review-g3-corrections-probe review_ --nocapture
```

They cover obsolete connections/fence generations, repeated delayed-permit ownership and pressure flags, and output acceptance after marker execution. Their production source came from the frozen files, expanded in memory. Only the test module was extended. The fixture include path was made absolute for stdin compilation. No source file was created or changed. Two initial C1 probe variants stopped at missing CREATE/MKDIR setup replies after fenced writer cleanup. Completing the retained RecoverSize job through production lifecycle admission and commit made the new-incarnation setup valid. The final probe retains the original C1 assertions. Those setup attempts are not C1 regression evidence.

### Original-behavior controls and retained artifacts

The author records genuine pre-fix assertions in `current/sol-daemon.md:8-13,86-99`. I independently checked their sensitivity by compiling the original problematic branches from stdin with the current E2 fixtures. These are controlled branch replays, not an archived immutable pre-fix checkout. Repository sources stayed frozen.

| Control executable under `target-sol-daemon-review/` | Actual result |
| --- | --- |
| `review-g3-original-branches` | Four exact tests each exited 101 at the intended boundary: C1 retained late v3 input; C2 first final left 45 s instead of 66.5 s; C2 later first final left 87.5 s instead of 90 s; C3 renewal failure left fatal state unset. |
| `review-e2-old-readiness` | Empty-marker regression exited 101 at `structural finish alone is not readiness`. |
| `review-e2-old-permit-pressure` | Removing only the busy-reservation guard did **not** fail the existing delayed-permit test. Its progress assertions still passed because the current pressure flag suppresses that checkpoint path. This does not independently reproduce the author's earlier exact delayed-permit red. |
| `review-e2-pressure-probe-control` | The stronger independent probe exited 101 at `a held marker permit is not a checkpoint request` with that same guard removed. Corrected source passes both pressure-flag and progress assertions. |

The C1 control restores the original raw hold/consumer and removes late decoded dispatch. C2 records activity only for incomplete chunks. C3 ignores renewal-send failure and removes the post-recovery/per-step failure rechecks. E2 old-readiness sets `recovered` at structural success. The delayed-permit controls remove only the busy-adapter distinction. All controls retain stable E2 and valid current fixtures. Thus the previous 8/34 G3 and 7/11 G2 fixture failures are not used as direct red proof.

Retained executable SHA256 values:

| Artifact | SHA256 |
| --- | --- |
| `review-g3-original-branches` | `819e29ed76c08ccfbd01ccb4b6b891f77499b4de92f29274f8c30d3463d7414f` |
| `review-e2-old-readiness` | `c3ac47bd003943e34cb638d5acf707b5231cf606654bca46d99176a87a739ade` |
| `review-e2-old-permit-pressure` | `88d2a43b05ff242d53253bc1fc79d802211c459aca015a76721eb362a048ef85` |
| `review-g3-corrections-probe` | `55b6f46bbc7e8f349ae09ea32fd64127527307fb4ae34d67917965521405bf0a` |
| `review-e2-pressure-probe-control` | `034c8cab3408ae0c6abef5e2f86dafd7c7e097447ff503d2cadf80f317ff1879` |

The 123-path source/manifest fingerprint stayed `c6d387007ad32c66674f1ad49c672b7f3515fe15e72b69082d0a4510df91877d` before and after the stdin checks. Its exact input set is `crates/*/src/**/*.rs`, `crates/*/Cargo.toml`, root `Cargo.toml` and `Cargo.lock`. Hash sorted relative-path bytes, NUL, file bytes, NUL with SHA256. This identifies those reviewed inputs; it is not a full repository or fixture archive.

Only this report and review build artifacts were written. Offline loopback and compound-operation memory-backend checks establish the scoped source/runtime result. No native, mounted-client, full D5/D6, E1 or complete G3 qualification is claimed. No live action, commit or subagent ran.

## Frozen D6 review: CHANGES REQUIRED

Reviewed on 2026-10-25 against `design/recovery-contract.md` section 4 and
`current/astra-g3-integration.md` D6/R14/R15. **D6 is not accepted.** One P2
requester-isolation finding remains. The earlier D1-D4/C1-C3/clientless E2 and G2
acceptances keep their original scopes. Nonempty D5 finalization remains held.

### D6-R1 — P2: queued reclaim success follows a replacement requester

**Locations:** `crates/mds/src/server/runtime/d6.rs:114-164`, especially the raw
`s.send` at lines 154-158; `crates/mds/src/session.rs:140-155,169-214,402-419,562-577`.
The durable OPEN completion calls `publish_open` at
`crates/mds/src/server/runtime.rs:990-997`.

The completion checks the requester's key, generation, connection, `open_id`,
credentials, lifecycle and current policy before calling `s.send`. That call can
enqueue the reply. The queue stores only raw reply bytes. It does not retain the
requester's ownership token. `finish_reclaims` then removes the target and its
waiters.

A later durable OPEN can change the requester's `open_id`, authorization context
and connection while that reply remains queued. `Session::handle(REQUEST_OPEN)`
replaces `s.link` and keeps the queue. `Session::transmit` sends the old reclaim
reply through the replacement link without checking its original owner.

This violates the D6 requirement that a changed requester cannot inherit an old
completion (`current/astra-g3-integration.md:131-134`). The reply carries no UUID
or request identifier. A replacement reclaim attempt can therefore receive an
unrelated success with the old target's addresses and fencing epoch.

**Independent reproduction on unchanged production source:**

1. Open target client 42 with UUID `old`. Open requester client 43.
2. Pause output and queue 40 ordinary renewal replies on requester 43. Submit
   RESET `old` from its current connection.
3. Complete the target's verified-fence entry point, Closing, actual engine
   cleanup receipt and tagged Closed. The reclaim success queues after the 40
   older outputs. No reclaim reply has reached either transport.
4. Resume output and drain the normal 32-message budget. Eight older messages
   and the reclaim reply remain queued. Complete a normal durable OPEN for
   requester 43 on connection 21 with a changed authorization context. The
   production dispatch and tagged Open callback perform this replacement.
5. Drain output. The old requester receives zero reclaim replies. The replacement
   receives `result=0, epoch=9, addrs=[127.0.0.1:4100/nonce10]`.

The probe asserts both the target's durable Closed/cleanup receipt and the
requester's changed `open_id`. It fails at:

```text
old reclaim completion must not migrate to a replacement attachment after enqueue
```

This interleaving does not require dispatch while output is paused. Step 4 resumes
output first. The daemon drains only 32 outputs before releasing held input
(`daemon.rs:327-330`). An OPEN released at that point can complete through
`on_storage_event` before the next output-drain turn.

**Required correction:** retain the reclaim completion's requester ownership
through the output queue. Check that ownership at the actual transport handoff.
An obsolete completion must not move to a replacement attachment. Preserve the
completed target cleanup and the existing ordered-output byte/count accounting.

**Required regression:** reproduce the sequence above through production dispatch,
durable OPEN completion and bounded output drain. Verify zero old reclaim replies
on the replacement. Cover a requester that enters Closing/Fencing while its reply
waits too. A current unchanged requester must still receive exactly one ordered
reply after Closed. Preserve push-sequence accounting and sticky output failure.

### Other reviewed boundaries

- **RESET/FINISH/authentication:** `runtime/d6.rs:237-338` implements the exact
  flags/error matrix. FINISH alone is silent. Invalid combinations and empty
  RESET UUID give EINVAL. Wrong authenticated target name gives EPERM without
  cleanup or activity refresh. Absent/Closed UUID gives result 0, epoch 0 and
  empty addresses. The common dispatch boundary and `is_attached` reject
  sessionless, obsolete-connection and changed-credential input.
- **UUID decisions:** `runtime/d6.rs:33-46` checks non-Closed rows and pending
  OPEN metadata. `runtime.rs:244-254` rejects duplicate live UUIDs at checked
  restore. OPEN retains both the old durable UUID and proposed UUID until its
  tagged result. Same-key updates remain legal. Closing retains the live
  reservation; Closed releases it. Existing failure handling stops admission
  before a cancelled/unknown OPEN can permit another claim.
- **Durable target cleanup:** pending target identity writes delay fence
  capture. Reclaim freezes target admission. The saved full address union,
  owner/key/attachment token and exact union match reach the existing daemon
  latest-map fence consumer. Failed, partial and stale proofs retain rights.
  A nonzero verified epoch precedes new fenced Closing/cleanup. An already
  Closing target joins its existing cleanup; success still waits for both
  fencing and Closed. `runtime.rs:1016-1038` requires the exact engine cleanup
  receipt before publishing Closed and trying reclaim completion. The real-worker
  D6 regression checks the retained Closed row and receipt on cold load.
- **Bounds and initial completion checks:** 128 aggregate waiters bound targets
  too. An unchanged requester duplicate reuses its waiter. The existing tests
  cover multiple waiters, target attachment updates, pre-completion requester
  disappearance/replacement, changed feature policy/blocklist, output overflow
  and stale completion. D6-R1 concerns the later queued-output interval.
- **Additional identity probes:** self-RESET completes exactly one cleanup and
  sends no reply to its now-Closed requester. A new incarnation cannot inherit
  that fence completion. After a committed UUID update, the previous UUID is
  absent. A Closed tombstone is absent too. These two independent probes pass.
- **ACTIVE timers:** `runtime/d6.rs:396-453` applies S first, A before metadata,
  then strict trimmed decimal u32 P. Zero/malformed metadata retains timeout
  treatment and stored text. S/A clamp to one second. Timeout sessions never
  enter ordinary STALE. Ordinary STALE preserves engine caps/locks; renewal
  resumes Open. Autoclose measures the retained activity clock, not STALE entry.
  The focused tests cover exact thresholds, signed/empty/overflow values,
  retained rights, renewal and stale fence generations.
- **Receive-time activity and health:** `note_held_activity` validates current
  attachment and message eligibility before updating the monotonic clock.
  `renew_at` and ordinary dispatch use `max(last_renew, received)`.
  `daemon.rs:1514-1533,1555-1586` accounts for held traffic before timer expiry.
  Healthy confirmed ACTIVE enables expiry; STOPPING retains its separate engine
  progress. Beacon loss blocks new expiry/cleanup/output. The real loopback test
  covers renewal behind 65 held messages, beyond one bounded dispatch turn.
- **Synchronous timer failure:** the additional STALE-send probe fills one
  requester's 1024-message funnel while another key has durable Closing and
  runnable cleanup. STALE output failure latches fatal state. A subsequent pump
  performs no append or worker transfer, creates no cleanup receipt and reports
  no runnable work. This probe passes.
- **Recovery separation:** final reconnect initializes activity. The existing
  45/90-second gather deadline remains separate. D6 adds no timeout-session hold.
  Both `try_finish_recovery` and `recovery_marker_ready` retain the sticky
  nonempty exclusion. No D5, E1 implementation or live G3 result follows.

### Independent verification and evidence limits

All Cargo commands used `CARGO_TARGET_DIR=target-sol-daemon-review`.

| Check | Actual result |
| --- | --- |
| `cargo test -p rmds --locked g3_d6_ -- --nocapture` | **17 passed**, 0 failed/ignored, 130 filtered out |
| Four additional stdin probes, `review_d6_ --nocapture` | **3 passed / 1 failed**, 0 ignored, 147 filtered out; D6-R1 fails |
| `cargo build -p rmds --locked` | PASS |
| `cargo clippy -p rmds --all-targets --no-deps --locked` | PASS; existing six production/four test dead-code groups |
| `rustfmt --check --edition 2024 --config skip_children=true` | PASS on all 20 daemon Rust sources and proto reconnect |
| Scoped `git diff --check` | PASS |
| Source path/mode/hash comparison before/after review | PASS |

The stdin build expands the frozen daemon sources in memory and adds only the
four probe functions. It uses the same compiled dependency libraries. The first
harness attempts failed to resolve nested/public/digit-named modules and ran no
tests. Correcting that in-memory expander produced the behavioral result above;
no production source changed.

The author evidence directory is exactly:

```text
/tmp/opencode/sol-daemon-d6-g2-fault-20261025.9LwREj/
```

Its provenance, before/after manifests, stability report and retained D6/build/
clippy/format logs were checked. Its logs report 61 G3, 11 preserved G2 and 147
full-daemon passes. Those broader counts remain author evidence. The eight
pre-fix reds are described in the author ledger; they were not independently
replayed by this review. The four new G2 fault tests and full fault accounting
remain with the separate auditor. This review ran the D6 filter, not those four
tests or another full suite.

### Release provenance and retained reproducer

Primary source root: `/mnt/workspace/rust-mds`.
Read-only HEAD observation: `85a37c8d615992b4600d7467fdedd514fecfac03`.
The reviewed package includes uncommitted/untracked source; HEAD alone does not
identify it.

All **129** entries in the author's before/after SHA256 manifests match primary
source. Their fingerprint remains:

```text
3fc3c84314b2727b5ffe6b642ed06b6e9add5000636cbd15c1472c5222e0c115
```

Fingerprint construction: sort relative paths, then hash each relative-path byte
string, NUL, file bytes, NUL with SHA256. Inputs are root Cargo files plus
`crates/*/Cargo.toml` and `crates/*/src/**/*.rs`. The earlier correction package's
123-path fingerprint is historical and does not identify D6.

Reviewer evidence directory:

```text
/mnt/workspace/rust-mds/target-sol-daemon-review/d6-independent-20261025.bjbe9khn/
```

`release-source-path-mode-sha256.txt` contains **132** relative source paths with
full regular-file mode (`100644`) and content SHA256. It includes all 129 Cargo
inputs plus the daemon README, recovery contract and integration matrix. Its
SHA256 is `0eedd83f3e9b14c65d7be6d7494ed9762d77db57dfb2194897a6e948677aff27`.
The initial permissions-only manifest is retained as
`source-path-mode-sha256.txt`. Every path, mode and content hash stayed unchanged.
These manifests identify the reviewed source; they do not authorize release of
the D6 package with D6-R1 open.

| Retained artifact | SHA256 |
| --- | --- |
| `review-d6-probes` | `6f4dc6369d690c6685270da3d965853fda8d5fb7f9f1722351a9e19edaae2f24` |
| `independent-probes.txt` | `442c78a7d1d7eb6dd36ab0c3b425b0ce1c787faaeb0c9299a54bfdd88f0f5303` |
| `independent-probes.log` | `9d791f6ae86bb5a7396f0f95c16b98f42108512678d126b75036360899993401` |

The directory also retains `commands.json`, `probe-build.log`,
`probe-input.sha256`, all independent check logs, the author-input manifest,
`source-stability.txt` and `artifacts.sha256`. Rerun only the exact finding with:

```sh
target-sol-daemon-review/d6-independent-20261025.bjbe9khn/review-d6-probes \
  --exact server::runtime::tests::d6::review_d6_queued_completion_must_not_follow_replacement_requester_link \
  --nocapture
```

Only this report and review artifacts were written. No production edit, live
operation, Git write, commit or subagent ran. Offline results establish the
reported source/runtime finding. No native or mounted-client G3 pass is claimed.

## Combined D6-R1 and strict-CI re-review: SCOPED ACCEPT D6

Reviewed on 2026-10-25. **SCOPED ACCEPT D6, including the R1 correction and the
six assigned strict-Clippy sites. D6-R1 is closed.** No blocking finding remains
in this scope. The original D6 source review above and this correction review
together establish this verdict. Earlier D1-D4/C1-C3/clientless E2/G2 acceptances
retain their scopes. Nonempty D5 finalization remains held.

### R1 closure: ownership reaches the actual transport handoff

`crates/mds/src/session.rs:32-91,192-212` stores every CLIENT_RECLAIM_REPLY as
`PendingOutput::Reclaim`. The owner contains client ID, runtime generation,
durable SessionKey/OPEN `open_id`, connection ID, active address, authenticated
name and authorization context. Both D6 response sites still use `Session::send`,
so existing-target success, absent-target success, EINVAL and EPERM all use this
common guard. The test-only compatibility caller uses the same funnel too.

`Session::transmit` checks ownership immediately before `ClientLink::send_raw`
(`session.rs:253-284`). The current state must be Open or Stale. The saved address
must match both the funnel and the current link. Every saved owner field must
match. The exclusive session task performs no await between this check and
transport handoff.

The durable tag is output ownership, not admission authority:

- The normal tagged Open result updates an existing funnel before its
  attachment-policy branches (`server/runtime.rs:942-949`). First successful
  publication sets the tag before later input can run (`:1000-1004`). A new
  committed decision invalidates old replies even when the new attachment is
  refused by blocklist/feature policy.
- Recovery Open completion sets the observed tag before obsolete/disconnected/
  evicting attachment checks and after installing a new recovery funnel
  (`server/runtime/recovery.rs:651-699`). Thus a committed replacement `open_id`
  cannot leave an old output owner current. Post-final Reset retains the funnel
  and rights until that checked attachment path updates it. Partial Reset removes
  only its incomplete funnel. The existing reset/replacement regressions pass in
  the full daemon run. Recovery attachment remains Recovering; the R1 guard does
  not authorize service or bypass D5.

`Transmission::{Sent,Discarded,Failed}` distinguishes stale discard from
transport acceptance. A queued discard pops one entry, subtracts its exact
message-byte charge and consumes one drain-budget turn (`session.rs:287-302`).
It does not increment `push_seq`. Later unrelated output remains ordered. An
immediate discard returns false without inventing a transport success or fatal
error. Capacity and actual transport failures still latch the rank's sticky
failure before another output can proceed. The failure check precedes ownership
validation, so obsolete output cannot mask an earlier failure.

The 15 session-output tests exercise each owner field, all excluded states,
exact byte/count release, reuse of the freed 1024-entry slot, rank byte pressure,
immediate discard, and immediate/queued transport refusal. The nine production
R1 cases also check queued Closing/Fencing, retired CLOSE funnels, valid Stale,
same-connection new OPEN, refused durable replacement, all immediate error
categories, unchanged positive FIFO/push sequencing and a valid new reclaim
behind an obsolete response. The old target's cleanup receipt and Closed row
remain intact. A stale discard cannot consume the new epoch-13 response.

### Original independent reproducer: PASS without changing the probe

I recompiled the exact retained probe from
`target-sol-daemon-review/d6-independent-20261025.bjbe9khn/independent-probes.txt`
against the combined source. Its SHA256 remains
`442c78a7d1d7eb6dd36ab0c3b425b0ce1c787faaeb0c9299a54bfdd88f0f5303`.
Production modules and the probe were combined in memory and passed to rustc
through stdin. No production or test file was edited.

The exact sequence still queues 40 older renewals, completes target Closing,
engine cleanup receipt and tagged Closed, drains 32 outputs, then uses ordinary
REQUEST_OPEN dispatch and tagged durable completion to replace the requester's
connection, `open_id` and authorization context. Draining the remainder now gives:

```text
old requester reclaim replies=0, replacement replies=[]
test result: ok. 1 passed; 0 failed; 0 ignored; 163 filtered out
```

The assertion that failed in the original review now passes unchanged. The
candidate's stronger production regression additionally verifies that the eight
older renewals and a new renewal still arrive, the byte budget reaches zero,
`push_seq` stays zero and the original cleanup receipt does not change.

Original author red evidence was checked at
`/tmp/opencode/sol-daemon-d6-r1-20261025.D7tKz0/`. `red-exact.log` contains the
same intended assertion failure; the original matrix reports 1 pass/5 failures.
Its recorded external monc/store edits prevent a stable whole-tree red claim.
This review preserves that qualification. The independently verified combined
green interval below has stable source identity. Both retained author artifact
inventories match every listed artifact hash.

### Exact six-site lint cleanup: ACCEPT

I compared the combined tree to the archived completed-R1 source. Only
`config.rs`, `rados.rs` and `session.rs` differ. The independently generated diff
matches `lint-only.patch` byte for byte. A second exact-content check proves:

| Site | Verified correction |
| --- | --- |
| `Config.cluster` | Removes only the unread field and its initializer. All parser, CLI, environment, config-search and keyring-expansion bytes remain identical. |
| `spawn_purge` | Removes only its uncalled fire-and-forget wrapper. |
| `spawn_truncate` | Removes only its uncalled fire-and-forget wrapper. |
| `spawn_recover` | Removes only its uncalled log-only wrapper. |
| `Session::push_seq` | Adds `#[cfg(test)]` to the getter; all callers are test-only. The production field, increments and validation remain. |
| `SessionMap::iter_mut` | Adds `#[cfg(test)]`; its session-map caller is the existing test-only timer regression. |

The pre-cleanup archive has only the declarations of the three spawn wrappers,
with no Cargo-source callers. The current workspace search finds no callers.
The accepted production route remains `Daemon::drive_jobs` -> awaited
`rados::execute_lifecycle` -> owner/operation/job-tagged completion ->
`Server::lifecycle_completed` -> admitted `EngineInput::LifecycleCompleted`
(`daemon.rs:663-713,1110-1140`; `rados.rs:251-344`). Errors retain the intent for
retry. Checked purge/truncate/probe primitives, budgets, completion handling and
their tests are byte-identical to completed R1. No functional path depended on
the removed wrappers' logging.

`Config` is a binary-internal type: `main.rs` has private `mod config`, and
`crates/mds/Cargo.toml` declares the daemon binary, not a public library target.
The parser's local `cluster` still defaults to `ceph`, accepts both long-option
forms, selects `/etc/ceph/{cluster}.conf` after CLI/CEPH_CONF precedence, and
expands configured/default keyrings (`config.rs:139-220`). Startup still logs
the same id, config path and monitor host (`main.rs:35-36`). There is no runtime
reader or log-dependent use of the removed stored field.

The lint delta adds no tests, removes no tests and changes no test bodies. It
adds no allowance, fabricated use or lint-gate change. The retained strict-red
log names exactly the six assigned production errors and four test-target
errors. The independent exact strict command now passes for both targets.

### Independent checks on the combined candidate

All Cargo commands used `CARGO_TARGET_DIR=target-sol-daemon-review`.

| Command/check | Actual result |
| --- | --- |
| `cargo test -p rmds --locked g3_d6_ -- --nocapture` | **26 passed**, 0 failed/ignored, 134 filtered out |
| `cargo test -p rmds --locked session::tests -- --nocapture` | **15 passed**, 0 failed/ignored, 145 filtered out |
| Exact original stdin R1 probe | **1 passed**, 0 failed/ignored, 163 filtered out |
| `cargo test -p rmds --locked --quiet` | **160 passed**, 0 failed/ignored |
| `cargo clippy -p rmds --all-targets --no-deps --locked -- -D warnings` | PASS, no warnings |
| `cargo build -p rmds --locked` | PASS, no warnings |
| `rustfmt --check --edition 2024 --config skip_children=true` on all 20 daemon Rust files | PASS |
| Scoped `git diff --check` | PASS |
| Primary/archive and before/after path/mode/hash checks | PASS |

One full daemon run was warranted because R1 changes the shared output queue and
transmission return path. It also covers the existing configuration and data
tests. The focused filters overlap that run; counts are not additive. The
separately accepted primary monc alias and store cfg(test) fixture are recorded
inputs, not edits made by this reviewer. E1 remains isolated. The full daemon
run is not a new full storage-fault audit or workspace/native qualification.

### Tested source identity and release manifest

Primary root: `/mnt/workspace/rust-mds`.
HEAD before/after: `85a37c8d615992b4600d7467fdedd514fecfac03`.
The manifest identifies the uncommitted candidate beyond HEAD.

Combined author evidence:

```text
/tmp/opencode/sol-daemon-r1-strict-ci-20261025.aKufTD/
```

Independent evidence:

```text
/mnt/workspace/rust-mds/target-sol-daemon-review/r1-strict-independent-20261025.uf2nihyo/
```

All **132** source/contract paths, full regular-file modes and SHA256 values match
the author manifest and archive. They remain identical before/after independent
tests, strict clippy, build and formatting checks. The **129** Cargo input
fingerprint is:

```text
6d46a8ffb9132e567adf1fbb3f1f8ad9ae7b3f766f95f297bd200757376fe4d8
```

It uses the same sorted relative-path/NUL/file-bytes/NUL SHA256 procedure as the
earlier reviews. The source set includes the three contract/document paths in
addition to the Cargo inputs. The earlier pre-R1 and pre-lint fingerprints remain
historical.

| Retained artifact | SHA256 |
| --- | --- |
| Independent `release-source-path-mode-sha256.txt` | `7b4b184505c6172fa522f5355f9ffcea2580fe2de1dbbe4c52470d4a4525c2e4` |
| Author `combined-source.tar.gz`, all archived bytes/modes checked | `b3a54565a845092aad91fb6cf76b8b1edcd4e6a5659f00dd7f45f067254ca14b` |
| Independent `review-d6-r1-exact-probe` | `fbf81d90222ccf571568f7bd6d3e72ca47eeae5c856aefc26a8b405f825a1862` |
| Independent `exact-probe.log` | `0527f28bd71e9614c62796c8ebd2747da65104310996b7a8638f6adc216ab79a` |

The independent directory retains matching before/after source manifests,
Cargo-input manifests, `source-stability.txt`, `independent-lint-only.patch`,
`independent-lint-preservation.txt`, exact-probe build/input identity, command
records, all independent logs and `artifacts.sha256`.

Only this report and review artifacts were written. No production/test edit,
live operation, Git write, commit or subagent ran. **Nonempty D5 remains held;
no live or mounted-client G3 result is claimed.**
