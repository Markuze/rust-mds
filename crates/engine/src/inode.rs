//! Inodes, directory entries and the per-inode runtime state (caps, holds,
//! file locks).

use std::collections::{BTreeMap, BTreeSet};

use crate::caps::Capability;
use crate::consts::*;
use crate::filelock::LockTable;
use crate::hash::dentry_hash24;
use crate::types::*;

/// Snapshot-realm node of a realm-root inode (`sr_t`, reduced to what P0
/// needs: realms exist for the root, quota directories and subvolume roots).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SnapRealmNode {
    pub seq: u64,
    pub created: u64,
    /// `sr_t::SUBVOLUME`: the directory is a subvolume root (a shard root).
    pub subvolume: bool,
    pub last_modified: Timestamp,
    pub change_attr: u64,
}

/// The persistent part of an inode: the fields clients see in `InodeStat`
/// plus what the MDS keeps (`inode_t`, `src/include/cephfs/types.h`). Journal
/// records carry full images of this struct.
#[derive(Debug, Clone, PartialEq)]
pub struct InodeCore {
    pub ino: Ino,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    /// Link count. Directories keep 1 while linked and 0 once removed;
    /// clients present `2 + nsubdirs`.
    pub nlink: i32,
    pub rdev: u32,
    pub size: u64,
    pub max_size_ever: u64,
    /// Starts at 1 for new inodes; bumped by every shrinking truncation.
    pub truncate_seq: u32,
    /// `u64::MAX` until the first truncation.
    pub truncate_size: u64,
    pub truncate_from: u64,
    pub ctime: Timestamp,
    pub mtime: Timestamp,
    pub atime: Timestamp,
    pub btime: Timestamp,
    pub change_attr: u64,
    /// Bumped on every change; clients see `2 * version`.
    pub version: u64,
    pub time_warp_seq: u32,
    pub layout: FileLayout,
    pub old_pools: BTreeSet<i64>,
    /// `dir_layout.dl_dir_hash` (directories: `CEPH_STR_HASH_RJENKINS`).
    pub dir_hash: u8,
    pub xattrs: XattrMap,
    pub xattr_version: u64,
    pub symlink: Vec<u8>,
    pub dirstat: DirStat,
    pub rstat: Rstat,
    pub quota: Quota,
    pub export_pin: i32,
    pub export_pin_random: f64,
    pub export_pin_distributed: bool,
    pub inline_version: u64,
    /// Writeable ranges per client (`client_ranges`); the value is the
    /// `max_size` the client may write up to.
    pub client_ranges: BTreeMap<ClientId, u64>,
    pub snaprealm: Option<SnapRealmNode>,
    pub fscrypt_auth: Vec<u8>,
    pub fscrypt_file: Vec<u8>,
}

impl InodeCore {
    /// A fresh inode as `Server::prepare_new_inode` builds it.
    pub fn new(ino: Ino, mode: u32, uid: u32, gid: u32, stamp: Timestamp) -> InodeCore {
        InodeCore {
            ino,
            mode,
            uid,
            gid,
            nlink: 1,
            rdev: 0,
            size: 0,
            max_size_ever: 0,
            truncate_seq: 1,
            truncate_size: u64::MAX,
            truncate_from: 0,
            ctime: stamp,
            mtime: stamp,
            atime: stamp,
            btime: stamp,
            change_attr: 0,
            version: 1,
            time_warp_seq: 0,
            layout: FileLayout::none(),
            old_pools: BTreeSet::new(),
            dir_hash: if is_dir(mode) {
                CEPH_STR_HASH_RJENKINS
            } else {
                0
            },
            xattrs: XattrMap::new(),
            xattr_version: 1,
            symlink: Vec::new(),
            dirstat: DirStat::default(),
            rstat: Rstat::default(),
            quota: Quota::default(),
            export_pin: MDS_RANK_NONE,
            export_pin_random: 0.0,
            export_pin_distributed: false,
            inline_version: CEPH_INLINE_NONE,
            client_ranges: BTreeMap::new(),
            snaprealm: None,
            fscrypt_auth: Vec::new(),
            fscrypt_file: Vec::new(),
        }
    }

    pub fn is_dir(&self) -> bool {
        is_dir(self.mode)
    }
    pub fn is_file(&self) -> bool {
        is_reg(self.mode)
    }
    pub fn is_symlink(&self) -> bool {
        is_symlink(self.mode)
    }

    /// The largest writeable range of any client (`inode_t::get_max_size`).
    pub fn max_client_range(&self) -> u64 {
        self.client_ranges.values().copied().max().unwrap_or(0)
    }

    /// Total xattr bytes as counted against `max_xattr_size` (names plus
    /// values, no framing).
    pub fn xattr_bytes(&self) -> u64 {
        self.xattrs
            .iter()
            .map(|(k, v)| (k.len() + v.len()) as u64)
            .sum()
    }
}

/// The 24-bit-hash-then-name key that orders a directory.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DentryKey {
    pub hash: u32,
    pub name: Vec<u8>,
}

impl DentryKey {
    pub fn new(dir_hash: u8, name: &[u8]) -> DentryKey {
        DentryKey {
            hash: dentry_hash24(dir_hash, name),
            name: name.to_vec(),
        }
    }
}

/// A linked directory entry. `primary` is the inode's primary link; other
/// links of the same inode are remote (hard links).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dentry {
    pub ino: Ino,
    pub primary: bool,
}

/// What a temporary restriction keeps clients from holding while a request
/// prepares or commits (the client-visible outcome of C++ rd/wr/xlocks).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldKind {
    /// Fresh size/mtime are needed (`filelock` rdlock): nobody may hold
    /// `Fx Fw Fb`.
    FileRead,
    /// Size/times change (`filelock` xlock): everybody keeps at most `Fc Fb`.
    FileXlock,
    /// Fresh mode/uid/gid needed: nobody may hold `Ax`.
    AuthRead,
    /// Mode/uid/gid/btime change: nobody may hold `As Ax`.
    AuthXlock,
    /// Fresh xattrs needed: nobody may hold `Xx`.
    XattrRead,
    /// xattrs change: nobody may hold `Xs Xx`.
    XattrXlock,
    /// Link count changes: nobody may hold `Ls Lx`.
    LinkXlock,
    /// A namespace change in this directory: other clients may not hold
    /// `Fs Fx` (directory-content caching) and are revoked until they ack.
    /// The requester is never restricted: like C++ `Locker::wrlock_force`,
    /// the mutation does not wait for it; the mutation reply itself reports
    /// its reduced cap (`Engine::dentry_dir_cap`), unless it is the
    /// directory's creator (the `filelock` loner,
    /// `SimpleLock::can_wrlock`), who keeps them.
    DirWrlock { requester: ClientId },
    /// The source directory of a rename: `Fs Fx` are recalled from every
    /// holder, the renaming client included. Unlike `DirWrlock`, the reply
    /// traces the destination directory only, so the renaming client's
    /// stale `nfiles`/`nsubdirs` cache can only be refreshed by the
    /// revoke/re-grant cycle (`MClientCaps` carries the fields,
    /// `handle_cap_grant`).
    RenameSrcWrlock { requester: ClientId },
}

/// A hold registered on an inode by a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hold {
    pub id: u64,
    pub kind: HoldKind,
}

impl Hold {
    /// The cap bits client `c` may not hold (pending or issued) under this
    /// hold.
    pub fn excluded_for(&self, c: ClientId) -> u32 {
        match self.kind {
            HoldKind::FileRead => CEPH_CAP_FILE_EXCL | CEPH_CAP_FILE_WR | CEPH_CAP_FILE_BUFFER,
            HoldKind::FileXlock => {
                (0xff << CEPH_CAP_SFILE) & !(CEPH_CAP_FILE_CACHE | CEPH_CAP_FILE_BUFFER)
            }
            HoldKind::AuthRead => CEPH_CAP_AUTH_EXCL,
            HoldKind::AuthXlock => CEPH_CAP_AUTH_SHARED | CEPH_CAP_AUTH_EXCL,
            HoldKind::XattrRead => CEPH_CAP_XATTR_EXCL,
            HoldKind::XattrXlock => CEPH_CAP_XATTR_SHARED | CEPH_CAP_XATTR_EXCL,
            HoldKind::LinkXlock => CEPH_CAP_LINK_SHARED | CEPH_CAP_LINK_EXCL,
            HoldKind::DirWrlock { requester } => {
                if c == requester {
                    0
                } else {
                    CEPH_CAP_FILE_SHARED | CEPH_CAP_FILE_EXCL
                }
            }
            HoldKind::RenameSrcWrlock { requester } => {
                let _ = requester;
                CEPH_CAP_FILE_SHARED | CEPH_CAP_FILE_EXCL
            }
        }
    }
}

/// An inode with its runtime state.
#[derive(Debug, Clone)]
pub struct Inode {
    pub core: InodeCore,
    /// Directory entries, ordered like the C++ dirfrag map.
    pub entries: BTreeMap<DentryKey, Dentry>,
    /// The primary dentry `(parent, name)`; `None` for the root and for
    /// unlinked inodes kept alive by caps (strays).
    pub parent: Option<(Ino, Vec<u8>)>,
    /// Remote (hard-link) dentries pointing at this inode.
    pub remote_links: BTreeSet<(Ino, Vec<u8>)>,
    pub caps: BTreeMap<ClientId, Capability>,
    pub holds: Vec<Hold>,
    /// Number of submitted, uncommitted journal records that changed this
    /// inode (or its entries). Other requests wait until it is zero.
    pub pending: u32,
    pub fcntl_locks: LockTable,
    pub flock_locks: LockTable,
    /// The session that created this inode: its `filelock` loner while no
    /// other client holds caps here (C++ leaves the lock EXCL for the
    /// creator). Runtime state, not journaled; `None` after replay.
    pub creator: Option<ClientId>,
}

impl Inode {
    pub fn new(core: InodeCore) -> Inode {
        Inode {
            core,
            entries: BTreeMap::new(),
            parent: None,
            remote_links: BTreeSet::new(),
            caps: BTreeMap::new(),
            holds: Vec::new(),
            pending: 0,
            fcntl_locks: LockTable::new(),
            flock_locks: LockTable::new(),
            creator: None,
        }
    }

    pub fn ino(&self) -> Ino {
        self.core.ino
    }
    pub fn is_dir(&self) -> bool {
        self.core.is_dir()
    }
    pub fn is_file(&self) -> bool {
        self.core.is_file()
    }

    pub fn lookup(&self, name: &[u8]) -> Option<Dentry> {
        self.entries
            .get(&DentryKey::new(self.core.dir_hash, name))
            .copied()
    }

    pub fn has_file_locks(&self) -> bool {
        !self.fcntl_locks.is_empty() || !self.flock_locks.is_empty()
    }

    /// Union of `issued` over caps of clients other than `c`.
    pub fn issued_by_others(&self, c: ClientId) -> u32 {
        self.caps
            .iter()
            .filter(|(k, _)| **k != c)
            .fold(0, |acc, (_, cap)| acc | cap.issued())
    }

    /// Union of `issued` over every cap.
    pub fn issued_by_all(&self) -> u32 {
        self.caps.values().fold(0, |acc, cap| acc | cap.issued())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_inode_defaults() {
        let c = InodeCore::new(
            0x0100_0000_0000,
            S_IFREG | 0o644,
            1,
            2,
            Timestamp::new(5, 6),
        );
        assert_eq!(c.truncate_seq, 1);
        assert_eq!(c.truncate_size, u64::MAX);
        assert_eq!(c.inline_version, CEPH_INLINE_NONE);
        assert_eq!(c.nlink, 1);
        assert_eq!(c.btime, Timestamp::new(5, 6));
        assert_eq!(c.dir_hash, 0);
        let d = InodeCore::new(2, S_IFDIR | 0o755, 0, 0, Timestamp::ZERO);
        assert_eq!(d.dir_hash, CEPH_STR_HASH_RJENKINS);
    }

    #[test]
    fn hold_exclusions() {
        let h = Hold {
            id: 1,
            kind: HoldKind::FileXlock,
        };
        let ex = h.excluded_for(1);
        assert_eq!(ex & (CEPH_CAP_FILE_CACHE | CEPH_CAP_FILE_BUFFER), 0);
        assert_ne!(ex & CEPH_CAP_FILE_SHARED, 0);
        let d = Hold {
            id: 2,
            kind: HoldKind::DirWrlock { requester: 7 },
        };
        // A client that is not the creator still keeps `Fs Fx` through its
        // own mutation holds (never restricted); the reply trims them.
        assert_eq!(d.excluded_for(7), 0);
        // Any other client loses `Fs Fx`.
        assert_ne!(d.excluded_for(8), 0);
    }
}
