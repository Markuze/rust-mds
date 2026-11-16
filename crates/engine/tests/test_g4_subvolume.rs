//! G4 tenancy edges that go through the client request path.
//! Shard-actor cases live in `engine::shard` tests. The live cluster check
//! is `test_g4_cluster` in the daemon package.

mod common;

use common::*;
use engine::consts::*;
use engine::*;

#[test]
fn a_cleared_child_id_blocks_adopting_the_parent() {
    let mut fs = Fs::new();
    let a = fs.mkdir(C1, ROOT, "a").unwrap();
    let inner = fs.mkdir(C1, a, "inner").unwrap();
    fs.setxattr(C1, inner, "ceph.dir.subvolume", b"1", 0)
        .unwrap();
    fs.rmxattr(C1, inner, "ceph.dir.subvolume").unwrap();
    assert_eq!(fs.e.inode(inner).unwrap().stable_subvolume_id, inner);
    assert_eq!(
        fs.setxattr(C1, a, "ceph.dir.subvolume", b"1", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(fs.e.inode(a).unwrap().stable_subvolume_id, 0);
    assert!(
        fs.e.inode(a)
            .unwrap()
            .snaprealm
            .as_ref()
            .is_none_or(|realm| !realm.subvolume)
    );
}

#[test]
fn a_remote_link_is_not_stamped_by_adoption() {
    let mut fs = Fs::new();
    let a = fs.mkdir(C1, ROOT, "a").unwrap();
    let other = fs.mkdir(C1, ROOT, "other").unwrap();
    let file = fs.mkfile(C1, other, "f").unwrap();
    fs.link(C1, a, "l", file).unwrap();
    fs.setxattr(C1, a, "ceph.dir.subvolume", b"1", 0).unwrap();
    assert_eq!(fs.e.inode(a).unwrap().stable_subvolume_id, a);
    assert_eq!(fs.e.inode(file).unwrap().stable_subvolume_id, 0);
    assert_eq!(fs.getattr(C1, file, 0).unwrap().subvolume_id, 0);
}

#[test]
fn an_unlinked_file_keeps_its_auth_path() {
    let mut fs = Fs::new();
    let dir = fs.mkdir(C1, ROOT, "tenant").unwrap();
    let file = fs.mkfile(C1, dir, "open").unwrap();
    fs.open(C1, file, CEPH_O_RDONLY).unwrap();
    fs.unlink(C1, dir, "open").unwrap();
    let path = fs.e.authorization_path(file).expect("stray path");
    let text = String::from_utf8_lossy(&path);
    assert!(text.contains("tenant"), "{text}");
    assert!(text.contains("open"), "{text}");
}

#[test]
fn quota_far_from_the_limit_does_not_push_and_clearing_pushes_once() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    fs.setxattr(C1, d, "ceph.quota.max_files", b"1000", 0)
        .unwrap();
    assert!(
        !fs.last
            .iter()
            .any(|effect| matches!(effect, Effect::Quota { .. }))
    );
    fs.setxattr(C1, d, "ceph.quota.max_files", b"1", 0).unwrap();
    assert!(fs.last.iter().any(|effect| matches!(
        effect,
        Effect::Quota {
            ino,
            quota,
            ..
        } if *ino == d && quota.max_files == 1
    )));
    fs.rmxattr(C1, d, "ceph.quota.max_files").unwrap();
    assert!(fs.last.iter().any(|effect| matches!(
        effect,
        Effect::Quota {
            ino,
            quota,
            client,
            ..
        } if *ino == d && *client == C1 && !quota.is_enabled()
    )));
    fs.mkfile(C1, d, "later").unwrap();
    assert!(
        !fs.last
            .iter()
            .any(|effect| matches!(effect, Effect::Quota { .. }))
    );
}

#[test]
fn the_mds_answers_success_when_the_client_must_apply_edquot() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "q").unwrap();
    fs.setxattr(C1, d, "ceph.quota.max_files", b"1", 0).unwrap();
    assert!(fs.last.iter().any(|effect| matches!(
        effect,
        Effect::Quota {
            ino,
            quota,
            ..
        } if *ino == d && quota.max_files == 1
    )));
    fs.mkfile(C1, d, "over").unwrap();
    assert_eq!(fs.e.inode(d).unwrap().rstat.rfiles, 1);
}
