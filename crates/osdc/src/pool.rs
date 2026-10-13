//! `pg_pool_t` (`src/osd/osd_types.{h,cc}`): a RADOS pool as the OSDMap describes it.
//!
//! Wire format (`pg_pool_t::encode`, current form `ENCODE_START(v, 5)` with v 29..33; the
//! decoder follows `pg_pool_t::decode` field by field and skips any newer tail):
//!
//! ```text
//! u8 type, u8 size, u8 crush_rule, u8 object_hash, u32 pg_num, u32 pgp_num,
//! u32 lpg_num (0), u32 lpgp_num (0), u32 last_change, u64 snap_seq, u32 snap_epoch,
//! map<u64, pool_snap_info_t> snaps, interval_set<u64> removed_snaps, u64 auid,
//! u64 flags, u32 crash_replay_interval, u8 min_size, u64 quota_max_bytes,
//! u64 quota_max_objects, set<u64> tiers, i64 tier_of, u8 cache_mode, i64 read_tier,
//! i64 write_tier, map<string,string> properties, HitSet::Params hit_set_params,
//! u32 hit_set_period, u32 hit_set_count, u32 stripe_width, u64 target_max_bytes,
//! u64 target_max_objects, u32 cache_target_dirty_ratio_micro,
//! u32 cache_target_full_ratio_micro, u32 cache_min_flush_age, u32 cache_min_evict_age,
//! string erasure_code_profile, u32 last_force_op_resend_preluminous,
//! u32 min_read_recency_for_promote, u64 expected_num_objects,
//! u32 cache_target_dirty_high_ratio_micro (v19), u32 min_write_recency_for_promote (v20),
//! bool use_gmt_hitset (v21), bool fast_read (v22), u32 hit_set_grade_decay_rate,
//! u32 hit_set_search_last_n (v23), pool_opts_t opts (v24),
//! u32 last_force_op_resend_prenautilus (v25), map<string,map<string,string>>
//! application_metadata (v26), utime_t create_time (v27), u32 pg_num_target,
//! u32 pgp_num_target, u32 pg_num_pending, u32 0, u32 0, u32 last_force_op_resend,
//! u8 pg_autoscale_mode (v28), pg_merge_meta_t last_pg_merge_meta (v29),
//! [v30: 4 × u32 peering_crush_*], optional<tuple<u32×4>> peering crush data (v31),
//! shard_id_set nonprimary_shards (v32: 2 varints), vector<i8> shard_mapping,
//! optional<u8> ec_data_shard_count, optional<u8> ec_coding_shard_count (v33)
//! ```

use std::collections::{BTreeMap, BTreeSet};

use denc::{Bytes, Decode, Result, UTime, decode_start};

use crate::hash::{CRUSH_HASH_RJENKINS1, cbits, ceph_stable_mod, ceph_str_hash, crush_hash32_2};
use crate::types::{EVersion, PgId};
use crate::util::{decode_start_legacy, decode_varint};

/// `pg_pool_t::TYPE_REPLICATED`.
pub const POOL_TYPE_REPLICATED: u8 = 1;
/// `pg_pool_t::TYPE_ERASURE`.
pub const POOL_TYPE_ERASURE: u8 = 3;

/// `pg_pool_t::FLAG_*`.
pub mod pool_flags {
    pub const HASHPSPOOL: u64 = 1 << 0;
    pub const FULL: u64 = 1 << 1;
    pub const EC_OVERWRITES: u64 = 1 << 2;
    pub const INCOMPLETE_CLONES: u64 = 1 << 3;
    pub const NODELETE: u64 = 1 << 4;
    pub const NOPGCHANGE: u64 = 1 << 5;
    pub const NOSIZECHANGE: u64 = 1 << 6;
    pub const WRITE_FADVISE_DONTNEED: u64 = 1 << 7;
    pub const NOSCRUB: u64 = 1 << 8;
    pub const NODEEP_SCRUB: u64 = 1 << 9;
    pub const FULL_QUOTA: u64 = 1 << 10;
    pub const NEARFULL: u64 = 1 << 11;
    pub const BACKFILLFULL: u64 = 1 << 12;
    pub const SELFMANAGED_SNAPS: u64 = 1 << 13;
    pub const POOL_SNAPS: u64 = 1 << 14;
    pub const CREATING: u64 = 1 << 15;
    pub const EIO: u64 = 1 << 16;
    pub const BULK: u64 = 1 << 17;
    pub const CRIMSON: u64 = 1 << 18;
    pub const EC_OPTIMIZATIONS: u64 = 1 << 19;
    pub const CLIENT_SPLIT_READS: u64 = 1 << 20;
    pub const OMAP: u64 = 1 << 21;
}

/// `pool_snap_info_t`: `ENCODE_START(2, 2)`, `u64 snapid`, `utime_t stamp`, `string name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolSnapInfo {
    pub snapid: u64,
    pub stamp: UTime,
    pub name: String,
}

impl Decode for PoolSnapInfo {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut st = decode_start_legacy(buf, 2, 2, "pool_snap_info_t")?;
        let b = &mut st.body;
        Ok(PoolSnapInfo {
            snapid: u64::decode(b)?,
            stamp: UTime::decode(b)?,
            name: String::decode(b)?,
        })
    }
}

/// `pg_merge_meta_t`: `ENCODE_START(1, 1)`, `pg_t source_pgid`, `u32 ready_epoch`,
/// `u32 last_epoch_started`, `u32 last_epoch_clean`, `eversion_t source_version`,
/// `eversion_t target_version`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PgMergeMeta {
    pub source_pgid: PgId,
    pub ready_epoch: u32,
    pub last_epoch_started: u32,
    pub last_epoch_clean: u32,
    pub source_version: EVersion,
    pub target_version: EVersion,
}

impl Decode for PgMergeMeta {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut st = decode_start(buf, 1, "pg_merge_meta_t")?;
        let b = &mut st.body;
        Ok(PgMergeMeta {
            source_pgid: PgId::decode(b)?,
            ready_epoch: u32::decode(b)?,
            last_epoch_started: u32::decode(b)?,
            last_epoch_clean: u32::decode(b)?,
            source_version: EVersion::decode(b)?,
            target_version: EVersion::decode(b)?,
        })
    }
}

/// A decoded `pg_pool_t`. Opaque sub-structures the client never interprets (`HitSet::Params`,
/// `pool_opts_t`) are kept as their raw framed encodings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgPool {
    pub pool_type: u8,
    pub size: u8,
    pub crush_rule: u8,
    pub object_hash: u8,
    pub pg_num: u32,
    pub pgp_num: u32,
    pub last_change: u32,
    pub snap_seq: u64,
    pub snap_epoch: u32,
    pub snaps: BTreeMap<u64, PoolSnapInfo>,
    /// `interval_set<snapid_t>`: start → length.
    pub removed_snaps: BTreeMap<u64, u64>,
    pub auid: u64,
    pub flags: u64,
    pub min_size: u8,
    pub quota_max_bytes: u64,
    pub quota_max_objects: u64,
    pub tiers: BTreeSet<u64>,
    pub tier_of: i64,
    pub cache_mode: u8,
    pub read_tier: i64,
    pub write_tier: i64,
    pub properties: BTreeMap<String, String>,
    pub hit_set_params: Bytes,
    pub hit_set_period: u32,
    pub hit_set_count: u32,
    pub stripe_width: u32,
    pub target_max_bytes: u64,
    pub target_max_objects: u64,
    pub cache_target_dirty_ratio_micro: u32,
    pub cache_target_full_ratio_micro: u32,
    pub cache_min_flush_age: u32,
    pub cache_min_evict_age: u32,
    pub erasure_code_profile: String,
    pub last_force_op_resend_preluminous: u32,
    pub min_read_recency_for_promote: u32,
    pub expected_num_objects: u64,
    pub cache_target_dirty_high_ratio_micro: u32,
    pub min_write_recency_for_promote: u32,
    pub use_gmt_hitset: bool,
    pub fast_read: bool,
    pub hit_set_grade_decay_rate: u32,
    pub hit_set_search_last_n: u32,
    pub opts: Bytes,
    pub last_force_op_resend_prenautilus: u32,
    pub application_metadata: BTreeMap<String, BTreeMap<String, String>>,
    pub create_time: UTime,
    pub pg_num_target: u32,
    pub pgp_num_target: u32,
    pub pg_num_pending: u32,
    pub last_force_op_resend: u32,
    pub pg_autoscale_mode: u8,
    pub last_pg_merge_meta: PgMergeMeta,
    pub peering_crush_bucket_count: u32,
    pub peering_crush_bucket_target: u32,
    pub peering_crush_bucket_barrier: u32,
    pub peering_crush_mandatory_member: i32,
    /// `shard_id_set` (128-bit set) words.
    pub nonprimary_shards: [u64; 2],
    pub shard_mapping: Vec<i8>,
    pub ec_data_shard_count: Option<u8>,
    pub ec_coding_shard_count: Option<u8>,
    /// `calc_pg_masks()`.
    pub pg_num_mask: u32,
    pub pgp_num_mask: u32,
}

/// `(1ULL << cbits(n - 1)) - 1` with `n - 1` wrapping for `n == 0`.
fn pg_mask(n: u32) -> u32 {
    ((1u64 << cbits(n.wrapping_sub(1))) - 1) as u32
}

impl PgPool {
    pub fn is_replicated(&self) -> bool {
        self.pool_type == POOL_TYPE_REPLICATED
    }

    pub fn is_erasure(&self) -> bool {
        self.pool_type == POOL_TYPE_ERASURE
    }

    /// `can_shift_osds()`: replicated pools compact their OSD lists; EC pools keep positions.
    pub fn can_shift_osds(&self) -> bool {
        self.is_replicated()
    }

    pub fn has_flag(&self, f: u64) -> bool {
        self.flags & f != 0
    }

    /// `allows_ecoptimizations()`.
    pub fn allows_ecoptimizations(&self) -> bool {
        self.has_flag(pool_flags::EC_OPTIMIZATIONS)
    }

    /// `is_nonprimary_shard(shard)`.
    pub fn is_nonprimary_shard(&self, shard: i8) -> bool {
        if shard < 0 {
            return false;
        }
        let s = shard as usize;
        self.nonprimary_shards[s / 64] & (1u64 << (s % 64)) != 0
    }

    pub fn nonprimary_shard_count(&self) -> usize {
        (self.nonprimary_shards[0].count_ones() + self.nonprimary_shards[1].count_ones()) as usize
    }

    pub fn has_read_tier(&self) -> bool {
        self.read_tier >= 0
    }

    pub fn has_write_tier(&self) -> bool {
        self.write_tier >= 0
    }

    /// `is_stretch_pool()`.
    pub fn is_stretch_pool(&self) -> bool {
        self.peering_crush_bucket_count != 0
    }

    /// `supports_omap()`: replicated pools, or pools flagged `FLAG_OMAP`.
    pub fn supports_omap(&self) -> bool {
        self.is_replicated() || self.has_flag(pool_flags::OMAP)
    }

    /// `hash_key(key, ns)`: `ceph_str_hash(object_hash, ns + '\x1f' + key)` (no separator when
    /// the namespace is empty).
    pub fn hash_key(&self, key: &[u8], ns: &[u8]) -> u32 {
        if ns.is_empty() {
            return ceph_str_hash(self.object_hash, key);
        }
        let mut buf = Vec::with_capacity(ns.len() + 1 + key.len());
        buf.extend_from_slice(ns);
        buf.push(0x1f);
        buf.extend_from_slice(key);
        ceph_str_hash(self.object_hash, &buf)
    }

    /// `raw_hash_to_pg(v)`.
    pub fn raw_hash_to_pg(&self, v: u32) -> u32 {
        ceph_stable_mod(v, self.pg_num, self.pg_num_mask)
    }

    /// `raw_pg_to_pg`: fold a full-precision placement seed into `[0, pg_num)`.
    pub fn raw_pg_to_pg(&self, pg: PgId) -> PgId {
        PgId::new(
            pg.pool,
            ceph_stable_mod(pg.seed, self.pg_num, self.pg_num_mask),
        )
    }

    /// `raw_pg_to_pps`: the CRUSH input for a pg. With `FLAG_HASHPSPOOL` the pool id is hashed
    /// in; otherwise (legacy) it is added.
    pub fn raw_pg_to_pps(&self, pg: PgId) -> u32 {
        let ps = ceph_stable_mod(pg.seed, self.pgp_num, self.pgp_num_mask);
        if self.flags & pool_flags::HASHPSPOOL != 0 {
            crush_hash32_2(CRUSH_HASH_RJENKINS1, ps, pg.pool as u32)
        } else {
            ps.wrapping_add(pg.pool as u32)
        }
    }

    /// `get_snap_context()`: the pool snapshots (pool-snap mode), newest first.
    pub fn snap_context(&self) -> crate::types::SnapContext {
        crate::types::SnapContext {
            seq: self.snap_seq,
            snaps: self.snaps.keys().rev().copied().collect(),
        }
    }

    /// `calc_pg_masks()`.
    pub fn calc_pg_masks(&mut self) {
        self.pg_num_mask = pg_mask(self.pg_num);
        self.pgp_num_mask = pg_mask(self.pgp_num);
    }
}

/// Raw framed sub-structure: returns the whole `ENCODE_START` frame (header included).
fn take_frame(buf: &mut Bytes, what: &'static str) -> Result<Bytes> {
    denc::need(buf, 6, what)?;
    let len = u32::from_le_bytes([buf[2], buf[3], buf[4], buf[5]]) as usize;
    denc::take(buf, 6 + len, what)
}

impl Decode for PgPool {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut st = decode_start_legacy(buf, 33, 5, "pg_pool_t")?;
        let v = st.v;
        let b = &mut st.body;
        let pool_type = u8::decode(b)?;
        let size = u8::decode(b)?;
        let crush_rule = u8::decode(b)?;
        let object_hash = u8::decode(b)?;
        let pg_num = u32::decode(b)?;
        let pgp_num = u32::decode(b)?;
        let _lpg_num = u32::decode(b)?;
        let _lpgp_num = u32::decode(b)?;
        let last_change = u32::decode(b)?;
        let snap_seq = u64::decode(b)?;
        let snap_epoch = u32::decode(b)?;
        let snaps = BTreeMap::<u64, PoolSnapInfo>::decode(b)?;
        let removed_snaps = BTreeMap::<u64, u64>::decode(b)?;
        let auid = u64::decode(b)?;
        let mut flags = u64::decode(b)?;
        let _crash_replay_interval = u32::decode(b)?;
        // upgrade path for selfmanaged vs pool snaps
        if snap_seq > 0 && flags & (pool_flags::SELFMANAGED_SNAPS | pool_flags::POOL_SNAPS) == 0 {
            if !removed_snaps.is_empty() {
                flags |= pool_flags::SELFMANAGED_SNAPS;
            } else {
                flags |= pool_flags::POOL_SNAPS;
            }
        }
        let min_size = if v >= 7 {
            u8::decode(b)?
        } else {
            size - size / 2
        };
        let (quota_max_bytes, quota_max_objects) = if v >= 8 {
            (u64::decode(b)?, u64::decode(b)?)
        } else {
            (0, 0)
        };
        let (tiers, tier_of, cache_mode, read_tier, write_tier) = if v >= 9 {
            (
                BTreeSet::<u64>::decode(b)?,
                i64::decode(b)?,
                u8::decode(b)?,
                i64::decode(b)?,
                i64::decode(b)?,
            )
        } else {
            (BTreeSet::new(), -1, 0, -1, -1)
        };
        let properties = if v >= 10 {
            BTreeMap::<String, String>::decode(b)?
        } else {
            BTreeMap::new()
        };
        let (hit_set_params, hit_set_period, hit_set_count) = if v >= 11 {
            (
                take_frame(b, "HitSet::Params")?,
                u32::decode(b)?,
                u32::decode(b)?,
            )
        } else {
            (Bytes::new(), 0, 0)
        };
        let stripe_width = if v >= 12 { u32::decode(b)? } else { 0 };
        let (
            target_max_bytes,
            target_max_objects,
            cache_target_dirty_ratio_micro,
            cache_target_full_ratio_micro,
            cache_min_flush_age,
            cache_min_evict_age,
        ) = if v >= 13 {
            (
                u64::decode(b)?,
                u64::decode(b)?,
                u32::decode(b)?,
                u32::decode(b)?,
                u32::decode(b)?,
                u32::decode(b)?,
            )
        } else {
            (0, 0, 0, 0, 0, 0)
        };
        let erasure_code_profile = if v >= 14 {
            String::decode(b)?
        } else {
            String::new()
        };
        let last_force_op_resend_preluminous = if v >= 15 { u32::decode(b)? } else { 0 };
        let min_read_recency_for_promote = if v >= 16 { u32::decode(b)? } else { 1 };
        let expected_num_objects = if v >= 17 { u64::decode(b)? } else { 0 };
        let cache_target_dirty_high_ratio_micro = if v >= 19 {
            u32::decode(b)?
        } else {
            cache_target_dirty_ratio_micro
        };
        let min_write_recency_for_promote = if v >= 20 { u32::decode(b)? } else { 1 };
        let use_gmt_hitset = if v >= 21 { bool::decode(b)? } else { false };
        let fast_read = if v >= 22 { bool::decode(b)? } else { false };
        let (hit_set_grade_decay_rate, hit_set_search_last_n) = if v >= 23 {
            (u32::decode(b)?, u32::decode(b)?)
        } else {
            (0, 1)
        };
        let opts = if v >= 24 {
            take_frame(b, "pool_opts_t")?
        } else {
            Bytes::new()
        };
        let last_force_op_resend_prenautilus = if v >= 25 {
            u32::decode(b)?
        } else {
            last_force_op_resend_preluminous
        };
        let application_metadata = if v >= 26 {
            BTreeMap::<String, BTreeMap<String, String>>::decode(b)?
        } else {
            BTreeMap::new()
        };
        let create_time = if v >= 27 {
            UTime::decode(b)?
        } else {
            UTime::default()
        };
        let (
            pg_num_target,
            pgp_num_target,
            pg_num_pending,
            last_force_op_resend,
            pg_autoscale_mode,
            last_pg_merge_meta,
        );
        if v >= 28 {
            pg_num_target = u32::decode(b)?;
            pgp_num_target = u32::decode(b)?;
            pg_num_pending = u32::decode(b)?;
            let old_merge_last_epoch_started = u32::decode(b)?;
            let old_merge_last_epoch_clean = u32::decode(b)?;
            last_force_op_resend = u32::decode(b)?;
            pg_autoscale_mode = u8::decode(b)?;
            last_pg_merge_meta = if v >= 29 {
                PgMergeMeta::decode(b)?
            } else {
                PgMergeMeta {
                    last_epoch_clean: old_merge_last_epoch_clean,
                    last_epoch_started: old_merge_last_epoch_started,
                    ..Default::default()
                }
            };
        } else {
            pg_num_target = pg_num;
            pgp_num_target = pgp_num;
            pg_num_pending = pg_num;
            last_force_op_resend = last_force_op_resend_prenautilus;
            pg_autoscale_mode = 1; // WARN
            last_pg_merge_meta = PgMergeMeta::default();
        }
        let mut peering = (0u32, 0u32, 0u32, 0x7fff_ffffi32);
        if v == 30 {
            peering = (
                u32::decode(b)?,
                u32::decode(b)?,
                u32::decode(b)?,
                i32::decode(b)?,
            );
        }
        if v >= 31 && bool::decode(b)? {
            peering = (
                u32::decode(b)?,
                u32::decode(b)?,
                u32::decode(b)?,
                u32::decode(b)? as i32,
            );
        }
        let nonprimary_shards = if v >= 32 {
            [decode_varint(b)?, decode_varint(b)?]
        } else {
            [0, 0]
        };
        let (shard_mapping, ec_data_shard_count, ec_coding_shard_count) = if v >= 33 {
            (
                Vec::<i8>::decode(b)?,
                Option::<u8>::decode(b)?,
                Option::<u8>::decode(b)?,
            )
        } else {
            (Vec::new(), None, None)
        };
        let mut pool = PgPool {
            pool_type,
            size,
            crush_rule,
            object_hash,
            pg_num,
            pgp_num,
            last_change,
            snap_seq,
            snap_epoch,
            snaps,
            removed_snaps,
            auid,
            flags,
            min_size,
            quota_max_bytes,
            quota_max_objects,
            tiers,
            tier_of,
            cache_mode,
            read_tier,
            write_tier,
            properties,
            hit_set_params,
            hit_set_period,
            hit_set_count,
            stripe_width,
            target_max_bytes,
            target_max_objects,
            cache_target_dirty_ratio_micro,
            cache_target_full_ratio_micro,
            cache_min_flush_age,
            cache_min_evict_age,
            erasure_code_profile,
            last_force_op_resend_preluminous,
            min_read_recency_for_promote,
            expected_num_objects,
            cache_target_dirty_high_ratio_micro,
            min_write_recency_for_promote,
            use_gmt_hitset,
            fast_read,
            hit_set_grade_decay_rate,
            hit_set_search_last_n,
            opts,
            last_force_op_resend_prenautilus,
            application_metadata,
            create_time,
            pg_num_target,
            pgp_num_target,
            pg_num_pending,
            last_force_op_resend,
            pg_autoscale_mode,
            last_pg_merge_meta,
            peering_crush_bucket_count: peering.0,
            peering_crush_bucket_target: peering.1,
            peering_crush_bucket_barrier: peering.2,
            peering_crush_mandatory_member: peering.3,
            nonprimary_shards,
            shard_mapping,
            ec_data_shard_count,
            ec_coding_shard_count,
            pg_num_mask: 0,
            pgp_num_mask: 0,
        };
        pool.calc_pg_masks();
        Ok(pool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks() {
        assert_eq!(pg_mask(8), 7);
        assert_eq!(pg_mask(12), 15);
        assert_eq!(pg_mask(1), 0);
        assert_eq!(pg_mask(0), u32::MAX);
    }
}
