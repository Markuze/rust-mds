//! CephFS feature bits (`CEPHFS_FEATURE_*`), the `feature_bitset_t` and `metric_spec_t`
//! encodings, and the bitmaps this MDS advertises in `CEPH_SESSION_OPEN`.
//!
//! Bit numbers follow Ceph `main` (`src/mds/cephfs_features.h`). Ceph 20.2.0 has the same
//! numbers but stops at bit 23 (`BLOCKDIFF`); `QUARANTINE` (24) exists only on `main`.

use std::fmt;

use denc::{BufMut, Bytes, BytesMut, Decode, Encode, Result};

use crate::util::{Ctx, get};

/// `CEPHFS_FEATURE_*` bit numbers (`src/mds/cephfs_features.h`). Bits 0-4 are reserved for
/// old releases and always set by current clients.
pub mod bit {
    pub const JEWEL: u32 = 5;
    pub const KRAKEN: u32 = 6;
    pub const LUMINOUS: u32 = 7;
    pub const MIMIC: u32 = 8;
    /// Versioned `InodeStat`/`DirStat`/`LeaseStat` (`ENCODE_START`) in replies.
    pub const REPLY_ENCODING: u32 = 9;
    /// `MClientReclaim` support (libcephfs/Ganesha refuse reclaim without it).
    pub const RECLAIM_CLIENT: u32 = 10;
    pub const LAZY_CAP_WANTED: u32 = 11;
    /// Clients may split `MClientReconnect` into several v5 messages.
    pub const MULTI_RECONNECT: u32 = 12;
    pub const NAUTILUS: u32 = 12;
    /// `CREATE` replies carry `openc_response_t` (both clients pick the parser from this bit).
    pub const DELEG_INO: u32 = 13;
    pub const OCTOPUS: u32 = 13;
    /// Clients send `MClientMetrics` only to an MDS with this bit.
    pub const METRIC_COLLECT: u32 = 14;
    pub const ALTERNATE_NAME: u32 = 15;
    /// A duplicate `REQUEST_OPEN` is answered with `OPEN`.
    pub const NOTIFY_SESSION_STATE: u32 = 16;
    /// Clients send `CEPH_MDS_OP_GETVXATTR` only to an MDS with this bit.
    pub const OP_GETVXATTR: u32 = 17;
    /// Request head v2+: 32-bit retry and forward counters.
    pub const BITS32_RETRY_FWD: u32 = 18;
    /// Snap traces in the `SnapRealmInfoNew` format (the kernel never sets it).
    pub const NEW_SNAPREALM_INFO: u32 = 19;
    /// Request head v3: `owner_uid`/`owner_gid`.
    pub const HAS_OWNER_UIDGID: u32 = 20;
    pub const MDS_AUTH_CAPS_CHECK: u32 = 21;
    /// `CHARMAP` in Ceph; the kernel (7.1+) uses bit 22 for subvolume metrics.
    pub const CHARMAP: u32 = 22;
    pub const BLOCKDIFF: u32 = 23;
    pub const QUARANTINE: u32 = 24;
    /// `CEPHFS_FEATURE_MAX` on Ceph `main`.
    pub const MAX: u32 = 24;
}

/// The CephFS features this MDS advertises in `OPEN` (design 4.2 table): bits 0-14 and 16-21.
/// Not advertised: 15 (`alternate_name`, fscrypt is P2), 22 (kernel subvolume metrics vs.
/// libcephfs charmap), 23 (`blockdiff`) and 24 (`quarantine`).
pub const MDS_SUPPORTED_FEATURES: u64 = 0x0000_7fff | 0x003f_0000;

/// The metric spec this MDS advertises: client metric types 0-15. Type 16
/// (`SUBVOLUME_METRICS`) waits for P1 subvolume metrics.
pub const MDS_METRIC_SPEC: u64 = 0xffff;

/// `CEPHFS_FEATURES_ALL` of Ceph 20.2.0 (bits 0-23), what libcephfs/ceph-fuse 20.2 advertise
/// and what the C++ 20.2 MDS sends in `OPEN`.
pub const CEPH_20_2_FEATURES: u64 = 0x00ff_ffff;

/// The bits a 7.x kernel advertises (`CEPHFS_FEATURES_CLIENT_SUPPORTED`): 0-9, 11-18, 20, 21
/// (plus 22 from Linux 7.1 on).
pub const KERNEL_7_0_FEATURES: u64 = 0x0037_fbff;

/// `feature_bitset_t` (`src/mds/mdstypes.cc`): a little-endian bitmap of 64-bit words.
///
/// Wire format: `u32 byte_len` followed by `byte_len / 8` `u64` words. A trailing partial word
/// (`byte_len % 8 != 0`) is accepted on decode and zero-extended, as C++ does; the encoder
/// always writes whole words.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct FeatureBitset {
    words: Vec<u64>,
}

impl FeatureBitset {
    pub const fn new() -> Self {
        FeatureBitset { words: Vec::new() }
    }

    /// `feature_bitset_t(unsigned long)`: one word, or no words at all for 0.
    pub fn from_word(w: u64) -> Self {
        if w == 0 { FeatureBitset::new() } else { FeatureBitset { words: vec![w] } }
    }

    /// A bitmap with the given bits set.
    pub fn from_bits(bits: impl IntoIterator<Item = u32>) -> Self {
        let mut s = FeatureBitset::new();
        for b in bits {
            s.insert(b);
        }
        s
    }

    pub fn from_words(words: Vec<u64>) -> Self {
        FeatureBitset { words }
    }

    pub fn words(&self) -> &[u64] {
        &self.words
    }

    /// The first 64 bits, which is all the kernel reads from the MDS bitmap.
    pub fn first_word(&self) -> u64 {
        self.words.first().copied().unwrap_or(0)
    }

    pub fn test(&self, bit: u32) -> bool {
        let (w, b) = ((bit / 64) as usize, bit % 64);
        self.words.get(w).is_some_and(|x| x & (1u64 << b) != 0)
    }

    pub fn insert(&mut self, bit: u32) {
        let (w, b) = ((bit / 64) as usize, bit % 64);
        if self.words.len() <= w {
            self.words.resize(w + 1, 0);
        }
        self.words[w] |= 1u64 << b;
    }

    /// Clear a bit, trimming trailing zero words as C++ `erase` does.
    pub fn erase(&mut self, bit: u32) {
        let (w, b) = ((bit / 64) as usize, bit % 64);
        if let Some(x) = self.words.get_mut(w) {
            *x &= !(1u64 << b);
            while self.words.last() == Some(&0) {
                self.words.pop();
            }
        }
    }

    /// C++ `empty()`: no bit set (zero words count as empty).
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|w| *w == 0)
    }

    /// Bits of `required` that `self` lacks (`required - self`).
    pub fn missing(&self, required: &FeatureBitset) -> FeatureBitset {
        let words =
            required.words.iter().enumerate().map(|(i, r)| r & !self.words.get(i).copied().unwrap_or(0)).collect();
        let mut s = FeatureBitset { words };
        while s.words.last() == Some(&0) {
            s.words.pop();
        }
        s
    }

    /// Set bits in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.words
            .iter()
            .enumerate()
            .flat_map(|(i, w)| (0..64u32).filter(move |b| w & (1u64 << b) != 0).map(move |b| i as u32 * 64 + b))
    }
}

impl fmt::Debug for FeatureBitset {
    /// Same form as C++ `feature_bitset_t::print`: `0x` and the words, most significant first.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.words.is_empty() {
            return write!(f, "0x0");
        }
        write!(f, "0x")?;
        for w in self.words.iter().rev() {
            write!(f, "{w:016x}")?;
        }
        Ok(())
    }
}

impl Encode for FeatureBitset {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u32_le((self.words.len() * 8) as u32);
        for w in &self.words {
            buf.put_u64_le(*w);
        }
    }
}

impl Decode for FeatureBitset {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let len = get::<u32>(buf, "feature_bitset_t.len")? as usize;
        let raw = denc::take(buf, len, "feature_bitset_t")?;
        let mut words = Vec::with_capacity(len.div_ceil(8));
        for chunk in raw.chunks(8) {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            words.push(u64::from_le_bytes(w));
        }
        Ok(FeatureBitset { words })
    }
}

/// `metric_spec_t` (`src/mds/mdstypes.cc`): `ENCODE_START(1, 1)` { `feature_bitset_t
/// metric_flags` }, one bit per `ClientMetricType`.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct MetricSpec {
    pub metric_flags: FeatureBitset,
}

impl MetricSpec {
    pub fn from_word(w: u64) -> Self {
        MetricSpec { metric_flags: FeatureBitset::from_word(w) }
    }

    /// The spec this MDS advertises ([`MDS_METRIC_SPEC`]).
    pub fn mds_default() -> Self {
        MetricSpec::from_word(MDS_METRIC_SPEC)
    }

    pub fn is_empty(&self) -> bool {
        self.metric_flags.is_empty()
    }
}

impl fmt::Debug for MetricSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MetricSpec({:?})", self.metric_flags)
    }
}

impl Encode for MetricSpec {
    fn encode(&self, buf: &mut BytesMut, features: u64) {
        denc::encode_versioned(buf, 1, 1, |b| self.metric_flags.encode(b, features));
    }
}

impl Decode for MetricSpec {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let mut f = denc::decode_start(buf, 1, "metric_spec_t")?;
        let metric_flags = FeatureBitset::decode(&mut f.body).ctx("metric_spec_t.metric_flags")?;
        Ok(MetricSpec { metric_flags })
    }
}

/// The MDS-advertised feature bitmap as a [`FeatureBitset`].
pub fn mds_supported_features() -> FeatureBitset {
    FeatureBitset::from_word(MDS_SUPPORTED_FEATURES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::{decode_exact, encode_to_bytes};

    #[test]
    fn mds_bitmap_matches_design_table() {
        let s = mds_supported_features();
        for b in 0..=14 {
            assert!(s.test(b), "bit {b}");
        }
        assert!(!s.test(bit::ALTERNATE_NAME));
        for b in 16..=21 {
            assert!(s.test(b), "bit {b}");
        }
        for b in [bit::CHARMAP, bit::BLOCKDIFF, bit::QUARANTINE, 63, 64] {
            assert!(!s.test(b), "bit {b}");
        }
        for b in [
            bit::REPLY_ENCODING,
            bit::RECLAIM_CLIENT,
            bit::MULTI_RECONNECT,
            bit::DELEG_INO,
            bit::OP_GETVXATTR,
            bit::BITS32_RETRY_FWD,
            bit::HAS_OWNER_UIDGID,
        ] {
            assert!(s.test(b), "required bit {b}");
        }
        assert_eq!(s.first_word(), 0x3f7fff);
    }

    #[test]
    fn kernel_bits() {
        let k = FeatureBitset::from_word(KERNEL_7_0_FEATURES);
        let bits: Vec<u32> = k.iter().collect();
        let mut want: Vec<u32> = (0..=9).chain(11..=18).collect();
        want.extend([20, 21]);
        assert_eq!(bits, want);
    }

    #[test]
    fn bitset_wire_format() {
        // C++ CEPHFS_FEATURES_ALL of 20.2.0: bits 0..23 in one word.
        let s = FeatureBitset::from_bits(0..=23);
        let b = encode_to_bytes(&s, 0);
        assert_eq!(hex::encode(&b), "08000000ffffff0000000000");
        assert_eq!(decode_exact::<FeatureBitset>(b, "fb").unwrap(), s);
        // Empty bitmap: zero length.
        assert_eq!(&encode_to_bytes(&FeatureBitset::new(), 0)[..], &[0, 0, 0, 0]);
        // A 3-byte (partial word) bitmap is zero-extended.
        let s = decode_exact::<FeatureBitset>(Bytes::from_static(&[3, 0, 0, 0, 0xff, 0x01, 0x80]), "fb").unwrap();
        assert_eq!(s.words(), &[0x8001ff]);
        // Two words.
        let s = FeatureBitset::from_bits([1, 70]);
        assert_eq!(s.words(), &[2, 64]);
        assert_eq!(format!("{s:?}"), "0x00000000000000400000000000000002");
        let b = encode_to_bytes(&s, 0);
        assert_eq!(b.len(), 4 + 16);
        assert_eq!(decode_exact::<FeatureBitset>(b, "fb").unwrap(), s);
    }

    #[test]
    fn bitset_ops() {
        let mut s = FeatureBitset::from_bits([3, 64]);
        s.erase(64);
        assert_eq!(s.words(), &[8]);
        assert!(!FeatureBitset::from_words(vec![0, 0]).test(3));
        assert!(FeatureBitset::from_words(vec![0, 0]).is_empty());
        let req = FeatureBitset::from_bits([1, 2, 70]);
        let have = FeatureBitset::from_bits([2]);
        assert_eq!(have.missing(&req), FeatureBitset::from_bits([1, 70]));
        assert!(req.missing(&have).is_empty());
    }

    #[test]
    fn metric_spec_wire_format() {
        let m = MetricSpec::mds_default();
        let b = encode_to_bytes(&m, 0);
        // ENCODE_START(1,1) + len 12 + bitset (len 8, word 0xffff)
        assert_eq!(hex::encode(&b), "01010c00000008000000ffff000000000000");
        assert_eq!(decode_exact::<MetricSpec>(b, "ms").unwrap(), m);
        // The kernel's REQUEST_OPEN metric spec: types 0-16 in one word.
        let k = hex::decode("01010c00000008000000ffff010000000000").unwrap();
        let m = decode_exact::<MetricSpec>(Bytes::from(k), "ms").unwrap();
        assert!(m.metric_flags.test(16));
    }
}
