# astra-generator-review
Task: Independent Astra review of small completed CI fix. Work /mnt/workspace/rust-mds. Read AGENTS/current/sol-platform.md latest. Parent granted author tools/gen_features.py and generated crates/denc/src/features.rs only; author done. Sole writable current/astra-generator-review.md (ledger first). No production edits/commits/live/subagents. Other source lanes active; don't review their unfinished diffs. Target target-astra-generator-review only if focused verification needed.
Status: ACCEPT. No actionable finding. Independent provenance, reproducibility, constant equivalence, strict denc clippy, all 8 existing denc tests, formatting, and scoped whitespace checks passed.
Context: Reviewed tools/gen_features.py and crates/denc/src/features.rs against HEAD a6ae05a10bfefdcea8dae75cd29597c84a3fadcd. Both reviewed file hashes and HEAD remained stable through verification. Acceptance covers this two-file package only.
Next: Parent may integrate this accepted generator package. Combined workspace CI requires the converged source lanes and its own QA result.

## Review result

- `tools/gen_features.py:32-33` replaces one mask-emission expression with a conditional expression. It emits `1 << bit` when the incarnation mask is zero. It retains the original expression for a nonzero mask. This is a minimal generator fix. It adds no lint suppression, parser change, broad refactor, or generated public identifier.
- The entire generated diff equals exactly 46 replacements of `(1 << bit) | 0x0` with `1 << bit`. All other bytes match HEAD.
- Parsed and evaluated all 143 public constants: 71 feature bits, 71 masks, and `ALL`. Names, types (`u64`), declaration order, and values match HEAD exactly. All identifiers remain unique. All 25 nonzero-incarnation expressions and `has()` match byte-for-byte. `ALL` remains `0x3f17fffffffdffff`. The change preserves wire feature semantics and existing conventions.

## Independent provenance and regeneration

- Reference: `/home/ubuntu/ceph/src/include/ceph_features.h`.
- Ceph HEAD: `8db74a411f998b5e6a697cf541113b0f98c1bf9f`. The working header exactly matches that revision's Git blob.
- Header SHA256: `c20efb803c63d95bb83f7b94092aa5ec1f64d2a5fb7c7f1c2e394ffa31b244f0`.
- The header defines incarnation 1 as zero, incarnation 2 as bit 57, and incarnation 3 as bits 57 and 28. The existing generator table agrees.
- Ran the HEAD generator from memory against the reference header. `rustfmt --edition 2024 --emit stdout` reproduced HEAD `features.rs` byte-for-byte.
- Ran `python3 -B tools/gen_features.py /home/ubuntu/ceph/src/include/ceph_features.h /dev/stdout` twice. Both raw outputs matched. Formatting the captured source with the same rustfmt command reproduced current `features.rs` byte-for-byte. Both runs reported 71 features and 66 `ALL` names. Comparison output stayed in memory.
- Current generator SHA256: `f870a0230e9d3fcaf3562c632f5811fb91a9d0d92f164519304f2b1d33f6ef45`.
- Current generated output SHA256: `b1a79615d5d9c69b446e6b4104be70c66d0851fc165e7230c7c74501e9b3b5eb`.

## Focused checks

- `CARGO_TARGET_DIR=target-astra-generator-review cargo clippy --locked --offline -p denc --all-targets -- -D warnings`: PASS, exit 0, zero diagnostics.
- `CARGO_TARGET_DIR=target-astra-generator-review cargo test --locked --offline -p denc`: PASS, 8 tests, 0 failed, 0 ignored, 0 doctests.
- `rustfmt --check --edition 2024 crates/denc/src/features.rs`: PASS.
- `git diff --check HEAD -- tools/gen_features.py crates/denc/src/features.rs current/astra-generator-review.md`: PASS.
- Toolchain: rustc 1.99.0, rustfmt 1.10.0-stable, clippy 0.1.99; revision `b940084d7`, 2026-09-28.
- The author's before-fix 46-error strict-clippy run and 8-test baseline remain author evidence in `current/sol-platform.md`. This review independently confirmed all 46 HEAD expressions and ran the current focused gates.
- Reviewer edits: this ledger only. Cargo artifacts use the assigned target directory. No production edits, commits, live operations, subagents, persistent mirror tests, or workspace CI claim.

Log:
- [2026-10-25] Created the review ledger before other edits.
- [2026-10-25] Completed source inspection and defined independent checks. The existing inc table matches CEPH_FEATURE_INCARNATION_1/2/3. The new conditional removes only OR zero; nonzero expressions retain their original emission.
- [2026-10-25 18:59 UTC] Completed independent source/provenance comparisons and focused verification. Final source hashes and HEAD match the reviewed snapshot. Verdict: ACCEPT.
