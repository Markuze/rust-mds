//! Cap policy, issue and revocation, and the inode attributes sent with caps.
//!
//! Policy (the client-visible outcome of `src/mds/locks.c` and
//! `Locker::file_eval`/`simple_eval`/`CInode::calc_ideal_loner`):
//!
//! * **Loner.** For a regular file the loner is the only non-stale client
//!   that wants `ANY_WR|Fr` or still holds write caps; for a directory, the
//!   only non-stale cap holder.
//! * **File bits, regular files.** A loner that wants or holds write caps
//!   gets `Fs Fx Fc Fr Fw Fb` and nobody else gets file bits (`LOCK_EXCL`);
//!   otherwise, if anybody wants `Fw`, everybody gets `Fr Fw Fl` (`LOCK_MIX`:
//!   synchronous I/O, no `Fb`/`Fc`); otherwise everybody gets `Fs Fc Fr Fl`
//!   (`LOCK_SYNC`). `Fl` is only issued when wanted.
//! * **File bits, directories.** The loner gets `Fs Fx`, everybody else `Fs`
//!   (dentry caching). `Fc`/`Fr` (async create/unlink, `CEPH_CAP_ANY_DIR_OPS`)
//!   and `Fw`/`Fb` are never issued on directories.
//! * **Auth, link, xattr bits.** The loner gets `s x` when it wants `x`
//!   (e.g. `Ax Xx` for a write open); otherwise everybody gets `s`.
//! * **Holds** of requests in progress remove bits from the targets.
//!
//! Issuing follows `Locker::issue_caps`: bits outside the target are revoked
//! (a revoking message never adds bits); missing wanted bits are granted
//! together with the "liked" ones, but a bit is only *added* when it does not
//! conflict with what other clients still have issued (so a grant waits for
//! the acknowledgement of a conflicting revocation). Caps carried in replies
//! never revoke (`issue_norevoke`).

use std::collections::{BTreeMap, BTreeSet};

use super::Engine;
use crate::caps::{Capability, conflicting_bits};
use crate::consts::*;
use crate::effects::*;
use crate::journal::{Journal, RecordKind};
use crate::types::*;

const P: u32 = CEPH_CAP_PIN;
const AS: u32 = CEPH_CAP_AUTH_SHARED;
const AX: u32 = CEPH_CAP_AUTH_EXCL;
const LS: u32 = CEPH_CAP_LINK_SHARED;
const LX: u32 = CEPH_CAP_LINK_EXCL;
const XS: u32 = CEPH_CAP_XATTR_SHARED;
const XX: u32 = CEPH_CAP_XATTR_EXCL;
const FS: u32 = CEPH_CAP_FILE_SHARED;
const FX: u32 = CEPH_CAP_FILE_EXCL;
const FC: u32 = CEPH_CAP_FILE_CACHE;
const FR: u32 = CEPH_CAP_FILE_RD;
const FW: u32 = CEPH_CAP_FILE_WR;
const FB: u32 = CEPH_CAP_FILE_BUFFER;
const FL: u32 = CEPH_CAP_FILE_LAZYIO;

impl<J: Journal> Engine<J> {
    pub(crate) fn is_client_stale(&self, client: ClientId) -> bool {
        self.sessions.get(&client).is_none_or(|s| s.is_stale())
    }

    /// `CInode::get_caps_liked`: what the MDS issues along with wanted caps.
    pub(crate) fn liked(&self, ino: Ino) -> u32 {
        match self.inodes.get(&ino) {
            Some(i) if i.is_dir() => P | CEPH_CAP_ANY_EXCL | CEPH_CAP_ANY_SHARED,
            _ => CEPH_CAP_ANY & !FL,
        }
    }

    /// The loner of an inode, if any. Stale sessions do not compete for it,
    /// but a client that already holds exclusive caps keeps them while no
    /// live client is active on the inode (`try_drop_loner` fails while the
    /// old loner still has caps others may not hold), so a session that is
    /// only briefly stale is not stripped of its caps for nothing.
    pub(crate) fn loner(&self, ino: Ino) -> Option<ClientId> {
        let i = self.inodes.get(&ino)?;
        let nonstale: Vec<ClientId> = i
            .caps
            .keys()
            .copied()
            .filter(|c| !self.is_client_stale(*c))
            .collect();
        let holders = |bits: u32| -> Vec<ClientId> {
            i.caps
                .iter()
                .filter(|(_, cap)| cap.issued() & bits != 0)
                .map(|(c, _)| *c)
                .collect()
        };
        if i.is_dir() {
            return match nonstale.len() {
                1 => Some(nonstale[0]),
                0 => {
                    let h = holders(FX);
                    if h.len() == 1 { Some(h[0]) } else { None }
                }
                _ => None,
            };
        }
        let active: Vec<ClientId> = nonstale
            .into_iter()
            .filter(|c| {
                let cap = &i.caps[c];
                cap.wanted() & (CEPH_CAP_ANY_WR | FR) != 0 || cap.issued() & CEPH_CAP_ANY_WR != 0
            })
            .collect();
        match active.len() {
            1 => Some(active[0]),
            0 => {
                let h = holders(CEPH_CAP_ANY_WR);
                if h.len() == 1 { Some(h[0]) } else { None }
            }
            _ => None,
        }
    }

    /// Per-client target cap sets of an inode under the current policy and
    /// holds.
    pub(crate) fn cap_targets(&self, ino: Ino) -> BTreeMap<ClientId, u32> {
        let mut out = BTreeMap::new();
        let Some(i) = self.inodes.get(&ino) else {
            return out;
        };
        let is_dir = i.is_dir();
        let loner = self.loner(ino);
        let any_wants_w = i
            .caps
            .iter()
            .any(|(c, cap)| !self.is_client_stale(*c) && cap.wanted() & FW != 0);
        let file_excl = !is_dir
            && loner.is_some_and(|l| {
                let cap = &i.caps[&l];
                (cap.wanted() | cap.issued()) & CEPH_CAP_ANY_FILE_WR != 0
            });
        for (c, _cap) in &i.caps {
            let mut t = P;
            t |= if is_dir {
                if Some(*c) == loner { FS | FX } else { FS }
            } else if file_excl {
                if Some(*c) == loner {
                    FS | FX | FC | FR | FW | FB
                } else {
                    0
                }
            } else if any_wants_w {
                FR | FW | FL
            } else {
                FS | FC | FR | FL
            };
            for (s, x) in [(AS, AX), (LS, LX), (XS, XX)] {
                match loner {
                    Some(l) if i.caps[&l].wanted() & x != 0 => {
                        if *c == l {
                            t |= s | x;
                        }
                    }
                    _ => t |= s,
                }
            }
            for h in &i.holds {
                t &= !h.excluded_for(*c);
            }
            if is_dir {
                t &= !(CEPH_CAP_ANY_DIR_OPS | FW | FB);
            }
            if self.config.read_only {
                t &= !CEPH_CAP_ANY_WR;
            }
            if self
                .jobs
                .values()
                .any(|j| j.ino == ino && matches!(j.kind, crate::LifecycleKind::RecoverSize { .. }))
            {
                // Stop remaining writers before probing a lost writer's bound.
                t &= !CEPH_CAP_ANY_WR;
            }
            out.insert(*c, t);
        }
        out
    }

    /// Bits of `target` the client may be given now: new bits must not
    /// conflict with caps other clients still have issued.
    fn grantable(&self, ino: Ino, client: ClientId, target: u32) -> u32 {
        let Some(i) = self.inodes.get(&ino) else {
            return 0;
        };
        let pending = i.caps.get(&client).map(|c| c.pending()).unwrap_or(0);
        let others = i.issued_by_others(client);
        target & !conflicting_bits(target & !pending, others)
    }

    /// Create the client's cap on an inode if it has none. `cap_id_one` for
    /// the creator of a new inode (`CInode::add_client_cap(..., new_inode)`).
    pub(crate) fn ensure_cap(&mut self, ino: Ino, client: ClientId, cap_id_one: bool) {
        let cap_gen = self.sessions.get(&client).map(|s| s.cap_gen).unwrap_or(0);
        let Some(i) = self.inodes.get(&ino) else {
            return;
        };
        if i.caps.contains_key(&client) {
            return;
        }
        let id = if cap_id_one {
            1
        } else {
            self.last_cap_id += 1;
            self.last_cap_id
        };
        self.inodes
            .get_mut(&ino)
            .unwrap()
            .caps
            .insert(client, Capability::new(id, cap_gen));
        if let Some(s) = self.sessions.get_mut(&client) {
            s.caps.insert(ino);
        }
        // Another cap holder may change the policy (loner, exclusive caps).
        self.eval_queue.insert(ino);
    }

    /// Re-evaluate caps of an inode: revoke what the policy no longer
    /// allows, grant missing wanted bits that can be granted.
    pub(crate) fn eval_caps(&mut self, ino: Ino, out: &mut Vec<Effect>) {
        if self.is_failed() {
            return;
        }
        self.eval_queue.remove(&ino);
        if !self.inodes.contains_key(&ino) {
            return;
        }
        // Policy also reads topology and link state. Re-evaluate only after
        // all submitted images are durable; acknowledgments still proceed.
        if !self.commits.is_empty() {
            self.eval_queue.insert(ino);
            return;
        }
        let targets = self.cap_targets(ino);
        let liked = self.liked(ino);
        let nlink0 = self.inodes[&ino].core.nlink == 0;
        for (c, target) in targets {
            let stale = self.is_client_stale(c);
            let sess_gen = self.sessions.get(&c).map(|s| s.cap_gen).unwrap_or(0);
            if stale {
                let cap = self.inodes.get_mut(&ino).unwrap().caps.get_mut(&c).unwrap();
                let lose = cap.pending() & !target;
                if lose != 0 {
                    if lose & CEPH_CAP_ANY_WR != 0 {
                        if self.evicting.insert(c) {
                            out.push(Effect::EvictClient { client: c });
                        }
                    } else {
                        // `Locker::revoke_stale_cap`: force the revocation.
                        let keep = cap.pending() & target;
                        cap.issue(keep);
                        cap.force_revoke();
                    }
                }
                continue;
            }
            let grantable = self.grantable(ino, c, target);
            let cap = self.inodes.get_mut(&ino).unwrap().caps.get_mut(&c).unwrap();
            let valid = cap.cap_gen == sess_gen;
            let mut wanted = cap.wanted();
            if nlink0 {
                // Notify clients of the unlink so they release caps promptly.
                wanted |= LS;
            }
            let pending = cap.pending();
            if !valid {
                cap.cap_gen = sess_gen;
                if pending & !P != 0 && pending & !target != 0 {
                    // After a stale period the client believes it holds only
                    // `p`: re-issue what it has, then revoke (`issue_caps`).
                    cap.issue(pending);
                    let m = self.cap_message(ino, c, CEPH_CAP_OP_GRANT);
                    out.push(Effect::Caps(m));
                }
            }
            let cap = self.inodes.get_mut(&ino).unwrap().caps.get_mut(&c).unwrap();
            let pending = cap.pending();
            let revoke = pending & !target;
            let missing = (wanted & grantable) & !pending;
            if revoke == 0 && missing == 0 && valid {
                continue;
            }
            if revoke == 0 && cap.is_new {
                // The reply of the request that created the cap carries it.
                continue;
            }
            let new = if revoke != 0 {
                (wanted | liked) & target & pending
            } else {
                (wanted | liked) & grantable
            };
            if new == pending && valid {
                continue;
            }
            cap.issue(new);
            let op = if pending & !new != 0 {
                CEPH_CAP_OP_REVOKE
            } else {
                CEPH_CAP_OP_GRANT
            };
            let m = self.cap_message(ino, c, op);
            out.push(Effect::Caps(m));
        }
    }

    /// Evaluate every inode queued by cap creation.
    pub(crate) fn drain_evals(&mut self, out: &mut Vec<Effect>) {
        if !self.commits.is_empty() {
            return;
        }
        if self.stepping {
            if let Some(ino) = self.eval_queue.pop_first() {
                self.eval_caps(ino, out);
            }
            return;
        }
        while let Some(ino) = self.eval_queue.pop_first() {
            self.eval_caps(ino, out);
        }
    }

    /// Build a GRANT/REVOKE/TRUNC message from the cap's current state.
    pub(crate) fn cap_message(&mut self, ino: Ino, client: ClientId, op: u32) -> CapMessage {
        let realm = self.realm_of(ino);
        let fields = self.cap_inode_fields(ino, client);
        let cap = &self.inodes[&ino].caps[&client];
        CapMessage {
            client,
            op,
            ino,
            realm,
            cap_id: cap.cap_id,
            seq: cap.last_sent,
            issue_seq: cap.last_issue,
            caps: cap.pending(),
            wanted: cap.wanted(),
            dirty: 0,
            mseq: cap.mseq,
            flush_tid: 0,
            oldest_flush_tid: 0,
            inode: Some(fields),
        }
    }

    /// Inode fields for a cap message (`CInode::encode_cap_message`); the
    /// xattr blob is included when the client gets `Xs` and lacks the
    /// current version.
    fn cap_inode_fields(&mut self, ino: Ino, client: ClientId) -> CapInodeFields {
        let i = self.inodes.get_mut(&ino).unwrap();
        let core = &self.committed_inodes[&ino];
        let max_size = core.client_ranges.get(&client).copied().unwrap_or(0);
        let mut xattrs = None;
        if let Some(cap) = i.caps.get_mut(&client)
            && cap.pending() & XS != 0
            && core.xattr_version > cap.client_xattr_version
        {
            cap.client_xattr_version = core.xattr_version;
            xattrs = Some((core.xattr_version, core.xattrs.clone()));
        }
        CapInodeFields {
            uid: core.uid,
            gid: core.gid,
            mode: core.mode,
            nlink: core.nlink.max(0) as u32,
            xattrs,
            size: core.size,
            max_size,
            truncate_size: core.truncate_size,
            truncate_seq: core.truncate_seq,
            mtime: core.mtime,
            atime: core.atime,
            ctime: core.ctime,
            btime: core.btime,
            layout: core.layout.clone(),
            time_warp_seq: core.time_warp_seq,
            change_attr: core.change_attr,
            nfiles: core.dirstat.nfiles,
            nsubdirs: core.dirstat.nsubdirs,
            inline_version: core.inline_version,
        }
    }

    /// `CInode::encode_inodestat`: attributes for a reply trace or readdir
    /// entry, issuing caps to the client unless `issue` is false (readdir
    /// entries in another realm) or its session is stale.
    pub(crate) fn inode_stat(
        &mut self,
        ino: Ino,
        client: ClientId,
        mask: u32,
        issue: bool,
    ) -> InodeStat {
        let realm = self.realm_of(ino);
        let subvolume_id = self.subvolume_of(ino);
        let stale = self.is_client_stale(client);
        let mut cap_rec = ReplyCap {
            realm,
            ..Default::default()
        };
        let mut include_xattrs = mask & XS != 0;
        if issue && !stale {
            self.ensure_cap(ino, client, false);
            let target = self.cap_targets(ino).get(&client).copied().unwrap_or(P);
            let grantable = self.grantable(ino, client, target);
            let liked = self.liked(ino);
            let xattr_version = self.committed_inodes[&ino].xattr_version;
            let cap = self
                .inodes
                .get_mut(&ino)
                .unwrap()
                .caps
                .get_mut(&client)
                .unwrap();
            let was_new = cap.is_new;
            // Like C++ `Locker::eval`, a traced reply grants the cap's
            // wanted bits, and the liked set only when the cap is new
            // (`issue_caps` for a fresh cap). Re-granting liked `Fs` on
            // every later reply would leave directory-content caching
            // continuously enabled: the kernel then never revalidates
            // through the MDS (`i_shared_gen` stays put) and answers
            // lookups from stale negative dentries after an eviction.
            let liked_grant = if was_new { liked } else { 0 };
            cap.issue_norevoke((cap.wanted() | liked_grant) & grantable | P);
            cap.set_last_issue();
            let pending = cap.pending();
            include_xattrs = (pending | mask) & XS != 0
                && (was_new || cap.client_xattr_version < xattr_version || mask & XS != 0);
            if include_xattrs {
                cap.client_xattr_version = xattr_version;
            }
            cap_rec = ReplyCap {
                caps: pending,
                wanted: cap.wanted(),
                cap_id: cap.cap_id,
                seq: cap.last_sent,
                mseq: cap.mseq,
                realm,
                flags: CEPH_CAP_FLAG_AUTH,
            };
        }
        let core = &self.committed_inodes[&ino];
        let max_size = core.client_ranges.get(&client).copied().unwrap_or(0);
        InodeStat {
            ino,
            snapid: CEPH_NOSNAP,
            rdev: core.rdev,
            version: core.version * 2,
            xattr_version: if include_xattrs {
                core.xattr_version
            } else {
                0
            },
            cap: cap_rec,
            layout: core.layout.clone(),
            ctime: core.ctime,
            mtime: core.mtime,
            atime: core.atime,
            btime: core.btime,
            time_warp_seq: core.time_warp_seq,
            size: core.size,
            max_size,
            truncate_size: core.truncate_size,
            truncate_seq: core.truncate_seq,
            mode: core.mode,
            uid: core.uid,
            gid: core.gid,
            nlink: core.nlink.max(0) as u32,
            dirstat: core.dirstat,
            rstat: core.rstat,
            dir_hash: core.dir_hash,
            symlink: core.symlink.clone(),
            xattrs: if include_xattrs {
                Some(core.xattrs.clone())
            } else {
                None
            },
            inline_version: core.inline_version,
            quota: core.quota,
            change_attr: core.change_attr,
            export_pin: core.export_pin,
            subvolume_id,
            fscrypt_auth: core.fscrypt_auth.clone(),
            fscrypt_file: core.fscrypt_file.clone(),
        }
    }

    /// GRANT carrying a new `max_size` (`Locker::share_inode_max_size`):
    /// only to a cap with `Fw|Fb` pending.
    pub(crate) fn send_max_size_grant(
        &mut self,
        ino: Ino,
        client: ClientId,
        out: &mut Vec<Effect>,
    ) {
        let Some(cap) = self
            .inodes
            .get_mut(&ino)
            .and_then(|i| i.caps.get_mut(&client))
        else {
            return;
        };
        if cap.pending() & (FW | FB) == 0 || cap.is_new {
            return;
        }
        cap.inc_last_seq();
        let m = self.cap_message(ino, client, CEPH_CAP_OP_GRANT);
        out.push(Effect::Caps(m));
    }

    /// `Locker::issue_truncate`: TRUNC to every cap holder (no seq
    /// increment), then the data truncation for the daemon.
    pub(crate) fn send_truncate(&mut self, ino: Ino, out: &mut Vec<Effect>) {
        let Some(i) = self.inodes.get(&ino) else {
            return;
        };
        let clients: Vec<ClientId> = i.caps.keys().copied().collect();
        for c in clients {
            if self.inodes[&ino].caps[&c].is_new {
                continue;
            }
            let m = self.cap_message(ino, c, CEPH_CAP_OP_TRUNC);
            out.push(Effect::Caps(m));
        }
        let core = &self.committed_inodes[&ino];
        out.push(Effect::TruncateData(TruncateData {
            ino,
            layout: core.layout.clone(),
            truncate_seq: core.truncate_seq,
            size: core.truncate_size,
            from: core.truncate_from,
        }));
    }

    /// MClientSnap SPLIT for a newly opened realm: every client with caps on
    /// inodes below it learns which inodes and child realms moved.
    pub(crate) fn send_realm_split(&mut self, realm: Ino, out: &mut Vec<Effect>) {
        let mut per_client: BTreeMap<ClientId, (Vec<Ino>, BTreeSet<Ino>)> = BTreeMap::new();
        let mut stack = vec![realm];
        let mut child_realms = Vec::new();
        while let Some(ino) = stack.pop() {
            let Some(i) = self.inodes.get(&ino) else {
                continue;
            };
            if ino != realm && i.core.snaprealm.is_some() {
                child_realms.push(ino);
                for c in i.caps.keys() {
                    per_client.entry(*c).or_default().1.insert(ino);
                }
                continue;
            }
            for c in i.caps.keys() {
                per_client.entry(*c).or_default().0.push(ino);
            }
            for d in i.entries.values() {
                if d.primary {
                    stack.push(d.ino);
                }
            }
        }
        let trace = self.snap_trace(realm);
        for (client, (mut inos, _)) in per_client {
            if !self.sessions.contains_key(&client) {
                continue;
            }
            inos.sort_unstable();
            out.push(Effect::RealmSplit(RealmSplit {
                client,
                realm,
                split_inos: inos,
                split_realms: child_realms.clone(),
                snap_trace: trace.clone(),
            }));
        }
    }

    /// Snap realm info for building snap traces.
    pub fn snap_realm_info(&self, realm: Ino) -> Option<SnapRealmInfo> {
        let i = self.inodes.get(&realm)?;
        let node = i.core.snaprealm.as_ref()?;
        let parent = match &i.parent {
            Some((p, _)) => self.realm_of(*p),
            None => 0,
        };
        Some(SnapRealmInfo {
            ino: realm,
            parent,
            seq: node.seq,
            created: node.created,
            parent_since: if parent == 0 { 0 } else { node.created },
            subvolume: node.subvolume,
            last_modified: node.last_modified,
            change_attr: node.change_attr,
        })
    }

    /// The realm chain of an inode, innermost first (what a snap trace
    /// encodes: the inode's realm followed by its ancestors).
    pub fn snap_trace(&self, ino: Ino) -> Vec<SnapRealmInfo> {
        let mut out = Vec::new();
        let mut r = self.realm_of(ino);
        while let Some(info) = self.snap_realm_info(r) {
            let parent = info.parent;
            out.push(info);
            if parent == 0 {
                break;
            }
            r = parent;
        }
        out
    }

    /// Purge an unlinked inode once no cap and no uncommitted record refers
    /// to it (`StrayManager::_eval_stray`, reduced).
    pub(crate) fn maybe_purge_stray(&mut self, ino: Ino, out: &mut Vec<Effect>) {
        // Other gather members can still reconstruct caps on this inode.
        if !self.recovery_ready {
            return;
        }
        let Some(i) = self.inodes.get(&ino) else {
            return;
        };
        if i.core.nlink != 0 || !i.caps.is_empty() || i.pending != 0 || ino == CEPH_INO_ROOT {
            return;
        }
        if self
            .jobs
            .values()
            .any(|j| j.ino == ino && !matches!(j.kind, crate::LifecycleKind::RecoverSize { .. }))
        {
            return;
        }
        let mut txn = super::Txn::new(RecordKind::Purge);
        self.purge_inode(&mut txn, ino);
        self.submit(txn, None, out);
    }

    /// Remove an inode for good within `txn` and schedule its data purge.
    pub(crate) fn purge_inode(&mut self, txn: &mut super::Txn, ino: Ino) {
        let Some(i) = self.inodes.remove(&ino) else {
            return;
        };
        txn.rec.purged.push(ino);
        txn.imaged.remove(&ino);
        txn.touched.remove(&ino);
        for c in i.caps.keys() {
            if let Some(s) = self.sessions.get_mut(c) {
                s.caps.remove(&ino);
            }
        }
        self.lock_inodes.remove(&ino);
        let mut size = i
            .core
            .size
            .max(i.core.max_size_ever)
            .max(i.core.max_client_range());
        let mut layouts = vec![i.core.layout.clone()];
        for old in &i.core.old_layouts {
            if !layouts.contains(old) {
                layouts.push(old.clone());
            }
        }
        for pool in &i.core.old_pools {
            if !layouts.iter().any(|l| l.pool_id == *pool) {
                let mut layout = i.core.layout.clone();
                layout.pool_id = *pool;
                layouts.push(layout);
            }
        }
        // No client claimed the stray. Replace any size probe with a complete
        // purge intent in the same transaction as inode retirement. A late
        // probe result then names a retired job and cannot change metadata.
        for old in self.jobs.values().filter(|j| j.ino == ino) {
            if let crate::LifecycleKind::RecoverSize {
                layouts: old_layouts,
                max_size,
                ..
            } = &old.kind
            {
                size = size.max(*max_size);
                for layout in old_layouts {
                    if !layouts.contains(layout) {
                        layouts.push(layout.clone());
                    }
                }
                txn.rec.completed_jobs.push(old.id);
            }
        }
        let job = crate::LifecycleIntent {
            id: self.next_job_id,
            ino,
            kind: crate::LifecycleKind::Purge {
                layouts,
                max_size: size,
            },
        };
        self.next_job_id += 1;
        txn.rec.lifecycle_intents.push(job.clone());
        txn.deferred.push(super::Deferred::StartJob(job));
    }

    /// Check the invariant that issued caps of different clients never
    /// conflict. Used by tests.
    pub fn check_cap_invariants(&self) -> Result<(), String> {
        for (ino, i) in &self.inodes {
            for (c, cap) in &i.caps {
                if cap.pending() & !cap.issued() != 0 {
                    return Err(format!("{ino:#x} client {c}: pending not within issued"));
                }
                let others = i.issued_by_others(*c);
                let bad = conflicting_bits(cap.issued(), others);
                if bad != 0 && !self.is_client_stale(*c) {
                    return Err(format!(
                        "{ino:#x} client {c}: issued {} conflicts with others {}",
                        crate::caps::cap_string(cap.issued()),
                        crate::caps::cap_string(others)
                    ));
                }
                if i.is_dir() && cap.issued() & (CEPH_CAP_ANY_DIR_OPS | FW | FB) != 0 {
                    return Err(format!(
                        "{ino:#x} client {c}: directory cap {}",
                        crate::caps::cap_string(cap.issued())
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_checker_rejects_conflicting_caps_and_directory_write_caps() {
        let mut e = Engine::new(Default::default(), crate::MemJournal::new());
        e.open_session(1);
        e.open_session(2);
        e.ensure_cap(CEPH_INO_ROOT, 1, false);
        e.ensure_cap(CEPH_INO_ROOT, 2, false);
        e.inodes
            .get_mut(&CEPH_INO_ROOT)
            .unwrap()
            .caps
            .get_mut(&1)
            .unwrap()
            .issue_norevoke(P | AX);
        e.inodes
            .get_mut(&CEPH_INO_ROOT)
            .unwrap()
            .caps
            .get_mut(&2)
            .unwrap()
            .issue_norevoke(P | AS);
        assert!(e.check_cap_invariants().unwrap_err().contains("conflicts"));
        e.inodes.get_mut(&CEPH_INO_ROOT).unwrap().caps.remove(&2);
        e.inodes
            .get_mut(&CEPH_INO_ROOT)
            .unwrap()
            .caps
            .get_mut(&1)
            .unwrap()
            .issue_norevoke(FW);
        assert!(
            e.check_cap_invariants()
                .unwrap_err()
                .contains("directory cap")
        );
    }
}
