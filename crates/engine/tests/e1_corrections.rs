//! Independent-review regressions through the production ownership boundary.
mod common;
use common::*;
use engine::consts::*;
use engine::*;

fn permit() -> Option<RecordPermit> {
    Some(RecordPermit {
        max_record_bytes: 16 << 20,
    })
}
fn key(e: &Engine<MemJournal>, client: ClientId) -> SessionKey {
    SessionKey {
        client,
        incarnation: e.session(client).unwrap().epoch,
    }
}
fn node(tid: u64, name: &str) -> Request {
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
}
fn input(e: &Engine<MemJournal>, client: ClientId, request: Request) -> EngineInput {
    EngineInput::Request {
        key: key(e, client),
        request,
    }
}
fn retirement(tid: u64, floor: u64) -> Request {
    let mut r = Request::new(
        tid,
        Op::Setattr {
            path: FilePath::ino(ROOT),
            args: SetattrArgs::default(),
        },
    );
    r.oldest_client_tid = floor;
    r
}
fn commit(e: &mut Engine<MemJournal>) -> EngineStep {
    let seq = e.journal().last_seq();
    e.journal_mut().commit_all();
    let step = e.journal_event(JournalEvent::Committed(seq));
    assert_eq!(e.journal().last_seq(), seq);
    step
}
fn drive(e: &mut Engine<MemJournal>, mut step: EngineStep) -> Vec<Effect> {
    let mut out = Vec::new();
    for _ in 0..256 {
        out.extend(step.effects);
        step = match step.state {
            StepState::WaitingCommit(_) => commit(e),
            StepState::NeedsAdmission => e.step(None, permit()).unwrap(),
            StepState::Idle | StepState::WaitingInput => return out,
            StepState::Failed => panic!("engine failed"),
        };
    }
    panic!("engine did not stop");
}
fn ready(cp: EngineCheckpoint, cfg: EngineConfig, xattr: Option<Ino>) -> Engine<MemJournal> {
    let mut e = Engine::restore(cfg, MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    let keys: Vec<_> = [C1, C2, C3]
        .into_iter()
        .filter_map(|c| {
            e.session(c).map(|s| SessionKey {
                client: c,
                incarnation: s.epoch,
            })
        })
        .collect();
    for k in &keys {
        let caps = if k.client == C1 {
            xattr
                .map(|ino| ReconnectCap {
                    ino,
                    cap_id: 77,
                    wanted: XX,
                    issued: XX | P,
                    realm: ROOT,
                    snap_follows: 0,
                    fcntl_locks: vec![],
                    flock_locks: vec![],
                })
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        e.reconnect_chunk(*k, &caps, &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drive(&mut e, step);
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    drive(&mut e, step);
    e
}
fn noop(e: &Engine<MemJournal>) -> EngineInput {
    EngineInput::CapRelease {
        key: key(e, C2),
        items: Vec::new(),
    }
}
fn pressure_with_checkpoint() -> (Engine<MemJournal>, EngineCheckpoint) {
    let mut fs = Fs::new();
    let mut r = node(10, "f");
    if let Op::Mknod { xattrs, .. } = &mut r.op {
        xattrs.insert(b"user.k".to_vec(), Vec::new());
    }
    let ino = fs.call_req(C1, r).target.unwrap().ino;
    let mut cfg = config();
    cfg.retry_limits.max_bytes = 256 + 4096 + 272 + 1 + 2 * (304 + 6) + 8192 + 1024;
    let cp = fs.e.checkpoint().unwrap();
    let mut e = ready(cp.clone(), cfg, Some(ino));
    let a = input(&e, C1, node(100, "waiting"));
    e.step(Some(a), None).unwrap();
    let cap = e.cap(ino, C1).unwrap();
    let dirty = EngineInput::CapUpdate {
        key: key(&e, C1),
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
            xattrs: Some((2, XattrMap::from([(b"user.k".to_vec(), vec![42; 4096])]))),
            ..Default::default()
        },
    };
    let step = e.step(Some(dirty), permit()).unwrap();
    drive(&mut e, step);
    assert_eq!(e.waiting_requests(), 1);
    assert_eq!(
        e.step(None, permit()).unwrap().state,
        StepState::WaitingInput
    );
    (e, cp)
}
fn pressure() -> Engine<MemJournal> {
    pressure_with_checkpoint().0
}
fn expect_stale(e: &mut Engine<MemJournal>, request: &Request) {
    let before = e.journal().last_seq();
    for _ in 0..64 {
        match e.step(None, permit()) {
            Err(StepError::InvalidRetryIdentity {
                input: Some(EngineInput::Request { key: k, request: r }),
                permit: p,
            }) => {
                assert_eq!(k, key(e, C1));
                assert_eq!(&r, request);
                assert_eq!(p, permit());
                assert_eq!(e.journal().last_seq(), before);
                return;
            }
            Ok(step) => {
                assert!(
                    !step
                        .effects
                        .iter()
                        .any(|x| matches!(x,Effect::Reply(r) if r.client==C1&&r.tid==request.tid))
                );
                assert_eq!(
                    e.journal().last_seq(),
                    before,
                    "stale request submitted WAL"
                );
                assert_eq!(
                    step.state,
                    StepState::NeedsAdmission,
                    "owned stale request was not returned"
                );
            }
            Err(other) => panic!("wrong returning error: {other}"),
        }
    }
    panic!("stale input was not returned");
}

#[test]
fn state_1_pressure_resume_rejects_retired_input_and_every_acknowledged_record_replays() {
    let mut fs = Fs::new();
    fs.call_req(
        C2,
        Request::new(
            1,
            Op::Getattr {
                path: FilePath::ino(ROOT),
                mask: 0,
            },
        ),
    );
    let cp = fs.e.checkpoint().unwrap();
    fs.e.journal_mut().set_auto_commit(false);
    let request = node(100, "must-not-execute");
    let pending = input(&fs.e, C1, request.clone());
    fs.e.step(Some(pending), None).unwrap();
    let cap = fs.e.cap(ROOT, C2).unwrap();
    let item = CapReleaseItem {
        ino: ROOT,
        cap_id: cap.cap_id,
        mseq: cap.mseq,
        issue_seq: cap.last_issue,
    };
    fs.e.step(
        Some(EngineInput::CapRelease {
            key: key(&fs.e, C2),
            items: vec![item],
        }),
        permit(),
    )
    .unwrap();
    let free = input(&fs.e, C1, retirement(200, 101));
    fs.e.step(Some(free), permit()).unwrap();
    let step = commit(&mut fs.e);
    assert!(reply_for(&step.effects, C1, 200).is_some());
    expect_stale(&mut fs.e, &request);
    assert_eq!(fs.e.session(C1).unwrap().durable_oldest_client_tid, 101);
    assert_eq!(fs.e.waiting_requests(), 0);
    let mut replay = Engine::restore(
        config(),
        MemJournal::at_seq(cp.covered_seq, false),
        cp.clone(),
    )
    .unwrap();
    for (seq, record) in
        fs.e.journal()
            .records()
            .iter()
            .filter(|(s, _)| *s > cp.covered_seq)
    {
        replay.replay_record(*seq, record).unwrap();
        *replay.journal_mut() = MemJournal::at_seq(*seq, false);
    }
    replay.namespace_eq(&fs.e).unwrap();
}

#[test]
fn state_1_continuation_rechecks_observed_floor_without_its_own_identity_exemption() {
    let mut fs = Fs::new();
    let request = node(100, "stale-continuation");
    let pending = input(&fs.e, C1, request.clone());
    fs.e.step(Some(pending), None).unwrap();
    // Exercise an existing public compatibility read while admission owns the
    // continuation. No WAL or committed floor is needed to make tid100 stale.
    let mut read = Request::new(
        200,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    read.oldest_client_tid = 101;
    fs.e.handle_request(C1, read);
    expect_stale(&mut fs.e, &request);
    assert_eq!(fs.e.session(C1).unwrap().durable_oldest_client_tid, 0);
}

#[test]
fn state_1_parked_request_rechecks_floor_before_another_execution_attempt() {
    let mut fs = Fs::new();
    fs.call_req(
        C2,
        Request::new(
            1,
            Op::Getattr {
                path: FilePath::ino(ROOT),
                mask: 0,
            },
        ),
    );
    let request = node(100, "stale-parked");
    let pending = input(&fs.e, C1, request.clone());
    let step = fs.e.step(Some(pending), permit()).unwrap();
    assert!(reply_for(&step.effects, C1, 100).is_none());
    assert_eq!(fs.e.waiting_requests(), 1);
    let mut unlock = Request::new(
        200,
        Op::SetFileLock {
            path: FilePath::ino(ROOT),
            lock: FileLockArgs {
                rule: CEPH_LOCK_FLOCK,
                ltype: CEPH_LOCK_UNLOCK,
                owner: 0,
                pid: 0,
                start: 0,
                length: 0,
                wait: false,
            },
        },
    );
    unlock.oldest_client_tid = 101;
    let unlock = input(&fs.e, C1, unlock);
    fs.e.step(Some(unlock), permit()).unwrap();
    expect_stale(&mut fs.e, &request);
}

#[test]
fn e1x_3_owned_fitting_watermark_bypasses_nonfitting_front_waiter() {
    let mut e = pressure();
    let b = input(&e, C1, retirement(301, 11));
    e.step(Some(b), None).unwrap();
    let control = noop(&e);
    let step = e.step(Some(control), permit()).unwrap();
    assert_eq!(e.waiting_requests(), 2);
    let effects = drive(&mut e, step);
    assert!(reply_for(&effects, C1, 301).is_some());
    assert!(reply_for(&effects, C1, 100).is_some());
    assert_eq!(e.waiting_requests(), 0);
    let tail = e.journal().last_seq();
    for _ in 0..3 {
        let step = e.step(Some(EngineInput::Tick), permit()).unwrap();
        assert_ne!(step.state, StepState::NeedsAdmission);
        assert_eq!(e.journal().last_seq(), tail);
    }
}

fn context_bound(limit: usize) {
    for supplied_permit in [None, permit()] {
        let mut fs = Fs::new();
        let a = node(1, "a");
        let b = node(2, "b");
        fs.call_req(C1, a.clone());
        fs.call_req(C1, b.clone());
        let mut cp = fs.e.checkpoint().unwrap();
        let s = cp.sessions.iter_mut().find(|s| s.key.client == C1).unwrap();
        for tid in 3..=limit as u64 {
            s.completed_requests
                .push(engine::journal::CompletedRequest {
                    client: C1,
                    session_incarnation: s.key.incarnation,
                    tid,
                    op: CEPH_MDS_OP_SETATTR,
                    result: 0,
                    created_ino: None,
                    oldest_client_tid: 0,
                    reply_target_ino: None,
                });
        }
        let mut cfg = config();
        cfg.retry_limits.max_completions = limit;
        let mut e = ready(cp, cfg, None);
        let first = input(&e, C1, a);
        e.step(Some(first), None).unwrap();
        let control = noop(&e);
        e.step(Some(control), permit()).unwrap();
        assert_eq!(e.waiting_requests(), 1);
        let second = input(&e, C1, b.clone());
        let supplied_limit = supplied_permit.as_ref().map(|p| p.max_record_bytes);
        let returned = match e.step(Some(second), supplied_permit) {
            Err(StepError::RetryCapacity {
                input: Some(EngineInput::Request { key: k, request }),
                permit: p,
            }) => {
                assert_eq!(k, key(&e, C1));
                assert_eq!(request, b);
                assert_eq!(p.as_ref().map(|p| p.max_record_bytes), supplied_limit);
                EngineInput::Request { key: k, request }
            }
            other => panic!("second capture exceeded the shared extra slot: {other:?}"),
        };
        assert_eq!(e.session(C1).unwrap().completed_requests.len(), limit);
        let step = e.step(None, permit()).unwrap();
        let effects = drive(&mut e, step);
        assert!(reply_for(&effects, C1, 1).is_some());
        let step = e.step(Some(returned), permit()).unwrap();
        let effects = drive(&mut e, step);
        assert!(reply_for(&effects, C1, 2).is_some());
        assert_eq!(e.waiting_requests(), 0);
    }
}

#[test]
fn e1x_2_small_limit_counts_preempted_context_before_cloning_another() {
    context_bound(2);
}

#[test]
fn e1x_2_default_limit_counts_preempted_context_before_cloning_another() {
    context_bound(65_536);
}

fn clean_seed() -> EngineCheckpoint {
    Engine::new(config(), MemJournal::new())
        .checkpoint()
        .unwrap()
}

#[test]
fn state_2_replayed_terminal_receipt_advances_active_and_receipt_highwater() {
    for (receipt, active) in [(0, 0), (100, 200), (300, 200)] {
        let mut cp = clean_seed();
        if receipt != 0 {
            cp.cleanup_receipts.push((
                SessionKey {
                    client: C3,
                    incarnation: receipt,
                },
                cp.covered_seq,
            ));
        }
        if active != 0 {
            cp.sessions.push(CheckpointSession {
                key: SessionKey {
                    client: C2,
                    incarnation: active,
                },
                completed_requests: Vec::new(),
                completed_flushes: Vec::new(),
                oldest_client_tid: 0,
                oldest_flush_tid: 0,
            });
        }
        let mut origin = ready(cp.clone(), config(), None);
        origin.open_session(C1);
        let closed = key(&origin, C1);
        let step = origin
            .step(
                Some(EngineInput::CloseSession {
                    key: closed,
                    kill: false,
                }),
                permit(),
            )
            .unwrap();
        drive(&mut origin, step);
        let (seq, record) = origin.journal().records().last().unwrap().clone();
        assert_eq!(record.closed_sessions, vec![closed]);
        assert!(record.completed_request.is_none());
        let mut replay =
            Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
        replay.replay_record(seq, &record).unwrap();
        *replay.journal_mut() = MemJournal::at_seq(seq, false);
        let replay_cp = replay.checkpoint().unwrap();
        let mut e = ready(replay_cp.clone(), config(), None);
        // Also exercise the runtime replay candidate before a checkpoint can
        // repair a stale highwater through restore.
        let mut direct =
            Engine::restore(config(), MemJournal::at_seq(1, false), clean_seed()).unwrap();
        direct.replay_record(seq, &record).unwrap();
        *direct.journal_mut() = MemJournal::at_seq(seq, false);
        let keys: Vec<SessionKey> = Vec::new();
        let step = direct.finish_reconnect(&keys).unwrap();
        drive(&mut direct, step);
        let step = direct
            .step(Some(EngineInput::RecoveryControlsDrained), permit())
            .unwrap();
        drive(&mut direct, step);
        direct.open_session(C1);
        assert!(key(&direct, C1).incarnation > closed.incarnation);
        e.open_session(C1);
        let reopened = key(&e, C1);
        assert!(
            reopened.incarnation > closed.incarnation && reopened.incarnation > receipt.max(active)
        );
        let request = input(&e, C1, node(1, "after-close"));
        let step = e.step(Some(request), permit()).unwrap();
        drive(&mut e, step);
        let mut checked = Engine::restore(
            config(),
            MemJournal::at_seq(replay_cp.covered_seq, false),
            replay_cp,
        )
        .unwrap();
        for (s, r) in e.journal().records() {
            checked.replay_record(*s, r).unwrap();
            *checked.journal_mut() = MemJournal::at_seq(*s, false);
        }
        checked.namespace_eq(&e).unwrap();
    }
}

#[test]
fn state_3_exhausted_incarnation_fails_before_insert_from_either_highwater_source() {
    for live in [false, true] {
        let mut cp = clean_seed();
        if live {
            cp.sessions.push(CheckpointSession {
                key: SessionKey {
                    client: C1,
                    incarnation: u64::MAX,
                },
                completed_requests: Vec::new(),
                completed_flushes: Vec::new(),
                oldest_client_tid: 0,
                oldest_flush_tid: 0,
            });
        } else {
            cp.cleanup_receipts.push((
                SessionKey {
                    client: C1,
                    incarnation: u64::MAX,
                },
                cp.covered_seq,
            ));
        }
        let mut e = ready(cp, config(), None);
        let tail = e.journal().last_seq();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| e.open_session(C2)));
        assert!(result.is_ok());
        assert!(e.session(C2).is_none());
        assert_eq!(e.failure(), Some(JournalError::Capacity));
        assert_eq!(e.journal().last_seq(), tail);
        let step = e.step(None, permit()).unwrap();
        assert_eq!(step.state, StepState::Failed);
        assert_eq!(step.unused_permit, permit());
        if live {
            assert_eq!(e.session(C1).unwrap().epoch, u64::MAX);
        }
    }
}

fn corrupt_completion(done: &mut engine::journal::CompletedRequest, case: &str) {
    match case {
        "open-created" => done.op = CEPH_MDS_OP_OPEN,
        "mkdir-file" => done.op = CEPH_MDS_OP_MKDIR,
        "symlink-file" => done.op = CEPH_MDS_OP_SYMLINK,
        "negative" => done.result = Errno::EIO.as_neg(),
        "setattr-created" => {
            done.op = CEPH_MDS_OP_SETATTR;
            done.reply_target_ino = None;
        }
        "unknown" => {
            done.op = 0xffff;
            done.reply_target_ino = None;
            done.created_ino = None;
        }
        _ => panic!("unknown test case"),
    }
}

#[test]
fn state_4_checked_restore_and_replay_reject_completion_contradictions_atomically() {
    let mut fs = Fs::new();
    let before = fs.e.checkpoint().unwrap();
    fs.call_req(C1, node(10, "regular"));
    let cp = fs.e.checkpoint().unwrap();
    let (seq, record) = fs.e.journal().records().last().unwrap().clone();
    for case in [
        "open-created",
        "mkdir-file",
        "symlink-file",
        "negative",
        "setattr-created",
        "unknown",
    ] {
        let mut bad = cp.clone();
        corrupt_completion(
            &mut bad
                .sessions
                .iter_mut()
                .find(|s| s.key.client == C1)
                .unwrap()
                .completed_requests[0],
            case,
        );
        assert!(
            matches!(
                Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), bad),
                Err(RestoreError::Invalid(_))
            ),
            "restore accepted {case}"
        );
        let mut bad_record = record.clone();
        corrupt_completion(bad_record.completed_request.as_mut().unwrap(), case);
        let mut e = Engine::restore(
            config(),
            MemJournal::at_seq(before.covered_seq, false),
            before.clone(),
        )
        .unwrap();
        assert!(
            matches!(
                e.replay_record(seq, &bad_record),
                Err(RestoreError::Invalid(_))
            ),
            "replay accepted {case}"
        );
        assert_eq!(e.checkpoint().unwrap(), before);
        assert_eq!(e.submitted_seq(), before.covered_seq);
        assert_eq!(e.durable_seq(), before.covered_seq);
    }
}

#[test]
fn state_4_preserves_positive_results_node_kinds_and_legacy_load_only_histories() {
    for mode in [
        S_IFREG, S_IFDIR, S_IFIFO, S_IFSOCK, S_IFCHR, S_IFBLK, S_IFLNK,
    ] {
        let mut fs = Fs::new();
        let mut req = node(10, "node");
        if let Op::Mknod { mode: m, .. } = &mut req.op {
            *m = mode | 0o644;
        }
        fs.call_req(C1, req);
        let mut cp = fs.e.checkpoint().unwrap();
        let done = &mut cp
            .sessions
            .iter_mut()
            .find(|s| s.key.client == C1)
            .unwrap()
            .completed_requests[0];
        done.result = 7;
        assert!(Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).is_ok());
    }
    let mut fs = Fs::new();
    fs.call_req(C1, node(10, "legacy"));
    let mut cp = fs.e.checkpoint().unwrap();
    cp.recovery_semantics = RecoverySemantics::Legacy2;
    let done = &mut cp
        .sessions
        .iter_mut()
        .find(|s| s.key.client == C1)
        .unwrap()
        .completed_requests[0];
    done.op = CEPH_MDS_OP_OPEN;
    done.reply_target_ino = None;
    let mut e = Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    assert!(matches!(
        e.step(Some(EngineInput::RecoveryControlsDrained), permit()),
        Err(StepError::UpgradeRequired { .. })
    ));
}

#[test]
fn state_4_semantic_validation_runs_after_terminal_retirement_on_the_final_candidate() {
    let mut fs = Fs::new();
    let before = fs.e.checkpoint().unwrap();
    let ino = fs.call_req(C1, node(10, "regular")).target.unwrap().ino;
    let (seq, record) = fs.e.journal().records().last().unwrap().clone();
    for case in ["negative", "symlink-file"] {
        let mut retired = record.clone();
        corrupt_completion(retired.completed_request.as_mut().unwrap(), case);
        retired.closed_sessions.push(key(&fs.e, C1));
        let mut e = Engine::restore(
            config(),
            MemJournal::at_seq(before.covered_seq, false),
            before.clone(),
        )
        .unwrap();
        e.replay_record(seq, &retired).unwrap();
        assert!(e.session(C1).is_none());
        assert!(e.inode(ino).is_some());
        assert_eq!(e.session_cleanup_seq(key(&fs.e, C1)), Some(seq));
    }
}

#[test]
fn state_1_and_e1x_3_owned_watermark_returns_an_older_invalid_waiter_without_unreplayable_wal() {
    let (mut e, cp) = pressure_with_checkpoint();
    let a = node(100, "waiting");
    let b = input(&e, C1, retirement(301, 101));
    e.step(Some(b), None).unwrap();
    let control = noop(&e);
    e.step(Some(control), permit()).unwrap();
    let mut returned = false;
    let mut effects = Vec::new();
    for _ in 0..64 {
        match e.step(None, permit()) {
            Ok(step) => {
                let state = step.state;
                effects.extend(step.effects);
                if matches!(state, StepState::WaitingCommit(_)) {
                    let step = commit(&mut e);
                    effects.extend(step.effects);
                }
            }
            Err(StepError::InvalidRetryIdentity {
                input: Some(EngineInput::Request { request, .. }),
                permit: p,
            }) => {
                assert_eq!(request, a);
                assert_eq!(p, permit());
                returned = true;
                break;
            }
            Err(other) => panic!("wrong owned-input result: {other}"),
        }
    }
    assert!(returned);
    assert!(reply_for(&effects, C1, 301).is_some());
    assert!(reply_for(&effects, C1, 100).is_none());
    let mut replay = Engine::restore(
        e.config().clone(),
        MemJournal::at_seq(cp.covered_seq, false),
        cp,
    )
    .unwrap();
    for (seq, record) in e.journal().records() {
        replay.replay_record(*seq, record).unwrap();
        *replay.journal_mut() = MemJournal::at_seq(*seq, false);
    }
    replay.namespace_eq(&e).unwrap();
    assert_eq!(e.waiting_requests(), 0);
}
