//! fcntl and flock byte-range locks: conflicts, GETLK, non-blocking and
//! blocking requests, wake-ups, interruption, deadlock detection, cleanup.

mod common;

use common::*;
use engine::consts::*;
use engine::*;

const OWNER_BIT: u64 = 1 << 63;

fn lk(rule: u8, ltype: u8, owner: u64, start: u64, length: u64, wait: bool) -> FileLockArgs {
    FileLockArgs {
        rule,
        ltype,
        owner: owner | OWNER_BIT,
        pid: 100 + owner,
        start,
        length,
        wait,
    }
}

fn setlk(fs: &mut Fs, c: ClientId, ino: Ino, a: FileLockArgs) -> (u64, Vec<Effect>) {
    fs.send(
        c,
        Op::SetFileLock {
            path: FilePath::ino(ino),
            lock: a,
        },
    )
}

fn result(eff: &[Effect], c: ClientId, tid: u64) -> Option<i32> {
    reply_for(eff, c, tid).map(|r| r.result)
}

fn getlk(fs: &mut Fs, c: ClientId, ino: Ino, a: FileLockArgs) -> engine::filelock::FileLock {
    let r = fs.call(
        c,
        Op::GetFileLock {
            path: FilePath::ino(ino),
            lock: a,
        },
    );
    match r.extra {
        ReplyExtra::FileLock(l) => l,
        other => panic!("{other:?}"),
    }
}

#[test]
fn conflicts_and_getlk() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let (t, e) = setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 100, false),
    );
    assert_eq!(result(&e, C1, t), Some(0));
    // Another client: conflict, non-blocking.
    let (t, e) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_SHARED, 1, 50, 10, false),
    );
    assert_eq!(result(&e, C2, t), Some(Errno::EAGAIN.as_neg()));
    // Disjoint range is fine.
    let (t, e) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_SHARED, 1, 100, 0, false),
    );
    assert_eq!(result(&e, C2, t), Some(0));
    // GETLK reports the conflicting lock with the holder's identity.
    let l = getlk(
        &mut fs,
        C3,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_SHARED, 1, 0, 0, false),
    );
    assert_eq!(
        (l.client, l.start, l.length, l.ltype),
        (C1, 0, 100, CEPH_LOCK_EXCL)
    );
    assert_eq!(l.pid, 101);
    let l = getlk(
        &mut fs,
        C3,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_SHARED, 1, 200, 10, false),
    );
    assert_eq!(
        l.ltype, CEPH_LOCK_UNLOCK,
        "shared test against a shared lock: no conflict"
    );
    assert_eq!(l.client, C3);
    let l = getlk(
        &mut fs,
        C3,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 200, 10, false),
    );
    assert_eq!(l.client, C2);
    // Same client, different owners conflict too.
    let (t, e) = setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 2, 0, 1, false),
    );
    assert_eq!(result(&e, C1, t), Some(Errno::EAGAIN.as_neg()));
    // flock locks live in a separate table.
    let (t, e) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_EXCL, 9, 0, 0, false),
    );
    assert_eq!(result(&e, C2, t), Some(0));
    assert_eq!(fs.e.file_locks(f, true).len(), 1);
    assert_eq!(fs.e.file_locks(f, false).len(), 2);
    // Bad rules.
    let (t, e) = setlk(&mut fs, C2, f, lk(9, CEPH_LOCK_EXCL, 9, 0, 0, false));
    assert_eq!(result(&e, C2, t), Some(Errno::EOPNOTSUPP.as_neg()));
    let r = fs.call(
        C2,
        Op::GetFileLock {
            path: FilePath::ino(f),
            lock: lk(CEPH_LOCK_FCNTL_INTR, 1, 1, 0, 0, false),
        },
    );
    assert_eq!(r.result, Errno::EINVAL.as_neg());
    // Lock requests are not journaled.
    let n = fs.e.journal().last_seq();
    setlk(
        &mut fs,
        C3,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_SHARED, 1, 1000, 1, false),
    );
    assert_eq!(fs.e.journal().last_seq(), n);
}

#[test]
fn blocking_wait_is_woken_by_unlock() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    let (wait_tid, e) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 10, 10, true),
    );
    assert_eq!(result(&e, C2, wait_tid), None, "blocked");
    assert_eq!(fs.e.waiting_requests(), 1);
    // A partial unlock that does not free the range keeps it waiting.
    let (t, e) = setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_UNLOCK, 1, 100, 10, false),
    );
    assert_eq!(result(&e, C1, t), Some(0));
    assert_eq!(result(&e, C2, wait_tid), None);
    // Unlocking the range wakes the waiter in the same call.
    let (t, e) = setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_UNLOCK, 1, 0, 50, false),
    );
    assert_eq!(result(&e, C1, t), Some(0));
    assert_eq!(result(&e, C2, wait_tid), Some(0));
    assert_eq!(fs.e.waiting_requests(), 0);
    let held = fs.e.file_locks(f, false);
    assert!(
        held.iter()
            .any(|l| l.client == C2 && l.start == 10 && l.length == 10)
    );
}

#[test]
fn shared_waiters_wake_together() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    let (a, _) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_SHARED, 1, 0, 0, true),
    );
    let (b, _) = setlk(
        &mut fs,
        C3,
        f,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_SHARED, 1, 0, 0, true),
    );
    let (_, e) = setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_UNLOCK, 1, 0, 0, false),
    );
    assert_eq!(result(&e, C2, a), Some(0));
    assert_eq!(result(&e, C3, b), Some(0));
}

#[test]
fn interrupted_wait_returns_eintr() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    let (wait_tid, _) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    // The kernel cancels with an *_INTR unlock of the same owner and range.
    let (t, e) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FCNTL_INTR, CEPH_LOCK_UNLOCK, 1, 0, 0, false),
    );
    assert_eq!(result(&e, C2, t), Some(0));
    assert_eq!(result(&e, C2, wait_tid), Some(Errno::EINTR.as_neg()));
    // The INTR unlock does not drop held locks.
    assert_eq!(fs.e.file_locks(f, false).len(), 1);
    // A non-INTR unlock of the same range by the waiting owner also cancels.
    let (wait2, _) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    let (_, e) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_UNLOCK, 1, 0, 0, false),
    );
    assert_eq!(result(&e, C2, wait2), Some(Errno::EINTR.as_neg()));
}

#[test]
fn deadlock_is_detected() {
    let mut fs = Fs::new();
    let a = fs.mkfile(C1, ROOT, "a").unwrap();
    let b = fs.mkfile(C1, ROOT, "b").unwrap();
    setlk(
        &mut fs,
        C1,
        a,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    setlk(
        &mut fs,
        C2,
        b,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    let (w, e) = setlk(
        &mut fs,
        C1,
        b,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    assert_eq!(result(&e, C1, w), None);
    let (t, e) = setlk(
        &mut fs,
        C2,
        a,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    assert_eq!(result(&e, C2, t), Some(Errno::EDEADLK.as_neg()));
    // A three-party cycle across inodes.
    let c = fs.mkfile(C1, ROOT, "c").unwrap();
    setlk(
        &mut fs,
        C3,
        c,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    let (w2, e) = setlk(
        &mut fs,
        C2,
        c,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    assert_eq!(result(&e, C2, w2), None, "C2 waits for C3, C1 waits for C2");
    let (t, e) = setlk(
        &mut fs,
        C3,
        a,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    assert_eq!(result(&e, C3, t), Some(Errno::EDEADLK.as_neg()));
    // flock has no deadlock detection: it just waits.
    setlk(
        &mut fs,
        C1,
        a,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    setlk(
        &mut fs,
        C2,
        b,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    let (_, e) = setlk(
        &mut fs,
        C1,
        b,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    assert!(replies(&e).is_empty());
    let (_, e) = setlk(
        &mut fs,
        C2,
        a,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    assert!(replies(&e).is_empty());
}

#[test]
fn upgrade_downgrade_and_owner_identity() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_SHARED, 1, 0, 100, false),
    );
    // Same owner (bit 63 set), different pid: the same owner, so an upgrade.
    let mut a = lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 100, false);
    a.pid = 999;
    let (t, e) = setlk(&mut fs, C1, f, a);
    assert_eq!(result(&e, C1, t), Some(0));
    let held = fs.e.file_locks(f, false);
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].ltype, CEPH_LOCK_EXCL);
    // Old clients without bit 63: the pid distinguishes owners.
    let old = |pid| FileLockArgs {
        rule: CEPH_LOCK_FCNTL,
        ltype: CEPH_LOCK_SHARED,
        owner: 5,
        pid,
        start: 200,
        length: 10,
        wait: false,
    };
    setlk(&mut fs, C2, f, old(1));
    let mut ex = old(2);
    ex.ltype = CEPH_LOCK_EXCL;
    let (t, e) = setlk(&mut fs, C2, f, ex);
    assert_eq!(result(&e, C2, t), Some(Errno::EAGAIN.as_neg()));
}

#[test]
fn session_close_drops_locks_and_wakes_waiters() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    let (w, _) = setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    // A third waiter from the departing client itself is dropped silently.
    let (w3, _) = setlk(
        &mut fs,
        C1,
        f,
        lk(CEPH_LOCK_FCNTL, CEPH_LOCK_SHARED, 2, 0, 0, true),
    );
    let eff = fs.e.close_session(C1, true);
    assert_eq!(
        result(&eff, C2, w),
        Some(0),
        "the departing client's locks are released"
    );
    assert_eq!(result(&eff, C1, w3), None);
    assert!(fs.e.file_locks(f, false).iter().all(|l| l.client == C2));
    assert!(fs.e.session(C1).is_none());
}

#[test]
fn cap_release_drops_that_clients_locks() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    fs.getattr(C2, f, 0).unwrap();
    setlk(
        &mut fs,
        C2,
        f,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_EXCL, 1, 0, 0, false),
    );
    let (w, _) = setlk(
        &mut fs,
        C3,
        f,
        lk(CEPH_LOCK_FLOCK, CEPH_LOCK_EXCL, 1, 0, 0, true),
    );
    let eff = fs.release(C2, f);
    assert_eq!(result(&eff, C3, w), Some(0));
    assert_eq!(fs.e.session(C2).unwrap().lock_inos.len(), 0);
}
