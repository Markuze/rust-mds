# sol-platform
Task: Implement G1 platform support for durable fencing in crates/monc and crates/osdc. Add monitor get_version, explicit map wait outcomes, checked compound operations, and relevant fault tests. Do not modify consumers or the live cluster.
Status: Independent reviewer accepted fixes and native listing. The opt-in real-peer probe is implemented and compiles offline. Ready for cleanup review, then QA execution. This lane did not run it.
Context: Own crates/monc/**, crates/osdc/**, and this ledger. Use CARGO_TARGET_DIR=target-sol-platform. Preserve raw operate and existing compatibility APIs where practical.
Next: Independent reviewer checks the new probe and cleanup rules. QA alone executes the command below on the existing allowed pool. No daemon/store changes are required for this probe.
## QA real-peer probe (new separate package)
Only changed crates/osdc/examples/rados_live.rs and added crates/osdc/examples/platform_probe/mod.rs for this package.

Exact QA command (not executed by this lane):
```sh
source devcluster/env.sh
CARGO_TARGET_DIR=target-sol-platform cargo run -p osdc --example rados_live -- --platform-probe --pool cephfs.a.data --conf "$CEPH_CONF" --keyring "$RMDS_KEYRING" --id rust --inc 1
```

- Reuses rados_live's monitor authentication, single messenger and OSDMap routing. The opt-in branch returns before all legacy scratch operations and monitor blocklist commands. No production platform APIs changed in this package.
- Defaults remain `--pool cephfs.a.meta`, `--id rust`, `--inc 1`; config comes from --conf/CEPH_CONF and keyring from --keyring/RMDS_KEYRING. The explicit QA command chooses the existing data pool to improve empty-PG coverage. QA may substitute another existing allowed pool. The probe never creates pools, changes PG counts, resets the filesystem, or calls newfs.
- Prerequisites: readable config/keyring for the selected MDS identity, reachable msgr1 monitor/OSDs, an existing pool, OSD caps for read/write/xattrs/remove in default and non-default namespaces, all-namespace PG listing, and SORTBITWISE. Full coverage requires at least one observed empty continuation page. A single-PG or populated pool may not provide this; the probe reports a prerequisite failure rather than claiming that coverage passed. It does not manufacture empty PGs.
- Exit 0 means every check and cleanup passed. Exit 1 means a protocol/check/deadline/cleanup failure. Exit 2 means prerequisite failure or missing empty-continuation coverage. Optional --skip-missing-prereqs changes only prerequisite exit 2 to explicit skip exit 77. Timeout or listing errors never mean empty success.
- get_version("osdmap") has a 5-second deadline, validates newest/oldest, converts newest with checked u32 conversion, and requires wait_for_map_timeout to reach it within 15 seconds. The probe does not feed unrelated query results into set_latest_map_version.
- Creates two unique objects via checked exclusive create+write+owner xattr+tail xattr: one in the default namespace at PG 0 and one in a unique non-default namespace at the final PG of the selection snapshot. Names/namespaces start osdc-platform-probe.<pid>.<Unix-nanos>; there are no rmds marker names. Candidate selection has an 8192-attempt bound. Exclusive-create collisions are failures, never overwrite/remove permission.
- Checks checked CMPXATTR+write+tail and ASSERT_VER+write+tail success, then wrong-owner and stale-version rejection. Each failed guard is tested through both raw and checked APIs. Prints only numeric overall/per-op results and exact scratch identities. Requires exact result cardinality for these real-peer compounds. Reads content and tail after each rejection and checks unchanged user_version as well.
- Performs full all-namespace/all-PG pagination with max_entries=1 and max_bytes=4096. Requires both sentinel identities and at least one empty continuation page. Only next=None completes the scan. MapChanged restarts from None with a fresh get_version/checked wait and discards found/empty-page evidence; at most three restarts and 16384 page attempts are allowed.
- Runtime bounds: shared setup waits at most 30 seconds for authentication and 20 seconds for the first map. Probe work has a 120-second outer deadline, ordinary OSD operations have a 10-second timeout, and cleanup has a separate 30-second deadline (about 200 seconds total network-wait budget). Native listing retains its own 30-second per-page deadline.
- Cleanup visits only the two exact candidates tracked in this run. It removes an object only after checked exclusive creation succeeded and no mutating outcome is uncertain. Every removal compounds a comparison of this run's unique probe_owner value with DELETE. It never lists names for deletion or removes an unverified collision.
- If creation or a later mutation has an uncertain result, cleanup leaves that candidate alone and prints its exact name/namespace. This prevents racing DELETE against a delayed write. Such leftovers or cleanup failures force exit 1, not skip/success. An external kill cannot guarantee cleanup; the pre-create candidate lines identify the only possible leftovers for QA inspection.
- Offline evidence: `CARGO_TARGET_DIR=target-sol-platform cargo build -p osdc --example rados_live` PASS; `CARGO_TARGET_DIR=target-sol-platform cargo clippy -p osdc --example rados_live --no-deps -- -D warnings` PASS. No live probe, broad regression rerun, or cluster command ran. No new reusable library helpers required new unit tests.
## Native listing API (separate package; ready for store consumption)
All types below are re-exported from `osdc` and defined in `osdc::listing`.
```rust
enum NamespaceScope { All }
struct PoolObject { pub namespace: Vec<u8>, pub name: Vec<u8>, pub locator: Vec<u8> }
struct PoolListCursor {
    pub osdmap_epoch: u32, pub pg: u32, pub handle: Vec<u8>,
    // private pool and fsid fields bind this opaque cursor to its source
}
struct PoolListPage {
    pub objects: Vec<PoolObject>, pub next: Option<PoolListCursor>,
    pub osdmap_epoch: u32,
}
enum PoolListError {
    MapChanged, NoMap, PoolDoesNotExist, Unsupported, InvalidCursor,
    InvalidLimits, SizeLimit, Decode, Io(OpError),
}
async fn Objecter::list_pool_page(&self, pool: i64, scope: NamespaceScope,
    cursor: Option<PoolListCursor>, max_entries: u32, max_bytes: usize)
    -> Result<PoolListPage, PoolListError>;
```
- Store starts at None, examines each page incrementally, and passes `page.next` unchanged. Empty objects with Some(next) is progress through empty PGs, never exhaustion. Only next=None proves every PG completed at page.osdmap_epoch. On MapChanged discard scan conclusions and restart at None under the store deadline/retry budget. Never write the format marker on any listing error.
- Each call performs at most one native PGNLS read against one numeric PG. OSD continuation is inclusive within the PG. End-of-PG advances to the next numeric PG; all PGs are visited independently of their primary OSD or bitwise order.
- All uses LIBRADOS_ALL_NSPACES (single byte 0x01). Native OSD semantics omit the OSD's internal hit-set namespace; every application namespace is included. Names/locators/namespaces stay opaque bytes.
- Cursor pins exact map epoch and fsid. Any epoch change, including unrelated map changes, returns MapChanged. This conservative policy covers PG split, merge, pool change, and remap. In-flight map-pinned operations fail instead of resending against a changed map.
- max_entries must be positive; requests above 1024 produce smaller pages capped at 1024. max_bytes must be 49..=4 MiB. The byte bound applies to encoded PG reply data, including framing and continuation, before object decoding/copying. PGNLS has no server byte-budget field: oversized replies return SizeLimit without partial success. Existing transport frame limits apply before this API receives the data.
- Each PG read has an independent 30-second deadline even with unbounded ordinary MDS operations. Timeout/cancellation removes its queued/in-flight Objecter operation. Shutdown/fencing/negative OSD results are explicit errors. A missing map or pool is an error, never empty success.
- Pool listing is not an atomic cross-object snapshot or initialization lock. Store must enforce exclusive assigned creating rank and no concurrent foreign initialization, as the accepted contract requires.
- Minimal codec extension: ObjectOperation::pg_nls(count: u64, start_epoch: u32, handle: &HObject) -> &mut Self. Encodes opcode 0x1505, count/start_epoch union, and hobject continuation. Objecter now includes the builder's documented op.flags in target flags; this was needed to preserve PGOP/IGNORE_OVERLAY.
- Offline golden fixture: crates/osdc/testdata/pgnls_response3.hex from `ceph-dencoder type pg_nls_response_t select_test 3 encode`. Its regeneration script makes no cluster calls.
### Listing verification
- `CARGO_TARGET_DIR=target-sol-platform cargo test -p monc -p osdc`: final PASS, 24 monc tests, 70 osdc tests, one osdc doc test. Zero failed/ignored. This includes the real 30-second listing deadline with ordinary op_timeout=None and zero retained ops afterward.
- Seven listing mock tests cover foreign namespace on PG 3 after three empty PGs (different primaries), stable full exhaustion, inclusive continuation, requested count and hard 1024-entry cap, malformed/wrong-pool cursors, byte/count limits, malformed/no-progress replies, split/map/pool-removal restart, missing map/pool, negative whole/per-op results, unsupported server/sort order, cancellation, shutdown, and fencing.
- Three listing unit tests cover the C++ pgls request layout, real ceph-dencoder namespace response with every truncation rejected, legacy extra_info/trailing bytes, and complete disjoint bitwise PG ranges for pg_num 1..=33.
- `CARGO_TARGET_DIR=target-sol-platform cargo clippy -p monc -p osdc --all-targets --no-deps`: PASS with only the existing monitor commands-map type_complexity warning. No new listing warnings.
- Owned-file rustfmt check and scoped git diff --check pass. No shared manifests or consumer files changed.
- Listing files: crates/osdc/src/listing.rs (new), crates/osdc/src/lib.rs, crates/osdc/src/ops.rs, crates/osdc/src/objecter.rs, crates/osdc/tests/objecter_mock.rs, crates/osdc/testdata/pgnls_response3.hex (new), crates/osdc/testdata/gen_pgnls_response.sh (new).
- QA still owes native PGNLS verification against real OSDs, including non-default namespaces and multi-PG paging. No shell/CLI fallback exists. The only ceph-dencoder invocation generated an offline codec fixture.
## Review fixes (separate package)
- P1: `operate_checked` retains the submitted operation count. Nonnegative top-level replies require exactly that many results, or return `OpError::InvalidReply`. Negative top-level errors keep their original errno even with zero results. Raw `operate`/`submit` remain unchanged.
- P1 regression `checked_compounds_require_exact_reply_cardinality` failed before the fix (zero results incorrectly succeeded). It now rejects zero/one/three results for a two-operation compound, accepts exactly two, preserves a negative overall result with zero results, and verifies raw incomplete results remain exposed.
- P2: `get_version` rejects calls before HaveSession with `monitor not authenticated`. It never inserts query payloads in the generic waiting queue. The caller must authenticate/retry. Cancellation before task admission retains neither waiter nor payload.
- P2 regressions: private task test issues 64 rejected and 64 cancelled calls both with no addresses and with an authenticating connection; asserts zero retained waiters and payloads after pruning. Fake monitor holds authentication during rejected/cancelled calls, then completes it and verifies that no expired query reaches the monitor.
- Review-fix test command: `CARGO_TARGET_DIR=target-sol-platform cargo test -p monc -p osdc` passed: 24 monc tests, 60 osdc tests, one osdc doc test. No ignored tests. No live calls.
## Integration API (2026-10-22)
- `MonClient::get_version(&self, what: &str, timeout: Duration) -> Result<(u64, u64), String>` (async): returns `(newest, oldest)`. Uses the payload handle. Session loss, shutdown, and timeout fail the query. Call after successful blocklist, then convert newest to u32 with a checked conversion.
- `Objecter::operate_checked(&self, req: OpRequest) -> Result<OpReply, OpError>` (async): checks top-level result and every per-operation result. Negative results become `OpError::Rados(errno)`. Raw `operate` remains unchanged.
- `Objecter::wait_for_map_result(&self, epoch: u32) -> Result<u32, OpError>` (async): explicit shutdown outcome.
- `Objecter::wait_for_map_timeout(&self, epoch: u32, timeout: Duration) -> Result<u32, OpError>` (async): bounded wait; returns `TimedOut` or `Shutdown`. Use this in fencing paths, not legacy `wait_for_map`.
- `ObjecterStatus::latest_map_version: u32`: latest known monitor/map newest epoch. Existing `set_latest_map_version(u32)` resolves `CheckLatestMap`. On query failure retry rather than assume the map is current.
- Latest-map query ordering: Objecter emits one coalesced `CheckLatestMap` event at a time. Run one monitor query per event. Retry failures until successful or terminate the owner; do not discard a failed query silently. Report only that event's successful response to `set_latest_map_version`. A response covers only operations present when the event was emitted. Later missing-pool operations cause another event. Do not feed unrelated fencing queries into this setter. Newest-map status alone is not current-map confirmation for a new operation.
- Daemon integration: handle `CheckLatestMap` with get_version and set_latest_map_version; use checked waits before session cleanup; use operate_checked for truncate work. Store integration: use operate_checked for all guarded compounds; never publish commit on Err.
## Verified changes
- Monitor messages encode the payload handle and map name. Reply v2 carries newest/oldest; reply v1 defaults oldest to zero. Fake monitor checks concurrent correlation with an unrelated header tid, timeout, monitor loss/re-hunt, shutdown, and stopped queries.
- Checked compound tests cover CMPXATTR ECANCELED, ASSERT_VER ERANGE, top-level errors, per-operation errors, and success. Existing raw-result tests still pass.
- Checked map waits report timeout, shutdown, and latched Blocklisted. Timed-out waiters are pruned.
- Cached monitor/map versions no longer prove pool nonexistence. Query watermarks exclude later submissions. Tests cover waiting for the confirmed epoch and a second operation arriving during the first query.
- EBLOCKLISTED latches Objecter fencing, fails every pending operation/waiter, closes OSD sessions, and rejects later operations/waits. No late reply can produce success after the latch.
- Backoff unblock and reset sends check pause/barrier state. Raising a barrier pauses existing targets so map arrival resumes them. The backoff test verifies that unblock cannot bypass a newly raised barrier.
- Single messenger identity and MDS client incarnation behavior remain unchanged. No msgr2 work.
## Files
- crates/monc/src/messages.rs
- crates/monc/src/monclient.rs
- crates/monc/tests/fake_mon.rs
- crates/monc/tests/version_wire.rs (new)
- crates/osdc/src/objecter.rs
- crates/osdc/tests/objecter_mock.rs
## Test evidence
- `CARGO_TARGET_DIR=target-sol-platform cargo test -p monc -p osdc`: final run passed 23 monc tests, 59 osdc tests, and one osdc doc test. No ignored tests. Fixture tests named live_* read stored data only; no cluster calls ran.
- `CARGO_TARGET_DIR=target-sol-platform cargo clippy -p monc -p osdc --all-targets --no-deps`: passed with one existing type_complexity warning for the monitor commands map.
- Strict clippy with `-D warnings`: blocked by 46 existing identity_op errors in generated crates/denc/src/features.rs. Strict --no-deps is blocked by the existing commands-map warning. The new versions map uses a local sender alias and adds no warning.
- Rustfmt ran only on the six touched owned Rust files. `git diff --check -- crates/monc crates/osdc current/sol-platform.md` passed.
## Remaining evidence
- QA must run native get_version against Ceph 20.2. New wire layout tests follow C++ headers; no captured get_version golden fixture exists in the supplied captures.
- Mock guard tests verify client result handling, not OSD atomic guard execution. QA must verify real guarded-compound mutation rejection.
Log:
- [2026-10-22] Added opt-in real-peer probe after independent platform acceptance. Compiled the example and passed strict example-only clippy offline. Published exact QA command, coverage prerequisites, deadlines, and conservative exact-object cleanup. Did not execute live or modify other lanes.
- [2026-10-22] Read CHANGES REQUIRED platform review and accepted persistence contract. Plan: reject checked result-count mismatches; fail pre-auth get_version without queueing; then implement native all-namespace bounded pool listing. Keep the raw operation API unchanged.
- [2026-10-22] Started platform work. No live-cluster access or shared-file edits.
- [2026-10-22] Offline baseline: monc 22 tests, osdc 55 tests, one osdc doc test passed. No live cluster accessed.
- [2026-10-22] First full rerun exposed an old test that assumed cached versions prove pool absence. Updated its ordering to request fresh confirmation. Focused Objecter tests then passed.
- [2026-10-22] Finished API implementation, query-watermark race test, latched fencing, and barrier/backoff regression. Final full targeted tests passed. Recorded pre-existing lint blockers.
- [2026-10-22] Addressed independent P1/P2 review with separate test evidence. Read accepted contract/reviewer resolution. Implemented bounded native PGNLS, exact map pinning, cancellation cleanup, golden codec fixture, and namespace/PG/bounds/error regressions. Final full targeted tests passed (94 tests plus one doc test).
