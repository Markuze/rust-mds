//! Plain value types shared by the engine's input, output and state.

use std::collections::BTreeMap;
use std::fmt;

use crate::consts::*;

/// An inode number (`inodeno_t`).
pub type Ino = u64;
/// A client id (`client_t`, the number of the client's entity name).
pub type ClientId = u64;
/// An xattr map: byte names to byte values, ordered like the C++
/// `std::map<std::string, bufferptr>` (`InodeStoreBase::xattr_map_t`).
pub type XattrMap = BTreeMap<Vec<u8>, Vec<u8>>;

/// A timestamp as carried on the wire (`utime_t` / `ceph_timespec`): unsigned
/// 32-bit seconds and nanoseconds. Ordered by (sec, nsec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Timestamp {
    pub sec: u32,
    pub nsec: u32,
}

impl Timestamp {
    pub const ZERO: Timestamp = Timestamp { sec: 0, nsec: 0 };

    pub fn new(sec: u32, nsec: u32) -> Timestamp {
        Timestamp { sec, nsec }
    }

    pub fn is_zero(&self) -> bool {
        self.sec == 0 && self.nsec == 0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:09}", self.sec, self.nsec)
    }
}

/// A file layout (`file_layout_t`, `src/include/fs_types.h`).
///
/// A layout with all zero sizes and `pool_id == -1` is "no layout"
/// (`file_layout_t()`); directories other than the root normally have none.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileLayout {
    pub stripe_unit: u32,
    pub stripe_count: u32,
    pub object_size: u32,
    pub pool_id: i64,
    pub pool_ns: String,
}

impl Default for FileLayout {
    fn default() -> Self {
        FileLayout::none()
    }
}

impl FileLayout {
    /// `file_layout_t()`: no layout.
    pub fn none() -> FileLayout {
        FileLayout {
            stripe_unit: 0,
            stripe_count: 0,
            object_size: 0,
            pool_id: -1,
            pool_ns: String::new(),
        }
    }

    /// `file_layout_t::get_default()` with the given pool: 4 MiB objects,
    /// 4 MiB stripe unit, stripe count 1 (`MDCache::gen_default_file_layout`).
    pub fn default_for_pool(pool_id: i64) -> FileLayout {
        FileLayout {
            stripe_unit: 1 << 22,
            stripe_count: 1,
            object_size: 1 << 22,
            pool_id,
            pool_ns: String::new(),
        }
    }

    /// `inode_t::has_layout()`.
    pub fn is_set(&self) -> bool {
        *self != FileLayout::none()
    }

    /// `file_layout_t::get_period()`.
    pub fn period(&self) -> u64 {
        u64::from(self.stripe_count) * u64::from(self.object_size)
    }

    /// `file_layout_t::is_valid()` (geometry only; the pool is checked by the
    /// engine against the file system's data pools, `Server::is_valid_layout`).
    pub fn is_valid_geometry(&self) -> bool {
        let m = CEPH_MIN_STRIPE_UNIT - 1;
        if self.stripe_unit == 0 || (self.stripe_unit & m) != 0 {
            return false;
        }
        if self.object_size == 0 || (self.object_size & m) != 0 {
            return false;
        }
        if self.object_size < self.stripe_unit || !self.object_size.is_multiple_of(self.stripe_unit)
        {
            return false;
        }
        self.stripe_count != 0
    }
}

/// Quota limits (`quota_info_t`). Valid iff both are non-negative; enabled iff
/// either is non-zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Quota {
    pub max_bytes: i64,
    pub max_files: i64,
}

impl Quota {
    pub fn is_valid(&self) -> bool {
        self.max_bytes >= 0 && self.max_files >= 0
    }
    pub fn is_enabled(&self) -> bool {
        self.max_bytes != 0 || self.max_files != 0
    }
}

/// Directory statistics (`frag_info_t` as kept in `inode_t::dirstat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DirStat {
    pub mtime: Timestamp,
    pub change_attr: u64,
    pub nfiles: i64,
    pub nsubdirs: i64,
}

impl DirStat {
    pub fn size(&self) -> i64 {
        self.nfiles + self.nsubdirs
    }
}

/// Recursive statistics (`nest_info_t` as kept in `inode_t::rstat`). A
/// directory counts itself in `rsubdirs`; a file or symlink has `rfiles = 1`
/// and `rbytes = size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rstat {
    pub rctime: Timestamp,
    pub rbytes: i64,
    pub rfiles: i64,
    pub rsubdirs: i64,
    pub rsnaps: i64,
}

impl Rstat {
    /// `nest_info_t::add_delta`-style accumulation: counters add, `rctime`
    /// takes the maximum.
    pub fn add(&mut self, d: &Rstat, sign: i64) {
        self.rbytes += sign * d.rbytes;
        self.rfiles += sign * d.rfiles;
        self.rsubdirs += sign * d.rsubdirs;
        self.rsnaps += sign * d.rsnaps;
        if d.rctime > self.rctime {
            self.rctime = d.rctime;
        }
    }
}

/// A request path (`filepath`): a base inode plus relative components. The
/// kernel client sends `(parent ino, name)` for namespace operations and a
/// bare inode number for inode operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FilePath {
    pub ino: Ino,
    pub names: Vec<Vec<u8>>,
}

impl FilePath {
    /// A path that names the inode itself (depth 0).
    pub fn ino(ino: Ino) -> FilePath {
        FilePath {
            ino,
            names: Vec::new(),
        }
    }

    /// A one-component path: `name` inside directory `dir`.
    pub fn child(dir: Ino, name: impl AsRef<[u8]>) -> FilePath {
        FilePath {
            ino: dir,
            names: vec![name.as_ref().to_vec()],
        }
    }

    /// A multi-component path relative to `base`, split at `/`.
    pub fn parse(base: Ino, path: &str) -> FilePath {
        FilePath {
            ino: base,
            names: path
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| s.as_bytes().to_vec())
                .collect(),
        }
    }

    pub fn depth(&self) -> usize {
        self.names.len()
    }

    pub fn last(&self) -> Option<&[u8]> {
        self.names.last().map(|v| v.as_slice())
    }
}

/// A half-open range of inode numbers `[start, start + len)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InoRange {
    pub start: Ino,
    pub len: u64,
}

impl InoRange {
    pub fn new(start: Ino, len: u64) -> InoRange {
        InoRange { start, len }
    }
    pub fn end(&self) -> Ino {
        self.start + self.len
    }
    pub fn contains(&self, ino: Ino) -> bool {
        ino >= self.start && ino < self.end()
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// File type helpers on a `mode` value.
pub fn is_dir(mode: u32) -> bool {
    mode & S_IFMT == S_IFDIR
}
pub fn is_reg(mode: u32) -> bool {
    mode & S_IFMT == S_IFREG
}
pub fn is_symlink(mode: u32) -> bool {
    mode & S_IFMT == S_IFLNK
}

/// The `d_type`-style file type bits of a mode (`mode & S_IFMT`).
pub fn file_type(mode: u32) -> u32 {
    mode & S_IFMT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_validity() {
        assert!(FileLayout::default_for_pool(3).is_valid_geometry());
        assert!(!FileLayout::none().is_set() || FileLayout::none().is_valid_geometry());
        let mut l = FileLayout::default_for_pool(1);
        l.stripe_unit = 1000;
        assert!(!l.is_valid_geometry());
        let mut l = FileLayout::default_for_pool(1);
        l.object_size = 1 << 16;
        assert!(!l.is_valid_geometry(), "object size below stripe unit");
        let mut l = FileLayout::default_for_pool(1);
        l.stripe_count = 0;
        assert!(!l.is_valid_geometry());
        assert_eq!(FileLayout::default_for_pool(1).period(), 4 << 20);
    }

    #[test]
    fn filepath_parse() {
        let p = FilePath::parse(1, "/a//b/c/");
        assert_eq!(p.names, vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
        assert_eq!(p.last(), Some(&b"c"[..]));
        assert_eq!(FilePath::ino(7).depth(), 0);
    }

    #[test]
    fn timestamp_order() {
        assert!(Timestamp::new(1, 5) < Timestamp::new(2, 0));
        assert!(Timestamp::new(2, 1) > Timestamp::new(2, 0));
    }
}
