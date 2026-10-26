# astra-g3-fault-interface
Task: Propose the smallest daemon-only deterministic fault interface and observations for full G3 after D5 integration. Cover selected durable request replies, dirty flush boundaries, reconnect OPEN, and Ganesha reclaim. No implementation, tests, live actions, or G3 pass claim.
Status: COMPLETE — frozen v1 candidate for C1–C8 consumer re-review. The prior API is superseded. Consumer acceptance, implementation review and qualification remain pending.
Context: This proposal and its embedded ledger are the only edited file. E1 contracts and D6/R1 implementation are accepted; E1 source review and D5 integration remain separate prerequisites. QA application tooling remains accepted. Read the full consumer review, accepted startup seams, local package/source metadata and focused version-pinned upstream peer sources. No unfinished owner code or QA tooling was inspected for this revision.
Next: Parent requests consumer re-review of this exact candidate and daemon-owner API review. Resolve any returned findings before assigning hooks/controller implementation. Later sole-QA preparation must establish the named installed-peer prerequisites; no live authorization follows from this proposal.
Log:
- [2026-10-25] Created ledger before review.
- [2026-10-25] Mapped DaemonJournal -> bounded command handoff -> tagged Committed -> journal_event -> Session funnel. Existing transport acceptance is messenger queue acceptance, not socket delivery.
- [2026-10-25] Completed deterministic cuts, control protocol, evidence schema, Ganesha observations, and focused verification requirements. No tests/builds or Git operations ran.
- [2026-10-25] Final consistency pass fixed startup observation ordering, frame/address bounds, attachment matching and terminal arm states. Confirmed the existing checkpoint gate requires engine quiescence. Proposal is ready for coordinator handoff.
- [2026-10-25] Resumed for the user's C1–C8 revision assignment. Prior completion did not establish consumer acceptance. Only this proposal and its embedded ledger may change.
- [2026-10-25] Resolved C1–C7 with typed startup/discovery/snapshot/exit handshakes, binary selectors, real dirty UPDATE, cumulative inode ACK coverage and the final CrashSealed prefix. Added bounded creator-inode reservation, exact history probes and streamed 64-address/696-item limits.
- [2026-10-25] Resolved C8's provider/predicate contract from installed metadata and Ceph v20.2.0/Linux v7.0/Ganesha V6.5 sources. Specified package-runtime probes, NFS file-handle/session joins, RESET success versus ENOENT, and exact N1–N4 grace/lock outcomes. No observer or peer was started.
- [2026-10-25] Completed the full document consistency read and froze this candidate for re-review. Updated only this file. Ran no tests, builds, live actions, Git writes, commits or subagents. The verification list is future required work, not results.

## 1. Frozen v1 candidate and C1–C8 disposition

This revision replaces the earlier API. It is ready for **consumer re-review**, not consumer ACCEPT or G3 qualification. The daemon owner implements the hooks after D5/E1 integration. QA implements the controller against this same version. Both implementations need independent review before separately authorized live use.

| Finding | Resolved v1 decision | Sections |
|---|---|---|
| C1 | Unsolicited ControllerReady precedes Hello. Checked restoration precedes a streamed snapshot. Only a controller-fsynced SnapshotAck opens admission. | 3, 4.2–4.3 |
| C2 | Eight reserved slots; exact live input probes and a persisted BindAck. No expansion of auth-name matches or Closed tombstones. | 4.4 |
| C3 | Typed Present/Absent items, immutable StatePoint, complete writer-range/address exports, exact history probes and one quiescent SnapshotRequest. | 3, 4.3 |
| C4 | Raw binary path length/hash/base and path-field index; separate head/open flag masks; dirty UPDATE and FLUSH accepted. | 3, 4.5 |
| C5 | A per-session/rank/inode cumulative-ACK guard starts at Bound. Final CrashSealed evidence closes the lost-invalidation race. Peer and syscall receipts remain separate. | 4.6, 5, 6.1 |
| C6 | Strict JSON integer/enumeration rules; one address/range per item; incremental streaming and explicit size bounds. | 3, 4.1–4.3 |
| C7 | ExpectExit distinguishes sealed SIGKILL from NormalStop. Normal exit requires ProcessExiting and ExitAck. EOF alone never proves success. | 4.6 |
| C8 | Installed 6.5-8 feature symbols and 6.5 source/log/FH facts replace V15.7 assumptions. Named peer providers and precise N1–N4 predicates are fixed. Runtime access/loss/identity checks remain prerequisites. | 6.2–6.3 |

| Cut | Exact boundary | Behavior until terminal crash seal |
|---|---|---|
| `ReplyBeforeTransport` | Successful mutation has a checked durable completion. Its validated SAFE reply reaches the session funnel before `ClientLink::send_raw`. | Suppress that logical reply and its regenerated duplicates. Continue cap revokes, renewals and the competing client's work. |
| `FlushBeforeWorker` | The selected dirty flush has sequence q and encoded bytes. Its Append has never entered the worker channel. | Retain that one command and record slot. Continue bounded control/map/connection I/O. Later mutations wait. |
| `FlushAckBeforeTransport` | The selected dirty flush is durable. Its own FLUSH_ACK reaches the funnel before transport acceptance. | Suppress it **and every covering FLUSH_ACK** for the same key/rank/inode with tid ≥ the selected tid. Other output continues. |

All cuts are one-shot and crash-only. There is no post-hit resume. Dropping an output at its funnel position does not count as transport acceptance, increment push_seq, or complete a recovery-output drain. Do not hold a session FIFO head: that can block the cap revoke needed by A's competing rename. The short terminal CrashSealed freeze occurs only after competitor work is complete (§4.6).

## 2. Exact source seams

Locations refer to the read source, not an acceptance review of that source.

| Source symbol | Small owner change |
|---|---|
| `daemon.rs::dispatch_client_raw_at`; `server/runtime.rs::Server::handle`; recovery's `Held` / D5 dispatch | Observe valid authenticated input before admission, with `SessionKey`, connection, attachment token, actual wire tid/op/flags and receive time. Retain tags through D5's early queues and Busy/continuation paths. Bind only the selected request/flush. Do not bind from an unvalidated raw packet. |
| `journal.rs::DaemonJournal::submit`, `Write` | Before encoding consumes `JournalRecord`, copy a small optional observation tag from `completed_request` / `client_flush`. Keep it beside `Write { seq, body }`. Never decode WAL again just for a tag. |
| `server/runtime.rs::Runtime::{command,flush_commands}`; `Server::pump` | Attach the tag to `Pending::Append` and gate the selected Append **before** `tx.try_send(command)`. Both `take_write` paths in pump, including its last-budget-slot transfer, must use this gate. The retained command remains charged and owned. |
| `persistence.rs::worker`, `CommandKind::Append` | Observe first append start before the first `store.append` poll, and successful `EventKind::Committed(seq)` after checked append/reconciliation. Keep existing `operation_id(owner.gid, seq)` and retry bytes. No blocking controller wait in the worker. |
| `daemon.rs::Daemon::on_storage_event`; `server/runtime.rs::Server::on_storage_event` | Only a membership/owner/operation/sequence-matching Committed receipt establishes the selected durable cut. Record it **before** `journal_event` can emit output. Process `DaemonJournal::complete` and `JournalEvent::Committed` normally; do not hold this callback for an output-loss test. |
| `server/runtime.rs::Server::consume_step`; `server.rs::Server::send_effects` | Carry the selected commit reference into reply/FLUSH_ACK output tags. For replies, keep `pending_requests` validation, original op/tid, actual trace/extra identity, and the invariant check. Observe, do not construct alternative replies. |
| `session.rs::{PendingOutput,Session::enqueue,Session::transmit,Session::drain_output}` | Retain tags with queued output. Consult the output cut immediately before `link.send_raw`. Report acceptance only after true. Preserve typed OPEN.seq assignment at transmission and count push_seq only on true acceptance. |
| `server/runtime/recovery.rs::{apply_reconnect,recovery_opened}`; D5 final OPEN producer | Observe final reconstruction and durable attachment separately. Tag D5's final typed OPEN with `purpose=Reconnect`, exact key/token, and final-chunk event ID. The current collection-only path does not yet produce this OPEN. |
| `daemon.rs::{maybe_start_storage,on_storage_event,drive_phase,runnable}` and dispatch/admission gates | Add the feature-only snapshot latch described in §4.2. Existing `ordinary_paused` alone does not stop controls, attachment, cleanup or initial commands. |
| `server/runtime/d6.rs::{reclaim,reclaim_fences,finish_reclaim_fence,finish_reclaims,active_timers}`; `daemon.rs::on_control_event`; `fencing.rs::fence` | Observe the accepted D6 selection, fence/barrier, durable Closing/cleanup/Closed and requester revalidation. Preserve R1's queued-output ownership check at actual transport handoff. |

`Session::send` returning true can mean **queued while paused**. It is not the observation point. `ClientLink` delegates to `msgr::Connection::send` (`crates/msgr/src/conn.rs:193`), which accepts into `out_q`; this is not a TCP write or client acknowledgment. Name the timestamp `transport_accepted_ns`. Actual network/client receipt needs separate peer evidence. No messenger API change is needed for the requested cut.

## 3. Types and evidence identities

These are private daemon/controller types, not Ceph wire or persistent types. JSON objects use exactly the named fields. An enum with no payload variants is a string. A union with any payload variant is always an object with a `kind` string and only that variant's fields; thus Tid.Next is `{"kind":"Next"}`. `?T` means a required field containing T or JSON null. No omitted optional fields or arbitrary metadata maps are allowed.

- `U8/U16/U32/U64` and `I32/I64` are JSON integer tokens with checked Rust ranges. Reject booleans, quoted numbers, fractional/exponent forms, negative unsigned values and overflow. IDs/counters never wrap. `V1` is the integer 1.
- `H128/H256` are exactly 32/64 lowercase hexadecimal ASCII characters. `Bool` is a JSON boolean. All other strings are the finite enum spellings below. Free-form error text and raw paths are absent.
- An `EventRef` is `{process:H128,event:U64}`. An external `ProofId:H256` is SHA256 of a fsynced artifact. The daemon echoes external proof IDs; it does not claim to verify their contents.

```text
Key = { client:U64, incarnation:U64 }
Owner = { shard_epoch:U64, rank:U64, incarnation:U64, gid:U64 }
Attachment = { connection:U64, generation:U64, recovery_generation:?U64 }
CommitRef = { owner:Owner, operation:U64, seq:U64, batch_id:H128 }
  // batch_id is the existing operation_id(owner.gid, seq), not operation counter.
ReqId = { key:Key, tid:U64, op:U32 }
FlushId = { key:Key, tid:U64 }     // a distinct identity domain
Mask32 = { mask:U32, value:U32 }   // value & ~mask must be zero
Path = { field:Path|Path2, base:U64, len:U32, hash:H256 }
Addr = { type:U32, family:IPv4|IPv6, ip:H128, port:U16,
         scope:U32, nonce:U32 }
AddrSetRef = { process:H128, set:U64, count:U8, digest:H256 }
StatePoint = { owner:Owner, tail:U64, engine_durable:U64,
               checkpoint_generation:U64, coverage:U64, trim:U64,
               manifest_checksum:U32, session_operation:U64,
               startup_operation:U64, osd_barrier:U32 }
Cut = ReplyBeforeTransport|FlushBeforeWorker|FlushAckBeforeTransport
Guard = ReplyIdentity|CumulativeInodeAck|WorkerNotSubmitted
ExitMode = CrashAfterConfirmedHit|NormalStop
Timeout = Absent | Valid { seconds:U32 } | Malformed { hash:H256 }
```

IPv4 uses twelve zero bytes followed by its four network-order bytes; scope=0. IPv6 uses sixteen network-order bytes and the numeric scope ID. Preserve Ceph address type and nonce. No textual address formatting is part of matching. Address vectors/old fencing unions have at most **64 distinct entries total**, not 64 per attachment. Export their existing order; reject duplicates. Digest = SHA256(concatenated entries encoded as `type:u32le,family:u8(4|6),ip:16 bytes,port:u16le,scope:u32le,nonce:u32le`). A larger union cannot be silently shortened and is outside v1 trial scope.

```text
InputRequest = { kind:Request, input:U64, probe:U8, id:ReqId, attachment:Attachment,
  received_ns:U64, retry:U32, head_flags:U32, open_flags:?U32,
  path:Path, path2:Path, oldest_tid:U64 }
InputFlush = { kind:Flush, input:U64, probe:U8, id:FlushId, attachment:Attachment,
  received_ns:U64, caps_op:U32, flags:U32, ino:U64, cap_id:U64,
  seq:U32, issue_seq:U32, mseq:U32, dirty:U32, oldest_flush_tid:U64,
  osd_barrier:U32, size:U64, max_size:U64, truncate_seq:U32 }
WalTag = Request { id:ReqId, result:I32, created_ino:?U64,
                   reply_target_ino:?U64 }
       | Flush { id:FlushId, oldest_flush_tid:U64 }
OutputTag = { output:U64, key:Key, attachment:Attachment,
  origin:Ordinary|RetryHit|FlushAck|ReconnectOpen|OrdinaryOpen|ReclaimReply,
  input:?U64, cause:?CommitRef }
Positions = { submitted:U64, adapter_committed:U64, engine_durable:U64,
  worker_enqueued:Bool, append_started:Bool, store_confirmed:Bool }
Counters = { selected_accepted:U64, cover_accepted:U64,
  exact_suppressed:U64, cover_suppressed:U64, max_ack_accepted:U64,
  push_before:U64, push_after:U64 }
```

`RetryHit` is an observed pre-step lookup in canonical completed history, with its actual subsequent output. It is not inferred from eventual application success. A retry with no new record has `cause=null`. Keep the original external CommitRef when WAL trim removes it. Do not invent an original commit sequence from a recovered tail. Observe all new selected-key/ino records through the seal/recovery end, including range-only records under their own CommitRef.

Local input/output/arm/snapshot/address/reclaim IDs are nonzero U64 counters, unique within their domain and process. An input ID identifies one received logical message, including each genuine retransmission; duplicate control responses reuse IDs. Event IDs identify transcript records, not Ceph messages. Assign a history probe to each new validated REQUEST or nonzero-tid dirty CAPS identity from a watched live key; a retry reuses its existing probe. Use the lowest unused index in the 32-entry table and expose it in Input. Preserve the exact `(key,domain,tid,op?)`; never evict a probe or conflate two opcodes. Discovery's matching input is watched too. Other session controls do not consume history entries. Emit watched input even if admission must wait; Decision records actual admission. Exhaustion is ScopeExceeded before executing that input. This covers the second client's input/commit/output without adding another fault arm.

Positions' sequence fields describe the current adapter/engine. Its booleans describe the selected Append. For a flush arm, Counters.exact_suppressed counts ACK tid=T; cover_suppressed counts strictly greater tids. cover_accepted counts all ACK tids≥T, including exact T; selected_accepted counts exact T. max_ack_accepted is the watched inode's accepted high-water tid, or zero if none. For a reply arm, exact_suppressed/selected_accepted count that logical reply; ACK-specific fields are zero. `push_before/after` describe the selected boundary, not unrelated later counted output. Timeout preserves the accepted distinction between absent, valid zero and malformed metadata; hash malformed text without exposing it. Valid seconds are the checked D6 u32 value.

Synchronous owning-task seams remain `observe_input`, `before_worker_handoff -> Pass|HoldNotSubmitted`, `observe_commit`, `before_transport -> Pass|SuppressForCrash`, `observe_transport` and `observe_transition`. Add only daemon-local startup/snapshot/discovery/exit latches. One `FaultState` owns them. Async socket/worker tasks report bounded events; they do not mutate the engine. No reactor-blocking I/O or shared blocking mutex is required.

**Cross-owner dependency:** accepted E1's `CompletedRequest.reply_target_ino: Option<Ino>` is required for existing-inode CREATE/OPEN. Use the completion's `created_ino` for a new creator. Never infer identity from an inode-image position or a later lookup. `ClientFlush` already supplies client/incarnation/tid. No other engine input, persisted field or schema change is proposed. Source adoption still waits for independently accepted E1 implementation and D5.

## 4. Local control protocol

### 4.1 Framing, bounds and persistence receipts

Use Cargo feature **`g3-faults`**, off by default, and startup-only **`--g3-fault-socket=<absolute-path>`**. A normal build rejects this option explicitly. A feature build without the option uses normal semantics. Compile the synchronous seams under `cfg(test)` too. Later builds use a separate target directory and preserve reviewed binary SHA256 `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`.

One Unix stream peer; u32 little-endian length followed by **1–16384 bytes** of strict UTF-8 JSON. Encoders emit compact JSON. Decoders reject duplicate/unknown fields, invalid union fields, malformed numbers, trailing JSON, excessive nesting (>8) and incomplete frames. Read/validate length before body allocation. Reject unknown version/kind; there is no extension map. The private 0700 parent belongs to the daemon effective uid, has no symlink components, and lies under `/tmp/opencode`. Bind a fresh 0600 endpoint after privilege setup. Never unlink an old endpoint. Require matching peer uid via SO_PEERCRED. The controller also binds peer PID, boot ID, `/proc/PID/stat` start ticks, executable hash and pidfd. Setup may use blocking setup work; reactor socket I/O stays async.

Every command after Hello carries `{v:1,run:H128,process:H128,command:U64,kind,body}`. Hello uses the same envelope with the run selected by QA. Every event carries `{v:1,run:?H128,process:H128,event:U64,mono_ns:U64,reply_to:?U64,kind,body}`. Only initial Ready has run=null; invalid input before accepted Hello closes the process without another event. `kind` is the exact command/event name defined here. `reply_to` is the command ID for its one response, otherwise null. `process` is a fresh random nonce. `mono_ns` is **CLOCK_MONOTONIC since this host boot**, not process-relative Instant elapsed time. Include boot ID in Ready; reject cross-boot latency subtraction. Wall-clock packet/log timestamps need recorded clock correlation and uncertainty; they cannot replace this clock.

Event IDs start at 1 and are contiguous across **all scoped records**, including responses. Command IDs start at 1 (Hello), increase by one, and have one cached last-command response. Only one command may await its response. A byte-identical duplicate last command repeats the identical cached response/event ID; it creates no new event or hash step. A pending command is not executed twice. Reject stale/conflicting IDs. The controller accepts a repeated event only as that exact cached response. No reconnect-to-suffix facility exists.

For persisted-prefix receipts, let `H0=32 zero bytes`; `Hi=SHA256(H(i-1) || u32le(frame_len) || exact event JSON bytes)`. A `Persisted={through:U64,digest:H256}` identifies that complete prefix. The controller first writes all frames through that ID, fsyncs the transcript and its initial directory entry, then sends the receipt. Keep these artifacts on the local evidence filesystem, outside the tested mounts. Continue reading asynchronously while fsync runs. The daemon checks the ID/hash against its generated prefix. This is a controller durability assertion, not an OSD receipt.

Bounds: 8 incoming control frames; 64 outgoing slots, four reserved for responses/failure; 8 session slots total; 16 inode probes; 32 request/flush probes/tags total; one active fault arm; one retained Append; one cached command response; one hit; one snapshot export. Slot/probe IDs are U8 indices within those limits. No silent eviction. Split a larger suite before admission. Charge queued tags to the existing output budgets. Keep at most 128 key/inode ACK high-water cells (8×16). Counters replace repetitive suppression events explicitly.

Address and range arrays never appear in Ready/Hit/Status/Session items. A snapshot streams one address or writer range per item. A new live address union uses `AddressesBegin {set:AddrSetRef}`, then `Address {set:U64,index:U8,value:Addr}`, then `AddressesEnd {set:AddrSetRef}`. References are usable only after all indices 0..count-1 and the digest match. At most eight retained sets plus one pending replacement exist; unchanged sets reuse their reference. Membership changes cannot overwrite an old referenced set. Stream a legal 64-entry union incrementally. Backpressure pauses the export cursor/operation that needs it, not a reactor thread or another session. Later scoped observations still use the bounded event queue; overflow fails the trial. A snapshot/address exporter must not enqueue its whole expansion into 60 ordinary slots.

**Encoding bound:** each SlotSpec is <160 bytes, InodeProbe <64, HistoryProbe <256 in compact maximum-width form. The largest Hello/SnapshotRequest is bounded by 2048 fixed bytes + 8×160 + 16×64 + 32×256 = **12544 bytes**. Discovery/Arm/Bound/Hit have at most two Path values, one input, one output summary and one CommitRef; budget **8192 bytes including envelope**. Other control/events/items are ≤4096 bytes including envelope. Variable strings are only fixed-width hashes or the named enums. A Session item carries an AddrSetRef, not 64 addresses; an Inode item carries a range count, not a range array. These structural limits make every legal v1 frame encodable below 16384. Later codec verification must encode every maximum-width shape; this assignment ran no such test.

### 4.2 Startup and admission barrier (C1)

```text
Ready body = { stage:ControllerReady, pid:U32, start_ticks:U64, boot:H128,
  clock:Monotonic, owner:null, loaded_tail:null, max_frame:16384,
  control_slots:8, event_slots:64, session_slots:8, inode_slots:16,
  history_slots:32 }
SlotSpec = { slot:U8, key:?Key }       // null reserves discovery capacity
InodeProbe = { probe:U8, ino:?U64 }     // null reserves a creator's target slot
HistoryProbe = { probe:U8, slot:U8, kind:Request|Flush,
  tid:U64, op:?U32, origin_proof:?H256 } // op required only for Request
Probes = { sessions:[SlotSpec;<=8], inodes:[InodeProbe;<=16],
           histories:[HistoryProbe;<=32] }
Hello body = { salt:H256, fsid:H128, filesystem:U64, metadata_pool:I64,
  data_pool:I64, probes:Probes }
SnapshotAck body = { snapshot:U64, end:U64, persisted:Persisted }
```

Order: verified connection → **unsolicited Ready** → Hello/HelloAccepted → checked native load → StartupPreResend snapshot → controller fsync → SnapshotAck/SnapshotReleased → ordinary recovery gates. Ready is neither Loaded nor ACTIVE. First loaded owner/positions occur in SnapshotBegin. Hello binds fs44/meta88/data89 and the cluster FSID before use. Restart Hello supplies the exact old keys/tids/inodes and original proof IDs from the prior fsynced Bound/Hit; it never rediscovers history by auth name.

Concrete daemon latch: `maybe_start_storage` waits for Hello. After `Server::from_loaded` completes checked replay/join in `Daemon::on_storage_event`, hold **all client and recovery execution** before the first pump/phase drive. Do not call attachment/OPEN handlers, E2 finish/marker, early retry/flush drain, D6 expiry/eviction, jobs or checkpoint publication while held. `from_loaded` currently queues TrimInitial and possible Closed receipts. Permit **only the existing TrimInitial command and its checked InitialTrimmed receipt**, without running pump afterward. It trims already-covered WAL, not the restored metadata image. All Closed commands remain held. This supplies an actual trim position for StatePoint; never substitute coverage for an unobserved trim. Then export the snapshot and wait for SnapshotAck.

`ordinary_paused` is insufficient and D5 has no implemented snapshot gate. Add this daemon-only admission predicate to dispatch, pump/handoff, phase/expiry/job drives and runnable checks. Keep monitors, beacons, storage completion delivery, authenticated connection reads and controller I/O runnable. Queue early peer messages under existing bounded recovery-input budgets, with original receive times. No client output or metadata execution occurs before release. Overflow/authority failure stops the trial. Do not reset D6's monitor-confirmed reconnect time to hide observer delay; if a deadline elapsed, apply the accepted policy only after release and record it.

Hello, each discovery and each snapshot export/ACK have fixed 30 s deadlines from Ready, ProbeInstalled and snapshot hold respectively. An active query's hold includes its quiescence wait. Arm supplies `lifetime_ms` in 1..300000, converted once to an absolute monotonic deadline. No extension, quiet timer, sleep-based cut or timeout-based success exists.

### 4.3 Typed snapshots and quiescent query (C3)

`SnapshotRequest body={owner:Owner,probes:Probes}` is the only state query beyond Status. It may fill reserved slots with already resolved exact keys and add probes within the fixed limits. It cannot delete/replace old probes or scan a table. It is valid only in Observing, outside discovery/arm/stop. A new inode may come from a preparatory completed creator's observed target. An active query pauses new admission, drains already admitted work under normal rules, and waits for engine quiescence, submitted=committed=durable, and no outstanding worker/session/checkpoint/job completion. No polling; use real completion wakes. If a stable point is unavailable by the deadline, fail rather than label a speculative image durable.

Capture the bounded metadata image at that point and hold execution until SnapshotAck. Clientless baseline sessions, histories and inodes come from the checked engine image and joined rank rows. Do not derive snapshots from a raw speculative inode getter. The initial snapshot purpose is `StartupPreResend`; later snapshots are `QuiescentBaseline`, never retrospective pre-resend proof. `session_operation` is the last checked rank-session command ID (startup ID at load), not an invented repository revision.

```text
Counts = { sessions:U8, requests:U8, flushes:U8, inodes:U8,
           addresses:U16, ranges:U16 }
SnapshotBegin = { snapshot:U64, purpose:StartupPreResend|QuiescentBaseline,
                  point:StatePoint, counts:Counts, items:U16 }
SnapshotItem = { snapshot:U64, ordinal:U16, value:Item }
SnapshotEnd = { snapshot:U64, begin:U64, point:StatePoint,
                counts:Counts, items:U16 }
Item = Session { slot:U8, key:Key, presence:Present|Absent,
  lifecycle:?(Open|Closing|Closed), auth:?H256, uuid:?H256, timeout:?Timeout,
  addresses:?AddrSetRef, attachment:?Attachment, cleanup_seq:?U64,
  oldest_request:?U64, oldest_flush:?U64 }
 | Request { probe:U8, slot:U8, tid:U64, presence:Present|Absent,
  absence:?(UnknownSession|ClosedSession|NoEntry),
  stored_op:?U32, result:?I32, created_ino:?U64, reply_target_ino:?U64 }
 | Flush { probe:U8, slot:U8, tid:U64, presence:Present|Absent,
  absence:?(UnknownSession|ClosedSession|NoEntry) }
 | Inode { probe:U8, ino:U64, presence:Present|Absent,
  size:?U64, max_size_ever:?U64, max_client_range:?U64,
  truncate_seq:?U32, truncate_size:?U64, truncate_from:?U64,
  version:?U64, change_attr:?U64, ranges:U8 }
 | WriterRange { inode_probe:U8, index:U8, key:Key,
                 first:U64, end_exclusive:U64 }
 | Address { slot:U8, set:U64, index:U8, value:Addr }
```

Emit one Session item per slot with an **exact key**, and one Inode item per probe with an exact inode. Unresolved reservations produce no item. Probe/slot indices must be unique. History probes must reference resolved exact keys. A Session with presence=Absent has all optional fields null. Present requires lifecycle/auth/timeout/addresses; uuid and current attachment may be null. Open has cleanup_seq=null; Closing may have a checked cleanup receipt awaiting rank Closed; Closed requires its matching cleanup_seq. Durable request/flush floors are present only while the joined engine session exists. UnknownSession means no checked rank row for that exact key. Closed is Present/Closed, not absence. History Absent/NoEntry means exact lookup found no entry in a known non-Closed key; it does not distinguish never committed from durably retired. The original external proof and durable watermark decide that. Present Request requires stored_op/result and absence=null; created_ino/reply_target_ino retain their real Option values. Present Flush has absence=null. All Absent requests have null stored_op/result/targets and the exact absence reason. Read/validation errors are failures, never Absent.

Inode Absent means no inode image at this checked point; all scalar options are null and ranges=0. Present scalars are required. Export **all** durable ranges, sorted by Key; each uses first=0 and the actual exclusive write bound. Join client IDs to their checked session incarnations. All range owners must be within the eight reserved/resolved keys; an extra owner is ScopeExceeded, not truncation. Missing completion and missing inode are independent facts. With eight owners, each inode has at most eight range items. Max snapshot expansion is 8+32+16+512+128 = **696 items**. Export only these scoped fields, not a second whole-namespace checkpoint copy.

Ordinals are exactly 0..items-1. Order: Session by slot with its Address items by index; Request/Flush by history probe ID; Inode by probe ID with its WriterRange items by index. SnapshotEnd.begin is SnapshotBegin's event ID. Begin/End point and counts must be identical; items equals the sum of all six counts. Addresses referenced by a Session item are complete only after its indexed Address items match the digest. SnapshotAck must name End and the complete fsynced event prefix through End. Missing, duplicated, reordered or surplus items invalidate the snapshot. The bounded immutable export streams under backpressure while admission stays held.

### 4.4 Bounded live discovery (C2)

Use **owned-path probe discovery** for kernel/FUSE. Do not require a UUID or guess a SessionKey from a mount UNIQ-ID. Kernel debug/trace and the independent msgr1 observer in §6 corroborate the live client global ID. They do not authorize enumeration of all `client.admin` rows.

```text
ResolveSlot = { owner:Owner, slot:U8, auth:H256, probe:Discovery }
Discovery = OwnedPath { request:RequestSelector }
          | Reset { uuid:H256, old_target_slot:U8 }
          | UuidOpen { uuid:H256, predecessor_slot:U8 }
SessionBound = { slot:U8, key:Key, attachment:Attachment, auth:H256,
                 peer:Addr, matched_input:?U64, lifecycle:Open }
BindAck = { slot:U8, bound:U64, peer_proof:H256, persisted:Persisted }
```

ResolveSlot is valid after SnapshotReleased, in an unused reserved slot. It records an installation event ID. Match only authenticated **current live** input received afterward. OwnedPath/Reset require lifecycle Open and current connection/generation. UuidOpen instead matches authenticated OPEN input and binds only after its durable Open receipt. Never search persisted auth/UUID rows for the requester. An exact old target can separately occupy a slot even when Closed. Resolved slots and unresolved reservations together total at most eight. Do not recycle a slot to hide a prior incarnation.

OwnedPath uses a new one-component random ASCII name under a physically verified private parent. Use an ordinary MKDIR probe with the exact observed parent inode, raw basename, flags and opcode; retain its intent/result. Calibration uses a different ordinary name and scoped wire observation to establish the actual representation. A cached LOOKUP or assumed local absolute path is not a provider. If the representation differs, do not broaden the selector; fail preparation and specify a new exact probe. RequestSelector's mandatory REPLAY=false applies here too.

At the match, hold that one decoded probe input before execution and emit SessionBound with nonnull matched_input. QA joins it to the exact owned mount/worker or process socket, fsyncs the independent peer proof and prefix, then sends BindAck. Recheck key/connection/generation and release that same input through its normal lane. A second matching **different** key/attachment, mismatched auth or failed proof is Ambiguous/Invalid. Do not choose the first historical or live auth-name match. Allow at most one held discovery input per slot, charged to existing input bounds; simple v1 probe input size is capped at 16 KiB. Duplicate wire delivery is not a new candidate.

For Ganesha, install Reset discovery **before launching the owned successor**. The exact expected UUID comes from the retained 6.5 node/export identity; old_target_slot is the exact old key. `Client::start_reclaim` first opens a temporary session. On its authenticated RESET receive, bind that temporary current key and hold RESET **before `d6::reclaim`** until BindAck proves the new Ganesha PID/start/socket/peer anchor. Install UuidOpen in a second reserved slot before releasing RESET. It may reference the still-unresolved Reset slot; it becomes eligible only after that predecessor binds. Match only its authenticated client entity and exact UUID. Bind at checked durable OPEN completion and hold that typed OPEN output until BindAck. This is the sole discovery variant that executes OPEN before binding. Preserve temporary-session Closing/Closed observations.

If UUID OPEN retains the same key, its new slot becomes an explicit alias. Both slots still consume the eight-slot budget, but runtime watches/history/high-water cells use the key once. Automatic history assignment uses its lowest slot. Snapshots include each resolved slot, including aliases, with identical key state; never reinterpret an alias as another client. This follows observed behavior rather than assuming UUID update always creates a new key.

Bind the initial preparatory Ganesha instance with OwnedPath using an ordinary MKDIR through its owned NFS export and the §6 inode/RPC join. Its initial absent RESET is preparation, not N1. Snapshot its actual UUID/key before installing successor discovery; no unknown old-target key is guessed.

A peer proof contains the owned boot/mount or PID/start identity, actual client entity `client.<global_id>`, observed advertised address/nonce, TCP stream tuple and packet/log reference. The server's connection ID is an additional local token, not a TCP port or process PID. A candidate with no independent mapping cannot be bound. After a daemon restart, watch an exact retained key's new authenticated attachment and emit AttachmentBound with the same SessionBound fields; matched_input may be null for a connection-only attachment. This exact-key observation adds no discovery wait or second SnapshotAck. A new owner/token must never inherit an old arm.

### 4.5 Selectors, arm and hit messages (C4)

```text
Tid = Next | Exact { value:U64 }
RequestSelector = { kind:Request, op:U32, tid:Tid, paths:[Path;1..2],
                    head:Mask32, open:?Mask32 }
FlushSelector = { kind:Flush, ino:U64, tid:Tid, caps_ops:[U32;1..2],
                  flags:Mask32, required_dirty:U32 }
Arm = { owner:Owner, slot:U8, attachment:Attachment, arm:U64, target_probe:U8,
        cut:Cut,
        selector:RequestSelector|FlushSelector, lifetime_ms:U32 }
Armed = { arm:U64, owner:Owner, key:Key, attachment:Attachment, target_probe:U8,
          cut:Cut, selector:RequestSelector|FlushSelector, deadline_ns:U64 }
Bound = { arm:U64, input:InputRequest|InputFlush }
Hit = { arm:U64, hit:U64, cut:Cut, input_event:U64, commit:CommitRef,
        output:?OutputTag, positions:Positions, counters:Counters,
        guard:Guard }
AckHit = { arm:U64, hit:U64, persisted:Persisted }
CutConfirmed = { arm:U64, hit:U64, hit_event:U64,
                 guard:Guard, counters:Counters }
Cancel = { arm:U64 }
Status = {}  // response is state, owner?, active arm/hit EventRefs?,
             // positions?, counters?, deadline_ns?; no copied history tables.
```

Arm requires released startup snapshot, Serving, exact loaded owner and bound live attachment, no pending discovery/query/stop, and an unused one-shot arm. Reply cut accepts only RequestSelector; flush cuts accept only FlushSelector. Cancel is allowed only before Bound. State sequence is `Observing → Armed → Bound → AwaitBoundary → Hit → HitAcknowledged → Sealing → CrashSealed`. A cancelled unbound arm returns to Observing. Bound consumes the process's one shot. A failed/nonmutating candidate gives terminal NotApplicable, not re-selection. All invalid states fail-stop; no post-hit release exists.

`target_probe` names a reserved inode slot. Flush arms require it already to equal selector.ino. At Encoded, a request arm resolves an empty slot from the actual completion target, or checks its existing inode. Emit `InodeBound={probe:U8,ino:U64,input:U64,commit:CommitRef}` before any committed image/output. A mismatched or missing creator target is terminal Candidate failure. This permits A0's not-yet-allocated inode without guessing or a post-hit query. Hit.input_event is Bound's event ID. Its CommitRef identifies the assigned Append even for B1; only Positions.store_confirmed and checked Journal stages establish durability. B1 has output=null; output cuts require the selected OutputTag and preceding Suppressed summary.

Path.hash is **SHA256(raw 32-byte run salt || decoded filepath path bytes)**. Path.base and Path.len are compared separately. Exclude filepath version/length prefixes. Preserve binary bytes, empty paths and embedded slash bytes exactly. Do not normalize, UTF-8 transcode, hash the printed `#ino/name`, or use a local mount pathname. `field` selects path or path2, with no duplicates. InputRequest always reports both. RENAME path is destination, path2 source; SYMLINK path2 is target content and is only hashed. Head flags are `ceph_mds_request_head.flags`; open flags are `args.open.flags` in **CEPH_O_*** namespace. `open=null` means no open-argument predicate, not zero flags. Non-CREATE/OPEN selectors must use null. Every ordinary request selector tests head REPLAY bit 1 as zero. CEPH_O_CREAT=64, CEPH_O_EXCL=128 and CEPH_O_TRUNC=512 are distinct from head flags; derive wire flags through the peer's sys2wire mapping.

Kernel paths may be `(inode, empty)` or `(parent, basename)`; libcephfs builds inode-relative paths. Calibrate each client/case on its isolated parent. Kernel CREATE may strip O_TRUNC and send another operation. No matching actual mutating record means the requested case was not established; never manufacture CREATE/OPEN or wire REPLAY.

FlushSelector permits only real **UPDATE=5 and FLUSH=7** in caps_ops, without duplicates. libcephfs sends dirty UPDATE. Require nonzero actual flush tid and dirty mask, with `(dirty & required_dirty)==required_dirty` and required_dirty!=0. Check cap-message flags separately. FLUSHSNAP=9 is not this case. Bind actual cap_id/seq/issue_seq/mseq, flags, barrier and oldest tid from InputFlush. A completed tid is not a fresh candidate. Cap ID is an observation, **not** a covering-ACK discriminator (§5.3).

### 4.6 Persistence barriers, final crash seal and normal stop (C5/C7)

The controller fsyncs Bound/Hit and sends AckHit naming the complete prefix through Hit. It fsyncs CutConfirmed before competitor work. Neither acknowledgment releases a fault. Any required competitor input/CommitRef/accepted SAFE reply and fsynced external syscall/durability receipt must exist before the next command.

```text
ExpectExit = { owner:Owner, mode:ExitMode,
  hit:?U64, evidence:H256, persisted:Persisted }
ExitReady = { mode:CrashAfterConfirmedHit, seal:U64, hit:U64, guard:Guard,
  point:StatePoint, positions:Positions, counters:Counters,
  held_command:Bool, evidence:H256 }
StopAccepted = { mode:NormalStop, evidence:H256 }
ProcessExiting = { reason:NormalStop, point:StatePoint,
  held_command:false, prior_event:U64 }
ExitAck = { exiting:U64, persisted:Persisted }
Failure = { class:NotApplicable|Invalid,
  reason:Protocol|Identity|Ambiguous|ScopeExceeded|Capacity|Deadline|
         ControllerLost|Storage|Transport|PeerProof|Candidate|CoverAccepted }
```

For CrashAfterConfirmedHit, hit is required and evidence is the digest of the controller's fsynced pre-kill proof manifest, including competitor and pending-call/peer receipts. The daemon checks the pinned Status prefix (§5.4), the earlier acknowledged Hit/CutConfirmed chain, identity, guard counters and sticky failure. It enters **Sealing**: stop new admission/jobs, finish already admitted completions/output with the same suppression guards, and wait for no worker/data-job/continuation operation. Unadmitted inputs may remain in existing bounded queues. B1 alone retains its one not-enqueued Append and pending engine mutation. A/B2 must be otherwise quiescent. Sealing has the original arm deadline; inability to reach it is failure.

Atomically set **CrashSealed before emitting ExitReady**. This freezes engine/session changes, worker handoffs and every client transport send. It does not wait synchronously. Socket and monitor I/O stay runnable. This is the explicit final freeze **after** A's competing rename, not a session-wide pause during the cut. ExitReady contains the final positions/counters and is the **last event of a successful crash process**. No later data-plane action can create a covering ACK or a hidden invalidation. Any failure before sealing prevents ExitReady. The daemon may terminate unsuccessfully on lost controller/authority/deadline while sealed; such a process exit is not the expected crash.

QA waits for and fsyncs ExitReady and the full prefix, then signals only that bound pidfd with SIGKILL before the recorded deadline. It retains its own monotonic signal receipt and actual wait status. EOF is expected only after that signal and exact SIGKILL status. EOF while the process remains alive, before the intended signal, or with another exit status fails the case. **A gap-free prefix ending at Hit/CutConfirmed is insufficient**: an invalidation could still be queued. The immutable ExitReady boundary closes that race. Independent peer evidence of a covering ACK or an early application return still overrides server counters and fails the case.

NormalStop requires hit=null and is valid only in Observing with no bound/hit cut, held command, discovery or snapshot. It disables new arms and emits StopAccepted while normal client close/unmount/drain proceeds. The controller stays connected, validates normal unmounts, and signals the owned daemon's normal stop only afterward. After real drain, the daemon emits ProcessExiting as its final event and waits for a controller-fsynced ExitAck before exit 0. prior_event is its event ID minus one. ExitAck is the only command without a response. Use fixed 30 s deadlines for daemon drain after the normal-stop signal and for ExitAck. No unsafe/forced unmount or detach-to-normal fallback exists. A recovered **new observing process** can use this path; the injected process cannot.

Unexpected EOF/invalid input/loss/capacity/deadline after admission is runtime failure, even if a preceding prefix looks complete. A B1 command is never handed off by any error, TERM or shutdown path. Started backend writes remain unknown until normal checked reconciliation. External outcomes are prerequisite=2 before admission, runtime/evidence failure=1 after admission, preparation-only=77/PARTIAL, or 0/FAULT_CASE_PASS for the named case with all predicates. None sets `full_g3_qualified=true`.

## 5. Cut proofs and runtime observations

### 5.1 A: durable mutation, no accepted SAFE reply

1. Bind validated real input to `(SessionKey, wire tid, original op, attachment)` and record REPLAY=false. Match its successful `CompletedRequest`, assigned sequence and original target. A pure volatile OPEN is not a candidate.
2. Allow the native append and its checked callback. Record `CommitRef`, loaded owner, expected/confirmed tail, `DaemonJournal::complete` and engine durable sequence. Store success must mean the checked append or exact uncertain-outcome reconciliation succeeded. A local submission/worker enqueue is not durability.
3. Build and validate the normal SAFE reply. At its funnel position, latch suppression, emit its Output/Suppressed summary, then Hit before calling any transport for it. Record result, safe bit, target ino, dentry parent ino/name fingerprint, create-extra ino, trace/snap presence, and zero/nonzero cap/range summary. Keep the response absent from accepted transport counters. A preceding committed callback must release the record slot normally.
4. The second client now performs and durably acknowledges rename, replacement or unlink of the selected original. Require its independent input/completion/accepted reply and fsynced external operation result. Keep the original call pending. Then obtain and fsync CrashSealed ExitReady before killing the MDS. Do not suppress the first client's cap revokes/renewals. If the real peer cannot reach the changed-path state while its syscall waits, fail this case as unestablished; do not inject fabricated messages.
5. After restart, fsync the StartupPreResend snapshot and acknowledge it before any drain. Observe the genuine same-key/same-tid/same-op retry with REPLAY=false. The server owner and transport generation change; the durable session incarnation does not. Apply accepted E1: unchanged identity returns recorded success with a valid trace; changed identity returns SAFE EINTR with only the original target, zero caps/max_size, no dentry/snap trace, and original CREATE-only creation extra. Verify no second completion/allocation/truncate or replacement-inode substitution. A fresh writer-range transaction has its own CommitRef and is not repeated creation. Keep the original external request result distinct from the retry reply's EINTR.

Keep the external original CommitRef when checkpoints trim its record. Restored histories lack an original sequence field today. Compare checked restored identity/result/target with that external receipt; do not invent a recovered sequence. A new application call/new tid after EINTR is a separate operation, not proof of same-tid replay. Do not manufacture wire REPLAY. Its accepted traceless branch remains a separate offline protocol case.

### 5.2 B1: proven not handed to the backend

The word **submitted** is ambiguous here. `DaemonJournal::submit` assigns `q` and queues encoded bytes. `take_write` marks adapter `in_flight` before the worker sees them. Neither means a backend write began.

Hold in `Runtime::flush_commands` before the selected command's first successful `try_send`. Require Positions `{submitted:q,engine_durable:d,adapter_committed:d,worker_enqueued:false,append_started:false,store_confirmed:false}`, q=d+1, the exact command CommitRef, and Counters.selected_accepted=cover_accepted=0. Do not drop/cancel the pending engine mutation or release its record slot. Both pump transfer paths must be covered. A held command is not runnable work; wait on real control/input events without polling the hold. One-record serialization blocks later WAL records. Keep the existing `engine.is_quiescent()` checkpoint gate so the pending mutation cannot enter a checkpoint. Maps, renewals and controller traffic continue; other mutations necessarily wait behind this one uncommitted record.

For B1 too, require the watched inode's accepted ACK high-water mark < selected tid at Bound. Observe the same cumulative coverage predicate through CrashSealed. Any exact/covering ACK acceptance invalidates B1; holding an Append alone does not excuse an ACK already queued from another source. B1 does not manufacture an ACK to drop.

Only this **not-handed-to-worker** state proves this selected metadata record was not submitted to storage. A paused worker completion, dropped future, timeout or UnknownOutcome after `append` started does not prove non-durability. Label such evidence `StartedOutcomeUnknown`; reconcile it through existing store behavior and do not count it as B1. No special backend cancellation or rollback is proposed.

After the crash, observe checked load **before** the client's resend is applied: selected flush tid absent from durable dedupe, prior metadata/writer-range state intact, and no selected record at the frozen durable tail. Then observe the peer's same-tid flush resend and its one actual durable completion/ACK. File data already written to OSDs can exist even when this metadata flush did not commit; B1 makes no claim that data writes were undone.

### 5.3 B2: durable flush, no accepted exact or covering ACK

**Source facts, not a new peer policy.** Ceph **v20.2.0** `src/client/Client.cc::handle_caps` (5508–5596) selects the authenticated MDS session/connection, head inode and that rank's cap. It does not compare cap_id for FLUSH_ACK. `handle_cap_flush_ack` (5707–5771) removes non-snapshot pending flush tids **≤ the ACK tid** from that inode. Dirty-mask intersection affects cleaned bits, but is not a guard against retirement of an earlier tid. `handle_cap_flushsnap_ack` is a separate path. Linux **v7.0** `fs/ceph/caps.c::{ceph_handle_caps,handle_cap_flush_ack}` (4328 onward; 3811–3922) also selects head inode/rank, removes earlier non-capsnap flush entries, and does not use cap_id as this guard. The local 7.3 snapshot confirms the same behavior. Kernel source is a behavior reference only; do not translate it into Rust.

For selected `(key K, rank R, ino I, tid T)`, define:

```text
covers(output) := destination_key == K && rank == R &&
                  caps_op == FLUSH_ACK(8) && ino == I && ack_tid >= T
```

Ignore cap_id, seq, mseq and dirty-mask equality in this predicate. Record them for correlation. Another inode's higher tid does **not** complete this inode's flush. Request tid and flush tid are different domains. Cap export/auth-rank transfer or selected key/attachment replacement before seal invalidates v1 instead of expanding scope. Rank 0 is the only rank in this interface.

Install a **CumulativeInodeAck** guard atomically at Bound, before executing the selected input. The watched key/inode's accepted-ACK high-water mark must be `< T`; otherwise the candidate is already covered and Invalid. The final `Session::transmit` seam examines every actual FLUSH_ACK for that destination, including raw/queued/regenerated replies, before `ClientLink::send_raw`. Suppress every `covers` message at its FIFO position until process death. Count exact-T and greater-T suppressions separately; retain first/last tid and count for the latter in the final Coverage record. This is a bounded extension of one lost-ACK cut, not loss of all CAPS or all session output. N3's other workload uses a **distinct inode**.

Hit still requires the selected T's own checked `JournalRecord.client_flush`/CommitRef and its own ACK at the funnel. A covering ACK alone cannot create Hit. Allow native Committed, journal_event, record-slot release and other effects normally. Suppression never increments push_seq or masquerades as `send_raw=true`. A false transport result retains normal fail-stop semantics.

At **actual successful transport acceptance**, update max_ack_accepted for every watched key/inode. If `covers` is true, increment cover_accepted/selected_accepted as applicable, latch **Invalid(CoverAccepted)** synchronously, and stop admission/output. Emit the invalidation if possible. Peer/capture evidence of the same condition also invalidates regardless of whether the client happened to ignore that packet. A cap_id mismatch is not an excuse to discard it.

An event queued just before SIGKILL could be lost even with no visible sequence gap. Therefore neither an empty ACK log nor CutConfirmed proves the cut lasted until death. **Only the §4.6 final seal is conclusive**: ExitReady requires cover_accepted=selected_accepted=0, guard still installed, complete required prefix, no pending action that can bypass the guard, and frozen data-plane progress. QA fsyncs that last event before SIGKILL. A test implementation must reject a forced acceptance on any bypass path before it can seal. This closes the invalidation-tail race without an ACK round trip for every ordinary output.

After MDS-only failure, keep the original kernel/FUSE/Ganesha process alive. The StartupPreResend snapshot must contain T's flush completion. Observe the real same-key/T resend, including an early resend if sent by that peer. Record FlushHistoryHit before metadata admission, then normal accepted ACK with its actual tid/dirty/barrier and independent peer receipt. A covering ACK **after recovery** may complete T; record its actual tid rather than renaming it T. Require an actual T resend/history hit for the dedupe case. No second record carrying that ClientFlush may appear. Check durable scalar/range state and every advertised writer bound/OSD barrier. Do not discharge D5's output drain with a suppressed output.

### 5.4 Fixed observation variants

Use the common event envelope from §4.1. Owner and generation appear in the relevant typed body or its referenced input/output/StatePoint. All references must resolve within the complete transcript. Required kinds are:

```text
Input = InputRequest | InputFlush
Journal = { stage:Encoded|WorkerEnqueued|AppendStarted|StoreConfirmed|EngineCommitted,
            commit:CommitRef, input:?U64, tag:?WalTag }
Decision = { input:U64,
  outcome:RequestHistoryHit|FlushHistoryHit|NewMutation|NoRecord|Busy,
  history_probe:?U8, original_proof:?H256, commit:?CommitRef }
Output = { tag:OutputTag, stage:Queued|Suppressed|TransportAccepted|TransportFailed,
           summary:Summary, push_before:U64, push_after:U64 }
Summary = Reply { tid:U64, op:U32, result:I32, safe:Bool,
  target:?U64, dentry_parent:?U64, dentry_hash:?H256, created_ino:?U64,
  trace:Bool, snap_trace:Bool, cap_id:U64, caps:U32, max_size:U64 }
 | Caps { tid:U64, op:U32, ino:U64, cap_id:U64, seq:U32, issue_seq:U32,
  mseq:U32, dirty:U32, flags:U32, size:U64, max_size:U64,
  truncate_seq:U32, osd_barrier:U32 }
 | Open { seq:U64, final_chunk:?U64 }
 | Reclaim { result:I32, epoch:U32, addresses:AddrSetRef, reclaim:U64 }
Coverage = { arm:U64, key:Key, ino:U64, selected_tid:U64,
  exact_suppressed:U64, greater_suppressed:U64, first_greater:?U64,
  last_greater:?U64, cover_accepted:U64, max_ack_accepted:U64 }
Phase = { owner:?Owner, map_epoch:U32, state:I32, confirmed:Bool }
Reconnect = { owner:Owner, key:Key, attachment:Attachment,
  stage:FinalReceived|FinalValidated|AttachmentDurable, input:U64, received_ns:U64 }
Reclaim = { reclaim:U64, input:U64, requester:Key, attachment:Attachment, uuid:H256,
  flags:U32, old_target:?Key,
  stage:Received|Selected|Absent|FenceStarted|FenceVerified|ClosingDurable|
        CleanupCommitted|ClosedDurable|NoReplyExpected,
  addresses:?AddrSetRef, target_epoch:?U32, applied_epoch:?U32,
  operation:?U64, cleanup_seq:?U64 }
SessionIdentity = { input:U64, key:Key, attachment:Attachment, op:U32,
  auth:H256, uuid:?H256, timeout:Timeout, addresses:AddrSetRef }
Lifecycle = { key:Key, reason:ReclaimReset|ReconnectMissing|ActiveExpiry|ClientClose,
  stage:DeadlineExpired|FenceStarted|FenceVerified|ClosingDurable|
        CleanupCommitted|ClosedDurable,
  reclaim:?U64, addresses:?AddrSetRef, deadline_ns:?U64,
  target_epoch:?U32, applied_epoch:?U32, operation:?U64, cleanup_seq:?U64 }
RecoveryGate = { owner:Owner, attempt:U64,
  stage:Reconstruction|ControlDrain|MarkerExecuted|OutputDrained|Serving,
  received_cut:?U64, committed_through:U64 }
Job = { job:U64, ino:U64, kind:Purge|Truncate|RecoverSize,
  stage:Started|Result|CompletionCommitted, guard_version:?U64,
  truncate_seq:?U32, max_bound:?U64, outcome:?(Done|Recovered|Stale|Failed),
  recovered_size:?U64, completion:?CommitRef }
```

Decision uses a canonical exact history lookup at the actual admitted step, not only at network receive. Keep the input context across Busy/continuations. Emit `NoRecord` only when that input completed without a record, not on an intermediate step. Journal emits a separate event at each real stage. Encoded is emitted after Runtime assigns the command operation, before its first handoff; it proves bytes already exist, not the precise encoding-end timestamp. Worker start/confirmation facts use one ordered completion lane to the owning task. Worker enqueue/start is not durability. A checked store completion precedes EngineCommitted and output construction. An unrelated record may have tag=null; still emit its selected input/commit link and watched inode/range changes.

Use `DurableInode={commit:CommitRef,value:Inode|WriterRange}` with the Item variant fields for committed watched images, including exactly the declared range count. Keep only the scoped scalar/range summaries from the record. Reuse the bounded export cursor for a large image expansion; do not enqueue up to 144 image/range items at once. Materialize the commit's effects and return the record permit normally. Pause later admission while that export drains, without blocking the reactor or claiming that an output was accepted. Emit the first selected Suppressed Output before Hit; later exact/covering drops use Counters/Coverage rather than an unbounded stream of duplicate suppression records. Accepted scoped outputs still get their actual individual summaries.

Control response bodies are `HelloAccepted={owner:null}`, `ProbeInstalled={slot:U8,owner:Owner}`, `BindReleased={slot:U8,owner:Owner}`, `SnapshotReleased={snapshot:U64,owner:Owner}` and `Cancelled={arm:U64,owner:Owner}`. AttachmentBound uses SessionBound's schema. Status response is `{state,owner:?Owner,arm:?EventRef,hit:?EventRef,positions:?Positions,counters:?Counters,deadline_ns:?U64}`; state is one of the states named in §4 plus ControllerReady, Loading, SnapshotHeld, Discovering, Stopping or Failed. It carries references to Armed/Hit events, not copies. Ready's bounds are fixed v1 constants. A streaming SnapshotRequest's cached response is SnapshotBegin; its Items/End are asynchronous events. ResolveSlot caches ProbeInstalled and emits SessionBound asynchronously. A pending ExpectExit creates one seal; its eventual ExitReady is the response. Failure is terminal, not a recoverable command error.

SessionIdentity records authenticated OPEN/CLOSE metadata and exact peer address-set references; lifecycle and output receipts separately prove their effects. Lifecycle covers missing-member cleanup even when **no reclaim requester exists**. Require the complete address set and target/applied map epochs for a FenceVerified event, operation for durable rank transitions, and cleanup_seq for CleanupCommitted/ClosedDurable. Reclaim uses a local counter as its ID because the wire message has no application tid. A requester generation change blocks success/output under D6/R1. RecoveryGate observes D5's real boundaries; received_cut is its actual ordered input-cut counter, not an invented commit sequence. Repeated observations never execute a gate.

Job reports only already-existing daemon work for watched inodes. Copy the actual job guard/bound when that job variant has one; otherwise use null. Started has outcome/completion=null. A stale result is not a committed completion. CompletionCommitted requires its checked CommitRef; Recovered requires its actual size. This is the B3 dependency proof, not a new job scheduler or engine callback.

Persisted receipts name pinned hashes: SnapshotAck exactly SnapshotEnd; BindAck exactly SessionBound; AckHit exactly Hit; ExitAck exactly ProcessExiting. ExpectExit names the immediately preceding **Status response**, which QA obtains and fsyncs after collecting external prerequisites. Keep those bounded milestone hashes plus the one cached response hash; do not retain an unbounded per-event hash table. Coverage is emitted before Status used for exit and again before ExitReady if its counters changed. It is not a substitute for the final seal counters.

Pair Reconnect FinalReceived/FinalValidated/AttachmentDurable with **ReconnectOpen Output/TransportAccepted** and that final-chunk reference. OPEN.seq reflects preceding accepted counted pushes. Ordinary/duplicate OPEN and application first success have separate observations. Queue acceptance still does not prove TCP delivery or peer processing.

Store IDs, scalar metadata and fingerprints only. Hash UUID/auth names with the same raw salt-prefix SHA256 rule; never hash/display authorization-context secrets as identity. Do not emit file data, xattr values, symlink content, keys/tickets or raw message bodies. The controller's independent data oracle stores digests and expected data separately. Fixed address bytes/nonces are retained in private evidence for fencing correlation.

## 6. Independent peer and application evidence

### 6.1 Installed providers and exact proof levels (C5)

Read-only package metadata on 2026-10-25 reports kernel image package `7.0.0-1012.12`, ceph-fuse/libcephfs `20.2.0-0ubuntu2.1`, Ganesha/FSAL_CEPH `6.5-8`, and tshark `4.6.4-1`. The installed kernel configuration enables DYNAMIC_DEBUG, TRACEPOINTS, EVENT_TRACING, FTRACE_SYSCALLS, BTF, KPROBE_EVENTS and UPROBE_EVENTS. The installed FSAL imports `ceph_start_reclaim`, `ceph_finish_reclaim`, `ceph_set_uuid`, `ceph_set_session_timeout`, `ceph_ll_setlk`, `ceph_ll_sync_inode` and `ceph_ll_fsync`. These are **static prerequisites**, not evidence that an observer ran or a case is reachable.

Use the following fixed provider combination. Passive observation leaves each kernel/FUSE/libcephfs/Ganesha peer unmodified. QA must prove every required field during separately authorized preparation before arming a fault.

| Provider | Required records and correlation | Limit |
|---|---|---|
| `WireCeph1` — scoped live msgr1 metadata decoder on owned MDS/client TCP streams | Connection tuple/advertised address+nonce, message source entity, header seq/tid/type/version; request op/retry/head flags/open flags/both binary filepath summaries; reply safe/result/trace identity; CAPS op/ino/tid/cap-id/seq/issue-seq/mseq/dirty/flags/barrier/size/max-size/truncate-seq; SESSION OPEN/seq/UUID/timeout, reconnect final flag, RECLAIM flags/UUID and reply result/epoch/address entries. | Network observation is not peer processing. Stock tshark alone is **not the complete Ceph decoder**. Its field registry exposes `ceph.seq`, `ceph.tid`, reply safe/op/result and basic CAPS fields, but does not establish modern request flags, barrier, reconnect or reclaim decoding. |
| `KernelPeer7` — tracefs Ceph events plus targeted dynamic-debug sites | `ceph:ceph_mdsc_send_request`: tid/op/mds; `ceph:ceph_mdsc_complete_request`: tid/op/err; `ceph:ceph_handle_caps`: mds/op/ino/snap/seq/mseq/issue_seq. Dynamic-debug sites in `mds_client.c` (`__send_request`, `handle_reply`) and `caps.c` (cap-send path, `handle_cap_flush_ack`) add the `[FSID global_client_id]` doutc prefix, request attempt and cap cleanup facts. | The cap tracepoint lacks flush tid and client ID. Join it to the exact owned client's decoded stream and debug prefix. A unique matching received ACK and subsequent cleanup event prove processing; a bare tracepoint or TCP ACK does not. Ambiguous joins fail the case. |
| `Libcephfs20` — `debug_client=20/20` on the owned ceph-fuse or Ganesha libcephfs instance, with a whitelist reader | `send_request` logs rebuilding tid/rank and the request; `handle_client_reply` logs Safe/tid and caller signaling; `send_cap`/`kick_flushing_caps` identify inode/rank/seq/dirty; `handle_cap_flush_ack` logs inode/rank/cleaned bits and wakeup; `handle_client_reclaim_reply` prints result/epoch/addresses. Join missing fields to WireCeph1 by PID/start, stream, inode and message order. | `send_cap`'s standard log is not a complete tid/barrier record. Logs and wire records together must establish the join. Discarded/sessionless/stale replies do not count as processed. Do not enable auth/key logging or assume absence of an error line proves success. |
| `CallTrace` — tracefs syscall entry/exit for the exact workload PID/TID, plus durable worker receipts | Trace the actual creator/rename/unlink/open/fsync/fdatasync/fcntl syscall selected by the worker. Record its call ID, thread, FD/inode anchor, entry, return/errno and monotonic time. | An intent or silent stdout does not prove a blocked syscall. One entry with no intervening exit and a later matching exit, across a complete loss-free trace interval, proves it remained pending at the crash. An automatic userspace EINTR retry is not allowed. |
| `WireNfs41` plus `Ganesha65Log` | §6.2 defines exact NFS and grace/FSAL records. These add NFS client/session/owner/file-handle evidence to Libcephfs20. | Two NFS mounts or successful application I/O cannot substitute for these records. |

**WireCeph1 implementation boundary:** the later QA decoder is a passive, bounded parser, independent of the daemon observation tags. Read the C++ msgr1 header/front formats and versioned `MClientRequest`, `MClientReply`, `MClientCaps`, `MClientSession`, `MClientReconnect`, `MClientReclaim` and `MClientReclaimReply` layouts. Preserve exact path bytes only long enough to hash them. Decode signed reply errors as I32, even where tshark exposes an unsigned field. Validate message lengths, versions, CRCs and per-direction messenger sequence; record TCP retransmissions without treating them as another Ceph send. A same-tid Ceph resend is a **new message instance** on the verified peer stream, not a retransmitted TCP segment.

Scope capture to the assigned rank listener/owned peers and NFS 12049. Decode live in memory; retain only the whitelisted summaries. Do not archive raw packets, RPC credentials, cephx authorizers or file/xattr data. Cap active streams at eight and reassembly at 4 MiB per direction/stream; fail on overflow or an unsupported front length/version. Stream-skip unneeded data after bounded framing validation. Ended stream identities remain in the transcript across reconnects. The parser does not need secrets or signature verification: the authenticated daemon binding and independent process/socket evidence supply the identity join. If the installed dissector cannot supply a field, the version-pinned metadata parser must supply it **before** the provider is declared ready. A guessed/defaulted field is failure. This parser is specified here, not implemented or verified here.

The normalized peer envelope is `{provider,record:U64,boot:H128,observed_ns:U64,clock_error_ns:U64,peer_proof:H256,stream:?U64,source_offset:U64,stage:WireSend|WireReceive|PeerProcessed|LocalEvent,body}`. Provider names are the finite names above; non-wire providers use stream=null and their local/processing stage. WireCeph1 bodies use the exact scalar fields in the table, with request/flush tid domains distinct; raw key incarnation and local attachment are **not** on the wire. QA adds a separate join `{peer_record,daemon_event:EventRef,key:Key,attachment:Attachment}` after checking identity. Peer records never copy daemon values to fill a missing independent field. This envelope describes external evidence artifacts; it is not another daemon control message.

Worker receipts contain `{case_id,call_id:U64,client_slot:U8,pid:U32,tid:U32,start_ticks:U64,mount_proof:H256,action,names:[LocalName;0..2],stage:Intent|Return,mono_ns:U64,result:?I64,errno:?U32,ino:?U64,size:?U64,data_digest:?H256}`. `LocalName={base_proof:H256,len:U32,hash:H256}` anchors the actual syscall argument bytes to its saved parent/FD proof. It is **not** a guessed wire Path; the independent decoder establishes that separate representation. case_id is one individual §7 case name; action is Mkdir|Mknod|Symlink|Open|Rename|Replace|Unlink|Write|Fsync|Fdatasync|Lock|Unlock. One worker has one pending call. Fsync Intent before entering it; fsync Return before notifying the controller. Preserve the original call/FD across failure. Use a raw libc call through the worker to avoid Python's automatic EINTR retry; it still uses the unmodified mounted client. Track syscall entry/exit independently. Worker call_id is never a Ceph tid. Capture complete receipts through return or identity-checked process exit, then fsync the final artifact. A crash/cancellation that prevents the selected real return fails the application predicate.

**Loss and timing rules:** each provider records a start barrier, continuous record/byte offsets, and a final drained/fsynced barrier. Keep capture drop/truncation counters, unresolved TCP sequence holes, parser errors, tracefs per-CPU lost/overrun counters, `/dev/kmsg` sequence gaps if used, and file inode/offset/rotation continuity for logs. Any nonzero required-stream loss or unknown continuity fails the admitted case. A locally consecutive adapter counter cannot repair missing upstream records. Stop neither observer nor worker reader at MDS EOF. Keep them through recovery outcomes and normal closure. Correlate wall-clock capture/log times to same-boot CLOCK_MONOTONIC before and after the interval; an ordering inside the uncertainty interval is unproved. No quiet interval is an acknowledgment.

For A/B1/B2, pass requires all three independent levels: **daemon transport acceptance**, **real peer send/resend/processing**, and **real application return**. Before SIGKILL require the selected call still pending and no exact/covering peer response. The complete final peer/call trace must confirm this retrospectively; the fsynced ExitReady proves the daemon's sealed no-send condition. After restart require the actual same-key/tid resend, its dedupe/new-commit decision, accepted response, peer processing and original syscall outcome. A server snapshot plus eventual fsync is insufficient.

### 6.2 Ganesha 6.5 source facts and NFS identity mapping (C8)

Use only the owned foreground helper on **127.0.0.1:12049**, export **100**, with the retained `fs` recovery directory, node ID, server owner/scope, and grace configuration. The system service/port **2049** is outside this scope. Preserve helper executable/library hashes, PID/start ticks, config hash and recovery-backend identity before each run.

Upstream **V6.5**, `FSAL/FSAL_CEPH/main.c::reclaim_reset`, sets session timeout **300 s** and builds `ganesha-<nodeid>-<export_id as four lowercase hex digits>` (export 100 suffix `0064`). `COMPONENT_FSAL` DEBUG logs **“Issuing reclaim reset for %s”**. EVENT logs **“start_reclaim failed: %s”** on a nonzero libcephfs result. Crucially, this V6.5 function then calls finish_reclaim and set_uuid and returns 0 **even after that error**. Export readiness alone therefore proves neither RESET success nor the expected ENOENT outcome. The installed imports confirm the feature is linked; they do not prove execution or equivalence to upstream without package patches.

Ceph v20.2.0 `Client::start_reclaim` opens a temporary session, checks RECLAIM_CLIENT, sends RESET, and consumes its actual reply. Nonempty target addresses with nonnegative result give RESET success. Empty addresses yield **-ENOENT**. RESET does not set `reclaiming_uuid`; its `finish_reclaim()` sends **no Ceph FINISH packet**. Record the temporary CLOSE and subsequent UUID-bearing OPEN if they occur. NFS **RECLAIM_COMPLETE** is a different protocol operation and is still required below.

`Ganesha65Log` uses STATE at EVENT plus FSAL/NFS_V4/NFS_V4_LOCK/CLIENTID at DEBUG/FULL_DEBUG as required for the scoped fields. Known V6.5 STATE messages are **“NFS Server Now IN GRACE, duration %d”**, **“grace reload client info completed from backend”**, **“NFS Server Now NOT IN GRACE”**, and **“check grace:reclaim complete(%d) clid count(%d)”**. Retain helper PID/thread, source component/function, complete interval and parsed values. “Graceless is true” or missing grace entry fails grace qualification. The configured 90 s interval is not an observed grace duration. Do not assume optional Ganesha LTTng tracepoints were compiled into this package.

`WireNfs41` requires the actual negotiated minorversion **1** and records each RPC call/reply and each COMPOUND operation index. Required fields are TCP peer/stream, xid, opcode/status, NFS clientid, sessionid, SEQUENCE slot/sequence, OPEN claim type/open-owner, LOCK reclaim flag/type/range/lock-owner, stateid, current/returned file handle, and RECLAIM_COMPLETE's one_fs flag. Preserve the PUTFH/GETFH and current-file-handle state of each compound; do not flatten repeated fields into unrelated arrays. Hash full opaque client/session/owner/stateid/handle bytes with the run salt. Do not use tshark's short CRC handle/stateid hashes as identities.

Installed tshark's registry supplies `nfs.opcode`, `nfs.status`, `nfs.clientid`, `nfs.session_id4`, `nfs.slotid`, `nfs.seqid`, `nfs.open.claim_type`, `nfs.open_owner4`, `nfs.lock_owner4`, `nfs.lock.reclaim`, `nfs.stateid`, `nfs.fhandle` and `nfs.reclaim_one_fs4`. Bind these to the nested decoded RPC operation and exact bytes. Field registration is not proof of complete runtime dissection. RPCSEC_GSS privacy or an undecodable compound is a missing provider, not permission to change credentials. v4.0 is outside this v1 NFS trial profile.

**File handle to inode:** V6.5's `GANESHA_FH_VERSION` is **0x43**, despite a stale struct comment mentioning 0x41. Its ordinary packed NFSv4 handle has a five-byte header: version:u8, flags:u8, export_id:u16 big-endian, fs_len:u8. For FSAL_CEPH, fs_len=24 and the following bytes are **ino:u64le, snap:u64le, fscid:i64le**. Total length is 29 bytes. Hash the entire handle and retain these decoded scalars. Require the ordinary non-DS handle, export100, fscid44 and the observed head snapshot value. Reject an unsupported shape rather than guessing offsets. Correlate this inode with a native request/reply or CAPS message on the owned Ganesha stream and with the daemon's exact key/attachment. NFS `st_ino` alone cannot make this join.

Two independently identified NFS clients must have distinct on-wire client identities, with each session mapped by EXCHANGE_ID/CREATE_SESSION/SEQUENCE records and each open/lock owner mapped to that client's state. They may share the **same** Ganesha PID/libcephfs instance and MDS key. Prove that join separately for each workload: worker/mount → NFS RPC+file handle → decoded Ceph inode/path+Ganesha stream → daemon key. Distinct mounts/workers, or a timing coincidence without the inode/RPC join, are insufficient. At MDS restart the Ganesha process and Ceph client entity must survive, but the TCP connection and daemon attachment may change normally.

### 6.3 Exact N1–N4 predicates and missing prerequisites

| Case | Required schedule and predicates |
|---|---|
| **N1 — ganesha-reset** | Keep MDS alive. Prove old UUID/key Open, timeout300, nonempty full address union, acknowledged data and NFS OPEN/LOCK state. Install successor Reset and UuidOpen discovery, then kill/restart only the owned helper. Require a newly identified requester; exact old-target selection; verified full-union fencing/map barrier; durable Closing → cleanup receipt → Closed; accepted result0 with old addresses/post-fence epoch; peer processing and UUID OPEN. The FSAL continuation must follow that reply. An already expired/Closed target is **not N1**. |
| **N2 — ganesha-absent-reset** | Preserve old NFS OPEN/LOCK/data evidence. For the joint crash, use an ordinary Ganesha dirty operation to reach the existing B2 cut and final seal; kill MDS, then the owned old helper before restarting MDS. This supplies the existing crash handshake, not a same-tid B2 recovery claim. Keep NFS clients mounted. Restart MDS with old exact probes while Ganesha stays absent. Fsync the pre-resend image proving old persisted UUID/key. Require the actual missing-key deadline and fence/Closing/cleanup/Closed receipts, then D5/monitor recovery progress with no extra Ganesha hold. Only then launch the successor. Require RESET Absent, wire result0/epoch0/empty addresses, libcephfs ENOENT (FSAL error log plus decoded reply), continuation and UUID OPEN. A missing startup UUID or a different error cannot be relabeled this case. |
| **N3 — ganesha-shared-flush** | Run B1 and B2 separately. Keep the same Ganesha PID/start/libcephfs instance/client entity and durable key across **MDS-only** failure. Map two distinct NFS clients and distinct inodes to that same key as above. Prove selected dirty flush pending and the second real operation already outstanding **before Sealing and still at SIGKILL**, using continuous call/RPC evidence. Other-inode traffic remains permitted. If it finishes before the crash, the two-outstanding predicate was not established. Starting it only after the final freeze does not qualify. Require the selected same native tid resend/history decision and each original NFS/syscall outcome. A helper restart/RESET cannot prove N3. |
| **N4 — ganesha-lock-gap** | Use N2's missing-session cleanup. Before launching successor Ganesha, a separate native client requests a nonblocking write lock on the exact old NFS-locked inode/range. Record native GET/SETFILELOCK input/result and syscall outcome. To establish the accepted gap, acquisition must succeed and remain held through the successor's in-grace reclaim LOCK on that same range. Require reclaimed OPEN success, LOCK(reclaim=true) **NFS4ERR_DENIED**, and the native holder still excluding an independent overlapping request. Release only after recording the conflict. Native acquisition denied means the gap was unexercised; it is not a gap pass. |

V6.5's conflict chain is `ceph_ll_setlk` EAGAIN/EACCES → FSAL_DELAY/ACCESS → STATE_LOCK_CONFLICT → **NFS4ERR_DENIED**. Attribute that result to the exact overlapping native lock, not an unrelated owner or permission failure. Use nonblocking locks. **NFS4ERR_NO_GRACE** means reclaim timing/eligibility failed; it is not an acceptable substitute for this lock conflict. DELAY, BAD_STATEID, STALE_CLIENTID, unexplained ACCESS/IO and arbitrary errors do not satisfy N4. Preserve the original blocked/lost-lock application outcome; do not silently reopen or relock it and call that transparent recovery.

N1/N2 also need **NFS-vs-NFS grace evidence**: retained old handles and acknowledged bytes; old owner's OPEN(CLAIM_PREVIOUS) success during observed grace; reclaim LOCK success for the non-conflicting case; NFSv4.1 RECLAIM_COMPLETE(one_fs=false) with NFS4_OK; a separately identified new non-reclaim owner getting NFS4ERR_GRACE during that interval; observed grace exit; then overlapping lock denial under the reclaimed old lock. A retransmitted RECLAIM_COMPLETE may return COMPLETE_ALREADY only with the earlier exact NFS4_OK receipt. N4 deliberately replaces the conflicting old LOCK-success predicate with the proved native conflict; it must still demonstrate ordinary NFS grace protection on a separate inode. Compare retained data/handles before any healing write.

Fixed prerequisite names are `WireCephFields`, `KernelIdentityJoin`, `PeerProcessingJoin`, `CallTraceContinuity`, `Nfs41IdentityJoin`, `FhInodeJoin`, `GaneshaResetLog`, `GraceWindow`, `ObserverAccess` and `ObserverCapacity`. Missing decoder/permissions/installed log fields during preparation gives **2** for the named case; an explicitly preparation-only report may give **77/PARTIAL** and list them. Loss, ambiguity, missing required evidence, an unreachable selected sequence or a wrong outcome **after admission gives 1**, never a downgraded successful partial. No current runtime provider is claimed ready. Remaining unknowns are actual access, package-emitted fields/patch behavior, loss-free operation and reachable real-peer schedules. The API and success predicates are fixed; those probes cannot relax them.

### 6.4 Version-pinned references

- [Ceph v20.2.0 Client.cc](https://raw.githubusercontent.com/ceph/ceph/v20.2.0/src/client/Client.cc): `send_request` 2696–2745; `handle_client_reply` 2880 onward; `send_cap` 3923–4049; `handle_caps` 5508–5596; `handle_cap_flush_ack` 5707–5771; `start_reclaim`/`finish_reclaim`/`handle_client_reclaim_reply` 17333–17476. Wire definitions are the same tag's `src/messages/MClient*.h`, `src/include/ceph_fs.h`, `src/include/msgr.h` and `src/include/filepath.h`.
- [Linux v7.0 caps.c](https://raw.githubusercontent.com/torvalds/linux/v7.0/fs/ceph/caps.c), `handle_cap_flush_ack` 3811–3922 and `ceph_handle_caps` 4328 onward; [trace events](https://raw.githubusercontent.com/torvalds/linux/v7.0/include/trace/events/ceph.h). Local `/mnt/workspace/ceph-client` is 7.3.0-rc6; it corroborates behavior, not exact installed symbol offsets. Kernel sources remain behavior references only.
- [Ganesha V6.5 reclaim_reset](https://raw.githubusercontent.com/nfs-ganesha/nfs-ganesha/V6.5/src/FSAL/FSAL_CEPH/main.c), [grace](https://raw.githubusercontent.com/nfs-ganesha/nfs-ganesha/V6.5/src/SAL/nfs4_recovery.c), [LOCK](https://raw.githubusercontent.com/nfs-ganesha/nfs-ganesha/V6.5/src/Protocols/NFS/nfs4_op_lock.c), [RECLAIM_COMPLETE](https://raw.githubusercontent.com/nfs-ganesha/nfs-ganesha/V6.5/src/Protocols/NFS/nfs4_op_reclaim_complete.c). Error conversion: same tag's `FSAL/fsal_convert.c` and `SAL/state_misc.c`.
- V6.5 handle sources: [common header](https://raw.githubusercontent.com/nfs-ganesha/nfs-ganesha/V6.5/src/include/nfs_fh.h), `support/nfs_filehandle_mgmt.c`, and `FSAL/FSAL_CEPH/{internal.h,handle.c,export.c}`. V15.7 map facts do not establish installed 6.5 behavior. Upstream V6.5 was read because local Ganesha sources were absent; the package patch series was not obtained. No package/source equivalence or runtime observation is asserted.

## 7. Case IDs and meaningful verification

| ID | Real-peer case |
|---|---|
| `G3-A0` | Lost SAFE new creator, unchanged identity; same-wire-tid retry. Kernel and FUSE/libcephfs. |
| `G3-A1/A2/A3` | Same cut, then other-client rename / replacement / unlink, each acknowledged before crash. Include O_EXCL, directory and symlink creators where reachable. |
| `G3-A4` | Mutating existing-inode CREATE/OPEN: O_TRUNC (also zero old size), range-only, combined mutation; recorded target but no created inode. Depends on accepted E1. |
| `G3-B1` | Dirty flush encoded but never handed to worker; no ACK; checked recovery lacks its completion before real resend. |
| `G3-B2` | Dirty flush durable; exact and covering FLUSH_ACKs suppressed through the final seal; checked recovery has dedupe; same-tid resend does not mutate twice. |
| `G3-B3` | Early resend/control drain plus competing rights and retained job progress; commit before grant release. Use accepted E2. |
| `G3-R0` | Final reconnect OPEN transport timestamp, actual phase sequence, and separately application first success for kernel/FUSE/Ganesha. |
| `G3-N1/N2/N3/N4` | Ganesha-only RESET / joint absent RESET / shared-session MDS-only flush / accepted native-lock-gap observations above. |

Run B cases on kernel and FUSE/libcephfs, and N3 with real Ganesha. The protocol-loss cases use unmodified peers and normal operations; no injected REPLAY, fabricated request, synthetic transport success or regenerated application tid substitutes for the selected retry. Random SIGKILL history tests remain useful broader coverage, not evidence for these selected windows.

**Reuse:** `journal.rs::tests::reservation_recovered_sequence_and_completion_are_not_submission`; production `server/runtime/tests.rs::{Fixture,Peer}` with explicit commands/completions; `persistence.rs::tests::{real_worker_delays_open_reply_and_cleanup_close_until_durable_completions,failed_real_append_discards_reply_and_stops_engine_service,commit_completion_wakes_queued_work_without_new_client_input}`; session funnel acceptance/sequence doubles; fencing backend doubles. They provide the seams. Existing delayed callbacks and test `Peer::send_raw` do not provide native deterministic controller cuts, native commit evidence, or true reconnect OPEN timing. QA source was not inspected.

**Required later offline verification. These checks were not run in this assignment.** Use the production DaemonJournal path and meaningful transport/storage/controller doubles:

1. Exercise both pump handoffs, full worker channel and mismatched owner/operation/sequence. B1 retains exactly one unsubmitted command/permit, q=d+1, no ACK, no checkpoint and no runnable spin. TERM, EOF, timeout and teardown must never flush it. A started/unknown append cannot produce B1 Hit.
2. Verify callback → record-slot release → selected output suppression. Let later cap revokes and the second client's committed rename finish. Suppress regenerated selected replies; pass other identities. Verify output failure remains real fail-stop and no suppression completes D5's output drain.
3. Cover B2 exact-T and greater-T ACKs with different cap IDs, sequences and dirty masks, plus another inode/key and FLUSHSNAP_ACK. Install the guard at Bound, check the preexisting high-water mark, and cover every queued/raw/final transport path. A forced accepted covering ACK latches Invalid **before** another turn or successful seal. Test B1's corresponding acceptance invalidation.
4. Exercise a queued-but-unpersisted invalidation and a disconnect immediately after Hit/CutConfirmed. Neither prefix permits a crash PASS. Test competitor completion before Sealing, outstanding operations during drain, immutable CrashSealed, final Coverage/ExitReady hashes, no later send/handoff, and kill/wait-status correlation. Reject an early signal, seal deadline, wrong pidfd or missing final frame.
5. Verify unsolicited Ready, Hello identity/bounds, checked restore, TrimInitial-only handoff and actual receipt. Queue OPEN/reconnect/dirty retry/Close/expiry/checkpoint work before load and during export. None may alter the image or produce client output before valid **fsynced SnapshotAck**. Queuing/sending End alone must leave admission closed. Wrong snapshot/end/hash/owner and partial fsync must not release it.
6. Verify streamed 696-item maximum snapshots and 64-address live unions under backpressure with only 60 ordinary event slots. Validate Present/Absent/Closed/unknown meanings, exact histories, checkpoint-only histories, all writer ranges, immutable Begin/End points/counts and contiguous ordinals. Reject extra owners, truncation and a post-admission snapshot labeled StartupPreResend.
7. Verify discovery in eight reservations with many unselected Closed admin rows. Hold exact MKDIR/RESET until valid BindAck; reject ambiguity/stale generation. Cover preinstalled successor RESET, separately preinstalled UUID OPEN, duplicate-key slot aliases and old-target preservation. Resolve unknown creator inode only from its completion into the reserved probe. Cover the shared 32-history limit without silent eviction.
8. Check binary/empty/non-UTF8 paths, path2 roles, raw-salt hash bytes, base/length mismatch, head versus CEPH_O_* masks, real dirty UPDATE/FLUSH and flags. No-record/negative candidates are terminal NotApplicable. A worker call ID, wrong opcode/incarnation or later application tid must never match a selected wire identity.
9. Encode every legal maximum-width command/event/item including envelope. Verify the stated <16 KiB bounds and strict integer tokens, fixed hashes, unknown/duplicate fields, invalid UTF-8, nesting, partial frames and overflow-before-allocation. Verify incremental exports, monotonic IDs, identical cached response replay and conflicting-ID failure. Pin only bounded milestone hashes; exercise each fsynced-prefix acknowledgment and interleaved asynchronous event.
10. Verify NormalStop independently: no bound arm/hold, observed close/unmount/drain, final ProcessExiting, fsynced ExitAck and exit0. Missing acknowledgment, live-process EOF, abnormal status or cleanup failure cannot pass. A recovered observer may stop normally; the injected process cannot resume.
11. Follow real D6 lifecycle receipts and requester revalidation, including delayed queued reply, changed address union, stale Closed, absent RESET and FINISH-only NoReplyExpected. Test missing-member cleanup with no reclaim requester and D5 gate/job observations. Never turn command submission into durable cleanup or queueing into transport acceptance.
12. Validate independent decoder/correlation fixtures: TCP retransmission versus Ceph resend; covering ACKs; log/trace losses and ambiguous joins; original syscall EINTR without userspace retry; NFS compound/owner/FH mapping; Ganesha 6.5 ENOENT continuation; exact N4 DENIED versus NO_GRACE. Use credential-free version-pinned fixtures. Modeled peers validate the controller only; they cannot qualify a live-peer case.
13. Verify default-feature-off behavior and normal wire bytes. Cover malformed control, authority loss, queue/counter overflow and every fail-stop seam. Preserve the single owner, bounded input/output accounting, one-record permit and session funnel. Reuse accepted E1/E2 semantic checks; do not claim this observer replaces their implementation review.

Later native evidence must bind reviewed source/binary/config/provider hashes, run/peer identities, FSID `3ceca2c6-851f-41cf-b538-1cfded4f1e82`, filesystem `a`/44, metadata88/data89, and both retained `restart-history-1629333-1792700434474727536` and `restart-history-2663893-1792950162527640133` histories. Retain Ready/Hello/SnapshotAck, BindAck, Armed/Bound/Hit/AckHit/CutConfirmed, the final Status/ExitReady prefix, signal/wait receipts, restart snapshot and genuine peer outcomes. A consecutive transcript without its required terminal record is incomplete.

Use new per-trial namespaces/artifacts on the same filesystem. Preserve OSD processes, the reviewed G2 binary, and all original/retry evidence. Compare cumulative baselines and independent cold views before healing writes. The parent must assign implementation/review and then sole-QA live execution separately. Consumer re-review remains pending; no build, test, upgrade, live operation, Git write or G3 qualification occurred in this revision.
