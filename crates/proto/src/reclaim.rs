//! `MClientReclaim` / `MClientReclaimReply` (`CEPH_MSG_CLIENT_RECLAIM(_REPLY)`,
//! `src/messages/MClientReclaim.h`, `MClientReclaimReply.h`), both version 1 compat 1.
//!
//! NFS-Ganesha (through `ceph_start_reclaim`) asks the MDS to take over the state of a previous
//! session with the same `uuid` metadata. Reclaim: `string uuid`, `u32 flags`
//! (`CEPH_RECLAIM_RESET` = 1, `FLAG_FINISH` = 1 << 31; only RESET is supported and FINISH must be
//! alone). Reply: `i32 result` (raw errno, not converted), `u32 epoch` (OSDMap epoch after
//! evicting the old session), `entity_addrvec_t addrs` (the old session's address; empty when
//! no session had the uuid).

use denc::{BufMut, Bytes, BytesMut, Decode, Encode, EntityAddrVec, Message, Payload, Result};

use crate::msg_type;
use crate::util::{Ctx, get, get_string_lossy, impl_from_raw};

/// `CEPH_RECLAIM_RESET`.
pub const CEPH_RECLAIM_RESET: u32 = 1;
/// `MClientReclaim::FLAG_FINISH`.
pub const RECLAIM_FLAG_FINISH: u32 = 1 << 31;

/// `MClientReclaim`, client to MDS.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MClientReclaim {
    pub uuid: String,
    pub flags: u32,
}

impl Message for MClientReclaim {
    const TYPE: u16 = msg_type::CLIENT_RECLAIM;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 1;

    fn encode_payload(&self, f: u64) -> Payload {
        let mut b = BytesMut::new();
        self.uuid.encode(&mut b, f);
        b.put_u32_le(self.flags);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(
        _version: u16,
        mut front: Bytes,
        _middle: Bytes,
        _data: Bytes,
    ) -> Result<Self> {
        let p = &mut front;
        Ok(MClientReclaim {
            uuid: get_string_lossy(p, "MClientReclaim.uuid")?,
            flags: get(p, "MClientReclaim.flags")?,
        })
    }

    impl_from_raw!();
}

/// `MClientReclaimReply`, MDS to client.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MClientReclaimReply {
    /// 0 or a negative errno (raw `i32`).
    pub result: i32,
    pub epoch: u32,
    pub addrs: EntityAddrVec,
}

impl Message for MClientReclaimReply {
    const TYPE: u16 = msg_type::CLIENT_RECLAIM_REPLY;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 1;

    /// The address vector uses the connection `features` (`MSG_ADDR2` form when present).
    fn encode_payload(&self, features: u64) -> Payload {
        let mut b = BytesMut::new();
        b.put_i32_le(self.result);
        b.put_u32_le(self.epoch);
        self.addrs.encode(&mut b, features);
        Payload {
            front: b,
            ..Default::default()
        }
    }

    fn decode_payload(
        _version: u16,
        mut front: Bytes,
        _middle: Bytes,
        _data: Bytes,
    ) -> Result<Self> {
        let p = &mut front;
        Ok(MClientReclaimReply {
            result: get(p, "MClientReclaimReply.result")?,
            epoch: get(p, "MClientReclaimReply.epoch")?,
            addrs: EntityAddrVec::decode(p).ctx("MClientReclaimReply.addrs")?,
        })
    }

    impl_from_raw!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::{AddrType, EntityAddr, features};

    #[test]
    fn reclaim_round_trip() {
        let m = MClientReclaim {
            uuid: "ganesha-node1-0042".into(),
            flags: CEPH_RECLAIM_RESET,
        };
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version), (1, 1));
        assert_eq!(&raw.front[raw.front.len() - 4..], &1u32.to_le_bytes());
        assert_eq!(MClientReclaim::from_raw(raw).unwrap(), m);
    }

    #[test]
    fn reply_round_trip() {
        let f = features::MSG_ADDR2 | features::MASK_SERVER_NAUTILUS;
        let a = EntityAddr::new(AddrType::MSGR2, "10.0.0.5:0".parse().unwrap(), 77);
        let m = MClientReclaimReply {
            result: 0,
            epoch: 12,
            addrs: EntityAddrVec(vec![a]),
        };
        assert_eq!(MClientReclaimReply::from_raw(m.to_raw(f)).unwrap(), m);
        // No session with that uuid: result 0, no address.
        let none = MClientReclaimReply::default();
        let raw = none.to_raw(f);
        assert_eq!(
            hex::encode(&raw.front),
            "00000000".to_owned() + "00000000" + "02" + "00000000"
        );
        assert_eq!(MClientReclaimReply::from_raw(raw).unwrap(), none);
    }
}
