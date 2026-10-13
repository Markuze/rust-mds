//! Dentry name hashes (`src/common/ceph_hash.cc`, `include/ceph_hash.h`).
//!
//! A directory's `ceph_dir_layout.dl_dir_hash` names the function; the C++ MDS stores
//! `mds_default_dir_hash` = 2 (rjenkins) in new directories and treats 0 as linux
//! (`InodeStoreBase::hash_dentry_name`). Readdir order and client readdir offsets use the 24-bit
//! frag value of the hash ([`crate::frag::ceph_frag_value`]).

/// `CEPH_STR_HASH_LINUX`: the Linux dcache hash.
pub const CEPH_STR_HASH_LINUX: u8 = 1;
/// `CEPH_STR_HASH_RJENKINS`: Robert Jenkins' hash (the default for new directories).
pub const CEPH_STR_HASH_RJENKINS: u8 = 2;

#[inline(always)]
fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 13);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 8);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 13);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 12);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 16);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 5);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 3);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 10);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 15);
}

#[inline(always)]
fn le32(k: &[u8]) -> u32 {
    u32::from_le_bytes([k[0], k[1], k[2], k[3]])
}

/// `ceph_str_hash_rjenkins`: Jenkins' lookup2 hash with the golden-ratio seed, as Ceph uses it.
pub fn ceph_str_hash_rjenkins(name: &[u8]) -> u32 {
    let length = name.len() as u32;
    let (mut a, mut b, mut c) = (0x9e37_79b9u32, 0x9e37_79b9u32, 0u32);
    let mut k = name;
    while k.len() >= 12 {
        a = a.wrapping_add(le32(&k[0..4]));
        b = b.wrapping_add(le32(&k[4..8]));
        c = c.wrapping_add(le32(&k[8..12]));
        mix(&mut a, &mut b, &mut c);
        k = &k[12..];
    }
    c = c.wrapping_add(length);
    // The C switch falls through from the remaining length down to 1; the first byte of `c`
    // is reserved for the length, so bytes 8..10 are shifted by one byte.
    let n = k.len();
    if n >= 11 {
        c = c.wrapping_add((k[10] as u32) << 24);
    }
    if n >= 10 {
        c = c.wrapping_add((k[9] as u32) << 16);
    }
    if n >= 9 {
        c = c.wrapping_add((k[8] as u32) << 8);
    }
    if n >= 8 {
        b = b.wrapping_add((k[7] as u32) << 24);
    }
    if n >= 7 {
        b = b.wrapping_add((k[6] as u32) << 16);
    }
    if n >= 6 {
        b = b.wrapping_add((k[5] as u32) << 8);
    }
    if n >= 5 {
        b = b.wrapping_add(k[4] as u32);
    }
    if n >= 4 {
        a = a.wrapping_add((k[3] as u32) << 24);
    }
    if n >= 3 {
        a = a.wrapping_add((k[2] as u32) << 16);
    }
    if n >= 2 {
        a = a.wrapping_add((k[1] as u32) << 8);
    }
    if n >= 1 {
        a = a.wrapping_add(k[0] as u32);
    }
    mix(&mut a, &mut b, &mut c);
    c
}

/// `ceph_str_hash_linux`: the Linux dcache name hash.
pub fn ceph_str_hash_linux(name: &[u8]) -> u32 {
    let mut hash = 0u32;
    for &c in name {
        let c = c as u32;
        hash = hash
            .wrapping_add(c << 4)
            .wrapping_add(c >> 4)
            .wrapping_mul(11);
    }
    hash
}

/// `ceph_str_hash`: hash `name` with function `kind`; an unknown kind yields `0xffffffff`.
pub fn ceph_str_hash(kind: u8, name: &[u8]) -> u32 {
    match kind {
        CEPH_STR_HASH_LINUX => ceph_str_hash_linux(name),
        CEPH_STR_HASH_RJENKINS => ceph_str_hash_rjenkins(name),
        _ => u32::MAX,
    }
}

/// `InodeStoreBase::hash_dentry_name`: the hash of a dentry name in a directory whose
/// `dl_dir_hash` is `dir_hash` (0 means linux).
pub fn hash_dentry_name(dir_hash: u8, name: &[u8]) -> u32 {
    let kind = if dir_hash == 0 {
        CEPH_STR_HASH_LINUX
    } else {
        dir_hash
    };
    ceph_str_hash(kind, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vectors produced by compiling Ceph's `src/common/ceph_hash.cc` (name, rjenkins, linux).
    const VECTORS: &[(&[u8], u32, u32)] = &[
        (b"", 0xbd49d10d, 0x00000000),
        (b"a", 0x29eec818, 0x000042f2),
        (b"ab", 0x9879ac41, 0x00032408),
        (b"abc", 0x251e4793, 0x0022d0aa),
        (b"foo", 0x7fc1f406, 0x0024db2a),
        (b"bar", 0xefe6384b, 0x002326a5),
        (b"hello", 0xb706399e, 0x11990232),
        (b"file1", 0x0a086551, 0x11584b61),
        (b"file2", 0xbe5b00c1, 0x11584c11),
        (b"dir", 0x63d72e26, 0x00240985),
        (b".snap", 0xd5266b31, 0x08d943e2),
        (b"abcdefghijk", 0xe52b8e4c, 0x73230f0a),
        (b"abcdefghijkl", 0x0b1b3ea5, 0xf281eff0),
        (b"abcdefghijklm", 0x3122b031, 0x6b959a82),
        (
            b"0123456789abcdefghijklmnopqrstuvwxyz",
            0xbfbe133d,
            0x29ed3c1b,
        ),
        (b"\xff\xfe\x80 high bytes", 0x4ffc6ce0, 0x4a4b5fad),
        (b"lost+found", 0x939d89b6, 0xfbd7df7b),
        (
            b"a much longer file name used to exercise several twelve byte rounds of mixing",
            0x0489fe96,
            0x381ccc96,
        ),
    ];

    #[test]
    fn matches_ceph_reference() {
        for (name, rj, lx) in VECTORS {
            assert_eq!(
                ceph_str_hash_rjenkins(name),
                *rj,
                "rjenkins {:?}",
                String::from_utf8_lossy(name)
            );
            assert_eq!(
                ceph_str_hash_linux(name),
                *lx,
                "linux {:?}",
                String::from_utf8_lossy(name)
            );
            assert_eq!(ceph_str_hash(CEPH_STR_HASH_RJENKINS, name), *rj);
            assert_eq!(hash_dentry_name(0, name), *lx);
            assert_eq!(hash_dentry_name(2, name), *rj);
        }
        assert_eq!(ceph_str_hash(7, b"x"), u32::MAX);
    }
}
