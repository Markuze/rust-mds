# glm-daemon-e1-d5-runtime-addendum

Task: NEW narrow provenance addendum to the completed SCOPED ACCEPT runtime review (`current/glm-daemon-e1-d5-runtime-review.md`). Resolve two report ambiguities by read-only local file/map/archive verification ONLY — zero test/build/formatter/import/provider/native/Git executions: (1) count the supplemental map/archive relation exactly — expect 667 unchanged shared entries + 1 modified shared entry (sole `mod supplemental_epoch;` addition in `crates/mds/src/server/runtime/tests.rs`) + 1 new test file = 669 supplemental files, and pin tests.rs before/after, the added file and the capture; state which original report/handoff statements that correction supersedes without editing them. (2) Identify the one absent external /tmp reference snapshot among the 16 recorded snapshots: exact source path, expected authoritative hash/size/mode, retained file path/hash, the original manifest that binds it, and whether retained BYTES independently verify or only metadata survives; distinguish the intentionally-skipped mutable coordinator reference from this absent immutable source; report a concrete blocker if equivalent retained bytes cannot be verified. Preserve strict 2848/2872 FAILED vs 24 origin-routed wording; no test counts rerun; candidate map 7a901827…/archive e65c7b5e…/receipt bb665008… and the original seal b86be4a453e03343fbca2f6afbf1583fd4196973ad3bab1912ced1d378b45007 (handoff e9218c9e…, aggregate 10093d38…) must remain stable.

Status: COMPLETE — SCOPED provenance clarification. Both assigned ambiguities are resolved by fresh read-only verification (zero tests/builds/reruns; original report, handoff and sealed root untouched and reverified byte/mode-identical): (1) supplemental relation is exactly 667 unchanged shared entries + 1 modified shared entry (`crates/mds/src/server/runtime/tests.rs`, sole added line `mod supplemental_epoch;`) + 1 new test file = 669; (2) the "absent /tmp reference" was a script-gate misclassification — the path is a symlink whose readable target and the retained copy both byte-verify; the one intentionally-skipped entry is the mutable coordinator integration ledger, named with scope. No blocker remains. Details and exact superseded quotes below.

Context: Addendum root `/mnt/workspace/glm-daemon-e1-d5-runtime-addendum-OKoKszZq` (fresh, exclusive; prior /tmp/opencode quota exhaustion recorded, stable /mnt fallback kept per earlier authorization). Original review root `/mnt/workspace/glm-daemon-d5-runtime-review-r40vN6LT` reverified untouched: seal `artifacts.sha256.json` sha256 `b86be4a453e03343fbca2f6afbf1583fd4196973ad3bab1912ced1d378b45007` still hashes itself; all 2,020 sealed files re-verified byte/mode; aggregate canonical `10093d38e67739b3352abfff98d9600ed54ad9a543896fe9fb70c84642100157` unchanged; REVIEW-HANDOFF.md `e9218c9e…` unchanged. Frozen candidate references reverified read-only: map `7a901827…` over 667 current primary inputs, archive `e65c7b5e…`, receipt `bb665008…`, capture `84f149cb…`. Fresh verification lives in `provenance-verification.json` + `verify_provenance.py`; no pinned artifact was rewritten; my verifier had five dry-run corrections (comparison cardinality and symlink handling) before its final PASS.

Next: Coordinator intake. Original report/handoff/seal stay frozen; startup/storage lane remains its own review. No live upgrade authorized.

Log:
- [2026-10-26 07:11] Ledger created first per parent instruction, before any verification work. No primary/candidate/source edits; no executions yet.
- [2026-10-26 07:24] Read authority `current/sol-e1-r4-integration.md`, sealed `verify_pins.py`/`pin-verification.json`/`verify_supplemental.py`, author `prepare-freeze.py`/`SEAL.json`, `reference-snapshots.json` and the artifacts inventory. Recovered the equality-loop exclusion (`modified = {tests.rs}`; loop over 667) and the 16-entry reference manifest's binder (the 494-member evidence bundle with SEAL-pinned member manifest `4eff891b…`).
- [2026-10-26 07:25] Fresh read-only verification run PASS in the addendum root (`verify_provenance.py` → `provenance-verification.json`): (A) original sealed root intact — 2,020 files byte/mode-identical, aggregate `10093d38…` and seal `b86be4a4…` unchanged, handoff `e9218c9e…` unchanged; (B) supplemental relation 667/1/1=669 verified archive-vs-archive and against both sealed extractions, tests.rs sole delta line, added file exactly one test, capture identical in both archives and primary; (C) 16/16 retained reference copies re-verified; 15/16 sources byte-verified live (including the three /tmp sol-daemon-r1 baseline files), the shared-E1 entry resolved as a verified SYMLINK, zero absent sources now; (D) primary 667 inputs == candidate map with capture pin. Report-before hash `5b59c431…` recorded. Verifier had five dry-run corrections of my own comparison code before the final PASS; no external state touched.

---

# Report: provenance addendum — supplemental equality and the snapshot classified absent

## Verdict

**SCOPED provenance clarification.** Both ambiguities are resolved by read-only local verification (`verify_provenance.py` → `provenance-verification.json` in `/mnt/workspace/glm-daemon-e1-d5-runtime-addendum-OKoKszZq`). No test/build/formatter/import/provider/native/Git execution was made; no test count is rerun. The original report, REVIEW-HANDOFF.md and the whole original sealed root are byte/mode-identical (receipt in section 1). No blocker remains.

## 1. Preservation and stability receipts (fresh, read-only)

- Original sealed root re-verified untouched: `artifacts.sha256.json` still hashes to `b86be4a453e03343fbca2f6afbf1583fd4196973ad3bab1912ced1d378b45007`; every one of the 2,020 sealed records re-verified (bytes + normalized/fs mode); aggregate canonical `10093d38e67739b3352abfff98d9600ed54ad9a543896fe9fb70c84642100157` unchanged; `REVIEW-HANDOFF.md` still `e9218c9ea015779a88708b458655a02d31d6bcd30eac113427ae9d7b9940f445`.
- Candidate references reverified: 667 current primary Cargo/crate inputs equal the map `7a901827b32455aa234912a3f7f00eb92c02e046feef28b48702c611f3eb0aa4`; `qualified-source.tar.gz` recomputes to `e65c7b5e38c77bc9532a1e69fcf3d2cfee092fa18ff4851d7f1e142e14e35c52`; `supplemental-epoch-source.tar.gz` recomputes to `b5f6270a2c4acffb03dc61a5faa08ae38703e115cfbd247636e093a5f313bb52`; `promotion-receipt.json` recomputes to `bb6650089858df0bb54de82e391902ad5321dc1411c58267008f3e206b505785`; capture `testdata/maps/fsmap.bin` is `84f149cb0ca619006f6f6e9ac7117d5749b9828ee826ba4c8c695149944dfc47` in both archives and in the primary tree.
- The original report file was hashed before any addendum work and re-verified at the end: sha256 `5b59c431d814792aa34b92f00584a932aa20af9dc865d787f80aabf1f5ebe564`, 20,361 bytes, mode 100644.

## 2. Ambiguity 1 — supplemental shared-entry relation (exact)

Verified authoritatively archive-vs-archive: the supplemental archive `b5f6270a…` has 669 file members; the qualified archive `e65c7b5e…` has 668; the shared intersection is 668 paths, of which **667 are byte-identical** (including the capture) and **one shared path is modified**: `crates/mds/src/server/runtime/tests.rs`. Plus one new file: `crates/mds/src/server/runtime/tests/supplemental_epoch.rs`. Total **667 unchanged + 1 modified + 1 added = 669**, as the authority expected. Both sealed extractions (`qualified-source/`, `supplemental-copy/` in the original root) were re-verified to still equal their archives, so the stored probe artifacts remain provable.

Exact new pins:

| Item | Record |
|---|---|
| `tests.rs` before (qualified archive) | 33,335 B, mode 100644, sha256 `45425af7b93ce937de51a4a0182b40b7e5acc5d4f94a786c30e5149e60a8599f` |
| `tests.rs` after (supplemental archive) | 33,359 B, mode 100644, sha256 `b9c456fbe1afdf5b2a80f2f755c20b1a6eba71788b539b3e1b5fddc7142b7bd7` |
| Sole delta | exactly one added line `mod supplemental_epoch;` (no removals; 24-byte size delta) |
| Added file | 9,931 B, mode 100644, sha256 `6fe4a7eb723880a3ed7a6d3dd14f81aac2955faaea4bc870dc6ce1bce503391c`, exactly one `#[test]`: `supplemental_daemon_empty_wait_epoch_contrast_and_finite_external_pass` |
| Capture in both archives and primary | 1,643 B, 100644, `84f149cb0ca619006f6f6e9ac7117d5749b9828ee826ba4c8c695149944dfc47` |
| Canonical map of the qualified 667 records | `d6a70aec2880748b4245e4bcafe29ee6ea4c1d1211be3258534992f5c3057ba2` (= the bound source-snapshot identity) |
| Canonical map of the 669 supplemental member records | `f07671f4e8b8f0d4a3ee5ffdc6adc016bcd8faa515fd88958f7b142aacbb0049` (addendum-new pin) |

Mechanism of the ambiguity: the sealed `verify_supplemental.py` checked byte-equality over `qualified_names - modified` (667 names) and verified the modified `tests.rs` through the delta check instead. The original wording folded both into "all shared 668 entries byte-identical". No test result changes follow: the probe's identity was exactly this relation all along.

**Original statements superseded by this correction (quoted, not edited):**

- `current/glm-daemon-e1-d5-runtime-review.md`, Context ledger line: "…16 reference snapshots (1 absent /tmp source; retained record verified)." — see section 3 for this one's correction.
- same file, log line "Supplemental probe verified: …": "…shared 668 entries byte-identical to the candidate; fsmap capture pinned inside."
- same file, section 3: "- Archive `b5f6270a…` extracts 669 entries: all shared 668 entries byte-identical to the candidate extraction; the sole delta is one added line `mod supplemental_epoch;` in `crates/mds/src/server/runtime/tests.rs` plus the new `tests/supplemental_epoch.rs` containing exactly one `#[test]` (`supplemental_daemon_empty_wait_epoch_contrast_and_finite_external_pass`); the pinned fsmap capture is included."
- `REVIEW-HANDOFF.md`, "Fresh evidence" item 3: "3. Supplemental external probe on an owned copy (verified sole delta `mod supplemental_epoch;` + one test file; shared 668 entries byte-identical to the candidate): passed per mode, debug and release, each 1 passed / 179 filtered, reproducing the author tuples exactly …"
- `REVIEW-HANDOFF.md`, "Fresh evidence" item 1 tail: "…16 reference snapshots (1 external `/tmp` source now absent; retained record verified)."

Corrected statement for all of the above: "668 shared supplemental entries = 667 byte-identical + 1 modified (`crates/mds/src/server/runtime/tests.rs`, sole added line `mod supplemental_epoch;`) + 1 new file (`tests/supplemental_epoch.rs`, one `#[test]`); total 669; probe results unchanged."

`pin-verification.json`'s PASS detail strings ("1 absent external /tmp sources: only retained records verified" and the equality-loop wording) are historical script-output receipts and stay as recorded.

## 3. Ambiguity 2 — the snapshot classified "absent" among the 16 references

Original classification, in one of 16 `artifacts/reference-snapshots.json` entries, was produced by my `verify_pins.py`: for each reference it verified the retained copy and then tried the live source with `record()`, whose first gate is `lstat`-is-regular. The entry below failed that gate because the path is a **symlink**, so the script labelled it "absent" without reading through the link. The directory-cleanup narrative in the original report ("paths under cleaned `/tmp/opencode/sol-e1-shared-…`") is wrong and is superseded.

Resolved facts:

- **Source path:** `/tmp/opencode/sol-e1-shared-20261025.0lt2912d/source/current/astra-engine-e1-compat-close-r4-review.md` — a symlink created 2026-10-26 04:12, resolving to `/mnt/workspace/astra-engine-e1-r4-review-bka04gtl/artifacts/astra-engine-e1-compat-close-r4-review.md` (regular file on /mnt/workspace; birth 04:12:13, mtime 04:22:02 — present through both review runs).
- **Expected authoritative record** (bound by `artifacts/reference-snapshots.json`): 19,376 B, mode 100644, sha256 `e5da595f71750b3b995ede6d39abd75ec9016dd94ab224cccdaf3760a481b84c`.
- **Retained file:** `/mnt/workspace/sol-daemon-e1-d5-20261026.g5o2cugy/references/engine-r4/execution-review.md` — NOW re-verified `record == e5da595f…` (bytes independently verify; not metadata-only).
- **Read through the link now:** 19,376 B, sha256 `e5da595f…`, target fs_mode `0o664` — identical to the expectation. Content is **equivalent, live and byte-verified through the exact recorded path**, and double-sourced by the retained copy. Provenance chain: each `copy_reference` in `prepare-freeze.py` asserts `record(source) == record(dest)` at capture; the manifest is sealed as a member of the 494-member evidence bundle whose member manifest `artifacts/bundle-members.json` is a SEAL pin (`4eff891b42b490b2d60dee3265105923cd4fb75101fcf7bf9783751299783648`, re-verified before use this run).
- No other copy needed: there is no `current/astra-engine-e1-compat-close-r4-review.md` in the primary tree (checked; the isolated r4 review lives in its own review root), but the symlink target and the retained copy agree byte-for-byte, so no blocker exists.
- Nothing was recreated, moved or edited; all observations were read-only.

**Scope of the mutable-coordinator distinction:** the one intentionally-skipped entry is `/mnt/workspace/rust-mds/current/sol-e1-r4-integration.md` — the coordinator's mutable integration ledger. Its retained snapshot (`references/current/sol-e1-r4-integration.md`, 7,415 B, sha256 `2405184bc4c6fea7fe5baf2e07af076a9ea134cd653deb8bf45257d0ee497f57`) binds the handoff-time authority content; live content today is 15,020 B, sha256 `7f3d671c50b2919756ad44a8f547e57a207fc0fe0041c81e098a5d139d3fc50a` (grown by coordinator updates). The live-content comparison is intentionally skipped BY DESIGN, and no equality claim covers it.

**Fresh reference-snapshot state (all 16 retained copies re-verified now):**

- 11 sources: present immutable files under `/mnt/workspace/rust-mds` and the joined/handoff roots — live bytes equal their recorded expectations (includes `sol-daemon-e1-d5-plan.md`, `recovery-contract.md`, `persistence-contract.md`, `astra-e1-r4-joined-review.md`, `sol-daemon-review.md`, joined HANDOFF/SEAL/promotion-inventory/joined-inputs, engine-r4 HANDOFF).
- 4 sources under `/tmp/opencode/sol-daemon-r1-strict-ci-20261025.aKufTD/` (`final-daemon-full.log` 285 B, `combined-source-path-mode-sha256-after.txt` 13,207 B, `combined-publication-provenance.json` 342 B, `handoff-sol-daemon.md` 75,081 B): still present regular files; live bytes equal their recorded expectations.
- 1 source (the shared-E1 symlink): readable through the link; bytes equal the expectation (above).
- 1 source: mutable coordinator ledger — intentionally skipped, scoped as above.
- **Absent sources now: 0.**
- The original `pin-verification.json` wording "1 absent external /tmp sources" is superseded by these facts as an enumeration-script artifact, not an evidence problem: retained bytes were verified then and NOW, and the live content is also verified now.

## 4. Original report/handoff stability

Quoted superseded statements were left byte-identical in their files (verified by the section-1 receipt). Addendum work changed only this new ledger file and new artifacts in the addendum root.

## 5. Unchanged boundaries restated

- strict current-path check stays FAILED at 2,848/2,872 with exactly 24 origin-routed authorized-promotion preimages; never a claim of unchanged-2872.
- All test counts (179 full, 10 optimized D5, 1 supplemental per mode, 160 original IDs, 1,162/1,161 assertion blocks) remain fresh historical executions of the original review — not rerun, not extended.
- No live cluster/daemon/mount/key operation, no Git write, no source edit, no new authority claim. Runtime/D5 scope only; startup/storage remains its own lane.

Seal (final): 2026-10-26 07:23 — `ADDENDUM-HANDOFF.md` sha256 `9143ebe0453d520d696b8674ac7388bf8fcb7fa066a2f427602fb5abf1a5d9c5` (4,785 B); addendum `artifacts.sha256.json` sha256 `0fef1a50b3b8953b40a89e3d4e4e4c7b7d61791d5cea9f9f2b603c19885ba7fb` seals 3 files with aggregate canonical `8542f8ed79859f1478c98f9252c24583963ca8dd9600c98ef652cd097c7de3fc`; readback `VERIFIED 3 files`. Original report re-verified after work: sha256 `5b59c431d814792aa34b92f00584a932aa20af9dc865d787f80aabf1f5ebe564` unchanged; original seal/aggregate/handoff unchanged.
