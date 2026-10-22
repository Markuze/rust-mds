# sol-release

Task: You are a subagent spawned by another session.
You are the designated Sol release/git integration agent, user explicitly requested 'make sure to commit and push all changes'. Model Sol Ultrafast max. Parent orchestrates; all coding/reviews remain assigned to others. Work /mnt/workspace/rust-mds. Read AGENTS.md git etiquette and current/sol-orchestration.md. Sole writable file for this initial preparation is current/sol-release.md (create ledger first). Do not edit/stage/revert source or other ledgers. No commits or push YET: daemon runtime fixes currently active; final review/live gates pending. This initial task is substantive repository/remote readiness and atomic commit preparation, not polling other agents.

Inspect current branch, HEAD, clean/dirty/untracked ownership inventory, configured remotes/upstream, existing local commits vs origin, git user.name/email/signoff readiness, gitignored secret/build paths. User authored initial baseline was clean f54d298 (main ahead origin by1); preserve it and all subsequent work. Report possible unexpected user edits rather than deleting. Don't print secret credential values or unredacted auth URLs. No force push/reset/checkout/rebase/amend. No GitHub issue/PR creation unless requested. You may read remote state with git ls-remote/fetch (no ref/branch switching) if useful, never overwrite working files. Detect auth/upstream issues noninteractively where safe, no push probe.

Propose minimal atomic commit grouping consistent with actual dependency graph: platform APIs first if compatible; engine/store/daemon schema changes may need one cohesive commit; verification tooling and status docs separately if each commit stays buildable. User wants ALL repository changes included (tracked+intended untracked code/docs/ledgers), but never force-add gitignored keys/build artifacts/test outputs; inspect meaningful new files before including. Every commit must have subsystem subject, rationale body, and real configured Signed-off-by identity. Do not invent identity. Need final verified workspace tests/build/fmt/clippy status and QA evidence before release; record existing warnings distinctly. Parent will later authorize execution once reviewed integration is ready, at which point you will commit/push normally and verify local/remote HEAD.

Return exact branch/upstream, identity readiness (name/email acceptable as git authorship, no secret credentials), ahead/behind status, proposed commit grouping, blockers and ownership hazards. Avoid broad code review or rerunning tests now against active changes. No subagents.

Status: PAUSED. Platform `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`, functional `ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93` and QA `805a8b0f1c8388548f0c8adf869aa28c0e70b926` are normally committed/pushed. Actual remote main matched QA HEAD; ahead/behind 0/0. All 640 committed and working Cargo inputs retain accepted hashes. The final documentation checkpoint at branch HEAD contains the remaining 27 intended status/handovers/task-review ledgers. This text freezes before its commit; the final report alone records its actual SHA, final push/remote confirmation and clean-tree result. No new tests, builds, formatting, source/tooling edits, live actions or subagents ran. Publication does not resume progress.

Context:
- Read `AGENTS.md`, `HANDOVER.md`, and `current/sol-orchestration.md`.
- Preserve user baseline `f54d2988ebaeeb8311314fe6894ee030a80e955d` and all later work.
- At initial preparation, daemon runtime corrections remained active. The parent now reports reviewed frozen source. Parent owns authorization and release sequencing. QA alone controls the cluster.
- Inspect Git state and intended file ownership. Sanitize remote URLs before display. Do not expose credential values.
- Read dependency manifests and narrow API diffs to prepare atomic groups. Do not perform a broad code review.

Next: At this text freeze, only the release owner's final documentation commit/push and read-only local/remote/clean-tree verification remain. The final report records their actual outcome. After that report, all agents stop and wait for explicit user resume; no further file edit or automatic publication is pending. No implementation, tests, build, G2 retry, G3 or live action may resume. Preserve fs44/meta88/data89 and all acknowledged namespace/evidence.

Log:
- [2026-10-22 20:01 UTC] Created this ledger before any other edit. Release execution remains prohibited.
- [2026-10-22 20:04 UTC] Recorded Git state and all 113 changed paths. Read dependency manifests, narrow API diffs, new-file headers/interfaces and existing review/evidence records. No unassigned path, deleted tracked file, symlink or non-text changed file was found. Path ownership does not prove the author of each hunk.
- [2026-10-22 20:05 UTC] Completed noninteractive origin read check, ignore checks, limited secret-shape scan and atomic commit preparation. No tests ran. Only this ledger was written. Parent authorization and release gates remain pending.
- [2026-10-22 20:07 UTC] Cross-checked the exact ledger inventory against Git: all 113 paths match, with no omission or extra path. HEAD/upstream remain unchanged, ahead/behind remains 1/0, and the index remains empty. Ledger has no trailing whitespace. Preparation is complete; execution remains held.
- [2026-10-22 20:15 UTC] Parent authorized isolated prospective platform verification only. Source is frozen per parent. HEAD remains `f54d298`, upstream `origin/main`, ahead/behind 1/0, index empty. Index SHA256 is `b211ff1a85c5f1a402cab4c8e0584d54af843e1131eea3339c7c2129a0277efc`. Identity remains Alex Markuze <amarkuze@redhat.com>. Platform inventory remains exactly 8 modified and 5 untracked files. No source or Git-state mutation occurred.
- [2026-10-22 20:18 UTC] Created an exclusive archive-plus-overlay snapshot under `/tmp/opencode/sol-release-platform-20261022.chxm820v/`. Verified all 1,722 base blobs, then overlaid the exact 13 platform files. All manifests/lock stay at base. Started isolated all-target build, then focused tests. No cluster/capture credential environment was forwarded.
- [2026-10-22 20:20 UTC] Both isolated checks exited 0. Workspace all-target build took 95.388 s. Focused platform tests took 33.273 s. Harness reports 24 monc, 70 osdc and 1 osdc doctest passed, 0 failures/ignored. One monc capture-key body returned early with an explicit skip note. Only unchanged base daemon files emitted build warnings.
- [2026-10-22 20:21 UTC] Post-run checks verified candidate bytes/modes/path set unchanged, all 640 primary Cargo inputs unchanged, exact platform overlay match, index bytes unchanged/empty, baseline HEAD and identity preserved. No additional changed path appeared. Documentation/ledger content updates by assigned owners are expected and remain separate. Platform-first split needs no adjustment. Release remains held.
- [2026-10-22 20:23 UTC] Final immutability checks still pass. Cross-checked all 13 overlay hashes and four stable result-artifact hashes against this ledger. No trailing whitespace or new changed path. Wrote artifact checksums under the scratch directory. No test rerun, primary source edit or Git-state mutation. Returned the standalone boundary result; await parent authorization.
- [2026-10-22 20:26 UTC] Parent explicitly authorized the first 13-path platform commit and normal push, independent of unrelated native G2 outcome. Preflight verifies exact candidate bytes/modes, unchanged 640-input content map `d2f4417f7ee3b60349a6d2cd6b6731fc5149862408ded94579fa9dd632a43adc`, expected baseline HEAD/main/upstream, empty index and Alex Markuze identity. No staging yet. Other groups remain held.
- [2026-10-22 20:27 UTC] `cargo fmt --all --check` exited 0. Then `cargo fmt --all` exited 0 and changed no file among all 640 Cargo inputs. The content-map hash remains identical to the frozen QA source. Index remains empty. Noninteractive remote read confirms main remains `66718c1b353cefa91c03507eff2c1031448f970e`. No test rerun or source edit. Proceeding to the exact authorized staging boundary.
- [2026-10-22 20:28 UTC] Staged only the explicit 13 platform paths (8 modified, 5 added). Staged whitespace check passes. Limited secret-shape scan found no matches. Verified every indexed blob/mode/path against the complete 1,727-file isolated candidate: exact match. No lockfile, other source, QA file, document or ledger is staged. Commit is authorized; normal hooks remain enabled.
- [2026-10-22 20:30 UTC] Normal signed-off commit succeeded: `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`, `platform: add checked fencing and native pool enumeration`. Parent remains `f54d2988ebaeeb8311314fe6894ee030a80e955d`. Verified exact 13-path committed scope/content/modes, author/committer/sign-off, empty index and unchanged 640-input source map. Remaining 100 paths exactly match functional 53, QA 22 and docs/ledgers 25. No unrelated hook/source change. Normal push is next; branch is 2 ahead / 0 behind tracking origin.
- [2026-10-22 20:31 UTC] `git push --porcelain origin HEAD:refs/heads/main` exited 0: remote advanced normally from `66718c1` to `a3dcd6f`, including the unchanged baseline parent. Read-only ls-remote verifies actual remote main equals local/tracking HEAD `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`; ahead/behind 0/0. All 640 frozen Cargo inputs still match the pre-release source map exactly. Platform paths are clean, index empty, other 100 intended paths remain unstaged. No test rerun, hook-induced source change, force push, amend or history rewrite. First milestone is complete; all changes are not yet committed.
- [2026-10-22 21:03 UTC] Read the current pause instructions, accepted portable handover, independent handover review and orchestration state. Parent authorized all remaining reviewed checkpoint groups, not resumed implementation. Preflight verifies main/origin-main at `a3dcd6f`, ahead/behind 0/0, empty index and all 640 Cargo inputs identical to accepted release evidence. Three intended additions since the earlier inventory are `current/sol-handover.md`, `current/sol-handover-review.md` and `scripts/tests/test_mount_identity.py`; no unknown path appeared. Final group counts are 53/23/27. No tests/build/fmt/live work ran.
- [2026-10-22 21:06 UTC] Matched all 640 Cargo inputs and all 53 functional/normative paths to the accepted native-QA manifest. Matched all 23 QA paths to that manifest plus the six-file accepted unique-mount correction. Shared oracle remains unchanged. Refreshed exact 103-path inventory; limited secret scan found no matches and all changed paths are regular text files. Actual remote main is still `a3dcd6f`. Reuse recorded 524/0/2/build/fmt/default-clippy and scoped native evidence; strict baseline failures remain documented. Staged only functional 53; staged scope/content and diff check pass.
- [2026-10-22 21:08 UTC] Functional signed-off commit succeeded: `ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93`, `mds: persist root-shard state with guarded WAL and checkpoints`. Exact 53 paths; all committed blobs match accepted native-QA snapshot; parent is platform `a3dcd6f`. Normal commit/hooks changed none of all 1,787 repository file bytes/modes. Then staged exactly QA 23, including accepted unique-mount regression; staged blob/scope/diff checks pass. No docs/ledger is staged; no remote confirmation is claimed yet.
- [2026-10-22 21:10 UTC] QA signed-off commit is `805a8b0f1c8388548f0c8adf869aa28c0e70b926`, `qa: add guarded restart verification and CI lanes`, exact 23 paths on source parent `ed83ca6`. Normal commit and push/hooks changed no repository file bytes/modes. Normal source+QA push exited 0 (`a3dcd6f..805a8b0`); ls-remote confirmed actual main = local/tracking `805a8b0`, ahead/behind 0/0. Verified all 640 committed/working Cargo blobs and modes against accepted release evidence. Remaining intended scope is exactly the 27 docs/ledgers. Finalize authorized release-status text only, then freeze files before final docs publication; final confirmation belongs in the final report, not a later file edit.
- [2026-10-22 21:13 UTC] Final documentation scope is exactly 27 reviewed paths, including all 22 current task/review reports. Staged path/blob/whitespace checks pass; every intended repository change is accounted for, with no unstaged or non-ignored untracked residue. Release-status paragraphs alone were finalized in the six authorized files. This is the final ledger edit before documentation commit/push. The release agent's final report will supply observed final commit/remote/clean-tree results; no file edit follows final verification. No tests, builds, formatting, tooling/source change, live action or subagent ran during this final checkpoint.

## Final paused checkpoint authorization

Task: FINAL RELEASE AUTHORIZATION. User requested allchangescommit/push, then 'when agentsfinish pauseprogress andcreatehandover'. All assignmentsfinished; implementation/tests/livework PAUSED. Handover current/sol-handover.md independentlyACCEPTED current/sol-handover-review.md. Only your Git checkpoint+release-status documentation allowed now. Do NOT run newtests/builds/liveactions/source/toolingchanges/subagents. Follow AGENTS pause. Code/source640hashesunchanged sinceacceptedQA, normalfmtall alreadyran beforeplatformcommit withnochanges; preserveidenticalinputs and reuseverifiedevidence insteadofrerunning.

Publish ALL REMAINING intended repositorychanges in proposed atomic groups, with normal signed-offcommits/push using realconfiguredidentity:
1. cohesive engine/store/mds + Cargo.lock + normative docs (known53paths beforehandover updates), subject `mds: persist root-shard state with guarded WAL and checkpoints` or equivalent.
2. QA scripts/devcluster/tests/CI (updateexactlist toincludeacceptedunique-mount fix), subject `qa: add guarded restart verification and CI lanes`.
3. statusdocs, AGENTS,HANDOVER,current/sol-handover.md, ALL intended task/review ledgers (inclnewhandoverreview, orchestrationrelease, etc), subject `docs: record paused durability checkpoint and handover`.

Alreadypushedplatforma3dcd6f mustremainintact. Beforestage refreshinventoryandexplicitpathgroups; verifyindexempty,currentmain/upstream,640CargoinputhashesmatchnativeQA/release platform evidence, scriptbytesmatchlatestacceptedproof asrecorded. Source group identical to final524pass0fail2ignored/build/fmt/defaultclippy nativebootstrap/restart/smoke tree; strictbaselinefailsdocumented. Intermediateplatformtreealreadyverified, functionalgroup canreuseexactCargo-inputQAhashes; toolgrouphas83author/21independentfocusedproof. No completeG2PASS: oldtrialexit1 beforecompare/nonreuse, uniqueidentityfixacceptedbutunrun; G3nonemptyheld. Commitbodies/documentationmuststatecorrectqualification, notpretendfullG2/G3.

Stageexplicitreviewedpaths only; inspectstageddiff/pathlist andgitdiff--check. Neverforce-add .env,keys,buildoutput,/tmpartifacts or unknownuserfiles. If unexpectedsourcechange/index/remoteconflict stopreport; no reset/rebase/amend/forcepush/hooksbypass. Normalhookspermitted but if theymodifyoutsideacceptedpaths stopreport. No newcode changes forlintbaseline.

You may firstcommit/push source+QA andverifyremote, then finalize RELEASE-STATUS PARAGRAPHS ONLY in current/sol-release.md,current/sol-handover.md,current/sol-orchestration.md,HANDOVER.md,progress_ledger.md,AGENTS.md withactualknownsource/QAcommit hashes and pause/allchangescheckpointstate. This temporarydocownershipisexplicit; otherownersstopped. Do notchangegateevidence/resumesteps. No self-referential finaldocSHAneeded; identifyfinaldoccheckpointasbranchHEAD andsupplyactualSHAinfinaltoolreport. Keeptruthful aboutpush status: don'tclaimremoteconfirmationbeforeobserved. FinalizeledgersBEFOREfinaldoccommit; afterfinalnormalpush+remoteverification doNOTeditfilesagain. Ensurefinal`git status --short`clean forintendedrepochanges.

VerifylocalHEAD equalsoriginrefs/heads/main afterfinalpush,ahead0behind0. Userpausepersists; releasecompletiondoesnotresumeG2/G3. Returnallnewcommitshashes/subjects,finalHEAD/remotehash,pushstatus,workingtreestatus,anyexcludedfilesandreason,exacthandoverpath. Ifauth/nonFF/protectionfailure occursreportactionableblocker andpreservecommits; donotrewriteremote. Goal allintendedchangescommitted/pushed and noactivework, notanotherverificationcycle.

This authorization supersedes earlier remaining-group holds. It permits only
normal Git publication and release-status documentation. Existing tests/builds,
the verified no-op formatter run and accepted native observations are reused by
source hash. Complete G2 remains NOT PASSED; the accepted unique-mount correction
has no live retry; nonempty G3 gathers remain held. Nothing in publication
authorizes a new test, build, daemon start, mount action or implementation task.

### Final grouping and evidence identity

| Group | Exact scope | Count | Canonical content-map SHA256 before staging |
|---|---|---:|---|
| Functional | Existing `functional.paths`: engine/store/mds, Cargo.lock and the 3 normative design files | 53 | `7ff4b06ae587d15617d3b9323ec1c19180bcad85f8a17663eba34a7452afa232` |
| QA | Existing 22 QA paths plus `scripts/tests/test_mount_identity.py` | 23 | `cb720e3729889add154be13562605af92dd44712cc5b81df723bd3e2c01694c7` |
| Final documentation | Existing 25 docs/ledger paths plus `current/sol-handover.md` and `current/sol-handover-review.md` | 27 | Refresh only authorized release-status text before the final docs commit |

The 53 functional paths retain the exact initial path list; no new source is
added to that boundary. All 640 Cargo inputs match both the release manifest
and `/tmp/opencode/sol-integrated-g2-20261022-2014/tree-final.sha256.json`.
The unchanged canonical Cargo content-map SHA256 is
`d2f4417f7ee3b60349a6d2cd6b6731fc5149862408ded94579fa9dd632a43adc`.
The six later QA correction hashes match
`/tmp/opencode/sol-unique-mount-correction-20261022/changed-files.sha256.json`;
the other QA hashes match the integrated manifest. The accepted oracle is still
`d391fe4f1267a5765a33ab7b0bb27e1f8cb8e40024ebcb4a57815263e1f851f9`.

The recorded combined result is 524 passed, 0 failed, 2 ignored; build, fmt and
default clippy passed. Strict baseline lint failed. Native Ready/bootstrap,
clientless restart and same-filesystem smoke have scoped accepted evidence.
Complete G2 stopped before comparison/nonreuse; the unique-mount fix has 83
author and 21 independent focused tests per Python mode but no live retry.
These are reused results, not new execution during publication. G3 stays held.

### Confirmed source/QA publication and final documentation checkpoint

| Commit | Subject | Confirmed publication |
|---|---|---|
| `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22` | `platform: add checked fencing and native pool enumeration` | Earlier normal push; preserved baseline parent `f54d298` published |
| `ed83ca6dbb22d69918b6e2c4f2a63c836fa37c93` | `mds: persist root-shard state with guarded WAL and checkpoints` | Normal source+QA push confirmed at remote main `805a8b0` |
| `805a8b0f1c8388548f0c8adf869aa28c0e70b926` | `qa: add guarded restart verification and CI lanes` | Actual local/tracking/remote main equality observed, ahead/behind 0/0 |
| Branch HEAD documentation checkpoint | `docs: record paused durability checkpoint and handover` | Final report must record the actual SHA and remote confirmation after normal push; this frozen text does not claim an unobserved result |

The final 27-path documentation scope is `AGENTS.md`, `HANDOVER.md`,
`progress_ledger.md`, `design/build-brief.md`, `design/workplan.md`, and all 22
intended `current/sol-*.md` reports, including both accepted handover files.
Only release-status paragraphs changed in the six temporarily assigned files.
Gate evidence, preserved identities, normative contracts and resume steps stay
unchanged. All intended source, QA, documentation and ledger changes belong to
these four checkpoint commits; no unknown file is included.

Reuse the accepted 640-input source manifest, recorded 524/0/2 QA and accepted
unique-mount tooling proof. The verified pre-platform `cargo fmt --all` was a
no-op and those inputs remain identical; do not rerun it while paused. Commit
bodies state incomplete G2/G3 and the strict baseline lint failure. Publication
changes Git metadata, not tested source. No new executable verification ran.

Ignored `.env`, build targets, Python caches and local `.crush` data stay excluded
and untouched. External keyrings and `/tmp/opencode` evidence/scratch artifacts
are not repository additions. No intended non-ignored repository file is
excluded. No issue or PR is created. After the final normal push and read-only
verification, make no file edits and stop. The portable handover is
`current/sol-handover.md`; explicit user resume remains mandatory.

## Authorized first milestone task

Task: AUTHORIZE FIRST MILESTONE COMMIT AND PUSH now. User requested allchanges committed/pushed; platform scope has implementation+independent review+native probe/evidence+isolated intermediate-tree build/tests, so no need wait for unrelated G2 runtime live result. Commit ONLY exactly the13 platform paths verified in your isolated snapshot and ledger, with `platform: ...` subject/rationale and real configured Signed-off-by Alex Markuze. Existing baseline f54d298 must remain intact. Other groups stay unstaged for later milestones.

Before staging, confirm those13 current file bytes equal verified candidate hashes, branch main/upstream/currentHEAD expected, nounexpectedstagedchanges. Source code is frozen for QA so no source edits. Follow repo fmtbeforecommit rule safely: `cargo fmt --all --check` must succeed first, then `cargo fmt --all` should produce no changes; verify all Cargo source/manifests unchanged. If formatting fails or hooks alter unrelated files, stop/report, don't reformat another lane or bypass hooks. Don't rerun accepted tests without changed source; isolatedbuild/tests and nativeplatform evidence qualify firstcommit.

Stage explicit13paths only (tracked+intendednative listing/probe newfiles). Check staged diff/pathlist, no secrets or unintended files. Commit signed off normal git hooks; no amend/reset/force. Push main normally to origin (user authorized), including existing ahead baseline commit; if nonfastforward/auth/protection failure stop/report exact actionable blocker, no history rewrite. Verify localHEAD=remote refs/heads/main after successful push, record exact commit/hash/status. NoPR/issues. Other code/QA/docs/ledgers remain uncommitted now and must be committed in follow-up once qualified; don't mark allchanges done.

Only edit current/sol-release.md as ledger outside git operations. Parent owns orchestrationledger. No subagents/livecluster. QA may have before/afterHEAD change; source hashes must be identical, record that explicitly so evidence is correctly attributed. Return commit subject/hash,pushresult,remotehash,remaininggroupstatus.

The new authorization releases only the verified platform milestone. It
supersedes the earlier hold for that group only. Reuse accepted isolated build,
focused tests, review and native primitive evidence. Do not rerun tests or claim
that this commit qualifies G2/G3. This ledger stays uncommitted in the later
documentation group.

Formatting precondition passed on the full current workspace. The subsequent
format command was a verified no-op across all 640 Cargo inputs. Their canonical
content-map SHA256 is still
`d2f4417f7ee3b60349a6d2cd6b6731fc5149862408ded94579fa9dd632a43adc`.
All 13 platform paths retain their tested bytes and modes. The pre-push remote
read matched the original tracking tip. No accepted test was rerun.

### First milestone release result

| Item | Result |
|---|---|
| Subject | `platform: add checked fencing and native pool enumeration` |
| Commit | `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22` |
| Parent preserved | `f54d2988ebaeeb8311314fe6894ee030a80e955d` |
| Scope | Exactly the 13 verified platform paths; 8 modified, 5 added; 2,061 insertions, 52 deletions |
| Candidate match | Complete 1,727-file staged tree matched the isolated candidate; all 13 committed contents/modes matched after commit |
| Author and committer | `Alex Markuze <amarkuze@redhat.com>` |
| Verified trailer | `Signed-off-by: Alex Markuze <amarkuze@redhat.com>` |
| Formatting | `cargo fmt --all --check` exit 0; `cargo fmt --all` exit 0, verified no-op |
| Commit/hooks | Normal signed-off commit; no hook bypass and no unrelated source change |
| Push | `git push --porcelain origin HEAD:refs/heads/main`, exit 0; ordinary fast-forward `66718c1..a3dcd6f` |
| Verified local HEAD | `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22` |
| Verified actual remote main | `a3dcd6f559c178cee1bbfe0cebf6d8d8a3705a22`, read with `git ls-remote --exit-code origin refs/heads/main` |
| Tracking ref | `origin/main` equals the same commit; ahead/behind 0/0 |
| Index | Empty after commit and push |
| Remaining intended changes | 100 paths: functional 53, QA 22, docs/ledgers 25; all unstaged |

QA attribution: HEAD changed from `f54d298` to `a3dcd6f` through this authorized
commit. The working-tree source did not change. All 640 recorded Cargo inputs
retained exact bytes, Git modes and path membership before formatting, after
formatting, after commit/hooks and after push. Their canonical content-map SHA256
remains `d2f4417f7ee3b60349a6d2cd6b6731fc5149862408ded94579fa9dd632a43adc`.
The pre-release source manifest at
`/tmp/opencode/sol-release-platform-20261022.chxm820v/primary-cargo-inputs-before.json`
still describes the working-tree source. QA can attribute a before/after HEAD
change to publication metadata if its own source hashes match. Do not claim
source changes or a test rerun from this Git metadata change alone.

The platform paths are now clean. The remaining changes are 43 modified tracked
files and 57 intended untracked files. Their path set still matches the three
explicit remaining lists below. This release ledger remains untracked and
unstaged in the later documentation group, as authorized. No ignored key, build
artifact, scratch tool or test output was added. No PR or issue was created.
No native G2 or G3 result is implied. The parent still controls qualification
and release authorization for every remaining group. The all-changes request
remains open.

## Follow-up task: prospective boundary verification

Task: Proceed with NON-MUTATING prospective commit-boundary verification while QA runs combined checks/nativeG2. Source files are reviewed and frozen (daemon82tests final), no commits/staging/push authorization yet. Same release ledger ownership; scratch snapshots under /tmp/opencode allowed. No edits to primary source/index/HEAD, no cluster/subagents. User explicitly requires eventual commit+push.

Verify proposed first platform commit builds/tests on original base f54d298 without relying on unstaged engine/store/daemon changes. Create isolated source snapshot (git archive base plus exact intended tracked/untracked crates/monc and crates/osdc changes, no .env/secrets/buildartifacts). Unique CARGO_TARGET_DIR=target-sol-release-platform under original root or snapshot appropriate; avoid lock collision. Record exact candidate content hashes/paths and compatible manifest/lock boundary. Run workspace all-target compile/build and focused monc/osdc tests sufficient to prove intermediate tree compatibility; don't repeat full current-tree tests because QA owns final evidence. Preserve configured user/signoff and existing baseline ahead commit. If dependency changes require including another file in firstcommit, report precise minimal adjustment, don't edit code or stage prematurely.

Prepare explicit path grouping for remaining cohesive engine/store/mds, QA, docs/ledgers. Intended untracked files in current/** are reports to include per user's allchanges request; exclude temporary tests/keys, flag any uncertain file. Later current-tree source hashes from QA can cover functional group if identical; no pointless repeat of already accepted checks. Strictlint baseline failures documented, not blocker to reporting honest checks unless new diagnostics introduced.

Return platform intermediate-tree verification result and pending evidence required for release. Keep release held until parent authorizes after native G2 outcome (may choose reviewed milestone commits before later G3; no decision/action without parent). Avoid polling QA/daemon progress. This task is standalone readiness verification.

The later sections retain the initial preparation snapshot. New verification
evidence will appear below this follow-up task. Existing strict-lint failures
must remain visible. They do not block honest result reporting. Do not claim
new diagnostics are baseline without comparison.

### Isolated candidate identity

Artifacts: `/tmp/opencode/sol-release-platform-20261022.chxm820v/`.
The scripts `prepare_snapshot.py`, `run_checks.py` and `finalize_snapshot.py`
record the exact preparation, commands and immutability checks. They are scratch
tools, not intended repository additions.

- Base: `f54d2988ebaeeb8311314fe6894ee030a80e955d`.
- Archive SHA256: `42a44099e89747f65138acdfd0d3c6deaae3abe35d21768d1da92467b7acdaf3`.
- Verified all 1,722 archived files against their base Git blob IDs.
- Overlaid exactly 8 modified and 5 untracked platform files. The candidate has
  1,727 files. Every file outside that 13-path overlay retains its base bytes and
  Git mode. No current engine/store/daemon file or changed lockfile was copied.
- Source: `/tmp/opencode/sol-release-platform-20261022.chxm820v/source/`.
- Exclusive Cargo target:
  `/tmp/opencode/sol-release-platform-20261022.chxm820v/target-sol-release-platform/`.
  It did not exist before this run. It is outside the source snapshot.
- Candidate canonical content-map SHA256:
  `78c7cc2ebf881739ec226361be8450ff81341a2ac938733c19c9f517974b9d70`.
- Complete `candidate-content.json` SHA256:
  `37f4edb9b9171ddabfa5b35f1c24db04eed94a7665f6793ed7143208f66036bc`.
  This map records every candidate path, Git mode, size and content SHA256.
- `platform-overlay.json` SHA256:
  `299034e0c33c0cafc48806b707e8cabb486a53704662eb9ca787b04a9719174e`.
- The snapshot has no `.git`, `.env`, ignored build tree or copied keyring.
  The runner forwards no cluster/capture credential environment. It uses offline
  Cargo resolution and executes no live example. Optional capture-key tests can
  report skips; those skips must remain visible.

Exact overlay (all Git mode `100644`):

| Path | Content SHA256 |
|---|---|
| `crates/monc/src/messages.rs` | `b96b97a2a254a551ad632e2912cddd8b4bc29b633b6fa83f1d783e472869880c` |
| `crates/monc/src/monclient.rs` | `d9eb93742c104ac82e864a7544627ccd6ebcb086d905d8e91ed5708405fe80ca` |
| `crates/monc/tests/fake_mon.rs` | `a4a2a81b143937f9d733bd1bca09ebeb8b128d1637466264ce30ab23fef185d4` |
| `crates/monc/tests/version_wire.rs` | `5750a03c94218c575f68784a8393d5ac684f4a6ffa0ab3c8477258bbef994f4f` |
| `crates/osdc/examples/platform_probe/mod.rs` | `274becdacf5843b6340480739b39a85f009c3d757a6576fddaf4e9f46315ccbb` |
| `crates/osdc/examples/rados_live.rs` | `82c937de86ec00de3a1c30fedebb90830de0d04a5a1e60f8db43733d81cec488` |
| `crates/osdc/src/lib.rs` | `b709b399450463af6284fa5caf7654ed7c5b7d4a4899a0dae93015a4e88d5eb6` |
| `crates/osdc/src/listing.rs` | `dc410908cb23da9450684bc7a830c3a9ac3cc95c6fa76d42ead91596fd382a9f` |
| `crates/osdc/src/objecter.rs` | `3da997046d45e715893e657b82778c19f494caff0cbf392c3650951542586cfc` |
| `crates/osdc/src/ops.rs` | `391f8370d0e6ff0ceecbf0d5c92c8a22d2c9b64f8a6b1789aa65f9ccff098f39` |
| `crates/osdc/testdata/gen_pgnls_response.sh` | `e1839a11282514d04e202397015d5fba955796437038819680476a91cdddc4f8` |
| `crates/osdc/testdata/pgnls_response3.hex` | `4aa3868b1cbaa8dba54cbe76c7651f7233fe55bf71aeac742f8b57e2d48b7915` |
| `crates/osdc/tests/objecter_mock.rs` | `4d7dcafdf05006d451f751f40e7f6e121b3a4d1380445ba06e3f8ad43f2e16ad` |

### Manifest and lock boundary

All seven inspected workspace/package manifests and `Cargo.lock` remain exact
base files. `manifest-lock-boundary.json` records their hashes. Its SHA256 is
`a2d03391babb50e8aa72e4aaf5d0f965b5282a3e4deaf5fd38d8329f85958076`.

- Candidate `Cargo.lock` SHA256:
  `0d354fd93b9982f24a445f2e34b62be28e7293e252c5583279abb485ad7f2bff`.
- Candidate workspace `Cargo.toml` SHA256:
  `b994d2b1d62490dc3642268efa0d3096f9e2e89c555d4f42be454c3b1b2c0f06`.
- No dependency edge is added by the platform group. Retain current
  `rmds -> store` and `store -> osdc` manifest/lock changes in the later cohesive
  functional group. Do not move the changed current lockfile into this candidate.

### Authorized isolated checks

Run from the candidate source with `CARGO_BUILD_JOBS=4`, offline resolution,
the exclusive Cargo target above and a credential-free test environment:

```sh
cargo build --workspace --all-targets --locked --offline
cargo test -p monc -p osdc --locked --offline -- --nocapture
```

The runner wrote `workspace-build.log`, `platform-tests.log`, `toolchain.json`
and `check-results.json`. Both checks completed successfully on 2026-10-22:

| Check | UTC interval | Result |
|---|---|---|
| Workspace all-target build | 20:18:26-20:20:01 | Exit 0; 95.388 s; base engine/store/daemon and their test/example targets compile against the platform overlay |
| Focused monc/osdc tests | 20:20:01-20:20:34 | Exit 0; 33.273 s; harness totals: 24 monc + 70 osdc + 1 osdc doctest passed, 0 failed/ignored |

One of the 24 monc tests, `cpp_cephx_exchange_replays_through_our_client`, returned
before its key-dependent body because `RMDS_TEST_MDS_A_KEY` was absent. The log
prints the skip. The harness counts this early return as passed; do not claim the
decryption exchange was exercised. The remaining golden, wire, fake-monitor,
listing and Objecter tests ran. Tests named `live_*` use archived map fixtures;
they did not access the dev cluster.

Toolchain: rustc 1.99.0 (`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`),
cargo 1.99.0 (`5f94df478`). No current-tree test/build/fmt/clippy check ran here.
QA owns final combined evidence. No strict-lint success is claimed.

Build warnings were confined to unchanged base daemon files. Production emitted
the same six dead-code warnings in the retained baseline build log: `cluster`,
`fs_name`, `Closing`, `client`, `push_seq`, and grouped `iter_mut`/`len`.
The test target emitted five warnings, four duplicates; its remaining diagnostic
was `iter_mut`. The snapshot's complete non-platform blob verification proves
these files stayed at base. No monc/osdc warning appeared. Focused test output
had no compiler warnings. These are not the current durable daemon warnings.

Stable result artifact SHA256 values:

| Artifact | SHA256 |
|---|---|
| `workspace-build.log` | `43276a8ddb7d3879d47531d31ce07fe5977766e00b0360fa858e59b6cc5168ee` |
| `platform-tests.log` | `f3f1aedd6028be0c05d0986cff65af26ea1a59592a6d8571418707e902c39d7c` |
| `check-results.json` | `91c3ae09909f9e87a2842daad937385efac52ca8bd2af4fe7724827ea9b06edc` |
| `toolchain.json` | `920e89ef1bcb76b18075ca86fc73b0e8aaa8db59c81f7ca7ef840472b3f4150c` |

`verification-result.json` records the post-run invariants. `primary-before.json`
and `primary-after.json` record unchanged HEAD/index/identity. The primary Cargo
input manifests before/after are byte-identical, SHA256
`3ac8e274878d07fd306c8e438b6b78c4de8e073a17ae880acb97c43778d27783`.
The candidate has no changed, missing or extra path after build/tests. Its tested
platform content still matches the primary source exactly. No staged file or
new changed path appeared. Index SHA256 remains
`b211ff1a85c5f1a402cab4c8e0584d54af843e1131eea3339c7c2129a0277efc`.
The final 20:23 UTC `verification-result.json` SHA256 is
`4bfa4a84ee9552f83071d96594fa6ebdff3ad22c509f296919822acaec1575be`.
`artifact-sha256.json` records all preparation, command, manifest and result
artifact checksums. These scratch artifacts are not intended commit contents.

Expected content updates occurred in `AGENTS.md`, `HANDOVER.md`,
`current/sol-design.md`, `current/sol-orchestration.md`,
`current/sol-verification.md` and this release ledger. These are documentation
changes by assigned owners, not functional-source changes. Do not overwrite
them from the earlier grouping/hash snapshot.

### Explicit remaining groups and QA source matching

`prospective-groups.json` records all 113 intended paths and their observed
content hashes. Its snapshot SHA256 is
`7738a48a4ab93a113751f9fcb04e21c1dd221cced5fd971f28deb0126c6f36a6`.
The existing path inventory below names every member. The following exact
newline-delimited path lists provide an explicit future staging boundary. They
are proposals only; this task does not stage them.

| Future group | Count | Artifact path list | Path-list SHA256 |
|---|---:|---|---|
| Platform | 13 | `platform.paths` | `66b62c3bcd60d197f6c159684250b9384d64d1571ecfa08ca6f3937ab8d37014` |
| Cohesive engine/store/mds + lock + 3 normative docs | 53 | `functional.paths` | `ec6e7793608e0ed4480737f905511b8c80ca163667a015b55ee9680439a9da35` |
| QA tooling, tests, CI and procedures | 22 | `qa.paths` | `d2dd687df0b2fb3cbecc96124ca35c8af01ce68555faa9288db6031ddba7250a` |
| Remaining status docs and all 20 task/review ledgers | 25 | `docs-ledgers.paths` | `12081c9b06e6f2f89bf760ed46d912a6c93dc475c8cc11e0437d7df7920efbb9` |

All path-list files live in the artifact directory above. No uncertain temporary
test/key/output path appeared in the intended set. Include intended `current/**`
reports after owners finalize them. Scratch verification scripts, archives,
logs, snapshots, targets and manifests remain outside the repository.

For later QA comparison, `primary-cargo-inputs-before.json` records all 640
tracked/intended non-ignored `Cargo.toml`, `Cargo.lock` and `crates/**` inputs.
Its file SHA256 is
`3ac8e274878d07fd306c8e438b6b78c4de8e073a17ae880acb97c43778d27783`.
Its canonical content-map SHA256 is
`d2f4417f7ee3b60349a6d2cd6b6731fc5149862408ded94579fa9dd632a43adc`.
The 50 changed functional code/lock paths alone have canonical map SHA256
`ee4a43850cf9ebc51e051c1941eb79a169b30f0bc1d78d6f4a1f2bc593d9e45d`.
Compare individual relative-path content hashes with QA's final source manifest.
If they match, reuse QA's combined functional evidence. Do not repeat accepted
current-tree checks merely for the commit grouping. Status-document hashes can
change during QA and must be refreshed before eventual staging.

### Follow-up outcome and remaining release evidence

The 13-file platform-first commit boundary is verified. It needs no additional
file, dependency edge or lockfile change. The other three exact path groups
remain the proposed sequence. No uncertain file was found. This is prospective
verification only; no Git tree/commit was written or staged.

Still pending from the parent/QA, without polling them:

1. Final combined workspace tests/build/fmt/default and strict clippy reports,
   including skips and actual diagnostics, tied to the frozen source manifest.
   Existing strict-lint baseline failures are reported limits, not a reason to
   conceal successful checks. Investigate any new diagnostic separately.
2. QA's separately authorized native G2 outcome and its exact binary/source and
   filesystem/pool identities. Do not infer a native pass from this offline
   platform boundary. No G3 mounted-client qualification is claimed.
3. Updated canonical status/evidence documents. Refresh their hashes after owner
   edits. Preserve all intended `current/**` reports in the final docs group.
4. The parent's release/milestone decision after the native G2 outcome. The
   parent may choose reviewed milestone commits before later G3 work. This agent
   does not make that decision or require an unrequested G3 implementation.
5. Explicit staging/commit/push authorization. Then recheck source identity,
   remote state and intended path completeness. Preserve the existing baseline
   ahead commit. Use `Signed-off-by: Alex Markuze <amarkuze@redhat.com>` on every
   new atomic commit. Push normally and verify local/remote HEAD equality.

Release remains held. This task changed only this repository ledger and allowed
scratch artifacts. It did not alter primary source files, Git index/HEAD,
configured identity, remote configuration or cluster state.

## Git readiness snapshot

Observed on 2026-10-22 at 20:02-20:07 UTC. Other owners can still change the tree.

| Item | Verified value |
|---|---|
| Branch | `main` |
| HEAD | `f54d2988ebaeeb8311314fe6894ee030a80e955d` |
| Upstream | `origin/main`; remote `origin`, merge ref `refs/heads/main` |
| Local origin tracking tip | `66718c1b353cefa91c03507eff2c1031448f970e` |
| Actual remote main tip | `66718c1b353cefa91c03507eff2c1031448f970e`, confirmed by `git ls-remote --exit-code origin refs/heads/main`, exit 0 |
| Ahead / behind | 1 / 0, both against the tracking tip and the checked remote tip |
| Existing unpublished commit | `f54d2988ebaeeb8311314fe6894ee030a80e955d` — `Add handover notes for the next agent to the progress ledger` |
| Origin fetch and push destination | SSH, `github.com:Markuze/rust-mds.git`; same destination; userinfo omitted |
| Author / committer | `Alex Markuze <amarkuze@redhat.com>`; effective identities match configured values and baseline authorship |
| New commit sign-off | `Signed-off-by: Alex Markuze <amarkuze@redhat.com>`; use the configured identity, not an invented agent identity |
| Cryptographic signing | No `commit.gpgsign` or `user.signingkey` configured. A sign-off trailer is ready; it is not a cryptographic signature. |
| Push selection | No push override or explicit `push.default`; use explicit `origin HEAD:refs/heads/main` after authorization |
| Index | Empty of staged changes; no unmerged entries |
| Working tree | 51 modified tracked files; 62 non-ignored untracked files, including this ledger |
| Git operation state | No index lock, merge, cherry-pick, revert or rebase state observed |
| Hooks | No custom hooks path; no active default hooks |

The remote read used disabled terminal prompts, disabled askpass, SSH BatchMode,
strict existing-host-key checking and a finite connection timeout. No fetch,
push probe or credential/configuration change occurred. Read access does not
prove write permission or absence of branch protection. An ordinary authorized
push can still fail. Stop and report that failure; never force push.

Preserve the existing unpublished baseline commit unchanged. Its subject and
trailers are user history. Do not amend it to retrofit the new commit policy.
A later normal push must include it as an ancestor.

## Ownership inventory

`M` means modified tracked file. `?` means non-ignored untracked file. All paths
fall within the parent's assigned lanes. No unexpected path was found. Do not
discard any hunk based only on that path classification.

| Lane | M | ? | Ownership constraint |
|---|---:|---:|---|
| Platform | 8 | 5 | `crates/monc/**`, `crates/osdc/**`; accepted, idle |
| Engine | 18 | 6 | `crates/engine/**`; accepted, idle |
| Store | 3 | 8 | `crates/store/**`; accepted, idle |
| Daemon and lockfile | 8 | 7 | `crates/mds/**`, dependency-only `Cargo.lock`; runtime corrections active |
| Verification | 7 | 15 | `scripts/**`, `devcluster/**`, `.github/**`; QA owns all live actions |
| Design/status | 7 | 1 | Assigned design owner; some status summaries lag later accepted reviews |
| Task/review ledgers | 0 | 20 | Each owner retains its ledger; this agent writes only `current/sol-release.md` |
| Total | 51 | 62 | 113 paths; no deletion, changed symlink or changed binary found |

### Platform

```text
M crates/monc/src/messages.rs
M crates/monc/src/monclient.rs
M crates/monc/tests/fake_mon.rs
M crates/osdc/examples/rados_live.rs
M crates/osdc/src/lib.rs
M crates/osdc/src/objecter.rs
M crates/osdc/src/ops.rs
M crates/osdc/tests/objecter_mock.rs
? crates/monc/tests/version_wire.rs
? crates/osdc/examples/platform_probe/mod.rs
? crates/osdc/src/listing.rs
? crates/osdc/testdata/gen_pgnls_response.sh
? crates/osdc/testdata/pgnls_response3.hex
```

The new files contain wire tests, native pool listing, an opt-in QA probe and
its regenerable Ceph-dencoder hex fixture. They are intended source/fixtures,
not live output.

### Engine

```text
M crates/engine/src/caps.rs
M crates/engine/src/effects.rs
M crates/engine/src/engine/caps_eval.rs
M crates/engine/src/engine/client_caps.rs
M crates/engine/src/engine/mod.rs
M crates/engine/src/engine/ops_ns.rs
M crates/engine/src/engine/ops_read.rs
M crates/engine/src/engine/ops_xattr.rs
M crates/engine/src/engine/paths.rs
M crates/engine/src/engine/replay.rs
M crates/engine/src/inode.rs
M crates/engine/src/journal.rs
M crates/engine/src/lib.rs
M crates/engine/src/session.rs
M crates/engine/tests/caps.rs
M crates/engine/tests/common/mod.rs
M crates/engine/tests/journal.rs
M crates/engine/tests/stress.rs
? crates/engine/src/durability.rs
? crates/engine/src/engine/durable.rs
? crates/engine/src/engine/reconnect.rs
? crates/engine/src/engine/step.rs
? crates/engine/tests/durability.rs
? crates/engine/tests/review_regressions.rs
```

The new files contain the wire-free durable interface, checkpoint/replay and
one-record execution, reconnect adapters and regression tests.

### Store

```text
M crates/store/Cargo.toml
M crates/store/src/lib.rs
M crates/store/src/record.rs
? crates/store/README.md
? crates/store/src/allocator.rs
? crates/store/src/backend.rs
? crates/store/src/bootstrap.rs
? crates/store/src/framing.rs
? crates/store/src/sessions.rs
? crates/store/src/stable.rs
? crates/store/tests/checkpoint.rs
```

The new files contain native storage adapters, guarded bootstrap/WAL/checkpoint
and session/allocator protocols, integration requirements and schema tests.

### Daemon and lockfile

```text
M Cargo.lock
M crates/mds/Cargo.toml
M crates/mds/src/config.rs
M crates/mds/src/daemon.rs
M crates/mds/src/main.rs
M crates/mds/src/rados.rs
M crates/mds/src/server.rs
M crates/mds/src/session.rs
? crates/mds/README.md
? crates/mds/src/fencing.rs
? crates/mds/src/journal.rs
? crates/mds/src/persistence.rs
? crates/mds/src/server/runtime.rs
? crates/mds/src/server/runtime/tests.rs
? crates/mds/src/test_backend.rs
```

The new files contain fencing, the production journal/storage/runtime adapters,
runtime regressions, an offline compound-operation backend and operating limits.
Do not stage this lane while its owner still writes it. The current lockfile
diff adds only `rmds -> store` and `store -> osdc` dependency edges.

### Verification

```text
M devcluster/README.md
M devcluster/ganesha.sh
M devcluster/mount-fuse.sh
M devcluster/mount-kernel.sh
M devcluster/start.sh
M devcluster/umount.sh
M scripts/smoke.sh
? .github/workflows/verification.yml
? devcluster/mount_guard.py
? scripts/VERIFICATION.md
? scripts/ci-live.sh
? scripts/ci-offline.sh
? scripts/crash-history.py
? scripts/metadata-bench.py
? scripts/recovery-trial.sh
? scripts/recovery_trial.py
? scripts/restart_history.py
? scripts/tests/oracle_faults.py
? scripts/tests/test_history.py
? scripts/tests/test_recovery_gate.py
? scripts/tests/test_restart_history.py
? scripts/tests/test_safety.py
```

The new files contain CI, mount guards, history oracles, recovery controllers,
benchmarks, procedures and offline regressions. They are not generated results.
Keep the shared guards, callers and tests together.

### Design/status and ledgers

```text
M AGENTS.md
M HANDOVER.md
M design/build-brief.md
M design/minimum-feature-set.md
M design/store.md
M design/workplan.md
M progress_ledger.md
? design/persistence-contract.md
? current/sol-contract-review.md
? current/sol-daemon-review.md
? current/sol-daemon-store-review.md
? current/sol-daemon.md
? current/sol-design.md
? current/sol-engine-review.md
? current/sol-engine-state-review.md
? current/sol-engine.md
? current/sol-orchestration.md
? current/sol-platform-review.md
? current/sol-platform.md
? current/sol-rados-review.md
? current/sol-rados.md
? current/sol-recovery-review.md
? current/sol-release.md
? current/sol-store-state-review.md
? current/sol-store-wal-review.md
? current/sol-store.md
? current/sol-verification-review.md
? current/sol-verification.md
```

The new contract and all 20 ledgers are intended repository documents. Include
them after owners finalize them. Do not edit another owner's ledger. The
generated feature file and generated feature catalogs are unchanged.

## Secrets and artifact boundary

- `.gitignore` protects `.env`, `/target`, `/target-*`, `__pycache__/`, `*.pyc`
  and swap files. `git check-ignore --verbose` confirmed those protections.
- Existing ignored local paths include `.env`, 22 `target-*` directories,
  `target/`, `testdata/__pycache__/` and `.crush/` application data. The local
  `.crush/.gitignore` ignores its contents, including its database and logs.
  Leave all of them untouched and excluded.
- `.git/info/exclude` also protects local Claude state. Do not alter it.
- Cluster keyrings stay outside the repository in
  `/mnt/workspace/devcluster-data/keyring/`. QA results stay under
  `/tmp/opencode/`. Never copy either into tracked paths.
- The repository does not globally ignore arbitrary keyrings, private-key files,
  JSON logs or test-result directories. Broad `git add .` is unsafe. Use an
  explicit reviewed path list. Never use `git add -f`.
- No changed file was binary. A limited scan of all 113 changed paths found no
  private-key headers, common GitHub/AWS token shapes, literal Ceph key
  assignments or password-bearing HTTP URLs. The scan printed no matched
  values. It is not a complete secret audit.
- New-file classification inspected headers, interfaces, documentation headings
  and fixture shape. Final content inspection must use the frozen versions
  before staging. Do not treat this preparation as a broad code review.
- Tracked secret-like names were only `.env.example` and
  `crates/auth/src/keyring.rs`. Both are unchanged. No tracked keyring/private-key
  path was found by the filename scan.
- `devcluster/READY` and `devcluster/PACKAGES_READY` are existing tracked markers,
  not new output. They are unchanged. Do not remove them as cleanup.

## Provisional atomic commit sequence

This is a proposal, not authorization. Keep package tests, fixtures and READMEs
with their implementation. Every new commit must use the configured sign-off
above and a body that explains the problem, cause and chosen fix. Do not split
active files by historical review phase.

### 1. `platform: add checked fencing and native pool enumeration`

Paths: all 13 platform paths above.

Proposed rationale: Raw compound results and legacy map waits cannot establish
safe fencing. Bootstrap also needs bounded enumeration across application
namespaces. Add monitor version queries, checked compound results, explicit
bounded map waits and native PG listing. Retain legacy wrappers. Include the
wire/fault tests and opt-in probe with its fixture.

Dependency check: These crates import no engine/store/daemon API. Their manifests
are unchanged. `Objecter::wait_for_map` retains its old signature. Base daemon
Objecter configuration uses `..Default`. Base external `OpError` matches have
fallback arms. This is a plausible independent first commit, not a verified
build claim. Validate its isolated prospective tree before committing it.

### 2. `mds: persist root-shard state with guarded WAL and checkpoints`

Paths: all 24 engine, 11 store and 15 daemon/lockfile paths above, plus
`design/persistence-contract.md`, `design/store.md` and
`design/minimum-feature-set.md` (53 paths at this snapshot).

Proposed rationale: The prototype journal cannot preserve acknowledged metadata
across restart. Introduce fallible one-record admission and materialized recovery
state. Encode schema 2 records and guard stable WAL/checkpoint/session authority
with native RADOS operations. Wire bounded asynchronous storage, durable session
lifecycle and fenced lifecycle jobs into the daemon. Fail closed on uncertainty
or invalid recovery. Document the format decision and operating limits. State
the unqualified G3 boundary explicitly.

Keep these crates cohesive. `Journal::submit` now returns `Result`;
`closed_sessions` now stores `SessionKey`; retry/flush rows carry incarnations;
`RecordKind` and `Effect` gain lifecycle variants. The base store codec cannot
encode those types or construct the new record fields. The base daemon's effect
match does not handle the new lifecycle variant. Store consumes engine recovery
types and the new platform APIs. Daemon adds `store`; store adds `osdc`.
Separate engine/store/daemon commits would require consumer edits or fragile
hunk splitting. Keep both lockfile edges with their manifests.

### 3. `qa: add guarded restart verification and CI lanes`

Paths: all 22 verification paths above.

Proposed rationale: Smoke alone cannot prove durable restart or detect lost
acknowledged state. Add external fsynced histories, positive directory-enumeration
proof, owned-process/mount guards and same-filesystem restart checks. Keep live
execution opt-in and serialized. Report incomplete G3 coverage as partial, not
PASS. Add offline regressions, strict offline CI and the Ganesha recovery mode.

Keep the shared oracles, mount guard, callers, tests, CI and procedures together.
This adds no Rust dependency edge. Its strict CI intentionally exposes existing
lint failures; it is not currently a green CI claim.

### 4. `docs: record integration evidence and agent handover`

Paths: `AGENTS.md`, `HANDOVER.md`, `design/build-brief.md`,
`design/workplan.md`, `progress_ledger.md` and all 20 `current/sol-*.md` ledgers
listed above (25 paths at this snapshot).

Proposed rationale: Historical prototype results do not qualify the durable
integration. Record exact reviewed revisions, commands, results, warnings,
skips, artifacts and remaining limits. Preserve ownership and recovery rules.
Include the complete intended task/review state so another session can resume.

The assigned owners must refresh stale status first. For example,
`progress_ledger.md` still labels the complete-G2/partial-G3 harness review
pending, while the review ledger accepts it. Do not promote package evidence to
a complete gate. Finalize this release ledger before the last documentation
commit so later ledger edits do not leave an intended uncommitted file.

### Buildability rule

Initial preparation did not run an intermediate-tree build. The follow-up above
now verifies the platform boundary only. A test on the current
dirty workspace would include later groups and cannot prove a proposed earlier
commit. After authorization, validate each prospective Git tree in an isolated
source snapshot and an exclusive, never-shared Cargo target. Keep tracked and
intended untracked inputs in that snapshot. Record source identity and commands.
Do not switch branches or discard working files to construct the snapshot.
If the platform-only tree is not compatible, combine commits 1 and 2 instead of
committing a broken intermediate tree. Do not guess that the split passes.

## Verification status and release blockers

No compiler, formatter, linter, unit suite or live gate ran in the initial
preparation. The isolated follow-up results above are separate. The following
table retains the initial evidence snapshot and belongs to earlier, identified
revisions:

| Evidence | Recorded result | Release meaning |
|---|---|---|
| Isolated baseline `f54d298` | 347 tests pass, 0 fail, 2 ignored; daemon build and fmt pass; original kernel/FUSE smoke and native RADOS pass | Historical baseline only, not final integration verification |
| Baseline default / strict clippy | Default exit 0 with warnings; strict exit 101 | Existing failure, not clean lint |
| Engine author final package | 175 unit/integration tests plus 1 doctest pass; default clippy passes with 2 baseline warnings; strict fails | Author/package evidence; independent reviewers accept their scoped checks |
| Engine state independent re-review | 42 focused tests plus rejection/valid-transition probes pass | Scoped accepted package, not workspace/live proof |
| Store core independent re-review | 50 tests and store-only strict clippy pass | Accepted package; no native/live G2 proof |
| Platform independent review | 24 monc + 70 osdc tests + 1 doctest pass | Accepted package |
| Native platform QA probe | Exit 0; guard rejection/state preservation and complete all-namespace listing evidence accepted | Exercised primitives only, not bootstrap/WAL/restart qualification |
| Daemon persistence review revision | 71 tests and daemon-only default clippy pass with dead-code warnings | Storage scope accepted; runtime review requires four corrections; active revisions invalidate use as final result |
| Complete-G2/partial-G3 tooling | Author reports 67 focused tests normal/optimized; reviewer accepts 11 targeted tests normal/optimized | Tooling accepted, not a live recovery pass |

Baseline warnings are distinct from integration evidence:

- Baseline QA recorded 46 generated `denc` `identity_op` warnings, 2 engine
  warnings (`for_kv_map`, `collapsible_if`), 1 `monc` warning and daemon warnings.
- Engine author identifies its two remaining lints as unchanged baseline lints.
- The daemon storage reviewer recorded 6 production and 4 test-build dead-code
  warnings at its revision: `Config.cluster`, compatibility `spawn_purge`,
  `spawn_truncate`, `spawn_recover`, production-unused `Session::push_seq`, and
  `SessionMap::iter_mut`/`remove` (one grouped diagnostic).
- Final QA must report actual current diagnostics separately. Do not silently
  suppress warnings or edit generated files to claim strict lint success.

The named baseline logs under `/tmp/opencode/sol-g0/f54d298-*` exist, including
unit/build/fmt/default-clippy/strict-clippy/smoke/RADOS logs. The platform probe
log/result/command/source archive/manifests/cleanup verdict also exist under
`/tmp/opencode/sol-platform-live-20261022.YcfShl/`. The tooling red-regression log
exists under `/tmp/opencode/sol-g2-enumeration-coverage-20261022/`. This agent
checked file presence only. The independent review records provide the evidence
interpretation. Earlier contaminated `immutable-*` logs remain invalid.

Initial release checklist (historical). The follow-up outcome above updates the
runtime status, strict-lint reporting policy and parent-owned milestone decision:

1. Daemon runtime acceptance for attachment admission, CLOSE sequence recheck,
   pending-OPEN slots and Busy-input progress. Freeze all intended files.
2. Fresh final workspace tests and daemon build on the exact frozen source,
   including visible optional skips and peer/toolchain identity.
3. Final formatting and default/strict clippy results. Assign any source edits to
   their owners. Resolve strict lint or explicitly decide how its recorded
   baseline failure affects release. Do not call failing strict CI green.
4. QA-owned native bootstrap/store and same-filesystem G2 restart evidence for
   the reviewed binary. G3 remains incomplete; no release note may claim full
   mounted-client recovery. Only the parent can authorize the separate fresh
   Rust setup. Do not run reset/newfs during persistence gates or use fs43 as a
   fresh Rust filesystem.
5. Owner-updated canonical evidence/status documents. Include all intended
   untracked code, fixtures, procedures and ledgers. Preserve external evidence
   without committing raw keys, build artifacts or test outputs.
6. Explicit commit/push authorization. Then refresh remote/main noninteractively,
   inspect any new paths/hunks and stage only the final reviewed path list.
   Check each staged tree, its sign-off and its isolated build/test evidence.
   Recheck intended unstaged/untracked residue before the final normal push.
7. Verify final local HEAD equals `refs/heads/main` from `git ls-remote` after
   the normal push. Report exact hashes and any remaining intended changes.
   Preserve ignored local data. If remote main advances, stop for coordination;
   do not rebase, amend, reset or force push.
