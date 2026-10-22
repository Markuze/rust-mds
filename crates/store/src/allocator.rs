//! Global monotonic inode-range reservations. Compare-and-swap on this object's
//! high watermark prevents reuse. A reservation may be wasted before shard WAL
//! publication. Shard ownership cannot be checked on a different RADOS object.

use crate::{
    backend::{Backend, checked},
    stable::{StoreError, StoreIdentity},
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use engine::types::InoRange;
use osdc::{ObjectOperation, ops::cmp_op};
use std::collections::{BTreeMap, BTreeSet};

const OBJECT: &str = "rmds.inoalloc";
const IDENTITY: &str = "rmds.identity";
const HIGH: &str = "rmds.high_watermark";

pub struct InoAllocator<B> {
    backend: B,
    identity: StoreIdentity,
    pending: Option<([u8; 16], u64)>,
}

impl<B: Backend> InoAllocator<B> {
    pub fn new(backend: B, identity: StoreIdentity) -> Self {
        Self {
            backend,
            identity,
            pending: None,
        }
    }

    /// The caller retains reservation_id across uncertainty and restart. A
    /// different count under the same id is corruption, never a new reservation.
    pub async fn reserve_range(
        &mut self,
        reservation_id: [u8; 16],
        count: u64,
    ) -> Result<InoRange, StoreError> {
        if count == 0 {
            return Err(StoreError::Capacity);
        }
        if self.pending.is_some_and(|p| p != (reservation_id, count)) {
            return Err(StoreError::UnknownOutcome);
        }
        self.pending = Some((reservation_id, count));
        let mut op = ObjectOperation::new();
        op.getxattr(IDENTITY).getxattr(HIGH);
        let r = checked(&self.backend, OBJECT, op).await?;
        if r.out(0) != self.identity.encode() {
            return Err(StoreError::WrongFormat);
        }
        let high_bytes = r.out(1);
        let high = std::str::from_utf8(&high_bytes)
            .map_err(|_| StoreError::Corrupt)?
            .parse::<u64>()
            .map_err(|_| StoreError::Corrupt)?;
        if high.to_string().as_bytes() != high_bytes.as_ref() || !(0x1000..=1 << 48).contains(&high)
        {
            return Err(StoreError::Corrupt);
        }
        let key = reservation_key(reservation_id);
        let mut op = ObjectOperation::new();
        op.cmpxattr_str(IDENTITY, cmp_op::EQ, &self.identity.encode());
        op.cmpxattr_str(HIGH, cmp_op::EQ, &high_bytes);
        op.omap_get_vals_by_keys(&[key.clone()].into_iter().collect::<BTreeSet<_>>());
        let r = checked(&self.backend, OBJECT, op).await?;
        let mut values = r.out(2);
        if values.len() < 4 {
            return Err(StoreError::Corrupt);
        }
        let n = values.get_u32_le();
        if n > 1 {
            return Err(StoreError::Corrupt);
        }
        if n == 1 {
            if take(&mut values, 34)? != key {
                return Err(StoreError::Corrupt);
            }
            let range =
                decode_reservation(&take(&mut values, 128)?, &self.identity, reservation_id)?;
            if !values.is_empty()
                || range.len != count
                || range
                    .start
                    .checked_add(range.len)
                    .is_none_or(|end| end > high)
            {
                return Err(StoreError::Corrupt);
            }
            self.pending = None;
            return Ok(range);
        }
        if !values.is_empty() {
            return Err(StoreError::Corrupt);
        }
        let Some(end) = high.checked_add(count) else {
            self.pending = None;
            return Err(StoreError::Capacity);
        };
        if end > (1 << 48) {
            self.pending = None;
            return Err(StoreError::Capacity);
        }
        let range = InoRange {
            start: high,
            len: count,
        };
        let mut op = ObjectOperation::new();
        op.cmpxattr_str(IDENTITY, cmp_op::EQ, &self.identity.encode());
        op.cmpxattr_str(HIGH, cmp_op::EQ, &high_bytes);
        op.omap_set_vals(
            &[(
                key,
                encode_reservation(&self.identity, reservation_id, range),
            )]
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
        );
        op.setxattr(HIGH, Bytes::from(end.to_string()));
        checked(&self.backend, OBJECT, op).await?;
        self.pending = None;
        Ok(range)
    }
}

pub(crate) fn reservation_key(id: [u8; 16]) -> Bytes {
    Bytes::from(format!(
        "a/{}",
        id.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
pub(crate) fn encode_reservation(identity: &StoreIdentity, id: [u8; 16], range: InoRange) -> Bytes {
    let mut b = BytesMut::new();
    b.put_u8(1);
    b.extend_from_slice(&identity.encode());
    b.extend_from_slice(&id);
    b.put_u64_le(range.start);
    b.put_u64_le(range.len);
    b.put_u32_le(crc32c::crc32c(&b));
    b.freeze()
}
fn decode_reservation(
    bytes: &Bytes,
    identity: &StoreIdentity,
    id: [u8; 16],
) -> Result<InoRange, StoreError> {
    if bytes.len() != 69 {
        return Err(StoreError::Corrupt);
    }
    let mut checksum = bytes.slice(65..);
    if crc32c::crc32c(&bytes[..65]) != checksum.get_u32_le() {
        return Err(StoreError::Corrupt);
    }
    let mut b = bytes.clone();
    if b.get_u8() != 1 {
        return Err(StoreError::UnsupportedVersion);
    }
    if b.split_to(32) != identity.encode() || b.split_to(16).as_ref() != id {
        return Err(StoreError::WrongFormat);
    }
    let range = InoRange {
        start: b.get_u64_le(),
        len: b.get_u64_le(),
    };
    if range.start < 0x1000
        || range.len == 0
        || range
            .start
            .checked_add(range.len)
            .is_none_or(|end| end > 1 << 48)
    {
        return Err(StoreError::Corrupt);
    }
    Ok(range)
}
fn take(b: &mut Bytes, max: usize) -> Result<Bytes, StoreError> {
    if b.len() < 4 {
        return Err(StoreError::Corrupt);
    }
    let n = b.get_u32_le() as usize;
    if n > max || n > b.len() {
        return Err(StoreError::Corrupt);
    }
    Ok(b.split_to(n))
}
