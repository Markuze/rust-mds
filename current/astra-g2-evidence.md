# astra-g2-evidence

Task: "Review QA's new G2 application pass before implementation resumes."

Status: ACCEPT EVIDENCE for the 2026-10-25 complete accepted G2 clean-restart application workload. No blocking contradiction found. This permits parent-assigned G3 source implementation. Full G2 storage-fault qualification and G3 PASS remain open. Symlink persistence was not exercised.

Context: The only writable file is `current/astra-g2-evidence.md`. Evidence root: `/tmp/opencode/sol-g2-resume-20261025-1736/`. Review command exits, oracle coverage, corrected-controller provenance, preserved filesystem and old namespace, cold-mount identity, native recovery, and final cleanup. No source edits, tests, builds, Git writes, live calls, or subagents.

Next: Parent may assign G3 source implementation and independent review. Preserve fs44/meta88/data89, both acknowledged namespaces, the reviewed binary, and all evidence. Sole QA retains live ownership. This review does not authorize a G3 live trial.

Log:
- [2026-10-25] Created the review ledger before other edits.
- [2026-10-25] Read repository instructions, handover, accepted harness documentation, current QA ledger, and independent tooling acceptance. Read the accepted controller/oracle source directly from retained archives without extraction or execution.
- [2026-10-25] Rehashed 139 current evidence files and 13 original evidence files: no mismatch. Compared 640 archived Cargo inputs with both retry manifests: no mismatch. The six correction-archive files and accepted oracle match the seven-tool retry manifests. Wrapper, environment helper, and mount helper match accepted archived bytes and retry before/after tree manifests. Only the QA ledger changed in the recorded run snapshots.
- [2026-10-25] Confirmed controller exit 0, complete 17-operation/10-allocation history, four successful oracle subprocess records, 16 completed future allocations, changed boot-scoped unique mount identity, and native checked-load logs. Subprocess return codes are enforced by the accepted controller before it records successful output. No tests or live calls ran in this review.
- [2026-10-25] Identified a coverage limit: the accepted workload creates directories and regular files, tests hard links, and rejects unexpected symlinks. It does not create or recover symlinks. A symlink-persistence pass must not be inferred.
- [2026-10-25] Independently decoded retained final control fields, checkpoint frames and their CRC32C/linkage, and seven session rows. Confirmed Ready, owner5/incarnation354/gid16766, tail165/checkpoint15/covered165/trim165, empty WAL key list, and seven Closed sessions. All five original session rows retain their hashes.
- [2026-10-25] Independently recounted both 17-operation histories and the 16 future allocations. All new IDs are unique and disjoint from both prior allocation sets, including deleted/overwritten IDs. Native data-read hashes and the complete final data-object listing match the 20 distinct surviving files derived from the histories.
- [2026-10-25] Completed cleanup and identity review. Retained records show normal owned-client unmount before pidfd TERM of the selected MDS, then namespace-holder release. Final snapshots show no MDS/standby/FUSE/CephFS mounts, original mon/mgr/three OSD processes, and 17 active+clean PGs. Issued ACCEPT EVIDENCE within the scope below.

## Verdict and scope

**ACCEPT EVIDENCE.** The accepted G2 clean-restart application controller completed on the preserved filesystem. `controller-exit.txt` contains `0`. `g2-trial/result.json` records `PASS` with coverage `clean-restart-application`. The retained histories, checked subprocess path, native logs, identity records, and native state support this result.

This closes the missing application comparison/nonreuse prerequisite for G3 source implementation. It does not establish the full G2 storage-fault matrix or G3 PASS. The historical 2026-10-22 failed trial remains exit 1 with no pass artifact.

Review method: read retained artifacts and accepted source archives. Compute hashes, parse histories, and decode retained native bytes in memory. Do not execute the controller, oracle, daemon, tests, or builds. No live query, Git operation, subagent, or edit outside this report occurred.

## 1. Provenance

Evidence root: `/tmp/opencode/sol-g2-resume-20261025-1736/`.

- Rehashed all **139** files in its evidence manifest. All match.
- Rehashed all **13** files in the original integrated evidence manifest. All match. The original `g2-trial-isolated/error.json` still records `cold_remount`, recycled legacy ID323, and exit 1. Its `result.json` remains absent.
- Compared all **640** Cargo inputs in `accepted-source.tar` with the retry before/after manifests. All match. The recorded source HEAD is `a6ae05a10bfefdcea8dae75cd29597c84a3fadcd`.
- Compared all six files in `sol-unique-mount-correction-20261022/correction.tar.gz` with both retry manifests. Together with the unchanged accepted oracle, these establish the seven accepted-tool hashes. The wrapper, environment helper, common helper, and kernel mount helper also match the accepted archive and both run snapshots.
- The recorded whole-tree snapshots differ only at `current/sol-verification.md` during QA's run.
- Binary SHA256 is `330cf3ee8d934408fb270eaad21daf10d699506c5c248632071eea317bedb374`. Historical build hashes, startup ownership, trial metadata, and post-run provenance agree. The accepted controller also hashes the running restarted executable before accepting active.
- Config SHA256 is `e1ce5d1777850aca28c765719dbb97bc7d15d621bea1dcb0cc1e7f45a8d47837`. The original trial, retry, and post-run records agree.

Accepted controller hash: `c3123e78dcabaf599f4bd901eacd2c227c293dfdfaaf4bbbe75c0fc80d6d61f8`. Accepted mount guard hash: `9117ce280a216534f5cce9e4abf55567edce9123a5cf6ffad066aba0ec74926f`. Accepted oracle hash: `d391fe4f1267a5765a33ab7b0bb27e1f8cb8e40024ebcb4a57815263e1f851f9`.

## 2. Actual controller and oracle completion

QA's retained ledger gives the invocation inside the private mount namespace:

```sh
RMDS_RECOVERY_READY=yes bash scripts/recovery-trial.sh \
  g2 2662760 /mnt/workspace/devcluster-data/sol-g2-retry/mnt-kernel \
  /tmp/opencode/sol-g2-resume-20261025-1736/g2-trial \
  --owned-mount --rust-filesystem --expected-fscid 44 \
  --binary /mnt/workspace/rust-mds/target-sol-verification/debug/ceph-rmds \
  --keyring "$RMDS_KEYRING"
```

`g2-trial/oracle-output.jsonl` records four successful subprocess completions: **record -> verify -> allocate -> verify**. In the retained accepted controller, `command` raises on every nonzero subprocess exit before returning stdout (`recovery_trial.py:54-81`). `history` records successful output only after that check (`:396-410`). The final verify supplies `--future-history` (`:573-577`). Result creation follows all these steps (`:586-602`). Thus the evidence establishes zero exits for the four oracle calls, rather than relying only on printed success text. No `error.json` exists for this trial.

The oracle source performs real filesystem calls. It fsyncs external intent/ack records. It saves an ACK only after the operation and required file/directory fsyncs return. Its completion marker requires all 17 ordered operations and all ten initial allocation results. The retained history has all 36 expected records. The future history has all 34 expected records, including 16 ACKs and its completion marker.

### Compared application state

- Exact seven-entry saved namespace: root, `left`, `right`, `empty`, `left/keep`, `right/hard`, and `right/replace`.
- Positive enumeration of all four expected directories, including root and `empty`. Directory errors propagate; merely seeing a directory name does not satisfy coverage.
- Exact inode identity, type, mode, uid, and gid for every saved entry. Observed directory modes are `0700`; file modes are `0600`; uid/gid are 0.
- File size, SHA256, mtime, hard-link count, and complete `user.*` xattr sets. The main data is **65,811 bytes**. The retained second file is **8 bytes**. Both hashes match independently derived payloads.
- `right/hard` and `right/replace` retain inode `1099511627798`, link count 2, and equal data/metadata. The xattrs include an empty value and binary value `00 61 63 6b 6e 6f 77 6c 65 64 67 65 64 ff`.
- The source rename name, overwritten target identity, two deleted files, and deleted directory do not reappear. Exact namespace comparison checks their absence.
- **Symlink limit:** no symlink creation/readlink/restart-persistence operation exists in this accepted workload. The oracle rejects a symlink that replaces an expected entry. This is substitution detection, not positive symlink-persistence coverage.

Saved retry IDs are `1099511627793..1099511627802`. The absent/overwritten IDs are `1099511627797`, `1099511627799`, `1099511627801`, and `1099511627802`. Future IDs are **16 distinct values**, `1099511627803..1099511627818`. Independent history parsing confirms disjointness from every saved ID, including those four absent IDs. All values remain below `2^48`. Future ACKs include their expected bytes/metadata, and the final comparison covers all 23 old-plus-new entries.

## 3. Cold client and native recovery

Cluster fsid remains `3ceca2c6-851f-41cf-b538-1cfded4f1e82`; filesystem `a` remains **44**, metadata pool **88**, data pool **89**. Original and retry final filesystem records retain creation time `2026-10-22T20:17:48.295407+0000`. Trial before/after identities agree. No new filesystem is substituted.

The initial daemon is PID2662760/start138850848/gid16691. The controller's successful path requires one normal `sudo umount -- MOUNT`, an all-visible-namespace no-CephFS audit, and validated pidfd TERM before restart. `stop-intent.json` records signal **15**. The old daemon log records shutdown at 17:42:43 UTC.

The selected direct restart is PID2663998/start138858664/gid16766. `restart.log` shows native monitor authentication, connections to all three OSDs, checked persistent load with **zero restored Open sessions**, and replay354 -> reconnect355 -> rejoin356 -> active357. The accepted archived production path constructs `store::RadosBackend`, installs `Server::from_loaded`, and fails on checked-restore errors. The observed path is native clientless recovery. It is not an offline mock or an unchecked empty-state fallback.

The controller repeats the no-old-mount audit before cold remount. Its accepted helper call passes `noshare`, and the unchanged mount helper appends that option to the actual mount command. The recorded private namespace is `mnt:[4026533527]`. Both client identity files record boot UUID `cf0e3d48-7435-41f6-93ef-a8c06fd6d749`. Kernel UNIQ-ID changes **2147565794 -> 2147565795**. Legacy ID430 and device `0:60` recycle. The accepted unique-ID check handles this case correctly. The combination of normal unmount, no-old-mount audits, `noshare`, and changed unique identity establishes the independent cold client.

Evidence granularity: the controller does not retain separate raw stdout/status files for each successful unmount, intermediate mount audit, or cold mount. Their successful execution follows from the hash-matched fail-closed controller path. The new client identity, new native session, and complete comparison corroborate that path. This is sufficient for the accepted harness scope.

## 4. Original namespace and retained native state

The original namespace is **`restart-history-1629333-1792700434474727536`**. The retry uses **`restart-history-2663893-1792950162527640133`**. The original history remains hash-identical. Its full before/after verification logs report success. Both names occur in the retained materialized checkpoint. Its original ten inode identities are `1099511627776..1099511627785`; all retry and future IDs are disjoint from them.

Independent decoding of `native-final/` confirms:

- All five persisted object identities match fsid/fs44/meta88.
- Format phase **Ready**; owner **epoch5/rank0/incarnation354/gid16766**.
- Tail **165**; checkpoint generation **15**, covered sequence **165**; trim **165**; empty WAL key list.
- The manifest and state frames have valid magic, version, kind, length, identity, owner, sequence, and CRC32C. The descriptor checksum matches the manifest. The manifest's state name, length, and checksum match the retained state object.
- **Seven Closed sessions**, with cleanup receipts `[2, 31, 36, 73, 79, 112, 165]`. The five original rows match their preflight hashes. The two new rows correspond to clients16747 and16791.
- QA's checked native reads report **20** distinct surviving acknowledged data objects. Their sizes and hashes match values independently derived from the original, retry, and future manifests. The complete final data-pool listing contains exactly those 20 objects. Raw file-data payloads are not retained separately; the cold oracle and checked native-read results provide the data evidence.

The preflight and pretrial records also agree with preserved-state progression: owner3/tail83/checkpoint7/coverage82/trim82 -> owner4/tail83/checkpoint8/coverage83/trim83. These positions corroborate native load/publication/trim. They do not establish an exact replay-record count or crash-boundary fault qualification.

## 5. Cleanup and remaining limits

`client-cleanup.json` matches the cold client's complete boot-scoped identity. It records normal unmount and an empty all-visible-namespace CephFS audit at 17:44:10 UTC. `daemon-cleanup-intent.json` and `daemon-cleanup.json` bind TERM to PID2663998/start138858664/gid16766 and its listener/config/binary identity. The daemon stopped at 17:44:48 UTC. `namespace-cleanup.json` releases the owned holder only after client and MDS cleanup.

One initial postgate operator wrapper failed on a relative Python import before mount probes or mutations. `cleanup-notes.json` records the correction to absolute existing paths. This did not rerun or change the accepted gate.

Final retained process/map/mount snapshots show no MDS, standby, FUSE client, or CephFS mount. Original mon2312786, mgr2312859, and OSDs2312891/2312954/2313017 retain their start identities. All three OSDs are up/in. All **17 PGs are active+clean**. The system Ganesha process matches this run's preflight; it is not the historical Oct22 PID.

Still unqualified: positive symlink persistence, broader metadata/ACL cases, storage fault/uncertain-outcome matrices, checkpoint/trim crash boundaries, host/OSD power loss, nonempty reconnect, mounted-client takeover, locks, dirty caps, FLUSH_ACK, unlink-open, precise reply-loss/same-tid retries, and Ganesha reclaim. This trial used clean client close and MDS TERM. Its result explicitly reports `same_wire_tid_retry_proven=false`.

**Unlocked:** parent-assigned G3 source implementation and independent review. **Not established:** full G2 storage-fault qualification or G3 PASS.
