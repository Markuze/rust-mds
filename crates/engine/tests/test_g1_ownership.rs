//! Public compatibility reservation ownership regression. No boundary seam.
mod common;
use common::*;
use engine::consts::*;
use engine::*;

fn node(tid: u64, name: &str) -> Request {
    Request::new(
        tid,
        Op::Mknod {
            path: FilePath::child(ROOT, name),
            mode: S_IFREG | 0o644,
            rdev: 0,
            xattrs: XattrMap::new(),
            ino: 0,
        },
    )
    .with_flags(CEPH_MDS_FLAG_WANT_DENTRY)
}

#[test]
fn r3_compatibility_replay_preserves_another_continuations_reservation_and_own_credit() {
    for replay_between in [false, true] {
        let mut fs = Fs::new();
        let original = node(1, "recorded");
        fs.call_req(C1, original.clone());
        let cp = fs.e.checkpoint().unwrap();
        let mut e =
            Engine::restore(config(), MemJournal::at_seq(cp.covered_seq, false), cp).unwrap();
        let keys: Vec<_> = [C1, C2, C3]
            .into_iter()
            .map(|client| SessionKey {
                client,
                incarnation: e.session(client).unwrap().epoch,
            })
            .collect();
        for key in &keys {
            e.reconnect_chunk(*key, &[], &[], false).unwrap();
        }
        let step = e.finish_reconnect(&keys).unwrap();
        assert!(step.effects.is_empty());
        e.step(
            Some(EngineInput::RecoveryControlsDrained),
            Some(RecordPermit {
                max_record_bytes: 16 << 20,
            }),
        )
        .unwrap();
        let key = keys[0];
        let owned = node(100, "owned");
        e.step(
            Some(EngineInput::Request {
                key,
                request: owned.clone(),
            }),
            None,
        )
        .unwrap();
        let before_replay = e.retry_capacity_epoch();
        let tail = e.journal().last_seq();
        if replay_between {
            let caps_before: Vec<_> = e
                .inode_numbers()
                .into_iter()
                .map(|ino| (ino, e.caps_of(ino)))
                .collect();
            let mut replay = original;
            replay.flags |= CEPH_MDS_FLAG_REPLAY;
            let effects = e.handle_request(C1, replay);
            let reply = reply_for(&effects, C1, 1).unwrap();
            assert_eq!(reply.result, 0);
            assert!(reply.target.is_none() && reply.dentry.is_none());
            let caps_after: Vec<_> = e
                .inode_numbers()
                .into_iter()
                .map(|ino| (ino, e.caps_of(ino)))
                .collect();
            assert_eq!(caps_after, caps_before);
            assert_eq!(
                e.retry_capacity_epoch(),
                before_replay + 1,
                "captured replay has its own release"
            );
        }
        let before_floor = e.retry_capacity_epoch();
        let mut read = Request::new(
            200,
            Op::Getattr {
                path: FilePath::ino(ROOT),
                mask: 0,
            },
        );
        read.oldest_client_tid = 101;
        e.handle_request(C1, read);
        assert_eq!(e.retry_capacity_epoch(), before_floor);
        match e.step(None, None) {
            Err(StepError::InvalidRetryIdentity {
                input:
                    Some(EngineInput::Request {
                        key: returned_key,
                        request,
                    }),
                permit: None,
            }) => {
                assert_eq!(returned_key, key);
                assert_eq!(request, owned);
            }
            other => panic!("lost stale owned input: {other:?}"),
        }
        assert_eq!(e.retry_capacity_epoch(), before_floor + 1);
        assert_eq!(e.journal().last_seq(), tail);
        assert_eq!(e.session(C1).unwrap().durable_oldest_client_tid, 0);
        println!("replay_between={replay_between}: actual stale-return release delta=1");
    }
}
