//! Operations `mgr/volumes` uses on a subvolume root.
//! This is the MDS side of that lifecycle. It is not the mgr module.

mod common;

use common::*;
use engine::*;

#[test]
fn volumes_can_mark_rename_and_refuse_a_nonempty_subvolume() {
    let mut fs = Fs::new();
    let left = fs.mkdir(C1, ROOT, "left").unwrap();
    let right = fs.mkdir(C1, ROOT, "right").unwrap();
    let vol = fs.mkdir(C1, left, "vol").unwrap();
    fs.setxattr(C1, vol, "ceph.dir.subvolume", b"1", 0).unwrap();
    assert_eq!(fs.getvxattr(C1, vol, "ceph.dir.subvolume").unwrap(), "1");
    fs.setxattr(C1, vol, "ceph.quota.max_bytes", b"4096", 0)
        .unwrap();
    assert_eq!(
        fs.getvxattr(C1, vol, "ceph.quota.max_bytes").unwrap(),
        "4096"
    );
    fs.rename(C1, left, "vol", right, "vol").unwrap();
    let moved = fs.lookup(C1, right, "vol").unwrap();
    assert_eq!(moved, vol);
    assert_eq!(fs.e.inode(vol).unwrap().stable_subvolume_id, vol);
    let inner = fs.mkdir(C1, vol, "inner").unwrap();
    assert_ne!(inner, vol);
    assert_eq!(fs.rmdir(C1, right, "vol").unwrap_err(), Errno::ENOTEMPTY);
    fs.rmdir(C1, vol, "inner").unwrap();
    let link = fs.symlink(C1, vol, "s", "target-name").unwrap();
    assert_eq!(fs.e.inode(link).unwrap().size, b"target-name".len() as u64);
    fs.rmxattr(C1, vol, "ceph.dir.subvolume").unwrap_err();
    fs.rmdir(C1, vol, "s").unwrap_err();
    fs.unlink(C1, vol, "s").unwrap();
    fs.rmxattr(C1, vol, "ceph.dir.subvolume").unwrap();
    assert_eq!(fs.e.inode(vol).unwrap().stable_subvolume_id, vol);
}
