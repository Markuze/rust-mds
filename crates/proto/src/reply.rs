//! `MClientReply` (`CEPH_MSG_CLIENT_REPLY`, `src/messages/MClientReply.h`) and the records it
//! carries: `InodeStat`, `DirStat`, `LeaseStat` in their versioned ("REPLY_ENCODING") forms, the
//! trace, and the per-op extra payloads.
//!
//! Front (message version 1, compat 0; `header.tid` = request tid):
//!
//! ```text
//! ceph_mds_reply_head   u32 op, i32 result, u32 mdsmap_epoch, u8 safe, u8 is_dentry, u8 is_target  (15 bytes)
//! bufferlist trace      [InodeStat dir, DirStat, string dname, LeaseStat]   if is_dentry
//!                       [InodeStat target]                                  if is_target
//! bufferlist extra      READDIR/LSSNAP: DirStat, u32 n, u16 flags, n x (string name, LeaseStat, InodeStat)
//!                       CREATE: openc_response_t; GETFILELOCK: ceph_filelock;
//!                       GETVXATTR: ENCODE_START(1,1){string}; any other op: empty
//! bufferlist snapbl     snap trace of the target's realm (or the dir's), in the session's format
//! ```
//!
//! The encoders follow `CInode::encode_inodestat` (`src/mds/CInode.cc`), `CDir::encode_dirstat`,
//! `Locker::encode_lease`, `Server::set_trace_dist` and `Server::handle_client_readdir` /
//! `_finalize_readdir` (`src/mds/Server.cc`); the decoders mirror `InodeStat::decode` etc. in
//! `MClientReply.h` (what libcephfs runs) and accept every struct version with compat 1.
//! Both clients require the whole front, and the kernel the whole trace, to be consumed exactly;
//! the encoders produce nothing else. [`MClientReply::check_invariants`] checks the client
//! invariants of design section 7 that a reply can violate.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Message, Payload, Result, UTime};

use crate::frag::{Frag, FragTree, ceph_frag_value};
use crate::hash::hash_dentry_name;
use crate::msg_type;
use crate::request::{MClientRequest, MdsOp};
use crate::snap::{SnapFormat, SnapTrace};
use crate::types::{
    CEPH_INLINE_NONE, CEPH_NOSNAP, DirLayout, FileLayout, FileLock, MAX_CLIENT_INO, QuotaInfo,
    XattrMap, decode_xattr_blob, encode_xattrs, is_private_ino, readdir_flags,
};
use crate::util::{Ctx, get, get_string_map_lossy, impl_from_raw, invalid};

/// `S_IFMT` and the file types the MDS reports in `mode`.
pub mod mode {
    pub const S_IFMT: u32 = 0o170000;
    pub const S_IFSOCK: u32 = 0o140000;
    pub const S_IFLNK: u32 = 0o120000;
    pub const S_IFREG: u32 = 0o100000;
    pub const S_IFBLK: u32 = 0o060000;
    pub const S_IFDIR: u32 = 0o040000;
    pub const S_IFCHR: u32 = 0o020000;
    pub const S_IFIFO: u32 = 0o010000;
}

/// Linux errno values, which are the wire values of `MClientReply.result` (negated).
pub mod errno {
    pub const EPERM: i32 = 1;
    pub const ENOENT: i32 = 2;
    pub const EINTR: i32 = 4;
    pub const EIO: i32 = 5;
    pub const ENXIO: i32 = 6;
    pub const EAGAIN: i32 = 11;
    pub const EACCES: i32 = 13;
    pub const EBUSY: i32 = 16;
    pub const EEXIST: i32 = 17;
    pub const EXDEV: i32 = 18;
    pub const ENOTDIR: i32 = 20;
    pub const EISDIR: i32 = 21;
    pub const EINVAL: i32 = 22;
    pub const EFBIG: i32 = 27;
    pub const ENOSPC: i32 = 28;
    pub const EROFS: i32 = 30;
    pub const EMLINK: i32 = 31;
    pub const EDOM: i32 = 33;
    pub const EDEADLK: i32 = 35;
    pub const ENAMETOOLONG: i32 = 36;
    pub const ENOTEMPTY: i32 = 39;
    pub const ENODATA: i32 = 61;
    pub const EMULTIHOP: i32 = 72;
    pub const EBADMSG: i32 = 74;
    pub const EOPNOTSUPP: i32 = 95;
    pub const EINPROGRESS: i32 = 115;
    pub const ESTALE: i32 = 116;
    pub const EDQUOT: i32 = 122;
}

/// `ceph_mds_reply_cap` (33 raw bytes): the cap a reply issues on an inode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ReplyCap {
    pub caps: u32,
    pub wanted: u32,
    pub cap_id: u64,
    pub seq: u32,
    pub mseq: u32,
    /// Snap realm of the inode (must be known to the client from a snap trace).
    pub realm: u64,
    /// `CEPH_CAP_FLAG_AUTH` when this rank is the inode's auth.
    pub flags: u8,
}

impl ReplyCap {
    pub const LEN: usize = 33;
}

impl Encode for ReplyCap {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u32_le(self.caps);
        buf.put_u32_le(self.wanted);
        buf.put_u64_le(self.cap_id);
        buf.put_u32_le(self.seq);
        buf.put_u32_le(self.mseq);
        buf.put_u64_le(self.realm);
        buf.put_u8(self.flags);
    }
}

impl Decode for ReplyCap {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        denc::need(buf, Self::LEN, "ceph_mds_reply_cap")?;
        Ok(ReplyCap {
            caps: buf.get_u32_le(),
            wanted: buf.get_u32_le(),
            cap_id: buf.get_u64_le(),
            seq: buf.get_u32_le(),
            mseq: buf.get_u32_le(),
            realm: buf.get_u64_le(),
            flags: buf.get_u8(),
        })
    }
}

/// Optional inode metadata of `InodeStat` v8 (`optmetadata_multiton`, `include/cephfs/types.h`):
/// `ENCODE_START(1, 1)` { `u32 n` + n x (`u64 kind`, kind payload) }, where every payload is
/// itself an `ENCODE_START` frame (kind 1 = charmap, 2 = quarantine). Entries are kept as raw
/// frames; P0 sends none (charmap and quarantine are P2), and the kernel skips the field.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct OptMetadata {
    pub entries: Vec<OptMetadataEntry>,
}

/// One optmetadata entry: its kind and its complete `ENCODE_START` frame (header included).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OptMetadataEntry {
    pub kind: u64,
    pub frame: Bytes,
}

impl OptMetadata {
    pub const KIND_CHARMAP: u64 = 1;
    pub const KIND_QUARANTINE: u64 = 2;
}

impl Encode for OptMetadata {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        denc::encode_versioned(buf, 1, 1, |b| {
            b.put_u32_le(self.entries.len() as u32);
            for e in &self.entries {
                b.put_u64_le(e.kind);
                b.put_slice(&e.frame);
            }
        });
    }
}

impl Decode for OptMetadata {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut f = denc::decode_start(buf, 1, "optmetadata")?;
        let b = &mut f.body;
        let n = get::<u32>(b, "optmetadata.count")?;
        let mut entries = Vec::with_capacity((n as usize).min(b.remaining()));
        for _ in 0..n {
            let kind = get::<u64>(b, "optmetadata.kind")?;
            denc::need(b, 6, "optmetadata entry")?;
            let len = u32::from_le_bytes(b[2..6].try_into().unwrap()) as usize;
            let frame = denc::take(b, 6usize.saturating_add(len), "optmetadata entry")?;
            entries.push(OptMetadataEntry { kind, frame });
        }
        Ok(OptMetadata { entries })
    }
}

/// `InodeStat`: what a reply tells a client about one inode. The versioned encoding
/// (`ENCODE_START(9, 1)`, `CInode::encode_inodestat` with `CEPHFS_FEATURE_REPLY_ENCODING`):
///
/// | field | type | since |
/// |---|---|---|
/// | `ino`, `snapid` | u64, u64 | 1 |
/// | `rdev` | u32 | 1 |
/// | `version`, `xattr_version` | u64, u64 | 1 |
/// | `cap` | `ceph_mds_reply_cap` (33 bytes) | 1 |
/// | `layout` | legacy `ceph_file_layout` (28 bytes) | 1 |
/// | `ctime`, `mtime`, `atime` | utime x3 | 1 |
/// | `time_warp_seq` | u32 | 1 |
/// | `size`, `max_size`, `truncate_size` | u64 x3 | 1 |
/// | `truncate_seq`, `mode`, `uid`, `gid`, `nlink` | u32 x5 | 1 |
/// | `nfiles`, `nsubdirs`, `rbytes`, `rfiles`, `rsubdirs` | i64 x5 | 1 |
/// | `rctime` | utime | 1 |
/// | `fragtree` | `fragtree_t` | 1 |
/// | `symlink` | string | 1 |
/// | `dir_layout` | `ceph_dir_layout` (8 bytes) | 1 |
/// | `xattrs` | `u32 len` + blob (len 0 = not included) | 1 |
/// | `inline_version`, `inline_data` | u64, bufferlist | 1 |
/// | `quota` | `quota_info_t` | 1 |
/// | `layout.pool_ns` | string | 1 |
/// | `btime`, `change_attr` | utime, u64 | 1 |
/// | `dir_pin` | i32 | 2 |
/// | `snap_btime` | utime | 3 |
/// | `rsnaps` | i64 | 4 |
/// | `snap_metadata` | `map<string,string>` | 5 |
/// | fscrypt flag | bool (`!fscrypt_auth.empty()`) | 6 |
/// | `fscrypt_auth`, `fscrypt_file` | `vector<u8>` x2 | 7 |
/// | `optmetadata` | [`OptMetadata`] | 8 |
/// | `subvolume_id` | u64 | 9 |
///
/// The fixed prefix up to the fragtree count is the kernel's packed `ceph_mds_reply_inode`.
/// Ceph 20.2 MDS daemons send v8 (no `subvolume_id`); this crate sends v9.
#[derive(Clone, PartialEq, Eq)]
pub struct InodeStat {
    pub ino: u64,
    /// `CEPH_NOSNAP` for head inodes.
    pub snapid: u64,
    pub rdev: u32,
    /// Inode version: `2 * version + 1` if it has uncommitted (projected) changes, else even.
    pub version: u64,
    /// Non-zero only when `xattrs` is included.
    pub xattr_version: u64,
    pub cap: ReplyCap,
    /// File layout (directories: their layout policy, if any).
    pub layout: FileLayout,
    pub ctime: UTime,
    pub mtime: UTime,
    pub atime: UTime,
    pub time_warp_seq: u32,
    /// Directories: 0.
    pub size: u64,
    /// The client's writeable range (only meaningful with an auth cap).
    pub max_size: u64,
    pub truncate_size: u64,
    pub truncate_seq: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    /// Directories: 1 (libcephfs aborts on anything other than 0 or 1).
    pub nlink: u32,
    /// Directory entry counts (`dirstat`).
    pub nfiles: i64,
    pub nsubdirs: i64,
    /// Recursive stats (`rstat`).
    pub rbytes: i64,
    pub rfiles: i64,
    pub rsubdirs: i64,
    pub rctime: UTime,
    pub fragtree: FragTree,
    /// Symlink target (its length is the symlink's size).
    pub symlink: Bytes,
    pub dir_layout: DirLayout,
    /// `None`: not included (`u32 0`); `Some`: the blob, even when empty.
    pub xattrs: Option<XattrMap>,
    /// [`CEPH_INLINE_NONE`] when the inode has no inline data; never 0 from this MDS.
    pub inline_version: u64,
    pub inline_data: Bytes,
    pub quota: QuotaInfo,
    pub btime: UTime,
    pub change_attr: u64,
    /// `export_pin`: -1 when unset (decoders report `-ENODATA` for v1 records).
    pub dir_pin: i32,
    /// Snapshot creation time, for snapped inodes.
    pub snap_btime: UTime,
    pub rsnaps: i64,
    pub snap_metadata: BTreeMap<String, String>,
    pub fscrypt_auth: Bytes,
    pub fscrypt_file: Bytes,
    pub optmetadata: OptMetadata,
    /// The subvolume (shard root) the inode belongs to; 0 = none. Never changes once non-zero.
    pub subvolume_id: u64,
}

impl Default for InodeStat {
    /// A head inode with no inline data, no pin, no layout and everything else zero.
    fn default() -> Self {
        InodeStat {
            ino: 0,
            snapid: CEPH_NOSNAP,
            rdev: 0,
            version: 0,
            xattr_version: 0,
            cap: ReplyCap::default(),
            layout: FileLayout::default(),
            ctime: UTime::default(),
            mtime: UTime::default(),
            atime: UTime::default(),
            time_warp_seq: 0,
            size: 0,
            max_size: 0,
            truncate_size: 0,
            truncate_seq: 0,
            mode: 0,
            uid: 0,
            gid: 0,
            nlink: 0,
            nfiles: 0,
            nsubdirs: 0,
            rbytes: 0,
            rfiles: 0,
            rsubdirs: 0,
            rctime: UTime::default(),
            fragtree: FragTree::default(),
            symlink: Bytes::new(),
            dir_layout: DirLayout::default(),
            xattrs: None,
            inline_version: CEPH_INLINE_NONE,
            inline_data: Bytes::new(),
            quota: QuotaInfo::default(),
            btime: UTime::default(),
            change_attr: 0,
            dir_pin: -1,
            snap_btime: UTime::default(),
            rsnaps: 0,
            snap_metadata: BTreeMap::new(),
            fscrypt_auth: Bytes::new(),
            fscrypt_file: Bytes::new(),
            optmetadata: OptMetadata::default(),
            subvolume_id: 0,
        }
    }
}

impl fmt::Debug for InodeStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InodeStat")
            .field("ino", &format_args!("{:#x}", self.ino))
            .field("snapid", &self.snapid)
            .field("mode", &format_args!("{:#o}", self.mode))
            .field("version", &self.version)
            .field(
                "cap",
                &format_args!(
                    "{} {:?}",
                    crate::types::cap::cap_string(self.cap.caps),
                    self.cap
                ),
            )
            .field("size", &self.size)
            .field("nlink", &self.nlink)
            .field("uid", &self.uid)
            .field("gid", &self.gid)
            .field("ctime", &self.ctime)
            .field("mtime", &self.mtime)
            .field("change_attr", &self.change_attr)
            .field("nfiles", &self.nfiles)
            .field("nsubdirs", &self.nsubdirs)
            .field("rbytes", &self.rbytes)
            .field("xattrs", &self.xattrs.as_ref().map(|x| x.len()))
            .field("layout", &self.layout)
            .field("dir_hash", &self.dir_layout.dir_hash)
            .field("subvolume_id", &self.subvolume_id)
            .finish_non_exhaustive()
    }
}

impl InodeStat {
    /// The struct version this crate encodes.
    pub const STRUCT_V: u8 = 9;

    pub fn file_type(&self) -> u32 {
        self.mode & mode::S_IFMT
    }
    pub fn is_dir(&self) -> bool {
        self.file_type() == mode::S_IFDIR
    }

    /// Encode at `struct_v` (1..=9; fields of later versions are left out). Use [`Encode`],
    /// which writes v9, except to reproduce the bytes of older senders (Ceph 20.2 sends v8).
    pub fn encode_struct_v(&self, buf: &mut BytesMut, struct_v: u8) {
        let struct_v = struct_v.clamp(1, Self::STRUCT_V);
        denc::encode_versioned(buf, struct_v, 1, |b| {
            b.put_u64_le(self.ino);
            b.put_u64_le(self.snapid);
            b.put_u32_le(self.rdev);
            b.put_u64_le(self.version);
            b.put_u64_le(self.xattr_version);
            self.cap.encode(b, 0);
            self.layout.encode_legacy(b);
            self.ctime.encode(b, 0);
            self.mtime.encode(b, 0);
            self.atime.encode(b, 0);
            b.put_u32_le(self.time_warp_seq);
            b.put_u64_le(self.size);
            b.put_u64_le(self.max_size);
            b.put_u64_le(self.truncate_size);
            b.put_u32_le(self.truncate_seq);
            b.put_u32_le(self.mode);
            b.put_u32_le(self.uid);
            b.put_u32_le(self.gid);
            b.put_u32_le(self.nlink);
            b.put_i64_le(self.nfiles);
            b.put_i64_le(self.nsubdirs);
            b.put_i64_le(self.rbytes);
            b.put_i64_le(self.rfiles);
            b.put_i64_le(self.rsubdirs);
            self.rctime.encode(b, 0);
            self.fragtree.encode(b, 0);
            self.symlink.encode(b, 0);
            self.dir_layout.encode(b, 0);
            match &self.xattrs {
                None => b.put_u32_le(0),
                Some(x) => {
                    let len_pos = b.len();
                    b.put_u32_le(0);
                    encode_xattrs(x, b);
                    let len = (b.len() - len_pos - 4) as u32;
                    b[len_pos..len_pos + 4].copy_from_slice(&len.to_le_bytes());
                }
            }
            b.put_u64_le(self.inline_version);
            self.inline_data.encode(b, 0);
            self.quota.encode(b, 0);
            self.layout.pool_ns.encode(b, 0);
            self.btime.encode(b, 0);
            b.put_u64_le(self.change_attr);
            if struct_v >= 2 {
                b.put_i32_le(self.dir_pin);
            }
            if struct_v >= 3 {
                self.snap_btime.encode(b, 0);
            }
            if struct_v >= 4 {
                b.put_i64_le(self.rsnaps);
            }
            if struct_v >= 5 {
                self.snap_metadata.encode(b, 0);
            }
            if struct_v >= 6 {
                b.put_u8(u8::from(!self.fscrypt_auth.is_empty()));
            }
            if struct_v >= 7 {
                self.fscrypt_auth.encode(b, 0);
                self.fscrypt_file.encode(b, 0);
            }
            if struct_v >= 8 {
                self.optmetadata.encode(b, 0);
            }
            if struct_v >= 9 {
                b.put_u64_le(self.subvolume_id);
            }
        });
    }

    /// The encoded size (for readdir byte budgets).
    pub fn encoded_len(&self) -> usize {
        let mut b = BytesMut::with_capacity(256);
        self.encode(&mut b, 0);
        b.len()
    }

    /// Client invariants this record must satisfy (design section 7); returns the violations.
    pub fn check_invariants(&self) -> Vec<String> {
        let mut v = Vec::new();
        if is_private_ino(self.ino) {
            v.push(format!(
                "ino {:#x} is in the MDS-private range [0x100, 0x1000)",
                self.ino
            ));
        }
        if self.ino >= MAX_CLIENT_INO {
            v.push(format!("ino {:#x} is not below 2^48", self.ino));
        }
        if self.inline_version == 0 {
            v.push(format!(
                "ino {:#x}: inline_version 0 (use CEPH_INLINE_NONE)",
                self.ino
            ));
        }
        if self.is_dir() {
            if self.nlink > 1 {
                v.push(format!(
                    "dir {:#x}: nlink {} (directories report 1)",
                    self.ino, self.nlink
                ));
            }
            if self.size != 0 {
                v.push(format!(
                    "dir {:#x}: size {} (directories report 0)",
                    self.ino, self.size
                ));
            }
        }
        v
    }
}

impl Encode for InodeStat {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        self.encode_struct_v(buf, Self::STRUCT_V);
    }
}

impl Decode for InodeStat {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut f = denc::decode_start(buf, InodeStat::STRUCT_V, "InodeStat")?;
        let v = f.v;
        let b = &mut f.body;
        let fixed = (|| -> Result<InodeStat> {
            let ino = get(b, "ino")?;
            let snapid = get(b, "snapid")?;
            let rdev = get(b, "rdev")?;
            let version = get(b, "version")?;
            let xattr_version = get(b, "xattr_version")?;
            let cap = ReplyCap::decode(b)?;
            let layout = FileLayout::decode_legacy(b)?;
            Ok(InodeStat {
                ino,
                snapid,
                rdev,
                version,
                xattr_version,
                cap,
                layout,
                ctime: get(b, "ctime")?,
                mtime: get(b, "mtime")?,
                atime: get(b, "atime")?,
                time_warp_seq: get(b, "time_warp_seq")?,
                size: get(b, "size")?,
                max_size: get(b, "max_size")?,
                truncate_size: get(b, "truncate_size")?,
                truncate_seq: get(b, "truncate_seq")?,
                mode: get(b, "mode")?,
                uid: get(b, "uid")?,
                gid: get(b, "gid")?,
                nlink: get(b, "nlink")?,
                nfiles: get(b, "nfiles")?,
                nsubdirs: get(b, "nsubdirs")?,
                rbytes: get(b, "rbytes")?,
                rfiles: get(b, "rfiles")?,
                rsubdirs: get(b, "rsubdirs")?,
                rctime: get(b, "rctime")?,
                ..Default::default()
            })
        })()
        .ctx("InodeStat")?;
        let mut st = fixed;
        st.fragtree = FragTree::decode(b).ctx("InodeStat.fragtree")?;
        st.symlink = get(b, "InodeStat.symlink")?;
        st.dir_layout = DirLayout::decode(b).ctx("InodeStat.dir_layout")?;
        let xblob = get::<Bytes>(b, "InodeStat.xattrs")?;
        st.xattrs = if xblob.is_empty() {
            None
        } else {
            Some(decode_xattr_blob(xblob).ctx("InodeStat.xattrs")?)
        };
        st.inline_version = get(b, "InodeStat.inline_version")?;
        st.inline_data = get(b, "InodeStat.inline_data")?;
        st.quota = QuotaInfo::decode(b).ctx("InodeStat.quota")?;
        st.layout.pool_ns =
            String::from_utf8_lossy(&get::<Bytes>(b, "InodeStat.pool_ns")?).into_owned();
        st.btime = get(b, "InodeStat.btime")?;
        st.change_attr = get(b, "InodeStat.change_attr")?;
        st.dir_pin = if v >= 2 {
            get(b, "InodeStat.dir_pin")?
        } else {
            -errno::ENODATA
        };
        if v >= 3 {
            st.snap_btime = get(b, "InodeStat.snap_btime")?;
        }
        if v >= 4 {
            st.rsnaps = get(b, "InodeStat.rsnaps")?;
        }
        if v >= 5 {
            st.snap_metadata = get_string_map_lossy(b, "InodeStat.snap_metadata")?;
        }
        if v >= 6 {
            let _fscrypt_flag = get::<u8>(b, "InodeStat.fscrypt")?;
        }
        if v >= 7 {
            st.fscrypt_auth = get(b, "InodeStat.fscrypt_auth")?;
            st.fscrypt_file = get(b, "InodeStat.fscrypt_file")?;
        }
        if v >= 8 {
            st.optmetadata = OptMetadata::decode(b).ctx("InodeStat.optmetadata")?;
        }
        if v >= 9 {
            st.subvolume_id = get(b, "InodeStat.subvolume_id")?;
        }
        Ok(st)
    }
}

/// `DirStat`: where a directory fragment lives (`CDir::encode_dirstat`): `ENCODE_START(1, 1)`
/// { `u32 frag`, `i32 auth`, `set<i32> dist` }. `auth` -1 (`CDIR_AUTH_PARENT`) means "same as
/// the directory inode's auth"; `dist` lists replica ranks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DirStat {
    pub frag: Frag,
    pub auth: i32,
    pub dist: BTreeSet<i32>,
}

impl Default for DirStat {
    fn default() -> Self {
        DirStat {
            frag: Frag::ROOT,
            auth: -1,
            dist: BTreeSet::new(),
        }
    }
}

impl DirStat {
    /// The single frag of a P0 directory, authoritative on `auth` (or -1 = the inode's auth).
    pub fn root(auth: i32) -> Self {
        DirStat {
            frag: Frag::ROOT,
            auth,
            dist: BTreeSet::new(),
        }
    }
}

impl Encode for DirStat {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        denc::encode_versioned(buf, 1, 1, |b| {
            self.frag.encode(b, f);
            b.put_i32_le(self.auth);
            self.dist.encode(b, f);
        });
    }
}

impl Decode for DirStat {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut fr = denc::decode_start(buf, 1, "DirStat")?;
        let b = &mut fr.body;
        Ok(DirStat {
            frag: Frag::decode(b)?,
            auth: get(b, "DirStat.auth")?,
            dist: get(b, "DirStat.dist")?,
        })
    }
}

/// `LeaseStat`: the dentry lease a reply grants (`Locker::encode_lease`): `ENCODE_START(2, 1)`
/// { `u16 mask`, `u32 duration_ms`, `u32 seq`, `string alternate_name` (v2) }. P0 sends null
/// leases (mask 0): clients then cache dentries only through the directory's `Fs` cap.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct LeaseStat {
    /// `CEPH_LEASE_VALID` (3) and `CEPH_LEASE_PRIMARY_LINK` (4), or 0 for no lease.
    pub mask: u16,
    pub duration_ms: u32,
    pub seq: u32,
    /// fscrypt ciphertext name (P2); empty otherwise.
    pub alternate_name: Bytes,
}

impl LeaseStat {
    /// The null lease (mask 0, duration 0, seq 0).
    pub fn null() -> Self {
        LeaseStat::default()
    }
}

impl Encode for LeaseStat {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        denc::encode_versioned(buf, 2, 1, |b| {
            b.put_u16_le(self.mask);
            b.put_u32_le(self.duration_ms);
            b.put_u32_le(self.seq);
            self.alternate_name.encode(b, f);
        });
    }
}

impl Decode for LeaseStat {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut fr = denc::decode_start(buf, 2, "LeaseStat")?;
        let b = &mut fr.body;
        let mask = get(b, "LeaseStat.mask")?;
        let duration_ms = get(b, "LeaseStat.duration_ms")?;
        let seq = get(b, "LeaseStat.seq")?;
        let alternate_name = if fr.v >= 2 {
            get(b, "LeaseStat.alternate_name")?
        } else {
            Bytes::new()
        };
        Ok(LeaseStat {
            mask,
            duration_ms,
            seq,
            alternate_name,
        })
    }
}

/// The dentry part of a trace: the parent directory, its frag, the name and the lease. The
/// directory must be the request's locked parent (ino and snapid; kernel `BUG_ON`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceDentry {
    pub dir: InodeStat,
    pub dirstat: DirStat,
    pub name: Bytes,
    pub lease: LeaseStat,
}

/// A reply trace (`Server::set_trace_dist`): optional dentry part, optional target inode.
/// A dentry without a target is a null dentry (e.g. `-ENOENT` with `WANT_DENTRY`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trace {
    pub dentry: Option<TraceDentry>,
    pub target: Option<InodeStat>,
}

impl Trace {
    pub fn is_empty(&self) -> bool {
        self.dentry.is_none() && self.target.is_none()
    }

    /// Append the trace records (no length prefix).
    pub fn encode_into(&self, buf: &mut BytesMut) {
        self.encode_into_v(buf, InodeStat::STRUCT_V);
    }

    /// Like [`Trace::encode_into`] with the `InodeStat` records at `inodestat_v`.
    pub fn encode_into_v(&self, buf: &mut BytesMut, inodestat_v: u8) {
        if let Some(d) = &self.dentry {
            d.dir.encode_struct_v(buf, inodestat_v);
            d.dirstat.encode(buf, 0);
            d.name.encode(buf, 0);
            d.lease.encode(buf, 0);
        }
        if let Some(t) = &self.target {
            t.encode_struct_v(buf, inodestat_v);
        }
    }

    /// Decode a trace blob, which must be consumed exactly (the kernel's rule).
    pub fn decode(mut blob: Bytes, is_dentry: bool, is_target: bool) -> Result<Self> {
        let b = &mut blob;
        let dentry = if is_dentry {
            Some(TraceDentry {
                dir: InodeStat::decode(b).ctx("trace dir")?,
                dirstat: DirStat::decode(b).ctx("trace dirstat")?,
                name: get(b, "trace dname")?,
                lease: LeaseStat::decode(b).ctx("trace lease")?,
            })
        } else {
            None
        };
        let target = if is_target {
            Some(InodeStat::decode(b).ctx("trace target")?)
        } else {
            None
        };
        if b.has_remaining() {
            return Err(invalid(
                "trace",
                format!("{} trailing bytes", b.remaining()),
            ));
        }
        Ok(Trace { dentry, target })
    }
}

/// One READDIR/LSSNAP entry: name, dentry lease, inode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaddirEntry {
    pub name: Bytes,
    pub lease: LeaseStat,
    pub inode: InodeStat,
}

/// The READDIR/LSSNAP extra (`Server::handle_client_readdir`, `_finalize_readdir`):
/// `DirStat`, `u32 count`, `u16 flags`, then the entries. `flags`: `FRAG_END` when the frag is
/// exhausted, `FRAG_COMPLETE` when it was also read from its start, `HASH_ORDER|OFFSET_HASH`
/// when the request had `REPLY_BITFLAGS`. Entries must come in non-decreasing
/// `(ceph_frag_value(hash(name)), name)` order and never exceed the request's `max_entries`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReaddirReply {
    pub dirstat: DirStat,
    pub flags: u16,
    pub entries: Vec<ReaddirEntry>,
}

impl ReaddirReply {
    /// Flags for a chunk of a frag: `start` = listed from the beginning, `end` = nothing left,
    /// `bitflags` = the request had `REPLY_BITFLAGS`.
    pub fn flags_for(start: bool, end: bool, bitflags: bool) -> u16 {
        let mut f = 0;
        if bitflags {
            f |= readdir_flags::HASH_ORDER | readdir_flags::OFFSET_HASH;
        }
        if end {
            f |= readdir_flags::FRAG_END;
            if start {
                f |= readdir_flags::FRAG_COMPLETE;
            }
        }
        f
    }

    pub fn encode_into(&self, buf: &mut BytesMut) {
        self.encode_into_v(buf, InodeStat::STRUCT_V);
    }

    /// Like [`ReaddirReply::encode_into`] with the `InodeStat` records at `inodestat_v`.
    pub fn encode_into_v(&self, buf: &mut BytesMut, inodestat_v: u8) {
        self.dirstat.encode(buf, 0);
        buf.put_u32_le(self.entries.len() as u32);
        buf.put_u16_le(self.flags);
        for e in &self.entries {
            e.name.encode(buf, 0);
            e.lease.encode(buf, 0);
            e.inode.encode_struct_v(buf, inodestat_v);
        }
    }

    pub fn decode(mut blob: Bytes) -> Result<Self> {
        let b = &mut blob;
        let dirstat = DirStat::decode(b).ctx("readdir dirstat")?;
        let n = get::<u32>(b, "readdir count")? as usize;
        let flags = get::<u16>(b, "readdir flags")?;
        let mut entries = Vec::with_capacity(n.min(b.remaining() / 64 + 1));
        for _ in 0..n {
            entries.push(ReaddirEntry {
                name: get(b, "readdir name")?,
                lease: LeaseStat::decode(b).ctx("readdir lease")?,
                inode: InodeStat::decode(b).ctx("readdir inode")?,
            });
        }
        if b.has_remaining() {
            return Err(invalid(
                "readdir extra",
                format!("{} trailing bytes", b.remaining()),
            ));
        }
        Ok(ReaddirReply {
            dirstat,
            flags,
            entries,
        })
    }

    /// The encoded size of this extra.
    pub fn to_extra_len(&self) -> usize {
        let mut b = BytesMut::new();
        self.encode_into(&mut b);
        b.len()
    }

    /// The bytes one more entry adds to the extra.
    pub fn entry_len(name: &[u8], lease: &LeaseStat, inode: &InodeStat) -> usize {
        4 + name.len() + 6 + 10 + 4 + lease.alternate_name.len() + inode.encoded_len()
    }

    /// Build one READDIR chunk the way `Server::handle_client_readdir` budgets it.
    ///
    /// `entries` yields the frag's entries after the resume point, already in
    /// [`crate::frag::dentry_key_cmp`] order. At most `args.max_entries` entries are taken (0 =
    /// no limit; the kernel fails a reply with more), and the encoded entries stay within
    /// `args.max_bytes` (0 = 512 KiB + `max_xattr_size`, the MDSMap's limit) minus the DirStat,
    /// count, flags and `snap_trace_len`. Unlike C++, at least one entry is always returned when
    /// one remains, so a client asking for a tiny budget still makes progress. `start` says the
    /// listing began at the frag's start (no resume name or hash); flags get `FRAG_END` (and
    /// `FRAG_COMPLETE` with `start`) when `entries` is exhausted, and `HASH_ORDER|OFFSET_HASH`
    /// when the request had `REPLY_BITFLAGS`.
    pub fn fill(
        dirstat: DirStat,
        entries: impl IntoIterator<Item = ReaddirEntry>,
        args: &crate::request::ReaddirArgs,
        snap_trace_len: usize,
        max_xattr_size: u64,
        start: bool,
    ) -> ReaddirReply {
        let max_entries = if args.max_entries == 0 {
            usize::MAX
        } else {
            args.max_entries as usize
        };
        let max_bytes = if args.max_bytes == 0 {
            (512usize << 10).saturating_add(max_xattr_size as usize)
        } else {
            args.max_bytes as usize
        };
        let front = denc::encode_to_bytes(&dirstat, 0).len() + 4 + 2;
        let budget = max_bytes
            .saturating_sub(front)
            .saturating_sub(snap_trace_len);
        let mut out = Vec::new();
        let mut used = 0usize;
        let mut it = entries.into_iter().peekable();
        while out.len() < max_entries {
            let Some(e) = it.peek() else { break };
            let len = Self::entry_len(&e.name, &e.lease, &e.inode);
            if !out.is_empty() && used + len > budget {
                break;
            }
            used += len;
            out.push(it.next().expect("peeked"));
        }
        let end = it.peek().is_none();
        let bitflags = args.flags & readdir_flags::REPLY_BITFLAGS != 0;
        ReaddirReply {
            dirstat,
            flags: Self::flags_for(start, end, bitflags),
            entries: out,
        }
    }
}

/// `openc_response_t` (`MClientReply.h`), the CREATE extra whenever the MDS advertised
/// `DELEG_INO` (both clients pick the parser from that bit): `ENCODE_START(1, 1)` { `u64
/// created_ino`, `interval_set<inodeno_t> delegated_inos` (`u32 n` + n x (`u64 start`, `u64
/// len`)) }. P0 never delegates, so `delegated_inos` stays empty; an empty extra instead tells
/// the client that an existing file was opened.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct OpencResponse {
    pub created_ino: u64,
    /// Sorted, non-overlapping `(start, len)` ranges.
    pub delegated_inos: Vec<(u64, u64)>,
}

impl Encode for OpencResponse {
    fn encode(&self, buf: &mut BytesMut, _f: u64) {
        denc::encode_versioned(buf, 1, 1, |b| {
            b.put_u64_le(self.created_ino);
            b.put_u32_le(self.delegated_inos.len() as u32);
            for (s, l) in &self.delegated_inos {
                b.put_u64_le(*s);
                b.put_u64_le(*l);
            }
        });
    }
}

impl Decode for OpencResponse {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut fr = denc::decode_start(buf, 1, "openc_response_t")?;
        let b = &mut fr.body;
        let created_ino = get(b, "openc_response_t.created_ino")?;
        let n = get::<u32>(b, "openc_response_t.delegated_inos")? as usize;
        let mut delegated_inos = Vec::with_capacity(n.min(b.remaining() / 16));
        for _ in 0..n {
            delegated_inos.push((get(b, "delegated start")?, get(b, "delegated len")?));
        }
        Ok(OpencResponse {
            created_ino,
            delegated_inos,
        })
    }
}

/// The op-specific `extra_bl` of a reply. Only CREATE, READDIR, LSSNAP, GETFILELOCK and
/// GETVXATTR may carry one (the kernel fails any other non-empty extra with `-EIO`); a
/// successful READDIR/LSSNAP must carry one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ReplyExtra {
    #[default]
    None,
    /// READDIR, LSSNAP (and READDIR_SNAPDIFF, P1).
    Readdir(ReaddirReply),
    /// CREATE that created the inode.
    Create(OpencResponse),
    /// GETFILELOCK: the conflicting lock, or `lock_type = UNLOCK` if none.
    FileLock(FileLock),
    /// GETVXATTR: `ENCODE_START(1, 1)` { `string value` } (the kernel requires exactly this).
    Vxattr(Bytes),
    /// Undecoded bytes (decoder fallback for unknown or unparsable extras).
    Raw(Bytes),
}

impl ReplyExtra {
    pub fn is_none(&self) -> bool {
        match self {
            ReplyExtra::None => true,
            ReplyExtra::Raw(b) => b.is_empty(),
            _ => false,
        }
    }

    pub fn to_bytes(&self) -> Bytes {
        let mut b = BytesMut::new();
        self.encode_into(&mut b);
        b.freeze()
    }

    pub fn encode_into(&self, b: &mut BytesMut) {
        self.encode_into_v(b, InodeStat::STRUCT_V);
    }

    /// Like [`ReplyExtra::encode_into`] with readdir `InodeStat` records at `inodestat_v`.
    pub fn encode_into_v(&self, b: &mut BytesMut, inodestat_v: u8) {
        match self {
            ReplyExtra::None => {}
            ReplyExtra::Readdir(r) => r.encode_into_v(b, inodestat_v),
            ReplyExtra::Create(c) => c.encode(b, 0),
            ReplyExtra::FileLock(l) => l.encode(b, 0),
            ReplyExtra::Vxattr(v) => denc::encode_versioned(b, 1, 1, |b| v.encode(b, 0)),
            ReplyExtra::Raw(r) => b.put_slice(r),
        }
    }

    /// Parse an extra blob the way the reply op dictates (the session is assumed to have
    /// `REPLY_ENCODING` and the MDS `DELEG_INO`). Unparsable blobs, and blobs with trailing bytes,
    /// are kept as [`ReplyExtra::Raw`].
    pub fn parse(op: MdsOp, blob: Bytes) -> ReplyExtra {
        if blob.is_empty() {
            return ReplyExtra::None;
        }
        let parsed = match op {
            MdsOp::READDIR | MdsOp::LSSNAP | MdsOp::READDIR_SNAPDIFF => {
                ReaddirReply::decode(blob.clone())
                    .ok()
                    .map(ReplyExtra::Readdir)
            }
            MdsOp::CREATE => denc::decode_exact::<OpencResponse>(blob.clone(), "openc")
                .ok()
                .map(ReplyExtra::Create),
            MdsOp::GETFILELOCK => denc::decode_exact::<FileLock>(blob.clone(), "filelock")
                .ok()
                .map(ReplyExtra::FileLock),
            MdsOp::GETVXATTR => {
                let mut b = blob.clone();
                match denc::decode_start(&mut b, 1, "getvxattr") {
                    Ok(f) if !b.has_remaining() => denc::decode_exact::<Bytes>(f.body, "getvxattr")
                        .ok()
                        .map(ReplyExtra::Vxattr),
                    _ => None,
                }
            }
            _ => None,
        };
        parsed.unwrap_or(ReplyExtra::Raw(blob))
    }

    /// Whether a reply to `op` may carry this extra.
    pub fn allowed_for(&self, op: MdsOp) -> bool {
        match self {
            ReplyExtra::None => true,
            ReplyExtra::Readdir(_) => {
                matches!(op, MdsOp::READDIR | MdsOp::LSSNAP | MdsOp::READDIR_SNAPDIFF)
            }
            ReplyExtra::Create(_) => op == MdsOp::CREATE,
            ReplyExtra::FileLock(_) => op == MdsOp::GETFILELOCK,
            ReplyExtra::Vxattr(_) => op == MdsOp::GETVXATTR,
            ReplyExtra::Raw(b) => b.is_empty() || op == MdsOp::FILE_BLOCKDIFF,
        }
    }
}

/// `MClientReply`. See the module documentation for the wire format. Build one with
/// [`MClientReply::for_request`], set `trace`, `extra` and `snapbl`, and send it with
/// [`Message::to_raw`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MClientReply {
    /// The request tid (`header.tid`).
    pub tid: u64,
    /// The request's op; the kernel selects the extra parser by it.
    pub op: MdsOp,
    /// 0 or a negative Linux errno ([`errno`]).
    pub result: i32,
    /// The MDSMap epoch (set in final replies).
    pub mdsmap_epoch: u32,
    /// Committed (`true`) or early reply. This MDS only sends safe replies.
    pub safe: bool,
    pub trace: Trace,
    pub extra: ReplyExtra,
    /// The snap trace of the target's realm (or the directory's), encoded for the session with
    /// [`SnapTrace::encode`]; empty for snapshot (non-head) replies.
    pub snapbl: Bytes,
}

impl MClientReply {
    /// A safe reply with no trace (`MClientReply(req, result)`).
    pub fn new(tid: u64, op: MdsOp, result: i32) -> Self {
        MClientReply {
            tid,
            op,
            result,
            mdsmap_epoch: 0,
            safe: true,
            trace: Trace::default(),
            extra: ReplyExtra::None,
            snapbl: Bytes::new(),
        }
    }

    /// A safe reply to `req` with `result`.
    pub fn for_request(req: &MClientRequest, result: i32) -> Self {
        Self::new(req.tid, req.op(), result)
    }

    /// Set the snap blob from a typed trace in the session's format.
    pub fn set_snap_trace(&mut self, trace: &SnapTrace, format: SnapFormat) {
        self.snapbl = trace.encode(format);
    }

    /// Check the client invariants of design section 7 that this reply can violate on its own:
    /// private or too-large inode numbers, `inline_version` 0, directory nlink/size, extra only
    /// for the listed ops, a successful READDIR/LSSNAP with its extra, FRAG_END on an empty
    /// LSSNAP, readdir hash order (using the target directory's hash function), a LOOKUPNAME
    /// dentry trace with a target, and a parsable snap blob. Returns the violations.
    pub fn check_invariants(&self) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(d) = &self.trace.dentry {
            v.extend(d.dir.check_invariants());
            if !d.dir.is_dir() {
                v.push(format!("trace dir {:#x} is not a directory", d.dir.ino));
            }
        }
        if let Some(t) = &self.trace.target {
            v.extend(t.check_invariants());
        }
        if !self.extra.allowed_for(self.op) {
            v.push(format!(
                "{:?} reply must not carry {:?}",
                self.op,
                extra_kind(&self.extra)
            ));
        }
        if self.op == MdsOp::LOOKUPNAME
            && self.trace.dentry.is_some()
            && self.trace.target.is_none()
        {
            v.push("LOOKUPNAME dentry trace without the target".into());
        }
        if matches!(self.op, MdsOp::READDIR | MdsOp::LSSNAP) && self.result == 0 {
            match &self.extra {
                ReplyExtra::Readdir(r) => {
                    if self.op == MdsOp::LSSNAP
                        && r.entries.is_empty()
                        && r.flags & readdir_flags::FRAG_END == 0
                    {
                        v.push("empty LSSNAP reply without FRAG_END".into());
                    }
                    for e in &r.entries {
                        v.extend(e.inode.check_invariants());
                    }
                    if r.flags & readdir_flags::HASH_ORDER != 0
                        && self.op == MdsOp::READDIR
                        && let Some(dir) = &self.trace.target
                    {
                        let h = dir.dir_layout.dir_hash;
                        let keys: Vec<u32> = r
                            .entries
                            .iter()
                            .map(|e| ceph_frag_value(hash_dentry_name(h, &e.name)))
                            .collect();
                        if keys.windows(2).any(|w| w[0] > w[1]) {
                            v.push("readdir entries are not in hash order".into());
                        }
                        if r.entries
                            .iter()
                            .any(|e| !r.dirstat.frag.contains_value(hash_dentry_name(h, &e.name)))
                        {
                            v.push("readdir entry outside the reported frag".into());
                        }
                    }
                }
                _ => v.push(format!(
                    "successful {:?} reply without its readdir extra",
                    self.op
                )),
            }
        }
        if !self.snapbl.is_empty()
            && SnapTrace::decode(self.snapbl.clone(), SnapFormat::Old).is_err()
            && SnapTrace::decode(self.snapbl.clone(), SnapFormat::New).is_err()
        {
            v.push("snap blob does not parse as a snap trace".into());
        }
        v
    }

    /// Like [`MClientReply::check_invariants`], plus the checks that need the request: the
    /// dentry trace's directory is the request's base inode when the request names one
    /// dentry, and a readdir reply holds at most `max_entries` entries.
    pub fn check_invariants_for(&self, req: &MClientRequest) -> Vec<String> {
        let mut v = self.check_invariants();
        if self.op != req.op() || self.tid != req.tid {
            v.push(format!(
                "reply ({:?}, tid {}) does not match the request ({:?}, tid {})",
                self.op,
                self.tid,
                req.op(),
                req.tid
            ));
        }
        if let Some(d) = &self.trace.dentry {
            let comps = req.path.components();
            if req.path.ino != 0
                && comps.len() == 1
                && !comps[0].is_empty()
                && (d.dir.ino != req.path.ino || d.dir.snapid != CEPH_NOSNAP)
            {
                v.push(format!(
                    "trace dir {:#x} is not the request's parent {:#x}",
                    d.dir.ino, req.path.ino
                ));
            }
        }
        if let ReplyExtra::Readdir(r) = &self.extra {
            let max = req.head.args.readdir().max_entries;
            if max != 0 && r.entries.len() > max as usize {
                v.push(format!(
                    "readdir reply has {} entries, request allows {max}",
                    r.entries.len()
                ));
            }
        }
        v
    }
}

fn extra_kind(e: &ReplyExtra) -> &'static str {
    match e {
        ReplyExtra::None => "no extra",
        ReplyExtra::Readdir(_) => "a readdir extra",
        ReplyExtra::Create(_) => "a create extra",
        ReplyExtra::FileLock(_) => "a filelock extra",
        ReplyExtra::Vxattr(_) => "a getvxattr extra",
        ReplyExtra::Raw(_) => "a raw extra",
    }
}

impl MClientReply {
    /// Encode with every `InodeStat` at `inodestat_v` (this crate sends v9; Ceph 20.2 sends v8).
    pub fn encode_payload_inodestat_v(&self, inodestat_v: u8) -> Payload {
        let mut b = BytesMut::with_capacity(1024);
        b.put_u32_le(self.op.0);
        b.put_i32_le(self.result);
        b.put_u32_le(self.mdsmap_epoch);
        b.put_u8(u8::from(self.safe));
        b.put_u8(u8::from(self.trace.dentry.is_some()));
        b.put_u8(u8::from(self.trace.target.is_some()));
        put_section(&mut b, |b| self.trace.encode_into_v(b, inodestat_v));
        put_section(&mut b, |b| self.extra.encode_into_v(b, inodestat_v));
        self.snapbl.encode(&mut b, 0);
        Payload {
            front: b,
            ..Default::default()
        }
    }
}

/// Write `u32 len` + the bytes `f` appends.
fn put_section(buf: &mut BytesMut, f: impl FnOnce(&mut BytesMut)) {
    let pos = buf.len();
    buf.put_u32_le(0);
    f(buf);
    let len = (buf.len() - pos - 4) as u32;
    buf[pos..pos + 4].copy_from_slice(&len.to_le_bytes());
}

impl Message for MClientReply {
    const TYPE: u16 = msg_type::CLIENT_REPLY;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, _features: u64) -> Payload {
        self.encode_payload_inodestat_v(InodeStat::STRUCT_V)
    }

    fn decode_payload(
        _version: u16,
        mut front: Bytes,
        _middle: Bytes,
        _data: Bytes,
    ) -> Result<Self> {
        let p = &mut front;
        denc::need(p, 15, "ceph_mds_reply_head")?;
        let op = MdsOp(p.get_u32_le());
        let result = p.get_i32_le();
        let mdsmap_epoch = p.get_u32_le();
        let safe = p.get_u8() != 0;
        let is_dentry = p.get_u8() != 0;
        let is_target = p.get_u8() != 0;
        let trace_bl = get::<Bytes>(p, "MClientReply.trace")?;
        let extra_bl = get::<Bytes>(p, "MClientReply.extra")?;
        let snapbl = get::<Bytes>(p, "MClientReply.snapbl")?;
        if p.has_remaining() {
            return Err(invalid(
                "MClientReply",
                format!("{} trailing bytes", p.remaining()),
            ));
        }
        let trace = Trace::decode(trace_bl, is_dentry, is_target)?;
        let extra = ReplyExtra::parse(op, extra_bl);
        Ok(MClientReply {
            tid: 0,
            op,
            result,
            mdsmap_epoch,
            safe,
            trace,
            extra,
            snapbl,
        })
    }

    impl_from_raw!(tid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snap::SnapRealmInfo;
    use crate::types::{
        CEPH_CAP_FLAG_AUTH, CEPH_LEASE_PRIMARY_LINK, CEPH_LEASE_VALID, FilePath, cap, filelock,
    };
    use denc::{decode_exact, encode_to_bytes};

    pub(crate) fn dir_inode(ino: u64) -> InodeStat {
        InodeStat {
            ino,
            version: 4,
            cap: ReplyCap {
                caps: cap::PIN | cap::ANY_SHARED,
                wanted: 0,
                cap_id: 1,
                seq: 1,
                mseq: 0,
                realm: 1,
                flags: CEPH_CAP_FLAG_AUTH,
            },
            layout: FileLayout::default_for_pool(2),
            ctime: UTime { sec: 100, nsec: 1 },
            mtime: UTime { sec: 100, nsec: 1 },
            atime: UTime { sec: 100, nsec: 1 },
            mode: mode::S_IFDIR | 0o755,
            nlink: 1,
            nfiles: 1,
            rfiles: 1,
            rsubdirs: 1,
            rctime: UTime { sec: 100, nsec: 1 },
            dir_layout: DirLayout { dir_hash: 2 },
            btime: UTime { sec: 99, nsec: 0 },
            change_attr: 3,
            ..Default::default()
        }
    }

    pub(crate) fn file_inode(ino: u64) -> InodeStat {
        let mut x = XattrMap::new();
        x.insert(Bytes::from_static(b"user.k"), Bytes::from_static(b"v"));
        InodeStat {
            ino,
            version: 2,
            xattr_version: 1,
            cap: ReplyCap {
                caps: cap::PIN | cap::ANY_SHARED | cap::ANY_EXCL,
                wanted: 0x3f00,
                cap_id: 9,
                seq: 1,
                mseq: 0,
                realm: 1,
                flags: CEPH_CAP_FLAG_AUTH,
            },
            layout: FileLayout::default_for_pool(2),
            mode: mode::S_IFREG | 0o644,
            uid: 1000,
            gid: 1000,
            nlink: 1,
            size: 0,
            max_size: 4 << 20,
            truncate_seq: 1,
            truncate_size: u64::MAX,
            xattrs: Some(x),
            ..Default::default()
        }
    }

    #[test]
    fn inodestat_layout_and_round_trip() {
        let st = file_inode(0x100_0000_0001);
        let b = encode_to_bytes(&st, 0);
        assert_eq!(&b[..2], &[9, 1]);
        assert_eq!(
            u32::from_le_bytes(b[2..6].try_into().unwrap()) as usize,
            b.len() - 6
        );
        // The fixed part matches the kernel's packed ceph_mds_reply_inode (221 bytes incl. the
        // fragtree count).
        let body = &b[6..];
        assert_eq!(&body[0..8], &st.ino.to_le_bytes());
        assert_eq!(&body[8..16], &CEPH_NOSNAP.to_le_bytes());
        assert_eq!(&body[36..40], &st.cap.caps.to_le_bytes()); // cap starts after 8+8+4+8+8
        assert_eq!(body[36 + 32], CEPH_CAP_FLAG_AUTH);
        let mode_off = 8 + 8 + 4 + 8 + 8 + 33 + 28 + 24 + 4 + 24 + 4;
        assert_eq!(&body[mode_off..mode_off + 4], &st.mode.to_le_bytes());
        let ft_off = mode_off + 16 + 40 + 8;
        assert_eq!(ft_off + 4, 221);
        assert_eq!(&body[ft_off..ft_off + 4], &[0, 0, 0, 0]);
        assert_eq!(decode_exact::<InodeStat>(b, "st").unwrap(), st);
        let d = dir_inode(1);
        assert_eq!(
            decode_exact::<InodeStat>(encode_to_bytes(&d, 0), "st").unwrap(),
            d
        );
        assert!(d.check_invariants().is_empty());
        assert!(st.check_invariants().is_empty());
    }

    #[test]
    fn inodestat_tail_bytes() {
        // A head inode with nothing optional: check the v2+ tail byte for byte.
        let st = InodeStat {
            ino: 0x1000,
            mode: mode::S_IFREG,
            nlink: 1,
            ..Default::default()
        };
        let b = encode_to_bytes(&st, 0);
        let tail_len = 4 + 8 + 8 + 4 + 2 + 4 + 4 + 10 + 8;
        let tail = &b[b.len() - tail_len..];
        let want = "ffffffff".to_owned() // dir_pin -1
            + "0000000000000000" // snap_btime
            + "0000000000000000" // rsnaps
            + "00000000" // snap_metadata
            + "00" // fscrypt flag
            + "00000000" // fscrypt_auth
            + "00000000" // fscrypt_file
            + "01010400000000000000" // optmetadata: ENCODE_START(1,1) len 4, count 0
            + "0000000000000000"; // subvolume_id
        assert_eq!(hex::encode(&tail[1..]), want); // tail[0] is the last byte of change_attr
        // xattrs not included: u32 0; inline: CEPH_INLINE_NONE + empty; quota 22 bytes.
        let x_off = 6 + 221 + 4 + 8; // fragtree count included in 221; symlink len, dir_layout
        assert_eq!(&b[x_off..x_off + 4], &[0, 0, 0, 0]);
        assert_eq!(&b[x_off + 4..x_off + 12], &u64::MAX.to_le_bytes());
        // v8 omits only subvolume_id.
        let mut v8 = BytesMut::new();
        st.encode_struct_v(&mut v8, 8);
        assert_eq!(v8.len() + 8, b.len());
        assert_eq!(&v8[..2], &[8, 1]);
        let back = decode_exact::<InodeStat>(v8.freeze(), "v8").unwrap();
        assert_eq!(back, st);
        // v1 decodes with dir_pin -ENODATA.
        let mut v1 = BytesMut::new();
        st.encode_struct_v(&mut v1, 1);
        assert_eq!(
            decode_exact::<InodeStat>(v1.freeze(), "v1")
                .unwrap()
                .dir_pin,
            -61
        );
    }

    #[test]
    fn inodestat_future_version_is_skipped() {
        let st = file_inode(0x2000);
        let mut b = BytesMut::new();
        denc::encode_versioned(&mut b, 12, 1, |b| {
            let mut inner = BytesMut::new();
            st.encode(&mut inner, 0);
            b.put_slice(&inner[6..]);
            b.put_slice(&[0xab; 9]); // fields from a newer MDS
        });
        let mut bytes = b.freeze();
        assert_eq!(InodeStat::decode(&mut bytes).unwrap(), st);
        assert!(!bytes.has_remaining());
        // compat 2 is refused.
        let mut b = BytesMut::from(&encode_to_bytes(&st, 0)[..]);
        b[1] = 10;
        assert!(InodeStat::decode(&mut b.freeze()).is_err());
    }

    #[test]
    fn dirstat_lease_openc_bytes() {
        let ds = DirStat::root(0);
        assert_eq!(
            hex::encode(encode_to_bytes(&ds, 0)),
            "01010c000000000000000000000000000000"
        );
        let mut ds2 = DirStat::default();
        ds2.dist.insert(1);
        ds2.dist.insert(0);
        let b = encode_to_bytes(&ds2, 0);
        assert_eq!(
            hex::encode(&b),
            "010114000000".to_owned()
                + "00000000"
                + "ffffffff"
                + "02000000"
                + "00000000"
                + "01000000"
        );
        assert_eq!(decode_exact::<DirStat>(b, "ds").unwrap(), ds2);
        let l = LeaseStat::null();
        assert_eq!(
            hex::encode(encode_to_bytes(&l, 0)),
            "02010e000000".to_owned() + "0000" + "00000000" + "00000000" + "00000000"
        );
        let l = LeaseStat {
            mask: CEPH_LEASE_VALID | CEPH_LEASE_PRIMARY_LINK,
            duration_ms: 30000,
            seq: 1,
            alternate_name: Bytes::new(),
        };
        assert_eq!(
            decode_exact::<LeaseStat>(encode_to_bytes(&l, 0), "l").unwrap(),
            l
        );
        let o = OpencResponse {
            created_ino: 0x100_0000_0000,
            delegated_inos: vec![],
        };
        assert_eq!(
            hex::encode(encode_to_bytes(&o, 0)),
            "01010c000000".to_owned() + "0000000000010000" + "00000000"
        );
        let o = OpencResponse {
            created_ino: 1,
            delegated_inos: vec![(1, 10)],
        };
        assert_eq!(
            decode_exact::<OpencResponse>(encode_to_bytes(&o, 0), "o").unwrap(),
            o
        );
    }

    fn lookup_reply() -> (MClientRequest, MClientReply) {
        let mut req = MClientRequest::new(77, MdsOp::LOOKUP);
        req.path = FilePath::new(1, Bytes::from_static(b"f"));
        let mut r = MClientReply::for_request(&req, 0);
        r.mdsmap_epoch = 12;
        r.trace = Trace {
            dentry: Some(TraceDentry {
                dir: dir_inode(1),
                dirstat: DirStat::root(0),
                name: Bytes::from_static(b"f"),
                lease: LeaseStat::null(),
            }),
            target: Some(file_inode(0x100_0000_0000)),
        };
        r.set_snap_trace(
            &SnapTrace::new(vec![SnapRealmInfo::new(1, 0, 1)]),
            SnapFormat::Old,
        );
        (req, r)
    }

    #[test]
    fn reply_round_trip_and_head() {
        let (req, r) = lookup_reply();
        let raw = r.to_raw(0);
        assert_eq!((raw.version, raw.compat_version, raw.tid), (1, 0, 77));
        assert_eq!(
            &raw.front[..15],
            &[0, 1, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 1, 1, 1]
        );
        let back = MClientReply::from_raw(raw.clone()).unwrap();
        assert_eq!(back, r);
        assert!(
            back.check_invariants_for(&req).is_empty(),
            "{:?}",
            back.check_invariants_for(&req)
        );
        // Trailing bytes after the snap blob are refused.
        let mut f = BytesMut::from(&raw.front[..]);
        f.put_u8(0);
        assert!(MClientReply::decode_payload(1, f.freeze(), Bytes::new(), Bytes::new()).is_err());
        // A trace with a trailing byte is refused (the kernel requires exact consumption).
        let mut t = BytesMut::new();
        r.trace.encode_into(&mut t);
        t.put_u8(0);
        assert!(Trace::decode(t.freeze(), true, true).is_err());
    }

    #[test]
    fn errors_and_null_dentry() {
        let mut req = MClientRequest::new(5, MdsOp::LOOKUP);
        req.path = FilePath::new(1, Bytes::from_static(b"missing"));
        let mut r = MClientReply::for_request(&req, -errno::ENOENT);
        r.trace.dentry = Some(TraceDentry {
            dir: dir_inode(1),
            dirstat: DirStat::root(0),
            name: Bytes::from_static(b"missing"),
            lease: LeaseStat::null(),
        });
        let raw = r.to_raw(0);
        assert_eq!(&raw.front[4..8], &(-2i32).to_le_bytes());
        assert_eq!(&raw.front[13..15], &[1, 0]);
        assert_eq!(MClientReply::from_raw(raw).unwrap(), r);
        assert!(r.check_invariants_for(&req).is_empty());
        // Wrong parent directory in the trace.
        let mut bad = r.clone();
        bad.trace.dentry.as_mut().unwrap().dir.ino = 0x1000;
        assert!(!bad.check_invariants_for(&req).is_empty());
    }

    #[test]
    fn readdir_extra_and_order() {
        let names: Vec<&[u8]> = vec![b"a", b"b", b"c", b"file1", b"file2", b"dir"];
        let mut entries: Vec<ReaddirEntry> = names
            .iter()
            .enumerate()
            .map(|(i, n)| ReaddirEntry {
                name: Bytes::copy_from_slice(n),
                lease: LeaseStat::null(),
                inode: file_inode(0x1000 + i as u64),
            })
            .collect();
        entries.sort_by(|a, b| {
            crate::frag::dentry_key_cmp(
                hash_dentry_name(2, &a.name),
                &a.name,
                hash_dentry_name(2, &b.name),
                &b.name,
            )
        });
        let mut req = MClientRequest::new(9, MdsOp::READDIR);
        req.path = FilePath::new(1, Bytes::new());
        req.head.args = crate::request::ReaddirArgs {
            frag: 0,
            max_entries: 6,
            max_bytes: 0,
            flags: 1,
            offset_hash: 0,
        }
        .to_args();
        let mut r = MClientReply::for_request(&req, 0);
        r.trace.target = Some(dir_inode(1));
        let rd = ReaddirReply {
            dirstat: DirStat::root(0),
            flags: ReaddirReply::flags_for(true, true, true),
            entries,
        };
        assert_eq!(rd.flags, 0x701);
        r.extra = ReplyExtra::Readdir(rd.clone());
        let raw = r.to_raw(0);
        let back = MClientReply::from_raw(raw).unwrap();
        assert_eq!(back, r);
        assert!(
            back.check_invariants_for(&req).is_empty(),
            "{:?}",
            back.check_invariants_for(&req)
        );
        // Extra layout: DirStat (18 bytes), count, flags.
        let eb = r.extra.to_bytes();
        assert_eq!(&eb[18..22], &6u32.to_le_bytes());
        assert_eq!(&eb[22..24], &0x701u16.to_le_bytes());
        // Out of order, too many entries and a missing extra are reported.
        let mut bad = r.clone();
        if let ReplyExtra::Readdir(x) = &mut bad.extra {
            x.entries.reverse();
        }
        assert!(
            bad.check_invariants()
                .iter()
                .any(|e| e.contains("hash order"))
        );
        req.head.args = crate::request::ReaddirArgs {
            max_entries: 2,
            flags: 1,
            ..Default::default()
        }
        .to_args();
        assert!(
            r.check_invariants_for(&req)
                .iter()
                .any(|e| e.contains("allows 2"))
        );
        let mut none = r.clone();
        none.extra = ReplyExtra::None;
        assert!(
            none.check_invariants()
                .iter()
                .any(|e| e.contains("without its readdir extra"))
        );
        // An empty LSSNAP needs FRAG_END.
        let mut ls = MClientReply::new(1, MdsOp::LSSNAP, 0);
        ls.extra = ReplyExtra::Readdir(ReaddirReply {
            dirstat: DirStat::default(),
            flags: 0,
            entries: vec![],
        });
        assert!(!ls.check_invariants().is_empty());
        ls.extra = ReplyExtra::Readdir(ReaddirReply {
            dirstat: DirStat::default(),
            flags: readdir_flags::FRAG_END,
            entries: vec![],
        });
        assert!(ls.check_invariants().is_empty());
    }

    #[test]
    fn readdir_fill_budgets() {
        use crate::request::ReaddirArgs;
        let entries: Vec<ReaddirEntry> = (0..10u64)
            .map(|i| ReaddirEntry {
                name: Bytes::from(format!("f{i}")),
                lease: LeaseStat::null(),
                inode: file_inode(0x1000 + i),
            })
            .collect();
        let per = ReaddirReply::entry_len(&entries[0].name, &entries[0].lease, &entries[0].inode);
        // max_entries limits the chunk; not at the end.
        let args = ReaddirArgs {
            max_entries: 4,
            flags: readdir_flags::REPLY_BITFLAGS,
            ..Default::default()
        };
        let r = ReaddirReply::fill(DirStat::root(0), entries.clone(), &args, 48, 65536, true);
        assert_eq!(r.entries.len(), 4);
        assert_eq!(
            r.flags,
            readdir_flags::HASH_ORDER | readdir_flags::OFFSET_HASH
        );
        // Everything fits: FRAG_END and FRAG_COMPLETE.
        let args = ReaddirArgs {
            flags: readdir_flags::REPLY_BITFLAGS,
            ..Default::default()
        };
        let r = ReaddirReply::fill(DirStat::root(0), entries.clone(), &args, 48, 65536, true);
        assert_eq!(r.entries.len(), 10);
        assert_eq!(r.flags, 0x701);
        // A resumed listing that reaches the end: FRAG_END only.
        let r = ReaddirReply::fill(
            DirStat::root(0),
            entries[7..].to_vec(),
            &args,
            48,
            65536,
            false,
        );
        assert_eq!(r.flags & 0x101, readdir_flags::FRAG_END);
        // The byte budget: room for three entries after the fixed part and the snap trace.
        let max_bytes = (18 + 6 + 48 + 3 * per + per / 2) as u32;
        let args = ReaddirArgs {
            max_bytes,
            ..Default::default()
        };
        let r = ReaddirReply::fill(DirStat::root(0), entries.clone(), &args, 48, 0, true);
        assert_eq!(r.entries.len(), 3);
        assert_eq!(r.flags, 0);
        assert!(r.to_extra_len() <= max_bytes as usize - 48);
        // A budget too small for any entry still returns one.
        let args = ReaddirArgs {
            max_bytes: 1,
            ..Default::default()
        };
        assert_eq!(
            ReaddirReply::fill(DirStat::root(0), entries.clone(), &args, 48, 0, true)
                .entries
                .len(),
            1
        );
        // Nothing left: an empty chunk with FRAG_END.
        let r = ReaddirReply::fill(DirStat::root(0), Vec::new(), &args, 48, 0, false);
        assert!(r.entries.is_empty() && r.flags & readdir_flags::FRAG_END != 0);
    }

    #[test]
    fn create_filelock_getvxattr_extras() {
        let mut r = MClientReply::new(3, MdsOp::CREATE, 0);
        r.trace.target = Some(file_inode(0x100_0000_0002));
        r.extra = ReplyExtra::Create(OpencResponse {
            created_ino: 0x100_0000_0002,
            delegated_inos: vec![],
        });
        assert_eq!(MClientReply::from_raw(r.to_raw(0)).unwrap(), r);
        let mut r = MClientReply::new(3, MdsOp::GETFILELOCK, 0);
        r.extra = ReplyExtra::FileLock(FileLock {
            lock_type: filelock::UNLOCK,
            ..Default::default()
        });
        assert_eq!(MClientReply::from_raw(r.to_raw(0)).unwrap(), r);
        let mut r = MClientReply::new(3, MdsOp::GETVXATTR, 0);
        r.extra = ReplyExtra::Vxattr(Bytes::from_static(b"{\"pool\":\"data\"}"));
        let eb = r.extra.to_bytes();
        assert_eq!(&eb[..2], &[1, 1]);
        assert_eq!(
            u32::from_le_bytes(eb[2..6].try_into().unwrap()) as usize,
            eb.len() - 6
        );
        assert_eq!(
            u32::from_le_bytes(eb[6..10].try_into().unwrap()) as usize,
            eb.len() - 10
        );
        assert_eq!(MClientReply::from_raw(r.to_raw(0)).unwrap(), r);
        // Extras only for the listed ops.
        let mut bad = MClientReply::new(3, MdsOp::MKDIR, 0);
        bad.extra = ReplyExtra::Create(OpencResponse::default());
        assert!(!bad.check_invariants().is_empty());
        // Unknown extra bytes on decode stay raw.
        assert_eq!(
            ReplyExtra::parse(MdsOp::MKNOD, Bytes::from_static(&[1, 2])),
            ReplyExtra::Raw(Bytes::from_static(&[1, 2]))
        );
        assert_eq!(
            ReplyExtra::parse(MdsOp::CREATE, Bytes::new()),
            ReplyExtra::None
        );
    }

    #[test]
    fn inode_invariants() {
        let mut d = dir_inode(0x200);
        d.nlink = 2;
        d.size = 4096;
        d.inline_version = 0;
        let v = d.check_invariants();
        assert_eq!(v.len(), 4, "{v:?}");
        let f = InodeStat {
            ino: 1 << 48,
            ..Default::default()
        };
        assert_eq!(f.check_invariants().len(), 1);
    }
}
