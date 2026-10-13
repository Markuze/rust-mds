//! Multi-client capability scenarios: sharing, exclusive writers, mixed
//! readers and writers, two-step revocation, flushes and FLUSH_ACK ordering,
//! max_size, directory caps, unlink of open files, stale sessions.

mod common;

use common::*;
use engine::caps::cap_string;
use engine::consts::*;
use engine::*;

const ALL_EXCL: u32 = P | AS | AX | LS | XS | XX | FS | FX | FC | FR | FW | FB;

fn flush(fs: &Fs, client: ClientId, ino: Ino, tid: u64, dirty: u32, size: u64) -> CapUpdate {
    let cap = fs.e.cap(ino, client).unwrap().clone();
    CapUpdate {
        op: CEPH_CAP_OP_FLUSH,
        ino,
        cap_id: cap.cap_id,
        seq: cap.last_sent,
        issue_seq: cap.last_issue,
        mseq: cap.mseq,
        caps: cap.issued(),
        wanted: cap.wanted(),
        dirty,
        flush_tid: tid,
        oldest_flush_tid: tid,
        size,
        mtime: ts(9000 + tid as u32),
        ctime: ts(9000 + tid as u32),
        ..Default::default()
    }
}

#[test]
fn creator_gets_exclusive_caps() {
    let mut fs = Fs::new();
    let r = fs.create_reply(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let st = r.target.unwrap();
    assert_eq!(cap_string(st.cap.caps), "pAsxLsXsxFsxcrwb");
    assert_eq!(
        st.cap.cap_id, 1,
        "creator's cap id is 1 (matches kclient async create)"
    );
    assert_eq!(st.cap.flags, CEPH_CAP_FLAG_AUTH);
    assert_eq!(
        st.cap.wanted,
        P | FS | FR | FC | FX | FW | FB | AS | AX | XS | XX
    );
    assert_eq!(
        st.max_size,
        4 << 20,
        "initial writeable range is one stripe unit"
    );
    // The parent directory: the wrlock of the mutation recalls Fs Fx from a
    // client that is not the directory's creator (the kernel re-reads the
    // directory and gets them re-issued, bumping its shared_gen).
    let dir = r.dentry.unwrap().dir;
    assert_eq!(cap_string(dir.cap.caps), "pAsLsXs");
    // A read-only create gets no write caps and no range.
    let st = fs
        .create_reply(C1, ROOT, "ro", CEPH_O_RDONLY)
        .unwrap()
        .target
        .unwrap();
    assert_eq!(st.cap.caps & (FX | FW | FB), 0);
    assert_eq!(st.cap.caps & (FS | FR | FC), FS | FR | FC);
    assert_eq!(st.max_size, 0);
}

#[test]
fn readers_share_without_revocation() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let a = fs.open(C1, f, CEPH_O_RDONLY).unwrap();
    let b = fs.open(C2, f, CEPH_O_RDONLY).unwrap();
    let c = fs.open(C3, f, CEPH_O_RDONLY).unwrap();
    for st in [&a, &b, &c] {
        assert_eq!(
            st.cap.caps & (FS | FC | FR),
            FS | FC | FR,
            "{}",
            cap_string(st.cap.caps)
        );
        assert_eq!(st.cap.caps & (FX | FW | FB), 0);
    }
    assert!(revokes(&fs.last).is_empty());
}

#[test]
fn second_opener_turns_exclusive_writer_into_sync_io() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    assert_eq!(fs.issued(f, C1), ALL_EXCL);
    let (tid, eff) = fs.send(
        C2,
        Op::Open {
            path: FilePath::ino(f),
            flags: CEPH_O_RDONLY,
            mode: 0,
            mask: 0,
            old_size: 0,
        },
    );
    // The reader's open is answered at once, without conflicting bits.
    let r = reply_for(&eff, C2, tid).unwrap();
    assert_eq!(
        r.target.as_ref().unwrap().cap.caps & (FS | FC | FR | FW | FB | FX),
        0
    );
    // The writer is told to give up buffering, caching and exclusivity.
    let rv = revokes(&eff);
    assert_eq!(rv.len(), 1);
    assert_eq!(rv[0].client, C1);
    assert_eq!(rv[0].caps & (FS | FX | FC | FB | AX | XX), 0);
    assert_eq!(rv[0].caps & (FR | FW), FR | FW, "sync I/O stays possible");
    assert!(rv[0].seq > 1);
    assert_eq!(fs.issued(f, C1), ALL_EXCL, "issued until acknowledged");
    // Nothing is granted to the reader before the writer acknowledges.
    assert!(caps_to(&eff, C2).is_empty());
    let eff = fs.ack(&rv[0]);
    let grants = caps_to(&eff, C2);
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].op, CEPH_CAP_OP_GRANT);
    assert_eq!(grants[0].caps & (FR | FW), FR | FW);
    assert_eq!(grants[0].caps & (FB | FC | FS | FX), 0);
    assert_eq!(fs.issued(f, C1) & (FB | FC | FX | FS), 0);
    // Both now do synchronous I/O.
    for c in [C1, C2] {
        let i = fs.issued(f, c);
        assert_eq!(i & (FR | FW), FR | FW, "{}", cap_string(i));
        assert_eq!(i & (FB | FC), 0);
    }
}

#[test]
fn stale_ack_does_not_complete_revocation() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let old_seq = fs.e.cap(f, C1).unwrap().last_sent;
    let (_, eff) = fs.send(
        C2,
        Op::Open {
            path: FilePath::ino(f),
            flags: CEPH_O_RDONLY,
            mode: 0,
            mask: 0,
            old_size: 0,
        },
    );
    let rv = revokes(&eff).remove(0);
    // A racing update from before the revoke still holds everything.
    let racing = CapUpdate {
        op: CEPH_CAP_OP_UPDATE,
        ino: f,
        cap_id: rv.cap_id,
        seq: old_seq,
        issue_seq: rv.issue_seq,
        caps: ALL_EXCL,
        wanted: rv.wanted,
        ..Default::default()
    };
    let eff = fs.cap_update(C1, racing);
    assert!(
        caps_to(&eff, C2).is_empty(),
        "no grant while the revoke is unacknowledged"
    );
    assert_eq!(fs.issued(f, C1), ALL_EXCL);
    // The real ack.
    let eff = fs.ack(&rv);
    assert_eq!(caps_to(&eff, C2).len(), 1);
    // A wrong cap id is ignored.
    let cap = fs.e.cap(f, C1).unwrap().clone();
    let bogus = CapUpdate {
        op: CEPH_CAP_OP_UPDATE,
        ino: f,
        cap_id: cap.cap_id + 100,
        seq: cap.last_sent,
        caps: 0,
        ..Default::default()
    };
    fs.cap_update(C1, bogus);
    assert_eq!(fs.e.cap(f, C1).unwrap().issued(), cap.issued());
}

#[test]
fn seqs_increase_and_cap_id_is_stable() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let id = fs.e.cap(f, C1).unwrap().cap_id;
    let mut last = fs.e.cap(f, C1).unwrap().last_sent;
    for c in [C2, C3] {
        let (_, eff) = fs.send(
            c,
            Op::Open {
                path: FilePath::ino(f),
                flags: CEPH_O_RDWR,
                mode: 0,
                mask: 0,
                old_size: 0,
            },
        );
        let mut all = eff.clone();
        all.extend(fs.settle(&eff));
        for m in caps_to(&all, C1) {
            assert_eq!(m.cap_id, id);
            assert!(m.seq > last, "seq {} after {}", m.seq, last);
            last = m.seq;
        }
    }
}

#[test]
fn setattr_waits_for_writer_flush() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let (tid, eff) = fs.send(
        C2,
        Op::Setattr {
            path: FilePath::ino(f),
            args: SetattrArgs {
                mask: CEPH_SETATTR_SIZE,
                size: 10,
                ..Default::default()
            },
        },
    );
    assert!(
        reply_for(&eff, C2, tid).is_none(),
        "truncate waits for the writer"
    );
    let rv = revokes(&eff);
    assert_eq!(rv.len(), 1);
    assert_eq!(rv[0].client, C1);
    // During a size change everybody keeps at most Fc and Fb.
    assert_eq!(rv[0].caps & (FS | FX | FR | FW | FL), 0);
    assert_eq!(rv[0].caps & (FC | FB), FC | FB);
    // The writer acknowledges and flushes the size it wrote.
    let mut m = flush(&fs, C1, f, 1, FW, 4096);
    m.seq = rv[0].seq;
    m.caps = rv[0].caps;
    let eff = fs.cap_update(C1, m);
    let acks: Vec<_> = caps_to(&eff, C1)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_FLUSH_ACK)
        .collect();
    assert_eq!(acks.len(), 1);
    assert_eq!((acks[0].flush_tid, acks[0].dirty), (1, FW));
    let r = reply_for(&eff, C2, tid).expect("setattr done after the ack");
    assert_eq!(r.target.as_ref().unwrap().size, 10);
    assert_eq!(
        r.target.as_ref().unwrap().truncate_seq,
        2,
        "4096 -> 10 is a truncation"
    );
    let trunc: Vec<_> = caps_to(&eff, C1)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_TRUNC)
        .collect();
    assert_eq!(trunc.len(), 1);
    assert_eq!(trunc[0].inode.as_ref().unwrap().size, 10);
    // Order: the flush ack (its record committed first), TRUNC, then the reply.
    let pos = |pred: &dyn Fn(&Effect) -> bool| eff.iter().position(pred).unwrap();
    let p_ack = pos(&|e| matches!(e, Effect::Caps(m) if m.op == CEPH_CAP_OP_FLUSH_ACK));
    let p_trunc = pos(&|e| matches!(e, Effect::Caps(m) if m.op == CEPH_CAP_OP_TRUNC));
    let p_reply = pos(&|e| matches!(e, Effect::Reply(r) if r.tid == tid));
    assert!(p_ack < p_trunc && p_trunc < p_reply);
    // Afterwards the writer gets its exclusive caps back.
    let grants: Vec<_> = caps_to(&eff, C1)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_GRANT)
        .collect();
    assert!(
        grants
            .iter()
            .any(|g| g.caps & (FX | FW | FB) == FX | FW | FB)
    );
}

#[test]
fn getattr_by_another_client_collects_dirty_size() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let (tid, eff) = fs.send(
        C2,
        Op::Getattr {
            path: FilePath::ino(f),
            mask: CEPH_STAT_CAP_INODE_ALL,
        },
    );
    assert!(reply_for(&eff, C2, tid).is_none());
    let rv = revokes(&eff);
    assert_eq!(rv.len(), 1);
    assert_eq!(
        rv[0].caps & (FX | FW | FB),
        0,
        "fresh size needs the writer to stop writing"
    );
    let mut m = flush(&fs, C1, f, 7, FW, 777);
    m.seq = rv[0].seq;
    m.caps = rv[0].caps;
    let eff = fs.cap_update(C1, m);
    let r = reply_for(&eff, C2, tid).unwrap();
    assert_eq!(r.target.as_ref().unwrap().size, 777);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 777);
    // The writer is re-granted once the getattr is done.
    assert!(
        caps_to(&eff, C1)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_GRANT && m.caps & FX != 0)
    );
}

#[test]
fn flush_acks_only_after_commit_and_in_order() {
    let mut fs = Fs::manual();
    let (tid, _) = fs.send(
        C1,
        Op::Create {
            path: FilePath::child(ROOT, "f"),
            flags: CEPH_O_CREAT | CEPH_O_RDWR,
            mode: 0o644,
            layout: LayoutOverride::default(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let eff = fs.commit();
    let f = reply_for(&eff, C1, tid)
        .unwrap()
        .target
        .as_ref()
        .unwrap()
        .ino;
    let base = fs.e.journal().last_seq();
    let eff1 = fs.cap_update(C1, flush(&fs, C1, f, 1, FW, 100));
    let eff2 = fs.cap_update(C1, flush(&fs, C1, f, 2, FW, 200));
    assert!(
        cap_msgs(&eff1)
            .iter()
            .chain(cap_msgs(&eff2).iter())
            .all(|m| m.op != CEPH_CAP_OP_FLUSH_ACK)
    );
    assert_eq!(fs.e.journal().last_seq(), base + 2, "one record per flush");
    // The values are applied in memory already.
    assert_eq!(fs.stat(f).size, 200);
    fs.e.journal_mut().commit_upto(base + 1);
    let eff = fs.e.journal_committed();
    let acks: Vec<_> = cap_msgs(&eff)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_FLUSH_ACK)
        .collect();
    assert_eq!(
        acks.iter().map(|m| m.flush_tid).collect::<Vec<_>>(),
        vec![1]
    );
    let eff = fs.commit();
    let acks: Vec<_> = cap_msgs(&eff)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_FLUSH_ACK)
        .collect();
    assert_eq!(
        acks.iter().map(|m| m.flush_tid).collect::<Vec<_>>(),
        vec![2]
    );
    // A resent flush is acknowledged at once and not applied again.
    let mut again = flush(&fs, C1, f, 2, FW, 50);
    again.oldest_flush_tid = 2;
    let eff = fs.cap_update(C1, again);
    let acks: Vec<_> = cap_msgs(&eff)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_FLUSH_ACK)
        .collect();
    assert_eq!(acks.len(), 1);
    assert_eq!(acks[0].issue_seq, 0);
    assert_eq!(fs.e.journal().last_seq(), base + 2);
    assert_eq!(fs.stat(f).size, 200, "sizes only grow through flushes");
    // The completed flush is journaled with the update.
    let recs = fs.e.journal().records();
    let rec = &recs.iter().find(|(s, _)| *s == base + 2).unwrap().1;
    assert_eq!(rec.client_flush.unwrap().tid, 2);
    // A sync flush asks the journal to commit now.
    let flushes = fs.e.journal().flush_count();
    let mut m = flush(&fs, C1, f, 3, FW, 300);
    m.flags = CEPH_CLIENT_CAPS_SYNC;
    fs.cap_update(C1, m);
    assert!(fs.e.journal().flush_count() > flushes);
}

#[test]
fn flushed_fields_follow_cxx_rules() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let before = fs.stat(f);
    // Fx: mtime may go backwards, atime and time_warp_seq apply.
    let mut m = flush(&fs, C1, f, 1, FX, 10);
    m.mtime = ts(5);
    m.atime = ts(6);
    m.time_warp_seq = before.time_warp_seq + 3;
    m.change_attr = before.change_attr + 10;
    fs.cap_update(C1, m);
    let c = fs.stat(f);
    assert_eq!(
        (c.mtime, c.atime, c.time_warp_seq, c.size),
        (ts(5), ts(6), before.time_warp_seq + 3, 10)
    );
    assert_eq!(c.change_attr, before.change_attr + 10);
    // Fw: mtime only forward, size only grows, atime ignored.
    let mut m = flush(&fs, C1, f, 2, FW, 3);
    m.mtime = ts(4);
    m.atime = ts(99);
    fs.cap_update(C1, m);
    let c = fs.stat(f);
    assert_eq!((c.mtime, c.atime, c.size), (ts(5), ts(6), 10));
    // Ax: owner and mode, file type preserved.
    let mut m = flush(&fs, C1, f, 3, AX, 0);
    m.uid = 7;
    m.gid = 8;
    m.mode = 0o600;
    fs.cap_update(C1, m);
    let c = fs.stat(f);
    assert_eq!((c.uid, c.gid, c.mode), (7, 8, S_IFREG | 0o600));
    // Xx: the whole blob if the version is newer; an oversized blob is
    // dropped but the version still advances.
    let mut x = XattrMap::new();
    x.insert(b"user.a".to_vec(), b"1".to_vec());
    let mut m = flush(&fs, C1, f, 4, XX, 0);
    m.xattrs = Some((c.xattr_version + 1, x.clone()));
    fs.cap_update(C1, m);
    assert_eq!(fs.stat(f).xattrs, x);
    let mut big = XattrMap::new();
    big.insert(b"user.big".to_vec(), vec![0; 70_000]);
    let v = fs.stat(f).xattr_version;
    let mut m = flush(&fs, C1, f, 5, XX, 0);
    m.xattrs = Some((v + 1, big));
    fs.cap_update(C1, m);
    assert_eq!(fs.stat(f).xattrs, x);
    assert_eq!(fs.stat(f).xattr_version, v + 1);
    // Old version: ignored.
    let mut m = flush(&fs, C1, f, 6, XX, 0);
    m.xattrs = Some((1, XattrMap::new()));
    fs.cap_update(C1, m);
    assert_eq!(fs.stat(f).xattrs, x);
}

#[test]
fn max_size_grants_after_commit() {
    let mut fs = Fs::manual();
    let (tid, _) = fs.send(
        C1,
        Op::Create {
            path: FilePath::child(ROOT, "f"),
            flags: CEPH_O_CREAT | CEPH_O_WRONLY,
            mode: 0o644,
            layout: LayoutOverride::default(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let eff = fs.commit();
    let st = reply_for(&eff, C1, tid).unwrap().target.clone().unwrap();
    assert_eq!(st.max_size, 4 << 20);
    let f = st.ino;
    // The client wants to write up to 10 MiB.
    let cap = fs.e.cap(f, C1).unwrap().clone();
    let m = CapUpdate {
        op: CEPH_CAP_OP_UPDATE,
        ino: f,
        cap_id: cap.cap_id,
        seq: cap.last_sent,
        issue_seq: cap.last_issue,
        caps: cap.issued(),
        wanted: cap.wanted(),
        max_size: 10 << 20,
        ..Default::default()
    };
    let eff = fs.cap_update(C1, m);
    assert!(
        caps_to(&eff, C1).is_empty(),
        "no max_size beyond the durable range"
    );
    let eff = fs.commit();
    let g = caps_to(&eff, C1);
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].op, CEPH_CAP_OP_GRANT);
    assert_eq!(g[0].caps, cap.pending(), "a max_size grant keeps the caps");
    assert!(g[0].seq > cap.last_sent);
    assert_eq!(
        g[0].inode.as_ref().unwrap().max_size,
        24 << 20,
        "round_up(2 * (10 MiB + 1), 4 MiB)"
    );
    assert_eq!(fs.stat(f).client_ranges.get(&C1), Some(&(24 << 20)));
    // Reported size growth near the limit extends the range too.
    let mut m = flush(&fs, C1, f, 1, FW, 20 << 20);
    m.max_size = 0;
    fs.cap_update(C1, m);
    fs.commit();
    assert_eq!(fs.stat(f).client_ranges.get(&C1), Some(&(44 << 20)));
    // Giving up write caps drops the range.
    let cap = fs.e.cap(f, C1).unwrap().clone();
    let m = CapUpdate {
        op: CEPH_CAP_OP_UPDATE,
        ino: f,
        cap_id: cap.cap_id,
        seq: cap.last_sent,
        issue_seq: cap.last_issue,
        caps: P | FS | FR | FC,
        wanted: P | FS | FR | FC,
        ..Default::default()
    };
    fs.cap_update(C1, m);
    fs.commit();
    assert_eq!(fs.stat(f).client_ranges.get(&C1), None);
}

#[test]
fn directory_fs_revoked_before_namespace_change() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    // C1 alone: Fs and Fx on the directory.
    assert_eq!(fs.issued(d, C1) & (FS | FX), FS | FX);
    // C2 gets a cap on d; C1 loses Fx (two holders), both want Fs.
    fs.getattr(C2, d, 0).unwrap();
    fs.settle(&fs.last.clone());
    assert_eq!(fs.issued(d, C1) & FX, 0);
    let eff = fs.want(C2, d, P | AS | LS | XS | FS);
    fs.settle(&eff);
    let eff = fs.want(C1, d, P | AS | LS | XS | FS);
    fs.settle(&eff);
    assert_eq!(fs.issued(d, C1) & FS, FS);
    assert_eq!(fs.issued(d, C2) & FS, FS);
    // C2 creates in d with an embedded release of its own Fs (as the kernel does).
    let cap2 = fs.e.cap(d, C2).unwrap().clone();
    let mut req = Request::new(
        0,
        Op::Mknod {
            path: FilePath::child(d, "x"),
            mode: S_IFREG | 0o644,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_owner(5, 5)
    .with_flags(CEPH_MDS_FLAG_WANT_DENTRY);
    req.releases.push(RequestRelease {
        ino: d,
        cap_id: cap2.cap_id,
        caps: cap2.issued() & !FS,
        wanted: cap2.wanted(),
        seq: cap2.last_sent,
        issue_seq: cap2.last_issue,
        mseq: 0,
        dname: None,
    });
    let (tid, eff) = fs.send_req(C2, req);
    assert!(
        reply_for(&eff, C2, tid).is_none(),
        "waits for C1 to drop Fs"
    );
    let rv = revokes(&eff);
    assert_eq!(rv.len(), 1, "only the other client is revoked");
    assert_eq!(rv[0].client, C1);
    assert_eq!(rv[0].ino, d);
    assert_eq!(rv[0].caps & FS, 0);
    let eff = fs.ack(&rv[0]);
    let r = reply_for(&eff, C2, tid).expect("create proceeds after the ack");
    assert_eq!(r.result, 0);
    // After the commit both get Fs back (they want it).
    assert!(
        caps_to(&eff, C1)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_GRANT && m.ino == d && m.caps & FS != 0)
    );
    assert_eq!(fs.issued(d, C2) & FS, FS);
}

#[test]
fn directories_never_get_async_dirop_bits() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    // The kernel asks for Fx plus CEPH_CAP_ANY_DIR_OPS on directories it writes.
    let eff = fs.want(C1, d, P | AS | LS | XS | FS | FX | CEPH_CAP_ANY_DIR_OPS);
    fs.settle(&eff);
    let i = fs.issued(d, C1);
    assert_eq!(
        i & (FC | FR | FW | FB | FL | CEPH_CAP_FILE_WREXTEND),
        0,
        "{}",
        cap_string(i)
    );
    assert_eq!(
        i & (FS | FX),
        FS | FX,
        "a lone client keeps the exclusive dir state"
    );
    let st = fs.open(C1, d, CEPH_O_DIRECTORY).unwrap();
    assert_eq!(st.cap.caps & CEPH_CAP_ANY_DIR_OPS, 0);
}

#[test]
fn readdir_from_second_client_revokes_dir_fx() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    fs.mkfile(C1, d, "a").unwrap();
    assert_eq!(fs.issued(d, C1) & FX, FX);
    let (tid, eff) = fs.send(
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
    assert!(
        reply_for(&eff, C2, tid).is_none(),
        "listing waits for the exclusive holder"
    );
    let rv = revokes(&eff);
    assert_eq!(rv.len(), 1);
    assert_eq!(rv[0].caps & FX, 0);
    let eff = fs.ack(&rv[0]);
    let r = reply_for(&eff, C2, tid).unwrap();
    match &r.extra {
        ReplyExtra::Readdir(rd) => assert_eq!(rd.entries.len(), 1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn unlink_of_file_open_elsewhere_keeps_stray_until_release() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let mut m = flush(&fs, C1, f, 1, FW, 1234);
    m.max_size = 0;
    fs.cap_update(C1, m);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 1234);
    let (tid, eff) = fs.send(
        C2,
        Op::Unlink {
            path: FilePath::child(ROOT, "f"),
        },
    );
    assert!(
        reply_for(&eff, C2, tid).is_none(),
        "link count change waits for Ls"
    );
    let rv = revokes(&eff);
    assert!(rv.iter().all(|m| m.client == C1));
    assert!(rv.iter().any(|m| m.ino == f && m.caps & LS == 0));
    let mut all = Vec::new();
    for m in &rv {
        all.extend(fs.ack(m));
    }
    let r = reply_for(&all, C2, tid).unwrap();
    assert_eq!(r.result, 0);
    // The inode lives on as a stray, outside the namespace's statistics.
    assert_eq!(fs.stat(f).nlink, 0);
    assert_eq!(fs.e.strays(), vec![f]);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 0);
    assert_eq!(fs.stat(ROOT).rstat.rfiles, 0);
    assert_eq!(fs.lookup(C2, ROOT, "f"), Err(Errno::ENOENT));
    // The holder is told promptly (Ls with nlink 0) so it releases soon.
    let g: Vec<_> = caps_to(&all, C1)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_GRANT && m.ino == f)
        .collect();
    assert!(!g.is_empty());
    assert_eq!(g.last().unwrap().inode.as_ref().unwrap().nlink, 0);
    assert_ne!(g.last().unwrap().caps & LS, 0);
    // The open file still works for its holder.
    let st = fs.getattr(C1, f, CEPH_STAT_CAP_INODE_ALL).unwrap();
    assert_eq!(st.nlink, 0);
    let mut m = flush(&fs, C1, f, 2, FW, 5000);
    m.max_size = 0;
    fs.cap_update(C1, m);
    assert_eq!(fs.stat(f).size, 5000);
    assert_eq!(fs.stat(ROOT).rstat.rbytes, 0);
    // The last release purges it.
    let eff = fs.release(C1, f);
    let purge: Vec<_> = eff
        .iter()
        .filter_map(|e| {
            if let Effect::PurgeInode(p) = e {
                Some(p.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(purge.len(), 1);
    assert_eq!(purge[0].ino, f);
    assert!(purge[0].size >= 5000);
    assert!(fs.e.inode(f).is_none());
    assert_eq!(
        fs.try_call(C1, Op::LookupIno { ino: f, mask: 0 })
            .unwrap_err(),
        Errno::ESTALE
    );
}

#[test]
fn stale_reader_caps_are_revoked_silently() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    fs.open(C1, f, CEPH_O_RDONLY).unwrap();
    fs.e.session_stale(C1);
    let (tid, eff) = fs.send(
        C2,
        Op::Open {
            path: FilePath::ino(f),
            flags: CEPH_O_RDWR,
            mode: 0,
            mask: 0,
            old_size: 0,
        },
    );
    let r = reply_for(&eff, C2, tid).unwrap();
    assert_eq!(
        r.target.as_ref().unwrap().cap.caps & (FX | FW | FB),
        FX | FW | FB,
        "no wait for a stale client"
    );
    assert!(
        caps_to(&eff, C1).is_empty(),
        "nothing is sent to a stale session"
    );
    assert_eq!(fs.issued(f, C1) & (FS | FC | FR), 0);
    // Replies to a stale session carry no caps.
    let st = fs.getattr(C1, ROOT, 0).unwrap();
    assert_eq!(st.cap.caps, 0);
    // When the session renews, its caps are re-issued.
    let eff = fs.e.session_resume(C1);
    let g = caps_to(&eff, C1);
    assert!(!g.is_empty());
    assert!(g.iter().all(|m| m.op == CEPH_CAP_OP_GRANT));
    fs.e.check_cap_invariants().unwrap();
}

#[test]
fn stale_writer_is_evicted_and_size_recovered() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let eff = fs.e.session_stale(C1);
    assert!(
        eff.is_empty(),
        "a stale writer nobody competes with keeps its caps: {eff:?}"
    );
    assert_eq!(fs.issued(f, C1), ALL_EXCL);
    let (tid, eff) = fs.send(
        C2,
        Op::Open {
            path: FilePath::ino(f),
            flags: CEPH_O_RDONLY,
            mode: 0,
            mask: 0,
            old_size: 0,
        },
    );
    assert!(
        eff.iter()
            .any(|e| matches!(e, Effect::EvictClient { client } if *client == C1))
    );
    let r = reply_for(&eff, C2, tid).unwrap();
    assert_eq!(
        r.target.as_ref().unwrap().cap.caps & (FS | FR | FC),
        0,
        "the stale writer still holds its caps"
    );
    let eff = fs.e.close_session(C1, true);
    assert!(eff.iter().any(|e| matches!(e, Effect::RecoverFileSize { ino, max_size, .. } if *ino == f && *max_size == 4 << 20)));
    let g = caps_to(&eff, C2);
    assert!(
        g.iter()
            .any(|m| m.op == CEPH_CAP_OP_GRANT && m.caps & FR != 0)
    );
    assert!(fs.e.session(C1).is_none());
    assert!(fs.e.cap(f, C1).is_none());
}

#[test]
fn release_with_old_issue_seq_keeps_cap() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let first = fs.e.cap(f, C1).unwrap().clone();
    fs.getattr(C1, f, 0).unwrap();
    let now = fs.e.cap(f, C1).unwrap().clone();
    assert!(now.last_issue > first.last_issue);
    fs.e.handle_cap_release(
        C1,
        &[CapReleaseItem {
            ino: f,
            cap_id: first.cap_id,
            mseq: 0,
            issue_seq: first.last_issue,
        }],
    );
    assert!(fs.e.cap(f, C1).is_some(), "a newer reply re-issued the cap");
    fs.e.handle_cap_release(
        C1,
        &[CapReleaseItem {
            ino: f,
            cap_id: now.cap_id,
            mseq: 0,
            issue_seq: now.last_issue,
        }],
    );
    assert!(fs.e.cap(f, C1).is_none());
    assert!(!fs.e.session(C1).unwrap().caps.contains(&f));
}

#[test]
fn wanted_from_stale_issue_seq_only_grows() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let cap = fs.e.cap(f, C1).unwrap().clone();
    let m = CapUpdate {
        op: CEPH_CAP_OP_UPDATE,
        ino: f,
        cap_id: cap.cap_id,
        seq: cap.last_sent,
        issue_seq: cap.last_issue.wrapping_sub(1),
        caps: cap.issued(),
        wanted: P,
        ..Default::default()
    };
    fs.cap_update(C1, m);
    assert_eq!(fs.e.cap(f, C1).unwrap().wanted(), cap.wanted());
    let m = CapUpdate {
        op: CEPH_CAP_OP_UPDATE,
        ino: f,
        cap_id: cap.cap_id,
        seq: cap.last_sent,
        issue_seq: cap.last_issue,
        caps: cap.issued(),
        wanted: P,
        ..Default::default()
    };
    fs.cap_update(C1, m);
    assert_eq!(fs.e.cap(f, C1).unwrap().wanted(), P);
}

#[test]
fn recall_hint_for_large_sessions() {
    let mut cfg = config();
    cfg.max_caps_per_client = 2;
    let mut fs = Fs::with(cfg, MemJournal::new());
    for n in ["a", "b", "c"] {
        fs.mkfile(C1, ROOT, n).unwrap();
    }
    let hints = fs.e.recall_hints();
    assert!(hints.contains(&Effect::Recall {
        client: C1,
        max_caps: 2
    }));
    assert!(
        !hints
            .iter()
            .any(|h| matches!(h, Effect::Recall { client, .. } if *client == C2))
    );
}

#[test]
fn session_close_drops_caps_and_unblocks() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    // C2's truncate waits for C1, which then goes away.
    let (tid, eff) = fs.send(
        C2,
        Op::Setattr {
            path: FilePath::ino(f),
            args: SetattrArgs {
                mask: CEPH_SETATTR_SIZE,
                size: 0,
                ..Default::default()
            },
        },
    );
    assert!(reply_for(&eff, C2, tid).is_none());
    let eff = fs.e.close_session(C1, false);
    assert!(
        reply_for(&eff, C2, tid).is_some(),
        "the waiting request proceeds"
    );
    assert!(fs.e.cap(f, C1).is_none());
    assert_eq!(fs.stat(f).client_ranges.get(&C1), None);
    // Requests from the closed session are ignored.
    let eff = fs.e.handle_request(
        C1,
        Request::new(
            999,
            Op::Getattr {
                path: FilePath::ino(ROOT),
                mask: 0,
            },
        ),
    );
    assert!(eff.is_empty());
}

#[test]
fn closing_session_purges_its_strays() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    fs.unlink(C1, ROOT, "f").unwrap();
    assert_eq!(fs.e.strays(), vec![f]);
    let eff = fs.e.close_session(C1, false);
    assert!(
        eff.iter()
            .any(|e| matches!(e, Effect::PurgeInode(p) if p.ino == f))
    );
    assert!(fs.e.strays().is_empty());
}

#[test]
fn mix_and_exclusive_transitions_keep_invariants() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    for c in [C2, C3] {
        let (_, eff) = fs.send(
            c,
            Op::Open {
                path: FilePath::ino(f),
                flags: CEPH_O_WRONLY,
                mode: 0,
                mask: 0,
                old_size: 0,
            },
        );
        fs.settle(&eff);
    }
    for c in [C1, C2, C3] {
        let i = fs.issued(f, c);
        assert_eq!(i & (FX | FB | FC | FS), 0, "{}", cap_string(i));
        assert_eq!(i & FW, FW);
    }
    // Everybody but C3 closes: C3 becomes the exclusive writer again.
    for c in [C1, C2] {
        let eff = fs.release(c, f);
        fs.settle(&eff);
    }
    let i = fs.issued(f, C3);
    assert_eq!(
        i & (FX | FW | FB | FC | FS | FR),
        FX | FW | FB | FC | FS | FR,
        "{}",
        cap_string(i)
    );
}
