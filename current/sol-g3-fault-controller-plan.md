# sol-g3-fault-controller-plan

Task: Re-review frozen PRIMARY current/astra-g3-fault-interface.md as the actual QA controller consumer. Write only this ledger. Preserve the prior C1–C8 findings/history. No source/tool edits, tests, builds, live queries, signals, mounts, Git writes, commits or subagents.
Status: CONSUMER ACCEPT frozen v1 for implementation at proposal SHA256 3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523, mode 0664. Full candidate read; C1–C8 have no remaining controller-interface blocker. Before/after proposal and primary contract hashes/modes match. Producer feasibility is reviewed separately. This is not implementation, gate, provider-readiness or live acceptance. Accepted application tooling and old QA ledger remain unchanged at the user-reported dd4cc57 publication.
Context: Sole QA retains live ownership without an assigned live task. Preserve fs44/meta88/data89, both acknowledged histories and reviewed G2 binary. E1/D5 integration and production handoff remain separate prerequisites. The fault proposal is not implemented evidence. This planning ledger was excluded from the published QA milestone.
Next: Coordinator combines this pinned consumer ACCEPT with the independent producer verdict before assigning implementation. Assign only the new controller/provider/worker/test/doc paths listed in the re-review below, against the exact v1 contract. Do not implement now. Any controller-visible revision needs a new hash and review. Independent hook/controller implementation review and separate sole-QA live/upgrade authorization remain mandatory.
Log:
- [2026-10-25] Completed full revised-candidate consumer review, sections 1–7 (all 513 lines), not just its disposition table. CONSUMER ACCEPT at 3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523/0664. Read-only after checks match all three before hashes, modes and byte counts. C1–C8 close for this consumer contract. Actual provider access/continuity, case reachability, producer feasibility and implementation correctness remain unverified. Only this ledger changed; no tests/builds, source/tool edits, live query, signal, mount, Git command or subagent.
- [2026-10-25] Started the assigned revised-proposal consumer re-review. Read this ledger first. Read-only before checks: proposal 3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523 (90651 bytes, 0664); primary recovery contract 7f54a3325c574bfdd1e6e201ada22e0b4a5114f6c165d9b92ca8e8fe4a1c9db3 (88965 bytes, 0664); parent persistence contract 63c8cfce32c8b9c0818ccbbe8512d35c444d954a9e0b00f608bd982e0190f262 (38763 bytes, 0664). No verification run or live query occurred. Producer feasibility is assigned separately.
- [2026-10-25] Created the only writable path for this assignment. No accepted source or old ledger changed. No source, test, build, live or Git command ran.
- [2026-10-25] Read the proposal, accepted R6 review and frozen controller/client/ownership/Ganesha interfaces. Read the reference map's request path and cap-flush facts. Found unknown-key discovery and startup ordering gaps, missing snapshot/item schemas, incompatible restart argv/cleanup, and cumulative FLUSH_ACK coverage. No daemon implementation was inspected or inferred.
- [2026-10-25] Completed the consumer plan and owner questions. Only this new ledger changed. No tests, builds, source/tool edits, live queries, mounts, signals, Git commands or subagents ran. Installed-peer trace/log availability and live feasibility remain unobserved.

The initial review and proposed API below are preserved as history. The final frozen-v1 re-review at the end supersedes their CHANGES REQUIRED verdict and provisional field/transition suggestions. Do not mix the initial API sketches with the accepted candidate.

## Initial verdict and retained scope

**CHANGES REQUIRED for the consumer interface.** Accept the proposed meanings of `ReplyBeforeTransport`, `FlushBeforeWorker` and `FlushAckBeforeTransport`, not an implemented API. Keep the off-by-default `g3-faults` feature, explicit socket option, checked CommitRef, exact key/attachment tags and selected-output suppression. B1 must mean **not handed to the worker**, not merely “backend not known to have started.” Do not pause an entire session for output cuts. Do not release a held command or saved response after a hit.

Application tooling is independently ACCEPTED at `current/sol-verification-review.md:359–419`. Its frozen source and `current/sol-verification.md` stay untouched. This plan is not part of that release package. Full G3, upgrade permission and installed-peer feasibility remain open.

## Frozen interfaces: reuse and limits

| Existing interface | Consumer use; required separation |
|---|---|
| `recovery_trial.Trial`, `process_identity`, `validate_target`, `failure_code`, `write_json` | Reuse ownership, filesystem/process checks, pidfd discipline, fsynced artifacts and exit classification. `Trial.preflight()` requires an active PID and mounted client. `Trial.run()` takes the lock after that preflight. A fault controller needs a separate startup stage under the same exclusive QA lock **before launching its daemon**. Do not weaken the existing preflight. |
| `ApplicationTrial.capacity_check`, `persistence_settings` | Reuse normalized config parsing and retained-generation checks for the selected daemon ID. Supply a case/restart/time budget, not an assumption that the old 128-record application estimate covers every new case. Check standby settings if one is explicitly selected. Pressure after admission is runtime failure. |
| `ApplicationTrial.start_daemon`, `adopt_active`, `signal_mds` | Existing restart argv has no fault socket. Existing failure path has no Armed/Hit handshake. Add a separate launcher that creates a fresh endpoint for **every** process and binds nonce/PID/start/owner before any cut. Do not modify application execution to guess the new API. Initially prefer explicit owned restart, not an unprepared external standby. |
| `ClientProcess.send/receive` | One pending application command; useful for a real blocked syscall. Its newline/128 KiB framing is not the new 16 KiB daemon protocol. Drain control events independently while a worker call waits. |
| `restart_history` record/load/verify, shared sequential version2, `g3_client.load_snapshot/compare`, `recovery_directory` | Reuse exact data/metadata/type/inode/root checks, no-follow FD handling and cumulative histories. Do not append new fault records to the strict accepted version2 client format. New case transcripts reference unchanged baseline manifests and add only the selected operation's typed evidence. Never derive the expected post-state from a recovery listing. |
| `capture_mount_identity`, `require_same_mount`, mount audits and cold comparison | Retain boot UUID + kernel UNIQ-ID, physical mount/type/root checks and the private namespace. Existing `noshare`/`nosharecache` options establish comparison isolation only. They do not bypass an audit or repair failed recovery. |
| `start_ganesha`, `capture_ganesha_evidence` | Reuse owned 12049 process/start/config/recovery-root checks and bounded log/tree retention. EVENT/NIV_EVENT logs and configured 90 s grace do **not** establish actual reclaim. Existing Ganesha gates remain PARTIAL. |

## Minimum new paths after authorization

All names below are proposals, not files to create during this assignment.

- `scripts/g3_fault_trial.py`: separate named gates, startup/restart/control state, ownership, cumulative baseline comparisons and per-case result aggregation.
- `scripts/g3_fault_protocol.py`: only the fixed v1 codec, bounded socket pump, typed transcript/snapshot validation and event/control ID checks. No generic admin/RPC framework.
- `scripts/g3_fault_client.py`: real POSIX calls through existing unmodified mounts; fsynced intent/return receipts, retained FDs and second-client changed-path work. No fabricated Ceph messages.
- `scripts/tests/test_g3_fault_controller.py`: later offline framing/state/ordering/cleanup tests with modeled native boundaries and real local worker/oracle I/O.
- `scripts/G3_FAULTS.md`: later frozen control schema, exact provider prerequisites and case matrix. Leave current application documentation/results unchanged.

Daemon changes belong to its owner: a private fault-state module and the proposal's existing synchronous seams, behind `g3-faults`/`cfg(test)`. E1's real `reply_target_ino` is the only identified cross-owner engine dependency. Do not infer target identity from inode order or path lookup.

## Blocking owner questions and minimum contract changes

### C1. Startup order, owner binding and pre-resend persistence

Proposal §4 says Hello precedes storage startup, but lists Ready after Hello and leaves snapshot delivery/admission ordering implicit. Existing active-PID startup cannot establish this boundary.

**Proposed order:** verified Unix connection -> initial `Ready(ControllerReady, owner=null, loaded_tail=null, process_nonce, pid, boot_id, bounds)` -> Hello -> checked native load -> snapshot Begin/Items/End -> controller fsync -> `SnapshotAck` -> client/recovery admission. Initial Ready is not Loaded or ACTIVE. Arm remains invalid until the loaded owner and exact current attachment are known. Pair SO_PEERCRED with independently bound PID/start/executable hash; the nonce alone is not process identity.

Add only `SnapshotAck {command_id, snapshot_id, end_event_id}`. Hold startup admission until this ACK, while bounded control/map/connection I/O remains runnable. Queuing End does not prove external persistence. On restart, Hello already contains the prior process's exact selected IDs/inodes, obtained from its fsynced Bound/Hit records. Automatic real-client resends may arrive, but must not mutate the checked image before this barrier.

**Daemon/analyst question:** confirm the exact startup point and whether initial Ready is unsolicited as above. Which existing D5 gate prevents OPEN/attachment, early flush/retry drain, eviction or checkpoint-visible recovery mutation until SnapshotAck? No new engine pause API is requested. A post-admission snapshot must never be labeled pre-resend.

### C2. Discover a new client's durable key without an auth-name wildcard

Hello cannot initially know a newly mounted kernel/FUSE client's SessionKey or attachment. Both can use `client.admin`; an auth fingerprint alone can collide with retained Closed admin sessions and exhaust eight keys. A mount UNIQ-ID is a host identity, not a wire SessionKey. UUID is not assumed available on every native client.

Reserve at most eight **discovery slots** in Hello without enumerating historical matches. Proposed narrow command: `ResolveSlot {command_id, slot_id, exact_authenticated_client_entity, auth_fingerprint, peer_instance_anchor}` -> `SessionBound {slot_id, key, attachment, lifecycle, event_id}`. Resolve only the current authenticated live attachment, never “first admin session.” Obtain the exact client entity/transport anchor from a provider tied to the owned mount/FUSE process. Reject ambiguity, changed attachment and excess scope. Thereafter Arm uses the exact returned key/token. On restart, old history probes use the exact prior key even if it is now Closed; a new attachment event supplies the current token.

**Analyst question:** name the real installed kernel/FUSE provider and exact fields that expose this client entity/anchor. If unavailable, specify one bounded **exact owned-path probe** alternative for a reserved slot: known auth fingerprint, exact op/base/raw-path hash, fresh/live attachment and one unique match. This is a normal filesystem probe, not a new client protocol or a global key wildcard. Do not implement either guessed discovery API. Define how Ganesha's successor requester binds to its reserved slot before RESET, while retaining the exact old target key. Discovery slots, exact watches and resolved keys must stay within the same eight-key budget; old Closed admin sessions must not be auto-expanded.

### C3. Define Snapshot Items and the only additional query

`LoadedSnapshot.item?` and Status do not specify enough data to assert restored dedupe or durable range state. Initial Hello also cannot name inodes created during later preparation.

Specify typed items for: session lifecycle/UUID/auth/addresses; exact request history Present/Absent with result/created_ino/reply_target_ino; exact flush history Present/Absent; and watched inode durable size/max_size/truncate sequence/writer ranges. Include snapshot owner, tail, checkpoint generation/coverage/trim, purpose and per-kind counts. Never invent an original commit sequence when checkpoint-only history lacks one; retain the prior external CommitRef.

Add a bounded `SnapshotRequest {command_id, expected_process, expected_owner, probes}` for already reserved/resolved scope. It gives a **quiescent baseline**, not a retrospective pre-resend image. Permit late-bound IDs/inodes only within 8 session/16 inode/32 request-or-flush tag reservations. Restart Hello supplies the exact accumulated old probes. No table scan, counter-only dedupe proof or silent probe eviction is acceptable.

**Daemon/analyst question:** freeze each Item's field/type/range and Present/Absent meaning, including missing inode versus absent completion and Closed versus unknown session. Confirm the stable durable read point for active baseline queries and startup images. Define one snapshot ordinal sequence with matching Begin/End IDs, counts and immutable owner/tail; a missing/duplicate/reordered Item invalidates it.

### C4. Match actual wire paths, operations and flush inputs

The selector's singular `path_base/path_fingerprint` and `flags_mask/value` are ambiguous. Freeze `path_field=path|path2`, base inode, raw byte length and `SHA256(run_salt || raw filepath bytes)`. Hash decoded raw filepath bytes only; do not hash printed `#ino/name`, normalize slashes, decode/re-encode names or substitute a local absolute path. Distinguish request-head flag masks from CREATE/OPEN argument flag masks.

Reference facts: kernel paths can be `(ino, empty)` or `(parent_ino, basename)`; FUSE/libcephfs usually uses inode-relative paths. RENAME's first path is destination and second is source. SYMLINK path2 is target content, not the new name. The kernel can strip O_TRUNC from CREATE and issue a separate operation. An application `open()` name therefore does not prove a particular wire op or path. See `kclient-expectations.md:290–345` and `libcephfs-fuse-ganesha-expectations.md:193–238`.

Add `caps_op` and relevant cap-message flags to the Flush InputTag. `NextDirtyFlush` must accept the real dirty UPDATE form: libcephfs uses UPDATE, not a fabricated FLUSH opcode (`libcephfs-fuse-ganesha-expectations.md:281–291`). Bind actual nonzero flush tid and all proposed cap/dirty/barrier fields; request tids and flush tids remain distinct domains.

**Analyst question:** freeze these selector fields and the provider's actual wire-name representation for each client. Calibration must use observed ordinary inputs on an isolated owned parent; it must not assume a future tid or broaden a mismatch into a wildcard. No match/failed mutation/no-record OPEN is an unestablished case, not a cut. Do not add client protocol support to manufacture a reachable case.

### C5. External return/ACK evidence and cumulative flush coverage

The hook is server evidence, not peer receipt. New worker receipts need `{case_id, call_id, client_slot, pid/start, mount_identity, action, raw-name fingerprint, intent/return, monotonic_ns, result/errno, relevant inode/size/data digest}`. Fsync externally before reporting a return. Keep the original call pending across failure; do not retry it in Python after EINTR. A later new application call/tid is separate. One worker sequence number must never be labeled a Ceph tid.

Join independent peer request-send/resend/reply and cap-send/ACK summaries to Bound's actual key/tid/op/attachment. Run an asynchronous control drain while the selected syscall blocks. Before crash, require the selected response to remain unaccepted and the worker to have no return receipt. Retain a complete receipt stream through return/exit; lack of a stdout line alone proves nothing. On recovery, require a real same-tid ordinary retry/resend and the corresponding peer outcome, not just eventual fsync success.

**New consumer hazard:** later FLUSH_ACKs can complete earlier flush tids (`kclient-expectations.md:699`; `libcephfs-fuse-ganesha-expectations.md:291`). “Suppress the same logical ACK” does not alone preserve B2. Use distinct inodes for the selected flush and the other N3 workload. Add a scoped accepted-ACK coverage observation/counter for the watched inode/tid; any different ACK that can complete the selected flush before crash invalidates the trial. Do not silently suppress all traffic or relabel another tid as the selected one.

**Analyst/daemon questions:** give actual installed trace/log provider names, fields and loss detection for both unmodified clients. Freeze cumulative-ACK invalidation and the retry/dedupe outcome records needed to prove no second mutation/allocation/truncate. If independent send/receipt or syscall correlation cannot be obtained, declare that qualification prerequisite missing. No such provider was verified in this review.

### C6. Frame, address and event bounds must be encodable

Keep u32 little-endian length + strict UTF-8 JSON v1, 16 KiB maximum frame, 8 incoming controls, 64 outgoing slots with 4 reserved, one active arm/retained command/cache/hit, 8 keys, 16 inodes and 32 request/flush tags. Reject oversize before allocation; reject malformed types, numeric lookalikes, conflicting control IDs and incomplete frames. Fix enum and integer encodings before either side implements.

**Analyst/daemon question:** provide a maximum-size legal Ready/Hit/Status/Item schema, including all 64 exact addresses/nonces and the common envelope. “Fail on oversize” is insufficient if a supported legal maximum cannot fit. Prefer one bounded typed address entry per sequenced snapshot Item plus an immutable address-set reference/count in Hit/Status; stream under backpressure without allowing startup admission. Do not enqueue an entire expanded snapshot into 60 ordinary slots or truncate an address union. Define the legal union bound explicitly. No arbitrary textual addresses, payload bodies or credentials enter the protocol.

Require per-process contiguous scoped event IDs, per-snapshot ordinals/counts and exact process/run tags. Duplicate-response retransmission may repeat its event ID but cannot create an apparent gap/new observation. Explicit suppression counters are not dropped events. On any loss/overflow/partial snapshot, invalidate rather than restart collection and accept only the later suffix.

### C7. Expected crash and ordinary shutdown are separate protocol outcomes

The proposal makes EOF fatal and requires the controller to remain connected. Existing application success leaves MDS/mounts running; generic cleanup can TERM/KILL a child without observer coordination. Neither behavior is a drop-in fault-mode lifecycle.

Propose `ExpectExit {command_id, expected_process, mode=CrashAfterConfirmedHit|NormalStop, hit_id?}` with an acknowledgment, plus a final `ProcessExiting {reason,last_event_id,held_command,final_positions}` for orderly exit. This authorizes no signal itself. NormalStop is invalid with a bound/hit cut or held Append. It never resumes/sends a cut. The controller stays connected through client close, identity-checked normal unmount, daemon drain and final exit; normal service cannot continue after detach.

**Daemon/analyst question:** confirm this minimal exit handshake or an equivalent explicit terminal sequence. EOF after an acknowledged stop intent is accepted only after the exact owned process exits with the expected status; EOF while it is alive or before the intended kill is failure. CrashAfterConfirmedHit also requires fsynced Hit/AckHit/CutConfirmed and the second-client receipt when selected. Unsolicited EOF, invalid input, owner/attachment loss, deadline, NotApplicable or Invalid after admission is runtime 1, never clean shutdown/PASS. B1's retained command must not flush on any shutdown path. Busy/unvalidated mounts are not force-detached; cleanup failure cannot be turned into success or an untracked observer disconnect.

### C8. Installed Ganesha/NFS evidence provider is a prerequisite

The installed package is documented as Ganesha 6.5, while reference FSAL facts are pinned to V15.7. The accepted helper uses `fs` recovery state, stable node/server owner/scope, 90 s configured grace and EVENT/NIV_EVENT logs on 12049. These facts do not establish that 6.5 exports the required grace/RESET/lock evidence.

**Analyst/daemon question:** name the 6.5/libcephfs log components/fields or a scoped decoded-protocol provider for the exact N1–N4 records below. Include NFS file-handle-to-Ceph-inode correlation and NFS client/session/open-owner/lock-owner identifiers. Do not assume NFS `st_ino`, mount count or two worker processes uniquely identify two wire clients or one native session. No credential changes, system-service queries/actions on 2049, or raw credential-bearing packet archive is part of this plan.

## Concrete case execution after contract freeze

### Common sequence

1. Obtain publication completion, accepted D5/E1/E2/D6 integration, independently reviewed hooks, exact feature binary/config provenance and explicit sole-QA live permission. Use a separate later build target; never overwrite reviewed G2 binary `330cf3ee…`.
2. Hold the existing exclusive QA lock. Check fixed fsid `3ceca2c6-851f-41cf-b538-1cfded4f1e82`, filesystem `a`/44, metadata88/data89, selected PID/start/listener/hash/namespace and permitted mounts. Preserve both acknowledged `restart-history-1629333-1792700434474727536` and `restart-history-2663893-1792950162527640133` namespaces and their original/retry evidence. No newfs/reset, metadata deletion, OSD stop or implicit clean-session upgrade.
3. Preflight peer observers, scope/frame capacity and actual checkpoint/time/record budget. New run namespace/artifacts only. Launch an owned feature daemon with a fresh absolute 0700 parent/0600 endpoint; never unlink a stale socket. Every process gets a fresh nonce/endpoint. Retain the same run ID/salt and accumulated exact old history probes through its restarts. Do not forget probes to fit the bound; split the planned suite into explicit smaller trials before admission.
4. Complete Ready/Hello/persisted startup snapshot. Bind actual peer slots/attachments, prepare known state through unchanged clients, collect a quiescent baseline snapshot and run exact state oracles. Capture physical root/FD identities before any mutation. NFS keeps its explicit no-xattr profile; native xattrs remain mandatory.
5. Fsync case intent and exact selector. Wait for owner/attachment-matching Armed **before** issuing the real selected call. Persist Bound's actual tid. Validate native commit/handoff/output positions, persist Hit, send AckHit and persist CutConfirmed. The reader must continue consuming events during slow external fsync and worker waits. No sleep selects a cut.
6. Perform required competitor work, then recheck identities/capacity and expected exit. Kill only the bound daemon via its pidfd after the exact evidence exists. Retain mounted kernel/FUSE processes. Restart through a new endpoint; Hello includes old RequestId/FlushId/inode probes. Persist pre-resend Present/Absent image before admission. Later Arm binds the **new** owner/current attachment, never the dead token.
7. Observe genuine same-key/tid/op retry or flush resend. Join restored history, dedupe/commit decision, actual accepted output and peer syscall/ACK evidence. Preserve optional fresh writer-range transactions under their own CommitRef. Compare every retained baseline and prior case namespace through mounted and independent cold views before any healing write. Finalize only with complete transcripts and expected shutdown/cleanup.

### Named native gates

| Gate/cases | Exact execution and dependency |
|---|---|
| `reply-before-transport`, A0 | Kernel and FUSE separately. New creator reaches checked commit and SAFE output suppression. No external original return before kill. Restart snapshot has original completion/target; same ordinary tid retry does not create twice. |
| Same gate, A1/A2/A3 | After selected Hit, the other independent client renames, replaces or unlinks the original. Require its own selected input/CommitRef, accepted SAFE output **and** external syscall + durability receipt before kill. Check original/replacement inode distinction and exact names/trace. If cap revokes or the waiting real syscall prevent that sequence, stop as unestablished; no synthetic client or broad output pause. Directory/symlink/O_EXCL variants need their actual reachable wire ops. |
| Same gate, A4 | Existing-inode CREATE/OPEN mutation: nonempty and zero-size O_TRUNC, range-only and combined cases. Wait for implemented/reviewed E1 target identity and actual client op/argument mapping. Never infer target from a later lookup or assume open() is one CREATE. |
| `flush-before-worker`, B1 | Bind real dirty UPDATE/FLUSH. Require q=d+1, exact owner/operation/batch, command retained, worker_enqueued=false, append_started=false and no accepted ACK. After restart, selected flush completion is Absent and prior durable scalar/range state remains before resend. Same-tid resend commits once. StartedOutcomeUnknown is not B1. File data already written to OSDs need not be absent. |
| `flush-ack-before-transport`, B2 | Checked flush commit, selected ACK suppressed, push_seq unchanged, no covering accepted ACK before crash. Loaded completion Present; real same-tid resend dedupes, then accepted ACK/barriers/grants and external fsync outcome are checked. No second flush mutation. |
| B3 on B1/B2 machinery | Add independently observed competing rights and retained job progress. Require early resend/control drain and durable completion before grants/TRUNC/output release. Depends on real D5/E2 integration, not a new peer protocol. |
| `reconnect-open`, R0 | Observe actual final-chunk receive/validation, attachment reconstruction and **reconnect-purpose** OPEN transport acceptance/seq. Preserve separate requested/confirmed phase events and application first-success time. Same host boot/clock basis is mandatory for cross-process elapsed times; queue acceptance is not peer receipt. |

SAFE/FLUSH_ACK suppression never invents transport success or completes a recovery-output drain. Keep the external original CommitRef when trim removes its WAL record. Fresh application retries after EINTR and a fresh Ganesha client identity are not same-tid evidence.

## N1–N4: evidence and explicit limits

Use only the owned foreground helper on 127.0.0.1:12049 and retained recovery directory/config. Later evidence retrieval must whitelist metadata from private owned-process logs or an authorized live decode stream restricted to that endpoint. Store operation/result/epoch/state/fingerprints, not raw RPC/auth headers, keys, tickets or unfiltered captures. Record provider versions, dropped-record counters, log inode/offset continuity and complete grace windows. EVENT-level slices remain useful artifacts but are not presumed sufficient. No collection was performed here.

| Gate | Required evidence; unsupported inference |
|---|---|
| `ganesha-reset`, N1 | MDS remains alive; old UUID/key Open and NFS open/lock state are proved before killing only the test helper. Successor requester has a new native identity. Observe RESET selecting the exact old key/address union -> checked fence/map barrier -> durable Closing -> cleanup receipt -> Closed -> accepted reclaim reply with nonempty old addresses/post-fence epoch -> libcephfs/FSAL outcome and new UUID OPEN. If target already expired/closed, N1 was not established. No FINISH packet is required after successful RESET. |
| `ganesha-absent-reset`, N2 | Joint failure, but restart MDS while Ganesha stays absent. Initial snapshot proves old persisted session/UUID. Wait for observed missing-key deadline, fencing, durable Closed and confirmed recovery progress, with no additional reclaim hold. Then start successor Ganesha. Expect wire result 0/epoch0/empty addresses, libcephfs ENOENT and FSAL continuation. Never confuse this intentional absence with failed UUID persistence. The application controller's immediate helper restart is not this schedule. |
| `ganesha-shared-flush`, N3 | Keep the same Ganesha PID/start/libcephfs connection and durable SessionKey alive across **MDS-only** B1/B2. Map two labeled NFS workloads on distinct inodes through RPC/FSAL file-handle and path fingerprints to that same key; record actual NFS client/session and lock owners. Prove selected outstanding flush and the other outstanding operation at failure. Two workers/mounts alone prove neither independent NFS clients nor this correlation. Ganesha restart creates a different identity and cannot prove this same-tid dedupe case. |
| `ganesha-lock-gap`, N4 | After joint missing-session cleanup and before restarting Ganesha, schedule an independent native lock contender. Record its actual acquisition/denial and later NFS reclaim contention under the accepted gap policy. Do not claim transparent preservation of old native-vs-NFS locks. An unobserved gap stays explicitly unexercised. Analyst must specify allowed 6.5 reclaim/lock results, not accept any error as expected. |

For NFS grace qualification, retain actual old OPEN/LOCK state, grace-enter/exit evidence, reclaim OPEN with the applicable CLAIM_PREVIOUS form, reclaim LOCK where required, RECLAIM_COMPLETE for the negotiated NFSv4.1 sequence, and new-client/owner GRACE/conflict outcomes. Preserve original FDs/file handles, lock exclusion and acknowledged bytes before any write. Correlate export100, path/file-handle fingerprint, NFS client/session/owner identifiers, helper process and native key. Stable config, elapsed 90 s, RESET success or eventual application I/O alone cannot prove NFS grace. Decoded protocol can prove on-wire operations/results; peer logs can explain local grace/FSAL behavior only when exact fields and complete intervals are available. Missing providers block the corresponding qualification.

## Results and execution dependencies

- Frozen application gates remain unchanged: native >=2 rounds 0/APPLICATION_PASS; one round or Ganesha 77/PARTIAL; prerequisite 2; runtime/evidence failure 1 without success result. Existing `reply-loss`/`flush-ack-loss` placeholders remain prerequisite 2 until separately changed after release/authorization.
- New names above write separate case artifacts. Proposed 0 is **FAULT_CASE_PASS for that named case only**, with exact established predicates; `full_g3_qualified=false` until the required complete matrix is independently accepted. Never overwrite/promote the baseline result or silently mark excluded NFS xattrs/native zero-link behavior passed.
- Missing required hook/provider/budget before admission -> 2. Malformed frames, missing snapshot/peer evidence, NotApplicable/Invalid, wrong identities, unexpected EOF, deadline/capacity/I/O/cancellation after admission -> 1, no success result. Disabled readiness or an explicitly selected preparation-only run may return 77/PARTIAL with missing predicates listed. Do not convert an admitted failed fault case to successful partial coverage.

**Assignments, in order:**

1. Protocol analyst + daemon owner resolve C1–C8 and freeze one concrete v1 schema, provider matrix and expected terminal sequence. Supply maximum-bound encoded examples later under an authorized verification task. No parallel implementation against guesses.
2. After publication and explicit coding assignment, daemon owner completes accepted D5/E1 joins and implements the agreed feature-only cuts/observer. QA can then implement the new codec/worker/controller paths against that same frozen schema and offline fixtures in parallel. Existing accepted paths remain unchanged unless separately released.
3. Independent reviewers check both pump handoffs, storage-versus-transport positions, callback/funnel ordering, two-client progress, cumulative ACK invalidation, scope isolation, startup/snapshot ordering, full bounds, feature-off behavior and every EOF/shutdown path. This plan is not that review.
4. Only after runtime/hook/controller acceptance, identify a separate feature binary and obtain explicit sole-QA live/upgrade authorization. Run named native cases first; qualify N1–N4 only with established 6.5/NFS evidence providers. No force/unmount bypass, fabricated retry, new peer protocol, reformat or credential/system2049 action can obtain PASS.

Only this new ledger is writable now. Sole QA live ownership remains assigned but inactive. The pending release must exclude this planning ledger and retain every accepted source/old-ledger byte.

## Frozen v1 consumer re-review — 2026-10-25

### Verdict and exact inputs

**CONSUMER ACCEPT frozen v1 for implementation. No remaining C1–C8 consumer-interface blocker.** This verdict applies only to the exact proposal below. It does not accept daemon feasibility or implementation, a decoder/provider, a fault gate, live feasibility or full G3. The daemon owner reviews producer feasibility separately; no runtime internals were inspected for this re-review.

Read all candidate sections: source seams, types, startup/discovery/snapshots, selectors, sealing/stopping, observations, independent providers, N1–N4 and later verification. The author's closure table alone was not the basis for acceptance.

Read-only SHA256/lstat checks before and after the full review matched:

| Frozen primary input | SHA256, before = after | Mode, before = after | Bytes |
|---|---|---|---|
| `current/astra-g3-fault-interface.md` | `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523` | `0664` | 90651 |
| `design/recovery-contract.md` (primary recovery contract) | `7f54a3325c574bfdd1e6e201ada22e0b4a5114f6c165d9b92ca8e8fe4a1c9db3` | `0664` | 88965 |
| `design/persistence-contract.md` (parent contract) | `63c8cfce32c8b9c0818ccbbe8512d35c444d954a9e0b00f608bd982e0190f262` | `0664` | 38763 |

All three were regular files. These were local read/hash/mode checks, not tests or live queries. The dd4cc57 application-tooling publication is the user's supplied release fact; no Git verification ran. No accepted source, contract, proposal or old QA ledger was edited.

### C1–C8 closure from the full contract

| ID | Consumer disposition and required use |
|---|---|
| C1 | **Closed**, §§4.1–4.3. Unsolicited ControllerReady has no owner/tail; Hello is possible before native startup. Checked restore/allowed initial-trim receipt precede an immutable StartupPreResend export. The controller fsyncs the complete End prefix before SnapshotAck; only SnapshotReleased permits execution. No attachment or reconnect is needed to query old exact history. Arm waits for Serving, the loaded owner and a current bound attachment. This removes the Ready/Hello/reconnect cycle. Producer latch placement is the other reviewer's lane. |
| C2 | **Closed**, §4.4. Eight reservations include unresolved slots and aliases. Exact authenticated MKDIR probes bind only live post-installation input; BindAck joins the independent owned-peer proof before releasing that input. No auth-name table expansion or Closed-session wildcard exists. Reset/UuidOpen discovery is preinstalled before launching a successor, with predecessor linkage and explicit same-key aliases. After MDS restart, exact retained keys get new AttachmentBound tokens; an old owner/arm cannot transfer. |
| C3 | **Closed**, §4.3/§5.4. Typed session/history/inode/range/address items distinguish Present, Closed and explicit absence reasons. NoEntry is not automatically “never committed”; the controller must use the original proof and floors. Read errors cannot become absence. QuiescentBaseline queries are distinct from StartupPreResend and also require persisted release. Creator inode reservations resolve from the actual completion, not a later listing. Counts/ordinals/StatePoint bind the entire image. |
| C4 | **Closed**, §4.5. Select by exact binary path field/base/length/salt hash and separate head versus CEPH_O_* masks, after ordinary-client calibration. Tid.Next binds the unpredictable actual input tid. Both paths remain visible with their real roles. Actual dirty UPDATE/FLUSH is supported; fabricated FLUSH, guessed CREATE/O_TRUNC mappings and wire REPLAY are excluded. A mismatch or nonmutating candidate fails instead of widening/reselecting. |
| C5 | **Closed**, §§4.6,5.1–5.4,6.1. Selected reply suppression leaves cap revokes and second-client operations runnable. Competitor input/commit/accepted reply plus external durability/return receipts precede Sealing. B1 proves no worker handoff, not merely missing callback. B2 suppresses all cumulative covering ACKs for the selected key/rank/inode, regardless of cap-id/dirty-mask differences; any actual covering acceptance is sticky Invalid. The final immutable ExitReady closes the missing-invalidation-tail race. Loss-free peer processing and original syscall return remain separate required proof levels. |
| C6 | **Closed**, §§3,4.1–4.3,5.4. Strict v1 scalar/union/field rules, 1–16384-byte frames, one pending command, exact cached-response replay and contiguous events are specified. Sixty ordinary event slots cannot be used as a whole-export buffer. Full 64-entry total address unions use indexed entries and binary count/digest verification; snapshot exports can stream all 696 items. No truncation, scope recycling or suffix-only recovery is allowed. Legal maximum encoding remains a required later implementation check, not a claimed result here. |
| C7 | **Closed**, §4.6. CrashAfterConfirmedHit requires Status-prefix persistence, final ExitReady persistence and the exact owned SIGKILL status before EOF is expected. A new observing recovery process uses NormalStop: StopAccepted, real worker close/normal unmount/drain, final ProcessExiting, fsynced ExitAck and exit 0. The injected process cannot resume or use this path. Unsolicited EOF, incomplete frame, missing final event/ACK, wrong status or failed cleanup cannot pass. |
| C8 | **Closed as an evidence/prerequisite contract**, §6.2–6.3. Version-pinned 6.5 facts replace the old map assumption without claiming installed equivalence or readiness. Successful RESET needs no Ceph FINISH; NFS RECLAIM_COMPLETE is distinct. Decoded full handles join export100/fscid44/inode to the owned native stream/key; two NFS clients require distinct on-wire identities, not two worker/mount labels. N1 found-target reclaim, N2 intentionally absent target, N3 same surviving Ganesha identity and N4 exact native-lock conflict have separate success predicates. |

### Controller schedule and terminal-prefix interpretation

The controller can implement this sequence without predicting tids or waiting for reconnect to release the restored image:

1. Hold the existing QA lock and bind the owned feature process/endpoint/uid/PID/start/hash/pidfd. Read initial Ready, then send Hello with reserved discovery/creator slots. Across restarts use a fresh private endpoint/process nonce but the same run ID/salt and accumulated exact old probes.
2. Persist StartupPreResend Begin/Items/End, validate all counts/digests/points and send SnapshotAck. This needs only the restored exact keys, not their new attachments. Discover live peers with independent wire/process/mount proof and BindAck. Prepare the owned parent/state and take a QuiescentBaseline query when needed.
3. Persist Armed before submitting the selected raw-libc worker call. Tid.Next chooses only the first exact eligible real input; Bound supplies its actual identity. Persist Hit/AckHit/CutConfirmed while the worker remains alive. For A1–A3, complete the other client's rename/replacement/unlink and its independent receipt before requesting a seal. No global freeze occurs during that mutation.
4. After external prerequisites, obtain/fsync the Status response and send ExpectExit with that exact persisted prefix/proof manifest. Sealing may drain already admitted work under the same guards. **ExitReady's envelope event ID is the exact last successful sealed sequence**, not Hit, CutConfirmed, Status or its opaque seal ID. Fsync the full prefix through ExitReady. No engine/session change, worker handoff or client transport send follows it. Only bounded control/monitor handling remains; an unsuccessful exit is not the expected crash.
5. Signal only that bound pidfd before its deadline, record the signal and require actual SIGKILL wait status. Keep the control reader through EOF and all peer/call observers through recovery and original returns. A queued invalidation before the seal must appear in the complete prefix or prevent sealing. An unknown/lost sender or peer interval cannot prove a negative ACK. Independent contradictory evidence overrides server counters.
6. Start a fresh observer process with old exact probes. Persist and acknowledge its pre-resend snapshot before normal recovery execution. Check new owner/attachment, genuine same-key/tid/op resend, exact history decision, new records (if any), accepted response, peer processing and original syscall return. R0's reconnect-purpose OPEN transport timestamp remains distinct from queueing, network delivery, peer processing and application first success.
7. Run unchanged exact state oracles before healing writes and cumulatively across cases. Finish a non-injected recovery observer through acknowledged NormalStop. A successful process shutdown does not convert an incomplete fault case into PASS.

Command-response duplicates repeat only the identical cached event and do not advance the rolling prefix hash. Each other scoped event advances the sequence/hash. Pin SnapshotEnd, SessionBound, Hit, the exit Status and ProcessExiting at their specified receipts. Reject reordered/extra/missing items, stale owner/attachment, wrong epoch chain, unresolved references, malformed absence, partial frame or EOF. Address references become usable only after their full count/order/hash proof. No later valid snapshot can erase an incomplete earlier proof.

### Exact oracle reuse and remaining operational prerequisites

Snapshots supply durable completion/range/lifecycle facts, not a new namespace/data oracle. Keep `restart_history` and sequential version2 comparisons, R6-validated `g3_client.load_snapshot/compare`, physical-directory guards and the full surviving-FD inode/size/SHA256/link-count checks. The proposal does not require weakening those schemas. Keep original acknowledged-byte verification before append/mutation, including unlinked-open FDs. Store the selected operation's typed case transcript separately; never manufacture an application ACK from a server commit or learn an expected inode/data image from a post-recovery listing.

WireCeph1 must be an independent bounded passive metadata parser; stock tshark is explicitly insufficient for all required Ceph fields. CallTrace plus fsynced raw-libc worker receipts prove the original syscall interval/return, not stdout silence or a Python EINTR retry. KernelPeer7/Libcephfs20 joins need real independent fields, processing evidence and continuity. The final loss-free trace must validate the pre-kill negative interval retrospectively; ExitReady alone is not peer evidence. These predicates are fixed even if a provider is unavailable.

For Ganesha, the proposal correctly distinguishes V6.5 FSAL continuation after an error from real RESET success. N1 requires found-target cleanup/nonempty reply and actual peer processing. N2 uses B2 only to establish the joint-crash point, then requires the old UUID at startup, missing-member cleanup while Ganesha remains absent, and the later empty-address/epoch0/ENOENT outcome; it is not an old-client B2 dedupe claim. N3 retains the Ganesha PID/client entity/key across MDS-only failure; it does not require an unchanged TCP connection or daemon attachment. N4 requires the specific overlapping native-lock acquisition and NFS4ERR_DENIED, not NO_GRACE or arbitrary errors. Configured grace or export readiness alone proves none of these outcomes.

Access, package-emitted fields/patch behavior, loss-free capture, distinct NFS identities and reachable real-peer schedules are **later named prerequisite probes**, not residual interface blockers. Missing preparation prerequisites return 2; explicitly preparation-only output may return 77/PARTIAL. Loss/ambiguity/unreachable sequence/wrong outcome after admission returns 1 with no successful case result. No runtime provider or case feasibility was observed in this review. Credentials and system Ganesha/2049 remain outside scope.

### Proposed coordinator assignment after producer handoff

Do not implement now. After the coordinator receives the independent producer verdict and explicitly assigns coding, use the exact accepted v1 contract, not the initial sketches above:

- `scripts/g3_fault_protocol.py`: strict control/event codec, bounded async drain, command/event IDs, persisted-prefix receipts, streamed address/snapshot verification and crash/normal-stop states.
- `scripts/g3_fault_trial.py`: separate named A/B/R/N case gates; fresh process endpoints, same-run restart probes, real arm/hit/seal schedule, unchanged ownership/capacity guards and per-case outcomes.
- `scripts/g3_fault_client.py`: ordinary mounted POSIX operations via raw libc, original-call/FD retention, durable external receipts and second-client work. Reuse exact accepted baseline/FD oracles; do not alter their formats.
- `scripts/g3_fault_peer.py`: the contract's bounded passive WireCeph1/required WireNfs41 metadata decoding, provider continuity and independent process/mount/RPC/native identity joins. No new Ceph client/server protocol support, credentials or raw packet archive.
- `scripts/tests/test_g3_fault_controller.py` and `scripts/tests/test_g3_fault_peer.py`: later offline codec, full-bound/stream/selector/seal/stop/peer-loss fixtures, real disposable worker/oracle checks and modeled native boundaries.
- `scripts/G3_FAULTS.md`: exact v1 usage, provider preparation requirements and case/result scope. Published application entry points, results and old ledger stay unchanged unless separately released.

Daemon source adoption and hook wiring depend on the separately accepted E1/D5 production integration and producer review. QA codec/worker/controller work may proceed in parallel only against the shared pinned contract after explicit assignment. Consumer-visible changes require a new proposal hash and review. Later independent implementation reviews must cover the maximum frame/item/address shapes and every failure/terminal path; this source-only review ran no fixtures or tests.

After implementation acceptance, identify a separate feature binary without overwriting the reviewed G2 binary. Parent must separately authorize any same-filesystem upgrade and sole-QA live cases. Preserve fs44/meta88/data89, both acknowledged namespaces and all old evidence. A named gate may report 0/FAULT_CASE_PASS only for its established predicates; full_g3_qualified stays false. Published APPLICATION_PASS/PARTIAL behavior and prior G2 acceptance remain unchanged. Full G3 stays pending.

Only `current/sol-g3-fault-controller-plan.md` changed in this re-review. No source/tool edit, test, build, live query, mount, process signal, Git operation, commit or subagent ran.
