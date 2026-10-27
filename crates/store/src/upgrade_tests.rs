// Included in stable's compound-operation fault tests. No alternative WAL model.

fn assert_format_attempt_unchanged(
    backend: &TestBackend,
    before: &BTreeMap<String, Object>,
    calls: usize,
    writes: usize,
) {
    let memory = backend.0.lock().unwrap();
    assert_eq!(memory.native_calls, calls);
    assert_eq!(memory.writes, writes);
    assert_eq!(
        memory.objects.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>()
    );
    for (name, object) in before {
        assert_eq!(memory.objects[name].data, object.data, "{name} data");
        assert_eq!(memory.objects[name].attrs, object.attrs, "{name} xattrs");
        assert_eq!(memory.objects[name].values, object.values, "{name} omap");
    }
}

#[test]
fn upg1_preparing_completion_rejects_fabricated_ready_reconciliation() {
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let a = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&a, owner())).unwrap();
    assert_eq!((a.phase, a.revision), (crate::FormatPhase::Preparing, 1));
    let b_owner = replacement(owner());
    let (mut store, mut rank, b, _, sessions) = acquire_all(&backend, &a, b_owner);
    assert_eq!((b.phase, b.revision), (crate::FormatPhase::Preparing, 2));
    let opened = run(bootstrap.resume_bootstrap(&b, b_owner)).unwrap();
    let ready = run(bootstrap.publish_ready(&b)).unwrap();
    assert_eq!(
        (ready.phase, ready.revision),
        (crate::FormatPhase::Ready, 3)
    );
    assert!(matches!(
        run(bootstrap.upgrade_retry_targets(
            &ready,
            &mut store,
            &mut rank,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(())
        )),
        Err(StoreError::UpgradeRequired)
    ));

    // Use only public fields and the original genuine A fencing evidence.
    let mut fabricated = a.clone();
    fabricated.phase = crate::FormatPhase::Ready;
    fabricated.revision = 2;
    let before = backend.0.lock().unwrap().objects.clone();
    let calls = backend.0.lock().unwrap().native_calls;
    let writes = backend.0.lock().unwrap().writes;
    let result = run(bootstrap.acquire_format(
        &fabricated,
        writer(b_owner),
        FencedWriter {
            writer: a.writer.clone(),
            verified_osd_epoch: 55,
        },
    ));
    // A red run also exercises the granted proof, not only its diagnostic tag.
    let bypass = result.as_ref().ok().map(|forged| {
        run(bootstrap.upgrade_retry_targets(
            forged,
            &mut store,
            &mut rank,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(()),
        ))
        .map(|adopted| adopted.format.phase)
    });
    assert!(
        matches!(result, Err(StoreError::Corrupt)),
        "UPG1 acquisition={:?}, same-start upgrade={bypass:?}",
        result
            .as_ref()
            .map(|format| (format.phase, format.revision))
    );
    assert_format_attempt_unchanged(&backend, &before, calls, writes);

    // Only a real later Ready acquisition may adopt the unchanged seed.
    let (mut store, mut rank, c, opened, sessions) =
        acquire_all(&backend, &ready, replacement(b_owner));
    let adopted = run(bootstrap.upgrade_retry_targets(
        &c,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    let mut expected = a.plan.root_state;
    expected.recovery_semantics = RecoverySemantics::RetainedTargets3;
    assert_eq!(run(store.load(&adopted.opened)).unwrap().0, expected);
}

#[test]
fn upg1_ready_acquisition_reconciles_exact_arguments_after_lost_response() {
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let a = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&a, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&a)).unwrap();
    let next = replacement(owner());
    let evidence = FencedWriter {
        writer: ready.writer.clone(),
        verified_osd_epoch: 55,
    };
    let writes = backend.0.lock().unwrap().writes;
    backend.0.lock().unwrap().lose_reply = true;
    let acquired = run(bootstrap.acquire_format(&ready, writer(next), evidence.clone())).unwrap();
    assert_eq!(backend.0.lock().unwrap().writes, writes + 1);
    assert!(!backend.0.lock().unwrap().lose_reply);
    let reconciled = run(bootstrap.acquire_format(&ready, writer(next), evidence)).unwrap();
    assert_eq!(acquired, reconciled);
    assert_eq!(backend.0.lock().unwrap().writes, writes + 1);
    // acquire_all repeats the exact format acquisition, then acquires WAL/rank.
    let (mut store, mut rank, format, opened, sessions) = acquire_all(&backend, &ready, next);
    let adopted = run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    assert_eq!(adopted.format.phase, crate::FormatPhase::Ready);
    assert_eq!(
        adopted.opened.required_semantics,
        RecoverySemantics::RetainedTargets3
    );
}

#[test]
fn upg1_ready_proof_rejects_changed_public_snapshots_before_any_io() {
    type Mutation = (&'static str, fn(&mut crate::FormatInspection));
    let mutations: &[Mutation] = &[
        ("revision", |f| f.revision += 1),
        ("phase", |f| f.phase = crate::FormatPhase::Preparing),
        ("semantics", |f| {
            f.required_semantics = RecoverySemantics::RetainedTargets3
        }),
        ("writer", |f| f.writer.rank_owner.incarnation += 1),
        ("predecessor writer", |f| {
            f.predecessor.as_mut().unwrap().writer.rank_owner.gid += 1
        }),
        ("predecessor", |f| {
            f.predecessor.as_mut().unwrap().verified_osd_epoch += 1
        }),
        ("missing predecessor", |f| f.predecessor = None),
        ("identity", |f| f.plan.identity.filesystem_id += 1),
        ("cluster identity", |f| f.plan.identity.cluster_fsid[0] ^= 1),
        ("pool identity", |f| f.plan.identity.metadata_pool += 1),
        ("intent insertion", |f| {
            let source = f.plan.initial_checkpoint.clone();
            let mut target = source.clone();
            target.generation += 1;
            f.retry_upgrade = Some(crate::RetryUpgradePlan {
                id: [7; 16],
                from: RecoverySemantics::Legacy2,
                to: RecoverySemantics::RetainedTargets3,
                frozen_tail: source.covered_seq,
                prepared_by: f.writer.clone(),
                source,
                target,
            });
        }),
        ("descriptor generation", |f| {
            f.plan.initial_checkpoint.generation += 1
        }),
        ("descriptor coverage", |f| {
            f.plan.initial_checkpoint.covered_seq += 1
        }),
        ("descriptor name", |f| {
            f.plan.initial_checkpoint.manifest_name.push('x')
        }),
        ("descriptor checksum", |f| {
            f.plan.initial_checkpoint.manifest_checksum ^= 1
        }),
        ("bootstrap id", |f| f.plan.bootstrap_id[0] ^= 1),
        ("root", |f| f.plan.root_state.last_cap_id += 1),
        ("range", |f| f.plan.initial_range.len -= 1),
        ("initial owner", |f| f.plan.initial_owner.gid += 1),
        ("initial writer", |f| {
            f.plan.initial_writer.rank_owner.gid += 1
        }),
        ("object data", |f| f.plan.objects[0].data = Bytes::new()),
        ("object xattrs", |f| f.plan.objects[2].xattrs.clear()),
        ("object omap", |f| f.plan.objects[2].omap.clear()),
    ];
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let a = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&a, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&a)).unwrap();
    let b_owner = replacement(owner());
    let (mut store, mut rank, proof, opened, sessions) = acquire_all(&backend, &ready, b_owner);
    let before = backend.0.lock().unwrap().objects.clone();
    let calls = backend.0.lock().unwrap().native_calls;
    let writes = backend.0.lock().unwrap().writes;
    for (name, mutate) in mutations {
        let mut changed = proof.clone();
        mutate(&mut changed);
        let evidence = FencedWriter {
            writer: changed.writer.clone(),
            verified_osd_epoch: 55,
        };
        let result =
            run(bootstrap.acquire_format(&changed, writer(replacement(b_owner)), evidence));
        assert!(matches!(result, Err(StoreError::Corrupt)), "acquire {name}");
        let evidence = FencedWriter {
            writer: changed.writer.clone(),
            verified_osd_epoch: 55,
        };
        assert!(
            matches!(
                run(bootstrap.acquire_preparing(&changed, writer(replacement(b_owner)), evidence)),
                Err(StoreError::Corrupt)
            ),
            "acquire_preparing {name}"
        );
        assert!(
            matches!(
                run(bootstrap.upgrade_retry_targets(
                    &changed,
                    &mut store,
                    &mut rank,
                    &opened,
                    &sessions,
                    &engine::EngineConfig::default(),
                    [7; 16],
                    &|| Ok(())
                )),
                Err(StoreError::Corrupt)
            ),
            "upgrade {name}"
        );
        assert!(
            matches!(
                run(bootstrap.publish_ready(&changed)),
                Err(StoreError::Corrupt)
            ),
            "publish {name}"
        );
        assert!(
            matches!(
                run(bootstrap.validate_ready(&changed)),
                Err(StoreError::Corrupt)
            ),
            "validate {name}"
        );
        assert!(
            matches!(
                run(bootstrap.resume_bootstrap(&changed, b_owner)),
                Err(StoreError::Corrupt)
            ),
            "resume {name}"
        );
        assert_format_attempt_unchanged(&backend, &before, calls, writes);
    }

    let mut foreign_identity = identity();
    foreign_identity.filesystem_id += 1;
    let foreign = crate::BootstrapStore::new(backend.clone(), foreign_identity);
    assert!(matches!(
        run(foreign.acquire_format(
            &proof,
            writer(replacement(b_owner)),
            FencedWriter {
                writer: proof.writer.clone(),
                verified_osd_epoch: 55,
            }
        )),
        Err(StoreError::WrongFormat)
    ));
    assert_format_attempt_unchanged(&backend, &before, calls, writes);

    let adopted = run(bootstrap.upgrade_retry_targets(
        &proof,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    // Ready3 acquisition evidence includes the retained origin intent too.
    let (mut store, mut rank, proof, opened, sessions) =
        acquire_all(&backend, &adopted.format, replacement(b_owner));
    let intent_mutations: &[Mutation] = &[
        ("intent absence", |f| f.retry_upgrade = None),
        ("intent id", |f| {
            f.retry_upgrade.as_mut().unwrap().id[0] ^= 1
        }),
        ("intent source semantics", |f| {
            f.retry_upgrade.as_mut().unwrap().from = RecoverySemantics::RetainedTargets3
        }),
        ("intent target semantics", |f| {
            f.retry_upgrade.as_mut().unwrap().to = RecoverySemantics::Legacy2
        }),
        ("intent tail", |f| {
            f.retry_upgrade.as_mut().unwrap().frozen_tail += 1
        }),
        ("intent producer", |f| {
            f.retry_upgrade.as_mut().unwrap().prepared_by.rank_owner.gid += 1
        }),
        ("intent source descriptor", |f| {
            f.retry_upgrade.as_mut().unwrap().source.manifest_checksum ^= 1
        }),
        ("intent target descriptor", |f| {
            f.retry_upgrade
                .as_mut()
                .unwrap()
                .target
                .manifest_name
                .push('x')
        }),
    ];
    let before = backend.0.lock().unwrap().objects.clone();
    let calls = backend.0.lock().unwrap().native_calls;
    let writes = backend.0.lock().unwrap().writes;
    for (name, mutate) in intent_mutations {
        let mut changed = proof.clone();
        mutate(&mut changed);
        let result = run(bootstrap.acquire_format(
            &changed,
            writer(replacement(opened.owner)),
            FencedWriter {
                writer: changed.writer.clone(),
                verified_osd_epoch: 55,
            },
        ));
        assert!(matches!(result, Err(StoreError::Corrupt)), "acquire {name}");
        assert!(
            matches!(
                run(bootstrap.upgrade_retry_targets(
                    &changed,
                    &mut store,
                    &mut rank,
                    &opened,
                    &sessions,
                    &engine::EngineConfig::default(),
                    [7; 16],
                    &|| Ok(())
                )),
                Err(StoreError::Corrupt)
            ),
            "upgrade {name}"
        );
        assert_format_attempt_unchanged(&backend, &before, calls, writes);
    }
}

#[test]
fn upg1_ready_proof_cannot_follow_another_genuine_public_snapshot() {
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let a = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&a, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&a)).unwrap();
    let b_owner = replacement(owner());
    let (_, _, b, _, _) = acquire_all(&backend, &ready, b_owner);
    let (mut store, mut rank, c, opened, sessions) =
        acquire_all(&backend, &b, replacement(b_owner));
    let mut transplanted = b;
    transplanted.required_semantics = c.required_semantics;
    transplanted.retry_upgrade = c.retry_upgrade.clone();
    transplanted.revision = c.revision;
    transplanted.writer = c.writer.clone();
    transplanted.predecessor = c.predecessor.clone();
    transplanted.phase = c.phase;
    transplanted.plan = c.plan.clone();
    // Persisted-field equality does not establish runtime provenance.
    assert_eq!(transplanted, c);
    let before = backend.0.lock().unwrap().objects.clone();
    let calls = backend.0.lock().unwrap().native_calls;
    let writes = backend.0.lock().unwrap().writes;
    assert!(matches!(
        run(bootstrap.upgrade_retry_targets(
            &transplanted,
            &mut store,
            &mut rank,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(())
        )),
        Err(StoreError::Corrupt)
    ));
    assert_format_attempt_unchanged(&backend, &before, calls, writes);
    run(bootstrap.upgrade_retry_targets(
        &c,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
}

fn legacy_history_fixture() -> (
    TestBackend,
    crate::FormatInspection,
    engine::durability::EngineCheckpoint,
) {
    use engine::{durability::*, inode::*, journal::*, types::*};
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let plan = bootstrap_plan();
    let preparing = run(bootstrap.begin_bootstrap(plan.clone(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&preparing)).unwrap();
    let mut source = plan.root_state;
    let base = source.ino_alloc.free[0].start;
    source.ino_alloc.free[0].start += 4;
    source.ino_alloc.free[0].len -= 4;
    let layout = engine::EngineConfig::default().default_layout();
    let hash = source.inodes[0].core.dir_hash;
    for (offset, name, mode) in [
        (0, b"file".as_slice(), engine::consts::S_IFREG),
        (1, b"link".as_slice(), engine::consts::S_IFLNK),
    ] {
        let mut core = InodeCore::new(base + offset, mode | 0o644, 20, 30, Timestamp::new(77, 88));
        core.layout = layout.clone();
        core.rstat.rfiles = 1;
        core.rstat.rctime = core.ctime;
        core.xattrs
            .insert(b"user.kept".to_vec(), b"original-value".to_vec());
        if offset == 1 {
            core.symlink = b"file".to_vec();
            core.size = 4;
            core.rstat.rbytes = 4;
        }
        source.inodes[0].entries.push((
            DentryKey::new(hash, name),
            Dentry {
                ino: core.ino,
                primary: true,
            },
        ));
        source.inodes.push(CheckpointInode {
            core,
            entries: vec![],
            parent: Some((1, name.to_vec())),
            remote_links: vec![],
        });
    }
    source.inodes[0].entries.sort_by(|a, b| a.0.cmp(&b.0));
    source.inodes[0].core.dirstat.nfiles = 2;
    source.inodes[0].core.rstat.rfiles = 2;
    source.inodes[0].core.rstat.rbytes = 4;
    let historical = SessionKey {
        client: 11,
        incarnation: 1,
    };
    let mut stray = InodeCore::new(
        base + 2,
        engine::consts::S_IFREG | 0o600,
        0,
        0,
        Timestamp::default(),
    );
    stray.layout = layout.clone();
    stray.nlink = 0;
    stray.stray_auth_path = Some(b"/old-stray".to_vec());
    stray.client_ranges.insert(11, 4096);
    source.jobs = vec![
        LifecycleIntent {
            id: 1,
            ino: base + 3,
            kind: LifecycleKind::Purge {
                layouts: vec![layout.clone()],
                max_size: 8192,
            },
        },
        LifecycleIntent {
            id: 2,
            ino: base,
            kind: LifecycleKind::Truncate {
                layout: layout.clone(),
                from: 8192,
                size: 0,
                truncate_seq: 1,
            },
        },
        LifecycleIntent {
            id: 3,
            ino: stray.ino,
            kind: LifecycleKind::RecoverSize {
                layouts: vec![layout],
                max_size: 4096,
                inode_version: stray.version,
                truncate_seq: stray.truncate_seq,
            },
        },
    ];
    source.inodes.push(CheckpointInode {
        core: stray,
        entries: vec![],
        parent: None,
        remote_links: vec![],
    });
    source.next_job_id = 4;
    source.last_cap_id = 100;
    source.cleanup_receipts.push((historical, 1));
    engine::Engine::restore(
        engine::EngineConfig::default(),
        MemJournal::at_seq(1, false),
        source.clone(),
    )
    .unwrap();
    let store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
    let checkpoint = run(store.write_checkpoint_bytes(
        owner(),
        2,
        1,
        &crate::record::encode_checkpoint_legacy(&source).unwrap(),
    ))
    .unwrap();
    let keys = [
        SessionKey {
            client: 10,
            incarnation: 2,
        },
        SessionKey {
            client: 10,
            incarnation: 3,
        },
    ];
    let mut records = Vec::new();
    let mut core = source.inodes[1].core.clone();
    for key in keys {
        core.version += 1;
        core.client_ranges.insert(10, 8192);
        let mut range = JournalRecord::new(RecordKind::ClientRanges);
        range.recovery_semantics = RecoverySemantics::Legacy2;
        range.inodes.push(core.clone());
        records.push(range);
        core.version += 1;
        core.client_ranges.remove(&10);
        let mut cleanup = JournalRecord::new(RecordKind::ClientRanges);
        cleanup.recovery_semantics = RecoverySemantics::Legacy2;
        cleanup.inodes.push(core.clone());
        records.push(cleanup);
        let mut terminal = JournalRecord::new(RecordKind::SessionClose);
        terminal.recovery_semantics = RecoverySemantics::Legacy2;
        terminal.closed_sessions.push(key);
        records.push(terminal);
    }
    let mut values = BTreeMap::new();
    for (offset, record) in records.iter().enumerate() {
        let seq = offset as u64 + 2;
        values.insert(
            key(seq),
            framing::encode(
                FrameKind::Record,
                &identity(),
                owner(),
                seq,
                [8; 16],
                &crate::record::encode_record_legacy(record),
                1 << 20,
            )
            .unwrap(),
        );
    }
    let c = run(store.inspect_frozen()).unwrap();
    let checksum = values.values().fold(0, |crc, value| {
        crc32c::crc32c_append(crc, &value[..value.len() - 4])
    });
    let batch = BatchIdentity {
        id: [8; 16],
        before: 1,
        tail: 7,
        checksum,
    };
    let mut op = ObjectOperation::new();
    guards(&mut op, &identity(), owner(), &c.authority).unwrap();
    op.cmpxattr_str(TAIL, cmp_op::EQ, b"1")
        .omap_set_vals(&values)
        .setxattr(TAIL, decimal(7))
        .setxattr(BATCH, batch.encode())
        .setxattr(CHECKPOINT, checkpoint.encode().unwrap());
    run(backend.operate(OBJECT, op)).unwrap();
    let mut session_rows = BTreeMap::new();
    for (key, seq) in [(historical, 1), (keys[0], 4), (keys[1], 7)] {
        let row = crate::SessionRecord {
            key,
            identity: new_session(key.client),
            lifecycle: crate::SessionLifecycle::Closed,
            cleanup_seq: Some(seq),
        };
        session_rows.insert(
            crate::sessions::row_key(key),
            crate::sessions::encode_row(&row).unwrap(),
        );
        let mut key_bytes = BytesMut::new();
        key_bytes.put_u64_le(key.client);
        key_bytes.put_u64_le(key.incarnation);
        session_rows.insert(
            Bytes::from(format!("c/{:016x}", key.client)),
            key_bytes.freeze(),
        );
    }
    let repository = crate::SessionRepository::new(backend.clone(), identity(), 0, 128).unwrap();
    let control = run(repository.inspect_sessions()).unwrap();
    let mut next = control.clone();
    next.revision += 1;
    next.next_incarnation = 4;
    let mut op = repository.guarded(&control).unwrap();
    op.omap_set_vals(&session_rows)
        .setxattr(
            AUTHORITY,
            crate::sessions::authority(&next).encode().unwrap(),
        )
        .setxattr("rmds.next_incarnation", decimal(4));
    run(backend.operate("rmds.sessions.0", op)).unwrap();
    // Independent final logical oracle: only the real range images and terminal
    // receipts change. No history/session is copied into this expected image.
    source.covered_seq = 7;
    source.inodes[1].core = core;
    source.cleanup_receipts.extend([(keys[0], 4), (keys[1], 7)]);
    source.cleanup_receipts.sort_by_key(|(key, _)| *key);
    (backend, ready, source)
}

fn replacement(previous: OwnerToken) -> OwnerToken {
    OwnerToken {
        shard_epoch: previous.shard_epoch + 1,
        incarnation: previous.incarnation + 1,
        gid: previous.gid + 1,
        ..previous
    }
}

#[test]
fn exact_old_new_codec_pairs_and_unknown_target_semantics() {
    let legacy = Bytes::from(
        hex::decode(include_str!("../testdata/legacy-completion-wal.hex").trim()).unwrap(),
    );
    let old = crate::record::decode_record(&legacy).unwrap();
    assert_eq!(old.recovery_semantics, RecoverySemantics::Legacy2);
    assert_eq!(
        old.completed_request.as_ref().unwrap().created_ino,
        Some(99)
    );
    assert_eq!(
        old.completed_request.as_ref().unwrap().reply_target_ino,
        None
    );
    assert_eq!(
        crate::record::encode_record_bounded(&old, 1 << 20),
        Err(StoreError::UpgradeRequired)
    );
    assert_eq!(crate::record::encode_record_legacy(&old), legacy);
    let mut checkpoint = bootstrap_plan().root_state;
    checkpoint
        .sessions
        .push(engine::durability::CheckpointSession {
            key: engine::durability::SessionKey {
                client: 1,
                incarnation: 2,
            },
            completed_requests: vec![old.completed_request.as_ref().unwrap().clone()],
            completed_flushes: vec![],
            oldest_client_tid: 1,
            oldest_flush_tid: 0,
        });
    let old_checkpoint = crate::record::encode_checkpoint_legacy(&checkpoint).unwrap();
    assert_eq!(&old_checkpoint[..2], &[1, 1]);
    assert_eq!(
        crate::record::decode_checkpoint(&old_checkpoint).unwrap(),
        checkpoint
    );
    let mut mixed_checkpoint = old_checkpoint.to_vec();
    mixed_checkpoint[0] = 2;
    mixed_checkpoint[1] = 2;
    assert_eq!(
        crate::record::store_decode_error(
            crate::record::decode_checkpoint(&Bytes::from(mixed_checkpoint)).unwrap_err()
        ),
        StoreError::UnsupportedVersion
    );
    let mut new = old;
    new.recovery_semantics = RecoverySemantics::RetainedTargets3;
    new.completed_request.as_mut().unwrap().reply_target_ino = Some(99);
    let bytes = crate::record::encode_record_bounded(&new, 1 << 20).unwrap();
    assert_eq!(&bytes[..2], &[3, 3]);
    assert_eq!(&bytes[8..10], &[2, 2]);
    assert_eq!(crate::record::decode_record(&bytes).unwrap(), new);
    for version in [0, 1, 4, 255] {
        let mut unsupported = bytes.to_vec();
        unsupported[0] = version;
        let error = crate::record::decode_record(&Bytes::from(unsupported)).unwrap_err();
        assert_eq!(
            crate::record::store_decode_error(error),
            StoreError::UnsupportedVersion
        );
    }
    let mut mixed = bytes.to_vec();
    mixed[8] = 1;
    mixed[9] = 1;
    assert_eq!(
        crate::record::store_decode_error(
            crate::record::decode_record(&Bytes::from(mixed)).unwrap_err()
        ),
        StoreError::UnsupportedVersion
    );
}

#[test]
fn archived_old_authority_decoder_rejects_all_three_promoted_empty_controls() {
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let preparing = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&preparing)).unwrap();
    for name in ["rmds.format", OBJECT, "rmds.sessions.0"] {
        archived_authority_v1::decode(&backend.0.lock().unwrap().objects[name].attrs[AUTHORITY])
            .unwrap();
    }
    let (mut store, mut repository, format, opened, sessions) =
        acquire_all(&backend, &ready, replacement(owner()));
    run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut repository,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    let memory = backend.0.lock().unwrap();
    assert!(memory.objects[OBJECT].values.is_empty());
    for name in ["rmds.format", OBJECT, "rmds.sessions.0"] {
        let bytes = &memory.objects[name].attrs[AUTHORITY];
        assert_eq!(&bytes[..3], &[2, 3, 0]);
        assert_eq!(
            archived_authority_v1::decode(bytes),
            Err(StoreError::UnsupportedVersion)
        );
        let mut unknown = bytes.to_vec();
        unknown[1] = 4;
        let end = unknown.len() - 4;
        let crc = crc32c::crc32c(&unknown[..end]);
        unknown[end..].copy_from_slice(&crc.to_le_bytes());
        assert_eq!(
            Authority::decode(&Bytes::from(unknown)),
            Err(StoreError::UnsupportedVersion)
        );
    }
}

#[test]
fn legacy_range_only_reused_keys_upgrade_every_logical_field_and_nonreuse() {
    let (backend, old, expected) = legacy_history_fixture();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let (mut store, mut repository, format, opened, sessions) =
        acquire_all(&backend, &old, replacement(owner()));
    let result = run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut repository,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    let (candidate, tail) = run(store.load(&result.opened)).unwrap();
    let mut expected = expected;
    expected.recovery_semantics = RecoverySemantics::RetainedTargets3;
    assert_eq!(candidate, expected);
    assert!(tail.is_empty());
    assert!(backend.0.lock().unwrap().objects[OBJECT].values.is_empty());
    assert_eq!(
        run(repository.load_sessions(&result.sessions))
            .unwrap()
            .control
            .next_incarnation,
        4
    );
    let mut allocator = crate::InoAllocator::new(backend, identity());
    let reservation = run(allocator.reserve_range([8; 16], 16)).unwrap();
    assert!(reservation.start >= expected.ino_alloc.floor);
}

#[test]
fn legacy_range_history_missing_expired_or_wrong_receipt_never_invents_a_key() {
    for fault in 0..4 {
        let (backend, old, _) = legacy_history_fixture();
        let key1 = engine::durability::SessionKey {
            client: 10,
            incarnation: 2,
        };
        let mut memory = backend.0.lock().unwrap();
        let object = memory.objects.get_mut("rmds.sessions.0").unwrap();
        if fault == 0 {
            object.values.remove(&crate::sessions::row_key(key1));
        } else {
            let key = if fault == 3 {
                engine::durability::SessionKey {
                    client: 10,
                    incarnation: 3,
                }
            } else {
                key1
            };
            let row = crate::SessionRecord {
                key,
                identity: new_session(10),
                lifecycle: if fault == 3 {
                    crate::SessionLifecycle::Open
                } else {
                    crate::SessionLifecycle::Closed
                },
                cleanup_seq: if fault == 3 {
                    None
                } else {
                    Some(if fault == 1 { 1 } else { 3 })
                },
            };
            object.values.insert(
                crate::sessions::row_key(key),
                crate::sessions::encode_row(&row).unwrap(),
            );
        }
        drop(memory);
        let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
        let (mut store, mut repository, format, opened, sessions) =
            acquire_all(&backend, &old, replacement(owner()));
        let writes = backend.0.lock().unwrap().writes;
        let result = run(bootstrap.upgrade_retry_targets(
            &format,
            &mut store,
            &mut repository,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(()),
        ));
        assert!(
            matches!(result, Err(error) if error == if fault == 3 { StoreError::UpgradeRequired } else { StoreError::Corrupt }),
            "fault {fault}: {result:?}"
        );
        assert_eq!(backend.0.lock().unwrap().writes, writes);
        assert!(
            run(bootstrap.inspect_format())
                .unwrap()
                .unwrap()
                .retry_upgrade
                .is_none()
        );
    }
}

#[test]
fn upgrade_replacement_resumes_every_candidate_promotion_trim_and_ready_cut() {
    // Every write boundary: state, manifest, format barrier, sessions promotion,
    // WAL publication, six one-key trim pages, and final Ready.
    for cut in 0..12 {
        for lost_response in [false, true] {
            let (backend, old, mut expected) = legacy_history_fixture();
            let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
            let first_owner = replacement(owner());
            let (mut store, mut repository, format, opened, sessions) =
                acquire_all(&backend, &old, first_owner);
            store.limits.page_keys = 1;
            let before = backend.0.lock().unwrap().writes;
            if lost_response {
                backend.0.lock().unwrap().lose_at_write = Some(before + cut + 1);
            } else {
                backend.0.lock().unwrap().fail_at_write = Some(before + cut);
            }
            let result = run(bootstrap.upgrade_retry_targets(
                &format,
                &mut store,
                &mut repository,
                &opened,
                &sessions,
                &engine::EngineConfig::default(),
                [7; 16],
                &|| Ok(()),
            ));
            if !lost_response {
                assert!(result.is_err(), "cut {cut}");
            }
            let persisted = run(bootstrap.inspect_format()).unwrap().unwrap();
            let recorded_candidate = persisted
                .retry_upgrade
                .as_ref()
                .map(|p| (p.id, p.target.clone(), p.prepared_by.clone()));
            let (mut store, mut repository, format, opened, sessions) =
                acquire_all(&backend, &persisted, replacement(first_owner));
            store.limits.page_keys = 1;
            let result = run(bootstrap.upgrade_retry_targets(
                &format,
                &mut store,
                &mut repository,
                &opened,
                &sessions,
                &engine::EngineConfig::default(),
                [7; 16],
                &|| Ok(()),
            ))
            .unwrap_or_else(|e| panic!("cut={cut} lost={lost_response}: {e}"));
            expected.recovery_semantics = RecoverySemantics::RetainedTargets3;
            assert_eq!(run(store.load(&result.opened)).unwrap().0, expected);
            if let Some((id, descriptor, producer)) = recorded_candidate {
                let intent = result.format.retry_upgrade.as_ref().unwrap();
                assert_eq!(
                    (intent.id, intent.target.clone(), intent.prepared_by.clone()),
                    (id, descriptor, producer)
                );
            }
            assert!(backend.0.lock().unwrap().objects[OBJECT].values.is_empty());
        }
    }
}

#[test]
fn membership_loss_at_every_inner_read_write_page_and_reconciliation_stops_io() {
    let (backend, old, _) = legacy_history_fixture();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let (mut store, mut repository, format, opened, sessions) =
        acquire_all(&backend, &old, replacement(owner()));
    store.limits.page_keys = 1;
    let before = backend.0.lock().unwrap().native_calls;
    run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut repository,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    let operation_count = backend.0.lock().unwrap().native_calls - before;
    assert!(operation_count > 30);
    for cut in 0..=operation_count {
        let (backend, old, _) = legacy_history_fixture();
        let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
        let (mut store, mut repository, format, opened, sessions) =
            acquire_all(&backend, &old, replacement(owner()));
        store.limits.page_keys = 1;
        let before = backend.0.lock().unwrap().native_calls;
        let watch = || {
            if backend.0.lock().unwrap().native_calls >= before + cut {
                Err(StoreError::Fenced)
            } else {
                Ok(())
            }
        };
        let result = run(bootstrap.upgrade_retry_targets(
            &format,
            &mut store,
            &mut repository,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &watch,
        ));
        assert!(
            matches!(result, Err(StoreError::Fenced)),
            "cut {cut}: {result:?}"
        );
        assert_eq!(
            backend.0.lock().unwrap().native_calls,
            before + cut,
            "extra I/O after lost authority at {cut}"
        );
    }
}

#[test]
fn stale_wal_operations_compare_complete_authority_even_when_owner_is_unchanged() {
    for operation in 0..3 {
        let (backend, mut store) = fixture();
        let descriptor = if operation == 0 {
            None
        } else {
            run(store.append(owner(), 0, [4; 16], &records())).unwrap();
            let descriptor =
                run(store.write_checkpoint_bytes(owner(), 1, 2, &Bytes::from_static(b"state")))
                    .unwrap();
            if operation == 2 {
                run(store.publish_checkpoint(owner(), 0, &descriptor)).unwrap();
            }
            Some(descriptor)
        };
        let before = backend.0.lock().unwrap().objects[OBJECT].clone();
        backend.0.lock().unwrap().revise_authority_on_write = Some(OBJECT.to_owned());
        let result = match operation {
            0 => run(store.append(owner(), 0, [4; 16], &records())).map(|_| ()),
            1 => run(store.publish_checkpoint(owner(), 0, descriptor.as_ref().unwrap())),
            _ => run(store.trim(owner(), descriptor.as_ref().unwrap(), &[1, 2])),
        };
        assert_eq!(result, Err(StoreError::GuardRejected));
        let memory = backend.0.lock().unwrap();
        let after = &memory.objects[OBJECT];
        assert_eq!(before.data, after.data);
        assert_eq!(before.values, after.values);
        for (name, value) in before
            .attrs
            .iter()
            .filter(|(name, _)| name.as_str() != AUTHORITY)
        {
            assert_eq!(after.attrs[name], *value);
        }
    }
}

#[test]
fn old_prepared_operations_cannot_cross_new_owner_acquisition_on_any_object() {
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let preparing = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    let opened = run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&preparing)).unwrap();
    let repository = crate::SessionRepository::new(backend.clone(), identity(), 0, 128).unwrap();
    let old_sessions = run(repository.inspect_sessions()).unwrap();
    let mut old_wal = ObjectOperation::new();
    // An actually prepared old binary compared only identity/owner/tail.
    old_wal
        .cmpxattr_str(IDENTITY, cmp_op::EQ, &identity().encode())
        .cmpxattr_str(OWNER, cmp_op::EQ, &owner().encode())
        .cmpxattr_str(TAIL, cmp_op::EQ, opened.tail.to_string().as_bytes())
        .setxattr(TAIL, decimal(99));
    let mut old_format = bootstrap.guard(&ready).unwrap();
    old_format.setxattr("rmds.phase", Bytes::from_static(b"Preparing"));
    let mut old_rank = repository.guarded(&old_sessions).unwrap();
    old_rank.setxattr("rmds.next_incarnation", decimal(99));
    let (_store, _rank, _format, _opened, _sessions) =
        acquire_all(&backend, &ready, replacement(owner()));
    for (name, op) in [
        (OBJECT, old_wal),
        ("rmds.format", old_format),
        ("rmds.sessions.0", old_rank),
    ] {
        let before = backend.0.lock().unwrap().objects[name].clone();
        assert_eq!(
            run(backend.operate(name, op)),
            Err(StoreError::GuardRejected)
        );
        let after = backend.0.lock().unwrap().objects[name].clone();
        assert_eq!(before.attrs, after.attrs);
        assert_eq!(before.data, after.data);
        assert_eq!(before.values, after.values);
    }
}

#[test]
fn uncertain_pending_store_or_repository_action_blocks_upgrade_before_io() {
    let (backend, mut store) = fixture();
    backend.0.lock().unwrap().unknown_before_write = true;
    assert_eq!(
        run(store.append(owner(), 0, [4; 16], &records())),
        Err(StoreError::UnknownOutcome)
    );
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let format_store = crate::BootstrapStore::new(TestBackend::default(), identity());
    let dummy = run(format_store.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    let mut rank = crate::SessionRepository::new(backend.clone(), identity(), 0, 128).unwrap();
    let sessions = crate::AcquiredSessions {
        control: crate::SessionsInspection {
            required_semantics: RecoverySemantics::Legacy2,
            owner: rank_owner(owner()),
            writer: writer(owner()),
            predecessor: None,
            revision: 1,
            next_incarnation: 1,
        },
    };
    let opened = OpenedShard::from_frozen(run(store.inspect_frozen()).unwrap());
    let calls = backend.0.lock().unwrap().native_calls;
    assert!(matches!(
        run(bootstrap.upgrade_retry_targets(
            &dummy,
            &mut store,
            &mut rank,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(())
        )),
        Err(StoreError::UnknownOutcome)
    ));
    assert_eq!(backend.0.lock().unwrap().native_calls, calls);
    let (backend, mut rank, acquired) = session_fixture();
    backend.0.lock().unwrap().lose_reply = true;
    assert_eq!(
        run(rank.open_session_record(&acquired, acquired.control.revision, new_session(10))),
        Err(StoreError::UnknownOutcome)
    );
    let mut store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
    let opened = OpenedShard::from_frozen(run(store.inspect_frozen()).unwrap());
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let calls = backend.0.lock().unwrap().native_calls;
    assert!(matches!(
        run(bootstrap.upgrade_retry_targets(
            &dummy,
            &mut store,
            &mut rank,
            &opened,
            &acquired,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(())
        )),
        Err(StoreError::UnknownOutcome)
    ));
    assert_eq!(backend.0.lock().unwrap().native_calls, calls);
}

#[test]
fn valid_legacy_open_with_missing_original_target_is_upgrade_required_not_corrupt() {
    use engine::{durability::*, journal::*};
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let plan = bootstrap_plan();
    let preparing = run(bootstrap.begin_bootstrap(plan.clone(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&preparing)).unwrap();
    let mut state = plan.root_state;
    let key = SessionKey {
        client: 10,
        incarnation: 1,
    };
    state.sessions.push(CheckpointSession {
        key,
        completed_requests: vec![CompletedRequest {
            client: 10,
            session_incarnation: 1,
            tid: 1,
            op: engine::consts::CEPH_MDS_OP_CREATE,
            result: 0,
            created_ino: Some(state.ino_alloc.free[0].start),
            reply_target_ino: None,
            oldest_client_tid: 1,
        }],
        completed_flushes: vec![],
        oldest_client_tid: 1,
        oldest_flush_tid: 0,
    });
    engine::Engine::restore(
        engine::EngineConfig::default(),
        MemJournal::at_seq(1, false),
        state.clone(),
    )
    .unwrap();
    let store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
    let descriptor = run(store.write_checkpoint_bytes(
        owner(),
        2,
        1,
        &crate::record::encode_checkpoint_legacy(&state).unwrap(),
    ))
    .unwrap();
    let c = run(store.inspect_frozen()).unwrap();
    let mut op = ObjectOperation::new();
    guards(&mut op, &identity(), owner(), &c.authority).unwrap();
    op.setxattr(CHECKPOINT, descriptor.encode().unwrap());
    run(backend.operate(OBJECT, op)).unwrap();
    let rank = crate::SessionRepository::new(backend.clone(), identity(), 0, 128).unwrap();
    let c = run(rank.inspect_sessions()).unwrap();
    let row = crate::SessionRecord {
        key,
        identity: new_session(10),
        lifecycle: crate::SessionLifecycle::Open,
        cleanup_seq: None,
    };
    let mut raw_key = BytesMut::new();
    raw_key.put_u64_le(10);
    raw_key.put_u64_le(1);
    let values = [
        (
            crate::sessions::row_key(key),
            crate::sessions::encode_row(&row).unwrap(),
        ),
        (Bytes::from_static(b"c/000000000000000a"), raw_key.freeze()),
    ]
    .into_iter()
    .collect();
    let mut next = c.clone();
    next.next_incarnation = 2;
    next.revision += 1;
    let mut op = rank.guarded(&c).unwrap();
    op.omap_set_vals(&values)
        .setxattr(
            AUTHORITY,
            crate::sessions::authority(&next).encode().unwrap(),
        )
        .setxattr("rmds.next_incarnation", decimal(2));
    run(backend.operate("rmds.sessions.0", op)).unwrap();
    let (mut store, mut rank, format, opened, sessions) =
        acquire_all(&backend, &ready, replacement(owner()));
    let writes = backend.0.lock().unwrap().writes;
    assert!(matches!(
        run(bootstrap.upgrade_retry_targets(
            &format,
            &mut store,
            &mut rank,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(())
        )),
        Err(StoreError::UpgradeRequired)
    ));
    assert_eq!(backend.0.lock().unwrap().writes, writes);
    assert!(
        run(bootstrap.inspect_format())
            .unwrap()
            .unwrap()
            .retry_upgrade
            .is_none()
    );
}

#[test]
fn normal_semantics_three_load_rejects_legacy_checkpoint_or_uncovered_wal() {
    let (backend, old, _) = legacy_history_fixture();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let (mut store, mut rank, format, opened, sessions) =
        acquire_all(&backend, &old, replacement(owner()));
    let ready = run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    let legacy_pointer = ready.format.retry_upgrade.as_ref().unwrap().source.clone();
    let mut altered = ready.opened.frozen.clone();
    altered.checkpoint = Some(legacy_pointer.clone());
    let mut memory = backend.0.lock().unwrap();
    memory
        .objects
        .get_mut(OBJECT)
        .unwrap()
        .attrs
        .insert(CHECKPOINT.to_owned(), legacy_pointer.encode().unwrap());
    drop(memory);
    assert!(matches!(
        run(store.load(&OpenedShard::from_frozen(altered))),
        Err(StoreError::UnsupportedVersion)
    ));
    let mut old_record = engine::JournalRecord::new(engine::journal::RecordKind::ClientRanges);
    old_record.recovery_semantics = RecoverySemantics::Legacy2;
    let body = crate::record::encode_record_legacy(&old_record);
    let value = framing::encode(
        FrameKind::Record,
        &identity(),
        ready.opened.owner,
        8,
        [9; 16],
        &body,
        1 << 20,
    )
    .unwrap();
    let mut memory = backend.0.lock().unwrap();
    let object = memory.objects.get_mut(OBJECT).unwrap();
    object.attrs.insert(
        CHECKPOINT.to_owned(),
        ready.opened.checkpoint.as_ref().unwrap().encode().unwrap(),
    );
    object.attrs.insert(TAIL.to_owned(), decimal(8));
    object.values.insert(key(8), value);
    drop(memory);
    let opened = OpenedShard::from_frozen(run(store.inspect_frozen()).unwrap());
    assert!(matches!(
        run(store.load(&opened)),
        Err(StoreError::UnsupportedVersion)
    ));
}

#[test]
fn ready_three_normal_restart_keeps_new_open_rows_and_uses_latest_checkpoint() {
    let (backend, old, _) = legacy_history_fixture();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let first_owner = replacement(owner());
    let (mut store, mut rank, format, opened, sessions) = acquire_all(&backend, &old, first_owner);
    let upgraded = run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    let (key, _) = run(rank.open_session_record(
        &upgraded.sessions,
        upgraded.sessions.control.revision,
        new_session(20),
    ))
    .unwrap();
    let (mut state, _) = run(store.load(&upgraded.opened)).unwrap();
    state.sessions.push(engine::durability::CheckpointSession {
        key,
        completed_requests: vec![],
        completed_flushes: vec![],
        oldest_client_tid: 100,
        oldest_flush_tid: 90,
    });
    let descriptor = run(store.write_checkpoint(first_owner, 4, &state)).unwrap();
    run(store.publish_checkpoint(first_owner, 3, &descriptor)).unwrap();
    let ready = run(bootstrap.inspect_format()).unwrap().unwrap();
    let (mut store, mut rank, format, opened, sessions) =
        acquire_all(&backend, &ready, replacement(first_owner));
    let loaded = run(store.load(&opened)).unwrap().0;
    assert_eq!(loaded, state);
    assert_eq!(loaded.sessions[0].key, key);
    // Repeating the coordinator at Ready3 only refreshes/validates. It must not
    // apply the old all-Closed rule or substitute the retained origin pointer.
    let ready = run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    assert_eq!(ready.opened.checkpoint, Some(descriptor));
    assert_eq!(
        run(rank.load_sessions(&ready.sessions))
            .unwrap()
            .records
            .last()
            .unwrap()
            .lifecycle,
        crate::SessionLifecycle::Open
    );
}

#[test]
fn upgrade_candidate_capacity_failure_preserves_legacy_identity_without_deletion() {
    let (backend, old, _) = legacy_history_fixture();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let (mut store, mut rank, format, opened, sessions) =
        acquire_all(&backend, &old, replacement(owner()));
    // Seed generation + current source use four immutable objects. No room for
    // two candidate objects remains under this explicit backlog bound.
    store.limits.max_checkpoint_generations = 2;
    let before = backend.0.lock().unwrap().objects.clone();
    let writes = backend.0.lock().unwrap().writes;
    assert!(matches!(
        run(bootstrap.upgrade_retry_targets(
            &format,
            &mut store,
            &mut rank,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(())
        )),
        Err(StoreError::Capacity)
    ));
    let after = backend.0.lock().unwrap().objects.clone();
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>()
    );
    for (name, object) in &before {
        assert_eq!(object.attrs, after[name].attrs);
        assert_eq!(object.data, after[name].data);
        assert_eq!(object.values, after[name].values);
    }
    assert_eq!(backend.0.lock().unwrap().writes, writes);
}

#[test]
fn unknown_bootstrap_semantic_version_preserves_unsupported_error() {
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let plan = bootstrap_plan();
    run(bootstrap.begin_bootstrap(plan.clone(), writer(owner()))).unwrap();
    let bytes = plan.encode().unwrap();
    let mut body = framing::decode(&bytes, FrameKind::Bootstrap, &identity(), 1, 4 << 20)
        .unwrap()
        .body;
    let mut cursor = body.clone();
    cursor.advance(32);
    get_writer(&mut cursor).unwrap();
    let offset = body.len() - cursor.len() + 4; // length prefix precedes root checkpoint
    let mut altered = body.to_vec();
    altered[offset] = 99;
    body = Bytes::from(altered);
    let damaged = framing::encode(
        FrameKind::Bootstrap,
        &identity(),
        owner(),
        1,
        plan.bootstrap_id,
        &body,
        4 << 20,
    )
    .unwrap();
    let mut op = ObjectOperation::new();
    op.write_full(damaged);
    run(backend.operate("rmds.format", op)).unwrap();
    assert!(matches!(
        run(bootstrap.inspect_format()),
        Err(StoreError::UnsupportedVersion)
    ));
}

#[test]
fn crash_or_unknown_at_each_independent_acquisition_preserves_source_and_counter() {
    for cut in 0..3 {
        for unknown in [false, true] {
            let (backend, old, mut expected) = legacy_history_fixture();
            let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
            let store = StableStore::new(backend.clone(), identity(), Limits::default()).unwrap();
            let rank = crate::SessionRepository::new(backend.clone(), identity(), 0, 128).unwrap();
            let first_owner = replacement(owner());
            for step in 0..=cut {
                if step == cut {
                    if unknown {
                        backend.0.lock().unwrap().lose_reply = true;
                    } else {
                        let writes = backend.0.lock().unwrap().writes;
                        backend.0.lock().unwrap().fail_at_write = Some(writes);
                    }
                }
                let result = match step {
                    0 => run(bootstrap.acquire_format(
                        &old,
                        writer(first_owner),
                        FencedWriter {
                            writer: old.writer.clone(),
                            verified_osd_epoch: 55,
                        },
                    ))
                    .map(|_| ()),
                    1 => {
                        let c = run(store.inspect_control()).unwrap();
                        run(store.open(
                            &c,
                            first_owner,
                            writer(first_owner),
                            FencedWriter {
                                writer: c.writer.clone(),
                                verified_osd_epoch: 55,
                            },
                        ))
                        .map(|_| ())
                    }
                    _ => {
                        let c = run(rank.inspect_sessions()).unwrap();
                        run(rank.acquire_sessions(
                            &c,
                            rank_owner(first_owner),
                            writer(first_owner),
                            FencedWriter {
                                writer: c.writer.clone(),
                                verified_osd_epoch: 55,
                            },
                        ))
                        .map(|_| ())
                    }
                };
                if step == cut && !unknown {
                    assert_eq!(result, Err(StoreError::Io));
                } else {
                    result.unwrap();
                }
            }
            // A new assigned owner reads all actual mixed writers, not an
            // assumed transactional acquisition state.
            let old = run(bootstrap.inspect_format()).unwrap().unwrap();
            let current_wal = run(store.inspect_control()).unwrap();
            let mut next = replacement(first_owner);
            next.shard_epoch = current_wal.owner.shard_epoch + 1;
            let (mut store, mut rank, format, opened, sessions) = acquire_all(&backend, &old, next);
            let result = run(bootstrap.upgrade_retry_targets(
                &format,
                &mut store,
                &mut rank,
                &opened,
                &sessions,
                &engine::EngineConfig::default(),
                [7; 16],
                &|| Ok(()),
            ))
            .unwrap();
            expected.recovery_semantics = RecoverySemantics::RetainedTargets3;
            assert_eq!(run(store.load(&result.opened)).unwrap().0, expected);
            assert_eq!(run(rank.inspect_sessions()).unwrap().next_incarnation, 4);
        }
    }
}

#[test]
fn uncertain_uncommitted_upgrade_steps_retry_identical_operation_with_original_handles() {
    for cut in 0..12 {
        let (backend, old, mut expected) = legacy_history_fixture();
        let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
        let (mut store, mut rank, format, opened, sessions) =
            acquire_all(&backend, &old, replacement(owner()));
        store.limits.page_keys = 1;
        let writes = backend.0.lock().unwrap().writes;
        backend.0.lock().unwrap().unknown_at_write = Some(writes + cut);
        let result = run(bootstrap.upgrade_retry_targets(
            &format,
            &mut store,
            &mut rank,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(()),
        ));
        assert!(
            matches!(result, Err(StoreError::UnknownOutcome)),
            "cut {cut}: {result:?}"
        );
        let result = run(bootstrap.upgrade_retry_targets(
            &format,
            &mut store,
            &mut rank,
            &opened,
            &sessions,
            &engine::EngineConfig::default(),
            [7; 16],
            &|| Ok(()),
        ))
        .unwrap_or_else(|e| panic!("retry cut {cut}: {e}"));
        expected.recovery_semantics = RecoverySemantics::RetainedTargets3;
        assert_eq!(run(store.load(&result.opened)).unwrap().0, expected);
        assert_eq!(result.format.retry_upgrade.unwrap().id, [7; 16]);
    }
}

#[test]
fn format_intent_is_bounded_versioned_required_and_distinct_from_absence() {
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let preparing = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&preparing)).unwrap();
    let (mut store, mut rank, format, opened, sessions) =
        acquire_all(&backend, &ready, replacement(owner()));
    let result = run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    let plan = result.format.retry_upgrade.unwrap();
    let bytes = plan.encode().unwrap();
    assert!(bytes.len() <= 8192);
    assert_eq!(&bytes[..5], &[1, 2, 0, 3, 0]);
    assert_eq!(crate::RetryUpgradePlan::decode(&bytes).unwrap(), plan);
    assert_eq!(
        crate::RetryUpgradePlan::decode(&Bytes::from(vec![0; 8193])),
        Err(StoreError::Capacity)
    );
    let mut unknown = bytes.to_vec();
    unknown[0] = 2;
    let end = unknown.len() - 4;
    let crc = crc32c::crc32c(&unknown[..end]);
    unknown[end..].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(
        crate::RetryUpgradePlan::decode(&Bytes::from(unknown)),
        Err(StoreError::UnsupportedVersion)
    );
    for empty in [false, true] {
        let mut memory = backend.0.lock().unwrap();
        let attrs = &mut memory.objects.get_mut("rmds.format").unwrap().attrs;
        if empty {
            attrs.insert("rmds.retry_upgrade".to_owned(), Bytes::new());
        } else {
            attrs.remove("rmds.retry_upgrade");
        }
        drop(memory);
        assert!(matches!(
            run(bootstrap.inspect_format()),
            Err(StoreError::Corrupt)
        ));
    }
}

#[test]
fn normal_publication_cannot_publish_an_unpublished_predecessor_candidate() {
    let backend = TestBackend::default();
    let bootstrap = crate::BootstrapStore::new(backend.clone(), identity());
    let preparing = run(bootstrap.begin_bootstrap(bootstrap_plan(), writer(owner()))).unwrap();
    run(bootstrap.resume_bootstrap(&preparing, owner())).unwrap();
    let ready = run(bootstrap.publish_ready(&preparing)).unwrap();
    let first = replacement(owner());
    let (mut store, mut rank, format, opened, sessions) = acquire_all(&backend, &ready, first);
    let upgraded = run(bootstrap.upgrade_retry_targets(
        &format,
        &mut store,
        &mut rank,
        &opened,
        &sessions,
        &engine::EngineConfig::default(),
        [7; 16],
        &|| Ok(()),
    ))
    .unwrap();
    let state = run(store.load(&upgraded.opened)).unwrap().0;
    let orphan = run(store.write_checkpoint(first, 3, &state)).unwrap();
    let next = replacement(first);
    let (store, _, _, _, _) = acquire_all(&backend, &upgraded.format, next);
    let before = backend.0.lock().unwrap().objects[OBJECT].clone();
    assert_eq!(
        run(store.publish_checkpoint(next, 2, &orphan)),
        Err(StoreError::Fenced)
    );
    let after = backend.0.lock().unwrap().objects[OBJECT].clone();
    assert_eq!(before.attrs, after.attrs);
    assert_eq!(before.values, after.values);
}
