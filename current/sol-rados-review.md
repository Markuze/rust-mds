# sol-rados-review
Task: Independent Sol review of completed crates/mds/src/rados.rs changes vs f54d298. Review striping, truncation, probe, error propagation, and compatibility. Do not inspect unfinished daemon diffs.
Status: Complete. ACCEPT. No correctness or compatibility blocker found in the scoped diff. Fresh targeted tests passed: 13 passed, 0 failed, 0 ignored.
Context: Reviewed git diff f54d298 -- crates/mds/src/rados.rs and the complete owned file. Read AGENTS.md, HANDOVER.md, build brief, relevant requirements/decisions, approved workplan G1, persistence lifecycle contract, and current/sol-rados.md. No unfinished daemon diffs were inspected. No code changes, commits, subagents, or live actions.
Next: Parent integrates awaitable Results with durable lifecycle intent/completion. QA supplies live evidence. These are integration dependencies, not blockers for this patch.
Log:
- [2026-10-22] Created review ledger before review.
- [2026-10-22] Compared object counts, overlap, object-local truncation, and inverse mapping with C++ Striper.cc, Filer.cc, and include/fs_types.h. Inspected only the required Objecter/operation boundary source, not other daemon implementations.
- [2026-10-22] CARGO_TARGET_DIR=target-sol-rados-review cargo test -p rmds rados::tests: 13 passed, 0 failed, 0 ignored, 22 filtered out. Five dead-code warnings in other daemon files; no rados.rs warnings. No live tests.
- [2026-10-22] git diff --check f54d298 -- crates/mds/src/rados.rs passed.

## Verdict and ranked findings

**ACCEPT.** No blocking findings. The following are integration dependencies and evidence limits, not new defects:

1. The spawn wrappers still log only (`rados.rs:251-289`). The durable executor must await the new Results. It must retain intent on error, commit completion and recovered metadata, reject stale completions, and fence/drain conflicting writers and destructive work. This patch documents that boundary. It does not claim to complete the durable lifecycle.
2. Probe uses one sequential STAT per candidate object (`rados.rs:237-243`). Time and OSD operation count are O(num_objects(max_size)); memory is O(1). This is the correct conservative tradeoff for maximum mtime across all objects. C++ also continues backward to offset zero when mtime is requested (`Filer.cc:304-306,370-395`). Future bounded concurrency is an optimization, not an acceptance requirement.
3. Tests verify pure mapping/error helpers, not a mocked end-to-end Objecter execution. No fresh live evidence was collected. The reviewed call sites explicitly carry each local size, sequence, pool, and namespace, and check their returned errors.

## Correctness evidence

- Period is OS*SC, not SU*SC (`fs_types.h:129-131`). `num_objects` now agrees with `Striper::get_num_objects` (`Striper.cc:350-364`) without its overflowing ceiling-add expression. For SU=4, SC=3, OS=8, counts at file sizes 0,4,5,12,13,24,25 are 0,1,2,3,3,3,4.
- Independently read stripe positions for SU=4, SC=3, OS=8: object 0 holds file bytes [0,4) and [12,16); object 1 holds [4,8) and [16,20); object 2 holds [8,12) and [20,24). Object 3 starts at byte 24. This validates the fixed reference vectors without treating their implementation formulas as an oracle.
- Truncation to 14 keeps local lengths 6,4,4,0 in objects 0,1,2,3. Range [14,18) touches objects 0 and 1 only. Exact stripe and object-set boundaries follow the same prefix argument. Fully retained objects return None; later objects return Some(0). This matches `Striper.cc:310-348` for actual truncate inputs. C++'s UINT64_MAX no-truncate sentinel is not used by these range operations.
- Overlap compares two local prefix lengths. Their positive difference is exactly the object's contribution to [size,from). Each object is emitted once. Empty ranges emit none. Ascending object order does not change the per-object truncate sequence. Each operation receives the unchanged input sequence and the correct local length (`rados.rs:209-218`; `Filer.cc:590-597`).
- Probe maps the final byte plus one. Mapping the exclusive local end would fail at SU boundaries. Object 0 length 8 ends at file byte 16, while object 2 length 1 ends at 9. Taking the largest object number would therefore be wrong. The patch correctly takes maximum mapped end, independently from maximum mtime. Missing objects contribute neither. Existing empty objects contribute mtime and zero size. A zero bound has no candidates. An end beyond max_size, an object larger than OS, or inverse overflow returns EIO rather than false recovery success.
- Layout fields are u32. Valid layouts have positive SU/SC and OS a positive multiple of SU. OS*SC fits u64. Count is at most file size. Prefix lengths are at most OS. The inverse's within-set term is at most OS*SC-SU; base and final addition use checked arithmetic. Public async calls validate before division. Range subtraction occurs only after from>size, so its saturating end addition cannot hide overflow in the production call.
- Purge and overlap iteration use O(1) memory, even at u64::MAX. The tests exercise lazy iteration over that bound. Scanning a large stripe count can still take linear work; this is not an allocation or arithmetic defect.
- All three async operations derive the locator from the supplied layout (`rados.rs:157-160`). They preserve the retained pool and namespace, rather than consulting current defaults. Snapshot context remains the existing empty-context extension point; snapshots are not newly claimed.
- Purge checks negative overall and suboperation results and ignores only ENOENT (`rados.rs:178-188`). Truncate checks both layers and propagates other errors. C++ OSD TRIMTRUNC on a missing object returns successful no-op (`PrimaryLogPG.cc:7128-7145`), so sparse holes do not require an ENOENT exception in truncate. Sequence checks remain OSD-side (`PrimaryLogPG.cc:7153-7163`).
- Recovery uses Objecter::stat, which checks both reply layers before decoding (`objecter.rs:568-576`). Only ENOENT becomes an absent candidate. Timeout, missing pool, blocklist, and other OSD failures propagate. The local pure helper tests exercise those distinctions.
- The three async signatures now return usable Results. Compatibility spawn signatures remain unchanged. No new kernel-derived code was used or reviewed.
