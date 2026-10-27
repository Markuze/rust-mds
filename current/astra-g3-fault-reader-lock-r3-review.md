# astra-g3-fault-reader-lock-r3-review
Task: NEW independent narrow r3 review for P2 G3-FC1-R2-LOCK. Verify the frozen twelve-path candidate, minimal reader-lock delta, all preservation pins and retained full275 evidence. Run guarded archive-copy unchanged reader-lock2, focused109 and copied16/8/3/6 once per Python mode. Return scoped acceptance or ranked concrete findings with sealed review artifacts.
Status: COMPLETE — SCOPED ACCEPT. G3-FC1-R2-LOCK closes for the reviewed offline reader-lock delta. Exact unchanged lock2, focused109 and copied16/8/3/6 pass independently in both Python modes. Two added queue-boundary probes pass per mode. Retained full275 proof verifies independently and was not rerun. All initial preservation and copy pins match. Prior offline closures remain valid in their recorded scopes.
Context: Own only this new primary ledger and new exclusive external review artifacts/copy. Candidate/source/tests/docs, prior reports and evidence stay read-only. B=/mnt/workspace/sol-g3-fault-reader-lock-r3-20261026.89jceo6n; F3=B/freeze. No fixes, Git/live/native/installed/cluster/capture actions, process/network control or delegation. Active NFS/P3/daemon/release outputs stay unread. Use pre-import guards and disposable socketpairs/threads/files/action doubles. Verify and reuse full275; do not rerun it or execute old qualify.py.
Next: Parent records this narrow acceptance and assigns dependent work separately. Preserve the sealed review, F3 and all earlier source-specific evidence. Reset/UuidOpen binding, full case assessment, P4/P5, hooks and native qualification remain separate work.
Log:
- [2026-10-26] Created the sole writable primary ledger before review edits.
- [2026-10-26] Read the full assigned handoff/author ledger/freeze/package/prior review and parent assignment. Created `/mnt/workspace/astra-g3-fault-reader-lock-r3-review-fk36dobp/`. Independently verified exact12 archive members, complete three-path/new-ledger delta,13,653 immutable pins,4,620 new proof pins,194 qualified inputs and18 runtime inputs. Exactly3 current repins leave191 unchanged. Verified both action-transition ASTs,36 prior correction ASTs,8 daemon shapes and3 typed peer shapes. Extracted pinned copies before candidate import.
- [2026-10-26] All12 prescribed guarded commands exit0, each once: unchanged lock2, focused109 and copied16/8/3/6 per mode. Full275 retained proof audit exits0: exact275 loaded/executed/passed IDs,506 direct children on37 baseline IDs,564 joined processes and351 roots/relocations per mode. Four guards/storage adapter/policy are identical.17 allow/6 deny receipt reuse is byte-identical; zero new policy executions.
- [2026-10-26] Added two concrete queue probes for full-queue concurrent pops and error priority with older ordinary entries. Both pass per mode,0 failures/errors/skips. Real socket bytes pass through the real parser after bounded barriers. Saved-result and queue-mutation audits exit0. All copied predicates and intended rejection reasons remain intact.
- [2026-10-26] Final pins match12 candidate,13,653 immutable,4,620 new proof,194 qualified,18 runtime and36 copied-reference entries across18,461 original paths, plus34 copy pins.152 native files and binary330cf3ee match as file preservation only. Recorded SCOPED ACCEPT and sealed this report with the exclusive review artifacts.

## Decision

**SCOPED ACCEPT — close P2 G3-FC1-R2-LOCK.** No new finding remains in this
narrow offline delta. The correction meets the outside-condition parsing and
atomic queue-accounting requirements. This closes the pre-existing missed
requirement; it does not relabel it as a new false-action regression.

The exact unchanged action-transition ASTs and peer/client/trial bytes preserve
the r2 readiness correction, FC45-C1 worker/source closure and earlier FC2/FC3/FC6
offline scopes. The assigned offline fault-core findings can close in those
scopes. Reset/UuidOpen provider binding and full case assessment remain open.
Native construction remains disabled; native/full-G3 flags remain false.

Review root **R3**: `/mnt/workspace/astra-g3-fault-reader-lock-r3-review-fk36dobp/`.
Author root **B**: `/mnt/workspace/sol-g3-fault-reader-lock-r3-20261026.89jceo6n/`.

## Reader-lock and queue audit

Reviewed source references use `scripts/g3_fault_protocol.py` from the exact F3
archive. All12 primary candidate files matched that archive before and after.

### Parsing and ownership

- `:644–648` marks `read_in_progress=true` under the condition before recv.
- `:649–663` performs recv, Framer.feed, JSON loads, envelope validation and
  ordinary-event classification outside the condition. Ownership stays true.
- Each new raw frame has one reader-validation decode. Only a boolean survives
  classification; the reader retains no decoded-envelope batch. It does not
  reparse queued raw frames. Consumer parsing remains on its unchanged path.
- The exact unchanged lock2 probe now observes the condition free during both
  framing and JSON parsing. Admission acquires it and rejects active ownership.
  The five new owned regressions also pause envelope validation and classification.
  Every stage retains ownership and permits condition acquisition.

### Bounds, publication and pops

The unchanged Framer and recv limits bound local raw frames. A successful Framer
batch has at most64 frames. Local metadata has one boolean per frame. The local
ordinary check rejects excess ordinary input before queue publication. Published
metadata stays at most64 flags and60 ordinary events. No decoded batch or
unbounded accumulation was introduced. This is a boundedness review, not a
maximum-size throughput or memory-use qualification.

At`:664–674`, publication checks **live** raw queue length and ordinary count
under the condition. Consumer pops during parsing therefore affect admission.
Both total and ordinary capacity checks precede all appends. Publication appends
raw frames and matching flags, increments the count, then clears ownership and
notifies. A validation/capacity rejection publishes none of its batch.

At`:681–693`, receive holds the same condition. It removes one matching flag
and raw frame, and subtracts the flag from the count. Under supported use,
`len(queue)==len(_ordinary_flags)` and
`_ordinary_count==sum(_ordinary_flags)` hold after every observable transition.
All flag values come from a boolean classification expression.

The four-module source audit found six queue mutation calls, all in protocol.py:
two in synchronous Channel.receive and four in BufferedChannel publication/pop.
BufferedChannel overrides synchronous receive. Controller drains call the
overridden receive method; no separate drain/clear bypass changes metadata.
Initialization creates empty aligned lists/count before the reader starts.
`queue-source-audit.json` records mutation sites and condition call sites.
Arbitrary external mutation of queue internals is outside this contract.

### Error, EOF and partial bytes

`:675–679` publishes the sticky error before clearing ownership. The reader then
exits. Receive checks that error before returning an older queued frame. Repeated
receives and admission raise the same error; older frames/flags/count stay aligned.

`:650–657` checks Framer.eof outside the condition. Clean EOF publishes ended and
eof_after_action before clearing ownership. Partial EOF enters the sticky-error
path. An incomplete non-EOF read retains bytes/length in Framer state before
ownership clears. The unchanged admission predicate rejects that state.
There is no consumed-byte/empty-queue/false-ownership admission gap.

### Unchanged action boundary

Both Channel.action_transition and BufferedChannel.action_transition retain
exact r2 AST identity. The buffered deadline/seal check precedes the zero-time
readiness probe, action_started and bounded action callback under the condition.
Fsync, transcript/proof checks and ownership I/O remain outside. Callback
nonblocking/non-reentrant obligations remain unchanged.

The exact paused-real-select EOF and bad-frame negatives still invoke no action,
latch failure and keep expected/post-action EOF false. Genuine post-action EOF
completes its A0 control. NormalStop retains its separate final handshake/status
path. FC45 and refined A1 controls preserve decoded-source binding, RENAME roles,
raw syscall versus wire results, first-result joins, typed Call.anchor and saved
descriptors. No TypeError/AttributeError receives closure credit.

## Fresh independent tests

All runs use a new extracted archive/helper copy with pre-import process/signal/
network guards. The exact lock probe and copied public/adjacent scripts retain
their original bytes/modes and predicates. The external runner changes only
its104-to109 inventory checks. Root injection and disposable tempfile relocation
select R3; candidate source remains read-only. Each prescribed suite ran once.

| Suite | Normal | Optimized |
|---|---|---|
| Exact unchanged lock2 |2 pass;0.103s |2 pass;0.103s |
| Focused109 |109 pass;15.103s |109 pass;14.918s |
| Exact copied public16 |16 pass;2.483s |16 pass;2.522s |
| Exact copied supplemental8 |8 pass;0.994s |8 pass;1.016s |
| Exact copied refined3 |3 pass;1.003s |3 pass;1.020s |
| Exact copied adjacent6 |6 pass;1.349s |6 pass;1.308s |
| Added queue-boundary2 |2 pass;0.006s |2 pass;0.006s |

Every row has0 failures/errors/skips and exit0 in both modes. Guard denial/event
lists are empty.109=68 original+36 prior corrections+5 new corrections; the
correction module contains41 test methods. Counts overlap and are not additive.
`independent-results-audit.json` gives actual IDs and durations.

The two added cases address concrete gaps in the smaller owned queue fixtures:

1. **Full queue with concurrent pops:** pause classification of a mixed incoming
   batch while64 raw frames/60 ordinary events are queued. Pop two ordinary and
   two reserved frames. Publication observes60/58, then admits the four-frame
   mixed batch to64/60. All frames drain in exact order; flags/count remain aligned
   through clean EOF. Exactly68 frames receive68 reader-validation decodes.
2. **Older ordinary frames plus an invalid batch:** retain two queued frames with
   flags `[true,false]` and count1. Parse a new ordinary frame followed by an
   invalid-version frame. The batch publishes nothing. Older bytes/flags/count
   remain unchanged. Repeated receive and admission raise
   `unsupported protocol version`; no final check or action runs.

These tests use real socketpairs and parsers. A bounded socket wrapper groups
known byte batches within the unchanged recv limit. It changes no payload or
parser predicate. `queue-boundary-{normal,optimized}.json` retains observations.

Executed commands, with `R3` set to the exclusive review root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 "$R3/reader-lock-probe.py"
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R3/reader-lock-probe.py"
PYTHONDONTWRITEBYTECODE=1 python3 "$R3/run-review.py" focused
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R3/run-review.py" focused
PYTHONDONTWRITEBYTECODE=1 python3 "$R3/run-review.py" probes
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R3/run-review.py" probes
PYTHONDONTWRITEBYTECODE=1 python3 "$R3/run-review.py" supplemental
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R3/run-review.py" supplemental
PYTHONDONTWRITEBYTECODE=1 python3 "$R3/run-review.py" joins_refined
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R3/run-review.py" joins_refined
PYTHONDONTWRITEBYTECODE=1 python3 "$R3/run-review.py" adjacent
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R3/run-review.py" adjacent
PYTHONDONTWRITEBYTECODE=1 python3 "$R3/queue-boundary-probes.py"
PYTHONDONTWRITEBYTECODE=1 PYTHONOPTIMIZE=1 python3 "$R3/queue-boundary-probes.py"
```

Read-only verification commands use the same normal Python environment with
`provenance.py`, `audit-full-proof.py`, `audit-results.py` and `verify-final.py`.
Each exits0. No candidate test rerun was needed.

The author red lock2 is **reused retained evidence**, not rerun. Exact source/probe
pins match the reviewed F2 snapshot. Both retained modes stay1 control pass/1
failure/0 errors/skips,exit1, with no action. The original reviewer red2 also
remains sealed and failed. These failures are not relabelled by current green2.

## RETAINED full275 — independently verified, not rerun

`retained-full275-audit.json` verifies166 unchanged baseline+109 fault IDs in the
inventory, loaded-ID enforcement, executed lists and actual passing test logs.
Both retained modes report275 pass,0 failures/errors/skips,exit0:
normal78.373s; optimized175.401s. Final stdout agrees with result.json. Both
post-suite194-input maps match. The pinned wrapper propagates SystemExit.

Retained commands are `PYTHONDONTWRITEBYTECODE=1 CHECK_LABEL=full275 python3
"$B/full-verifier/run-full-local.py"`, adding `PYTHONOPTIMIZE=1` for optimized.
The author records a600s outer limit. The source retains the unchanged590s
watchdog; no deadline event occurred. This review did not execute that runner,
unused qualify.py, old policy scripts, or any baseline child.

Verified policy and source facts:

- All four guards, storage adapter, preparation/qualification scripts and command
  policy document retain exact r2 bytes/modes. Per-test groups, helpers,
  executables, snippets, argv/env/cwd and descendant rules remain identical.
- Qualification changes only the three authorized source pins and explicit IDs.
 191 qualified inputs remain unchanged. Full runner changes only270-to275 and
 104-to109 count/messages. New exclusive verifier paths are derived as before.
- The18 runtime inputs retain membership after path remapping. Only inventory,
  qualification and runner bytes change. The storage adapter still redirects
  only exact `/tmp/opencode` mkdtemp requests into the private run tmp root.
- The unchanged tracker binds UID/device/inode; the unchanged containment check
  resolves physical paths. Each mode records351 fresh roots and351 relocations.
- The17 allow/6 deny policy receipt is **REUSED BY BYTE IDENTITY**. There were
  **zero new pure-policy executions**. Old disposable PID/path identities remain
  in that receipt. It is not relabelled as a new policy run.

`retained-process-audit.json` independently joins all process evidence:

| Per-mode fact | Verified count |
|---|---:|
| Qualified/started/exited direct children |506 |
| Exact baseline IDs with child capability |37 |
| Direct exits0 / expected1 / expected2 / expected77 |277 /226 /1 /2 |
| Confined gateways / waited leaves |31 /27 |
| Distinct logged processes |564 |
| Observed Popen wait/poll statuses |533 |
| Gateway return/SystemExit statuses |31 |
| Denials / exceptional teardown signals / deadlines |0 /0 /0 |

Tickets, exact argv, root histories, guard/qualification pins, environment hashes,
descendant entries, nested tickets and statuses match. The independent join
agrees with the author process audit after field-name normalization. The31
gateway statuses use pinned wrapper-return provenance, not separate kernel waits.
No fault test or import has child capability. All retained stdin/stdout/stderr
and process logs remain covered by the new proof inventory.

## Candidate and preservation pins

| F3/B entry point | SHA256 |
|---|---|
| Twelve-path manifest | `92970520e1cbdbe0ef8911b561a0430cbc61a90fdb6943c4c74c14c677e35d52` |
| Exact twelve-member archive | `370f247def22520e1bec259539125f7c567fa4f9e0fa8647499e813ee47906da` |
| Complete delta | `1648eac29fbe9deea29e0dd1eccbd2a6fc6fd4d2d6b4f25a684d2657aa0e697a` |
| Three-existing-path delta | `b932ac0638e60ebfce91eb532d422b3526a8b4e1ad8adbcd1f1f31a481553f78` |
| New proof map | `2e47a36ddbb51300f1deadbad13a0450252af31420767854502296706c6fd86e` |
| Final author preservation | `d6c652c8fd456dca374e4b95cb9a34c638c7c7469e0dda058a9ebf14b48560ef` |
| Explicit inventory | `3c97df5b70107ffbddb3a4f8d2cc04907902a0de3a041e7f6809fb0afd7f480a` |
|194 qualified inputs | `3647839f4369f956b5fb928e855aea713479cf4031c9ad2425631be43c2d148e` |
|18 runtime inputs | `20f8955a0a59daf09fa78c716410fe80e130272500e41195af145ab65332b5ff` |

Both independently regenerated deltas match. Only protocol.py, correction tests
and G3_FAULTS.md change among existing source paths; the complete delta adds the
new author ledger. All12 candidate files retain0664 modes. Peer/client/trial
and the original five test/fixture modules match r2 byte-for-byte. Both action
ASTs,36 prior correction methods,8 daemon shapes and3 typed peer shapes match.
Ten Python files parse/compile in both modes. Runtime files have no assert;
candidate files have no trailing whitespace.

`inputs-before.json` and `inputs-after.json` match12 candidate,13,653 unique
immutable,4,620 new proof,194 qualified,18 runtime and36 copied-reference entries.
Groups overlap across18,461 original paths. All34 extracted/helper/exact-probe
copy pins match. The13,653 immutable union independently reconstructs overlapping
8409 old/4694 r2/547 review groups plus explicit map/package/report entries.
The new4,620 proof pins include the map's two exclusions; author-root membership
matches. Exact three authorized r2-to-r3 working repins remain distinct from
immutable old archive bytes. `final-pins.json` records the final checks.

Retained F2 manifest/archive remain3fa82c50/b96a90a4 at their full pinned hashes.
The547-artifact r2 review manifest remains
`f894828c831a97b4514eb6d3f4168d747e59a497762236174e7352bc9f2d3771`.
Its audit KeyErrors1/2 and corrected3 remain preserved. Retained270 and accepted
unchanged256 remain source-specific. Original strict256 stays failed at219
passing/37 blocked IDs,6 failures/240 subtest errors/246 denials per mode.
Verifier17/148-test stops, red probes,68/234 checks, timeout/retry, missing-output,
quota and packaging failures retain their original results.

Fault-v1 remains `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523`.
The binary remains `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`.
All152 native files match in the initial/final qualified maps. These are retained
file checks only; no cluster query or binary/native requalification occurred.

## Sealed handoff and remaining scope

R3 contains `HANDOFF.md`, byte-identical `report.md`, `review-files.sha256.json`
and `REVIEW-PACKAGE.sha256`. The manifest covers all new retained files/symlinks,
including source copies, scripts, scratch proof files, test/audit outputs and
report. It excludes only itself and PACKAGE to avoid checksum cycles. PACKAGE
binds the manifest and report entry points. Existing outputs refuse reuse.

Only this new primary report and R3 artifacts were written. Candidate/old evidence
remained read-only. Active NFS/P3/daemon/release outputs remained unread/unpolled.
No Rust suite, provider discovery, Git/live/installed/native/cluster/capture
action, process/network control or delegation occurred. Reset/UuidOpen binding,
P4/P5, hooks, native construction, A4/B3/R0/N1–N4 and full G3 remain separate
unfinished work. This decision supplies scoped offline acceptance only.
