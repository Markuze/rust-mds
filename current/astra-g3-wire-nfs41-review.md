# astra-g3-wire-nfs41-review

Task: NEW independent P2 WireNfs41 OFFLINE implementation review. Parent orchestrates.
Status: COMPLETE — CHANGES REQUIRED. Four P1 findings and two P2 findings block offline P2 acceptance. Exact 39 focused tests pass in each Python mode. Independent public probes expose nine assertion failures and two unexpected WireError results per mode. All reviewed input bytes and modes remain stable.
Context: Own only this new primary ledger and new exclusive external artifacts/review copy under /tmp/opencode. Review the frozen six-path P2 candidate and its pinned references. Never read or freeze active g3_fault_peer.py. Use only the injected function pin in the scratch copy. No code fixes, Git writes, live queries, capture, installed probes, mounts, native collection or subagents. Install signal/process-launch guards before provider import or test discovery.
Next: Parent can assign corrections for NFS41-1 through NFS41-6. Retain this review and all frozen evidence. A new frozen candidate needs independent review before offline P2 acceptance.
Log:
- [2026-10-26] Created the review ledger. No candidate edits or execution occurred.
- [2026-10-26] Created exclusive evidence root /tmp/opencode/astra-p2-nfs41-review-20261026.i2sp06gt. verify.py before passes: 37 pinned members, 38 ordinary archive files, six primary/scratch byte+mode pins, ten references and exact 39-test inventory. Injected only the frozen helper function into scratch. Both focused runs pass 39/0/0, with guards before discovery/import. Two initial review-runner imports failed before discovery/provider import because the socket guard broke ssl subclassing. bootstrap-failure.md retains that harness failure. The corrected guard uses an audit hook to block socket creation. Author history and EDQUOT evidence remain unchanged.
- [2026-10-26] Completed 24 independent public probes and eight new supplemental probes in each Python mode. Per mode: 21 pass, nine assertion failures, two unexpected WireError results. Counts across modes overlap. The two errors reject valid LOCKU/profile and parallel OPEN inputs; they are candidate behavior, not runner failures. diagnostics.json retains accepted malformed reports and the same-wire LOCKU tree contrast. reference-checks.json verifies 59 required field registrations with zero masks and five author test-input pins. Final verify.py after passes and matches before.json exactly except the stage label.
- [2026-10-26] Sealed 88 new review artifacts in REVIEW-MANIFEST.json, SHA256 e5bf5d98b33815e7381cd1d42762c2f7904f87293ef94e551f5f53234cafa337. A read-only recheck verified the exact inventory and every byte/mode pin. Final decision: four P1 findings and two P2 findings. No candidate correction or promotion occurred.

## Decision and scope

**CHANGES REQUIRED.** This review covers the frozen offline P2 decoder and reducer. It does not accept installed-provider readiness, native collection, P3/P4 integration, or a G3 case.

Evidence root: `/tmp/opencode/astra-p2-nfs41-review-20261026.i2sp06gt/`.

Candidate root: `/tmp/opencode/g3-wire-nfs41-20261026.ab31ryng/`.

The reviewer used the frozen source and ten pinned reference bodies. The reviewer used only the pinned helper function in the scratch dependency. The active core and P1/P3 outputs remained unread. The model catalog confirmed the Astra model and its `max` variant. No subagent was used.

All source locations below refer to frozen `scripts/g3_wire_nfs41.py`, SHA256 `3110731e75e7e9a9ad5e0bf7b69ecc71b7d6d12ba8ca3c500424acf4e2557204`.

## Ranked findings

### P1 NFS41-1 — Operation fields are not bound to their XDR positions and lengths

**Source:** lines 552–557, 580–597, 617–733; `_Fields` at 214–235.

The decoder checks that each selected field equals bytes somewhere inside the operation interval. It does not check that those bytes occupy that field's XDR position. It also does not validate each operation's opaque counts, padding, or exact consumed length. The fixed opcode/status/preamble checks do not close these gaps.

Independent public reproducers in `public_probes.py`:

- `test_operation_scalar_position_alias_is_rejected`: the actual OPEN share-access word is 1. The PDML share-access field points to the legacy-seqid word, whose value is 3. Both raw comparisons pass. The terminal report exports share-access 3.
- `test_nfs_opaque_length_mismatch_is_rejected`: PUTFH declares 28 opaque bytes. PDML selects 29 bytes, including one padding byte. The decoder accepts the selected bytes as a full 29-byte handle and emits inode/OPEN joins.
- `test_nonzero_nfs_opaque_padding_is_rejected`: the PUTFH padding contains `0x7f`. The decoder accepts it.
- `test_trailing_operation_bytes_are_rejected`: RECLAIM_COMPLETE(false) has four extra bytes, `JUNK`, after its boolean. The decoder accepts it and emits ReclaimComplete.

All four rejection assertions fail in both modes. `diagnostics.json` retains successful terminal reports for these malformed inputs. These are false source facts, not native PASS results.

**Required correction:** Validate the bounded XDR layout of each supported operation. Bind required PDML fields to exact positions and lengths. Check union arms, counts, padding, skipped payload lengths, and operation ends. Keep missing-required-field rejection; this correction does not require a missing-field fallback. Add malformed and valid nested fixtures for each layout family. Preserve the original status-position regression.

### P1 NFS41-2 — Deferred GETFH can contradict an established stateid target

**Source:** `_resolve` lines 899–916; GETFH lines 1029–1034; LOCK lines 1043–1065; inode edges lines 1081–1086.

`_resolve` compares handles only when both targets already have handles. It does not retain an equality constraint when one target is unresolved. A later GETFH can bind that unresolved target to a different inode.

`test_deferred_getfh_cannot_contradict_open_state_target` establishes OPEN state for inode 501. The next compound performs PUTFH(501), LOOKUP(other), LOCK with that OPEN stateid, then GETFH(502). The reader accepts the successful replies. It emits lock-owner/state/range facts and an OperationInode edge for inode 502, despite the OPEN state being bound to inode 501. The rejection assertion fails in both modes. The direct known-handle mismatch control passes.

**Required correction:** Preserve the stateid/target equality relation through unresolved FH transitions. Validate it when GETFH supplies a handle. A successful state operation must not produce conflicting OPEN/LOCK/inode joins. Keep unresolved targets pending when no exact binding exists. Add this late-binding contradiction and the valid same-target case.

### P1 NFS41-3 — Returned records can alter validated internal evidence

**Source:** `feed_pdml` line 436; `_emit` lines 764–766; `_call` lines 784–834; `_execute` lines 942–946; `_edge` lines 768–772; `finish` lines 1188–1189.

The public return tuples contain the same mutable dictionaries and nested objects that the reader uses internally. The original call identity remains bound to the RPC bytes, but callers can change the facts used for later joins.

`test_returned_call_mutation_cannot_change_validated_owner` feeds a valid OPEN call, changes the returned `operations[2].facts.owner`, then feeds the unchanged reply. The terminal report uses the changed owner. The expected raw-source owner hash is `a765c9e5835eb2329f8975d541700aa3bd0f0d24e290f1a05c792933b0fa3b6e`; the reader emits `e25e75fbc4af25559cc70181210439a7fc575012e868f0df8064445f1a6dee49`. The assertion fails in both modes.

The README's instruction not to mutate returned dictionaries does not enforce the requested source-evidence integrity boundary. The helper callback's returned FH dictionary is copied; the independent callback-mutation control passes. The defect is the public records/edges aliasing internal evidence.

**Required correction:** Return immutable records or detached deep snapshots from every public output path. Protect nested facts, source references, state records, and edges. Add mutation probes for feed results, returned join edges, and final reports.

### P1 NFS41-4 — A matching name overrides a contradictory worker handle

**Source:** `join_worker` lines 1115–1132.

The acceptance predicate uses `matching_handle OR matching_name`. It accepts an anchor that supplies both a correct name and a wrong full handle.

`test_worker_name_must_not_override_contradictory_handle` decodes name OPEN(target) and GETFH(501). Its WorkerAnchor supplies the correct salted name and the handle for inode 999. `join_worker` accepts the anchor and emits WorkerNfs with the handle for inode 501. The rejection assertion fails in both modes.

This is an internal contradiction in the supplied typed anchor. It does not require P2 to authenticate or read the external receipt.

**Required correction:** Require every supplied, comparable anchor field to agree. Require at least one exact supported join. Reject contradictory known fields. Retain the matched name/handle basis in the edge so downstream code can distinguish name evidence from handle evidence. Add matching-both, conflicting-both, name-only, and handle-only cases.

### P2 NFS41-5 — Standard Wireshark LOCKU fields are outside the searched subtree

**Source:** lines 585–589 and 707.

Pinned Wireshark `packet-nfs.c:10212–10219` adds LOCKU's legacy seqid to `tree`, the NFS protocol tree. It adds the other LOCKU fields to `newftree`, the opcode subtree. `dissect_nfs4_compound_call:10800–10815` supplies that NFS tree. The seqid is therefore a sibling of the operations-count field, not a descendant of it.

The candidate comment recognizes a sibling field, but the implementation searches only `_walk(containers[0])`. `test_standard_wireshark_locku_root_sibling_is_supported` supplies complete valid OPEN/LOCK/LOCKU bytes and the standard field placement. Both modes raise `Nfs41IdentityJoin: missing/ambiguous raw field nfs.seqid`.

`diagnostics.py` repeats this exact input, then changes only the XML parent of the LOCKU seqid. The original standard tree fails. The nonstandard opcode-child placement passes. RPC bytes and field positions/values stay identical. This is a pinned-profile implementation defect, not an installed-producer qualification failure.

**Required correction:** Include the exact supported NFS-root sibling at its LOCKU XDR position. Preserve operation isolation and reject wrong-position or cross-operation aliases. Add the actual Wireshark hierarchy as a positive fixture.

### P2 NFS41-6 — Stateid generation handling accepts invalid starts and rejects valid concurrency

**Source:** `_put_state` lines 918–934; immediate reply reduction at 876–881.

The reducer accepts any first seqid for a non-special returned stateid. It then requires every reply to arrive with exactly the next generation. These rules are insufficient for complete state history.

- `test_initial_zero_returned_stateid_is_rejected` and `test_initial_future_returned_stateid_is_rejected` use a complete establishment chain and a first serial OPEN returning seqid 0 or 9. Both malformed histories reach a terminal report. Both rejection assertions fail in both modes.
- `supplemental_probes.py::test_parallel_open_replies_can_arrive_in_reverse_generation_order` starts with generation 1. It sends two OPENs for the same owner/FH through separate slots and connections. The generation-3 response arrives before generation 2. This valid history raises `returned stateid generation did not advance` in both modes. Both calls and their matching replies are present in the input.
- Static boundary defect: line 927 advances `0xffffffff` to 0. RFC8881 §8.2.2, pinned lines 7746–7755, requires first generation 1 and wrap to 1. This wrap observation is a source/reference proof, not an executed multi-billion-operation trial. Slot sequence arithmetic is different and legitimately wraps to 0.

RFC8881 §9.9–9.10, pinned lines 9258–9317, requires OPEN upgrade increments and permits parallel OPENs. Wire arrival order does not establish server state-transition order.

**Required correction:** Track bounded generation observations and pending concurrent operations. Validate first-generation and predecessor completeness without treating response arrival order as state order. Apply nonzero stateid wrap arithmetic. Preserve rejection of unexplained gaps, future request stateids, wrong state kinds, and stale lifetimes. Add valid reversed-response and wrap-boundary fixtures with explicit expected behavior.

## Execution evidence

Working directory for all independent commands:

`/tmp/opencode/astra-p2-nfs41-review-20261026.i2sp06gt`

| Commands | Independent result |
|---|---|
| `/usr/bin/python3 -B verify.py before` | exit0; bundle/input/archive/reference verification; scratch copy created |
| `/usr/bin/python3 -B run_focused.py` | exit0; 39 passed, no failures/errors/skips |
| `/usr/bin/python3 -B -O run_focused.py` | exit0; 39 passed, no failures/errors/skips |
| `/usr/bin/python3 -B public_probes.py` | exit1; 24 probes: 14 passed, nine failures, one error |
| `/usr/bin/python3 -B -O public_probes.py` | exit1; same inventory and result |
| `/usr/bin/python3 -B supplemental_probes.py` | exit1; eight probes: seven passed, one error |
| `/usr/bin/python3 -B -O supplemental_probes.py` | exit1; same inventory and result |
| `/usr/bin/python3 -B diagnostics.py` | exit0; records observed acceptance/rejection for failed cases; not another passing test suite |
| `/usr/bin/python3 -B reference_checks.py` | exit0; 59 required field registrations found, all zero-mask; five author test-input pins match |
| `/usr/bin/python3 -B verify.py after` | exit0; complete before/after stability match |

The 24 public probes and eight supplemental probes are disjoint. Together they give **32 probes: 21 passed, nine failures, two errors per mode**. Normal and optimized counts overlap. The exact focused suite ran once successfully per mode. The two initial runner attempts failed before discovery/provider import; `bootstrap-failure.md` records that harness defect separately.

`focused-*.json`, `probes-*.json`, and `supplemental-*.json` retain exact inventories, commands, counts, and exits. Matching `.log` files retain tracebacks. `probe-inputs/` retains synthetic PDML and complete RPC source-byte/span JSON for the protocol cases. The builders and independent hash expectations are in `public_probes.py`; they do not import the candidate test module or its builders.

To reproduce one finding without overwriting retained logs:

```sh
/usr/bin/python3 -B probe_one.py public_probes test_operation_scalar_position_alias_is_rejected
/usr/bin/python3 -B probe_one.py public_probes test_standard_wireshark_locku_root_sibling_is_supported
/usr/bin/python3 -B probe_one.py supplemental_probes test_parallel_open_replies_can_arrive_in_reverse_generation_order
```

Add `-O` after `-B` for optimized Python. `probe_one.py` installs guards before import/discovery. It runs one retained probe and writes its result to the console.

## Other reviewed boundaries

- **XML/resources:** The lexical gate bounds chunks, tokens, attributes and declarations before Expat consumes each token. Node/depth/packet/operation/map/RPC budgets are explicit. Independent small limit+1, split-declaration, malformed-expert and sticky-error probes pass. The budget remains a decoder accounting bound, not an RSS measurement.
- **RPC/provenance:** Complete record markers, envelope identity, direction, source span coverage, and ordered reply prefixes have checks. The focused suite covers multiple RPCs/spanning packets, fragment offsets, retransmissions, conflicting xid bodies and ended-connection history. Independent fragment, replay-on-new-connection and error-prefix controls pass. Field-layout qualification remains blocked by NFS41-1.
- **Unsupported profiles:** Source checks reject RPC privacy/unsupported auth flavors, non-SP4_NONE protection, delegated OPEN replies/claims, RDMA binding/session flags, and unmodeled relevant operations. They skip declared credentials/verifiers and READ/WRITE data without exporting them. These checks do not cure the operation-layout aliases in NFS41-1.
- **Establishment/replay/lifecycle:** The observed EXCHANGE_ID → CREATE_SESSION → SEQUENCE chain supplies client identity. Legacy OPEN/LOCK clientids and owner seqids remain raw facts. The focused suite exercises session binding/destruction/recreation, slot bounds, and cached reply agreement. Independent identities match separately calculated salted hashes. Installed establishment and capture continuity remain external prerequisites.
- **FH/owners/state/ranges:** Full 29-byte handle validation agrees with pinned Ganesha `nfs_fh.h:45–78`, `nfs_filehandle_mgmt.c:124–164`, and `ceph-handle.c:2995–3019`. Independent foreign-export, known-FH mismatch and state-kind controls pass. Unresolved name OPEN remains unresolved across another PUTFH/GETFH. Owner namespaces, both LOCK unions, LOCKU/CLOSE and held-range code were reviewed. NFS41-2, NFS41-5 and NFS41-6 block complete acceptance.
- **Special stateids:** Zero request seqid correctly selects the current established generation. All-ones stateid on WRITE is valid anonymous semantics under RFC8881 §8.2.3; its independent control passes. It is not a read-only restriction. Current/saved state behavior remains covered by the focused suite.
- **Atomic failure:** A later invalid operation can follow earlier internal reducer changes. The independent probe confirms that the failure is sticky, uses exit1 after admission, and blocks finish and joins. It does not yield a successful terminal report. Mutable already-returned records remain the separate NFS41-3 defect.
- **Reclaim:** Independent client-wide RECLAIM_COMPLETE(false) has no fabricated owner/FH. A new-sequence COMPLETE_ALREADY without exact successful replay is rejected. GRACE, DENIED and NO_GRACE remain distinct; denied-owner/requester separation and exact replay are covered by the focused suite. P4 owns the complete reclaim gate.
- **Receipts/joins:** Positive matched worker/native controls pass. Wrong native inode is rejected. Worker consistency is blocked by NFS41-4. NativeAnchor retains PID/start/library, native key/incarnation, attachment, Ceph stream and receipt fields. P3/P4 must validate receipt contents, source scope/time, process/library lifetime, authenticated key/attachment, and ambiguity across reused inodes or multiple native observations. A syntactically valid receipt hash or positive ID is not that validation. Barrier supplies packet/drain/loss/receipt fields; external receipts must supply capture identity and clock/scope evidence.
- **Masked PDML fields:** The suspected masked-value incompatibility is not a finding. All 59 selected raw field names have zero-mask registrations in the pinned dissectors. A valid short masked child with full unmasked parent bytes passes the independent control. `print.c:559–639` supports this distinction.

## Frozen pins and stability

| Input | SHA256 |
|---|---|
| Candidate `final/MANIFEST.json` | `8fd40b432dd77ef2e0990091143d193d0e9f0122174320fb315f4a00f63fd081` |
| Candidate `wire-nfs41-offline.tar.gz` | `fcc30864086b885f4848cf39094b4fda590e5f5a4813fcd18b491a064b198f1c` |
| Provider | `3110731e75e7e9a9ad5e0bf7b69ecc71b7d6d12ba8ca3c500424acf4e2557204` |
| Focused tests | `955b408ec9841cb03a3d3a7574483213089ea3011d5c56f508a1d0ea584e54da` |
| Function-only helper | `67bac6edddcc1d41cc93b0ec1a63a684a08ef13ccc723ef8b7821205cf6ef12e` |
| Independent `REVIEW-MANIFEST.json` (88 artifacts; 17186 bytes) | `e5bf5d98b33815e7381cd1d42762c2f7904f87293ef94e551f5f53234cafa337` |
| Independent `before.json` | `11c0299c107bf3cb230bc2be62f27da87ddebe9056fa0e351d521a631ec43a97` |
| Independent `after.json` | `de1c6c555c0f097ee1dcc173bc7e9d3bd2d854ec63e3d29bc02bd0152e1ee41d` |
| Independent `public_probes.py` | `5fa3a655f3c7f95ce64674c62031bc510e85ca9476cb48bc052733a65d78f7ff` |
| Independent `supplemental_probes.py` | `4e1e732cb80a272a8bf742f106f99eb063179fa90ea6dedfbfae0f13031b1801` |
| Independent `diagnostics.json` | `37104660acbd2694778601f180a8010ce3ab1c751c1b337189590f5035e75200` |

`before.json` and `after.json` verify all 37 manifest members, the exact 38-file archive inventory, archive size 653727 bytes, all six primary paths and scratch copies, ten reference bodies/URLs, and the helper pin. Every archive entry is an ordinary file with matching bytes and mode. All six primary files remain mode 0664. No extra frozen-tree member or source change occurred. The author test-input/log provenance matches the final candidate.

The retained status-position failure stays failed. The separate earlier 38-test history stays historical. The EDQUOT write-check artifact and author freeze report remain unchanged. The one-shot freeze script was not rerun.

Only this new primary review ledger and new exclusive external review artifacts were written. Candidate code/tests/fixtures/ledger, static plan, accepted interface and prior reports remain unchanged. No Git write, live query, capture, mount, installed probe, native collection, or promotion occurred. `native` and `qualified` remain false. This decision grants no publication or live permission.
