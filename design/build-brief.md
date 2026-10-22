# Build brief: the basic Rust MDS

> The working brief the build agents followed during the first build (2026-10-11): conventions, crate ownership, decisions in force and the dev-cluster contract. Kept for whoever continues the work; see `HANDOVER.md` for the current status.

**Historical ownership notice (2026-10-22):** The owner table, editing restrictions, environment markers and first-build status below describe the original run. They do not assign files or cluster access in the coordinated G0-G3 run. Follow the parent's explicit assignments, `AGENTS.md`, `design/workplan.md` and `design/persistence-contract.md`. The parent orchestrates; assigned Sol implementers and independent Sol reviewers do the work. QA alone controls the live cluster. No commits or branch changes are authorized by this brief. Wire, licensing and safe-reply conventions remain applicable. The kernel reference tree now exists at `/mnt/workspace/ceph-client`; the old absence statement below is historical.

You are one of several agents building a new CephFS metadata server (MDS) in Rust, in the repository `/mnt/workspace/rust-mds`. Read this brief fully, then your task. The goal right now: **a basic Rust MDS that a real Ceph cluster accepts, that the Linux kernel client and ceph-fuse can mount, and on which they can do simple I/O** (mkdir, create, write, read, ls, unlink, rename). Speed matters, but so does not having to rewrite things later.

## Context you must know

- Design documents: `/mnt/workspace/rust-mds/design/minimum-feature-set.md` (read sections 3, 4.1, 4.2, 5.1 and 8.1 at least) and `design/workplan.md` (stages 1-3 are what we are building now).
- Reference material, facts only, about the C++ MDS and its peers: `/mnt/workspace/rust-mds/mds-rust-map/` (start at `cpp-map/README.md`; the client-protocol pages under `cpp-map/02-client-protocol/` and `cpp-map/00-architecture/transport-auth-and-client-library-contract.md` have byte-level detail and checklists).
- C++ Ceph source (LGPL), the reference for exact encodings and behaviour: `/home/ubuntu/ceph/src` (commit of the map). Useful: `messages/*.h`, `msg/async/ProtocolV1.cc`, `auth/cephx/*`, `mon/MonClient.cc`, `mds/*.cc`, `include/ceph_fs.h`, `include/msgr.h`, `include/rados.h`, `common/ceph_strings.cc`.
- Kernel CephFS client headers (wire structs): `/usr/src/linux-ibm-headers-7.0.0-1012/include/linux/ceph/*.h`. The kernel's `.c` sources are not on this machine.
- **License caution:** the kernel's `net/ceph` and `fs/ceph` are GPL-2.0. You may read kernel code (for example fetched from git.kernel.org or GitHub into a new empty directory under your scratch space) to understand what the client expects, but do **not** translate kernel code into ours. Implement from the protocol facts, the map, and the C++ sources.

## Decisions that constrain the code

- Target peers: Ceph 20.2 packages in the dev cluster (Ubuntu 26.04), the host kernel 7.0 CephFS client, ceph-fuse/libcephfs 20.2. Later Ceph `main`.
- **msgr1 first, in both directions** (decision 4): the MDS binds a msgr1 (v1, `TYPE_LEGACY`) address only, connects to monitors over msgr1, and accepts kernel and libcephfs clients over msgr1. msgr2 comes later behind the same messenger abstraction, so keep the transport behind a trait/enum boundary.
- cephx authentication everywhere; AES keys (the cluster default). Message signatures on msgr1 (`MSG_AUTH`) are required by the kernel.
- **Native Rust for every layer**: no librados, no linking to C++ Ceph libraries.
- **Compat set** (decision 2): the daemon advertises the C++ default MDS incompat bits 1-12 (including 7) plus a private bit 63 named `rust mds format v0`. Constant for the process lifetime.
- **Safe replies only** (decision 5): a reply is sent after the mutation is committed; no unsafe/early replies in this phase.
- Day-one hooks (minimum-feature-set.md 5.1): connection reading is separate from request execution; every outgoing client message goes through one per-session send queue; the engine produces self-contained journal records per transaction even if the journal is in memory for now.
- Incremental (D18): simplest correct form first; no performance tuning now.

## Repository layout and ownership

Cargo workspace at `/mnt/workspace/rust-mds` (edition 2024, Rust 1.99). Crates under `crates/`:

| Crate | Owner | Content |
|---|---|---|
| `denc` | orchestrator (do not edit) | encoding traits `Encode`/`Decode`, primitives, containers, `encode_start`/`decode_start` framing, `UTime`, `EntityName`, `EntityType`, `EntityAddr`, `EntityAddrVec`, `Uuid`, feature constants `denc::features::*`, the message envelope `denc::msg::{RawMessage, Message, Payload}` |
| `auth`, `msgr`, `monc` | platform agent | cephx; msgr1 client and server; MonClient, MDSMap/FSMap decoding, beacons |
| `proto` | protocol agent | CephFS client messages (`MClient*`) |
| `engine` | engine agent | in-memory metadata engine and caps model |
| `osdc` | osd-client agent | OSDMap decoding, CRUSH, object placement, MOSDOp/MOSDOpReply, the OSD client (Objecter) |
| `mds` (binary `ceph-rmds`) | orchestrator, later | the daemon that ties them together |

Rules:

- Edit only the files of your own crates (plus `Cargo.toml` of your crates). Do not edit `crates/denc`, the workspace `Cargo.toml` (except adding a `[workspace.dependencies]` entry is NOT allowed either: put dependencies directly in your crate's `Cargo.toml`), or other agents' crates. If you need something in `denc`, implement it locally in your crate and say so in your final report.
- Build and test only your crates, with your own target directory to avoid lock contention: `CARGO_TARGET_DIR=/mnt/workspace/rust-mds/target-<your-agent-name> cargo test -p <crate>`.
- Do not run `git commit`, `git checkout` or anything that changes git state.
- Only the environment agent installs packages with apt (one dpkg lock). Others wait for the marker files described below.
- Write doc comments that state the wire format you implement and cite the C++ file and symbol you followed. Add unit tests with byte-level expectations; prefer golden data from real Ceph (`ceph-dencoder`, captures) over hand-made bytes.

## Dev cluster

The environment agent sets up a single-node Ceph 20.2 cluster and writes:

- `/mnt/workspace/rust-mds/devcluster/PACKAGES_READY` once Ceph packages (and `ceph-dencoder` if available) are installed;
- `/mnt/workspace/rust-mds/devcluster/READY` once the cluster runs; `devcluster/env.sh` (source it) exports `CEPH_CONF`, the keyring paths, the monitor addresses and the fsid; `devcluster/README.md` explains the scripts (start, stop, mount helpers, which MDS is running).

Poll for these files (for example every 60 s) before tests that need them; do offline work meanwhile. Never stop or reset the cluster yourself unless your task says so; coordinate through the scripts it provides and do not leave stray daemons running.

## Useful facts

- Message type ids: `include/ceph_fs.h` (`CEPH_MSG_CLIENT_*`, `CEPH_MSG_MDS_MAP` = 21, `CEPH_MSG_FS_MAP` = 45, ...), `include/msgr.h`, `msg/Message.h` (`MSG_MDS_BEACON` = 100, `CEPH_MSG_AUTH` = 17, `CEPH_MSG_AUTH_REPLY` = 18, `CEPH_MSG_MON_SUBSCRIBE` = 15, `CEPH_MSG_MON_SUBSCRIBE_ACK` = 16, `CEPH_MSG_MON_MAP` = 4, `MSG_CONFIG` = 62, ...).
- `ceph-dencoder` (once installed): `ceph-dencoder list_types`, `ceph-dencoder type <T> import <file> decode dump_json` decodes bytes you produce; `... encode export <file>` produces reference bytes for `generate_test_instances`. Many message types are supported.

## Final report

At most 400 words: what works (with the exact command that demonstrates it), what does not, the public API of your crates (key types and functions), anything you needed in `denc`, and open problems.
