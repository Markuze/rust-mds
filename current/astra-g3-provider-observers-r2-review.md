# astra-g3-provider-observers-r2-review

Task: NEW independent P3 r2 correction review for OBS1–7. Review the frozen seven-path correction, exact receipt/scope/clock/call boundaries and original probes. Run the 157 focused tests and minimally adapted original 16 probes once per Python mode. Preserve all prior evidence. No source fixes, Git writes, live/installed/native actions or delegation.
Status: FINAL — CHANGES REQUIRED. Four residual findings: two P1 and two P2. Complete157 and adapted-original16 pass once per Python mode. New boundary9 gives5 pass/4 fail per mode, with zero errors/skips/guard hits. OBS2/4/5 close in the reviewed offline scope. Exact original regressions close, but residual OBS1/3/6/7 gaps prevent package acceptance. All327 existing pins and37 copied input records remain identical;126 old artifact pins and21 references match.
Context: Only this new repository report and new exclusive external review artifacts are writable. Author freeze: /mnt/workspace/sol-g3-provider-observers-r2-zsspjhll. Expected manifest846c82ce95249cf0204d0b05bbca1ca9f28f7ee7d811c2323a6c69dc618bfc9a; archive420b884023390fb112423bbc801de791cf2caeb0ed814d8c5e5e4927155e2f83; patch9e5aa2d5761a69066202a27617f130ba01dc041f908b6c85218bec2c1892ae52. Tests use a verified new copy, pre-import guards and explicit injected I/O.
Next: Parent assigns narrow corrections for P3-OBS-1-R2, P3-OBS-3-R2, P3-OBS-6-R2 and P3-OBS-7-R2. Preserve this review, the frozen r2 candidate and all red evidence. Freeze new inputs before independent re-review. P4/P5 retain their dependency gates.
Log:
- [2026-10-26] Created the sole writable repository report. Prior reports and evidence remain read-only.
- [2026-10-26 05:01 UTC] New /tmp/opencode/astra-g3-observers-r2-review-20261026.k5sk4p8z/creation.json hit EDQUOT. Retained the failed file. Continued in new exclusive /mnt/workspace/astra-g3-observers-r2-review-1sqjuuma; quota-failure.json pins the failure. Startup verifies327 existing files, including126 old artifacts, seven primary/snapshot/archive inputs and21 references. Extracted only verified archive members. Both new candidate copies and references match37 input records.
- [2026-10-26] Independent static AST/compile0/1/2/no-runtime-assert/import/guard/whitespace checks pass. Reconstructed patch matches9e5aa2d5. Test inventory retains127 IDs with receipt plumbing and adds30; it is not an unchanged127 baseline. Adapted probe source is byte/AST-identical fb06e9d2 and plumbing is exact3c23f214.
- [2026-10-26] Complete157 and adapted16 pass once per Python mode. Pre-import/discovery external-action guards are active. OBS3 rejects with query outside certified acquisition/clock interval after receiving valid persisted worker Intent/Return. The genuine full-call control passes. Actual worker hashes/membership match retained author evidence. Unadapted15/1, red-before8, intermediate26/1 and author quota outcomes remain retained failures.
- [2026-10-26] New independent boundary9 runs once per mode:5 pass/4 failed rejection assertions, zero errors/skips/guard hits. Failures cover intervening cleanup, acquisition after Intent, a mutable anchor alias and contradictory same-token RESET returns. Matched valid controls pass. Kept all logs and real persisted artifacts.
- [2026-10-26 05:08 UTC] Result audit passes. Final pin at05:08:33 UTC has zero errors and before_after_identical=true for327 existing files and37 copied inputs. Exact original probe/plumbing, seven-path archive,126 old artifacts and21 references remain stable. Completed scoped review with four residual findings. No implementation correction or offline package acceptance occurred.

## 1. Verdict and OBS1–7 disposition

**CHANGES REQUIRED — two P1 and two P2.** The r2 package fixes the exact original reproductions. Four adjacent acceptance gaps remain in changed code.

| Original finding | Independent r2 disposition |
|---|---|
| OBS1: borrowed kernel clean state | Original seq9/seq10 reproduction rejects. **Open:** an intervening cleanup before the selected cleanup still permits an ambiguous handler join. P3-OBS-1-R2. |
| OBS2: unrelated empty capture | **SCOPED CLOSED.** Final coverage requires a persisted `stream_scope` fact. Full acquisition source/config/boot equality and selected stream fields bind the explicit distinct-source edge. Independent valid scope, changed-config and changed-stream controls pass. |
| OBS3: receipt ends before exit | Original entry-only receipt rejects with a real valid worker boundary. Exact trace/worker membership and continuation tests pass. **Open:** final coverage can start after Intent and omit an earlier userspace invocation. P3-OBS-3-R2. |
| OBS4: wall-clock extrapolation | **SCOPED CLOSED.** `ClockPin` certifies the range and final quiet tail. Final source and clock pins share the persisted seal. Uncovered ranges, changed support and quiet-tail gaps reject; bracketed empty coverage passes. |
| OBS5: future close timestamps | **SCOPED CLOSED.** Normal, close and all post-close states use `_accept_state`. Clock regression/deadline/source checks and uncertain-close no-retry controls pass. |
| OBS6: failed call still joins | Exact failed-call latch reproduction now rejects with exit1. Immutable retained joins pin both receipts. **Open:** `begin` accepts a mutable anchor and the retained binding aliases it. P3-OBS-6-R2. |
| OBS7: RESET success ignores owned EIO | Exact owned EIO log rejects. ENOENT continuation and unrelated task/Client/closed-invocation controls pass. **Open:** a second same-token return outside the chosen timing window is ignored. P3-OBS-7-R2. |

These dispositions concern offline reducers and injected fixtures. They do not depend on installed collector qualification. The package is not accepted while the four findings remain.

## 2. Ranked residual findings

Implementation paths refer to the verified archive copy under:
`/mnt/workspace/astra-g3-observers-r2-review-1sqjuuma/candidate/`.

The original and adapted runtime copies have identical seven-path inputs. New probe paths below are relative to the review root. Each failure reproduces in normal and optimized Python.

### P3-OBS-1-R2 — P1: an intervening cleanup does not end handler selection

**Source:** `scripts/g3_peer_observe.py:659–688`. Cleanup selection at663/668 uses only `caps` as boundaries. The broader cleanup boundary set at671 applies only after the selected cleanup. Detach entry selection at684 also uses only `caps`.

**Probe:** `boundary-probes.py:139`, `test_obs1_intervening_cleanup_without_cap_ends_handler`; setup at80.

Feed one client/task/source with contiguous original spans:

```text
t20 wire ACK tid100 inode0x100 seq9
t30 CAPS handler inode0x100 seq9
t40 cleanup inode0x200 seq10
t60 cleanup inode0x100 seq9, cleaned zero
t70 inode0x100 now clean
```

The intervening inode0x200 cleanup is observed, but its CAPS row is absent. This is a partial/ambiguous handler chain. `join((ack,), identity)` nevertheless returns `FlushProcessed` by binding t30 to t60/t70. The observed cleanup at t40 must end the earlier handler interval. A missing row cannot supply permission to cross that boundary.

Pinned Linux v7.0 `caps.c:3856–3863` emits cleanup for a handler that can exit without detaching anything. Positive clean follows later in that same handler at3885–3890. This does not permit a cleanup from another invocation between the chosen handler and its cleanup.

**Control:** `boundary-probes.py:143` supplies one handler with a zero-cleaned cleanup and an actual detach entry/false-wake return before it. It returns `FlushProcessed` in both modes.

**Required correction:** Bound the entire handler→detach→cleanup→clean chain by every intervening handler or cleanup in the relevant task context. Do not restrict the boundary set to the selected inode/sequence. Preserve the valid zero-cleaned and false-wake case.

### P3-OBS-3-R2 — P1: trace coverage can begin after the worker Intent

**Source:** `scripts/g3_peer_observe.py:1163–1176,1194–1198`. Final validation checks trace coverage from the selected entry through the exit. It compares events from Intent onward, but does not require the receipt to cover the interval from Intent to that entry.

**Probe:** `boundary-probes.py:147`, `test_obs3_trace_must_cover_intent_to_first_selected_entry`; setup at54.

1. Persist worker Intent at `[5,6]` and matching Return at `[110,111]` in a real worker artifact.
2. Start the loss-free trace at t15. Persist an entry at `[20,21]` and matching exit at `[90,91]`. Its source watermark is200.
3. Supply both genuine final receipts and ask for `pending_at([40,41], trace, final=True, worker_coverage=worker)`.

Observed: `PendingCut(final=True)`. Both stored worker facts are actual receipt members. The trace artifact and seal accurately record support `[15,200]`. The missing interval `[5,15)` can contain a first entry/exit, for example t8/t12. The selected t20 entry can therefore be a second userspace invocation. The final evidence does not establish the original call.

The pinned provider plan §5 requires one original Intent→entry→exit→Return chain. It explicitly rejects substitution of a userspace retry for the original syscall. This is a source-coverage gap, not a requirement for an installed ABI or a new worker parser.

**Control:** `boundary-probes.py:157` starts trace coverage at t1 and supplies the same full boundaries. Kernel return−13 maps to libc result−1/errno13. Final pending proof passes in both modes. The adapted original truncated-tail probe independently still rejects with a valid worker receipt.

**Required correction:** Require loss-free trace support from the original Intent boundary through the selected exit. Reconcile all relevant syscall/continuation records in that interval. A source that starts later must not prove that its first visible entry is the original call.

### P3-OBS-6-R2 — P2: the original anchor snapshot aliases a mutable input

**Source:** `scripts/g3_peer_observe.py:994–1037,1160–1162,778–781`. `begin` does not require an immutable `CallAnchor`. It stores the same anchor object in `self.anchor` and `self.binding`.

**Probe:** `boundary-probes.py:162`, `test_obs6_mutable_anchor_alias_cannot_revalidate_retained_join`; fixture at54–67.

Pass `SimpleNamespace(**valid_anchor.__dict__)` to the public `begin` seam. Its validated initial fields equal the ordinary immutable anchor. Complete Open with original fd5, exact entry/exit, real persisted worker Intent/Return and valid metadata processing. Obtain a successful joined tuple.

Then change that accepted anchor's `fd` and first raw argument to6. Call the documented revalidation API:

```python
observer.join_call(processing, call, trace, worker, retained=old_tuple)
```

Observed: the retained tuple is accepted unchanged. Both receipt digests remain the same. The source trace still records fd5, but the current anchor claims fd6. The equality test compares the mutable object to its own alias, so it cannot detect the change. This uses ordinary attribute assignment; it does not bypass a frozen dataclass or alter source/receipt bytes.

**Control:** `boundary-probes.py:175` uses the exact frozen `CallAnchor`. The initial and retained joins pass and contain both actual receipt digests. Replacing the anchor with a changed frozen value correctly rejects.

**Required correction:** Reject mutable/noncanonical anchor inputs at `begin`, or retain a validated immutable value snapshot. Revalidation must compare independent original values and retain argument/FD consistency. Apply the same boundary rule to accepted call/worker objects. A new caller already revalidates in this reproduction; documenting that obligation does not resolve the alias.

### P3-OBS-7-R2 — P2: timing filters hide a contradictory same-invocation return

**Source:** `scripts/g3_peer_observe.py:712–731,740–749`. The library return selection considers only returns after the wire reply and before the FSAL return. It does not reconcile all returns for the selected token/context.

**Probe:** `boundary-probes.py:187`, `test_obs7_earlier_same_token_return_is_contradictory`; setup at101.

Use one exact UUID/RESET flag, PID/start/thread/Client instance and library token:

```text
t5  FSAL entry
t10 RESET attempt
t20 ceph_start_reclaim entry, token K
t25 ceph_start_reclaim return−5, token K
t30 wire RESET request
t50 successful nonempty wire reply
t60 ceph_start_reclaim return0, token K
t70 UUID continuation
t80 FSAL return0
```

Observed: `ResetSucceeded`. The t25 return is ignored because it precedes the wire reply. One invocation cannot return twice or continue to issue the selected request after its recorded return. These contradictory owned observations must remain unresolved or fail.

Ganesha V6.5 `main.c:384–389` calls `ceph_start_reclaim` once and uses that one result before UUID continuation. Ceph v20.2.0 `Client.cc:17401–17409` has distinct ENOENT/success returns. Neither source permits selecting one convenient outcome from two same-token returns.

**Control:** `boundary-probes.py:191` adds a complete later FSAL/library invocation with a different token/UUID and EIO. The valid selected RESET still succeeds. Its closure and ownership make it unrelated. Existing157 controls preserve ENOENT continuation and RESET without a Ceph FINISH.

**Required correction:** Gather all entry/return outcomes for the selected invocation token/context before applying ordering requirements. Require one consistent entry and return. Reject duplicate/contradictory outcomes even outside the preferred wire timing window. Preserve genuinely unrelated closed invocations.

## 3. Exact inputs and stability

Frozen author root: `/mnt/workspace/sol-g3-provider-observers-r2-zsspjhll`.

Independent root: `/mnt/workspace/astra-g3-observers-r2-review-1sqjuuma`.

| Input | SHA256 |
|---|---|
| Seven-path manifest | `846c82ce95249cf0204d0b05bbca1ca9f28f7ee7d811c2323a6c69dc618bfc9a` |
| Seven-path archive | `420b884023390fb112423bbc801de791cf2caeb0ed814d8c5e5e4927155e2f83` |
| Reconstructed exact r1→r2 patch | `9e5aa2d5761a69066202a27617f130ba01dc041f908b6c85218bec2c1892ae52` |
| Peer implementation | `ff2c6d7cd0c3c90d5f919d690bfef755e9ce24bab79355dc6f2f4d0edf750406` |
| I/O implementation | `c9ece4f796b58ad7680006b1cfc412677df90523541bb1d5ef3ddec0bb217618` |
| 21-reference map | `7de4ed59c02753f5bbf070c54ec0d6054eb4c3ee79223e4c2ba87c7b18c6d028` |
| Exact original16 probe source | `fb06e9d21afa1ac16db56cbabdf49dd2d3bb9d6c83cf1581296bcbb71d21ad6b` |
| Reviewed external fixture plumbing | `3c23f2140387009ba5b70b721ded69b77a3b69471787ab1c623939467b4f0f46` |
| Independent new boundary9 source | `e5fbab3a506914ea77cfeafc35738d0e3fff99e088089c5a0ba01738da38dc6a` |
| Immutable fault-v1 | `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523` |
| Original completed review | `ee867189fbfba9fe3b5458e6f00f47d0d09d5507935c113348dd14b871fc9860` |

All seven candidate files have mode0664. The archive contains exactly seven regular-file members. The old author ledger stays outside that archive and matches its original485551fb pin.

`before.json` at05:01:21 UTC and `after.json` at05:08:33 UTC verify **327 existing files**. This fixed inventory includes the completed r2 freeze,126 old artifact pins, seven primary source/ledger paths and immutable reference inputs. It excludes active coordinator ledgers. Both verified candidate copies,21 copied references and the copied original probe/plumbing add **37 copied input records**. All before/after values match. There are zero mismatch exclusions.

The new local archive/manifest/patch snapshots also match the author files in `result-audit.json`. The original report, original probe, old archives, red/intermediate/unadapted outcomes and author quota failure remain unchanged.

## 4. Independent commands and results

Set `R=/mnt/workspace/astra-g3-observers-r2-review-1sqjuuma` for the command forms below. Actual commands used the full absolute path. `guarded.py` installs process/signal/network guards before loading any runner, test or provider. The copied owned runner changes only `ROOT` to `$R/candidate`; its exact adaptation audit passes. The copied probe/plumbing files remain byte-identical to their supplied pins.

| Command | Workdir | Result |
|---|---|---|
| `python3 -B "$R/pin.py" before` | `$R` | Pins/archive/static checks pass; exit0. |
| `python3 -B "$R/guarded.py" "$R/checks/run_owned_r2.py" independent-normal all` | `$R/candidate` | 157 pass; exit0. |
| `python3 -B -O "$R/guarded.py" "$R/checks/run_owned_r2.py" independent-optimized all` | `$R/candidate` | 157 pass; exit0. |
| `python3 -B "$R/guarded.py" "$R/adapted/worker_receipt_plumbing.py"` | `$R/adapted/candidate` | 16 pass; exit0. |
| `python3 -B -O "$R/guarded.py" "$R/adapted/worker_receipt_plumbing.py"` | `$R/adapted/candidate` | 16 pass; exit0. |
| `python3 -B "$R/guarded.py" "$R/boundary-probes.py"` | `$R/candidate` | 9 tests:5 pass/4 fail; exit1. |
| `python3 -B -O "$R/guarded.py" "$R/boundary-probes.py"` | `$R/candidate` | 9 tests:5 pass/4 fail; exit1. |
| `python3 -B "$R/audit-results.py"` | `$R` | Runner, predicates, actual worker artifacts and result audit pass; exit0. |
| `python3 -B "$R/pin.py" after` | `$R` | All327 existing/37 copied pins stable; exit0. |

All runtime suites record zero errors, skips and guard hits. Each suite ran once per Python mode. Counts overlap and are not added as unique tests. The157 inventory is94 peer/call plus63 I/O/clock methods:127 retained IDs with receipt plumbing plus30 regressions. It is not an unchanged127-baseline result.

Static checks cover the four candidate Python ASTs, compile optimize0/1/2 without execution, no runtime `assert`, the implementation import allowlist, pre-import guards, whitespace/newlines, file modes, exact patch reconstruction and reference/input hashes. Source audit checked the finite record/byte/line/queue/descriptor/work bounds and the new tuple/byte receipt limits. No broad Python or Rust suite ran. No author freeze script ran.

### Original16 adaptation audit

The original probe bytes, AST,16 test bodies, rejection predicates and positive control remain unchanged. The external wrapper replaces only `call`/`finish_call` fixture helpers and supplies `worker_coverage` to the original `pending_at` method. It changes no expected result or exception handler.

Both final-call probes receive actual fsynced/closed worker artifacts. The original truncated trace rejects with:

```text
CallTraceContinuity: query outside certified acquisition/clock interval
```

It does not reject for a missing worker argument. Its worker Intent/Return members and the full positive control's members are verified against actual saved bytes. The full control preserves kernel−13/libc−1/errno13. Worker artifact hashes match in both modes:

- Truncated-trace case: `9dcb8efc8cce48cb8deead71ac969828805f05fb1924473eaa709acbe3da73b9`.
- Full-call control: `ad02ba74a8f6a1dc1673e5ea6c2b9fa835d9202ae02bbcb591d929336dab2e6b`.

Verified retained final-source unadapted15-pass/one-positive-control-error receipts were reused. They remain failed; they were not rerun or relabelled. The original8 red failures, initial26-pass/one-fixture-error result, other intermediate runs and zero-byte author quota runner retain their prior scopes.

### New boundary9 evidence

`boundary-probes.py` imports only the exact original independent probe helpers and candidate APIs. It imports no author test or fixture. New raw literals, scoped typed records and expected outcomes define the nine probes. Its persisted trace/worker/scope sinks use real file writes, file fsync, close and directory fsync. Saved bytes equal receipt frames and digests. The new negative transcripts are deliberate incomplete/contradictory inputs; their valid controls establish that the intended proof path is reachable.

Retained artifacts include `checks/independent-{normal,optimized}.{log,json}`, `adapted/adapted-{normal,optimized}.{log,json}`, `boundary-{normal,optimized}.{log,json}`, `result-audit.json`, `before.json`, `after.json`, all new persisted sink files, `REVIEW.md` and `review-artifacts.json`. The final inventory seals these new artifacts. The failed `/tmp/opencode/...k5sk4p8z/creation.json` remains at its original location; `quota-failure.json` records the fallback.

## 5. Remaining scope and consumer limits

The new receipt retains immutable bounded facts, byte frames, supported interval and optional clock pin. Exact fact membership includes original spans and task/source metadata. Actual ProviderIO execution checks source loss/barriers, late records and persistence phases. Typed worker/continuation/scope records remain explicit injected fixture-adapter seams. They are not new native parsers.

Full P1 fronts, address unions, wire final-inventory construction and P2 joins remain P4 inputs. The P3 `Wire` projection does not replace those inputs. Scope membership is required even for an empty relevant wire list. Call joins still cover metadata operations; flush-to-worker and NFS joins remain explicit dependencies. A consumer must revalidate retained tuples before use; P3-OBS-6-R2 exercises that obligation and still fails.

Origins remain **Fixture/Unqualified** and levels remain **OFFLINE_VALIDATED**. Native construction/flags stay disabled. Installed ABI equivalence, actual collector clock/counter truth, access and package qualification remain separate. They are not prerequisites imposed on this offline verdict. Full G3 and P4/P5 dependency acceptance remain open.

Only this new repository report and exclusive external artifacts were written. Source/tests/fixtures/contracts/old reports stay unchanged. No active core/P2/daemon/release output was read. No fixes, Git write, installed/live/native collection, capture, process/network control or delegation occurred.
