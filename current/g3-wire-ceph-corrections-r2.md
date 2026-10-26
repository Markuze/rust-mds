# g3-wire-ceph-corrections-r2

Task: Assign P1 r2 corrections for WCEP-1–3. Preserve the original49f8f7e6 frozen archive, author ledger, review and evidence. Correct only the six assigned source/test/fixture paths. Use this new ledger and new exclusive artifacts. Reproduce the exact reviewer failures against verified frozen r1 before source edits. Then run the owned focused suite once per Python mode and both exact reviewer suites per mode against stable corrected source. Freeze a new candidate for independent r2 review.
Status: AUTHOR COMPLETE/FROZEN for independent r2 re-review. WCEP-1–3 corrections pass the owned89 tests once per Python mode and the exact unchanged reviewer19+follow-up3 per mode, with0 failures/errors/skips. Final static/input/preservation audit passes1073 checks. The seven-path source/test/fixture/NEW-ledger candidate and exact archive identities are in the canonical HANDOFF below. The tested six files did not change after execution. Red-before remains16/3 and2/1 per mode. This is author evidence, not independent acceptance or native readiness.
Context: Owned paths are scripts/g3_wire_tcp.py, scripts/g3_wire_ceph.py, scripts/tests/test_g3_wire_tcp.py, scripts/tests/test_g3_wire_ceph.py, scripts/tests/fixtures/g3-wire-ceph/README.md, scripts/tests/fixtures/g3-wire-ceph/vectors.json, and this new ledger. Original current/g3-wire-ceph.md stays frozen. No core/P2/P3/crate/contract/other-doc edit, active-output read, Git action, live action, process control, capture, native collection or delegation is authorized. Keep the existing pre-import native-action guards. Use synthetic bytes/local files only.
Next: Parent assigns independent r2 re-review from /mnt/workspace/sol-g3-wire-ceph-r2-20261026-4fiiyiho/HANDOFF.md. Review the pinned candidate, retained red-before probes and correction delta. Do not alter this candidate or original evidence. Further source corrections require a new assignment. Native construction, installed qualification, P3/P4 joins and full G3 remain outside this package.
Log:
- [2026-10-26] Created the r2 ledger first. WCEP-1–3 are three P2-severity Ceph-provider findings, not the separate NFS package. No correction acceptance or native qualification is claimed.
- [2026-10-26] Initial exclusive artifact root: /tmp/opencode/g3-wire-ceph-r2-20261026-2n1666_5/. Initial allocation succeeded; the later EDQUOT fallback is recorded below. Verified original manifest49f8f7e693d310979cf6fce2715fe3fcd8a9749d80c68aba6595b83d25e77287 and archive97ca85d4a6eb1bd88b414c2732a47fd1bdfe836ecc092734b99ab44c3fae3eed. Twenty-one primary/frozen/review file preimages match. New r1-source/ holds the verified seven-file extraction. Original directories stay read-only.
- [2026-10-26] Exact independent_cases.py SHA256e4a6721656153d3abb68f63345da4c01ebecef98e5d03b16138854101cfd2bac and followup_cases.py SHA256e20da371a121382b4dce724de90ee13ba8c0f269a5278a359c4aa955e07515ab match the completed review inventory. Copied them unchanged into new probes/. New red-independent-{normal,optimized}.{json,log} reproduce WCEP-1/2/3; red-followup-{normal,optimized}.{json,log} reproduce WCEP-1's65th-chunk bypass. All four commands exit1 with ordinary assertion failures and0 errors/skips. Positive cases remain16/2 per mode. Counts overlap and are not added to r1 author78.
- [2026-10-26] Read pinned v20.2.0 ProtocolV1::handle_connect_reply_2, Client::send_reconnect and Server final-chunk handling. Read review-pinned local AsyncMessenger::get_global_seq and its u32 declaration. The allocator raises the current counter to the hint if needed, then increments. Both added reference hashes match the review. No executable or native probe ran.
- [2026-10-26] Implemented the three planned corrections. Added11 regressions: independent A/B series with distinct or identical advertisements; per-server chunk/cap/realm pressure; finite chain-row admission/lifetime; same-epoch SEQ and server-local interruption; obsolete-epoch rejection; complete deferred-FIN message with exact packet8 provenance; original/duplicate/empty FIN anchors and charged allocation boundaries; and RETRY_GLOBAL positive/negative/rollover matrices with valid u32 wire fields. Existing pre-import guards and all literal vectors remain byte-identical.
- [2026-10-26] Read the entire574-line source/test/fixture delta in new r2.diff. Four Python ASTs contain0 runtime assert statements and compile for optimization0/1. Owned whitespace preflight passes. Original78 method names remain in the89-test inventory (26TCP+63Ceph/static). All61 retained r1 source-format/contract archive members verify before extraction into new references/. These static checks are not final test results.
- [2026-10-26] EDQUOT(errno122) stopped the first corrected-source copy under /tmp/opencode/g3-wire-ceph-r2-20261026-2n1666_5/source/. No corrected-source test had run. Preserved that attempt, including its partial source file and all red-before evidence. Created NEW authorized fallback /mnt/workspace/sol-g3-wire-ceph-r2-20261026-4fiiyiho/; copied and byte/size/mode-verified all93 attempt files without changing the original attempt. quota-fallback.json records the failure and copy map. Use fresh source-corrected/ there, not the partial source/ tree.
- [2026-10-26] Final owned execution ran once per mode from source-corrected/:89 pass normally(2.169s),89 pass optimized(2.136s),0 failures/errors/skips. Exact reviewer19 and follow-up3 also pass per mode with0 failures/errors/skips. Six commands exit0. They use the unchanged31 pre-import process/network/signal guards. These author reruns of reviewer scripts are not independent r2 acceptance. Suites/modes overlap and are not added together.
- [2026-10-26] Checked actual corrected probe observations: A1/B1/A2/B2 chunks are1/1/2/2, each server retains its own series, and previous_incomplete staysfalse. Deferred FIN reports original packet8/offset800000/time8000 with actual FIN flags17. RETRY_GLOBAL100→hint90→91 rejects as generation did not advance. Alternating64 A/B nonfinal chunks then A65 rejects exactly ObserverCapacity: reconnect series bound. No parse/setup error substitutes for a finding's intended result.
- [2026-10-26] All six primary/corrected-copy source/test/fixture entries match their before-test SHA256/size/mode pins. The normal248/optimized137 mode-specific runtime maps have a stable248-file union. source-after-tests.json, runtime-after-tests.json and corrected-results-summary.json record the checks. Only this r2 ledger changes after testing.
- [2026-10-26] Final static/input/preservation audit exits0 with1073 checks. All86 original r1 artifact entries and96 completed-review artifact entries match. All63 reference/contract inputs, original seven-file extraction, exact public probes and248 runtime-input union pins match. All original78 test bodies remain AST-identical. Four Python files compile for optimization0/1 and contain0 runtime assert statements. The full guard prefix, literal vectors and exact574-line r2.diff remain unchanged.
- [2026-10-26] Closed this ledger for the seven-path freeze. The candidate contains the six tested paths and this NEW ledger only. candidate-manifest.json, candidate.tar.gz, archive-verification.json and HANDOFF.md carry the final identities and membership/mode checks. r2.diff stays unchanged; r2-candidate.diff adds this ledger without deleting the original ledger. The r1/r2 inventory union is eight paths. Complete reference, fixture, command, runtime, preservation and artifact inventories are retained. Only this ledger's source-corrected/ counterpart is updated after tests. No Git command, live action, process control, capture, native collection, core/P2/P3/crate/contract edit or delegation ran.

## Implemented corrections and limits

- WCEP-1: Keep client capabilities client-scoped. Key reconnect accounting by that observed client plus actual server TCP endpoint, advertised server address/nonce and negotiated host/protocol. Use the same server scope for messenger and server-feature continuity. Keep at most128 chain rows for the reader lifetime; admit a row before allocation and never evict it. A reducible max_reconnect_chains limit permits a public pressure regression. SEQ retains the same chain; a newer epoch interrupts only that server's chain. Do not join logical ranks across addresses. Reject an obsolete epoch rather than rolling its scoped counters backward.
- WCEP-2: Store one immutable original FIN SourceRef with its byte position. Charge256 bytes before creating that retained record. Duplicate FIN does not replace it. Deferred emission uses the retained packet/time/offset/range, not the gap-filling packet. Release the retained charge only at successful TCP finish. Keep conflicting-FIN/hole checks.
- WCEP-3: Require a retry generation greater than both the previous client value and server hint. Permit a lower hint when the client advances correctly. Define the supported u32 policy without wrap; reject rollover explicitly. Keep unrelated negotiation fields unchanged.

## Actual focused results

| Suite | Normal Python | Optimized Python |
|---|---|---|
| Verified frozen-r1 exact independent19 | 16 pass/3 ordinary failures; exit1 | 16 pass/3 ordinary failures; exit1 |
| Verified frozen-r1 exact follow-up3 | 2 pass/1 ordinary failure; exit1 | 2 pass/1 ordinary failure; exit1 |
| Corrected owned89 (26 TCP+63 Ceph/static) | 89 pass;2.169150014s;exit0 | 89 pass;2.136340106s;exit0 |
| Corrected exact independent19 | 19 pass;exit0 | 19 pass;exit0 |
| Corrected exact follow-up3 | 3 pass;exit0 | 3 pass;exit0 |

All rows have0 errors/skips. Corrected rows have0 failures. Suite families and modes overlap. Do not add them. Original78 and earlier70/77 results remain historical. Replaying reviewer scripts here is not independent r2 acceptance. No full Python discovery, Rust suite or native test ran.

The new11 owned regressions preserve byte-level protocol expectations. They include valid controls. Invalid generation/rollover and pressure probes require named WireError rejections, not parse/setup errors.

## Exact execution commands

Red-before working directory: `/tmp/opencode/g3-wire-ceph-r2-20261026-2n1666_5/r1-source`.
All four commands returned1. Their copied evidence is also in the canonical fallback root.

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../probes/independent_cases.py ../red-independent-normal.json > ../red-independent-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../probes/independent_cases.py ../red-independent-optimized.json > ../red-independent-optimized.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../probes/followup_cases.py ../red-followup-normal.json > ../red-followup-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../probes/followup_cases.py ../red-followup-optimized.json > ../red-followup-optimized.log 2>&1
```

Corrected working directory: `/mnt/workspace/sol-g3-wire-ceph-r2-20261026-4fiiyiho/source-corrected`.
All six commands returned0. Each owned-suite command ran exactly once after the stable fixes.

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../run_owned.py ../owned-normal.json > ../owned-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../run_owned.py ../owned-optimized.json > ../owned-optimized.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../probes/independent_cases.py ../corrected-independent-normal.json > ../corrected-independent-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../probes/independent_cases.py ../corrected-independent-optimized.json > ../corrected-independent-optimized.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../probes/followup_cases.py ../corrected-followup-normal.json > ../corrected-followup-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../probes/followup_cases.py ../corrected-followup-optimized.json > ../corrected-followup-optimized.log 2>&1
```

Final static audit working directory: `/mnt/workspace/sol-g3-wire-ceph-r2-20261026-4fiiyiho`.
This command returned0. It did not run tests.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 audit_r2.py > final-audit.log 2>&1
```

`commands.json` records exact working directories, commands, returned exit codes and evidence paths. Packaging commands and archive verification are recorded in HANDOFF.md. No test reran during packaging.

## Tested source identities

All six entries have actual mode0664. `source-before-tests.json` and `source-after-tests.json` are identical. Only this ledger closes after testing; its final identity is in candidate-manifest.json.

| Path | SHA256 |
|---|---|
| `scripts/g3_wire_ceph.py` | `bed41157074d3a9eb05c46b05ab5c80122d7ed182f09663e87d71581953a953f` |
| `scripts/g3_wire_tcp.py` | `a50f4ca1b11bb6bfb0652abe4ee0e1c706ff9a8c782ab16bc6a1eca7b75ac3ba` |
| `scripts/tests/test_g3_wire_ceph.py` | `bb3b098990585dee2ec8180fd9fb1a686063afcbfa5a81a501ffd36a325ef1c3` |
| `scripts/tests/test_g3_wire_tcp.py` | `f647c0e0b1a5a9c985bf29696d9b49745b1d035a4adee1230ee882781bb663f2` |
| `scripts/tests/fixtures/g3-wire-ceph/README.md` | `3aabb735d633d8338cfbbbc5e3b3abe8642d452809845b3b26e73e2862b30ad6` |
| `scripts/tests/fixtures/g3-wire-ceph/vectors.json` | `08e2fdb8df9779dc53db3d24cf76852177baa29be6d8b739a25f60f4e9499d7c` |

The six-path delta has SHA256`dff57c1021c87f90b4bf0dbbcd1d41ddc7745611ae7ab7af8281990e101620e7`,37084 bytes and574 lines. It includes no literal-vector change. The full candidate delta adds only this NEW ledger. Its final hash and both new archive hashes are external in HANDOFF.md to avoid a recursive ledger hash.

## API and qualification boundary

The public flow stays `TcpReader.feed(PacketMeta, packet_bytes)` → spans → `CephReader.feed(span)` → sanitized records. Call `CephReader.finish(barrier)` before `TcpReader.finish(barrier)`. Supply exactly32 salt bytes. A TCP-only consumer must release each span. The fixture README is the full API/profile document.

The new reducible keyword is `CephReader(..., max_reconnect_chains=128)`. Rows are admitted before allocation and remain for the reader lifetime. Final series do not evict a row. Each series permits at most64 chunks,4096 caps and4096 realms. FIN anchors retain original source coordinates and consume256 shared-budget bytes until successful TCP finish. RETRY_GLOBAL does not support u32 rollover.

Other limits stay bounded:8 active TCP streams,128 retained streams,4 MiB per-direction shared byte-buffer budget,65536-byte overlap history,4096 pending segments,128 messenger peer/client rows,16 negotiation rounds,1 MiB metadata/reconnect payload and64 MiB skipped payload. Defaults are100000 packets/records, with explicit maxima1000000. The byte budget is not a CPython RSS bound.

Supported transport remains Ethernet-II IPv4/TCP or raw IPv4/TCP with SYN negotiation. Messenger remains lossless msgr1 `ceph v027`, client8 to MDS protocol32. Required metadata fronts and their version gates stay unchanged. msgr2, unsupported links/versions, holes, conflicting overlaps, invalid negotiation and capacity pressure fail closed. TCP ACKs, signatures that were not verified and passive identity joins are not peer-processing or authentication proof.

The63 retained reference inputs comprise59 original protocol references,2 frozen contract/plan snapshots and2 review-added allocator sources. Original reference archive SHA256`10ab927ac6e66cbbc941db4fee5b32c0f8330c76ecd794cee6d4b77145b40463` remains preserved. Added allocator pins are AsyncMessenger.cc`47dff709e86e92676119504ee30774583e2f4d49155b84a89e22c74585918050` and AsyncMessenger.h`7a3828732ec2e7dd4b1d5ce0f31a538280a3c4cb33264212125920e2a45a95f7`. Retained fault-v1 stays at`3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523`. No contract was edited.

Canonical handoff: `/mnt/workspace/sol-g3-wire-ceph-r2-20261026-4fiiyiho/HANDOFF.md`.
The incomplete quota-failed `source/` tree is retained evidence only. The tested tree is `source-corrected/`; the final review tree is `candidate/`. Original r1 and completed-review directories remain unchanged. No Git status/publication or global primary-tree stability is claimed. This package supplies no native source, native-mode switch or installed-provider qualification. P3/P4 joins, hooks and full G3 remain outside scope.
