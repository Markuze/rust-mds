# sol-g3-fault-controller

Task: Implement the accepted frozen G3 fault-v1 QA consumer offline. Own only the new g3_fault protocol/controller/client/peer modules, matching new tests, scripts/G3_FAULTS.md and this ledger. Preserve all published tooling, plans, contracts, native state, binary and evidence. No crates, existing helper edits, live actions, signals, mounts, daemon starts, cluster queries, Git writes, commits or subagents.
Status: AUTHOR COMPLETE / FROZEN FOR INDEPENDENT REVIEW of the offline package. Final focused68/68 and full234/234 tests pass normally/optimized, exit0, with no failures/errors/skips. The full suites contain all166 accepted baseline test identities. Ten source/doc paths stayed byte/mode stable across final verification. Complete native providers, a native connector/launcher and complete A4/B3/R/N assessment are not implemented. This is not full accepted v1 implementation, native FAULT_CASE_PASS or full G3 qualification.
Context: Primary tree /mnt/workspace/rust-mds. Accepted proposal SHA256 3cf2dcca8a081a82c4ac24793f82ab573a9d941f933a613a59ea96dd6a52d523, mode0664, remains unchanged. Daemon E1/D5 adoption and fault hooks remain dependencies. This task neither reviews nor adopts active E1 source. Native boundaries remain mocked under real-signal guards. Sole QA retains live ownership without live authorization.
Next: Parent assigns independent Astra review of the frozen owned files and retained artifacts. Review the implemented offline scope and the precise incomplete native-provider/case boundaries. Do not enable live execution, upgrade, promote E1, edit frozen files or publish without a new assignment. No subagent was spawned.
Log:
- [2026-10-25] Created the sole task ledger before source edits or tests. No accepted path changed.
- [2026-10-26] Resumed from the checkpoint. Read this ledger first and reread the frozen v1, consumer plan and producer report. Artifacts are under `/tmp/opencode/sol-g3-fault-controller-20261025.2_nqiu7r/`. Baseline log records 166 passed in 50.702 s, exit 0. The latest patch follows the 37-test normal focused pass. Existing helpers remain unchanged. Complete live WireCeph1/processing/continuity and NFS compound/owner providers remain unavailable; fixtures cannot establish native proof.
- [2026-10-26] Corrected the checkpoint's focused count: the retained `peer-progress.log`, `resume-progress.log` and `strict-progress.log` each record **36**, not 37, tests. The earlier “37” checkpoint sentence above is inaccurate. Subsequent intermediate passes progressed through the new worker/reader/restart/lifecycle cases to61. `worker-progress.log` retains an import-order failure; `pre-final-progress.log` retains66 tests with one test-helper argument error. Both corrections affect only new files. Do not relabel these failed intermediate runs.
- [2026-10-26] Completed real disposable worker attachment/rename/replacement/unlink/fsync receipts, two-round A0/B1/B2 modeled conversations, A1–A3 changed-target recovery, async-reader/fsync/overflow/EOF tests, complete typed snapshot/address proofs, eight live reservations,32-history bounds, lifecycle/reconnect receipt checks and explicit provider/case limits. Native API/CLI refusal remains. A4/B3 full assessment, R0 full qualification, N1–N4 complete joins/schedules and native startup/providers remain unimplemented dependencies, not accepted cases. No existing-helper edit was needed.
- [2026-10-26] Final focused normal/optimized runs each passed68, exit0. Final full normal passed234 in54.781s. The first optimized full attempt exceeded its120s tool limit and has no completed summary/PASS; its log remains retained. The unchanged-source optimized retry passed234 in208.036s with a600s limit. All completed runs have0 failures/0 errors/0 skips and real-signal guards. Both full logs contain every166 baseline test identity.
- [2026-10-26] Final preservation check passed: all14 startup-pinned byte/mode records,8 accepted G2 sources,7 published G3 source/ledger records, old binary330cf3ee and13 original+139 retry evidence hashes match. All10 owned source/doc records match before/after final verification. AST/runtime-assert/whitespace and read-only indexless diff checks pass. No shell file was added. The first static helper incorrectly expected indexless diff exit0; the corrected helper records expected new-file exit1 with empty diagnostics. No live query, native signal, mount, daemon start, packet capture, package install, Rust check, existing-helper edit, Git write/commit or subagent ran.

## Implemented files and public entry points

- `scripts/g3_fault_protocol.py`: fixed schemas/codec, `Framer`, typed `Snapshot`, fsynced rolling-prefix `Transcript`, `Channel` and independent `BufferedChannel`.
- `scripts/g3_fault_trial.py`: `Consumer`, explicit offline `Controller`, unchanged-helper `Ownership`/`CumulativeOracle` adapters, supported-case `OfflineSeries`, prerequisite-only CLI.
- `scripts/g3_fault_client.py`: raw-libc `Worker`, exact second-client root attachment, saved-descriptor checks and strict `load_receipts`.
- `scripts/g3_fault_peer.py`: provenance-bound offline `Evidence`/`Proof`, persisted worker-proof manifests, partial CAPS/tracefs/Ganesha-log/full-handle/PDML parsers and explicit capabilities.
- Five new `scripts/tests/test_g3_fault_*.py` files. The fixtures encode the producer independently of the consumer schema registry. They model native boundaries; disposable worker/oracle/fsync I/O is real.
- `scripts/G3_FAULTS.md`: exact API signatures, protocol/result scope, provider profiles and missing work. This ledger is the eleventh owned path.

All11 owned files have mode0664. The complete path/size/mode/SHA256 manifest is `candidate-files.sha256.json` in the artifact root. The four module hashes are:

| Module | SHA256 |
|---|---|
| `g3_fault_protocol.py` | `1f5db68df8cf56bd3a8d0325013c468dbb46f4d885d2dab04dc539673c23d3d9` |
| `g3_fault_trial.py` | `060a2fdcad6e2bad79baf23aa7d81188c764d89d1111ff888d36c3874c8b7ee3` |
| `g3_fault_client.py` | `7c3720d409f9171c77fa6156ef2ea2d4f04f2652ab33260e42d1a4f772a85cf2` |
| `g3_fault_peer.py` | `a823d5555f9bcdee2f391685b14e25d2c415a5669e86dfb3b2b0abdf70450b04` |

## Verification, exact commands and artifacts

Artifact root: `/tmp/opencode/sol-g3-fault-controller-20261025.2_nqiu7r/`.

Prefix every command below with `PYTHONDONTWRITEBYTECODE=1`. Add `PYTHONOPTIMIZE=1` only for optimized mode. Run from `/mnt/workspace/rust-mds`.

```sh
python3 /tmp/opencode/sol-g3-fault-controller-20261025.2_nqiu7r/run-tests.py focused
python3 /tmp/opencode/sol-g3-fault-controller-20261025.2_nqiu7r/run-tests.py full
```

The runner installs fail-fast guards before test discovery. It blocks `os.kill`, `os.killpg`, `signal.pidfd_send_signal` and `Popen.send_signal/terminate/kill`. No guard substitutes successful native actions. It writes a separate completed JSON summary for each mode/scope.

| Run | Log | Tests | Fail/error/skip | Exit | Seconds |
|---|---|---:|---|---:|---:|
| Pre-change accepted-tooling baseline | `baseline.log` | 166 | 0/0/0 | 0 | 50.702 |
| Final focused normal | `focused-normal.log` | 68 | 0/0/0 | 0 | 3.714 |
| Final focused optimized | `focused-optimized.log` | 68 | 0/0/0 | 0 | 3.725 |
| Final full normal | `full-normal.log` | 234 | 0/0/0 | 0 | 54.781 |
| Initial full optimized | `full-optimized.log` | incomplete | no completed summary | tool timeout120s | — |
| Final full optimized retry, unchanged source | `full-optimized-retry.log` | 234 | 0/0/0 | 0 | 208.036 |

Counts overlap. Do not add focused68 to full234. Intermediate failures and the incomplete optimized run remain separate evidence. `test-summary.json` records exact log hashes and baseline-identity inclusion. `static-checks.json` records AST, no runtime Assert nodes, whitespace and read-only `git diff --no-index --check /dev/null <owned-path>` results. Its expected exit1 denotes a new-file difference; empty diagnostics and the independent whitespace check establish the check result. There is no new shell syntax check or Cargo result.

Preservation artifacts are `preserved-before.json`, `preserved-after.json`, `owned-source-before-final.json` and `owned-source-after-final.json`. Final source/ledger archive: `offline-source.tar.gz`. Full freeze manifest: `candidate-files.sha256.json`. `HANDOFF.md` and `artifact-files.sha256.json` retain archive and artifact hashes. No keys or payload secrets belong in these artifacts.

## Qualification limits and next dependencies

The supported offline series is A0–A3/B1/B2. It validates modeled storage/transport/restart evidence plus real local receipts/oracles. Its only successful result is `OFFLINE_VALIDATED`, with `native_fault_case_pass=false` and `full_g3_qualified=false`. It rejects other case labels rather than relabeling these cases. Admitted failures create no successful result.

R0 reports a reconnect-purpose transport receipt only. N1–N4 peer matrices return `OFFLINE_PEER_PREDICATES`/77 with missing joins; daemon lifecycle validation is separate. These do not establish complete R/N case predicates. A4/B3 assessment remains held. Native API construction and `live`/`prerequisites` return prerequisite2 before queries/actions. Explicit `prepare` returns77/PARTIAL without native preparation. Published APPLICATION_PASS/PARTIAL behavior is unchanged.

Missing implementation is precise: complete bounded TCP/msgr1 modern-front/CRC/sequence decoding; retransmission versus Ceph-message instance identity; installed KernelPeer7/Libcephfs20/CallTrace identity, processing, loss/rotation/continuity and clock joins; complete NFSv4.1 compound/session/SEQUENCE/owner/stateid/handle joins; real Ganesha RESET/grace/lock schedules and outcomes; and a reviewed fresh-endpoint/SO_PEERCRED/pidfd/QA-lock native launcher. Partial raw parsers and fixture sources cannot fill these gaps. Later prerequisite probes must use the frozen names WireCephFields, KernelIdentityJoin, PeerProcessingJoin, CallTraceContinuity, Nfs41IdentityJoin, FhInodeJoin, GaneshaResetLog, GraceWindow, ObserverAccess and ObserverCapacity.

Preserved native state/evidence remains historical. The fresh retained-file check verified binary `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`,13 original and139 retry evidence files. It did not query the cluster or inspect either acknowledged namespace through a mount. Preserve fs44/meta88/data89 and both original/retry namespaces. No native gate, live provider readiness, E1 promotion, implemented daemon hook acceptance or full G3 claim follows from this package.
