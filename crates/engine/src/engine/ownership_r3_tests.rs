//! Terminal-capacity ownership tests. Only the scalar boundary is injected.
use super::*;
use crate::{CapReleaseItem, CapUpdate, MemJournal};

const C1: ClientId = 4101;
const C2: ClientId = 4102;
fn permit() -> Option<RecordPermit> {
    Some(RecordPermit {
        max_record_bytes: 16 << 20,
    })
}
fn key(e: &Engine<MemJournal>, client: ClientId) -> SessionKey {
    SessionKey {
        client,
        incarnation: e.session_epoch(client),
    }
}
fn node(tid: u64, name: &str) -> Request {
    Request::new(
        tid,
        Op::Mknod {
            path: FilePath::child(CEPH_INO_ROOT, name),
            mode: S_IFREG | 0o644,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_flags(CEPH_MDS_FLAG_WANT_DENTRY)
}
fn input(e: &Engine<MemJournal>, client: ClientId, request: Request) -> EngineInput {
    EngineInput::Request {
        key: key(e, client),
        request,
    }
}
fn engine() -> Engine<MemJournal> {
    let mut e = Engine::new(EngineConfig::default(), MemJournal::new());
    e.open_session(C1);
    e.open_session(C2);
    e
}
fn drain(e: &mut Engine<MemJournal>, mut step: EngineStep) -> Vec<Effect> {
    let mut effects = Vec::new();
    for _ in 0..128 {
        effects.extend(step.effects);
        step = match step.state {
            StepState::NeedsAdmission => e.step(None, permit()).unwrap(),
            StepState::WaitingCommit(_) => {
                let seq = e.journal().last_seq();
                e.journal_mut().commit_all();
                e.journal_event(JournalEvent::Committed(seq))
            }
            StepState::Idle | StepState::WaitingInput => return effects,
            StepState::Failed => panic!("unexpected failure"),
        };
    }
    panic!("unbounded fixture drain");
}
fn call(e: &mut Engine<MemJournal>, request: Request) -> Vec<Effect> {
    let step = e.step(Some(input(e, C1, request)), permit()).unwrap();
    drain(e, step)
}
fn restore(cp: EngineCheckpoint) -> Engine<MemJournal> {
    let mut e = Engine::restore(
        EngineConfig::default(),
        MemJournal::at_seq(cp.covered_seq, false),
        cp,
    )
    .unwrap();
    let keys = [key(&e, C1), key(&e, C2)];
    for key in keys {
        e.reconnect_chunk(key, &[], &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drain(&mut e, step);
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    drain(&mut e, step);
    e
}
fn capture(e: &mut Engine<MemJournal>, request: Request) {
    e.step(Some(input(e, C1, request)), None).unwrap();
    e.step(
        Some(EngineInput::CapRelease {
            key: key(e, C2),
            items: Vec::new(),
        }),
        permit(),
    )
    .unwrap();
}
fn caps(e: &Engine<MemJournal>) -> Vec<(Ino, Vec<(ClientId, crate::caps::Capability)>)> {
    e.inode_numbers()
        .into_iter()
        .map(|ino| (ino, e.caps_of(ino)))
        .collect()
}
fn assert_failed(
    e: &mut Engine<MemJournal>,
    tail: u64,
    before: &[(Ino, Vec<(ClientId, crate::caps::Capability)>)],
) {
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert!(e.capacity_failure.is_some());
    assert_eq!(e.journal().last_seq(), tail);
    assert_eq!(caps(e), before);
    let late = e.journal_event(JournalEvent::Committed(tail));
    assert_eq!(late.state, StepState::Failed);
    assert!(late.effects.is_empty() && late.notices.is_empty());
    assert_eq!(e.journal().last_seq(), tail);
    assert_eq!(caps(e), before);
}
fn close_case(already_owned: bool, kill: bool) {
    let mut e = engine();
    let original = node(1, "captured");
    call(&mut e, original.clone());
    let mut e = restore(e.checkpoint().unwrap());
    capture(&mut e, original.clone());
    let close_key = key(&e, C1);
    if already_owned {
        let step = e
            .step(
                Some(EngineInput::CloseSession {
                    key: close_key,
                    kill,
                }),
                None,
            )
            .unwrap();
        assert_eq!(step.state, StepState::NeedsAdmission);
    }
    let tail = e.journal().last_seq();
    let before = caps(&e);
    e.retry_capacity_epoch = u64::MAX;
    let supplied = (!already_owned).then_some(EngineInput::CloseSession {
        key: close_key,
        kill,
    });
    match e.step(
        supplied,
        Some(RecordPermit {
            max_record_bytes: 12345,
        }),
    ) {
        Err(StepError::ControlCapacity {
            input:
                Some(EngineInput::CloseSession {
                    key,
                    kill: returned,
                }),
            permit: Some(p),
            ..
        }) => {
            assert_eq!(key, close_key);
            assert_eq!(returned, kill);
            assert_eq!(p.max_record_bytes, 12345);
        }
        other => panic!("lost close ownership: {other:?}"),
    }
    assert_failed(&mut e, tail, &before);
    assert!(e.session(C1).is_some());
    assert_eq!(e.session_cleanup_seq(close_key), None);
    assert_eq!(e.pressure_wait.len(), 1);
    assert_eq!(e.pressure_wait[0].req, original);
    assert!(e.pressure_wait[0].completed.is_some());
    assert!(
        e.reply_holds_target(
            e.pressure_wait[0]
                .completed
                .as_ref()
                .unwrap()
                .reply_target_ino
                .unwrap()
        )
    );
    let again = e.step(None, permit()).unwrap();
    assert_eq!(again.state, StepState::Failed);
    assert_eq!(again.unused_permit, permit());
    assert!(again.effects.is_empty() && again.notices.is_empty());
}

#[test]
fn r3_terminal_new_kill_close_returns_exact_control_and_preserves_captured_work() {
    close_case(false, true);
}
#[test]
fn r3_terminal_owned_graceful_close_returns_exact_control_and_linear_permit() {
    close_case(true, false);
}

#[test]
fn r3_callback_capacity_retains_commit_requests_and_remaining_control_cursor() {
    let mut e = engine();
    let mut files = Vec::new();
    for (tid, name) in [(1, "a"), (3, "b")] {
        let effects = call(&mut e, node(tid, name));
        let ino = effects
            .iter()
            .find_map(|x| match x {
                Effect::Reply(r) => r.target.as_ref().map(|t| t.ino),
                _ => None,
            })
            .unwrap();
        call(
            &mut e,
            Request::new(
                tid + 1,
                Op::Open {
                    path: FilePath::ino(ino),
                    flags: CEPH_O_RDWR,
                    mode: 0,
                    mask: 0,
                    old_size: 0,
                },
            ),
        );
        files.push(ino);
    }
    let owned = node(100, "unacknowledged");
    e.step(Some(input(&e, C1, owned.clone())), None).unwrap();
    let release_key = key(&e, C1);
    let items: Vec<_> = [CEPH_INO_ROOT, files[0], files[1]]
        .into_iter()
        .map(|ino| {
            let cap = e.cap(ino, C1).unwrap();
            CapReleaseItem {
                ino,
                cap_id: cap.cap_id,
                mseq: cap.mseq,
                issue_seq: cap.last_issue,
            }
        })
        .collect();
    let step = e
        .step(
            Some(EngineInput::CapRelease {
                key: release_key,
                items: items.clone(),
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(step.state, StepState::NeedsAdmission);
    let step = e.step(None, permit()).unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(
        e.release_cursor.iter().copied().collect::<Vec<_>>(),
        vec![items[2]]
    );
    let tail = e.journal().last_seq();
    let before = caps(&e);
    let commit_seq = e.commits[0].seq;
    let images = e.commits[0].images.clone();
    let jobs = e.commits[0].jobs.clone();
    e.retry_capacity_epoch = u64::MAX;
    e.journal_mut().commit_all();
    let failed = e.journal_event(JournalEvent::Committed(tail));
    assert_eq!(failed.state, StepState::Failed);
    assert!(failed.effects.is_empty() && failed.notices.is_empty());
    assert_eq!(e.commits.len(), 1);
    assert_eq!(e.commits[0].seq, commit_seq);
    assert_eq!(e.commits[0].images, images);
    assert_eq!(e.commits[0].jobs, jobs);
    assert_eq!(e.pressure_wait.len(), 1);
    assert_eq!(e.pressure_wait[0].req, owned);
    let fresh_items = vec![CapReleaseItem {
        ino: files[1],
        cap_id: 999,
        mseq: 17,
        issue_seq: 19,
    }];
    match e.step(
        Some(EngineInput::CapRelease {
            key: key(&e, C2),
            items: fresh_items.clone(),
        }),
        permit(),
    ) {
        Err(StepError::ControlCapacity {
            input:
                Some(EngineInput::CapRelease {
                    key: k,
                    items: returned,
                }),
            permit: p,
            ..
        }) => {
            assert_eq!(k, key(&e, C2));
            assert_eq!(returned, fresh_items);
            assert_eq!(p, permit());
        }
        other => panic!("fresh release consumed old cursor: {other:?}"),
    }
    let dirty = CapUpdate {
        op: CEPH_CAP_OP_FLUSH,
        ino: CEPH_INO_ROOT,
        flush_tid: 77,
        dirty: CEPH_CAP_XATTR_EXCL,
        xattrs: Some((8, XattrMap::from([(b"user.keep".to_vec(), vec![7; 16])]))),
        ..Default::default()
    };
    match e.step(
        Some(EngineInput::CapUpdate {
            key: release_key,
            update: dirty.clone(),
        }),
        permit(),
    ) {
        Err(StepError::ControlCapacity {
            input: Some(EngineInput::CapUpdate { key, update }),
            permit: p,
            ..
        }) => {
            assert_eq!(key, release_key);
            assert_eq!(update, dirty);
            assert_eq!(p, permit());
        }
        other => panic!("later dirty input lost: {other:?}"),
    }
    assert_eq!(
        e.release_cursor.iter().copied().collect::<Vec<_>>(),
        vec![items[2]]
    );
    match e.step(
        None,
        Some(RecordPermit {
            max_record_bytes: 54321,
        }),
    ) {
        Err(StepError::ControlCapacity {
            input:
                Some(EngineInput::CapRelease {
                    key,
                    items: returned,
                }),
            permit: Some(p),
            ..
        }) => {
            assert_eq!(key, release_key);
            assert_eq!(returned, vec![items[2]]);
            assert_eq!(p.max_record_bytes, 54321);
        }
        other => panic!("remaining owned cursor lost: {other:?}"),
    }
    assert!(e.continuation.is_none() && e.release_cursor.is_empty());
    let again = e.step(None, permit()).unwrap();
    assert_eq!(again.state, StepState::Failed);
    assert_eq!(again.unused_permit, permit());
    assert_failed(&mut e, tail, &before);
    assert_eq!(e.commits.len(), 1);
    assert_eq!(e.pressure_wait[0].req, owned);
    assert_eq!(e.session_cleanup_seq(release_key), None);
}

#[test]
fn r3_terminal_batch_failure_retains_all_captures_and_other_clients_input() {
    let mut e = engine();
    let first = node(1, "first");
    let second = node(2, "second");
    call(&mut e, first.clone());
    call(&mut e, second.clone());
    let mut e = restore(e.checkpoint().unwrap());
    capture(&mut e, first.clone());
    capture(&mut e, second.clone());
    let other = node(100, "other-client");
    e.step(Some(input(&e, C2, other.clone())), None).unwrap();
    e.retry_capacity_epoch = u64::MAX - 1;
    let tail = e.journal().last_seq();
    let before = caps(&e);
    let close_key = key(&e, C1);
    match e.step(
        Some(EngineInput::CloseSession {
            key: close_key,
            kill: true,
        }),
        permit(),
    ) {
        Err(StepError::ControlCapacity {
            input: Some(EngineInput::CloseSession { key, kill: true }),
            permit: p,
            ..
        }) => {
            assert_eq!(key, close_key);
            assert_eq!(p, permit());
        }
        other => panic!("batch close lost ownership: {other:?}"),
    }
    assert_eq!(e.retry_capacity_epoch(), u64::MAX);
    assert_failed(&mut e, tail, &before);
    assert_eq!(
        e.pressure_wait
            .iter()
            .map(|p| (p.client, p.req.clone()))
            .collect::<Vec<_>>(),
        vec![(C1, first), (C1, second), (C2, other)]
    );
    assert_eq!(e.owned_completion_contexts(), Some(2));
    assert_eq!(e.session_cleanup_seq(close_key), None);
}

#[test]
fn r3_nonjournaled_finish_failure_retains_the_unacknowledged_request() {
    let mut e = engine();
    e.config.read_only = true;
    e.retry_capacity_epoch = u64::MAX;
    let request = node(100, "not-created");
    let tail = e.journal().last_seq();
    let before = caps(&e);
    let step = e
        .step(Some(input(&e, C1, request.clone())), permit())
        .unwrap();
    assert_eq!(step.state, StepState::Failed);
    assert_eq!(step.unused_permit, permit());
    assert!(step.effects.is_empty());
    assert_eq!(e.parked.len(), 1);
    assert_eq!(e.parked[0].req, request);
    assert_failed(&mut e, tail, &before);
}

#[test]
fn r3_immediate_compatibility_reply_failure_retains_both_owned_inputs() {
    let mut e = engine();
    let original = node(1, "recorded");
    call(&mut e, original.clone());
    let mut e = restore(e.checkpoint().unwrap());
    let owned = node(100, "owned");
    e.step(Some(input(&e, C1, owned.clone())), None).unwrap();
    e.retry_capacity_epoch = u64::MAX;
    let tail = e.journal().last_seq();
    let before = caps(&e);
    let mut replay = original;
    replay.flags |= CEPH_MDS_FLAG_REPLAY;
    assert!(e.handle_request(C1, replay.clone()).is_empty());
    assert_eq!(e.parked.len(), 1);
    assert_eq!(e.parked[0].req, replay);
    assert!(e.parked[0].completed.is_some());
    match e.step(None, permit()) {
        Err(StepError::ControlCapacity {
            input: Some(EngineInput::Request { request, .. }),
            permit: p,
            ..
        }) => {
            assert_eq!(request, owned);
            assert_eq!(p, permit());
        }
        other => panic!("other continuation was lost: {other:?}"),
    }
    assert_failed(&mut e, tail, &before);
    assert_eq!(e.parked[0].req, replay);
}

#[test]
fn r3_noncapacity_failure_keeps_the_existing_cleanup_behavior() {
    let mut e = engine();
    e.step(Some(input(&e, C1, node(100, "owned"))), None)
        .unwrap();
    let failed = e.journal_event(JournalEvent::Failed(JournalError::Unavailable));
    assert_eq!(failed.state, StepState::Failed);
    assert_eq!(e.failure(), Some(JournalError::Unavailable));
    assert!(
        e.capacity_failure.is_none()
            && e.continuation.is_none()
            && e.parked.is_empty()
            && e.pressure_wait.is_empty()
    );
    let late = e
        .step(
            Some(EngineInput::CloseSession {
                key: key(&e, C1),
                kill: true,
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(late.state, StepState::Failed);
    assert_eq!(late.unused_permit, permit());
}
