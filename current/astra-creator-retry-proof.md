# astra-creator-retry-proof

Task: Independently prove a peer-compatible, at-most-once outcome for an ordinary retry of a committed CREATE/MKDIR/MKNOD/SYMLINK after a lost SAFE reply when the current name is replaced or the locked parent and original target are missing. Distinguish wire REPLAY from ordinary retry. Prove reachability from committed engine behavior. Identify the minimal retained-state requirement if the current completed-request schema is insufficient. Cite upstream and local sources and define regression expectations.

Status: COMPLETE, source-only. A truthful original-target-only negative reply is peer-compatible when the original inode is resident. A complete policy for already-missing originals is still blocked by the unproved request-dentry attachment state. The report gives the case matrix, qualified reachability proof, a narrow prospective retention rule, and regression expectations. No protocol policy or implementation is accepted by this report.

Context: Sole writable file is `current/astra-creator-retry-proof.md`. Analysis is source-only. No production edits, builds, live operations, Git writes, or subagents are authorized. Daemon baseline is `a6ae05a` if daemon inspection is necessary.

Next: Parent/design owner decides the application error policy and completion-derived original-inode retention rule. Resolve the already-missing-original compatibility case, or supply the missing supported-peer exclusion proof, before claiming complete ordinary E1 acceptance. Obtain separate review of the resulting contract. E2/D6 and wire REPLAY retain their separate scoped acceptance.

Log:
- [2026-10-25] Created the task ledger. Started independent protocol and reachability analysis.
- [2026-10-25] Traced C++ completed dispatch/open, libcephfs insertion/verification, Linux negative trace handling, and local commit/cap/stray recovery. Negative results do not bypass trace insertion. Healthy FIFO ordering constrains namespace races, but stale-session revocation can remove read rights without an acknowledgment. Existing completion records identify the created inode without retaining it. No tests, builds, probes, live operations, Git writes, or subagents ran.
- [2026-10-25] Resumed from the source-analysis checkpoint. Read the current contract and independent re-review. E2/D6 and matching wire REPLAY now have scoped contract acceptance; ordinary E1 remains open.
- [2026-10-25] Checked the committed daemon, not its changing working files. Production runtime.rs:1269-1270 deliberately avoids engine stale invalidation. The stale-revocation sequence below is an engine/API and prospective G3 sequence, not a baseline production reproducer. Confirmed that inspected engine files have no diff from a6ae05a.
- [2026-10-25] Completed the report. Only this ledger was edited. No tests, builds, temporary probes, live actions, Git writes, or subagents ran. Read-only Git commands inspected source provenance and committed daemon code.

## 1. Conclusion

**Ordinary E1 is not fully proved. There is a small sufficient prospective design, but it does not repair an already-purged original inode.**

1. When recorded inode `I` is resident, a terminal error such as EINTR with **only its current target trace `T(I)`** avoids both clients' named-success reconstruction paths. It also avoids libcephfs's empty-trace dereference of the old request dentry. This encoding works for CREATE, MKDIR, MKNOD and SYMLINK. It must carry no false dentry and grant no new rights or writer range. CREATE's special ENOENT handling is described below.
2. For a replacement name, recommend an explicit ordinary-retry **`-EINTR` policy**, separate from that encoding proof. libcephfs already uses EINTR when post-create lookup finds a different inode. This is a source-supported application error choice, not a C++ server rule or a special success status. Both inspected Ceph client paths return it without opening the replacement or automatically repeating the creator.
3. Retaining current inode `I` while its creator completion remains retry-eligible would make this response available even after unlink, replacement or parent removal. Existing `created_ino` references can identify these pins. A new historical dentry, full reply image or persistent field is not inherently required for this prospective rule.
4. Existing schema-2 histories can legally retain a creator completion after `I` has been purged. Retention cannot reconstruct that inode. If the name now names `J`, a truthful null-dentry trace is unavailable too. An empty error is safe only with an absent or attached client request dentry. The server has no attachment proof. A complete supported-peer exclusion of the detached state was not established here. **This is the exact remaining compatibility-proof blocker.** Missing parent plus missing `I` has the same blocker.
5. Do not claim an end-to-end baseline reproducer. The committed daemon neither invalidates engine caps through STALE nor finishes nonempty gathers. The engine has the relevant state transitions, but that is a different claim from a reachable production/unmodified-client history.

The report permits an owner decision; it does not approve implementation. The independent review in `current/astra-recovery-contract-review.md` separately accepts E2, D6 and matching wire REPLAY. This report does not reopen those scopes.

## 2. Evidence and notation

- Rust daemon citations below mean **`a6ae05a`**, read with `git show`/`git grep`. The changing daemon working files were not used. `git diff a6ae05a --name-only -- crates/engine` returned no paths when checked. Engine references identify that inspected revision.
- C++ paths are relative to `/home/ubuntu/ceph/src/`. The checkout reports HEAD `8db74a411f998b5e6a697cf541113b0f98c1bf9f`. These are inspected source facts, not a verification of the installed Ceph 20.2 binaries.
- Linux paths are relative to `/mnt/workspace/ceph-client/`. The checkout reports HEAD `a76266bff051bfa94907a1b6e3f1d41e573c3df8`. Linux source was read only for client behavior. No GPL code was translated. Installed-host qualification remains separate.
- `I`: the completed request's recorded created inode. `P/n`: the requested parent inode and final name. `J`: a different inode now at that name.
- `D(P,n)+T(I)`: current positive dentry and target. `D0(P,n)`: current parent/name with a null dentry and no target. `T(I)`: target only, with no parent/name assertion.
- Ordinary retry: `num_retry > 0`, with no earlier unsafe reply. SAFE is the reply's durable-completion flag. REPLAY is the separate request flag used after an unsafe reply.

Match durable session incarnation, tid and original operation before trimming history or applying embedded releases. The completion remains successful even if an ordinary retry reports a reconstruction error. Same-tid deduplication is not a guarantee against a new application request with a new tid.

## 3. Complete state matrix and limits

This matrix separates the **available truthful encoding** from the **application error choice**. It does not turn an unresolved row into implementation freedom.

| State at the reply's committed-state cut | Truthful response shape | Application result and disposition |
|---|---|---|
| Matching wire REPLAY, any later namespace state | Empty target/dentry/snap traces. CREATE alone keeps original `I` in its create extra. | Recorded result. Already settled independently. No reopen, cap grant or ordinary reconstruction. |
| Ordinary named retry; `P/n -> I` | `D(P,n)+T(I)` | Recorded success. Reconstruct the reply, not the mutation. A writable CREATE may need a separately committed writer range. |
| An ancestor moved; request still uses the same `P`; `P/n -> I` | Same as preceding row | Success. Do not reinterpret the request as an absolute historical path. |
| Requested link still names `I`, but `I` has another primary link | Trace the requested link, not the primary link | Success. A changed primary link is not a changed requested identity. |
| `P` traceable; `P/n` absent; `I` resident elsewhere or unlinked | `D0(P,n)`, or negative `T(I)` without a dentry | Contract's proposed path result is ENOENT. It has the kernel CREATE exception below. A uniform changed-identity EINTR policy with `T(I)` is a separate owner choice. Neither shape may claim the old positive link. |
| `P` traceable; `P/n` absent; `I` already purged | `D0(P,n)` | A source-supported ENOENT response exists. This row does not require retaining `I`. CREATE still carries recorded `I` in its extra; no target is asserted. |
| `P/n -> J != I`; `I` resident, linked or unlinked | Negative `T(I)`, no dentry, no target `J` | Recommend EINTR as an explicit changed-identity policy. Source proof for the encoding is in section 4. EEXIST/ENOENT are not the original committed result. ENOENT also misstates a present name and has special kernel behavior. |
| Parent/path cannot be traced at the requested locked parent; `I` resident | Negative `T(I)`, no dentry | Actual resolution error is possible as proposed in the contract. A uniform EINTR policy avoids the CREATE/ENOENT special path. Error selection needs owner approval; encoding alone does not choose it. |
| `P/n -> J != I`; `I` missing | No permitted nonempty trace established | **BLOCKED** without an absent/attached request-dentry proof. `D0` would falsely remove `J`; `T(J)` with extra `I` is forbidden; target-only success can resolve `J`; an unrelated target is not a solution. |
| `P` cannot be traced and `I` missing | No permitted nonempty trace established | **BLOCKED** on the same attachment proof. Never substitute a different parent, historical link, dummy inode or generic empty errno. |
| Request actually has no request dentry; required inode missing | Empty negative reply | Its empty-trace dereference is absent. This is not a fallback inferred from a named creator or from a missing server dentry. Supported named creators normally have a request dentry. |
| Same tid, different op or different durable incarnation | No completed-creator response | Identity violation or different session. Do not use another operation's result or trim its history. |
| Matching open-like completion has `created_ino=None` | No recorded original creator target | Separate unresolved identity case. All four successful **new-inode** paths record `Some(I)`. CREATE of an existing inode with truncation can record `None`; it is not a new-inode creator and must not acquire a fabricated create extra. |

For every ordinary CREATE row with recorded `Some(I)`, preserve the opcode-valid `openc_response_t` extra with `created_ino=I` and no delegated ranges. Other creator opcodes carry no CREATE extra. Preserve the original wire op/tid and SAFE flag. No row reruns allocation, link insertion, O_EXCL, O_TRUNC, initial xattrs, timestamps or the original completion transaction.

The new-inode identity fact follows from `engine/ops_ns.rs:424-435,554-561` and `engine/mod.rs:1368-1381`. The existing-inode truncation distinction follows from `engine/ops_attr.rs:227-271`. Do not encode an affected existing inode as `created_ino` merely to obtain a retry anchor; that field has creation semantics on the wire.

## 4. Peer proof for the resident-original error

### 4.1 libcephfs/ceph-fuse

`Client::handle_client_reply` installs every first reply through `insert_trace` **before** it handles SAFE completion or wakes the caller (`client/Client.cc:3060-3112`). A negative result does not skip this step.

For an empty ordinary trace, `insert_trace` gets the request dentry and dereferences `d->dir->parent_inode` at `:1788-1795`. It checks the result only afterward. Thus the ordinary empty-error proof needs `d == nullptr` or `d->dir != nullptr`.

For a nonempty target-only trace, the function decodes `I`, calls `add_update_inode`, skips the dentry branch, and stores `request->target` (`:1848-1868,1871-1913,1944-1973`). None of these creator branches dereferences the old request dentry's directory. `add_update_inode` explicitly accepts a stat with no caps (`:1244-1245`). The response can therefore carry committed metadata for resident `I` with **zero issued caps**, without reopening it or inventing a cap ID.

`make_request` runs `verify_reply_trace` only for a nonnegative result (`:2378-2403`). `_create` returns on a negative result before creating a file handle (`:16214-16233`). `_mknod`, `_mkdir` and `_symlink` return the error (`:16064-16069,16316-16322,16463-16468`). No successful target or replacement file handle is delivered by this negative result.

For a truthful null-dentry response, insertion uses the **decoded parent**, not the old request-dentry directory (`:1871-1913`). This also avoids the empty branch, including when the request's old dentry object detached.

### 4.2 Linux client

The first SAFE response is parsed even on error. `handle_reply` prepares `r_target_inode` from the target trace and calls `ceph_fill_trace` (`fs/ceph/mds_client.c:4152-4187,4218-4229`). `ceph_fill_trace` fills target metadata on a negative result with fmode `-1`, then skips named insertion when `is_dentry` is false (`fs/ceph/inode.c:1710-1743`). Zero-cap stats do not allocate a new cap (`:1063-1069`). This insertion step does not splice `I` under `P/n` or splice `J`. A terminal error such as EINTR then exits without a successful open. ENOENT has the exception below.

CREATE extra is not a binding safeguard. Target selection uses the trace. Ordinary CREATE errors exit before `FMODE_CREATED` and `finish_open` (`fs/ceph/file.c:978-1005`). MKNOD/SYMLINK/MKDIR likewise skip their success-only follow-up on error (`fs/ceph/dir.c:977-988,1097-1106,1177-1195`).

**ENOENT is different for CREATE.** `ceph_atomic_open` converts it to zero and can call `ceph_handle_notrace_create` or return `finish_no_open` to VFS name resolution (`fs/ceph/file.c:957-983`). Therefore ENOENT is not proof that the syscall terminates without another lookup or a fresh create. EINTR takes the normal error exit. This report proves the inspected Ceph paths, not every application's retry policy.

Even on error, a supplied dentry trace can change the client's namespace cache. The locked-parent checks and insertion are not success-gated (`fs/ceph/inode.c:1740-1855`). Never use a null dentry for a name that currently names `J`.

### 4.3 Why EINTR is a policy recommendation

`Client::verify_reply_trace` can return EINTR when a successful traceless CREATE is followed by lookup of a different inode (`client/Client.cc:2182-2220`). That establishes an existing application-visible outcome: the mutation may have committed while the caller receives EINTR.

It does **not** make every direct server-side EINTR correct. The recommendation is limited to a proved completion hit whose named success cannot preserve the original identity. Use truthful nonempty `T(I)` and retain the original completion. The application sees an error, not “successful create with interrupted reconstruction.” A new `make_request` allocates a new tid (`:2260-2262`); application retries remain separate operations.

### 4.4 C++ is not a replacement-identity proof

`Server::handle_client_request` separates completed REPLAY from ordinary retries (`mds/Server.cc:2657-2692`). Completed CREATE sets original extra `I` before ordinary open (`:2896-2905`). Open resolves the **current** path (`:4660`), suppresses repeated truncation (`:4738`), and traces the current target/name (`:4818-4826`). That can combine target `J` with extra `I`. Copying it violates this task's original-identity requirement.

`reply_client_request` constructs traces and attaches extra independently of a negative result (`:2438-2457`). `rdlock_path_pin_ref` can supply a truthful null-dentry ENOENT (`:3905-3917`). These support the narrower error encodings; they do not supply missing metadata or a universal changed-path errno.

The local reply builder also needs deliberate treatment. `materialize_reply` currently calls `inode_stat(..., true)` for every target (`engine/mod.rs:878-899`). Setting a negative result alone therefore does not suppress grants. `inode_stat(..., false)` already illustrates capless metadata construction (`engine/caps_eval.rs:381-482`). An implementation must use an explicit no-new-rights error path and keep advertised ranges within existing durable bounds.

## 5. Reachability: what is and is not established

### 5.1 Durable append before commit callback

There is no same-process namespace-change window here. `Engine::step` refuses another input while a commit is pending (`engine/step.rs:128-149`). Every new creator installs its runtime cap and records its completion with the inode mutation before submission (`ops_ns.rs:424-436,554-561`).

On callback, the engine removes request holds, materializes deferred output, then evaluates caps (`engine/mod.rs:1209-1226`). The daemon consumes the effect vector before another admitted step (`server/runtime.rs:501-519,674-675,768-774,885-887`, baseline). A crash before callback loses volatile caps/output, but restore recovers the committed inode and completion. It does not itself erase `I` or `P`.

### 5.2 Built SAFE reply not observed: healthy retained session

The initial target trace issues LINK_SHARED (`Ls`). Unlink and both relevant rename targets require `LinkXlock`, which excludes `Ls|Lx` (`inode.rs:193-194,232`; `ops_ns.rs:663-672,754-775`). A normal peer cannot acknowledge an unseen grant on unknown `I`. A later revoke sent through the same ordered session cannot overtake the SAFE reply (`session.rs:159-218`, baseline).

This rules out the simple proposed race “drop just the SAFE result, let its later revoke be acknowledged, then replace the name” on an intact healthy stream. An offline test that discards the reply and supplies such an acknowledgment is an engine-state test, not an unmodified-peer reproducer. Transport acceptance is not proof that the application saw the reply, and transport loss alone does not authorize dropping rights.

### 5.3 Engine STALE can bypass a read-cap acknowledgment

The engine API has a different transition. `session_stale` forces already-revoking non-write rights. `eval_caps` also forcibly removes newly conflicting non-write rights for a stale holder (`client_caps.rs:458-496`; `caps_eval.rs:235-253`). `CEPH_CAP_ANY_WR` includes exclusive rights (`consts.rs:52`); do not assume that every unused capability is safe to force.

A concrete **engine-level** sequence is:

1. Use shared parent `P`. Client A commits a read-only CREATE, regular-file MKNOD or SYMLINK at `P/n`; its SAFE reply remains unseen. Do not give A conflicting exclusive parent rights.
2. A stops renewing. B's unlink/rename needs A's `Ls` removed. Enter engine STALE. Only non-write conflicts are forced; A's durable session and creator completion remain.
3. B commits unlink/move, then can install replacement `J`. Runtime cap-map residency still retains unlinked `I`, even though A never learned it (`ops_ns.rs:237-241`).
4. Crash. A resends the ordinary request and reconnects only caps it knows. An unseen `I` is absent from its reconnect message. B can release or trim its now-unused cap before final reconnect. `I` can therefore be capless after complete reconstruction while A's creator completion remains.
5. The existing engine's unclaimed-stray path can purge `I` before a later request turn (section 5.4).

Steps 1-3 use operations available to unmodified peers **if the daemon integrates this engine STALE transition**. This was not executed. MKDIR needs separate care: its original directory cap can contain `Fx`; a competing lookup can change its loner and demand a write/exclusive revoke. The simple read-only sequence above is not a complete unmodified-client MKDIR reproducer. An injected engine RMDIR by path can exercise the state, but must not be reported as that peer proof.

**Baseline production does not take this STALE transition.** `server/runtime.rs:1269-1270` explicitly retains engine rights until fenced cleanup. The calls in `server.rs:1255` belong to `#[cfg(test)] impl Server<MemJournal>` (`:1169-1170`). Production `try_finish_recovery` also holds nonempty gathers (`runtime.rs:890-902`). Thus neither engine capability invalidation nor a raw finish-gather call proves that the currently committed daemon can complete this mounted-client scenario. D6's prospective ordinary STALE behavior needs this analysis when integrated.

### 5.4 Restore and purge do not derive retention from history

Restore recreates inode metadata and histories, but no volatile caps (`engine/durable.rs:115-145`). Before reconstruction finishes, `maybe_purge_stray` is gated. After finish, `queue_unclaimed_strays` selects `nlink==0 && caps.empty()` without checking creator history (`reconnect.rs:6-11,138-139`). `step(None, permit)` services that purge queue before parked work (`step.rs:193-208`). `purge_inode` removes metadata and writes its durable purge intent (`caps_eval.rs:610-640`).

The existing test `gather_discovers_range_free_strays_and_replays_purge_before_callback` (`tests/review_regressions.rs:821-887`) explicitly exercises retained sessions, empty reconstructed caps and durable purge across a lost callback. It was read, not run. Its initial creator was acknowledged in its test setup. It proves a legal engine/history state, **not** a lost-SAFE peer history.

Processing known retries before purge closes one scheduling window. It is not a general retention invariant. A capless error reply can itself be lost; transport acceptance does not retire the completion. A later purge/restart can then encounter the same missing-original problem.

### 5.5 Parent inode and client dentry are separate objects

There is strong, explicit parent retention for the normal single-rank pending-creator path:

- libcephfs creators keep `wdr.diri` in `MetaRequest::_inode`, an `InodeRef` (`client/MetaRequest.h:24-26,116-123`; `Client.cc:16046-16047,16177-16179,16278-16279,16456-16461`). Live request paths use that parent inode plus name (`:6569-6577`; `client/Inode.cc:146-150`). Ancestor rename does not lose this parent identity.
- `check_caps` retains PIN (`Client.cc:4313`). Final inode destruction removes all caps (`:3707-3722`). Ordinary recall does not discard the sole auth cap merely because its dentry changed (`:5165-5209`). Reconnect sends retained caps, including generation-expired caps reduced to PIN (`:3463-3510`).
- Linux takes a parent PIN reference for the request (`fs/ceph/mds_client.c:3918-3932`). PIN is used state (`caps.c:1011-1015`); cap trimming respects it (`mds_client.c:2281-2311`), and embedded releases exclude used caps (`caps.c:4944-4953`).
- The engine keeps an unlinked inode while any cap entry exists. STALE changes rights, not cap-map membership. Restore holds purge until the retained parent cap can be reconstructed.

Consequently, ordinary rename of an ancestor, unlink of the parent name, or normal cache trimming does **not** prove that a pending creator's parent inode disappears. With the known parent cap retained through the same session and complete reconstruction, purge of `P` is excluded by the actual cap check. Terminal close/fencing changes the retry-eligible session and cannot be used as a same-incarnation lost-reply example.

This is a scoped parent-cap argument, not a universal server invariant for every accepted path or capless input. `CompletedRequest` records no parent, and the engine can accept synthetic requests without a peer's parent-cap history. No complete exclusion for all capless-parent/reconnect histories is claimed. Missing-parent fixtures must state how the parent cap was absent, rather than equating removal of the parent's name with removal of its inode.

The **request dentry's attachment** needs a different proof. `DentryRef` pins its lifetime and cache LRU entry (`MetaRequest.cc:59-64`; `Dentry.h:26-43`). `detach` still clears `dir` (`Dentry.h:71-77`). Rename insertion can detach an old source dentry despite another reference (`Client.cc:1323-1328,3871-3879`). Ordinary trimming respects pins (`:5083-5099,6080-6103`); null-dentry and unlink handling normally retain the object. These facts neither produce a complete pending-creator detach sequence nor exclude all overlapping rename/request/output schedules. **This report claims no detached-creator crash reproducer and no universal attached-dentry invariant.** WANT_DENTRY reports the existence of a request dentry, not whether it remains attached when the response arrives.

## 6. Minimum retained-state proposal and retirement rule

If the owner chooses a response that must work without proving dentry attachment, the narrow sufficient invariant is:

> For each retry-eligible successful new-inode creator completion `(session incarnation, tid, op, created_ino=I)`, retain authoritative committed inode metadata for `I` until that completion is durably retired. Link removal may make `I` a stray. It must not make `I` unavailable for a capless negative target trace.

This retains **current inode state**, not its historical name or a serialized old reply. Existing inode images retain symlink text, layout, attributes, authorization path and stable subvolume identity. Existing completion fields identify the dependency. Reconstruct runtime reference counts/sets after restore and replay. Share one inode among multiple references if needed. No format change follows merely from adding this lifetime rule.

Implementation obligations for this rule:

1. Install the dependency with the creator's committed inode/completion. Cover the append-durable/callback-lost cut by deriving it during replay. Do not let a later watermark delete the dependency before the matching retry captures its reply work.
2. Check every retirement path, including immediate last-link purge (`ops_ns.rs:237-241`), queued stray purge, cap release, session cleanup, restored unclaimed strays and lifecycle eligibility. Keep data-lifecycle rules coherent with the retained inode. The simplest narrow implementation postpones its existing purge intent together with metadata retirement; it does not need a separate retained-metadata/data-GC subsystem.
3. Preserve `I` across reconnect even when the client omits it. Client knowledge of an inode is not required for a server history reference. Retention supplies no client capability and must not revive an old namespace link.
4. Release the dependency only when its history retirement is durable, or when terminal cleanup for that key commits. A sent reply, returned effect, cap release, transport reset, elapsed time, grant-gate marker or final reconnect chunk is not that event.
5. Recheck eligibility after release and queue ordinary purge. Hold through any admitted reply/range work that still needs the inode. A new session incarnation must not inherit the old completion or its reply context.

### 6.1 Volatile trimming is the main durability trap

`dispatch_new` calls `Session::trim_completed_requests` on ordinary input (`engine/mod.rs:557`; `session.rs:74-81`). A read-only request can therefore erase runtime history without writing a record. Scanning only that already-trimmed map is **not sufficient** for durable pins.

Counterexample: checkpoint retains completion C; a later read advances the watermark only in memory; code drops C's pin; another client's unlink durably purges `I`; crash before a checkpoint or record persists the trim. Replay can restore C but not `I`. This breaks the proposed invariant even if a conforming client no longer retries C.

Existing records carry `CompletedRequest.oldest_client_tid`, and replay applies that trim (`journal.rs:80-82`; `engine/replay.rs:103-111`). Checkpoints store the history and watermark (`durable.rs:66-78`), and session-close records retire the key (`replay.rs:123-126`). These can supply retirement evidence without adding a history schema. The owner must retain a durable-retention view until the trim reaches one of those durable boundaries. A conservative first version can wait for a committed watermark-bearing mutation or terminal session cleanup. A checkpoint can retire it after publication if the implementation has the corresponding publication proof. Merely constructing a checkpoint is insufficient.

Tradeoff: a stalled oldest tid or a long-lived session can keep unlinked inode metadata and, with the simple existing purge path, data longer. History counts are already not generally bounded by this task. Do not silently replace durable retirement with a timer. Parent retention alone is insufficient for the replacement row: a present `P/n -> J` cannot truthfully be encoded as a null dentry.

### 6.2 Already-missing originals remain a separate obligation

`validate_state` checks completion key/tid consistency, not `created_ino` residency (`engine/durable.rs:326-335`). Such older images are valid under the current schema. A new retention validator cannot simply relabel them corruption. Neither old result bytes nor the inode number reconstruct an authoritative stat. Checkpoint/WAL trimming also prevents assuming that the original full image remains available indefinitely.

An absent old name with resident `P` still has the `D0(P,n)` solution. The unresolved legacy cases are replacement `J` with missing `I`, and untraceable `P` with missing `I`, where the server lacks an attached/no-dentry proof. Retaining new originals does not close them retroactively.

The accepted G2 evidence records an all-Closed checkpoint. An owner may choose an explicitly verified adoption boundary with no surviving retry-eligible histories. That is a deployment choice, not proof that every legal older image is covered. This report made no fresh cluster query and authorizes no newfs/reset or session eviction. If general recovery of such older histories remains required, ordinary E1 must stay open until the client-state exclusion or another complete compatible policy is proved.

## 7. Concrete regression expectations

These are requested acceptance expectations, not tests executed by this task.

### Engine and daemon tests

| Case | Required assertions |
|---|---|
| Each of CREATE/MKDIR/MKNOD/SYMLINK; durable commit before callback; callback before output; output accepted before peer observation | Original allocation and completion appear once. No success before commit. Same incarnation/tid/op retry never re-executes the mutation. Record each cut separately. |
| Unchanged requested name, ancestor rename, requested hard link with another primary | Recorded success; exact requested `P/n` and target `I`. CREATE-only extra `I`. No new link, O_EXCL failure, repeated truncate, xattr or timestamp change. |
| Replaced name, `I` still linked elsewhere or retained as stray | Under the proposed policy: SAFE EINTR; only target `I`; no dentry, no `J`, no new caps/range, no added lifecycle write merely to answer. The completion stays successful. |
| Parent cannot be traced, `I` resident | Approved negative result and only `T(I)`. No substituted parent or historical name. Exercise removed MKDIR inode and symlink metadata as well as a regular file. |
| Absent name, `I` missing but requested parent resident | Truthful `D0(P,n)` and approved error. Explicitly distinguish kernel CREATE's ENOENT follow-up from EINTR behavior. |
| Retention on unlink, rename-overwrite, last-cap release, close, unclaimed-stray scheduling | `I` stays resident while any retry-eligible durable history reference remains. No purge intent races that reference. Ordinary namespace and quota removal still occurs once. |
| Restore/replay omits unseen `I` from reconnect caps | History-derived retention remains. No purge before or after the grant marker or retry-output cut. Lose the negative reply again and repeat recovery. |
| Read-only watermark trim then another client's unlink; crash before trim persistence | Older checkpoint plus new WAL cannot produce retained completion with missing `I` under the new rule. Test WAL-persisted trim and checkpoint-published trim separately. |
| Terminal close and reincarnation | Drop old references only at durable cleanup retirement. Later purge is valid. Old callbacks/retries cannot answer through a new key. |
| Old legal checkpoint already lacks `I` | Do not invent metadata, relabel it corrupt, return `J`, mutate, evict or silently return an empty error. Replacement/missing-parent expected output remains blocked pending the explicit compatibility decision. |
| CREATE existing-inode truncation with `created_ino=None` | Keep it separate from new-inode creators. Do not repeat truncation or fabricate creation identity. Its original-target policy needs its own resolution. |
| Genuine REPLAY after rename/replacement/purge | Recorded result, empty target/dentry/snap traces, original CREATE extra only, no grant or mutation. Never test ordinary loss by setting REPLAY. |
| Healthy FIFO versus engine STALE | Healthy unlink waits for a real revoke acknowledgment. Read-only engine STALE may force non-write conflicts. A write/exclusive conflict requires fencing. Test that baseline daemon behavior is not confused with the test-only compatibility API. |

### Unmodified-client expectations

1. Use one saved wire tid with no prior unsafe reply. Confirm `num_retry` changes and REPLAY stays clear. Dropping a syscall result or issuing a second syscall is not this test.
2. Exercise Linux and libcephfs/ceph-fuse independently. For unchanged state, return the original identity and normal creator result. For resident-original replacement under the proposed policy, require application EINTR, no opened replacement, no positive old-name cache insertion, no malformed reply/assertion, and no Ceph-internal automatic creator resend on EINTR.
3. Keep namespace observation separate from the first caller's result. Another client may observe the committed object even when the original caller receives an error. A deliberately new application operation gets a new tid and is outside the same-tid guarantee.
4. For a reachable changed-name trial, record cap masks, stale/fence decisions, complete ordering and client knowledge of `I`. Start with read-only CREATE or regular MKNOD/SYMLINK and a shared parent. Do not claim the same sequence proves MKDIR or a write-open case. Do not let the original key close and then call the new session an ordinary retry.
5. Verify parent PIN reconstruction while a creator waits. Removal/rename of the parent's directory entry must not be reported as inode purge. Include cache recall/reconnect trimming and concurrent rename traffic when investigating request-dentry attachment.
6. A synthetic detached libcephfs request can validate the **encoding branch**, but it does not prove unmodified-peer reachability. A real detached-creator sequence or a complete exclusion proof is still needed before an empty error can close the legacy blocker.
7. Run the ENOENT CREATE case separately. Record VFS follow-up and any new tid; do not score it as an EINTR terminal-error case. Validate symlink and removed-directory stats through the normal codec/invariant checks.

No passing source analysis substitutes for these peer observations. Only the assigned QA owner can run live operations after contract and implementation review.

## 8. Minimal owner decision

Approve or reject these two narrow choices explicitly:

1. **Application semantics:** For an ordinary matching completion whose requested name no longer identifies its recorded inode, return EINTR with a capless, truthful original-target trace. Preserve the successful durable completion. State whether the existing truthful null-dentry ENOENT rows remain, with their documented kernel CREATE behavior.
2. **Lifetime:** Retain current original-inode metadata using existing creator-completion references until durable history retirement. Specify volatile-trim versus durable-retirement handling and how older legal missing-original histories are covered.

The first choice is sufficient for resident-original replacement. The second supplies that original prospectively. **Neither alone establishes full ordinary E1 compatibility for already-missing originals.** Resolve that exact remaining proof before accepting the entire matrix. No broader storage architecture is required by the findings here.
