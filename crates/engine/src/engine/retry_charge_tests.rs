//! Compare scalar preflight with state reached through production commits.
use super::*;
use crate::{CapReleaseItem, LayoutOverride, MemJournal};

const CLIENT: ClientId = 4101;
fn permit() -> Option<RecordPermit> {
    Some(RecordPermit {
        max_record_bytes: 16 << 20,
    })
}
fn key(e: &Engine<MemJournal>) -> SessionKey {
    SessionKey {
        client: CLIENT,
        incarnation: e.session(CLIENT).unwrap().epoch,
    }
}
fn drain(e: &mut Engine<MemJournal>, mut step: EngineStep) {
    for _ in 0..128 {
        assert_ne!(step.state, StepState::Failed);
        step = match step.state {
            StepState::WaitingCommit(_) => {
                e.journal_mut().commit_all();
                e.journal_event(JournalEvent::Committed(e.journal().committed_seq()))
            }
            StepState::NeedsAdmission => e.step(None, permit()).unwrap(),
            _ => return,
        };
    }
    panic!("production step did not drain");
}
fn engine() -> Engine<MemJournal> {
    let mut cfg = EngineConfig {
        data_pools: vec![2, 3],
        ..Default::default()
    };
    cfg.pool_names.insert(3, "fast".into());
    let mut e = Engine::new(cfg, MemJournal::manual());
    e.journal_mut().commit_all();
    e.journal_event(JournalEvent::Committed(e.journal().last_seq()));
    e.open_session(CLIENT);
    e
}
fn clear_caps(e: &mut Engine<MemJournal>) {
    let inos: Vec<_> = e.session(CLIENT).unwrap().caps.iter().copied().collect();
    for ino in inos {
        let cap = e.cap(ino, CLIENT).unwrap();
        let items = vec![CapReleaseItem {
            ino,
            cap_id: cap.cap_id,
            mseq: cap.mseq,
            issue_seq: cap.last_issue,
        }];
        let step = e
            .step(
                Some(EngineInput::CapRelease { key: key(e), items }),
                permit(),
            )
            .unwrap();
        drain(e, step);
    }
}
fn apply(e: &mut Engine<MemJournal>, tid: u64, op: Op) {
    clear_caps(e);
    let input = EngineInput::Request {
        key: key(e),
        request: Request::new(tid, op).with_stamp(Timestamp::new(2000 + tid as u32, 0)),
    };
    let projected = e.projected_retry_charge(&input, None);
    let step = e.step(Some(input), permit()).unwrap();
    drain(e, step);
    let actual = e.retry_charge();
    assert_eq!(
        projected.completions, actual.completions,
        "completion count at tid {tid}"
    );
    assert_eq!(
        projected.bytes, actual.bytes,
        "metadata charge at tid {tid}"
    );
}
fn node(path: FilePath, mode: u32) -> Op {
    Op::Mknod {
        path,
        mode,
        rdev: 0,
        xattrs: XattrMap::new(),
        ino: 0,
    }
}
fn xattr(ino: Ino, name: &str, value: &[u8]) -> Op {
    Op::Setxattr {
        path: FilePath::ino(ino),
        name: name.as_bytes().to_vec(),
        value: value.to_vec(),
        flags: 0,
    }
}

#[test]
fn scalar_projection_matches_layout_history_namespace_and_creators() {
    let mut e = engine();
    apply(
        &mut e,
        1,
        xattr(
            CEPH_INO_ROOT,
            "ceph.dir.layout.pool_namespace",
            b"inherited",
        ),
    );
    let file = e.ino_alloc.peek().unwrap();
    apply(
        &mut e,
        2,
        node(FilePath::child(CEPH_INO_ROOT, "file"), S_IFREG | 0o644),
    );
    apply(
        &mut e,
        3,
        xattr(
            file,
            "ceph.file.layout.pool_namespace",
            b"a long replacement namespace",
        ),
    );
    apply(
        &mut e,
        4,
        xattr(file, "ceph.file.layout.pool_namespace", b"short"),
    );
    apply(&mut e,5,xattr(file,"ceph.file.layout.json",br#"{"stripe_unit":4194304,"stripe_count":1,"object_size":4194304,"pool_id":3,"pool_namespace":"JSON namespace"}"#));
    apply(
        &mut e,
        6,
        node(FilePath::child(CEPH_INO_ROOT, "fifo"), S_IFIFO | 0o644),
    );
    apply(
        &mut e,
        7,
        Op::Symlink {
            path: FilePath::child(CEPH_INO_ROOT, "link"),
            target: b"symlink bytes".to_vec(),
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    apply(
        &mut e,
        8,
        Op::Create {
            path: FilePath::child(CEPH_INO_ROOT, "created"),
            flags: CEPH_O_CREAT | CEPH_O_RDWR,
            mode: 0o644,
            layout: LayoutOverride {
                pool: 3,
                ..Default::default()
            },
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
}

#[test]
fn scalar_projection_matches_remote_rename_promotion_and_retained_directory() {
    let mut e = engine();
    let dir = e.ino_alloc.peek().unwrap();
    apply(
        &mut e,
        1,
        Op::Mkdir {
            path: FilePath::child(CEPH_INO_ROOT, "dir"),
            mode: 0o755,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    );
    let file = e.ino_alloc.peek().unwrap();
    apply(
        &mut e,
        2,
        node(FilePath::child(dir, "file"), S_IFREG | 0o644),
    );
    apply(
        &mut e,
        3,
        Op::Link {
            path: FilePath::child(dir, "remote"),
            target: FilePath::ino(file),
        },
    );
    apply(
        &mut e,
        4,
        Op::Rename {
            src: FilePath::child(dir, "remote"),
            dst: FilePath::child(dir, "a-long-remote-link"),
        },
    );
    apply(
        &mut e,
        5,
        Op::Unlink {
            path: FilePath::child(dir, "file"),
        },
    );
    apply(
        &mut e,
        6,
        Op::Unlink {
            path: FilePath::child(dir, "a-long-remote-link"),
        },
    );
    apply(&mut e, 7, node(FilePath::child(dir, "a"), S_IFREG | 0o644));
    apply(&mut e, 8, node(FilePath::child(dir, "b"), S_IFREG | 0o644));
    apply(
        &mut e,
        9,
        Op::Rename {
            src: FilePath::child(dir, "a"),
            dst: FilePath::child(dir, "b"),
        },
    );
    apply(
        &mut e,
        10,
        Op::Unlink {
            path: FilePath::child(dir, "b"),
        },
    );
    apply(
        &mut e,
        11,
        Op::Rmdir {
            path: FilePath::child(CEPH_INO_ROOT, "dir"),
        },
    );
}

#[test]
fn scalar_projection_preserves_existing_xattr_key_capacity() {
    let mut e = engine();
    let ino = e.ino_alloc.peek().unwrap();
    apply(
        &mut e,
        1,
        node(FilePath::child(CEPH_INO_ROOT, "f"), S_IFREG | 0o644),
    );
    let mut name = Vec::with_capacity(256);
    name.extend_from_slice(b"user.key");
    e.inodes
        .get_mut(&ino)
        .unwrap()
        .core
        .xattrs
        .insert(name, vec![1]);
    e.committed_inodes.insert(ino, e.inodes[&ino].core.clone());
    apply(&mut e, 2, xattr(ino, "user.key", b"replacement"));
}

#[test]
fn distinct_completions_share_one_derived_target_charge() {
    let mut e = engine();
    let ino = e.ino_alloc.peek().unwrap();
    apply(
        &mut e,
        1,
        node(FilePath::child(CEPH_INO_ROOT, "file"), S_IFREG | 0o644),
    );
    apply(
        &mut e,
        2,
        Op::Open {
            path: FilePath::ino(ino),
            flags: CEPH_O_RDWR,
            mode: 0,
            mask: 0,
            old_size: 0,
        },
    );
    assert_eq!(e.retry_refs, BTreeMap::from([(ino, 2)]));
    let expected = sum([
        Some(2 * COMPLETION + FIXED_INODE + INDEX),
        core_bytes(&e.inodes[&ino].core, false),
        core_bytes(&e.committed_inodes[&ino], false),
        topology(&e.inodes[&ino]),
    ]);
    assert_eq!(e.retry_charge().bytes, expected);
    let cp = e.checkpoint().unwrap();
    let restored = Engine::restore(
        e.config.clone(),
        MemJournal::at_seq(cp.covered_seq, false),
        cp,
    )
    .unwrap();
    assert_eq!(restored.retry_refs, BTreeMap::from([(ino, 2)]));
    assert_eq!(restored.retry_charge().bytes, expected);
}

fn parser_engine() -> (Engine<MemJournal>, Ino) {
    let mut e = engine();
    let ino = e.ino_alloc.peek().unwrap();
    apply(
        &mut e,
        1,
        node(FilePath::child(CEPH_INO_ROOT, "file"), S_IFREG | 0o644),
    );
    clear_caps(&mut e);
    (e, ino)
}
fn extension_json(extension: &str) -> String {
    format!(
        r#"{{"object_size":4194304,"stripe_unit":4194304,"stripe_count":1,"pool_id":2,"extension":{extension}}}"#
    )
}
fn parser_request(ino: Ino, name: &str, value: String) -> Request {
    Request::new(
        100,
        Op::Setxattr {
            path: FilePath::ino(ino),
            name: name.as_bytes().to_vec(),
            value: value.into_bytes(),
            flags: 0,
        },
    )
}

#[test]
fn e1x_1_wide_json_is_rejected_before_projection_or_execution_allocates_a_tree() {
    use std::fmt::Write;
    let mut value = String::from(
        r#"{"object_size":4194304,"stripe_unit":4194304,"stripe_count":1,"pool_id":2"#,
    );
    for n in 0..40_000 {
        write!(&mut value, ",\"k{n:05}\":0").unwrap();
    }
    value.push('}');
    for production in [true, false] {
        let (mut e, ino) = parser_engine();
        let request = parser_request(ino, "ceph.file.layout.json", value.clone());
        let before = super::super::vxattr::json_parse_count();
        let tail = e.journal().last_seq();
        let core = e.inode(ino).unwrap().clone();
        if production {
            match e.step(
                Some(EngineInput::Request {
                    key: key(&e),
                    request: request.clone(),
                }),
                permit(),
            ) {
                Err(StepError::RetryCapacity {
                    input:
                        Some(EngineInput::Request {
                            request: returned, ..
                        }),
                    permit: p,
                }) => {
                    assert_eq!(returned, request);
                    assert_eq!(p, permit());
                }
                other => panic!("wide parser pressure not returned: {other:?}"),
            }
        } else {
            assert!(e.handle_request(CLIENT, request).is_empty());
            assert_eq!(e.waiting_requests(), 1);
        }
        assert_eq!(super::super::vxattr::json_parse_count(), before);
        assert_eq!(e.journal().last_seq(), tail);
        assert_eq!(e.inode(ino).unwrap(), &core);
    }
}

#[test]
fn e1x_1_fitting_nested_array_extensions_keep_existing_layout_semantics() {
    let (mut e, ino) = parser_engine();
    let value = extension_json(&format!("[{}]", vec!["[]"; 10_000].join(",")));
    let before = super::super::vxattr::json_parse_count();
    let request = parser_request(ino, "ceph.file.layout.json", value);
    let step = e
        .step(
            Some(EngineInput::Request {
                key: key(&e),
                request,
            }),
            permit(),
        )
        .unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(super::super::vxattr::json_parse_count(), before + 2);
    drain(&mut e, step);
    assert_eq!(
        e.inode(ino).unwrap().layout,
        FileLayout::default_for_pool(2)
    );
}

#[test]
fn e1x_1_parser_threshold_accounts_for_coexisting_projection_map_before_parse() {
    use super::super::vxattr::{ParsePeak, json_parse_count, virtual_attr_parse_peak};
    let name = "ceph.file.layout.json";
    let ParsePeak::Bytes(base) = virtual_attr_parse_peak(name, &extension_json("\"\""), 0) else {
        panic!("finite base");
    };
    for scratch in [false, true] {
        let (mut e, ino) = parser_engine();
        let margin = if scratch { 128 } else { 1024 };
        let length = (TRANSIENT_LIMIT - margin - base) / 9;
        let value = extension_json(&format!("\"{}\"", "a".repeat(length)));
        let ParsePeak::Bytes(peak) = virtual_attr_parse_peak(name, &value, 0) else {
            panic!("finite threshold");
        };
        assert!(peak < TRANSIENT_LIMIT);
        println!(
            "parser threshold scratch={scratch} bytes={peak} margin={}",
            TRANSIENT_LIMIT - peak
        );
        let mut request = parser_request(ino, name, value);
        let cap_before = if scratch {
            let step = e
                .step(
                    Some(EngineInput::Request {
                        key: key(&e),
                        request: Request::new(
                            90,
                            Op::Getattr {
                                path: FilePath::ino(CEPH_INO_ROOT),
                                mask: 0,
                            },
                        ),
                    }),
                    permit(),
                )
                .unwrap();
            drain(&mut e, step);
            let cap = e.cap(CEPH_INO_ROOT, CLIENT).unwrap().clone();
            request.releases.push(crate::RequestRelease {
                ino: CEPH_INO_ROOT,
                cap_id: cap.cap_id,
                seq: cap.last_sent,
                issue_seq: cap.last_issue,
                mseq: cap.mseq,
                ..Default::default()
            });
            Some(cap)
        } else {
            None
        };
        let count = json_parse_count();
        let tail = e.journal().last_seq();
        let result = e.step(
            Some(EngineInput::Request {
                key: key(&e),
                request: request.clone(),
            }),
            permit(),
        );
        if scratch {
            match result {
                Err(StepError::RetryCapacity {
                    input: Some(EngineInput::Request { request: r, .. }),
                    permit: p,
                }) => {
                    assert_eq!(r, request);
                    assert_eq!(p, permit());
                }
                other => panic!("projection scratch omitted: {other:?}"),
            }
            assert_eq!(json_parse_count(), count);
            assert_eq!(e.journal().last_seq(), tail);
            assert_eq!(e.cap(CEPH_INO_ROOT, CLIENT), cap_before.as_ref());
        } else {
            let step = result.unwrap();
            assert!(matches!(step.state, StepState::WaitingCommit(_)));
            assert_eq!(json_parse_count(), count + 2);
            drain(&mut e, step);
        }
    }
}

#[test]
fn e1x_1_composite_layout_and_quota_maps_are_bounded_before_allocation() {
    use std::fmt::Write;
    let mut value = String::new();
    for n in 0..40_000 {
        if n != 0 {
            value.push(' ');
        }
        write!(&mut value, "k{n:05}=0").unwrap();
    }
    for name in ["ceph.file.layout", "ceph.quota"] {
        let (mut e, ino) = parser_engine();
        let count = super::super::vxattr::keys_values_parse_count();
        let request = parser_request(
            if name == "ceph.quota" {
                CEPH_INO_ROOT
            } else {
                ino
            },
            name,
            value.clone(),
        );
        assert!(matches!(
            e.step(
                Some(EngineInput::Request {
                    key: key(&e),
                    request
                }),
                permit()
            ),
            Err(StepError::RetryCapacity { .. })
        ));
        assert_eq!(super::super::vxattr::keys_values_parse_count(), count);
    }
}

#[test]
fn e1x_2_active_range_commit_keeps_one_context_until_callback_materialization() {
    let (mut e, ino) = parser_engine();
    let open = Op::Open {
        path: FilePath::ino(ino),
        flags: CEPH_O_RDWR,
        mode: 0,
        mask: 0,
        old_size: 0,
    };
    apply(&mut e, 2, open.clone());
    clear_caps(&mut e);
    let cp = e.checkpoint().unwrap();
    let mut cfg = e.config.clone();
    cfg.retry_limits.max_completions = 2;
    let mut e = Engine::restore(cfg, MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
    let k = key(&e);
    e.reconnect_chunk(k, &[], &[], false).unwrap();
    let step = e.finish_reconnect(&[k]).unwrap();
    drain(&mut e, step);
    let step = e
        .step(Some(EngineInput::RecoveryControlsDrained), permit())
        .unwrap();
    drain(&mut e, step);
    let step = e
        .step(
            Some(EngineInput::Request {
                key: k,
                request: Request::new(2, open),
            }),
            permit(),
        )
        .unwrap();
    assert!(matches!(step.state, StepState::WaitingCommit(_)));
    assert_eq!(e.owned_completion_contexts(), Some(1));
    assert_eq!(e.retry_charge().active_slots, Some(3));
    assert!(e.reply_holds_target(ino));
    e.journal_mut().commit_all();
    let step = e.journal_event(JournalEvent::Committed(e.journal().last_seq()));
    assert!(step.effects.iter().any(
        |x| matches!(x,Effect::Reply(r) if r.tid==2&&r.target.as_ref().is_some_and(|t|t.ino==ino))
    ));
    assert_eq!(e.owned_completion_contexts(), Some(0));
    assert_eq!(e.retry_charge().active_slots, Some(2));
}
