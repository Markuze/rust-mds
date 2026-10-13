//! `OSDMap` and `OSDMap::Incremental` (`src/osd/OSDMap.{h,cc}`): decoding, applying
//! incrementals, and the placement chain a client uses.
//!
//! Full map wire format (`OSDMap::encode`, modern form, struct_v 8 compat 7):
//!
//! ```text
//! ENCODE_START(8, 7)                          meta-encoding wrapper
//!   ENCODE_START(v ≤ 10, 1)                   client-usable data
//!     uuid fsid, u32 epoch, utime created, utime modified,
//!     map<i64, pg_pool_t> pools, map<i64, string> pool_name, i32 pool_max, u32 flags,
//!     i32 max_osd, vector<u32> osd_state, vector<u32> osd_weight,
//!     vector<entity_addrvec_t> client_addrs, map<pg_t, vector<i32>> pg_temp,
//!     map<pg_t, i32> primary_temp, vector<u32> osd_primary_affinity (empty = default),
//!     bufferlist crush, map<string, map<string,string>> erasure_code_profiles,
//!     map<pg_t, vector<i32>> pg_upmap, map<pg_t, vector<pair<i32,i32>>> pg_upmap_items,
//!     u32 crush_version (v6), map<i64, interval_set> new_removed_snaps, new_purged_snaps
//!     (v7), utime last_up_change, last_in_change (v9), map<pg_t, i32> pg_upmap_primaries (v10)
//!   ENCODE_START(v ≤ 12, 1)                   extended, osd-only data
//!     vector<entity_addrvec_t> hb_back_addrs, vector<osd_info_t> osd_info,
//!     map<entity_addr_t, utime> blocklist, vector<entity_addrvec_t> cluster_addrs,
//!     u32 cluster_snapshot_epoch, string cluster_snapshot, vector<uuid> osd_uuid,
//!     vector<osd_xinfo_t> osd_xinfo, vector<entity_addrvec_t> hb_front_addrs,
//!     float nearfull_ratio, float full_ratio (v2), float backfillfull_ratio (v3),
//!     u8 require_min_compat_client, u8 require_osd_release (v5),
//!     map<i64, interval_set> removed_snaps_queue (v6), map<i32,u32> crush_node_flags (v8),
//!     map<i32,u32> device_class_flags (v9), bool stretch_mode_enabled,
//!     u32 stretch_bucket_count, u32 degraded_stretch_mode, u32 recovering_stretch_mode,
//!     i32 stretch_mode_bucket (v10), map<entity_addr_t, utime> range_blocklist (v11),
//!     bool allow_crimson (v12)
//!   u32 crc   (crc32c(-1) over everything before it, continued over the bytes after it)
//! ```
//!
//! The incremental (`OSDMap::Incremental::encode`, struct_v 8 compat 7) has the same shape
//! with "new_*/old_*" deltas, then `u32 inc_crc`, `u32 full_crc`. Pre-v7 ("classic")
//! encodings are rejected.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use denc::{
    AddrType, Buf, Bytes, Decode, DecodeError, EntityAddr, EntityAddrVec, UTime, Uuid, decode_start,
};

use crate::crush::{CRUSH_ITEM_NONE, CrushMap};
use crate::hash::{CRUSH_HASH_RJENKINS1, crush_hash32_2};
use crate::pool::PgPool;
use crate::types::{ObjectLocator, PgId, Release, decode_addr_time_map, invalid};
use crate::util::{ceph_crc32c, decode_f32};

/// OSD state bits (`CEPH_OSD_*`).
pub mod osd_state {
    pub const EXISTS: u32 = 1 << 0;
    pub const UP: u32 = 1 << 1;
    pub const AUTOOUT: u32 = 1 << 2;
    pub const NEW: u32 = 1 << 3;
    pub const FULL: u32 = 1 << 4;
    pub const NEARFULL: u32 = 1 << 5;
    pub const BACKFILLFULL: u32 = 1 << 6;
    pub const DESTROYED: u32 = 1 << 7;
    pub const NOUP: u32 = 1 << 8;
    pub const NODOWN: u32 = 1 << 9;
    pub const NOIN: u32 = 1 << 10;
    pub const NOOUT: u32 = 1 << 11;
    pub const STOP: u32 = 1 << 12;
}

/// OSDMap flags (`CEPH_OSDMAP_*`).
pub mod map_flags {
    pub const NEARFULL: u32 = 1 << 0;
    pub const FULL: u32 = 1 << 1;
    pub const PAUSERD: u32 = 1 << 2;
    pub const PAUSEWR: u32 = 1 << 3;
    pub const PAUSEREC: u32 = 1 << 4;
    pub const NOUP: u32 = 1 << 5;
    pub const NODOWN: u32 = 1 << 6;
    pub const NOOUT: u32 = 1 << 7;
    pub const NOIN: u32 = 1 << 8;
    pub const NOBACKFILL: u32 = 1 << 9;
    pub const NORECOVER: u32 = 1 << 10;
    pub const NOSCRUB: u32 = 1 << 11;
    pub const NODEEP_SCRUB: u32 = 1 << 12;
    pub const NOTIERAGENT: u32 = 1 << 13;
    pub const NOREBALANCE: u32 = 1 << 14;
    pub const SORTBITWISE: u32 = 1 << 15;
    pub const REQUIRE_JEWEL: u32 = 1 << 16;
    pub const REQUIRE_KRAKEN: u32 = 1 << 17;
    pub const REQUIRE_LUMINOUS: u32 = 1 << 18;
    pub const RECOVERY_DELETES: u32 = 1 << 19;
    pub const PURGED_SNAPDIRS: u32 = 1 << 20;
    pub const NOSNAPTRIM: u32 = 1 << 21;
    pub const PGLOG_HARDLIMIT: u32 = 1 << 22;
    pub const NOAUTOSCALE: u32 = 1 << 23;
    pub const LEGACY_REQUIRE_FLAGS: u32 = REQUIRE_JEWEL | REQUIRE_KRAKEN | REQUIRE_LUMINOUS;
}

/// `CEPH_OSD_IN`.
pub const CEPH_OSD_IN: u32 = 0x10000;
/// `CEPH_OSD_DEFAULT_PRIMARY_AFFINITY` (= `CEPH_OSD_MAX_PRIMARY_AFFINITY`).
pub const DEFAULT_PRIMARY_AFFINITY: u32 = 0x10000;

/// Errors from applying maps.
#[derive(Debug, thiserror::Error)]
pub enum OsdMapError {
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error("incremental fsid {inc} does not match map fsid {map}")]
    FsidMismatch { map: String, inc: String },
    #[error("incremental epoch {inc} does not follow map epoch {map}")]
    EpochGap { map: u32, inc: u32 },
    #[error("incremental refers to osd.{0} beyond max_osd")]
    BadOsd(i32),
    #[error("removed snaps queue for pool {0} missing")]
    MissingSnapQueue(i64),
}

/// `osd_info_t`: `u8 struct_v` (1, no length), then six `u32` epochs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OsdInfo {
    pub last_clean_begin: u32,
    pub last_clean_end: u32,
    pub up_from: u32,
    pub up_thru: u32,
    pub down_at: u32,
    pub lost_at: u32,
}

impl Decode for OsdInfo {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let _v = u8::decode(buf)?;
        Ok(OsdInfo {
            last_clean_begin: u32::decode(buf)?,
            last_clean_end: u32::decode(buf)?,
            up_from: u32::decode(buf)?,
            up_thru: u32::decode(buf)?,
            down_at: u32::decode(buf)?,
            lost_at: u32::decode(buf)?,
        })
    }
}

/// `osd_xinfo_t`: `ENCODE_START(4, 1)`, `utime down_stamp`, `u32 laggy_probability`
/// (fraction × 0xffffffff), `u32 laggy_interval`, `u64 features`, `u32 old_weight`,
/// `utime last_purged_snaps_scrub`, `u32 dead_epoch`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OsdXInfo {
    pub down_stamp: UTime,
    /// As the C++ keeps it: `(float)lp / (float)0xffffffff`.
    pub laggy_probability: f32,
    pub laggy_interval: u32,
    pub features: u64,
    pub old_weight: u32,
    pub last_purged_snaps_scrub: UTime,
    pub dead_epoch: u32,
}

impl Decode for OsdXInfo {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let mut st = decode_start(buf, 4, "osd_xinfo_t")?;
        let b = &mut st.body;
        let down_stamp = UTime::decode(b)?;
        let lp = u32::decode(b)?;
        let laggy_interval = u32::decode(b)?;
        let features = if st.v >= 2 { u64::decode(b)? } else { 0 };
        let old_weight = if st.v >= 3 { u32::decode(b)? } else { 0 };
        let (last_purged_snaps_scrub, dead_epoch) = if st.v >= 4 {
            (UTime::decode(b)?, u32::decode(b)?)
        } else {
            (UTime::default(), 0)
        };
        Ok(OsdXInfo {
            down_stamp,
            laggy_probability: lp as f32 / 0xffff_ffffu32 as f32,
            laggy_interval,
            features,
            old_weight,
            last_purged_snaps_scrub,
            dead_epoch,
        })
    }
}

/// `interval_set<snapid_t>` as `start → length`.
pub type IntervalSet = BTreeMap<u64, u64>;

/// `interval_set::insert(start, len)`, merging with neighbours.
pub fn interval_insert(m: &mut IntervalSet, start: u64, len: u64) {
    if len == 0 {
        return;
    }
    let mut s = start;
    let mut e = start + len;
    // merge with a predecessor that overlaps or touches
    if let Some((&ps, &pl)) = m.range(..=s).next_back()
        && ps + pl >= s
    {
        s = ps;
        e = e.max(ps + pl);
        m.remove(&ps);
    }
    // swallow successors that overlap or touch
    loop {
        let next = m.range(s..).next().map(|(&k, &v)| (k, v));
        match next {
            Some((ns, nl)) if ns <= e => {
                e = e.max(ns + nl);
                m.remove(&ns);
            }
            _ => break,
        }
    }
    m.insert(s, e - s);
}

/// `interval_set::union_of`.
pub fn interval_union(m: &mut IntervalSet, other: &IntervalSet) {
    for (&s, &l) in other {
        interval_insert(m, s, l);
    }
}

/// `interval_set::subtract` (removes each interval of `other`; parts not present are ignored).
pub fn interval_subtract(m: &mut IntervalSet, other: &IntervalSet) {
    for (&os, &ol) in other {
        let oe = os + ol;
        let overlapping: Vec<(u64, u64)> = m
            .range(..oe)
            .filter(|&(&s, &l)| s + l > os)
            .map(|(&s, &l)| (s, l))
            .collect();
        for (s, l) in overlapping {
            m.remove(&s);
            let e = s + l;
            if s < os {
                m.insert(s, os - s);
            }
            if e > oe {
                m.insert(oe, e - oe);
            }
        }
    }
}

/// `OSDMap::range_bits`: a CIDR range from a range-blocklist entry (the nonce is the prefix
/// length).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RangeBits {
    V4 {
        ip: u32,
        mask: u32,
    },
    V6 {
        upper: u64,
        lower: u64,
        upper_mask: u64,
        lower_mask: u64,
    },
    None,
}

impl RangeBits {
    fn parse(a: &EntityAddr) -> Self {
        match a.addr {
            Some(std::net::SocketAddr::V4(v4)) => {
                let ip = u32::from(*v4.ip());
                let mask = if a.nonce > 0 {
                    u32::MAX
                        .checked_shl(32u32.saturating_sub(a.nonce))
                        .unwrap_or(0)
                } else {
                    0
                };
                RangeBits::V4 { ip, mask }
            }
            Some(std::net::SocketAddr::V6(v6)) => {
                let o = v6.ip().octets();
                let upper = u64::from_be_bytes(o[0..8].try_into().unwrap());
                let lower = u64::from_be_bytes(o[8..16].try_into().unwrap());
                let n = a.nonce as i64;
                let lower_shift = (128 - n).min(64);
                let upper_shift = (64 - n).max(0);
                let mask = |shift: i64| {
                    if (0..64).contains(&shift) {
                        u64::MAX << shift
                    } else {
                        0
                    }
                };
                RangeBits::V6 {
                    upper,
                    lower,
                    upper_mask: mask(upper_shift),
                    lower_mask: mask(lower_shift),
                }
            }
            None => RangeBits::None,
        }
    }

    fn matches(&self, a: &EntityAddr) -> bool {
        match (self, a.addr) {
            (RangeBits::V4 { ip, mask }, Some(std::net::SocketAddr::V4(v4))) => {
                (u32::from(*v4.ip()) & mask) == (ip & mask)
            }
            (
                RangeBits::V6 {
                    upper,
                    lower,
                    upper_mask,
                    lower_mask,
                },
                Some(std::net::SocketAddr::V6(v6)),
            ) => {
                let o = v6.ip().octets();
                let u = u64::from_be_bytes(o[0..8].try_into().unwrap());
                let l = u64::from_be_bytes(o[8..16].try_into().unwrap());
                (u & upper_mask) == (upper & upper_mask) && (l & lower_mask) == (lower & lower_mask)
            }
            _ => false,
        }
    }
}

/// A decoded OSDMap.
#[derive(Debug, Clone, PartialEq)]
pub struct OsdMap {
    pub fsid: Uuid,
    pub epoch: u32,
    pub created: UTime,
    pub modified: UTime,
    pub pools: BTreeMap<i64, PgPool>,
    pub pool_name: BTreeMap<i64, String>,
    pub pool_max: i32,
    pub flags: u32,
    pub max_osd: i32,
    pub osd_state: Vec<u32>,
    /// 16.16 reweight, 0x10000 = in, 0 = out.
    pub osd_weight: Vec<u32>,
    /// Public (client-facing) addresses of each OSD.
    pub client_addrs: Vec<EntityAddrVec>,
    pub pg_temp: BTreeMap<PgId, Vec<i32>>,
    pub primary_temp: BTreeMap<PgId, i32>,
    /// `None` = every OSD has the default affinity.
    pub osd_primary_affinity: Option<Vec<u32>>,
    pub crush: Arc<CrushMap>,
    pub erasure_code_profiles: BTreeMap<String, BTreeMap<String, String>>,
    pub pg_upmap: BTreeMap<PgId, Vec<i32>>,
    pub pg_upmap_items: BTreeMap<PgId, Vec<(i32, i32)>>,
    pub pg_upmap_primaries: BTreeMap<PgId, i32>,
    pub crush_version: u32,
    pub new_removed_snaps: BTreeMap<i64, IntervalSet>,
    pub new_purged_snaps: BTreeMap<i64, IntervalSet>,
    pub last_up_change: UTime,
    pub last_in_change: UTime,
    // --- osd-only section ---
    pub hb_back_addrs: Vec<EntityAddrVec>,
    pub osd_info: Vec<OsdInfo>,
    pub blocklist: HashMap<EntityAddr, UTime>,
    pub cluster_addrs: Vec<EntityAddrVec>,
    pub cluster_snapshot_epoch: u32,
    pub cluster_snapshot: String,
    pub osd_uuid: Vec<Uuid>,
    pub osd_xinfo: Vec<OsdXInfo>,
    pub hb_front_addrs: Vec<EntityAddrVec>,
    pub nearfull_ratio: f32,
    pub full_ratio: f32,
    pub backfillfull_ratio: f32,
    pub require_min_compat_client: Release,
    pub require_osd_release: Release,
    pub removed_snaps_queue: BTreeMap<i64, IntervalSet>,
    pub crush_node_flags: BTreeMap<i32, u32>,
    pub device_class_flags: BTreeMap<i32, u32>,
    pub stretch_mode_enabled: bool,
    pub stretch_bucket_count: u32,
    pub degraded_stretch_mode: u32,
    pub recovering_stretch_mode: u32,
    pub stretch_mode_bucket: i32,
    pub range_blocklist: HashMap<EntityAddr, UTime>,
    pub allow_crimson: bool,
}

/// Up and acting sets of a pg (`_pg_to_up_acting_osds`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpActing {
    pub up: Vec<i32>,
    pub up_primary: i32,
    pub acting: Vec<i32>,
    pub acting_primary: i32,
}

/// Where an object maps (for tools and tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectPlacement {
    /// Full-precision hash pg (`object_locator_to_pg`).
    pub raw_pg: PgId,
    /// `raw_pg_to_pg`: the pg that exists.
    pub pg: PgId,
    pub sets: UpActing,
}

fn decode_pg_map<V>(
    buf: &mut Bytes,
    mut f: impl FnMut(&mut Bytes) -> denc::Result<V>,
) -> denc::Result<BTreeMap<PgId, V>> {
    let n = u32::decode(buf)?;
    let mut m = BTreeMap::new();
    for _ in 0..n {
        let k = PgId::decode(buf)?;
        let v = f(buf)?;
        m.insert(k, v);
    }
    Ok(m)
}

fn decode_i32_vec(buf: &mut Bytes) -> denc::Result<Vec<i32>> {
    Vec::<i32>::decode(buf)
}

fn decode_pair_vec(buf: &mut Bytes) -> denc::Result<Vec<(i32, i32)>> {
    Vec::<(i32, i32)>::decode(buf)
}

fn decode_snap_sets(buf: &mut Bytes) -> denc::Result<BTreeMap<i64, IntervalSet>> {
    BTreeMap::<i64, IntervalSet>::decode(buf)
}

/// Check the crc of an OSDMap or Incremental encoding: crc32c(-1) over
/// `[start, crc_offset)`, continued over `[tail_offset, end)`.
fn check_crc(
    all: &[u8],
    crc_offset: usize,
    tail_offset: usize,
    end: usize,
    expected: u32,
    what: &'static str,
) -> denc::Result<()> {
    let mut actual = ceph_crc32c(u32::MAX, &all[..crc_offset]);
    if tail_offset < end {
        actual = ceph_crc32c(actual, &all[tail_offset..end]);
    }
    if actual != expected {
        return Err(invalid(
            what,
            format!("bad crc, actual {actual} != expected {expected}"),
        ));
    }
    Ok(())
}

impl OsdMap {
    /// An empty map at epoch 0 (what the Objecter starts with).
    pub fn empty() -> Self {
        OsdMap {
            fsid: Uuid::default(),
            epoch: 0,
            created: UTime::default(),
            modified: UTime::default(),
            pools: BTreeMap::new(),
            pool_name: BTreeMap::new(),
            pool_max: 0,
            flags: 0,
            max_osd: 0,
            osd_state: Vec::new(),
            osd_weight: Vec::new(),
            client_addrs: Vec::new(),
            pg_temp: BTreeMap::new(),
            primary_temp: BTreeMap::new(),
            osd_primary_affinity: None,
            crush: Arc::new(CrushMap::empty()),
            erasure_code_profiles: BTreeMap::new(),
            pg_upmap: BTreeMap::new(),
            pg_upmap_items: BTreeMap::new(),
            pg_upmap_primaries: BTreeMap::new(),
            crush_version: 1,
            new_removed_snaps: BTreeMap::new(),
            new_purged_snaps: BTreeMap::new(),
            last_up_change: UTime::default(),
            last_in_change: UTime::default(),
            hb_back_addrs: Vec::new(),
            osd_info: Vec::new(),
            blocklist: HashMap::new(),
            cluster_addrs: Vec::new(),
            cluster_snapshot_epoch: 0,
            cluster_snapshot: String::new(),
            osd_uuid: Vec::new(),
            osd_xinfo: Vec::new(),
            hb_front_addrs: Vec::new(),
            nearfull_ratio: 0.0,
            full_ratio: 0.0,
            backfillfull_ratio: 0.0,
            require_min_compat_client: Release::UNKNOWN,
            require_osd_release: Release::UNKNOWN,
            removed_snaps_queue: BTreeMap::new(),
            crush_node_flags: BTreeMap::new(),
            device_class_flags: BTreeMap::new(),
            stretch_mode_enabled: false,
            stretch_bucket_count: 0,
            degraded_stretch_mode: 0,
            recovering_stretch_mode: 0,
            stretch_mode_bucket: 0,
            range_blocklist: HashMap::new(),
            allow_crimson: false,
        }
    }

    /// Decode a full map (`OSDMap::decode`), verifying its crc.
    pub fn decode(bytes: &Bytes) -> denc::Result<Self> {
        let all = bytes.clone();
        let mut buf = bytes.clone();
        denc::need(&buf, 1, "OSDMap")?;
        if buf[0] < 7 {
            return Err(invalid(
                "OSDMap",
                format!("classic encoding v{} not supported", buf[0]),
            ));
        }
        let wrapper = decode_start(&mut buf, 8, "OSDMap")?;
        let body_len = wrapper.body.len();
        let mut w = wrapper.body;
        let mut m = OsdMap::empty();

        // client-usable data
        {
            let mut st = decode_start(&mut w, 10, "OSDMap client data")?;
            let v = st.v;
            let b = &mut st.body;
            m.fsid = Uuid::decode(b)?;
            m.epoch = u32::decode(b)?;
            m.created = UTime::decode(b)?;
            m.modified = UTime::decode(b)?;
            m.pools = BTreeMap::<i64, PgPool>::decode(b)?;
            m.pool_name = BTreeMap::<i64, String>::decode(b)?;
            m.pool_max = i32::decode(b)?;
            m.flags = u32::decode(b)?;
            m.max_osd = i32::decode(b)?;
            m.osd_state = if v >= 5 {
                Vec::<u32>::decode(b)?
            } else {
                Vec::<u8>::decode(b)?.into_iter().map(u32::from).collect()
            };
            m.osd_weight = Vec::<u32>::decode(b)?;
            m.client_addrs = Vec::<EntityAddrVec>::decode(b)?;
            m.pg_temp = decode_pg_map(b, decode_i32_vec)?;
            m.primary_temp = decode_pg_map(b, i32::decode)?;
            if v >= 2 {
                let aff = Vec::<u32>::decode(b)?;
                m.osd_primary_affinity = if aff.is_empty() { None } else { Some(aff) };
            }
            let mut cbl = Bytes::decode(b)?;
            m.crush = Arc::new(CrushMap::decode(&mut cbl)?);
            if v >= 3 {
                m.erasure_code_profiles = BTreeMap::<String, BTreeMap<String, String>>::decode(b)?;
            }
            if v >= 4 {
                m.pg_upmap = decode_pg_map(b, decode_i32_vec)?;
                m.pg_upmap_items = decode_pg_map(b, decode_pair_vec)?;
            }
            if v >= 6 {
                m.crush_version = u32::decode(b)?;
            }
            if v >= 7 {
                m.new_removed_snaps = decode_snap_sets(b)?;
                m.new_purged_snaps = decode_snap_sets(b)?;
            }
            if v >= 9 {
                m.last_up_change = UTime::decode(b)?;
                m.last_in_change = UTime::decode(b)?;
            }
            if v >= 10 {
                m.pg_upmap_primaries = decode_pg_map(b, i32::decode)?;
            }
        }

        // extended, osd-only data
        {
            let mut st = decode_start(&mut w, 12, "OSDMap osd data")?;
            let v = st.v;
            let b = &mut st.body;
            m.hb_back_addrs = Vec::<EntityAddrVec>::decode(b)?;
            m.osd_info = Vec::<OsdInfo>::decode(b)?;
            m.blocklist = decode_addr_time_map(b)?.into_iter().collect();
            m.cluster_addrs = Vec::<EntityAddrVec>::decode(b)?;
            m.cluster_snapshot_epoch = u32::decode(b)?;
            m.cluster_snapshot = String::decode(b)?;
            m.osd_uuid = Vec::<Uuid>::decode(b)?;
            m.osd_xinfo = Vec::<OsdXInfo>::decode(b)?;
            m.hb_front_addrs = Vec::<EntityAddrVec>::decode(b)?;
            if v >= 2 {
                m.nearfull_ratio = decode_f32(b)?;
                m.full_ratio = decode_f32(b)?;
            }
            if v >= 3 {
                m.backfillfull_ratio = decode_f32(b)?;
            }
            if v == 4 {
                let _r = String::decode(b)?;
            }
            if v >= 5 {
                m.require_min_compat_client = Release(u8::decode(b)?);
                m.require_osd_release = Release(u8::decode(b)?);
                if m.require_osd_release.ge(Release::NAUTILUS) {
                    m.flags |= map_flags::PGLOG_HARDLIMIT;
                }
                if m.require_osd_release.ge(Release::LUMINOUS) {
                    m.flags &= !map_flags::LEGACY_REQUIRE_FLAGS;
                    m.flags |= map_flags::RECOVERY_DELETES;
                }
            } else if m.flags & map_flags::REQUIRE_LUMINOUS != 0 {
                m.require_osd_release = Release::LUMINOUS;
                m.flags &= !map_flags::LEGACY_REQUIRE_FLAGS;
                m.flags |= map_flags::RECOVERY_DELETES;
            } else if m.flags & map_flags::REQUIRE_KRAKEN != 0 {
                m.require_osd_release = Release::KRAKEN;
            } else if m.flags & map_flags::REQUIRE_JEWEL != 0 {
                m.require_osd_release = Release::JEWEL;
            } else {
                m.require_osd_release = Release::UNKNOWN;
            }
            if v >= 6 {
                m.removed_snaps_queue = decode_snap_sets(b)?;
            }
            if v >= 8 {
                m.crush_node_flags = BTreeMap::<i32, u32>::decode(b)?;
            }
            if v >= 9 {
                m.device_class_flags = BTreeMap::<i32, u32>::decode(b)?;
            }
            if v >= 10 {
                m.stretch_mode_enabled = bool::decode(b)?;
                m.stretch_bucket_count = u32::decode(b)?;
                m.degraded_stretch_mode = u32::decode(b)?;
                m.recovering_stretch_mode = u32::decode(b)?;
                m.stretch_mode_bucket = i32::decode(b)?;
            }
            if v >= 11 {
                m.range_blocklist = decode_addr_time_map(b)?.into_iter().collect();
            }
            if v >= 12 {
                m.allow_crimson = bool::decode(b)?;
            }
        }

        if wrapper.v >= 8 {
            let crc_offset = 6 + (body_len - w.remaining());
            let crc = u32::decode(&mut w)?;
            let tail_offset = crc_offset + 4;
            check_crc(&all, crc_offset, tail_offset, 6 + body_len, crc, "OSDMap")?;
        }
        Ok(m)
    }

    // ---- accessors (OSDMap.h) ----

    pub fn exists(&self, osd: i32) -> bool {
        osd >= 0
            && osd < self.max_osd
            && self
                .osd_state
                .get(osd as usize)
                .is_some_and(|s| s & osd_state::EXISTS != 0)
    }

    pub fn is_up(&self, osd: i32) -> bool {
        self.exists(osd) && self.osd_state[osd as usize] & osd_state::UP != 0
    }

    pub fn is_down(&self, osd: i32) -> bool {
        !self.is_up(osd)
    }

    pub fn is_out(&self, osd: i32) -> bool {
        !self.exists(osd) || self.osd_weight.get(osd as usize).copied().unwrap_or(0) == 0
    }

    pub fn is_in(&self, osd: i32) -> bool {
        !self.is_out(osd)
    }

    pub fn test_flag(&self, f: u32) -> bool {
        self.flags & f != 0
    }

    pub fn pool(&self, id: i64) -> Option<&PgPool> {
        self.pools.get(&id)
    }

    /// Pool id by name.
    pub fn pool_id(&self, name: &str) -> Option<i64> {
        self.pool_name
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    }

    /// Client-facing addresses of an OSD.
    pub fn osd_addrs(&self, osd: i32) -> Option<&EntityAddrVec> {
        if osd < 0 {
            return None;
        }
        self.client_addrs.get(osd as usize)
    }

    pub fn primary_affinity(&self, osd: i32) -> u32 {
        match &self.osd_primary_affinity {
            Some(v) => v
                .get(osd as usize)
                .copied()
                .unwrap_or(DEFAULT_PRIMARY_AFFINITY),
            None => DEFAULT_PRIMARY_AFFINITY,
        }
    }

    // ---- placement ----

    /// `map_to_pg` / `object_locator_to_pg`: the raw (full-hash) pg of an object; `None` if the
    /// pool does not exist (`-ENOENT`).
    pub fn object_locator_to_pg(&self, oid: &[u8], loc: &ObjectLocator) -> Option<PgId> {
        let pool = self.pool(loc.pool)?;
        if loc.hash >= 0 {
            return Some(PgId::new(loc.pool as u64, loc.hash as u32));
        }
        let ps = if !loc.key.is_empty() {
            pool.hash_key(&loc.key, &loc.nspace)
        } else {
            pool.hash_key(oid, &loc.nspace)
        };
        Some(PgId::new(loc.pool as u64, ps))
    }

    /// `raw_pg_to_pg`.
    pub fn raw_pg_to_pg(&self, pg: PgId) -> Option<PgId> {
        self.pool(pg.pool as i64).map(|p| p.raw_pg_to_pg(pg))
    }

    /// `_remove_nonexistent_osds`.
    fn remove_nonexistent_osds(&self, pool: &PgPool, osds: &mut Vec<i32>) {
        if pool.can_shift_osds() {
            osds.retain(|&o| self.exists(o));
        } else {
            for o in osds.iter_mut() {
                if !self.exists(*o) {
                    *o = CRUSH_ITEM_NONE;
                }
            }
        }
    }

    /// `_pg_to_raw_osds`: CRUSH output (minus nonexistent OSDs) and the placement seed.
    fn pg_to_raw_osds_inner(&self, pool: &PgPool, pg: PgId) -> (Vec<i32>, u32) {
        let pps = pool.raw_pg_to_pps(pg);
        let size = pool.size as usize;
        let ruleno = i32::from(pool.crush_rule);
        let mut osds =
            self.crush
                .do_rule(ruleno, pps as i32, size, &self.osd_weight, pg.pool as i64);
        self.remove_nonexistent_osds(pool, &mut osds);
        (osds, pps)
    }

    /// `_pick_primary`: first non-NONE entry, or -1.
    fn pick_primary(osds: &[i32]) -> i32 {
        osds.iter()
            .copied()
            .find(|&o| o != CRUSH_ITEM_NONE)
            .unwrap_or(-1)
    }

    fn weight_of(&self, osd: i32) -> u32 {
        self.osd_weight.get(osd as usize).copied().unwrap_or(0)
    }

    /// `_apply_upmap`: explicit pg remaps (`pg_upmap`), per-OSD swaps (`pg_upmap_items`) and
    /// primary overrides (`pg_upmap_primaries`).
    fn apply_upmap(&self, pi: &PgPool, raw_pg: PgId, raw: &mut Vec<i32>) {
        let pg = pi.raw_pg_to_pg(raw_pg);
        if let Some(target) = self.pg_upmap.get(&pg) {
            // make sure targets aren't marked out
            for &osd in target {
                if osd != CRUSH_ITEM_NONE
                    && osd < self.max_osd
                    && osd >= 0
                    && self.weight_of(osd) == 0
                {
                    // reject/ignore the explicit mapping
                    return;
                }
            }
            *raw = target.clone();
            // continue to check and apply pg_upmap_items if any
        }
        if let Some(items) = self.pg_upmap_items.get(&pg) {
            for &(osd_from, osd_to) in items {
                // make sure the replacement value doesn't already appear
                let mut exists = false;
                let mut pos: isize = -1;
                for (i, &osd) in raw.iter().enumerate() {
                    if osd == osd_to {
                        exists = true;
                        break;
                    }
                    // ignore mapping if target is marked out (or invalid osd id)
                    if osd == osd_from
                        && pos < 0
                        && !(osd_to != CRUSH_ITEM_NONE
                            && osd_to < self.max_osd
                            && osd_to >= 0
                            && self.weight_of(osd_to) == 0)
                    {
                        pos = i as isize;
                    }
                }
                if !exists && pos >= 0 {
                    raw[pos as usize] = osd_to;
                }
            }
        }
        if let Some(&new_prim) = self.pg_upmap_primaries.get(&pg) {
            // apply only if the new primary is valid and not marked out
            if new_prim != CRUSH_ITEM_NONE
                && new_prim < self.max_osd
                && new_prim >= 0
                && self.weight_of(new_prim) != 0
            {
                // start from 1 on purpose
                let new_prim_idx = raw
                    .iter()
                    .skip(1)
                    .position(|&o| o == new_prim)
                    .map(|p| p + 1)
                    .unwrap_or(0);
                if new_prim_idx > 0 {
                    raw[new_prim_idx] = raw[0];
                    raw[0] = new_prim;
                }
            }
        }
    }

    /// `_raw_to_up_osds`: drop (replicated) or blank (EC) down and nonexistent OSDs.
    fn raw_to_up_osds(&self, pool: &PgPool, raw: &[i32]) -> Vec<i32> {
        if pool.can_shift_osds() {
            raw.iter()
                .copied()
                .filter(|&o| self.exists(o) && !self.is_down(o))
                .collect()
        } else {
            raw.iter()
                .map(|&o| {
                    if !self.exists(o) || self.is_down(o) {
                        CRUSH_ITEM_NONE
                    } else {
                        o
                    }
                })
                .collect()
        }
    }

    /// `_apply_primary_affinity`.
    fn apply_primary_affinity(
        &self,
        seed: u32,
        pool: &PgPool,
        osds: &mut [i32],
        primary: &mut i32,
    ) {
        let Some(aff) = &self.osd_primary_affinity else {
            return;
        };
        let affinity = |o: i32| {
            aff.get(o as usize)
                .copied()
                .unwrap_or(DEFAULT_PRIMARY_AFFINITY)
        };
        // do we have any non-default primary_affinity values for these osds?
        if !osds
            .iter()
            .any(|&o| o != CRUSH_ITEM_NONE && affinity(o) != DEFAULT_PRIMARY_AFFINITY)
        {
            return;
        }
        // pick the primary.  feed both the seed (for the pg) and the osd into the hash/rng so
        // that a proportional fraction of an osd's pgs get rejected as primary.
        let mut pos: isize = -1;
        for (i, &o) in osds.iter().enumerate() {
            if o == CRUSH_ITEM_NONE {
                continue;
            }
            let a = affinity(o);
            if a < DEFAULT_PRIMARY_AFFINITY
                && (crush_hash32_2(CRUSH_HASH_RJENKINS1, seed, o as u32) >> 16) >= a
            {
                // we chose not to use this primary.  note it anyway as a fallback in case we
                // don't pick anyone else, but keep looking.
                if pos < 0 {
                    pos = i as isize;
                }
            } else {
                pos = i as isize;
                break;
            }
        }
        if pos < 0 {
            return;
        }
        let pos = pos as usize;
        *primary = osds[pos];
        if pool.can_shift_osds() && pos > 0 {
            // move the new primary to the front.
            for i in (1..=pos).rev() {
                osds[i] = osds[i - 1];
            }
            osds[0] = *primary;
        }
    }

    /// `pgtemp_undo_primaryfirst` (optimized EC pools reorder pg_temp so that shards that can
    /// be primary come first).
    fn pgtemp_undo_primaryfirst(&self, pool: &PgPool, acting: Vec<i32>) -> Vec<i32> {
        if !pool.allows_ecoptimizations() || acting.len() != pool.size as usize {
            return acting;
        }
        let mut result = Vec::with_capacity(acting.len());
        let mut primaryshard = 0usize;
        let mut nonprimaryshard = pool.size as usize - pool.nonprimary_shard_count();
        for shard in 0..pool.size {
            if pool.is_nonprimary_shard(shard as i8) {
                result.push(
                    acting
                        .get(nonprimaryshard)
                        .copied()
                        .unwrap_or(CRUSH_ITEM_NONE),
                );
                nonprimaryshard += 1;
            } else {
                result.push(acting.get(primaryshard).copied().unwrap_or(CRUSH_ITEM_NONE));
                primaryshard += 1;
            }
        }
        result
    }

    /// `_get_temp_osds`: the pg_temp acting set (if any) and the temp primary (`primary_temp`,
    /// else the first live pg_temp entry), or -1.
    fn get_temp_osds(&self, pool: &PgPool, pg: PgId) -> (Vec<i32>, i32) {
        let pg = pool.raw_pg_to_pg(pg);
        let mut temp = Vec::new();
        let mut temp_pg = Vec::new();
        if let Some(t) = self.pg_temp.get(&pg) {
            for &o in t {
                if !self.exists(o) || self.is_down(o) {
                    if pool.can_shift_osds() {
                        continue;
                    }
                    temp.push(CRUSH_ITEM_NONE);
                } else {
                    temp.push(o);
                }
            }
            temp_pg = self.pgtemp_undo_primaryfirst(pool, temp.clone());
        }
        let temp_primary = if let Some(&p) = self.primary_temp.get(&pg) {
            p
        } else {
            temp.iter()
                .copied()
                .find(|&o| o != CRUSH_ITEM_NONE)
                .unwrap_or(-1)
        };
        (temp_pg, temp_primary)
    }

    /// `pg_to_raw_osds`: CRUSH result for a pg, and its first entry.
    pub fn pg_to_raw_osds(&self, pg: PgId) -> (Vec<i32>, i32) {
        let Some(pool) = self.pool(pg.pool as i64) else {
            return (Vec::new(), -1);
        };
        let (raw, _) = self.pg_to_raw_osds_inner(pool, pg);
        let p = Self::pick_primary(&raw);
        (raw, p)
    }

    /// `pg_to_raw_up`: up set before pg_temp (upmaps, up filtering, primary affinity).
    pub fn pg_to_raw_up(&self, pg: PgId) -> (Vec<i32>, i32) {
        let Some(pool) = self.pool(pg.pool as i64) else {
            return (Vec::new(), -1);
        };
        let (mut raw, pps) = self.pg_to_raw_osds_inner(pool, pg);
        self.apply_upmap(pool, pg, &mut raw);
        let mut up = self.raw_to_up_osds(pool, &raw);
        let mut primary = Self::pick_primary(&raw);
        self.apply_primary_affinity(pps, pool, &mut up, &mut primary);
        (up, primary)
    }

    /// `pg_to_up_acting_osds` (`_pg_to_up_acting_osds` with `raw_pg_to_pg = true`): the full
    /// chain CRUSH → upmaps → up set → primary affinity → pg_temp/primary_temp.
    pub fn pg_to_up_acting_osds(&self, pg: PgId) -> UpActing {
        let Some(pool) = self.pool(pg.pool as i64) else {
            return UpActing {
                up: Vec::new(),
                up_primary: -1,
                acting: Vec::new(),
                acting_primary: -1,
            };
        };
        let (mut acting, mut acting_primary) = self.get_temp_osds(pool, pg);
        let (mut raw, pps) = self.pg_to_raw_osds_inner(pool, pg);
        self.apply_upmap(pool, pg, &mut raw);
        let mut up = self.raw_to_up_osds(pool, &raw);
        let mut up_primary = Self::pick_primary(&up);
        self.apply_primary_affinity(pps, pool, &mut up, &mut up_primary);
        if acting.is_empty() {
            acting = up.clone();
            if acting_primary == -1 {
                acting_primary = up_primary;
            }
        }
        UpActing {
            up,
            up_primary,
            acting,
            acting_primary,
        }
    }

    /// Map an object name in a pool (default namespace, no key) through the whole chain.
    pub fn map_object(&self, pool: i64, name: &[u8], nspace: &[u8]) -> Option<ObjectPlacement> {
        let loc = ObjectLocator::with_namespace(pool, Bytes::copy_from_slice(nspace));
        let raw_pg = self.object_locator_to_pg(name, &loc)?;
        let pg = self.raw_pg_to_pg(raw_pg)?;
        let sets = self.pg_to_up_acting_osds(pg);
        Some(ObjectPlacement { raw_pg, pg, sets })
    }

    // ---- blocklist ----

    /// `is_blocklisted(addr)`: exact entry (type ANY since nautilus), whole-IP entry (port and
    /// nonce 0), or a range entry.
    pub fn is_blocklisted(&self, orig: &EntityAddr) -> bool {
        if self.blocklist.is_empty() && self.range_blocklist.is_empty() {
            return false;
        }
        let mut a = *orig;
        a.ty = if self.require_osd_release.lt(Release::NAUTILUS) {
            AddrType::LEGACY
        } else {
            AddrType::ANY
        };
        if self.blocklist.contains_key(&a) {
            return true;
        }
        if let Some(sa) = a.addr.as_mut() {
            sa.set_port(0);
            a.nonce = 0;
            if self.blocklist.contains_key(&a) {
                return true;
            }
        }
        self.range_blocklist
            .keys()
            .any(|r| RangeBits::parse(r).matches(&a))
    }

    /// `is_blocklisted(addrvec)`: any of the addresses.
    pub fn is_blocklisted_vec(&self, av: &EntityAddrVec) -> bool {
        av.0.iter().any(|a| self.is_blocklisted(a))
    }

    // ---- incrementals ----

    /// `set_max_osd`.
    pub fn set_max_osd(&mut self, m: i32) {
        let n = m.max(0) as usize;
        self.max_osd = m;
        self.osd_state.resize(n, 0);
        self.osd_weight.resize(n, 0);
        self.osd_info.resize(n, OsdInfo::default());
        self.osd_xinfo.resize(n, OsdXInfo::default());
        self.client_addrs.resize(n, EntityAddrVec::default());
        self.cluster_addrs.resize(n, EntityAddrVec::default());
        self.hb_back_addrs.resize(n, EntityAddrVec::default());
        self.hb_front_addrs.resize(n, EntityAddrVec::default());
        self.osd_uuid.resize(n, Uuid::default());
        if let Some(a) = self.osd_primary_affinity.as_mut() {
            a.resize(n, DEFAULT_PRIMARY_AFFINITY);
        }
    }

    fn check_osd(&self, osd: i32) -> Result<usize, OsdMapError> {
        if osd < 0 || osd >= self.max_osd || osd as usize >= self.osd_state.len() {
            return Err(OsdMapError::BadOsd(osd));
        }
        Ok(osd as usize)
    }

    fn set_primary_affinity(&mut self, osd: usize, w: u32) {
        let n = self.max_osd.max(0) as usize;
        let a = self
            .osd_primary_affinity
            .get_or_insert_with(|| vec![DEFAULT_PRIMARY_AFFINITY; n]);
        if osd < a.len() {
            a[osd] = w;
        }
    }

    /// `apply_incremental`: advance this map by one epoch.
    pub fn apply_incremental(&mut self, inc: &Incremental) -> Result<(), OsdMapError> {
        if inc.epoch == 1 {
            self.fsid = inc.fsid;
        } else if inc.fsid != self.fsid {
            return Err(OsdMapError::FsidMismatch {
                map: format!("{:?}", self.fsid),
                inc: format!("{:?}", inc.fsid),
            });
        }
        if inc.epoch != self.epoch.wrapping_add(1) {
            return Err(OsdMapError::EpochGap {
                map: self.epoch,
                inc: inc.epoch,
            });
        }
        // full map?
        if !inc.fullmap.is_empty() {
            *self = OsdMap::decode(&inc.fullmap)?;
            return Ok(());
        }
        // Validate everything that indexes per-OSD vectors before mutating.
        let new_max = if inc.new_max_osd >= 0 {
            inc.new_max_osd
        } else {
            self.max_osd
        };
        let osd_ok = |o: &i32| *o >= 0 && *o < new_max;
        for o in inc
            .new_weight
            .keys()
            .chain(inc.new_primary_affinity.keys())
            .chain(inc.new_state.keys())
            .chain(inc.new_up_client.keys())
            .chain(inc.new_up_cluster.keys())
            .chain(inc.new_up_thru.keys())
            .chain(inc.new_last_clean_interval.keys())
            .chain(inc.new_lost.keys())
            .chain(inc.new_xinfo.keys())
            .chain(inc.new_uuid.keys())
        {
            if !osd_ok(o) {
                return Err(OsdMapError::BadOsd(*o));
            }
        }
        for p in inc.new_purged_snaps.keys() {
            let queued =
                self.removed_snaps_queue.contains_key(p) || inc.new_removed_snaps.contains_key(p);
            if !queued {
                return Err(OsdMapError::MissingSnapQueue(*p));
            }
        }

        self.epoch += 1;
        self.modified = inc.modified;

        if inc.new_flags >= 0 {
            self.flags = inc.new_flags as u32;
            // cover a newly-upgraded luminous mon cluster that has to set require_jewel_osds or
            // require_kraken_osds before the osds can be upgraded to luminous
            if self.flags & map_flags::REQUIRE_KRAKEN != 0 {
                if self.require_osd_release.lt(Release::KRAKEN) {
                    self.require_osd_release = Release::KRAKEN;
                }
            } else if self.flags & map_flags::REQUIRE_JEWEL != 0
                && self.require_osd_release.lt(Release::JEWEL)
            {
                self.require_osd_release = Release::JEWEL;
            }
        }
        if inc.new_max_osd >= 0 {
            self.set_max_osd(inc.new_max_osd);
        }
        if inc.new_pool_max != -1 {
            self.pool_max = inc.new_pool_max as i32;
        }
        for (id, pool) in &inc.new_pools {
            let mut p = pool.clone();
            p.last_change = self.epoch;
            self.pools.insert(*id, p);
        }
        self.new_removed_snaps = inc.new_removed_snaps.clone();
        self.new_purged_snaps = inc.new_purged_snaps.clone();
        for (pool, set) in &self.new_removed_snaps {
            interval_union(self.removed_snaps_queue.entry(*pool).or_default(), set);
        }
        for (pool, set) in &self.new_purged_snaps {
            if let Some(q) = self.removed_snaps_queue.get_mut(pool) {
                interval_subtract(q, set);
                if q.is_empty() {
                    self.removed_snaps_queue.remove(pool);
                }
            }
        }
        if inc.new_last_up_change != UTime::default() {
            self.last_up_change = inc.new_last_up_change;
        }
        if inc.new_last_in_change != UTime::default() {
            self.last_in_change = inc.new_last_in_change;
        }
        for (id, name) in &inc.new_pool_names {
            self.pool_name.insert(*id, name.clone());
        }
        for id in &inc.old_pools {
            self.pools.remove(id);
            self.pool_name.remove(id);
        }
        for (&osd, &w) in &inc.new_weight {
            let o = self.check_osd(osd)?;
            self.osd_weight[o] = w;
            if w != 0 {
                self.osd_state[o] |= osd_state::EXISTS;
                // if we are marking in, clear the AUTOOUT and NEW bits, and clear xinfo
                // old_weight.
                self.osd_state[o] &= !(osd_state::AUTOOUT | osd_state::NEW);
                self.osd_xinfo[o].old_weight = 0;
            }
        }
        for (&osd, &a) in &inc.new_primary_affinity {
            let o = self.check_osd(osd)?;
            self.set_primary_affinity(o, a);
        }
        // erasure_code_profiles
        for p in &inc.old_erasure_code_profiles {
            self.erasure_code_profiles.remove(p);
        }
        for (name, prof) in &inc.new_erasure_code_profiles {
            self.erasure_code_profiles
                .insert(name.clone(), prof.clone());
        }
        // up/down
        for (&osd, &st) in &inc.new_state {
            let o = self.check_osd(osd)?;
            let s = if st != 0 { st } else { osd_state::UP };
            if self.osd_state[o] & osd_state::UP != 0 && s & osd_state::UP != 0 {
                self.osd_info[o].down_at = self.epoch;
                self.osd_xinfo[o].down_stamp = self.modified;
            }
            if self.osd_state[o] & osd_state::EXISTS != 0 && s & osd_state::EXISTS != 0 {
                // osd is destroyed; clear out anything interesting.
                self.osd_uuid[o] = Uuid::default();
                self.osd_info[o] = OsdInfo::default();
                self.osd_xinfo[o] = OsdXInfo::default();
                self.set_primary_affinity(o, DEFAULT_PRIMARY_AFFINITY);
                self.client_addrs[o] = EntityAddrVec::default();
                self.cluster_addrs[o] = EntityAddrVec::default();
                self.hb_front_addrs[o] = EntityAddrVec::default();
                self.hb_back_addrs[o] = EntityAddrVec::default();
                self.osd_state[o] = 0;
            } else {
                self.osd_state[o] ^= s;
            }
        }
        for (&osd, addrs) in &inc.new_up_client {
            let o = self.check_osd(osd)?;
            self.osd_state[o] |= osd_state::EXISTS | osd_state::UP;
            self.osd_state[o] &= !osd_state::STOP;
            self.client_addrs[o] = addrs.clone();
            self.hb_back_addrs[o] = inc.new_hb_back_up.get(&osd).cloned().unwrap_or_default();
            self.hb_front_addrs[o] = inc.new_hb_front_up.get(&osd).cloned().unwrap_or_default();
            self.osd_info[o].up_from = self.epoch;
        }
        for (&osd, addrs) in &inc.new_up_cluster {
            let o = self.check_osd(osd)?;
            self.cluster_addrs[o] = addrs.clone();
        }
        // info
        for (&osd, &e) in &inc.new_up_thru {
            let o = self.check_osd(osd)?;
            self.osd_info[o].up_thru = e;
        }
        for (&osd, &(b, e)) in &inc.new_last_clean_interval {
            let o = self.check_osd(osd)?;
            self.osd_info[o].last_clean_begin = b;
            self.osd_info[o].last_clean_end = e;
        }
        for (&osd, &e) in &inc.new_lost {
            let o = self.check_osd(osd)?;
            self.osd_info[o].lost_at = e;
        }
        // xinfo
        for (&osd, x) in &inc.new_xinfo {
            let o = self.check_osd(osd)?;
            self.osd_xinfo[o] = *x;
        }
        // uuid
        for (&osd, u) in &inc.new_uuid {
            let o = self.check_osd(osd)?;
            self.osd_uuid[o] = *u;
        }
        // pg rebuild
        for (pg, v) in &inc.new_pg_temp {
            if v.is_empty() {
                self.pg_temp.remove(pg);
            } else {
                self.pg_temp.insert(*pg, v.clone());
            }
        }
        for (pg, &p) in &inc.new_primary_temp {
            if p == -1 {
                self.primary_temp.remove(pg);
            } else {
                self.primary_temp.insert(*pg, p);
            }
        }
        for (pg, v) in &inc.new_pg_upmap {
            self.pg_upmap.insert(*pg, v.clone());
        }
        for pg in &inc.old_pg_upmap {
            self.pg_upmap.remove(pg);
        }
        for (pg, v) in &inc.new_pg_upmap_items {
            self.pg_upmap_items.insert(*pg, v.clone());
        }
        for pg in &inc.old_pg_upmap_items {
            self.pg_upmap_items.remove(pg);
        }
        for (pg, &p) in &inc.new_pg_upmap_primary {
            self.pg_upmap_primaries.insert(*pg, p);
        }
        for pg in &inc.old_pg_upmap_primary {
            self.pg_upmap_primaries.remove(pg);
        }
        // blocklist (insert keeps an existing entry, as unordered_map::insert does)
        for (a, t) in &inc.new_blocklist {
            self.blocklist.entry(*a).or_insert(*t);
        }
        for a in &inc.old_blocklist {
            self.blocklist.remove(a);
        }
        for (a, t) in &inc.new_range_blocklist {
            self.range_blocklist.entry(*a).or_insert(*t);
        }
        for a in &inc.old_range_blocklist {
            self.range_blocklist.remove(a);
        }
        for (&id, &f) in &inc.new_crush_node_flags {
            if f != 0 {
                self.crush_node_flags.insert(id, f);
            } else {
                self.crush_node_flags.remove(&id);
            }
        }
        for (&id, &f) in &inc.new_device_class_flags {
            if f != 0 {
                self.device_class_flags.insert(id, f);
            } else {
                self.device_class_flags.remove(&id);
            }
        }
        // cluster snapshot?
        if !inc.cluster_snapshot.is_empty() {
            self.cluster_snapshot = inc.cluster_snapshot.clone();
            self.cluster_snapshot_epoch = inc.epoch;
        } else {
            self.cluster_snapshot.clear();
            self.cluster_snapshot_epoch = 0;
        }
        if inc.new_nearfull_ratio >= 0.0 {
            self.nearfull_ratio = inc.new_nearfull_ratio;
        }
        if inc.new_backfillfull_ratio >= 0.0 {
            self.backfillfull_ratio = inc.new_backfillfull_ratio;
        }
        if inc.new_full_ratio >= 0.0 {
            self.full_ratio = inc.new_full_ratio;
        }
        if inc.new_require_min_compat_client.gt(Release::UNKNOWN) {
            self.require_min_compat_client = inc.new_require_min_compat_client;
        }
        if inc.new_require_osd_release.ge(Release::UNKNOWN) {
            self.require_osd_release = inc.new_require_osd_release;
            if self.require_osd_release.ge(Release::LUMINOUS) {
                self.flags &= !map_flags::LEGACY_REQUIRE_FLAGS;
                self.flags |= map_flags::RECOVERY_DELETES;
            }
            if self.require_osd_release.ge(Release::NAUTILUS) {
                self.flags |= map_flags::PGLOG_HARDLIMIT;
            }
        }
        // do new crush map last (after up/down stuff)
        if !inc.crush.is_empty() {
            let mut cb = inc.crush.clone();
            let crush = CrushMap::decode(&mut cb)?;
            if self.require_osd_release.ge(Release::LUMINOUS) {
                self.crush_version = self.crush_version.wrapping_add(1);
            }
            self.device_class_flags
                .retain(|id, _| crush.class_name.contains_key(id));
            self.crush = Arc::new(crush);
        }
        if inc.change_stretch_mode {
            self.stretch_mode_enabled = inc.stretch_mode_enabled;
            self.stretch_bucket_count = inc.new_stretch_bucket_count;
            self.degraded_stretch_mode = inc.new_degraded_stretch_mode;
            self.recovering_stretch_mode = inc.new_recovering_stretch_mode;
            self.stretch_mode_bucket = inc.new_stretch_mode_bucket;
        }
        match inc.mutate_allow_crimson {
            1 => self.allow_crimson = true,
            2 => self.allow_crimson = false,
            _ => {}
        }
        Ok(())
    }
}

impl CrushMap {
    /// An empty CRUSH map (`CrushWrapper::create()` with default tunables).
    pub fn empty() -> Self {
        CrushMap {
            buckets: Vec::new(),
            rules: Vec::new(),
            max_devices: 0,
            encoded_max_devices: 0,
            tunables: crate::crush::Tunables::optimal(),
            type_map: BTreeMap::new(),
            name_map: BTreeMap::new(),
            rule_name_map: BTreeMap::new(),
            class_map: BTreeMap::new(),
            class_name: BTreeMap::new(),
            class_bucket: BTreeMap::new(),
            choose_args: BTreeMap::new(),
            tail: Default::default(),
        }
    }
}

/// `OSDMap::Incremental`.
#[derive(Debug, Clone, PartialEq)]
pub struct Incremental {
    pub fsid: Uuid,
    pub epoch: u32,
    pub modified: UTime,
    pub new_pool_max: i64,
    pub new_flags: i32,
    /// An embedded full map, in lieu of the deltas.
    pub fullmap: Bytes,
    /// A new CRUSH map.
    pub crush: Bytes,
    pub new_max_osd: i32,
    pub new_pools: BTreeMap<i64, PgPool>,
    pub new_pool_names: BTreeMap<i64, String>,
    pub old_pools: BTreeSet<i64>,
    pub new_up_client: BTreeMap<i32, EntityAddrVec>,
    /// XORed onto the previous state (0 means `UP`).
    pub new_state: BTreeMap<i32, u32>,
    pub new_weight: BTreeMap<i32, u32>,
    /// Empty vector = remove.
    pub new_pg_temp: BTreeMap<PgId, Vec<i32>>,
    /// -1 = remove.
    pub new_primary_temp: BTreeMap<PgId, i32>,
    pub new_primary_affinity: BTreeMap<i32, u32>,
    pub new_erasure_code_profiles: BTreeMap<String, BTreeMap<String, String>>,
    pub old_erasure_code_profiles: Vec<String>,
    pub new_pg_upmap: BTreeMap<PgId, Vec<i32>>,
    pub old_pg_upmap: BTreeSet<PgId>,
    pub new_pg_upmap_items: BTreeMap<PgId, Vec<(i32, i32)>>,
    pub old_pg_upmap_items: BTreeSet<PgId>,
    pub new_removed_snaps: BTreeMap<i64, IntervalSet>,
    pub new_purged_snaps: BTreeMap<i64, IntervalSet>,
    pub new_last_up_change: UTime,
    pub new_last_in_change: UTime,
    pub new_pg_upmap_primary: BTreeMap<PgId, i32>,
    pub old_pg_upmap_primary: BTreeSet<PgId>,
    // --- osd-only ---
    pub new_hb_back_up: BTreeMap<i32, EntityAddrVec>,
    pub new_up_thru: BTreeMap<i32, u32>,
    pub new_last_clean_interval: BTreeMap<i32, (u32, u32)>,
    pub new_lost: BTreeMap<i32, u32>,
    pub new_blocklist: Vec<(EntityAddr, UTime)>,
    pub old_blocklist: Vec<EntityAddr>,
    pub new_up_cluster: BTreeMap<i32, EntityAddrVec>,
    pub cluster_snapshot: String,
    pub new_uuid: BTreeMap<i32, Uuid>,
    pub new_xinfo: BTreeMap<i32, OsdXInfo>,
    pub new_hb_front_up: BTreeMap<i32, EntityAddrVec>,
    pub encode_features: u64,
    pub new_nearfull_ratio: f32,
    pub new_full_ratio: f32,
    pub new_backfillfull_ratio: f32,
    pub new_require_min_compat_client: Release,
    pub new_require_osd_release: Release,
    pub new_crush_node_flags: BTreeMap<i32, u32>,
    pub new_device_class_flags: BTreeMap<i32, u32>,
    pub change_stretch_mode: bool,
    pub new_stretch_bucket_count: u32,
    pub new_degraded_stretch_mode: u32,
    pub new_recovering_stretch_mode: u32,
    pub new_stretch_mode_bucket: i32,
    pub stretch_mode_enabled: bool,
    pub new_range_blocklist: Vec<(EntityAddr, UTime)>,
    pub old_range_blocklist: Vec<EntityAddr>,
    /// 0 none, 1 set, 2 clear.
    pub mutate_allow_crimson: u8,
    /// crc of the resulting full map, if the encoding carried one.
    pub full_crc: Option<u32>,
}

impl Incremental {
    /// Decode (`OSDMap::Incremental::decode`), verifying the incremental's own crc.
    pub fn decode(bytes: &Bytes) -> denc::Result<Self> {
        let all = bytes.clone();
        let mut buf = bytes.clone();
        denc::need(&buf, 1, "OSDMap::Incremental")?;
        if buf[0] < 7 {
            return Err(invalid(
                "OSDMap::Incremental",
                format!("classic encoding v{} not supported", buf[0]),
            ));
        }
        let wrapper = decode_start(&mut buf, 8, "OSDMap::Incremental")?;
        let body_len = wrapper.body.len();
        let mut w = wrapper.body;

        let mut st = decode_start(&mut w, 9, "Incremental client data")?;
        let v = st.v;
        let b = &mut st.body;
        let fsid = Uuid::decode(b)?;
        let epoch = u32::decode(b)?;
        let modified = UTime::decode(b)?;
        let new_pool_max = i64::decode(b)?;
        let new_flags = i32::decode(b)?;
        let fullmap = Bytes::decode(b)?;
        let crush = Bytes::decode(b)?;
        let new_max_osd = i32::decode(b)?;
        let new_pools = BTreeMap::<i64, PgPool>::decode(b)?;
        let new_pool_names = BTreeMap::<i64, String>::decode(b)?;
        let old_pools = BTreeSet::<i64>::decode(b)?;
        let new_up_client = BTreeMap::<i32, EntityAddrVec>::decode(b)?;
        let new_state = if v >= 5 {
            BTreeMap::<i32, u32>::decode(b)?
        } else {
            BTreeMap::<i32, u8>::decode(b)?
                .into_iter()
                .map(|(k, v)| (k, u32::from(v)))
                .collect()
        };
        let new_weight = BTreeMap::<i32, u32>::decode(b)?;
        let new_pg_temp = decode_pg_map(b, decode_i32_vec)?;
        let new_primary_temp = decode_pg_map(b, i32::decode)?;
        let new_primary_affinity = if v >= 2 {
            BTreeMap::<i32, u32>::decode(b)?
        } else {
            BTreeMap::new()
        };
        let (new_erasure_code_profiles, old_erasure_code_profiles) = if v >= 3 {
            (
                BTreeMap::<String, BTreeMap<String, String>>::decode(b)?,
                Vec::<String>::decode(b)?,
            )
        } else {
            (BTreeMap::new(), Vec::new())
        };
        let (mut new_pg_upmap, mut old_pg_upmap, mut new_pg_upmap_items, mut old_pg_upmap_items) = (
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeMap::new(),
            BTreeSet::new(),
        );
        if v >= 4 {
            new_pg_upmap = decode_pg_map(b, decode_i32_vec)?;
            old_pg_upmap = BTreeSet::<PgId>::decode(b)?;
            new_pg_upmap_items = decode_pg_map(b, decode_pair_vec)?;
            old_pg_upmap_items = BTreeSet::<PgId>::decode(b)?;
        }
        let (mut new_removed_snaps, mut new_purged_snaps) = (BTreeMap::new(), BTreeMap::new());
        if v >= 6 {
            new_removed_snaps = decode_snap_sets(b)?;
            new_purged_snaps = decode_snap_sets(b)?;
        }
        let (mut new_last_up_change, mut new_last_in_change) = (UTime::default(), UTime::default());
        if v >= 8 {
            new_last_up_change = UTime::decode(b)?;
            new_last_in_change = UTime::decode(b)?;
        }
        let (mut new_pg_upmap_primary, mut old_pg_upmap_primary) =
            (BTreeMap::new(), BTreeSet::new());
        if v >= 9 {
            new_pg_upmap_primary = decode_pg_map(b, i32::decode)?;
            old_pg_upmap_primary = BTreeSet::<PgId>::decode(b)?;
        }

        let mut st = decode_start(&mut w, 12, "Incremental osd data")?;
        let v = st.v;
        let b = &mut st.body;
        let new_hb_back_up = BTreeMap::<i32, EntityAddrVec>::decode(b)?;
        let new_up_thru = BTreeMap::<i32, u32>::decode(b)?;
        let new_last_clean_interval = BTreeMap::<i32, (u32, u32)>::decode(b)?;
        let new_lost = BTreeMap::<i32, u32>::decode(b)?;
        let new_blocklist = decode_addr_time_map(b)?;
        let old_blocklist = Vec::<EntityAddr>::decode(b)?;
        let new_up_cluster = BTreeMap::<i32, EntityAddrVec>::decode(b)?;
        let cluster_snapshot = String::decode(b)?;
        let new_uuid = BTreeMap::<i32, Uuid>::decode(b)?;
        let new_xinfo = BTreeMap::<i32, OsdXInfo>::decode(b)?;
        let new_hb_front_up = BTreeMap::<i32, EntityAddrVec>::decode(b)?;
        let encode_features = if v >= 2 {
            u64::decode(b)?
        } else {
            denc::features::PGID64 | denc::features::OSDMAP_ENC
        };
        let (mut new_nearfull_ratio, mut new_full_ratio, mut new_backfillfull_ratio) =
            (-1.0f32, -1.0f32, -1.0f32);
        if v >= 3 {
            new_nearfull_ratio = decode_f32(b)?;
            new_full_ratio = decode_f32(b)?;
        }
        if v >= 4 {
            new_backfillfull_ratio = decode_f32(b)?;
        }
        if v == 5 {
            let _r = String::decode(b)?;
        }
        let (mut new_require_min_compat_client, mut new_require_osd_release) =
            (Release::UNSET, Release::UNSET);
        if v >= 6 {
            new_require_min_compat_client = Release(u8::decode(b)?);
            new_require_osd_release = Release(u8::decode(b)?);
        }
        let new_crush_node_flags = if v >= 8 {
            BTreeMap::<i32, u32>::decode(b)?
        } else {
            BTreeMap::new()
        };
        let new_device_class_flags = if v >= 9 {
            BTreeMap::<i32, u32>::decode(b)?
        } else {
            BTreeMap::new()
        };
        let mut change_stretch_mode = false;
        let (
            mut new_stretch_bucket_count,
            mut new_degraded_stretch_mode,
            mut new_recovering_stretch_mode,
        ) = (0, 0, 0);
        let mut new_stretch_mode_bucket = 0;
        let mut stretch_mode_enabled = false;
        if v >= 10 {
            change_stretch_mode = bool::decode(b)?;
            new_stretch_bucket_count = u32::decode(b)?;
            new_degraded_stretch_mode = u32::decode(b)?;
            new_recovering_stretch_mode = u32::decode(b)?;
            new_stretch_mode_bucket = i32::decode(b)?;
            stretch_mode_enabled = bool::decode(b)?;
        }
        let (mut new_range_blocklist, mut old_range_blocklist) = (Vec::new(), Vec::new());
        if v >= 11 {
            new_range_blocklist = decode_addr_time_map(b)?;
            old_range_blocklist = Vec::<EntityAddr>::decode(b)?;
        }
        let mutate_allow_crimson = if v >= 12 { u8::decode(b)? } else { 0 };

        let mut full_crc = None;
        if wrapper.v >= 8 {
            let crc_offset = 6 + (body_len - w.remaining());
            let inc_crc = u32::decode(&mut w)?;
            let tail_offset = crc_offset + 4;
            full_crc = Some(u32::decode(&mut w)?);
            check_crc(
                &all,
                crc_offset,
                tail_offset,
                6 + body_len,
                inc_crc,
                "OSDMap::Incremental",
            )?;
        }

        Ok(Incremental {
            fsid,
            epoch,
            modified,
            new_pool_max,
            new_flags,
            fullmap,
            crush,
            new_max_osd,
            new_pools,
            new_pool_names,
            old_pools,
            new_up_client,
            new_state,
            new_weight,
            new_pg_temp,
            new_primary_temp,
            new_primary_affinity,
            new_erasure_code_profiles,
            old_erasure_code_profiles,
            new_pg_upmap,
            old_pg_upmap,
            new_pg_upmap_items,
            old_pg_upmap_items,
            new_removed_snaps,
            new_purged_snaps,
            new_last_up_change,
            new_last_in_change,
            new_pg_upmap_primary,
            old_pg_upmap_primary,
            new_hb_back_up,
            new_up_thru,
            new_last_clean_interval,
            new_lost,
            new_blocklist,
            old_blocklist,
            new_up_cluster,
            cluster_snapshot,
            new_uuid,
            new_xinfo,
            new_hb_front_up,
            encode_features,
            new_nearfull_ratio,
            new_full_ratio,
            new_backfillfull_ratio,
            new_require_min_compat_client,
            new_require_osd_release,
            new_crush_node_flags,
            new_device_class_flags,
            change_stretch_mode,
            new_stretch_bucket_count,
            new_degraded_stretch_mode,
            new_recovering_stretch_mode,
            new_stretch_mode_bucket,
            stretch_mode_enabled,
            new_range_blocklist,
            old_range_blocklist,
            mutate_allow_crimson,
            full_crc,
        })
    }

    /// Addresses newly blocklisted by this incremental (`emit_blocklist_events(inc)`): the
    /// `new_blocklist` entries and the `new_range_blocklist` entries.
    pub fn new_blocklist_addrs(&self) -> impl Iterator<Item = &EntityAddr> {
        self.new_blocklist
            .iter()
            .map(|(a, _)| a)
            .chain(self.new_range_blocklist.iter().map(|(a, _)| a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_set_ops() {
        let mut m = IntervalSet::new();
        interval_insert(&mut m, 5, 2); // [5,7)
        interval_insert(&mut m, 1, 2); // [1,3)
        interval_insert(&mut m, 3, 2); // touches both -> [1,7)
        assert_eq!(m.into_iter().collect::<Vec<_>>(), vec![(1, 6)]);
        let mut m: IntervalSet = [(1, 10)].into_iter().collect();
        interval_subtract(&mut m, &[(3, 2)].into_iter().collect());
        assert_eq!(
            m.clone().into_iter().collect::<Vec<_>>(),
            vec![(1, 2), (5, 6)]
        );
        interval_subtract(&mut m, &[(1, 2), (5, 6)].into_iter().collect());
        assert!(m.is_empty());
    }

    #[test]
    fn range_blocklist_matching() {
        use std::net::SocketAddr;
        let range = EntityAddr::new(
            AddrType::ANY,
            "10.1.0.0:0".parse::<SocketAddr>().unwrap(),
            16,
        );
        let r = RangeBits::parse(&range);
        let inside = EntityAddr::new(
            AddrType::ANY,
            "10.1.200.3:6800".parse::<SocketAddr>().unwrap(),
            77,
        );
        let outside = EntityAddr::new(
            AddrType::ANY,
            "10.2.0.3:6800".parse::<SocketAddr>().unwrap(),
            77,
        );
        assert!(r.matches(&inside));
        assert!(!r.matches(&outside));
    }
}
