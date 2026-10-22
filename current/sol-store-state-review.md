# sol-store-state-review
Task: Independent Sol review of completed G2 store AUTHORITY/RECOVERY STATE package. Review bootstrap, allocator, sessions, codec, and materialized recovery. Do not edit source or run live actions.
Status: ACCEPT for the assigned store authority/recovery-state scope. No blocking source defect found. Independent store tests and store-only strict clippy passed. One non-blocking test-coverage finding remains below.
Context: Base f54d298. Sole writable file is this ledger. Read AGENTS.md, HANDOVER.md, build brief, requirements/hooks/invariants, approved workplan, final persistence/store contracts, store README and author ledger. Reviewed bootstrap.rs, allocator.rs, sessions.rs, record.rs, framing.rs, assigned tests, and only necessary StableStore acquisition/load boundaries. StableStore core belongs to a parallel reviewer. Did not inspect unfinished engine diffs, edit source, commit, spawn agents, or perform live actions.
Next: Parent combines this result with the core-store review. Add the paginated bootstrap regression when extending offline scan evidence. Daemon integration must preserve the independent acquisition and recovery-join policy.
Log:
- [2026-10-22] Created review ledger before other edits.
- [2026-10-22] Verified all 40 store tests independently: CARGO_TARGET_DIR=target-sol-store-state-review cargo test -p store --locked. Result: 31 unit + 5 checkpoint/schema + 4 fixture tests passed; 0 failed/ignored. Doc tests: 0.
- [2026-10-22] CARGO_TARGET_DIR=target-sol-store-state-review cargo clippy -p store --all-targets --no-deps --locked -- -D warnings passed. This is store-only evidence, not a workspace lint result.
- [2026-10-22] Completed assigned review. No live G2 qualification follows from offline tests.

## Verified boundaries

- Format creation follows an empty-pool scan. Preparing persists the complete deterministic plan. Resume uses the original root, range and immutable names. The format object's own authority/revision/phase guards acquisition and Ready publication.
- Takeover is not circular. Resume can create missing deterministic dependencies under their original planned guards, then return Fenced. The daemon can inspect and acquire each existing shard/session object with its own predecessor proof. Retrying resume permits changed authority fields but requires unchanged initial data, counters and positions. Publication checks converged guards. Daemon fencing and exclusive creating assignment remain prerequisites.
- The allocator atomically stores a stable reservation receipt and advances its high watermark. A lost response reconciles by reservation id and count. CAS rejection cannot overwrite another reservation. Allocation stays outside the reserved inode interval and below 2^48.
- Session acquisition preserves rows and the next-incarnation counter. Open reattachment retains the key. Closing blocks Open. Closed retains its cleanup sequence and tombstone; later Open allocates a new key. Pending mutations retain exact bytes and prevent another action. The daemon must validate cleanup receipts and join shard histories; the repository does not duplicate that policy.
- Schema 2 records reject schema 1 and unsupported nested versions. Evolving nested elements use length-delimited frames. Decode counts do not cause direct unbounded preallocation; remaining input limits successful decoding. Production encoding performs a conservative size preflight.
- The checkpoint codec carries materialized inode cores/linkage, authorization paths, stable identity, old layouts, allocator state, incarnation-scoped retry/flush history and empty-history watermarks, cleanup receipts, jobs and identifier seeds. Runtime caps/locks are absent. Existing tests independently restore a materialized checkpoint after WAL removal and check checkpoint-at-L/unlink-at-L+1 codec replay.

## Non-blocking finding

### P3 — Bootstrap pagination/map-reset behavior lacks an offline regression

Location: crates/store/src/stable.rs:1357-1391 (PoolListing test implementation); crates/store/src/bootstrap.rs:701-760 (scan).

The mock ignores the supplied cursor and always returns next=None. The map-change test injects failures before any successful page. Thus the passing tests do not demonstrate a foreign object on a later page, an empty continuation, or restarting at None after partial progress. Code inspection found the expected incremental/restart logic; this is not a verified source defect.

Concrete regression: add a scripted listing backend with multiple pages and an empty continuation. Put a foreign object in a non-default namespace on the last page. Inject MapChanged after the first successful page. Assert that the next call receives None, that the full restarted scan reaches the foreign object, and that no bootstrap dependency or initial format write occurs before the fresh scan completes. Also exercise successful pagination and repeated-cursor rejection.

## Limits

This review accepts only the assigned store package scope. It does not accept the parallel StableStore core scope, unfinished engine durability work, daemon recovery joins, live bootstrap, native map-change behavior, mounted-client failover, or G2/G3 gates.
