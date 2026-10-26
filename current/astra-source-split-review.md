# astra-source-split-review

Task: NEW narrow independent review of an INTERMEDIATE-ONLY #[cfg(test)] annotation and the exact two-commit source split.
Status: SCOPED ACCEPT the intermediate-only SessionMap::remove cfg(test) annotation and the exact two-commit source split. No blocker remains in this scope. All five callers are test-gated. External application reconstructs the exact approved 1802-file final tree. Independent retained-log accounting verifies524/0/2 and five warning-free checks. All pinned sources/artifacts, primary34/649, index/HEAD and the prior daemon report stayed unchanged.
Context: Review the frozen annotation-proposal-20261025.5wg74669 under /tmp/opencode/sol-release-source-series-20261025.jy8r5hmk. Base dd4cc57ac1b0c44b60cf33e3ec88bfa539b9c736. Keep the prior daemon review, failed intermediate and approved final candidate immutable. Preserve prior functional/lint acceptances. Write verification artifacts only under /tmp/opencode. No source edits, Git writes, live actions or subagents.
Next: Coordinator may renew publication authorization for this exact 6-path then32-path series, union34. The release owner must apply its publication guards. This review does not authorize staging, commit or push. Prior functional acceptances and the nonempty D5 hold retain their scopes.
Log:
- [2026-10-26] Completed independent review: SCOPED ACCEPT. Recounted42 source-matched harnesses:302unit/219integration/3doctest passes,0failed/2ignored,4optional early returns. All five retained command exits0; no diagnostics. Verified complete formatter no-op, all24 remove expressions, exact inverse annotation and both external patch applications. Before/after integrity records are byte-identical. No Cargo check was repeated.
- [2026-10-26] Before-integrity and exact series reconstruction pass in /tmp/opencode/astra-source-split-review-20261026.9GSHe74w/. All dd4cc57 archive blobs match Git object IDs/modes. Both patches pass check and application externally. Path sets are 6/32, union34, overlap4. Final full-mode manifest is 533f40694b0b4fce9acdca6581c248087d5dc7ae27c367a5c92952fa4a3984a1. The proof copy restores only rados.rs non-Git permission0664 to pinned0644 after byte/Git-mode equality; no candidate changes.
- [2026-10-26] Created the review ledger before any edit. Read AGENTS.md, HANDOVER.md and current/sol-orchestration.md. The final 612-test proof is retained; this narrow review uses existing check logs and independent artifact/tree verification.

## Decision

**SCOPED ACCEPT.** The one-line annotation correctly limits an unused,
binary-internal method to its existing test callers in commit1. Commit2 removes
that exact annotation when the accepted production callers arrive. No additional
functional or lint finding arose from this split review.

The nine accepted lint sites remain unchanged. Removing the new method hunk from
`lint-only.patch` reproduces the retained previous lint patch byte for byte.
The proposal differs from the failed intermediate only at `session.rs:583`:

```rust
    #[cfg(test)]
    pub fn remove(&mut self, client: u64) -> bool {
        self.sessions.remove(&client).is_some()
    }
```

The signature, body, comments, callers and tests are unchanged. There is no new
allowance, fabricated use, test, runtime logic or weakened check. The base has no
`Config.cluster` test reader. That accepted cleanup removes only the stored field
and initializer; the complete config test module remains byte-identical.

## Caller and accessibility proof

Paths and lines in this table refer to the proposal's `source/` directory.
I inspected the actual declarations, caller bodies and enclosing cfg scopes.

| Caller | Actual enclosing test gate |
| --- | --- |
| `crates/mds/src/server.rs:1177`, `Server<MemJournal>::kill_session` | `#[cfg(test)] impl Server<MemJournal>` at1169; `sessions: &mut SessionMap` at1173 |
| `crates/mds/src/daemon.rs:1944`, `terminal_output_failure_keeps_daemon_failed_after_runtime_removal` | `#[cfg(test)] mod tests` at1344; `#[tokio::test]` at1904; `Daemon::sessions: SessionMap` at320 |
| `crates/mds/src/session.rs:757`, `output_budget_failure_retains_push_sequence_and_releases_budget_on_remove` | `#[cfg(test)] mod tests` at616; `#[test]` at731; local SessionMap at733 |
| `crates/mds/src/session.rs:816`, `close_retains_funnel_until_older_output_and_close_are_transmitted` | Same test module; `#[test]` at787; local SessionMap at790 |
| `crates/mds/src/session.rs:899`, `terminal_send_failure_survives_removal_and_blocks_new_admission` | Same test module; `#[test]` at859; local SessionMap at880 |

All24 remove expressions were classified from source. Five call SessionMap.
The other19 call HashMap, BTreeMap, BTreeSet, VecDeque or Objecter. In particular,
the three `self.sessions.remove` expressions inside SessionMap operate on its
`HashMap<u64, Session>` field at265. No `::remove` reference exists in this crate.
`caller-audit.json` retains all24 locations and receiver declarations.

`crates/mds/Cargo.toml` declares only the `ceph-rmds` binary. No library target
exists. `main.rs:18` declares private `mod session`. Thus this public method is
internal to the binary. The retained production build also succeeds with the
method absent.

The final source's ungated `server -> runtime -> recovery` chain reaches
`impl Server<DaemonJournal>` at `server/runtime/recovery.rs:220`. Its production
`stage_recovery` calls `sessions.remove(client)` at536.
`recovery_connection_gone` calls it at737. Both parameters are `&mut SessionMap`.
This module is absent in the intermediate. The functional patch adds it and
removes exactly the method's new attribute. The final method at `session.rs:731`
has the same signature/body and is available in production.

## Independent series reconstruction

Base: `dd4cc57ac1b0c44b60cf33e3ec88bfa539b9c736`.

I verified all1793 archived base files against read-only `git ls-tree`: each
path, Git mode and computed Git blob ID matches. I then applied both patches in
a new external non-Git proof directory. Each patch passed
`git apply --check --whitespace=error-all` and actual application. All four
commands exited0. No index, object database or ref was written.

Commit1 has exactly these six paths:

```text
crates/engine/src/engine/caps_eval.rs
crates/engine/src/engine/mod.rs
crates/mds/src/config.rs
crates/mds/src/rados.rs
crates/mds/src/session.rs
crates/monc/src/monclient.rs
```

Commit2's exact32 paths match `provenance/functional-paths.txt`. The union is the
approved34-path overlay. The four overlaps are the two engine files, daemon
`config.rs` and `session.rs`. Independent `series-result.json` records both full
path sets. Intermediate and final prospective blob maps include every path,
Git mode, blob ID, byte length and SHA256.

The applied intermediate equals all1793 proposal entries. The applied final
equals all1802 approved entries, including bytes and complete filesystem modes.
Both independently serialized manifests match the pinned hashes below.

Git does not encode group-write permission. After commit1 application, only
`rados.rs` needed its proof-copy permission restored from0664 to pinned0644.
Its bytes and Git mode100644 already matched. This is recorded separately;
commit2 needed no such restoration. Neither candidate source was modified.

## Retained verification audit

I audited the raw logs, command records, runner and source guards. I derived
the expected harness set from the pinned Cargo manifests and test paths.
All42 harnesses match:9 unit,25 integration,8 doctest.

| Retained command | Verified result |
| --- | --- |
| `cargo test --workspace --locked -- --nocapture` | Exit0; **524 passed,0 failed,2 ignored**; no filtered tests |
| `cargo build -p rmds --locked` | Exit0; no warning/error diagnostics |
| `cargo fmt --all -- --check` | Exit0; empty output |
| `cargo fmt --all` | Exit0; complete1793-file byte/full-mode no-op |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit0; no warning/error diagnostics |

The raw summaries total302 unit +219 integration +3 doctest passes. Four
optional bodies return early: three missing capture-key bodies and the absent
kernel-v1-mdsv1 capture/corpus body. `dump` and `extract_corpus` stay ignored.
These limits remain attached to the524 result.

The five commands ran once between00:00:13 and00:03:30 UTC on2026-10-26.
The runner uses the proposal source and its unique
`target-sol-release-annotation`, offline Cargo, four build/test threads, isolated
HOME/TMPDIR and only PATH/LANG forwarded. No cluster/capture credentials,
LD_PRELOAD or stress-scale override was forwarded. Before/after manifests and
the per-command guards support the recorded complete formatter no-op.

The earlier intermediate remains failed: its strict Clippy exit101 names unused
`SessionMap::remove` at583:12; its build warned. Its1793-file map and28 frozen
artifacts remain unchanged. The final **612/0/2 and six-command acceptance** is
reused for the exact unchanged final tree. All56 final artifacts match. This
review executed no Cargo command and adds no new functional/native qualification.

## Exact pinned identities and review provenance

Proposal root:
`/tmp/opencode/sol-release-source-series-20261025.jy8r5hmk/annotation-proposal-20261025.5wg74669/`.

Final root:
`/tmp/opencode/sol-release-primary-corrected-20261025.2h7ofr85/`.

All hashes below were checked before and after this review.

| Artifact | SHA256 |
| --- | --- |
| Proposal `provenance/proposal-files.json` | `2012546c37570f203be8397d6440b676eb6d05ae748d3286f695063937c297ad` |
| Proposal `provenance/extra-annotation.diff` | `994f711a2d70ea07f5cc173a5d6a6143803336e7ae80e67d215385a388127ce7` |
| Proposal `provenance/lint-only.patch` | `5125f4764070f89ea77efc755652fb8bf0977daf3db416490e4bd7db05d1d198` |
| Proposal `provenance/functional.patch` | `8af03889538b300196f87f28cc197caf0f09e3a9e6c61d6e6e0eef0e701fb597` |
| Proposal `provenance/artifact-checksums.json`, all40 artifacts verified | `09d299509615865a88a9611a604ad82f7c8dc5325a51f49a07aab3b5ab5b99a9` |
| Final `provenance/reviewed-source-overlay.json` | `f8e955ff2d526492bc692d4d1cf9adac798cba5faed2bd26d53bdccb8646ef43` |
| Final and independently reconstructed complete manifest | `533f40694b0b4fce9acdca6581c248087d5dc7ae27c367a5c92952fa4a3984a1` |

Independent evidence:
`/tmp/opencode/astra-source-split-review-20261026.9GSHe74w/`.
Its28 scripts/proof records are bound by `artifacts.sha256.json`, SHA256
`04a11bbb7a7f0ca6748a900f5a80cbd78bf9cdb277b457eff0be5440d9508a45`.
The before/after integrity records are byte-identical, SHA256
`a9285a6879a5fca651e33ad69466a1fc10d7934d588ab7560f12d4f837eb736f`.

Primary34/649 source entries match before/after. Read-only local HEAD and
origin/main remain dd4cc57, ahead/behind0/0, with an empty index. Its raw SHA256
remains `e843538cb98fb83b5b5e4ff4202d7c3e017c0b0c04675c32af25750161880eef`.
Actual remote equality remains the release owner's retained observation; this
review made no remote query. The prior `current/sol-daemon-review.md` stays
byte/mode-identical at SHA256
`fcd8811698b54a5caaef776d1701c5ca21e73a3c586efa4a8b1dba0af7b7fe0c`.

Only this new ledger and independent external artifacts were written. Prior
acceptances, isolated E1 ownership, the nonempty D5 hold and live G3 boundaries
remain unchanged. Publication requires renewed coordinator authorization.
