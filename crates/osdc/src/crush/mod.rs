//! The CRUSH map and `crush_do_rule`.
//!
//! Wire format: `CrushWrapper::encode`/`decode` (`src/crush/CrushWrapper.cc`):
//!
//! ```text
//! u32 magic (0x00010000)  i32 max_buckets  u32 max_rules  i32 max_devices
//! max_buckets × bucket:   u32 alg (0 = no bucket), then i32 id, u16 type, u8 alg, u8 hash,
//!                         u32 weight, u32 size, size × i32 item, then per alg:
//!                           uniform: u32 item_weight
//!                           list:    size × (u32 item_weight, u32 sum_weight)
//!                           tree:    u8 num_nodes, num_nodes × u32 node_weight
//!                           straw:   size × (u32 item_weight, u32 straw)
//!                           straw2:  size × u32 item_weight
//! max_rules × rule:       u32 present, then u32 len, u8 ruleset (= rule id), u8 type,
//!                         u8 min_size, u8 max_size, len × (u32 op, i32 arg1, i32 arg2)
//! map<i32,string> type_map, name_map, rule_name_map   (keys may be 32 or 64 bit, see below)
//! tunables (each group optional at the end of the buffer):
//!   u32 choose_local_tries, u32 choose_local_fallback_tries, u32 choose_total_tries
//!   u32 chooseleaf_descend_once | u8 chooseleaf_vary_r | u8 straw_calc_version
//!   u32 allowed_bucket_algs | u8 chooseleaf_stable
//! map<i32,i32> class_map, map<i32,string> class_name, map<i32,map<i32,i32>> class_bucket
//! choose_args: u32 n × (i64 index, u32 n_args × (u32 bucket_index, u32 positions,
//!              positions × (u32 n, n × u32 weight), u32 n_ids, n_ids × i32 id))
//! u32 msr_descents, u32 msr_collision_tries
//! ```
//!
//! After decoding, `finalize()` recomputes `max_devices` from the bucket items (and the highest
//! named device), and `update_choose_args()` drops/fixes choose_args that do not match the
//! buckets; both are mirrored here because they change placement.

mod ln_table;
pub mod mapper;

use std::collections::BTreeMap;

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, DecodeError, Encode, Result, features};

pub use mapper::CRUSH_ITEM_NONE;

/// `CRUSH_MAGIC`.
pub const CRUSH_MAGIC: u32 = 0x0001_0000;

/// Bucket algorithms (`enum crush_algorithm`).
pub const CRUSH_BUCKET_UNIFORM: u8 = 1;
pub const CRUSH_BUCKET_LIST: u8 = 2;
pub const CRUSH_BUCKET_TREE: u8 = 3;
pub const CRUSH_BUCKET_STRAW: u8 = 4;
pub const CRUSH_BUCKET_STRAW2: u8 = 5;

/// Rule step opcodes (`enum crush_opcodes`).
pub const CRUSH_RULE_NOOP: u32 = 0;
pub const CRUSH_RULE_TAKE: u32 = 1;
pub const CRUSH_RULE_CHOOSE_FIRSTN: u32 = 2;
pub const CRUSH_RULE_CHOOSE_INDEP: u32 = 3;
pub const CRUSH_RULE_EMIT: u32 = 4;
pub const CRUSH_RULE_CHOOSELEAF_FIRSTN: u32 = 6;
pub const CRUSH_RULE_CHOOSELEAF_INDEP: u32 = 7;
pub const CRUSH_RULE_SET_CHOOSE_TRIES: u32 = 8;
pub const CRUSH_RULE_SET_CHOOSELEAF_TRIES: u32 = 9;
pub const CRUSH_RULE_SET_CHOOSE_LOCAL_TRIES: u32 = 10;
pub const CRUSH_RULE_SET_CHOOSE_LOCAL_FALLBACK_TRIES: u32 = 11;
pub const CRUSH_RULE_SET_CHOOSELEAF_VARY_R: u32 = 12;
pub const CRUSH_RULE_SET_CHOOSELEAF_STABLE: u32 = 13;
pub const CRUSH_RULE_SET_MSR_DESCENTS: u32 = 14;
pub const CRUSH_RULE_SET_MSR_COLLISION_TRIES: u32 = 15;
pub const CRUSH_RULE_CHOOSE_MSR: u32 = 16;

/// Rule types (`enum crush_rule_type`).
pub const CRUSH_RULE_TYPE_REPLICATED: u8 = 1;
pub const CRUSH_RULE_TYPE_ERASURE: u8 = 3;
pub const CRUSH_RULE_TYPE_MSR_FIRSTN: u8 = 4;
pub const CRUSH_RULE_TYPE_MSR_INDEP: u8 = 5;

/// `CrushWrapper::DEFAULT_CHOOSE_ARGS`.
pub const DEFAULT_CHOOSE_ARGS: i64 = -1;

/// Algorithm-specific bucket data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BucketData {
    Uniform {
        item_weight: u32,
    },
    List {
        item_weights: Vec<u32>,
        sum_weights: Vec<u32>,
    },
    Tree {
        num_nodes: u8,
        node_weights: Vec<u32>,
    },
    Straw {
        item_weights: Vec<u32>,
        straws: Vec<u32>,
    },
    Straw2 {
        item_weights: Vec<u32>,
    },
}

/// `struct crush_bucket` and its algorithm extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bucket {
    /// `< 0`; the bucket lives at index `-1 - id`.
    pub id: i32,
    pub bucket_type: u16,
    pub alg: u8,
    pub hash: u8,
    /// 16.16 fixed point.
    pub weight: u32,
    pub items: Vec<i32>,
    pub data: BucketData,
}

impl Bucket {
    pub fn size(&self) -> u32 {
        self.items.len() as u32
    }

    /// `crush_get_bucket_item_weight`.
    pub fn item_weight(&self, pos: usize) -> u32 {
        if pos >= self.items.len() {
            return 0;
        }
        match &self.data {
            BucketData::Uniform { item_weight } => *item_weight,
            BucketData::List { item_weights, .. } => item_weights[pos],
            BucketData::Tree { node_weights, .. } => {
                let n = ((pos + 1) << 1) - 1; // crush_calc_tree_node
                node_weights.get(n).copied().unwrap_or(0)
            }
            BucketData::Straw { item_weights, .. } => item_weights[pos],
            BucketData::Straw2 { item_weights } => item_weights[pos],
        }
    }
}

/// `struct crush_rule_step`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleStep {
    pub op: u32,
    pub arg1: i32,
    pub arg2: i32,
}

/// `struct crush_rule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub rule_type: u8,
    /// `deprecated_min_size` / `deprecated_max_size`, carried for re-encoding.
    pub min_size: u8,
    pub max_size: u8,
    pub steps: Vec<RuleStep>,
}

/// `struct crush_choose_arg`: replacement ids and per-position weight sets for one straw2 bucket.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChooseArg {
    /// Empty = use the bucket's items.
    pub ids: Vec<i32>,
    /// `weight_set[position][item]`; empty = use the bucket's item weights.
    pub weight_set: Vec<Vec<u32>>,
}

/// Tunables (`struct crush_map` fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tunables {
    pub choose_local_tries: u32,
    pub choose_local_fallback_tries: u32,
    pub choose_total_tries: u32,
    pub chooseleaf_descend_once: u32,
    pub chooseleaf_vary_r: u8,
    pub chooseleaf_stable: u8,
    pub straw_calc_version: u8,
    pub allowed_bucket_algs: u32,
    pub msr_descents: u32,
    pub msr_collision_tries: u32,
}

/// `CRUSH_LEGACY_ALLOWED_BUCKET_ALGS`.
const LEGACY_ALLOWED_BUCKET_ALGS: u32 =
    (1 << CRUSH_BUCKET_UNIFORM) | (1 << CRUSH_BUCKET_LIST) | (1 << CRUSH_BUCKET_STRAW);

impl Tunables {
    /// `set_tunables_legacy()` (argonaut + `straw_calc_version` 0, default MSR tunables): what
    /// a decoder assumes for groups missing at the end of the encoding.
    pub fn legacy() -> Self {
        Tunables {
            choose_local_tries: 2,
            choose_local_fallback_tries: 5,
            choose_total_tries: 19,
            chooseleaf_descend_once: 0,
            chooseleaf_vary_r: 0,
            chooseleaf_stable: 0,
            straw_calc_version: 0,
            allowed_bucket_algs: LEGACY_ALLOWED_BUCKET_ALGS,
            msr_descents: 100,
            msr_collision_tries: 100,
        }
    }

    /// `set_tunables_optimal()` (jewel, `straw_calc_version` 1).
    pub fn optimal() -> Self {
        Tunables {
            choose_local_tries: 0,
            choose_local_fallback_tries: 0,
            choose_total_tries: 50,
            chooseleaf_descend_once: 1,
            chooseleaf_vary_r: 1,
            chooseleaf_stable: 1,
            straw_calc_version: 1,
            allowed_bucket_algs: (1 << CRUSH_BUCKET_UNIFORM)
                | (1 << CRUSH_BUCKET_LIST)
                | (1 << CRUSH_BUCKET_STRAW)
                | (1 << CRUSH_BUCKET_STRAW2),
            msr_descents: 100,
            msr_collision_tries: 100,
        }
    }
}

/// A decoded CRUSH map (`CrushWrapper`). Equality ignores encoding artifacts (which optional
/// tail groups were present, the encoded `max_devices`).
#[derive(Debug, Clone)]
pub struct CrushMap {
    /// `buckets[-1 - id]`; `None` for removed buckets. Length = `max_buckets`.
    pub buckets: Vec<Option<Bucket>>,
    /// `rules[ruleno]`; length = `max_rules`.
    pub rules: Vec<Option<Rule>>,
    /// `max_devices` as `finalize()` computes it.
    pub max_devices: i32,
    /// `max_devices` as encoded (kept for re-encoding).
    pub encoded_max_devices: i32,
    pub tunables: Tunables,
    pub type_map: BTreeMap<i32, String>,
    pub name_map: BTreeMap<i32, String>,
    pub rule_name_map: BTreeMap<i32, String>,
    /// device/bucket id → class id.
    pub class_map: BTreeMap<i32, i32>,
    /// class id → class name.
    pub class_name: BTreeMap<i32, String>,
    /// bucket id → (class id → shadow bucket id).
    pub class_bucket: BTreeMap<i32, BTreeMap<i32, i32>>,
    /// choose_args index (pool id or `DEFAULT_CHOOSE_ARGS`) → one arg per bucket index.
    pub choose_args: BTreeMap<i64, Vec<ChooseArg>>,
    /// Which optional tail groups were present (for byte-exact re-encoding).
    pub(crate) tail: TailPresence,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TailPresence {
    groups: u8,
}

impl PartialEq for CrushMap {
    fn eq(&self, o: &Self) -> bool {
        self.buckets == o.buckets
            && self.rules == o.rules
            && self.max_devices == o.max_devices
            && self.tunables == o.tunables
            && self.type_map == o.type_map
            && self.name_map == o.name_map
            && self.rule_name_map == o.rule_name_map
            && self.class_map == o.class_map
            && self.class_name == o.class_name
            && self.class_bucket == o.class_bucket
            && self.choose_args == o.choose_args
    }
}

impl Eq for CrushMap {}

impl CrushMap {
    pub fn max_buckets(&self) -> i32 {
        self.buckets.len() as i32
    }

    pub fn max_rules(&self) -> u32 {
        self.rules.len() as u32
    }

    /// The bucket with this (negative) id.
    pub fn bucket(&self, id: i32) -> Option<&Bucket> {
        if id >= 0 {
            return None;
        }
        let idx = (-1 - id) as usize;
        self.buckets.get(idx).and_then(|b| b.as_ref())
    }

    pub fn rule(&self, ruleno: i32) -> Option<&Rule> {
        if ruleno < 0 {
            return None;
        }
        self.rules.get(ruleno as usize).and_then(|r| r.as_ref())
    }

    /// Id of the named item (device or bucket).
    pub fn item_id(&self, name: &str) -> Option<i32> {
        self.name_map
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    }

    /// Rule id by name.
    pub fn rule_id(&self, name: &str) -> Option<i32> {
        self.rule_name_map
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    }

    /// Device class id of an item, or -1 (`get_item_class_id`).
    pub fn item_class_id(&self, item: i32) -> i32 {
        self.class_map.get(&item).copied().unwrap_or(-1)
    }

    /// Class name by class id (`get_class_name`).
    pub fn class_name_of(&self, class_id: i32) -> Option<&str> {
        self.class_name.get(&class_id).map(|s| s.as_str())
    }

    /// `crush_finalize` + the `name_map` extension in `CrushWrapper::finalize`.
    fn finalize(&mut self) {
        let mut max_devices = 0i32;
        for b in self.buckets.iter().flatten() {
            for &it in &b.items {
                if it >= max_devices {
                    max_devices = it + 1;
                }
            }
        }
        if let Some((&last, _)) = self.name_map.iter().next_back()
            && last >= max_devices
        {
            max_devices = last + 1;
        }
        self.max_devices = max_devices;
    }

    /// `CrushWrapper::update_choose_args(nullptr)`.
    fn update_choose_args(&mut self) {
        let buckets = &self.buckets;
        for args in self.choose_args.values_mut() {
            // get_choose_args_positions: infer from the first bucket that has weight sets.
            let positions = args
                .iter()
                .find(|a| !a.weight_set.is_empty())
                .map(|a| a.weight_set.len())
                .unwrap_or(1);
            for (j, carg) in args.iter_mut().enumerate() {
                let b = buckets.get(j).and_then(|b| b.as_ref());
                let Some(b) = b.filter(|b| b.alg == CRUSH_BUCKET_STRAW2) else {
                    carg.ids.clear();
                    carg.weight_set.clear();
                    continue;
                };
                if carg.weight_set.is_empty() || carg.weight_set.len() != positions {
                    continue;
                }
                for ws in carg.weight_set.iter_mut() {
                    if ws.len() != b.items.len() {
                        ws.resize(b.items.len(), 0);
                    }
                }
            }
        }
    }

    /// Choose args for `do_rule` with `choose_args_index` (`choose_args_get_with_fallback`).
    pub fn choose_args_for(&self, index: i64) -> Option<&[ChooseArg]> {
        self.choose_args
            .get(&index)
            .or_else(|| self.choose_args.get(&DEFAULT_CHOOSE_ARGS))
            .map(|v| v.as_slice())
    }

    /// `CrushWrapper::do_rule(rule, x, out, maxout, weight, choose_args_index)`.
    pub fn do_rule(
        &self,
        ruleno: i32,
        x: i32,
        result_max: usize,
        weight: &[u32],
        choose_args_index: i64,
    ) -> Vec<i32> {
        let args = self.choose_args_for(choose_args_index);
        mapper::crush_do_rule(self, ruleno, x, result_max, weight, args)
    }
}

/// `decode_32_or_64_string_map`: a historical bug encoded `int` keys as 64 bits in some maps;
/// a zero string length means the key was 64-bit and the real length follows.
fn decode_32_or_64_string_map(buf: &mut Bytes) -> Result<BTreeMap<i32, String>> {
    let n = u32::decode(buf)?;
    let mut m = BTreeMap::new();
    for _ in 0..n {
        let key = i32::decode(buf)?;
        let mut len = u32::decode(buf)?;
        if len == 0 {
            len = u32::decode(buf)?;
        }
        let raw = denc::take(buf, len as usize, "crush name")?;
        let s = String::from_utf8(raw.to_vec()).map_err(|e| DecodeError::Invalid {
            what: "crush name",
            detail: e.to_string(),
        })?;
        m.insert(key, s);
    }
    Ok(m)
}

fn decode_u32s(buf: &mut Bytes, n: usize, what: &'static str) -> Result<Vec<u32>> {
    denc::need(buf, n.saturating_mul(4), what)?;
    Ok((0..n).map(|_| buf.get_u32_le()).collect())
}

fn decode_bucket(buf: &mut Bytes) -> Result<Option<Bucket>> {
    let alg = u32::decode(buf)?;
    if alg == 0 {
        return Ok(None);
    }
    if !(1..=5).contains(&alg) {
        return Err(DecodeError::Invalid {
            what: "crush bucket",
            detail: format!("unsupported bucket algorithm: {alg}"),
        });
    }
    let id = i32::decode(buf)?;
    let bucket_type = u16::decode(buf)?;
    let balg = u8::decode(buf)?;
    let hash = u8::decode(buf)?;
    let weight = u32::decode(buf)?;
    let size = u32::decode(buf)? as usize;
    denc::need(buf, size.saturating_mul(4), "crush bucket items")?;
    let items: Vec<i32> = (0..size).map(|_| buf.get_i32_le()).collect();
    // The second (inner) alg byte decides the layout, as in decode_crush_bucket.
    let data = match balg {
        CRUSH_BUCKET_UNIFORM => BucketData::Uniform {
            item_weight: u32::decode(buf)?,
        },
        CRUSH_BUCKET_LIST => {
            let mut item_weights = Vec::with_capacity(size);
            let mut sum_weights = Vec::with_capacity(size);
            for _ in 0..size {
                item_weights.push(u32::decode(buf)?);
                sum_weights.push(u32::decode(buf)?);
            }
            BucketData::List {
                item_weights,
                sum_weights,
            }
        }
        CRUSH_BUCKET_TREE => {
            let num_nodes = u8::decode(buf)?;
            let node_weights = decode_u32s(buf, num_nodes as usize, "crush tree weights")?;
            BucketData::Tree {
                num_nodes,
                node_weights,
            }
        }
        CRUSH_BUCKET_STRAW => {
            let mut item_weights = Vec::with_capacity(size);
            let mut straws = Vec::with_capacity(size);
            for _ in 0..size {
                item_weights.push(u32::decode(buf)?);
                straws.push(u32::decode(buf)?);
            }
            BucketData::Straw {
                item_weights,
                straws,
            }
        }
        CRUSH_BUCKET_STRAW2 => BucketData::Straw2 {
            item_weights: decode_u32s(buf, size, "crush straw2 weights")?,
        },
        other => {
            return Err(DecodeError::Invalid {
                what: "crush bucket",
                detail: format!("bucket alg {other} differs from {alg}"),
            });
        }
    };
    Ok(Some(Bucket {
        id,
        bucket_type,
        alg: balg,
        hash,
        weight,
        items,
        data,
    }))
}

impl Decode for CrushMap {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let magic = u32::decode(buf)?;
        if magic != CRUSH_MAGIC {
            return Err(DecodeError::Invalid {
                what: "crush map",
                detail: format!("bad magic number {magic:#x}"),
            });
        }
        let max_buckets = i32::decode(buf)?;
        let max_rules = u32::decode(buf)?;
        let encoded_max_devices = i32::decode(buf)?;
        if max_buckets < 0 {
            return Err(DecodeError::Invalid {
                what: "crush map",
                detail: format!("max_buckets {max_buckets}"),
            });
        }
        let mut tunables = Tunables::legacy();

        let mut buckets = Vec::with_capacity((max_buckets as usize).min(buf.remaining()));
        for _ in 0..max_buckets {
            buckets.push(decode_bucket(buf)?);
        }

        let mut rules = Vec::with_capacity((max_rules as usize).min(buf.remaining()));
        for i in 0..max_rules {
            let yes = u32::decode(buf)?;
            if yes == 0 {
                rules.push(None);
                continue;
            }
            let len = u32::decode(buf)? as usize;
            let ruleset = u8::decode(buf)?;
            if u32::from(ruleset) != i {
                return Err(DecodeError::Invalid {
                    what: "crush rule",
                    detail: "crush ruleset_id != rule_id; encoding is too old".into(),
                });
            }
            let rule_type = u8::decode(buf)?;
            let min_size = u8::decode(buf)?;
            let max_size = u8::decode(buf)?;
            denc::need(buf, len.saturating_mul(12), "crush rule steps")?;
            let steps = (0..len)
                .map(|_| RuleStep {
                    op: buf.get_u32_le(),
                    arg1: buf.get_i32_le(),
                    arg2: buf.get_i32_le(),
                })
                .collect();
            rules.push(Some(Rule {
                rule_type,
                min_size,
                max_size,
                steps,
            }));
        }

        let type_map = decode_32_or_64_string_map(buf)?;
        let name_map = decode_32_or_64_string_map(buf)?;
        let rule_name_map = decode_32_or_64_string_map(buf)?;

        let mut tail = TailPresence::default();
        let mut class_map = BTreeMap::new();
        let mut class_name = BTreeMap::new();
        let mut class_bucket = BTreeMap::new();
        let mut choose_args: BTreeMap<i64, Vec<ChooseArg>> = BTreeMap::new();
        if buf.has_remaining() {
            tunables.choose_local_tries = u32::decode(buf)?;
            tunables.choose_local_fallback_tries = u32::decode(buf)?;
            tunables.choose_total_tries = u32::decode(buf)?;
            tail.groups = 1;
        }
        if buf.has_remaining() {
            tunables.chooseleaf_descend_once = u32::decode(buf)?;
            tail.groups = 2;
        }
        if buf.has_remaining() {
            tunables.chooseleaf_vary_r = u8::decode(buf)?;
            tail.groups = 3;
        }
        if buf.has_remaining() {
            tunables.straw_calc_version = u8::decode(buf)?;
            tail.groups = 4;
        }
        if buf.has_remaining() {
            tunables.allowed_bucket_algs = u32::decode(buf)?;
            tail.groups = 5;
        }
        if buf.has_remaining() {
            tunables.chooseleaf_stable = u8::decode(buf)?;
            tail.groups = 6;
        }
        if buf.has_remaining() {
            class_map = BTreeMap::<i32, i32>::decode(buf)?;
            class_name = BTreeMap::<i32, String>::decode(buf)?;
            class_bucket = BTreeMap::<i32, BTreeMap<i32, i32>>::decode(buf)?;
            tail.groups = 7;
        }
        if buf.has_remaining() {
            let n = u32::decode(buf)?;
            for _ in 0..n {
                let index = i64::decode(buf)?;
                let mut args = vec![ChooseArg::default(); max_buckets as usize];
                let size = u32::decode(buf)?;
                for _ in 0..size {
                    let bucket_index = u32::decode(buf)? as usize;
                    if bucket_index >= args.len() {
                        return Err(DecodeError::Invalid {
                            what: "crush choose_args",
                            detail: format!(
                                "bucket index {bucket_index} >= max_buckets {max_buckets}"
                            ),
                        });
                    }
                    let positions = u32::decode(buf)?;
                    let mut weight_set =
                        Vec::with_capacity((positions as usize).min(buf.remaining()));
                    for _ in 0..positions {
                        let n = u32::decode(buf)? as usize;
                        weight_set.push(decode_u32s(buf, n, "crush weight_set")?);
                    }
                    let ids_size = u32::decode(buf)? as usize;
                    let mut ids = Vec::new();
                    if ids_size > 0 {
                        let bsize = buckets[bucket_index].as_ref().map(|b| b.items.len());
                        if bsize != Some(ids_size) {
                            return Err(DecodeError::Invalid {
                                what: "crush choose_args",
                                detail: format!("ids_size {ids_size} != bucket size {bsize:?}"),
                            });
                        }
                        denc::need(buf, ids_size * 4, "crush choose_args ids")?;
                        ids = (0..ids_size).map(|_| buf.get_i32_le()).collect();
                    }
                    args[bucket_index] = ChooseArg { ids, weight_set };
                }
                choose_args.insert(index, args);
            }
            tail.groups = 8;
        }
        if buf.has_remaining() {
            tunables.msr_descents = u32::decode(buf)?;
            tunables.msr_collision_tries = u32::decode(buf)?;
            tail.groups = 9;
        } else {
            tunables.msr_descents = 100;
            tunables.msr_collision_tries = 100;
        }

        let mut map = CrushMap {
            buckets,
            rules,
            max_devices: encoded_max_devices,
            encoded_max_devices,
            tunables,
            type_map,
            name_map,
            rule_name_map,
            class_map,
            class_name,
            class_bucket,
            choose_args,
            tail,
        };
        map.update_choose_args();
        map.finalize();
        Ok(map)
    }
}

fn encode_string_map(m: &BTreeMap<i32, String>, buf: &mut BytesMut) {
    buf.put_u32_le(m.len() as u32);
    for (k, v) in m {
        buf.put_i32_le(*k);
        v.as_str().encode(buf, 0);
    }
}

/// `CrushWrapper::encode` for a peer with `features` (used by tests to verify that decoding
/// kept every field; the MDS never sends CRUSH maps). Optional tail groups that were absent in
/// the decoded input are omitted again.
impl Encode for CrushMap {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u32_le(CRUSH_MAGIC);
        buf.put_i32_le(self.buckets.len() as i32);
        buf.put_u32_le(self.rules.len() as u32);
        buf.put_i32_le(self.encoded_max_devices);
        for b in &self.buckets {
            let Some(b) = b else {
                buf.put_u32_le(0);
                continue;
            };
            buf.put_u32_le(u32::from(b.alg));
            buf.put_i32_le(b.id);
            buf.put_u16_le(b.bucket_type);
            buf.put_u8(b.alg);
            buf.put_u8(b.hash);
            buf.put_u32_le(b.weight);
            buf.put_u32_le(b.items.len() as u32);
            for &it in &b.items {
                buf.put_i32_le(it);
            }
            match &b.data {
                BucketData::Uniform { item_weight } => buf.put_u32_le(*item_weight),
                BucketData::List {
                    item_weights,
                    sum_weights,
                } => {
                    for (w, s) in item_weights.iter().zip(sum_weights) {
                        buf.put_u32_le(*w);
                        buf.put_u32_le(*s);
                    }
                }
                BucketData::Tree {
                    num_nodes,
                    node_weights,
                } => {
                    buf.put_u8(*num_nodes);
                    for w in node_weights {
                        buf.put_u32_le(*w);
                    }
                }
                BucketData::Straw {
                    item_weights,
                    straws,
                } => {
                    for (w, s) in item_weights.iter().zip(straws) {
                        buf.put_u32_le(*w);
                        buf.put_u32_le(*s);
                    }
                }
                BucketData::Straw2 { item_weights } => {
                    for w in item_weights {
                        buf.put_u32_le(*w);
                    }
                }
            }
        }
        for (i, r) in self.rules.iter().enumerate() {
            let Some(r) = r else {
                buf.put_u32_le(0);
                continue;
            };
            buf.put_u32_le(1);
            buf.put_u32_le(r.steps.len() as u32);
            buf.put_u8(i as u8);
            buf.put_u8(r.rule_type);
            if features::has(f, features::MASK_SERVER_QUINCY) {
                buf.put_u8(1);
                buf.put_u8(100);
            } else {
                buf.put_u8(r.min_size);
                buf.put_u8(r.max_size);
            }
            for s in &r.steps {
                buf.put_u32_le(s.op);
                buf.put_i32_le(s.arg1);
                buf.put_i32_le(s.arg2);
            }
        }
        encode_string_map(&self.type_map, buf);
        encode_string_map(&self.name_map, buf);
        encode_string_map(&self.rule_name_map, buf);
        let g = self.tail.groups;
        let t = &self.tunables;
        if g >= 1 {
            buf.put_u32_le(t.choose_local_tries);
            buf.put_u32_le(t.choose_local_fallback_tries);
            buf.put_u32_le(t.choose_total_tries);
        }
        if g >= 2 {
            buf.put_u32_le(t.chooseleaf_descend_once);
        }
        if g >= 3 {
            buf.put_u8(t.chooseleaf_vary_r);
        }
        if g >= 4 {
            buf.put_u8(t.straw_calc_version);
        }
        if g >= 5 {
            buf.put_u32_le(t.allowed_bucket_algs);
        }
        if g >= 6 && f & features::CRUSH_TUNABLES5 != 0 {
            buf.put_u8(t.chooseleaf_stable);
        }
        if g >= 7 && features::has(f, features::MASK_SERVER_LUMINOUS) {
            self.class_map.encode(buf, f);
            self.class_name.encode(buf, f);
            self.class_bucket.encode(buf, f);
            buf.put_u32_le(self.choose_args.len() as u32);
            for (index, args) in &self.choose_args {
                buf.put_i64_le(*index);
                let present: Vec<(usize, &ChooseArg)> = args
                    .iter()
                    .enumerate()
                    .filter(|(_, a)| !(a.weight_set.is_empty() && a.ids.is_empty()))
                    .collect();
                buf.put_u32_le(present.len() as u32);
                for (i, a) in present {
                    buf.put_u32_le(i as u32);
                    buf.put_u32_le(a.weight_set.len() as u32);
                    for ws in &a.weight_set {
                        buf.put_u32_le(ws.len() as u32);
                        for w in ws {
                            buf.put_u32_le(*w);
                        }
                    }
                    buf.put_u32_le(a.ids.len() as u32);
                    for id in &a.ids {
                        buf.put_i32_le(*id);
                    }
                }
            }
        }
        if g >= 9 && features::has(f, features::MASK_CRUSH_MSR) {
            buf.put_u32_le(t.msr_descents);
            buf.put_u32_le(t.msr_collision_tries);
        }
    }
}
