//! Decoding helpers that `denc` does not provide (implemented locally, see the crate docs).

use denc::{Buf, Bytes, Decode, DecodeError, DecodeStart, Result, decode_start, need};
#[cfg(test)]
use denc::{BufMut, BytesMut};

/// `DECODE_START_LEGACY_COMPAT_LEN(supported, compatv, lenv, bl)` for encodings whose modern form
/// (struct_v >= `lenv`) is the normal `ENCODE_START` frame. Pre-`lenv` encodings come from
/// releases older than anything that can talk to us (and the monitor re-encodes maps for our
/// features), so they are rejected instead of decoded.
pub(crate) fn decode_start_legacy(
    buf: &mut Bytes,
    supported: u8,
    lenv: u8,
    what: &'static str,
) -> Result<DecodeStart> {
    need(buf, 1, what)?;
    let v = buf[0];
    if v < lenv {
        return Err(DecodeError::Invalid {
            what,
            detail: format!("legacy encoding v{v} (< {lenv}) not supported"),
        });
    }
    decode_start(buf, supported, what)
}

/// `denc_varint`: little-endian base-128 (7 bits per byte, high bit = more).
pub(crate) fn decode_varint(buf: &mut Bytes) -> Result<u64> {
    let mut v = 0u64;
    let mut shift = 0u32;
    loop {
        let b = u8::decode(buf)?;
        if shift >= 64 {
            return Err(DecodeError::Invalid {
                what: "varint",
                detail: "too long".into(),
            });
        }
        v |= u64::from(b & 0x7f) << shift;
        if b & 0x80 == 0 {
            return Ok(v);
        }
        shift += 7;
    }
}

/// `denc_varint` encoder.
#[cfg(test)]
pub(crate) fn encode_varint(mut v: u64, buf: &mut BytesMut) {
    let mut byte = (v & 0x7f) as u8;
    v >>= 7;
    while v != 0 {
        buf.put_u8(byte | 0x80);
        byte = (v & 0x7f) as u8;
        v >>= 7;
    }
    buf.put_u8(byte);
}

/// `float` (IEEE-754 single precision, little-endian).
pub(crate) fn decode_f32(buf: &mut Bytes) -> Result<f32> {
    Ok(f32::from_bits(u32::decode(buf)?))
}

/// Skip `n` bytes.
pub(crate) fn skip(buf: &mut Bytes, n: usize, what: &'static str) -> Result<()> {
    need(buf, n, what)?;
    buf.advance(n);
    Ok(())
}

/// The raw CRC32C Ceph uses (`ceph_crc32c(seed, data)`): no pre/post inversion.
pub fn ceph_crc32c(seed: u32, data: &[u8]) -> u32 {
    // crc32c::crc32c_append(c, d) computes !update(!c, d); undo both inversions.
    !crc32c::crc32c_append(!seed, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_round_trip() {
        for v in [0u64, 1, 127, 128, 300, u32::MAX as u64, u64::MAX] {
            let mut b = BytesMut::new();
            encode_varint(v, &mut b);
            let mut bytes = b.freeze();
            assert_eq!(decode_varint(&mut bytes).unwrap(), v);
            assert!(bytes.is_empty());
        }
        let mut b = BytesMut::new();
        encode_varint(300, &mut b);
        assert_eq!(&b[..], &[0xac, 0x02]);
    }

    #[test]
    fn crc32c_matches_ceph_convention() {
        // src/test/common/test_crc32c.cc (Crc32c.Small)
        assert_eq!(ceph_crc32c(0, b"foo bar baz"), 4119623852);
        assert_eq!(ceph_crc32c(1234, b"foo bar baz"), 881700046);
    }
}
