//! `MonMap` (the fields a MonClient needs). Follows `MonMap::decode` and
//! `mon_info_t::decode` (`src/mon/MonMap.cc`).
//!
//! Modern form (struct_v >= 6, `ENCODE_START` framing): `uuid fsid` (raw 16 bytes),
//! `u32 epoch`, `utime last_changed`, `utime created`, `mon_feature_t persistent`,
//! `mon_feature_t optional`, `map<string, mon_info_t>`, `vector<string> ranks`, then
//! `u8 min_mon_release` (v>=7), removed ranks/strategy/disallowed leaders (v>=8), stretch mode
//! (v>=9), and `u32 auth_epoch` + cipher policy (v>=10, Ceph after 20.2). `mon_info_t`
//! (`ENCODE_START(6, 1)`): `string name`, `entity_addrvec_t public_addrs`, `u16 priority`
//! (v>=2), `u16 weight` (v>=4), crush location (v>=5), time added (v>=6).

use std::collections::BTreeMap;

use bytes::Bytes;
use denc::{AddrType, Decode, DecodeError, EntityAddr, EntityAddrVec, UTime, Uuid, decode_start};

/// One monitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonInfo {
    pub name: String,
    pub addrs: EntityAddrVec,
    pub priority: u16,
    pub weight: u16,
}

impl MonInfo {
    /// The msgr1 address of this monitor, if it has one.
    pub fn v1_addr(&self) -> Option<EntityAddr> {
        self.addrs.first_of(AddrType::LEGACY)
    }
}

impl Decode for MonInfo {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let mut f = decode_start(buf, 6, "mon_info_t")?;
        let b = &mut f.body;
        let name = String::decode(b)?;
        let addrs = EntityAddrVec::decode(b)?;
        let priority = if f.v >= 2 { u16::decode(b)? } else { 0 };
        let weight = if f.v >= 4 { u16::decode(b)? } else { 0 };
        Ok(MonInfo { name, addrs, priority, weight })
    }
}

/// The monitor map.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonMap {
    pub fsid: Uuid,
    pub epoch: u32,
    pub last_changed: UTime,
    pub created: UTime,
    pub mons: BTreeMap<String, MonInfo>,
    /// Monitor names by rank.
    pub ranks: Vec<String>,
    pub min_mon_release: u8,
    /// MonMap v10 `auth_epoch` (0 when absent): growing it invalidates tickets and rotating
    /// secrets.
    pub auth_epoch: u32,
}

impl MonMap {
    /// msgr1 addresses of all monitors, in rank order.
    pub fn v1_addrs(&self) -> Vec<EntityAddr> {
        let mut out = Vec::new();
        for name in &self.ranks {
            if let Some(a) = self.mons.get(name).and_then(|m| m.v1_addr()) {
                out.push(a);
            }
        }
        for (name, m) in &self.mons {
            if !self.ranks.contains(name)
                && let Some(a) = m.v1_addr()
            {
                out.push(a);
            }
        }
        out
    }

    /// The name of the monitor at `addr` (matching ip and port).
    pub fn name_of(&self, addr: &EntityAddr) -> Option<&str> {
        self.mons.values().find(|m| m.addrs.0.iter().any(|a| a.addr == addr.addr)).map(|m| m.name.as_str())
    }
}

impl Decode for MonMap {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let mut f = decode_start(buf, 10, "MonMap")?;
        if f.v < 6 {
            return Err(DecodeError::Invalid { what: "MonMap", detail: format!("struct_v {} too old", f.v) });
        }
        let b = &mut f.body;
        let fsid = Uuid::decode(b)?;
        let epoch = u32::decode(b)?;
        let last_changed = UTime::decode(b)?;
        let created = UTime::decode(b)?;
        // persistent_features, optional_features (mon_feature_t, versioned)
        let _ = decode_start(b, 255, "mon_feature_t")?;
        let _ = decode_start(b, 255, "mon_feature_t")?;
        let mons: BTreeMap<String, MonInfo> = BTreeMap::decode(b)?;
        let ranks: Vec<String> = Vec::decode(b)?;
        let min_mon_release = if f.v >= 7 { u8::decode(b)? } else { 0 };
        let mut auth_epoch = 0;
        if f.v >= 10 {
            // removed_ranks, strategy, disallowed_leaders (v8), stretch fields (v9).
            let _removed: std::collections::BTreeSet<i32> = Decode::decode(b)?;
            let _strategy = u8::decode(b)?;
            let _disallowed: std::collections::BTreeSet<String> = Decode::decode(b)?;
            let _stretch = bool::decode(b)?;
            let _tiebreaker = String::decode(b)?;
            let _marked_down: std::collections::BTreeSet<String> = Decode::decode(b)?;
            auth_epoch = u32::decode(b)?;
        }
        Ok(MonMap { fsid, epoch, last_changed, created, mons, ranks, min_mon_release, auth_epoch })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_dencoder_test_instance() {
        // ceph-dencoder type MonMap select_test 3 encode export (Ceph 20.2, struct_v 9).
        let b = Bytes::from_static(include_bytes!("../tests/data/monmap_test3.bin"));
        let m = denc::decode_exact::<MonMap>(b, "monmap").unwrap();
        assert_eq!(m.epoch, 1);
        assert_eq!(m.last_changed, UTime { sec: 123, nsec: 456 });
        assert_eq!(m.mons.len(), 4);
        assert_eq!(m.ranks, vec!["empty_addr_zero", "empty_addr_one", "empty_addr_two", "filled_pub_addr"]);
        assert_eq!(m.mons["empty_addr_one"].addrs.0[0].nonce, 1);
        assert!(m.v1_addrs().is_empty());
    }
}
