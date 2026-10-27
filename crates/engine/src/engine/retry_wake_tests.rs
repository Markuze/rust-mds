use super::*;
use crate::MemJournal;

const CLIENT: ClientId = 4101;
fn engine() -> Engine<MemJournal> {
    let mut e = Engine::new(EngineConfig::default(), MemJournal::new());
    e.open_session(CLIENT);
    e
}
fn request(tid: u64) -> Request {
    Request::new(
        tid,
        Op::Mknod {
            path: FilePath::child(CEPH_INO_ROOT, "owned"),
            mode: S_IFREG | 0o644,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
}
fn pending(e: &Engine<MemJournal>, req: Request) -> Pending {
    Pending {
        client: CLIENT,
        epoch: e.session_epoch(CLIENT),
        req,
        holds: Vec::new(),
        flock_was_waiting: false,
        completed: None,
        retry_reservation: (0, 0),
        started: false,
    }
}
fn key(e: &Engine<MemJournal>) -> SessionKey {
    SessionKey {
        client: CLIENT,
        incarnation: e.session_epoch(CLIENT),
    }
}

#[test]
fn epoch_overflow_preserves_invalid_owned_input_and_unused_permit_before_output() {
    for continuation in [true, false] {
        for with_permit in [false, true] {
            let mut e = engine();
            let original = request(100);
            let mut p = pending(&e, original.clone());
            p.retry_reservation = (1, 256);
            if continuation {
                e.continuation = Some(EngineInput::Request {
                    key: key(&e),
                    request: original.clone(),
                });
                e.continuation_reservation = p.retry_reservation;
            } else {
                e.parked.push_back(p);
            }
            e.observe_request_floor(CLIENT, 101);
            e.retry_capacity_epoch = u64::MAX;
            let tail = e.journal().last_seq();
            let root = e.inode(CEPH_INO_ROOT).unwrap().clone();
            let permit = with_permit.then_some(RecordPermit {
                max_record_bytes: 12345,
            });
            match e.step(None, permit) {
                Err(StepError::InvalidRetryIdentity {
                    input:
                        Some(EngineInput::Request {
                            key: k,
                            request: returned,
                        }),
                    permit: p,
                }) => {
                    assert_eq!(k, key(&e));
                    assert_eq!(returned, original);
                    assert_eq!(p.map(|p| p.max_record_bytes), with_permit.then_some(12345));
                }
                other => panic!("lost terminal ownership: {other:?}"),
            }
            assert_eq!(e.failure(), Some(JournalError::Capacity));
            assert_eq!(e.retry_capacity_epoch(), u64::MAX);
            assert_eq!(e.journal().last_seq(), tail);
            assert_eq!(e.inode(CEPH_INO_ROOT).unwrap(), &root);
            let late = e.journal_event(JournalEvent::Committed(tail));
            assert_eq!(late.state, StepState::Failed);
            assert!(late.effects.is_empty() && late.notices.is_empty());
        }
    }
}

#[test]
fn epoch_overflow_preserves_new_protected_input_and_linear_permit() {
    let mut e = engine();
    e.continuation = Some(EngineInput::Request {
        key: key(&e),
        request: request(100),
    });
    e.continuation_reservation = (1, 256);
    e.retry_capacity_epoch = u64::MAX;
    let control = EngineInput::CapRelease {
        key: key(&e),
        items: Vec::new(),
    };
    let tail = e.journal().last_seq();
    match e.step(
        Some(control.clone()),
        Some(RecordPermit {
            max_record_bytes: 12345,
        }),
    ) {
        Err(StepError::ControlCapacity {
            input: Some(returned),
            permit: Some(p),
            ..
        }) => {
            match returned {
                EngineInput::CapRelease { key: k, items } => {
                    assert_eq!(k, key(&e));
                    assert!(items.is_empty());
                }
                other => panic!("wrong returned control: {other:?}"),
            }
            assert_eq!(p.max_record_bytes, 12345);
        }
        other => panic!("protected terminal ownership: {other:?}"),
    }
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert_eq!(e.retry_capacity_epoch(), u64::MAX);
    assert_eq!(e.journal().last_seq(), tail);
}

#[test]
fn epoch_overflow_precedes_commit_materialization_and_never_wraps() {
    let mut e = Engine::new(EngineConfig::default(), MemJournal::manual());
    assert_eq!(e.retry_capacity_epoch(), 0);
    e.retry_capacity_epoch = u64::MAX;
    let seq = e.journal().last_seq();
    e.journal_mut().commit_all();
    let step = e.journal_event(JournalEvent::Committed(seq));
    assert_eq!(step.state, StepState::Failed);
    assert!(step.effects.is_empty() && step.notices.is_empty());
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert_eq!(e.retry_capacity_epoch(), u64::MAX);
    let mut e = engine();
    e.retry_capacity_epoch = u64::MAX - 1;
    assert!(e.retry_capacity_released());
    assert_eq!(e.retry_capacity_epoch(), u64::MAX);
    assert!(!e.retry_capacity_released());
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert_eq!(e.retry_capacity_epoch(), u64::MAX);
}

#[test]
fn epoch_overflow_precedes_captured_reply_and_target_pin_release() {
    let mut e = engine();
    let req = request(100);
    e.sessions
        .get_mut(&CLIENT)
        .unwrap()
        .completed_requests
        .insert(
            100,
            CompletedRequest {
                client: CLIENT,
                session_incarnation: 1,
                tid: 100,
                op: CEPH_MDS_OP_MKNOD,
                result: 0,
                created_ino: None,
                oldest_client_tid: 0,
                reply_target_ino: Some(CEPH_INO_ROOT),
            },
        );
    e.rebuild_retry_refs();
    e.retry_capacity_epoch = u64::MAX;
    let mut replay = req;
    replay.flags |= CEPH_MDS_FLAG_REPLAY;
    let tail = e.journal().last_seq();
    let step = e
        .step(
            Some(EngineInput::Request {
                key: key(&e),
                request: replay,
            }),
            Some(RecordPermit {
                max_record_bytes: 12345,
            }),
        )
        .unwrap();
    assert_eq!(step.state, StepState::Failed);
    assert!(step.effects.is_empty() && step.notices.is_empty());
    assert_eq!(step.unused_permit.unwrap().max_record_bytes, 12345);
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert_eq!(e.retry_capacity_epoch(), u64::MAX);
    assert_eq!(e.journal().last_seq(), tail);
}

#[test]
fn identity_only_pass_skips_valid_and_captured_pressure_waiters_in_fifo_order() {
    let mut e = engine();
    for tid in [300, 100, 90, 80] {
        let mut p = pending(&e, request(tid));
        if tid == 80 {
            p.completed = Some(CompletedRequest {
                client: CLIENT,
                session_incarnation: p.epoch,
                tid,
                op: CEPH_MDS_OP_MKNOD,
                result: 0,
                created_ino: None,
                oldest_client_tid: 0,
                reply_target_ino: Some(CEPH_INO_ROOT),
            });
        }
        e.pressure_wait.push_back(p);
    }
    let tail = e.journal().last_seq();
    let root = e.inode(CEPH_INO_ROOT).unwrap().clone();
    e.observe_request_floor(CLIENT, 101);
    assert!(!e.pressure_ready);
    assert!(e.invalid_retry_ready);
    assert_eq!(e.retry_capacity_epoch(), 0);
    for tid in [100, 90] {
        assert_eq!(
            e.result_step(Vec::new(), None).state,
            StepState::NeedsAdmission
        );
        match e.step(None, None) {
            Err(StepError::InvalidRetryIdentity {
                input: Some(EngineInput::Request { request, .. }),
                permit: None,
            }) => assert_eq!(request.tid, tid),
            other => panic!("identity-only FIFO: {other:?}"),
        }
        assert!(!e.pressure_ready);
        assert_eq!(e.retry_capacity_epoch(), 0);
    }
    assert!(!e.invalid_retry_ready);
    assert_eq!(
        e.result_step(Vec::new(), None).state,
        StepState::WaitingInput
    );
    assert_eq!(
        e.pressure_wait
            .iter()
            .map(|p| p.req.tid)
            .collect::<Vec<_>>(),
        vec![300, 80]
    );
    assert!(e.reply_holds_target(CEPH_INO_ROOT));
    assert_eq!(e.journal().last_seq(), tail);
    assert_eq!(e.inode(CEPH_INO_ROOT).unwrap(), &root);
    assert_eq!(e.session(CLIENT).unwrap().durable_oldest_client_tid, 0);
}

#[test]
fn pressure_resume_and_completion_and_replay_floor_sites_schedule_identity_only_returns() {
    for site in [0, 1, 2] {
        let mut e = engine();
        if site == 0 {
            let mut read = Request::new(
                200,
                Op::Getattr {
                    path: FilePath::ino(CEPH_INO_ROOT),
                    mask: 0,
                },
            );
            read.oldest_client_tid = 101;
            e.pressure_wait.push_back(pending(&e, read));
        }
        e.pressure_wait.push_back(pending(&e, request(100)));
        let done = CompletedRequest {
            client: CLIENT,
            session_incarnation: e.session_epoch(CLIENT),
            tid: 200,
            op: CEPH_MDS_OP_SETATTR,
            result: 0,
            created_ino: None,
            oldest_client_tid: 101,
            reply_target_ino: None,
        };
        match site {
            0 => {
                e.pressure_ready = true;
                let step = e
                    .step(
                        None,
                        Some(RecordPermit {
                            max_record_bytes: 16 << 20,
                        }),
                    )
                    .unwrap();
                assert!(
                    step.effects
                        .iter()
                        .any(|x| matches!(x, Effect::Reply(r) if r.tid == 200))
                );
            }
            1 => e.commit_retry_completion(done),
            _ => {
                let mut record = JournalRecord::new(RecordKind::Setattr);
                record.completed_request = Some(done);
                e.apply_record(&record);
            }
        }
        assert!(e.invalid_retry_ready);
        match e.step(None, None) {
            Err(StepError::InvalidRetryIdentity {
                input: Some(EngineInput::Request { request, .. }),
                permit: None,
            }) => assert_eq!(request.tid, 100),
            other => panic!("floor site {site}: {other:?}"),
        }
    }
}

#[test]
fn simultaneous_capacity_and_identity_wakes_keep_first_eligible_fifo() {
    let mut e = engine();
    let read = Request::new(
        300,
        Op::Getattr {
            path: FilePath::ino(CEPH_INO_ROOT),
            mask: 0,
        },
    );
    e.pressure_wait.push_back(pending(&e, read));
    e.pressure_wait.push_back(pending(&e, request(100)));
    e.observe_request_floor(CLIENT, 101);
    e.pressure_ready = true;
    let first = e
        .step(
            None,
            Some(RecordPermit {
                max_record_bytes: 16 << 20,
            }),
        )
        .unwrap();
    assert!(
        first
            .effects
            .iter()
            .any(|x| matches!(x, Effect::Reply(r) if r.tid == 300))
    );
    match e.step(None, None) {
        Err(StepError::InvalidRetryIdentity {
            input: Some(EngineInput::Request { request, .. }),
            permit: None,
        }) => assert_eq!(request.tid, 100),
        other => panic!("first eligible FIFO: {other:?}"),
    }
}
