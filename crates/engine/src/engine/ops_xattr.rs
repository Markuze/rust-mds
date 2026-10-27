//! SETXATTR, RMXATTR and GETVXATTR (`Server::handle_client_setxattr`,
//! `handle_client_removexattr`, `handle_client_setvxattr`,
//! `handle_client_getvxattr`).
//!
//! Plain xattrs (`user.*`, `security.*`, `trusted.*`, `system.posix_acl_*`,
//! anything outside `ceph.*`) are stored opaquely; zero-length values are
//! values. `ceph.*` names other than the virtual ones and the two mirror names
//! are refused with EINVAL.

use super::vxattr::*;
use super::{Deferred, Engine, Exec, Pending, Txn};
use crate::consts::*;
use crate::effects::*;
use crate::errno::Errno;
use crate::inode::{HoldKind, SnapRealmNode};
use crate::journal::{Journal, RecordKind};
use crate::types::*;

pub(super) struct EnginePools<'a> {
    pub(super) names: &'a std::collections::BTreeMap<i64, String>,
}

impl PoolLookup for EnginePools<'_> {
    fn pool_by_name(&self, name: &str) -> Option<i64> {
        self.names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    }
    fn pool_exists(&self, id: i64) -> bool {
        self.names.contains_key(&id)
    }
}

impl<J: Journal> Engine<J> {
    pub(crate) fn op_setxattr(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        name: Vec<u8>,
        value: Vec<u8>,
        flags: u32,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let t = match self.resolve(&path) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        if let Ok(n) = std::str::from_utf8(&name)
            && is_ceph_vxattr(n)
        {
            let v = String::from_utf8_lossy(&value).into_owned();
            return self.op_set_vxattr(p, t, n.to_string(), v, false, out);
        }
        if !is_allowed_plain_ceph_xattr(&name) {
            return self.reply_error(p, Errno::EINVAL, out);
        }
        if !self.none_pending(&[t]) {
            return Exec::Wait;
        }
        if !self.acquire_holds(p, &[(t, HoldKind::XattrXlock)], out) {
            return Exec::Wait;
        }
        let core = &self.inodes[&t].core;
        let replace = flags & CEPH_XATTR_REPLACE != 0;
        let cur: u64 = core
            .xattrs
            .iter()
            .filter(|(k, _)| !(replace && **k == name))
            .map(|(k, v)| (k.len() + v.len()) as u64)
            .sum();
        if cur + (name.len() + value.len()) as u64 > self.config.max_xattr_size {
            return self.reply_error(p, Errno::ENOSPC, out);
        }
        let exists = core.xattrs.contains_key(&name);
        if flags & CEPH_XATTR_CREATE != 0 && exists {
            return self.reply_error(p, Errno::EEXIST, out);
        }
        if flags & (CEPH_XATTR_REPLACE | CEPH_XATTR_REMOVE2) != 0 && !exists {
            return self.reply_error(p, Errno::ENODATA, out);
        }
        let mut txn = Txn::new(RecordKind::Setxattr);
        self.touch(&mut txn, t);
        let c = &mut self.inodes.get_mut(&t).unwrap().core;
        if flags & (CEPH_XATTR_REMOVE | CEPH_XATTR_REMOVE2) != 0 {
            c.xattrs.remove(&name);
        } else {
            c.xattrs.insert(name, value);
        }
        c.xattr_version += 1;
        self.finish_attr_change(p, t, txn, out)
    }

    pub(crate) fn op_rmxattr(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        name: Vec<u8>,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let t = match self.resolve(&path) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        if let Ok(n) = std::str::from_utf8(&name)
            && is_ceph_vxattr(n)
        {
            return self.op_set_vxattr(p, t, n.to_string(), String::new(), true, out);
        }
        if !is_allowed_plain_ceph_xattr(&name) {
            return self.reply_error(p, Errno::EINVAL, out);
        }
        if !self.none_pending(&[t]) {
            return Exec::Wait;
        }
        if !self.acquire_holds(p, &[(t, HoldKind::XattrXlock)], out) {
            return Exec::Wait;
        }
        if !self.inodes[&t].core.xattrs.contains_key(&name) {
            return self.reply_error(p, Errno::ENODATA, out);
        }
        let mut txn = Txn::new(RecordKind::Rmxattr);
        self.touch(&mut txn, t);
        let c = &mut self.inodes.get_mut(&t).unwrap().core;
        c.xattrs.remove(&name);
        c.xattr_version += 1;
        self.finish_attr_change(p, t, txn, out)
    }

    /// The common tail of attribute changes: `ctime = stamp`,
    /// `change_attr += 1`, `rctime`, completed request, reply with the target.
    fn finish_attr_change(
        &mut self,
        p: &mut Pending,
        t: Ino,
        mut txn: Txn,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let stamp = p.req.stamp;
        self.touch(&mut txn, t);
        {
            let c = &mut self.inodes.get_mut(&t).unwrap().core;
            c.ctime = stamp;
            c.change_attr += 1;
        }
        self.update_file_rstat(&mut txn, t);
        self.bump_rctime(&mut txn, t, stamp);
        self.complete_request(p, &mut txn, None, None);
        let mut s = self.spec(p, 0);
        s.target = Some(t);
        txn.deferred.push(Deferred::Reply(s));
        self.submit(txn, Some(p), out);
        Exec::Done
    }

    fn pools(&self) -> EnginePools<'_> {
        EnginePools {
            names: &self.config.pool_names,
        }
    }

    fn layout_ok(&self, l: &FileLayout) -> bool {
        l.is_valid_geometry() && self.config.is_data_pool(l.pool_id)
    }

    /// Set or remove a `ceph.*` virtual xattr. Flags `CREATE`/`REPLACE` are
    /// ignored, as in the C++.
    fn op_set_vxattr(
        &mut self,
        p: &mut Pending,
        t: Ino,
        name: String,
        mut value: String,
        rm: bool,
        out: &mut Vec<Effect>,
    ) -> Exec {
        // Step preflight returns the untouched ordinary input on pressure.
        // Recheck here as well, before either allocating parser builds a tree.
        // Compatibility callers retain the request instead of parsing it.
        if !self.virtual_attr_parser_fits(t, &name, &value) {
            return Exec::Wait;
        }
        if !self.none_pending(&[t]) {
            return Exec::Wait;
        }
        let is_dir = self.inodes[&t].is_dir();
        let is_file = self.inodes[&t].is_file();
        let mut txn = Txn::new(RecordKind::SetVxattr);
        if let Some(field) = name.strip_prefix("ceph.dir.layout") {
            if !is_dir {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            let own = self.inodes[&t].core.layout.clone();
            if rm && field.is_empty() {
                if !own.is_set() {
                    return self.reply_error_ok(p, out);
                }
                self.touch(&mut txn, t);
                let c = &mut self.inodes.get_mut(&t).unwrap().core;
                c.layout = if t == CEPH_INO_ROOT {
                    self.config.default_layout()
                } else {
                    FileLayout::none()
                };
            } else {
                let mut layout = if own.is_set() {
                    own
                } else {
                    self.inherited_layout(t).0
                };
                let Some(field) = layout_subfield(field) else {
                    return self.reply_error(p, Errno::ENODATA, out);
                };
                match apply_layout_field(&mut layout, field, &value, &self.pools()) {
                    Ok(()) => {}
                    Err(Errno::ENOENT) => return self.reply_error(p, Errno::EINVAL, out),
                    Err(e) => return self.reply_error(p, e, out),
                }
                if !self.layout_ok(&layout) {
                    return self.reply_error(p, Errno::EINVAL, out);
                }
                self.touch(&mut txn, t);
                self.inodes.get_mut(&t).unwrap().core.layout = layout;
            }
        } else if let Some(field) = name.strip_prefix("ceph.file.layout") {
            if !is_file {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            let c = &self.inodes[&t].core;
            if !c.layout.is_set() {
                return self.reply_error_ok(p, out);
            }
            if c.size != 0 || c.truncate_seq > 1 {
                return self.reply_error(p, Errno::ENOTEMPTY, out);
            }
            let mut layout = c.layout.clone();
            let Some(field) = layout_subfield(field) else {
                return self.reply_error(p, Errno::ENODATA, out);
            };
            match apply_layout_field(&mut layout, field, &value, &self.pools()) {
                Ok(()) => {}
                Err(Errno::ENOENT) => return self.reply_error(p, Errno::EINVAL, out),
                Err(e) => return self.reply_error(p, e, out),
            }
            if !self.layout_ok(&layout) {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            if !self.acquire_holds(p, &[(t, HoldKind::FileXlock)], out) {
                return Exec::Wait;
            }
            self.touch(&mut txn, t);
            let c = &mut self.inodes.get_mut(&t).unwrap().core;
            let old_pool = c.layout.pool_id;
            c.old_pools.insert(old_pool);
            if c.layout != layout && !c.old_layouts.contains(&c.layout) {
                // Keep Vec growth explicit for retry metadata preflight.
                c.old_layouts.reserve_exact(1);
                c.old_layouts.push(c.layout.clone());
            }
            c.layout = layout;
        } else if let Some(field) = name.strip_prefix("ceph.quota") {
            if !is_dir {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            let mut quota = self.inodes[&t].core.quota;
            if rm {
                if !quota.is_enabled() {
                    return self.reply_error_ok(p, out);
                }
                value = "0".to_string();
            }
            let r = match field {
                "" => {
                    if value.is_empty() {
                        Ok(())
                    } else {
                        match parse_keys_and_values(&value) {
                            None => Err(Errno::EINVAL),
                            Some(m) => m
                                .iter()
                                .try_for_each(|(k, v)| set_quota_field(&mut quota, k, v)),
                        }
                    }
                }
                ".max_bytes" => set_quota_field(&mut quota, "max_bytes", &value),
                ".max_files" => set_quota_field(&mut quota, "max_files", &value),
                _ => Err(Errno::EINVAL),
            };
            if let Err(e) = r {
                return self.reply_error(p, e, out);
            }
            if !quota.is_valid() {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            if quota == self.inodes[&t].core.quota {
                return self.reply_error_ok(p, out);
            }
            self.touch(&mut txn, t);
            let c = &mut self.inodes.get_mut(&t).unwrap().core;
            c.quota = quota;
            if quota.is_enabled() && c.snaprealm.is_none() {
                // Every quota directory roots a snap realm (clients walk
                // realms to find quotas).
                c.snaprealm = Some(SnapRealmNode {
                    seq: 1,
                    created: 1,
                    ..Default::default()
                });
                txn.deferred.push(Deferred::RealmSplit { realm: t });
            }
        } else if name == "ceph.dir.subvolume" {
            if !is_dir {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            let cur = self.inodes[&t]
                .core
                .snaprealm
                .as_ref()
                .is_some_and(|r| r.subvolume);
            if rm {
                if !cur {
                    return self.reply_error_ok(p, out);
                }
                value = "0".to_string();
            }
            let Some(val) = parse_bool(&value) else {
                return self.reply_error(p, Errno::EINVAL, out);
            };
            if val == cur {
                // mgr/volumes sets the flag on every open: a cheap no-op.
                return self.reply_error_ok(p, out);
            }
            // Durable subvolume adoption/clearing requires the G4 protocol.
            // Keep the prototype helper behavior outside production stepping.
            if self.stepping && ((!val) || !self.inodes[&t].entries.is_empty()) {
                return self.reply_error(p, Errno::EOPNOTSUPP, out);
            }
            if val {
                let outer = self.subvolume_of(t);
                if outer != 0 && outer != t {
                    return self.reply_error(p, Errno::EINVAL, out);
                }
            }
            self.touch(&mut txn, t);
            let c = &mut self.inodes.get_mut(&t).unwrap().core;
            if val && c.stable_subvolume_id == 0 {
                c.stable_subvolume_id = t;
            }
            match &mut c.snaprealm {
                Some(r) => r.subvolume = val,
                None => {
                    c.snaprealm = Some(SnapRealmNode {
                        seq: 1,
                        created: 1,
                        subvolume: val,
                        ..Default::default()
                    });
                    txn.deferred.push(Deferred::RealmSplit { realm: t });
                }
            }
        } else if name == "ceph.dir.pin" {
            if !is_dir || t == CEPH_INO_ROOT {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            if rm {
                if self.inodes[&t].core.export_pin == MDS_RANK_NONE {
                    return self.reply_error_ok(p, out);
                }
                value = "-1".to_string();
            }
            let Some(rank) = parse_i64(&value)
                .filter(|v| *v >= i64::from(i32::MIN) && *v <= i64::from(i32::MAX))
            else {
                return self.reply_error(p, Errno::EINVAL, out);
            };
            let rank = if rank < 0 { MDS_RANK_NONE } else { rank as i32 };
            if rank >= MAX_MDS {
                return self.reply_error(p, Errno::EDOM, out);
            }
            self.touch(&mut txn, t);
            self.inodes.get_mut(&t).unwrap().core.export_pin = rank;
        } else if name == "ceph.dir.pin.random" {
            if !is_dir || t == CEPH_INO_ROOT {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            if rm {
                if self.inodes[&t].core.export_pin_random == 0.0 {
                    return self.reply_error_ok(p, out);
                }
                value = "0".to_string();
            }
            let Ok(val) = value.parse::<f64>() else {
                return self.reply_error(p, Errno::EINVAL, out);
            };
            if !(0.0..=1.0).contains(&val) {
                return self.reply_error(p, Errno::EDOM, out);
            }
            if val > self.config.export_ephemeral_random_max {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            self.touch(&mut txn, t);
            self.inodes.get_mut(&t).unwrap().core.export_pin_random = val;
        } else if name == "ceph.dir.pin.distributed" {
            if !is_dir || t == CEPH_INO_ROOT {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            if rm {
                if !self.inodes[&t].core.export_pin_distributed {
                    return self.reply_error_ok(p, out);
                }
                value = "0".to_string();
            }
            let Some(val) = parse_bool(&value) else {
                return self.reply_error(p, Errno::EINVAL, out);
            };
            self.touch(&mut txn, t);
            self.inodes.get_mut(&t).unwrap().core.export_pin_distributed = val;
        } else if name == "ceph.quiesce.block" || name == "ceph.dir.subvolume.snaps.visible" {
            // Accepted and ignored in P0 (quiesce is P2, snapshots P1).
            if name == "ceph.dir.subvolume.snaps.visible" && !is_dir {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            if parse_bool(&value).is_none() {
                return self.reply_error(p, Errno::EINVAL, out);
            }
            return self.reply_error_ok(p, out);
        } else {
            // Charmap names: case-insensitive directories are P2.
            return self.reply_error(p, Errno::EOPNOTSUPP, out);
        }
        self.finish_attr_change(p, t, txn, out)
    }

    /// GETVXATTR: values of the `ceph.*` virtual xattrs the MDS serves;
    /// ENODATA for every other name (mgr/volumes reads unknown ones and only
    /// accepts ENODATA as "not set").
    pub(crate) fn op_getvxattr(
        &mut self,
        p: &mut Pending,
        path: FilePath,
        name: Vec<u8>,
        out: &mut Vec<Effect>,
    ) -> Exec {
        let Ok(name) = String::from_utf8(name) else {
            return self.reply_error(p, Errno::ENODATA, out);
        };
        if !is_ceph_vxattr(&name) {
            return self.reply_error(p, Errno::ENODATA, out);
        }
        let t = match self.resolve(&path) {
            Ok(t) => t,
            Err(e) => return self.reply_error(p, e, out),
        };
        let is_dir = self.inodes[&t].is_dir();
        if (is_ceph_dir_vxattr(&name) && !is_dir) || (is_ceph_file_vxattr(&name) && is_dir) {
            return self.reply_error(p, Errno::ENODATA, out);
        }
        if !self.none_pending(&[t]) {
            return Exec::Wait;
        }
        match self.vxattr_value(t, &name) {
            Ok(v) => {
                let mut s = self.spec(p, 0);
                s.extra = ReplyExtra::Vxattr(v.into_bytes());
                self.reply_now(s, out)
            }
            Err(e) => self.reply_error(p, e, out),
        }
    }

    fn pool_name(&self, id: i64) -> Option<&str> {
        self.config.pool_names.get(&id).map(|s| s.as_str())
    }

    /// The value of a virtual xattr of inode `t`.
    pub fn vxattr_value(&self, t: Ino, name: &str) -> Result<String, Errno> {
        let i = self.inodes.get(&t).ok_or(Errno::ESTALE)?;
        let c = &i.core;
        let layout_field = name
            .strip_prefix("ceph.dir.layout")
            .or_else(|| name.strip_prefix("ceph.file.layout"));
        if let Some(field) = layout_field {
            let own = &c.layout;
            // Inherited layout and its status for the C++-served names.
            let (inh, from) = self.inherited_layout(t);
            let status = if inh == self.config.default_layout() {
                "default"
            } else if from == t {
                "set"
            } else {
                "inherited"
            };
            return match field {
                ".json" => Ok(layout_json(&inh, self.pool_name(inh.pool_id), status)),
                ".pool_name" => Ok(self.pool_name(inh.pool_id).unwrap_or("").to_string()),
                ".pool_id" => Ok((inh.pool_id as u64).to_string()),
                _ if !own.is_set() => Err(Errno::ENODATA),
                "" => Ok(layout_string(own, self.pool_name(own.pool_id))),
                ".stripe_unit" => Ok(own.stripe_unit.to_string()),
                ".stripe_count" => Ok(own.stripe_count.to_string()),
                ".object_size" => Ok(own.object_size.to_string()),
                ".pool" => Ok(self
                    .pool_name(own.pool_id)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| own.pool_id.to_string())),
                ".pool_namespace" => Ok(own.pool_ns.clone()),
                _ => Err(Errno::ENODATA),
            };
        }
        if let Some(field) = name.strip_prefix("ceph.quota") {
            if !i.is_dir() || !c.quota.is_enabled() || c.snaprealm.is_none() {
                return Err(Errno::ENODATA);
            }
            return match field {
                "" => Ok(format!(
                    "max_bytes={} max_files={}",
                    c.quota.max_bytes, c.quota.max_files
                )),
                ".max_bytes" => Ok(c.quota.max_bytes.to_string()),
                ".max_files" => Ok(c.quota.max_files.to_string()),
                _ => Err(Errno::ENODATA),
            };
        }
        match name {
            "ceph.dir.subvolume" => Ok(if c.snaprealm.as_ref().is_some_and(|r| r.subvolume) {
                "1"
            } else {
                "0"
            }
            .to_string()),
            "ceph.dir.pin" => Ok(c.export_pin.to_string()),
            "ceph.dir.pin.random" => Ok(format_double(c.export_pin_random)),
            "ceph.dir.pin.distributed" => {
                Ok(if c.export_pin_distributed { "1" } else { "0" }.to_string())
            }
            "ceph.quiesce.block" => Ok("0".to_string()),
            "ceph.dir.subvolume.snaps.visible" => Ok(if c.snaprealm.is_some() {
                "1".to_string()
            } else {
                String::new()
            }),
            _ => Err(Errno::ENODATA),
        }
    }
}

/// What follows `ceph.{dir,file}.layout`: `""` (composite) or `.field`.
pub(super) fn layout_subfield(rest: &str) -> Option<&str> {
    if rest.is_empty() {
        return Some("");
    }
    rest.strip_prefix('.').filter(|f| !f.is_empty())
}

/// `Server::parse_quota_vxattr` for one field.
fn set_quota_field(q: &mut Quota, k: &str, v: &str) -> Result<(), Errno> {
    match k {
        "max_bytes" => q.max_bytes = parse_iec_i64(v).ok_or(Errno::EINVAL)?,
        "max_files" => {
            let n = parse_i64(v).ok_or(Errno::EINVAL)?;
            if n < 0 {
                return Err(Errno::EINVAL);
            }
            q.max_files = n;
        }
        _ => return Err(Errno::EINVAL),
    }
    Ok(())
}
