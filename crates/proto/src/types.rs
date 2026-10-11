//! Constants and small wire structs shared by several CephFS messages (`include/ceph_fs.h`,
//! `include/rados.h`, `include/fs_types.h`, `include/filepath.h`, `include/cephfs/types.h`).

use std::collections::BTreeMap;
use std::fmt;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Result};

use crate::util::{Ctx, get, invalid};

/// `CEPH_NOSNAP`: the head (live) revision of an inode or dentry.
pub const CEPH_NOSNAP: u64 = u64::MAX - 1;
/// `CEPH_SNAPDIR`: the snapid of the virtual `.snap` directory.
pub const CEPH_SNAPDIR: u64 = u64::MAX;
/// `CEPH_MAXSNAP`: the largest valid snapshot id.
pub const CEPH_MAXSNAP: u64 = u64::MAX - 2;

/// `CEPH_INLINE_NONE`: `inline_version` of an inode without inline data. libcephfs requires a
/// non-zero `inline_version` in every `InodeStat`.
pub const CEPH_INLINE_NONE: u64 = u64::MAX;

/// `CEPH_INO_ROOT`.
pub const CEPH_INO_ROOT: u64 = 1;
/// `CEPH_INO_CEPH`: the hidden `.ceph` directory (never listed by readdir).
pub const CEPH_INO_CEPH: u64 = 2;
/// `CEPH_INO_GLOBAL_SNAPREALM`.
pub const CEPH_INO_GLOBAL_SNAPREALM: u64 = 3;
/// `CEPH_INO_LOST_AND_FOUND`.
pub const CEPH_INO_LOST_AND_FOUND: u64 = 4;
/// `MDS_INO_MDSDIR_OFFSET`: start of the MDS-private inode range.
pub const MDS_INO_PRIVATE_START: u64 = 0x100;
/// `MDS_INO_SYSTEM_BASE` / kernel `CEPH_INO_SYSTEM_BASE`: first inode number a client may see
/// after the base inodes. The kernel refuses `[0x100, 0x1000)` (`ceph_vino_is_reserved`).
pub const MDS_INO_SYSTEM_BASE: u64 = 0x1000;
/// Inode numbers must stay below 2^48 (ceph-fuse packs them into 48 bits).
pub const MAX_CLIENT_INO: u64 = 1 << 48;

/// Whether `ino` is in the MDS-private range `[0x100, 0x1000)` that must never reach a client.
pub const fn is_private_ino(ino: u64) -> bool {
    ino >= MDS_INO_PRIVATE_START && ino < MDS_INO_SYSTEM_BASE
}

/// `CEPH_LEASE_VALID`: a dentry lease is valid (old and new bit values).
pub const CEPH_LEASE_VALID: u16 = 1 | 2;
/// `CEPH_LEASE_PRIMARY_LINK`: the dentry is the inode's primary link.
pub const CEPH_LEASE_PRIMARY_LINK: u16 = 4;

/// `CEPH_CAP_FLAG_AUTH`: the cap is issued by the auth MDS.
pub const CEPH_CAP_FLAG_AUTH: u8 = 1;
/// `CEPH_CAP_FLAG_RELEASE`: ask the client to release the cap (import peer flags).
pub const CEPH_CAP_FLAG_RELEASE: u8 = 2;

/// Capability bits (`CEPH_CAP_*`, `include/ceph_fs.h`).
pub mod cap {
    pub const PIN: u32 = 1;
    pub const GSHARED: u32 = 1;
    pub const GEXCL: u32 = 2;
    pub const GCACHE: u32 = 4;
    pub const GRD: u32 = 8;
    pub const GWR: u32 = 16;
    pub const GBUFFER: u32 = 32;
    pub const GWREXTEND: u32 = 64;
    pub const GLAZYIO: u32 = 128;
    pub const SAUTH: u32 = 2;
    pub const SLINK: u32 = 4;
    pub const SXATTR: u32 = 6;
    pub const SFILE: u32 = 8;
    pub const AUTH_SHARED: u32 = GSHARED << SAUTH;
    pub const AUTH_EXCL: u32 = GEXCL << SAUTH;
    pub const LINK_SHARED: u32 = GSHARED << SLINK;
    pub const LINK_EXCL: u32 = GEXCL << SLINK;
    pub const XATTR_SHARED: u32 = GSHARED << SXATTR;
    pub const XATTR_EXCL: u32 = GEXCL << SXATTR;
    pub const FILE_SHARED: u32 = GSHARED << SFILE;
    pub const FILE_EXCL: u32 = GEXCL << SFILE;
    pub const FILE_CACHE: u32 = GCACHE << SFILE;
    pub const FILE_RD: u32 = GRD << SFILE;
    pub const FILE_WR: u32 = GWR << SFILE;
    pub const FILE_BUFFER: u32 = GBUFFER << SFILE;
    pub const FILE_WREXTEND: u32 = GWREXTEND << SFILE;
    pub const FILE_LAZYIO: u32 = GLAZYIO << SFILE;
    /// `CEPH_STAT_RSTAT`: getattr mask bit asking for recursive stats (`Fa`).
    pub const STAT_RSTAT: u32 = FILE_WREXTEND;
    pub const STAT_CAP_INODE_ALL: u32 = PIN | AUTH_SHARED | LINK_SHARED | FILE_SHARED | XATTR_SHARED;
    pub const ANY_SHARED: u32 = AUTH_SHARED | LINK_SHARED | XATTR_SHARED | FILE_SHARED;
    pub const ANY_RD: u32 = ANY_SHARED | FILE_RD | FILE_CACHE;
    pub const ANY_EXCL: u32 = AUTH_EXCL | LINK_EXCL | XATTR_EXCL | FILE_EXCL;
    pub const ANY_FILE_RD: u32 = FILE_RD | FILE_CACHE | FILE_SHARED;
    pub const ANY_FILE_WR: u32 = FILE_WR | FILE_BUFFER | FILE_EXCL;
    pub const ANY_WR: u32 = ANY_EXCL | ANY_FILE_WR;
    pub const ANY: u32 = ANY_RD | ANY_EXCL | ANY_FILE_WR | FILE_LAZYIO | PIN;
    /// Async dirop caps on directories (never granted in P0).
    pub const DIR_CREATE: u32 = FILE_CACHE;
    pub const DIR_UNLINK: u32 = FILE_RD;
    pub const ANY_DIR_OPS: u32 = FILE_CACHE | FILE_RD | FILE_WREXTEND | FILE_LAZYIO;

    fn gcap_string(c: u32, s: &mut String) {
        for (bit, ch) in [
            (GSHARED, 's'),
            (GEXCL, 'x'),
            (GCACHE, 'c'),
            (GRD, 'r'),
            (GWR, 'w'),
            (GBUFFER, 'b'),
            (GWREXTEND, 'a'),
            (GLAZYIO, 'l'),
        ] {
            if c & bit != 0 {
                s.push(ch);
            }
        }
    }

    /// `ccap_string`: the C++ log form of a cap mask, e.g. `pAsLsXsFs`; `-` for none.
    pub fn cap_string(caps: u32) -> String {
        let mut s = String::new();
        if caps & PIN != 0 {
            s.push('p');
        }
        for (shift, ch, mask) in [(SAUTH, 'A', 3), (SLINK, 'L', 3), (SXATTR, 'X', 3), (SFILE, 'F', u32::MAX)] {
            let a = (caps >> shift) & mask;
            if a != 0 {
                s.push(ch);
                gcap_string(a, &mut s);
            }
        }
        if s.is_empty() {
            s.push('-');
        }
        s
    }
}

/// Open flags on the wire (`CEPH_O_*`, octal like asm-generic Linux).
pub mod open_flags {
    pub const RDONLY: u32 = 0o0;
    pub const WRONLY: u32 = 0o1;
    pub const RDWR: u32 = 0o2;
    pub const CREAT: u32 = 0o100;
    pub const EXCL: u32 = 0o200;
    pub const TRUNC: u32 = 0o1000;
    pub const LAZY: u32 = 0o20000;
    pub const DIRECTORY: u32 = 0o200000;
    pub const NOFOLLOW: u32 = 0o400000;
}

/// `CEPH_SETATTR_*` mask bits of `args.setattr.mask`.
pub mod setattr {
    pub const MODE: u32 = 1 << 0;
    pub const UID: u32 = 1 << 1;
    pub const GID: u32 = 1 << 2;
    pub const MTIME: u32 = 1 << 3;
    pub const ATIME: u32 = 1 << 4;
    pub const SIZE: u32 = 1 << 5;
    pub const CTIME: u32 = 1 << 6;
    pub const MTIME_NOW: u32 = 1 << 7;
    pub const ATIME_NOW: u32 = 1 << 8;
    pub const BTIME: u32 = 1 << 9;
    pub const KILL_SGUID: u32 = 1 << 10;
    pub const FSCRYPT_AUTH: u32 = 1 << 11;
    pub const FSCRYPT_FILE: u32 = 1 << 12;
    pub const KILL_SUID: u32 = 1 << 13;
    pub const KILL_SGID: u32 = 1 << 14;
}

/// `CEPH_XATTR_*` flags of `args.setxattr.flags`.
pub mod xattr_flags {
    pub const CREATE: u32 = 1 << 0;
    pub const REPLACE: u32 = 1 << 1;
    pub const REMOVE2: u32 = 1 << 30;
    pub const REMOVE: u32 = 1 << 31;
}

/// Readdir request flag (`args.readdir.flags`) and reply flags (`CEPH_READDIR_*`).
pub mod readdir_flags {
    /// Request: the client understands the reply bit flags below.
    pub const REPLY_BITFLAGS: u16 = 1 << 0;
    pub const FRAG_END: u16 = 1 << 0;
    pub const FRAG_COMPLETE: u16 = 1 << 8;
    pub const HASH_ORDER: u16 = 1 << 9;
    pub const OFFSET_HASH: u16 = 1 << 10;
}

/// File lock rules and types (`CEPH_LOCK_*` of `filelock_change`; the kernel's numbering of
/// these two sets matches userspace, unlike the metadata lock types).
pub mod filelock {
    pub const RULE_FCNTL: u8 = 1;
    pub const RULE_FLOCK: u8 = 2;
    pub const RULE_FCNTL_INTR: u8 = 3;
    pub const RULE_FLOCK_INTR: u8 = 4;
    pub const SHARED: u8 = 1;
    pub const EXCL: u8 = 2;
    pub const UNLOCK: u8 = 4;
}

/// `file_layout_t` (`include/fs_types.h`). On the client wire it travels as the legacy 28-byte
/// `ceph_file_layout` (`to_legacy`/`from_legacy`, `common/fs_types.cc`) plus a separate
/// `pool_ns` string.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileLayout {
    pub stripe_unit: u32,
    pub stripe_count: u32,
    pub object_size: u32,
    /// RADOS pool id; -1 = none.
    pub pool_id: i64,
    pub pool_ns: String,
}

impl Default for FileLayout {
    /// `file_layout_t()`: all zero, `pool_id` -1.
    fn default() -> Self {
        FileLayout { stripe_unit: 0, stripe_count: 0, object_size: 0, pool_id: -1, pool_ns: String::new() }
    }
}

impl FileLayout {
    /// Size of the legacy `ceph_file_layout`.
    pub const LEGACY_LEN: usize = 28;

    /// `file_layout_t::get_default()` with the given pool: 4 MiB objects, one stripe.
    pub fn default_for_pool(pool_id: i64) -> Self {
        FileLayout { stripe_unit: 1 << 22, stripe_count: 1, object_size: 1 << 22, pool_id, pool_ns: String::new() }
    }

    /// `to_legacy` + raw encode: stripe_unit, stripe_count, object_size, cas_hash 0,
    /// object_stripe_unit 0, unused 0, pg_pool (0 when `pool_id` < 0).
    pub fn encode_legacy(&self, buf: &mut BytesMut) {
        buf.put_u32_le(self.stripe_unit);
        buf.put_u32_le(self.stripe_count);
        buf.put_u32_le(self.object_size);
        buf.put_u32_le(0);
        buf.put_u32_le(0);
        buf.put_u32_le(0);
        buf.put_u32_le(if self.pool_id >= 0 { self.pool_id as u32 } else { 0 });
    }

    /// Raw decode + `from_legacy`: `pool_id` is the signed 32-bit `fl_pg_pool`, and an
    /// all-zero layout means "no layout" (`pool_id` -1). `pool_ns` is left empty.
    pub fn decode_legacy(buf: &mut Bytes) -> Result<Self> {
        denc::need(buf, Self::LEGACY_LEN, "ceph_file_layout")?;
        let stripe_unit = buf.get_u32_le();
        let stripe_count = buf.get_u32_le();
        let object_size = buf.get_u32_le();
        buf.advance(12); // cas_hash, object_stripe_unit, unused
        let mut pool_id = buf.get_u32_le() as i32 as i64;
        if pool_id == 0 && stripe_unit == 0 && stripe_count == 0 && object_size == 0 {
            pool_id = -1;
        }
        Ok(FileLayout { stripe_unit, stripe_count, object_size, pool_id, pool_ns: String::new() })
    }
}

/// `ceph_dir_layout`: 8 raw bytes, of which only `dl_dir_hash` is used (the name hash
/// function, [`crate::hash`]). Directories created by the C++ MDS carry 2 (rjenkins).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DirLayout {
    pub dir_hash: u8,
}

impl DirLayout {
    pub const LEN: usize = 8;
}

impl Encode for DirLayout {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u8(self.dir_hash);
        buf.put_slice(&[0; 7]);
    }
}

impl Decode for DirLayout {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        denc::need(buf, Self::LEN, "ceph_dir_layout")?;
        let dir_hash = buf.get_u8();
        buf.advance(7);
        Ok(DirLayout { dir_hash })
    }
}

/// `filepath` (`include/filepath.cc`): `u8 struct_v` (1), `u64 ino`, `string path`. `ino` is the
/// base inode the path is relative to; 0 with an absolute path means "from the root". Empty
/// path components are kept (an empty first component names the snapdir).
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct FilePath {
    pub ino: u64,
    pub path: Bytes,
}

impl FilePath {
    pub fn new(ino: u64, path: impl Into<Bytes>) -> Self {
        FilePath { ino, path: path.into() }
    }

    pub fn is_empty(&self) -> bool {
        self.ino == 0 && self.path.is_empty()
    }

    /// The path components as C++ `filepath::parse_bits` splits a path decoded from the wire:
    /// on `/`, keeping empty components (a trailing `/` adds none). An empty **first** component
    /// names the snapdir (`/s1/b` relative to an inode is `.snap/s1/b`); the C++ MDS refuses an
    /// empty component anywhere else with `-EINVAL` (`MDCache::path_traverse`).
    pub fn components(&self) -> Vec<&[u8]> {
        let p: &[u8] = &self.path;
        if p.is_empty() {
            return Vec::new();
        }
        let p = p.strip_suffix(b"/").unwrap_or(p);
        p.split(|c| *c == b'/').collect()
    }

    /// Whether the first component is the snapdir (an empty component).
    pub fn starts_with_snapdir(&self) -> bool {
        self.components().first().is_some_and(|c| c.is_empty())
    }
}

impl fmt::Debug for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:x}/{}", self.ino, String::from_utf8_lossy(&self.path))
    }
}

impl Encode for FilePath {
    fn encode(&self, buf: &mut BytesMut, features: u64) {
        buf.put_u8(1);
        buf.put_u64_le(self.ino);
        self.path.encode(buf, features);
    }
}

impl Decode for FilePath {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let _struct_v = get::<u8>(buf, "filepath.struct_v")?;
        let ino = get::<u64>(buf, "filepath.ino")?;
        let path = get::<Bytes>(buf, "filepath.path")?;
        Ok(FilePath { ino, path })
    }
}

/// `quota_info_t`: `ENCODE_START(1, 1)` { `i64 max_bytes`, `i64 max_files` } (0 = no limit).
/// The kernel reads it at fixed offsets in `MClientQuota`, so it is always exactly 22 bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct QuotaInfo {
    pub max_bytes: i64,
    pub max_files: i64,
}

impl QuotaInfo {
    pub fn is_enabled(&self) -> bool {
        self.max_bytes != 0 || self.max_files != 0
    }
}

impl Encode for QuotaInfo {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        denc::encode_versioned(buf, 1, 1, |b| {
            b.put_i64_le(self.max_bytes);
            b.put_i64_le(self.max_files);
        });
    }
}

impl Decode for QuotaInfo {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut f = denc::decode_start(buf, 1, "quota_info_t")?;
        let max_bytes = get::<i64>(&mut f.body, "quota_info_t.max_bytes")?;
        let max_files = get::<i64>(&mut f.body, "quota_info_t.max_files")?;
        Ok(QuotaInfo { max_bytes, max_files })
    }
}

/// `ceph_filelock` (41 raw bytes): a byte-range or flock lock, as carried by GETFILELOCK
/// replies and reconnect lock blobs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FileLock {
    pub start: u64,
    /// 0 = to the end of the file.
    pub length: u64,
    pub client: u64,
    /// Lock owner; the kernel sets bit 63 ("the owner is sufficient").
    pub owner: u64,
    pub pid: u64,
    /// [`filelock::SHARED`], [`filelock::EXCL`] or [`filelock::UNLOCK`].
    pub lock_type: u8,
}

impl FileLock {
    pub const LEN: usize = 41;
}

impl Encode for FileLock {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u64_le(self.start);
        buf.put_u64_le(self.length);
        buf.put_u64_le(self.client);
        buf.put_u64_le(self.owner);
        buf.put_u64_le(self.pid);
        buf.put_u8(self.lock_type);
    }
}

impl Decode for FileLock {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        denc::need(buf, Self::LEN, "ceph_filelock")?;
        Ok(FileLock {
            start: buf.get_u64_le(),
            length: buf.get_u64_le(),
            client: buf.get_u64_le(),
            owner: buf.get_u64_le(),
            pid: buf.get_u64_le(),
            lock_type: buf.get_u8(),
        })
    }
}

/// An inode's extended attributes, name -> value, in C++ `std::map` order (bytewise).
pub type XattrMap = BTreeMap<Bytes, Bytes>;

/// Encode an xattr map the way C++ encodes `map<string, bufferptr>`: `u32 count` + count x
/// (`string name`, `u32 len` + value). This is the cap-message middle segment, the inner blob
/// of `InodeStat.xattrs`, and the initial-xattr payload of create requests.
pub fn encode_xattrs(xattrs: &XattrMap, buf: &mut BytesMut) {
    buf.put_u32_le(xattrs.len() as u32);
    for (k, v) in xattrs {
        k.encode(buf, 0);
        v.encode(buf, 0);
    }
}

/// Encode an xattr map into a fresh blob (see [`encode_xattrs`]).
pub fn xattrs_to_blob(xattrs: &XattrMap) -> Bytes {
    let mut b = BytesMut::new();
    encode_xattrs(xattrs, &mut b);
    b.freeze()
}

/// Decode an xattr blob that must be consumed exactly (see [`encode_xattrs`]).
pub fn decode_xattr_blob(mut blob: Bytes) -> Result<XattrMap> {
    let n = get::<u32>(&mut blob, "xattrs.count")?;
    let mut m = XattrMap::new();
    for _ in 0..n {
        let k = get::<Bytes>(&mut blob, "xattrs.name")?;
        let v = get::<Bytes>(&mut blob, "xattrs.value")?;
        m.insert(k, v);
    }
    if blob.has_remaining() {
        return Err(invalid("xattrs", format!("{} trailing bytes", blob.remaining())));
    }
    Ok(m)
}

/// Decode a `u32 n` + n x `T` sequence whose count may come from an untrusted peer.
pub(crate) fn decode_counted<T: Decode>(buf: &mut Bytes, n: usize, what: &'static str) -> Result<Vec<T>> {
    let mut v = Vec::with_capacity(n.min(buf.remaining()));
    for _ in 0..n {
        v.push(T::decode(buf).ctx(what)?);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::{decode_exact, encode_to_bytes};

    #[test]
    fn cap_strings_match_cpp() {
        assert_eq!(cap::cap_string(0), "-");
        assert_eq!(cap::cap_string(cap::PIN), "p");
        assert_eq!(cap::cap_string(cap::STAT_CAP_INODE_ALL), "pAsLsXsFs");
        assert_eq!(
            cap::cap_string(
                cap::PIN
                    | cap::ANY_EXCL
                    | cap::ANY_SHARED
                    | cap::FILE_CACHE
                    | cap::FILE_RD
                    | cap::FILE_WR
                    | cap::FILE_BUFFER
            ),
            "pAsxLsxXsxFsxcrwb"
        );
        assert_eq!(cap::ANY, 0xbffd);
    }

    #[test]
    fn layout_legacy_round_trip() {
        let l = FileLayout::default_for_pool(3);
        let mut b = BytesMut::new();
        l.encode_legacy(&mut b);
        assert_eq!(hex::encode(&b), "00004000010000000000400000000000000000000000000003000000");
        assert_eq!(FileLayout::decode_legacy(&mut b.freeze()).unwrap(), l);
        // The zero layout means "none" and maps back to zeros.
        let none = FileLayout::default();
        let mut b = BytesMut::new();
        none.encode_legacy(&mut b);
        assert_eq!(&b[..], &[0u8; 28]);
        assert_eq!(FileLayout::decode_legacy(&mut b.freeze()).unwrap().pool_id, -1);
    }

    #[test]
    fn filepath_wire_and_components() {
        let p = FilePath::new(0x1000, Bytes::from_static(b"a/b"));
        let b = encode_to_bytes(&p, 0);
        assert_eq!(hex::encode(&b), "01001000000000000003000000612f62");
        assert_eq!(decode_exact::<FilePath>(b, "fp").unwrap(), p);
        // As C++ parse_bits with encoded = true.
        assert_eq!(FilePath::new(1, Bytes::from_static(b"/s1/b")).components(), vec![&b""[..], b"s1", b"b"]);
        assert!(FilePath::new(1, Bytes::from_static(b"/s1/b")).starts_with_snapdir());
        assert_eq!(FilePath::new(1, Bytes::from_static(b"foo//bar")).components(), vec![&b"foo"[..], b"", b"bar"]);
        assert_eq!(FilePath::new(1, Bytes::from_static(b"a/")).components(), vec![&b"a"[..]]);
        assert_eq!(FilePath::new(1, Bytes::from_static(b"/")).components(), vec![&b""[..]]);
        assert_eq!(FilePath::new(1, Bytes::from_static(b"f")).components(), vec![&b"f"[..]]);
        assert!(!FilePath::new(1, Bytes::from_static(b"f")).starts_with_snapdir());
        assert!(FilePath::new(1, Bytes::new()).components().is_empty());
    }

    #[test]
    fn quota_and_filelock() {
        let q = QuotaInfo { max_bytes: 1 << 30, max_files: 7 };
        let b = encode_to_bytes(&q, 0);
        assert_eq!(hex::encode(&b), "0101".to_owned() + "10000000" + "0000004000000000" + "0700000000000000");
        assert_eq!(decode_exact::<QuotaInfo>(b, "q").unwrap(), q);
        let l = FileLock { start: 1, length: 2, client: 3, owner: 1 << 63, pid: 5, lock_type: filelock::EXCL };
        let b = encode_to_bytes(&l, 0);
        assert_eq!(b.len(), FileLock::LEN);
        assert_eq!(decode_exact::<FileLock>(b, "l").unwrap(), l);
    }

    #[test]
    fn xattr_blob() {
        let mut m = XattrMap::new();
        m.insert(Bytes::from_static(b"user.b"), Bytes::from_static(b""));
        m.insert(Bytes::from_static(b"user.a"), Bytes::from_static(b"xy"));
        let blob = xattrs_to_blob(&m);
        assert_eq!(
            hex::encode(&blob),
            "02000000".to_owned() + "06000000757365722e61" + "020000007879" + "06000000757365722e62" + "00000000"
        );
        assert_eq!(decode_xattr_blob(blob.clone()).unwrap(), m);
        let mut trailing = BytesMut::from(&blob[..]);
        trailing.put_u8(0);
        assert!(decode_xattr_blob(trailing.freeze()).is_err());
        assert!(is_private_ino(0x100) && is_private_ino(0xfff) && !is_private_ino(0x1000) && !is_private_ino(1));
    }
}
