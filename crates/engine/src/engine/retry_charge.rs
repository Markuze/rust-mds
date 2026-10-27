//! Deterministic retry metadata accounting. This is not an RSS estimate.
//! Projection stores scalar charges, not speculative inode/cap copies.
use super::*;
use crate::inode::{Dentry, DentryKey};
use std::mem::size_of;

#[cfg(test)]
#[path = "retry_charge_tests.rs"]
mod tests;

pub(super) const TRANSIENT_LIMIT: usize = 8 * 1024 * 1024;
const COMPLETION: usize = 256;
const FIXED_INODE: usize = 4096;
const INDEX: usize = 256 + size_of::<(Ino, usize)>();
const RANGE: usize = 256 + size_of::<(ClientId, u64)>();
fn sum(values: impl IntoIterator<Item = Option<usize>>) -> Option<usize> {
    values
        .into_iter()
        .try_fold(0usize, |total, n| total.checked_add(n?))
}
fn entry<K, V>() -> usize {
    256 + size_of::<(K, V)>()
}
fn vec_storage<T>(capacity: usize) -> Option<usize> {
    capacity.checked_mul(size_of::<T>().max(256))
}
fn xattrs(map: &XattrMap, cloned: bool) -> Option<usize> {
    sum(map.iter().map(|(key, value)| {
        entry::<Vec<u8>, Vec<u8>>().checked_add(if cloned {
            key.len().checked_add(value.len())?
        } else {
            key.capacity().checked_add(value.capacity())?
        })
    }))
}
fn core_bytes(c: &InodeCore, cloned: bool) -> Option<usize> {
    let buffer = |v: &Vec<u8>| if cloned { v.len() } else { v.capacity() };
    sum([
        Some(if cloned {
            c.layout.pool_ns.len()
        } else {
            c.layout.pool_ns.capacity()
        }),
        c.old_pools.len().checked_mul(entry::<i64, ()>()),
        vec_storage::<FileLayout>(if cloned {
            c.old_layouts.len()
        } else {
            c.old_layouts.capacity()
        }),
        sum(c.old_layouts.iter().map(|l| {
            Some(if cloned {
                l.pool_ns.len()
            } else {
                l.pool_ns.capacity()
            })
        })),
        xattrs(&c.xattrs, cloned),
        Some(buffer(&c.symlink)),
        c.client_ranges.len().checked_mul(RANGE),
        Some(buffer(&c.fscrypt_auth)),
        Some(buffer(&c.fscrypt_file)),
        Some(c.stray_auth_path.as_ref().map_or(0, buffer)),
    ])
}
fn topology(i: &Inode) -> Option<usize> {
    sum([
        sum(i
            .entries
            .keys()
            .map(|k| entry::<DentryKey, Dentry>().checked_add(k.name.capacity()))),
        Some(i.parent.as_ref().map_or(0, |(_, name)| name.capacity())),
        sum(i
            .remote_links
            .iter()
            .map(|(_, n)| entry::<(Ino, Vec<u8>), ()>().checked_add(n.capacity()))),
    ])
}
fn layout_copies(layouts: &[FileLayout]) -> Option<usize> {
    sum([
        vec_storage::<FileLayout>(layouts.len()),
        sum(layouts.iter().map(|l| Some(l.pool_ns.len()))),
    ])
}
fn parser_bytes(peak: super::vxattr::ParsePeak) -> Option<usize> {
    match peak {
        super::vxattr::ParsePeak::Bytes(n) => Some(n),
        super::vxattr::ParsePeak::Capacity => Some(TRANSIENT_LIMIT + 1),
        super::vxattr::ParsePeak::Overflow => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Charge {
    pub completions: Option<usize>,
    pub bytes: Option<usize>,
    pub transient: Option<usize>,
    active_bytes: Option<usize>,
    active_slots: Option<usize>,
}
impl Charge {
    fn with_phase_peak(mut self, peak: Option<usize>) -> Self {
        self.transient = sum([self.transient, peak]);
        self.active_bytes = sum([self.active_bytes, peak]);
        self
    }
    pub fn reservation(self, current: Self) -> (usize, usize) {
        (
            self.completions
                .unwrap_or(usize::MAX)
                .saturating_sub(current.completions.unwrap_or(0)),
            self.bytes
                .unwrap_or(usize::MAX)
                .saturating_sub(current.bytes.unwrap_or(0)),
        )
    }
    fn reserve(&mut self, slots: usize, bytes: usize) {
        self.completions = self.completions.and_then(|n| n.checked_add(slots));
        self.bytes = self.bytes.and_then(|n| n.checked_add(bytes));
        self.active_slots = self.active_slots.and_then(|n| n.checked_add(slots));
        self.active_bytes = self.active_bytes.and_then(|n| n.checked_add(bytes));
    }
    pub fn fits(self, limits: RetryLimits) -> bool {
        limits.max_completions > 0
            && limits.max_bytes > 0
            && self
                .completions
                .zip(Some(limits.max_completions))
                .is_some_and(|(n, max)| n <= max)
            && self.bytes.is_some_and(|n| n <= limits.max_bytes)
            && self.transient.is_some_and(|n| n <= TRANSIENT_LIMIT)
            && self
                .active_bytes
                .zip(limits.max_bytes.checked_add(TRANSIENT_LIMIT))
                .is_some_and(|(n, max)| n <= max)
            && self
                .active_slots
                .zip(limits.max_completions.checked_add(1))
                .is_some_and(|(n, max)| n <= max)
    }
}

#[derive(Clone, Copy)]
struct Metadata {
    live: Option<usize>,
    image: Option<usize>,
    topology: Option<usize>,
    changed: bool,
    variable_changed: bool,
}
impl Metadata {
    fn replace(
        &mut self,
        old_live: Option<usize>,
        old_image: Option<usize>,
        new_live: Option<usize>,
        new_image: Option<usize>,
    ) {
        self.live = self
            .live
            .and_then(|n| n.checked_sub(old_live?))
            .and_then(|n| n.checked_add(new_live?));
        self.image = self
            .image
            .and_then(|n| n.checked_sub(old_image?))
            .and_then(|n| n.checked_add(new_image?));
        self.changed = true;
        self.variable_changed = true;
    }
    fn topology_delta(&mut self, old: Option<usize>, new: Option<usize>) {
        self.topology = self
            .topology
            .and_then(|n| n.checked_sub(old?))
            .and_then(|n| n.checked_add(new?));
    }
}

struct Projection<'a, J: Journal> {
    engine: &'a Engine<J>,
    metadata: BTreeMap<Ino, Metadata>,
    completion: Option<(SessionKey, u64, Option<Ino>)>,
    closed: Option<SessionKey>,
    cap_changes: BTreeMap<(Ino, ClientId), (u32, u32)>,
    context: bool,
    context_owned: bool,
    context_target: Option<Ino>,
    insert_completion: bool,
    extra_transient: Option<usize>,
    preflight_peak: Option<usize>,
}
impl<'a, J: Journal> Projection<'a, J> {
    fn new(engine: &'a Engine<J>) -> Self {
        Self {
            engine,
            metadata: BTreeMap::new(),
            completion: None,
            closed: None,
            cap_changes: BTreeMap::new(),
            context: false,
            context_owned: false,
            context_target: None,
            insert_completion: false,
            extra_transient: Some(0),
            preflight_peak: Some(0),
        }
    }
    fn account_preflight_peak(&mut self, peak: Option<usize>) -> bool {
        let scratch = sum([
            self.metadata.len().checked_mul(entry::<Ino, Metadata>()),
            self.cap_changes
                .len()
                .checked_mul(entry::<(Ino, ClientId), (u32, u32)>()),
        ]);
        let peak = sum([peak, scratch]);
        self.preflight_peak = self.preflight_peak.zip(peak).map(|(a, b)| a.max(b));
        // This check precedes the temporary allocation. Final projection also
        // includes this phase, not only the final metadata/image sizes.
        self.engine
            .retry_charge()
            .with_phase_peak(peak)
            .fits(self.engine.config.retry_limits)
    }
    fn image(&mut self, ino: Ino) -> Option<&mut Metadata> {
        let i = self.engine.inodes.get(&ino)?;
        let charge = self.metadata.entry(ino).or_insert(Metadata {
            live: core_bytes(&i.core, false),
            image: core_bytes(&i.core, true),
            topology: topology(i),
            changed: true,
            variable_changed: false,
        });
        charge.changed = true;
        Some(charge)
    }
    fn ancestors(&mut self, mut ino: Ino) {
        loop {
            self.image(ino);
            match self.engine.inodes.get(&ino).and_then(|i| i.parent.as_ref()) {
                Some((parent, _)) => ino = *parent,
                None => break,
            }
        }
    }
    fn range(&mut self, ino: Ino, client: ClientId, present: bool) {
        let old = self.engine.inodes[&ino]
            .core
            .client_ranges
            .contains_key(&client);
        if let Some(m) = self.image(ino) {
            m.replace(
                Some(if old { RANGE } else { 0 }),
                Some(if old { RANGE } else { 0 }),
                Some(if present { RANGE } else { 0 }),
                Some(if present { RANGE } else { 0 }),
            );
        }
    }
    fn recompute_ranges(&mut self, ino: Ino, new_writer: Option<ClientId>, size: u64) -> bool {
        let i = &self.engine.inodes[&ino];
        let writer = |client: ClientId, cap: &crate::caps::Capability| {
            let (issued, wanted) = self
                .cap_changes
                .get(&(ino, client))
                .copied()
                .unwrap_or((cap.issued(), cap.wanted()));
            Some(client) == new_writer || (issued | wanted) & CEPH_CAP_ANY_FILE_WR != 0
        };
        let count = i
            .caps
            .iter()
            .filter(|(client, cap)| writer(**client, cap))
            .count()
            + usize::from(new_writer.is_some_and(|c| !i.caps.contains_key(&c)));
        let bound = if i.core.layout.is_set() {
            calc_new_max_size(
                &i.core.layout,
                size,
                self.engine.config.writeable_range_max_inc_objs,
            )
        } else {
            0
        };
        let changed = count != i.core.client_ranges.len()
            || i.caps.iter().any(|(client, cap)| {
                writer(*client, cap) && i.core.client_ranges.get(client).is_none_or(|b| *b < bound)
            })
            || new_writer.is_some_and(|c| !i.caps.contains_key(&c));
        if !changed {
            return false;
        }
        let old = i.core.client_ranges.len().checked_mul(RANGE);
        let new = count.checked_mul(RANGE);
        if let Some(m) = self.image(ino) {
            m.replace(old, old, new, new);
        }
        true
    }
    fn dentry(&mut self, dir: Ino, name: &[u8], insert: bool, previously_removed: bool) {
        self.extra_transient = sum([
            self.extra_transient,
            Some(256usize).and_then(|n| n.checked_add(name.len())),
        ]);
        let old = if previously_removed {
            Some(0)
        } else {
            self.engine.inodes[&dir]
                .entries
                .get_key_value(&DentryKey::new(
                    self.engine.inodes[&dir].core.dir_hash,
                    name,
                ))
                .map_or(Some(0), |(k, _)| {
                    entry::<DentryKey, Dentry>().checked_add(k.name.capacity())
                })
        };
        let new = if insert {
            entry::<DentryKey, Dentry>().checked_add(name.len())
        } else {
            Some(0)
        };
        if let Some(m) = self.image(dir) {
            m.topology_delta(old, new);
        }
        self.ancestors(dir);
    }
    fn remove_link(&mut self, dir: Ino, name: &[u8]) {
        let Some(dn) = self.engine.inodes[&dir].lookup(name) else {
            return;
        };
        let i = &self.engine.inodes[&dn.ino];
        let old_parent = i.parent.as_ref().map_or(0, |(_, n)| n.capacity());
        let promotion = if dn.primary && i.core.nlink > 1 {
            i.remote_links.first()
        } else {
            None
        };
        let new_parent = if dn.primary {
            promotion.map_or(0, |(_, n)| n.len())
        } else {
            old_parent
        };
        let remote_removed = if dn.primary {
            promotion.map(|(_, n)| n)
        } else {
            i.remote_links
                .iter()
                .find(|(p, n)| *p == dir && n.as_slice() == name)
                .map(|(_, n)| n)
        };
        let remote_charge = remote_removed.map_or(Some(0), |n| {
            entry::<(Ino, Vec<u8>), ()>().checked_add(n.capacity())
        });
        let path = if i.core.nlink == 1 && i.core.stray_auth_path.is_none() {
            self.engine.authorization_path_bytes(dn.ino)
        } else {
            None
        };
        if let Some(m) = self.image(dn.ino) {
            if let Some(len) = path {
                m.replace(Some(0), Some(0), Some(len), Some(len));
            }
            let removed = if dn.primary && promotion.is_none() {
                Some(0)
            } else {
                remote_charge
            };
            m.topology_delta(
                Some(old_parent).and_then(|n| n.checked_add(removed?)),
                Some(new_parent),
            );
        }
        self.dentry(dir, name, false, false);
        self.ancestors(dn.ino);
        if let Some((dir, name)) = promotion {
            self.extra_transient = sum([
                self.extra_transient,
                Some(256usize).and_then(|n| n.checked_add(name.len())),
            ]);
            self.ancestors(*dir);
        }
    }
    fn request(&mut self, key: SessionKey, req: &Request, captured: Option<&CompletedRequest>) {
        self.context = captured.is_some();
        self.context_owned = self.context && self.engine.owns_completed_context(key, req.tid);
        self.context_target = captured.and_then(|c| c.reply_target_ino);
        if let Some(done) = captured {
            if !req.is_replay()
                && let Some(ino) = done.reply_target_ino
            {
                let path = match &req.op {
                    Op::Create { path, .. } | Op::Open { path, .. } => Some(path),
                    _ => None,
                };
                let flags = match &req.op {
                    Op::Create { flags, .. } | Op::Open { flags, .. } => *flags,
                    _ => 0,
                };
                if path.is_some_and(|p| self.engine.resolve(p) == Ok(ino))
                    && flags & CEPH_O_ACCMODE != CEPH_O_RDONLY
                    && self.engine.inodes[&ino].core.is_file()
                {
                    self.recompute_ranges(
                        ino,
                        Some(key.client),
                        self.engine.inodes[&ino].core.size,
                    );
                }
            }
            return;
        }
        // Embedded releases change range policy before the request executes.
        // Caps are outside retry metadata accounting; only these scalar results
        // enter the projection. Do not mutate the live capability.
        let release_peak = req.releases.iter().try_fold(0usize, |n, r| {
            let cap = self
                .engine
                .inodes
                .get(&r.ino)
                .and_then(|i| i.caps.get(&key.client));
            let Some(cap) = cap.filter(|c| c.cap_id == r.cap_id && r.mseq >= c.mseq) else {
                return Some(n);
            };
            let count = cap.revoke_history().count();
            let capacity = if count == 0 {
                0
            } else {
                count.checked_next_power_of_two()?
            };
            n.checked_add(entry::<Ino, crate::caps::Capability>())?
                .checked_add(vec_storage::<crate::caps::RevokeInfo>(capacity)?)?
                .checked_add(entry::<(Ino, ClientId), (u32, u32)>())
        });
        if !self.account_preflight_peak(release_peak) {
            return;
        }
        let mut released = BTreeMap::new();
        for r in &req.releases {
            if let Some(cap) = self
                .engine
                .inodes
                .get(&r.ino)
                .and_then(|i| i.caps.get(&key.client))
                && r.cap_id == cap.cap_id
                && r.mseq >= cap.mseq
            {
                let cap = released.entry(r.ino).or_insert_with(|| cap.clone());
                cap.confirm_receipt(r.seq, r.caps & cap.issued());
                cap.set_wanted(if r.issue_seq == cap.last_issue {
                    r.wanted
                } else {
                    cap.wanted() | r.wanted
                });
            }
        }
        for (ino, cap) in released {
            self.cap_changes
                .insert((ino, key.client), (cap.issued(), cap.wanted()));
        }
        // Wire WRITE includes volatile file-lock operations. Only durable
        // namespace/attribute mutations reserve a completion here. OPEN
        // decides from its actual truncate/range changes below.
        let mut mutation = matches!(
            &req.op,
            Op::Create { .. }
                | Op::Mknod { .. }
                | Op::Mkdir { .. }
                | Op::Symlink { .. }
                | Op::Setattr { .. }
                | Op::Setxattr { .. }
                | Op::Rmxattr { .. }
                | Op::Unlink { .. }
                | Op::Rmdir { .. }
                | Op::Link { .. }
                | Op::Rename { .. }
        );
        let mut target = None;
        match &req.op {
            Op::Create {
                path,
                mode,
                layout: _,
                xattrs,
                ..
            }
            | Op::Mknod {
                path, mode, xattrs, ..
            }
            | Op::Mkdir {
                path, mode, xattrs, ..
            } => {
                let flags = if let Op::Create { flags, .. } = &req.op {
                    *flags
                } else {
                    0
                };
                if let Ok(ino) = self.engine.resolve(path) {
                    if matches!(req.op, Op::Create { .. }) && flags & CEPH_O_EXCL == 0 {
                        mutation = self.open(ino, key.client, flags, 0);
                        if mutation {
                            target = Some(ino);
                        }
                    } else {
                        mutation = false;
                    }
                } else if let Ok((dir, name)) = self.engine.resolve_parent(path) {
                    let directory = matches!(req.op, Op::Mkdir { .. })
                        || (matches!(req.op, Op::Mknod { .. }) && is_dir(*mode));
                    let mode = if directory {
                        S_IFDIR
                    } else if matches!(req.op, Op::Create { .. }) || mode & S_IFMT == 0 {
                        S_IFREG
                    } else {
                        *mode
                    };
                    let inherited = if mode & S_IFMT == S_IFREG {
                        self.engine.inherited_layout_ref(dir)
                    } else {
                        None
                    };
                    let namespace = inherited.map_or(0, |l| l.pool_ns.len());
                    let pool = match &req.op {
                        Op::Create { layout: lo, .. } if lo.pool >= 0 => lo.pool,
                        _ => inherited.map_or(self.engine.config.default_pool(), |l| l.pool_id),
                    };
                    let old_pool = matches!(req.op, Op::Create { .. })
                        && pool != self.engine.config.default_pool();
                    target = self.new_inode(
                        dir,
                        &name,
                        xattrs,
                        None,
                        namespace,
                        old_pool,
                        matches!(req.op, Op::Create { .. })
                            && flags & CEPH_O_ACCMODE != CEPH_O_RDONLY,
                    );
                } else {
                    mutation = false;
                }
            }
            Op::Symlink {
                path,
                target: bytes,
                xattrs,
                ..
            } => {
                if self.engine.resolve(path).is_ok() {
                    mutation = false;
                } else if let Ok((dir, name)) = self.engine.resolve_parent(path) {
                    target = self.new_inode(dir, &name, xattrs, Some(bytes), 0, false, false);
                } else {
                    mutation = false;
                }
            }
            Op::Open {
                path,
                flags,
                old_size,
                ..
            } => {
                mutation = self.engine.resolve(path).ok().is_some_and(|ino| {
                    let changed = self.open(ino, key.client, *flags, *old_size);
                    if changed {
                        target = Some(ino);
                    }
                    changed
                });
            }
            Op::Setattr { path, args } => {
                if let Ok(ino) = self.engine.resolve(path) {
                    if args.mask & CEPH_SETATTR_SIZE != 0 && self.engine.inodes[&ino].core.is_file()
                    {
                        self.recompute_ranges(ino, None, args.size);
                    }
                    if args.mask & CEPH_SETATTR_SIZE != 0
                        && args.size < self.engine.inodes[&ino].core.size.max(args.old_size)
                    {
                        self.truncate_copy(ino);
                    }
                    self.ancestors(ino);
                } else {
                    mutation = false;
                }
            }
            Op::Setxattr {
                path,
                name,
                value,
                flags,
            } => {
                if let Ok(ino) = self.engine.resolve(path) {
                    let core = &self.engine.inodes[&ino].core;
                    if let Ok(name_str) = std::str::from_utf8(name)
                        && super::vxattr::is_ceph_vxattr(name_str)
                    {
                        if let Ok(value) = std::str::from_utf8(value) {
                            self.layout(ino, name_str, value, false);
                        } else if self.account_preflight_peak(value.len().checked_mul(6)) {
                            self.layout(ino, name_str, &String::from_utf8_lossy(value), false);
                        }
                    } else {
                        let old = core.xattrs.get_key_value(name);
                        let remove = flags & (CEPH_XATTR_REMOVE | CEPH_XATTR_REMOVE2) != 0;
                        let old_live = old.map_or(Some(0), |(k, v)| {
                            entry::<Vec<u8>, Vec<u8>>()
                                .checked_add(k.capacity())?
                                .checked_add(v.capacity())
                        });
                        let old_image = old.map_or(Some(0), |(k, v)| {
                            entry::<Vec<u8>, Vec<u8>>()
                                .checked_add(k.len())?
                                .checked_add(v.len())
                        });
                        let new_live = if remove {
                            Some(0)
                        } else {
                            entry::<Vec<u8>, Vec<u8>>()
                                .checked_add(old.map_or(name.len(), |(k, _)| k.capacity()))
                                .and_then(|n| n.checked_add(value.len()))
                        };
                        let new_image = if remove {
                            Some(0)
                        } else {
                            entry::<Vec<u8>, Vec<u8>>()
                                .checked_add(name.len())
                                .and_then(|n| n.checked_add(value.len()))
                        };
                        if let Some(m) = self.image(ino) {
                            m.replace(old_live, old_image, new_live, new_image);
                        }
                    }
                    self.ancestors(ino);
                } else {
                    mutation = false;
                }
            }
            Op::Rmxattr { path, name } => {
                if let Ok(ino) = self.engine.resolve(path) {
                    if let Ok(name) = std::str::from_utf8(name)
                        && super::vxattr::is_ceph_vxattr(name)
                    {
                        self.layout(ino, name, "", true);
                    }
                    let old = self.engine.inodes[&ino].core.xattrs.get_key_value(name);
                    if let Some((k, v)) = old {
                        let a = entry::<Vec<u8>, Vec<u8>>()
                            .checked_add(k.capacity())
                            .and_then(|n| n.checked_add(v.capacity()));
                        let b = entry::<Vec<u8>, Vec<u8>>()
                            .checked_add(k.len())
                            .and_then(|n| n.checked_add(v.len()));
                        if let Some(m) = self.image(ino) {
                            m.replace(a, b, Some(0), Some(0));
                        }
                    }
                    self.ancestors(ino);
                } else {
                    mutation = false;
                }
            }
            Op::Unlink { path } | Op::Rmdir { path } => {
                if let Ok((dir, name)) = self.engine.resolve_parent(path) {
                    self.remove_link(dir, &name);
                } else {
                    mutation = false;
                }
            }
            Op::Link { path, target } => {
                if let (Ok((dir, name)), Ok(ino)) = (
                    self.engine.resolve_parent(path),
                    self.engine.resolve(target),
                ) {
                    self.dentry(dir, &name, true, false);
                    if let Some(m) = self.image(ino) {
                        m.topology_delta(
                            Some(0),
                            entry::<(Ino, Vec<u8>), ()>().checked_add(name.len()),
                        );
                    }
                    self.ancestors(ino);
                } else {
                    mutation = false;
                }
            }
            Op::Rename { src, dst } => {
                if let (Ok((sdir, sname)), Ok((ddir, dname)), Ok(ino)) = (
                    self.engine.resolve_parent(src),
                    self.engine.resolve_parent(dst),
                    self.engine.resolve(src),
                ) {
                    if self.engine.resolve(dst) == Ok(ino) {
                        mutation = false;
                    } else {
                        let overwritten = self.engine.resolve(dst).is_ok();
                        if overwritten {
                            self.remove_link(ddir, &dname);
                        }
                        self.dentry(sdir, &sname, false, false);
                        self.dentry(ddir, &dname, true, overwritten);
                        let primary = self.engine.inodes[&sdir]
                            .lookup(&sname)
                            .is_some_and(|d| d.primary);
                        let old = if primary {
                            Some(
                                self.engine.inodes[&ino]
                                    .parent
                                    .as_ref()
                                    .map_or(0, |(_, n)| n.capacity()),
                            )
                        } else {
                            self.engine.inodes[&ino]
                                .remote_links
                                .iter()
                                .find(|(p, n)| *p == sdir && *n == sname)
                                .and_then(|(_, n)| {
                                    entry::<(Ino, Vec<u8>), ()>().checked_add(n.capacity())
                                })
                        };
                        let new = if primary {
                            Some(dname.len())
                        } else {
                            entry::<(Ino, Vec<u8>), ()>().checked_add(dname.len())
                        };
                        if let Some(m) = self.image(ino) {
                            m.topology_delta(old, new);
                        }
                        self.ancestors(ino);
                    }
                } else {
                    mutation = false;
                }
            }
            _ => {}
        }
        if mutation {
            self.completion = Some((key, req.oldest_client_tid, target));
            self.insert_completion = self
                .engine
                .sessions
                .get(&key.client)
                .is_none_or(|s| req.tid >= s.durable_oldest_client_tid);
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn new_inode(
        &mut self,
        dir: Ino,
        name: &[u8],
        attrs: &XattrMap,
        symlink: Option<&Vec<u8>>,
        namespace: usize,
        old_pool: bool,
        writer: bool,
    ) -> Option<Ino> {
        let ino = self.engine.ino_alloc.peek()?;
        let var = sum([
            xattrs(attrs, true),
            Some(namespace),
            Some(symlink.map_or(0, Vec::len)),
            Some(if writer { RANGE } else { 0 }),
            Some(if old_pool { entry::<i64, ()>() } else { 0 }),
        ]);
        self.metadata.insert(
            ino,
            Metadata {
                live: var,
                image: var,
                topology: Some(name.len()),
                changed: true,
                variable_changed: true,
            },
        );
        self.dentry(dir, name, true, false);
        Some(ino)
    }
    fn open(&mut self, ino: Ino, client: ClientId, flags: u32, old_size: u64) -> bool {
        let core = &self.engine.inodes[&ino].core;
        let truncate = flags & CEPH_O_TRUNC != 0 && core.is_file();
        let writer = flags & CEPH_O_ACCMODE != CEPH_O_RDONLY && core.is_file();
        let size = if truncate { 0 } else { core.size };
        let range = writer && self.recompute_ranges(ino, Some(client), size);
        if truncate && self.engine.inodes[&ino].core.size.max(old_size) > 0 {
            self.truncate_copy(ino);
        }
        if truncate {
            self.ancestors(ino);
        }
        truncate || range
    }
    fn truncate_copy(&mut self, ino: Ino) {
        let copy = Some(256usize)
            .and_then(|n| n.checked_add(self.engine.inodes[&ino].core.layout.pool_ns.len()));
        self.extra_transient = sum([self.extra_transient, copy.and_then(|n| n.checked_mul(3))]);
    }
    fn recovery_layout_copies(&self, ino: Ino) -> Option<usize> {
        let c = &self.engine.inodes[&ino].core;
        let mut count = 1usize;
        let mut names = c.layout.pool_ns.len();
        for (offset, l) in c.old_layouts.iter().enumerate() {
            if *l != c.layout && !c.old_layouts[..offset].contains(l) {
                count = count.checked_add(1)?;
                names = names.checked_add(l.pool_ns.len())?;
            }
        }
        for pool in &c.old_pools {
            if *pool != c.layout.pool_id && !c.old_layouts.iter().any(|l| l.pool_id == *pool) {
                count = count.checked_add(1)?;
                names = names.checked_add(c.layout.pool_ns.len())?;
            }
        }
        sum([vec_storage::<FileLayout>(count), Some(names)]).and_then(|n| n.checked_mul(3))
    }
    fn layout(&mut self, ino: Ino, name: &str, value: &str, rm: bool) {
        let c = &self.engine.inodes[&ino].core;
        let file = name.starts_with("ceph.file.layout");
        let namespace = if file || name.starts_with("ceph.dir.layout") {
            if c.layout.is_set() {
                c.layout.pool_ns.len()
            } else {
                self.engine
                    .inherited_layout_ref(ino)
                    .map_or(0, |l| l.pool_ns.len())
            }
        } else {
            0
        };
        if !self.account_preflight_peak(parser_bytes(super::vxattr::virtual_attr_parse_peak(
            name, value, namespace,
        ))) {
            return;
        }
        if file || name.starts_with("ceph.dir.layout") {
            let prefix = if file {
                "ceph.file.layout"
            } else {
                "ceph.dir.layout"
            };
            let Some(field) = super::ops_xattr::layout_subfield(&name[prefix.len()..]) else {
                return;
            };
            let mut next = if c.layout.is_set() {
                c.layout.clone()
            } else {
                self.engine.inherited_layout(ino).0
            };
            if rm && field.is_empty() && !file {
                next = if ino == CEPH_INO_ROOT {
                    self.engine.config.default_layout()
                } else {
                    FileLayout::none()
                };
            } else if super::vxattr::apply_layout_field(
                &mut next,
                field,
                value,
                &super::ops_xattr::EnginePools {
                    names: &self.engine.config.pool_names,
                },
            )
            .is_err()
                || !next.is_valid_geometry()
                || !self.engine.config.is_data_pool(next.pool_id)
            {
                return;
            }
            let next_ns = next.pool_ns.capacity();
            if let Some(m) = self.image(ino) {
                m.replace(
                    Some(c.layout.pool_ns.capacity()),
                    Some(c.layout.pool_ns.len()),
                    Some(next_ns),
                    Some(next_ns),
                );
                if file && c.layout != next && !c.old_layouts.contains(&c.layout) {
                    let old_live = vec_storage::<FileLayout>(c.old_layouts.capacity());
                    let old_image = vec_storage::<FileLayout>(c.old_layouts.len());
                    let len = c.old_layouts.len().checked_add(1);
                    let live = len.and_then(|len| {
                        vec_storage::<FileLayout>(len.max(c.old_layouts.capacity()))
                    });
                    let image = len.and_then(vec_storage::<FileLayout>);
                    m.replace(old_live, old_image, live, image);
                    m.replace(
                        Some(0),
                        Some(0),
                        Some(c.layout.pool_ns.len()),
                        Some(c.layout.pool_ns.len()),
                    );
                }
                if file && !c.old_pools.contains(&c.layout.pool_id) {
                    m.replace(
                        Some(0),
                        Some(0),
                        Some(entry::<i64, ()>()),
                        Some(entry::<i64, ()>()),
                    );
                }
            }
        }
    }
    fn cleanup_range(&mut self, ino: Ino, client: ClientId, kill: bool) {
        if self
            .engine
            .jobs
            .values()
            .any(|j| j.ino == ino && matches!(j.kind, LifecycleKind::RecoverSize { .. }))
        {
            return;
        }
        if self.engine.inodes[&ino]
            .core
            .client_ranges
            .contains_key(&client)
        {
            if !kill {
                self.range(ino, client, false);
            } else {
                self.image(ino);
                self.extra_transient =
                    sum([self.extra_transient, self.recovery_layout_copies(ino)]);
            }
        }
    }
    fn control(&mut self, input: &EngineInput) {
        match input {
            EngineInput::CapUpdate { key, update: m } => {
                let Some(i) = self.engine.inodes.get(&m.ino) else {
                    return;
                };
                let Some(cap) = i.caps.get(&key.client) else {
                    return;
                };
                if cap.cap_id != m.cap_id || m.mseq < cap.mseq || m.op == CEPH_CAP_OP_FLUSHSNAP {
                    return;
                }
                let duplicate = m.flush_tid != 0
                    && self.engine.sessions[&key.client]
                        .completed_flushes
                        .contains(&m.flush_tid);
                let dirty = if duplicate { 0 } else { m.dirty };
                if dirty == 0
                    && self.engine.jobs.values().any(|j| {
                        j.ino == m.ino && matches!(j.kind, LifecycleKind::RecoverSize { .. })
                    })
                {
                    return;
                }
                if dirty & CEPH_CAP_XATTR_EXCL != 0
                    && let Some((version, attrs)) = &m.xattrs
                    && *version > i.core.xattr_version
                {
                    let bytes = attrs.iter().try_fold(0usize, |n, (k, v)| {
                        n.checked_add(k.len())?.checked_add(v.len())
                    });
                    if bytes.is_some_and(|b| b as u64 <= self.engine.config.max_xattr_size)
                        && let Some(c) = self.image(m.ino)
                    {
                        c.replace(
                            xattrs(&i.core.xattrs, false),
                            xattrs(&i.core.xattrs, true),
                            xattrs(attrs, true),
                            xattrs(attrs, true),
                        );
                    }
                }
                let old = i.core.client_ranges.get(&key.client).copied().unwrap_or(0);
                let issued = cap.issued_after_receipt(m.seq, m.caps & cap.issued());
                let wanted = if m.issue_seq == cap.last_issue {
                    m.wanted
                } else {
                    cap.wanted() | m.wanted
                };
                let writer = (issued | wanted) & CEPH_CAP_ANY_FILE_WR != 0;
                let nm = if writer {
                    calc_new_max_size(
                        &i.core.layout,
                        if m.max_size > old { m.max_size } else { m.size },
                        self.engine.config.writeable_range_max_inc_objs,
                    )
                    .max(old)
                } else {
                    0
                };
                if i.core.is_file() && nm != old {
                    self.range(m.ino, key.client, nm != 0);
                }
                if dirty != 0 || nm != old {
                    self.ancestors(m.ino);
                }
                if dirty != 0 {
                    for job in self.engine.jobs.values().filter(|j| j.ino == m.ino) {
                        if let LifecycleKind::RecoverSize { layouts, .. } = &job.kind {
                            self.extra_transient = sum([
                                self.extra_transient,
                                layout_copies(layouts).and_then(|n| n.checked_mul(3)),
                            ]);
                        }
                    }
                }
            }
            EngineInput::CapRelease { key, items } => {
                if let Some(item) = items.first().or_else(|| self.engine.release_cursor.front())
                    && let Some(cap) = self
                        .engine
                        .inodes
                        .get(&item.ino)
                        .and_then(|i| i.caps.get(&key.client))
                    && item.cap_id == cap.cap_id
                    && item.mseq >= cap.mseq
                    && item.issue_seq >= cap.last_issue
                {
                    self.cleanup_range(item.ino, key.client, false);
                }
            }
            EngineInput::CloseSession { key, kill } => {
                if let Some(ino) = self.engine.sessions[&key.client].caps.first() {
                    self.cleanup_range(*ino, key.client, *kill);
                } else if self.engine.sessions[&key.client].lock_inos.is_empty() {
                    if let Some((ino, _)) = self.engine.inodes.iter().find(|(ino, i)| {
                        i.core.client_ranges.contains_key(&key.client)
                            && !self.engine.writer_range_is_covered(**ino, key.client)
                    }) {
                        self.cleanup_range(*ino, key.client, true);
                    } else {
                        self.closed = Some(*key);
                    }
                }
            }
            EngineInput::LifecycleCompleted { id, result } => {
                if let Some(job) = self
                    .engine
                    .jobs
                    .get(id)
                    .filter(|j| self.engine.job_is_eligible(j))
                    && let (
                        LifecycleKind::RecoverSize {
                            inode_version,
                            truncate_seq,
                            max_size,
                            ..
                        },
                        LifecycleResult::Recovered { size, .. },
                    ) = (&job.kind, result)
                    && let Some(i) = self.engine.inodes.get(&job.ino)
                    && *inode_version == i.core.version
                    && *truncate_seq == i.core.truncate_seq
                    && *size <= *max_size
                {
                    let ranges = i
                        .core
                        .client_ranges
                        .keys()
                        .filter(|c| i.caps.contains_key(c))
                        .count()
                        .checked_mul(RANGE);
                    let old = i.core.client_ranges.len().checked_mul(RANGE);
                    if let Some(m) = self.image(job.ino) {
                        m.replace(old, old, ranges, ranges);
                    }
                    self.ancestors(job.ino);
                }
            }
            _ => {}
        }
    }
    fn completion_survives(&self, session: &Session, tid: u64) -> bool {
        if self
            .closed
            .is_some_and(|k| k.client == session.client && k.incarnation == session.epoch)
        {
            return false;
        }
        self.completion.is_none_or(|(key, floor, _)| {
            key.client != session.client
                || key.incarnation != session.epoch
                || tid >= floor.max(session.durable_oldest_client_tid)
        })
    }
    fn target_survives(&self, ino: Ino, retired: &BTreeMap<Ino, usize>) -> bool {
        self.insert_completion
            && self
                .completion
                .is_some_and(|(_, _, target)| target == Some(ino))
            || self.engine.retry_refs.get(&ino).copied().unwrap_or(0)
                > retired.get(&ino).copied().unwrap_or(0)
    }
    fn target_bytes(&self, ino: Ino, current_commit: bool) -> Option<usize> {
        let m = self.metadata.get(&ino);
        let live = m.map_or_else(
            || core_bytes(&self.engine.inodes.get(&ino)?.core, false),
            |m| m.live,
        );
        let committed = if current_commit {
            self.engine
                .committed_inodes
                .get(&ino)
                .map_or(Some(0), |c| core_bytes(c, false))
        } else {
            m.map_or_else(
                || core_bytes(self.engine.committed_inodes.get(&ino)?, false),
                |m| {
                    if m.variable_changed {
                        return m.image;
                    }
                    // A statistics ancestor may not need an image when its
                    // values are unchanged. Do not assume that this releases
                    // spare capacity in its existing committed buffers.
                    let old = self
                        .engine
                        .committed_inodes
                        .get(&ino)
                        .map_or(Some(0), |c| core_bytes(c, false));
                    m.image.zip(old).map(|(a, b)| a.max(b))
                },
            )
        };
        let topo = m.map_or_else(|| topology(self.engine.inodes.get(&ino)?), |m| m.topology);
        sum([Some(FIXED_INODE + INDEX), live, committed, topo])
    }
    fn charge(&self) -> Charge {
        // This temporary projection index never owns history. Drop it before
        // mutation. Account for its allocation in the preflight copy phase.
        let mut retired = BTreeMap::<Ino, usize>::new();
        for session in self.engine.sessions.values() {
            for done in session.completed_requests.values() {
                if !self.completion_survives(session, done.tid)
                    && let Some(ino) = done.reply_target_ino
                {
                    *retired.entry(ino).or_default() += 1;
                }
            }
        }
        let total = self
            .engine
            .sessions
            .values()
            .try_fold(0usize, |n, s| n.checked_add(s.completed_requests.len()));
        let count = self
            .engine
            .sessions
            .values()
            .flat_map(|s| s.completed_requests.keys().map(move |tid| (s, *tid)))
            .filter(|(s, t)| self.completion_survives(s, *t))
            .try_fold(0usize, |n, _| n.checked_add(1))
            .and_then(|n| n.checked_add(usize::from(self.insert_completion)));
        let mut bytes = count.and_then(|n| n.checked_mul(COMPLETION));
        let new_slot =
            usize::from(self.completion.is_some() || self.context && !self.context_owned);
        let contexts = self
            .engine
            .owned_completion_contexts()
            .and_then(|n| n.checked_add(new_slot));
        let context_bytes = contexts.and_then(|n| n.checked_mul(COMPLETION));
        let active_slots = total.zip(contexts).and_then(|(a, b)| a.checked_add(b));
        let mut active = active_slots.and_then(|n| n.checked_mul(COMPLETION));
        for ino in self.engine.inodes.keys().filter(|ino| {
            self.engine.retry_refs.contains_key(ino)
                || self.engine.reply_holds_target(**ino)
                || self.context_target == Some(**ino)
                || self
                    .completion
                    .is_some_and(|(_, _, target)| target == Some(**ino))
        }) {
            if self.target_survives(*ino, &retired) {
                bytes = sum([bytes, self.target_bytes(*ino, false)]);
            }
            active = sum([active, self.target_bytes(*ino, true)]);
        }
        if let Some(ino) = self.completion.and_then(|(_, _, i)| i)
            && !self.engine.inodes.contains_key(&ino)
        {
            if self.insert_completion {
                bytes = sum([bytes, self.target_bytes(ino, false)]);
            }
            active = sum([active, self.target_bytes(ino, true)]);
        }
        let transient = self
            .metadata
            .values()
            .filter(|m| m.changed)
            .try_fold(0usize, |n, m| {
                n.checked_add(
                    m.image?
                        .checked_add(size_of::<InodeCore>().max(256))?
                        .checked_mul(2)?,
                )
            });
        let transient = sum([transient, self.extra_transient]);
        let scratch = sum([
            retired.len().checked_mul(INDEX),
            self.metadata.len().checked_mul(entry::<Ino, Metadata>()),
            self.cap_changes
                .len()
                .checked_mul(entry::<(Ino, ClientId), (u32, u32)>()),
        ]);
        // Projection scratch and submitted image copies do not coexist.
        let phase_transient = transient
            .zip(scratch)
            .zip(self.preflight_peak)
            .map(|((a, b), c)| a.max(b).max(c));
        let transient = sum([phase_transient, context_bytes]);
        Charge {
            completions: count,
            bytes,
            transient,
            // Context bytes already enter active through active_slots.
            active_bytes: sum([active, phase_transient]),
            active_slots,
        }
    }
}

impl<J: Journal> Engine<J> {
    pub(super) fn virtual_attr_parser_fits(&self, ino: Ino, name: &str, value: &str) -> bool {
        let core = &self.inodes[&ino].core;
        let namespace =
            if name.starts_with("ceph.file.layout") || name.starts_with("ceph.dir.layout") {
                if core.layout.is_set() {
                    core.layout.pool_ns.len()
                } else {
                    self.inherited_layout_ref(ino)
                        .map_or(0, |l| l.pool_ns.len())
                }
            } else {
                0
            };
        let peak = parser_bytes(super::vxattr::virtual_attr_parse_peak(
            name, value, namespace,
        ));
        self.retry_charge()
            .with_phase_peak(peak)
            .fits(self.config.retry_limits)
    }
    pub(super) fn retry_charge(&self) -> Charge {
        Projection::new(self).charge()
    }
    pub(super) fn projected_retry_charge(
        &self,
        input: &EngineInput,
        captured: Option<&CompletedRequest>,
    ) -> Charge {
        let mut projection = Projection::new(self);
        match input {
            EngineInput::Request { key, request } => projection.request(*key, request, captured),
            _ => projection.control(input),
        }
        projection.charge()
    }
    pub(super) fn projected_request_charge(&self, p: &Pending) -> Charge {
        let mut projection = Projection::new(self);
        projection.request(
            SessionKey {
                client: p.client,
                incarnation: p.epoch,
            },
            &p.req,
            p.completed.as_ref(),
        );
        projection.charge()
    }
    pub(super) fn with_retry_reservations(
        &self,
        mut charge: Charge,
        exclude: Option<(ClientId, u64)>,
    ) -> Charge {
        for p in &self.parked {
            if exclude != Some((p.client, p.req.tid)) {
                charge.reserve(p.retry_reservation.0, p.retry_reservation.1);
            }
        }
        if let Some(EngineInput::Request { key, request }) = &self.continuation
            && exclude != Some((key.client, request.tid))
        {
            charge.reserve(
                self.continuation_reservation.0,
                self.continuation_reservation.1,
            );
        }
        charge
    }
    pub(super) fn valid_retry_limits(&self) -> bool {
        let limits = self.config.retry_limits;
        limits.max_bytes > 0
            && limits.max_completions > 0
            && limits.max_bytes.checked_add(TRANSIENT_LIMIT).is_some()
            && limits.max_completions.checked_add(1).is_some()
    }
}
