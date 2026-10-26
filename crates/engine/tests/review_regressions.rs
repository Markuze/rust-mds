//! Independent-review scenarios, exercised through production APIs.
mod common;
use common::*;
use engine::consts::*;
use engine::journal::{DentryLink, DentryUpdate, JournalRecord, RecordKind};
use engine::*;

fn permit() -> Option<RecordPermit> {
    Some(RecordPermit {
        max_record_bytes: 1 << 20,
    })
}
fn key(e: &Engine<MemJournal>, client: ClientId) -> SessionKey {
    SessionKey {
        client,
        incarnation: e.session(client).unwrap().epoch,
    }
}
fn commit(e: &mut Engine<MemJournal>) -> EngineStep {
    let before = e.journal().last_seq();
    e.journal_mut().commit_all();
    let s = e.journal_event(JournalEvent::Committed(before));
    assert_eq!(e.journal().last_seq(), before, "completion must not submit");
    s
}
fn close(e: &mut Engine<MemJournal>, key: SessionKey, kill: bool) -> Vec<Effect> {
    let mut input = Some(EngineInput::CloseSession { key, kill });
    let mut out = Vec::new();
    for _ in 0..64 {
        let before = e.journal().last_seq();
        let s = e.step(input.take(), permit()).unwrap();
        assert_ne!(
            s.state,
            StepState::WaitingInput,
            "cleanup must advertise its continuation or commit wait"
        );
        assert!(
            e.journal().last_seq() <= before + 1,
            "one record per permit"
        );
        out.extend(s.effects);
        if e.uncommitted_records() > 0 {
            out.extend(commit(e).effects);
        }
        if e.session_cleanup_seq(key).is_some() {
            return out;
        }
    }
    panic!("cleanup did not reach its terminal receipt");
}
fn drain(e: &mut Engine<MemJournal>) -> Vec<Effect> {
    let mut out = Vec::new();
    for _ in 0..64 {
        if e.is_quiescent() {
            return out;
        }
        let before = e.journal().last_seq();
        let s = if e.uncommitted_records() > 0 {
            commit(e)
        } else {
            e.step(None, permit()).unwrap()
        };
        assert!(e.journal().last_seq() <= before + 1);
        out.extend(s.effects);
        assert_ne!(
            s.state,
            StepState::WaitingInput,
            "driver must not poll a lost wakeup"
        );
    }
    panic!("continuation did not drain");
}
fn restore(cp: EngineCheckpoint) -> Engine<MemJournal> {
    Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap()
}
fn read_dir(ino: Ino) -> Op {
    Op::Readdir {
        path: FilePath::ino(ino),
        frag: 0,
        max_entries: 0,
        max_bytes: 0,
        last_name: None,
        offset_hash: 0,
        flags: 0,
    }
}

#[test]
fn resumed_pure_ack_preserves_the_immediate_admission_wakeup() {
    for deferred in [false, true] {
        let mut fs = Fs::new();
        let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
        fs.e.journal_mut().set_auto_commit(false);
        let c1 = key(&fs.e, C1);
        let c2 = key(&fs.e, C2);
        let s =
            fs.e.step(
                Some(EngineInput::Request {
                    key: c2,
                    request: Request::new(
                        100,
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
        let rv = revokes(&s.effects).remove(0);
        let wanted = fs.e.cap(f, C1).unwrap().wanted();
        let base = fs.e.journal().last_seq();
        let ack = EngineInput::CapUpdate {
            key: c1,
            update: CapUpdate {
                op: CEPH_CAP_OP_UPDATE,
                ino: f,
                cap_id: rv.cap_id,
                seq: rv.seq,
                issue_seq: rv.issue_seq,
                caps: rv.caps,
                wanted,
                ..Default::default()
            },
        };
        let s = if deferred {
            assert_eq!(
                fs.e.step(Some(ack), None).unwrap().state,
                StepState::NeedsAdmission
            );
            fs.e.step(None, permit()).unwrap()
        } else {
            fs.e.step(Some(ack), permit()).unwrap()
        };
        assert_eq!(
            fs.e.journal().last_seq(),
            base,
            "this is a pure ack, not a flush"
        );
        assert!(s.unused_permit.is_some());
        assert!(reply_for(&s.effects, C2, 100).is_none());
        assert_eq!(s.state, StepState::NeedsAdmission, "deferred={deferred}");
        let out = drain(&mut fs.e);
        assert_eq!(reply_for(&out, C2, 100).unwrap().result, 0);
    }
}

#[test]
fn resumed_unlock_wakes_its_parked_lock_request() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    fs.e.journal_mut().set_auto_commit(false);
    let c1 = key(&fs.e, C1);
    let c2 = key(&fs.e, C2);
    let lock = |owner: u64, ltype, wait| Op::SetFileLock {
        path: FilePath::ino(f),
        lock: FileLockArgs {
            rule: CEPH_LOCK_FCNTL,
            owner: owner | (1 << 63),
            ltype,
            wait,
            pid: 1,
            start: 0,
            length: 0,
        },
    };
    fs.e.step(
        Some(EngineInput::Request {
            key: c1,
            request: Request::new(100, lock(1, CEPH_LOCK_EXCL, false)),
        }),
        permit(),
    )
    .unwrap();
    let s =
        fs.e.step(
            Some(EngineInput::Request {
                key: c2,
                request: Request::new(101, lock(2, CEPH_LOCK_EXCL, true)),
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(s.state, StepState::WaitingInput);
    let base = fs.e.journal().last_seq();
    fs.e.step(
        Some(EngineInput::Request {
            key: c1,
            request: Request::new(102, lock(1, CEPH_LOCK_UNLOCK, false)),
        }),
        None,
    )
    .unwrap();
    let s = fs.e.step(None, permit()).unwrap();
    assert!(s.unused_permit.is_some());
    assert_eq!(fs.e.journal().last_seq(), base);
    assert_eq!(s.state, StepState::NeedsAdmission);
    assert!(reply_for(&s.effects, C2, 101).is_none());
    assert_eq!(reply_for(&drain(&mut fs.e), C2, 101).unwrap().result, 0);
}

#[test]
fn last_resumed_release_item_wakes_a_directory_reader() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    fs.e.journal_mut().set_auto_commit(false);
    let c1 = key(&fs.e, C1);
    let c2 = key(&fs.e, C2);
    let s =
        fs.e.step(
            Some(EngineInput::Request {
                key: c2,
                request: Request::new(100, read_dir(d)),
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(s.state, StepState::WaitingInput);
    let items = [ROOT, d]
        .into_iter()
        .map(|ino| {
            let c = fs.e.cap(ino, C1).unwrap();
            CapReleaseItem {
                ino,
                cap_id: c.cap_id,
                mseq: c.mseq,
                issue_seq: c.last_issue,
            }
        })
        .collect();
    let base = fs.e.journal().last_seq();
    fs.e.step(Some(EngineInput::CapRelease { key: c1, items }), None)
        .unwrap();
    for _ in 0..2 {
        let s = fs.e.step(None, permit()).unwrap();
        assert!(s.unused_permit.is_some());
        assert_eq!(s.state, StepState::NeedsAdmission);
        assert!(reply_for(&s.effects, C2, 100).is_none());
    }
    assert_eq!(fs.e.journal().last_seq(), base);
    assert_eq!(reply_for(&drain(&mut fs.e), C2, 100).unwrap().result, 0);
}

#[test]
fn terminal_cleanup_retires_eviction_suppression_for_client_reuse() {
    let mut fs = Fs::new();
    fs.e.journal_mut().set_auto_commit(false);
    for incarnation in 0..2 {
        let c1 = key(&fs.e, C1);
        let c2 = key(&fs.e, C2);
        let tid = 100 + incarnation * 10;
        fs.e.step(
            Some(EngineInput::Request {
                key: c1,
                request: Request::new(
                    tid,
                    Op::Mkdir {
                        path: FilePath::child(ROOT, format!("d{incarnation}")),
                        mode: 0o755,
                        xattrs: XattrMap::new(),
                        ino: 0,
                    },
                ),
            }),
            permit(),
        )
        .unwrap();
        let s = commit(&mut fs.e);
        let d = reply_for(&s.effects, C1, tid)
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .ino;
        drain(&mut fs.e);
        let s =
            fs.e.step(
                Some(EngineInput::Request {
                    key: c2,
                    request: Request::new(tid + 1, read_dir(d)),
                }),
                permit(),
            )
            .unwrap();
        assert_eq!(s.state, StepState::WaitingInput);
        let eff = fs.e.session_stale(C1);
        assert_eq!(
            eff.iter()
                .filter(|e| matches!(e,Effect::EvictClient {client} if *client==C1))
                .count(),
            1,
            "incarnation={incarnation}"
        );
        assert!(
            fs.e.session_stale(C1)
                .iter()
                .all(|e| !matches!(e, Effect::EvictClient { .. }))
        );
        close(&mut fs.e, c1, true);
        drain(&mut fs.e);
        close(&mut fs.e, c2, false);
        drain(&mut fs.e);
        if incarnation == 0 {
            fs.e.install_session_key(SessionKey {
                client: C1,
                incarnation: 10,
            })
            .unwrap();
            fs.e.install_session_key(SessionKey {
                client: C2,
                incarnation: 11,
            })
            .unwrap();
        }
    }
}

#[test]
fn checkpoint_only_cleanup_records_recovery_before_its_receipt() {
    for kill in [false, true] {
        let mut fs = Fs::new();
        let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
        let orphan = fs.create(C1, ROOT, "orphan", CEPH_O_RDWR).unwrap();
        fs.unlink(C1, ROOT, "orphan").unwrap();
        let c1 = key(&fs.e, C1);
        let cp = fs.e.checkpoint().unwrap();
        let at = cp.covered_seq;
        let mut e = restore(cp.clone());
        assert!(e.caps_of(f).is_empty());
        let bound = e.inode(f).unwrap().client_ranges[&C1];
        close(&mut e, c1, kill);
        drain(&mut e);
        let state = e.checkpoint().unwrap();
        let receipt = e.session_cleanup_seq(c1).unwrap();
        assert_eq!(
            state.jobs.len(),
            2,
            "both durable writer bounds need intents"
        );
        for ino in [f, orphan] {
            let j = state.jobs.iter().find(|j| j.ino == ino).unwrap();
            let seq = e
                .journal()
                .records()
                .iter()
                .find(|(_, r)| r.lifecycle_intents.iter().any(|intent| intent.id == j.id))
                .unwrap()
                .0;
            assert!(seq < receipt);
            assert!(matches!(j.kind, LifecycleKind::RecoverSize { .. }));
        }
        let job = state
            .jobs
            .iter()
            .find(|j| j.ino == f)
            .expect("durable writer bound needs recovery before cleanup proof");
        assert!(matches!(job.kind,LifecycleKind::RecoverSize {max_size,..} if max_size>=bound));
        let intent_seq = e
            .journal()
            .records()
            .iter()
            .find(|(_, r)| r.lifecycle_intents.iter().any(|j| j.id == job.id))
            .unwrap()
            .0;
        assert!(intent_seq < receipt);
        assert_eq!(e.inode(f).unwrap().client_ranges[&C1], bound);
        assert!(e.pending_jobs().is_empty(), "gather has not completed");
        // Crash after the intent, before the terminal record. Replay no later record.
        let mut interrupted = restore(cp);
        for (seq, r) in e
            .journal()
            .records()
            .iter()
            .filter(|(seq, _)| *seq > at && *seq <= intent_seq)
        {
            interrupted.replay_record(*seq, r).unwrap();
        }
        *interrupted.journal_mut() = MemJournal::at_seq(intent_seq, false);
        assert!(interrupted.session_cleanup_seq(c1).is_none());
        close(&mut interrupted, c1, kill);
        drain(&mut interrupted);
        assert_eq!(
            interrupted.checkpoint().unwrap().jobs.len(),
            2,
            "resumption must not duplicate intent"
        );
        let keys: Vec<_> = [C2, C3].into_iter().map(|c| key(&interrupted, c)).collect();
        for k in &keys {
            interrupted.reconnect_chunk(*k, &[], &[], false).unwrap();
        }
        interrupted.finish_reconnect(&keys).unwrap();
        interrupted
            .step(Some(EngineInput::RecoveryControlsDrained), permit())
            .unwrap();
        let job = interrupted
            .pending_jobs()
            .into_iter()
            .find(|j| j.ino == f)
            .unwrap();
        let waiting = interrupted
            .step(
                Some(EngineInput::Request {
                    key: keys[0],
                    request: Request::new(
                        500,
                        Op::Getattr {
                            path: FilePath::ino(f),
                            mask: 0,
                        },
                    ),
                }),
                permit(),
            )
            .unwrap();
        assert!(reply_for(&waiting.effects, C2, 500).is_none());
        // The gather also queued the orphan. Drain that advertised work, but
        // never answer the linked file's request before its recovery result.
        let mut state = waiting.state;
        for _ in 0..16 {
            if state == StepState::WaitingInput {
                break;
            }
            assert_eq!(state, StepState::NeedsAdmission);
            let before = interrupted.journal().last_seq();
            let s = interrupted.step(None, permit()).unwrap();
            assert!(interrupted.journal().last_seq() <= before + 1);
            assert!(reply_for(&s.effects, C2, 500).is_none());
            state = s.state;
            if interrupted.uncommitted_records() > 0 {
                let s = commit(&mut interrupted);
                assert!(reply_for(&s.effects, C2, 500).is_none());
                state = s.state;
            }
        }
        assert_eq!(state, StepState::WaitingInput);
        interrupted
            .step(
                Some(EngineInput::LifecycleCompleted {
                    id: job.id,
                    result: LifecycleResult::Recovered {
                        size: 321,
                        mtime: Timestamp::new(9000, 0),
                    },
                }),
                permit(),
            )
            .unwrap();
        assert!(interrupted.lifecycle_intent(job.id).is_some());
        assert!(reply_for(&commit(&mut interrupted).effects, C2, 500).is_none());
        let out = drain(&mut interrupted);
        assert_eq!(
            reply_for(&out, C2, 500)
                .unwrap()
                .target
                .as_ref()
                .unwrap()
                .size,
            321
        );
        assert_eq!(interrupted.inode(f).unwrap().size, 321);
        assert!(interrupted.inode(f).unwrap().client_ranges.is_empty());
        assert!(interrupted.inode(orphan).is_none());
        assert!(
            interrupted
                .pending_jobs()
                .iter()
                .any(|j| j.ino == orphan && matches!(j.kind, LifecycleKind::Purge { .. }))
        );
    }
}

#[test]
fn completed_gather_schedules_unclaimed_strays_but_preserves_reconnected_ones() {
    let mut fs = Fs::new();
    let orphan = fs.create(C1, ROOT, "orphan", CEPH_O_RDWR).unwrap();
    let kept = fs.create(C1, ROOT, "kept", CEPH_O_RDWR).unwrap();
    fs.open(C2, kept, CEPH_O_RDONLY).unwrap();
    fs.settle(&fs.last.clone());
    fs.unlink(C1, ROOT, "orphan").unwrap();
    fs.unlink(C1, ROOT, "kept").unwrap();
    let c1 = key(&fs.e, C1);
    let c2 = key(&fs.e, C2);
    let c3 = key(&fs.e, C3);
    let cap = fs.e.cap(kept, C2).unwrap().clone();
    let cp = fs.e.checkpoint().unwrap();
    let bound = cp
        .inodes
        .iter()
        .find(|i| i.core.ino == orphan)
        .unwrap()
        .core
        .max_client_range();
    let mut e = restore(cp);
    close(&mut e, c1, true);
    drain(&mut e);
    assert!(e.inode(orphan).is_some());
    assert!(e.inode(kept).is_some());
    assert!(e.pending_jobs().is_empty());
    e.reconnect_chunk(
        c2,
        &[ReconnectCap {
            ino: kept,
            cap_id: cap.cap_id,
            wanted: cap.wanted(),
            issued: cap.issued(),
            realm: ROOT,
            snap_follows: 0,
            fcntl_locks: vec![],
            flock_locks: vec![],
        }],
        &[],
        false,
    )
    .unwrap();
    e.reconnect_chunk(c3, &[], &[], false).unwrap();
    let before = e.journal().last_seq();
    let s = e.finish_reconnect(&[c2, c3]).unwrap();
    assert_eq!(e.journal().last_seq(), before);
    assert!(
        e.inode(orphan).is_some(),
        "structural completion cannot purge"
    );
    assert_eq!(s.state, StepState::NeedsAdmission);
    let s = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    assert_eq!(
        s.state,
        StepState::NeedsAdmission,
        "unclaimed strays need a durable purge step"
    );
    drain(&mut e);
    assert!(e.inode(orphan).is_none());
    assert!(e.inode(kept).is_some());
    assert!(e.cap(kept, C2).is_some());
    let cp = e.checkpoint().unwrap();
    assert!(cp.jobs.iter().any(|j| j.ino == orphan
        && matches!(&j.kind,LifecycleKind::Purge {max_size,..} if *max_size>=bound)));
    assert!(
        !cp.jobs
            .iter()
            .any(|j| j.ino == kept && matches!(j.kind, LifecycleKind::Purge { .. }))
    );
}

#[test]
fn replay_rejects_delegation_of_a_retired_inode_without_mutation() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    fs.unlink(C1, ROOT, "f").unwrap();
    fs.release(C1, f);
    let purge =
        fs.e.pending_jobs()
            .into_iter()
            .find(|j| j.ino == f)
            .unwrap();
    fs.e.step(
        Some(EngineInput::LifecycleCompleted {
            id: purge.id,
            result: LifecycleResult::Done,
        }),
        permit(),
    )
    .unwrap();
    commit(&mut fs.e);
    drain(&mut fs.e);
    let cp = fs.e.checkpoint().unwrap();
    let mut e = restore(cp.clone());
    let mut r = JournalRecord::new(RecordKind::ClientRanges);
    let mut a = cp.ino_alloc.clone();
    a.delegated.push((C1, InoRange::new(f, 1)));
    r.ino_alloc = Some(a);
    assert!(
        e.replay_record(cp.covered_seq + 1, &r).is_err(),
        "consumed inode cannot return through delegation"
    );
    assert_eq!(e.checkpoint().unwrap(), cp);
}

#[test]
fn replay_rejects_delegation_owner_transfer_without_mutation() {
    let fs = Fs::new();
    let mut cp = fs.e.checkpoint().unwrap();
    let start = cp.ino_alloc.free[0].start;
    cp.ino_alloc.free[0].start += 4;
    cp.ino_alloc.free[0].len -= 4;
    cp.ino_alloc.delegated.push((C1, InoRange::new(start, 4)));
    let mut e = restore(cp.clone());
    let mut r = JournalRecord::new(RecordKind::ClientRanges);
    let mut a = cp.ino_alloc.clone();
    a.delegated[0].0 = C2;
    r.ino_alloc = Some(a);
    assert!(
        e.replay_record(cp.covered_seq + 1, &r).is_err(),
        "delegation owner is part of its provenance"
    );
    assert_eq!(e.checkpoint().unwrap(), cp);
}

fn remote_only(cp: &mut EngineCheckpoint, ino: Ino) {
    let i = cp.inodes.iter_mut().find(|i| i.core.ino == ino).unwrap();
    let parent = i.parent.take().unwrap();
    i.remote_links.push(parent.clone());
    let dir = cp
        .inodes
        .iter_mut()
        .find(|i| i.core.ino == parent.0)
        .unwrap();
    dir.entries
        .iter_mut()
        .find(|(_, d)| d.ino == ino)
        .unwrap()
        .1
        .primary = false;
}
#[test]
fn restore_rejects_remote_only_linked_files_and_directories() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let cp = fs.e.checkpoint().unwrap();
    let results: Vec<_> = [f, d]
        .into_iter()
        .map(|ino| {
            let mut bad = cp.clone();
            remote_only(&mut bad, ino);
            Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), bad).is_err()
        })
        .collect();
    assert_eq!(results, vec![true, true]);
}
#[test]
fn replay_rejects_remote_only_primary_replacement_transactionally() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let cp = fs.e.checkpoint().unwrap();
    let mut e = restore(cp.clone());
    for (ino, name) in [(f, b"f"), (d, b"d")] {
        let mut r = JournalRecord::new(RecordKind::Rename);
        r.dentries.push(DentryUpdate {
            dir: ROOT,
            name: name.to_vec(),
            link: Some(DentryLink {
                ino,
                primary: false,
            }),
        });
        assert!(e.replay_record(cp.covered_seq + 1, &r).is_err());
        assert_eq!(e.checkpoint().unwrap(), cp);
    }
}

#[test]
fn linked_primary_chain_must_reach_root_in_restore_and_replay() {
    let mut fs = Fs::new();
    let d = fs.mkdir(C1, ROOT, "d").unwrap();
    let f = fs.mkfile(C1, d, "f").unwrap();
    let cp = fs.e.checkpoint().unwrap();
    let mut bad = cp.clone();
    let root = bad.inodes.iter_mut().find(|i| i.core.ino == ROOT).unwrap();
    root.entries.clear();
    root.core.dirstat.nsubdirs = 0;
    let dir = bad.inodes.iter_mut().find(|i| i.core.ino == d).unwrap();
    dir.parent = None;
    dir.core.nlink = 0;
    dir.core.stray_auth_path = Some(b"/d".to_vec());
    let rejected_restore =
        Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), bad).is_err();
    let mut r = JournalRecord::new(RecordKind::Rmdir);
    let mut root = cp
        .inodes
        .iter()
        .find(|i| i.core.ino == ROOT)
        .unwrap()
        .core
        .clone();
    root.version += 1;
    root.dirstat.nsubdirs = 0;
    r.inodes.push(root);
    let mut dir = cp
        .inodes
        .iter()
        .find(|i| i.core.ino == d)
        .unwrap()
        .core
        .clone();
    dir.version += 1;
    dir.nlink = 0;
    dir.stray_auth_path = Some(b"/d".to_vec());
    r.inodes.push(dir);
    r.dentries.push(DentryUpdate {
        dir: ROOT,
        name: b"d".to_vec(),
        link: None,
    });
    let mut e = restore(cp.clone());
    let rejected_replay = e.replay_record(cp.covered_seq + 1, &r).is_err();
    assert!(
        rejected_restore && rejected_replay,
        "linked {f:#x} terminates at an unlinked parent"
    );
    assert_eq!(e.checkpoint().unwrap(), cp);
}

#[test]
fn allocator_provenance_accepts_split_retirement_and_new_reservations() {
    let fs = Fs::new();
    let mut cp = fs.e.checkpoint().unwrap();
    let base = cp.ino_alloc.free[0].start;
    cp.ino_alloc = engine::ino::InoAllocState {
        floor: base + 32,
        free: vec![InoRange::new(base + 4, 8)],
        delegated: vec![(C1, InoRange::new(base, 4))],
    };
    let mut e = restore(cp.clone());
    let mut next = cp.ino_alloc.clone();
    next.floor = base + 40;
    next.free = vec![InoRange::new(base + 10, 2), InoRange::new(base + 32, 4)];
    next.delegated = vec![
        (C1, InoRange::new(base, 10)),
        (C2, InoRange::new(base + 36, 4)),
    ];
    let mut r = JournalRecord::new(RecordKind::ClientRanges);
    r.ino_alloc = Some(next.clone());
    e.replay_record(cp.covered_seq + 1, &r).unwrap();
    // Shrink a delegation. Its retired suffix is burned, not free space.
    next.delegated[0].1.len = 3;
    r.ino_alloc = Some(next.clone());
    e.replay_record(cp.covered_seq + 2, &r).unwrap();
    *e.journal_mut() = MemJournal::at_seq(cp.covered_seq + 2, false);
    let before = e.checkpoint().unwrap();
    assert_eq!(before.ino_alloc, next);
    for delegated in [false, true] {
        let mut bad = next.clone();
        if delegated {
            bad.delegated.push((C1, InoRange::new(base + 3, 1)));
        } else {
            bad.free.insert(0, InoRange::new(base + 3, 1));
        }
        r.ino_alloc = Some(bad);
        assert!(e.replay_record(before.covered_seq + 1, &r).is_err());
        assert_eq!(e.checkpoint().unwrap(), before);
    }
}

#[test]
fn primary_cycles_are_rejected_and_hardlink_promotion_stays_valid() {
    let mut fs = Fs::new();
    let a = fs.mkdir(C1, ROOT, "a").unwrap();
    let b = fs.mkdir(C1, a, "b").unwrap();
    let f = fs.mkfile(C1, a, "f").unwrap();
    fs.link(C1, b, "alias", f).unwrap();
    let cp = fs.e.checkpoint().unwrap();
    let mut bad = cp.clone();
    let root = bad.inodes.iter_mut().find(|i| i.core.ino == ROOT).unwrap();
    root.entries.clear();
    root.core.dirstat.nsubdirs = 0;
    let dir = bad.inodes.iter_mut().find(|i| i.core.ino == a).unwrap();
    dir.parent = Some((b, b"cycle".to_vec()));
    let dir = bad.inodes.iter_mut().find(|i| i.core.ino == b).unwrap();
    dir.entries.push((
        engine::inode::DentryKey::new(dir.core.dir_hash, b"cycle"),
        engine::inode::Dentry {
            ino: a,
            primary: true,
        },
    ));
    dir.core.dirstat.nsubdirs += 1;
    assert!(Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), bad).is_err());
    let mut r = JournalRecord::new(RecordKind::Rename);
    let mut root = cp
        .inodes
        .iter()
        .find(|i| i.core.ino == ROOT)
        .unwrap()
        .core
        .clone();
    root.version += 1;
    root.dirstat.nsubdirs = 0;
    r.inodes.push(root);
    let mut dir = cp
        .inodes
        .iter()
        .find(|i| i.core.ino == b)
        .unwrap()
        .core
        .clone();
    dir.version += 1;
    dir.dirstat.nsubdirs += 1;
    r.inodes.push(dir);
    r.dentries = vec![
        DentryUpdate {
            dir: ROOT,
            name: b"a".to_vec(),
            link: None,
        },
        DentryUpdate {
            dir: b,
            name: b"cycle".to_vec(),
            link: Some(DentryLink {
                ino: a,
                primary: true,
            }),
        },
    ];
    let mut e = restore(cp.clone());
    assert!(e.replay_record(cp.covered_seq + 1, &r).is_err());
    assert_eq!(e.checkpoint().unwrap(), cp);
    fs.unlink(C1, a, "f").unwrap();
    let at = cp.covered_seq;
    for (seq, r) in fs.e.journal().records().iter().filter(|(seq, _)| *seq > at) {
        e.replay_record(*seq, r).unwrap();
    }
    assert_eq!(e.parent_of(f), Some((b, b"alias".to_vec())));
    assert_eq!(e.authorization_path(f), Some(b"/a/b/alias".to_vec()));
    let at = e.durable_seq();
    fs.unlink(C1, b, "alias").unwrap();
    for (seq, r) in fs.e.journal().records().iter().filter(|(seq, _)| *seq > at) {
        e.replay_record(*seq, r).unwrap();
    }
    assert_eq!(e.inode(f).unwrap().nlink, 0);
    assert_eq!(e.authorization_path(f), Some(b"/a/b/alias".to_vec()));
    *e.journal_mut() = MemJournal::at_seq(e.durable_seq(), false);
    let cp = e.checkpoint().unwrap();
    assert_eq!(restore(cp.clone()).checkpoint().unwrap(), cp);
}

#[test]
fn gather_discovers_range_free_strays_and_replays_purge_before_callback() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "read-only").unwrap();
    fs.unlink(C1, ROOT, "read-only").unwrap();
    let cp = fs.e.checkpoint().unwrap();
    assert!(
        cp.inodes
            .iter()
            .find(|i| i.core.ino == f)
            .unwrap()
            .core
            .client_ranges
            .is_empty()
    );
    let mut e = restore(cp.clone());
    let keys: Vec<_> = [C1, C2, C3].into_iter().map(|c| key(&e, c)).collect();
    for k in &keys {
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let before = e.journal().last_seq();
    assert_eq!(e.finish_reconnect(&keys).unwrap().state, StepState::Idle);
    assert_eq!(e.journal().last_seq(), before);
    assert!(e.inode(f).is_some());
    assert_eq!(
        e.step(Some(EngineInput::RecoveryControlsDrained), None)
            .unwrap()
            .state,
        StepState::NeedsAdmission
    );
    e.step(None, None).unwrap();
    assert!(e.inode(f).is_some());
    let s = e.step(None, permit()).unwrap();
    assert_eq!(s.unused_permit, permit());
    assert_eq!(
        e.journal().last_seq(),
        before,
        "release itself submits no record"
    );
    let s = e.step(None, permit()).unwrap();
    assert!(s.effects.is_empty());
    assert_eq!(e.journal().last_seq(), before + 1);
    e.journal_mut().commit_all(); // The durable intent's callback is lost.
    let record = e.journal().records().last().unwrap().1.clone();
    let mut recovered = restore(cp);
    recovered.replay_record(before + 1, &record).unwrap();
    *recovered.journal_mut() = MemJournal::at_seq(before + 1, false);
    assert!(recovered.inode(f).is_none());
    assert!(recovered.pending_jobs().is_empty());
    for k in &keys {
        recovered.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let s = recovered.finish_reconnect(&keys).unwrap();
    recovered
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
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
    assert!(matches!(job.kind, LifecycleKind::Purge { .. }));
    recovered
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: job.id,
                result: LifecycleResult::Done,
            }),
            permit(),
        )
        .unwrap();
    commit(&mut recovered);
    drain(&mut recovered);
    assert!(recovered.pending_jobs().is_empty());
}

#[test]
fn restored_missing_writer_cleanup_preserves_a_remaining_live_writer() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    fs.open(C2, f, CEPH_O_RDWR).unwrap();
    fs.settle(&fs.last.clone());
    let c1 = key(&fs.e, C1);
    let c2 = key(&fs.e, C2);
    let c3 = key(&fs.e, C3);
    let cap = fs.e.cap(f, C2).unwrap().clone();
    let mut e = restore(fs.e.checkpoint().unwrap());
    close(&mut e, c1, true);
    drain(&mut e);
    let old = e.checkpoint().unwrap().jobs[0].id;
    e.reconnect_chunk(
        c2,
        &[ReconnectCap {
            ino: f,
            cap_id: cap.cap_id,
            wanted: cap.wanted(),
            issued: cap.issued(),
            realm: ROOT,
            snap_follows: 0,
            fcntl_locks: vec![],
            flock_locks: vec![],
        }],
        &[],
        false,
    )
    .unwrap();
    e.reconnect_chunk(c3, &[], &[], false).unwrap();
    let before = e.journal().last_seq();
    let s = e.finish_reconnect(&[c2, c3]).unwrap();
    assert_eq!(e.journal().last_seq(), before);
    assert!(s.effects.iter().all(|e| !matches!(e, Effect::Lifecycle(_))));
    let mut effects = s.effects;
    effects.extend(drain(&mut e));
    let rv = revokes(&effects)
        .into_iter()
        .find(|m| m.ino == f && m.client == C2)
        .unwrap();
    e.step(
        Some(EngineInput::CapUpdate {
            key: c2,
            update: CapUpdate {
                op: CEPH_CAP_OP_FLUSH,
                ino: f,
                cap_id: rv.cap_id,
                seq: rv.seq,
                issue_seq: rv.issue_seq,
                caps: rv.caps,
                wanted: rv.wanted,
                dirty: FW,
                flush_tid: 77,
                size: 1234,
                ..Default::default()
            },
        }),
        None,
    )
    .unwrap();
    let s = e.step(None, permit()).unwrap();
    assert_eq!(s.state, StepState::WaitingCommit(before + 1));
    let s = commit(&mut e);
    assert!(
        cap_msgs(&s.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK && m.flush_tid == 77)
    );
    let job = e.pending_jobs().pop().unwrap();
    assert!(job.id > old);
    assert!(e.lifecycle_intent(old).is_none());
    e.step(
        Some(EngineInput::LifecycleCompleted {
            id: job.id,
            result: LifecycleResult::Recovered {
                size: 1234,
                mtime: Timestamp::new(9000, 0),
            },
        }),
        permit(),
    )
    .unwrap();
    commit(&mut e);
    drain(&mut e);
    assert_eq!(e.inode(f).unwrap().size, 1234);
    assert!(!e.inode(f).unwrap().client_ranges.contains_key(&C1));
    assert!(e.inode(f).unwrap().client_ranges.contains_key(&C2));
    assert!(e.cap(f, C2).is_some());
    e.step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    drain(&mut e);
}

#[test]
fn restored_stray_waits_for_truncate_before_replacing_recovery_with_purge() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    for size in [1000, 10] {
        fs.setattr(
            C1,
            f,
            SetattrArgs {
                mask: CEPH_SETATTR_SIZE,
                size,
                ..Default::default()
            },
        )
        .unwrap();
    }
    fs.unlink(C1, ROOT, "f").unwrap();
    let c1 = key(&fs.e, C1);
    let cp = fs.e.checkpoint().unwrap();
    let at = cp.covered_seq;
    let bound = fs.e.inode(f).unwrap().max_client_range();
    let mut e = restore(cp.clone());
    close(&mut e, c1, false);
    drain(&mut e);
    let state = e.checkpoint().unwrap();
    assert!(
        state
            .jobs
            .iter()
            .any(|j| j.ino == f && matches!(j.kind, LifecycleKind::RecoverSize { .. }))
    );
    let keys: Vec<_> = [C2, C3].into_iter().map(|c| key(&e, c)).collect();
    for k in &keys {
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    e.finish_reconnect(&keys).unwrap();
    e.step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    drain(&mut e);
    assert!(
        e.inode(f).is_some(),
        "truncate data work must drain before inode purge"
    );
    let jobs = e.pending_jobs();
    assert_eq!(jobs.len(), 1);
    assert!(matches!(jobs[0].kind, LifecycleKind::Truncate { .. }));
    e.step(
        Some(EngineInput::LifecycleCompleted {
            id: jobs[0].id,
            result: LifecycleResult::Done,
        }),
        permit(),
    )
    .unwrap();
    commit(&mut e);
    drain(&mut e);
    assert!(e.inode(f).is_none());
    let jobs = e.pending_jobs();
    assert_eq!(jobs.len(), 1);
    assert!(matches!(jobs[0].kind,LifecycleKind::Purge {max_size,..} if max_size>=bound));
    let mut recovered = restore(cp);
    for (seq, r) in e.journal().records().iter().filter(|(seq, _)| *seq > at) {
        recovered.replay_record(*seq, r).unwrap();
    }
    assert!(recovered.inode(f).is_none());
    assert!(recovered.session_cleanup_seq(c1).is_some());
}

#[test]
fn replay_rejects_cleanup_receipt_without_a_durable_writer_decision() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "f", CEPH_O_RDWR).unwrap();
    let c1 = key(&fs.e, C1);
    let c2 = key(&fs.e, C2);
    let cp = fs.e.checkpoint().unwrap();
    let mut e = restore(cp.clone());
    let mut record = JournalRecord::new(RecordKind::SessionClose);
    record.closed_sessions.push(c1);
    assert!(
        e.replay_record(cp.covered_seq + 1, &record).is_err(),
        "a bare receipt cannot abandon its writer bound"
    );
    assert_eq!(e.checkpoint().unwrap(), cp);
    // Closing an unrelated session does not require recovering the live owner.
    record.closed_sessions = vec![c2];
    e.replay_record(cp.covered_seq + 1, &record).unwrap();
    assert!(e.session_cleanup_seq(c2).is_some());
    assert!(e.session(C1).is_some());
    assert!(e.inode(f).unwrap().client_ranges.contains_key(&C1));
}

fn closed_writer_checkpoint(stray: bool) -> (EngineCheckpoint, Ino, SessionKey) {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "writer", CEPH_O_RDWR).unwrap();
    if stray {
        fs.unlink(C1, ROOT, "writer").unwrap();
    }
    let owner = key(&fs.e, C1);
    let mut e = restore(fs.e.checkpoint().unwrap());
    close(&mut e, owner, true);
    drain(&mut e);
    let cp = e.checkpoint().unwrap();
    assert_eq!(cp.jobs.len(), 1);
    assert!(cp.cleanup_receipts.iter().any(|(key, _)| *key == owner));
    (cp, f, owner)
}

#[test]
fn restore_rejects_abandoned_closed_or_orphan_writer_range() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "writer", CEPH_O_RDWR).unwrap();
    let owner = key(&fs.e, C1);
    let original = fs.e.checkpoint().unwrap();
    for receipt in [true, false] {
        let mut cp = original.clone();
        cp.sessions.retain(|s| s.key != owner);
        if receipt {
            cp.cleanup_receipts.push((owner, cp.covered_seq));
        }
        assert!(cp.jobs.is_empty());
        assert!(
            cp.inodes
                .iter()
                .find(|i| i.core.ino == f)
                .unwrap()
                .core
                .client_ranges
                .contains_key(&C1)
        );
        assert!(
            Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).is_err(),
            "ownerless writer range without recovery; receipt={receipt}"
        );
    }
}

#[test]
fn replay_rejects_bare_recovery_retirement_without_state_change() {
    let (cp, f, _) = closed_writer_checkpoint(false);
    let job = cp.jobs[0].clone();
    let mut e = restore(cp.clone());
    assert_eq!(
        e.checkpoint().unwrap(),
        cp,
        "a covering pending recovery intent is valid"
    );
    let mut bad = JournalRecord::new(RecordKind::LifecycleComplete);
    bad.completed_jobs.push(job.id);
    assert!(
        e.replay_record(cp.covered_seq + 1, &bad).is_err(),
        "job retirement cannot abandon the closed writer's range"
    );
    assert_eq!(e.checkpoint().unwrap(), cp);
    assert_eq!(e.durable_seq(), cp.covered_seq);
    assert_eq!(e.submitted_seq(), cp.covered_seq);
    assert!(e.journal().records().is_empty());
    assert!(!e.is_failed());

    // A real completion journals recovered metadata and range retirement.
    let mut source = restore(cp.clone());
    let keys: Vec<_> = [C2, C3].into_iter().map(|c| key(&source, c)).collect();
    for k in &keys {
        source.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    source.finish_reconnect(&keys).unwrap();
    source
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: job.id,
                result: LifecycleResult::Recovered {
                    size: 321,
                    mtime: Timestamp::new(9000, 0),
                },
            }),
            permit(),
        )
        .unwrap();
    let (seq, record) = source.journal().records().last().unwrap().clone();
    e.replay_record(seq, &record).unwrap();
    *e.journal_mut() = MemJournal::at_seq(seq, false);
    commit(&mut source);
    drain(&mut source);
    assert_eq!(e.checkpoint().unwrap(), source.checkpoint().unwrap());
    assert_eq!(e.inode(f).unwrap().size, 321);
    assert!(e.inode(f).unwrap().client_ranges.is_empty());
}

#[test]
fn newer_live_incarnation_preserves_ranges_across_restore_and_replay() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "writer", CEPH_O_RDWR).unwrap();
    let old = key(&fs.e, C1);
    fs.e.journal_mut().set_auto_commit(false);
    close(&mut fs.e, old, false);
    drain(&mut fs.e);
    let before = fs.e.checkpoint().unwrap();
    let new = SessionKey {
        client: C1,
        incarnation: 10,
    };
    fs.e.install_session_key(new).unwrap();
    fs.e.step(
        Some(EngineInput::Request {
            key: new,
            request: Request::new(
                900,
                Op::Open {
                    path: FilePath::ino(f),
                    flags: CEPH_O_RDWR,
                    mode: 0,
                    mask: 0,
                    old_size: 0,
                },
            ),
        }),
        permit(),
    )
    .unwrap();
    let (seq, record) = fs.e.journal().records().last().unwrap().clone();
    commit(&mut fs.e);
    drain(&mut fs.e);
    let cp = fs.e.checkpoint().unwrap();
    assert!(cp.jobs.is_empty());
    assert!(
        cp.inodes
            .iter()
            .find(|i| i.core.ino == f)
            .unwrap()
            .core
            .client_ranges
            .contains_key(&C1)
    );
    assert_eq!(restore(cp.clone()).checkpoint().unwrap(), cp);

    // The rank repository supplies the new live key before a range-only WAL
    // record. The retired key's receipt must not invalidate that new owner.
    let mut replayed = restore(before);
    replayed.install_session_key(new).unwrap();
    replayed.replay_record(seq, &record).unwrap();
    assert_eq!(replayed.session(C1).unwrap().epoch, new.incarnation);
    assert_eq!(
        replayed.session_cleanup_seq(old),
        fs.e.session_cleanup_seq(old)
    );
    assert!(replayed.inode(f).unwrap().client_ranges.contains_key(&C1));
    replayed.namespace_eq(&fs.e).unwrap();
}

#[test]
fn replacement_recovery_intent_preserves_closed_writer_coverage() {
    let (cp, f, _) = closed_writer_checkpoint(false);
    let mut replacement = cp.jobs[0].clone();
    let old = replacement.id;
    replacement.id = cp.next_job_id;
    let mut record = JournalRecord::new(RecordKind::LifecycleComplete);
    record.completed_jobs.push(old);
    record.lifecycle_intents.push(replacement.clone());
    let mut e = restore(cp.clone());
    e.replay_record(cp.covered_seq + 1, &record).unwrap();
    *e.journal_mut() = MemJournal::at_seq(cp.covered_seq + 1, false);
    let after = e.checkpoint().unwrap();
    assert_eq!(after.jobs, vec![replacement]);
    assert!(e.inode(f).unwrap().client_ranges.contains_key(&C1));
    assert_eq!(restore(after.clone()).checkpoint().unwrap(), after);
}

#[test]
fn closed_writer_recovery_to_purge_replays_atomically() {
    let (cp, f, owner) = closed_writer_checkpoint(true);
    let old = cp.jobs[0].id;
    let bound = cp
        .inodes
        .iter()
        .find(|i| i.core.ino == f)
        .unwrap()
        .core
        .client_ranges[&C1];
    let mut source = restore(cp.clone());
    let keys: Vec<_> = [C2, C3].into_iter().map(|c| key(&source, c)).collect();
    for k in &keys {
        source.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    source.finish_reconnect(&keys).unwrap();
    source
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    source.step(None, permit()).unwrap();
    let (seq, record) = source.journal().records().last().unwrap().clone();
    assert!(record.completed_jobs.contains(&old));
    assert!(record.purged.contains(&f));
    let mut e = restore(cp);
    e.replay_record(seq, &record).unwrap();
    *e.journal_mut() = MemJournal::at_seq(seq, false);
    commit(&mut source);
    drain(&mut source);
    assert_eq!(e.checkpoint().unwrap(), source.checkpoint().unwrap());
    assert!(e.inode(f).is_none());
    assert!(e.session_cleanup_seq(owner).is_some());
    assert!(
        e.checkpoint().unwrap().jobs.iter().any(|j| j.ino == f
            && matches!(j.kind,LifecycleKind::Purge {max_size,..} if max_size>=bound))
    );
}
