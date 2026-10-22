//! Path resolution (`MDCache::path_traverse`, reduced): symlinks are never
//! followed (clients resolve them), a non-directory component gives ENOTDIR,
//! a missing one ENOENT, an unknown or MDS-private base inode ESTALE.

use super::Engine;
use crate::consts::*;
use crate::errno::Errno;
use crate::journal::Journal;
use crate::types::*;

impl<J: Journal> Engine<J> {
    /// The base inode of a path, or ESTALE.
    pub(crate) fn base(&self, ino: Ino) -> Result<Ino, Errno> {
        if (MDS_INO_MDSDIR_OFFSET..MDS_INO_SYSTEM_BASE).contains(&ino) {
            return Err(Errno::ESTALE);
        }
        if self.inodes.contains_key(&ino) {
            Ok(ino)
        } else {
            Err(Errno::ESTALE)
        }
    }

    /// One step of a walk from directory `cur`.
    fn path_step(&self, cur: Ino, name: &[u8]) -> Result<Ino, Errno> {
        let dir = self.inodes.get(&cur).ok_or(Errno::ESTALE)?;
        if !dir.is_dir() {
            return Err(Errno::ENOTDIR);
        }
        match name {
            b"" | b"." => Ok(cur),
            b".." => Ok(dir.parent.as_ref().map(|(p, _)| *p).unwrap_or(cur)),
            _ => dir.lookup(name).map(|d| d.ino).ok_or(Errno::ENOENT),
        }
    }

    /// Resolve a full path to its target inode.
    pub(crate) fn resolve(&self, path: &FilePath) -> Result<Ino, Errno> {
        let mut cur = self.base(path.ino)?;
        for name in &path.names {
            cur = self.path_step(cur, name)?;
        }
        Ok(cur)
    }

    /// Resolve all but the last component: `(parent dir, last name)`.
    /// A path without components gives EINVAL; a parent that is not a
    /// directory gives ENOTDIR.
    pub(crate) fn resolve_parent(&self, path: &FilePath) -> Result<(Ino, Vec<u8>), Errno> {
        let Some((last, prefix)) = path.names.split_last() else {
            return Err(Errno::EINVAL);
        };
        let mut cur = self.base(path.ino)?;
        for name in prefix {
            cur = self.path_step(cur, name)?;
        }
        let dir = self.inodes.get(&cur).ok_or(Errno::ESTALE)?;
        if !dir.is_dir() {
            return Err(Errno::ENOTDIR);
        }
        Ok((cur, last.clone()))
    }

    /// Is `anc` an ancestor of `ino` (or `ino` itself) through primary links?
    pub(crate) fn is_ancestor_or_self(&self, anc: Ino, mut ino: Ino) -> bool {
        loop {
            if ino == anc {
                return true;
            }
            match self.inodes.get(&ino).and_then(|i| i.parent.as_ref()) {
                Some((p, _)) => ino = *p,
                None => return false,
            }
        }
    }

    /// The nearest snap-realm root at or above `ino` (strays belong to the
    /// root realm).
    pub(crate) fn realm_of(&self, mut ino: Ino) -> Ino {
        loop {
            let Some(i) = self.inodes.get(&ino) else {
                return CEPH_INO_ROOT;
            };
            if i.core.snaprealm.is_some() {
                return ino;
            }
            match &i.parent {
                Some((p, _)) => ino = *p,
                None => return CEPH_INO_ROOT,
            }
        }
    }

    /// The outermost subvolume root at or above `ino`, 0 if none
    /// (`SnapRealm::get_subvolume_ino`).
    pub(crate) fn subvolume_of(&self, mut ino: Ino) -> Ino {
        if let Some(i) = self.inodes.get(&ino)
            && i.core.stable_subvolume_id != 0
        {
            return i.core.stable_subvolume_id;
        }
        let mut found = 0;
        loop {
            let Some(i) = self.inodes.get(&ino) else {
                return found;
            };
            if i.core.snaprealm.as_ref().is_some_and(|r| r.subvolume) {
                found = ino;
            }
            match &i.parent {
                Some((p, _)) => ino = *p,
                None => return found,
            }
        }
    }

    /// The layout new files in `dir` inherit: the nearest layout at or above
    /// it, else the default (`mdr->dir_layout` from `try_rdlock_snap_layout`).
    pub(crate) fn inherited_layout(&self, mut ino: Ino) -> (FileLayout, Ino) {
        loop {
            let Some(i) = self.inodes.get(&ino) else {
                return (self.config.default_layout(), CEPH_INO_ROOT);
            };
            if i.core.layout.is_set() {
                return (i.core.layout.clone(), ino);
            }
            match &i.parent {
                Some((p, _)) => ino = *p,
                None => return (self.config.default_layout(), CEPH_INO_ROOT),
            }
        }
    }

    /// Absolute path of an inode for diagnostics (`/a/b`), `None` for strays.
    pub fn authorization_path(&self, mut ino: Ino) -> Option<Vec<u8>> {
        let initial = self.inodes.get(&ino)?;
        if initial.core.nlink == 0 {
            return initial.core.stray_auth_path.clone();
        }
        let mut parts = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        while ino != CEPH_INO_ROOT {
            if !seen.insert(ino) {
                return None;
            }
            let (parent, name) = self.inodes.get(&ino)?.parent.as_ref()?;
            parts.push(name.as_slice());
            ino = *parent;
        }
        let mut path = Vec::new();
        for part in parts.into_iter().rev() {
            path.push(b'/');
            path.extend_from_slice(part);
        }
        if path.is_empty() {
            path.push(b'/');
        }
        Some(path)
    }

    /// Absolute path for diagnostics. Authorization uses raw bytes instead.
    pub fn path_of(&self, mut ino: Ino) -> Option<String> {
        let mut parts = Vec::new();
        loop {
            if ino == CEPH_INO_ROOT {
                break;
            }
            let (p, name) = self.inodes.get(&ino)?.parent.clone()?;
            parts.push(String::from_utf8_lossy(&name).into_owned());
            ino = p;
        }
        parts.reverse();
        Some(format!("/{}", parts.join("/")))
    }
}
