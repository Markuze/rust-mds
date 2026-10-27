//! Retained-target retries use the production step and commit boundaries.
mod common;
use common::*;
use engine::consts::*;
use engine::journal::RecordKind;
use engine::*;

fn permit() -> Option<RecordPermit> {
    Some(RecordPermit {
        max_record_bytes: 16 << 20,
    })
}
fn assert_input_eq(a: &EngineInput, b: &EngineInput) {
    match (a, b) {
        (
            EngineInput::Request { key: a, request: x },
            EngineInput::Request { key: b, request: y },
        ) => {
            assert_eq!(a, b);
            assert_eq!(x, y);
        }
        (
            EngineInput::CapUpdate { key: a, update: x },
            EngineInput::CapUpdate { key: b, update: y },
        ) => {
            assert_eq!(a, b);
            assert_eq!(x, y);
        }
        (
            EngineInput::LifecycleCompleted { id: a, result: x },
            EngineInput::LifecycleCompleted { id: b, result: y },
        ) => {
            assert_eq!(a, b);
            assert_eq!(x, y);
        }
        _ => panic!("returned input kind changed"),
    }
}
fn key(e: &Engine<MemJournal>, client: ClientId) -> SessionKey {
    SessionKey {
        client,
        incarnation: e.session(client).unwrap().epoch,
    }
}
fn commit(e: &mut Engine<MemJournal>) -> EngineStep {
    let seq = e.journal().last_seq();
    e.journal_mut().commit_all();
    let step = e.journal_event(JournalEvent::Committed(seq));
    assert_eq!(e.journal().last_seq(), seq, "callbacks cannot submit");
    step
}
fn send(e: &mut Engine<MemJournal>, client: ClientId, request: Request) -> EngineStep {
    let before = e.journal().last_seq();
    let step = e
        .step(
            Some(EngineInput::Request {
                key: key(e, client),
                request,
            }),
            permit(),
        )
        .unwrap();
    assert!(e.journal().last_seq() <= before + 1);
    step
}
fn mknod(tid: u64, name: &str) -> Request {
    Request::new(
        tid,
        Op::Mknod {
            path: FilePath::child(ROOT, name),
            mode: S_IFREG | 0o644,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_flags(CEPH_MDS_FLAG_WANT_DENTRY)
}

#[test]
fn zero_counter_retry_preserves_creator_identity_before_releases() {
    let mut fs = Fs::new();
    fs.e.journal_mut().set_auto_commit(false);
    let req = mknod(100, "original");
    send(&mut fs.e, C1, req.clone());
    let first = commit(&mut fs.e);
    let target = reply_for(&first.effects, C1, 100)
        .unwrap()
        .target
        .as_ref()
        .unwrap()
        .ino;
    let seq = fs.e.journal().last_seq();
    let retry = send(&mut fs.e, C1, req);
    let reply = reply_for(&retry.effects, C1, 100).unwrap();
    assert_eq!(reply.result, 0);
    assert_eq!(reply.target.as_ref().unwrap().ino, target);
    assert_eq!(fs.e.journal().last_seq(), seq);
}

#[test]
fn same_tid_wrong_opcode_returns_identity_error_without_mutation() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let tid = fs.tid;
    let before = fs.e.inode(f).unwrap().clone();
    let wrong = Request::new(
        tid,
        Op::Setattr {
            path: FilePath::ino(f),
            args: SetattrArgs {
                mask: CEPH_SETATTR_MODE,
                mode: 0o777,
                ..Default::default()
            },
        },
    );
    assert!(matches!(
        fs.e.step(
            Some(EngineInput::Request {
                key: key(&fs.e, C1),
                request: wrong
            }),
            permit()
        ),
        Err(StepError::InvalidRetryIdentity { .. })
    ));
    assert_eq!(fs.e.inode(f).unwrap(), &before);
}

#[test]
fn range_only_open_records_the_original_target_without_creation() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    fs.e.journal_mut().set_auto_commit(false);
    send(
        &mut fs.e,
        C2,
        Request::new(
            100,
            Op::Open {
                path: FilePath::ino(f),
                flags: CEPH_O_RDWR,
                mode: 0,
                mask: 0,
                old_size: 0,
            },
        ),
    );
    let record = &fs.e.journal().records().last().unwrap().1;
    let completion = record
        .completed_request
        .as_ref()
        .expect("range-only open needs durable identity");
    assert_eq!(completion.created_ino, None);
    assert_eq!(completion.reply_target_ino, Some(f));
    assert_eq!(completion.op, CEPH_MDS_OP_OPEN);
}

#[test]
fn read_only_watermark_does_not_retire_canonical_history() {
    let mut fs = Fs::new();
    fs.mkfile(C1, ROOT, "f").unwrap();
    let tid = fs.tid;
    let mut read = Request::new(
        100,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    read.oldest_client_tid = 100;
    send(&mut fs.e, C1, read);
    assert!(
        fs.e.session(C1)
            .unwrap()
            .completed_requests
            .contains_key(&tid)
    );
}

#[test]
fn moved_creator_retry_returns_capless_original_target_error() {
    let mut fs = Fs::new();
    let req = mknod(100, "old");
    let first = fs.call_req(C1, req.clone());
    let f = first.target.unwrap().ino;
    fs.rename(C1, ROOT, "old", ROOT, "new").unwrap();
    let before = fs.e.caps_of(f);
    let seq = fs.e.journal().last_seq();
    let mut retry = req;
    retry.retry_attempt = 1;
    let step = send(&mut fs.e, C1, retry);
    let r = reply_for(&step.effects, C1, 100).unwrap();
    assert_eq!(r.result, Errno::EINTR.as_neg());
    assert!(r.dentry.is_none() && r.snap_trace.is_empty());
    let st = r.target.as_ref().unwrap();
    assert_eq!(st.ino, f);
    assert_eq!(
        (
            st.cap.caps,
            st.cap.wanted,
            st.cap.cap_id,
            st.cap.seq,
            st.cap.mseq,
            st.cap.flags,
            st.max_size
        ),
        (0, 0, 0, 0, 0, 0, 0)
    );
    assert_eq!(fs.e.caps_of(f), before);
    assert_eq!(fs.e.journal().last_seq(), seq);
}

#[test]
fn legacy_histories_load_without_new_target_rules_but_cannot_serve() {
    let mut fs = Fs::new();
    fs.mkfile(C1, ROOT, "f").unwrap();
    let mut cp = fs.e.checkpoint().unwrap();
    cp.recovery_semantics = RecoverySemantics::Legacy2;
    let done = &mut cp
        .sessions
        .iter_mut()
        .find(|s| s.key.client == C1)
        .unwrap()
        .completed_requests[0];
    done.reply_target_ino = None;
    done.created_ino = Some(0x1234_5678_0000);
    let mut e = Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    let result = e.step(Some(EngineInput::RecoveryControlsDrained), permit());
    assert!(matches!(result, Err(StepError::UpgradeRequired { .. })));
    assert!(matches!(
        e.checkpoint_for_retry_upgrade(),
        Err(RestoreError::UpgradeRequired(_))
    ));
}

fn drive(e: &mut Engine<MemJournal>, mut step: EngineStep) -> Vec<Effect> {
    let mut effects = Vec::new();
    for _ in 0..256 {
        effects.extend(step.effects);
        step = match step.state {
            StepState::NeedsAdmission => e.step(None, permit()).unwrap(),
            StepState::WaitingCommit(_) => commit(e),
            StepState::Idle | StepState::WaitingInput => return effects,
            StepState::Failed => panic!("engine failed"),
        };
    }
    panic!("step did not drain");
}
fn recover(cp: EngineCheckpoint, cfg: EngineConfig) -> Engine<MemJournal> {
    let mut e = Engine::restore(cfg, MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    let keys: Vec<_> = [C1, C2, C3]
        .into_iter()
        .filter_map(|client| {
            e.session(client).map(|s| SessionKey {
                client,
                incarnation: s.epoch,
            })
        })
        .collect();
    for k in &keys {
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drive(&mut e, step);
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    drive(&mut e, step);
    e
}

#[test]
fn retained_stray_survives_read_floor_checkpoint_callback_loss_and_second_error_loss() {
    let mut fs = Fs::new();
    let req = mknod(10, "retained");
    let reply = fs.call_req(C1, req.clone());
    let ino = reply.target.unwrap().ino;
    let mut read = Request::new(
        100,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    read.oldest_client_tid = 100;
    let step = send(&mut fs.e, C1, read);
    drive(&mut fs.e, step);
    let cp = fs.e.checkpoint().unwrap();
    assert_eq!(
        cp.sessions
            .iter()
            .find(|s| s.key.client == C1)
            .unwrap()
            .oldest_client_tid,
        0
    );
    let mut e = recover(cp, config());
    let step = send(
        &mut e,
        C2,
        Request::new(
            200,
            Op::Unlink {
                path: FilePath::child(ROOT, "retained"),
            },
        ),
    );
    drive(&mut e, step);
    assert_eq!(e.inode(ino).unwrap().nlink, 0);
    assert!(e.pending_jobs().is_empty());
    let cp = e.checkpoint().unwrap();
    let mut e = recover(cp, config());
    for _ in 0..2 {
        let step = send(&mut e, C1, req.clone());
        let r = reply_for(&step.effects, C1, 10).unwrap();
        assert_eq!(r.result, Errno::EINTR.as_neg());
        assert_eq!(r.target.as_ref().unwrap().ino, ino);
        drive(&mut e, step);
        let cp = e.checkpoint().unwrap();
        e = recover(cp, config());
    }
    let before = e.checkpoint().unwrap();
    let mut retirement = Request::new(
        300,
        Op::Setattr {
            path: FilePath::ino(ROOT),
            args: SetattrArgs {
                mask: CEPH_SETATTR_MODE,
                mode: 0o755,
                ..Default::default()
            },
        },
    );
    retirement.oldest_client_tid = 300;
    let step = send(&mut e, C1, retirement);
    assert!(step.effects.is_empty());
    assert!(e.session(C1).unwrap().completed_requests.contains_key(&10));
    assert!(e.inode(ino).is_some());
    let (seq, record) = e.journal().records().last().unwrap().clone();
    e.journal_mut().commit_all(); // Lose the committed callback.
    assert!(record.purged.is_empty());
    let mut loaded = Engine::restore(
        config(),
        MemJournal::at_seq(before.covered_seq, false),
        before,
    )
    .unwrap();
    loaded.replay_record(seq, &record).unwrap();
    *loaded.journal_mut() = MemJournal::at_seq(seq, false);
    assert!(
        !loaded
            .session(C1)
            .unwrap()
            .completed_requests
            .contains_key(&10)
    );
    assert!(
        loaded.inode(ino).is_some(),
        "retirement is not the purge transaction"
    );
    let loaded_cp = loaded.checkpoint().unwrap();
    let loaded = recover(loaded_cp, config());
    assert!(loaded.inode(ino).is_none());
    assert!(
        loaded
            .journal()
            .records()
            .iter()
            .any(|(s, r)| *s > seq && r.purged.contains(&ino))
    );
}

#[test]
fn full_count_returns_owned_retry_pressure_but_accepts_a_fitting_watermark() {
    let mut fs = Fs::new();
    fs.mkfile(C1, ROOT, "one").unwrap();
    let mut cfg = config();
    cfg.retry_limits.max_completions = 1;
    let mut e = recover(fs.e.checkpoint().unwrap(), cfg);
    let input = EngineInput::Request {
        key: key(&e, C1),
        request: mknod(100, "blocked"),
    };
    let seq = e.journal().last_seq();
    let returned = match e.step(Some(input), permit()).unwrap_err() {
        StepError::RetryCapacity {
            input: Some(input),
            permit: returned,
        } => {
            assert_eq!(returned, permit());
            input
        }
        other => panic!("wrong pressure classification: {other:?}"),
    };
    assert!(!e.is_failed());
    assert_eq!(e.journal().last_seq(), seq);
    assert_eq!(
        e.step(None, permit()).unwrap().state,
        StepState::Idle,
        "outside pressure is not runnable work"
    );
    let mut free = mknod(101, "retirement");
    free.oldest_client_tid = 100;
    let step = send(&mut e, C1, free);
    drive(&mut e, step);
    assert_eq!(e.session(C1).unwrap().durable_oldest_client_tid, 100);
    assert!(
        matches!(
            e.step(Some(returned), permit()),
            Err(StepError::RetryCapacity { .. })
        ),
        "the one new entry still fills the count"
    );
}

#[test]
fn protected_xattr_capacity_failure_is_sticky_and_preserves_control_ownership() {
    let mut fs = Fs::new();
    let mut req = mknod(10, "f");
    if let Op::Mknod { xattrs, .. } = &mut req.op {
        xattrs.insert(b"user.k".to_vec(), Vec::new());
    }
    let ino = fs.call_req(C1, req).target.unwrap().ino;
    let cp = fs.e.checkpoint().unwrap();
    // 256 completion + 4096 inode + 272 reference-index entry + one parent
    // byte + two (304-byte xattr map entries + six key bytes).
    let base = 256 + 4096 + 272 + 1 + 2 * (304 + 6);
    let mut cfg = config();
    cfg.retry_limits.max_bytes = base + 1024;
    let mut e = Engine::restore(
        cfg.clone(),
        MemJournal::at_seq(cp.covered_seq, false),
        cp.clone(),
    )
    .unwrap();
    let keys = [key(&e, C1), key(&e, C2), key(&e, C3)];
    e.reconnect_chunk(
        keys[0],
        &[ReconnectCap {
            ino,
            cap_id: 77,
            wanted: XX,
            issued: XX | P,
            realm: ROOT,
            snap_follows: 0,
            fcntl_locks: vec![],
            flock_locks: vec![],
        }],
        &[],
        false,
    )
    .unwrap();
    for k in &keys[1..] {
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drive(&mut e, step);
    let mut free = mknod(100, "fitting-watermark");
    free.oldest_client_tid = 100;
    assert!(matches!(
        e.step(
            Some(EngineInput::Request {
                key: keys[0],
                request: free
            }),
            permit()
        ),
        Err(StepError::Busy { .. })
    ));
    let cap = e.cap(ino, C1).unwrap().clone();
    let dirty = CapUpdate {
        op: CEPH_CAP_OP_FLUSH,
        ino,
        cap_id: 77,
        seq: cap.last_sent,
        issue_seq: cap.last_issue,
        caps: cap.pending(),
        wanted: XX,
        dirty: XX,
        flush_tid: 99,
        xattrs: Some((2, XattrMap::from([(b"user.k".to_vec(), vec![42; 4096])]))),
        ..Default::default()
    };
    let before = e.inode(ino).unwrap().clone();
    let tail = e.journal().last_seq();
    match e
        .step(
            Some(EngineInput::CapUpdate {
                key: keys[0],
                update: dirty.clone(),
            }),
            permit(),
        )
        .unwrap_err()
    {
        StepError::ControlCapacity {
            input: Some(EngineInput::CapUpdate { key, update }),
            permit: returned,
            required_completions,
            required_bytes,
            required_transient_bytes,
        } => {
            assert_eq!(key, keys[0]);
            assert_eq!(update, dirty);
            assert_eq!(returned, permit());
            assert_eq!(required_completions, Some(1));
            assert_eq!(required_bytes, Some(base + 8192));
            assert!(required_transient_bytes.unwrap() < 8 * 1024 * 1024);
        }
        other => panic!("wrong terminal capacity classification: {other:?}"),
    }
    assert!(e.is_failed());
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert_eq!(e.inode(ino).unwrap(), &before);
    assert_eq!(e.cap(ino, C1).unwrap(), &cap);
    assert!(e.session(C1).unwrap().completed_flushes.is_empty());
    assert_eq!(e.journal().last_seq(), tail);
    assert_eq!(e.session(C1).unwrap().durable_oldest_client_tid, 0);
    assert!(matches!(
        e.step(Some(EngineInput::RecoveryControlsDrained), permit()),
        Err(StepError::ControlCapacity {
            input: Some(EngineInput::RecoveryControlsDrained),
            ..
        })
    ));
    for _ in 0..3 {
        let step = e.step(None, permit()).unwrap();
        assert_eq!(step.state, StepState::Failed);
        assert!(step.effects.is_empty() && step.notices.is_empty());
        assert_eq!(step.unused_permit, permit());
    }
    assert!(
        e.journal_event(JournalEvent::Committed(tail))
            .effects
            .is_empty()
    );
    cfg.retry_limits.max_bytes = base + 8192;
    let mut replacement =
        Engine::restore(cfg, MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    replacement
        .reconnect_chunk(
            keys[0],
            &[ReconnectCap {
                ino,
                cap_id: 77,
                wanted: XX,
                issued: XX | P,
                realm: ROOT,
                snap_follows: 0,
                fcntl_locks: vec![],
                flock_locks: vec![],
            }],
            &[],
            false,
        )
        .unwrap();
    for k in &keys[1..] {
        replacement.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let step = replacement.finish_reconnect(&keys).unwrap();
    drive(&mut replacement, step);
    let step = replacement
        .step(
            Some(EngineInput::CapUpdate {
                key: keys[0],
                update: dirty,
            }),
            permit(),
        )
        .unwrap();
    assert!(step.effects.is_empty());
    let step = commit(&mut replacement);
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK && m.flush_tid == 99)
    );
    assert_eq!(replacement.session(C1).unwrap().completed_flushes.len(), 1);
}

fn creator(kind: u32, tid: u64, path: FilePath) -> Request {
    let op = match kind {
        CEPH_MDS_OP_CREATE => Op::Create {
            path,
            flags: CEPH_O_CREAT | CEPH_O_EXCL | CEPH_O_RDONLY,
            mode: 0o644,
            layout: LayoutOverride::default(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
        CEPH_MDS_OP_MKDIR => Op::Mkdir {
            path,
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
        CEPH_MDS_OP_SYMLINK => Op::Symlink {
            path,
            target: b"actual symlink".to_vec(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
        _ => Op::Mknod {
            path,
            mode: S_IFREG | 0o644,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    };
    Request::new(tid, op).with_flags(CEPH_MDS_FLAG_WANT_DENTRY)
}
fn assert_capless(r: &Reply, ino: Ino) {
    assert_eq!(r.result, Errno::EINTR.as_neg());
    assert!(r.dentry.is_none() && r.snap_realm.is_none() && r.snap_trace.is_empty());
    let st = r.target.as_ref().unwrap();
    assert_eq!(st.ino, ino);
    assert_eq!(
        (
            st.cap.caps,
            st.cap.wanted,
            st.cap.cap_id,
            st.cap.seq,
            st.cap.mseq,
            st.cap.flags,
            st.max_size
        ),
        (0, 0, 0, 0, 0, 0, 0)
    );
    assert_ne!(st.inline_version, 0);
}

#[test]
fn every_creator_reconstructs_requested_identity_or_capless_original_metadata() {
    for kind in [
        CEPH_MDS_OP_CREATE,
        CEPH_MDS_OP_MKDIR,
        CEPH_MDS_OP_MKNOD,
        CEPH_MDS_OP_SYMLINK,
    ] {
        for scenario in 0..6 {
            if scenario == 2 && kind == CEPH_MDS_OP_MKDIR {
                continue;
            }
            let mut fs = Fs::new();
            let parent = fs.mkdir(C2, ROOT, "parent").unwrap();
            fs.setxattr(C2, parent, "ceph.dir.subvolume", b"1", 0)
                .unwrap();
            let req = creator(kind, 100, FilePath::child(parent, "old"));
            let ino = fs.call_req(C1, req.clone()).target.unwrap().ino;
            match scenario {
                1 => {
                    fs.rename(C2, ROOT, "parent", ROOT, "renamed-parent")
                        .unwrap();
                }
                2 => {
                    fs.link(C2, parent, "primary", ino).unwrap();
                    fs.unlink(C2, parent, "old").unwrap();
                    fs.link(C2, parent, "old", ino).unwrap();
                }
                3 | 4 => {
                    fs.rename(C2, parent, "old", parent, "moved").unwrap();
                    if scenario == 4 {
                        assert_ne!(fs.mkfile(C2, parent, "old").unwrap(), ino);
                    }
                }
                5 => {
                    if kind == CEPH_MDS_OP_MKDIR {
                        fs.rmdir(C2, parent, "old").unwrap();
                    } else {
                        fs.unlink(C2, parent, "old").unwrap();
                    }
                    fs.rmdir(C2, ROOT, "parent").unwrap();
                    fs.retire_completed(C2);
                    fs.release(C2, parent);
                    fs.release(C1, parent);
                    assert!(fs.e.inode(parent).is_none());
                }
                _ => {}
            }
            let core = fs.e.inode(ino).unwrap().clone();
            let caps = fs.e.caps_of(ino);
            let seq = fs.e.journal().last_seq();
            let done = fs.e.session(C1).unwrap().completed_requests[&100].clone();
            let mut retry = req;
            retry.stamp = Timestamp::new(9999, 0);
            retry.oldest_client_tid = 1000;
            // A completion hit precedes both the observed floor and releases.
            retry.releases.push(RequestRelease {
                ino,
                cap_id: fs.e.cap(ino, C1).unwrap().cap_id,
                ..Default::default()
            });
            let step = send(&mut fs.e, C1, retry);
            let effects = drive(&mut fs.e, step);
            let r = reply_for(&effects, C1, 100).unwrap();
            assert_eq!(r.op, kind);
            if scenario <= 2 {
                assert_eq!(r.result, done.result);
                assert_eq!(r.target.as_ref().unwrap().ino, ino);
                assert_eq!(r.dentry.as_ref().unwrap().dir.ino, parent);
                assert_eq!(r.dentry.as_ref().unwrap().name, b"old");
            } else {
                assert_capless(r, ino);
                assert_eq!(fs.e.caps_of(ino), caps);
                let stat = r.target.as_ref().unwrap();
                assert_eq!(stat.subvolume_id, parent);
                assert_eq!(stat.symlink, core.symlink);
                assert_eq!(stat.layout, core.layout);
                if kind == CEPH_MDS_OP_MKDIR {
                    assert_eq!(stat.nlink, 1);
                }
            }
            assert_eq!(fs.e.inode(ino).unwrap(), &core);
            assert_eq!(fs.e.journal().last_seq(), seq);
            assert_eq!(fs.e.session(C1).unwrap().completed_requests[&100], done);
            assert_eq!(fs.e.session(C1).unwrap().oldest_client_tid, 0);
            if kind == CEPH_MDS_OP_CREATE {
                assert!(matches!(r.extra,ReplyExtra::Create{ino:i,..} if i==ino));
            } else {
                assert_eq!(r.extra, ReplyExtra::None);
            }
        }
    }
}

#[test]
fn wire_replay_uses_the_recorded_result_and_never_reconstructs_a_trace() {
    for kind in [
        CEPH_MDS_OP_CREATE,
        CEPH_MDS_OP_MKDIR,
        CEPH_MDS_OP_MKNOD,
        CEPH_MDS_OP_SYMLINK,
    ] {
        let mut fs = Fs::new();
        let req = creator(kind, 100, FilePath::child(ROOT, "old"));
        let ino = fs.call_req(C1, req.clone()).target.unwrap().ino;
        fs.rename(C2, ROOT, "old", ROOT, "moved").unwrap();
        fs.mkfile(C2, ROOT, "old").unwrap();
        let mut cp = fs.e.checkpoint().unwrap();
        cp.sessions
            .iter_mut()
            .find(|s| s.key.client == C1)
            .unwrap()
            .completed_requests[0]
            .result = 7;
        let mut e = recover(cp, config());
        let core = e.inode(ino).unwrap().clone();
        let seq = e.journal().last_seq();
        let mut replay = req;
        replay.flags |= CEPH_MDS_FLAG_REPLAY;
        replay.oldest_client_tid = 1000;
        let step = send(&mut e, C1, replay);
        let r = reply_for(&step.effects, C1, 100).unwrap();
        assert_eq!((r.op, r.result), (kind, 7));
        assert!(
            r.target.is_none()
                && r.dentry.is_none()
                && r.snap_realm.is_none()
                && r.snap_trace.is_empty()
        );
        if kind == CEPH_MDS_OP_CREATE {
            assert!(matches!(r.extra,ReplyExtra::Create{ino:i,..} if i==ino));
        } else {
            assert_eq!(r.extra, ReplyExtra::None);
        }
        assert_eq!(e.inode(ino).unwrap(), &core);
        assert!(e.caps_of(ino).is_empty());
        assert_eq!(e.journal().last_seq(), seq);
    }
}

#[test]
fn existing_create_and_open_complete_each_actual_mutation_exactly_once() {
    for create in [false, true] {
        for flags in [
            CEPH_O_RDONLY,
            CEPH_O_RDONLY | CEPH_O_TRUNC,
            CEPH_O_RDWR,
            CEPH_O_RDWR | CEPH_O_TRUNC,
        ] {
            for size in [0, 19] {
                let mut fs = Fs::new();
                let ino = fs.mkfile(C2, ROOT, "old").unwrap();
                if size != 0 {
                    fs.setattr(
                        C2,
                        ino,
                        SetattrArgs {
                            mask: CEPH_SETATTR_SIZE,
                            size,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
                fs.release(C2, ino);
                fs.release(C2, ROOT);
                fs.e.journal_mut().set_auto_commit(false);
                let op = if create {
                    Op::Create {
                        path: FilePath::child(ROOT, "old"),
                        flags: flags | CEPH_O_CREAT,
                        mode: 0o644,
                        layout: LayoutOverride::default(),
                        xattrs: XattrMap::new(),
                        ino: 0,
                    }
                } else {
                    Op::Open {
                        path: FilePath::child(ROOT, "old"),
                        flags,
                        mode: 0,
                        mask: 0,
                        old_size: size,
                    }
                };
                let req = Request::new(100, op)
                    .with_flags(CEPH_MDS_FLAG_WANT_DENTRY)
                    .with_stamp(Timestamp::new(3000, 0));
                let before = fs.e.journal().last_seq();
                let step = send(&mut fs.e, C1, req.clone());
                let effects = drive(&mut fs.e, step);
                assert_eq!(reply_for(&effects, C1, 100).unwrap().created_ino, None);
                if flags == CEPH_O_RDONLY {
                    assert_eq!(fs.e.journal().last_seq(), before);
                    assert!(
                        !fs.e
                            .session(C1)
                            .unwrap()
                            .completed_requests
                            .contains_key(&100)
                    );
                    continue;
                }
                assert_eq!(fs.e.journal().last_seq(), before + 1);
                let done = fs.e.session(C1).unwrap().completed_requests[&100].clone();
                assert_eq!(
                    (done.created_ino, done.reply_target_ino, done.op),
                    (None, Some(ino), req.op.code())
                );
                let mut e = recover(fs.e.checkpoint().unwrap(), config());
                let step = send(
                    &mut e,
                    C2,
                    Request::new(
                        200,
                        Op::Rename {
                            src: FilePath::child(ROOT, "old"),
                            dst: FilePath::child(ROOT, "moved"),
                        },
                    ),
                );
                drive(&mut e, step);
                let step = send(&mut e, C2, mknod(201, "old"));
                drive(&mut e, step);
                for _ in 0..2 {
                    let core = e.inode(ino).unwrap().clone();
                    let seq = e.journal().last_seq();
                    let step = send(&mut e, C1, req.clone());
                    let r = reply_for(&step.effects, C1, 100).unwrap();
                    assert_capless(r, ino);
                    assert_eq!(r.extra, ReplyExtra::None);
                    assert_eq!(r.created_ino, None);
                    drive(&mut e, step);
                    assert_eq!(e.journal().last_seq(), seq);
                    assert_eq!(e.inode(ino).unwrap(), &core);
                    assert_eq!(e.session(C1).unwrap().completed_requests[&100], done);
                    e = recover(e.checkpoint().unwrap(), config());
                }
            }
        }
    }
}

fn reconnect_xattr(cp: EngineCheckpoint, cfg: EngineConfig, ino: Ino) -> Engine<MemJournal> {
    let mut e = Engine::restore(cfg, MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    let keys = [key(&e, C1), key(&e, C2), key(&e, C3)];
    e.reconnect_chunk(
        keys[0],
        &[ReconnectCap {
            ino,
            cap_id: 77,
            wanted: XX,
            issued: XX | P,
            realm: ROOT,
            snap_follows: 0,
            fcntl_locks: vec![],
            flock_locks: vec![],
        }],
        &[],
        false,
    )
    .unwrap();
    for k in &keys[1..] {
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drive(&mut e, step);
    e
}
fn dirty_xattr(e: &Engine<MemJournal>, ino: Ino, len: usize) -> EngineInput {
    let cap = e.cap(ino, C1).unwrap();
    EngineInput::CapUpdate {
        key: key(e, C1),
        update: CapUpdate {
            op: CEPH_CAP_OP_FLUSH,
            ino,
            cap_id: cap.cap_id,
            seq: cap.last_sent,
            issue_seq: cap.last_issue,
            caps: cap.pending(),
            wanted: XX,
            dirty: XX,
            flush_tid: 99,
            xattrs: Some((2, XattrMap::from([(b"user.k".to_vec(), vec![42; len])]))),
            ..Default::default()
        },
    }
}

#[test]
fn owned_pressure_releases_reservations_and_allows_fitting_durable_retirement() {
    let mut fs = Fs::new();
    let mut req = mknod(10, "f");
    if let Op::Mknod { xattrs, .. } = &mut req.op {
        xattrs.insert(b"user.k".to_vec(), Vec::new());
    }
    let ino = fs.call_req(C1, req).target.unwrap().ino;
    let base = 256 + 4096 + 272 + 1 + 2 * (304 + 6);
    let mut cfg = config();
    cfg.retry_limits.max_bytes = base + 8192 + 1024;
    let mut e = reconnect_xattr(fs.e.checkpoint().unwrap(), cfg, ino);
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    drive(&mut e, step);
    let pending = EngineInput::Request {
        key: key(&e, C1),
        request: mknod(100, "waiting"),
    };
    assert_eq!(
        e.step(Some(pending), None).unwrap().state,
        StepState::NeedsAdmission
    );
    let control = dirty_xattr(&e, ino, 4096);
    let step = e.step(Some(control), permit()).unwrap();
    assert!(step.effects.is_empty());
    let step = commit(&mut e);
    drive(&mut e, step);
    assert_eq!(e.waiting_requests(), 1);
    assert_eq!(
        e.step(None, permit()).unwrap().state,
        StepState::WaitingInput
    );
    let seq = e.journal().last_seq();
    for _ in 0..3 {
        let step = e.step(Some(EngineInput::Tick), permit()).unwrap();
        assert_eq!(step.state, StepState::WaitingInput);
        assert_eq!(step.unused_permit, permit());
        assert!(step.effects.is_empty());
    }
    let mut floor = Request::new(
        300,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    floor.oldest_client_tid = 11;
    let step = send(&mut e, C1, floor);
    drive(&mut e, step);
    assert_eq!(
        e.step(None, permit()).unwrap().state,
        StepState::WaitingInput
    );
    assert!(e.session(C1).unwrap().completed_requests.contains_key(&10));
    assert_eq!(e.journal().last_seq(), seq);
    let mut free = Request::new(
        301,
        Op::Setattr {
            path: FilePath::ino(ROOT),
            args: SetattrArgs::default(),
        },
    );
    free.oldest_client_tid = 11;
    let step = send(&mut e, C1, free);
    assert!(e.session(C1).unwrap().completed_requests.contains_key(&10));
    let effects = drive(&mut e, step);
    assert!(reply_for(&effects, C1, 100).is_some());
    assert!(reply_for(&effects, C1, 301).is_some());
    assert!(!e.session(C1).unwrap().completed_requests.contains_key(&10));
    assert!(e.session(C1).unwrap().completed_requests.contains_key(&100));
    assert!(!e.is_failed());
}

#[test]
fn default_byte_boundary_fails_protected_control_without_using_a_held_watermark() {
    use std::mem::size_of;
    let mut fs = Fs::new();
    let mut req = mknod(10, "f");
    if let Op::Mknod { xattrs, .. } = &mut req.op {
        xattrs.insert(b"user.k".to_vec(), Vec::new());
    }
    let ino = fs.call_req(C1, req).target.unwrap().ino;
    fs.tid = 1000;
    for i in 0..31 {
        fs.mkfile(C1, ROOT, &format!("a{i:02}")).unwrap();
    }
    let mut cp = fs.e.checkpoint().unwrap();
    drop(fs);
    for n in cp
        .inodes
        .iter_mut()
        .filter(|n| n.core.ino != ROOT && n.core.ino != ino)
    {
        n.core
            .xattrs
            .insert(b"user.k".to_vec(), vec![0; 1024 * 1024]);
    }
    let entry = 256 + size_of::<(Vec<u8>, Vec<u8>)>();
    let index = 256 + size_of::<(Ino, usize)>();
    let base: usize = cp
        .inodes
        .iter()
        .filter(|n| n.core.ino != ROOT)
        .map(|n| {
            assert!(
                n.core.old_layouts.is_empty()
                    && n.core.client_ranges.is_empty()
                    && n.core.layout.pool_ns.is_empty()
            );
            256 + 4096
                + index
                + n.parent.as_ref().unwrap().1.len()
                + 2 * n
                    .core
                    .xattrs
                    .iter()
                    .map(|(k, v)| entry + k.len() + v.len())
                    .sum::<usize>()
        })
        .sum();
    let desired = 64 * 1024 * 1024 - 1024;
    let padding = (desired - base) / 2 - (entry + 1);
    assert_eq!((desired - base) % 2, 0);
    cp.inodes
        .iter_mut()
        .find(|n| n.core.ino != ROOT && n.core.ino != ino)
        .unwrap()
        .core
        .xattrs
        .insert(b"x".to_vec(), vec![0; padding]);
    let mut e = reconnect_xattr(cp.clone(), config(), ino);
    let mut held = mknod(5000, "retire-later");
    held.oldest_client_tid = 5000;
    let held = match e
        .step(
            Some(EngineInput::Request {
                key: key(&e, C1),
                request: held,
            }),
            permit(),
        )
        .unwrap_err()
    {
        StepError::Busy {
            input: Some(input),
            permit: p,
        } => {
            assert_eq!(p, permit());
            input
        }
        other => panic!("unexpected pre-release request result: {other}"),
    };
    let before = e.inode(ino).unwrap().clone();
    let cap = e.cap(ino, C1).unwrap().clone();
    let control = dirty_xattr(&e, ino, 4096);
    let seq = e.journal().last_seq();
    let returned = match e.step(Some(control.clone()), permit()).unwrap_err() {
        StepError::ControlCapacity {
            input: Some(input),
            permit: p,
            required_completions,
            required_bytes,
            required_transient_bytes,
        } => {
            assert_input_eq(&input, &control);
            assert_eq!(p, permit());
            assert_eq!(required_completions, Some(32));
            assert_eq!(required_bytes, Some(desired + 8192));
            assert!(required_transient_bytes.unwrap() < 8 * 1024 * 1024);
            input
        }
        other => panic!("expected terminal base-byte pressure, got {other}"),
    };
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert_eq!(e.journal().last_seq(), seq);
    assert_eq!(e.submitted_seq(), seq);
    assert_eq!(e.durable_seq(), seq);
    assert_eq!(e.inode(ino).unwrap(), &before);
    assert_eq!(e.cap(ino, C1).unwrap(), &cap);
    assert_eq!(e.session(C1).unwrap().completed_requests.len(), 32);
    assert!(e.session(C1).unwrap().completed_flushes.is_empty());
    assert_eq!(e.session(C1).unwrap().durable_oldest_client_tid, 0);
    assert!(e.pending_jobs().is_empty());
    assert!(matches!(
        e.step(Some(held), permit()),
        Err(StepError::ControlCapacity {
            input: Some(EngineInput::Request { .. }),
            ..
        })
    ));
    e.journal_event(JournalEvent::Failed(JournalError::Corrupt));
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert_eq!(e.step(None, permit()).unwrap().state, StepState::Failed);
    let mut cfg = config();
    cfg.retry_limits.max_bytes += 8192;
    let mut replacement = reconnect_xattr(cp, cfg, ino);
    let step = replacement.step(Some(returned.clone()), permit()).unwrap();
    assert!(step.effects.is_empty());
    let step = commit(&mut replacement);
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK && m.flush_tid == 99)
    );
    drive(&mut replacement, step);
    let tail = replacement.journal().last_seq();
    let step = replacement.step(Some(returned), permit()).unwrap();
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK && m.flush_tid == 99)
    );
    assert_eq!(replacement.journal().last_seq(), tail);
    assert_eq!(replacement.session(C1).unwrap().completed_flushes.len(), 1);
}

#[test]
fn captured_retry_survives_canonical_retirement_and_a_separate_range_commit() {
    let mut fs = Fs::new();
    let req = Request::new(
        100,
        Op::Create {
            path: FilePath::child(ROOT, "f"),
            flags: CEPH_O_CREAT | CEPH_O_EXCL | CEPH_O_RDWR,
            mode: 0o644,
            layout: LayoutOverride::default(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let ino = fs.call_req(C1, req.clone()).target.unwrap().ino;
    fs.release(C1, ino);
    fs.lookup(C2, ROOT, "f").unwrap();
    fs.e.journal_mut().set_auto_commit(false);
    assert!(fs.e.inode(ino).unwrap().client_ranges.is_empty());
    let input = EngineInput::Request {
        key: key(&fs.e, C1),
        request: req,
    };
    assert_eq!(
        fs.e.step(Some(input), None).unwrap().state,
        StepState::NeedsAdmission
    );
    let cap = fs.e.cap(ROOT, C2).unwrap();
    let items = vec![CapReleaseItem {
        ino: ROOT,
        cap_id: cap.cap_id,
        mseq: cap.mseq,
        issue_seq: cap.last_issue,
    }];
    fs.e.step(
        Some(EngineInput::CapRelease {
            key: key(&fs.e, C2),
            items,
        }),
        permit(),
    )
    .unwrap();
    let mut retirement = Request::new(
        200,
        Op::Setattr {
            path: FilePath::ino(ROOT),
            args: SetattrArgs::default(),
        },
    );
    retirement.oldest_client_tid = 101;
    let step = send(&mut fs.e, C1, retirement);
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    let mut step = commit(&mut fs.e);
    assert!(
        !fs.e
            .session(C1)
            .unwrap()
            .completed_requests
            .contains_key(&100)
    );
    for _ in 0..64 {
        if matches!(step.state, StepState::WaitingCommit(_)) {
            break;
        }
        step = fs.e.step(None, permit()).unwrap();
    }
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    let record = &fs.e.journal().records().last().unwrap().1;
    assert_eq!(record.kind, RecordKind::ClientRanges);
    assert!(record.completed_request.is_none() && record.allocated.is_empty());
    assert!(
        record
            .inodes
            .iter()
            .any(|c| c.ino == ino && c.client_ranges.contains_key(&C1))
    );
    assert!(reply_for(&step.effects, C1, 100).is_none());
    let step = commit(&mut fs.e);
    let r = reply_for(&step.effects, C1, 100).unwrap();
    assert_eq!((r.op, r.result), (CEPH_MDS_OP_CREATE, 0));
    assert_eq!(r.target.as_ref().unwrap().ino, ino);
    assert!(matches!(r.extra,ReplyExtra::Create{ino:i,..} if i==ino));
    assert!(
        !fs.e
            .session(C1)
            .unwrap()
            .completed_requests
            .contains_key(&100)
    );
}

#[test]
fn invalid_retry_identities_preserve_inputs_permits_caps_and_watermarks() {
    let mut fs = Fs::new();
    fs.e.journal_mut().set_auto_commit(false);
    let request = mknod(100, "one");
    let input = EngineInput::Request {
        key: key(&fs.e, C1),
        request: request.clone(),
    };
    fs.e.step(Some(input), None).unwrap();
    let mut wrong = Request::new(
        100,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    wrong.oldest_client_tid = 999;
    for phase in 0..2 {
        let input = EngineInput::Request {
            key: key(&fs.e, C1),
            request: wrong.clone(),
        };
        match fs.e.step(Some(input.clone()), permit()).unwrap_err() {
            StepError::InvalidRetryIdentity {
                input: Some(returned),
                permit: p,
            } => {
                assert_input_eq(&returned, &input);
                assert_eq!(p, permit());
            }
            other => panic!("wrong identity classification: {other}"),
        }
        assert_eq!(fs.e.session(C1).unwrap().oldest_client_tid, 0);
        if phase == 0 {
            fs.e.step(None, permit()).unwrap();
        }
    }
    let seq = fs.e.journal().last_seq();
    let step = commit(&mut fs.e);
    drive(&mut fs.e, step);
    let mut floor = Request::new(
        200,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    floor.oldest_client_tid = 150;
    let step = send(&mut fs.e, C1, floor);
    drive(&mut fs.e, step);
    let mut stale = mknod(99, "stale");
    stale.oldest_client_tid = 999;
    assert!(matches!(
        fs.e.step(
            Some(EngineInput::Request {
                key: key(&fs.e, C1),
                request: stale
            }),
            permit()
        ),
        Err(StepError::InvalidRetryIdentity { .. })
    ));
    assert_eq!(fs.e.journal().last_seq(), seq);
    assert!(
        fs.e.session(C1)
            .unwrap()
            .completed_requests
            .contains_key(&100)
    );
    let step = send(&mut fs.e, C1, request);
    assert_eq!(reply_for(&step.effects, C1, 100).unwrap().result, 0);
    assert_eq!(fs.e.session(C1).unwrap().oldest_client_tid, 150);
}

#[test]
fn legacy_clean_export_preserves_every_logical_field_and_cannot_enable_service() {
    let mut fs = Fs::new();
    fs.mkfile(C1, ROOT, "file").unwrap();
    fs.symlink(C2, ROOT, "link", "target").unwrap();
    for c in [C1, C2, C3] {
        fs.e.close_session(c, false);
    }
    let mut cp = fs.e.checkpoint().unwrap();
    assert!(cp.sessions.is_empty());
    cp.recovery_semantics = RecoverySemantics::Legacy2;
    let mut e = Engine::restore(
        config(),
        MemJournal::at_seq(cp.covered_seq, false),
        cp.clone(),
    )
    .unwrap();
    let exported = e.checkpoint_for_retry_upgrade().unwrap();
    let mut expected = cp.clone();
    expected.recovery_semantics = RecoverySemantics::RetainedTargets3;
    assert_eq!(exported, expected);
    assert_eq!(e.checkpoint().unwrap(), cp);
    assert!(matches!(
        e.step(Some(EngineInput::RecoveryControlsDrained), permit()),
        Err(StepError::UpgradeRequired { .. })
    ));
    assert!(e.open_session(C1).is_empty());
    assert!(e.session(C1).is_none());
    assert!(e.reconnect_done().is_empty());
    assert!(matches!(
        e.finish_reconnect(&[]),
        Err(RestoreError::UpgradeRequired(_))
    ));
    let mut replacement = recover(exported, config());
    replacement.open_session(C1);
    let step = send(&mut replacement, C1, mknod(100, "after-upgrade"));
    drive(&mut replacement, step);
    let _ = recover(replacement.checkpoint().unwrap(), config());
}

#[test]
fn checked_state_rejects_missing_targets_and_purge_before_reference_retirement() {
    let mut fs = Fs::new();
    let ino = fs.mkfile(C1, ROOT, "file").unwrap();
    fs.unlink(C2, ROOT, "file").unwrap();
    let cp = fs.e.checkpoint().unwrap();
    let mut missing = cp.clone();
    missing.inodes.retain(|n| n.core.ino != ino);
    assert!(matches!(
        Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), missing),
        Err(RestoreError::Invalid(_))
    ));
    let mut missing_field = cp.clone();
    missing_field
        .sessions
        .iter_mut()
        .find(|s| s.key.client == C1)
        .unwrap()
        .completed_requests[0]
        .reply_target_ino = None;
    assert!(matches!(
        Engine::restore(
            config(),
            MemJournal::at_seq(cp.covered_seq, false),
            missing_field
        ),
        Err(RestoreError::Invalid(_))
    ));
    let mut e = Engine::restore(
        config(),
        MemJournal::at_seq(cp.covered_seq, false),
        cp.clone(),
    )
    .unwrap();
    let mut record = JournalRecord::new(RecordKind::Purge);
    record.purged.push(ino);
    record.lifecycle_intents.push(LifecycleIntent {
        id: cp.next_job_id,
        ino,
        kind: LifecycleKind::Purge {
            layouts: vec![e.inode(ino).unwrap().layout.clone()],
            max_size: 0,
        },
    });
    assert!(matches!(
        e.replay_record(cp.covered_seq + 1, &record),
        Err(RestoreError::Invalid(_))
    ));
    assert_eq!(e.checkpoint().unwrap(), cp);
}

#[test]
fn fixed_transient_allowance_is_independent_of_base_and_guarded_job_retirement() {
    let mut fs = Fs::new();
    let ino = fs.mkfile(C1, ROOT, "file").unwrap();
    let mut cp = fs.e.checkpoint().unwrap();
    let core = &mut cp
        .inodes
        .iter_mut()
        .find(|n| n.core.ino == ino)
        .unwrap()
        .core;
    core.xattrs
        .insert(b"user.big".to_vec(), vec![0; 4 * 1024 * 1024]);
    core.client_ranges.insert(C1, 4 * 1024 * 1024);
    cp.jobs.push(LifecycleIntent {
        id: cp.next_job_id,
        ino,
        kind: LifecycleKind::RecoverSize {
            layouts: vec![core.layout.clone()],
            max_size: 4 * 1024 * 1024,
            inode_version: core.version,
            truncate_seq: core.truncate_seq,
        },
    });
    cp.next_job_id += 1;
    let mut e = Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    let keys = [key(&e, C1), key(&e, C2), key(&e, C3)];
    for k in &keys {
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drive(&mut e, step);
    let job = e.pending_jobs()[0].clone();
    let invalid = EngineInput::LifecycleCompleted {
        id: job.id,
        result: LifecycleResult::Recovered {
            size: 8 * 1024 * 1024,
            mtime: Timestamp::ZERO,
        },
    };
    let step = e.step(Some(invalid), permit()).unwrap();
    assert!(step.effects.is_empty());
    assert!(!e.is_failed());
    let before = e.inode(ino).unwrap().clone();
    let tail = e.journal().last_seq();
    let input = EngineInput::LifecycleCompleted {
        id: job.id,
        result: LifecycleResult::Recovered {
            size: 17,
            mtime: Timestamp::ZERO,
        },
    };
    match e.step(Some(input.clone()), permit()).unwrap_err() {
        StepError::ControlCapacity {
            input: Some(returned),
            permit: p,
            required_bytes,
            required_transient_bytes,
            ..
        } => {
            assert_input_eq(&returned, &input);
            assert_eq!(p, permit());
            assert!(required_bytes.unwrap() < 64 * 1024 * 1024);
            assert!(required_transient_bytes.unwrap() > 8 * 1024 * 1024);
        }
        other => panic!("wrong additional-copy classification: {other}"),
    }
    assert_eq!(e.inode(ino).unwrap(), &before);
    assert_eq!(e.journal().last_seq(), tail);
    assert!(e.is_failed());
}

#[test]
fn owned_release_cursor_returns_its_unexecuted_suffix_on_terminal_capacity() {
    let mut fs = Fs::new();
    let ino = fs.mkfile(C1, ROOT, "file").unwrap();
    let mut cp = fs.e.checkpoint().unwrap();
    let core = &mut cp
        .inodes
        .iter_mut()
        .find(|n| n.core.ino == ino)
        .unwrap()
        .core;
    core.xattrs
        .insert(b"user.big".to_vec(), vec![0; 4 * 1024 * 1024]);
    core.client_ranges.insert(C1, 4 * 1024 * 1024);
    let mut e = Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    let keys = [key(&e, C1), key(&e, C2), key(&e, C3)];
    e.reconnect_chunk(
        keys[0],
        &[ReconnectCap {
            ino,
            cap_id: 77,
            wanted: FW,
            issued: FW | P,
            realm: ROOT,
            snap_follows: 0,
            fcntl_locks: vec![],
            flock_locks: vec![],
        }],
        &[],
        false,
    )
    .unwrap();
    for k in &keys[1..] {
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drive(&mut e, step);
    let cap = e.cap(ino, C1).unwrap().clone();
    let remaining = CapReleaseItem {
        ino,
        cap_id: cap.cap_id,
        mseq: cap.mseq,
        issue_seq: cap.last_issue,
    };
    let ignored = CapReleaseItem {
        ino: ino + 1,
        cap_id: 999,
        mseq: 0,
        issue_seq: 0,
    };
    let step = e
        .step(
            Some(EngineInput::CapRelease {
                key: keys[0],
                items: vec![ignored, remaining],
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(step.state, StepState::NeedsAdmission);
    assert_eq!(step.unused_permit, permit());
    let before = e.inode(ino).unwrap().clone();
    let tail = e.journal().last_seq();
    match e.step(None, permit()).unwrap_err() {
        StepError::ControlCapacity {
            input: Some(EngineInput::CapRelease { key, items }),
            permit: p,
            required_transient_bytes,
            ..
        } => {
            assert_eq!(key, keys[0]);
            assert_eq!(items, vec![remaining]);
            assert_eq!(p, permit());
            assert!(required_transient_bytes.unwrap() > 8 * 1024 * 1024);
        }
        other => panic!("control origin or remaining cursor lost: {other}"),
    }
    assert_eq!(e.inode(ino).unwrap(), &before);
    assert_eq!(e.cap(ino, C1).unwrap(), &cap);
    assert_eq!(e.journal().last_seq(), tail);
}

#[test]
fn a_full_completion_count_does_not_block_fitting_byte_only_control_growth() {
    let mut fs = Fs::new();
    let mut req = mknod(10, "f");
    if let Op::Mknod { xattrs, .. } = &mut req.op {
        xattrs.insert(b"user.k".to_vec(), Vec::new());
    }
    let ino = fs.call_req(C1, req).target.unwrap().ino;
    let mut cfg = config();
    cfg.retry_limits.max_completions = 1;
    let mut e = reconnect_xattr(fs.e.checkpoint().unwrap(), cfg.clone(), ino);
    let control = dirty_xattr(&e, ino, 4096);
    let step = e.step(Some(control), permit()).unwrap();
    assert!(step.effects.is_empty());
    let step = commit(&mut e);
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK)
    );
    drive(&mut e, step);
    assert_eq!(e.session(C1).unwrap().completed_requests.len(), 1);
    assert!(!e.is_failed());
    let cp = e.checkpoint().unwrap();
    let mut too_small = cfg.clone();
    too_small.retry_limits.max_bytes = 4096;
    assert!(matches!(
        Engine::restore(
            too_small,
            MemJournal::at_seq(cp.covered_seq, false),
            cp.clone()
        ),
        Err(RestoreError::Capacity)
    ));
    for limits in [
        RetryLimits {
            max_completions: 0,
            max_bytes: 1,
        },
        RetryLimits {
            max_completions: 1,
            max_bytes: 0,
        },
        RetryLimits {
            max_completions: usize::MAX,
            max_bytes: 1,
        },
        RetryLimits {
            max_completions: 1,
            max_bytes: usize::MAX,
        },
    ] {
        let mut bad = cfg.clone();
        bad.retry_limits = limits;
        assert!(matches!(
            Engine::restore(
                bad.clone(),
                MemJournal::at_seq(cp.covered_seq, false),
                cp.clone()
            ),
            Err(RestoreError::Capacity)
        ));
        let new = Engine::new(bad, MemJournal::new());
        assert_eq!(new.failure(), Some(JournalError::Capacity));
        assert_eq!(new.journal().last_seq(), 0);
    }
}

#[test]
fn default_completion_limit_keeps_the_extra_slot_transient_until_watermark_commit() {
    let mut fs = Fs::new();
    let ino = fs.mkfile(C1, ROOT, "f").unwrap();
    let mut cp = fs.e.checkpoint().unwrap();
    let session = cp.sessions.iter_mut().find(|s| s.key.client == C1).unwrap();
    let incarnation = session.key.incarnation;
    for tid in 2..=65_536 {
        session
            .completed_requests
            .push(engine::journal::CompletedRequest {
                client: C1,
                session_incarnation: incarnation,
                tid,
                op: CEPH_MDS_OP_SETATTR,
                result: 0,
                created_ino: None,
                oldest_client_tid: 0,
                reply_target_ino: None,
            });
    }
    let mut e = recover(cp, config());
    let unlock = Request::new(
        65_537,
        Op::SetFileLock {
            path: FilePath::ino(ino),
            lock: FileLockArgs {
                rule: CEPH_LOCK_FLOCK,
                ltype: CEPH_LOCK_UNLOCK,
                owner: 1,
                pid: 1,
                start: 0,
                length: 0,
                wait: false,
            },
        },
    );
    let before = e.journal().last_seq();
    let step = send(&mut e, C1, unlock);
    assert_eq!(reply_for(&step.effects, C1, 65_537).unwrap().result, 0);
    drive(&mut e, step);
    assert_eq!(e.journal().last_seq(), before);
    assert_eq!(e.session(C1).unwrap().completed_requests.len(), 65_536);
    let input = EngineInput::Request {
        key: key(&e, C1),
        request: mknod(65_537, "blocked"),
    };
    assert!(matches!(
        e.step(Some(input), permit()),
        Err(StepError::RetryCapacity { .. })
    ));
    let mut fitting = mknod(65_538, "fitting");
    fitting.oldest_client_tid = 65_536;
    let step = send(&mut e, C1, fitting);
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(e.session(C1).unwrap().completed_requests.len(), 65_536);
    assert!(e.inode(ino).is_some());
    let step = commit(&mut e);
    assert_eq!(e.session(C1).unwrap().completed_requests.len(), 2);
    drive(&mut e, step);
    assert_eq!(e.session(C1).unwrap().durable_oldest_client_tid, 65_536);
    assert!(!e.is_failed());
}

#[test]
fn killed_retained_stray_journals_recovery_before_terminal_history_retirement() {
    let mut fs = Fs::new();
    let ino = fs.create(C1, ROOT, "file", CEPH_O_RDWR).unwrap();
    fs.unlink(C2, ROOT, "file").unwrap();
    fs.e.journal_mut().set_auto_commit(false);
    let k = key(&fs.e, C1);
    let mut step =
        fs.e.step(
            Some(EngineInput::CloseSession { key: k, kill: true }),
            permit(),
        )
        .unwrap();
    for _ in 0..32 {
        if matches!(step.state, StepState::WaitingCommit(_)) {
            break;
        }
        step = fs.e.step(None, permit()).unwrap();
    }
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    let (range_seq, record) = fs.e.journal().records().last().unwrap();
    let range_seq = *range_seq;
    assert_eq!(record.kind, RecordKind::ClientRanges);
    assert!(record.purged.is_empty());
    assert!(
        record
            .lifecycle_intents
            .iter()
            .any(|j| j.ino == ino && matches!(j.kind, LifecycleKind::RecoverSize { .. }))
    );
    assert_eq!(fs.e.session(C1).unwrap().completed_requests.len(), 1);
    assert!(fs.e.inode(ino).is_some());
    let mut step = commit(&mut fs.e);
    for _ in 0..32 {
        if matches!(step.state, StepState::WaitingCommit(_)) {
            break;
        }
        step = fs.e.step(None, permit()).unwrap();
    }
    let (close_seq, record) = fs.e.journal().records().last().unwrap();
    let close_seq = *close_seq;
    assert!(close_seq > range_seq);
    assert_eq!(record.closed_sessions, vec![k]);
    assert!(record.purged.is_empty());
    assert_eq!(fs.e.session(C1).unwrap().completed_requests.len(), 1);
    let step = commit(&mut fs.e);
    assert!(fs.e.session(C1).is_none());
    assert!(fs.e.inode(ino).is_some());
    drive(&mut fs.e, step);
    assert!(fs.e.inode(ino).is_none());
    assert!(
        fs.e.journal()
            .records()
            .iter()
            .any(|(seq, record)| *seq > close_seq && record.purged.contains(&ino))
    );
}
