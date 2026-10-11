//! SETATTR and OPEN (`Server::handle_client_setattr`, `handle_client_open`,
//! `do_open_truncate`) and client writeable ranges (`max_size`).

use super::{Deferred, Engine, Exec, Pending, Txn, calc_new_max_size};
use crate::caps::{caps_for_mode, flags_to_mode};
use crate::consts::*;
use crate::effects::*;
use crate::errno::Errno;
use crate::inode::HoldKind;
use crate::journal::{Journal, RecordKind};
use crate::request::SetattrArgs;
use crate::types::*;

impl<J: Journal> Engine<J> {
    /// `Locker::calc_new_client_ranges`: every client with write caps issued
    /// or wanted gets a range of at least `calc_new_max_size(size)` (ranges
    /// only grow); ranges of clients without write caps are dropped.
    /// Returns (changed, clients whose range grew).
    pub(crate) fn recompute_client_ranges(&mut self, ino: Ino, size: u64) -> (bool, Vec<ClientId>) {
        let inc = self.config.writeable_range_max_inc_objs;
        let i = self.inodes.get_mut(&ino).unwrap();
        if !i.is_file() {
            return (false, Vec::new());
        }
        let ms = if i.core.layout.is_set() {
            calc_new_max_size(&i.core.layout, size, inc)
        } else {
            0
        };
        let writers: Vec<ClientId> = i
            .caps
            .iter()
            .filter(|(_, c)| (c.issued() | c.wanted()) & CEPH_CAP_ANY_FILE_WR != 0)
            .map(|(k, _)| *k)
            .collect();
        let mut changed = false;
        let mut grew = Vec::new();
        let before: Vec<ClientId> = i.core.client_ranges.keys().copied().collect();
        for c in before {
            if !writers.contains(&c) {
                i.core.client_ranges.remove(&c);
                changed = true;
            }
        }
        for c in writers {
            let cur = i.core.client_ranges.get(&c).copied();
            if cur.is_none_or(|r| ms > r) {
                i.core.client_ranges.insert(c, ms.max(cur.unwrap_or(0)));
                changed = true;
                grew.push(c);
            }
        }
        (changed, grew)
    }

    pub(crate) fn op_setattr(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        a: SetattrArgs,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let t = match self.resolve(&path) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        if t < MDS_INO_SYSTEM_BASE && t != CEPH_INO_ROOT {
            return self.reply_error(p, Errno::EPERM, out);
        }
        if !self.none_pending(&[t]) {
            return Exec::Wait;
        }
        let mask = a.mask;
        let mut holds = Vec::new();
        let auth_bits = CEPH_SETATTR_MODE
            | CEPH_SETATTR_UID
            | CEPH_SETATTR_GID
            | CEPH_SETATTR_BTIME
            | CEPH_SETATTR_KILL_SGUID
            | CEPH_SETATTR_KILL_SUID
            | CEPH_SETATTR_KILL_SGID;
        if mask & auth_bits != 0 {
            holds.push((t, HoldKind::AuthXlock));
        }
        if mask & (CEPH_SETATTR_MTIME | CEPH_SETATTR_ATIME | CEPH_SETATTR_SIZE) != 0 {
            holds.push((t, HoldKind::FileXlock));
        }
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        let stamp = p.req.stamp;
        let mut txn = Txn::new(RecordKind::Setattr);
        self.touch(&mut txn, t);
        let mut truncated = false;
        let mut share: Vec<ClientId> = Vec::new();
        {
            let c = &mut self.inodes.get_mut(&t).unwrap().core;
            if mask & CEPH_SETATTR_UID != 0 {
                c.uid = a.uid;
            }
            if mask & CEPH_SETATTR_GID != 0 {
                c.gid = a.gid;
            }
            if mask & CEPH_SETATTR_MODE != 0 {
                c.mode = (c.mode & !0o7777) | (a.mode & 0o7777);
            } else if mask
                & (CEPH_SETATTR_UID
                    | CEPH_SETATTR_GID
                    | CEPH_SETATTR_KILL_SGUID
                    | CEPH_SETATTR_KILL_SUID
                    | CEPH_SETATTR_KILL_SGID)
                != 0
                && is_reg(c.mode)
            {
                if mask & (CEPH_SETATTR_UID | CEPH_SETATTR_GID | CEPH_SETATTR_KILL_SGUID) != 0
                    && c.mode & (S_IXUSR | S_IXGRP | S_IXOTH) != 0
                {
                    c.mode &= !(S_ISUID | S_ISGID);
                } else {
                    if mask & CEPH_SETATTR_KILL_SUID != 0 {
                        c.mode &= !S_ISUID;
                    }
                    if mask & CEPH_SETATTR_KILL_SGID != 0 {
                        c.mode &= !S_ISGID;
                    }
                }
            }
            if mask & CEPH_SETATTR_MTIME != 0 {
                c.mtime = a.mtime;
            }
            if mask & CEPH_SETATTR_ATIME != 0 {
                c.atime = a.atime;
            }
            if mask & CEPH_SETATTR_BTIME != 0 {
                c.btime = a.btime;
            }
            if mask & (CEPH_SETATTR_ATIME | CEPH_SETATTR_MTIME | CEPH_SETATTR_BTIME) != 0 {
                c.time_warp_seq = c.time_warp_seq.wrapping_add(1);
            }
            if mask & CEPH_SETATTR_SIZE != 0 {
                let old_size = c.size.max(a.old_size);
                if a.size < old_size {
                    // `inode_t::truncate`
                    c.max_size_ever = c.max_size_ever.max(old_size);
                    c.truncate_from = old_size;
                    c.size = a.size;
                    c.truncate_size = a.size;
                    c.truncate_seq = c.truncate_seq.wrapping_add(1);
                    truncated = true;
                } else {
                    c.size = a.size;
                }
                c.mtime = stamp;
            }
            c.ctime = stamp;
            c.change_attr += 1;
        }
        if mask & CEPH_SETATTR_SIZE != 0 {
            let size = self.inodes[&t].core.size;
            let (_, grew) = self.recompute_client_ranges(t, size);
            share = grew;
        }
        self.update_file_rstat(&mut txn, t);
        self.bump_rctime(&mut txn, t, stamp);
        self.complete_request(p, &mut txn, None);
        // C_MDS_inode_update_finish order: TRUNC, reply, max_size grants.
        if truncated {
            txn.deferred.push(Deferred::Truncate { ino: t });
        }
        let mut s = self.spec(p, 0);
        s.target = Some(t);
        txn.deferred.push(Deferred::Reply(s));
        if !share.is_empty() {
            txn.deferred.push(Deferred::ShareMaxSize {
                ino: t,
                clients: share,
            });
        }
        self.submit(txn, Some(p), out);
        Exec::Done
    }

    /// OPEN of an existing inode (also CREATE of an existing name without
    /// `O_EXCL`, which passes the dentry for the trace).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn op_open(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        flags: u32,
        mask: u32,
        old_size: u64,
        dentry: Option<(Ino, Vec<u8>)>,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let Some(mut cmode) = flags_to_mode(flags) else {
            return self.reply_error(p, Errno::EINVAL, out);
        };
        let t = match self.resolve(&path) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        let mut flags = flags;
        let (is_file, is_dir, is_symlink) = {
            let c = &self.inodes[&t].core;
            (c.is_file(), c.is_dir(), c.is_symlink())
        };
        if !is_file {
            cmode = CEPH_FILE_MODE_PIN;
        }
        if is_symlink && flags & CEPH_O_NOFOLLOW == 0 {
            flags &= !CEPH_O_TRUNC;
        }
        if flags & CEPH_O_DIRECTORY != 0 && !is_dir && !is_symlink {
            return self.reply_error(p, Errno::EINVAL, out);
        }
        if flags & CEPH_O_TRUNC != 0 && !is_file {
            return self.reply_error(p, if is_dir { Errno::EISDIR } else { Errno::EINVAL }, out);
        }
        let mut wait = vec![t];
        if let Some((d, _)) = &dentry {
            wait.push(*d);
        }
        if !self.none_pending(&wait) {
            return Exec::Wait;
        }
        let truncate = flags & CEPH_O_TRUNC != 0 && !p.has_completed;
        let mut holds = self.read_holds(
            t,
            p.client,
            mask & (CEPH_CAP_AUTH_SHARED | CEPH_CAP_XATTR_SHARED),
        );
        if truncate {
            holds.push((t, HoldKind::FileXlock));
        }
        if !self.acquire_holds(p, &holds, out) {
            return Exec::Wait;
        }
        if is_file || is_dir {
            self.ensure_cap(t, p.client, false);
            let cap = self
                .inodes
                .get_mut(&t)
                .unwrap()
                .caps
                .get_mut(&p.client)
                .unwrap();
            let w = cap.wanted() | caps_for_mode(cmode);
            cap.set_wanted(w);
        }
        let stamp = p.req.stamp;
        let mut txn: Option<Txn> = None;
        if truncate {
            let mut tx = Txn::new(RecordKind::Open);
            self.touch(&mut tx, t);
            let c = &mut self.inodes.get_mut(&t).unwrap().core;
            c.mtime = stamp;
            c.ctime = stamp;
            c.change_attr += 1;
            let old = c.size.max(old_size);
            if old > 0 {
                c.max_size_ever = c.max_size_ever.max(old);
                c.truncate_from = old;
                c.size = 0;
                c.truncate_size = 0;
                c.truncate_seq = c.truncate_seq.wrapping_add(1);
                tx.deferred.push(Deferred::Truncate { ino: t });
            }
            self.update_file_rstat(&mut tx, t);
            self.bump_rctime(&mut tx, t, stamp);
            self.complete_request(p, &mut tx, None);
            txn = Some(tx);
        }
        if is_file && cmode & CEPH_FILE_MODE_WR != 0 {
            let size = self.inodes[&t].core.size;
            let (changed, _) = self.recompute_client_ranges(t, size);
            if changed {
                let tx = txn.get_or_insert_with(|| Txn::new(RecordKind::ClientRanges));
                self.touch(tx, t);
            }
        }
        // Other clients may have to give up caps for the new opener.
        self.eval_caps(t, out);
        let mut s = self.spec(p, 0);
        s.target = Some(t);
        s.mask = mask;
        if let Some(d) = dentry {
            s.dentry = Some(d);
        }
        if p.has_completed
            && let Some(ino) = p.completed_ino
        {
            s.extra = ReplyExtra::Create {
                ino,
                delegated: Vec::new(),
            };
            s.created_ino = Some(ino);
        }
        match txn {
            Some(mut tx) => {
                tx.deferred.push(Deferred::Reply(s));
                self.submit(tx, Some(p), out);
                Exec::Done
            }
            None => self.reply_now(s, out),
        }
    }
}
