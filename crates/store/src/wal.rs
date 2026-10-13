//! WAL segment framing (`design/store.md` section 3).
//!
//! One segment object holds a sequence of framed records:
//!
//! ```text
//! u32 len | u32 crc32c(body) | u64 seq | u64 txid | body
//! ```
//!
//! `seq` is the per-stream sequence of the record (the engine's journal
//! sequence); `txid` carries the same value until transactions grow their
//! own ids. Reading a segment stops at the first record whose length,
//! crc or sequence number is wrong: that is the torn tail of a crash
//! (`design/store.md` 3, "End of log"; no prezeroing, no probing).

use bytes::{Buf, BufMut, Bytes, BytesMut};

use engine::journal::{JournalRecord, JournalSeq};

use crate::record;

/// Header bytes in front of every record body.
pub const HEADER_LEN: usize = 4 + 4 + 8 + 8;

/// One framed record: its per-stream sequence and its encoded body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramedRecord {
    pub seq: JournalSeq,
    pub txid: u64,
    pub body: Bytes,
}

impl FramedRecord {
    /// Frame the encoded `record` with its sequence number.
    pub fn new(seq: JournalSeq, record: &JournalRecord) -> FramedRecord {
        FramedRecord {
            seq,
            txid: seq,
            body: record::encode_record(record),
        }
    }

    /// The framed bytes: header plus body.
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(HEADER_LEN + self.body.len());
        buf.put_u32_le(self.body.len() as u32);
        buf.put_u32_le(crc32c::crc32c(&self.body));
        buf.put_u64_le(self.seq);
        buf.put_u64_le(self.txid);
        buf.put_slice(&self.body);
        buf.freeze()
    }

    /// Decode the record body.
    pub fn decode(&self) -> denc::Result<JournalRecord> {
        record::decode_record(&self.body)
    }
}

/// The result of reading a segment: the records that parse, and how many
/// bytes they span. Bytes beyond `len` are a torn or corrupt tail.
#[derive(Debug, Clone, Default)]
pub struct Segment {
    pub records: Vec<FramedRecord>,
    /// Bytes consumed by `records` (the prefix a rewriter keeps).
    pub len: usize,
}

/// Read framed records from the front of `buf` until the buffer ends or a
/// record is torn or corrupt (bad length, bad crc, or a sequence number
/// that does not continue `first_seq`).
///
/// `first_seq` is the sequence number the first record in the buffer must
/// carry (the stream's checkpoint plus one); each following record must
/// continue it. A gap means the tail is garbage from a deposed owner or a
/// partial write, and the stream ends there.
pub fn read_segment(buf: &Bytes, first_seq: JournalSeq) -> Segment {
    let mut records = Vec::new();
    let mut pos = 0usize;
    let mut expect = first_seq;
    while buf.len() - pos >= HEADER_LEN {
        let mut head = buf.slice(pos..pos + HEADER_LEN);
        let body_len = head.get_u32_le() as usize;
        let crc = head.get_u32_le();
        let seq = head.get_u64_le();
        let txid = head.get_u64_le();
        if seq != expect || body_len == 0 || buf.len() - pos - HEADER_LEN < body_len {
            break;
        }
        let body = buf.slice(pos + HEADER_LEN..pos + HEADER_LEN + body_len);
        if crc32c::crc32c(&body) != crc {
            break;
        }
        records.push(FramedRecord { seq, txid, body });
        pos += HEADER_LEN + body_len;
        expect = seq + 1;
    }
    Segment { records, len: pos }
}
