# sol-contract-review
Task: Independently review the completed G0 persistence contract against engine, store, and Objecter APIs. Report actionable correctness issues and blocking implementation ambiguities only.
Status: Re-review complete. Verdict: ACCEPT. The amended contract closes all five original findings. No production edits, tests, live operations, commits, or child agents.
Context: Reviewed repository guidance, approved workplan, both completed design documents, G3 reference audit, and engine/store/Objecter boundary source. No implementation diffs were inspected. Findings concern target contracts, not the progress of implementation lanes.
Next: Parent can freeze the amended ownership/bootstrap and engine interfaces and release dependent implementation lanes. Verify the required fault and recovery tests during G1-G3 integration.
Log:
- [2026-10-22] Created review ledger before work.
- [2026-10-22] Finished source and crash-boundary review. No broad tests were needed for this document review.
- [2026-10-22] Re-read the amended persistence contract and store design against all five findings. ACCEPT. Reviewed continuation/commit ordering, interrupted bootstrap, separate authority acquisition, session-history join, and WAL reconstruction. No further safety or blocking contradiction found.

## Re-review verdict: ACCEPT

Reviewed amended design/persistence-contract.md (305 lines) and design/store.md (69 lines). All five findings below are closed for contract freeze. They remain below as historical review context, not outstanding implementation blockers.

| Original finding | Amendment that closes it |
|---|---|
| Prior-owner fencing evidence | Contract section 5, lines 149-176, and section 7, line 280: durable writer address vectors, read-only inspection, union fencing, explicit zero-epoch handling, and predecessor evidence retained across partial acquisition. |
| Bootstrap enumeration and interruption | Contract section 5, lines 178-220: assigned native bounded PG listing, all namespaces, map-change restart, explicit empty initial allowlist, persisted deterministic plan, separate format CAS, interrupted Preparing takeover, and Ready reconciliation. |
| Rank-session guard acquisition and recovery join | Contract section 7, lines 250-291: separate session CAS, preserved allocation counter and records, uncertain-outcome reconciliation, ordered acquisition, checked lifecycle/history join, and durable terminal cleanup proof. |
| One-record continuation boundary | Contract section 2, lines 43-68: one-use RecordPermit, Engine::step, explicit wait/resume states, internal cursors, no submissions from journal_event, and SessionCleanupCommitted only after the terminal record commits. |
| Checkpoint-only persistent fields | Contract section 3, lines 114-118: persistent InodeCore fields, versioned full WAL images, explicit pre-unlink path capture, stable identity rules, and checkpoint-before-unlink recovery tests. |

The revised boundaries are implementable without cross-object atomicity. A persisted bootstrap plan supplies the initial seed before dependencies exist. Each mutable authority has its own acquisition guard. Recovery can discover and fence different writers after partial acquisition. It loads and joins histories only after both shard/session guards succeed. Commit callbacks materialize output without admitting a second transaction. The next admitted step resumes remaining cleanup work. The terminal cleanup receipt survives checkpoint and replay.

The native pool-listing API is new platform work, not an existing capability. The contract now says so and assigns it. Its absence during implementation must remain a bootstrap failure, not a probe-only fallback. The initial empty allowlist and retained checkpoint generations are explicit first-milestone limits, not hidden claims of broader service.

No further required changes. This verdict approves the target design only. It does not claim implementation completion, passing tests, fresh G0 baseline evidence, or G2/G3 recovery qualification. The required fault tests and current/sol-recovery-review.md still govern integration acceptance.

## Original review verdict: CHANGES REQUIRED (superseded)

The selected model is retained. Do not redesign the WAL or add independent streams.

### 1. High: prior-owner fencing lacks a recoverable evidence source

Contract: persistence-contract.md sections 5 and 8, lines 120-133 and 171-175; store.md sections 2 and 6.

The daemon must validate prior-owner fencing before `open`. The zero-failure-epoch warning is correct. However, `OwnerToken` stores only epoch/rank/incarnation/gid. Neither `OpenedShard` nor another specified durable record preserves the previous daemon's messenger address/nonce. A cold replacement cannot recover that address from a current MDSMap after the previous `MdsInfo` disappears. Rank-session peer addresses identify clients, not the previous MDS.

This leaves no defined way to satisfy the required zero-epoch takeover path. Waiting for a latest map alone does not establish which address must be blocklisted. See current/sol-recovery-review.md section 1, especially the standby-replay-removal case.

Required resolution: persist the previous writer's beaconed address vector/nonce with authoritative owner state. Expose a read-only control inspection before acquisition. Define the daemon proof: identify predecessor, obtain or verify its blocklist entry, wait for the map that includes it, raise the barrier, then acquire/load. Define the no-predecessor bootstrap case separately. Fail closed on missing evidence. Retain this evidence across an interrupted acquisition.

API boundary: store owner/control metadata and pre-open inspection -> daemon -> `MonClient::blocklist`, latest-map query, `Objecter::wait_for_map_result`, `OsdMap::is_blocklisted`. `MdsMap.last_failure_osd_epoch == 0` cannot replace this chain.

### 2. High: bootstrap depends on an absent platform operation and does not define interrupted-writer takeover

Contract: persistence-contract.md section 5, lines 127 and 135; store.md section 2, line 27.

Bootstrap must enumerate the pool before its first write. `Objecter` exposes object-targeted `submit`/`operate`; `ObjectOperation::omap_get_vals` enumerates keys in one object, not objects in a pool. No pool/PG object-enumeration API exists at this boundary. The platform lane needs an explicit assignment and API. Replacing enumeration with probes for a few C++ names would violate the contract.

The restart path is also incomplete. Bootstrap takes a new `owner` and `initial_checkpoint`, but retries must verify identical bootstrap identity/content and resume Preparing. A crash can leave the marker and control under the previous owner, or immutable root checkpoint names under its epoch. The contract does not say how the replacement recovers the bootstrap id/initial state, acquires Preparing authority, or guards publication of Ready. A comparison on the WAL cannot atomically guard a write to `rmds.format`.

Required resolution: assign and specify bounded native pool enumeration, including namespace scope and map-change/error handling. State the initial explicit shared-object allowlist. Define a durable Preparing descriptor and CAS/reconciliation protocol on the format object. Preserve its bootstrap id and root/allocation identity on replacement. Specify which same-object guard authorizes Ready publication after all required objects are verified. Reject concurrent initialization rather than infer cross-object atomicity.

API boundary: platform enumeration -> store bootstrap; store inspection/resume descriptor -> daemon bootstrap orchestration. This is G2-enabling contract work, not a requirement to implement enumeration during review.

### 3. High: rank-session guard acquisition and cross-object recovery join are unspecified

Contract: persistence-contract.md sections 5 and 7, lines 133 and 161-167.

The document says to acquire the rank-session guard separately. It defines guarded session mutations but no acquisition operation, predecessor check, unknown-outcome reconciliation, or recovery order relative to shard ownership. The current engine stores sessions by `ClientId`; its runtime `epoch` is not a durable rank guard. `open_session_key` is specified only as an open operation, not as a checked join between restored shard history and rank lifecycle.

Crash case: a replacement changes the shard owner, then dies before changing the session guard. Another replacement must acquire both without resetting incarnation allocation or losing Open histories. The reverse partial state must also fail closed. Restoring the engine can contain histories for Open, Closing, and Closed rank keys. Reinstalling an Open key must preserve its replayed history; blindly opening a new engine session must not erase it.

Required resolution: define `acquire_sessions` with exact expected/new rank identity, CAS revision, preserved next-incarnation counter, and reconciliation after uncertain completion. Require successful acquisition of both authorities before any session/shard mutation. Specify the load/join order and handling of each lifecycle state and unknown/mismatched SessionKey. Resume Closing cleanup before allowing a new incarnation of that client. Rebinding an Open key must preserve its restored retry/flush history.

API boundary: store session repository acquisition/load -> daemon loader -> `Engine::restore`, `replay_record`, and checked session installation. No cross-object atomic operation is needed; a resumable ordered protocol is sufficient.

### 4. Medium: one-record admission needs an explicit continuation boundary for multi-transaction engine calls

Contract: persistence-contract.md section 2, lines 39-47, and section 7, line 167.

The one-record-slot policy is sound, but it cannot be applied solely around existing public engine calls. `close_session` loops over all caps. `remove_cap` can submit a range transaction and trigger a purge transaction. `handle_cap_release` loops over items. `journal_committed` and `tick` call `run_parked`; commit processing can also trigger new work. The contract requires these loops to yield before a second transaction, but it gives the daemon no continuation entry point or cleanup-complete signal. The new `close_session_key -> Vec<Effect>` signature does not establish when shard cleanup has fully committed.

Required resolution: define one explicit engine step/resume protocol, or define equivalent semantics on a named existing entry point. Each step can admit at most one record. Return and materialize that step's effects before another step runs. Preserve the remainder of release/close work internally. Expose durable session-cleanup completion so the daemon cannot mark Closed after only the first record. Distinguish waiting for a WAL commit from needing another admitted step. Apply this rule to commit-triggered purge and parked work too.

API boundary: engine continuation/admission state and cleanup completion -> daemon scheduler/journal adapter. Do not solve this by reserving an unbounded number of slots per session close.

### 5. Medium: checkpoint-only fields need a defined WAL reconstruction rule

Contract: persistence-contract.md section 3, lines 61-90; section 4, lines 111-113; store.md sections 4 and 5.

`CheckpointInode` adds `stray_auth_path`, and the text requires stable subvolume identity. At the source boundary, neither is a field of `InodeCore`; `JournalRecord` has inode images and dentry changes but no prior-authorization-path or stable-subvolume update. A checkpoint field alone does not cover a committed unlink after the checkpoint. Dentry replay removes the primary parent. Once replay removes that link, the required former path and path-derived subvolume context can be lost.

Required resolution: state exactly how WAL replay preserves these values on the last unlink and rename-over-target path. Prefer explicit versioned inode/WAL fields. Alternatively require checked replay to capture the pre-unlink values before changing linkage, with a deterministic rule for hard links. Define the materialized runtime field that checkpoint export reads. Add a checkpoint-at-L, unlink-at-L+1, crash-before-next-checkpoint test. Verify prior authorization path and stable subvolume id, not only inode/dentry equality.

API boundary: engine persistent inode/transaction schema -> store record codec -> checked replay/checkpoint exporter. This does not require full G4 authorization enforcement now.

## Reviewed rules that do not need redesign

- One-stream contiguous completion and fail-closed submit errors are adequate. Capacity rejection before mutation and terminal failure after mutation are distinct.
- Global execution serialization is an acceptable initial visibility proof. The contract correctly retains partial-commit tests for any concurrent path.
- Materialized checkpoint export, checked sequence replay, session-incarnation dedupe, and retained trimming watermarks are required explicitly. Existing unchecked replay remains a test helper only.
- Lifecycle intent precedes work. Completion journals metadata and retirement together. Transient worker errors retain intent. Stale recovery results cannot overwrite later inode/truncate state.
- Same-object owner/tail guards, expected-generation checkpoint publication, exact-descriptor trim guards, and unknown-batch reconciliation are implementable with existing compound xattr/omap operations. Check all result layers. Do not use string ordering for a numeric tail comparison; equality CAS or numeric comparison is sufficient.
- Global allocator reservation before shard publication prevents reuse. Wasted reservations are safe. Restore must use persisted allocator state, not the default rank-0 lease. No new allocator safety model is required.
- Retaining checkpoint generations with a configured admission bound is conservative and valid. It can eventually stop new mutations. Garbage collection is deliberately deferred; do not claim indefinite bounded-space service.
- Source status is labeled accurately. Both design documents say these APIs are targets, not implemented features. Fresh G0 evidence remains a separate requirement.

## Later-gate work, not required changes here

Independent streams, aggregate deltas, warm follower/trim coordination, indexed loading, adoption, checkpoint garbage collection, and live migration remain later work. G3 still requires the acceptance checks in current/sol-recovery-review.md. No stylistic findings were raised.
