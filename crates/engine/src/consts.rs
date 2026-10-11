//! Protocol constants the engine reasons about.
//!
//! Values follow `src/include/ceph_fs.h`, `src/include/ceph_hash.h`,
//! `src/mds/mdstypes.h` and `src/include/fs_types.h` of the C++ tree at the
//! commit of the map. They are plain numbers here so that the daemon can copy
//! them to and from the wire unchanged.

// ---------------------------------------------------------------------------
// Capability bits (`CEPH_CAP_*`, ceph_fs.h "capability bits").

/// `CEPH_CAP_PIN`: no specific capability beyond keeping the inode pinned.
pub const CEPH_CAP_PIN: u32 = 1;

/// Generic bits, shifted per lock (`CEPH_CAP_G*`).
pub const CEPH_CAP_GSHARED: u32 = 1;
pub const CEPH_CAP_GEXCL: u32 = 2;
pub const CEPH_CAP_GCACHE: u32 = 4;
pub const CEPH_CAP_GRD: u32 = 8;
pub const CEPH_CAP_GWR: u32 = 16;
pub const CEPH_CAP_GBUFFER: u32 = 32;
pub const CEPH_CAP_GWREXTEND: u32 = 64;
pub const CEPH_CAP_GLAZYIO: u32 = 128;

/// Per-lock shifts (`CEPH_CAP_SAUTH` etc.).
pub const CEPH_CAP_SAUTH: u32 = 2;
pub const CEPH_CAP_SLINK: u32 = 4;
pub const CEPH_CAP_SXATTR: u32 = 6;
pub const CEPH_CAP_SFILE: u32 = 8;

pub const CEPH_CAP_AUTH_SHARED: u32 = CEPH_CAP_GSHARED << CEPH_CAP_SAUTH;
pub const CEPH_CAP_AUTH_EXCL: u32 = CEPH_CAP_GEXCL << CEPH_CAP_SAUTH;
pub const CEPH_CAP_LINK_SHARED: u32 = CEPH_CAP_GSHARED << CEPH_CAP_SLINK;
pub const CEPH_CAP_LINK_EXCL: u32 = CEPH_CAP_GEXCL << CEPH_CAP_SLINK;
pub const CEPH_CAP_XATTR_SHARED: u32 = CEPH_CAP_GSHARED << CEPH_CAP_SXATTR;
pub const CEPH_CAP_XATTR_EXCL: u32 = CEPH_CAP_GEXCL << CEPH_CAP_SXATTR;
pub const CEPH_CAP_FILE_SHARED: u32 = CEPH_CAP_GSHARED << CEPH_CAP_SFILE;
pub const CEPH_CAP_FILE_EXCL: u32 = CEPH_CAP_GEXCL << CEPH_CAP_SFILE;
pub const CEPH_CAP_FILE_CACHE: u32 = CEPH_CAP_GCACHE << CEPH_CAP_SFILE;
pub const CEPH_CAP_FILE_RD: u32 = CEPH_CAP_GRD << CEPH_CAP_SFILE;
pub const CEPH_CAP_FILE_WR: u32 = CEPH_CAP_GWR << CEPH_CAP_SFILE;
pub const CEPH_CAP_FILE_BUFFER: u32 = CEPH_CAP_GBUFFER << CEPH_CAP_SFILE;
pub const CEPH_CAP_FILE_WREXTEND: u32 = CEPH_CAP_GWREXTEND << CEPH_CAP_SFILE;
pub const CEPH_CAP_FILE_LAZYIO: u32 = CEPH_CAP_GLAZYIO << CEPH_CAP_SFILE;

pub const CEPH_CAP_ANY_SHARED: u32 =
    CEPH_CAP_AUTH_SHARED | CEPH_CAP_LINK_SHARED | CEPH_CAP_XATTR_SHARED | CEPH_CAP_FILE_SHARED;
pub const CEPH_CAP_ANY_RD: u32 = CEPH_CAP_ANY_SHARED | CEPH_CAP_FILE_RD | CEPH_CAP_FILE_CACHE;
pub const CEPH_CAP_ANY_EXCL: u32 =
    CEPH_CAP_AUTH_EXCL | CEPH_CAP_LINK_EXCL | CEPH_CAP_XATTR_EXCL | CEPH_CAP_FILE_EXCL;
pub const CEPH_CAP_ANY_FILE_RD: u32 = CEPH_CAP_FILE_RD | CEPH_CAP_FILE_CACHE | CEPH_CAP_FILE_SHARED;
pub const CEPH_CAP_ANY_FILE_WR: u32 = CEPH_CAP_FILE_WR | CEPH_CAP_FILE_BUFFER | CEPH_CAP_FILE_EXCL;
pub const CEPH_CAP_ANY_WR: u32 = CEPH_CAP_ANY_EXCL | CEPH_CAP_ANY_FILE_WR;
pub const CEPH_CAP_ANY: u32 = CEPH_CAP_ANY_RD
    | CEPH_CAP_ANY_EXCL
    | CEPH_CAP_ANY_FILE_WR
    | CEPH_CAP_FILE_LAZYIO
    | CEPH_CAP_PIN;
/// `CEPH_CAP_DIR_CREATE` / `CEPH_CAP_DIR_UNLINK` / `CEPH_CAP_ANY_DIR_OPS`:
/// the async-dirop bits. Never granted on directories in P0.
pub const CEPH_CAP_DIR_CREATE: u32 = CEPH_CAP_FILE_CACHE;
pub const CEPH_CAP_DIR_UNLINK: u32 = CEPH_CAP_FILE_RD;
pub const CEPH_CAP_ANY_DIR_OPS: u32 =
    CEPH_CAP_FILE_CACHE | CEPH_CAP_FILE_RD | CEPH_CAP_FILE_WREXTEND | CEPH_CAP_FILE_LAZYIO;

/// getattr masks (`CEPH_STAT_CAP_*`).
pub const CEPH_STAT_CAP_INODE: u32 = CEPH_CAP_PIN;
pub const CEPH_STAT_CAP_INODE_ALL: u32 = CEPH_CAP_PIN
    | CEPH_CAP_AUTH_SHARED
    | CEPH_CAP_LINK_SHARED
    | CEPH_CAP_FILE_SHARED
    | CEPH_CAP_XATTR_SHARED;
/// `CEPH_STAT_RSTAT`: ask for exact recursive stats (served by the auth).
pub const CEPH_STAT_RSTAT: u32 = CEPH_CAP_FILE_WREXTEND;

/// `CEPH_CAP_FLAG_AUTH`: the reply cap is the auth cap.
pub const CEPH_CAP_FLAG_AUTH: u8 = 1;
/// `CEPH_CAP_FLAG_RELEASE`.
pub const CEPH_CAP_FLAG_RELEASE: u8 = 2;

// Cap message ops (`CEPH_CAP_OP_*`).
pub const CEPH_CAP_OP_GRANT: u32 = 0;
pub const CEPH_CAP_OP_REVOKE: u32 = 1;
pub const CEPH_CAP_OP_TRUNC: u32 = 2;
pub const CEPH_CAP_OP_EXPORT: u32 = 3;
pub const CEPH_CAP_OP_IMPORT: u32 = 4;
pub const CEPH_CAP_OP_UPDATE: u32 = 5;
pub const CEPH_CAP_OP_DROP: u32 = 6;
pub const CEPH_CAP_OP_FLUSH: u32 = 7;
pub const CEPH_CAP_OP_FLUSH_ACK: u32 = 8;
pub const CEPH_CAP_OP_FLUSHSNAP: u32 = 9;
pub const CEPH_CAP_OP_FLUSHSNAP_ACK: u32 = 10;
pub const CEPH_CAP_OP_RELEASE: u32 = 11;
pub const CEPH_CAP_OP_RENEW: u32 = 12;

/// `MClientCaps` flags (`CEPH_CLIENT_CAPS_*`, `MClientCaps.h`).
pub const CEPH_CLIENT_CAPS_SYNC: u32 = 1;
pub const CEPH_CLIENT_CAPS_NO_CAPSNAP: u32 = 2;
pub const CEPH_CLIENT_CAPS_PENDING_CAPSNAP: u32 = 4;

// ---------------------------------------------------------------------------
// File modes for open (`CEPH_FILE_MODE_*`).
pub const CEPH_FILE_MODE_PIN: u32 = 0;
pub const CEPH_FILE_MODE_RD: u32 = 1;
pub const CEPH_FILE_MODE_WR: u32 = 2;
pub const CEPH_FILE_MODE_RDWR: u32 = 3;
pub const CEPH_FILE_MODE_LAZY: u32 = 4;

// Open flags on the wire (`CEPH_O_*`, Linux numeric values).
pub const CEPH_O_RDONLY: u32 = 0o0;
pub const CEPH_O_WRONLY: u32 = 0o1;
pub const CEPH_O_RDWR: u32 = 0o2;
pub const CEPH_O_ACCMODE: u32 = 0o3;
pub const CEPH_O_CREAT: u32 = 0o100;
pub const CEPH_O_EXCL: u32 = 0o200;
pub const CEPH_O_TRUNC: u32 = 0o1000;
pub const CEPH_O_LAZY: u32 = 0o20000;
pub const CEPH_O_DIRECTORY: u32 = 0o200000;
pub const CEPH_O_NOFOLLOW: u32 = 0o400000;

// ---------------------------------------------------------------------------
// MDS request ops (`CEPH_MDS_OP_*`).
pub const CEPH_MDS_OP_WRITE: u32 = 0x001000;
pub const CEPH_MDS_OP_LOOKUP: u32 = 0x00100;
pub const CEPH_MDS_OP_GETATTR: u32 = 0x00101;
pub const CEPH_MDS_OP_LOOKUPHASH: u32 = 0x00102;
pub const CEPH_MDS_OP_LOOKUPPARENT: u32 = 0x00103;
pub const CEPH_MDS_OP_LOOKUPINO: u32 = 0x00104;
pub const CEPH_MDS_OP_LOOKUPNAME: u32 = 0x00105;
pub const CEPH_MDS_OP_GETVXATTR: u32 = 0x00106;
pub const CEPH_MDS_OP_SETXATTR: u32 = 0x01105;
pub const CEPH_MDS_OP_RMXATTR: u32 = 0x01106;
pub const CEPH_MDS_OP_SETLAYOUT: u32 = 0x01107;
pub const CEPH_MDS_OP_SETATTR: u32 = 0x01108;
pub const CEPH_MDS_OP_SETFILELOCK: u32 = 0x01109;
pub const CEPH_MDS_OP_GETFILELOCK: u32 = 0x00110;
pub const CEPH_MDS_OP_SETDIRLAYOUT: u32 = 0x0110a;
pub const CEPH_MDS_OP_MKNOD: u32 = 0x01201;
pub const CEPH_MDS_OP_LINK: u32 = 0x01202;
pub const CEPH_MDS_OP_UNLINK: u32 = 0x01203;
pub const CEPH_MDS_OP_RENAME: u32 = 0x01204;
pub const CEPH_MDS_OP_MKDIR: u32 = 0x01220;
pub const CEPH_MDS_OP_RMDIR: u32 = 0x01221;
pub const CEPH_MDS_OP_SYMLINK: u32 = 0x01222;
pub const CEPH_MDS_OP_CREATE: u32 = 0x01301;
pub const CEPH_MDS_OP_OPEN: u32 = 0x00302;
pub const CEPH_MDS_OP_READDIR: u32 = 0x00305;
pub const CEPH_MDS_OP_LOOKUPSNAP: u32 = 0x00400;
pub const CEPH_MDS_OP_MKSNAP: u32 = 0x01400;
pub const CEPH_MDS_OP_RMSNAP: u32 = 0x01401;
pub const CEPH_MDS_OP_LSSNAP: u32 = 0x00402;
pub const CEPH_MDS_OP_RENAMESNAP: u32 = 0x01403;

/// Request head flags (`CEPH_MDS_FLAG_*`).
pub const CEPH_MDS_FLAG_REPLAY: u32 = 1;
pub const CEPH_MDS_FLAG_WANT_DENTRY: u32 = 2;
pub const CEPH_MDS_FLAG_ASYNC: u32 = 4;

// setattr mask bits (`CEPH_SETATTR_*`).
pub const CEPH_SETATTR_MODE: u32 = 1 << 0;
pub const CEPH_SETATTR_UID: u32 = 1 << 1;
pub const CEPH_SETATTR_GID: u32 = 1 << 2;
pub const CEPH_SETATTR_MTIME: u32 = 1 << 3;
pub const CEPH_SETATTR_ATIME: u32 = 1 << 4;
pub const CEPH_SETATTR_SIZE: u32 = 1 << 5;
pub const CEPH_SETATTR_CTIME: u32 = 1 << 6;
pub const CEPH_SETATTR_MTIME_NOW: u32 = 1 << 7;
pub const CEPH_SETATTR_ATIME_NOW: u32 = 1 << 8;
pub const CEPH_SETATTR_BTIME: u32 = 1 << 9;
pub const CEPH_SETATTR_KILL_SGUID: u32 = 1 << 10;
pub const CEPH_SETATTR_FSCRYPT_AUTH: u32 = 1 << 11;
pub const CEPH_SETATTR_FSCRYPT_FILE: u32 = 1 << 12;
pub const CEPH_SETATTR_KILL_SUID: u32 = 1 << 13;
pub const CEPH_SETATTR_KILL_SGID: u32 = 1 << 14;

// setxattr flags (`CEPH_XATTR_*`).
pub const CEPH_XATTR_CREATE: u32 = 1 << 0;
pub const CEPH_XATTR_REPLACE: u32 = 1 << 1;
pub const CEPH_XATTR_REMOVE2: u32 = 1 << 30;
pub const CEPH_XATTR_REMOVE: u32 = 1 << 31;

// readdir request/reply flags.
pub const CEPH_READDIR_REPLY_BITFLAGS: u32 = 1 << 0;
pub const CEPH_READDIR_FRAG_END: u16 = 1 << 0;
pub const CEPH_READDIR_FRAG_COMPLETE: u16 = 1 << 8;
pub const CEPH_READDIR_HASH_ORDER: u16 = 1 << 9;
pub const CEPH_READDIR_OFFSET_HASH: u16 = 1 << 10;

// File lock rules and types (`CEPH_LOCK_*`).
pub const CEPH_LOCK_FCNTL: u8 = 1;
pub const CEPH_LOCK_FLOCK: u8 = 2;
pub const CEPH_LOCK_FCNTL_INTR: u8 = 3;
pub const CEPH_LOCK_FLOCK_INTR: u8 = 4;
pub const CEPH_LOCK_SHARED: u8 = 1;
pub const CEPH_LOCK_EXCL: u8 = 2;
pub const CEPH_LOCK_UNLOCK: u8 = 4;

// Leases.
pub const CEPH_LEASE_VALID: u16 = 1 | 2;
pub const CEPH_LEASE_PRIMARY_LINK: u16 = 4;

/// `CEPH_INLINE_NONE`: no inline data (`inline_version` must never be 0).
pub const CEPH_INLINE_NONE: u64 = u64::MAX;
/// `CEPH_NOSNAP`: the head (live) version of an inode.
pub const CEPH_NOSNAP: u64 = u64::MAX - 1;
/// `CEPH_MIN_STRIPE_UNIT`: layouts use multiples of 64 KiB.
pub const CEPH_MIN_STRIPE_UNIT: u32 = 65536;

// Directory hash types (`ceph_hash.h`).
pub const CEPH_STR_HASH_LINUX: u8 = 1;
pub const CEPH_STR_HASH_RJENKINS: u8 = 2;

// Special inode numbers (`ceph_fs.h`, `mdstypes.h`).
pub const CEPH_INO_ROOT: u64 = 1;
pub const CEPH_INO_CEPH: u64 = 2;
pub const CEPH_INO_GLOBAL_SNAPREALM: u64 = 3;
pub const CEPH_INO_LOST_AND_FOUND: u64 = 4;
/// `MDS_INO_MDSDIR_OFFSET`: first MDS-private inode number.
pub const MDS_INO_MDSDIR_OFFSET: u64 = 0x100;
/// `MDS_INO_SYSTEM_BASE`: first inode number after the MDS-private range.
pub const MDS_INO_SYSTEM_BASE: u64 = 0x1000;
/// ceph-fuse needs inode numbers below 2^48.
pub const INO_LIMIT: u64 = 1 << 48;
/// Start of the rank-0 inode range of the C++ `InoTable` (`(rank+1) << 40`).
pub const RANK0_INO_START: u64 = 1 << 40;
/// Length of the rank-0 inode range (`1 << 40`).
pub const RANK0_INO_LEN: u64 = 1 << 40;

/// `MDS_RANK_NONE`: no export pin.
pub const MDS_RANK_NONE: i32 = -1;
/// `MAX_MDS`: pins at or above this value are refused with `EDOM`.
pub const MAX_MDS: i32 = 0x100;

/// Default `max_xattr_size` of the MDSMap (`MDS_MAX_XATTR_SIZE`).
pub const MDS_MAX_XATTR_SIZE: u64 = 1 << 16;

// POSIX file types and mode bits.
pub const S_IFMT: u32 = 0o170000;
pub const S_IFSOCK: u32 = 0o140000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFBLK: u32 = 0o060000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFIFO: u32 = 0o010000;
pub const S_ISUID: u32 = 0o4000;
pub const S_ISGID: u32 = 0o2000;
pub const S_ISVTX: u32 = 0o1000;
pub const S_IXUSR: u32 = 0o100;
pub const S_IXGRP: u32 = 0o010;
pub const S_IXOTH: u32 = 0o001;
