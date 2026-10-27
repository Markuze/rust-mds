//! Compatibility cleanup must not cross a latched journal failure.
use super::*;
use crate::{EngineConfig, JournalError, JournalEvent, JournalSeq, MemJournal, Op, StepState};

const CLIENT: ClientId = 4101;

struct TestJournal {
    inner: MemJournal,
    reject: bool,
    attempts: Vec<JournalRecord>,
}
impl Journal for TestJournal {
    fn submit(&mut self, record: JournalRecord) -> Result<JournalSeq, JournalError> {
        self.attempts.push(record.clone());
        if self.reject {
            Err(JournalError::Capacity)
        } else {
            self.inner.submit(record)
        }
    }
    fn committed_seq(&self) -> JournalSeq {
        self.inner.committed_seq()
    }
    fn flush(&mut self) {
        self.inner.flush();
    }
}

fn fixture() -> (Engine<TestJournal>, Vec<Ino>) {
    let mut e = Engine::new(
        EngineConfig::default(),
        TestJournal {
            inner: MemJournal::new(),
            reject: false,
            attempts: Vec::new(),
        },
    );
    e.open_session(CLIENT);
    let mut files = Vec::new();
    for (tid, name) in [(1, "a"), (3, "b")] {
        let effects = e.handle_request(
            CLIENT,
            Request::new(
                tid,
                Op::Mknod {
                    path: FilePath::child(CEPH_INO_ROOT, name),
                    mode: S_IFREG | 0o644,
                    rdev: 0,
                    xattrs: XattrMap::new(),
                    ino: 0,
                },
            ),
        );
        let ino = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Reply(reply) => reply.target.as_ref().map(|target| target.ino),
                _ => None,
            })
            .unwrap();
        let effects = e.handle_request(
            CLIENT,
            Request::new(
                tid + 1,
                Op::Open {
                    path: FilePath::ino(ino),
                    flags: CEPH_O_RDWR,
                    mode: 0,
                    mask: 0,
                    old_size: 0,
                },
            ),
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Reply(reply) if reply.result == 0))
        );
        assert!(e.inode(ino).unwrap().client_ranges.contains_key(&CLIENT));
        files.push(ino);
    }
    // Test-only allocator setup. No public API or production seam is added.
    e.ino_alloc.delegate(CLIENT, 7).unwrap();
    (e, files)
}

fn cleanup(e: &mut Engine<TestJournal>, files: &[Ino], close: bool, kill: bool) -> Vec<Effect> {
    if close {
        e.close_session(CLIENT, kill)
    } else {
        let items: Vec<_> = files
            .iter()
            .map(|ino| {
                let cap = e.cap(*ino, CLIENT).unwrap();
                CapReleaseItem {
                    ino: *ino,
                    cap_id: cap.cap_id,
                    mseq: cap.mseq,
                    issue_seq: cap.last_issue,
                }
            })
            .collect();
        e.handle_cap_release(CLIENT, &items)
    }
}

fn failure_case(counter: bool, close: bool, kill: bool) {
    let (mut e, files) = fixture();
    let later_cap = e.cap(files[1], CLIENT).unwrap().clone();
    let later_core = e.inode(files[1]).unwrap().clone();
    let allocator = e.ino_alloc.state();
    let key = crate::SessionKey {
        client: CLIENT,
        incarnation: e.session_epoch(CLIENT),
    };
    let session_state = e.session(CLIENT).unwrap().state;
    let tail = e.journal().inner.last_seq();
    let attempts = e.journal().attempts.len();
    assert_eq!(e.retry_capacity_epoch(), 4);
    if counter {
        e.retry_capacity_epoch = u64::MAX;
    } else {
        e.journal_mut().reject = true;
    }
    let epoch = e.retry_capacity_epoch();
    assert!(cleanup(&mut e, &files, close, kill).is_empty());
    assert_eq!(e.failure(), Some(JournalError::Capacity));
    assert_eq!(e.journal().attempts.len(), attempts + 1);
    let attempted = e.journal().attempts.last().unwrap();
    assert_eq!(attempted.kind, RecordKind::ClientRanges);
    assert!(attempted.inodes.iter().any(|core| core.ino == files[0]));
    assert!(!attempted.inodes.iter().any(|core| core.ino == files[1]));
    assert_eq!(e.journal().inner.last_seq(), tail + u64::from(counter));
    assert_eq!(e.retry_capacity_epoch(), epoch);
    assert_eq!(e.cap(files[1], CLIENT), Some(&later_cap));
    assert_eq!(e.inode(files[1]), Some(&later_core));
    assert_eq!(e.ino_alloc.state(), allocator);
    assert_eq!(e.session(CLIENT).unwrap().state, session_state);
    assert!(e.session(CLIENT).unwrap().caps.contains(&files[1]));
    assert_eq!(e.session_cleanup_seq(key), None);
    assert_eq!(e.commits.len(), usize::from(counter));
    let uncertain = e
        .commits
        .front()
        .map(|commit| (commit.seq, commit.images.clone(), commit.jobs.clone()));
    if counter && kill {
        assert_eq!(e.commits[0].jobs.len(), 1);
        assert_eq!(e.commits[0].jobs[0].ino, files[0]);
    }
    let flushes = e.journal().inner.flush_count();
    // Terminal helpers cannot retry the remaining cleanup or consume work.
    assert!(e.close_session(CLIENT, kill).is_empty());
    assert!(e.handle_cap_release(CLIENT, &[]).is_empty());
    assert!(e.handle_cap_update(CLIENT, CapUpdate::default()).is_empty());
    let late = e.journal_event(JournalEvent::Committed(tail + u64::from(counter)));
    assert_eq!(late.state, StepState::Failed);
    assert!(late.effects.is_empty() && late.notices.is_empty());
    assert_eq!(e.journal().attempts.len(), attempts + 1);
    assert_eq!(e.journal().inner.flush_count(), flushes);
    assert_eq!(e.cap(files[1], CLIENT), Some(&later_cap));
    assert_eq!(e.inode(files[1]), Some(&later_core));
    assert_eq!(e.ino_alloc.state(), allocator);
    assert_eq!(e.session_cleanup_seq(key), None);
    assert_eq!(
        e.commits
            .front()
            .map(|commit| (commit.seq, commit.images.clone(), commit.jobs.clone())),
        uncertain
    );
}

#[test]
fn r4_ordinary_capacity_stops_close_and_release_before_later_state() {
    for (close, kill) in [(true, false), (true, true), (false, false)] {
        failure_case(false, close, kill);
    }
}

#[test]
fn r4_counter_callback_capacity_keeps_later_state_and_uncertain_commit() {
    for (close, kill) in [(true, false), (true, true), (false, false)] {
        failure_case(true, close, kill);
    }
}

#[test]
fn r4_failed_cap_application_does_not_flush_after_the_latch() {
    for counter in [false, true] {
        let (mut e, files) = fixture();
        let cap = e.cap(files[0], CLIENT).unwrap().clone();
        let later_cap = e.cap(files[1], CLIENT).unwrap().clone();
        let later_core = e.inode(files[1]).unwrap().clone();
        let allocator = e.ino_alloc.state();
        let attempts = e.journal().attempts.len();
        let flushes = e.journal().inner.flush_count();
        if counter {
            e.retry_capacity_epoch = u64::MAX;
        } else {
            e.journal_mut().reject = true;
        }
        let effects = e.handle_cap_update(
            CLIENT,
            CapUpdate {
                op: CEPH_CAP_OP_FLUSH,
                ino: files[0],
                cap_id: cap.cap_id,
                seq: cap.last_sent,
                issue_seq: cap.last_issue,
                caps: cap.pending(),
                wanted: cap.wanted(),
                dirty: CEPH_CAP_XATTR_EXCL,
                flush_tid: 99,
                flags: CEPH_CLIENT_CAPS_SYNC,
                xattrs: Some((1, XattrMap::from([(b"user.keep".to_vec(), vec![7; 16])]))),
                ..Default::default()
            },
        );
        assert_eq!(e.failure(), Some(JournalError::Capacity));
        assert!(effects.is_empty());
        assert_eq!(e.journal().attempts.len(), attempts + 1);
        assert_eq!(
            e.journal().attempts.last().unwrap().kind,
            RecordKind::CapUpdate
        );
        assert_eq!(e.journal().inner.flush_count(), flushes);
        assert_eq!(e.cap(files[1], CLIENT), Some(&later_cap));
        assert_eq!(e.inode(files[1]), Some(&later_core));
        assert_eq!(e.ino_alloc.state(), allocator);
    }
}

#[test]
fn r4_successful_close_keeps_graceful_and_kill_cleanup() {
    for kill in [false, true] {
        let (mut e, files) = fixture();
        let key = crate::SessionKey {
            client: CLIENT,
            incarnation: e.session_epoch(CLIENT),
        };
        let tail = e.journal().inner.last_seq();
        let attempts = e.journal().attempts.len();
        let effects = e.close_session(CLIENT, kill);
        assert!(!e.is_failed());
        assert_eq!(e.journal().attempts.len(), attempts + 3);
        assert_eq!(e.journal().inner.last_seq(), tail + 3);
        assert_eq!(e.session_cleanup_seq(key), Some(tail + 3));
        assert!(e.session(CLIENT).is_none());
        assert!(e.ino_alloc.delegated_to(CLIENT).is_empty());
        for ino in files {
            assert!(e.cap(ino, CLIENT).is_none());
            assert_eq!(
                e.inode(ino).unwrap().client_ranges.contains_key(&CLIENT),
                kill
            );
        }
        assert_eq!(
            effects
                .iter()
                .filter(|effect| matches!(effect, Effect::RecoverFileSize { .. }))
                .count(),
            usize::from(kill) * 2
        );
    }
}
