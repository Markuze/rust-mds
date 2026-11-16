//! Recovery controls use the production permit and journal-completion boundary.
mod common;
use common::*;
use engine::consts::*;
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
fn restore(cp: EngineCheckpoint) -> Engine<MemJournal> {
    Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap()
}
fn reconnect(e: &mut Engine<MemJournal>, client: ClientId, ino: Ino, wanted: u32, issued: u32) {
    e.reconnect_chunk(
        key(e, client),
        &[ReconnectCap {
            ino,
            cap_id: client,
            wanted,
            issued,
            realm: ROOT,
            snap_follows: 0,
            fcntl_locks: vec![],
            flock_locks: vec![],
        }],
        &[],
        false,
    )
    .unwrap();
}
fn commit(e: &mut Engine<MemJournal>) -> EngineStep {
    let seq = e.journal().last_seq();
    e.journal_mut().commit_all();
    let step = e.journal_event(JournalEvent::Committed(seq));
    assert_eq!(e.journal().last_seq(), seq, "a completion must not submit");
    step
}
fn no_grants(effects: &[Effect]) {
    assert!(
        cap_msgs(effects).iter().all(|m| m.op != CEPH_CAP_OP_GRANT),
        "{effects:#?}"
    );
    assert!(replies(effects).is_empty(), "ordinary replies stay held");
}
fn close(e: &mut Engine<MemJournal>, owner: SessionKey) -> Vec<Effect> {
    let mut input = Some(EngineInput::CloseSession {
        key: owner,
        kill: true,
    });
    let mut effects = Vec::new();
    for _ in 0..64 {
        let seq = e.journal().last_seq();
        let step = e.step(input.take(), permit()).unwrap();
        assert!(e.journal().last_seq() <= seq + 1);
        effects.extend(step.effects);
        if e.uncommitted_records() != 0 {
            effects.extend(commit(e).effects);
        }
        if e.session_cleanup_seq(owner).is_some() {
            return effects;
        }
    }
    panic!("cleanup did not finish");
}
fn drive(e: &mut Engine<MemJournal>, mut step: EngineStep) -> Vec<Effect> {
    let mut out = Vec::new();
    for _ in 0..128 {
        out.extend(step.effects);
        step = match step.state {
            StepState::NeedsAdmission => {
                let seq = e.journal().last_seq();
                let next = e.step(None, permit()).unwrap();
                assert!(e.journal().last_seq() <= seq + 1);
                next
            }
            StepState::WaitingCommit(_) => commit(e),
            StepState::Idle | StepState::WaitingInput => return out,
            StepState::Failed => panic!("recovery failed"),
        };
    }
    panic!("runnable work did not drain");
}
fn update(e: &Engine<MemJournal>, client: ClientId, ino: Ino) -> CapUpdate {
    let cap = e.cap(ino, client).unwrap();
    CapUpdate {
        op: CEPH_CAP_OP_UPDATE,
        ino,
        cap_id: cap.cap_id,
        seq: cap.last_sent,
        issue_seq: cap.last_issue,
        mseq: cap.mseq,
        caps: cap.pending(),
        wanted: cap.wanted(),
        ..Default::default()
    }
}
fn control(e: &mut Engine<MemJournal>, client: ClientId, update: CapUpdate) -> EngineStep {
    e.step(
        Some(EngineInput::CapUpdate {
            key: key(e, client),
            update,
        }),
        permit(),
    )
    .unwrap()
}
fn request(e: &Engine<MemJournal>, client: ClientId, ino: Ino) -> EngineInput {
    EngineInput::Request {
        key: key(e, client),
        request: Request::new(
            900,
            Op::Getattr {
                path: FilePath::ino(ino),
                mask: 0,
            },
        ),
    }
}
fn marker_busy(e: &mut Engine<MemJournal>) {
    match e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap_err()
    {
        StepError::Busy {
            input: Some(EngineInput::RecoveryControlsDrained),
            permit: returned,
        } => assert_eq!(returned, permit()),
        other => panic!("marker/permit ownership changed: {other:?}"),
    }
}
fn release(e: &mut Engine<MemJournal>) -> Vec<Effect> {
    let seq = e.journal().last_seq();
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    assert_eq!(
        e.journal().last_seq(),
        seq,
        "the marker is a pure transition"
    );
    assert_eq!(step.unused_permit, permit());
    assert!(step.effects.is_empty());
    drive(e, step)
}

#[test]
fn structural_finish_defers_competitor_grants_and_sequence_changes() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "early-flush", CEPH_O_RDWR).unwrap();
    let mut e = restore(fs.e.checkpoint().unwrap());
    // Dirty rights need not remain in the reconnect issued mask. The early
    // flush is outside the engine until the whole gather has been validated.
    reconnect(&mut e, C1, f, P, P);
    reconnect(&mut e, C2, f, AX | XX | FW, P);
    let c3 = key(&e, C3);
    reconnect(&mut e, C3, f, P, P);
    let before = e.caps_of(f);
    let keys = [key(&e, C1), key(&e, C2), c3];
    let seq = e.journal().last_seq();
    let step = e.finish_reconnect(&keys).unwrap();
    no_grants(&step.effects);
    assert_eq!(e.caps_of(f), before, "do not evaluate then hide grants");
    assert_eq!(e.journal().last_seq(), seq);
    no_grants(&drive(&mut e, step));
    assert_eq!(e.caps_of(f), before);
    assert_eq!(
        e.step(None, None).unwrap().state,
        StepState::Idle,
        "grant-only evaluation is not runnable admission work"
    );

    let mut dirty = update(&e, C1, f);
    dirty.op = CEPH_CAP_OP_FLUSH;
    dirty.dirty = AX | XX | FW; // None of these rights occur in reconnect issued.
    dirty.flush_tid = 701;
    dirty.uid = 7001;
    dirty.gid = 7002;
    dirty.mode = 0o600;
    dirty.size = 12345;
    dirty.mtime = ts(5000);
    dirty.ctime = ts(5001);
    dirty.xattrs = Some((
        9,
        XattrMap::from([(b"user.recovered".to_vec(), b"durable".to_vec())]),
    ));
    let step = control(&mut e, C1, dirty);
    assert_eq!(step.state, StepState::WaitingCommit(seq + 1));
    assert!(step.effects.is_empty(), "no early FLUSH_ACK");
    assert_eq!(e.cap(f, C2).unwrap().last_sent, 0);
    marker_busy(&mut e);
    assert!(matches!(
        e.step(
            Some(EngineInput::CloseSession {
                key: c3,
                kill: true
            }),
            permit()
        ),
        Err(StepError::Busy { .. })
    ));
    let duplicate = e.journal_event(JournalEvent::Committed(seq));
    assert!(duplicate.effects.is_empty());
    assert_eq!(duplicate.state, StepState::WaitingCommit(seq + 1));
    let step = commit(&mut e);
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK && m.flush_tid == 701)
    );
    no_grants(&drive(&mut e, step));
    no_grants(&close(&mut e, c3));
    assert_eq!(e.cap(f, C2).unwrap().last_sent, 0);
    assert_eq!(e.cap(f, C2).unwrap().pending(), P);

    // The seal names the original survivor set, even after a later close.
    let duplicate = e.finish_reconnect(&keys).unwrap();
    assert!(duplicate.effects.is_empty() && duplicate.notices.is_empty());
    assert!(e.finish_reconnect(&keys[..2]).is_err());
    assert!(e.reset_reconnect(keys[0]).is_err());
    assert!(e.reconnect_chunk(keys[0], &[], &[], false).is_err());
    assert!(matches!(
        e.step(Some(request(&e, C2, f)), permit()),
        Err(StepError::Busy { .. })
    ));

    let out = release(&mut e);
    let grant = cap_msgs(&out)
        .into_iter()
        .find(|m| m.client == C2 && m.op == CEPH_CAP_OP_GRANT)
        .unwrap();
    assert_eq!(grant.caps & (AX | XX | FW), AX | XX | FW);
    let fields = grant.inode.unwrap();
    assert_eq!(
        (fields.uid, fields.gid, fields.mode & 0o777),
        (7001, 7002, 0o600)
    );
    assert_eq!(fields.size, 12345);
    assert_eq!(fields.mtime, ts(5000));
    assert_eq!(fields.xattrs.as_ref().unwrap().0, 9);
    assert_eq!(
        fields.xattrs.unwrap().1[b"user.recovered".as_slice()],
        b"durable"
    );
    let step = e.step(Some(request(&e, C2, f)), permit()).unwrap();
    assert_eq!(
        reply_for(&step.effects, C2, 900)
            .unwrap()
            .target
            .as_ref()
            .unwrap()
            .size,
        12345
    );
}

#[test]
fn missing_cleanup_before_structural_finish_never_grants() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "missing-writer", CEPH_O_RDWR).unwrap();
    let mut e = restore(fs.e.checkpoint().unwrap());
    reconnect(&mut e, C2, f, FR | AX, P);
    let missing = key(&e, C1);
    let before = e.cap(f, C2).unwrap().clone();
    let effects = close(&mut e, missing);
    no_grants(&effects);
    assert_eq!(e.cap(f, C2).unwrap(), &before);
    assert!(
        e.pending_jobs().is_empty(),
        "no data work in Reconstruction"
    );
    assert!(e.inode(f).unwrap().client_ranges.contains_key(&C1));
}

#[test]
fn empty_checked_load_requires_both_boundaries_without_tick_or_commit() {
    // This checkpoint also models the fresh Ready bootstrap root. Checked
    // load must not inherit Engine::new's direct serving initialization.
    let bootstrap = Engine::new(config(), MemJournal::new());
    let cp = bootstrap.checkpoint().unwrap();
    let mut e = restore(cp.clone());
    marker_busy(&mut e);
    let step = e.finish_reconnect(&[]).unwrap();
    assert_eq!(step.state, StepState::Idle);
    assert!(step.effects.is_empty() && step.notices.is_empty() && step.unused_permit.is_none());
    assert!(e.reconnect_done().is_empty()); // Cannot open the checked-load gate.
    let late = SessionKey {
        client: C1,
        incarnation: 100,
    };
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), None)
        .unwrap();
    assert_eq!(step.state, StepState::NeedsAdmission);
    assert!(step.effects.is_empty());
    marker_busy(&mut e); // Do not submit a second marker while the engine owns one.
    let step = e.step(None, permit()).unwrap();
    assert_eq!(step.state, StepState::Idle);
    assert_eq!(step.unused_permit, permit());
    assert!(step.effects.is_empty() && step.notices.is_empty());
    assert_eq!(e.journal().last_seq(), cp.covered_seq);
    assert!(
        e.journal().records().is_empty(),
        "no append or callback is needed"
    );
    for supplied in [None, permit()] {
        let had_permit = supplied.is_some();
        let step = e
            .step(Some(EngineInput::RecoveryControlsDrained), supplied)
            .unwrap();
        assert_eq!(step.state, StepState::Idle);
        assert!(step.effects.is_empty() && step.notices.is_empty());
        assert_eq!(step.unused_permit, if had_permit { permit() } else { None });
    }
    assert!(
        e.session(C1).is_none(),
        "the marker executed with zero sessions"
    );
    e.install_session_key(late).unwrap();
    let step = e.step(Some(request(&e, C1, ROOT)), permit()).unwrap();
    assert_eq!(reply_for(&step.effects, C1, 900).unwrap().result, 0);
}

#[test]
fn range_only_grant_waits_for_release_even_if_cap_mask_does_not_change() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "range", CEPH_O_RDWR).unwrap();
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
    let cp = fs.e.checkpoint().unwrap();
    let truncate = cp
        .jobs
        .iter()
        .find(|j| matches!(j.kind, LifecycleKind::Truncate { .. }))
        .unwrap()
        .clone();
    let mut e = restore(cp);
    reconnect(&mut e, C1, f, FW, P | FW);
    for c in [C2, C3] {
        e.reconnect_chunk(key(&e, c), &[], &[], false).unwrap();
    }
    let keys = [key(&e, C1), key(&e, C2), key(&e, C3)];
    let step = e.finish_reconnect(&keys).unwrap();
    assert!(
        step.effects
            .iter()
            .any(|eff| matches!(eff, Effect::Lifecycle(j) if j.id == truncate.id))
    );
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_TRUNC)
    );
    for m in cap_msgs(&step.effects)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_TRUNC)
    {
        assert_eq!(m.inode.unwrap().max_size, 0);
    }
    no_grants(&drive(&mut e, step));
    let before = e.cap(f, C1).unwrap().clone();
    let old_bound = e.inode(f).unwrap().client_ranges[&C1];
    let mut dirty = update(&e, C1, f);
    dirty.op = CEPH_CAP_OP_FLUSH;
    dirty.dirty = FW;
    dirty.flush_tid = 702;
    dirty.size = 8 << 20;
    dirty.max_size = 16 << 20;
    dirty.mtime = ts(6000);
    assert!(control(&mut e, C1, dirty).effects.is_empty());
    let step = commit(&mut e);
    no_grants(&drive(&mut e, step));
    let bound = e.inode(f).unwrap().client_ranges[&C1];
    assert!(bound > old_bound);
    assert_eq!(
        e.cap(f, C1).unwrap(),
        &before,
        "no grant seq or mask change at flush commit"
    );
    // Retained data work and its completion are runnable before grant release.
    let step = e
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: truncate.id,
                result: LifecycleResult::Done,
            }),
            permit(),
        )
        .unwrap();
    assert!(step.effects.is_empty());
    let step = commit(&mut e);
    no_grants(&drive(&mut e, step));
    assert_eq!(
        e.step(None, permit()).unwrap().state,
        StepState::Idle,
        "range-only wait cannot spin"
    );
    let out = release(&mut e);
    let grants: Vec<_> = cap_msgs(&out)
        .into_iter()
        .filter(|m| m.op == CEPH_CAP_OP_GRANT && m.client == C1)
        .collect();
    assert_eq!(
        grants.len(),
        1,
        "the deferred identity must survive an unchanged mask"
    );
    assert_eq!(grants[0].caps, P | FW);
    assert_eq!(grants[0].inode.as_ref().unwrap().max_size, bound);
    assert_eq!(grants[0].inode.as_ref().unwrap().size, 8 << 20);
    assert!(grants[0].seq > before.last_sent);
}

#[test]
fn retained_probe_and_truncate_drain_a_blocked_dirty_control_before_release() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "probe", CEPH_O_RDWR).unwrap();
    fs.open(C2, f, CEPH_O_RDWR).unwrap();
    fs.settle(&fs.last.clone());
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
    let cp = fs.e.checkpoint().unwrap();
    let truncate = cp
        .jobs
        .iter()
        .find(|j| matches!(j.kind, LifecycleKind::Truncate { .. }))
        .unwrap()
        .clone();
    let mut e = restore(cp);
    reconnect(&mut e, C2, f, FW, P | FW);
    let c2 = key(&e, C2);
    let c3 = key(&e, C3);
    let missing = key(&e, C1);
    no_grants(&close(&mut e, missing));
    let old = e
        .checkpoint()
        .unwrap()
        .jobs
        .iter()
        .find(|j| matches!(j.kind, LifecycleKind::RecoverSize { .. }))
        .unwrap()
        .clone();
    e.reconnect_chunk(c3, &[], &[], false).unwrap();
    let step = e.finish_reconnect(&[c2, c3]).unwrap();
    let out = drive(&mut e, step);
    no_grants(&out);
    let revoke = revokes(&out)
        .into_iter()
        .find(|m| m.ino == f && m.client == C2)
        .unwrap();
    assert_eq!(revoke.caps & !P, 0, "a revoke cannot add rights");
    assert_eq!(revoke.inode.as_ref().unwrap().max_size, 0);
    assert!(
        !e.pending_jobs().iter().any(|j| j.id == old.id),
        "issued writer still blocks probe"
    );

    // A dirty writer's revoke acknowledgment must not be blanket-blocked.
    let mut dirty = update(&e, C2, f);
    dirty.op = CEPH_CAP_OP_FLUSH;
    dirty.dirty = FW;
    dirty.flush_tid = 703;
    dirty.size = 321;
    dirty.mtime = ts(7000);
    let step = control(&mut e, C2, dirty.clone());
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert!(step.effects.is_empty());
    let step = commit(&mut e);
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK && m.flush_tid == 703)
    );
    no_grants(&drive(&mut e, step));
    let replacement = e
        .checkpoint()
        .unwrap()
        .jobs
        .iter()
        .find(|j| matches!(j.kind, LifecycleKind::RecoverSize { .. }))
        .unwrap()
        .clone();
    assert!(replacement.id > old.id);
    assert!(e.lifecycle_intent(old.id).is_none());
    assert!(matches!((&old.kind, &replacement.kind),
        (LifecycleKind::RecoverSize { layouts: a, max_size: x, .. },
         LifecycleKind::RecoverSize { layouts: b, max_size: y, inode_version, truncate_seq })
            if a == b && y >= x && *inode_version == e.inode(f).unwrap().version
                && *truncate_seq == e.inode(f).unwrap().truncate_seq));

    // A committed duplicate needs no new metadata/probe dependency.
    let seq = e.journal().last_seq();
    let step = control(&mut e, C2, dirty);
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK && m.flush_tid == 703)
    );
    assert_eq!(e.journal().last_seq(), seq);
    let mut waiting = update(&e, C2, f);
    waiting.op = CEPH_CAP_OP_FLUSH;
    waiting.dirty = FW;
    waiting.flush_tid = 704;
    waiting.size = 999;
    waiting.mtime = ts(7001);
    let input = Some(EngineInput::CapUpdate {
        key: c2,
        update: waiting.clone(),
    });
    let retained = match e.step(input, permit()).unwrap_err() {
        StepError::Busy {
            input: Some(EngineInput::CapUpdate { key, update }),
            permit: returned,
        } => {
            assert_eq!(key, c2);
            assert_eq!(update, waiting);
            assert_eq!(returned, permit());
            EngineInput::CapUpdate { key, update }
        }
        other => panic!("control ownership changed: {other:?}"),
    };
    // Dependencies run ahead of the retained dirty control, without ACTIVE.
    assert_eq!(e.pending_jobs().len(), 1);
    assert_eq!(e.pending_jobs()[0].id, truncate.id);
    let step = e
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: old.id,
                result: LifecycleResult::Recovered {
                    size: 1,
                    mtime: ts(1),
                },
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(step.unused_permit, permit());
    assert_eq!(e.journal().last_seq(), seq);
    let step = e
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: truncate.id,
                result: LifecycleResult::Done,
            }),
            permit(),
        )
        .unwrap();
    assert!(step.effects.is_empty());
    marker_busy(&mut e);
    let step = commit(&mut e);
    no_grants(&drive(&mut e, step));
    assert_eq!(e.pending_jobs(), vec![replacement.clone()]);
    let step = e
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: replacement.id,
                result: LifecycleResult::Done,
            }),
            permit(),
        )
        .unwrap();
    assert_eq!(
        step.unused_permit,
        permit(),
        "a failed/mismatched probe cannot become an empty file"
    );
    assert_eq!(e.pending_jobs(), vec![replacement.clone()]);
    let step = e
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: replacement.id,
                result: LifecycleResult::Recovered {
                    size: 321,
                    mtime: ts(7000),
                },
            }),
            permit(),
        )
        .unwrap();
    assert!(step.effects.is_empty());
    assert!(e.lifecycle_intent(replacement.id).is_some());
    let step = commit(&mut e);
    no_grants(&drive(&mut e, step));
    assert!(e.lifecycle_intent(replacement.id).is_none());
    assert!(!e.inode(f).unwrap().client_ranges.contains_key(&C1));
    let step = e.step(Some(retained), permit()).unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert!(step.effects.is_empty());
    let step = commit(&mut e);
    assert!(
        cap_msgs(&step.effects)
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_FLUSH_ACK && m.flush_tid == 704)
    );
    no_grants(&drive(&mut e, step));
    let out = release(&mut e);
    let grant = cap_msgs(&out)
        .into_iter()
        .find(|m| m.op == CEPH_CAP_OP_GRANT && m.client == C2)
        .unwrap();
    assert_ne!(grant.caps & FW, 0);
    assert_eq!(grant.inode.unwrap().size, 999);
}

#[test]
fn stale_resume_defers_reissue_until_the_release_marker() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "reader").unwrap();
    let mut e = restore(fs.e.checkpoint().unwrap());
    reconnect(&mut e, C1, f, FR, P | FR);
    for c in [C2, C3] {
        e.reconnect_chunk(key(&e, c), &[], &[], false).unwrap();
    }
    let keys = [key(&e, C1), key(&e, C2), key(&e, C3)];
    let step = e.finish_reconnect(&keys).unwrap();
    no_grants(&drive(&mut e, step));
    no_grants(&e.session_stale(C1));
    let before = e.cap(f, C1).unwrap().clone();
    no_grants(&e.session_resume(C1));
    assert_eq!(
        e.cap(f, C1).unwrap(),
        &before,
        "no speculative generation/seq reissue"
    );
    assert_eq!(e.step(None, None).unwrap().state, StepState::Idle);
    assert!(
        cap_msgs(&release(&mut e))
            .iter()
            .any(|m| m.op == CEPH_CAP_OP_GRANT && m.client == C1)
    );
}

#[test]
fn malformed_and_stale_inputs_cannot_release_or_replace_the_seal() {
    let mut fs = Fs::new();
    let f = fs.mkfile(C1, ROOT, "f").unwrap();
    let mut e = restore(fs.e.checkpoint().unwrap());
    let keys = [key(&e, C1), key(&e, C2), key(&e, C3)];
    reconnect(&mut e, C1, f, AX, P);
    marker_busy(&mut e);
    assert!(e.finish_reconnect(&keys).is_err());
    for k in &keys[1..] {
        e.reconnect_chunk(*k, &[], &[], false).unwrap();
    }
    assert!(
        e.finish_reconnect(&[keys[0], keys[0], keys[1], keys[2]])
            .is_err()
    );
    assert!(e.finish_reconnect(&keys[..2]).is_err());
    let step = e.finish_reconnect(&keys).unwrap();
    no_grants(&drive(&mut e, step));
    let before = e.cap(f, C1).unwrap().clone();
    let mut bad = update(&e, C1, f);
    bad.cap_id += 1;
    assert!(control(&mut e, C1, bad).effects.is_empty());
    let stale = SessionKey {
        incarnation: keys[0].incarnation + 1,
        ..keys[0]
    };
    assert!(matches!(
        e.step(
            Some(EngineInput::CapRelease {
                key: stale,
                items: vec![]
            }),
            permit()
        ),
        Err(StepError::InvalidSession { .. })
    ));
    assert_eq!(e.cap(f, C1).unwrap(), &before);
    assert!(matches!(
        e.step(Some(request(&e, C1, f)), permit()),
        Err(StepError::Busy { .. })
    ));
    assert!(e.reset_reconnect(keys[0]).is_err());
    let duplicate = e.finish_reconnect(&keys).unwrap();
    assert!(duplicate.effects.is_empty() && duplicate.notices.is_empty());
    let out = release(&mut e);
    assert!(!cap_msgs(&out).is_empty());
    let before = e.cap(f, C1).unwrap().clone();
    let duplicate = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    assert!(duplicate.effects.is_empty() && duplicate.notices.is_empty());
    assert_eq!(duplicate.unused_permit, permit());
    assert_eq!(e.cap(f, C1).unwrap(), &before);
    let duplicate = e.finish_reconnect(&keys).unwrap();
    assert!(duplicate.effects.is_empty() && duplicate.notices.is_empty());
}

#[test]
fn marker_waits_for_cleanup_and_release_cursors_without_losing_progress() {
    for release_caps in [false, true] {
        let mut fs = Fs::new();
        let a = fs.create(C1, ROOT, "a", CEPH_O_RDWR).unwrap();
        let b = fs.create(C1, ROOT, "b", CEPH_O_RDWR).unwrap();
        let mut e = restore(fs.e.checkpoint().unwrap());
        let c1 = key(&e, C1);
        for ino in [a, b] {
            reconnect(&mut e, C1, ino, FW, P | FW);
        }
        for c in [C2, C3] {
            e.reconnect_chunk(key(&e, c), &[], &[], false).unwrap();
        }
        let keys = [c1, key(&e, C2), key(&e, C3)];
        let step = e.finish_reconnect(&keys).unwrap();
        no_grants(&drive(&mut e, step));
        let mut input = Some(if release_caps {
            EngineInput::CapRelease {
                key: c1,
                items: [a, b]
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
                    .collect(),
            }
        } else {
            EngineInput::CloseSession {
                key: c1,
                kill: true,
            }
        });
        let base = e.journal().last_seq();
        for _ in 0..16 {
            let before = e.journal().last_seq();
            let step = e.step(input.take(), permit()).unwrap();
            no_grants(&step.effects);
            assert!(e.journal().last_seq() <= before + 1);
            assert!(matches!(
                step.state,
                StepState::NeedsAdmission | StepState::WaitingCommit(_)
            ));
            marker_busy(&mut e);
            let duplicate = e.finish_reconnect(&keys).unwrap();
            assert_eq!(duplicate.state, step.state);
            assert!(duplicate.effects.is_empty() && duplicate.notices.is_empty());
            let step = commit(&mut e);
            no_grants(&step.effects);
            if step.state == StepState::Idle {
                break;
            }
            assert_eq!(
                step.state,
                StepState::NeedsAdmission,
                "cleanup continuation must wake without Tick"
            );
            marker_busy(&mut e); // The cursor still owns the next admitted turn.
            assert_eq!(e.step(None, None).unwrap().state, StepState::NeedsAdmission);
        }
        assert!(e.is_quiescent());
        assert_eq!(
            e.journal().last_seq() - base,
            if release_caps { 2 } else { 3 }
        );
        if !release_caps {
            assert!(e.session_cleanup_seq(c1).is_some());
        }
        no_grants(&release(&mut e));
    }
}

#[test]
fn zero_survivors_can_complete_retained_work_before_new_stray_purge() {
    let mut fs = Fs::new();
    let f = fs.create(C1, ROOT, "stray", CEPH_O_RDWR).unwrap();
    fs.unlink(C1, ROOT, "stray").unwrap();
    let mut e = restore(fs.e.checkpoint().unwrap());
    for c in [C1, C2, C3] {
        let owner = key(&e, c);
        no_grants(&close(&mut e, owner));
    }
    assert!(e.pending_jobs().is_empty());
    let base = e.journal().last_seq();
    let step = e.finish_reconnect(&[]).unwrap();
    let job = e.pending_jobs().pop().unwrap();
    assert!(matches!(job.kind, LifecycleKind::RecoverSize { .. }));
    let out = drive(&mut e, step);
    no_grants(&out);
    assert!(e.inode(f).is_some());
    assert_eq!(
        e.journal().last_seq(),
        base,
        "new stray purge waits throughout Control drain"
    );
    let step = e
        .step(
            Some(EngineInput::LifecycleCompleted {
                id: job.id,
                result: LifecycleResult::Recovered {
                    size: 123,
                    mtime: ts(8000),
                },
            }),
            permit(),
        )
        .unwrap();
    assert!(step.effects.is_empty());
    let step = commit(&mut e);
    no_grants(&drive(&mut e, step));
    assert!(e.inode(f).unwrap().client_ranges.is_empty());
    assert!(e.pending_jobs().is_empty());
    assert!(e.inode(f).is_some());
    let out = release(&mut e);
    assert!(e.inode(f).is_none());
    assert!(out.iter().any(|eff| matches!(eff, Effect::Lifecycle(j)
        if j.ino == f && matches!(j.kind, LifecycleKind::Purge { .. }))));
}

#[test]
fn failure_keeps_marker_output_and_job_permissions_closed() {
    let bootstrap = Engine::new(config(), MemJournal::new());
    for structural in [false, true] {
        let mut e = restore(bootstrap.checkpoint().unwrap());
        if structural {
            e.finish_reconnect(&[]).unwrap();
        }
        let failed = e.journal_event(JournalEvent::Failed(JournalError::Fenced));
        assert_eq!(failed.state, StepState::Failed);
        assert!(failed.effects.is_empty());
        let step = e
            .step(Some(EngineInput::RecoveryControlsDrained), permit())
            .unwrap();
        assert_eq!(step.state, StepState::Failed);
        assert!(step.effects.is_empty() && step.notices.is_empty());
        assert_eq!(step.unused_permit, permit());
        assert_eq!(e.finish_reconnect(&[]).unwrap_err(), RestoreError::Failed);
        assert!(e.pending_jobs().is_empty());
    }
}
