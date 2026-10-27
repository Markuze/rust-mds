# astra-g3-wire-nfs41-r3-review

Task: NEW independent P2 r3 review of NFS41-R2-1–4. Parent orchestrates. Review the frozen offline Python parser/reducer, its retained evidence, exact scope and bounded history. Return SCOPED ACCEPT or ranked residual findings with valid controls, executed counts and sealed pins.
Status: COMPLETE — CHANGES REQUIRED. One confirmed P1 residual finding NFS41-R3-CHILD-CLOSE blocks offline P2 closure. All four r2 reproduction defects (NFS41-R2-1–4) are closed by candidate r3 in their recorded scopes; NFS41-1–5 stay preserved. Before/after/result preservation verified. Review evidence sealed in REVIEW-MANIFEST.json SHA256 `09d2ec01c34d835dad4d0609bf0b72616b1c21c8538f5a01e7d3749244d4ade0` (196 artifacts, 4136869 B, all mode 0664), re-verified read-only with 0 drift.
Context: Own only this new primary ledger and new exclusive external review artifacts/copy. Candidate source/tests/fixtures, old ledgers/reviews, contracts and evidence remain read-only. Use only archived helper67bac6ed and frozen references. Install process/network/signal guards before provider imports or discovery. Active P1 publication, P3/core reviews and daemon outputs stay unread/unpolled. No fixes, Git writes, live/installed/capture/native actions, process/network control or delegation. Model handover 2026-10-26: the earlier Astra continuation ended with a provider cyber_policy error after all measurements were recorded; this ongoing report is the GLM continuation (opencode/glm-5.3-flash-free#max) selected by the parent after Z.AI insufficient-balance failure. No scope or source change follows from the model change.
Next: No further read/write/test/work from this reviewer on this topic; parent decides whether to assign an r4 author correction cycle for the residual P1 defect (see FIN.1/FIN.2) or route a different reviewer. Anything outside `current/astra-g3-wire-nfs41-r3-review.md` and `/mnt/workspace/astra-g3-wire-nfs41-r3-review-q7w79itj/` must not be touched without a new parent assignment.

Review Decision:
Offline P2 verdict CHANGES REQUIRED with one P1 residual finding.

## FIN.1 Residual P1 finding NFS41-R3-CHILD-CLOSE
A late same-compound child lock stateid READ/WRITE after CLOSE succeeds even though its open state was terminally closed in the same compound. This survives both the separate public red-control suite and the two new rejection subcases, run once per mode in normal and optimized.

Exact reproduction (same in both failed subcases):
- Prior compounds: OPEN generation1 (inode 501) and a successful LOCK generation1 child stateid; both keep their request context.
- Final compound: `SEQUENCE(3) | PUTFH | CLOSE(open seqid 0) | READ/WRITE(lock seqid 1)` — the consumed operation appears AFTER CLOSE inside the same compound.
- FAIL is the same-compound-after-CLOSE order. PASS is the same-compound-before-CLOSE order. All other stream bytes and cross-compound conditions are identical.

Observations recorded (both modes; per-call id/other fact values pinpointed in the pin-verified `run/history-observations/` JSON):
- last reply: terminal success `failed=false` with no outstanding/unresolved, a valid `OperationInode` edge for `ino==501` with `owner_kind=="lock"` on the post-CLOSE op index 3, and an already-recorded `StateClosed` for the `seqid 1` open state of the same compound.
- `finish()` accepted this final report — directly contradicting RFC8881 §8.2.4 (stateid lifetime ends with its accounts-returned OPEN CLOSE) and §18.2.3 (a server that releases locks on CLOSE must not leave lock state consumable).

## FIN.2 Root cause (verified by source reading)
- `g3_wire_nfs41.py:1455` — `self._need(view['output']['life'] not in closed, ...)` only checks the single record's own `life`. `closed` is built at 1473 from each CLOSE record's own output life (an open state's life), never extended across the record's open_life relation at 1466, so a child lock state is never closed when its OPEN gets closed.
- `g3_wire_nfs41.py:1553–1559` — `_resolve(..., historical=True, purpose in (25, 38))` lets the already-inactive state pass, returning a temporary active view with `active=True` while the underlying state has already ended.
- `g3_wire_nfs41.py:1750–1756` — `_execute` correctly sets `state["active"] = False` on CLOSE, confirming that the allow-inactive path above contradicts the recorded state terminator.

## FIN.3 Valid controls (same candidate provider stream and RPC bytes; only compound-C operation order differs)
The before-CLOSE order control (same source, same per-stream arrival) produces a valid final report with a StateClosed edge; both modes pass. Recorded in:
- `run/history-observations/test_child_consumption_before_compound_close_is_valid-{25,38}-{normal,optimized}.json`.

## FIN.4 Scope of r2 findings after the r3 candidate
The 12 added owned test IDs (listed in `final-verification.json` under `added12`) cover the four r2 reproductions and run once per mode inside this review's 67-case `run_focused.py` suite.
- NFS41-R2-1 (history creation/observation boundary, unrepairable missing history): CLOSED — no replay/multiplication, canonical per-op history preserved.
- NFS41-R2-2 (exact earlier predecessors replace broad intra-state/state_inputs): CLOSED — unrelated OPEN/LOCK/LOCKU prefix cannot authorize an unobserved nonzero request; only exact earlier outputs do.
- NFS41-R2-3 (delayed READ/WRITE keeps issue context not newest reduced state): CLOSED — valid delayed WRITE after upgrade/open-CLOSE/child-lock-CLOSE and stale-at-issue rejections all pass in both modes (reviewer-authored `run/history_probes.py`).
- NFS41-R2-4 (zero-seqid CLOSE retains observed causal deps + transitive chain): CLOSED — new zero-close waits-for-pending-opens and prior-chain subcases pass.
- NFS41-1..NFS41-5: each CLOSED scoped unchanged from r2 review (offline scope only).
- NFS41-6: BLOCKED OFFLINE at the same-compound child-lifetime defect. No offline P2 acceptance from this cycle.

## FIN.5 Bounds and history budget
- `Limits` diff from r2 = only the new `history_links: 65536`; canonical operation_records, FH targets, `states`, `pending`, and compound-op arrays all keep their r2 values (verified by reading the frozen r2 and r3 provider `Limits` sources and comparing field-by-field).
- Sticky/`ObserverCapacity` behaviors confirmed for the unresolved-requires chain and link-budget bounded admission (no partial-success final evidence in either mode).
- `ReplyPending` remains incomplete: unresolved required relations still block finish and pending joins.

## FIN.6 Exact executed commands and counts (exactly once per mode unless marked "retained")
All candidate/provider modules imported under the review-root guard runner; normal uses `-B`, optimized uses `-B -O`. No source file was edited; `run/` is byte-stable.

```text
cd review-root/run
python3 -B [-O]  run_focused.py            # 67 pass, 0 errors, 0 skipped
python3 -B [-O]  public_probes.py         # 24 pass, 0 errors, 0 skipped
python3 -B [-O]  supplemental_probes.py   # 8 pass, 0 errors, 0 skipped
python3 -B [-O]  adjacent_probes.py       # 12 pass, 0 errors, 0 skipped
python3 -B [-O]  close_control.py         # 1 pass, 0 errors, 0 skipped
python3 -B [-O]  history_probes.py        # 4 IDs, 10 subcases; 8 pass, 2 fail (READ + WRITE), 0 errors/skips, exit 1
cd review-root
python3 -B  verify_review.py before        # package/archive/preimage/reference seal comparison (1082/1083 + prior groups)
python3 -B  verify_review.py after         # identical record, full before/after equality
python3 -B  summarize_results.py           # validates counts/controls/observations/RFC excerpts + result-summary.json pin
python3 -B  final_seal.py verify-preservation  # full record comparison to before.json = PASS + final-verification.json
python3 -B  final_seal.py seal | verify-seal   # seal and read-only re-verify of REVIEW-MANIFEST.json
```
Per-mode totals: 112 required root-suite cases (67 owned + 24 public + 8 supplemental + 12 adjacent + 1 close-control) with zero errors and zero skipped; the narrow history suite adds 4 test IDs (10 subcases total, 8 pass / 2 fail). All failing subcases come from `test_child_consumption_after_compound_close_is_rejected` (opcode=25 and opcode=38) and all failed values are pre-serialized in `run/history-observations/` JSON. `final_seal.py`/`summarize_results.py`/`verify_review.py` steps do only local file reads/hashes/record comparison — no candidate execution and no network or child-process control.

## FIN.7 Runtime vs retained provenance
- All 67+24+8+12+1 suite runs and the 4-ID history runs above are fresh independent runs made by this review, executed one time per mode.
- The retained red13 suites (original r2 review `run/adjacent-*` outputs and the author r3 `before/` directory interior red evidence) are pinned and byte-compared but never rerun: red13 = 8 pass / 3 fail / 2 WireError per mode.
- The candidate archive and the r2 review/author retained archive are verified through `members()` plus internal `pin()` equality and a full archive-path walk count.

## FIN.8 Preservation verification (this cycle's own step)
- Package/archive/preimage/reference comparisons across r3 candidate (1082/1083), r2 package and archive (386/387), r2 review (151), old author package (37/38), review1 (88) and prequota (277) plus retainer-copy equality, all match their saved pins — zero drift.
- `final-verification.json` (exit=0) records member_counts, primary pins, added12, delta_scope, closure/field/reference counts, stopped-verifier SHA pin, per-group before/after pins and the exact comparison graph used.

## FIN.9 Model/provider handover facts (this cycle only)
- 2026-10-26 this finalization continuation runs as `opencode/glm-5.3-flash-free#max` per parent model routing; the prior Astra continuation ended with a provider cyber_policy error, and an earlier Z.AI GLM attempt failed with insufficient balance.
- No source, test-count or scope change follows from the model change. All numbers in this report are taken from recorded artifacts (`before.json`, `after.json`, `result-summary.json`, `final-verification.json`, `run/*` outputs) — no estimate lines anywhere.

## FIN.10 Remaining / not in scope
- Seal REVIEW-MANIFEST.json has been built and re-verified read-only (see log entry with the manifest pin); this decision is now closed from this reviewer's side.
- Fixing the defect itself belongs to a later author cycle; this reviewer owns no fix, no Git write, no source/topology edits, and no candidate promotion.
- No live cluster access, no mounted or socket-based checks, no provider upgrades, no delegation beyond this single deadline, no change to connected parties.
Log:
- [2026-10-26] Created this ledger first. Prior r2 evidence and verdict remain unchanged.
- [2026-10-26] Read the full253-line handoff, primary correction ledger, all four post-archive documents and completed r2 review. Created authorized exclusive workspace root /mnt/workspace/astra-g3-wire-nfs41-r3-review-q7w79itj because of known temporary quota limits. Retained one new verifier schema stop before scratch creation/import/discovery: the r2 registration key differs from r3. The stopped verifier and explanation remain sealed inputs. Corrected schema comparison now verifies the exact retained independent59 zero-mask map.
- [2026-10-26] verify_review.py before passes1082/1083, six primary/copy pins, old37/archive38/review88/prequota277/r2package386/archive387/review151, ten references, six preimages, frozen ledgers/contracts and the old zero-byte partial. Four post-archive files and r2 receipt04cb4790 remain separate. Exact four-path delta regenerates; all55 old test bodies match,12 IDs are additive, closure17/snapshots4 and all old Limits match. Only history_links65536 is added. Fresh run/ preserves all five modules/guard/input bytes; only scratch_root plumbing changed.
- [2026-10-26] Ran all ten required commands once:67 owned,24 public,8 supplemental,12 adjacent,1 CLOSE control per normal/optimized mode. All exit0, zero failures/errors/skips. Pre-import guards and helper67bac6ed only. Author and old red13 results remain retained, not new independent executions. Narrow source review identified a concrete child-lock lifetime check to probe after an earlier CLOSE in the same compound.
- [2026-10-26] Resumed after a response interruption. Confirmed history_probes.py existed and no history-* outputs existed. Ran it once normally and once optimized. Four test IDs/ten labelled subcases per mode: three IDs/eight subcases pass; one ID/two rejection subcases fail, zero errors/skips, both commands exit1. The failed READ/WRITE cases return terminal success after same-compound CLOSE with lock-owner/inode evidence. Valid before-CLOSE controls pass. Delayed WRITE after upgrade/open-CLOSE/child-lock-CLOSE and the corresponding already-stale rejections all pass, with exact same per-stream RPC bytes across issue-order controls. Retained all synthetic inputs and reports. No completed required suite reran.
- [2026-10-26] summarize_results.py passed once (exit0): validates 12 result-file pairs, exact two-failure subcases per mode, retained red13 pins from both the original r2 review root and the author r3 before-root, per-stream RPC equality for the three WRITE controls, 20 malformed/success observation files, and RFC8881 excerpts. result-summary.json SHA256 `542c0e35c7d220e4db84dfb20027f00533cacbe1490d3212fdd86832e63f60be`, 81728 bytes.
- [2026-10-26] final_seal.py verify-preservation passed once (exit0): full record comparison with before.json/after.json, r3 candidate 1082/1083, r2 package 386/387, review2 151, old author 37/38, review1 88, prequota 277, 10 references, 6 preimages, all primary candidate pins, delta/regenerated scopes (4 outputs), Limits, closure17 texts, field/registration counts, and stopped-verifier SHA. final-verification.json and result-summary.json are byte-stable; before == after verification equality. Writable review root has no symlinks and no non-regular entries. No candidate file or prior review artifact changed. Finalization pending only the REVIEW-MANIFEST.json seal.
- [2026-10-26] final_seal.py seal built REVIEW-MANIFEST.json once: SHA256 `09d2ec01c34d835dad4d0609bf0b72616b1c21c8538f5a01e7d3749244d4ade0`, 41508 B, mode0664; 196 artifacts, 4136869 B, all mode0664, no symlink, no non-regular entry, manifest members = the complete exclusive review root excluding REVIEW-MANIFEST.json itself. final-verification.json SHA256 `3fb69498b9e9bccc9532d9b3f3999f21ea2ab7f20532710e12b093293f050385` (9729 B) is included in the sealed tree. Read-only re-check `verify-seal` match count 196/196; all file byte/mode pins equal after the build; no inventory drift, no missing entries. Review decision and scope recorded above (FIN.1..FIN.10). No more candidate/provider execution steps are owned by this reviewer.
