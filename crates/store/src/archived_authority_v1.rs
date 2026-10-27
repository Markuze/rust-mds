// Archived decoder from published 85a37c8 stable.rs::Authority::decode.
// Keep independent of the new semantic decoder. The writer codec is unchanged.
use super::*;

pub(super) fn decode(bytes: &Bytes) -> Result<(), StoreError> {
    if bytes.len() < 14 || bytes.len() > 8192 {
        return Err(StoreError::Corrupt);
    }
    let end = bytes.len() - 4;
    let mut crc = bytes.slice(end..);
    if crc32c::crc32c(&bytes[..end]) != crc.get_u32_le() {
        return Err(StoreError::Corrupt);
    }
    let mut b = bytes.slice(..end);
    if b.get_u8() != 1 {
        return Err(StoreError::UnsupportedVersion);
    }
    let _revision = b.get_u64_le();
    let _writer = get_writer(&mut b)?;
    if b.is_empty() {
        return Err(StoreError::Corrupt);
    }
    let _predecessor = match b.get_u8() {
        0 => None,
        1 => {
            let writer = get_writer(&mut b)?;
            if b.len() < 4 {
                return Err(StoreError::Corrupt);
            }
            let verified_osd_epoch = b.get_u32_le();
            if verified_osd_epoch == 0 {
                return Err(StoreError::Corrupt);
            }
            Some(FencedWriter {
                writer,
                verified_osd_epoch,
            })
        }
        _ => return Err(StoreError::Corrupt),
    };
    if !b.is_empty() {
        return Err(StoreError::Corrupt);
    }
    Ok(())
}
