# sol-recovery-review
Task: Independent reference audit of the G3 recovery protocol boundary. Produce acceptance checks and mandatory wire/API gaps for kernel, FUSE, and Ganesha recovery. No production edits or live tests.
Status: Complete. Independent source audit produced the acceptance checklist below. No build, tests, live operations, production edits, or other agents' diffs were used.
Context: Sole writable file is current/sol-recovery-review.md. Reviewed the approved plan G3, repository guidance, Rust proto/map representations, the reference map, and C++ sources. Findings describe boundary requirements, not the status of other agents' implementations.
Next: Coordinator assigns completion-driven daemon recovery integration against this checklist. Keep the wire codecs; add recovery orchestration and explicit engine/session adapters. Request real mounted-client evidence from the designated runner.
Log:
- [2026-10-22 00:00] Created review ledger before reference inspection.
- [2026-10-22] Completed source audit. Found an important documentation trap: reconnect v5 chunks may contain both caps and realms. Existing Rust decoding already accepts this.

## Outcome and scope

No missing G3 message type or MDS state was found. Existing codecs cover the required input. G3 needs runtime state, durable identity, reconstruction adapters, and ordering checks. Do not replace codecs with new recovery messages. The approved single-rank scope does not require the distributed resolve/rejoin exchange.

References below use the local C++ checkout at `/home/ubuntu/ceph/src/`. The reference map pins `5f7e383ae6d`. These are source facts, not evidence from the installed 20.2 peers. Kernel behavior comes from the reference map only. No kernel code was translated.

## Acceptance checklist for the next daemon assignment

### 1. Membership and prior-owner barrier

- [ ] Separate requested state from monitor-confirmed state. Enter each phase only after the MDSMap confirms it. Advance only after its completion event. Keep maps, storage completions, reconnect traffic, renewals, and eviction progress runnable while ordinary requests wait.
- [ ] For one in-rank and no failed ranks, use `replay -> reconnect -> rejoin -> active`, or `rejoin -> clientreplay -> active` if recovery requests need draining. `replay -> resolve -> reconnect` is also legal, but distributed resolve is not necessary for G3. Never use `reconnect -> active` or skip rejoin. Preserve the internal clientreplay hook even when it is empty.
- [ ] Before takeover reads or destructive work, obtain an OSDMap at least `last_failure_osd_epoch` and validate prior-owner fencing. Zero is not proof of fencing. A later standby-replay removal can overwrite this field with zero. Reconcile the old holder's identity/address with current blocklist evidence and the ownership contract. Fail closed if safety cannot be established.
- [ ] Accept standby-replay as a cold follower. Do not acquire ownership, append, checkpoint, purge, or report damage while following. On promotion, perform the complete fenced replay path with the new rank incarnation.
- [ ] Handle `active -> stopping -> stopped` through drain completions. Do not invent a direct active-to-stopped transition.

Sources: `mds/MDSMap.cc:1104-1138` (`MDSMap::state_transition_valid`); `mds/MDSRank.cc:1792-1819` (`replay_start`), `:1897-1968` (`replay_done`), `:1988-2035` (`resolve_done`, `reconnect_start`, `reconnect_done`), `:2048-2127` (`rejoin_done`, `clientreplay_start`, `maybe_clientreplay_done`); `mds/FSMap.cc:1092+` (`FSMap::erase`). The zero-epoch overwrite is documented in `mds-rust-map/cpp-map/01-cluster-integration/fsmap-and-mdsmap.md`, field table at line 431.

### 2. Durable session restore and reconnect gather

- [ ] Restore session identity and lifecycle before advertising reconnect. Include client global id/incarnation scope, authenticated name, old address/nonce, CephFS features, metadata (`uuid`, `timeout`), and retry/flush authority. Rebind a new authenticated connection to the restored session. A connection reset must not itself erase the session.
- [ ] Keep CephFS feature bits separate from messenger feature bits. Enforce `required_client_features` from MDSMap. OPEN after reconnect carries supported features, metric specification, and cap auths. Preserve `reply_encoding`, `reclaim_client`, and `multi_reconnect` behavior. Use MDSMap session timeout/autoclose values and persisted per-session timeout policy.
- [ ] Build the gather from restored open sessions. An empty gather may complete immediately. A nonempty gather must remain open until every member supplies its final chunk or completes fenced eviction. Permit the requested-reconnect/map-delivery race without prematurely denying the reconnect.
- [ ] Deny a closed or late reconnect with CLOSE. REJECT is legal only for OPEN or reconnect, not for a sessionless renewal. Reconnect success uses OPEN, not a new acknowledgment type.
- [ ] Reset push sequence and reconnect cap sequence state consistently with clients. Count only CAPS, SNAP, LEASE, and QUOTA in the supported feature set. Preserve per-session output order across OPEN, cap/realm messages, and deferred replies.

Sources: `mds/Server.cc:1563-1767` (`reconnect_clients`, `handle_client_reconnect`); `mds/MDSDaemon.cc:1266-1319` (`ms_handle_accept`); `client/Client.cc:3438-3522` (`send_reconnect`); `mds-rust-map/cpp-map/02-client-protocol/sessions.md`, sections 3.3, 3.9, and 3.10.

### 3. Reconnect v3/v5, caps, locks, and realms

- [ ] Read reconnect payload from the data segment. Accept v3 as one complete reconnect with raw 24-byte realm records. Accept v5 as an ordered series with `more=true` until its final `more=false` message. Do not send success OPEN or remove a gather member on a partial chunk. Include an empty final chunk in tests.
- [ ] Accept mixed caps and realms in a v5 chunk. `Client::send_reconnect` adds both to the same message. The statement in `crates/proto/src/reconnect.rs:9` that a part never mixes caps and realms is false for libcephfs. This is a comment gap, not a decoder gap: Rust already decodes both. The existing `reconnect_v3_and_v5_through_cpp` test also uses mixed contents.
- [ ] Validate all lock blobs before applying a chunk. `CapReconnect::locks()` can fail even after `MClientReconnect::from_raw` succeeds. Do not partially install a malformed chunk. Bound held reconnect bytes/counts. The protocol has no chunk transaction id or chunk index; do not invent one. Define connection-reset/repeated-chunk handling so locks do not duplicate.
- [ ] Preserve the client's cap id, issued and wanted masks, snap realm, and `snap_follows`. Raise the new cap-id allocator above every recovered id. Reset sequence state as the peers do. Do not grant conflicting rights merely because durable replay did not retain volatile caps.
- [ ] Restore both fcntl and flock held locks. Replace the blob's client field with the authenticated reconnecting client id. Preserve owner, pid, type, start, and length, including length zero. Load/recover the authoritative inode before completing reconstruction. This includes unlinked-open inodes and hard-linked inodes; a path hint is not authority to create missing metadata.
- [ ] Reconcile root and quota/subvolume realms, even without snapshots. Use client realm sequence/parent information to decide the required updates. Keep valid old/new snap-trace encoding per session features. Complete cap/lock/realm reconstruction before ordinary admission or purge.

Sources: `messages/MClientReconnect.h:100-168`; `client/Client.cc:3455-3522` (`send_reconnect`); `mds/Server.cc:1689-1767`, `:1945-1965` (`recover_filelocks`); `mds/CInode.cc:3676-3690` (`reconnect_cap`). C++ can defer missing inode caps to rejoin; its later attach paths do not restore file locks. Do not copy that limitation into the fully resident single-rank implementation.

### 4. Durable mutation with lost reply and clientreplay ordering

- [ ] Preserve completed-request identity and result in the same durable history as its mutation. Scope it to the restored session incarnation. Restore completed flushes too. Trim only under the defined `oldest_client_tid`/flush contract, not on disconnect.
- [ ] Classify requests that arrive before the reconnect message. Clients resend unsafe/retried requests and flushing caps before reconnect. Queue known committed retries for recovery handling ahead of ordinary requests. Suppress old embedded cap releases on reconnect retries so they cannot release newly reconstructed rights.
- [ ] Answer the committed retry without rerunning its mutation. For CREATE/O_EXCL, return the original created inode and valid create extra/trace. For rename-over-target, link, unlink, xattrs, and truncate, do not replay side effects. A stored result code alone does not prove a valid reply trace.
- [ ] Drain recovery replies and required recovery mutations before releasing ordinary requests that can change the reply's inode or parent. Retain original request-validation context until the output passes `MClientReply::check_invariants_for`. Do not send a stored stale trace or a traceless create response when the client requires a trace.
- [ ] Complete the internal clientreplay phase only after the replay queue drains and its required journal writes become durable. Then request/confirm active before ordinary admission. Safe-only service removes unsafe reply replay obligations; it does not remove the durable-mutation/lost-safe-reply case. A real `up:clientreplay` is legal when needed; it is not mandatory when the hook is empty.

Sources: `mds/Server.cc:319-397` (`dispatch`), `:2657-2697` (`handle_client_request`); `mds/MDSRank.cc:2075-2127` (`rejoin_done`, `queue_one_replay`, `maybe_clientreplay_done`); `client/Client.cc:3453-3457` (`send_reconnect`). C++ converts some completed creates to lookup/getattr so it can reconstruct a valid trace before new mutations race it. The Rust adapter needs equivalent externally visible correctness, not a copied C++ request implementation.

### 5. Missing sessions, Ganesha RESET, and no-extra-hold policy

- [ ] At reconnect timeout, evict every missing restored session, including sessions with `timeout`. Wait for successful fencing and durable cleanup before finishing the gather. Do not enter C++'s additional timeout-session reclaim hold. This is the deliberate decision 6 deviation, not a missing protocol state.
- [ ] During normal active operation, retain silent timeout sessions according to the accepted timeout/autoclose policy. Reject a duplicate live uuid on OPEN. Persist uuid, timeout, and authenticated name across MDS crashes.
- [ ] Implement RESET on an open/stale requesting session. Empty uuid and unsupported flags return `-EINVAL`; a target with a different authenticated name returns `-EPERM`. Target lookup uses uuid, not client global id.
- [ ] If the target exists: blocklist its address, obtain the latest OSDMap containing the entry, raise the barrier, kill and durably close it, then reply with result 0, the old target address, and the post-fencing OSDMap epoch. Recheck the requesting session on completion. A failed blocklist must not produce success or release conflicting rights.
- [ ] If the target does not exist: reply result 0 with an empty address vector. `Client::start_reclaim` converts all-empty RESET replies into `-ENOENT`; FSAL_CEPH accepts that. Do not falsely return a previous address. FINISH alone has no reply; FINISH mixed with other flags returns `-EINVAL`. RESET needs no later FINISH exchange.
- [ ] Test Ganesha-only restart and simultaneous MDS/Ganesha failure separately. Under no-extra-hold, native clients may acquire locks before the absent NFS client reclaims them. This accepted gap must appear in results. Ganesha grace protects NFS-vs-NFS contention, not native CephFS-vs-NFS contention.

Sources: `mds/Server.cc:454-597` (`find_session_by_uuid`, `reclaim_session`, `finish_reclaim_session`, `handle_client_reclaim`), `:1900-1939` (`reconnect_tick`, C++ behavior intentionally not retained); `client/Client.cc:18861-19004` (`start_reclaim`, `finish_reclaim`, `handle_client_reclaim_reply`); `design/minimum-feature-set.md` decision 6. The live runner must enable a takeover-capable Ganesha grace configuration; the helper's graceless mode is not G3 evidence.

### 6. Epoch barriers and recovery work

- [ ] Maintain a monotonic OSD epoch barrier. On eviction: verified blocklist -> latest-map completion containing it -> raise barrier -> durable session cleanup -> release conflicting rights/reclaim reply. Keep the reconnect gather blocked on this chain.
- [ ] Consume `MClientCaps.osd_epoch_barrier` and `MClientCapRelease.osd_epoch_barrier`. Raise the Objecter barrier if its map is behind; propagate the maximum on outgoing cap messages and flush acknowledgments. Do not leave outgoing barriers at zero after fencing.
- [ ] Start purge and lost-writer probes only after prior-owner fencing and reconnect reconstruction establish who still holds rights. Purge must still wait for all caps to disappear. Preserve pending truncate sequence and write-range bounds. Journal recovered size/mtime before output can publish them. Do not treat a probe error as an empty file.
- [ ] Duplicate cap flushes across restart produce acknowledgments only after the durable dedupe/result contract permits it. Do not grant a range larger than its durable range while recovery requests drain.

Sources: `mds/MDSRank.cc:3951-4060` (`evict_client`), `:2597-2609` (active map handling barrier); `mds/Locker.cc:3495-3503` (`handle_client_caps`), `:4270-4277` (`handle_client_cap_release`). C++'s eviction callback ignores command status in this inspected path; follow the approved stricter success/verification contract instead of copying that weakness.

## Existing Rust coverage versus required APIs

| Boundary | Existing representation | Required G3 work, not a new wire type |
|---|---|---|
| Reconnect | `proto::MClientReconnect { caps, realms, more, version }`; `CapReconnect`; `FlockState`; `SnaprealmReconnect` in `crates/proto/src/reconnect.rs` | Session-scoped chunk gather, validated atomic chunk installation, reconnect completion event, bounded reset/retry policy. |
| Locks | `CapReconnect::locks()` returns `Result<FlockState>`; `types::FileLock` preserves owner/pid/client/range/type | Explicit engine adapter to restore held fcntl/flock locks and attach them to authenticated session identity. Do not silently discard locks. |
| Caps and realms | `CapReconnect` includes issued/wanted/id/realm/pathbase/path/snap_follows; `SnaprealmReconnect` includes ino/seq/parent; `SnapFormat::for_client` exists | Reconstruction completion must cover cap-id allocation, sequence reset, realm reconciliation, inode loading, and conflict admission. Existing `Engine::reconnect_cap` / `reconnect_done` hooks alone are not evidence that daemon translation covers all these fields. |
| Session | `MClientSession` supports v0-v7 fields, metadata, features, metric spec, cap_auths, oldest_client_tid, and all defined ops | Durable session repository, restore/rebind, uuid uniqueness, timeout policy, authenticated-name persistence, ordered final OPEN. |
| Reclaim | `MClientReclaim`, `MClientReclaimReply`, `CEPH_RECLAIM_RESET`, `RECLAIM_FLAG_FINISH` in `reclaim.rs` | UUID target lookup, authorization, fenced durable removal, completion-driven reply. |
| Epoch barrier | `MClientCaps.osd_epoch_barrier`; `MClientCapRelease.osd_epoch_barrier` | Connect monotonic daemon/Objecter barrier inputs and outputs. |
| Requests | `RequestHead.oldest_client_tid`, request flags including REPLAY, retry fields, and request releases; reply invariant checker | Restore dedupe state and validation context; classify recovery versus ordinary admission; build valid committed retry replies. |
| Maps | `monc::mdsmap::MdsState::transition_valid`; `MdsMap.last_failure_osd_epoch`, timeout/autoclose, required features, up/in/failed sets; `MdsInfo.inc` | Completion-driven orchestration and explicit zero-epoch fencing proof. No missing recovery state representation. |

Existing evidence is useful but limited: `crates/proto/tests/dencoder.rs::reconnect_v3_and_v5_through_cpp` checks C++ decoding of v3/v5; inline `v3_kernel_layout`, `v5_parts`, and `malformed` cover layouts and delayed lock validation. They do not prove mounted-client recovery, gather lifecycle, or lock restoration. The v3 dencoder test explicitly handles a C++ re-encoding quirk that drops the raw realms; it must not be mistaken for evidence that kernel v3 realms are optional.

## Mandatory targeted hazards for the integration owner

1. Retry and dirty flush arrive before reconnect; the ordinary request queue must not block receipt of the reconnect behind them.
2. Mixed v5 caps/realms, more=true chunk, connection interruption, empty final chunk, malformed flock blob, and duplicate/repeated reconnect. Observe no premature OPEN, duplicate locks, or double gather completion.
3. One missing timeout session plus one successful reconnect. Inject failed blocklist and delayed latest map. Observe no active admission, rights release, or reclaim success before fencing completes.
4. Durable O_EXCL create loses its reply. Race another client's rename/unlink after reconnect. Observe one created inode, no EEXIST from rerunning the create, and a client-valid trace.
5. Hold fcntl and flock locks, keep an unlinked file open, keep dirty writable caps, and restart the MDS. A competitor must not obtain conflicting rights before reconstruction.
6. Lose a cap FLUSH_ACK after durable completion. Resend before reconnect and after OPEN. Observe no duplicate mutation and no acknowledgment of uncommitted attributes.
7. Remove a standby-replay daemon after active failure so `last_failure_osd_epoch` becomes zero. Observe independent prior-owner fencing validation.
8. RESET absent uuid, duplicate live uuid, wrong auth name, FINISH-only, invalid flags, and requesting-session disappearance during target eviction. Verify exact reply/no-reply behavior.
9. Kill the daemon during each transition and during missing-session cleanup. A new holder must recover the same filesystem and retry/session scope. A late completion from the old incarnation must not advance the new phase.

## Not required for G3

Distributed resolve decisions, weak/strong MDS-to-MDS rejoin, cap export/import, forced sessions, cross-rank lock recovery, migration FLUSHMSG barriers, and forwarding belong to G6. G3 still needs the monitor-visible rejoin state and local cap/lock/realm completion. Snapshots, unsafe replies, and async dirops remain deferred. Reconnect realms and the clientreplay hook are still required. Do not add ad hoc recovery states or messages to work around admission ordering.
