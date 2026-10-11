//! Small RADOS types (`src/osd/osd_types.h`, `src/common/hobject.h`, `src/include/object.h`).

use std::cmp::Ordering;
use std::fmt;

use denc::{BufMut, Bytes, BytesMut, Decode, DecodeError, Encode, EntityName, Result, encode_finish, encode_start};

use crate::hash::ceph_stable_mod;
use crate::util::decode_start_legacy;

/// `CEPH_NOSNAP`: the head object.
pub const CEPH_NOSNAP: u64 = u64::MAX - 1;
/// `CEPH_SNAPDIR`.
pub const CEPH_SNAPDIR: u64 = u64::MAX;

/// `pg_t`: pool + placement seed. Wire: `u8 1`, `u64 pool`, `u32 seed`, `i32 -1` (the old
/// "preferred" osd), 17 bytes. Ordered by (pool, seed) like `operator<`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PgId {
    pub pool: u64,
    pub seed: u32,
}

impl PgId {
    pub const fn new(pool: u64, seed: u32) -> Self {
        PgId { pool, seed }
    }

    pub fn ps(&self) -> u32 {
        self.seed
    }

    /// `pg_t::is_split(old_pg_num, new_pg_num, nullptr)`.
    pub fn is_split(&self, old_pg_num: u32, new_pg_num: u32) -> bool {
        let seed = self.seed;
        if seed >= old_pg_num {
            // degenerate case
            return false;
        }
        if new_pg_num <= old_pg_num {
            return false;
        }
        let old_bits = crate::hash::cbits(old_pg_num);
        let old_mask = ((1u64 << old_bits) - 1) as u32;
        let mut n: u32 = 1;
        loop {
            let next_bit = (n as u64) << (old_bits - 1);
            if next_bit > u32::MAX as u64 {
                break;
            }
            let s = next_bit as u32 | seed;
            n += 1;
            if s < old_pg_num || s == seed {
                continue;
            }
            if s >= new_pg_num {
                break;
            }
            if ceph_stable_mod(s, old_pg_num, old_mask) == seed {
                return true;
            }
        }
        false
    }

    /// `pg_t::is_merge_source(old_pg_num, new_pg_num, nullptr)`.
    pub fn is_merge_source(&self, old_pg_num: u32, new_pg_num: u32) -> bool {
        self.seed < old_pg_num && self.seed >= new_pg_num
    }

    /// `pg_t::is_merge_target(old_pg_num, new_pg_num)`.
    pub fn is_merge_target(&self, old_pg_num: u32, new_pg_num: u32) -> bool {
        self.seed < new_pg_num && self.is_split(new_pg_num, old_pg_num)
    }
}

impl fmt::Display for PgId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:x}", self.pool, self.seed)
    }
}

impl Encode for PgId {
    fn encode(&self, buf: &mut BytesMut, _f: u64) {
        buf.put_u8(1);
        buf.put_u64_le(self.pool);
        buf.put_u32_le(self.seed);
        buf.put_i32_le(-1);
    }
}

impl Decode for PgId {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let _v = u8::decode(buf)?;
        let pool = u64::decode(buf)?;
        let seed = u32::decode(buf)?;
        let _preferred = i32::decode(buf)?;
        Ok(PgId { pool, seed })
    }
}

/// `shard_id_t::NO_SHARD`.
pub const NO_SHARD: i8 = -1;

/// `spg_t`: `ENCODE_START(1, 1)`, `pg_t`, `i8 shard`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpgId {
    pub pgid: PgId,
    pub shard: i8,
}

impl SpgId {
    pub const fn new(pgid: PgId) -> Self {
        SpgId { pgid, shard: NO_SHARD }
    }
}

impl Default for SpgId {
    fn default() -> Self {
        SpgId::new(PgId::default())
    }
}

impl fmt::Display for SpgId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.shard == NO_SHARD { write!(f, "{}", self.pgid) } else { write!(f, "{}s{}", self.pgid, self.shard) }
    }
}

impl Encode for SpgId {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        let s = encode_start(buf, 1, 1);
        self.pgid.encode(buf, f);
        buf.put_i8(self.shard);
        encode_finish(buf, s);
    }
}

impl Decode for SpgId {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut st = denc::decode_start(buf, 1, "spg_t")?;
        let pgid = PgId::decode(&mut st.body)?;
        let shard = i8::decode(&mut st.body)?;
        Ok(SpgId { pgid, shard })
    }
}

/// `eversion_t`: `u64 version`, `u32 epoch` (12 bytes, raw).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EVersion {
    pub version: u64,
    pub epoch: u32,
}

impl Encode for EVersion {
    fn encode(&self, buf: &mut BytesMut, _f: u64) {
        buf.put_u64_le(self.version);
        buf.put_u32_le(self.epoch);
    }
}

impl Decode for EVersion {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        Ok(EVersion { version: u64::decode(buf)?, epoch: u32::decode(buf)? })
    }
}

/// `osd_reqid_t`: `ENCODE_START(2, 2)`, `entity_name_t name`, `u64 tid`, `i32 inc` (27 bytes).
/// The OSD identifies retried writes by this triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OsdReqId {
    pub name: EntityName,
    pub tid: u64,
    pub inc: i32,
}

impl Default for OsdReqId {
    fn default() -> Self {
        // entity_name_t() is type 0, num 0.
        OsdReqId { name: EntityName::new(denc::EntityType(0), 0), tid: 0, inc: 0 }
    }
}

impl Encode for OsdReqId {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        let s = encode_start(buf, 2, 2);
        self.name.encode(buf, f);
        buf.put_u64_le(self.tid);
        buf.put_i32_le(self.inc);
        encode_finish(buf, s);
    }
}

impl Decode for OsdReqId {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut st = denc::decode_start(buf, 2, "osd_reqid_t")?;
        let b = &mut st.body;
        Ok(OsdReqId { name: EntityName::decode(b)?, tid: u64::decode(b)?, inc: i32::decode(b)? })
    }
}

/// `object_locator_t`: where an object lives (pool, optional key and namespace, or an explicit
/// hash). Wire: `ENCODE_START(6, 3 or 6)`, `i64 pool`, `i32 preferred (-1)`, `string key`,
/// `string nspace`, `i64 hash`; compat is 6 when `hash != -1`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectLocator {
    pub pool: i64,
    /// Locator key (hashed instead of the object name when non-empty).
    pub key: Bytes,
    pub nspace: Bytes,
    /// Explicit placement hash, or -1.
    pub hash: i64,
}

impl ObjectLocator {
    pub fn new(pool: i64) -> Self {
        ObjectLocator { pool, key: Bytes::new(), nspace: Bytes::new(), hash: -1 }
    }

    pub fn with_namespace(pool: i64, nspace: impl Into<Bytes>) -> Self {
        ObjectLocator { pool, key: Bytes::new(), nspace: nspace.into(), hash: -1 }
    }
}

impl Encode for ObjectLocator {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        let compat = if self.hash != -1 { 6 } else { 3 };
        let s = encode_start(buf, 6, compat);
        buf.put_i64_le(self.pool);
        buf.put_i32_le(-1);
        self.key.encode(buf, f);
        self.nspace.encode(buf, f);
        buf.put_i64_le(self.hash);
        encode_finish(buf, s);
    }
}

impl Decode for ObjectLocator {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut st = decode_start_legacy(buf, 6, 3, "object_locator_t")?;
        let b = &mut st.body;
        let pool = i64::decode(b)?;
        let _preferred = i32::decode(b)?;
        let key = Bytes::decode(b)?;
        let nspace = if st.v >= 5 { Bytes::decode(b)? } else { Bytes::new() };
        let hash = if st.v >= 6 { i64::decode(b)? } else { -1 };
        Ok(ObjectLocator { pool, key, nspace, hash })
    }
}

/// `hobject_t`: a hashed object name. Wire: `ENCODE_START(4, 3)`, `string key`, `string oid`,
/// `u64 snap`, `u32 hash`, `bool max`, `string nspace`, `i64 pool`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HObject {
    pub oid: Bytes,
    pub key: Bytes,
    pub snap: u64,
    pub hash: u32,
    pub max: bool,
    pub nspace: Bytes,
    pub pool: i64,
}

impl HObject {
    pub fn new(oid: impl Into<Bytes>, key: impl Into<Bytes>, snap: u64, hash: u32, pool: i64, nspace: impl Into<Bytes>) -> Self {
        HObject { oid: oid.into(), key: key.into(), snap, hash, max: false, nspace: nspace.into(), pool }
    }

    /// `hobject_t::_reverse_bits(hash)`.
    pub fn bitwise_key_u32(&self) -> u32 {
        self.hash.reverse_bits()
    }

    /// `get_bitwise_key()`: max sorts after every hash.
    pub fn bitwise_key(&self) -> u64 {
        if self.max { 0x1_0000_0000 } else { u64::from(self.hash.reverse_bits()) }
    }

    fn effective_key(&self) -> &Bytes {
        if self.key.is_empty() { &self.oid } else { &self.key }
    }
}

/// `operator<=>(hobject_t)` / `cmp()`: the bitwise sort order (max, pool, reversed hash,
/// namespace, effective key, oid, snap).
impl Ord for HObject {
    fn cmp(&self, r: &Self) -> Ordering {
        self.max
            .cmp(&r.max)
            .then(self.pool.cmp(&r.pool))
            .then(self.bitwise_key().cmp(&r.bitwise_key()))
            .then(self.nspace.cmp(&r.nspace))
            .then_with(|| {
                if self.key.is_empty() && r.key.is_empty() {
                    Ordering::Equal
                } else {
                    self.effective_key().cmp(r.effective_key())
                }
            })
            .then(self.oid.cmp(&r.oid))
            .then(self.snap.cmp(&r.snap))
    }
}

impl PartialOrd for HObject {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Encode for HObject {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        let s = encode_start(buf, 4, 3);
        self.key.encode(buf, f);
        self.oid.encode(buf, f);
        buf.put_u64_le(self.snap);
        buf.put_u32_le(self.hash);
        buf.put_u8(u8::from(self.max));
        self.nspace.encode(buf, f);
        buf.put_i64_le(self.pool);
        encode_finish(buf, s);
    }
}

impl Decode for HObject {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut st = decode_start_legacy(buf, 4, 3, "hobject_t")?;
        let b = &mut st.body;
        let key = Bytes::decode(b)?;
        let oid = Bytes::decode(b)?;
        let snap = u64::decode(b)?;
        let hash = u32::decode(b)?;
        let max = if st.v >= 2 { bool::decode(b)? } else { false };
        let (nspace, mut pool) = if st.v >= 4 { (Bytes::decode(b)?, i64::decode(b)?) } else { (Bytes::new(), -1) };
        if st.v >= 4 && pool == -1 && snap == 0 && hash == 0 && !max && oid.is_empty() {
            // compat with hammer: the MIN object
            pool = i64::MIN;
        }
        if max {
            // hobject_t::get_max()
            return Ok(HObject { oid: Bytes::new(), key: Bytes::new(), snap: 0, hash: 0, max: true, nspace: Bytes::new(), pool: 0 });
        }
        Ok(HObject { oid, key, snap, hash, max, nspace, pool })
    }
}

/// `SnapContext` sent with writes: `seq` and the existing snapshots, newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SnapContext {
    pub seq: u64,
    pub snaps: Vec<u64>,
}

/// `ceph_release_t` with the C++ comparison semantics: values with the high bit set (`0xff`,
/// formerly `-1`) mean "unset" and compare below every release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Release(pub u8);

impl Release {
    pub const UNKNOWN: Release = Release(0);
    pub const UNSET: Release = Release(0xff);
    pub const JEWEL: Release = Release(10);
    pub const KRAKEN: Release = Release(11);
    pub const LUMINOUS: Release = Release(12);
    pub const MIMIC: Release = Release(13);
    pub const NAUTILUS: Release = Release(14);
    pub const OCTOPUS: Release = Release(15);
    pub const PACIFIC: Release = Release(16);
    pub const QUINCY: Release = Release(17);
    pub const REEF: Release = Release(18);
    pub const SQUID: Release = Release(19);
    pub const TENTACLE: Release = Release(20);
    pub const UMBRELLA: Release = Release(21);

    /// `operator<` of `ceph_release_t`.
    pub fn lt(self, r: Release) -> bool {
        if (self.0 as i8) < 0 {
            true
        } else if (r.0 as i8) < 0 {
            false
        } else {
            self.0 < r.0
        }
    }

    /// `operator>`.
    pub fn gt(self, r: Release) -> bool {
        if (self.0 as i8) < 0 {
            false
        } else if (r.0 as i8) < 0 {
            true
        } else {
            self.0 > r.0
        }
    }

    /// `operator>=` (`!(lhs < rhs)`).
    pub fn ge(self, r: Release) -> bool {
        !self.lt(r)
    }

    pub fn name(self) -> &'static str {
        const NAMES: [&str; 23] = [
            "unknown", "argonaut", "bobtail", "cuttlefish", "dumpling", "emperor", "firefly", "giant", "hammer",
            "infernalis", "jewel", "kraken", "luminous", "mimic", "nautilus", "octopus", "pacific", "quincy", "reef",
            "squid", "tentacle", "umbrella", "max",
        ];
        NAMES.get(self.0 as usize).copied().unwrap_or("unknown")
    }
}

impl Default for Release {
    fn default() -> Self {
        Release::UNKNOWN
    }
}

/// `errorcode32_t` on the wire is a plain `i32` on Linux (`hostos_to_ceph_errno` is the
/// identity there).
pub fn decode_errorcode32(buf: &mut Bytes) -> Result<i32> {
    i32::decode(buf)
}

/// Name of object `bno` of file `ino` (`file_object_t::c_str`, `"%llx.%08llx"`), e.g.
/// `10000000000.00000000`.
pub fn file_object_name(ino: u64, bno: u64) -> String {
    format!("{ino:x}.{bno:08x}")
}

/// Turn a byte string into text for messages (lossy).
pub fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// `decode` of an `entity_addr_t`-keyed `map<entity_addr_t, utime_t>` into a vector (the denc
/// address type has no total order).
pub fn decode_addr_time_map(buf: &mut Bytes) -> Result<Vec<(denc::EntityAddr, denc::UTime)>> {
    let n = u32::decode(buf)? as usize;
    let mut v = Vec::with_capacity(n.min(1024));
    for _ in 0..n {
        let a = denc::EntityAddr::decode(buf)?;
        let t = denc::UTime::decode(buf)?;
        v.push((a, t));
    }
    Ok(v)
}

pub(crate) fn invalid(what: &'static str, detail: impl Into<String>) -> DecodeError {
    DecodeError::Invalid { what, detail: detail.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::{decode_exact, encode_to_bytes};

    #[test]
    fn pg_t_wire() {
        let pg = PgId::new(4, 0x1f);
        let b = encode_to_bytes(&pg, 0);
        assert_eq!(hex::encode(&b), "0104000000000000001f000000ffffffff");
        assert_eq!(decode_exact::<PgId>(b, "pg").unwrap(), pg);
        assert_eq!(pg.to_string(), "4.1f");
    }

    #[test]
    fn spg_t_wire() {
        let s = SpgId::new(PgId::new(4, 3));
        let b = encode_to_bytes(&s, 0);
        // v1 compat1 len 18 | pg_t (v1, pool 4, seed 3, preferred -1) | shard -1
        assert_eq!(hex::encode(&b), concat!("010112000000", "01", "0400000000000000", "03000000", "ffffffff", "ff"));
        assert_eq!(decode_exact::<SpgId>(b, "spg").unwrap(), s);
    }

    #[test]
    fn reqid_wire() {
        let r = OsdReqId { name: EntityName::new(denc::EntityType::MDS, 0), tid: 7, inc: 12 };
        let b = encode_to_bytes(&r, 0);
        assert_eq!(b.len(), 27);
        // v2 compat2 len 21 | name (type mds, num 0) | tid 7 | inc 12
        assert_eq!(hex::encode(&b), concat!("020215000000", "02", "0000000000000000", "0700000000000000", "0c000000"));
        assert_eq!(decode_exact::<OsdReqId>(b, "reqid").unwrap(), r);
    }

    #[test]
    fn locator_compat_follows_hash() {
        let l = ObjectLocator::new(4);
        let b = encode_to_bytes(&l, 0);
        assert_eq!(&b[0..2], &[6, 3]);
        assert_eq!(decode_exact::<ObjectLocator>(b, "loc").unwrap(), l);
        let mut h = ObjectLocator::new(4);
        h.hash = 5;
        assert_eq!(&encode_to_bytes(&h, 0)[0..2], &[6, 6]);
    }

    #[test]
    fn hobject_order_is_bitwise() {
        let a = HObject::new(&b"a"[..], Bytes::new(), CEPH_NOSNAP, 0x8000_0000, 1, Bytes::new());
        let b = HObject::new(&b"b"[..], Bytes::new(), CEPH_NOSNAP, 0x0000_0001, 1, Bytes::new());
        // reversed bits: 0x80000000 -> 1, 0x1 -> 0x80000000
        assert!(a < b);
        let max = HObject { max: true, ..a.clone() };
        assert!(b < max);
    }

    #[test]
    fn split_and_merge() {
        // pg 1.0 splits when pg_num goes 1 -> 2 (child 1.1)
        assert!(PgId::new(1, 0).is_split(1, 2));
        assert!(PgId::new(1, 3).is_split(8, 16));
        assert!(!PgId::new(1, 3).is_split(8, 8));
        assert!(PgId::new(1, 9).is_merge_source(16, 8));
        assert!(PgId::new(1, 1).is_merge_target(16, 8));
        assert!(!PgId::new(1, 1).is_merge_target(16, 16));
    }

    #[test]
    fn file_object_names() {
        assert_eq!(file_object_name(0x10000000000, 0), "10000000000.00000000");
        assert_eq!(file_object_name(0x1000000000a, 0x1_0000_0003), "1000000000a.100000003");
        assert_eq!(file_object_name(1, 0), "1.00000000");
    }

    #[test]
    fn release_ordering() {
        assert!(Release::UNSET.lt(Release::UNKNOWN));
        assert!(!Release::UNSET.ge(Release::UNKNOWN));
        assert!(Release::TENTACLE.ge(Release::NAUTILUS));
        assert!(Release::LUMINOUS.lt(Release::MIMIC));
    }
}
