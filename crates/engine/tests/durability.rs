//! Production permit, crash, checkpoint and lifecycle contracts.
mod common;
use common::*;
use engine::consts::*;
use engine::journal::{JournalRecord, RecordKind};
use engine::*;

const KEY: SessionKey = SessionKey {
    client: C1,
    incarnation: 1,
};
fn permit() -> Option<RecordPermit> {
    Some(RecordPermit {
        max_record_bytes: 1 << 20,
    })
}
fn create(name: &str, tid: u64) -> EngineInput {
    EngineInput::Request {
        key: KEY,
        request: Request::new(
            tid,
            Op::Create {
                path: FilePath::child(ROOT, name),
                flags: CEPH_O_CREAT | CEPH_O_RDWR,
                mode: 0o644,
                layout: LayoutOverride::default(),
                xattrs: XattrMap::new(),
                ino: 0,
            },
        ),
    }
}
fn engine() -> Engine<MemJournal> {
    let mut e = Engine::new(config(), MemJournal::manual());
    e.journal_mut().commit_all();
    e.journal_event(JournalEvent::Committed(1));
    e.install_session_key(KEY).unwrap();
    e
}
fn commit(e: &mut Engine<MemJournal>) -> EngineStep {
    e.journal_mut().commit_all();
    e.journal_event(JournalEvent::Committed(e.journal().last_seq()))
}
fn drain(e: &mut Engine<MemJournal>) {
    for _ in 0..100 {
        if e.is_quiescent() {
            return;
        }
        if e.uncommitted_records() != 0 {
            commit(e);
        } else {
            e.step(None, permit()).unwrap();
        }
    }
    panic!("drain did not finish");
}
fn created(e: &mut Engine<MemJournal>, name: &str, tid: u64) -> Ino {
    let s = e.step(Some(create(name, tid)), permit()).unwrap();
    assert!(matches!(s.state, StepState::WaitingCommit(_)));
    let s = commit(e);
    let ino = reply_for(&s.effects, C1, tid)
        .unwrap()
        .target
        .as_ref()
        .unwrap()
        .ino;
    drain(e);
    ino
}

#[test]
fn permit_precedes_mutation_and_completion_never_submits() {
    let mut e = engine();
    let base = e.journal().last_seq();
    let s = e.step(Some(create("f", 1)), None).unwrap();
    assert_eq!(s.state, StepState::NeedsAdmission);
    assert_eq!(e.lookup_name(ROOT, b"f"), None);
    assert_eq!(e.journal().last_seq(), base);
    let s = e.step(None, permit()).unwrap();
    assert_eq!(e.journal().last_seq(), base + 1);
    assert!(s.unused_permit.is_none());
    assert!(s.effects.is_empty());
    let rejected = e.step(Some(create("other", 2)), permit()).unwrap_err();
    assert!(matches!(
        rejected,
        StepError::Busy {
            input: Some(_),
            permit: Some(_)
        }
    ));
    let s = commit(&mut e);
    assert!(reply_for(&s.effects, C1, 1).is_some());
    assert_eq!(e.journal().last_seq(), base + 1);
    assert!(matches!(s.state, StepState::NeedsAdmission));
    drain(&mut e);
    assert!(e.is_quiescent());
}

#[derive(Debug)]
struct Failing {
    inner: MemJournal,
    error: Option<JournalError>,
}
impl Journal for Failing {
    fn submit(&mut self, r: JournalRecord) -> Result<u64, JournalError> {
        if let Some(e) = self.error {
            Err(e)
        } else {
            self.inner.submit(r)
        }
    }
    fn committed_seq(&self) -> u64 {
        self.inner.committed_seq()
    }
    fn flush(&mut self) {
        self.inner.flush();
    }
}
#[test]
fn failure_after_mutation_is_terminal_and_late_commit_is_silent() {
    for error in [
        JournalError::Capacity,
        JournalError::Fenced,
        JournalError::Unavailable,
        JournalError::Corrupt,
        JournalError::RecordTooLarge,
    ] {
        let mut e = Engine::new(
            config(),
            Failing {
                inner: MemJournal::new(),
                error: None,
            },
        );
        e.install_session_key(KEY).unwrap();
        e.journal_mut().error = Some(error);
        let s = e.step(Some(create("f", 1)), permit()).unwrap();
        assert_eq!(s.state, StepState::Failed);
        assert!(s.effects.is_empty());
        assert!(e.is_failed());
        assert_eq!(e.failure(), Some(error));
        assert!(
            e.journal_event(JournalEvent::Committed(100))
                .effects
                .is_empty()
        );
        assert!(
            e.handle_request(
                C1,
                Request::new(
                    2,
                    Op::Getattr {
                        path: FilePath::ino(ROOT),
                        mask: 0
                    }
                )
            )
            .is_empty()
        );
        assert!(e.pending_jobs().is_empty());
        assert_eq!(e.checkpoint(), Err(CheckpointError::Failed));
    }
}
#[test]
fn undersized_permit_fails_closed_and_invalid_prefix_fences() {
    let mut e = engine();
    let s = e
        .step(
            Some(create("f", 1)),
            Some(RecordPermit {
                max_record_bytes: 1,
            }),
        )
        .unwrap();
    assert_eq!(s.state, StepState::Failed);
    assert!(s.effects.is_empty());
    let mut e = engine();
    assert_eq!(
        e.journal_event(JournalEvent::Committed(99)).state,
        StepState::Failed
    );
}

#[test]
fn materialized_checkpoint_survives_all_history_removal() {
    let mut fs = Fs::new();
    let sv = fs.mkdir(C1, ROOT, "sv").unwrap();
    fs.setxattr(C1, sv, "ceph.dir.subvolume", b"1", 0).unwrap();
    let f = fs.create(C1, sv, "f", CEPH_O_RDWR).unwrap();
    fs.link(C1, sv, "alias", f).unwrap();
    let g = fs.create(C1, sv, "stray", CEPH_O_RDWR).unwrap();
    fs.unlink(C1, sv, "stray").unwrap();
    let mut checkpoint = fs.e.checkpoint().unwrap();
    // Exercise a watermark with no surviving entries.
    checkpoint.sessions.iter_mut().for_each(|s| {
        if s.key.client == C2 {
            s.completed_requests.clear();
            s.oldest_client_tid = 999;
            s.oldest_flush_tid = 123;
        } else if s.key.client == C1 {
            s.completed_flushes = vec![9, 10];
            s.oldest_flush_tid = 9;
        }
    });
    // A delegated lease survives even when every earlier WAL key is gone.
    let free = &mut checkpoint.ino_alloc.free[0];
    let delegated = InoRange::new(free.start, 3);
    free.start += 3;
    free.len -= 3;
    checkpoint.ino_alloc.delegated.push((C1, delegated));
    let seq = checkpoint.covered_seq;
    let e = Engine::restore(config(), MemJournal::at_seq(seq, false), checkpoint.clone()).unwrap();
    assert_eq!(e.checkpoint().unwrap(), checkpoint);
    assert_eq!(
        e.inode(g).unwrap().stray_auth_path.as_deref(),
        Some(b"/sv/stray".as_slice())
    );
    assert_eq!(e.inode(g).unwrap().stable_subvolume_id, sv);
    assert_eq!(e.parent_of(f), Some((sv, b"f".to_vec())));
    assert!(e.pending_jobs().is_empty());
    let mut e = e;
    let key = checkpoint
        .sessions
        .iter()
        .find(|s| s.key.client == C1)
        .unwrap()
        .key;
    e.install_session_key(key).unwrap();
    assert_eq!(e.session(C2).unwrap().oldest_client_tid, 999);
    assert_eq!(e.session(C2).unwrap().oldest_flush_tid, 123);
    assert!(!e.session(C1).unwrap().completed_requests.is_empty());
    assert_eq!(
        e.session(C1)
            .unwrap()
            .completed_flushes
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![9, 10]
    );
    assert_eq!(e.ino_allocator().delegated_to(C1), &[delegated]);
    assert!(
        e.install_session_key(SessionKey {
            incarnation: key.incarnation + 1,
            ..key
        })
        .is_err()
    );
    assert!(e.journal().records().is_empty());
}

#[test]
fn checkpoint_then_unlink_replay_preserves_path_and_stable_identity() {
    let mut fs = Fs::new();
    let sv = fs.mkdir(C1, ROOT, "sv").unwrap();
    fs.setxattr(C1, sv, "ceph.dir.subvolume", b"1", 0).unwrap();
    let f = fs.create(C1, sv, "f", CEPH_O_RDWR).unwrap();
    let checkpoint = fs.e.checkpoint().unwrap();
    let seq = checkpoint.covered_seq;
    fs.unlink(C1, sv, "f").unwrap();
    let record = fs.e.journal().records().last().unwrap().1.clone();
    let mut e = Engine::restore(config(), MemJournal::at_seq(seq, false), checkpoint).unwrap();
    e.replay_record(seq + 1, &record).unwrap();
    assert_eq!(
        e.inode(f).unwrap().stray_auth_path.as_deref(),
        Some(b"/sv/f".as_slice())
    );
    assert_eq!(e.inode(f).unwrap().stable_subvolume_id, sv);
    assert_eq!(e.parent_of(f), None);
    *e.journal_mut() = MemJournal::at_seq(seq + 1, false);
    assert_eq!(e.checkpoint().unwrap().covered_seq, seq + 1);
    assert!(matches!(
        e.replay_record(seq + 1, &record),
        Err(RestoreError::Sequence { .. })
    ));
}

#[test]
fn malformed_restore_and_replay_are_atomic() {
    let mut e = engine();
    created(&mut e, "f", 1);
    let original = e.checkpoint().unwrap();
    let mut cases = Vec::new();
    let mut c = original.clone();
    c.inodes.push(c.inodes[0].clone());
    cases.push(c);
    let mut c = original.clone();
    c.inodes.retain(|i| i.core.ino != ROOT);
    cases.push(c);
    let mut c = original.clone();
    c.ino_alloc.free.push(c.ino_alloc.free[0]);
    cases.push(c);
    let mut c = original.clone();
    c.inodes[0].entries[0].1.ino = 99999;
    cases.push(c);
    let mut c = original.clone();
    c.sessions[0].key.incarnation = 0;
    cases.push(c);
    let mut c = original.clone();
    let file = c
        .inodes
        .iter()
        .find(|i| !i.core.is_dir())
        .unwrap()
        .core
        .clone();
    c.jobs.push(LifecycleIntent {
        id: c.next_job_id,
        ino: file.ino,
        kind: LifecycleKind::Truncate {
            layout: file.layout,
            from: 100,
            size: 10,
            truncate_seq: u32::MAX,
        },
    });
    c.next_job_id += 1;
    cases.push(c);
    for c in cases {
        assert!(Engine::restore(config(), MemJournal::new(), c).is_err());
    }
    let mut bad = JournalRecord::new(RecordKind::Unlink);
    bad.dentries.push(engine::journal::DentryUpdate {
        dir: 98765,
        name: b"x".to_vec(),
        link: None,
    });
    assert!(e.replay_record(original.covered_seq + 1, &bad).is_err());
    assert_eq!(e.checkpoint().unwrap(), original);
}

#[test]
fn close_is_one_record_per_permit_and_terminal_receipt_survives_restart() {
    let mut e = engine();
    created(&mut e, "a", 1);
    created(&mut e, "b", 2);
    let mut input = Some(EngineInput::CloseSession {
        key: KEY,
        kill: true,
    });
    let mut receipt = None;
    for _ in 0..30 {
        let base = e.journal().last_seq();
        let s = e.step(input.take(), permit()).unwrap();
        assert!(
            e.install_session_key(SessionKey {
                incarnation: 2,
                ..KEY
            })
            .is_err(),
            "pending cleanup cannot reopen"
        );
        assert!(e.journal().last_seq() <= base + 1);
        assert!(s.notices.is_empty());
        if e.uncommitted_records() != 0 {
            let before = e.journal().last_seq();
            let s = commit(&mut e);
            assert_eq!(e.journal().last_seq(), before);
            for n in s.notices {
                let EngineNotice::SessionCleanupCommitted { key, seq } = n;
                assert_eq!(key, KEY);
                receipt = Some(seq);
            }
        }
        if receipt.is_some() {
            break;
        }
    }
    let seq = receipt.expect("terminal cleanup receipt");
    assert_eq!(e.session_cleanup_seq(KEY), Some(seq));
    drain(&mut e);
    assert!(e.install_session_key(KEY).is_err());
    let checkpoint = e.checkpoint().unwrap();
    let mut restored = Engine::restore(
        config(),
        MemJournal::at_seq(checkpoint.covered_seq, false),
        checkpoint,
    )
    .unwrap();
    assert_eq!(restored.session_cleanup_seq(KEY), Some(seq));
    assert!(restored.install_session_key(KEY).is_err());
    restored
        .install_session_key(SessionKey {
            incarnation: 2,
            ..KEY
        })
        .unwrap();
    assert!(restored.session(C1).unwrap().completed_requests.is_empty());
}

#[test]
fn lost_writer_range_and_job_survive_crash_until_completion_commit() {
    let mut e = engine();
    let f = created(&mut e, "f", 1);
    e.step(
        Some(EngineInput::CloseSession {
            key: KEY,
            kill: true,
        }),
        permit(),
    )
    .unwrap();
    while e.uncommitted_records() == 0 {
        e.step(None, permit()).unwrap();
    }
    assert_eq!(e.inode(f).unwrap().client_ranges[&C1], 4 << 20);
    assert!(e.pending_jobs().is_empty());
    let s = commit(&mut e);
    let job = s
        .effects
        .iter()
        .find_map(|e| {
            if let Effect::Lifecycle(j) = e {
                Some(j.clone())
            } else {
                None
            }
        })
        .unwrap();
    for _ in 0..10 {
        if e.session_cleanup_seq(KEY).is_some() {
            break;
        }
        e.step(None, permit()).unwrap();
        if e.uncommitted_records() > 0 {
            commit(&mut e);
        }
    }
    drain(&mut e);
    let checkpoint = e.checkpoint().unwrap();
    let mut restored = Engine::restore(
        config(),
        MemJournal::at_seq(checkpoint.covered_seq, false),
        checkpoint,
    )
    .unwrap();
    assert!(restored.pending_jobs().is_empty());
    restored.finish_reconnect(&[]).unwrap();
    restored
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    assert_eq!(restored.pending_jobs(), vec![job.clone()]);
    let base = restored.journal().last_seq();
    let s = restored
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: job.id,
                result: LifecycleResult::Recovered {
                    size: 123,
                    mtime: Timestamp::new(9999, 0),
                },
            }),
            permit(),
        )
        .unwrap();
    assert!(s.effects.is_empty());
    assert_eq!(restored.journal().last_seq(), base + 1);
    // A crash before completion retains the original range/job in checkpoint.
    assert_eq!(restored.checkpoint(), Err(CheckpointError::Busy));
    commit(&mut restored);
    drain(&mut restored);
    assert!(restored.pending_jobs().is_empty());
    assert!(restored.inode(f).unwrap().client_ranges.is_empty());
    assert_eq!(restored.inode(f).unwrap().size, 123);
    let seq = restored.journal().last_seq();
    let s = restored
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: job.id,
                result: LifecycleResult::Recovered {
                    size: 999,
                    mtime: Timestamp::ZERO,
                },
            }),
            permit(),
        )
        .unwrap();
    assert!(s.unused_permit.is_some());
    assert_eq!(restored.journal().last_seq(), seq);
}

#[test]
fn cap_ack_resumes_parked_request_without_hidden_submission() {
    let mut e = engine();
    let f = created(&mut e, "f", 1);
    let key2 = SessionKey {
        client: C2,
        incarnation: 2,
    };
    e.install_session_key(key2).unwrap();
    let s = e
        .step(
            Some(EngineInput::Request {
                key: key2,
                request: Request::new(
                    2,
                    Op::Getattr {
                        path: FilePath::ino(f),
                        mask: CEPH_STAT_CAP_INODE_ALL,
                    },
                ),
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(s.state, StepState::WaitingInput);
    assert_eq!(
        e.step(None, None).unwrap().state,
        StepState::WaitingInput,
        "no admission spin while an ack is missing"
    );
    let rv = revokes(&s.effects).remove(0);
    let base = e.journal().last_seq();
    let s = e
        .step(
            Some(EngineInput::CapUpdate {
                key: KEY,
                update: CapUpdate {
                    op: CEPH_CAP_OP_UPDATE,
                    ino: f,
                    cap_id: rv.cap_id,
                    seq: rv.seq,
                    issue_seq: rv.issue_seq,
                    caps: rv.caps,
                    wanted: 0,
                    ..Default::default()
                },
            }),
            permit(),
        )
        .unwrap();
    // The ack can journal range removal, but cannot also run the parked read.
    assert!(e.journal().last_seq() <= base + 1);
    assert!(reply_for(&s.effects, C2, 2).is_none());
    if e.uncommitted_records() > 0 {
        commit(&mut e);
    }
    let mut answered = false;
    for _ in 0..10 {
        let s = e.step(None, permit()).unwrap();
        answered |= reply_for(&s.effects, C2, 2).is_some();
        if answered {
            break;
        }
    }
    assert!(answered);
}

#[test]
fn purge_intent_retains_all_pools_after_inode_retirement() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    fs.setxattr(C1, f, "ceph.file.layout.pool_namespace", b"old", 0)
        .unwrap();
    fs.setxattr(C1, f, "ceph.file.layout.pool", b"fast", 0)
        .unwrap();
    fs.setxattr(C1, f, "ceph.file.layout.pool_namespace", b"new", 0)
        .unwrap();
    fs.retire_completed(C1);
    fs.unlink(C1, ROOT, "f").unwrap();
    let cap = fs.e.cap(f, C1).unwrap().clone();
    let key = SessionKey {
        client: C1,
        incarnation: fs.e.session(C1).unwrap().epoch,
    };
    fs.e.journal_mut().set_auto_commit(false);
    let s =
        fs.e.step(
            Some(EngineInput::CapRelease {
                key,
                items: vec![CapReleaseItem {
                    ino: f,
                    cap_id: cap.cap_id,
                    mseq: cap.mseq,
                    issue_seq: cap.last_issue,
                }],
            }),
            permit(),
        )
        .unwrap();
    assert!(s.effects.iter().all(|e| !matches!(e, Effect::Lifecycle(_))));
    assert!(fs.e.inode(f).is_none());
    let s = commit(&mut fs.e);
    let job = s
        .effects
        .iter()
        .find_map(|e| {
            if let Effect::Lifecycle(j) = e {
                Some(j.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert!(
        matches!(&job.kind,LifecycleKind::Purge {layouts,max_size} if layouts[0].pool_id==3 && layouts[0].pool_ns=="new"
        && layouts.iter().any(|l|l.pool_id==2 && l.pool_ns=="old") && *max_size>=4<<20)
    );
    drain(&mut fs.e);
    let checkpoint = fs.e.checkpoint().unwrap();
    let mut restored = Engine::restore(
        config(),
        MemJournal::at_seq(checkpoint.covered_seq, false),
        checkpoint,
    )
    .unwrap();
    assert!(restored.pending_jobs().is_empty());
    let keys: Vec<_> = [C1, C2, C3]
        .into_iter()
        .map(|client| SessionKey {
            client,
            incarnation: restored.session(client).unwrap().epoch,
        })
        .collect();
    for key in &keys {
        restored.reconnect_chunk(*key, &[], &[], false).unwrap();
    }
    restored.finish_reconnect(&keys).unwrap();
    restored
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    assert_eq!(restored.pending_jobs(), vec![job.clone()]);
    restored
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: job.id,
                result: LifecycleResult::Done,
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(restored.lifecycle_intent(job.id), Some(&job));
    assert!(restored.pending_jobs().is_empty());
    commit(&mut restored);
    drain(&mut restored);
    assert!(restored.pending_jobs().is_empty());
}

#[test]
fn truncate_output_uses_its_own_durable_image() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    fs.setattr(
        C1,
        f,
        SetattrArgs {
            mask: CEPH_SETATTR_SIZE,
            size: 1000,
            ..Default::default()
        },
    )
    .unwrap();
    fs.e.journal_mut().set_auto_commit(false);
    let (tid, eff) = fs.send(
        C1,
        Op::Setattr {
            path: FilePath::ino(f),
            args: SetattrArgs {
                mask: CEPH_SETATTR_SIZE,
                size: 10,
                ..Default::default()
            },
        },
    );
    let mut all = eff.clone();
    all.extend(fs.settle(&eff));
    let first = fs.e.journal().last_seq();
    let cap = fs.e.cap(f, C1).unwrap().clone();
    fs.cap_update(
        C1,
        CapUpdate {
            op: CEPH_CAP_OP_FLUSH,
            ino: f,
            cap_id: cap.cap_id,
            seq: cap.last_sent,
            issue_seq: cap.last_issue,
            caps: cap.issued(),
            wanted: cap.wanted(),
            dirty: FW,
            size: 500,
            flush_tid: 999,
            ..Default::default()
        },
    );
    fs.e.journal_mut().commit_upto(first);
    let eff = fs.e.journal_committed();
    let st = reply_for(&eff, C1, tid).unwrap().target.as_ref().unwrap();
    assert_eq!(st.size, 10);
    for m in cap_msgs(&eff)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_TRUNC)
    {
        assert_eq!(m.inode.as_ref().unwrap().size, 10);
    }
    assert!(
        eff.iter()
            .any(|e| matches!(e,Effect::TruncateData(t) if t.size==10 && t.from==1000))
    );
}

#[test]
fn reconnect_chunk_validates_all_then_replaces_authenticated_locks() {
    let mut e = engine();
    let f = created(&mut e, "f", 1);
    let checkpoint = e.checkpoint().unwrap();
    let mut e = Engine::restore(
        config(),
        MemJournal::at_seq(checkpoint.covered_seq, false),
        checkpoint,
    )
    .unwrap();
    let lock = engine::filelock::FileLock {
        start: 1,
        length: 0,
        client: 999,
        owner: 1 << 63,
        pid: 2,
        ltype: CEPH_LOCK_EXCL,
    };
    let cap = ReconnectCap {
        ino: f,
        cap_id: 100,
        wanted: FW,
        issued: FW,
        realm: ROOT,
        snap_follows: 9,
        fcntl_locks: vec![lock],
        flock_locks: vec![lock],
    };
    let mut bad = cap.clone();
    bad.ino = 12345;
    assert!(
        e.reconnect_chunk(KEY, &[cap.clone(), bad], &[], true)
            .is_err()
    );
    assert!(e.cap(f, C1).is_none());
    assert!(
        !e.reconnect_chunk(KEY, std::slice::from_ref(&cap), &[], true)
            .unwrap()
            .complete
    );
    assert!(e.finish_reconnect(&[KEY]).is_err());
    assert_eq!(e.reconnect_cap_context(KEY, f), Some((ROOT, 9)));
    e.reconnect_chunk(KEY, std::slice::from_ref(&cap), &[], true)
        .unwrap();
    assert_eq!(e.file_locks(f, false).len(), 1);
    assert_eq!(e.file_locks(f, true).len(), 1);
    assert_eq!(e.file_locks(f, false)[0].client, C1);
    assert!(
        e.reconnect_chunk(
            KEY,
            &[],
            &[ReconnectRealm {
                ino: ROOT,
                seq: 0,
                parent: 0
            }],
            false
        )
        .unwrap()
        .complete
    );
    e.reset_reconnect(KEY).unwrap();
    assert!(e.cap(f, C1).is_none());
    assert!(e.file_locks(f, false).is_empty());
}

#[test]
fn cleanup_wal_receipt_replays_without_resurrecting_old_incarnation() {
    let mut e = engine();
    created(&mut e, "f", 1);
    let checkpoint = e.checkpoint().unwrap();
    let at = checkpoint.covered_seq;
    let mut input = Some(EngineInput::CloseSession {
        key: KEY,
        kill: false,
    });
    for _ in 0..30 {
        e.step(input.take(), permit()).unwrap();
        if e.uncommitted_records() > 0 {
            commit(&mut e);
        }
        if e.session_cleanup_seq(KEY).is_some() {
            break;
        }
    }
    let receipt = e.session_cleanup_seq(KEY).unwrap();
    let mut recovered =
        Engine::restore(config(), MemJournal::at_seq(at, false), checkpoint).unwrap();
    for (seq, r) in e.journal().records().iter().filter(|(seq, _)| *seq > at) {
        recovered.replay_record(*seq, r).unwrap();
    }
    assert_eq!(recovered.session_cleanup_seq(KEY), Some(receipt));
    assert!(recovered.install_session_key(KEY).is_err());
    recovered
        .install_session_key(SessionKey {
            incarnation: 2,
            ..KEY
        })
        .unwrap();
    assert!(matches!(
        recovered.step(Some(create("old", 88)), permit()),
        Err(StepError::InvalidSession { .. })
    ));
}

#[test]
fn asynchronous_failure_discards_pending_reply_and_data_work() {
    let mut e = engine();
    e.step(Some(create("f", 1)), permit()).unwrap();
    assert_eq!(
        e.journal_event(JournalEvent::Failed(JournalError::Fenced))
            .state,
        StepState::Failed
    );
    e.journal_mut().commit_all();
    let s = e.journal_event(JournalEvent::Committed(e.journal().last_seq()));
    assert!(s.effects.is_empty());
    assert!(s.notices.is_empty());
    assert_eq!(s.state, StepState::Failed);
    let s = e.step(Some(EngineInput::Tick), permit()).unwrap();
    assert_eq!(s.state, StepState::Failed);
    assert!(s.effects.is_empty());
    assert!(s.unused_permit.is_some());
}

#[test]
fn rename_over_target_wal_retains_binary_authorization_path() {
    let mut fs = Fs::new();
    let sv = fs.mkdir(C1, ROOT, "sv").unwrap();
    fs.setxattr(C1, sv, "ceph.dir.subvolume", b"1", 0).unwrap();
    let source = fs.create(C1, sv, "source", CEPH_O_RDWR).unwrap();
    let name = vec![b't', 0xff];
    let target = fs
        .call(
            C1,
            Op::Create {
                path: FilePath::child(sv, &name),
                flags: CEPH_O_CREAT | CEPH_O_RDWR,
                mode: 0o644,
                layout: LayoutOverride::default(),
                xattrs: XattrMap::new(),
                ino: 0,
            },
        )
        .target
        .unwrap()
        .ino;
    let checkpoint = fs.e.checkpoint().unwrap();
    let at = checkpoint.covered_seq;
    fs.call(
        C1,
        Op::Rename {
            src: FilePath::child(sv, "source"),
            dst: FilePath::child(sv, &name),
        },
    );
    let mut e = Engine::restore(config(), MemJournal::at_seq(at, false), checkpoint).unwrap();
    for (seq, r) in fs.e.journal().records().iter().filter(|(seq, _)| *seq > at) {
        e.replay_record(*seq, r).unwrap();
    }
    assert_eq!(e.lookup_name(sv, &name), Some(source));
    assert_eq!(
        e.inode(target).unwrap().stray_auth_path,
        Some(vec![b'/', b's', b'v', b'/', b't', 0xff])
    );
    assert_eq!(e.inode(target).unwrap().stable_subvolume_id, sv);
}

#[test]
fn auto_commit_still_stops_at_each_record_boundary() {
    let mut e = Engine::new(config(), MemJournal::new());
    e.install_session_key(KEY).unwrap();
    let base = e.journal().last_seq();
    let s = e.step(Some(create("f", 1)), permit()).unwrap();
    assert_eq!(e.journal().last_seq(), base + 1);
    assert!(s.effects.is_empty());
    assert_eq!(s.state, StepState::WaitingCommit(base + 1));
    let s = e.journal_event(JournalEvent::Committed(base + 1));
    assert!(reply_for(&s.effects, C1, 1).is_some());
    assert_eq!(e.journal().last_seq(), base + 1);
    drain(&mut e);
    let before = e.journal().last_seq();
    e.journal_event(JournalEvent::Committed(before));
    assert_eq!(e.journal().last_seq(), before);
}

#[test]
fn release_cursor_never_submits_two_records_from_one_permit() {
    let mut e = engine();
    let a = created(&mut e, "a", 1);
    let b = created(&mut e, "b", 2);
    let items = [a, b]
        .into_iter()
        .map(|ino| {
            let c = e.cap(ino, C1).unwrap();
            CapReleaseItem {
                ino,
                cap_id: c.cap_id,
                mseq: c.mseq,
                issue_seq: c.last_issue,
            }
        })
        .collect();
    let base = e.journal().last_seq();
    let s = e
        .step(Some(EngineInput::CapRelease { key: KEY, items }), permit())
        .unwrap();
    assert_eq!(e.journal().last_seq(), base + 1);
    assert!(s.unused_permit.is_none());
    commit(&mut e);
    assert_eq!(e.journal().last_seq(), base + 1);
    e.step(None, permit()).unwrap();
    assert_eq!(e.journal().last_seq(), base + 2);
    commit(&mut e);
    drain(&mut e);
    assert!(e.inode(a).unwrap().client_ranges.is_empty());
    assert!(e.inode(b).unwrap().client_ranges.is_empty());
}

#[test]
fn stale_probe_result_cannot_retire_or_change_metadata() {
    let mut e = engine();
    let f = created(&mut e, "f", 1);
    e.step(
        Some(EngineInput::CloseSession {
            key: KEY,
            kill: true,
        }),
        permit(),
    )
    .unwrap();
    while e.uncommitted_records() == 0 {
        e.step(None, permit()).unwrap();
    }
    commit(&mut e);
    let job = e.pending_jobs().into_iter().next().unwrap();
    let base = e.journal().last_seq();
    // A result beyond the retained write bound is invalid, not an empty file.
    let s = e.step(
        Some(EngineInput::LifecycleCompleted {
            id: job.id,
            result: LifecycleResult::Recovered {
                size: u64::MAX,
                mtime: Timestamp::ZERO,
            },
        }),
        permit(),
    );
    assert!(
        matches!(s, Err(StepError::Busy { .. })),
        "close continuation must drain first"
    );
    for _ in 0..20 {
        if e.session_cleanup_seq(KEY).is_some() {
            break;
        }
        e.step(None, permit()).unwrap();
        if e.uncommitted_records() > 0 {
            commit(&mut e);
        }
    }
    drain(&mut e);
    let checkpoint = e.checkpoint().unwrap();
    let mut bad = checkpoint.clone();
    let LifecycleKind::RecoverSize { inode_version, .. } = &mut bad.jobs[0].kind else {
        panic!()
    };
    *inode_version += 1;
    assert!(Engine::restore(config(), MemJournal::new(), bad).is_err());
    let before = e.journal().last_seq();
    let s = e
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: job.id,
                result: LifecycleResult::Recovered {
                    size: u64::MAX,
                    mtime: Timestamp::ZERO,
                },
            }),
            permit(),
        )
        .unwrap();
    assert!(s.unused_permit.is_some());
    assert_eq!(e.journal().last_seq(), before);
    assert!(before >= base);
    assert_eq!(e.inode(f).unwrap().size, 0);
    assert_eq!(e.pending_jobs(), vec![job]);
}

#[test]
fn production_readdir_limits_child_materialization_per_step() {
    let mut e = engine();
    for tid in 1..=70 {
        created(&mut e, &format!("f{tid}"), tid);
    }
    let s = e
        .step(
            Some(EngineInput::Request {
                key: KEY,
                request: Request::new(
                    100,
                    Op::Readdir {
                        path: FilePath::ino(ROOT),
                        frag: 0,
                        max_entries: 0,
                        max_bytes: 0,
                        last_name: None,
                        offset_hash: 0,
                        flags: 0,
                    },
                ),
            }),
            permit(),
        )
        .unwrap();
    let ReplyExtra::Readdir(rd) = &reply_for(&s.effects, C1, 100).unwrap().extra else {
        panic!()
    };
    assert_eq!(rd.entries.len(), 64);
    assert_eq!(rd.flags & CEPH_READDIR_FRAG_END, 0);
    assert!(s.unused_permit.is_some());
}

#[test]
fn recovery_recalls_remaining_writer_and_accepts_its_dirty_ack() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    fs.open(C2, f, CEPH_O_RDWR).unwrap();
    fs.settle(&fs.last.clone());
    assert_ne!(fs.issued(f, C2) & FW, 0);
    fs.e.journal_mut().set_auto_commit(false);
    let key1 = SessionKey {
        client: C1,
        incarnation: fs.e.session(C1).unwrap().epoch,
    };
    let key2 = SessionKey {
        client: C2,
        incarnation: fs.e.session(C2).unwrap().epoch,
    };
    let mut input = Some(EngineInput::CloseSession {
        key: key1,
        kill: true,
    });
    let mut revoke = None;
    for _ in 0..30 {
        let s = fs.e.step(input.take(), permit()).unwrap();
        for m in revokes(&s.effects) {
            if m.ino == f && m.client == C2 {
                revoke = Some(m);
            }
        }
        if fs.e.uncommitted_records() > 0 {
            let s = commit(&mut fs.e);
            for m in revokes(&s.effects) {
                if m.ino == f && m.client == C2 {
                    revoke = Some(m);
                }
            }
        }
        if fs.e.session_cleanup_seq(key1).is_some() {
            break;
        }
    }
    let rv = revoke.expect("remaining writer must stop before probe");
    assert!(fs.e.pending_jobs().is_empty());
    let cap = fs.e.cap(f, C2).unwrap().clone();
    let s =
        fs.e.step(
            Some(EngineInput::CapUpdate {
                key: key2,
                update: CapUpdate {
                    op: CEPH_CAP_OP_FLUSH,
                    ino: f,
                    cap_id: cap.cap_id,
                    seq: rv.seq,
                    issue_seq: rv.issue_seq,
                    caps: rv.caps,
                    wanted: rv.wanted,
                    dirty: FW,
                    flush_tid: 77,
                    size: 4096,
                    ..Default::default()
                },
            }),
            permit(),
        )
        .unwrap();
    assert!(s.effects.iter().all(|e| !matches!(e, Effect::Lifecycle(_))));
    let s = commit(&mut fs.e);
    assert!(
        s.effects
            .iter()
            .any(|e| matches!(e,Effect::Caps(m) if m.op==CEPH_CAP_OP_FLUSH_ACK && m.flush_tid==77))
    );
    let job = s
        .effects
        .iter()
        .find_map(|e| {
            if let Effect::Lifecycle(j) = e {
                Some(j.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert!(
        matches!(job.kind,LifecycleKind::RecoverSize {inode_version,..} if inode_version==fs.e.inode(f).unwrap().version)
    );
    fs.e.step(
        Some(EngineInput::LifecycleCompleted {
            id: job.id,
            result: LifecycleResult::Recovered {
                size: 4096,
                mtime: Timestamp::ZERO,
            },
        }),
        permit(),
    )
    .unwrap();
    commit(&mut fs.e);
    drain(&mut fs.e);
    assert!(fs.e.pending_jobs().is_empty());
}
