//! Plain xattrs, POSIX ACLs stored opaquely, and the `ceph.*` virtual
//! xattrs of design 4.2 (layouts, quota, subvolume flag, pins).

mod common;

use common::*;
use engine::consts::*;
use engine::*;

#[test]
fn set_get_remove_with_flags() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let before = fs.stat(f);
    fs.setxattr(C1, f, "user.a", b"1", 0).unwrap();
    let c = fs.stat(f);
    assert_eq!(c.xattrs.get(&b"user.a"[..]), Some(&b"1".to_vec()));
    assert_eq!(c.xattr_version, before.xattr_version + 1);
    assert_eq!(c.change_attr, before.change_attr + 1);
    assert!(c.ctime > before.ctime);
    assert_eq!(
        fs.setxattr(C1, f, "user.a", b"2", CEPH_XATTR_CREATE)
            .unwrap_err(),
        Errno::EEXIST
    );
    assert_eq!(
        fs.setxattr(C1, f, "user.b", b"2", CEPH_XATTR_REPLACE)
            .unwrap_err(),
        Errno::ENODATA
    );
    fs.setxattr(C1, f, "user.a", b"2", CEPH_XATTR_REPLACE)
        .unwrap();
    assert_eq!(fs.stat(f).xattrs.get(&b"user.a"[..]), Some(&b"2".to_vec()));
    // Zero-length values are values (mgr/volumes earmarks).
    fs.setxattr(C1, f, "user.ceph.subvolume.earmark", b"", 0)
        .unwrap();
    assert_eq!(
        fs.stat(f).xattrs.get(&b"user.ceph.subvolume.earmark"[..]),
        Some(&vec![])
    );
    // Removal through SETXATTR flags.
    assert_eq!(
        fs.setxattr(C1, f, "user.none", b"", CEPH_XATTR_REMOVE2)
            .unwrap_err(),
        Errno::ENODATA
    );
    fs.setxattr(C1, f, "user.none", b"", CEPH_XATTR_REMOVE)
        .unwrap();
    fs.setxattr(C1, f, "user.a", b"", CEPH_XATTR_REMOVE)
        .unwrap();
    assert!(!fs.stat(f).xattrs.contains_key(&b"user.a"[..]));
    // RMXATTR.
    assert_eq!(fs.rmxattr(C1, f, "user.a").unwrap_err(), Errno::ENODATA);
    fs.rmxattr(C1, f, "user.ceph.subvolume.earmark").unwrap();
    assert!(fs.stat(f).xattrs.is_empty());
    // Listing: the blob comes with a getattr asking for Xs.
    fs.setxattr(C1, f, "trusted.t", b"x", 0).unwrap();
    fs.setxattr(C1, f, "security.s", b"y", 0).unwrap();
    let st = fs.getattr(C2, f, CEPH_STAT_CAP_INODE_ALL).unwrap();
    let names: Vec<_> = st.xattrs.unwrap().keys().cloned().collect();
    assert_eq!(names, vec![b"security.s".to_vec(), b"trusted.t".to_vec()]);
    assert_eq!(st.xattr_version, fs.stat(f).xattr_version);
}

#[test]
fn posix_acls_are_opaque() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let acl = vec![
        2u8, 0, 0, 0, 1, 0, 7, 0, 0xff, 0xff, 0xff, 0xff, 4, 0, 5, 0, 0xff, 0xff, 0xff, 0xff,
    ];
    fs.setxattr(C1, d, "system.posix_acl_default", &acl, 0)
        .unwrap();
    fs.setxattr(C1, d, "system.posix_acl_access", &acl, 0)
        .unwrap();
    assert_eq!(
        fs.stat(d).xattrs.get(&b"system.posix_acl_default"[..]),
        Some(&acl)
    );
    // The kernel removes an ACL with SETXATTR + CEPH_XATTR_REMOVE.
    fs.setxattr(C1, d, "system.posix_acl_access", b"", CEPH_XATTR_REMOVE)
        .unwrap();
    assert!(
        !fs.stat(d)
            .xattrs
            .contains_key(&b"system.posix_acl_access"[..])
    );
    // The MDS does not apply a default ACL to children: clients send it.
    let f = fs.mkfile(C1, d, "f").unwrap();
    assert!(fs.stat(f).xattrs.is_empty());
}

#[test]
fn size_limit_and_reserved_names() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let big = vec![b'x'; 40_000];
    fs.setxattr(C1, f, "user.a", &big, 0).unwrap();
    assert_eq!(
        fs.setxattr(C1, f, "user.b", &big, 0).unwrap_err(),
        Errno::ENOSPC
    );
    // Replacing excludes the old value from the accounting.
    fs.setxattr(C1, f, "user.a", &big, CEPH_XATTR_REPLACE)
        .unwrap();
    // Plain replace without the flag counts both.
    assert_eq!(
        fs.setxattr(C1, f, "user.a", &big, 0).unwrap_err(),
        Errno::ENOSPC
    );
    // ceph.* names that are not virtual xattrs.
    assert_eq!(
        fs.setxattr(C1, f, "ceph.dir.rbytes", b"1", 0).unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, f, "ceph.whatever", b"1", 0).unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.rmxattr(C1, f, "ceph.dir.rctime").unwrap_err(),
        Errno::EINVAL
    );
    fs.setxattr(C1, f, "ceph.mirror.dirty_snap_id", b"5", 0)
        .unwrap();
}

#[test]
fn getvxattr_names_and_enodata() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let f = fs.mkfile(C1, d, "f").unwrap();
    // Not a vxattr at all, unknown ceph.* names, charmap names.
    assert_eq!(fs.getvxattr(C1, d, "user.x").unwrap_err(), Errno::ENODATA);
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.dir.charmap").unwrap_err(),
        Errno::ENODATA
    );
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.dir.casesensitive").unwrap_err(),
        Errno::ENODATA
    );
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.nope").unwrap_err(),
        Errno::ENODATA
    );
    // Dir names on files and file names on dirs.
    assert_eq!(
        fs.getvxattr(C1, f, "ceph.dir.layout.json").unwrap_err(),
        Errno::ENODATA
    );
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.file.layout.json").unwrap_err(),
        Errno::ENODATA
    );
    // Inherited layout of a directory without its own.
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.dir.layout.json").unwrap(),
        r#"{"stripe_unit": 4194304, "stripe_count": 1, "object_size": 4194304, "pool_name": "cephfs.a.data", "pool_id": 2, "pool_namespace": "", "inheritance": "@default"}"#
    );
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.dir.layout.pool_name").unwrap(),
        "cephfs.a.data"
    );
    assert_eq!(fs.getvxattr(C1, d, "ceph.dir.layout.pool_id").unwrap(), "2");
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.dir.layout").unwrap_err(),
        Errno::ENODATA,
        "no own layout"
    );
    // The root has its own explicit layout (mgr/volumes walks up to /).
    assert_eq!(
        fs.getvxattr(C1, ROOT, "ceph.dir.layout").unwrap(),
        "stripe_unit=4194304 stripe_count=1 object_size=4194304 pool=cephfs.a.data"
    );
    assert_eq!(
        fs.getvxattr(C1, ROOT, "ceph.dir.layout.pool").unwrap(),
        "cephfs.a.data"
    );
    assert_eq!(
        fs.getvxattr(C1, f, "ceph.file.layout.object_size").unwrap(),
        "4194304"
    );
    assert_eq!(fs.getvxattr(C1, d, "ceph.dir.subvolume").unwrap(), "0");
    assert_eq!(fs.getvxattr(C1, d, "ceph.dir.pin").unwrap(), "-1");
    assert_eq!(fs.getvxattr(C1, d, "ceph.dir.pin.random").unwrap(), "0");
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.dir.pin.distributed").unwrap(),
        "0"
    );
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.quota").unwrap_err(),
        Errno::ENODATA
    );
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.quota.max_bytes").unwrap_err(),
        Errno::ENODATA
    );
    // GETVXATTR replies carry no trace.
    let r = fs.call(
        C1,
        Op::Getvxattr {
            path: FilePath::ino(d),
            name: b"ceph.dir.pin".to_vec(),
        },
    );
    assert!(r.target.is_none() && r.dentry.is_none());
}

#[test]
fn dir_layouts_set_inherit_remove() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let sub = fs.mkdir(C1, d, "sub").unwrap();
    let before = fs.stat(d);
    fs.setxattr(C1, d, "ceph.dir.layout.pool", b"fast", 0)
        .unwrap();
    assert_eq!(fs.stat(d).layout.pool_id, 3);
    assert_eq!(fs.stat(d).change_attr, before.change_attr + 1);
    fs.setxattr(C1, d, "ceph.dir.layout.stripe_unit", b"1048576", 0)
        .unwrap();
    fs.setxattr(C1, d, "ceph.dir.layout.pool_namespace", b"ns1", 0)
        .unwrap();
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.dir.layout").unwrap(),
        "stripe_unit=1048576 stripe_count=1 object_size=4194304 pool=fast pool_namespace=ns1"
    );
    // Children inherit at creation time; the inheritance status shows it.
    let f = fs.mkfile(C1, sub, "f").unwrap();
    let l = fs.stat(f).layout;
    assert_eq!(
        (l.pool_id, l.stripe_unit, l.pool_ns.as_str()),
        (3, 1 << 20, "ns1")
    );
    assert!(
        fs.getvxattr(C1, sub, "ceph.dir.layout.json")
            .unwrap()
            .ends_with(r#""inheritance": "@inherited"}"#)
    );
    assert!(
        fs.getvxattr(C1, d, "ceph.dir.layout.json")
            .unwrap()
            .ends_with(r#""inheritance": "@set"}"#)
    );
    // Composite and JSON forms.
    fs.setxattr(
        C1,
        sub,
        "ceph.dir.layout",
        b"stripe_unit=65536 stripe_count=4 object_size=262144 pool=cephfs.a.data",
        0,
    )
    .unwrap();
    let l = fs.stat(sub).layout;
    assert_eq!(
        (l.stripe_unit, l.stripe_count, l.object_size, l.pool_id),
        (65536, 4, 262144, 2)
    );
    fs.setxattr(
        C1,
        sub,
        "ceph.dir.layout.json",
        br#"{"object_size": 4194304, "stripe_unit": 4194304, "stripe_count": 1, "pool_id": 3}"#,
        0,
    )
    .unwrap();
    assert_eq!(fs.stat(sub).layout.pool_id, 3);
    // Errors.
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.layout.pool", b"nosuch", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.layout.pool", b"1", 0)
            .unwrap_err(),
        Errno::EINVAL,
        "not a data pool"
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.layout.stripe_unit", b"1000", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.layout.stripe_unit", b"abc", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.layout.bogus", b"1", 0)
            .unwrap_err(),
        Errno::ENODATA
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.layout", b"stripe_unit=", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, f, "ceph.dir.layout.pool", b"fast", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    // Remove: back to inheriting; the root resets to the default.
    fs.rmxattr(C1, d, "ceph.dir.layout").unwrap();
    assert!(!fs.stat(d).layout.is_set());
    fs.rmxattr(C1, d, "ceph.dir.layout").unwrap();
    fs.setxattr(C1, ROOT, "ceph.dir.layout.pool", b"fast", 0)
        .unwrap();
    fs.rmxattr(C1, ROOT, "ceph.dir.layout").unwrap();
    assert_eq!(fs.stat(ROOT).layout, FileLayout::default_for_pool(2));
}

#[test]
fn file_layouts_only_on_empty_files() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    fs.setxattr(C1, f, "ceph.file.layout.pool", b"fast", 0)
        .unwrap();
    let c = fs.stat(f);
    assert_eq!(c.layout.pool_id, 3);
    assert!(c.old_pools.contains(&2));
    fs.setxattr(C1, f, "ceph.file.layout", b"stripe_count=2", 0)
        .unwrap();
    assert_eq!(fs.stat(f).layout.stripe_count, 2);
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        fs.setxattr(C1, f, "ceph.file.layout.pool", b"cephfs.a.data", 0)
            .unwrap_err(),
        Errno::ENOTEMPTY
    );
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    assert_eq!(
        fs.setxattr(C1, d, "ceph.file.layout.pool", b"fast", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
}

#[test]
fn quotas_open_realms_and_split() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "q").unwrap();
    let f = fs.mkfile(C1, d, "f").unwrap();
    fs.getattr(C2, f, 0).unwrap();
    fs.setxattr(C1, d, "ceph.quota.max_bytes", b"10G", 0)
        .unwrap();
    let c = fs.stat(d);
    assert_eq!(c.quota.max_bytes, 10 << 30);
    assert!(c.snaprealm.is_some(), "a quota directory roots a realm");
    // Clients with caps below it are told before the reply.
    let splits: Vec<_> = fs
        .last
        .iter()
        .filter_map(|e| {
            if let Effect::RealmSplit(s) = e {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect();
    let for_c2 = splits
        .iter()
        .find(|s| s.client == C2)
        .expect("split for C2");
    assert_eq!(for_c2.realm, d);
    assert!(for_c2.split_inos.contains(&f));
    let reply_pos = fs
        .last
        .iter()
        .position(|e| matches!(e, Effect::Reply(_)))
        .unwrap();
    let split_pos = fs
        .last
        .iter()
        .position(|e| matches!(e, Effect::RealmSplit(_)))
        .unwrap();
    assert!(split_pos < reply_pos);
    // The reply and later caps name the new realm.
    let st = fs.getattr(C1, f, 0).unwrap();
    assert_eq!(st.cap.realm, d);
    assert_eq!(
        fs.e.snap_trace(f).iter().map(|r| r.ino).collect::<Vec<_>>(),
        vec![d, ROOT]
    );
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.quota").unwrap(),
        format!("max_bytes={} max_files=0", 10u64 << 30)
    );
    fs.setxattr(C1, d, "ceph.quota", b"max_files=100 max_bytes=1M", 0)
        .unwrap();
    assert_eq!(
        fs.stat(d).quota,
        Quota {
            max_bytes: 1 << 20,
            max_files: 100
        }
    );
    assert_eq!(fs.getvxattr(C1, d, "ceph.quota.max_files").unwrap(), "100");
    // Errors and removal.
    assert_eq!(
        fs.setxattr(C1, d, "ceph.quota.max_files", b"-1", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.quota.max_bytes", b"lots", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.quota.other", b"1", 0).unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, f, "ceph.quota.max_files", b"1", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    fs.rmxattr(C1, d, "ceph.quota.max_files").unwrap();
    fs.rmxattr(C1, d, "ceph.quota.max_bytes").unwrap();
    assert!(!fs.stat(d).quota.is_enabled());
    fs.rmxattr(C1, d, "ceph.quota").unwrap();
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.quota").unwrap_err(),
        Errno::ENODATA
    );
}

#[test]
fn subvolume_flag_and_exdev() {
    let mut fs = Fs::new();
    let vols = fs.mkdir(C1, ROOT, "volumes").unwrap();
    let a = fs.mkdir(C1, vols, "a").unwrap();
    let b = fs.mkdir(C1, vols, "b").unwrap();
    let a_inner = fs.mkdir(C1, a, "inner").unwrap();
    let fa = fs.mkfile(C1, a_inner, "fa").unwrap();
    fs.setxattr(C1, a, "ceph.dir.subvolume", b"1", 0).unwrap();
    fs.setxattr(C1, b, "ceph.dir.subvolume", b"true", 0)
        .unwrap();
    // Setting it again is a cheap no-op (no record).
    let n = fs.e.journal().last_seq();
    fs.setxattr(C1, a, "ceph.dir.subvolume", b"1", 0).unwrap();
    assert_eq!(fs.e.journal().last_seq(), n);
    assert_eq!(fs.getvxattr(C1, a, "ceph.dir.subvolume").unwrap(), "1");
    let st = fs.getattr(C1, fa, 0).unwrap();
    assert_eq!(st.subvolume_id, a);
    // Nested subvolumes and non-directories are refused.
    assert_eq!(
        fs.setxattr(C1, a_inner, "ceph.dir.subvolume", b"1", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, fa, "ceph.dir.subvolume", b"1", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, b, "ceph.dir.subvolume", b"maybe", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    // Rename and link across subvolumes: EXDEV.
    assert_eq!(
        fs.rename(C1, a_inner, "fa", b, "fa").unwrap_err(),
        Errno::EXDEV
    );
    assert_eq!(fs.link(C1, b, "l", fa).unwrap_err(), Errno::EXDEV);
    assert_eq!(
        fs.rename(C1, a_inner, "fa", vols, "fa").unwrap_err(),
        Errno::EXDEV
    );
    // Inside one subvolume, fine.
    fs.rename(C1, a_inner, "fa", a, "fa2").unwrap();
    fs.link(C1, a_inner, "l", fa).unwrap();
    // Moving a subvolume root between non-subvolume parents is allowed
    // (mgr/volumes trash).
    let trash = fs.mkdir(C1, vols, "_deleting").unwrap();
    fs.rename(C1, vols, "a", trash, "a").unwrap();
    assert_eq!(fs.getattr(C1, fa, 0).unwrap().subvolume_id, a);
    // Clearing it.
    fs.rmxattr(C1, b, "ceph.dir.subvolume").unwrap();
    assert_eq!(fs.getvxattr(C1, b, "ceph.dir.subvolume").unwrap(), "0");
    fs.rmxattr(C1, b, "ceph.dir.subvolume").unwrap();
}

#[test]
fn pins_are_accepted() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    fs.setxattr(C1, d, "ceph.dir.pin", b"3", 0).unwrap();
    assert_eq!(fs.getvxattr(C1, d, "ceph.dir.pin").unwrap(), "3");
    assert_eq!(fs.getattr(C1, d, 0).unwrap().export_pin, 3);
    fs.setxattr(C1, d, "ceph.dir.pin", b"-5", 0).unwrap();
    assert_eq!(fs.stat(d).export_pin, -1);
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.pin", b"256", 0).unwrap_err(),
        Errno::EDOM
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.pin", b"x", 0).unwrap_err(),
        Errno::EINVAL
    );
    assert_eq!(
        fs.setxattr(C1, ROOT, "ceph.dir.pin", b"1", 0).unwrap_err(),
        Errno::EINVAL
    );
    fs.setxattr(C1, d, "ceph.dir.pin.distributed", b"1", 0)
        .unwrap();
    assert_eq!(
        fs.getvxattr(C1, d, "ceph.dir.pin.distributed").unwrap(),
        "1"
    );
    fs.setxattr(C1, d, "ceph.dir.pin.random", b"0.005", 0)
        .unwrap();
    assert_eq!(fs.getvxattr(C1, d, "ceph.dir.pin.random").unwrap(), "0.005");
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.pin.random", b"2", 0)
            .unwrap_err(),
        Errno::EDOM
    );
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.pin.random", b"0.5", 0)
            .unwrap_err(),
        Errno::EINVAL
    );
    fs.rmxattr(C1, d, "ceph.dir.pin").unwrap();
    fs.rmxattr(C1, d, "ceph.dir.pin.random").unwrap();
    fs.rmxattr(C1, d, "ceph.dir.pin.distributed").unwrap();
    let c = fs.stat(d);
    assert_eq!(
        (c.export_pin, c.export_pin_random, c.export_pin_distributed),
        (-1, 0.0, false)
    );
    // Accepted and ignored.
    fs.setxattr(C1, d, "ceph.quiesce.block", b"1", 0).unwrap();
    assert_eq!(
        fs.setxattr(C1, d, "ceph.dir.charmap", b"x", 0).unwrap_err(),
        Errno::EOPNOTSUPP
    );
}

#[test]
fn xattr_changes_revoke_shared_caps_of_others() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    assert_ne!(fs.issued(f, C1) & XX, 0);
    let (tid, eff) = fs.send(
        C2,
        Op::Setxattr {
            path: FilePath::ino(f),
            name: b"user.k".to_vec(),
            value: b"v".to_vec(),
            flags: 0,
        },
    );
    assert!(reply_for(&eff, C2, tid).is_none());
    let rv = revokes(&eff);
    assert_eq!(rv.len(), 1);
    assert_eq!(rv[0].caps & (XS | XX), 0);
    // The holder flushes its own buffered xattrs with the ack.
    let mut mine = XattrMap::new();
    mine.insert(b"user.mine".to_vec(), b"1".to_vec());
    let v = fs.stat(f).xattr_version;
    let m = CapUpdate {
        op: CEPH_CAP_OP_UPDATE,
        ino: f,
        cap_id: rv[0].cap_id,
        seq: rv[0].seq,
        issue_seq: rv[0].issue_seq,
        caps: rv[0].caps,
        wanted: rv[0].wanted,
        dirty: XX,
        flush_tid: 1,
        oldest_flush_tid: 1,
        xattrs: Some((v + 1, mine)),
        ..Default::default()
    };
    let eff = fs.cap_update(C1, m);
    assert_eq!(reply_for(&eff, C2, tid).unwrap().result, 0);
    let x = fs.stat(f).xattrs;
    assert!(x.contains_key(&b"user.mine"[..]) && x.contains_key(&b"user.k"[..]));
}
