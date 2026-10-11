//! `MClientLease` (`CEPH_MSG_CLIENT_LEASE`, `src/messages/MClientLease.h`), version 1.
//!
//! Front: raw `ceph_mds_lease` (`u8 action`, `u16 mask`, `u64 ino` (the parent directory),
//! `u64 first`, `u64 last`, `u32 seq`, `u32 duration_ms`; 35 bytes) and `string dname`.
//! Trailing bytes are ignored: the kernel's RENEW always sends `NAME_MAX` bytes of name space.
//!
//! The MDS sends only `REVOKE` (libcephfs asserts on anything else); clients send
//! `REVOKE_ACK` (kernel), `RELEASE` (libcephfs) and `RENEW` (kernel). P0 grants no dentry
//! leases, so it only has to tolerate client lease messages. Counted in `push_seq`.

use std::fmt;

use denc::{Buf, BufMut, Bytes, BytesMut, Encode, Message, Payload, Result};

use crate::msg_type;
use crate::types::CEPH_NOSNAP;
use crate::util::{get, impl_from_raw};

/// `CEPH_MDS_LEASE_*` actions.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LeaseAction(pub u8);

impl LeaseAction {
    /// MDS to client.
    pub const REVOKE: LeaseAction = LeaseAction(1);
    /// Client to MDS.
    pub const RELEASE: LeaseAction = LeaseAction(2);
    /// Both directions.
    pub const RENEW: LeaseAction = LeaseAction(3);
    /// Client to MDS.
    pub const REVOKE_ACK: LeaseAction = LeaseAction(4);

    /// `ceph_lease_op_name`.
    pub fn name(self) -> &'static str {
        match self.0 {
            1 => "revoke",
            2 => "release",
            3 => "renew",
            4 => "revoke_ack",
            _ => "???",
        }
    }
}

impl fmt::Debug for LeaseAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// The mask the C++ MDS revokes with (`1 | CEPH_LOCK_DN`, which is 1).
pub const LEASE_REVOKE_MASK: u16 = 1;

/// `MClientLease`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MClientLease {
    pub action: LeaseAction,
    pub mask: u16,
    /// The parent directory's inode number.
    pub ino: u64,
    pub first: u64,
    pub last: u64,
    pub seq: u32,
    pub duration_ms: u32,
    pub dname: Bytes,
}

impl MClientLease {
    /// `REVOKE` of the lease on `dir_ino/dname` (`Locker::revoke_client_leases`): mask 1,
    /// `first` = the directory's first snapid, `last` = `CEPH_NOSNAP`.
    pub fn revoke(dir_ino: u64, dir_first: u64, dname: Bytes, seq: u32) -> Self {
        MClientLease {
            action: LeaseAction::REVOKE,
            mask: LEASE_REVOKE_MASK,
            ino: dir_ino,
            first: dir_first,
            last: CEPH_NOSNAP,
            seq,
            duration_ms: 0,
            dname,
        }
    }
}

impl Message for MClientLease {
    const TYPE: u16 = msg_type::CLIENT_LEASE;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, features: u64) -> Payload {
        let mut b = BytesMut::with_capacity(39 + self.dname.len());
        b.put_u8(self.action.0);
        b.put_u16_le(self.mask);
        b.put_u64_le(self.ino);
        b.put_u64_le(self.first);
        b.put_u64_le(self.last);
        b.put_u32_le(self.seq);
        b.put_u32_le(self.duration_ms);
        self.dname.encode(&mut b, features);
        Payload { front: b, ..Default::default() }
    }

    fn decode_payload(_version: u16, mut front: Bytes, _middle: Bytes, _data: Bytes) -> Result<Self> {
        let p = &mut front;
        denc::need(p, 35, "ceph_mds_lease")?;
        Ok(MClientLease {
            action: LeaseAction(p.get_u8()),
            mask: p.get_u16_le(),
            ino: p.get_u64_le(),
            first: p.get_u64_le(),
            last: p.get_u64_le(),
            seq: p.get_u32_le(),
            duration_ms: p.get_u32_le(),
            dname: get(p, "MClientLease.dname")?,
        })
    }

    impl_from_raw!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoke_bytes_and_round_trip() {
        let m = MClientLease::revoke(1, 2, Bytes::from_static(b"f"), 7);
        let raw = m.to_raw(0);
        assert_eq!(
            hex::encode(&raw.front),
            "01".to_owned()
                + "0100"
                + "0100000000000000"
                + "0200000000000000"
                + "feffffffffffffff"
                + "07000000"
                + "00000000"
                + "0100000066"
        );
        assert_eq!(MClientLease::from_raw(raw).unwrap(), m);
    }

    #[test]
    fn kernel_renew_with_slack() {
        // The kernel's RENEW front is sizeof(ceph_mds_lease) + 4 + NAME_MAX bytes.
        let mut b = BytesMut::new();
        b.put_u8(3);
        b.put_u16_le(0xdead); // uninitialized mask
        b.put_u64_le(1);
        b.put_u64_le(CEPH_NOSNAP);
        b.put_u64_le(CEPH_NOSNAP);
        b.put_u32_le(4);
        b.put_u32_le(0xbeef);
        b.put_u32_le(3);
        b.put_slice(b"abc");
        b.put_bytes(0x5a, 252);
        let m = MClientLease::from_raw(denc::RawMessage::new(msg_type::CLIENT_LEASE, 0, 0, b.freeze())).unwrap();
        assert_eq!(m.action, LeaseAction::RENEW);
        assert_eq!(&m.dname[..], b"abc");
        assert_eq!(m.seq, 4);
    }
}
