//! Engine inputs: client requests and client cap messages, as the daemon
//! decodes them from `MClientRequest`, `MClientCaps` and `MClientCapRelease`.

use crate::consts::*;
use crate::types::*;

/// Arguments of SETATTR (`ceph_mds_request_args::setattr`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetattrArgs {
    /// `CEPH_SETATTR_*` mask.
    pub mask: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: Timestamp,
    pub atime: Timestamp,
    pub btime: Timestamp,
    pub size: u64,
    /// The client's view of the size before a truncation.
    pub old_size: u64,
}

/// Layout fields a CREATE may override (`args.open`); zero (or a negative
/// pool) means "inherit".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutOverride {
    pub stripe_unit: u32,
    pub stripe_count: u32,
    pub object_size: u32,
    pub pool: i64,
}

impl Default for LayoutOverride {
    fn default() -> Self {
        LayoutOverride {
            stripe_unit: 0,
            stripe_count: 0,
            object_size: 0,
            pool: -1,
        }
    }
}

/// A file-lock request (`args.filelock_change`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileLockArgs {
    /// `CEPH_LOCK_FCNTL`, `CEPH_LOCK_FLOCK` or their `_INTR` variants.
    pub rule: u8,
    /// `CEPH_LOCK_SHARED`, `CEPH_LOCK_EXCL` or `CEPH_LOCK_UNLOCK`.
    pub ltype: u8,
    pub owner: u64,
    pub pid: u64,
    pub start: u64,
    /// 0 means to end of file.
    pub length: u64,
    /// Block until granted (F_SETLKW / blocking flock).
    pub wait: bool,
}

/// The operation of a request with its arguments.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// LOOKUP (0x100): `path` = parent + name. `mask` is the getattr mask.
    Lookup { path: FilePath, mask: u32 },
    /// GETATTR (0x101): `path` names the inode (depth 0) or a path below it.
    Getattr { path: FilePath, mask: u32 },
    /// LOOKUPINO (0x104) and LOOKUPHASH (0x102).
    LookupIno { ino: Ino, mask: u32 },
    /// LOOKUPPARENT (0x103).
    LookupParent { ino: Ino },
    /// LOOKUPNAME (0x105); `parent` is the optional `filepath2` ino.
    LookupName { ino: Ino, parent: Option<Ino> },
    /// SETATTR (0x1108).
    Setattr { path: FilePath, args: SetattrArgs },
    /// OPEN (0x302) of an existing inode.
    Open {
        path: FilePath,
        flags: u32,
        mode: u32,
        mask: u32,
        old_size: u64,
    },
    /// CREATE (0x1301): `path` = parent + name.
    Create {
        path: FilePath,
        flags: u32,
        mode: u32,
        layout: LayoutOverride,
        xattrs: XattrMap,
        ino: Ino,
    },
    /// MKNOD (0x1201).
    Mknod {
        path: FilePath,
        mode: u32,
        rdev: u32,
        xattrs: XattrMap,
        ino: Ino,
    },
    /// MKDIR (0x1220).
    Mkdir {
        path: FilePath,
        mode: u32,
        xattrs: XattrMap,
        ino: Ino,
    },
    /// SYMLINK (0x1222): `target` is `path2`.
    Symlink {
        path: FilePath,
        target: Vec<u8>,
        xattrs: XattrMap,
        ino: Ino,
    },
    /// LINK (0x1202): new name `path`, existing inode `target` (`path2`,
    /// either a bare ino or parent + name).
    Link { path: FilePath, target: FilePath },
    /// UNLINK (0x1203).
    Unlink { path: FilePath },
    /// RMDIR (0x1221).
    Rmdir { path: FilePath },
    /// RENAME (0x1204): `dst` is `path`, `src` is `path2`.
    Rename { dst: FilePath, src: FilePath },
    /// READDIR (0x305).
    Readdir {
        path: FilePath,
        frag: u32,
        max_entries: u32,
        max_bytes: u32,
        /// `path2`: the last name the client received.
        last_name: Option<Vec<u8>>,
        offset_hash: u32,
        /// `CEPH_READDIR_REPLY_BITFLAGS` if set by the client.
        flags: u32,
    },
    /// SETXATTR (0x1105).
    Setxattr {
        path: FilePath,
        name: Vec<u8>,
        value: Vec<u8>,
        flags: u32,
    },
    /// RMXATTR (0x1106).
    Rmxattr { path: FilePath, name: Vec<u8> },
    /// GETVXATTR (0x106).
    Getvxattr { path: FilePath, name: Vec<u8> },
    /// SETFILELOCK (0x1109).
    SetFileLock { path: FilePath, lock: FileLockArgs },
    /// GETFILELOCK (0x110).
    GetFileLock { path: FilePath, lock: FileLockArgs },
    /// LOOKUPSNAP (0x400): no snapshots in P0, answered ENOENT.
    LookupSnap { path: FilePath },
    /// LSSNAP (0x402): an empty listing with FRAG_END.
    Lssnap { path: FilePath },
    /// MKSNAP (0x1400): EPERM.
    Mksnap { path: FilePath },
    /// Any other op code: EOPNOTSUPP.
    Other { op: u32 },
}

impl Op {
    /// The wire op code (`CEPH_MDS_OP_*`).
    pub fn code(&self) -> u32 {
        match self {
            Op::Lookup { .. } => CEPH_MDS_OP_LOOKUP,
            Op::Getattr { .. } => CEPH_MDS_OP_GETATTR,
            Op::LookupIno { .. } => CEPH_MDS_OP_LOOKUPINO,
            Op::LookupParent { .. } => CEPH_MDS_OP_LOOKUPPARENT,
            Op::LookupName { .. } => CEPH_MDS_OP_LOOKUPNAME,
            Op::Setattr { .. } => CEPH_MDS_OP_SETATTR,
            Op::Open { .. } => CEPH_MDS_OP_OPEN,
            Op::Create { .. } => CEPH_MDS_OP_CREATE,
            Op::Mknod { .. } => CEPH_MDS_OP_MKNOD,
            Op::Mkdir { .. } => CEPH_MDS_OP_MKDIR,
            Op::Symlink { .. } => CEPH_MDS_OP_SYMLINK,
            Op::Link { .. } => CEPH_MDS_OP_LINK,
            Op::Unlink { .. } => CEPH_MDS_OP_UNLINK,
            Op::Rmdir { .. } => CEPH_MDS_OP_RMDIR,
            Op::Rename { .. } => CEPH_MDS_OP_RENAME,
            Op::Readdir { .. } => CEPH_MDS_OP_READDIR,
            Op::Setxattr { .. } => CEPH_MDS_OP_SETXATTR,
            Op::Rmxattr { .. } => CEPH_MDS_OP_RMXATTR,
            Op::Getvxattr { .. } => CEPH_MDS_OP_GETVXATTR,
            Op::SetFileLock { .. } => CEPH_MDS_OP_SETFILELOCK,
            Op::GetFileLock { .. } => CEPH_MDS_OP_GETFILELOCK,
            Op::LookupSnap { .. } => CEPH_MDS_OP_LOOKUPSNAP,
            Op::Lssnap { .. } => CEPH_MDS_OP_LSSNAP,
            Op::Mksnap { .. } => CEPH_MDS_OP_MKSNAP,
            Op::Other { op } => *op,
        }
    }

    /// `MClientRequest::may_write`: write ops, and OPEN with `O_CREAT|O_TRUNC`.
    pub fn may_write(&self) -> bool {
        match self {
            Op::Open { flags, .. } => flags & (CEPH_O_CREAT | CEPH_O_TRUNC) != 0,
            _ => self.code() & CEPH_MDS_OP_WRITE != 0,
        }
    }
}

/// A cap/dentry-lease release embedded in a request
/// (`ceph_mds_request_release`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestRelease {
    pub ino: Ino,
    pub cap_id: u64,
    /// Caps the client keeps.
    pub caps: u32,
    pub wanted: u32,
    pub seq: u32,
    pub issue_seq: u32,
    pub mseq: u32,
    /// Dentry lease being released with it (P0 issues no leases).
    pub dname: Option<Vec<u8>>,
}

/// A client request (`MClientRequest`, head v3 fields the engine uses).
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub tid: u64,
    pub op: Op,
    pub caller_uid: u32,
    pub caller_gid: u32,
    /// Owner of inodes the request creates (head v3 `owner_uid`/`owner_gid`).
    pub owner_uid: u32,
    pub owner_gid: u32,
    /// The client's time stamp; every ctime/mtime the request sets uses it.
    /// The daemon fills a zero stamp with its own clock
    /// (`MutationImpl::get_op_stamp`).
    pub stamp: Timestamp,
    /// `CEPH_MDS_FLAG_*`.
    pub flags: u32,
    pub retry_attempt: u32,
    pub oldest_client_tid: u64,
    pub releases: Vec<RequestRelease>,
}

impl Request {
    /// A request with root credentials, no flags and a zero stamp.
    pub fn new(tid: u64, op: Op) -> Request {
        Request {
            tid,
            op,
            caller_uid: 0,
            caller_gid: 0,
            owner_uid: 0,
            owner_gid: 0,
            stamp: Timestamp::ZERO,
            flags: 0,
            retry_attempt: 0,
            oldest_client_tid: 0,
            releases: Vec::new(),
        }
    }

    pub fn with_stamp(mut self, stamp: Timestamp) -> Request {
        self.stamp = stamp;
        self
    }

    pub fn with_owner(mut self, uid: u32, gid: u32) -> Request {
        self.owner_uid = uid;
        self.owner_gid = gid;
        self.caller_uid = uid;
        self.caller_gid = gid;
        self
    }

    pub fn with_flags(mut self, flags: u32) -> Request {
        self.flags |= flags;
        self
    }

    pub fn want_dentry(&self) -> bool {
        self.flags & CEPH_MDS_FLAG_WANT_DENTRY != 0
    }

    pub fn is_replay(&self) -> bool {
        self.flags & CEPH_MDS_FLAG_REPLAY != 0
    }
}

/// A cap message from a client (`MClientCaps` with op UPDATE, FLUSH, DROP,
/// RELEASE, RENEW; all are processed alike, as in `Locker::handle_client_caps`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CapUpdate {
    pub op: u32,
    pub ino: Ino,
    pub cap_id: u64,
    pub seq: u32,
    pub issue_seq: u32,
    pub mseq: u32,
    /// Caps the client still holds (its `implemented` set): the revoke ack.
    pub caps: u32,
    pub wanted: u32,
    /// Bits whose metadata is flushed with this message.
    pub dirty: u32,
    /// Header tid identifying the flush (0 when nothing is flushed).
    pub flush_tid: u64,
    pub oldest_flush_tid: u64,
    /// `CEPH_CLIENT_CAPS_SYNC`, `CEPH_CLIENT_CAPS_PENDING_CAPSNAP`.
    pub flags: u32,
    pub size: u64,
    /// The `max_size` the client asks for.
    pub max_size: u64,
    pub mtime: Timestamp,
    pub atime: Timestamp,
    pub ctime: Timestamp,
    pub btime: Timestamp,
    pub time_warp_seq: u32,
    pub change_attr: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    /// The whole xattr blob with its version, when `Xx` is flushed.
    pub xattrs: Option<(u64, XattrMap)>,
}

/// One item of `MClientCapRelease`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapReleaseItem {
    pub ino: Ino,
    pub cap_id: u64,
    pub mseq: u32,
    pub issue_seq: u32,
}
