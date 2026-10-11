//! `EntityName` of `src/common/entity_name.h`: the cephx principal name such as `mds.rust` or
//! `client.admin`.
//!
//! Not to be confused with `entity_name_t` (`denc::EntityName`: type + number). Wire format
//! (`EntityName::encode`): `u32 type` (`CEPH_ENTITY_TYPE_*`) + `string id`.

use bytes::{BufMut, Bytes, BytesMut};
use denc::{Decode, Encode, EntityType};

/// A cephx principal: entity type + id (`mds.rust` has type MDS and id `rust`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthName {
    pub ty: EntityType,
    pub id: String,
}

impl AuthName {
    pub fn new(ty: EntityType, id: impl Into<String>) -> Self {
        AuthName { ty, id: id.into() }
    }

    /// Parse `type.id` (`EntityName::from_str`); the type must be one of `auth`, `mon`, `osd`,
    /// `mds`, `mgr`, `client`.
    pub fn parse(s: &str) -> Option<Self> {
        let (t, id) = s.split_once('.')?;
        let ty = match t {
            "auth" => EntityType::AUTH,
            "mon" => EntityType::MON,
            "osd" => EntityType::OSD,
            "mds" => EntityType::MDS,
            "mgr" => EntityType::MGR,
            "client" => EntityType::CLIENT,
            _ => return None,
        };
        Some(AuthName { ty, id: id.to_string() })
    }
}

impl std::fmt::Display for AuthName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.ty.name(), self.id)
    }
}

impl Encode for AuthName {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        buf.put_u32_le(u32::from(self.ty.0));
        self.id.encode(buf, f);
    }
}

impl Decode for AuthName {
    fn decode(buf: &mut Bytes) -> denc::Result<Self> {
        let ty = u32::decode(buf)?;
        let id = String::decode(buf)?;
        Ok(AuthName { ty: EntityType(ty as u8), id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_display_encode() {
        let n = AuthName::parse("mds.rust").unwrap();
        assert_eq!(n.ty, EntityType::MDS);
        assert_eq!(n.to_string(), "mds.rust");
        // ceph-dencoder type EntityName select_test 3 (mds.a) encode export:
        // 02000000 01000000 61
        let b = denc::encode_to_bytes(&AuthName::new(EntityType::MDS, "a"), 0);
        assert_eq!(hex::encode(&b), "020000000100000061");
        assert_eq!(denc::decode_exact::<AuthName>(b, "name").unwrap(), AuthName::new(EntityType::MDS, "a"));
        assert!(AuthName::parse("bogus.x").is_none());
        assert!(AuthName::parse("mds").is_none());
    }
}
