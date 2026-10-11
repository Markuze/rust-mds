//! `MClientRequestForward` (`CEPH_MSG_CLIENT_REQUEST_FORWARD`,
//! `src/messages/MClientRequestForward.h`), version 1, MDS to client.
//!
//! Front: `i32 dest_mds`, `i32 num_fwd`, `bool client_must_resend` (always true); `header.tid`
//! is the request tid. The client resends the request to `dest_mds` with `num_fwd`, which must
//! be strictly greater than the request's previous forward count (else `-EMULTIHOP`). Never
//! forward a tid that already got a reply: that is a kernel `BUG_ON`.

use denc::{BufMut, Bytes, BytesMut, Message, Payload, Result};

use crate::msg_type;
use crate::util::{get, impl_from_raw};

/// `MClientRequestForward`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MClientRequestForward {
    /// The request tid (`header.tid`).
    pub tid: u64,
    pub dest_mds: i32,
    /// The request's forward count plus one.
    pub num_fwd: i32,
    pub client_must_resend: bool,
}

impl MClientRequestForward {
    /// Tell the client to resend request `tid` to `dest_mds` with forward count `num_fwd`
    /// (`MDSRank::forward_message_mds`: the request's `num_fwd + 1`).
    pub fn new(tid: u64, dest_mds: i32, num_fwd: i32) -> Self {
        MClientRequestForward { tid, dest_mds, num_fwd, client_must_resend: true }
    }
}

impl Message for MClientRequestForward {
    const TYPE: u16 = msg_type::CLIENT_REQUEST_FORWARD;
    const HEAD_VERSION: u16 = 1;
    const COMPAT_VERSION: u16 = 0;

    fn encode_payload(&self, _features: u64) -> Payload {
        let mut b = BytesMut::with_capacity(9);
        b.put_i32_le(self.dest_mds);
        b.put_i32_le(self.num_fwd);
        b.put_u8(u8::from(self.client_must_resend));
        Payload { front: b, ..Default::default() }
    }

    fn decode_payload(_version: u16, mut front: Bytes, _middle: Bytes, _data: Bytes) -> Result<Self> {
        let p = &mut front;
        Ok(MClientRequestForward {
            tid: 0,
            dest_mds: get(p, "MClientRequestForward.dest_mds")?,
            num_fwd: get(p, "MClientRequestForward.num_fwd")?,
            client_must_resend: get(p, "MClientRequestForward.client_must_resend")?,
        })
    }

    impl_from_raw!(tid);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes() {
        let m = MClientRequestForward::new(99, 1, 2);
        let raw = m.to_raw(0);
        assert_eq!((raw.tid, raw.version, raw.compat_version), (99, 1, 0));
        assert_eq!(hex::encode(&raw.front), "010000000200000001");
        assert_eq!(MClientRequestForward::from_raw(raw).unwrap(), m);
    }
}
