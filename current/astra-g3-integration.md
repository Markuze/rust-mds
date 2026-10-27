# astra-g3-integration

Task: Independent Astra Ultrafast max source-oriented G3 review and preparation. Compare the held daemon reconnect path, engine reconnect adapters, durable sessions, and protocol codecs. Produce the smallest concrete implementation assignment after independent G2 acceptance.
Status: Source review complete. The assignment below identifies daemon work, two separately owned engine changes, existing reusable APIs, and required regressions. This is preparation, not G3 implementation or acceptance. No new G2 outcome was supplied to this review.
Context: Startup base supplied by coordinator: a6ae05a. QA owns the same-filesystem G2 retry. Cargo sources stay unchanged until independent acceptance. Read-only source review only; no builds, tests, commits, live calls, or subagents.
Next: Coordinator obtains independent G2 acceptance, resolves the owner decisions in section 8, and assigns the engine and daemon packages in section 6. Independent review and sole-QA mounted-client qualification follow implementation.
Log:
- [2026-10-25] Created ledger before source review.
- [2026-10-25] Reviewed repository contracts, the accepted G2 boundaries, current daemon/engine/store/proto sources, and the C++ recovery paths. Existing chunk, lock replacement, session-history join, cleanup receipt, and lifecycle APIs cover most of G3.
- [2026-10-25] Completed the dirty-flush and realm checks. Current finalization can evaluate caps before early dirty flushes commit. Realm UPDATE does not replace a real inode-to-realm SPLIT. Recorded these requirements and the committed-creator retry gap below.
- [2026-10-25] Finished the integration assignment and regression matrix. This ledger is the only edited file. No production edits, tests, builds, Git operations, live calls, or subagents ran.

## Review report

### 1. Conclusion and evidence scope

**G3 is mainly daemon orchestration over existing persistence and reconnect APIs.** Assign two engine changes before qualifying recovery: committed-creator reply reconstruction, and an early-dirty-flush barrier before recovery cap grants. No missing G3 wire codec was found.

Use the existing one-record `Engine::step` / `journal_event` path. Keep recovery state on the owning daemon task. Existing workers perform I/O and return tagged completions. Do not introduce another engine owner or a second session repository.

This report follows `design/persistence-contract.md` sections 7–8, `design/workplan.md` G3, and `current/sol-recovery-review.md`. The earlier reference report states protocol requirements. Its API-gap table does not establish which current engine APIs are absent.

Source locations refer to the inspected working files. The coordinator supplied base `a6ae05a`; this review did not verify Git provenance. C++ paths below are relative to `/home/ubuntu/ceph/src/`. The reference report identifies its source-map revision as `5f7e383ae6d`. Source inspection does not establish installed-peer behavior or a passing gate.

### 2. Current implementation boundaries

| Boundary | Current Rust source and finding | C++ reference |
|---|---|---|
| Recovery phases | `crates/mds/src/daemon.rs::drive_phase` already preserves monitor-visible REJOIN for clientless replay. It combines REJOIN and CLIENTREPLAY and requests ACTIVE when drained. `is_serving`, `refresh_authority`, `runnable`, `release_held`, and `on_tick` do not provide nonempty recovery admission/output permissions. | `mds/MDSRank.cc::replay_done`, `reconnect_start`, `reconnect_done`, `rejoin_done`, `clientreplay_start`, `queue_one_replay`, `maybe_clientreplay_done`; `mds/MDSMap.cc::state_transition_valid`. |
| Prior-owner proof and cold followers | `daemon.rs::maybe_start_storage` excludes standby-replay. Membership requires `map.up[rank] == gid`. `crates/mds/src/persistence.rs::open` and `prove_writers` inspect and fence current/predecessor writers before acquisition and load. Zero failure epoch is not treated as sufficient proof. These are existing foundations. | `mds/MDSRank.cc::replay_start`, `replay_done`; `mds/FSMap.cc::erase`. |
| Durable session join | `crates/mds/src/server/runtime.rs::from_loaded` joins Open/Closing/Closed rows with shard histories and cleanup receipts. It builds `Runtime::gather` from Open keys. Startup `persistence.rs::open` already fences recovered Closing rows. `try_finish_recovery` currently requires an empty gather and calls `finish_reconnect(&[])`. | `mds/Server.cc::reconnect_clients`; Rust persistence contract supplies the stronger cross-object lifecycle ordering. |
| Reattachment | `daemon.rs::on_msgr_event` only logs Accepted/Reset/Closed. `dispatch_client_raw` requires an open, durably attached runtime session for reconnect, requests, and caps. `runtime.rs::Opening` / `on_storage_event` publishes ordinary OPEN after the durable Open result. Recovery needs silent reattachment before a reconnect packet or fresh REQUEST_OPEN. | `mds/MDSDaemon.cc::ms_handle_accept` (1266–1319); `mds/Server.cc::dispatch` (319–397). |
| Reconnect input | `daemon.rs::on_client_raw` puts reconnect packets in `reconnect_held`; no consumer reconstructs them. `runtime.rs::handle` ignores `ClientMessage::Reconnect`. The raw holding queue is bounded at 1024 messages and 16 MiB, but no applied-reconstruction budget exists yet. | `mds/Server.cc::handle_client_reconnect`; `client/Client.cc::send_reconnect` (3438–3522). |
| Reconstruction | `crates/engine/src/engine/reconnect.rs::{reconnect_chunk, reset_reconnect, finish_reconnect, reconnect_cap_context}` already exist. `client_caps.rs::reconnect_cap` preserves cap IDs, resets sequences, raises the allocator, replaces locks, and substitutes authenticated client IDs. `crates/engine/src/durability.rs` already defines `ReconnectCap`, `ReconnectRealm`, and `ReconnectProgress`. | `mds/Server.cc::{handle_client_reconnect, recover_filelocks}`; `mds/CInode.cc::reconnect_cap`. |
| Early dirty flushes | `engine/step.rs::step` rejects CapUpdate before `recovery_ready`. `engine/reconnect.rs::finish_reconnect` sets readiness and immediately calls `eval_caps`. No dirty-reconnect protection exists between these actions. `engine/caps_eval.rs::eval_caps` also has no recovery-phase guard for calls from cleanup paths. | `client/Client.cc::early_kick_flushing_caps` (5382–5407); `mds/Locker.cc::handle_client_caps` (3398–3417); `mds/MDCache.cc::choose_lock_states_and_reconnect_caps`; `mds/CInode.cc::choose_lock_states`. |
| Committed retries | `engine/mod.rs::dispatch_new` (528–555) returns traceless success for every completed REPLAY and every completed non-OPEN/non-CREATE retry. Completed CREATE instead reaches `ops_ns.rs::op_create` and `ops_attr.rs::op_open`; it opens the current pathname but returns the stored created inode. These paths do not establish G3 creator-trace correctness. | `mds/Server.cc::dispatch` (363–380), `handle_client_request` (2657–2692). |
| Missing-session eviction | `server.rs::request_eviction`, `daemon.rs::matches_eviction`, and `runtime.rs::finish_eviction` require a runtime `Session`. Missing restored sessions have none. `daemon.rs::drive_control` also waits for `is_serving` before consuming a successful fence. The durable Closing → cleanup → Closed chain already exists. | `mds/Server.cc::reconnect_tick`; `mds/MDSRank.cc::evict_client`. |
| Retained jobs | `engine/durable.rs::{pending_jobs, lifecycle_intent, complete_job}` and `EngineInput::LifecycleCompleted` already provide durable job ownership. `daemon.rs::drive_jobs` is serving-only. `runtime.rs::ready_jobs` requires recovery and unpaused work. This must progress during recovery drain. | `mds/MDSRank.cc` recovery completion; `mds/MDCache.cc` recovered-cap/lost-writer handling. Rust job durability follows the persistence contract. |
| UUID/reclaim/timeouts | `runtime.rs::handle` returns `MClientReclaimReply::default()` for every reclaim. `handle_session` lacks UUID uniqueness and required-feature policy. `tick` uses `session_timeout + session_autoclose` and ignores timeout metadata. Store rows already persist identity/features/metadata. | `mds/Server.cc::{find_session_by_uuid, reclaim_session, finish_reclaim_session, handle_client_reclaim, find_idle_sessions}`. |

### 3. Daemon assignment

#### D1. Phase permissions and event-driven progress

Owned paths: `crates/mds/src/daemon.rs`, `crates/mds/src/server/runtime.rs`, and their tests.

1. Separate healthy recovery control, recovery output, recovery data work, and ordinary request admission. All use current membership, owner, beacon health, self-blocklist state, and fail-stop checks. Only ordinary admission requires confirmed ACTIVE and completed recovery. Preserve the existing STOPPING drain policy.
2. Continue maps, beacons, store completions, reconnect chunks, renewals, cap control, fencing, and permitted lifecycle completions while ordinary requests wait. Extend `runnable` and the bounded owning-task turn. Progress must not depend on a later Tick or unrelated client message.
3. Use monitor-confirmed `REPLAY -> RECONNECT -> REJOIN -> ACTIVE` for an empty recovery drain. Use `REJOIN -> CLIENTREPLAY -> ACTIVE` when recovery replies or required writes remain. Keep requested state separate from confirmed state. Do not expose ordinary service merely because the monitor confirmed a requested state.
4. Bound and retain messages received after RECONNECT was requested but before its map confirmation. Process their reconnect state after confirmation. A map-delivery race is not a late-reconnect rejection.
5. Track recovery drain separately from held ordinary traffic. Waiting for the ordinary queue to empty before ACTIVE would deadlock. Conversely, current `Server::drained()` alone does not prove that recovery output reached its session funnel's transport-acceptance boundary.
6. Preserve clientless bootstrap/restart, prior-owner proof, and `ACTIVE -> STOPPING -> STOPPED`. Reject stale owner/operation completions. Keep cold standby and standby-replay free of authority acquisition, WAL append, checkpoint, purge, and damage reporting. Promotion enters the complete fenced load with the new rank incarnation.

No new monitor state or distributed rejoin exchange is needed.

#### D2. Durable, authenticated recovery attachment

Owned paths: `daemon.rs`, `server/runtime.rs`, `session.rs`.

- Bind the authenticated messenger global ID to the existing durable `SessionKey { client, incarnation }`. Require the durable authenticated entity to match. A new authenticated authorization context or address must complete its durable update before it can authorize recovery input.
- Start attachment on Accepted or the first eligible early request/cap message. Reconnect is not necessarily the first message. Keep early input bounded while attachment persists.
- Reuse `store::SessionRepository::open_session_record` through existing `CommandKind::Open`. Updating an Open row preserves its key. Retain metadata, CephFS features, and the full old-plus-new address/nonce union. The repository replaces identity data; the daemon must build that union. Never reopen Closing or reuse a Closed incarnation.
- Give `Opening` an explicit recovery-attachment purpose. Its completion installs the matching key and a silent funnel; it must not publish ordinary OPEN. `SessionMap::install_silent`, `publish_open`, and typed `PendingOutput::Open` provide most of this machinery. Recovery publication needs to be explicit and independent of duplicate-REQUEST_OPEN notification policy.
- Carry the connection ID and runtime attachment generation with staged input and completion checks. `msgr::Connection::id()` already exists. A daemon-local attachment record suffices; no new messenger/platform API is required. Address equality alone cannot identify the incomplete chunk stream to reset.
- Recheck attachment identity, lifecycle, features, blocklist, owner, and generation when a durable update completes. A stale attachment result must not publish into a replacement funnel.
- Enforce MDSMap `required_client_features` on OPEN and restored reconnect. Compare CephFS bits, not messenger feature bits. Current G3 bits fit the persisted bitmap. Preserve `reply_encoding`, `reclaim_client`, and `multi_reconnect` support. Use `MClientSession::open_default` for supported features, metric specification, and cap auths.
- Closed/denied/late reconnect gets CLOSE. Unknown session reconnect may get the defined sessionless REJECT. REJECT is legal only for OPEN/reconnect, never for a sessionless renewal. These terminal messages must also have a bounded, ordered funnel.

Recovery starts a fresh push sequence at zero. `Session::transmit` already increments it only after successful transmission of CAPS/SNAP/LEASE/QUOTA. `PendingOutput::Open` assigns OPEN.seq at its FIFO position. Preserve both rules when recovery output becomes runnable.

#### D3. Bounded chunks and reconstruction

Owned paths: `server/runtime.rs`, `daemon.rs`, `session.rs`; use existing engine/proto interfaces.

1. Decode `MClientReconnect` from its data segment. v3 is one complete message with raw realm records. v5 chunks may each contain both caps and realms. `more=false`, including an empty final chunk, ends that session's collection. The protocol has no chunk ID or index.
2. Parse **every** `CapReconnect::locks()` blob before calling the engine. Translate both fcntl and flock locks. Pass `ino`, `cap_id`, `wanted`, `issued`, `snaprealm`, `snap_follows`, and both lock vectors through `ReconnectCap`; pass realm `ino/seq/parent` through `ReconnectRealm`.
3. Call `Engine::reconnect_chunk` once for the whole translated chunk. It validates the session, all inode references, duplicate inode entries, lock types/ranges, and realm records before installation. It rejects missing resident metadata. `pathbase/path` are hints, not authority to recreate an inode. Preserve unlinked-open and hard-linked inode identity.
4. Maintain a missing-member set and surviving Open-key set separately. Partial chunks never remove a member or publish OPEN. Only validated final reconstruction or durable Closed removes a missing member. At rank finalization, pass **all surviving Open keys** to `finish_reconnect`, not the now-empty missing-member set. The engine checks exact equality with its remaining sessions.
5. Account for staged/applied caps, lock vectors, realms, retained corrections, early requests/flushes, and raw bytes. Releasing a raw buffer does not release the memory budget for installed reconstruction. Charge replacement entries by their new retained size. Bound both count and bytes. Preserve the current fail-closed overflow behavior unless an independently reviewed alternative is assigned.
6. Before the final chunk, Reset/Closed from the matching connection discards that generation's staged reconstruction with `reset_reconnect(key)` and releases its staging budget. It preserves durable identity, retry/flush histories, and gather membership. Ignore resets and packets from older connections. Repeated per-inode chunks replace cap/lock state; they do not append locks. A duplicate final chunk must not complete the gather or publish OPEN twice.
7. Once a session is complete, a transport reset still does not mean durable session close. Define its replacement-connection policy explicitly. Do not discard final reconstructed rights merely because Closed arrived.

The engine already resets cap sequences, preserves IDs, raises `last_cap_id`, and overwrites lock client IDs with the authenticated client. Use these functions rather than restoring fields directly. Do not call legacy `reconnect_done` to bypass gather checks.

**Realm correction:** `ReconnectProgress::realm_traces` supports SNAP UPDATE for stale realm information. It does not identify all inode moves. For each retained reconnect cap, compare its reported realm with the first realm of authoritative `Engine::snap_trace(ino)`. A different realm requires a real `MClientSnap::split` with the affected `split_inos`, authoritative child-realm relationships, and the correct trace. UPDATE alone does not move an inode between client realms.

Existing `snap_trace`, `snap_realm_info`, `inode_numbers`, retained cap/realm records, and `proto::MClientSnap::{update, split}` suffice at the frozen, quiescent reconstruction cut. Derive child-realm relationships from authoritative realm information; do not use the reported parent as authority. Use `SnapFormat::for_client` for old/new trace encoding. Cover root, quota/subvolume, and nested realm relationships. For the first implementation, enqueue validated SNAP corrections before typed final OPEN in the same funnel. Let transmission assign OPEN.seq after those counted messages. Ordinary requests and purge remain held through reconstruction.

References: `mds/MDCache.cc::{open_snaprealms, prepare_realm_split, finish_snaprealm_reconnect}` (5982–6057, 5730–5748, 6151 onward). `ReconnectProgress` does not need a new wire representation. The proto comment at `crates/proto/src/reconnect.rs:9` is wrong about mixed chunks; assign that comment correction separately to the proto owner.

#### D4. Fenced cleanup for missing durable keys

Owned paths: `server.rs`, `server/runtime.rs`, `daemon.rs`; reuse `fencing.rs` and persistence commands.

- Extend the eviction target to identify a durable `SessionKey`, with an optional current runtime attachment. A missing restored session needs no fabricated connection. Freeze its full persisted address union for the fence attempt and stop new attachment/admission for that key.
- Start a reconnect deadline only for the confirmed reconnect window. Use the agreed reconnect timeout/activity-extension policy. At expiry, evict every missing member, including partial reconnects and sessions with `timeout` metadata. Do not add C++'s extra timeout-session reclaim hold.
- Reuse `fencing::fence`: successful blocklist or verified existing entry, checked latest-map wait, current-map proof for every address, then monotonic Objecter/server barrier. A failed command or map wait leaves rights and gather membership intact.
- Permit successful fence consumption during healthy recovery. Current `drive_control` waits for serving and would otherwise deadlock the gather.
- Continue the existing chain: persist Closing, drain/clean through `EngineInput::CloseSession`, wait for `EngineNotice::SessionCleanupCommitted`, then persist Closed. Only the tagged Closed completion removes the gather member. Recovery of an interrupted chain must use the existing cleanup receipt.
- Validate owner, operation, durable key, lifecycle, and any attachment generation at every completion. If the address union changes, the old proof does not cover the new address. A completion for an old incarnation must not close a newer one.

Current `runtime.rs::on_storage_event` already removes the gather key at durable Closed. Keep that boundary. Existing startup fencing of recovered Closing rows also remains the recovery path after a crash in this sequence.

#### D5. Recovery retry/control drain and retained jobs

Owned paths: `daemon.rs`, `server/runtime.rs`, `server.rs`. Depends on E1/E2 below.

- Classify early requests after checked load and durable attachment. `Engine::session(client)` exposes its epoch, completed requests, completed flushes, and watermarks. Match the durable key and request identity. Known committed retries enter a bounded recovery queue ahead of ordinary requests. Unknown safe-only retries remain ordinary work. Preserve flags; do not manufacture REPLAY to obtain a completed reply.
- Strip pre-reconnect embedded cap releases from reconnect-era retries before engine admission. Preserve the original request's validation context. Clients resend old requests and flushes before reconnect (`Client::send_reconnect`, `resend_unsafe_requests`). Receipt of those messages must not block reading the final reconnect chunk.
- Process cap updates/releases through existing `EngineInput` variants and the one-record scheduler. Deduplicate completed flushes through restored history. An uncommitted flush needs its journal completion before FLUSH_ACK or dependent cap grants. E2 supplies the missing engine ordering barrier.
- In confirmed CLIENTREPLAY, drain controls and required jobs that unblock committed replies. Use `pending_jobs` / `ready_jobs`, existing owner/operation-tagged worker results, and `LifecycleCompleted`. Do not keep jobs or cap acknowledgments behind ACTIVE: `Engine::execute` parks requests while any RecoverSize intent exists.
- Start destructive/probe work only after predecessor fencing and complete session reconstruction/eviction. Respect the engine's eligibility checks: no purge with caps; RecoverSize waits for issued writer rights and conflicting truncate work. Retain truncate sequence, layouts, and maximum writer bounds. Probe failure retains the job; it is not an empty-file result. Commit recovered size/mtime and job retirement before publishing their effects.
- Preserve the existing monotonic barrier inputs from `MClientCaps` / `MClientCapRelease` and outgoing cap/FLUSH_ACK encoding. Recovery dispatch must use these paths too.
- Run committed requests through `EngineInput::Request`. Keep `pending_requests` context until `send_effects` validates the reply. Drain required journal writes and recovery output before releasing ordinary requests that can change the reply's inode/parent. Completion means ordered transport acceptance, not a new client acknowledgment protocol.
- Request ACTIVE only after the recovery queue, required engine continuations/commits, cleanup, and recovery output have drained. Confirm ACTIVE before ordinary admission. Independent retained background work may continue under its existing durable eligibility rules.

#### D6. UUID, active timeout policy, and Ganesha RESET/FINISH

Owned paths: `server/runtime.rs`, `server.rs`, `session.rs`, `daemon.rs`.

Use the durable row set for UUID lookup. Include pending OPEN reservations in uniqueness checks, so two concurrent OPENs cannot claim the same UUID. A duplicate update of the same key is different from a new session claiming another live key's UUID. Retain reservations until the corresponding lifecycle decision is durable. Closed tombstones are not reclaim targets.

| Input/state | Required result |
|---|---|
| Sessionless requester, or requester not Open/Stale | Do not run reclaim under another session's identity. |
| FINISH alone | No reply. RESET does not require a subsequent FINISH. |
| FINISH combined with any other flag | `-EINVAL`. |
| Non-FINISH flags other than exactly RESET, or RESET with empty UUID | `-EINVAL`. |
| Existing UUID owned by a different authenticated name | `-EPERM`; no target cleanup. |
| UUID absent from live durable rows | Result 0, epoch 0, empty address vector. libcephfs converts all-empty RESET success to `-ENOENT`. |
| UUID target present | Save its old addresses; complete D4 fencing and durable close; then result 0 with old addresses and a post-fencing OSDMap epoch. Recheck requesting key, attachment generation, and lifecycle before sending. |
| Target already closing or another reclaim is pending | Join/serialize the existing target cleanup. Do not release rights twice or return success before Closed. Bound pending reclaim state. |

Reply through the requester's funnel. Failure to fence cannot become success. A changed or disappeared requester cannot inherit an old completion. The existing row metadata and `MClientReclaimReply` fields suffice; no store schema or new reclaim message is needed.

During normal ACTIVE service, apply the accepted timeout/autoclose policy to persisted metadata. A timeout session must not lose caps/locks through the ordinary stale-session path while it is retained for reclaim. The current sum of `session_timeout + session_autoclose` is not that policy. Keep active-session timers separate from the reconnect deadline. Source detail to resolve before editing: `Server::find_idle_sessions` checks autoclose before its timeout override, skips stale for timeout sessions, treats zero as no override expiry, and can directly evict at a positive override. The design text describes retention until reclaim/kill/autoclose. The design/daemon owners must state the supported timer rule explicitly; this does not reopen the no-extra-recovery-hold decision.

References: `mds/Server.cc` 454–597 and 1211–1353; `client/Client.cc::{start_reclaim, finish_reclaim, handle_client_reclaim_reply}` (18861–19004).

### 4. Separately assigned engine work

The daemon owner must not edit `crates/engine/**` under its current ownership. Agree the following contracts with the engine owner before changing consumers.

#### E1. Correct committed-creator replies through the existing request API

Owned paths: `crates/engine/src/engine/{mod.rs,ops_ns.rs,ops_attr.rs}` and engine tests. Use existing internal reply construction and committed visibility.

Current source facts:

- `dispatch_new` returns no target or dentry for completed non-open creators such as MKDIR/MKNOD/SYMLINK. Its completed REPLAY branch is also traceless. It uses literal result 0. The durable `CompletedRequest` already stores op/result/created inode; current records describe successful mutations.
- Completed CREATE skips O_TRUNC but resolves the current path through `op_open`. If that name now refers to another inode, the target and stored create extra can disagree.
- C++ queues completed retries before new mutations and reconstructs some creator replies through lookup/getattr in clientreplay. Its unsafe REPLAY shortcut is not proof that a lost **safe** creator reply can always be traceless.

Required behavior:

1. Identify the recorded completion by the same session incarnation and tid, and validate its operation. Preserve the recorded result and original created inode. Do not repeat allocation, link changes, overwrite, truncate, xattr mutation, or timestamps merely to answer a retry.
2. Reconstruct the required trace/caps from the engine's committed state. For CREATE, keep target identity, create extra, and dentry relationship consistent. Handle completed non-open creators through a read/reply path, not by rerunning their creator operations. Keep the original wire operation/tid for daemon validation.
3. Preserve request holding and commit rules if reply reconstruction needs a new durable writer range. “Do not rerun the mutation” does not authorize an unjournaled range grant.
4. Define the changed-path/missing-original-inode case before implementation. Never invent a historical dentry or return a different target with the old create inode. Reconstruct supported cases from committed linkage before ordinary recovery traffic can change it. If required information is absent, return to the owners with the exact compatibility requirement; do not silently add stored stale traces or a new persistence schema.

**API recommendation:** retain `EngineInput::Request` and the existing `Effect::Reply` contract. Repair the engine's completed-request interpretation internally. No new daemon “fetch inode and make reply” API is justified: `Engine::inode` explicitly forbids constructing client output from its diagnostic working state.

`MClientReply::check_invariants_for` checks parent identity when a dentry exists; it does not require every successful creator reply to contain a trace. Tests must assert the actual trace and original inode. An empty violation list alone is insufficient.

#### E2. Early dirty flushes must precede conflicting cap grants

Owned paths: `crates/engine/src/engine/{reconnect.rs,step.rs,caps_eval.rs,client_caps.rs,durable.rs}`, `crates/engine/src/durability.rs` only if the agreed event needs it, and engine tests.

This is a concrete ordering gap, not just a missing daemon queue. A client can resend an uncommitted dirty flush before reconnect after the corresponding rights were revoked. Its reconnect `issued` mask need not describe all dirty metadata still awaiting commit. C++ records those dirty bits during reconnect and includes them in recovery lock-state selection. Rust currently blocks CapUpdate until `finish_reconnect`, but that function evaluates/grants caps before the queued flush can enter `step`. Reconstructed caps are eligible for this evaluation: `reconnect_cap` calls `Capability::issue_norevoke`, which clears `is_new` (`crates/engine/src/caps.rs:274–281`).

Pausing outgoing messages alone is insufficient. `eval_caps` already changes cap sequence/state and materializes attribute payloads. Releasing those queued messages later can publish a grant built before the dirty metadata committed.

**Proposed minimal contract for owner agreement:** split structural gather completion from release of recovery grants. Let checked `finish_reconnect(keys)` enter a control-drain phase. Add one explicit wire-free completion input, tentatively `EngineInput::RecoveryControlsDrained`, consumed through `step` and returning normal `EngineStep` effects. This is a proposal, not an existing API.

- During control drain, existing CapUpdate/CapRelease/CloseSession/LifecycleCompleted inputs remain the transaction interface. Ordinary requests stay held. Protect early uncommitted dirty metadata until its commit.
- Gate cap-grant evaluation in all paths, including missing-session cleanup and commit callbacks, not only the direct `finish_reconnect` loop. Do not publish stale prebuilt grants. Preserve FLUSH_ACK commit ordering and durable-range limits.
- Permit the revoke/ack and eligible retained-job progress required to drain. Do not create a deadlock between RecoverSize, dirty CapUpdate Busy, and the grant gate. New unclaimed-stray purge must not outrun reconstruction or protected recovery work.
- The daemon sends the completion input only after its ordered early-control set has drained. The engine also checks its pending commit/continuation state. The completion releases evaluation once and uses the existing one-record/effect protocol. An empty drain takes the same checked path.
- On disconnect, retry, or eviction, retain or retire dirty protection under the matching session/connection and durable cleanup rules. A reset must not make dirty metadata safe by merely deleting a queue entry.

This conservative gate is smaller than copying C++ lock-state machinery or adding a second flush pipeline. The engine owner must prove the RecoverSize/control progress case before freezing the signature. No WAL/checkpoint schema change is identified by this review.

### 5. API sufficiency summary

| Need | Existing interface / required change |
|---|---|
| Session history restore and gather validation | Existing `install_session_key`, `session`, `session_cleanup_seq`, `reconnect_chunk`, `reset_reconnect`, `finish_reconnect`. Daemon integration is missing. |
| Reconnect locks and IDs | Existing `ReconnectCap`, `ReconnectRealm`, `reconnect_cap_context`, `CapReconnect::locks`. No new lock-import API. |
| Realm UPDATE and actual SPLIT | Existing `snap_trace`, `snap_realm_info`, retained reconnect records, and proto SNAP constructors. No missing codec. Preserve quiescent reconstruction when reading topology. |
| Early-flush admission before grants | Missing engine phase distinction/release event: E2. Existing CapUpdate/CapRelease remain the work inputs. |
| Committed creator trace | Missing engine behavior: E1. Existing Request/Reply API is the preferred boundary. |
| Recovery attachment and absent-session eviction | Missing daemon-local state/APIs in D2/D4. Existing store Open/Closing/Closed commands and cleanup notice suffice. |
| Jobs, barriers, and checked fencing | Existing `pending_jobs`, `LifecycleCompleted`, `raise_epoch_barrier`, `fencing::fence`, and tagged completion paths. Change phase eligibility, not the backend contract. |
| Reclaim and session wire output | Existing reclaim/session codecs and session send funnel suffice. Implement policy and completion ownership in the daemon. |

### 6. Testable dependency order

| Step | Assignment and prerequisite | Exit evidence required before the next dependent step |
|---|---|---|
| 0 | Coordinator: independent acceptance of the actual G2 retry; then thaw assigned files. | Accepted result/evidence and explicit code ownership. This review supplies neither. |
| 1 | Daemon: D1/D2, plus bounded recovery state and transport-generation handling. | Delayed map/Open callbacks; early retry/flush before reconnect; no premature OPEN or ordinary admission; stale connection isolation. |
| 2 | Engine owner: E1/E2 under agreed semantics. May proceed independently of daemon-local work after step 0. | Creator trace/identity tests and early-dirty conflict tests through the production step/journal interface, including retained-job progress. |
| 3 | Daemon: D3/D4 using existing reconstruction plus E2 finalization. | Nonempty mixed-client gather; interrupted/repeated chunks; real realm correction; failed/delayed fences; durable missing-key cleanup. |
| 4 | Daemon: D5 using E1/E2 and step 3. | Confirmed REJOIN/CLIENTREPLAY; early-control/retry drain; jobs and caps progress without ACTIVE; ordinary traffic released only at the final barrier. |
| 5 | Daemon: D6 using D2/D4; design owner resolves timer wording first. | Exact RESET/FINISH/error/no-reply matrix; UUID race and requester-generation tests; active timeout versus reconnect timeout tests. |
| 6 | Independent reviewers: engine and daemon packages; designated QA: combined offline checks. | Review acceptance; relevant crate/workspace results, build/fmt/clippy outcomes and actual baseline warnings recorded by assigned owners. |
| 7 | Sole QA: mounted-client G3 trials on preserved identity/pools, using reviewed tooling/binary and explicit live assignment. | External acknowledged-history oracle, exact peer/version/provenance records, repeated failure results, and independent evidence acceptance. |

The daemon owner can keep D1–D6 in one coherent local state machine. Separate file ownership is needed for E1/E2 and the proto comment correction, not separate recovery actors.

### 7. Mandatory G3 regression matrix

The following are requirements for implementers and QA. **This review ran none of them.** Use delayed, duplicate, failed, and stale tagged callbacks. Engine tests must exercise `step`/`journal_event`; daemon tests must exercise the production `DaemonJournal` runtime rather than only MemJournal compatibility helpers.

| ID / owner | Scenario | Required observation |
|---|---|---|
| R1 / daemon | Empty and nonempty gathers; delayed/reordered monitor confirmation; reconnect arrives after request-state but before map; crash at each phase. | REJOIN remains visible. CLIENTREPLAY is confirmed when needed. No ordinary input on a requested state alone. Old completion cannot advance replacement recovery. |
| R2 / daemon | Retry, dirty flush, renewal, then reconnect without REQUEST_OPEN; changed address/authorization context; wrong name; delayed durable attachment; stale connection reset. | Same durable key/history; complete address union before input authority; no early OPEN; current generation survives stale callbacks; no sessionless-renew REJECT. |
| R3 / daemon + engine | Mixed v5 caps/realms in multiple chunks, empty final, repeated inode, duplicate final, v3 raw realms, malformed last lock blob. | No partial malformed installation, duplicate locks, partial OPEN, or double completion. v3 completes once. Gather limits include applied data. |
| R4 / daemon + engine | Partial reconnect reset/replacement; cap-ID allocator boundary; fcntl/flock shared/exclusive ranges, including length zero; spoofed lock client field. | Incomplete state is replaced under the right connection. History survives. IDs/sequences follow the existing contract. Authenticated client owns every restored lock. |
| R5 / daemon + engine | Root/quota/subvolume realms; changed realm sequence/parent; cap moved to another realm; nested child realms; old/new encoding. | UPDATE corrects realm data. Genuine SPLIT carries affected inode/child-realm identities. Traces are well formed and funnel order/push sequence is correct. |
| R6 / daemon | Successful reconnect plus one missing timeout session; failed blocklist, delayed latest map, partial address proof; crash at Closing/cleanup/Closed. | Gather and conflicting rights remain held until verified fence and durable Closed. No extra timeout-session recovery hold. Interrupted cleanup resumes from durable proof. |
| R7 / engine + daemon | Uncommitted dirty flush precedes reconnect with dirty rights absent from reconnect issued mask; competitor wants those rights; missing-session cleanup also runs. | No conflicting grant before dirty commit. New grant metadata reflects the committed flush. Controls, revokes, and eligible jobs still make progress. |
| R8 / engine + daemon | Lose FLUSH_ACK after commit; resend before reconnect and after OPEN; delay a new flush commit. | Restored dedupe prevents duplicate mutation. Committed flush can be acknowledged again; new flush cannot. Correct cap ID/sequence and maximum OSD barrier survive. |
| R9 / engine + daemon | Lose safe reply after O_EXCL CREATE, MKDIR, MKNOD, SYMLINK; queue another client's rename/unlink/replacement; include already-changed committed path. | One original mutation/inode. No rerun EEXIST or repeat truncate. Actual required trace/extra/target identity agrees. Reply uses original op/tid and correct locked parent. Defined changed-path behavior passes peer-compatible assertions. |
| R10 / engine + daemon | Completed rename-over-target, link, unlink, xattr, truncate/grow retry; ordinary and completed requests interleaved. | Recovery retries precede ordinary mutations and do not repeat effects. Old embedded releases cannot discard new caps. Durable histories retain incarnation scope. |
| R11 / daemon + engine | Unlinked-open/hard-linked files and both lock classes; retained purge/truncate/RecoverSize; late job result, failed probe, dirty flush while a probe is pending. | No purge before reconstruction/last-cap release. No conflicting lock grant. Recovery drain does not wait forever for active-only jobs. Failed/late results do not retire another job; recovered fields wait for commit. |
| R12 / daemon | Laggy beacon, rank removal, self blocklist, owner mismatch during gather, output drain, or cleanup; standby-replay removal makes failure epoch zero. | Admission/output/data permissions close on lost authority. Durable predecessor proof remains mandatory. Cold follower performs no ownership or destructive work; promotion uses full load. |
| R13 / daemon | Paused/partial output drain around SNAP, OPEN, CAPS, FLUSH_ACK, CLOSE; send failure; old close sequence. | Exactly counted transmitted pushes; OPEN.seq bound to its FIFO position; no old-generation CLOSE authority; no bypass of the session funnel. |
| R14 / daemon | Duplicate live UUID, pending-OPEN UUID race, absent RESET, wrong auth, invalid flags, FINISH-only/mixed, target already closing, requester disappears or changes generation. | Exact D6 results/no-reply behavior. Existing-target success follows durable Closed and reports old addresses/post-fence epoch only to the same requester. |
| R15 / daemon | Active silent timeout session, ordinary session, renewal, autoclose, and reconnect deadline. | Approved active retention/expiry policy is explicit. Every eviction fences first. Timeout metadata never adds a post-reconnect reclaim hold. |
| R16 / sole QA | Repeated active-MDS SIGKILL and cold takeover with kernel and FUSE mounts, externally saved acknowledged history, O_EXCL, rename-over-target, links, unlink-open, xattrs, truncate/grow, dirty caps, locks, and reply/FLUSH_ACK loss windows. | Same filesystem/pools and continuous mounted-client recovery. Exact acknowledged-state/identity checks pass; retries do not repeat mutations. Record reconnect and first-success latency. A final listing or normal smoke is insufficient. |
| R17 / sole QA | Ganesha-only restart, then separate simultaneous MDS/Ganesha failure, with takeover-capable grace and shared-session outstanding flushes. | RESET and NFS reclaim work within the accepted policy. Record the accepted native-CephFS-versus-NFS lock gap after an absent session is evicted. Graceless helper smoke is not recovery evidence. |

G3 does not require distributed resolve/rejoin messages, cap migration/export/import, forced sessions, cross-rank locks, FLUSHMSG migration barriers, or forwarding; those belong to G6. Multiple shard actors/budgets and complete MDS authorization belong to their G4 assignment. Multi-stream concurrency belongs to G5. msgr2, snapshots, unsafe replies, and async dirops retain their existing separate tracks. Disk-backed host/OSD power-loss qualification remains G7; memstore MDS-crash trials do not establish it. These exclusions do not remove G3's local realm, retry, fencing, or clientreplay obligations.

### 8. Owner decisions before implementation

1. **Coordinator:** confirm accepted G2 evidence and grant disjoint assignments. Keep exactly one live QA owner. The same-fs44/meta88/data89 retry and original evidence/acknowledged namespace remain preserved under the current coordination rules.
2. **Engine + daemon owners:** freeze E2's control-drain/release semantics and one-step completion interface. Prove the RecoverSize/dirty-flush progress case before the consumer relies on it.
3. **Engine + daemon owners:** define E1's creator trace for unchanged, moved, replaced, and missing original targets. Keep original wire validation context and avoid diagnostic inode output. Escalate any genuine history-format need before changing schema.
4. **Daemon owner:** choose numeric aggregate reconnect/recovery budgets, deadline/activity extension, final-chunk duplicate behavior, and post-final transport replacement policy. Test these as local generation rules; do not invent wire chunk numbers.
5. **Design + daemon owners:** state the active timeout/autoclose rule precisely against the accepted retention policy and the cited C++ override behavior. The no-extra-Ganesha-recovery-hold policy is already decided.
6. **Coordinator/proto owner:** assign only the mixed-chunk documentation correction and any needed wire regression additions. No codec redesign or new proto API is identified.

After these decisions, the implementation package is bounded: daemon D1–D6, engine E1/E2, focused protocol documentation, independent review, and sole-QA G3 qualification. Publication remains the designated release owner's task after authorization.
