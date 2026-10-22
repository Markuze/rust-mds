//! Read-side requests: LOOKUP, GETATTR, LOOKUPINO, LOOKUPPARENT, LOOKUPNAME,
//! READDIR and LSSNAP (`Server::handle_client_getattr`, `_lookup_ino`,
//! `handle_client_readdir`).

use super::{Engine, Exec, Pending};
use crate::consts::*;
use crate::effects::*;
use crate::errno::Errno;
use crate::inode::{DentryKey, HoldKind};
use crate::journal::Journal;
use crate::types::*;

/// Rough encoded size of a readdir entry, used against `max_bytes`
/// (the C++ estimates the same way before encoding each entry).
fn entry_estimate(name: &[u8], symlink: usize, xattrs: u64) -> u64 {
    // name (u32 len + bytes), LeaseStat v2 (~22 bytes), InodeStat v9 (~280
    // bytes of fixed fields plus layout, fragtree and optional parts).
    (4 + name.len() as u64) + 22 + 320 + symlink as u64 + xattrs
}

impl<J: Journal> Engine<J> {
    /// Holds a read with `mask` needs: fresh values of field groups that
    /// another client may be changing under an exclusive cap.
    pub(crate) fn read_holds(&self, ino: Ino, client: ClientId, mask: u32) -> Vec<(Ino, HoldKind)> {
        let Some(i) = self.inodes.get(&ino) else {
            return Vec::new();
        };
        let mine = i.caps.get(&client).map(|c| c.issued()).unwrap_or(0);
        let others = i.issued_by_others(client);
        let mut v = Vec::new();
        if mask & CEPH_CAP_AUTH_SHARED != 0
            && mine & CEPH_CAP_AUTH_EXCL == 0
            && others & CEPH_CAP_AUTH_EXCL != 0
        {
            v.push((ino, HoldKind::AuthRead));
        }
        if mask & CEPH_CAP_XATTR_SHARED != 0
            && mine & CEPH_CAP_XATTR_EXCL == 0
            && others & CEPH_CAP_XATTR_EXCL != 0
        {
            v.push((ino, HoldKind::XattrRead));
        }
        if mask & CEPH_CAP_FILE_SHARED != 0
            && mine & CEPH_CAP_FILE_EXCL == 0
            && others & (CEPH_CAP_FILE_EXCL | CEPH_CAP_FILE_WR | CEPH_CAP_FILE_BUFFER) != 0
        {
            v.push((ino, HoldKind::FileRead));
        }
        v
    }

    pub(crate) fn op_lookup(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        mask: u32,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let (dir, name) = match self.resolve_parent(&path) {
            Ok(x) => x,
            Err(e) => return self.reply_error(p, e, out),
        };
        let special = matches!(name.as_slice(), b"." | b"..");
        let target = match name.as_slice() {
            b"." => Some(dir),
            b".." => Some(
                self.inodes[&dir]
                    .parent
                    .as_ref()
                    .map(|(d, _)| *d)
                    .unwrap_or(dir),
            ),
            _ => self.inodes[&dir].lookup(&name).map(|d| d.ino),
        };
        let mut wait = vec![dir];
        wait.extend(target);
        if !self.none_pending(&wait) {
            return Exec::Wait;
        }
        let Some(t) = target else {
            if p.req.want_dentry() {
                let mut s = self.spec(p, Errno::ENOENT.as_neg());
                s.dentry = Some((dir, name));
                return self.reply_now(s, out);
            }
            return self.reply_error(p, Errno::ENOENT, out);
        };
        let holds = self.read_holds(t, p.client, mask);
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        let mut s = self.spec(p, 0);
        if !special {
            s.dentry = Some((dir, name));
        }
        s.target = Some(t);
        s.mask = mask;
        self.reply_now(s, out)
    }

    pub(crate) fn op_getattr(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        mask: u32,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let t = match self.resolve(&path) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        if !self.none_pending(&[t]) {
            return Exec::Wait;
        }
        let holds = self.read_holds(t, p.client, mask);
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        let mut s = self.spec(p, 0);
        s.target = Some(t);
        s.mask = mask;
        self.reply_now(s, out)
    }

    /// LOOKUPINO and LOOKUPHASH: any live inode by number; ESTALE for
    /// unknown, purged and MDS-private numbers.
    pub(crate) fn op_lookupino(
        &mut self,
        p: &mut Pending,
        ino: Ino,
        mask: u32,
        out: &mut Vec<Effect>,
    ) -> Exec {
        if let Err(e) = self.base(ino) {
            return self.reply_error(p, e, out);
        }
        if !self.none_pending(&[ino]) {
            return Exec::Wait;
        }
        let holds = self.read_holds(
            ino,
            p.client,
            mask & (CEPH_CAP_AUTH_SHARED | CEPH_CAP_XATTR_SHARED),
        );
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        let mut s = self.spec(p, 0);
        s.target = Some(ino);
        s.mask = mask;
        self.reply_now(s, out)
    }

    /// LOOKUPPARENT: EINVAL for the root, ESTALE for unlinked inodes.
    pub(crate) fn op_lookupparent(
        &mut self,
        p: &mut Pending,
        ino: Ino,
        out: &mut Vec<Effect>,
    ) -> Exec {
        if let Err(e) = self.base(ino) {
            return self.reply_error(p, e, out);
        }
        if ino == CEPH_INO_ROOT {
            return self.reply_error(p, Errno::EINVAL, out);
        }
        let Some((parent, _)) = self.inodes[&ino].parent.clone() else {
            return self.reply_error(p, Errno::ESTALE, out);
        };
        if !self.none_pending(&[ino, parent]) {
            return Exec::Wait;
        }
        let mut s = self.spec(p, 0);
        s.target = Some(parent);
        self.reply_now(s, out)
    }

    /// LOOKUPNAME: the primary dentry of an inode, with the inode as target
    /// (a LOOKUPNAME trace without target is a kernel BUG_ON).
    pub(crate) fn op_lookupname(
        &mut self,
        p: &mut Pending,
        ino: Ino,
        parent: Option<Ino>,
        out: &mut Vec<Effect>,
    ) -> Exec {
        if let Err(e) = self.base(ino) {
            return self.reply_error(p, e, out);
        }
        let Some((dir, name)) = self.inodes[&ino].parent.clone() else {
            return self.reply_error(p, Errno::ENOENT, out);
        };
        if parent.is_some_and(|pp| pp != dir) {
            return self.reply_error(p, Errno::ENOENT, out);
        }
        if !self.none_pending(&[ino, dir]) {
            return Exec::Wait;
        }
        let mut s = self.spec(p, 0);
        s.dentry = Some((dir, name));
        s.target = Some(ino);
        self.reply_now(s, out)
    }

    /// READDIR: one frag per directory, entries in `(hash24, name)` order,
    /// resuming strictly after `last_name` or at `offset_hash`; at most
    /// `max_entries` entries (0: all) within `max_bytes` (0: 512 KiB +
    /// max_xattr_size). FRAG_END when the end was reached, FRAG_COMPLETE when
    /// the listing also started at the beginning.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op_readdir(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        _frag: u32,
        max_entries: u32,
        max_bytes: u32,
        last_name: Option<Vec<u8>>,
        offset_hash: u32,
        flags: u32,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let dir = match self.resolve(&path) {
            Ok(d) => d,
            Err(e) => return self.reply_error(p, e, out),
        };
        if !self.inodes[&dir].is_dir() {
            return self.reply_error(p, Errno::ENOTDIR, out);
        }
        if !self.none_pending(&[dir]) {
            return Exec::Wait;
        }
        // Listing needs a stable directory: no other client may hold Fx on it.
        let holds = self.read_holds(dir, p.client, CEPH_CAP_FILE_SHARED);
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        let dir_hash = self.inodes[&dir].core.dir_hash;
        let (offset_str, offset_hash) = match &last_name {
            Some(n) if !n.is_empty() => (n.clone(), crate::hash::dentry_hash24(dir_hash, n)),
            _ => (Vec::new(), offset_hash & 0x00ff_ffff),
        };
        let start = offset_hash == 0 && offset_str.is_empty();
        let max = if max_entries == 0 {
            u64::MAX
        } else {
            u64::from(max_entries)
        };
        // Production stepping materializes at most one bounded page. Clients
        // resume after the last returned name when FRAG_END is not set.
        let max = if self.stepping { max.min(64) } else { max };
        let budget = if max_bytes == 0 {
            (512 << 10) + self.config.max_xattr_size
        } else {
            u64::from(max_bytes)
        };
        let dir_realm = self.realm_of(dir);
        let resume = DentryKey {
            hash: offset_hash,
            name: offset_str.clone(),
        };
        let candidates: Vec<(Vec<u8>, Ino)> = self.inodes[&dir]
            .entries
            .range(resume.clone()..)
            .filter(|(k, _)| start || **k > resume)
            // One look-ahead distinguishes a complete page from a partial one.
            .take(max.saturating_add(1).min(budget / 346 + 1) as usize)
            .map(|(k, d)| (k.name.clone(), d.ino))
            .collect();
        let mut entries = Vec::new();
        let mut used: u64 = 64; // DirStat, count and flags
        let mut end = true;
        for (name, ino) in candidates {
            if entries.len() as u64 >= max {
                end = false;
                break;
            }
            let Some(child) = self.inodes.get(&ino) else {
                continue;
            };
            let est = entry_estimate(&name, child.core.symlink.len(), child.core.xattr_bytes());
            if used + est > budget {
                end = false;
                break;
            }
            used += est;
            // Entries that root their own realm get no caps (`dir_realm` check).
            let issue = self.realm_of(ino) == dir_realm;
            let inode = self.inode_stat(ino, p.client, 0, issue);
            entries.push(ReaddirEntry {
                name,
                lease: LeaseStat::default(),
                inode,
            });
        }
        let mut rflags = 0u16;
        if end {
            rflags |= CEPH_READDIR_FRAG_END;
            if start {
                rflags |= CEPH_READDIR_FRAG_COMPLETE;
            }
        }
        if flags & CEPH_READDIR_REPLY_BITFLAGS != 0 {
            rflags |= CEPH_READDIR_HASH_ORDER | CEPH_READDIR_OFFSET_HASH;
        }
        let mut s = self.spec(p, 0);
        s.target = Some(dir);
        s.extra = ReplyExtra::Readdir(ReaddirReply {
            dirfrag: self.dirfrag_info(dir),
            entries,
            flags: rflags,
        });
        self.reply_now(s, out)
    }

    /// LSSNAP: no snapshots in P0; an empty listing that ends the frag.
    pub(crate) fn op_lssnap(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let dir = match self.resolve(&path) {
            Ok(d) => d,
            Err(e) => return self.reply_error(p, e, out),
        };
        if !self.inodes[&dir].is_dir() {
            return self.reply_error(p, Errno::ENOTDIR, out);
        }
        let mut s = self.spec(p, 0);
        s.target = Some(dir);
        s.extra = ReplyExtra::Readdir(ReaddirReply {
            dirfrag: self.dirfrag_info(dir),
            entries: Vec::new(),
            flags: CEPH_READDIR_FRAG_END | CEPH_READDIR_FRAG_COMPLETE,
        });
        self.reply_now(s, out)
    }
}
