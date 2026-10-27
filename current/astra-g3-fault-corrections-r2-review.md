# astra-g3-fault-corrections-r2-review
Task: NEW independent residual r2 correction review for G3-FC1-C1 and G3-FC45-C1. Verify the frozen package and retained full270 proof. Audit the two correction boundaries. Run independent guarded focused104 and exact copied16/8/3/6 suites in both Python modes. Return scoped acceptance or reproducible findings.
Status: COMPLETE — CHANGES REQUIRED. One P2 finding remains: G3-FC1-R2-LOCK, JSON parsing under the reader condition. Both original P1 reproductions now reject correctly. G3-FC45-C1 receives scoped offline closure. The FC1 readiness correction passes its original probes, but its assigned lock boundary is not complete. Independent focused104 and copied16/8/3/6 pass per mode. New lock2 gives1 pass/1 failure/0 errors per mode. Retained full270 proof verifies independently. All initial pins match after execution.
Context: Own only this new ledger and new exclusive external review artifacts. Candidate source/tests/docs, old reports and evidence stay read-only. A=/mnt/workspace/sol-g3-fault-r2-20261026.rb058gbv; F2=A/freeze. No fixes, Git writes, live/native/installed actions, cluster/network/process control or delegation. Guard all candidate imports. Use only disposable local file/socketpair/thread/action doubles. Full270 is retained sole-QA evidence to verify and reuse, not rerun.
Next: Parent assigns a narrow reader parsing-lock correction and independent delta review. Preserve this review, F2 and all previous evidence. No source correction or native action is assigned to this reviewer.
Log:
- [2026-10-26] Created the sole writable primary ledger before review edits.
- [2026-10-26] Created exclusive review root `/mnt/workspace/astra-g3-fault-r2-review-1yfba8j6/`. Read the frozen handoff, completed author ledger, prior review and parent assignment. Independently verified the exact12-member F2 archive and complete delta, old F/P packages,4,694 new proof pins,8,409 immutable old pins,194 qualified inputs,18 runtime inputs and784 retained references. Exactly four assigned source repins differ;190 qualified inputs remain unchanged. Verified eight daemon shape ASTs,three typed peer shape ASTs and22 unchanged prior correction test ASTs.
- [2026-10-26] Ran all ten prescribed guarded archive-copy commands once. Focused104 and copied16/8/3/6 pass per mode. The six adjacent probes now preserve the genuine controls and reject paused real-select EOF/bad-frame and mutated decoded worker receipts. Candidate/probe copies have33 initial pins. No full270 rerun occurred.
- [2026-10-26] Independently joined retained full270 test IDs, tickets, environments, process events and status evidence. Verified506 direct children on37 baseline IDs,31 gateways,27 waited leaves and346 fresh roots/relocations per mode. Verified unchanged policy and17 allow/6 deny controls. Retained full270 passes; it was not rerun.
- [2026-10-26] Added only two adjacent lock-boundary probes. Both modes give1 passing framing control and1 failed JSON-lock expectation,0 errors/skips. The failure uses a real socketpair and valid frame. No action executes. The parsing/publication block is AST-identical to F; this is a missed assigned boundary, not a new r2 regression.
- [2026-10-26] Saved-log audit completed on attempt3. Attempts1/2 stopped on reviewer lookup KeyErrors; their script preimages/stdout/stderr remain retained. Candidate tests were not rerun. All prescribed rejection reasons and controls verify without TypeError/AttributeError closure credit.
- [2026-10-26] Final verification matches12 candidate,8,409 immutable old,4,694 new proof,194 qualified,18 runtime and784 reference entries, plus33 copy pins. These groups overlap across13,889 original paths. The152 native files and preserved binary match as file evidence only. Decision and review inventory are sealed under the exclusive review root.

## Decision and scope

**CHANGES REQUIRED: P2 G3-FC1-R2-LOCK.**

- **G3-FC1-C1:** The original unread-readiness false-admission defect is fixed in the reviewed offline model. Both paused real-select negatives now prevent the action. The separate parsing-lock requirement remains open below. Do not accept the complete FC1 r2 scope yet.
- **G3-FC45-C1:** SCOPED ACCEPT for decoded worker/source binding and sticky failure in the reviewed offline core.
- Prior FC2/FC3 defect closures and FC6 closure retain their original offline scopes. Reset/UuidOpen provider binding remains incomplete.
- Full270 is verified retained sole-QA evidence. The104 and16/8/3/6 results are fresh independent archive-copy execution. Counts overlap.

Review root **R2**: `/mnt/workspace/astra-g3-fault-r2-review-1yfba8j6/`.

## P2 G3-FC1-R2-LOCK — parsing holds the reader condition

**Source:** `scripts/g3_fault_protocol.py:653–660`; admission at`:698–708`.
The candidate calls `envelope(loads(raw))` inside `with self.condition`.
It then calls `loads(raw)` for every queued frame under the same condition.
`Framer.feed()` only splits frame bytes; it does not perform JSON parsing.

This violates the assigned rule that parsing must run outside the condition.
It also contradicts `scripts/G3_FAULTS.md:136–140` and F2/HANDOFF`:51–57`.
Admission acquires this condition without a timeout. While the parser holds it,
admission cannot inspect `read_in_progress`, reject the owned read, or perform
the final deadline check. Frame limits bound the work, but do not move that work
outside the condition. This finding does not claim a new false signal or an
unbounded parser hang.

**Reproduction:** `R2/reader-lock-probe.py`, on the exact archive copy.
It sends one valid protocol Failure envelope through a real socketpair.
It pauses the reader at a bounded parser-entry barrier, then tries to acquire
the condition from the other thread for50ms. The parser wrapper calls the real
parser after release. It does not alter bytes or validation results.

| Observation | Framing control | JSON-parser probe |
|---|---|---|
| Paused function | `Framer.feed` | `loads` |
| `read_in_progress` | true | true |
| Other thread acquires condition | true | false |
| Admission rejects owned read immediately | yes | blocked on condition |
| Action called / reader error | no / none | no / none |

Both modes give **1 pass/1 failure/0 errors/0 skips**, exit1.
The exact failure is: `JSON parsing holds the reader condition; action admission cannot inspect read_in_progress until parsing returns`.
Instrumentation also observes envelope validation under the condition, followed
by a second JSON parse with `read_in_progress=false` while the condition remains
held. The held condition still prevents an admission gap at that point.

`audit-independent.py` confirms that this publication block is AST-identical
to old F. Keep this distinction: the readiness correction works, but the newly
specified parsing-lock boundary was not completed.

**Required correction:** Perform JSON decoding, envelope validation and ordinary
event classification outside the condition. Keep read ownership set until
frames/error and bounded queue metadata publish atomically. Preserve the clean
zero-time readiness probe and both nonblocking, non-reentrant callback contracts.
Use this failing probe and its passing framing control for the delta review.

## Semantic audit of the two corrections

### FC1 readiness and action admission

The final deadline/seal callback runs before the zero-time readiness probe.
The probe and `action_started` update share the reader condition. A reader
paused after real select has not consumed the socket data or EOF; the admission
probe detects them. Before recv, the reader sets `read_in_progress` under that
condition. It retains ownership through framing and publication/error handling.
No ownership gap permits the original false-admission reproduction.

The exact copied EOF/bad-frame probes now report no actions, sticky consumer
failure, `closed_expected=false` and `eof_after_action=false`. The EOF probe
confirms readable EOF before admission. The valid post-action EOF control
completes A0. NormalStop retains its separate ProcessExiting/ExitAck/status path.
Controller transcript, proof and ownership I/O precede action_transition.
Its final callback performs only state/deadline checks. The supplied action
remains a bounded offline double. The parsing-lock finding above is separate.

### FC45 complete decoded worker authority

`Evidence._worker_receipts` retains source path, digest, retained-file pin and
checked immutable raw bytes. Checks compare the complete source inventory,
recheck retained files, decode the retained bytes, validate every public row,
and compare complete canonical decoded sequences. Typed validation rejects
bool substitutions for integer fields. Equal detached list/dict copies pass.

The focused tests exercise list removal/reordering/replacement, result and
identity changes, nested names, source aliases, every attached source and
restoration after detected failure. Source/persistence/attachment exceptions
latch `Evidence.failed`. Restoring public fields does not heal that latch.
The exact copied mutated-worker probe now rejects with
`decoded worker facts changed after source validation`, while the raw receipt
files remain unchanged. Its matching A1 control passes. The unchanged receipt
with contradictory wire names still rejects at the argument/process join.

Proof checks after intent persistence, cumulative earlier-round verification,
and the post-`result.pending.json` fsync check all reach the source binding.
The caller still owns the documented final exclusive mutation interval through
validation/link publication. Limits remain8 files,4MiB per file and4096 rows.
This review makes no maximum-size throughput or memory-use claim.

RENAME destination/source roles, raw syscall return versus wire errno,
first-result causal joins, saved descriptors and typed Call.anchor stay intact.
The refined A1 control passes; its two negatives reject for the exact first-result
and RENAME-role reasons. No TypeError/AttributeError is used as closure evidence.

## Fresh independent execution

All imports use pinned archive/helper copies and pre-import process/signal/network
guards. The copied public scripts retain their original bytes/modes, controls
and predicates. Only external `review.ROOT` injection and disposable tempfile
relocation select the new source/output roots. Each prescribed command ran once.

| Suite | Normal | Optimized |
|---|---|---|
| Explicit104 =68 original+22 prior+14 new |104 pass;18.592s |104 pass;18.480s |
| Copied public16 |16 pass;3.024s |16 pass;2.957s |
| Copied supplemental8 |8 pass;1.116s |8 pass;1.194s |
| Copied refined3 |3 pass;1.150s |3 pass;1.140s |
| Copied adjacent6 |6 pass;1.486s |6 pass;1.457s |
| New reader-lock2 |1 pass/1 failure;0.154s |1 pass/1 failure;0.155s |

All suites have0 errors/skips. The five prescribed suites exit0 per mode;
the new lock suite exits1 per mode. All independent guard event lists are empty.
`independent-results-audit.json` records exact log IDs, durations and predicates.

Executed commands, with `R2` set to the exclusive review root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 "$R2/run-review.py" focused
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R2/run-review.py" focused
PYTHONDONTWRITEBYTECODE=1 python3 "$R2/run-review.py" probes
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R2/run-review.py" probes
PYTHONDONTWRITEBYTECODE=1 python3 "$R2/run-review.py" supplemental
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R2/run-review.py" supplemental
PYTHONDONTWRITEBYTECODE=1 python3 "$R2/run-review.py" joins_refined
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R2/run-review.py" joins_refined
PYTHONDONTWRITEBYTECODE=1 python3 "$R2/run-review.py" adjacent
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R2/run-review.py" adjacent
PYTHONDONTWRITEBYTECODE=1 python3 "$R2/reader-lock-probe.py"
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R2/reader-lock-probe.py"
```

The read-only audits ran as `PYTHONDONTWRITEBYTECODE=1 python3 "$R2/<script>"`:
`provenance.py`, `audit-full-proof.py`, `audit-independent.py` and `verify-final.py`.
The saved-log audit's first two lookup failures and script preimages remain
retained as `audit-independent-attempt{1,2}.*`. Attempt3 passes. It changed no
test, expectation in a copied probe, or candidate source; it corrected only
the reviewer's JSON-key lookup to the actual retained schema.

## Retained sole-QA full270 — independently verified, not rerun

`retained-full270-audit.json` verifies all166 baseline+104 fault IDs in the
inventory, loaded-ID enforcement, executed lists, actual test logs and final
summaries. Both retained commands report270 pass,0 failures/errors/skips,exit0:
normal81.082s; optimized178.418s. Their final194 source pins match.

The exact retained commands use `CHECK_LABEL=full270` and
`A/full-verifier/run-full-local.py`, with `PYTHONDONTWRITEBYTECODE=1` in both
modes and `PYTHONOPTIMIZE=1` in the optimized mode. The pinned wrapper propagates
the full runner's SystemExit. Final stdout agrees with result.json.

Verified policy and runner facts:

- Four guard files retain exact bytes/modes. `prepare-verifier.py` and unused
  `qualify.py` also match the accepted verifier. No old qualification code ran.
- Qualification differs only in source pins and inventory IDs. Exactly four
  assigned source pins change;190 unassigned pins match. Baseline bodies,
  snippets, executables, per-test groups, argv/env/cwd and descendant policy match.
- Full runner changes only its256-to270 counts/messages. Its watchdog and guards
  remain identical. All17 allow/6 deny pure-policy cases match after normalizing
  only disposable path/PID identities; no candidate body ran in those checks.
- The18th runtime input redirects only exact `/tmp/opencode` mkdtemp requests
  into the run's private tmp root. The unchanged tracker checks UID/device/inode;
  the unchanged containment function resolves physical paths. Each mode records
 346 fresh roots and346 relocations. No argv/env or policy fallback was added.

`retained-process-audit.json` independently joins each retained process:

| Per-mode evidence | Verified count |
|---|---:|
| Qualified/started/exited direct children |506 |
| Exact baseline IDs with children |37 |
| Direct statuses0 / expected1 / expected2 / expected77 |277 /226 /1 /2 |
| Confined gateways / waited leaves |31 /27 |
| Distinct logged processes |564 |
| Observed Popen wait/poll statuses |533 |
| Gateway wrapper-return statuses |31 |
| Denials / deadlines / exceptional teardown signals |0 /0 /0 |

Direct tickets match their test/group, argv, roots, guard/qualification pins and
environment hashes. Child entries, nested tickets, allowed descendants and
status joins match. The independent join equals the author audit. The31 gateway
statuses derive from the pinned return/SystemExit path; they are not separate
kernel waits. Each mode retains504 child JSONL logs,488 stdout,438 stderr and50
stdin artifacts. These files remain included in the frozen proof pins.

## Provenance and final preservation

| F2 entry point | SHA256 |
|---|---|
| Candidate12 manifest | `3fa82c5039b4442f2b4bded4fa8ef736f7b3d459c7dbaafc7d757130150d3163` |
| Exact12-member archive | `b96a90a498873f224ad1a264efce606910b7f08c40e316bb47357f37f7ee471c` |
| Complete delta | `5cf5e5b36ffe368e9b9aec8a64691afbf2ddd640d26e9e44670ac1ab278ed851` |

The complete regenerated delta matches. Only protocol.py,peer.py,correction tests
and G3_FAULTS.md change among existing source paths. The new author ledger is
the package's ledger entry; the old ledger remains preserved. All12 candidate
files retain0664 modes. Trial/client and the original five test/fixture files
match F. Ten Python files parse/compile in both modes. Runtime files have no
assert statements; candidate files have no trailing whitespace. Eight daemon
shape ASTs,three typed peer shape ASTs and22 prior correction test ASTs match F.

`inputs-before.json` and `inputs-after.json` match for all groups:
12 candidate,8,409 immutable old,4,694 new proof,194 qualified,18 runtime and784
retained reference entries. They cover13,889 distinct original paths; counts
overlap. All33 source/helper/copied-runner pins match before/after. The new proof
map includes its two independently pinned exclusions; frozen author membership
also matches. `final-pins.json` gives the final result.

Old F stays pinned to manifest `dca2d63bf08d627b987dbd33cfb49a261a5f34e1026efed3b05a79175ceec7e0`
and archive `0568b6dd5172e35695b8662b26e722cc8caa17409a5f53210a2c25fd49c5e5b5`.
Accepted unchanged256 stays pinned to manifest `7a1746c90defb0a23c7b2039a8715f92d51c65a942c633c3152d5fba2822a6b6`
and archive `ea8e696557eedd89e85a49cc7b38aac832dea1771e7c7521f175dc2cc730bd70`.
That256 PASS applies to F. Original strict256 remains failed at219 passing IDs,
37 blocked IDs,6 failures/240 subtest errors/246 denials per mode. Original red
adjacent6 remains3 pass/3 failures/0 errors per mode. Owned-red14 retains11
failures/3 AttributeErrors. Missing-output/quota stops,17/148-test verifier stops,
original68/234,timeout/retry and packaging failures remain unchanged.

Fault-v1 remains `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523`.
The preserved binary remains `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`.
All13 original+139 retry native files match. These are retained-file checks only.

## Handoff and limits

R2 contains `HANDOFF.md`, a byte-identical copy of this report,
`review-files.sha256.json` and `REVIEW-PACKAGE.sha256`. The inventory covers all
new artifacts, including scratch proof files and failed audit attempts. It
excludes only itself and PACKAGE to avoid checksum cycles. PACKAGE binds the
inventory and report entry points. Use those files as the sealed review inputs.

Only this new primary report and R2 artifacts were written. Candidate/source/old
evidence remained read-only. No full270 rerun, Rust suite, provider discovery,
Git write, process control, live action or delegation ran. Active P2/P3/daemon/
release outputs were not inspected. Native construction remains disabled and
native/full-G3 flags remain false. Reset/UuidOpen provider binding, native
providers, P4/P5, hooks and full A4/B3/R0/N1–N4 assessment remain unfinished.
