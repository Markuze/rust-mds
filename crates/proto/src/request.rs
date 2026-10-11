//! `MClientRequest` (`CEPH_MSG_CLIENT_REQUEST`, `src/messages/MClientRequest.h`), the
//! request head (`ceph_mds_request_head`, codec in `src/common/ceph_fs.cc`), the per-op argument
//! union and the `CEPH_MDS_OP_*` table.
//!
//! Front of message version 4-6 (version 6 is what current clients send):
//!
//! | # | field | since |
//! |---|---|---|
//! | 1 | versioned request head ([`RequestHead`]) | 4 |
//! | 2 | `filepath path` | 1 |
//! | 3 | `filepath path2` | 1 |
//! | 4 | `head.num_releases` x ([`RequestRelease`]: raw 44-byte record + `dname_len` bytes) | 1 |
//! | 5 | `utime stamp` (client time of the operation) | 2 |
//! | 6 | `vector<u64> gid_list` | 4 |
//! | 7 | `string alternate_name` | 5 |
//! | 8 | `vector<u8> fscrypt_auth`, `vector<u8> fscrypt_file` | 6 |
//!
//! Trailing bytes are ignored. The data segment holds an op-specific blob ([`MClientRequest::data`]).
//! The `header.tid` is the request tid. Message versions below 4 (the 88-byte legacy head) are
//! refused (legacy, catalog F-PROTO-066).

use std::fmt;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Message, Payload, Result, UTime};

use crate::msg_type;
use crate::types::{FileLayout, XattrMap, decode_xattr_blob, open_flags};
use crate::util::{Ctx, get, impl_from_raw, invalid};

/// `CEPH_MDS_OP_*` (`include/ceph_fs.h`). Ops with [`MdsOp::WRITE_BIT`] modify metadata.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MdsOp(pub u32);

impl MdsOp {
    /// `CEPH_MDS_OP_WRITE`.
    pub const WRITE_BIT: u32 = 0x1000;

    pub const LOOKUP: MdsOp = MdsOp(0x00100);
    pub const GETATTR: MdsOp = MdsOp(0x00101);
    pub const LOOKUPHASH: MdsOp = MdsOp(0x00102);
    pub const LOOKUPPARENT: MdsOp = MdsOp(0x00103);
    pub const LOOKUPINO: MdsOp = MdsOp(0x00104);
    pub const LOOKUPNAME: MdsOp = MdsOp(0x00105);
    pub const GETVXATTR: MdsOp = MdsOp(0x00106);
    pub const DUMMY: MdsOp = MdsOp(0x00107);
    pub const SETXATTR: MdsOp = MdsOp(0x01105);
    pub const RMXATTR: MdsOp = MdsOp(0x01106);
    pub const SETLAYOUT: MdsOp = MdsOp(0x01107);
    pub const SETATTR: MdsOp = MdsOp(0x01108);
    pub const SETFILELOCK: MdsOp = MdsOp(0x01109);
    pub const GETFILELOCK: MdsOp = MdsOp(0x00110);
    pub const SETDIRLAYOUT: MdsOp = MdsOp(0x0110a);
    pub const MKNOD: MdsOp = MdsOp(0x01201);
    pub const LINK: MdsOp = MdsOp(0x01202);
    pub const UNLINK: MdsOp = MdsOp(0x01203);
    pub const RENAME: MdsOp = MdsOp(0x01204);
    pub const MKDIR: MdsOp = MdsOp(0x01220);
    pub const RMDIR: MdsOp = MdsOp(0x01221);
    pub const SYMLINK: MdsOp = MdsOp(0x01222);
    pub const CREATE: MdsOp = MdsOp(0x01301);
    pub const OPEN: MdsOp = MdsOp(0x00302);
    pub const READDIR: MdsOp = MdsOp(0x00305);
    pub const LOOKUPSNAP: MdsOp = MdsOp(0x00400);
    pub const MKSNAP: MdsOp = MdsOp(0x01400);
    pub const RMSNAP: MdsOp = MdsOp(0x01401);
    pub const LSSNAP: MdsOp = MdsOp(0x00402);
    pub const RENAMESNAP: MdsOp = MdsOp(0x01403);
    pub const READDIR_SNAPDIFF: MdsOp = MdsOp(0x01404);
    pub const FILE_BLOCKDIFF: MdsOp = MdsOp(0x01405);
    pub const SNAP_METADATA: MdsOp = MdsOp(0x01406);
    // Internal ops: never valid from a client (answered with -EOPNOTSUPP).
    pub const FRAGMENTDIR: MdsOp = MdsOp(0x01500);
    pub const EXPORTDIR: MdsOp = MdsOp(0x01501);
    pub const FLUSH: MdsOp = MdsOp(0x01502);
    pub const ENQUEUE_SCRUB: MdsOp = MdsOp(0x01503);
    pub const REPAIR_FRAGSTATS: MdsOp = MdsOp(0x01504);
    pub const REPAIR_INODESTATS: MdsOp = MdsOp(0x01505);
    pub const RDLOCK_FRAGSSTATS: MdsOp = MdsOp(0x01507);
    pub const QUIESCE_PATH: MdsOp = MdsOp(0x01508);
    pub const QUIESCE_INODE: MdsOp = MdsOp(0x01509);
    pub const LOCK_PATH: MdsOp = MdsOp(0x0150a);
    pub const UNINLINE_DATA: MdsOp = MdsOp(0x0150b);
    pub const QUARANTINEDIR_AUTH: MdsOp = MdsOp(0x0150c);
    pub const QUARANTINE_INODE: MdsOp = MdsOp(0x0150e);
    pub const QUARANTINEDIR_WORK: MdsOp = MdsOp(0x0150f);

    /// Every op a client may legitimately send, in `ceph_fs.h` order.
    pub const CLIENT_OPS: [MdsOp; 32] = [
        Self::LOOKUP,
        Self::GETATTR,
        Self::LOOKUPHASH,
        Self::LOOKUPPARENT,
        Self::LOOKUPINO,
        Self::LOOKUPNAME,
        Self::GETVXATTR,
        Self::DUMMY,
        Self::SETXATTR,
        Self::RMXATTR,
        Self::SETLAYOUT,
        Self::SETATTR,
        Self::SETFILELOCK,
        Self::GETFILELOCK,
        Self::SETDIRLAYOUT,
        Self::MKNOD,
        Self::LINK,
        Self::UNLINK,
        Self::RENAME,
        Self::MKDIR,
        Self::RMDIR,
        Self::SYMLINK,
        Self::CREATE,
        Self::OPEN,
        Self::READDIR,
        Self::LOOKUPSNAP,
        Self::MKSNAP,
        Self::RMSNAP,
        Self::LSSNAP,
        Self::RENAMESNAP,
        Self::READDIR_SNAPDIFF,
        Self::FILE_BLOCKDIFF,
    ];

    /// `ceph_mds_op_name` (`src/common/ceph_strings.cc`).
    pub fn name(self) -> &'static str {
        match self {
            Self::LOOKUP => "lookup",
            Self::LOOKUPHASH => "lookuphash",
            Self::LOOKUPPARENT => "lookupparent",
            Self::LOOKUPINO => "lookupino",
            Self::LOOKUPNAME => "lookupname",
            Self::GETATTR => "getattr",
            Self::DUMMY => "dummy",
            Self::GETVXATTR => "getvxattr",
            Self::SETXATTR => "setxattr",
            Self::SETATTR => "setattr",
            Self::RMXATTR => "rmxattr",
            Self::SETLAYOUT => "setlayou",
            Self::SETDIRLAYOUT => "setdirlayout",
            Self::READDIR => "readdir",
            Self::MKNOD => "mknod",
            Self::LINK => "link",
            Self::UNLINK => "unlink",
            Self::RENAME => "rename",
            Self::MKDIR => "mkdir",
            Self::RMDIR => "rmdir",
            Self::SYMLINK => "symlink",
            Self::CREATE => "create",
            Self::OPEN => "open",
            Self::LOOKUPSNAP => "lookupsnap",
            Self::LSSNAP => "lssnap",
            Self::MKSNAP => "mksnap",
            Self::RMSNAP => "rmsnap",
            Self::RENAMESNAP => "renamesnap",
            Self::SNAP_METADATA => "snap_md_op",
            Self::READDIR_SNAPDIFF => "readdir_snapdiff",
            Self::SETFILELOCK => "setfilelock",
            Self::GETFILELOCK => "getfilelock",
            Self::FRAGMENTDIR => "fragmentdir",
            Self::EXPORTDIR => "exportdir",
            Self::FLUSH => "flush_path",
            Self::ENQUEUE_SCRUB => "enqueue_scrub",
            Self::REPAIR_FRAGSTATS => "repair_fragstats",
            Self::REPAIR_INODESTATS => "repair_inodestats",
            Self::QUIESCE_PATH => "quiesce_path",
            Self::QUIESCE_INODE => "quiesce_inode",
            Self::LOCK_PATH => "lock_path",
            Self::UNINLINE_DATA => "uninline_data",
            Self::FILE_BLOCKDIFF => "blockdiff",
            _ => "???",
        }
    }

    /// The op has the `CEPH_MDS_OP_WRITE` bit (READDIR_SNAPDIFF and FILE_BLOCKDIFF have it too).
    pub const fn is_write(self) -> bool {
        self.0 & Self::WRITE_BIT != 0
    }

    /// `IS_CEPH_MDS_OP_NEWINODE`: the ops that create an inode and consume `head.ino` and
    /// `owner_uid/gid`.
    pub const fn is_new_inode(self) -> bool {
        matches!(self, Self::CREATE | Self::MKNOD | Self::MKDIR | Self::SYMLINK)
    }

    /// Internal ops (`0x1500`..): not valid from clients.
    pub const fn is_internal(self) -> bool {
        self.0 >= 0x01500 && self.0 < 0x01600
    }
}

impl fmt::Debug for MdsOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:#x})", self.name(), self.0)
    }
}

/// `CEPH_MDS_FLAG_*` request flags.
pub mod request_flags {
    /// A resend of a request that already got an unsafe reply.
    pub const REPLAY: u32 = 1;
    /// The client wants the dentry trace in the reply (also the null dentry on `-ENOENT`).
    pub const WANT_DENTRY: u32 = 2;
    /// Async create/unlink (only with granted `Fx|Dc`/`Fx|Dr`, which P0 never grants).
    pub const ASYNC: u32 = 4;
}

/// `union ceph_mds_request_args`: 56 raw bytes whose meaning depends on the op. Read them
/// through the typed views (`getattr_mask`, [`SetattrArgs`], [`ReaddirArgs`], ...).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestArgs(pub [u8; RequestArgs::LEN]);

impl Default for RequestArgs {
    fn default() -> Self {
        RequestArgs([0; Self::LEN])
    }
}

impl fmt::Debug for RequestArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RequestArgs({})", hex(&self.0))
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl RequestArgs {
    pub const LEN: usize = 56;

    #[inline]
    fn u8_at(&self, off: usize) -> u8 {
        self.0[off]
    }
    #[inline]
    fn u16_at(&self, off: usize) -> u16 {
        u16::from_le_bytes([self.0[off], self.0[off + 1]])
    }
    #[inline]
    fn u32_at(&self, off: usize) -> u32 {
        u32::from_le_bytes(self.0[off..off + 4].try_into().unwrap())
    }
    #[inline]
    fn u64_at(&self, off: usize) -> u64 {
        u64::from_le_bytes(self.0[off..off + 8].try_into().unwrap())
    }
    #[inline]
    fn time_at(&self, off: usize) -> UTime {
        UTime { sec: self.u32_at(off), nsec: self.u32_at(off + 4) }
    }
    #[inline]
    fn put_u8(&mut self, off: usize, v: u8) {
        self.0[off] = v;
    }
    #[inline]
    fn put_u16(&mut self, off: usize, v: u16) {
        self.0[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    #[inline]
    fn put_u32(&mut self, off: usize, v: u32) {
        self.0[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    #[inline]
    fn put_u64(&mut self, off: usize, v: u64) {
        self.0[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }
    #[inline]
    fn put_time(&mut self, off: usize, t: UTime) {
        self.put_u32(off, t.sec);
        self.put_u32(off + 4, t.nsec);
    }

    /// `args.getattr.mask` (GETATTR, LOOKUP, LOOKUPSNAP): `CEPH_CAP_*` / `CEPH_STAT_*` bits.
    pub fn getattr_mask(&self) -> u32 {
        self.u32_at(0)
    }
    pub fn with_getattr_mask(mask: u32) -> Self {
        let mut a = Self::default();
        a.put_u32(0, mask);
        a
    }
    /// `args.mkdir.mode` (MKDIR, MKSNAP).
    pub fn mkdir_mode(&self) -> u32 {
        self.u32_at(0)
    }
    pub fn with_mkdir_mode(mode: u32) -> Self {
        let mut a = Self::default();
        a.put_u32(0, mode);
        a
    }
    pub fn setattr(&self) -> SetattrArgs {
        SetattrArgs::from_args(self)
    }
    pub fn readdir(&self) -> ReaddirArgs {
        ReaddirArgs::from_args(self)
    }
    pub fn mknod(&self) -> MknodArgs {
        MknodArgs::from_args(self)
    }
    pub fn open(&self) -> OpenArgs {
        OpenArgs::from_args(self)
    }
    pub fn setxattr(&self) -> SetxattrArgs {
        SetxattrArgs::from_args(self)
    }
    pub fn filelock(&self) -> FilelockArgs {
        FilelockArgs::from_args(self)
    }
    pub fn lookupino(&self) -> LookupinoArgs {
        LookupinoArgs::from_args(self)
    }
    pub fn snapdiff(&self) -> SnapdiffArgs {
        SnapdiffArgs::from_args(self)
    }
    pub fn blockdiff(&self) -> BlockdiffArgs {
        BlockdiffArgs::from_args(self)
    }
    /// `args.setlayout.layout` (SETLAYOUT, SETDIRLAYOUT): a legacy 28-byte `ceph_file_layout`.
    pub fn setlayout(&self) -> FileLayout {
        let mut b = Bytes::copy_from_slice(&self.0[..FileLayout::LEGACY_LEN]);
        FileLayout::decode_legacy(&mut b).expect("28 bytes are present")
    }
    pub fn with_setlayout(layout: &FileLayout) -> Self {
        let mut b = BytesMut::new();
        layout.encode_legacy(&mut b);
        let mut a = Self::default();
        a.0[..FileLayout::LEGACY_LEN].copy_from_slice(&b);
        a
    }
}

/// `args.setattr` (SETATTR). Offsets: mode 0, uid 4, gid 8, mtime 12, atime 20, size 28,
/// old_size 36, mask 44, btime 48.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SetattrArgs {
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub mtime: UTime,
    pub atime: UTime,
    pub size: u64,
    pub old_size: u64,
    /// [`crate::types::setattr`] bits saying which fields apply.
    pub mask: u32,
    pub btime: UTime,
}

impl SetattrArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        SetattrArgs {
            mode: a.u32_at(0),
            uid: a.u32_at(4),
            gid: a.u32_at(8),
            mtime: a.time_at(12),
            atime: a.time_at(20),
            size: a.u64_at(28),
            old_size: a.u64_at(36),
            mask: a.u32_at(44),
            btime: a.time_at(48),
        }
    }
    pub fn to_args(&self) -> RequestArgs {
        let mut a = RequestArgs::default();
        a.put_u32(0, self.mode);
        a.put_u32(4, self.uid);
        a.put_u32(8, self.gid);
        a.put_time(12, self.mtime);
        a.put_time(20, self.atime);
        a.put_u64(28, self.size);
        a.put_u64(36, self.old_size);
        a.put_u32(44, self.mask);
        a.put_time(48, self.btime);
        a
    }
}

/// `args.readdir` (READDIR, LSSNAP). Offsets: frag 0, max_entries 4, max_bytes 8, flags 12
/// (u16), offset_hash 14.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ReaddirArgs {
    /// The frag the client asks for (a `frag_t`).
    pub frag: u32,
    /// 0 = no entry limit; the kernel's reply buffer holds at most this many entries.
    pub max_entries: u32,
    /// 0 = MDS default (512 KiB + max xattr size).
    pub max_bytes: u32,
    /// [`crate::types::readdir_flags::REPLY_BITFLAGS`].
    pub flags: u16,
    /// Hash-order resume point when `path2` (the last name) is empty.
    pub offset_hash: u32,
}

impl ReaddirArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        ReaddirArgs {
            frag: a.u32_at(0),
            max_entries: a.u32_at(4),
            max_bytes: a.u32_at(8),
            flags: a.u16_at(12),
            offset_hash: a.u32_at(14),
        }
    }
    pub fn to_args(&self) -> RequestArgs {
        let mut a = RequestArgs::default();
        a.put_u32(0, self.frag);
        a.put_u32(4, self.max_entries);
        a.put_u32(8, self.max_bytes);
        a.put_u16(12, self.flags);
        a.put_u32(14, self.offset_hash);
        a
    }
}

/// `args.mknod` (MKNOD). Offsets: mode 0, rdev 4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MknodArgs {
    pub mode: u32,
    pub rdev: u32,
}

impl MknodArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        MknodArgs { mode: a.u32_at(0), rdev: a.u32_at(4) }
    }
    pub fn to_args(&self) -> RequestArgs {
        let mut a = RequestArgs::default();
        a.put_u32(0, self.mode);
        a.put_u32(4, self.rdev);
        a
    }
}

/// `args.open` (OPEN, CREATE). Offsets: flags 0, mode 4, stripe_unit 8, stripe_count 12,
/// object_size 16, pool 20, mask 24, old_size 28.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct OpenArgs {
    /// `CEPH_O_*` ([`crate::types::open_flags`]).
    pub flags: u32,
    pub mode: u32,
    /// Layout overrides for a new file (0 = inherit).
    pub stripe_unit: u32,
    pub stripe_count: u32,
    pub object_size: u32,
    /// Pool for a new file if >= 0 (honoured only with `CEPH_FEATURE_CREATEPOOLID`).
    pub pool: i32,
    pub mask: u32,
    pub old_size: u64,
}

impl OpenArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        OpenArgs {
            flags: a.u32_at(0),
            mode: a.u32_at(4),
            stripe_unit: a.u32_at(8),
            stripe_count: a.u32_at(12),
            object_size: a.u32_at(16),
            pool: a.u32_at(20) as i32,
            mask: a.u32_at(24),
            old_size: a.u64_at(28),
        }
    }
    pub fn to_args(&self) -> RequestArgs {
        let mut a = RequestArgs::default();
        a.put_u32(0, self.flags);
        a.put_u32(4, self.mode);
        a.put_u32(8, self.stripe_unit);
        a.put_u32(12, self.stripe_count);
        a.put_u32(16, self.object_size);
        a.put_u32(20, self.pool as u32);
        a.put_u32(24, self.mask);
        a.put_u64(28, self.old_size);
        a
    }
}

/// `args.setxattr` (SETXATTR). Offsets: flags 0, osdmap_epoch 4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SetxattrArgs {
    /// [`crate::types::xattr_flags`].
    pub flags: u32,
    /// The client's OSDMap epoch, for layout-pool validation.
    pub osdmap_epoch: u32,
}

impl SetxattrArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        SetxattrArgs { flags: a.u32_at(0), osdmap_epoch: a.u32_at(4) }
    }
    pub fn to_args(&self) -> RequestArgs {
        let mut a = RequestArgs::default();
        a.put_u32(0, self.flags);
        a.put_u32(4, self.osdmap_epoch);
        a
    }
}

/// `args.filelock_change` (SETFILELOCK, GETFILELOCK). Offsets: rule 0 (u8), type 1 (u8),
/// owner 2, pid 10, start 18, length 26, wait 34 (u8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FilelockArgs {
    /// [`crate::types::filelock`] `RULE_*`.
    pub rule: u8,
    /// [`crate::types::filelock`] `SHARED`/`EXCL`/`UNLOCK`.
    pub lock_type: u8,
    pub owner: u64,
    pub pid: u64,
    pub start: u64,
    /// 0 = to the end of the file.
    pub length: u64,
    /// Blocking request.
    pub wait: u8,
}

impl FilelockArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        FilelockArgs {
            rule: a.u8_at(0),
            lock_type: a.u8_at(1),
            owner: a.u64_at(2),
            pid: a.u64_at(10),
            start: a.u64_at(18),
            length: a.u64_at(26),
            wait: a.u8_at(34),
        }
    }
    pub fn to_args(&self) -> RequestArgs {
        let mut a = RequestArgs::default();
        a.put_u8(0, self.rule);
        a.put_u8(1, self.lock_type);
        a.put_u64(2, self.owner);
        a.put_u64(10, self.pid);
        a.put_u64(18, self.start);
        a.put_u64(26, self.length);
        a.put_u8(34, self.wait);
        a
    }
}

/// `args.lookupino` (LOOKUPINO, LOOKUPHASH, LOOKUPPARENT, LOOKUPNAME). Offsets: mask 0,
/// snapid 4, parent 12, hash 20.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LookupinoArgs {
    pub mask: u32,
    /// Non-zero: look up a snapped inode.
    pub snapid: u64,
    pub parent: u64,
    pub hash: u32,
}

impl LookupinoArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        LookupinoArgs { mask: a.u32_at(0), snapid: a.u64_at(4), parent: a.u64_at(12), hash: a.u32_at(20) }
    }
    pub fn to_args(&self) -> RequestArgs {
        let mut a = RequestArgs::default();
        a.put_u32(0, self.mask);
        a.put_u64(4, self.snapid);
        a.put_u64(12, self.parent);
        a.put_u32(20, self.hash);
        a
    }
}

/// `args.snapdiff` (READDIR_SNAPDIFF, P1). Offsets: frag 0, max_entries 4, max_bytes 8,
/// flags 12 (u16), offset_hash 14, snap_other 18, mask 26.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SnapdiffArgs {
    pub frag: u32,
    pub max_entries: u32,
    pub max_bytes: u32,
    pub flags: u16,
    pub offset_hash: u32,
    pub snap_other: u64,
    pub mask: u32,
}

impl SnapdiffArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        SnapdiffArgs {
            frag: a.u32_at(0),
            max_entries: a.u32_at(4),
            max_bytes: a.u32_at(8),
            flags: a.u16_at(12),
            offset_hash: a.u32_at(14),
            snap_other: a.u64_at(18),
            mask: a.u32_at(26),
        }
    }
}

/// `args.blockdiff` (FILE_BLOCKDIFF, P1). Offsets: scan_idx 0, max_objects 8.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockdiffArgs {
    pub scan_idx: u64,
    pub max_objects: u64,
}

impl BlockdiffArgs {
    pub fn from_args(a: &RequestArgs) -> Self {
        BlockdiffArgs { scan_idx: a.u64_at(0), max_objects: a.u64_at(8) }
    }
}

/// `ceph_mds_request_head` (`include/ceph_fs.h`, codec `src/common/ceph_fs.cc`).
///
/// Wire layout: `u16 version`, `u64 oldest_client_tid`, `u32 mdsmap_epoch`, `u32 flags`,
/// `u8 num_retry`, `u8 num_fwd` (legacy 8-bit counters), `u16 num_releases`, `u32 op`,
/// `u32 caller_uid`, `u32 caller_gid`, `u64 ino`, 56 bytes of args (98 bytes so far, v1);
/// v2: `u32 ext_num_retry`, `u32 ext_num_fwd` (106); v3: `u32 struct_len` (total head size,
/// 118), `u32 owner_uid`, `u32 owner_gid`. A v3+ decoder skips head bytes beyond what it knows
/// up to `struct_len` and refuses a `struct_len` shorter than what it read.
///
/// For heads below v3 the decoder applies the C++ fallbacks: the retry/forward counts come
/// from the 8-bit fields and the owner is the caller. Current clients send v3 to an MDS that
/// advertises bits 18 and 20; heads below v3 are legacy, so a server may refuse
/// `version < 3`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestHead {
    pub version: u16,
    /// Lowest tid the client still waits for (drives completed-request trimming).
    pub oldest_client_tid: u64,
    /// The client's MDSMap epoch (unused by the MDS).
    pub mdsmap_epoch: u32,
    /// [`request_flags`].
    pub flags: u32,
    /// Attempt count minus one (32-bit `ext_num_retry`).
    pub num_retry: u32,
    /// Forward count (32-bit `ext_num_fwd`).
    pub num_fwd: u32,
    pub op: MdsOp,
    pub caller_uid: u32,
    pub caller_gid: u32,
    /// Inode number for a new inode (replay of an unsafe create, or a delegated ino).
    pub ino: u64,
    pub args: RequestArgs,
    /// Owner of a new inode (new-inode ops); `u32::MAX` otherwise.
    pub owner_uid: u32,
    pub owner_gid: u32,
}

impl Default for RequestHead {
    fn default() -> Self {
        RequestHead {
            version: Self::VERSION,
            oldest_client_tid: 0,
            mdsmap_epoch: 0,
            flags: 0,
            num_retry: 0,
            num_fwd: 0,
            op: MdsOp(0),
            caller_uid: 0,
            caller_gid: 0,
            ino: 0,
            args: RequestArgs::default(),
            owner_uid: u32::MAX,
            owner_gid: u32::MAX,
        }
    }
}

impl RequestHead {
    /// `CEPH_MDS_REQUEST_HEAD_VERSION`.
    pub const VERSION: u16 = 3;
    /// `sizeof(struct ceph_mds_request_head)` at v3, the `struct_len` written by encoders.
    pub const V3_LEN: u32 = 118;

    pub fn is_replay(&self) -> bool {
        self.flags & request_flags::REPLAY != 0
    }
    pub fn is_async(&self) -> bool {
        self.flags & request_flags::ASYNC != 0
    }
    pub fn wants_dentry(&self) -> bool {
        self.flags & request_flags::WANT_DENTRY != 0
    }

    /// Encode with `num_releases` releases (C++ `encode(ceph_mds_request_head)`), at
    /// `self.version` (1, 2 or 3).
    pub fn encode_with(&self, buf: &mut BytesMut, num_releases: u16) {
        buf.put_u16_le(self.version);
        buf.put_u64_le(self.oldest_client_tid);
        buf.put_u32_le(self.mdsmap_epoch);
        buf.put_u32_le(self.flags);
        buf.put_u8(self.num_retry as u8);
        buf.put_u8(self.num_fwd as u8);
        buf.put_u16_le(num_releases);
        buf.put_u32_le(self.op.0);
        buf.put_u32_le(self.caller_uid);
        buf.put_u32_le(self.caller_gid);
        buf.put_u64_le(self.ino);
        buf.put_slice(&self.args.0);
        if self.version >= 2 {
            buf.put_u32_le(self.num_retry);
            buf.put_u32_le(self.num_fwd);
        }
        if self.version >= 3 {
            buf.put_u32_le(Self::V3_LEN);
            buf.put_u32_le(self.owner_uid);
            buf.put_u32_le(self.owner_gid);
        }
    }

    /// Decode a versioned head; returns the head and `num_releases`.
    pub fn decode_with_releases(buf: &mut Bytes) -> Result<(Self, u16)> {
        let start_len = buf.remaining();
        denc::need(buf, 98, "ceph_mds_request_head")?;
        let version = buf.get_u16_le();
        let oldest_client_tid = buf.get_u64_le();
        let mdsmap_epoch = buf.get_u32_le();
        let flags = buf.get_u32_le();
        let num_retry8 = buf.get_u8();
        let num_fwd8 = buf.get_u8();
        let num_releases = buf.get_u16_le();
        let op = MdsOp(buf.get_u32_le());
        let caller_uid = buf.get_u32_le();
        let caller_gid = buf.get_u32_le();
        let ino = buf.get_u64_le();
        let mut args = RequestArgs::default();
        buf.copy_to_slice(&mut args.0);
        let (num_retry, num_fwd) = if version >= 2 {
            (get::<u32>(buf, "request_head.ext_num_retry")?, get::<u32>(buf, "request_head.ext_num_fwd")?)
        } else {
            (num_retry8 as u32, num_fwd8 as u32)
        };
        let (owner_uid, owner_gid) = if version >= 3 {
            let struct_len = get::<u32>(buf, "request_head.struct_len")? as usize;
            let owner_uid = get::<u32>(buf, "request_head.owner_uid")?;
            let owner_gid = get::<u32>(buf, "request_head.owner_gid")?;
            let consumed = start_len - buf.remaining();
            if consumed > struct_len {
                return Err(invalid(
                    "ceph_mds_request_head",
                    format!("struct_len {struct_len} shorter than the {consumed} bytes of a v{version} head"),
                ));
            }
            // Skip head fields from a newer client.
            denc::take(buf, struct_len - consumed, "ceph_mds_request_head tail")?;
            (owner_uid, owner_gid)
        } else {
            (caller_uid, caller_gid)
        };
        Ok((
            RequestHead {
                version,
                oldest_client_tid,
                mdsmap_epoch,
                flags,
                num_retry,
                num_fwd,
                op,
                caller_uid,
                caller_gid,
                ino,
                args,
                owner_uid,
                owner_gid,
            },
            num_releases,
        ))
    }
}

/// `ceph_mds_request_release` + name (`MClientRequest::Release`): a cap (and, with a name, a
/// dentry lease) the client releases with the request. Raw record: `u64 ino, cap_id`, `u32
/// caps, wanted, seq, issue_seq, mseq, dname_seq, dname_len` (44 bytes), then `dname_len`
/// bytes of name without a length prefix.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RequestRelease {
    pub ino: u64,
    pub cap_id: u64,
    /// Caps the client keeps (new issued).
    pub caps: u32,
    pub wanted: u32,
    pub seq: u32,
    pub issue_seq: u32,
    pub mseq: u32,
    /// Dentry lease seq when `dname` is set (`ino` is then the parent directory).
    pub dname_seq: u32,
    pub dname: Bytes,
}

impl RequestRelease {
    pub const RAW_LEN: usize = 44;
}

impl Encode for RequestRelease {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u64_le(self.ino);
        buf.put_u64_le(self.cap_id);
        buf.put_u32_le(self.caps);
        buf.put_u32_le(self.wanted);
        buf.put_u32_le(self.seq);
        buf.put_u32_le(self.issue_seq);
        buf.put_u32_le(self.mseq);
        buf.put_u32_le(self.dname_seq);
        buf.put_u32_le(self.dname.len() as u32);
        buf.put_slice(&self.dname);
    }
}

impl Decode for RequestRelease {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        denc::need(buf, Self::RAW_LEN, "ceph_mds_request_release")?;
        let ino = buf.get_u64_le();
        let cap_id = buf.get_u64_le();
        let caps = buf.get_u32_le();
        let wanted = buf.get_u32_le();
        let seq = buf.get_u32_le();
        let issue_seq = buf.get_u32_le();
        let mseq = buf.get_u32_le();
        let dname_seq = buf.get_u32_le();
        let dname_len = buf.get_u32_le() as usize;
        let dname = denc::take(buf, dname_len, "ceph_mds_request_release.dname")?;
        Ok(RequestRelease { ino, cap_id, caps, wanted, seq, issue_seq, mseq, dname_seq, dname })
    }
}

/// `MClientRequest`. See the module documentation for the wire format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MClientRequest {
    /// The request tid (`header.tid`).
    pub tid: u64,
    pub head: RequestHead,
    /// Primary path: base inode + relative path (`#ino/a/b`); for dentry ops the parent
    /// directory and the name.
    pub path: crate::types::FilePath,
    /// Secondary path or string: rename/link source, xattr or vxattr name, symlink target,
    /// readdir resume name.
    pub path2: crate::types::FilePath,
    pub releases: Vec<RequestRelease>,
    /// The client's time of the operation (ctime/mtime/btime of mutations).
    pub stamp: UTime,
    /// Supplementary gids of the caller.
    pub gid_list: Vec<u64>,
    /// fscrypt: the full ciphertext name when the dentry name is a hash (P2).
    pub alternate_name: Bytes,
    pub fscrypt_auth: Bytes,
    pub fscrypt_file: Bytes,
    /// Data segment: initial xattrs (create-type ops, [`MClientRequest::initial_xattrs`]),
    /// the xattr value (SETXATTR), the fscrypt last block (SETATTR), snap payloads.
    pub data: Bytes,
}

impl Default for MClientRequest {
    fn default() -> Self {
        MClientRequest {
            tid: 0,
            head: RequestHead::default(),
            path: Default::default(),
            path2: Default::default(),
            releases: Vec::new(),
            stamp: UTime::default(),
            gid_list: Vec::new(),
            alternate_name: Bytes::new(),
            fscrypt_auth: Bytes::new(),
            fscrypt_file: Bytes::new(),
            data: Bytes::new(),
        }
    }
}

impl MClientRequest {
    /// A request for `op` with a v3 head.
    pub fn new(tid: u64, op: MdsOp) -> Self {
        MClientRequest { tid, head: RequestHead { op, ..Default::default() }, ..Default::default() }
    }

    pub fn op(&self) -> MdsOp {
        self.head.op
    }

    /// `MClientRequest::may_write`: write ops, and OPEN with `O_CREAT` or `O_TRUNC`. Decides
    /// `-EROFS` on a read-only cache and whether success is recorded as a completed request.
    pub fn may_write(&self) -> bool {
        self.head.op.is_write()
            || (self.head.op == MdsOp::OPEN
                && self.head.args.open().flags & (open_flags::CREAT | open_flags::TRUNC) != 0)
    }

    /// The initial xattrs a create-type op (CREATE, MKNOD, MKDIR, SYMLINK) carries in its data
    /// segment (`Server::prepare_new_inode`): POSIX ACLs and security labels from the kernel.
    /// An empty data segment means none.
    pub fn initial_xattrs(&self) -> Result<XattrMap> {
        if self.data.is_empty() {
            return Ok(XattrMap::new());
        }
        decode_xattr_blob(self.data.clone()).ctx("MClientRequest initial xattrs")
    }
}

impl Message for MClientRequest {
    const TYPE: u16 = msg_type::CLIENT_REQUEST;
    const HEAD_VERSION: u16 = 6;
    const COMPAT_VERSION: u16 = 1;

    /// Message v6 with the head at `head.version` (normally 3). Clients encode requests; the MDS
    /// only decodes them, so this is mostly for tests.
    fn encode_payload(&self, features: u64) -> Payload {
        let mut b = BytesMut::with_capacity(256);
        self.head.encode_with(&mut b, self.releases.len() as u16);
        self.path.encode(&mut b, features);
        self.path2.encode(&mut b, features);
        for r in &self.releases {
            r.encode(&mut b, features);
        }
        self.stamp.encode(&mut b, features);
        self.gid_list.encode(&mut b, features);
        self.alternate_name.encode(&mut b, features);
        self.fscrypt_auth.encode(&mut b, features);
        self.fscrypt_file.encode(&mut b, features);
        Payload { front: b, middle: Bytes::new(), data: self.data.clone() }
    }

    fn decode_payload(version: u16, mut front: Bytes, _middle: Bytes, data: Bytes) -> Result<Self> {
        if version < 4 {
            return Err(invalid(
                "MClientRequest",
                format!("message v{version} uses the legacy 88-byte request head, which is not supported"),
            ));
        }
        let p = &mut front;
        let (head, num_releases) = RequestHead::decode_with_releases(p).ctx("MClientRequest.head")?;
        let path = Decode::decode(p).ctx("MClientRequest.path")?;
        let path2 = Decode::decode(p).ctx("MClientRequest.path2")?;
        let mut releases = Vec::with_capacity((num_releases as usize).min(p.remaining() / RequestRelease::RAW_LEN));
        for _ in 0..num_releases {
            releases.push(RequestRelease::decode(p).ctx("MClientRequest.releases")?);
        }
        let mut m = MClientRequest { head, path, path2, releases, data, ..Default::default() };
        m.stamp = get(p, "MClientRequest.stamp")?;
        m.gid_list = get(p, "MClientRequest.gid_list")?;
        if version >= 5 {
            m.alternate_name = get(p, "MClientRequest.alternate_name")?;
        }
        if version >= 6 {
            m.fscrypt_auth = get(p, "MClientRequest.fscrypt_auth")?;
            m.fscrypt_file = get(p, "MClientRequest.fscrypt_file")?;
        }
        Ok(m)
    }

    impl_from_raw!(tid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FilePath, setattr};
    use denc::RawMessage;

    fn sample() -> MClientRequest {
        let mut m = MClientRequest::new(0x1234, MdsOp::CREATE);
        m.head.oldest_client_tid = 0x1230;
        m.head.mdsmap_epoch = 9;
        m.head.flags = request_flags::WANT_DENTRY;
        m.head.num_retry = 300;
        m.head.num_fwd = 2;
        m.head.caller_uid = 1000;
        m.head.caller_gid = 1001;
        m.head.owner_uid = 1000;
        m.head.owner_gid = 1001;
        m.head.args = OpenArgs {
            flags: open_flags::CREAT | open_flags::EXCL | open_flags::WRONLY,
            mode: 0o100644,
            ..Default::default()
        }
        .to_args();
        m.path = FilePath::new(1, Bytes::from_static(b"newfile"));
        m.releases.push(RequestRelease {
            ino: 1,
            cap_id: 7,
            caps: 0x155,
            wanted: 0,
            seq: 3,
            issue_seq: 2,
            mseq: 0,
            dname_seq: 1,
            dname: Bytes::from_static(b"newfile"),
        });
        m.stamp = UTime { sec: 1_760_000_000, nsec: 5 };
        m.gid_list = vec![1001, 27];
        m
    }

    #[test]
    fn head_v3_layout() {
        let m = sample();
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version, raw.tid), (6, 1, 0x1234));
        let f = &raw.front;
        assert_eq!(&f[0..2], &3u16.to_le_bytes());
        assert_eq!(&f[2..10], &0x1230u64.to_le_bytes());
        assert_eq!(f[18], 300u32 as u8); // legacy 8-bit retry
        assert_eq!(f[19], 2);
        assert_eq!(&f[20..22], &1u16.to_le_bytes()); // num_releases
        assert_eq!(&f[22..26], &0x1301u32.to_le_bytes());
        assert_eq!(&f[42..46], &(open_flags::CREAT | open_flags::EXCL | open_flags::WRONLY).to_le_bytes());
        assert_eq!(&f[98..102], &300u32.to_le_bytes());
        assert_eq!(&f[106..110], &118u32.to_le_bytes());
        assert_eq!(&f[110..114], &1000u32.to_le_bytes());
        assert_eq!(&f[114..118], &1001u32.to_le_bytes());
        // path: struct_v 1, ino 1, "newfile"
        assert_eq!(&f[118..119], &[1]);
        let back = MClientRequest::from_raw(raw).unwrap();
        assert_eq!(back, m);
        assert!(back.may_write());
        assert!(back.head.op.is_new_inode());
        assert_eq!(back.head.args.open().mode, 0o100644);
    }

    #[test]
    fn head_struct_len_rules() {
        let m = sample();
        let raw = m.to_raw(0);
        // A newer client with 8 more head bytes: struct_len 126, extra bytes skipped.
        let mut f = BytesMut::from(&raw.front[..118]);
        f[106..110].copy_from_slice(&126u32.to_le_bytes());
        f.put_slice(&[0xee; 8]);
        f.put_slice(&raw.front[118..]);
        let back = MClientRequest::decode_payload(6, f.freeze(), Bytes::new(), Bytes::new()).unwrap();
        assert_eq!(back.path, m.path);
        assert_eq!(back.head.owner_gid, 1001);
        // struct_len shorter than the v3 head is malformed.
        let mut f = BytesMut::from(&raw.front[..]);
        f[106..110].copy_from_slice(&100u32.to_le_bytes());
        assert!(MClientRequest::decode_payload(6, f.freeze(), Bytes::new(), Bytes::new()).is_err());
    }

    #[test]
    fn older_heads_and_versions() {
        // Head v2 in message v6: owner falls back to caller (C++ semantics).
        let mut m = sample();
        m.head.version = 2;
        let raw = m.to_raw(0);
        assert_eq!(raw.front.len() + 12, sample().to_raw(0).front.len());
        let back = MClientRequest::from_raw(raw).unwrap();
        assert_eq!((back.head.owner_uid, back.head.owner_gid), (1000, 1001));
        assert_eq!(back.head.num_retry, 300);
        // Head v1: 8-bit counters.
        m.head.version = 1;
        let back = MClientRequest::from_raw(m.to_raw(0)).unwrap();
        assert_eq!(back.head.num_retry, 300 & 0xff);
        // Message v4 (no alternate_name / fscrypt fields; trailing bytes ignored).
        let mut raw = sample().to_raw(0);
        raw.version = 4;
        let back = MClientRequest::from_raw(raw).unwrap();
        assert_eq!(back.gid_list, vec![1001, 27]);
        // Legacy message versions are refused.
        let mut raw = sample().to_raw(0);
        raw.version = 3;
        assert!(MClientRequest::from_raw(raw).is_err());
        // Truncation anywhere fails cleanly.
        let raw = sample().to_raw(0);
        for cut in 0..raw.front.len() {
            let r = RawMessage { front: raw.front.slice(..cut), ..raw.clone() };
            assert!(MClientRequest::from_raw(r).is_err(), "cut {cut}");
        }
    }

    #[test]
    fn args_views() {
        let s = SetattrArgs {
            mode: 0o644,
            uid: 1,
            gid: 2,
            mtime: UTime { sec: 3, nsec: 4 },
            atime: UTime { sec: 5, nsec: 6 },
            size: 7,
            old_size: 8,
            mask: setattr::MODE | setattr::SIZE,
            btime: UTime { sec: 9, nsec: 10 },
        };
        let a = s.to_args();
        assert_eq!(&a.0[44..48], &(setattr::MODE | setattr::SIZE).to_le_bytes());
        assert_eq!(&a.0[48..52], &9u32.to_le_bytes());
        assert_eq!(a.setattr(), s);
        let r = ReaddirArgs {
            frag: 0x0100_0000,
            max_entries: 1024,
            max_bytes: 512 << 10,
            flags: 1,
            offset_hash: 0x00ab_cdef,
        };
        let a = r.to_args();
        assert_eq!(&a.0[12..14], &[1, 0]);
        assert_eq!(&a.0[14..18], &0x00ab_cdefu32.to_le_bytes());
        assert_eq!(a.readdir(), r);
        let l = FilelockArgs { rule: 1, lock_type: 2, owner: 1 << 63, pid: 99, start: 10, length: 0, wait: 1 };
        let a = l.to_args();
        assert_eq!(a.0[0], 1);
        assert_eq!(a.0[1], 2);
        assert_eq!(&a.0[2..10], &(1u64 << 63).to_le_bytes());
        assert_eq!(a.0[34], 1);
        assert_eq!(a.filelock(), l);
        let li = LookupinoArgs { mask: 1, snapid: 2, parent: 3, hash: 4 };
        assert_eq!(li.to_args().lookupino(), li);
        let mk = MknodArgs { mode: 0o20644, rdev: 0x0801 };
        assert_eq!(mk.to_args().mknod(), mk);
        let sx = SetxattrArgs { flags: crate::types::xattr_flags::CREATE, osdmap_epoch: 77 };
        assert_eq!(sx.to_args().setxattr(), sx);
        assert_eq!(RequestArgs::with_getattr_mask(0x155).getattr_mask(), 0x155);
        assert_eq!(RequestArgs::with_mkdir_mode(0o755).mkdir_mode(), 0o755);
        let lay = FileLayout::default_for_pool(2);
        assert_eq!(RequestArgs::with_setlayout(&lay).setlayout(), lay);
        let o = OpenArgs {
            flags: 0o1,
            mode: 0,
            stripe_unit: 0,
            stripe_count: 0,
            object_size: 0,
            pool: -1,
            mask: 0,
            old_size: 5,
        };
        assert_eq!(o.to_args().open(), o);
    }

    #[test]
    fn op_table() {
        assert_eq!(MdsOp::CLIENT_OPS.len(), 32);
        for op in MdsOp::CLIENT_OPS {
            assert!(!op.is_internal());
        }
        assert!(MdsOp::FRAGMENTDIR.is_internal());
        assert!(MdsOp::READDIR_SNAPDIFF.is_write());
        assert!(!MdsOp::READDIR.is_write());
        assert!(MdsOp::RENAME.is_write());
        assert_eq!(MdsOp::SETLAYOUT.name(), "setlayou");
        assert_eq!(format!("{:?}", MdsOp::GETATTR), "getattr(0x101)");
        let mut open = MClientRequest::new(1, MdsOp::OPEN);
        assert!(!open.may_write());
        open.head.args = OpenArgs { flags: open_flags::TRUNC, ..Default::default() }.to_args();
        assert!(open.may_write());
    }

    #[test]
    fn initial_xattrs_payload() {
        let mut x = XattrMap::new();
        x.insert(Bytes::from_static(b"system.posix_acl_access"), Bytes::from_static(&[2, 0, 0, 0]));
        let mut m = MClientRequest::new(1, MdsOp::MKDIR);
        assert!(m.initial_xattrs().unwrap().is_empty());
        m.data = crate::types::xattrs_to_blob(&x);
        let raw = m.to_raw(0);
        assert_eq!(raw.data, m.data);
        assert_eq!(MClientRequest::from_raw(raw).unwrap().initial_xattrs().unwrap(), x);
        m.data = Bytes::from_static(&[1, 0, 0]);
        assert!(m.initial_xattrs().is_err());
    }
}
