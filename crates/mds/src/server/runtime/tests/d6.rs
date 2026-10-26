use super::*;

fn open_metadata(f: &mut Fixture, client: u64, peer: &Peer, metadata: &[(&str, &str)]) {
    let mut msg = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
    msg.metadata = metadata
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect();
    f.send(client, peer, msg);
    f.drain();
    assert!(f.sessions.get(client).unwrap().is_open());
    peer.clear();
}

fn replies(peer: &Peer) -> Vec<proto::MClientReclaimReply> {
    peer.output
        .lock()
        .unwrap()
        .iter()
        .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_RECLAIM_REPLY)
        .map(|raw| proto::MClientReclaimReply::from_raw(raw.clone()).unwrap())
        .collect()
}

#[test]
fn g3_d6_reclaim_flag_errors_finish_silence_and_absent_uuid() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    peer.clear();
    for (flags, uuid, expected) in [
        (proto::reclaim::RECLAIM_FLAG_FINISH, "", None),
        (proto::reclaim::RECLAIM_FLAG_FINISH | 1, "x", Some(-22)),
        (0, "x", Some(-22)),
        (2, "x", Some(-22)),
        (1, "", Some(-22)),
        (1, "absent", Some(0)),
    ] {
        f.send(
            42,
            &peer,
            proto::MClientReclaim {
                flags,
                uuid: uuid.into(),
            },
        );
        let output = replies(&peer);
        assert_eq!(
            output.len(),
            usize::from(expected.is_some()),
            "flags={flags}"
        );
        if let Some(result) = expected {
            assert_eq!(
                output[0],
                proto::MClientReclaimReply {
                    result,
                    ..Default::default()
                }
            );
        }
        peer.clear();
    }
    f.send(
        99,
        &peer,
        proto::MClientReclaim {
            flags: 1,
            uuid: "absent".into(),
        },
    );
    assert!(replies(&peer).is_empty());
}

#[test]
fn g3_d6_uuid_uniqueness_includes_pending_open_and_live_closing_rows() {
    let mut f = Fixture::new();
    let a = Peer::new(10);
    let b = Peer::new(11);
    let mut open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
    open.metadata.insert("uuid".into(), "shared".into());
    f.send(42, &a, open.clone());
    let first = f.commands.try_recv().unwrap();
    f.send(43, &b, open.clone());
    assert!(
        f.commands.try_recv().is_err(),
        "pending UUID is reserved before durable OPEN"
    );
    assert_eq!(
        MClientSession::from_raw(b.output.lock().unwrap()[0].clone())
            .unwrap()
            .op,
        SessionOp::REJECT
    );
    f.complete(first);
    f.send(42, &a, open.clone()); // Same-key metadata update is legal.
    f.drain();
    f.send(
        42,
        &a,
        MClientSession::new(
            SessionOp::REQUEST_CLOSE,
            f.sessions.get(42).unwrap().push_seq(),
        ),
    );
    let closing = f.commands.try_recv().unwrap();
    f.send(43, &b, open.clone());
    assert!(f.commands.try_recv().is_err());
    f.complete(closing);
    f.drain();
    b.clear();
    f.send(43, &b, open);
    f.drain();
    assert!(f.sessions.get(43).unwrap().is_open());
}

#[test]
fn g3_d6_active_timeout_thresholds_preserve_rights_until_fencing() {
    for (timeout, expiry) in [
        (None, 300),
        (Some("0"), 300),
        (Some("oops"), 300),
        (Some("30"), 60),
        (Some(" 120 "), 120),
        (Some("600"), 300),
    ] {
        let mut f = Fixture::new();
        let peer = Peer::new(10);
        let metadata = timeout.map(|v| vec![("timeout", v)]).unwrap_or_default();
        open_metadata(&mut f, 42, &peer, &metadata);
        let ino = f.create(42, &peer);
        let cap = f.server.engine.cap(ino, 42).unwrap().clone();
        peer.clear();
        let (tx, mut fences) = mpsc::channel(4);
        f.server.set_evict_channel(tx);
        f.sessions.get_mut(42).unwrap().last_renew = Instant::now() - Duration::from_secs(60);
        f.server.tick(&mut f.sessions);
        if expiry == 60 {
            assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Fencing);
        } else {
            assert_eq!(
                f.sessions.get(42).unwrap().state,
                if timeout.is_none() {
                    SessionState::Stale
                } else {
                    SessionState::Open
                }
            );
            assert!(fences.try_recv().is_err());
            f.sessions.get_mut(42).unwrap().last_renew =
                Instant::now() - Duration::from_secs(expiry);
            f.server.tick(&mut f.sessions);
        }
        let fence = fences
            .try_recv()
            .expect("expiry must fence, not drop rights");
        assert_eq!(f.server.engine.cap(ino, 42).unwrap(), &cap);
        assert!(f.commands.try_recv().is_err());
        assert!(f.server.finish_eviction(&mut f.sessions, fence, 9));
        f.drain();
        assert!(f.server.engine.session(42).is_none());
    }
}

#[test]
fn g3_d6_reset_waits_for_full_union_fence_and_closed_before_ordered_reply() {
    let mut f = Fixture::new();
    let old = Peer::new(10);
    let requester = Peer::new(20);
    open_metadata(&mut f, 42, &old, &[("uuid", "old"), ("timeout", "0")]);
    let ino = f.create(42, &old);
    f.open(43, &requester);
    requester.clear();
    let changed = Peer::new(11);
    open_metadata(&mut f, 42, &changed, &[("uuid", "old"), ("timeout", "0")]);
    f.sessions.pause_output(true);
    f.sessions
        .get_mut(43)
        .unwrap()
        .send(&MClientSession::renewcaps(99));
    f.send(
        43,
        &requester,
        proto::MClientReclaim {
            flags: 1,
            uuid: "old".into(),
        },
    );
    assert!(replies(&requester).is_empty());
    assert!(f.commands.try_recv().is_err());
    let targets = f.server.recovery_fences(&mut f.sessions);
    assert_eq!(
        targets.len(),
        1,
        "RESET must enqueue full durable target fencing"
    );
    assert_eq!(
        targets[0].addresses,
        EntityAddrVec(vec![old.addr, changed.addr])
    );
    assert!(f.server.engine.cap(ino, 42).is_some());
    assert!(
        f.server
            .finish_recovery_eviction(&mut f.sessions, &targets[0], 12)
    );
    f.drain();
    f.sessions.pause_output(false);
    f.sessions.drain_output(32);
    let output = requester.output.lock().unwrap();
    assert_eq!(MClientSession::from_raw(output[0].clone()).unwrap().seq, 99);
    drop(output);
    assert_eq!(
        replies(&requester),
        vec![proto::MClientReclaimReply {
            result: 0,
            epoch: 12,
            addrs: targets[0].addresses.clone()
        }]
    );
    assert_eq!(
        f.server.runtime.as_ref().unwrap().rows[&targets[0].key].lifecycle,
        SessionLifecycle::Closed
    );
}

#[test]
fn g3_d6_wrong_auth_and_obsolete_requester_cannot_reclaim_or_renew() {
    let mut f = Fixture::new();
    let target = Peer::new(10);
    open_metadata(&mut f, 42, &target, &[("uuid", "old")]);
    let mut denied = Peer::new(20);
    denied.identity.0 = "client.other".into();
    f.open(43, &denied);
    denied.clear();
    let before = Instant::now() - Duration::from_secs(100);
    f.sessions.get_mut(43).unwrap().last_renew = before;
    f.send(
        43,
        &denied,
        proto::MClientReclaim {
            flags: 1,
            uuid: "old".into(),
        },
    );
    assert_eq!(replies(&denied)[0].result, -1);
    assert_eq!(f.sessions.get(43).unwrap().last_renew, before);
    assert!(f.server.recovery_fences(&mut f.sessions).is_empty());
    let mut obsolete = denied.clone();
    obsolete.connection += 100;
    denied.clear();
    f.send(
        43,
        &obsolete,
        proto::MClientReclaim {
            flags: 1,
            uuid: "old".into(),
        },
    );
    f.send(
        43,
        &obsolete,
        MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 1),
    );
    assert!(replies(&denied).is_empty());
    assert_eq!(f.sessions.get(43).unwrap().last_renew, before);
    assert!(f.sessions.get(42).unwrap().is_open());
}

#[test]
fn g3_d6_pending_target_attachment_and_requester_replacement_do_not_inherit_reply() {
    for replacement in [
        "remove",
        "generation",
        "open-id",
        "authorization",
        "address",
    ] {
        let mut f = Fixture::new();
        let a = Peer::new(10);
        let b = Peer::new(11);
        let requester = Peer::new(20);
        open_metadata(&mut f, 42, &a, &[("uuid", "old")]);
        f.open(43, &requester);
        requester.clear();
        let mut open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
        open.metadata.insert("uuid".into(), "old".into());
        f.send(42, &b, open);
        let update = f.commands.try_recv().unwrap();
        f.send(
            43,
            &requester,
            proto::MClientReclaim {
                flags: 1,
                uuid: "old".into(),
            },
        );
        assert!(
            f.server.recovery_fences(&mut f.sessions).is_empty(),
            "wait for the durable address union"
        );
        f.complete(update);
        assert!(
            b.output.lock().unwrap().is_empty(),
            "a reclaim-frozen attachment cannot publish OPEN"
        );
        let targets = f.server.recovery_fences(&mut f.sessions);
        assert_eq!(targets[0].addresses, EntityAddrVec(vec![a.addr, b.addr]));
        match replacement {
            "remove" => {
                f.sessions.remove(43);
            }
            "generation" => {
                f.sessions.get_mut(43).unwrap().generation += 1;
            }
            "open-id" => {
                f.send(
                    43,
                    &requester,
                    MClientSession::new(SessionOp::REQUEST_OPEN, 0),
                );
                f.drain();
                requester.clear();
            }
            _ => {
                let mut replaced = requester.clone();
                if replacement == "authorization" {
                    replaced.identity.1 = Bytes::from_static(b"changed context");
                } else {
                    replaced.addr = Peer::new(99).addr;
                }
                f.sessions.get_mut(43).unwrap().link = Box::new(replaced);
            }
        }
        assert!(
            f.server
                .finish_recovery_eviction(&mut f.sessions, &targets[0], 9)
        );
        f.drain();
        assert!(replies(&requester).is_empty(), "replacement={replacement}");
    }
}

#[test]
fn g3_d6_closing_target_and_duplicate_reclaims_join_one_cleanup() {
    for proof_after_closed in [false, true] {
        let mut f = Fixture::new();
        let target = Peer::new(10);
        let requester = Peer::new(20);
        let other = Peer::new(21);
        open_metadata(&mut f, 42, &target, &[("uuid", "old")]);
        f.open(43, &requester);
        f.open(44, &other);
        requester.clear();
        other.clear();
        f.send(
            42,
            &target,
            MClientSession::new(SessionOp::REQUEST_CLOSE, 0),
        );
        let closing = f.commands.try_recv().unwrap();
        for (client, peer) in [(43, &requester), (43, &requester), (44, &other)] {
            f.send(
                client,
                peer,
                proto::MClientReclaim {
                    flags: 1,
                    uuid: "old".into(),
                },
            );
        }
        let targets = f.server.recovery_fences(&mut f.sessions);
        assert_eq!(targets.len(), 1);
        if proof_after_closed {
            f.complete(closing);
            f.drain();
            assert!(replies(&requester).is_empty());
            assert!(replies(&other).is_empty());
            let tail = f.server.engine.submitted_seq();
            assert!(
                f.server
                    .finish_recovery_eviction(&mut f.sessions, &targets[0], 9)
            );
            f.drain();
            assert_eq!(
                f.server.engine.submitted_seq(),
                tail,
                "Closed target needs no second kill/receipt"
            );
        } else {
            assert!(
                f.server
                    .finish_recovery_eviction(&mut f.sessions, &targets[0], 9)
            );
            f.complete(closing);
            f.drain();
        }
        assert_eq!(replies(&requester).len(), 1);
        assert_eq!(replies(&other).len(), 1);
        assert_eq!(
            f.server.engine.checkpoint().unwrap().cleanup_receipts.len(),
            1
        );
        assert!(
            !f.server
                .finish_recovery_eviction(&mut f.sessions, &targets[0], 9)
        );
    }
}

#[test]
fn g3_d6_timeout_parsing_exact_thresholds_renewal_and_retained_locks() {
    for value in [
        "",
        " ",
        "+120",
        "-1",
        "12x",
        "4294967296",
        "0",
        " 120 ",
        "30",
        "600",
    ] {
        let mut f = Fixture::new();
        let peer = Peer::new(10);
        open_metadata(&mut f, 42, &peer, &[("timeout", value)]);
        let ino = f.create(42, &peer);
        let mut lock = proto::MClientRequest::new(2, MdsOp::SETFILELOCK);
        lock.path = WireFilePath::new(ino, "");
        lock.head.args = proto::request::FilelockArgs {
            rule: proto::types::filelock::RULE_FCNTL,
            lock_type: proto::types::filelock::EXCL,
            owner: 77,
            length: 0,
            ..Default::default()
        }
        .to_args();
        f.send(42, &peer, lock);
        f.drain();
        let locks = f.server.engine.file_locks(ino, false);
        assert_eq!(locks.len(), 1);
        let base = Instant::now();
        f.sessions.get_mut(42).unwrap().last_renew = base;
        let expiry = match value {
            "30" => 60,
            " 120 " => 120,
            _ => 300,
        };
        let (tx, mut fences) = mpsc::channel(4);
        f.server.set_evict_channel(tx);
        f.server
            .active_timers(&mut f.sessions, base + Duration::from_secs(expiry - 1));
        assert_eq!(
            f.sessions.get(42).unwrap().state,
            SessionState::Open,
            "timeout={value:?}"
        );
        assert!(fences.try_recv().is_err());
        assert_eq!(f.server.engine.file_locks(ino, false), locks);
        f.server
            .active_timers(&mut f.sessions, base + Duration::from_secs(expiry));
        let request = fences.try_recv().unwrap();
        assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Fencing);
        assert_eq!(f.server.engine.file_locks(ino, false), locks);
        let key = f.durable_keys[&42];
        assert_eq!(
            f.server.runtime.as_ref().unwrap().rows[&key]
                .identity
                .metadata[b"timeout".as_slice()]
            .as_ref(),
            value.as_bytes()
        );
        let mut obsolete = request;
        obsolete.session_generation += 1;
        assert!(!f.server.finish_eviction(&mut f.sessions, obsolete, 9));
        assert_eq!(f.server.engine.file_locks(ino, false), locks);
    }
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    let base = Instant::now();
    f.sessions.get_mut(42).unwrap().last_renew = base;
    f.server
        .active_timers(&mut f.sessions, base + Duration::from_secs(60));
    assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Stale);
    crate::daemon::dispatch_client_raw_at(
        &mut f.server,
        &mut f.sessions,
        42,
        Box::new(peer.clone()),
        vec![],
        MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 5).to_raw(peer.features()),
        base + Duration::from_secs(61),
    );
    assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Open);
    f.server
        .active_timers(&mut f.sessions, base + Duration::from_secs(120));
    assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Open);
    f.server
        .active_timers(&mut f.sessions, base + Duration::from_secs(121));
    assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Stale);
    let (tx, mut fences) = mpsc::channel(4);
    f.server.set_evict_channel(tx);
    f.server
        .active_timers(&mut f.sessions, base + Duration::from_secs(360));
    assert!(fences.try_recv().is_err());
    f.server
        .active_timers(&mut f.sessions, base + Duration::from_secs(361));
    assert!(
        fences.try_recv().is_ok(),
        "autoclose measures activity, not STALE entry"
    );
}

#[test]
fn g3_d6_uuid_old_and_new_reservations_remain_until_durable_update() {
    let mut f = Fixture::new();
    let a = Peer::new(10);
    let b = Peer::new(11);
    open_metadata(&mut f, 42, &a, &[("uuid", "old")]);
    let mut update = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
    update.metadata.insert("uuid".into(), "new".into());
    f.send(42, &a, update);
    let pending = f.commands.try_recv().unwrap();
    for uuid in ["old", "new"] {
        let mut msg = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
        msg.metadata.insert("uuid".into(), uuid.into());
        f.send(43, &b, msg);
        assert!(
            f.commands.try_recv().is_err(),
            "both UUID decisions remain reserved: {uuid}"
        );
    }
    f.complete(pending);
    f.drain();
    open_metadata(&mut f, 43, &b, &[("uuid", "old")]);
    assert_eq!(f.sessions.get(42).unwrap().metadata["uuid"], "new");
}

#[test]
fn g3_d6_reclaim_waiter_bound_and_terminal_output_failure_fail_closed() {
    let mut f = Fixture::new();
    let target = Peer::new(10);
    open_metadata(&mut f, 42, &target, &[("uuid", "old")]);
    for client in 43..=171 {
        let peer = Peer::new(client as u32);
        f.open(client, &peer);
        f.send(
            client,
            &peer,
            proto::MClientReclaim {
                flags: 1,
                uuid: "old".into(),
            },
        );
        if client < 171 {
            assert!(f.server.fatal_error().is_none());
        }
    }
    assert!(f.server.fatal_error().unwrap().contains("128"));
    assert!(f.server.engine.session(42).is_some());
    assert_eq!(
        f.server.engine.session_cleanup_seq(f.durable_keys[&42]),
        None
    );

    let mut f = Fixture::new();
    let requester = Peer::new(20);
    open_metadata(&mut f, 42, &target, &[("uuid", "old")]);
    f.open(43, &requester);
    requester.clear();
    f.send(
        43,
        &requester,
        proto::MClientReclaim {
            flags: 1,
            uuid: "old".into(),
        },
    );
    let target = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
    f.sessions.pause_output(true);
    for seq in 0..1024 {
        assert!(
            f.sessions
                .get_mut(43)
                .unwrap()
                .send(&MClientSession::renewcaps(seq))
        );
    }
    assert!(
        f.server
            .finish_recovery_eviction(&mut f.sessions, &target, 9)
    );
    // Apply actual production steps with explicit delayed callbacks. Only the
    // final Closed callback can attempt the now-over-budget reclaim reply.
    loop {
        let command = f.commands.try_recv().unwrap();
        if let CommandKind::Closed { key, cleanup_seq } = command.kind {
            f.server.on_storage_event(
                &mut f.sessions,
                command.owner,
                command.operation,
                EventKind::Closed { key, cleanup_seq },
            );
            break;
        }
        f.complete(command);
    }
    assert!(f.sessions.output_failed());
    assert!(f.server.fatal_error().is_some());
    assert!(!f.server.has_work());
    assert!(requester.output.lock().unwrap().is_empty());
}

#[test]
fn g3_d6_valid_current_traffic_advances_monotonically_but_invalid_input_does_not() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    let base = Instant::now();
    f.sessions.get_mut(42).unwrap().last_renew = base;
    for received in [
        base + Duration::from_secs(50),
        base + Duration::from_secs(40),
    ] {
        let mut req = proto::MClientRequest::new(2, MdsOp::LOOKUP);
        req.path = WireFilePath::new(1, "absent");
        crate::daemon::dispatch_client_raw_at(
            &mut f.server,
            &mut f.sessions,
            42,
            Box::new(peer.clone()),
            vec![],
            req.to_raw(peer.features()),
            received,
        );
        f.drain();
        assert_eq!(
            f.sessions.get(42).unwrap().last_renew,
            base + Duration::from_secs(50)
        );
    }
    let req = proto::MClientRequest::new(3, MdsOp(0xffff));
    crate::daemon::dispatch_client_raw_at(
        &mut f.server,
        &mut f.sessions,
        42,
        Box::new(peer.clone()),
        vec![],
        req.to_raw(peer.features()),
        base + Duration::from_secs(59),
    );
    assert_eq!(
        f.sessions.get(42).unwrap().last_renew,
        base + Duration::from_secs(50)
    );
    f.server
        .active_timers(&mut f.sessions, base + Duration::from_secs(109));
    assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Open);
    f.server
        .active_timers(&mut f.sessions, base + Duration::from_secs(110));
    assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Stale);
}

#[test]
fn g3_d6_map_timer_minimum_and_autoclose_precedence() {
    for (timeout, autoclose, expiry) in [(0, 0, 1), (60, 30, 60)] {
        let mut f = Fixture::new();
        let peer = Peer::new(10);
        open_metadata(&mut f, 42, &peer, &[("timeout", "0")]);
        let map = monc::MdsMap {
            session_timeout: timeout,
            session_autoclose: autoclose,
            ..Default::default()
        };
        f.server.set_mdsmap(&map);
        let base = Instant::now();
        f.sessions.get_mut(42).unwrap().last_renew = base;
        let (tx, mut fences) = mpsc::channel(4);
        f.server.set_evict_channel(tx);
        f.server
            .active_timers(&mut f.sessions, base + Duration::from_secs(expiry - 1));
        assert!(fences.try_recv().is_err());
        f.server
            .active_timers(&mut f.sessions, base + Duration::from_secs(expiry));
        assert!(fences.try_recv().is_ok());
        assert_eq!(f.sessions.get(42).unwrap().state, SessionState::Fencing);
    }
}

#[tokio::test]
async fn g3_d6_completion_rechecks_requester_features_and_current_blocklist() {
    for changed in ["features", "blocklist"] {
        let mut f = Fixture::new();
        let target = Peer::new(10);
        let requester = Peer::new(20);
        open_metadata(&mut f, 42, &target, &[("uuid", "old")]);
        f.open(43, &requester);
        requester.clear();
        f.send(
            43,
            &requester,
            proto::MClientReclaim {
                flags: 1,
                uuid: "old".into(),
            },
        );
        let target = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
        let (objecter, _) = crate::test_backend::data::DataIo::default()
            .objecter()
            .await;
        f.server.set_objecter(objecter.clone());
        if changed == "features" {
            f.server.required_client_features = Bytes::from_static(&[0x80]);
        } else {
            let mut map = (*objecter.osdmap()).clone();
            map.epoch += 1;
            let mut blocked = requester.addr;
            blocked.ty = denc::AddrType::ANY; // The real Nautilus+ map stores ANY entries.
            map.blocklist.insert(blocked, denc::UTime::default());
            let epoch = map.epoch;
            objecter.install_map(map);
            objecter
                .wait_for_map_timeout(epoch, Duration::from_secs(1))
                .await
                .unwrap();
            assert!(objecter.osdmap().is_blocklisted(&requester.addr));
        }
        assert!(
            f.server
                .finish_recovery_eviction(&mut f.sessions, &target, 9)
        );
        f.drain();
        assert!(
            replies(&requester).is_empty(),
            "requester policy changed: {changed}"
        );
        objecter.shutdown();
    }
}

fn r1_bytes<M: Message>(peer: &Peer, msg: &M) -> usize {
    let raw = msg.to_raw(peer.features());
    raw.front.len() + raw.middle.len() + raw.data.len() + 64
}

fn r1_renewals(peer: &Peer) -> Vec<u64> {
    peer.output
        .lock()
        .unwrap()
        .iter()
        .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_SESSION)
        .map(|raw| MClientSession::from_raw(raw.clone()).unwrap())
        .filter(|msg| msg.op == SessionOp::RENEWCAPS)
        .map(|msg| msg.seq)
        .collect()
}

/// Retained independent reproducer: production dispatch, real engine cleanup,
/// tagged Closed, then one normal 32-output turn before replacement OPEN.
fn r1_queued_completion() -> (Fixture, Peer, RecoveryFence) {
    let mut f = Fixture::new();
    let target = Peer::new(10);
    let requester = Peer::new(20);
    open_metadata(&mut f, 42, &target, &[("uuid", "old")]);
    f.open(43, &requester);
    requester.clear();
    f.sessions.pause_output(true);
    for seq in 0..40 {
        assert!(
            f.sessions
                .get_mut(43)
                .unwrap()
                .send(&MClientSession::renewcaps(seq))
        );
    }
    f.send(
        43,
        &requester,
        proto::MClientReclaim {
            flags: 1,
            uuid: "old".into(),
        },
    );
    let fence = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
    assert!(
        f.server
            .finish_recovery_eviction(&mut f.sessions, &fence, 9)
    );
    f.drain();
    assert_eq!(
        f.server.runtime.as_ref().unwrap().rows[&fence.key].lifecycle,
        SessionLifecycle::Closed
    );
    assert!(f.server.engine.session_cleanup_seq(fence.key).is_some());
    assert!(replies(&requester).is_empty());
    let expected = 40 * r1_bytes(&requester, &MClientSession::renewcaps(0))
        + r1_bytes(
            &requester,
            &proto::MClientReclaimReply {
                result: 0,
                epoch: 9,
                addrs: fence.addresses.clone(),
            },
        );
    assert_eq!(f.sessions.queued_output_bytes(), expected);
    f.sessions.pause_output(false);
    f.sessions.drain_output(32);
    assert_eq!(r1_renewals(&requester), (0..32).collect::<Vec<_>>());
    assert_eq!(f.sessions.get(43).unwrap().push_seq(), 0);
    assert_eq!(
        f.sessions.queued_output_bytes(),
        expected - 32 * r1_bytes(&requester, &MClientSession::renewcaps(0))
    );
    (f, requester, fence)
}

#[test]
fn g3_d6_r1_queued_completion_must_not_follow_replacement_requester_link() {
    let (mut f, requester, fence) = r1_queued_completion();
    let key = f.durable_keys[&43];
    let old_open_id = f.server.runtime.as_ref().unwrap().rows[&key]
        .identity
        .open_id;
    let generation = f.sessions.get(43).unwrap().generation;
    let receipt = f.server.engine.session_cleanup_seq(fence.key);
    let mut replacement = Peer::new(21);
    replacement.identity.1 = Bytes::from_static(b"renewed authorization context");
    f.open(43, &replacement);
    let row = &f.server.runtime.as_ref().unwrap().rows[&key];
    assert_ne!(row.identity.open_id, old_open_id);
    assert_eq!(row.identity.authorization_context, replacement.identity.1);
    assert_eq!(
        f.sessions.get(43).unwrap().link.connection_id(),
        replacement.connection
    );
    assert_eq!(f.sessions.get(43).unwrap().generation, generation);
    f.send(
        43,
        &replacement,
        MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 777),
    );
    f.sessions.drain_output(32);
    assert!(
        replies(&replacement).is_empty(),
        "old reclaim completion must not migrate to a replacement attachment after enqueue"
    );
    assert!(replies(&requester).is_empty());
    assert_eq!(
        r1_renewals(&replacement),
        (32..40).chain([777]).collect::<Vec<_>>()
    );
    assert_eq!(f.sessions.queued_output_bytes(), 0);
    assert_eq!(f.sessions.get(43).unwrap().push_seq(), 0);
    assert!(!f.sessions.output_failed());
    assert_eq!(f.server.engine.session_cleanup_seq(fence.key), receipt);
    assert_eq!(
        f.server.runtime.as_ref().unwrap().rows[&fence.key].lifecycle,
        SessionLifecycle::Closed
    );
}

#[test]
fn g3_d6_r1_same_connection_durable_open_also_invalidates_queued_completion() {
    let (mut f, requester, _) = r1_queued_completion();
    let key = f.durable_keys[&43];
    let old_open_id = f.server.runtime.as_ref().unwrap().rows[&key]
        .identity
        .open_id;
    f.open(43, &requester);
    assert_ne!(
        f.server.runtime.as_ref().unwrap().rows[&key]
            .identity
            .open_id,
        old_open_id
    );
    f.sessions.drain_output(32);
    assert!(
        replies(&requester).is_empty(),
        "connection equality does not preserve the old durable OPEN identity"
    );
    assert_eq!(r1_renewals(&requester), (0..40).collect::<Vec<_>>());
    assert_eq!(f.sessions.queued_output_bytes(), 0);
    assert!(!f.sessions.output_failed());
}

#[test]
fn g3_d6_r1_queued_completion_is_suppressed_on_requester_closing_or_fencing() {
    for state in [SessionState::Closing, SessionState::Fencing] {
        let (mut f, requester, target) = r1_queued_completion();
        let receipt = f.server.engine.session_cleanup_seq(target.key);
        let key = f.durable_keys[&43];
        let (tx, mut evictions) = mpsc::channel(4);
        f.server.set_evict_channel(tx);
        if state == SessionState::Closing {
            f.send(
                43,
                &requester,
                MClientSession::new(SessionOp::REQUEST_CLOSE, 0),
            );
        } else {
            f.server.request_eviction(&mut f.sessions, 43);
            assert_eq!(evictions.try_recv().unwrap().client, 43);
        }
        assert_eq!(f.sessions.get(43).unwrap().state, state);
        f.sessions.drain_output(32);
        assert!(
            replies(&requester).is_empty(),
            "queued old success cannot pass lifecycle {state:?}"
        );
        assert_eq!(r1_renewals(&requester), (0..40).collect::<Vec<_>>());
        assert_eq!(f.sessions.queued_output_bytes(), 0);
        assert_eq!(f.sessions.get(43).unwrap().push_seq(), 0);
        assert_eq!(f.server.engine.session_cleanup_seq(key), None);
        assert_eq!(f.server.engine.session_cleanup_seq(target.key), receipt);
        assert!(!f.sessions.output_failed());
        if state == SessionState::Closing {
            f.drain();
            assert!(f.sessions.get(43).is_none());
            assert!(f.server.engine.session_cleanup_seq(key).is_some());
            assert!(
                requester
                    .output
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_SESSION)
                    .map(|raw| MClientSession::from_raw(raw.clone()).unwrap())
                    .any(|msg| msg.op == SessionOp::CLOSE)
            );
        }
    }
}

#[test]
fn g3_d6_r1_unchanged_requester_keeps_fifo_bytes_and_push_sequence() {
    let mut f = Fixture::new();
    let target = Peer::new(10);
    let requester = Peer::new(20);
    open_metadata(&mut f, 42, &target, &[("uuid", "old")]);
    f.open(43, &requester);
    requester.clear();
    f.sessions.pause_output(true);
    let cap = proto::MClientCaps::default();
    assert!(f.sessions.get_mut(43).unwrap().send(&cap));
    for seq in 0..40 {
        f.send(
            43,
            &requester,
            MClientSession::new(SessionOp::REQUEST_RENEWCAPS, seq),
        );
    }
    f.send(
        43,
        &requester,
        proto::MClientReclaim {
            flags: 1,
            uuid: "old".into(),
        },
    );
    let fence = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
    assert!(replies(&requester).is_empty());
    assert!(
        f.server
            .finish_recovery_eviction(&mut f.sessions, &fence, 9)
    );
    f.drain();
    assert_eq!(
        f.server.runtime.as_ref().unwrap().rows[&fence.key].lifecycle,
        SessionLifecycle::Closed
    );
    assert!(f.server.engine.session_cleanup_seq(fence.key).is_some());
    let reply = proto::MClientReclaimReply {
        result: 0,
        epoch: 9,
        addrs: fence.addresses,
    };
    assert!(f.sessions.get_mut(43).unwrap().send(&cap));
    assert_eq!(
        f.sessions.queued_output_bytes(),
        2 * r1_bytes(&requester, &cap)
            + 40 * r1_bytes(&requester, &MClientSession::renewcaps(0))
            + r1_bytes(&requester, &reply)
    );
    f.sessions.pause_output(false);
    f.sessions.drain_output(32);
    assert_eq!(f.sessions.get(43).unwrap().push_seq(), 1);
    assert!(replies(&requester).is_empty());
    f.sessions.drain_output(10); // Nine renewals, then the reclaim reply. No later cap.
    assert_eq!(replies(&requester), vec![reply]);
    assert_eq!(r1_renewals(&requester), (0..40).collect::<Vec<_>>());
    assert_eq!(f.sessions.get(43).unwrap().push_seq(), 1);
    assert_eq!(f.sessions.queued_output_bytes(), r1_bytes(&requester, &cap));
    f.sessions.drain_output(1);
    let output = requester.output.lock().unwrap();
    assert_eq!(output[0].msg_type, proto::msg_type::CLIENT_CAPS);
    assert_eq!(output[41].msg_type, proto::msg_type::CLIENT_RECLAIM_REPLY);
    assert_eq!(output[42].msg_type, proto::msg_type::CLIENT_CAPS);
    drop(output);
    assert_eq!(f.sessions.get(43).unwrap().push_seq(), 2);
    assert_eq!(f.sessions.queued_output_bytes(), 0);
    assert!(!f.sessions.output_failed());
    f.sessions.drain_output(32);
    assert_eq!(replies(&requester).len(), 1);
}

#[test]
fn g3_d6_r1_immediate_absent_and_error_replies_keep_queued_requester_ownership() {
    for (flags, uuid, result) in [
        (1, "absent", 0),
        (0, "x", -22),
        (1, "", -22),
        (proto::reclaim::RECLAIM_FLAG_FINISH | 1, "x", -22),
        (1, "denied", -1),
    ] {
        let mut f = Fixture::new();
        let mut target = Peer::new(10);
        target.identity.0 = "client.other".into();
        open_metadata(&mut f, 42, &target, &[("uuid", "denied")]);
        let requester = Peer::new(20);
        f.open(43, &requester);
        requester.clear();
        f.sessions.pause_output(true);
        for seq in 0..40 {
            assert!(
                f.sessions
                    .get_mut(43)
                    .unwrap()
                    .send(&MClientSession::renewcaps(seq))
            );
        }
        f.send(
            43,
            &requester,
            proto::MClientReclaim {
                flags,
                uuid: uuid.into(),
            },
        );
        assert_eq!(
            f.sessions.queued_output_bytes(),
            40 * r1_bytes(&requester, &MClientSession::renewcaps(0))
                + r1_bytes(
                    &requester,
                    &proto::MClientReclaimReply {
                        result,
                        ..Default::default()
                    }
                )
        );
        assert!(f.server.recovery_fences(&mut f.sessions).is_empty());
        f.sessions.pause_output(false);
        f.sessions.drain_output(32);
        let mut replacement = Peer::new(21);
        replacement.identity.1 = Bytes::from_static(b"new authorization");
        f.open(43, &replacement);
        f.send(
            43,
            &replacement,
            proto::MClientReclaim {
                flags: 1,
                uuid: "new-absent".into(),
            },
        );
        f.sessions.drain_output(9); // Eight renewals plus the obsolete reply.
        assert!(
            replies(&replacement).is_empty(),
            "obsolete immediate reply: flags={flags}, uuid={uuid}"
        );
        assert_eq!(r1_renewals(&replacement), (32..40).collect::<Vec<_>>());
        assert_eq!(
            f.sessions.queued_output_bytes(),
            r1_bytes(&replacement, &proto::MClientReclaimReply::default())
        );
        f.sessions.drain_output(1);
        assert_eq!(
            replies(&replacement),
            vec![proto::MClientReclaimReply::default()]
        );
        assert_eq!(f.sessions.queued_output_bytes(), 0);
        assert_eq!(f.sessions.get(43).unwrap().push_seq(), 0);
        assert!(!f.sessions.output_failed());
    }
}

#[test]
fn g3_d6_r1_obsolete_completion_cannot_consume_a_valid_new_reclaim() {
    let (mut f, requester, old_fence) = r1_queued_completion();
    let old_receipt = f.server.engine.session_cleanup_seq(old_fence.key);
    let next_target = Peer::new(11);
    open_metadata(&mut f, 44, &next_target, &[("uuid", "next")]);
    let mut replacement = Peer::new(21);
    replacement.identity.1 = Bytes::from_static(b"new authorization");
    f.open(43, &replacement);
    f.send(
        43,
        &replacement,
        proto::MClientReclaim {
            flags: 1,
            uuid: "next".into(),
        },
    );
    let next_fence = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
    assert_eq!(next_fence.key.client, 44);
    assert!(
        f.server
            .finish_recovery_eviction(&mut f.sessions, &next_fence, 13)
    );
    f.drain();
    assert_eq!(
        f.server.runtime.as_ref().unwrap().rows[&next_fence.key].lifecycle,
        SessionLifecycle::Closed
    );
    assert!(
        f.server
            .engine
            .session_cleanup_seq(next_fence.key)
            .is_some()
    );
    f.sessions.drain_output(9);
    assert!(
        replies(&replacement).is_empty(),
        "obsolete response must not complete the new UUID attempt"
    );
    assert!(replies(&requester).is_empty());
    let expected = proto::MClientReclaimReply {
        result: 0,
        epoch: 13,
        addrs: next_fence.addresses,
    };
    assert_eq!(
        f.sessions.queued_output_bytes(),
        r1_bytes(&replacement, &expected)
    );
    f.sessions.drain_output(1);
    assert_eq!(replies(&replacement), vec![expected]);
    assert_eq!(r1_renewals(&replacement), (32..40).collect::<Vec<_>>());
    assert_eq!(
        f.server.engine.session_cleanup_seq(old_fence.key),
        old_receipt
    );
    assert_eq!(
        f.server.engine.checkpoint().unwrap().cleanup_receipts.len(),
        2
    );
    assert_eq!(f.sessions.get(43).unwrap().push_seq(), 0);
    assert_eq!(f.sessions.queued_output_bytes(), 0);
    assert!(!f.sessions.output_failed());
}

#[test]
fn g3_d6_r1_retired_requester_keeps_close_funnel_but_not_old_reclaim() {
    let (mut f, requester, target) = r1_queued_completion();
    let old_key = f.durable_keys[&43];
    f.send(
        43,
        &requester,
        MClientSession::new(SessionOp::REQUEST_CLOSE, 0),
    );
    f.drain();
    assert!(f.sessions.get(43).is_none());
    assert!(f.server.engine.session_cleanup_seq(old_key).is_some());
    assert!(f.sessions.terminal_close_pending(43));
    f.sessions.drain_output(32);
    assert!(replies(&requester).is_empty());
    assert_eq!(r1_renewals(&requester), (0..40).collect::<Vec<_>>());
    assert_eq!(
        MClientSession::from_raw(requester.output.lock().unwrap().last().unwrap().clone())
            .unwrap()
            .op,
        SessionOp::CLOSE
    );
    assert!(!f.sessions.terminal_close_pending(43));
    assert_eq!(f.sessions.queued_output_bytes(), 0);
    let replacement = Peer::new(21);
    f.open(43, &replacement);
    assert_ne!(f.durable_keys[&43], old_key);
    assert!(replies(&replacement).is_empty());
    assert_eq!(
        f.server.runtime.as_ref().unwrap().rows[&target.key].lifecycle,
        SessionLifecycle::Closed
    );
    assert!(!f.sessions.output_failed());
}

#[test]
fn g3_d6_r1_stale_current_requester_can_still_receive_queued_completion() {
    let (mut f, requester, target) = r1_queued_completion();
    let base = Instant::now();
    f.sessions.get_mut(43).unwrap().last_renew = base;
    f.server
        .active_timers(&mut f.sessions, base + Duration::from_secs(60));
    assert_eq!(f.sessions.get(43).unwrap().state, SessionState::Stale);
    f.sessions.drain_output(32);
    assert_eq!(
        replies(&requester),
        vec![proto::MClientReclaimReply {
            result: 0,
            epoch: 9,
            addrs: target.addresses
        }]
    );
    assert_eq!(r1_renewals(&requester), (0..40).collect::<Vec<_>>());
    assert_eq!(f.sessions.get(43).unwrap().push_seq(), 0);
    assert_eq!(f.sessions.queued_output_bytes(), 0);
    assert!(!f.sessions.output_failed());
}

#[tokio::test]
async fn g3_d6_r1_refused_durable_attachment_also_invalidates_older_reclaim_output() {
    let (mut f, requester, _) = r1_queued_completion();
    let key = f.durable_keys[&43];
    let old_open_id = f.server.runtime.as_ref().unwrap().rows[&key]
        .identity
        .open_id;
    let mut replacement = Peer::new(21);
    replacement.identity.1 = Bytes::from_static(b"new authorization");
    f.send(
        43,
        &replacement,
        MClientSession::new(SessionOp::REQUEST_OPEN, 0),
    );
    let update = f.commands.try_recv().unwrap();
    assert!(matches!(update.kind, CommandKind::Open(_)));
    let (objecter, _) = crate::test_backend::data::DataIo::default()
        .objecter()
        .await;
    f.server.set_objecter(objecter.clone());
    let mut map = (*objecter.osdmap()).clone();
    map.epoch += 1;
    let mut blocked = replacement.addr;
    blocked.ty = denc::AddrType::ANY;
    map.blocklist.insert(blocked, denc::UTime::default());
    let epoch = map.epoch;
    objecter.install_map(map);
    objecter
        .wait_for_map_timeout(epoch, Duration::from_secs(1))
        .await
        .unwrap();
    f.complete(update);
    assert_ne!(
        f.server.runtime.as_ref().unwrap().rows[&key]
            .identity
            .open_id,
        old_open_id
    );
    assert_eq!(f.sessions.get(43).unwrap().state, SessionState::Open);
    assert_eq!(
        f.sessions.get(43).unwrap().link.connection_id(),
        requester.connection
    );
    f.sessions.drain_output(32);
    assert!(
        replies(&requester).is_empty(),
        "a newer durable decision invalidates the old output even when its attachment is refused"
    );
    assert!(replies(&replacement).is_empty());
    assert_eq!(r1_renewals(&requester), (0..40).collect::<Vec<_>>());
    assert_eq!(f.sessions.queued_output_bytes(), 0);
    assert!(!f.sessions.output_failed());
    objecter.shutdown();
}
