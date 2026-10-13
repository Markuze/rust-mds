//! The encoding of engine journal records (WAL bodies).
//!
//! Every type maps to the fields of its engine counterpart, in declaration
//! order, inside one `ENCODE_START(1, 1)` frame per record
//! (`design/store.md` section 3). New fields append and bump the frame
//! version only; changing or removing a field needs a new kind. The
//! functions are plain (not `denc` trait impls) because the types belong
//! to the engine, which stays free of wire code.

use std::collections::{BTreeMap, BTreeSet};

use bytes::{Buf, BufMut, Bytes, BytesMut};

use denc::{DecodeError, decode_start, encode_finish, encode_start, need};
use engine::ino::InoAllocState;
use engine::inode::{InodeCore, SnapRealmNode};
use engine::journal::{
    ClientFlush, CompletedRequest, DentryLink, DentryUpdate, JournalRecord, RecordKind,
};
use engine::types::{ClientId, DirStat, FileLayout, InoRange, Quota, Rstat, Timestamp, XattrMap};

/// The WAL record encoding version this crate writes.
pub const RECORD_VERSION: u8 = 1;

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
    put_vec(&mut buf, &record.closed_sessions, |b, v| b.put_u64_le(*v));
    encode_finish(&mut buf, s);
    buf.freeze()
}

/// Decode one record from its framed body, ignoring fields a newer writer
/// appended inside the frame.
pub fn decode_record(buf: &Bytes) -> Result<JournalRecord, DecodeError> {
    let frame = decode_start(&mut buf.clone(), RECORD_VERSION, "JournalRecord")?;
    let mut b = frame.body;
    Ok(JournalRecord {
        kind: kind_from_code(get_u8(&mut b, "RecordKind")?)?,
        completed_request: get_opt(&mut b, get_completed_request)?,
        client_flush: get_opt(&mut b, get_client_flush)?,
        allocated: get_vec(&mut b, get_u64)?,
        ino_alloc: get_opt(&mut b, get_ino_alloc)?,
        inodes: get_vec(&mut b, get_inode)?,
        dentries: get_vec(&mut b, get_dentry_update)?,
        purged: get_vec(&mut b, get_u64)?,
        closed_sessions: get_vec(&mut b, get_u64)?,
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
    Ok(if buf.get_u8() != 0 {
        Some(get(buf)?)
    } else {
        None
    })
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
    buf.put_u32_le(l.stripe_unit);
    buf.put_u32_le(l.stripe_count);
    buf.put_u32_le(l.object_size);
    buf.put_i64_le(l.pool_id);
    put_blob(buf, l.pool_ns.as_bytes());
}

fn get_layout(buf: &mut Bytes) -> Result<FileLayout, DecodeError> {
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
    put_timestamp(buf, d.mtime);
    buf.put_u64_le(d.change_attr);
    buf.put_i64_le(d.nfiles);
    buf.put_i64_le(d.nsubdirs);
}

fn get_dirstat(buf: &mut Bytes) -> Result<DirStat, DecodeError> {
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
    put_timestamp(buf, r.rctime);
    buf.put_i64_le(r.rbytes);
    buf.put_i64_le(r.rfiles);
    buf.put_i64_le(r.rsubdirs);
    buf.put_i64_le(r.rsnaps);
}

fn get_rstat(buf: &mut Bytes) -> Result<Rstat, DecodeError> {
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
    buf.put_i64_le(q.max_bytes);
    buf.put_i64_le(q.max_files);
}

fn get_quota(buf: &mut Bytes) -> Result<Quota, DecodeError> {
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
    buf.put_u64_le(s.seq);
    buf.put_u64_le(s.created);
    buf.put_u8(u8::from(s.subvolume));
    put_timestamp(buf, s.last_modified);
    buf.put_u64_le(s.change_attr);
}

fn get_snaprealm(buf: &mut Bytes) -> Result<SnapRealmNode, DecodeError> {
    Ok(SnapRealmNode {
        seq: get_u64(buf)?,
        created: get_u64(buf)?,
        subvolume: get_u8(buf, "subvolume")? != 0,
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
        m.insert(k, v);
    }
    Ok(m)
}

/// The full inode image, field order as declared in `InodeCore`.
pub fn put_inode(buf: &mut BytesMut, i: &InodeCore) {
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
}

pub fn get_inode(buf: &mut Bytes) -> Result<InodeCore, DecodeError> {
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
        old_pools.insert(buf.get_i64_le());
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
    let export_pin_distributed = buf.get_u8() != 0;
    let inline_version = get_u64(buf)?;
    need(buf, 4, "InodeCore.client_ranges")?;
    let nranges = buf.get_u32_le() as usize;
    let mut client_ranges = BTreeMap::new();
    for _ in 0..nranges {
        let c = get_u64(buf)?;
        let r = get_u64(buf)?;
        client_ranges.insert(c, r);
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
    })
}

fn put_ino_range(buf: &mut BytesMut, r: &InoRange) {
    buf.put_u64_le(r.start);
    buf.put_u64_le(r.len);
}

fn get_ino_range(buf: &mut Bytes) -> Result<InoRange, DecodeError> {
    Ok(InoRange {
        start: get_u64(buf)?,
        len: get_u64(buf)?,
    })
}

fn put_ino_alloc(buf: &mut BytesMut, s: &InoAllocState) {
    put_vec(buf, &s.free, put_ino_range);
    put_vec(buf, &s.delegated, |b, (c, r)| {
        b.put_u64_le(*c);
        put_ino_range(b, r);
    });
    buf.put_u64_le(s.floor);
}

fn get_ino_alloc(buf: &mut Bytes) -> Result<InoAllocState, DecodeError> {
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
    buf.put_u64_le(c.client);
    buf.put_u64_le(c.tid);
    buf.put_u32_le(c.op);
    buf.put_i32_le(c.result);
    put_opt(buf, c.created_ino.as_ref(), |b, v| b.put_u64_le(*v));
    buf.put_u64_le(c.oldest_client_tid);
}

fn get_completed_request(buf: &mut Bytes) -> Result<CompletedRequest, DecodeError> {
    let client = get_u64(buf)?;
    let tid = get_u64(buf)?;
    need(buf, 8, "CompletedRequest.op_result")?;
    let op = buf.get_u32_le();
    let result = buf.get_i32_le();
    Ok(CompletedRequest {
        client,
        tid,
        op,
        result,
        created_ino: get_opt(buf, get_u64)?,
        oldest_client_tid: get_u64(buf)?,
    })
}

fn put_client_flush(buf: &mut BytesMut, f: &ClientFlush) {
    buf.put_u64_le(f.client);
    buf.put_u64_le(f.tid);
    buf.put_u64_le(f.oldest_flush_tid);
}

fn get_client_flush(buf: &mut Bytes) -> Result<ClientFlush, DecodeError> {
    Ok(ClientFlush {
        client: get_u64(buf)?,
        tid: get_u64(buf)?,
        oldest_flush_tid: get_u64(buf)?,
    })
}

fn put_dentry_update(buf: &mut BytesMut, d: &DentryUpdate) {
    buf.put_u64_le(d.dir);
    put_blob(buf, &d.name);
    put_opt(buf, d.link.as_ref(), |b, l| {
        b.put_u64_le(l.ino);
        b.put_u8(u8::from(l.primary));
    });
}

fn get_dentry_update(buf: &mut Bytes) -> Result<DentryUpdate, DecodeError> {
    let dir = get_u64(buf)?;
    let name = get_blob(buf, "dentry name")?;
    let link = get_opt(buf, |b| {
        Ok(DentryLink {
            ino: get_u64(b)?,
            primary: get_u8(b, "primary")? != 0,
        })
    })?;
    Ok(DentryUpdate { dir, name, link })
}

/// The number of a client, as carried in records (used by tests).
#[allow(dead_code)]
fn client_of(c: ClientId) -> ClientId {
    c
}
