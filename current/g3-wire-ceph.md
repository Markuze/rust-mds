# g3-wire-ceph
Task: Implement offline P1 bounded TCP/msgr1 framing and all required modern Ceph metadata fronts. Own only scripts/g3_wire_tcp.py, scripts/g3_wire_ceph.py, their two named tests, the g3-wire-ceph README/vectors fixtures, and this ledger. No collection, native capture, live actions, existing-helper edits, Git writes, or subagents.
Status: AUTHOR COMPLETE / FROZEN for independent review. Both internal milestones are complete. Final focused78 normal and78 optimized pass, without failures/errors/skips. AST, compilation, whitespace and61 reference/contract stability checks pass. No independent acceptance, installed-provider readiness or native G3 result is claimed.
Context: Standalone provider-local records avoid mutable core dependencies. Public source pins and downloaded credential-free v20.2.0 references are in /tmp/opencode/g3-wire-ceph-20261026-i7jxz0dh/. Preserve raw payloads only in bounded transient buffers. Persistent outputs contain sanitized metadata and source ranges only.
Next: Independent reviewer consumes the frozen seven-file candidate and complete manifests in /tmp/opencode/g3-wire-ceph-20261026-i7jxz0dh/frozen-r1-07_6u5it/HANDOFF.md. Do not edit this candidate after handoff. Parent assigns any corrections separately. P3 acquisition/qualification and P4 adapters remain separate dependencies.
## API decisions
- Keep TcpReader.feed(packet_meta, packet_bytes)/finish(barrier), and CephReader.feed(span)/finish(barrier). Constructors take explicit finite limits; CephReader takes a 32-byte run salt. No import-time I/O, subprocess, socket, capture, or external helper dependency.
- First link/IP profile: complete Ethernet-II IPv4/TCP frames, no VLAN/IP options/IP fragmentation, plus explicit raw IPv4/TCP. Metadata supplies immutable capture/boot/network-namespace/interface/packet identity, original packet offset/length and timestamp. Unsupported profiles fail explicitly. No parser interprets pcap containers.
- Eight active TCP streams. Four MiB aggregate buffered bytes per stream direction includes delivered validation history, pending reassembly, and parser-held bytes. A shared budget transfers ownership from TCP spans into the Ceph parser. Pressure is fatal; no stream eviction.
- WireCephFields errors identify unsupported relevant versions. Framing/continuity errors remain distinct. Forward message sequence gaps are metadata, not capture loss. Output records do not assert authentication from opaque skipped signatures/authorizers.
Log:
- [2026-10-26] Created ledger before implementation. P1 includes both framing and required fronts. Native readiness, P3 collection/continuity, P4 adapters and all live qualification stay separate.
- [2026-10-26] Pre-audit command `PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -m unittest test_g3_wire_tcp test_g3_wire_ceph` passed 43 tests. Earlier 36/40-test runs remain historical. Audit found parser temporary-copy accounting, tail-ACK continuity and reply negotiation gaps. Corrections and regressions are in progress. No live action, existing-helper import, Cargo command, Git write or subagent ran.
- [2026-10-26] Completed the audit corrections: shared admitted temporary/parser buffers, charged source references before dictionary construction, fixed-size front/reconnect storage, TCP ACK/empty-segment tail checks, terminal ACK/FIN/RST handling, original source-coordinate checks, partial/full retransmission receipts, exact profile/version checks, nonempty fragtree entries, aggregate nested counts, separate accepted client/server SESSION feature bases, retained passive client continuity, reconnect series and single reclaim exchanges. Runtime outputs contain only sanitized scalars/hashes/source ranges.
- [2026-10-26] Added seven independent literal fronts and one literal full53-header/13-footer frame. The focused run passed77 normally after correcting two literal-hex transcription errors. The earlier77-test run had two errors; the earlier70-test run had one failure because the probe compared a prior high-water peak against a later lowered limit. Those failed runs remain failed. The corrected probe reaches header53 and proves front allocation is not attempted on failed parser admission.
- [2026-10-26] Added the complete public API/profile/limit/reference README. Preserved the original pin map and created reference-pins-audited.json with25 local Ceph,24 tag and10 Rust source hashes. Tag common/frag.cc returned404; pinned tag include/frag.h contains the inline encoder. This is a source relocation, not an installed-package equivalence claim. Added the explicit negotiated FS_BTIME request-head guard and a lossy-profile rejection regression before final verification.
- [2026-10-26 03:07 UTC] Final focused verification passed78 tests normally in1.175s and78 optimized in1.181s. The explicit inventory is22 TCP +56 Ceph/static tests. Both modes report0 failures,0 errors and0 skips. The artifact runner loads exactly the two owned modules. Their31 signal/process/socket guards precede provider import. It records248 normal/137 optimized source/binary/cache input files; the different cache inventories are mode-specific, not test counts. No core/NFS/provider-owner test was discovered or imported.
- [2026-10-26 03:07 UTC] All six source/test/fixture SHA256/size/mode entries matched before/after final tests. All59 protocol source pins and both completed contract/plan inputs matched. Four Python ASTs contain0 runtime assert statements and compile at optimization0/1. Owned seven-file whitespace checks passed. The task ledger closes after those source checks; its final hash belongs to the seven-file candidate manifest. No global tree-stability claim is made.

## Frozen handoff

Root: `/tmp/opencode/g3-wire-ceph-20261026-i7jxz0dh/frozen-r1-07_6u5it/`.

- `HANDOFF.md`: review entry, archive/manifest hashes and exact scope.
- `candidate-manifest.json`, `candidate.tar.gz`, `candidate/`: exactly the seven assigned paths, including this final ledger. Actual source modes are0664; they were not changed for publication. No Git operation publishes them.
- `source-before-tests.json`, `source-after-tests.json`: six tested source/test/fixture hashes, sizes and modes. All six stayed stable. The ledger is deliberately outside this before/after claim.
- `normal.log`, `optimized.log`, `normal.json`, `optimized.json`, `run_focused.py`, `test-inventory.json`: exact focused results, runtime input/mode hashes and guarded runner. Tests use only the two named modules.
- `static-and-stability.json`: AST/compile/whitespace results and61 stable protocol/contract inputs.
- `input-manifest.json`, `reference-pins.json`, `references.tar.gz`, `references/`: complete source-format input hashes/modes and portable copies. These reference inputs are not imported at runtime. The original smaller pin map remains outside this frozen directory.
- `fixture-manifest.json`, `runtime-after-tests.json`, `archive-verification.json`, `artifact-manifest.json`: fixture/mode pins, runtime-file recheck, exact archive verification and final evidence inventory.

Exact final commands, with `R` expanded to the root above:

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 "$R/run_focused.py" "$R/normal.json"
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts:scripts/tests python3 -O "$R/run_focused.py" "$R/optimized.json"
```

The runner preserves the two-module unittest scope and adds runtime input manifests.
The ordinary `python3 [-O] -m unittest test_g3_wire_tcp test_g3_wire_ceph` commands
remain the direct replay API. Final results are not added to historical core or
application test baselines.

### Frozen candidate source pins

| Path | SHA256 |
|---|---|
| scripts/g3_wire_tcp.py | `fa951960e0ab31959bf3f82b2e8bb41469a84b717e3aa9641edea8b18b0fd3f9` |
| scripts/g3_wire_ceph.py | `4aeba063c5e1631895cad879d8315d3380050739e3be3d773abc1ec0db6a4cb7` |
| scripts/tests/test_g3_wire_tcp.py | `b1dfa46732ad9ef4b8aba4935bc3996b06a37ffc6e1169dd00c51e0523e06d83` |
| scripts/tests/test_g3_wire_ceph.py | `9195640cfae0f6006173af88e9b83b38a3fa9d7f605464968df4f76c11e0134b` |
| scripts/tests/fixtures/g3-wire-ceph/README.md | `d1855991722b7986a2447b9b9aa20aa73c924221ab3dab0dec3e961ac66355fa` |
| scripts/tests/fixtures/g3-wire-ceph/vectors.json | `08e2fdb8df9779dc53db3d24cf76852177baa29be6d8b739a25f60f4e9499d7c` |

Public API/profile/limits and all59 exact C++/Rust pins are in the fixture README.
The complete output carries original packet ranges, transport/message identities,
sanitized required fronts, distinct transport replay and message-gap labels, full
reclaim address unions and passive SESSION/reconnect/reclaim correlation. It does
not contain raw file/xattr/auth data or an authentication assertion.

Unsupported profiles fail explicitly: nonselected link/IP formats, missing/ambiguous
TCP establishment, unverifiable old overlap, loss/pressure, msgr2/lossy/failed
negotiation, legacy request/reply/reconnect formats and unsupported relevant
versions. Native start/drain/fsync/counter provenance, clock and peer-processing
proofs, qualified identity, installed equivalence, P4 projection and full G3 remain
outside P1. No raw pcap, key or auth-handshake artifact was read. No native capture,
collector, live action, Cargo command, existing-core edit, Git write or subagent ran.
