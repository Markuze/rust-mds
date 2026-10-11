//! Internal helpers shared by the message modules: error context, lossy text decoding, raw
//! fixed-width records and the `from_raw` checks every message applies.

use std::collections::BTreeMap;

use denc::{Buf, Bytes, Decode, DecodeError, Message, RawMessage, Result};

/// Attach a field or struct name to a decode error, so a failure in a capture points at the
/// field that did not fit instead of at a primitive type.
pub(crate) trait Ctx<T> {
    fn ctx(self, what: &'static str) -> Result<T>;
}

impl<T> Ctx<T> for Result<T> {
    fn ctx(self, what: &'static str) -> Result<T> {
        self.map_err(|e| match e {
            DecodeError::Short { need, have, .. } => DecodeError::Short { what, need, have },
            DecodeError::Incompatible { compat, supported, .. } => {
                DecodeError::Incompatible { what, compat, supported }
            }
            DecodeError::Invalid { what: inner, detail } => {
                DecodeError::Invalid { what, detail: format!("{inner}: {detail}") }
            }
        })
    }
}

/// Decode one value and name it in the error.
#[inline]
pub(crate) fn get<T: Decode>(buf: &mut Bytes, what: &'static str) -> Result<T> {
    T::decode(buf).ctx(what)
}

pub(crate) fn invalid(what: &'static str, detail: impl Into<String>) -> DecodeError {
    DecodeError::Invalid { what, detail: detail.into() }
}

/// `std::string` decoded as text without failing on invalid UTF-8 (C++ strings are byte
/// strings; metadata values are text in practice). Invalid sequences become U+FFFD.
pub(crate) fn get_string_lossy(buf: &mut Bytes, what: &'static str) -> Result<String> {
    let raw = get::<Bytes>(buf, what)?;
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

/// `std::map<std::string, std::string>` decoded lossily (see [`get_string_lossy`]).
pub(crate) fn get_string_map_lossy(buf: &mut Bytes, what: &'static str) -> Result<BTreeMap<String, String>> {
    let n = get::<u32>(buf, what)?;
    let mut m = BTreeMap::new();
    for _ in 0..n {
        let k = get_string_lossy(buf, what)?;
        let v = get_string_lossy(buf, what)?;
        m.insert(k, v);
    }
    Ok(m)
}

/// `n` little-endian `u64` values without a count prefix (`decode_nohead` of a vector).
pub(crate) fn get_u64s_nohead(buf: &mut Bytes, n: usize, what: &'static str) -> Result<Vec<u64>> {
    let bytes = n.checked_mul(8).ok_or_else(|| invalid(what, format!("count {n} overflows")))?;
    denc::need(buf, bytes, what)?;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(buf.get_u64_le());
    }
    Ok(v)
}

/// Common checks of [`Message::from_raw`]: the type matches and the sender's
/// `compat_version` is not newer than the version this crate implements (C++
/// `decode_message` refuses such messages too).
pub(crate) fn check_raw<M: Message>(raw: &RawMessage) -> Result<()> {
    if raw.msg_type != M::TYPE {
        return Err(invalid("message type", format!("expected {}, got {}", M::TYPE, raw.msg_type)));
    }
    if raw.compat_version > M::HEAD_VERSION {
        return Err(invalid(
            "message compat_version",
            format!("type {}: compat_version {} > supported {}", M::TYPE, raw.compat_version, M::HEAD_VERSION),
        ));
    }
    Ok(())
}

/// Implement `from_raw` with the compat check, and optionally copy the header `tid` into the
/// message's `tid` field (requests, replies, forwards and cap messages carry their tid in the
/// header, not in the payload).
macro_rules! impl_from_raw {
    () => {
        fn from_raw(raw: denc::RawMessage) -> denc::Result<Self> {
            $crate::util::check_raw::<Self>(&raw)?;
            Self::decode_payload(raw.version, raw.front, raw.middle, raw.data)
        }
    };
    (tid) => {
        fn tid(&self) -> u64 {
            self.tid
        }
        fn from_raw(raw: denc::RawMessage) -> denc::Result<Self> {
            $crate::util::check_raw::<Self>(&raw)?;
            let tid = raw.tid;
            let mut m = Self::decode_payload(raw.version, raw.front, raw.middle, raw.data)?;
            m.tid = tid;
            Ok(m)
        }
    };
}
pub(crate) use impl_from_raw;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctx_renames_short_errors() {
        let mut b = Bytes::from_static(&[1]);
        let e = get::<u32>(&mut b, "field").unwrap_err();
        assert_eq!(e, DecodeError::Short { what: "field", need: 4, have: 1 });
    }

    #[test]
    fn lossy_strings_do_not_fail() {
        let mut b = Bytes::from_static(&[2, 0, 0, 0, 0xff, b'a']);
        assert_eq!(get_string_lossy(&mut b, "s").unwrap(), "\u{fffd}a");
    }

    #[test]
    fn u64s_nohead_bounds() {
        let mut b = Bytes::from_static(&[0; 15]);
        assert!(get_u64s_nohead(&mut b, 2, "v").is_err());
        let mut b = Bytes::from_static(&[1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(get_u64s_nohead(&mut b, 2, "v").unwrap(), vec![1, 2]);
        assert!(get_u64s_nohead(&mut Bytes::new(), usize::MAX, "v").is_err());
    }
}
