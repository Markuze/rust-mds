//! POSIX namespace semantics (pjdfstest-style cases for the parts the MDS
//! enforces: existence, types, emptiness, link counts, timestamps,
//! change_attr, setgid inheritance, truncation, renames).

mod common;

use common::*;
use engine::consts::*;
use engine::*;

#[test]
fn root_inode() {
    let mut fs = Fs::new();
    let st = fs.getattr(C1, ROOT, CEPH_STAT_CAP_INODE_ALL).unwrap();
    assert_eq!(st.ino, 1);
    assert_eq!(st.mode, S_IFDIR | 0o755);
    assert_eq!(st.nlink, 1);
    assert_eq!(st.size, 0);
    assert_eq!(
        st.layout,
        FileLayout::default_for_pool(2),
        "explicit default layout on /"
    );
    assert_eq!(st.dir_hash, CEPH_STR_HASH_RJENKINS);
    assert_eq!(st.inline_version, CEPH_INLINE_NONE);
    assert_eq!(st.rstat.rsubdirs, 1);
    assert_eq!(st.cap.realm, 1);
    assert_eq!(st.cap.flags, CEPH_CAP_FLAG_AUTH);
    assert_eq!(st.snapid, CEPH_NOSNAP);
}

#[test]
fn mkdir_sets_times_counts_and_owner() {
    let mut fs = Fs::new();
    let before = fs.stat(ROOT);
    let (tid, eff) = fs.send(
        C1,
        Op::Mkdir {
            path: FilePath::child(ROOT, "a"),
            mode: 0o750,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let r = reply_for(&eff, C1, tid).unwrap();
    assert_eq!(r.result, 0);
    let st = r.target.as_ref().unwrap();
    let a = st.ino;
    assert!((RANK0_INO_START..RANK0_INO_START + RANK0_INO_LEN).contains(&a));
    assert_eq!(st.mode, S_IFDIR | 0o750);
    assert_eq!(st.nlink, 1, "directories report nlink 1");
    assert_eq!((st.uid, st.gid), (1000, 1000));
    assert_eq!(st.truncate_seq, 1);
    assert_eq!(st.truncate_size, u64::MAX);
    assert_eq!(st.change_attr, 0);
    assert_eq!(st.btime, st.ctime);
    assert_eq!(st.mtime, st.atime);
    // The dentry trace names the request's parent.
    let d = r.dentry.as_ref().unwrap();
    assert_eq!(d.dir.ino, ROOT);
    assert_eq!(d.name, b"a");
    assert_eq!(d.lease.mask, 0, "null lease");
    let root = fs.stat(ROOT);
    assert_eq!(root.dirstat.nsubdirs, 1);
    assert_eq!(root.mtime, st.mtime);
    assert_eq!(root.ctime, st.mtime);
    assert_eq!(root.change_attr, before.change_attr + 1);
    assert!(root.version > before.version);
    assert_eq!(root.rstat.rsubdirs, 2);
}

#[test]
fn create_gates() {
    let mut fs = Fs::new();
    fs.mkdir(C1, ROOT, "d").unwrap();
    assert_eq!(fs.mkdir(C1, ROOT, "d"), Err(Errno::EEXIST));
    assert_eq!(fs.mkdir(C1, ROOT, "."), Err(Errno::EEXIST));
    assert_eq!(fs.mkdir(C1, ROOT, ".."), Err(Errno::EEXIST));
    assert_eq!(fs.mkdir(C1, ROOT, ""), Err(Errno::EINVAL));
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    assert_eq!(fs.mkdir(C1, f, "x"), Err(Errno::ENOTDIR));
    assert_eq!(fs.mkfile(C1, ROOT, "d"), Err(Errno::EEXIST));
    assert_eq!(fs.symlink(C1, ROOT, "f", "x"), Err(Errno::EEXIST));
    // Multi-component paths resolve; missing components give ENOENT.
    assert_eq!(
        fs.try_call(
            C1,
            Op::Mkdir {
                path: FilePath::parse(ROOT, "nope/x"),
                mode: 0o755,
                xattrs: XattrMap::new(),
                ino: 0
            }
        )
        .unwrap_err(),
        Errno::ENOENT
    );
    let r = fs.try_call(
        C1,
        Op::Mkdir {
            path: FilePath::parse(ROOT, "d/x"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    assert!(r.is_ok());
    // A path without components.
    assert_eq!(
        fs.try_call(
            C1,
            Op::Mkdir {
                path: FilePath::ino(ROOT),
                mode: 0o755,
                xattrs: XattrMap::new(),
                ino: 0
            }
        )
        .unwrap_err(),
        Errno::EINVAL
    );
    // Unknown and MDS-private base inodes.
    assert_eq!(fs.mkdir(C1, 0x2000_0000_0000, "x"), Err(Errno::ESTALE));
    assert_eq!(fs.mkdir(C1, 0x100, "x"), Err(Errno::ESTALE));
    assert_eq!(fs.getattr(C1, 0x600, 0).unwrap_err(), Errno::ESTALE);
}

#[test]
fn create_excl_and_existing() {
    let mut fs = Fs::new();
    let r = fs.create_reply(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let ino = r.target.as_ref().unwrap().ino;
    assert_eq!(
        r.extra,
        ReplyExtra::Create {
            ino,
            delegated: vec![]
        }
    );
    assert_eq!(r.created_ino, Some(ino));
    assert_eq!(
        fs.create(C1, ROOT, "f", CEPH_O_RDWR | CEPH_O_EXCL),
        Err(Errno::EEXIST)
    );
    // Without O_EXCL the existing file is opened: no created ino.
    let r2 = fs.create_reply(C2, ROOT, "f", CEPH_O_RDONLY).unwrap();
    assert_eq!(r2.target.as_ref().unwrap().ino, ino);
    assert_eq!(r2.extra, ReplyExtra::None);
    assert_eq!(r2.created_ino, None);
    assert_eq!(r2.dentry.as_ref().unwrap().name, b"f");
    // Invalid layout override.
    let r = fs.try_call(
        C1,
        Op::Create {
            path: FilePath::child(ROOT, "bad"),
            flags: CEPH_O_CREAT | CEPH_O_RDWR,
            mode: 0o644,
            layout: LayoutOverride {
                stripe_unit: 1000,
                ..Default::default()
            },
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    assert_eq!(r.unwrap_err(), Errno::EINVAL);
    let r = fs.try_call(
        C1,
        Op::Create {
            path: FilePath::child(ROOT, "badpool"),
            flags: CEPH_O_CREAT | CEPH_O_RDWR,
            mode: 0o644,
            layout: LayoutOverride {
                pool: 1,
                ..Default::default()
            },
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    assert_eq!(
        r.unwrap_err(),
        Errno::EINVAL,
        "metadata pool is not a data pool"
    );
    let r = fs
        .try_call(
            C1,
            Op::Create {
                path: FilePath::child(ROOT, "fast"),
                flags: CEPH_O_CREAT | CEPH_O_RDWR,
                mode: 0o644,
                layout: LayoutOverride {
                    pool: 3,
                    stripe_unit: 1 << 20,
                    ..Default::default()
                },
                xattrs: XattrMap::new(),
                ino: 0,
            },
        )
        .unwrap();
    let l = &r.target.unwrap().layout;
    assert_eq!(
        (l.pool_id, l.stripe_unit, l.object_size),
        (3, 1 << 20, 4 << 20)
    );
}

#[test]
fn new_inode_fields_and_initial_xattrs() {
    let mut fs = Fs::new();
    let mut x = XattrMap::new();
    x.insert(
        b"system.posix_acl_access".to_vec(),
        vec![2, 0, 0, 0, 1, 0, 6, 0],
    );
    x.insert(b"security.selinux".to_vec(), b"ctx".to_vec());
    let r = fs
        .try_call(
            C1,
            Op::Create {
                path: FilePath::child(ROOT, "f"),
                flags: CEPH_O_CREAT | CEPH_O_WRONLY,
                mode: 0o640 | S_IFDIR, // type bits are replaced by S_IFREG
                layout: LayoutOverride::default(),
                xattrs: x.clone(),
                ino: 0,
            },
        )
        .unwrap();
    let st = r.target.unwrap();
    assert_eq!(st.mode, S_IFREG | 0o640);
    assert_eq!(st.layout, FileLayout::default_for_pool(2));
    assert_eq!(st.size, 0);
    assert_eq!(st.nlink, 1);
    assert_eq!(st.rstat.rfiles, 1);
    assert_eq!(fs.stat(st.ino).xattrs, x, "initial xattrs stored as sent");
    assert_eq!(
        st.xattrs.as_ref(),
        Some(&x),
        "a new cap gets the xattr blob"
    );
}

#[test]
fn setgid_directory_inheritance() {
    let mut fs = Fs::new();
    let d = fs.mkdir_mode(C1, ROOT, "sg", 0o2775).unwrap();
    fs.setattr(
        C1,
        d,
        SetattrArgs {
            mask: CEPH_SETATTR_GID,
            gid: 77,
            ..Default::default()
        },
    )
    .unwrap();
    let f = fs.mkfile(C1, d, "f").unwrap();
    let sub = fs.mkdir(C1, d, "sub").unwrap();
    assert_eq!(fs.stat(f).gid, 77);
    assert_eq!(fs.stat(f).uid, 1000);
    assert_eq!(fs.stat(f).mode & S_ISGID, 0);
    assert_eq!(fs.stat(sub).gid, 77);
    assert_ne!(
        fs.stat(sub).mode & S_ISGID,
        0,
        "new subdirectories inherit S_ISGID"
    );
    let plain = fs.mkdir(C1, ROOT, "plain").unwrap();
    let g = fs.mkfile(C1, plain, "g").unwrap();
    assert_eq!(fs.stat(g).gid, 1000);
}

#[test]
fn mknod_types() {
    let mut fs = Fs::new();
    let fifo = fs.mknod(C1, ROOT, "fifo", S_IFIFO | 0o600, 0).unwrap();
    let sock = fs.mknod(C1, ROOT, "sock", S_IFSOCK | 0o600, 0).unwrap();
    let chr = fs.mknod(C1, ROOT, "chr", S_IFCHR | 0o600, 0x0103).unwrap();
    let blk = fs.mknod(C1, ROOT, "blk", S_IFBLK | 0o600, 0x0801).unwrap();
    let reg = fs.mknod(C1, ROOT, "reg", 0o600, 0).unwrap();
    assert_eq!(fs.stat(fifo).mode, S_IFIFO | 0o600);
    assert_eq!(fs.stat(sock).mode, S_IFSOCK | 0o600);
    assert_eq!(
        (fs.stat(chr).mode, fs.stat(chr).rdev),
        (S_IFCHR | 0o600, 0x0103)
    );
    assert_eq!(fs.stat(blk).rdev, 0x0801);
    assert_eq!(
        fs.stat(reg).mode,
        S_IFREG | 0o600,
        "type 0 means a regular file"
    );
    assert_eq!(fs.stat(ROOT).dirstat.nfiles, 5);
    assert_eq!(fs.stat(ROOT).rstat.rfiles, 5);
}

#[test]
fn symlink_target_and_size() {
    let mut fs = Fs::new();
    let s = fs.symlink(C1, ROOT, "s", "some/where/else").unwrap();
    let st = fs.getattr(C1, s, CEPH_STAT_CAP_INODE_ALL).unwrap();
    assert_eq!(st.mode, S_IFLNK | 0o777);
    assert_eq!(st.symlink, b"some/where/else");
    assert_eq!(st.size, 15);
    assert_eq!(st.rstat.rbytes, 15);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 15);
    // Symlinks are never followed by the MDS.
    assert_eq!(fs.mkdir(C1, s, "x"), Err(Errno::ENOTDIR));
}

#[test]
fn lookup_positive_and_negative() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let f = fs.mkfile(C1, d, "f").unwrap();
    let r = fs.call(
        C1,
        Op::Lookup {
            path: FilePath::child(d, "f"),
            mask: CEPH_STAT_CAP_INODE | AS,
        },
    );
    assert_eq!(r.result, 0);
    assert_eq!(r.target.as_ref().unwrap().ino, f);
    assert_eq!(r.dentry.as_ref().unwrap().dir.ino, d);
    assert_eq!(r.snap_realm, Some(1));
    // Negative lookup with WANT_DENTRY: parent and name, no target.
    let r = fs.call(
        C1,
        Op::Lookup {
            path: FilePath::child(d, "nope"),
            mask: 0,
        },
    );
    assert_eq!(r.result, Errno::ENOENT.as_neg());
    assert!(r.target.is_none());
    assert_eq!(r.dentry.as_ref().unwrap().name, b"nope");
    assert_eq!(r.dentry.as_ref().unwrap().dir.ino, d);
    // Without WANT_DENTRY: traceless ENOENT.
    let req = Request::new(
        0,
        Op::Lookup {
            path: FilePath::child(d, "nope"),
            mask: 0,
        },
    );
    let (tid, eff) = fs.send_req(C1, req);
    let r = reply_for(&eff, C1, tid).unwrap();
    assert_eq!(r.result, Errno::ENOENT.as_neg());
    assert!(r.dentry.is_none());
    // LOOKUP with no name is EINVAL; lookups through a file are ENOTDIR.
    assert_eq!(
        fs.try_call(
            C1,
            Op::Lookup {
                path: FilePath::ino(d),
                mask: 0
            }
        )
        .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(fs.lookup(C1, f, "x"), Err(Errno::ENOTDIR));
    // The client's snapdir name does not exist on the MDS.
    assert_eq!(fs.lookup(C1, d, ".snap"), Err(Errno::ENOENT));
    // Multi-component getattr (mount of a subdirectory).
    let r = fs.call(
        C1,
        Op::Getattr {
            path: FilePath::parse(ROOT, "d/f"),
            mask: 0,
        },
    );
    assert_eq!(r.target.unwrap().ino, f);
}

#[test]
fn lookupino_parent_name() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let f = fs.mkfile(C1, d, "f").unwrap();
    let r = fs.call(C2, Op::LookupIno { ino: f, mask: 0 });
    assert_eq!(r.target.unwrap().ino, f);
    assert_eq!(
        fs.try_call(
            C2,
            Op::LookupIno {
                ino: 0x123,
                mask: 0
            }
        )
        .unwrap_err(),
        Errno::ESTALE
    );
    assert_eq!(
        fs.try_call(
            C2,
            Op::LookupIno {
                ino: 0xfff,
                mask: 0
            }
        )
        .unwrap_err(),
        Errno::ESTALE
    );
    assert_eq!(
        fs.try_call(
            C2,
            Op::LookupIno {
                ino: 0x1_0000_0000_0099,
                mask: 0
            }
        )
        .unwrap_err(),
        Errno::ESTALE
    );
    let r = fs.call(C2, Op::LookupParent { ino: f });
    assert_eq!(r.target.unwrap().ino, d);
    assert_eq!(
        fs.try_call(C2, Op::LookupParent { ino: ROOT }).unwrap_err(),
        Errno::EINVAL
    );
    let r = fs.call(
        C2,
        Op::LookupName {
            ino: f,
            parent: None,
        },
    );
    assert_eq!(r.dentry.as_ref().unwrap().name, b"f");
    assert_eq!(r.dentry.as_ref().unwrap().dir.ino, d);
    assert_eq!(r.target.unwrap().ino, f, "LOOKUPNAME must carry the target");
    assert_eq!(
        fs.try_call(
            C2,
            Op::LookupName {
                ino: f,
                parent: Some(ROOT)
            }
        )
        .unwrap_err(),
        Errno::ENOENT
    );
    assert_eq!(
        fs.try_call(
            C2,
            Op::LookupName {
                ino: ROOT,
                parent: None
            }
        )
        .unwrap_err(),
        Errno::ENOENT
    );
}

#[test]
fn hard_links() {
    let mut fs = Fs::new();
    let d1 = fs.mkdir(C1, ROOT, "d1").unwrap();
    let d2 = fs.mkdir(C1, ROOT, "d2").unwrap();
    let f = fs.mkfile(C1, d1, "f").unwrap();
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 100,
            ..Default::default()
        },
    )
    .unwrap();
    let before = fs.stat(f);
    let r = fs.link(C1, d2, "g", f).unwrap();
    assert_eq!(r.target.as_ref().unwrap().ino, f);
    assert_eq!(r.target.as_ref().unwrap().nlink, 2);
    assert_eq!(r.dentry.as_ref().unwrap().dir.ino, d2);
    let after = fs.stat(f);
    assert_eq!(after.nlink, 2);
    assert!(after.ctime > before.ctime);
    assert_eq!(after.change_attr, before.change_attr + 1);
    assert_eq!(
        fs.stat(d2).dirstat.nfiles,
        1,
        "remote dentries count in dirstat"
    );
    // rstat is accounted through the primary link only.
    assert_eq!(fs.stat(d2).rstat.rbytes, 0);
    assert_eq!(fs.stat(d1).rstat.rbytes, 100);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 100);
    assert_eq!(fs.stat(ROOT).rstat.rfiles, 1);
    assert_eq!(fs.lookup(C1, d2, "g").unwrap(), f);
    // Errors.
    assert_eq!(fs.link(C1, d2, "g", f).unwrap_err(), Errno::EEXIST);
    assert_eq!(fs.link(C1, d2, "dirlink", d1).unwrap_err(), Errno::EINVAL);
    assert_eq!(fs.link(C1, d2, ".", f).unwrap_err(), Errno::EEXIST);
    assert_eq!(
        fs.link(C1, d2, "z", 0x1_0000_0000_0099).unwrap_err(),
        Errno::ESTALE
    );
    // Link by path2 = parent + name.
    let r = fs
        .try_call(
            C1,
            Op::Link {
                path: FilePath::child(ROOT, "h"),
                target: FilePath::child(d1, "f"),
            },
        )
        .unwrap();
    assert_eq!(r.target.unwrap().nlink, 3);
    // LOOKUPNAME names the primary link.
    let r = fs.call(
        C1,
        Op::LookupName {
            ino: f,
            parent: None,
        },
    );
    assert_eq!(
        (
            r.dentry.as_ref().unwrap().dir.ino,
            r.dentry.as_ref().unwrap().name.clone()
        ),
        (d1, b"f".to_vec())
    );
}

#[test]
fn unlink_primary_moves_primary_to_remaining_link() {
    let mut fs = Fs::new();
    let d1 = fs.mkdir(C1, ROOT, "d1").unwrap();
    let d2 = fs.mkdir(C1, ROOT, "d2").unwrap();
    let f = fs.mkfile(C1, d1, "f").unwrap();
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 7,
            ..Default::default()
        },
    )
    .unwrap();
    fs.link(C1, d2, "g", f).unwrap();
    fs.unlink(C1, d1, "f").unwrap();
    assert_eq!(fs.lookup(C1, d1, "f"), Err(Errno::ENOENT));
    assert_eq!(fs.lookup(C1, d2, "g").unwrap(), f);
    assert_eq!(fs.stat(f).nlink, 1);
    assert_eq!(fs.e.parent_of(f), Some((d2, b"g".to_vec())));
    assert_eq!(fs.stat(d1).rstat.rbytes, 0);
    assert_eq!(
        fs.stat(d2).rstat.rbytes,
        7,
        "rstat follows the new primary link"
    );
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 7);
    let r = fs.call(
        C1,
        Op::LookupName {
            ino: f,
            parent: None,
        },
    );
    assert_eq!(r.dentry.unwrap().name, b"g");
    fs.unlink(C1, d2, "g").unwrap();
    assert_eq!(fs.stat(ROOT).rstat.rfiles, 0);
}

#[test]
fn unlink_and_rmdir_errors() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    fs.mkfile(C1, d, "f").unwrap();
    assert_eq!(fs.unlink(C1, ROOT, "d").unwrap_err(), Errno::EISDIR);
    assert_eq!(fs.rmdir(C1, d, "f").unwrap_err(), Errno::ENOTDIR);
    assert_eq!(fs.rmdir(C1, ROOT, "d").unwrap_err(), Errno::ENOTEMPTY);
    assert_eq!(fs.unlink(C1, ROOT, "nope").unwrap_err(), Errno::ENOENT);
    assert_eq!(fs.rmdir(C1, ROOT, "nope").unwrap_err(), Errno::ENOENT);
    assert_eq!(fs.unlink(C1, d, ".").unwrap_err(), Errno::ENOTEMPTY);
    assert_eq!(fs.rmdir(C1, d, "..").unwrap_err(), Errno::ENOTEMPTY);
    let before = fs.stat(d);
    let r = fs.unlink(C1, d, "f").unwrap();
    assert!(r.target.is_none(), "unlink replies with the dentry only");
    assert_eq!(r.dentry.unwrap().name, b"f");
    let after = fs.stat(d);
    assert_eq!(after.dirstat.nfiles, 0);
    assert!(after.mtime > before.mtime);
    assert_eq!(after.change_attr, before.change_attr + 1);
    fs.rmdir(C1, ROOT, "d").unwrap();
    assert_eq!(fs.stat(ROOT).dirstat.nsubdirs, 0);
    assert_eq!(fs.stat(ROOT).rstat.rsubdirs, 1);
}

#[test]
fn create_in_removed_directory_fails() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    // C2 keeps the directory alive with a cap.
    fs.getattr(C2, d, 0).unwrap();
    fs.rmdir(C1, ROOT, "d").unwrap();
    assert_eq!(fs.stat(d).nlink, 0, "removed directories report nlink 0");
    assert_eq!(fs.mkfile(C2, d, "x"), Err(Errno::ENOENT));
    assert_eq!(
        fs.rename(C2, ROOT, "nothing", d, "x").unwrap_err(),
        Errno::ENOENT
    );
}

#[test]
fn timestamps_from_client_clock_only_move_dir_mtime_forward() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let req = Request::new(
        0,
        Op::Mknod {
            path: FilePath::child(d, "a"),
            mode: S_IFREG,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_stamp(ts(5000))
    .with_owner(1, 1);
    fs.send_req(C1, req);
    assert_eq!(fs.stat(d).mtime, ts(5000));
    // A client whose clock is behind does not move the directory back.
    let req = Request::new(
        0,
        Op::Mknod {
            path: FilePath::child(d, "b"),
            mode: S_IFREG,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_stamp(ts(10))
    .with_owner(1, 1);
    fs.send_req(C1, req);
    let st = fs.stat(d);
    assert_eq!(st.mtime, ts(5000));
    assert_eq!(st.ctime, ts(5000));
    assert_eq!(st.dirstat.nfiles, 2);
    let b = fs.e.lookup_name(d, b"b").unwrap();
    assert_eq!(
        fs.stat(b).btime,
        ts(10),
        "new inodes take the client's stamp"
    );
}

#[test]
fn rename_file_basic_and_over_existing() {
    let mut fs = Fs::new();
    let d1 = fs.mkdir(C1, ROOT, "d1").unwrap();
    let d2 = fs.mkdir(C1, ROOT, "d2").unwrap();
    let a = fs.mkfile(C1, d1, "a").unwrap();
    fs.setattr(
        C1,
        a,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 10,
            ..Default::default()
        },
    )
    .unwrap();
    let b = fs.mkfile(C1, d2, "b").unwrap();
    fs.setattr(
        C1,
        b,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 1000,
            ..Default::default()
        },
    )
    .unwrap();
    let ctime_before = fs.stat(a).ctime;
    let r = fs.rename(C1, d1, "a", d2, "b").unwrap();
    assert_eq!(r.target.as_ref().unwrap().ino, a);
    assert_eq!(
        r.dentry.as_ref().unwrap().dir.ino,
        d2,
        "trace dir is the destination parent"
    );
    assert_eq!(r.dentry.as_ref().unwrap().name, b"b");
    assert_eq!(fs.lookup(C1, d2, "b").unwrap(), a);
    assert_eq!(fs.lookup(C1, d1, "a"), Err(Errno::ENOENT));
    assert!(
        fs.stat(a).ctime > ctime_before,
        "rename updates the ctime of the renamed inode"
    );
    // The replaced file is kept as a stray while its creator holds a cap,
    // and purged when the cap is released.
    assert_eq!(fs.stat(b).nlink, 0);
    assert!(fs.e.strays().contains(&b));
    fs.retire_completed(C1);
    let eff = fs.release(C1, b);
    assert!(fs.e.inode(b).is_none());
    assert!(
        eff.iter()
            .any(|e| matches!(e, Effect::PurgeInode(p) if p.ino == b && p.size == 1000))
    );
    assert_eq!(fs.stat(d1).dirstat.nfiles, 0);
    assert_eq!(fs.stat(d2).dirstat.nfiles, 1);
    assert_eq!(fs.stat(d1).rstat.rbytes, 0);
    assert_eq!(fs.stat(d2).rstat.rbytes, 10);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 10);
    assert_eq!(fs.stat(ROOT).rstat.rfiles, 1);
}

#[test]
fn rename_directory_rules() {
    let mut fs = Fs::new();
    let a = fs.mkdir(C1, ROOT, "a").unwrap();
    let b = fs.mkdir(C1, a, "b").unwrap();
    let c = fs.mkdir(C1, ROOT, "c").unwrap();
    let full = fs.mkdir(C1, ROOT, "full").unwrap();
    fs.mkfile(C1, full, "x").unwrap();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    // Into its own subtree.
    assert_eq!(
        fs.rename(C1, ROOT, "a", b, "a2").unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.rename(C1, ROOT, "a", a, "self").unwrap_err(),
        Errno::EINVAL
    );
    // Over a non-empty directory, over a file, a file over a directory.
    assert_eq!(
        fs.rename(C1, ROOT, "c", ROOT, "full").unwrap_err(),
        Errno::ENOTEMPTY
    );
    assert_eq!(
        fs.rename(C1, ROOT, "c", ROOT, "f").unwrap_err(),
        Errno::ENOTDIR
    );
    assert_eq!(
        fs.rename(C1, ROOT, "f", ROOT, "c").unwrap_err(),
        Errno::EISDIR
    );
    // `.` and `..`.
    assert_eq!(
        fs.rename(C1, ROOT, ".", ROOT, "z").unwrap_err(),
        Errno::EBUSY
    );
    assert_eq!(fs.rename(C1, ROOT, "c", a, "..").unwrap_err(), Errno::EBUSY);
    // Missing source.
    assert_eq!(
        fs.rename(C1, ROOT, "nope", ROOT, "z").unwrap_err(),
        Errno::ENOENT
    );
    // Over an empty directory: the target goes away.
    fs.rename(C1, a, "b", ROOT, "c").unwrap();
    assert_eq!(fs.lookup(C1, ROOT, "c").unwrap(), b);
    assert!(fs.e.inode(c).is_none() || fs.stat(c).nlink == 0);
    assert_eq!(fs.stat(a).dirstat.nsubdirs, 0);
    assert_eq!(fs.stat(ROOT).dirstat.nsubdirs, 3); // a, c (= b), full
    assert_eq!(fs.stat(ROOT).rstat.rsubdirs, 4);
    assert_eq!(fs.e.parent_of(b), Some((ROOT, b"c".to_vec())));
    assert_eq!(f, fs.lookup(C1, ROOT, "f").unwrap());
}

#[test]
fn rename_noops() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    fs.link(C1, ROOT, "g", f).unwrap();
    let before = fs.stat(f);
    let r = fs.rename(C1, ROOT, "f", ROOT, "f").unwrap();
    assert!(r.target.is_none() && r.dentry.is_none());
    // Two names of the same inode: success, nothing changes.
    let r = fs.rename(C1, ROOT, "f", ROOT, "g").unwrap();
    assert!(r.target.is_none());
    assert_eq!(fs.lookup(C1, ROOT, "f").unwrap(), f);
    assert_eq!(fs.lookup(C1, ROOT, "g").unwrap(), f);
    assert_eq!(fs.stat(f).nlink, 2);
    assert_eq!(fs.stat(f).ctime, before.ctime);
}

#[test]
fn rename_hardlink_name_and_overwrite_primary() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    fs.link(C1, d, "remote", f).unwrap();
    // Move the remote name.
    fs.rename(C1, d, "remote", ROOT, "moved").unwrap();
    assert_eq!(fs.lookup(C1, ROOT, "moved").unwrap(), f);
    assert_eq!(
        fs.e.parent_of(f),
        Some((ROOT, b"f".to_vec())),
        "the primary link did not move"
    );
    // Overwrite the primary link with another file: the remote one becomes primary.
    let g = fs.mkfile(C1, ROOT, "g").unwrap();
    fs.rename(C1, ROOT, "g", ROOT, "f").unwrap();
    assert_eq!(fs.lookup(C1, ROOT, "f").unwrap(), g);
    assert_eq!(fs.stat(f).nlink, 1);
    assert_eq!(fs.e.parent_of(f), Some((ROOT, b"moved".to_vec())));
    assert_eq!(fs.stat(ROOT).rstat.rfiles, 2);
}

#[test]
fn setattr_mode_owner_and_kill_bits() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_MODE,
                mode: 0o6755 | S_IFDIR,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.mode, S_IFREG | 0o6755, "file type preserved");
    // chown without MODE clears setuid and setgid of an executable file.
    let before = fs.stat(f);
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_UID,
                uid: 1000,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.mode, S_IFREG | 0o0755);
    assert!(st.ctime > before.ctime);
    assert_eq!(st.change_attr, before.change_attr + 1);
    // Not executable: chown keeps the bits, KILL_SUID clears only setuid.
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_MODE,
            mode: 0o6644,
            ..Default::default()
        },
    )
    .unwrap();
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_GID,
                gid: 5,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.mode & 0o7777, 0o6644);
    assert_eq!(st.gid, 5);
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_KILL_SUID,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.mode & 0o7777, 0o2644);
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_KILL_SGID,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.mode & 0o7777, 0o0644);
    // With MODE in the mask the kill logic is skipped.
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_MODE | CEPH_SETATTR_UID,
                mode: 0o4755,
                uid: 3,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!((st.mode & 0o7777, st.uid), (0o4755, 3));
    // Directories never lose bits through chown.
    let d = fs.mkdir_mode(C1, ROOT, "d", 0o2755).unwrap();
    let st = fs
        .setattr(
            C1,
            d,
            SetattrArgs {
                mask: CEPH_SETATTR_UID,
                uid: 9,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.mode & 0o7777, 0o2755);
}

#[test]
fn setattr_times_and_ctime_only() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let tws = fs.stat(f).time_warp_seq;
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_MTIME | CEPH_SETATTR_ATIME | CEPH_SETATTR_BTIME,
                mtime: ts(10),
                atime: ts(20),
                btime: ts(5),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!((st.mtime, st.atime, st.btime), (ts(10), ts(20), ts(5)));
    assert_eq!(st.time_warp_seq, tws + 1);
    assert!(st.ctime > ts(20), "ctime comes from the request stamp");
    // A ctime-only setattr ("touch ctime") and an empty mask.
    let before = fs.stat(f);
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_CTIME,
            ..Default::default()
        },
    )
    .unwrap();
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: 0,
            ..Default::default()
        },
    )
    .unwrap();
    let after = fs.stat(f);
    assert_eq!(after.change_attr, before.change_attr + 2);
    assert_eq!(after.time_warp_seq, before.time_warp_seq);
    assert!(after.rstat.rctime >= after.ctime);
    assert_eq!(
        fs.stat(ROOT).rstat.rctime,
        after.ctime,
        "rctime propagates up"
    );
}

#[test]
fn truncate_semantics() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_SIZE,
                size: 5000,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.size, 5000);
    assert_eq!(st.truncate_seq, 1, "growth is not a truncation");
    assert_eq!(st.truncate_size, u64::MAX);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 5000);
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_SIZE | CEPH_SETATTR_MTIME,
                size: 100,
                mtime: ts(1),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.size, 100);
    assert_eq!(st.truncate_seq, 2);
    assert_eq!(st.truncate_size, 100);
    assert_ne!(
        st.mtime,
        ts(1),
        "SIZE sets mtime to the stamp over an explicit MTIME"
    );
    assert_eq!(st.mtime, st.ctime);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 100);
    assert_eq!(fs.stat(f).max_size_ever, 5000);
    // The data truncation is handed to the daemon; cap holders get TRUNC.
    let td: Vec<_> = fs
        .last
        .iter()
        .filter_map(|e| {
            if let Effect::TruncateData(t) = e {
                Some(t.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(td.len(), 1);
    assert_eq!((td[0].size, td[0].from, td[0].truncate_seq), (100, 5000, 2));
    let trunc: Vec<_> = cap_msgs(&fs.last)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_TRUNC)
        .collect();
    assert_eq!(trunc.len(), 1);
    assert_eq!(trunc[0].client, C1);
    assert_eq!(trunc[0].inode.as_ref().unwrap().truncate_seq, 2);
    // old_size from the client (buffered data) counts.
    let st = fs
        .setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_SIZE,
                size: 100,
                old_size: 300,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(st.truncate_seq, 3);
}

#[test]
fn open_flags() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let s = fs.symlink(C1, ROOT, "s", "f").unwrap();
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 42,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(fs.open(C1, f, CEPH_O_DIRECTORY).unwrap_err(), Errno::EINVAL);
    assert_eq!(
        fs.open(C1, d, CEPH_O_WRONLY | CEPH_O_TRUNC).unwrap_err(),
        Errno::EISDIR
    );
    assert!(fs.open(C1, d, CEPH_O_DIRECTORY).is_ok());
    let fifo = fs.mknod(C1, ROOT, "fifo", S_IFIFO | 0o644, 0).unwrap();
    assert_eq!(
        fs.open(C1, fifo, CEPH_O_WRONLY | CEPH_O_TRUNC).unwrap_err(),
        Errno::EINVAL
    );
    // A symlink being followed ignores O_TRUNC.
    assert!(fs.open(C1, s, CEPH_O_RDONLY | CEPH_O_TRUNC).is_ok());
    assert_eq!(
        fs.open(C1, s, CEPH_O_RDONLY | CEPH_O_TRUNC | CEPH_O_NOFOLLOW)
            .unwrap_err(),
        Errno::EINVAL
    );
    // O_TRUNC on a file.
    let before = fs.stat(f);
    let st = fs.open(C1, f, CEPH_O_WRONLY | CEPH_O_TRUNC).unwrap();
    assert_eq!(st.size, 0);
    assert_eq!(st.truncate_seq, before.truncate_seq + 1);
    assert!(st.mtime > before.mtime && st.ctime == st.mtime);
    assert_eq!(st.change_attr, before.change_attr + 1);
    assert_eq!(
        fs.stat(ROOT).rstat.rbytes,
        1,
        "only the symlink's target length is left"
    );
    // O_TRUNC on an empty file still sets the times, without a truncation.
    let st2 = fs.open(C1, f, CEPH_O_WRONLY | CEPH_O_TRUNC).unwrap();
    assert_eq!(st2.truncate_seq, st.truncate_seq);
    assert!(st2.mtime > st.mtime);
    // Writers get a writeable range on open.
    assert_eq!(st2.max_size, 4 << 20);
}

#[test]
fn inode_numbers_never_reused() {
    let mut fs = Fs::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut last = 0;
    for i in 0..50 {
        let n = format!("f{i}");
        let ino = fs.mkfile(C1, ROOT, &n).unwrap();
        assert!(ino > last, "allocation is monotonic");
        assert!(ino < (1u64 << 48));
        assert!(!(0x100..0x1000).contains(&ino));
        last = ino;
        assert!(seen.insert(ino));
        fs.release(C1, ino);
        fs.unlink(C1, ROOT, &n).unwrap();
        fs.retire_completed(C1);
        assert!(fs.e.inode(ino).is_none());
    }
}

#[test]
fn completed_requests_answer_retries() {
    let mut fs = Fs::new();
    // A create, retried after the reply was lost: opened, not created again.
    let req = Request::new(
        77,
        Op::Create {
            path: FilePath::child(ROOT, "f"),
            flags: CEPH_O_CREAT | CEPH_O_EXCL | CEPH_O_RDWR,
            mode: 0o644,
            layout: LayoutOverride::default(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_owner(1, 1)
    .with_flags(CEPH_MDS_FLAG_WANT_DENTRY);
    let (_, eff) = fs.send_req(C1, req.clone());
    let ino = reply_for(&eff, C1, 77)
        .unwrap()
        .target
        .as_ref()
        .unwrap()
        .ino;
    let mut retry = req.clone();
    retry.retry_attempt = 1;
    let (_, eff) = fs.send_req(C1, retry);
    let r = reply_for(&eff, C1, 77).unwrap();
    assert_eq!(
        r.result, 0,
        "not EEXIST: the O_EXCL create is not executed twice"
    );
    assert_eq!(r.target.as_ref().unwrap().ino, ino);
    assert_eq!(
        r.extra,
        ReplyExtra::Create {
            ino,
            delegated: vec![]
        }
    );
    // An unlink retried: answered 0 from the table instead of ENOENT.
    let req = Request::new(
        78,
        Op::Unlink {
            path: FilePath::child(ROOT, "f"),
        },
    )
    .with_owner(1, 1);
    assert_eq!(fs.call_req(C1, req.clone()).result, 0);
    let mut retry = req.clone();
    retry.retry_attempt = 1;
    let (_, eff) = fs.send_req(C1, retry);
    let r = reply_for(&eff, C1, 78).unwrap();
    assert_eq!(r.result, 0);
    assert!(r.target.is_none() && r.dentry.is_none(), "traceless");
    // A replayed mkdir: traceless with the created ino.
    let req = Request::new(
        79,
        Op::Mkdir {
            path: FilePath::child(ROOT, "d"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_owner(1, 1);
    let (_, eff) = fs.send_req(C1, req.clone());
    let d = reply_for(&eff, C1, 79)
        .unwrap()
        .target
        .as_ref()
        .unwrap()
        .ino;
    let replay = req.clone().with_flags(CEPH_MDS_FLAG_REPLAY);
    let (_, eff) = fs.send_req(C1, replay);
    let r = reply_for(&eff, C1, 79).unwrap();
    assert_eq!((r.result, r.created_ino), (0, Some(d)));
    // A failed request is not recorded: its retry executes again.
    let req = Request::new(
        80,
        Op::Unlink {
            path: FilePath::child(ROOT, "nope"),
        },
    )
    .with_owner(1, 1);
    fs.send_req(C1, req.clone());
    let mut retry = req;
    retry.retry_attempt = 1;
    let (_, eff) = fs.send_req(C1, retry);
    assert_eq!(
        reply_for(&eff, C1, 80).unwrap().result,
        Errno::ENOENT.as_neg()
    );
    // A read-only oldest_client_tid is observed, not durable retirement.
    let mut req = Request::new(
        81,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    req.oldest_client_tid = 80;
    fs.send_req(C1, req);
    let s = fs.e.session(C1).unwrap();
    assert!(s.completed_requests.contains_key(&77) && s.completed_requests.contains_key(&79));
    fs.retire_completed(C1);
    let s = fs.e.session(C1).unwrap();
    assert!(!s.completed_requests.contains_key(&77) && !s.completed_requests.contains_key(&79));
}

#[test]
fn read_only_and_snapshot_ops() {
    let mut cfg = config();
    cfg.read_only = true;
    let mut fs = Fs::with(cfg, MemJournal::new());
    assert_eq!(fs.mkdir(C1, ROOT, "d"), Err(Errno::EROFS));
    assert!(fs.getattr(C1, ROOT, 0).is_ok());
    assert_eq!(
        fs.try_call(
            C1,
            Op::Open {
                path: FilePath::ino(ROOT),
                flags: CEPH_O_TRUNC | CEPH_O_WRONLY,
                mode: 0,
                mask: 0,
                old_size: 0
            }
        )
        .unwrap_err(),
        Errno::EROFS
    );
    let mut fs = Fs::new();
    assert_eq!(
        fs.try_call(
            C1,
            Op::LookupSnap {
                path: FilePath::child(ROOT, "s1")
            }
        )
        .unwrap_err(),
        Errno::ENOENT
    );
    assert_eq!(
        fs.try_call(
            C1,
            Op::Mksnap {
                path: FilePath::child(ROOT, "s1")
            }
        )
        .unwrap_err(),
        Errno::EPERM
    );
    let r = fs.call(
        C1,
        Op::Lssnap {
            path: FilePath::ino(ROOT),
        },
    );
    match r.extra {
        ReplyExtra::Readdir(rd) => {
            assert!(rd.entries.is_empty());
            assert_ne!(
                rd.flags & CEPH_READDIR_FRAG_END,
                0,
                "an empty LSSNAP sets FRAG_END"
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        fs.try_call(
            C1,
            Op::Other {
                op: CEPH_MDS_OP_RMSNAP
            }
        )
        .unwrap_err(),
        Errno::ENOENT
    );
    assert_eq!(
        fs.try_call(C1, Op::Other { op: 0x1500 }).unwrap_err(),
        Errno::EOPNOTSUPP
    );
}

#[test]
fn requests_without_session_are_dropped() {
    let mut fs = Fs::new();
    let eff = fs.e.handle_request(
        999,
        Request::new(
            1,
            Op::Getattr {
                path: FilePath::ino(ROOT),
                mask: 0,
            },
        ),
    );
    assert!(eff.is_empty());
}
