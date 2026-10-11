//! Small core types shared by every layer.

use crate::{Bytes, BytesMut, Decode, Encode, Result};
use bytes::BufMut;

/// `utime_t` / `ceph_timespec`: `u32` seconds + `u32` nanoseconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UTime {
    pub sec: u32,
    pub nsec: u32,
}

impl UTime {
    pub fn now() -> Self {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        UTime { sec: d.as_secs() as u32, nsec: d.subsec_nanos() }
    }
}

impl Encode for UTime {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u32_le(self.sec);
        buf.put_u32_le(self.nsec);
    }
}
impl Decode for UTime {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        Ok(UTime { sec: u32::decode(buf)?, nsec: u32::decode(buf)? })
    }
}

/// Entity types (`CEPH_ENTITY_TYPE_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityType(pub u8);

impl EntityType {
    pub const MON: EntityType = EntityType(0x01);
    pub const MDS: EntityType = EntityType(0x02);
    pub const OSD: EntityType = EntityType(0x04);
    pub const CLIENT: EntityType = EntityType(0x08);
    pub const MGR: EntityType = EntityType(0x10);
    pub const AUTH: EntityType = EntityType(0x20);
    pub const ANY: EntityType = EntityType(0xff);

    pub fn name(self) -> &'static str {
        match self.0 {
            0x01 => "mon",
            0x02 => "mds",
            0x04 => "osd",
            0x08 => "client",
            0x10 => "mgr",
            0x20 => "auth",
            0xff => "any",
            _ => "unknown",
        }
    }
}

/// `entity_name_t` (`ceph_entity_name`): `u8` type + `i64` number. For an MDS rank holder the
/// number is the rank; for clients it is the cephx global id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityName {
    pub ty: EntityType,
    pub num: i64,
}

impl EntityName {
    pub const fn new(ty: EntityType, num: i64) -> Self {
        EntityName { ty, num }
    }
}

impl std::fmt::Display for EntityName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.ty.name(), self.num)
    }
}

impl Encode for EntityName {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u8(self.ty.0);
        buf.put_i64_le(self.num);
    }
}
impl Decode for EntityName {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        Ok(EntityName { ty: EntityType(u8::decode(buf)?), num: i64::decode(buf)? })
    }
}

/// `uuid_d`: 16 raw bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uuid(pub [u8; 16]);

impl Encode for Uuid {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        self.0.encode(buf, f);
    }
}
impl Decode for Uuid {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        Ok(Uuid(<[u8; 16]>::decode(buf)?))
    }
}

impl Uuid {
    /// Parse the canonical `8-4-4-4-12` hex form (as in `ceph.conf` `fsid`).
    pub fn parse(s: &str) -> Option<Self> {
        let hex: String = s.chars().filter(|c| *c != '-').collect();
        if hex.len() != 32 {
            return None;
        }
        let mut out = [0u8; 16];
        for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
            let t = std::str::from_utf8(chunk).ok()?;
            out[i] = u8::from_str_radix(t, 16).ok()?;
        }
        Some(Uuid(out))
    }
}
