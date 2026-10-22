//! The encoding of engine journal records (WAL bodies).
//!
//! Schema 2 includes durable session identities and lifecycle intents. Schema 1
//! lacks these semantics and is rejected. Evolving nested types have independent
//! length-delimited version frames (`design/persistence-contract.md`). The
//! functions are plain (not `denc` trait impls) because the types belong
//! to the engine, which stays free of wire code.

use std::collections::{BTreeMap, BTreeSet};

use bytes::{Buf, BufMut, Bytes, BytesMut};

use denc::{DecodeError, decode_start, encode_finish, encode_start, need};
use engine::durability::{
    CheckpointInode, CheckpointSession, EngineCheckpoint, LifecycleIntent, LifecycleKind,
    SessionKey,
};
use engine::ino::InoAllocState;
use engine::inode::{Dentry, DentryKey, InodeCore, SnapRealmNode};
use engine::journal::{
    ClientFlush, CompletedRequest, DentryLink, DentryUpdate, JournalRecord, RecordKind,
};
use engine::types::{ClientId, DirStat, FileLayout, InoRange, Quota, Rstat, Timestamp, XattrMap};

/// The WAL record encoding version this crate writes.
pub const RECORD_VERSION: u8 = 2;

/// Production encoder. The preflight bound includes fixed-field headroom. It may
/// reject a record near the byte limit, but never allocates an unbounded buffer.
/// `max_bytes` bounds the body; reserve `framing::OVERHEAD` separately.
pub fn encode_record_bounded(
    record: &JournalRecord,
    max_bytes: usize,
) -> Result<Bytes, crate::stable::StoreError> {
    let max_bytes = max_bytes.min(u32::MAX as usize);
    let mut budget = ByteBudget(max_bytes);
    budget.take(512)?;
    budget.count(record.allocated.len(), 8)?;
    budget.count(record.purged.len(), 8)?;
    budget.count(record.closed_sessions.len(), 32)?;
    budget.count(record.completed_jobs.len(), 8)?;
    budget.count(record.inodes.len(), 2048)?;
    for inode in &record.inodes {
        budget.inode_variable(inode)?;
    }
    budget.count(record.dentries.len(), 128)?;
    for dentry in &record.dentries {
        budget.take(dentry.name.len())?;
    }
    if let Some(allocator) = &record.ino_alloc {
        budget.allocator(allocator)?;
    }
    budget.jobs(&record.lifecycle_intents)?;
    let bytes = encode_record(record);
    if bytes.len() > max_bytes {
        return Err(crate::stable::StoreError::Capacity);
    }
    Ok(bytes)
}

pub fn encode_checkpoint_bounded(
    state: &EngineCheckpoint,
    max_bytes: usize,
) -> Result<Bytes, crate::stable::StoreError> {
    let max_bytes = max_bytes.min(u32::MAX as usize);
    let mut budget = ByteBudget(max_bytes);
    budget.take(128)?;
    budget.count(state.inodes.len(), 4096)?;
    for inode in &state.inodes {
        budget.inode_variable(&inode.core)?;
        budget.count(inode.entries.len(), 128)?;
        for (key, _) in &inode.entries {
            budget.take(key.name.len())?;
        }
        if let Some((_, name)) = &inode.parent {
            budget.take(name.len())?;
        }
        budget.count(inode.remote_links.len(), 64)?;
        for (_, name) in &inode.remote_links {
            budget.take(name.len())?;
        }
    }
    budget.allocator(&state.ino_alloc)?;
    budget.count(state.sessions.len(), 128)?;
    for session in &state.sessions {
        budget.count(session.completed_requests.len(), 128)?;
        budget.count(session.completed_flushes.len(), 8)?;
    }
    budget.jobs(&state.jobs)?;
    budget.count(state.cleanup_receipts.len(), 64)?;
    let bytes = encode_checkpoint(state);
    if bytes.len() > max_bytes {
        return Err(crate::stable::StoreError::Capacity);
    }
    Ok(bytes)
}

struct ByteBudget(usize);
impl ByteBudget {
    fn take(&mut self, n: usize) -> Result<(), crate::stable::StoreError> {
        self.0 = self
            .0
            .checked_sub(n)
            .ok_or(crate::stable::StoreError::Capacity)?;
        Ok(())
    }
    fn count(&mut self, count: usize, per_item: usize) -> Result<(), crate::stable::StoreError> {
        self.take(
            count
                .checked_mul(per_item)
                .ok_or(crate::stable::StoreError::Capacity)?,
        )
    }
    fn allocator(&mut self, state: &InoAllocState) -> Result<(), crate::stable::StoreError> {
        self.take(128)?;
        self.count(state.free.len(), 32)?;
        self.count(state.delegated.len(), 64)
    }
    fn layout(&mut self, layout: &FileLayout) -> Result<(), crate::stable::StoreError> {
        self.take(64)?;
        self.take(layout.pool_ns.len())
    }
    fn inode_variable(&mut self, inode: &InodeCore) -> Result<(), crate::stable::StoreError> {
        self.layout(&inode.layout)?;
        self.count(inode.old_layouts.len(), 64)?;
        for layout in &inode.old_layouts {
            self.take(layout.pool_ns.len())?;
        }
        self.count(inode.old_pools.len(), 8)?;
        self.count(inode.client_ranges.len(), 16)?;
        self.count(inode.xattrs.len(), 16)?;
        for (key, value) in &inode.xattrs {
            self.take(key.len())?;
            self.take(value.len())?;
        }
        self.take(inode.symlink.len())?;
        self.take(inode.fscrypt_auth.len())?;
        self.take(inode.fscrypt_file.len())?;
        if let Some(path) = &inode.stray_auth_path {
            self.take(path.len())?;
        }
        Ok(())
    }
    fn jobs(&mut self, jobs: &[LifecycleIntent]) -> Result<(), crate::stable::StoreError> {
        self.count(jobs.len(), 128)?;
        for job in jobs {
            match &job.kind {
                LifecycleKind::Purge { layouts, .. }
                | LifecycleKind::RecoverSize { layouts, .. } => {
                    self.count(layouts.len(), 64)?;
                    for layout in layouts {
                        self.take(layout.pool_ns.len())?;
                    }
                }
                LifecycleKind::Truncate { layout, .. } => self.layout(layout)?,
            }
        }
        Ok(())
    }
}

/// Encode one record to bytes (its WAL body).
pub fn encode_record(record: &JournalRecord) -> Bytes {
    let mut buf = BytesMut::new();
    let s = encode_start(&mut buf, RECORD_VERSION, RECORD_VERSION);
    put_u8(&mut buf, kind_code(record.kind));
    put_opt(
        &mut buf,
        record.completed_request.as_ref(),
        put_completed_request,
    );
    put_opt(&mut buf, record.client_flush.as_ref(), put_client_flush);
    put_vec(&mut buf, &record.allocated, |b, v| b.put_u64_le(*v));
    put_opt(&mut buf, record.ino_alloc.as_ref(), put_ino_alloc);
    put_vec(&mut buf, &record.inodes, put_inode);
    put_vec(&mut buf, &record.dentries, put_dentry_update);
    put_vec(&mut buf, &record.purged, |b, v| b.put_u64_le(*v));
    put_vec(&mut buf, &record.closed_sessions, put_session_key);
    put_vec(&mut buf, &record.lifecycle_intents, put_lifecycle);
    put_vec(&mut buf, &record.completed_jobs, |b, v| b.put_u64_le(*v));
    encode_finish(&mut buf, s);
    buf.freeze()
}

/// Decode one record from its framed body, ignoring fields a newer writer
/// appended inside the frame.
pub fn decode_record(buf: &Bytes) -> Result<JournalRecord, DecodeError> {
    let mut input = buf.clone();
    let mut b = get_frame(&mut input, RECORD_VERSION, "JournalRecord")?;
    if !input.is_empty() {
        return Err(invalid("JournalRecord", "trailing bytes"));
    }
    Ok(JournalRecord {
        kind: kind_from_code(get_u8(&mut b, "RecordKind")?)?,
        completed_request: get_opt(&mut b, get_completed_request)?,
        client_flush: get_opt(&mut b, get_client_flush)?,
        allocated: get_vec(&mut b, get_u64)?,
        ino_alloc: get_opt(&mut b, get_ino_alloc)?,
        inodes: get_vec(&mut b, get_inode)?,
        dentries: get_vec(&mut b, get_dentry_update)?,
        purged: get_vec(&mut b, get_u64)?,
        closed_sessions: get_vec(&mut b, get_session_key)?,
        lifecycle_intents: get_vec(&mut b, get_lifecycle)?,
        completed_jobs: get_vec(&mut b, get_u64)?,
    })
}

// ------------------------------------------------------------------
// Element helpers

fn put_u8(buf: &mut BytesMut, v: u8) {
    buf.put_u8(v);
}

fn get_u8(buf: &mut Bytes, what: &'static str) -> Result<u8, DecodeError> {
    need(buf, 1, what)?;
    Ok(buf.get_u8())
}

fn get_u64(buf: &mut Bytes) -> Result<u64, DecodeError> {
    need(buf, 8, "u64")?;
    Ok(buf.get_u64_le())
}

/// `u32 count` then one element per item.
fn put_vec<T>(buf: &mut BytesMut, v: &[T], put: impl Fn(&mut BytesMut, &T)) {
    buf.put_u32_le(v.len() as u32);
    for item in v {
        put(buf, item);
    }
}

fn get_vec<T>(
    buf: &mut Bytes,
    get: impl Fn(&mut Bytes) -> Result<T, DecodeError>,
) -> Result<Vec<T>, DecodeError> {
    need(buf, 4, "vec")?;
    let n = buf.get_u32_le() as usize;
    if n > buf.len() {
        return Err(invalid("vec", "count exceeds remaining bytes"));
    }
    let mut v = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        v.push(get(buf)?);
    }
    Ok(v)
}

/// `u8 present` then the element.
fn put_opt<T>(buf: &mut BytesMut, v: Option<&T>, put: impl Fn(&mut BytesMut, &T)) {
    match v {
        Some(t) => {
            buf.put_u8(1);
            put(buf, t);
        }
        None => buf.put_u8(0),
    }
}

fn get_opt<T>(
    buf: &mut Bytes,
    get: impl Fn(&mut Bytes) -> Result<T, DecodeError>,
) -> Result<Option<T>, DecodeError> {
    need(buf, 1, "option")?;
    let present = buf.get_u8();
    if present > 1 {
        return Err(invalid("option", "invalid discriminant"));
    }
    Ok(if present != 0 { Some(get(buf)?) } else { None })
}

/// `u32 len` + raw bytes.
fn put_blob(buf: &mut BytesMut, b: &[u8]) {
    buf.put_u32_le(b.len() as u32);
    buf.put_slice(b);
}

fn get_blob(buf: &mut Bytes, what: &'static str) -> Result<Vec<u8>, DecodeError> {
    need(buf, 4, what)?;
    let len = buf.get_u32_le() as usize;
    need(buf, len, what)?;
    Ok(buf.copy_to_bytes(len).to_vec())
}

fn kind_code(kind: RecordKind) -> u8 {
    match kind {
        RecordKind::Mkfs => 0,
        RecordKind::Create => 1,
        RecordKind::Mknod => 2,
        RecordKind::Mkdir => 3,
        RecordKind::Symlink => 4,
        RecordKind::Link => 5,
        RecordKind::Unlink => 6,
        RecordKind::Rmdir => 7,
        RecordKind::Rename => 8,
        RecordKind::Setattr => 9,
        RecordKind::Open => 10,
        RecordKind::Setxattr => 11,
        RecordKind::Rmxattr => 12,
        RecordKind::SetVxattr => 13,
        RecordKind::CapUpdate => 14,
        RecordKind::ClientRanges => 15,
        RecordKind::Purge => 16,
        RecordKind::SessionClose => 17,
        RecordKind::LifecycleComplete => 18,
    }
}

fn kind_from_code(code: u8) -> Result<RecordKind, DecodeError> {
    Ok(match code {
        0 => RecordKind::Mkfs,
        1 => RecordKind::Create,
        2 => RecordKind::Mknod,
        3 => RecordKind::Mkdir,
        4 => RecordKind::Symlink,
        5 => RecordKind::Link,
        6 => RecordKind::Unlink,
        7 => RecordKind::Rmdir,
        8 => RecordKind::Rename,
        9 => RecordKind::Setattr,
        10 => RecordKind::Open,
        11 => RecordKind::Setxattr,
        12 => RecordKind::Rmxattr,
        13 => RecordKind::SetVxattr,
        14 => RecordKind::CapUpdate,
        15 => RecordKind::ClientRanges,
        16 => RecordKind::Purge,
        17 => RecordKind::SessionClose,
        18 => RecordKind::LifecycleComplete,
        other => {
            return Err(DecodeError::Invalid {
                what: "RecordKind",
                detail: format!("unknown kind {other}"),
            });
        }
    })
}

// ------------------------------------------------------------------
// Engine types

fn put_timestamp(buf: &mut BytesMut, t: Timestamp) {
    buf.put_u32_le(t.sec);
    buf.put_u32_le(t.nsec);
}

fn get_timestamp(buf: &mut Bytes) -> Result<Timestamp, DecodeError> {
    need(buf, 8, "Timestamp")?;
    Ok(Timestamp {
        sec: buf.get_u32_le(),
        nsec: buf.get_u32_le(),
    })
}

fn put_layout(buf: &mut BytesMut, l: &FileLayout) {
    let s = encode_start(buf, 1, 1);
    buf.put_u32_le(l.stripe_unit);
    buf.put_u32_le(l.stripe_count);
    buf.put_u32_le(l.object_size);
    buf.put_i64_le(l.pool_id);
    put_blob(buf, l.pool_ns.as_bytes());
    encode_finish(buf, s);
}

fn get_layout(buf: &mut Bytes) -> Result<FileLayout, DecodeError> {
    let mut frame = get_frame(buf, 1, "FileLayout")?;
    let buf = &mut frame;
    need(buf, 20, "FileLayout")?;
    let stripe_unit = buf.get_u32_le();
    let stripe_count = buf.get_u32_le();
    let object_size = buf.get_u32_le();
    let pool_id = buf.get_i64_le();
    let pool_ns = String::from_utf8(get_blob(buf, "FileLayout.pool_ns")?).map_err(|_| {
        DecodeError::Invalid {
            what: "FileLayout.pool_ns",
            detail: "not UTF-8".into(),
        }
    })?;
    Ok(FileLayout {
        stripe_unit,
        stripe_count,
        object_size,
        pool_id,
        pool_ns,
    })
}

fn put_dirstat(buf: &mut BytesMut, d: &DirStat) {
    let start = encode_start(buf, 1, 1);
    put_timestamp(buf, d.mtime);
    buf.put_u64_le(d.change_attr);
    buf.put_i64_le(d.nfiles);
    buf.put_i64_le(d.nsubdirs);
    encode_finish(buf, start);
}

fn get_dirstat(buf: &mut Bytes) -> Result<DirStat, DecodeError> {
    let mut frame = get_frame(buf, 1, "DirStat")?;
    let buf = &mut frame;
    Ok(DirStat {
        mtime: get_timestamp(buf)?,
        change_attr: get_u64(buf)?,
        nfiles: {
            need(buf, 8, "i64")?;
            buf.get_i64_le()
        },
        nsubdirs: {
            need(buf, 8, "i64")?;
            buf.get_i64_le()
        },
    })
}

fn put_rstat(buf: &mut BytesMut, r: &Rstat) {
    let start = encode_start(buf, 1, 1);
    put_timestamp(buf, r.rctime);
    buf.put_i64_le(r.rbytes);
    buf.put_i64_le(r.rfiles);
    buf.put_i64_le(r.rsubdirs);
    buf.put_i64_le(r.rsnaps);
    encode_finish(buf, start);
}

fn get_rstat(buf: &mut Bytes) -> Result<Rstat, DecodeError> {
    let mut frame = get_frame(buf, 1, "Rstat")?;
    let buf = &mut frame;
    let i = |buf: &mut Bytes| {
        need(buf, 8, "i64")?;
        Ok(buf.get_i64_le())
    };
    Ok(Rstat {
        rctime: get_timestamp(buf)?,
        rbytes: i(buf)?,
        rfiles: i(buf)?,
        rsubdirs: i(buf)?,
        rsnaps: i(buf)?,
    })
}

fn put_quota(buf: &mut BytesMut, q: &Quota) {
    let start = encode_start(buf, 1, 1);
    buf.put_i64_le(q.max_bytes);
    buf.put_i64_le(q.max_files);
    encode_finish(buf, start);
}

fn get_quota(buf: &mut Bytes) -> Result<Quota, DecodeError> {
    let mut frame = get_frame(buf, 1, "Quota")?;
    let buf = &mut frame;
    let i = |buf: &mut Bytes| {
        need(buf, 8, "i64")?;
        Ok(buf.get_i64_le())
    };
    Ok(Quota {
        max_bytes: i(buf)?,
        max_files: i(buf)?,
    })
}

fn put_snaprealm(buf: &mut BytesMut, s: &SnapRealmNode) {
    let start = encode_start(buf, 1, 1);
    buf.put_u64_le(s.seq);
    buf.put_u64_le(s.created);
    buf.put_u8(u8::from(s.subvolume));
    put_timestamp(buf, s.last_modified);
    buf.put_u64_le(s.change_attr);
    encode_finish(buf, start);
}

fn get_snaprealm(buf: &mut Bytes) -> Result<SnapRealmNode, DecodeError> {
    let mut frame = get_frame(buf, 1, "SnapRealmNode")?;
    let buf = &mut frame;
    Ok(SnapRealmNode {
        seq: get_u64(buf)?,
        created: get_u64(buf)?,
        subvolume: get_bool(buf)?,
        last_modified: get_timestamp(buf)?,
        change_attr: get_u64(buf)?,
    })
}

fn put_xattrs(buf: &mut BytesMut, x: &XattrMap) {
    buf.put_u32_le(x.len() as u32);
    for (k, v) in x {
        put_blob(buf, k);
        put_blob(buf, v);
    }
}

fn get_xattrs(buf: &mut Bytes) -> Result<XattrMap, DecodeError> {
    need(buf, 4, "XattrMap")?;
    let n = buf.get_u32_le() as usize;
    let mut m = XattrMap::new();
    for _ in 0..n {
        let k = get_blob(buf, "xattr name")?;
        let v = get_blob(buf, "xattr value")?;
        if m.insert(k, v).is_some() {
            return Err(invalid("XattrMap", "duplicate name"));
        }
    }
    Ok(m)
}

/// The full inode image, field order as declared in `InodeCore`.
pub fn put_inode(buf: &mut BytesMut, i: &InodeCore) {
    let start = encode_start(buf, 1, 1);
    buf.put_u64_le(i.ino);
    buf.put_u32_le(i.mode);
    buf.put_u32_le(i.uid);
    buf.put_u32_le(i.gid);
    buf.put_i32_le(i.nlink);
    buf.put_u32_le(i.rdev);
    buf.put_u64_le(i.size);
    buf.put_u64_le(i.max_size_ever);
    buf.put_u32_le(i.truncate_seq);
    buf.put_u64_le(i.truncate_size);
    buf.put_u64_le(i.truncate_from);
    put_timestamp(buf, i.ctime);
    put_timestamp(buf, i.mtime);
    put_timestamp(buf, i.atime);
    put_timestamp(buf, i.btime);
    buf.put_u64_le(i.change_attr);
    buf.put_u64_le(i.version);
    buf.put_u32_le(i.time_warp_seq);
    put_layout(buf, &i.layout);
    buf.put_u32_le(i.old_pools.len() as u32);
    for p in &i.old_pools {
        buf.put_i64_le(*p);
    }
    buf.put_u8(i.dir_hash);
    put_xattrs(buf, &i.xattrs);
    buf.put_u64_le(i.xattr_version);
    put_blob(buf, &i.symlink);
    put_dirstat(buf, &i.dirstat);
    put_rstat(buf, &i.rstat);
    put_quota(buf, &i.quota);
    buf.put_i32_le(i.export_pin);
    buf.put_f64_le(i.export_pin_random);
    buf.put_u8(u8::from(i.export_pin_distributed));
    buf.put_u64_le(i.inline_version);
    buf.put_u32_le(i.client_ranges.len() as u32);
    for (c, r) in &i.client_ranges {
        buf.put_u64_le(*c);
        buf.put_u64_le(*r);
    }
    put_opt(buf, i.snaprealm.as_ref(), put_snaprealm);
    put_blob(buf, &i.fscrypt_auth);
    put_blob(buf, &i.fscrypt_file);
    put_opt(buf, i.stray_auth_path.as_ref(), |b, p| put_blob(b, p));
    buf.put_u64_le(i.stable_subvolume_id);
    put_vec(buf, &i.old_layouts, put_layout);
    encode_finish(buf, start);
}

pub fn get_inode(buf: &mut Bytes) -> Result<InodeCore, DecodeError> {
    let mut frame = get_frame(buf, 1, "InodeCore")?;
    let buf = &mut frame;
    let ino = get_u64(buf)?;
    need(buf, 4 * 4 + 4, "InodeCore")?;
    let mode = buf.get_u32_le();
    let uid = buf.get_u32_le();
    let gid = buf.get_u32_le();
    let nlink = buf.get_i32_le();
    let rdev = buf.get_u32_le();
    let size = get_u64(buf)?;
    let max_size_ever = get_u64(buf)?;
    need(buf, 4, "truncate_seq")?;
    let truncate_seq = buf.get_u32_le();
    let truncate_size = get_u64(buf)?;
    let truncate_from = get_u64(buf)?;
    let ctime = get_timestamp(buf)?;
    let mtime = get_timestamp(buf)?;
    let atime = get_timestamp(buf)?;
    let btime = get_timestamp(buf)?;
    let change_attr = get_u64(buf)?;
    let version = get_u64(buf)?;
    need(buf, 4, "time_warp_seq")?;
    let time_warp_seq = buf.get_u32_le();
    let layout = get_layout(buf)?;
    need(buf, 4, "InodeCore.old_pools")?;
    let npools = buf.get_u32_le() as usize;
    let mut old_pools = BTreeSet::new();
    for _ in 0..npools {
        need(buf, 8, "pool")?;
        if !old_pools.insert(buf.get_i64_le()) {
            return Err(invalid("old_pools", "duplicate pool"));
        }
    }
    let dir_hash = get_u8(buf, "dir_hash")?;
    let xattrs = get_xattrs(buf)?;
    let xattr_version = get_u64(buf)?;
    let symlink = get_blob(buf, "symlink")?;
    let dirstat = get_dirstat(buf)?;
    let rstat = get_rstat(buf)?;
    let quota = get_quota(buf)?;
    need(buf, 4 + 8 + 1 + 8, "InodeCore.export_pin")?;
    let export_pin = buf.get_i32_le();
    let export_pin_random = buf.get_f64_le();
    let export_pin_distributed = get_bool(buf)?;
    let inline_version = get_u64(buf)?;
    need(buf, 4, "InodeCore.client_ranges")?;
    let nranges = buf.get_u32_le() as usize;
    let mut client_ranges = BTreeMap::new();
    for _ in 0..nranges {
        let c = get_u64(buf)?;
        let r = get_u64(buf)?;
        if client_ranges.insert(c, r).is_some() {
            return Err(invalid("client_ranges", "duplicate client"));
        }
    }
    let snaprealm = get_opt(buf, get_snaprealm)?;
    let fscrypt_auth = get_blob(buf, "fscrypt_auth")?;
    let fscrypt_file = get_blob(buf, "fscrypt_file")?;
    Ok(InodeCore {
        ino,
        mode,
        uid,
        gid,
        nlink,
        rdev,
        size,
        max_size_ever,
        truncate_seq,
        truncate_size,
        truncate_from,
        ctime,
        mtime,
        atime,
        btime,
        change_attr,
        version,
        time_warp_seq,
        layout,
        old_pools,
        dir_hash,
        xattrs,
        xattr_version,
        symlink,
        dirstat,
        rstat,
        quota,
        export_pin,
        export_pin_random,
        export_pin_distributed,
        inline_version,
        client_ranges,
        snaprealm,
        fscrypt_auth,
        fscrypt_file,
        stray_auth_path: get_opt(buf, |b| get_blob(b, "stray_auth_path"))?,
        stable_subvolume_id: get_u64(buf)?,
        old_layouts: get_vec(buf, get_layout)?,
    })
}

fn put_ino_range(buf: &mut BytesMut, r: &InoRange) {
    let start = encode_start(buf, 1, 1);
    buf.put_u64_le(r.start);
    buf.put_u64_le(r.len);
    encode_finish(buf, start);
}

fn get_ino_range(buf: &mut Bytes) -> Result<InoRange, DecodeError> {
    let mut frame = get_frame(buf, 1, "InoRange")?;
    let buf = &mut frame;
    Ok(InoRange {
        start: get_u64(buf)?,
        len: get_u64(buf)?,
    })
}

fn put_ino_alloc(buf: &mut BytesMut, s: &InoAllocState) {
    let start = encode_start(buf, 1, 1);
    put_vec(buf, &s.free, put_ino_range);
    put_vec(buf, &s.delegated, |b, (c, r)| {
        b.put_u64_le(*c);
        put_ino_range(b, r);
    });
    buf.put_u64_le(s.floor);
    encode_finish(buf, start);
}

fn get_ino_alloc(buf: &mut Bytes) -> Result<InoAllocState, DecodeError> {
    let mut frame = get_frame(buf, 1, "InoAllocState")?;
    let buf = &mut frame;
    let free = get_vec(buf, get_ino_range)?;
    let delegated = get_vec(buf, |b| Ok((get_u64(b)?, get_ino_range(b)?)))?;
    let floor = get_u64(buf)?;
    Ok(InoAllocState {
        free,
        delegated,
        floor,
    })
}

fn put_completed_request(buf: &mut BytesMut, c: &CompletedRequest) {
    let start = encode_start(buf, 1, 1);
    buf.put_u64_le(c.client);
    buf.put_u64_le(c.session_incarnation);
    buf.put_u64_le(c.tid);
    buf.put_u32_le(c.op);
    buf.put_i32_le(c.result);
    put_opt(buf, c.created_ino.as_ref(), |b, v| b.put_u64_le(*v));
    buf.put_u64_le(c.oldest_client_tid);
    encode_finish(buf, start);
}

fn get_completed_request(buf: &mut Bytes) -> Result<CompletedRequest, DecodeError> {
    let mut frame = get_frame(buf, 1, "CompletedRequest")?;
    let buf = &mut frame;
    let client = get_u64(buf)?;
    let session_incarnation = get_u64(buf)?;
    let tid = get_u64(buf)?;
    need(buf, 8, "CompletedRequest.op_result")?;
    let op = buf.get_u32_le();
    let result = buf.get_i32_le();
    Ok(CompletedRequest {
        client,
        session_incarnation,
        tid,
        op,
        result,
        created_ino: get_opt(buf, get_u64)?,
        oldest_client_tid: get_u64(buf)?,
    })
}

fn put_client_flush(buf: &mut BytesMut, f: &ClientFlush) {
    let start = encode_start(buf, 1, 1);
    buf.put_u64_le(f.client);
    buf.put_u64_le(f.session_incarnation);
    buf.put_u64_le(f.tid);
    buf.put_u64_le(f.oldest_flush_tid);
    encode_finish(buf, start);
}

fn get_client_flush(buf: &mut Bytes) -> Result<ClientFlush, DecodeError> {
    let mut frame = get_frame(buf, 1, "ClientFlush")?;
    let buf = &mut frame;
    Ok(ClientFlush {
        client: get_u64(buf)?,
        session_incarnation: get_u64(buf)?,
        tid: get_u64(buf)?,
        oldest_flush_tid: get_u64(buf)?,
    })
}

fn put_dentry_update(buf: &mut BytesMut, d: &DentryUpdate) {
    let start = encode_start(buf, 1, 1);
    buf.put_u64_le(d.dir);
    put_blob(buf, &d.name);
    put_opt(buf, d.link.as_ref(), |b, l| {
        let start = encode_start(b, 1, 1);
        b.put_u64_le(l.ino);
        b.put_u8(u8::from(l.primary));
        encode_finish(b, start);
    });
    encode_finish(buf, start);
}

fn get_dentry_update(buf: &mut Bytes) -> Result<DentryUpdate, DecodeError> {
    let mut frame = get_frame(buf, 1, "DentryUpdate")?;
    let buf = &mut frame;
    let dir = get_u64(buf)?;
    let name = get_blob(buf, "dentry name")?;
    let link = get_opt(buf, |b| {
        let mut frame = get_frame(b, 1, "DentryLink")?;
        let b = &mut frame;
        Ok(DentryLink {
            ino: get_u64(b)?,
            primary: get_bool(b)?,
        })
    })?;
    Ok(DentryUpdate { dir, name, link })
}

/// The number of a client, as carried in records (used by tests).
#[allow(dead_code)]
fn client_of(c: ClientId) -> ClientId {
    c
}

fn invalid(what: &'static str, detail: &str) -> DecodeError {
    DecodeError::Invalid {
        what,
        detail: detail.to_owned(),
    }
}

pub(crate) fn store_decode_error(error: DecodeError) -> crate::stable::StoreError {
    match error {
        DecodeError::Incompatible { .. } => crate::stable::StoreError::UnsupportedVersion,
        DecodeError::Invalid { detail, .. } if detail == "unsupported semantic schema" => {
            crate::stable::StoreError::UnsupportedVersion
        }
        _ => crate::stable::StoreError::Corrupt,
    }
}

fn get_frame(buf: &mut Bytes, version: u8, what: &'static str) -> Result<Bytes, DecodeError> {
    let frame = decode_start(buf, version, what)?;
    if frame.v != version || frame.compat != version {
        return Err(invalid(what, "unsupported semantic schema"));
    }
    Ok(frame.body)
}

fn put_session_key(b: &mut BytesMut, key: &SessionKey) {
    let start = encode_start(b, 1, 1);
    b.put_u64_le(key.client);
    b.put_u64_le(key.incarnation);
    encode_finish(b, start);
}

fn get_session_key(b: &mut Bytes) -> Result<SessionKey, DecodeError> {
    let mut b = get_frame(b, 1, "SessionKey")?;
    Ok(SessionKey {
        client: get_u64(&mut b)?,
        incarnation: get_u64(&mut b)?,
    })
}

fn put_lifecycle(b: &mut BytesMut, job: &LifecycleIntent) {
    let start = encode_start(b, 1, 1);
    b.put_u64_le(job.id);
    b.put_u64_le(job.ino);
    let kind = encode_start(b, 1, 1);
    match &job.kind {
        LifecycleKind::Purge { layouts, max_size } => {
            b.put_u8(0);
            put_vec(b, layouts, put_layout);
            b.put_u64_le(*max_size);
        }
        LifecycleKind::Truncate {
            layout,
            from,
            size,
            truncate_seq,
        } => {
            b.put_u8(1);
            put_layout(b, layout);
            b.put_u64_le(*from);
            b.put_u64_le(*size);
            b.put_u32_le(*truncate_seq);
        }
        LifecycleKind::RecoverSize {
            layouts,
            max_size,
            inode_version,
            truncate_seq,
        } => {
            b.put_u8(2);
            put_vec(b, layouts, put_layout);
            b.put_u64_le(*max_size);
            b.put_u64_le(*inode_version);
            b.put_u32_le(*truncate_seq);
        }
    }
    // Snapshot context extension point. Only an empty context is supported.
    b.put_u32_le(0);
    encode_finish(b, kind);
    encode_finish(b, start);
}

fn get_lifecycle(b: &mut Bytes) -> Result<LifecycleIntent, DecodeError> {
    let mut b = get_frame(b, 1, "LifecycleIntent")?;
    let id = get_u64(&mut b)?;
    let ino = get_u64(&mut b)?;
    let mut b = get_frame(&mut b, 1, "LifecycleKind")?;
    let kind = match get_u8(&mut b, "LifecycleKind")? {
        0 => LifecycleKind::Purge {
            layouts: get_vec(&mut b, get_layout)?,
            max_size: get_u64(&mut b)?,
        },
        1 => LifecycleKind::Truncate {
            layout: get_layout(&mut b)?,
            from: get_u64(&mut b)?,
            size: get_u64(&mut b)?,
            truncate_seq: get_u32(&mut b)?,
        },
        2 => LifecycleKind::RecoverSize {
            layouts: get_vec(&mut b, get_layout)?,
            max_size: get_u64(&mut b)?,
            inode_version: get_u64(&mut b)?,
            truncate_seq: get_u32(&mut b)?,
        },
        _ => return Err(invalid("LifecycleKind", "unknown kind")),
    };
    if get_u32(&mut b)? != 0 {
        return Err(invalid(
            "LifecycleKind",
            "nonempty snapshot context unsupported",
        ));
    }
    Ok(LifecycleIntent { id, ino, kind })
}

fn get_u32(b: &mut Bytes) -> Result<u32, DecodeError> {
    need(b, 4, "u32")?;
    Ok(b.get_u32_le())
}

fn get_bool(b: &mut Bytes) -> Result<bool, DecodeError> {
    match get_u8(b, "bool")? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(invalid("bool", "invalid discriminant")),
    }
}

/// Encode complete materialized recovery state. No historical WAL is included.
pub fn encode_checkpoint(state: &EngineCheckpoint) -> Bytes {
    let mut b = BytesMut::new();
    let start = encode_start(&mut b, 1, 1);
    b.put_u64_le(state.covered_seq);
    put_vec(&mut b, &state.inodes, put_checkpoint_inode);
    put_ino_alloc(&mut b, &state.ino_alloc);
    put_vec(&mut b, &state.sessions, put_checkpoint_session);
    put_vec(&mut b, &state.jobs, put_lifecycle);
    b.put_u64_le(state.next_job_id);
    b.put_u64_le(state.last_cap_id);
    put_vec(&mut b, &state.cleanup_receipts, |b, (key, seq)| {
        let start = encode_start(b, 1, 1);
        put_session_key(b, key);
        b.put_u64_le(*seq);
        encode_finish(b, start);
    });
    encode_finish(&mut b, start);
    b.freeze()
}

pub fn decode_checkpoint(bytes: &Bytes) -> Result<EngineCheckpoint, DecodeError> {
    let mut input = bytes.clone();
    let mut b = get_frame(&mut input, 1, "EngineCheckpoint")?;
    if !input.is_empty() {
        return Err(invalid("EngineCheckpoint", "trailing bytes"));
    }
    Ok(EngineCheckpoint {
        covered_seq: get_u64(&mut b)?,
        inodes: get_vec(&mut b, get_checkpoint_inode)?,
        ino_alloc: get_ino_alloc(&mut b)?,
        sessions: get_vec(&mut b, get_checkpoint_session)?,
        jobs: get_vec(&mut b, get_lifecycle)?,
        next_job_id: get_u64(&mut b)?,
        last_cap_id: get_u64(&mut b)?,
        cleanup_receipts: get_vec(&mut b, |b| {
            let mut b = get_frame(b, 1, "CleanupReceipt")?;
            Ok((get_session_key(&mut b)?, get_u64(&mut b)?))
        })?,
    })
}

fn put_parent(b: &mut BytesMut, parent: &(u64, Vec<u8>)) {
    let start = encode_start(b, 1, 1);
    b.put_u64_le(parent.0);
    put_blob(b, &parent.1);
    encode_finish(b, start);
}
fn get_parent(b: &mut Bytes) -> Result<(u64, Vec<u8>), DecodeError> {
    let mut b = get_frame(b, 1, "ParentLink")?;
    Ok((get_u64(&mut b)?, get_blob(&mut b, "parent.name")?))
}
fn put_checkpoint_inode(b: &mut BytesMut, inode: &CheckpointInode) {
    let start = encode_start(b, 1, 1);
    put_inode(b, &inode.core);
    put_vec(b, &inode.entries, |b, (key, dentry)| {
        let start = encode_start(b, 1, 1);
        let key_start = encode_start(b, 1, 1);
        b.put_u32_le(key.hash);
        put_blob(b, &key.name);
        encode_finish(b, key_start);
        let dentry_start = encode_start(b, 1, 1);
        b.put_u64_le(dentry.ino);
        b.put_u8(u8::from(dentry.primary));
        encode_finish(b, dentry_start);
        encode_finish(b, start);
    });
    put_opt(b, inode.parent.as_ref(), put_parent);
    put_vec(b, &inode.remote_links, put_parent);
    encode_finish(b, start);
}
fn get_checkpoint_inode(b: &mut Bytes) -> Result<CheckpointInode, DecodeError> {
    let mut b = get_frame(b, 1, "CheckpointInode")?;
    Ok(CheckpointInode {
        core: get_inode(&mut b)?,
        entries: get_vec(&mut b, |b| {
            let mut entry = get_frame(b, 1, "CheckpointEntry")?;
            let mut key = get_frame(&mut entry, 1, "DentryKey")?;
            let key = DentryKey {
                hash: get_u32(&mut key)?,
                name: get_blob(&mut key, "dentry.name")?,
            };
            let mut value = get_frame(&mut entry, 1, "Dentry")?;
            let ino = get_u64(&mut value)?;
            let primary = get_u8(&mut value, "primary")?;
            if primary > 1 {
                return Err(invalid("Dentry", "invalid boolean"));
            }
            Ok((
                key,
                Dentry {
                    ino,
                    primary: primary != 0,
                },
            ))
        })?,
        parent: get_opt(&mut b, get_parent)?,
        remote_links: get_vec(&mut b, get_parent)?,
    })
}
fn put_checkpoint_session(b: &mut BytesMut, session: &CheckpointSession) {
    let start = encode_start(b, 1, 1);
    put_session_key(b, &session.key);
    put_vec(b, &session.completed_requests, put_completed_request);
    put_vec(b, &session.completed_flushes, |b, n| b.put_u64_le(*n));
    b.put_u64_le(session.oldest_client_tid);
    b.put_u64_le(session.oldest_flush_tid);
    encode_finish(b, start);
}
fn get_checkpoint_session(b: &mut Bytes) -> Result<CheckpointSession, DecodeError> {
    let mut b = get_frame(b, 1, "CheckpointSession")?;
    Ok(CheckpointSession {
        key: get_session_key(&mut b)?,
        completed_requests: get_vec(&mut b, get_completed_request)?,
        completed_flushes: get_vec(&mut b, get_u64)?,
        oldest_client_tid: get_u64(&mut b)?,
        oldest_flush_tid: get_u64(&mut b)?,
    })
}
