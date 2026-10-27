# glm-g3-reset-uuidopen-binding

Task: NEW offline-core assignment. Design and implement the binding of deterministic fault windows
to the RESET-reclaim and transport UuidOpen/OPEN timing seams in the owned offline fault core
(`scripts/g3_fault_*.py`), per the accepted fault-v1 interface and the provider plan P5 row. A
fault action targeting the RESET-reclaim path and one targeting the transport-OPEN/UuidOpen window
must each bind to the exact observed wire/event identity (uuid/boot/open epoch) fault-v1 requires,
with no cross-identity misattribution; delayed/replayed observations must resolve to the correct
window; reader-lock r3 ownership rules (outside-condition decode, atomic publication, bounded
metadata) stay preserved; the P4 NativeEvidence boundary must still refuse fixture→native upgrades
(tests prove the binding cannot synthesize native provenance). Keep every accepted scope:
action-transition ASTs, limits, sanitization, guards-before-imports. No seam edit to
`scripts/g3_fault_trial.py` beyond what the binding strictly requires; if the qualified-native
route is needed, STOP and report. No daemon/code hooks, no Python provider/P1-P4 edits, no Git
writes, no live/installed/capture/mount/keys/signals. Verification extends owned core tests
red-first where misattribution is possible (wrong uuid, replayed OPEN, RESET after reclaim
boundary, cross-boot identity), plus limit+1 rejections and sanitization, and reruns the existing
reader-lock r2/r3 correction suites EXACTLY as established (exact copied bodies, archived helper
only, guards before imports, normal+optimized modes, once each). Freeze: one-shot
manifest/archive/HANDOFF with all input pins INCLUDING the full transitive import closure
(devcluster/mount_guard.py, scripts/recovery_directory.py, scripts/restart_history.py pinned
explicitly). Return AUTHOR COMPLETE/FROZEN with exact pins and per-mode counts, or the exact
stopped blocker. Independent review follows; no self-review. External root:
`/tmp/opencode/glm-g3-reset-uuidopen-*` (EDQUOT fallback `/mnt/workspace/glm-g3-reset-uuidopen-*`);
retain failed attempts.

Status: AUTHOR COMPLETE/FROZEN for independent review. Implementation: `SeamBinding` appended
to `scripts/g3_fault_protocol.py` (31 pre-existing top-level definitions AST-identical to the
sealed r3 pinned source; exactly9 added definitions; `g3_fault_trial.py`/`g3_fault_peer.py`/
`g3_fault_client.py` byte-unchanged vs r3 — zero seam edits). New owned test module
`scripts/tests/test_g3_fault_seam_binding.py` (38 tests). `scripts/G3_FAULTS.md` documents the
API/limits. Verification once each per mode: NEW suite38/38 exit0 both modes; established r3
suites from a fresh exclusive relocation of the exact harness bytes with the pinned source
byte-verified27 (focused109/109, probes16/16, supplemental8/8, joins_refined3/3, adjacent6/6 —
both modes, exit0); live-tree regression corrections41/41 and protocol19/19 both modes exit0.
full275 NOT rerun BY DESIGN: the r3 full verifier pins the current tree at its194 r3 inputs
(ROOT=/mnt/workspace/rust-mds; its protocol.py pin16d60af4≠current d934475b after this
authorized edit) — retained as r3 source-specific proof (same discipline as r2's full270 in r3).
Native boundary proven: receipts cannot enter peer.Evidence as native (origin/format refused),
native_pass() still raises Prerequisite, Proof.native default False. Freeze at
/mnt/workspace/glm-g3-reset-uuidopen-g343g8to/freeze/: manifest15 pins +2 extra (contract
3cf2dcca…6a52d523 + r3 pinned source), mechanically computed13-file transitive import closure
EXPLICITLY including devcluster/mount_guard.py, scripts/recovery_directory.py,
scripts/restart_history.py (P4 pinlist gap closed), archive f7c1b4198656a94080eb2edad6c83afeb835f
34db756a28db942f9ff5e16adcf (25 members), HANDOFF.md. /tmp/opencode hit EDQUOT (Errno122) at root
creation — authorized /mnt workspace fallback used; explicit /tmp/opencode temp requests in live
runs relocated via the established mkdtemp patch (receipts retained); failed relocation attempts
(green-rerun, green-rerun2, quota-stop-1/2, wrapper-fix-1) all retained.

Context: fault-v1 interface verified at exactly
3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523 (the `6a52` spelling; the
`6c52` dispatch typo exists nowhere here). Provider plan011a7e1d4c6232f193d9bde63061f31d171072
2c0ece1461e4221c4ceff64d13, reader-lock r3 ledger e198aa6f9f3a3fe97ded3e1538125865b53a765a9d5a
2326b3422c339f27342c, controller plan1ea96d6f1bfe72838dc2d49de444a4cad138f3b4a3d4e9fc6ee672106d
e5a67f, g3-native-evidence 52aaecbf447b84e407e6af8235eb71afebe6d4c7bdcb485d1a4165c2af7a4a61,
P4 review8afd9cb15122a100e0277f21dc5454a16fd7f6dc075a2a5f324cce090fb14a7d all read before work.

Reading list (before any source work):
- `current/astra-g3-fault-interface.md` — verify SHA256
  `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523` (parent dispatch previously
  typo'd `6c52`; the pinned spelling here is `6a52`).
- `current/astra-g3-provider-plan.md` — RESET-reclaim / transport-OPEN-timing / UuidOpen
  requirements and the P5 row.
- `current/sol-g3-fault-reader-lock-r3.md` — completed r3 package sealed at manifest92970520 /
  archive370f247d (my own authorship; re-read for exact scopes).
- `current/sol-g3-fault-controller-plan.md` — consumer plan.
- `current/g3-native-evidence.md` and `current/glm-g3-native-p4-review.md` — P4 NativeEvidence
  boundary the binding must feed; note the flagged import-closure pin gap.

Next: IMPLEMENTATION IN PROGRESS. Design (cited to the accepted docs; fault-v1 =
`current/astra-g3-fault-interface.md` at 3cf2dcca…6a52d523):

- Gap: the trial Controller (`g3_fault_trial.py`) models Reset/UuidOpen discovery HOLDS
  (ResolveSlot/SessionBound/check_discovery/check_observation/execution_allowed,
  lines 102–117/484–511/468–473) but no component resolves a fault action targeting the
  RESET-reclaim path or the transport-OPEN/UuidOpen window to the exact observed wire/event
  identity. That missing layer is the recorded open item in the r3 ledger (line 109), the P4
  review scope (line 157) and G3_FAULTS.md line 99.
- New pure component in my owned `g3_fault_protocol.py`: `SeamBinding` (working name) resolves
  both seams from already-decoded validated envelopes (`envelope(loads(raw))` output). It never
  touches Channel/BufferedChannel — r3 reader-lock invariants stay byte-untouched; it holds
  bounded metadata only (≤8 live windows, ≤64 retained observations per window, limit+1 refuses
  before allocation).
- RESET-reclaim window (fault-v1 §4.4 Reset + §5.4 Reclaim): opened by exactly one
  post-installation `Reclaim{stage:Received, flags&1, uuid}`; identity = (reclaim, input,
  requester=temporary key, attachment, uuid, flags, old_target). Window stays open only while
  the reclaim is at Received — before `d6::reclaim` executes (Selected). Binding requires the
  exact old_target retained-key match and same-boot evidence; requester/attachment generation
  change refuses.
- transport-OPEN/UuidOpen window (§4.4 UuidOpen + §5.4 SessionIdentity/Output Open): opened by
  exactly one post-installation authenticated `SessionIdentity{op:0 OPEN, uuid}`; binding
  becomes eligible only after (a) predecessor Reset slot bound (its SessionBound observed —
  "becomes eligible only after that predecessor binds") and (b) checked durable OPEN completion
  (`Journal{EngineCommitted, input}`). Identity = (uuid, key, attachment, output, open epoch =
  Open summary seq, origin, final_chunk). The typed OPEN output must be held at
  `Output{Queued, Open}`; TransportAccepted closes the window.
- Replay/delayed resolution (§4.4 "Duplicate wire delivery is not a new candidate"):
  observations dedupe by identity — a duplicate Reclaim-Received/SessionIdentity-OPEN resolves
  to the SAME window; a new input after the window closed (Selected observed / OPEN
  transmitted) refuses ("RESET after reclaim boundary" / closed window), never rebinds.
- Cross-identity refusal set (exact Invalid reasons): wrong uuid; ambiguous two distinct
  post-installation candidates (never choose-first, §4.4); pre-installation observation
  (event ≤ installed); cross-boot identity (observed boot ≠ run boot; §4.1/§6.1 boot field);
  unbound predecessor; missing durable completion; held OPEN transmitted; generation change;
  old_target mismatch; capacity limit+1 before allocation.
- Native boundary (P4 review claim 10; plan §2 "No constructor option turns a fixture into
  native proof"): receipts carry `provenance:"offline-model"` and no native flag at all; tests
  prove `g3_fault_peer.Evidence` still only accepts `g3-offline-peer-fixture-v1` origin=fixture,
  `Proof.native` stays False and `native_pass()` still refuses for any binding-derived input.
- Zero `g3_fault_trial.py` edits: the resolver is a pure protocol-layer component the
  trial/P4/P5 can call; no seam change is strictly required.
- Doc: `scripts/G3_FAULTS.md` gains the binding API/limits subsection; the "complete N-case
  support" caveat stays (offline binding ≠ native/N-case qualification).

Implementation plan: edit `scripts/g3_fault_protocol.py` (append component; no existing line
changes), new owned test module `scripts/tests/test_g3_fault_seam_binding.py` (red-first
misattribution cases, limit+1, sanitization, native-boundary), update `scripts/G3_FAULTS.md`,
run new suite + established r3 suites both modes once each, freeze with full transitive import
closure (devcluster/mount_guard.py, scripts/recovery_directory.py, scripts/restart_history.py
pinned explicitly).

Next: None for this owner — AUTHOR COMPLETE/FROZEN. Parent assigns independent review.
Preserve everything frozen above; no rerun of passing suites; the qualified-native route
remains a separate reviewed assignment (not needed and not used here).

Log:
- [2026-10-27] Ledger created first. Nothing else written.
- [2026-10-27] Read all six required docs (interface pin verified at the canonical 6a52 hash). Design recorded above with citations before implementation.
- [2026-10-27] Implemented SeamBinding in scripts/g3_fault_protocol.py (append-only; AST invariance vs r3 pinned source proven: 31 unchanged/9 added/0 changed/0 removed; trial/peer/client byte-unchanged). New 38-test module + G3_FAULTS.md doc rows.
- [2026-10-27] /tmp/opencode EDQUOT at root creation → authorized /mnt/workspace/glm-g3-reset-uuidopen-g343g8to fallback. Two early harness-relocation failures (pin-key prefix; runners layout) retained in green-rerun/green-rerun2; clean green-rerun3 (pins relocated, source byte-verified 27).
- [2026-10-27] All suites pass once each per mode: new38/38; established focused109/probes16/supplemental8/joins_refined3/adjacent6 (pinned source); live corrections41/protocol19. full275 deliberately not rerun (tree-pinned r3 verifier; documented). Live-run /tmp quota stops retained (failed-attempts/quota-stop-1,2; wrapper-fix-1) before the TMPDIR/mkdtemp relocation fix.
- [2026-10-27] FREEZE complete one-shot: manifest (15 pins + closure13 incl. the three named files + contract + r3 source), archive f7c1b419… (25 members), HANDOFF. AUTHOR COMPLETE/FROZEN; independent review follows; no self-review.
