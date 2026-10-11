//! The metadata engine of the basic Rust MDS: an in-memory namespace of one
//! shard with POSIX semantics, the client capability (caps) model, advisory
//! file locks and per-client sessions, with no networking and no dependency
//! on the wire crates. The daemon decodes `MClient*` messages into the input
//! types of this crate, calls the engine, and encodes the returned
//! [`Effect`]s into wire messages, in order, through each session's send
//! queue.
//!
//! # Calling model
//!
//! * One [`Engine`] per shard, driven by one task: every entry point takes
//!   `&mut self` (coarse locking, the proposal's Phase 1). The engine never
//!   blocks and never does I/O.
//! * Every entry point returns `Vec<Effect>`: replies, cap messages, snap
//!   realm splits, and work for the daemon (data truncation and purge, size
//!   recovery, evictions). Send client-bound effects in the returned order;
//!   per (inode, client) cap messages are already in `seq` order.
//! * A request that has to wait (for cap revocations, for an uncommitted
//!   record of another request, for a blocking file lock) produces no reply
//!   from its own call; its reply comes out of a later call (a cap ack, a
//!   journal commit, an unlock, a session close). Each request gets exactly
//!   one reply, unless its session goes away first.
//! * **Safe replies only.** Every mutation becomes exactly one
//!   [`JournalRecord`] handed to the [`Journal`]; its reply (and its
//!   FLUSH_ACKs, `max_size` grants, TRUNC messages and snap splits) are
//!   released only once [`Journal::committed_seq`] covers the record. Until
//!   then other clients' requests touching the changed inodes wait, so no
//!   client observes uncommitted state. With [`MemJournal::new`] records
//!   commit immediately; an asynchronous journal calls back through the
//!   daemon, which then calls [`Engine::journal_committed`].
//!
//! # Entry points
//!
//! | Call | When | Wire source |
//! |---|---|---|
//! | [`Engine::new`] | new file system (root `/` = inode 1, default layout, root realm) | |
//! | [`Engine::replay`] | rebuild from journal records after a restart | |
//! | [`Engine::open_session`] | a client session opened (or was reclaimed) | `CEPH_SESSION_REQUEST_OPEN` |
//! | [`Engine::handle_request`] | a client request | `MClientRequest` |
//! | [`Engine::handle_cap_update`] | cap ack / update / flush / FLUSHSNAP | `MClientCaps` from a client |
//! | [`Engine::handle_cap_release`] | released caps | `MClientCapRelease` |
//! | [`Engine::session_stale`] | the session missed its renewals | (session timer) |
//! | [`Engine::session_resume`] | a stale session renewed | `CEPH_SESSION_REQUEST_RENEWCAPS` |
//! | [`Engine::close_session`] | session close (`kill = false`) or eviction (`kill = true`) | `REQUEST_CLOSE`, eviction |
//! | [`Engine::reconnect_cap`], [`Engine::reconnect_done`] | client reconnect after a restart | `MClientReconnect` |
//! | [`Engine::journal_committed`] | the journal made records durable | (WAL completion) |
//! | [`Engine::tick`] | periodically (bounded durability lag, retries) | (timer) |
//! | [`Engine::recall_hints`] | periodically: sessions over `max_caps_per_client` | sends `RECALL_STATE` |
//! | [`Engine::set_pools`] | a new MDSMap/OSDMap changed the data pools | `MMDSMap`, `MOSDMap` |
//!
//! Introspection for the daemon and tests: [`Engine::inode`],
//! [`Engine::lookup_name`], [`Engine::resolve_path`], [`Engine::list_dir`],
//! [`Engine::cap`], [`Engine::caps_of`], [`Engine::session`],
//! [`Engine::strays`], [`Engine::file_locks`], [`Engine::snap_trace`],
//! [`Engine::snap_realm_info`], [`Engine::vxattr_value`],
//! [`Engine::check_cap_invariants`].
//!
//! # Inputs (decoded by the daemon)
//!
//! * [`Request`]: `tid`, [`Op`] with its arguments (paths are [`FilePath`]:
//!   base inode plus components, as `filepath`/`filepath2`), `caller_uid/gid`,
//!   `owner_uid/gid` (head v3; fill them from the caller for older heads),
//!   the client's `stamp` (all ctime/mtime values come from it), flags
//!   (`CEPH_MDS_FLAG_WANT_DENTRY`, `_REPLAY`), `retry_attempt`,
//!   `oldest_client_tid`, and the embedded [`RequestRelease`]s. Initial
//!   xattrs of create ops (the request data payload, decoded as an xattr map)
//!   go in the op's `xattrs`. Unknown ops map to [`Op::Other`].
//! * [`CapUpdate`]: an `MClientCaps` from a client (op, ino, cap_id, seq,
//!   issue_seq, mseq, caps = the client's implemented set, wanted, dirty,
//!   flush tid, oldest_flush_tid, flags, and the inode fields).
//! * [`CapReleaseItem`]: one `ceph_mds_cap_item`.
//!
//! # Outputs ([`Effect`])
//!
//! * [`Effect::Reply`] → `MClientReply` (always safe): `result`;
//!   `dentry` = the `is_dentry` part (parent [`InodeStat`], [`DirFragInfo`],
//!   name, [`LeaseStat`] — null leases only in P0); `target` = the
//!   `is_target` [`InodeStat`]; `snap_trace` = the `snapbl`; `extra` =
//!   `openc_response_t` for CREATE ([`ReplyExtra::Create`], no delegated
//!   inodes in P0), the readdir blob, the GETFILELOCK `ceph_filelock`, or the
//!   GETVXATTR value (encode as a versioned string). Each [`InodeStat`]
//!   carries the client's cap ([`ReplyCap`], always `CEPH_CAP_FLAG_AUTH`),
//!   `version = 2 * inode version`, `inline_version = CEPH_INLINE_NONE`,
//!   directory `nlink` 1 (0 once removed), the xattr blob when the client
//!   needs it, and `subvolume_id`.
//! * [`Effect::Caps`] → `MClientCaps` GRANT, REVOKE, TRUNC, FLUSH_ACK or
//!   FLUSHSNAP_ACK ([`CapMessage`]; `flush_tid` is the header tid; `inode`
//!   carries the fields `encode_cap_message` fills). Counted for `push_seq`.
//! * [`Effect::RealmSplit`] → `MClientSnap` op SPLIT with the given trace
//!   (a quota or the subvolume flag opened a realm); sent before the reply.
//! * [`Effect::TruncateData`], [`Effect::PurgeInode`]: data-object work after
//!   the record committed. [`Effect::RecoverFileSize`]: an evicted writer had
//!   a writeable range; probe objects up to `max_size`.
//! * [`Effect::Recall`] (from [`Engine::recall_hints`]) → `RECALL_STATE`.
//! * [`Effect::EvictClient`]: a stale client holds write caps another client
//!   needs: blocklist it, then call [`Engine::close_session`] with `kill`.
//!
//! # Namespace
//!
//! Operations and their C++ check order are those of `Server.cc`
//! (`cpp-map/03-request-path`): LOOKUP (negative lookups reply with the
//! dentry when `WANT_DENTRY` is set), GETATTR, LOOKUPINO/LOOKUPHASH (ESTALE
//! for unknown, purged and `[0x100, 0x1000)` numbers), LOOKUPPARENT,
//! LOOKUPNAME (always with the target), SETATTR (mode, uid/gid with the
//! setuid/setgid clearing rules, `KILL_*`, times with `time_warp_seq`, size
//! growth and truncation with `truncate_seq`), MKDIR, MKNOD, CREATE (with
//! `O_EXCL`; an existing name without `O_EXCL` is opened), OPEN (with
//! `O_TRUNC`), SYMLINK, LINK, UNLINK, RMDIR, RENAME (all overwrite and
//! ancestry rules; EXDEV across subvolumes), READDIR (one frag, `(rjenkins
//! hash24, name)` order, resume by last name or offset hash, FRAG_END and
//! FRAG_COMPLETE), SETXATTR/RMXATTR (plain xattrs opaque, POSIX ACLs
//! included, zero-length values kept, 64 KiB total), the `ceph.*` virtual
//! xattrs of design 4.2 (layouts, quota, `ceph.dir.subvolume`, pins) and
//! GETVXATTR (ENODATA for other names), SETFILELOCK/GETFILELOCK (fcntl and
//! flock), LOOKUPSNAP (ENOENT), LSSNAP (empty, FRAG_END), MKSNAP (EPERM).
//!
//! New inodes take the request's owner, the group (and for directories
//! `S_ISGID`) of a setgid parent, the client's stamp for all four times,
//! `truncate_seq = 1`, `truncate_size = u64::MAX`, the inherited layout
//! (files), rjenkins dentry hashing (directories). Directory statistics and
//! recursive statistics (`rbytes`, `rfiles`, `rsubdirs` counting the
//! directory itself, `rctime`) are kept exact through primary links. An
//! unlinked inode that a client still holds caps on stays as a stray
//! (`nlink` 0, outside the statistics) and is purged when the last cap goes.
//! When the primary link of a hard-linked file goes, a remaining link becomes
//! primary. Inode numbers come from [`ino::InoAllocator`] (range leases, the
//! C++ rank-0 range first, never reused, below 2^48, sub-range delegation
//! for later async creates). Successful mutations are recorded in the
//! session's completed-request table in the same journal record; a retried
//! request is answered from it (a retried CREATE re-opens what it created).
//!
//! # Caps
//!
//! Per (inode, client) a [`caps::Capability`] with `cap_id`, `pending`,
//! `issued`, `wanted`, `last_sent` (seq), `last_issue` (issue_seq) and `mseq`,
//! following the C++ issue/acknowledge rules (`Capability::issue`,
//! `confirm_receipt`). The policy is the client-visible outcome of the C++
//! lock tables (see `engine::caps_eval`): a lone writer gets
//! `Fs Fx Fc Fr Fw Fb` plus `Ax`/`Xx` it wants; readers share `Fs Fc Fr`;
//! writers with others fall back to `Fr Fw` (synchronous I/O); directories
//! get `Fs` (and `Fx` for a lone client), never `Fc`/`Fr`/`Fw`/`Fb`.
//! Requests that change or need fresh state register *holds* that revoke
//! conflicting caps first (e.g. a namespace change revokes the other clients'
//! `Fs` on the directory, a truncate leaves everybody at most `Fc Fb`) and
//! wait for the acknowledgements. Cap flushes apply dirty metadata with the
//! C++ field rules and are acknowledged after commit, in commit order;
//! writeable ranges (`max_size`) are journaled before they are granted.
//! Stale sessions: their caps are ignored by the policy, revoked silently
//! when needed, or the client is evicted when it holds write caps others
//! need; on resume the caps are re-issued.
//!
//! # Example
//!
//! ```
//! use engine::consts::*;
//! use engine::*;
//!
//! let mut e = Engine::new(EngineConfig::default(), MemJournal::new());
//! e.open_session(4100);
//! let req = Request::new(
//!     1,
//!     Op::Create {
//!         path: FilePath::child(1, "hello"),
//!         flags: CEPH_O_CREAT | CEPH_O_RDWR,
//!         mode: 0o644,
//!         layout: LayoutOverride::default(),
//!         xattrs: XattrMap::new(),
//!         ino: 0,
//!     },
//! )
//! .with_owner(1000, 1000)
//! .with_stamp(Timestamp::new(1_700_000_000, 0));
//! let effects = e.handle_request(4100, req);
//! let Effect::Reply(reply) = &effects[0] else { panic!() };
//! assert_eq!(reply.result, 0);
//! let st = reply.target.as_ref().unwrap();
//! assert_eq!(caps::cap_string(st.cap.caps), "pAsxLsXsxFsxcrwb");
//! assert_eq!(st.max_size, 4 << 20);
//! ```
//!
//! # Not (yet) here
//!
//! Dentry leases (null leases only), early replies, async dirops and inode
//! delegation (the hooks exist), snapshots, MClientQuota pushes, MDS auth-cap
//! checks (POSIX permission checks are done by clients, as with the C++ MDS),
//! multiple ranks and cap export/import, persistence beyond the journal
//! records. See the module docs for the remaining deviations from the C++.

pub mod caps;
pub mod consts;
pub mod effects;
mod engine;
pub mod errno;
pub mod filelock;
pub mod hash;
pub mod ino;
pub mod inode;
pub mod journal;
pub mod request;
pub mod session;
pub mod types;

pub use effects::*;
pub use engine::{Engine, EngineConfig};
pub use errno::Errno;
pub use journal::{Journal, JournalRecord, JournalSeq, MemJournal};
pub use request::*;
pub use types::*;
