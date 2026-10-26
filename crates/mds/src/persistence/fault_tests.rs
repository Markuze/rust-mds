//! Interrupted Closing cuts use the actual store worker, not fabricated completions.
use super::*;
use crate::{
    session::{ClientLink, SessionMap},
    test_backend::MemoryBackend,
    test_runtime::{Peer, Worker, create, send},
};

#[tokio::test]
async fn g2_fault_closing_restart_cuts() {
    for cut in 0..3 {
        let backend = MemoryBackend::default();
        let config = EngineConfig {
            writeable_range_max_inc_objs: 2,
            ..Default::default()
        };
        let (loaded, mut worker) = Worker::load(backend.clone(), 1, true, config.clone()).await;
        let mut server = worker.server(loaded);
        let mut sessions = SessionMap::default();
        worker.ready(&mut server, &mut sessions).await;
        let peer = Peer::new(42);
        let file = create(&mut worker, &mut server, &mut sessions, &peer).await;
        let before = server.checkpoint_for_test().unwrap();
        let key = before.sessions[0].key;
        let bound = before
            .inodes
            .iter()
            .find(|i| i.core.ino == file.ino)
            .unwrap()
            .core
            .client_ranges[&42];
        assert!(!before.sessions[0].completed_requests.is_empty());
        assert!(bound > 0);
        let (evict, mut evictions) = mpsc::channel(4);
        server.set_evict_channel(evict);
        server.request_eviction(&mut sessions, 42);
        let target = evictions.try_recv().unwrap();
        let fence = crate::test_backend::Fence::new(backend.clone());
        let epoch = crate::fencing::fence(&fence, &peer.peer_addr(), 0)
            .await
            .unwrap();
        assert!(server.finish_eviction(&mut sessions, target, epoch));
        peer.clear();
        let mut event = worker.event().await;
        assert!(matches!(event.kind, EventKind::Closing(k) if k == key));
        for stage in 0..cut {
            Worker::apply(&mut server, &mut sessions, event);
            event = worker.event().await;
            assert!(
                matches!(event.kind, EventKind::Committed(_)),
                "cut={cut}, stage={stage}"
            );
        }
        worker.stop().await;
        drop(server);
        backend.memory.lock().unwrap().trace.clear();
        let (loaded, mut restarted) = Worker::load(backend.clone(), 2, false, config.clone()).await;
        assert_eq!(loaded.sessions.records[0].key, key);
        assert_eq!(
            loaded.sessions.records[0].lifecycle,
            SessionLifecycle::Closing
        );
        assert_eq!(
            loaded.sessions.control.next_incarnation,
            key.incarnation + 1
        );
        let appends = {
            let memory = backend.memory.lock().unwrap();
            assert!(memory.trace.iter().any(|line| line == "blocklist:42"));
            memory.appends
        };
        let mut server = restarted.server(loaded);
        let recovered = server.checkpoint_for_test().unwrap();
        if cut == 0 {
            assert_eq!(
                recovered.sessions[0].completed_requests,
                before.sessions[0].completed_requests
            );
        }
        if cut >= 1 {
            assert_eq!(recovered.jobs.len(), 1);
            assert!(
                matches!(&recovered.jobs[0].kind, engine::LifecycleKind::RecoverSize { max_size, .. } if *max_size == bound)
            );
        }
        let old_job = recovered.jobs.first().cloned();
        let mut sessions = SessionMap::default();
        assert_eq!(server.gather_len(), 0);
        restarted.settle(&mut server, &mut sessions).await;
        let image = server.checkpoint_for_test().unwrap();
        assert_eq!(image.cleanup_receipts.len(), 1);
        assert_eq!(image.cleanup_receipts[0].0, key);
        assert!(image.sessions.is_empty());
        if let Some(job) = old_job {
            assert_eq!(image.jobs, vec![job]);
        }
        assert_eq!(image.jobs.len(), 1);
        assert!(
            matches!(&image.jobs[0].kind, engine::LifecycleKind::RecoverSize { max_size, .. } if *max_size == bound)
        );
        assert!(peer.output.lock().unwrap().is_empty());
        assert_eq!(sessions.len(), 0);
        if cut == 2 {
            assert_eq!(
                backend.memory.lock().unwrap().appends,
                appends,
                "terminal receipt must not be resubmitted"
            );
        }
        let receipt = image.cleanup_receipts[0];
        server.request_checkpoint();
        restarted.settle(&mut server, &mut sessions).await;
        restarted.stop().await;
        let (loaded, third) = Worker::load(backend.clone(), 3, false, config).await;
        assert!(loaded.records.is_empty());
        assert_eq!(
            loaded.sessions.records[0].lifecycle,
            SessionLifecycle::Closed
        );
        assert_eq!(loaded.sessions.records[0].cleanup_seq, Some(receipt.1));
        let server = third.server(loaded);
        assert_eq!(
            server.checkpoint_for_test().unwrap().cleanup_receipts,
            vec![receipt]
        );
        assert_eq!(server.gather_len(), 0);
    }
}

#[tokio::test]
async fn g2_fault_recovery_join_rejects_mismatches() {
    for closed in [false, true] {
        let backend = MemoryBackend::default();
        let (loaded, mut worker) =
            Worker::load(backend.clone(), 1, true, EngineConfig::default()).await;
        let mut server = worker.server(loaded);
        let mut sessions = SessionMap::default();
        worker.ready(&mut server, &mut sessions).await;
        let peer = Peer::new(42);
        create(&mut worker, &mut server, &mut sessions, &peer).await;
        if closed {
            let seq = sessions.get(42).unwrap().push_seq();
            send(
                &mut server,
                &mut sessions,
                42,
                &peer,
                proto::MClientSession::new(proto::SessionOp::REQUEST_CLOSE, seq),
            );
            worker.settle(&mut server, &mut sessions).await;
        }
        worker.stop().await;
        for variant in 0..3 {
            let (mut loaded, next) =
                Worker::load(backend.clone(), variant + 2, false, EngineConfig::default()).await;
            if closed {
                match variant {
                    0 => {
                        loaded.sessions.records[0].lifecycle = SessionLifecycle::Open;
                        loaded.sessions.records[0].cleanup_seq = None;
                    }
                    1 => loaded.sessions.records[0].cleanup_seq = Some(loaded.tail + 1),
                    _ => loaded.sessions.records[0].key.incarnation += 100,
                }
            } else {
                match variant {
                    0 => loaded.sessions.records.clear(),
                    1 => {
                        loaded.sessions.records[0].lifecycle = SessionLifecycle::Closed;
                        loaded.sessions.records[0].cleanup_seq = Some(loaded.tail);
                    }
                    _ => {
                        let mut duplicate = loaded.sessions.records[0].clone();
                        duplicate.key.incarnation += 100;
                        loaded.sessions.records.push(duplicate);
                    }
                }
            }
            let result = crate::server::Server::from_loaded(
                "a".into(),
                2,
                1,
                loaded,
                next.config.clone(),
                &Limits::default(),
                next.tx.clone(),
                1,
                10000,
                Duration::from_secs(3600),
            );
            assert!(result.is_err(), "closed={closed}, variant={variant}");
            assert!(
                peer.output
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|raw| raw.msg_type != proto::msg_type::CLIENT_RECLAIM_REPLY)
            );
        }
    }
}

#[tokio::test]
async fn g3_d6_checked_restore_rejects_duplicate_live_uuid() {
    let backend = MemoryBackend::default();
    let (loaded, mut worker) =
        Worker::load(backend.clone(), 1, true, EngineConfig::default()).await;
    let mut server = worker.server(loaded);
    let mut sessions = SessionMap::default();
    worker.ready(&mut server, &mut sessions).await;
    let peer = Peer::new(42);
    create(&mut worker, &mut server, &mut sessions, &peer).await;
    worker.stop().await;
    let (mut loaded, next) = Worker::load(backend, 2, false, EngineConfig::default()).await;
    loaded.sessions.records[0].identity.metadata.insert(
        Bytes::from_static(b"uuid"),
        Bytes::from_static(b"duplicate"),
    );
    let mut other = loaded.sessions.records[0].clone();
    other.key.client = 43;
    other.key.incarnation += 1;
    other.identity.client = 43;
    loaded.sessions.records.push(other);
    assert!(
        crate::server::Server::from_loaded(
            "a".into(),
            2,
            1,
            loaded,
            next.config.clone(),
            &Limits::default(),
            next.tx.clone(),
            1,
            10000,
            Duration::from_secs(3600)
        )
        .is_err(),
        "checked restore must reject ambiguous live UUID ownership"
    );
}
