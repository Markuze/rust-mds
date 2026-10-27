//! D5 routing and actual-worker tests. Neither is native RADOS evidence.
use super::*;

fn finish(f: &mut Fixture) {
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
    assert!(f.server.recovery_structural_done());
    f.server.allow_recovery_finalization(true); // confirmed REJOIN in this routing fixture
    f.server.allow_recovery_requests(true); // confirmed CLIENTREPLAY
    f.drain();
    assert!(f.server.try_finish_recovery(&mut f.sessions));
}
fn reconnect(f: &mut Fixture, peer: &Peer, ino: u64, cap_id: u64) {
    let mut message = proto::MClientReconnect::default();
    message.caps.insert(
        ino,
        proto::CapReconnect {
            cap_id,
            wanted: engine::consts::CEPH_CAP_FILE_RD,
            issued: 0,
            snaprealm: 1,
            ..Default::default()
        },
    );
    f.send(42, peer, message);
    f.drain();
}
fn setup() -> (Fixture, Peer, u64, u64) {
    let mut f = Fixture::new();
    let old = Peer::new(10);
    f.open(42, &old);
    let ino = f.create(42, &old);
    let cap_id = f.server.engine.cap(ino, 42).unwrap().cap_id;
    f.cold_restore();
    (f, Peer::new(20), ino, cap_id)
}

#[test]
fn d5_nonempty_marker_precedes_final_open_and_transport_readiness() {
    let (mut f, peer, ino, cap_id) = setup();
    reconnect(&mut f, &peer, ino, cap_id);
    assert!(peer.output.lock().unwrap().is_empty());
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
    f.server.allow_recovery_finalization(true);
    f.sessions.pause_output(true);
    f.drain();
    assert_eq!(
        f.server.runtime.as_ref().unwrap().recovery_drain,
        RecoveryDrain::GrantsReleased
    );
    assert!(!f.server.recovered());
    assert!(peer.output.lock().unwrap().is_empty());
    f.sessions.pause_output(false);
    f.sessions.drain_output(1);
    f.drain();
    assert!(
        !f.server.recovered(),
        "accepted OPEN alone does not drain later grants"
    );
    f.sessions.drain_output(32);
    f.drain();
    assert!(f.server.try_finish_recovery(&mut f.sessions));
    let output = peer.output.lock().unwrap();
    let open = output
        .iter()
        .position(|r| {
            r.msg_type == proto::msg_type::CLIENT_SESSION
                && MClientSession::from_raw(r.clone()).unwrap().op == SessionOp::OPEN
        })
        .unwrap();
    assert_eq!(open, 0, "final OPEN precedes released grant/reply output");
    assert_eq!(
        output
            .iter()
            .filter(|r| r.msg_type == proto::msg_type::CLIENT_SESSION
                && MClientSession::from_raw((*r).clone()).unwrap().op == SessionOp::OPEN)
            .count(),
        1
    );
}

#[test]
fn d5_committed_retry_keeps_original_wire_identity_and_precedes_fresh_mutation() {
    for replay in [false, true] {
        let (mut f, peer, ino, cap_id) = setup();
        let mut retry = proto::MClientRequest::new(1, MdsOp::CREATE);
        retry.path = WireFilePath::new(1, "file");
        retry.head.flags = proto::request::request_flags::WANT_DENTRY;
        if replay {
            retry.head.flags |= proto::request::request_flags::REPLAY;
        }
        retry.head.args = proto::request::OpenArgs {
            flags: proto::open_flags::CREAT | proto::open_flags::RDWR,
            mode: 0o644,
            pool: 2,
            ..Default::default()
        }
        .to_args();
        f.send(42, &peer, retry);
        let mut fresh = proto::MClientRequest::new(2, MdsOp::UNLINK);
        fresh.path = WireFilePath::new(1, "file");
        f.send(42, &peer, fresh);
        reconnect(&mut f, &peer, ino, cap_id);
        let tail = f.server.engine.submitted_seq();
        finish(&mut f);
        assert_eq!(
            f.server.engine.submitted_seq(),
            tail,
            "retry does not repeat the creator or execute fresh unlink"
        );
        let output = peer.output.lock().unwrap();
        let reply = output
            .iter()
            .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|raw| MClientReply::from_raw(raw.clone()).unwrap())
            .find(|r| r.tid == 1)
            .unwrap();
        assert_eq!((reply.op, reply.result), (MdsOp::CREATE, 0));
        assert_eq!(
            reply.trace.target.as_ref().map(|i| i.ino),
            (!replay).then_some(ino)
        );
        assert_eq!(reply.snapbl.is_empty(), replay);
        assert!(matches!(reply.extra, proto::ReplyExtra::Create(ref c) if c.created_ino == ino));
        drop(output);
        f.server.set_runtime_permissions(true, true, true); // confirmed ACTIVE
        f.drain();
        assert!(
            f.server.pending_requests.contains_key(&(42, 2)),
            "fresh mutation enters the engine only after confirmed ACTIVE; cap acknowledgments can still delay its record"
        );
    }
}

#[test]
fn d5_raw_control_cut_and_held_permit_do_not_release_the_marker() {
    let (mut f, peer, ino, cap_id) = setup();
    reconnect(&mut f, &peer, ino, cap_id);
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
    f.server.allow_recovery_finalization(true);
    f.server.set_recovery_raw_cut(true);
    let tail = f.server.engine.submitted_seq();
    f.drain();
    assert_eq!(
        f.server.runtime.as_ref().unwrap().recovery_drain,
        RecoveryDrain::Controls
    );
    assert!(peer.output.lock().unwrap().is_empty());
    let permit = f.server.engine.journal_mut().reserve().unwrap();
    f.server.set_recovery_raw_cut(false);
    f.server.pump(&mut f.sessions, 1);
    assert!(!f.server.recovered());
    f.server.engine.journal_mut().release(Some(permit));
    f.drain();
    assert!(f.server.try_finish_recovery(&mut f.sessions));
    assert_eq!(f.server.engine.submitted_seq(), tail);
}

#[test]
fn d5_post_final_replacement_rebuilds_unsent_open_after_stale_discard() {
    let mut f = Fixture::new();
    let old = Peer::new(10);
    f.open(42, &old);
    f.cold_restore();
    let a = Peer::new(20);
    f.send(42, &a, proto::MClientReconnect::default());
    f.drain();
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
    f.server.allow_recovery_finalization(true);
    f.sessions.pause_output(true);
    f.drain();
    assert!(!f.server.recovered());
    assert!(f.sessions.queued_output_bytes() > 0);
    f.server
        .recovery_connection_gone(&mut f.sessions, 42, a.connection);
    let b = Peer::new(30);
    f.send(
        42,
        &b,
        MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 20),
    );
    f.drain();
    f.sessions.pause_output(false);
    f.sessions.drain_output(1);
    assert!(a.output.lock().unwrap().is_empty());
    assert!(
        b.output.lock().unwrap().is_empty(),
        "discard is not an accepted OPEN"
    );
    f.server.pump(&mut f.sessions, 1);
    assert!(!f.server.recovered());
    f.sessions.drain_output(32);
    f.drain();
    assert!(f.server.try_finish_recovery(&mut f.sessions));
    assert!(f.server.recovery_fences(&mut f.sessions).is_empty());
    let output = b.output.lock().unwrap();
    let opens: Vec<_> = output
        .iter()
        .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_SESSION)
        .map(|raw| MClientSession::from_raw(raw.clone()).unwrap())
        .filter(|msg| msg.op == SessionOp::OPEN)
        .collect();
    assert_eq!(opens.len(), 1);
    assert_eq!(opens[0].seq, 0);
}

#[test]
fn d5_terminal_control_capacity_preserves_input_charge_write_commands_and_latch() {
    let mut config = EngineConfig::default();
    config.retry_limits.max_bytes = 65536;
    let mut f = Fixture::unready_config(config);
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
    f.drain();
    assert!(f.server.try_finish_recovery(&mut f.sessions));
    f.server.pause_data_work(false);
    let peer = Peer::new(10);
    f.open(42, &peer);
    let ino = f.create(42, &peer);
    let mut flush = f.flush(42, ino);
    flush.dirty = proto::types::cap::XATTR_EXCL;
    flush.xattr_version = f.server.engine.inode(ino).unwrap().xattr_version + 1;
    flush.xattrbl = proto::xattrs_to_blob(
        &[(
            Bytes::from_static(b"user.capacity"),
            Bytes::from(vec![0x61; 32768]),
        )]
        .into(),
    );
    let before = f.server.engine.inode(ino).unwrap().clone();
    let tail = f.server.engine.submitted_seq();
    peer.clear();
    f.send(42, &peer, flush);
    assert_eq!(f.server.engine.failure(), Some(JournalError::Capacity));
    assert!(f.server.fatal_error().unwrap().contains("capacity"));
    assert_eq!(f.server.engine.inode(ino).unwrap(), &before);
    assert_eq!(f.server.engine.submitted_seq(), tail);
    let r = f.server.runtime.as_ref().unwrap();
    assert_eq!(r.terminal_hold.len(), 1);
    assert!(matches!(
        r.terminal_hold[0].input,
        EngineInput::CapUpdate { .. }
    ));
    assert_eq!(r.terminal_hold[0].bytes, r.control_bytes);
    assert!(!f.server.has_work());
    assert!(f.server.ready_jobs().is_empty());
    assert!(f.commands.try_recv().is_err());
    assert!(peer.output.lock().unwrap().is_empty());
    f.server.pump(&mut f.sessions, 32);
    assert_eq!(f.server.runtime.as_ref().unwrap().terminal_hold.len(), 1);
}

#[test]
fn d5_external_capacity_waiters_get_one_finite_pass_per_real_epoch() {
    let mut config = EngineConfig::default();
    config.retry_limits.max_completions = 1;
    let mut f = Fixture::unready_config(config);
    assert!(!f.server.try_finish_recovery(&mut f.sessions));
    f.drain();
    assert!(f.server.try_finish_recovery(&mut f.sessions));
    f.server.pause_data_work(false);
    let peer = Peer::new(10);
    f.open(42, &peer);
    f.create(42, &peer);
    for tid in [100, 101] {
        let mut request = proto::MClientRequest::new(tid, MdsOp::MKDIR);
        request.path = WireFilePath::new(1, format!("wait{tid}"));
        f.send(42, &peer, request);
    }
    let r = f.server.runtime.as_ref().unwrap();
    assert_eq!(r.retry_wait.len(), 2);
    let epoch = r.capacity_epoch;
    assert!(r.retry_wait.iter().all(|p| p.epoch == epoch));
    assert!(!f.server.has_work());
    for _ in 0..3 {
        f.server.tick(&mut f.sessions);
        f.drain();
    }
    assert_eq!(f.server.runtime.as_ref().unwrap().capacity_epoch, epoch);
    assert!(!f.server.has_work());
    // A fitting watermark mutation passes both external waiters. Only commit
    // changes the capacity epoch; a read-only floor or a Tick did not do so.
    let mut retire = proto::MClientRequest::new(4, MdsOp::MKDIR);
    retire.path = WireFilePath::new(1, "retire");
    retire.head.oldest_client_tid = 4;
    f.send(42, &peer, retire);
    f.drain();
    let r = f.server.runtime.as_ref().unwrap();
    assert!(r.capacity_epoch > epoch);
    assert!(r.retry_wait.iter().all(|p| p.epoch == r.capacity_epoch));
    assert!(!f.server.has_work());
}

#[test]
fn d5_owned_identity_return_without_permit_uses_the_original_origin() {
    for floor in [99, 101] {
        let mut config = EngineConfig::default();
        config.retry_limits.max_bytes = 256 + 4096 + 272 + 1 + 2 * (304 + 6) + 8192 + 1024;
        let mut f = Fixture::unready_config(config);
        assert!(!f.server.try_finish_recovery(&mut f.sessions));
        f.drain();
        assert!(f.server.try_finish_recovery(&mut f.sessions));
        f.server.pause_data_work(false);
        let peer = Peer::new(10);
        f.open(42, &peer);
        let node = |tid, name: &str| {
            let mut req = proto::MClientRequest::new(tid, MdsOp::MKNOD);
            req.path = WireFilePath::new(1, name.to_string());
            req.head.flags = proto::request::request_flags::WANT_DENTRY;
            req.head.args = proto::request::MknodArgs {
                mode: 0o100644,
                rdev: 0,
            }
            .to_args();
            req
        };
        let mut create = node(10, "f");
        create.data =
            proto::xattrs_to_blob(&[(Bytes::from_static(b"user.k"), Bytes::new())].into());
        f.send(42, &peer, create);
        f.drain();
        let ino = peer
            .output
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|r| MClientReply::from_raw(r.clone()).unwrap())
            .find(|r| r.tid == 10)
            .unwrap()
            .trace
            .target
            .unwrap()
            .ino;
        let mut r = f.server.runtime.take().unwrap();
        f.server.queue_message_at(
            &mut f.sessions,
            &mut r,
            42,
            ClientMessage::Request(node(100, "waiting")),
            Instant::now(),
            None,
        );
        let q = r.normal.pop_front().unwrap();
        let original_bytes = q.bytes;
        r.owned.insert(q.origin.serial, q.origin);
        let step = f.server.engine.step(Some(q.input), None).unwrap();
        f.server.consume_step(&mut f.sessions, &mut r, step);
        let cap = f.server.engine.cap(ino, 42).unwrap();
        let control = EngineInput::CapUpdate {
            key: r.keys[&42],
            update: engine::CapUpdate {
                op: engine::consts::CEPH_CAP_OP_FLUSH,
                ino,
                cap_id: cap.cap_id,
                seq: cap.last_sent,
                issue_seq: cap.last_issue,
                caps: cap.pending(),
                wanted: engine::consts::CEPH_CAP_XATTR_EXCL,
                dirty: engine::consts::CEPH_CAP_XATTR_EXCL,
                flush_tid: 99,
                xattrs: Some((2, [(b"user.k".to_vec(), vec![42; 4096])].into())),
                ..Default::default()
            },
        };
        r.enqueue(control, 4608).unwrap();
        let q = r.control.pop_front().unwrap();
        r.owned.insert(q.origin.serial, q.origin);
        let permit = f.server.engine.journal_mut().reserve().unwrap();
        let step = f.server.engine.step(Some(q.input), Some(permit)).unwrap();
        f.server.consume_step(&mut f.sessions, &mut r, step);
        f.server.runtime = Some(r);
        f.drain();
        assert_eq!(
            f.server.runtime.as_ref().unwrap().state,
            StepState::WaitingInput
        );
        let tail = f.server.engine.submitted_seq();
        let epoch = f.server.engine.retry_capacity_epoch();
        let mut read = proto::MClientRequest::new(200, MdsOp::GETATTR);
        read.path = WireFilePath::new(1, "");
        read.head.oldest_client_tid = floor;
        let mut r = f.server.runtime.take().unwrap();
        f.server.queue_message_at(
            &mut f.sessions,
            &mut r,
            42,
            ClientMessage::Request(read),
            Instant::now(),
            None,
        );
        let q = r.normal.pop_front().unwrap();
        r.owned.insert(q.origin.serial, q.origin);
        let permit = f.server.engine.journal_mut().reserve().unwrap();
        let step = f.server.engine.step(Some(q.input), Some(permit)).unwrap();
        f.server.consume_step(&mut f.sessions, &mut r, step);
        f.server.runtime = Some(r);
        assert_eq!(f.server.engine.retry_capacity_epoch(), epoch);
        let held_permit = f.server.engine.journal_mut().reserve().unwrap();
        f.server.pump(&mut f.sessions, 32);
        let r = f.server.runtime.as_ref().unwrap();
        if floor == 101 {
            assert!(f.server.fatal_error().is_some());
            assert!(
                matches!(&r.terminal_hold[0].input, EngineInput::Request { request, .. } if request.tid == 100 && request.op.code() == MdsOp::MKNOD.0)
            );
            assert_eq!(r.terminal_hold[0].bytes, original_bytes);
            assert_eq!(f.server.pending_requests[&(42, 100)].op(), MdsOp::MKNOD);
            assert!(!f.server.has_work());
        } else {
            assert!(f.server.fatal_error().is_none());
            assert!(r.terminal_hold.is_empty());
            assert_eq!(r.state, StepState::WaitingInput);
        }
        assert_eq!(f.server.engine.submitted_seq(), tail);
        assert_eq!(f.server.engine.retry_capacity_epoch(), epoch);
        f.server.engine.journal_mut().release(Some(held_permit));
    }
}

#[test]
fn d5_terminal_stop_retains_unknown_commands_protected_inputs_and_queued_write() {
    let mut f = Fixture::new();
    let peer = Peer::new(10);
    f.open(42, &peer);
    let ino = f.create(42, &peer);
    peer.clear();
    let flush = f.flush(42, ino);
    let mut r = f.server.runtime.take().unwrap();
    let update = cap_update_from_proto(&flush).unwrap();
    r.enqueue(
        EngineInput::CapUpdate {
            key: r.keys[&42],
            update,
        },
        512,
    )
    .unwrap();
    let q = r.control.pop_front().unwrap();
    r.owned.insert(q.origin.serial, q.origin);
    let permit = f.server.engine.journal_mut().reserve().unwrap();
    let step = f.server.engine.step(Some(q.input), Some(permit)).unwrap();
    f.server.consume_step(&mut f.sessions, &mut r, step);
    let seq = f.server.engine.submitted_seq();
    r.enqueue(
        EngineInput::CapRelease {
            key: r.keys[&42],
            items: Vec::new(),
        },
        64,
    )
    .unwrap();
    r.command(
        CommandKind::Closing(r.keys[&42]),
        Pending::Closing(r.keys[&42]),
    );
    let operation = r.next_operation;
    f.server.runtime = Some(r);
    f.server.fail_runtime(
        JournalError::Capacity,
        "test terminal uncertain-operation cut",
    );
    let r = f.server.runtime.as_ref().unwrap();
    assert_eq!(r.terminal_write.as_ref().unwrap().seq, seq);
    let record = store::record::decode_record(&r.terminal_write.as_ref().unwrap().body).unwrap();
    assert_eq!(record.client_flush.unwrap().tid, 99);
    assert!(r.pending.contains_key(&operation));
    assert!(matches!(
        r.submitted_commands[&operation].kind,
        CommandKind::Closing(_)
    ));
    assert_eq!(r.control.len(), 1);
    assert!(r.control_bytes >= 64);
    f.server.pump(&mut f.sessions, 32);
    assert!(f.commands.try_recv().is_err());
    assert!(peer.output.lock().unwrap().is_empty());
    assert!(f.server.ready_jobs().is_empty());
}

#[tokio::test]
async fn d5_actual_worker_default_budget_terminal_cut_and_larger_same_identity_restart() {
    use crate::{
        test_backend::MemoryBackend,
        test_runtime::{Peer as NativePeer, Worker, send},
    };
    let backend = MemoryBackend::default();
    let config = EngineConfig {
        max_xattr_size: 64 << 20,
        ..Default::default()
    };
    let (loaded, mut worker) = Worker::load(backend.clone(), 1, true, config.clone()).await;
    let identity = loaded.identity.clone();
    let mut server = worker.server(loaded);
    let mut sessions = SessionMap::default();
    worker.ready(&mut server, &mut sessions).await;
    let peer = NativePeer::new(42);
    send(
        &mut server,
        &mut sessions,
        42,
        &peer,
        MClientSession::new(SessionOp::REQUEST_OPEN, 0),
    );
    worker.settle(&mut server, &mut sessions).await;
    let mut inos = Vec::new();
    for (tid, name) in [(1, "bulk"), (2, "small")] {
        let mut request = proto::MClientRequest::new(tid, MdsOp::CREATE);
        request.path = WireFilePath::new(1, name);
        request.head.flags = proto::request::request_flags::WANT_DENTRY;
        request.head.args = proto::request::OpenArgs {
            flags: proto::open_flags::CREAT,
            mode: 0o644,
            pool: 2,
            ..Default::default()
        }
        .to_args();
        send(&mut server, &mut sessions, 42, &peer, request);
        worker.settle(&mut server, &mut sessions).await;
        let reply = peer
            .output
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|r| MClientReply::from_raw(r.clone()).unwrap())
            .find(|r| r.tid == tid)
            .unwrap();
        assert_eq!(reply.result, 0);
        inos.push(reply.trace.target.unwrap().ino);
    }
    let small_cap = server.engine.cap(inos[1], 42).unwrap().cap_id;
    let mut image = server.checkpoint_for_test().unwrap();
    // Explicit validated Ready3 fixture. The large *other* retained inode does
    // not enter the small dirty control or its independent transient-copy lane.
    for inode in &mut image.inodes {
        if inos.contains(&inode.core.ino) {
            inode
                .core
                .xattrs
                .insert(b"user.capacity".to_vec(), Vec::new());
            inode.core.xattr_version = 2;
        }
    }
    let fits = |image: &engine::EngineCheckpoint, bytes: usize| {
        let mut cfg = config.clone();
        cfg.retry_limits.max_bytes = bytes;
        Engine::restore(
            cfg,
            engine::MemJournal::at_seq(image.covered_seq, true),
            image.clone(),
        )
        .is_ok()
    };
    // Use only the checked public load boundary. No private charge query or
    // production accounting dependency is introduced in the daemon.
    let (mut low, mut high) = (1, 1 << 20);
    while low < high {
        let mid = (low + high) / 2;
        if fits(&image, mid) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(!fits(&image, low - 1));
    let target = (64 << 20) - 1024;
    let fill = (target - low) / 2;
    image
        .inodes
        .iter_mut()
        .find(|i| i.core.ino == inos[0])
        .unwrap()
        .core
        .xattrs
        .insert(b"user.capacity".to_vec(), vec![0x42; fill]);
    assert!(fits(&image, target));
    assert!(!fits(&image, target - 2));
    // Publish this checked fixture through the actual serialized worker, with
    // the real owner/tail and ordinary immutable checkpoint/pointer guards.
    let operation = 900000;
    worker
        .tx
        .send(Command {
            owner: server.owner(),
            operation,
            kind: CommandKind::Checkpoint(image.clone()),
        })
        .await
        .unwrap();
    let published = worker.event().await;
    assert_eq!(published.operation, operation);
    assert!(matches!(published.kind, EventKind::Checkpoint(_)));
    worker.stop().await;

    let flush = MClientCaps {
        op: CapOp::FLUSH,
        ino: inos[1],
        cap_id: small_cap,
        caps: engine::consts::CEPH_CAP_XATTR_EXCL,
        wanted: engine::consts::CEPH_CAP_XATTR_EXCL,
        dirty: engine::consts::CEPH_CAP_XATTR_EXCL,
        tid: 99,
        xattr_version: 3,
        xattrbl: proto::xattrs_to_blob(
            &[(
                Bytes::from_static(b"user.capacity"),
                Bytes::from(vec![0x61; 4096]),
            )]
            .into(),
        ),
        ..Default::default()
    };
    for (incarnation, extra, should_fail) in [(3, 0, true), (4, 16384, false)] {
        let mut cfg = config.clone();
        cfg.retry_limits.max_bytes += extra;
        let mut worker = Worker::launch(backend.clone(), incarnation, false, false, cfg);
        let EventKind::Loaded(loaded) = worker.event().await.kind else {
            panic!("same-identity restart failed");
        };
        assert_eq!(loaded.identity, identity);
        assert_eq!(loaded.state, image);
        let mut server = worker.server(*loaded);
        let mut sessions = SessionMap::default();
        worker.settle(&mut server, &mut sessions).await;
        server.confirm_reconnect(Instant::now(), Duration::from_secs(45));
        let peer = NativePeer::new(100 + incarnation as u32);
        send(&mut server, &mut sessions, 42, &peer, flush.clone());
        let mut watermark = proto::MClientRequest::new(1000, MdsOp::MKDIR);
        watermark.path = WireFilePath::new(1, "freeing-request-must-stay-held");
        watermark.head.oldest_client_tid = 1000;
        send(&mut server, &mut sessions, 42, &peer, watermark);
        let reconnect = proto::MClientReconnect {
            caps: [(
                inos[1],
                proto::CapReconnect {
                    cap_id: small_cap,
                    wanted: engine::consts::CEPH_CAP_XATTR_EXCL,
                    issued: engine::consts::CEPH_CAP_XATTR_EXCL,
                    snaprealm: 1,
                    ..Default::default()
                },
            )]
            .into(),
            ..Default::default()
        };
        send(&mut server, &mut sessions, 42, &peer, reconnect);
        worker.settle(&mut server, &mut sessions).await;
        assert!(!server.try_finish_recovery(&mut sessions));
        assert!(server.recovery_structural_done());
        let before = server.engine.inode(inos[1]).unwrap().clone();
        let tail = server.engine.submitted_seq();
        let appends = backend.memory.lock().unwrap().appends;
        server.allow_recovery_finalization(true);
        server.allow_recovery_requests(true);
        for _ in 0..32 {
            server.pump(&mut sessions, 32);
            if server.fatal_error().is_some() || !server.has_work() {
                break;
            }
        }
        if should_fail {
            assert_eq!(server.engine.failure(), Some(JournalError::Capacity));
            assert_eq!(server.engine.inode(inos[1]).unwrap(), &before);
            assert_eq!(server.engine.submitted_seq(), tail);
            assert_eq!(backend.memory.lock().unwrap().appends, appends);
            assert!(peer.output.lock().unwrap().is_empty());
            let r = server.runtime.as_ref().unwrap();
            assert_eq!(r.terminal_hold.len(), 1);
            assert_eq!(r.terminal_hold[0].bytes, r.control_bytes);
            assert_eq!(r.recovery_drain, RecoveryDrain::Controls);
            assert_eq!(r.recovery_status_held_for_test(), 1);
            assert!(!server.has_work());
            assert!(server.ready_jobs().is_empty());
            server.pump(&mut sessions, 64);
            assert_eq!(backend.memory.lock().unwrap().appends, appends);
        } else {
            worker.settle(&mut server, &mut sessions).await;
            assert!(server.try_finish_recovery(&mut sessions));
            assert_eq!(
                server.engine.inode(inos[1]).unwrap().xattrs[b"user.capacity".as_slice()],
                vec![0x61; 4096]
            );
            assert_eq!(server.engine.submitted_seq(), tail + 1);
            assert_eq!(backend.memory.lock().unwrap().appends, appends + 1);
            let output = peer.output.lock().unwrap();
            let ack = output
                .iter()
                .position(|raw| {
                    raw.msg_type == proto::msg_type::CLIENT_CAPS
                        && MClientCaps::from_raw(raw.clone()).unwrap().op == CapOp::FLUSH_ACK
                })
                .unwrap();
            let open = output
                .iter()
                .position(|raw| {
                    raw.msg_type == proto::msg_type::CLIENT_SESSION
                        && MClientSession::from_raw(raw.clone()).unwrap().op == SessionOp::OPEN
                })
                .unwrap();
            assert!(ack < open);
        }
        worker.stop().await;
    }
}
