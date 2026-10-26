use super::*;

fn restored_file() -> (Fixture, Peer, u64, engine::caps::Capability) {
    let mut f = Fixture::new();
    let old = Peer::new(10);
    f.open(42, &old);
    let ino = f.create(42, &old);
    let cap = f.server.engine.cap(ino, 42).unwrap().clone();
    f.cold_restore();
    (f, Peer::new(20), ino, cap)
}

fn timed_reconnect(
    f: &mut Fixture,
    client: u64,
    peer: &Peer,
    message: proto::MClientReconnect,
    received: Instant,
) {
    crate::daemon::dispatch_client_raw_at(
        &mut f.server,
        &mut f.sessions,
        client,
        Box::new(peer.clone()),
        vec![],
        message.to_raw(peer.features()),
        received,
    );
    f.drain();
}

#[test]
fn g3_c1_late_current_active_reconnect_waits_for_verified_fence_and_closed() {
    for (version, waiting_close) in [(3, false), (3, true), (5, false), (5, true)] {
        let mut f = Fixture::new();
        let peer = Peer::new(10);
        f.open(42, &peer);
        let ino = f.create(42, &peer);
        peer.clear();
        if waiting_close {
            f.sessions.pause_output(true);
            f.sessions
                .get_mut(42)
                .unwrap()
                .send(&MClientCaps::default());
            let seq = f.sessions.get(42).unwrap().push_seq();
            f.send(
                42,
                &peer,
                MClientSession::new(SessionOp::REQUEST_CLOSE, seq),
            );
            let key = f.durable_keys[&42];
            assert!(matches!(
                f.server.runtime.as_ref().unwrap().closing[&key].stage,
                CloseStage::WaitSeq
            ));
        }
        let (tx, mut evictions) = mpsc::channel(4);
        f.server.set_evict_channel(tx);
        f.send(
            42,
            &peer,
            proto::MClientReconnect {
                version,
                ..Default::default()
            },
        );
        let target = evictions.try_recv().unwrap();
        assert!(f.server.engine.cap(ino, 42).is_some());
        assert!(peer.output.lock().unwrap().is_empty());
        assert!(
            f.commands.try_recv().is_err(),
            "late reconnect is not cleanup authority"
        );
        f.send(
            42,
            &peer,
            proto::MClientReconnect {
                version,
                ..Default::default()
            },
        );
        assert!(evictions.try_recv().is_err());
        assert!(f.server.finish_eviction(&mut f.sessions, target, 9));
        f.drain();
        f.sessions.pause_output(false);
        f.sessions.drain_output(32);
        assert!(f.sessions.get(42).is_none());
        assert!(f.server.engine.session(42).is_none());
        let output = peer.output.lock().unwrap();
        assert_eq!(
            output
                .iter()
                .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_SESSION
                    && MClientSession::from_raw((*raw).clone()).unwrap().op == SessionOp::CLOSE)
                .count(),
            1
        );
    }
}

#[test]
fn g3_c2_first_final_extends_remaining_gather_but_duplicate_does_not() {
    for version in [3, 5] {
        let mut f = Fixture::new();
        f.open(42, &Peer::new(10));
        f.open(43, &Peer::new(11));
        f.cold_restore();
        let start = f.server.reconnect_deadline().unwrap() - Duration::from_secs(45);
        let a = Peer::new(20);
        let b = Peer::new(21);
        if version == 5 {
            timed_reconnect(
                &mut f,
                42,
                &a,
                proto::MClientReconnect {
                    version,
                    more: true,
                    ..Default::default()
                },
                start,
            );
        }
        let final_chunk = proto::MClientReconnect {
            version,
            ..Default::default()
        };
        timed_reconnect(
            &mut f,
            42,
            &a,
            final_chunk.clone(),
            start + Duration::from_secs(44),
        );
        assert_eq!(f.server.gather_len(), 1);
        assert_eq!(
            f.server.reconnect_deadline().unwrap(),
            start + Duration::from_millis(66500),
            "first valid v{version} final is missing-member activity"
        );
        timed_reconnect(
            &mut f,
            42,
            &a,
            final_chunk.clone(),
            start + Duration::from_secs(64),
        );
        assert_eq!(
            f.server.reconnect_deadline().unwrap(),
            start + Duration::from_millis(66500),
            "completed-member duplicate is not activity"
        );
        f.server
            .expire_reconnect(&mut f.sessions, start + Duration::from_secs(65));
        assert!(f.server.recovery_fences(&mut f.sessions).is_empty());
        timed_reconnect(&mut f, 43, &b, final_chunk, start + Duration::from_secs(65));
        assert_eq!(f.server.gather_len(), 0);
        assert!(!f.server.try_finish_recovery(&mut f.sessions));
    }
}

#[test]
fn g3_c2_first_final_activity_still_has_the_ninety_second_hard_bound() {
    let mut f = Fixture::new();
    for client in 42..=44 {
        f.open(client, &Peer::new(client as u32));
    }
    f.cold_restore();
    let start = f.server.reconnect_deadline().unwrap() - Duration::from_secs(45);
    let a = Peer::new(100);
    let b = Peer::new(101);
    let partial = proto::MClientReconnect {
        more: true,
        ..Default::default()
    };
    timed_reconnect(
        &mut f,
        42,
        &a,
        partial.clone(),
        start + Duration::from_secs(44),
    );
    timed_reconnect(&mut f, 43, &b, partial, start + Duration::from_secs(65));
    timed_reconnect(
        &mut f,
        42,
        &a,
        proto::MClientReconnect::default(),
        start + Duration::from_secs(86),
    );
    assert_eq!(
        f.server.reconnect_deadline().unwrap(),
        start + Duration::from_secs(90)
    );
    timed_reconnect(
        &mut f,
        42,
        &a,
        proto::MClientReconnect::default(),
        start + Duration::from_secs(89),
    );
    assert_eq!(
        f.server.reconnect_deadline().unwrap(),
        start + Duration::from_secs(90)
    );
    f.server
        .expire_reconnect(&mut f.sessions, start + Duration::from_secs(90));
    assert_eq!(f.server.recovery_fences(&mut f.sessions).len(), 2);
    assert_eq!(f.server.gather_len(), 2);
}

#[test]
fn g3_c3_renewal_output_failure_stops_before_already_ready_cleanup_or_worker_transfer() {
    #[derive(Clone)]
    struct FailRenew(Peer);
    impl ClientLink for FailRenew {
        fn connection_id(&self) -> u64 {
            self.0.connection_id()
        }
        fn features(&self) -> u64 {
            self.0.features()
        }
        fn peer_addr(&self) -> EntityAddr {
            self.0.peer_addr()
        }
        fn authenticated_identity(&self) -> Option<(String, Bytes)> {
            self.0.authenticated_identity()
        }
        fn send_raw(&self, raw: RawMessage) -> bool {
            if raw.msg_type == proto::msg_type::CLIENT_SESSION
                && MClientSession::from_raw(raw.clone()).unwrap().op == SessionOp::RENEWCAPS
            {
                return false;
            }
            self.0.send_raw(raw)
        }
    }
    for capacity_failure in [false, true] {
        let mut f = Fixture::new();
        f.open(42, &Peer::new(10));
        f.open(43, &Peer::new(11));
        f.cold_restore();
        let a = Peer::new(20);
        let link = || -> Box<dyn ClientLink> {
            if capacity_failure {
                Box::new(a.clone())
            } else {
                Box::new(FailRenew(a.clone()))
            }
        };
        crate::daemon::dispatch_client_raw(
            &mut f.server,
            &mut f.sessions,
            42,
            link(),
            vec![],
            proto::MClientReconnect {
                more: true,
                ..Default::default()
            }
            .to_raw(a.features()),
        );
        f.drain();
        f.server.request_eviction(&mut f.sessions, 43);
        let target = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
        assert!(
            f.server
                .finish_recovery_eviction(&mut f.sessions, &target, 9)
        );
        let marking = f.commands.try_recv().unwrap();
        assert!(matches!(marking.kind, CommandKind::Closing(_)));
        f.server.pause_execution(true);
        f.complete(marking);
        assert!(matches!(
            f.server.runtime.as_ref().unwrap().closing[&target.key].stage,
            CloseStage::ReadyClean
        ));
        if capacity_failure {
            f.sessions.pause_output(true);
            for seq in 0..1024 {
                assert!(
                    f.sessions
                        .get_mut(42)
                        .unwrap()
                        .send(&MClientSession::renewcaps(seq))
                );
            }
        }
        crate::daemon::dispatch_client_raw(
            &mut f.server,
            &mut f.sessions,
            42,
            link(),
            vec![],
            MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 71).to_raw(a.features()),
        );
        assert!(!f.sessions.output_failed());
        let tail = f.server.engine.submitted_seq();
        assert!(f.commands.try_recv().is_err());
        f.server.pause_execution(false);
        f.server.pump(&mut f.sessions, 32);
        assert!(f.sessions.output_failed());
        assert!(
            f.server.fatal_error().is_some(),
            "renewal send failure must become synchronous fatal state"
        );
        assert_eq!(
            f.server.engine.submitted_seq(),
            tail,
            "no cleanup step or journal submission after failure"
        );
        assert!(
            f.commands.try_recv().is_err(),
            "no worker transfer after failure"
        );
        assert!(f.server.engine.session(43).is_some());
        assert_eq!(f.server.engine.session_cleanup_seq(target.key), None);
        assert!(a.output.lock().unwrap().is_empty());
        assert!(!f.server.has_work());
    }
}
fn chunk(ino: u64, cap_id: u64, more: bool) -> proto::MClientReconnect {
    proto::MClientReconnect {
        caps: [(
            ino,
            proto::CapReconnect {
                cap_id,
                wanted: 0x181,
                issued: 0x181,
                snaprealm: 1,
                ..Default::default()
            },
        )]
        .into(),
        realms: vec![proto::SnaprealmReconnect {
            ino: 1,
            seq: 1,
            parent: 0,
        }],
        more,
        version: 5,
    }
}
fn assert_no_open(peer: &Peer) {
    assert!(
        peer.output
            .lock()
            .unwrap()
            .iter()
            .all(|raw| raw.msg_type != proto::msg_type::CLIENT_SESSION
                || MClientSession::from_raw(raw.clone()).unwrap().op != SessionOp::OPEN)
    );
}

#[test]
fn g3_mixed_chunks_empty_final_replace_locks_and_preserve_history() {
    let (mut f, peer, ino, cap) = restored_file();
    let before = f
        .server
        .engine
        .session(42)
        .unwrap()
        .completed_requests
        .clone();
    let mut part = chunk(ino, cap.cap_id, true);
    part.caps.get_mut(&ino).unwrap().flockbl = proto::FlockState {
        fcntl: vec![proto::FileLock {
            start: 17,
            length: 0,
            client: 999,
            owner: 9,
            pid: 5,
            lock_type: engine::consts::CEPH_LOCK_EXCL,
        }],
        flock: vec![proto::FileLock {
            start: 0,
            length: 0,
            client: 123,
            owner: 4,
            pid: 6,
            lock_type: engine::consts::CEPH_LOCK_SHARED,
        }],
    }
    .encode();
    f.send(42, &peer, part.clone());
    f.drain();
    assert_eq!(f.server.gather_len(), 1);
    assert_no_open(&peer);
    assert_eq!(f.server.engine.cap(ino, 42).unwrap().cap_id, cap.cap_id);
    assert_eq!(f.server.engine.cap(ino, 42).unwrap().last_sent, 0);
    assert_eq!(f.server.engine.file_locks(ino, false)[0].client, 42);
    assert_eq!(f.server.engine.file_locks(ino, true)[0].client, 42);
    f.send(42, &peer, part);
    f.drain();
    assert_eq!(f.server.engine.file_locks(ino, false).len(), 1);
    assert_eq!(f.server.recovery_status().unwrap().locks, 2);
    let bytes = f.server.recovery_status().unwrap().bytes;
    f.send(42, &peer, proto::MClientReconnect::default());
    f.drain();
    assert_eq!(f.server.gather_len(), 0);
    assert_eq!(f.server.recovery_status().unwrap().surviving, 1);
    assert_eq!(f.server.recovery_status().unwrap().complete, 1);
    assert!(f.server.recovery_status().unwrap().bytes >= bytes);
    assert_no_open(&peer);
    let count = peer.output.lock().unwrap().len();
    f.send(42, &peer, proto::MClientReconnect::default());
    f.drain();
    assert_eq!(peer.output.lock().unwrap().len(), count);
    assert_eq!(
        f.server.engine.session(42).unwrap().completed_requests,
        before
    );
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
    assert!(f.server.recovery_status().unwrap().finalization_blocked);
}

#[test]
fn g3_v3_raw_realms_complete_once_without_path_hint_creation() {
    let (mut f, peer, ino, cap) = restored_file();
    let mut msg = chunk(ino, cap.cap_id, false);
    msg.version = 3;
    msg.realms[0].seq = 999;
    f.send(42, &peer, msg.clone());
    f.drain();
    assert_eq!(f.server.gather_len(), 0);
    assert_no_open(&peer);
    let count = peer.output.lock().unwrap().len();
    assert!(count > 0, "stale v3 raw realm must produce UPDATE");
    f.send(42, &peer, msg);
    f.drain();
    assert_eq!(peer.output.lock().unwrap().len(), count);
}

#[test]
fn g3_unlinked_open_and_hardlinked_inodes_reconstruct_by_saved_identity() {
    for hardlinked in [false, true] {
        let mut f = Fixture::new();
        let old = Peer::new(10);
        f.open(42, &old);
        let ino = f.create(42, &old);
        let mut request = proto::MClientRequest::new(
            2,
            if hardlinked {
                MdsOp::LINK
            } else {
                MdsOp::UNLINK
            },
        );
        request.path = WireFilePath::new(1, if hardlinked { "alias" } else { "file" });
        if hardlinked {
            request.path2 = WireFilePath::new(1, "file");
        }
        f.send(42, &old, request);
        f.drain();
        let mut seen = 0;
        for _ in 0..8 {
            let output = old.output.lock().unwrap();
            let batch = output[seen..].to_vec();
            seen = output.len();
            drop(output);
            let mut acknowledged = false;
            for raw in batch {
                if raw.msg_type != proto::msg_type::CLIENT_CAPS {
                    continue;
                }
                let message = MClientCaps::from_raw(raw).unwrap();
                if message.op != CapOp::REVOKE {
                    continue;
                }
                f.send(
                    42,
                    &old,
                    MClientCaps {
                        op: CapOp::UPDATE,
                        ino: message.ino,
                        cap_id: message.cap_id,
                        seq: message.seq,
                        issue_seq: message.issue_seq,
                        caps: message.caps,
                        wanted: message.wanted,
                        migrate_seq: message.migrate_seq,
                        ..Default::default()
                    },
                );
                acknowledged = true;
            }
            f.drain();
            if !acknowledged {
                break;
            }
        }
        let reply = old
            .output
            .lock()
            .unwrap()
            .iter()
            .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|raw| MClientReply::from_raw(raw.clone()).unwrap())
            .find(|reply| reply.tid == 2)
            .unwrap();
        assert_eq!(reply.result, 0);
        let nlink = f.server.engine.inode(ino).unwrap().nlink;
        assert_eq!(nlink, if hardlinked { 2 } else { 0 });
        let cap_id = f.server.engine.cap(ino, 42).unwrap().cap_id;
        f.cold_restore();
        let peer = Peer::new(20);
        let mut message = chunk(ino, cap_id, false);
        let cap = message.caps.get_mut(&ino).unwrap();
        cap.pathbase = 1;
        cap.path = Bytes::from_static(b"untrusted/missing/path/hint");
        cap.snap_follows = 17;
        cap.flockbl = proto::FlockState {
            fcntl: vec![proto::FileLock {
                start: 7,
                length: 0,
                client: 999,
                owner: 3,
                pid: 4,
                lock_type: engine::consts::CEPH_LOCK_EXCL,
            }],
            flock: vec![],
        }
        .encode();
        f.send(42, &peer, message);
        f.drain();
        assert_eq!(f.server.engine.inode(ino).unwrap().nlink, nlink);
        assert_eq!(f.server.engine.cap(ino, 42).unwrap().cap_id, cap_id);
        assert_eq!(
            f.server
                .engine
                .reconnect_cap_context(f.durable_keys[&42], ino),
            Some((1, 17))
        );
        assert_eq!(f.server.engine.file_locks(ino, false)[0].client, 42);
        assert_eq!(f.server.gather_len(), 0);
        assert!(f.server.ready_jobs().is_empty());
        assert_no_open(&peer);
    }
}

#[test]
fn g3_malformed_last_lock_or_semantic_chunk_installs_nothing() {
    for case in 0..4 {
        let (mut f, peer, ino, cap) = restored_file();
        let mut msg = chunk(ino, cap.cap_id, false);
        msg.caps.insert(
            1,
            proto::CapReconnect {
                cap_id: 91,
                snaprealm: 1,
                ..Default::default()
            },
        );
        match case {
            0 => msg.caps.get_mut(&ino).unwrap().flockbl = Bytes::from_static(&[1, 0, 0, 0]),
            1 | 2 => {
                msg.caps.get_mut(&ino).unwrap().flockbl = proto::FlockState {
                    fcntl: vec![proto::FileLock {
                        start: u64::MAX,
                        length: if case == 2 { 3 } else { 0 },
                        client: 99,
                        owner: 8,
                        pid: 3,
                        lock_type: if case == 1 { 99 } else { 1 },
                    }],
                    flock: vec![],
                }
                .encode()
            }
            _ => {
                let mut c = msg.caps.remove(&ino).unwrap();
                c.path = Bytes::from_static(b"file");
                c.pathbase = 1;
                msg.caps.insert(ino + 100, c);
            }
        }
        f.send(42, &peer, msg);
        let attach = f.commands.try_recv().unwrap();
        // Complete the tagged durable attachment. Validation fails before any
        // member of this malformed chunk reaches reconnect_chunk installation.
        let CommandKind::Open(identity) = attach.kind else {
            panic!("expected attachment");
        };
        let key = f.durable_keys[&42];
        f.server.on_storage_event(
            &mut f.sessions,
            attach.owner,
            attach.operation,
            EventKind::Opened(SessionRecord {
                key,
                identity,
                lifecycle: SessionLifecycle::Open,
                cleanup_seq: None,
            }),
        );
        assert!(f.server.fatal_error().is_some(), "case {case}");
        assert!(f.server.engine.cap(1, 42).is_none());
        assert!(f.server.engine.cap(ino, 42).is_none());
        assert!(f.server.engine.file_locks(ino, false).is_empty());
        assert_eq!(f.server.gather_len(), 1);
        assert_no_open(&peer);
    }
}

#[test]
fn g3_reset_partial_and_stale_events_do_not_reset_replacement_generation() {
    let (mut f, a, ino, cap) = restored_file();
    f.send(42, &a, chunk(ino, cap.cap_id, true));
    f.drain();
    let before = f
        .server
        .engine
        .session(42)
        .unwrap()
        .completed_requests
        .clone();
    f.server
        .recovery_connection_gone(&mut f.sessions, 42, a.connection);
    assert!(f.server.engine.cap(ino, 42).is_none());
    assert_eq!(f.server.recovery_status().unwrap().caps, 0);
    assert_eq!(f.server.gather_len(), 1);
    let b = Peer::new(30);
    f.send(42, &b, chunk(ino, cap.cap_id + 1, true));
    f.drain();
    f.server
        .recovery_connection_gone(&mut f.sessions, 42, a.connection);
    f.send(42, &a, proto::MClientReconnect::default());
    assert_eq!(f.server.engine.cap(ino, 42).unwrap().cap_id, cap.cap_id + 1);
    assert_eq!(f.server.gather_len(), 1);
    f.send(42, &b, proto::MClientReconnect::default());
    f.drain();
    f.server
        .recovery_connection_gone(&mut f.sessions, 42, b.connection);
    assert_eq!(f.server.gather_len(), 0);
    assert!(
        f.server.engine.cap(ino, 42).is_some(),
        "final reconstructed rights survive transport loss"
    );
    assert_eq!(
        f.server.engine.session(42).unwrap().completed_requests,
        before
    );
}

#[test]
fn g3_pending_replacement_updates_full_union_without_stale_publication() {
    let (mut f, a, ino, cap) = restored_file();
    f.send(42, &a, chunk(ino, cap.cap_id, true));
    let first = f.commands.try_recv().unwrap();
    let mut b = Peer::new(30);
    b.identity.1 = Bytes::from_static(b"fresh verified authorization");
    f.send(42, &b, chunk(ino, cap.cap_id + 2, true));
    f.complete(first);
    assert!(f.sessions.get(42).is_none());
    assert!(f.server.engine.cap(ino, 42).is_none());
    let replacement = f.commands.try_recv().unwrap();
    assert!(
        matches!(&replacement.kind, CommandKind::Open(row) if row.peer_addresses.0.len() == 3 && row.authorization_context == b.identity.1)
    );
    f.server
        .recovery_connection_gone(&mut f.sessions, 42, a.connection);
    f.complete(replacement);
    assert_eq!(f.sessions.get(42).unwrap().addr, b.addr);
    assert_eq!(f.server.engine.cap(ino, 42).unwrap().cap_id, cap.cap_id + 2);
    assert_no_open(&a);
    assert_no_open(&b);
}

#[test]
fn g3_same_connection_credential_change_needs_another_durable_attachment() {
    let (mut f, a, ino, cap) = restored_file();
    f.send(42, &a, chunk(ino, cap.cap_id, true));
    f.drain();
    let mut b = a.clone();
    b.identity.1 = Bytes::from_static(b"renewed authorization context");
    f.send(42, &b, chunk(ino, cap.cap_id + 1, true));
    assert!(f.server.engine.cap(ino, 42).is_none());
    let update = f.commands.try_recv().unwrap();
    assert!(
        matches!(&update.kind, CommandKind::Open(row) if row.authorization_context == b.identity.1)
    );
    f.complete(update);
    assert_eq!(f.server.engine.cap(ino, 42).unwrap().cap_id, cap.cap_id + 1);
}

#[test]
fn g3_feature_change_at_attachment_completion_denies_reconstruction_and_keeps_union() {
    let (mut f, peer, ino, cap) = restored_file();
    f.send(42, &peer, chunk(ino, cap.cap_id, false));
    let pending = f.commands.try_recv().unwrap();
    let map = monc::MdsMap {
        required_client_features: Bytes::copy_from_slice(
            &(1u64 << proto::features::bit::REPLY_ENCODING).to_le_bytes(),
        ),
        ..Default::default()
    };
    f.server.set_mdsmap(&map);
    f.complete(pending);
    assert!(f.server.engine.cap(ino, 42).is_none());
    assert!(f.sessions.get(42).is_none());
    assert_eq!(f.server.gather_len(), 1);
    let target = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
    assert_eq!(target.addresses.0, vec![Peer::new(10).addr, peer.addr]);
    assert_eq!(
        MClientSession::from_raw(peer.output.lock().unwrap()[0].clone())
            .unwrap()
            .op,
        SessionOp::CLOSE
    );
    assert!(f.commands.try_recv().is_err());
}

#[test]
fn g3_map_change_after_attachment_rechecks_staged_reconstruction() {
    let (mut f, peer, ino, cap) = restored_file();
    f.server.allow_recovery_collection(false);
    f.send(42, &peer, chunk(ino, cap.cap_id, false));
    f.drain();
    assert!(f.sessions.get(42).is_some());
    assert!(f.server.engine.cap(ino, 42).is_none());
    let map = monc::MdsMap {
        required_client_features: Bytes::copy_from_slice(
            &(1u64 << proto::features::bit::REPLY_ENCODING).to_le_bytes(),
        ),
        ..Default::default()
    };
    f.server.set_mdsmap(&map);
    f.server.allow_recovery_collection(true);
    f.drain();
    assert!(
        f.server.engine.cap(ino, 42).is_none(),
        "new map must revoke staged reconstruction permission"
    );
    assert_eq!(f.server.gather_len(), 1);
    assert_eq!(f.server.recovery_fences(&mut f.sessions).len(), 1);
    assert!(
        peer.output.lock().unwrap().is_empty(),
        "current attachment CLOSE must await Closed"
    );
}

#[test]
fn g3_deadline_waits_for_pending_attachment_union_without_publishing_it() {
    let (mut f, peer, ino, cap) = restored_file();
    f.send(42, &peer, chunk(ino, cap.cap_id, true));
    let pending = f.commands.try_recv().unwrap();
    let deadline = f.server.reconnect_deadline().unwrap();
    f.server.expire_reconnect(&mut f.sessions, deadline);
    assert!(
        f.server.recovery_fences(&mut f.sessions).is_empty(),
        "in-flight identity write must settle before fencing"
    );
    f.complete(pending);
    let target = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
    assert_eq!(target.addresses.0, vec![Peer::new(10).addr, peer.addr]);
    assert_eq!(target.attachment, None);
    assert!(f.sessions.get(42).is_none());
    assert!(f.server.engine.cap(ino, 42).is_none());
    assert!(peer.output.lock().unwrap().is_empty());
    assert_eq!(f.server.gather_len(), 1);
}

#[test]
fn g3_post_final_same_connection_new_credentials_cannot_start_second_stream() {
    let (mut f, peer, ino, cap) = restored_file();
    f.send(42, &peer, chunk(ino, cap.cap_id, false));
    f.drain();
    let mut renewed = peer.clone();
    renewed.identity.1 = Bytes::from_static(b"new verified context after final");
    f.send(42, &renewed, chunk(ino, cap.cap_id + 1, false));
    f.drain();
    assert_eq!(f.server.engine.cap(ino, 42).unwrap().cap_id, cap.cap_id);
    assert_eq!(f.server.recovery_fences(&mut f.sessions).len(), 1);
}

#[test]
fn g3_required_cephfs_bits_not_messenger_bits_and_wrong_name_denied() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    let map = monc::MdsMap {
        required_client_features: Bytes::copy_from_slice(
            &(1u64 << proto::features::bit::REPLY_ENCODING).to_le_bytes(),
        ),
        ..Default::default()
    };
    f.server.set_mdsmap(&map);
    f.send(42, &peer, MClientSession::new(SessionOp::REQUEST_OPEN, 0));
    assert!(f.commands.try_recv().is_err());
    assert!(f.sessions.get(42).is_none());
    let mut open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
    open.supported_features = FeatureBitset::from_word(proto::features::CEPH_20_2_FEATURES);
    f.send(42, &peer, open);
    f.drain();
    f.cold_restore();
    f.server.set_mdsmap(&map);
    let mut wrong = Peer::new(20);
    wrong.identity.0 = "client.other".into();
    f.send(42, &wrong, proto::MClientReconnect::default());
    assert!(f.commands.try_recv().is_err());
    let raw = wrong.output.lock().unwrap()[0].clone();
    assert_eq!(MClientSession::from_raw(raw).unwrap().op, SessionOp::CLOSE);
    let valid = Peer::new(30);
    f.send(42, &valid, proto::MClientReconnect::default());
    f.drain();
    assert_eq!(f.server.gather_len(), 0);
    assert_no_open(&valid);
}

#[test]
fn g3_early_flush_retained_and_chunk_read_not_blocked_by_retry_lane() {
    let (mut f, peer, ino, cap) = restored_file();
    let key = f.durable_keys[&42];
    let mut request = proto::MClientRequest::new(1, MdsOp::CREATE);
    request.path = WireFilePath::new(1, "file");
    f.send(42, &peer, request);
    let attachment = f.commands.try_recv().unwrap();
    f.send(
        42,
        &peer,
        MClientCaps {
            op: CapOp::FLUSH,
            ino,
            cap_id: cap.cap_id,
            dirty: proto::types::cap::FILE_WR,
            tid: 99,
            size: 4096,
            osd_epoch_barrier: 17,
            ..Default::default()
        },
    );
    f.send(
        42,
        &peer,
        MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 71),
    );
    f.send(42, &peer, chunk(ino, cap.cap_id, false));
    let tail = f.server.engine.submitted_seq();
    f.complete(attachment);
    f.drain();
    assert_eq!(f.server.gather_len(), 0);
    assert_eq!(f.server.engine.submitted_seq(), tail);
    assert_eq!(f.server.epoch_barrier(), 17);
    assert_eq!(f.server.recovery_status().unwrap().early_inputs, 2);
    assert!(
        f.server
            .engine
            .session(42)
            .unwrap()
            .completed_requests
            .contains_key(&1)
    );
    assert_eq!(f.durable_keys[&42], key);
    assert_no_open(&peer);
    assert!(
        peer.output
            .lock()
            .unwrap()
            .iter()
            .all(|raw| raw.msg_type != proto::msg_type::CLIENT_REPLY
                && raw.msg_type != proto::msg_type::CLIENT_CAPS)
    );
}

#[test]
fn g3_applied_budget_remains_after_raw_release_and_same_inode_replaces_charge() {
    let (mut f, peer, ino, cap) = restored_file();
    f.send(42, &peer, chunk(ino, cap.cap_id, true));
    f.drain();
    let status = f.server.recovery_status().unwrap();
    assert_eq!(status.caps, 1);
    assert_eq!(status.realms, 1);
    assert!(status.bytes > 512);
    for _ in 0..10 {
        f.send(42, &peer, chunk(ino, cap.cap_id, true));
        f.drain();
    }
    assert_eq!(f.server.recovery_status().unwrap().bytes, status.bytes);
    assert!(
        !f.server.has_work(),
        "applied or protected state is not a runnable spin loop"
    );
}

#[test]
fn g3_aggregate_applied_lock_limit_rejects_next_chunk_atomically() {
    let mut f = Fixture::new();
    let old = Peer::new(10);
    f.open(42, &old);
    let ino = f.create(42, &old);
    for client in 43..=46 {
        f.open(client, &Peer::new(client as u32));
    }
    f.cold_restore();
    for client in 42..=46 {
        let peer = Peer::new(100 + client as u32);
        let count = if client == 46 { 1 } else { 16384 };
        let mut message = chunk(ino, 1000 + client, true);
        message.caps.get_mut(&ino).unwrap().flockbl = proto::FlockState {
            fcntl: (0..count)
                .map(|index| proto::FileLock {
                    start: index,
                    length: 0,
                    client: 999,
                    owner: index,
                    pid: 5,
                    lock_type: engine::consts::CEPH_LOCK_SHARED,
                })
                .collect(),
            flock: vec![],
        }
        .encode();
        f.send(client, &peer, message);
        if client != 46 {
            f.drain();
            continue;
        }
        let pending = f.commands.try_recv().unwrap();
        let CommandKind::Open(identity) = pending.kind else {
            panic!("expected attachment");
        };
        f.server.on_storage_event(
            &mut f.sessions,
            pending.owner,
            pending.operation,
            EventKind::Opened(SessionRecord {
                key: f.durable_keys[&client],
                identity,
                lifecycle: SessionLifecycle::Open,
                cleanup_seq: None,
            }),
        );
    }
    assert!(
        f.server
            .fatal_error()
            .unwrap()
            .contains("applied reconnect aggregate budget")
    );
    assert_eq!(f.server.recovery_status().unwrap().locks, 65536);
    assert_eq!(f.server.engine.file_locks(ino, false).len(), 65536);
    assert!(f.server.engine.cap(ino, 46).is_none());
    assert_eq!(f.server.gather_len(), 5);
}

#[tokio::test]
async fn g3_missing_durable_key_failed_fence_retains_gather_until_closed_chain() {
    use crate::fencing::{FenceBackend, fence};
    let mut f = Fixture::new();
    let old = Peer::new(10);
    f.open(42, &old);
    let ino = f.create(42, &old);
    let second = Peer::new(15);
    f.open(42, &second);
    f.cold_restore();
    let key = f.durable_keys[&42];
    let history = f
        .server
        .engine
        .session(42)
        .unwrap()
        .completed_requests
        .clone();
    let deadline = f.server.reconnect_deadline().unwrap();
    f.server.expire_reconnect(&mut f.sessions, deadline);
    let target = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
    assert_eq!(target.key, key);
    assert_eq!(target.attachment, None);
    assert_eq!(target.addresses.0, vec![old.addr, second.addr]);
    assert!(
        f.sessions.get(42).is_none(),
        "missing durable key must not need a fabricated connection"
    );
    let memory = crate::test_backend::MemoryBackend::default();
    let mut failed = crate::test_backend::Fence::new(memory.clone());
    failed.fail = true;
    assert!(fence(&failed, &target.addresses.0[0], 9).await.is_err());
    assert_eq!(f.server.gather_len(), 1);
    assert!(f.commands.try_recv().is_err());
    assert_eq!(
        f.server.engine.session(42).unwrap().completed_requests,
        history
    );
    let successful = crate::test_backend::Fence::new(memory);
    let mut epoch = 9;
    for addr in &target.addresses.0 {
        epoch = epoch.max(fence(&successful, addr, epoch).await.unwrap());
    }
    successful.raise_barrier(epoch);
    let mut stale = target.clone();
    stale.owner.gid += 1;
    assert!(
        !f.server
            .finish_recovery_eviction(&mut f.sessions, &stale, epoch)
    );
    assert!(
        !f.server
            .finish_recovery_eviction(&mut f.sessions, &target, 0)
    );
    assert!(
        f.server
            .finish_recovery_eviction(&mut f.sessions, &target, epoch)
    );
    let closing = f.commands.try_recv().unwrap();
    assert!(matches!(closing.kind, CommandKind::Closing(k) if k == key));
    assert_eq!(
        f.server.engine.session(42).unwrap().completed_requests,
        history
    );
    f.complete(closing);
    for _ in 0..64 {
        assert_eq!(f.server.gather_len(), 1);
        let command = f.commands.try_recv().unwrap();
        if matches!(command.kind, CommandKind::Closed { key: k, .. } if k == key) {
            assert!(f.server.engine.session_cleanup_seq(key).is_some());
            f.complete(command);
            break;
        }
        assert!(matches!(command.kind, CommandKind::Append { .. }));
        f.complete(command);
    }
    assert_eq!(f.server.gather_len(), 0);
    assert!(f.server.engine.session_cleanup_seq(key).is_some());
    assert!(f.server.engine.session(42).is_none());
    assert!(
        f.server.engine.inode(ino).is_some(),
        "data recovery intent must remain durable, not purge before finalization"
    );
    assert!(f.server.ready_jobs().is_empty());
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
}

#[test]
fn g3_requested_window_does_not_apply_until_confirmed_and_deadline_is_bounded() {
    let (mut f, peer, ino, cap) = restored_file();
    f.server.allow_recovery_collection(false);
    f.send(42, &peer, chunk(ino, cap.cap_id, true));
    f.drain();
    assert!(f.sessions.get(42).is_some());
    assert!(f.server.engine.cap(ino, 42).is_none());
    f.server.allow_recovery_collection(true);
    f.drain();
    assert!(f.server.engine.cap(ino, 42).is_some());
    let original = f.server.reconnect_deadline().unwrap();
    let start = original - Duration::from_secs(45);
    let partial = chunk(ino, cap.cap_id, true);
    crate::daemon::dispatch_client_raw_at(
        &mut f.server,
        &mut f.sessions,
        42,
        Box::new(peer.clone()),
        vec![],
        partial.to_raw(peer.features()),
        start + Duration::from_secs(44),
    );
    assert_eq!(
        f.server.reconnect_deadline().unwrap(),
        start + Duration::from_millis(66500)
    );
    f.server.recovery_input(
        &mut f.sessions,
        42,
        Box::new(peer.clone()),
        ClientMessage::Reconnect(chunk(ino, cap.cap_id, true)),
        vec![],
        1024,
        start + Duration::from_secs(88),
    );
    assert_eq!(
        f.server.reconnect_deadline().unwrap(),
        start + Duration::from_secs(90)
    );
    f.server
        .expire_reconnect(&mut f.sessions, start + Duration::from_secs(90));
    f.send(42, &peer, proto::MClientReconnect::default());
    f.drain();
    assert_eq!(f.server.gather_len(), 1);
    assert!(f.server.reconnect_deadline().is_none());
    assert_eq!(f.server.recovery_fences(&mut f.sessions).len(), 1);
}

#[test]
fn g3_post_final_replacement_keeps_rights_but_second_stream_requires_fenced_close() {
    let (mut f, a, ino, cap) = restored_file();
    f.send(42, &a, chunk(ino, cap.cap_id, false));
    f.drain();
    f.server
        .recovery_connection_gone(&mut f.sessions, 42, a.connection);
    let b = Peer::new(30);
    f.server
        .recovery_accepted(&mut f.sessions, 42, Box::new(b.clone()), vec![]);
    f.drain();
    assert_eq!(f.server.gather_len(), 0);
    assert_eq!(f.server.engine.cap(ino, 42).unwrap().cap_id, cap.cap_id);
    assert_eq!(f.sessions.get(42).unwrap().addr, b.addr);
    assert_no_open(&b);
    f.send(42, &b, chunk(ino, cap.cap_id + 1, false));
    f.drain();
    assert_eq!(f.server.engine.cap(ino, 42).unwrap().cap_id, cap.cap_id);
    assert_eq!(f.server.recovery_fences(&mut f.sessions).len(), 1);
}

#[test]
fn g3_late_current_reconnect_closes_only_after_closed_and_keeps_funnel_order() {
    for during_publication in [false, true] {
        let (mut f, peer, ino, cap) = restored_file();
        f.send(42, &peer, chunk(ino, cap.cap_id, true));
        f.drain();
        f.sessions.pause_output(true);
        f.sessions
            .get_mut(42)
            .unwrap()
            .send(&MClientSession::renewcaps(71));
        let deadline = f.server.reconnect_deadline().unwrap();
        f.server.expire_reconnect(&mut f.sessions, deadline);
        let target = f.server.recovery_fences(&mut f.sessions).pop().unwrap();
        if !during_publication {
            f.send(42, &peer, proto::MClientReconnect::default());
        }
        assert!(
            f.server
                .finish_recovery_eviction(&mut f.sessions, &target, 9)
        );
        let marking = f.commands.try_recv().unwrap();
        assert!(matches!(marking.kind, CommandKind::Closing(_)));
        f.complete(marking);
        let closed = loop {
            let command = f.commands.try_recv().unwrap();
            if matches!(command.kind, CommandKind::Closed { .. }) {
                break command;
            }
            assert!(matches!(command.kind, CommandKind::Append { .. }));
            f.complete(command);
        };
        if during_publication {
            f.send(42, &peer, proto::MClientReconnect::default());
        }
        assert!(
            peer.output.lock().unwrap().is_empty(),
            "CLOSE cannot precede durable Closed"
        );
        f.complete(closed);
        assert!(f.sessions.get(42).is_none());
        assert!(
            !f.sessions.remove(42),
            "terminal CLOSE must retain its own funnel"
        );
        f.sessions.pause_output(false);
        f.sessions.drain_output(32);
        let output = peer.output.lock().unwrap();
        let messages: Vec<_> = output
            .iter()
            .cloned()
            .map(|raw| MClientSession::from_raw(raw).unwrap())
            .collect();
        assert_eq!(
            messages.iter().map(|m| m.op).collect::<Vec<_>>(),
            vec![SessionOp::RENEWCAPS, SessionOp::CLOSE]
        );
        assert_eq!(messages[0].seq, 71);
        assert_eq!(f.server.gather_len(), 0);
        assert!(!f.server.recovered());
    }
}

#[test]
fn g3_late_unmatched_reconnect_uses_terminal_without_rebinding_current_attachment() {
    let (mut f, current, ino, cap) = restored_file();
    f.send(42, &current, chunk(ino, cap.cap_id, true));
    f.drain();
    let deadline = f.server.reconnect_deadline().unwrap();
    f.server.expire_reconnect(&mut f.sessions, deadline);
    let late = Peer::new(30);
    f.send(42, &late, proto::MClientReconnect::default());
    assert_eq!(f.sessions.get(42).unwrap().addr, current.addr);
    assert_eq!(f.server.gather_len(), 1);
    assert!(f.commands.try_recv().is_err());
    let output = late.output.lock().unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(
        MClientSession::from_raw(output[0].clone()).unwrap().op,
        SessionOp::CLOSE
    );
}

#[test]
fn g3_recovery_terminal_cannot_bypass_same_connection_output() {
    let (mut f, peer, ino, cap) = restored_file();
    f.send(42, &peer, chunk(ino, cap.cap_id, true));
    f.drain();
    f.sessions.pause_output(true);
    f.sessions
        .get_mut(42)
        .unwrap()
        .send(&MClientSession::renewcaps(77));
    f.sessions.pause_output(false);
    f.sessions.recovery_terminal(
        42,
        Box::new(peer.clone()),
        MClientSession::reject("denied OPEN", false),
    );
    assert!(
        peer.output.lock().unwrap().is_empty(),
        "terminal response overtook queued same-connection output"
    );
    f.sessions.drain_output(32);
    let output = peer.output.lock().unwrap();
    let first = MClientSession::from_raw(output[0].clone()).unwrap();
    assert_eq!((first.op, first.seq), (SessionOp::RENEWCAPS, 77));
    assert_eq!(
        MClientSession::from_raw(output[1].clone()).unwrap().op,
        SessionOp::REJECT
    );
}

#[test]
fn g3_eviction_effect_uses_detached_runtime_and_never_active_only_fence_queue() {
    for attached in [false, true] {
        let (mut f, peer, ino, cap) = restored_file();
        if attached {
            f.send(42, &peer, chunk(ino, cap.cap_id, true));
            f.drain();
        }
        let (tx, mut legacy) = mpsc::channel(8);
        f.server.set_evict_channel(tx);
        let mut runtime = f.server.runtime.take().unwrap();
        f.server.consume_step(
            &mut f.sessions,
            &mut runtime,
            EngineStep {
                effects: vec![Effect::EvictClient { client: 42 }],
                notices: vec![],
                state: StepState::Idle,
                unused_permit: None,
            },
        );
        f.server.runtime = Some(runtime);
        assert!(
            legacy.try_recv().is_err(),
            "recovery eviction cannot wait for ACTIVE"
        );
        assert_eq!(f.server.recovery_fences(&mut f.sessions).len(), 1);
        assert_eq!(f.server.gather_len(), 1);
        assert!(f.commands.try_recv().is_err());
        assert!(f.server.fatal_error().is_none());
    }
}

#[test]
fn g3_nonempty_grant_effect_fails_closed_without_retaining_stale_payload() {
    let (mut f, peer, ino, cap) = restored_file();
    f.send(42, &peer, chunk(ino, cap.cap_id, false));
    f.drain();
    let mut runtime = f.server.runtime.take().unwrap();
    f.server.consume_step(
        &mut f.sessions,
        &mut runtime,
        EngineStep {
            effects: vec![Effect::Caps(engine::effects::CapMessage {
                client: 42,
                op: engine::consts::CEPH_CAP_OP_GRANT,
                ino,
                realm: 1,
                cap_id: cap.cap_id,
                seq: 1,
                issue_seq: 1,
                caps: 0x181,
                wanted: 0x181,
                dirty: 0,
                mseq: 0,
                flush_tid: 0,
                oldest_flush_tid: 0,
                inode: None,
            })],
            notices: vec![],
            state: StepState::Idle,
            unused_permit: None,
        },
    );
    f.server.runtime = Some(runtime);
    assert!(
        f.server
            .fatal_error()
            .unwrap()
            .contains("nonempty recovery drain")
    );
    assert!(peer.output.lock().unwrap().is_empty());
    assert_eq!(f.sessions.queued_output_bytes(), 0);
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
}

#[test]
fn g3_reconnect_message_and_retained_early_input_budgets_fail_closed() {
    let (mut f, peer, _ino, _cap) = restored_file();
    let mut request = proto::MClientRequest::new(1000, MdsOp::LOOKUP);
    request.path = WireFilePath::new(1, "file");
    f.server.recovery_input(
        &mut f.sessions,
        42,
        Box::new(peer.clone()),
        ClientMessage::Request(request),
        vec![],
        (1 << 20) + 1,
        Instant::now(),
    );
    assert!(f.server.fatal_error().is_some());
    assert!(f.commands.try_recv().is_err());
    let (mut f, peer, _ino, _cap) = restored_file();
    for tid in 1000..2000 {
        if f.server.fatal_error().is_some() {
            break;
        }
        let mut request = proto::MClientRequest::new(tid, MdsOp::LOOKUP);
        request.path = WireFilePath::new(1, "file");
        f.send(42, &peer, request);
    }
    assert!(f.server.fatal_error().is_some());
    assert_eq!(f.server.gather_len(), 1);
    assert_no_open(&peer);
}

fn mkdir(f: &mut Fixture, peer: &Peer, tid: u64, path: &str) -> u64 {
    let mut msg = proto::MClientRequest::new(tid, MdsOp::MKDIR);
    msg.path = WireFilePath::new(1, path.to_owned());
    msg.head.flags = proto::request::request_flags::WANT_DENTRY;
    f.send(42, peer, msg);
    f.drain();
    peer.output
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.msg_type == proto::msg_type::CLIENT_REPLY)
        .map(|r| MClientReply::from_raw(r.clone()).unwrap())
        .find(|r| r.tid == tid)
        .unwrap()
        .trace
        .target
        .unwrap()
        .ino
}
fn vxattr(f: &mut Fixture, peer: &Peer, tid: u64, ino: u64, name: &str, value: &'static [u8]) {
    let mut msg = proto::MClientRequest::new(tid, MdsOp::SETXATTR);
    msg.path = WireFilePath::new(ino, "");
    msg.path2 = WireFilePath::new(0, name.to_owned());
    msg.data = Bytes::from_static(value);
    f.send(42, peer, msg);
    f.drain();
    let out = peer.output.lock().unwrap();
    let reply = out
        .iter()
        .filter(|r| r.msg_type == proto::msg_type::CLIENT_REPLY)
        .map(|r| MClientReply::from_raw(r.clone()).unwrap())
        .find(|r| r.tid == tid)
        .unwrap();
    assert_eq!(reply.result, 0, "vxattr {name}");
}

#[test]
fn g3_realm_split_and_updates_use_authoritative_nested_topology_and_funnel_sequence() {
    for feature_word in [0, 1 << proto::features::bit::NEW_SNAPREALM_INFO] {
        let mut f = Fixture::new();
        let old = Peer::new(10);
        let mut open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
        open.supported_features = FeatureBitset::from_word(feature_word);
        f.send(42, &old, open);
        f.drain();
        let quota = mkdir(&mut f, &old, 2, "quota");
        vxattr(&mut f, &old, 3, quota, "ceph.quota.max_bytes", b"1000000");
        let nested = mkdir(&mut f, &old, 4, "quota/nested");
        vxattr(&mut f, &old, 5, nested, "ceph.dir.subvolume", b"1");
        let mut create = proto::MClientRequest::new(1, MdsOp::CREATE);
        create.path = WireFilePath::new(quota, "file");
        create.head.flags = proto::request::request_flags::WANT_DENTRY;
        create.head.args = proto::request::OpenArgs {
            flags: proto::open_flags::CREAT | proto::open_flags::RDWR,
            mode: 0o644,
            pool: 2,
            ..Default::default()
        }
        .to_args();
        f.send(42, &old, create);
        f.drain();
        let ino = old
            .output
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|r| MClientReply::from_raw(r.clone()).unwrap())
            .find(|r| r.tid == 1)
            .unwrap()
            .trace
            .target
            .unwrap()
            .ino;
        assert_eq!(f.server.engine.snap_trace(ino)[0].ino, quota);
        let cap = f.server.engine.cap(ino, 42).unwrap().clone();
        f.cold_restore();
        let peer = Peer::new(20);
        let mut msg = chunk(ino, cap.cap_id, false);
        msg.realms = vec![
            proto::SnaprealmReconnect {
                ino: 1,
                seq: 999,
                parent: 0,
            },
            proto::SnaprealmReconnect {
                ino: quota,
                seq: 999,
                parent: 999,
            },
            proto::SnaprealmReconnect {
                ino: nested,
                seq: 999,
                parent: 999,
            },
        ];
        f.send(42, &peer, msg);
        f.drain();
        assert_no_open(&peer);
        let output = peer.output.lock().unwrap().clone();
        let format = SnapFormat::for_client(&FeatureBitset::from_word(feature_word));
        let snaps: Vec<_> = output
            .into_iter()
            .filter(|r| r.msg_type == proto::msg_type::CLIENT_SNAP)
            .map(|r| proto::MClientSnap::from_raw(r).unwrap())
            .collect();
        let split = snaps
            .iter()
            .find(|s| s.op == proto::snap::SnapOp::SPLIT)
            .unwrap();
        assert_eq!(split.split, quota);
        assert_eq!(split.split_inos, vec![ino]);
        assert_eq!(split.split_realms, vec![nested]);
        for snap in &snaps {
            let trace = SnapTrace::decode(snap.trace.clone(), format).unwrap();
            assert!(!trace.realms.is_empty());
            assert!(trace.is_well_formed());
        }
        assert_eq!(
            snaps
                .iter()
                .filter(|s| s.op == proto::snap::SnapOp::UPDATE)
                .count(),
            3
        );
        assert_eq!(f.sessions.get(42).unwrap().push_seq(), snaps.len() as u64);
        // Only the funnel's typed OPEN encoding is tested here. Production
        // nonempty finalization cannot call it until the accepted E2 hook exists.
        f.sessions
            .get_mut(42)
            .unwrap()
            .send(&MClientSession::open_default(0, vec![]));
        let last = peer.output.lock().unwrap().last().unwrap().clone();
        assert_eq!(
            MClientSession::from_raw(last).unwrap().seq,
            snaps.len() as u64
        );
    }
}
