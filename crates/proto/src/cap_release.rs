//! `MClientCapRelease` (`CEPH_MSG_CLIENT_CAPRELEASE`, `src/messages/MClientCapRelease.h`),
//! version 2 compat 1, client to MDS.
//!
//! Front: raw `ceph_mds_cap_release` (`u32 num`), `num` raw `ceph_mds_cap_item` records
//! (`u64 ino`, `u64 cap_id`, `u32 migrate_seq`, `u32 issue_seq`; 24 bytes each), then (v2)
//! `u32 osd_epoch_barrier`. The kernel sends at most 170 items per message.

use denc::{Buf, BufMut, Bytes, BytesMut, Message, Payload, Result};

use crate::msg_type;
use crate::util::{get, impl_from_raw};

/// `ceph_mds_cap_item`: one released cap. A release whose `migrate_seq` is stale or whose
/// `issue_seq` predates the cap's last issue is ignored by the MDS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CapItem {
    pub ino: u64,
    pub cap_id: u64,
    pub migrate_seq: u32,
    pub issue_seq: u32,
}

impl CapItem {
    pub const LEN: usize = 24;
}

/// `MClientCapRelease`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MClientCapRelease {
    pub caps: Vec<CapItem>,
    /// The newest OSDMap epoch barrier the client saw; the MDS must not let other clients use
    /// these caps' objects before that epoch.
    pub osd_epoch_barrier: u32,
}

impl Message for MClientCapRelease {
    const TYPE: u16 = msg_type::CLIENT_CAPRELEASE;
    const HEAD_VERSION: u16 = 2;
    const COMPAT_VERSION: u16 = 1;

    fn encode_payload(&self, _features: u64) -> Payload {
        let mut b = BytesMut::with_capacity(8 + CapItem::LEN * self.caps.len());
        b.put_u32_le(self.caps.len() as u32);
        for c in &self.caps {
            b.put_u64_le(c.ino);
            b.put_u64_le(c.cap_id);
            b.put_u32_le(c.migrate_seq);
            b.put_u32_le(c.issue_seq);
        }
        b.put_u32_le(self.osd_epoch_barrier);
        Payload { front: b, ..Default::default() }
    }

    fn decode_payload(version: u16, mut front: Bytes, _middle: Bytes, _data: Bytes) -> Result<Self> {
        let p = &mut front;
        let n = get::<u32>(p, "ceph_mds_cap_release.num")? as usize;
        let bytes = n.saturating_mul(CapItem::LEN);
        denc::need(p, bytes, "ceph_mds_cap_item")?;
        let caps = (0..n)
            .map(|_| CapItem {
                ino: p.get_u64_le(),
                cap_id: p.get_u64_le(),
                migrate_seq: p.get_u32_le(),
                issue_seq: p.get_u32_le(),
            })
            .collect();
        let osd_epoch_barrier = if version >= 2 { get(p, "MClientCapRelease.osd_epoch_barrier")? } else { 0 };
        Ok(MClientCapRelease { caps, osd_epoch_barrier })
    }

    impl_from_raw!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_bytes() {
        let m = MClientCapRelease {
            caps: vec![
                CapItem { ino: 0x1000, cap_id: 2, migrate_seq: 0, issue_seq: 3 },
                CapItem { ino: 0x1001, cap_id: 4, migrate_seq: 1, issue_seq: 5 },
            ],
            osd_epoch_barrier: 9,
        };
        let raw = m.to_raw(0);
        assert_eq!((raw.version, raw.compat_version), (2, 1));
        assert_eq!(raw.front.len(), 4 + 48 + 4);
        assert_eq!(
            hex::encode(&raw.front[..28]),
            "02000000".to_owned() + "0010000000000000" + "0200000000000000" + "00000000" + "03000000"
        );
        assert_eq!(MClientCapRelease::from_raw(raw.clone()).unwrap(), m);
        let mut v1 = raw.clone();
        v1.version = 1;
        assert_eq!(MClientCapRelease::from_raw(v1).unwrap().osd_epoch_barrier, 0);
        let short = denc::RawMessage { front: raw.front.slice(..30), ..raw };
        assert!(MClientCapRelease::from_raw(short).is_err());
        // A huge count is refused without allocating.
        let huge =
            denc::RawMessage::new(msg_type::CLIENT_CAPRELEASE, 2, 1, Bytes::from_static(&[0xff, 0xff, 0xff, 0xff]));
        assert!(MClientCapRelease::from_raw(huge).is_err());
    }
}
