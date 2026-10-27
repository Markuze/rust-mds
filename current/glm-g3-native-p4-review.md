# glm-g3-native-p4-review
Task: NEW independent P4 evidence/cases review (offline milestone). Verify the frozen author claims against actual bytes: freeze/accounting, §6 source-fact retention, provenance boundary, frozen call shapes, finite admission limits and sanitization, all-13 case predicate semantics, fresh independent execution of both suites once per Python mode, and reconciliation honesty (interface-hash dispatch typo, freezer stdout-parse stop, deleted partial receipt pair). Return SCOPED ACCEPT (offline P4) or ranked CHANGES REQUIRED with sealed review artifacts.
Status: COMPLETE — SCOPED ACCEPT (offline P4 milestone). All hard scope requirements verify against actual bytes: complete §6 source facts retained in a new provider-local record union (not manufactured into the frozen FACTS), unbeaten provenance boundary, exact frozen call shapes, admission limits firing before per-record validation, strict-schema sanitization rejecting every forbidden-bytes attempt, all-13 case predicates with 2/77/1 semantics and native_fault_case_pass always false. Fresh independent execution on a pinned archive copy: 51+51 evidence and 34+34 cases, rc0 both Python modes, zero process/signal/network events, no bytecode, no copy drift. Two non-blocking P3 qualifications recorded (projection "lossless" overstatement; freeze pin-list transitive gaps). Sealed review inventory in the exclusive root.
Context: Owned only this ledger and `/mnt/workspace/glm-g3-native-p4-review-90vhy9cq/` (the first `/tmp/opencode/glm-g3-native-p4-review-bb00tdgr` attempt is retained read-only after EDQUOT; its partial outputs are superseded by the workspace root). Author root `/tmp/opencode/glm-g3-native-p4-80nUG3Cq` (HANDOFF 6e3b342b…, MANIFEST 6c697899…, archive 68f7859b…, freezer 7dd44915…) verified untouched. No owned source/test/fixture/ledger edited, no Git writes, no live/installed/capture/mount/keys/signals/daemon/seam actions, no active-output reads (`current/glm-e1-d5-combined-qa.md` untouched). Offline review only.
Next: Parent records this scoped acceptance. Native/live/installed qualification, P5 scheduling, seam changes and Reset/UuidOpen binding remain separate unfinished work; the two P3 qualifications belong to a later documentation/accounting pass, not a predicate change.
Log:
- [2026-10-26] Created this ledger before any other write.
- [2026-10-26] Read the full plan (P4 row, §2 seams/bounds/sanitization, §7 case table), fault-v1 interface (§3 digest, §5.4 event kinds, §6 source facts, §6.3 predicates, §7 case IDs), frozen author ledger, HANDOFF/MANIFEST/freeze.py, and both owned modules plus both owned test modules end to end.
- [2026-10-26] Provenance verification exit0: 32 archive members re-read pin-equal (31 files + MANIFEST itself), owned repo pins match disk and archived mirror, 18 fixture pins match disk and the pins embedded in the owned test, 10 HANDOFF import pins match disk, fault-v1 at canonical 3cf2dcca…, core chain (protocol/peer/client/trial + G3_FAULTS.md) byte-identical to my independently sealed reader-lock r3 archive, ledger packaging-intent snapshot preserved verbatim. Extracted a 36-file pinned execution copy.
- [2026-10-26] Fresh guarded execution once per mode: both suites rc0 (evidence 51/51, cases 34/34), guards installed before candidate imports, zero audit-hook events, no __pycache__, no pinned-copy drift. Adversarial probe battery (38 probes) all pass. Fixture regeneration check: all 16 data fixtures rebuild byte-identically from the committed builder. Final verification exit0: every pin group, the author root and the repo owned/imported files unchanged.
- [2026-10-26] Sealed REVIEW-MANIFEST/PACKAGE + handoff in the exclusive root; recorded SCOPED ACCEPT with two P3 qualifications. Seal: 60 files, manifest `03eff67c59e00815e7229108fda3c74e8fa2bbdbf009e3bd0df844706472f643`, package `237d4faeb9c641e4338bbca09f8a2ebaf642bb3156059e4411d7c15ef1850d51`, report copy `292bfe5e199b87687f608d0e60cda4e1bc1494cace2083111f8b8564765e4e79`.

## Decision

**SCOPED ACCEPT — offline P4 evidence/cases milestone.** The author's claims
verify against actual bytes with two qualifications that do not affect any
predicate, boundary or accepted prior scope. No CHANGES-REQUIRED finding.

Ranked non-blocking qualifications (P3, documentation/accounting accuracy;
assign to the author for a later doc pass — no predicate change authorized):

1. **P3 GLM-P4-PROJ-CLAIM — `from_ceph_summary` is §6-complete, not field-lossless.**
   Verified against the real `g3_wire_ceph.decode_front` return fields
   (`g3_wire_ceph.py:260–408`), not the author's synthetic test summary. Every
   §6.1-required fact is retained (probe `projection_retains_all_six_facts`,
   empty misses), and Reclaim correctly refuses projection. But the handoff/
   ledger/test name "lossless" overstates: the Request projection drops
   `epoch, forward, ino, caller_uid, caller_gid`; the Caps projection drops
   `caps, wanted, snap_follows, oserrno`; Reconnect reduces the `caps`/`realms`
   entry lists to counts. None of these is a §6-required source fact, so no
   evidence value is lost — the *word* "lossless" is what fails verification.
   Also note `ReclaimReply` summaries have no projection route at all; reclaim
   records are assembled directly by the integrator/builder (which the fixtures
   do). Correct the claim wording, not the code.
2. **P3 GLM-P4-PINLIST-GAP — the frozen import pin list is not self-contained.**
   Suite execution transitively imports `devcluster/mount_guard.py` (via
   `recovery_trial`, which inserts `<root>/devcluster` on sys.path) and
   `scripts/recovery_directory.py` + `scripts/restart_history.py` (via the
   pinned `g3_fault_client`). None is in the HANDOFF's ten import pins. My first
   execution attempt failed on exactly these until I copied and pinned them
   separately (`unpinned-transitive-copy.json`). The author's retained receipts
   ran against repo files outside the pin set. Freeze-integrity of the ten
   pinned modules is not affected; a future freeze should pin the full
   transitive closure.

## Per-claim disposition

| # | Author claim | Disposition | Evidence |
|---|---|---|---|
| 1 | MANIFEST 32 members incl. itself; archive member-by-member pin-equal; owned repo pins match disk | **VERIFIED** | provenance.py: 31 file entries + MANIFEST = 32; every member re-read pin-equal; 4 owned + ledger + 18 fixtures match disk and mirror |
| 2 | Imported read-only modules pinned and unchanged | **VERIFIED with P3-2** | 10 HANDOFF import pins match disk; protocol/peer/client additionally byte-identical to my sealed r3 archive (core chain incl. `g3_fault_trial.py` seam and G3_FAULTS.md unchanged since r3); 6 provider files have no independent prior pin from my seals — P4 is a different agent from their P1/P2/P3 owners; 3 transitive deps missing from the pin list |
| 3 | fault-v1 pinned at canonical hash | **VERIFIED** | `current/astra-g3-fault-interface.md` == 3cf2dcca…; freezer hard-codes the canonical spelling |
| 4 | §6 msgr header+connection on every WireCeph1 record | **VERIFIED** | `_check_record` requires both non-null and stream agreement; negative: dropping header rejected; NFS record carrying msgr identity rejected |
| 5 | §6 CAPS size/max_size/truncate_size/truncate_seq (+realm, oldest_flush_tid) | **VERIFIED** | new CAPS body carries all; `(size is None)==(truncate_seq is None)` completeness rule; A4 predicate requires them populated |
| 6 | §6 SESSION op/seq/uuid/timeout/max_caps/max_leases; reconnect final/more+counts | **VERIFIED** | SESSION body complete; `final == not more` and table bounds enforced |
| 7 | §6 full reclaim addresses ≤64 with interface-§3 digest and Success/ENOENT rules | **VERIFIED** | `reclaim_digest` = SHA256 over `p.address_bytes` concatenation (the exact §3 encoding); duplicates rejected; Success needs result0/epoch>0/nonempty; ENOENT needs result0/epoch0/empty; 65-address union rejected; digest tamper rejected |
| 8 | §6 NFS joins incl. RFC8881 no-owner/no-handle RECLAIM_COMPLETE | **VERIFIED** | stateid-present-iff-OK on Open/Lock; ReclaimComplete must carry `one_fs` and must NOT carry owner/handle/stateid — invented-owner variant rejected |
| 9 | Facts retained in the new artifact format, NOT manufactured into frozen RECORD/FACTS | **VERIFIED** | new `NATIVE_FACTS`/`NATIVE_RECORD` provider-local union; frozen `peer.RECORD/FACTS` and both action-transition ASTs byte/AST-identical to sealed r3 |
| 10 | No constructor option flips fixture origin to native; `native_pass()` refuses; `manifest_complete()` distinct from `qualified()` | **VERIFIED** | constructor signature is exactly `(raw, expected_sha256)`; origin=fixture → manifest_complete False; origin=native → manifest_complete True but qualified False (empty `QUALIFIED_PROVIDERS`); native_pass raises Prerequisite for both origins; no `native=True` assignment exists in the module; produced Proofs stay `native=False` |
| 11 | Exact frozen call shapes; source supplies boot/proof_digest/check_persisted | **VERIFIED** | inspect signatures equal `binding(self, bound, actual)`, `prekill(self, actual, attachment, slot, at, competitor=False)`, `recovered(self, actual, slot, after, target=None, result=None, resend=None)`; attrs present and callable |
| 12 | Interval ordering `observed+error < observed-error`; overlap is unproved Prerequisite, never `clock_error_ns=0` | **VERIFIED** | `proved_before` strict math; overlap AND touching intervals both raise Prerequisite in `_records_before`; no zeroing assignment in the module; `interval_of` requires positive error |
| 13 | Limits fire before per-record validation/allocation | **VERIFIED** | records/sources/daemon-event caps checked before any `_check_record`/receipt validation — probe uses invalid content and gets the capacity message, not the content error; address/array and NFS-table bounds reject at validation of the offending record (content-dependent bounds; JSON parse allocates upfront by necessity — earliest post-parse point) |
| 14 | Sanitization: strict schema proves no raw packets/authorizers/credentials/file-or-xattr/secret debug can persist | **VERIFIED** | 8 injection attempts (record raw_packet/authorizer blob/free-text/xattr, envelope raw dump, source secret note, header payload, non-hex proof) all rejected Invalid; whitelist scan in-suite over committed bytes; builder regenerates all 16 data fixtures byte-identically — nothing hidden |
| 15 | Cases: all-of predicates, missing→2, fixture-complete→77/OFFLINE_VALIDATED, wrong outcome→1, `native_fault_case_pass` always false | **VERIFIED** | `_Checks` all-of rows; independent probes confirm B1-handed→1 with exact detail, B1 encoded-link break→1, B2 cover-accepted→1, N3 identity collapse→1, N4 NO_GRACE→1, B3-on-a0→PREREQUISITE_MISSING/2; every 77 result carries `native_fault_case_pass: False` |
| 16 | Structurally hard cases correct in source | **VERIFIED** | B1: never-handed Positions (worker_enqueued/append_started/store_confirmed false, submitted==engine_durable+1) linked to Encoded-only CommitRef chain + exact NoEntry StartupPreResend; B2: Present snapshot, Coverage exact≥1/cover_accepted=0/max_ack<tid, FlushHistoryHit, full five-stage chain; N3: embeds B1 and B2 (each must exit 77), two NFS identities/inodes, both outstanding at kill (no pre-kill exits); N4: DENIED required, NO_GRACE explicitly rejected, ordinary grace on separate inode; grace matrix shared N1/N2/N4 |
| 17 | Suites 51+51 and 34+34, rc0, once per mode, guards before imports | **VERIFIED fresh** | see execution table |
| 18 | Interface-hash dispatch typo reconciliation | **VERIFIED** | canonical repo hash is what the freezer pins; the author ledger retains the first-stop note; the dispatch spelling exists nowhere in frozen artifacts |
| 19 | Freezer stdout-parse stop + deleted partial receipt pair | **VERIFIED with qualification** | see reconciliation conclusions |

## Fresh vs reused evidence

| Evidence | Source | Result |
|---|---|---|
| Suite runs (evidence 51, cases 34) × normal/optimized | **FRESH** — my pinned archive copy, `/usr/bin/python3 -B`, guards before imports | rc0 ×4; zero audit events; no `__pycache__`; no copy drift |
| 38 adversarial/semantics probes | **FRESH** | all pass (details in `probes.json`) |
| Fixture regeneration determinism | **FRESH** | 16/16 byte-identical rebuilds |
| Final pin verification | **FRESH** | all groups, author root, repo owned/imported files unchanged |
| Author's four retained receipts/logs | REUSED (read-only) | internally consistent; superseded by my fresh runs |
| Sealed reader-lock r3 archive (core-chain comparison) | REUSED (my prior sealed evidence) | protocol/peer/client/trial + G3_FAULTS.md byte-identical |
| P1–P3 provider-file bytes | HANDOFF pins + disk only | no independent prior pin of mine; P4 author ≠ their owners |

Exact fresh commands (from the review root `R=/mnt/workspace/glm-g3-native-p4-review-90vhy9cq`):

```sh
PYTHONDONTWRITEBYTECODE=1 python3 "$R/provenance.py"      # freeze/pins/copy extraction
PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -B "$R/run-suites.py"          # normal: 51+34 rc0
PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -O -B "$R/run-suites.py"       # optimized: 51+34 rc0
PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -B "$R/probes.py"              # 38/38 pass
PYTHONDONTWRITEBYTECODE=1 python3 -B "$R/regen-check.py"  # 16/16 deterministic
PYTHONDONTWRITEBYTECODE=1 python3 -B "$R/verify-final.py" # all pins stable
```

Receipts: `fresh-run-{normal,optimized}.json`, `test_g3_native_*-{normal,optimized}.log`,
`probes.json`, `regen-check.json`, `final-verification.json`.

## Adversarial probe outcomes (selection)

- **Provenance upgrade attempts:** origin rewrite to `native` yields
  manifest_complete=True but qualified=False; `native_pass()` raises
  Prerequisite for fixture AND native origins; constructor offers no flag;
  no `native=True` assignment exists; binding on a native-origin artifact
  still returns `Proof.native=False`. The boundary held every attempt.
- **Forbidden-bytes payloads:** eight injections (raw packet b64, cephx
  authorizer blob, secret debug text, xattr value, envelope raw dump,
  key-looking source note, header payload, non-hex proof) — all rejected
  `Invalid` by the strict schema. Nothing sanitized can persist.
- **Interval adapter:** exact ordering true; overlap false; **touching
  intervals (a+b == c-d) also false** and raise Prerequisite in prekill — the
  conservative reading. No clock-error zeroing anywhere.
- **Limit ordering:** records-4097/sources-9/daemon-257 with deliberately
  invalid content all produce the *capacity* message, proving the check
  precedes per-record validation; 65-address union and the 513th NFS row are
  rejected at their bounds.
- **Red-first exactness:** B1 handed-to-worker fails with
  `selected append was handed to the worker`; B1 Encoded-link break fails
  `journal-encoded-only`; B2 cover acceptance fails `suppressed-through-seal`;
  N3 identity collapse fails `two-nfs-identities`; N4 NO_GRACE fails
  `reclaim-lock-denied`; A0 reply-before-kill raises exactly
  `peer received selected reply before crash`. Missing prerequisites give
  PREREQUISITE_MISSING/2. Committed-byte pins are real: one flipped fixture
  byte changes the pinned digest.
- **Worker receipts in the new class:** mutating `worker_records` after
  attachment is rejected with `decoded worker facts changed after source
  validation` — the r2 FC45 binding carried over.

## Reconciliation conclusions

(a) **Interface-hash dispatch typo.** The canonical repo hash
`3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523` is what the
freezer pins and what every frozen artifact references; the one-character
dispatch spelling exists only in the assignment text and the author's retained
ledger note. Verified as claimed. Physical evidence of the first freeze stop on
the dispatch spelling: ledger note only — no artifact, which is acceptable
because the successful freeze is fully evidenced.

(b) **Freezer stdout-parse stop and deleted partial receipt pair.** The author
root contains exactly the four clean-rerun receipt/log pairs plus
HANDOFF/MANIFEST/freeze.py/archive — no partial pair survives. The physical
evidence of the stop is the ledger narrative only. The retained receipts are
self-standing (`command`, rc, ran-count, `Ran N tests … OK` in each log), the
stderr-parse fix is visible in freeze.py source (`run_suite` parses
`proc.stderr`), and **no accepted claim depends on the deleted material**: my
fresh runs re-establish both suites' results independently. Qualified honest,
nothing relabelled.

(c) **Ledger packaging-intent snapshot.** The frozen ledger member (ff21a9be…)
is a mid-edit state (`Status: ACTIVE`); the live ledger completes the Status
line (adding the owned pins) and appends the freeze receipt plus both
reconciliation notes. The archive preserves the snapshot verbatim, the HANDOFF
declares "frozen at packaging intent", and the full diff is retained
(`ledger-postfreeze-diff.txt`). Owned-file pins inside the live ledger match
the manifest. No claim depends on the delta.

## Scope and limits

Offline P4 fixture milestone acceptance only. Native/live/installed
qualification, P5 scheduling, `g3_fault_trial.py` seam work, Reset/UuidOpen
binding, P4→P5 promotion and full G3 remain separate unfinished work;
`native_fault_case_pass`/`full_g3_qualified` remain false everywhere. Active
QA/release outputs were not read. The two P3 qualifications are documentation/
accounting corrections for a later pass; they authorize no code change here.

## Sealed review inventory

Exclusive root `R=/mnt/workspace/glm-g3-native-p4-review-90vhy9cq/` contains
this report's byte-identical copy (`report.md`), `HANDOFF.md`,
`REVIEW-MANIFEST.json` + `REVIEW-PACKAGE.sha256` (mirroring my r3 practice),
all scripts, receipts, logs, probe outputs, the pinned execution copy, the
ledger diff and the failed-attempt pointer. The superseded
`/tmp/opencode/glm-g3-native-p4-review-bb00tdgr/` attempt is retained read-only
(EDQUOT during copy extraction; its provenance.py runs before the quota stop
are re-established by the workspace root).
