//! The hash functions of RADOS placement.
//!
//! - Object name → placement seed: `ceph_str_hash` (`src/common/ceph_hash.cc`), Bob Jenkins'
//!   lookup2 hash (`CEPH_STR_HASH_RJENKINS`, the default `object_hash` of every pool) or the
//!   Linux dcache hash (`CEPH_STR_HASH_LINUX`).
//! - CRUSH's integer hashes `crush_hash32_{1..5}` (`src/crush/hash.c`), rjenkins1 mixing with
//!   seed 1315423911.
//! - `ceph_stable_mod` (`src/include/rados.h`).

/// `CEPH_STR_HASH_LINUX`.
pub const CEPH_STR_HASH_LINUX: u8 = 1;
/// `CEPH_STR_HASH_RJENKINS`.
pub const CEPH_STR_HASH_RJENKINS: u8 = 2;
/// `CRUSH_HASH_RJENKINS1`, the only CRUSH hash.
pub const CRUSH_HASH_RJENKINS1: u8 = 0;

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

/// `ceph_str_hash_rjenkins` (`src/common/ceph_hash.cc`).
pub fn ceph_str_hash_rjenkins(k: &[u8]) -> u32 {
    let length = k.len() as u32;
    let mut a: u32 = 0x9e37_79b9;
    let mut b: u32 = a;
    let mut c: u32 = 0;
    let mut k = k;
    let le = |s: &[u8]| u32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    while k.len() >= 12 {
        a = a.wrapping_add(le(&k[0..4]));
        b = b.wrapping_add(le(&k[4..8]));
        c = c.wrapping_add(le(&k[8..12]));
        mix(&mut a, &mut b, &mut c);
        k = &k[12..];
    }
    c = c.wrapping_add(length);
    let len = k.len();
    // The C switch falls through from `len` down to 1.
    if len >= 11 {
        c = c.wrapping_add(u32::from(k[10]) << 24);
    }
    if len >= 10 {
        c = c.wrapping_add(u32::from(k[9]) << 16);
    }
    if len >= 9 {
        c = c.wrapping_add(u32::from(k[8]) << 8);
    }
    if len >= 8 {
        b = b.wrapping_add(u32::from(k[7]) << 24);
    }
    if len >= 7 {
        b = b.wrapping_add(u32::from(k[6]) << 16);
    }
    if len >= 6 {
        b = b.wrapping_add(u32::from(k[5]) << 8);
    }
    if len >= 5 {
        b = b.wrapping_add(u32::from(k[4]));
    }
    if len >= 4 {
        a = a.wrapping_add(u32::from(k[3]) << 24);
    }
    if len >= 3 {
        a = a.wrapping_add(u32::from(k[2]) << 16);
    }
    if len >= 2 {
        a = a.wrapping_add(u32::from(k[1]) << 8);
    }
    if len >= 1 {
        a = a.wrapping_add(u32::from(k[0]));
    }
    mix(&mut a, &mut b, &mut c);
    c
}

/// `ceph_str_hash_linux` (`src/common/ceph_hash.cc`).
pub fn ceph_str_hash_linux(s: &[u8]) -> u32 {
    let mut hash: u32 = 0;
    for &c in s {
        let c = u32::from(c);
        hash = hash
            .wrapping_add(c << 4)
            .wrapping_add(c >> 4)
            .wrapping_mul(11);
    }
    hash
}

/// `ceph_str_hash(type, s, len)`: an unknown type yields `-1`.
pub fn ceph_str_hash(ty: u8, s: &[u8]) -> u32 {
    match ty {
        CEPH_STR_HASH_LINUX => ceph_str_hash_linux(s),
        CEPH_STR_HASH_RJENKINS => ceph_str_hash_rjenkins(s),
        _ => u32::MAX,
    }
}

const CRUSH_HASH_SEED: u32 = 1_315_423_911;

/// `crush_hash32_rjenkins1` (one input).
pub fn crush_hash32_1(ty: u8, a: u32) -> u32 {
    if ty != CRUSH_HASH_RJENKINS1 {
        return 0;
    }
    let mut hash = CRUSH_HASH_SEED ^ a;
    let mut b = a;
    let mut x: u32 = 231_232;
    let mut y: u32 = 1232;
    let mut a = a;
    mix(&mut b, &mut x, &mut hash);
    mix(&mut y, &mut a, &mut hash);
    hash
}

/// `crush_hash32_2`.
pub fn crush_hash32_2(ty: u8, a: u32, b: u32) -> u32 {
    if ty != CRUSH_HASH_RJENKINS1 {
        return 0;
    }
    let (mut a, mut b) = (a, b);
    let mut hash = CRUSH_HASH_SEED ^ a ^ b;
    let mut x: u32 = 231_232;
    let mut y: u32 = 1232;
    mix(&mut a, &mut b, &mut hash);
    mix(&mut x, &mut a, &mut hash);
    mix(&mut b, &mut y, &mut hash);
    hash
}

/// `crush_hash32_3`.
pub fn crush_hash32_3(ty: u8, a: u32, b: u32, c: u32) -> u32 {
    if ty != CRUSH_HASH_RJENKINS1 {
        return 0;
    }
    let (mut a, mut b, mut c) = (a, b, c);
    let mut hash = CRUSH_HASH_SEED ^ a ^ b ^ c;
    let mut x: u32 = 231_232;
    let mut y: u32 = 1232;
    mix(&mut a, &mut b, &mut hash);
    mix(&mut c, &mut x, &mut hash);
    mix(&mut y, &mut a, &mut hash);
    mix(&mut b, &mut x, &mut hash);
    mix(&mut y, &mut c, &mut hash);
    hash
}

/// `crush_hash32_4`.
pub fn crush_hash32_4(ty: u8, a: u32, b: u32, c: u32, d: u32) -> u32 {
    if ty != CRUSH_HASH_RJENKINS1 {
        return 0;
    }
    let (mut a, mut b, mut c, mut d) = (a, b, c, d);
    let mut hash = CRUSH_HASH_SEED ^ a ^ b ^ c ^ d;
    let mut x: u32 = 231_232;
    let mut y: u32 = 1232;
    mix(&mut a, &mut b, &mut hash);
    mix(&mut c, &mut d, &mut hash);
    mix(&mut a, &mut x, &mut hash);
    mix(&mut y, &mut b, &mut hash);
    mix(&mut c, &mut x, &mut hash);
    mix(&mut y, &mut d, &mut hash);
    hash
}

/// `crush_hash32_5`.
pub fn crush_hash32_5(ty: u8, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
    if ty != CRUSH_HASH_RJENKINS1 {
        return 0;
    }
    let (mut a, mut b, mut c, mut d, mut e) = (a, b, c, d, e);
    let mut hash = CRUSH_HASH_SEED ^ a ^ b ^ c ^ d ^ e;
    let mut x: u32 = 231_232;
    let mut y: u32 = 1232;
    mix(&mut a, &mut b, &mut hash);
    mix(&mut c, &mut d, &mut hash);
    mix(&mut e, &mut x, &mut hash);
    mix(&mut y, &mut a, &mut hash);
    mix(&mut b, &mut x, &mut hash);
    mix(&mut y, &mut c, &mut hash);
    mix(&mut d, &mut x, &mut hash);
    mix(&mut y, &mut e, &mut hash);
    hash
}

/// `ceph_stable_mod(x, b, bmask)` (`src/include/rados.h`), with the C `int` arithmetic.
pub fn ceph_stable_mod(x: u32, b: u32, bmask: u32) -> u32 {
    let (x, b, bmask) = (x as i32, b as i32, bmask as i32);
    if (x & bmask) < b {
        (x & bmask) as u32
    } else {
        (x & (bmask >> 1)) as u32
    }
}

/// `cbits(v)` (`src/include/intarith.h`): number of significant bits.
pub fn cbits(v: u32) -> u32 {
    32 - v.leading_zeros()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vectors produced by compiling `src/common/ceph_hash.cc` and `src/crush/hash.c` from the
    /// reference tree.
    #[test]
    fn string_hash_reference_vectors() {
        let v: &[(&[u8], u32, u32)] = &[
            (b"", 3175731469, 0),
            (b"a", 703514648, 17138),
            (b"abc", 622741395, 2281642),
            (b"abcdefghijk", 3844836940, 1931677450),
            (b"abcdefghijkl", 186334885, 4068601840),
            (b"abcdefghijklm", 824356913, 1804966530),
            (b"10000000000.00000000", 4038422320, 1565748509),
            (b"mds0_inotable", 3092428947, 2038048634),
            (b"200.00000000", 2219783316, 2483692861),
        ];
        for (s, rj, lin) in v {
            assert_eq!(ceph_str_hash(CEPH_STR_HASH_RJENKINS, s), *rj, "{:?}", s);
            assert_eq!(ceph_str_hash(CEPH_STR_HASH_LINUX, s), *lin, "{:?}", s);
        }
        assert_eq!(ceph_str_hash(7, b"x"), u32::MAX);
    }

    #[test]
    fn crush_hash_reference_vectors() {
        assert_eq!(crush_hash32_2(CRUSH_HASH_RJENKINS1, 1, 2), 3079532188);
        assert_eq!(
            crush_hash32_2(CRUSH_HASH_RJENKINS1, 0xdeadbeef, 7),
            3745702617
        );
        assert_eq!(crush_hash32_3(CRUSH_HASH_RJENKINS1, 1, 2, 3), 1935332395);
        assert_eq!(crush_hash32_3(1, 1, 2, 3), 0);
    }

    #[test]
    fn stable_mod() {
        // pg_num 12 (mask 15): 13 -> 13 & 7 = 5; 11 -> 11.
        assert_eq!(ceph_stable_mod(13, 12, 15), 5);
        assert_eq!(ceph_stable_mod(11, 12, 15), 11);
        assert_eq!(ceph_stable_mod(0xffff_fff3, 8, 7), 3);
    }

    #[test]
    fn cbits_values() {
        assert_eq!(cbits(0), 0);
        assert_eq!(cbits(1), 1);
        assert_eq!(cbits(7), 3);
        assert_eq!(cbits(8), 4);
        assert_eq!(cbits(u32::MAX), 32);
    }
}
