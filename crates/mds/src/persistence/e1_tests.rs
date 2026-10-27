//! Actual asynchronous production workers over atomic local compound operations.
use super::*;
use crate::{test_backend::MemoryBackend, test_runtime::Worker};

async fn failed(worker: &mut Worker, expected: StoreError) {
    let event = worker.event().await;
    assert_eq!(event.owner, None);
    assert_eq!(event.operation, 1);
    assert!(matches!(event.kind, EventKind::Failed(error) if error == expected));
    assert!(
        worker.rx.recv().await.is_none(),
        "failed startup must never emit Loaded"
    );
}

#[tokio::test]
async fn e1_worker_u1_seed_same_owner_optin_and_genuine_successor() {
    for optin in [false, true] {
        let backend = MemoryBackend::default();
        let config = EngineConfig {
            retry_limits: engine::RetryLimits {
                max_completions: 17,
                max_bytes: 1048576,
            },
            ..Default::default()
        };
        let mut seed = Worker::launch(backend.clone(), 1, true, optin, config.clone());
        failed(&mut seed, StoreError::UpgradeRequired).await;
        let identity = StoreIdentity {
            cluster_fsid: [7; 16],
            filesystem_id: 3,
            metadata_pool: 6,
        };
        let bootstrap = BootstrapStore::new(backend.clone(), identity.clone());
        let format = bootstrap.inspect_format().await.unwrap().unwrap();
        assert_eq!(format.phase, FormatPhase::Ready);
        assert_eq!(format.required_semantics, RecoverySemantics::Legacy2);
        assert!(format.retry_upgrade.is_none());
        let originals: Vec<_> = backend
            .memory
            .lock()
            .unwrap()
            .objects
            .iter()
            .filter(|(name, _)| name.contains("checkpoint"))
            .map(|(name, object)| (name.clone(), object.data.clone()))
            .collect();
        let writes = backend.memory.lock().unwrap().writes;
        let mut same = Worker::launch(backend.clone(), 1, false, true, config.clone());
        failed(&mut same, StoreError::UpgradeRequired).await;
        assert_eq!(backend.memory.lock().unwrap().writes, writes);
        let mut no_option = Worker::launch(backend.clone(), 2, false, false, config.clone());
        failed(&mut no_option, StoreError::UpgradeRequired).await;
        assert_eq!(backend.memory.lock().unwrap().writes, writes);
        let mut successor = Worker::launch(backend.clone(), 2, false, true, config.clone());
        let event = successor.event().await;
        assert_eq!(event.membership.incarnation, 2);
        let EventKind::Loaded(loaded) = event.kind else {
            panic!("successor failed checked adoption");
        };
        assert_eq!(loaded.owner.incarnation, 2);
        assert_eq!(loaded.sessions.control.owner.incarnation, 2);
        assert_eq!(
            loaded.state.recovery_semantics,
            RecoverySemantics::RetainedTargets3
        );
        assert_eq!(
            loaded.sessions.control.required_semantics,
            RecoverySemantics::RetainedTargets3
        );
        assert!(loaded.state.sessions.is_empty());
        assert_eq!(loaded.initial_range, format.plan.initial_range);
        let mut server = successor.server(*loaded);
        let mut sessions = crate::session::SessionMap::default();
        successor.ready(&mut server, &mut sessions).await;
        assert_eq!(server.retry_limits_for_test(), config.retry_limits);
        for (name, bytes) in originals {
            assert_eq!(backend.memory.lock().unwrap().objects[&name].data, bytes);
        }
        let adopted = bootstrap.inspect_format().await.unwrap().unwrap();
        assert_eq!(adopted.plan, format.plan);
        assert_eq!(
            adopted.required_semantics,
            RecoverySemantics::RetainedTargets3
        );
        assert!(adopted.retry_upgrade.is_some());
    }
}

#[tokio::test]
async fn e1_worker_invalid_limits_precede_every_native_write() {
    for limits in [
        engine::RetryLimits {
            max_completions: 0,
            max_bytes: 1,
        },
        engine::RetryLimits {
            max_completions: 1,
            max_bytes: 0,
        },
        engine::RetryLimits {
            max_completions: usize::MAX,
            max_bytes: 1,
        },
        engine::RetryLimits {
            max_completions: 1,
            max_bytes: usize::MAX,
        },
    ] {
        let backend = MemoryBackend::default();
        let mut worker = Worker::launch(
            backend.clone(),
            1,
            true,
            true,
            EngineConfig {
                retry_limits: limits,
                ..Default::default()
            },
        );
        failed(&mut worker, StoreError::Capacity).await;
        let memory = backend.memory.lock().unwrap();
        assert_eq!(memory.writes, 0);
        assert!(memory.trace.is_empty());
    }
}

#[tokio::test]
async fn e1_worker_each_native_await_rechecks_authority_before_next_operation() {
    for (object, mutation) in [
        ("rmds.format", false),
        ("rmds.format", true),
        ("rmds.wal.0.0", true),
        ("rmds.sessions.0", true),
    ] {
        for after in [false, true] {
            let backend = MemoryBackend::default();
            let mut seed = Worker::launch(backend.clone(), 1, true, false, EngineConfig::default());
            failed(&mut seed, StoreError::UpgradeRequired).await;
            *backend.pause_operation.lock().unwrap() = Some((object.into(), mutation, after));
            let mut worker =
                Worker::launch(backend.clone(), 2, false, true, EngineConfig::default());
            tokio::time::timeout(Duration::from_secs(3), backend.operation_paused.notified())
                .await
                .unwrap();
            worker.authority.send_modify(|a| a.healthy = false);
            let trace_before = backend.memory.lock().unwrap().trace.len();
            backend.release_operation.notify_one();
            failed(&mut worker, StoreError::Fenced).await;
            if mutation {
                let held = worker.attempts.0.lock().unwrap();
                assert!(
                    held.uncertain
                        .iter()
                        .any(|(name, op)| name == object && op.has_write())
                );
            }
            let memory = backend.memory.lock().unwrap();
            assert_eq!(
                memory.trace.len(),
                trace_before + usize::from(!after),
                "no native operation may follow the paused operation"
            );
        }
    }
}

#[tokio::test]
async fn e1_worker_preparing_completion_stops_at_u1_even_with_option() {
    for optin in [false, true] {
        let backend = MemoryBackend::default();
        *backend.pause_operation.lock().unwrap() = Some(("rmds.format".into(), true, true));
        let mut original = Worker::launch(backend.clone(), 1, true, false, EngineConfig::default());
        tokio::time::timeout(Duration::from_secs(3), backend.operation_paused.notified())
            .await
            .unwrap();
        backend.memory.lock().unwrap().fail_write = Some("rmds.format".into());
        backend.release_operation.notify_one();
        failed(&mut original, StoreError::Io).await;
        let identity = StoreIdentity {
            cluster_fsid: [7; 16],
            filesystem_id: 3,
            metadata_pool: 6,
        };
        let bootstrap = BootstrapStore::new(backend.clone(), identity);
        let preparing = bootstrap.inspect_format().await.unwrap().unwrap();
        assert_eq!(preparing.phase, FormatPhase::Preparing);
        let mut completing =
            Worker::launch(backend.clone(), 1, true, optin, EngineConfig::default());
        failed(&mut completing, StoreError::UpgradeRequired).await;
        let ready = bootstrap.inspect_format().await.unwrap().unwrap();
        assert_eq!(ready.phase, FormatPhase::Ready);
        assert_eq!(ready.required_semantics, RecoverySemantics::Legacy2);
        assert_eq!(ready.plan, preparing.plan);
        assert!(ready.retry_upgrade.is_none());
        assert!(
            !backend
                .memory
                .lock()
                .unwrap()
                .objects
                .keys()
                .any(|name| name.starts_with("rmds.checkpoint.0.2."))
        );
    }
}

#[tokio::test]
async fn e1_worker_loaded_publication_rechecks_after_channel_capacity_wait() {
    let backend = MemoryBackend::default();
    let mut seed = Worker::launch(backend.clone(), 1, true, false, EngineConfig::default());
    failed(&mut seed, StoreError::UpgradeRequired).await;
    backend.memory.lock().unwrap().trace.clear();
    let (start, gate, authority) = super::tests::start(2, false);
    let membership = gate.expected;
    let (_tx, commands) = mpsc::channel(1);
    let (events, mut rx) = mpsc::channel(1);
    events
        .send(Event {
            membership,
            owner: None,
            operation: 0,
            kind: EventKind::Capacity("test occupied slot"),
        })
        .await
        .unwrap();
    let task = tokio::spawn(worker(
        backend.clone(),
        start,
        gate,
        crate::test_backend::Fence::new(backend.clone()),
        commands,
        events,
    ));
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            // The second barrier is the final synchronous open boundary. No
            // native I/O or await remains before the occupied event reservation.
            if backend
                .memory
                .lock()
                .unwrap()
                .trace
                .iter()
                .filter(|s| s.starts_with("barrier:"))
                .count()
                >= 2
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    authority.send_modify(|a| a.healthy = false);
    assert_eq!(rx.recv().await.unwrap().operation, 0);
    let event = rx.recv().await.unwrap();
    assert_eq!(
        (event.membership, event.owner, event.operation),
        (membership, None, 1)
    );
    assert!(matches!(event.kind, EventKind::Failed(StoreError::Fenced)));
    assert!(rx.recv().await.is_none());
    task.await.unwrap();
}

#[tokio::test]
async fn e1_worker_ready3_nonempty_restart_needs_no_optin_and_uses_current_pointer() {
    let backend = MemoryBackend::default();
    let (loaded, mut first) = Worker::load(backend.clone(), 1, true, EngineConfig::default()).await;
    let mut server = first.server(loaded);
    let mut sessions = crate::session::SessionMap::default();
    first.ready(&mut server, &mut sessions).await;
    let peer = crate::test_runtime::Peer::new(42);
    crate::test_runtime::create(&mut first, &mut server, &mut sessions, &peer).await;
    server.request_checkpoint();
    first.settle(&mut server, &mut sessions).await;
    let expected = server.checkpoint_for_test().unwrap();
    first.stop().await;
    let mut next = Worker::launch(backend, 3, false, false, EngineConfig::default());
    let event = next.event().await;
    let EventKind::Loaded(loaded) = event.kind else {
        panic!("Ready3 restart must not rerun clean adoption");
    };
    assert_eq!(loaded.state, expected);
    assert!(loaded.checkpoint.generation > 2);
    assert!(
        loaded
            .sessions
            .records
            .iter()
            .any(|row| row.lifecycle == SessionLifecycle::Open)
    );
    let mut restored = next.server(*loaded);
    next.settle(&mut restored, &mut crate::session::SessionMap::default())
        .await;
    assert_eq!(restored.gather_len(), 1);
    assert!(!restored.recovered());
}
