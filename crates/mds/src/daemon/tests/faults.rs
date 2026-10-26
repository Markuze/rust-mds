use super::*;
use crate::{
    test_backend::{MemoryBackend, data::DataIo},
    test_runtime::{Peer, Worker, create, send},
};
use engine::{EngineConfig, LifecycleIntent, LifecycleKind};
use proto::{MClientRequest, MClientSession, MdsOp, SessionOp};

fn install(d: &mut Daemon, server: Server, worker: &Worker) {
    let owner = server.owner();
    let membership = Membership {
        rank: 0,
        incarnation: owner.incarnation as i32,
        gid: owner.gid,
    };
    d.test_global_id = Some(owner.gid);
    d.membership = Some(membership);
    let map = d.mdsmap.as_mut().unwrap();
    let mut info = map.mds_info.values().next().unwrap().clone();
    info.global_id = owner.gid;
    info.inc = owner.incarnation as i32;
    map.up.insert(0, owner.gid);
    map.mds_info.clear();
    map.mds_info.insert(owner.gid, info);
    d.server = Some(server);
    d.phase = Phase::Loading;
    d.state = MdsState::ACTIVE;
    d.authority = worker.authority.clone();
    d.authority_rx = d.authority.subscribe();
    d.storage_tx = Some(worker.tx.clone());
    d.jobs.clear();
    d.refresh_authority();
}
async fn settle_real(d: &mut Daemon, worker: &mut Worker) {
    for _ in 0..4096 {
        d.refresh_authority();
        let server = d.server.as_mut().unwrap();
        server.pump(&mut d.sessions, 32);
        if server.pending_storage() > 0 {
            d.on_storage_event(worker.event().await).unwrap();
        } else if !server.has_work() {
            d.drive_phase().unwrap();
            return;
        }
    }
    panic!("actual daemon worker did not settle");
}
async fn next_job(rx: &mut mpsc::Receiver<ControlEvent>) -> ControlEvent {
    tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .unwrap()
        .unwrap()
}

async fn job_fixture(kind: &str, pool: i64) -> (MemoryBackend, Worker, Server, LifecycleIntent) {
    let backend = MemoryBackend::default();
    let config = EngineConfig {
        data_pools: vec![pool],
        writeable_range_max_inc_objs: 2,
        ..Default::default()
    };
    let (loaded, mut worker) = Worker::load(backend.clone(), 1, true, config).await;
    let mut server = worker.server(loaded);
    let mut sessions = SessionMap::default();
    worker.ready(&mut server, &mut sessions).await;
    let peer = Peer::new(42);
    send(
        &mut server,
        &mut sessions,
        42,
        &peer,
        MClientSession::new(SessionOp::REQUEST_OPEN, 0),
    );
    worker.settle(&mut server, &mut sessions).await;
    let mut namespace = MClientRequest::new(100, MdsOp::SETXATTR);
    namespace.path = proto::FilePath::new(1, "");
    namespace.path2 = proto::FilePath::new(0, "ceph.dir.layout.pool_namespace");
    namespace.data = denc::Bytes::from_static(b"fault-data");
    send(&mut server, &mut sessions, 42, &peer, namespace);
    worker.settle(&mut server, &mut sessions).await;
    assert!(
        peer.output
            .lock()
            .unwrap()
            .iter()
            .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
            .map(|raw| proto::MClientReply::from_raw(raw.clone()).unwrap())
            .any(|reply| reply.tid == 100 && reply.result == 0)
    );
    let file = create(&mut worker, &mut server, &mut sessions, &peer).await;
    assert_eq!(file.layout.pool_ns, "fault-data");
    let os = u64::from(file.layout.object_size);
    if kind != "probe" {
        send(
            &mut server,
            &mut sessions,
            42,
            &peer,
            proto::MClientCaps {
                op: proto::CapOp::FLUSH,
                ino: file.ino,
                cap_id: file.cap.cap_id,
                seq: file.cap.seq,
                issue_seq: file.cap.seq,
                caps: file.cap.caps,
                wanted: file.cap.wanted,
                dirty: proto::types::cap::FILE_WR,
                tid: 55,
                size: os + os / 2,
                ..Default::default()
            },
        );
        worker.settle(&mut server, &mut sessions).await;
        assert_eq!(
            server
                .checkpoint_for_test()
                .unwrap()
                .inodes
                .iter()
                .find(|i| i.core.ino == file.ino)
                .unwrap()
                .core
                .size,
            os + os / 2
        );
    }
    if kind == "truncate" {
        let mut request = MClientRequest::new(2, MdsOp::SETATTR);
        request.path = proto::FilePath::new(file.ino, "");
        request.head.args = proto::request::SetattrArgs {
            size: os / 2,
            old_size: os + os / 2,
            mask: proto::types::setattr::SIZE,
            ..Default::default()
        }
        .to_args();
        send(&mut server, &mut sessions, 42, &peer, request);
        worker.settle(&mut server, &mut sessions).await;
        let mut seen = 0;
        for _ in 0..8 {
            let batch = {
                let output = peer.output.lock().unwrap();
                let batch = output[seen..].to_vec();
                seen = output.len();
                batch
            };
            let mut acked = false;
            for raw in batch
                .into_iter()
                .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_CAPS)
            {
                let cap = proto::MClientCaps::from_raw(raw).unwrap();
                if cap.op != proto::CapOp::REVOKE {
                    continue;
                }
                send(
                    &mut server,
                    &mut sessions,
                    42,
                    &peer,
                    proto::MClientCaps {
                        op: proto::CapOp::UPDATE,
                        ino: cap.ino,
                        cap_id: cap.cap_id,
                        seq: cap.seq,
                        issue_seq: cap.issue_seq,
                        caps: cap.caps,
                        wanted: cap.wanted,
                        migrate_seq: cap.migrate_seq,
                        ..Default::default()
                    },
                );
                acked = true;
            }
            worker.settle(&mut server, &mut sessions).await;
            if !acked {
                break;
            }
        }
        assert!(
            peer.output
                .lock()
                .unwrap()
                .iter()
                .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_REPLY)
                .map(|raw| proto::MClientReply::from_raw(raw.clone()).unwrap())
                .any(|reply| reply.tid == 2 && reply.result == 0),
            "truncate setup must complete"
        );
    } else if kind == "purge" {
        let mut request = MClientRequest::new(2, MdsOp::UNLINK);
        request.path = proto::FilePath::new(1, "file");
        send(&mut server, &mut sessions, 42, &peer, request);
        worker.settle(&mut server, &mut sessions).await;
        let latest = peer
            .output
            .lock()
            .unwrap()
            .iter()
            .filter(|raw| raw.msg_type == proto::msg_type::CLIENT_CAPS)
            .map(|raw| proto::MClientCaps::from_raw(raw.clone()).unwrap())
            .filter(|cap| cap.ino == file.ino)
            .last();
        let seq = latest.map_or(file.cap.seq, |cap| cap.seq);
        send(
            &mut server,
            &mut sessions,
            42,
            &peer,
            proto::MClientCapRelease {
                caps: vec![proto::CapItem {
                    ino: file.ino,
                    cap_id: file.cap.cap_id,
                    migrate_seq: file.cap.mseq,
                    issue_seq: seq,
                }],
                ..Default::default()
            },
        );
        worker.settle(&mut server, &mut sessions).await;
    }
    if kind == "probe" {
        send(
            &mut server,
            &mut sessions,
            42,
            &peer,
            proto::MClientCaps {
                op: proto::CapOp::UPDATE,
                ino: file.ino,
                cap_id: file.cap.cap_id,
                seq: file.cap.seq,
                issue_seq: file.cap.seq,
                caps: file.cap.caps,
                wanted: file.cap.wanted,
                max_size: os * 2,
                ..Default::default()
            },
        );
        worker.settle(&mut server, &mut sessions).await;
        let (tx, mut rx) = mpsc::channel(4);
        server.set_evict_channel(tx);
        server.request_eviction(&mut sessions, 42);
        let fence = crate::test_backend::Fence::new(backend.clone());
        let epoch = fencing::fence(&fence, &peer.addr, 0).await.unwrap();
        assert!(server.finish_eviction(&mut sessions, rx.try_recv().unwrap(), epoch));
    } else {
        let seq = sessions.get(42).unwrap().push_seq();
        send(
            &mut server,
            &mut sessions,
            42,
            &peer,
            MClientSession::new(SessionOp::REQUEST_CLOSE, seq),
        );
    }
    worker.settle(&mut server, &mut sessions).await;
    let jobs = server.checkpoint_for_test().unwrap().jobs;
    let job = jobs
        .into_iter()
        .find(|job| {
            matches!(
                (&job.kind, kind),
                (LifecycleKind::Purge { .. }, "purge")
                    | (LifecycleKind::Truncate { .. }, "truncate")
                    | (LifecycleKind::RecoverSize { .. }, "probe")
            )
        })
        .unwrap_or_else(|| panic!("missing production {kind} job"));
    (backend, worker, server, job)
}

#[tokio::test]
async fn g2_fault_lifecycle_partial_io_retry() {
    for kind in ["purge", "truncate", "probe"] {
        let data = DataIo::default();
        let (objecter, pool) = data.objecter().await;
        let (backend, mut worker, server, job) = job_fixture(kind, pool).await;
        let initial = server.checkpoint_for_test().unwrap();
        let layout = match &job.kind {
            LifecycleKind::Purge { layouts, .. } | LifecycleKind::RecoverSize { layouts, .. } => {
                &layouts[0]
            }
            LifecycleKind::Truncate { layout, .. } => layout,
        };
        let os = layout.object_size as usize;
        data.put(
            pool,
            &layout.pool_ns,
            job.ino,
            0,
            vec![0x61; os],
            denc::UTime { sec: 30, nsec: 0 },
        );
        data.put(
            pool,
            &layout.pool_ns,
            job.ino,
            1,
            vec![0x62; 123],
            denc::UTime { sec: 42, nsec: 0 },
        );
        data.put(
            pool,
            "unrelated",
            job.ino,
            0,
            vec![0x73; 9],
            denc::UTime { sec: 99, nsec: 0 },
        );
        data.fail_after(1);
        let (mut d, _, mut controls) = offline_daemon().await;
        d.objecter.shutdown();
        d.objecter = objecter;
        install(&mut d, server, &worker);
        d.drive_phase().unwrap();
        d.refresh_authority();
        d.drive_jobs();
        let failure = next_job(&mut controls).await;
        assert!(
            matches!(&failure, ControlEvent::Job { id, result: Err(_), .. } if *id == job.id),
            "kind={kind}, job={job:?}, trace={:?}",
            data.0.lock().unwrap().trace
        );
        d.on_control_event(failure);
        let image = d.server.as_ref().unwrap().checkpoint_for_test().unwrap();
        assert_eq!(
            image, initial,
            "failed data I/O cannot mutate metadata, bounds or layouts"
        );
        assert_eq!(image.jobs, vec![job.clone()]);
        assert_eq!(d.jobs[&job.id].failures, 1);
        assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
        let trace = data.0.lock().unwrap().trace.clone();
        assert_eq!(trace.len(), 2);
        assert!(
            trace
                .iter()
                .all(|(key, _, _, _)| key.0 == pool && key.1.as_ref() == layout.pool_ns.as_bytes())
        );
        if kind == "probe" {
            assert_eq!(
                image
                    .inodes
                    .iter()
                    .find(|i| i.core.ino == job.ino)
                    .unwrap()
                    .core
                    .size,
                0
            );
        }
        let partial = {
            let state = data.0.lock().unwrap();
            let first = (
                pool,
                denc::Bytes::copy_from_slice(layout.pool_ns.as_bytes()),
                crate::rados::object_name(job.ino, 0),
            );
            let second = (
                pool,
                denc::Bytes::copy_from_slice(layout.pool_ns.as_bytes()),
                crate::rados::object_name(job.ino, 1),
            );
            if kind == "purge" {
                assert!(!state.objects.contains_key(&first));
            }
            if kind == "truncate" {
                assert_eq!(state.objects[&first].bytes, vec![0x61; os / 2]);
            }
            assert_eq!(state.objects[&second].bytes, vec![0x62; 123]);
            state
                .objects
                .iter()
                .map(|(key, object)| {
                    (
                        key.clone(),
                        object.bytes.clone(),
                        object.mtime,
                        object.truncate_seq,
                    )
                })
                .collect::<Vec<_>>()
        };
        worker.stop().await;
        stop_offline(&d);
        drop(d);
        let (loaded, mut restarted) =
            Worker::load(backend.clone(), 2, false, worker.config.clone()).await;
        let server = restarted.server(loaded);
        let restored = server.checkpoint_for_test().unwrap();
        let mut expected = initial.clone();
        // WAL-only load has no volatile cap grants. Reconnect, not data-job
        // replay, restores cap IDs. Compare every durable field unchanged.
        expected.last_cap_id = restored.last_cap_id;
        assert_eq!(restored, expected);
        assert_eq!(
            data.0
                .lock()
                .unwrap()
                .objects
                .iter()
                .map(|(key, object)| (
                    key.clone(),
                    object.bytes.clone(),
                    object.mtime,
                    object.truncate_seq
                ))
                .collect::<Vec<_>>(),
            partial
        );
        assert_eq!(
            server.checkpoint_for_test().unwrap().jobs,
            vec![job.clone()]
        );
        let (mut d, _, mut controls) = offline_daemon().await;
        let (objecter, _) = data.objecter().await;
        d.objecter.shutdown();
        d.objecter = objecter;
        install(&mut d, server, &restarted);
        let before = data.0.lock().unwrap().trace.len();
        d.drive_jobs();
        assert_eq!(
            data.0.lock().unwrap().trace.len(),
            before,
            "data cannot run before checked initial work and E2 structural finish"
        );
        settle_real(&mut d, &mut restarted).await;
        d.refresh_authority();
        d.drive_jobs();
        let success = next_job(&mut controls).await;
        assert!(matches!(&success, ControlEvent::Job { id, result: Ok(_), .. } if *id == job.id));
        d.on_control_event(success);
        assert!(d.server.as_ref().unwrap().has_job(job.id));
        settle_real(&mut d, &mut restarted).await;
        let image = d.server.as_ref().unwrap().checkpoint_for_test().unwrap();
        assert!(image.jobs.is_empty());
        let state = data.0.lock().unwrap();
        match &job.kind {
            LifecycleKind::Purge { .. } => assert_eq!(state.objects.len(), 1),
            LifecycleKind::Truncate {
                size, truncate_seq, ..
            } => {
                let objects: Vec<_> = state
                    .objects
                    .iter()
                    .filter(|(key, _)| key.1.as_ref() == layout.pool_ns.as_bytes())
                    .map(|(_, object)| object)
                    .collect();
                assert_eq!(objects[0].bytes, vec![0x61; *size as usize]);
                assert!(objects[1].bytes.is_empty());
                assert!(
                    state
                        .trace
                        .iter()
                        .filter(|(_, op, _, _)| *op == osdc::ops::op::TRIMTRUNC)
                        .all(|(_, _, seq, _)| seq == truncate_seq)
                );
            }
            LifecycleKind::RecoverSize { .. } => {
                let inode = &image
                    .inodes
                    .iter()
                    .find(|i| i.core.ino == job.ino)
                    .unwrap()
                    .core;
                assert_eq!(inode.size, os as u64 + 123);
                assert_eq!(inode.mtime, engine::Timestamp::new(42, 0));
                assert!(inode.client_ranges.is_empty());
            }
        }
        assert!(
            state.objects.iter().any(
                |(key, object)| key.1.as_ref() == b"unrelated" && object.bytes == vec![0x73; 9]
            )
        );
        drop(state);
        stop_offline(&d);
    }
}

#[tokio::test]
async fn g2_fault_lifecycle_completion_cuts_and_late_events() {
    for cut in ["before", "failed", "durable", "delivered"] {
        let data = DataIo::default();
        let (objecter, pool) = data.objecter().await;
        let (backend, mut worker, server, job) = job_fixture("probe", pool).await;
        let LifecycleKind::RecoverSize { layouts, .. } = &job.kind else {
            unreachable!();
        };
        data.put(
            pool,
            &layouts[0].pool_ns,
            job.ino,
            0,
            vec![0x7a; 1234],
            denc::UTime { sec: 42, nsec: 0 },
        );
        let (mut d, _, mut controls) = offline_daemon().await;
        d.objecter.shutdown();
        d.objecter = objecter;
        install(&mut d, server, &worker);
        d.drive_phase().unwrap();
        d.refresh_authority();
        d.drive_jobs();
        let success = next_job(&mut controls).await;
        let ControlEvent::Job {
            owner,
            operation,
            id,
            result: Ok(result),
        } = success
        else {
            panic!("probe must succeed");
        };
        let before = d.server.as_ref().unwrap().checkpoint_for_test().unwrap();
        let mut stale = owner;
        stale.gid += 100;
        for (owner, operation) in [(stale, operation), (owner, operation + 1)] {
            d.on_control_event(ControlEvent::Job {
                owner,
                operation,
                id,
                result: Ok(result.clone()),
            });
            assert_eq!(
                d.server.as_ref().unwrap().checkpoint_for_test().unwrap(),
                before
            );
            assert!(d.jobs[&id].active);
        }
        d.on_control_event(ControlEvent::Job {
            owner,
            operation,
            id,
            result: Ok(result.clone()),
        });
        d.on_control_event(ControlEvent::Job {
            owner,
            operation,
            id,
            result: Ok(result),
        });
        assert!(!d.jobs[&id].active && d.jobs[&id].submitted);
        if cut != "before" {
            if cut == "failed" {
                backend.memory.lock().unwrap().fail_write = Some("rmds.wal.0.0".into());
            }
            d.server.as_mut().unwrap().pump(&mut d.sessions, 32);
            let event = worker.event().await;
            if cut == "failed" {
                assert!(matches!(
                    event.kind,
                    persistence::EventKind::Failed(store::StoreError::Io)
                ));
                d.on_storage_event(event).unwrap();
                assert!(d.check_failure().is_err());
                assert!(d.server.as_ref().unwrap().fatal_error().is_some());
            } else {
                assert!(matches!(event.kind, persistence::EventKind::Committed(_)));
                if cut == "delivered" {
                    d.on_storage_event(event).unwrap();
                    let after = d.server.as_ref().unwrap().checkpoint_for_test().unwrap();
                    d.on_control_event(ControlEvent::Job {
                        owner,
                        operation,
                        id,
                        result: Ok(engine::LifecycleResult::Recovered {
                            size: 9999,
                            mtime: engine::Timestamp::ZERO,
                        }),
                    });
                    d.server.as_mut().unwrap().pump(&mut d.sessions, 32);
                    assert_eq!(
                        d.server.as_ref().unwrap().checkpoint_for_test().unwrap(),
                        after
                    );
                    assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
                }
            }
        }
        worker.stop().await;
        stop_offline(&d);
        drop(d);
        let (loaded, mut restarted) = Worker::load(backend, 2, false, worker.config.clone()).await;
        let mut server = restarted.server(loaded);
        let image = server.checkpoint_for_test().unwrap();
        let inode = &image
            .inodes
            .iter()
            .find(|i| i.core.ino == job.ino)
            .unwrap()
            .core;
        if cut == "durable" || cut == "delivered" {
            assert!(image.jobs.is_empty());
            assert_eq!(inode.size, 1234);
            assert!(inode.client_ranges.is_empty());
            assert_eq!(
                image.covered_seq,
                before.covered_seq + 1,
                "one completion despite duplicate result"
            );
            let mut sessions = SessionMap::default();
            restarted.ready(&mut server, &mut sessions).await;
            assert!(
                server.ready_jobs().is_empty(),
                "retired jobs cannot reschedule after E2 readiness"
            );
        } else {
            assert_eq!(image.jobs, vec![job]);
            assert_eq!(inode.size, 0);
            assert!(!inode.client_ranges.is_empty());
            assert_eq!(image.covered_seq, before.covered_seq);
        }
    }
}

#[tokio::test]
async fn g3_d6_real_worker_reclaim_failed_stale_partial_fences_then_closed_reply() {
    let backend = MemoryBackend::default();
    let (loaded, mut worker) =
        Worker::load(backend.clone(), 1, true, EngineConfig::default()).await;
    let mut server = worker.server(loaded);
    let mut sessions = SessionMap::default();
    worker.ready(&mut server, &mut sessions).await;
    let old = Peer::new(42);
    let file = create(&mut worker, &mut server, &mut sessions, &old).await;
    let changed = Peer::new(43);
    let requester = Peer::new(44);
    let mut open = MClientSession::new(SessionOp::REQUEST_OPEN, 0);
    open.metadata.insert("uuid".into(), "old".into());
    open.metadata.insert("timeout".into(), "0".into());
    send(&mut server, &mut sessions, 42, &changed, open);
    send(
        &mut server,
        &mut sessions,
        43,
        &requester,
        MClientSession::new(SessionOp::REQUEST_OPEN, 0),
    );
    worker.settle(&mut server, &mut sessions).await;
    let (mut d, _, _) = offline_daemon().await;
    install(&mut d, server, &worker);
    d.sessions = sessions;
    d.drive_phase().unwrap();
    d.refresh_authority();
    requester.clear();
    d.sessions.pause_output(true);
    d.sessions
        .get_mut(43)
        .unwrap()
        .send(&MClientSession::renewcaps(99));
    send(
        d.server.as_mut().unwrap(),
        &mut d.sessions,
        43,
        &requester,
        proto::MClientReclaim {
            flags: 1,
            uuid: "old".into(),
        },
    );
    let targets = d.server.as_mut().unwrap().recovery_fences(&mut d.sessions);
    assert_eq!(targets.len(), 1);
    let target = targets[0].clone();
    assert_eq!(target.addresses.0, vec![old.addr, changed.addr]);
    let before = d.server.as_ref().unwrap().checkpoint_for_test().unwrap();
    d.recovery_fences.insert(
        target.key,
        PendingRecoveryFence {
            target: target.clone(),
            operation: 10,
            active: true,
            retry_at: Instant::now() + Duration::from_secs(3600),
            proof: None,
        },
    );
    d.on_control_event(ControlEvent::RecoveryFence {
        membership: d.membership.unwrap(),
        operation: 10,
        target: target.clone(),
        result: Err("injected latest-map/fence failure".into()),
    });
    assert_eq!(
        d.server.as_ref().unwrap().checkpoint_for_test().unwrap(),
        before
    );
    assert!(requester.output.lock().unwrap().is_empty());
    d.recovery_fences.get_mut(&target.key).unwrap().active = true;
    d.on_control_event(ControlEvent::RecoveryFence {
        membership: d.membership.unwrap(),
        operation: 9,
        target: target.clone(),
        result: Ok(9),
    });
    assert!(d.recovery_fences[&target.key].active);
    let mut map = osdc::OsdMap::empty();
    map.epoch = 9;
    map.blocklist.insert(old.addr, denc::UTime::default());
    d.objecter.install_map(map.clone());
    d.objecter
        .wait_for_map_timeout(9, Duration::from_secs(1))
        .await
        .unwrap();
    d.on_control_event(ControlEvent::RecoveryFence {
        membership: d.membership.unwrap(),
        operation: 10,
        target: target.clone(),
        result: Ok(9),
    });
    d.recovery_fences.get_mut(&target.key).unwrap().retry_at =
        Instant::now() + Duration::from_secs(3600);
    d.drive_recovery_fences();
    assert_eq!(
        d.server.as_ref().unwrap().checkpoint_for_test().unwrap(),
        before
    );
    assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
    map.epoch = 10;
    map.blocklist.insert(changed.addr, denc::UTime::default());
    d.objecter.install_map(map);
    d.objecter
        .wait_for_map_timeout(10, Duration::from_secs(1))
        .await
        .unwrap();
    d.recovery_fences.get_mut(&target.key).unwrap().active = true;
    d.on_control_event(ControlEvent::RecoveryFence {
        membership: d.membership.unwrap(),
        operation: 10,
        target: target.clone(),
        result: Ok(10),
    });
    d.drive_recovery_fences();
    let committed_receipt = loop {
        let event = worker.event().await;
        if let persistence::EventKind::Closed { key, cleanup_seq } = event.kind {
            assert_eq!(key, target.key);
            assert!(requester.output.lock().unwrap().is_empty());
            assert_eq!(
                d.server
                    .as_ref()
                    .unwrap()
                    .checkpoint_for_test()
                    .unwrap()
                    .cleanup_receipts,
                vec![(key, cleanup_seq)]
            );
            d.on_storage_event(event).unwrap();
            break Some(cleanup_seq);
        }
        d.on_storage_event(event).unwrap();
        assert!(requester.output.lock().unwrap().is_empty());
    };
    d.sessions.pause_output(false);
    d.sessions.drain_output(32);
    {
        let output = requester.output.lock().unwrap();
        assert_eq!(MClientSession::from_raw(output[0].clone()).unwrap().seq, 99);
        let reply = proto::MClientReclaimReply::from_raw(output[1].clone()).unwrap();
        assert_eq!(
            (reply.result, reply.epoch, reply.addrs),
            (0, 10, target.addresses.clone())
        );
    }
    assert!(d.sessions.get(42).is_none());
    assert!(
        d.server
            .as_ref()
            .unwrap()
            .checkpoint_for_test()
            .unwrap()
            .jobs
            .iter()
            .any(|job| job.ino == file.ino)
    );
    worker.stop().await;
    let (loaded, next) = Worker::load(backend, 2, false, worker.config.clone()).await;
    let old_row = loaded
        .sessions
        .records
        .iter()
        .find(|row| row.key == target.key)
        .unwrap();
    assert_eq!(old_row.lifecycle, store::SessionLifecycle::Closed);
    assert_eq!(old_row.cleanup_seq, committed_receipt);
    let server = next.server(loaded);
    assert!(!server.recovered());
    stop_offline(&d);
}

#[tokio::test]
async fn g3_d6_healthy_active_timer_accounts_received_activity_and_stays_closed_on_loss() {
    let backend = MemoryBackend::default();
    let (loaded, mut worker) = Worker::load(backend, 1, true, EngineConfig::default()).await;
    let mut server = worker.server(loaded);
    let mut sessions = SessionMap::default();
    worker.ready(&mut server, &mut sessions).await;
    let peer = Peer::new(42);
    create(&mut worker, &mut server, &mut sessions, &peer).await;
    let (mut d, mut evictions, _) = offline_daemon().await;
    install(&mut d, server, &worker);
    d.sessions = sessions;
    d.server
        .as_mut()
        .unwrap()
        .set_evict_channel(d.evict_tx.clone());
    d.drive_phase().unwrap();
    d.refresh_authority();
    d.sessions.get_mut(42).unwrap().last_renew = Instant::now() - Duration::from_secs(301);
    d.beacon_lag_override = Some(true);
    d.on_tick();
    assert!(d.sessions.get(42).unwrap().is_open());
    assert!(evictions.try_recv().is_err());
    d.beacon_lag_override = Some(false);
    d.state = MdsState::STOPPING;
    d.on_tick();
    assert!(d.sessions.get(42).unwrap().is_open());
    d.state = MdsState::ACTIVE;
    let received = Instant::now();
    crate::daemon::dispatch_client_raw_at(
        d.server.as_mut().unwrap(),
        &mut d.sessions,
        42,
        Box::new(peer.clone()),
        vec![],
        MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 71).to_raw(denc::features::ALL),
        received,
    );
    d.on_tick();
    assert_eq!(
        d.sessions.get(42).unwrap().state,
        crate::session::SessionState::Open
    );
    assert!(evictions.try_recv().is_err());
    d.sessions.get_mut(42).unwrap().last_renew = Instant::now() - Duration::from_secs(301);
    d.on_tick();
    let target = evictions.try_recv().unwrap();
    d.queue_eviction(target).unwrap();
    let operation = d.fences[&42].operation;
    d.fences.get_mut(&42).unwrap().active = true;
    d.on_control_event(ControlEvent::Fence {
        membership: d.membership.unwrap(),
        operation,
        request: target,
        result: Err("injected fence failure".into()),
    });
    assert_eq!(
        d.sessions.get(42).unwrap().state,
        crate::session::SessionState::Fencing
    );
    assert_eq!(d.server.as_ref().unwrap().pending_storage(), 0);
    assert!(
        d.server
            .as_ref()
            .unwrap()
            .checkpoint_for_test()
            .unwrap()
            .cleanup_receipts
            .is_empty()
    );
    stop_offline(&d);
}

#[tokio::test]
async fn g3_d6_held_authenticated_renewal_precedes_active_expiry() {
    let (mut d, mut evictions, _, mut events) = offline_daemon_loading(vec![]).await;
    settle(&mut d).await;
    let issuer = auth::service::MonTicketIssuer::new(
        d.msgr.auth().name().clone(),
        d.msgr.auth().key().clone(),
        1,
    );
    d.msgr.auth().set_rotating(issuer.mds_keystore().rotating);
    let (client, link, _received, _) = loopback_client(&mut d, &mut events, &issuer, 42).await;
    assert!(link.send(MClientSession::new(SessionOp::REQUEST_OPEN, 0).to_raw(denc::features::ALL)));
    d.on_msgr_event(next_loopback_event(&mut events).await);
    settle(&mut d).await;
    let (unknown, unknown_link, _unknown_received, _) =
        loopback_client(&mut d, &mut events, &issuer, 99).await;
    d.sessions.get_mut(42).unwrap().last_renew = Instant::now() - Duration::from_secs(301);
    d.beacon_lag_override = Some(true);
    for tid in 1..=65 {
        assert!(
            unknown_link.send(MClientRequest::new(tid, MdsOp::LOOKUP).to_raw(denc::features::ALL))
        );
        d.on_msgr_event(next_loopback_event(&mut events).await);
    }
    assert!(
        link.send(
            MClientSession::new(SessionOp::REQUEST_RENEWCAPS, 99).to_raw(denc::features::ALL)
        )
    );
    d.on_msgr_event(next_loopback_event(&mut events).await);
    assert_eq!(d.held.len(), 66);
    let received = d.held.back().unwrap().2;
    d.beacon_lag_override = Some(false);
    d.on_tick();
    assert!(
        !d.held.is_empty(),
        "the activity test must exceed one bounded dispatch turn"
    );
    assert_eq!(d.sessions.get(42).unwrap().last_renew, received);
    assert_eq!(
        d.sessions.get(42).unwrap().state,
        crate::session::SessionState::Open
    );
    assert!(evictions.try_recv().is_err());
    client.shutdown();
    unknown.shutdown();
    stop_offline(&d);
}
