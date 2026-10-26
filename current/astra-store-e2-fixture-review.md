# astra-store-e2-fixture-review

Task: Independently review the tiny E2 store fixture migration in primary `/mnt/workspace/rust-mds`. Read `current/sol-store-e2-fixture.md` and `/tmp/opencode/sol-store-e2-fixture-20261025.a2_q7li2/`. Check the actual `crates/store/src/stable.rs` diff, accepted E2 contract, real engine recovery path, session keys, final chunks, empty-cut permit handling and preserved storage assertions. Own only this new report. No production/test edits, live actions, Git writes, commits or subagents. Use `target-astra-store-e2-fixture-review`. Record the promotion delta for later isolated E1 integration.
Status: ACCEPT for the primary E2 store fixture migration. No blocking finding. The independent original test passed. The retained author run passed all 50 store tests and the empty doctest harness. All 55 captured inputs remained unchanged during independent execution. The earlier G2 report remained unchanged.
Context: Reviewed `crates/store/src/stable.rs` at SHA256 `7b5fcd89da1baaec3eb3eed4b71cfa89be266befadd6f78abc21392ce7ac5304` against HEAD/retained-before source and accepted recovery-contract section 2. Primary D6 R1 work changes another crate. Isolated E1 engine/store work is separate. Reviewer edited only this report.
Next: Coordinator can use this scoped acceptance for integration. The later E1 integration owner must reconcile the exact two-hunk fixture delta below and verify the resulting integrated source.
Log:
- [2026-10-25] Created this ledger before review or other edits.
- [2026-10-25] Matched retained before source to HEAD and current source to retained after source. Verified the 40000-byte production prefix, every byte outside the existing test, and the original request/storage assertion suffix are unchanged. Verified all 19 artifact checksums and all 55 input entries; only stable.rs changed across the author run. Traced checked restore, final chunk, full survivor-key seal and admitted marker execution against recovery-contract section 2.
- [2026-10-25] Independent artifacts: `/tmp/opencode/astra-store-e2-fixture-review-20261025.9f6htqhg/`. Started only the original exact test with locked/offline Cargo resolution and fresh `target-astra-store-e2-fixture-review`.
- [2026-10-25 22:03 UTC] Independent focused execution completed: exit 0, 1 passed, 0 failed/ignored. Rehashed all 55 captured inputs and their path set after execution. They still match the author final manifest. Reverified all 19 author artifacts. Confirmed the earlier G2 report hash is unchanged. Issued scoped ACCEPT and recorded the required E1 promotion delta.

## Verdict and source identity

**ACCEPT.** The fixture executes both accepted E2 recovery boundaries before its original request. It uses the actual surviving session and the real admitted engine marker. It preserves the complete original storage proof.

Primary repository: `/mnt/workspace/rust-mds`.

| Item | Exact identity |
|---|---|
| HEAD used for the before-source comparison | `85a37c8d615992b4600d7467fdedd514fecfac03` |
| Before `crates/store/src/stable.rs` | 108344 bytes; SHA256 `8d7d21d5d291a309d1a6d1572a9fdc10c27a92cd386911cc0c85a8ffa56d433e` |
| Accepted `crates/store/src/stable.rs` | 109661 bytes; SHA256 `7b5fcd89da1baaec3eb3eed4b71cfa89be266befadd6f78abc21392ce7ac5304` |
| Unchanged production prefix | 40000 bytes before `#[cfg(test)]` at line 1190; SHA256 `19ad63a129f3e76e386738e7ea039ceea6d7a9ed39f639b24d17b7782ff23313` |
| Exact `minimal.diff` | SHA256 `2ca84ee06c0172c51aa9775e0ac5658bc360c92f028f7d95baa91832127599a1` |
| Author final input manifest; independent before/after manifests | SHA256 `2bbbeeb671cc283ba7b7d6002d391612d0455d05af6ff93142e0d2794988d47d` |

The retained before source matches `HEAD:crates/store/src/stable.rs` byte-for-byte. The current source matches retained `stable.after.rs` byte-for-byte. Independent diff generation reproduces the retained two hunks exactly: local imports plus 30 recovery-setup lines, 31 insertions/1 deletion in total. Every byte outside the existing test is unchanged. Within the test, every byte from the original `let effects = engine.handle_request(` through the test end is unchanged.

The author manifests contain the same 55 paths: engine/store Rust inputs, both crate manifests, the workspace manifest and `Cargo.lock`. Only `crates/store/src/stable.rs` differs between author before/after manifests. The other 54 inputs are unchanged. All 55 current inputs match the author final manifest before and after independent execution. Their path set is also unchanged.

## Recovery-path review

1. **Checked restore and actual survivor.** The fixture obtains `key` from `SessionRepository::open_session_record(new_session(10))` at `crates/store/src/stable.rs:2198`. It restores the bootstrap checkpoint and installs that same key. `Engine::restore` in `crates/engine/src/engine/durable.rs` starts in Reconstruction. The bootstrap fixture has no other session.
2. **Validated final chunk.** At line 2213, `reconnect_chunk(key, &[], &[], false)` uses the installed incarnation. The session has no caps or realms, so the empty final chunk is valid. The test asserts `.complete`. The implementation in `crates/engine/src/engine/reconnect.rs` validates the session and records final-chunk completion.
3. **Structural boundary.** At line 2217, `finish_reconnect(&[key])` seals the actual surviving-key set. An empty key list would be wrong for this fixture. The implementation checks the complete survivor set and final chunks, then enters Control drain. The fixture asserts Idle, empty effects/notices and no unused permit. This boundary alone does not release grants.
4. **Admitted grant-release boundary.** At lines 2224–2229, the fixture calls `Engine::step` with `EngineInput::RecoveryControlsDrained` and `Some(RecordPermit { max_record_bytes })`. The bound comes from the store's default record limit. The marker passes through the admission and recovery checks in `crates/engine/src/engine/step.rs`. It reaches the real transition to Grants released. The fixture does not write a readiness flag or call legacy `reconnect_done`.
5. **Empty journal cut and permit return.** The marker has no recovery output or work in this fixture. The engine returns the supplied permit through `unused_permit`. Lines 2230–2240 assert Idle, empty effects/notices, the returned permit, quiescence, an empty journal, equal submitted/durable sequences and the original checkpoint sequence. No append or journal completion is injected to release the gate.
6. **Original request still checks release.** The unchanged `handle_request` calls the engine request path. `dispatch_new` in `crates/engine/src/engine/mod.rs` checks `grants_released`. The unchanged successful-reply assertion therefore exercises the released gate. The retained baseline fails at that same assertion when this recovery setup is absent.

These checks match `design/recovery-contract.md` section 2, including the distinct structural and grant-release boundaries and the admitted marker's unused-permit path.

## Original storage proof

The byte-identical suffix still performs all of the following:

- Create `retained` with mode `0o640` and xattr `user.persist=yes`; require a successful reply.
- Capture the materialized checkpoint and encode/append the engine's journal records.
- Write and publish the checkpoint, trim the write-ahead log (WAL), and assert that every WAL key is absent.
- Acquire replacement shard and rank-session authority with the retained fencing evidence.
- Load the checkpoint with empty replay and compare the complete recovered checkpoint.
- Restore the recovered engine, install its persisted session key, compare the checkpoint, verify the directory linkage and assert an empty restored journal.

The final restored engine is inspected only. It does not admit another request, so that comparison needs no additional grant-release marker. No original assertion was removed, weakened or replaced.

## Verification and retained evidence

Independent evidence: `/tmp/opencode/astra-store-e2-fixture-review-20261025.9f6htqhg/`.

Author evidence: `/tmp/opencode/sol-store-e2-fixture-20261025.a2_q7li2/`.

Fresh independent command:

```sh
CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-astra-store-e2-fixture-review cargo test -p store --locked --offline stable::tests::native_store_shape_restores_materialized_engine_after_every_wal_key_is_removed -- --exact --nocapture
```

| Evidence | Result |
|---|---|
| Independent `original-test.log` and `original-test.command.json` | Exit 0. 1 passed, 0 failed/ignored; 40 other unit tests and 9 integration tests filtered. The assigned target was absent before this execution. |
| Verified author `original-test.before.log` and command JSON | Exit 101 at the original reply-success assertion, then line 2225. It matches the release failure in `/tmp/opencode/sol-release-e2-d6-20261025.bl5z3why/logs/workspace-test.log:1145`. |
| Verified author `original-test.final.log` and command JSON | Final source: exit 0, 1 passed. |
| Verified author `store-full.after.log` and command JSON | Exit 0. 41 unit + 5 checkpoint + 4 WAL tests passed. 0 failed/ignored. Doctest harness passed with 0 cases. This is retained author evidence, not an independent full-suite rerun. |
| Verified author `rustfmt-check.log` and command JSON | `rustfmt --edition 2024 --check crates/store/src/stable.rs`: exit 0. |
| Independent `provenance.json`, `verified-minimal.diff`, `inputs.before.json`, `inputs.after.json`, `post-test-stability.json` | Exact diff and source comparison; all 55 captured inputs and the earlier G2 report stable across independent execution. |

The focused test is one of the 50 store tests. These counts are not additive. The author's intermediate `original-test.after.log` predates two formatting adjustments; the final targeted log and full-suite log supply the final-source evidence.

All 19 files in the author's `artifact-checksums.json` were checksum-verified and reverified after independent execution. That manifest has SHA256 `e4c3801c4bb06e8980383667607ca7430dff6ba7047a7660917530bb1ef68e7e`.

The independent `artifact-checksums.json` binds seven evidence files. Its SHA256 is `5d0b729c1cbb75dc2920a88819720df924059a0f61917582e12eebf2298abce6`.

## Required delta for later E1 promotion

The accepted primary fixture delta is the retained `minimal.diff` at SHA256 `2ca84ee06c0172c51aa9775e0ac5658bc360c92f028f7d95baa91832127599a1`. It changes only the local imports at line 2183 and recovery setup at lines 2211–2240.

The later E1 integration owner must reconcile these two hunks with the E1 fixture/API changes. Do not replace primary `crates/store/src/stable.rs` wholesale with the isolated snapshot. Preserve:

- The repository-issued SessionKey and its validated final reconnect chunk.
- Structural completion with the actual surviving-key set.
- The distinct `RecoveryControlsDrained` boundary through admitted `Engine::step`.
- The empty-cut unused-permit, no-record, quiescence and sequence assertions.
- Every original request and storage assertion listed above.

After reconciliation, verify the original targeted test and the store suite against the resulting integrated inputs. This review's exact-source evidence applies to the captured primary inputs.

## Scope record

Reviewer edited only `current/astra-store-e2-fixture-review.md`. Evidence files and assigned build outputs contain the independent verification. `current/astra-g2-fault-matrix.md` retains its pre-assignment SHA256 `858c80cc6b24682c3c6efa886974cab25fe6aedead78f3d0844e38593f0646cd`. The earlier G2 verdict remains unchanged. No production/test edits, live actions, Git writes, commits, subagents or broad workspace test run occurred in this assignment.
