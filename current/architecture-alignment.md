# architecture-alignment
Task: Align the Rust MDS design documents with subtree sharding. A shard is an arbitrary namespace partition. A subvolume is a tenant boundary. Fine-grained transactions are proved at workplan G5, after the selected single-shard store.
Status: Rebased onto Markuze/rust-mds main. D3, D4 and D5 no longer treat a subvolume as a shard. G0–G3 and the stable WAL stay. `design/transactions.md` is the G5 model. `design/store.md` sections 6 and 7 hold the later identity and cross-shard rules. The catalog JSON is not reclassified. No engine concurrency change in this pass.
Context: The 2026-10-28 revision was written against the pre-persistence tree. Upstream already selected one root shard, one stream, full images and a scalar sequence. The rebase keeps that implementation and does not encode inode-range ownership.
Next: G5 concurrency proof. Do not replace the landed single-shard WAL with inode-range ownership.
Log:
- [2026-10-28 13:30] Started from the architecture review. Normative edits are the decision log, the store identity model, transaction isolation, and the workplan order.
- [2026-10-28 14:10] Design documents revised. Catalog reclassification left open on purpose.
- [2026-10-28 15:30] Rebased onto upstream. Kept G0–G3. Mapped the concurrency proof to G5 and the tenant boundary to G4.
