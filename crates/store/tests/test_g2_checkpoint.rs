//! Persistent schemas compare every recovery field, not only namespace equality.

use bytes::{Bytes, BytesMut};
use engine::{durability::*, ino::InoAllocState, inode::*, journal::*, types::*};
use store::record::*;

fn state() -> EngineCheckpoint {
    let key = SessionKey {
        client: 45,
        incarnation: 98,
    };
    let mut core = InodeCore::new(
        0x10001,
        engine::consts::S_IFREG | 0o640,
        2,
        3,
        Timestamp::new(100, 20),
    );
    core.stray_auth_path = Some(b"/former/primary".to_vec());
    core.stable_subvolume_id = 0x10000;
    core.client_ranges.insert(key.client, 16 << 20);
    core.old_pools.insert(9);
    core.old_layouts.push(FileLayout {
        stripe_unit: 1024,
        stripe_count: 4,
        object_size: 8192,
        pool_id: 9,
        pool_ns: "old-namespace".to_owned(),
    });
    core.xattrs.insert(b"user.test".to_vec(), b"value".to_vec());
    core.snaprealm = Some(SnapRealmNode {
        seq: 3,
        created: 1,
        subvolume: true,
        last_modified: Timestamp::new(5, 6),
        change_attr: 7,
    });
    let layout = FileLayout {
        stripe_unit: 4096,
        stripe_count: 2,
        object_size: 8192,
        pool_id: 7,
        pool_ns: "archive".to_owned(),
    };
    EngineCheckpoint {
        recovery_semantics: RecoverySemantics::RetainedTargets3,
        covered_seq: 32,
        inodes: vec![CheckpointInode {
            core,
            entries: vec![(
                DentryKey {
                    hash: 0x123456,
                    name: b"child".to_vec(),
                },
                Dentry {
                    ino: 0x10002,
                    primary: false,
                },
            )],
            parent: Some((1, b"primary".to_vec())),
            remote_links: vec![(2, b"remote".to_vec())],
        }],
        ino_alloc: InoAllocState {
            free: vec![InoRange {
                start: 0x20000,
                len: 50,
            }],
            delegated: vec![(
                key.client,
                InoRange {
                    start: 0x30000,
                    len: 4,
                },
            )],
            floor: 0x40000,
        },
        sessions: vec![
            CheckpointSession {
                key,
                completed_requests: vec![CompletedRequest {
                    client: key.client,
                    session_incarnation: key.incarnation,
                    tid: 19,
                    op: 3,
                    result: 0,
                    created_ino: Some(0x10001),
                    reply_target_ino: Some(0x10001),
                    oldest_client_tid: 17,
                }],
                completed_flushes: vec![20, 21],
                oldest_client_tid: 17,
                oldest_flush_tid: 18,
            },
            CheckpointSession {
                key: SessionKey {
                    client: 46,
                    incarnation: 99,
                },
                completed_requests: vec![],
                completed_flushes: vec![],
                oldest_client_tid: 500,
                oldest_flush_tid: 600,
            },
        ],
        jobs: vec![
            LifecycleIntent {
                id: 1,
                ino: 0x10001,
                kind: LifecycleKind::Purge {
                    layouts: vec![layout.clone()],
                    max_size: 99,
                },
            },
            LifecycleIntent {
                id: 2,
                ino: 0x10001,
                kind: LifecycleKind::Truncate {
                    layout: layout.clone(),
                    from: 900,
                    size: 500,
                    truncate_seq: 2,
                },
            },
            LifecycleIntent {
                id: 3,
                ino: 0x10001,
                kind: LifecycleKind::RecoverSize {
                    layouts: vec![layout],
                    max_size: 1000,
                    inode_version: 5,
                    truncate_seq: 4,
                },
            },
        ],
        next_job_id: 4,
        last_cap_id: 200,
        cleanup_receipts: vec![(
            SessionKey {
                client: 47,
                incarnation: 97,
            },
            30,
        )],
    }
}

#[test]
fn materialized_checkpoint_roundtrip_preserves_all_fields() {
    let state = state();
    let bytes = encode_checkpoint(&state);
    assert_eq!(decode_checkpoint(&bytes).unwrap(), state);
    for n in 0..bytes.len() {
        assert!(decode_checkpoint(&bytes.slice(..n)).is_err(), "length {n}");
    }
}

#[test]
fn new_wal_semantics_roundtrip_in_nested_frames() {
    let state = state();
    let mut record = JournalRecord::new(RecordKind::LifecycleComplete);
    record.inodes.push(state.inodes[0].core.clone());
    record.ino_alloc = Some(state.ino_alloc);
    record.completed_request = Some(state.sessions[0].completed_requests[0].clone());
    record.client_flush = Some(ClientFlush {
        client: 45,
        session_incarnation: 98,
        tid: 23,
        oldest_flush_tid: 22,
    });
    record.closed_sessions.push(state.sessions[0].key);
    record.lifecycle_intents = state.jobs;
    record.completed_jobs = vec![6, 7];
    let bytes = encode_record(&record);
    assert_eq!(decode_record(&bytes).unwrap(), record);
    for n in 0..bytes.len() {
        assert!(decode_record(&bytes.slice(..n)).is_err(), "length {n}");
    }
}

#[test]
fn unsupported_semantics_are_not_silently_read_as_old_records() {
    let record = JournalRecord::new(RecordKind::Create);
    let bytes = encode_record(&record);
    for version in [0, 1, 4, 255] {
        let mut changed = bytes.to_vec();
        changed[0] = version;
        assert!(decode_record(&Bytes::from(changed)).is_err());
    }
    let mut changed = BytesMut::from(bytes.as_ref());
    changed.extend_from_slice(b"extra");
    assert!(decode_record(&changed.freeze()).is_err());
}

#[test]
fn production_encoder_rejects_capacity_before_allocating_output() {
    let record = JournalRecord::new(RecordKind::Create);
    assert_eq!(
        encode_record_bounded(&record, 1),
        Err(store::StoreError::Capacity)
    );
    assert_eq!(
        encode_record_bounded(&record, 4096).unwrap(),
        encode_record(&record)
    );
    let checkpoint = state();
    assert_eq!(
        encode_checkpoint_bounded(&checkpoint, 100),
        Err(store::StoreError::Capacity)
    );
    assert_eq!(
        encode_checkpoint_bounded(&checkpoint, 1 << 20).unwrap(),
        encode_checkpoint(&checkpoint)
    );
}

#[test]
fn checkpoint_then_unlink_codec_replay_preserves_authorization_path_and_identity() {
    use engine::consts::*;
    use engine::{Engine, EngineConfig, Op, Request};
    let config = EngineConfig::default();
    let mut e = Engine::new(config.clone(), MemJournal::new());
    e.open_session(45);
    let mut tid = 0;
    let mut request = |e: &mut Engine<MemJournal>, op: Op| {
        tid += 1;
        let mut effects = e.handle_request(45, Request::new(tid, op));
        let mut cursor = 0;
        while cursor < effects.len() {
            assert!(cursor < 64);
            if let engine::Effect::Caps(message) = effects[cursor].clone()
                && message.op == CEPH_CAP_OP_REVOKE
            {
                effects.extend(e.handle_cap_update(
                    message.client,
                    engine::CapUpdate {
                        op: CEPH_CAP_OP_UPDATE,
                        ino: message.ino,
                        cap_id: message.cap_id,
                        seq: message.seq,
                        issue_seq: message.issue_seq,
                        mseq: message.mseq,
                        caps: message.caps,
                        wanted: message.wanted,
                        ..Default::default()
                    },
                ));
            }
            cursor += 1;
        }
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, engine::Effect::Reply(reply) if reply.result == 0)),
            "{effects:?}"
        );
    };
    request(
        &mut e,
        Op::Mkdir {
            path: FilePath::child(CEPH_INO_ROOT, "sv"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let sv = e.lookup_name(CEPH_INO_ROOT, b"sv").unwrap();
    request(
        &mut e,
        Op::Setxattr {
            path: FilePath::ino(sv),
            name: b"ceph.dir.subvolume".to_vec(),
            value: b"1".to_vec(),
            flags: 0,
        },
    );
    request(
        &mut e,
        Op::Create {
            path: FilePath::child(sv, "f"),
            flags: CEPH_O_CREAT | CEPH_O_RDWR,
            mode: 0o640,
            layout: Default::default(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let f = e.lookup_name(sv, b"f").unwrap();
    let checkpoint = decode_checkpoint(&encode_checkpoint(&e.checkpoint().unwrap())).unwrap();
    let seq = checkpoint.covered_seq;
    request(
        &mut e,
        Op::Unlink {
            path: FilePath::child(sv, "f"),
        },
    );
    let records: Vec<_> = e
        .journal()
        .records()
        .iter()
        .filter(|(s, _)| *s > seq)
        .map(|(s, r)| (*s, decode_record(&encode_record(r)).unwrap()))
        .collect();
    assert_eq!(records.len(), 1);
    let mut restored = Engine::restore(config, MemJournal::at_seq(seq, false), checkpoint).unwrap();
    for (seq, record) in records {
        restored.replay_record(seq, &record).unwrap();
    }
    assert_eq!(
        restored.inode(f).unwrap().stray_auth_path.as_deref(),
        Some(b"/sv/f".as_slice())
    );
    assert_eq!(restored.inode(f).unwrap().stable_subvolume_id, sv);
    assert_eq!(restored.parent_of(f), None);
}
