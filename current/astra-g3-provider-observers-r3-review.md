# astra-g3-provider-observers-r3-review

Task: NEW independent P3 r3 delta review of P3-OBS-1-R2, P3-OBS-3-R2, P3-OBS-6-R2 and P3-OBS-7-R2. Verify frozen scope and preservation. Review assertion-timing adaptation. Run complete181, original16 with unchanged worker plumbing, unchanged boundary9 and separately adapted boundary9 once per Python mode. Review only offline synthetic/injected evidence.
Status: FINAL — SCOPED ACCEPT, offline P3. P3-OBS-1-R2, P3-OBS-3-R2, P3-OBS-6-R2 and P3-OBS-7-R2 close at the frozen r3 pins. Accepted offline OBS2/4/5 remain closed. Per normal/optimized mode: complete181 passes; exact original16 plus unchanged worker plumbing passes; unchanged boundary9 remains8 pass/one canonical-input ObservationError, exit1; separately labelled assertion-timing9 passes; three new narrow controls pass. Final pins match657 existing files and70 copied inputs. Result audit passes. Scope remains Fixture/Unqualified and OFFLINE_VALIDATED.
Context: Own only this new repository report and new exclusive external review artifacts. Author freeze: /mnt/workspace/sol-g3-provider-observers-r3-6b36vzy5. Expected manifestc8518d1479db1d80842fd10a6c910ab6475c6bfe4ac4847c62263afa8f8d36f3; archivedfa98bead0a8f03cf6de32413068b5236607fa6a04411e30237cd9e21139c861; delta4fa5a4a26fc9cb4bb2e787a1fe465c4683b6b67c43d628773d5a8c1a6f2261bc. Source/tests/fixtures/contracts/old reports/evidence are read-only. No fixes, Git writes, installed/live/native actions, capture, process/network control or delegation. Active outputs remain unread.
Next: Parent receives this completed scoped review and its sealed inventory. Preserve all old decisions and failed outcomes. Full P1/P2 inputs, P3–P4 joins, installed qualification and P4/P5/hooks/native/full G3 retain their separate gates.
Log:
- [2026-10-26] Created the sole writable repository report first.
- [2026-10-26 05:43 UTC] Preserved the new EDQUOT attempt. Created exclusive /mnt/workspace/astra-g3-observers-r3-review-suxxm_ya. Independent before.json verifies657 existing files and70 copied inputs with zero errors. Groups include428 immutable records,218 author artifacts,219 bundle members, seven r2 preimages,105 old review artifacts and21 references.
- [2026-10-26] Verified the exact six-method delta, unchanged I/O bytes,157 unchanged r2 test-method ASTs,24 new methods and one append-only fixture vector after29 exact originals. Static compile0/1/2/import/guard/whitespace/archive checks pass. Only the copied owned runner's ROOT changes.
- [2026-10-26] Ran all four requested suites once in normal and optimized Python. Complete181 and original16 pass. Unchanged boundary9 remains8 pass/one early canonical-input error. Labelled assertion-timing9 passes with exactly one changed method and the original mutable input. All eight outer guards and all recorded inner guards have zero hits. No verdict yet.
- [2026-10-26 05:52 UTC] Three new narrow controls pass per mode: ten wrapper-predicate cases, six endpoint cases and two changed-frozen-worker cases. These controls address specific uncovered boundaries. They execute no prior test method. All ten runtime guard reports have zero hits.
- [2026-10-26 05:52 UTC] Independent result audit passes with zero errors. It checks all new suite logs/JSON,56 saved sink files,36 explicit persistence-phase transcripts and original worker membership. It keeps four empty failed sinks and four post-seal failed sinks distinct from48 successful receipt artifacts.
- [2026-10-26 05:52 UTC] Final pin at05:52:45.715545 UTC passes. All657 existing files and70 copied inputs match startup. Completed the semantic delta/adaptation review. Verdict: SCOPED ACCEPT offline P3. The exclusive REVIEW.md snapshot, review-artifacts.json inventory and REVIEW-SEAL.json envelope record this cutoff.

## 1. Verdict

**SCOPED ACCEPT — offline P3 at the exact r3 manifest.** No residual finding remains in the four assigned boundaries. This decision preserves the independent r2 acceptance of OBS2/4/5.

| Finding | Independent r3 disposition |
|---|---|
| P3-OBS-1-R2, P1 | **CLOSED offline.** One full task-context CAPS/cleanup boundary set bounds handler selection, detach and final clean. Other inode/sequence rows cannot disappear from that set. Distinct rows that overlap selected endpoints reject. |
| P3-OBS-3-R2, P1 | **CLOSED offline.** Final trace support covers original Intent.lo through selected exit.hi. Exact syscall/continuation inventory, source/CPU scope and worker Intent/Return/tail membership remain required. |
| P3-OBS-6-R2, P2 | **CLOSED offline.** Early canonical rejection prevents the mutable alias. Accepted nested values are immutable. Final validation retains both receipt digests and independently checks original argument/FD values. Equal frozen replacements pass; changed frozen worker or anchor facts reject. |
| P3-OBS-7-R2, P2 | **CLOSED offline.** Selected library/FSAL invocation tokens require one entry and one return before outcome/timing selection. Duplicate or contradictory outcomes reject. Valid unrelated invocations, ENOENT continuation and RESET without FINISH remain valid. |
| OBS2 | **Preserved offline closure.** Persisted acquisition scope binds source/config/stream. The unchanged boundary control accepts a distinct acquisition source and rejects changed config/stream. |
| OBS4/5 | **Preserved offline closure.** Reuse pinned r2 clock-support and uniform-state reasoning. I/O source/tests remain byte-identical. The new complete181 and original16 runs retain the clock and close-state controls. |

## 2. Semantic delta audit

Source locations below refer to the verified copy at:
`/mnt/workspace/astra-g3-observers-r3-review-suxxm_ya/candidate/`.

### OBS1: handler ownership

`scripts/g3_peer_observe.py:528–531,647–690` uses strict conservative ordering. A boundary must be the selected endpoint, strictly before the start, or strictly after the candidate. Equality of timestamps does not exempt another source reference.

The boundary inventory at659 includes every observed same-context `cap` or `cleanup`. It does not filter inode, sequence or client first. The same inventory applies to selected cleanup, detach entry/return and positive clean. Each selected endpoint remains distinct from another observed row. Contiguous source-span checks also reject duplicate source-row reuse.

The original intervening other-inode cleanup now rejects with `flush cleanup missing/ambiguous`. Complete181 covers cleanup before/during detach, before/after selected cleanup and legal different-task/later cleanup. The new endpoint control puts a distinct cleanup at the exact uncertainty interval of the selected CAPS, cleanup or clean row. All three reject. Its ordinary, different-task and strictly later controls pass. The original zero-cleaned/false-wake detach control passes in both boundary runs.

Pinned Linux v7.0 `caps.c:3829–3903` supports this order: detach precedes cleanup logging; a false wake return does not mean detach failure; positive clean can follow later in the same handler. These are behavioral references, not translated code or installed-ABI proof.

### OBS3: conservative original-call support

`scripts/g3_peer_observe.py:1169–1208` checks persisted trace support over `[Intent.lo, exit.hi]`. `Coverage.check` requires support to start no later than Intent.lo and to extend strictly beyond exit.hi. It verifies receipt/source/clock pins. Final validation checks complete required CPU/source scope and the exact selected-task syscall/continuation inventory. Events that overlap Intent are retained in the comparison. Observed reuse of the numeric PID/TID with different lifetime fields fails that comparison.

The independently executed181 includes start6/start15 rejection for Intent[5,6], start1/start5 controls, hidden earlier entry/exit, prefix continuation and overlap controls. The original restart controls remain valid. The unchanged/adapted boundary runs independently reject start15 with a genuine persisted worker receipt. Original16 rejects a trace ending at55 before the selected exit[90,91]. Its error is incomplete certified support, not a missing worker argument.

Worker coverage independently binds the exact original Intent and Return. The ordered worker inventory rejects duplicate/conflicting same-call facts. Original source spans and final tail remain verified. The full original-call control retains kernel−13, libc−1 and errno13. Numeric task identity and installed lifetime attribution retain their documented qualification limits.

### OBS6: canonical values and retained joins

`scripts/g3_peer_observe.py:1001–1051` requires exact `WorkerReceipt`, `CallAnchor` and anchor `Ref` types before retaining state. Worker sanitization also requires an exact `Ref`. `Ref` construction requires exact frozen `Source`, `Cursor` and `Interval`; their constructors check concrete primitive fields and immutable CPU tuples.

The anchor accepts six exact integer arguments, exact integer FD values, concrete SHA256 strings, at most two immutable path pairs and an optional four-integer lock tuple. Subclasses, namespace aliases, mutable argument/path/lock containers and noncanonical references reject. Accepted values cannot retain the original mutable namespace alias.

`finish:1145–1167` requires an exact frozen worker receipt and the signed kernel/libc result relation. `validate_final:1169–1208` requires exact call-state types and equality with the previously validated immutable binding/finished state. It separately compares the original entry argument hash and FD value. `join_call:766–790` still requires exact `CallTrace` and retains the six-element join tuple, including both actual receipt digests.

Complete181 and the unchanged frozen boundary control verify equal replacements and changed argument/FD rejection. Two additional independent cases validate both original receipts, accept equal frozen replacements, then reject a changed frozen Intent call_id or Return result with `retained original call state changed`. They use fresh real persisted trace/worker sinks.

### OBS7: complete selected-invocation outcomes

`scripts/g3_peer_observe.py:707–764` first selects the library entry and enclosing FSAL invocation. It then gathers the bounded owned probe inventory by site/token/context. It requires exactly the selected entry and one return for each invocation. UUID/RESET flags, wire-reply ordering, UUID continuation, FSAL return and owned error-log consistency follow those inventory checks.

Thus an early−5/later0 pair, duplicate success, duplicate entry, conflicting UUID/flags or duplicate FSAL outcome cannot disappear behind a convenient wire-time filter. The original same-token counterexample now rejects with `actual ceph_start_reclaim return absent/ambiguous`. Complete181 checks additional entries/returns before and after the preferred window. It preserves distinct closed tokens/tasks/explicit Client instances. The original later different-UUID invocation also remains valid. ENOENT remains `AbsentResetContinued`, distinct from `ResetSucceeded`.

Pinned Ganesha V6.5 `main.c:382–390` uses one `ceph_start_reclaim` result before UUID continuation. Pinned Ceph v20.2.0 `Client.cc:17401–17409` distinguishes ENOENT and successful RESET. RESET does not require a Ceph FINISH. New lists filter existing bounded inventories; they add no retained history.

## 3. Independent execution

Set `R=/mnt/workspace/astra-g3-observers-r3-review-suxxm_ya`. Actual commands used absolute paths. Python was3.14.4. Each listed runtime suite ran once in normal mode and once with `-O`.

Command form:

```sh
python3 -B [-O] "$R/checks/guarded.py" "$R/checks/<stage>-<mode>-guard.json" <target> [arguments]
```

| Stage / target / arguments | Workdir | Normal | Optimized |
|---|---|---|---|
| `independent`; `$R/checks/run_owned_r3.py independent-<mode> all` | `$R/candidate` |181 pass, exit0|181 pass, exit0|
| `public16`; `$R/public16/worker_receipt_plumbing.py` | `$R/public16/candidate` |16 pass, exit0|16 pass, exit0|
| `boundary-unadapted`; `$R/boundary-unadapted/boundary-probes.py` | `$R/boundary-unadapted/adapted/candidate` |8 pass/0 failed assertions/1 early ObservationError, exit1|same|
| `boundary-early`; `$R/boundary-early-assertion/earlier_assertion.py` | `$R/boundary-early-assertion/adapted/candidate` |9 pass, exit0|9 pass, exit0|
| `controls`; `$R/controls.py` | `$R` |3 pass, exit0|3 pass, exit0|

All ten runtime commands record zero outer/inner guard hits and zero skips. Passing suites record zero failures/errors. The unchanged boundary suite remains failed. The three new tests contain ten wrapper-predicate cases, six endpoint cases and two frozen-worker cases. These are separate, narrowly justified controls, not additional claimed unique package tests.

The complete inventory is118 peer/call plus63 I/O tests. Its24 r3 methods overlap181 and were not rerun separately. All157 r2 test-method ASTs remain identical. The old157 retained127 IDs with earlier fixture changes plus30 methods; this review does not establish unchanged127 proof. Counts and modes overlap and are not added.

Other new commands, each from `$R`:

```sh
python3 -B "$R/pin.py" before
python3 -B "$R/audit-results.py"
python3 -B "$R/pin.py" after
python3 -B "$R/seal.py"
```

`pin.py` verifies archives, copies, references, exact delta and static scope without importing providers. `audit-results.py` reads saved results and sink bytes. `seal.py` snapshots this report and seals only the new review artifacts. The copied owned runner changes only `ROOT`. All original probe/plumbing/wrapper bytes remain exact. No author freeze script ran.

## 4. Required adaptation audit

### Original16 worker plumbing

The exact original16 source, all test bodies and rejection predicates remain unchanged. The exact reviewed wrapper only supplies worker fixture rows and independently persisted worker coverage to both final-call probes. It leaves the original pending method and expected outcomes intact.

Both modes persist two actual worker facts for each case. Saved bytes, SHA256, seal, source spans and Intent/Return membership match:

- Truncated trace: `9dcb8efc8cce48cb8deead71ac969828805f05fb1924473eaa709acbe3da73b9`.
- Full signed-return control: `ad02ba74a8f6a1dc1673e5ea6c2b9fa835d9202ae02bbcb591d929336dab2e6b`.

The truncated trace rejects with `CallTraceContinuity: query outside certified acquisition/clock interval`. Its valid worker boundary extends through111. The full trace extends through140 and the worker boundary through121. Actual saved facts preserve exit−13 and Return−1/errno13.

### Unchanged9 versus assertion-timing9

The unchanged boundary source remains `e5fbab3a...`. Its original mutable `SimpleNamespace` reaches `CallTrace.begin:1003` and raises:

```text
CallTraceContinuity: canonical frozen worker receipt and call anchor required
```

That pre-admission error has exit_code2. The old assertion occurs later, so **unchanged9 remains8 pass/one error, exit1** in both modes. This failed result is retained.

The pinned wrapper invokes that original method and mutable input unchanged. It replaces only that method's assertion timing. It catches only `ObservationError`; checks prerequisite, exact detail and exit_code2; and requires both `complete_call` and `begin` traceback frames. The actual saved frames are `expect_earlier_validation`, original mutable test, `complete_call`, `begin`, `_run`. The other eight methods remain unchanged, including the genuine frozen positive/changed-frozen negative control.

The new predicate control compiles only the exact wrapper function AST. It executes no original test method. A synthetic matching exception is the positive predicate control. TypeError, AttributeError, unrelated RuntimeError, wrong prerequisite/detail/exit, either missing frame and no exception all fail the wrapper. These ten cases pass in both Python modes. Synthetic predicate success is not provider acceptance evidence; the actual unchanged/adapted runs supply that evidence.

## 5. Saved persistence evidence

`result-audit.json` verifies56 new saved sink files:

- **48 successful receipt artifacts:**12 from original16,28 from the two boundary stages and8 from new frozen-worker controls.
- **4 empty failed sinks:** future-close and uncovered-clock cases, each mode.
- **4 post-seal failed sinks:** future-revalidation and late-loss cases, each mode. A saved seal alone does not mean `finish` returned a receipt.

For all52 nonempty files, the audit checks exact canonical frames, fact count/digest, source/config/boot, support interval, CPU counters/barriers, contiguous original spans and consumed terminal tail. It compares every declared artifact hash with actual saved bytes.

The28 boundary and8 new-control artifacts have36 explicit phase transcripts. They record real writes, three artifact fsync calls, one source close, one sink close, directory fsync and final persisted observation in order. The pinned helper executes actual file writes, `os.fsync`, file close and directory `os.fsync`. Original16 retains its worker hash/membership records and successful real-sink/closed-resource assertions. Acquisition, clock and source-state rows remain injected fixtures.

## 6. Exact inputs, scope and preservation

Author freeze: `/mnt/workspace/sol-g3-provider-observers-r3-6b36vzy5`.

| Input | SHA256 |
|---|---|
| R3 seven-path manifest | `c8518d1479db1d80842fd10a6c910ab6475c6bfe4ac4847c62263afa8f8d36f3` |
| R3 seven-path archive | `dfa98bead0a8f03cf6de32413068b5236607fa6a04411e30237cd9e21139c861` |
| Exact r2→r3 delta | `4fa5a4a26fc9cb4bb2e787a1fe465c4683b6b67c43d628773d5a8c1a6f2261bc` |
| Peer source | `8f13f598f2bf2a171ecaa5b062fcdf4ff1711bb3426d814289a685a5395c2b47` |
| Unchanged I/O source | `c9ece4f796b58ad7680006b1cfc412677df90523541bb1d5ef3ddec0bb217618` |
| Peer tests | `c1915e61b769dc32f708384d46c3d0876df1b5c7683dec2dc3b526806b194e19` |
| Unchanged I/O tests | `3eb9c7be872927aca7a468bdd6a5c42b2e3af31d587bb0ce7849dcf64620cf21` |
| Fixture events | `f7478b575e02cd5bf9f9225afd40e713917a3c2bcf39f203639bbcc07a11e43e` |
| Fixture README | `44b0245a8c4a38e16877edc4c177c6c855c8c886009447cf741fc9056da4d90e` |
| Completed r3 author ledger | `69128f90edb56083584b57d6a0f8005b110db841e3d926990ba323ba8135a8bc` |
| Author218-file inventory | `c2af39801e3b8742385993ed14969830e05039d18600401eb9a55d0544d44fa5` |
| Author219-member bundle | `c865608677872af557950ba7cc2a4231c5db2fcfae99c00fb1fd33eeea36c5bc` |
| 21-reference map | `7de4ed59c02753f5bbf070c54ec0d6054eb4c3ee79223e4c2ba87c7b18c6d028` |
| Fault-v1 contract | `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523` |
| Original16 source | `fb06e9d21afa1ac16db56cbabdf49dd2d3bb9d6c83cf1581296bcbb71d21ad6b` |
| Exact worker plumbing | `3c23f2140387009ba5b70b721ded69b77a3b69471787ab1c623939467b4f0f46` |
| Unchanged boundary9 | `e5fbab3a506914ea77cfeafc35738d0e3fff99e088089c5a0ba01738da38dc6a` |
| Assertion-timing wrapper | `ba444fda25f8fb99708e913b848d3ac0c817a7ce844de0f5618ec88ac0b93c04` |
| Old105-artifact review inventory | `adb8c9d79cde3de42aecd603ff869c67b8c5497730ae967b98a56a1332cca8a4` |
| Completed r2 review | `b0fadd0ed5dc8038559d04e17905fdb3a3e738f378d0ee68731faaf26c2a4d76` |

The source archive contains exactly seven regular files, all mode0664. Four verified candidate copies match it. The exact implementation delta changes only `_within`, `_flush`, `_reset`, `begin`, `finish` and `validate_final`. Two test helpers add default-preserving start parameters. The29 original fixture vectors remain byte-identical; one invented cleanup vector appends. The new author ledger replaces no old ledger.

Static AST/compile optimize0/1/2, no-runtime-assert, implementation import allowlist, pre-import guards, whitespace and exact reconstructed delta checks pass. Peer/call default4096/max65536 record bounds remain. Peer default4MiB bytes and16777216 join-work gate remain. I/O record/byte/line/queue/descriptor/duration/drain/CPU/receipt bounds remain unchanged.

`before.json` at05:43:10.831637 UTC and `after.json` at05:52:45.715545 UTC verify **657 existing files and70 copied input records**, with zero errors and `before_after_identical=true`. They verify428 immutable old records, seven complete r2 preimages,218 author artifacts,219 bundle members,105 old review artifacts and21 references. These groups overlap.

The check separates the six authorized primary working inputs from immutable r2 evidence. It does not claim those working paths still contain r2 bytes. The old327/37/126 groups keep their historical scopes. All old reports, freezes, archives, probes and failures remain unchanged. The verified reviewer r2 red9 and author r2 red9 both remain5 pass/4 failed assertions per mode. They were reused, not rerun. Original8/8, unadapted15/1, intermediate26/1, author79/three-subtest and quota outcomes keep their recorded scopes.

The new empty EDQUOT artifact remains at `/tmp/opencode/astra-g3-observers-r3-review-20261026.5iaaibge/creation.json`. `quota-failure.json` records the fallback. The sealed inventory includes its external hash.

## 7. Review seal and limits

Exclusive review root: `/mnt/workspace/astra-g3-observers-r3-review-suxxm_ya/`.

- `REVIEW.md`: exact snapshot of this final report.
- `review-artifacts.json`: SHA256/byte/mode inventory of all new review files, including runners, copies, results, failures, saved sinks and this snapshot.
- `REVIEW-SEAL.json`: inventory hash, report hash, file count and successful inventory recheck. The inventory excludes itself and this envelope explicitly.

Origins remain **Fixture/Unqualified**. Levels remain **OFFLINE_VALIDATED**. The reviewed `Wire` projection does not replace full P1/P2 inputs. P3–P4 joins, installed ABI/collector/identity/clock/counter qualification, P4/P5/hooks and native/full G3 remain separate. A consumer must still revalidate retained call tuples before use.

This owner wrote only this new repository report and exclusive external review artifacts. Candidate source/tests/fixtures/contracts and old evidence stayed read-only. Active NFS/core/daemon/release/canonical outputs remained unread and unpolled. The review performed no fixes, Git writes, live/installed/native action, capture, process/network control or delegation.
