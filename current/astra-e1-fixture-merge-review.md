# astra-e1-fixture-merge-review

Task: Independently review the accepted primary E2 store fixture reconciliation with isolated E1. Own only this new primary report and external review artifacts under `/tmp/opencode`. Review the actual three-way diff, assertion mapping, durable session ordering, recovery boundaries, Legacy2-to-Ready3 adoption, production/unrelated-test byte equality, source/mode/hash provenance and retained verification. Run one independent affected fixture on the frozen external candidate with a new absolute target outside that candidate. Preserve prior reports and all source. Record the exact scoped verdict and later promotion recipe.
Status: COMPLETE — SCOPED ACCEPT for fixture SOURCE reconciliation and the pinned R1 focused test. No blocking finding. All 20 accepted primary assertions match byte-for-byte and in order. The independent affected fixture passed once. The 651-file candidate, 649 primary inputs, all 698 author artifact files and monitored reports/contracts remained unchanged. Final R2 dependency-join evidence awaits new pins.
Context: Author artifact root is `/tmp/opencode/sol-e1-fixture-merge-20261026.lz4yvr6q/`. The frozen external candidate and saved R1 evidence are authoritative for this review. The original isolated engine has authorized concurrent wake-fix work. This review covers fixture source reconciliation and a pinned R1 focused test; the final R2 dependency join awaits new pins.
Next: Coordinator can use this fixture-source acceptance. After R2 pins and an explicit integration assignment exist, reconcile the exact two-hunk patch with the authorized E1 source base and obtain fresh joined verification. Preserve the prior primary fixture and G2 reports.
Log:
- [2026-10-26] Created the exclusive review ledger before review or other edits.
- [2026-10-26] Verified all 698 author artifact entries and all 651 candidate inputs, including saved corrected-engine and store manifest joins. Only stable.rs differs from the saved unmerged map. Independent evidence: `/tmp/opencode/astra-e1-fixture-merge-review-20261026.trb_1gdy/`.
- [2026-10-26] Reproduced all three diffs. Exact context application to the saved E1 base reconstructs the proposed full file. Confirmed 29 insertions/8 deletions, unchanged E1 production/unrelated tests and all 20 accepted primary assertions. The patch does not apply to the accepted primary source.
- [2026-10-26] Ran the affected fixture once from the immutable external candidate with a fresh absolute target under the independent evidence root. Exit 0: 1 passed, 0 failed/ignored; 64 other unit and 9 integration tests filtered. Full before/after input, mode, configuration, command, toolchain, log and output-hash evidence is retained. The original isolated engine was not read or checked.
- [2026-10-26 01:00 UTC] Completed the actual recovery/adoption path review and retained-log checks. Reverified the old-scope 74-test author log and its saved corrected-manifest input arrays. Final byte/mode/inventory checks passed. Issued fixture-source and pinned-R1 focused-test SCOPED ACCEPT. The evidence-audit script's initial workspace-member wildcard assumption was corrected; no Cargo test failed or was repeated.

## 1. Verdict and scope

**SCOPED ACCEPT — fixture merge.** The proposed delta restores the accepted primary recovery-gather and assertion coverage within E1's actual adoption fixture. It preserves E1 production code, all unrelated tests and the existing request/storage sequence. No ranked source finding remains in this assignment.

The unmerged E1 fixture already passed. It finished an empty engine gather, discarded the marker result and installed the durable rank key afterward. The merge replaces that setup with the accepted key-before-reconnect sequence and 13 recovery assertions. It retains the seven existing storage assertions. This is assertion/provenance reconciliation, not a new production bug or a red-to-green test claim.

The verdict applies to the frozen external candidate and its R1 engine dependency. The original isolated engine has authorized concurrent wake-fix work. I did not read that engine or assert that its current input set remained unchanged. The final R2 dependency join awaits new pins. This report does not grant full E1 acceptance or promotion.

## 2. Exact source and provenance

Author root: `/tmp/opencode/sol-e1-fixture-merge-20261026.lz4yvr6q/`.

Frozen tested candidate: `/tmp/opencode/sol-e1-fixture-merge-20261026.lz4yvr6q/candidate/`.

Independent evidence: `/tmp/opencode/astra-e1-fixture-merge-review-20261026.trb_1gdy/`.

| Input | Verified SHA256 |
|---|---|
| Common `stable.rs`, published baseline `85a37c8d615992b4600d7467fdedd514fecfac03` | `8d7d21d5d291a309d1a6d1572a9fdc10c27a92cd386911cc0c85a8ffa56d433e` |
| Accepted primary `stable.rs` | `7b5fcd89da1baaec3eb3eed4b71cfa89be266befadd6f78abc21392ce7ac5304` |
| Saved unmerged E1 `stable.rs` — patch base | `d57e49ac1f3239bc79cee3f1d89e24752449ba87c0b97238f1e968eb26025972` |
| Proposed full `stable.rs`, 121094 bytes, normalized mode `100644` | `2cb747b0f997dd87cfbe0c7720f7c98e9010573561682589c1210906a5b011d6` |
| `comparison/proposed-fixture.diff`, 2297 bytes | `7e6540e5dadcbd80139e3fbd40cd6162b86dd30ad6978030f3cb23c655c88554` |
| Saved corrected engine manifest | `7bfce91702deee435a84d3c66c6cb9c65534d99d595a51b852de487da93c7382` |
| Ordered 46-engine input array | `0a9a6a43b08ab06214a6705bc39a6b19be8126c4ffad09122a35fd2956b75f5a` |
| Saved base store manifest | `badbc7e2a8adf3d0f39df597ebc9a82300fa8e44ff9108a8065999235abb8c11` |
| Unmerged 651-file relative input map | `198edd8317fa4d2b367105f555da6f03b522303fb5415a59b3ca219cf9d4a1b0` |
| Merged 651-file relative input map | `282887d612d7f6887e63bad5022be36b82fe62b4d6c707cfe551e23fe4367232` |
| Author `ARTIFACT-SHA256.json` | `c2a7a41e9b249fb023d1dfef954e9812631a151b9b0d0bb7bf9d547247cfc31f` |

The input-map hashes use sorted compact JSON from relative paths to `{bytes,mode,sha256}`. The ordered engine-array hash retains the saved manifest's absolute path labels; those labels were mapped to the external candidate for byte checks. They were not used to read the in-flight original engine.

Verified all 698 author artifact entries and the complete non-build artifact inventory. Verified all 651 actual candidate paths, bytes and normalized modes. All 46 engine entries match the saved corrected engine manifest. All 18 unmerged store entries match the saved store manifest. The only difference between the unmerged and merged maps is `crates/store/src/stable.rs`; the other 650 inputs are identical.

The saved common and primary sources match the earlier primary author's retained `stable.before.rs` and `stable.after.rs`. Current primary `stable.rs` still matches the accepted primary bytes. The reproduced common-to-primary diff equals the earlier accepted `minimal.diff`, SHA256 `2ca84ee06c0172c51aa9775e0ac5658bc360c92f028f7d95baa91832127599a1`. No fresh Git/publication claim is needed for these byte comparisons.

## 3. Actual three-way reconciliation

Independently regenerated and matched all three retained diffs:

- Common baseline to accepted primary.
- Common baseline to unmerged E1.
- Unmerged E1 to the proposed merge.

The incremental patch contains exactly **two cfg(test) hunks, 29 insertions and 8 deletions**. The first changes this test's local imports. The second replaces its recovery setup. Exact line/context application to the saved E1 base reconstructs the proposed full file without offsets or fuzz. The same exact application rejects accepted primary as the wrong base.

The complete local imports and recovery block from `engine.install_session_key(key)` through the final pre-request sequence assertion are byte-identical to accepted primary. The E1 setup through checked restore is byte-identical to unmerged E1. The entire request/storage suffix is also byte-identical to unmerged E1. Its adopted owner and checkpoint generations remain intact.

Every byte outside this one test is unchanged from E1. This includes all unrelated tests and the **43290-byte production prefix** before `#[cfg(test)]` at line 1259, SHA256 `656779f54cde4eee0a6d9750c69402e25fec4d6b9823b829dde08cf488aa2782`. The common-to-primary comparison separately preserves the earlier 40000-byte production prefix.

Evidence: independent `audit_merge.py`, `verified-common-to-primary.diff`, `verified-common-to-e1.diff`, `verified-proposed-fixture.diff` and `three-way-assertion-proof.json`.

## 4. All accepted assertions preserved

I extracted each actual assertion statement, including its whitespace, from the accepted primary and merged tests. All 20 statements match byte-for-byte in the same order. I also checked the author's individual line mapping. The independent JSON proof retains each complete statement and its SHA256; this decision does not rely on the count alone.

Line numbers below refer to `crates/store/src/stable.rs` in each saved version.

| # | Primary line | Merged line | Assertion preserved |
|---|---:|---:|---|
| 1 | 2211 | 2493 | The installed key's final empty reconnect chunk completes. |
| 2 | 2218 | 2500 | Structural result is `Idle`. |
| 3 | 2219 | 2501 | Structural effects are empty. |
| 4 | 2220 | 2502 | Structural notices are empty. |
| 5 | 2221 | 2503 | Structural result has no unused permit. |
| 6 | 2230 | 2512 | Marker result is `Idle`. |
| 7 | 2231 | 2513 | Marker effects are empty. |
| 8 | 2232 | 2514 | Marker notices are empty. |
| 9 | 2233 | 2515 | Marker returns `Some(RecordPermit { max_record_bytes })`. |
| 10 | 2237 | 2519 | Engine is quiescent. |
| 11 | 2238 | 2520 | Engine journal has no records before the request. |
| 12 | 2239 | 2521 | Submitted and durable sequences are equal. |
| 13 | 2240 | 2522 | Durable sequence equals original seed checkpoint coverage. |
| 14 | 2255 | 2537 | Original create returns a successful reply. |
| 15 | 2278 | 2559 | Every write-ahead log (WAL) key is removed. |
| 16 | 2299 | 2580 | Recovered replay records are empty. |
| 17 | 2300 | 2581 | Recovered checkpoint equals the complete saved checkpoint. |
| 18 | 2310 | 2591 | Restored engine exports the same checkpoint. |
| 19 | 2311 | 2592 | The restored `retained` directory entry exists. |
| 20 | 2312 | 2593 | Restored journal has no records. |

## 5. Real session, adoption and recovery paths

All source references in this section are relative to the frozen external candidate.

### E1 adoption stays real

`crates/store/src/stable.rs:2145–2159` still creates the explicit Legacy2 seed. The affected test publishes that legacy Ready state, increments owner epoch/incarnation/GID, and calls `acquire_all` before adoption (`:2451–2474`). The unchanged helper at `:2162–2208` separately acquires format, WAL and rank-session authority with predecessor evidence.

The fixture calls the actual `upgrade_retry_targets` implementation. `crates/store/src/upgrade.rs:279–349` checks the acquired legacy Ready state, all three predecessor authorities and the clean source, then writes and verifies the new checkpoint. Lines 399–479 promote session/WAL semantics and complete Ready. The clean-source comparison at `:620–630` preserves the full source image and sequence while changing only recovery semantics. `StableStore::load` at `stable.rs:423–463` requires semantics 3 for the authority, checkpoint and uncovered records.

Thus the restored `root_state` comes from the actual Legacy2-to-later-owner Ready3 adoption. The fixture does not substitute a Ready3 seed or a fabricated clean checkpoint. It keeps adoption generation 2, normal publication generation 3, `current_owner` for storage mutations and the subsequent owner takeover. The legacy/clientless inspection tests, clean-boundary refusals, actual adoption/fault tests and `upgrade_tests.rs` remain byte-identical.

### The nonempty Open gather is exercised

The fixture obtains `key` from `SessionRepository::open_session_record(new_session(10))` at `crates/store/src/stable.rs:2480–2485`. That method requires semantics 3, stages an Open row with the repository incarnation and completes the guarded mutation (`crates/store/src/sessions.rs:313–386`). The engine receives the returned key, not a constructed substitute.

Checked restore enters Reconstruction (`crates/engine/src/engine/durable.rs:107–125`). The fixture installs the key at `crates/store/src/stable.rs:2492`, before the final chunk at `:2495` and `finish_reconnect(&[key])` at `:2499`. The clean adopted root has no other session, caps or jobs. The actual installed session makes the gather nonempty even though its cap/realm lists are empty.

`crates/engine/src/engine/reconnect.rs:25–96` checks the exact incarnation and records the final chunk. `finish_reconnect` at `:138–189` requires the full surviving-key set, completed chunks and a committed empty cut before entering Control drain. The test contains exactly one finish and one marker. It no longer uses `finish_reconnect(&[])` before installing the session.

### The marker is admitted and returns its permit

At `stable.rs:2505–2511`, `max_record_bytes` comes from `Limits::default()`. The fixture submits `RecoveryControlsDrained` with a real `RecordPermit` through `Engine::step`.

The pinned engine checks semantics/recovery state and earlier work (`crates/engine/src/engine/step.rs:129–209`), retains admission ownership and requires the permit (`:339–359`), then executes the grant-release transition (`:391–395`). The unused permit comes from `self.step_permit.take()` (`:424–426`). The assertions prove empty effects/notices, quiescence, no extra journal record and unchanged submitted/durable/seed sequence. No append or journal callback releases this gate.

The original `handle_request` still reaches the `grants_released` check in `crates/engine/src/engine/mod.rs:553–575`. Its successful reply therefore follows the real released gate. The final restored engine is only inspected; it admits no later request. These paths match recovery-contract section 2 and preserve the accepted primary storage proof.

## 6. Verification and evidence scope

One independent affected-fixture run, UTC **00:54:46–00:56:01 on 2026-10-26**:

```sh
CARGO_TARGET_DIR=/tmp/opencode/astra-e1-fixture-merge-review-20261026.trb_1gdy/target-astra-e1-fixture-merge-review cargo test -p store --locked --offline stable::tests::native_store_shape_restores_materialized_engine_after_every_wal_key_is_removed -- --exact --nocapture
```

Working directory: `/tmp/opencode/sol-e1-fixture-merge-20261026.lz4yvr6q/candidate`.

The absolute target was absent before this run and lies outside the candidate. Result: **exit 0; 1 passed, 0 failed/ignored**. The command filtered 64 other unit tests, five checkpoint tests and four WAL tests. It did not run doctests or a full store/engine/workspace suite. The log reports a fresh engine/store dependency build and no warnings.

Independent log SHA256: `5298f719979f57fe060642cb9149b3cc4c5271822683af3b389dcccd27a5d21c`.

`focused.command.json` retains exact argv, workdir, target, UTC boundaries, exit and log identity. `focused.before.json`, `focused.after.json` and `final-boundary.json` retain complete byte/mode maps and configuration observations. `focused.outputs.json` binds the three test executables and engine/store libraries. `toolchain.json` records rustc 1.99.0 `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, Cargo 1.99.0 `5f94df4789f005f9a352888e8355ffc645b7ed0e`, LLVM 23.1.1 and an empty inspected compiler-override environment before the assigned target. Ambient Cargo/toolchain configuration was absent at the checked paths and remained absent. All 17 local path-dependency declarations resolve inside the frozen nine-member candidate.

### Retained checks verified, not rerun

| Evidence under the author root | Verified result and source scope |
|---|---|
| `checks/unmerged-fixture.*` | Exit 0; 1 pass on unmerged map `198edd83...`. The baseline was already green. |
| `checks/merged-fixture.*` | Exit 0; 1 pass on merged map `282887d6...`. |
| `checks/store-clippy.*` | `cargo clippy -p store --all-targets --no-deps --locked --offline -- -D warnings`; exit 0, no warnings, merged map stable. |
| `checks/store-fmt.*` | `cargo fmt -p store -- --check`; exit 0, empty log, merged map stable. |

For each command, the actual log size/mode/hash matches its command record. Both retained source maps equal the appropriate saved 651-file map. Retained boundary-group hashes also agree before/after. Those historical original-tree observations are not a fresh stability claim about the concurrently changing original engine.

The completed `current/astra-store-e1-corrected-dependency-review.md` under the original isolated source retains its accepted **unmerged** engine7bfce917/storebadbc7e2 scope. I reused that dependency decision and reverified the retained author full-store result at `/tmp/opencode/sol-e1-shared-20261025.0lt2912d/engine-e1-corrections-20261025-r1/`: result SHA256 `fb4bf7ec652462a8c4b4a8fd4a7ffd2557368f09f6f95e38b14bf1225cd3f0a8`, log SHA256 `a32fe39d2fe0a21382248cd1f2bc6d2aec4b2a10ad9aaa9387a5b3c2bd1d7fa2`. Its saved before/after arrays match the saved corrected manifest's 46 engine, 47 store and 10 workspace records. The log contains **65 + 5 + 4 = 74 passes**, zero failures/ignored and a completed zero-case doctest group. This remains unmerged R1 dependency evidence. It is not a full merged-candidate or R2 suite result. Counts are not additive.

The reviewer evidence audit initially treated `workspace.members` as nine explicit entries. It stopped because the manifest contains `crates/*`. The corrected audit expands that pattern to the actual nine members. This was an audit-script assumption, not a fixture failure; it caused no source change or additional Cargo run.

## 7. Final read-only boundary

At **01:00:23 UTC**, final checks confirmed:

- All 651 candidate files and their complete path set match the tested merged map.
- All 649 primary Cargo/crate inputs match the pre-test map, SHA256 `f3c1dea703ed25031607e1ec34ad0600342dc0f4279e4696842a969a93c027f2`.
- All 698 author artifact files, the complete non-build inventory and the author manifest are unchanged.
- Ten monitored reports/contracts and seven additional retained references are unchanged; these sets overlap at the corrected-dependency report.
- Byte counts, SHA256, Git-normalized modes, recorded filesystem modes and configuration observations are unchanged.

The prior primary fixture report retains SHA256 `02a518d50c9896c74e36fe17e3decd8118b0b0da00247556ca89e11b14340a10`. The G2 fault report retains SHA256 `858c80cc6b24682c3c6efa886974cab25fe6aedead78f3d0844e38593f0646cd`.

Independent `ARTIFACT-SHA256.json` binds 23 review artifact files. Its SHA256 is `434533eb477cf7c42010b61408f956aadb13e2153af789f13eaaebedcf6c3afc`. It excludes itself and the build target; the output manifest separately binds executed binaries/libraries.

## 8. Later promotion recipe

1. Obtain the final R2 engine/dependency pins and the integration assignment. Keep this R1 focused result attached to its existing pins.
2. Reconcile the accepted E1 production source with primary. The fixture patch base is E1 `stable.rs` SHA256 `d57e49ac...`, not primary `7b5fcd89...` or the common baseline.
3. Apply only the reviewed two-hunk `comparison/proposed-fixture.diff` to that exact base. Exact application produces full-file SHA256 `2cb747b0...`. If the base changed, reconcile its context explicitly and record a new source identity.
4. Preserve the real Legacy2/later-owner adoption, all three acquired authorities, actual durable Open key ordering, one finish, one admitted marker, all 20 assertions and every unrelated test/production byte. The full proposed file is a reference artifact; do not copy it wholesale over primary.
5. Pin the resulting complete R2 input inventory and obtain new affected-fixture/dependency-join verification before claiming promotion. This report supplies the reviewed fixture-source delta, not acceptance carry-forward for a changed engine dependency.

Reviewer writes were limited to this new primary report and `/tmp/opencode/astra-e1-fixture-merge-review-20261026.trb_1gdy/`. No production/test/probe edits, Git writes, live actions, subagents or additional suite runs occurred.
