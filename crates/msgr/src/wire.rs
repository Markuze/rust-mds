//! msgr1 wire structures (`src/include/msgr.h`; kernel copy `include/linux/ceph/msgr.h`).
//! All integers little-endian, structs packed.
//!
//! | Structure | Size | Fields |
//! |---|---|---|
//! | banner | 9 | `"ceph v027"` |
//! | `ceph_entity_addr` (legacy) | 136 | le32 type (written 0), le32 nonce, sockaddr_storage with big-endian family |
//! | `ceph_msg_connect` | 33 | le64 features, le32 host_type, le32 global_seq, le32 connect_seq, le32 protocol_version, le32 authorizer_protocol, le32 authorizer_len, u8 flags |
//! | `ceph_msg_connect_reply` | 26 | u8 tag, le64 features, le32 global_seq, le32 connect_seq, le32 protocol_version, le32 authorizer_len, u8 flags |
//! | `ceph_msg_header` | 53 | le64 seq, le64 tid, le16 type, le16 priority, le16 version, le32 front_len, le32 middle_len, le32 data_len, le16 data_off, u8 src.type + le64 src.num, le16 compat_version, le16 reserved, le32 crc (crc32c of the first 49 bytes, seed 0) |
//! | `ceph_msg_footer` | 21 | le32 front_crc, middle_crc, data_crc, le64 sig, u8 flags (with `MSG_AUTH`) |
//! | `ceph_msg_footer_old` | 13 | le32 front_crc, middle_crc, data_crc, u8 flags (without `MSG_AUTH`) |

use denc::{EntityName, EntityType};

/// `CEPH_BANNER`.
pub const BANNER: &[u8; 9] = b"ceph v027";
/// Size of a legacy `ceph_entity_addr`.
pub const ADDR_LEN: usize = 136;

/// `CEPH_MSGR_TAG_*`.
pub mod tag {
    pub const READY: u8 = 1;
    pub const RESETSESSION: u8 = 2;
    pub const WAIT: u8 = 3;
    pub const RETRY_SESSION: u8 = 4;
    pub const RETRY_GLOBAL: u8 = 5;
    pub const CLOSE: u8 = 6;
    pub const MSG: u8 = 7;
    pub const ACK: u8 = 8;
    pub const KEEPALIVE: u8 = 9;
    pub const BADPROTOVER: u8 = 10;
    pub const BADAUTHORIZER: u8 = 11;
    pub const FEATURES: u8 = 12;
    pub const SEQ: u8 = 13;
    pub const KEEPALIVE2: u8 = 14;
    pub const KEEPALIVE2_ACK: u8 = 15;
    pub const CHALLENGE_AUTHORIZER: u8 = 16;

    pub fn name(t: u8) -> &'static str {
        match t {
            READY => "READY",
            RESETSESSION => "RESETSESSION",
            WAIT => "WAIT",
            RETRY_SESSION => "RETRY_SESSION",
            RETRY_GLOBAL => "RETRY_GLOBAL",
            CLOSE => "CLOSE",
            MSG => "MSG",
            ACK => "ACK",
            KEEPALIVE => "KEEPALIVE",
            BADPROTOVER => "BADPROTOVER",
            BADAUTHORIZER => "BADAUTHORIZER",
            FEATURES => "FEATURES",
            SEQ => "SEQ",
            KEEPALIVE2 => "KEEPALIVE2",
            KEEPALIVE2_ACK => "KEEPALIVE2_ACK",
            CHALLENGE_AUTHORIZER => "CHALLENGE_AUTHORIZER",
            _ => "?",
        }
    }
}

/// `CEPH_MSG_CONNECT_LOSSY`.
pub const CONNECT_LOSSY: u8 = 1;

/// `CEPH_MSG_FOOTER_COMPLETE`.
pub const FOOTER_COMPLETE: u8 = 1;
/// `CEPH_MSG_FOOTER_NOCRC`.
pub const FOOTER_NOCRC: u8 = 2;
/// `CEPH_MSG_FOOTER_SIGNED`.
pub const FOOTER_SIGNED: u8 = 4;

/// Protocol numbers (`src/include/ceph_fs.h`, `src/mds/MDSDaemon.h`, `src/mon/Monitor.h`,
/// `src/osd/OSD.h`).
pub const CEPH_OSDC_PROTOCOL: u32 = 24;
pub const CEPH_MDSC_PROTOCOL: u32 = 32;
pub const CEPH_MONC_PROTOCOL: u32 = 15;
pub const CEPH_MDS_PROTOCOL: u32 = 36;
pub const CEPH_MON_PROTOCOL: u32 = 13;
pub const CEPH_OSD_PROTOCOL: u32 = 10;

/// `AsyncMessenger::get_proto_version(peer_type, connect)`.
pub fn proto_version(my_type: EntityType, peer_type: EntityType, connect: bool) -> u32 {
    if peer_type == my_type {
        return match my_type {
            EntityType::MDS => CEPH_MDS_PROTOCOL,
            EntityType::MON => CEPH_MON_PROTOCOL,
            EntityType::OSD => CEPH_OSD_PROTOCOL,
            _ => 0,
        };
    }
    match if connect { peer_type } else { my_type } {
        EntityType::OSD => CEPH_OSDC_PROTOCOL,
        EntityType::MDS => CEPH_MDSC_PROTOCOL,
        EntityType::MON => CEPH_MONC_PROTOCOL,
        _ => 0,
    }
}

/// Ceph's `ceph_crc32c(seed, data)`: CRC-32C (Castagnoli) without the initial and final
/// inversion of the standard form, so `ceph_crc32c(0, ..)` is what messages carry.
pub fn ceph_crc32c(seed: u32, data: &[u8]) -> u32 {
    !crc32c::crc32c_append(!seed, data)
}

/// `ceph_msg_connect`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConnectMsg {
    pub features: u64,
    pub host_type: u32,
    pub global_seq: u32,
    pub connect_seq: u32,
    pub protocol_version: u32,
    pub authorizer_protocol: u32,
    pub authorizer_len: u32,
    pub flags: u8,
}

impl ConnectMsg {
    pub const LEN: usize = 33;

    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut b = [0u8; Self::LEN];
        b[0..8].copy_from_slice(&self.features.to_le_bytes());
        b[8..12].copy_from_slice(&self.host_type.to_le_bytes());
        b[12..16].copy_from_slice(&self.global_seq.to_le_bytes());
        b[16..20].copy_from_slice(&self.connect_seq.to_le_bytes());
        b[20..24].copy_from_slice(&self.protocol_version.to_le_bytes());
        b[24..28].copy_from_slice(&self.authorizer_protocol.to_le_bytes());
        b[28..32].copy_from_slice(&self.authorizer_len.to_le_bytes());
        b[32] = self.flags;
        b
    }

    pub fn from_bytes(b: &[u8; Self::LEN]) -> Self {
        ConnectMsg {
            features: u64::from_le_bytes(b[0..8].try_into().unwrap()),
            host_type: u32::from_le_bytes(b[8..12].try_into().unwrap()),
            global_seq: u32::from_le_bytes(b[12..16].try_into().unwrap()),
            connect_seq: u32::from_le_bytes(b[16..20].try_into().unwrap()),
            protocol_version: u32::from_le_bytes(b[20..24].try_into().unwrap()),
            authorizer_protocol: u32::from_le_bytes(b[24..28].try_into().unwrap()),
            authorizer_len: u32::from_le_bytes(b[28..32].try_into().unwrap()),
            flags: b[32],
        }
    }
}

/// `ceph_msg_connect_reply`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConnectReply {
    pub tag: u8,
    pub features: u64,
    pub global_seq: u32,
    pub connect_seq: u32,
    pub protocol_version: u32,
    pub authorizer_len: u32,
    pub flags: u8,
}

impl ConnectReply {
    pub const LEN: usize = 26;

    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut b = [0u8; Self::LEN];
        b[0] = self.tag;
        b[1..9].copy_from_slice(&self.features.to_le_bytes());
        b[9..13].copy_from_slice(&self.global_seq.to_le_bytes());
        b[13..17].copy_from_slice(&self.connect_seq.to_le_bytes());
        b[17..21].copy_from_slice(&self.protocol_version.to_le_bytes());
        b[21..25].copy_from_slice(&self.authorizer_len.to_le_bytes());
        b[25] = self.flags;
        b
    }

    pub fn from_bytes(b: &[u8; Self::LEN]) -> Self {
        ConnectReply {
            tag: b[0],
            features: u64::from_le_bytes(b[1..9].try_into().unwrap()),
            global_seq: u32::from_le_bytes(b[9..13].try_into().unwrap()),
            connect_seq: u32::from_le_bytes(b[13..17].try_into().unwrap()),
            protocol_version: u32::from_le_bytes(b[17..21].try_into().unwrap()),
            authorizer_len: u32::from_le_bytes(b[21..25].try_into().unwrap()),
            flags: b[25],
        }
    }
}

/// `ceph_msg_header`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsgHeader {
    pub seq: u64,
    pub tid: u64,
    pub msg_type: u16,
    pub priority: u16,
    pub version: u16,
    pub front_len: u32,
    pub middle_len: u32,
    pub data_len: u32,
    pub data_off: u16,
    pub src: EntityName,
    pub compat_version: u16,
    pub reserved: u16,
    pub crc: u32,
}

impl MsgHeader {
    pub const LEN: usize = 53;

    /// Encode, computing `crc` over the first 49 bytes.
    pub fn to_bytes(&mut self) -> [u8; Self::LEN] {
        let mut b = [0u8; Self::LEN];
        b[0..8].copy_from_slice(&self.seq.to_le_bytes());
        b[8..16].copy_from_slice(&self.tid.to_le_bytes());
        b[16..18].copy_from_slice(&self.msg_type.to_le_bytes());
        b[18..20].copy_from_slice(&self.priority.to_le_bytes());
        b[20..22].copy_from_slice(&self.version.to_le_bytes());
        b[22..26].copy_from_slice(&self.front_len.to_le_bytes());
        b[26..30].copy_from_slice(&self.middle_len.to_le_bytes());
        b[30..34].copy_from_slice(&self.data_len.to_le_bytes());
        b[34..36].copy_from_slice(&self.data_off.to_le_bytes());
        b[36] = self.src.ty.0;
        b[37..45].copy_from_slice(&self.src.num.to_le_bytes());
        b[45..47].copy_from_slice(&self.compat_version.to_le_bytes());
        b[47..49].copy_from_slice(&self.reserved.to_le_bytes());
        self.crc = ceph_crc32c(0, &b[..49]);
        b[49..53].copy_from_slice(&self.crc.to_le_bytes());
        b
    }

    /// Decode; the caller checks [`crc_ok`](Self::crc_ok).
    pub fn from_bytes(b: &[u8; Self::LEN]) -> Self {
        MsgHeader {
            seq: u64::from_le_bytes(b[0..8].try_into().unwrap()),
            tid: u64::from_le_bytes(b[8..16].try_into().unwrap()),
            msg_type: u16::from_le_bytes(b[16..18].try_into().unwrap()),
            priority: u16::from_le_bytes(b[18..20].try_into().unwrap()),
            version: u16::from_le_bytes(b[20..22].try_into().unwrap()),
            front_len: u32::from_le_bytes(b[22..26].try_into().unwrap()),
            middle_len: u32::from_le_bytes(b[26..30].try_into().unwrap()),
            data_len: u32::from_le_bytes(b[30..34].try_into().unwrap()),
            data_off: u16::from_le_bytes(b[34..36].try_into().unwrap()),
            src: EntityName::new(
                EntityType(b[36]),
                i64::from_le_bytes(b[37..45].try_into().unwrap()),
            ),
            compat_version: u16::from_le_bytes(b[45..47].try_into().unwrap()),
            reserved: u16::from_le_bytes(b[47..49].try_into().unwrap()),
            crc: u32::from_le_bytes(b[49..53].try_into().unwrap()),
        }
    }

    /// Verify the header crc of raw header bytes.
    pub fn crc_ok(b: &[u8; Self::LEN]) -> bool {
        ceph_crc32c(0, &b[..49]) == u32::from_le_bytes(b[49..53].try_into().unwrap())
    }
}

/// `ceph_msg_footer` (also used for the old footer, whose `sig` is 0).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MsgFooter {
    pub front_crc: u32,
    pub middle_crc: u32,
    pub data_crc: u32,
    pub sig: u64,
    pub flags: u8,
}

impl MsgFooter {
    pub const LEN: usize = 21;
    pub const OLD_LEN: usize = 13;

    /// Encode the 21-byte footer, or the 13-byte old footer when `msg_auth` is false.
    pub fn encode(&self, msg_auth: bool, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.front_crc.to_le_bytes());
        out.extend_from_slice(&self.middle_crc.to_le_bytes());
        out.extend_from_slice(&self.data_crc.to_le_bytes());
        if msg_auth {
            out.extend_from_slice(&self.sig.to_le_bytes());
        }
        out.push(self.flags);
    }

    pub fn decode(b: &[u8], msg_auth: bool) -> Self {
        let mut f = MsgFooter {
            front_crc: u32::from_le_bytes(b[0..4].try_into().unwrap()),
            middle_crc: u32::from_le_bytes(b[4..8].try_into().unwrap()),
            data_crc: u32::from_le_bytes(b[8..12].try_into().unwrap()),
            ..Default::default()
        };
        if msg_auth {
            f.sig = u64::from_le_bytes(b[12..20].try_into().unwrap());
            f.flags = b[20];
        } else {
            f.flags = b[12];
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_crc32c_matches_ceph_test_vectors() {
        // src/test/common/test_crc32c.cc (TEST(Crc32c, Small)).
        assert_eq!(ceph_crc32c(0, b"foo bar baz"), 4119623852);
        assert_eq!(ceph_crc32c(1234, b"foo bar baz"), 881700046);
        assert_eq!(ceph_crc32c(0, b"whiz bang boom"), 2360230088);
        assert_eq!(ceph_crc32c(5678, b"whiz bang boom"), 3743019208);
    }

    #[test]
    fn struct_sizes_and_round_trips() {
        let c = ConnectMsg {
            features: 1,
            host_type: 8,
            global_seq: 2,
            connect_seq: 3,
            protocol_version: 32,
            authorizer_protocol: 2,
            authorizer_len: 9,
            flags: 1,
        };
        assert_eq!(ConnectMsg::from_bytes(&c.to_bytes()), c);
        let r = ConnectReply {
            tag: tag::SEQ,
            features: 7,
            global_seq: 1,
            connect_seq: 2,
            protocol_version: 32,
            authorizer_len: 4,
            flags: 0,
        };
        assert_eq!(ConnectReply::from_bytes(&r.to_bytes()), r);
        let mut h = MsgHeader {
            seq: 1,
            tid: 2,
            msg_type: 22,
            priority: 127,
            version: 1,
            front_len: 10,
            middle_len: 0,
            data_len: 0,
            data_off: 0,
            src: EntityName::new(EntityType::MDS, -1),
            compat_version: 1,
            reserved: 0,
            crc: 0,
        };
        let b = h.to_bytes();
        assert!(MsgHeader::crc_ok(&b));
        assert_eq!(MsgHeader::from_bytes(&b), h);
        assert_eq!(&b[37..45], &[0xff; 8]);
        let mut v = Vec::new();
        MsgFooter {
            front_crc: 1,
            middle_crc: 2,
            data_crc: 3,
            sig: 4,
            flags: 5,
        }
        .encode(true, &mut v);
        assert_eq!(v.len(), MsgFooter::LEN);
        assert_eq!(MsgFooter::decode(&v, true).sig, 4);
        v.clear();
        MsgFooter {
            front_crc: 1,
            middle_crc: 2,
            data_crc: 3,
            sig: 4,
            flags: 5,
        }
        .encode(false, &mut v);
        assert_eq!(v.len(), MsgFooter::OLD_LEN);
        assert_eq!(MsgFooter::decode(&v, false).flags, 5);
    }

    #[test]
    fn protocol_versions() {
        assert_eq!(proto_version(EntityType::MDS, EntityType::MON, true), 15);
        assert_eq!(
            proto_version(EntityType::MDS, EntityType::CLIENT, false),
            32
        );
        assert_eq!(proto_version(EntityType::CLIENT, EntityType::MDS, true), 32);
        assert_eq!(proto_version(EntityType::MDS, EntityType::MDS, false), 36);
        assert_eq!(proto_version(EntityType::MDS, EntityType::OSD, true), 24);
        assert_eq!(proto_version(EntityType::MDS, EntityType::MGR, true), 0);
    }
}
