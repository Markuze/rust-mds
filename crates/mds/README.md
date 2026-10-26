# Durable root-shard daemon: G2 and collection-only G3

This package uses the native Rust Objecter and `store` compound operations.
Production uses `Engine<DaemonJournal>`, not `MemJournal`.
The G2 packages and same-filesystem clean-restart application evidence are
independently accepted. Full storage-fault qualification remains open.
The new D1-D4 recovery package needs independent review. It cannot activate
nonempty recovery. QA alone owns the cluster.

## Build and run

From the repository root:

```sh
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo build -p rmds
RMDS_LOG=info /mnt/workspace/rust-mds/target-sol-daemon/debug/ceph-rmds \
  -i rust -c "$CEPH_CONF" --keyring "$RMDS_KEYRING" --mds-join-fs a
```

Use the existing cluster configuration and the `mds.rust` keyring. Set
`CEPH_CONF` and `RMDS_KEYRING` to their actual paths. No manual filesystem id
is needed. The daemon discovers it through the native FSMap subscription.
The assigned rank must be 0. Use a finite, reachable advertised messenger
address. Each process uses a new messenger nonce.

Only monitor-confirmed `up:creating` can bootstrap a compatible empty metadata
pool. The daemon enumerates the pool before writing the format plan.
`up:starting` and replay require valid existing format, shard and session state.
Missing, foreign or corrupt state fails closed. Do not recreate the filesystem
or change its pools to test restart. The predecessor messenger addresses must
remain available in persisted authority state. The daemon verifies blocklisting
and publishes an Objecter epoch barrier before authority acquisition.

## Private configuration

Set these keys in `ceph.conf`, under `[mds.rust]` (or the corresponding daemon id).
Underscores and spaces follow the normal configuration key normalization.

| Key | Default | Meaning |
| --- | ---: | --- |
| `rmds_osd_op_timeout` | 30 | Objecter actor operation timeout, seconds; must be nonzero |
| `rmds_checkpoint_interval` | 30 | Periodic checkpoint interval, seconds; must be nonzero |
| `rmds_checkpoint_records` | 16 | Committed records between checkpoint requests |
| `rmds_max_record_bytes` | 1048576 | Maximum framed WAL record bytes; at most 16 MiB |
| `rmds_max_wal_bytes` | 67108864 | Retained WAL budget; must fit at least eight maximum records |
| `rmds_max_checkpoint_generations` | 64 | Retained immutable checkpoint generation limit; at least two |
| `mds_reconnect_timeout` | 45 | Reconnect base window R, seconds; positive, finite and representable through 2R |

The journal reserves one maximum record slot before each engine step. It keeps
one record in flight. The serialized store worker retains identical append
arguments across unknown outcomes. It retries at most eight times before
fail-stop. Storage callbacks carry owner and operation identities.

Send `SIGUSR1` to the daemon PID to request a manual checkpoint:

```sh
kill -USR1 "$RMDS_PID"
```

A checkpoint waits for a quiescent durable engine cut. The worker writes
immutable objects, publishes the descriptor, then trims WAL keys. A client
`REQUEST_FLUSH_MDLOG` only requests commit through the captured submitted tail.
It does not request a checkpoint or send a synthetic acknowledgment.

There is no checkpoint garbage collector. Generation exhaustion pauses ordinary
admission and logs a diagnostic. Increase the generation limit and restart with
the same filesystem identity and pools. If a nonquiescent workload consumes the
WAL reserve, increase `rmds_max_wal_bytes` before restart. Do not delete control,
checkpoint, session or WAL objects. Storage/encoding failure stops service and
discards uncommitted output.

## Session and lifecycle ordering

OPEN output follows durable rank Open. CLOSE stops admission, drains admitted
control input, persists Closing, commits engine cleanup, persists Closed, and
only then sends CLOSE through the retained session funnel. Failed blocklisting
never releases rights. Reattachment retains the union of peer addresses.

All session-dependent input must match the active attachment's exact address
and nonce, authenticated entity, authorization context, and runtime generation.
A changed address or credentials must complete durable OPEN first. Unmatched
input is dropped, not routed under the old attachment. A same-address transport
reset with unchanged credentials remains valid. Attachment completion cannot
publish into a different runtime generation.

The 1024-session limit includes durable live keys and reserved new OPENs.
Duplicate attachments reserve no additional slot. A new OPEN at capacity is
dropped before the repository command; a later retry can use a released slot.
A successful durable Open converts its reservation into a live key. Closed
releases that key. Failure cancels pending reservations and stops service.
Unrelated stale callbacks cannot release a current operation's reservation.

Normal CLOSE retains its requested sequence through drain. Before publishing
Closing, it rechecks the transmitted push sequence and excludes pending counted
output. New counted output leaves admission stopped and rights intact. The
client must retry CLOSE with the current sequence, or verified fencing must
authorize cleanup. Non-counted terminal output remains ordered ahead of CLOSE.
Engine execution pauses while a validated Closing write is pending.

A Busy control input rotates behind other controls. The scheduler makes one
bounded pass before declaring all controls blocked. A queued lifecycle result
can therefore start and commit without a Tick or another client message.
The all-Busy case sleeps until a progress event; it does not spin.

Durable lifecycle intents precede data work. Completion enters the engine as a
new admitted journal transaction. Four data jobs can run concurrently. A job
preflight bounds each attempt to 4096 candidate object operations and 64 layouts.
Oversized jobs remain durable and log errors; no partial oversized attempt starts.
Truncate preflight conservatively counts the full `from` bound. Failed attempts
retain the intent and use capped exponential retry delay. Owner-tagged callbacks
cannot retire another process's jobs.

## Collection-only G3 recovery (D1-D4)

### Permissions and attachment

Recovery control, recovery collection, output, data work and ordinary admission
have separate permissions. Each permission checks current membership, the
loaded owner when present, a healthy beacon, no self blocklist and no fail-stop.
Requested monitor state does not authorize ordinary service. Replay retains
confirmed RECONNECT and REJOIN. Required output or writes select CLIENTREPLAY
before ACTIVE. Cold standby and standby-replay do not acquire authority or run
metadata/data work. Promotion uses the full fenced load.

Every checked clientless restore now takes both accepted E2 boundaries.
`try_finish_recovery` first calls structural `finish_reconnect(&[])` and records
`RecoveryDrain::Controls`. It does not report readiness. The owning `pump`
reserves a fresh `RecordPermit` and executes `RecoveryControlsDrained` through
`Engine::step`. It returns the unused permit to the journal adapter. An empty
drain needs no WAL append, commit callback, client packet or Tick. Marker
execution records `GrantsReleased`; only drained work/output establishes
`recovered`. There is no compatibility readiness override.
If the adapter holds a permit, the marker remains pending without a checkpoint
request. Returning that permit makes the marker runnable in a fresh turn.

`recovery_structural_done` permits requesting REJOIN before full readiness.
Eligible clientless retained jobs can run after structural finish and before
ACTIVE. Required Truncate/RecoverSize completion commits precede marker release;
independent background Purge work adds no extra marker hold. CREATING/STARTING
perform both boundaries before their legal ACTIVE request. An already-ACTIVE
map during load does not skip them. The daemon rechecks phase readiness after
ordered output drains, without waiting for another external event.

`ClientLink::connection_id` identifies a messenger session. Accepted or the first
eligible early message can start `Server::recovery_accepted` / `recovery_input`.
`Opening::purpose` distinguishes recovery attachment from ordinary OPEN.
The tagged durable Open completion installs `SessionState::Recovering` through
`SessionMap::attach_recovery`. It sends no OPEN. It retains the exact durable
`SessionKey`, authenticated entity, metadata and request/flush histories.
Changed authorization context, address/nonce or advertised CephFS features must
complete another durable update before dependent input. Each update retains the
full persisted address union. One pending update and one desired replacement
can exist per key. An obsolete result retains its committed union but cannot
install its funnel. Required CephFS bits are not messenger feature bits. The
daemon rechecks them at input, attachment completion and reconstruction.

### Chunks, resets and realm output

The runtime accepts mixed v5 chunks and v3 raw realms. It parses every fcntl/flock
blob before one atomic `Engine::reconnect_chunk` call. Path hints cannot create
metadata. Repeated inode entries replace caps/locks and their retained charge.
Partial chunks retain missing membership. An empty final chunk is valid.
Final reconstruction removes missing membership once, but retains the surviving
Open key and its applied-state charge. Duplicate final chunks do not send OPEN.

Matching pre-final Reset/Closed removes partial reconstruction and staging.
It preserves identity, histories, missing membership and protected early dirty
controls. Older connection events cannot reset a replacement generation.
Final reconstructed rights survive transport loss. A replacement can attach
silently and retain the funnel sequence. A second reconstruction stream under
a different attachment token requires fenced close, not a second installation.

Authoritative `snap_trace` / `snap_realm_info` determine realm UPDATE and genuine
inode-to-realm SPLIT messages. SPLIT includes authoritative child realms.
Old/new encoding follows the session's CephFS features. Corrections use the same
ordered funnel as later OPEN. Only transmitted pushes increment `push_seq`.
Typed OPEN will receive its sequence at its FIFO transmission position.

### Bounds and missing-key cleanup

| Runtime recovery resource | Limit |
| --- | ---: |
| Charged retained attachment/input/reconstruction/correction bytes | 32 MiB per rank |
| Incoming message, including envelope charge | 1 MiB |
| Held decoded inputs | 1024: 128 reconnect chunks plus 896 early inputs |
| Applied caps / locks / reported realms | 16384 / 65536 / 4096 per rank |
| Child realms in one SPLIT | 4096 |
| Persisted peer addresses/nonces | 64 per key |

Releasing raw input does not release installed reconstruction charges.
Correction construction checks the remaining byte budget before retaining each
message. Overflow fails closed before chunk installation. These are retained
recovery-state budgets, not a process-memory bound. The existing pre-load raw
ordinary and reconnect queues each have a separate 1024-message / 16 MiB bound.
Paused output has a separate 16 MiB rank budget and 1024 messages per funnel.
At most 1024 terminal funnels retain non-admitted output. Active messenger
backlog still lacks capacity feedback; no end-to-end outbound bound is claimed.

The reconnect window starts at monitor confirmation, not the requested state or
load completion. With start t0, last valid missing-member activity L and base R,
the deadline is `min(t0 + 2R, max(t0 + R, L + R/2))`. The default base is 45 s;
the default hard bound is 90 s. A deadline wakeup needs no later Tick/input.
Raw holding queues preserve the original daemon receive time through dispatch.
The first valid final chunk counts as activity before missing-member removal.
An empty v5 final is valid activity. Completed-member duplicate finals do not
extend the deadline. The configured 2R hard bound is unchanged.
Expiry freezes every missing key, including `timeout` sessions. It adds no
extra reclaim hold. A pending attachment must settle before fencing freezes its
complete address union.

`RecoveryFence` carries owner, exact key, optional current connection/generation
and the frozen union. `recovery_fences`, `matches_recovery_fence` and
`finish_recovery_eviction` form the checked completion boundary. The daemon
requires successful fencing, a checked map containing every address and a
nonzero monotonic barrier. Failed/stale/partial proof retains rights and missing
membership. No fabricated connection is needed. Closing, committed cleanup
receipt and Closed remain mandatory. Only the tagged Closed result removes an
evicted missing member. A late current-attachment CLOSE waits for Closed and
stays behind older funnel output. Unmatched terminal output uses a bounded
non-admitted funnel. Restored Closing rows retain the existing startup fencing.

Raw reconnect holds apply only to startup/map-confirmation or health waits.
After checked clientless/all-Closed load, late/Closed reconnect receives ordered
CLOSE and unknown reconnect receives sessionless REJECT. Both immediate and
pre-load queued packets have a consumer. A current active attachment joins
verified fencing and durable Closed before its CLOSE; no reconnect packet
itself authorizes cleanup. Sessionless renewal remains silent.

A recovery RENEWCAPS send or queue-capacity failure becomes synchronous fatal
state. `pump` rechecks fatal/output state immediately after recovery processing
and before another engine or worker turn. It cancels pending admission without
starting another key's already-ready cleanup or transferring another command.

### Explicit finalization barrier and next package

`Server::try_finish_recovery` currently refuses every load that began with an
Open key. `Recovery::nonempty` stays true even if all keys later reach Closed.
It never calls `finish_reconnect` for that load. No final OPEN, ordinary
admission or ACTIVE request follows successful nonempty collection alone.
`consume_step` fails closed if cleanup emits a cap GRANT during such recovery.
It does not retain a grant payload that could predate an early dirty commit.

The clientless callers now consume the accepted E2 structural/marker API. The
engine implementation has its own review. D5 must still join the nonempty
collection state to that API before this barrier can open. It must pass all
surviving Open keys, not the missing-member set, at structural finalization.
It must drain protected early controls through the one-record engine interface,
commit dirty metadata, run eligible required jobs, execute the release marker,
drain ordered recovery output, and confirm ACTIVE before ordinary admission.
Early retries/dirty controls are currently held and charged, not executed or
acknowledged. Renewals are wire-only; valid cap/release barriers still raise the
monotonic epoch barrier. D5 retry classification and retained-job drain remain
open. E1 creator-retry behavior and mounted-client G3 qualification remain
separate work. D6 is described below. This package uses no
unaccepted traced-REPLAY or empty `-EINTR` fallback.

## ACTIVE timers and RESET reclaim (D6)

`server/runtime/d6.rs` owns bounded daemon-local policy. It adds no engine input,
store schema, wire operation or persistent timer. It follows C++
`mds/Server.cc::{handle_client_reclaim,reclaim_session,find_idle_sessions}` and
the accepted `design/recovery-contract.md` section 4.

UUID lookup uses non-Closed durable rows. OPEN uniqueness also includes pending
OPEN reservations. A same-key metadata update is legal. The old durable UUID
and the requested new UUID stay reserved until the update completes. A second
client gets the existing session REJECT, before another OPEN command. Checked
restore rejects duplicate live UUIDs. Closed tombstones are not reclaim targets
or UUID reservations.

Only an Open/Stale requester on its current durable authenticated attachment can
run reclaim. FINISH alone sends no reply. Mixed FINISH flags, unsupported flags,
or RESET with an empty UUID return `-EINVAL`. Only exact RESET is supported.
A different authenticated target name returns `-EPERM` without cleanup. An
absent live UUID returns result 0, epoch 0 and empty addresses. libcephfs converts
that all-empty RESET success to ENOENT. No sessionless REJECT is added.

An existing target freezes attachment/admission. A pending identity write must
settle before its full old-plus-new address union is frozen. The existing
`RecoveryFence` consumer proves every address in a latest applied map and raises
the barrier. Existing Closing/fencing work is joined, not killed twice.
Closing -> actual committed engine cleanup receipt -> durable Closed precedes
success. A normal close already in progress can finish before the reclaim fence;
the reply still waits for both proofs. The response reports the saved addresses
and post-fence epoch through the requester's existing FIFO funnel.

There are at most 128 aggregate reclaim waiters and thus at most 128 targets.
Repeated input from one unchanged requester joins its existing waiter. Each
waiter retains the durable key, runtime generation, connection ID and durable
OPEN identity. Completion rechecks those fields, Open/Stale state, credentials,
address, current feature requirements and the current blocklist. A removed or
replaced requester cannot inherit success. Capacity or output failure uses the
existing sticky fail-stop path. A failed fence cannot become success.

Reclaim ownership also survives queued output. `Session::send_raw` stores every
`CLIENT_RECLAIM_REPLY` in a reclaim-specific queue entry. This covers success,
absent-target and error replies. It retains the durable key/OPEN decision,
runtime generation, connection ID, address and credentials. Durable OPEN
callbacks update the funnel's decision tag, including a committed attachment
that service policy later refuses. Recovery attachment uses the same tag.

Immediately before `ClientLink::send_raw`, the funnel rechecks that identity and
requires Open/Stale state. An obsolete entry yields `Transmission::Discarded`,
not transport acceptance or failure. It consumes one bounded dequeue turn and
releases its exact byte/count charge. It cannot advance `push_seq`, reach a
replacement requester or consume a new reclaim attempt. Unrelated queued output
keeps its order. Actual transport/capacity failures keep the sticky fail-stop
behavior. This is the D6-R1 author correction; independent re-review remains
required. It does not change completed target cleanup or unblock D5.

`tick_serving` preserves engine/checkpoint progress during STOPPING but runs
session timers only in healthy confirmed ACTIVE. Let
`S=max(1,session_timeout)` and `A=max(1,session_autoclose)`. Both come from MDSMap.
Activity uses a monotonic runtime clock, initialized at durable OPEN/final
reconnect. Valid current traffic and renewals advance it; old/rejected/unknown
input cannot. `note_held_activity` validates traffic before a bounded raw queue
can delay its dispatch. This changes only the clock, not state, rights or output.

Before S, retain the session. At S or later, check autoclose first. Without a
`timeout` key, send ordinary STALE at S. A valid renewal resumes Open. STALE does
not reset the activity clock or remove engine caps/locks. Unsafe writer rights
remain until acknowledged release or verified fenced cleanup.

With a `timeout` key, never use ordinary STALE. Trim whitespace and parse ASCII
decimal `u32`. A positive P expires at `max(S,min(A,P))`. Zero/malformed values
expire at `max(S,A)`. Malformed values produce a diagnostic and retain the exact
stored text. At S=60/A=300, absent metadata becomes Stale at 60 and autocloses at
300; P=30 expires at 60, P=120 at 120 and P=0/600 at 300. Every expiry starts
fencing; elapsed time alone releases no rights. These timers do not complete a
reconnect gather or add a Ganesha reclaim hold after the separate 45/90 s window.

## Offline verification

```sh
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo test -p rmds --locked
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo test -p rmds --locked g3_
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo test -p rmds --locked g2_r
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo test -p rmds --locked g3_d6_
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo test -p rmds --locked session::tests
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo test -p rmds --locked g2_fault_
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo clippy -p rmds --all-targets --no-deps --locked
```

Pre-R1 author D6/G2-fault verification on 2026-10-25 passed 147 daemon tests, including
17 D6 tests, 4 new G2 fault tests, 61 total G3 tests and 11 preserved G2 regressions,
with 0 failed/ignored. Filters overlap. Build, daemon-only clippy,
scoped formatting and whitespace checks pass. Only existing dead-code warnings
remain. These working-tree results are not independent acceptance or live gate
evidence. Nonempty finalization remains held for D5 review.
Post-R1 results and fresh path/mode/hash manifests are in
`current/sol-daemon.md`. Prior release evidence does not identify the correction.

The persistence tests use the real store API with an atomic compound-operation
backend. They cover fencing-before-acquisition, failed/missing/foreign startup,
partial acquisition, durable reply timing, stale completion ownership, identical
unknown append reconciliation, checkpoint-only restart, failure output discard,
queued-work wakeup and retained-generation pressure. They do not prove native
RADOS or mounted-client behavior.

The `g2_r` runtime regressions cover attachment credentials/generations, both
peer addresses at the fencing boundary, delayed FLUSH_ACK and paused-output
CLOSE validation, interleaved pending-OPEN capacity, and Busy-control progress
across one-step scheduler turns. These use explicit delayed command callbacks,
not a native RADOS durability oracle.

The `g3_` tests use production `DaemonJournal` dispatch and tagged completions.
They cover early retry/flush, silent attachment/replacement, map/feature races,
mixed/v3/empty-final chunks, atomic malformed locks, unlinked-open/hard-linked
identity, reset isolation, authoritative nested realm corrections, retained
budgets, missing-key fencing/Closed ordering, phase permissions and the explicit
grant/finalization barrier. One test restarts the real store worker over the
compound-operation memory backend and inspects persisted attachment identity
before applying its completion. Offline daemon fixtures have no monitor
addresses. C1 regressions use authenticated loopback messengers through the real
daemon receive path, including all-Closed/unknown v3/v5 and the separate early
map race. C2 tests control receive times across first/duplicate finals and the
hard bound. C3 covers immediate send and paused-funnel capacity failure with
another key already ready for cleanup. E2 tests cover empty/delayed-permit marker progress,
fresh/already-ACTIVE load, REJOIN/CLIENTREPLAY and retained clientless size work.
None of these tests establishes mounted-client or native G3 recovery.

The four `g2_fault_` tests close the assigned offline coverage gaps, subject to
independent review. Actual workers reload durable Closing before cleanup, after
a range/job decision and after its terminal receipt. They preserve bounds/job
identities and verify exact Closed receipt joins and six mismatch refusals.
Lifecycle tests run the actual Objecter and `rados::execute_lifecycle` against a
deterministic OSD transport. They fail partial purge/TRIMTRUNC/STAT, preserve
partial data across metadata-worker recreation and retry the same durable job.
They check namespace isolation, bytes, truncate sequences and recovered mtime.
Probe errors never become zero-size success. Completion tests cut before append,
after failed append and after durable append with/without its callback. Old-owner,
old-attempt and duplicate callbacks cannot add another completion record.
There is no native fault seam or new live fault-matrix claim.
