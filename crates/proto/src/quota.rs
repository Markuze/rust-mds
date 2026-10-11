//! `MClientQuota` (`CEPH_MSG_CLIENT_QUOTA`, `src/messages/MClientQuota.h`), version 1, MDS to
//! client, counted in `push_seq`.
//!
//! Front: `u64 ino`, `utime rctime`, `i64 rbytes`, `i64 rfiles`, `i64 rsubdirs`, `quota_info_t`
//! (`ENCODE_START(1, 1)` { `i64 max_bytes`, `i64 max_files` }); 62 bytes, nothing after (the
//! kernel reads fixed offsets, libcephfs asserts the end). Sent to cap holders of a quota-realm
//! inode when its rstats or quota change.

use denc::{Buf, BufMut, Bytes, BytesMut, Decode, Encode, Message, Payload, Result, UTime};

use crate::msg_type;
use crate::types::QuotaInfo;
use crate::util::{Ctx, get, impl_from_raw, invalid};

/// `MClientQuota`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MClientQuota {
    pub ino: u64,
    pub rctime: UTime,
    pub rbytes: i64,
    pub rfiles: i64,
    pub rsubdirs: i64,
    pub quota: QuotaInfo,
}

impl MClientQuota {
    /// Encoded front size.
    pub const LEN: usize = 8 + 8 + 24 + 22;
}

impl Message for MClientQuota {
    const TYPE: u16 = msg_type::CLIENT_QUOTA;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, features: u64) -> Payload {
        let mut b = BytesMut::with_capacity(Self::LEN);
        b.put_u64_le(self.ino);
        self.rctime.encode(&mut b, features);
        b.put_i64_le(self.rbytes);
        b.put_i64_le(self.rfiles);
        b.put_i64_le(self.rsubdirs);
        self.quota.encode(&mut b, features);
        Payload { front: b, ..Default::default() }
    }

    fn decode_payload(_version: u16, mut front: Bytes, _middle: Bytes, _data: Bytes) -> Result<Self> {
        let p = &mut front;
        let m = MClientQuota {
            ino: get(p, "MClientQuota.ino")?,
            rctime: get(p, "MClientQuota.rctime")?,
            rbytes: get(p, "MClientQuota.rbytes")?,
            rfiles: get(p, "MClientQuota.rfiles")?,
            rsubdirs: get(p, "MClientQuota.rsubdirs")?,
            quota: QuotaInfo::decode(p).ctx("MClientQuota.quota")?,
        };
        if p.has_remaining() {
            return Err(invalid("MClientQuota", format!("{} trailing bytes", p.remaining())));
        }
        Ok(m)
    }

    impl_from_raw!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_layout() {
        let m = MClientQuota {
            ino: 0x1000,
            rctime: UTime { sec: 1, nsec: 2 },
            rbytes: 3,
            rfiles: 4,
            rsubdirs: 5,
            quota: QuotaInfo { max_bytes: 6, max_files: 7 },
        };
        let raw = m.to_raw(0);
        assert_eq!(raw.front.len(), MClientQuota::LEN);
        // The kernel's ceph_mds_quota: ino, rctime, rbytes, rfiles, rsubdirs, u8 v, u8 compat,
        // u32 len, max_bytes, max_files.
        assert_eq!(&raw.front[40..46], &[1, 1, 16, 0, 0, 0]);
        assert_eq!(&raw.front[46..54], &6i64.to_le_bytes());
        assert_eq!(MClientQuota::from_raw(raw.clone()).unwrap(), m);
        let mut f = BytesMut::from(&raw.front[..]);
        f.put_u8(0);
        assert!(MClientQuota::decode_payload(1, f.freeze(), Bytes::new(), Bytes::new()).is_err());
    }
}
