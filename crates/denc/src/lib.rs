//! Ceph wire encoding ("denc").
//!
//! - Integers are little-endian; `bool` is one byte.
//! - Strings, byte buffers (`bufferlist`) and containers are prefixed with a `u32` length or count.
//! - Versioned structs use the `ENCODE_START(v, compat)` framing: `u8 v`, `u8 compat`, `u32 len`,
//!   then `len` bytes of body. A decoder accepts any `v` whose `compat` it supports and skips
//!   body bytes it does not understand ([`decode_start`]).
//!
//! Decoding consumes from a [`Bytes`] cursor, so sub-buffers are zero-copy slices.

use std::collections::{BTreeMap, BTreeSet};

pub use bytes::{Buf, BufMut, Bytes, BytesMut};

mod addr;
pub mod features;
pub mod msg;
mod types;

pub use addr::{AddrType, EntityAddr, EntityAddrVec};
pub use msg::{Message, Payload, RawMessage};
pub use types::{EntityName, EntityType, UTime, Uuid};

/// Error returned by every decoder.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("short buffer decoding {what}: need {need} bytes, have {have}")]
    Short {
        what: &'static str,
        need: usize,
        have: usize,
    },
    #[error("{what}: struct compat {compat} is newer than supported {supported}")]
    Incompatible {
        what: &'static str,
        compat: u8,
        supported: u8,
    },
    #[error("{what}: {detail}")]
    Invalid { what: &'static str, detail: String },
}

pub type Result<T> = std::result::Result<T, DecodeError>;

/// Encode `self` onto the end of `buf`. `features` are the connection features of the peer;
/// most types ignore them.
pub trait Encode {
    fn encode(&self, buf: &mut BytesMut, features: u64);
}

/// Decode a value from the front of `buf`, advancing it.
pub trait Decode: Sized {
    fn decode(buf: &mut Bytes) -> Result<Self>;
}

/// Encode a value into a fresh buffer.
pub fn encode_to_bytes<T: Encode + ?Sized>(value: &T, features: u64) -> Bytes {
    let mut buf = BytesMut::new();
    value.encode(&mut buf, features);
    buf.freeze()
}

/// Decode a value that must consume the whole buffer (trailing bytes are an error).
pub fn decode_exact<T: Decode>(mut buf: Bytes, what: &'static str) -> Result<T> {
    let v = T::decode(&mut buf)?;
    if buf.has_remaining() {
        return Err(DecodeError::Invalid {
            what,
            detail: format!("{} trailing bytes", buf.remaining()),
        });
    }
    Ok(v)
}

/// Fail with [`DecodeError::Short`] unless `buf` holds at least `n` bytes.
#[inline]
pub fn need(buf: &Bytes, n: usize, what: &'static str) -> Result<()> {
    if buf.remaining() < n {
        Err(DecodeError::Short {
            what,
            need: n,
            have: buf.remaining(),
        })
    } else {
        Ok(())
    }
}

/// Split off exactly `n` bytes.
pub fn take(buf: &mut Bytes, n: usize, what: &'static str) -> Result<Bytes> {
    need(buf, n, what)?;
    Ok(buf.split_to(n))
}

macro_rules! int_impl {
    ($t:ty, $put:ident, $get:ident) => {
        impl Encode for $t {
            #[inline]
            fn encode(&self, buf: &mut BytesMut, _features: u64) {
                buf.$put(*self);
            }
        }
        impl Decode for $t {
            #[inline]
            fn decode(buf: &mut Bytes) -> Result<Self> {
                need(buf, std::mem::size_of::<$t>(), stringify!($t))?;
                Ok(buf.$get())
            }
        }
    };
}

int_impl!(u8, put_u8, get_u8);
int_impl!(i8, put_i8, get_i8);
int_impl!(u16, put_u16_le, get_u16_le);
int_impl!(i16, put_i16_le, get_i16_le);
int_impl!(u32, put_u32_le, get_u32_le);
int_impl!(i32, put_i32_le, get_i32_le);
int_impl!(u64, put_u64_le, get_u64_le);
int_impl!(i64, put_i64_le, get_i64_le);

impl Encode for bool {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u8(u8::from(*self));
    }
}
impl Decode for bool {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        Ok(u8::decode(buf)? != 0)
    }
}

impl Encode for f64 {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_f64_le(*self);
    }
}
impl Decode for f64 {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        need(buf, 8, "f64")?;
        Ok(buf.get_f64_le())
    }
}

/// `std::string`: `u32` length + bytes. Ceph strings are byte strings; use [`Bytes`] for names
/// that may not be UTF-8 (file names), `String` only for text.
impl Encode for String {
    fn encode(&self, buf: &mut BytesMut, f: u64) {
        self.as_str().encode(buf, f);
    }
}
impl Encode for str {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u32_le(self.len() as u32);
        buf.put_slice(self.as_bytes());
    }
}
impl Decode for String {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let raw = Bytes::decode(buf)?;
        String::from_utf8(raw.to_vec()).map_err(|e| DecodeError::Invalid {
            what: "string",
            detail: e.to_string(),
        })
    }
}

/// `bufferlist` or byte `std::string`: `u32` length + raw bytes.
impl Encode for Bytes {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u32_le(self.len() as u32);
        buf.put_slice(self);
    }
}
impl Decode for Bytes {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let len = u32::decode(buf)? as usize;
        take(buf, len, "bytes")
    }
}

impl Encode for [u8] {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u32_le(self.len() as u32);
        buf.put_slice(self);
    }
}

/// `std::vector<T>` / `std::list<T>`: `u32` count + items. (`Vec<u8>` matches the bufferlist
/// wire format, since a `u8` encodes as one byte.)
impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, buf: &mut BytesMut, features: u64) {
        self.as_slice().encode_items(buf, features);
    }
}
impl<T: Decode> Decode for Vec<T> {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let n = u32::decode(buf)? as usize;
        // Bound the pre-allocation by what the buffer could possibly hold.
        let mut v = Vec::with_capacity(n.min(buf.remaining()));
        for _ in 0..n {
            v.push(T::decode(buf)?);
        }
        Ok(v)
    }
}

/// Helper so slices of encodable items can be encoded with a count prefix.
pub trait EncodeItems {
    fn encode_items(&self, buf: &mut BytesMut, features: u64);
}
impl<T: Encode> EncodeItems for [T] {
    fn encode_items(&self, buf: &mut BytesMut, features: u64) {
        buf.put_u32_le(self.len() as u32);
        for item in self {
            item.encode(buf, features);
        }
    }
}

/// `std::map<K, V>`: `u32` count + (key, value) pairs in key order.
impl<K: Encode, V: Encode> Encode for BTreeMap<K, V> {
    fn encode(&self, buf: &mut BytesMut, features: u64) {
        buf.put_u32_le(self.len() as u32);
        for (k, v) in self {
            k.encode(buf, features);
            v.encode(buf, features);
        }
    }
}
impl<K: Decode + Ord, V: Decode> Decode for BTreeMap<K, V> {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let n = u32::decode(buf)?;
        let mut m = BTreeMap::new();
        for _ in 0..n {
            let k = K::decode(buf)?;
            let v = V::decode(buf)?;
            m.insert(k, v);
        }
        Ok(m)
    }
}

/// `std::set<T>`: `u32` count + items.
impl<T: Encode> Encode for BTreeSet<T> {
    fn encode(&self, buf: &mut BytesMut, features: u64) {
        buf.put_u32_le(self.len() as u32);
        for item in self {
            item.encode(buf, features);
        }
    }
}
impl<T: Decode + Ord> Decode for BTreeSet<T> {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let n = u32::decode(buf)?;
        let mut s = BTreeSet::new();
        for _ in 0..n {
            s.insert(T::decode(buf)?);
        }
        Ok(s)
    }
}

/// `std::pair<A, B>`.
impl<A: Encode, B: Encode> Encode for (A, B) {
    fn encode(&self, buf: &mut BytesMut, features: u64) {
        self.0.encode(buf, features);
        self.1.encode(buf, features);
    }
}
impl<A: Decode, B: Decode> Decode for (A, B) {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        Ok((A::decode(buf)?, B::decode(buf)?))
    }
}

/// `std::optional<T>`: `bool` present + value.
impl<T: Encode> Encode for Option<T> {
    fn encode(&self, buf: &mut BytesMut, features: u64) {
        match self {
            Some(v) => {
                buf.put_u8(1);
                v.encode(buf, features);
            }
            None => buf.put_u8(0),
        }
    }
}
impl<T: Decode> Decode for Option<T> {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        if bool::decode(buf)? {
            Ok(Some(T::decode(buf)?))
        } else {
            Ok(None)
        }
    }
}

/// Fixed-size byte arrays are raw (no length prefix), like `encode_raw` of a packed struct.
impl<const N: usize> Encode for [u8; N] {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_slice(self);
    }
}
impl<const N: usize> Decode for [u8; N] {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        need(buf, N, "byte array")?;
        let mut a = [0u8; N];
        buf.copy_to_slice(&mut a);
        Ok(a)
    }
}

/// Position of an open `ENCODE_START` frame; close it with [`encode_finish`].
#[must_use = "close the frame with encode_finish"]
pub struct EncodeStart {
    len_pos: usize,
}

/// `ENCODE_START(v, compat, bl)`: write `v`, `compat` and a length placeholder.
pub fn encode_start(buf: &mut BytesMut, v: u8, compat: u8) -> EncodeStart {
    buf.put_u8(v);
    buf.put_u8(compat);
    let len_pos = buf.len();
    buf.put_u32_le(0);
    EncodeStart { len_pos }
}

/// `ENCODE_FINISH(bl)`: patch the body length.
pub fn encode_finish(buf: &mut BytesMut, start: EncodeStart) {
    let len = (buf.len() - start.len_pos - 4) as u32;
    buf[start.len_pos..start.len_pos + 4].copy_from_slice(&len.to_le_bytes());
}

/// Encode a versioned struct whose body is written by `body`.
pub fn encode_versioned(buf: &mut BytesMut, v: u8, compat: u8, body: impl FnOnce(&mut BytesMut)) {
    let s = encode_start(buf, v, compat);
    body(buf);
    encode_finish(buf, s);
}

/// A decoded `DECODE_START` frame: the struct version and its body. Decode fields from `body`;
/// fields of newer versions that remain in `body` are ignored, as C++ does.
pub struct DecodeStart {
    pub v: u8,
    pub compat: u8,
    pub body: Bytes,
}

/// `DECODE_START(supported, bl)`: read the frame header, reject a `compat` newer than
/// `supported`, and split off the body.
pub fn decode_start(buf: &mut Bytes, supported: u8, what: &'static str) -> Result<DecodeStart> {
    need(buf, 6, what)?;
    let v = buf.get_u8();
    let compat = buf.get_u8();
    if compat > supported {
        return Err(DecodeError::Incompatible {
            what,
            compat,
            supported,
        });
    }
    let len = buf.get_u32_le() as usize;
    let body = take(buf, len, what)?;
    Ok(DecodeStart { v, compat, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ints_are_little_endian() {
        let b = encode_to_bytes(&0x0102_0304u32, 0);
        assert_eq!(&b[..], &[4, 3, 2, 1]);
        assert_eq!(decode_exact::<u32>(b, "u32").unwrap(), 0x0102_0304);
    }

    #[test]
    fn string_and_containers() {
        let mut m = BTreeMap::new();
        m.insert("a".to_string(), vec![1u32, 2]);
        let b = encode_to_bytes(&m, 0);
        assert_eq!(
            hex::encode(&b),
            "010000000100000061020000000100000002000000"
        );
        let back: BTreeMap<String, Vec<u32>> = decode_exact(b, "map").unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn versioned_frame_skips_unknown_tail() {
        let mut buf = BytesMut::new();
        encode_versioned(&mut buf, 3, 1, |b| {
            7u32.encode(b, 0);
            9u64.encode(b, 0); // a field this decoder does not know
        });
        let mut bytes = buf.freeze();
        let mut f = decode_start(&mut bytes, 2, "test").unwrap();
        assert_eq!((f.v, f.compat), (3, 1));
        assert_eq!(u32::decode(&mut f.body).unwrap(), 7);
        assert!(!bytes.has_remaining());
    }

    #[test]
    fn newer_compat_is_rejected() {
        let mut buf = BytesMut::new();
        encode_versioned(&mut buf, 5, 4, |_| {});
        let err = decode_start(&mut buf.freeze(), 3, "test").err().unwrap();
        assert!(matches!(
            err,
            DecodeError::Incompatible {
                compat: 4,
                supported: 3,
                ..
            }
        ));
    }

    #[test]
    fn short_buffer_is_an_error() {
        let mut b = Bytes::from_static(&[1, 2]);
        assert!(matches!(
            u32::decode(&mut b),
            Err(DecodeError::Short { .. })
        ));
    }
}
