//! READDIR ordering and paging: entries in `(frag_value(rjenkins(name)),
//! name)` order, resume after the last name or at an offset hash, END and
//! COMPLETE flags, entry and byte budgets.

mod common;

use std::collections::HashMap;

use common::*;
use engine::consts::*;
use engine::hash::ceph_str_hash_rjenkins;
use engine::*;

fn key(name: &str) -> (u32, Vec<u8>) {
    (
        ceph_str_hash_rjenkins(name.as_bytes()) & 0xff_ffff,
        name.as_bytes().to_vec(),
    )
}

fn expected_order(names: &[String]) -> Vec<String> {
    let mut v = names.to_vec();
    v.sort_by_key(|n| key(n));
    v
}

fn populate(fs: &mut Fs, dir: Ino, n: usize) -> Vec<String> {
    let mut names = Vec::new();
    for i in 0..n {
        let name = format!("entry-{i:04}");
        if i % 5 == 0 {
            fs.mkdir(C1, dir, &name).unwrap();
        } else {
            fs.mkfile(C1, dir, &name).unwrap();
        }
        names.push(name);
    }
    names
}

fn names_of(rd: &ReaddirReply) -> Vec<String> {
    rd.entries
        .iter()
        .map(|e| String::from_utf8(e.name.clone()).unwrap())
        .collect()
}

#[test]
fn full_listing_is_in_hash_order() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let names = populate(&mut fs, d, 200);
    let (rd, dirst) = fs.readdir(C2, d, 0, None, 0).unwrap();
    assert_eq!(names_of(&rd), expected_order(&names));
    assert_eq!(
        rd.flags,
        CEPH_READDIR_FRAG_END
            | CEPH_READDIR_FRAG_COMPLETE
            | CEPH_READDIR_HASH_ORDER
            | CEPH_READDIR_OFFSET_HASH
    );
    assert_eq!(rd.dirfrag.frag, 0);
    assert_eq!(rd.dirfrag.auth, -1);
    assert_eq!(dirst.ino, d, "the reply's target is the directory");
    assert_eq!(dirst.dirstat.nfiles + dirst.dirstat.nsubdirs, 200);
    // Hash order: non-decreasing frag values, which is what the kernel checks.
    let hashes: Vec<u32> = rd
        .entries
        .iter()
        .map(|e| key(std::str::from_utf8(&e.name).unwrap()).0)
        .collect();
    assert!(hashes.windows(2).all(|w| w[0] <= w[1]));
    // Every entry carries its inode and a cap for the reader.
    for e in &rd.entries {
        assert_eq!(e.lease.mask, 0);
        assert_ne!(e.inode.cap.cap_id, 0);
        assert_eq!(fs.e.lookup_name(d, &e.name), Some(e.inode.ino));
    }
    // Without REPLY_BITFLAGS the order flags are not set.
    let r = fs.call(
        C2,
        Op::Readdir {
            path: FilePath::ino(d),
            frag: 0,
            max_entries: 0,
            max_bytes: 0,
            last_name: None,
            offset_hash: 0,
            flags: 0,
        },
    );
    match r.extra {
        ReplyExtra::Readdir(rd) => {
            assert_eq!(rd.flags, CEPH_READDIR_FRAG_END | CEPH_READDIR_FRAG_COMPLETE)
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn paging_by_last_name() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let names = populate(&mut fs, d, 100);
    let mut got = Vec::new();
    let mut last: Option<String> = None;
    let mut pages = 0;
    loop {
        let (rd, _) = fs.readdir(C2, d, 7, last.as_deref(), 0).unwrap();
        pages += 1;
        assert!(rd.entries.len() <= 7, "never more than max_entries");
        let page = names_of(&rd);
        if rd.flags & CEPH_READDIR_FRAG_END != 0 {
            got.extend(page);
            assert_eq!(
                rd.flags & CEPH_READDIR_FRAG_COMPLETE,
                0,
                "a resumed listing is not complete"
            );
            break;
        }
        assert_eq!(page.len(), 7);
        last = page.last().cloned();
        got.extend(page);
    }
    assert_eq!(pages, 15);
    assert_eq!(got, expected_order(&names));
}

#[test]
fn paging_survives_concurrent_changes() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let names = populate(&mut fs, d, 40);
    let order = expected_order(&names);
    let (rd, _) = fs.readdir(C2, d, 10, None, 0).unwrap();
    let first = names_of(&rd);
    // The last returned name is removed, a new name is added.
    let last = first.last().unwrap().clone();
    let (dir_or_file, _) = (fs.e.lookup_name(d, last.as_bytes()).unwrap(), ());
    if fs.stat(dir_or_file).mode & S_IFMT == S_IFDIR {
        fs.rmdir(C1, d, &last).unwrap();
    } else {
        fs.unlink(C1, d, &last).unwrap();
    }
    let (rd, _) = fs.readdir(C2, d, 0, Some(&last), 0).unwrap();
    let rest = names_of(&rd);
    // Every entry present for the whole scan is returned exactly once.
    let mut all = first.clone();
    all.extend(rest);
    let expect: Vec<String> = order.iter().filter(|n| **n != last).cloned().collect();
    for n in &expect {
        assert_eq!(all.iter().filter(|x| *x == n).count(), 1, "{n}");
    }
}

#[test]
fn resume_by_offset_hash() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let names = populate(&mut fs, d, 60);
    let order = expected_order(&names);
    let pivot = &order[25];
    let h = key(pivot).0;
    let (rd, _) = fs.readdir(C2, d, 0, None, h).unwrap();
    let got = names_of(&rd);
    // Every name whose hash is at or after the offset.
    let expect: Vec<String> = order.iter().filter(|n| key(n).0 >= h).cloned().collect();
    assert_eq!(got, expect);
    assert_eq!(got[0], *pivot);
    assert_eq!(rd.flags & CEPH_READDIR_FRAG_COMPLETE, 0);
    assert_ne!(rd.flags & CEPH_READDIR_FRAG_END, 0);
}

#[test]
fn equal_hashes_are_ordered_by_name() {
    // Find three names with the same 24-bit hash.
    let mut seen: HashMap<u32, Vec<String>> = HashMap::new();
    let mut group = None;
    for i in 0..2_000_000u32 {
        let n = format!("c{i}");
        let v = seen.entry(key(&n).0).or_default();
        v.push(n);
        if v.len() == 2 {
            group = Some(v.clone());
            break;
        }
    }
    let group = group.expect("a 24-bit collision exists among a few thousand names");
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let mut names = group.clone();
    names.extend(["x".to_string(), "y".to_string(), "z".to_string()]);
    // Insert in reverse to make sure order does not follow insertion.
    for n in names.iter().rev() {
        fs.mkfile(C1, d, n).unwrap();
    }
    let all = fs.names(C2, d);
    assert_eq!(all, expected_order(&names));
    let mut sorted_group = group.clone();
    sorted_group.sort();
    let pos: Vec<usize> = sorted_group
        .iter()
        .map(|g| all.iter().position(|n| n == g).unwrap())
        .collect();
    assert_eq!(pos[1], pos[0] + 1, "colliding names are adjacent, by name");
    // Resuming after the first colliding name returns the second one next.
    let (rd, _) = fs.readdir(C2, d, 1, Some(&sorted_group[0]), 0).unwrap();
    assert_eq!(names_of(&rd), vec![sorted_group[1].clone()]);
    // Resuming at the shared hash returns both again (offset-hash semantics).
    let (rd, _) = fs.readdir(C2, d, 0, None, key(&sorted_group[0]).0).unwrap();
    assert_eq!(names_of(&rd)[..2], sorted_group[..]);
}

#[test]
fn byte_budget_and_empty_dirs() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let (rd, _) = fs.readdir(C2, d, 0, None, 0).unwrap();
    assert!(rd.entries.is_empty());
    assert_eq!(
        rd.flags & (CEPH_READDIR_FRAG_END | CEPH_READDIR_FRAG_COMPLETE),
        CEPH_READDIR_FRAG_END | CEPH_READDIR_FRAG_COMPLETE
    );
    populate(&mut fs, d, 30);
    let r = fs.call(
        C2,
        Op::Readdir {
            path: FilePath::ino(d),
            frag: 0,
            max_entries: 0,
            max_bytes: 2000,
            last_name: None,
            offset_hash: 0,
            flags: CEPH_READDIR_REPLY_BITFLAGS,
        },
    );
    match r.extra {
        ReplyExtra::Readdir(rd) => {
            assert!(!rd.entries.is_empty() && rd.entries.len() < 30);
            assert_eq!(rd.flags & CEPH_READDIR_FRAG_END, 0);
        }
        other => panic!("{other:?}"),
    }
    // Not a directory.
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    assert_eq!(fs.readdir(C2, f, 0, None, 0).unwrap_err(), Errno::ENOTDIR);
}

#[test]
fn entries_in_another_realm_get_no_caps() {
    let mut fs = Fs::new();
    let vols = fs.mkdir(C1, ROOT, "volumes").unwrap();
    let sv = fs.mkdir(C1, vols, "sv").unwrap();
    let plain = fs.mkdir(C1, vols, "plain").unwrap();
    fs.setxattr(C1, sv, "ceph.dir.subvolume", b"1", 0).unwrap();
    let (rd, _) = fs.readdir(C2, vols, 0, None, 0).unwrap();
    for e in &rd.entries {
        if e.inode.ino == sv {
            assert_eq!(
                e.inode.cap.caps, 0,
                "a realm root listed from its parent gets no caps"
            );
            assert_eq!(e.inode.cap.realm, sv);
            assert_eq!(e.inode.subvolume_id, sv);
        } else {
            assert_eq!(e.inode.ino, plain);
            assert_ne!(e.inode.cap.caps, 0);
            assert_eq!(e.inode.subvolume_id, 0);
        }
    }
}
