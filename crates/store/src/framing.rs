//! Rust metadata framing, not a C++ metadata format. Every byte except the
//! trailing CRC32C is checksummed, including length, version, identity and owner.

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::stable::{OwnerToken, StoreError, StoreIdentity};

const MAGIC: &[u8; 8] = b"RMDSWAL\0";
const VERSION: u16 = 1;
const HEADER: usize = 8 + 2 + 2 + 4 + 32 + 32 + 8 + 16;
pub const OVERHEAD: usize = HEADER + 4;

/// Bind a descriptor to protected content. Do not checksum a frame including its
/// own CRC: CRC(message || CRC(message)) has a constant polynomial residue.
pub fn content_checksum(frame: &Bytes) -> Result<u32, StoreError> {
    if frame.len() < OVERHEAD {
        return Err(StoreError::Corrupt);
    }
    Ok(crc32c::crc32c(&frame[..frame.len() - 4]))
}

/// Values identify their role; a checkpoint cannot be substituted for a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum FrameKind {
    Record = 1,
    Checkpoint = 2,
    Manifest = 3,
    Bootstrap = 4,
}

pub fn encode(
    kind: FrameKind,
    identity: &StoreIdentity,
    owner: OwnerToken,
    seq: u64,
    transaction: [u8; 16],
    body: &[u8],
    max_bytes: usize,
) -> Result<Bytes, StoreError> {
    let len = body
        .len()
        .checked_add(OVERHEAD)
        .ok_or(StoreError::Capacity)?;
    if len > max_bytes || len > u32::MAX as usize {
        return Err(StoreError::Capacity);
    }
    let mut b = BytesMut::with_capacity(len);
    b.extend_from_slice(MAGIC);
    b.put_u16_le(VERSION);
    b.put_u16_le(kind as u16);
    b.put_u32_le(len as u32);
    b.extend_from_slice(&identity.encode());
    b.extend_from_slice(&owner.encode());
    b.put_u64_le(seq);
    b.extend_from_slice(&transaction);
    b.extend_from_slice(body);
    b.put_u32_le(crc32c::crc32c(&b));
    Ok(b.freeze())
}

pub struct Frame {
    pub owner: OwnerToken,
    pub transaction: [u8; 16],
    pub body: Bytes,
}

pub fn decode(
    bytes: &Bytes,
    kind: FrameKind,
    identity: &StoreIdentity,
    seq: u64,
    max_bytes: usize,
) -> Result<Frame, StoreError> {
    if bytes.len() < OVERHEAD || bytes.len() > max_bytes {
        return Err(StoreError::Corrupt);
    }
    let end = bytes.len() - 4;
    let mut checksum = bytes.slice(end..);
    if crc32c::crc32c(&bytes[..end]) != checksum.get_u32_le() {
        return Err(StoreError::Corrupt);
    }
    let mut b = bytes.slice(..end);
    if &b[..8] != MAGIC {
        return Err(StoreError::WrongFormat);
    }
    b.advance(8);
    if b.get_u16_le() != VERSION {
        return Err(StoreError::UnsupportedVersion);
    }
    if b.get_u16_le() != kind as u16 || b.get_u32_le() as usize != bytes.len() {
        return Err(StoreError::Corrupt);
    }
    if b.split_to(32) != identity.encode() {
        return Err(StoreError::WrongFormat);
    }
    let owner = OwnerToken::decode(&b.split_to(32))?;
    if b.get_u64_le() != seq {
        return Err(StoreError::Corrupt);
    }
    let mut transaction = [0; 16];
    b.copy_to_slice(&mut transaction);
    Ok(Frame {
        owner,
        transaction,
        body: b,
    })
}
