//! Namespace mutations: CREATE, MKNOD, MKDIR, SYMLINK, LINK, UNLINK, RMDIR,
//! RENAME (`Server::handle_client_openc`, `_mknod`, `_mkdir`, `_symlink`,
//! `_link`, `_unlink`, `_rename`), with the C++ check order and effects:
//!
//! * new inodes: owner from the request, group and `S_ISGID` from a setgid
//!   parent, `atime = mtime = ctime = btime = stamp`, `change_attr = 0`,
//!   `truncate_seq = 1`, `truncate_size = -1`, initial xattrs stored as sent;
//! * the parent: `nfiles`/`nsubdirs` +/- 1, `change_attr += 1`, `mtime = ctime
//!   = stamp` only when the stamp is newer (P-touch);
//! * the target of link/unlink/rename: `ctime = stamp`, `change_attr += 1`,
//!   `nlink` +/- 1 (T-ctime);
//! * recursive stats are kept exact: every change is added to all ancestors
//!   through primary links (images of the ancestors go into the record).

use super::{Deferred, Engine, Exec, Pending, Txn};
use crate::caps::{caps_for_mode, flags_to_mode};
use crate::consts::*;
use crate::effects::*;
use crate::errno::Errno;
use crate::inode::{Dentry, DentryKey, HoldKind, Inode, InodeCore};
use crate::journal::{DentryLink, DentryUpdate, Journal, RecordKind};
use crate::request::LayoutOverride;
use crate::types::*;

impl<J: Journal> Engine<J> {
    // ------------------------------------------------------------------
    // Stat helpers

    /// Add `delta` (times `sign` for counters; `rctime` as a maximum) to the
    /// recursive stats of `dir` and all its ancestors.
    pub(crate) fn propagate_rstat(&mut self, txn: &mut Txn, dir: Ino, delta: &Rstat, sign: i64) {
        let mut cur = Some(dir);
        while let Some(ino) = cur {
            let Some(i) = self.inodes.get_mut(&ino) else {
                break;
            };
            i.core.rstat.add(delta, sign);
            cur = i.parent.as_ref().map(|(p, _)| *p);
            self.image(txn, ino);
        }
    }

    /// Raise `rctime` of `ino` and its ancestors to at least `t`.
    pub(crate) fn bump_rctime(&mut self, txn: &mut Txn, ino: Ino, t: Timestamp) {
        let mut cur = Some(ino);
        while let Some(c) = cur {
            let Some(i) = self.inodes.get_mut(&c) else {
                break;
            };
            if i.core.rstat.rctime >= t {
                break;
            }
            i.core.rstat.rctime = t;
            cur = i.parent.as_ref().map(|(p, _)| *p);
            self.image(txn, c);
        }
    }

    /// Re-derive the recursive stats of a non-directory from its size and
    /// times and propagate the change.
    pub(crate) fn update_file_rstat(&mut self, txn: &mut Txn, ino: Ino) {
        let Some(i) = self.inodes.get_mut(&ino) else {
            return;
        };
        if i.is_dir() {
            let t = i.core.ctime.max(i.core.mtime);
            self.bump_rctime(txn, ino, t);
            return;
        }
        let old = i.core.rstat;
        let mut new = old;
        new.rbytes = if i.core.is_symlink() {
            i.core.symlink.len() as i64
        } else {
            i.core.size as i64
        };
        new.rfiles = 1;
        new.rctime = old.rctime.max(i.core.ctime).max(i.core.mtime);
        if new == old {
            return;
        }
        i.core.rstat = new;
        let parent = i.parent.as_ref().map(|(p, _)| *p);
        self.image(txn, ino);
        if let Some(p) = parent {
            let delta = Rstat {
                rbytes: new.rbytes - old.rbytes,
                rctime: new.rctime,
                ..Default::default()
            };
            self.propagate_rstat(txn, p, &delta, 1);
        }
    }

    /// P-touch: a name was added to or removed from `dir`.
    pub(crate) fn touch_parent(
        &mut self,
        txn: &mut Txn,
        dir: Ino,
        stamp: Timestamp,
        dfiles: i64,
        dsubdirs: i64,
    ) {
        self.touch(txn, dir);
        let i = self.inodes.get_mut(&dir).unwrap();
        let c = &mut i.core;
        c.dirstat.nfiles += dfiles;
        c.dirstat.nsubdirs += dsubdirs;
        c.dirstat.change_attr += 1;
        if stamp > c.dirstat.mtime {
            c.dirstat.mtime = stamp;
            if stamp > c.mtime {
                c.mtime = stamp;
                c.ctime = stamp;
            }
        }
        c.change_attr += 1;
        self.bump_rctime(txn, dir, stamp);
    }

    /// T-ctime: the inode itself changed.
    pub(crate) fn touch_ctime(&mut self, txn: &mut Txn, ino: Ino, stamp: Timestamp) {
        self.touch(txn, ino);
        let c = &mut self.inodes.get_mut(&ino).unwrap().core;
        c.ctime = stamp;
        c.change_attr += 1;
        self.bump_rctime(txn, ino, stamp);
    }

    // ------------------------------------------------------------------
    // Dentry helpers

    fn link_dentry(&mut self, txn: &mut Txn, dir: Ino, name: &[u8], ino: Ino, primary: bool) {
        let d = self.inodes.get_mut(&dir).unwrap();
        let key = DentryKey::new(d.core.dir_hash, name);
        d.entries.insert(key, Dentry { ino, primary });
        let t = self.inodes.get_mut(&ino).unwrap();
        if primary {
            t.parent = Some((dir, name.to_vec()));
        } else {
            t.remote_links.insert((dir, name.to_vec()));
        }
        txn.rec.dentries.push(DentryUpdate {
            dir,
            name: name.to_vec(),
            link: Some(DentryLink { ino, primary }),
        });
    }

    fn unlink_dentry(&mut self, txn: &mut Txn, dir: Ino, name: &[u8]) -> Option<Dentry> {
        let d = self.inodes.get_mut(&dir)?;
        let key = DentryKey::new(d.core.dir_hash, name);
        let dn = d.entries.remove(&key)?;
        if let Some(t) = self.inodes.get_mut(&dn.ino) {
            if dn.primary {
                if t.parent
                    .as_ref()
                    .is_some_and(|(p, n)| *p == dir && n == name)
                {
                    t.parent = None;
                }
            } else {
                t.remote_links.remove(&(dir, name.to_vec()));
            }
        }
        txn.rec.dentries.push(DentryUpdate {
            dir,
            name: name.to_vec(),
            link: None,
        });
        Some(dn)
    }

    /// Remove the name `dir/name` and drop one link of its inode: the
    /// inode's primary link moves to a remaining hard link, or the inode
    /// becomes a stray (kept while caps are held) or is purged.
    fn remove_link(&mut self, txn: &mut Txn, dir: Ino, name: &[u8], stamp: Timestamp) {
        let Some(dn) = self.unlink_dentry(txn, dir, name) else {
            return;
        };
        let t = dn.ino;
        let is_dir = self.inodes[&t].is_dir();
        self.touch_parent(
            txn,
            dir,
            stamp,
            if is_dir { 0 } else { -1 },
            if is_dir { -1 } else { 0 },
        );
        self.touch_ctime(txn, t, stamp);
        self.inodes.get_mut(&t).unwrap().core.nlink -= 1;
        if dn.primary {
            let rstat = self.inodes[&t].core.rstat;
            self.propagate_rstat(txn, dir, &rstat, -1);
            let promote = self.inodes[&t].remote_links.iter().next().cloned();
            if let Some((rd, rn)) = promote
                && self.inodes[&t].core.nlink > 0
            {
                // A remaining hard link becomes the primary one (the C++
                // reintegrates the stray onto a remote dentry).
                let rdi = self.inodes.get_mut(&rd).unwrap();
                let key = DentryKey::new(rdi.core.dir_hash, &rn);
                rdi.entries.insert(
                    key,
                    Dentry {
                        ino: t,
                        primary: true,
                    },
                );
                let ti = self.inodes.get_mut(&t).unwrap();
                ti.remote_links.remove(&(rd, rn.clone()));
                ti.parent = Some((rd, rn.clone()));
                txn.rec.dentries.push(DentryUpdate {
                    dir: rd,
                    name: rn,
                    link: Some(DentryLink {
                        ino: t,
                        primary: true,
                    }),
                });
                self.propagate_rstat(txn, rd, &rstat, 1);
            }
        }
        let ti = self.inodes.get_mut(&t).unwrap();
        if ti.core.nlink <= 0 {
            ti.core.nlink = 0;
            if ti.caps.is_empty() {
                self.purge_inode(txn, t);
            }
        }
    }

    // ------------------------------------------------------------------
    // Gates

    /// Create gates (`rdlock_path_xlock_dentry` with `create`): the parent
    /// directory, the name and an existing dentry.
    fn create_gates(&self, path: &FilePath) -> Result<(Ino, Vec<u8>, Option<Dentry>), Errno> {
        let (dir, name) = self.resolve_parent(path)?;
        if name.is_empty() {
            return Err(Errno::EINVAL);
        }
        if name == b"." || name == b".." {
            return Err(Errno::EEXIST);
        }
        let d = &self.inodes[&dir];
        if d.core.nlink == 0 && dir != CEPH_INO_ROOT {
            return Err(Errno::ENOENT);
        }
        Ok((dir, name.clone(), d.lookup(&name)))
    }

    /// A new inode under `dir` (`Server::prepare_new_inode`).
    fn new_inode(&self, ino: Ino, dir: Ino, mode: u32, p: &Pending) -> InodeCore {
        let parent = &self.inodes[&dir].core;
        let mut mode = mode;
        let gid = if parent.mode & S_ISGID != 0 {
            if is_dir(mode) {
                mode |= S_ISGID;
            }
            parent.gid
        } else {
            p.req.owner_gid
        };
        let mut c = InodeCore::new(ino, mode, p.req.owner_uid, gid, p.req.stamp);
        c.rstat.rctime = p.req.stamp;
        if is_dir(mode) {
            c.rstat.rsubdirs = 1;
        } else {
            c.rstat.rfiles = 1;
            c.layout = self.config.default_layout();
        }
        c
    }

    /// Insert a new inode, link it into `dir` and account for it.
    fn install_new(
        &mut self,
        txn: &mut Txn,
        dir: Ino,
        name: &[u8],
        core: InodeCore,
        stamp: Timestamp,
    ) {
        let ino = core.ino;
        let is_dir = core.is_dir();
        let rstat = core.rstat;
        self.inodes.insert(ino, Inode::new(core));
        self.link_dentry(txn, dir, name, ino, true);
        self.touch(txn, ino);
        self.touch_parent(
            txn,
            dir,
            stamp,
            if is_dir { 0 } else { 1 },
            if is_dir { 1 } else { 0 },
        );
        self.propagate_rstat(txn, dir, &rstat, 1);
    }

    fn created_reply(
        &self,
        p: &Pending,
        dir: Ino,
        name: Vec<u8>,
        ino: Ino,
        create_extra: bool,
    ) -> Deferred {
        let mut s = self.spec(p, 0);
        s.dentry = Some((dir, name));
        s.target = Some(ino);
        s.created_ino = Some(ino);
        if create_extra {
            s.extra = ReplyExtra::Create {
                ino,
                delegated: Vec::new(),
            };
        }
        Deferred::Reply(s)
    }

    // ------------------------------------------------------------------
    // CREATE

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op_create(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        flags: u32,
        mode: u32,
        lo: LayoutOverride,
        xattrs: XattrMap,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let Some(cmode) = flags_to_mode(flags) else {
            return self.reply_error(p, Errno::EINVAL, out);
        };
        let (dir, name, existing) = match self.create_gates(&path) {
            Ok(x) => x,
            Err(e) => return self.reply_error(p, e, out),
        };
        if p.has_completed {
            // A retried CREATE that already committed: open what exists.
            return match existing {
                Some(_) => {
                    self.op_open(p, path, flags & !CEPH_O_TRUNC, 0, 0, Some((dir, name)), out)
                }
                None => self.reply_error(p, Errno::ENOENT, out),
            };
        }
        if existing.is_some() {
            if flags & CEPH_O_EXCL != 0 {
                return self.reply_error(p, Errno::EEXIST, out);
            }
            return self.op_open(p, path, flags, 0, 0, Some((dir, name)), out);
        }
        if !self.none_pending(&[dir]) {
            return Exec::Wait;
        }
        if !self.acquire_holds(
            p,
            &[(
                dir,
                HoldKind::DirWrlock {
                    requester: p.client,
                },
            )],
            out,
        ) {
            return Exec::Wait;
        }
        // Layout: inherited, with the request's overrides.
        let (mut layout, _) = self.inherited_layout(dir);
        if lo.stripe_unit != 0 {
            layout.stripe_unit = lo.stripe_unit;
        }
        if lo.stripe_count != 0 {
            layout.stripe_count = lo.stripe_count;
        }
        if lo.object_size != 0 {
            layout.object_size = lo.object_size;
        }
        if lo.pool >= 0 {
            layout.pool_id = lo.pool;
        }
        if !layout.is_valid_geometry() || !self.config.is_data_pool(layout.pool_id) {
            return self.reply_error(p, Errno::EINVAL, out);
        }
        let mut txn = Txn::new(RecordKind::Create);
        let ino = match self.alloc_ino(&mut txn) {
            Ok(i) => i,
            Err(e) => return self.reply_error(p, e, out),
        };
        let mut core = self.new_inode(ino, dir, (mode & !S_IFMT) | S_IFREG, p);
        core.layout = layout.clone();
        if layout.pool_id != self.config.default_pool() {
            core.old_pools.insert(self.config.default_pool());
        }
        core.xattrs = xattrs;
        if cmode & CEPH_FILE_MODE_WR != 0 {
            core.client_ranges
                .insert(p.client, u64::from(layout.stripe_unit));
        }
        let stamp = p.req.stamp;
        self.install_new(&mut txn, dir, &name, core, stamp);
        self.ensure_cap(ino, p.client, true);
        let cap = self
            .inodes
            .get_mut(&ino)
            .unwrap()
            .caps
            .get_mut(&p.client)
            .unwrap();
        cap.set_wanted(caps_for_mode(cmode));
        self.complete_request(p, &mut txn, Some(ino));
        let r = self.created_reply(p, dir, name, ino, true);
        txn.deferred.push(r);
        self.submit(txn, Some(p), out);
        Exec::Done
    }

    // ------------------------------------------------------------------
    // MKNOD, MKDIR, SYMLINK

    pub(crate) fn op_mknod(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        mode: u32,
        rdev: u32,
        xattrs: XattrMap,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let mut mode = mode;
        if mode & S_IFMT == 0 {
            mode |= S_IFREG;
        }
        if is_dir(mode) {
            // The VFS never sends this; libcephfs may. Create a directory
            // properly rather than a half-made one.
            return self.op_mkdir(p, path, mode, xattrs, out);
        }
        self.op_create_node(p, path, mode, rdev, None, xattrs, RecordKind::Mknod, out)
    }

    pub(crate) fn op_mkdir(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        mode: u32,
        xattrs: XattrMap,
        out: &mut Vec<Effect>,
    ) -> Exec {
        self.op_create_node(
            p,
            path,
            (mode & !S_IFMT) | S_IFDIR,
            0,
            None,
            xattrs,
            RecordKind::Mkdir,
            out,
        )
    }

    pub(crate) fn op_symlink(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        target: Vec<u8>,
        xattrs: XattrMap,
        out: &mut Vec<Effect>,
    ) -> Exec {
        self.op_create_node(
            p,
            path,
            S_IFLNK | 0o777,
            0,
            Some(target),
            xattrs,
            RecordKind::Symlink,
            out,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn op_create_node(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        mode: u32,
        rdev: u32,
        symlink: Option<Vec<u8>>,
        xattrs: XattrMap,
        kind: RecordKind,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let (dir, name, existing) = match self.create_gates(&path) {
            Ok(x) => x,
            Err(e) => return self.reply_error(p, e, out),
        };
        if existing.is_some() {
            return self.reply_error(p, Errno::EEXIST, out);
        }
        if !self.none_pending(&[dir]) {
            return Exec::Wait;
        }
        if !self.acquire_holds(
            p,
            &[(
                dir,
                HoldKind::DirWrlock {
                    requester: p.client,
                },
            )],
            out,
        ) {
            return Exec::Wait;
        }
        let mut txn = Txn::new(kind);
        let ino = match self.alloc_ino(&mut txn) {
            Ok(i) => i,
            Err(e) => return self.reply_error(p, e, out),
        };
        let mut core = self.new_inode(ino, dir, mode, p);
        core.rdev = rdev;
        core.xattrs = xattrs;
        if is_reg(mode) {
            core.layout = self.inherited_layout(dir).0;
        }
        if let Some(t) = symlink {
            core.size = t.len() as u64;
            core.rstat.rbytes = t.len() as i64;
            core.symlink = t;
        }
        let stamp = p.req.stamp;
        self.install_new(&mut txn, dir, &name, core, stamp);
        // The creator's cap (cap id 1, `CInode::add_client_cap(new_inode)`).
        self.ensure_cap(ino, p.client, true);
        self.complete_request(p, &mut txn, Some(ino));
        let r = self.created_reply(p, dir, name, ino, false);
        txn.deferred.push(r);
        self.submit(txn, Some(p), out);
        Exec::Done
    }

    // ------------------------------------------------------------------
    // LINK

    pub(crate) fn op_link(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        target: FilePath,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let (dir, name, existing) = match self.create_gates(&path) {
            Ok(x) => x,
            Err(e) => return self.reply_error(p, e, out),
        };
        let t = match self.resolve(&target) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        if existing.is_some() {
            return self.reply_error(p, Errno::EEXIST, out);
        }
        let ti = &self.inodes[&t];
        if ti.is_dir() {
            return self.reply_error(p, Errno::EINVAL, out);
        }
        if ti.core.nlink == 0 {
            return self.reply_error(p, Errno::ENOENT, out);
        }
        if let Some((tp, _)) = &ti.parent
            && *tp != dir
            && self.subvolume_of(*tp) != self.subvolume_of(dir)
        {
            return self.reply_error(p, Errno::EXDEV, out);
        }
        if !self.none_pending(&[dir, t]) {
            return Exec::Wait;
        }
        let holds = [
            (
                dir,
                HoldKind::DirWrlock {
                    requester: p.client,
                },
            ),
            (t, HoldKind::LinkXlock),
        ];
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        let stamp = p.req.stamp;
        let mut txn = Txn::new(RecordKind::Link);
        self.link_dentry(&mut txn, dir, &name, t, false);
        self.touch_parent(&mut txn, dir, stamp, 1, 0);
        self.touch_ctime(&mut txn, t, stamp);
        self.inodes.get_mut(&t).unwrap().core.nlink += 1;
        self.complete_request(p, &mut txn, None);
        let mut s = self.spec(p, 0);
        s.dentry = Some((dir, name));
        s.target = Some(t);
        txn.deferred.push(Deferred::Reply(s));
        self.submit(txn, Some(p), out);
        Exec::Done
    }

    // ------------------------------------------------------------------
    // UNLINK and RMDIR

    pub(crate) fn op_unlink(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        rmdir: bool,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let (dir, name) = match self.resolve_parent(&path) {
            Ok(x) => x,
            Err(e) => return self.reply_error(p, e, out),
        };
        if name == b"." || name == b".." {
            return self.reply_error(p, Errno::ENOTEMPTY, out);
        }
        let Some(dn) = self.inodes[&dir].lookup(&name) else {
            return self.reply_error(p, Errno::ENOENT, out);
        };
        let t = dn.ino;
        let ti = &self.inodes[&t];
        if !rmdir && ti.is_dir() {
            return self.reply_error(p, Errno::EISDIR, out);
        }
        if rmdir && !ti.is_dir() {
            return self.reply_error(p, Errno::ENOTDIR, out);
        }
        if rmdir && !ti.entries.is_empty() {
            return self.reply_error(p, Errno::ENOTEMPTY, out);
        }
        if !self.none_pending(&[dir, t]) {
            return Exec::Wait;
        }
        let holds = [
            (
                dir,
                HoldKind::DirWrlock {
                    requester: p.client,
                },
            ),
            (t, HoldKind::LinkXlock),
        ];
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        let mut txn = Txn::new(if rmdir {
            RecordKind::Rmdir
        } else {
            RecordKind::Unlink
        });
        self.remove_link(&mut txn, dir, &name, p.req.stamp);
        self.complete_request(p, &mut txn, None);
        let mut s = self.spec(p, 0);
        s.dentry = Some((dir, name));
        txn.deferred.push(Deferred::Reply(s));
        self.submit(txn, Some(p), out);
        Exec::Done
    }

    // ------------------------------------------------------------------
    // RENAME

    pub(crate) fn op_rename(
        &mut self,
        p: &mut Pending,
        dst: FilePath,
        src: FilePath,
        out: &mut Vec<Effect>,
    ) -> Exec {
        for path in [&dst, &src] {
            if matches!(path.last(), Some(b".") | Some(b"..")) {
                return self.reply_error(p, Errno::EBUSY, out);
            }
        }
        let (ddir, dname) = match self.resolve_parent(&dst) {
            Ok(x) => x,
            Err(e) => return self.reply_error(p, e, out),
        };
        let (sdir, sname) = match self.resolve_parent(&src) {
            Ok(x) => x,
            Err(e) => return self.reply_error(p, e, out),
        };
        if dname.is_empty() || sname.is_empty() {
            return self.reply_error(p, Errno::EINVAL, out);
        }
        if self.inodes[&ddir].core.nlink == 0 && ddir != CEPH_INO_ROOT {
            return self.reply_error(p, Errno::ENOENT, out);
        }
        let Some(sdn) = self.inodes[&sdir].lookup(&sname) else {
            return self.reply_error(p, Errno::ENOENT, out);
        };
        if sdir == ddir && sname == dname {
            return self.reply_error_ok(p, out);
        }
        let srci = sdn.ino;
        let src_is_dir = self.inodes[&srci].is_dir();
        if src_is_dir && self.is_ancestor_or_self(srci, ddir) {
            return self.reply_error(p, Errno::EINVAL, out);
        }
        let oldin = self.inodes[&ddir].lookup(&dname).map(|d| d.ino);
        if let Some(o) = oldin {
            let oi = &self.inodes[&o];
            if oi.is_dir() && !oi.entries.is_empty() {
                return self.reply_error(p, Errno::ENOTEMPTY, out);
            }
            if oi.is_dir() && !src_is_dir {
                return self.reply_error(p, Errno::EISDIR, out);
            }
            if !oi.is_dir() && src_is_dir {
                return self.reply_error(p, Errno::ENOTDIR, out);
            }
            if o == srci {
                // Two names of one inode: a successful no-op.
                return self.reply_error_ok(p, out);
            }
        }
        if self.subvolume_of(sdir) != self.subvolume_of(ddir) {
            return self.reply_error(p, Errno::EXDEV, out);
        }
        let mut wait = vec![sdir, ddir, srci];
        wait.extend(oldin);
        if !self.none_pending(&wait) {
            return Exec::Wait;
        }
        let mut holds = vec![
            (
                ddir,
                HoldKind::DirWrlock {
                    requester: p.client,
                },
            ),
            (srci, HoldKind::LinkXlock),
        ];
        if sdir != ddir {
            holds.push((
                sdir,
                HoldKind::DirWrlock {
                    requester: p.client,
                },
            ));
        }
        if let Some(o) = oldin {
            holds.push((o, HoldKind::LinkXlock));
        }
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        let stamp = p.req.stamp;
        let mut txn = Txn::new(RecordKind::Rename);
        if oldin.is_some() {
            self.remove_link(&mut txn, ddir, &dname, stamp);
        }
        let moved = self
            .unlink_dentry(&mut txn, sdir, &sname)
            .expect("source dentry");
        let (df, dd) = if src_is_dir { (0, 1) } else { (1, 0) };
        self.touch_parent(&mut txn, sdir, stamp, -df, -dd);
        let rstat = self.inodes[&srci].core.rstat;
        if moved.primary {
            self.propagate_rstat(&mut txn, sdir, &rstat, -1);
        }
        self.link_dentry(&mut txn, ddir, &dname, srci, moved.primary);
        self.touch_parent(&mut txn, ddir, stamp, df, dd);
        if moved.primary {
            self.propagate_rstat(&mut txn, ddir, &rstat, 1);
        }
        self.touch_ctime(&mut txn, srci, stamp);
        self.complete_request(p, &mut txn, None);
        let mut s = self.spec(p, 0);
        s.dentry = Some((ddir, dname));
        s.target = Some(srci);
        txn.deferred.push(Deferred::Reply(s));
        self.submit(txn, Some(p), out);
        Exec::Done
    }

    /// A successful no-op answered without a trace (the C++
    /// `respond_to_request(mdr, 0)` before any tracei/tracedn is set).
    pub(crate) fn reply_error_ok(&mut self, p: &mut Pending, out: &mut Vec<Effect>) -> Exec {
        let s = self.spec(p, 0);
        self.reply_now(s, out)
    }
}
