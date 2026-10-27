# astra-g3-wire-nfs41-r2-review

Task: NEW independent P2 r2 correction review for NFS41-1–6. Parent orchestrates.
Status: COMPLETE — CHANGES REQUIRED. Two P1 and two P2 residual findings block offline P2 acceptance. NFS41-1–5 close in this correction-review scope. NFS41-6 remains open. Exact 55+24+8 pass per mode. New 12+1 probes give eight passes, three assertion failures and two WireError results per mode. Final input verification passes. All151 new review artifacts are sealed and verified in REVIEW-MANIFEST.json, SHA256 5ff7ab61595281379fc107f082289967b03ac096b259da2b050ed2e045e67edf.
Context: Own only this new primary ledger and new exclusive external review artifacts. Candidate source/tests/fixtures, old ledgers/review, protocol and plan are read-only. Use only frozen references and the archived helper function. Core/P1/P3 outputs remain unread. Guards must precede provider imports and test discovery. No fixes, Git writes, live queries, capture, installed probes, native collection or delegation.
Next: Parent can assign NFS41-R2-1–4 corrections. Preserve this candidate and all red/control evidence. Require a new frozen candidate and independent review before offline P2 acceptance or P4 integration.
Log:
- [2026-10-26] Created the review ledger. No candidate edits or test execution occurred.
- [2026-10-26] Resumed the interrupted startup. The prior exclusive /tmp/opencode/astra-g3-wire-nfs41-r2-review-77see0sq directory was empty. The first resumed artifact patch failed with EDQUOT and left only a zero-byte STOP-RESUME.md. Retained that partial file. Created the authorized exclusive fallback /mnt/workspace/astra-g3-wire-nfs41-r2-review-bw066eix. No provider import or test execution preceded either stop.
- [2026-10-26] Retained one pre-test verification stop: the outer candidate directory also contains handoff-digests.json. Inspected its post-handoff receipt and added exact receipt/pin verification. No pinned member was missing or changed. Corrected verifier passes386/387, six primary/scratch pins, old37/review88/prequota277, ten references, exact six-path delta regeneration, static checks and original six golden packet/RPC preservation. Fresh run/ contains the archived helper and unchanged runners, without old output logs. All six commands pass55/24/8 with zero failures/errors/skips per mode. No focused rerun is needed.
- [2026-10-26] Ran only twelve targeted adjacent public cases once per mode with the unchanged independent byte builder and pre-import guards. Both commands exit1: seven pass, three assertion failures, two WireError results. Retained all logs, source inputs and actual returned reports/errors. The failures concern unrelated pending-call capacity, a later request adopted as a missing predecessor, and an unrelated in-compound OPEN authorizing a future stateid. The errors concern a valid delayed READ response and zero-seqid CLOSE with a delayed earlier OPEN response. RFC8881 sections8.2.2,9.9–9.10,18.2 support request-time ordering and zero-seqid CLOSE; a matching CLOSE control remains to be checked. Candidate and old evidence stay read-only.
- [2026-10-26] Added one narrow CLOSE positive control. It passes once per mode. Its RPC bytes, connection/direction identities and per-stream order match the failed case exactly; only cross-connection arrival order differs. Static result validation confirms the same equivalence for the READ and future-request control pairs. New13 total: eight pass, three assertion failures, two candidate WireError results per mode. No old suite reran.
- [2026-10-26] Final verify_review.py after passes and matches before.json except the stage label. All386 candidate members,387 archive members, six primary/scratch paths, old37/review88/prequota277 and ten references remain stable. summarize_results.py validates exact inventories, retained results, same-wire controls, RFC excerpts and the zero-byte quota partial. Decision: two P1 and two P2 residual generation/history findings. Evidence sealing follows.
- [2026-10-26] Sealed151 new review artifacts in REVIEW-MANIFEST.json,31654bytes, SHA256 5ff7ab61595281379fc107f082289967b03ac096b259da2b050ed2e045e67edf. seal_review.py exits0. Its post-write read-only check verifies exact inventory and every byte/mode pin. Completed this report. No offline P2 acceptance, integration, publication or live permission follows.

## Decision and scope

**CHANGES REQUIRED.** This is an independent offline P2 r2 parser/reducer review.
NFS41-1–5 close only for the exact corrections described below. NFS41-6 remains
open through four residual findings. Passing original probes do not close these
new failures.

- Candidate: `/mnt/workspace/sol-g3-wire-nfs41-r2-20261026.97n2e79z/`.
- Review evidence: `/mnt/workspace/astra-g3-wire-nfs41-r2-review-bw066eix/`.
- Execution directory: the review root's `run/` directory.
- Source locations below refer to frozen `final/scratch/scripts/g3_wire_nfs41.py`,
  SHA256 `50da723762dbeba0ebd44dbe7ed17d29ad6dc3704b34c90688aa9593e33b82f7`.

The review used synthetic RPC (Remote Procedure Call), XDR (External Data
Representation) and PDML (Packet Details Markup Language) inputs. The new probes
use the unchanged original independent byte builder. They do not import author
test builders. Process/network/signal guards precede provider import and test
discovery. Only the archived function-only handle helper was used.

## Ranked residual findings

### P1 NFS41-R2-1 — An unrelated pending call can admit a gap that a future call later fills

**Source:** `_predecessor_capacity`1187–1200; `_generation_ready`1225–1232;
`_drain_replies`1236–1272.

The pending-call count does not identify the missing predecessor. OPEN candidates
need only the same client and opaque owner. Their file handle, state identity and
lifetime need not match. LOCK candidates need only a LOCK/LOCKU opcode. The code
does not retain which calls were eligible when it observed the later generation.

`adjacent_probes.py` contains two public reproductions:

1. `test_wrong_fh_pending_is_rejected_on_reply` establishes inode501 generation1.
   Packet7/xid103 opens inode502 with the same owner. Packet8/xid104 opens inode501.
   Its packet9 reply returns generation3. There is no possible generation2 call
   for inode501. The reader emits `ReplyPending` and keeps `failed=false`.
   The rejection assertion fails in both modes. This first case does not call
   `finish`; it proves incorrect eligibility, not terminal success.
2. `test_wrong_pending_cannot_adopt_a_later_request_as_predecessor` adds a new
   inode501 OPEN at packet10/xid105, after generation3 was already observed.
   Its packet11 reply returns generation2. The reader applies generation2, then
   generation3. Packet12 completes the unrelated inode502 call with its own
   generation1. `finish` succeeds with empty outstanding/unresolved lists and
   OwnerState edges for the impossible inode501 history. The rejection assertion
   fails in both modes. The packet10 request cannot produce a server transition
   that preceded the packet9 reply.

**Valid controls:** unchanged
`supplemental_probes.py::test_parallel_open_replies_can_arrive_in_reverse_generation_order`
passes with actual earlier same-owner/FH calls. New
`test_pending_replay_and_rpc_duplicate_apply_once` also passes; it emits exactly
generations1,2,3 despite an RPC duplicate and logical replay.

RFC8881 §8.2.2 lines7732–7755 ties state to owner/file/type.
§9.9 lines9260–9286 requires the same file handle for an OPEN upgrade.
§9.10 lines9301–9317 permits reply reordering, not future causal predecessors.

**Required correction:** Retain bounded, deduplicated eligible predecessor
identities at observation time. Match client, owner/type, file-handle constraints
and lifetime. Validate the actual predecessor result against that relation.
A call first observed after the dependent reply cannot fill its missing history.
Preserve valid same-state concurrency, replay and unresolved-final rejection.

### P1 NFS41-R2-2 — An unrelated in-compound OPEN authorizes a future request stateid

**Source:** `_call`1095–1108; `_generation_ready`1208–1219.

`intra_state` becomes true after any OPEN, LOCK or LOCKU. That single boolean
marks every later nonzero request stateid as source-observed. It does not bind the
request to a preceding operation's actual state identity.

`test_unrelated_open_prefix_cannot_authorize_future_request` establishes ownerA's
inode501 generation1. Packet7/xid103 contains OPEN(ownerB), then LOCK with
ownerA's generation2. No generation2 response existed when that request arrived.
A later call returns ownerA generation2 before xid103's reply. The reader then
accepts OPEN(ownerB)'s distinct state and the LOCK. `finish` succeeds with empty
outstanding/unresolved lists and lock OwnerState/LockRanges evidence. The
rejection assertion fails in both modes.

**Valid control:** `test_observed_request_after_unrelated_open_prefix_is_valid`
passes with exactly the same RPC bytes when ownerA's generation2 reply precedes
the request. Per-stream order is unchanged. The existing focused future-request
test also passes when the unrelated OPEN prefix is absent.

**Required correction:** Replace the broad prefix permission with exact
per-operation dependency validation. An earlier unrelated state-changing
operation cannot supply a later request's stateid. Preserve legitimate current
stateid and same-state in-compound transitions, plus previously observed inputs.
Do not validate a future request retroactively at reply time.

### P2 NFS41-R2-3 — A valid delayed READ reply is checked against the newest reduced generation

**Source:** `_generation_ready`1203–1219; `_resolve`1284–1288.

`test_read_reply_after_upgrade_is_valid` sends READ(generation1) at packet7,
while generation1 is established. A parallel OPEN request follows on another
connection. Its generation2 reply arrives before the READ reply. The server can
execute the READ before the upgrade and deliver the two replies in this order.

The reader raises `Nfs41IdentityJoin: request stateid generation mismatch` at
line1219. Both modes record an unexpected WireError. The reader is failed; its
pre-admission error code is2. The test runner exits1. This is candidate rejection
of a valid input, not a guard or runner failure.

**Valid control:** `test_read_reply_before_upgrade_control` passes. It changes
only cross-connection reply arrival order. Every RPC and each stream's order are
identical. RFC8881 §8.2.2 lines7764–7774 checks a nonzero seqid when the server
processes the request. It does not require that generation still be the observer's
latest state when the reply arrives.

**Required correction:** Preserve the request's valid generation/lifetime
context and causal execution constraints. Validate a delayed successful result
against that context. Do not compare all replies only with the latest reduced
state. Preserve rejection of requests that were already stale or unobserved when
issued; this finding does not permit arbitrary old stateids.

### P2 NFS41-R2-4 — Zero-seqid CLOSE executes before its already-observed OPEN predecessors

**Source:** `_generation_ready`1208–1221; `_drain_replies`1242–1248;
CLOSE1473–1479; `_put_state`1330–1336.

`test_zero_request_close_waits_for_prior_pending_opens` establishes generation1
and sends two same-owner/FH OPENs. The generation3 reply arrives on slot1 while
generation2's response is delayed on another connection. The next slot1 call is
CLOSE with a normal stateid whose seqid is0. Its successful reply arrives before
the delayed generation2 reply.

Zero-seqid requests bypass the incoming-generation check. CLOSE returns no
ordinary generation to order. The code closes the currently reduced generation1
before reducing the already-observed generation3 history. When generation2's
reply arrives, `_put_state` rejects the now-inactive state with
`Nfs41IdentityJoin: stateid other identity collision`. Both modes record this
unexpected WireError, failed reader and pre-admission error code2.

**Valid control:** `close_control.py::test_zero_close_after_prior_open_replies_is_valid`
passes in both modes. It moves only generation2's reply earlier. It checks exact
RPC bytes, connection/direction identities and unchanged per-stream order against
the retained failed input. It emits generations1,2,3 and one StateClosed(gen3).

RFC8881 §8.2.2 lines7764–7780 and §9.9 lines9273–9279 permit a normal stateid
with seqid0, although nonzero CLOSE is generally recommended for parallel OPENs.
The CLOSE result uses the invalid special stateid required as the recommended
diagnostic value by §18.2.4 lines21498–21506. It is not malformed returned OPEN
state. The slot1 CLOSE follows the generation3 reply and must follow that OPEN's
server execution.

**Required correction:** Retain causal dependencies for zero-seqid operations
and lifetime termination. Reduce the correct already-observed predecessor chain
before CLOSE changes active state. Keep truly stale post-CLOSE requests rejected.
Do not fix this case by treating an inactive state as a new OPEN lifetime.

## Correction closure and other reviewed boundaries

| Original finding | r2 decision and evidence |
|---|---|
| NFS41-1, exact XDR/PDML fields | CLOSED in offline scope. `_Layout`250–490 binds selected fields to exact positions/widths, bounds arrays/opaque lengths, validates union arms/padding and consumes exact operation ends. All four original malformed probes pass as rejections. Added call/reply-family, union and payload-length tests pass. New optional implementation-ID plus AUTH_SYS callback16-group control passes;17 groups fail at the bound. Missing fields still fail. |
| NFS41-2, late FH equality | CLOSED in offline scope. `_constrain_targets`1304–1314 and `_bind_target`1316–1323 retain and enforce target equality. Original contradiction rejection and same-target/unresolved controls pass. The new multi-step constraint-chain positive/negative pair passes. Unknown targets are not fabricated from equality alone. |
| NFS41-3, returned evidence mutation | CLOSED in offline scope. Feed, worker/native join and terminal output use deep snapshots. Configuration and callback batches are copied before helper callbacks. Original owner-mutation probe and new nested/source/edge/configuration/callback tests pass. |
| NFS41-4, supplied worker consistency | CLOSED in offline scope. Lines1531–1544 require every supplied comparable handle/name to match and at least one match. Both/name-only/handle-only controls pass. Contradictory and neither-field cases fail. `matched_basis` lists the fields actually supplied and checked; it does not claim both for a one-field match. |
| NFS41-5, standard LOCKU tree | CLOSED in offline scope. Lines854–864 select the exact NFS-root sibling at opcode+8. Standard-tree positive and child/misplaced negative cases pass. The original six golden packets and RPC inputs are unchanged. The appended pair follows pinned packet-nfs.c10212–10219/10800–10815. |
| NFS41-6, generation/history order | OPEN. Initial0/9 rejection, MAX-to1 arithmetic, actual same-state reversed OPEN replies, unresolved-final rejection and pending worker-join rejection pass. New findings NFS41-R2-1–4 block closure. |

The MAX-to1 author test remains a labelled seeded arithmetic/transition test.
It does not prove a full observed wrap history. Exact RPC duplicate/logical
replay applies once in the new pending control. Outstanding2 plus a third call
fails with sticky `ObserverCapacity` after admission. Existing kind, known-FH,
error-prefix and source-provenance controls retain their narrow passing scope.
Static lifetime checks still require active state at1287/1331. These checks do
not compensate for the four ordering findings.

## Exact execution and reproduction

All test commands ran from the new review root's `run/` directory. Each command
ran once on its stated inventory. No required-suite rerun occurred.

| Commands (`/usr/bin/python3`) | Result per mode |
|---|---|
| `-B run_focused.py` and `-B -O run_focused.py` |55 pass; exit0; no failures/errors/skips |
| `-B public_probes.py` and `-B -O public_probes.py` |24 pass; exit0; unchanged independent module |
| `-B supplemental_probes.py` and `-B -O supplemental_probes.py` |8 pass; exit0; unchanged independent module |
| `-B adjacent_probes.py` and `-B -O adjacent_probes.py` |12 cases:7 pass,3 assertion failures,2 errors; exit1; no skips |
| `-B close_control.py` and `-B -O close_control.py` |1 pass; exit0; same-wire positive control |

The required inventories contain87 passing IDs per mode. The new probes contain
13 distinct IDs:8 pass,3 failures,2 errors per mode. Normal/optimized inventories
overlap. Subcases and author runs are not additional independent passes.

From the review root, `/usr/bin/python3 -B verify_review.py before` and
`/usr/bin/python3 -B verify_review.py after` pass. The first initial `before`
attempt stopped on the extra post-handoff receipt, before scratch creation or
tests. `inventory-stop.md` retains that stop and its resolution.
`/usr/bin/python3 -B summarize_results.py` passes; it reads existing evidence
only. It does not rerun candidate tests.

`run/*-{normal,optimized}.json` and matching logs retain exact commands, IDs,
counts, exits and tracebacks. `run/probe-inputs/` retains synthetic PDML and full
RPC/source-span JSON. `run/adjacent-observations/` retains the actual malformed
terminal successes, pending output, valid controls and errors. The retained
successful malformed reports still have `native=false` and `qualified=false`.
`result-summary.json` pins and compares all results, the three same-wire control
pairs and the quoted RFC excerpts.

For a correction review, make a new exclusive copy of `run/scratch`, the five
runner/probe modules, `guard.py`, `test-inputs.json` and `probe-inputs/`. Do not
copy existing result JSON/logs or `adjacent-observations/` into that output
location. Run the commands above there. The runners create output exclusively.
Do not rerun a one-shot freeze or overwrite any retained evidence.

## Pins and stability

| Input/artifact | SHA256 |
|---|---|
| Candidate manifest386 | `42f4fa292ff9b85e681debc4ae302a84369febe8fd91c494a0c1cd9954ea603b` |
| Candidate archive387,2454743bytes | `afccd44c4c349bd5a40c7a82bcc3ee7d902f912dad54724a81093e7d18c42a3c` |
| Exact six-path delta | `1f5475b5c921ca12640b635406dfa5d1b9ca1ab7e674bc393a673921d0b18e59` |
| Provider | `50da723762dbeba0ebd44dbe7ed17d29ad6dc3704b34c90688aa9593e33b82f7` |
| Owned tests | `c1c7839fca3ccb937f90a33101f90afd9e4d9adf55df2182ae0e8d0d577e6478` |
| Function-only helper | `67bac6edddcc1d41cc93b0ec1a63a684a08ef13ccc723ef8b7821205cf6ef12e` |
| New before receipt | `7a33b17cbe10c7eb50214f0359ccc5173d8babc9ac7ab430f12133226f0e2788` |
| New after receipt | `0e9915138a7344d35db5de5595348c21d489450a8e2f044c95f0c83f67273ebb` |
| New adjacent probes | `11745286e96e45606c1994c74d1e163e67b305a822a25f4ae4c25154ace9ea31` |
| New CLOSE control | `8c7ee6d24e37b0146052b962a4933dbccd54175758bbc76545157a65d098cccd` |
| New result summary | `7a6f9b9c20423515c8c89f795873db9531ed4543c878fbceaa89bd178cb114b8` |
| New review manifest151,31654bytes | `5ff7ab61595281379fc107f082289967b03ac096b259da2b050ed2e045e67edf` |
| RFC8881 | `3b7750c792f5e507cd69289cd1132034aed5ceadc03f5ddf24779426ea42474d` |
| Original author manifest37 | `8fd40b432dd77ef2e0990091143d193d0e9f0122174320fb315f4a00f63fd081` |
| Original author archive38 | `fcc30864086b885f4848cf39094b4fda590e5f5a4813fcd18b491a064b198f1c` |
| Original independent manifest88 | `e5bf5d98b33815e7381cd1d42762c2f7904f87293ef94e551f5f53234cafa337` |

Before/after verification checks all386 candidate members and exact387 ordinary
archive entries, byte lengths and modes. It also checks six primary/scratch pins,
the old37 author members/archive38,88 old review artifacts,277 prequota originals
and retained copies, ten reference bodies/maps, immutable primary ledgers/plan/
interface and all review runner/helper inputs. All six primary modes remain0664.
The six-path delta regenerates byte-for-byte. AST compilation at optimize0/2,
whitespace/newline checks and zero provider runtime assertions pass.

The interrupted startup and both quota histories remain retained. The review's
zero-byte `/tmp/opencode/astra-g3-wire-nfs41-r2-review-77see0sq/STOP-RESUME.md`
still exists. The post-handoff `handoff-digests.json` is verified separately;
it was not added to or removed from the candidate archive. Old malformed
successes, valid-input failures, status-position failures and author development
counts keep their original scope.

Only this new primary review ledger and exclusive new review artifacts changed.
Candidate source/tests/fixtures and old evidence remain read-only. Current core
and active P1/P3 outputs remained unread. No source fix, Git write, broad suite,
installed probe, capture, live query, native collection or delegation occurred.
External receipt validation, installed-producer qualification, P3/P4 integration,
native/full-G3 qualification, publication and live permission remain separate.
This review grants none of them.
