//! Dentry name hashes and frag values.
//!
//! Directory entries are ordered by `(ceph_frag_value(hash(name)), name)`
//! (`operator<(dentry_key_t)`, `src/mds/mdstypes.h`), and clients compute the
//! same hash to derive readdir offsets (`(0xff << 52) | (hash24 << 28) | n`).
//! New directories use `mds_default_dir_hash = 2`, Robert Jenkins' hash
//! (`ceph_str_hash_rjenkins`, `src/common/ceph_hash.cc`, public domain
//! algorithm from burtleburtle.net/bob/hash/evahash.html); a directory whose
//! `dir_layout.dl_dir_hash` is 0 uses the Linux dcache hash
//! (`InodeStoreBase::hash_dentry_name`, `src/mds/CInode.cc`).

use crate::consts::{CEPH_STR_HASH_LINUX, CEPH_STR_HASH_RJENKINS};

#[inline(always)]
fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*b);
    *a = a.wrapping_sub(*c);
    *a ^= *c >> 13;
    *b = b.wrapping_sub(*c);
    *b = b.wrapping_sub(*a);
    *b ^= *a << 8;
    *c = c.wrapping_sub(*a);
    *c = c.wrapping_sub(*b);
    *c ^= *b >> 13;
    *a = a.wrapping_sub(*b);
    *a = a.wrapping_sub(*c);
    *a ^= *c >> 12;
    *b = b.wrapping_sub(*c);
    *b = b.wrapping_sub(*a);
    *b ^= *a << 16;
    *c = c.wrapping_sub(*a);
    *c = c.wrapping_sub(*b);
    *c ^= *b >> 5;
    *a = a.wrapping_sub(*b);
    *a = a.wrapping_sub(*c);
    *a ^= *c >> 3;
    *b = b.wrapping_sub(*c);
    *b = b.wrapping_sub(*a);
    *b ^= *a << 10;
    *c = c.wrapping_sub(*a);
    *c = c.wrapping_sub(*b);
    *c ^= *b >> 15;
}

#[inline(always)]
fn le32(k: &[u8]) -> u32 {
    u32::from(k[0]) | (u32::from(k[1]) << 8) | (u32::from(k[2]) << 16) | (u32::from(k[3]) << 24)
}

/// `ceph_str_hash_rjenkins` (`CEPH_STR_HASH_RJENKINS`, hash type 2).
pub fn ceph_str_hash_rjenkins(s: &[u8]) -> u32 {
    let length = s.len() as u32;
    let mut a: u32 = 0x9e37_79b9;
    let mut b: u32 = a;
    let mut c: u32 = 0;
    let mut k = s;
    while k.len() >= 12 {
        a = a.wrapping_add(le32(&k[0..4]));
        b = b.wrapping_add(le32(&k[4..8]));
        c = c.wrapping_add(le32(&k[8..12]));
        mix(&mut a, &mut b, &mut c);
        k = &k[12..];
    }
    // The last 11 bytes; the low byte of `c` is reserved for the length.
    c = c.wrapping_add(length);
    let n = k.len();
    if n >= 11 {
        c = c.wrapping_add(u32::from(k[10]) << 24);
    }
    if n >= 10 {
        c = c.wrapping_add(u32::from(k[9]) << 16);
    }
    if n >= 9 {
        c = c.wrapping_add(u32::from(k[8]) << 8);
    }
    if n >= 8 {
        b = b.wrapping_add(u32::from(k[7]) << 24);
    }
    if n >= 7 {
        b = b.wrapping_add(u32::from(k[6]) << 16);
    }
    if n >= 6 {
        b = b.wrapping_add(u32::from(k[5]) << 8);
    }
    if n >= 5 {
        b = b.wrapping_add(u32::from(k[4]));
    }
    if n >= 4 {
        a = a.wrapping_add(u32::from(k[3]) << 24);
    }
    if n >= 3 {
        a = a.wrapping_add(u32::from(k[2]) << 16);
    }
    if n >= 2 {
        a = a.wrapping_add(u32::from(k[1]) << 8);
    }
    if n >= 1 {
        a = a.wrapping_add(u32::from(k[0]));
    }
    mix(&mut a, &mut b, &mut c);
    c
}

/// `ceph_str_hash_linux` (`CEPH_STR_HASH_LINUX`, hash type 1).
pub fn ceph_str_hash_linux(s: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    for &ch in s {
        let c = u32::from(ch);
        hash = hash
            .wrapping_add(c << 4)
            .wrapping_add(c >> 4)
            .wrapping_mul(11);
    }
    hash
}

/// `ceph_str_hash(type, ...)`; type 0 means "Linux" as in
/// `hash_dentry_name`. Unknown types return `u32::MAX` like the C++ `-1`.
pub fn ceph_str_hash(hash_type: u8, s: &[u8]) -> u32 {
    match hash_type {
        0 | CEPH_STR_HASH_LINUX => ceph_str_hash_linux(s),
        CEPH_STR_HASH_RJENKINS => ceph_str_hash_rjenkins(s),
        _ => u32::MAX,
    }
}

/// `ceph_frag_value`: the low 24 bits of a frag (or of a hash fed as a frag).
#[inline]
pub fn frag_value(f: u32) -> u32 {
    f & 0x00ff_ffff
}

/// `ceph_frag_bits`.
#[inline]
pub fn frag_bits(f: u32) -> u32 {
    f >> 24
}

/// `ceph_frag_mask`.
#[inline]
pub fn frag_mask(f: u32) -> u32 {
    (0x00ff_ffffu32 << (24 - frag_bits(f))) & 0x00ff_ffff
}

/// `ceph_frag_contains_value`.
#[inline]
pub fn frag_contains_value(f: u32, v: u32) -> bool {
    (v & frag_mask(f)) == frag_value(f)
}

/// The 24-bit key a dentry sorts by inside its directory.
#[inline]
pub fn dentry_hash24(hash_type: u8, name: &[u8]) -> u32 {
    frag_value(ceph_str_hash(hash_type, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    /// Golden values produced by compiling `src/common/ceph_hash.cc` of the
    /// C++ tree with a small driver (names given in hex).
    const GOLDEN: &[(&str, u32, u32)] = &[
        ("", 0xbd49d10d, 0x00000000),
        ("61", 0x29eec818, 0x000042f2),
        ("62", 0x54aca597, 0x000043a2),
        ("6162", 0x9879ac41, 0x00032408),
        ("616263", 0x251e4793, 0x0022d0aa),
        ("666f6f", 0x7fc1f406, 0x0024db2a),
        ("626172", 0xefe6384b, 0x002326a5),
        ("66696c65", 0x2e6fb49a, 0x0193a6b0),
        ("646972", 0x63d72e26, 0x00240985),
        ("68656c6c6f20776f726c64", 0x1aa919e6, 0x1f2e3c54),
        ("3031323334353637383961", 0x90d7829e, 0x9ad5b096),
        ("303132333435363738396162", 0x92f31ad0, 0xa72eda14),
        ("30313233343536373839616263", 0x88c1bd29, 0x2f03a32e),
        (
            "6162636465666768696a6b6c6d6e6f707172737475767778797a",
            0xc52fcee8,
            0xfacbdc8f,
        ),
        ("2e736e6170", 0xd5266b31, 0x08d943e2),
        (
            "757365722e636570682e737562766f6c756d652e6561726d61726b",
            0xc74c2981,
            0x8016ca10,
        ),
        ("6630", 0xf7b49b71, 0x00032757),
        ("6631", 0x67b909e9, 0x00032807),
        ("6632", 0x54e93ef4, 0x000328b7),
        ("6633", 0xa9eca2aa, 0x00032967),
        ("6634", 0x3df631ce, 0x00032a17),
        ("6635", 0xf9b2f7b8, 0x00032ac7),
        ("6636", 0x5d026ad5, 0x00032b77),
        ("6637", 0x5b994840, 0x00032c27),
        ("6638", 0xb11e49c3, 0x00032cd7),
        ("6639", 0x96a793f2, 0x00032d87),
        ("fffe8001", 0x4d93f887, 0x03e97ae4),
    ];

    #[test]
    fn rjenkins_matches_ceph() {
        for (hex, rj, _) in GOLDEN {
            assert_eq!(ceph_str_hash_rjenkins(&unhex(hex)), *rj, "name {hex}");
        }
    }

    #[test]
    fn linux_matches_ceph() {
        for (hex, _, lx) in GOLDEN {
            assert_eq!(ceph_str_hash_linux(&unhex(hex)), *lx, "name {hex}");
        }
    }

    #[test]
    fn dispatch_and_frag_helpers() {
        assert_eq!(ceph_str_hash(2, b"foo"), 0x7fc1f406);
        assert_eq!(ceph_str_hash(1, b"foo"), 0x0024db2a);
        assert_eq!(ceph_str_hash(0, b"foo"), 0x0024db2a);
        assert_eq!(ceph_str_hash(9, b"foo"), u32::MAX);
        assert_eq!(dentry_hash24(2, b"foo"), 0xc1f406);
        assert_eq!(frag_value(0x0100_0000 | 0x80_0000), 0x80_0000);
        assert_eq!(frag_bits(0x0100_0000), 1);
        // The root frag (0 bits) contains every value.
        assert!(frag_contains_value(0, 0xabcdef));
        // The frag "1*" (1 bit, value 0x800000) contains only the upper half.
        assert!(frag_contains_value(0x0180_0000, 0x90_0000));
        assert!(!frag_contains_value(0x0180_0000, 0x10_0000));
    }
}
