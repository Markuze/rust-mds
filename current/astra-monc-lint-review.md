# astra-monc-lint-review
Task: Independently review the completed private CommandWaiter alias strict-Clippy fix in crates/monc/src/monclient.rs against HEAD. Verify exact type, visibility, lifetime and Send equivalence, source scope, retained evidence hashes, and focused verification. Return ACCEPT or a precise finding with actual hashes and commands. Own only this new ledger. No production/test edits, live operations, Git writes, commits, or subagents.
Status: ACCEPT. No actionable finding. Exact type/source equivalence, all retained artifact hashes, and all 69 focused input hashes verified. Independent strict monc Clippy, formatting, and scoped whitespace checks passed. Accepted the retained 24-test result through exact input-hash proof.
Context: Reviewed only crates/monc/src/monclient.rs against HEAD 85a37c8d615992b4600d7467fdedd514fecfac03. Evidence: /tmp/opencode/sol-monc-type-20261025-iwjpmgbv/. All 69 focused inputs and HEAD remained stable through independent checks. Cargo artifacts use target-astra-monc-lint-review.
Next: Parent/release owner may include this accepted fix in the next candidate. Combined workspace CI still requires its own result after the other source lanes converge.

## Source review

- `monclient.rs:107` adds the private alias `type CommandWaiter = oneshot::Sender<Result<(i32, String), String>>;` beside the existing `VersionWaiter` alias.
- `monclient.rs:445` uses it in the private field `commands: HashMap<u64, CommandWaiter>`. The map key remains `u64`. The result payload remains the ordered tuple `(i32, String)` with a `String` error.
- Removing the alias declaration and expanding this sole use reproduces HEAD byte-for-byte. The retained before snapshot exactly matches HEAD. The retained after snapshot exactly matches current source. There are exactly two `CommandWaiter` occurrences.
- Both type spellings resolve in the same module through the same `tokio::sync::oneshot` import and standard `Result`/`String` names. The alias introduces no new type identity, generic parameter, reference, or lifetime. Visibility, layout, mutability, ownership, drop behavior, and auto-traits such as `Send` remain identical.
- The complete diff changes no executable statement, public signature, wire encoding, authentication, fencing, async scheduling, or channel operation. It adds no lint suppression or configuration change. This is a minimal fix that follows the adjacent alias convention.

## Retained evidence verification

- Artifact manifest: `artifact-checksums.json`, independently verified SHA256 `8871a4b3880ca82410e6fdd4c0f4a4d7d4c4147fa445072a09bc5cdd6d9cfd2c`.
- All 16 listed artifacts match their recorded SHA256, byte length, and filesystem mode. The manifest covers every other file in the evidence directory.
- Before input manifest SHA256: `f6e6531b826ea20b42cfc83c2ade92b4246efb86f5b53249ba0e77e56609130f`.
- After input manifest SHA256: `3837262369bdf137ac7e07e97cffc6c581c261a5a4bcc54a403568ffe1790029`.
- Both manifests cover the same 69 tracked files: root Cargo.toml/lock and crates/monc, auth, msgr, and denc. Only monclient.rs differs between them. Every current file matches the after hash, length, and mode. Git modes and the scoped tracked-file inventory also match.
- Source SHA256 before: `d9eb93742c104ac82e864a7544627ccd6ebcb086d905d8e91ed5708405fe80ca`.
- Source SHA256 after: `3d6bc14b63f406dd45e1bdf297e68c4305d6f7ffbe5289cef1f3baa7d17fb876`.
- Source mode stays filesystem `0664`, Git `100644`.
- Retained before/after command: `CARGO_TARGET_DIR=target-sol-platform CARGO_NET_OFFLINE=true cargo clippy -p monc --all-targets --no-deps --locked -- -D warnings`. Before exits 101 with the sole type_complexity diagnostic at line 444, also reported for lib-test compilation. After exits 0 with zero diagnostics.
- Retained test command: `CARGO_TARGET_DIR=target-sol-platform CARGO_NET_OFFLINE=true cargo test -p monc --locked`. The exit is 0. Independently recounted 15 unit + 7 capture + 1 fake-monitor + 1 version-wire = 24 harness tests passed, 0 failed/ignored/filtered, and 0 doctests. Exact input hashes justify reuse. Optional-key capture behavior retains the existing test scope.

## Independent commands

- `CARGO_TARGET_DIR=target-astra-monc-lint-review CARGO_NET_OFFLINE=true cargo clippy -p monc --all-targets --no-deps --locked -- -D warnings`: PASS, exit 0, zero diagnostics.
- `rustfmt --check --edition 2024 crates/monc/src/monclient.rs`: PASS, exit 0.
- `git diff --check HEAD -- crates/monc/src/monclient.rs current/astra-monc-lint-review.md`: PASS, exit 0.
- Read-only Python comparisons verified artifact/input hashes, byte lengths, modes, retained log counts, and normalized source equality against `git show HEAD:crates/monc/src/monclient.rs`. Rechecked all 69 focused inputs after Clippy.
- Toolchain: rustc 1.99.0, Clippy 0.1.99, rustfmt 1.10.0-stable, revision b940084d7, 2026-09-28.
- Reviewer edits: this new ledger only. No production/test edits, live operations, Git writes, commits, subagents, or new mirror tests. The prior generator report remains intact. This acceptance makes no workspace CI claim; daemon lint and active D6/store work remain outside this review.

Log:
- [2026-10-25] Created the new review ledger before other edits. The prior generator report is a separate completed review.
- [2026-10-25] Inspected the two-hunk source diff and its module/Task context. The alias expands to oneshot::Sender<Result<(i32, String), String>> in the same module scope. It introduces no lifetime parameters or wrapper type.
- [2026-10-25 21:57 UTC] Verified all 16 artifacts, all 69 focused inputs, and exact before/after source identity. Reused the verified 24-test evidence. Independent strict monc Clippy/fmt/whitespace checks passed. Final focused hashes and HEAD remained stable. Verdict: ACCEPT.
