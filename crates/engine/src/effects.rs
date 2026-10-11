//! Engine outputs ("effects"): reply payloads, cap messages, lock wake-ups as
//! replies, and work for the daemon. Every engine call returns an ordered
//! `Vec<Effect>`; the daemon sends the client-bound ones in that order through
//! the per-session send queue (which counts `push_seq` for cap messages) and
//! performs the others.

use crate::consts::*;
use crate::filelock::FileLock;
use crate::types::*;

/// The cap part of an `InodeStat` (`ceph_mds_reply_cap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReplyCap {
    pub caps: u32,
    pub wanted: u32,
    pub cap_id: u64,
    pub seq: u32,
    pub mseq: u32,
    /// Snap realm the inode belongs to.
    pub realm: Ino,
    /// `CEPH_CAP_FLAG_AUTH` (always, with a single rank).
    pub flags: u8,
}

/// Inode attributes as sent in reply traces and readdir entries (the content
/// of InodeStat v9, `CInode::encode_inodestat`).
#[derive(Debug, Clone, PartialEq)]
pub struct InodeStat {
    pub ino: Ino,
    /// Always `CEPH_NOSNAP` in P0.
    pub snapid: u64,
    pub rdev: u32,
    /// `2 * inode version` (the C++ adds 1 for projected inodes; never here).
    pub version: u64,
    /// Version of the included xattr blob; 0 when no blob is included.
    pub xattr_version: u64,
    pub cap: ReplyCap,
    pub layout: FileLayout,
    pub ctime: Timestamp,
    pub mtime: Timestamp,
    pub atime: Timestamp,
    pub btime: Timestamp,
    pub time_warp_seq: u32,
    pub size: u64,
    /// The client's writeable range (`client_ranges[client]`), 0 if none.
    pub max_size: u64,
    pub truncate_size: u64,
    pub truncate_seq: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub nlink: u32,
    pub dirstat: DirStat,
    pub rstat: Rstat,
    /// `dir_layout.dl_dir_hash` (directories), 0 otherwise.
    pub dir_hash: u8,
    pub symlink: Vec<u8>,
    /// The xattr blob, when included (see `xattr_version`).
    pub xattrs: Option<XattrMap>,
    /// `CEPH_INLINE_NONE`; never 0 (libcephfs requirement).
    pub inline_version: u64,
    pub quota: Quota,
    pub change_attr: u64,
    pub export_pin: i32,
    /// The subvolume root this inode is in, 0 if none. Never changes once
    /// non-zero (kernel WARN).
    pub subvolume_id: Ino,
    pub fscrypt_auth: Vec<u8>,
    pub fscrypt_file: Vec<u8>,
}

/// `DirStat` of a dentry trace or readdir reply (`ceph_mds_reply_dirfrag`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirFragInfo {
    /// Always the root frag (0): one frag per directory (design D5).
    pub frag: u32,
    /// Auth rank of the dirfrag if it is a subtree root, else -1
    /// (`CDIR_AUTH_PARENT`).
    pub auth: i32,
    pub dist: Vec<i32>,
}

/// `LeaseStat` v2. P0 issues only null leases (mask 0, duration 0).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LeaseStat {
    pub mask: u16,
    pub duration_ms: u32,
    pub seq: u32,
    pub alternate_name: Vec<u8>,
}

/// The dentry part of a trace (`is_dentry`): the request's parent directory,
/// its frag info, the name and a lease.
#[derive(Debug, Clone, PartialEq)]
pub struct DentryTrace {
    pub dir: InodeStat,
    pub dirfrag: DirFragInfo,
    pub name: Vec<u8>,
    pub lease: LeaseStat,
}

/// One readdir entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ReaddirEntry {
    pub name: Vec<u8>,
    pub lease: LeaseStat,
    pub inode: InodeStat,
}

/// Readdir extra payload: `DirStat`, entries, flags (`CEPH_READDIR_*`).
#[derive(Debug, Clone, PartialEq)]
pub struct ReaddirReply {
    pub dirfrag: DirFragInfo,
    pub entries: Vec<ReaddirEntry>,
    pub flags: u16,
}

/// The op-specific extra payload of a reply.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ReplyExtra {
    #[default]
    None,
    /// CREATE: `openc_response_t {created_ino, delegated_inos}` (P0: no
    /// delegation). Absent when CREATE opened an existing file.
    Create {
        ino: Ino,
        delegated: Vec<InoRange>,
    },
    Readdir(ReaddirReply),
    /// GETFILELOCK: the conflicting lock, or the request with type UNLOCK.
    FileLock(FileLock),
    /// GETVXATTR: the value (the daemon encodes it as a versioned string).
    Vxattr(Vec<u8>),
}

/// A reply (`MClientReply`), always safe (sent after commit).
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub client: ClientId,
    pub tid: u64,
    pub op: u32,
    /// 0 or a negative errno.
    pub result: i32,
    /// `is_dentry` part of the trace.
    pub dentry: Option<DentryTrace>,
    /// `is_target` part of the trace.
    pub target: Option<InodeStat>,
    /// Realm whose snap trace goes into `snapbl` (the target's, else the
    /// dentry directory's); `None` for traceless replies.
    pub snap_realm: Option<Ino>,
    /// That realm and its ancestors, innermost first (the `snapbl` content).
    pub snap_trace: Vec<SnapRealmInfo>,
    pub extra: ReplyExtra,
    /// Inode created by this request (also for a retried request answered
    /// from the completed-request table).
    pub created_ino: Option<Ino>,
}

impl Reply {
    pub fn is_ok(&self) -> bool {
        self.result == 0
    }
}

/// Inode fields carried by GRANT, REVOKE and TRUNC cap messages
/// (`CInode::encode_cap_message`).
#[derive(Debug, Clone, PartialEq)]
pub struct CapInodeFields {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub nlink: u32,
    /// `(xattr_version, blob)` when the client gets the blob.
    pub xattrs: Option<(u64, XattrMap)>,
    pub size: u64,
    pub max_size: u64,
    pub truncate_size: u64,
    pub truncate_seq: u32,
    pub mtime: Timestamp,
    pub atime: Timestamp,
    pub ctime: Timestamp,
    pub btime: Timestamp,
    pub layout: FileLayout,
    pub time_warp_seq: u32,
    pub change_attr: u64,
    pub nfiles: i64,
    pub nsubdirs: i64,
    pub inline_version: u64,
}

/// A cap message to a client (`MClientCaps`).
#[derive(Debug, Clone, PartialEq)]
pub struct CapMessage {
    pub client: ClientId,
    /// `CEPH_CAP_OP_GRANT`, `_REVOKE`, `_TRUNC` or `_FLUSH_ACK`.
    pub op: u32,
    pub ino: Ino,
    pub realm: Ino,
    pub cap_id: u64,
    pub seq: u32,
    pub issue_seq: u32,
    pub caps: u32,
    pub wanted: u32,
    pub dirty: u32,
    pub mseq: u32,
    /// Header tid: the flush tid for FLUSH_ACK, else 0.
    pub flush_tid: u64,
    pub oldest_flush_tid: u64,
    /// Inode fields for GRANT/REVOKE/TRUNC; `None` for FLUSH_ACK.
    pub inode: Option<CapInodeFields>,
}

impl CapMessage {
    pub fn op_name(&self) -> &'static str {
        match self.op {
            CEPH_CAP_OP_GRANT => "grant",
            CEPH_CAP_OP_REVOKE => "revoke",
            CEPH_CAP_OP_TRUNC => "trunc",
            CEPH_CAP_OP_FLUSH_ACK => "flush_ack",
            CEPH_CAP_OP_FLUSHSNAP_ACK => "flushsnap_ack",
            _ => "other",
        }
    }
}

/// Truncate RADOS data objects after a shrinking truncation was committed.
#[derive(Debug, Clone, PartialEq)]
pub struct TruncateData {
    pub ino: Ino,
    pub layout: FileLayout,
    pub truncate_seq: u32,
    /// New size.
    pub size: u64,
    /// Size before the truncation.
    pub from: u64,
}

/// Delete the data objects of an inode that is gone for good (after the purge
/// record committed). `size` is the largest size a client may have written.
#[derive(Debug, Clone, PartialEq)]
pub struct PurgeInode {
    pub ino: Ino,
    pub layout: FileLayout,
    pub size: u64,
    pub is_dir: bool,
}

/// A new snap realm was opened (quota or subvolume flag set): tell a client
/// which of its cached inodes and child realms moved into it
/// (`MClientSnap` op SPLIT). Sent before the reply of the request.
#[derive(Debug, Clone, PartialEq)]
pub struct RealmSplit {
    pub client: ClientId,
    pub realm: Ino,
    pub split_inos: Vec<Ino>,
    pub split_realms: Vec<Ino>,
    /// The new realm and its ancestors, innermost first (the message's trace).
    pub snap_trace: Vec<SnapRealmInfo>,
}

/// Snap realm description for building snap traces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapRealmInfo {
    pub ino: Ino,
    /// Parent realm, 0 for the root realm.
    pub parent: Ino,
    pub seq: u64,
    pub created: u64,
    pub parent_since: u64,
    pub subvolume: bool,
    pub last_modified: Timestamp,
    pub change_attr: u64,
}

/// Everything an engine call asks the daemon to do, in order.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Effect {
    Reply(Reply),
    Caps(CapMessage),
    RealmSplit(RealmSplit),
    TruncateData(TruncateData),
    PurgeInode(PurgeInode),
    /// Cache-pressure hint: ask the client to trim to `max_caps` caps
    /// (`CEPH_SESSION_RECALL_STATE`).
    Recall {
        client: ClientId,
        max_caps: u64,
    },
    /// A stale client holds write caps another client needs: evict it
    /// (blocklist, then `Engine::close_session(client, true)`).
    EvictClient {
        client: ClientId,
    },
    /// A writer was lost while it had a writeable range: recover size and
    /// mtime by probing objects up to `max_size` (`RecoveryQueue`).
    RecoverFileSize {
        ino: Ino,
        max_size: u64,
    },
}

impl Effect {
    /// The client a message effect is addressed to.
    pub fn client(&self) -> Option<ClientId> {
        match self {
            Effect::Reply(r) => Some(r.client),
            Effect::Caps(c) => Some(c.client),
            Effect::RealmSplit(s) => Some(s.client),
            Effect::Recall { client, .. } => Some(*client),
            _ => None,
        }
    }
}
