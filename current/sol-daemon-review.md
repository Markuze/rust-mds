# sol-daemon-review
Task: Independent protocol-safety review of daemon changes against f54d298. Return ACCEPT or CHANGES REQUIRED with ranked findings and minimal regression requirements.
Status: Focused G2 correction re-review complete: ACCEPT. G2-R1 through G2-R4 are closed. Independent focused tests: 11 passed. Full daemon tests: 82 passed. Production build passed. No remaining blocker in this reviewed runtime package.
Context: Full new daemon integration is in scope. Store/engine packages are accepted. Detailed storage authority/timeout/allocator/checkpoint internals belong to a parallel reviewer. Sole writable file is this ledger. No source edits, live operations, commits, or subagents.
Next: Coordinator can send the combined package to QA for offline checks and native G2 clean-restart validation. Nonempty G3 reconstruction and active messenger backlog accounting remain explicit limits. No live action is authorized for this reviewer.
Log:
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
