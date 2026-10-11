//! `MDSMap`, `mds_info_t`, `CompatSet` and MDS states.
//!
//! Follows `MDSMap::decode` and `MDSMap::mds_info_t::decode` (`src/mds/MDSMap.cc`),
//! `CompatSet::encode/decode` (`src/common/CompatSet.cc`) and the state values of
//! `src/include/ceph_fs.h` (`CEPH_MDS_STATE_*`) with the names of `ceph_mds_state_name`
//! (`src/common/ceph_strings.cc`).
//!
//! `MDSMap` native form: `ENCODE_START(5, 4)`; `u32 epoch`, `u32 flags`, `u32 last_failure`,
//! `s32 root`, `u32 session_timeout`, `u32 session_autoclose`, `u64 max_file_size`,
//! `s32 max_mds`, `map<u64 gid, mds_info_t>`, `vector<s64> data_pools` (`u32` each before
//! struct_v 3), `s64 cas_pool`, then (struct_v >= 2) `u16 ev` and the extended fields:
//! `CompatSet` (ev>=3), `s64 metadata_pool` (`u32` before ev 5), `utime created, modified`,
//! `s32 tableserver`, `set<s32> in`, `map<s32,s32> inc` (legacy, dropped), `map<s32,u64> up`,
//! `set<s32> failed, stopped`, `u32 last_failure_osd_epoch` (ev>=4), two `u8` feature
//! bitmaps (ev>=6; bools before ev 10), `bool inline_data_enabled` (ev>=7), `bool enabled` +
//! `string fs_name` (ev>=8), `set<s32> damaged` (ev>=9), `string balancer` (ev>=11),
//! `s32 standby_count_wanted` (ev>=12), `s32 old_max_mds` (ev>=13), `u8 min_compat_client`
//! (ev>=14), `feature_bitset_t required_client_features` (`u32` byte length + bytes, ev>=16),
//! `string bal_rank_mask` (ev>=17), `u64 max_xattr_size` (ev>=18), `u64 qdb_cluster_leader` +
//! `set<u64> qdb_cluster_members` (ev>=19), `bool standby_enable_host_anti_affinity` (ev>=20).
//! Ceph 20.2 encodes ev 19, `main` ev 20. Unknown trailing bytes are skipped by the framing.
//! Pre-Nautilus/pre-MDSENC legacy forms (`u16` version) are not decoded.

use std::collections::{BTreeMap, BTreeSet};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use denc::{Decode, DecodeError, Encode, EntityAddrVec, UTime, decode_start};

/// An MDS daemon or rank state (`MDSMap::DaemonState`, `CEPH_MDS_STATE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MdsState(pub i32);

impl MdsState {
    pub const DNE: MdsState = MdsState(0);
    pub const STOPPED: MdsState = MdsState(-1);
    pub const BOOT: MdsState = MdsState(-4);
    pub const STANDBY: MdsState = MdsState(-5);
    pub const CREATING: MdsState = MdsState(-6);
    pub const STARTING: MdsState = MdsState(-7);
    pub const STANDBY_REPLAY: MdsState = MdsState(-8);
    pub const REPLAYONCE: MdsState = MdsState(-9);
    pub const NULL: MdsState = MdsState(-10);
    pub const REPLAY: MdsState = MdsState(8);
    pub const RESOLVE: MdsState = MdsState(9);
    pub const RECONNECT: MdsState = MdsState(10);
    pub const REJOIN: MdsState = MdsState(11);
    pub const CLIENTREPLAY: MdsState = MdsState(12);
    pub const ACTIVE: MdsState = MdsState(13);
    pub const STOPPING: MdsState = MdsState(14);
    pub const DAMAGED: MdsState = MdsState(15);

    /// `ceph_mds_state_name`.
    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "down:dne",
            -1 => "down:stopped",
            15 => "down:damaged",
            -4 => "up:boot",
            -5 => "up:standby",
            -8 => "up:standby-replay",
            -9 => "up:oneshot-replay",
            -6 => "up:creating",
            -7 => "up:starting",
            8 => "up:replay",
            9 => "up:resolve",
            10 => "up:reconnect",
            11 => "up:rejoin",
            12 => "up:clientreplay",
            13 => "up:active",
            14 => "up:stopping",
            -10 => "null",
            _ => "???",
        }
    }

    /// `MDSMap::state_transition_valid(prev, next)`.
    pub fn transition_valid(prev: MdsState, next: MdsState) -> bool {
        use MdsState as S;
        if next == prev || next == S::DAMAGED {
            return true;
        }
        match prev {
            S::BOOT => next == S::STANDBY,
            S::STANDBY => matches!(next, S::STANDBY_REPLAY | S::REPLAY | S::CREATING | S::STARTING),
            S::CREATING | S::STARTING => next == S::ACTIVE,
            S::STANDBY_REPLAY => next == S::REPLAY,
            S::REPLAY => matches!(next, S::RESOLVE | S::RECONNECT),
            p if p.0 >= S::RESOLVE.0 && p.0 < S::ACTIVE.0 => {
                (p == S::REJOIN && matches!(next, S::ACTIVE | S::STOPPED)) || next.0 == p.0 + 1
            }
            S::ACTIVE => next == S::STOPPING,
            S::STOPPING => next == S::STOPPED,
            _ => false,
        }
    }
}

impl std::fmt::Display for MdsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// `CompatSet::FeatureSet`: `u64 mask` (bit 0 cleared on the wire, set in memory) +
/// `map<u64, string>` names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureSet {
    /// Feature ids (1..=63) with names.
    pub names: BTreeMap<u64, String>,
}

impl FeatureSet {
    pub fn mask(&self) -> u64 {
        self.names.keys().fold(0u64, |m, id| m | (1u64 << id))
    }

    pub fn insert(&mut self, id: u64, name: &str) {
        assert!(id > 0 && id < 64, "feature id {id} out of range");
        self.names.insert(id, name.to_string());
    }

    pub fn contains(&self, id: u64) -> bool {
        self.names.contains_key(&id)
    }

    /// `mask` superset test.
    pub fn is_superset_of(&self, other: &FeatureSet) -> bool {
        other.mask() & !self.mask() == 0
    }
}

impl Encode for FeatureSet {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u64_le(self.mask() & !1);
        self.names.encode(buf, f);
    }
}

impl Decode for FeatureSet {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let mask = u64::decode(buf)?;
        let names: BTreeMap<u64, String> = BTreeMap::decode(buf)?;
        // Old encodings set bit 0 and the mask is rebuilt from the names; otherwise only the
        // ids present in the mask count. Keep the names as the source of truth, filtered by
        // the mask for the modern form.
        let names = if mask & 1 != 0 {
            names.into_iter().filter(|(id, _)| *id > 0 && *id < 64).collect()
        } else {
            let mut n: BTreeMap<u64, String> = names.into_iter().filter(|(id, _)| *id < 64 && mask & (1 << id) != 0).collect();
            for id in 1..64u64 {
                if mask & (1 << id) != 0 {
                    n.entry(id).or_default();
                }
            }
            n
        };
        Ok(FeatureSet { names })
    }
}

/// `CompatSet`: compat, ro_compat, incompat (no version header).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompatSet {
    pub compat: FeatureSet,
    pub ro_compat: FeatureSet,
    pub incompat: FeatureSet,
}

/// MDS incompat feature ids (`src/mds/MDSMap.h`).
pub mod incompat {
    pub const BASE: u64 = 1;
    pub const CLIENTRANGES: u64 = 2;
    pub const FILELAYOUT: u64 = 3;
    pub const DIRINODE: u64 = 4;
    pub const ENCODING: u64 = 5;
    pub const OMAPDIRFRAG: u64 = 6;
    pub const INLINE: u64 = 7;
    pub const NOANCHOR: u64 = 8;
    pub const FILE_LAYOUT_V2: u64 = 9;
    pub const SNAPREALM_V2: u64 = 10;
    pub const MINORLOGSEGMENTS: u64 = 11;
    pub const QUIESCE_SUBVOLUMES: u64 = 12;
    /// The Rust MDS's private format bit (decision 2).
    pub const RUST_MDS_FORMAT_V0: u64 = 63;
}

impl CompatSet {
    /// `MDSMap::get_compat_set_all()`: incompat bits 1-12 with their C++ names.
    pub fn mds_all() -> Self {
        let mut c = CompatSet::default();
        for (id, name) in [
            (incompat::BASE, "base v0.20"),
            (incompat::CLIENTRANGES, "client writeable ranges"),
            (incompat::FILELAYOUT, "default file layouts on dirs"),
            (incompat::DIRINODE, "dir inode in separate object"),
            (incompat::ENCODING, "mds uses versioned encoding"),
            (incompat::OMAPDIRFRAG, "dirfrag is stored in omap"),
            (incompat::INLINE, "mds uses inline data"),
            (incompat::NOANCHOR, "no anchor table"),
            (incompat::FILE_LAYOUT_V2, "file layout v2"),
            (incompat::SNAPREALM_V2, "snaprealm v2"),
            (incompat::MINORLOGSEGMENTS, "minor log segments"),
            (incompat::QUIESCE_SUBVOLUMES, "quiesce subvolumes"),
        ] {
            c.incompat.insert(id, name);
        }
        c
    }

    /// The compat set the Rust MDS advertises (decision 2): [`mds_all`](Self::mds_all) plus
    /// incompat bit 63 `rust mds format v0`. Constant for the process lifetime.
    pub fn rust_mds() -> Self {
        let mut c = Self::mds_all();
        c.incompat.insert(incompat::RUST_MDS_FORMAT_V0, "rust mds format v0");
        c
    }

    /// `CompatSet::readable(other)`: other's incompat ⊆ ours.
    pub fn readable(&self, other: &CompatSet) -> bool {
        self.incompat.is_superset_of(&other.incompat)
    }

    /// `CompatSet::writeable(other)`: readable and other's ro_compat ⊆ ours.
    pub fn writeable(&self, other: &CompatSet) -> bool {
        self.readable(other) && self.ro_compat.is_superset_of(&other.ro_compat)
    }
}

impl Encode for CompatSet {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        self.compat.encode(buf, f);
        self.ro_compat.encode(buf, f);
        self.incompat.encode(buf, f);
    }
}

impl Decode for CompatSet {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        Ok(CompatSet { compat: FeatureSet::decode(buf)?, ro_compat: FeatureSet::decode(buf)?, incompat: FeatureSet::decode(buf)? })
    }
}

/// `MDSMap::mds_info_t`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdsInfo {
    pub global_id: u64,
    pub name: String,
    pub rank: i32,
    pub inc: i32,
    pub state: MdsState,
    pub state_seq: u64,
    pub addrs: EntityAddrVec,
    pub laggy_since: UTime,
    pub export_targets: BTreeSet<i32>,
    pub mds_features: u64,
    pub join_fscid: i32,
    pub flags: u64,
    pub compat: CompatSet,
}

impl MdsInfo {
    pub fn is_laggy(&self) -> bool {
        self.laggy_since != UTime::default()
    }
}

impl Decode for MdsInfo {
    /// `mds_info_t::decode`: `DECODE_START_LEGACY_COMPAT_LEN(10, 4, 4)`; this decoder needs
    /// the length-framed form (struct_v >= 4).
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let mut f = decode_start(buf, 10, "mds_info_t")?;
        if f.v < 4 {
            return Err(DecodeError::Invalid { what: "mds_info_t", detail: format!("legacy struct_v {}", f.v) });
        }
        let b = &mut f.body;
        let global_id = u64::decode(b)?;
        let name = String::decode(b)?;
        let rank = i32::decode(b)?;
        let inc = i32::decode(b)?;
        let state = MdsState(i32::decode(b)?);
        let state_seq = u64::decode(b)?;
        let addrs = EntityAddrVec::decode(b)?;
        let laggy_since = UTime::decode(b)?;
        let _standby_for_rank = i32::decode(b)?;
        let _standby_for_name = String::decode(b)?;
        let export_targets = if f.v >= 2 { BTreeSet::decode(b)? } else { BTreeSet::new() };
        let mds_features = if f.v >= 5 { u64::decode(b)? } else { 0 };
        let join_fscid = if f.v >= 6 { i32::decode(b)? } else { -1 };
        if f.v >= 7 {
            let _standby_replay = bool::decode(b)?;
        }
        let flags = if f.v >= 9 { u64::decode(b)? } else { 0 };
        let compat = if f.v >= 10 { CompatSet::decode(b)? } else { v16_2_4_compat() };
        Ok(MdsInfo { global_id, name, rank, inc, state, state_seq, addrs, laggy_since, export_targets, mds_features, join_fscid, flags, compat })
    }
}

/// `MDSMap::get_compat_set_v16_2_4()`: bits 1-10.
fn v16_2_4_compat() -> CompatSet {
    let mut c = CompatSet::mds_all();
    c.incompat.names.retain(|id, _| *id <= incompat::SNAPREALM_V2);
    c
}

/// `MDSMap` (the fields the daemon and tools need).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MdsMap {
    pub epoch: u32,
    pub flags: u32,
    pub last_failure: u32,
    pub root: i32,
    pub session_timeout: u32,
    pub session_autoclose: u32,
    pub max_file_size: u64,
    pub max_mds: i32,
    pub mds_info: BTreeMap<u64, MdsInfo>,
    pub data_pools: Vec<i64>,
    pub cas_pool: i64,
    pub ev: u16,
    pub compat: CompatSet,
    pub metadata_pool: i64,
    pub created: UTime,
    pub modified: UTime,
    pub tableserver: i32,
    pub in_: BTreeSet<i32>,
    pub up: BTreeMap<i32, u64>,
    pub failed: BTreeSet<i32>,
    pub stopped: BTreeSet<i32>,
    pub last_failure_osd_epoch: u32,
    pub ever_allowed_features: u8,
    pub explicitly_allowed_features: u8,
    pub inline_data_enabled: bool,
    pub enabled: bool,
    pub fs_name: String,
    pub damaged: BTreeSet<i32>,
    pub balancer: String,
    pub standby_count_wanted: i32,
    pub old_max_mds: i32,
    pub min_compat_client: u8,
    /// `feature_bitset_t` as raw bytes (little-endian u64 blocks).
    pub required_client_features: Bytes,
    pub bal_rank_mask: String,
    pub max_xattr_size: u64,
    pub qdb_cluster_leader: u64,
    pub qdb_cluster_members: BTreeSet<u64>,
    pub standby_enable_host_anti_affinity: bool,
}

/// `CEPH_MDSMAP_*` flags.
pub mod flags {
    pub const NOT_JOINABLE: u32 = 1 << 0;
    pub const ALLOW_SNAPS: u32 = 1 << 1;
    pub const ALLOW_MULTIMDS_SNAPS: u32 = 1 << 4;
    pub const ALLOW_STANDBY_REPLAY: u32 = 1 << 5;
    pub const REFUSE_CLIENT_SESSION: u32 = 1 << 6;
    pub const REFUSE_STANDBY_FOR_ANOTHER_FS: u32 = 1 << 7;
    pub const BALANCE_AUTOMATE: u32 = 1 << 8;
}

impl MdsMap {
    /// The epoch of the null map the monitor sends to removed daemons (`UINT32_MAX`).
    pub const NULL_EPOCH: u32 = u32::MAX;

    /// The entry of daemon `gid`.
    pub fn info(&self, gid: u64) -> Option<&MdsInfo> {
        self.mds_info.get(&gid)
    }

    /// The state of daemon `gid` (`STATE_NULL` when absent, as `get_state_gid`).
    pub fn state_of(&self, gid: u64) -> MdsState {
        self.info(gid).map(|i| i.state).unwrap_or(MdsState::NULL)
    }
}

impl Decode for MdsMap {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        // The legacy forms start with a u16 version (2 or 3) instead of u8 v + u8 compat.
        denc::need(buf, 6, "MDSMap")?;
        let mut f = decode_start(buf, 5, "MDSMap")?;
        if f.v < 4 {
            return Err(DecodeError::Invalid { what: "MDSMap", detail: format!("legacy struct_v {}", f.v) });
        }
        let b = &mut f.body;
        let mut m = MdsMap {
            epoch: u32::decode(b)?,
            flags: u32::decode(b)?,
            last_failure: u32::decode(b)?,
            root: i32::decode(b)?,
            session_timeout: u32::decode(b)?,
            session_autoclose: u32::decode(b)?,
            max_file_size: u64::decode(b)?,
            max_mds: i32::decode(b)?,
            mds_info: BTreeMap::decode(b)?,
            ..Default::default()
        };
        if f.v < 3 {
            let n = u32::decode(b)?;
            for _ in 0..n {
                m.data_pools.push(i64::from(u32::decode(b)?));
            }
            m.cas_pool = i64::from(i32::decode(b)?);
        } else {
            m.data_pools = Vec::decode(b)?;
            m.cas_pool = i64::decode(b)?;
        }
        let ev = if f.v >= 2 { u16::decode(b)? } else { 1 };
        m.ev = ev;
        m.compat = if ev >= 3 { CompatSet::decode(b)? } else { CompatSet::default() };
        m.metadata_pool = if ev < 5 { i64::from(u32::decode(b)?) } else { i64::decode(b)? };
        m.created = UTime::decode(b)?;
        m.modified = UTime::decode(b)?;
        m.tableserver = i32::decode(b)?;
        m.in_ = BTreeSet::decode(b)?;
        let _inc: BTreeMap<i32, i32> = BTreeMap::decode(b)?;
        m.up = BTreeMap::decode(b)?;
        m.failed = BTreeSet::decode(b)?;
        m.stopped = BTreeSet::decode(b)?;
        if ev >= 4 {
            m.last_failure_osd_epoch = u32::decode(b)?;
        }
        if ev >= 6 {
            m.ever_allowed_features = u8::decode(b)?;
            m.explicitly_allowed_features = u8::decode(b)?;
        }
        if ev >= 7 {
            m.inline_data_enabled = bool::decode(b)?;
        }
        if ev >= 8 {
            m.enabled = bool::decode(b)?;
            m.fs_name = String::decode(b)?;
        } else {
            m.enabled = m.epoch > 1;
        }
        if ev >= 9 {
            m.damaged = BTreeSet::decode(b)?;
        }
        if ev >= 11 {
            m.balancer = String::decode(b)?;
        }
        if ev >= 12 {
            m.standby_count_wanted = i32::decode(b)?;
        }
        if ev >= 13 {
            m.old_max_mds = i32::decode(b)?;
        }
        if ev >= 14 {
            m.min_compat_client = u8::decode(b)?;
        }
        if ev >= 16 {
            let len = u32::decode(b)? as usize;
            m.required_client_features = denc::take(b, len, "required_client_features")?;
        }
        if ev >= 17 {
            m.bal_rank_mask = String::decode(b)?;
        }
        if ev >= 18 {
            m.max_xattr_size = u64::decode(b)?;
        }
        if ev >= 19 {
            m.qdb_cluster_leader = u64::decode(b)?;
            m.qdb_cluster_members = BTreeSet::decode(b)?;
        }
        if ev >= 20 {
            m.standby_enable_host_anti_affinity = bool::decode(b)?;
        }
        // "All MDS since at least v14.0.0 understand INLINE": inserted into the fs compat and
        // into every daemon compat (an empty daemon compat first inherits the fs compat).
        m.compat.incompat.insert(incompat::INLINE, "mds uses inline data");
        let fs_compat = m.compat.clone();
        for info in m.mds_info.values_mut() {
            if info.compat == CompatSet::default() {
                info.compat = fs_compat.clone();
            }
            info.compat.incompat.insert(incompat::INLINE, "mds uses inline data");
        }
        let _ = b.remaining();
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Bytes {
        Bytes::from(hex::decode(s).unwrap())
    }

    #[test]
    fn compat_set_golden() {
        // ceph-dencoder type CompatSet select_test 2.
        let b = unhex(
            "0600000000000000020000000100000000000000030000006f6e6502000000000000000300000074776f100000000000000001000000040000000000000004000000666f75720800000000000000010000000300000000000000050000007468726565",
        );
        let c = denc::decode_exact::<CompatSet>(b.clone(), "compat").unwrap();
        assert_eq!(c.compat.names[&1], "one");
        assert_eq!(c.ro_compat.names[&4], "four");
        assert_eq!(c.incompat.names[&3], "three");
        assert_eq!(denc::encode_to_bytes(&c, 0), b);
    }

    #[test]
    fn rust_compat_bits() {
        let c = CompatSet::rust_mds();
        assert_eq!(c.incompat.mask(), 0x1ffe | (1 << 63));
        let b = denc::encode_to_bytes(&c, 0);
        // compat and ro_compat empty (mask 0, no names), incompat mask with bit 0 cleared.
        assert_eq!(&b[..24], &[0u8; 24]);
        assert_eq!(&b[24..32], &(0x1ffeu64 | (1 << 63)).to_le_bytes());
        assert!(c.writeable(&CompatSet::mds_all()));
        assert!(!CompatSet::mds_all().writeable(&c));
    }

    #[test]
    fn mds_info_golden() {
        // ceph-dencoder type MDSMap::mds_info_t select_test 2.
        let b = unhex(
            "0a047f00000001000000000000000d000000746573745f696e7374616e63650000000000000000fbffffff000000000000000002000000000000000000000000ffffffff00000000000000000000000000000000ffffffff000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        );
        let i = denc::decode_exact::<MdsInfo>(b, "info").unwrap();
        assert_eq!(i.global_id, 1);
        assert_eq!(i.name, "test_instance");
        assert_eq!(i.rank, 0);
        assert_eq!(i.state, MdsState::STANDBY);
        assert_eq!(i.join_fscid, -1);
        assert!(i.addrs.0.is_empty());
    }

    #[test]
    fn mdsmap_golden() {
        let b = Bytes::from_static(include_bytes!("../tests/data/mdsmap_test1.bin"));
        let m = denc::decode_exact::<MdsMap>(b, "mdsmap").unwrap();
        assert_eq!(m.epoch, 0);
        assert_eq!(m.flags, 18);
        assert_eq!(m.session_timeout, 61);
        assert_eq!(m.session_autoclose, 301);
        assert_eq!(m.max_file_size, 16777216);
        assert_eq!(m.max_mds, 1);
        assert_eq!(m.data_pools, vec![0]);
        assert_eq!(m.metadata_pool, 1);
        assert_eq!(m.fs_name, "cephfs");
        assert_eq!(m.bal_rank_mask, "-1");
        assert_eq!(m.max_xattr_size, 65536);
        assert_eq!(m.compat, CompatSet::mds_all());
        assert!(m.mds_info.is_empty());
    }

    #[test]
    fn transitions() {
        use MdsState as S;
        assert!(MdsState::transition_valid(S::BOOT, S::STANDBY));
        assert!(!MdsState::transition_valid(S::BOOT, S::ACTIVE));
        assert!(MdsState::transition_valid(S::STANDBY, S::CREATING));
        assert!(MdsState::transition_valid(S::CREATING, S::ACTIVE));
        assert!(MdsState::transition_valid(S::REPLAY, S::RECONNECT));
        assert!(MdsState::transition_valid(S::RECONNECT, S::REJOIN));
        assert!(MdsState::transition_valid(S::REJOIN, S::ACTIVE));
        assert!(!MdsState::transition_valid(S::RECONNECT, S::ACTIVE));
        assert!(MdsState::transition_valid(S::ACTIVE, S::STOPPING));
        assert_eq!(S::ACTIVE.name(), "up:active");
        assert_eq!(S::STANDBY_REPLAY.name(), "up:standby-replay");
    }
}
