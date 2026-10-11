//! The message envelope shared by the messenger and the message encodings.
//!
//! The messenger moves [`RawMessage`]s; message crates convert between typed messages and
//! `RawMessage` through the [`Message`] trait. Header fields that only the transport owns
//! (sequence numbers, crcs, acks) are not part of the envelope.

use crate::{Bytes, BytesMut, DecodeError, EntityName, Result};

/// A message as the transport carries it: header fields plus the three payload segments.
#[derive(Debug, Clone)]
pub struct RawMessage {
    /// `CEPH_MSG_*` / `MSG_*` type.
    pub msg_type: u16,
    /// Encoding version of the payload (`HEAD_VERSION` of the sender).
    pub version: u16,
    /// Oldest version a receiver must understand (`COMPAT_VERSION`).
    pub compat_version: u16,
    /// `CEPH_MSG_PRIO_*`; 127 (default) unless a message says otherwise.
    pub priority: u16,
    /// Transaction id chosen by the sender (requests and their replies share it).
    pub tid: u64,
    /// Data offset hint (`data_off`), used by OSD ops; 0 otherwise.
    pub data_off: u16,
    /// Sender name. Filled in by the messenger on receive; ignored on send.
    pub src: Option<EntityName>,
    pub front: Bytes,
    pub middle: Bytes,
    pub data: Bytes,
}

/// Default message priority (`CEPH_MSG_PRIO_DEFAULT`).
pub const PRIO_DEFAULT: u16 = 127;
/// `CEPH_MSG_PRIO_HIGH`.
pub const PRIO_HIGH: u16 = 196;

impl RawMessage {
    pub fn new(msg_type: u16, version: u16, compat_version: u16, front: Bytes) -> Self {
        RawMessage {
            msg_type,
            version,
            compat_version,
            priority: PRIO_DEFAULT,
            tid: 0,
            data_off: 0,
            src: None,
            front,
            middle: Bytes::new(),
            data: Bytes::new(),
        }
    }
}

/// The three payload segments a typed message encodes to.
#[derive(Debug, Clone, Default)]
pub struct Payload {
    pub front: BytesMut,
    pub middle: Bytes,
    pub data: Bytes,
}

/// A typed message.
pub trait Message: Sized {
    /// `CEPH_MSG_*` type.
    const TYPE: u16;
    /// Version this implementation encodes (`HEAD_VERSION`).
    const HEAD_VERSION: u16;
    /// `COMPAT_VERSION` this implementation advertises when encoding.
    const COMPAT_VERSION: u16;
    /// Priority to send with.
    const PRIORITY: u16 = PRIO_DEFAULT;

    /// Encode for a peer with connection `features`.
    fn encode_payload(&self, features: u64) -> Payload;

    /// Decode a payload of the given header version.
    fn decode_payload(version: u16, front: Bytes, middle: Bytes, data: Bytes) -> Result<Self>;

    /// Transaction id carried in the header, if the message has one.
    fn tid(&self) -> u64 {
        0
    }

    fn to_raw(&self, features: u64) -> RawMessage {
        let p = self.encode_payload(features);
        RawMessage {
            msg_type: Self::TYPE,
            version: Self::HEAD_VERSION,
            compat_version: Self::COMPAT_VERSION,
            priority: Self::PRIORITY,
            tid: self.tid(),
            data_off: 0,
            src: None,
            front: p.front.freeze(),
            middle: p.middle,
            data: p.data,
        }
    }

    fn from_raw(raw: RawMessage) -> Result<Self> {
        if raw.msg_type != Self::TYPE {
            return Err(DecodeError::Invalid {
                what: "message type",
                detail: format!("expected {}, got {}", Self::TYPE, raw.msg_type),
            });
        }
        Self::decode_payload(raw.version, raw.front, raw.middle, raw.data)
    }
}
