# sol-platform-review
Task: Independent Sol review of completed platform changes vs f54d298 in crates/monc/** and crates/osdc/**. Review wire format, query lifecycle, checked operations, fencing, map waits, and latest-map races. No production edits, commits, subagents, or live cluster actions.
Status: Original fixes, native listing, and opt-in probe ACCEPT. Retained QA evidence independently validated: platform native validation ACCEPTED on Ceph 20.2.0/data pool 87. No bootstrap or durability qualification.
Context: Sole writable file is this ledger. Author ledger: current/sol-platform.md. Use CARGO_TARGET_DIR=target-sol-platform-review.
Next: Parent may use this scoped native evidence for integration. Store must still prove bootstrap/format hygiene and durability separately. No new live action is authorized by this report.
## Retained live-evidence review (2026-10-22) — ACCEPT

Evidence only: read `current/sol-verification.md` section Approved native platform probe and retained artifacts in `/tmp/opencode/sol-platform-live-20261022.YcfShl/`. No new cluster calls, builds, tests, or production edits.

### Source and execution identity

- Independently recomputed differences between tree-before.sha256.json and tree-after.sha256.json. The only change is unrelated `crates/store/src/sessions.rs`, from `80b7cc3533ade87064b93370085cf9123566ee1cefa100d3236d94f84d8b2a9f` to `7b9e02601012f52fde99064d149d0fa1f859a6dd7b35121d7b2e78a78b6f24d8`. No native source or manifest change occurred in the recorded interval.
- Independently hashed all 545 regular files in native-source.tar against both manifests: zero mismatches. Archived probe sources also match the previously reviewed working files. Probe module SHA256: `274becdacf5843b6340480739b39a85f009c3d757a6576fddaf4e9f46315ccbb`. Example SHA256: `82c937de86ec00de3a1c30fedebb90830de0d04a5a1e60f8db43733d81cec488`.
- Cargo.toml SHA256: `b994d2b1d62490dc3642268efa0d3096f9e2e89c555d4f42be454c3b1b2c0f06`. Cargo.lock SHA256: `6c08a4339d38684717afe40b86654742b0381c518f332885d6f6227cd66feb31`. Both remained unchanged. The archived source and build log establish modified-tree evidence, not an immutable f54d298 rerun or a binary-hash attestation.
- command.txt records sourcing devcluster/env.sh and `CARGO_TARGET_DIR=target-sol-verification cargo run -p osdc --example rados_live -- --platform-probe --pool cephfs.a.data --conf "$CEPH_CONF" --keyring "$RMDS_KEYRING" --id rust --inc 1`. The verification ledger additionally records an outer `timeout --kill-after=15 360` wrapper; command.txt omits that wrapper. The Cargo Running line confirms the actual executable, probe-only mode, explicit data pool, expanded config/keyring paths, rust identity, and incarnation 1. No skip flag appears.
- probe.log records a 16.35-second build, authentication as mds.rust/global_id 16405, messenger `v1:127.0.0.1:41463/2163076927`, and PASS. result.txt records exit=0 at 18:14:57 UTC. Environment records Ceph 20.2.0 `69f84cc2651aa259a15bc192ddaabd3baba07489`, rustc/cargo 1.99.0, and kernel 7.0.0-1012-ibm. Final process/mount artifacts show no remaining probe/MDS process or test mount.

### Native observations and cleanup

- Monitor get_version returned newest=402, oldest=1. The checked map wait reached 402.
- Both checked exclusive creations succeeded with four results. CMPXATTR success returned [1,0,0]. Wrong-owner raw rejection returned overall -125 and [-125,0,0]. ASSERT_VER success returned [0,0,0]. Stale-version raw rejection returned overall -34 and [-34,0,0].
- The log does not print data/tail bytes or numeric user_version values. The hash-verified archived probe makes exit 0 conditional on exact data/tail/version comparisons after both raw and checked rejections. CMPXATTR rejection preserves `cmp-committed` / tail `1` / its nonzero version. ASSERT_VER rejection preserves `ver-committed` / tail `2` / its advanced version. Archived guard_failure also requires checked calls to return exactly Rados(-125) and Rados(-34). Successful state-read lines and final PASS substantiate these assertions without claiming separately logged version numbers.
- The full all-namespace scan completed at epoch 402 with 8 pages, zero restarts, 6 empty continuation pages, and sentinels=[true,true]. Archived code reaches that summary only at next=None and requires both exact namespace/name identities. The read-only PG summary contains all eight PGs 87.0 through 87.7, active+clean, with primary set {0,1,2}. Combined with complete numeric-PG traversal, this supports coverage across all three primaries; the probe log itself does not contain a per-request primary trace.
- Exact default candidate: `osdc-platform-probe.1290818.1792692897111588293.default.10`. Exact non-default candidate: namespace `osdc-platform-probe.1290818.1792692897111588293`, name `osdc-platform-probe.1290818.1792692897111588293.namespaced.2`. Both owner-guarded removals returned [1,0]. Independent stat artifacts report CLI exit=1 and explicit errno 2/ENOENT for each exact identity. The initial QA assumption that ENOENT implied CLI exit 2 was corrected against retained stderr. This is not a cleanup failure or evidence of an additional deletion.
- Filesystem before/after JSON is identical: filesystem a, fscid 43, metadata pool 86, data pool 87. The retained process snapshot preserves the original mon/mgr/three OSDs, system Ganesha, and known unmounted orphan FUSE. Nothing in the reviewed probe or retained execution indicates a pool/filesystem reset, baseline-object deletion, or legacy blocklist workload.

Verdict: **platform native validation ACCEPTED**, limited to the exercised monitor query/map wait, successful and rejected guarded compounds, stable-epoch all-namespace/all-PG listing, and exact scratch cleanup on this existing allowed data pool. Live MapChanged recovery and nonempty locators were not exercised. This does not validate metadata bootstrap, format hygiene, concurrent initialization, WAL/checkpoint durability, restart/recovery, fencing takeover, or G1/G2/G3 gates. The unrelated store edit is outside this evidence package.

## Opt-in live probe review (2026-10-22) — ACCEPT

Scope: only `crates/osdc/examples/rados_live.rs` probe/setup changes and new `crates/osdc/examples/platform_probe/mod.rs`. Did not repeat the accepted core API/listing review or broad tests.

- The opt-in branch returns before legacy scratch workloads and blocklist commands. It performs no pool creation/reset, filesystem reset, PG parameter change, or baseline-object mutation.
- Scratch names and the non-default namespace include PID and Unix nanoseconds. Both objects use checked exclusive create in the same compound as initial data and owner/tail xattrs. The cleanup-created flag is set only after checked creation succeeds. A collision or negative create result never authorizes deletion. Uncertain creation leaves the candidate untouched and forces check failure.
- Every mutation sets safe_to_remove=false before awaiting. Timeout/cancellation/invalid reply keeps that state false. The body stops on failure. Cleanup does not replace uncertainty with a later read-based ownership guess. Completed mutations permit cleanup, but only for a previously verified creation.
- Cleanup visits only the two exact tracked candidates. Each DELETE compounds CMPXATTR against that candidate's unique owner value. It never derives deletion targets from pool enumeration or a name prefix. Cleanup errors or uncertain leftovers override prerequisite/skip results with exit 1. External process termination can leave only the printed candidates; it cannot guarantee cleanup.
- Guard checks establish successful CMPXATTR and ASSERT_VER mutation states first. Wrong-owner and stale-version mutations use distinct forbidden data/tail values. Raw checks require the expected overall errno and exact result cardinality. Checked checks require the expected Rados error. After each rejection, a checked read compares exact content, tail xattr, and unchanged nonzero user_version. A false-success response or actual state change cannot pass these checks.
- Full NamespaceScope::All pagination uses one-entry pages. Both exact sentinel namespace/name identities must appear. Only next=None proves scan completion. Empty continuation pages count as progress and at least one is required for the requested coverage. A MapChanged discards the cursor and all found/empty evidence, refreshes monitor/map confirmation, and restarts. Three restarts, 16384 total page attempts, and the outer deadline bound the work.
- Exit 2 reports setup/coverage/capability prerequisites. The opt-in skip flag changes only those outcomes to 77. Protocol/check/runtime deadlines and cleanup errors return 1. Missing empty-page coverage does not claim success. Once the work begins, query/map/listing timeout is a check failure, not an empty scan or skip.
- Ordinary scratch operations time out after 10 seconds. Work has a 120-second deadline. Cleanup has a separate 30-second deadline. Monitor/map setup waits are bounded. No live probe was executed during this review.

Independent offline verification: `CARGO_TARGET_DIR=target-sol-platform-review cargo build -p osdc --example rados_live` PASS. `git diff --check -- crates/osdc/examples/rados_live.rs crates/osdc/examples/platform_probe/mod.rs` PASS. No broad tests, live actions, production edits, commits, or subagents.

Exact command approved for designated QA only, on the existing allowed data pool:
```sh
source devcluster/env.sh
CARGO_TARGET_DIR=target-sol-platform-review cargo run -p osdc --example rados_live -- --platform-probe --pool cephfs.a.data --conf "$CEPH_CONF" --keyring "$RMDS_KEYRING" --id rust --inc 1
```
Do not omit --platform-probe or the explicit pool. Do not add --skip-missing-prereqs when collecting mandatory qualification evidence. Preserve output with candidate identities and cleanup results. Exit 0 is probe evidence, not complete bootstrap qualification.

## Re-review verdicts (2026-10-22)

### Original fixes — ACCEPT

- P1 is resolved in `crates/osdc/src/objecter.rs::operate_checked`. It retains the expected operation count. A nonnegative overall result requires exact cardinality. Mismatches return InvalidReply. A negative overall result preserves its errno before cardinality validation. Raw operations remain unchanged.
- `checked_compounds_require_exact_reply_cardinality` covers zero/one/three results for a two-operation request, exact-count success, a negative overall result with zero results, and unchanged raw behavior. This test passed independently.
- P2 is resolved in `crates/monc/src/monclient.rs::handle_cmd`. GetVersion requires HaveSession before allocating a handle or retaining a waiter. It sends directly on that session. It never enters the generic pre-authentication message queue. Closed receivers are rejected before admission.
- `pre_auth_queries_retain_no_waiters_or_payloads` covers no-address and authentication-in-progress states, with rejected and cancelled calls. The fake-monitor test delays authentication and checks that expired queries do not reach the monitor afterward. Both passed independently.

### Separate native PGNLS listing package — ACCEPT

No blocking source defects found in the reviewed package. This accepts the implementation for QA and integration. It does not qualify bootstrap against a live OSD.

Reviewed paths: `crates/osdc/src/listing.rs`, `crates/osdc/src/ops.rs`, `crates/osdc/src/objecter.rs`, `crates/osdc/src/lib.rs`, listing tests in `crates/osdc/tests/objecter_mock.rs`, and the offline PGNLS response fixture.

Reference checks and safety conclusions:

- `ObjectOperation::pg_nls` matches C++ Objecter.h: opcode 0x1505, u64 count, u32 start_epoch, and encoded hobject continuation. PGOP and IGNORE_OVERLAY reach the request flags.
- The explicit locator hash routes to the numeric raw PG. The existing target calculation selects its mapped primary and actual PG without hashing the empty object name or the all-namespace selector. Wire encoding preserves the raw hash separately from the actual PG. This matches C++ `pg_read`'s precalculated-PG path.
- NamespaceScope::All uses the single-byte 0x01 selector. `PrimaryLogPG.cc::do_pg_op` compares this with librados::all_nspaces. It includes application namespaces and excludes the configured internal hit-set namespace. The Rust path does not replace this with an empty/default namespace.
- Response decoding matches `osd_types.h::pg_nls_response_template`: versioned frame, hobject continuation, count, and namespace/name/locator byte strings. It accepts the legacy extra_info suffix without accepting arbitrary trailing data. The fixture comes from ceph-dencoder, not a mock-only encoding.
- PG start/end match C++ `pg_t::get_hobj_start/get_hobj_end/get_split_bits`. The next handle is an inclusive lower bound. Results must stay in PG, remain ordered, and precede the next handle. The implementation does not increment the returned object cursor and therefore does not skip its next object. Numeric PG traversal visits every PG, including empty PGs and PGs on different primary OSDs. Empty objects with Some(next) is not exhaustion.
- Pool/fsid/epoch binding and PG/handle-range checks reject wrong-pool, malformed, and stale continuations. The cursor is a trusted in-process continuation: callers must pass it unchanged, as documented. It is not an authenticated token for untrusted callers. Store must start at None and consume every returned continuation; a caller that edits public cursor fields invalidates its own scan proof.
- Any installed map epoch change invalidates outstanding or resumed listing. Map-pinned operations do not resend against a changed epoch. The reply epoch and final installed epoch must match the scan. Split, merge, pool removal, and unrelated remap require restart. Store must discard all earlier conclusions after MapChanged.
- Zero count and invalid byte limits fail. Counts above 1024 are capped. Encoded PG replies above max_bytes or count fail with SizeLimit. The code never truncates a reply and then advances its cursor. Decode/cursor errors, negative overall/suboperation results, absent maps/pools, shutdown, and fencing cannot become empty success.
- The independent 30-second timeout applies with ordinary operation timeout disabled. Dropping the list future closes its receiver and sends PruneLists. Cancellation and timeout tests verify zero retained operations. Transport frame limits remain separate from the page decoding budget.
- Exhaustion means successful end-of-PG handling for all numeric PGs at one epoch. It is not a cross-object snapshot or a concurrent-initialization lock. Bootstrap still requires exclusive creating-rank authorization and no concurrent foreign initialization. Native scan success is mandatory before the format write.

### Fresh re-review evidence

- `CARGO_TARGET_DIR=target-sol-platform-review cargo test -p monc -p osdc`: PASS. 24 monc tests, 70 osdc tests, and one osdc doc test. Zero failures or ignored tests. This includes all seven listing mock tests, three listing unit tests, both original-fix regressions, and the real 30-second listing deadline test. Tests named live_* use stored fixtures. No live cluster action ran. Environment-conditional early returns are not reported as ignored tests.
- `git diff --check f54d298 -- crates/monc crates/osdc`: PASS.
- Read C++ `PrimaryLogPG.cc::do_pg_op`, `Objecter.h::pg_nls/pg_read`, `Objecter.cc::list_nobjects/_nlist_reply`, `osd_types.cc::get_split_bits/get_hobj_start/get_hobj_end`, `osd_types.h::pg_nls_response_template`, and the current Rust wire/placement/hobject code.
- Did not add tests, edit production code, run live commands, create commits, or spawn subagents. Clippy and workspace tests were not rerun.

QA requirements remain: real get_version, guarded-compound rejection, PGNLS with non-default namespaces and locators, multi-page continuation, and full traversal across multiple PGs/primaries. No bootstrap qualification follows from offline acceptance alone.

## Original findings (historical; resolved above)

### P1 — Checked compounds accept incomplete success replies

Location: `crates/osdc/src/objecter.rs:528-536`, `Objecter::operate_checked`.

The helper checks only the results supplied by the reply. It does not retain the submitted operation count. A two-operation CMPXATTR + WRITE_FULL request therefore succeeds on `result = 0, ops = []` or `ops = [rval: 0]`. The reply decoder accepts a self-consistent message with that smaller count. `handle_op_reply` does not compare reply and request counts either. The checked helper can thus publish success without a result for each requested guard/mutation. This violates the intended fail-closed durable-caller contract. This is a malformed-reply case, not a claim that normal Ceph emits such replies.

Fix: retain `req.op`'s expected operation count before submission. Before returning checked success, require exactly that many results. Return an explicit protocol error or EIO on mismatch. Preserve raw `operate` compatibility. Preserve a negative overall error even if the failure reply omits operation results.

Regression expectations: submit a two-operation guarded mutation. Inject zero, one, and three nonnegative results with a nonnegative overall result. All must fail. Exactly two nonnegative results must succeed. Existing overall/per-operation negative cases must still fail. Durable callers must not advance a committed prefix on a cardinality failure.

Decision: **Yes**, `operate_checked` must verify expected suboperation cardinality for durable callers. Checking only the returned subset is not sufficient.

### P2 — Query timeout removes the waiter but retains its queued message

Location: `crates/monc/src/monclient.rs:595-603`, with `send_now` at 552-558, `tick` at 909-910, and `reopen_session` at 509-516.

During authentication/hunting, GetVersion inserts an encoded message into the generic `waiting` queue. Timeout or cancellation closes the oneshot receiver. Pruning `versions` does not remove that queued message. Authentication can later send requests whose callers already timed out. With an empty initial monitor address list, `reopen_session` returns before `waiting.clear()`. Repeated bounded get_version calls then grow the queued payloads indefinitely, despite regular waiter pruning. The public constructor accepts this configuration without validation.

Fix: tie queued version messages to their handles and remove or skip them when their waiters close. Alternatively, avoid queuing them until an authenticated session exists and fail/retry explicitly. Ensure the no-address path clears/fails pending query payloads too. Do not rely only on periodic session replacement.

Regression expectations: delay authentication, issue and cancel/timeout many queries, and assert that retained query payloads return to zero after pruning. Complete authentication and assert that no expired query is sent. Repeat with no monitor addresses; retained payload memory must not increase across completed timeout cycles.

## Reviewed behavior without additional findings

- Request payload is u64 handle then map-name string. Reply v2 is three u64 fields. Reply v1 omits oldest and defaults it to zero. These match C++ `MMonGetVersion.h` and `MMonGetVersionReply.h`. `Monitor.cc::handle_get_version` echoes the payload handle independently of header tid.
- Current-connection filtering rejects old-session replies. Re-hunting fails pending version waiters. Handles do not reset across hunts. Shutdown fails pending waiters. Practical handle reuse is not a concern before u64 exhaustion.
- The latest-map watermark excludes submissions after the emitted query. Cached newest-map status alone no longer proves pool absence. The setter requires the documented one-query-per-event ordering; failed queries must retry. No native pool-listing finding belongs to this completed package.
- EBLOCKLISTED latches before pending completions. It fails operations and map waiters with Blocklisted, closes sessions, and rejects later submissions/waits. Explicit map waits distinguish shutdown and timeout. Closed map waiters are pruned on ticks.
- Reset and backoff-unblock sends share the barrier/pause check. Raising a future barrier marks existing targets paused; a later map recalculates them.

## Verification actually run

- `CARGO_TARGET_DIR=target-sol-platform-review cargo test -p monc -p osdc`: PASS. monc: 23 tests. osdc: 59 tests plus one doc test. Zero failures or ignored tests. Tests named live_* in golden_maps use stored fixtures. No live cluster action ran. The command reports no skipped-test count; environment-conditional tests may return early.
- `git diff --check f54d298 -- crates/monc crates/osdc`: PASS.
- Inspected the scoped tracked diff, new `crates/monc/tests/version_wire.rs`, the author ledger, G1 plan, persistence contract, and C++ monitor/reply references.
- Did not add or run new reproduction tests because the sole writable file is this review ledger. Findings above follow directly from the control flow. Did not run clippy or workspace tests. No production edits, commits, subagents, or live cluster access.
Log:
- [2026-10-22] Read repository guidance and author ledger. Started independent review.
- [2026-10-22] Offline monc/osdc tests and scoped diff whitespace check passed. Completed source review. Recorded CHANGES REQUIRED and two regression expectations.
- [2026-10-22] Re-reviewed original fixes and separate native PGNLS implementation against C++ and Rust wire/placement paths. Independent tests passed: 24 monc, 70 osdc, one doc test. Both packages ACCEPT for QA/integration. No live qualification claimed.
- [2026-10-22] Reviewed only the new opt-in probe and cleanup paths. Targeted example build and whitespace check passed. Probe ACCEPT for designated QA execution on the explicit existing allowed pool. No live execution or broad test rerun.
- [2026-10-22] Independently validated retained live artifacts and all 545 archived source hashes against before/after manifests. Native platform evidence ACCEPTED with explicit scope limits. No new live calls, builds, or tests.
