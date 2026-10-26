# sol-store-e2-fixture
Task: Migrate the primary stable-store test fixture to the accepted E2 recovery gates. Own only cfg(test) fixture sections/imports in crates/store/src/stable.rs and this ledger. Reproduce the existing failure, use finish_reconnect then an admitted RecoveryControlsDrained step, verify the original test and full store tests/doctests, and preserve every production byte. Do not edit the isolated E1 tree or run live/Git-write/workspace operations.
Status: Fixture correction complete; independent review remains required. The original test passes. All 50 existing store tests pass (41 unit, 5 checkpoint integration, 4 WAL integration); the doctest harness passes with 0 cases. Scoped rustfmt/diff checks pass. Production bytes and every other captured engine/store input are unchanged.
Context: Primary /mnt/workspace/rust-mds. Use CARGO_TARGET_DIR=target-sol-store-e2-fixture. Read the required guidance and accepted recovery-contract section 2. The old rados ownership is not active. Independent review is required before any commit by the release owner. Evidence root: /tmp/opencode/sol-store-e2-fixture-20261025.a2_q7li2.
Next: Coordinator assigns independent review. Release owner must reconcile only the retained minimal fixture delta during any later isolated-E1 promotion. Do not overwrite this delta with the isolated baseline. No commit, promotion, workspace rerun or live action is authorized by this report.

## Baseline

- Original test: `stable::tests::native_store_shape_restores_materialized_engine_after_every_wal_key_is_removed`.
- Release failure: `/tmp/opencode/sol-release-e2-d6-20261025.bl5z3why/logs/workspace-test.log`, lines 1145–1175.
- Primary/release-candidate stable.rs are byte-identical: SHA256 `8d7d21d5d291a309d1a6d1572a9fdc10c27a92cd386911cc0c85a8ffa56d433e`, 108344 bytes.
- Production prefix: all 40000 bytes before `#[cfg(test)]` at line 1190. SHA256 `19ad63a129f3e76e386738e7ea039ceea6d7a9ed39f639b24d17b7782ff23313`.
- Saved `stable.before.rs`, `baseline.json`, and `inputs.before.json` (all primary engine/store Rust inputs, both crate manifests, workspace manifest and Cargo.lock).
- Immediate-coupling audit: this module has three checked restores. Only the first restore in the original failing test requests work. The other two only compare restored checkpoint/materialized state; they need no grant release. No additional fixture fix is justified.

## Correction and current source proof

- Only the existing test at `crates/store/src/stable.rs:2181` changed. Its local imports at line 2183 add `EngineInput`, `RecordPermit`, and `StepState`. No production API changed.
- Validate the installed key's final empty chunk. Call `finish_reconnect(&[key])` at line 2217. Assert Idle, empty effects/notices and no permit.
- Execute `EngineInput::RecoveryControlsDrained` through `step` with a fresh `RecordPermit` at lines 2224–2230. Assert Idle, empty effects/notices, the returned unused permit, engine quiescence, an empty journal and unchanged submitted/durable sequence. No append or journal callback occurs. The original successful reply, checkpoint, WAL trim and restored-state assertions remain intact.
- Final stable.rs SHA256: `7b5fcd89da1baaec3eb3eed4b71cfa89be266befadd6f78abc21392ce7ac5304`, 109661 bytes.
- Production prefix remains byte-identical: 40000 bytes, SHA256 `19ad63a129f3e76e386738e7ea039ceea6d7a9ed39f639b24d17b7782ff23313`.
- The byte comparisons also cover every byte before/after this single test, including all other fixtures. All 54 other captured input paths remain unchanged; no engine/store source path was added or removed.
- Fresh manifests: `inputs.before.json` and `inputs.after.json`, 55 primary engine/store Rust/manifest/lock paths. `source-verification.json` records byte comparisons, file hashes and line locations. `stable.before.rs` and `stable.after.rs` retain exact bytes.
- Input-manifest SHA256: before `d7f021a1da71a428745da64d412c42acbaf3c38b6bc21222bcd6c59dafea4124`; after `2bbbeeb671cc283ba7b7d6002d391612d0455d05af6ff93142e0d2794988d47d`. `artifact-checksums.json` binds all 19 evidence files; its SHA256 is `e4c3801c4bb06e8980383667607ca7430dff6ba7047a7660917530bb1ef68e7e`.
- Retained `minimal.diff` has only two fixture hunks (local imports plus recovery setup), 31 insertions/1 deletion. Its SHA256 is `2ca84ee06c0172c51aa9775e0ac5658bc360c92f028f7d95baa91832127599a1`.
- The isolated E1 tree was not inspected, copied, tested or changed. No production store code, other crate source, manifest, lockfile, Git state or live resource was edited. No workspace command, formatter write or subagent ran.

## Final verification

All Cargo commands used `CARGO_TARGET_DIR=target-sol-store-e2-fixture` in the primary tree with locked, offline resolution.

| Command | Result | Evidence |
|---|---|---|
| `cargo test -p store --locked --offline stable::tests::native_store_shape_restores_materialized_engine_after_every_wal_key_is_removed -- --exact --nocapture` | Before: exit 101, original assertion failure. Final: exit 0, 1 passed; other tests filtered. | `original-test.before.log`, `original-test.final.log`, corresponding command JSON files |
| `cargo test -p store --locked --offline -- --nocapture` | Exit 0: 41 unit + 5 checkpoint + 4 WAL tests passed; 0 failed/ignored/filtered. Doctest harness: 0 cases. | `store-full.after.log`, `store-full.after.command.json` |
| `rustfmt --edition 2024 --check crates/store/src/stable.rs` | Exit 0. | `rustfmt-check.log`, command JSON |
| `git diff --check -- crates/store/src/stable.rs current/sol-store-e2-fixture.md` | Exit 0. Read-only check. | `diff-check.log`, command JSON |

The earlier `original-test.after.log` records an intermediate passing fixture before two inserted-expression formatting adjustments. Use the final targeted log and full-suite log for the final source hash above. No new test or mirror test was added.
Log:
- [2026-10-25] Created the exclusive fixture ledger before source edits.
- [2026-10-25] Original targeted command: CARGO_TARGET_DIR=target-sol-store-e2-fixture cargo test -p store --locked --offline stable::tests::native_store_shape_restores_materialized_engine_after_every_wal_key_is_removed -- --exact --nocapture. Exit 101, 0 passed/1 failed/40 filtered, unchanged reply-success assertion at line 2225. Retained original-test.before.log and original-test.before.command.json. No source edit preceded reproduction.
- [2026-10-25] Migrated only the existing fixture through validated final chunk, structural completion and admitted control-drain marker. The original test passed. A scoped rustfmt check identified two newly inserted expression layouts; applied only those fixture adjustments with patch.
- [2026-10-25] Final full store suite passed all 50 existing tests and the empty doctest harness. Repeated the original targeted test on the final bytes: 1 passed. Scoped rustfmt/diff checks passed. Captured final 55-input manifest, exact source snapshots and minimal delta; verified production and all other captured inputs byte-identical. Await independent review; no commit/live action.
