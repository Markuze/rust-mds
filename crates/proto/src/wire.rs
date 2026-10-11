//! Message framing helpers for tests and tools (not used by the daemon's data path):
//!
//! - the container `encode_message`/`decode_message` use (`src/msg/Message.cc`), which is what
//!   `ceph-dencoder type MClient... import <file> decode` reads: raw `ceph_msg_header` (53
//!   bytes), raw `ceph_msg_footer_old` (13 bytes), then front, middle and data as length-prefixed
//!   buffers;
//! - a msgr1 (protocol v1, `src/msg/async/ProtocolV1.cc`) conversation parser that extracts the
//!   messages of both directions of a captured TCP connection, for golden tests against real
//!   C++ MDS traffic. It follows the handshake (banner, addresses, connect / connect-reply
//!   rounds, the optional `TAG_SEQ` exchange) and then the tagged stream (`MSG`, `ACK`,
//!   `KEEPALIVE`, `KEEPALIVE2(_ACK)`, `CLOSE`). CRCs and signatures are not checked.

use denc::{BufMut, Bytes, BytesMut, DecodeError, EntityName, EntityType, RawMessage, Result};

use crate::util::invalid;

/// `CEPH_BANNER`.
pub const BANNER: &[u8] = b"ceph v027";
/// Legacy `ceph_entity_addr` size in the msgr1 banner.
const ADDR_LEN: usize = 136;
const CONNECT_LEN: usize = 33;
const CONNECT_REPLY_LEN: usize = 26;

/// msgr1 tags (`CEPH_MSGR_TAG_*`, `include/msgr.h`).
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
}

/// `ceph_msg_header` (53 raw bytes).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    pub src_type: u8,
    pub src_num: u64,
    pub compat_version: u16,
    pub reserved: u16,
    pub crc: u32,
}

impl MsgHeader {
    pub const LEN: usize = 53;

    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < Self::LEN {
            return Err(DecodeError::Short { what: "ceph_msg_header", need: Self::LEN, have: b.len() });
        }
        let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        let u64_at = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        Ok(MsgHeader {
            seq: u64_at(0),
            tid: u64_at(8),
            msg_type: u16_at(16),
            priority: u16_at(18),
            version: u16_at(20),
            front_len: u32_at(22),
            middle_len: u32_at(26),
            data_len: u32_at(30),
            data_off: u16_at(34),
            src_type: b[36],
            src_num: u64_at(37),
            compat_version: u16_at(45),
            reserved: u16_at(47),
            crc: u32_at(49),
        })
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u64_le(self.seq);
        buf.put_u64_le(self.tid);
        buf.put_u16_le(self.msg_type);
        buf.put_u16_le(self.priority);
        buf.put_u16_le(self.version);
        buf.put_u32_le(self.front_len);
        buf.put_u32_le(self.middle_len);
        buf.put_u32_le(self.data_len);
        buf.put_u16_le(self.data_off);
        buf.put_u8(self.src_type);
        buf.put_u64_le(self.src_num);
        buf.put_u16_le(self.compat_version);
        buf.put_u16_le(self.reserved);
        buf.put_u32_le(self.crc);
    }

    /// The envelope for these segments.
    pub fn to_raw(&self, front: Bytes, middle: Bytes, data: Bytes) -> RawMessage {
        RawMessage {
            msg_type: self.msg_type,
            version: self.version,
            compat_version: self.compat_version,
            priority: self.priority,
            tid: self.tid,
            data_off: self.data_off,
            src: Some(EntityName::new(EntityType(self.src_type), self.src_num as i64)),
            front,
            middle,
            data,
        }
    }

    pub fn from_raw(raw: &RawMessage, seq: u64) -> Self {
        let src = raw.src.unwrap_or(EntityName::new(EntityType::MDS, 0));
        MsgHeader {
            seq,
            tid: raw.tid,
            msg_type: raw.msg_type,
            priority: raw.priority,
            version: raw.version,
            front_len: raw.front.len() as u32,
            middle_len: raw.middle.len() as u32,
            data_len: raw.data.len() as u32,
            data_off: raw.data_off,
            src_type: src.ty.0,
            src_num: src.num as u64,
            compat_version: raw.compat_version,
            reserved: 0,
            crc: 0,
        }
    }
}

/// The `encode_message` container (`src/msg/Message.cc`): header, old footer (CRCs 0, flags
/// `CEPH_MSG_FOOTER_COMPLETE`), then front, middle and data each as `u32 len` + bytes. Feed it
/// to `ceph-dencoder type <MClientX> import <file> decode dump_json`.
pub fn encode_message(raw: &RawMessage) -> Bytes {
    let mut b = BytesMut::with_capacity(MsgHeader::LEN + 13 + 12 + raw.front.len() + raw.middle.len() + raw.data.len());
    MsgHeader::from_raw(raw, 0).encode(&mut b);
    b.put_u32_le(0);
    b.put_u32_le(0);
    b.put_u32_le(0);
    b.put_u8(1);
    for seg in [&raw.front, &raw.middle, &raw.data] {
        b.put_u32_le(seg.len() as u32);
        b.put_slice(seg);
    }
    b.freeze()
}

/// Parse the [`encode_message`] container (as `ceph-dencoder ... encode export` writes it).
pub fn decode_message(bytes: &[u8]) -> Result<RawMessage> {
    let h = MsgHeader::decode(bytes)?;
    let mut off = MsgHeader::LEN + 13;
    let mut segs = Vec::with_capacity(3);
    for what in ["front", "middle", "data"] {
        let len = bytes
            .get(off..off + 4)
            .map(|x| u32::from_le_bytes(x.try_into().unwrap()) as usize)
            .ok_or(DecodeError::Short { what, need: off + 4, have: bytes.len() })?;
        off += 4;
        let seg = bytes.get(off..off + len).ok_or(DecodeError::Short { what, need: off + len, have: bytes.len() })?;
        segs.push(Bytes::copy_from_slice(seg));
        off += len;
    }
    let data = segs.pop().unwrap();
    let middle = segs.pop().unwrap();
    let front = segs.pop().unwrap();
    Ok(h.to_raw(front, middle, data))
}

/// One message read from a msgr1 stream.
#[derive(Clone, Debug)]
pub struct Msgr1Message {
    pub header: MsgHeader,
    pub raw: RawMessage,
}

/// Both directions of a parsed msgr1 connection.
#[derive(Clone, Debug, Default)]
pub struct Msgr1Conversation {
    /// The features of the server's final connect reply (the session features).
    pub features: u64,
    /// The client's `host_type` from its connect (`CEPH_ENTITY_TYPE_*`).
    pub client_type: u32,
    pub from_client: Vec<Msgr1Message>,
    pub from_server: Vec<Msgr1Message>,
    /// A stream ended in the middle of a frame (capture cut off).
    pub truncated: bool,
}

struct Cursor<'a> {
    b: &'a [u8],
    off: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8]> {
        let s = self.b.get(self.off..self.off + n).ok_or(DecodeError::Short {
            what,
            need: n,
            have: self.b.len() - self.off.min(self.b.len()),
        })?;
        self.off += n;
        Ok(s)
    }
    fn u8(&mut self, what: &'static str) -> Result<u8> {
        Ok(self.take(1, what)?[0])
    }
    fn u32_at(s: &[u8], o: usize) -> u32 {
        u32::from_le_bytes(s[o..o + 4].try_into().unwrap())
    }
    fn u64_at(s: &[u8], o: usize) -> u64 {
        u64::from_le_bytes(s[o..o + 8].try_into().unwrap())
    }
    fn at_end(&self) -> bool {
        self.off >= self.b.len()
    }
}

/// Parse a captured msgr1 connection: the bytes the client sent and the bytes the server sent.
pub fn parse_msgr1(client_to_server: &[u8], server_to_client: &[u8]) -> Result<Msgr1Conversation> {
    let mut c = Cursor { b: client_to_server, off: 0 };
    let mut s = Cursor { b: server_to_client, off: 0 };
    if c.take(BANNER.len(), "client banner")? != BANNER || s.take(BANNER.len(), "server banner")? != BANNER {
        return Err(invalid("msgr1", "missing ceph v027 banner"));
    }
    s.take(2 * ADDR_LEN, "server addresses")?;
    c.take(ADDR_LEN, "client address")?;
    let mut conv = Msgr1Conversation::default();
    // Connect rounds until READY or SEQ.
    loop {
        let connect = c.take(CONNECT_LEN, "ceph_msg_connect")?;
        conv.client_type = Cursor::u32_at(connect, 8);
        let auth_len = Cursor::u32_at(connect, 28) as usize;
        c.take(auth_len, "connect authorizer")?;
        let reply = s.take(CONNECT_REPLY_LEN, "ceph_msg_connect_reply")?;
        let rtag = reply[0];
        conv.features = Cursor::u64_at(reply, 1);
        let reply_auth_len = Cursor::u32_at(reply, 21) as usize;
        s.take(reply_auth_len, "connect reply authorizer")?;
        match rtag {
            tag::READY => break,
            tag::SEQ => {
                s.take(8, "server seq")?;
                c.take(8, "client seq")?;
                break;
            }
            tag::RESETSESSION
            | tag::WAIT
            | tag::RETRY_SESSION
            | tag::RETRY_GLOBAL
            | tag::BADAUTHORIZER
            | tag::CHALLENGE_AUTHORIZER => {
                continue;
            }
            t => return Err(invalid("msgr1", format!("connect reply tag {t}"))),
        }
    }
    let footer_len = if conv.features & denc::features::MSG_AUTH != 0 { 21 } else { 13 };
    for (cur, out) in [(&mut c, &mut conv.from_client), (&mut s, &mut conv.from_server)] {
        match read_tagged(cur, footer_len, out) {
            Ok(()) => {}
            Err(DecodeError::Short { .. }) => conv.truncated = true,
            Err(e) => return Err(e),
        }
    }
    Ok(conv)
}

fn read_tagged(cur: &mut Cursor<'_>, footer_len: usize, out: &mut Vec<Msgr1Message>) -> Result<()> {
    while !cur.at_end() {
        match cur.u8("tag")? {
            tag::MSG => {
                let header = MsgHeader::decode(cur.take(MsgHeader::LEN, "ceph_msg_header")?)?;
                let front = Bytes::copy_from_slice(cur.take(header.front_len as usize, "front")?);
                let middle = Bytes::copy_from_slice(cur.take(header.middle_len as usize, "middle")?);
                let data = Bytes::copy_from_slice(cur.take(header.data_len as usize, "data")?);
                cur.take(footer_len, "footer")?;
                let raw = header.to_raw(front, middle, data);
                out.push(Msgr1Message { header, raw });
            }
            tag::ACK => {
                cur.take(8, "ack seq")?;
            }
            tag::KEEPALIVE => {}
            tag::KEEPALIVE2 | tag::KEEPALIVE2_ACK => {
                cur.take(8, "keepalive stamp")?;
            }
            tag::CLOSE => return Ok(()),
            t => return Err(invalid("msgr1", format!("unexpected tag {t} at offset {}", cur.off - 1))),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::MClientSession;
    use denc::Message;

    #[test]
    fn dencoder_container_round_trip() {
        let mut raw = MClientSession::renewcaps(3).to_raw(0);
        raw.tid = 5;
        raw.src = Some(EntityName::new(EntityType::MDS, 0));
        let b = encode_message(&raw);
        assert_eq!(&b[16..18], &22u16.to_le_bytes());
        assert_eq!(b.len(), 53 + 13 + 4 + raw.front.len() + 8);
        let back = decode_message(&b).unwrap();
        assert_eq!(back.front, raw.front);
        assert_eq!((back.msg_type, back.version, back.tid), (22, 1, 5));
    }

    fn frame(raw: &RawMessage, seq: u64, out: &mut BytesMut) {
        out.put_u8(tag::MSG);
        MsgHeader::from_raw(raw, seq).encode(out);
        out.put_slice(&raw.front);
        out.put_slice(&raw.middle);
        out.put_slice(&raw.data);
        out.put_bytes(0, 21);
    }

    #[test]
    fn synthetic_conversation() {
        let mut c = BytesMut::new();
        let mut s = BytesMut::new();
        c.put_slice(BANNER);
        s.put_slice(BANNER);
        s.put_bytes(0, 2 * ADDR_LEN);
        c.put_bytes(0, ADDR_LEN);
        // First connect gets a challenge, the second READY.
        for (i, rtag) in [tag::CHALLENGE_AUTHORIZER, tag::READY].into_iter().enumerate() {
            c.put_u64_le(u64::MAX);
            c.put_u32_le(8); // client
            c.put_u32_le(1);
            c.put_u32_le(0);
            c.put_u32_le(32);
            c.put_u32_le(2);
            c.put_u32_le(10 + i as u32);
            c.put_u8(0);
            c.put_bytes(0xaa, 10 + i);
            s.put_u8(rtag);
            s.put_u64_le(denc::features::ALL);
            s.put_u32_le(1);
            s.put_u32_le(0);
            s.put_u32_le(32);
            s.put_u32_le(4);
            s.put_u8(0);
            s.put_bytes(0xbb, 4);
        }
        let req = MClientSession::new(crate::session::SessionOp::REQUEST_OPEN, 0).to_raw(0);
        frame(&req, 1, &mut c);
        c.put_u8(tag::KEEPALIVE2);
        c.put_bytes(0, 8);
        s.put_u8(tag::ACK);
        s.put_u64_le(1);
        let rep = MClientSession::open_default(0, vec![]).to_raw(0);
        frame(&rep, 1, &mut s);
        s.put_slice(&[tag::MSG, 1, 2]); // cut off
        let conv = parse_msgr1(&c, &s).unwrap();
        assert_eq!(conv.client_type, 8);
        assert_eq!(conv.from_client.len(), 1);
        assert_eq!(conv.from_server.len(), 1);
        assert!(conv.truncated);
        let m = MClientSession::from_raw(conv.from_server[0].raw.clone()).unwrap();
        assert_eq!(m.supported_features.first_word(), crate::features::MDS_SUPPORTED_FEATURES);
    }
}
