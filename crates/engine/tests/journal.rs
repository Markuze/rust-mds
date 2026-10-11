//! Safe replies (only after commit), waiting on uncommitted state, one
//! self-contained record per mutation, and replay.

mod common;

use common::*;
use engine::consts::*;
use engine::journal::RecordKind;
use engine::*;

#[test]
fn replies_only_after_commit() {
    let mut fs = Fs::manual();
    let (tid, eff) = fs.send(
        C1,
        Op::Mkdir {
            path: FilePath::child(ROOT, "d"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    assert!(reply_for(&eff, C1, tid).is_none(), "no unsafe replies");
    assert_eq!(fs.e.uncommitted_records(), 1);
    let eff = fs.commit();
    let r = reply_for(&eff, C1, tid).unwrap();
    assert_eq!(r.result, 0);
    assert_eq!(fs.e.uncommitted_records(), 0);
}

#[test]
fn other_requests_wait_for_uncommitted_changes() {
    let mut fs = Fs::manual();
    let (t0, _) = fs.send(
        C1,
        Op::Mkdir {
            path: FilePath::child(ROOT, "d"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    fs.commit();
    let d = fs.e.lookup_name(ROOT, b"d").unwrap();
    let _ = t0;
    let (t1, _) = fs.send(
        C1,
        Op::Mknod {
            path: FilePath::child(d, "x"),
            mode: S_IFREG | 0o644,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let flushes = fs.e.journal().flush_count();
    // Another client's lookup of the new name must not see uncommitted state.
    let (t2, eff) = fs.send(
        C2,
        Op::Lookup {
            path: FilePath::child(d, "x"),
            mask: 0,
        },
    );
    assert!(reply_for(&eff, C2, t2).is_none());
    assert!(
        fs.e.journal().flush_count() > flushes,
        "a waiter asks for an early commit"
    );
    // A request on unrelated inodes is not held up.
    let (t3, eff) = fs.send(
        C2,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    assert!(reply_for(&eff, C2, t3).is_some());
    let eff = fs.commit();
    let p1 = eff
        .iter()
        .position(|e| matches!(e, Effect::Reply(r) if r.tid == t1))
        .unwrap();
    let p2 = eff
        .iter()
        .position(|e| matches!(e, Effect::Reply(r) if r.tid == t2))
        .unwrap();
    assert!(p1 < p2, "the mutation's reply comes before the waiter's");
    assert_eq!(reply_for(&eff, C2, t2).unwrap().result, 0);
    // A resend of a request still in flight is ignored (its reply will come).
    let (t4, _) = fs.send(
        C1,
        Op::Mknod {
            path: FilePath::child(d, "y"),
            mode: S_IFREG,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let mut again = Request::new(
        t4,
        Op::Mknod {
            path: FilePath::child(d, "y"),
            mode: S_IFREG,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    again.retry_attempt = 1;
    let eff = fs.e.handle_request(C1, again);
    assert!(eff.is_empty());
    let eff = fs.commit();
    assert_eq!(replies(&eff).iter().filter(|r| r.tid == t4).count(), 1);
}

#[test]
fn one_self_contained_record_per_mutation() {
    let mut fs = Fs::new();
    let n0 = fs.e.journal().last_seq();
    let r = fs.create_reply(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let f = r.target.unwrap().ino;
    assert_eq!(fs.e.journal().last_seq(), n0 + 1);
    let rec = fs.e.journal().records().last().unwrap().1.clone();
    assert_eq!(rec.kind, RecordKind::Create);
    let cr = rec.completed_request.clone().unwrap();
    assert_eq!((cr.client, cr.created_ino), (C1, Some(f)));
    assert_eq!(rec.allocated, vec![f]);
    assert!(rec.ino_alloc.is_some());
    assert!(
        rec.inodes
            .iter()
            .any(|i| i.ino == f && i.client_ranges.get(&C1) == Some(&(4 << 20)))
    );
    assert!(
        rec.inodes
            .iter()
            .any(|i| i.ino == ROOT && i.dirstat.nfiles == 1)
    );
    assert_eq!(rec.dentries.len(), 1);
    assert_eq!(rec.dentries[0].name, b"f");
    // Reads write nothing.
    fs.getattr(C2, f, CEPH_STAT_CAP_INODE_ALL).unwrap();
    fs.names(C2, ROOT);
    fs.lookup(C2, ROOT, "f").unwrap();
    assert_eq!(fs.e.journal().last_seq(), n0 + 1);
    // Versions grow record over record.
    let v1 = fs.stat(f).version;
    fs.setxattr(C1, f, "user.a", b"b", 0).unwrap();
    let rec = fs.e.journal().records().last().unwrap().1.clone();
    let img = rec.inodes.iter().find(|i| i.ino == f).unwrap();
    assert!(img.version > v1);
}

fn workload(fs: &mut Fs) {
    let a = fs.mkdir(C1, ROOT, "a").unwrap();
    let b = fs.mkdir(C2, ROOT, "b").unwrap();
    let f1 = fs.create(C1, a, "f1", CEPH_O_RDWR).unwrap();
    fs.setattr(
        C1,
        f1,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 12345,
            ..Default::default()
        },
    )
    .unwrap();
    let f2 = fs.mkfile(C2, b, "f2").unwrap();
    fs.link(C2, a, "f2link", f2).unwrap();
    fs.symlink(C1, b, "s", "../a/f1").unwrap();
    fs.mknod(C1, b, "fifo", S_IFIFO | 0o600, 0).unwrap();
    fs.setxattr(C2, f2, "user.x", b"1", 0).unwrap();
    fs.setxattr(C2, f2, "user.empty", b"", 0).unwrap();
    fs.rmxattr(C2, f2, "user.x").unwrap();
    fs.setxattr(C1, a, "ceph.dir.layout.pool", b"fast", 0)
        .unwrap();
    fs.setxattr(C1, b, "ceph.quota.max_files", b"100", 0)
        .unwrap();
    let vols = fs.mkdir(C1, ROOT, "volumes").unwrap();
    let sv = fs.mkdir(C1, vols, "sv").unwrap();
    fs.setxattr(C1, sv, "ceph.dir.subvolume", b"1", 0).unwrap();
    let g = fs.mkfile(C1, a, "g").unwrap();
    fs.rename(C1, a, "g", b, "g2").unwrap();
    fs.rename(C2, b, "f2", a, "f1").unwrap(); // over an existing file, kept as a stray (C1 has it open)
    fs.unlink(C1, a, "f2link").unwrap();
    fs.mkdir(C1, b, "sub").unwrap();
    fs.rename(C1, b, "sub", a, "sub").unwrap();
    fs.rmdir(C1, a, "sub").unwrap();
    fs.setattr(
        C1,
        g,
        SetattrArgs {
            mask: CEPH_SETATTR_MODE | CEPH_SETATTR_UID,
            mode: 0o600,
            uid: 42,
            ..Default::default()
        },
    )
    .unwrap();
    let eff = fs.release(C1, f1);
    assert!(eff.iter().any(|e| matches!(e, Effect::PurgeInode(_))));
    // A cap flush is journaled too.
    let f3 = fs.create(C2, sv, "f3", CEPH_O_RDWR).unwrap();
    let cap = fs.e.cap(f3, C2).unwrap().clone();
    fs.cap_update(
        C2,
        CapUpdate {
            op: CEPH_CAP_OP_FLUSH,
            ino: f3,
            cap_id: cap.cap_id,
            seq: cap.last_sent,
            issue_seq: cap.last_issue,
            caps: cap.issued(),
            wanted: cap.wanted(),
            dirty: FW,
            flush_tid: 5,
            oldest_flush_tid: 5,
            size: 999,
            mtime: ts(99999),
            ..Default::default()
        },
    );
    fs.e.close_session(C3, false);
}

#[test]
fn replay_rebuilds_the_namespace() {
    let mut fs = Fs::new();
    workload(&mut fs);
    let records: Vec<_> =
        fs.e.journal()
            .records()
            .iter()
            .map(|(_, r)| r.clone())
            .collect();
    let replayed = Engine::replay(config(), MemJournal::new(), records.iter());
    fs.e.namespace_eq(&replayed).unwrap();
    // The allocator continues where it left off: no number is reused.
    let next_orig = {
        let mut a = fs.e.ino_allocator().clone();
        a.alloc().unwrap()
    };
    let next_replayed = {
        let mut a = replayed.ino_allocator().clone();
        a.alloc().unwrap()
    };
    assert_eq!(next_orig, next_replayed);
    // Completed requests and flushes survive replay.
    let s = replayed.session(C2).unwrap();
    assert!(s.completed_flushes.contains(&5));
    assert!(!s.completed_requests.is_empty());
    assert!(
        replayed.session(C3).is_none(),
        "closed sessions stay closed"
    );
}

#[test]
fn replayed_engine_answers_retries_from_the_table() {
    let mut fs = Fs::new();
    let req = Request::new(
        500,
        Op::Mkdir {
            path: FilePath::child(ROOT, "d"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_owner(1, 1);
    let r = fs.call_req(C1, req.clone());
    let d = r.target.unwrap().ino;
    let records: Vec<_> =
        fs.e.journal()
            .records()
            .iter()
            .map(|(_, r)| r.clone())
            .collect();
    let mut e = Engine::replay(config(), MemJournal::new(), records.iter());
    e.open_session(C1);
    let mut retry = req;
    retry.retry_attempt = 1;
    let eff = e.handle_request(C1, retry);
    let r = reply_for(&eff, C1, 500).unwrap();
    assert_eq!(
        (r.result, r.created_ino),
        (0, Some(d)),
        "not EEXIST after a restart"
    );
}

#[test]
fn restart_replay_then_reconnect_caps_and_locks() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let cap = fs.e.cap(f, C1).unwrap().clone();
    let records: Vec<_> =
        fs.e.journal()
            .records()
            .iter()
            .map(|(_, r)| r.clone())
            .collect();
    // A new MDS: replay, then clients reconnect.
    let mut e = Engine::replay(config(), MemJournal::new(), records.iter());
    e.open_session(C1);
    e.open_session(C2);
    let lock = engine::filelock::FileLock {
        start: 0,
        length: 0,
        client: 0,
        owner: 7 | (1 << 63),
        pid: 1,
        ltype: CEPH_LOCK_EXCL,
    };
    e.reconnect_cap(C1, f, cap.cap_id, cap.wanted(), cap.issued(), &[lock], &[])
        .unwrap();
    assert_eq!(
        e.reconnect_cap(C1, 0x1_0000_0000_0999, 9, 0, 0, &[], &[]),
        Err(Errno::ESTALE)
    );
    assert!(
        e.reconnect_done().is_empty(),
        "a consistent reconnect needs no messages"
    );
    let c = e.cap(f, C1).unwrap();
    assert_eq!(
        (c.cap_id, c.issued(), c.last_sent),
        (cap.cap_id, cap.issued(), 0)
    );
    assert_eq!(
        e.file_locks(f, false)[0].client,
        C1,
        "the lock is reinstated for the session's client"
    );
    // The writer's range survived the restart; a reader now forces sync I/O.
    assert_eq!(e.inode(f).unwrap().client_ranges.get(&C1), Some(&(4 << 20)));
    let eff = e.handle_request(
        C2,
        Request::new(
            1,
            Op::Open {
                path: FilePath::ino(f),
                flags: CEPH_O_RDONLY,
                mode: 0,
                mask: 0,
                old_size: 0,
            },
        ),
    );
    let rv = revokes(&eff);
    assert_eq!(rv.len(), 1);
    assert_eq!(rv[0].client, C1);
    assert_eq!(rv[0].seq, 1, "sequence numbers restart after reconnect");
    let eff = e.handle_request(
        C2,
        Request::new(
            2,
            Op::SetFileLock {
                path: FilePath::ino(f),
                lock: FileLockArgs {
                    rule: CEPH_LOCK_FCNTL,
                    ltype: CEPH_LOCK_SHARED,
                    owner: 1 << 63,
                    pid: 2,
                    start: 5,
                    length: 1,
                    wait: false,
                },
            },
        ),
    );
    assert_eq!(
        reply_for(&eff, C2, 2).unwrap().result,
        Errno::EAGAIN.as_neg()
    );
}
