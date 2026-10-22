# Durable root-shard daemon (G2)

This package uses the native Rust Objecter and `store` compound operations.
Production uses `Engine<DaemonJournal>`, not `MemJournal`.
Offline tests pass. Independent daemon review and native/live G2 validation
remain required. QA owns the cluster.

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

## G3 boundary and limits

Clientless recovery advances only after checked load and monitor-confirmed
recovery phases. Nonempty Open-session gathers stay held for G3. Reconnect
packets have a separate bounded holding queue. G2 does not fabricate reconnect
success or reject supported clients merely because G3 is not installed.
Restored Closing rows undergo verified client fencing before cleanup resumes.

G3 still needs reconnect reconstruction, gather timeouts, client replay, and
mounted-client recovery qualification. Active messenger outbound backlog still
lacks capacity feedback. Daemon-side admission, paused output and worker queues
are bounded, but this is not an end-to-end outbound memory bound.

## Offline verification

```sh
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo test -p rmds
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-sol-daemon cargo clippy -p rmds --all-targets --no-deps
```

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
