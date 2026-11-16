//! Smaller paths: tick, pools, paths, special names, mknod of a directory,
//! FLUSHSNAP, inode-number exhaustion, pjdfstest-style time checks.

mod common;

use common::*;
use engine::consts::*;
use engine::*;

#[test]
fn tick_flushes_and_releases() {
    let mut fs = Fs::manual();
    let (tid, _) = fs.send(
        C1,
        Op::Mkdir {
            path: FilePath::child(ROOT, "d"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let flushes = fs.e.journal().flush_count();
    let eff = fs.e.tick();
    assert!(eff.is_empty(), "nothing committed yet");
    assert!(
        fs.e.journal().flush_count() > flushes,
        "tick asks for a commit when records are pending"
    );
    fs.e.journal_mut().commit_all();
    let eff = fs.e.tick();
    assert!(reply_for(&eff, C1, tid).is_some());
}

#[test]
fn pools_paths_and_dot_names() {
    let mut fs = Fs::new();
    let a = fs.mkdir(C1, ROOT, "a").unwrap();
    let b = fs.mkdir(C1, a, "b").unwrap();
    assert_eq!(fs.e.path_of(b).unwrap(), "/a/b");
    assert_eq!(fs.e.path_of(ROOT).unwrap(), "/");
    assert_eq!(
        fs.e.resolve_path(&FilePath::parse(b, "../b/./..")).unwrap(),
        a
    );
    assert_eq!(
        fs.e.resolve_path(&FilePath::parse(ROOT, "..")).unwrap(),
        ROOT
    );
    // LOOKUP of "." and ".." answers with the inode and no dentry.
    let r = fs.call(
        C1,
        Op::Lookup {
            path: FilePath::child(b, ".."),
            mask: 0,
        },
    );
    assert_eq!(r.target.unwrap().ino, a);
    assert!(r.dentry.is_none());
    // New pools from a newer map.
    let mut names = fs.e.config().pool_names.clone();
    names.insert(9, "new".into());
    fs.e.set_pools(vec![2, 3, 9], names);
    fs.setxattr(C1, a, "ceph.dir.layout.pool", b"new", 0)
        .unwrap();
    let f = fs.mkfile(C1, b, "f").unwrap();
    assert_eq!(fs.stat(f).layout.pool_id, 9);
    assert_eq!(
        fs.getvxattr(C1, f, "ceph.file.layout.pool_name").unwrap(),
        "new"
    );
}

#[test]
fn mknod_of_a_directory_makes_a_directory() {
    let mut fs = Fs::new();
    let d = fs.mknod(C1, ROOT, "d", S_IFDIR | 0o700, 0).unwrap();
    let c = fs.stat(d);
    assert!(c.is_dir());
    assert_eq!(c.dir_hash, CEPH_STR_HASH_RJENKINS);
    assert_eq!(fs.stat(ROOT).dirstat.nsubdirs, 1);
    fs.mkfile(C1, d, "x").unwrap();
}

#[test]
fn flushsnap_is_acknowledged() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let cap = fs.e.cap(f, C1).unwrap().clone();
    let eff = fs.cap_update(
        C1,
        CapUpdate {
            op: CEPH_CAP_OP_FLUSHSNAP,
            ino: f,
            cap_id: 0,
            seq: 0,
            mseq: cap.mseq,
            dirty: FW,
            flush_tid: 3,
            ..Default::default()
        },
    );
    let m = caps_to(&eff, C1);
    assert_eq!(m.len(), 1);
    assert_eq!((m[0].op, m[0].flush_tid), (CEPH_CAP_OP_FLUSHSNAP_ACK, 3));
    // From a client without a cap: dropped.
    let eff = fs.cap_update(
        C2,
        CapUpdate {
            op: CEPH_CAP_OP_FLUSHSNAP,
            ino: f,
            flush_tid: 4,
            ..Default::default()
        },
    );
    assert!(eff.is_empty());
}

#[test]
fn inode_numbers_run_out_cleanly() {
    let mut cfg = config();
    cfg.ino_lease = InoRange::new(0x10_0000, 3);
    let mut fs = Fs::with(cfg, MemJournal::new());
    for n in ["a", "b", "c"] {
        fs.mkfile(C1, ROOT, n).unwrap();
    }
    assert_eq!(fs.mkfile(C1, ROOT, "d"), Err(Errno::ENOSPC));
    assert_eq!(fs.mkdir(C1, ROOT, "e"), Err(Errno::ENOSPC));
    assert!(
        fs.e.lookup_name(ROOT, b"d").is_none(),
        "nothing half-created"
    );
    assert_eq!(fs.stat(ROOT).dirstat.nfiles, 3);
}

/// pjdfstest rename/00.t, link/00.t, unlink/00.t, mkdir/00.t, chmod/00.t,
/// truncate/00.t: which times change.
#[test]
fn pjd_time_updates() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let f = fs.mkfile(C1, d, "f").unwrap();
    let snap = |fs: &Fs, i| {
        let c = fs.e.inode(i).unwrap();
        (c.ctime, c.mtime)
    };
    // link: file ctime, dir mtime+ctime.
    let (fc, fm) = snap(&fs, f);
    let (dc, dm) = snap(&fs, d);
    fs.link(C1, d, "g", f).unwrap();
    assert!(snap(&fs, f).0 > fc && snap(&fs, f).1 == fm);
    assert!(snap(&fs, d).0 > dc && snap(&fs, d).1 > dm);
    // unlink of one name: file ctime, dir mtime+ctime.
    let (fc, fm) = snap(&fs, f);
    let (dc, dm) = snap(&fs, d);
    fs.unlink(C1, d, "g").unwrap();
    assert!(snap(&fs, f).0 > fc && snap(&fs, f).1 == fm);
    assert!(snap(&fs, d).0 > dc && snap(&fs, d).1 > dm);
    // rename: the file's ctime; both directories' mtime+ctime.
    let e = fs.mkdir(C1, ROOT, "e").unwrap();
    let (fc, fm) = snap(&fs, f);
    let (dc, dm) = snap(&fs, d);
    let (ec, em) = snap(&fs, e);
    fs.rename(C1, d, "f", e, "f2").unwrap();
    assert!(snap(&fs, f).0 > fc && snap(&fs, f).1 == fm);
    assert!(snap(&fs, d).0 > dc && snap(&fs, d).1 > dm);
    assert!(snap(&fs, e).0 > ec && snap(&fs, e).1 > em);
    // chmod: ctime only.
    let (fc, fm) = snap(&fs, f);
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_MODE,
            mode: 0o600,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(snap(&fs, f).0 > fc && snap(&fs, f).1 == fm);
    // truncate: ctime and mtime; the parent is untouched.
    let (fc, fm) = snap(&fs, f);
    let (ec, em) = snap(&fs, e);
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 123,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(snap(&fs, f).0 > fc && snap(&fs, f).1 > fm);
    assert_eq!(snap(&fs, e), (ec, em));
    // setxattr: ctime only.
    let (fc, fm) = snap(&fs, f);
    fs.setxattr(C1, f, "user.x", b"1", 0).unwrap();
    assert!(snap(&fs, f).0 > fc && snap(&fs, f).1 == fm);
    // rmdir: the parent's mtime+ctime.
    let sub = fs.mkdir(C1, e, "sub").unwrap();
    let _ = sub;
    let (ec, em) = snap(&fs, e);
    fs.rmdir(C1, e, "sub").unwrap();
    assert!(snap(&fs, e).0 > ec && snap(&fs, e).1 > em);
}

#[test]
fn directory_nlink_and_size_contract() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    for i in 0..3 {
        fs.mkdir(C1, d, &format!("s{i}")).unwrap();
    }
    fs.mkfile(C1, d, "f").unwrap();
    let st = fs.getattr(C2, d, CEPH_STAT_CAP_INODE_ALL).unwrap();
    // libcephfs aborts on a directory nlink other than 0 or 1; clients show
    // 2 + nsubdirs.
    assert_eq!(st.nlink, 1);
    assert_eq!(st.size, 0);
    assert_eq!((st.dirstat.nfiles, st.dirstat.nsubdirs), (1, 3));
    assert_eq!((st.rstat.rfiles, st.rstat.rsubdirs), (1, 4));
}
