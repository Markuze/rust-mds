//! Capacity release observation and identity-only wake regressions.
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
    .with_flags(CEPH_MDS_FLAG_WANT_DENTRY)
}
fn input(e: &Engine<MemJournal>, request: Request) -> EngineInput {
    EngineInput::Request {
        key: key(e, C1),
        request,
    }
}
fn commit(e: &mut Engine<MemJournal>) -> EngineStep {
    let seq = e.journal().last_seq();
    e.journal_mut().commit_all();
    let out = e.journal_event(JournalEvent::Committed(seq));
    assert_eq!(e.journal().last_seq(), seq);
    out
}
fn drain(e: &mut Engine<MemJournal>, mut step: EngineStep) -> Vec<Effect> {
    let mut out = Vec::new();
    for _ in 0..256 {
        out.extend(step.effects);
        step = match step.state {
            StepState::NeedsAdmission => e.step(None, permit()).unwrap(),
            StepState::WaitingCommit(_) => commit(e),
            StepState::Idle | StepState::WaitingInput => return out,
            StepState::Failed => panic!("engine failed"),
        };
    }
    panic!("unbounded owned drain");
}
fn restore(cp: EngineCheckpoint) -> Engine<MemJournal> {
    let mut e = Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    assert_eq!(e.retry_capacity_epoch(), 0);
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
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drain(&mut e, step);
    let before = e.retry_capacity_epoch();
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    assert_eq!(
        e.retry_capacity_epoch(),
        before,
        "grant marker is not a release"
    );
    drain(&mut e, step);
    e
}
fn noop(e: &Engine<MemJournal>) -> EngineInput {
    EngineInput::CapRelease {
        key: key(e, C1),
        items: Vec::new(),
    }
}
fn clear_caps(e: &mut Engine<MemJournal>, client: ClientId) {
    let caps: Vec<_> = e.session(client).unwrap().caps.iter().copied().collect();
    for ino in caps {
        let cap = e.cap(ino, client).unwrap();
        let item = CapReleaseItem {
            ino,
            cap_id: cap.cap_id,
            mseq: cap.mseq,
            issue_seq: cap.last_issue,
        };
        let step = e
            .step(
                Some(EngineInput::CapRelease {
                    key: key(e, client),
                    items: vec![item],
                }),
                permit(),
            )
            .unwrap();
        drain(e, step);
    }
}

#[test]
fn zero_instance_baseline_getter_and_checkpoint_export_do_not_create_credit() {
    let e = Engine::new(config(), MemJournal::new());
    assert_eq!(e.retry_capacity_epoch(), 0);
    let cp = e.checkpoint().unwrap();
    assert_eq!(e.retry_capacity_epoch(), 0);
    let restored = Engine::restore(
        config(),
        MemJournal::at_seq(cp.covered_seq, false),
        cp.clone(),
    )
    .unwrap();
    assert_eq!(restored.retry_capacity_epoch(), 0);
    assert_eq!(restored.retry_capacity_epoch(), 0);
    let mut legacy = cp;
    legacy.recovery_semantics = RecoverySemantics::Legacy2;
    let legacy = Engine::restore(
        config(),
        MemJournal::at_seq(legacy.covered_seq, false),
        legacy,
    )
    .unwrap();
    legacy.checkpoint_for_retry_upgrade().unwrap();
    assert_eq!(legacy.retry_capacity_epoch(), 0);
}

#[test]
fn empty_revalidation_epoch_distinguishes_release_without_another_pressure_waiter() {
    let mut observed = Vec::new();
    for parent_bytes in [1024, 1536 * 1024] {
        let mut cfg = config();
        cfg.max_xattr_size = 8 << 20;
        cfg.max_record_bytes = 16 << 20;
        let mut fs = Fs::with(cfg, MemJournal::manual());
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
        let cap = fs.e.cap(ROOT, C2).unwrap();
        let grant = EngineInput::CapUpdate {
            key: key(&fs.e, C2),
            update: CapUpdate {
                ino: ROOT,
                cap_id: cap.cap_id,
                seq: cap.last_sent,
                issue_seq: cap.last_issue,
                caps: cap.pending(),
                wanted: XX | FS,
                ..Default::default()
            },
        };
        let step = fs.e.step(Some(grant), permit()).unwrap();
        drain(&mut fs.e, step);
        let mut request = node(100, "large-waiter");
        if let Op::Mknod { xattrs, .. } = &mut request.op {
            xattrs.insert(b"user.large".to_vec(), vec![1; 3 << 20]);
        }
        let step = fs.e.step(Some(input(&fs.e, request)), permit()).unwrap();
        assert!(reply_for(&step.effects, C1, 100).is_none());
        assert_eq!(fs.e.waiting_requests(), 1);
        let cap = fs.e.cap(ROOT, C2).unwrap();
        let flush = EngineInput::CapUpdate {
            key: key(&fs.e, C2),
            update: CapUpdate {
                op: CEPH_CAP_OP_FLUSH,
                ino: ROOT,
                cap_id: cap.cap_id,
                seq: cap.last_sent.wrapping_sub(1),
                issue_seq: cap.last_issue,
                caps: cap.issued(),
                wanted: XX | FS,
                dirty: XX,
                flush_tid: 1,
                xattrs: Some((
                    2,
                    XattrMap::from([(b"user.parent".to_vec(), vec![2; parent_bytes])]),
                )),
                ..Default::default()
            },
        };
        let step = fs.e.step(Some(flush), permit()).unwrap();
        assert!(matches!(step.state, StepState::WaitingCommit(_)));
        let mut step = commit(&mut fs.e);
        let mut delta = None;
        for _ in 0..16 {
            assert_eq!(step.state, StepState::NeedsAdmission);
            let before = fs.e.retry_capacity_epoch();
            step = fs.e.step(None, permit()).unwrap();
            if step.state != StepState::NeedsAdmission {
                delta = Some(fs.e.retry_capacity_epoch() - before);
                break;
            }
        }
        let public = (
            step.state,
            step.effects.len(),
            step.notices.len(),
            step.unused_permit.as_ref().map(|p| p.max_record_bytes),
        );
        assert_eq!(public, (StepState::WaitingInput, 0, 0, Some(16 << 20)));
        assert_eq!(delta, Some(u64::from(parent_bytes != 1024)));
        assert_eq!(
            fs.e.waiting_requests(),
            1,
            "no second internal waiter supplies the wake"
        );
        let next = input(&fs.e, node(101, "small-successor"));
        let next = fs.e.step(Some(next), permit());
        assert_eq!(
            matches!(next, Err(StepError::Busy { .. })),
            parent_bytes == 1024
        );
        println!("parent_bytes={parent_bytes}, empty_wait={public:?}, release_delta={delta:?}");
        observed.push((public, delta));
    }
    assert_eq!(observed[0].0, observed[1].0);
}

#[test]
fn protected_revocation_observes_nonzero_continuation_reservation_without_other_waiters() {
    let mut fs = Fs::new();
    fs.e.step(Some(input(&fs.e, node(10, "owned"))), None)
        .unwrap();
    let before = fs.e.retry_capacity_epoch();
    let step = fs.e.step(Some(noop(&fs.e)), permit()).unwrap();
    assert!(fs.e.retry_capacity_epoch() > before);
    assert!(step.effects.is_empty());
    assert_eq!(fs.e.waiting_requests(), 1);
}

#[test]
fn nonjournaled_ordinary_completion_releases_its_admitted_reservation() {
    let mut cfg = config();
    cfg.read_only = true;
    let mut e = Engine::new(cfg, MemJournal::new());
    e.open_session(C1);
    let before = e.retry_capacity_epoch();
    let tail = e.journal().last_seq();
    let step = e
        .step(Some(input(&e, node(10, "read-only"))), permit())
        .unwrap();
    assert_eq!(
        reply_for(&step.effects, C1, 10).unwrap().result,
        Errno::EROFS.as_neg()
    );
    assert!(e.retry_capacity_epoch() > before);
    assert_eq!(e.journal().last_seq(), tail);
    assert_eq!(step.unused_permit, permit());
}

#[test]
fn captured_transfer_is_not_release_but_terminal_drop_and_immediate_reply_are() {
    let mut fs = Fs::new();
    let request = node(10, "captured");
    fs.call_req(C1, request.clone());
    let cp = fs.e.checkpoint().unwrap();
    let mut e = restore(cp.clone());
    e.step(Some(input(&e, request.clone())), None).unwrap();
    let before = e.retry_capacity_epoch();
    e.step(Some(noop(&e)), permit()).unwrap();
    assert_eq!(
        e.retry_capacity_epoch(),
        before,
        "same context moved to pressure"
    );
    let step = e
        .step(
            Some(EngineInput::CloseSession {
                key: key(&e, C1),
                kill: false,
            }),
            permit(),
        )
        .unwrap();
    assert!(
        e.retry_capacity_epoch() > before,
        "terminal drop releases the context before its cleanup commit"
    );
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    drain(&mut e, step);
    let mut e = restore(cp);
    let before = e.retry_capacity_epoch();
    let tail = e.journal().last_seq();
    let step = e.step(Some(input(&e, request.clone())), permit()).unwrap();
    assert!(reply_for(&step.effects, C1, 10).is_some());
    assert!(e.retry_capacity_epoch() > before);
    assert_eq!(e.journal().last_seq(), tail);
    let mut replay = request;
    replay.flags |= CEPH_MDS_FLAG_REPLAY;
    let before = e.retry_capacity_epoch();
    let step = e.step(Some(input(&e, replay)), permit()).unwrap();
    assert!(reply_for(&step.effects, C1, 10).is_some());
    assert!(e.retry_capacity_epoch() > before);
    assert_eq!(e.journal().last_seq(), tail);
}

#[test]
fn captured_range_transaction_transfers_until_materialization_and_duplicate_callback_is_not_release()
 {
    let mut fs = Fs::new();
    let ino = fs.call_req(C1, node(1, "range")).target.unwrap().ino;
    let open = Request::new(
        2,
        Op::Open {
            path: FilePath::ino(ino),
            flags: CEPH_O_RDWR,
            mode: 0,
            mask: 0,
            old_size: 0,
        },
    );
    fs.call_req(C1, open.clone());
    clear_caps(&mut fs.e, C1);
    let mut e = restore(fs.e.checkpoint().unwrap());
    let before = e.retry_capacity_epoch();
    let step = e.step(Some(input(&e, open)), permit()).unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(
        e.retry_capacity_epoch(),
        before,
        "capture moved to the range record"
    );
    let seq = e.journal().last_seq();
    let step = commit(&mut e);
    assert!(reply_for(&step.effects, C1, 2).is_some());
    assert!(e.retry_capacity_epoch() > before);
    let after = e.retry_capacity_epoch();
    let duplicate = e.journal_event(JournalEvent::Committed(seq));
    assert!(duplicate.effects.is_empty());
    assert_eq!(e.retry_capacity_epoch(), after);
}

#[test]
fn canonical_retirement_referenced_metadata_shrink_and_terminal_commit_publish_release() {
    let mut fs = Fs::new();
    let mut create = node(10, "retained");
    if let Op::Mknod { xattrs, .. } = &mut create.op {
        xattrs.insert(b"user.large".to_vec(), vec![1; 4096]);
    }
    let ino = fs.call_req(C1, create).target.unwrap().ino;
    let mut e = restore(fs.e.checkpoint().unwrap());
    let before = e.retry_capacity_epoch();
    let remove = Request::new(
        20,
        Op::Rmxattr {
            path: FilePath::ino(ino),
            name: b"user.large".to_vec(),
        },
    );
    let step = e.step(Some(input(&e, remove)), permit()).unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(e.retry_capacity_epoch(), before);
    let step = commit(&mut e);
    assert!(e.retry_capacity_epoch() > before);
    drain(&mut e, step);
    let mut retire = Request::new(
        30,
        Op::Setattr {
            path: FilePath::ino(ROOT),
            args: SetattrArgs::default(),
        },
    );
    retire.oldest_client_tid = 21;
    let before = e.retry_capacity_epoch();
    let step = e.step(Some(input(&e, retire)), permit()).unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(e.retry_capacity_epoch(), before);
    let step = commit(&mut e);
    assert!(e.retry_capacity_epoch() > before);
    assert!(!e.session(C1).unwrap().completed_requests.contains_key(&10));
    drain(&mut e, step);
    clear_caps(&mut e, C1);
    let before = e.retry_capacity_epoch();
    let step = e
        .step(
            Some(EngineInput::CloseSession {
                key: key(&e, C1),
                kill: false,
            }),
            permit(),
        )
        .unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(e.retry_capacity_epoch(), before);
    let step = commit(&mut e);
    assert!(e.retry_capacity_epoch() > before);
    assert!(e.session(C1).is_none());
    drain(&mut e, step);
}

#[test]
fn owned_identity_return_releases_reservation_with_no_permit_or_pressure_queue() {
    let mut fs = Fs::new();
    let original = node(100, "never-created");
    fs.e.step(Some(input(&fs.e, original.clone())), None)
        .unwrap();
    let mut read = Request::new(
        200,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    read.oldest_client_tid = 101;
    let before = fs.e.retry_capacity_epoch();
    fs.e.handle_request(C1, read);
    assert_eq!(
        fs.e.retry_capacity_epoch(),
        before,
        "observed floor alone is not release"
    );
    let tail = fs.e.journal().last_seq();
    match fs.e.step(None, None) {
        Err(StepError::InvalidRetryIdentity {
            input: Some(EngineInput::Request { request, .. }),
            permit: None,
        }) => assert_eq!(request, original),
        other => panic!("owned no-permit return: {other:?}"),
    }
    assert!(fs.e.retry_capacity_epoch() > before);
    assert_eq!(fs.e.journal().last_seq(), tail);
    assert_eq!(fs.e.session(C1).unwrap().durable_oldest_client_tid, 0);
}

fn wide_layout(tid: u64) -> Request {
    use std::fmt::Write;
    let mut value = String::from("{");
    for n in 0..40_000 {
        if n != 0 {
            value.push(',');
        }
        write!(&mut value, "\"k{n}\":0").unwrap();
    }
    value.push('}');
    Request::new(
        tid,
        Op::Setxattr {
            path: FilePath::ino(ROOT),
            name: b"ceph.dir.layout.json".to_vec(),
            value: value.into_bytes(),
            flags: 0,
        },
    )
}

#[test]
fn rejected_preflight_permits_ticks_read_only_floor_busy_and_getters_do_not_supply_credit() {
    let mut fs = Fs::new();
    let before = fs.e.retry_capacity_epoch();
    assert!(matches!(
        fs.e.step(Some(input(&fs.e, wide_layout(100))), permit()),
        Err(StepError::RetryCapacity { .. })
    ));
    assert_eq!(fs.e.retry_capacity_epoch(), before);
    let mut read = Request::new(
        200,
        Op::Getattr {
            path: FilePath::ino(ROOT),
            mask: 0,
        },
    );
    read.oldest_client_tid = 100;
    let step = fs.e.step(Some(input(&fs.e, read)), permit()).unwrap();
    assert!(reply_for(&step.effects, C1, 200).is_some());
    assert_eq!(fs.e.retry_capacity_epoch(), before);
    drain(&mut fs.e, step);
    for _ in 0..3 {
        let step = fs.e.step(Some(EngineInput::Tick), permit()).unwrap();
        drain(&mut fs.e, step);
        assert_eq!(fs.e.retry_capacity_epoch(), before);
    }
    let step = fs.e.step(None, permit()).unwrap();
    drain(&mut fs.e, step);
    assert_eq!(fs.e.retry_capacity_epoch(), before);
    fs.e.journal_mut().set_auto_commit(false);
    let step =
        fs.e.step(Some(input(&fs.e, node(300, "commit"))), permit())
            .unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(fs.e.retry_capacity_epoch(), before);
    assert!(matches!(
        fs.e.step(Some(EngineInput::Tick), permit()),
        Err(StepError::Busy { .. })
    ));
    assert_eq!(fs.e.retry_capacity_epoch(), before);
}

#[test]
fn bounded_external_consumer_attempts_each_waiter_once_per_observed_epoch() {
    let mut fs = Fs::new();
    let mut waiters = Vec::new();
    let mut attempted = Vec::new();
    for tid in [100, 101] {
        match fs.e.step(Some(input(&fs.e, wide_layout(tid))), permit()) {
            Err(StepError::RetryCapacity {
                input: Some(input),
                permit: p,
            }) => {
                assert_eq!(p, permit());
                waiters.push((input, fs.e.retry_capacity_epoch()));
                attempted.push((tid, fs.e.retry_capacity_epoch()));
            }
            other => panic!("initial external pressure: {other:?}"),
        }
    }
    let pass = |e: &mut Engine<MemJournal>,
                waiters: &mut Vec<(EngineInput, u64)>,
                attempted: &mut Vec<(u64, u64)>| {
        let now = e.retry_capacity_epoch();
        let mut next = Vec::new();
        for (owned, last) in waiters.drain(..) {
            if last == now {
                next.push((owned, last));
                continue;
            }
            let tid = match &owned {
                EngineInput::Request { request, .. } => request.tid,
                _ => panic!("request waiter"),
            };
            attempted.push((tid, now));
            match e.step(Some(owned), permit()) {
                Err(StepError::RetryCapacity {
                    input: Some(returned),
                    permit: p,
                }) => {
                    assert_eq!(p, permit());
                    next.push((returned, now));
                }
                other => panic!("external reattempt: {other:?}"),
            }
        }
        *waiters = next;
    };
    for _ in 0..3 {
        let step = fs.e.step(Some(EngineInput::Tick), permit()).unwrap();
        drain(&mut fs.e, step);
        pass(&mut fs.e, &mut waiters, &mut attempted);
    }
    assert_eq!(attempted.len(), 2);
    fs.e.journal_mut().set_auto_commit(false);
    let step =
        fs.e.step(Some(input(&fs.e, node(10, "real-release"))), permit())
            .unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    let step = commit(&mut fs.e);
    drain(&mut fs.e, step);
    pass(&mut fs.e, &mut waiters, &mut attempted);
    pass(&mut fs.e, &mut waiters, &mut attempted);
    assert_eq!(attempted.len(), 4);
    assert_eq!((attempted[2].0, attempted[3].0), (100, 101));
    let unique: std::collections::BTreeSet<_> = attempted.iter().copied().collect();
    assert_eq!(unique.len(), attempted.len());
}

fn pressure() -> Engine<MemJournal> {
    let mut fs = Fs::new();
    let mut r = node(10, "f");
    if let Op::Mknod { xattrs, .. } = &mut r.op {
        xattrs.insert(b"user.k".to_vec(), Vec::new());
    }
    let ino = fs.call_req(C1, r).target.unwrap().ino;
    let cp = fs.e.checkpoint().unwrap();
    let mut cfg = config();
    cfg.retry_limits.max_bytes = 256 + 4096 + 272 + 1 + 2 * (304 + 6) + 8192 + 1024;
    let mut e = Engine::restore(cfg, MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    let keys: Vec<_> = [C1, C2, C3].into_iter().map(|c| key(&e, c)).collect();
    for k in &keys {
        let caps = if k.client == C1 {
            vec![ReconnectCap {
                ino,
                cap_id: 77,
                wanted: XX,
                issued: XX | P,
                realm: ROOT,
                snap_follows: 0,
                fcntl_locks: Vec::new(),
                flock_locks: Vec::new(),
            }]
        } else {
            Vec::new()
        };
        e.reconnect_chunk(*k, &caps, &[], false).unwrap();
    }
    let step = e.finish_reconnect(&keys).unwrap();
    drain(&mut e, step);
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    drain(&mut e, step);
    e.step(Some(input(&e, node(100, "waiting"))), None).unwrap();
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
    drain(&mut e, step);
    assert_eq!(
        e.step(None, permit()).unwrap().state,
        StepState::WaitingInput
    );
    e
}

#[test]
fn read_only_pressure_invalidation_returns_without_permit_and_changes_no_durable_or_cap_state() {
    let mut cap_contrasts = Vec::new();
    for floor in [99, 101] {
        let mut e = pressure();
        let tail = e.journal().last_seq();
        assert_eq!(tail, 3);
        let before = e.retry_capacity_epoch();
        let root = e.inode(ROOT).unwrap().clone();
        let mut read = Request::new(
            200,
            Op::Getattr {
                path: FilePath::ino(ROOT),
                mask: 0,
            },
        );
        read.oldest_client_tid = floor;
        let step = e.step(Some(input(&e, read)), permit()).unwrap();
        assert!(reply_for(&step.effects, C1, 200).is_some());
        assert_eq!(e.retry_capacity_epoch(), before);
        assert_eq!(e.journal().last_seq(), tail);
        assert_eq!(e.session(C1).unwrap().durable_oldest_client_tid, 0);
        assert!(e.session(C1).unwrap().completed_requests.contains_key(&10));
        assert_eq!(e.inode(ROOT).unwrap(), &root);
        let caps: Vec<_> = e
            .inode_numbers()
            .into_iter()
            .map(|ino| (ino, e.caps_of(ino)))
            .collect();
        cap_contrasts.push(caps.clone());
        let mut state = step.state;
        let mut returned = false;
        for _ in 0..16 {
            if state != StepState::NeedsAdmission {
                break;
            }
            match e.step(None, if floor == 101 { None } else { permit() }) {
                Err(StepError::InvalidRetryIdentity {
                    input: Some(EngineInput::Request { request, .. }),
                    permit: None,
                }) => {
                    assert_eq!(request, node(100, "waiting"));
                    returned = true;
                    break;
                }
                Ok(next) => state = next.state,
                other => panic!("identity wake: {other:?}"),
            }
        }
        assert_eq!(returned, floor == 101);
        if floor == 99 {
            assert_eq!(state, StepState::WaitingInput);
        }
        assert_eq!(
            e.retry_capacity_epoch(),
            before,
            "zero-reservation return releases no charged resource"
        );
        assert_eq!(e.journal().last_seq(), tail);
        let after: Vec<_> = e
            .inode_numbers()
            .into_iter()
            .map(|ino| (ino, e.caps_of(ino)))
            .collect();
        assert_eq!(after, caps, "identity wake changed caps");
    }
    assert_eq!(
        cap_contrasts[0], cap_contrasts[1],
        "observed floor added cap effects"
    );
}
