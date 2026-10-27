# sol-g3-fault-reader-lock-r3

Task: Assign narrow r3 correction for P2 G3-FC1-R2-LOCK. Move reader JSON decoding, envelope validation and ordinary-event classification outside the reader condition. Retain read ownership through atomic queue/error publication. Own only protocol.py, correction tests, G3_FAULTS.md, this new ledger and new exclusive artifacts. Preserve all r2 source preimages, old ledgers/reviews/archives/proof outputs. No other source, provider, crate, contract or canonical-status edit, active-other-owner read, Git/live/native/cluster/capture/process-control action or delegation. Only the already qualified offline child policy may execute baseline children.
Status: AUTHOR COMPLETE/FROZEN for independent narrow delta review. Exact lock2, focused109, copied16/8/3/6 and new full275 pass per Python mode,0 failures/errors/skips. Each passing suite ran once. All13,653 unique immutable proof pins,194 current qualified inputs and18 runtime inputs match. Exactly3 authorized existing source paths changed;191 qualified inputs remain unchanged. Action-admission ASTs,8 daemon shapes,3 typed peer shapes and36 r2 correction test ASTs remain unchanged. The parsing-lock block predates r2; this is not a new false-action regression. No complete-core/P4/P5/native/full-G3/publication acceptance follows.
Context: R2 A=/mnt/workspace/sol-g3-fault-r2-20261026.rb058gbv; F2=A/freeze. Manifest3fa82c5039b4442f2b4bded4fa8ef736f7b3d459c7dbaafc7d757130150d3163/archiveb96a90a498873f224ad1a264efce606910b7f08c40e316bb47357f37f7ee471c. Completed review R=/mnt/workspace/astra-g3-fault-r2-review-1yfba8j6. Review547-artifact manifestf894828c831a97b4514eb6d3f4168d747e59a497762236174e7352bc9f2d3771. Preserve8409 old pins,4694 r2 pins and547 review pins, with overlap. Keep native flags false and construction disabled.
Next: Parent consumes `/mnt/workspace/sol-g3-fault-reader-lock-r3-20261026.89jceo6n/freeze/HANDOFF.md` and assigns independent narrow delta review. Keep all source, ledgers, preimages, red/green results, qualified verifier and freeze outputs immutable. Reviewers need new exclusive output roots; existing scripts refuse reuse. Do not rerun passing suites, execute unused qualify.py, inspect active other-owner outputs, publish/promote source or run native actions under this assignment.
Log:
- [2026-10-26] Created this ledger first. R2 proof remains source-specific and immutable. No complete-core/P4/P5/native/full-G3/publication acceptance follows this assignment.
- [2026-10-26] Read the full completed primary review, R/HANDOFF.md and reader-lock-probe.py before edits. Exclusive artifacts: `/mnt/workspace/sol-g3-fault-reader-lock-r3-20261026.89jceo6n/`. The authorized workspace fallback avoids the known temporary quota. No old file moved/deleted. prepare.py verifies all12 F2 archive/current preimages, old8409+r24694+review547 groups, packages,194 qualified inputs and18 r2 runtime inputs. Groups overlap across13,653 unique immutable entries. Review KeyError attempts1/2 and corrected3 remain pinned. No candidate import occurred during preparation.
- [2026-10-26] Exact byte-identical reader-lock2 scripts ran against new pinned F2 source copies. Both modes exit1:2 tests,1 framing control pass/1 JSON-lock failure,0 errors/skips/denials. Observations confirm read ownership remains true, but JSON/envelope parsing holds the condition in F2. No action runs. All red source/probe/outcome files remain retained in red/. This is the missed pre-existing boundary, not a new false-action defect.
- [2026-10-26] Narrow correction moves JSON decoding, envelope validation and ordinary-event classification outside the condition. The reader keeps read_in_progress true from before recv through all framing/validation/classification. It builds bounded boolean metadata only, not a batch of decoded objects. Atomic publication checks current queue occupancy, appends raw bytes/parallel flags, updates the ordinary count, then clears ownership. Pops remove one raw frame/flag and decrement the count under the same condition. Errors/EOF publish terminal state before clearing ownership. Overflow publishes no partial batch. No queued payload is reparsed under the condition. The final zero-time admission/callback block remains unchanged.
- [2026-10-26] Added5 owned IDs for all four parsing stages, concurrent pop/accounting and one parse per frame, atomic sticky error, ordinary/total queue limits and partial-tail EOF. All36 r2 correction tests remain present. Measured explicit109=68 original+41 correction IDs and full275=166 baseline+109 fault IDs. New full verifier copies the r2 guards,37 groups, snippets/executables/helpers, storage adapter and policy unchanged. Only3 authorized source pins, inventory/count messages and new paths change;191 other qualified inputs stay stable. Reused the exact r2 17-allow/6-deny receipt by byte identity; no new pure-policy execution or unused qualify.py execution occurred. New18-input runtime-map SHA25620f8955a0a59daf09fa78c716410fe80e130272500e41195af145ab65332b5ff.
- [2026-10-26] Exact unchanged corrected-source lock2 passes2/0 failures/0 errors/0 skips per mode,exit0. Its framing/JSON observations show the condition free and read ownership true, with no action. Focused109 passes per mode: normal15.603s/optimized15.679s,exit0 and0 failures/errors/skips. Copied public16/supplemental8/refined3/adjacent6 pass per mode. Their original bytes/predicates and all readiness/worker closure controls stay unchanged. Guards precede candidate imports; guard event lists remain empty. Counts overlap.
- [2026-10-26] New explicit full275 passes once per mode: normal78.373s and optimized175.401s,exit0,0 failures/errors/skips. Actual loaded/executed/passing IDs equal166 unchanged baseline+109 fault IDs. Each command had a600s limit and the unchanged590s owned-child watchdog; neither fired. Each records506 direct child starts/observed exits across exactly37 baseline IDs,31 gateways,27 waited leaves and564 distinct logged processes. Direct statuses are277 exit0/226 expected exit1/1 expected exit2/2 expected exit77. The506 direct+27 leaf statuses are Popen wait/poll observations;31 gateway statuses derive from pinned wrapper return/SystemExit paths, not independent kernel waits. Each records351 fresh roots/storage relocations. No fault test/import gains process capability. No denial, signal, exceptional teardown or deadline occurs.
- [2026-10-26] Final audit exit0. All13,653 immutable entries match across overlapping8409 old/4694 r2/547 review groups and pinned exclusions. All194 new qualified and18 runtime inputs match; only protocol.py,correction tests,G3_FAULTS.md differ from r2. All191 unassigned inputs, old runtime18, binary330cf3ee and152 native evidence files match. All12 complete r2 preimages remain intact. Both Channel/BufferedChannel action_transition ASTs,8 daemon shape assignments,3 typed peer shape assignments and36 r2 regression ASTs remain unchanged. Candidate runtime/test files parse/compile in both modes; runtime files have no assert. No provider/client/peer/trial/originalfive-test/crate/contract/canonical-status edit occurred. No active other-owner output, Rust/Git/live/cluster/capture/process-control action or delegation ran.

## Queue and lock invariants

- The reader sets read_in_progress under the condition before recv. It stays
  true through framing, JSON decoding, envelope validation and classification.
  These stages run outside the condition and retain no decoded batch.
- Local raw frames remain limited by the unchanged recv/frame/Framer bounds.
  Private local flags have at most64 entries; ordinary events have limit60.
- Publication holds the condition. It checks live occupancy after any concurrent
  pops. It then appends raw frames/parallel flags, increments the ordinary count,
  clears read ownership and notifies. A rejected batch publishes nothing.
- receive removes one raw frame and flag and decrements the count under that
  same condition. Raw queue length and flag length agree. The ordinary count
  equals the number of true flags. No queued payload is reparsed under the lock.
- Error publication sets the sticky error before clearing ownership. Old queued
  bytes/metadata stay aligned and error takes receive priority. Clean EOF sets
  ended/eof_after_action before clearing ownership. Partial EOF becomes error.
- A partial frame with no complete envelope remains owned by Framer.buffer/
  length after its read publishes. The unchanged action boundary checks that
  partial state. There is no unrepresented-byte/admission gap.
- The final zero-time readiness probe, deadline/seal callback, action_started
  and bounded nonblocking/non-reentrant callback boundary remain unchanged.
  Fsync/transcript/proof/ownership I/O and all parsing stay outside the condition.
  Genuine post-action EOF and NormalStop retain their separate paths.

## Verification and exact commands

Artifact root (**B**): `/mnt/workspace/sol-g3-fault-reader-lock-r3-20261026.89jceo6n/`.
Working directory: `/mnt/workspace/rust-mds`.

| Scope | Normal | Optimized |
|---|---|---|
| Exact lock2 on F2 |1 control pass/1 failure/0 errors/skips,exit1 |same |
| Exact lock2 on r3 |2 pass,0 failures/errors/skips,exit0 |same |
| Explicit109 fault IDs |109 pass,0/0/0,exit0;15.603s |109 pass,0/0/0,exit0;15.679s |
| Copied16/8/3/6 |all pass |all pass |
| New explicit275 full IDs |275 pass,0/0/0,exit0;78.373s |275 pass,0/0/0,exit0;175.401s |

Counts overlap. Do not add them as independent coverage. All IDs appear in
inventory.json, results.json and actual passing logs. process-audit.json records
every process and its status provenance. final-checks.json records the complete
source-specific audit. No old270 run was rerun or relabelled as r3 proof.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 "$B/prepare.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$B/red/reader-lock-probe.py"
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$B/red/reader-lock-probe.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$B/prepare-checks.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$B/green/reader-lock-probe.py"
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$B/green/reader-lock-probe.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$B/green/run-review.py" focused
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$B/green/run-review.py" focused
PYTHONDONTWRITEBYTECODE=1 python3 "$B/green/run-review.py" probes
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$B/green/run-review.py" probes
PYTHONDONTWRITEBYTECODE=1 python3 "$B/green/run-review.py" supplemental
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$B/green/run-review.py" supplemental
PYTHONDONTWRITEBYTECODE=1 python3 "$B/green/run-review.py" joins_refined
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$B/green/run-review.py" joins_refined
PYTHONDONTWRITEBYTECODE=1 python3 "$B/green/run-review.py" adjacent
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$B/green/run-review.py" adjacent
PYTHONDONTWRITEBYTECODE=1 CHECK_LABEL=full275 python3 "$B/full-verifier/run-full-local.py"
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 CHECK_LABEL=full275 python3 "$B/full-verifier/run-full-local.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$B/audit-final.py"
```

prepare-checks.py measures the actual IDs and copies the unchanged policy. Only
external count messages change afterward. The18-runtime input map was generated
before either full run; its hash is recorded above. Exact17-allow/6-deny policy
receipt reuse is by byte identity. Four guards, storage adapter,37 groups and
all executable/helper/snippet/argv/env/cwd/descendant rules remain unchanged.
No pure-policy child/body runs or unused qualify.py execution occurred here.
All temporary roots and outputs are new exclusive workspace files. No old
evidence was deleted/moved. Red lock failure and all previous partial/failure
attempts remain retained; no current-source verification stop occurred.

## Freeze and scope

freeze/candidate-files.sha256.json and freeze/offline-reader-lock-r3-source.tar.gz
cover12 complete candidate inputs:4 modules,6 owned test modules,G3_FAULTS.md and
this new ledger. source-delta.patch covers exactly3 existing scripts. The complete
frozen delta adds this new ledger; the old r2 ledger stays immutable. The archive
retains bytes/modes and all r2 preimages remain under preimages/.

freeze/HANDOFF.md, freeze/freeze.json, freeze/final-preservation-after.json,
freeze/proof-files.sha256.json and freeze/PACKAGE.sha256 provide exact hashes,
commands and source-frozen final checks. Immutable map checks distinguish expected
r2-to-r3 primary-source changes from unchanged old archives/proof bytes. Retained
full270 is independently verified r2 proof, not current r3 proof. Original
strict256, verifier17/148 stops, red probes, audit KeyErrors1/2/corrected3,
68/234/timeout/retry/packaging evidence and prior review decisions stay unchanged.

FC45-C1 scoped offline closure, original readiness fixes and earlier FC2/FC3/FC6
scopes remain unchanged. Peer/source authority and the final caller-owned
exclusive oracle/evidence interval remain unchanged. Reset/UuidOpen binding,
P4/P5, providers/hooks/native construction and full A4/B3/R0/N1–N4/G3 remain
unfinished/disabled. Native flags remain false. Independent review follows;
no complete-core, promotion, publication or live permission is claimed.

Preserve fs44/meta88/data89, both acknowledged namespaces, FSID
3ceca2c6-851f-41cf-b538-1cfded4f1e82, binary
330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374 and all13+139
native evidence files. These are file-preservation facts, not cluster queries
or fresh binary/native qualification.
