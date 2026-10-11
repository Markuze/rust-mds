//! Directory fragments (`include/ceph_frag.h`, `include/frag.h`, `common/frag.cc`) and the
//! dentry key order of a dirfrag (`dentry_key_t`, `src/mds/mdstypes.h`).
//!
//! A frag is a `u32`: the upper 8 bits are the number of significant bits ("bits"), the lower
//! 24 bits the value, whose most significant `bits` bits select the subset of the 24-bit hash
//! space. The root frag (all names) is `0`.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use denc::{BufMut, Bytes, BytesMut, Decode, Encode, Result};

use crate::util::get;

/// `ceph_frag_make(b, v)`: a frag with `b` significant bits taken from `v`.
/// `b` is clamped to 24 (the C macro is undefined above that).
#[inline]
pub const fn ceph_frag_make(b: u32, v: u32) -> u32 {
    let b = if b > 24 { 24 } else { b };
    (b << 24) | (v & (0x00ff_ffffu32 << (24 - b)) & 0x00ff_ffff)
}

/// `ceph_frag_bits`.
#[inline]
pub const fn ceph_frag_bits(f: u32) -> u32 {
    f >> 24
}

/// `ceph_frag_value`: the 24-bit value (also used to reduce a 32-bit name hash to the frag
/// space, as readdir ordering does).
#[inline]
pub const fn ceph_frag_value(f: u32) -> u32 {
    f & 0x00ff_ffff
}

/// `ceph_frag_mask`.
#[inline]
pub const fn ceph_frag_mask(f: u32) -> u32 {
    let b = ceph_frag_bits(f);
    if b >= 24 { 0x00ff_ffff } else { (0x00ff_ffffu32 << (24 - b)) & 0x00ff_ffff }
}

/// `ceph_frag_contains_value`: whether hash value `v` falls in frag `f`.
#[inline]
pub const fn ceph_frag_contains_value(f: u32, v: u32) -> bool {
    (v & ceph_frag_mask(f)) == ceph_frag_value(f)
}

/// `ceph_frag_contains_frag`: whether `sub` is as specific as `f` and inside it.
#[inline]
pub const fn ceph_frag_contains_frag(f: u32, sub: u32) -> bool {
    ceph_frag_bits(sub) >= ceph_frag_bits(f) && (ceph_frag_value(sub) & ceph_frag_mask(f)) == ceph_frag_value(f)
}

/// `frag_t`, as a typed `u32`. Ordered like C++ `frag_t::operator<`: by value, then by bits.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Frag(pub u32);

impl Frag {
    /// The root frag: every name.
    pub const ROOT: Frag = Frag(0);

    pub const fn new(bits: u32, value: u32) -> Frag {
        Frag(ceph_frag_make(bits, value))
    }
    pub const fn bits(self) -> u32 {
        ceph_frag_bits(self.0)
    }
    pub const fn value(self) -> u32 {
        ceph_frag_value(self.0)
    }
    pub const fn mask(self) -> u32 {
        ceph_frag_mask(self.0)
    }
    pub const fn is_root(self) -> bool {
        self.bits() == 0
    }
    /// Whether the (32-bit or 24-bit) hash `v` falls in this frag; like `frag_t::contains`,
    /// only the low 24 bits of `v` count.
    pub const fn contains_value(self, v: u32) -> bool {
        ceph_frag_contains_value(self.0, v)
    }
    pub const fn contains(self, sub: Frag) -> bool {
        ceph_frag_contains_frag(self.0, sub.0)
    }
    pub const fn is_leftmost(self) -> bool {
        self.value() == 0
    }
    pub const fn is_rightmost(self) -> bool {
        self.value() == self.mask()
    }
    /// `ceph_frag_make_child(f, by, i)`: the `i`-th child when splitting by `by` bits.
    pub const fn make_child(self, by: u32, i: u32) -> Frag {
        let newbits = self.bits().saturating_add(by);
        let shift = 24u32.saturating_sub(newbits);
        Frag::new(newbits, self.value() | (i << shift))
    }
    pub const fn left_child(self) -> Frag {
        Frag::new(self.bits() + 1, self.value())
    }
    pub const fn right_child(self) -> Frag {
        let shift = if self.bits() >= 23 { 0 } else { 23 - self.bits() };
        Frag::new(self.bits() + 1, self.value() | (1 << shift))
    }
    pub const fn parent(self) -> Frag {
        if self.bits() == 0 {
            return self;
        }
        Frag::new(self.bits() - 1, self.value() & (self.mask() << 1))
    }
    /// `ceph_frag_next`: the frag to the right at the same depth (wraps to the leftmost value
    /// past the rightmost frag; check [`Frag::is_rightmost`] first).
    pub const fn next(self) -> Frag {
        let step = match 0x0100_0000u32.checked_shr(self.bits()) {
            Some(s) => s,
            None => 0,
        };
        Frag::new(self.bits(), self.value().wrapping_add(step))
    }
}

impl PartialOrd for Frag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Frag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value().cmp(&other.value()).then(self.bits().cmp(&other.bits()))
    }
}

impl fmt::Debug for Frag {
    /// C++ prints a frag as its bits in binary followed by `*`; `*` alone is the root.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::with_capacity(25);
        for i in 0..self.bits().min(24) {
            s.push(if self.value() & (1 << (23 - i)) != 0 { '1' } else { '0' });
        }
        s.push('*');
        f.write_str(&s)
    }
}

impl Encode for Frag {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u32_le(self.0);
    }
}

impl Decode for Frag {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        Ok(Frag(get::<u32>(buf, "frag_t")?))
    }
}

/// `fragtree_t`: the split points of a directory, `frag -> split bits`. Wire format
/// (`fragtree_t::encode`): `u32 n` + n x (`u32 frag`, `i32 bits`), in [`Frag`] order.
/// P0 directories always have an empty tree (one frag, design D5).
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct FragTree {
    pub splits: BTreeMap<Frag, i32>,
}

impl FragTree {
    pub fn new() -> Self {
        FragTree::default()
    }
    pub fn is_empty(&self) -> bool {
        self.splits.is_empty()
    }
    /// `fragtree_t::operator[]`: the leaf frag containing hash value `v`.
    pub fn leaf_for(&self, v: u32) -> Frag {
        let mut f = Frag::ROOT;
        // Bounded by 24 levels; ignore nonsensical split counts.
        for _ in 0..=24 {
            match self.splits.get(&f) {
                Some(&by) if by > 0 && f.bits() + (by as u32) <= 24 => {
                    let nb = f.bits() + by as u32;
                    f = Frag::new(nb, v);
                }
                _ => break,
            }
        }
        f
    }
}

impl fmt::Debug for FragTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.splits.iter()).finish()
    }
}

impl Encode for FragTree {
    fn encode(&self, buf: &mut BytesMut, _features: u64) {
        buf.put_u32_le(self.splits.len() as u32);
        for (f, by) in &self.splits {
            buf.put_u32_le(f.0);
            buf.put_i32_le(*by);
        }
    }
}

impl Decode for FragTree {
    fn decode(buf: &mut Bytes) -> Result<Self> {
        let n = get::<u32>(buf, "fragtree_t")?;
        let mut splits = BTreeMap::new();
        for _ in 0..n {
            let f = Frag::decode(buf)?;
            let by = get::<i32>(buf, "fragtree_t.split")?;
            splits.insert(f, by);
        }
        Ok(FragTree { splits })
    }
}

/// The order of dentries in a dirfrag and in readdir replies (C++ `operator<(dentry_key_t,
/// dentry_key_t)` for head dentries): the 24-bit frag value of the name hash, then the name
/// compared bytewise. `hash_a`/`hash_b` are full 32-bit name hashes.
pub fn dentry_key_cmp(hash_a: u32, name_a: &[u8], hash_b: u32, name_b: &[u8]) -> Ordering {
    ceph_frag_value(hash_a).cmp(&ceph_frag_value(hash_b)).then_with(|| name_a.cmp(name_b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use denc::{decode_exact, encode_to_bytes};

    #[test]
    fn make_and_contains() {
        assert_eq!(ceph_frag_make(0, 0xffff_ffff), 0);
        assert_eq!(ceph_frag_make(1, 0x00ff_ffff), 0x0180_0000);
        assert_eq!(ceph_frag_make(24, 0x1234_5678), 0x1834_5678);
        assert_eq!(ceph_frag_make(99, 0x1234_5678), 0x1834_5678);
        let left = Frag::ROOT.left_child();
        let right = Frag::ROOT.right_child();
        assert_eq!(left, Frag(0x0100_0000));
        assert_eq!(right, Frag(0x0180_0000));
        assert!(Frag::ROOT.contains_value(0xdead_beef));
        assert!(left.contains_value(0x007f_ffff));
        assert!(!left.contains_value(0x0080_0000));
        assert!(right.contains_value(0x0080_0000));
        assert!(Frag::ROOT.contains(right));
        assert!(!left.contains(right));
        assert_eq!(right.parent(), Frag::ROOT);
        assert_eq!(left.next(), right);
        assert!(right.is_rightmost() && left.is_leftmost());
        assert_eq!(Frag::ROOT.make_child(2, 3), Frag::new(2, 0x00c0_0000));
        assert!(ceph_frag_contains_value(0, 5));
        assert_eq!(format!("{:?}", Frag::new(3, 0x00a0_0000)), "101*");
        assert_eq!(format!("{:?}", Frag::ROOT), "*");
    }

    #[test]
    fn frag_order_is_value_then_bits() {
        let mut v = vec![Frag::new(1, 0x0080_0000), Frag::new(2, 0), Frag::ROOT, Frag::new(1, 0)];
        v.sort();
        assert_eq!(v, vec![Frag::ROOT, Frag::new(1, 0), Frag::new(2, 0), Frag::new(1, 0x0080_0000)]);
    }

    #[test]
    fn fragtree_wire_and_lookup() {
        let mut t = FragTree::new();
        assert_eq!(&encode_to_bytes(&t, 0)[..], &[0, 0, 0, 0]);
        assert_eq!(t.leaf_for(0x00ab_cdef), Frag::ROOT);
        t.splits.insert(Frag::ROOT, 1);
        t.splits.insert(Frag::new(1, 0x0080_0000), 2);
        let b = encode_to_bytes(&t, 0);
        assert_eq!(hex::encode(&b), "02000000".to_owned() + "00000000" + "01000000" + "00008001" + "02000000");
        assert_eq!(decode_exact::<FragTree>(b, "ft").unwrap(), t);
        assert_eq!(t.leaf_for(0x0010_0000), Frag::new(1, 0));
        assert_eq!(t.leaf_for(0x00f0_0000), Frag::new(3, 0x00e0_0000));
    }

    #[test]
    fn dentry_order() {
        use std::cmp::Ordering::*;
        // Only the low 24 bits of the hash count.
        assert_eq!(dentry_key_cmp(0xff00_0001, b"b", 0x0000_0001, b"a"), Greater);
        assert_eq!(dentry_key_cmp(0x0000_0001, b"b", 0x0000_0002, b"a"), Less);
        assert_eq!(dentry_key_cmp(5, b"ab", 5, b"b"), Less);
        assert_eq!(dentry_key_cmp(5, b"\xff", 5, b"a"), Greater);
    }
}
