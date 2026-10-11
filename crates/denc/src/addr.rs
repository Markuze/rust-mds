//! `entity_addr_t` and `entity_addrvec_t`.
//!
//! Two wire forms exist (`src/msg/msg_types.h`, `msg_types.cc` at the reference commit):
//!
//! - **Legacy** (peer lacks `MSG_ADDR2`): `u32 0`, `u32 nonce`, then a 128-byte Linux
//!   `sockaddr_storage` whose `ss_family` is big-endian. The msgr1 banner uses this form too.
//! - **ADDR2**: `u8 1`, then `ENCODE_START(1, 1)` with `u32 type`, `u32 nonce`, `u32 elen` and,
//!   if `elen > 0`, `u16 family` (little-endian) followed by `elen - 2` bytes of `sa_data`
//!   (port big-endian, then the address).
//!
//! An address vector with `MSG_ADDR2` is `u8 2` + `vector<entity_addr_t>`; without it, the
//! legacy form of its `TYPE_LEGACY` entry.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use bytes::{Buf, BufMut};

use crate::{Bytes, BytesMut, Decode, DecodeError, Encode, Result, decode_start, encode_finish, encode_start, features, need};

const AF_INET: u16 = 2;
const AF_INET6: u16 = 10;
const SOCKADDR_STORAGE_LEN: usize = 128;

/// `entity_addr_t::type_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddrType(pub u32);

impl AddrType {
    pub const NONE: AddrType = AddrType(0);
    /// msgr1.
    pub const LEGACY: AddrType = AddrType(1);
    /// msgr2.
    pub const MSGR2: AddrType = AddrType(2);
    pub const ANY: AddrType = AddrType(3);
}

/// `entity_addr_t`: address type, nonce (unique per process instance) and socket address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityAddr {
    pub ty: AddrType,
    pub nonce: u32,
    pub addr: Option<SocketAddr>,
}

impl EntityAddr {
    pub fn new(ty: AddrType, addr: SocketAddr, nonce: u32) -> Self {
        EntityAddr { ty, nonce, addr: Some(addr) }
    }

    /// An unset address (`TYPE_NONE`, no socket address).
    pub fn none() -> Self {
        EntityAddr { ty: AddrType::NONE, nonce: 0, addr: None }
    }

    /// The 128-byte Linux `sockaddr_storage` with a big-endian family, as the legacy encoding
    /// and the msgr1 banner use it.
    pub fn sockaddr_storage_be(&self) -> [u8; SOCKADDR_STORAGE_LEN] {
        let mut ss = [0u8; SOCKADDR_STORAGE_LEN];
        match self.addr {
            Some(SocketAddr::V4(a)) => {
                ss[0..2].copy_from_slice(&AF_INET.to_be_bytes());
                ss[2..4].copy_from_slice(&a.port().to_be_bytes());
                ss[4..8].copy_from_slice(&a.ip().octets());
            }
            Some(SocketAddr::V6(a)) => {
                ss[0..2].copy_from_slice(&AF_INET6.to_be_bytes());
                ss[2..4].copy_from_slice(&a.port().to_be_bytes());
                ss[4..8].copy_from_slice(&a.flowinfo().to_be_bytes());
                ss[8..24].copy_from_slice(&a.ip().octets());
                ss[24..28].copy_from_slice(&a.scope_id().to_le_bytes());
            }
            None => {}
        }
        ss
    }

    /// Parse a 128-byte `sockaddr_storage` whose family is big-endian.
    pub fn parse_sockaddr_storage_be(ss: &[u8]) -> Result<Option<SocketAddr>> {
        if ss.len() < SOCKADDR_STORAGE_LEN {
            return Err(DecodeError::Short { what: "sockaddr_storage", need: SOCKADDR_STORAGE_LEN, have: ss.len() });
        }
        let family = u16::from_be_bytes([ss[0], ss[1]]);
        Self::parse_sockaddr_body(family, &ss[2..])
    }

    /// `sa_data` after the family: port (big-endian) then the address.
    fn parse_sockaddr_body(family: u16, data: &[u8]) -> Result<Option<SocketAddr>> {
        match family {
            0 => Ok(None),
            AF_INET => {
                if data.len() < 6 {
                    return Err(DecodeError::Short { what: "sockaddr_in", need: 6, have: data.len() });
                }
                let port = u16::from_be_bytes([data[0], data[1]]);
                let ip = Ipv4Addr::new(data[2], data[3], data[4], data[5]);
                Ok(Some(SocketAddr::V4(SocketAddrV4::new(ip, port))))
            }
            AF_INET6 => {
                if data.len() < 26 {
                    return Err(DecodeError::Short { what: "sockaddr_in6", need: 26, have: data.len() });
                }
                let port = u16::from_be_bytes([data[0], data[1]]);
                let flow = u32::from_be_bytes([data[2], data[3], data[4], data[5]]);
                let mut oct = [0u8; 16];
                oct.copy_from_slice(&data[6..22]);
                let scope = u32::from_le_bytes([data[22], data[23], data[24], data[25]]);
                Ok(Some(SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::from(oct), port, flow, scope))))
            }
            f => Err(DecodeError::Invalid { what: "sockaddr", detail: format!("unsupported family {f}") }),
        }
    }

    /// Legacy body after the leading `u32 0`: nonce + sockaddr_storage.
    fn decode_legacy_after_marker(buf: &mut Bytes) -> Result<Self> {
        // The legacy encoding starts with `u32 0`; the marker byte was its first byte.
        need(buf, 3 + 4 + SOCKADDR_STORAGE_LEN, "legacy entity_addr")?;
        buf.advance(3);
        let nonce = buf.get_u32_le();
        let ss = buf.split_to(SOCKADDR_STORAGE_LEN);
        let addr = Self::parse_sockaddr_storage_be(&ss)?;
        let ty = if addr.is_some() { AddrType::LEGACY } else { AddrType::NONE };
        Ok(EntityAddr { ty, nonce, addr })
    }

    /// Encode in the legacy form (also what the msgr1 banner carries).
    pub fn encode_legacy(&self, buf: &mut BytesMut) {
        buf.put_u32_le(0);
        buf.put_u32_le(self.nonce);
        buf.put_slice(&self.sockaddr_storage_be());
    }

    /// Decode the legacy form without the ADDR2 marker logic (msgr1 banner and `ceph_entity_addr`).
    pub fn decode_legacy(buf: &mut Bytes) -> Result<Self> {
        need(buf, 1, "legacy entity_addr")?;
        buf.advance(1);
        Self::decode_legacy_after_marker(buf)
    }

    fn sockaddr_len(&self) -> u32 {
        match self.addr {
            Some(SocketAddr::V4(_)) => 16,
            Some(SocketAddr::V6(_)) => 28,
            None => 0,
        }
    }
}

impl Encode for EntityAddr {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        if f & features::MSG_ADDR2 == 0 {
            self.encode_legacy(buf);
            return;
        }
        buf.put_u8(1);
        let s = encode_start(buf, 1, 1);
        let ty = if !features::has(f, features::MASK_SERVER_NAUTILUS) && self.ty == AddrType::ANY {
            AddrType::LEGACY
        } else {
            self.ty
        };
        buf.put_u32_le(ty.0);
        buf.put_u32_le(self.nonce);
        let elen = self.sockaddr_len();
        buf.put_u32_le(elen);
        match self.addr {
            Some(SocketAddr::V4(a)) => {
                buf.put_u16_le(AF_INET);
                buf.put_u16(a.port());
                buf.put_slice(&a.ip().octets());
                buf.put_slice(&[0u8; 8]); // sin_zero
            }
            Some(SocketAddr::V6(a)) => {
                buf.put_u16_le(AF_INET6);
                buf.put_u16(a.port());
                buf.put_u32(a.flowinfo());
                buf.put_slice(&a.ip().octets());
                buf.put_u32_le(a.scope_id());
            }
            None => {}
        }
        encode_finish(buf, s);
    }
}

impl Decode for EntityAddr {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let marker = u8::decode(buf)?;
        match marker {
            0 => Self::decode_legacy_after_marker(buf),
            1 => {
                let mut f = decode_start(buf, 1, "entity_addr_t")?;
                let b = &mut f.body;
                let ty = AddrType(u32::decode(b)?);
                let nonce = u32::decode(b)?;
                let elen = u32::decode(b)? as usize;
                let addr = if elen == 0 {
                    None
                } else {
                    let raw = crate::take(b, elen, "sockaddr")?;
                    if raw.len() < 2 {
                        return Err(DecodeError::Invalid { what: "entity_addr_t", detail: "elen < 2".into() });
                    }
                    let family = u16::from_le_bytes([raw[0], raw[1]]);
                    Self::parse_sockaddr_body(family, &raw[2..])?
                };
                Ok(EntityAddr { ty, nonce, addr })
            }
            m => Err(DecodeError::Invalid { what: "entity_addr_t", detail: format!("marker {m}") }),
        }
    }
}

impl std::fmt::Display for EntityAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match self.ty {
            AddrType::LEGACY => "v1:",
            AddrType::MSGR2 => "v2:",
            AddrType::ANY => "any:",
            _ => "",
        };
        match self.addr {
            Some(a) => write!(f, "{t}{a}/{}", self.nonce),
            None => write!(f, "{t}-/{}", self.nonce),
        }
    }
}

impl EntityAddr {
    pub fn ip(&self) -> Option<IpAddr> {
        self.addr.map(|a| a.ip())
    }
}

/// `entity_addrvec_t`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EntityAddrVec(pub Vec<EntityAddr>);

impl EntityAddrVec {
    /// The `TYPE_LEGACY` entry, as `legacy_addr()` returns it (or an unset address).
    pub fn legacy_addr(&self) -> EntityAddr {
        self.0.iter().copied().find(|a| a.ty == AddrType::LEGACY).unwrap_or_else(EntityAddr::none)
    }

    /// The first entry of the given type.
    pub fn first_of(&self, ty: AddrType) -> Option<EntityAddr> {
        self.0.iter().copied().find(|a| a.ty == ty)
    }
}

impl Encode for EntityAddrVec {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        if f & features::MSG_ADDR2 == 0 {
            self.legacy_addr().encode_legacy(buf);
            return;
        }
        buf.put_u8(2);
        buf.put_u32_le(self.0.len() as u32);
        for a in &self.0 {
            a.encode(buf, f);
        }
    }
}

impl Decode for EntityAddrVec {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        need(buf, 1, "entity_addrvec_t")?;
        match buf[0] {
            0 | 1 => Ok(EntityAddrVec(vec![EntityAddr::decode(buf)?])),
            2 => {
                buf.advance(1);
                Ok(EntityAddrVec(Vec::<EntityAddr>::decode(buf)?))
            }
            m => Err(DecodeError::Invalid { what: "entity_addrvec_t", detail: format!("marker {m}") }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decode_exact, encode_to_bytes};

    fn v4(port: u16) -> SocketAddr {
        "192.168.1.2:0".parse::<SocketAddr>().map(|mut a| {
            a.set_port(port);
            a
        }).unwrap()
    }

    #[test]
    fn legacy_form_is_136_bytes_with_big_endian_family() {
        let a = EntityAddr::new(AddrType::LEGACY, v4(6789), 0x1234);
        let b = encode_to_bytes(&a, 0);
        assert_eq!(b.len(), 136);
        assert_eq!(&b[0..4], &[0, 0, 0, 0]);
        assert_eq!(&b[4..8], &0x1234u32.to_le_bytes());
        assert_eq!(&b[8..10], &[0, 2]); // AF_INET, big-endian
        assert_eq!(&b[10..12], &6789u16.to_be_bytes());
        assert_eq!(&b[12..16], &[192, 168, 1, 2]);
        assert_eq!(decode_exact::<EntityAddr>(b, "addr").unwrap(), a);
    }

    #[test]
    fn addr2_form_round_trips() {
        let a = EntityAddr::new(AddrType::MSGR2, v4(3300), 7);
        let f = features::MSG_ADDR2 | features::MASK_SERVER_NAUTILUS;
        let b = encode_to_bytes(&a, f);
        // marker, v, compat, len, type, nonce, elen=16, family LE, port BE, ip, 8 zero bytes
        assert_eq!(b[0], 1);
        assert_eq!(&b[1..3], &[1, 1]);
        assert_eq!(u32::from_le_bytes(b[3..7].try_into().unwrap()) as usize, b.len() - 7);
        assert_eq!(&b[15..19], &16u32.to_le_bytes());
        assert_eq!(&b[19..21], &2u16.to_le_bytes());
        assert_eq!(decode_exact::<EntityAddr>(b, "addr").unwrap(), a);
    }

    #[test]
    fn addrvec_with_and_without_addr2() {
        let v = EntityAddrVec(vec![
            EntityAddr::new(AddrType::MSGR2, v4(3300), 9),
            EntityAddr::new(AddrType::LEGACY, v4(6789), 9),
        ]);
        let f = features::MSG_ADDR2 | features::MASK_SERVER_NAUTILUS;
        assert_eq!(decode_exact::<EntityAddrVec>(encode_to_bytes(&v, f), "v").unwrap(), v);
        let legacy = decode_exact::<EntityAddrVec>(encode_to_bytes(&v, 0), "v").unwrap();
        assert_eq!(legacy.0, vec![v.0[1]]);
    }
}
