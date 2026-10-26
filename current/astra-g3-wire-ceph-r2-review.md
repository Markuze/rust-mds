# astra-g3-wire-ceph-r2-review
Task: Independent P1 r2 correction review of WCEP-1–3. Verify the frozen candidate, exact delta, reference inputs, and immutable prior evidence. Run the owned89 and unchanged reviewer19+3 once per Python mode from a new guarded copy. Return scoped offline acceptance or concrete findings.
Status: COMPLETE — SCOPED ACCEPT offline P1. WCEP-1–3 close on the exact frozen r2 candidate. Owned89, unchanged reviewer19, unchanged followup3, and new narrow boundary3 pass per normal/optimized mode, with0 failures/errors/skips. Startup799/pre-execution158/final2059 checks pass. No new finding.
Context: Own only this new report and new exclusive external review artifacts. Handoff: /mnt/workspace/sol-g3-wire-ceph-r2-20261026-4fiiyiho/HANDOFF.md. Candidate/source/tests/fixtures/old reports/contracts stay read-only. No Git writes, live/network/installed probes, native collection, source fixes, or delegation.
Next: Parent records this exact scoped decision and frozen review pins in current/sol-g3-provider-r2-orchestration.md. Installed qualification, P3/P4 joins and full G3 need their own decisions.
Artifacts: /mnt/workspace/astra-g3-wire-ceph-r2-review-axjfaqcd/. Fresh final candidate is in source/. The failed new /tmp/opencode/astra-g3-wire-ceph-r2-review-ckab3f_i/ remains retained. startup-verification.json records EDQUOT(errno122),799 checks and separate inventory counts.
Log:
- [2026-10-26] Created the sole writable repository report before review work.
- [2026-10-26] Read completed r2 handoff/ledger and original review. Verified candidate/archive/diff/reference pins, exact regenerated six-path/full diffs, seven0664 candidate entries,147 r2 artifact entries,63 reference inputs,86 r1 artifacts,96 prior review artifacts, and93 failed-attempt files in both original/fallback locations. All799 startup checks passed. Preserved a new quota-failed temporary extraction and used a new exclusive workspace fallback. No test ran during extraction.
- [2026-10-26] All158 pre-execution checks passed. Fresh copies match. The old78 test bodies are AST-identical. The pre-import guard prefix and reviewer scripts are unchanged. Both source/test modules compile at optimization0/1, contain no runtime assert, and pass whitespace checks. Pinned the one retained partial extraction file.
- [2026-10-26] Ran exact owned89, unchanged reviewer19, and unchanged followup3 once per normal/optimized mode. Each of the six commands exited0, with0 failures/errors/skips. Original public regressions now show reconnect chunks1/1/2/2 without false interruption; deferred FIN retains packet8/offset800000/time8000/flags17; decreasing RETRY_GLOBAL rejects; interleaved A65 rejects at the series bound. Source audit continues before a verdict.
- [2026-10-26 04:48 UTC] Completed the exact delta/reference review. Added only three external public boundary cases. Both modes pass3/0/0/0. Confirmed same-session SEQ across a changed client port, shared-advertisement endpoint isolation, and direct/deferred FIN immutability and budget release. Final audit passes2059 checks with0 mismatches. All old red evidence retains its original result. Return SCOPED ACCEPT offline P1 for WCEP-1–3. The external inventory and seal pin the completed report and evidence.

## Verdict

**SCOPED ACCEPT — offline P1, exact frozen r2.** WCEP-1, WCEP-2 and WCEP-3 close. The changed source agrees with the pinned protocol references and the independent controls. No new defect was found in the assigned delta.

Candidate identity is manifest `188ceb97b29d89aca5a8e3af582a62248fd2751753a80767cc68d3aedc5bb37b`, archive `cb717c6fd953ab7030e0f65cc1fc269603235efee6a8db66aade7dcbf599f1e7`. Review execution used a fresh extraction of that final archive at the external root's `source/`. It did not use the author's partial quota-failed `source/`.

The original r1 review remains **CHANGES REQUIRED** for its original candidate. Its16 pass/3 fail and2 pass/1 fail per mode remain failed evidence. The author's exact red-before reproductions retain those same results. This r2 decision does not relabel them.

## Finding closures

### WCEP-1 — closed: reconnect scope, continuity and lifetime bounds

Reviewed `scripts/g3_wire_ceph.py:554–571,813–836,917–951,963–1009`.

- Reconnect rows use observed client identity plus one shared server scope. Client identity contains capture/boot/netns/interface, advertised client address/nonce and client-origin entity. Server scope contains the actual TCP server IP/port, advertised server address/nonce and host/protocol. Address tuples use the decoder's fixed field order, including family, flowinfo and scope.
- Messenger continuity, reconnect accounting and server-feature lookup/update use that same server scope. The client ephemeral port is absent. A changed observed server endpoint or advertised identity creates a distinct row. The decoder makes no logical-rank or cross-address equivalence claim.
- Interleaved A/B reconnect chunks retain separate series and counters. The unchanged independent regression emits chunks1/1/2/2 without false interruption. The unchanged follow-up rejects A's65th chunk after64 alternating A/B chunks with `ObserverCapacity: reconnect series bound`.
- The owned corrections verify64 chunks/4096 caps/4096 realms per series. A newer epoch interrupts only that server's unfinished series. Same-epoch SEQ continues it. An obsolete nonreplayed epoch rejects before changing the chain. Transport replay labeling stays separate from new chunk accounting.
- `max_reconnect_chains` defaults to128 and accepts only integer1–128. Admission precedes row allocation. Rows remain for the reader lifetime. Finality reuses the same row for a later series; it does not evict a peer or release row capacity. The reduced1-row test verifies rejection without row-table changes. Existing stream, session and client limits keep the related identity tables bounded.

New public boundary controls add two concrete checks:

1. An established connection opens features and starts a reconnect series. A new TCP connection changes only the client port and performs SEQ. The messenger epoch stays4, the predecessor stream points to the original stream, and the final chunk is series1/chunk2. The original server feature receipt still selects valid delegated CREATE decoding.
2. Two actual server ports advertise the same address/nonce. SEQ on the second endpoint without its own predecessor rejects with `WireCeph1: unobserved messenger session predecessor`. The valid READY control keeps separate epochs4/10 and chunks1/1/2/2. The second endpoint does not inherit the first server's feature receipt. A later first-endpoint connection recovers receipt6; the other endpoint retains receipt12. Both delegated and nondelegated CREATE replies decode with their own observed feature bases.

Reference reasoning reuses the verified original review. Pinned v20.2.0 `Client.cc:3293–3377`, `Client::send_reconnect(MetaSession*)`, sends chunks through the selected MDS session. `Server.cc:1566–1572,1664–1674,1733–1738` handles finality on that receiving session. The correction preserves this boundary.

### WCEP-2 — closed: original FIN source and owned budget

Reviewed `scripts/g3_wire_tcp.py:57–66,95–106,334–346,390–410`.

- The first FIN stores one frozen `SourceRef` after a256-byte shared-budget admission. The direction owns that retained reference. The FIN span refers to it; releasing the span does not release the direction's retained charge.
- Deferred emission uses the original reference. Duplicate FINs do not replace it or add a second anchor charge. The reference keeps the original packet, time, source offset, complete packet range and terminal stream position. The packet range contains the original flags; flags are verified from the immutable synthetic input, not invented as a `SourceRef` field.
- The unchanged public regression now reports packet8/offset800000/time8000 and TCP flags17 after packet9 fills the gap. The complete message still decodes before the FIN event.
- Owned cases verify payload-bearing and empty deferred FINs, duplicates, conflicting FINs, unfilled holes and the255/256-byte admission boundary. The failed admission allocates no reference. Successful finish includes the retained anchor in the ownership check and releases it with the overlap history.
- The new public case covers direct and deferred FIN. Both retain packet4/range[0,43)/flags17. Normal attribute mutation raises `FrozenInstanceError`. The retained reference remains equal after duplicates and successful finish. The public budget falls from history+256 to0 in both paths.

Conflict, data-beyond-FIN, incomplete-hole and EOF checks retain their original fail-closed behavior. No new transport profile was added.

### WCEP-3 — closed: monotonic RETRY_GLOBAL and explicit rollover policy

Reviewed `scripts/g3_wire_ceph.py:685–703` and pinned v20.2.0 `ProtocolV1.cc:1710–1715`.

The next CONNECT generation must exceed `max(previous_client_generation, server_hint)`. The lower hint90 with client100→101 stays valid. Hint101 with new102 stays valid. Larger jumps remain valid. The final nonwrapped value`0xffffffff` remains valid when it exceeds both preceding values.

The unchanged public failure100→hint90→91 now rejects with `WireCeph1: RETRY_GLOBAL generation did not advance`. Owned controls reject equality/decrease relative to either preceding value. If either preceding value is already`0xffffffff`, a wire-valid CONNECT with wrapped0 rejects with `WireCeph1: RETRY_GLOBAL u32 rollover unsupported`. This is the named protocol rejection, not a packing/setup failure.

Reconfirmed the exact review-added local `AsyncMessenger.cc:1065–1096` and `.h:293–296,401–408`. The allocator uses a u32 counter, raises it to the hint if needed, then increments under its lock. Other connections can advance it further. The observer correctly permits larger jumps and declares rollover unsupported. These are pinned source references; no C++ executable ran.

## Independent execution evidence

All eight commands ran from `/mnt/workspace/astra-g3-wire-ceph-r2-review-axjfaqcd/source/`. Each command ran once. The guards install before either provider import. The owned runner loads exactly the two named modules. The exact reviewer scripts and their predicates remain unchanged.

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../run_owned.py ../owned-normal.json > ../owned-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../run_owned.py ../owned-optimized.json > ../owned-optimized.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../probes/independent_cases.py ../independent-normal.json > ../independent-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../probes/independent_cases.py ../independent-optimized.json > ../independent-optimized.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../probes/followup_cases.py ../followup-normal.json > ../followup-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../probes/followup_cases.py ../followup-optimized.json > ../followup-optimized.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../probes/r2_boundary_cases.py ../boundary-normal.json > ../boundary-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../probes/r2_boundary_cases.py ../boundary-optimized.json > ../boundary-optimized.log 2>&1
```

| Execution | Normal | Optimized | Exit per mode |
|---|---:|---:|---:|
| Owned89: original78 plus11 | 89 pass | 89 pass | 0 |
| Unchanged independent19 | 19 pass | 19 pass | 0 |
| Unchanged followup3 | 3 pass | 3 pass | 0 |
| New narrow public boundary3 | 3 pass | 3 pass | 0 |

All runs have0 failures/errors/skips. Counts are unittest methods; subcases do not increase them. The owned suite has26 TCP and63 Ceph/static methods. All78 prior method ASTs and the guard prefix remain identical. The three new cases use the exact independent packet/frame builders or small synthetic TCP builders. They do not use provider encoders, mock provider state or alter the original probes.

Python is3.14.4. The owned run maps retain248 normal/137 optimized runtime files. Their248-file union matches at final audit. Author89/19/3 and author1073-check results remain separate from these independent executions.

## Preservation and static audit

From the exclusive review root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 audit_review.py > final-audit.log 2>&1
PYTHONDONTWRITEBYTECODE=1 python3 freeze_review.py
```

Startup passed799 checks. Pre-execution passed158. Final audit exited0 with2059 checks and0 mismatches. The final audit verifies434 startup-tracked paths,73 pre-execution copied paths, complete retained inventories, archive member types/bytes/sizes/modes, exact regenerated diffs, the original test ASTs/guards, compile optimization0/1, runtime assertions, whitespace, all eight result/log pairs and the248-file runtime union. These check counts overlap; they are not test counts.

Separate retained inventories remain:

| Inventory | Files | Result |
|---|---:|---|
| Completed r2 author artifacts | 147 | All pins and membership match |
| Reference inputs | 63 | All copies/archive entries match;59 protocol+2 contract/plan+2 allocator |
| Original frozen r1 artifacts | 86 | All pins and membership match |
| Original review artifacts | 96 | All pins and membership match |
| Author quota-failed attempt | 93 | Original entries and fallback copies match |

All seven candidate files retain mode0664. The six shared source/test/fixture paths produce exactly `r2.diff`. Adding only the NEW ledger produces exactly `r2-candidate.diff`. The old ledger remains outside that new candidate and unchanged. Literal `vectors.json` bytes remain identical.

The new review quota failure remains at `/tmp/opencode/astra-g3-wire-ceph-r2-review-ckab3f_i/`. Startup records EDQUOT(errno122). Its one partial file matches the pre-execution pin at final audit. The exclusive workspace fallback holds all successful review work. The author quota failures and older70/77-test failures remain preserved.

`review-report.md` is the completed report copy. `review-artifacts-manifest.json` pins all regular files in this exclusive review root, except that manifest and `review-seal.json`. The seal pins the inventory, report, final audit and candidate identities. The freeze verifies those files and the final observed input map. Candidate modes stay intact.

## Exact pins

| Input/evidence | SHA256 |
|---|---|
| r2 candidate manifest | `188ceb97b29d89aca5a8e3af582a62248fd2751753a80767cc68d3aedc5bb37b` |
| r2 candidate archive | `cb717c6fd953ab7030e0f65cc1fc269603235efee6a8db66aade7dcbf599f1e7` |
| Six-path delta | `dff57c1021c87f90b4bf0dbbcd1d41ddc7745611ae7ab7af8281990e101620e7` |
| Full candidate delta | `ca87812427fbc02d92a4be560f4b758f168c746a5d0e3ef2c6f288bfae568d3f` |
| Reference archive | `0fde305521ba7287415f0a92c1ac7ed317b954dd0a116225a56eade3cbea80ce` |
| Reference pins | `05091b1b7b037188547675692c03a84cb7ab4c0843651eb32cc7e43a087e03ba` |
| r2 Ceph source | `bed41157074d3a9eb05c46b05ab5c80122d7ed182f09663e87d71581953a953f` |
| r2 TCP source | `a50f4ca1b11bb6bfb0652abe4ee0e1c706ff9a8c782ab16bc6a1eca7b75ac3ba` |
| Exact independent19 script | `e4a6721656153d3abb68f63345da4c01ebecef98e5d03b16138854101cfd2bac` |
| Exact followup3 script | `e20da371a121382b4dce724de90ee13ba8c0f269a5278a359c4aa955e07515ab` |
| New boundary3 script | `e64915bf35a0b317f68f403195286ce123fd953a4362afaf76134ede7e496910` |
| Startup799 audit | `21c0ca84fdee83b3962a78a42c90ee71a14edc543042165e88c66d1710a6fc9a` |
| Pre-execution158 audit | `a724bcb496d42d82d74549b5ec4aff6eadad8c8ca1e9bbf8cb3681890aa6da5c` |
| Final2059 audit | `baf1fd942ea76ed2f4ad9ae29a50743e362aebf1dd385ff961a91207bb8a700b` |
| Original review96 inventory | `3bea48c6c8791699cd23ccebecca2371163dc94fcf25affa54e3815bc8bc9240` |
| Original r1 review report | `832ce24a32b89e277ff73ce8fb2a111cc6a7475b4a71d4f628ff90bbb3b34c27` |
| Review-added AsyncMessenger.cc | `47dff709e86e92676119504ee30774583e2f4d49155b84a89e22c74585918050` |
| Review-added AsyncMessenger.h | `7a3828732ec2e7dd4b1d5ce0f31a538280a3c4cb33264212125920e2a45a95f7` |

## Scope

Acceptance covers only this frozen offline P1 decoder delta and closes WCEP-1–3. The original review's unchanged protocol/layout reasoning and supported-profile limits remain applicable. Identity continuity remains passive evidence. Installed source loss/counters, clock barriers, authentication/peer processing, P3/P4 joins and full G3 remain separate qualifications.

Only this NEW repository report and the NEW exclusive review artifacts were written. Candidate/source/tests/fixtures, original reports/evidence and contracts stayed read-only. Canonical documents and the parent's orchestration ledger stayed outside this write scope. No Git write, delegation, source fix, live/network/capture/installed probe, native collection, active daemon/fault/P2/P3 output read, broad Python discovery or Rust suite occurred. The review makes no repository-wide or publication-state claim.
