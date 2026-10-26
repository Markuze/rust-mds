# astra-g3-wire-ceph-review
Task: Independent P1 offline TCP/msgr1/Ceph decoder review. Verify frozen source, archive pins and modes. Run the focused 78 tests once per Python mode with pre-import guards. Review supported protocol behavior and add independent public-API cases. Source, tests, fixtures, reports, and contracts remain read-only. No live collection, traffic, process control, Git writes, or delegation.
Status: COMPLETE — CHANGES REQUIRED for offline P1. Three severity-P2 findings have public synthetic reproductions in both Python modes. Supplied78 pass per mode; independent19 yield16 pass/3 fail; narrow follow-up3 yield2 pass/1 fail. Final298 input/archive/copy checks pass. No scoped acceptance is issued.
Context: Own this new ledger and exclusive external review artifacts only. Candidate handoff: /tmp/opencode/g3-wire-ceph-20261026-i7jxz0dh/frozen-r1-07_6u5it/HANDOFF.md. Scope is offline P1, not installed capture readiness or native G3.
Next: Parent assigns corrections for WCEP-1–3. Review the corrected frozen delta and retained public failures before offline acceptance. P3/P4 joins and installed-provider qualification remain separate work.
Artifacts: /tmp/opencode/astra-g3-wire-ceph-review-x5_cwplq/. Candidate is in source/; pinned references are in references/. startup-verification.json records supplied pins, complete artifact inventory, archive membership, bytes, sizes, and modes. No EDQUOT occurred.
Log:
- [2026-10-26] Created review ledger before review work.
- [2026-10-26] Read handoff, fixture/API profile, completed provider plan, accepted interface, and pre-import guards. Verified supplied manifest/archive pins and all retained artifact entries. Extracted seven candidate files and 61 reference/contract files into an exclusive review directory. All 220 startup checks passed. Preserved actual 0664 candidate modes.
- [2026-10-26] Ran the supplied focused suite once normally and once optimized from source/: 78 pass in each mode. Added external independent_cases.py with independent byte builders and the supplied pre-import guards. Both modes returned 16 pass/3 fail. Confirmed deferred-FIN source corruption, cross-server reconnect-chain mixing, and decreasing RETRY_GLOBAL generation acceptance. Retained all failed assertions and emitted receipts.
- [2026-10-26] Read pinned C++20.2/main and Rust layout/state references. Added read-only, hash-pinned copies of local C++ AsyncMessenger.cc/.h to confirm the RETRY_GLOBAL allocator rule. Ran only three narrow follow-up cases per mode for endpoint-correct CAPS, valid RETRY_GLOBAL controls, and the reconnect chunk bound. Original tests and failed evidence remain unchanged.
- [2026-10-26] Final audit passed298 checks. All220 startup checks repeat unchanged. Seven extracted candidate files,61 reference/contract copies, six copied harness/manifest files, and two added C++ references/corresponding sources match. Four candidate Python files pass AST and trailing-whitespace checks. Completed this report with three severity-P2 findings. No source correction was made.

## Verdict

**CHANGES REQUIRED — offline P1.** The required front layouts have substantial positive evidence. Three state/provenance errors block scoped acceptance. They produce successful drained receipts with incorrect metadata or a contradictory negotiation. Each finding below uses only `TcpReader.feed` → `CephReader.feed` and their public `finish` methods. Synthetic inputs use the supplied pre-import process/network/signal guards.

Severity P2 below means a review priority. It does not refer to the separate NFS provider package.

## Ranked findings

### WCEP-1 — P2: reconnect chains mix distinct server sessions and bypass the chunk bound

**Location:** pinned `scripts/g3_wire_ceph.py:801–814`, especially `client.setdefault('reconnect', ...)` at803. `_client` at936–942 keys the client independently of the server. The single reconnect state therefore belongs to every server connection for that client.

**Public reproduction:** `independent_cases.py::IndependentTests.test_18_reconnect_chunks_are_scoped_to_client_and_server`. Create two valid TCP/msgr1 connections with the same client address/nonce/entity. Use distinct server ports and advertised server nonces. Send complete v5 chunks in order A1(more), B1(more), A2(final), B2(final). All frames have valid CRCs and complete source ranges.

- Actual `(stream, epoch, series, chunk, previous_incomplete)`:
  `(1,4,1,1,false)`, `(2,8,2,1,true)`, `(1,4,3,1,true)`, `(2,8,4,1,false)`.
- A2 must be chunk2 of A1's series. B2 must be chunk2 of B1's separate series. Interleaving the other server does not interrupt either series.
- Both finish receipts return `drained=true`. The failed assertion is `(1,3,1) != (1,1,2)` for A's series/chunk tuple.
- `followup_cases.py::FollowupTests.test_interleaving_cannot_bypass_64_chunk_limit_per_session` alternates64 nonfinal A/B chunks. A's65th chunk passes as `series129/chunk1`. Both finish receipts succeed. Thus the declared64-chunk bound is not applied to the actual session series.

**Reference:** pinned C++20.2 `Client.cc:3293–3377`, `Client::send_reconnect(MetaSession*)`, selects a specific MDS and sends all parts through that session. `Server.cc:1566–1572,1664–1674,1733–1738` handles finality on the receiving MDS session. Provider-plan§3.2 requires tracking all chunks on the bound session.

**Minimal fix:** key reconnect state by the observed client **and server/session identity**. Preserve SEQ continuity and detect epoch interruption within that scope. Do not use an unrelated server's epoch to reset the current series. Admit any added identity row before allocation. Apply chunk/cap/realm counters to that same scoped chain. Keep cross-address logical-rank joins with the separately qualified identity layer.

### WCEP-2 — P2: deferred FIN cites the gap-filling packet as its source

**Location:** pinned `scripts/g3_wire_tcp.py:333–340,369–374`. The direction stores only the FIN byte position. When later data fills a preceding hole, `_event(..., m, 'fin')` uses the later packet's metadata.

**Public reproduction:** `independent_cases.py::IndependentTests.test_17_reordered_fin_keeps_original_fin_source`. After valid TCP/msgr1 establishment, send packet8 with ACK+FIN and message bytes20–end. Then send packet9 with ACK+PSH and bytes0–19. Consume all emitted spans in order.

- The complete SESSION message decodes. The deferred FIN event claims packet9, source offset900000, timestamp9000 and packet range`[0,60)`.
- Packet9 has TCP flags24 (`ACK|PSH`), with no FIN. The FIN is in packet8, at source offset800000 and timestamp8000.
- Both finish receipts return `drained=true`. The failed public assertion is reported packet9 != original FIN packet8.

**Impact:** the terminal event loses its original source anchor and observation time under supported TCP reordering. A consumer cannot use its source reference to verify the stated event. This violates the source-range/provenance contract even though the message data reassembles correctly.

**Minimal fix:** retain one bounded original FIN source reference/metadata record with the FIN sequence position. Emit that record when the preceding bytes become contiguous. A duplicate FIN must not replace the original anchor with an unrelated gap-filling packet. Retain existing conflicting-FIN and hole checks.

### WCEP-3 — P2: RETRY_GLOBAL accepts a decreasing client generation

**Location:** pinned `scripts/g3_wire_ceph.py:682–691`. For tag5, the code checks only `new_global_seq > reply.global_seq`. It exempts equality with the previous connect value but supplies no monotonic check against that previous value.

**Public reproduction:** `independent_cases.py::IndependentTests.test_19_retry_global_cannot_decrease_client_generation` sends:

1. CONNECT with client global_seq100/connect_seq0.
2. RETRY_GLOBAL with hint90.
3. CONNECT with client global_seq91/connect_seq0.
4. READY with server global_seq200/connect_seq1.

The candidate emits all four negotiation records and returns successful Ceph/TCP finish receipts. It accepts the contradictory client transition100→91.

**Reference:** pinned C++20.2 `ProtocolV1.cc:1710–1715` calls `messenger->get_global_seq(connect_reply.global_seq)`. The additional pinned local-main `AsyncMessenger.cc:1065–1084` first raises its counter to the supplied value when needed, then increments it. A lower hint does not reduce the existing counter. The main helper was read and copied only; no C++ executable ran.

**Minimal fix:** validate the new client generation against both the prior CONNECT generation and the RETRY_GLOBAL hint, under the declared u32 policy. Reject an unsupported rollover explicitly. The narrow positive control proves that hint90/new101 and hint101/new102 both pass. Do not reject a lower hint when the client correctly advances its own generation.

## Execution evidence

All commands below ran from `/tmp/opencode/astra-g3-wire-ceph-review-x5_cwplq/source`. These are retained execution commands. A reproduction must use new output names to preserve the existing evidence.

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../run_focused.py ../normal.json > ../normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../run_focused.py ../optimized.json > ../optimized.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../independent_cases.py ../independent-normal.json > ../independent-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../independent_cases.py ../independent-optimized.json > ../independent-optimized.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 ../followup_cases.py ../followup-normal.json > ../followup-normal.log 2>&1
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O ../followup_cases.py ../followup-optimized.json > ../followup-optimized.log 2>&1
```

| Run | Normal | Optimized | Exit per mode |
|---|---:|---:|---:|
| Supplied focused suite, exactly two modules | 78 pass | 78 pass | 0 |
| Independent public cases | 16 pass / 3 fail | 16 pass / 3 fail | 1 |
| Narrow follow-up | 2 pass / 1 fail | 2 pass / 1 fail | 1 |

All six runs have zero errors and zero skips. The independent failures are ordinary failed assertions, not expected-failure labels. Test counts include unittest methods; version/field subcases do not increase those counts. The supplied78 comprise22 TCP and56 Ceph/static tests. They ran once per mode. Python is `/usr/bin/python3`,3.14.4. The supplied JSON records interpreter/runtime input hashes and loaded modules.

`independent_cases.py` reuses only the supplied guard installer. Its packet, address, message/front and CRC builders are independent of the author's helpers and provider encoders. It uses synthetic bytes only. The follow-up supplies endpoint-correct client FLUSH/server ACK pairs for CAPS10–14. Its covering/exact ACK checks use changed cap-id/sequence/dirty metadata. Earlier CAPS matrix cases validate front layouts; they are not all endpoint-valid conversations.

Final audit command, from the exclusive artifact root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 audit_review.py > final-audit.log 2>&1
```

Result: exit0,298 checks, zero mismatches. `final-verification.json` records all checks and separate retained test results. It repeats all220 startup checks unchanged, then checks the68 extracted files, six copied harness/manifest files and four added-reference source/copy comparisons. AST parsing and trailing-whitespace checks pass for all four candidate Python files. This audit result does not convert the public failures into passes.

## Pins and immutability

| Input | SHA256 |
|---|---|
| Supplied candidate manifest | `49f8f7e693d310979cf6fce2715fe3fcd8a9749d80c68aba6595b83d25e77287` |
| Seven-file candidate archive | `97ca85d4a6eb1bd88b414c2732a47fd1bdfe836ecc092734b99ab44c3fae3eed` |
| Reference archive | `10ab927ac6e66cbbc941db4fee5b32c0f8330c76ecd794cee6d4b77145b40463` |
| TCP module | `fa951960e0ab31959bf3f82b2e8bb41469a84b717e3aa9641edea8b18b0fd3f9` |
| Ceph module | `4aeba063c5e1631895cad879d8315d3380050739e3be3d773abc1ec0db6a4cb7` |
| Accepted fault interface copy | `3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523` |
| Completed provider-plan copy | `011a7e1d4c6232f193d9bde63061f31d1710722c0ece1461e4221c4ceff64d13` |
| Additional local-main AsyncMessenger.cc | `47dff709e86e92676119504ee30774583e2f4d49155b84a89e22c74585918050` |
| Additional local-main AsyncMessenger.h | `7a3828732ec2e7dd4b1d5ce0f31a538280a3c4cb33264212125920e2a45a95f7` |

Exact archive membership, regular-file type, SHA256, sizes and actual0664 candidate modes match. All original frozen artifact entries match at startup and final audit. All extracted candidate/reference bytes and modes remain stable. The two extra C++ references and their original local files also remain stable. The review makes no mutable-primary-tree or Git-state attestation. Only this repository report and exclusive external review artifacts were written.

`review-report.md` retains the completed report in the exclusive artifact root. `review-artifacts-manifest.json` pins that copy, the extracted inputs, added reference copies, review scripts, audit results and execution logs by bytes/mode/SHA256. The manifest excludes only itself.

The original author70-test setup failure and77-test literal failures remain historical failures. The author's78-test claims remain separate from the two new independent78-test executions. No old evidence was removed or relabeled. No EDQUOT occurred.

## Protocol review coverage and limits

| Area | Independent assessment |
|---|---|
| TCP | Reviewed SYN/SYN-ACK/final ACK, bidirectional identities, tuple generations, wrap arithmetic, pending/delivered overlaps, retained-history rejection, FIN/RST and ACK/empty-segment tail checks. Independent reordered/wrapped frame bytes reconstruct exactly from original packet ranges. WCEP-2 blocks terminal-source correctness. |
| Admission and ownership | Shared budget covers pending data, span leases, history/temp copies, parser queues/scratch and retained metadata. Explicit stream/segment/source/count checks precede admitted copies. Pressure fails without eviction. Independent reduced-budget and ninth-stream inputs fail. Reconnect per-session bounds remain defective under WCEP-1. |
| msgr1 | Checked banner9/address136/connect33/reply26, challenge/retry/RESET/READY/SEQ, accepted feature intersection, separate epochs, header53/footer13or21, all control tags and CRC32C convention. Gaps/old sequences retain distinct labels. WCEP-3 blocks full negotiation-state correctness. |
| REQUEST | Independent positive v4–6 × head1–3, counted releases, binary path/path2, separate replay/open flags and version suffixes. GET/SETFILELOCK owner/pid/start/EOF/wait offsets match the pinned sources. |
| REPLY | Independent InodeStat1–9 with modern dentry/DirStat/LeaseStat traces; separate client/server feature bases; capless target, absent target, signed negative result and created-inode distinction. Supplied tests cover both LeaseStat versions and directory extras. Source review covers GETFILELOCK/GETVXATTR extras. |
| CAPS | Independent10–14 layouts and range scalars, IMPORT/EXPORT suffixes, xattr/middle mismatch, daemon14 errno and normal20.2 v12. Follow-up checks client-origin FLUSH and MDS-origin exact/covering ACK with changed cap metadata. FLUSHSNAP_ACK stays excluded. |
| SESSION | Independent0–7 layouts, separate SESSION/header sequence values, whitelisted salted UUID/decimal timeout and dropped metadata. Pinned metric/cap-auth layouts match. Feature basis promotion requires observed OPEN. |
| Reconnect/reclaim | Independent3/4/5 data layouts, mixed cap/realm chunks, EOF locks, final flag and complete legacy/ADDR2 IPv4/IPv6 reclaim union. Reclaim request/reply join uses one outstanding exchange, not header tid. WCEP-1 blocks multi-session reconnect tracking. |
| Malformed/privacy | Required unknown versions, segment mismatch, truncated fronts/control/footer and bad mandatory CRC fail. Tested failures prevent later successful parser finish. Unknown unrelated messages still receive framing/CRC/sequence checks. Sanitized records contain no input byte views, authorizers, signatures or tested file/xattr/fscrypt payload strings. No authentication or peer-processing assertion is produced. |

The supported transport profile is Ethernet-II or raw IPv4/TCP. IPv6 is supported inside address fields, not as the captured transport. VLAN, cooked capture containers, IP options/fragments, ECN/urgent/fast-open/simultaneous-open and missing establishment are explicit failures. IP/TCP checksums are not verified. The messenger profile is lossless client→MDS protocol32/msgr1. REQUEST requires negotiated FS_BTIME. Legacy request heads/reply traces and reconnect1/2 are unsupported. Optional inode metadata bodies and snap/inline/fscrypt/xattr payloads are bounded opaque fields, not semantic validations.

Default limits are eight active/128 total TCP streams,4MiB shared byte-buffer budget per direction,64KiB overlap history,4096 pending/source ranges,1MiB fronts/reconnect data,64MiB streamed middle/data/authorizer limits,16 negotiation rounds and100000 packets/records. Packet/record ceilings can rise to1000000. Identity tables have explicit finite bounds. The4MiB limit is a byte-buffer accounting limit, not a CPython RSS or caller-output bound. The declared64 reconnect chunks/4096 caps/realms must be corrected per WCEP-1.

This review validates only the frozen offline decoder scope and returns findings. Installed capture profiles/counters, clocks, identity/authentication and peer-processing evidence remain P3/sole-QA qualification. Lossless daemon/native joins and case qualification remain P4. No active core, P2 or P3 implementation/review output was inspected. No live collection, network traffic, cluster/process control, Git write, delegation, full Python suite or Rust test/build ran. No installed-provider readiness, bootstrap/durability result or native/full-G3 acceptance is claimed.
