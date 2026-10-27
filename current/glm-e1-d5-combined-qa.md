# glm-e1-d5-combined-qa

Task: NEW sole-QA combined offline Rust verification after coordinator acceptance of integrated E1
engine/store and daemon startup/configuration/adoption/D5 source. Build the COMPLETE immutable
candidate (committed base 36a920d94c1e1b2cb96041fe04bd85d91d718f73 + only accepted Cargo/crate
overlay bytes from frozen source + separately pinned root testdata/maps/fsmap.bin), then run EXACTLY
one pass of the six assigned Cargo commands on it with unique CARGO_TARGET_DIR and task-local
TMPDIR. Record actual complete commands/env policy, logs, durable exits, compiler/tool versions, all
harness counts, ignored/filtered counts, optional early-return bodies and fixture gaps. Seal final
logs/maps/results plus a compact HANDOFF with exact candidate/full-source archive pins and binary
hash WITHOUT running or replacing the daemon. Return nonempty COMPLETE/PASS or the exact stopped
blocker with remaining Next.

Status: COMPLETE/PASS — SEALED. All six assigned commands ran exactly once each, in order, on the
immutable candidate under ordinary checked-in profiles (corrected runner run-one-ordinary.py;
draft run-one.py preserved at SHA05f5ac18). Every command exit0. Workspace test: 733 passed/
0 failed/2 ignored/0 filtered across 47 harnesses (9 unit, 30 integration, 8 doctest groups);
ignored = dump/extract_corpus only; four optional early-return bodies labeled; store integrations
and all doctest groups reached. Daemon build exit0 (dev profile, 35.03s). fmt check clean; fmt
write PROVEN byte/mode no-op (0 of 1,843 files changed). Default clippy 0 warnings; strict clippy
-D warnings exit0, 0 warnings. Stage4 post-check: candidate tree equals recorded map after all
checks; candidate667 and primary667 both frozen-equal; fsmap equal on both. New binary
3e8669bfcf82397078f179418efa70846d0f42a2c3d7e625537090d7e2a63d7d built, never executed, does not
replace preserved 330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374. Sealed:
results.json, SEAL.json, seal-addendum.json (pins HANDOFF.md 7a1cba89…), HANDOFF.md, evidence
archive combined-qa-evidence.tar.gz fbeaf71f20a452db49ad88159cf87b02778a54ea9d662eeb76d1022a6b57112f
(31 members, entry-map b39d9b1bdda3549e4bf869764d18d77e6a72fe10e63c7b5268815d7b646facdf).
Old labels unchanged: 612/0/2 stays pre-D5; 586/1/2 stays failed; ID323 stays failed;
strict2848/2872 stays FAILED; retained author/reviewer counts NOT added to 733. No Git write,
no live/cluster/mount/key/daemon action, no supplemental promotion; no live adoption follows.
Context: Accepted frozen D5 candidate B=/mnt/workspace/sol-daemon-e1-d5-20261026.g5o2cugy
(SEAL.json, artifacts/HANDOFF.md, OWNERSHIP-AND-COVERAGE.md, VERIFICATION.md,
qualified-inputs.json, promotion-receipt.json).
Candidate667 map SHA7a901827b32455aa234912a3f7f00eb92c02e046feef28b48702c611f3eb0aa4.
Source668 archive SHA e65c7b5e38c77bc9532a1e69fcf3d2cfee092fa18ff4851d7f1e142e14e35c52.
Promotion receipt SHA bb6650089858df0bb54de82e391902ad5321dc1411c58267008f3e206b505785.
EXTRA root testdata/maps/fsmap.bin SHA 84f149cb0ca619006f6f6e9ac7117d5749b9828ee826ba4c8c695149944dfc47
(outside667 and MUST be included in the candidate).
Exact69 engine/store accepted reference 6da20506bdcafabf04b0c461e431d0a7c168cb96c08024e83f50e993bcb65d9b;
current source groups 51 engine/18 store/25 mds/573 other. Source stays frozen during this task.

Writable scope (ONLY): current/glm-e1-d5-combined-qa.md; NEW exclusive external
/mnt/workspace/glm-e1-d5-combined-qa-20261026-* root; unique CARGO_TARGET_DIR; task-local TMPDIR.
Prefer /tmp/opencode; authorized new /mnt/workspace fallback on EDQUOT. No existing file moved/
removed. Retain failed attempts. No .env or devcluster/env.sh load; sanitise inherited live env.

Read first (no writes): current/sol-e1-r4-integration.md (Combined QA authority), current/sol-orchestration.md,
completed current/sol-g3-fault-reader-lock-r3.md, completed current/glm-daemon-e1-d5-runtime-review.md,
current/glm-daemon-e1-d5-startup-review.md, current/glm-daemon-e1-d5-runtime-addendum.md.
Addendum binding: 668 shared supplemental entries = 667 unchanged+1 changed, plus 1 new = 669; the
supplemental test is external and MUST NOT be promoted into the combined candidate. The alleged
missing E1 report is a verified symlink with retained bytes; the mutable coordinator report is
intentionally excluded from equality checks. Retained separate evidence: runtime fresh179, D5
optimized10, probe1 per mode, build/strict/fmt, startup 29 debug (10 overlap)/17 optimized. Strict
current-path 2848/2872 remains FAILED with 24 origin-routed authorized preimages; do not re-audit
the whole historical archive series; verify authoritative relevant seals and use credited retained
audit where identity holds.

Old counts to preserve, never relabel: 612/0/2 pre-D5, failed 586/1/2, original ID323 retained.

Candidate rules: complete candidate = committed base 36a920d94c1e1b2cb96041fe04bd85d91d718f73
(pinned explicitly, never mutable HEAD) + only accepted Cargo/crate overlay bytes from frozen
source668, + separately pinned root testdata/maps/fsmap.bin, + all required committed fixture/
corpus inputs (not merely the 668 narrow daemon archive). Read-only Git object/archive access
permitted; no real/alternate index/object/ref writes. Exact9-path P1 release may advance HEAD
concurrently but adds no Cargo/Rust inputs and cannot change the fixed candidate. Verify PRIMARY
667 + capture equals frozen source before execution and after all checks. Validate source and
fixture identity; investigate any unexpected compile/test input rather than copying active docs/
tooling wholesale. No active NFS-review/release output reads. Committed baseline docs may be read;
current canonical docs are an older cutoff.

Six commands, EXACTLY one pass each, in this order, on the complete immutable candidate with unique
CARGO_TARGET_DIR and task-local TMPDIR:
1. cargo test --workspace --locked --offline -- --nocapture   (all integration + doctest harnesses)
2. cargo build -p rmds --locked --offline
3. cargo fmt --all -- --check
4. cargo fmt --all              (authorized ONLY on the external candidate copy; prove byte/mode
                                 no-op against the pre-format map; if fmt --check fails, preserve
                                 the diff and STOP without running the formatter)
5. cargo clippy --workspace --all-targets --locked --offline
6. cargo clippy --workspace --all-targets --locked --offline -- -D warnings

Failure/scope rules: run formatting check before mutating fmt. Offline cache policy is fixed; if
missing dependencies prevent the offline build, report the exact blocker instead of changing cache/
network policy. Inspect failures and return concrete owner-scoped findings; no source fix and no
broad repeats. Optional golden bodies may return early; label them rather than claim exercised.
All tests must be reachable; store integration tests and every doctest group must be accounted.
Target counts come only from actually observed complete outputs; never add author/reviewer counts.
Sanitise inherited Ceph/live/credential env; do not load .env or devcluster/env.sh.

Seal: final logs/maps/results and compact HANDOFF with actual decision, exact candidate/full-source
archive pins and binary hash WITHOUT running or replacing the daemon. Preserve old
target-sol-verification/debug/ceph-rmds SHA 330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374
as file-preservation. Native fs44/meta88/data89, both namespaces, all 152 prior evidence checks stay
file-preservation only. No live adoption follows from a pass. No Python provider tests and no
full275 rerun under this assignment.

Next: STAGE3 IN PROGRESS on resumed assignment (zai-coding-plan GLM route after the free-usage
stop). Resume re-verification PASSED (stage3-reverify.json): draft run-one.py preserved byte-
identically at SHA05f5ac18; candidate-input-map.json still88939e65; candidate-equality.json still
1b93dc30; no logs/, no candidate target/tmp, no receipts — nothing had run. Candidate tree equals
its recorded1,843-file map; candidate667 and primary667 both frozen-equal; fsmap equal.
SUPERSEDED PLAN NOTE: the earlier Next text mirrored the author's env overrides
(CARGO_PROFILE_DEV/TEST/RELEASE_DEBUG=0 + overflow + CARGO_INCREMENTAL=0). That plan is superseded:
the assigned policy is ORDINARY CHECKED-IN PROFILES with no CARGO_PROFILE_*/CARGO_INCREMENTAL/
RUSTFLAGS overrides. Corrected successor run-one-ordinary.py (whitelist PATH/HOME/CARGO_HOME/
RUSTUP_HOME/RUSTUP_TOOLCHAIN/LANG/LC_ALL, toolchain bin first, CARGO_TARGET_DIR=candidate/target,
TMPDIR=candidate/tmp, CARGO_BUILD_JOBS=2 scheduling only, exclusive-create receipts, PID+env+maps
per command) is the execution runner; the draft stays preserved. START.json + versions.json
recorded (cargo 1.99.0 5f94df478, rustc 1.99.0 b940084d7, clippy 0.1.99, rustfmt 1.10.0-stable).
Then: run the six commands once each in order (test → build → fmt check → fmt → default clippy →
strict clippy) in background with completion notifications, parse harness counts, prove fmt no-op,
stop on failure with exact blocker; then stage4 post-checks and seal.

COMMAND RESULTS (observed):
1. workspace-test exit0 (pid3309725, log3fa5e83964061fbe…, source map unchanged): 47 harnesses all
   ok = 9 unittest + 30 integration + 8 doctest; 733 passed / 0 failed / 2 ignored / 0 filtered /
   0 measured. Ignored = proto tests/captures.rs dump + extract_corpus (exact historical helpers).
   Store unittests65 + integrations checkpoint5/wal4 reached and ok; all 8 doctest groups completed
   (engine1, osdc1, proto1, others0). Daemon unittest harness =179 tests (matches accepted author
   full-daemon count). Four optional early-return bodies labeled from log lines503/652/654/846:
   monc capture_golden::mdsmap_for_unknown_daemon (RMDS_TEST_MDS_A_KEY missing), msgr
   kernel_capture::kernel_and_cpp_signatures_verify and ::kernel_authorizer_and_cpp_challenge
   (RMDS_TEST_MDS_ROTATING_SECRET_2 missing ×2), proto captures::kernel_v1_mdsv1_capture
   (kernel-v1-mdsv1 pcap+msgs pair missing) — same census as accepted pre-D5 runs; all four still
   report ok. Prior612/0/2 stays pre-D5 evidence; this is the new combined candidate's own result.
2. daemon-build exit0 (pid3320847, log a8ba9b0972894c04…, 174 bytes, "Finished `dev` profile
   [optimized + debuginfo] in 35.03s" — ordinary checked-in profile, no env overrides). Produced
   candidate/target/debug/ceph-rmds 138,373,888 bytes sha256
   3e8669bfcf82397078f179418efa70846d0f42a2c3d7e625537090d7e2a63d7d, recorded in
   logs/new-binary-hash.json with executed=false; it does not replace preserved old binary
   330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374 (untouched file).
3. fmt-check exit0 (pid3321329, empty log e3b0c442…, no diffs).
4. fmt-write exit0 (pid3321380, empty log) with PROVEN byte/mode no-op: source_map_changed=false,
   changed_files=0 across all1,843 candidate source files (bytes+fs_mode per-file compare).
   Check-before-mutation order respected (check passed first).
5. clippy-default exit0 (pid3321414, log bc114e44be81b2f1…, 2,532 bytes): ZERO warnings — log is
   Checking/Finished lines only; "Finished `dev` profile [optimized + debuginfo] in 22.04s".
6. clippy-strict exit0 (pid3321947, log effa3b9139d66a21…, 1,014 bytes): zero warnings/errors,
   "Finished in 13.94s".
Stage4 postcheck PASSED (stage4-postcheck.json): candidate tree (1,843 files) equals recorded map
after all checks; candidate667==frozen; primary667==frozen; fsmap equal both trees; six receipts
all exit0. Sealed (stage4-seal.py): results.json, SEAL.json, evidence archive
fbeaf71f20a452db49ad88159cf87b02778a54ea9d662eeb76d1022a6b57112f (31 members, entry-map
b39d9b1bdda3549e4bf869764d18d77e6a72fe10e63c7b5268815d7b646facdf), HANDOFF.md
7a1cba89f61a8849d03d239f3f841cc8e253e2e0b4e2ad9e4a82ec3e523856ce pinned via seal-addendum.json.
Next: None for this owner — assignment COMPLETE/PASS. Parent decides next steps (source
publication/live qualification are separate owners and separate authority).
Log:
- [2026-10-26] Ledger created first. Required readings, candidate construction, six-command execution and sealing remain pending.
- [2026-10-26] Stage1 PASSED: 16 SEAL pins + primary667/frozen/fsmap/archive all verified; authoritative extraction at frozen-source-r3.
- [2026-10-26] Stage2 PASSED: candidate 1,843 files built from base36a920d + 667 overlay; deltas accounted, no unexpected inputs.
- [2026-10-26] PROVIDER STOP: OpenCode free-usage limit hit before any command execution; no logs/target/tmp/receipts written. Preserved as-is.
- [2026-10-26] RESUMED on zai-coding-plan GLM per user steering. stage3-reverify.py PASSED (cheap hashes; checkpoint intact, nothing ran). Draft run-one.py preserved; corrected run-one-ordinary.py created (ordinary profiles, no env profile overrides). START.json + versions.json recorded. Six-command execution beginning.
- [2026-10-26] ALL SIX COMMANDS PASSED once each in order (pids 3309725/3320847/3321329/3321380/3321414/3321947; every source map unchanged). Totals: workspace test 733/0/2 across 47 harnesses; build fmt-check/no-op-fmt/clippy-default/clippy-strict all exit0, zero clippy warnings. Stage4 postcheck + seal complete. COMPLETE/PASS returned to parent.
