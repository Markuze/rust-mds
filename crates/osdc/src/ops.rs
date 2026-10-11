//! OSD operations: op codes and flags (`src/include/rados.h`), the 38-byte `struct ceph_osd_op`,
//! and an `ObjectOperation` builder for the plain RADOS ops the MDS uses (no object classes:
//! the MDS's cephx OSD caps lack `x`).
//!
//! `struct ceph_osd_op` (packed, little-endian): `u16 op`, `u32 flags`, a 28-byte union whose
//! layout depends on the op (`extent`: `u64 offset`, `u64 length`, `u64 truncate_size`,
//! `u32 truncate_seq`; `xattr`: `u32 name_len`, `u32 value_len`, `u8 cmp_op`, `u8 cmp_mode`;
//! `assert_ver`: `u64 unused`, `u64 ver`; ...), `u32 payload_len`. Each op's input data is
//! concatenated, in op order, into the message's data segment and `payload_len` gives its
//! length (`OSDOp::merge_osd_op_vector_in_data`); replies carry each op's output the same way
//! (`split_osd_op_vector_out_data`).

use std::collections::{BTreeMap, BTreeSet};

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Result, UTime};

/// Op codes (`CEPH_OSD_OP_*`).
pub mod op {
    const MODE_RD: u16 = 0x1000;
    const MODE_WR: u16 = 0x2000;
    const TYPE_DATA: u16 = 0x0200;
    const TYPE_ATTR: u16 = 0x0300;
    pub const READ: u16 = MODE_RD | TYPE_DATA | 1;
    pub const STAT: u16 = MODE_RD | TYPE_DATA | 2;
    pub const ASSERT_VER: u16 = MODE_RD | TYPE_DATA | 8;
    pub const LIST_SNAPS: u16 = MODE_RD | TYPE_DATA | 10;
    pub const WRITE: u16 = MODE_WR | TYPE_DATA | 1;
    pub const WRITEFULL: u16 = MODE_WR | TYPE_DATA | 2;
    pub const TRUNCATE: u16 = MODE_WR | TYPE_DATA | 3;
    pub const ZERO: u16 = MODE_WR | TYPE_DATA | 4;
    pub const DELETE: u16 = MODE_WR | TYPE_DATA | 5;
    pub const APPEND: u16 = MODE_WR | TYPE_DATA | 6;
    pub const SETTRUNC: u16 = MODE_WR | TYPE_DATA | 8;
    pub const TRIMTRUNC: u16 = MODE_WR | TYPE_DATA | 9;
    pub const CREATE: u16 = MODE_WR | TYPE_DATA | 13;
    pub const OMAPGETKEYS: u16 = MODE_RD | TYPE_DATA | 17;
    pub const OMAPGETVALS: u16 = MODE_RD | TYPE_DATA | 18;
    pub const OMAPGETHEADER: u16 = MODE_RD | TYPE_DATA | 19;
    pub const OMAPGETVALSBYKEYS: u16 = MODE_RD | TYPE_DATA | 20;
    pub const OMAPSETVALS: u16 = MODE_WR | TYPE_DATA | 21;
    pub const OMAPSETHEADER: u16 = MODE_WR | TYPE_DATA | 22;
    pub const OMAPCLEAR: u16 = MODE_WR | TYPE_DATA | 23;
    pub const OMAPRMKEYS: u16 = MODE_WR | TYPE_DATA | 24;
    pub const GETXATTR: u16 = MODE_RD | TYPE_ATTR | 1;
    pub const GETXATTRS: u16 = MODE_RD | TYPE_ATTR | 2;
    pub const CMPXATTR: u16 = MODE_RD | TYPE_ATTR | 3;
    pub const SETXATTR: u16 = MODE_WR | TYPE_ATTR | 1;
    pub const RMXATTR: u16 = MODE_WR | TYPE_ATTR | 4;

    /// `ceph_osd_op_mode_read`.
    pub fn is_read(op: u16) -> bool {
        op & MODE_RD != 0
    }

    /// `ceph_osd_op_mode_modify`.
    pub fn is_write(op: u16) -> bool {
        op & MODE_WR != 0
    }

    pub fn name(op: u16) -> &'static str {
        match op {
            READ => "read",
            STAT => "stat",
            ASSERT_VER => "assert-version",
            LIST_SNAPS => "list-snaps",
            WRITE => "write",
            WRITEFULL => "writefull",
            TRUNCATE => "truncate",
            ZERO => "zero",
            DELETE => "delete",
            APPEND => "append",
            SETTRUNC => "settrunc",
            TRIMTRUNC => "trimtrunc",
            CREATE => "create",
            OMAPGETKEYS => "omap-get-keys",
            OMAPGETVALS => "omap-get-vals",
            OMAPGETHEADER => "omap-get-header",
            OMAPGETVALSBYKEYS => "omap-get-vals-by-keys",
            OMAPSETVALS => "omap-set-vals",
            OMAPSETHEADER => "omap-set-header",
            OMAPCLEAR => "omap-clear",
            OMAPRMKEYS => "omap-rm-keys",
            GETXATTR => "getxattr",
            GETXATTRS => "getxattrs",
            CMPXATTR => "cmpxattr",
            SETXATTR => "setxattr",
            RMXATTR => "rmxattr",
            _ => "???",
        }
    }
}

/// Message flags (`CEPH_OSD_FLAG_*`).
pub mod flags {
    pub const ACK: u32 = 0x0001;
    pub const ONNVRAM: u32 = 0x0002;
    pub const ONDISK: u32 = 0x0004;
    pub const RETRY: u32 = 0x0008;
    pub const READ: u32 = 0x0010;
    pub const WRITE: u32 = 0x0020;
    pub const ORDERSNAP: u32 = 0x0040;
    pub const BALANCE_READS: u32 = 0x0100;
    pub const PGOP: u32 = 0x0400;
    pub const EXEC: u32 = 0x0800;
    pub const LOCALIZE_READS: u32 = 0x2000;
    pub const RWORDERED: u32 = 0x4000;
    pub const IGNORE_CACHE: u32 = 0x8000;
    pub const SKIPRWLOCKS: u32 = 0x10000;
    pub const IGNORE_OVERLAY: u32 = 0x20000;
    pub const FLUSH: u32 = 0x40000;
    pub const MAP_SNAP_CLONE: u32 = 0x80000;
    pub const ENFORCE_SNAPC: u32 = 0x100000;
    pub const REDIRECTED: u32 = 0x200000;
    pub const KNOWN_REDIR: u32 = 0x400000;
    pub const FULL_TRY: u32 = 0x800000;
    pub const FULL_FORCE: u32 = 0x1000000;
    pub const IGNORE_REDIRECT: u32 = 0x2000000;
    pub const RETURNVEC: u32 = 0x4000000;
    pub const SUPPORTSPOOLEIO: u32 = 0x8000000;
    pub const EC_DIRECT_READ: u32 = 0x10000000;
    pub const FAIL_ON_EAGAIN: u32 = 0x20000000;
    pub const FORCE_OSD: u32 = 0x40000000;
}

/// Per-op flags (`CEPH_OSD_OP_FLAG_*`).
pub mod op_flags {
    pub const EXCL: u32 = 0x1;
    pub const FAILOK: u32 = 0x2;
    pub const FADVISE_RANDOM: u32 = 0x4;
    pub const FADVISE_SEQUENTIAL: u32 = 0x8;
    pub const FADVISE_WILLNEED: u32 = 0x10;
    pub const FADVISE_DONTNEED: u32 = 0x20;
    pub const FADVISE_NOCACHE: u32 = 0x40;
}

/// `CEPH_OSD_CMPXATTR_OP_*`.
pub mod cmp_op {
    pub const EQ: u8 = 1;
    pub const NE: u8 = 2;
    pub const GT: u8 = 3;
    pub const GTE: u8 = 4;
    pub const LT: u8 = 5;
    pub const LTE: u8 = 6;
}

/// `CEPH_OSD_CMPXATTR_MODE_*`.
pub mod cmp_mode {
    pub const STRING: u8 = 1;
    pub const U64: u8 = 2;
}

/// Size of `struct ceph_osd_op` on the wire.
pub const CEPH_OSD_OP_SIZE: usize = 38;

/// One `OSDOp`: the fixed `ceph_osd_op` plus its input data (requests) or output data and
/// result (replies).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OsdOp {
    pub op: u16,
    pub flags: u32,
    /// The 28-byte argument union, raw.
    pub args: [u8; 28],
    /// Request input (merged into the message data segment).
    pub indata: Bytes,
    /// Reply output (split out of the reply data segment).
    pub outdata: Bytes,
    /// Per-op return value (replies).
    pub rval: i32,
    /// `payload_len` as decoded (equals the in/out data length).
    pub payload_len: u32,
}

impl OsdOp {
    pub fn new(op: u16) -> Self {
        OsdOp { op, ..Default::default() }
    }

    fn put_u64(&mut self, at: usize, v: u64) {
        self.args[at..at + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn put_u32(&mut self, at: usize, v: u32) {
        self.args[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn get_u64(&self, at: usize) -> u64 {
        u64::from_le_bytes(self.args[at..at + 8].try_into().unwrap())
    }
    fn get_u32(&self, at: usize) -> u32 {
        u32::from_le_bytes(self.args[at..at + 4].try_into().unwrap())
    }

    /// `extent.offset`.
    pub fn extent_offset(&self) -> u64 {
        self.get_u64(0)
    }
    /// `extent.length`.
    pub fn extent_length(&self) -> u64 {
        self.get_u64(8)
    }
    /// `extent.truncate_size`.
    pub fn truncate_size(&self) -> u64 {
        self.get_u64(16)
    }
    /// `extent.truncate_seq`.
    pub fn truncate_seq(&self) -> u32 {
        self.get_u32(24)
    }
    pub fn set_extent(&mut self, offset: u64, length: u64) {
        self.put_u64(0, offset);
        self.put_u64(8, length);
    }
    pub fn set_truncate(&mut self, truncate_size: u64, truncate_seq: u32) {
        self.put_u64(16, truncate_size);
        self.put_u32(24, truncate_seq);
    }
    /// `xattr.{name_len, value_len, cmp_op, cmp_mode}`.
    pub fn set_xattr(&mut self, name_len: u32, value_len: u32, cmp_op: u8, cmp_mode: u8) {
        self.put_u32(0, name_len);
        self.put_u32(4, value_len);
        self.args[8] = cmp_op;
        self.args[9] = cmp_mode;
    }
    pub fn xattr_name_len(&self) -> u32 {
        self.get_u32(0)
    }
    pub fn xattr_value_len(&self) -> u32 {
        self.get_u32(4)
    }
    /// `assert_ver.ver`.
    pub fn set_assert_ver(&mut self, ver: u64) {
        self.put_u64(0, 0);
        self.put_u64(8, ver);
    }
    pub fn assert_ver(&self) -> u64 {
        self.get_u64(8)
    }

    /// Encode the 38-byte `ceph_osd_op` with `payload_len`.
    pub fn encode_raw(&self, buf: &mut BytesMut, payload_len: u32) {
        buf.put_u16_le(self.op);
        buf.put_u32_le(self.flags);
        buf.put_slice(&self.args);
        buf.put_u32_le(payload_len);
    }

    /// Decode a 38-byte `ceph_osd_op`.
    pub fn decode_raw(buf: &mut Bytes) -> Result<Self> {
        denc::need(buf, CEPH_OSD_OP_SIZE, "ceph_osd_op")?;
        let op = buf.get_u16_le();
        let flags = buf.get_u32_le();
        let mut args = [0u8; 28];
        buf.copy_to_slice(&mut args);
        let payload_len = buf.get_u32_le();
        Ok(OsdOp { op, flags, args, payload_len, ..Default::default() })
    }
}

/// A compound operation on one object (`ObjectOperation`): applied atomically by the OSD in op
/// order; the first failing op (without `FAILOK`) aborts the whole operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectOperation {
    pub ops: Vec<OsdOp>,
    /// `CEPH_OSD_FLAG_*` the operation adds (e.g. `RWORDERED`).
    pub flags: u32,
}

/// `encode(std::map<std::string, bufferlist>)`.
pub fn encode_omap(map: &BTreeMap<Bytes, Bytes>) -> Bytes {
    let mut b = BytesMut::new();
    map.encode(&mut b, 0);
    b.freeze()
}

/// `encode(std::set<std::string>)`.
pub fn encode_key_set(keys: &BTreeSet<Bytes>) -> Bytes {
    let mut b = BytesMut::new();
    keys.encode(&mut b, 0);
    b.freeze()
}

impl ObjectOperation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Whether any op modifies the object (decides `CEPH_OSD_FLAG_WRITE` vs `READ`).
    pub fn has_write(&self) -> bool {
        self.ops.iter().any(|o| op::is_write(o.op))
    }

    pub fn has_read(&self) -> bool {
        self.ops.iter().any(|o| op::is_read(o.op))
    }

    fn add(&mut self, o: OsdOp) -> &mut OsdOp {
        self.ops.push(o);
        self.ops.last_mut().unwrap()
    }

    /// `add_data(op, off, len, bl)`.
    fn add_data(&mut self, code: u16, off: u64, len: u64, data: Bytes) -> &mut OsdOp {
        let mut o = OsdOp::new(code);
        o.set_extent(off, len);
        o.indata = data;
        self.add(o)
    }

    /// `add_xattr(op, name, data)`: `indata = name ++ data`.
    fn add_xattr(&mut self, code: u16, name: &[u8], data: &[u8], cmp: Option<(u8, u8)>) -> &mut OsdOp {
        let mut o = OsdOp::new(code);
        let (c_op, c_mode) = cmp.unwrap_or((0, 0));
        o.set_xattr(name.len() as u32, data.len() as u32, c_op, c_mode);
        let mut b = BytesMut::with_capacity(name.len() + data.len());
        b.put_slice(name);
        b.put_slice(data);
        o.indata = b.freeze();
        self.add(o)
    }

    /// `CEPH_OSD_OP_FLAG_*` on the last op (e.g. `FAILOK`, `FADVISE_DONTNEED`).
    pub fn set_last_op_flags(&mut self, f: u32) -> &mut Self {
        if let Some(o) = self.ops.last_mut() {
            o.flags |= f;
        }
        self
    }

    /// `CEPH_OSD_OP_CREATE`; `exclusive` fails with `-EEXIST` if the object exists.
    pub fn create(&mut self, exclusive: bool) -> &mut Self {
        let o = self.add(OsdOp::new(op::CREATE));
        o.flags = if exclusive { op_flags::EXCL } else { 0 };
        self
    }

    /// `CEPH_OSD_OP_WRITEFULL`: replace the object's data.
    pub fn write_full(&mut self, data: Bytes) -> &mut Self {
        let len = data.len() as u64;
        self.add_data(op::WRITEFULL, 0, len, data);
        self
    }

    /// `CEPH_OSD_OP_WRITE` at `off`.
    pub fn write(&mut self, off: u64, data: Bytes) -> &mut Self {
        self.write_trunc(off, data, 0, 0)
    }

    /// `CEPH_OSD_OP_WRITE` with `truncate_size`/`truncate_seq` (file data).
    pub fn write_trunc(&mut self, off: u64, data: Bytes, truncate_size: u64, truncate_seq: u32) -> &mut Self {
        let len = data.len() as u64;
        self.add_data(op::WRITE, off, len, data).set_truncate(truncate_size, truncate_seq);
        self
    }

    /// `CEPH_OSD_OP_APPEND`.
    pub fn append(&mut self, data: Bytes) -> &mut Self {
        let len = data.len() as u64;
        self.add_data(op::APPEND, 0, len, data);
        self
    }

    /// `CEPH_OSD_OP_READ` of `len` bytes at `off` (0 = to the end).
    pub fn read(&mut self, off: u64, len: u64) -> &mut Self {
        self.add_data(op::READ, off, len, Bytes::new());
        self
    }

    /// `CEPH_OSD_OP_STAT`: reply data `u64 size`, `utime mtime`.
    pub fn stat(&mut self) -> &mut Self {
        self.add(OsdOp::new(op::STAT));
        self
    }

    /// `CEPH_OSD_OP_DELETE`.
    pub fn remove(&mut self) -> &mut Self {
        self.add_data(op::DELETE, 0, 0, Bytes::new());
        self
    }

    /// `CEPH_OSD_OP_TRUNCATE` to `off`.
    pub fn truncate(&mut self, off: u64) -> &mut Self {
        self.add_data(op::TRUNCATE, off, 0, Bytes::new());
        self
    }

    /// `CEPH_OSD_OP_TRIMTRUNC` (as `Filer::truncate`): trim to `truncate_size` unless the
    /// object already saw `truncate_seq`.
    pub fn trimtrunc(&mut self, truncate_seq: u32, truncate_size: u64) -> &mut Self {
        let o = self.add(OsdOp::new(op::TRIMTRUNC));
        o.set_truncate(truncate_size, truncate_seq);
        self
    }

    /// `CEPH_OSD_OP_ZERO`.
    pub fn zero(&mut self, off: u64, len: u64) -> &mut Self {
        self.add_data(op::ZERO, off, len, Bytes::new());
        self
    }

    /// `CEPH_OSD_OP_SETXATTR`.
    pub fn setxattr(&mut self, name: &str, value: Bytes) -> &mut Self {
        self.add_xattr(op::SETXATTR, name.as_bytes(), &value, None);
        self
    }

    /// `CEPH_OSD_OP_GETXATTR`: reply data is the value.
    pub fn getxattr(&mut self, name: &str) -> &mut Self {
        self.add_xattr(op::GETXATTR, name.as_bytes(), &[], None);
        self
    }

    /// `CEPH_OSD_OP_RMXATTR`.
    pub fn rmxattr(&mut self, name: &str) -> &mut Self {
        self.add_xattr(op::RMXATTR, name.as_bytes(), &[], None);
        self
    }

    /// `CEPH_OSD_OP_CMPXATTR` (string mode): fails the operation with `-ECANCELED` when false.
    pub fn cmpxattr_str(&mut self, name: &str, cmp: u8, value: &[u8]) -> &mut Self {
        self.add_xattr(op::CMPXATTR, name.as_bytes(), value, Some((cmp, cmp_mode::STRING)));
        self
    }

    /// `CEPH_OSD_OP_CMPXATTR` (u64 mode, value encoded as `u64` LE; the stored value is a
    /// decimal string, absent = 0).
    pub fn cmpxattr_u64(&mut self, name: &str, cmp: u8, value: u64) -> &mut Self {
        self.add_xattr(op::CMPXATTR, name.as_bytes(), &value.to_le_bytes(), Some((cmp, cmp_mode::U64)));
        self
    }

    /// `CEPH_OSD_OP_OMAPSETVALS`.
    pub fn omap_set_vals(&mut self, map: &BTreeMap<Bytes, Bytes>) -> &mut Self {
        let bl = encode_omap(map);
        let len = bl.len() as u64;
        self.add_data(op::OMAPSETVALS, 0, len, bl);
        self
    }

    /// `CEPH_OSD_OP_OMAPGETVALS`: indata `string start_after`, `u64 max`, `string prefix`;
    /// reply `map<string,bufferlist>` and `bool more`.
    pub fn omap_get_vals(&mut self, start_after: &[u8], filter_prefix: &[u8], max_to_get: u64) -> &mut Self {
        let mut b = BytesMut::new();
        start_after.encode(&mut b, 0);
        b.put_u64_le(max_to_get);
        filter_prefix.encode(&mut b, 0);
        let len = b.len() as u64;
        self.add_data(op::OMAPGETVALS, 0, len, b.freeze());
        self
    }

    /// `CEPH_OSD_OP_OMAPGETVALSBYKEYS`: reply `map<string,bufferlist>`.
    pub fn omap_get_vals_by_keys(&mut self, keys: &BTreeSet<Bytes>) -> &mut Self {
        let bl = encode_key_set(keys);
        let len = bl.len() as u64;
        self.add_data(op::OMAPGETVALSBYKEYS, 0, len, bl);
        self
    }

    /// `CEPH_OSD_OP_OMAPRMKEYS`.
    pub fn omap_rm_keys(&mut self, keys: &BTreeSet<Bytes>) -> &mut Self {
        let bl = encode_key_set(keys);
        let len = bl.len() as u64;
        self.add_data(op::OMAPRMKEYS, 0, len, bl);
        self
    }

    /// `CEPH_OSD_OP_OMAPGETHEADER`: reply data is the header.
    pub fn omap_get_header(&mut self) -> &mut Self {
        self.add(OsdOp::new(op::OMAPGETHEADER));
        self
    }

    /// `CEPH_OSD_OP_OMAPSETHEADER`.
    pub fn omap_set_header(&mut self, header: Bytes) -> &mut Self {
        let len = header.len() as u64;
        self.add_data(op::OMAPSETHEADER, 0, len, header);
        self
    }

    /// `CEPH_OSD_OP_OMAPCLEAR`.
    pub fn omap_clear(&mut self) -> &mut Self {
        self.add(OsdOp::new(op::OMAPCLEAR));
        self
    }

    /// `CEPH_OSD_OP_ASSERT_VER`: fail unless the object's user version equals `ver`
    /// (`-ERANGE` if older, `-EOVERFLOW` if newer).
    pub fn assert_version(&mut self, ver: u64) -> &mut Self {
        let o = self.add(OsdOp::new(op::ASSERT_VER));
        o.set_assert_ver(ver);
        self
    }
}

// ---- reply decoding helpers ----

/// `STAT` output: size and mtime.
pub fn decode_stat(out: &Bytes) -> Result<(u64, UTime)> {
    let mut b = out.clone();
    Ok((u64::decode(&mut b)?, UTime::decode(&mut b)?))
}

/// `OMAPGETVALS` output: the values and the `more` flag (absent from old OSDs; then inferred
/// from the count like `CB_ObjectOperation_decodevals`).
pub fn decode_omap_vals(out: &Bytes, max_to_get: u64) -> Result<(BTreeMap<Bytes, Bytes>, bool)> {
    let mut b = out.clone();
    let m = BTreeMap::<Bytes, Bytes>::decode(&mut b)?;
    let more = if b.has_remaining() { bool::decode(&mut b)? } else { m.len() as u64 == max_to_get };
    Ok((m, more))
}

/// `OMAPGETVALSBYKEYS` output.
pub fn decode_omap_map(out: &Bytes) -> Result<BTreeMap<Bytes, Bytes>> {
    let mut b = out.clone();
    BTreeMap::<Bytes, Bytes>::decode(&mut b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_codes() {
        assert_eq!(op::READ, 0x1201);
        assert_eq!(op::WRITEFULL, 0x2202);
        assert_eq!(op::DELETE, 0x2205);
        assert_eq!(op::TRIMTRUNC, 0x2209);
        assert_eq!(op::CREATE, 0x220d);
        assert_eq!(op::OMAPGETVALS, 0x1212);
        assert_eq!(op::OMAPSETVALS, 0x2215);
        assert_eq!(op::GETXATTR, 0x1301);
        assert_eq!(op::CMPXATTR, 0x1303);
        assert_eq!(op::SETXATTR, 0x2301);
        assert_eq!(op::ASSERT_VER, 0x1208);
    }

    #[test]
    fn raw_op_is_38_bytes() {
        let mut o = OsdOp::new(op::WRITE);
        o.set_extent(4096, 10);
        o.set_truncate(7, 3);
        let mut b = BytesMut::new();
        o.encode_raw(&mut b, 10);
        assert_eq!(b.len(), CEPH_OSD_OP_SIZE);
        assert_eq!(
            hex::encode(&b),
            concat!("0122", "00000000", "0010000000000000", "0a00000000000000", "0700000000000000", "03000000", "0a000000")
        );
        let back = OsdOp::decode_raw(&mut b.freeze()).unwrap();
        assert_eq!(back.extent_offset(), 4096);
        assert_eq!(back.truncate_seq(), 3);
        assert_eq!(back.payload_len, 10);
    }

    #[test]
    fn xattr_indata_is_name_then_value() {
        let mut op = ObjectOperation::new();
        op.setxattr("parent", Bytes::from_static(b"xyz"));
        let o = &op.ops[0];
        assert_eq!(&o.indata[..], b"parentxyz");
        assert_eq!(o.xattr_name_len(), 6);
        assert_eq!(o.xattr_value_len(), 3);
    }
}
